# Pilot Devnet

This directory contains the Milestone 20 regular-distributed-use assets.

It keeps the landed pilot topology pack and extends it with:

- `localhost/`: runnable localhost rehearsal configs and pinned bootstrap seeds
  for `./devnet/run-distributed-pilot-checklist.sh`
- `examples/`: copy-and-edit configs and bootstrap seeds for a real 5-node,
  5-host pilot with two relay-capable fallback paths

Suggested host map:

- `host-a`: `node-a`, static bootstrap seed server for `node-foundation.json`
- `host-b`: `node-b`, static bootstrap seed server for `node-a-seed.json`
- `host-c`: `node-c`
- `host-relay-a`: `node-relay`, static bootstrap seed server for `node-ab-seed.json`
- `host-relay-b-ops`: `node-relay-b`, operator/report host running status
  collection and report assembly

Use [docs/PILOT_RUNBOOK.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/PILOT_RUNBOOK.md)
for the full execution order, distributed operator commands, fault scenarios,
and reporting steps.
