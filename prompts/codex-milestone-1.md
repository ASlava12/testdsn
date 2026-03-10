Read `AGENTS.md`, `IMPLEMENT.md`, `spec/records.md`, `spec/wire-protocol.md`, and `spec/mvp-scope.md`.

Goal:
Audit or repair the closed Milestone 1 baseline from `IMPLEMENT.md` without
rewriting work that is already in the repository.

Current repository baseline:
- `NodeId` and `AppId` derivation already exist and are covered by vectors.
- record structs already exist and are covered by deterministic record fixtures.
- frame header and message catalog already exist, including a frame header vector.
- Milestone 2 is also closed, and current new feature work is within Milestone 9.

Requirements:
- verify the current `identity`, `records`, and `wire` modules against spec;
- touch Milestone 1 only for regression fixes, fixture maintenance, or spec-conformance fixes;
- keep network I/O out of scope;
- do not refactor stable code paths without a concrete bug or spec mismatch.

Validation:
- run the applicable commands from `VALIDATION.md`;
- prefer the focused Milestone 1 regression runs when the change is limited to this baseline.

Report:
- changed files
- tests or vectors updated
- remaining underspecified areas
