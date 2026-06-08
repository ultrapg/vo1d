use crate::config;
use crate::config::settings::Settings;
use crate::core::hardware::HardwareProfile;
use crate::core::paths::Vo1dPaths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A model entry in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub download_url: String,
    pub filename: String,
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

/// The model registry holds all known models.
#[derive(Clone)]
pub struct ModelRegistry {
    /// Embedded + user-custom models
    entries: Vec<ModelEntry>,
    /// Quick lookup by ID
    by_id: HashMap<String, usize>,
    /// Paths for file resolution
    paths: Vo1dPaths,
    /// Settings reference for defaults
    settings: Settings,
}

impl ModelRegistry {
    pub fn new(paths: &Vo1dPaths, settings: &Settings) -> Result<Self> {
        // Load embedded default models
        let embedded: Vec<ModelEntry> = load_embedded_models();

        // Load user custom models
        let custom_path = paths.config_dir().join("models.toml");
        let custom = config::models::load_custom_models(&custom_path)?;

        // Merge: custom overrides embedded by ID
        let mut entries: Vec<ModelEntry> = embedded;
        let custom_ids: std::collections::HashSet<String> = custom.iter().map(|m| m.id.clone()).collect();

        // Remove embedded entries that are overridden
        entries.retain(|e| !custom_ids.contains(&e.id));

        // Convert CustomModelEntry to ModelEntry and add
        for c in custom {
            entries.push(ModelEntry {
                id: c.id,
                name: c.name,
                provider: c.provider,
                download_url: c.download_url,
                filename: c.filename,
                size_bytes: c.size_bytes,
                min_ram_gb: c.min_ram_gb,
                context_length: c.context_length,
                supports_tools: c.supports_tools,
                native_tools: c.native_tools,
                quantization: c.quantization,
                reasoning: c.reasoning,
                instruct: c.instruct,
            });
        }

        // Build by_id lookup
        let by_id: HashMap<String, usize> = entries.iter().enumerate().map(|(i, e)| (e.id.clone(), i)).collect();

        Ok(Self {
            entries,
            by_id,
            paths: paths.clone(),
            settings: settings.clone(),
        })
    }

    /// List all models in the catalog.
    pub fn list(&self) -> &[ModelEntry] {
        &self.entries
    }

    /// Look up a model by ID.
    pub fn get(&self, id: &str) -> Option<&ModelEntry> {
        self.by_id.get(id).map(|&i| &self.entries[i])
    }

    /// Get the default model from settings or hardware profile.
    pub fn default(&self, hw: &HardwareProfile) -> &ModelEntry {
        let default_id = &self.settings.default_model;
        self.get(default_id).unwrap_or_else(|| {
            // Fall back to hardware-appropriate model
            let size_cat = crate::core::hardware::recommended_model_size_category(hw);
            self.entries.iter().find(|m| m.id.contains(size_cat) && m.min_ram_gb <= hw.total_ram_gb)
                .unwrap_or(&self.entries[0])
        })
    }

    /// Check if a model is installed (file exists).
    pub fn is_installed(&self, entry: &ModelEntry) -> bool {
        let model_path = self.paths.models_backend_dir("llamacpp").join(&entry.filename);
        model_path.exists()
    }

    /// Get the file path for an installed model.
    pub fn model_path(&self, entry: &ModelEntry) -> std::path::PathBuf {
        self.paths.models_backend_dir("llamacpp").join(&entry.filename)
    }

    /// Filter models by minimum RAM requirement.
    pub fn compatible_with_hardware(&self, hw: &HardwareProfile) -> Vec<&ModelEntry> {
        self.entries.iter().filter(|m| m.min_ram_gb <= hw.total_ram_gb).collect()
    }
}

/// Remove a model (delete file).
pub async fn remove_model(ctx: &crate::AppContext, id: &str) -> Result<()> {
    let entry = ctx.model_registry.get(id)
        .ok_or_else(|| anyhow::anyhow!("Model '{}' not found in registry", id))?;

    let model_path = ctx.paths.models_backend_dir("llamacpp").join(&entry.filename);
    if model_path.exists() {
        std::fs::remove_file(&model_path)
            .with_context(|| format!("Failed to remove model file: {}", model_path.display()))?;
        println!("Removed model: {} ({})", entry.name, entry.id);
    } else {
        println!("Model '{}' is not installed.", id);
    }

    ctx.audit.log_model("remove_model", &id, ctx.security.current_mode)?;
    Ok(())
}

/// Load the 20+ embedded models compiled into the binary.
fn load_embedded_models() -> Vec<ModelEntry> {
    let toml_str = include_str!("../../config/default_models.toml");
    toml::from_str::<ModelCatalog>(toml_str)
        .map(|c| c.model)
        .unwrap_or_else(|e| {
            tracing::warn!("Failed to parse embedded model catalog: {}", e);
            vec![]
        })
}

#[derive(Debug, Deserialize)]
struct ModelCatalog {
    model: Vec<ModelEntry>,
}
