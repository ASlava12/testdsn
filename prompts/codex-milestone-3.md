Read `AGENTS.md`, `IMPLEMENT.md`, `spec/mvp-scope.md`, `spec/architecture.md`,
`spec/state-machines.md`, `docs/OPEN_QUESTIONS.md`, and the session/transport
modules under `crates/overlay-core/src/`.

Goal:
Audit or repair the closed Milestone 3 transport/session baseline from
`IMPLEMENT.md` without broadening scope into Milestone 4+ feature work.

Current repository baseline:
- Milestone 0 is complete.
- Milestone 1 identities, records, and wire foundations are implemented,
  vectorized, and validated.
- Milestone 2 crypto wrappers and handshake surface are implemented,
  vectorized, validated, and considered closed.
- Milestone 3 already has a closed transport/session baseline with an explicit
  runner boundary and bounded local stores.
- Milestone 4 is closed; Milestone 5 presence/lookup work is closed; Milestone 6
  relay intro/fallback work is active; Milestone 7 and later are still
  placeholders.

Requirements:
- touch Milestone 3 only for regression fixes, fixture maintenance, runner-boundary
  adjustments, validation maintenance, or conservative spec-conformance fixes;
- keep session states and transitions aligned with `spec/state-machines.md`;
- preserve the explicit runner boundary and bounded local stores unless the task
  explicitly changes them;
- do not add Milestone 4+ peer/bootstrap behavior here;
- keep status docs, open questions, fixtures, and baseline validation aligned if
  the Milestone 3 baseline changes.

Constraints:
- do not rework Milestone 1-2 code except for a concrete regression or spec mismatch;
- do not implement full real QUIC/WS behavior yet unless the task explicitly requires it;
- preserve explicit layering between transport/session and peer/bootstrap logic.

Validation:
- run the applicable commands from `VALIDATION.md`;
- if the work only touches Milestone 3 scaffolding, still keep the Milestone 1-2 regression runs clean.
