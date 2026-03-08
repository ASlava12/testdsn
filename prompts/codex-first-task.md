Read `AGENTS.md`, `IMPLEMENT.md`, `VALIDATION.md`, `docs/OPEN_QUESTIONS.md`,
and the relevant files under `spec/`.

Goal:
Synchronize the repository's status documents with the actual code baseline and
finish any missing Milestone 1 / Milestone 2 artifacts without advancing into
Milestone 3.

Current repository baseline:
- Milestone 0 is already complete.
- Milestone 1 foundations already exist in `overlay-core` (`identity`, `records`, `wire`).
- Milestone 2 crypto and handshake surface already exist in `overlay-core`
  (`crypto`, `session::handshake`).
- Milestone 3 and later are still placeholders.

Constraints:
- do not restart the repository from Milestone 0;
- do not refactor completed Milestone 1-2 code without a concrete bug or spec mismatch;
- do not advance protocol logic beyond the documented current stage;
- keep changes minimal and local.

Tasks:
- sync `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, and prompt files to the current stage;
- lock down the agreed conservative MVP decisions in `docs/OPEN_QUESTIONS.md`;
- fill missing Milestone 1 fixtures such as `tests/vectors/node_id.json`;
- validate or conservatively complete the existing Milestone 2 handshake surface.

Validation:
- run the applicable commands from `VALIDATION.md`;
- report exactly what passed, what failed, and whether any failure is due to
  environment/dependency availability rather than a code regression.
