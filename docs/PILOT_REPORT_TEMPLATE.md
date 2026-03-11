# Pilot Report Template

Use this template for each Milestone 18 pilot run.

Do not describe the result as GA, production-ready, or ready for hostile
public deployment.

## Metadata

- Date: `<YYYY-MM-DD>`
- Repository stage: `milestone-18-real-pilot`
- Commit: `<git-sha>`
- Operator: `<name>`
- Topology: `pilot-5-host`

## Topology Summary

- `host-a` / `node-a`: `<status>`
- `host-b` / `node-b`: `<status>`
- `host-c` / `node-c`: `<status>`
- `host-relay` / `node-relay`: `<status>`
- `host-ops`: `<status>`

## Baseline Results

- Launch gate (`./devnet/run-launch-gate.sh`): `<pass/fail>`
- Distributed smoke (`./devnet/run-distributed-smoke.sh`): `<pass/fail>`
- Pilot checklist (`./devnet/run-pilot-checklist.sh`): `<pass/fail>`
- Bootstrap/startup notes: `<notes>`
- Session-establishment notes: `<notes>`
- Publish/lookup/service-open notes: `<notes>`
- Relay-fallback notes: `<notes>`

## Lookup Latency

- Baseline lookup latency ms: `<value>`
- Node-down lookup latency ms: `<value>`
- Bootstrap-seed-unavailable lookup latency ms: `<value>`
- Notes on measurement method: `current smoke-harness/local lookup timing`

## Relay Usage

- Relay fallback bound in baseline: `<yes/no>`
- Relay usage bytes in baseline: `<value>`
- Relay unavailable scenario result: `<expected_degraded/fail>`
- Notes: `<notes>`

## Restart Outcomes

- Pilot config restart check: `<pass/fail>`
- `startup_count` after second start: `<value>`
- `clean_shutdown` after restart: `<value>`
- Notes: `<notes>`

## Fault Scenarios

- `node-c-down`: `<pass/fail>`; observed outcome: `<notes>`
- `relay-unavailable`: `<pass/fail>`; observed outcome: `<notes>`
- `bootstrap-seed-unavailable`: `<pass/fail>`; observed outcome: `<notes>`

## Known Limitations

- bootstrap remains static JSON over plain `http://`
- the runtime remains in-memory for peers, presence, services, sessions, relay
  tunnels, and path probes
- standalone distributed operator commands for publish, lookup, service-open,
  and relay intro do not exist yet
- the full publish/lookup/service-open/relay proof still runs through the smoke
  harness against the pilot config model
- relay fallback remains documented for one path only:
  `node-a -> node-relay -> node-b`
- exact lookup is still `node_id` only and exact service resolution is still
  `app_id` only

## Concrete Blockers Before Wider Deployment

- `<blocker 1>`
- `<blocker 2>`
- `<blocker 3>`
