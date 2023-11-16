use std::{collections::HashMap, net::SocketAddr};

use clap::Parser;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
};

mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = config::Args::parse();

    let config = if let Some(ref config_file) = args.config {
        // check if config_file exists
        if !config_file.exists() {
            eprintln!("Config file {:?} does not exist", config_file);
            std::process::exit(1);
        }
        println!("Loading config from {:?}", args.config);
        config::load(&config_file)?
    } else {
        let backends = args.backend.expect("backend is required by clap here");
        let backends = if backends.iter().any(|b| b.contains('=')) {
            config::Backends::Named(
                backends
                    .into_iter()
                    .enumerate()
                    .map(|(i, s)| {
                        let mut parts = s.splitn(2, '=');
                        let first = parts.next().unwrap_or_default().to_string();
                        let last = parts.next();

                        match last {
                            Some(last) => (
                                first.clone(),
                                last.parse()
                                    .map_err(|e| {
                                        eprintln!("Error parsing backend {}: {}", first, e);
                                        std::process::exit(1);
                                    })
                                    .unwrap(),
                            ),
                            None => (
                                format!("backend{}", i + 1),
                                first
                                    .parse()
                                    .map_err(|e| {
                                        eprintln!("Error parsing backend {}: {}", first, e);
                                        std::process::exit(1);
                                    })
                                    .unwrap(),
                            ),
                        }
                    })
                    .collect::<HashMap<String, SocketAddr>>(),
            )
        } else {
            config::Backends::Anon(
                backends
                    .iter()
                    .map(|b| {
                        b.parse()
                            .map_err(|e| {
                                eprintln!("Error parsing backend {}: {}", b, e);
                                std::process::exit(1);
                            })
                            .unwrap()
                    })
                    .collect::<Vec<_>>(),
            )
        };
        config::Config {
            bind: args.bind.unwrap(),
            backends,
            default_backend: None,
            scripts: vec![],
        }
    };

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

async fn handle_client(
    backends: &[(String, SocketAddr)],
    selected_backend: &str,
    socket: TcpStream,
) {
    let client_address = socket.peer_addr().unwrap();
    // creates a broadcast channel to send messages from the client
    let (broadcast_sender, _) = broadcast::channel::<Vec<u8>>(32);
    // creates a channel to receive responses from each backend
    let (backend_response_sender, mut backend_response_receiver) = mpsc::channel::<Vec<u8>>(32);
    let (mut client_receiver, mut client_sender) = socket.into_split();

    for backend in backends {
        let name = backend.0.clone();

        // connects to each backend
        let backend_connection = TcpStream::connect(backend.1).await.unwrap();
        let backend_address = backend_connection.peer_addr().unwrap();
        let (mut backend_receiver, mut backend_sender) = backend_connection.into_split();

        // sender task
        let broadcast_sender = broadcast_sender.clone();
        let bname = name.clone();
        tokio::spawn(async move {
            loop {
                let mut broadcast_receiver = broadcast_sender.subscribe();
                match broadcast_receiver.recv().await {
                    Ok(data) => {
                        // sends the broadcast data to this backend
                        hex_dump(&data, format!("{} -> {}", &client_address, &bname).as_str());
                        if let Err(err) = backend_sender.write_all(&data).await {
                            eprintln!(
                                "Error sending data from client {} to backend {} ({}): {}",
                                client_address, &bname, backend_address, err
                            );
                            break;
                        }
                    }
                    Err(err) => {
                        eprintln!(
                            "Error receiving broadcast for backend {} ({}) for client {}: {}",
                            &bname, backend_address, client_address, err
                        );
                        break;
                    }
                }
            }
        });

        // receiver task
        let backend_response_sender = backend_response_sender.clone();
        let selected_backend = selected_backend.to_string();
        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            loop {
                match backend_receiver.read(&mut buffer).await {
                    Ok(n) => {
                        if n == 0 {
                            println!("Backend disconnected: {}", backend_address);
                            break;
                        }
                        let data = buffer[..n].to_vec();
                        // sends the backend response to the client
                        // only sends this response for the selected backend
                        if name == selected_backend {
                            hex_dump(&data, format!("{} -> {}", &name, &client_address).as_str());
                            if let Err(err) = backend_response_sender.send(data).await {
                                eprintln!(
                                    "Error sending backend {} response to client {}: {}",
                                    name, client_address, err
                                );
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
            let Some(data) = backend_response_receiver.recv().await else {
                eprintln!(
                    "Could not receive a backend response sending to client {}",
                    client_address
                );
                continue;
            };

            // sends the backend response to the client
            if let Err(err) = client_sender.write_all(&data).await {
                eprintln!(
                    "Error sending a backend response to client {}: {}",
                    client_address, err
                );
                break;
            }
        }
    });

    let mut buffer = [0; 1024];
    loop {
        // receives and broadcasts data from the client
        match client_receiver.read(&mut buffer).await {
            Ok(n) => {
                if n == 0 {
                    println!("Client disconnected: {}", client_address);
                    break;
                }
                let data = buffer[..n].to_vec();
                if let Err(err) = broadcast_sender.send(data) {
                    eprintln!(
                        "Error sending data from client {} to backend: {}",
                        client_address, err
                    );
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
