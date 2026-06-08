# vo1d

**Local-first autonomous AI execution agent.** vo1d runs local LLMs (llama.cpp with optional Vulkan GPU acceleration) to interpret tasks and execute them via tools — file operations, shell commands, HTTP requests, web search, plan management, reusable skills, and more. All inference runs entirely offline on your hardware. Downloads are verified via SHA256 checksums fetched from Hugging Face API.

## Features

- **Local LLM inference** — Built-in llama.cpp backend, no cloud dependency
- **Native tool calling** — Models with structured function calling support (Qwen 2.5/3, Llama 3.1+, Gemma 3/4, Mistral, etc.) receive inline tool definitions and their JSON tool call responses are parsed directly; fallback to text-based ````json```` extraction for all others
- **Autonomous task execution** — ReAct agent loop that plans, acts, and iterates
- **5 security modes** — Safe, Interactive, PowerUser, Autonomous, YOLO
- **24 built-in tools** — File operations, shell commands, web tools, change tracking, restore, plan management, and skill authoring
- **Skill system** — Create, store, invoke, list, and delete reusable multi-step procedures. Skills persist as JSON files and support parameter schemas, making common workflows (setup project, run tests, deploy) repeatable with one command
- **Plan tools** — `plan_create`, `plan_step_complete`, `plan_step_fail`, `plan_status` for structured multi-step task execution with DAG dependency ordering and auto-advancement
- **Web tools** — Web search (DuckDuckGo) and web page fetching (HTML→markdown conversion)
- **File & shell tools** — Complete file operations, command execution, directory management, HTTP requests
- **Context compression** — Two-phase pruning & compacting at >100% usage. At >60% usage, uses LLM summarization to preserve information intelligently. Compression triggers early for proactive window management
- **Training curriculum** — Progressive learning system with 13 built-in curricula, real-world testing environments, and autotrain mode (`vo1d train --all`)
- **Self-correction system** — Failure tracking, error classification with tailored suggestions, auto-correction prompts after repeated failures
- **Documentation-driven system prompt** — 10 markdown docs loaded at startup and injected into system prompt for tool usage guidance
- **Learning memory system** — Cross-session memory that learns from successes and mistakes, stores solutions with action sequences for future recall, and finds relevant past experiences by keyword similarity
- **Built-in planner** — Model can create plans, complete/fail steps, and check status via dedicated plan tools (`plan_create`, `plan_step_complete`, `plan_step_fail`, `plan_status`). PLAN.md workflow also supported as fallback
- **Behavior modes** — Normal, Fix, Research, Refactor, and TDD modes each enforce read-only phases, plan requirements, and mode-specific system prompt notes
- **Limitless iterations** — No hard iteration cap; contexts stay manageable via intelligent compression. If the model repeats the same action 5+ times, the iteration restarts automatically
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
├── config/
│   └── default_models.toml # Model registry catalog
├── curriculum/             # 13 JSON curriculum files (also embedded in binary)
├── docs/                   # 10 markdown docs injected into system prompt
└── src/
    ├── main.rs             # CLI entry point (clap parser)
    ├── lib.rs              # AppContext initialization, module exports
    ├── agent/              # ReAct agent loop
    │   ├── loop_.rs        # Main agent iteration loop (plan/skill tracking, compression, streaming display)
    │   ├── parser.rs       # JSON action parser (extracts ```json blocks, heuristic fallback)
    │   ├── planner.rs      # Plan creation, step completion, DAG dependency management
    │   ├── plan_parser.rs  # Markdown PLAN.md parser (step headers, checkboxes, actions)
    │   ├── executor.rs     # Tool execution dispatch (all 24 tools + skill resolution)
    │   ├── session.rs      # Session state, save/resume
    │   ├── checkpoint.rs   # Iteration checkpointing every 5 turns
    │   └── train.rs        # Training curriculum system
    ├── core/               # Core utilities
    │   ├── paths.rs        # Vo1dPaths (portable directory resolver, ensure_dirs)
    │   ├── hardware.rs     # Hardware profiling (CPU/GPU/RAM)
    │   ├── memory.rs       # Cross-session memory store (6 stores)
    │   ├── compression.rs  # Context compression (usage_ratio, summarization_range, prune+compact)
    │   ├── curriculum.rs   # Curriculum evaluation & loading
    │   ├── embedded_curricula.rs # Binary-embedded curriculum JSON (include_str!)
    │   ├── behavior.rs     # Behavior modes (Normal, Fix, Research, Refactor, Tdd)
    │   ├── docs.rs         # DocProvider — loads markdown from docs/ for system prompt
    │   ├── error.rs        # Unified Vo1dError enum (thiserror)
    │   ├── self_correction.rs   # FailureTracker & ErrorClassifier
    │   └── error_suggestions.rs # Detailed error analysis with markdown suggestions
    ├── llm/                # LLM backends
    │   ├── backend.rs      # LlmBackend trait (chat, stream_chat)
    │   ├── registry.rs     # Model registry
    │   ├── downloader.rs   # Hugging Face model downloader
    │   ├── builtin.rs      # llama.cpp built-in backend
    │   ├── ollama.rs       # Ollama API backend (cfg: ollama)
    │   ├── lmstudio.rs     # LM Studio backend (cfg: lmstudio)
    │   ├── llamacpp_server.rs # llama.cpp server backend (cfg: llamacpp-server)
    │   └── custom.rs       # Custom OpenAI-compatible API (cfg: custom-api)
    ├── tools/              # Tool system
    │   ├── mod.rs          # Module exports
    │   ├── registry.rs     # ToolRegistry — metadata for all 24 tools
    │   ├── files.rs        # File operations (read, write, list, search, delete, copy, metadata)
    │   ├── shell.rs        # Shell command execution with timeout
    │   ├── web.rs          # HTTP request tool
    │   ├── web_search.rs   # Web search (DuckDuckGo)
    │   ├── web_fetch.rs    # Web page fetch & HTML→markdown
    │   ├── schema.rs       # JSON schema helpers
    │   ├── changes.rs      # show_changes & restore_backup tools
    │   └── skills.rs       # Skill system (Skill, SkillStep, SkillRegistry, CRUD, validation)
    ├── security/           # Security & policy system
    │   ├── modes.rs        # SecurityMode enum
    │   ├── policy.rs       # Policy evaluation engine
    │   ├── approval.rs     # Interactive approval prompts
    │   ├── audit.rs        # JSONL audit logging
    │   ├── privilege.rs    # Windows privilege escalation
    │   └── sandbox.rs      # Workspace sandbox enforcement
    ├── config/             # Configuration
    │   ├── settings.rs     # Settings structs with serde defaults
    │   └── mod.rs          # Load/save settings.toml
    ├── ui/                 # User interfaces
    │   └── cli.rs          # CLI REPL and output helpers
    ├── models/             # Data models
    │   ├── message.rs      # Message, LlmResponse, TokenUsage
    │   ├── action.rs       # Action enum (26 variants — all tools, plan steps, skills)
    │   ├── plan.rs         # Plan, PlanStep, StepStatus structs
    │   └── tool.rs         # Tool definition
    └── utils/              # Misc utilities
        ├── crypto.rs       # SHA256 hashing (including StreamingSha256 for incremental download verification)
        ├── time.rs         # Timestamp formatting
        └── stderr_guard.rs # stderr suppression for llama.cpp
