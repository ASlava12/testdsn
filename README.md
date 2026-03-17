# Overlay

Specification-first Rust workspace for a censorship-resistant overlay network.

## Current Stage

The current repository stage is
`milestone-24-bootstrap-trust-delivery-hardening`.

Milestones 0-21 are landed baseline work. Current tasks should stay narrow:
maintain the hardened bootstrap boundary, keep the acceptance/runbook docs
honest, and fix validation or runtime regressions without widening protocol
scope.

## Current Green Path

Use this repository in the current stage with one sign-off flow:

1. run the applicable commands in `VALIDATION.md`;
2. run `./devnet/run-first-user-acceptance.sh` on the same commit;
3. use `docs/PILOT_RUNBOOK.md` to collect separate-host evidence before
   claiming first-user-ready status on that commit.

`./devnet/run-launch-gate.sh` and
`./devnet/run-distributed-pilot-checklist.sh` remain the landed component
scripts inside that acceptance flow. `./devnet/run-pilot-checklist.sh` is
retained only as the older Milestone 18 localhost rehearsal pack.

## Current Acceptance Surface

The current validated surface includes:

- node identity, wire framing, handshake, transport/session, peer/bootstrap,
  presence publish, exact lookup by `node_id`, relay fallback, path scoring,
  service registration/open, and structured metrics/logs;
- `overlay-cli run`, `status`, `status --summary`, `doctor`,
  `bootstrap-serve`, `bootstrap-sign`, `publish`, `lookup`, `open-service`,
  and `relay-intro`;
- repo-local proof paths in `devnet/run-smoke.sh`,
  `devnet/run-distributed-smoke.sh`, `devnet/run-multihost-smoke.sh`,
  `devnet/run-launch-gate.sh`,
  `devnet/run-first-user-acceptance.sh`, and
  `devnet/run-distributed-pilot-checklist.sh`;
- bounded per-source bootstrap diagnostics in `runtime_status.health.bootstrap`
  with `last_attempt_summary` and `last_sources`, including explicit
  `unavailable`, `integrity_mismatch`, `trust_verification_failed`, `stale`,
  and `empty_peer_set` outcomes;
- bounded restart recovery from the last-known active bootstrap peers embedded
  in persisted `runtime_status`, plus continued bootstrap retry until a live
  source succeeds again;
- an explicit acceptance pack covering fresh join, service publish/open,
  relay-fallback proof, one-bootstrap-down startup, one-relay-down service
  open, ordinary restart recovery, and stale-state cleanup;
- the dedicated distributed pilot pack under `devnet/pilot/`.

## First-User Ready Boundary

The current repo may be described as sufficiently working for first users only
within this bounded claim:

- the exact acceptance scenarios in `docs/FIRST_USER_ACCEPTANCE.md` passed on
  the same commit;
- operators use static signed bootstrap artifacts over `http://`, pinned
  signer keys with optional SHA-256 pins, explicit point-to-point operator
  commands, and the checked-in two-relay pilot topology;
- expected degraded cases remain explicit, including one failed primary
  relay-intro during relay-unavailable rehearsal and rejected tampered
  bootstrap artifacts;
- separate-host evidence is still attached before the claim is used for a
  release note.

## Primary Docs

- `HANDOFF.md`: current stage summary and first-task guidance
- `IMPLEMENT.md`: repository stage history and current execution boundaries
- `VALIDATION.md`: required validation commands and current sign-off order
- `docs/FIRST_USER_ACCEPTANCE.md`: exact acceptance scenarios and boundary
- `docs/LAUNCH_CHECKLIST.md`: current launch gate and localhost sign-off flow
- `docs/PILOT_RUNBOOK.md`: separate-host pilot execution and evidence
- `docs/DEVNET.md`: checked-in devnet layouts and proof wrappers
- `docs/OPEN_QUESTIONS.md`: conservative defaults for underspecified areas

## Remaining Limitations After Milestone 24

- bootstrap is still static signed artifact delivery over `http://`, not HTTPS
  or a public trust framework
- the distributed operator commands are one-shot point-to-point proof
  surfaces, not a general distributed control plane or discovery layer
- only the last-known active bootstrap peers are recovered across restart;
  presence, registered services, sessions, relay tunnels, and path probes are
  still rebuilt
- relay fallback is proven for the checked-in two-relay pilot topology, not
  arbitrary relay graphs or public-network conditions
- off-box evidence still must be collected on the validated commit before a
  release note can claim first-user-ready status

## Stage Marker Discipline

The repository stage marker lives in the root `REPOSITORY_STAGE` file and in
`overlay_core::REPOSITORY_STAGE`. Keep `README.md`, `HANDOFF.md`,
`IMPLEMENT.md`, `VALIDATION.md`, `docs/FIRST_USER_ACCEPTANCE.md`,
`docs/PILOT_RUNBOOK.md`, `docs/DEVNET.md`, `docs/LAUNCH_CHECKLIST.md`, and
`docs/OPEN_QUESTIONS.md` synchronized with that marker whenever the stage
changes.

In sandboxed Linux-on-Windows environments, set `TMPDIR=/tmp` for commands
that link test binaries if the default temp directory is not writable.
