use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{mpsc, Arc},
    thread,
};

use crate::{
    error::{Error, Result},
    server::Message,
    target::Target,
};

pub fn broadcaster(
    targets: Vec<(String, SocketAddr)>,
    receive_broadcast: mpsc::Receiver<Box<[u8]>>,
    send_message: mpsc::Sender<Message>,
) -> Result<()> {
    let mut broadcaster = Broadcaster::new(targets, &send_message)?;

    // spawn the target threads (target -> server)
    for t in &broadcaster.targets {
        let stream = t.stream.clone();
        let name = t.name.clone();
        let send_message = send_message.clone();
        thread::spawn(|| target(name, stream, send_message));
    }

    loop {
        let bytes = receive_broadcast.recv()?;
        broadcaster.new_broadcast(&bytes)?;
    }
}

fn target(name: String, stream: Arc<TcpStream>, send_message: mpsc::Sender<Message>) -> Result<()> {
    let addr = stream.peer_addr()?;
    let mut buffer = [0; 1024];
    loop {
        let n = stream.as_ref().read(&mut buffer)?;
        if n > 0 {
            let bytes: Box<[u8]> = buffer[..n].iter().cloned().collect();
            send_message.send(Message::NewTargetMessage {
                name: name.clone(),
                addr,
                bytes,
            })?;
        } else {
            send_message.send(Message::TargetDisconnected { name, addr })?;
            break;
        }
    }

    Ok(())
}

struct Broadcaster {
    targets: Vec<Target>,
}

impl Broadcaster {
    fn new(
        targets: Vec<(String, SocketAddr)>,
        send_message: &mpsc::Sender<Message>,
    ) -> Result<Self> {
        let connections: Result<Vec<_>> = targets
            .into_iter()
            .map(|(name, target)| {
                let conn = TcpStream::connect(target)
                    .map_err(|cause| Error::ConnectionError { target, cause });
                match conn {
                    Ok(stream) => {
                        send_message.send(Message::TargetConnected {
                            name: name.clone(),
                            addr: stream.peer_addr()?,
                        })?;
                        Ok((name, stream))
                    }
                    Err(err) => Err(err),
                }
            })
            .collect();

        let targets = connections?
            .into_iter()
            .map(|(name, stream)| Target {
                stream: Arc::new(stream),
                name,
            })
            .collect();

        Ok(Self { targets })
    }

    fn new_broadcast(&mut self, bytes: &[u8]) -> Result<()> {
        for target in &self.targets {
            let stream = target.stream.clone();
            let bytes = bytes.to_vec();
            // TODO handle result below
            _ = stream.as_ref().write_all(&bytes);
        }

        Ok(())
    }
}
