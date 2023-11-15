use std::{
    net::{SocketAddr, TcpListener},
    sync::{mpsc, Arc, Mutex},
    thread,
};

use broadcaster::broadcaster;
use client::client;
use error::Result;
use hooks::{hook_executor, Hook, Request, Response};
use server::server;

pub mod broadcaster;
pub mod cli;
pub mod client;
pub mod error;
pub mod hooks;
pub mod server;
pub mod target;
pub mod utils;

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
/// let bind_addr = "127.0.0.1:8080".parse().unwrap();
/// let targets = vec![
///     ("server1".to_string(), "127.0.0.1:8081".parse().unwrap()),
///     ("server2".to_string(), "127.0.0.1:8082".parse().unwrap())
/// ];
/// start_proxy(bind_addr, targets);
/// ```
pub fn start_proxy(bind_addr: SocketAddr, targets: Vec<(String, SocketAddr)>) -> Result<()> {
    start_proxy_with_hooks(bind_addr, targets, vec![])
}

pub fn start_proxy_with_hooks(
    bind_addr: SocketAddr,
    targets: Vec<(String, SocketAddr)>,
    hooks: Vec<Hook>,
) -> Result<()> {
    let listener = TcpListener::bind(bind_addr)?;

    // used to send messages to the server
    let (send_message, receive_message) = mpsc::channel();

    // used to send broadcasts to all targets
    let (send_broadcast, receive_broadcast) = mpsc::channel();

    // hooks
    let hooks = Arc::new(Mutex::new(hooks));

    // used to send requests to the hook executor from the server context
    let (server_request_sender, server_request_receiver) = mpsc::channel::<Request>();
    let (server_response_sender, server_response_receiver) = mpsc::channel::<Result<Response>>();
    hook_executor(
        hooks.clone(),
        server_request_receiver,
        server_response_sender,
    );

    // used to send requests to the hook executor from the broadcaster context
    let (broadcaster_request_sender, broadcaster_request_receiver) = mpsc::channel::<Request>();
    let (broadcaster_response_sender, broadcaster_response_receiver) =
        mpsc::channel::<Result<Response>>();

    hook_executor(
        hooks,
        broadcaster_request_receiver,
        broadcaster_response_sender,
    );

    // spawn the server thread (handles server -> client and server -> broadcast)
    // handles messages between client and server, and sends broadcasts
    let server_request_sender = server_request_sender.clone();
    thread::spawn(|| {
        server(
            receive_message,
            send_broadcast,
            server_request_sender,
            server_response_receiver,
        )
    });

    // spawn the broadcasting thread (handles server -> targets and targets -> server)
    // the breadcaster receives broadcast requests and sends them to all targets
    // it also receives the send_message handle so that each target can send individual
    // responses to the server
    let send_message_clone = send_message.clone();
    let broadcaster_request_sender = broadcaster_request_sender.clone();
    thread::spawn(|| {
        broadcaster(
            targets,
            receive_broadcast,
            send_message_clone,
            broadcaster_request_sender,
            broadcaster_response_receiver,
        )
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
