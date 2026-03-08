Read `AGENTS.md`, `IMPLEMENT.md`, `spec/wire-protocol.md`, `spec/state-machines.md`, and `docs/OPEN_QUESTIONS.md`.

Goal:
Implement Milestone 2 from `IMPLEMENT.md`.

Requirements:
- add crypto wrappers for BLAKE3, Ed25519, X25519, HKDF-SHA256, and ChaCha20-Poly1305;
- implement `ClientHello`, `ServerHello`, and `ClientFinish` base handling;
- implement transcript hashing and session key derivation in the simplest conservative MVP form;
- add tests for valid and invalid handshake paths.

Constraints:
- no PQ/hybrid suites;
- document any conservative assumptions in `docs/OPEN_QUESTIONS.md` if needed.

Validation:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`
