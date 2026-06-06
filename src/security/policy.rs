use crate::config::settings::Settings;
use crate::models::action::Action;
use crate::core::paths::Vo1dPaths;
use super::modes::SecurityMode;

/// Result of policy evaluation for an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyResult {
    Allow,
    Ask,
    Block,
}

/// Risk level of an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskLevel {
    Safe,
    Risky,
    Destructive,
    Privileged,
}

/// Policy engine evaluates actions against security rules.
#[derive(Clone)]
pub struct PolicyEngine {
    command_blacklist: Vec<String>,
}

impl PolicyEngine {
    pub fn new(config: &Settings) -> Self {
        let mut blacklist = config.command_blacklist.clone();
        blacklist.extend(config.security.command_blacklist.clone());
        Self {
            command_blacklist: blacklist,
        }
    }

    /// Evaluate an action and return the policy decision.
    pub fn evaluate(&self, action: &Action, mode: SecurityMode, paths: &Vo1dPaths) -> PolicyResult {
        match mode {
            SecurityMode::Safe => self.evaluate_safe(action, paths),
            SecurityMode::Interactive => self.evaluate_interactive(action, paths),
            SecurityMode::PowerUser => self.evaluate_power_user(action),
            SecurityMode::Autonomous => self.evaluate_autonomous(action),
            SecurityMode::Yolo => PolicyResult::Allow,
        }
    }

    fn evaluate_safe(&self, action: &Action, paths: &Vo1dPaths) -> PolicyResult {
        match action {
            Action::ReadFile { path, .. } => {
                let p = paths.resolve_workspace_path(path);
                if paths.is_within_workspace(&p) {
                    PolicyResult::Allow
                } else {
                    PolicyResult::Block
                }
            }
            Action::ListDirectory { .. } => PolicyResult::Allow,
            Action::FileMetadata { .. } => PolicyResult::Allow,
            Action::SearchFiles { .. } => PolicyResult::Allow,
            Action::Finish { .. } => PolicyResult::Allow,
            _ => PolicyResult::Block,
        }
    }

    fn evaluate_interactive(&self, action: &Action, paths: &Vo1dPaths) -> PolicyResult {
        match action {
            Action::ReadFile { path, .. } => {
                let p = paths.resolve_workspace_path(path);
                if paths.is_within_workspace(&p) {
                    PolicyResult::Allow
                } else {
                    PolicyResult::Ask
                }
            }
            Action::WriteFile { path, .. } => {
                let p = paths.resolve_workspace_path(path);
                if paths.is_within_workspace(&p) {
                    PolicyResult::Ask
                } else {
                    PolicyResult::Ask
                }
            }
            Action::ExecuteCommand { command, .. } => {
                if self.is_blacklisted(command) {
                    return PolicyResult::Block;
                }
                PolicyResult::Ask
            }
            Action::DeleteFile { path, .. } => {
                let p = paths.resolve_workspace_path(path);
                if paths.is_within_workspace(&p) {
                    PolicyResult::Ask
                } else {
                    PolicyResult::Block
                }
            }
            Action::HttpRequest { .. } => PolicyResult::Ask,
            Action::CopyFile { source, .. } => {
                let p = paths.resolve_workspace_path(source);
                if paths.is_within_workspace(&p) {
                    PolicyResult::Ask
                } else {
                    PolicyResult::Block
                }
            }
            Action::Finish { .. } => PolicyResult::Allow,
            Action::AskUser { .. } => PolicyResult::Allow,
            _ => PolicyResult::Ask,
        }
    }

    fn evaluate_power_user(&self, action: &Action) -> PolicyResult {
        match action {
            Action::ExecuteCommand { command, .. } => {
                if self.is_blacklisted(command) {
                    return PolicyResult::Block;
                }
                PolicyResult::Ask
            }
            Action::WriteFile { .. } | Action::DeleteFile { .. } => PolicyResult::Ask,
            Action::HttpRequest { .. } => PolicyResult::Allow,
            _ => PolicyResult::Allow,
        }
    }

    fn evaluate_autonomous(&self, action: &Action) -> PolicyResult {
        match action {
            Action::ExecuteCommand { command, .. } => {
                if self.is_blacklisted(command) {
                    return PolicyResult::Block;
                }
                PolicyResult::Allow
            }
            _ => PolicyResult::Allow,
        }
    }

    /// Classify the risk level of a command string.
    pub fn classify_risk(&self, command: &str) -> RiskLevel {
        let lower = command.to_lowercase();

        // Privileged operations
        if lower.starts_with("sudo") || lower.starts_with("runas") {
            return RiskLevel::Privileged;
        }

        // Destructive operations
        let destructive_patterns = [
            "rm -rf", "rmdir /s", "del /f", "format ", "mkfs.", "dd if=",
            "rd /s", "reg delete", "sc delete",
        ];
        for pat in &destructive_patterns {
            if lower.contains(pat) {
                return RiskLevel::Destructive;
            }
        }

        // Risky operations
        let risky_patterns = [
            "chmod", "chown", "iptables", "netsh", "reg add",
            "shutdown", "reboot", "taskkill", "wmic",
            "invoke-expression", "iex(", "iex ",
        ];
        for pat in &risky_patterns {
            if lower.contains(pat) {
                return RiskLevel::Risky;
            }
        }

        RiskLevel::Safe
    }

    /// Check if a command is on the blacklist.
    pub fn is_blacklisted(&self, command: &str) -> bool {
        let lower = command.to_lowercase();
        for pattern in &self.command_blacklist {
            if lower.contains(&pattern.to_lowercase()) {
                return true;
            }
        }
        false
    }
}
