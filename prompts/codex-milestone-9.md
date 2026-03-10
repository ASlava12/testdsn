Read `AGENTS.md`, `IMPLEMENT.md`, `VALIDATION.md`, `spec/mvp-scope.md`,
`spec/architecture.md`, `spec/observability.md`, `spec/threat-model.md`,
`spec/config.md`, `docs/OPEN_QUESTIONS.md`, and the relevant modules under
`crates/overlay-core/src/`.

Goal:
Continue Milestone 9 hardening and polish work from the current closed
Milestone 1-8 baseline.

Current repository baseline:
- Milestone 0 is complete.
- Milestone 1 identities, records, and wire foundations are implemented,
  vectorized, and validated.
- Milestone 2 crypto wrappers and handshake surface are implemented,
  vectorized, validated, and considered closed.
- Milestone 3 transport/session work is implemented, validated, and considered
  closed.
- Milestone 4 peer/bootstrap work is implemented, validated, and considered
  closed.
- Milestone 5 rendezvous/presence publish and exact lookup work is implemented,
  validated, and considered closed.
- Milestone 6 relay intro/fallback work is implemented, validated, and
  considered closed.
- Milestone 7 routing/path work is implemented, validated, and considered
  closed.
- Milestone 8 service-layer work is implemented, validated, and considered
  closed.
- Milestone 9 hardening and polish is active with observability/config
  groundwork, a bounded replay cache in `session::manager`, and broad explicit
  subsystem observability integration landed, plus expanded malformed-input
  coverage in relay/routing/service wire helpers, and with the current
  regression suites, stage-boundary integration tests, and Milestone 9 unit
  coverage as its working boundary.

Requirements:
- keep hardening work aligned with `spec/observability.md`,
  `spec/threat-model.md`, `spec/config.md`, and `docs/OPEN_QUESTIONS.md`;
- keep explicit layering between identity, transport/session, peer/bootstrap,
  rendezvous, relay, routing, and service code;
- prefer local, bounded hardening changes such as rate limits, byte budgets,
  replay-risk mitigation, structured metrics/logs, stale/malformed input tests,
  and validation maintenance;
- keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, affected prompts, and
  `docs/OPEN_QUESTIONS.md` synchronized if the Milestone 9 baseline changes.

Constraints:
- do not rework Milestones 1-8 except for a concrete regression or spec
  mismatch;
- do not broaden into new protocol scope, global enumeration, or
  simulation-focused work;
- prefer explicit rejection of malformed, stale, replay-risk, or over-budget
  inputs over silent fallback.

Validation:
- run the applicable commands from `VALIDATION.md`;
- keep the Milestone 1-8 regression runs and stage-boundary smoke tests clean
  while Milestone 9 lands.
