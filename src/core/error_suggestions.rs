use regex::Regex;

/// Structured error message with suggestions for self-correction.
#[derive(Debug, Clone)]
pub struct ErrorSuggestion {
    pub title: String,
    pub message: String,
    pub suggestions: Vec<String>,
    pub markdown: String,
}

impl ErrorSuggestion {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            suggestions: Vec::new(),
            markdown: String::new(),
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }

    pub fn with_suggestions(mut self, suggestions: Vec<impl Into<String>>) -> Self {
        for s in suggestions {
            self.suggestions.push(s.into());
        }
        self
    }

    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("### ⚠️  {}  \n\n", self.title));
        md.push_str(&format!("{}\n\n", self.message));
        if !self.suggestions.is_empty() {
            md.push_str("#### Suggestions:\n\n");
            for (i, s) in self.suggestions.iter().enumerate() {
                md.push_str(&format!("{}. {}\n", i + 1, s));
            }
        }
        md
    }
}

/// Analyze an error message and return structured suggestions.
pub fn analyze_error(error_msg: &str) -> Option<ErrorSuggestion> {
    // File not found
    if error_msg.contains("No such file or directory") ||
       error_msg.contains("File not found") ||
       error_msg.contains("cannot find the file") ||
       error_msg.contains("does not exist") {
        return Some(file_not_found_suggestion(error_msg));
    }

    // Permission denied
    if error_msg.contains("Permission denied") ||
       error_msg.contains("access denied") ||
       error_msg.contains("requires elevation") {
        return Some(permission_denied_suggestion(error_msg));
    }

    // Command not found
    if error_msg.contains("not recognized") ||
       error_msg.contains("command not found") ||
       error_msg.contains("is not recognized as an internal") {
        return Some(command_not_found_suggestion(error_msg));
    }

    // Timeout
    if error_msg.contains("timed out") || error_msg.contains("timeout") {
        return Some(timeout_suggestion(error_msg));
    }

    // Invalid JSON
    if error_msg.contains("invalid JSON") ||
       error_msg.contains("expected `,` or `}`") ||
       error_msg.contains("trailing comma") {
        return Some(invalid_json_suggestion(error_msg));
    }

    // File already exists
    if error_msg.contains("already exists") ||
       error_msg.contains("File exists") {
        return Some(file_exists_suggestion(error_msg));
    }

    // Directory not empty
    if error_msg.contains("Directory not empty") ||
       error_msg.contains("directory is not empty") {
        return Some(directory_not_empty_suggestion(error_msg));
    }

    // Pattern not found
    if error_msg.contains("No files matching") ||
       error_msg.contains("no matches found") {
        return Some(pattern_not_found_suggestion(error_msg));
    }

    // Network error
    if error_msg.contains("network") ||
       error_msg.contains("connection refused") ||
       error_msg.contains("DNS lookup failed") ||
       error_msg.contains("reqwest") {
        return Some(network_error_suggestion(error_msg));
    }

    None
}

fn file_not_found_suggestion(error: &str) -> ErrorSuggestion {
    let mut sug = ErrorSuggestion::new(
        "File Not Found",
        format!("The file you tried to access does not exist: {}", extract_path(error))
    );
        sug = sug.with_suggestions(vec![
        format!("Check the filename spelling: {}", extract_filename(error)),
        "Use list_directory to see what files exist in the directory".to_string(),
        "If you meant a different path, use the correct relative path from workspace".to_string(),
        "For glob patterns, use '*' not '*.*' to match all files".to_string(),
    ]);
    sug
}

fn permission_denied_suggestion(error: &str) -> ErrorSuggestion {
    let mut sug = ErrorSuggestion::new(
        "Permission Denied",
        format!("You don't have permission to access: {}", extract_path(error))
    );
        sug = sug.with_suggestions(vec![
        "Try a different action that doesn't require write access".to_string(),
        "Use read_file or list_directory instead of write operations".to_string(),
        "If you need to delete, use delete_file with a specific path".to_string(),
        "Check security mode — you may be in Safe mode".to_string(),
    ]);
    sug
}

fn command_not_found_suggestion(error: &str) -> ErrorSuggestion {
    let cmd = extract_command(error);
    let mut sug = ErrorSuggestion::new(
        "Command Not Found",
        format!("The command '{}' is not available on this system", cmd)
    );
        sug = sug.with_suggestions(vec![
        format!("Check if '{}' is spelled correctly", cmd),
        "Use built-in actions like read_file, write_file instead of shell commands when possible".to_string(),
        "For Windows, use 'dir' not 'ls'; for Unix, use 'ls' not 'dir'".to_string(),
        "Use execute_command only when absolutely necessary".to_string(),
    ]);
    sug
}

fn timeout_suggestion(_error: &str) -> ErrorSuggestion {
    let mut sug = ErrorSuggestion::new(
        "Command Timeout",
        "The command took too long to complete and was stopped".to_string()
    );
    sug = sug.with_suggestions(vec![
        "Try a simpler command or break it into smaller steps",
        "Increase timeout with 'timeout' parameter in execute_command",
        "Avoid long-running operations like downloads or builds",
        "Use list_directory instead of 'ls -R' for large directories",
    ]);
    sug
}

