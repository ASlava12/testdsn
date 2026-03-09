Read `AGENTS.md`, `IMPLEMENT.md`, `spec/mvp-scope.md`, `spec/architecture.md`,
`spec/records.md`, `spec/wire-protocol.md`, `docs/OPEN_QUESTIONS.md`, and the
rendezvous modules under `crates/overlay-core/src/`.

Goal:
Start Milestone 5 presence publish and exact lookup work from the current
closed Milestone 1-4 baseline.

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
- Milestone 5 and later are still placeholders.

Requirements:
- implement exact `LookupNode` / `LookupResult` / `LookupNotFound` behavior with
  no prefix or range scan;
- implement presence publication and conflict handling using expiry, epoch, and
  sequence rules already locked in `docs/OPEN_QUESTIONS.md`;
- add bounded lookup budgets, seen-set, and negative-cache behavior without
  crossing into Milestone 6 relay logic;
- preserve explicit layering between rendezvous/presence and the already closed
  bootstrap and transport/session baselines;
- keep status docs, prompts, fixtures, and `docs/OPEN_QUESTIONS.md` aligned if
  the documented baseline changes;
- stop before adding Milestone 6+ behavior.

Constraints:
- do not rework Milestones 1-4 except for a concrete regression or spec mismatch;
- reject expired records as fresh lookup results;
- keep lookup exact-only and non-enumerable.

Validation:
- run the applicable commands from `VALIDATION.md`;
- keep the Milestone 1-4 regression runs clean while Milestone 5 lands.
