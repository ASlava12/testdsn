# Pilot Runbook

This runbook defines the Milestone 19 pilot-closure exercise.

It extends the landed Milestone 18 pilot pack with minimal distributed
operator surfaces, two relay-capable fallback paths, conservative bootstrap
artifact integrity pins, and the evidence expected from the first real off-box
pilot closure run.

It is still a pilot-only document. It does not claim public-production or
hostile-environment readiness.

## Scope

Use this runbook to prove:

- 5 overlay nodes on 3-5 separate hosts;
- 3 static bootstrap seeds served over `http://` with SHA-256-pinned artifact
  URLs;
- real TCP session establishment across hosts;
- networked `publish`, `lookup`, `open-service`, and `relay-intro` flows over
  established runtime sessions;
- two documented relay fallback paths:
  `node-a -> node-relay -> node-b` and
  `node-a -> node-relay-b -> node-b`;
- the node-down, primary-relay-down, bootstrap-seed-down, service-restart, and
  tampered-bootstrap fault scenarios;
- an operator-collected off-box report with exact hosts, date, and commit SHA.

Current pilot limits remain in force:

- bootstrap remains static JSON served over `http://`; integrity comes from
  pinned `#sha256=<hex>` URL fragments, not from HTTPS or a public trust root;
- the distributed operator commands are one-shot, point-to-point, and
  operator-directed, not a general distributed control plane or discovery
  system;
- lookup remains exact-by-`node_id` only and service resolution remains
  exact-by-`app_id` only;
- peers, presence, services, sessions, relay tunnels, and path probes remain
  in-memory runtime state;
- this runbook still does not claim hostile-environment or public-Internet
  deployment readiness.

## Topology

Use the files under [devnet/pilot](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot).

Suggested 5-host pilot:

- `host-a`: `node-a`, seed server `0.0.0.0:4301`
- `host-b`: `node-b`, seed server `0.0.0.0:4302`
- `host-c`: `node-c`
- `host-relay-a`: `node-relay`, seed server `0.0.0.0:4303`
- `host-relay-b-ops`: `node-relay-b`, operator/report collection host

Separate-host example addresses:

- `host-a`: `198.51.100.10`, `node-a` listener `198.51.100.10:4111`
- `host-b`: `198.51.100.11`, `node-b` listener `198.51.100.11:4112`
- `host-c`: `198.51.100.12`, `node-c` listener `198.51.100.12:4113`
- `host-relay-a`: `198.51.100.13`, `node-relay` listener `198.51.100.13:4198`
- `host-relay-b-ops`: `198.51.100.15`, `node-relay-b` listener `198.51.100.15:4197`

Example node IDs from the checked-in deterministic keys:

- `node-a`: `83561adb398fd87f8e7ed8331bff2fcb945733cc3012879cb9fab07928667062`
- `node-b`: `1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b`
- `node-relay`: `16f52d6fea63ef086405aa71b537dd4833bd0b36ffe054be0fd07fb525af157d`
- `node-relay-b`: `90bdeef49d5d2664e6ef317c3fc4dec4975f13287af7ce3ff4dd9fdf19bb2d7e`

Files:

- localhost rehearsal configs:
  [devnet/pilot/localhost/configs](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot/localhost/configs)
- localhost rehearsal bootstrap seeds:
  [devnet/pilot/localhost/bootstrap](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot/localhost/bootstrap)
- separate-host example configs:
  [devnet/pilot/examples/configs](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot/examples/configs)
- separate-host example bootstrap seeds:
  [devnet/pilot/examples/bootstrap](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot/examples/bootstrap)
- localhost proof wrapper:
  [devnet/run-distributed-pilot-checklist.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-distributed-pilot-checklist.sh)
- report template:
  [docs/PILOT_REPORT_TEMPLATE.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/PILOT_REPORT_TEMPLATE.md)

## Prerequisites

- Run the current launch gate first:

  ```bash
  ./devnet/run-launch-gate.sh
  ```

- Keep the same validated commit on all pilot hosts.
- Copy one config and one key file to each node host.
- Copy the three bootstrap JSON files to the designated seed hosts.
- If you edit any bootstrap artifact, recompute and update the `#sha256=<hex>`
  pin in every config that references it:

  ```bash
  sha256sum devnet/pilot/examples/bootstrap/node-foundation.json
  ```

- Keep `TMPDIR=/tmp` available if your environment needs an explicit temp
  directory.

## Separate-host startup order

1. Start the three seed servers:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- bootstrap-serve --bind 0.0.0.0:4301 --bootstrap-file devnet/pilot/examples/bootstrap/node-foundation.json
   ```

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- bootstrap-serve --bind 0.0.0.0:4302 --bootstrap-file devnet/pilot/examples/bootstrap/node-a-seed.json
   ```

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- bootstrap-serve --bind 0.0.0.0:4303 --bootstrap-file devnet/pilot/examples/bootstrap/node-ab-seed.json
   ```

2. Start the five runtimes on their matching hosts:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config /path/to/node-a.json --status-every 30
   ```

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config /path/to/node-b.json --service devnet:terminal --status-every 30
   ```

3. Confirm each node emits:

   - `bootstrap_fetch`
   - `bootstrap_ingest`
   - `state_transition`

4. Confirm each node reports status:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- status --config /path/to/node-a.json
   ```

5. Use [devnet/run-multihost-smoke.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-multihost-smoke.sh)
   as the repo-local proof for bootstrap plus real networked operator flows on
   the host-style config pack.

