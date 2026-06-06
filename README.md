# vo1d

**Local-first autonomous AI execution agent.** vo1d runs local LLMs (llama.cpp) to interpret tasks and execute them via tools — file operations, shell commands, HTTP requests, and more. All inference runs entirely offline on your hardware.

> $\color{orange}{\textbf{\Large WORK IN PROGRESS}}$

## Features

- **Local LLM inference** — Built-in llama.cpp backend, no cloud dependency
- **Autonomous task execution** — ReAct agent loop that plans, acts, and iterates
- **5 security modes** — Safe, Interactive, PowerUser, Autonomous, YOLO
- **Tool system** — File read/write, command execution, directory listing, file search, HTTP requests, and more
- **Audit logging** — Every action is logged with timestamps and security context
- **Hardware profiling** — Auto-detects CPU/GPU/RAM and recommends compatible models
- **Interactive REPL** — Chat-like interface for iterative task execution
- **Session management** — Save, resume, and checkpoint long-running tasks
- **Fully portable** — All data stored relative to the executable; runs from any folder

---

## Project Structure

```
vo1d/
├── Cargo.toml              # Workspace root with feature flags
├── src/
│   ├── main.rs             # CLI entry point (clap parser)
│   ├── lib.rs              # AppContext initialization
│   ├── agent/              # ReAct agent loop
│   │   ├── loop_.rs        # Main agent iteration loop
│   │   ├── parser.rs       # JSON action parser (extracts ```json blocks)
│   │   ├── planner.rs      # High-level plan generation
│   │   ├── executor.rs     # Tool execution dispatch
│   │   ├── session.rs      # Session state, save/resume
│   │   └── checkpoint.rs   # Iteration checkpointing
│   ├── llm/                # LLM backends
│   │   ├── backend.rs      # LlmBackend trait definition
│   │   ├── builtin.rs      # llama.cpp backend (via llama-cpp-2)
│   │   ├── registry.rs     # Model registry (download/cache/lookup)
│   │   └── downloader.rs   # Model download from Hugging Face
│   ├── tools/              # Tool system
│   │   ├── registry.rs     # Tool registry
│   │   ├── read_file.rs    # Read file tool
│   │   ├── write_file.rs   # Write file tool
│   │   ├── execute.rs      # Command execution tool
│   │   ├── list_dir.rs     # Directory listing tool
│   │   ├── search_files.rs # File search tool
│   │   ├── http.rs         # HTTP request tool
│   │   └── helpers.rs      # Path resolution helpers
│   ├── security/           # Security & policy system
│   │   ├── modes.rs        # SecurityMode enum
│   │   ├── policy.rs       # Policy evaluation engine
│   │   ├── approval.rs     # Interactive approval prompts
│   │   ├── audit.rs        # JSONL audit logging
│   │   ├── privilege.rs    # Windows privilege escalation
│   │   └── sandbox.rs      # Workspace sandbox enforcement
│   ├── config/             # Configuration
│   │   ├── settings.rs     # Settings structs with serde defaults
│   │   └── mod.rs          # Load/save settings.toml
│   ├── ui/                 # User interfaces
│   │   └── cli.rs          # CLI REPL and output helpers
│   ├── models/             # Data models
│   │   ├── message.rs      # Message, LlmResponse, TokenUsage
│   │   ├── action.rs       # Action enum (all tool actions)
│   │   └── tool.rs         # Tool definition
│   ├── core/               # Core utilities
│   │   ├── paths.rs        # Vo1dPaths (portable directory resolver)
│   │   └── hardware.rs     # Hardware profiling (CPU/GPU/RAM)
│   └── utils/              # Misc utilities
│       ├── crypto.rs       # SHA256 hashing
│       ├── time.rs         # Timestamp formatting
│       └── stderr_guard.rs # stderr suppression for llama.cpp
```

---

## Installation

### Prerequisites

- **Rust 1.75+**
- **CMake** (for llama.cpp build)
- **C++ build tools** (MSVC on Windows, GCC/Clang on Linux/macOS)

### Build

```bash
# Clone the repo
git clone https://github.com/your-username/vo1d.git
cd vo1d

