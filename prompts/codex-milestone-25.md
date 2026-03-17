Read `AGENTS.md`, `IMPLEMENT.md`, `README.md`, `HANDOFF.md`, `VALIDATION.md`,
`docs/FIRST_USER_ACCEPTANCE.md`, `docs/LAUNCH_CHECKLIST.md`,
`docs/PILOT_RUNBOOK.md`, `docs/RUNBOOK.md`, `docs/DEVNET.md`,
`docs/OPEN_QUESTIONS.md`, `devnet/README.md`, `spec/mvp-scope.md`,
`spec/architecture.md`, `spec/wire-protocol.md`, `spec/records.md`, and
`spec/state-machines.md`.

Goal:
Work within the current repository stage,
`milestone-25-runtime-persistence-recovery-hardening`, from the landed
Milestone 1-24 baseline and keep Milestone 25 focused on bounded runtime
persistence, explicit recovery semantics, operator diagnostics, and honest
restart/runbook docs without widening protocol scope.

Current repository baseline:
- The current repository stage marker is
  `milestone-25-runtime-persistence-recovery-hardening`.
- Milestones 0-12 are implemented, validated, and considered closed.
- Milestone 14 launch gate, Milestone 16 network bootstrap, Milestone 17
  operator runtime hardening, Milestone 18 real pilot, Milestone 19 pilot
  closure, Milestone 20 regular distributed use closure, Milestone 21
  first-user runtime, Milestone 22 first-user acceptance pack, and Milestone
  24 bootstrap trust/delivery hardening are landed baseline work.
- Milestone 25 runtime persistence/recovery hardening adds bounded recovery of
  persisted bootstrap-source preference, last-known active bootstrap peers,
  and local service registration intent, plus explicit recovery fields on the
  status/doctor surfaces and restart-focused proof updates.

Requirements:
- preserve the current launch surface and distributed pilot proof path;
- prefer bounded restart usability, explicit operator recovery state, and
  checklist honesty over new features;
- keep Milestones 1-12 limited to regression fixes, validation maintenance,
  vector maintenance, or conservative spec-conformance fixes;
- keep status docs, prompts, and runbooks synchronized if the stage boundary
  changes again.

Constraints:
- do not add a database, broad durable protocol-state persistence, or a
  distributed control plane;
- do not redesign wire, handshake, record, quota, or routing semantics unless
  a concrete bug requires it;
- do not claim public-production or hostile-environment readiness;
- do not widen the first-user-ready claim beyond what the current acceptance
  pack actually proves.

Validation:
- run the applicable commands from `VALIDATION.md`;
- rerun `./devnet/run-restart-smoke.sh`,
  `./devnet/run-distributed-pilot-checklist.sh`, and
  `./devnet/run-first-user-acceptance.sh` when restart, recovery, current-stage
  scripts, or acceptance docs change;
- report exactly what passed, what failed, and whether any failure is an
  environment issue rather than a code regression.