fn invalid_json_suggestion(_error: &str) -> ErrorSuggestion {
    let mut sug = ErrorSuggestion::new(
        "Invalid JSON",
        "Your action JSON is malformed and could not be parsed".to_string()
    );
    sug = sug.with_suggestions(vec![
        "Check for missing commas or quotes in your JSON",
        "Use double quotes \" not single quotes",
        "Validate your JSON structure",
        "Example: {\"action\": \"read_file\", \"path\": \"file.txt\"}",
    ]);
    sug
}

fn file_exists_suggestion(error: &str) -> ErrorSuggestion {
    let path = extract_path(error);
    let mut sug = ErrorSuggestion::new(
        "File Already Exists",
        format!("Cannot create file because it already exists: {}", path)
    );
    sug = sug.with_suggestions(vec![
        "Use write_file with append: true to add content",
        "Delete the file first with delete_file",
        "Choose a different filename",
        "Check if the file is needed first with file_metadata",
    ]);
    sug
}

fn directory_not_empty_suggestion(error: &str) -> ErrorSuggestion {
    let path = extract_path(error);
    let mut sug = ErrorSuggestion::new(
        "Directory Not Empty",
        format!("Cannot delete directory because it contains files: {}", path)
    );
    sug = sug.with_suggestions(vec![
        "List the directory first with list_directory",
        "Delete files inside first with delete_file using a pattern",
        "Use delete_file with pattern: '*' to delete all contents",
        "Then delete the empty directory",
    ]);
    sug
}

fn pattern_not_found_suggestion(_error: &str) -> ErrorSuggestion {
    let mut sug = ErrorSuggestion::new(
        "Pattern Not Found",
        "No files matched your search pattern".to_string()
    );
    sug = sug.with_suggestions(vec![
        "Check your glob pattern syntax",
        "Use '*' to match all files (not '*.*')",
        "Try list_directory first to see what files exist",
        "Use search_files with pattern only (no path) for recursive search",
    ]);
    sug
}

fn network_error_suggestion(_error: &str) -> ErrorSuggestion {
    let mut sug = ErrorSuggestion::new(
        "Network Error",
        "A network request failed".to_string()
    );
    sug = sug.with_suggestions(vec![
        "Check the URL is correct",
        "Try again later — the site may be down",
        "Use web_search instead of direct HTTP requests when possible",
        "If using http_request, check method and headers",
    ]);
    sug
}

fn extract_path(error: &str) -> String {
    let re = Regex::new(r#"['"]([^'"\\]+)['"]"#).unwrap();
    if let Some(captures) = re.captures(error) {
        if let Some(m) = captures.get(1) {
            return m.as_str().to_string();
        }
    }
    error.split_whitespace().last().unwrap_or("").to_string()
}

fn extract_filename(error: &str) -> String {
    let path = extract_path(error);
    std::path::Path::new(&path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&path)
        .to_string()
}

fn extract_command(error: &str) -> String {
    let parts = error.split_whitespace().collect::<Vec<_>>();
    for p in &parts {
        if p.contains("not recognized") || p.contains("command not found") {
            if let Some(prev) = parts.iter().position(|x| x == p).and_then(|i| i.checked_sub(1)) {
                return parts[prev].to_string();
            }
        }
    }
    error.split(':').next().unwrap_or("").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_file_not_found() {
        let e = analyze_error("No such file or directory: 'file.txt'").unwrap();
        assert_eq!(e.title, "File Not Found");
        assert!(e.to_markdown().contains("file.txt"));
        assert!(e.suggestions.len() >= 4);
    }

    #[test]
    fn test_analyze_permission_denied() {
        let e = analyze_error("Permission denied: /etc/passwd").unwrap();
        assert_eq!(e.title, "Permission Denied");
        assert!(e.message.contains("/etc/passwd"));
    }

    #[test]
    fn test_analyze_command_not_found() {
        let e = analyze_error("'cargoo' is not recognized as an internal or external command").unwrap();
        assert_eq!(e.title, "Command Not Found");
        assert!(e.message.contains("cargoo"));
    }

    #[test]
    fn test_analyze_timeout() {
        let e = analyze_error("Command timed out after 60 seconds").unwrap();
        assert_eq!(e.title, "Command Timeout");
    }

    #[test]
    fn test_analyze_invalid_json() {
        let e = analyze_error("invalid JSON: expected `,` or `}` at line 1 column 20").unwrap();
        assert_eq!(e.title, "Invalid JSON");
    }

    #[test]
    fn test_analyze_file_exists() {
        let e = analyze_error("File already exists: /path/to/file.txt").unwrap();
        assert_eq!(e.title, "File Already Exists");
    }

    #[test]
    fn test_analyze_directory_not_empty() {
        let e = analyze_error("Directory not empty: /path/to/dir").unwrap();
        assert_eq!(e.title, "Directory Not Empty");
    }

    #[test]
    fn test_analyze_pattern_not_found() {
        let e = analyze_error("No files matching '*.rs' found").unwrap();
        assert_eq!(e.title, "Pattern Not Found");
    }

    #[test]
    fn test_analyze_network_error() {
        let e = analyze_error("reqwest::Error: connection refused").unwrap();
        assert_eq!(e.title, "Network Error");
    }

    #[test]
    fn test_analyze_unknown_error() {
        let e = analyze_error("Some random error happened");
        assert!(e.is_none());
    }

    #[test]
    fn test_markdown_format() {
        let e = analyze_error("File not found: test.txt").unwrap();
        let md = e.to_markdown();
        assert!(md.starts_with("### ⚠️  File Not Found"));
        assert!(md.contains("#### Suggestions:"));
    }
}