# Build with built-in llama.cpp (recommended for local-only use)
cargo build --features llamacpp-builtin --release

# Or with Vulkan GPU acceleration
cargo build --features vulkan --release

# Or with all features (Ollama, LM Studio, etc.)
cargo build --features full --release
```

### Feature Flags

| Flag | Description |
|------|-------------|
| `default` | No backends enabled (CLI only) |
| `llamacpp-builtin` | Built-in llama.cpp inference via `llama-cpp-2` |
| `vulkan` | `llamacpp-builtin` + Vulkan GPU acceleration |
| `ollama` | Ollama API backend |
| `lmstudio` | LM Studio API backend |
| `llamacpp-server` | llama.cpp server backend |
| `custom-api` | Custom OpenAI-compatible API backend |
| `full` | All backends |

### Install a Model

```bash
# List available models
vo1d models

# Install a recommended model for your hardware
vo1d models install qwen25_1.5b

# Show hardware profile and compatible models
vo1d models profile
```

Models are downloaded from Hugging Face and stored in `models/llamacpp/` next to the binary.

---

## Quick Start

```bash
# Start interactive chat (default mode)
vo1d

# Run a one-shot task
vo1d task "list all files in the workspace"

# Run with autonomous mode (auto-approve actions)
vo1d task "create a python script and run it" --mode autonomous

# Enable verbose debug output
vo1d task "search for todos in the codebase" --debug

# Resume a previous session
vo1d --resume <session-id>
```

---

## How It Works

vo1d uses a **ReAct** (Reasoning + Acting) agent loop:

1. **System prompt** is constructed with current mode, workspace path, and available tools
2. **User task** is appended as a message
3. **Model generates** a response containing a JSON action block
4. **Parser extracts** the JSON action (e.g. `{"action": "write_file", ...}`)
5. **Security policy** evaluates the action against the current mode
6. **Action is executed** and the result (or error) is appended to the conversation
7. **Loop repeats** with the updated conversation until `finish` is called or max iterations reached

```
 User input → [System Prompt + Conversation] → LLM → JSON Action
                                                       ↓
                                              Security Policy
                                                       ↓
                                              Tool Executor → Result
                                                       ↓
                                              Append to Conversation
                                                       ↓
                                              Loop or Finish
```

---

## Available Tools

The model communicates actions via JSON blocks enclosed in `` ```json ```. The parser extracts the first valid JSON block from the model output.

### Read File
```json
{
  "action": "read_file",
  "path": "relative/path/to/file.txt",
  "start_line": 1,
  "end_line": 50
}
```
Fields `start_line` and `end_line` are optional.

### Write File
```json
{
  "action": "write_file",
  "path": "output.py",
  "content": "print('hello world')",
  "append": false
}
```
Set `append: true` to append instead of overwrite.

### Execute Command
```json
{
  "action": "execute_command",
  "command": "dir /b",
  "timeout": 30,
  "workdir": "some/subdir"
}
```
`timeout` defaults to 60s. `workdir` defaults to workspace root.

### List Directory
```json
{
  "action": "list_directory",
  "path": ".",
  "pattern": "*.rs"
}
```
`pattern` is optional (glob filter).

### Search Files
```json
{
  "action": "search_files",
  "pattern": "**/*.toml",
  "path": ".",
  "type": "file"
}
```
Uses recursive glob matching.

### Delete File
```json
{
  "action": "delete_file",
  "path": "old-file.txt"
}
```

### Copy File
```json
{
  "action": "copy_file",
  "source": "a.txt",
  "destination": "b.txt"
}
```

### Create Directory
```json
{
  "action": "create_directory",
  "path": "new-folder"
}
```

### File Metadata
```json
{
  "action": "file_metadata",
  "path": "some-file.txt"
}
```

### HTTP Request
```json
{
  "action": "http_request",
  "url": "https://api.example.com/data",
  "method": "GET",
  "headers": { "Authorization": "Bearer token" },
  "body": "{\"key\": \"value\"}"
}
```
`method` defaults to `GET`. `headers` and `body` are optional.

### Finish (Task Complete)
```json
{
  "action": "finish",
  "output": "Summary of completed work."
}
```
Ends the agent loop.

### Ask User
```json
{
  "action": "ask_user",
  "question": "Should I proceed with this operation?"
}
```
Prompts the user for approval.

---

## Security Modes

| Mode | Writes | Commands | Outside Workspace | System Mods | Elevation |
|------|--------|----------|-------------------|-------------|-----------|
| **Safe** | ❌ Blocked | ❌ Blocked | ❌ Blocked | ❌ Blocked | ❌ Blocked |
| **Interactive** (default) | ✅ Ask | ✅ Ask | ✅ Ask | ❌ Blocked | ❌ Blocked |
| **PowerUser** | ✅ Ask | ✅ Ask | ✅ Ask | ✅ Ask | ❌ Blocked |
| **Autonomous** | ✅ Auto | ✅ Auto | ✅ Ask | ❌ Blocked | ❌ Blocked |
| **YOLO** | ✅ Auto | ✅ Auto | ✅ Auto | ✅ Auto | ✅ Auto |

Modes are set via the `--mode` flag or configured in `settings.toml`.

---

## Configuration

vo1d creates `config/settings.toml` next to the binary on first run:

```toml
[llm]
backend = "builtin"

