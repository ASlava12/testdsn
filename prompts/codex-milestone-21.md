Read `AGENTS.md`, `IMPLEMENT.md`, `README.md`, `HANDOFF.md`, `VALIDATION.md`,
`docs/LAUNCH_CHECKLIST.md`, `docs/PILOT_RUNBOOK.md`, `docs/RUNBOOK.md`,
`docs/CONFIG_EXAMPLES.md`, `docs/TROUBLESHOOTING.md`,
`docs/OPEN_QUESTIONS.md`, `devnet/README.md`, `spec/mvp-scope.md`,
`spec/architecture.md`, `spec/wire-protocol.md`, `spec/records.md`, and
`spec/state-machines.md`.

Goal:
Audit or repair the closed Milestone 21 first-user-runtime surface from the
current `milestone-24-bootstrap-trust-delivery-hardening` repository stage and keep
Milestone 21 work focused on bounded restart recovery, operator visibility,
and first-user config/runtime usability without widening protocol scope.

Current repository baseline:
- The current repository stage marker is
  `milestone-24-bootstrap-trust-delivery-hardening`.
- Milestones 0-12 are implemented, validated, and considered closed.
- Milestone 14 launch gate, Milestone 16 network bootstrap, Milestone 17
  operator runtime hardening, Milestone 18 real pilot, Milestone 19 pilot
  closure, and Milestone 20 regular distributed use closure are landed
  baseline work.
- Milestone 21 first-user runtime adds bounded restart recovery from
  last-known active bootstrap peers, continued bootstrap retry after peer-cache
  recovery, persisted status summaries, `overlay-cli doctor`, stable
  first-user example profiles, and more actionable config validation.

Requirements:
- preserve the current launch surface and distributed pilot proof path;
- keep Milestones 1-12 limited to regression fixes, validation maintenance,
  vector maintenance, or conservative spec-conformance fixes;
- prefer runtime usability, operator-surface hardening, and documentation sync
  over new protocol or deployment features;
- keep status docs, prompts, and runbooks synchronized if the stage boundary
  changes again.

Constraints:
- do not add new protocol layers, a database, or a general distributed control
  plane;
- do not redesign wire, handshake, record, quota, or routing semantics unless
  a concrete bug requires it;
- do not claim hostile-environment or public-production readiness;
- keep recovery bounded to conservative local state and document the exact
  assumptions.

Validation:
- run the applicable commands from `VALIDATION.md`;
- rerun `./devnet/run-launch-gate.sh` when launch scripts, restart behavior,
  doctor surfaces, or current-stage status docs change;
- run `./devnet/run-distributed-pilot-checklist.sh` when distributed pilot
  docs, configs, bootstrap diagnostics, or relay/operator flows change;
- report exactly what passed, what failed, and whether any failure is an
  environment issue rather than a code regression.
