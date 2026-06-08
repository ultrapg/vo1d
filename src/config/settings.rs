use serde::{Deserialize, Serialize};

/// Top-level VO1D configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// LLM backend configuration
    pub llm: LlmConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Default security mode on startup
    pub default_mode: String,
    /// Default model ID
    pub default_model: String,
    /// Workspace path override (empty = use portable default)
    pub workspace_path: String,
    /// Network whitelist (empty = no restriction)
    pub network_whitelist: Vec<String>,
    /// Command blacklist patterns
    pub command_blacklist: Vec<String>,
    /// Default behavioral mode (normal, fix, research, refactor, tdd)
    pub default_behavior: String,
    /// Maximum plan iterations before forced halt
    pub max_iterations: u32,
    /// Default command timeout in seconds
    pub command_timeout_secs: u64,
    /// Maximum number of backups per file
    pub max_backups: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LlmConfig {
    /// Backend to use: "builtin", "ollama", "lmstudio", "llamacpp-server", "custom"
    pub backend: String,
    /// Built-in model parameters
    pub builtin: BuiltinConfig,
    /// Custom API endpoint config
    pub custom_api: CustomApiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BuiltinConfig {
    /// Number of CPU threads for inference (-1 = auto)
    pub threads: i32,
    /// GPU layers to offload (0 = CPU only, -1 = all)
    pub gpu_layers: i32,
    /// Batch size for prompt processing
    pub batch_size: u32,
    /// Context size
    pub context_size: u32,
    /// Temperature for generation
    pub temperature: f32,
    /// Top-p sampling
    pub top_p: f32,
    /// Top-k sampling (0 = disabled)
    pub top_k: u32,
    /// Repeat penalty
    pub repeat_penalty: f32,
    /// Max tokens to generate
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomApiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Require approval for workspace writes in Interactive mode
    pub require_workspace_write_approval: bool,
    /// Command blacklist patterns
    pub command_blacklist: Vec<String>,
    /// Network whitelist (empty = no restriction)
    pub network_whitelist: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
            security: SecurityConfig::default(),
            default_mode: "interactive".to_string(),
            default_model: "qwen3_1.7b".to_string(),
            workspace_path: String::new(),
            network_whitelist: vec![],
            command_blacklist: vec![
                "rm -rf /".to_string(),
                "mkfs".to_string(),
                "dd if=".to_string(),
                "format ".to_string(),
                "del /f /s /q".to_string(),
                "rd /s /q".to_string(),
                "reg delete".to_string(),
                "sc delete".to_string(),
                "systemctl disable".to_string(),
                "shutdown".to_string(),
                "reboot".to_string(),
            ],
            default_behavior: "normal".to_string(),
            max_iterations: 999999,
            command_timeout_secs: 60,
            max_backups: 10,
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            backend: "builtin".to_string(),
            builtin: BuiltinConfig::default(),
            custom_api: CustomApiConfig::default(),
        }
    }
}

impl Default for BuiltinConfig {
    fn default() -> Self {
        Self {
            threads: -1,
            gpu_layers: -1,
            batch_size: 4096,
            context_size: 8192,
            temperature: 0.7,
            top_p: 0.9,
            top_k: 40,
            repeat_penalty: 1.1,
            max_tokens: 2048,
        }
    }
}

impl Default for CustomApiConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            api_key: String::new(),
            model_name: String::new(),
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_workspace_write_approval: true,
            command_blacklist: vec![],
            network_whitelist: vec![],
        }
    }
}
