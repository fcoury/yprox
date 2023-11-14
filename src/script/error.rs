#[derive(Debug)]
pub enum Error {
    ScriptError { target: String, cause: String },
}

pub type Result<T> = std::result::Result<T, Error>;
