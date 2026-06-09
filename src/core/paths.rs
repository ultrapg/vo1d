use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::info;

/// Resolves all VO1D paths relative to the executable location for full portability.
#[derive(Clone)]
pub struct Vo1dPaths {
    /// Root directory containing the binary
    root: PathBuf,
    /// Optional override for workspace_dir (used during training)
    workspace_override: Option<PathBuf>,
}

impl Vo1dPaths {
    /// Create a new path resolver from the current executable location.
    pub fn new() -> Result<Self> {
        let exe = std::env::current_exe()
            .context("Failed to get current executable path")?;
        let root = exe
            .parent()
            .context("Failed to get executable parent directory")?
            .to_path_buf();
        info!("VO1D root directory: {}", root.display());
        Ok(Self { root, workspace_override: None })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root
    }

    pub fn config_dir(&self) -> PathBuf {
        self.root.join("config")
    }

    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    pub fn models_backend_dir(&self, backend: &str) -> PathBuf {
        self.root.join("models").join(backend)
    }

    pub fn workspace_dir(&self) -> PathBuf {
        self.workspace_override
            .clone()
            .unwrap_or_else(|| self.root.join("workspace"))
    }

    /// Create a copy with an overridden workspace directory (used for training).
    pub fn with_workspace_override(&self, path: PathBuf) -> Self {
        Self {
            root: self.root.clone(),
            workspace_override: Some(path),
        }
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn http_cache_dir(&self) -> PathBuf {
        self.root.join("cache").join("http_cache")
    }

    pub fn partial_download_dir(&self) -> PathBuf {
        self.root.join("cache").join("partial")
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    pub fn session_dir(&self, session_id: &str) -> PathBuf {
        self.root.join("sessions").join(session_id)
    }

    pub fn session_checkpoints_dir(&self, session_id: &str) -> PathBuf {
        self.root.join("sessions").join(session_id).join("checkpoints")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn audit_log_path(&self) -> PathBuf {
        let date = chrono::Utc::now().format("%Y-%m-%d");
        self.root.join("logs").join(format!("audit_{}.jsonl", date))
    }

    pub fn unrestricted_audit_log_path(&self) -> PathBuf {
        let date = chrono::Utc::now().format("%Y-%m-%d");
        self.root.join("logs").join(format!("unrestricted_audit_{}.jsonl", date))
    }

    pub fn error_log_path(&self) -> PathBuf {
        self.root.join("logs").join("errors.jsonl")
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.root.join("downloads")
    }

    pub fn tools_dir(&self) -> PathBuf {
        self.root.join("downloads").join("tools")
    }

    pub fn plugins_dir(&self) -> PathBuf {
        self.root.join("plugins")
    }

    pub fn memory_dir(&self) -> PathBuf {
        self.root.join("memory")
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    pub fn curriculum_dir(&self) -> PathBuf {
        self.root.join("curriculum")
    }

    pub fn train_sandbox_dir(&self) -> PathBuf {
        self.workspace_dir().join("train_sandbox")
    }

    pub fn resume_temp_path(&self) -> PathBuf {
        self.root.join("sessions").join("resume_temp.json")
    }

    pub fn backup_dir(&self) -> PathBuf {
        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        self.root.join("workspace").join(format!(".vo1d_backup.{}", ts))
    }

    pub fn model_index_path(&self) -> PathBuf {
        self.root.join("cache").join("model_index.json")
    }

    pub fn default_models_config(&self) -> PathBuf {
        self.root.join("config").join("default_models.toml")
    }

    /// Create all required directories if they don't exist.
    pub fn ensure_dirs(&self) -> Result<()> {
        let dirs = [
            self.config_dir(),
            self.models_dir(),
            self.models_backend_dir("llamacpp"),
            self.workspace_dir(),
            self.cache_dir(),
            self.http_cache_dir(),
            self.partial_download_dir(),
            self.sessions_dir(),
            self.logs_dir(),
            self.downloads_dir(),
            self.tools_dir(),
            self.plugins_dir(),
            self.memory_dir(),
            self.curriculum_dir(),
            self.skills_dir(),
        ];

        for dir in &dirs {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("Failed to create directory: {}", dir.display()))?;
        }

        info!("VO1D directory structure initialized at {}", self.root.display());
        Ok(())
    }

    /// Resolve a path relative to the workspace, or absolute if already absolute.
    /// Validates the resolved path stays within the workspace boundary.
    /// Returns the canonicalized path if valid, or an error if it escapes.
    pub fn resolve_workspace_path(&self, user_path: &str) -> PathBuf {
        let path = PathBuf::from(user_path);
        if path.is_absolute() {
            // Check absolute paths don't use traversal to escape
            self.sanitize_path(&path)
        } else {
            let resolved = self.workspace_dir().join(path);
            self.sanitize_path(&resolved)
        }
    }

    /// Sanitize a path by canonicalizing and checking it stays within workspace.
    fn sanitize_path(&self, path: &Path) -> PathBuf {
        // Try to canonicalize; if it fails (e.g. path doesn't exist yet),
        // do a manual traversal check
        match path.canonicalize() {
            Ok(canonical) => {
                let ws = self.workspace_dir().canonicalize().unwrap_or_else(|_| self.workspace_dir());
                if canonical.starts_with(&ws) {
                    canonical
                } else {
                    tracing::warn!("Path escapes workspace: {} -> {}", path.display(), canonical.display());
                    ws.join(path.file_name().unwrap_or_default())
                }
            }
            Err(_) => {
                // Path doesn't exist yet - check for traversal components
                let ws = self.workspace_dir();
                let components: Vec<_> = path.components().collect();
                let mut depth: i32 = 0;
                for comp in &components {
                    if comp.as_os_str() == ".." {
                        depth -= 1;
                    } else if comp.as_os_str() != "." && comp.as_os_str() != "" {
                        depth += 1;
                    }
                    if depth < 0 {
                        tracing::warn!("Path traversal detected: {}", path.display());
                        return ws.join(path.file_name().unwrap_or_default());
                    }
                }
                path.to_path_buf()
            }
        }
    }

    /// Check if a given path is within the workspace boundary.
    /// Walks up the directory tree for non-existent paths (e.g. about-to-be-deleted files).
    pub fn is_within_workspace(&self, path: &Path) -> bool {
        let ws = match self.workspace_dir().canonicalize() {
            Ok(w) => w,
            Err(_) => return false,
        };
        let mut p = Some(path);
        while let Some(current) = p {
            if let Ok(canonical) = current.canonicalize() {
                return canonical.starts_with(&ws);
            }
            p = current.parent();
        }
        false
    }

    /// Check if a given path is within the VO1D root (safe for internal ops).
    pub fn is_within_root(&self, path: &Path) -> bool {
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };
        let root = match self.root.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };
        canonical.starts_with(&root)
    }
}
