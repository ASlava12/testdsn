# Handoff to Codex

## What this package is

A Codex-oriented handoff bundle containing:
- project instructions (`AGENTS.md`)
- implementation milestones (`IMPLEMENT.md`)
- validation commands (`VALIDATION.md`)
- open questions to avoid silent invention (`docs/OPEN_QUESTIONS.md`)
- starter prompts (`prompts/*.md`)
- protocol/spec files (`spec/*.md`)

## Current repository stage

- `REPOSITORY_STAGE` is `milestone-9-hardening`.
- Milestones 0-8 are a closed baseline in this repository.
- Milestone 9 hardening and polish is the active stage with
  observability/config groundwork, a bounded handshake replay cache, explicit
  subsystem observability integration, and the current regression,
  stage-boundary, and Milestone 9 unit suites as the working validation
  boundary.
- Local runtime/devnet follow-on work through Milestone 12 is also present on
  top of that stage marker, including runtime cleanup/retry/status hardening
  and the local devnet soak path, but it does not change `REPOSITORY_STAGE`.

## Recommended first Codex task

Use `prompts/codex-milestone-9.md` as the first task prompt for the current
`milestone-9-hardening` stage. It assumes the repository already has a closed
Milestone 1-8 baseline and does not need to restart from Milestone 0/1/2.

## Recommended workflow

1. Confirm from `README.md`, `AGENTS.md`, and `IMPLEMENT.md` that the current
   stage is `milestone-9-hardening`.
2. Do not restart from Milestone 0/1/2; treat Milestones 1-8 as
   regression-fix, vector-maintenance, and validation-maintenance territory
   only unless the task explicitly reopens them.
3. Scope Milestone 9 work narrowly from the current hardening boundary instead
   of treating it as a broad umbrella task.
4. Keep simulation-focused expansion and broader protocol scope out of work
   until Milestone 9 is materially complete.
5. Keep status docs, milestone prompts, and `docs/OPEN_QUESTIONS.md` aligned as
   the repository stage evolves.
