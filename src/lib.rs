use std::net::SocketAddr;
use std::sync::Arc;

use futures::future::try_join_all;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::try_join;

/// Starts a TCP server that forwards incoming connections to multiple destinations.
///
/// # Arguments
///
/// * `from_addr` - The address to bind the server to.
/// * `to_addrs` - A vector of destination addresses to forward incoming connections to.
///
/// # Example
///
/// ```
/// use yprox::start;
///
/// #[tokio::main]
/// async fn main() {
///     let from_addr = "127.0.0.1:8080";
///     let to_addrs = vec!["127.0.0.1:8081", "127.0.0.1:8082"];
///     start(from_addr, to_addrs).await;
/// }
/// ```
pub async fn start(from_addr: SocketAddr, to_addrs: Vec<SocketAddr>) -> io::Result<()> {
    start_modifying(from_addr, to_addrs, None).await
}

/// Starts a TCP server that forwards incoming connections to multiple destinations with an optional data modification function.
///
/// This function is an extension of `start`, allowing the caller to specify a function to modify
/// the data before forwarding. The `modify` function is applied to each chunk of data
/// received from the client before it is sent to the server.
///
/// # Arguments
///
/// * `from_addr` - The address to bind the server to.
/// * `to_addrs` - A vector of destination addresses to forward incoming connections to.
/// * `modify` - An optional function to modify the data. It takes a `Vec<u8>` and returns a `Vec<u8>`.
///
/// # Example
///
/// ```
/// use yprox::start_modifying;
///
/// #[tokio::main]
/// async fn main() {
///     let from_addr = "127.0.0.1:8080";
///     let to_addrs = vec!["127.0.0.1:8081", "127.0.0.1:8082"];
///     let modify_fn = |data: Vec<u8>| -> Vec<u8> {
///         // Modify data here
///         data
///     };
///     start_modifying(from_addr, to_addrs, Some(modify_fn)).await;
/// }
/// ```
pub async fn start_modifying(
    from_addr: SocketAddr,
    to_addrs: Vec<SocketAddr>,
    modify: Option<fn(Vec<u8>) -> Vec<u8>>,
) -> io::Result<()> {
    let listener = TcpListener::bind(from_addr).await?;
    let to_addrs = Arc::from(to_addrs);

    loop {
        let (client, _) = listener.accept().await?;
        tokio::spawn(handle_client(client, Arc::clone(&to_addrs), modify));
    }
}

/// Handles a client connection by proxying data to multiple target addresses.
///
/// # Arguments
///
/// * `client` - A `TcpStream` representing the client connection.
/// * `target_addrs` - A `Vec<String>` containing the target addresses to proxy data to.
/// * `modify` - An optional function to modify the data. It takes a `Vec<u8>` and returns a `Vec<u8>`.
///
/// # Examples
///
/// ```
/// use tokio::net::TcpStream;
/// use yprox::handle_client;
///
/// async fn run() {
///     let client = TcpStream::connect("127.0.0.1:8080").await.unwrap();
///     let target_addrs = vec!["127.0.0.1:9000".to_string(), "127.0.0.1:9001".to_string()];
///     let modify_fn = |data: Vec<u8>| -> Vec<u8> {
///         // Modify data here
///         data
///     };
///     handle_client(client, target_addrs, modify_fn).await;
/// }
/// ```
async fn handle_client(
    client: TcpStream,
    target_addrs: Arc<[SocketAddr]>,
    modify: Option<fn(Vec<u8>) -> Vec<u8>>,
) {
    let mut server_handles: Vec<JoinHandle<_>> = Vec::new();
    let mut client_to_server_handles: Vec<JoinHandle<_>> = Vec::new();
    let client = Arc::new(Mutex::new(client));

    for (n, target_addr) in target_addrs.iter().enumerate() {
        let server = match TcpStream::connect(&target_addr).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to connect to target: {}", e);
                continue;
            }
        };
        let server = Arc::new(Mutex::new(server));

        let (client_tx, client_rx) = mpsc::channel(32);
        let client_rx = Arc::new(Mutex::new(client_rx));
        let client_tx = Arc::new(Mutex::new(client_tx));

        let (server_tx, server_rx) = mpsc::channel(32);
        let server_rx = Arc::new(Mutex::new(server_rx));
        let server_tx = Arc::new(Mutex::new(server_tx));

        let direction = format!("Client -> Server[{}]", target_addr);
        let client_to_server = tokio::spawn(proxy(
            client.clone(),
            client_tx.clone(),
            server_rx.clone(),
            direction,
            modify,
        ));
        client_to_server_handles.push(client_to_server);

        if n == 0 {
            let direction = format!("Server[{}] -> Client", target_addr);
            let server_to_client =
                tokio::spawn(proxy(server, server_tx, client_rx, direction, modify));
            server_handles.push(server_to_client);
        }
    }

    let _ = try_join!(
        try_join_all(client_to_server_handles),
        try_join_all(server_handles)
    )
    .map_err(|e| {
        eprintln!("Error in communication: {}", e);
    });
}

