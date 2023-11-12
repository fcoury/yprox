use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
};

use clap::Parser;
use mlua::prelude::*;
use yprox::{start, start_modifying};

#[derive(Parser)]
struct Cli {
    /// The address to listen on
    from_addr: String,

    /// The address to forward to
    to_addr: Vec<String>,

    /// Modifying script
    #[clap(short, long)]
    script: Option<String>,
}

struct EvalMessage {
    script: String,
    data: Vec<u8>,
}

fn eval_worker(rx: Receiver<EvalMessage>, tx: Sender<Vec<u8>>) {
    let lua = Lua::new();
    for message in rx {
        let result = modify_with_lua(&lua, message.data, message.script);
        tx.send(result.unwrap_or(vec![])).unwrap();
    }
}

fn modify_with_lua(lua: &Lua, data: Vec<u8>, script: String) -> Result<Vec<u8>, LuaError> {
    let globals = lua.globals();
    let modify_fn: LuaFunction = globals.get(script)?;
    let result: LuaValue = modify_fn.call(data)?;
    let result: Vec<u8> = result
        .as_table()
        .and_then(|table| {
            let mut result = vec![];
            let len = match table.len() {
                Ok(len) => len,
                Err(err) => return Some(Err(err)),
            };
            for i in 1..=len {
                let value: Result<LuaTable, _> = table.get(i);
                match value {
                    Ok(value) => {
                        let value: u8 = value.get(1).unwrap();
                        result.push(value);
                        continue;
                    }
                    Err(err) => {
                        return Some(Err(err));
                    }
                }
            }
            Some(Ok(result))
        })
        .unwrap_or(Ok(vec![]))?;
    Ok(result)
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();

    println!("Listening on {} -> {:?}", args.from_addr, args.to_addr);
    match args.script {
        Some(script) => {
            let (send_eval, receive_eval) = mpsc::channel();
            let (send_result, receive_result) = mpsc::channel();

            thread::spawn(move || {
                eval_worker(receive_eval, send_result);
            });

            println!("Using script: {}", script);
            let receive_result = Arc::new(Mutex::new(receive_result));
            let modify_fn = Arc::new(Box::new(move |data: Vec<u8>| -> Vec<u8> {
                send_eval
                    .send(EvalMessage {
                        script: script.clone(),
                        data,
                    })
                    .expect("Failed to send eval message");
                let locked_receive_result = receive_result.lock().unwrap();
                locked_receive_result
                    .recv()
                    .expect("Failed to receive result")
            })
                as Box<dyn Fn(Vec<u8>) -> Vec<u8> + Send + Sync>);

            start_modifying(args.from_addr, args.to_addr, Some(modify_fn)).await;
        }
        None => start(args.from_addr, args.to_addr).await,
    }
}