```

---

## Installation

### Prerequisites

- **Rust 1.75+**
- **CMake** (for llama.cpp build)
- **C++ build tools** (MSVC on Windows, GCC/Clang on Linux/macOS)

### Build

```bash
git clone https://github.com/your-username/vo1d.git
cd vo1d

# Build with built-in llama.cpp (recommended for local-only use)
cargo build --features llamacpp-builtin --release

# With Vulkan GPU acceleration
cargo build --features vulkan --release

# With all features (Ollama, LM Studio, etc.)
cargo build --features full --release
```

### Feature Flags

| Flag | Description |
|------|-------------|
| `default` | No backends enabled (CLI only) |
| `llamacpp-builtin` | Built-in llama.cpp inference via `llama-cpp-2` |
| `vulkan` | `llamacpp-builtin` + Vulkan GPU acceleration (compiles ggml-vulkan backend; requires `VULKAN_SDK` on Windows) |
| `ollama` | Ollama API backend |
| `lmstudio` | LM Studio API backend |
| `llamacpp-server` | llama.cpp server backend |
| `custom-api` | Custom OpenAI-compatible API backend |
| `full` | All backends including `vulkan` |

### Install a Model

```bash
# List available models
vo1d models

# Install the default model
vo1d models install qwen3_1.7b

# Show hardware profile and compatible models
vo1d models profile
```

Models are downloaded from Hugging Face and stored in `models/llamacpp/` next to the binary. Each download is verified with **streaming SHA256** — the checksum is fetched from the Hugging Face API at download time and verified incrementally as data streams in, eliminating the need for hardcoded checksums in config files.

---

## Quick Start

```bash
# Start interactive chat (default mode)
vo1d

# Run a one-shot task
vo1d task "list all files in the workspace"

# Run with autonomous mode (auto-approve actions)
vo1d task "create a python script and run it" --mode autonomous

# Run with Fix behavior mode (read-only phase, focused debugging)
vo1d task "fix the broken Rust test" --behavior fix

# Enable verbose debug output
vo1d task "search for todos in the codebase" --debug

# Resume a previous session
vo1d --resume <session-id>

# Train on a curriculum (progressive learning)
vo1d train 00_hello_world
vo1d train curriculum/my_custom_curriculum.json

# Run all curricula with failure retry policy
vo1d train --all --on-failure retry

# Auto-approve all actions (skip prompts)
vo1d task "update all dependencies" --yes

# Test context compression (fills window, then compresses)
vo1d test compression

