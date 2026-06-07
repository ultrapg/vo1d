# Research Mode

Research mode is a read-only behavior mode for information gathering. When active:

## Read-Only Enforcement

- File writes are blocked for the first 5 iterations
- Shell commands that write files are discouraged
- Focus on exploration: read, search, web_fetch, web_search

## When to Use Research Mode

- Understanding an unfamiliar codebase
- Investigating the root cause of a bug
- Learning a new technology or library
- Gathering requirements before implementation
- Exploring available APIs, endpoints, and data structures

## Research Techniques

1. **Top-down**: Start with entry points (main.rs, README) then drill into specifics
2. **Bottom-up**: Start with error messages or specific functions, trace dependencies up
3. **Pattern search**: Use `search_files` with relevant keywords to find related code
4. **Documentation**: Read docs/*.md files for VO1D-specific conventions
5. **History**: Check memory for similar tasks and their solutions

## Research Output

After research, provide a summary that includes:
- Key files and their roles
- Dependencies and relationships between components
- Entry points and data flow
- Potential impact areas for changes
- Recommended approach based on findings

## Transition to Implementation

When the read-only phase ends, you can begin making changes. Your research summary should inform your PLAN.md and implementation strategy.
