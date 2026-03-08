Read `AGENTS.md`, `IMPLEMENT.md`, `spec/records.md`, `spec/wire-protocol.md`, and `spec/mvp-scope.md`.

Goal:
Implement Milestone 1 from `IMPLEMENT.md`.

Requirements:
- implement `NodeId` and `AppId` derivation;
- implement record structs and validation helpers;
- implement frame header and base message type definitions;
- add unit tests for ID derivation and header round-trip;
- do not add network I/O.

Validation:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`

Report:
- changed files
- tests added
- remaining underspecified areas
