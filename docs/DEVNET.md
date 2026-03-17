# Devnet

The repository ships a reproducible local and host-style devnet under
[devnet](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet).

It is the operator-facing proof path for the current runtime surface, including
the Milestone 16 host-style network-bootstrap layout under
[devnet/hosts](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/hosts)
and the Milestone 20 regular-distributed-use pack under
[devnet/pilot](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot).

## Node roles

- `node-a`: bootstrap anchor and lookup client
- `node-b`: presence publisher and service host
- `node-c`: extra standard peer so bootstrap is not a two-node edge case
- `node-relay`: primary relay-enabled node
- `node-relay-b`: alternate relay-enabled node in the pilot pack

## Files

- [devnet/configs](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/configs):
  four runnable local `OverlayConfig` files
- [devnet/keys](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/keys):
  deterministic seed files in hex form
- [devnet/bootstrap](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/bootstrap):
  local bootstrap seed JSON files
- [devnet/hosts](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/hosts):
  host-style localhost and example multi-host layouts
- [devnet/pilot](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot):
  dedicated Milestone 20 regular-distributed-use topology/config pack
- [devnet/run-smoke.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-smoke.sh):
  wrapper for the repo-local smoke flow
- [devnet/run-distributed-smoke.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-distributed-smoke.sh):
  wrapper for the real-process localhost bootstrap/session smoke
- [devnet/run-multihost-smoke.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-multihost-smoke.sh):
  wrapper for the host-style network-bootstrap and operator-flow smoke
- [devnet/run-distributed-pilot-checklist.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-distributed-pilot-checklist.sh):
  wrapper for the current regular-distributed-use checklist
- [devnet/run-pilot-checklist.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-pilot-checklist.sh):
  retained Milestone 18 localhost rehearsal pack, not the current sign-off
  path

## Smoke flow

Run either command:

```bash
./devnet/run-smoke.sh
```

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- smoke --devnet-dir devnet
```

Expected step sequence:

1. `startup` for all four nodes
2. `session_established` from `node-a` to `node-b`
3. `publish_presence` for `node-b`
4. `lookup_node` from `node-a` to `node-b`
5. `open_service` against `node-b`
6. `relay_fallback_planned`
7. `relay_fallback_bound`
8. `smoke_complete`

What this proves:

- the sample configs load;
- bootstrap files validate and populate local peers;
- the runtime can carry a real handshake-backed TCP session;
- the repo-local harness can still exercise publish, exact lookup, service
  open, and one relay fallback path without extra orchestration.

## Network-bootstrap smoke

Run:

```bash
./devnet/run-multihost-smoke.sh
```

Optional evidence-preserving form:

```bash
./devnet/run-multihost-smoke.sh --evidence-dir /tmp/overlay-multihost-evidence
```

This starts three static bootstrap seed servers with `overlay-cli
bootstrap-serve`, then starts the host-style runtimes from
`devnet/hosts/localhost` and drives networked operator commands across them.

Expected additions beyond the local smoke:

1. startup succeeds from pinned `http://...#sha256=<pin>` bootstrap sources
   rather than local files;
2. the session step uses the configured TCP listeners;
3. `publish`, `lookup`, `open-service`, and `relay-intro` all complete over
   real runtime sessions instead of local in-process injection.

## Distributed pilot checklist

Run:

```bash
./devnet/run-distributed-pilot-checklist.sh
```

Optional evidence-preserving form:

```bash
./devnet/run-distributed-pilot-checklist.sh --evidence-dir /tmp/overlay-pilot-evidence
```

This is the current localhost regular-distributed-use sign-off path after
`./devnet/run-launch-gate.sh`.

This uses the dedicated `devnet/pilot/localhost` topology pack and validates:

- the baseline distributed operator flow
- the `node-c-down` fault path
- the primary-relay-down path with alternate relay fallback
- the one-bootstrap-seed-unavailable path
- the integrity-mismatch, stale-bootstrap, and empty-peer-set fallback paths
- the service-host restart/status outcome
- the tampered-bootstrap rejection path
- the final `pilot_checklist_complete` summary with lookup latency and relay
  path fields

## Single-node inspection

Use `overlay-cli run` when you want one node's raw logs and status snapshots:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config devnet/configs/node-a.json --max-ticks 2 --status-every 1
```

Register a bounded local service host with the same command surface:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config devnet/configs/node-b.json --service devnet:terminal --status-every 30
```

Use `overlay-cli status` when you want the same node's last-known persisted
health and lifecycle state:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- status --config devnet/configs/node-a.json
```

## Devnet limits

- Bootstrap remains static JSON served over `http://`; integrity comes from
  pinned SHA-256 artifact URLs rather than HTTPS or a public trust root.
- The local `run-smoke.sh` path remains repo-local and still uses the checked-in
  harness for publish/lookup/service/relay orchestration.
- The distributed operator commands are explicit point-to-point CLI calls, not
  a persistent control plane or discovery system.
- Lookup is still exact-by-`node_id` only, and service resolution is still
  exact-by-`app_id` only.
- The checked-in `tcp://127.0.0.1:*` dial hints in `hosts/localhost/` and
  `pilot/localhost/` are localhost stand-ins for the separate-host example
  addresses.
