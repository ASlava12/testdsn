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
cargo test -p overlay-core --test integration_routing
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

## Milestone 7 regression runs

```bash
cargo test -p overlay-core routing::tests
cargo test -p overlay-core --test integration_routing
```

## Milestone 8 regression runs

```bash
cargo test -p overlay-core service::tests
cargo test -p overlay-core --test integration_service_open
```

## Milestone 9 hardening runs

```bash
cargo test -p overlay-core config::tests
cargo test -p overlay-core metrics::tests
cargo test -p overlay-core peer::tests
cargo test -p overlay-core rendezvous::tests
cargo test -p overlay-core records::tests
cargo test -p overlay-core relay::tests
cargo test -p overlay-core routing::tests
cargo test -p overlay-core service::tests
cargo test -p overlay-core session::manager::tests
cargo test -p overlay-core --test integration_bootstrap
cargo test -p overlay-core --test integration_publish_lookup
cargo test -p overlay-core --test integration_relay_fallback
cargo test -p overlay-core --test integration_routing
cargo test -p overlay-core --test integration_service_open
```

## Notes

- Milestone 8 is considered closed, and Milestone 9 hardening and polish is now the active repository stage.
- Use the Milestone 1-8 regression runs above as the primary checks for baseline regressions while Milestone 9 continues to land.
- If `REPOSITORY_STAGE`, milestone prompts, or other status markers change, rerun the stage-boundary smoke tests so code and docs stay aligned.
- `integration_publish_lookup` remains the real Milestone 5 integration path; `integration_relay_fallback` is the real Milestone 6 integration path; `integration_routing` is the real Milestone 7 integration path; `integration_service_open` is now the real Milestone 8 integration path.
- Milestone 9 currently extends the closed-baseline regression runs and
  stage-boundary smoke tests with `config::tests`, `metrics::tests`,
  `peer::tests`, `rendezvous::tests`, and `session::manager::tests` while
  broader hardening coverage continues to land.
- `rendezvous::tests` now also covers deterministic publish/lookup message vectors in `tests/vectors/rendezvous_messages.json`.
- `relay::tests` now also covers deterministic relay intro message vectors in `tests/vectors/relay_intro_messages.json`.
- `routing::tests` now covers deterministic path-probe message vectors, bounded local probe tracking, the deterministic path-score formula, integer EWMA updates, hysteresis thresholds, and switch-rate caps.
- `service::tests` now covers deterministic service message vectors in `tests/vectors/service_messages.json`, verified `ServiceRecord` registration, exact `app_id` resolution, `reachability_ref` binding checks, local open-session limits, and policy denials.
- routing probe message vectors live in `tests/vectors/path_probe_messages.json`.
- If the default temp directory is not writable in your environment, prefix the build, lint, and test commands with `TMPDIR=/tmp`.

If a command fails, report exactly which command failed and whether it failed because:
- the milestone has not introduced that subsystem yet;
- dependencies are not wired yet;
- a real regression was introduced;
- the local environment blocked temp-file or linker access.
