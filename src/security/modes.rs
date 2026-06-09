use serde::{Deserialize, Serialize};

/// Security mode controlling what actions require approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityMode {
    /// No auto-approve, read-only workspace, all commands blocked
    Safe,
    /// Ask for approval on writes, commands, and outside-workspace access
    Interactive,
    /// Full system access with approval prompts
    PowerUser,
    /// Auto-approve except privilege escalation and system mods
    Autonomous,
    /// Auto-approve everything including privilege escalation
    Unrestricted,
}

impl SecurityMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "safe" => Some(Self::Safe),
            "interactive" => Some(Self::Interactive),
            "power-user" | "poweruser" => Some(Self::PowerUser),
            "autonomous" => Some(Self::Autonomous),
            "unrestricted" => Some(Self::Unrestricted),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Safe => "Safe",
            Self::Interactive => "Interactive",
            Self::PowerUser => "PowerUser",
            Self::Autonomous => "Autonomous",
            Self::Unrestricted => "Unrestricted",
        }
    }

    /// Returns true if this mode can auto-approve actions.
    pub fn auto_approves(&self) -> bool {
        matches!(self, Self::Autonomous | Self::Unrestricted)
    }

    /// Returns true if this mode allows command execution.
    pub fn allows_commands(&self) -> bool {
        !matches!(self, Self::Safe)
    }

    /// Returns true if privilege escalation is auto-approved.
    pub fn auto_elevates(&self) -> bool {
        matches!(self, Self::Unrestricted)
    }

    /// Returns true if system modifications are allowed.
    pub fn allows_system_mods(&self) -> bool {
        matches!(self, Self::Unrestricted) || matches!(self, Self::PowerUser)
    }
}

impl std::fmt::Display for SecurityMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
