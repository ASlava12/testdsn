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
It assumes the repository already has a closed Milestone 1-5 baseline and that
Milestone 5 is closed and Milestone 6 relay work is now active in code with an
explicit relay role model, canonical relay-intro messages, and direct-first
fallback planning.

## Recommended workflow

1. Confirm from `IMPLEMENT.md` that Milestones 1-4 are closed.
2. Treat Milestones 1-4 as regression-fix, vector-maintenance, and
   validation-maintenance territory only unless the task explicitly reopens them.
3. Treat Milestone 5 rendezvous/presence work as a closed baseline and reopen it
   only for regressions, vectors, or spec-conformance fixes.
4. Continue Milestone 6 relay intro/fallback work from the current relay
   baseline instead of leaving it at a placeholder, and keep recursive
   relay-on-relay behavior out of scope unless the spec explicitly reopens it.
5. Keep status docs, milestone prompts, and `docs/OPEN_QUESTIONS.md` aligned as
   the repository stage evolves.
