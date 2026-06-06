use vo1d::agent::parser::ToolParser;
use vo1d::models::action::Action;

fn parser() -> ToolParser {
    ToolParser::new()
}

#[test]
fn test_parse_full_json_read_file() {
    let input = r#"{"action": "read_file", "path": "test.txt"}"#;
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::ReadFile { path, .. } if path == "test.txt"));
}

#[test]
fn test_parse_full_json_write_file() {
    let input = r#"{"action": "write_file", "path": "out.txt", "content": "hello"}"#;
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::WriteFile { path, content, .. } if path == "out.txt" && content == "hello"));
}

#[test]
fn test_parse_full_json_execute_command() {
    let input = r#"{"action": "execute_command", "command": "dir", "timeout": 30}"#;
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::ExecuteCommand { command, timeout, .. } if command == "dir" && timeout == Some(30)));
}

#[test]
fn test_parse_full_json_finish() {
    let input = r#"{"action": "finish", "output": "Done."}"#;
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::Finish { output } if output == Some("Done.".to_string())));
}

#[test]
fn test_parse_markdown_block() {
    let input = "Some thoughts...\n```json\n{\"action\": \"read_file\", \"path\": \"src/main.rs\"}\n```\nMore text.";
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::ReadFile { path, .. } if path == "src/main.rs"));
}

#[test]
fn test_parse_markdown_block_no_lang() {
    let input = "```\n{\"action\": \"list_directory\", \"path\": \".\"}\n```";
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::ListDirectory { path, .. } if path == "."));
}

#[test]
fn test_parse_curly_brace_extraction() {
    let input = "Here is the action: {\"action\": \"search_files\", \"pattern\": \"*.rs\"} please run it.";
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::SearchFiles { pattern, .. } if pattern == "*.rs"));
}

#[test]
fn test_parse_heuristic_execute_command() {
    let input = "I will execute command: cargo test";
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::ExecuteCommand { command, .. } if command == "cargo test"));
}

#[test]
fn test_parse_heuristic_read_file() {
    let input = "Let me read file: Cargo.toml";
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::ReadFile { path, .. } if path == "cargo.toml"));
}

#[test]
fn test_parse_heuristic_write_file() {
    let input = "I will write file: hello.txt content: world";
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::WriteFile { path, content, .. } if path == "hello.txt" && content == "world"));
}

#[test]
fn test_parse_heuristic_list_directory() {
    let input = "List directory: src";
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::ListDirectory { path, .. } if path == "src"));
}

#[test]
fn test_parse_heuristic_search_files() {
    let input = "Search: **/*.toml";
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::SearchFiles { pattern, .. } if pattern == "**/*.toml"));
}

#[test]
fn test_parse_invalid_input_fails() {
    let input = "This is just a random sentence with no action.";
    let result = parser().parse(input, false);
    assert!(result.is_err());
}

#[test]
fn test_parse_empty_input_fails() {
    let result = parser().parse("", false);
    assert!(result.is_err());
}

#[test]
fn test_parse_nested_curly_braces_in_content() {
    let input = r#"```json
{"action": "write_file", "path": "test.json", "content": "{\"key\": \"value\"}"}
```"#;
    let result = parser().parse(input, false).unwrap();
    assert!(matches!(result, Action::WriteFile { path, .. } if path == "test.json"));
}

#[test]
fn test_parse_native_tools_flag() {
    let input = r#"{"action": "read_file", "path": "test.txt"}"#;
    let result = parser().parse(input, true).unwrap();
    assert!(matches!(result, Action::ReadFile { path, .. } if path == "test.txt"));
}
