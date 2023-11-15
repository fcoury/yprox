#[derive(Debug)]
pub enum Error {
    ScriptError { target: String, cause: String },
    UnexpectedError(String),
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Self::UnexpectedError(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
