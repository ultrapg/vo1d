use crate::config::settings::BuiltinConfig;
use crate::llm::backend::LlmBackend;
use crate::models::message::{LlmResponse, Message, TokenUsage, Tool};
use anyhow::Result;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use futures_util::Stream;

#[cfg(feature = "llamacpp-builtin")]
use std::num::NonZeroU32;
#[cfg(feature = "llamacpp-builtin")]
use anyhow::Context;
#[cfg(feature = "llamacpp-builtin")]
use {
    llama_cpp_2::context::params::LlamaContextParams,
    llama_cpp_2::llama_backend::LlamaBackend,
    llama_cpp_2::llama_batch::LlamaBatch,
    llama_cpp_2::model::params::LlamaModelParams,
    llama_cpp_2::model::{AddBos, LlamaModel},
    llama_cpp_2::sampling::LlamaSampler,
};

pub struct BuiltinBackend {
    config: BuiltinConfig,
    model_path: std::path::PathBuf,
    model: Arc<Mutex<Option<LlamaModelHandle>>>,
    native_tools: bool,
}

struct LlamaModelHandle {
    #[cfg(feature = "llamacpp-builtin")]
    backend: LlamaBackend,
    #[cfg(feature = "llamacpp-builtin")]
    model: LlamaModel,
}

#[cfg(feature = "llamacpp-builtin")]
struct GeneratedOutput {
    text: String,
    prompt_tokens: u32,
    completion_tokens: u32,
}

impl BuiltinBackend {
    pub fn new(config: BuiltinConfig, model_path: std::path::PathBuf, native_tools: bool) -> Self {
        Self {
            config,
            model_path,
            model: Arc::new(Mutex::new(None)),
            native_tools,
        }
    }

    pub async fn load(&self) -> Result<()> {
        if !self.model_path.exists() {
            anyhow::bail!("Model file not found: {}", self.model_path.display());
        }

        #[cfg(not(feature = "llamacpp-builtin"))]
        {
            let _ = &self.model_path;
            anyhow::bail!("Built-in llama.cpp not compiled. Rebuild with: cargo build --features llamacpp-builtin (or --features full to enable all backends).");
        }

        #[cfg(feature = "llamacpp-builtin")]
        {
            tracing::info!(
                "Loading model: {} (requested GPU layers: {}, {} CPU threads)",
                self.model_path.display(),
                self.config.gpu_layers,
                self.config.threads,
            );

            let path = self.model_path.clone();
            let requested_layers = self.config.gpu_layers;
            let no_gpu = self.config.no_gpu;

            let handle = tokio::task::spawn_blocking(move || -> Result<LlamaModelHandle> {
                let backend =
                    LlamaBackend::init().map_err(|e| anyhow::anyhow!("Backend init: {:?}", e))?;

                let gpu_available = backend.supports_gpu_offload() && !no_gpu;

                if no_gpu && backend.supports_gpu_offload() {
                    tracing::info!("GPU available but disabled by no_gpu setting. Using CPU.");
                }

                let effective_layers = if gpu_available {
                    if requested_layers > 0 {
                        requested_layers
                    } else {
                        -1 // all layers
                    }
                } else if requested_layers != 0 {
                    tracing::warn!(
                        "GPU offload requested ({}) but not available at runtime. Falling back to CPU.",
                        requested_layers
                    );
                    0
                } else {
                    0
                };

                let mut model_params = LlamaModelParams::default();
                match effective_layers {
                    -1 => {} // all layers, leave C default
                    0 => { model_params = model_params.with_n_gpu_layers(0); }
                    n if n > 0 => { model_params = model_params.with_n_gpu_layers(n as u32); }
                    _ => {}
                }

                let model = LlamaModel::load_from_file(&backend, &path, &model_params)
                    .map_err(|e| anyhow::anyhow!("Model load: {:?}", e))?;

                Ok(LlamaModelHandle { backend, model })
            })
            .await
            .context("spawn_blocking panicked")??;

            let mut guard = self.model.lock().unwrap();
            *guard = Some(handle);
            tracing::info!("Model loaded successfully");
            Ok(())
        }
    }

    pub async fn is_loaded(&self) -> bool {
        self.model.lock().unwrap().is_some()
    }

    pub async fn unload(&self) {
        let mut guard = self.model.lock().unwrap();
        *guard = None;
        tracing::info!("Model unloaded");
    }

    #[cfg(feature = "llamacpp-builtin")]
    fn generate_blocking(
        handle: &LlamaModelHandle,
        config: &BuiltinConfig,
        prompt: &str,
        token_sink: Option<&tokio::sync::mpsc::Sender<
            Result<String, Box<dyn std::error::Error + Send + Sync>>,
        >>,
    ) -> Result<GeneratedOutput> {
        let _stderr_guard = crate::utils::stderr_guard::StderrGuard::suppress();
        let ctx_size =
            NonZeroU32::new(config.context_size).unwrap_or(NonZeroU32::new(4096).unwrap());

        let batch_size = if config.batch_size > 0 { config.batch_size } else { 4096 };
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(ctx_size))
            .with_n_threads(config.threads)
            .with_n_batch(batch_size.min(ctx_size.get()));

