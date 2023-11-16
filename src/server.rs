use std::{
    collections::HashMap,
    io::Write,
    net::{SocketAddr, TcpStream},
    sync::{mpsc, Arc},
};

use crate::{
    broadcaster::BroadcastRequest,
    client::Client,
    hooks::{Direction, HookRequest, HookResponse},
    utils::hex_dump,
    Result,
};

pub fn server(
    receive_message: mpsc::Receiver<Message>,
    send_broadcast: mpsc::Sender<BroadcastRequest>,
    send_hook_request: mpsc::Sender<HookRequest>,
    recv_hook_response: mpsc::Receiver<Result<HookResponse>>,
) -> Result<()> {
    let mut server = Server::new();

    for message in receive_message {
        match message {
            Message::ClientConnected { stream, addr } => {
                server.client_connected(stream, addr);
            }
            Message::ClientDisconnected { addr } => {
                server.client_disconnected(addr);
            }
            Message::TargetConnected { name, addr } => {
                server.target_connected(name, addr);
            }
            Message::TargetDisconnected { name, addr } => {
                server.target_disconnected(name, addr);
            }
            Message::TargetReconnected { name, addr } => {
                server.target_reconnected(name, addr);
            }
            Message::NewClientMessage { addr, bytes } => {
                server.new_message(addr, &bytes);
                send_broadcast.send(BroadcastRequest {
                    from_addr: addr,
                    bytes,
                })?;
            }
            Message::NewTargetMessage {
                from_target,
                to_addr,
                bytes,
            } => {
                send_hook_request.send(HookRequest::new(
                    Direction::TargetToClient,
                    &from_target,
                    bytes,
                ))?;
                let response = recv_hook_response.recv()?;
                server.new_response(from_target, to_addr, &response?.data);
            }
        }
    }

    Ok(())
}

pub struct Server {
    pub clients: HashMap<SocketAddr, Client>,
}

impl Server {
    fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    fn client_connected(&mut self, stream: Arc<TcpStream>, addr: SocketAddr) {
        println!("Client connected: {}", addr);

        self.clients.insert(addr, Client { stream });
    }

    fn client_disconnected(&mut self, addr: SocketAddr) {
        println!("Client disconnected: {}", addr);
        self.clients.remove(&addr);
    }

    fn target_connected(&mut self, name: String, addr: SocketAddr) {
        println!("Target {} connected: {}", name, addr);
    }

    fn target_reconnected(&mut self, name: String, addr: SocketAddr) {
        println!("Target {} reconnected: {}", name, addr);
    }

    fn target_disconnected(&mut self, name: String, addr: SocketAddr) {
        println!("Target {} disconnected: {}", name, addr);
    }

    fn new_message(&mut self, addr: SocketAddr, bytes: &[u8]) {
        hex_dump(bytes, format!("{}", addr).as_str());
    }

    fn new_response(&mut self, from_target: String, to_addr: SocketAddr, bytes: &[u8]) {
        hex_dump(bytes, format!("{from_target} -> {to_addr}").as_str());

        match self.clients.get(&to_addr) {
            Some(client) => {
                _ = client.stream.as_ref().write_all(bytes);
                client.stream.as_ref().flush().unwrap();
            }
            None => {
                eprintln!("Could not send response to Client {}: not found", to_addr);
            }
        }
    }
}

pub enum Message {
    ClientConnected {
        stream: Arc<TcpStream>,
        addr: SocketAddr,
    },
    ClientDisconnected {
        addr: SocketAddr,
    },
    NewClientMessage {
        addr: SocketAddr,
        bytes: Box<[u8]>,
    },
    TargetConnected {
        name: String,
        addr: SocketAddr,
    },
    TargetDisconnected {
        name: String,
        addr: SocketAddr,
    },
    NewTargetMessage {
        from_target: String,
        to_addr: SocketAddr,
        bytes: Box<[u8]>,
    },
    TargetReconnected {
        name: String,
        addr: SocketAddr,
    },
}
