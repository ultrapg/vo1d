use crate::agent::executor::ToolExecutor;
use crate::agent::parser::ToolParser;
use crate::agent::planner::Planner;
use crate::agent::session::Session;
use crate::core::self_correction::{ErrorClassifier, FailureTracker};
use crate::AppContext;
use crate::models::action::Action;
use crate::models::message::Message;
use crate::security::policy::PolicyResult;
use crate::tools::registry::ToolRegistry;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use regex::Regex;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// The main ReAct agent loop.
pub async fn agent_loop(ctx: AppContext, mut session: Session) -> Result<Session> {
    let cancel = CancellationToken::new();
    let tool_registry = Arc::new(ToolRegistry::new(&ctx));
    let tool_parser = ToolParser::new();
    let _planner = Planner::new(ctx.config.max_iterations);
    let mut failure_tracker = FailureTracker::new();

    let model_id = ctx.config.default_model.clone();
    let backend = ctx.model_registry.get(&model_id)
        .ok_or_else(|| anyhow::anyhow!("Default model '{}' not found", model_id))?;

    let model_path = ctx.model_registry.model_path(backend);
    let llm_config = ctx.config.llm.builtin.clone();
    let llm = crate::llm::builtin::create_backend(&llm_config, &model_path).await
        .context("Failed to create LLM backend")?;

    let supports_native_tools = llm.supports_tools();
    let context_limit = (llm_config.context_size as f64 * 4.0 * 0.8) as usize;
    let system_prompt = build_system_prompt(&ctx, supports_native_tools);
    let compressor = crate::core::compression::ContextCompressor::new(
        crate::core::compression::CompressionConfig::default()
    );

    let mut conversation = vec![Message::system(&system_prompt)];
    let task_msg = Message::user(&session.base_task);
    conversation.push(task_msg);

    let max_iters = ctx.config.max_iterations as usize;
    let mut action_history: Vec<String> = Vec::new();

    for iteration in 0..max_iters {
        if cancel.is_cancelled() {
            session.status = crate::agent::session::SessionStatus::Cancelled;
            break;
        }

        // Context compression at ~80% usage
        let conv_chars: usize = conversation.iter().map(|m| m.content.len() + 100).sum();
        if conv_chars > context_limit {
            let before = conversation.len();
            let (compressed, summary) = compressor.compress(&conversation, llm_config.context_size as usize);
            conversation = compressed;
            if !summary.is_empty() {
                if let Ok(mut mem) = ctx.memory.lock() {
                    mem.add_preference("_compressed_section", &summary.chars().take(500).collect::<String>());
                }
            }
            if !session.tui_mode {
                eprintln!("── [Context compressed: {} msgs → {} msgs] ──",
                    before, conversation.len());
            }
        }

        if iteration % 5 == 0 {
            crate::agent::checkpoint::save_checkpoint(&ctx, &session, iteration)?;
        }

        if !session.tui_mode {
            eprintln!("─── Iteration {}/{} ───", iteration + 1, max_iters);
        }

        if iteration > 0 {
            let prev_action = session.variables.get("last_action")
                .map(|s| s.as_str())
                .unwrap_or("started execution");
            let prev_result = session.variables.get("last_result")
                .filter(|s| !s.is_empty())
                .map(|s| format!(". Result: {}", s.chars().take(500).collect::<String>()))
                .unwrap_or_default();
            let progress_msg = format!(
                "[Progress: step {}/{}. Last action: {}{}. Original task: {}. Think about what to do next, then take ONE action.]",
                iteration + 1,
                max_iters,
                prev_action.trim_end_matches('.'),
                prev_result,
                session.base_task,
            );
            conversation.push(Message::system(progress_msg));
        }

        let mut response_text = String::new();
        let mut stream = llm.stream_chat(&conversation).await
            .map_err(|e| anyhow::anyhow!("LLM chat failed: {}", e))?;

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(token) => {
                    if !session.tui_mode {
                        print!("{}", token);
                        std::io::Write::flush(&mut std::io::stdout())?;
                    }
                    response_text.push_str(&token);
                }
                Err(e) => {
                    anyhow::bail!("Generation error: {}", e);
                }
            }
        }

        // Parse action from response
        let action = match tool_parser.parse(&response_text, supports_native_tools) {
            Ok(a) => a,
            Err(_e) => {
                // Fallback to Finish if no action found
                tracing::warn!("No structured action found; treating as Finish output: {}",
                    response_text.chars().take(100).collect::<String>());
                Action::Finish { output: Some(response_text.clone()) }
            }
        };

        // Check for conversational response (no tool intent)
        if is_conversational(&response_text) {
            if iteration == 0 {
                // First iteration: treat as completion
                session.status = crate::agent::session::SessionStatus::Completed;
                session.final_output = Some(response_text.clone());
                if let Ok(mut mem) = ctx.memory.lock() {
                    mem.add_task(&session.base_task, vec![], &session.final_output.clone().unwrap_or_default());
                }
                break;
            } else {
                // Mid-task: nudge the model
                conversation.push(Message::system(
                    "You responded without a tool action. If the task is not done, output a tool action now. If it is done, call finish."
                ));
                continue;
            }
        }

        // Extract reasoning for display
        let reasoning = extract_reasoning(&response_text);
        let clean_reasoning = clean_reasoning(&reasoning);
        if !clean_reasoning.is_empty() {
            if !session.tui_mode {
                println!("\n[REASONING]\n{}\n", clean_reasoning);
            }
        } else if iteration > 0 {
            // Reasoning reminder after first iteration
            conversation.push(Message::system(
                "REMINDER: You MUST explain your reasoning in natural language before each JSON action. Start with 'I need to...' or 'The previous result shows...' and explain what you'll do next."
            ));
        }

        // Check for multiple actions in one response
        let json_block_count = response_text.matches("```json").count();
        if json_block_count > 1 || has_multiple_json_objects(&response_text) {
            let warn = "WARNING: Multiple actions detected in one response. Only the first was used. Output exactly ONE action at a time.";
            conversation.push(Message::system(warn));
        }

        // Loop detection
        let action_type = action_type_name(&action);
        action_history.push(action_type.to_string());
        if action_history.len() > 50 {
            action_history.remove(0);
        }
        // Store action history on session for memory recall
        session.variables.insert("action_history".to_string(), action_history.join(","));
        let repeat_count = action_history.iter()
            .rev()
            .take_while(|&a| a == action_type)
            .count();
        if repeat_count >= 3 && !matches!(&action, Action::Finish { .. }) {
            let loop_warning = format!(
                "WARNING: You performed '{}' {} times in a row. This is a loop. STOP this pattern and do something different. Try something different or call finish if the task is done.",
                action_type, repeat_count,
            );
            conversation.push(Message::tool(loop_warning, format!("loop_{}", iteration)));
        }

        // Handle Finish
        if let Action::Finish { output } = &action {
            session.status = crate::agent::session::SessionStatus::Completed;
            session.final_output = output.clone();
            if let Ok(mut mem) = ctx.memory.lock() {
                mem.add_task(&session.base_task, action_history.clone(), &session.final_output.clone().unwrap_or_default());
            }
            break;
        }

        // Security policy evaluation
        let policy_result = ctx.security.policy.evaluate(&action, ctx.security.current_mode, &ctx.paths);
        match policy_result {
            PolicyResult::Allow => {},
            PolicyResult::Ask => {
                if !session.tui_mode {
                    println!("\n[APPROVAL REQUIRED]\n{}", action.description());
                    println!("Approve? (y/n): ");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input)?;
                    if input.trim().to_lowercase() != "y" {
                        tracing::warn!("Action rejected by user: {}", action.description());
                        conversation.push(Message::tool(
                            "Action rejected by user. Propose a different approach.",
                            format!("rejected_{}", iteration)
                        ));
                        session.variables.insert("last_action".to_string(), action_type.to_string());
                        session.variables.insert("last_result".to_string(), "Rejected by user".to_string());
                        continue;
                    }
                }
            }
            PolicyResult::Block => {
                tracing::warn!("Action blocked by policy: {}", action.description());
                conversation.push(Message::tool(
                    format!("Action BLOCKED by security policy: {}. Task cannot proceed with this action.", action.description()),
                    format!("blocked_{}", iteration)
                ));
                session.variables.insert("last_action".to_string(), action_type.to_string());
                session.variables.insert("last_result".to_string(), "Blocked by policy".to_string());
                continue;
            }
        }

        // Execute the action
        match ToolExecutor::execute(&action, &ctx, &tool_registry).await {
            Ok(output) => {
                if !session.tui_mode {
                    eprintln!("  ✓ Success");
                }
                tracing::info!("Action succeeded: {}", action.description());

                // Truncate large outputs
                let truncated = if output.len() > 2000 {
                    format!("{}... [truncated {} chars]", &output[..2000], output.len() - 2000)
                } else {
                    output.clone()
                };

                conversation.push(Message::tool(truncated, format!("result_{}", iteration)));
                session.variables.insert("last_action".to_string(), action_type.to_string());
                session.variables.insert("last_result".to_string(), output.clone());

                // Clear failure tracking on success
                failure_tracker.clear(action_type);
            }
            Err(e) => {
                if !session.tui_mode {
                    eprintln!("  ✗ Failed: {}", e);
                }
                tracing::error!("Action failed: {}", e);
                ctx.audit.log_security("error",
                    &format!("{}: {}", action.description(), e),
                    ctx.security.current_mode, true)?;

                conversation.push(Message::tool(
                    ErrorClassifier::format_with_suggestion(&e, &action),
                    format!("error_{}", iteration),
                ));

                session.variables.insert("last_action".to_string(), action_type.to_string());
                session.variables.insert("last_result".to_string(), format!("Error: {}", e));

                // Track failures for self-correction
                let action_type_str = action_type_name(&action).to_string();
                failure_tracker.record(&action_type_str);
                
                // Learn from mistakes: store in persistent memory
                if failure_tracker.should_suggest_correction(&action_type_str, 2) {
                    if let Ok(mut mem) = ctx.memory.lock() {
                        let error_msg = format!("{}: {}", action.description(), e);
                        let lesson = failure_tracker.correction_prompt(&action_type_str);
                        mem.add_mistake(
                            &session.base_task,
                            &error_msg,
                            &lesson,
                            &format!("Avoid repeating '{}'. Verify file paths, command syntax, or try a different approach.", action_type_str),
                            &action_history,
                        );
                    }
                }

                // Self-correction prompt after repeated failures
                if failure_tracker.should_suggest_correction(&action_type_str, 3) {
                    let correction = failure_tracker.correction_prompt(&action_type_str);
                    conversation.push(Message::system(correction));
                }
            }
        }

        // Save session state
        crate::agent::session::save_session_metadata(&ctx, &session)?;
    }

    // Check if we exited without completing
    if session.status != crate::agent::session::SessionStatus::Completed {
        if !cancel.is_cancelled() {
            session.status = crate::agent::session::SessionStatus::Failed;
            session.final_output = Some("Maximum iterations reached without task completion.".to_string());
        }
    }

    Ok(session)
}

