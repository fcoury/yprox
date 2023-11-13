use std::{net::SocketAddr, str::FromStr};

use clap::Parser;

#[derive(Parser)]
pub struct Args {
    /// The address to listen on
    pub listen_addr: SocketAddr,

    /// Main target address
    pub main_target_addr: Target,

    /// Additional target addresses
    pub secondary_target_addrs: Vec<Target>,

    /// Modifying script
    #[clap(short, long)]
    pub script: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Target {
    Anon(SocketAddr),
    Named(String, SocketAddr),
}

impl FromStr for Target {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_target(s)
    }
}

fn parse_target(s: &str) -> Result<Target, String> {
    if let Some(pos) = s.find('=') {
        let key = s[..pos].to_string();
        let value = s[pos + 1..]
            .parse::<SocketAddr>()
            .map_err(|_| format!("Invalid SocketAddr: {}", &s[pos + 1..]))?;
        Ok(Target::Named(key, value))
    } else {
        let addr = s
            .parse::<SocketAddr>()
            .map_err(|_| format!("Invalid SocketAddr: {}", s))?;
        Ok(Target::Anon(addr))
    }
}
