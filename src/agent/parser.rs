use crate::models::action::Action;
use anyhow::Result;
use regex::Regex;
use serde::Deserialize;
use tracing::warn;

/// Multi-phase tool parser for extracting structured actions from LLM output.
pub struct ToolParser {
    json_block_re: Regex,
}

impl ToolParser {
    pub fn new() -> Self {
        Self {
            json_block_re: Regex::new(r"```(?:json)?\s*([\s\S]*?)```")
                .expect("Failed to compile JSON block regex"),
        }
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
        self.parse_simulated(text)
    }

    /// Parse tool calls from simulated (non-tool) model output.
    fn parse_simulated(&self, text: &str) -> Result<Action> {
        // Phase 1: Try full JSON parse
        if let Ok(action) = serde_json::from_str::<Action>(text) {
            return Ok(action);
        }
        // Phase 1b: Try OpenAI-style {"name": "...", "arguments": {...}} format
        if let Some(action) = try_convert_openai_format(text) {
            return Ok(action);
        }

        // Phase 2: Extract from markdown code block
        if let Some(caps) = self.json_block_re.captures(text) {
            let extracted = caps.get(1).unwrap().as_str().trim();
            if let Ok(action) = serde_json::from_str::<Action>(extracted) {
                return Ok(action);
            }
            if let Some(action) = try_convert_openai_format(extracted) {
                return Ok(action);
            }
            // Try with smart quote cleaning
            let cleaned = clean_smart_quotes(extracted);
            if let Ok(action) = serde_json::from_str::<Action>(&cleaned) {
                return Ok(action);
            }
            if let Some(action) = try_convert_openai_format(&cleaned) {
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
                            if let Some(action) = try_convert_openai_format(candidate) {
                                return Some(action);
                            }
                            // Try cleaning smart quotes
                            let cleaned = clean_smart_quotes(candidate);
                            if let Ok(action) = serde_json::from_str::<Action>(&cleaned) {
                                return Some(action);
                            }
                            if let Some(action) = try_convert_openai_format(&cleaned) {
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

    /// Max length for heuristic-extracted values to prevent matching on natural language.
    const MAX_HEURISTIC_LEN: usize = 200;

    /// Heuristic parse for simple action patterns.
    fn heuristic_parse(&self, text: &str) -> Option<Action> {
        let lower = text.to_lowercase();

        // Pattern: "execute command: cargo test"
        if let Some(cmd) = self.extract_after_prefix(&lower, &["execute command:", "run command:", "command:"]) {
            if cmd.len() > Self::MAX_HEURISTIC_LEN { return None; }
            return Some(Action::ExecuteCommand {
                command: cmd.trim().to_string(),
                timeout: None,
                workdir: None,
            });
        }

        // Pattern: "read file: path/to/file.txt"
        if let Some(path) = self.extract_after_prefix(&lower, &["read file:", "open file:"]) {
            if path.len() > Self::MAX_HEURISTIC_LEN { return None; }
            return Some(Action::ReadFile {
                path: path.trim().to_string(),
                start_line: None,
                end_line: None,
            });
        }

        // Pattern: "write file: path/to/file.txt content: ..."
        if let Some(rest) = self.extract_after_prefix(&lower, &["write file:", "create file:"]) {
            if rest.len() > Self::MAX_HEURISTIC_LEN { return None; }
            if let Some((path, content)) = rest.split_once("content:") {
                return Some(Action::WriteFile {
                    path: path.trim().to_string(),
                    content: content.trim().to_string(),
                    append: None,
                });
            }
        }

        // Pattern: "list directory: path"
        if let Some(path) = self.extract_after_prefix(&lower, &["list directory:", "ls "]) {
            if path.len() > Self::MAX_HEURISTIC_LEN { return None; }
            return Some(Action::ListDirectory {
                path: path.trim().to_string(),
                pattern: None,
            });
        }

        // Pattern: "web search: query" (must be before "search" to avoid collision)
        if let Some(query) = self.extract_after_prefix(&lower, &["web search:", "search web:", "search for:"]) {
            if query.len() > Self::MAX_HEURISTIC_LEN { return None; }
            return Some(Action::WebSearch {
                query: query.trim().to_string(),
                num_results: None,
            });
        }

        // Pattern: "web fetch: url"
        if let Some(url) = self.extract_after_prefix(&lower, &["web fetch:", "fetch url:"]) {
            if url.len() > Self::MAX_HEURISTIC_LEN { return None; }
            return Some(Action::WebFetch {
                url: url.trim().to_string(),
                max_chars: None,
            });
        }

        // Pattern: search files
        if let Some(pat) = self.extract_after_prefix(&lower, &["search files:", "find files:", "glob:"]) {
            if pat.len() > Self::MAX_HEURISTIC_LEN { return None; }
            return Some(Action::SearchFiles {
                pattern: pat.trim().to_string(),
                path: None,
                search_type: None,
            });
        }

        // Pattern: "delete file: path"
        if let Some(path) = self.extract_after_prefix(&lower, &["delete file:", "remove file:"]) {
            if path.len() > Self::MAX_HEURISTIC_LEN { return None; }
            return Some(Action::DeleteFile {
                path: path.trim().to_string(),
                pattern: None,
            });
        }

        // Pattern: "copy file: source -> destination"
        if let Some(rest) = self.extract_after_prefix(&lower, &["copy file:"]) {
            if rest.len() > Self::MAX_HEURISTIC_LEN { return None; }
            if let Some((src, dst)) = rest.split_once("->").or_else(|| rest.split_once(" to ")) {
                return Some(Action::CopyFile {
                    source: src.trim().to_string(),
                    destination: dst.trim().to_string(),
                });
            }
        }

        // Pattern: "create directory: path"
        if let Some(path) = self.extract_after_prefix(&lower, &["create directory:", "mkdir:", "create dir:"]) {
            if path.len() > Self::MAX_HEURISTIC_LEN { return None; }
            return Some(Action::CreateDirectory {
                path: path.trim().to_string(),
            });
        }

        // Pattern: "file metadata: path"
        if let Some(path) = self.extract_after_prefix(&lower, &["file metadata:", "metadata:", "stat:", "file info:"]) {
            if path.len() > Self::MAX_HEURISTIC_LEN { return None; }
            return Some(Action::FileMetadata {
                path: path.trim().to_string(),
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

/// Try to convert OpenAI-style {"name": "...", "arguments": {...}} to Action tagged format.
fn try_convert_openai_format(text: &str) -> Option<Action> {
    let val: serde_json::Value = serde_json::from_str(text).ok()?;
    let name = val.get("name")?.as_str()?;
    let args = val.get("arguments")?;

    let mut action_obj = serde_json::Map::new();
    action_obj.insert("action".into(), serde_json::Value::String(name.into()));

    if let Some(obj) = args.as_object() {
        for (k, v) in obj {
            action_obj.insert(k.clone(), v.clone());
        }
    } else if let Some(s) = args.as_str() {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(s) {
            if let Some(obj) = parsed.as_object() {
                for (k, v) in obj {
                    action_obj.insert(k.clone(), v.clone());
                }
            }
        }
    }

    Action::deserialize(serde_json::Value::Object(action_obj)).ok()
}

/// Clean smart/unicode quotes from text, replacing them with ASCII equivalents.
fn clean_smart_quotes(text: &str) -> String {
    text
        .replace('\u{201C}', "\"")   // Left double smart quote
        .replace('\u{201D}', "\"")   // Right double smart quote
        .replace('\u{2018}', "'")    // Left single smart quote
        .replace('\u{2019}', "'")    // Right single smart quote
        .replace('\u{201E}', "\"")   // Double low-9 quote
        .replace('\u{201A}', "'")    // Single low-9 quote
        .replace('\u{00AB}', "\"")   // Left-pointing double angle
        .replace('\u{00BB}', "\"")   // Right-pointing double angle
}