# Test all tool types (verifies registrations, descriptions, executor dispatch)
vo1d test tools
```

---

## How It Works

vo1d uses a **ReAct** (Reasoning + Acting) agent loop with **LLM-powered context compression**:

1. **System prompt** is constructed with current mode, workspace path, memory context, available tools (including any saved skills), **and markdown documentation** (loaded from `docs/`)
2. **User task** is appended as a message (ChatML format: `<|im_start|>system` / `<|im_start|>user` / `<|im_start|>assistant`)
3. **Model generates** a response — tokens stream to console in real time. `<think>...</think>` blocks are displayed as `─── Reasoning ───` sections. Tool calls (````json`) are shown under `─── Tool Call ───` labels. Tool results appear with `─── Result ───`
4. **Parser extracts** the JSON action from plain JSON, markdown code blocks, or heuristic patterns
5. **Security policy** evaluates the action against the current mode
6. **Action is executed** and the result (or error) is appended to the conversation
7. **Self-correction** checks for repeated failures per action type (≥3 consecutive → auto-correction prompt injected in next iteration)
8. **Context compression** occurs before the LLM call when the conversation exceeds limits:
   - **>60% usage** (`usage_ratio > 0.60`): LLM summarization — the model summarizes older messages, preserving information intelligently instead of dropping them
   - **>100% usage** (`conv_chars > context_limit`): Two-phase prune + compact (truncate oversized tool outputs, keep critical + recent messages)
   - Summaries are stored in memory for future reference
9. **Plan tracking** (via plan tools or PLAN.md): Model can use dedicated plan tools to manage multi-step execution, or fall back to PLAN.md workflow. The loop tracks step progress, iteration counts per step, and injects stuck/recovery messages as needed
10. **Skill resolution**: If the action is `invoke_skill`, the skill's steps are resolved into individual actions and fed back into the loop
11. **Behavior mode enforcement**: Fix/Research modes enforce a read-only phase (blocking writes for first 5 iterations)
12. **Loop repeats** with the updated conversation until `finish` is called. No hard iteration limit — context compression keeps conversations manageable indefinitely. If the model repeats the same action 5+ times, the iteration restarts automatically

```
  User input → [System Prompt + Docs + Skills + Conversation] → LLM → JSON Action
                                                                         ↓
                                                                 Security Policy
                                                                         ↓
                                                                 Tool Executor → Result
                                                                         ↓
                                                         Append to Conversation
                                                                         ↓
                                                   Plan Tracking + Skill Resolution
                                                                  + Behavior Check
                                                                         ↓
                                                        Context Compression (if needed)
                                                       • >60% → LLM summarization
                                                       • >100% → prune + compact
                                                                         ↓
                                                                 Loop or Finish
```

---

## Available Tools

The model communicates actions via JSON blocks enclosed in `` ```json ```. The parser extracts the first valid JSON block from the model output. **24 tools** are registered:

### Read File
```json
{ "action": "read_file", "path": "relative/path/to/file.txt", "start_line": 1, "end_line": 50 }
```
`start_line` and `end_line` are optional.

### Write File
```json
{ "action": "write_file", "path": "output.py", "content": "print('hello world')", "append": false }
```
Set `append: true` to append instead of overwrite.

### Execute Command
```json
{ "action": "execute_command", "command": "dir /b", "timeout": 30, "workdir": "some/subdir" }
```
`timeout` defaults to 60s. `workdir` defaults to workspace root.

### List Directory
```json
{ "action": "list_directory", "path": ".", "pattern": "*.rs" }
```
`pattern` is optional (glob filter).

### Search Files
```json
{ "action": "search_files", "pattern": "**/*.toml", "path": ".", "type": "file" }
```
Uses recursive glob matching.

### Delete File
```json
{ "action": "delete_file", "path": "old-file.txt" }
```
Batch delete by glob pattern:
```json
{ "action": "delete_file", "path": ".", "pattern": "*.txt" }
```
`delete_files` is accepted as an alias.

### Copy File
```json
{ "action": "copy_file", "source": "a.txt", "destination": "b.txt" }
```

### Create Directory
```json
{ "action": "create_directory", "path": "new-folder" }
```

### File Metadata
```json
{ "action": "file_metadata", "path": "some-file.txt" }
```

### Show Changes
```json
{ "action": "show_changes", "path": "." }
```
Lists file changes made in the current session. Uses `git diff` if available, otherwise lists recently modified files.

### Restore Backup
```json
{ "action": "restore_backup", "path": "src/main.rs" }
```
Restores a file to its original state from git.

### HTTP Request
```json
{ "action": "http_request", "url": "https://api.example.com/data", "method": "GET", "headers": {...}, "body": "..." }
```

### Web Search
```json
{ "action": "web_search", "query": "rust programming language features", "num_results": 5 }
```
Uses DuckDuckGo (no API key). `num_results` defaults to 5 (max 10).

### Web Fetch
```json
{ "action": "web_fetch", "url": "https://example.com", "max_chars": 5000 }
```
HTML→markdown conversion. `max_chars` defaults to 8000 (max 50000).

### Finish
```json
{ "action": "finish", "output": "Summary of completed work." }
```

### Ask User
```json
{ "action": "ask_user", "question": "Should I proceed?" }
```

### Plan Create
```json
{
  "action": "plan_create",
  "goal": "Fix bugs",
  "steps": [
    {"id": 1, "description": "Read source", "action": "read_file"},
    {"id": 2, "description": "Fix bugs", "action": "write_file", "depends_on": [1]},
    {"id": 3, "description": "Verify build", "action": "execute_command", "command": "cargo check"}
  ]
}
```

### Plan Step Complete / Fail / Status
```json
{ "action": "plan_step_complete", "step_id": 1, "result": "Done" }
{ "action": "plan_step_fail", "step_id": 2, "error": "File not found" }
{ "action": "plan_status" }
```

