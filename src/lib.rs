use std::sync::Arc;

use futures::future::try_join_all;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::try_join;

async fn handle_client(client: TcpStream, target_addrs: Vec<String>) {
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
        ));
        client_to_server_handles.push(client_to_server);

        if n == 0 {
            let direction = format!("Server[{}] -> Client", target_addr);
            let server_to_client = tokio::spawn(proxy(server, server_tx, client_rx, direction));
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

async fn proxy(
    stream: Arc<Mutex<TcpStream>>,
    tx: Arc<Mutex<mpsc::Sender<Vec<u8>>>>,
    rx: Arc<Mutex<mpsc::Receiver<Vec<u8>>>>,
    direction: String,
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
                tx.lock().await.send(buf[..n].to_vec()).await.expect("Failed to send data");
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

pub async fn start(from_addr: impl Into<String>, to_addrs: Vec<String>) {
    let listener = TcpListener::bind(from_addr.into())
        .await
        .expect("Failed to bind listener");

    loop {
        let (client, _) = listener
            .accept()
            .await
            .expect("Failed to accept connection");
        tokio::spawn(handle_client(client, to_addrs.clone()));
    }
}
