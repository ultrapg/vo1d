use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use vo1d::security::SecurityMode;
use vo1d::ui::cli::CliOutput;
use vo1d::AppContext;

#[derive(ValueEnum, Clone, Debug)]
enum BehaviorArg {
    Normal,
    Fix,
    Research,
    Refactor,
    Tdd,
}

#[derive(Parser)]
#[command(name = "vo1d", version, about = "VO1D - Local-first autonomous AI execution agent")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Override default model
    #[arg(long, global = true)]
    model: Option<String>,

    /// Override security mode
    #[arg(long, global = true)]
    mode: Option<SecurityModeArg>,

    /// Behavioral mode (normal, fix, research, refactor, tdd)
    #[arg(long, global = true)]
    behavior: Option<BehaviorArg>,

    /// Set custom workspace directory
    #[arg(long, global = true)]
    workspace: Option<PathBuf>,

    /// Enable YOLO mode (absolute autonomy)
    #[arg(long, global = true)]
    yolo: bool,

    /// Auto-approve all actions (Interactive/Power User only)
    #[arg(long, global = true, visible_alias = "jes")]
    yes: bool,

    /// Enable verbose debug tracing
    #[arg(long, global = true)]
    debug: bool,

    /// Resume a session by ID
    #[arg(long, global = true)]
    resume: Option<String>,

    /// Resume after privilege elevation (internal)
    #[arg(long, global = true, hide = true)]
    resume_after_elevation: Option<String>,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum SecurityModeArg {
    Safe,
    Interactive,
    PowerUser,
    Autonomous,
    Yolo,
}

#[derive(Subcommand)]
enum TestAction {
    /// Test context compression by filling the context window then triggering compression
    Compression,
    /// Test all tool types (plan tools, file ops, etc.)
    Tools,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a one-shot task
    Task {
        /// The task description
        task: String,
    },
    /// Start an interactive chat session
    Chat {
        /// Model override for this session
        #[arg(long)]
        model: Option<String>,
    },
    /// List and manage models
    Models {
        #[command(subcommand)]
        action: Option<ModelAction>,
    },
    /// List and manage saved sessions
    Sessions,
    /// Run training curriculum from JSON file
    Train {
        /// Path to curriculum JSON file (or built-in name like "00_hello_world")
        curriculum: Option<String>,
        /// Manual mode: complete tasks yourself without an LLM model
        #[arg(long, default_value_t = false)]
        manual: bool,
        /// Run all curricula in sequence (autotrain)
        #[arg(long, short, default_value_t = false)]
        all: bool,
        /// Autotrain failure policy: stop, skip, or retry
        #[arg(long, default_value = "skip")]
        on_failure: String,
    },
    /// Edit or view configuration
    Config,
    /// View audit logs
    Logs {
        /// Number of recent log lines to show
        #[arg(long, default_value = "20")]
        tail: usize,
    },
    /// Run system tests
    Test {
        #[command(subcommand)]
        action: Option<TestAction>,
    },
    /// Manage VO1D's memory and learning
    Memory {
        #[command(subcommand)]
        action: Option<MemoryAction>,
    },
}

#[derive(Subcommand)]
enum MemoryAction {
    /// Show memory stats and recent entries
    List,
    /// Show a specific memory by ID
    Show { id: String },
    /// Delete a specific memory by ID
    Delete { id: String },
    /// Clear all memories (or a subset: all, solutions, mistakes, notes, patterns, history)
    Clear {
        #[arg(default_value = "all")]
        memory_type: String,
    },
    /// Add a custom note to memory
    Add {
        content: String,
        #[arg(long, default_value = "")]
        tags: String,
    },
}

