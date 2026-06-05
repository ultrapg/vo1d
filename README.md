# VO1D

**Local-first autonomous AI execution agent.** VO1D runs local LLMs (llama.cpp) to interpret tasks and execute them via tools — file operations, shell commands, HTTP requests, and more. All inference runs entirely offline on your hardware.

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

# Or with all features (Ollama, LM Studio, TUI, etc.)
cargo build --features full --release
```

### Install a Model

```bash
# List available models
vo1d models

# Install a recommended model for your hardware
vo1d models install qwen25_1.5b

# Show hardware profile and compatible models
vo1d models profile
```

## Quick Start

```bash
# Start interactive chat
vo1d

# Run a one-shot task
vo1d task "list all files in the workspace"

# Run with autonomous mode (auto-approve actions)
vo1d task "create a python script and run it" --mode autonomous

# Enable verbose output
vo1d task "search for todos in the codebase" --debug
```

## Security Modes

| Mode | Description |
|------|-------------|
| **Safe** | Read-only access to workspace. No commands, no writes. |
| **Interactive** (default) | Prompts for approval on writes, commands, and outside-workspace access. |
| **PowerUser** | Full system access with approval prompts. |
| **Autonomous** | Auto-approves all actions except privilege escalation. |
| **YOLO** | Auto-approves everything, including privilege escalation. Requires explicit `--yolo` flag and confirmation. |

## Configuration

VO1D creates a `config/settings.toml` next to the binary on first run. You can edit it to change:

```toml
[llm]
backend = "builtin"

[llm.builtin]
context_size = 4096
batch_size = 4096
threads = -1          # -1 = auto
gpu_layers = 0        # 0 = CPU only
temperature = 0.7
max_tokens = 2048
```

## Models

VO1D automatically downloads models from Hugging Face via direct links. Compatible models are shown via `vo1d models profile`.

## Architecture

```
                        ┌──────────────┐
                        │    CLI/TUI   │
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
      --model <MODEL>    Override default model
      --mode <MODE>      Security mode (safe, interactive, power-user, autonomous, yolo)
      --workspace <DIR>  Custom workspace directory
      --yolo             Enable YOLO mode (absolute autonomy)
      --yes              Auto-approve all actions
      --debug            Enable verbose debug tracing
      --resume <ID>      Resume a session by ID
  -h, --help             Print help
  -V, --version          Print version
```

## License

GNU General Public License v3.0
