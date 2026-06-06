pub mod backend;
pub mod registry;
pub mod downloader;
pub mod builtin;

#[cfg(feature = "ollama")]
pub mod ollama;
#[cfg(feature = "lmstudio")]
pub mod lmstudio;
#[cfg(feature = "llamacpp-server")]
pub mod llamacpp_server;
#[cfg(feature = "custom-api")]
pub mod custom;

pub use backend::LlmBackend;
pub use registry::ModelRegistry;
