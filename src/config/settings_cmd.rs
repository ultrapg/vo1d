use crate::config::{load_config, save_settings};
use crate::core::paths::Vo1dPaths;
use crate::llm::registry::ModelRegistry;
use anyhow::{bail, Context, Result};

/// List of all settable builtin config keys with descriptions and value kinds.
const SETTABLE_KEYS: &[(&str, &str, &str)] = &[
    ("gpu_layers", "int", "GPU layers to offload (-1 = all, 0 = CPU, N = first N layers)"),
    ("no_gpu", "bool", "Force CPU inference even if GPU is available (true/false)"),
    ("threads", "int", "CPU threads for inference (-1 = auto)"),
    ("batch_size", "int", "Batch size for prompt processing"),
    ("context_size", "int", "Context window size in tokens"),
    ("inference_timeout_secs", "int", "LLM inference timeout in seconds"),
    ("temperature", "float", "Sampling temperature (0.0-2.0)"),
    ("top_p", "float", "Top-p sampling (0.0-1.0)"),
    ("top_k", "int", "Top-k sampling (0 = disabled)"),
    ("repeat_penalty", "float", "Repeat penalty (0.5-5.0)"),
    ("max_tokens", "int", "Max tokens to generate per response"),
    ("default_model", "string", "Default model ID"),
    ("default_mode", "string", "Default security mode (safe/interactive/power-user/autonomous/unrestricted)"),
    ("default_behavior", "string", "Default behavior mode (normal/fix/research/refactor/tdd)"),
    ("command_timeout_secs", "int", "Command execution timeout in seconds"),
    ("max_iterations", "int", "Maximum plan iterations before forced halt"),
    ("max_backups", "int", "Maximum backups per file"),
];

/// Show all current settings as pretty TOML.
pub fn show_settings(paths: &Vo1dPaths) -> Result<()> {
    let settings = load_config(paths)?;
    let toml_str = toml::to_string_pretty(&settings)
        .context("Failed to serialize settings")?;
    println!("{}", toml_str);
    Ok(())
}

/// List all settable keys with descriptions.
pub fn list_keys() {
    println!("Available settings:");
    for (key, kind, desc) in SETTABLE_KEYS {
        println!("  {:<30} {}  {}", key, kind, desc);
    }
}