        let mut ctx = handle
            .model
            .new_context(&handle.backend, ctx_params)
            .map_err(|e| anyhow::anyhow!("Context: {:?}", e))?;

        let tokens = handle
            .model
            .str_to_token(prompt, AddBos::Never)
            .map_err(|e| anyhow::anyhow!("Tokenize: {:?}", e))?;

        let n_prompt = tokens.len() as u32;
        let max_tokens = config.max_tokens as usize;

        let mut batch = LlamaBatch::new(tokens.len(), 1);
        for (i, &token) in tokens.iter().enumerate() {
            batch
                .add(token, i as i32, &[0], i == tokens.len() - 1)
                .map_err(|e| anyhow::anyhow!("Batch add: {:?}", e))?;
        }

        ctx.decode(&mut batch)
            .map_err(|e| anyhow::anyhow!("Decode: {:?}", e))?;

        let temp = if config.temperature <= 0.0 {
            0.0
        } else {
            config.temperature
        };
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::top_k(config.top_k as i32),
            LlamaSampler::top_p(config.top_p, 1),
            LlamaSampler::temp(temp),
            LlamaSampler::dist(0),
        ]);

        let mut output = String::new();
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut gen_batch = LlamaBatch::new(1, 1);

        let mut step: u32 = 0;
        while (step as usize) < max_tokens {
            let token = sampler.sample(&ctx, -1);

            if handle.model.is_eog_token(token) {
                break;
            }

            let piece = handle
                .model
                .token_to_piece(token, &mut decoder, false, None)
                .map_err(|e| anyhow::anyhow!("Token decode: {:?}", e))?;

            if let Some(sink) = token_sink {
                if sink.blocking_send(Ok(piece.clone())).is_err() {
                    break;
                }
            }

            output.push_str(&piece);

            gen_batch.clear();
            gen_batch
                .add(token, (n_prompt + step) as i32, &[0], true)
                .map_err(|e| anyhow::anyhow!("Batch add: {:?}", e))?;

            ctx.decode(&mut gen_batch)
                .map_err(|e| anyhow::anyhow!("Decode: {:?}", e))?;

            step += 1;
        }

        Ok(GeneratedOutput {
            text: output,
            prompt_tokens: n_prompt,
            completion_tokens: step,
        })
    }

    async fn generate(&self, prompt: &str) -> Result<(String, TokenUsage)> {
        #[cfg(not(feature = "llamacpp-builtin"))]
        {
            let _ = prompt;
            anyhow::bail!("Built-in llama.cpp not compiled. Rebuild with: cargo build --features llamacpp-builtin");
        }

        #[cfg(feature = "llamacpp-builtin")]
        {
            let model_arc = self.model.clone();
            let config = self.config.clone();
            let prompt = prompt.to_string();

            let output = tokio::task::spawn_blocking(move || -> Result<GeneratedOutput> {
                let guard = model_arc.lock().unwrap();
                let handle = guard
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Model not loaded"))?;
                Self::generate_blocking(handle, &config, &prompt, None)
            })
            .await
            .context("spawn_blocking panicked")??;

            let usage = TokenUsage {
                prompt_tokens: output.prompt_tokens,
                completion_tokens: output.completion_tokens,
                total_tokens: output.prompt_tokens + output.completion_tokens,
            };

            Ok((output.text, usage))
        }
    }

    async fn generate_stream(
        &self,
        prompt: String,
        tx: tokio::sync::mpsc::Sender<
            Result<String, Box<dyn std::error::Error + Send + Sync>>,
        >,
    ) {
        #[cfg(not(feature = "llamacpp-builtin"))]
        {
            let _ = (prompt, tx);
        }

        #[cfg(feature = "llamacpp-builtin")]
        {
            let model_arc = self.model.clone();
            let config = self.config.clone();

            tokio::task::spawn_blocking(move || {
                let guard = model_arc.lock().unwrap();
                let handle = match guard.as_ref() {
                    Some(h) => h,
                    None => {
                        let _ = tx.blocking_send(Err("Model not loaded".into()));
                        return;
                    }
                };

                if let Err(e) = Self::generate_blocking(handle, &config, &prompt, Some(&tx)) {
                    let _ = tx.blocking_send(Err(format!("Generation failed: {}", e).into()));
                }
            });
        }
    }
}

#[async_trait::async_trait]
impl LlmBackend for BuiltinBackend {
    fn name(&self) -> &str {
        "llama.cpp (built-in)"
    }

    fn supports_tools(&self) -> bool {
        self.native_tools
    }

    fn context_length(&self) -> usize {
        self.config.context_size as usize
    }

    async fn chat(
        &self,
        messages: &[Message],
        tools: Option<&[Tool]>,
    ) -> Result<LlmResponse, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = if self.native_tools && tools.is_some_and(|t| !t.is_empty()) {
            build_chat_prompt_with_tools(messages, tools.unwrap(), self.config.context_size as usize)
        } else {
            build_chat_prompt(messages, self.config.context_size as usize)
        };

