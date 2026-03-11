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

## Milestone 14 pilot launch gate

```bash
./devnet/run-launch-gate.sh
```

Equivalent explicit command order:

```bash
cargo fmt --all --check
TMPDIR=/tmp cargo clippy --workspace --all-targets --all-features -- -D warnings
TMPDIR=/tmp cargo check --workspace
TMPDIR=/tmp cargo test --workspace
TMPDIR=/tmp cargo test -p overlay-core --test integration_bootstrap
TMPDIR=/tmp cargo test -p overlay-core --test integration_publish_lookup
TMPDIR=/tmp cargo test -p overlay-core --test integration_relay_fallback
TMPDIR=/tmp cargo test -p overlay-core --test integration_routing
TMPDIR=/tmp cargo test -p overlay-core --test integration_service_open
./devnet/run-smoke.sh
./devnet/run-restart-smoke.sh
```

## Milestone 11 local devnet smoke

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- smoke --devnet-dir devnet
```

or

```bash
./devnet/run-smoke.sh
```

## Milestone 12 local runtime soak

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- smoke --devnet-dir devnet --soak-seconds 1800 --status-interval-seconds 300
```

or

```bash
./devnet/run-soak.sh
```

## Milestone 15 distributed localhost smoke

```bash
./devnet/run-distributed-smoke.sh
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
cargo test -p overlay-core bootstrap::tests
cargo test -p overlay-core config::tests
cargo test -p overlay-core metrics::tests
cargo test -p overlay-core peer::tests
cargo test -p overlay-core rendezvous::tests
cargo test -p overlay-core records::tests
cargo test -p overlay-core relay::tests
cargo test -p overlay-core routing::tests
cargo test -p overlay-core service::tests
cargo test -p overlay-core session::manager::tests
cargo test -p overlay-core transport::tests
cargo test -p overlay-core --test integration_bootstrap
cargo test -p overlay-core --test integration_publish_lookup
cargo test -p overlay-core --test integration_relay_fallback
cargo test -p overlay-core --test integration_routing
cargo test -p overlay-core --test integration_service_open
```

## Notes

- Milestones 1-12 are considered implemented baseline work, and the current
  repository stage marker is `milestone-14-launch-gate` (Milestone 14 launch
  gate and pilot tag).
- Use the Milestone 1-12 regression runs, stage-boundary smoke tests, and the
  Milestone 14 pilot launch gate as the primary checks for the frozen pilot
  baseline.
- If `REPOSITORY_STAGE`, `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, milestone
  prompts, or other status markers change, rerun the stage-boundary smoke
  tests so code and docs stay aligned.
- `integration_publish_lookup` remains the real Milestone 5 integration path; `integration_relay_fallback` is the real Milestone 6 integration path; `integration_routing` is the real Milestone 7 integration path; `integration_service_open` is now the real Milestone 8 integration path.
- Milestone 9 hardening coverage remains part of the frozen baseline through
  `bootstrap::tests`, `config::tests`, `metrics::tests`, `peer::tests`,
  `rendezvous::tests`, `relay::tests`, `routing::tests`, `service::tests`,
  `session::manager::tests`, and `transport::tests`.
- `bootstrap::tests` now also covers bootstrap provider fetch/validation
  observability for accepted, rejected, and unavailable provider outcomes.
- `transport::tests` now also covers bounded transport-buffer config
  validation and oversized received-frame rejection.
- `session::manager::tests` now also covers converting bounded
  `TransportPollEvent` values into runner inputs and rejecting oversized
  transport frames at that boundary.
- `bootstrap::tests` now also covers unsupported schema versions,
  `generated_at_unix_s > expires_at_unix_s`, blank `network_id`, zero
  `epoch_duration_s`, zero `presence_ttl_s`, zero or oversized
  `max_frame_body_len`, duplicate peer-node rejection, duplicate bridge-hint
  rejection, blank peer dial-hint rejection after trimming, and expired bridge
  hints.
- `rendezvous::tests` now also covers deterministic publish/lookup message vectors in `tests/vectors/rendezvous_messages.json`, derived placement-key validation on `PublishAck` / `LookupResult` / `LookupNotFound`, and `LookupResult.record.node_id` shape validation.
- `relay::tests` now also covers deterministic relay intro message vectors in `tests/vectors/relay_intro_messages.json` and oversize relay wire-body rejection.
- `routing::tests` now covers deterministic path-probe message vectors, bounded local probe tracking, the deterministic path-score formula, integer EWMA updates, hysteresis thresholds, switch-rate caps, and oversize probe-body rejection.
- `routing::tests` now also covers observability wrapper rejection logging for
  unknown probe completions and `selected_initial` route-selection logging
  without incrementing `path_switch_total`.
- `service::tests` now covers deterministic service message vectors in `tests/vectors/service_messages.json`, verified `ServiceRecord` registration, exact `app_id` resolution, `reachability_ref` binding checks, local open-session limits, policy denials, invalid response/result wire shapes, and oversize service wire-body rejection.
- `service::tests` now also covers observability wrapper logs for rejected
  registration, not-found resolution, rejected-not-found session opens,
  rejected reachability mismatches, rejected policy opens, and close-session
  not-found outcomes.
- `peer::tests` now also covers rejected bootstrap ingest observability without clobbering the active-peer gauge.
- `session::manager::tests` now also covers bounded handshake transcript replay-cache validation, rejection, pruning, oldest-entry eviction, and explicit established-session gauge synchronization.
- routing probe message vectors live in `tests/vectors/path_probe_messages.json`.
- The local devnet smoke flow is intentionally in-process and local-file based:
  it validates the sample configs, runtime startup, handshake-backed placeholder
  session establishment, verified presence publish handoff, exact lookup,
  service open, and one relay-fallback path without introducing real network
  listeners.
- The Milestone 14 launch gate adds a bounded restart smoke and freezes the
  required command order for pilot tags, but it does not replace the Milestone
  12 logical soak as an optional supporting hardening check.
- The Milestone 12 soak path also stays in-process and advances logical time
  through repeated runtime ticks so stale-session/service/relay/probe cleanup,
  bootstrap retry, and health snapshots can be exercised without a separate
  simulation platform or long wall-clock sleeps.
- `overlay-cli run --status-every <ticks>` now emits JSON health snapshots with
  runtime counts, observability counters, relay usage, cleanup totals, and
  resource-limit surfaces for a single node.
- `./devnet/run-distributed-smoke.sh` is the minimal real-socket regression
  path for listener bind, outbound dial, accept, and handshake-backed session
  establishment across separate localhost processes.
- If the default temp directory is not writable in your environment, prefix the build, lint, and test commands with `TMPDIR=/tmp`.

If a command fails, report exactly which command failed and whether it failed because:
- the milestone has not introduced that subsystem yet;
- dependencies are not wired yet;
- a real regression was introduced;
- the local environment blocked temp-file or linker access.
