Read `AGENTS.md`, `IMPLEMENT.md`, `VALIDATION.md`, `docs/OPEN_QUESTIONS.md`,
and the relevant files under `spec/`.

Goal:
Synchronize the repository's status documents and conservative defaults with
the actual current code state before any further feature work.

Current repository baseline:
- Milestone 0 is already complete.
- Milestone 1 foundations are already implemented, vectorized, and validated in
  `overlay-core` (`identity`, `records`, `wire`).
- Milestone 2 crypto and handshake surface are already implemented, vectorized,
  and validated in `overlay-core` (`crypto`, `session::handshake`).
- Milestone 2 is considered closed.
- Milestone 3 already has a minimal compileable transport/session skeleton in
  `overlay-core` (`transport`, `session::manager`).
- Milestone 4 and later are still placeholders.

Constraints:
- do not restart the repository from Milestone 0;
- do not refactor completed Milestone 1-2 code without a concrete bug or spec mismatch;
- do not advance protocol logic beyond the documented current stage;
- keep changes minimal and local.

Tasks:
- sync `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, and any affected prompts to the
  actual repository baseline;
- lock conservative MVP defaults in `docs/OPEN_QUESTIONS.md` instead of silently
  inventing them later;
- fill missing Milestone 1 identity fixtures, especially `tests/vectors/node_id.json`,
  with real values and keep them aligned with existing record fixtures;
- run the applicable Milestone 1, Milestone 2, and stage-boundary validation from
  `VALIDATION.md`;
- stop before adding new Milestone 3+ protocol behavior.

Validation:
- run the applicable commands from `VALIDATION.md`;
- report exactly what passed, what failed, and whether any failure is due to
  environment/dependency availability rather than a code regression.
