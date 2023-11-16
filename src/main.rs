use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = "127.0.0.1:27018";
    let backends = vec!["127.0.0.1:27017".to_string(), "127.0.0.1:27016".to_string()];

    let listener = TcpListener::bind(addr).await?;

    loop {
        // accept connection
        let (socket, client_address) = listener.accept().await?;
        // send tcp stream to a task handler
        let backends = backends.clone();
        println!("Client connected: {}", client_address);
        tokio::spawn(async move {
            handle_client(&backends, socket).await;
        });
    }
}

async fn handle_client(backends: &[String], socket: TcpStream) {
    let client_address = socket.peer_addr().unwrap();
    // creates a broadcast channel to send messages from the client
    let (broadcast_sender, _) = broadcast::channel::<Vec<u8>>(32);
    // creates a channel to receive responses from each backend
    let (backend_response_sender, mut backend_response_receiver) = mpsc::channel::<Vec<u8>>(32);
    let (mut client_receiver, mut client_sender) = socket.into_split();

    for backend in backends {
        // connects to each backend
        let backend_connection = TcpStream::connect(backend).await.unwrap();
        let backend_address = backend_connection.peer_addr().unwrap();
        let (mut backend_receiver, mut backend_sender) = backend_connection.into_split();

        // sender task
        let broadcast_sender = broadcast_sender.clone();
        tokio::spawn(async move {
            loop {
                let mut broadcast_receiver = broadcast_sender.subscribe();
                match broadcast_receiver.recv().await {
                    Ok(data) => {
                        // sends the broadcast data to this backend
                        hex_dump(
                            &data,
                            format!("{} -> {}", &client_address, &backend_address).as_str(),
                        );
                        if let Err(err) = backend_sender.write_all(&data).await {
                            eprintln!(
                                "Error sending data from client {} to backend {}: {}",
                                client_address, backend_address, err
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
        });

        // receiver task
        let backend_response_sender = backend_response_sender.clone();
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
                        hex_dump(
                            &data,
                            format!("{} -> {}", &backend_address, &client_address).as_str(),
                        );
                        if let Err(err) = backend_response_sender.send(data).await {
                            eprintln!(
                                "Error sending backend response from {} to client {}: {}",
                                backend_address, client_address, err
                            );
                            break;
                        }
                    }
                    Err(err) => {
                        eprintln!(
                            "Error receiving data from backend {}: {}",
                            backend_address, err
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

        println!("{:20}: {:47}  |{}|", info, hex.join(" "), ascii);
    }
}
