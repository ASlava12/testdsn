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

- The root `REPOSITORY_STAGE` marker and `overlay_core::REPOSITORY_STAGE` both
  read `milestone-20-regular-distributed-use-closure`.
- Milestones 0-12 are a closed baseline in this repository, and Milestones
  14/16/17/18 are landed pilot-baseline work.
- Milestone 17 operator-grade runtime hardening is part of the landed baseline
  with signal-aware `overlay-cli run`, restart-safe operator lock/status files,
  `overlay-cli status`, stricter startup/config validation, the bounded soak in
  the launch gate, and explicit pilot-only limitations.
- Milestone 18 real pilot remains part of the landed baseline with
  `docs/PILOT_RUNBOOK.md`, `docs/PILOT_REPORT_TEMPLATE.md`, `devnet/pilot/`,
  and the retained `devnet/run-pilot-checklist.sh` localhost rehearsal pack.
- Milestone 19 pilot closure is part of the landed baseline with distributed operator
  surfaces, SHA-256-pinned static bootstrap artifacts, the two-relay pilot
  topology, and `devnet/run-distributed-pilot-checklist.sh`.
- Milestone 20 regular distributed use closure is the current stage with
  per-source bootstrap diagnostics on the runtime status surface, expanded
  bootstrap-fallback proof for unavailable/integrity/stale/empty cases,
  stronger relay-bind evidence across the checked-in two-relay topology, and
  reproducible `--evidence-dir` script support for the distributed smoke and
  pilot checklist.
- The current validation green path is `./devnet/run-launch-gate.sh` followed
  by `./devnet/run-distributed-pilot-checklist.sh` on the same commit.
- Separate-host evidence is still required on the validated commit before
  claiming regular distributed use for that release candidate.

## Current green path

1. Run the applicable commands in `VALIDATION.md`.
2. Run `./devnet/run-launch-gate.sh`.
3. Run `./devnet/run-distributed-pilot-checklist.sh`.
4. Use `docs/PILOT_RUNBOOK.md` for the off-box pilot run and evidence
   collection.

Do not treat `./devnet/run-pilot-checklist.sh` as the current sign-off path.
It is retained only for the older Milestone 18 localhost rehearsal.

## Remaining blockers for regular distributed use

- off-box evidence is still required on separate hosts for the exact release
  commit
- bootstrap remains static pinned `http://` artifact delivery
- distributed operator commands remain one-shot proof surfaces, not a general
  control plane
- peers, presence, services, sessions, relay tunnels, and path probes remain
  in-memory across restart
- relay fallback proof remains bounded to the checked-in two-relay pilot pack

## Recommended first Codex task

Use `prompts/codex-milestone-20.md` as the first task prompt for the current
`milestone-20-regular-distributed-use-closure` stage. It assumes the repository already has a
closed Milestone 1-12 baseline and does not need to restart from Milestone
0/1/2.

## Recommended workflow

1. Confirm from `README.md`, `AGENTS.md`, and `IMPLEMENT.md` that the current
   stage is `milestone-20-regular-distributed-use-closure`.
2. Do not restart from Milestone 0/1/2; treat Milestones 1-12 as
   regression-fix, vector-maintenance, validation-maintenance, and
   launch-maintenance territory only unless the task explicitly reopens them.
3. Treat `./devnet/run-launch-gate.sh` plus
   `./devnet/run-distributed-pilot-checklist.sh` as the current localhost
   sign-off flow.
4. Scope work narrowly from the pilot execution boundary instead of treating
   the stage as a feature umbrella.
5. Keep broader protocol scope, public-production claims, and redesign work out
   of current-stage tasks unless explicitly requested.
6. Keep status docs, milestone prompts, `docs/OPEN_QUESTIONS.md`, and the root
   `REPOSITORY_STAGE` marker aligned as the repository stage evolves.
