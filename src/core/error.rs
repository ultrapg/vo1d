use std::error::Error as StdError;
use thiserror::Error;

/// A wrapper to preserve error source chains when converting from `anyhow::Error`.
#[derive(Debug)]
struct SourceError(String);

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StdError for SourceError {}

/// Unified error types for the VO1D system.
#[derive(Error, Debug)]
pub enum Vo1dError {
    #[error("Configuration error: {0}")]
    Config(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Model error: {0}")]
    Model(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("Security error: {0}")]
    Security(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("Tool execution error: {0}")]
    ToolExecution(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("LLM backend error: {0}")]
    LlmBackend(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("Parser error: {0}")]
    Parser(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("Session error: {0}")]
    Session(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("Download error: {0}")]
    Download(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Network error: {0}")]
    Network(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("Not found: {0}")]
    NotFound(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("Permission denied: {0}")]
    PermissionDenied(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("Timeout: {0}")]
    Timeout(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("Unsupported: {0}")]
    Unsupported(String, #[source] Option<Box<dyn StdError + Send + Sync>>),

    #[error("Internal error: {0}")]
    Internal(String, #[source] Option<Box<dyn StdError + Send + Sync>>),
}

impl From<anyhow::Error> for Vo1dError {
    fn from(err: anyhow::Error) -> Self {
        let msg = err.to_string();
        let source: Option<Box<dyn StdError + Send + Sync>> = err
            .source()
            .map(|s| Box::new(SourceError(s.to_string())) as Box<dyn StdError + Send + Sync>);
        Vo1dError::Internal(msg, source)
    }
}

impl From<toml::de::Error> for Vo1dError {
    fn from(err: toml::de::Error) -> Self {
        Vo1dError::Config(err.to_string(), None)
    }
}

impl From<toml::ser::Error> for Vo1dError {
    fn from(err: toml::ser::Error) -> Self {
        Vo1dError::Config(err.to_string(), None)
    }
}
