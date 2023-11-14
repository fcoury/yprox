#[derive(Debug)]
pub enum Error {
    InvalidScriptResult { target: String, result: String },
    ScriptExecutionError(String),
}

impl From<mlua::Error> for Error {
    fn from(err: mlua::Error) -> Self {
        Self::ScriptExecutionError(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
