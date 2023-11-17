use std::{
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use clap::Parser;
use indexmap::IndexMap;
use serde::Deserialize;

pub fn parse() -> anyhow::Result<Config> {
    let args = Args::parse();
    let default_config = Path::new("yprox.toml");
    let config_file = if args.config.is_some() {
        args.config
    } else if args.backend.is_none() && default_config.exists() {
        Some(default_config.to_path_buf())
    } else {
        None
    };

    let config = if let Some(ref config_file) = config_file {
        // check if config_file exists
        if !config_file.exists() {
            eprintln!(
                "\x1b[31merror:\x1b[0m config file {:?} does not exist",
                config_file
            );
            std::process::exit(1);
        }
        println!("Loading config from {:?}", config_file);
        load(&config_file)?
    } else {
        let Some(backends) = args.backend else {
            eprintln!(
                "\x1b[31merror:\x1b[0m you need to provide `backend`, `config` attributes or create a yprox.toml file",
            );
            std::process::exit(1);
        };
        let backends = if backends.iter().any(|b| b.contains('=')) {
            Backends::Named(
                backends
                    .into_iter()
                    .enumerate()
                    .map(|(i, s)| {
                        let mut parts = s.splitn(2, '=');
                        let first = parts.next().unwrap_or_default().to_string();
                        let last = parts.next();

                        match last {
                            Some(last) => (
                                first.clone(),
                                last.parse()
                                    .map_err(|e| {
                                        eprintln!(
                                            "\x1b[31merror:\x1b[0m can't parse backend {} - {}",
                                            first, e
                                        );
                                        std::process::exit(1);
                                    })
                                    .unwrap(),
                            ),
                            None => (
                                format!("backend{}", i + 1),
                                first
                                    .parse()
                                    .map_err(|e| {
                                        eprintln!(
                                            "\x1b[31merror:\x1b[0m can't parse backend {} - {}",
                                            first, e
                                        );
                                        std::process::exit(1);
                                    })
                                    .unwrap(),
                            ),
                        }
                    })
                    .collect::<IndexMap<String, SocketAddr>>(),
            )
        } else {
            Backends::Anon(
                backends
                    .iter()
                    .map(|b| {
                        b.parse()
                            .map_err(|e| {
                                eprintln!(
                                    "\x1b[31merror:\x1b[0m can't parse backend {} - {}",
                                    b, e
                                );
                                std::process::exit(1);
                            })
                            .unwrap()
                    })
                    .collect::<Vec<_>>(),
            )
        };
        Config {
            bind: args.bind.unwrap(),
            backends,
            default_backend: args.default,
            scripts: vec![],
        }
    };

    Ok(config)
}

fn load(config_file: &Path) -> anyhow::Result<Config> {
    let config = fs::read_to_string(config_file)?;
    Ok(toml::from_str::<Config>(&config)?)
}

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
    pub fn backends(&self) -> IndexMap<String, SocketAddr> {
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
    Named(IndexMap<String, SocketAddr>),
}

#[derive(Debug, Parser)]
#[clap(author, version, about)]
pub struct Args {
    /// Full path to the configuration file in toml format
    /// When no arguments are given, yprox will look for a yprox.toml file in the current directory.
    #[clap(short, long, verbatim_doc_comment)]
    pub config: Option<PathBuf>,

    /// Bind address in the `ip:port` format.
    #[clap(long, requires = "backend")]
    pub bind: Option<SocketAddr>,

    /// Backend address. Can be specified multiple times.
    /// Each backend can be specified as `ip:port` or `name=ip:port`. If no name is specified, the
    /// backend will be named `backend1`, `backend2`, etc.
    #[clap(long, requires = "bind", verbatim_doc_comment)]
    pub backend: Option<Vec<String>>,

    /// Default backend
    ///
    /// Name of the backend for which response will be sent to the client. If not specified, the
    /// first backend will be used.
    #[clap(long, requires = "backend", verbatim_doc_comment)]
    pub default: Option<String>,
}
