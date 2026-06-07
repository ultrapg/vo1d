# Self-Improvement Guide

VO1D can improve its performance over time through memory retention and error analysis.

## Learning from Errors

When an action fails, VO1D analyzes the error and provides targeted suggestions:

1. **File not found** — suggests listing the directory first
2. **Permission denied** — suggests checking security mode or using a different operation
3. **Command not found** — notes OS differences (dir vs ls)
4. **Timeout** — suggests simplifying or increasing timeout
5. **Invalid JSON** — provides JSON syntax tips
6. **Network errors** — suggests checking URL or using web_search

## Pattern Recognition

VO1D tracks repeated failures and generates correction prompts:
- After 3 consecutive failures with the same action, a correction prompt is injected
- The prompt includes a specific suggestion for the failing action type
- Success with an action clears its failure counter

## Memory System

Cross-session memory stores:
- Task history with outcomes
- Learned preferences
- High-confidence patterns

Memory is injected into the system prompt at the start of each session, allowing VO1D to build on past experience.

## Tips for Better Learning

1. Finish tasks explicitly with `finish` to record the outcome
2. Try different approaches when an action fails
3. Use built-in actions (file ops) over shell commands
4. List directories before modifying files
5. Use `*` not `*.*` in glob patterns
