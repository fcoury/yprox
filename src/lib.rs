use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

async fn handle_client(client: TcpStream, target_addr: String) {
    let server = match TcpStream::connect(target_addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to connect to target: {}", e);
            return;
        }
    };

    let (client_tx, client_rx) = mpsc::channel(32);
    let (server_tx, server_rx) = mpsc::channel(32);

    let client_to_server = tokio::spawn(proxy(client, client_tx, server_rx, "Client -> Server"));
    let server_to_client = tokio::spawn(proxy(server, server_tx, client_rx, "Server -> Client"));

    let _ = tokio::try_join!(client_to_server, server_to_client).map_err(|e| {
        eprintln!("Error in communication: {}", e);
    });
}

async fn proxy(
    mut stream: TcpStream,
    tx: mpsc::Sender<Vec<u8>>,
    mut rx: mpsc::Receiver<Vec<u8>>,
    direction: &str,
) -> Result<(), std::io::Error> {
    let mut buf = vec![0u8; 4096];

    loop {
        tokio::select! {
            n = stream.read(&mut buf) => {
                let n = n?;
                if n == 0 {
                    println!("{}: Connection closed", direction);
                    break;
                }
                println!("\n{}: Transferred {} bytes", direction, n);
                hex_dump(&buf[..n], direction);
                tx.send(buf[..n].to_vec()).await.expect("Failed to send data");
            }
            data = rx.recv() => {
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

pub async fn start(from_addr: impl Into<String>, to_addr: impl Into<String>) {
    let listener = TcpListener::bind(from_addr.into())
        .await
        .expect("Failed to bind listener");

    let to_addr = to_addr.into();
    loop {
        let (client, _) = listener
            .accept()
            .await
            .expect("Failed to accept connection");
        tokio::spawn(handle_client(client, to_addr.clone()));
    }
}
