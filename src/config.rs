use std::{
    collections::HashMap,
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use clap::Parser;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub bind: SocketAddr,
    pub backends: Backends,
    #[serde(default)]
    pub default_backend: Option<String>,
    #[serde(default)]
    pub scripts: Vec<String>,
}

impl Config {
    pub fn backends(&self) -> HashMap<String, SocketAddr> {
        match &self.backends {
            Backends::Anon(backends) => backends
                .iter()
                .enumerate()
                .map(|(i, backend)| (format!("backend{}", i + 1), *backend))
                .collect(),
            Backends::Named(backends) => backends.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Backends {
    Anon(Vec<SocketAddr>),
    Named(HashMap<String, SocketAddr>),
}

pub fn load(config_file: &Path) -> anyhow::Result<Config> {
    let config = fs::read_to_string(config_file)?;
    Ok(toml::from_str::<Config>(&config)?)
}

#[derive(Debug, Parser)]
pub struct Args {
    /// Location of the config file
    #[clap(short, long, default_value = "yprox.toml")]
    pub config: PathBuf,
}
