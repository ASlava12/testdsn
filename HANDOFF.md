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
minimal Milestone 3 transport/session skeleton, and that the immediate next
task is baseline alignment before further feature work.

## Recommended workflow

1. Confirm from `IMPLEMENT.md` that Milestones 1-2 are closed and that a
   minimal Milestone 3 skeleton already exists.
2. Sync status docs, milestone prompts, and `docs/OPEN_QUESTIONS.md` to that
   actual baseline before extending protocol logic.
3. Use Milestones 1-2 for regression fixes, vector maintenance, and handshake
   validation if gaps are found.
4. Resume Milestone 3 transport/session work only after the baseline stays in
   sync and validation is green.
5. Keep Milestones 4+ out of scope unless the task explicitly advances them.