6. Use [devnet/run-distributed-pilot-checklist.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-distributed-pilot-checklist.sh)
   as the localhost proof for the current Milestone 19 pilot-closure checklist.

## Distributed operator flow

Run the baseline off-box proof in this order.

1. Publish `node-b` presence to `node-a`:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- publish --config /path/to/node-b.json --target tcp://198.51.100.10:4111 --relay-ref 16f52d6fea63ef086405aa71b537dd4833bd0b36ffe054be0fd07fb525af157d --relay-ref 90bdeef49d5d2664e6ef317c3fc4dec4975f13287af7ce3ff4dd9fdf19bb2d7e --capability service-host
   ```

2. Lookup `node-b` from `node-a` against the runtime that stored the presence:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- lookup --config /path/to/node-a.json --target tcp://198.51.100.10:4111 --node-id 1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b
   ```

3. Resolve and open the `devnet/terminal` service on `node-b`:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- open-service --config /path/to/node-a.json --target tcp://198.51.100.11:4112 --target-node-id 1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b --service-namespace devnet --service-name terminal
   ```

4. Prove the primary relay path:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- relay-intro --config /path/to/node-b.json --target tcp://198.51.100.13:4198 --relay-node-id 16f52d6fea63ef086405aa71b537dd4833bd0b36ffe054be0fd07fb525af157d --requester-node-id 83561adb398fd87f8e7ed8331bff2fcb945733cc3012879cb9fab07928667062
   ```

5. Prove the alternate relay path:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- relay-intro --config /path/to/node-b.json --target tcp://198.51.100.15:4197 --relay-node-id 90bdeef49d5d2664e6ef317c3fc4dec4975f13287af7ce3ff4dd9fdf19bb2d7e --requester-node-id 83561adb398fd87f8e7ed8331bff2fcb945733cc3012879cb9fab07928667062
   ```

These commands are intentionally bounded and explicit:

- each command loads one local node config and its signing key;
- each command opens one temporary session to one target runtime;
- each command proves a real networked runtime flow without adding a new
  always-on control socket or orchestration layer.

## Pilot checklist command

Run from the repository root for the localhost closure proof:

```bash
./devnet/run-distributed-pilot-checklist.sh
```

This script:

- starts the pilot bootstrap servers on `127.0.0.1:4301-4303`
- starts `node-a`, `node-b`, `node-c`, `node-relay`, and `node-relay-b`
- drives networked `publish`, `lookup`, `open-service`, and both relay-intro
  paths over real runtime sessions
- exercises the node-down, primary-relay-down, bootstrap-seed-down,
  service-host-restart, and tampered-bootstrap-artifact scenarios
- emits JSON lines for each scenario plus a final `pilot_checklist_complete`
  summary

Operational note:

- during `relay-unavailable`, the first relay-intro attempt against
  `node-relay` is expected to fail once before the alternate `node-relay-b`
  path succeeds; treat the final scenario result and
  `pilot_checklist_complete` summary as the pass signal.

## Fault scenarios

Record all five scenarios in the pilot report:

1. `node-c-down`

   - `node-c` is unavailable
   - expected outcome: `publish`, `lookup`, `open-service`, and both relay
     paths still succeed

2. `relay-unavailable`

   - `node-relay` is unavailable
   - expected outcome: the primary relay path fails, and the alternate path via
     `node-relay-b` still succeeds

3. `bootstrap-seed-unavailable`

   - the `node-a-seed.json` server is intentionally unavailable
   - expected outcome: bootstrap still succeeds with the remaining pinned seed
     URLs

4. `service-host-restart`

   - `node-b` is restarted after the baseline publish/lookup flow
   - expected outcome: the service host comes back cleanly, `startup_count`
     increases, and `open-service` succeeds again after restart

5. `tampered-bootstrap-artifact`

   - a config is pointed at a deliberately pin-mismatched bootstrap artifact
   - expected outcome: `bootstrap_fetch` reports `rejected` and startup
     degrades instead of accepting the artifact

## Evidence to collect

- exact date and UTC/local time range
- exact commit SHA on every host
- exact hostnames and IPs used in the run
- `./devnet/run-launch-gate.sh` result on the validated commit
- `./devnet/run-multihost-smoke.sh` result on the validated commit
- `./devnet/run-distributed-pilot-checklist.sh` result on the validated commit
- the raw off-box `publish`, `lookup`, `open-service`, and both `relay-intro`
  command outputs
- per-host `overlay-cli status --config ...` JSON snapshots
- lookup latency values from the `lookup` command
- relay path evidence from both `relay-intro` commands and relay status output
- service restart evidence from the persisted status JSON
- tampered-bootstrap rejection logs

## Remaining closure items for regular distributed use

- The localhost checklist is the current Milestone 19 green path, but it still
  does not replace the required off-box pilot evidence on separate hosts.
- Bootstrap remains static pinned `http://` artifact delivery; operators must
  keep the artifacts and `#sha256=<hex>` URLs synchronized manually.
- The distributed operator commands remain one-shot proof surfaces, not a
  general control plane or distributed discovery layer.
- Runtime peers, presence, services, sessions, relay tunnels, and path probes
  still reset on restart.
- Only the two documented relay fallback paths are proven for this pilot
  closure stage.

Use [docs/PILOT_REPORT_TEMPLATE.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/PILOT_REPORT_TEMPLATE.md)
to write the final report.
