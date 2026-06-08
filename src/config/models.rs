use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// User-defined model registration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomModelEntry {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub download_url: String,
    pub filename: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub min_ram_gb: f64,
    pub context_length: u32,
    pub supports_tools: bool,
    #[serde(default)]
    pub native_tools: bool,
    pub quantization: String,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub instruct: bool,
}

/// Container for custom model registrations from models.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsConfig {
    pub model: Vec<CustomModelEntry>,
}

impl Default for ModelsConfig {
    fn default() -> Self {
        Self { model: vec![] }
    }
}

/// Load custom model registrations from config/models.toml.
pub fn load_custom_models(path: &Path) -> Result<Vec<CustomModelEntry>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read models config: {}", path.display()))?;
    let config: ModelsConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse models config: {}", path.display()))?;
    Ok(config.model)
}

/// Save custom model registration.
pub fn save_custom_models(path: &Path, models: &[CustomModelEntry]) -> Result<()> {
    let config = ModelsConfig {
        model: models.to_vec(),
    };
    let toml_str = toml::to_string_pretty(&config)
        .context("Failed to serialize models config")?;
    std::fs::write(path, &toml_str)
        .with_context(|| format!("Failed to write models config: {}", path.display()))?;
    Ok(())
}
