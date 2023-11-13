use std::{net::TcpStream, sync::Arc};

pub struct Target {
    pub stream: Arc<TcpStream>,
    pub name: String,
}
