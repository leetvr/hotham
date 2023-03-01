# Hotham Editor Protocol
## Why?
Whenever two programs want to talk to eachother, the most complicated question is *where is the protocol defined?*. Sometimes the protocol is inferred from the way the programs serialise and deserialise their messages, but this leads to all sorts of problems.

In essence, this is a *contract* problem. What is the contract, where is it, and how is it enforced?

For more on this topic, we can, as always, defer to [Joe Armstrong (RIP)](https://www.youtube.com/watch?v=ed7A7r6DBsM).

## How?
Simple. We define:

- What the messages of the protocol are
- A means to **encode** them to bytes
- A means to **decode** them to bytes

We can even take that a step further and define FSMs (as Joe would suggest), but that is future work.


## Examples
Let's say we're using Unix sockets:

```rust
let socket = UnixStream::connect("hotham_editor.socket").unwrap();
let client = EditorClient::new(socket); // woah, generics

let view_configuration = client.request(&requests::GetViewConfiguration {}).unwrap(); // view_configuration is the correct type!!
```

This magic is made possible through the `Request` trait:

```rust
pub trait Request {
    /// What should the response to this request be?
    type Response: Clone;

    /// Get a `RequestType` tag that we can use to identify requests
    fn request_type(&self) -> RequestType;
}
```
