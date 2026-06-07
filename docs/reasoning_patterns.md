# Reasoning Patterns and Best Practices

## Effective Reasoning Structure

### Before Action: Planning Phase
```
I need to [objective]. To achieve this, I should:
1. [Step 1 reason]
2. [Step 2 reason]
3. [Step 3 reason]

I will start by [first action] because [reason].
```

### During Action: Execution Phase
```
Executing [action] with [parameters]...

[Action execution result]

The [action] [succeeded/failed] because [reason].
```

### After Action: Reflection Phase
```
Now I need to [next step] because [reason].

Observations so far:
- [Observation 1]
- [Observation 2]
- [Observation 3]

Next, I will [next action] to [achieve goal].
```

## Reasoning Best Practices

### 1. Clear Goal Orientation
- Always start with a clear understanding of the objective
- Break down complex goals into manageable sub-goals
- Prioritize actions based on importance and dependencies

### 2. Logical Progression
- Use step-by-step reasoning with clear connections between steps
- Explain the "why" behind each action
- Show how each step contributes to the overall goal

### 3. Evidence-Based Reasoning
- Base decisions on actual results from previous actions
- Use observations to inform next steps
- Update reasoning based on new information

### 4. Proactive Problem Solving
- Anticipate potential issues before they occur
- Have backup plans ready
- Learn from mistakes and adapt approach

### 5. Clear Communication
- Explain technical concepts in simple terms
- Provide context for why actions are being taken
- Be honest about limitations and uncertainties

## Common Reasoning Patterns

### Exploration Pattern
```
I need to explore the workspace to understand the current state. I'll start by listing the top-level directory to see what's available.
```

### Problem-Solving Pattern
```
The task is to [problem]. I need to:
1. Understand the current state
2. Identify what needs to be changed
3. Implement the solution
4. Verify the result

First, let me [exploration step].
```

### Iterative Improvement Pattern
```
I'll implement a basic solution first, then iteratively improve it based on feedback and testing.

Step 1: Create minimal version
Step 2: Test and identify improvements
Step 3: Implement improvements
Step 4: Final verification
```

### Tool Selection Pattern
```
To achieve [goal], I need to choose the right tool. Let me consider:
- [Tool A] is good for [reason]
- [Tool B] is better for [reason]
- [Tool C] is specifically designed for [reason]

I'll use [chosen tool] because [reason].
```

## Anti-Patterns to Avoid

### 1. Jumping to Conclusions
- ❌ "I'll just delete the file and recreate it"
- ✅ "I need to understand what's in the file first to avoid losing important data"

### 2. Circular Reasoning
- ❌ "I need to do X because I need to do X"
- ✅ "I need to do X because it will help achieve Y, which is necessary for the overall goal"

### 3. Overly Broad Actions
- ❌ "I'll fix everything"
- ✅ "I'll focus on the specific issue [X] first, then address [Y]"

### 4. Ignoring Context
- ❌ Proceeding without understanding the current state
- ✅ "Let me first understand the current directory structure before making changes"

## Reasoning Templates

### File Analysis Template
```
I need to analyze [file] to understand its contents and structure. Let me read it first.

[File reading result]

The file contains:
- [Key feature 1]
- [Key feature 2]
- [Key feature 3]

Based on this, I can see that [analysis conclusion].
```

### Task Planning Template
```
The task is to [objective]. To accomplish this, I need to:

1. [First step] - [Reason]
2. [Second step] - [Reason]
3. [Third step] - [Reason]

I'll start with [first step] because [reason].
```

### Error Recovery Template
```
The previous [action] failed because [reason]. I need to:

1. [Analysis of failure]
2. [Alternative approach]
3. [Implementation of fix]

Let me try [alternative approach] instead.
```