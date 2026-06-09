use crate::core::paths::Vo1dPaths;
use crate::security::modes::SecurityMode;
use crate::utils::time;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::sync::Mutex;
use tracing::info;

/// A single audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub session_id: String,
    pub actor: String,
    pub action: String,
    pub payload: String,
    pub security_mode: String,
    pub authorized: bool,
    pub exit_code: Option<i32>,
    pub elevated: bool,
}

/// Non-blocking JSONL audit logger.
#[derive(Clone)]
pub struct AuditLogger {
    audit_writer: std::sync::Arc<Mutex<Option<std::fs::File>>>,
    unrestricted_writer: std::sync::Arc<Mutex<Option<std::fs::File>>>,
    session_id: String,
    elevated: bool,
}

impl AuditLogger {
    pub fn new(paths: &Vo1dPaths) -> Result<Self> {
        let audit_path = paths.audit_log_path();
        let unrestricted_path = paths.unrestricted_audit_log_path();

        let audit_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&audit_path)
            .with_context(|| format!("Failed to open audit log: {}", audit_path.display()))?;

        let unrestricted_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&unrestricted_path)
            .with_context(|| format!("Failed to open unrestricted audit log: {}", unrestricted_path.display()))?;

        let session_id = uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown").to_string();
        let elevated = crate::security::privilege::is_elevated();

        info!("Audit logger initialized: session={}, elevated={}", session_id, elevated);

        Ok(Self {
            audit_writer: std::sync::Arc::new(Mutex::new(Some(audit_file))),
            unrestricted_writer: std::sync::Arc::new(Mutex::new(Some(unrestricted_file))),
            session_id,
            elevated,
        })
    }

    /// Get current session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Log an event to the audit trail.
    pub fn log(&self, actor: &str, action: &str, payload: &str, mode: SecurityMode, authorized: bool, exit_code: Option<i32>) -> Result<()> {
        let entry = AuditEntry {
            timestamp: time::iso_timestamp(),
            session_id: self.session_id.clone(),
            actor: actor.to_string(),
            action: action.to_string(),
            payload: payload.to_string(),
            security_mode: mode.as_str().to_string(),
            authorized,
            exit_code,
            elevated: self.elevated,
        };

        let json = serde_json::to_string(&entry)
            .context("Failed to serialize audit entry")?;

        // Always write to main audit log
        if let Ok(mut guard) = self.audit_writer.lock() {
            if let Some(file) = guard.as_mut() {
                let _ = writeln!(file, "{}", json);
                let _ = file.flush();
            }
        }

        // Also write to unrestricted audit log if in Unrestricted mode
        if mode == SecurityMode::Unrestricted {
            if let Ok(mut guard) = self.unrestricted_writer.lock() {
                if let Some(file) = guard.as_mut() {
                    let _ = writeln!(file, "{}", json);
                    let _ = file.flush();
                }
            }
        }

        Ok(())
    }

    /// Convenience: log model action
    pub fn log_model(&self, action: &str, payload: &str, mode: SecurityMode) -> Result<()> {
        self.log("model", action, payload, mode, true, None)
    }

    /// Convenience: log executor action
    pub fn log_executor(&self, action: &str, payload: &str, mode: SecurityMode, exit_code: i32) -> Result<()> {
        self.log("executor", action, payload, mode, true, Some(exit_code))
    }

    /// Convenience: log user action
    pub fn log_user(&self, action: &str, payload: &str, mode: SecurityMode) -> Result<()> {
        self.log("user", action, payload, mode, true, None)
    }

    /// Convenience: log security event
    pub fn log_security(&self, action: &str, payload: &str, mode: SecurityMode, authorized: bool) -> Result<()> {
        self.log("system", action, payload, mode, authorized, None)
    }
}

/// Tail recent audit log entries.
pub async fn tail_logs(n: usize) -> Result<()> {
    let paths = crate::core::paths::Vo1dPaths::new()?;
    let log_path = paths.audit_log_path();

    if !log_path.exists() {
        println!("No audit log found at {}", log_path.display());
        return Ok(());
    }

    let content = std::fs::read_to_string(&log_path)
        .with_context(|| format!("Failed to read audit log: {}", log_path.display()))?;

    let lines: Vec<&str> = content.lines().collect();
    let start = if lines.len() > n { lines.len() - n } else { 0 };

    for line in &lines[start..] {
        println!("{}", line);
    }

    Ok(())
}
