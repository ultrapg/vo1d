use crate::llm::backend::LlmBackend;
use crate::models::message::{LlmResponse, Message, TokenUsage, Tool};
use async_trait::async_trait;
use std::pin::Pin;
use futures_util::{Stream, StreamExt};

/// Custom OpenAI-compatible REST API backend.
pub struct CustomBackend {
    base_url: String,
    api_key: String,
    model: String,
}

impl CustomBackend {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self {
            base_url,
            api_key,
            model,
        }
    }
}

#[async_trait]
impl LlmBackend for CustomBackend {
    fn name(&self) -> &str {
        "Custom API"
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn context_length(&self) -> usize {
        8192
    }

    async fn chat(
        &self,
        messages: &[Message],
        _tools: Option<&[Tool]>,
    ) -> Result<LlmResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "temperature": 0.7,
            "max_tokens": 2048,
        });

        let response = client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;
        let content = result["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();

        let usage = result.get("usage").map(|u| TokenUsage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
        });

        Ok(LlmResponse {
            content,
            tool_calls: None,
            usage,
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
            "model": self.model,
            "messages": messages,
            "stream": true,
            "temperature": 0.7,
            "max_tokens": 2048,
        });

        let response = client.post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
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
