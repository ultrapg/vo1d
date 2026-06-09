pub mod settings;
pub mod models;
pub mod settings_cmd;

use anyhow::{Context, Result};
use crate::core::paths::Vo1dPaths;

/// Load all configuration: settings + merged model registry.
pub fn load_config(paths: &Vo1dPaths) -> Result<settings::Settings> {
    let settings_path = paths.config_dir().join("settings.toml");
    let settings = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)
            .with_context(|| format!("Failed to read settings: {}", settings_path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse settings: {}", settings_path.display()))?
    } else {
        // Write defaults
        let defaults = settings::Settings::default();
        let toml_str = toml::to_string_pretty(&defaults)
            .context("Failed to serialize default settings")?;
        std::fs::write(&settings_path, &toml_str)
            .with_context(|| format!("Failed to write default settings: {}", settings_path.display()))?;
        tracing::info!("Created default settings at {}", settings_path.display());
        defaults
    };

    Ok(settings)
}

/// Save settings to disk atomically.
pub fn save_settings(paths: &Vo1dPaths, settings: &settings::Settings) -> Result<()> {
    let settings_path = paths.config_dir().join("settings.toml");
    let tmp_path = paths.config_dir().join("settings.toml.tmp");
    let toml_str = toml::to_string_pretty(settings)
        .context("Failed to serialize settings")?;
    std::fs::write(&tmp_path, &toml_str)
        .with_context(|| format!("Failed to write temporary settings: {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &settings_path)
        .with_context(|| format!("Failed to rename settings file: {}", settings_path.display()))?;
    Ok(())
}
