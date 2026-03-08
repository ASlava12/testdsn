# Handoff to Codex

## What this package is

A Codex-oriented handoff bundle containing:
- project instructions (`AGENTS.md`)
- implementation milestones (`IMPLEMENT.md`)
- validation commands (`VALIDATION.md`)
- open questions to avoid silent invention (`docs/OPEN_QUESTIONS.md`)
- starter prompts (`prompts/*.md`)
- protocol/spec files (`spec/*.md`)

## Recommended first Codex task

Use the contents of `prompts/codex-first-task.md` as the first task.
It assumes Milestones 0-2 are already closed and that the next active work
starts at Milestone 3.

## Recommended workflow

1. Confirm from `IMPLEMENT.md` that Milestones 0-2 are closed.
2. Treat Milestones 1-2 as regression-fix and validation-maintenance work only.
3. Start Milestone 3 transport/session work using the current prompts.
4. Keep Milestones 4+ out of scope unless the task explicitly advances them.
