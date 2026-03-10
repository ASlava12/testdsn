# Devnet

The repository ships a reproducible four-node local devnet under
[devnet](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet).

It is the operator-facing proof path for the current runtime surface.

## Node roles

- `node-a`: bootstrap anchor and smoke-flow client.
- `node-b`: presence publisher and service host.
- `node-c`: extra standard peer so bootstrap is not a two-node edge case.
- `node-relay`: relay-enabled node for the documented fallback path.

## Files

- [devnet/configs](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/configs): four runnable `OverlayConfig` files.
- [devnet/keys](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/keys): deterministic seed files in hex form.
- [devnet/bootstrap](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/bootstrap): local bootstrap seed JSON files.
- [devnet/run-smoke.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-smoke.sh): wrapper for the smoke flow.
- [devnet/run-soak.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-soak.sh): wrapper for the logical soak.

## Smoke flow

Run either command:

```bash
./devnet/run-smoke.sh
```

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- smoke --devnet-dir devnet
```

Expected step sequence:

1. `startup` for all four nodes.
2. `session_established` from `node-a` to `node-b`.
3. `publish_presence` for `node-b`.
4. `lookup_node` from `node-a` to `node-b`.
5. `open_service` against `node-b`.
6. `relay_fallback_planned`.
7. `relay_fallback_bound`.
8. `smoke_complete`.

What this proves:

- the sample configs load;
- bootstrap files validate and populate local peers;
- the placeholder session runner can carry a real handshake outcome;
- a verified presence record can be published and looked up locally;
- a verified service record can be registered and opened locally;
- one direct-first, relay-second fallback path works.

## Soak flow

Run either command:

```bash
./devnet/run-soak.sh
```

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- smoke --devnet-dir devnet --soak-seconds 1800 --status-interval-seconds 300
```

This is a logical soak, not a 30-minute wall-clock sleep. The harness advances
runtime time in-process and emits periodic `runtime_status` steps.

The soak currently checks that:

- stale managed sessions are reaped;
- stale service-open sessions are pruned;
- relay tunnels age out and are removed;
- stale path probes become bounded loss observations;
- `node-b` refreshes its local presence during the run.

## Single-node inspection

Use `overlay-cli run` when you want one node's raw logs and status snapshots
without the in-process orchestration:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config devnet/configs/node-a.json --max-ticks 2 --status-every 1
```

## Devnet limits

- The devnet does not create real sockets or listeners.
- Bootstrap remains local-file based.
- Presence propagation, exact lookup visibility, and service open are performed
  inside the harness rather than over a distributed control plane.
- Dial hints are configuration artifacts for coherence only; the runtime does
  not bind them to live endpoints.
- Relay fallback is demonstrated for one local path only:
  `node-a -> node-relay -> node-b`.
