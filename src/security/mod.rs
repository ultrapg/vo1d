pub mod modes;
pub mod policy;
pub mod sandbox;
pub mod approval;
pub mod privilege;
pub mod audit;

use crate::config::settings::Settings;
use anyhow::{Context, Result};
pub use audit::AuditLogger;
pub use modes::SecurityMode;
use tracing::info;

/// Security manager: holds current mode, policy engine, and audit logger.
#[derive(Clone)]
pub struct SecurityManager {
    pub current_mode: SecurityMode,
    pub policy: policy::PolicyEngine,
    pub sandbox: sandbox::Sandbox,
    pub audit: AuditLogger,
    pub approval: approval::ApprovalSystem,
}

impl SecurityManager {
    pub fn new(config: &Settings, audit: &AuditLogger) -> Result<Self> {
        let mode = SecurityMode::from_str(&config.default_mode)
            .unwrap_or(SecurityMode::Interactive);
        let policy = policy::PolicyEngine::new(config);
        let sandbox = sandbox::Sandbox::new(config);
        let approval = approval::ApprovalSystem::new(mode);

        info!("Security manager initialized with mode: {:?}", mode);
        Ok(Self {
            current_mode: mode,
            policy,
            sandbox,
            audit: audit.clone(),
            approval,
        })
    }

    /// Set the security mode at runtime.
    pub fn set_mode(&mut self, mode: SecurityMode) {
        self.current_mode = mode;
        self.approval = approval::ApprovalSystem::new(mode);
        info!("Security mode changed to: {:?}", mode);
    }
}

/// Perform the YOLO mode handshake.
pub fn yolo_handshake() -> Result<()> {
    println!("{}", "=".repeat(70));
    println!("{}", "WARNING: VO1D IS INITIALIZING IN \"YOLO\" MODE.");
    println!("{}", "IN THIS MODE:");
    println!("{}", "- ALL EXECUTIONS (READS, WRITES, SYSTEM CALLS) RUN WITH ABSOLUTE AUTONOMY.");
    println!("{}", "- ACTIONS CANNOT BE ROLLED BACK OR CANCELLED BY PROMPTS.");
    println!("{}", "- RUNNING ELEVATED (ADMIN) WILL ACCORD THE LLM UNLIMITED SYSTEM ACCESS.");
    println!("{}", "=".repeat(70));

    let mut input = String::new();
    print!("Please type the phrase \"I UNDERSTAND THE RISKS\" to confirm: ");
    std::io::Write::flush(&mut std::io::stdout()).context("Failed to flush stdout")?;
    std::io::stdin()
        .read_line(&mut input)
        .context("Failed to read confirmation")?;

    let trimmed = input.trim();
    if trimmed != "I UNDERSTAND THE RISKS" {
        anyhow::bail!("YOLO mode confirmation failed. Exact phrase required. Exiting.");
    }

    println!("[YOLO MODE] Confirmed. All actions will be executed with absolute autonomy.");
    Ok(())
}
