use std::net::SocketAddr;

use clap::Parser;
use tokio::io;
use yprox::start;

#[derive(Parser)]
struct Cli {
    /// The address to listen on
    from_addr: SocketAddr,

    /// The address that replies
    active_to_addr: SocketAddr,

    /// The addresses that only listen
    passive_to_addr: Vec<SocketAddr>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Cli::parse();

    println!(
        "Listening on {} -> {} + {:?}",
        args.from_addr, args.active_to_addr, args.passive_to_addr
    );
    start(args.from_addr, args.active_to_addr, args.passive_to_addr).await
}
