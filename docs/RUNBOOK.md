# Runbook

This runbook is for the repository's current local and pilot launch surface,
not for hostile-Internet or public-production deployment.

Use [docs/LAUNCH_CHECKLIST.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/LAUNCH_CHECKLIST.md)
as the release gate, this runbook as the operator flow behind that gate, and
[docs/FIRST_USER_ACCEPTANCE.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/FIRST_USER_ACCEPTANCE.md)
for the current first-user-ready boundary, and
[docs/PILOT_RUNBOOK.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/PILOT_RUNBOOK.md)
for the dedicated current-stage off-box distributed exercise.

The current localhost sign-off path is
`./devnet/run-first-user-acceptance.sh`. The older
`./devnet/run-pilot-checklist.sh` remains a retained Milestone 18 rehearsal only.

## Current boundary

What exists today:

- `overlay-cli run` loads one JSON node config, reads one Ed25519 seed file,
  ingests bootstrap seed files from local paths or pinned `http://` URLs,
  ticks the in-memory runtime, prints structured JSON logs, and handles
  `SIGINT` / `SIGTERM` through the runtime shutdown path
- `overlay-cli run --service <namespace:name[:version]>` registers a bounded
  local service record on startup
- `overlay-cli status --config <path>` reads the last-known health and
  lifecycle snapshot from the config-local `.overlay-runtime/` directory
- `overlay-cli status --config <path> --summary` reads the persisted operator
  summary for peers, bootstrap, presence, services, relay usage, and recent
  failures
- `overlay-cli doctor --config <path>` checks config validity, persisted
  runtime state, bootstrap health, and peer-cache recovery using local files
  only
- `overlay-cli publish`, `lookup`, `open-service`, and `relay-intro` provide
  bounded one-shot operator flows over established runtime sessions
- `overlay-cli smoke --devnet-dir <path>` still starts the local four-node
  devnet in-process for the checked-in repo-local proof path
- `overlay-cli bootstrap-serve --bind <addr> --bootstrap-file <path>` serves
  one static bootstrap response over minimal `http://` for devnet or lab use
- `./devnet/run-first-user-acceptance.sh` wraps the landed launch gate and the
  distributed checklist into the current bounded first-user-ready proof

What does not exist today:

- no public bootstrap-provider infrastructure or HTTPS bootstrap fetch
- no general distributed control plane beyond the explicit operator commands
- no broad persistent on-disk runtime state beyond bounded operator metadata,
  last-known health, and the last-known active bootstrap peers used for restart
  recovery
- no rolling upgrade or orchestration framework

## Prerequisites

- Rust and Cargo installed locally
- a writable temp directory; in this repository, use `TMPDIR=/tmp` when needed
- one node config JSON file
- one node key file as either exactly 32 raw Ed25519 seed bytes or exactly 64
  hex characters
- at least one bootstrap seed JSON file

## Startup checklist

1. Generate a starter config with
   `overlay-cli config-template --profile <user-node|relay-capable|bootstrap-seed> --output <path>`
   or pick a stable profile example from
   [docs/CONFIG_EXAMPLES.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/CONFIG_EXAMPLES.md).
2. Verify the config only uses supported top-level fields.
3. Verify `node_key_path` points to an existing seed file.
4. Verify every `bootstrap_sources[]` entry points to a local `.json` file,
   uses `file:<path>`, or uses a static `http://host:port/path#sha256=<hex>`
   seed URL.
5. Start the node with a bounded run first.
6. Confirm the first stdout records include `bootstrap_fetch`,
   `bootstrap_ingest`, and a runtime `state_transition`.
7. Confirm `health.runtime.state` becomes `running` or, if bootstrap failed,
   `degraded`.
8. Confirm `overlay-cli status --config <path>` returns a matching
   `runtime_status` payload with `lifecycle.clean_shutdown == false` while the
   process is still active.
9. If bootstrap is degraded or unexpectedly slow, inspect
   `health.bootstrap.last_attempt_summary`, `health.bootstrap.last_sources`,
   and `overlay-cli status --config <path> --summary` before changing configs.
10. For cross-node behavior, use the distributed operator commands or the
   checked-in smoke/checklist wrappers after single-node startup looks healthy.

## Launch commands

Generate a new template config:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- config-template --profile user-node --output /path/to/node.json
```

Single-node bounded startup:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config docs/config-examples/bootstrap-node.json --max-ticks 2 --status-every 1
```

Continuous ticking with a local service and periodic status:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config docs/config-examples/service-host-node.json --service devnet:terminal --status-every 30
```

Read the persisted operator status:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- status --config docs/config-examples/relay-enabled-node.json
```

Read only the persisted operator summary:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- status --config docs/config-examples/relay-capable.json --summary
```

Run the local doctor surface:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- doctor --config docs/config-examples/user-node.json
```

