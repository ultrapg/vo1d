use serde::{Deserialize, Serialize};
use std::fmt;

/// Behavioral modes that change how the agent approaches tasks.
/// Separate from security modes (which control permissions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorMode {
    /// Default: standard ReAct loop with optional PLAN.md
    Normal,
    /// Fix: read-only phase first, hypothesis before edits, backups, verify
    Fix,
    /// Research: read-only for first 5 iterations, search-before-read
    Research,
    /// Refactor: run tests before and after each change, small steps
    Refactor,
    /// TDD: failing test first, then minimal code, then refactor
    Tdd,
}

impl BehaviorMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            BehaviorMode::Normal => "normal",
            BehaviorMode::Fix => "fix",
            BehaviorMode::Research => "research",
            BehaviorMode::Refactor => "refactor",
            BehaviorMode::Tdd => "tdd",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "normal" => Some(BehaviorMode::Normal),
            "fix" => Some(BehaviorMode::Fix),
            "research" | "researching" => Some(BehaviorMode::Research),
            "refactor" => Some(BehaviorMode::Refactor),
            "tdd" | "test-driven" => Some(BehaviorMode::Tdd),
            _ => None,
        }
    }

    /// Number of initial read-only iterations for this mode.
    pub fn read_only_iters(&self) -> usize {
        match self {
            BehaviorMode::Fix => 3,
            BehaviorMode::Research => 5,
            BehaviorMode::Refactor => 1,
            BehaviorMode::Tdd => 1,
            BehaviorMode::Normal => 0,
        }
    }

    /// Whether this mode requires a plan file before execution.
    pub fn requires_plan(&self) -> bool {
        matches!(self, BehaviorMode::Fix | BehaviorMode::Refactor | BehaviorMode::Tdd)
    }

    /// Extra prompt instructions for the mode.
    pub fn system_prompt_note(&self) -> &'static str {
        match self {
            BehaviorMode::Normal => "",
            BehaviorMode::Fix => r#"
FIX MODE RULES:
- First 3 iterations: READ-ONLY — inspect the code before making any changes.
- Before each edit, state a hypothesis about what's wrong.
- Create FIX.md documenting each change you make.
- After every change, verify the fix works (compile/test).
- If a fix fails, roll back and try a different hypothesis."#,
            BehaviorMode::Research => r#"
RESEARCH MODE RULES:
- First 5 iterations: READ-ONLY — gather information before acting.
- Use search_files before read_file on large codebases.
- Summarize findings in RESEARCH.md when done.
- Only make changes if explicitly asked."#,
            BehaviorMode::Refactor => r#"
REFACTOR MODE RULES:
- Run existing tests BEFORE making any changes.
- Make ONE small mechanical change at a time.
- Run tests AFTER every change.
- If tests fail, revert immediately.
- Do not change behavior — only restructure code."#,
            BehaviorMode::Tdd => r#"
TDD MODE RULES:
- Step 1: Write a failing test first.
- Step 2: Write minimal code to make the test pass.
- Step 3: Refactor the code.
- You CANNOT skip phases or reorder them.
- If no test framework exists, create one first."#,
        }
    }
}

impl Default for BehaviorMode {
    fn default() -> Self {
        BehaviorMode::Normal
    }
}

impl fmt::Display for BehaviorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_behavior_roundtrip() {
        for mode in &[BehaviorMode::Normal, BehaviorMode::Fix, BehaviorMode::Research,
                       BehaviorMode::Refactor, BehaviorMode::Tdd] {
            assert_eq!(BehaviorMode::from_str(mode.as_str()), Some(*mode));
        }
    }

    #[test]
    fn test_read_only_iters() {
        assert_eq!(BehaviorMode::Fix.read_only_iters(), 3);
        assert_eq!(BehaviorMode::Research.read_only_iters(), 5);
        assert_eq!(BehaviorMode::Normal.read_only_iters(), 0);
    }

    #[test]
    fn test_requires_plan() {
        assert!(BehaviorMode::Fix.requires_plan());
        assert!(BehaviorMode::Tdd.requires_plan());
        assert!(!BehaviorMode::Normal.requires_plan());
    }

    #[test]
    fn test_prompt_notes_not_empty_for_modes() {
        assert!(BehaviorMode::Fix.system_prompt_note().contains("FIX MODE"));
        assert!(BehaviorMode::Research.system_prompt_note().contains("RESEARCH MODE"));
        assert!(BehaviorMode::Tdd.system_prompt_note().contains("TDD MODE"));
        assert!(BehaviorMode::Normal.system_prompt_note().is_empty());
    }
}
