use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents an action the agent can take.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Action {
    #[serde(rename = "read_file")]
    ReadFile {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_line: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_line: Option<usize>,
    },
    #[serde(rename = "write_file")]
    WriteFile {
        path: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        append: Option<bool>,
    },
    #[serde(rename = "execute_command")]
    ExecuteCommand {
        command: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        workdir: Option<String>,
    },
    #[serde(rename = "list_directory")]
    ListDirectory {
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pattern: Option<String>,
    },
    #[serde(rename = "search_files")]
    SearchFiles {
        pattern: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(rename = "type")]
        search_type: Option<String>,
    },
    #[serde(rename = "delete_file", alias = "delete_files")]
    DeleteFile {
        #[serde(default = "default_dot")]
        path: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pattern: Option<String>,
    },
    #[serde(rename = "copy_file")]
    CopyFile {
        source: String,
        destination: String,
    },
    #[serde(rename = "create_directory")]
    CreateDirectory {
        path: String,
    },
    #[serde(rename = "file_metadata")]
    FileMetadata {
        path: String,
    },
    #[serde(rename = "http_request")]
    HttpRequest {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        method: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<String>,
    },
    #[serde(rename = "finish")]
    Finish {
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    },
    #[serde(rename = "ask_user")]
    AskUser {
        question: String,
    },
}

fn default_dot() -> String {
    ".".to_string()
}

impl Action {
    /// Returns a human-readable description of this action.
    pub fn description(&self) -> String {
        match self {
            Action::ReadFile { path, .. } => format!("Read file: {}", path),
            Action::WriteFile { path, .. } => format!("Write file: {}", path),
            Action::ExecuteCommand { command, .. } => format!("Execute: {}", command),
            Action::ListDirectory { path, .. } => format!("List directory: {}", path),
            Action::SearchFiles { pattern, .. } => format!("Search: {}", pattern),
            Action::DeleteFile { path, pattern } => {
                if let Some(pat) = pattern {
                    format!("Delete files matching '{}' in {}", pat, path)
                } else {
                    format!("Delete file: {}", path)
                }
            }
            Action::CopyFile { source, destination } => format!("Copy {} to {}", source, destination),
            Action::CreateDirectory { path } => format!("Create directory: {}", path),
            Action::FileMetadata { path } => format!("Metadata: {}", path),
            Action::HttpRequest { url, .. } => format!("HTTP request: {}", url),
            Action::Finish { output: _ } => "Finish task".to_string(),
            Action::AskUser { question } => format!("Ask user: {}", question),
        }
    }

    /// Returns true if this action is potentially destructive.
    pub fn is_destructive(&self) -> bool {
        matches!(self, Action::DeleteFile { .. } | Action::WriteFile { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_description_read_file() {
        let a = Action::ReadFile { path: "test.txt".to_string(), start_line: None, end_line: None };
        assert_eq!(a.description(), "Read file: test.txt");
    }

    #[test]
    fn test_action_description_write_file() {
        let a = Action::WriteFile { path: "out.txt".to_string(), content: "hi".to_string(), append: None };
        assert_eq!(a.description(), "Write file: out.txt");
    }

    #[test]
    fn test_action_description_execute_command() {
        let a = Action::ExecuteCommand { command: "echo hi".to_string(), timeout: None, workdir: None };
        assert_eq!(a.description(), "Execute: echo hi");
    }

    #[test]
    fn test_action_description_list_directory() {
        let a = Action::ListDirectory { path: "/tmp".to_string(), pattern: None };
        assert_eq!(a.description(), "List directory: /tmp");
    }

    #[test]
    fn test_action_description_search_files() {
        let a = Action::SearchFiles { pattern: "*.rs".to_string(), path: None, search_type: None };
        assert_eq!(a.description(), "Search: *.rs");
    }

    #[test]
    fn test_action_description_delete_file() {
        let a = Action::DeleteFile { path: "old.txt".to_string(), pattern: None };
        assert_eq!(a.description(), "Delete file: old.txt");
    }

    #[test]
    fn test_action_description_delete_files_pattern() {
        let a = Action::DeleteFile { path: ".".to_string(), pattern: Some("*.txt".to_string()) };
        assert_eq!(a.description(), "Delete files matching '*.txt' in .");
    }

    #[test]
    fn test_action_description_copy_file() {
        let a = Action::CopyFile { source: "a.txt".to_string(), destination: "b.txt".to_string() };
        assert_eq!(a.description(), "Copy a.txt to b.txt");
    }

    #[test]
    fn test_action_description_create_directory() {
        let a = Action::CreateDirectory { path: "new_dir".to_string() };
        assert_eq!(a.description(), "Create directory: new_dir");
    }

    #[test]
    fn test_action_description_file_metadata() {
        let a = Action::FileMetadata { path: "f.txt".to_string() };
        assert_eq!(a.description(), "Metadata: f.txt");
    }

    #[test]
    fn test_action_description_http_request() {
        let a = Action::HttpRequest { url: "http://example.com".to_string(), method: None, headers: None, body: None };
        assert_eq!(a.description(), "HTTP request: http://example.com");
    }

    #[test]
    fn test_action_description_finish() {
        let a = Action::Finish { output: Some("done".to_string()) };
        assert_eq!(a.description(), "Finish task");
    }

    #[test]
    fn test_action_description_ask_user() {
        let a = Action::AskUser { question: "Proceed?".to_string() };
        assert_eq!(a.description(), "Ask user: Proceed?");
    }

    #[test]
    fn test_action_is_destructive() {
        assert!(Action::DeleteFile { path: "x".to_string(), pattern: None }.is_destructive());
        assert!(Action::WriteFile { path: "x".to_string(), content: "".to_string(), append: None }.is_destructive());
        assert!(!Action::ReadFile { path: "x".to_string(), start_line: None, end_line: None }.is_destructive());
        assert!(!Action::ExecuteCommand { command: "ls".to_string(), timeout: None, workdir: None }.is_destructive());
        assert!(!Action::Finish { output: None }.is_destructive());
    }

    #[test]
    fn test_action_serde_roundtrip_read_file() {
        let original = Action::ReadFile { path: "test.txt".to_string(), start_line: None, end_line: None };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Action = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Action::ReadFile { path, .. } if path == "test.txt"));
    }

    #[test]
    fn test_action_serde_roundtrip_execute_command() {
        let original = Action::ExecuteCommand {
            command: "cargo test".to_string(),
            timeout: Some(60),
            workdir: Some("/tmp".to_string()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: Action = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, Action::ExecuteCommand { command, timeout, workdir }
            if command == "cargo test" && timeout == Some(60) && workdir == Some("/tmp".to_string())));
    }

    #[test]
    fn test_action_serde_tagged_deserialization() {
        let json = r#"{"action": "finish", "output": "All done"}"#;
        let action: Action = serde_json::from_str(json).unwrap();
        assert!(matches!(action, Action::Finish { output } if output == Some("All done".to_string())));
    }
}
