# File Operations

VO1D provides built-in file operations that are preferred over shell commands.

## Reading Files

Use `read_file` to read file contents. Supports line ranges:
- `{"action": "read_file", "path": "file.txt"}` — read entire file
- `{"action": "read_file", "path": "file.txt", "start_line": 5, "end_line": 10}` — read lines 5-10

## Writing Files

Use `write_file` to create or overwrite files. Set `append: true` to add content:
- `{"action": "write_file", "path": "file.txt", "content": "hello"}` — create/replace
- `{"action": "write_file", "path": "file.txt", "content": " appended", "append": true}` — append

## File Metadata

Use `file_metadata` to check file size, modification time, type:
- `{"action": "file_metadata", "path": "file.txt"}`

## Copying

Use `copy_file` to duplicate files:
- `{"action": "copy_file", "source": "a.txt", "destination": "b.txt"}`

## Deleting

Use `delete_file` to remove files. Supports pattern-based batch delete:
- `{"action": "delete_file", "path": "file.txt"}` — single file
- `{"action": "delete_file", "path": ".", "pattern": "*"}` — delete all files
- Use `*` not `*.*` to match all files (`*.*` misses files without a dot)

## Best Practices

- Always list a directory before deleting files
- Use built-in file actions instead of shell commands when possible
- Prefer `write_file` over `echo > file` for reliable writes
- Check file existence with `file_metadata` before reading
