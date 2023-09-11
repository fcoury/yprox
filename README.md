<img align="right" src="docs/logo-circle.png" style="float: right">

# yprox

A modifying, multiplexer tcp proxy server tool and library.

## Usage

### As an executable

To use `yprox` as an executable, simply install it using Cargo:

```sh
cargo install yprox
```

Then, you can run it with:

```sh
yprox <listen_addr> <target_addr1>...<target_addrN>
```

For example:

```sh
yprox 127.0.0.1:8080 127.0.0.1:9000 127.0.0.1:9001
```

This will start a proxy server that listens on 127.0.0.1:8080 and forwards incoming connections to 127.0.0.1:9000 and 127.0.0.1:9001.

### As a library

To use yprox as a library, add it to your Cargo.toml file:

```toml
[dependencies]
yprox = "0.1"
```

Then, you can use it in your code:

```rust
use yprox::proxy::Proxy;

#[tokio::main]
async fn main() {
    let listen_addr = "127.0.0.1:8080";
    let target_addrs = vec!["127.0.0.1:9000", "127.0.0.1:9001"];
    proxy(listen_addr, target_addrs).await.unwrap();
}
```

This will start a proxy server that listens on 127.0.0.1:8080 and forwards incoming connections to 127.0.0.1:9000 and 127.0.0.1:9001.
