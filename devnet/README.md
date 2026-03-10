# Local Devnet

This directory contains a minimal 4-node local devnet that reuses the Milestone 10 runtime and drives the Milestone 1-8 in-memory subsystems through one reproducible smoke flow.

Nodes:
- `node-a`: bootstrap anchor and smoke-flow client.
- `node-b`: presence publisher and service host.
- `node-c`: extra peer so the seed set is not a 2-node degenerate case.
- `node-relay`: relay-enabled node for the fallback scenario.

## Files

- `configs/*.json`: example `OverlayConfig` files.
- `keys/*.key`: deterministic Ed25519 seed files in hex form.
- `bootstrap/*.json`: local bootstrap seed files used by the runtime startup path.
- `run-smoke.sh`: wrapper around `overlay-cli smoke`.
- `run-restart-smoke.sh`: wrapper that runs the same checked-in service-host
  config twice for a bounded restart smoke.
- `run-launch-gate.sh`: wrapper around the full Milestone 14 pilot launch gate.
- `run-soak.sh`: wrapper around `overlay-cli smoke --soak-seconds ...` for the
  logical long-run runtime soak.

## Run The Smoke Flow

From the repository root:

```bash
./devnet/run-smoke.sh
```

The smoke command prints one JSON object per step so failures stop on the exact stage:

1. start `node-a`, `node-b`, `node-c`, and `node-relay`;
2. establish a real handshake-backed placeholder session from `node-a` to `node-b`;
3. sign and publish `node-b` presence, then inject the verified record into `node-a`'s local lookup store;
4. exact-lookup `node-b` from `node-a`;
5. register a `devnet/terminal` service on `node-b` and open an app session to it;
6. build a reachability plan for `node-b`, force the direct path to fail locally, and bind the fallback tunnel on `node-relay`.

## Inspect A Single Node

To watch the existing runtime startup and tick logs for one config:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config devnet/configs/node-a.json --max-ticks 2
```

To emit periodic runtime health snapshots while the node ticks:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config devnet/configs/node-a.json --max-ticks 120 --status-every 30
```

Each status dump includes:
- runtime state plus peer/session/path/service counts;
- publish/lookup/session/relay/probe observability counters;
- relay usage, cleanup totals, and the effective local resource limits.

## Long-Run Soak

For a logical 30-minute local soak without wall-clock sleeps:

```bash
./devnet/run-soak.sh
```

This drives the same 4-node in-process devnet, advances logical time through
repeated runtime ticks, and checks that:
- stale placeholder sessions are reaped after timeout;
- stale service-open sessions are pruned;
- relay tunnels are cleaned up after the local retention window;
- expired path probes are converted into bounded local loss observations;
- node-b keeps refreshing its installed local presence with rolled freshness
  during the soak.

## Restart Smoke

For the bounded restart smoke:

```bash
./devnet/run-restart-smoke.sh
```

This runs the same checked-in service-host config twice with `overlay-cli run
--max-ticks 0 --status-every 1` so the current in-memory runtime restart path
is covered by a reproducible local command.

## Full Launch Gate

For the full Milestone 14 pilot gate:

```bash
./devnet/run-launch-gate.sh
```

This executes the required launch-order checks:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo check --workspace`
- `cargo test --workspace`
- the stage-boundary integration tests
- the devnet smoke
- the restart smoke

## Relay Fallback Scenario

The documented fallback path is `node-a -> node-relay -> node-b`.

- `node-b` publishes a hybrid presence record with direct `quic`/`tcp` attempts plus relay support.
- The smoke harness creates a fresh intro ticket for `node-a`.
- The direct path is intentionally treated as unavailable inside the smoke harness.
- `node-relay` accepts `ResolveIntro` and binds a relay tunnel for the fallback.

## Local-Only Assumptions

- Bootstrap is local-file only because the Milestone 10 runtime does not fetch network bootstrap providers yet.
- The session step uses the existing placeholder transport boundary with a real handshake outcome, not live sockets.
- Presence propagation, exact lookup visibility, and service open are orchestrated in-process after signature verification instead of over a distributed control plane.
- Dial hints in the seed files are illustrative local endpoints for config coherence; the current runtime does not bind them to real listeners.
