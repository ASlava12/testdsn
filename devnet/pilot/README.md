# Pilot Devnet

This directory contains the Milestone 18 pilot rehearsal assets.

It keeps the current launch surface unchanged and adds a dedicated topology pack
for the first separate-host pilot stage:

- `localhost/`: runnable localhost rehearsal configs and bootstrap seeds for
  `./devnet/run-pilot-checklist.sh`
- `examples/`: copy-and-edit configs and bootstrap seeds for a real 4-node,
  5-host pilot

Suggested host map:

- `host-a`: `node-a`, static bootstrap seed server for `node-foundation.json`
- `host-b`: `node-b`, static bootstrap seed server for `node-a-seed.json`
- `host-c`: `node-c`
- `host-relay`: `node-relay`, static bootstrap seed server for `node-ab-seed.json`
- `host-ops`: operator/report host running the checklist, status collection,
  and report assembly

Use [docs/PILOT_RUNBOOK.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/PILOT_RUNBOOK.md)
for the full execution order, fault scenarios, and reporting steps.
