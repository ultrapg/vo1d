# vo1d

**Local-first autonomous AI execution agent.** vo1d runs local LLMs (llama.cpp) to interpret tasks and execute them via tools — file operations, shell commands, HTTP requests, and more. All inference runs entirely offline on your hardware.

> **WORK IN PROGRESS**

## Features

- **Local LLM inference** — Built-in llama.cpp backend, no cloud dependency
- **Autonomous task execution** — ReAct agent loop that plans, acts, and iterates
- **5 security modes** — Safe, Interactive, PowerUser, Autonomous, YOLO
- **Web tools** — Web search (DuckDuckGo) and web page fetching (HTML→markdown conversion)
- **File & shell tools** — Complete file operations, command execution, directory management, HTTP requests
- **Context compression** — Opencode-style two-phase pruning & compacting (prune oversized tool outputs + compact old messages)
- **Training curriculum** — Progressive learning system with 6 curriculum files and task evaluation
- **Self-correction system** — Failure tracking, error classification with tailored suggestions, auto-correction prompts after repeated failures
- **Documentation-driven system prompt** — Markdown docs loaded at startup and injected into system prompt for better tool usage guidance
- **Learning memory system** — Cross-session memory that learns from successes and mistakes, stores solutions for future recall, and finds relevant past experiences by keyword similarity
- **Self-improvement through training** — Training stores successful solutions and records mistakes as lessons; future tasks receive relevant past experiences injected into the prompt
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
│   │   ├── checkpoint.rs   # Iteration checkpointing
│   │   └── train.rs        # Training curriculum system
│   ├── core/               # Core utilities
│   │   ├── paths.rs        # Vo1dPaths (portable directory resolver)
│   │   ├── hardware.rs     # Hardware profiling (CPU/GPU/RAM)
│   │   ├── memory.rs       # Cross-session memory store
│   │   ├── compression.rs  # Context compression (prune+compact)
│   │   ├── curriculum.rs   # Curriculum evaluation & loading
│   │   ├── docs.rs         # Markdown documentation provider for system prompt
│   │   ├── self_correction.rs   # FailureTracker & ErrorClassifier
│   │   └── error_suggestions.rs # Detailed error analysis with markdown suggestions
│   ├── llm/                # LLM backends
│   ├── tools/              # Tool system
│   │   ├── mod.rs          # Module exports
│   │   ├── registry.rs     # Tool registry (tool metadata)
│   │   ├── files.rs        # File operations
│   │   ├── shell.rs        # Shell command execution
│   │   ├── web.rs          # HTTP request tool
│   │   ├── web_search.rs   # Web search (DuckDuckGo)
│   │   ├── web_fetch.rs    # Web page fetch & HTML→markdown
│   │   └── schema.rs       # JSON schema helpers
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

# Build with web tools (requires websearch and html2md crates)
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

**Web Tools Support:**
- `websearch = "0.1"` and `html2md = "0.2"` dependencies enable web search and web fetch tools
- Web search: DuckDuckGo provider (no API key required)
- Web fetch: HTTP requests with HTML-to-markdown conversion

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

# Train on a curriculum (progressive learning)
vo1d train 00_hello_world
vo1d train curriculum/my_custom_curriculum.json
```

---

## How It Works

vo1d uses a **ReAct** (Reasoning + Acting) agent loop with **context compression**:

1. **System prompt** is constructed with current mode, workspace path, memory context, available tools, **and markdown documentation** (loaded from `docs/`)
2. **User task** is appended as a message (ChatML format: `<|im_start|>system` / `<|im_start|>user` / `<|im_start|>assistant`)
3. **Model generates** a response containing a JSON action block
4. **Parser extracts** the JSON action (e.g. `{"action": "web_search", "query": "rust programming"}`) from plain JSON, markdown code blocks, or heuristic patterns
5. **Security policy** evaluates the action against the current mode
6. **Action is executed** and the result (or error) is appended to the conversation
7. **Self-correction** checks for repeated failures per action type (≥3 consecutive → auto-correction prompt injected in next iteration)
8. **Context compression** occurs if conversation exceeds ~80% of context limit:
   - **Phase 1 (Prune)**: Truncate oversized tool outputs (>2000 chars) with length markers
   - **Phase 2 (Compact)**: Keep system prompt, user task, and recent messages; compress old sections into summary markers
9. **Loop repeats** with the updated conversation until `finish` is called or max iterations reached

```
  User input → [System Prompt + Conversation] → LLM → JSON Action
                                                        ↓
                                               Security Policy
                                                        ↓
                                               Tool Executor → Result
                                                        ↓
                                               Append to Conversation
                                                        ↓
                                               Context Compression (if needed)
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
Batch delete by glob pattern (e.g. `delete all txts`):
```json
{
  "action": "delete_file",
  "path": ".",
  "pattern": "*.txt"
}
```
`path` defaults to `"."` when `pattern` is set. `delete_files` is accepted as an alias for `delete_file`.

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

