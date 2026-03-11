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

- `REPOSITORY_STAGE` is `milestone-19-pilot-closure`.
- Milestones 0-12 are a closed baseline in this repository.
- Milestone 17 operator-grade runtime hardening is part of the landed baseline
  with signal-aware `overlay-cli run`, restart-safe operator lock/status files,
  `overlay-cli status`, stricter startup/config validation, the bounded soak in
  the launch gate, and explicit pilot-only limitations.
- Milestone 18 real pilot remains part of the landed baseline with
  `docs/PILOT_RUNBOOK.md`, `docs/PILOT_REPORT_TEMPLATE.md`, `devnet/pilot/`,
  and `devnet/run-pilot-checklist.sh`.
- Milestone 19 pilot closure is the current stage with distributed operator
  surfaces, SHA-256-pinned static bootstrap artifacts, the two-relay pilot
  topology, and `devnet/run-distributed-pilot-checklist.sh`.
- The current validation green path is `./devnet/run-launch-gate.sh` followed
  by `./devnet/run-distributed-pilot-checklist.sh`, with separate-host evidence
  still required before claiming regular distributed pilot closure.

## Recommended first Codex task

Use `prompts/codex-milestone-19.md` as the first task prompt for the current
`milestone-19-pilot-closure` stage. It assumes the repository already has a
closed Milestone 1-12 baseline and does not need to restart from Milestone
0/1/2.

## Recommended workflow

1. Confirm from `README.md`, `AGENTS.md`, and `IMPLEMENT.md` that the current
   stage is `milestone-19-pilot-closure`.
2. Do not restart from Milestone 0/1/2; treat Milestones 1-12 as
   regression-fix, vector-maintenance, validation-maintenance, and
   launch-maintenance territory only unless the task explicitly reopens them.
3. Scope work narrowly from the pilot execution boundary instead of treating
   the stage as a feature umbrella.
4. Keep broader protocol scope, public-production claims, and redesign work out
   of current-stage tasks unless explicitly requested.
5. Keep status docs, milestone prompts, and `docs/OPEN_QUESTIONS.md` aligned as
   the repository stage evolves.
