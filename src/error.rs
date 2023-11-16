use std::{net::SocketAddr, sync::mpsc};

use crate::{broadcaster::BroadcastRequest, hooks::HookRequest, server::Message};

#[derive(Debug)]
pub enum Error {
    AcceptingConnection(std::io::Error),
    ReceiveError(mpsc::RecvError),
    SendError(mpsc::SendError<Message>),
    HookExecutionError(mpsc::SendError<HookRequest>),
    BroadcastError(mpsc::SendError<BroadcastRequest>),
    ConnectionError {
        target: SocketAddr,
        cause: std::io::Error,
    },
    UnexpectedError(String),
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

impl From<mpsc::SendError<BroadcastRequest>> for Error {
    fn from(err: mpsc::SendError<BroadcastRequest>) -> Self {
        Self::BroadcastError(err)
    }
}

impl From<mpsc::SendError<HookRequest>> for Error {
    fn from(err: mpsc::SendError<HookRequest>) -> Self {
        Self::HookExecutionError(err)
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self::UnexpectedError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
