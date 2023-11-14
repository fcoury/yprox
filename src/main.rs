use std::{
    fs,
    sync::{mpsc, Arc, Mutex},
    thread,
};

use clap::Parser;
use cli::Args;
use script::exec_worker;

pub use yprox::{
    error::Result,
    server::{Hook, HookDirection},
    start_proxy, start_proxy_with_hooks,
};

use crate::script::ExecRequest;

mod cli;
mod script;

fn main() -> Result<()> {
    let args = Args::parse();
    let mut targets = vec![args.main_target_addr];
    targets.extend(args.secondary_target_addrs);

    let targets = targets
        .into_iter()
        .enumerate()
        .map(|(i, target)| match target {
            cli::Target::Anon(addr) => Ok((format!("target{}", i + 1), addr)),
            cli::Target::Named(name, addr) => Ok((name, addr)),
        })
        .collect::<Result<Vec<_>>>()?;

    match args.script {
        Some(script) => {
            let (send_exec_request, receive_exec_request) = mpsc::channel();
            let (send_exec_response, receive_exec_response) = mpsc::channel();

            thread::spawn(move || {
                exec_worker(receive_exec_request, send_exec_response);
            });

            let script = fs::read_to_string(script)?;
            let receive_exec_response = Arc::new(Mutex::new(receive_exec_response));
            let exec_fn = Box::new(
                move |direction: HookDirection, target: String, data: Box<[u8]>| {
                    send_exec_request
                        .send(ExecRequest {
                            script: script.clone(),
                            direction,
                            target,
                            data,
                        })
                        .unwrap();
                    let locked_receive_exec_response = receive_exec_response.lock().unwrap();
                    let result = locked_receive_exec_response.recv().unwrap();
                    result.unwrap().data
                },
            );

            start_proxy_with_hooks(args.listen_addr, targets, vec![exec_fn])?
        }
        None => start_proxy(args.listen_addr, targets)?,
    }

    Ok(())
}
