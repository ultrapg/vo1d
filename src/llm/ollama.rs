use crate::llm::backend::LlmBackend;
use crate::models::message::{LlmResponse, Message, TokenUsage, Tool, ToolCall};
use async_trait::async_trait;
use std::pin::Pin;
use futures_util::{Stream, StreamExt};

/// Ollama HTTP API backend.
pub struct OllamaBackend {
    base_url: String,
    model: String,
}

impl OllamaBackend {
    pub fn new(base_url: Option<String>, model: String) -> Self {
        Self {
            base_url: base_url.unwrap_or_else(|| "http://localhost:11434".to_string()),
            model,
        }
    }

    /// Check if Ollama is running.
    pub async fn detect() -> Option<String> {
        let url = "http://localhost:11434/api/tags";
        match reqwest::get(url).await {
            Ok(resp) if resp.status().is_success() => Some("http://localhost:11434".to_string()),
            _ => None,
        }
    }
}

#[async_trait]
impl LlmBackend for OllamaBackend {
    fn name(&self) -> &str {
        "Ollama"
    }

    fn supports_tools(&self) -> bool {
        true // Ollama supports tool calling via the API
    }

    fn context_length(&self) -> usize {
        8192
    }

    async fn chat(
        &self,
        messages: &[Message],
        _tools: Option<&[Tool]>,
    ) -> Result<LlmResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/api/chat", self.base_url);
        let client = reqwest::Client::new();

        let mut body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
        });

        if let Some(tools) = _tools {
            body["tools"] = serde_json::to_value(tools)?;
        }

        let response = client.post(&url)
            .json(&body)
            .send()
            .await?;

        let result: serde_json::Value = response.json().await?;

        let content = result["message"]["content"].as_str().unwrap_or("").to_string();
        let tool_calls = result["message"]["tool_calls"].as_array().map(|arr| {
            arr.iter().map(|tc| ToolCall {
                id: tc["id"].as_str().unwrap_or("call_unknown").to_string(),
                call_type: "function".to_string(),
                function: crate::models::message::FunctionCall {
                    name: tc["function"]["name"].as_str().unwrap_or("").to_string(),
                    arguments: tc["function"]["arguments"].to_string(),
                },
            }).collect()
        });

        let usage = result.get("usage").map(|u| TokenUsage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
        });

        Ok(LlmResponse { content, tool_calls, usage })
    }

    async fn stream_chat(
        &self,
        messages: &[Message],
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send + Sync>>> + Send>>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let url = format!("{}/api/chat", self.base_url);
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": true,
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
