# Overlay

Specification-first Rust workspace for a censorship-resistant overlay network.

## Current Stage

The current repository stage is `milestone-20-regular-distributed-use-closure`.

Milestones 0-18 are landed baseline work. Current tasks should stay narrow:
stabilize the regular distributed-use path, keep the launch/runbook docs
honest, and fix validation or operator-surface regressions without widening
protocol scope.

## Current Green Path

Use this repository in the current stage with one sign-off flow:

1. run the applicable commands in `VALIDATION.md`;
2. run `./devnet/run-launch-gate.sh`;
3. run `./devnet/run-distributed-pilot-checklist.sh` on the same commit;
4. use `docs/PILOT_RUNBOOK.md` to collect separate-host evidence before
   claiming regular distributed use on that commit.

`./devnet/run-pilot-checklist.sh` is retained only as the older Milestone 18
localhost rehearsal pack. It is not the current sign-off path.

## Current Regular-Distributed-Use Surface

The current validated surface includes:

- node identity, wire framing, handshake, transport/session, peer/bootstrap,
  presence publish, exact lookup by `node_id`, relay fallback, path scoring,
  service registration/open, and structured metrics/logs;
- `overlay-cli run`, `status`, `bootstrap-serve`, `publish`, `lookup`,
  `open-service`, and `relay-intro`;
- repo-local proof paths in `devnet/run-smoke.sh`,
  `devnet/run-distributed-smoke.sh`, `devnet/run-multihost-smoke.sh`,
  `devnet/run-launch-gate.sh`, and
  `devnet/run-distributed-pilot-checklist.sh`;
- bounded per-source bootstrap diagnostics in `runtime_status.health.bootstrap`
  with `last_attempt_summary` and `last_sources`;
- the dedicated distributed pilot pack under `devnet/pilot/`.

## Primary Docs

- `HANDOFF.md`: current stage summary and first-task guidance
- `IMPLEMENT.md`: repository stage history and current execution boundaries
- `VALIDATION.md`: required validation commands and current sign-off order
- `docs/LAUNCH_CHECKLIST.md`: current launch gate and localhost sign-off flow
- `docs/PILOT_RUNBOOK.md`: separate-host pilot execution and evidence
- `docs/DEVNET.md`: checked-in devnet layouts and proof wrappers
- `docs/OPEN_QUESTIONS.md`: conservative defaults for underspecified areas

## Remaining Blockers For Regular Distributed Use

- off-box evidence still must be collected on the validated commit before a
  release note can claim regular distributed use
- bootstrap is still static pinned `http://` artifact delivery, not HTTPS or a
  public trust framework
- the distributed operator commands are one-shot point-to-point proof
  surfaces, not a general distributed control plane or discovery layer
- peers, presence, services, sessions, relay tunnels, and path probes remain
  in-memory runtime state across restart
- relay fallback is proven for the checked-in two-relay pilot topology, not
  arbitrary relay graphs or public-network conditions

## Stage Marker Discipline

The repository stage marker lives in the root `REPOSITORY_STAGE` file and in
`overlay_core::REPOSITORY_STAGE`. Keep `README.md`, `HANDOFF.md`,
`IMPLEMENT.md`, `VALIDATION.md`, `docs/LAUNCH_CHECKLIST.md`, and
`docs/OPEN_QUESTIONS.md` synchronized with that marker whenever the stage
changes.

In sandboxed Linux-on-Windows environments, set `TMPDIR=/tmp` for commands
that link test binaries if the default temp directory is not writable.
