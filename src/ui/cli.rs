use crate::AppContext;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

/// CLI output formatting helper.
pub struct CliOutput;

impl CliOutput {
    pub fn new() -> Self {
        Self
    }

    /// Print an info message with a styled prefix.
    pub fn info(&self, msg: &str) {
        println!("[VO1D] {}", msg);
    }

    /// Print a success message.
    pub fn success(&self, msg: &str) {
        println!("[✓] {}", msg);
    }

    /// Print a warning message.
    pub fn warn(&self, msg: &str) {
        println!("[!] {}", msg);
    }

    /// Print an error message.
    pub fn error(&self, msg: &str) {
        eprintln!("[ERROR] {}", msg);
    }

    /// Print YOLO mode banner.
    pub fn yolo_banner(&self) {
        println!("\x1b[31m[YOLO MODE]\x1b[0m All actions will be executed with absolute autonomy.");
    }

    /// Create a progress spinner.
    pub fn spinner(&self, msg: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap(),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        pb
    }
}

/// Interactive REPL for chat mode.
pub async fn interactive_repl(ctx: AppContext) -> Result<()> {
    let output = CliOutput::new();
    let mut input = String::new();

    loop {
        input.clear();
        print!(
            "VO1D [{}] >> ",
            ctx.security.current_mode.as_str()
        );
        std::io::Write::flush(&mut std::io::stdout())?;

        std::io::stdin()
            .read_line(&mut input)
            .context("Failed to read input")?;

        let trimmed = input.trim();
        match trimmed {
            "" => continue,
            "/exit" | "/quit" => {
                println!("Goodbye.");
                break;
            }
            "/help" => {
                println!("Commands:");
                println!("  <task>     Execute a task");
                println!("  /exit      Exit the REPL");
                println!("  /help      Show this help");
                println!("  /memory    Show learned memory");
                println!("  /model     Show current model");
                println!("  /mode      Show security mode");
                println!("  /session   Show session info");
                continue;
            }
            "/model" => {
                println!("Model: {}", ctx.config.default_model);
                continue;
            }
            "/mode" => {
                println!("Mode: {}", ctx.security.current_mode);
                continue;
            }
            "/memory" => {
                let mem = ctx.memory.lock().await;
                let stats = mem.stats();
                println!("=== MEMORY ===\n{}\n", stats);
                if !mem.solutions.is_empty() {
                    println!("Recent solutions:");
                    for sol in mem.solutions.iter().rev().take(5) {
                        println!("  {} → {}", sol.task_description, sol.outcome);
                    }
                }
                if !mem.mistakes.is_empty() {
                    println!("\nLearned mistakes:");
                    for mist in mem.mistakes.iter().rev().take(5) {
                        println!("  (freq:{}) {}", mist.frequency, mist.mistake);
                    }
                }
                continue;
            }
            _ => {}
        }

        // Run the task
        let session = crate::agent::session::Session::new(trimmed, &ctx)?;
        match crate::agent::run(ctx.clone(), session).await {
            Ok(final_session) => {
                if let Some(output) = final_session.final_output {
                    println!("\nResult: {}", output);
                }
                println!("\n✓ Task completed. Session: {}", final_session.session_id);
            }
            Err(e) => {
                output.error(&format!("Task failed: {}", e));
            }
        }
    }

    Ok(())
}

/// List models in the catalog with install status.
pub async fn list_models(ctx: &AppContext) -> Result<()> {
    let _output = CliOutput::new();
    let all = ctx.model_registry.list();

    if all.is_empty() {
        println!("No models in catalog.");
        return Ok(());
    }

    // Compute dynamic column widths
    let max_id = all.iter().map(|m| m.id.len()).max().unwrap_or(10).max(10);
    let max_name = all.iter().map(|m| m.name.len()).max().unwrap_or(20).max(20);
    let total_width = max_id + max_name + 10 + 5 + 5 + 8 + 3 + 18;

    println!("{:<id$} {:<name$} {:<10} {:<5} {:<5} {:<8} {:>3}",
        "ID", "Name", "Size", "Reas", "Inst", "RAM", "",
        id = max_id, name = max_name);
    println!("{}", "-".repeat(total_width));

    for model in all {
        let status = if ctx.model_registry.is_installed(model) {
            "✓"
        } else {
            "✗"
        };

        let size_gb = model.size_bytes as f64 / 1_073_741_824.0;
        let size_str = if size_gb >= 1.0 {
            format!("{:.1} GB", size_gb)
        } else {
            format!("{:.0} MB", model.size_bytes as f64 / 1_048_576.0)
        };

        println!("{:<id$} {:<name$} {:<10} {:<5} {:<5} {:<8} {:>3}",
            model.id,
            model.name,
            size_str,
            if model.reasoning { "✓" } else { "" },
            if model.instruct { "✓" } else { "" },
            format!("{:.0} GB", model.min_ram_gb),
            status,
            id = max_id, name = max_name,
        );
    }

    Ok(())
}

/// Show the hardware profile and recommended models.
pub async fn show_hardware_profile(ctx: &AppContext) -> Result<()> {
    let hw = &ctx.hardware;

    println!("=== Hardware Profile ===");
    println!("CPU: {} ({} cores)", hw.cpu_name, hw.cpu_cores);
    println!("RAM: {:.1} GB total, {:.1} GB available", hw.total_ram_gb, hw.available_ram_gb);
    println!("Tier: {:?}", hw.tier);
    println!("Recommended Max Context: {} tokens", hw.recommended_max_context);
    println!("\nGPU(s):");
    for gpu in &hw.gpu_info {
        let vram = gpu.dedicated_vram_mb
            .map(|m| format!("{} MB VRAM", m))
            .unwrap_or_else(|| "No dedicated VRAM".to_string());
        println!("  {} ({}) - {}", gpu.name, gpu.vendor, vram);
    }

    println!("\nCompatible Models:");
    let compatible = ctx.model_registry.compatible_with_hardware(hw);
    for model in compatible.iter().take(5) {
        let installed = if ctx.model_registry.is_installed(model) { "✓" } else { "✗" };
        println!("  [{}] {} ({})", installed, model.name, model.id);
    }
    if compatible.len() > 5 {
        println!("  ... and {} more compatible models", compatible.len() - 5);
    }

    Ok(())
}