---

## Skill System

Skills are reusable, named multi-step procedures that the agent can create, store, and invoke. Each skill is a JSON file on disk in the `skills/` directory.

### Create Skill
```json
{
  "action": "create_skill",
  "name": "setup-rust-project",
  "description": "Initialize a new Rust project with standard structure",
  "params_schema": { "type": "object", "properties": { "project_name": { "type": "string" } }, "required": ["project_name"] },
  "steps": [
    { "tool": "execute_command", "args": { "command": "cargo init --name {{project_name}}" } },
    { "tool": "create_directory", "args": { "path": "src/bin" } },
    { "tool": "write_file", "args": { "path": "rust-toolchain.toml", "content": "[toolchain]\nchannel = \"stable\"" } }
  ]
}
```

### Invoke Skill
```json
{ "action": "invoke_skill", "name": "setup-rust-project", "params": { "project_name": "my-app" } }
```

### List Skills
```json
{ "action": "list_skills", "keyword": "rust" }
```

### Delete Skill
```json
{ "action": "delete_skill", "name": "setup-rust-project" }
```

### Skill Features

- **Parameter schemas** — Define optional JSON schema for invocation parameters; validated at invoke time
- **Parameter interpolation** — Call-site params are merged into step args (template vars like `{{name}}`)
- **Persistence** — Each skill stored as `skills/<name>.json`; loaded on startup
- **Prompt injection** — Available skills are listed in the system prompt (capped at ~1500 chars)
- **Recursion protection** — Circular skill invocations are capped at 10 levels to prevent infinite loops
- **Validation** — Names must match `^[a-z0-9][a-z0-9-]{0,63}$`
- **Atomic writes** — Skills are written via temp file + rename to prevent corruption

---

## Security Modes

| Mode | Writes | Commands | Outside Workspace | System Mods | Elevation |
|------|--------|----------|-------------------|-------------|-----------|
| **Safe** | ❌ Blocked | ❌ Blocked | ❌ Blocked | ❌ Blocked | ❌ Blocked |
| **Interactive** (default) | ✅ Ask | ✅ Ask | ✅ Ask | ❌ Blocked | ❌ Blocked |
| **PowerUser** | ✅ Ask | ✅ Ask | ✅ Ask | ✅ Ask | ❌ Blocked |
| **Autonomous** | ✅ Auto | ✅ Auto | ✅ Ask | ❌ Blocked | ❌ Blocked |
| **YOLO** | ✅ Auto | ✅ Auto | ✅ Auto | ✅ Auto | ✅ Auto |

### Security & Robustness Features

- **SHA256 download verification** — Model checksums fetched from Hugging Face API at download time and verified incrementally via streaming SHA256 hasher; no hardcoded checksums needed in config
- **Token-aware command blacklist** — Blacklist patterns are matched token-by-token, preventing indirection bypass (e.g., `rm -rf /` vs `rm -rf /tmp` are distinguished correctly even with shell expansion)
- **Curriculum setup whitelist** — Dangerous operations (format, shutdown, reg delete, etc.) are blocked in curriculum setup commands
- **Recursion depth limit** — Circular skill invocations are detected and capped at 10 levels
- **Plan recovery counter** — Plan failure recovery uses a `u32` counter (up to 3 recovery attempts before giving up)
- **Memory size cap** — Memory serialization is capped at 50 MB to prevent resource exhaustion
- **CancellationToken for graceful shutdown** — Ctrl+C triggers a `CancellationToken` for clean shutdown instead of abrupt termination
- **Workspace sandbox escape prevention** — `resolve_workspace_path()` validates that resolved paths stay within the sandbox, blocking directory traversal attacks
- **Output size limits** — Tool return values are truncated at 1 MB to prevent context overflow
- **Session versioning** — Sessions carry a `version` field with automatic migration for forward compatibility
- **Compiled-once Regex** — Tool parser regex is compiled once in `ToolParser::new()` instead of per-call
- **Async file I/O** — All session and config I/O uses `tokio::fs` for non-blocking operations
- **Error source chain preservation** — `Vo1dError` variants carry `#[source]` attributes preserving the full error chain via `std::error::Error::source()`

## Behavior Modes

Behavior modes control *how* the agent approaches a task, independent of security modes:

| Mode | Read-Only Phase | Requires Plan | Best For |
|------|-----------------|---------------|----------|
| **normal** (default) | No | No | General-purpose task execution |
| **fix** | 5 iterations | No | Debugging and fixing broken code |
| **research** | 5 iterations | No | Information gathering and analysis |
| **refactor** | No | Yes | Code restructuring without changing behavior |
| **tdd** | No | Yes | Writing tests before implementation |

Fix and Research modes include mode-specific guidance in the system prompt.

---

## Context Compression

Before each LLM call, vo1d checks the conversation size in **characters** (content length + 100 overhead per message). The budget is `context_size × 3` (~24576 chars for 8192 context).

### Path 1 — LLM Summarization (preferred, at >60% usage)

When `usage_ratio > 0.60` and the conversation has 10+ messages:
1. Selects old messages to summarize — always keeps system prompt, first user task, and the last 4+ recent messages
2. Calls the **LLM itself** with a summarization prompt to condense the old messages
3. Replaces the old messages with a `[Summary of previous work: …]` system message
4. Stores the full raw summary in memory for future reference
5. **If summarization fails** (LLM error), falls back to prune + compact

