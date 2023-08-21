use clap::Parser;
use yprox::start;

#[derive(Parser)]
struct Cli {
    /// The address to listen on
    from_addr: String,

    /// The address to forward to
    to_addr: Vec<String>,
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    println!("Listening on {} -> {:?}", args.from_addr, args.to_addr);
    start(args.from_addr, args.to_addr).await;
}
