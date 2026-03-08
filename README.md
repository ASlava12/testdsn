# Overlay

Specification-first Rust workspace for a censorship-resistant overlay network.

Current repository status:
- Milestone 0 bootstrap is complete.
- Milestone 1 foundations in `overlay-core` (`identity`, `records`, `wire`) are implemented and covered by deterministic vectors in `tests/vectors/`, including `node_id`, `app_id`, `frame_header`, and record fixtures.
- Milestone 2 crypto and handshake surface in `overlay-core` (`crypto`, `session::handshake`) are implemented, covered by the handshake transcript vector, and validated with negative tests for version, identity binding, signature, client-finish, and replay-unsafe shared-secret rejection.
- Milestone 2 is considered closed; next work starts at Milestone 3.
- Milestone 3 and later remain placeholder modules or stage-boundary smoke tests until that work begins.

Validation commands live in `VALIDATION.md`. In sandboxed Linux-on-Windows environments, set `TMPDIR=/tmp` for commands that link test binaries if the default temp directory is not writable.
