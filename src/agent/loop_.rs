use crate::agent::executor::ToolExecutor;
use crate::agent::parser::ToolParser;
use crate::agent::plan_parser::PlanParser;
use crate::agent::planner::Planner;
use crate::agent::session::Session;
use crate::core::compression::ContextCompressor;
use crate::core::self_correction::{ErrorClassifier, FailureTracker};
use crate::AppContext;
use crate::models::action::Action;
use crate::models::message::Message;
use crate::models::plan::{Plan, PlanStep, StepStatus};
use crate::security::policy::PolicyResult;
use crate::models::message::Tool;
use crate::tools::registry::ToolRegistry;
use anyhow::{Context, Result};
use futures_util::StreamExt;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// TDD phase tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TddPhase {
    Red,
    Green,
    Refactor,
}

/// The main ReAct agent loop.
pub async fn agent_loop(ctx: AppContext, mut session: Session) -> Result<Session> {
    let cancel = CancellationToken::new();
    let tool_registry = Arc::new(ToolRegistry::new(&ctx));
    let tool_parser = ToolParser::new();
    let _planner = Planner::new(ctx.config.max_iterations);
    let mut failure_tracker = FailureTracker::new();
    let mut plan: Option<Plan> = None;
    let mut current_step: usize = 0;
    let mut step_iterations: HashMap<u32, u32> = HashMap::new();
    let mut step_failures: HashMap<u32, u32> = HashMap::new();
    let mut plan_checked = false;
    let mut plan_recovery_fired = false;

    // Behavioral mode state
    let behavior = crate::core::behavior::BehaviorMode::from_str(&ctx.config.default_behavior).unwrap_or_default();
    let mut tdd_phase = TddPhase::Red;
    let mut fix_hypothesis: Option<String> = None;
    let mut tests_passed_before_write = false;

    let model_id = ctx.config.default_model.clone();
    let backend = ctx.model_registry.get(&model_id)
        .ok_or_else(|| anyhow::anyhow!("Default model '{}' not found", model_id))?;

    let model_path = ctx.model_registry.model_path(backend);
    let llm_config = ctx.config.llm.builtin.clone();
    let native_tools = backend.native_tools;
    let llm = crate::llm::builtin::create_backend(&llm_config, &model_path, native_tools).await
        .context("Failed to create LLM backend")?;

    let supports_native_tools = llm.supports_tools();
    let context_limit = (llm_config.context_size as f64 * 3.0) as usize;
    let system_prompt = build_system_prompt(&ctx, supports_native_tools);
    let compressor = ContextCompressor::new(
        crate::core::compression::CompressionConfig::default()
    );

    let mut conversation = vec![Message::system(&system_prompt)];
    let task_msg = Message::user(&session.base_task);
    conversation.push(task_msg);

    let mut action_history: Vec<String> = Vec::new();
    let max_iters: u64 = ctx.config.max_iterations as u64;
    let mut iteration: u64 = 0;

    loop {
        if cancel.is_cancelled() {
            session.status = crate::agent::session::SessionStatus::Cancelled;
            break;
        }

        if iteration > 0 && iteration % 5 == 0 {
            crate::agent::checkpoint::save_checkpoint(&ctx, &session, iteration as usize)?;
        }

        // --- Plan state ---
        let total_steps = plan.as_ref().map(|p| p.steps.len()).unwrap_or(0);
        let plan_progress = if total_steps > 0 {
            let done = plan.as_ref().map(|p| p.steps.iter().filter(|s| s.status == StepStatus::Completed).count()).unwrap_or(0);
            let bar_width = 10;
            let filled = (done * bar_width / total_steps).min(bar_width);
            let empty = bar_width - filled;
            format!(" [{}]{}/{} steps", format!("{}{}", "█".repeat(filled), "░".repeat(empty)), done, total_steps)
        } else {
            String::new()
        };

        if !session.tui_mode {
            let plan_info = plan.as_ref().map(|p| {
                let step_desc = p.steps.get(current_step).map(|s| s.description.as_str()).unwrap_or("done");
                format!(" [Step {}/{}: {}]", current_step + 1, p.steps.len(), step_desc)
            }).unwrap_or_default();

            // Show iteration count (limitless — no max displayed)
            eprintln!("─── Iteration {} {} {} ───", iteration + 1, plan_info, plan_progress);

            // Show planner TODO in terminal if plan is loaded
            if let Some(ref p) = plan {
                let done_count = p.steps.iter().filter(|s| s.status == StepStatus::Completed).count();
                let failed_count = p.steps.iter().filter(|s| s.status == StepStatus::Failed).count();
                let pending_count = p.steps.len() - done_count - failed_count;
                eprintln!("  Goal: {}", p.goal.chars().take(80).collect::<String>());
                eprintln!("  Steps: {} done | {} pending | {} failed", done_count, pending_count, failed_count);
                for (i, step) in p.steps.iter().enumerate() {
                    let marker = match step.status {
                        StepStatus::Completed => "✓",
                        StepStatus::Failed => "✗",
                        StepStatus::Running => "→",
                        StepStatus::Pending | StepStatus::Skipped => "☐",
                    };
                    let current_marker = if i == current_step { " ← CURRENT" } else { "" };
                    eprintln!("    {} {}. {}{}", marker, i + 1, step.description.chars().take(50).collect::<String>(), current_marker);
                }
            }
        }

        // --- Requires-plan enforcement (iteration 1+) ---
        if iteration >= 1 && behavior.requires_plan() && plan.is_none() && plan_checked {
            conversation.push(Message::system(
                "This mode requires a PLAN.md file. Create one before proceeding. \
                 Write a PLAN.md in the workspace root with steps like:\n\
                 ## Step 1: ...\n- [ ] ...\n**Action:** read_file\n\nThen continue."
            ));
        }

        if iteration > 0 {
            let prev_action = session.variables.get("last_action")
                .map(|s| s.as_str())
                .unwrap_or("started execution");
            let prev_result = session.variables.get("last_result")
                .filter(|s| !s.is_empty())
                .map(|s| format!(". Result: {}", s.chars().take(500).collect::<String>()))
                .unwrap_or_default();

            let plan_context = plan.as_ref().map(|p| {
                let step = p.steps.get(current_step);
                match step {
                    Some(s) => format!(" | Plan step {}/{}: {}", current_step + 1, p.steps.len(), s.description),
                    None => String::new(),
                }
            }).unwrap_or_default();

            // Per-step iteration bound
            let step_iter = step_iterations.get(&(current_step as u32)).copied().unwrap_or(0);
            let step_note = if step_iter > 0 && step_iter % 10 == 0 {
                format!("\nNOTE: You've been working on the current plan step for {} iterations. If you're stuck, consider a different approach.", step_iter)
            } else {
                String::new()
            };

            // Per-step limit enforcement (generous — context compression handles overflow)
            let per_step_limit: u32 = 100;
            let step_limit_note = if step_iter >= per_step_limit && plan.is_some() {
                format!("\nYou have exceeded the iteration limit ({}) for this plan step. The step will be marked as failed. Move to the next step or replan.", per_step_limit)
            } else {
                String::new()
            };

            let progress_msg = format!(
                "[Iteration {}{}. Last action: {}{}. Original task: {}. Think about what to do next, then take ONE action.]{}{}",
                iteration + 1,
                plan_context,
                prev_action.trim_end_matches('.'),
                prev_result,
                session.base_task,
                step_note,
                step_limit_note,
            );
            conversation.push(Message::system(progress_msg));
        }

        // Context compression right before LLM call (after all messages added)
        let conv_chars: usize = conversation.iter().map(|m| m.content.len() + 100).sum();
        let usage_ratio = if context_limit > 0 { conv_chars as f64 / context_limit as f64 } else { 0.0 };

        if usage_ratio > 0.60 && conversation.len() > 10 {
            // --- LLM summarization at high usage ---
            let summarization_range = compressor.summarization_range(&conversation);
            if let Some((start, end, insert_at)) = summarization_range {
                // Build summarization prompt from the slice
                let summarize_slice: Vec<String> = conversation[start..end].iter()
                    .map(|m| format!("[{}]: {}", m.role, m.content.chars().take(200).collect::<String>()))
                    .collect();
                let summary_prompt = format!(
                    "Summarize the following conversation between a user and an AI assistant working on a task. \
                     Focus on: what files were read/written, what commands were run, what decisions were made, \
                     and what the current status is. Be concise but comprehensive.\n\nTask: {}\n\n{}",
                    session.base_task,
                    summarize_slice.join("\n")
                );
                let summary_messages = vec![
                    crate::models::message::Message::system(
                        "You are a conversation summarizer. Provide a concise structured summary."
                    ),
                    crate::models::message::Message::user(&summary_prompt),
                ];

                let summary_result = llm.chat(&summary_messages, None).await;
                match summary_result {
                    Ok(resp) => {
                        let summary_text = resp.content.trim().to_string();
                        if !summary_text.is_empty() {
                            let before = conversation.len();
                            conversation = ContextCompressor::inject_summary(
                                conversation, &summary_text, start, end, insert_at
                            );
                            if let Ok(mut mem) = ctx.memory.lock() {
                                mem.add_preference("_compressed_section",
                                    &summary_text.chars().take(500).collect::<String>());
                            }
                            if !session.tui_mode {
                                eprintln!("── [Context summarized: {} msgs → {} msgs] ──",
                                    before, conversation.len());
                            }
                        }
                    }
                    Err(_) => {
                        // Fall back to old-style compression
                        let before = conversation.len();
                        let (compressed, summary) = compressor.compress(&conversation, llm_config.context_size as usize);
                        conversation = compressed;
                        if !summary.is_empty() {
                            if let Ok(mut mem) = ctx.memory.lock() {
                                mem.add_preference("_compressed_section",
                                    &summary.chars().take(500).collect::<String>());
                            }
                        }
                        if !session.tui_mode {
                            eprintln!("── [Context compressed: {} msgs → {} msgs] ──",
                                before, conversation.len());
                        }
                    }
                }
            }
        } else if conv_chars > context_limit {
            // Old-style compression for moderate overflow
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

        let mut response_text = String::new();
        let mut tool_calls_result: Option<Vec<crate::models::message::ToolCall>> = None;
        let mut display_printed: usize = 0;

        if supports_native_tools {
            let tools: Vec<Tool> = tool_registry.as_llm_tools();
            let result = llm.chat(&conversation, Some(&tools)).await
                .map_err(|e| anyhow::anyhow!("LLM chat failed: {}", e))?;
            response_text = result.content;
            tool_calls_result = result.tool_calls;
            if !session.tui_mode {
                let clean = format_for_display(&response_text);
                print!("{}", clean);
                std::io::Write::flush(&mut std::io::stdout())?;
            }
        } else {
            let mut stream = llm.stream_chat(&conversation).await
                .map_err(|e| anyhow::anyhow!("LLM chat failed: {}", e))?;

            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(token) => {
                        response_text.push_str(&token);

                        if !session.tui_mode {
                            let clean = format_for_display(&response_text);
                            let clean_chars: Vec<char> = clean.chars().collect();
                            let total_chars = clean_chars.len();
                            if total_chars > display_printed {
                                let new_part: String = clean_chars[display_printed..].iter().collect();
                                print!("{}", new_part);
                                std::io::Write::flush(&mut std::io::stdout())?;
                                display_printed = total_chars;
                            }
                        }
                    }
                    Err(e) => {
                        anyhow::bail!("Generation error: {}", e);
                    }
                }
            }
        }

        // Strip <think> tags before any further processing
        // (the raw response is already stripped for display above;
        //  we strip again so parsing also sees clean text)
        response_text = strip_think_tags(&response_text);

        // Parse action from response (either from tool_calls or text)
        let action = if let Some(tcalls) = tool_calls_result {
            if !tcalls.is_empty() {
                tool_call_to_action(&tcalls[0], &response_text)
            } else {
                tool_parser.parse(&response_text, supports_native_tools).unwrap_or_else(|_| {
                    Action::Finish { output: Some(response_text.clone()) }
                })
            }
        } else {
            match tool_parser.parse(&response_text, supports_native_tools) {
                Ok(a) => a,
                Err(_e) => {
                    tracing::warn!("No structured action found; treating as Finish output: {}",
                        response_text.chars().take(100).collect::<String>());
                    Action::Finish { output: Some(response_text.clone()) }
                }
            }
        };

        if !session.tui_mode {
            // No separate reasoning display — already streamed clean above
            // Just print a newline after the streamed output and show the action
            eprintln!("");
        }

        // Check for conversational response (no tool intent)
        if is_conversational(&response_text) {
            if iteration == 0 {
                session.status = crate::agent::session::SessionStatus::Completed;
                session.final_output = Some(response_text.clone());
                if let Ok(mut mem) = ctx.memory.lock() {
                    mem.add_task(&session.base_task, vec![], &session.final_output.clone().unwrap_or_default());
                }
                break;
            } else {
                conversation.push(Message::system(
                    "You responded without a tool action. If the task is not done, output a tool action now. If it is done, call finish."
                ));
                continue;
            }
        }

        // Extract hypothesis from reasoning for Fix mode
        let reasoning = extract_reasoning(&response_text);
        if strip_think_tags(&reasoning).trim().is_empty() && iteration > 0 {
            conversation.push(Message::system(
                "REMINDER: You MUST explain your reasoning in natural language before each JSON action. Start with 'I need to...' or 'The previous result shows...' and explain what you'll do next."
            ));
        }
        if behavior == crate::core::behavior::BehaviorMode::Fix && reasoning.len() > 20 {
            let hyp = reasoning.chars().take(300).collect::<String>();
            fix_hypothesis = Some(hyp);
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
        session.variables.insert("action_history".to_string(), serde_json::to_string(&action_history).unwrap_or_default());
        let repeat_count = action_history.iter()
            .rev()
            .take_while(|&a| a == action_type)
            .count();
        if repeat_count >= 5 && !matches!(&action, Action::Finish { .. }) {
            conversation.push(Message::system(
                "CRITICAL: You have repeated this action 5+ times. Restarting this iteration — \
                 clear your mind and try a completely different approach. Think step by step about what \
                 would actually make progress, and do something you haven't tried yet."
            ));
            continue; // Restart iteration without incrementing counter
        }
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

        // Behavioral mode: read-only phase enforcement
        let read_only_until = behavior.read_only_iters() as u64;
        if iteration < read_only_until && !matches!(&action, Action::Finish { .. }) {
            let is_write = matches!(&action,
                Action::WriteFile { .. } | Action::DeleteFile { .. } | Action::CopyFile { .. }
                | Action::CreateDirectory { .. } | Action::RestoreBackup { .. }
            );
            if is_write {
                let ro_msg = format!(
                    "READ-ONLY PHASE: You are in {} mode. No modifications allowed for first {} iterations. \
                     You are on iteration {}. Only read, list, search, or execute commands. \
                     Inspect the codebase first.",
                    behavior.as_str(), read_only_until, iteration + 1,
                );
                conversation.push(Message::system(ro_msg));
                session.variables.insert("last_action".to_string(), action_type.to_string());
                session.variables.insert("last_result".to_string(), "Blocked by read-only phase".to_string());
                continue;
            }
        }

        // --- Refactor mode: enforce test run before first write ---
        if behavior == crate::core::behavior::BehaviorMode::Refactor && !tests_passed_before_write {
            if matches!(&action, Action::WriteFile { .. } | Action::DeleteFile { .. } | Action::CopyFile { .. }) {
                let test_msg = "REFACTOR MODE: You must run the existing tests BEFORE making any changes. \
                                Execute the test command first, verify tests pass, then proceed with changes.";
                conversation.push(Message::system(test_msg));
                session.variables.insert("last_action".to_string(), action_type.to_string());
                session.variables.insert("last_result".to_string(), "Blocked: run tests first".to_string());
                continue;
            }
            if matches!(&action, Action::ExecuteCommand { .. }) {
                // Assume any command might be a test — clear the gate after one execution
                tests_passed_before_write = true;
            }
        }

        // --- TDD phase enforcement ---
        if behavior == crate::core::behavior::BehaviorMode::Tdd {
            match tdd_phase {
                TddPhase::Red => {
                    // Red phase: only allow writing test files, reading, running tests
                    let is_test_write = matches!(&action, Action::WriteFile { path, .. } if path.contains("test") || path.ends_with("_test.rs") || path.ends_with(".spec.ts"));
                    let _is_read_or_command = matches!(&action, Action::ReadFile { .. } | Action::ExecuteCommand { .. } | Action::ListDirectory { .. } | Action::SearchFiles { .. });
                    if matches!(&action, Action::WriteFile { .. }) && !is_test_write {
                        let tdd_msg = "TDD RED PHASE: You must write a FAILING TEST first. \
                                        Only create test files now. Implementation code goes in the GREEN phase.";
                        conversation.push(Message::system(tdd_msg));
                        session.variables.insert("last_action".to_string(), action_type.to_string());
                        session.variables.insert("last_result".to_string(), "Blocked: write test first".to_string());
                        continue;
                    }
                    // Check if test was written and run — heuristic: look for test write then command execute
                    if is_test_write {
                        tdd_phase = TddPhase::Green;
                    }
                }
                TddPhase::Green => {
                    // Green phase: write minimal code to pass tests
                    if matches!(&action, Action::Finish { .. }) {
                        let tdd_msg = "TDD GREEN PHASE: The test exists. Write minimal implementation code to make it pass before finishing.";
                        conversation.push(Message::system(tdd_msg));
                    }
                }
                TddPhase::Refactor => {
                    // Refactor phase: already tracked separately
                }
            }
        }

        // Security policy evaluation
        let policy_result = ctx.security.policy.evaluate(&action, ctx.security.current_mode, &ctx.paths);
        match policy_result {
            PolicyResult::Allow => {},
            PolicyResult::Ask => {
                if ctx.auto_approve {
                    // --yes flag: auto-approve without prompting
                } else if !session.tui_mode {
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

        // --- PlanStep action enforcement (soft guidance) ---
        if let Some(ref p) = plan {
            if let Some(step) = p.steps.get(current_step) {
                if step.action != "execute_command" && action_type != step.action && step.status == StepStatus::Pending {
                    let guidance = format!(
                        "NOTE: The current plan step '{}' suggests action '{}', but you used '{}'. \
                         This is not a block, but try to align with the plan's suggested action if possible.",
                        step.description, step.action, action_type
                    );
                    conversation.push(Message::system(guidance));
                }
            }
        }

        // --- Plan action handling (before executor) ---
        if matches!(&action, Action::PlanCreate { .. } | Action::PlanStepComplete { .. } | Action::PlanStepFail { .. } | Action::PlanStatus { .. }) {
            let result_text = handle_plan_action(
                &action, &mut plan, &mut current_step,
                &mut step_iterations, &mut step_failures, &mut plan_recovery_fired,
            );
            conversation.push(Message::tool(result_text.clone(), format!("plan_{}", iteration)));
            session.variables.insert("last_action".to_string(), action_type.to_string());
            session.variables.insert("last_result".to_string(), result_text);
            continue;
        }

        // Execute the action
        match ToolExecutor::execute(&action, &ctx, &tool_registry).await {
            Ok(output) => {
                if !session.tui_mode {
                    eprintln!("  ✓ Success");
                    // Show tool result in the streaming output
                    let display_out = if output.len() > 2000 {
                        format!("{}... [truncated {} chars]", &output[..2000], output.len() - 2000)
                    } else {
                        output.clone()
                    };
                    eprintln!("─── Result ───");
                    eprintln!("{}", display_out);
                }
                tracing::info!("Action succeeded: {}", action.description());

                // Truncate large outputs for conversation
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

                // --- Plan step completion detection ---
                if let Some(ref mut p) = plan {
                    if let Some(step) = p.steps.get(current_step) {
                        // Check if step action matches or is generic
                        let action_matches = step.action == "execute_command" || action_type == step.action;
                        if action_matches && step.status != StepStatus::Completed {
                            Planner::complete_step(p, current_step, format!("{} succeeded", action_type));

                            // If this was a WriteFile to a test file in TDD mode, advance phase
                            if behavior == crate::core::behavior::BehaviorMode::Tdd {
                                if action_type == "write_file" && tdd_phase == TddPhase::Red {
                                    tdd_phase = TddPhase::Green;
                                } else if action_type == "write_file" && tdd_phase == TddPhase::Green {
                                    tdd_phase = TddPhase::Refactor;
                                } else if action_type == "execute_command" && tdd_phase == TddPhase::Green {
                                    tdd_phase = TddPhase::Refactor;
                                }
                            }

                            // If Refactor mode test succeeded, allow writes
                            if behavior == crate::core::behavior::BehaviorMode::Refactor && action_type == "execute_command" {
                                let output_lower = output.to_lowercase();
                                if !output_lower.contains("fail") && !output_lower.contains("error") {
                                    tests_passed_before_write = true;
                                }
                            }

                            // Advance to next ready step
                            let next = Planner::next_ready_step(p);
                            match next {
                                Some(idx) => {
                                    current_step = idx;
                                    step_iterations.insert(idx as u32, 0);
                                    step_failures.insert(idx as u32, 0);
                                    if !session.tui_mode {
                                        eprintln!("── [Plan step {}/{} complete. Moving to step {}: {}] ──",
                                            current_step, p.steps.len(), current_step + 1,
                                            p.steps.get(current_step).map(|s| s.description.as_str()).unwrap_or("done"));
                                    }
                                }
                                None => {
                                    // All steps done!
                                    if p.steps.iter().all(|s| s.status == StepStatus::Completed) {
                                        if !session.tui_mode {
                                            eprintln!("── [All plan steps complete!] ──");
                                        }
                                        current_step = p.steps.len(); // past-the-end sentinel
                                    }
                                }
                            }
                        }
                    }
                }
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

                        // Fix mode: include hypothesis in mistake record
                        if behavior == crate::core::behavior::BehaviorMode::Fix {
                            if let Some(ref hyp) = fix_hypothesis {
                                mem.add_mistake(
                                    &format!("{} (hypothesis was: {})", session.base_task, hyp),
                                    &error_msg,
                                    &lesson,
                                    "Hypothesis was wrong. Re-evaluate the problem and try a different approach.",
                                    &action_history,
                                );
                            }
                        }
                    }
                }

                // Self-correction prompt after repeated failures
                if failure_tracker.should_suggest_correction(&action_type_str, 3) {
                    let correction = failure_tracker.correction_prompt(&action_type_str);
                    conversation.push(Message::system(correction));
                }

                // --- Plan step failure handling ---
                if let Some(ref mut p) = plan {
                    let current_step_idx = current_step;
                    let fc = step_failures.entry(current_step_idx as u32).or_insert(0);
                    *fc += 1;
                    let fail_count = *fc;

                    if fail_count >= 3 {
                        // Exceeded retries — check if retryable
                        if Planner::retryable_steps(p).contains(&current_step_idx) {
                            Planner::increment_retry(p, current_step_idx);
                            let retry_msg = format!(
                                "Plan step '{}' failed. Retry attempt {}/3. Try a different approach.",
                                p.steps.get(current_step_idx).map(|s| s.description.as_str()).unwrap_or("unknown"),
                                p.steps.get(current_step_idx).map(|s| s.retry_count).unwrap_or(0)
                            );
                            conversation.push(Message::system(retry_msg));
                        } else {
                            // Max retries reached — mark failed, move on
                            Planner::fail_step(p, current_step_idx, format!("Failed after multiple retries: {}", e));
                            let next = Planner::next_ready_step(p);
                            match next {
                                Some(idx) => {
                                    current_step = idx;
                                    step_iterations.insert(idx as u32, 0);
                                    step_failures.insert(idx as u32, 0);
                                    if !session.tui_mode {
                                        eprintln!("── [Step failed. Moving to next ready step {}: {}] ──",
                                            current_step + 1,
                                            p.steps.get(current_step).map(|s| s.description.as_str()).unwrap_or("done"));
                                    }
                                }
                                None => {}
                            }
                        }
                    }
                }

                // --- Plan replanning trigger ---
                if let Some(ref mut p) = plan {
                    let consecutive_fails: u32 = (0..p.steps.len())
                        .filter_map(|i| step_failures.get(&(i as u32)))
                        .sum();
                    if consecutive_fails >= 6 && !plan_recovery_fired {
                        plan_recovery_fired = true;
                        conversation.push(Message::system(
                            "Replan needed: multiple steps are failing. Update PLAN.md with a new approach for the remaining steps. \
                             Consider breaking large steps into smaller ones."
                        ));
                    }
                }
            }
        }

        // --- Plan tracking ---
        let workspace = ctx.paths.workspace_dir();
        let plan_path = workspace.join("PLAN.md");

        // Try to load PLAN.md on iterations 1+ if not yet loaded
        if iteration >= 1 && plan.is_none() && !plan_checked {
            plan_checked = true;
            if plan_path.exists() {
                if let Ok(p) = PlanParser::from_file(&plan_path, &session.base_task) {
                    let total = p.steps.len();
                    plan = Some(p);
                    // Initialize current_step to first ready step
                    if let Some(ref p) = plan {
                        current_step = Planner::next_ready_step(p).unwrap_or(0);
                    }
                    if !session.tui_mode {
                        eprintln!("── [Plan loaded: {} steps] ──", total);
                    }
                }
            }
        }

        // Re-parse PLAN.md only when written to PLAN.md specifically
        if matches!(&action, Action::WriteFile { path, .. } if path.ends_with("PLAN.md") || path.ends_with("plan.md")) {
            if plan_path.exists() {
                if let Ok(p) = PlanParser::from_file(&plan_path, &session.base_task) {
                    plan = Some(p);
                    // Reset step tracking for re-parsed plan
                    if let Some(ref p) = plan {
                        current_step = Planner::next_ready_step(p).unwrap_or(0);
                    }
                    step_iterations.clear();
                    step_failures.clear();
                    plan_recovery_fired = false;
                    if !session.tui_mode {
                        eprintln!("── [Plan re-parsed from PLAN.md] ──");
                    }
                }
            }
        }

        // Increment per-step iteration counter
        if plan.is_some() && current_step < plan.as_ref().map(|p| p.steps.len()).unwrap_or(0) {
            *step_iterations.entry(current_step as u32).or_insert(0) += 1;

            // Per-step iteration limit enforcement
            let step_iter = step_iterations[&(current_step as u32)];
            let per_step_limit: u32 = 100;
            if step_iter >= per_step_limit {
                if let Some(ref mut p) = plan {
                    Planner::fail_step(p, current_step, "Exceeded per-step iteration limit.".to_string());
                    let next = Planner::next_ready_step(p);
                    match next {
                        Some(idx) => {
                            current_step = idx;
                            step_iterations.insert(idx as u32, 0);
                            step_failures.insert(idx as u32, 0);
                        }
                        None => {}
                    }
                }
            }
        }

        // Save session state with plan metadata
        if let Some(ref p) = plan {
            let done_count = p.steps.iter().filter(|s| s.status == StepStatus::Completed).count();
            session.variables.insert("plan_current_step".to_string(), current_step.to_string());
            session.variables.insert("plan_total_steps".to_string(), p.steps.len().to_string());
            session.variables.insert("plan_completed_steps".to_string(), done_count.to_string());
            session.variables.insert("plan_goal".to_string(), p.goal.clone());
        }
        crate::agent::session::save_session_metadata(&ctx, &session)?;

        // Check loop exit conditions
        if session.status != crate::agent::session::SessionStatus::Active {
            break;
        }

        // Safety: break on absurdly high iteration count (catastrophic loop protection)
        iteration += 1;
        if iteration > max_iters {
            session.status = crate::agent::session::SessionStatus::Failed;
            session.final_output = Some("Safety limit reached: too many iterations without completion.".to_string());
            break;
        }
    }

    Ok(session)
}

fn format_for_display(text: &str) -> String {
    text
        .replace("<think>", "─── Reasoning ───\n")
        .replace("</think>", "\n───────────────\n")
        .replace("```json\n", "─── Tool Call ───\n")
}

fn strip_think_tags(text: &str) -> String {
    let re = Regex::new(r"(?s)<think>.*?</think>").unwrap();
    re.replace_all(text, "").to_string()
}

fn tool_call_to_action(tc: &crate::models::message::ToolCall, fallback_text: &str) -> Action {
    match tc.function.name.as_str() {
        "read_file" => Action::ReadFile { path: tc.function.arguments.clone(), start_line: None, end_line: None },
        "write_file" => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                Action::WriteFile {
                    path: val.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    content: val.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    append: None,
                }
            } else {
                Action::Finish { output: Some(fallback_text.to_string()) }
            }
        }
        "execute_command" => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                Action::ExecuteCommand {
                    command: val.get("command").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    timeout: val.get("timeout").and_then(|v| v.as_u64()),
                    workdir: val.get("workdir").and_then(|v| v.as_str()).map(|s| s.to_string()),
                }
            } else {
                Action::Finish { output: Some(fallback_text.to_string()) }
            }
        }
        "search_files" => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                Action::SearchFiles {
                    pattern: val.get("pattern").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    path: val.get("path").and_then(|v| v.as_str()).map(|s| s.to_string()),
                    search_type: val.get("type").and_then(|v| v.as_str()).map(|s| s.to_string()),
                }
            } else {
                Action::SearchFiles { pattern: tc.function.arguments.clone(), path: None, search_type: None }
            }
        }
        "list_directory" => {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&tc.function.arguments) {
                Action::ListDirectory {
                    path: val.get("path").and_then(|v| v.as_str()).unwrap_or(".").to_string(),
                    pattern: val.get("pattern").and_then(|v| v.as_str()).map(|s| s.to_string()),
                }
            } else {
                Action::ListDirectory { path: tc.function.arguments.clone(), pattern: None }
            }
        }
        "finish" => Action::Finish { output: Some(fallback_text.to_string()) },
        "web_search" => Action::WebSearch { query: tc.function.arguments.clone(), num_results: None },
        "web_fetch" => Action::WebFetch { url: tc.function.arguments.clone(), max_chars: None },
        "ask_user" => Action::AskUser { question: tc.function.arguments.clone() },
        _ => Action::Finish { output: Some(fallback_text.to_string()) },
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
        Action::WebSearch { .. } => "web_search",
        Action::WebFetch { .. } => "web_fetch",
        Action::ShowChanges { .. } => "show_changes",
        Action::RestoreBackup { .. } => "restore_backup",
        Action::PlanCreate { .. } => "plan_create",
        Action::PlanStepComplete { .. } => "plan_step_complete",
        Action::PlanStepFail { .. } => "plan_step_fail",
        Action::PlanStatus {} => "plan_status",
        Action::CreateSkill { .. } => "create_skill",
        Action::InvokeSkill { .. } => "invoke_skill",
        Action::ListSkills { .. } => "list_skills",
        Action::DeleteSkill { .. } => "delete_skill",
    }
}

