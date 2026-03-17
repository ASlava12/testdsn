# Pilot Devnet

This directory contains the Milestone 20 regular-distributed-use assets carried
into the current Milestone 27 relay-topology-generalization stage.

It keeps the landed pilot topology pack and extends it with:

- `localhost/`: runnable localhost rehearsal configs and signed bootstrap seed
  artifacts for `./devnet/run-distributed-pilot-checklist.sh`
- `examples/`: copy-and-edit configs and bootstrap seeds for a real 6-node,
  5-6 host pilot with three relay-capable fallback paths

Suggested host map:

- `host-a`: `node-a`, static bootstrap seed server for `node-foundation.json`
- `host-b`: `node-b`, static bootstrap seed server for `node-a-seed.json`
- `host-c`: `node-c`
- `host-relay-a`: `node-relay`, static bootstrap seed server for `node-ab-seed.json`
- `host-relay-b-ops`: `node-relay-b`, operator/report host running status
  collection and report assembly
- `host-relay-c`: `node-relay-c`

Use [docs/PILOT_RUNBOOK.md](../../docs/PILOT_RUNBOOK.md)
for the full execution order, distributed operator commands, fault scenarios,
and reporting steps.
