use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:27018").await.unwrap();
    println!("Server listening on port 27018");

    loop {
        let (client_stream, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            if let Err(e) = handle_client(client_stream).await {
                println!("Error handling client: {}", e);
            }
        });
    }
}

async fn handle_client(mut client_stream: TcpStream) -> tokio::io::Result<()> {
    let mut backend_stream = TcpStream::connect("127.0.0.1:27017").await?;

    let (client_reader, client_writer) = client_stream.split();
    let (backend_reader, backend_writer) = backend_stream.split();

    let client_to_backend =
        log_and_forward_data(client_reader, backend_writer, "Client to Backend");
    let backend_to_client =
        log_and_forward_data(backend_reader, client_writer, "Backend to Client");

    tokio::try_join!(client_to_backend, backend_to_client)?;

    Ok(())
}

async fn log_and_forward_data(
    mut read_stream: impl AsyncRead + Unpin,
    mut write_stream: impl AsyncWrite + Unpin,
    info: &str,
) -> tokio::io::Result<()> {
    let mut buffer = [0; 1024];

    loop {
        let bytes_read = read_stream.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }

        // Log the data
        hex_dump(&buffer[..bytes_read], info);

        // Write the data
        write_stream.write_all(&buffer[..bytes_read]).await?;
    }

    Ok(())
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
