Read `AGENTS.md`, `IMPLEMENT.md`, `README.md`, `HANDOFF.md`, `VALIDATION.md`,
`docs/LAUNCH_CHECKLIST.md`, `docs/PILOT_RUNBOOK.md`, `docs/PILOT_REPORT_TEMPLATE.md`,
`docs/RUNBOOK.md`, `docs/DEVNET.md`, `docs/OPEN_QUESTIONS.md`,
`devnet/README.md`, `devnet/pilot/README.md`, `spec/mvp-scope.md`,
`spec/architecture.md`, `spec/wire-protocol.md`, `spec/records.md`, and
`spec/state-machines.md`.

Goal:
Audit or repair the closed Milestone 18 real-pilot surface from the current
`milestone-22-first-user-acceptance-pack` repository stage and keep Milestone
18 work focused on pilot execution support on separate hosts.

Current repository baseline:
- The current repository stage marker is
  `milestone-22-first-user-acceptance-pack`.
- Milestones 0-12 are implemented, validated, and considered closed.
- Milestone 14 launch gate and pilot tag remain part of the landed pilot
  baseline.
- Milestone 16 network bootstrap and multi-host devnet remain part of the
  landed pilot baseline.
- Milestone 17 operator-runtime hardening remains part of the landed pilot
  baseline with signal-aware shutdown, config-local `.overlay-runtime/` state,
  `overlay-cli status`, and the bounded soak in the launch gate.
- Milestone 18 real-pilot support now adds a dedicated pilot topology pack,
  `docs/PILOT_RUNBOOK.md`, `docs/PILOT_REPORT_TEMPLATE.md`,
  `./devnet/run-pilot-checklist.sh`, and fault-scenario reporting.

Requirements:
- preserve the Milestone 17 launch surface and use it as the prerequisite gate;
- keep Milestones 1-12 limited to regression fixes, validation maintenance,
  vector maintenance, or conservative spec-conformance fixes;
- keep status docs, prompts, and pilot docs synchronized if the stage boundary
  changes again;
- prefer conservative pilot execution support, reporting, and runbook clarity
  over new protocol or deployment features.

Constraints:
- do not claim hostile-environment or public-production readiness;
- do not add public bootstrap infrastructure or rollout automation;
- do not redesign protocol semantics or widen scope into general Internet
  deployment;
- keep publish/lookup/service-open/relay validation honest about any remaining
  smoke-harness coordination.

Validation:
- run the applicable commands from `VALIDATION.md`;
- rerun `./devnet/run-launch-gate.sh` when stage markers, launch docs, launch
  scripts, or current-stage smoke behavior change;
- run `./devnet/run-pilot-checklist.sh` when Milestone 18 pilot docs, configs,
  or fault flows change;
- report exactly what passed, what failed, and whether any failure is an
  environment issue rather than a code regression.
