# Pilot Runbook

This runbook defines the current Milestone 28
production-gates-packaging-safety-hardening distributed exercise.

It extends the landed pilot pack with minimal distributed operator surfaces,
three bounded relay-capable fallback paths, conservative bootstrap artifact
trust pins, and the separate-host evidence expected before a bounded
production release note is honest.

It is still a bounded operator-managed deployment document. It does not claim
hostile-environment or broad public-Internet readiness.

## Scope

Use this runbook to prove:

- 6 overlay nodes on 4-6 separate hosts;
- 3 static bootstrap seeds served over `http://` as signed artifacts with
  pinned `ed25519=<hex>` trust roots and optional `sha256=<hex>` integrity
  pins;
- real TCP session establishment across hosts;
- networked `publish`, `lookup`, `open-service`, and `relay-intro` flows over
  established runtime sessions;
- a fresh-node-join proof where `node-c` joins after the rest of the topology
  is already running;
- three documented relay fallback paths:
  `node-a -> node-relay -> node-b`,
  `node-a -> node-relay-b -> node-b`, and
  `node-a -> node-relay-c -> node-b`;
- the node-down, primary-relay-down, repeated-relay-bind-failure-recovery,
  relay-unavailable-service-open,
  bootstrap-seed-down, integrity-mismatch, trust-verification-fallback,
  stale-bootstrap, empty-peer-set, service-restart, and tampered-bootstrap
  scenarios;
- per-source bootstrap diagnostics through
  `runtime_status.health.bootstrap.last_attempt_summary` and `last_sources`;
- persisted status summaries through `overlay-cli status --summary`;
- local self-diagnosis through `overlay-cli doctor --config <path>`;
- bounded machine-readable operator reports through `overlay-cli inspect`;
- an operator-collected off-box report with exact hosts, date, and commit SHA
  that can be attached to the current bounded production release note.

Current pilot limits remain in force:

- bootstrap remains static signed JSON served over `http://`; trust comes from
  pinned `#ed25519=<hex>` URL fragments with optional `#sha256=<hex>` defense
  in depth, not from HTTPS or a public trust root;
- the distributed operator surfaces are explicit and operator-directed.
  `overlay-cli inspect` may bundle multiple requested probes, but the repo
  still has no general distributed control plane or discovery system;
- lookup remains exact-by-`node_id` only and service resolution remains
  exact-by-`app_id` only;
- restart recovery is bounded to persisted bootstrap-source preference,
  last-known active bootstrap peers, and local service registration intent;
  presence, service-open sessions, relay tunnels, and path probes still
  remain in-memory runtime state;
- this runbook still does not claim hostile-environment or broad public-Internet
  deployment readiness.

## Topology

Use the files under [devnet/pilot](../devnet/pilot).

Suggested 6-host pilot:

- `host-a`: `node-a`, seed server `0.0.0.0:4301`
- `host-b`: `node-b`, seed server `0.0.0.0:4302`
- `host-c`: `node-c`
- `host-relay-a`: `node-relay`, seed server `0.0.0.0:4303`
- `host-relay-b-ops`: `node-relay-b`, operator/report collection host
- `host-relay-c`: `node-relay-c`

Separate-host example addresses:

- `host-a`: `198.51.100.10`, `node-a` listener `198.51.100.10:4111`
- `host-b`: `198.51.100.11`, `node-b` listener `198.51.100.11:4112`
- `host-c`: `198.51.100.12`, `node-c` listener `198.51.100.12:4113`
- `host-relay-a`: `198.51.100.13`, `node-relay` listener `198.51.100.13:4198`
- `host-relay-b-ops`: `198.51.100.15`, `node-relay-b` listener `198.51.100.15:4197`
- `host-relay-c`: `198.51.100.16`, `node-relay-c` listener `198.51.100.16:4196`

Example node IDs from the checked-in deterministic keys:

- `node-a`: `83561adb398fd87f8e7ed8331bff2fcb945733cc3012879cb9fab07928667062`
- `node-b`: `1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b`
- `node-relay`: `16f52d6fea63ef086405aa71b537dd4833bd0b36ffe054be0fd07fb525af157d`
- `node-relay-b`: `90bdeef49d5d2664e6ef317c3fc4dec4975f13287af7ce3ff4dd9fdf19bb2d7e`
- `node-relay-c`: `529eb3098bf4c47a11adee0f63dbbaa72d91359d2d89b4fed36d0da93d199d35`

