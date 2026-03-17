# Pilot Release Template

Use this template for each current-stage distributed pilot candidate tag.

Do not describe the release as GA, production-ready, or ready for hostile
public deployment.

## Release metadata

- Tag: `pilot-v0.1.0-rcN`
- Repository stage: `milestone-21-first-user-runtime`
- Commit: `<git-sha>`
- Release date: `<YYYY-MM-DD>`
- Operator: `<name>`

## Summary

Short statement of what this pilot candidate is intended to prove.

## Frozen launch surface

- `overlay-cli run` for single-node bounded startup and status inspection
- `./devnet/run-launch-gate.sh` followed by
  `./devnet/run-distributed-pilot-checklist.sh` as the current localhost
  sign-off path
- supporting repo-local proof paths in `./devnet/run-smoke.sh`,
  `./devnet/run-distributed-smoke.sh`, and `./devnet/run-multihost-smoke.sh`
- exact `node_id` lookup, exact `app_id` service resolution, and the two
  documented relay fallback paths
- structured JSON logs and `runtime_status` snapshots

## Launch gate evidence

- `cargo fmt --all --check`: `<pass/fail>`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: `<pass/fail>`
- `cargo check --workspace`: `<pass/fail>`
- `cargo test --workspace`: `<pass/fail>`
- `cargo test -p overlay-core --test integration_bootstrap`: `<pass/fail>`
- `cargo test -p overlay-core --test integration_publish_lookup`: `<pass/fail>`
- `cargo test -p overlay-core --test integration_relay_fallback`: `<pass/fail>`
- `cargo test -p overlay-core --test integration_routing`: `<pass/fail>`
- `cargo test -p overlay-core --test integration_service_open`: `<pass/fail>`
- `./devnet/run-smoke.sh`: `<pass/fail>`
- `./devnet/run-distributed-smoke.sh`: `<pass/fail>`
- `./devnet/run-multihost-smoke.sh`: `<pass/fail>`
- `./devnet/run-soak.sh`: `<pass/fail>`
- `./devnet/run-doctor-smoke.sh`: `<pass/fail>`
- `./devnet/run-distributed-pilot-checklist.sh`: `<pass/fail>`
- `./devnet/run-restart-smoke.sh`: `<pass/fail>`

## Smoke summary

- `startup`: `<notes>`
- `session_established`: `<notes>`
- `publish_presence`: `<notes>`
- `lookup_node`: `<notes>`
- `open_service`: `<notes>`
- `relay_fallback_planned`: `<notes>`
- `relay_fallback_bound`: `<notes>`
- `smoke_complete`: `<notes>`

## Known limitations

- only the last-known active bootstrap peers are recovered across restart;
  presence, services, sessions, relay tunnels, and path probes still rebuild
- static bootstrap over pinned `http://...#sha256=<pin>` artifacts only
- distributed operator commands are one-shot and operator-directed
- off-box evidence must still be collected manually from separate hosts
- relay fallback validated only on the documented
  `node-a -> node-relay -> node-b` and `node-a -> node-relay-b -> node-b`
  paths
- pilot-only release; not a public-production deployment claim

## Go / no-go

- [ ] Launch gate stayed green on the tagged commit.
- [ ] Release note matches the exact validated commit and tag.
- [ ] Known limitations are carried forward without dilution.
- [ ] The release is described as pilot-ready only.