fn clean_reasoning(text: &str) -> String {
    let mut s = text.to_string();
    for label in &["STEP 1 — REASON", "STEP 2 — ACTION", "STEP 1 —", "STEP 2 —"] {
        s = s.replace(label, "");
    }
    s.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn action_type_name(action: &Action) -> &'static str {
    match action {
        Action::ReadFile { .. } => "read_file",
        Action::WriteFile { .. } => "write_file",
        Action::ExecuteCommand { .. } => "execute_command",
        Action::ListDirectory { .. } => "list_directory",
        Action::SearchFiles { .. } => "search_files",
        Action::DeleteFile { .. } => "delete_file",
        Action::CopyFile { .. } => "copy_file",
        Action::CreateDirectory { .. } => "create_directory",
        Action::FileMetadata { .. } => "file_metadata",
        Action::HttpRequest { .. } => "http_request",
        Action::Finish { .. } => "finish",
        Action::AskUser { .. } => "ask_user",
        Action::WebSearch { .. } => "web_search",
        Action::WebFetch { .. } => "web_fetch",
    }
}

fn build_system_prompt(ctx: &AppContext, supports_native_tools: bool) -> String {
    let mode = ctx.security.current_mode.as_str();
    let workspace = ctx.paths.workspace_dir();
    let ws = workspace.display();
    let os_hint = if cfg!(windows) { "Windows" } else { "Linux/Mac" };
    let shell_hint = if cfg!(windows) { "cmd.exe" } else { "bash" };

    let memory_context = match ctx.memory.lock() {
        Ok(m) => m.to_context_string(),
        Err(_) => String::new(),
    };

    let doc_provider = crate::core::docs::DocProvider::load(&ctx.paths.root_dir().join("docs"));
    let doc_context = doc_provider.to_context_string();

    let tool_docs = format!(
        r#"AVAILABLE ACTIONS:

--- FILE OPERATIONS ---
- read_file:      Read a file's contents
  JSON: {{"action": "read_file", "path": "file.txt"}}
- write_file:     Write content to a file (creates if not exists)
  JSON: {{"action": "write_file", "path": "file.txt", "content": "..."}}
- delete_file:    Delete a file or files matching a glob pattern
  JSON: {{"action": "delete_file", "path": "file.txt"}}
  JSON: {{"action": "delete_file", "path": ".", "pattern": "*.txt"}}
  NOTE: To match ALL files use "pattern": "*" not "*.*" (which misses files without a dot)
- copy_file:      Copy source to destination
  JSON: {{"action": "copy_file", "source": "a.txt", "destination": "b.txt"}}
- create_directory: Create directory (and any missing parents)
  JSON: {{"action": "create_directory", "path": "new/folder"}}
- list_directory: List files in a directory
  JSON: {{"action": "list_directory", "path": "."}}
- search_files:   Find files matching a glob pattern
  JSON: {{"action": "search_files", "pattern": "*.rs"}}
- file_metadata:  Get metadata of a file or directory
  JSON: {{"action": "file_metadata", "path": "file.txt"}}

--- COMMAND EXECUTION ---
- execute_command: Run a shell command (timeout in seconds)
  JSON: {{"action": "execute_command", "command": "dir", "timeout": 60}}
  Common {os} commands:
{cmds}
  Prefer built-in file actions over shell commands when possible.

--- WEB ---
- web_search:    Search the web using DuckDuckGo (no API key)
  JSON: {{"action": "web_search", "query": "rust programming", "num_results": 5}}
- web_fetch:     Fetch a URL and convert HTML to markdown
  JSON: {{"action": "web_fetch", "url": "https://example.com", "max_chars": 5000}}
- http_request:  Make an HTTP request
  JSON: {{"action": "http_request", "url": "https://...", "method": "GET"}}

--- INTERACTION ---
- ask_user:  Ask the user a question
  JSON: {{"action": "ask_user", "question": "What format?"}}

--- COMPLETION ---
- finish:  Task is fully done — provide a summary
  JSON: {{"action": "finish", "output": "What was accomplished"}}"#,
        os = os_hint,
        cmds = os_command_docs(),
    );

    let conversational_note = r#"If the user is just chatting, asking a question, or having a conversation — respond naturally without tools.
Only use actions when the user asks you to perform a task (read/write files, run commands, search, etc.)."#;

    let planning_note = r#"
PLANNING WORKFLOW (for complex tasks):
1. First create a PLAN.md file describing the steps you will take
2. Execute each step one at a time
3. Update PLAN.md with progress as you go (mark steps [x] when done)
4. Check your work between steps
5. Call finish when all steps are complete"#;

    let tool_instructions = r#"When using tools: reason first in natural language, then output exactly ONE JSON action inside ```json``` tags.

--- CORRECT EXAMPLE (single action after reasoning) ---
I need to create readme.md with a story. The file doesn't exist yet so I'll write it.

```json
{"action": "write_file", "path": "readme.md", "content": "My story content."}
```

--- CORRECT EXAMPLE (delete file safely) ---
I should check what files exist before deleting. Let me list the directory first.

```json
{"action": "list_directory", "path": "."}
```

--- AFTER seeing the listing, then delete ---
I can see file.txt in the listing. No important files. I'll delete it now.

```json
{"action": "delete_file", "path": "file.txt"}
```

--- To delete all files, list first, then delete matching ones. ---
The workspace has only test files. I'll delete all of them with a pattern.

```json
{"action": "delete_file", "path": ".", "pattern": "*"}
```

IMPORTANT: Use `*` (not `*.*`) to match ALL files. `*.*` only matches files containing a dot.

--- CORRECT EXAMPLE (finish after result) ---
The file was created. The task is done.

```json
{"action": "finish", "output": "Created readme.md with a story."}
```

RULES:
- Exactly ONE JSON action per response. Multiple actions will be rejected — only the first is used.
- Do not repeat the same action 3+ times — that is a loop. Try something different or call finish.
- Always put reasoning BEFORE the JSON, never after it.
- Use "pattern": "*" (not "*.*`) to match ALL files. "*.*" misses files without a dot."#;

    if supports_native_tools {
        format!(
            r#"You are VO1D, an advanced system agent running in {mode} mode.
Your workspace is: {ws}

{conversational_note}

{planning_note}

{tool_docs}

{tool_instructions}

RULES:
- Always reason before each action
- All paths are relative to: {ws}
- OS: {os} — use {shell} syntax for shell commands
- Current mode: {mode}
- Use "pattern": "*" (not "*.*") to match ALL files{doc_context}{memory}"#,
            mode = mode, ws = ws,
            conversational_note = conversational_note,
            planning_note = planning_note,
            tool_docs = tool_docs, tool_instructions = tool_instructions,
            os = os_hint, shell = shell_hint, doc_context = doc_context, memory = memory_context,
        )
    } else {
        let tool_instructions_sim = r#"When using tools: reason first in natural language, then output exactly ONE JSON action inside ```json``` tags.

--- CORRECT EXAMPLE (write file) ---
The workspace is empty. I need to create readme.md with a story. I'll write it now.

```json
{"action": "write_file", "path": "readme.md", "content": "My story content here."}
```

--- CORRECT EXAMPLE (safe delete) ---
I need to delete files. Let me list the directory first to see what's there.

```json
{"action": "list_directory", "path": "."}
```

RULES:
- Always list the directory BEFORE deleting files — know what you're removing.
- Exactly ONE JSON action per response. Multiple actions will be rejected.
- Do not repeat the same action 3+ times — that is a loop.
- Always put reasoning BEFORE the JSON, never after it.
- Use "pattern": "*" (not "*.*") to match ALL files. "*.*" misses files without a dot."#;

        format!(
            r#"You are VO1D, an advanced system agent running in {mode} mode.
Your workspace is: {ws}

{conversational_note}

{planning_note}

{tool_docs}

{tool_instructions_sim}

RULES:
- All paths are relative to: {ws}
- OS: {os} — use {shell} syntax for shell commands
- Current mode: {mode}
- Use "pattern": "*" (not "*.*") to match ALL files{doc_context}{memory}"#,
            mode = mode, ws = ws,
            conversational_note = conversational_note,
            planning_note = planning_note,
            tool_docs = tool_docs, tool_instructions_sim = tool_instructions_sim,
            os = os_hint, shell = shell_hint, doc_context = doc_context, memory = memory_context,
        )
    }
}

fn os_command_docs() -> String {
    if cfg!(windows) {
        r#"  - dir         List directory contents
  - del         Delete file(s) — use /s for subdirectories, /q for quiet
  - copy        Copy file(s)
  - move        Move/rename file(s)
  - mkdir / rmdir   Create / remove directory
  - echo        Print text or redirect to file (echo text > file, >> appends)
  - type        Display file contents
  - findstr     Search for text in files (like grep)
  - cd          Change directory"#.to_string()
    } else {
        r#"  - ls          List directory contents
  - rm          Delete file(s) — use -r for directories, -f to force
  - cp          Copy file(s)
  - mv          Move/rename file(s)
  - mkdir / rmdir   Create / remove directory
  - echo        Print text or redirect to file (echo text > file, >> appends)
  - cat         Display file contents
  - grep        Search for text in files
  - cd          Change directory"#.to_string()
    }
}

fn extract_reasoning(text: &str) -> String {
    let re = Regex::new(r"```(?:json)?\s*[\s\S]*?```").unwrap();
    if let Some(m) = re.find(text) {
        let before = text[..m.start()].trim();
        before.to_string()
    } else {
        text.to_string()
    }
}

fn is_conversational(text: &str) -> bool {
    if text.contains(r#"{"action""#) {
        return false;
    }
    if text.contains("```json") {
        let lines_after: Vec<&str> = text.lines()
            .skip_while(|l| !l.contains("```json"))
            .skip(1)
            .take(10)
            .collect();
        let joined = lines_after.join(" ");
        if joined.contains(r#"{"action""#) {
            return false;
        }
    }
    true
}

fn has_multiple_json_objects(text: &str) -> bool {
    let count = text.matches(r#"{"action""#).count();
    count > 1
}