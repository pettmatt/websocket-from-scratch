use std::convert::Infallible;
use std::net::SocketAddr;
use std::future::Future;
use std::u16;

use http_body_util::{Full, BodyExt, combinators::BoxBody};
use hyper::body::{Bytes, Frame};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, Method, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

// Service that return simple 200 response.
async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

async fn routing(
    request: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/") => {
            Ok(Response::new(full(
                "Try to request /websocket",
            )))
        },
        (&Method::GET, "/websocket") => {
            let upgrade_header = request.headers().get("Upgrade")
                .and_then(|header| header.to_str().ok())
                .unwrap_or("Missing Upgrade value");

            if upgrade_header != "websocket" {
                let error_response = error(
                    StatusCode::NOT_ACCEPTABLE,
                    String::from("Unacceptable Upgrade value")
                );
                return Ok(error_response);
            }

            let (parts, _body) = request.into_parts();
            let message = parts.headers
                .iter()
                .map(|(key, value)| {
                    let key_str = key.as_str();
                    let value_str = value.to_str().unwrap_or_default();
                    format!("{}: {}", key_str, value_str)
                }
            ).collect::<Vec<_>>().join("\n");

            Ok(Response::new(full(message)))
        },
        // Return 404 Not Found for other routes.
        _ => {
            let mut not_found = Response::new(empty());
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

// Functions to create the body of a request
fn empty() -> BoxBody<Bytes, hyper::Error> {
    http_body_util::Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}

fn full<T: Into<Bytes>>(chunk: T) -> BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

fn error<T: Into<Bytes>>(status_code: StatusCode, message: T) -> Response<BoxBody<Bytes, hyper::Error>> {
    Response::builder()
        .status(status_code)
        .header("Content-Type", "text/plain")
        .body(full(message))
        .expect("Failed to process request")
}

// fn error<T: Into<Bytes>>(status: StatusCode, message: T) -> Response<BoxBody<Bytes, Infallible>> {
// }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let address = SocketAddr::from(([127, 0, 0, 1], 3000));

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(address).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            let builder = http1::Builder::new();
            if let Err(error) = builder
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(routing))
                .await
            {
                eprintln!("Error serving connection: {:?}", error.to_string());
            } else {
                println!("Serving connection");
            }
        });
    }
}
