# Overlay

Specification-first Rust workspace for a censorship-resistant overlay network.

The repository is past Milestone 0.

Current implemented baseline:
- Milestone 1 foundations are present in `overlay-core` (`identity`, `records`, `wire`).
- Milestone 2 crypto and handshake surface are present in `overlay-core` (`crypto`, `session::handshake`) with handshake tests and vectors.
- Milestone 3 and later remain placeholder modules and smoke-test stubs.

Treat the repo as a Milestone 1 + Milestone 2 baseline that needs doc sync,
missing artifact cleanup, and validation before any Milestone 3 work begins.
