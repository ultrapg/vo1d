pub mod config;
pub mod core;
pub mod llm;
pub mod agent;
pub mod tools;
pub mod security;
pub mod ui;
pub mod models;
pub mod utils;

use anyhow::Result;

/// VO1D version constant
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Initialize all VO1D subsystems
pub async fn initialize() -> Result<AppContext> {
    let ctx = AppContext::new().await?;
    Ok(ctx)
}

/// Top-level application context holding all runtime state
#[derive(Clone)]
pub struct AppContext {
    pub config: config::settings::Settings,
    pub paths: core::paths::Vo1dPaths,
    pub hardware: core::hardware::HardwareProfile,
    pub security: security::SecurityManager,
    pub audit: security::audit::AuditLogger,
    pub model_registry: llm::registry::ModelRegistry,
    pub memory: std::sync::Arc<std::sync::Mutex<core::memory::MemoryStore>>,
    /// Auto-approve all actions (--yes flag)
    pub auto_approve: bool,
}

impl AppContext {
    pub async fn new() -> Result<Self> {
        let paths = core::paths::Vo1dPaths::new()?;
        paths.ensure_dirs()?;

        let config = config::load_config(&paths)?;
        let hardware = core::hardware::profile()?;
        let audit = security::audit::AuditLogger::new(&paths)?;
        let security = security::SecurityManager::new(&config, &audit)?;
        let model_registry = llm::registry::ModelRegistry::new(&paths, &config)?;
        let memory = std::sync::Arc::new(std::sync::Mutex::new(
            core::memory::MemoryStore::new(&paths.memory_dir())?
        ));

        Ok(Self {
            config,
            paths,
            hardware,
            security,
            audit,
            model_registry,
            memory,
            auto_approve: false,
        })
    }
}
