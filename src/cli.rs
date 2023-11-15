use std::{collections::HashMap, net::SocketAddr, path::PathBuf, str::FromStr};

use clap::{ArgGroup, Parser};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub listen: SocketAddr,
    pub targets: ConfigTargets,
    #[serde(default)]
    pub scripts: Vec<Script>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ConfigTargets {
    Anon(Vec<SocketAddr>),
    Named(HashMap<String, SocketAddr>),
}

#[derive(Debug, Deserialize)]
pub enum Trigger {
    Client,
    Target(Option<String>),
}

#[derive(Debug, Deserialize)]
pub struct Script {
    pub trigger: Option<Trigger>,
    pub target: String,
    pub script: String,
}

#[derive(Debug, Parser)]
#[clap(group = ArgGroup::new("config_or_args").required(false))]
pub struct Args {
    /// The address to listen on
    #[clap(short, long)]
    pub listen: Option<SocketAddr>,

    /// Main target address
    #[clap(long, value_delimiter = ',')]
    pub targets: Option<Vec<Target>>,

    /// Modifying script
    #[clap(short, long)]
    pub script: Option<PathBuf>,

    /// Path to a toml config file
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum Target {
    Anon(SocketAddr),
    Named(String, SocketAddr),
}

impl Target {
    pub fn is_anon(&self) -> bool {
        matches!(self, Target::Anon(_))
    }

    pub fn is_named(&self) -> bool {
        matches!(self, Target::Named(_, _))
    }

    pub fn as_anon(&self) -> Option<SocketAddr> {
        match self {
            Target::Anon(addr) => Some(*addr),
            _ => None,
        }
    }

    pub fn as_named(&self) -> Option<(String, SocketAddr)> {
        match self {
            Target::Named(key, addr) => Some((key.to_string(), *addr)),
            _ => None,
        }
    }
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
