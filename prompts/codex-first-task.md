Read `AGENTS.md`, `IMPLEMENT.md`, `VALIDATION.md`, `docs/OPEN_QUESTIONS.md`,
`spec/mvp-scope.md`, `spec/architecture.md`, `spec/observability.md`,
`spec/threat-model.md`, and `spec/config.md`.

Goal:
Continue Milestone 9 from the current closed Milestone 1-8 baseline while keeping
status documents and conservative defaults synchronized to the actual code
state.

Current repository baseline:
- Milestone 0 is already complete.
- Milestone 1 foundations are already implemented, vectorized, and validated in
  `overlay-core` (`identity`, `records`, `wire`).
- Milestone 2 crypto and handshake surface are already implemented, vectorized,
  and validated in `overlay-core` (`crypto`, `session::handshake`).
- Milestone 2 is considered closed.
- Milestone 3 transport/session work is implemented, validated, and considered
  closed in `overlay-core` (`transport`, `session::manager`).
- Milestone 4 peer/bootstrap work is implemented, validated, and considered
  closed in `overlay-core` (`bootstrap`, `peer`).
- Milestone 5 rendezvous/presence publish and exact lookup work is closed in
  `overlay-core` (`rendezvous`).
- Milestone 6 relay intro/fallback work is closed in `overlay-core` (`relay`)
  with a minimal local relay role model, canonical `ResolveIntro` /
  `IntroResponse` bodies, verified `IntroTicket` usage, and direct-first
  fallback planning.
- Milestone 7 routing work is closed in `overlay-core` (`routing`) with
  canonical `PathProbe` / `PathProbeResult` bodies, a bounded local probe
  tracker, deterministic path metrics, integer EWMA updates, and switch
  hysteresis. Milestone 7 is considered closed.
- Milestone 8 service-layer work is closed in `overlay-core` (`service`)
  with canonical service wire bodies, verified `ServiceRecord` registration, a
  bounded local service registry and open-session store, exact `app_id`
  resolution, `reachability_ref` binding checks, allow/deny local policy
  enforcement, and `integration_service_open` coverage.
- Milestone 9 hardening and polish is now active with the current regression
  suites and stage-boundary integration tests as its entry boundary.

Constraints:
- do not restart the repository from Milestone 0;
- do not refactor completed Milestone 1-2 code without a concrete bug or spec mismatch;
- do not advance protocol logic beyond the documented current stage;
- keep changes minimal and local.

Tasks:
- continue Milestone 9 hardening and polish work from the current validation
  and stage-boundary baseline;
- keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, affected prompts, and
  `docs/OPEN_QUESTIONS.md` synchronized if the repository baseline changes;
- treat Milestones 1-8 as closed baseline work, touching them only for
  regression fixes, vector maintenance, or validation maintenance;
- run the applicable Milestone 1-8 and stage-boundary validation from
  `VALIDATION.md`;
- stop before adding simulation-focused work or broader protocol scope.

Validation:
- run the applicable commands from `VALIDATION.md`;
- report exactly what passed, what failed, and whether any failure is due to
  environment/dependency availability rather than a code regression.
