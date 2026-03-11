# Devnet

This directory contains the checked-in devnet and pilot assets for the current
Milestone 19 stage:

- the original four-node local-file devnet under `configs/` and `bootstrap/`
- the host-style multi-host devnet layouts under `hosts/`
- the dedicated Milestone 19 pilot pack under `pilot/`
- wrapper scripts for the local, distributed, network-bootstrap, and
  pilot-closure proof paths

Nodes:

- `node-a`: bootstrap anchor and lookup client
- `node-b`: presence publisher and service host
- `node-c`: extra peer so the seed set is not a 2-node degenerate case
- `node-relay`: primary relay node
- `node-relay-b`: alternate relay node in the pilot pack

## Files

- `configs/*.json`: example `OverlayConfig` files
- `keys/*.key`: deterministic Ed25519 seed files in hex form
- `bootstrap/*.json`: local bootstrap seed files used by runtime startup
- `hosts/`: host-style config layouts for localhost proof and multi-host copy/edit use
- `pilot/`: dedicated Milestone 19 pilot configs and pinned bootstrap artifacts
- `run-smoke.sh`: wrapper around `overlay-cli smoke`
- `run-distributed-smoke.sh`: wrapper around the minimal multi-process localhost TCP smoke
- `run-multihost-smoke.sh`: wrapper around the host-style network-bootstrap smoke
- `run-distributed-pilot-checklist.sh`: wrapper around the Milestone 19 pilot-closure checklist
- `run-pilot-checklist.sh`: retained Milestone 18 localhost rehearsal pack
- `run-launch-gate.sh`: wrapper around the Milestone 17 launch gate
- `run-restart-smoke.sh`: wrapper around the bounded restart smoke
- `run-soak.sh`: wrapper around the logical long-run runtime soak

## Run the local smoke flow

```bash
./devnet/run-smoke.sh
```

The repo-local smoke still prints one JSON object per step and exercises:

1. `startup`
2. `session_established`
3. `publish_presence`
4. `lookup_node`
5. `open_service`
6. `relay_fallback_planned`
7. `relay_fallback_bound`
8. `smoke_complete`

## Inspect a single node

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config devnet/configs/node-a.json --max-ticks 2 --status-every 1
```

Register a local service on startup:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config devnet/configs/node-b.json --service devnet:terminal --status-every 30
```

Read the persisted status surface:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- status --config devnet/configs/node-a.json
```

## Distributed TCP smoke

```bash
./devnet/run-distributed-smoke.sh
```

This starts static bootstrap seed servers, starts `node-a` and `node-b` as
separate processes, and checks accepted bootstrap fetches, listener bind, dial,
accept, and session-establishment logs.

## Multi-host smoke

```bash
./devnet/run-multihost-smoke.sh
```

This starts the static seed servers, then starts the host-style runtimes and
drives bounded `publish`, `lookup`, `open-service`, and `relay-intro` commands
across them.

## Pilot checklist

```bash
./devnet/run-distributed-pilot-checklist.sh
```

This starts the dedicated pilot bootstrap servers, runs the current distributed
operator flow against `pilot/localhost/`, exercises the documented fault
scenarios, checks service-host restart/status behavior, validates tampered
bootstrap rejection, and emits a final `pilot_checklist_complete` summary.

## Current limits

- bootstrap remains static JSON served over `http://`; integrity comes from
  pinned SHA-256 artifact URLs rather than HTTPS or a public trust root
- the local `run-smoke.sh` path remains repo-local and harness-driven for the
  publish/lookup/service/relay steps
- the distributed operator flows are explicit point-to-point CLI surfaces, not
  a general distributed control plane
- lookup is still exact-by-`node_id` only, and service resolution is still
  exact-by-`app_id` only
