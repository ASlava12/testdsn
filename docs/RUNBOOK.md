# Runbook

This runbook is for the repository's current local or pilot launch surface, not
for hostile-Internet or public-production deployment.

## Current boundary

What exists today:

- `overlay-cli run` loads one JSON node config, reads one Ed25519 seed file,
  ingests local bootstrap seed files, ticks the in-memory runtime, and prints
  structured JSON logs to stdout.
- `overlay-cli run --status-every <ticks>` also prints periodic
  `runtime_status` JSON snapshots with runtime state, metrics, relay usage,
  cleanup totals, bootstrap status, and resource limits.
- `overlay-cli smoke --devnet-dir <path>` starts the local four-node devnet
  in-process and exercises the bootstrap, session, presence, lookup, service,
  and relay-fallback path that the repository currently validates.

What does not exist today:

- no public bootstrap fetch over the network;
- no real transport listeners or distributed multi-process data plane;
- no daemon management, PID files, or signal-driven graceful shutdown path;
- no persistent on-disk runtime state for peers, presence, services, or relay
  tunnels.

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
4. Verify every `bootstrap_sources[]` entry points to a local `.json` file or
   uses `file:<path>`.
5. Start the node with a bounded run first.
6. Confirm the first stdout records include `bootstrap_fetch`,
   `bootstrap_ingest`, and a runtime `state_transition`.
7. Confirm `health.runtime.state` becomes `running` or, if bootstrap failed,
   `degraded`.
8. For cross-node behavior, use the smoke harness after single-node startup
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

Repository devnet smoke:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- smoke --devnet-dir devnet
```

Repository devnet logical soak:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- smoke --devnet-dir devnet --soak-seconds 1800 --status-interval-seconds 300
```

Wrapper scripts:

```bash
./devnet/run-smoke.sh
./devnet/run-soak.sh
```

## What healthy output looks like

Structured log records are emitted as one JSON object per line, for example:

- `{"component":"bootstrap","event":"bootstrap_fetch","result":"accepted"}`
- `{"component":"peer","event":"bootstrap_ingest","result":"accepted"}`
- `{"component":"runtime","event":"state_transition","result":"running"}`

`runtime_status` snapshots contain:

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

Use the same key and config files, then rerun `overlay-cli run ...`.

For a bounded restart check:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config docs/config-examples/service-host-node.json --max-ticks 0 --status-every 1
```

## Shutdown notes

`overlay-cli run` calls the runtime shutdown path only when the process reaches
its natural end, such as `--max-ticks <count>`.

The current CLI does not install signal handling. If you interrupt the process
manually, do not assume a final structured shutdown record will be emitted.

## Operator limits to remember

- A "bootstrap node" in this repository means a node identity that other local
  seed files point at. It is not a live bootstrap server process.
- `relay_mode` is the only relay-related JSON switch. Relay quotas are compiled
  profile defaults and are surfaced through `runtime_status`, not configured in
  the JSON file.
- Local service allow or deny policy is code-driven today. It is not exposed in
  the node config schema.
- Lookup is exact-by-`node_id` only.
- Service resolution is exact-by-`app_id` only.
