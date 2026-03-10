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
It assumes the repository already has a closed Milestone 1-6 baseline and that
Milestone 7 routing work is now active in code with deterministic path metrics,
integer EWMA updates, and switch hysteresis.

## Recommended workflow

1. Confirm from `IMPLEMENT.md` that Milestones 1-6 are closed.
2. Treat Milestones 1-6 as regression-fix, vector-maintenance, and
   validation-maintenance territory only unless the task explicitly reopens them.
3. Continue Milestone 7 routing metrics and path switching work from the current
   routing baseline instead of leaving it at a placeholder.
4. Keep Milestone 8+ service behavior out of scope until Milestone 7 is
   materially complete.
5. Keep status docs, milestone prompts, and `docs/OPEN_QUESTIONS.md` aligned as
   the repository stage evolves.
