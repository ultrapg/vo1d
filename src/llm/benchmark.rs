use crate::config::settings::BuiltinConfig;
use crate::llm::backend::LlmBackend;
use crate::llm::builtin::BuiltinBackend;
use crate::models::message::Message;
use anyhow::Result;
use std::path::Path;
use std::time::Instant;

const BENCHMARK_SYSTEM: &str = "You are a concise assistant.";
const BENCHMARK_PROMPT: &str =
    "Explain what neural networks are and how they learn from data. Keep it to 2-3 sentences.";
const WARMUP_PROMPT: &str = "Say 'Hello' and nothing else.";
const BENCHMARK_MAX_TOKENS: u32 = 128;

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub label: String,
    pub load_ms: u64,
    pub inference_ms: u64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub tokens_per_second: f64,
}

pub async fn run_gpu_vs_cpu(
    model_path: &Path,
    base_config: &BuiltinConfig,
) -> Result<(BenchmarkResult, BenchmarkResult)> {
    let gpu_cfg = BuiltinConfig {
        gpu_layers: -1,
        no_gpu: false,
        max_tokens: BENCHMARK_MAX_TOKENS,
        ..base_config.clone()
    };
    println!("  GPU (Vulkan) benchmark...");
    let gpu = run_single("GPU (Vulkan)", &gpu_cfg, model_path).await?;

    let cpu_cfg = BuiltinConfig {
        gpu_layers: 0,
        no_gpu: true,
        max_tokens: BENCHMARK_MAX_TOKENS,
        ..base_config.clone()
    };
    println!("  CPU benchmark...");
    let cpu = run_single("CPU", &cpu_cfg, model_path).await?;

    Ok((gpu, cpu))
}

async fn run_single(
    label: &str,
    config: &BuiltinConfig,
    model_path: &Path,
) -> Result<BenchmarkResult> {
    let load_start = Instant::now();
    let backend = BuiltinBackend::new(config.clone(), model_path.to_path_buf(), false);
    backend
        .load()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to load model: {}", e))?;
    let load_ms = load_start.elapsed().as_millis() as u64;

    let system = Message::system(BENCHMARK_SYSTEM);

    // Warmup
    backend
        .chat(&[system.clone(), Message::user(WARMUP_PROMPT)], None)
        .await
        .map_err(|e| anyhow::anyhow!("Warmup failed: {}", e))?;

    // Timed inference
    let messages = [system, Message::user(BENCHMARK_PROMPT)];
    let inf_start = Instant::now();
    let response = backend
        .chat(&messages, None)
        .await
        .map_err(|e| anyhow::anyhow!("Benchmark inference failed: {}", e))?;
    let inf_ms = inf_start.elapsed().as_millis() as u64;

    let usage = response.usage.unwrap_or(crate::models::message::TokenUsage {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
    });
    let ct = usage.completion_tokens;
    let pt = usage.prompt_tokens;
    let tps = if inf_ms > 0 {
        ct as f64 / (inf_ms as f64 / 1000.0)
    } else {
        0.0
    };

    backend.unload().await;

    Ok(BenchmarkResult {
        label: label.to_string(),
        load_ms,
        inference_ms: inf_ms,
        prompt_tokens: pt,
        completion_tokens: ct,
        tokens_per_second: tps,
    })
}

pub fn print_comparison(gpu: &BenchmarkResult, cpu: &BenchmarkResult) {
    println!();
    println!("═{}═", "═".repeat(58));
    println!(" {:^58}", "BENCHMARK RESULTS");
    println!("{}", "═".repeat(60));
    println!(
        " {:<20} {:>15} {:>15}",
        "Metric", "GPU (Vulkan)", "CPU"
    );
    println!("{}", "─".repeat(60));
    println!(
        " {:<20} {:>15} {:>15}",
        "Load time",
        format!("{} ms", gpu.load_ms),
        format!("{} ms", cpu.load_ms),
    );
    println!(
        " {:<20} {:>15} {:>15}",
        "Inference time",
        format!("{} ms", gpu.inference_ms),
        format!("{} ms", cpu.inference_ms),
    );
    println!(
        " {:<20} {:>15.1} {:>15.1}",
        "Tokens/sec", gpu.tokens_per_second, cpu.tokens_per_second,
    );
    println!(
        " {:<20} {:>15} {:>15}",
        "Prompt tokens",
        gpu.prompt_tokens,
        cpu.prompt_tokens,
    );
    println!(
        " {:<20} {:>15} {:>15}",
        "Generated tokens",
        gpu.completion_tokens,
        cpu.completion_tokens,
    );
    println!("{}", "═".repeat(60));

    let winner = if cpu.tokens_per_second >= gpu.tokens_per_second {
        "CPU"
    } else {
        "GPU (Vulkan)"
    };
    println!();
    println!(
        "  Recommendation: {} is faster ({:.1}x).",
        winner,
        if cpu.tokens_per_second >= gpu.tokens_per_second {
            cpu.tokens_per_second / gpu.tokens_per_second
        } else {
            gpu.tokens_per_second / cpu.tokens_per_second
        }
    );

    if winner == "CPU" {
        println!("  Set `no_gpu = true` in settings to use CPU by default.");
    } else {
        println!("  GPU offloading is working well. Keep current settings.");
    }
    println!();
}
