## Running command

```bash
cargo run --bin server
```

## Based on

https://www.rfc-editor.org/rfc/rfc6455

## Todo

- [ ] Investigate why requests from Postman always create an error.
- [ ] The server selects one or none of the acceptable protocols and echoes
   that value in its handshake to indicate that it has selected that
   protocol.
