Read `AGENTS.md`, `IMPLEMENT.md`, `VALIDATION.md`, `docs/OPEN_QUESTIONS.md`,
and the relevant files under `spec/`.

Goal:
Continue Milestone 3 from the current aligned baseline while keeping status
documents and conservative defaults synchronized to the actual code state.

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
- continue the current `transport` / `session::manager` skeleton without
  broadening scope beyond documented Milestone 3 work;
- keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, affected prompts, and
  `docs/OPEN_QUESTIONS.md` synchronized if the repository baseline changes;
- treat Milestones 1-2 as closed baseline work, touching them only for
  regression fixes, vector maintenance, or validation maintenance;
- run the applicable Milestone 1-3 and stage-boundary validation from
  `VALIDATION.md`;
- stop before adding Milestone 4+ protocol behavior.

Validation:
- run the applicable commands from `VALIDATION.md`;
- report exactly what passed, what failed, and whether any failure is due to
  environment/dependency availability rather than a code regression.
