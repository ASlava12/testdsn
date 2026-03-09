# VALIDATION.md

Run the following commands when applicable.

## Formatting

```bash
cargo fmt --all --check
```

## Lints

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Build

```bash
cargo check --workspace
```

## Tests

```bash
cargo test --workspace
```

## Milestone 1 regression runs

```bash
cargo test -p overlay-core identity::tests
cargo test -p overlay-core records::tests
cargo test -p overlay-core wire::tests
```

## Milestone 2 regression runs

```bash
cargo test -p overlay-core crypto::kex::tests
cargo test -p overlay-core session::handshake::tests
```

## Milestone 3 regression runs

```bash
cargo test -p overlay-core transport::tests
cargo test -p overlay-core session::manager::tests
cargo test -p overlay-core --test integration_session_handshake
```

## Milestone 4 regression runs

```bash
cargo test -p overlay-core bootstrap::tests
cargo test -p overlay-core peer::tests
cargo test -p overlay-core --test integration_bootstrap
```

## Stage-boundary smoke tests

```bash
cargo test -p overlay-core --test integration_bootstrap
cargo test -p overlay-core --test integration_publish_lookup
cargo test -p overlay-core --test integration_relay_fallback
cargo test -p overlay-core --test integration_service_open
```

## Milestone 5 regression runs

```bash
cargo test -p overlay-core rendezvous::tests
cargo test -p overlay-core --test integration_publish_lookup
```

## Notes

- Milestone 4 is considered closed, and Milestone 5 rendezvous/presence work is now active in code.
- Use the Milestone 1-4 regression runs above as the primary checks for baseline regressions while Milestone 5 continues to land.
- If `REPOSITORY_STAGE`, milestone prompts, or other status markers change, rerun the stage-boundary smoke tests so code and docs stay aligned.
- `integration_publish_lookup` is now a real Milestone 5 integration path; `integration_relay_fallback` and `integration_service_open` remain stage-boundary smoke tests until their later milestones land.
- `rendezvous::tests` now also covers deterministic publish/lookup message vectors in `tests/vectors/rendezvous_messages.json`.
- If the default temp directory is not writable in your environment, prefix the build, lint, and test commands with `TMPDIR=/tmp`.

If a command fails, report exactly which command failed and whether it failed because:
- the milestone has not introduced that subsystem yet;
- dependencies are not wired yet;
- a real regression was introduced;
- the local environment blocked temp-file or linker access.
