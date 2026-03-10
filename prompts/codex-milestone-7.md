Read `AGENTS.md`, `IMPLEMENT.md`, `spec/mvp-scope.md`, `spec/architecture.md`,
`spec/routing.md`, `docs/OPEN_QUESTIONS.md`, and the routing modules under
`crates/overlay-core/src/`.

Goal:
Continue Milestone 7 routing metrics and path switching work from the current
closed Milestone 1-6 baseline.

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
- Milestone 5 rendezvous/presence publish and exact lookup work is implemented,
  validated, and considered closed.
- Milestone 6 relay intro/fallback work is implemented, validated, and
  considered closed.
- Milestone 7 routing/path work is active in `overlay-core` (`routing`) with
  canonical `PathProbe` / `PathProbeResult` bodies, a bounded local probe
  tracker, deterministic path metrics, integer EWMA observation updates, path
  scoring, switch hysteresis, and anti-flapping tests.
- Milestone 8 and later are still placeholders.

Requirements:
- keep explicit layering between routing, relay, transport/session,
  rendezvous, and service code;
- keep the deterministic score formula and hysteresis defaults aligned with
  `spec/routing.md` and `docs/OPEN_QUESTIONS.md`;
- keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, affected prompts, and
  `docs/OPEN_QUESTIONS.md` synchronized if the Milestone 7 baseline changes;
- stop before adding Milestone 8+ service behavior.

Constraints:
- do not rework Milestones 1-6 except for a concrete regression or spec mismatch;
- do not invent non-deterministic route choice or hidden background work;
- preserve deterministic tie-breaking and anti-flapping behavior.

Validation:
- run the applicable commands from `VALIDATION.md`;
- keep the Milestone 1-6 regression runs clean while Milestone 7 lands.
