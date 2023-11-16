use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{mpsc, Arc},
    thread,
};

use crate::{
    error::{Error, Result},
    hooks::{Direction, HookRequest, HookResponse},
    server::Message,
    target::Target,
};

#[derive(Clone)]
pub struct BroadcastRequest {
    pub bytes: Box<[u8]>,
    pub from_addr: SocketAddr,
}

pub fn broadcaster(
    targets: Vec<(String, SocketAddr)>,
    receive_broadcast: mpsc::Receiver<BroadcastRequest>,
    send_message: mpsc::Sender<Message>,
    send_hook_request: mpsc::Sender<HookRequest>,
    recv_hook_response: mpsc::Receiver<Result<HookResponse>>,
) -> Result<()> {
    let mut broadcaster = Broadcaster::new(targets, &send_message)?;

    loop {
        let bytes = receive_broadcast.recv()?;
        broadcaster.new_broadcast(
            bytes,
            &send_hook_request,
            &recv_hook_response,
            &send_message,
        )?;
    }
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

    fn new_broadcast(
        &mut self,
        request: BroadcastRequest,
        send_hook_request: &mpsc::Sender<HookRequest>,
        recv_hook_response: &mpsc::Receiver<Result<HookResponse>>,
        send_message: &mpsc::Sender<Message>,
    ) -> Result<()> {
        for target in &self.targets {
            let stream = target.stream.clone();
            let bytes = request.bytes.clone();

            send_hook_request.send(HookRequest::new(
                Direction::ClientToTarget,
                target.name.clone(),
                bytes,
            ))?;

            let response = recv_hook_response.recv()?;
            _ = stream.as_ref().write_all(&response?.data);

            handle_response(
                target.name.clone(),
                request.from_addr,
                stream,
                send_message.clone(),
            );
        }

        Ok(())
    }
}

fn handle_response(
    name: String,
    to_addr: SocketAddr,
    stream: Arc<TcpStream>,
    send_message: mpsc::Sender<Message>,
) {
    let send_message = send_message.clone();
    let mut stream = stream;
    thread::spawn(move || {
        let mut buffer = [0; 4096];
        loop {
            let n = stream.as_ref().read(&mut buffer).expect("read failed");
            let name = name.clone();

            if n > 0 {
                let bytes: Box<[u8]> = buffer[..n].iter().cloned().collect();
                send_message
                    .send(Message::NewTargetMessage {
                        from_target: name,
                        to_addr,
                        bytes,
                    })
                    .expect("send message failed");
            } else {
                let addr = stream.peer_addr().expect("get peer addr failed");

                send_message
                    .send(Message::TargetDisconnected {
                        name: name.clone(),
                        addr: addr.clone(),
                    })
                    .expect("send message failed");

                loop {
                    thread::sleep(std::time::Duration::from_secs(1));
                    if let Ok(new_stream) = TcpStream::connect(addr.clone()) {
                        send_message
                            .send(Message::TargetReconnected {
                                name: name.clone(),
                                addr,
                            })
                            .expect("send message failed");
                        stream = Arc::new(new_stream);
                        break;
                    }
                }
            }
        }
    });
}
