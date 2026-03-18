# Production Checklist

This checklist defines the current Milestone 28
`production-gates-packaging-safety-hardening` release gate.

It supports only a bounded production claim for operator-managed deployments
that stay inside the documented bootstrap, topology, and operator-surface
limits. It is not a hostile-environment or broad public-Internet claim.

## Required same-commit command order

Run from the repository root on the exact commit you intend to release:

```bash
cargo fmt --all --check
TMPDIR=/tmp cargo clippy --workspace --all-targets --all-features -- -D warnings
TMPDIR=/tmp cargo check --workspace
TMPDIR=/tmp cargo test --workspace
./devnet/run-production-gate.sh --evidence-dir /tmp/overlay-production-gate
```

The current production gate reuses these component proofs:

- `./devnet/run-first-user-acceptance.sh`
- `./devnet/run-production-soak.sh`
- `./devnet/run-packaging-check.sh`

After the gate stays green, produce the release artifact on the same commit:

```bash
./devnet/package-release.sh --output-dir /tmp/overlay-release
```

Before publishing the release note, collect the required separate-host
evidence on the same commit using [docs/PILOT_RUNBOOK.md](PILOT_RUNBOOK.md).

## Pass criteria

- workspace format, lint, build, and tests all pass;
- `./devnet/run-production-gate.sh` reaches `production_gate_complete`;
- the first-user acceptance component still reaches
  `first_user_acceptance_complete`;
- the longer soak reaches `soak_complete` with `soak_seconds: 3600`;
- the packaging check reaches `packaging_check_complete`;
- the release package checksum verifies and the install path runs cleanly;
- the installed binary reports the same repository stage as the repo marker;
- the release package excludes private key material;
- separate-host evidence is attached for the same commit before the release
  note is finalized;
- the release note carries forward the exact limits from
  [docs/KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md).

## No-go conditions

Do not describe the release as within the current bounded production claim if
any of the following are true:

- the release note omits or dilutes the limits in
  [docs/KNOWN_LIMITATIONS.md](KNOWN_LIMITATIONS.md);
- the packaged artifact was built from a different commit than the validated
  gate and separate-host evidence;
- private keys or signer secrets appear inside the package;
- the separate-host report is missing;
- the release note implies hostile-environment, public-Internet, anonymity, or
  autonomous control-plane behavior that the repo does not validate.

## Current bounded production claim

The current release may be described as production-ready only within these
bounds:

- small operator-managed deployments that stay inside the checked-in signed
  static bootstrap model;
- the checked-in bounded three-relay topology proof and the documented
  degraded/fallback cases;
- explicit CLI-driven operator workflows rather than autonomous orchestration;
- bounded restart recovery of bootstrap-source preference, active bootstrap
  peers, and local service registration intent only;
- the reproducible release package and install path validated by
  `./devnet/run-packaging-check.sh`.

Use [docs/PRODUCTION_RELEASE_TEMPLATE.md](PRODUCTION_RELEASE_TEMPLATE.md)
for the final release note.