#[derive(Subcommand)]
enum ModelAction {
    /// List all available models
    List,
    /// Install a model by ID
    Install { id: String },
    /// Show hardware profile recommendation
    Profile,
    /// Remove a model by ID
    Remove { id: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing
    if cli.debug {
        tracing_subscriber::fmt()
            .with_env_filter("vo1d=debug")
            .with_target(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("vo1d=info")
            .with_target(false)
            .init();
    }

    // Resolve security mode
    let security_mode = resolve_security_mode(&cli);

    // Handle YOLO handshake
    if matches!(security_mode, SecurityMode::Yolo) {
        vo1d::security::yolo_handshake()?;
    }

    // Initialize application context
    let mut ctx = AppContext::new().await?;

    // Override security mode if provided via CLI
    if cli.yolo || cli.mode.is_some() {
        ctx.security.set_mode(security_mode);
    }

    // Override behavioral mode if provided via CLI
    if let Some(ref b) = cli.behavior {
        let behavior_str = match b {
            BehaviorArg::Normal => "normal",
            BehaviorArg::Fix => "fix",
            BehaviorArg::Research => "research",
            BehaviorArg::Refactor => "refactor",
            BehaviorArg::Tdd => "tdd",
        };
        ctx.config.default_behavior = behavior_str.to_string();
    }

    // Override default model if provided via CLI (global --model flag)
    if let Some(ref model_id) = cli.model {
        ctx.config.default_model = model_id.clone();
    }

    // Auto-approve all actions if --yes flag is set
    if cli.yes {
        ctx.auto_approve = true;
    }

    // Override workspace if provided
    if let Some(ws) = cli.workspace {
        std::fs::create_dir_all(&ws)
            .with_context(|| format!("Failed to create workspace directory: {}", ws.display()))?;
    }

    // Handle commands
    match cli.command {
        Some(Commands::Task { task }) => {
            run_task(ctx, &task, cli.resume.as_deref()).await?;
        }
        Some(Commands::Chat { model }) => {
            run_chat(ctx, model.as_deref()).await?;
        }
        Some(Commands::Models { action }) => {
            run_models(ctx, action).await?;
        }
        Some(Commands::Sessions) => {
            run_sessions(ctx).await?;
        }
        Some(Commands::Train { curriculum, manual, all, on_failure }) => {
            if all {
                vo1d::agent::train::run_autotrain_with_policy(ctx, manual, &on_failure).await?;
            } else {
                match curriculum {
                    Some(name) => run_train(ctx, &name, manual).await?,
                    None => list_curricula(ctx).await?,
                }
            }
        }
        Some(Commands::Config) => {
            run_config(ctx).await?;
        }
        Some(Commands::Test { action }) => {
            match action {
                Some(TestAction::Compression) | None => {
                    vo1d::agent::train::run_test_compression(ctx).await?;
                }
                Some(TestAction::Tools) => {
                    vo1d::agent::train::run_test_tools(ctx).await?;
                }
            }
        }
        Some(Commands::Memory { action }) => {
            run_memory(ctx, action).await?;
        }
        Some(Commands::Logs { tail }) => {
            run_logs(ctx, tail).await?;
        }
        None => {
            run_chat(ctx, None).await?;
        }
    }

    Ok(())
}

fn resolve_security_mode(cli: &Cli) -> SecurityMode {
    if cli.yolo {
        return SecurityMode::Yolo;
    }
    match &cli.mode {
        Some(SecurityModeArg::Safe) => SecurityMode::Safe,
        Some(SecurityModeArg::Interactive) => SecurityMode::Interactive,
        Some(SecurityModeArg::PowerUser) => SecurityMode::PowerUser,
        Some(SecurityModeArg::Autonomous) => SecurityMode::Autonomous,
        Some(SecurityModeArg::Yolo) => SecurityMode::Yolo,
        None => SecurityMode::Interactive,
    }
}

async fn run_task(ctx: AppContext, task: &str, resume: Option<&str>) -> Result<()> {
    let output = CliOutput::new();

    if let Some(session_id) = resume {
        output.info(&format!("Resuming session: {}", session_id));
        vo1d::agent::session::resume_session(ctx, &session_id).await?;
    } else {
        output.info("Generating plan...");
        let session = vo1d::agent::session::Session::new(task, &ctx)?;
        let _result = vo1d::agent::run(ctx, session).await?;
    }

    Ok(())
}

async fn run_chat(ctx: AppContext, _model: Option<&str>) -> Result<()> {
    let output = CliOutput::new();
    output.info("Starting interactive chat. Type '/exit' to quit, '/help' for commands.");

    vo1d::ui::cli::interactive_repl(ctx).await?;
    Ok(())
}

async fn run_models(ctx: AppContext, action: Option<ModelAction>) -> Result<()> {
    let output = CliOutput::new();

    match action {
        Some(ModelAction::List) | None => {
            vo1d::ui::cli::list_models(&ctx).await?;
        }
        Some(ModelAction::Install { id }) => {
            output.info(&format!("Installing model: {}...", id));
            vo1d::llm::downloader::download_model(&ctx, &id).await?;
        }
        Some(ModelAction::Profile) => {
            vo1d::ui::cli::show_hardware_profile(&ctx).await?;
        }
        Some(ModelAction::Remove { id }) => {
            output.info(&format!("Removing model: {}...", id));
            vo1d::llm::registry::remove_model(&ctx, &id).await?;
        }
    }
    Ok(())
}

async fn list_curricula(ctx: AppContext) -> Result<()> {
    // Collect disk curricula
    let mut disk_names: Vec<String> = Vec::new();
    if let Some(dir) = find_curriculum_dir(&ctx) {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let fname = entry.file_name().to_string_lossy().to_string();
                if fname.ends_with(".json") {
                    disk_names.push(fname.strip_suffix(".json").unwrap_or(&fname).to_string());
                }
            }
        }
    }

