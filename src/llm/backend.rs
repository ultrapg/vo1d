use crate::models::message::{LlmResponse, Message, Tool};
use std::pin::Pin;
use futures_util::Stream;

/// Unified trait for all LLM backends.
#[async_trait::async_trait]
pub trait LlmBackend: Send + Sync {
    /// Human-readable backend name
    fn name(&self) -> &str;

    /// Whether this backend supports native tool calling
    fn supports_tools(&self) -> bool;

    /// Maximum context length in tokens
    fn context_length(&self) -> usize;

    /// Send a chat completion request (non-streaming)
    async fn chat(
        &self,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<LlmResponse, Box<dyn std::error::Error + Send + Sync>>;

    /// Send a streaming chat completion request
    async fn stream_chat(
        &self,
        messages: &[Message],
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send + Sync>>> + Send>>,
        Box<dyn std::error::Error + Send + Sync>,
    >;
}

use std::sync::Arc;

/// Unified client that dispatches to the appropriate backend.
pub struct UnifiedClient {
    backend: Arc<dyn LlmBackend>,
}

impl UnifiedClient {
    pub fn new(backend: Arc<dyn LlmBackend>) -> Self {
        Self { backend }
    }

    pub fn backend(&self) -> &Arc<dyn LlmBackend> {
        &self.backend
    }

    pub async fn chat(
        &self,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<LlmResponse, Box<dyn std::error::Error + Send + Sync>> {
        self.backend.chat(messages, tools).await
    }

    pub async fn stream_chat(
        &self,
        messages: &[Message],
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send + Sync>>> + Send>>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        self.backend.stream_chat(messages).await
    }

    pub fn supports_tools(&self) -> bool {
        self.backend.supports_tools()
    }

    pub fn context_length(&self) -> usize {
        self.backend.context_length()
    }
}
