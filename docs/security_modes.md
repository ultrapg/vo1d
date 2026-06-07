# Security Modes Reference

## Overview
vo1d provides five security modes that control what actions the agent can perform. Each mode balances functionality with safety according to your use case.

## Mode Descriptions

### Safe Mode (Default)
**Purpose**: Maximum safety, read-only operations  
**Use Case**: Exploring codebases, reading documentation, understanding projects  
**Permissions**: 
- ✅ File reading
- ✅ Directory listing
- ✅ Web search/fetch
- ✅ Shell commands (read-only)
- ❌ File writing/modification
- ❌ File deletion
- ❌ Command execution that modifies system

**Example**:
```bash
vo1d --mode safe "Analyze this codebase structure"
```

### Interactive Mode
**Purpose**: User-controlled execution with confirmation  
**Use Case**: Development tasks, file management, when you want to approve each action  
**Permissions**:
- ✅ All file operations
- ✅ All shell commands
- ✅ Web tools
- ❌ Auto-approval - requires user confirmation

**Behavior**:
- Asks for confirmation before any file modification or command execution
- Provides clear explanations of what will be done
- Waits for explicit user approval

**Example**:
```
I will create a new file named 'config.json'. Do you want to proceed? (y/n)
```

### PowerUser Mode
**Purpose**: Powerful execution with warnings for destructive operations  
**Use Case**: Development, system administration, when you trust the agent but want awareness of destructive actions  
**Permissions**:
- ✅ All operations
- ❌ Auto-approval for destructive operations (warns but doesn't ask)

**Behavior**:
- Warns about potentially destructive operations (deletes, overwrites)
- Executes immediately after warnings
- Provides detailed explanations of actions

**Example**:
```
WARNING: This will delete the entire 'build' directory. Proceeding...
```

### Autonomous Mode
**Purpose**: Fully autonomous execution  
**Use Case**: Long-running tasks, automation, when you want the agent to work independently  
**Permissions**:
- ✅ All operations
- ✅ Auto-approval for all actions

**Behavior**:
- Executes actions without user intervention
- Reports progress and results
- Handles errors automatically
- Uses context to make informed decisions

**Example**:
```bash
vo1d --mode autonomous "Set up the entire development environment"
```

### YOLO Mode (You Only Live Once)
**Purpose**: Maximum freedom, no restrictions  
**Use Case**: Experimental work, testing, when you understand the risks  
**Permissions**:
- ✅ All operations
- ❌ No warnings or restrictions

**Behavior**:
- Executes any action without questioning
- No safety checks or warnings
- Full responsibility lies with the user

**Example**:
```bash
vo1d --mode yolo "Do whatever is needed to complete the task"
```

## Mode Selection Guidelines

### Choose Safe Mode When:
- You're exploring unfamiliar codebases
- You're reading documentation or examples
- You want to understand a project without modifying it
- You're working with sensitive data that shouldn't be changed

### Choose Interactive Mode When:
- You're developing new features
- You're managing files and directories
- You want to understand each action before it happens
- You're learning how the agent works

### Choose PowerUser Mode When:
- You're experienced with the agent
- You're doing development work
- You trust the agent's judgment
- You want efficiency but still want awareness of destructive actions

### Choose Autonomous Mode When:
- You're running long, complex tasks
- You need automation without interruption
- You're confident in the agent's capabilities
- You're doing batch processing or repetitive work

### Choose YOLO Mode When:
- You're testing the agent's limits
- You're doing experimental work
- You fully understand the risks
- You need maximum flexibility for testing

## Mode Configuration Modes can be set in three ways:

### 1. Command Line
```bash
vo1d --mode safe "task"
vo1d --mode interactive "task"
vo1d --mode poweruser "task"
vo1d --mode autonomous "task"
vo1d --mode yolo "task"
```

### 2. Settings File
```toml
[security]
mode = "interactive"
```

### 3. Environment Variable
```bash
export VO1D_MODE=autonomous
vo1d "task"
```

## Mode Comparison

| Mode | Auto-Approve | Warnings | Read-Only | Full Access | Use Case |
|------|--------------|----------|-----------|-------------|----------|
| Safe | ❌ | ❌ | ✅ | ❌ | Exploration, reading |
| Interactive | ❌ | ✅ | ❌ | ✅ | Development, learning |
| PowerUser | ❌ | ✅ | ❌ | ✅ | Development, admin |
| Autonomous | ✅ | ❌ | ❌ | ✅ | Automation, long tasks |
| YOLO | ✅ | ❌ | ❌ | ✅ | Testing, experiments |

## Security Best Practices

### For Production Use
- Use Safe or Interactive mode for sensitive operations
- Always review the agent's plan before execution
- Use checkpoints for long-running tasks
- Monitor the agent's progress regularly

### For Development
- Start with Interactive mode to understand the agent's approach
- Move to PowerUser mode once you're comfortable
- Use Autonomous mode for repetitive tasks

### For Testing
- Use YOLO mode for experimental work
- Test in isolated environments first
- Always have backups of important data

## Mode Transition

You can change modes during a session:
```bash
# Start in safe mode
vo1d --mode safe "explore the codebase"

# Switch to interactive mode for file operations
vo1d --mode interactive "create a new configuration file"
```