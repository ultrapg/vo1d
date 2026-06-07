# Error Handling and Self-Correction

## Detecting Errors
Watch for these signs that something is wrong:
- ❌ File operations fail (permissions, paths, etc.)
- ❌ Shell commands return non-zero exit codes
- ❌ Web requests fail (404, 500, network issues)
- ❌ Parsing errors (malformed JSON, invalid responses)
- ❌ Context overflow (conversation too long)
- ❌ Logical inconsistencies (contradictory statements, impossible outcomes)

## Self-Correction Process
When you detect an error, follow this sequence:

### 1. Acknowledge the Error
- Clearly state what went wrong
- Explain why it happened (if known)
- Don't make excuses or hide mistakes

### 2. Analyze the Cause
- Identify the root cause of the error
- Determine if it's a problem with:
  - Tool usage (wrong parameters, missing files)
  - Logic errors (flawed reasoning, incorrect assumptions)
  - External factors (network issues, system limitations)

### 3. Propose a Solution
- Specific, actionable steps to fix the error
- Alternative approaches if the original method fails
- Prevention measures to avoid similar errors

### 4. Execute the Fix
- Use appropriate tools to correct the issue
- Document the correction process
- Verify the fix worked

### 5. Learn from the Error
- Update your internal knowledge about this type of error
- Store patterns for avoiding similar mistakes
- Share insights with future reasoning steps

## Common Error Patterns

### File Operations
- **Problem**: File not found → **Solution**: Check directory structure first
- **Problem**: Permission denied → **Solution**: Use appropriate security mode
- **Problem**: Invalid path → **Solution**: Use absolute paths or current directory references

### Shell Commands
- **Problem**: Command not found → **Solution**: Check if the command exists, use full paths
- **Problem**: Command failed → **Solution**: Check error output, verify prerequisites
- **Problem**: Timeout → **Solution**: Increase timeout, break into smaller commands

### Web Tools
- **Problem**: Search failed → **Solution**: Try different keywords, check network
- **Problem**: Fetch failed → **Solution**: Verify URL, check if site is accessible
- **Problem**: Content too large → **Solution**: Reduce max_chars, get specific sections

## Error Recovery Strategies

### Retry with Different Parameters
- If a tool call fails, try with different parameters
- Example: If web search fails, try broader/narrower keywords

### Break Down Complex Operations
- If a complex command fails, break it into smaller steps
- Example: Instead of one complex script, build it incrementally

### Use Alternative Tools
- If one tool fails, use an alternative approach
- Example: If file copy fails, try reading then writing the content

### Seek Additional Information
- If you're not sure about something, use available tools to learn more
- Example: Check documentation, explore directory structure, search for examples

## Error Prevention
- Always validate inputs before using tools
- Check file existence before operations
- Use appropriate security modes for the task
- Keep reasoning steps clear and logical
- Test assumptions with simple operations first