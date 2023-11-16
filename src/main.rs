use std::{collections::HashMap, fs, path::PathBuf, sync::mpsc, thread};

use clap::Parser;
use cli::{Args, Config, ConfigTargets, Script};
use script::exec_worker;

use yprox::hooks::{Hook, HookFn, HookRequest, HookResponse};
pub use yprox::{
    error::{Error, Result},
    start_proxy, start_proxy_with_hooks,
};

use crate::script::ExecRequest;

mod cli;
mod script;

fn main() -> Result<()> {
    let args = Args::parse();
    let config_path = args.config.unwrap_or_else(|| PathBuf::from("yprox.toml"));

    let config = if config_path.exists() {
        let config = fs::read_to_string(config_path)?;
        toml::from_str::<cli::Config>(&config).unwrap()
    } else {
        let Some(listen) = args.listen else {
            eprintln!("Error: must provide a listen address");
            return Ok(());
        };

        let Some(arg_targets) = args.targets else {
            eprintln!("Error: must provide at least one target");
            return Ok(());
        };

        let has_named = arg_targets.iter().any(|target| target.is_named());
        let has_anon = arg_targets.iter().any(|target| target.is_anon());

        if has_named && has_anon {
            eprintln!("Error: cannot mix named and anonymous targets");
            return Ok(());
        }

        let targets = if has_named {
            ConfigTargets::Named(HashMap::from_iter(
                arg_targets
                    .into_iter()
                    .map(|target| target.as_named().unwrap()),
            ))
        } else {
            ConfigTargets::Anon(
                arg_targets
                    .into_iter()
                    .map(|target| target.as_anon().unwrap())
                    .collect(),
            )
        };

        Config {
            listen,
            targets,
            scripts: vec![Script {
                trigger: None,
                target: "target1".to_string(),
                script: fs::read_to_string(args.script.unwrap()).unwrap(),
            }],
        }
    };

    let targets = match config.targets {
        ConfigTargets::Named(targets) => targets
            .into_iter()
            .map(|(name, addr)| (name, addr))
            .collect(),
        ConfigTargets::Anon(targets) => targets
            .into_iter()
            .enumerate()
            .map(|(i, addr)| (format!("target{}", i + 1), addr))
            .collect(),
    };

    if config.scripts.is_empty() {
        start_proxy(config.listen, targets)?;
    } else {
        if config.scripts.len() > 1 {
            eprintln!("Warning: multiple scripts are not supported yet");
            return Ok(());
        }

        let (send_exec_request, receive_exec_request) = mpsc::channel();
        let (send_exec_response, receive_exec_response) = crossbeam::channel::unbounded();

        thread::spawn(move || {
            exec_worker(receive_exec_request, send_exec_response);
        });

        let script_def = config.scripts.first().unwrap();

        let script = script_def.script.clone();
        let exec_fn = Box::new(move |request: HookRequest| {
            send_exec_request
                .send(ExecRequest {
                    script: script.clone(),
                    direction: request.direction,
                    target_name: request.target_name,
                    data: request.data,
                })
                .unwrap();
            let locked_receive_exec_response = receive_exec_response.clone();
            let result = locked_receive_exec_response.recv().unwrap();
            let data = result.unwrap().data;
            let response = data.map(HookResponse::new);

            Ok(response)
        }) as HookFn;

        let hooks = vec![Hook::builder(exec_fn).build()];

        start_proxy_with_hooks(config.listen, targets, hooks)?;
    }

    Ok(())
}
