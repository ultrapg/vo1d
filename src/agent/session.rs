use crate::core::paths::Vo1dPaths;
use crate::AppContext;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Completed,
    Failed,
    Cancelled,
}

/// Current session format version. Increment on breaking changes.
const SESSION_VERSION: u32 = 1;

/// A session stores all state for a task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub started_at: String,
    pub base_task: String,
    pub current_model: String,
    pub security_mode: String,
    pub execution_step_index: u32,
    pub variables: HashMap<String, String>,
    pub status: SessionStatus,
    pub final_output: Option<String>,
    #[serde(skip)]
    pub tui_mode: bool,
    #[serde(default)]
    pub version: u32,
}

impl Session {
    pub fn new(task: &str, ctx: &AppContext) -> Result<Self> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

        let session = Self {
            session_id,
            started_at: now,
            base_task: task.to_string(),
            current_model: ctx.config.default_model.clone(),
            security_mode: ctx.security.current_mode.as_str().to_string(),
            execution_step_index: 0,
            variables: HashMap::new(),
            status: SessionStatus::Active,
            final_output: None,
            tui_mode: false,
            version: SESSION_VERSION,
        };

        // Create session directory
        let session_dir = ctx.paths.session_dir(&session.session_id);
        std::fs::create_dir_all(&session_dir)
            .with_context(|| format!("Failed to create session directory: {}", session_dir.display()))?;

        info!("Created session: {} for task: {}", session.session_id, task);
        Ok(session)
    }
}

/// Save session metadata to disk.
pub async fn save_session_metadata(ctx: &AppContext, session: &Session) -> Result<()> {
    let metadata_path = ctx.paths.session_dir(&session.session_id).join("metadata.toml");
    let toml_str = toml::to_string_pretty(&session)
        .context("Failed to serialize session metadata")?;
    tokio::fs::write(&metadata_path, &toml_str).await
        .with_context(|| format!("Failed to write session metadata: {}", metadata_path.display()))?;
    Ok(())
}

/// Load session metadata from disk, with version migration.
pub async fn load_session_metadata(ctx: &AppContext, session_id: &str) -> Result<Session> {
    let metadata_path = ctx.paths.session_dir(session_id).join("metadata.toml");
    let content = tokio::fs::read_to_string(&metadata_path).await
        .with_context(|| format!("Failed to read session metadata: {}", metadata_path.display()))?;
    let mut session: Session = toml::from_str(&content)
        .with_context(|| format!("Failed to parse session metadata: {}", metadata_path.display()))?;

    // Version migration
    match session.version {
        0 => {
            // v0 -> v1: no structural changes yet, just bump version
            session.version = 1;
            // Re-save with updated version
            save_session_metadata(ctx, &session).await?;
        }
        SESSION_VERSION => {} // current, no migration needed
        v if v > SESSION_VERSION => {
            tracing::warn!(
                "Session '{}' has version {} which exceeds current version {}. Some data may be incompatible.",
                session_id, v, SESSION_VERSION
            );
        }
        _ => unreachable!(),
    }

    Ok(session)
}

/// Resume a session from saved state.
pub async fn resume_session(ctx: AppContext, session_id: &str) -> Result<()> {
    let session = load_session_metadata(&ctx, session_id).await?;
    info!("Resuming session: {} (status: {:?})", session_id, session.status);

    if session.status == SessionStatus::Completed {
        println!("Session '{}' is already completed.", session_id);
        return Ok(());
    }

    // Re-run the agent loop with restored session
    crate::agent::run(ctx, session).await?;
    Ok(())
}

/// List all saved sessions.
pub async fn list_sessions() -> Result<()> {
    let paths = Vo1dPaths::new()?;
    let sessions_dir = paths.sessions_dir();

    if !sessions_dir.exists() {
        println!("No sessions found.");
        return Ok(());
    }

    let mut entries: Vec<_> = std::fs::read_dir(&sessions_dir)
        .context("Failed to read sessions directory")?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();

    entries.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

    if entries.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    println!("{:<20} {:<30} {:<20} {:?}", "Session ID", "Task", "Status", "Date");
    println!("{}", "-".repeat(80));

    for entry in entries {
        let meta_path = entry.path().join("metadata.toml");
        if let Ok(content) = std::fs::read_to_string(&meta_path) {
            if let Ok(session) = toml::from_str::<Session>(&content) {
                println!("{:<20} {:<30} {:<20} {}",
                    session.session_id.chars().take(12).collect::<String>(),
                    session.base_task.chars().take(30).collect::<String>(),
                    format!("{:?}", session.status),
                    session.started_at,
                );
            }
        }
    }

    Ok(())
}
