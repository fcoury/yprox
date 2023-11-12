use std::net::SocketAddr;

use clap::Parser;
use tokio::io;
use yprox::start;

#[derive(Parser)]
struct Cli {
    /// The address to listen on
    from_addr: SocketAddr,

    /// The address to forward to
    to_addr: Vec<SocketAddr>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let args = Cli::parse();

    println!("Listening on {} -> {:?}", args.from_addr, args.to_addr);
    start(args.from_addr, args.to_addr).await
}
