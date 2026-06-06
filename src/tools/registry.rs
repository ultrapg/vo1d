use crate::models::message::Tool;
use crate::AppContext;
use std::collections::HashMap;

/// Tool metadata for registration.
pub struct ToolInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: serde_json::Value,
}

/// Registry of all available tools.
pub struct ToolRegistry {
    tools: HashMap<String, ToolInfo>,
}

impl ToolRegistry {
    pub fn new(_ctx: &AppContext) -> Self {
        let mut tools = HashMap::new();

        tools.insert("read_file".to_string(), ToolInfo {
            name: "read_file",
            description: "Read the contents of a file. Supports line range selection.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the file" },
                    "start_line": { "type": "integer", "description": "Starting line number (optional)" },
                    "end_line": { "type": "integer", "description": "Ending line number (optional)" }
                },
                "required": ["path"]
            }),
        });

        tools.insert("write_file".to_string(), ToolInfo {
            name: "write_file",
            description: "Write content to a file. Creates parent directories if needed.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the file" },
                    "content": { "type": "string", "description": "File content" },
                    "append": { "type": "boolean", "description": "Append to existing file" }
                },
                "required": ["path", "content"]
            }),
        });

        tools.insert("execute_command".to_string(), ToolInfo {
            name: "execute_command",
            description: "Execute a shell command and return its output.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Command to execute" },
                    "timeout": { "type": "integer", "description": "Timeout in seconds" },
                    "workdir": { "type": "string", "description": "Working directory" }
                },
                "required": ["command"]
            }),
        });

        tools.insert("list_directory".to_string(), ToolInfo {
            name: "list_directory",
            description: "List contents of a directory.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Directory path" },
                    "pattern": { "type": "string", "description": "Glob pattern filter" }
                },
                "required": ["path"]
            }),
        });

        tools.insert("search_files".to_string(), ToolInfo {
            name: "search_files",
            description: "Search for files matching a pattern.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "Search pattern (glob or regex)" },
                    "path": { "type": "string", "description": "Directory to search in" },
                    "type": { "type": "string", "description": "Search type: glob, regex, name", "enum": ["glob", "regex", "name"] }
                },
                "required": ["pattern"]
            }),
        });

        tools.insert("file_metadata".to_string(), ToolInfo {
            name: "file_metadata",
            description: "Get metadata about a file or directory.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Path to the file" }
                },
                "required": ["path"]
            }),
        });

        tools.insert("finish".to_string(), ToolInfo {
            name: "finish",
            description: "Signal that the task is complete.",
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "output": { "type": "string", "description": "Task completion summary" }
                }
            }),
        });

        Self { tools }
    }

    /// Get all tools as the format needed for LLM API calls.
    pub fn as_llm_tools(&self) -> Vec<Tool> {
        self.tools.values().map(|t| Tool {
            name: t.name.to_string(),
            description: t.description.to_string(),
            parameters: t.parameters.clone(),
        }).collect()
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<&ToolInfo> {
        self.tools.get(name)
    }

    /// Check if an action name is a registered tool.
    pub fn is_registered(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}