[llm.builtin]
context_size = 4096
batch_size = 4096
threads = -1              # -1 = auto-detect CPU core count
gpu_layers = 0            # 0 = CPU only, -1 = all layers
temperature = 0.7
top_p = 0.9
top_k = 40
repeat_penalty = 1.1
max_tokens = 2048

[security]
require_workspace_write_approval = true

[llm.custom_api]
base_url = ""
api_key = ""
model_name = ""
```

---

## Session System

Every task execution creates a session with:
- A unique UUID session ID
- Timestamp, task description, and security mode
- Full iteration history in the conversation

### Commands

```bash
# List all saved sessions
vo1d sessions

# Resume a session by ID
vo1d --resume <session-id> task "continue from where we left off"

# Sessions are stored in sessions/<uuid>/
```

### Checkpointing

Every 5 iterations, the session state is saved to `sessions/<uuid>/checkpoints/`. If the process is interrupted, you can resume from the last checkpoint.

---

## Audit Logging

All actions are logged to JSONL files in `logs/`:

```
logs/
├── audit_YYYY-MM-DD.jsonl      # All actions
├── yolo_audit_YYYY-MM-DD.jsonl  # YOLO mode actions (extra detail)
└── errors.jsonl                 # Error events
```

Each audit entry includes:
- Timestamp
- Action type and description
- Security mode at time of action
- Whether the action was approved/denied
- Model and session context

---

## CLI Reference

```
Usage: vo1d [OPTIONS] [COMMAND]

Commands:
  task      Execute a one-shot task
  chat      Start an interactive chat session
  models    List and manage models
  sessions  List and manage saved sessions
  config    View current configuration
  logs      View audit logs

Options:
      --model <MODEL>       Override default model
      --mode <MODE>         Security mode
                              (safe, interactive, power-user, autonomous, yolo)
      --workspace <DIR>     Custom workspace directory
      --yolo                Enable YOLO mode (implies --mode yolo)
      --yes                 Auto-approve all actions (Interactive/PowerUser only)
      --debug               Enable verbose debug tracing
      --resume <ID>         Resume a session by ID
  -h, --help                Print help
  -V, --version             Print version

Subcommands:
  models list               List all available models
  models install <id>       Download and install a model
  models remove <id>        Remove an installed model
  models profile            Show hardware profile and compatible models
```

---

## Hardware Profiling

vo1d automatically profiles your hardware on startup:

```bash
vo1d models profile

=== Hardware Profile ===
CPU: Intel(R) Core(TM) i5-7300U CPU @ 2.60GHz (4 cores)
RAM: 7.9 GB total, 5.6 GB available
Tier: Low
Recommended Max Context: 4096 tokens

GPU(s):
  Intel(R) HD Graphics 620 (Intel) - No dedicated VRAM

