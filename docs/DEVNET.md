# Devnet

The repository ships a reproducible local and host-style devnet under
[devnet](../devnet).

It is the operator-facing proof path for the current runtime surface, including
the Milestone 16 host-style network-bootstrap layout under
[devnet/hosts](../devnet/hosts)
and the carried-forward distributed pilot pack under
[devnet/pilot](../devnet/pilot).

## Node roles

- `node-a`: bootstrap anchor and lookup client
- `node-b`: presence publisher and service host
- `node-c`: extra standard peer so bootstrap is not a two-node edge case
- `node-relay`: primary relay-enabled node
- `node-relay-b`: alternate relay-enabled node in the pilot pack
- `node-relay-c`: tertiary relay-enabled node in the pilot pack

## Files

- [devnet/configs](../devnet/configs):
  four runnable local `OverlayConfig` files
- [devnet/keys](../devnet/keys):
  deterministic seed files in hex form
- [devnet/bootstrap](../devnet/bootstrap):
  local bootstrap seed JSON files
- [devnet/hosts](../devnet/hosts):
  host-style localhost and example multi-host layouts
- [devnet/pilot](../devnet/pilot):
  dedicated distributed pilot topology/config pack
- [devnet/run-smoke.sh](../devnet/run-smoke.sh):
  wrapper for the repo-local smoke flow
- [devnet/run-distributed-smoke.sh](../devnet/run-distributed-smoke.sh):
  wrapper for the real-process localhost bootstrap/session smoke
- [devnet/run-multihost-smoke.sh](../devnet/run-multihost-smoke.sh):
  wrapper for the host-style network-bootstrap and operator-flow smoke
- [devnet/run-distributed-pilot-checklist.sh](../devnet/run-distributed-pilot-checklist.sh):
  wrapper for the current distributed pilot checklist
- [devnet/run-first-user-acceptance.sh](../devnet/run-first-user-acceptance.sh):
  wrapper for the current Milestone 27 first-user acceptance flow
- [devnet/run-doctor-smoke.sh](../devnet/run-doctor-smoke.sh):
  wrapper for the landed Milestone 21 doctor/self-check surface
- [devnet/run-pilot-checklist.sh](../devnet/run-pilot-checklist.sh):
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

This starts three static bootstrap seed servers with signed `overlay-cli
bootstrap-serve --signing-key-file ...`, then starts the host-style runtimes from
`devnet/hosts/localhost` and drives networked operator commands plus one
bundled `overlay-cli inspect` report across them.

Expected additions beyond the local smoke:

1. startup succeeds from signed `http://...#ed25519=<pin>` bootstrap sources
   with optional `#sha256=<pin>` integrity checks rather than local files;
2. the session step uses the configured TCP listeners;
3. `publish`, `lookup`, `open-service`, and `relay-intro` all complete over
   real runtime sessions instead of local in-process injection;
4. `overlay-cli inspect` emits one machine-readable report that bundles local
   status/doctor data with explicit lookup, service-open, and relay-intro
   probes.

## Distributed pilot checklist

Run:

```bash
./devnet/run-distributed-pilot-checklist.sh
```

Optional evidence-preserving form:

```bash
./devnet/run-distributed-pilot-checklist.sh --evidence-dir /tmp/overlay-pilot-evidence
```

This is the current distributed component proof path inside
`./devnet/run-first-user-acceptance.sh`.

This uses the dedicated `devnet/pilot/localhost` topology pack and validates:

- the baseline distributed operator flow
- the three-relay candidate baseline proof
- the fresh-node-join proof with late `node-c` startup
- the `node-c-down` fault path
- the primary-relay-down path with alternate relay fallback
- the repeated-relay-bind-failure-recovery path with tertiary relay fallback
- the one-relay-down service-open proof
- the one-bootstrap-seed-unavailable path
- the integrity-mismatch, trust-verification-fallback, stale-bootstrap, and
  empty-peer-set fallback paths
- the service-host restart/status outcome through persisted local service
  intent recovery
- the tampered-bootstrap rejection path
- the final `pilot_checklist_complete` summary with lookup latency and relay
  path fields

## First-user acceptance wrapper

Run:

```bash
./devnet/run-first-user-acceptance.sh
```

Optional evidence-preserving form:

```bash
./devnet/run-first-user-acceptance.sh --evidence-dir /tmp/overlay-first-user-acceptance
```

This is the current top-level localhost sign-off path. It wraps:

- `./devnet/run-launch-gate.sh` for format, lint, build, test, soak, doctor,
  and restart recovery proof
- `./devnet/run-distributed-pilot-checklist.sh` for distributed acceptance
  scenarios on the checked-in pilot topology

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

Use `overlay-cli doctor` when you want a machine-readable self-check against
that node's persisted state:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- doctor --config devnet/configs/node-a.json
```

Use `overlay-cli inspect` when you want that same local operator context plus
explicit remote checks in one report:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- inspect --config devnet/hosts/localhost/configs/node-a.json --lookup tcp://127.0.0.1:4101,1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b --open-service tcp://127.0.0.1:4102,1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b,devnet,terminal --relay-intro tcp://127.0.0.1:4199,16f52d6fea63ef086405aa71b537dd4833bd0b36ffe054be0fd07fb525af157d,83561adb398fd87f8e7ed8331bff2fcb945733cc3012879cb9fab07928667062
```

## Devnet limits

- Bootstrap remains static signed JSON served over `http://`; trust comes from
  pinned signer-key URLs with optional SHA-256 artifact pins rather than HTTPS
  or a public trust root.
- The local `run-smoke.sh` path remains repo-local and still uses the checked-in
  harness for publish/lookup/service/relay orchestration.
- The distributed operator surfaces are explicit CLI calls. `overlay-cli
  inspect` may bundle requested probes, but it is not a persistent control
  plane or discovery system.
- Restart recovery is bounded to persisted bootstrap-source preference,
  last-known active bootstrap peers, and local service registration intent
  only; the devnet does not imply full durable protocol-state persistence.
- Lookup is still exact-by-`node_id` only, and service resolution is still
  exact-by-`app_id` only.
- The checked-in `tcp://127.0.0.1:*` dial hints in `hosts/localhost/` and
  `pilot/localhost/` are localhost stand-ins for the separate-host example
  addresses.
