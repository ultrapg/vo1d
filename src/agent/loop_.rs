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

        // Inject progress context so the model can track multi-step plans
        if iteration > 0 {
            let prev_action = session.variables.get("last_action")
                .map(|s| s.as_str())
                .unwrap_or("started execution");
            let progress_msg = format!(
                "[Progress: step {} of {}. Last action: {}. Continue the task or call finish when done.]",
                iteration + 1,
                max_iters,
                prev_action,
            );
            conversation.push(Message::system(progress_msg));
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

When given a task, break it into steps. Think about what to do, then take one action at a time.

First, plan the steps you need. Then explain your reasoning before each action.

Example:
The user wants me to find all TODO comments in the project.
Step 1: search for TODO pattern
Step 2: read the matching files
Step 3: report results

I'll start by searching for TODOs.

```json
{{"action": "search_files", "pattern": "TODO"}}
```

Always call finish when the task is complete with a summary of what was done."#,
            mode = mode,
            ws = ws,
        )
    } else {
        format!(
            r#"You are VO1D, an advanced system agent running in {mode} mode.
Your workspace is: {ws}

You complete tasks by reasoning step by step and taking one action at a time.

HOW TO RESPOND:
1. First, THINK in natural language — explain the situation, what you've done, what you plan to do next.
2. Then, output EXACTLY ONE JSON action inside ```json and ``` blocks.

EXAMPLE:
The user wants me to back up config.toml. I'll first check if it exists, then copy it.

```json
{{"action": "file_metadata", "path": "config.toml"}}
"""

ON COMPLEX TASKS:
- Break the task into clear steps before starting
- After each step, evaluate the result before deciding the next action
- If a step fails, adapt your approach

AVAILABLE ACTIONS (use these exact names):

--- FILE OPERATIONS ---
- read_file:   {{"action": "read_file", "path": "..."}}
- write_file:  {{"action": "write_file", "path": "...", "content": "..."}}
- delete_file: {{"action": "delete_file", "path": "..."}} or {{"action": "delete_file", "path": ".", "pattern": "*.txt"}}
- copy_file:   {{"action": "copy_file", "source": "...", "destination": "..."}}
- list_directory: {{"action": "list_directory", "path": "."}}
- search_files: {{"action": "search_files", "pattern": "*.rs"}}
- create_directory: {{"action": "create_directory", "path": "new/folder"}}
- file_metadata: {{"action": "file_metadata", "path": "file.txt"}}

--- COMMAND EXECUTION ---
- execute_command: {{"action": "execute_command", "command": "dir", "timeout": 60}}

--- WEB ---
- http_request: {{"action": "http_request", "url": "https://...", "method": "GET"}}

--- INTERACTION ---
- ask_user: {{"action": "ask_user", "question": "What to do?"}}

--- COMPLETION ---
- finish: {{"action": "finish", "output": "Summary of what was done."}}

RULES:
- Exactly ONE JSON action per response (after your reasoning text).
- All file paths are relative to the workspace: {ws}
- On Windows, use cmd.exe syntax for shell commands (dir, del, copy, move, echo).
- Prefer the built-in file actions over shell commands for file operations.
- Call finish with a summary when the task is complete.
- Do NOT call finish until the task is fully done.

Current mode: {mode}"#,
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
