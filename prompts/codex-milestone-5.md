Read `AGENTS.md`, `IMPLEMENT.md`, `spec/mvp-scope.md`, `spec/architecture.md`,
`spec/records.md`, `spec/wire-protocol.md`, `docs/OPEN_QUESTIONS.md`, and the
rendezvous modules under `crates/overlay-core/src/`.

Goal:
Audit or repair the closed Milestone 5 presence publish and exact lookup
baseline from the current closed Milestone 1-5 repository state.

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
- Milestone 6 relay intro/fallback work is active.
- Milestone 7 and later are still placeholders.

Requirements:
- keep the current `publish_verified` contract intact: signature verification
  happens upstream from the rendezvous store boundary;
- keep Milestone 5 limited to regression fixes, validation maintenance,
  vectors, or conservative spec-conformance fixes without broadening into
  active Milestone 6 relay logic;
- preserve explicit layering between rendezvous/presence and the already closed
  bootstrap, transport/session, and active relay baselines;
- keep status docs, prompts, fixtures, and `docs/OPEN_QUESTIONS.md` aligned if
  the documented baseline changes;
- stop before adding Milestone 6+ behavior.

Constraints:
- do not rework Milestones 1-4 except for a concrete regression or spec mismatch;
- reject expired records as fresh lookup results;
- keep lookup exact-only and non-enumerable.

Validation:
- run the applicable commands from `VALIDATION.md`;
- keep the Milestone 1-5 regression runs clean while Milestone 6 lands.
