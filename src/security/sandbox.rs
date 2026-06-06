use crate::config::settings::Settings;
use crate::models::action::Action;
use crate::core::paths::Vo1dPaths;
use std::path::Path;

/// Sandbox restricts file system access and network operations.
#[derive(Clone)]
pub struct Sandbox {
    pub restrict_network: bool,
    pub network_whitelist: Vec<String>,
}

impl Sandbox {
    pub fn new(config: &Settings) -> Self {
        Self {
            restrict_network: !config.network_whitelist.is_empty(),
            network_whitelist: config.network_whitelist.clone(),
        }
    }

    /// Validate that a path is within allowed boundaries.
    pub fn validate_path(&self, path: &Path, paths: &Vo1dPaths, action: &Action) -> Result<(), String> {
        // Allow paths within VO1D root for internal operations
        if paths.is_within_root(path) {
            return Ok(());
        }

        // Allow workspace paths
        if paths.is_within_workspace(path) {
            return Ok(());
        }

        // For non-destructive reads, we can be more lenient
        match action {
            Action::ReadFile { .. } | Action::ListDirectory { .. } | Action::FileMetadata { .. } => {
                if path.is_absolute() {
                    return Ok(()); // Allow absolute reads
                }
            }
            _ => {}
        }

        Err(format!(
            "Path '{}' is outside allowed sandbox. Use absolute path or workspace path.",
            path.display()
        ))
    }

    /// Validate a URL against network whitelist.
    pub fn validate_url(&self, url: &str) -> Result<(), String> {
        if !self.restrict_network {
            return Ok(());
        }
        for allowed in &self.network_whitelist {
            if url.contains(allowed) {
                return Ok(());
            }
        }
        Err(format!(
            "URL '{}' is not in the network whitelist",
            url
        ))
    }
}
