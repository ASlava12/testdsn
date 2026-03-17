Read `AGENTS.md`, `IMPLEMENT.md`, `spec/mvp-scope.md`, `spec/architecture.md`,
`spec/records.md`, `spec/wire-protocol.md`, `docs/OPEN_QUESTIONS.md`, and the
rendezvous modules under `crates/overlay-core/src/`.

Goal:
Audit or repair the closed Milestone 5 presence publish and exact lookup
baseline from the current closed Milestone 1-8 repository state.

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
- Milestone 5 rendezvous/presence publish and exact lookup work is closed in
  `overlay-core` (`rendezvous`) with bounded local publish/lookup state,
  freshness and conflict handling, verified-signature handoff at the store
  boundary, and integration coverage.
- Milestone 6 relay intro/fallback work is closed.
- Milestone 7 routing work is closed.
- Milestone 8 service-layer work is closed.
- The current repository stage is
  `milestone-24-bootstrap-trust-delivery-hardening`.

Requirements:
- keep the current `publish_verified` contract intact: signature verification
  happens upstream from the rendezvous store boundary;
- keep Milestone 5 limited to regression fixes, validation maintenance,
  vectors, or conservative spec-conformance fixes without broadening into
  closed Milestone 6-8 relay/routing/service logic or active Milestone 9
  hardening work;
- preserve explicit layering between rendezvous/presence and the already closed
  bootstrap, transport/session, relay, and routing baselines;
- keep status docs, prompts, fixtures, and `docs/OPEN_QUESTIONS.md` aligned if
  the documented baseline changes;
- stop before adding Milestone 6+ behavior beyond regression repair or
  broadening the frozen launch surface.

Constraints:
- do not rework Milestones 1-4 except for a concrete regression or spec mismatch;
- reject expired records as fresh lookup results;
- keep lookup exact-only and non-enumerable.

Validation:
- run the applicable commands from `VALIDATION.md`;
- keep the Milestone 1-8 regression runs clean under the current launch gate.
