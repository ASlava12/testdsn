# Production Release Template

Use this template for each bounded Milestone 28 production release candidate.

Do not describe the release as hostile-environment-ready, anonymous, or ready
for broad public-Internet deployment.

## Release metadata

- Release version: `v0.1.0`
- Repository stage: `milestone-28-production-gates-packaging-safety-hardening`
- Commit: `<git-sha>`
- Release date: `<YYYY-MM-DD>`
- Operator: `<name>`
- Package path: `<path/to/overlay-v0.1.0-<target>.tar.gz>`
- Package checksum file: `<path/to/overlay-v0.1.0-<target>.tar.gz.sha256>`

## Summary

Short statement of what this bounded production release is intended to support.

## Production gate evidence

- `cargo fmt --all --check`: `<pass/fail>`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: `<pass/fail>`
- `cargo check --workspace`: `<pass/fail>`
- `cargo test --workspace`: `<pass/fail>`
- `./devnet/run-production-gate.sh`: `<pass/fail>`
- `./devnet/run-first-user-acceptance.sh`: `<pass/fail>`
- `./devnet/run-production-soak.sh`: `<pass/fail>`
- `./devnet/run-packaging-check.sh`: `<pass/fail>`
- `./devnet/package-release.sh`: `<pass/fail>`
- separate-host evidence report: `<path/to/pilot-report>`

## Bounded production claim

This release is bounded to:

- static signed bootstrap artifacts over `http://` with pinned signer keys and
  optional SHA-256 pins;
- explicit operator-driven CLI workflows;
- the checked-in bounded three-relay topology and its documented degraded
  cases;
- bounded restart recovery of bootstrap-source preference, active bootstrap
  peers, and local service registration intent;
- the packaged binary/docs/examples validated by the Milestone 28 gate.

## Package contents

- binary: `overlay-cli`
- docs: production checklist, release template, known limitations,
  first-user acceptance boundary, pilot runbook, and troubleshooting docs
- examples: config examples plus host-style and pilot example layouts
- private keys included: `no`

## Separate-host evidence

- runbook used: `docs/PILOT_RUNBOOK.md`
- report path: `<path>`
- exact hosts and IPs recorded: `<yes/no>`
- exact commit SHA recorded on every host: `<yes/no>`
- degraded/fallback scenarios recorded: `<yes/no>`

## Known limitations

Copy these from [docs/KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md) without
dilution:

- `<limitation 1>`
- `<limitation 2>`
- `<limitation 3>`

## Go / no-go

- [ ] Production gate stayed green on the release commit.
- [ ] Release package and checksum were generated from the same validated commit.
- [ ] Separate-host evidence for the same commit is attached.
- [ ] Known limitations are carried forward without dilution.
- [ ] The release note stays inside the bounded Milestone 28 production claim.