### Web Search
```json
{
  "action": "web_search",
  "query": "rust programming language features",
  "num_results": 5
}
```
Searches the web using DuckDuckGo (no API key required). `num_results` defaults to 5 (max 10).

### Web Fetch
```json
{
  "action": "web_fetch",
  "url": "https://example.com",
  "max_chars": 5000
}
```
Fetches a web page and converts HTML to markdown. `max_chars` defaults to 8000 (max 50000).

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

## Context Compression

To handle long conversations within limited context windows, vo1d implements **opencode-style context compression** that activates when the conversation exceeds ~80% of the model's context limit.

### Compression Strategy

1. **Two-Phase Process**:
   - **Phase 1 (Prune)**: Truncate oversized tool outputs (>2000 characters)
   - **Phase 2 (Compact)**: Apply sliding window to keep critical messages

2. **Priority-Based Message Selection**:
   - **Critical**: System prompt (always kept)
   - **High**: User's original task (always kept)
   - **Normal**: Recent assistant/user messages (last 8+ messages)
   - **Low**: Recent tool results (truncated if long)
   - **Background**: Old messages (compressed into summaries)

3. **Memory Integration**: Compressed sections are summarized and stored in the memory system for future reference.

### Configuration

Compression settings are configurable in `settings.toml`:
```toml
[llm.builtin]
context_size = 4096
# Compression happens at ~80% usage (3276 tokens)
```

## Train Mode

Train mode provides a structured learning environment where vo1d works through progressive curriculum of tasks. Each curriculum consists of a JSON file with sequential tasks, evaluation criteria, and memory accumulation.

### Using Train Mode

```bash
# List available built-in curricula
vo1d train

# Run a built-in curriculum
vo1d train 00_hello_world           # Basic file operations
vo1d train 01_file_ops            # File read/write/append/copy
vo1d train 02_directory_ops       # Directory operations
vo1d train 03_search_nav          # File search and navigation
vo1d train 04_shell_basics        # Shell command basics
vo1d train 05_web_basics          # Web search and fetch

# Manual mode — complete tasks yourself without an LLM model
vo1d train 00_hello_world --manual

# Use a custom curriculum file
vo1d train my_curriculum.json
```

### Curriculum Format

Curricula are JSON files with this structure:
```json
{
  "name": "Hello World — File Creation",
  "description": "Learn to create text files",
  "tasks": [
    {
      "id": "create_hello_txt",
      "description": "Create a file named hello.txt with content 'Hello, World!'",
      "expected_outcome": "hello.txt exists with content 'Hello, World!'",
      "evaluation": {
        "check_file_exists": ["hello.txt"],
        "check_file_content": ["hello.txt::Hello, World!"],
        "check_directory_exists": ["src", "tests"],
        "check_command_output": ["dir /b::hello.txt"]
      }
    }
  ]
}
```

### Task Evaluation

Each task can have evaluation criteria (all fields accept arrays for multiple checks):
- `check_file_exists`: Files that must exist at specified paths
- `check_file_content`: File content checks in `"path::expected text"` format
- `check_directory_exists`: Directories that must exist at specified paths
- `check_command_output`: Command output checks in `"command::expected text"` format

### Training Features

- **Manual mode** (`--manual`): Complete tasks yourself without an LLM model — the system prints each task, waits for you to create the files, then evaluates
- **Sandboxed execution**: Each task runs in a clean sandbox directory
- **Memory accumulation**: Task outcomes are stored in memory across the curriculum
- **Solution storage**: Successful completions stored as solutions with keyword tags for future recall
- **Mistake learning**: Failures recorded as mistakes with lessons and avoidance strategies
- **Similar experience recall**: Before each task, memory is searched for similar past problems and their solutions/lessons are injected into the prompt
- **Progress tracking**: Shows task-by-task progress with success/failure indicators
- **Self-improvement**: Successful patterns are stored as learned patterns in memory, boosted on repeat success
- **Detailed feedback**: Shows specific evaluation results for each task

