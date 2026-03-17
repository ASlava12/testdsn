# Pilot Report Template

Use this template for each Milestone 20 regular-distributed-use run.

Do not describe the result as GA, production-ready, or ready for hostile
public deployment.

## Metadata

- Date: `<YYYY-MM-DD>`
- Repository stage: `milestone-20-regular-distributed-use-closure`
- Commit: `<git-sha>`
- Operator: `<name>`
- Topology: `pilot-5-host-two-relay`
- Off-box run window: `<start-end with timezone>`

## Host Map

- `host-a` / `node-a` / `198.51.100.10`: `<status>`
- `host-b` / `node-b` / `198.51.100.11`: `<status>`
- `host-c` / `node-c` / `198.51.100.12`: `<status>`
- `host-relay-a` / `node-relay` / `198.51.100.13`: `<status>`
- `host-relay-b-ops` / `node-relay-b` / `198.51.100.15`: `<status>`

## Validation Summary

- Launch gate (`./devnet/run-launch-gate.sh`): `<pass/fail>`
- Host-style smoke (`./devnet/run-multihost-smoke.sh`): `<pass/fail>`
- Distributed pilot checklist (`./devnet/run-distributed-pilot-checklist.sh`): `<pass/fail>`
- Off-box baseline flow: `<pass/fail>`
- Off-box fault matrix: `<pass/fail>`

## Distributed Operator Flows

- `publish`:
  host=`<host>`; target=`<host:port>`; result=`<pass/fail>`; notes=`<notes>`
- `lookup`:
  host=`<host>`; target=`<host:port>`; result=`<pass/fail>`; latency_ms=`<value>`
- `open-service`:
  host=`<host>`; target=`<host:port>`; result=`<pass/fail>`; session_id=`<value>`
- `relay-intro` primary path:
  host=`<host>`; target=`<host:port>`; result=`<pass/fail>`; path=`node-a -> node-relay -> node-b`
- `relay-intro` alternate path:
  host=`<host>`; target=`<host:port>`; result=`<pass/fail>`; path=`node-a -> node-relay-b -> node-b`

## Lookup Latency

- Baseline lookup latency ms: `<value>`
- Node-down lookup latency ms: `<value>`
- Bootstrap-seed-unavailable lookup latency ms: `<value>`
- Measurement method: `overlay-cli lookup JSON output`

## Relay Fallback

- Primary relay path bound: `<yes/no>`
- Alternate relay path bound: `<yes/no>`
- Primary-relay-down outcome: `<alternate_path_used/fail>`
- `node-relay` status snapshot notes: `<notes>`
- `node-relay-b` status snapshot notes: `<notes>`
- Relay byte counters, if non-zero: `<notes>`

## Bootstrap Integrity

- Seed URLs used `#sha256=<pin>`: `<yes/no>`
- One-seed-down startup outcome: `<pass/fail>`
- Integrity-mismatch fallback outcome: `<pass/fail>`
- Stale-bootstrap fallback outcome: `<pass/fail>`
- Empty-peer-set fallback outcome: `<pass/fail>`
- Tampered bootstrap artifact rejected: `<yes/no>`
- Notes: `<notes>`

## Restart Outcomes

- Service-host restart check: `<pass/fail>`
- `startup_count` after restart: `<value>`
- `previous_shutdown_clean` after restart: `<value>`
- Notes: `<notes>`

## Fault Scenarios

- `node-c-down`: `<pass/fail>`; observed outcome: `<notes>`
- `relay-unavailable`: `<pass/fail>`; observed outcome: `<notes>`
- `bootstrap-seed-unavailable`: `<pass/fail>`; observed outcome: `<notes>`
- `integrity-mismatch-fallback`: `<pass/fail>`; observed outcome: `<notes>`
- `stale-bootstrap-fallback`: `<pass/fail>`; observed outcome: `<notes>`
- `empty-bootstrap-fallback`: `<pass/fail>`; observed outcome: `<notes>`
- `service-host-restart`: `<pass/fail>`; observed outcome: `<notes>`
- `tampered-bootstrap-artifact`: `<pass/fail>`; observed outcome: `<notes>`

## Evidence Bundle

- Launch gate output: `<path or notes>`
- Host-style smoke output: `<path or notes>`
- Distributed pilot checklist output: `<path or notes>`
- Distributed pilot checklist evidence dir, if used: `<path or notes>`
- Off-box command logs: `<path or notes>`
- Per-host status JSON snapshots: `<path or notes>`
- Additional operator notes: `<path or notes>`

## Known Limitations

- bootstrap remains static JSON served over `http://`; integrity comes from
  pinned SHA-256 artifact URLs rather than HTTPS or a public trust root
- the distributed operator commands are one-shot and operator-directed rather
  than a general distributed control plane
- lookup is still exact-by-`node_id` only, and service resolution is still
  exact-by-`app_id` only
- peers, presence, services, sessions, relay tunnels, and path probes remain
  in-memory runtime state
- this stage still does not claim hostile-environment or public-Internet
  rollout readiness

## Concrete Remaining Blockers

- `<blocker 1>`
- `<blocker 2>`
- `<blocker 3>`