Compatible Models:
  [✓] qwen25_1.5b (Qwen2.5 1.5B Instruct)
  [✗] qwen25_3b (Qwen2.5 3B Instruct)
  [✗] qwen25_7b (Qwen2.5 7B Instruct)
  ... and 3 more compatible models
```

The profiler checks:
- CPU model and core count
- Total and available RAM
- GPU vendor, name, and dedicated VRAM
- Overall hardware tier (Low / Medium / High)
- Compatible models from the registry

---

## Architecture

```
                        ┌──────────────┐
                        │     CLI      │
                        └──────┬───────┘
                               │
                        ┌──────▼───────┐
                        │  Agent Loop  │
                        │  (ReAct)     │
                        └──┬───────┬───┘
                           │       │
                    ┌──────▼──┐ ┌──▼──────────┐
                    │  LLM   │ │  Tool System │
                    │ Backend │ │  (registry)  │
                    └─────────┘ └──┬───────────┘
                                   │
                        ┌──────────▼──────────┐
                        │  Tool Executors     │
                        │ (file, cmd, http…)  │
                        └─────────────────────┘
```

### Component Details

- **CLI**: Entry point. Clap argument parser dispatches to `vo1d task` or `vo1d chat`.
- **AppContext**: Shared runtime state — config, paths, hardware profile, security manager, audit logger, model registry. Cloned per task.
- **Agent Loop**: Iterates up to `max_iterations`. Each iteration: model generates response → parser extracts JSON action → security evaluates → executor runs → result appended to conversation.
- **LLM Backend**: Trait with `chat()` and `stream_chat()`. Current implementations: `builtin` (llama.cpp via `llama-cpp-2`). Backends are pluggable.
- **Tool System**: Registry of available tools. Each tool is a function that takes an `Action` and returns a `Result<String>`.
- **Security Manager**: Evaluates each action against the current mode. Can approve, ask, or block. All decisions are audited.

---

## Portability

vo1d is designed to be fully portable:

- All data (config, models, sessions, logs) is stored relative to the executable
- No registry entries, no system-wide install paths
- Runs from any folder, USB drive, or network share
- Config path: `<exe-dir>/config/settings.toml`
- Model path: `<exe-dir>/models/llamacpp/`
- Session path: `<exe-dir>/sessions/<uuid>/`
- Log path: `<exe-dir>/logs/`

---

## Development

### Running Tests

```bash
# Run all unit tests
cargo test

# Run tests with a specific feature
cargo test --features llamacpp-builtin
```

### Code Style

- Follow existing patterns (see `src/` structure)
- `serde` derives for all data structs
- `anyhow::Result` for error propagation
- `tracing` for logging (not `println`/`eprintln` in library code)
- CLI output uses `print!`/`eprintln!` (only in `loop_.rs`, guarded by `session.tui_mode`)
- No unsafe code unless absolutely necessary (see `stderr_guard.rs`)

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `llama_context: n_batch = 512` then crash | Delete `<exe-dir>/config/settings.toml` or set `batch_size = 4096` |
| `Model file not found` | Run `vo1d models install <model-id>` |
| `Failed to load model at startup` | The model file is missing or corrupt. Reinstall with `vo1d models install <model-id>` |
| `spawn_blocking panicked` in `builtin.rs` | Usually a model crash. Check RAM usage and reduce `context_size` or `gpu_layers` |
| Build fails with linker errors on Windows | You need MSVC build tools (Visual Studio Build Tools or Visual Studio). The `+crt-static` flag must NOT be in `.cargo/config.toml` |

---

## Model Registry

vo1d includes a built-in catalog of compatible models. The catalog is defined in `config/default_models.toml`.

### Adding a Custom Model

1. Edit `config/default_models.toml` (or create `config/models.toml`)
2. Add an entry:
```toml
[[model]]
id = "my-model"
name = "My Custom Model"
provider = "huggingface"
download_url = "https://huggingface.co/author/model/resolve/main/model.gguf"
filename = "model.gguf"
sha256 = "abcdef12345..."
size_bytes = 2000000000
min_ram_gb = 4.0
context_length = 4096
supports_tools = false
quantization = "Q4_K_M"
```

---

## License

GNU General Public License v3.0
