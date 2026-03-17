Read `AGENTS.md`, `IMPLEMENT.md`, `README.md`, `HANDOFF.md`, `VALIDATION.md`,
`docs/OPEN_QUESTIONS.md`, `spec/mvp-scope.md`, `spec/architecture.md`,
`spec/observability.md`, `spec/threat-model.md`, `spec/config.md`, and the
relevant modules under `crates/overlay-core/src/`.

Goal:
Audit or repair the Milestone 9 hardening baseline only if a task explicitly
reopens it from the current
`milestone-22-first-user-acceptance-pack` repository stage.

Current repository baseline:
- The current repository stage marker is
  `milestone-22-first-user-acceptance-pack`.
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
- Milestone 9 hardening and polish is implemented with observability/config
  groundwork, a bounded replay cache in `session::manager`, broad explicit
  subsystem observability integration, expanded malformed-input coverage, and
  the current regression suites, stage-boundary integration tests, and
  Milestone 9 unit coverage as its working boundary.
- Milestones 14/16/17/18/19/20/21 are landed baseline work ahead of the
  current Milestone 22 acceptance-pack stage.

Requirements:
- do not treat Milestone 9 as a broad umbrella task; take one concrete
  hardening slice at a time;
- keep hardening work aligned with `spec/observability.md`,
  `spec/threat-model.md`, `spec/config.md`, and `docs/OPEN_QUESTIONS.md`;
- keep explicit layering between identity, transport/session, peer/bootstrap,
  rendezvous, relay, routing, and service code;
- prefer local, bounded hardening changes such as rate limits, byte budgets,
  replay-risk mitigation, structured metrics/logs, stale/malformed input
  tests, and validation maintenance;
- keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, affected prompts, and
  `docs/OPEN_QUESTIONS.md` synchronized if the Milestone 9 baseline changes.

Constraints:
- do not restart from Milestone 0/1/2;
- do not rework Milestones 1-8 except for a concrete regression or spec
  mismatch;
- do not broaden into new protocol scope, global enumeration, or
  simulation-focused work;
- prefer explicit rejection of malformed, stale, replay-risk, or over-budget
  inputs over silent fallback.

Validation:
- run the applicable commands from `VALIDATION.md`;
- keep the Milestone 1-8 regression runs and stage-boundary smoke tests clean
  while Milestone 9 lands;
- if `REPOSITORY_STAGE` or the status docs change, rerun the stage-boundary
  smoke tests.
