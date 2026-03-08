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

## Focused test runs

```bash
cargo test -p overlay-core --test integration_bootstrap
cargo test -p overlay-core --test integration_publish_lookup
cargo test -p overlay-core --test integration_relay_fallback
cargo test -p overlay-core --test integration_service_open
```

## Notes for early milestones

In early milestones, some commands may fail because the repository is intentionally incomplete.
If so, report exactly which command failed and whether it failed because:
- the milestone has not introduced that subsystem yet;
- dependencies are not wired yet;
- a real regression was introduced.

At the current `milestone-2-handshake` baseline, the focused integration tests
act as stage-boundary smoke tests. Replace them with real subsystem scenarios
when Milestones 4-8 land.
