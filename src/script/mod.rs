use std::sync::mpsc;

use rhai::{Array, Dynamic, Engine, Scope};

use self::error::Result;

pub mod error;

pub struct ExecRequest {
    pub script: String,
    pub target: String,
    pub data: Box<[u8]>,
}

#[derive(Default)]
pub struct ExecResponse {
    pub data: Option<Box<[u8]>>,
}

pub fn exec_worker(
    receive_exec_request: mpsc::Receiver<ExecRequest>,
    send_exec_response: mpsc::Sender<Result<ExecResponse>>,
) {
    let engine = Engine::new();

    for message in receive_exec_request {
        let mut scope = Scope::new();
        let data = message
            .data
            .into_iter()
            .map(|x| Dynamic::from(x.clone() as i64))
            .collect::<Array>();

        scope.push("target", message.target.clone());
        scope.push("data", data);

        let response = engine
            .eval_with_scope::<Array>(&mut scope, &message.script)
            .map(|data| {
                data.into_iter()
                    .map(|x| x.as_int().unwrap() as u8)
                    .collect::<Vec<_>>()
            })
            .map(|data| ExecResponse {
                data: Some(data.into_boxed_slice()),
            })
            .map_err(|err| {
                eprintln!("Error running script: {:?}", err);
                error::Error::ScriptError {
                    target: message.target,
                    cause: err.to_string(),
                }
            });
        send_exec_response
            .send(response)
            .expect("send_exec_response");
    }
}
