# Host-Style Devnet Layouts

This directory contains the Milestone 16 host-style devnet assets carried into
the current Milestone 20 regular-distributed-use-closure stage.

## Layouts

- `localhost/`: runnable configs for the repo-local multi-host smoke.
  These use real pinned `http://127.0.0.1:*#sha256=<pin>` bootstrap sources and real
  `tcp://127.0.0.1:*` listener addresses.
- `examples/`: copy-and-edit examples for separate hosts or VMs.
  These use RFC 5737 documentation addresses and assume three static bootstrap
  seed servers:
  - `198.51.100.10:4201` serving `node-foundation.json`
  - `198.51.100.11:4202` serving `node-a-seed.json`
  - `198.51.100.13:4203` serving `node-ab-seed.json`

## Roles

- `node-a`: bootstrap anchor and smoke-flow client
- `node-b`: presence publisher and service host
- `node-c`: extra standard peer
- `node-relay`: relay-enabled node for fallback

## Bootstrap model

The host-style configs intentionally keep bootstrap minimal:

- bootstrap responses stay static JSON with the existing schema;
- nodes fetch them over static pinned `http://...#sha256=<pin>` URLs;
- nodes may list more than one seed URL for conservative fallback;
- the seed server is `overlay-cli bootstrap-serve`, not a public provider stack.

For the current dedicated pilot pack, use
`devnet/pilot/` and [docs/PILOT_RUNBOOK.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/PILOT_RUNBOOK.md).