        let (output, usage) = self
            .generate(&prompt)
            .await
            .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;

        let tool_calls = if self.native_tools && tools.is_some_and(|t| !t.is_empty()) {
            parse_tool_calls(&output)
        } else {
            None
        };

        Ok(LlmResponse {
            content: if tool_calls.is_some() { String::new() } else { output },
            tool_calls,
            usage: Some(usage),
        })
    }

    async fn stream_chat(
        &self,
        messages: &[Message],
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<String, Box<dyn std::error::Error + Send + Sync>>> + Send>>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let prompt = build_chat_prompt(messages, self.config.context_size as usize);
        let (tx, rx) = tokio::sync::mpsc::channel::<
            Result<String, Box<dyn std::error::Error + Send + Sync>>,
        >(64);

        let tx_gen = tx.clone();
        drop(tx);

        self.generate_stream(prompt, tx_gen).await;

        let stream = futures_util::stream::unfold(rx, |mut rx| async {
            rx.recv().await.map(|item| (item, rx))
        });

        Ok(Box::pin(stream))
    }
}

fn build_chat_prompt(messages: &[Message], max_context: usize) -> String {
    let mut parts = Vec::new();

    for msg in messages {
        let role = match msg.role.as_str() {
            "system" => "system",
            "user" => "user",
            "assistant" => "assistant",
            "tool" => "tool",
            _ => "user",
        };
        parts.push(format!("<|im_start|>{}\n{}<|im_end|>", role, msg.content));
    }

    parts.push("<|im_start|>assistant\n".to_string());

    let full = parts.join("\n");

    if full.len() > max_context * 4 {
        let start = full.len() - max_context * 4;
        format!("...{}", &full[start..])
    } else {
        full
    }
}

fn build_chat_prompt_with_tools(messages: &[Message], tools: &[Tool], max_context: usize) -> String {
    let mut parts = Vec::new();
    let system_msg = messages.iter().find(|m| m.role == "system");

    for msg in messages {
        let role = match msg.role.as_str() {
            "system" => "system",
            "user" => "user",
            "assistant" => "assistant",
            "tool" => "tool",
            _ => "user",
        };
        parts.push(format!("<|im_start|>{}\n{}<|im_end|>", role, msg.content));
    }

    // Append concise tool definitions after system message
    if let Some(ref sys) = system_msg {
        let mut tool_block = String::from("\n\nUse these tools by responding with: {\"name\": \"tool_name\", \"arguments\": {...}}\n\nTools:");
        for tool in tools {
            tool_block.push_str(&format!("\n- {}: {}", tool.name, tool.description));
        }
        // Replace original system message with enhanced one
        if let Some(first_sys) = parts.iter_mut().find(|p| p.starts_with("<|im_start|>system\n")) {
            *first_sys = format!("<|im_start|>system\n{}{}<|im_end|>", sys.content, tool_block);
        }
    }

    parts.push("<|im_start|>assistant\n".to_string());

    let full = parts.join("\n");

    if full.len() > max_context * 4 {
        let start = full.len() - max_context * 4;
        format!("...{}", &full[start..])
    } else {
        full
    }
}

fn parse_tool_calls(output: &str) -> Option<Vec<crate::models::message::ToolCall>> {
    let re = regex::Regex::new(r"```(?:json)?\s*([\s\S]*?)```").ok()?;
    // Try raw text first, then each markdown code block
    for candidate in std::iter::once(output).chain(
        re.captures_iter(output).filter_map(|c| c.get(1)).map(|m| m.as_str()),
    ) {
        let trimmed = candidate.trim();
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if let (Some(name), Some(args)) = (
                val.get("name").and_then(|v| v.as_str()),
                val.get("arguments"),
            ) {
                return Some(vec![crate::models::message::ToolCall::new(
                    name,
                    args.to_string(),
                )]);
            }
        }
        // Handle multiple JSON objects on separate lines (model may output >1 action)
        let mut calls = Vec::new();
        for line in trimmed.lines() {
            let line = line.trim();
            if line.starts_with('{') && line.ends_with('}') {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                    if let (Some(name), Some(args)) = (
                        val.get("name").and_then(|v| v.as_str()),
                        val.get("arguments"),
                    ) {
                        calls.push(crate::models::message::ToolCall::new(name, args.to_string()));
                    }
                }
            }
        }
        if !calls.is_empty() {
            return Some(calls);
        }
    }
    None
}

pub async fn create_backend(
    config: &BuiltinConfig,
    model_path: &Path,
    native_tools: bool,
) -> Result<Box<dyn LlmBackend + Send + Sync>> {
    let backend = BuiltinBackend::new(config.clone(), model_path.to_path_buf(), native_tools);

    if let Err(e) = backend.load().await {
        tracing::warn!(
            "Failed to load model at startup: {}. Use 'vo1d models install <model_id>' to download one.",
            e
        );
    }

    Ok(Box::new(backend))
}
