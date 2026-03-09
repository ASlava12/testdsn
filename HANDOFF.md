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
It assumes the repository already has a closed Milestone 1-2 baseline plus a
minimal Milestone 3 transport/session skeleton, and that baseline alignment for
docs, conservative defaults, and minimal identity fixtures is already done.

## Recommended workflow

1. Confirm from `IMPLEMENT.md` that Milestones 1-2 are closed and that a
   minimal Milestone 3 skeleton already exists.
2. Treat Milestones 1-2 as regression-fix, vector-maintenance, and
   validation-maintenance territory only.
3. Continue Milestone 3 transport/session work from the current skeleton.
4. Keep status docs, milestone prompts, and `docs/OPEN_QUESTIONS.md` aligned as
   the baseline evolves.
5. Keep Milestones 4+ out of scope unless the task explicitly advances them.
