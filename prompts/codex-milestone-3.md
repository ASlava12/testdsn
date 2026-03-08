Read `AGENTS.md`, `IMPLEMENT.md`, `spec/mvp-scope.md`, `spec/architecture.md`,
`spec/state-machines.md`, `docs/OPEN_QUESTIONS.md`, and the session/transport
modules under `crates/overlay-core/src/`.

Goal:
Start Milestone 3 transport abstraction and session-manager work from the
current closed Milestone 1-2 baseline.

Current repository baseline:
- Milestone 0 is complete.
- Milestone 1 identities, records, and wire foundations are implemented,
  vectorized, and validated.
- Milestone 2 crypto wrappers and handshake surface are implemented,
  vectorized, validated, and considered closed.
- Milestone 3 and later are still placeholders.

Requirements:
- define the `Transport` trait;
- add placeholder transport adapters for TCP, QUIC, WebSocket/HTTPS tunnel,
  and relay transport;
- implement session states and transitions that match `spec/state-machines.md`;
- add keepalive/timeout scaffolding and structured session events;
- keep Milestone 4+ behavior out of scope.

Constraints:
- do not rework Milestone 1-2 code except for a concrete regression or spec mismatch;
- do not implement full real QUIC/WS behavior yet unless the task explicitly requires it;
- preserve explicit layering between transport/session and peer/bootstrap logic.

Validation:
- run the applicable commands from `VALIDATION.md`;
- if the work only touches Milestone 3 scaffolding, still keep the Milestone 1-2 regression runs clean.
