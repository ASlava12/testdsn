# Pilot Release Template

Use this template for each current-stage distributed pilot candidate tag.

Do not describe the release as GA, production-ready, or ready for hostile
public deployment.

## Release metadata

- Tag: `pilot-v0.1.0-rcN`
- Repository stage: `milestone-27-relay-topology-generalization`
- Commit: `<git-sha>`
- Release date: `<YYYY-MM-DD>`
- Operator: `<name>`

## Summary

Short statement of what this pilot candidate is intended to prove.

## Frozen launch surface

- `overlay-cli run` for single-node bounded startup and status inspection
- `./devnet/run-first-user-acceptance.sh` as the current localhost sign-off
  path, reusing the landed launch gate and distributed checklist
- supporting repo-local proof paths in `./devnet/run-smoke.sh`,
  `./devnet/run-distributed-smoke.sh`, and `./devnet/run-multihost-smoke.sh`
- `overlay-cli inspect` for bounded machine-readable operator reports over the
  current host-style proof path
- exact `node_id` lookup, exact `app_id` service resolution, and the three
  documented relay fallback paths
- structured JSON logs and `runtime_status` snapshots

## Launch gate evidence

- `cargo fmt --all --check`: `<pass/fail>`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: `<pass/fail>`
- `cargo check --workspace`: `<pass/fail>`
- `cargo test --workspace`: `<pass/fail>`
- `./devnet/run-first-user-acceptance.sh`: `<pass/fail>`
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
- `operator_inspect`: `<notes>`
- `smoke_complete`: `<notes>`

## Known limitations

- restart recovery is bounded to bootstrap-source preference, last-known
  active bootstrap peers, and local service registration intent; presence,
  services, sessions, relay tunnels, and path probes still rebuild
- static signed bootstrap over `http://...#ed25519=<pin>` with optional
  `#sha256=<pin>` integrity checks only
- operator surfaces are explicit and operator-directed; `overlay-cli inspect`
  improves repeatable checks but does not add a distributed control plane
- off-box evidence must still be collected manually from separate hosts
- relay fallback validated only on the documented
  `node-a -> node-relay -> node-b`,
  `node-a -> node-relay-b -> node-b`, and
  `node-a -> node-relay-c -> node-b` paths
- pilot-only release; not a public-production deployment claim

## Go / no-go

- [ ] Launch gate stayed green on the tagged commit.
- [ ] Release note matches the exact validated commit and tag.
- [ ] Known limitations are carried forward without dilution.
- [ ] The release is described as pilot-ready only.
