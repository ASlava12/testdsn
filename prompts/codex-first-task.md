Read `AGENTS.md`, `IMPLEMENT.md`, `VALIDATION.md`, `docs/OPEN_QUESTIONS.md`,
`spec/mvp-scope.md`, `spec/architecture.md`, `spec/observability.md`,
`spec/threat-model.md`, and `spec/config.md`.

Goal:
Synchronize on the current repository stage,
`milestone-22-first-user-acceptance-pack`, and use the dedicated
Milestone 22 prompt
for the next concrete current-stage task.

Current repository baseline:
- The current repository stage is `milestone-22-first-user-acceptance-pack`.
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
- Milestone 9 hardening, Milestone 10 minimal runtime, Milestone 11 local
  devnet, and Milestone 12 launch hardening are implemented as part of the
  frozen baseline.
- Milestone 14 launch gate and pilot tag remains part of the landed pilot
  baseline.
- Milestone 16 network bootstrap and multi-host devnet is now part of the
  landed baseline with `overlay-cli bootstrap-serve`, host-style devnet
  configs under `devnet/hosts/`, and a documented green-path validation
  sequence.
- Milestone 17 operator-grade runtime hardening is now part of the landed
  baseline with
  signal-aware `overlay-cli run`, config-local `.overlay-runtime/` operator
  state, `overlay-cli status`, stricter startup/config validation, and the
  bounded soak in the current gate.
- Milestone 18 real pilot remains part of the landed baseline with
  `docs/PILOT_RUNBOOK.md`, `docs/PILOT_REPORT_TEMPLATE.md`, `devnet/pilot/`,
  and `./devnet/run-pilot-checklist.sh`.
- Milestone 19 pilot closure is part of the landed baseline with distributed
  operator commands, pinned static bootstrap artifacts, the expanded pilot
  topology, and `./devnet/run-distributed-pilot-checklist.sh`.
- Milestone 20 regular distributed use closure is landed baseline work.
- Milestone 21 first-user runtime is landed baseline work.
- Milestone 22 first-user acceptance pack is now active with the bounded
  `./devnet/run-first-user-acceptance.sh` wrapper, explicit
  first-user-ready scenarios, fresh-node-join and
  relay-unavailable-service-open coverage in the distributed checklist, and
  synchronized acceptance-boundary docs.

Constraints:
- do not restart the repository from Milestone 0;
- do not refactor completed Milestone 1-2 code without a concrete bug or spec mismatch;
- do not advance protocol logic beyond the documented current stage;
- keep changes minimal and local.

Tasks:
- confirm the repository already sits at the closed Milestone 1-8 baseline;
- use `prompts/codex-milestone-22.md` for the next concrete
  `milestone-22-first-user-acceptance-pack` task;
- keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, affected prompts, and
  `docs/OPEN_QUESTIONS.md` synchronized if the repository baseline changes;
- treat Milestones 1-12 as closed baseline work, touching them only for
  regression fixes, vector maintenance, validation maintenance, or launch
  maintenance;
- run the applicable Milestone 1-8 and stage-boundary validation from
  `VALIDATION.md`;
- stop before adding simulation-focused work or broader protocol scope.

Validation:
- run the applicable commands from `VALIDATION.md`;
- report exactly what passed, what failed, and whether any failure is due to
  environment/dependency availability rather than a code regression.
