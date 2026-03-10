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

Use the contents of `prompts/codex-milestone-9.md` as the first task.
It assumes the repository already has a closed Milestone 1-8 baseline and that
Milestone 9 hardening and polish is now active with observability/config
groundwork, a bounded replay cache, and broad explicit subsystem
observability integration landed, including bootstrap provider fetch/validation
logging, explicit transport-buffer config projection from top-level node
config and runner-boundary transport frame validation, plus expanded
malformed-input coverage in the wire-helper tests, including bootstrap schema
timing and frame-limit rejection paths, and an explicit established-session
gauge sync helper, and with the current regression, stage-boundary, and
Milestone 9 unit suites as its working boundary.

## Recommended workflow

1. Confirm from `IMPLEMENT.md` that Milestones 1-8 are closed and Milestone 9
   is the active stage.
2. Treat Milestones 1-8 as regression-fix, vector-maintenance, and
   validation-maintenance territory only unless the task explicitly reopens them.
3. Continue Milestone 9 hardening and polish work from the current validation
   baseline instead of broadening scope prematurely.
4. Keep simulation-focused expansion and broader protocol scope out of work
   until Milestone 9 is materially complete.
5. Keep status docs, milestone prompts, and `docs/OPEN_QUESTIONS.md` aligned as
   the repository stage evolves.
