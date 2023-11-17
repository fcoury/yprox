<img align="right" src="docs/logo-circle.png" style="float: right">

# yprox

`yprox` is a versatile TCP proxy server tool and a Rust library, designed for modifying and multiplexing network traffic. It allows users to track and analyze traffic for different backend systems, aiding in performance monitoring and system emulation. `yprox` can be seamlessly used both as a standalone executable and as an integrated library in Rust applications.

## Project Status

`yprox` is currently under active development. The tool's primary aim is to facilitate the monitoring of multiple backend behaviors by capturing and comparing traffic between the original system and a secondary system designed to replicate the original's functionality.

## Installation

Install `yprox` easily using Cargo, Rust's package manager:

```sh
cargo install yprox
```

## Configuration

### Using a Configuration File

`yprox` supports configuration through a TOML file. By default, it searches for a file named `yprox.toml` in the current directory. This file allows you to specify the server settings in a structured format.

Here is the structure of `yprox.toml`:

```toml
# Example yprox.toml configuration
bind = "ip:port"                   # The bind address
backends = ["ip:port", "ip:port"]  # List of backends
default_backend = "backendName"    # Optional: Default backend name
```

You can also provide `backends` as a *TOML table*:

```toml
backends = { primary = "127.0.0.1:27017", secondary = "127.0.0.1:27016" }
```

This way you can name each backend.

### Command Line Options

If you prefer not to use a configuration file, `yprox` can also be configured via command line arguments:

1. **Configuration File Path:**  
   Optionally specify a custom configuration file path. If not set, `yprox` looks for `yprox.toml` in the current directory.
   ```
   --config <path/to/config.toml>
   ```

2. **Bind Address:**  
   Set the bind address in `ip:port` format. This is required if specifying backends via command line.
   ```
   --bind <ip:port>
   ```

3. **Backend Addresses:**  
   Specify one or more backend addresses, either as `ip:port` or `name=ip:port`. Unnamed backends are automatically named `backend1`, `backend2`, etc.
   ```
   --backend <backend_address>
   ```

4. **Default Backend:**  
   Name the default backend. If not specified, the first backend will be used.
   ```
   --default <backend_name>
   ```

**Example Usage:**

```sh
yprox --bind 127.0.0.1:8080 --backend 127.0.0.1:9000 --backend 127.0.0.1:9001
```

In this example, responses from `127.0.0.1:9001` will be returned to clients.

**Naming Backends:**

For better log clarity, backends can be named using the `key=value` format:

```sh
yprox --bind 127.0.0.1:8080 --backend qa=127.0.0.1:9000 --backend test=127.0.0.1:9001 
```

Unspecified backends will be automatically assigned default names such as `backendN`.

**Setting a Default Backend:**

The _default backend_ refers to the chosen backend whose response is forwarded back to the client. In the absence of a specified default, the first backend is automatically selected.

To set a default backend, use:

```sh
yprox                               \
    --bind 127.0.0.1:8080           \
    --backend qa=127.0.0.1:9000     \
    --backend test=127.0.0.1:9001   \
    --default test
```

In this example, responses from `127.0.0.1:9001` will be returned to clients.

