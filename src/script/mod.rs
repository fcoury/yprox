use std::sync::mpsc;

use rhai::{packages::Package, Dynamic, Engine, Scope};
use rhai_fs::FilesystemPackage;
use yprox::hooks::Direction;

use self::error::Result;

pub mod error;

pub struct ExecRequest {
    pub script: String,
    pub direction: Direction,
    pub target_name: String,
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
    let mut engine = Engine::new();

    let package = FilesystemPackage::new();
    package.register_into_engine(&mut engine);

    for message in receive_exec_request {
        let mut scope = Scope::new();
        let data = message.data.into_vec();

        scope.push("direction", message.direction);
        scope.push(
            "trigger",
            if message.direction.from_client() {
                "client"
            } else {
                "target"
            },
        );
        scope.push("target", message.target_name.clone());
        scope.push("data", data);

        let response = engine
            .eval_with_scope::<Dynamic>(&mut scope, &message.script)
            .map_err(|err| {
                eprintln!("Error running script: {:?}", err);
                error::Error::ScriptError {
                    target: message.target_name.clone(),
                    cause: err.to_string(),
                }
            });

        let Ok(response) = response else {
            send_exec_response
                .send(Err(response.unwrap_err()))
                .expect("send_exec_response");
            continue;
        };

        if response.is_blob() {
            let data = response.into_blob().expect("is blob");
            send_exec_response
                .send(Ok(ExecResponse {
                    data: Some(data.into_boxed_slice()),
                }))
                .expect("send_exec_response");
            continue;
        }

        if response.is_array() {
            let data = response.into_array().expect("is array");
            let data = data
                .into_iter()
                .map(|x| x.as_int().unwrap() as u8)
                .collect::<Vec<_>>()
                .into_boxed_slice();
            send_exec_response
                .send(Ok(ExecResponse { data: Some(data) }))
                .expect("send_exec_response");
            continue;
        }

        if response.is_string() {
            let data = response.into_string().expect("is string");
            let data = data.into_bytes().into_boxed_slice();
            send_exec_response
                .send(Ok(ExecResponse { data: Some(data) }))
                .expect("send_exec_response");
            continue;
        }

        if response.is::<()>() {
            send_exec_response
                .send(Ok(ExecResponse { data: None }))
                .expect("send_exec_response");
            continue;
        }

        send_exec_response
            .send(Err(error::Error::ScriptError {
                target: message.target_name,
                cause: format!("Script returned an invalid value: {response:?}"),
            }))
            .expect("send_exec_response");
    }
}
