# Runbook

This runbook is for the repository's current local or pilot launch surface, not
for hostile-Internet or public-production deployment.

Use [docs/LAUNCH_CHECKLIST.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/LAUNCH_CHECKLIST.md)
as the release gate and this runbook as the operator flow behind that gate.

## Current boundary

What exists today:

- `overlay-cli run` loads one JSON node config, reads one Ed25519 seed file,
  ingests bootstrap seed files from local paths or plain `http://` URLs, ticks
  the in-memory runtime, prints structured JSON logs to stdout, and handles
  `SIGINT` / `SIGTERM` through the runtime shutdown path.
- `overlay-cli run --status-every <ticks>` also prints periodic
  `runtime_status` JSON snapshots with runtime state, metrics, relay usage,
  cleanup totals, bootstrap status, resource limits, and operator lifecycle
  state.
- `overlay-cli status --config <path>` reads the last-known health and
  lifecycle snapshot from the config-local `.overlay-runtime/` directory.
- `overlay-cli smoke --devnet-dir <path>` starts the local four-node devnet
  in-process and exercises the bootstrap, session, presence, lookup, service,
  and relay-fallback path that the repository currently validates.
- `overlay-cli bootstrap-serve --bind <addr> --bootstrap-file <path>` serves
  one static bootstrap response over minimal `http://` for devnet or lab use.

What does not exist today:

- no public bootstrap-provider infrastructure or HTTPS bootstrap fetch;
- no full distributed control plane beyond the checked-in bootstrap and session
  smoke paths;
- no persistent on-disk runtime state for peers, presence, services, or relay
  tunnels beyond bounded operator metadata and last-known health;
- no rolling upgrade or orchestration framework.

## Prerequisites

- Rust and Cargo installed locally.
- A writable temp directory. In this repository, use `TMPDIR=/tmp` when needed.
- One node config JSON file.
- One node key file. The runtime accepts either:
  - exactly 32 raw Ed25519 seed bytes; or
  - exactly 64 hex characters.
- At least one bootstrap seed JSON file.

## Startup checklist

1. Pick a role example from [docs/CONFIG_EXAMPLES.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/CONFIG_EXAMPLES.md).
2. Verify the config only uses supported top-level fields.
3. Verify `node_key_path` points to an existing 32-byte or 64-hex seed file.
4. Verify every `bootstrap_sources[]` entry points to a local `.json` file,
   uses `file:<path>`, or uses a devnet/lab `http://host:port/path` seed URL.
5. Start the node with a bounded run first.
6. Confirm the first stdout records include `bootstrap_fetch`,
   `bootstrap_ingest`, and a runtime `state_transition`.
7. Confirm `health.runtime.state` becomes `running` or, if bootstrap failed,
   `degraded`.
8. Confirm `overlay-cli status --config <path>` returns a matching
   `runtime_status` payload with `lifecycle.clean_shutdown == false` while the
   process is still active.
9. For cross-node behavior, use the smoke harness after single-node startup
   looks healthy.

## Launch commands

Single-node bounded startup:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config docs/config-examples/bootstrap-node.json --max-ticks 2 --status-every 1
```

Continuous ticking with periodic status:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config docs/config-examples/relay-enabled-node.json --status-every 30
```

Read the persisted operator status:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- status --config docs/config-examples/relay-enabled-node.json
```

Repository devnet smoke:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- smoke --devnet-dir devnet
```

Repository devnet logical soak:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- smoke --devnet-dir devnet --soak-seconds 1800 --status-interval-seconds 300
```

Host-style network-bootstrap smoke:

```bash
./devnet/run-multihost-smoke.sh
```

Wrapper scripts:

```bash
./devnet/run-smoke.sh
./devnet/run-distributed-smoke.sh
./devnet/run-multihost-smoke.sh
./devnet/run-restart-smoke.sh
./devnet/run-launch-gate.sh
./devnet/run-soak.sh
```

`./devnet/run-launch-gate.sh` is the CI-friendly Milestone 17 pilot gate. It
runs formatting, lint, build, workspace tests, the stage-boundary integration
tests, the local devnet smoke, the distributed network-bootstrap smoke, the
multi-host network-bootstrap smoke, the bounded logical soak, and the restart
smoke in the documented order.

## Multi-host bootstrap runbook

Use [devnet/hosts/README.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/hosts/README.md)
as the file layout reference.

Suggested four-host pilot topology:

- host-a: `node-a`, bootstrap seed server for `node-foundation.json`
- host-b: `node-b`, bootstrap seed server for `node-a-seed.json`
- host-c: `node-c`
- host-relay: `node-relay`, bootstrap seed server for `node-ab-seed.json`

Bring the lab up in this order:

1. Copy the example configs and bootstrap JSON from `devnet/hosts/examples/`.
2. Start one static seed server on each designated bootstrap host, for example:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- bootstrap-serve --bind 0.0.0.0:4201 --bootstrap-file devnet/hosts/examples/bootstrap/node-foundation.json
   ```

