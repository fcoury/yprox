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
        let (socket, _) = listener.accept().await?;
        // send tcp stream to a task handler
        let backends = backends.clone();
        tokio::spawn(async move {
            handle_client(&backends, socket).await;
        });
    }
}

async fn handle_client(backends: &[String], socket: TcpStream) {
    // creates a broadcast channel to send messages from the client
    let (broadcast_sender, _) = broadcast::channel::<Vec<u8>>(32);
    // creates a channel to receive responses from each backend
    let (backend_response_sender, mut backend_response_receiver) = mpsc::channel::<Vec<u8>>(32);
    let (mut client_receiver, mut client_sender) = socket.into_split();

    for backend in backends {
        // connects to each backend
        let backend_connection = TcpStream::connect(backend).await.unwrap();
        let (mut backend_receiver, mut backend_sender) = backend_connection.into_split();

        // sender task
        let broadcast_sender = broadcast_sender.clone();
        tokio::spawn(async move {
            loop {
                let mut broadcast_receiver = broadcast_sender.subscribe();
                let data = broadcast_receiver.recv().await.unwrap();
                // sends the broadcast data to this backend
                backend_sender.write_all(&data).await.unwrap();
            }
        });

        // receiver task
        let backend_response_sender = backend_response_sender.clone();
        tokio::spawn(async move {
            let mut buffer = [0; 1024];
            loop {
                let n = backend_receiver.read(&mut buffer).await.unwrap();
                if n == 0 {
                    break;
                }
                let data = buffer[..n].to_vec();
                // sends the backend response to the client
                backend_response_sender.send(data).await.unwrap();
            }
        });
    }

    tokio::spawn(async move {
        loop {
            let data = backend_response_receiver.recv().await.unwrap();
            // sends the backend response to the client
            client_sender.write_all(&data).await.unwrap();
        }
    });

    let mut buffer = [0; 1024];
    loop {
        // receives and broadcasts data from the client
        let n = client_receiver.read(&mut buffer).await.unwrap();
        if n == 0 {
            break;
        }
        let data = buffer[..n].to_vec();
        broadcast_sender.send(data).unwrap();
    }
}
