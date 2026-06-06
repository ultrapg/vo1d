use crate::agent::session::Session;
use crate::core::curriculum::{evaluate_task, Curriculum, EvaluationResult};
use crate::AppContext;
use anyhow::Result;
use std::path::Path;
use std::time::Instant;

/// Runs the agent on a curriculum of training tasks.
pub async fn run_curriculum(ctx: AppContext, curriculum_path: &str) -> Result<()> {
    let curriculum = Curriculum::load(curriculum_path)?;
    let curriculum_name = curriculum.name.clone();
    let total = curriculum.task_count();

    println!("\n=== TRAINING MODE: {} ===", curriculum_name);
    println!("Description: {}", curriculum.description);
    println!("Total tasks: {}\n", total);

    let sandbox = ctx.paths.workspace_dir().join("train_sandbox");
    let results = run_all_tasks(ctx, &curriculum, &sandbox).await?;

    print_summary(&curriculum, &results);
    Ok(())
}

/// Run all tasks in a curriculum sequentially.
async fn run_all_tasks(ctx: AppContext, curriculum: &Curriculum, sandbox: &Path) -> Result<Vec<EvaluationResult>> {
    let mut results = Vec::new();

    for (i, task) in curriculum.tasks.iter().enumerate() {
        println!("─── Task {}/{}: {} ───", i + 1, curriculum.task_count(), task.id);
        println!("  {}", task.description);

        // Clean sandbox before each task
        let _ = std::fs::remove_dir_all(sandbox);
        std::fs::create_dir_all(sandbox)?;

        let result = run_single_task(&ctx, task, sandbox).await?;

        results.push(result.clone());

        // Store outcome in memory
        if let Ok(mut mem) = ctx.memory.lock() {
            mem.add_task(
                &task.id,
                vec![format!("train:{}", task.description)],
                &result.outcome,
            );
            if result.passed {
                mem.add_pattern(
                    &format!("task:{}", task.id),
                    &format!("Completed: {}", task.expected_outcome),
                );
            }
        }

        if result.passed {
            println!("  ✓ PASSED");
        } else {
            println!("  ✗ FAILED");
            for detail in &result.details {
                println!("    {}", detail);
            }
        }
        println!();
    }

    Ok(results)
}

/// Run the agent on a single training task, then evaluate.
async fn run_single_task(ctx: &AppContext, task: &crate::core::curriculum::TaskDefinition, sandbox: &Path) -> Result<EvaluationResult> {
    // Set up the task description to guide the agent
    let prompt = format!(
        r#"Training exercise: {}

Work inside the sandbox directory: {}
Your task: {}

Expected outcome: {}
Complete this task and then use "finish" to signal completion."#,
        task.id,
        sandbox.display(),
        task.description,
        task.expected_outcome,
    );

    let start = Instant::now();

    // Run agent loop
    let session = Session::new(&prompt, ctx)?;
    let result = crate::agent::loop_::agent_loop(ctx.clone(), session).await?;

    let elapsed = start.elapsed();

    // Evaluate task completion
    let evaluation = evaluate_task(task, sandbox);

    // Log to memory
    let actions_taken: Vec<String> = result.variables.get("action_count")
        .map(|c| vec![format!("{} actions in {:?}", c, elapsed)])
        .unwrap_or_default();

    if let Ok(mut mem) = ctx.memory.lock() {
        mem.add_task(
            &format!("train:{}", task.id),
            actions_taken,
            &format!("{} ({:?})", evaluation.outcome, elapsed),
        );
    }

    Ok(evaluation)
}

/// Print a summary of all training results.
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