3. Start each node with its host-local config:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config /path/to/node-a.json --status-every 30
   ```

4. Confirm each node logs `bootstrap_fetch`, `bootstrap_ingest`, and
   `state_transition`.
5. Confirm `overlay-cli status --config /path/to/node-a.json` reports the same
   node's latest `health` and `.overlay-runtime/` lifecycle state.
6. Use `./devnet/run-distributed-smoke.sh` for the repo-local real-process
   proof of network bootstrap plus session establishment.
7. Use `./devnet/run-multihost-smoke.sh` for the repo-local host-style proof of
   bootstrap, publish, lookup, service open, and relay fallback against the
   same config layout.

## What healthy output looks like

Structured log records are emitted as one JSON object per line, for example:

- `{"component":"bootstrap","event":"bootstrap_fetch","result":"accepted"}`
- `{"component":"peer","event":"bootstrap_ingest","result":"accepted"}`
- `{"component":"runtime","event":"state_transition","result":"running"}`

`runtime_status` snapshots contain:

- `lifecycle`: config-local state path, pid, startup count, clean/unclean
  shutdown markers, and the most recent shutdown reason;
- `health.runtime`: node state plus peer, session, path, presence, and service
  counts;
- `health.metrics`: bounded counters and latest samples;
- `health.relay`: current relay tunnel and byte-usage snapshot;
- `health.bootstrap`: last bootstrap attempt and success counters;
- `health.cleanup_totals`: how many stale objects have been pruned;
- `health.resource_limits`: effective local limits after config projection.

Important fields to watch first:

- `health.runtime.state`
- `health.runtime.active_peers`
- `health.bootstrap.last_accepted_sources`
- `health.metrics.lookup_total`
- `health.metrics.relay_bind_total`
- `health.metrics.path_switch_total`
- `health.relay.active_tunnels`

## Restart procedure

The current runtime is in-memory only. A restart means:

- peer state is rebuilt from bootstrap files;
- sessions are reopened from scratch;
- published presence and service-open session state are lost unless your caller
  recreates them;
- relay tunnels and path probes are rebuilt from scratch.

What does persist across restarts:

- `.overlay-runtime/<config-stem>/runtime.lock` while the process is active;
- `.overlay-runtime/<config-stem>/runtime-status.json` with the last known
  `runtime_status` payload;
- startup counters plus clean/unclean shutdown markers for operator recovery.

Use the same key and config files, then rerun `overlay-cli run ...`.

For a bounded restart check:

```bash
./devnet/run-restart-smoke.sh
```

## Shutdown notes

`overlay-cli run` now routes `SIGINT` and `SIGTERM` through the runtime
shutdown path, emits a `runtime_control` shutdown-signal record, updates the
persisted `runtime_status`, and releases the config-local lock file on clean
exit.

If the process dies without that path completing, the lock file remains. The
next startup treats that as stale operator state, recovers it conservatively,
and reports `lifecycle.recovered_from_unclean_shutdown == true`.

## Operator limits to remember

- A "bootstrap node" in this repository may be represented either by a static
  seed file or by `overlay-cli bootstrap-serve` exposing that file over
  `http://`. It is still not a public bootstrap-provider framework.
- `.overlay-runtime/` is bounded operator metadata only, not a protocol-state
  database.
- `relay_mode` is the only relay-related JSON switch. Relay quotas are compiled
  profile defaults and are surfaced through `runtime_status`, not configured in
  the JSON file.
- Local service allow or deny policy is code-driven today. It is not exposed in
  the node config schema.
- Lookup is exact-by-`node_id` only.
- Service resolution is exact-by-`app_id` only.
