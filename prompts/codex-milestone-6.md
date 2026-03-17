Read `AGENTS.md`, `IMPLEMENT.md`, `spec/mvp-scope.md`, `spec/architecture.md`,
`spec/relay.md`, `spec/records.md`, `docs/OPEN_QUESTIONS.md`, and the relay
modules under `crates/overlay-core/src/`.

Goal:
Audit or repair the closed Milestone 6 relay intro and fallback baseline from
the current closed Milestone 1-8 repository state.

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
- Milestone 6 relay intro/fallback work is closed in `overlay-core` (`relay`)
  with a minimal local relay role model, bounded quota enforcement, canonical
  `ResolveIntro` / `IntroResponse` bodies, verified `IntroTicket` usage, and
  direct-first fallback planning.
- Milestone 7 routing work is closed in `overlay-core` (`routing`).
- Milestone 8 service-layer work is closed in `overlay-core` (`service`).
- The current repository stage is
  `milestone-22-first-user-acceptance-pack`.

Requirements:
- keep direct transport attempts first and use relay only as bounded fallback;
- maintain secondary relay candidates instead of collapsing to one mandatory
  relay;
- preserve explicit layering between relay, transport/session, peer/bootstrap,
  and rendezvous;
- keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, affected prompts, and
  `docs/OPEN_QUESTIONS.md` synchronized if the Milestone 6 baseline changes;
- stop before broadening beyond the frozen launch surface.

Constraints:
- do not rework Milestones 1-5 except for a concrete regression or spec mismatch;
- enforce local relay quotas conservatively;
- reject expired relay hints or intro tickets as fresh fallback inputs.

Validation:
- run the applicable commands from `VALIDATION.md`;
- keep the Milestone 1-8 regression runs clean under the current launch gate.
