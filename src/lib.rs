use std::{
    net::{SocketAddr, TcpListener},
    sync::{mpsc, Arc},
    thread,
};

use broadcaster::broadcaster;
use client::client;
pub use error::{Error, Result};
use server::server;
pub use server::Message;
use target::Target;

mod broadcaster;
mod cli;
mod client;
mod error;
mod server;
mod target;
mod utils;

/// Starts a TCP server that forwards incoming connections to multiple destinations.
///
/// # Arguments
///
/// * `bind_addr` - The address to bind the server to.
/// * `targets` - A vector of names and destination addresses to forward incoming connections to.
///
/// # Example
///
/// ```
/// use yprox::start_proxy;
///
/// fn main() {
///     let bind_addr = "127.0.0.1:8080".parse().unwrap();
///     let targets = vec![
///         ("server1".to_string(), "127.0.0.1:8081".parse().unwrap()),
///         ("server2".to_string(), "127.0.0.1:8082".parse().unwrap())
///     ];
///     start_proxy(bind_addr, targets);
/// }
/// ```
pub fn start_proxy(bind_addr: SocketAddr, targets: Vec<(String, SocketAddr)>) -> Result<()> {
    let listener = TcpListener::bind(bind_addr)?;

    // used to send messages to the server
    let (send_message, receive_message) = mpsc::channel();

    // used to send broadcasts to all targets
    let (send_broadcast, receive_broadcast) = mpsc::channel();

    // spawn the server thread (handles server -> client and server -> broadcast)
    // handles messages between client and server, and sends broadcasts
    thread::spawn(|| server(receive_message, send_broadcast));

    // spawn the broadcasting thread (handles server -> targets and targets -> server)
    // the breadcaster receives broadcast requests and sends them to all targets
    // it also receives the send_message handle so that each target can send individual
    // responses to the server
    let send_message_clone = send_message.clone();
    thread::spawn(|| {
        broadcaster(targets, receive_broadcast, send_message_clone)
            .map_err(|err| eprintln!("{:?}", err))
    });

    // spawn the client threads (handle client -> server)
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let stream = Arc::new(stream);
                let send_message = send_message.clone();
                thread::spawn(|| client(stream, send_message));
            }
            Err(err) => {
                eprintln!("Error accepting connection: {}", err);
            }
        }
    }

    Ok(())
}
