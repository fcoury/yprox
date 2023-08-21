<img align="right" src="docs/logo-circle.png" style="float: right">

# yprox

A modifying, multiplexer tcp proxy server tool and library.

```rust
#[tokio::main]
async fn main() {
  // Sends MongoDB requests to 2 different servers
  yprox::start("127.0.0.1:27017", vec!["127.0.0.1:27018", "127.0.0.1:27019]);
}
```
