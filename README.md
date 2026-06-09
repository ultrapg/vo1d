# vo1d

**Local-first autonomous AI execution agent.** vo1d runs local LLMs (llama.cpp with optional Vulkan GPU acceleration) to interpret tasks and execute them via tools — file operations, shell commands, HTTP requests, web search, plan management, reusable skills, and more. All inference runs entirely offline on your hardware.

## Philosophy

vo1d is built on the principle that AI agents should be **private, offline, and user-controlled**. Unlike cloud-based agents that send your data to third-party servers, vo1d runs entirely on your local hardware. The built-in llama.cpp backend means zero network dependencies for inference — your files, code, and prompts never leave your machine. The security model enforces progressive trust levels from read-only exploration to full system access, putting you in control of what the agent can do.

## Features

- **Local LLM inference** — Built-in llama.cpp backend, no cloud dependency
- **Native tool calling** — Parser handles both `{"action": ...}` and OpenAI-style `{"name": ..., "arguments": {...}}"` formats, with concise `- name: description` tool definitions to keep prompts small
- **Single action enforcement** — Only one tool action per iteration; multi-action responses are rejected to prevent runaway execution
- **Autonomous task execution** — ReAct agent loop that plans, acts, and iterates
- **5 security modes** — Safe, Interactive, PowerUser, Autonomous, Unrestricted
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
- **Model management** — Download, install, and manage multiple models with automatic dependency handling
- **Vulkan GPU acceleration** — Optional GPU offload for built-in llama.cpp backend, compatible with AMD, NVIDIA, and Intel GPUs via Vulkan API
- **Runtime GPU auto-detection** — GPU support is detected at model load time; same binary works with or without a compatible GPU
- **Graceful GPU fallback** — Falls back to CPU with a warning if no GPU is available at runtime
- **GPU disable toggle** — `no_gpu = true` in settings forces CPU-only even when a Vulkan device is available
- **Configurable inference timeout** — Adjustable timeout (default 600s) via `inference_timeout_secs` setting

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
    │   ├── downloader.rs   # Hugging Face model downloader with progress tracking
    │   ├── builtin.rs      # llama.cpp built-in backend with GPU auto-detection
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

        ├── time.rs         # Timestamp formatting
        └── stderr_guard.rs # stderr suppression for llama.cpp
```

---

## Installation

### Prerequisites

- **Rust 1.75+** (install via https://rustup.rs/)
- **CMake 3.20+** (for llama.cpp build; install via `winget install CMake` on Windows, `brew install cmake` on macOS, or your package manager on Linux)
- **C++ build tools** (MSVC on Windows, GCC/Clang on Linux/macOS)
  - Windows: Install "Desktop development with C++" via Visual Studio Installer, or install `llvm` and `clang` via `winget install LLVM`
  - Linux: `sudo apt install build-essential cmake` (Debian/Ubuntu) or `sudo dnf groupinstall "Development Tools"` (Fedora)
  - macOS: `xcode-select --install`

### Vulkan SDK (Required for `--features full` or `--features vulkan`)

Vulkan GPU acceleration requires the Vulkan SDK:

1. **Download** the Vulkan SDK from https://vulkan.lunarg.com/
2. **Install** it (default path: `C:\VulkanSDK\1.4.x.x` on Windows)
3. **Set the `VULKAN_SDK` environment variable in the SAME terminal** where you will run `cargo build`:
   - Windows: `set VULKAN_SDK=C:\VulkanSDK\1.4.309.0` (adjust version to match installed SDK)
   - Or set it system-wide (persists across terminals): `setx VULKAN_SDK "C:\VulkanSDK\1.4.309.0"`
   - Linux: `export VULKAN_SDK=/path/to/vulkan-sdk/x86_64`
   - macOS: `export VULKAN_SDK=/path/to/VulkanSDK/macOS`
4. **Verify**: `echo %VULKAN_SDK%` (Windows) or `echo $VULKAN_SDK` (Linux/macOS) — must print a non-empty path

**Important**: Installing the Vulkan SDK alone is **not sufficient** — the `VULKAN_SDK` environment variable must be explicitly set. Even if the SDK installer claims to set it, you may need to open a **new terminal** or set it manually. The `llama-cpp-sys-2` build script panics at `build.rs:768:58` with `"Please install Vulkan SDK and ensure that VULKAN_SDK env variable is set: NotPresent"` if this variable is missing.

If you previously built without Vulkan and are now enabling it, run `cargo clean` first to force a full rebuild of the C dependencies:

```bash
cargo clean
set VULKAN_SDK=C:\VulkanSDK\1.4.309.0
cargo build --features full --release
```

Without the Vulkan SDK, you can still build with CPU-only backends (`llamacpp-builtin`, `ollama`, `lmstudio`, `llamacpp-server`, `custom-api`).

### Build

```bash
git clone https://github.com/ultrapg/vo1d.git
cd vo1d

# Build with built-in llama.cpp (CPU only, no Vulkan SDK needed)
cargo build --features llamacpp-builtin --release

# Build with Vulkan GPU acceleration (requires VULKAN_SDK)
cargo build --features vulkan --release

# Build with all features (requires VULKAN_SDK for vulkan component)
cargo build --features full --release

# Build with all remote backends + builtin (no Vulkan, no SDK needed)
cargo build --features "ollama,lmstudio,llamacpp-server,custom-api,llamacpp-builtin" --release
```

### Feature Flags

| Flag | Description | SDK Required |
|------|-------------|--------------|
| `default` | No backends enabled (CLI only) | No |
| `llamacpp-builtin` | Built-in llama.cpp inference via `llama-cpp-2` | No |
| `vulkan` | `llamacpp-builtin` + Vulkan GPU acceleration (compiles ggml-vulkan backend) | Yes (`VULKAN_SDK`) |
| `ollama` | Ollama API backend | No |
| `lmstudio` | LM Studio API backend | No |
| `llamacpp-server` | llama.cpp server backend | No |
| `custom-api` | Custom OpenAI-compatible API backend | No |
| `full` | All backends including `vulkan` | Yes (`VULKAN_SDK`) |

**Note**: The `full` feature includes `vulkan`, which requires the Vulkan SDK. If you want all backends without Vulkan, enable each backend individually:
```bash
cargo build --features "llamacpp-builtin,ollama,lmstudio,llamacpp-server,custom-api" --release
```

### Install a Model

```bash
# List available models
vo1d models