/// Set a single config key with validation.
pub fn set_setting(paths: &Vo1dPaths, registry: &ModelRegistry, key: &str, value: &str) -> Result<()> {
    let mut settings = load_config(paths)?;

    match key {
        // BuiltinConfig fields
        "gpu_layers" => {
            let v: i32 = value.parse().context("gpu_layers must be an integer")?;
            if v < -1 {
                bail!("gpu_layers must be -1 (all), 0 (CPU), or a positive number");
            }
            settings.llm.builtin.gpu_layers = v;
        }
        "no_gpu" => {
            let v: bool = value.parse().context("no_gpu must be true or false")?;
            settings.llm.builtin.no_gpu = v;
            if v {
                settings.llm.builtin.gpu_layers = 0;
            }
        }
        "threads" => {
            let v: i32 = value.parse().context("threads must be an integer")?;
            if v < -1 || v > 128 {
                bail!("threads must be between -1 and 128");
            }
            settings.llm.builtin.threads = v;
        }
        "batch_size" => {
            let v: u32 = value.parse().context("batch_size must be a positive integer")?;
            if v < 32 || v > 65536 {
                bail!("batch_size must be between 32 and 65536");
            }
            settings.llm.builtin.batch_size = v;
        }
        "context_size" => {
            let v: u32 = value.parse().context("context_size must be a positive integer")?;
            let max_ctx = registry.get(&settings.default_model)
                .map(|e| e.context_length)
                .unwrap_or(32768);
            if v > max_ctx {
                bail!(
                    "context_size ({}) exceeds model '{}' max context ({}). Set to {} or lower.",
                    v, settings.default_model, max_ctx, max_ctx
                );
            }
            if v < 256 {
                bail!("context_size must be at least 256");
            }
            settings.llm.builtin.context_size = v;
        }
        "inference_timeout_secs" => {
            let v: u64 = value.parse().context("inference_timeout_secs must be a positive integer")?;
            if v < 30 || v > 36000 {
                bail!("inference_timeout_secs must be between 30 and 36000 (10 hours)");
            }
            settings.llm.builtin.inference_timeout_secs = v;
        }
        "temperature" => {
            let v: f32 = value.parse().context("temperature must be a number")?;
            if !(0.0..=2.0).contains(&v) {
                bail!("temperature must be between 0.0 and 2.0");
            }
            settings.llm.builtin.temperature = v;
        }
        "top_p" => {
            let v: f32 = value.parse().context("top_p must be a number")?;
            if !(0.0..=1.0).contains(&v) {
                bail!("top_p must be between 0.0 and 1.0");
            }
            settings.llm.builtin.top_p = v;
        }
        "top_k" => {
            let v: u32 = value.parse().context("top_k must be a non-negative integer")?;
            if v > 200 {
                bail!("top_k must be between 0 and 200");
            }
            settings.llm.builtin.top_k = v;
        }
        "repeat_penalty" => {
            let v: f32 = value.parse().context("repeat_penalty must be a number")?;
            if !(0.5..=5.0).contains(&v) {
                bail!("repeat_penalty must be between 0.5 and 5.0");
            }
            settings.llm.builtin.repeat_penalty = v;
        }
        "max_tokens" => {
            let v: u32 = value.parse().context("max_tokens must be a positive integer")?;
            if v < 64 || v > 65536 {
                bail!("max_tokens must be between 64 and 65536");
            }
            settings.llm.builtin.max_tokens = v;
        }

        // Top-level settings
        "default_model" => {
            let v = value.to_string();
            if registry.get(&v).is_none() {
                bail!("Model '{}' not found in registry. Use `vo1d models list` to see available models.", v);
            }
            settings.default_model = v;
        }
        "default_mode" => {
            let v = value.to_lowercase();
            match v.as_str() {
                "safe" | "interactive" | "power-user" | "poweruser" | "autonomous" | "unrestricted" => {}
                _ => bail!("Invalid mode '{}'. Valid: safe, interactive, power-user, autonomous, unrestricted", value),
            }
            settings.default_mode = v;
        }
        "default_behavior" => {
            match value.to_lowercase().as_str() {
                "normal" | "fix" | "research" | "refactor" | "tdd" => {}
                _ => bail!("Invalid behavior '{}'. Valid: normal, fix, research, refactor, tdd", value),
            }
            settings.default_behavior = value.to_lowercase();
        }
        "command_timeout_secs" => {
            let v: u64 = value.parse().context("command_timeout_secs must be a positive integer")?;
            if v < 5 || v > 3600 {
                bail!("command_timeout_secs must be between 5 and 3600");
            }
            settings.command_timeout_secs = v;
        }
        "max_iterations" => {
            let v: u32 = value.parse().context("max_iterations must be a positive integer")?;
            if v < 1 || v > 9999999 {
                bail!("max_iterations must be between 1 and 9999999");
            }
            settings.max_iterations = v;
        }
        "max_backups" => {
            let v: u32 = value.parse().context("max_backups must be a positive integer")?;
            if v > 1000 {
                bail!("max_backups must be between 0 and 1000");
            }
            settings.max_backups = v;
        }
        _ => bail!(
            "Unknown setting '{}'. Use `vo1d settings list` to see available settings.",
            key
        ),
    }

    save_settings(paths, &settings)?;
    let display_val = if key == "default_model" {
        format!("'{}'", value)
    } else {
        value.to_string()
    };
    println!("✓ set {} = {}", key, display_val);
    Ok(())
}
