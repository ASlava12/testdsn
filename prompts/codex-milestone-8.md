Read `AGENTS.md`, `IMPLEMENT.md`, `VALIDATION.md`, `spec/mvp-scope.md`,
`spec/architecture.md`, `spec/service-layer.md`, `spec/wire-protocol.md`,
`spec/records.md`, `docs/OPEN_QUESTIONS.md`, and the service-related modules
under `crates/overlay-core/src/`.

Goal:
Audit or repair the closed Milestone 8 service-layer baseline from the current
closed Milestone 1-8 repository state.

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
- Milestone 8 service-layer work is closed in `overlay-core` (`service`) with a
  bounded local service registry and open-session store, canonical service
  wire bodies, verified `ServiceRecord` registration, exact `app_id`
  resolution, `reachability_ref` binding checks, allow/deny local policy
  enforcement, deterministic message vectors, and
  `integration_service_open` coverage.
- The current repository stage is `milestone-9-hardening` (Milestone 9
  hardening and polish).

Requirements:
- keep exact `app_id` service resolution aligned with `spec/service-layer.md`,
  `spec/wire-protocol.md`, and `spec/records.md`;
- keep Milestone 8 limited to regression fixes, validation maintenance,
  vectors, or conservative spec-conformance fixes without broadening into
  active Milestone 9 hardening work;
- preserve the separation between node reachability and service access;
- keep explicit layering between service, routing, relay, rendezvous,
  transport/session, and identity code;
- keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, affected prompts, and
  `docs/OPEN_QUESTIONS.md` synchronized if the Milestone 8 baseline changes;
- stop before adding Milestone 9 hardening behavior beyond regression repair.

Constraints:
- no global service enumeration;
- do not rework Milestones 1-7 except for a concrete regression or spec
  mismatch;
- keep local registries, policies, and stores bounded;
- do not bypass the existing closed lookup and reachability layers when opening
  a service session.

Validation:
- run the applicable commands from `VALIDATION.md`;
- keep the Milestone 1-8 regression runs clean while Milestone 9 lands.