    // Merge with embedded (deduped, embedded wins if no disk file)
    use std::collections::BTreeSet;
    let mut all: BTreeSet<String> = BTreeSet::new();
    for name in vo1d::core::embedded_curricula::list() {
        all.insert(name.to_string());
    }
    for name in &disk_names {
        all.insert(name.clone());
    }

    if all.is_empty() {
        println!("No curricula available.");
        return Ok(());
    }

    println!("Available curricula:");
    for name in &all {
        let desc = if disk_names.contains(name) {
            // Try to read from disk for description
            if let Some(dir) = find_curriculum_dir(&ctx) {
                let path = dir.join(format!("{}.json", name));
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(curriculum) = serde_json::from_str::<serde_json::Value>(&content) {
                        curriculum.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string()
                    } else { String::new() }
                } else { String::new() }
            } else { String::new() }
        } else {
            // Parse from embedded
            vo1d::core::embedded_curricula::get(name)
                .and_then(|json| serde_json::from_str::<serde_json::Value>(json).ok())
                .and_then(|v| v.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()))
                .unwrap_or_default()
        };
        let marker = if disk_names.contains(name) { "" } else { " (embedded)" };
        println!("  {}{}  {}", name, marker, desc);
    }
    Ok(())
}

fn find_curriculum_dir(ctx: &AppContext) -> Option<std::path::PathBuf> {
    let candidates: Vec<std::path::PathBuf> = {
        let mut v = Vec::new();
        // 1. Exe-relative curriculum dir
        v.push(ctx.paths.curriculum_dir());
        // 2. Current working directory
        if let Ok(cwd) = std::env::current_dir() {
            v.push(cwd.join("curriculum"));
        }
        // 3. Parent of exe dir (common when running from target/release)
        if let Some(p) = ctx.paths.curriculum_dir().parent().and_then(|p| p.parent()) {
            v.push(p.join("curriculum"));
        }
        v
    };

    for dir in &candidates {
        if dir.exists() {
            // Only return if it has .json files
            if let Ok(entries) = std::fs::read_dir(dir) {
                let has_json = entries.flatten().any(|e| {
                    e.path().extension().map(|x| x == "json").unwrap_or(false)
                });
                if has_json {
                    return Some(dir.clone());
                }
            }
        }
    }
    None
}

async fn run_train(ctx: AppContext, curriculum: &str, manual: bool) -> Result<()> {
    vo1d::agent::train::run_curriculum_by_name(ctx, curriculum, manual).await
}