fn build_system_prompt(ctx: &AppContext, supports_native_tools: bool) -> String {
    let mode = ctx.security.current_mode.as_str();
    let behavior = crate::core::behavior::BehaviorMode::from_str(&ctx.config.default_behavior).unwrap_or_default();
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
- write_file:     Write content to a file (creates file and parent dirs automatically)
  JSON: {{"action": "write_file", "path": "file.txt", "content": "..."}}
- delete_file:    Delete a file or files matching a glob pattern
  JSON: {{"action": "delete_file", "path": "file.txt"}}
  JSON: {{"action": "delete_file", "path": ".", "pattern": "*.txt"}}
  NOTE: To match ALL files use "pattern": "*" (not "*.*") which misses files without a dot)
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

--- CHANGE TRACKING ---
- show_changes:   Show all file changes from git diff or recently modified files
  JSON: {{"action": "show_changes"}}
  JSON: {{"action": "show_changes", "path": "src/"}}
- restore_backup: Restore a file to its original state from git
  JSON: {{"action": "restore_backup", "path": "src/main.rs"}}

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
Instead of manually editing a PLAN.md file, use the built-in plan tools:

- plan_create:    Create a plan with ordered steps
  JSON: {"action": "plan_create", "goal": "Fix bugs", "steps": [{"id": 1, "description": "Read source", "action": "read_file"}, {"id": 2, "description": "Fix code", "action": "write_file", "depends_on": [1]}]}
