use std::convert::Infallible;
use std::future::IntoFuture;
use std::net::SocketAddr;
use std::time;

use http_body_util::{Full, BodyExt, combinators::BoxBody};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::upgrade::{Upgraded, OnUpgrade};
use hyper::{Request, Response, Method, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use base64::engine::general_purpose::STANDARD as base64;
use sha1::{Digest, Sha1};

fn validate_websocket_header(key_value: &str, value_priority_list: Vec<(&str, i32)>) -> Option<String> {
    // value_priority_list should contain valid string(s) and its priority value. "*" allows any value.
    let values: Vec<&str> = key_value.split(", ").collect();

    let mut most_valuable = None;
    let mut highest_priority = -1;

    for value in values {
        let value = value.trim();

        if let Some(priority) = value_priority_list.iter()
            .find_map(|(v, p)| if *v == value { Some(*p) } else { None }) {
                if priority > highest_priority {
                    highest_priority = priority;
                    most_valuable = Some(value.to_string());
                }
        }
    }

    most_valuable
}

// Service that return simple 200 response.
async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
}

async fn handle_websocket(upgraded: Upgraded) {}

async fn routing(request: Request<hyper::body::Incoming>) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    match request.method() {
        &Method::GET => {
            match request.uri().path() {
                "/" => {
                    Ok(Response::new(full(
                        "Try to request /websocket",
                    )))
                },
                "/websocket" => {
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

                    // Check if request contains end frame (to end the handshake)
                    // Start the websocket session
                    if let Some(upgrade) = request.extensions().get::<OnUpgrade>() {
                        tokio::spawn(async move {
                            if let Ok(upgraded) = upgrade.into_future().await {
                                if let error = handle_websocket(upgraded).await {
                                    eprintln!("WebSocket error: {:?}", error);
                                }
                            }
                        });
                    }
        
                    let (parts, _body) = request.into_parts();
                    let message = parts.headers
                        .iter()
                        .map(|(key, value)| {
                            let key_str = key.as_str();
                            let value_str = value.to_str().unwrap_or_default();
                            let mut validated_websocket_protocol = None;

                            // Todo: create more elegant way of checking headers and returning an error when validated_websocket_protocol returns None.
                            if key_str == "sec-websocket-protocol" {
                                let value_priority_list = vec![
                                    ("chat", 1),
                                    ("superchat", 2)
                                ];
                                validated_websocket_protocol = validate_websocket_header(value_str, value_priority_list);
                                return format!("{}: {}", key_str, validated_websocket_protocol.unwrap());
                            }

                            if key_str == "origin" {
                                let value_priority_list = vec![
                                    ("http://example.com", 1)
                                ];
                                validated_websocket_protocol = validate_websocket_header(value_str, value_priority_list);
                                return format!("{}: {}", key_str, validated_websocket_protocol.unwrap());
                            }

                            if key_str == "sec-websocket-key" {
                                // Globally Unique Identifier (GUID)
                                let websocket_guid = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
                                let concatenated = format!("{}{}", key_str, websocket_guid);

                                // Compute the SHA-1 hash
                                let mut hasher = Sha1::new();
                                hasher.update(concatenated.as_bytes());
                                let hash = hasher.finalize();

                                // Base64-encode the hash
                                let accept_value = base64.encode(hash);

                                return format!("Sec-WebSocket-Accept: {}", accept_value);
                            }

                            format!("{}: {}", key_str, value_str)
                        }
                    ).collect::<Vec<_>>().join("\n");
        
                    Ok(Response::new(full(message)))
                },
                _ => {
                    let mut not_found = Response::new(empty());
                    *not_found.status_mut() = StatusCode::NOT_FOUND;
                    Ok(not_found)
                }
            }
        },
        _ => {
            let mut not_found = Response::new(empty());
            *not_found.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let address = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(address).await?;

    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement `hyper::rt` IO traits.
        let io_stream = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            let start = time::SystemTime::now();
            let builder = http1::Builder::new();
            // `service_fn` converts our function to a `Service`
            if let Err(error) = builder
                .serve_connection(io_stream, service_fn(routing))
                .await
            {
                eprintln!("Error serving connection: {:?}", error.to_string());
            } else {
                println!("Serving connection");
            }

            println!("Time used to process {:?}", start.elapsed().unwrap_or_default());
        });
    }
}