### Built-in Curricula

| Curriculum | Description | Tasks |
|-----------|-------------|-------|
| `00_hello_world` | Basic file creation | 3 tasks |
| `01_file_ops` | File operations (read, write, append, copy) | 3 tasks |
| `02_directory_ops` | Directory operations and listing | 3 tasks |
| `03_search_nav` | File search and navigation | 3 tasks |
| `04_shell_basics` | Shell command usage | 3 tasks |
| `05_web_basics` | Web search and fetch | 2 tasks |

### Memory & Learning Integration

Train mode uses the full learning memory system:
- **Solutions**: Successful task completions are stored with extracted keyword tags for future recall
- **Mistakes**: Failures are recorded with what went wrong, the lesson learned, and how to avoid it
- **Similar past experiences**: Before each task, the system searches memory for similar past problems and injects their solutions and lessons into the prompt
- **Patterns**: Successful patterns are stored with confidence scores that increase on repeated success
- **Task history**: Outcome and execution time tracked across the curriculum

---

## Self-Correction

vo1d includes a multi-layered self-correction system that helps the model recover from errors autonomously and learns from them in persistent memory.

### In-Memory Learning from Failures

When an action fails 2+ times consecutively during execution, the error is automatically recorded as a **mistake** in persistent memory:
- Stores what went wrong (action type + error message)
- Stores the corrective suggestion
- Tracks frequency so recurring issues are prioritized in the memory summary

These learned mistakes are then surfaced via **similar past experience recall** during training and future tasks.

### Failure Tracking

The `FailureTracker` records consecutive failures per action type. When an action fails 3 times in a row, the agent loop injects a correction prompt on the next iteration:

```
The action "read_file" has failed 3 times in a row. Previous errors:
• File not found: missing.txt
• File not found: missing.txt
• File not found: missing.txt

Suggestions:
- Check that the file path is correct and relative to the workspace
- Use list_directory to verify the file exists before reading
- Try an alternative path or glob pattern
```

### Error Classification

The `ErrorClassifier` categorizes errors (file not found, permission denied, timeout, etc.) and provides targeted suggestions. These are used both in correction prompts and in the error output shown to the model after each failed action.

### Detailed Error Suggestions

When `format_with_suggestion()` is called on an error, it delegates to `error_suggestions::analyze_error()` which produces rich markdown output:

```markdown
### File Not Found
**Error:** No such file or directory: 'missing.txt'

**Likely Cause:** The specified file path does not exist.

**Suggested Fix:**
- Use `list_directory` to verify the file exists
- Check that the path is relative to the workspace root
- Verify the filename spelling and extension

**Prevention:**
- Use `search_files` with a glob pattern before reading
- Confirm the workspace path with `list_directory`
```

### Documentation-Driven System Prompt

The `DocProvider` loads markdown files from the `docs/` directory at startup and injects them into the system prompt. This provides the model with detailed reference documentation for tool usage, file operations, and self-improvement strategies — without hardcoding everything into the prompt template.

Current docs:
- `docs/file-ops.md` — File read/write/append/copy/delete patterns
- `docs/directory-ops.md` — Directory creation, listing, navigation
- `docs/self-improvement.md` — Learning from errors, retry strategies, memory usage

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

## Memory System

vo1d features a **learning memory system** that persists across sessions and improves over time.

### What Is Stored

| Store | Description | File |
|-------|-------------|------|
| **Task History** | Every task executed, with actions and outcome | `memory/task_history.json` |
| **Patterns** | Learned patterns with confidence scores (boosted on reuse, capped at 50) | `memory/patterns.json` |
| **Solutions** | Successful task solutions with keyword tags for similarity matching | `memory/solutions.json` |
| **Mistakes** | Recorded failures with lessons learned and avoidance strategies (frequency-tracked) | `memory/mistakes.json` |
| **Notes** | User-curated notes with tags | `memory/notes.json` |
| **Preferences** | Key-value settings learned from user interaction | `memory/preferences.json` |

### How Learning Works

