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

## Notes

- Milestone 4 is considered closed, and active feature work now begins at Milestone 5.
- Use the Milestone 1-4 regression runs above as the primary checks for baseline regressions while Milestone 5 and later land.
- If `REPOSITORY_STAGE`, milestone prompts, or other status markers change, rerun the stage-boundary smoke tests so code and docs stay aligned.
- The stage-boundary integration tests remain smoke tests until the Milestone 5-8 subsystems land.
- If the default temp directory is not writable in your environment, prefix the build, lint, and test commands with `TMPDIR=/tmp`.

If a command fails, report exactly which command failed and whether it failed because:
- the milestone has not introduced that subsystem yet;
- dependencies are not wired yet;
- a real regression was introduced;
- the local environment blocked temp-file or linker access.
