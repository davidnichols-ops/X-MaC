use thiserror::Error;

#[derive(Error, Debug)]
pub enum XMacError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Engine error: {0}")]
    Engine(#[from] EngineError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("Path not found: {path}")]
    PathNotFound { path: String },
}

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Scan failed: {0}")]
    ScanFailed(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Prerequisite not met: {0}")]
    PrerequisiteNotMet(String),

    #[error("Timeout after {0:?}")]
    Timeout(std::time::Duration),
}

pub type Result<T> = std::result::Result<T, XMacError>;
