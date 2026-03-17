Read `AGENTS.md`, `IMPLEMENT.md`, `README.md`, `HANDOFF.md`, `VALIDATION.md`,
`docs/FIRST_USER_ACCEPTANCE.md`, `docs/LAUNCH_CHECKLIST.md`,
`docs/PILOT_RUNBOOK.md`, `docs/RUNBOOK.md`, `docs/DEVNET.md`,
`docs/OPEN_QUESTIONS.md`, `devnet/README.md`, `spec/mvp-scope.md`,
`spec/architecture.md`, `spec/service-layer.md`, and `spec/state-machines.md`.

Goal:
Work within the current repository stage,
`milestone-26-bounded-operator-control-plane`, from the landed Milestone 1-25
baseline and keep Milestone 26 focused on bounded operator usability,
machine-readable status/health surfaces, repeatable explicit operator checks,
and honest runbook documentation without widening protocol scope.

Current repository baseline:
- The current repository stage marker is
  `milestone-26-bounded-operator-control-plane`.
- Milestones 0-12 are implemented, validated, and considered closed.
- Milestone 14 launch gate, Milestone 16 network bootstrap, Milestone 17
  operator runtime hardening, Milestone 18 real pilot, Milestone 19 pilot
  closure, Milestone 20 regular distributed use closure, Milestone 21
  first-user runtime, Milestone 22 first-user acceptance pack, Milestone 24
  bootstrap trust/delivery hardening, and Milestone 25 runtime
  persistence/recovery hardening are landed baseline work.
- Milestone 26 bounded operator control plane adds `overlay-cli inspect`,
  machine-readable bounded operator reports that combine local status/doctor
  data with explicit remote lookup/open-service/relay-intro probes, and the
  updated operator/runbook smoke coverage for that surface.

Requirements:
- preserve the current launch surface and distributed pilot proof path;
- prefer bounded operator visibility, repeatable explicit checks, and
  troubleshooting honesty over new protocol or orchestration features;
- keep Milestones 1-12 limited to regression fixes, validation maintenance,
  vector maintenance, or conservative spec-conformance fixes;
- keep status docs, prompts, and runbooks synchronized if the stage boundary
  changes again.

Constraints:
- do not add a distributed control plane, discovery mesh, or broad remote
  admin API;
- do not redesign wire, handshake, record, relay, routing, or service
  semantics unless a concrete bug requires it;
- do not claim public-production or hostile-environment readiness;
- do not widen the first-user-ready claim beyond what the current acceptance
  pack and off-box pilot evidence actually prove.

Validation:
- run the applicable commands from `VALIDATION.md`;
- rerun `./devnet/run-multihost-smoke.sh`,
  `./devnet/run-distributed-pilot-checklist.sh`, and
  `./devnet/run-first-user-acceptance.sh` when operator surfaces,
  current-stage scripts, or current-stage docs change;
- report exactly what passed, what failed, and whether any failure is an
  environment issue rather than a code regression.
