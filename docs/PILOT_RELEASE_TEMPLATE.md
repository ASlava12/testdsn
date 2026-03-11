# Pilot Release Template

Use this template for each Milestone 18 pilot candidate tag.

Do not describe the release as GA, production-ready, or ready for hostile
public deployment.

## Release metadata

- Tag: `pilot-v0.1.0-rcN`
- Repository stage: `milestone-18-real-pilot`
- Commit: `<git-sha>`
- Release date: `<YYYY-MM-DD>`
- Operator: `<name>`

## Summary

Short statement of what this pilot candidate is intended to prove.

## Frozen launch surface

- `overlay-cli run` for single-node bounded startup and status inspection
- `overlay-cli smoke --devnet-dir devnet` for the checked-in four-node green path
- exact `node_id` lookup, exact `app_id` service resolution, and one relay
  fallback path
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
- `./devnet/run-pilot-checklist.sh`: `<pass/fail>`
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

- in-memory runtime state only; restart rebuilds state from config and bootstrap
- minimal static bootstrap over plain `http://` only
- no standalone distributed operator CLI for publish, lookup, relay intro, or
  service open
- the full publish/lookup/service-open/relay proof still runs through the smoke
  harness against the validated config model
- relay fallback validated only on the documented `node-a -> node-relay ->
  node-b` path
- pilot-only release; not a public-production deployment claim

## Go / no-go

- [ ] Launch gate stayed green on the tagged commit.
- [ ] Release note matches the exact validated commit and tag.
- [ ] Known limitations are carried forward without dilution.
- [ ] The release is described as pilot-ready only.
