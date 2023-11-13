use std::{net::SocketAddr, sync::mpsc};

use crate::Message;

#[derive(Debug)]
pub enum Error {
    AcceptingConnection(std::io::Error),
    ReceiveError(mpsc::RecvError),
    SendError(mpsc::SendError<Message>),
    BroadcastError(mpsc::SendError<Box<[u8]>>),
    ConnectionError {
        target: SocketAddr,
        cause: std::io::Error,
    },
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::AcceptingConnection(err)
    }
}

impl From<mpsc::RecvError> for Error {
    fn from(err: mpsc::RecvError) -> Self {
        Self::ReceiveError(err)
    }
}

impl From<mpsc::SendError<Message>> for Error {
    fn from(err: mpsc::SendError<Message>) -> Self {
        Self::SendError(err)
    }
}

impl From<mpsc::SendError<Box<[u8]>>> for Error {
    fn from(err: mpsc::SendError<Box<[u8]>>) -> Self {
        Self::BroadcastError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
