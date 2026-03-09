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
It assumes the repository already has a closed Milestone 1-4 baseline and that
the next active feature work begins at Milestone 5.

## Recommended workflow

1. Confirm from `IMPLEMENT.md` that Milestones 1-4 are closed.
2. Treat Milestones 1-4 as regression-fix, vector-maintenance, and
   validation-maintenance territory only unless the task explicitly reopens them.
3. Start Milestone 5 presence publish/exact lookup work from the closed Milestone 4 boundary.
4. Keep status docs, milestone prompts, and `docs/OPEN_QUESTIONS.md` aligned as
   the repository stage evolves.
5. Keep Milestone 6+ out of scope unless the task explicitly advances them.
