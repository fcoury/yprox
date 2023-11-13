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
yprox <listen_addr> <target1>...<targetN>
```

For example:

```sh
yprox 127.0.0.1:8080 127.0.0.1:9000 127.0.0.1:9001
```

This will start a proxy server that listens on 127.0.0.1:8080 and forwards incoming connections to 127.0.0.1:9000 and 127.0.0.1:9001.

You can also optionally name each target using a `key=value` format:

```sh
yprox 127.0.0.1:8080 qa=127.0.0.1:9000 test=127.0.0.1:9001 
```

This way the logs will be using `qa` and `test` to identify the streams going to or coming from those targets. If you don't provide a name, a default one will be provided with the format `targetN`.

### As a library

To use yprox as a library, add it to your Cargo.toml file:

```toml
[dependencies]
yprox = "0.1"
```

Then, you can use it in your code:

```rust
use yprox::start_proxy;

#[tokio::main]

async fn main() {
    let bind_addr = SocketAddr::parse("127.0.0.1:8080");
    let targets = vec![
        ("server1".to_string(), SocketAddr::new("127.0.0.1:8081")),
        ("server2".to_string(), SocketAddr::new("127.0.0.1:8082"))
    ];
    start_proxy(bind_addr, targets).await;
}
```

This will start a proxy server that listens on 127.0.0.1:8080 and forwards incoming connections to 127.0.0.1:9000 and 127.0.0.1:9001.
