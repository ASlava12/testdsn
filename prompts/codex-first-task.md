Read `AGENTS.md`, `IMPLEMENT.md`, `VALIDATION.md`, `docs/OPEN_QUESTIONS.md`,
and the relevant files under `spec/`.

Goal:
Start Milestone 3 from the current closed Milestone 1-2 baseline without
reopening finished work.

Current repository baseline:
- Milestone 0 is already complete.
- Milestone 1 foundations are already implemented, vectorized, and validated in
  `overlay-core` (`identity`, `records`, `wire`).
- Milestone 2 crypto and handshake surface are already implemented, vectorized,
  and validated in `overlay-core` (`crypto`, `session::handshake`).
- Milestone 2 is considered closed; next work starts at Milestone 3.
- Milestone 3 and later are still placeholders.

Constraints:
- do not restart the repository from Milestone 0;
- do not refactor completed Milestone 1-2 code without a concrete bug or spec mismatch;
- do not advance protocol logic beyond the documented current stage;
- keep changes minimal and local.

Tasks:
- define the `Transport` trait;
- add placeholder transport adapters for TCP, QUIC, WebSocket/HTTPS tunnel, and relay transport;
- implement the session state machine surface from `spec/state-machines.md`;
- add keepalive/timeout scaffolding and structured session events;
- stop before Milestone 4 peer/bootstrap logic.

Validation:
- run the applicable commands from `VALIDATION.md`;
- report exactly what passed, what failed, and whether any failure is due to
  environment/dependency availability rather than a code regression.
