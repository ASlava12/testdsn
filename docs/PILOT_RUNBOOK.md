# Pilot Runbook

This runbook defines the Milestone 18 separate-host pilot exercise.

It extends the existing launch gate with a real host topology, a dedicated
pilot config pack, repeatable fault scenarios, and a report format.

It is still a pilot-only document. It does not claim public-production or
hostile-environment readiness.

## Scope

Use this runbook to prove:

- 4 overlay nodes running on separate hosts
- 3 static bootstrap seeds over plain `http://`
- bootstrap plus real TCP session establishment across hosts
- the current publish, lookup, service-open, and relay-fallback proof flow
  against the same pilot topology pack
- restart/status evidence and three documented fault scenarios

Current pilot limits remain in force:

- bootstrap is still static JSON over plain `http://`
- `overlay-cli bootstrap-serve` is still a lab seed server, not public
  bootstrap infrastructure
- standalone distributed operator commands for publish, lookup, service-open,
  and relay intro do not exist yet
- the checked-in full-flow proof for publish, lookup, service-open, and relay
  fallback is still the smoke harness running against the pilot config model

## Topology

Use the files under [devnet/pilot](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot).

Suggested 5-host pilot:

- `host-a`: `node-a`, bootstrap seed server on `0.0.0.0:4301`
- `host-b`: `node-b`, bootstrap seed server on `0.0.0.0:4302`
- `host-c`: `node-c`
- `host-relay`: `node-relay`, bootstrap seed server on `0.0.0.0:4303`
- `host-ops`: operator/report host

Separate-host example addresses:

- `host-a`: `198.51.100.10`, `node-a` listener `198.51.100.10:4111`
- `host-b`: `198.51.100.11`, `node-b` listener `198.51.100.11:4112`
- `host-c`: `198.51.100.12`, `node-c` listener `198.51.100.12:4113`
- `host-relay`: `198.51.100.13`, `node-relay` listener `198.51.100.13:4198`
- `host-ops`: `198.51.100.14`

Files:

- rehearsal configs: [devnet/pilot/localhost/configs](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot/localhost/configs)
- rehearsal bootstrap seeds: [devnet/pilot/localhost/bootstrap](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot/localhost/bootstrap)
- separate-host example configs: [devnet/pilot/examples/configs](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot/examples/configs)
- separate-host example bootstrap seeds: [devnet/pilot/examples/bootstrap](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/pilot/examples/bootstrap)
- report template: [docs/PILOT_REPORT_TEMPLATE.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/PILOT_REPORT_TEMPLATE.md)

## Prerequisites

- Run the current launch gate first:

  ```bash
  ./devnet/run-launch-gate.sh
  ```

- Copy one config and one key file to each node host.
- Copy the three bootstrap JSON files to the bootstrap hosts.
- Use the same validated commit on all hosts.
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

2. Start the four nodes on their matching hosts:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config /path/to/node-a.json --status-every 30
   ```

3. Confirm each node emits:

   - `bootstrap_fetch`
   - `bootstrap_ingest`
   - `state_transition`

4. Confirm each node reports status:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- status --config /path/to/node-a.json
   ```

5. Use [devnet/run-distributed-smoke.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-distributed-smoke.sh)
   as the repo-local proof for bootstrap plus real TCP session establishment.

6. Use [devnet/run-pilot-checklist.sh](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/run-pilot-checklist.sh)
   as the pilot rehearsal pack for the full current publish, lookup,
   service-open, relay-fallback, restart, and fault-reporting flow against the
   Milestone 18 topology model.

## Pilot checklist command

Run from the repository root:

```bash
./devnet/run-pilot-checklist.sh
```

This script:

- starts the pilot bootstrap servers on `127.0.0.1:4301-4303`
- runs the baseline full smoke against `devnet/pilot/localhost`
- runs the `node-c-down` fault rehearsal
- runs the `relay-unavailable` fault rehearsal
- reruns the smoke with one bootstrap seed intentionally unavailable
- performs a pilot-config restart/status check on `node-b`
- prints JSON lines for each scenario plus a final `pilot_checklist_complete`
  summary with lookup latency and relay-usage fields

## Fault scenarios

Record all three scenarios in the pilot report:

1. `node-c-down`

   - `node-c` startup is skipped
   - expected outcome: the core flow still reaches `smoke_complete`
   - this proves the spare peer is not required for the current happy-path
     publish/lookup/open/relay rehearsal

2. `relay-unavailable`

   - `node-relay` startup is skipped
   - expected outcome: bootstrap, publish, lookup, and service-open still
     complete; relay fallback stops after `relay_fallback_planned` and reports
     `relay_fallback_unavailable`

3. `bootstrap-seed-unavailable`

   - the `node-a-seed.json` server on `127.0.0.1:4302` is intentionally stopped
   - expected outcome: the smoke still reaches `smoke_complete` using the
     remaining seed URLs

## Evidence to collect

- the exact commit SHA
- the `./devnet/run-launch-gate.sh` result
- the `./devnet/run-distributed-smoke.sh` result
- the `./devnet/run-pilot-checklist.sh` output
- per-host `overlay-cli status --config ...` JSON snapshots
- lookup latency values from `lookup_node` / `smoke_complete`
- relay usage from `relay_fallback_bound` / `smoke_complete`
- restart evidence from the persisted status JSON

Use [docs/PILOT_REPORT_TEMPLATE.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/PILOT_REPORT_TEMPLATE.md)
to write the report.
