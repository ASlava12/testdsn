Read `AGENTS.md`, `IMPLEMENT.md`, `README.md`, `HANDOFF.md`, `VALIDATION.md`,
`docs/LAUNCH_CHECKLIST.md`, `docs/PILOT_RUNBOOK.md`, `docs/PILOT_REPORT_TEMPLATE.md`,
`docs/RUNBOOK.md`, `docs/DEVNET.md`, `docs/TROUBLESHOOTING.md`,
`docs/OPEN_QUESTIONS.md`, `devnet/README.md`, `devnet/pilot/README.md`,
`spec/mvp-scope.md`, `spec/architecture.md`, `spec/wire-protocol.md`,
`spec/records.md`, and `spec/state-machines.md`.

Goal:
Work within the current repository stage,
`milestone-20-regular-distributed-use-closure`, from the landed Milestone
1-19 baseline and keep Milestone 20 focused on closing the remaining blockers
for regular distributed use without widening scope into hostile-environment or
public-Internet rollout.

Current repository baseline:
- The current repository stage marker is
  `milestone-20-regular-distributed-use-closure`.
- Milestones 0-12 are implemented, validated, and considered closed.
- Milestone 14 launch gate, Milestone 16 network bootstrap, Milestone 17
  operator runtime hardening, Milestone 18 real pilot, and Milestone 19 pilot
  closure are landed baseline work.
- Milestone 20 regular distributed use closure adds per-source bootstrap
  diagnostics on the runtime status surface, preferred retry/fallback ordering
  across configured bootstrap sources, reproducible `--evidence-dir` support
  for the distributed smoke and pilot checklist, and stronger localhost proof
  for bootstrap and relay fallback behavior in the checked-in two-relay pilot
  topology.

Requirements:
- preserve the Milestone 17 launch surface and use it as the prerequisite
  launch gate;
- keep Milestones 1-12 limited to regression fixes, validation maintenance,
  vector maintenance, or conservative spec-conformance fixes;
- keep status docs, prompts, and pilot docs synchronized if the stage boundary
  changes again;
- prefer conservative operator-surface hardening, distributed proof
  maintenance, pilot evidence collection support, and runbook clarity over new
  protocol or deployment features.

Constraints:
- do not claim hostile-environment or public-production readiness;
- do not add public bootstrap infrastructure, rollout automation, or a general
  distributed control plane;
- do not redesign protocol semantics or widen scope into general Internet
  deployment;
- keep the distributed operator flows honest about their point-to-point,
  operator-directed nature and exact-ID scope.

Validation:
- run the applicable commands from `VALIDATION.md`;
- rerun `./devnet/run-launch-gate.sh` when stage markers, launch docs, launch
  scripts, or current-stage smoke behavior change;
- run `./devnet/run-distributed-pilot-checklist.sh` when Milestone 20 pilot
  docs, configs, runtime bootstrap diagnostics, or operator flows change;
- report exactly what passed, what failed, and whether any failure is an
  environment issue rather than a code regression.