/// Proxies data between a TCP stream and a channel of data, with an optional data modification function.
///
/// This function is similar to `proxy` but allows for modification of the data
/// using the provided `modify` function. The data received from the stream is modified
/// before being sent through the channel.
///
/// # Arguments
///
/// * `stream` - An `Arc<Mutex<TcpStream>>` representing the TCP stream to proxy data to/from.
/// * `tx` - An `Arc<Mutex<mpsc::Sender<Vec<u8>>>>` representing the channel to send modified data to the TCP stream.
/// * `rx` - An `Arc<Mutex<mpsc::Receiver<Vec<u8>>>>` representing the channel to receive data from the TCP stream.
/// * `direction` - A `String` representing the direction of the data flow (e.g. "client to server").
/// * `modify` - An optional function to modify the data before sending.
///
/// # Examples
///
/// ```
/// use tokio::net::TcpStream;
/// use tokio::sync::{mpsc, Mutex, Arc};
/// use yprox::proxy;
///
/// async fn run() {
///     let stream = TcpStream::connect("127.0.0.1:8080").await.unwrap();
///     let (tx, rx) = mpsc::channel(1024);
///     let stream = Arc::new(Mutex::new(stream));
///     let tx = Arc::new(Mutex::new(tx));
///     let rx = Arc::new(Mutex::new(rx));
///     let direction = "client to server".to_string();
///     let modify_fn = |data: Vec<u8>| -> Vec<u8> {
///         // Modify data here
///         data
///     };
///     proxy(stream, tx, rx, direction, Some(modify_fn)).await.unwrap();
/// }
/// ```
async fn proxy(
    stream: Arc<Mutex<TcpStream>>,
    tx: Arc<Mutex<mpsc::Sender<Vec<u8>>>>,
    rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    direction: String,
    modify: Option<fn(Vec<u8>) -> Vec<u8>>,
) -> Result<(), std::io::Error> {
    let mut buf = vec![0u8; 4096];

    loop {
        let mut locked_rx = rx.lock().await;
        let mut stream = stream.lock().await;

        tokio::select! {
            n = stream.read(&mut buf) => {
                let n = n?;
                if n == 0 {
                    println!("\n{}: Connection closed", direction);
                    break;
                }
                println!("\n{}: Transferred {} bytes", direction, n);
                hex_dump(&buf[..n], &direction);
                let modify = modify.unwrap_or(|x| x);
                tx.lock().await.send(modify(buf[..n].to_vec())).await.expect("Failed to send data");
            }
            data = locked_rx.recv() => {
                if let Some(data) = data {
                    stream.write_all(&data).await?;
                } else {
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Prints a hex dump of the given data with an optional direction string.
///
/// # Arguments
///
/// * `data` - A slice of bytes to be printed as a hex dump.
/// * `direction` - An optional string indicating the direction of the data flow.
///
/// # Example
///
/// ```
/// let data = [0x48, 0x65, 0x6C, 0x6C, 0x6F];
/// hex_dump(&data, "OUTGOING");
/// ```
fn hex_dump(data: &[u8], direction: &str) {
    const WIDTH: usize = 16;

    for chunk in data.chunks(WIDTH) {
        let hex: Vec<String> = chunk.iter().map(|b| format!("{:02X}", b)).collect();
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if (0x20..=0x7e).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();

        println!("{}: {:47}  |{}|", direction, hex.join(" "), ascii);
    }
}
