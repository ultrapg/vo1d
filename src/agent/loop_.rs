use crate::agent::executor::ToolExecutor;
use crate::agent::parser::ToolParser;
use crate::agent::planner::Planner;
use crate::agent::session::Session;
use crate::AppContext;
use crate::models::action::Action;
use crate::models::message::Message;
use crate::security::policy::PolicyResult;
use crate::tools::registry::ToolRegistry;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// The main ReAct agent loop.
pub async fn agent_loop(ctx: AppContext, mut session: Session) -> Result<Session> {
    let cancel = CancellationToken::new();
    let tool_registry = Arc::new(ToolRegistry::new(&ctx));
    let tool_parser = ToolParser::new();
    let _planner = Planner::new(ctx.config.max_iterations);

    let model_id = ctx.config.default_model.clone();
    let backend = ctx.model_registry.get(&model_id)
        .ok_or_else(|| anyhow::anyhow!("Default model '{}' not found", model_id))?;

    let model_path = ctx.model_registry.model_path(backend);
    let llm_config = ctx.config.llm.builtin.clone();
    let llm = crate::llm::builtin::create_backend(&llm_config, &model_path).await
        .context("Failed to create LLM backend")?;

    let supports_native_tools = llm.supports_tools();
    let system_prompt = build_system_prompt(&ctx, supports_native_tools);

    // Initialize conversation
    let mut conversation = vec![Message::system(&system_prompt)];

    let task_msg = Message::user(&session.base_task);
    conversation.push(task_msg);

    let max_iters = ctx.config.max_iterations as usize;
    for iteration in 0..max_iters {
        if cancel.is_cancelled() {
            session.status = crate::agent::session::SessionStatus::Cancelled;
            break;
        }

        // Save checkpoint
        if iteration % 5 == 0 {
            crate::agent::checkpoint::save_checkpoint(&ctx, &session, iteration)?;
        }

        if !session.tui_mode {
            eprintln!("─── Iteration {}/{} ───", iteration + 1, max_iters);
        }

        // ANALYZE + PLAN: Get model response via streaming
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
        if !session.tui_mode {
            println!();
        }

        // Add assistant response to conversation
        conversation.push(Message::assistant(&response_text));

        // PARSE: Extract action from model output
        let action = tool_parser.parse(&response_text, supports_native_tools)?;
        if !session.tui_mode {
            eprintln!("→ {}", action.description());
        }

        match &action {
            Action::Finish { output } => {
                if !session.tui_mode {
                    eprintln!("✓ Task completed: {}", output.as_deref().unwrap_or("done"));
                }
                tracing::info!("Task completed: {:?}", output);
                session.status = crate::agent::session::SessionStatus::Completed;
                session.final_output = output.clone();
                // Log audit
                ctx.audit.log_model("finish", output.as_deref().unwrap_or("done"),
                    ctx.security.current_mode)?;
                break;
            }
            _ => {
                // SECURITY: Evaluate action
                let policy_result = ctx.security.policy.evaluate(
                    &action,
                    ctx.security.current_mode,
                    &ctx.paths,
                );

                let approved = match policy_result {
                    PolicyResult::Allow => true,
                    PolicyResult::Ask => {
                        ctx.security.approval.ask(&action, "Approve this action?")?
                    }
                    PolicyResult::Block => {
                        if !session.tui_mode {
                            eprintln!("  ⛔ Blocked by policy");
                        }
                        tracing::warn!("Action blocked by policy: {}", action.description());
                        ctx.audit.log_security("blocked",
                            &action.description(),
                            ctx.security.current_mode,
                            false)?;
                        conversation.push(Message::tool(
                            format!("Action BLOCKED by security policy: {}. Task cannot proceed with this action.", action.description()),
                            format!("block_{}", iteration),
                        ));
                        continue;
                    }
                };

                if !approved {
                    if !session.tui_mode {
                        eprintln!("  ✋ Rejected by user");
                    }
                    tracing::info!("Action rejected by user: {}", action.description());
                    conversation.push(Message::tool(
                        "Action rejected by user. Propose a different approach.".to_string(),
                        format!("reject_{}", iteration),
                    ));
                    continue;
                }

                // EXECUTE: Run the action
                ctx.audit.log_model(&format!("execute_{}", action_type_name(&action)),
                    &action.description(), ctx.security.current_mode)?;

                let result = ToolExecutor::execute(&action, &ctx, &tool_registry).await;

                match result {
                    Ok(output) => {
                        if !session.tui_mode {
                            eprintln!("  ✓ {} chars, {} lines", output.len(), output.lines().count());
                        }
                        let truncated = if output.len() > 2000 {
                            format!("{}... [truncated {} chars]", &output[..2000], output.len() - 2000)
                        } else {
                            output.clone()
                        };

                        ctx.audit.log_executor(&action_type_name(&action),
                            &output.chars().take(200).collect::<String>(),
                            ctx.security.current_mode, 0)?;

                        conversation.push(Message::tool(truncated, format!("tool_{}", iteration)));
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
                            format!("Error: {}", e),
                            format!("error_{}", iteration),
                        ));
                    }
                }

                // Update session plan
                session.execution_step_index = iteration as u32;
                session.variables.insert("last_action".to_string(), action.description());
            }
        }

        // Save session state
        if let Err(e) = crate::agent::session::save_session_metadata(&ctx, &session) {
            tracing::warn!("Failed to save session: {}", e);
        }
    }

    if session.status != crate::agent::session::SessionStatus::Completed {
        if !cancel.is_cancelled() {
            session.status = crate::agent::session::SessionStatus::Failed;
            session.final_output = Some("Maximum iterations reached without task completion.".to_string());
        }
    }

    Ok(session)
}

fn build_system_prompt(ctx: &AppContext, supports_native_tools: bool) -> String {
    let mode = ctx.security.current_mode.as_str();
    let workspace = ctx.paths.workspace_dir();
    let ws = workspace.display();

    if supports_native_tools {
        format!(
            r#"You are VO1D, an advanced system agent running in {mode} mode.
Your workspace is: {ws}

You have access to tools for:
- Reading and writing files (within workspace in Interactive/Safe modes)
- Executing shell commands
- Listing directories and searching files
- Making HTTP requests
- File metadata operations

Always complete the task you are given. When done, call the finish action with a summary."#,
            mode = mode,
            ws = ws,
        )
    } else {
        format!(
            r#"You are VO1D, an advanced system agent running in {mode} mode.
Your workspace is: {ws}

You do not talk to the user in prose.
You output exactly one JSON-compliant block enclosed within ```json and ```.
You MUST use one of the following exact structures:

1. READ FILE:
```json
{{ "action": "read_file", "path": "relative/path/to/file.txt" }}
```

2. WRITE FILE:
```json
{{ "action": "write_file", "path": "relative/path/to/output.py", "content": "print('hello')" }}
```

3. EXECUTE COMMAND:
```json
{{ "action": "execute_command", "command": "dir", "timeout": 30 }}
```

4. LIST DIRECTORY:
```json
{{ "action": "list_directory", "path": "." }}
```

5. SEARCH FILES:
```json
{{ "action": "search_files", "pattern": "**/*.rs" }}
```

6. TASK COMPLETED:
```json
{{ "action": "finish", "output": "Summary of completed work." }}
```

Any conversational text outside these blocks will be ignored. Think only in structured Actions.
Current mode: {mode}
Workspace: {ws}"#,
            mode = mode,
            ws = ws,
        )
    }
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
    }
}
