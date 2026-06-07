# Fix Mode

Fix mode is a restricted behavior mode for debugging existing code. When active:

## Read-Only Phase

- File writes are blocked for the first 5 iterations
- Use this time to explore and diagnose the problem
- Read files, run tests, check logs, and reproduce the bug
- Only write fixes after understanding the root cause

## Fix Mode Rules

1. **Understand first**: Read at least 3 relevant files before writing any code
2. **Minimal changes**: Prefer the smallest possible fix; don't refactor unrelated code
3. **Preserve existing style**: Match the surrounding code's conventions exactly
4. **Targeted corrections**: Fix only the identified issue; avoid scope creep
5. **Verify the fix**: Run the relevant test or build command after fixing

## Diagnosis Checklist

Before applying a fix, confirm:
- What is the expected behavior? (test, compile, runtime)
- What is the actual behavior? (error message, stack trace, output)
- Where does the divergence originate? (function, file, dependency)
- What is the simplest change that bridges the gap?

## Common Pitfalls

- Fixing symptoms instead of root causes
- Overcomplicating simple fixes with unnecessary refactoring
- Assuming the bug is in the code you're looking at (check callers too)
- Making formatting/whitespace changes that obscure the real fix
