# Directory Operations

## Creating Directories

Use `create_directory` to create one or more nested directories:
- `{"action": "create_directory", "path": "src/components"}` — creates src/ and src/components/

## Listing Directories

Use `list_directory` to see contents. Supports glob filtering:
- `{"action": "list_directory", "path": "."}` — list all
- `{"action": "list_directory", "path": "src", "pattern": "*.rs"}` — filter by pattern

## Searching

Use `search_files` to find files matching a glob pattern recursively:
- `{"action": "search_files", "pattern": "*.json"}` — find all JSON files
- `{"action": "search_files", "pattern": "*.txt", "path": "docs"}` — limit search to path
- Use `*` for wildcard, `**` for recursive matching

## Best Practices

- write_file automatically creates parent directories — no need to call create_directory first
- List before searching to understand the structure
- Use `list_directory` with pattern for targeted listings
- Search is recursive by default, use path to limit scope
