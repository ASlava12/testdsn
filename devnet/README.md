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
