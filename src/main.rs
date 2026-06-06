use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use vo1d::security::SecurityMode;
use vo1d::ui::cli::CliOutput;
use vo1d::AppContext;

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

    /// Set custom workspace directory
    #[arg(long, global = true)]
    workspace: Option<PathBuf>,

    /// Enable YOLO mode (absolute autonomy)
    #[arg(long, global = true)]
    yolo: bool,

    /// Auto-approve all actions (Interactive/Power User only)
    #[arg(long, global = true)]
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
        curriculum: String,
    },
    /// Edit or view configuration
    Config,
    /// View audit logs
    Logs {
        /// Number of recent log lines to show
        #[arg(long, default_value = "20")]
        tail: usize,
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
        Some(Commands::Train { curriculum }) => {
            run_train(ctx, &curriculum).await?;
        }
        Some(Commands::Config) => {
            run_config(ctx).await?;
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

async fn run_train(ctx: AppContext, curriculum: &str) -> Result<()> {
    // Resolve curriculum path: check if it's a built-in name first
    let curriculum_path = if !curriculum.contains('\\') && !curriculum.contains('/') && !curriculum.contains('.') {
        let builtin = ctx.paths.curriculum_dir().join(format!("{}.json", curriculum));
        if builtin.exists() {
            builtin
        } else {
            // Try with leading digits prefix
            let entries = std::fs::read_dir(ctx.paths.curriculum_dir())?;
            let mut found = None;
            for entry in entries {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().to_string();
                if name.contains(curriculum) && name.ends_with(".json") {
                    found = Some(entry.path());
                    break;
                }
            }
            match found {
                Some(p) => p,
                None => anyhow::bail!("Curriculum '{}' not found in {}", curriculum, ctx.paths.curriculum_dir().display()),
            }
        }
    } else {
        std::path::PathBuf::from(curriculum)
    };

    if !curriculum_path.exists() {
        anyhow::bail!("Curriculum file not found: {}", curriculum_path.display());
    }

    vo1d::agent::train::run_curriculum(ctx, &curriculum_path.to_string_lossy()).await?;
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