1. **During training**: Each task result is analyzed — successes are stored as solutions, failures as mistakes with lessons (e.g., "file not found" → "check the path exists before writing")
2. **During execution**: If an action fails 2+ times consecutively, the error is automatically recorded as a mistake in persistent memory
3. **During recall**: Before starting a task, the system searches stored solutions, mistakes, and notes for keyword overlap with the current task. Top matches are injected into the system prompt
4. **Pattern reinforcement**: Repeated successes boost confidence scores on stored patterns; high-confidence patterns are shown in the memory summary

### CLI Commands

```bash
# Show memory stats and recent entries
vo1d memory
vo1d memory list

# View full detail of a specific memory by ID
vo1d memory show sol_1718000000_123456789

# Delete a specific memory by ID
vo1d memory delete sol_1718000000_123456789

# Clear all memories
vo1d memory clear

# Clear only a subset
vo1d memory clear solutions
vo1d memory clear mistakes
vo1d memory clear notes
vo1d memory clear patterns
vo1d memory clear history

# Add a custom note with optional tags
vo1d memory add "Remember to check file paths before writing" --tags "files,writing,best-practice"
```

### Interactive REPL

In chat mode, use `/memory` to view current memory stats and recent entries:

```
VO1D [interactive] >> /memory
=== MEMORY ===
Tasks: 12 | Patterns: 3 | Solutions: 5 | Mistakes: 2 | Notes: 1 | Preferences: 0

Recent solutions:
  Create hello.txt → passed
  Write greeting.txt → passed

Learned mistakes:
  (freq:2) FAIL: File not found: data.txt
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
  train     Run a training curriculum (e.g. `vo1d train 00_hello_world`)
  memory    Manage VO1D's memory and learning
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
  memory list               Show memory stats and entries
  memory show <id>          Show a specific memory by ID
  memory delete <id>        Delete a specific memory by ID
  memory clear [type]       Clear memories (all, solutions, mistakes, notes, patterns, history)
  memory add <content>      Add a custom note (--tags for comma-separated tags)
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
                        ┌──────▼──────────────┐
                        │    Agent Loop       │
                        │  (ReAct + Compress  │
                        │   + Self-Correct)   │
                        └──┬───────┬──────┬───┘
                           │       │      │
                    ┌──────▼──┐ ┌──▼──────▼──┐
                    │  LLM   │ │  Tool      │
                    │ Backend │ │  System    │
                    └─────────┘ │  (registry)│
                               └──┬─────────┘
                                  │
                       ┌──────────▼──────────┐
                       │  Tool Executors     │
                       │ (file, cmd, http,   │
                       │  web_search,        │
                       │  web_fetch)         │
                       └─────────────────────┘
```

### Component Details

- **CLI**: Entry point. Clap argument parser dispatches to `vo1d task`, `vo1d chat`, or `vo1d train`.
- **AppContext**: Shared runtime state — config, paths, hardware profile, security manager, audit logger, model registry, doc provider. Cloned per task.
- **Agent Loop**: Iterates up to `max_iterations`. Each iteration: model generates response → parser extracts JSON action → security evaluates → executor runs → result appended → self-correction checks → context compression if needed.
- **LLM Backend**: Trait with `chat()` and `stream_chat()`. Current implementations: `builtin` (llama.cpp via `llama-cpp-2`). Backends are pluggable.
- **Tool System**: Registry of 14 available tools — file operations, shell commands, directory management, HTTP requests, web search (DuckDuckGo), web fetch (HTML→markdown).
- **Security Manager**: Evaluates each action against the current mode. Can approve, ask, or block. All decisions are audited.
- **Doc Provider**: Loads markdown documentation from `docs/` and injects into system prompt for better tool usage guidance.
- **Self-Correction**: `FailureTracker` monitors consecutive failures per action type; `ErrorClassifier` and `error_suggestions` produce detailed markdown suggestions.
- **Context Compressor**: Two-phase pruning + compacting when conversation exceeds ~80% of context limit.

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

# Run integration tests
cargo test --test integration

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
| Model keeps generating infinite tool calls | The model may not be receiving the correct ChatML prompt. Check `add_bos_token` matches the model metadata. Qwen2.5 expects `<|im_start|>` tags |
| LLM output is garbled or mixed with C library stderr | llama.cpp writes to stderr via CRT `fprintf`. vo1d suppresses this with `_dup2` at the CRT file-descriptor level; verify `stderr_guard.rs` compiled correctly |
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
