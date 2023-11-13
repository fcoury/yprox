use clap::Parser;
use cli::Args;
use yprox::{start_proxy, Result};

mod cli;

fn main() -> Result<()> {
    let args = Args::parse();
    let mut targets = vec![args.main_target_addr];
    targets.extend(args.secondary_target_addrs);

    let targets = targets
        .into_iter()
        .enumerate()
        .map(|(i, target)| match target {
            cli::Target::Anon(addr) => Ok((format!("target_{i}"), addr)),
            cli::Target::Named(name, addr) => Ok((name, addr)),
        })
        .collect::<Result<Vec<_>>>()?;

    start_proxy(args.listen_addr, targets)?;
    Ok(())
}
