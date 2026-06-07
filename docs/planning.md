# Planning Mode

When tackling complex tasks, create a PLAN.md in the workspace root to structure your work.

## PLAN.md Format

The plan file uses markdown headers and checkboxes:

```markdown
## Plan

## Step 1: Analyze Requirements
- [ ] Read existing code to understand structure
- [ ] Identify key files that need changes

## Step 2: Implement Changes
- [ ] Modify src/core/module.rs
- [ ] Add new function `process_data`
```

Each `## Step N:` header defines a step. Steps are processed sequentially. The `[ ]` or `[x]` shows completion status.

## When to Plan

Create a plan when:
- The task requires modifying 3+ files
- The task involves multiple phases (analyze, implement, test)
- You encounter complex logic that benefits from breaking down
- You receive a "Replanning suggested" message from the system

## Following the Plan

- Work through steps one at a time
- The system tracks which step you're on and monitors progress
- After 10 iterations on the same step, evaluate if you need to adjust
- After 3 consecutive evaluation failures on a step, consider replanning
- Update PLAN.md checkboxes as you complete sub-tasks

## Plan Recovery

If stuck on a step:
1. Re-read the plan to ensure you understand the task
2. Break the current step into smaller sub-steps
3. Try a different approach than what you've been attempting
4. Update PLAN.md with adjusted approach
