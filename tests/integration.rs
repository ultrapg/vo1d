use vo1d::core::compression::{CompressionConfig, ContextCompressor, TokenCounter};
use vo1d::core::curriculum::{Curriculum, EvaluationCriteria, TaskDefinition, evaluate_task};
use vo1d::core::docs::DocProvider;
use vo1d::core::error_suggestions::analyze_error;
use vo1d::core::self_correction::{ErrorClassifier, FailureTracker};
use vo1d::models::action::Action;
use vo1d::models::message::Message;
use std::path::Path;

// ── Helpers ──

fn msg(role: &str, content: &str) -> Message {
    match role {
        "system" => Message::system(content),
        "user" => Message::user(content),
        "assistant" => Message::assistant(content),
        "tool" => Message::tool(content, "call_integration"),
        _ => Message::user(content),
    }
}

// ── 1. Compression Integration ──

#[test]
fn test_compression_pipeline_full() {
    let config = CompressionConfig {
        enabled: true,
        max_tool_output_chars: 100,
        min_recent_messages: 4,
        target_usage: 0.80,
    };
    let compressor = ContextCompressor::new(config);

    let mut conv = vec![msg("system", "You are VO1D"), msg("user", "do the task")];
    for i in 0..12 {
        conv.push(msg("assistant", &format!("response number {}", i)));
        let long = "x".repeat(500);
        conv.push(msg("tool", &format!("{} output: {}", i, long)));
    }

    let total_before = conv.len();
    let (compressed, summary) = compressor.compress(&conv, 256);
    assert!(compressed.len() < total_before, "Should compress {} -> {}", total_before, compressed.len());
    assert!(compressed[0].role == "system", "First message should be system");
    assert!(compressed.iter().any(|m| m.role == "user"), "Should keep user task");
    // Tool outputs should be truncated (100 chars + suffix " ... [truncated N chars]" ~26 chars ≈ 126)
    for m in &compressed {
        if m.role == "tool" {
            assert!(m.content.len() <= 130, "Tool output should be truncated, was {}", m.content.len());
        }
    }
    // Summary should exist if compression happened
    if total_before > compressed.len() {
        assert!(!summary.is_empty(), "Should have summary when compression happens");
    }
}

#[test]
fn test_token_counter_estimate_conversation() {
    let conv = vec![msg("system", "sys"), msg("user", "hello world")];
    let tokens = TokenCounter::estimate_conversation(&conv);
    assert!(tokens >= 12, "Should estimate at least 12 tokens for 2 messages");
}

// ── 3. Self-Correction Integration ──

#[test]
fn test_error_classifier_with_various_errors() {
    use anyhow::Error;

    let cases: Vec<(&str, bool)> = vec![
        ("file not found", true),
        ("access denied", true),
        ("operation timed out", true),
        ("something else", false),
    ];

    for (msg, should_have_suggestion) in &cases {
        let err = Error::msg(msg.to_string());
        let sug = ErrorClassifier::suggest(&err);
        assert_eq!(sug.is_some(), *should_have_suggestion,
            "Error '{}' should{} have suggestion", msg, if *should_have_suggestion { "" } else { " not" });
    }
}

#[test]
fn test_failure_tracker_full_cycle() {
    let mut tracker = FailureTracker::new();
    assert_eq!(tracker.count("read_file"), 0);

    tracker.record("read_file");
    tracker.record("read_file");
    assert!(!tracker.should_suggest_correction("read_file", 3));
    tracker.record("read_file");
    assert!(tracker.should_suggest_correction("read_file", 3));

    let prompt = tracker.correction_prompt("read_file");
    assert!(prompt.contains("read_file"));
    assert!(prompt.contains("3 times"));

    tracker.clear("read_file");
    assert_eq!(tracker.count("read_file"), 0);
}

#[test]
fn test_error_suggestions_roundtrip() {
    let cases = [
        ("No such file or directory: 'test.txt'", "File Not Found"),
        ("Permission denied: /etc/passwd", "Permission Denied"),
        ("'cargoo' is not recognized", "Command Not Found"),
        ("timed out after 60 seconds", "Command Timeout"),
        ("invalid JSON: expected `,` or `}`", "Invalid JSON"),
        ("connection refused", "Network Error"),
    ];

    for (input, expected_title) in &cases {
        let result = analyze_error(input);
        assert!(result.is_some(), "Error '{}' should be recognized", input);
        assert_eq!(result.unwrap().title, *expected_title);
    }
}