One-shot distributed operator commands:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- publish --config /path/to/node-b.json --target tcp://127.0.0.1:4111 --relay-ref 16f52d6fea63ef086405aa71b537dd4833bd0b36ffe054be0fd07fb525af157d --capability service-host
```

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- lookup --config /path/to/node-a.json --target tcp://127.0.0.1:4111 --node-id 1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b
```

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- open-service --config /path/to/node-a.json --target tcp://127.0.0.1:4112 --target-node-id 1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b --service-namespace devnet --service-name terminal
```

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- relay-intro --config /path/to/node-b.json --target tcp://127.0.0.1:4198 --relay-node-id 16f52d6fea63ef086405aa71b537dd4833bd0b36ffe054be0fd07fb525af157d --requester-node-id 83561adb398fd87f8e7ed8331bff2fcb945733cc3012879cb9fab07928667062
```

Wrapper scripts:

```bash
./devnet/run-smoke.sh
./devnet/run-distributed-smoke.sh
./devnet/run-multihost-smoke.sh [--evidence-dir <dir>]
./devnet/run-distributed-pilot-checklist.sh [--evidence-dir <dir>]
./devnet/run-restart-smoke.sh
./devnet/run-launch-gate.sh
./devnet/run-soak.sh
```

## Multi-host bootstrap runbook

Use [devnet/hosts/README.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/hosts/README.md)
as the host-style layout reference and
[devnet/pilot/README.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot/README.md)
for the current pilot pack.

Suggested five-host pilot topology:

- `host-a`: `node-a`, seed server for `node-foundation.json`
- `host-b`: `node-b`, seed server for `node-a-seed.json`
- `host-c`: `node-c`
- `host-relay-a`: `node-relay`, seed server for `node-ab-seed.json`
- `host-relay-b-ops`: `node-relay-b`, operator/report collection host

Bring the lab up in this order:

1. Copy the example configs and bootstrap JSON from `devnet/hosts/examples/`
   or `devnet/pilot/examples/`.
2. Start one static seed server on each designated bootstrap host.
3. Start each node with its host-local config; start service hosts with
   `overlay-cli run --service ...`.
4. Confirm each node logs `bootstrap_fetch`, `bootstrap_ingest`, and
   `state_transition`.
5. Confirm `overlay-cli status --config /path/to/node-a.json` reports the same
   node's latest `health` and `.overlay-runtime/` lifecycle state.
6. Use `./devnet/run-distributed-smoke.sh` for the repo-local bootstrap plus
   session-establishment proof.
7. Use `./devnet/run-multihost-smoke.sh` for the repo-local host-style proof
   of bootstrap, publish, lookup, service open, and relay fallback.
8. Use `./devnet/run-distributed-pilot-checklist.sh` for the localhost
   regular-distributed-use checklist. Add `--evidence-dir <dir>` when you want
   the wrapper to preserve raw logs and status files automatically.
9. Use [docs/PILOT_RUNBOOK.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/PILOT_RUNBOOK.md)
   for the actual off-box operator-command run and evidence collection order.

## What healthy output looks like

Structured log records are emitted as one JSON object per line, for example:

- `{"component":"bootstrap","event":"bootstrap_fetch","result":"accepted"}`
- `{"component":"peer","event":"bootstrap_ingest","result":"accepted"}`
- `{"component":"runtime","event":"state_transition","result":"running"}`

`runtime_status` snapshots contain:

- `lifecycle`: config-local state path, pid, startup count, clean/unclean
  shutdown markers, and the most recent shutdown reason
- `health.runtime`: node state plus peer, session, path, presence, and service
  counts
- `health.metrics`: bounded counters and latest samples
- `health.relay`: current relay tunnel and byte-usage snapshot
- `health.bootstrap`: last bootstrap attempt and success counters, plus
  `last_attempt_summary` and `last_sources` for per-source diagnostics
- `health.cleanup_totals`: how many stale objects have been pruned
- `health.resource_limits`: effective local limits after config projection
- `summary`: a concise persisted operator surface for peers, bootstrap,
  presence, services, relay usage, and recent failures

Important fields to watch first:

- `health.runtime.state`
- `health.runtime.active_peers`
- `health.bootstrap.last_accepted_sources`
- `health.bootstrap.last_attempt_summary`
- `health.bootstrap.last_sources`
- `health.metrics.lookup_total`
- `health.metrics.relay_bind_total`
- `health.relay.active_tunnels`

## Restart procedure

The current runtime keeps a bounded peer-cache recovery path. A restart means:

- the runtime first tries live bootstrap sources
- if all configured bootstrap sources are temporarily unavailable, the runtime
  may recover from the last-known active bootstrap peers persisted in
  `runtime_status`
- sessions are reopened from scratch
- published presence and service-open session state are lost unless the caller
  recreates them
- relay tunnels and path probes are rebuilt from scratch

What does persist across restarts:

- `.overlay-runtime/<config-stem>/runtime.lock` while the process is active
- `.overlay-runtime/<config-stem>/runtime-status.json` with the last known
  `runtime_status` payload, including the bounded peer-cache recovery state
- startup counters plus clean/unclean shutdown markers for operator recovery

For a bounded restart check:

```bash
./devnet/run-restart-smoke.sh
```
