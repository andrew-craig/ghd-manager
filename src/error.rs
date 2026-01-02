use thiserror::Error;

#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("Docker error: {0}")]
    Docker(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, MonitorError>;
