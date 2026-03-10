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

## Milestone 6 regression runs

```bash
cargo test -p overlay-core records::tests
cargo test -p overlay-core relay::tests
cargo test -p overlay-core --test integration_relay_fallback
```

## Notes

- Milestone 5 is considered closed, and Milestone 6 relay intro/fallback work is now active in code.
- Use the Milestone 1-5 regression runs above as the primary checks for baseline regressions while Milestone 6 continues to land.
- If `REPOSITORY_STAGE`, milestone prompts, or other status markers change, rerun the stage-boundary smoke tests so code and docs stay aligned.
- `integration_publish_lookup` remains the real Milestone 5 integration path; `integration_relay_fallback` is now a real Milestone 6 integration path; `integration_service_open` remains a stage-boundary smoke test until its later milestone lands.
- `rendezvous::tests` now also covers deterministic publish/lookup message vectors in `tests/vectors/rendezvous_messages.json`.
- `relay::tests` now also covers deterministic relay intro message vectors in `tests/vectors/relay_intro_messages.json`.
- If the default temp directory is not writable in your environment, prefix the build, lint, and test commands with `TMPDIR=/tmp`.

If a command fails, report exactly which command failed and whether it failed because:
- the milestone has not introduced that subsystem yet;
- dependencies are not wired yet;
- a real regression was introduced;
- the local environment blocked temp-file or linker access.
