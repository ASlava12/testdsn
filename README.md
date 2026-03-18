# Overlay

Specification-first Rust workspace for a censorship-resistant overlay network.

## Current Stage

The current repository stage is
`milestone-28-production-gates-packaging-safety-hardening`.

Milestones 0-26 are landed baseline work. Current tasks should stay narrow:
maintain the hardened bootstrap and bounded recovery boundary, keep the
release/packaging docs honest, tighten bounded production gates, and improve
safety validation without widening protocol scope.

## Current Green Path

Use this repository in the current stage with one sign-off flow:

1. run the applicable commands in `VALIDATION.md`;
2. run `./devnet/run-production-gate.sh` on the same commit;
3. use `docs/PILOT_RUNBOOK.md` to collect separate-host evidence before
   claiming bounded production release status on that commit;
4. generate the ship artifact with `./devnet/package-release.sh` on that same
   validated commit.

`./devnet/run-first-user-acceptance.sh` remains the landed functional
acceptance component inside that production flow.
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
  `inspect`, `bootstrap-serve`, `bootstrap-sign`, `publish`, `lookup`,
  `open-service`, and `relay-intro`;
- `./devnet/run-production-gate.sh`, `./devnet/run-production-soak.sh`,
  `./devnet/run-packaging-check.sh`, and `./devnet/package-release.sh` for
  bounded production release validation and operator packaging;
- repo-local proof paths in `devnet/run-smoke.sh`,
  `devnet/run-distributed-smoke.sh`, `devnet/run-multihost-smoke.sh`,
  `devnet/run-launch-gate.sh`,
  `devnet/run-first-user-acceptance.sh`, and
  `devnet/run-distributed-pilot-checklist.sh`;
- bounded per-source bootstrap diagnostics in `runtime_status.health.bootstrap`
  with `last_attempt_summary` and `last_sources`, including explicit
  `unavailable`, `integrity_mismatch`, `trust_verification_failed`, `stale`,
  and `empty_peer_set` outcomes;
- bounded restart recovery from persisted bootstrap-source preference,
  last-known active bootstrap peers, and local service registration intent
  embedded in `runtime_status`, plus continued bootstrap retry until a live
  source succeeds again;
- a bounded operator inspection surface through `overlay-cli inspect`, which
  combines one local persisted status/doctor report with an explicit set of
  requested `lookup`, `open-service`, and `relay-intro` probes in one
  machine-readable result;
- an explicit acceptance pack covering fresh join, service publish/open,
  deterministic three-relay candidate proof, one-bootstrap-down startup,
  one-relay-down service open, repeated relay-bind failure recovery, ordinary
  restart recovery, and stale-state cleanup;
- the dedicated distributed pilot pack under `devnet/pilot/`.

## Bounded Production Boundary

The current repo may be described as a bounded production release only within
this claim:

- `./devnet/run-production-gate.sh` passed on the same commit;
- the exact acceptance scenarios in `docs/FIRST_USER_ACCEPTANCE.md` remain
  green on that same commit as a production-gate component;
- operators use static signed bootstrap artifacts over `http://`, pinned
  signer keys with optional SHA-256 pins, explicit bounded operator surfaces,
  reproducible checked release packages, and the checked-in bounded
  three-relay pilot topology;
- expected degraded cases remain explicit, including one failed primary
  relay-intro during relay-unavailable rehearsal, two failed relay-intro
  attempts before tertiary recovery in the repeated-failure rehearsal, and
  rejected tampered bootstrap artifacts;
- separate-host evidence from `docs/PILOT_RUNBOOK.md` is still attached before
  the claim is used for a release note.

## Primary Docs

- `HANDOFF.md`: current stage summary and first-task guidance
- `IMPLEMENT.md`: repository stage history and current execution boundaries
- `VALIDATION.md`: required validation commands and current sign-off order
- `docs/PRODUCTION_CHECKLIST.md`: bounded production release gate
- `docs/PRODUCTION_RELEASE_TEMPLATE.md`: bounded production release note template
- `docs/KNOWN_LIMITATIONS.md`: limitations that must ship with every release
- `docs/FIRST_USER_ACCEPTANCE.md`: exact acceptance scenarios and boundary
- `docs/LAUNCH_CHECKLIST.md`: current launch gate and localhost sign-off flow
- `docs/PILOT_RUNBOOK.md`: separate-host pilot execution and evidence
- `docs/DEVNET.md`: checked-in devnet layouts and proof wrappers
- `docs/OPEN_QUESTIONS.md`: conservative defaults for underspecified areas

## Remaining Limitations After Milestone 28

- bootstrap is still static signed artifact delivery over `http://`, not HTTPS
  or a public trust framework
- operator surfaces remain explicit and operator-directed; `overlay-cli
  inspect` bundles requested remote probes, but the repo still has no general
  distributed control plane or discovery layer
- release packages are validated and installable, but they still target
  operator-managed hosts with Rust-free binary distribution only; there is no
  platform matrix, service manager integration, or auto-updater
- restart recovery remains bounded to bootstrap-source state, last-known
  active bootstrap peers, and local service registration intent; presence
  records, service-open sessions, relay tunnels, and path probes are still
  rebuilt
- relay fallback is proven for the checked-in bounded three-relay pilot
  topology, not arbitrary relay graphs or public-network conditions
- off-box evidence still must be collected on the validated commit before a
  release note can claim bounded production release status

## Stage Marker Discipline

The repository stage marker lives in the root `REPOSITORY_STAGE` file and in
`overlay_core::REPOSITORY_STAGE`. Keep `README.md`, `HANDOFF.md`,
`IMPLEMENT.md`, `VALIDATION.md`, `docs/FIRST_USER_ACCEPTANCE.md`,
`docs/PRODUCTION_CHECKLIST.md`, `docs/PILOT_RUNBOOK.md`, `docs/DEVNET.md`,
`docs/LAUNCH_CHECKLIST.md`, and `docs/OPEN_QUESTIONS.md` synchronized with
that marker whenever the stage changes.

In sandboxed Linux-on-Windows environments, set `TMPDIR=/tmp` for commands
that link test binaries if the default temp directory is not writable.