// ── 4. Curriculum Integration ──

#[test]
fn test_curriculum_loads_real_files() {
    let curriculums = [
        "curriculum/00_hello_world.json",
        "curriculum/01_file_ops.json",
        "curriculum/02_directory_ops.json",
        "curriculum/03_search_nav.json",
        "curriculum/04_shell_basics.json",
        "curriculum/05_web_basics.json",
    ];
    for path in &curriculums {
        let c = Curriculum::load(path)
            .unwrap_or_else(|e| panic!("Failed to load {}: {}", path, e));
        assert!(!c.name.is_empty(), "Curriculum {} should have a name", path);
        assert!(!c.tasks.is_empty(), "Curriculum {} should have tasks", path);
        for task in &c.tasks {
            assert!(!task.id.is_empty(), "Task in {} should have id", path);
            assert!(!task.description.is_empty(), "Task {} should have description", task.id);
        }
    }
}

#[test]
fn test_evaluate_task_integration() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("exists.txt"), "hello world").unwrap();
    std::fs::create_dir(dir.path().join("mydir")).unwrap();

    let task = TaskDefinition {
        id: "integration_test".to_string(),
        description: "Test task".to_string(),
        expected_outcome: "Done".to_string(),
        evaluation: Some(EvaluationCriteria {
            check_file_exists: Some(vec!["exists.txt".to_string()]),
            check_file_content: Some(vec!["exists.txt::world".to_string()]),
            check_directory_exists: Some(vec!["mydir".to_string()]),
            check_command_output: None,
        }),
        setup: None,
    };

    let result = evaluate_task(&task, dir.path());
    assert!(result.passed, "All checks should pass: {:?}", result.details);
    assert_eq!(result.task_id, "integration_test");
}

#[test]
fn test_curriculum_deserialize_full() {
    // Verify the full curriculum files deserialize correctly by loading each
    for entry in std::fs::read_dir("curriculum").unwrap() {
        let entry = entry.unwrap();
        if entry.path().extension().map(|e| e == "json").unwrap_or(false) {
            let c = Curriculum::load(&entry.path()).unwrap_or_else(|e| {
                panic!("Failed to deserialize {}: {}", entry.path().display(), e)
            });
            assert!(c.task_count() > 0);
            assert!(c.name.len() > 2);
        }
    }
}

// ── 5. Doc Provider Integration ──

#[test]
fn test_doc_provider_loads_all_md_files() {
    let dp = DocProvider::load(Path::new("docs"));
    let ctx = dp.to_context_string();
    assert!(ctx.contains("read_file"), "Should include content from file-ops.md");
    assert!(ctx.contains("delete_file"), "Should include content from directory-ops.md");
    assert!(ctx.contains("Memory"), "Should include content from self-improvement.md");
    assert!(ctx.contains("REFERENCE DOCUMENTATION"), "Should have header");
}

#[test]
fn test_doc_provider_specific_doc() {
    let dp = DocProvider::load(Path::new("docs"));
    let file_ops = dp.get("file-ops").expect("file-ops.md should be loadable");
    assert!(file_ops.contains("read_file"));
    assert!(file_ops.contains("write_file"));
    assert!(file_ops.contains("delete_file"));

    let si = dp.get("self-improvement").expect("self-improvement.md should be loadable");
    assert!(si.contains("Error"));
    assert!(si.contains("Memory"));
}

// ── 6. Action Enum Integration ──

#[test]
fn test_action_descriptions_all_non_empty() {
    use vo1d::models::action::Action;

    let actions = vec![
        Action::ReadFile { path: "a".into(), start_line: None, end_line: None },
        Action::WriteFile { path: "a".into(), content: "c".into(), append: None },
        Action::ExecuteCommand { command: "c".into(), timeout: None, workdir: None },
        Action::ListDirectory { path: ".".into(), pattern: None },
        Action::SearchFiles { pattern: "*".into(), path: None, search_type: None },
        Action::DeleteFile { path: "a".into(), pattern: None },
        Action::CopyFile { source: "a".into(), destination: "b".into() },
        Action::CreateDirectory { path: "d".into() },
        Action::FileMetadata { path: "a".into() },
        Action::HttpRequest { url: "https://example.com".into(), method: None, headers: None, body: None },
        Action::Finish { output: None },
        Action::AskUser { question: "?".into() },
        Action::WebSearch { query: "rust".into(), num_results: None },
        Action::WebFetch { url: "https://example.com".into(), max_chars: None },
    ];

    for action in &actions {
        let desc = action.description();
        assert!(!desc.is_empty(), "Action {:?} should have non-empty description", action);
    }
}

