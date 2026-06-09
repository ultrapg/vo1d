use crate::agent::session::Session;
use crate::core::curriculum::{evaluate_task, Curriculum, EvaluationResult};
use crate::AppContext;
use anyhow::Result;
use std::path::Path;
use std::time::Instant;

/// Runs a curriculum by name, trying disk then embedded fallback.
pub async fn run_curriculum_by_name(ctx: AppContext, name: &str, manual: bool) -> Result<()> {
    let curriculum = Curriculum::load_from_name(&ctx, name)?;
    run_curriculum_raw_with(ctx, curriculum, manual).await
}

/// Ordered list of built-in curricula for autotrain.
const AUTOTRAIN_CURRICULA: &[&str] = &[
    "00_hello_world",
    "01_file_ops",
    "02_directory_ops",
    "03_search_nav",
    "04_shell_basics",
    "05_web_basics",
    "06a_syntax",
    "06b_deps",
    "06c_logic",
    "06d_multi_file",
    "06e_environment",
    "07_project_setup",
];

/// Runs all built-in curricula in sequence (autotrain mode).
pub async fn run_autotrain(ctx: AppContext, manual: bool) -> Result<()> {
    run_autotrain_with_policy(ctx, manual, "skip").await
}

/// Runs autotrain with a configurable failure policy.
/// `on_failure` can be "stop", "skip", or "retry" (with max 3 retries).
pub async fn run_autotrain_with_policy(ctx: AppContext, manual: bool, on_failure: &str) -> Result<()> {
    println!("\n=== AUTOTRAIN MODE (on_failure={}) ===", on_failure);
    println!("Running all {} curricula in sequence...\n", AUTOTRAIN_CURRICULA.len());

    for (i, name) in AUTOTRAIN_CURRICULA.iter().enumerate() {
        println!("\n═══ Curriculum {}/{}: {} ═══", i + 1, AUTOTRAIN_CURRICULA.len(), name);

        let curriculum = match Curriculum::load_from_name(&ctx, name) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("  ⚠ {} - {}", name, e);
                match on_failure {
                    "stop" => return Err(anyhow::anyhow!("Autotrain stopped: {}", e)),
                    _ => continue,
                }
            }
        };

        let result = run_curriculum_raw_with(ctx.clone(), curriculum, manual).await;

        match result {
            Ok(_) => {}
            Err(e) => {
                eprintln!("  ✗ Curriculum {} failed: {}", name, e);
                match on_failure {
                    "stop" => return Err(anyhow::anyhow!("Autotrain stopped on {}: {}", name, e)),
                    "retry" => {
                        let mut retries = 0;
                        loop {
                            if retries >= 3 {
                                eprintln!("  ✗ {} failed after 3 retries, skipping.", name);
                                break;
                            }
                            println!("  Retrying {} (attempt {})...", name, retries + 2);
                            match Curriculum::load_from_name(&ctx, name) {
                                Ok(c) => {
                                    if run_curriculum_raw_with(ctx.clone(), c, manual).await.is_ok() {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                            retries += 1;
                        }
                    }
                    _ => {} // skip
                }
            }
        }

        if i + 1 < AUTOTRAIN_CURRICULA.len() {
            println!("\n  --- Moving to next curriculum in 2 seconds... ---");
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    println!("\n=== AUTOTRAIN COMPLETE ===");
    Ok(())
}

/// Runs the agent on a curriculum loaded from a file path.
/// Falls back to embedded if the path doesn't exist.
pub async fn run_curriculum(ctx: AppContext, name_or_path: &str, manual: bool) -> Result<()> {
    let path = std::path::Path::new(name_or_path);
    let curriculum = if path.is_file() {
        Curriculum::load(path)?
    } else {
        // Treat as name, try disk + embedded
        Curriculum::load_from_name(&ctx, name_or_path)?
    };
    run_curriculum_raw_with(ctx, curriculum, manual).await
}

async fn run_curriculum_raw_with(ctx: AppContext, curriculum: Curriculum, manual: bool) -> Result<()> {
    let curriculum_name = curriculum.name.clone();
    let total = curriculum.task_count();

    println!("\n=== TRAINING MODE: {} ===", curriculum_name);
    println!("Description: {}", curriculum.description);
    println!("Total tasks: {}\n", total);

    let sandbox = ctx.paths.workspace_dir().join("train_sandbox");
    let results = if manual {
        run_all_tasks_manual(&curriculum, &sandbox).await?
    } else {
        run_all_tasks(ctx, &curriculum, &sandbox).await?
    };

    print_summary(&curriculum, &results);
    Ok(())
}

/// Run all tasks via the agent loop.
async fn run_all_tasks(ctx: AppContext, curriculum: &Curriculum, sandbox: &Path) -> Result<Vec<EvaluationResult>> {
    let mut results = Vec::new();

    for (i, task) in curriculum.tasks.iter().enumerate() {
        println!("─── Task {}/{}: {} ───", i + 1, curriculum.task_count(), task.id);
        println!("  {}", task.description);

        let _ = std::fs::remove_dir_all(sandbox);
        std::fs::create_dir_all(sandbox)?;

        let result = run_single_task(&ctx, task, sandbox).await?;

        store_in_memory(&ctx, task, &result).await;

        if result.passed {
            println!("  ✓ PASSED");
        } else {
            println!("  ✗ FAILED");
            for detail in &result.details {
                println!("    {}", detail);
            }
        }
        println!();
        results.push(result);
    }

    Ok(results)
}

/// Run all tasks in manual mode (user completes tasks, model is not required).
async fn run_all_tasks_manual(curriculum: &Curriculum, sandbox: &Path) -> Result<Vec<EvaluationResult>> {
    let mut results = Vec::new();

    for (i, task) in curriculum.tasks.iter().enumerate() {
        println!("─── Task {}/{}: {} ───", i + 1, curriculum.task_count(), task.id);
        println!("  {}", task.description);
        println!();
        println!("  Expected outcome: {}", task.expected_outcome);
        println!();
        println!("  Work inside the sandbox directory:");
        println!("    {}", sandbox.display());
        println!();
        println!("  Press Enter when done, or type 'skip' to skip this task.");
        println!();

        let _ = std::fs::remove_dir_all(sandbox);
        std::fs::create_dir_all(sandbox)?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("skip") {
            results.push(EvaluationResult {
                task_id: task.id.clone(),
                passed: false,
                details: vec!["Skipped by user".to_string()],
                outcome: "skipped".to_string(),
            });
            println!("  ⏭ SKIPPED\n");
            continue;
        }

        let evaluation = evaluate_task(task, sandbox);

        if evaluation.passed {
            println!("  ✓ PASSED\n");
        } else {
            println!("  ✗ FAILED");
            for detail in &evaluation.details {
                println!("    {}", detail);
            }
            println!();
        }

        results.push(evaluation);
    }

    Ok(results)
}

async fn run_single_task(ctx: &AppContext, task: &crate::core::curriculum::TaskDefinition, sandbox: &Path) -> Result<EvaluationResult> {
    // Run setup commands to create testing environment
    if task.setup.is_some() {
        println!("  Setting up test environment...");
        if let Err(e) = execute_setup(task, sandbox) {
            eprintln!("  ⚠ Setup error: {}", e);
        }
    }

    // Pull relevant past experiences into the prompt
    let memory_recall = ctx.memory.lock().await.to_context_string_with_recall(&task.description);

    let setup_note = if task.setup.is_some() {
        "\nThe sandbox has been pre-configured with a test environment. Inspect it first before making changes."
    } else {
        ""
    };

    let prompt = format!(
        r#"Training exercise: {}

Work inside the sandbox directory: {}
Your task: {}

Expected outcome: {}
Complete this task and then use "finish" to signal completion.
{}{}

WORKFLOW:
1. Understand what needs to be done
2. Execute each step one at a time
3. Check your work after each step
4. When all steps are done, call finish

For complex multi-step tasks, consider creating a plan (via plan_create tool or PLAN.md) to track progress.
For simple tasks, just do the work directly.

NOTE: write_file automatically creates parent directories — no need to use create_directory first."#,
        task.id,
        sandbox.display(),
        task.description,
        task.expected_outcome,
        setup_note,
        if memory_recall.is_empty() { String::new() } else { format!("\nPrevious experiences to learn from:\n{}", memory_recall) },
    );

    let start = Instant::now();

    // Override workspace to the sandbox so file operations resolve correctly
    let mut train_ctx = ctx.clone();
    train_ctx.paths = ctx.paths.with_workspace_override(sandbox.to_path_buf());

    let session = Session::new(&prompt, &train_ctx)?;
    let result = crate::agent::loop_::agent_loop(train_ctx, session).await?;

    let elapsed = start.elapsed();
    let evaluation = evaluate_task(task, sandbox);

    // Collect action sequence for richer memory (stored as JSON array)
    let actions_taken: Vec<String> = result.variables.get("action_history")
        .and_then(|h| serde_json::from_str(h).ok())
        .unwrap_or_default();
    let action_summary = if actions_taken.is_empty() {
        vec![format!("{} actions in {:?}", result.variables.get("action_count").map(|c| c.as_str()).unwrap_or("?"), elapsed)]
    } else {
        actions_taken.clone()
    };

    let mut mem = ctx.memory.lock().await;
    mem.add_task(
        &format!("train:{}", task.id),
        action_summary,
        &format!("{} ({:?})", evaluation.outcome, elapsed),
    );

    Ok(evaluation)
}

/// Allowed command prefixes for curriculum setup (whitelist approach).
const ALLOWED_SETUP_PREFIXES: &[&str] = &[
    "echo", "mkdir", "rmdir", "del", "copy", "move", "ren", "type",
    "cd", "dir", "set", "if", "for", "attrib", "xcopy", "robocopy",
    "chcp", "ver", "whoami", "date", "time",
    "git init", "git config", "git add", "git commit",
    "cargo init", "npm init",
    "python", "node",
    "powershell -Command \"New-Item", "powershell -Command \"Set-Content",
    "powershell -Command \"Add-Content", "powershell -Command \"Remove-Item",
    "powershell -Command \"Copy-Item", "powershell -Command \"Move-Item",
    "powershell -Command \"Get-Content",
    "touch", "cat", "tee", "printf", "cp", "mv", "rm -f",
    "test", "[", "ln",
];

/// Run setup commands for a task (creates testing environment).
/// Validates each command against a whitelist before execution.
fn execute_setup(task: &crate::core::curriculum::TaskDefinition, sandbox: &Path) -> Result<()> {
    if let Some(ref setup_cmds) = task.setup {
        let shell = if cfg!(windows) { "cmd.exe" } else { "sh" };
        let arg = if cfg!(windows) { "/C" } else { "-c" };
        for cmd in setup_cmds {
            // Validate command against whitelist
            let cmd_trimmed = cmd.trim();
            let is_allowed = ALLOWED_SETUP_PREFIXES.iter().any(|prefix| {
                cmd_trimmed.to_lowercase().starts_with(&prefix.to_lowercase())
            });
            if !is_allowed && !cmd_trimmed.starts_with('#') && !cmd_trimmed.is_empty() {
                anyhow::bail!(
                    "Setup command '{}' is not in the allowed whitelist. \
                     Only safe file/shell operations are permitted in curriculum setup.",
                    cmd_trimmed
                );
            }

            let output = std::process::Command::new(shell)
                .args(&[arg, cmd])
                .current_dir(sandbox)
                .output()?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("  ⚠ Setup command warning: {} => {}", cmd, stderr.trim());
            }
        }
    }
    Ok(())
}

async fn store_in_memory(ctx: &AppContext, task: &crate::core::curriculum::TaskDefinition, result: &EvaluationResult) {
    let mut mem = ctx.memory.lock().await;
    if result.passed {
            let solution = format!("Used correct approach for '{}' and achieved: {}",
                task.description, task.expected_outcome);
            mem.add_solution(&task.description, &solution, &result.outcome, &[]);
            mem.add_pattern(
                &format!("task:{}", task.id),
                &format!("Completed: {}", task.expected_outcome),
            );
        } else {
            let mistakes: Vec<&str> = result.details.iter()
                .filter(|d| d.starts_with("FAIL:"))
                .map(|d| d.as_str())
                .collect();
            let mistake_desc = if mistakes.is_empty() {
                "Unknown failure".to_string()
            } else {
                mistakes.join("; ")
            };
            mem.add_mistake(
                &task.description,
                &mistake_desc,
                "This approach did not work — need to follow the expected outcome more carefully.",
                &format!("When working on '{}', ensure the expected outcome '{}' is fully met. Check file paths carefully.",
                    task.description, task.expected_outcome),
                &[],
            );
        }
}

fn print_summary(curriculum: &Curriculum, results: &[EvaluationResult]) {
    let passed = results.iter().filter(|r| r.passed).count();
    let total = results.len();

    println!("=== TRAINING SUMMARY: {} ===", curriculum.name);
    println!("{}/{} tasks passed ({:.0}%)", passed, total, (passed as f64 / total as f64) * 100.0);
    println!();

    for (i, result) in results.iter().enumerate() {
        let icon = if result.passed { "✓" } else { "✗" };
        println!("  {} {}. {} → {}", icon, i + 1, result.task_id, result.outcome);
        if !result.passed {
            for detail in &result.details {
                if detail.starts_with("FAIL:") {
                    println!("       {}", detail);
                }
            }
        }
    }
    println!();
}

/// Test context compression by generating enough conversation to trigger it.
/// This creates a task that deliberately fills the context window, then observes
/// how the compression system handles it.
pub async fn run_test_compression(ctx: AppContext) -> anyhow::Result<()> {
    println!("\n=== CONTEXT COMPRESSION TEST ===");
    println!("This test creates a task designed to fill the context window and trigger compression.\n");

    let sandbox = ctx.paths.workspace_dir().join("compression_test");
    let _ = std::fs::remove_dir_all(&sandbox);
    std::fs::create_dir_all(&sandbox)?;

    let task_desc = format!(
        r#"COMPRESSION TEST TASK

Work inside: {}

Your goal is to demonstrate context compression by:
1. First, list the workspace (it's empty)
2. Create 15 files (file_1.txt through file_15.txt) each containing a long story about AI (at least 500 words each)
3. Read back each file to verify the content
4. The large amount of content should fill the context window and trigger compression

This is a test — just keep going until you see the compression message or finish.

Expected outcome: The agent handles context compression gracefully without crashing."#,
        sandbox.display()
    );

    let mut train_ctx = ctx.clone();
    train_ctx.paths = ctx.paths.with_workspace_override(sandbox.to_path_buf());

    let session = crate::agent::session::Session::new(&task_desc, &train_ctx)?;
    let result = crate::agent::loop_::agent_loop(train_ctx, session).await?;

    println!("\n=== COMPRESSION TEST RESULT ===");
    match result.status {
        crate::agent::session::SessionStatus::Completed => {
            println!("  ✓ Compression test passed — agent completed without crashing");
        }
        crate::agent::session::SessionStatus::Failed => {
            println!("  ⚠ Compression test finished with status: Failed");
            if let Some(ref out) = result.final_output {
                println!("  Final output: {}", out);
            }
        }
        other => {
            println!("  ⚠ Compression test finished with status: {:?}", other);
        }
    }
    println!();

    Ok(())
}

/// Test all tool types by dispatching actions through the executor.
/// Verifies plan_create, plan_step_complete, plan_step_fail, plan_status,
/// and all other tool registrations are wired correctly.
pub async fn run_test_tools(ctx: AppContext) -> anyhow::Result<()> {
    use crate::models::action::Action;
    use crate::models::action::PlanStepDef;
    use crate::agent::executor::ToolExecutor;
    use crate::tools::registry::ToolRegistry;
    use std::sync::Arc;

    println!("\n=== TOOL TEST ===");

    let registry = Arc::new(ToolRegistry::new(&ctx));

    // 1. Plan creation
    println!("\n1. Testing plan_create...");
    let plan_action = Action::PlanCreate {
        goal: "Test all tools".to_string(),
        steps: vec![
            PlanStepDef {
                id: 1,
                description: "Read files".to_string(),
                action: "read_file".to_string(),
                command: None,
                depends_on: vec![],
            },
            PlanStepDef {
                id: 2,
                description: "Write output".to_string(),
                action: "write_file".to_string(),
                command: None,
                depends_on: vec![1],
            },
        ],
    };
    println!("   ✓ plan_create: {}", plan_action.description());

    // 2. Plan step complete
    let complete_action = Action::PlanStepComplete {
        step_id: 1,
        result: "Read successfully".to_string(),
    };
    println!("   ✓ plan_step_complete: {}", complete_action.description());

    // 3. Plan step fail
    let fail_action = Action::PlanStepFail {
        step_id: 2,
        error: "File not found".to_string(),
    };
    println!("   ✓ plan_step_fail: {}", fail_action.description());

    // 4. Plan status
    let status_action = Action::PlanStatus {};
    println!("   ✓ plan_status: {}", status_action.description());

    // 5. Execute each plan action through executor (should return handled-in-loop)
    for action in &[&plan_action as &Action, &complete_action, &fail_action, &status_action] {
        let result = ToolExecutor::execute(action, &ctx, &registry).await?;
        println!("   → {}", result);
    }

    // 6. Skill tools
    println!("\n1b. Testing skill tools...");
    let create_action = Action::CreateSkill {
        name: "test-skill".into(),
        description: "A test skill".into(),
        params_schema: None,
        steps: vec![
            crate::models::action::SkillStepDef {
                tool: "read_file".into(),
                args: serde_json::json!({"path": "Cargo.toml"}),
            },
        ],
    };
    println!("   ✓ create_skill: {}", create_action.description());
    let list_action = Action::ListSkills { keyword: None };
    println!("   ✓ list_skills: {}", list_action.description());
    let invoke_action = Action::InvokeSkill { name: "nonexistent".into(), params: None };
    println!("   ✓ invoke_skill: {}", invoke_action.description());
    let delete_action = Action::DeleteSkill { name: "test-skill".into() };
    println!("   ✓ delete_skill: {}", delete_action.description());
    // Execute create → list → delete
    let result = ToolExecutor::execute(&create_action, &ctx, &registry).await?;
    println!("   → create: {}", result);
    let result = ToolExecutor::execute(&list_action, &ctx, &registry).await?;
    println!("   → list: {}", result);
    let result = ToolExecutor::execute(&delete_action, &ctx, &registry).await?;
    println!("   → delete: {}", result);

    // 7. Verify all tools are registered
    println!("\n2. Verifying tool registrations...");
    let tool_names = [
        "read_file", "write_file", "execute_command", "list_directory",
        "search_files", "file_metadata", "finish", "delete_file",
        "copy_file", "create_directory", "http_request", "ask_user",
        "web_search", "web_fetch", "show_changes", "restore_backup",
        "plan_create", "plan_step_complete", "plan_step_fail", "plan_status",
        "create_skill", "invoke_skill", "list_skills", "delete_skill",
    ];
    for name in &tool_names {
        if registry.is_registered(name) {
            println!("   ✓ {}", name);
        } else {
            println!("   ✗ {} — NOT REGISTERED", name);
        }
    }

    // 7. Check description() and is_destructive() for all action types
    println!("\n3. Checking action metadata...");
    let all_actions: Vec<Action> = vec![
        Action::ReadFile { path: "x".into(), start_line: None, end_line: None },
        Action::WriteFile { path: "x".into(), content: "".into(), append: None },
        Action::ExecuteCommand { command: "echo hi".into(), timeout: None, workdir: None },
        Action::ListDirectory { path: ".".into(), pattern: None },
        Action::SearchFiles { pattern: "*.rs".into(), path: None, search_type: None },
        Action::DeleteFile { path: "x".into(), pattern: None },
        Action::CopyFile { source: "a".into(), destination: "b".into() },
        Action::CreateDirectory { path: "d".into() },
        Action::FileMetadata { path: "x".into() },
        Action::HttpRequest { url: "https://example.com".into(), method: None, headers: None, body: None },
        Action::Finish { output: None },
        Action::AskUser { question: "test?".into() },
        Action::WebSearch { query: "test".into(), num_results: None },
        Action::WebFetch { url: "https://example.com".into(), max_chars: None },
        Action::ShowChanges { path: None },
        Action::RestoreBackup { path: "x".into() },
        Action::PlanCreate { goal: "g".into(), steps: vec![] },
        Action::PlanStepComplete { step_id: 1, result: "r".into() },
        Action::PlanStepFail { step_id: 1, error: "e".into() },
        Action::PlanStatus {},
        Action::CreateSkill { name: "test".into(), description: "desc".into(), params_schema: None, steps: vec![] },
        Action::InvokeSkill { name: "test".into(), params: None },
        Action::ListSkills { keyword: None },
        Action::DeleteSkill { name: "test".into() },
    ];
    for action in &all_actions {
        println!("   {} (destructive: {})", action.description(), action.is_destructive());
    }

    println!("\n=== TOOL TEST COMPLETE ===\n");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::curriculum::{Curriculum, TaskDefinition};

    #[test]
    fn test_print_summary_no_panic() {
        let curriculum = Curriculum {
            name: "Test".to_string(),
            description: "desc".to_string(),
            tasks: vec![TaskDefinition {
                id: "t1".to_string(),
                description: "d".to_string(),
                expected_outcome: "o".to_string(),
                evaluation: None,
                setup: None,
            }],
        };
        let results = vec![EvaluationResult {
            task_id: "t1".to_string(),
            passed: true,
            details: vec!["ok".to_string()],
            outcome: "passed".to_string(),
        }];
        print_summary(&curriculum, &results);
    }
}
