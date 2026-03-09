Read `AGENTS.md`, `IMPLEMENT.md`, `spec/mvp-scope.md`, `spec/architecture.md`,
`docs/OPEN_QUESTIONS.md`, and the bootstrap/peer modules under
`crates/overlay-core/src/`.

Goal:
Audit or repair the closed Milestone 4 peer/bootstrap baseline from
`IMPLEMENT.md` without broadening scope into Milestone 5+ feature work.

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
- touch Milestone 4 only for regression fixes, fixture maintenance,
  bootstrap-schema adjustments, validation maintenance, or conservative
  spec-conformance fixes;
- preserve the explicit bootstrap/peer boundary and bounded peer-store policy
  unless the task explicitly changes them;
- keep status docs, prompts, fixtures, and `docs/OPEN_QUESTIONS.md` aligned if
  the documented Milestone 4 baseline changes;
- do not add Milestone 5+ presence/lookup behavior here.

Constraints:
- do not rework Milestones 1-3 except for a concrete regression or spec mismatch;
- keep bootstrap responses advisory only;
- preserve diversity-driven peer selection constraints from `IMPLEMENT.md`.

Validation:
- run the applicable commands from `VALIDATION.md`;
- keep the Milestone 1-4 regression runs clean while later milestones land.
