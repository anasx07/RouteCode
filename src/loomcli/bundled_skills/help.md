---
name: help
description: Explains how to use and create skills in LoomCLI
context: inline
---

# LoomCLI Skills Guide

Skills are Markdown files with frontmatter that define reusable prompts or autonomous sub-agent tasks.

## How to Create a Skill
Create a `.md` file in `~/.loomcli/skills/` with the following format:

```markdown
---
name: my-skill
description: A brief description of what this does
context: inline | fork
---
Your prompt content here...
```

## Context Modes
- **inline**: The prompt content is expanded directly into your current conversation.
- **fork**: The prompt is sent to a background sub-agent that works autonomously and returns a result.

## Usage
Type `/` in the REPL to see available skills and use tab-completion to select one.
