Read `AGENTS.md`, `IMPLEMENT.md`, `spec/records.md`, `spec/wire-protocol.md`, and `spec/mvp-scope.md`.

Goal:
Finish or validate the existing Milestone 1 baseline from `IMPLEMENT.md`
without rewriting work that is already in the repository.

Current repository baseline:
- `NodeId` and `AppId` derivation already exist.
- record structs already exist.
- frame header and message catalog already exist.
- the remaining work in this milestone should be gap-filling, vectors, tests,
  and spec-conformance fixes.

Requirements:
- verify the current `identity`, `records`, and `wire` modules against spec;
- add only the missing Milestone 1 artifacts such as fixtures, vectors, or tests;
- keep network I/O out of scope;
- do not refactor stable code paths without a concrete bug or spec mismatch.

Validation:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace`

Report:
- changed files
- tests added
- remaining underspecified areas
