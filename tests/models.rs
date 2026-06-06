use vo1d::models::message::{LlmResponse, Message, TokenUsage, ToolCall};

#[test]
fn test_message_system() {
    let msg = Message::system("system prompt");
    assert_eq!(msg.role, "system");
    assert_eq!(msg.content, "system prompt");
    assert!(msg.tool_calls.is_none());
    assert!(msg.tool_call_id.is_none());
}

#[test]
fn test_message_user() {
    let msg = Message::user("hello");
    assert_eq!(msg.role, "user");
    assert_eq!(msg.content, "hello");
}

#[test]
fn test_message_assistant() {
    let msg = Message::assistant("response");
    assert_eq!(msg.role, "assistant");
    assert_eq!(msg.content, "response");
}

#[test]
fn test_message_tool() {
    let msg = Message::tool("result", "call_123");
    assert_eq!(msg.role, "tool");
    assert_eq!(msg.content, "result");
    assert_eq!(msg.tool_call_id, Some("call_123".to_string()));
}

#[test]
fn test_tool_call_new() {
    let tc = ToolCall::new("read_file", r#"{"path": "test.txt"}"#);
    assert_eq!(tc.function.name, "read_file");
    assert!(tc.id.starts_with("call_"));
    assert_eq!(tc.call_type, "function");
}

#[test]
fn test_llm_response_with_usage() {
    let usage = TokenUsage { prompt_tokens: 10, completion_tokens: 20, total_tokens: 30 };
    let response = LlmResponse {
        content: "Hello".to_string(),
        tool_calls: None,
        usage: Some(usage),
    };
    assert_eq!(response.content, "Hello");
    assert_eq!(response.usage.as_ref().unwrap().total_tokens, 30);
}

#[test]
fn test_llm_response_without_usage() {
    let response = LlmResponse {
        content: "No usage".to_string(),
        tool_calls: None,
        usage: None,
    };
    assert!(response.usage.is_none());
}
