use crate::models::action::Action;
use anyhow::Result;
use regex::Regex;
use tracing::warn;

/// Multi-phase tool parser for extracting structured actions from LLM output.
pub struct ToolParser;

impl ToolParser {
    pub fn new() -> Self {
        Self
    }

    /// Parse an action from model response text.
    /// Uses fallback chain: full JSON -> markdown block -> curly braces -> heuristics
    pub fn parse(&self, text: &str, supports_native_tools: bool) -> Result<Action> {
        if supports_native_tools {
            return self.parse_native(text);
        }
        self.parse_simulated(text)
    }

    /// Parse native tool calls (from tool-capable models).
    fn parse_native(&self, text: &str) -> Result<Action> {
        // Tool-native models return tool_calls separately; here we parse the text
        self.parse_simulated(text)
    }

    /// Parse tool calls from simulated (non-tool) model output.
    fn parse_simulated(&self, text: &str) -> Result<Action> {
        // Phase 1: Try full JSON parse
        if let Ok(action) = serde_json::from_str::<Action>(text) {
            return Ok(action);
        }

        // Phase 2: Extract from markdown code block
        let re = Regex::new(r"```(?:json)?\s*([\s\S]*?)```")?;
        if let Some(caps) = re.captures(text) {
            let extracted = caps.get(1).unwrap().as_str().trim();
            if let Ok(action) = serde_json::from_str::<Action>(extracted) {
                return Ok(action);
            }
            // Try with escaped quotes
            let cleaned = extracted.replace('\"', "\"").replace('\'', "\"");
            if let Ok(action) = serde_json::from_str::<Action>(&cleaned) {
                return Ok(action);
            }
        }

        // Phase 3: Extract outermost curly braces
        if let Some(action) = self.extract_curly_brace_json(text) {
            return Ok(action);
        }

        // Phase 4: Heuristic fallback for simple command/action patterns
        if let Some(action) = self.heuristic_parse(text) {
            warn!("Heuristic fallback parsed action from: {}", text.chars().take(100).collect::<String>());
            return Ok(action);
        }

        // Phase 5: If all parsing fails, treat the text as a Finish action
        warn!(
            "No structured action found; treating as Finish output: {}",
            text.chars().take(200).collect::<String>()
        );
        return Ok(Action::Finish {
            output: Some(text.trim().to_string()),
        });
    }

    /// Extract JSON from outermost `{...}` using brace matching.
    fn extract_curly_brace_json(&self, text: &str) -> Option<Action> {
        let mut depth = 0i32;
        let mut start = None;

        for (i, ch) in text.char_indices() {
            match ch {
                '{' => {
                    if depth == 0 {
                        start = Some(i);
                    }
                    depth += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(s) = start {
                            let candidate = &text[s..=i];
                            if let Ok(action) = serde_json::from_str::<Action>(candidate) {
                                return Some(action);
                            }
                            // Try cleaning quotes
                            let cleaned = candidate.replace('\"', "\"").replace('\'', "\"");
                            if let Ok(action) = serde_json::from_str::<Action>(&cleaned) {
                                return Some(action);
                            }
                        }
                        start = None;
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Heuristic parse for simple action patterns.
    fn heuristic_parse(&self, text: &str) -> Option<Action> {
        let lower = text.to_lowercase();

        // Pattern: "execute command: cargo test"
        if let Some(cmd) = self.extract_after_prefix(&lower, &["execute command:", "run command:", "command:"]) {
            return Some(Action::ExecuteCommand {
                command: cmd.trim().to_string(),
                timeout: None,
                workdir: None,
            });
        }

        // Pattern: "read file: path/to/file.txt"
        if let Some(path) = self.extract_after_prefix(&lower, &["read file:", "read:", "open file:"]) {
            return Some(Action::ReadFile {
                path: path.trim().to_string(),
                start_line: None,
                end_line: None,
            });
        }

        // Pattern: "write file: path/to/file.txt content: ..."
        if let Some(rest) = self.extract_after_prefix(&lower, &["write file:", "write:", "create file:"]) {
            if let Some((path, content)) = rest.split_once("content:") {
                return Some(Action::WriteFile {
                    path: path.trim().to_string(),
                    content: content.trim().to_string(),
                    append: None,
                });
            }
        }

        // Pattern: "list directory: path"
        if let Some(path) = self.extract_after_prefix(&lower, &["list directory:", "list:", "ls "]) {
            return Some(Action::ListDirectory {
                path: path.trim().to_string(),
                pattern: None,
            });
        }

        // Pattern: "search: pattern"
        if let Some(pat) = self.extract_after_prefix(&lower, &["search:", "find:", "glob:"]) {
            return Some(Action::SearchFiles {
                pattern: pat.trim().to_string(),
                path: None,
                search_type: None,
            });
        }

        // Pattern: "delete file: path"
        if let Some(path) = self.extract_after_prefix(&lower, &["delete file:", "remove file:", "delete:", "remove:"]) {
            return Some(Action::DeleteFile {
                path: path.trim().to_string(),
                pattern: None,
            });
        }

        // Pattern: "copy file: source -> destination"
        if let Some(rest) = self.extract_after_prefix(&lower, &["copy file:", "copy:"]) {
            if let Some((src, dst)) = rest.split_once("->").or_else(|| rest.split_once(" to ")) {
                return Some(Action::CopyFile {
                    source: src.trim().to_string(),
                    destination: dst.trim().to_string(),
                });
            }
        }

        // Pattern: "create directory: path"
        if let Some(path) = self.extract_after_prefix(&lower, &["create directory:", "mkdir:", "create dir:"]) {
            return Some(Action::CreateDirectory {
                path: path.trim().to_string(),
            });
        }

        // Pattern: "file metadata: path"
        if let Some(path) = self.extract_after_prefix(&lower, &["file metadata:", "metadata:", "stat:", "file info:"]) {
            return Some(Action::FileMetadata {
                path: path.trim().to_string(),
            });
        }

        // Pattern: "web search: query"
        if let Some(query) = self.extract_after_prefix(&lower, &["web search:", "search web:", "search for:"]) {
            return Some(Action::WebSearch {
                query: query.trim().to_string(),
                num_results: None,
            });
        }

        // Pattern: "web fetch: url"
        if let Some(url) = self.extract_after_prefix(&lower, &["web fetch:", "fetch url:", "fetch:"]) {
            return Some(Action::WebFetch {
                url: url.trim().to_string(),
                max_chars: None,
            });
        }

        None
    }

    fn extract_after_prefix(&self, text: &str, prefixes: &[&str]) -> Option<String> {
        for prefix in prefixes {
            if let Some(idx) = text.find(prefix) {
                let rest = &text[idx + prefix.len()..];
                // Take up to next newline or end
                let result = rest.lines().next().unwrap_or(rest).trim();
                if !result.is_empty() {
                    return Some(result.to_string());
                }
            }
        }
        None
    }
}
