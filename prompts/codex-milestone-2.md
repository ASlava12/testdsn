Read `AGENTS.md`, `IMPLEMENT.md`, `spec/wire-protocol.md`, `spec/state-machines.md`, and `docs/OPEN_QUESTIONS.md`.

Goal:
Validate or conservatively complete the existing Milestone 2 handshake surface
from `IMPLEMENT.md` without broadening scope beyond the current repository stage.

Current repository baseline:
- crypto wrappers already exist in `crates/overlay-core/src/crypto/*`;
- `ClientHello`, `ServerHello`, and `ClientFinish` handling already exists;
- transcript hashing, key derivation, and handshake vectors already exist.

Requirements:
- audit the existing handshake code against the spec and `docs/OPEN_QUESTIONS.md`;
- add only missing validation, tests, vectors, or conservative fixes;
- keep the MVP handshake scope unchanged;
- do not start transport/session-manager work from Milestone 3.

Constraints:
- no PQ/hybrid suites;
- document any conservative assumptions in `docs/OPEN_QUESTIONS.md` if needed.

Validation:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
