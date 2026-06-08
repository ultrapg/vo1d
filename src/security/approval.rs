use crate::models::action::Action;
use crate::security::modes::SecurityMode;
use anyhow::Result;

/// Approval system for user-facing prompts.
#[derive(Clone)]
pub struct ApprovalSystem {
    mode: SecurityMode,
}

impl ApprovalSystem {
    pub fn new(mode: SecurityMode) -> Self {
        Self { mode }
    }

    /// Ask the user for approval of an action. Returns true if approved.
    pub fn ask(&self, action: &Action, message: &str) -> Result<bool> {
        if self.mode.auto_approves() {
            return Ok(true);
        }

        println!("\n[{}] {}", self.mode, message);
        println!("Action: {}", action.description());

        print!("Approve? (Y/n/custom instruction): ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_lowercase();

        match trimmed.as_str() {
            "y" | "yes" | "" => Ok(true),
            "n" | "no" => Ok(false),
            _ => {
                println!("Custom instruction: {}", trimmed);
                Ok(true)
            }
        }
    }
}

/// Prompt user for approval. Used by the CLI and TUI.
pub fn prompt_approval(action: &Action, mode: SecurityMode) -> bool {
    if mode.auto_approves() {
        return true;
    }

    println!("[{}] Approve action: {}", mode, action.description());
    print!("Proceed? (Y/n): ");
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(_) => {
            let trimmed = input.trim().to_lowercase();
            trimmed.is_empty() || trimmed == "y" || trimmed == "yes"
        }
        Err(_) => false,
    }
}
