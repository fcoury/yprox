use std::{
    io::Read,
    net::TcpStream,
    sync::{mpsc, Arc},
};

use crate::{Message, Result};

pub fn client(stream: Arc<TcpStream>, tx: mpsc::Sender<Message>) -> Result<()> {
    let addr = stream.peer_addr()?;

    tx.send(Message::ClientConnected {
        stream: stream.clone(),
        addr,
    })?;

    let mut buffer = [0; 1024];
    loop {
        let n = stream.as_ref().read(&mut buffer)?;
        if n > 0 {
            let bytes: Box<[u8]> = buffer[..n].iter().cloned().collect();
            println!("Request: {}", String::from_utf8_lossy(&bytes));
            tx.send(Message::NewClientMessage { addr, bytes })?;
        } else {
            tx.send(Message::ClientDisconnected { addr })?;
            break;
        }
    }

    Ok(())
}

pub struct Client {
    pub stream: Arc<TcpStream>,
}
