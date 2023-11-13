<img align="right" src="docs/logo-circle.png" style="float: right">

# yprox

`yprox` is a versatile TCP proxy server tool and library, designed to modify and multiplex network traffic. It can be used as a standalone executable or integrated as a library in Rust applications.

## Usage

### As an Executable

Install `yprox` using Cargo:

```sh
cargo install yprox
```

To run `yprox`, specify a listening address and one or more target addresses:

```sh
yprox <listen_addr> <target1> ... <targetN>
```

For example, to start a proxy server that listens on `127.0.0.1:8080` and forwards connections to `127.0.0.1:9000` and `127.0.0.1:9001`:

```sh
yprox 127.0.0.1:8080 127.0.0.1:9000 127.0.0.1:9001
```

Optionally, name each target using the `key=value` format for easier log identification:

```sh
yprox 127.0.0.1:8080 qa=127.0.0.1:9000 test=127.0.0.1:9001 
```

Unnamed targets will receive default names in the format `targetN`.

### As a Library

Add `yprox` to your `Cargo.toml`:

```toml
[dependencies]
yprox = "0.2.1"
```

Then, use `yprox` in your Rust application:

```rust
use yprox::start_proxy;

fn main() {
    let bind_addr = "127.0.0.1:8080".parse().unwrap();
    let targets = vec![
        ("server1".to_string(), "127.0.0.1:8081".parse().unwrap()),
        ("server2".to_string(), "127.0.0.1:8082".parse().unwrap())
    ];
    start_proxy(bind_addr, targets);
}
```

This will start a proxy server that listens on `127.0.0.1:8080` and forwards incoming connections to `127.0.0.1:9000` and `127.0.0.1:9001`.