Files:

- localhost rehearsal configs:
  [devnet/pilot/localhost/configs](../devnet/pilot/localhost/configs)
- localhost rehearsal bootstrap seeds:
  [devnet/pilot/localhost/bootstrap](../devnet/pilot/localhost/bootstrap)
- separate-host example configs:
  [devnet/pilot/examples/configs](../devnet/pilot/examples/configs)
- separate-host example bootstrap seeds:
  [devnet/pilot/examples/bootstrap](../devnet/pilot/examples/bootstrap)
- localhost proof wrapper:
  [devnet/run-distributed-pilot-checklist.sh](../devnet/run-distributed-pilot-checklist.sh)
- report template:
  [docs/PILOT_REPORT_TEMPLATE.md](PILOT_REPORT_TEMPLATE.md)

## Prerequisites

- Run the current production gate first:

  ```bash
  ./devnet/run-production-gate.sh
  ```

- Keep the same validated commit on all pilot hosts.
- Copy one config and one key file to each node host.
- Copy the three bootstrap JSON files to the designated seed hosts.
- Copy the bootstrap signer key file to each designated seed host.
- If you edit any bootstrap artifact, keep the signer key fixed and recompute
  the signed-artifact `#sha256=<hex>` pin in every config that references it:

  ```bash
  TMPDIR=/tmp cargo run -p overlay-cli -- bootstrap-sign --bootstrap-file devnet/pilot/examples/bootstrap/node-foundation.json --signing-key-file devnet/keys/bootstrap-signer.key --output /tmp/node-foundation.signed.json
  ```

- Keep `TMPDIR=/tmp` available if your environment needs an explicit temp
  directory.

## Separate-host startup order

1. Start the three seed servers:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- bootstrap-serve --bind 0.0.0.0:4301 --bootstrap-file devnet/pilot/examples/bootstrap/node-foundation.json --signing-key-file devnet/keys/bootstrap-signer.key
   ```

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- bootstrap-serve --bind 0.0.0.0:4302 --bootstrap-file devnet/pilot/examples/bootstrap/node-a-seed.json --signing-key-file devnet/keys/bootstrap-signer.key
   ```

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- bootstrap-serve --bind 0.0.0.0:4303 --bootstrap-file devnet/pilot/examples/bootstrap/node-ab-seed.json --signing-key-file devnet/keys/bootstrap-signer.key
   ```

2. Start the five always-on runtimes on their matching hosts:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config /path/to/node-a.json --status-every 30
   ```

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config /path/to/node-b.json --service devnet:terminal --status-every 30
   ```

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config /path/to/node-relay.json --status-every 30
   ```

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config /path/to/node-relay-b.json --status-every 30
   ```

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config /path/to/node-relay-c.json --status-every 30
   ```

3. Confirm each always-on node emits:

   - `bootstrap_fetch`
   - `bootstrap_ingest`
   - `state_transition`

4. Confirm each always-on node reports status:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- status --config /path/to/node-a.json
   ```

5. Run the local doctor surface against at least one live node:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- doctor --config /path/to/node-a.json
   ```

