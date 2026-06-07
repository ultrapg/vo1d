use crate::llm::backend::LlmBackend;
use crate::models::message::{LlmResponse, Message, Tool};
use async_trait::async_trait;
use std::pin::Pin;
use futures_util::{Stream, StreamExt};

/// llama.cpp server backend (OpenAI-compatible API).
pub struct LlamaCppServerBackend {
    base_url: String,
    _model: String,
}

impl LlamaCppServerBackend {
    pub fn new(base_url: Option<String>, model: String) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:8080".to_string()),
            _model: model,
        }
    }

    /// Check if llama.cpp server is running.
    pub async fn detect() -> Option<String> {
        let url = "http://localhost:8080/v1/models";
        match reqwest::get(url).await {
            Ok(resp) if resp.status().is_success() => Some("http://localhost:8080".to_string()),
            _ => None,
        }
    }
}

#[async_trait]
impl LlmBackend for LlamaCppServerBackend {
    fn name(&self) -> &str {
        "llama.cpp Server"
    }

    fn supports_tools(&self) -> bool {
        false
    }

    fn context_length(&self) -> usize {
        4096
    }

    async fn chat(
        &self,
        messages: &[Message],
        _tools: Option<&[Tool]>,
    ) -> Result<LlmResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 2048,
        });

        let response = client.post(&url)
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        let content = result["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();

        Ok(LlmResponse {
            content,
            tool_calls: None,
            usage: None,
        })
    }

    async fn stream_chat(
        &self,
        messages: &[Message],
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send + Sync>>> + Send>>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "messages": messages,
            "stream": true,
            "temperature": 0.7,
            "max_tokens": 2048,
        });

        let response = client.post(&url)
            .json(&body)
            .send()
            .await?;

        let stream = response.bytes_stream().map(|chunk| {
            chunk
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                .and_then(|bytes| {
                    String::from_utf8(bytes.to_vec())
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
                })
        });

        Ok(Box::pin(stream))
    }
}
