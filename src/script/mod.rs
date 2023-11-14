use std::sync::mpsc;

use mlua::{Function, Lua, Table};

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

impl ExecResponse {
    fn with_data(data: Box<[u8]>) -> Self {
        Self { data: Some(data) }
    }
}

pub fn exec_worker(
    receive_exec_request: mpsc::Receiver<ExecRequest>,
    send_exec_response: mpsc::Sender<Result<ExecResponse>>,
) {
    let lua = Lua::new();
    for message in receive_exec_request {
        let response = eval(&lua, message.script, message.target, message.data);
        send_exec_response
            .send(response)
            .expect("send_exec_response");
    }
}

fn eval(lua: &Lua, script: String, target: String, data: Box<[u8]>) -> Result<ExecResponse> {
    println!("script: {}", script);
    let handler: Function = lua.load(&script).eval()?;
    let data_table: Table = lua.create_table()?;
    for (i, byte) in data.iter().enumerate() {
        data_table.set(i + 1, *byte)?;
    }
    let result = handler.call::<_, mlua::Value>((target.clone(), data_table))?;

    Ok(match result {
        mlua::Value::String(s) => ExecResponse::with_data(s.as_bytes().to_vec().into_boxed_slice()),
        mlua::Value::Nil => ExecResponse::default(),
        result => {
            return Err(error::Error::InvalidScriptResult {
                target,
                result: result.to_string()?,
            })
        }
    })
}