async fn run_memory(ctx: AppContext, action: Option<MemoryAction>) -> Result<()> {
    match action {
        Some(MemoryAction::List) | None => {
            let mem = ctx.memory.lock().unwrap();
            let s = mem.stats();
            println!("=== VO1D MEMORY ===\n{}\n", s);

            if !mem.solutions.is_empty() {
                println!("--- SOLUTIONS ---");
                for sol in mem.solutions.iter().rev().take(10) {
                    println!("  [{}] {} → {}", sol.id, sol.task_description, sol.outcome);
                }
            }
            if !mem.mistakes.is_empty() {
                println!("\n--- MISTAKES ---");
                for mist in mem.mistakes.iter().rev().take(10) {
                    println!("  [{}] (freq:{}) {}", mist.id, mist.frequency, mist.mistake);
                }
            }
            if !mem.patterns.is_empty() {
                println!("\n--- PATTERNS ---");
                for p in mem.patterns.iter().filter(|p| p.confidence > 0.5).take(10) {
                    println!("  [conf:{:.0}%] \"{}\"", p.confidence * 100.0, p.suggestion);
                }
            }
            if !mem.notes.is_empty() {
                println!("\n--- NOTES ---");
                for note in mem.notes.iter().rev().take(10) {
                    println!("  [{}] {} (tags: {})", note.id, note.content, note.tags.join(", "));
                }
            }
            if !mem.preferences.is_empty() {
                println!("\n--- PREFERENCES ---");
                for (k, v) in &mem.preferences {
                    if !k.starts_with('_') {
                        println!("  {}: {}", k, v);
                    }
                }
            }
            println!();
        }
        Some(MemoryAction::Show { id }) => {
            let mem = ctx.memory.lock().unwrap();
            if let Some(sol) = mem.solutions.iter().find(|s| s.id == id) {
                println!("=== Solution: {} ===", sol.id);
                println!("Task: {}", sol.task_description);
                println!("Solution: {}", sol.solution);
                println!("Outcome: {}", sol.outcome);
                println!("Tags: {}", sol.tags.join(", "));
                println!("Timestamp: {}", sol.timestamp);
            } else if let Some(mist) = mem.mistakes.iter().find(|m| m.id == id) {
                println!("=== Mistake: {} ===", mist.id);
                println!("Task: {}", mist.task_description);
                println!("Mistake: {}", mist.mistake);
                println!("Lesson: {}", mist.lesson);
                println!("How to avoid: {}", mist.how_to_avoid);
                println!("Frequency: {}", mist.frequency);
                println!("Tags: {}", mist.tags.join(", "));
                println!("Timestamp: {}", mist.timestamp);
            } else if let Some(note) = mem.notes.iter().find(|n| n.id == id) {
                println!("=== Note: {} ===", note.id);
                println!("Content: {}", note.content);
                println!("Tags: {}", note.tags.join(", "));
                println!("Timestamp: {}", note.timestamp);
            } else {
                println!("No memory found with ID: {}", id);
            }
        }
        Some(MemoryAction::Delete { id }) => {
            let mut mem = ctx.memory.lock().unwrap();
            if mem.delete(&id) {
                println!("✓ Deleted memory: {}", id);
            } else {
                println!("No memory found with ID: {}", id);
            }
        }
        Some(MemoryAction::Clear { memory_type }) => {
            let mem_type = if memory_type == "all" { None } else { Some(memory_type.as_str()) };
            let mut mem = ctx.memory.lock().unwrap();
            mem.clear(mem_type);
            println!("✓ Cleared memories ({})", memory_type);
        }
        Some(MemoryAction::Add { content, tags }) => {
            let tag_list: Vec<String> = if tags.is_empty() {
                Vec::new()
            } else {
                tags.split(',').map(|t| t.trim().to_string()).collect()
            };
            let mut mem = ctx.memory.lock().unwrap();
            let id = mem.add_note(&content, tag_list);
            println!("✓ Added note: {}", id);
        }
    }
    Ok(())
}

async fn run_sessions(_ctx: AppContext) -> Result<()> {
    vo1d::agent::session::list_sessions().await?;
    Ok(())
}

async fn run_config(ctx: AppContext) -> Result<()> {
    let config_path = ctx.paths.config_dir().join("settings.toml");
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config: {}", config_path.display()))?;
    println!("{}", content);
    Ok(())
}

async fn run_logs(_ctx: AppContext, _tail: usize) -> Result<()> {
    vo1d::security::audit::tail_logs(_tail).await?;
    Ok(())
}
