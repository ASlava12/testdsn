# Devnet

The repository ships a reproducible four-node local devnet under
[devnet](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet).

It is the operator-facing proof path for the current runtime surface, including
the Milestone 16 host-style network-bootstrap layout under
[devnet/hosts](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/hosts)
and the Milestone 18 pilot pack under
[devnet/pilot](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot).

## Node roles

- `node-a`: bootstrap anchor and smoke-flow client.
- `node-b`: presence publisher and service host.
- `node-c`: extra standard peer so bootstrap is not a two-node edge case.
- `node-relay`: relay-enabled node for the documented fallback path.

## Files

- [devnet/configs](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/configs): four runnable `OverlayConfig` files.
- [devnet/keys](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/keys): deterministic seed files in hex form.
- [devnet/bootstrap](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/bootstrap): local bootstrap seed JSON files.
- [devnet/hosts](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/hosts): host-style localhost and example multi-host layouts.
- [devnet/run-smoke.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-smoke.sh): wrapper for the smoke flow.
- [devnet/run-restart-smoke.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-restart-smoke.sh): wrapper for the bounded restart smoke.
- [devnet/run-distributed-smoke.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-distributed-smoke.sh): wrapper for the real-process localhost network-bootstrap smoke.
- [devnet/run-multihost-smoke.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-multihost-smoke.sh): wrapper for the host-style network-bootstrap smoke.
- [devnet/run-launch-gate.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-launch-gate.sh): wrapper for the full Milestone 17 launch gate.
- [devnet/run-pilot-checklist.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-pilot-checklist.sh): wrapper for the Milestone 18 pilot rehearsal, fault scenarios, and restart/status evidence.
- [devnet/run-soak.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-soak.sh): wrapper for the logical soak.
- [devnet/pilot](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot): dedicated Milestone 18 pilot topology/config pack.

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
- the runtime can carry a real handshake-backed TCP session;
- a verified presence record can be published and looked up locally;
- a verified service record can be registered and opened locally;
- one direct-first, relay-second fallback path works.

## Network-Bootstrap Smoke

Run:

```bash
./devnet/run-multihost-smoke.sh
```

This starts three static bootstrap seed servers with `overlay-cli
bootstrap-serve`, then runs `overlay-cli smoke --devnet-dir
devnet/hosts/localhost`.

Expected additions beyond the local smoke:

1. startup succeeds from `http://` bootstrap sources rather than local files;
2. the session step uses the configured TCP listeners instead of the earlier
   placeholder-only path;
3. the same publish, lookup, service-open, and relay-fallback steps complete
   against the host-style config layout.

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

## Restart smoke

Run:

```bash
./devnet/run-restart-smoke.sh
```

This performs two consecutive bounded `overlay-cli run` startups against the
same checked-in service-host config. The first run is terminated with `SIGTERM`
and validated through `overlay-cli status`; the second run proves the same
config restarts cleanly with bounded operator state reuse.

## Full launch gate

Run:

```bash
./devnet/run-launch-gate.sh
```

This executes the Milestone 17 pilot gate in documented order:

- `fmt`
- `clippy`
- `check`
- `test`
- stage-boundary smoke tests
- devnet smoke
- distributed network-bootstrap smoke
- multi-host network-bootstrap smoke
- bounded logical soak
- restart smoke

## Pilot checklist

Run:

```bash
./devnet/run-pilot-checklist.sh
```

This uses the dedicated `devnet/pilot/localhost` topology pack and validates:

- the baseline publish, lookup, service-open, and relay-fallback rehearsal;
- the `node-c-down` fault path;
- the `relay-unavailable` fault path;
- the one-bootstrap-seed-unavailable path;
- the pilot-config restart/status outcome;
- the final `pilot_checklist_complete` summary with lookup latency and relay
  usage fields.

## Single-node inspection

Use `overlay-cli run` when you want one node's raw logs and status snapshots
without the in-process orchestration:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config devnet/configs/node-a.json --max-ticks 2 --status-every 1
```

Use `overlay-cli status` when you want the same node's last-known persisted
health and lifecycle state:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- status --config devnet/configs/node-a.json
```

## Devnet limits

- The network bootstrap path is plain `http://` serving static JSON only.
- Presence propagation, exact lookup visibility, and service open are still
  performed inside the smoke harness rather than over a distributed control
  plane.
- The distributed smoke proves bootstrap plus real TCP session establishment
  only; it does not yet carry publish, lookup, service-open, or relay control
  messages over that socket path.
- Relay fallback is still demonstrated for one documented path only:
  `node-a -> node-relay -> node-b`.
- The Milestone 18 pilot pack adds a real-host runbook, fault scenarios, and
  reporting, but the full publish/lookup/service-open/relay proof remains
  smoke-harness coordinated on the current runtime surface.