- plan_step_complete: Mark a step as done
  JSON: {"action": "plan_step_complete", "step_id": 1, "result": "Read the file"}
- plan_step_fail: Mark a step as failed
  JSON: {"action": "plan_step_fail", "step_id": 1, "error": "File not found"}
- plan_status:     Check current plan progress
  JSON: {"action": "plan_status"}

The agent loop will automatically advance to the next ready step when a step completes.
Steps can declare dependencies with 'depends_on' (list of step IDs)."#;

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

    let behavior_note = behavior.system_prompt_note();
    let skill_injection = ctx.skill_registry
        .lock()
        .map(|r| r.as_prompt_injection())
        .unwrap_or_default();

    if supports_native_tools {
        format!(
            r#"You are VO1D, an advanced system agent running in {mode} mode.
Your workspace is: {ws}

{conversational_note}

{planning_note}
{behavior_note}
{tool_docs}

{tool_instructions}

RULES:
- Always reason before each action
- All paths are relative to: {ws}
- OS: {os} — use {shell} syntax for shell commands
- Current mode: {mode}
- Behavioral mode: {behavior_name}
- Use "pattern": "*" (not "*.*") to match ALL files{doc_context}{memory}{skill_injection}"#,
            mode = mode, ws = ws, behavior_name = behavior.as_str(),
            conversational_note = conversational_note,
            planning_note = planning_note,
            behavior_note = if behavior_note.is_empty() { String::new() } else { format!("\n{}\n", behavior_note) },
            tool_docs = tool_docs, tool_instructions = tool_instructions,
            os = os_hint, shell = shell_hint, doc_context = doc_context, memory = memory_context,
            skill_injection = skill_injection,
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
{behavior_note}
{tool_docs}

{tool_instructions_sim}

RULES:
- All paths are relative to: {ws}
- OS: {os} — use {shell} syntax for shell commands
- Current mode: {mode}
- Behavioral mode: {behavior_name}
- Use "pattern": "*" (not "*.*") to match ALL files{doc_context}{memory}{skill_injection}"#,
            mode = mode, ws = ws, behavior_name = behavior.as_str(),
            conversational_note = conversational_note,
            planning_note = planning_note,
            behavior_note = if behavior_note.is_empty() { String::new() } else { format!("\n{}\n", behavior_note) },
            tool_docs = tool_docs, tool_instructions_sim = tool_instructions_sim,
            os = os_hint, shell = shell_hint, doc_context = doc_context, memory = memory_context,
            skill_injection = skill_injection,
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

/// Handle plan-related actions dispatched directly in the loop.
/// Returns a result string to be added to conversation.
fn handle_plan_action(
    action: &Action,
    plan: &mut Option<Plan>,
    current_step: &mut usize,
    step_iterations: &mut HashMap<u32, u32>,
    step_failures: &mut HashMap<u32, u32>,
    plan_recovery_fired: &mut bool,
) -> String {
    match action {
        Action::PlanCreate { goal, steps } => {
            let plan_steps: Vec<PlanStep> = steps.iter().map(|s| s.to_step()).collect();
            let total = plan_steps.len();
            let new_plan = Planner::new(100).create_plan(goal, plan_steps);
            *plan = Some(new_plan);
            if let Some(ref p) = *plan {
                *current_step = Planner::next_ready_step(p).unwrap_or(0);
            }
            step_iterations.clear();
            step_failures.clear();
            *plan_recovery_fired = false;
            format!("Plan created: {}. {} steps total.", goal, total)
        }
        Action::PlanStepComplete { step_id, result } => {
            if let Some(ref mut p) = *plan {
                // Find step by id
                if let Some(idx) = p.steps.iter().position(|s| s.id == *step_id) {
                    Planner::complete_step(p, idx, result.clone());
                    let next = Planner::next_ready_step(p);
                    match next {
                        Some(n) => {
                            *current_step = n;
                            step_iterations.insert(n as u32, 0);
                            step_failures.insert(n as u32, 0);
                            format!("Step {} completed. Moving to step {}.", step_id, n + 1)
                        }
                        None => {
                            if p.steps.iter().all(|s| s.status == StepStatus::Completed) {
                                format!("Step {} completed. All plan steps done!", step_id)
                            } else {
                                format!("Step {} completed.", step_id)
                            }
                        }
                    }
                } else {
                    format!("Step {} not found in plan.", step_id)
                }
            } else {
                "No active plan.".to_string()
            }
        }
        Action::PlanStepFail { step_id, error } => {
            if let Some(ref mut p) = *plan {
                if let Some(idx) = p.steps.iter().position(|s| s.id == *step_id) {
                    Planner::fail_step(p, idx, error.clone());
                    let next = Planner::next_ready_step(p);
                    match next {
                        Some(n) => {
                            *current_step = n;
                            step_iterations.insert(n as u32, 0);
                            step_failures.insert(n as u32, 0);
                            format!("Step {} failed: {}. Moving to step {}.", step_id, error, n + 1)
                        }
                        None => {
                            format!("Step {} failed: {}", step_id, error)
                        }
                    }
                } else {
                    format!("Step {} not found in plan.", step_id)
                }
            } else {
                "No active plan.".to_string()
            }
        }
        Action::PlanStatus {} => {
            if let Some(ref p) = *plan {
                let done = p.steps.iter().filter(|s| s.status == StepStatus::Completed).count();
                let failed = p.steps.iter().filter(|s| s.status == StepStatus::Failed).count();
                let pending = p.steps.len() - done - failed;
                let current_desc = p.steps.get(*current_step).map(|s| s.description.as_str()).unwrap_or("(none)");
                format!(
                    "Goal: {} | Steps: {}/{} done, {} pending, {} failed | Current: step {} ({})",
                    p.goal, done, p.steps.len(), pending, failed, *current_step + 1, current_desc
                )
            } else {
                "No active plan.".to_string()
            }
        }
        _ => unreachable!(),
    }
}