### Path 2 — Prune + Compact (at >100% usage)

When `conv_chars > context_limit`:
- **Phase 1 — Prune**: Truncates any tool output >2000 chars with a length marker
- **Phase 2 — Compact**: Drops older messages (keeps system prompt + first user task + last 4 messages). Targets **20% of the context size**

### Why Summarization First?

Traditional dropping loses context. LLM summarization preserves past decisions, file contents, and reasoning through a condensed summary.

### Configuration

```toml
[llm.builtin]
context_size = 8192
# Conversation budget = context_size × 3 chars
# >60% of budget → LLM summarization
# >100% of budget → prune + compact (target: 20% of context_size)
```

---

## Train Mode

Train mode provides a structured learning environment where vo1d works through progressive curriculum of tasks. Each curriculum consists of a JSON file with sequential tasks, evaluation criteria, and memory accumulation.

### Using Train Mode

```bash
# List available built-in curricula (disk + embedded)
vo1d train

# Run a built-in curriculum
vo1d train 00_hello_world           # Basic file operations
vo1d train 01_file_ops              # File read/write/append/copy
vo1d train 02_directory_ops         # Directory operations
vo1d train 03_search_nav            # File search and navigation
vo1d train 04_shell_basics          # Shell command basics
vo1d train 05_web_basics            # Web search and fetch
vo1d train 06a_syntax               # Fix syntax errors
vo1d train 06b_deps                 # Fix dependency & import errors
vo1d train 06c_logic                # Fix logic bugs
vo1d train 06d_multi_file           # Fix multi-file projects
vo1d train 06e_environment          # Fix build config issues
vo1d train 07_project_setup         # Real-world project scaffolding

# Original combined curriculum (still available)
vo1d train 06_rust_fix

# Run all curricula in sequence (autotrain)
vo1d train --all

# Autotrain with failure policy
vo1d train --all --on-failure stop    # Stop on first failure
vo1d train --all --on-failure skip    # Skip failures (default)
vo1d train --all --on-failure retry   # Retry up to 3 times

# Manual mode — complete tasks yourself
vo1d train 00_hello_world --manual

# Use a custom curriculum file
vo1d train my_curriculum.json
```

### Curriculum Format

```json
{
  "name": "Hello World — File Creation",
  "description": "Learn to create text files",
  "tasks": [
    {
      "id": "create_hello_txt",
      "description": "Create hello.txt with content 'Hello, World!'",
      "expected_outcome": "hello.txt exists with content 'Hello, World!'",
      "setup": ["echo 'broken content' > hello.txt"],
      "evaluation": {
        "check_file_exists": ["hello.txt"],
        "check_file_content": ["hello.txt::Hello, World!"],
        "check_directory_exists": ["src", "tests"],
        "check_command_output": ["dir /b::hello.txt"],
        "check_command_exit_code": ["cargo build 2>&1"]
      }
    }
  ]
}
```

### Evaluation Criteria

- `check_file_exists`: Files that must exist
- `check_file_content`: Content checks in `"path::expected text"` format
- `check_directory_exists`: Directories that must exist
- `check_command_output`: Command output checks in `"command::expected text"` format
- `check_command_exit_code`: Commands that must exit with code 0

### Built-in Curricula

| Curriculum | Description | Tasks |
|-----------|-------------|-------|
| `00_hello_world` | Basic file creation | 3 tasks |
| `01_file_ops` | File operations (read, write, append, copy) | 3 tasks |
| `02_directory_ops` | Directory operations and listing | 3 tasks |
| `03_search_nav` | File search and navigation | 3 tasks |
| `04_shell_basics` | Shell command usage | 3 tasks |
| `05_web_basics` | Web search and fetch | 2 tasks |
| `06_rust_fix` | Original combined Rust fix curriculum | 3 tasks |
| `06a_syntax` | Fix syntax errors | 3 tasks |
| `06b_deps` | Fix dependency & import errors | 3 tasks |
| `06c_logic` | Fix logic bugs | 3 tasks |
| `06d_multi_file` | Fix multi-file project issues | 3 tasks |
| `06e_environment` | Fix build config/environment issues | 3 tasks |
| `07_project_setup` | Real-world project scaffolding | 3 tasks |

### Training Features

- **Autotrain** (`--all`): Run all 13 built-in curricula in sequence
- **Failure policy** (`--on-failure stop|skip|retry`): Control behavior on curriculum failure
- **Embedded curricula**: All JSON files compiled into the binary — works without `curriculum/` on disk
- **Manual mode** (`--manual`): Complete tasks yourself without an LLM
- **Sandboxed execution**: Each task runs in a clean sandbox directory
- **Setup commands**: Run shell commands before each task to create realistic broken/incomplete projects
- **Memory accumulation**: Task outcomes stored in memory across the curriculum
- **Solution storage**: Successful completions stored with full action sequences
- **Mistake learning**: Failures recorded with lessons and avoidance strategies
- **Similar experience recall**: Before each task, memory is searched for similar past problems

---

## Planner (Built-in Plan Tools)

vo1d provides four dedicated plan tools for structured multi-step execution:

