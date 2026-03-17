# Devnet

This directory contains the checked-in devnet and pilot assets for the current
Milestone 25 stage:

- the original four-node local-file devnet under `configs/` and `bootstrap/`
- the host-style multi-host devnet layouts under `hosts/`
- the dedicated distributed pilot pack under `pilot/`
- wrapper scripts for the local, distributed, network-bootstrap, and
  first-user-runtime proof paths

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
- `pilot/`: dedicated distributed pilot configs and pinned bootstrap artifacts
- `run-smoke.sh`: wrapper around `overlay-cli smoke`
- `run-distributed-smoke.sh`: wrapper around the minimal multi-process localhost TCP smoke
- `run-multihost-smoke.sh`: wrapper around the host-style network-bootstrap smoke
- `run-distributed-pilot-checklist.sh`: wrapper around the current distributed pilot checklist
- `run-first-user-acceptance.sh`: wrapper around the current Milestone 25
  first-user acceptance flow
- `run-pilot-checklist.sh`: retained Milestone 18 localhost rehearsal pack,
  not the current sign-off path
- `run-launch-gate.sh`: wrapper around the Milestone 17 launch gate
- `run-doctor-smoke.sh`: wrapper around the landed doctor/self-check
  smoke
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

Run the local self-check surface:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- doctor --config devnet/configs/node-a.json
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

Optional evidence-preserving form:

```bash
./devnet/run-multihost-smoke.sh --evidence-dir /tmp/overlay-multihost-evidence
```

This starts the static seed servers, then starts the host-style runtimes and
drives bounded `publish`, `lookup`, `open-service`, and `relay-intro` commands
across them.

## Pilot checklist

```bash
./devnet/run-distributed-pilot-checklist.sh
```

Optional evidence-preserving form:

```bash
./devnet/run-distributed-pilot-checklist.sh --evidence-dir /tmp/overlay-pilot-evidence
```

This starts the dedicated pilot bootstrap servers, runs the current distributed
operator flow against `pilot/localhost/`, exercises the documented fault
scenarios including unavailable/integrity/stale/empty bootstrap cases, checks
service-host restart/status behavior plus repeated relay binds, validates
tampered bootstrap rejection, and emits a final `pilot_checklist_complete`
summary.

This is the current localhost sign-off path after `./devnet/run-launch-gate.sh`.

## Current limits

- bootstrap remains static JSON served over `http://`; integrity comes from
  pinned SHA-256 artifact URLs rather than HTTPS or a public trust root
- the local `run-smoke.sh` path remains repo-local and harness-driven for the
  publish/lookup/service/relay steps
- the distributed operator flows are explicit point-to-point CLI surfaces, not
  a general distributed control plane
- restart recovery is bounded to persisted bootstrap-source preference,
  last-known active bootstrap peers, and local service registration intent
  only
- lookup is still exact-by-`node_id` only, and service resolution is still
  exact-by-`app_id` only
