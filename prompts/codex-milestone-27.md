Read `AGENTS.md`, `IMPLEMENT.md`, `README.md`, `HANDOFF.md`, `VALIDATION.md`,
`docs/FIRST_USER_ACCEPTANCE.md`, `docs/LAUNCH_CHECKLIST.md`,
`docs/PILOT_RUNBOOK.md`, `docs/RUNBOOK.md`, `docs/DEVNET.md`,
`docs/OPEN_QUESTIONS.md`, `devnet/README.md`, `spec/mvp-scope.md`,
`spec/architecture.md`, `spec/routing.md`, and `spec/relay.md`.

Goal:
Work within the current repository stage,
`milestone-27-relay-topology-generalization`, from the landed Milestone 1-26
baseline and keep Milestone 27 focused on bounded relay-topology proof,
deterministic multi-candidate relay behavior, partial-failure recovery, and
honest runbook documentation without widening protocol scope.

Current repository baseline:
- The current repository stage marker is
  `milestone-27-relay-topology-generalization`.
- Milestones 0-12 are implemented, validated, and considered closed.
- Milestone 14 launch gate, Milestone 16 network bootstrap, Milestone 17
  operator runtime hardening, Milestone 18 real pilot, Milestone 19 pilot
  closure, Milestone 20 regular distributed use closure, Milestone 21
  first-user runtime, Milestone 22 first-user acceptance pack, Milestone 24
  bootstrap trust/delivery hardening, Milestone 25 runtime
  persistence/recovery hardening, and Milestone 26 bounded operator control
  plane are landed baseline work.
- Milestone 27 relay and topology generalization adds bounded three-relay
  pilot proof, deterministic multi-candidate relay-plan coverage, repeated
  relay-bind failure recovery through a later bounded candidate, and the
  updated relay/runbook smoke coverage for that surface.

Requirements:
- preserve the current launch surface and first-user acceptance path;
- prefer bounded relay-topology proof, deterministic fallback ordering, and
  honest failure visibility over new protocol or autonomy features;
- keep Milestones 1-12 limited to regression fixes, validation maintenance,
  vector maintenance, or conservative spec-conformance fixes;
- keep current-stage docs, prompts, and runbooks synchronized if the stage
  boundary changes again.

Constraints:
- do not redesign routing algorithms or relay semantics fundamentally;
- do not add arbitrary public-network relay-graph ambitions, discovery meshes,
  anonymity layers, or onion-routing behavior;
- do not claim public-production or hostile-environment readiness;
- do not widen the first-user-ready claim beyond what the current acceptance
  pack and off-box pilot evidence actually prove.

Validation:
- run the applicable commands from `VALIDATION.md`;
- rerun `./devnet/run-multihost-smoke.sh`,
  `./devnet/run-distributed-pilot-checklist.sh`, and
  `./devnet/run-first-user-acceptance.sh` when relay topology, current-stage
  pilot configs, current-stage scripts, or current-stage docs change;
- report exactly what passed, what failed, and whether any failure is an
  environment issue rather than a code regression.
