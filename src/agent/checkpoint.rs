use crate::agent::session::Session;
use crate::AppContext;
use anyhow::{Context, Result};

/// Save a checkpoint of the current session state.
pub fn save_checkpoint(ctx: &AppContext, session: &Session, step: usize) -> Result<()> {
    let checkpoint_dir = ctx.paths.session_checkpoints_dir(&session.session_id);
    std::fs::create_dir_all(&checkpoint_dir)
        .with_context(|| format!("Failed to create checkpoint directory: {}", checkpoint_dir.display()))?;

    let checkpoint_path = checkpoint_dir.join(format!("checkpoint_{}.json", step));
    let json = serde_json::to_string_pretty(&session)
        .context("Failed to serialize checkpoint")?;

    // Atomic write: write to temp, then rename
    let tmp_path = checkpoint_dir.join(format!("checkpoint_{}.tmp", step));
    std::fs::write(&tmp_path, &json)
        .with_context(|| format!("Failed to write temporary checkpoint: {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &checkpoint_path)
        .with_context(|| format!("Failed to rename checkpoint file: {}", checkpoint_path.display()))?;

    tracing::info!("Checkpoint saved at step {}", step);
    Ok(())
}

/// Load the latest checkpoint for a session.
pub fn load_latest_checkpoint(ctx: &AppContext, session_id: &str) -> Result<Option<Session>> {
    let checkpoint_dir = ctx.paths.session_checkpoints_dir(session_id);
    if !checkpoint_dir.exists() {
        return Ok(None);
    }

    let mut entries: Vec<_> = std::fs::read_dir(&checkpoint_dir)
        .context("Failed to read checkpoint directory")?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
        .collect();

    entries.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

    if let Some(latest) = entries.last() {
        let content = std::fs::read_to_string(latest.path())
            .with_context(|| format!("Failed to read checkpoint: {}", latest.path().display()))?;
        let session: Session = serde_json::from_str(&content)
            .context("Failed to parse checkpoint")?;
        Ok(Some(session))
    } else {
        Ok(None)
    }
}

/// Save resume temp for crash recovery.
pub fn save_resume_temp(ctx: &AppContext, session: &Session) -> Result<()> {
    let resume_path = ctx.paths.resume_temp_path();
    let tmp_path = resume_path.with_extension("tmp");
    let json = serde_json::to_string_pretty(&session)
        .context("Failed to serialize resume state")?;
    std::fs::write(&tmp_path, &json)
        .with_context(|| format!("Failed to write resume temp: {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, &resume_path)
        .with_context(|| format!("Failed to rename resume temp: {}", resume_path.display()))?;
    Ok(())
}

/// Check if resume temp exists.
pub fn has_resume_temp(ctx: &AppContext) -> bool {
    ctx.paths.resume_temp_path().exists()
}

/// Load and delete resume temp.
pub fn load_resume_temp(ctx: &AppContext) -> Result<Option<Session>> {
    let path = ctx.paths.resume_temp_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .context("Failed to read resume temp")?;
    let session: Session = serde_json::from_str(&content)
        .context("Failed to parse resume temp")?;
    std::fs::remove_file(&path)?;
    Ok(Some(session))
}
