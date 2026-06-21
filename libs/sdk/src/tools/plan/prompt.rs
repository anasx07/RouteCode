pub const ENTER_PLAN_MODE_PROMPT: &str = r#"Use this tool proactively when you're about to start a non-trivial implementation task. Getting user sign-off on your approach before writing code prevents wasted effort and ensures alignment.

## When to Use This Tool

**Prefer using EnterPlanMode** for implementation tasks unless they're simple. Use it when ANY of these conditions apply:

1. **New Feature Implementation**: Adding meaningful new functionality
2. **Multiple Valid Approaches**: The task can be solved in several different ways
3. **Code Modifications**: Changes that affect existing behavior or structure
4. **Architectural Decisions**: The task requires choosing between patterns or technologies
5. **Multi-File Changes**: The task will likely touch more than 2-3 files
6. **Unclear Requirements**: You need to explore before understanding the full scope
7. **User Preferences Matter**: The implementation could reasonably go multiple ways

## When NOT to Use This Tool

Only skip EnterPlanMode for simple tasks:
- Single-line or few-line fixes (typos, obvious bugs, small tweaks)
- Adding a single function with clear requirements
- Tasks where the user has given very specific, detailed instructions
- Pure research/exploration tasks

## What Happens in Plan Mode

In plan mode, you'll:
1. Thoroughly explore the codebase to understand existing patterns
2. Identify similar features and architectural approaches
3. Consider multiple approaches and their trade-offs
4. Design a concrete implementation strategy
5. When ready, use `exit_plan_mode` to present your plan for approval

In plan mode, only read-only tools are available. Bash commands that would
write or delete files are denied with a hard error. To actually implement
the plan after approval, the user must approve it via the exit_plan_mode
tool — at which point write tools are unlocked for the rest of the session.
"#;

pub const EXIT_PLAN_MODE_PROMPT: &str = r#"Use this tool when you are in plan mode and have finished designing your plan. It presents the current plan (persisted to disk during plan mode) to the user for approval.

The user can:
- **Approve** to unlock write tools for the rest of the session
- **Deny** to keep plan mode active and revise the plan
- **Send feedback** to request specific changes

After approval, you can implement the plan using the full tool set. If
the user denies, stay in plan mode and revise the plan based on their
feedback.
"#;
