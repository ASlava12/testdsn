Read `AGENTS.md`, `IMPLEMENT.md`, `README.md`, `HANDOFF.md`, `VALIDATION.md`,
`docs/LAUNCH_CHECKLIST.md`, `docs/PILOT_RUNBOOK.md`, `docs/PILOT_REPORT_TEMPLATE.md`,
`docs/RUNBOOK.md`, `docs/DEVNET.md`, `docs/TROUBLESHOOTING.md`,
`docs/OPEN_QUESTIONS.md`, `devnet/README.md`, `devnet/pilot/README.md`,
`spec/mvp-scope.md`, `spec/architecture.md`, `spec/wire-protocol.md`,
`spec/records.md`, and `spec/state-machines.md`.

Goal:
Work within the current repository stage, `milestone-19-pilot-closure`, from
the landed Milestone 1-18 baseline and keep Milestone 19 focused on closing
the real-pilot blockers without widening scope into public-Internet rollout.

Current repository baseline:
- The current repository stage marker is `milestone-19-pilot-closure`.
- Milestones 0-12 are implemented, validated, and considered closed.
- Milestone 14 launch gate and pilot tag remain part of the landed baseline.
- Milestone 16 network bootstrap and multi-host devnet remain part of the
  landed baseline.
- Milestone 17 operator-runtime hardening remains part of the landed baseline.
- Milestone 18 real pilot remains part of the landed baseline with
  `docs/PILOT_RUNBOOK.md`, `docs/PILOT_REPORT_TEMPLATE.md`, `devnet/pilot/`,
  and `./devnet/run-pilot-checklist.sh`.
- Milestone 19 pilot closure now adds networked operator commands for
  `publish`, `lookup`, `open-service`, and `relay-intro`,
  `overlay-cli run --service`, conservative `http://...#sha256=<pin>` bootstrap
  integrity checks, the two-relay pilot topology, and
  `./devnet/run-distributed-pilot-checklist.sh`.

Requirements:
- preserve the Milestone 17 launch surface and use it as the prerequisite gate;
- keep Milestones 1-12 limited to regression fixes, validation maintenance,
  vector maintenance, or conservative spec-conformance fixes;
- keep status docs, prompts, and pilot docs synchronized if the stage boundary
  changes again;
- prefer conservative operator-surface hardening, pilot evidence collection
  support, and runbook clarity over new protocol or deployment features.

Constraints:
- do not claim hostile-environment or public-production readiness;
- do not add public bootstrap infrastructure or rollout automation;
- do not redesign protocol semantics or widen scope into general Internet
  deployment;
- keep the distributed operator flows honest about their point-to-point,
  operator-directed nature and exact-ID scope.

Validation:
- run the applicable commands from `VALIDATION.md`;
- rerun `./devnet/run-launch-gate.sh` when stage markers, launch docs, launch
  scripts, or current-stage smoke behavior change;
- run `./devnet/run-distributed-pilot-checklist.sh` when Milestone 19 pilot
  docs, configs, or operator flows change;
- report exactly what passed, what failed, and whether any failure is an
  environment issue rather than a code regression.
