Read `AGENTS.md`, `IMPLEMENT.md`, `spec/wire-protocol.md`, `spec/state-machines.md`, and `docs/OPEN_QUESTIONS.md`.

Goal:
Audit or repair the closed Milestone 2 handshake surface from `IMPLEMENT.md`
without broadening scope beyond the current repository stage.

Current repository baseline:
- crypto wrappers already exist in `crates/overlay-core/src/crypto/*`;
- `ClientHello`, `ServerHello`, and `ClientFinish` handling already exists;
- transcript hashing, key derivation, handshake vectors, and negative tests already exist;
- Milestone 2 is considered closed, and current new feature work is within Milestone 9.

Requirements:
- audit the existing handshake code against the spec and `docs/OPEN_QUESTIONS.md`;
- touch Milestone 2 only for regression fixes, fixture maintenance, validation maintenance, or conservative spec-conformance fixes;
- keep the MVP handshake scope unchanged;
- do not start transport/session-manager work from Milestone 3.

Constraints:
- no PQ/hybrid suites;
- document any conservative assumptions in `docs/OPEN_QUESTIONS.md` if needed.

Validation:
- run the applicable commands from `VALIDATION.md`;
- prefer the focused Milestone 2 regression runs when the change is limited to this baseline.