- **`plan_create`** — Create or replace the execution plan with a goal and ordered steps
- **`plan_step_complete`** — Mark a step as completed (loop auto-advances to next ready step)
- **`plan_step_fail`** — Mark a step as failed (loop moves to next ready step)
- **`plan_status`** — Query current plan state

### Workflow

1. The model calls `plan_create` with a goal and steps (each with id, description, action type, optional `depends_on`)
2. The loop tracks current step, iteration count per step, and failure count per step
3. On `plan_step_complete`, the loop auto-advances to the next ready step respecting dependency order
4. After 10 iterations on the same step, a stuck-detection warning is injected
5. The model can call `plan_status` at any time to see progress

### PLAN.md Fallback

If the model prefers a file-based workflow, PLAN.md is still supported:
1. After iteration 1, the loop checks if PLAN.md exists in the workspace
2. Steps are extracted from `## Step N:` headers, numbered lists, and checkbox items
3. The model updates checkboxes (`[x]`) as it completes sub-tasks
4. If PLAN.md is rewritten, the plan is re-parsed and step tracking resets

### Terminal Display

Each iteration prints a live plan overview showing goal, step counts (done/pending/failed), and per-step status with markers (`✓`, `✗`, `→`, `☐`).

---

## Self-Correction

### In-Memory Learning from Failures

When an action fails 2+ times consecutively, the error is recorded as a **mistake** in persistent memory:
- Stores what went wrong (action type + error message)
- Stores the corrective suggestion
- Tracks frequency for priority in the memory summary

### Failure Tracking

The `FailureTracker` records consecutive failures per action type. After 3 consecutive failures, the agent injects a correction prompt on the next iteration:

```
The action "read_file" has failed 3 times in a row. Previous errors:
• File not found: missing.txt

Suggestions:
- Check that the file path is correct and relative to the workspace
- Use list_directory to verify the file exists before reading
```

### Error Classification

The `ErrorClassifier` categorizes errors (file not found, permission denied, timeout, etc.) and provides targeted suggestions used in correction prompts and error output.

### Documentation-Driven System Prompt

The `DocProvider` loads markdown files from `docs/` at startup and injects them into the system prompt. Current docs (10 files):

| Doc | Purpose |
|-----|---------|
| `file-ops.md` | File read/write/append/copy/delete patterns |
| `directory-ops.md` | Directory creation, listing, navigation |
| `planning.md` | PLAN.md format, when to plan, recovery strategies |
| `fix-mode.md` | Fix behavior mode rules, diagnosis checklist |
| `research-mode.md` | Research mode techniques and output format |
| `self-improvement.md` | Learning from errors, retry strategies, memory usage |
| `security_modes.md` | Security mode descriptions and capabilities |
| `error_handling.md` | Error analysis and recovery guidance |
| `reasoning_patterns.md` | Reasoning patterns for the model |
| `tool_guidelines.md` | Tool usage guidelines |

---

## Configuration

vo1d creates `config/settings.toml` next to the binary on first run:

```toml
[llm]
backend = "builtin"

[llm.builtin]
context_size = 8192
batch_size = 4096
threads = -1              # -1 = auto-detect CPU core count
gpu_layers = -1           # 0 = CPU only, -1 = all layers (auto-detects GPU at runtime)
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

[defaults]
behavior = "normal"
```

Additional settings: `workspace_path`, `network_whitelist`, `command_blacklist` (11 dangerous commands blocked by default, matches are token-aware to prevent indirection bypass), `max_iterations` (default 999999), `command_timeout_secs` (default 60), `max_backups` (default 10), `default_model`.

### GPU Auto-Detection

When built with `--features vulkan`, vo1d automatically detects GPU offload capability at model load time via `llama_supports_gpu_offload()`. If a compatible GPU is available, all layers are offloaded by default (`gpu_layers = -1`). If no GPU is detected at runtime, it gracefully falls back to CPU with a warning. Set `gpu_layers = 0` explicitly to force CPU-only.

---

## Memory System

vo1d features a **learning memory system** that persists across sessions and improves over time.

### Stores

| Store | Description | File |
|-------|-------------|------|
| **Task History** | Every task executed with actions and outcome | `memory/task_history.json` |
| **Patterns** | Learned patterns with confidence scores (boosted on reuse, capped at 50) | `memory/patterns.json` |
| **Solutions** | Successful solutions with keyword tags and action sequences | `memory/solutions.json` |
| **Mistakes** | Failures with lessons, avoidance strategies, and action sequences | `memory/mistakes.json` |
| **Notes** | User-curated notes with tags | `memory/notes.json` |
| **Preferences** | Key-value settings learned from interaction | `memory/preferences.json` |

### How Learning Works

1. **During training**: Each task result is analyzed — successes stored as solutions, failures as mistakes with lessons
2. **Action sequences**: Both store structured action sequences as JSON arrays
3. **During execution**: 2+ consecutive failures → automatically recorded as a mistake
4. **During recall**: Before a task, the system searches solutions, mistakes, and notes for keyword overlap with the current task
5. **Plan template matching**: Past solutions with PLAN.md structures are detected and presented as plan templates
6. **Pattern reinforcement**: Repeated successes boost confidence scores

### CLI Commands

