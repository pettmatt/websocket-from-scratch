## Running command

```bash
cargo run --bin server
```

## Based on

https://www.rfc-editor.org/rfc/rfc6455

## Todo

- [ ] Investigate why requests from Postman always create an error.
- [x] (Base implementation) The server selects one or none of the acceptable protocols and echoes
   that value in its handshake to indicate that it has selected that
   protocol.
- [ ] The Origin header handled
   - [x] The |Origin| header field [RFC6454] is used to protect against
   unauthorized cross-origin use of a WebSocket server by scripts using
   the WebSocket API in a web browser. 
   - [ ] The server is informed of the script origin generating the 
   WebSocket connection request. If the server does not wish to accept 
   connections from this origin, it can choose to reject the connection 
   by sending an appropriate HTTP error code. This header field is sent 
   by browser clients; for non-browser clients, this header field may be 
   sent if it makes sense in the context of those clients.
- [x] To prove that the handshake was received, the server has to take two
   pieces of information and combine them to form a response.
  - Header's `Sec-WebSocket-Key` value
  - WebSocket GUID, a fixed globally unique identifier specified in RFC 6455
- [ ] The client handshake