6. Start `node-c` only when you are ready to record the fresh-node-join proof:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config /path/to/node-c.json --status-every 30
   ```

7. Use [devnet/run-multihost-smoke.sh](../devnet/run-multihost-smoke.sh)
   as the repo-local proof for bootstrap plus real networked operator flows on
   the host-style config pack.

8. Use [devnet/run-distributed-pilot-checklist.sh](../devnet/run-distributed-pilot-checklist.sh)
   as the localhost proof for the current distributed checklist. Use
   `--evidence-dir <dir>` when you want the wrapper to preserve the raw logs
   and status files automatically.

## Distributed operator flow

Run the baseline off-box proof in this order.

1. Publish `node-b` presence to `node-a`:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- publish --config /path/to/node-b.json --target tcp://198.51.100.10:4111 --relay-ref 16f52d6fea63ef086405aa71b537dd4833bd0b36ffe054be0fd07fb525af157d --relay-ref 90bdeef49d5d2664e6ef317c3fc4dec4975f13287af7ce3ff4dd9fdf19bb2d7e --relay-ref 529eb3098bf4c47a11adee0f63dbbaa72d91359d2d89b4fed36d0da93d199d35 --capability service-host
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

6. Prove the tertiary relay path:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- relay-intro --config /path/to/node-b.json --target tcp://198.51.100.16:4196 --relay-node-id 529eb3098bf4c47a11adee0f63dbbaa72d91359d2d89b4fed36d0da93d199d35 --requester-node-id 83561adb398fd87f8e7ed8331bff2fcb945733cc3012879cb9fab07928667062
   ```