```bash
# Show memory stats and recent entries
vo1d memory
vo1d memory list

# View full detail of a specific memory by ID
vo1d memory show sol_1718000000_123456789

# Delete a specific memory by ID
vo1d memory delete sol_1718000000_123456789

# Clear all memories or a subset
vo1d memory clear
vo1d memory clear solutions
vo1d memory clear mistakes
vo1d memory clear notes
vo1d memory clear patterns
vo1d memory clear history

# Add a custom note with optional tags
vo1d memory add "Remember to check file paths before writing" --tags "files,writing,best-practice"
```

### Interactive REPL

In chat mode, use `/memory` to view stats and recent entries:

```
VO1D [interactive] >> /memory
=== MEMORY ===
Tasks: 12 | Patterns: 3 | Solutions: 5 | Mistakes: 2 | Notes: 1 | Preferences: 0
```

---

## Session System

Every task execution creates a session with:
- A unique UUID session ID
- Timestamp, task description, and security mode
- Full iteration history in the conversation

```bash
# List all saved sessions
vo1d sessions

# Resume a session by ID
vo1d --resume <session-id> task "continue from where we left off"
```

### Checkpointing

Every 5 iterations, the session state is saved to `sessions/<uuid>/checkpoints/`. Interrupted sessions can be resumed from the last checkpoint.

---

## Audit Logging

All actions are logged to JSONL files in `logs/`:

```
logs/
├── audit_YYYY-MM-DD.jsonl       # All actions
├── yolo_audit_YYYY-MM-DD.jsonl  # YOLO mode actions (extra detail)
└── errors.jsonl                 # Error events
```

Each entry includes: timestamp, action type, security mode, approval status, model and session context.

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
  test      Run system tests (`vo1d test compression`, `vo1d test tools`)
  memory    Manage VO1D's memory and learning
  config    View current configuration
  logs      View audit logs

Options:
      --model <MODEL>           Override default model
      --mode <MODE>             Security mode (safe, interactive, power-user, autonomous, yolo)
      --behavior <BEHAVIOR>     Behavior mode (normal, fix, research, refactor, tdd)
      --workspace <DIR>         Custom workspace directory
      --yolo                    Enable YOLO mode (implies --mode yolo)
      --yes, --jes              Auto-approve all actions
      --debug                   Enable verbose debug tracing
      --resume <ID>             Resume a session by ID
  -h, --help                    Print help
  -V, --version                 Print version

Subcommands:
  test compression         Test context compression
  test tools               Test all tool registrations and executor dispatch
  train --all --on-failure <POLICY>  Autotrain failure policy (stop, skip, retry)
  models list              List available models
  models install <id>      Download and install a model
  models remove <id>       Remove an installed model
  models profile           Show hardware profile and compatible models
  memory list              Show memory stats and entries
  memory show <id>         Show a specific memory by ID
  memory delete <id>       Delete a specific memory by ID
  memory clear [type]      Clear memories (all, solutions, mistakes, notes, patterns, history)
  memory add <content>     Add a custom note (--tags for comma-separated tags)
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
  [✓] qwen3_1.7b (Qwen3 1.7B)
  [✗] qwen3_4b (Qwen3 4B)
  [✗] qwen3_8b (Qwen3 8B)
  ... and 3 more compatible models
```

The profiler checks: CPU model and core count, total and available RAM, GPU vendor/name/dedicated VRAM, overall hardware tier (Low/Medium/High), and compatible models from the registry. When built with `--features vulkan`, `LlamaBackend::supports_gpu_offload()` is queried at model load time to confirm Vulkan acceleration is available.

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
                         │   + Plan Tracking   │
                         │   + Skill Dispatch  │
                         │   + Behavior Mode)  │
                         └──┬───────┬──────┬───┘
                            │       │      │
                     ┌──────▼──┐ ┌──▼──────▼──────────┐
                     │   LLM   │ │  Tool System        │
                     │ Backend │ │  (24 tools in       │
                     │ (trait) │ │   registry)         │
                     └─────────┘ │  • file ops         │
                                 │  • shell/cmd        │
                                 │  • web search/fetch │
                                 │  • plans             │
                                 │  • skills            │
                                 │  • HTTP, changes,    │
                                 │    restore, finish   │
                                 └──┬──────────────────┘
                                    │
                          ┌──────────▼──────────┐
                          │  Subsystems          │
                          │  • Security Manager  │
                          │  • Audit Logger      │
                          │  • Memory Store      │
                          │  • Skill Registry    │
                          │  • Doc Provider      │
                          └─────────────────────┘
```

### Component Details

