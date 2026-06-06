use thiserror::Error;

/// Unified error types for the VO1D system.
#[derive(Error, Debug)]
pub enum Vo1dError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Model error: {0}")]
    Model(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("LLM backend error: {0}")]
    LlmBackend(String),

    #[error("Parser error: {0}")]
    Parser(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Download error: {0}")]
    Download(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Unsupported: {0}")]
    Unsupported(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for Vo1dError {
    fn from(err: anyhow::Error) -> Self {
        Vo1dError::Internal(err.to_string())
    }
}

impl From<toml::de::Error> for Vo1dError {
    fn from(err: toml::de::Error) -> Self {
        Vo1dError::Config(err.to_string())
    }
}

impl From<toml::ser::Error> for Vo1dError {
    fn from(err: toml::ser::Error) -> Self {
        Vo1dError::Config(err.to_string())
    }
}