# Install the default model
vo1d models install qwen3_1.7b

# Show hardware profile and compatible models
vo1d models profile
```

Models are downloaded from Hugging Face and stored in `models/llamacpp/` next to the binary. Downloads include progress tracking and automatic resume support.

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

# View current configuration
vo1d config

# View audit logs
vo1d logs
```

### First Run

On first run, vo1d creates the directory structure next to the executable:
```
vo1d.exe
├── config/
│   └── settings.toml      # Created with defaults on first run
├── docs/                   # Markdown docs for system prompt (optional)
├── curriculum/             # Curriculum JSON files (optional)
├── models/
│   └── llamacpp/           # Downloaded GGUF models
├── sessions/               # Session checkpoints and state
├── logs/                   # Audit log files
├── skills/                 # Saved skill JSON files
└── memory/                 # Cross-session memory stores
```

---

## How It Works

vo1d uses a **ReAct** (Reasoning + Acting) agent loop with **LLM-powered context compression**:

1. **System prompt** is constructed with current mode, workspace path, memory context, available tools (including any saved skills), **and markdown documentation** (loaded from `docs/`)
2. **User task** is appended as a message (ChatML format: `<|im_start|>system` / `<|im_start|>user` / `<|im_start|>assistant`)
3. **Model generates** a response — tokens stream to console in real time. `<think>...</think>` blocks are displayed as `─── Reasoning ───` sections. Tool calls (```` ```jsonl ````) are shown under `─── Tool ───` labels. Tool results appear with `─── Result ───`
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
11. **Behavior mode enforcement**: Fix/Research modes enforce a read-only phase (blocking writes for first 5 iterations); TDD enforces write-test-first-then-implement order; Refactor enforces test-run-before-write
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

### ReAct Loop Details

Each iteration follows this sequence:
1. **Build conversation** — system prompt + task + progress messages + tool results
2. **Check context usage** — compress if >60% or >100% of budget
3. **Call LLM** — stream tokens with real-time display formatting
4. **Parse response** — extract action from JSON, tool calls, or heuristic fallback
5. **Check conversational response** — if no tool intent detected, prompt for action
6. **Check reasoning quality** — if no reasoning before action, inject reminder
7.  **Enforce single action** — only the first tool action is taken; multi-action responses are rejected
8.  **Loop detection** — if same action repeated 5+ times, restart iteration with warning
9.  **Handle Finish** — break loop if task is complete
10. **Behavior enforcement** — read-only phase, TDD phase transitions, refactor pre-checks
11. **Security evaluation** — Allow/Ask/Block based on current security mode
12. **Plan guidance** — soft alignment warning if action mismatches current plan step
13. **Plan action handling** — process plan_create/complete/fail/status without executor
14. **Execute action** — run tool, format result, append to conversation
15. **Track success/failure** — update memory, plan step completion, failure counters
16. **Plan auto-advancement** — if step action matches, complete step and advance to next
17. **Re-parse PLAN.md** — if PLAN.md was written to disk
18. **Save checkpoint** — every 5 iterations
19. **Increment iteration** — break if over safety limit

---

## Vulkan GPU Acceleration

vo1d supports optional GPU acceleration for the built-in llama.cpp backend via the Vulkan API. This enables offloading neural network computation to compatible GPUs for significantly faster inference.

### How It Works

- When built with `--features vulkan`, the `llama-cpp-2` crate compiles with the `ggml-vulkan` backend, adding Vulkan GPU support
- At model load time, `LlamaBackend::supports_gpu_offload()` checks whether a compatible Vulkan GPU is available on the system
- If GPU is detected and `gpu_layers` is set to `-1` (default), all layers are offloaded to GPU
- If no GPU is detected, the backend falls back to CPU with a warning message
- GPU support is detected **at runtime** — the same binary works on systems with or without Vulkan-compatible GPUs

### GPU Auto-Detection Logic (src/llm/builtin.rs)

```
Model load starts
    → Check backend.supports_gpu_offload()
    → If YES and gpu_layers == -1:
        → Keep gpu_layers = -1 (offload all layers)
    → If NO and gpu_layers == -1:
        → Set gpu_layers = 0 (CPU only)
        → Print warning: "GPU offload not supported, falling back to CPU"
    → Set with_n_gpu_layers(gpu_layers) on model context params
```

### GPU Compatibility

Vulkan GPU acceleration works with any GPU that has Vulkan driver support:
- **AMD**: Radeon RX 400 series and newer (via AMD Vulkan driver)
- **NVIDIA**: GeForce GTX 900 series and newer (via NVIDIA Vulkan driver)
- **Intel**: HD Graphics 500 series and newer, Arc GPUs (via Intel Vulkan driver)
- **Apple**: No Vulkan support — use CPU or external backends (Ollama, LM Studio)

### Performance Notes

- GPU offload is most beneficial for larger models (7B+ parameters) where CPU inference is slow
- Smaller models (1-3B parameters) may run adequately on CPU alone
- GPU inference reduces CPU load and frees CPU cores for other tasks
- Actual performance depends on GPU VRAM, driver version, and model size
- Batch size (`batch_size` in settings) affects GPU utilization — larger batches use GPU more efficiently

### Configuration

```toml
[llm.builtin]
gpu_layers = -1             # 0 = CPU only, -1 = all layers (auto-detects GPU at runtime)
no_gpu = false              # true = force CPU even if GPU available
inference_timeout_secs = 600 # Inference timeout in seconds (min 60)
```

Set `gpu_layers = 0` or `no_gpu = true` in `config/settings.toml` to force CPU-only mode even when a GPU is available.

---

## Available Tools

The model communicates actions via JSON blocks enclosed in `` ```json ```. The parser extracts the first valid JSON block from the model output. **24 tools** are registered:

### Read File
```json
{ "action": "read_file", "path": "relative/path/to/file.txt", "start_line": 1, "end_line": 50 }
```
`start_line` and `end_line` are optional. Omit both to read the entire file. Lines are 1-indexed. If the file is binary or very large (>1 MB), the output is truncated.

### Write File
```json
{ "action": "write_file", "path": "output.py", "content": "print('hello world')", "append": false }
```
Set `append: true` to append instead of overwrite. Parent directories are created automatically. Files larger than 1 MB are truncated in the result but written correctly to disk.

### Execute Command
```json
{ "action": "execute_command", "command": "dir /b", "timeout": 30, "workdir": "some/subdir" }
```
`timeout` defaults to 60s. `workdir` defaults to workspace root. Command output is captured and returned; stderr is included in the output. Use `&&` to chain dependent commands, `&` for independent parallel commands. Long-running commands are killed after the timeout.

### List Directory
```json
{ "action": "list_directory", "path": ".", "pattern": "*.rs" }
```
`pattern` is optional (glob filter). Directories are shown with a trailing `/`. If `pattern` is omitted, all entries are listed.

### Search Files
```json
{ "action": "search_files", "pattern": "**/*.toml", "path": ".", "type": "file" }
```
Uses recursive glob matching. `type` can be `"file"`, `"glob"`, `"regex"`, or `"name"` (default: glob). The `path` defaults to workspace root.

### Delete File
```json
{ "action": "delete_file", "path": "old-file.txt" }
```
Batch delete by glob pattern:
```json
{ "action": "delete_file", "path": ".", "pattern": "*.txt" }
```
`delete_files` is accepted as an alias. Use `"pattern": "*"` (not `"*.*"`) to match all files — `*.*` misses files without a dot extension.

### Copy File
```json
{ "action": "copy_file", "source": "a.txt", "destination": "b.txt" }
```
Creates parent directories of the destination if they don't exist. Overwrites destination if it exists.

### Create Directory
```json
{ "action": "create_directory", "path": "new-folder" }
```
Creates all parent directories as needed (like `mkdir -p`). Succeeds silently if directory already exists.

### File Metadata
```json
{ "action": "file_metadata", "path": "some-file.txt" }
```
Returns file size, modification time, file type (file/directory), and permissions. Useful for checking if a file exists before reading it.

### Show Changes
```json
{ "action": "show_changes", "path": "." }
```
Lists file changes made in the current session. Uses `git diff` if available, otherwise lists recently modified files sorted by modification time.

### Restore Backup
```json
{ "action": "restore_backup", "path": "src/main.rs" }
```
Restores a file to its original state from git version control. Uses `git checkout` or `git restore` internally. Fails gracefully if the file is not tracked by git.

### HTTP Request
```json
{ "action": "http_request", "url": "https://api.example.com/data", "method": "GET", "headers": {...}, "body": "..." }
```
Supports GET, POST, PUT, DELETE, PATCH methods. Headers are optional key-value pairs. Body is a string (use JSON.stringify for JSON payloads). Response includes status code, headers, and body (truncated at 1 MB).

### Web Search
```json
{ "action": "web_search", "query": "rust programming language features", "num_results": 5 }
```
Uses DuckDuckGo (no API key required). `num_results` defaults to 5 (max 10). Returns title, URL, and snippet for each result.

### Web Fetch
```json
{ "action": "web_fetch", "url": "https://example.com", "max_chars": 5000 }
```
HTML→markdown conversion. `max_chars` defaults to 8000 (max 50000). Fetches the page, converts HTML content to clean markdown, and returns the text.

### Finish
```json
{ "action": "finish", "output": "Summary of completed work." }
```
Signals task completion. The output is stored as the session's final result and displayed to the user. Always call this when the task is fully complete.

### Ask User
```json
{ "action": "ask_user", "question": "Should I proceed?" }
```
Pauses execution and displays the question to the user. Waits for a response before continuing. Use this when you need clarification or confirmation.

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
Creates a structured plan with DAG dependency ordering. Steps with `depends_on` will not be auto-advanced to until all dependencies are complete. Steps without `depends_on` are eligible immediately.

### Plan Step Complete / Fail / Status
```json
{ "action": "plan_step_complete", "step_id": 1, "result": "Done" }
{ "action": "plan_step_fail", "step_id": 2, "error": "File not found" }
{ "action": "plan_status" }
```
`plan_step_complete` marks a step done and triggers auto-advancement. `plan_step_fail` records the error and moves to next ready step. `plan_status` returns the full plan state (goal, all step statuses, current position).

### Plan Status Output
```
Goal: Fix bugs
Steps: 1 done | 1 pending | 0 failed
  ✓ 1. Read source
  ☐ 2. Fix bugs
```

### Create Skill
```json
{
  "action": "create_skill",
  "name": "setup-rust-project",
  "description": "Initialize a new Rust project with standard structure",
  "params_schema": { ... },
  "steps": [
    { "tool": "execute_command", "args": { "command": "cargo init --name {{project_name}}" } },
    { "tool": "create_directory", "args": { "path": "src/bin" } }
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

---

## Skill System

Skills are reusable, named multi-step procedures that the agent can create, store, and invoke. Each skill is a JSON file on disk in the `skills/` directory.

### Skill Lifecycle

1. **Create** — Model or user creates a skill with name, description, optional parameter schema, and tool steps
2. **Store** — Skill is saved as `skills/<name>.json` with atomic write (temp file + rename)
3. **List** — Available skills are injected into the system prompt (capped at ~1500 chars)
4. **Invoke** — Model calls `invoke_skill` with name and optional parameters; steps execute sequentially
5. **Update** — Re-creating a skill with the same name overwrites the existing one
6. **Delete** — Model or user calls `delete_skill` to remove it

### Parameter Interpolation

Skill step arguments support template variables using `{{variable_name}}` syntax. When a skill is invoked with params, the variables are replaced:

**Skill definition:**
```json
{
  "name": "create-file-with-content",
  "params_schema": {
    "type": "object",
    "properties": {
      "filename": { "type": "string" },
      "content": { "type": "string" }
    },
    "required": ["filename", "content"]
  },
  "steps": [
    { "tool": "write_file", "args": { "path": "{{filename}}", "content": "{{content}}" } }
  ]
}
```

**Invocation:**
```json
{ "action": "invoke_skill", "name": "create-file-with-content", "params": { "filename": "hello.txt", "content": "Hello World!" } }
```

### Skill Features

- **Parameter schemas** — Define optional JSON schema for invocation parameters; validated at invoke time
- **Parameter interpolation** — Call-site params are merged into step args (template vars like `{{name}}`)
- **Persistence** — Each skill stored as `skills/<name>.json`; loaded on startup
- **Prompt injection** — Available skills are listed in the system prompt (capped at ~1500 chars)
- **Recursion protection** — Circular skill invocations are capped at 10 levels to prevent infinite loops
- **Validation** — Names must match `^[a-z0-9][a-z0-9-]{0,63}$`
- **Atomic writes** — Skills are written via temp file + rename to prevent corruption

### Example: Project Setup Skill

```json
{
  "name": "setup-node-project",
  "description": "Initialize a Node.js project with package.json, ESLint, and Prettier",
  "params_schema": {
    "type": "object",
    "properties": {
      "name": { "type": "string" },
      "description": { "type": "string" }
    },
    "required": ["name"]
  },
  "steps": [
    { "tool": "create_directory", "args": { "path": "{{name}}" } },
    { "tool": "write_file", "args": { "path": "{{name}}/package.json", "content": "{\n  \"name\": \"{{name}}\",\n  \"description\": \"{{description}}\",\n  \"version\": \"1.0.0\"\n}" } },
    { "tool": "execute_command", "args": { "command": "cd {{name}} && npm install eslint prettier --save-dev" } },
    { "tool": "write_file", "args": { "path": "{{name}}/.eslintrc.json", "content": "{ \"extends\": \"eslint:recommended\" }" } }
  ]
}
```

---

## Security Modes

| Mode | Writes | Commands | Outside Workspace | System Mods | Elevation |
|------|--------|----------|-------------------|-------------|-----------|
| **Safe** | ❌ Blocked | ❌ Blocked | ❌ Blocked | ❌ Blocked | ❌ Blocked |
| **Interactive** (default) | ✅ Ask | ✅ Ask | ✅ Ask | ❌ Blocked | ❌ Blocked |
| **PowerUser** | ✅ Ask | ✅ Ask | ✅ Ask | ✅ Ask | ❌ Blocked |
| **Autonomous** | ✅ Auto | ✅ Auto | ✅ Ask | ❌ Blocked | ❌ Blocked |
| **Unrestricted** | ✅ Auto | ✅ Auto | ✅ Auto | ✅ Auto | ✅ Auto |

### Mode Details

- **Safe**: Read-only exploration. The model can read files, list directories, search, and fetch web content. No writes, no command execution, no outside-workspace access. Ideal for code review or investigation.
- **Interactive (default)**: Every write and command execution prompts the user for approval. Outside-workspace access also prompts. System modifications and privilege escalation are blocked. Best balance of safety and utility.
- **PowerUser**: Like Interactive but also allows system modifications with approval prompts. Use when you need to install packages or modify system configuration.
- **Autonomous**: Auto-approves all writes and commands within the workspace. Outside-workspace access still prompts. System modifications blocked. Use for trusted automated tasks.
- **Unrestricted**: All actions auto-approved, including privilege escalation and system modifications. Use with extreme caution — the model has full access to your system.

### Security & Robustness Features

- **Model management** — Download, install, and manage multiple models with progress tracking and automatic resume
- **Token-aware command blacklist** — Blacklist patterns are matched token-by-token, preventing indirection bypass (e.g., `rm -rf /` vs `rm -rf /tmp` are distinguished correctly even with shell expansion)
- **Default blacklist** — 11 dangerous commands blocked by default: `rm -rf /`, `mkfs`, `dd if=`, `format `, `del /f /s /q`, `rd /s /q`, `reg delete`, `sc delete`, `systemctl disable`, `shutdown`, `reboot`
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

Fix and Research modes include mode-specific guidance in the system prompt:

- **Fix mode**: Injects hypothesis tracking — the model's reasoning is stored as a hypothesis. If the fix attempt fails, the mistake memory includes "Hypothesis was wrong" to guide future attempts.
- **Research mode**: Encourages thorough exploration before concluding. Read-only phase ensures the model investigates before making changes.
- **Refactor mode**: Enforces test execution before any writes — the model must run existing tests first to establish a baseline.
- **TDD mode**: Enforces three-phase lifecycle — Red (write failing test), Green (write minimal implementation), Refactor (improve code). Phase transitions are tracked automatically.

### TDD Phase Lifecycle

```
TDD Mode Active
    → RED phase: Only test file writes allowed
    → Test file written → GREEN phase: Write implementation code
    → Implementation written → REFACTOR phase: Improve code quality
    → Tests pass → Done or cycle continues
```

---

## Context Compression

Before each LLM call, vo1d checks the conversation size in **characters** (content length + 100 overhead per message). The budget is `context_size × 3` (~24576 chars for 8192 context).

### Path 1 — LLM Summarization (preferred, at >60% usage)

When `usage_ratio > 0.60` and the conversation has 10+ messages:
1. Selects old messages to summarize — always keeps system prompt, first user task, and the last 4+ recent messages
2. Calls the **LLM itself** with a summarization prompt to condense the old messages
3. Replaces the old messages with a `[Summary of previous work: …]` system message
4. Stores the full raw summary in memory for future reference
5. **If summarization fails** (LLM error or timeout after 300s), falls back to prune + compact

### Path 2 — Prune + Compact (at >100% usage)

When `conv_chars > context_limit`:
- **Phase 1 — Prune**: Truncates any tool output >2000 chars with a length marker
- **Phase 2 — Compact**: Drops older messages (keeps system prompt + first user task + last 4 messages). Targets **20% of the context size**

### Why Summarization First?

Traditional dropping loses context. LLM summarization preserves past decisions, file contents, and reasoning through a condensed summary. The compressed summary is also stored in memory for future sessions, creating a persistent knowledge base.

### Configuration

```toml
[llm.builtin]
context_size = 8192
# Conversation budget = context_size × 3 chars
# >60% of budget → LLM summarization
# >100% of budget → prune + compact (target: 20% of context_size)
```

### Debugging Compression

To see compression in action, use verbose mode:
```bash
vo1d task "perform a complex multi-step task" --debug
```
You'll see messages like:
```
── [Context summarized: 24 msgs → 12 msgs] ──
── [Context compressed: 18 msgs → 7 msgs] ──
```

### Compression Tuning

- **Larger context_size** = less frequent compression but higher memory usage
- **8192** is recommended for most models (good balance)
- **4096** for low-RAM systems (compression fires more often)
- **16384**+ for large models with ample VRAM (compression rarely fires)

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

### Creating Custom Curricula

To create your own curriculum:
1. Create a JSON file following the format above
2. Place it in the `curriculum/` directory, or reference it by path: `vo1d train ./my_curriculum.json`
3. Each task should have:
   - `id`: Unique identifier
   - `description`: Task description shown to the LLM
   - `expected_outcome`: What success looks like
   - `setup` (optional): Shell commands to prepare the sandbox (run before task)
   - `evaluation`: Criteria to check (see below)
   - `setup_on_retry` (optional): Different setup commands for retry attempts

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

### Curriculum Task Execution Flow

```
1. Load curriculum JSON (disk or embedded)
2. For each task in sequence:
   a. Create clean sandbox directory
   b. Run setup commands (if any) — blocked for dangerous ops
   c. Present task description to LLM
   d. Run ReAct loop with 5-minute timeout
   e. Evaluate success criteria
   f. Record outcome in memory (solution or mistake)
   g. Clean up sandbox
3. Report final results (tasks passed/failed/total)
```

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
5. After 100 iterations on the same step, the step is automatically marked failed
6. The model can call `plan_status` at any time to see progress

### DAG Dependency Ordering

Steps can declare dependencies using the `depends_on` field. A step is only eligible for auto-advancement when all its dependencies are completed:

```json
{
  "steps": [
    {"id": 1, "description": "Read source", "action": "read_file"},
    {"id": 2, "description": "Analyze dependencies", "action": "read_file", "depends_on": [1]},
    {"id": 3, "description": "Implement fix", "action": "write_file", "depends_on": [1, 2]},
    {"id": 4, "description": "Run tests", "action": "execute_command", "command": "cargo test", "depends_on": [3]}
  ]
}
```
In this example, step 1 runs first. Steps 2 and 4 cannot run until their dependencies are complete.

### Plan Recovery

- After 3 consecutive failures on a step, it's marked as failed and the loop moves to the next ready step
- After 6 total consecutive failures across all steps, a replanning message is injected
- Plan recovery counter allows up to 3 replanning attempts before giving up

### PLAN.md Fallback

If the model prefers a file-based workflow, PLAN.md is still supported:
1. After iteration 1, the loop checks if PLAN.md exists in the workspace
2. Steps are extracted from `## Step N:` headers, numbered lists, and checkbox items
3. The model updates checkboxes (`[x]`) as it completes sub-tasks
4. If PLAN.md is rewritten, the plan is re-parsed and step tracking resets

### Terminal Display

Each iteration prints a live plan overview showing goal, step counts (done/pending/failed), and per-step status with markers (`✓`, `✗`, `→`, `☐`).

```
─── Iteration 3 [Step 2/4: Analyze dependencies] [█████░░░░░]2/4 steps ───
  Goal: Fix failing unit tests in the parser module
  Steps: 2 done | 1 pending | 0 failed
    ✓ 1. Read source
    → 2. Analyze dependencies ← CURRENT
    ☐ 3. Implement fix
    ✓ 4. Run tests
```

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

### Error Categories

- **FileNotFound** — File or directory doesn't exist
- **PermissionDenied** — Access denied to file or resource
- **Timeout** — Command or operation timed out
- **ParseError** — Could not parse output or response
- **NetworkError** — Web request failed
- **SecurityBlock** — Action blocked by security policy
- **Unknown** — Unclassified error

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
no_gpu = false            # true = force CPU even if GPU available
inference_timeout_secs = 600 # Inference timeout in seconds (min 60)
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

### Complete Settings Reference

| Setting | Type | Default | Description |
|---------|------|---------|-------------|
| `llm.backend` | string | `"builtin"` | LLM backend to use: `builtin`, `ollama`, `lmstudio`, `llamacpp-server`, `custom` |
| `llm.builtin.threads` | i32 | `-1` | CPU threads for inference (-1 = auto-detect) |
| `llm.builtin.gpu_layers` | i32 | `-1` | GPU layers to offload (0 = CPU only, -1 = all, N = first N layers) |
| `llm.builtin.batch_size` | u32 | `4096` | Batch size for prompt processing (higher = faster but more RAM) |
| `llm.builtin.context_size` | u32 | `8192` | Context window size in tokens |
| `llm.builtin.temperature` | f32 | `0.7` | Generation temperature (0.0 = deterministic, 2.0 = random) |
| `llm.builtin.top_p` | f32 | `0.9` | Nucleus sampling threshold (1.0 = disabled) |
| `llm.builtin.top_k` | u32 | `40` | Top-K sampling (0 = disabled) |
| `llm.builtin.repeat_penalty` | f32 | `1.1` | Repeat penalty (1.0 = disabled, higher = less repetition) |
| `llm.builtin.max_tokens` | u32 | `2048` | Maximum tokens per generation |
| `llm.builtin.no_gpu` | bool | `false` | Force CPU-only even if GPU available |
| `llm.builtin.inference_timeout_secs` | u64 | `600` | Inference timeout in seconds (min 60) |
| `llm.custom_api.base_url` | string | `""` | Custom API endpoint URL |
| `llm.custom_api.api_key` | string | `""` | Custom API key |
| `llm.custom_api.model_name` | string | `""` | Custom API model name |
| `security.require_workspace_write_approval` | bool | `true` | Require approval for workspace writes in Interactive mode |
| `default_mode` | string | `"interactive"` | Default security mode on startup |
| `default_model` | string | `"qwen3_1.7b"` | Default model ID |
| `workspace_path` | string | `""` | Workspace path override (empty = use portable default) |
| `network_whitelist` | string[] | `[]` | Allowed network hosts (empty = no restriction) |
| `command_blacklist` | string[] | _see below_ | Commands blocked by security policy |
| `default_behavior` | string | `"normal"` | Default behavior mode |
| `max_iterations` | u32 | `999999` | Maximum loop iterations before forced halt |
| `command_timeout_secs` | u64 | `60` | Default command timeout in seconds |
| `max_backups` | u32 | `10` | Maximum number of backups per file |

### Default Command Blacklist

```toml
command_blacklist = [
    "rm -rf /",
    "mkfs",
    "dd if=",
    "format ",
    "del /f /s /q",
    "rd /s /q",
    "reg delete",
    "sc delete",
    "systemctl disable",
    "shutdown",
    "reboot",
]
```

Matches are token-aware to prevent indirection bypass. You can customize the blacklist in `config/settings.toml` or use `config/models.toml` for model-specific settings.

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
2. **Action sequences**: Both store structured action sequences as JSON arrays (ordered list of action types used)
3. **During execution**: 2+ consecutive failures → automatically recorded as a mistake
4. **During recall**: Before a task, the system searches solutions, mistakes, and notes for keyword overlap with the current task
5. **Plan template matching**: Past solutions with PLAN.md structures are detected and presented as plan templates
6. **Pattern reinforcement**: Repeated successes boost confidence scores (capped at 50 to prevent saturation)
7. **Memory context**: Relevant memories are injected into the system prompt as context for the LLM

### Memory Injection in System Prompt

Before each task, memory is searched for relevant past experiences:
```
Related past experiences:
  [Solution] Fixed a parser bug (confidence: 0.85)
    → Used: read_file, search_files, write_file, execute_command
  [Mistake] File not found error (frequency: 3)
    → Lesson: Always check file existence before reading
```

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

### Memory ID Format

Memory IDs follow a pattern: `<type>_<timestamp>_<random_suffix>`
- `sol_1718000000_123456789` — Solution entry
- `mist_1718000000_987654321` — Mistake entry
- `pat_1718000000_555555555` — Pattern entry
- `note_1718000000_111111111` — Note entry
- `hist_1718000000_444444444` — Task history entry
- `pref_1718000000_333333333` — Preference entry

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

Every 5 iterations, the session state is saved to `sessions/<uuid>/checkpoints/`. Interrupted sessions can be resumed from the last checkpoint. Checkpoints include:
- Current conversation state
- Plan state (if any)
- Iteration count
- Action history
- Memory state
- Variables

### Session Directory Structure

```
sessions/
├── <uuid>/
│   ├── session.json          # Session metadata (task, mode, status)
│   ├── conversation.json     # Full conversation history
│   ├── checkpoints/
│   │   ├── checkpoint_5.json
│   │   ├── checkpoint_10.json
│   │   └── ...
│   └── variables.json        # Session variables
```

### Session Versioning

Sessions carry a `version` field (currently version 1). If the session format changes in a future release, automatic migration runs on resume to upgrade old sessions. This ensures forward compatibility without breaking existing sessions.

---

## Audit Logging

All actions are logged to JSONL files in `logs/`:

```
logs/
├── audit_YYYY-MM-DD.jsonl       # All actions
├── unrestricted_audit_YYYY-MM-DD.jsonl  # Unrestricted mode actions (extra detail)
└── errors.jsonl                 # Error events
```

### Audit Entry Format

Each entry includes:
- `timestamp` — ISO 8601 timestamp
- `action_type` — The tool/action name
- `action_description` — Human-readable action description
- `security_mode` — Current security mode
- `approval_status` — Allowed, Blocked, or Rejected
- `model` — LLM model used
- `session_id` — Session UUID
- `iteration` — Loop iteration number
- `result` — Success or error summary

### View Logs

```bash
# View today's audit log
vo1d logs

# Logs are stored as JSONL — one JSON object per line
# Use any JSON parser to analyze them:
cat logs/audit_2026-06-09.jsonl | jq '.action_type' | sort | uniq -c
```

---

## CLI Reference

```
Usage: vo1d [OPTIONS] [COMMAND]

Commands:
  task          Execute a one-shot task
  chat          Start an interactive chat session
  models        List and manage models
  sessions      List and manage saved sessions
  train         Run a training curriculum (e.g. `vo1d train 00_hello_world`)
  test          Run system tests (`vo1d test compression`, `vo1d test tools`)
  memory        Manage VO1D's memory and learning
  config        View current configuration
  settings      View or change settings (`vo1d settings set key value`)
  benchmark     Run GPU vs CPU inference benchmark
  logs          View audit logs

Options:
      --model <MODEL>           Override default model
      --mode <MODE>             Security mode (safe, interactive, power-user, autonomous, unrestricted)
      --behavior <BEHAVIOR>     Behavior mode (normal, fix, research, refactor, tdd)
      --workspace <DIR>         Custom workspace directory
      --unrestricted            Enable unrestricted mode (absolute autonomy)
      --yes, --jes              Auto-approve all actions
      --debug                   Enable verbose debug tracing
      --resume <ID>             Resume a session by ID
  -h, --help                    Print help
  -V, --version                 Print version

Subcommands:
  test compression              Test context compression
  test tools                    Test all tool registrations and executor dispatch
  settings                      View current settings
  settings list                 View current settings
  settings set <key> <value>    Change a setting (model-aware validation)
  benchmark [model]             Run GPU vs CPU inference comparison benchmark
  train --all --on-failure <POLICY>  Autotrain failure policy (stop, skip, retry)
  models list                   List available models
  models install <id>           Download and install a model
  models remove <id>            Remove an installed model
  models profile                Show hardware profile and compatible models
  memory list                   Show memory stats and entries
  memory show <id>              Show a specific memory by ID
  memory delete <id>            Delete a specific memory by ID
  memory clear [type]           Clear memories (all, solutions, mistakes, notes, patterns, history)
  memory add <content>          Add a custom note (--tags for comma-separated tags)

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

### Hardware Tier System

| Tier | RAM | Recommended Context | Compatible Models |
|------|-----|-------------------|-------------------|
| **Low** | < 8 GB | 4096 | 1-3B parameter models |
| **Medium** | 8-16 GB | 8192 | 3-7B parameter models |
| **High** | 16-32 GB | 16384 | 7-13B parameter models |
| **Ultra** | 32+ GB | 32768 | 13B+ parameter models |

The profiler checks: CPU model and core count, total and available RAM, GPU vendor/name/dedicated VRAM, overall hardware tier (Low/Medium/High), and compatible models from the registry. When built with `--features vulkan`, `LlamaBackend::supports_gpu_offload()` is queried at model load time to confirm Vulkan acceleration is available.

### GPU Detection

When built with Vulkan support, the profiler also checks:
- GPU vendor (AMD, NVIDIA, Intel)
- GPU name/model
- Dedicated VRAM (if available)
- Vulkan driver support (at model load time, via llama.cpp)

Models are marked compatible if they fit within available RAM (or VRAM with GPU offload).

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
- **Agent Loop**: Iterates without a hard cap — context compression keeps conversations manageable. Each iteration: model generates response (streamed in real time — `<think>` blocks as `─── Reasoning ───`, tool calls as `─── Tool ───`, results as `─── Result ───`) → parser extracts JSON action → security evaluates → executor runs → result appended → self-correction checks → plan tracking → skill resolution → behavior enforcement → context compression if needed. Auto-restarts if model repeats same action 5+ times. Only one tool action per iteration is processed.
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
- **Download Manager**: Progress tracking and automatic resume for model downloads
- **Hardware Profiler**: `sysinfo`-based CPU/GPU/RAM detection with tier classification and model compatibility matching.
- **Session Manager**: UUID-based session tracking, checkpointing every 5 iterations, auto-migration for version upgrades.
- **Audit Logger**: JSONL-based structured logging with per-action entries including security context and approval status.

### Data Flow

```
User Input
    → CLI parses args (task/chat/train)
    → AppContext initialized (config, paths, backends)
    → Session created (UUID, timestamp, task)
    → Agent loop starts
        → Build system prompt (docs + memory + tools + skills)
        → Check context usage (summarize or prune if needed)
        → Call LLM (stream response to console)
        → Parse action from response (JSON or tool calls)
        → Check for loops, reasoning quality, single action enforcement
        → Evaluate security policy (Allow/Ask/Block)
        → Execute tool action
        → Record result in conversation
        → Track plan progress (auto-advance if step done)
        → Save checkpoint (every 5 iterations)
        → Repeat until Finish or limit
    → Session saved
    → Final output displayed to user
```

---

## Portability

vo1d is fully portable:
- All data stored relative to the executable
- No registry entries, no system-wide install paths
- No dependencies on system-wide package installations
- Runs directly from a USB drive or network share
- **Config**: `<exe-dir>/config/settings.toml`
- **Models**: `<exe-dir>/models/llamacpp/`
- **Sessions**: `<exe-dir>/sessions/<uuid>/`
- **Logs**: `<exe-dir>/logs/`
- **Skills**: `<exe-dir>/skills/<name>.json`
- **Memory**: `<exe-dir>/memory/*.json`
- **Docs**: `<exe-dir>/docs/*.md` (optional, embedded defaults used if missing)
- **Curriculum**: `<exe-dir>/curriculum/*.json` (optional, all curricula embedded in binary)

---

## Development

### Running Tests

```bash
# Run all unit tests (85+ tests)
cargo test

# Run tests without any feature flags (tests that don't require backends)
cargo test -p vo1d --lib

# Run integration tests (6 test files)
cargo test --test integration
cargo test --test skill_ops
cargo test --test tool_parser
cargo test --test security
cargo test --test models
cargo test --test file_ops

# Run tests with a specific feature
cargo test --features llamacpp-builtin

# Run all tests including those requiring features
cargo test --features full

# Build with Vulkan GPU support (requires VULKAN_SDK)
cargo build --features vulkan --release

# Build with all features (requires VULKAN_SDK)
cargo build --features full --release

# Check compilation without Vulkan
cargo check --features "llamacpp-builtin,ollama,lmstudio,llamacpp-server,custom-api"

# Run with verbose output
cargo run --features llamacpp-builtin -- task "hello" --debug
```

### Code Style

- Follow existing patterns (see `src/` structure)
- `serde` derives for all data structs
- `anyhow::Result` for error propagation
- `tracing` for logging (not `println`/`eprintln` in library code)
- CLI output uses `print!`/`eprintln!` only in `loop_.rs`, guarded by `tui_mode`
- No unsafe code unless absolutely necessary (see `stderr_guard.rs`)
- All errors implement `std::error::Error` with `#[source]` chain preservation
- Use `async-trait` for async trait methods
- `tokio` as async runtime throughout

### Project Conventions

- **Feature flags**: Backend features are additive; each enables a separate LlmBackend impl
- **Configuration**: TOML with serde defaults — all settings have safe defaults
- **Error handling**: `thiserror` for library errors, `anyhow` for application-level propagation
- **Testing**: Unit tests alongside source files, integration tests in `tests/` directory
- **Documentation**: Markdown docs in `docs/` loaded at runtime and injected into system prompt
- **Dependencies**: Minimal and deliberate — avoid adding heavy dependencies without careful consideration

### Build Configuration

The workspace uses these key dependencies:

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime (full features) |
| `clap` | CLI argument parsing (derive) |
| `serde` / `serde_json` / `toml` | Serialization |
| `reqwest` | HTTP client (rustls-tls, no native deps) |
| `llama-cpp-2` | Built-in LLM inference (optional) |

| `tracing` / `tracing-subscriber` | Structured logging |
| `thiserror` / `anyhow` | Error handling |
| `sysinfo` | Hardware profiling |
| `regex` | Pattern matching |
| `schemars` | JSON schema generation |
| `indicatif` | Progress bars |
| `uuid` / `chrono` | Session IDs and timestamps |

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `llama_context: n_batch = 512` then crash | Delete `config/settings.toml` or set `batch_size = 4096` |
| `Model file not found` | Run `vo1d models install <model-id>` |
| `spawn_blocking panicked` in `builtin.rs` | Check RAM usage, reduce `context_size` or `gpu_layers` |
| Vulkan not found at runtime | Ensure a compatible GPU and Vulkan driver are installed. Build with `--features vulkan`. `vulkan-1.dll` must be on `PATH` |
| Vulkan build fails on Windows | Install Vulkan SDK from https://vulkan.lunarg.com/ and set `VULKAN_SDK` environment variable in the same terminal |
| `VULKAN_SDK` not set error in build | The `vulkan` and `full` features require `VULKAN_SDK`. Run `set VULKAN_SDK=C:\VulkanSDK\1.4.309.0` in the same terminal (adjust path/version) then retry. Or build without Vulkan: `cargo build --features "llamacpp-builtin,ollama,lmstudio,llamacpp-server,custom-api" --release` |
| Model keeps generating infinite tool calls | Check ChatML prompt format. Qwen3 expects `<|im_start|>` tags |
| LLM output garbled with C library stderr | Verify `stderr_guard.rs` compiled correctly |
| Build fails with linker errors on Windows | Need MSVC build tools installed. Ensure `+crt-static` is NOT in `.cargo/config.toml` |
| Curriculum file not found on disk | All curricula are embedded in the binary — run `vo1d train <name>` even without `curriculum/` directory |
| Skill file won't load | Check JSON syntax and that name matches `^[a-z0-9][a-z0-9-]{0,63}$` |
| Session won't load after update | Session migration runs automatically — old sessions are upgraded on resume |
| `cargo build` is slow | Use `cargo build --release` only for final builds; `cargo build` (debug) is faster for development |
| GPU layers not working | Set `gpu_layers = -1` in settings; verify `llama_supports_gpu_offload()` returns true at runtime |
| Python scripts in curriculum fail | Curriculum sandboxes use the system Python — ensure Python is installed and on PATH |
| Web search returns no results | DuckDuckGo may rate-limit. Wait a few seconds and try again with fewer results |
| Model download fails mid-way | Resume download by running the install command again — it skips existing files |
| High RAM usage during inference | Reduce `context_size`, `batch_size`, or `gpu_layers` in settings |
| `LNK1318` PDB file error on Windows | Pre-existing toolchain race condition — run the build command again |
| CMake not found during build | Install CMake 3.20+ and ensure it's on PATH |
| `llama-cpp-sys-2` build panic at `build.rs:768:58`: "Please install Vulkan SDK" | `VULKAN_SDK` env var is missing or not visible to the build. Run `set VULKAN_SDK=C:\VulkanSDK\1.4.309.0` in the same terminal, or disable Vulkan: `cargo build --features "llamacpp-builtin,ollama,lmstudio,llamacpp-server,custom-api" --release` |

---

## Performance Tuning

### Inference Speed

| Setting | Effect | Recommendation |
|---------|--------|----------------|
| `threads` | CPU threads for inference | Set to `-1` (auto) or match physical core count |
| `batch_size` | Prompt processing batch size | Higher = faster but more RAM. Start at 4096 |
| `gpu_layers` | Layers offloaded to GPU | `-1` = all (fastest with GPU), `0` = CPU only, `N` = first N layers |
| `context_size` | Context window | Higher = less compression but more RAM. 8192 is a good default |
| `max_tokens` | Max generation tokens | Higher = more output but slower. 2048 is sufficient for most tasks |

### Memory Usage

- **Model size**: GGUF quantized models use approximately their file size in RAM (e.g., a 4GB file uses ~4GB RAM)
- **Context overhead**: Each token of context uses ~2-4 KB. 8192 context ≈ 16-32 MB extra
- **Batch overhead**: Higher `batch_size` uses more RAM during prompt processing. Reduce for low-RAM systems
- **GPU offload**: Offloading to GPU reduces CPU RAM usage but requires VRAM on the GPU

### Recommended Settings by Hardware

**Low-end (4-8 GB RAM, no GPU):**
```toml
[llm.builtin]
context_size = 4096
batch_size = 2048
gpu_layers = 0
threads = 2
```

**Mid-range (8-16 GB RAM, integrated GPU):**
```toml
[llm.builtin]
context_size = 8192
batch_size = 4096
gpu_layers = -1
threads = -1
```

**High-end (16+ GB RAM, dedicated GPU):**
```toml
[llm.builtin]
context_size = 16384
batch_size = 8192
gpu_layers = -1
threads = -1
```

---

## Model Registry

vo1d includes a built-in catalog of compatible models defined in `config/default_models.toml` (default) and `config/models.toml` (user overrides). The registry supports models from Hugging Face in GGUF format.

### Adding a Custom Model

Edit `config/models.toml`:
```toml
[[model]]
id = "my-model"
name = "My Custom Model"
provider = "huggingface"
download_url = "https://huggingface.co/author/model/resolve/main/model.gguf"
filename = "model.gguf"
# sha256 field removed - no longer used for verification
size_bytes = 2000000000
min_ram_gb = 4.0
context_length = 4096
supports_tools = false
native_tools = false                 # set true if model supports structured function calling
quantization = "Q4_K_M"
```

### Model Fields Reference

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier used in CLI commands |
| `name` | string | Human-readable model name |
| `provider` | string | Source provider (currently only `huggingface`) |
| `download_url` | string | Direct download URL for the GGUF file |
| `filename` | string | Local filename for the downloaded model |
| `sha256` | string | (Removed - no longer used for verification) |
| `size_bytes` | integer | File size in bytes (used for compatibility checking) |
| `min_ram_gb` | float | Minimum RAM required (in GB) |
| `context_length` | integer | Maximum context length in tokens |
| `supports_tools` | boolean | Whether the model works with tool calling |
| `native_tools` | boolean | Whether the model supports structured function calling natively |
| `quantization` | string | Quantization format (e.g., "Q4_K_M", "Q5_K_M", "Q8_0") |

### Native Tool Support

The `native_tools` field indicates whether the model supports structured function calling natively (e.g., Qwen 2.5, Llama 3.1+, Gemma 3/4, Mistral). The parser handles both `{"action": ...}` and OpenAI-style `{"name": ..., "arguments": {...}}"` formats, so native tool schema injection is disabled by default to keep prompts small — the parser extracts tool calls from either format directly from model output.

### Model Compatibility

Models are automatically filtered by hardware profile:
- RAM check: `size_bytes <= available_ram`
- Context check: `context_length <= recommended_context`
- GPU check: If GPU is available, larger models may be compatible via GPU offload

---

## Security & Robustness Details

### Token-Aware Command Blacklist

The command blacklist uses token-aware matching to prevent indirection bypass. For example:
- `rm -rf /` → blocked (matches blacklist)
- `rm -rf /tmp` → allowed (different tokens)
- `rm -rf /` with shell expansion → still blocked (token-level matching)

This prevents the model from bypassing the blacklist through shell tricks, variable expansion, or character escaping.

### Workspace Sandbox

The workspace sandbox ensures all file operations stay within the allowed directory:
- `resolve_workspace_path()` validates resolved paths against the workspace root
- Directory traversal attacks (`../../../etc/passwd`) are detected and blocked
- Outside-workspace access is controlled by security mode (Safe/Interactive block, PowerUser/Autonomous/Unrestricted prompt or allow)

### Output Size Limits

- Tool return values are truncated at 1 MB to prevent context overflow
- Command output >2000 characters is truncated with a length marker in the conversation
- The full output is stored in the session result but truncated for the LLM context

---

## Backup and Recovery

### Session Recovery

If vo1d crashes or is interrupted during a task:
1. Checkpoints are saved every 5 iterations
2. Use `vo1d sessions` to list available sessions
3. Resume with `vo1d --resume <session-id> task "continue from checkpoint"`
4. The session continues from the last checkpoint

### File Recovery

- `show_changes` lists all file modifications in the current session
- `restore_backup` reverts a single file to its last git state
- For non-git workspaces, the change tracking shows recently modified files

### Data Integrity

- All model downloads include progress tracking and automatic resume
- Skills are saved atomically (write to temp file, then rename)
- Session state is written atomically to prevent corruption
- Memory serialization is capped at 50 MB to prevent resource exhaustion

---

## License

GNU General Public License v3.0