- **CLI**: Entry point. Clap argument parser dispatches to `vo1d task`, `vo1d chat`, or `vo1d train`.
- **AppContext**: Shared runtime state — config, paths, hardware profile, security manager, audit logger, model registry, doc provider, memory store, skill registry. Cloned per task.
- **Agent Loop**: Iterates without a hard cap — context compression keeps conversations manageable. Each iteration: model generates response (streamed in real time — `<think>` blocks as `─── Reasoning ───`, tool calls as `─── Tool Call ───`, results as `─── Result ───`) → parser extracts JSON action → security evaluates → executor runs → result appended → self-correction checks → plan tracking → skill resolution → behavior enforcement → context compression if needed. Auto-restarts if model repeats same action 5+ times.
- **LLM Backend**: Trait with `chat()` and `stream_chat()`. Implementations: builtin (llama.cpp), ollama, lmstudio, llamacpp-server, custom-api. Pluggable design.
- **Tool System**: Registry of 24 tools — file operations, shell commands, directory management, HTTP requests, web search (DuckDuckGo), web fetch (HTML→markdown), show changes, restore backup, plan tools (create, complete, fail, status), skill tools (create, invoke, list, delete).
- **Security Manager**: Evaluates each action against the current mode. Approve, ask, or block. All decisions audited. Includes token-aware command blacklist and sandbox escape prevention.
- **Doc Provider**: Loads 10 markdown docs from `docs/` into system prompt.
- **Memory System**: 6 stores (task history, preferences, patterns, solutions, mistakes, notes). Similarity matching, plan template matching, action sequence tracking. Capped at 50 MB.
- **Skill Registry**: Loads skills from `skills/` directory. Create, delete, list, get, resolve steps into actions. Prompt injection of available skills. Recursion capped at 10 levels.
- **Self-Correction**: `FailureTracker` monitors consecutive failures; `ErrorClassifier` + `error_suggestions` produce markdown suggestions.
- **Plan System**: Plan tools for structured multi-step execution with auto-advancement. PLAN.md fallback via `plan_parser`. Recovery counter up to 3 retries.
- **Behavior Engine**: Read-only phases, plan requirements, mode-specific prompt notes. TDD phase transitions are plan-independent.
- **Context Compressor**: Two-phase prune+compact at >100% usage; LLM summarization at >60%. Summaries stored in memory. Output truncated at 1 MB.
- **Download Verifier**: Streaming SHA256 checksum verification during model download; checksum fetched from Hugging Face API.

---

## Portability

vo1d is fully portable:
- All data stored relative to the executable
- No registry entries, no system-wide install paths
- Config: `<exe-dir>/config/settings.toml`
- Models: `<exe-dir>/models/llamacpp/`
- Sessions: `<exe-dir>/sessions/<uuid>/`
- Logs: `<exe-dir>/logs/`
- Skills: `<exe-dir>/skills/<name>.json`
- Memory: `<exe-dir>/memory/*.json`

---

## Development

### Running Tests

```bash
# Run all unit tests (85+ tests)
cargo test

# Run integration tests (6 test files)
cargo test --test integration
cargo test --test skill_ops
cargo test --test tool_parser
cargo test --test security
cargo test --test models
cargo test --test file_ops

# Run tests with a specific feature
cargo test --features llamacpp-builtin

# Build with Vulkan GPU support
cargo build --features vulkan --release
```

### Code Style

- Follow existing patterns (see `src/` structure)
- `serde` derives for all data structs
- `anyhow::Result` for error propagation
- `tracing` for logging (not `println`/`eprintln` in library code)
- CLI output uses `print!`/`eprintln!` only in `loop_.rs`, guarded by `tui_mode`
- No unsafe code unless absolutely necessary (see `stderr_guard.rs`)

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `llama_context: n_batch = 512` then crash | Delete `config/settings.toml` or set `batch_size = 4096` |
| `Model file not found` | Run `vo1d models install <model-id>` |
| `spawn_blocking panicked` in `builtin.rs` | Check RAM usage, reduce `context_size` or `gpu_layers` |
| Vulkan not found at runtime | Ensure `VULKAN_SDK` is installed and `vulkan-1.dll` is on `PATH`. Build with `--features vulkan` |
| Vulkan build fails on Windows | Install Vulkan SDK from https://vulkan.lunarg.com/ |
| Model keeps generating infinite tool calls | Check ChatML prompt format. Qwen3 expects `<|im_start|>` tags |
| LLM output garbled with C library stderr | Verify `stderr_guard.rs` compiled correctly |
| Build fails with linker errors on Windows | Need MSVC build tools; `+crt-static` must NOT be in `.cargo/config.toml` |
| Curriculum file not found on disk | All curricula are embedded in the binary — run `vo1d train <name>` |
| Skill file won't load | Check JSON syntax and that name matches `^[a-z0-9][a-z0-9-]{0,63}$` |
| Session won't load after update | Session migration runs automatically — old sessions are upgraded on resume |

---

## Model Registry

vo1d includes a built-in catalog of compatible models defined in `config/default_models.toml` (default) and `config/models.toml` (user overrides).

### Adding a Custom Model

Edit `config/models.toml`:
```toml
[[model]]
id = "my-model"
name = "My Custom Model"
provider = "huggingface"
download_url = "https://huggingface.co/author/model/resolve/main/model.gguf"
filename = "model.gguf"
sha256 = ""                          # Auto-fetched from Hugging Face API at download time
size_bytes = 2000000000
min_ram_gb = 4.0
context_length = 4096
supports_tools = false
native_tools = false                 # set true if model supports structured function calling
quantization = "Q4_K_M"
```

The `native_tools` field indicates whether the model supports structured function calling natively (e.g., Qwen 2.5, Llama 3.1+, Gemma 3/4, Mistral). When enabled, tool definitions are injected directly into the ChatML system prompt header and the model's JSON tool call responses are parsed automatically, bypassing the text-based ````json```` action extraction.

---

## License

GNU General Public License v3.0
