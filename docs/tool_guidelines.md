# Tool Usage Guidelines

## General Principles
- Use tools systematically: plan → act → observe → reason → repeat
- Always include reasoning before and after tool usage
- Provide clear, actionable explanations of what you're doing and why
- Handle errors gracefully and explain what went wrong

## File Operations
- **read_file**: Always specify full paths. Check file existence first.
- **write_file**: Provide complete content. Include proper formatting and structure.
- **delete_file**: Double-check paths before deletion. Warn about destructive operations.
- **list_directory**: Use to explore directory structure before file operations.
- **search_files**: Use pattern matching (e.g., `*.rs`, `*.toml`) to find relevant files.
- **copy_file**: Ensure source and destination paths are correct.
- **mkdir**: Create parent directories as needed for nested paths.

## Shell Commands
- **execute_command**: Use for system operations, file management, compilation, testing
- Always quote paths with spaces: `"path with spaces/file.txt"`
- Use `&&` for sequential commands that depend on previous success
- Use `&` for independent commands that can run in parallel
- Include clear error handling and output interpretation

## Web Tools
- **web_search**: Use for general web queries. Limit results with `num_results` (5-10 max).
- **web_fetch**: Use for specific pages. Set `max_chars` to control response size.
- Always verify fetched content before processing
- Convert HTML content to markdown for better readability

## Context Compression
- When context is compressed, reference stored summaries from memory
- Important information is preserved in memory even when messages are truncated
- Ask for clarification if critical context is missing due to compression

## Security Modes
- **Safe**: Only read operations allowed
- **Interactive**: Confirm before any file modifications or commands
- **PowerUser**: Full access with warnings for destructive operations
- **Autonomous**: Auto-approve all actions (use with caution)
- **YOLO**: No restrictions (completely unrestricted access)