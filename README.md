# Overlay

Specification-first Rust workspace for a censorship-resistant overlay network.

Current repository status:
- Milestone 0 bootstrap is complete.
- Milestone 1 foundations in `overlay-core` (`identity`, `records`, `wire`) are implemented and covered by deterministic vectors in `tests/vectors/`, including `node_id` fixtures tied to the shared record key and handshake transcript identities, `app_id`, `frame_header`, and record fixtures.
- Milestone 2 crypto and handshake surface in `overlay-core` (`crypto`, `session::handshake`) are implemented, covered by the handshake transcript vector, and validated with negative tests for version, identity binding, signature, client-finish, and replay-unsafe shared-secret rejection.
- Milestone 2 is considered closed.
- Milestone 3 transport/session work in `overlay-core` (`transport`, `session::manager`) now has an explicit placeholder runner boundary, runner-facing session input surface, bounded event and I/O-action stores, handshake-bound session context, integration coverage for handshake-to-session establishment, and state-transition tests. Milestone 3 is considered closed. It still intentionally excludes bootstrap, rendezvous, relay intro, and real path logic.
- Milestone 4 peer/bootstrap work in `overlay-core` (`bootstrap`, `peer`) now has validated bootstrap response types, a static bootstrap provider abstraction, a bounded peer store, deterministic diversity-preserving rebalance policy, and bootstrap integration coverage. Milestone 4 is considered closed.
- Milestone 5 and later remain placeholder modules or stage-boundary smoke tests.

The baseline docs and minimal identity fixtures are aligned to this repository state. The next active feature stage is Milestone 5 presence publish/exact lookup work; keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, prompts, and `docs/OPEN_QUESTIONS.md` in sync whenever that stage boundary changes.

In sandboxed Linux-on-Windows environments, set `TMPDIR=/tmp` for commands that link test binaries if the default temp directory is not writable.
