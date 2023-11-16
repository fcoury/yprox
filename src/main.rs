use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{tcp::OwnedWriteHalf, TcpListener, TcpStream},
    sync::{broadcast, mpsc},
};

mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::parse()?;

    let addr = config.bind;
    let backends = config.backends();
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on {}...", addr);

    let selected_backend = config.default_backend.unwrap_or_else(|| {
        let (name, _) = backends.iter().next().unwrap();
        name.clone()
    });
    let backends: Vec<(String, SocketAddr)> = backends.into_iter().collect();
    loop {
        // accept connection
        let (socket, client_address) = listener.accept().await?;
        // send tcp stream to a task handler
        println!("Client connected: {}", client_address);
        let backends = backends.clone();
        let selected_backend = selected_backend.clone();
        tokio::spawn(async move {
            handle_client(&backends, &selected_backend, socket).await;
        });
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Data(Vec<u8>),
    Disconnect,
}

async fn handle_client(
    backends: &[(String, SocketAddr)],
    selected_backend: &str,
    socket: TcpStream,
) {
    let client_address = socket.peer_addr().unwrap();
    // creates a broadcast channel to send messages from the client
    let (broadcast_tx, _) = broadcast::channel::<Message>(32);
    // creates a channel to receive responses from each backend
    let (backend_response_tx, mut backend_response_rx) = mpsc::channel(32);
    let (mut client_receiver, mut client_sender) = socket.into_split();
    let client_connected = Arc::new(AtomicBool::new(true));

    for backend in backends {
        let name = backend.0.clone();

        // connects to each backend
        let backend_connection = TcpStream::connect(backend.1).await.unwrap();
        let backend_address = backend_connection.peer_addr().unwrap();
        let (mut backend_receiver, mut backend_sender) = backend_connection.into_split();

        // sender task
        let broadcast_tx = broadcast_tx.clone();
        let bname = name.clone();
        let connected = Arc::clone(&client_connected);
        tokio::spawn(async move {
            handle_backend(
                &bname,
                &client_address,
                &backend_address,
                &mut backend_sender,
                broadcast_tx,
                connected,
            )
            .await;
        });

        // receiver task
        let backend_response_tx = backend_response_tx.clone();
        let selected_backend = selected_backend.to_string();
        let connected = Arc::clone(&client_connected);
        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            loop {
                let connected = &connected.load(Ordering::SeqCst);
                if !connected {
                    break;
                }

                match backend_receiver.read(&mut buffer).await {
                    Ok(n) => {
                        if n == 0 {
                            println!("Backend disconnected: {}", backend_address);
                            backend_response_tx.send(Message::Disconnect).await.ok();
                            break;
                        }

                        // sends the backend response to the client
                        // only sends this response for the selected backend
                        let data = buffer[..n].to_vec();
                        if name == selected_backend {
                            hex_dump(&data, format!("{} -> {}", &name, &client_address).as_str());
                            if let Err(_) = backend_response_tx.send(Message::Data(data)).await {
                                break;
                            }
                        } else {
                            hex_dump(&data, format!("{} -|", &name).as_str());
                        }
                    }
                    Err(err) => {
                        eprintln!(
                            "Error receiving data from backend {} ({}): {}",
                            name, backend_address, err
                        );
                        break;
                    }
                }
            }
        });
    }

    tokio::spawn(async move {
        loop {
            let connected = &client_connected.load(Ordering::SeqCst);
            if !connected {
                break;
            }

            let Some(data) = backend_response_rx.recv().await else {
                break;
            };

            match data {
                Message::Data(data) => {
                    // sends the backend response to the client
                    if let Err(err) = client_sender.write_all(&data).await {
                        eprintln!(
                            "Error sending a backend response to client {}: {}",
                            client_address, err
                        );
                        break;
                    }
                }
                Message::Disconnect => {
                    println!("Backend disconnected: {}", client_address);
                    break;
                }
            }
        }
    });

    let mut buffer = [0; 1024];
    loop {
        // receives and broadcasts data from the client
        match client_receiver.read(&mut buffer).await {
            Ok(n) => {
                let (message, disconnect) = if n == 0 {
                    println!("Client disconnected: {}", client_address);
                    (Message::Disconnect, true)
                } else {
                    let data = buffer[..n].to_vec();
                    (Message::Data(data), false)
                };
                if let Err(_) = broadcast_tx.send(message) {
                    break;
                }
                if disconnect {
                    break;
                }
            }
            Err(err) => {
                eprintln!(
                    "Error receiving data from client {}: {}",
                    client_address, err
                );
                break;
            }
        }
    }
}

async fn handle_backend(
    name: &str,
    client_address: &SocketAddr,
    backend_address: &SocketAddr,
    backend_sender: &mut OwnedWriteHalf,
    broadcast_tx: broadcast::Sender<Message>,
    connected: Arc<AtomicBool>,
) {
    loop {
        let mut broadcast_tx = broadcast_tx.subscribe();
        match broadcast_tx.recv().await {
            Ok(Message::Data(data)) => {
                // sends the broadcast data to this backend
                hex_dump(&data, format!("{} -> {}", &client_address, &name).as_str());
                if let Err(err) = backend_sender.write_all(&data).await {
                    eprintln!(
                        "Error sending data from client {} to backend {} ({}): {}",
                        client_address, name, backend_address, err
                    );
                    break;
                }
            }
            Ok(Message::Disconnect) => {
                println!("Client disconnected: {}", client_address);
                _ = &connected.store(false, Ordering::SeqCst);
                break;
            }
            Err(_) => break,
        }
    }
}

/// Prints a hex dump of the given data with an optional info string.
///
/// # Arguments
///
/// * `data` - A slice of bytes to be printed as a hex dump.
/// * `info` - An optional string with info about the data flow.
///
/// # Example
///
/// ```
/// let data = [0x48, 0x65, 0x6C, 0x6C, 0x6F];
/// hex_dump(&data, "OUTGOING");
/// ```
pub fn hex_dump(data: &[u8], info: &str) {
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

        println!("{:35}: {:47}  |{}|", info, hex.join(" "), ascii);
    }
}