#[test]
fn test_is_destructive_web_tools() {
    use vo1d::models::action::Action;

    assert!(!Action::WebSearch { query: "".into(), num_results: None }.is_destructive());
    assert!(!Action::WebFetch { url: "".into(), max_chars: None }.is_destructive());
    assert!(Action::DeleteFile { path: "".into(), pattern: None }.is_destructive());
    assert!(!Action::ReadFile { path: "".into(), start_line: None, end_line: None }.is_destructive());
}

// ── 7. Action Type Name Integration ──

#[test]
fn test_action_type_names_match_registry() {
    use vo1d::models::action::Action;

    let pairs: Vec<(Action, &str)> = vec![
        (Action::ReadFile { path: "".into(), start_line: None, end_line: None }, "read_file"),
        (Action::WriteFile { path: "".into(), content: "".into(), append: None }, "write_file"),
        (Action::ExecuteCommand { command: "".into(), timeout: None, workdir: None }, "execute_command"),
        (Action::ListDirectory { path: "".into(), pattern: None }, "list_directory"),
        (Action::SearchFiles { pattern: "".into(), path: None, search_type: None }, "search_files"),
        (Action::DeleteFile { path: "".into(), pattern: None }, "delete_file"),
        (Action::CopyFile { source: "".into(), destination: "".into() }, "copy_file"),
        (Action::CreateDirectory { path: "".into() }, "create_directory"),
        (Action::FileMetadata { path: "".into() }, "file_metadata"),
        (Action::HttpRequest { url: "".into(), method: None, headers: None, body: None }, "http_request"),
        (Action::Finish { output: None }, "finish"),
        (Action::AskUser { question: "".into() }, "ask_user"),
        (Action::WebSearch { query: "".into(), num_results: None }, "web_search"),
        (Action::WebFetch { url: "".into(), max_chars: None }, "web_fetch"),
    ];

    for (action, expected_name) in &pairs {
        // Use serde serialization to confirm the serde rename tag
        let json = serde_json::to_value(action).unwrap();
        let action_name = json.get("action").and_then(|v| v.as_str()).unwrap_or("");
        assert_eq!(action_name, *expected_name, "Action {:?} should serialize as '{}'", action, expected_name);
    }
}

// ── 8. Parsing Integration ──

#[test]
fn test_parser_handles_web_tools() {
    use vo1d::agent::parser::ToolParser;

    let parser = ToolParser::new();

    let web_search_input = r#"Let me search the web.
```json
{"action": "web_search", "query": "rust programming", "num_results": 5}
```"#;
    let result = parser.parse(web_search_input, false).unwrap();
    assert!(matches!(result, Action::WebSearch { query, .. } if query == "rust programming"));

    let web_fetch_input = r#"I'll fetch that page.
```json
{"action": "web_fetch", "url": "https://example.com", "max_chars": 1000}
```"#;
    let result = parser.parse(web_fetch_input, false).unwrap();
    assert!(matches!(result, Action::WebFetch { url, .. } if url == "https://example.com"));
}

#[test]
fn test_parser_heuristic_web_search() {
    use vo1d::agent::parser::ToolParser;

    let parser = ToolParser::new();
    let input = "web search: rust programming language";
    let result = parser.parse(input, false).unwrap();
    assert!(matches!(result, Action::WebSearch { query, .. } if query == "rust programming language"));
}

#[test]
fn test_parser_heuristic_web_fetch() {
    use vo1d::agent::parser::ToolParser;

    let parser = ToolParser::new();
    let input = "fetch: https://example.com";
    let result = parser.parse(input, false).unwrap();
    assert!(matches!(result, Action::WebFetch { url, .. } if url == "https://example.com"));
}
