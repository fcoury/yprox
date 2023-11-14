use std::{
    collections::HashMap,
    io::Write,
    net::{SocketAddr, TcpStream},
    sync::{mpsc, Arc},
};

use crate::{client::Client, utils::hex_dump, Result};

pub type HookFn = Box<dyn Fn(HookDirection, String, Box<[u8]>) -> Option<Box<[u8]>> + Sync + Send>;

pub struct Hook {
    pub typ: HookDirection,
    pub trigger_fn: HookFn,
}

impl Hook {
    pub fn new(typ: HookDirection, trigger_fn: HookFn) -> Self {
        Self { typ, trigger_fn }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HookDirection {
    #[default]
    ClientToTarget,
    TargetToClient,
}
impl HookDirection {
    pub fn from_client(&self) -> bool {
        matches!(self, Self::ClientToTarget)
    }

    pub fn from_target(&self) -> bool {
        matches!(self, Self::TargetToClient)
    }

    pub fn to_client(&self) -> bool {
        matches!(self, Self::TargetToClient)
    }

    pub fn to_target(&self) -> bool {
        matches!(self, Self::ClientToTarget)
    }
}

pub fn server(
    receive_message: mpsc::Receiver<Message>,
    send_broadcast: mpsc::Sender<Box<[u8]>>,
    hooks: Vec<HookFn>,
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
                send_broadcast.send(bytes)?;
            }
            Message::NewTargetMessage { name, addr, bytes } => {
                let bytes = if hooks.is_empty() {
                    bytes
                } else {
                    let mut bytes = bytes;
                    for trigger_fn in hooks {
                        if let Some(new_bytes) =
                            (trigger_fn)(HookDirection::TargetToClient, name.clone(), bytes.clone())
                        {
                            bytes = new_bytes;
                        }
                    }
                    bytes
                };
                server.new_response(name, addr, &bytes);
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

    fn new_response(&mut self, name: String, _addr: SocketAddr, bytes: &[u8]) {
        hex_dump(bytes, &name);
        for client in self.clients.values() {
            // TODO: handle result below
            _ = client.stream.as_ref().write_all(bytes);
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
        name: String,
        addr: SocketAddr,
        bytes: Box<[u8]>,
    },
    TargetReconnected {
        name: String,
        addr: SocketAddr,
    },
}