7. Capture one bounded operator inspection report from `node-a`:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- inspect --config /path/to/node-a.json --lookup tcp://198.51.100.10:4111,1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b --open-service tcp://198.51.100.11:4112,1eed29b1654fbca94617004d7969dfc4652b1f30a7a8b771c34800155483380b,devnet,terminal --relay-intro tcp://198.51.100.13:4198,16f52d6fea63ef086405aa71b537dd4833bd0b36ffe054be0fd07fb525af157d,83561adb398fd87f8e7ed8331bff2fcb945733cc3012879cb9fab07928667062
   ```

These commands are intentionally bounded and explicit:

- each command loads one local node config and its signing key;
- each one-shot command opens one temporary session to one target runtime;
- `overlay-cli inspect` reuses one local persisted status/doctor snapshot and
  opens one temporary session per requested probe target;
- these commands prove real networked runtime flows without adding a new
  always-on control socket or orchestration layer.

## Pilot checklist command

Run from the repository root for the localhost closure proof:

```bash
./devnet/run-distributed-pilot-checklist.sh
```

This script:

- starts the pilot bootstrap servers on `127.0.0.1:4301-4303`
- starts `node-a`, `node-b`, `node-relay`, `node-relay-b`, and
  `node-relay-c`, then starts `node-c` later for the fresh-node-join proof
- drives networked `publish`, `lookup`, `open-service`, and all three
  relay-intro paths over real runtime sessions
- exercises the fresh-node-join, node-down, primary-relay-down,
  repeated-relay-bind-failure-recovery, relay-unavailable-service-open, bootstrap-seed-down,
  integrity-mismatch, trust-verification-fallback, stale-bootstrap,
  empty-peer-set, service-host-restart, and tampered-bootstrap-artifact
  scenarios
- emits JSON lines for each scenario plus a final `pilot_checklist_complete`
  summary

Operational notes:

- during `relay-unavailable`, the first relay-intro attempt against
  `node-relay` is expected to fail once before the alternate `node-relay-b`
  path succeeds; treat the final scenario result and
  `pilot_checklist_complete` summary as the pass signal.
- during `repeated-relay-bind-failure-recovery`, the relay-intro attempts
  against `node-relay` and `node-relay-b` are expected to fail before the
  tertiary `node-relay-c` path succeeds.

## Fault scenarios

Record all thirteen scenarios in the pilot report:

1. `fresh-node-join`

   - `node-c` starts after the rest of the topology is already healthy
   - expected outcome: `node-c` publishes presence to `node-a`, and `node-a`
     can look up `node-c`

2. `node-c-down`

   - `node-c` is unavailable
   - expected outcome: `publish`, `lookup`, `open-service`, and all three
     relay paths still succeed

3. `relay-unavailable`

   - `node-relay` is unavailable
   - expected outcome: the primary relay path fails, and the alternate path via
     `node-relay-b` still succeeds

4. `relay-unavailable-service-open`

   - `node-relay` is unavailable
   - expected outcome: the primary relay-intro attempt degrades once, the
     alternate relay path still binds, and `open-service` still succeeds

5. `three-relay-candidate-set`

   - the checked-in pilot pack exposes `node-relay`, `node-relay-b`, and
     `node-relay-c` as bounded relay candidates
   - expected outcome: all three documented relay paths bind in the baseline
     proof

6. `repeated-relay-bind-failure-recovery`

   - `node-relay` and `node-relay-b` are unavailable
   - expected outcome: two relay-intro attempts fail explicitly, and the
     tertiary path via `node-relay-c` still succeeds

7. `bootstrap-seed-unavailable`

   - the `node-a-seed.json` server is intentionally unavailable
   - expected outcome: bootstrap still succeeds with the remaining pinned seed
     URLs

8. `service-host-restart`

   - `node-b` is restarted after the baseline publish/lookup flow
   - expected outcome: the service host comes back cleanly, `startup_count`
     increases, its persisted local service intent is restored without
     re-passing `--service`, `open-service` succeeds again after restart, and
     the alternate relay path binds again

9. `integrity-mismatch-fallback`

   - one configured bootstrap source uses a deliberately bad SHA-256 pin
   - expected outcome: startup still reaches `running` through the later
     configured source, and `health.bootstrap.last_attempt_summary` reports one
     `integrity_mismatch_sources`

10. `trust-verification-fallback`

   - one configured bootstrap source uses a deliberately bad signer pin
   - expected outcome: startup still reaches `running` through the later
     configured source, and `health.bootstrap.last_attempt_summary` reports one
     `trust_verification_failed_sources`

11. `stale-bootstrap-fallback`

   - one configured bootstrap source is present but expired
   - expected outcome: startup still reaches `running` through the later
     configured source, and `health.bootstrap.last_attempt_summary` reports one
     `stale_sources`

12. `empty-bootstrap-fallback`

   - one configured bootstrap source validates but contains an empty peer set
   - expected outcome: startup still reaches `running` through the later
     configured source, and `health.bootstrap.last_attempt_summary` reports one
     `empty_peer_set_sources`

13. `tampered-bootstrap-artifact`

   - a config is pointed at a deliberately pin-mismatched bootstrap artifact
   - expected outcome: `bootstrap_fetch` reports `integrity_mismatch` and
     startup degrades instead of accepting the artifact

## Evidence to collect

- exact date and UTC/local time range
- exact commit SHA on every host
- exact hostnames and IPs used in the run
- `./devnet/run-production-gate.sh` result on the validated commit
- `./devnet/run-launch-gate.sh` result on the validated commit
- `./devnet/run-multihost-smoke.sh` result on the validated commit
- `./devnet/run-distributed-pilot-checklist.sh --evidence-dir <dir>` result on
  the validated commit
- the raw off-box `publish`, `lookup`, `open-service`, and all three `relay-intro`
  command outputs
- per-host `overlay-cli status --config ...` JSON snapshots
- per-host `overlay-cli status --config ... --summary` excerpts
- at least one `overlay-cli doctor --config ...` output from a live node
- at least one `overlay-cli inspect ...` output from a live node
- bootstrap status excerpts showing `last_attempt_summary` and `last_sources`
  for any degraded or fallback startup
- lookup latency values from the `lookup` command
- relay path evidence from all three `relay-intro` commands and relay status output
- service restart evidence from the persisted status JSON
- tampered-bootstrap integrity-mismatch logs

## Remaining limitations after Milestone 28

- The localhost production gate is the current Milestone 28 green path, but it
  still does not replace the required off-box evidence on separate hosts for a
  release note.
- Bootstrap remains static signed artifact delivery over `http://`; operators
  must keep the signer pin and any `#sha256=<hex>` URLs synchronized manually.
- The distributed operator surfaces remain explicit proof flows.
  `overlay-cli inspect` improves repeatable checks, but it is not a general
  control plane or distributed discovery layer.
- Presence, service-open sessions, relay tunnels, and path probes still reset
  on restart except for the bounded bootstrap-source, active-peer, and local
  service-intent recovery state.
- Relay proof remains bounded to the checked-in three-relay topology rather than
  arbitrary relay graphs or public-network conditions.

Use [docs/PILOT_REPORT_TEMPLATE.md](PILOT_REPORT_TEMPLATE.md)
to write the final off-box evidence report, then attach it to the bounded
production release note from [docs/PRODUCTION_RELEASE_TEMPLATE.md](PRODUCTION_RELEASE_TEMPLATE.md).
