use anyhow::Error;
use crate::models::action::Action;

/// Error classifier that provides targeted suggestions.
pub struct ErrorClassifier;

impl ErrorClassifier {
    /// Analyze an error and generate a helpful suggestion.
    pub fn suggest(error: &Error) -> Option<String> {
        let msg = error.to_string().to_lowercase();

        if msg.contains("file not found") || msg.contains("no such file") {
            Some("Try listing the directory first to find the correct path. \
                  Check for typos. If the file was just created, verify the path."
                .to_string())
        } else if msg.contains("is a directory") || msg.contains("not a file") {
            Some("If you meant to list a directory, use list_directory. \
                  If you're trying to read a file, check that the path points to a file, not a directory."
                .to_string())
        } else if msg.contains("permission denied") || msg.contains("access denied") {
            Some("Check that the file is inside the workspace and not read-only. \
                  Consider using a different path or mode."
                .to_string())
        } else if msg.contains("invalid json") || msg.contains("parse error") {
            Some("Your JSON action is malformed. Check: \
                  - All strings must use double quotes. \
                  - Commas between fields only. \
                  - No trailing comma after the last field. \
                  - Match all opening and closing braces."
                .to_string())
        } else if msg.contains("pattern") && (msg.contains("no match") || msg.contains("not found")) {
            Some("Pattern might be wrong. Use '*' (not '*.*') to match all files. \
                  Try listing the directory first to see actual filenames."
                .to_string())
        } else if msg.contains("timeout") || msg.contains("timed out") {
            Some("Command timed out. Try: a shorter command, checking syntax, \
                  or increasing the timeout value."
                .to_string())
        } else if msg.contains("blocked") || msg.contains("denied") {
            Some("Action blocked by security policy. You may need to: \
                  - Switch to a more permissive mode, or \
                  - Use a different approach that stays within workspace."
                .to_string())
        } else if msg.contains("rate limit") || msg.contains("too many requests") {
            Some("Rate limited. Wait a moment before trying again, \
                  or use a smaller batch of operations."
                .to_string())
        } else {
            None
        }
    }

    /// Format an error with an optional suggestion for the agent.
    pub fn format_with_suggestion(error: &Error, action: &Action) -> String {
        let base = format!("Error ({}): {}", action.description(), error);
        // Try detailed suggestions first
        let detailed = crate::core::error_suggestions::analyze_error(&error.to_string());
        let suggestion = if let Some(ref d) = detailed {
            d.to_markdown()
        } else {
            Self::suggest(error).unwrap_or_default()
        };
        if suggestion.is_empty() {
            base
        } else {
            format!("{base}\n\nSuggestion: {suggestion}")
        }
    }
}

/// Tracks consecutive failures to detect patterns needing self-correction.
pub struct FailureTracker {
    /// Action type -> consecutive failure count
    failures: std::collections::HashMap<String, u32>,
}

impl FailureTracker {
    pub fn new() -> Self {
        Self {
            failures: std::collections::HashMap::new(),
        }
    }

    /// Record a failure for the given action type.
    pub fn record(&mut self, action_type: &str) {
        *self.failures.entry(action_type.to_string()).or_insert(0) += 1;
    }

    /// Check if there are too many consecutive failures for this action type.
    pub fn should_suggest_correction(&self, action_type: &str, threshold: u32) -> bool {
        self.failures.get(action_type).copied().unwrap_or(0) >= threshold
    }

    /// Get the failure count for an action type.
    pub fn count(&self, action_type: &str) -> u32 {
        self.failures.get(action_type).copied().unwrap_or(0)
    }

    /// Clear failures for an action type (on success).
    pub fn clear(&mut self, action_type: &str) {
        self.failures.remove(action_type);
    }

    /// Generate a self-correction prompt after repeated failures.
    pub fn correction_prompt(&self, action_type: &str) -> String {
        let count = self.count(action_type);
        let suggestion = match action_type {
            "read_file" => "Try listing the directory first to verify the file exists and get the exact path.",
            "write_file" => "Check that the directory exists, or use create_directory to make it first.",
            "delete_file" => "Verify the file exists. If deleting multiple files, list first to see what's there.",
            "execute_command" => "Try breaking the command into smaller steps, or check if you need a different approach.",
            "search_files" => "Try a simpler pattern like '*.txt' or list the directory first.",
            "list_directory" => "Check that the path exists and is a directory, not a file.",
            "copy_file" => "Verify both source and destination paths are valid. Source must exist.",
            "web_fetch" => "Check the URL is valid and accessible. Try http_request as an alternative.",
            "web_search" => "Try a simpler search query with fewer keywords.",
            _ => "Consider taking a different approach or calling finish if the task is complete.",
        };
        format!(
            "You've failed '{action_type}' {count} times in a row. \
             Think about what's going wrong and try something different.\n\nSuggestion: {suggestion}"
        )
    }
}

impl Default for FailureTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_suggest_file_not_found() {
        let err = Error::new(io::Error::new(io::ErrorKind::NotFound, "file not found: test.txt"));
        let s = ErrorClassifier::suggest(&err);
        assert!(s.unwrap().contains("listing the directory"));
    }

    #[test]
    fn test_suggest_permission_denied() {
        let err = Error::new(io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied"));
        let s = ErrorClassifier::suggest(&err);
        assert!(s.unwrap().contains("workspace"));
    }

    #[test]
    fn test_suggest_unknown() {
        let err = Error::msg("Something weird happened");
        let s = ErrorClassifier::suggest(&err);
        assert!(s.is_none());
    }

    #[test]
    fn test_failure_tracker() {
        let mut tracker = FailureTracker::new();
        tracker.record("read_file");
        tracker.record("read_file");
        assert_eq!(tracker.count("read_file"), 2);
        assert!(!tracker.should_suggest_correction("read_file", 3));
        tracker.record("read_file");
        assert!(tracker.should_suggest_correction("read_file", 3));
        tracker.clear("read_file");
        assert_eq!(tracker.count("read_file"), 0);
    }

    #[test]
    fn test_correction_prompt() {
        let mut tracker = FailureTracker::new();
        for _ in 0..4 {
            tracker.record("read_file");
        }
        let prompt = tracker.correction_prompt("read_file");
        assert!(prompt.contains("4 times"));
        assert!(prompt.contains("listing the directory"));
    }
}