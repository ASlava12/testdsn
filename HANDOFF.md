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
  read `milestone-27-relay-topology-generalization`.
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
- Milestone 20 regular distributed use closure is part of the landed baseline
  with per-source bootstrap diagnostics on the runtime status surface,
  expanded bootstrap-fallback proof for unavailable/integrity/stale/empty
  cases, stronger relay-bind evidence across the checked-in two-relay
  topology, and reproducible `--evidence-dir` support for the distributed
  smoke and pilot checklist.
- Milestone 21 first-user runtime is part of the landed baseline with bounded recovery of
  the last-known active bootstrap peers across restart, continued bootstrap
  retry after peer-cache recovery, `overlay-cli status --summary`,
  `overlay-cli doctor`, stable first-user example profiles, and more
  actionable config validation.
- Milestone 22 first-user acceptance pack remains part of the landed baseline
  with the bounded `./devnet/run-first-user-acceptance.sh` wrapper, explicit
  first-user-ready scenario coverage, and synchronized acceptance-boundary
  docs.
- Milestone 24 bootstrap trust and delivery hardening remains part of the
  landed baseline with signed bootstrap artifacts, pinned signer-key
  verification with optional SHA-256 integrity pins, `overlay-cli
  bootstrap-sign`, `overlay-cli bootstrap-serve --signing-key-file`,
  trust-failure diagnostics, and synchronized operator/bootstrap runbooks.
- Milestone 25 runtime persistence and recovery hardening remains part of the
  landed baseline with bounded recovery of persisted bootstrap-source
  preference, last-known active bootstrap peers, and local service
  registration intent, explicit recovery fields in status/doctor output, and
  updated restart proof paths.
- Milestone 26 bounded operator control plane remains part of the landed
  baseline
  with `overlay-cli inspect`, bounded machine-readable operator reports that
  combine local status/doctor data with explicit remote lookup, service, and
  relay probes, and synchronized operator/runbook docs.
- Milestone 27 relay and topology generalization is the current stage with a
  bounded third relay-capable pilot node, deterministic multi-candidate relay
  proof, repeated relay-bind failure recovery through a later bounded
  candidate, and synchronized relay/runbook docs.
- The current validation green path is `./devnet/run-first-user-acceptance.sh`
  on the same commit after the applicable workspace validation commands.
- Separate-host evidence is still required on the validated commit before
  claiming first-user-ready status for that release candidate.

## Current green path

1. Run the applicable commands in `VALIDATION.md`.
2. Run `./devnet/run-first-user-acceptance.sh`.
3. Use `docs/PILOT_RUNBOOK.md` for the off-box pilot run and evidence
   collection.

`./devnet/run-launch-gate.sh` and
`./devnet/run-distributed-pilot-checklist.sh` remain the landed component
scripts inside the current acceptance flow. `./devnet/run-pilot-checklist.sh`
is retained only for the older Milestone 18 localhost rehearsal.

## Remaining limitations after Milestone 27

- bootstrap remains static signed artifact delivery over `http://`
- operator surfaces remain bounded and operator-directed; `overlay-cli
  inspect` aggregates explicit requested probes, but the repo still has no
  general control plane
- restart recovery stays bounded to bootstrap-source state, last-known active
  bootstrap peers, and local service registration intent; presence, service
  sessions, relay tunnels, and path probes still reset
- relay fallback proof remains bounded to the checked-in three-relay pilot pack
- off-box evidence is still required on separate hosts for the exact release
  commit before claiming first-user-ready status

## Recommended first Codex task

Use `prompts/codex-milestone-27.md` as the first task prompt for the current
`milestone-27-relay-topology-generalization` stage. It assumes the repository already has a
closed Milestone 1-12 baseline and does not need to restart from Milestone
0/1/2.

## Recommended workflow

1. Confirm from `README.md`, `AGENTS.md`, and `IMPLEMENT.md` that the current
   stage is `milestone-27-relay-topology-generalization`.
2. Do not restart from Milestone 0/1/2; treat Milestones 1-12 as
   regression-fix, vector-maintenance, validation-maintenance, and
   launch-maintenance territory only unless the task explicitly reopens them.
3. Treat `./devnet/run-first-user-acceptance.sh` as the current localhost
   sign-off flow and the launch gate plus distributed pilot checklist as its
   landed component scripts.
4. Scope work narrowly from the pilot execution boundary instead of treating
   the stage as a feature umbrella.
5. Keep broader protocol scope, public-production claims, and redesign work out
   of current-stage tasks unless explicitly requested.
6. Keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, `VALIDATION.md`,
   `docs/FIRST_USER_ACCEPTANCE.md`, `docs/PILOT_RUNBOOK.md`,
   `docs/DEVNET.md`, `docs/LAUNCH_CHECKLIST.md`, `docs/OPEN_QUESTIONS.md`,
   milestone prompts, and the root `REPOSITORY_STAGE` marker aligned as the
   repository stage evolves.
