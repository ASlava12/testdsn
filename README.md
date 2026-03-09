# Overlay

Specification-first Rust workspace for a censorship-resistant overlay network.

Current repository status:
- Milestone 0 bootstrap is complete.
- Milestone 1 foundations in `overlay-core` (`identity`, `records`, `wire`) are implemented and covered by deterministic vectors in `tests/vectors/`, including `node_id`, `app_id`, `frame_header`, and record fixtures.
- Milestone 2 crypto and handshake surface in `overlay-core` (`crypto`, `session::handshake`) are implemented, covered by the handshake transcript vector, and validated with negative tests for version, identity binding, signature, client-finish, and replay-unsafe shared-secret rejection.
- Milestone 2 is considered closed.
- Milestone 3 now has a minimal compileable transport/session skeleton in `overlay-core` (`transport`, `session::manager`) with placeholder adapters, structured session events, handshake-bound session context, explicit polled keepalive/timeout scaffolding, a queued I/O-action surface for future runners, explicit degraded-to-established recovery, and state-transition unit tests. It intentionally excludes bootstrap, rendezvous, relay intro, and real path logic.
- Milestone 4 and later remain placeholder modules or stage-boundary smoke tests.

Before more Milestone 3 feature work, keep the repository aligned to this baseline: sync status docs and prompts to the actual code state, lock conservative MVP defaults in `docs/OPEN_QUESTIONS.md`, keep Milestone 1 identity vectors complete, and re-run the focused validation listed in `VALIDATION.md`.

In sandboxed Linux-on-Windows environments, set `TMPDIR=/tmp` for commands that link test binaries if the default temp directory is not writable.
