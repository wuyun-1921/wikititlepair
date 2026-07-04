use thiserror::Error;

#[derive(Error, Debug)]
pub enum WikiDictError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, WikiDictError>;
