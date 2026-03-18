# First-User Acceptance

This document defines the functional acceptance boundary reused inside the
current Milestone 28 production-gates-packaging-safety-hardening release gate.

It remains the bounded basis for describing the repository as functionally
working for first users. On its own it is not the full Milestone 28
production-release claim, and it is still not a hostile-environment or
public-Internet claim.

## Current acceptance flow

Run from the repository root on the exact commit you intend to hand to first
users:

```bash
cargo fmt --all --check
TMPDIR=/tmp cargo clippy --workspace --all-targets --all-features -- -D warnings
TMPDIR=/tmp cargo check --workspace
TMPDIR=/tmp cargo test --workspace
./devnet/run-first-user-acceptance.sh
```

Optional evidence-preserving form:

```bash
./devnet/run-first-user-acceptance.sh --evidence-dir /tmp/overlay-first-user-acceptance
```

The acceptance wrapper reuses the landed component proofs:

- `./devnet/run-launch-gate.sh`
- `./devnet/run-distributed-pilot-checklist.sh`

The current Milestone 28 production gate adds two more bounded checks above
this acceptance layer:

- `./devnet/run-production-soak.sh`
- `./devnet/run-packaging-check.sh`

## Required acceptance scenarios

The current first-user-ready claim is bounded to these scenarios:

1. `fresh-node-join`
   - a fresh `node-c` starts after the rest of the pilot topology is already
     running;
   - `node-c` publishes presence to `node-a`;
   - `node-a` can look up `node-c`.

2. `service-publish`
   - `node-b` publishes a reachable presence record with the `service-host`
     capability.

3. `service-discover-and-open`
   - `node-a` looks up `node-b`;
   - `node-a` resolves and opens the `devnet:terminal` service on `node-b`.

4. `direct-path-loss-relay-fallback`
   - direct attempts are treated as lost for the rehearsal path;
   - relay fallback is planned and bound on the documented relay paths.

5. `three-relay-candidate-set`
   - the checked-in pilot pack exposes three bounded relay-capable candidates;
   - each documented relay path binds successfully in the baseline proof.

6. `bootstrap-source-unavailable`
   - one configured bootstrap source is unavailable;
   - startup still succeeds through the remaining configured sources.

7. `trust-verification-fallback`
   - one configured bootstrap source uses a deliberately bad signer pin;
   - startup still succeeds through a later configured trusted source;
   - runtime bootstrap status reports a trust-verification failure explicitly.

8. `relay-unavailable-service-open`
   - the primary relay is unavailable;
   - the primary relay-intro attempt degrades as expected;
   - the alternate relay path still binds;
   - `open-service` still succeeds where expected.

9. `repeated-relay-bind-failure-recovery`
   - the primary and secondary relays are unavailable;
   - two relay-intro attempts fail explicitly;
   - the tertiary relay path still binds successfully.

10. `ordinary-restart-recovery`
   - a node receives an ordinary `SIGTERM` shutdown;
   - persisted status remains readable;
   - the next startup recovers usable bootstrap/source state through the
     bounded bootstrap-source-preference plus peer-cache path;
   - persisted local service registration intent is restored when it was
     present before shutdown;
   - later startup state remains explicitly marked as recovered.

11. `stale-presence-and-expired-state-recovery`
   - presence refresh republishes before local expiry;
   - stale managed sessions, stale service-open sessions, stale relay tunnels,
     and stale path probes are pruned during the bounded soak path.

## Scenario mapping to scripts

- `fresh-node-join`: `./devnet/run-distributed-pilot-checklist.sh`
- `service-publish`: `./devnet/run-distributed-pilot-checklist.sh`
- `service-discover-and-open`: `./devnet/run-distributed-pilot-checklist.sh`
- `direct-path-loss-relay-fallback`: `./devnet/run-distributed-pilot-checklist.sh`
- `three-relay-candidate-set`: `./devnet/run-distributed-pilot-checklist.sh`
- `bootstrap-source-unavailable`: `./devnet/run-distributed-pilot-checklist.sh`
- `trust-verification-fallback`: `./devnet/run-distributed-pilot-checklist.sh`
- `relay-unavailable-service-open`: `./devnet/run-distributed-pilot-checklist.sh`
- `repeated-relay-bind-failure-recovery`: `./devnet/run-distributed-pilot-checklist.sh`
- `ordinary-restart-recovery`: `./devnet/run-restart-smoke.sh` and the
  service-host-restart scenario inside
  `./devnet/run-distributed-pilot-checklist.sh`
- `stale-presence-and-expired-state-recovery`: `./devnet/run-soak.sh`

## Expected degraded or rejected cases

These outcomes are part of the current honest acceptance boundary:

- during `relay-unavailable`, one primary relay-intro failure against the
  unavailable relay is expected before the alternate relay path succeeds;
- during `repeated-relay-bind-failure-recovery`, two relay-intro failures
  against unavailable relays are expected before the tertiary path succeeds;
- a bootstrap source with a bad `#ed25519=<pin>` is expected to report
  `trust_verification_failed` and may still recover through a later trusted
  source;
- a tampered bootstrap artifact with a bad `#sha256=<pin>` is expected to
  report `integrity_mismatch` and may leave startup degraded;
- restart does not preserve presence records, service-open sessions, relay
  tunnels, or path probes beyond the bounded bootstrap-source, active-peer,
  and local-service-intent recovery state.

## Acceptance boundary inside the production gate

The repository may be described as functionally accepted for first-user
behavior only when all of the following are true:

- the commands above passed on the same commit;
- the acceptance wrapper reached `first_user_acceptance_complete`;
- operators stay within the checked-in topology, bounded explicit operator
  surfaces (`publish`, `lookup`, `open-service`, `relay-intro`, and
  `inspect`), the signed bootstrap-artifact model described in the runbooks,
  and the checked-in bounded three-relay pilot topology;
- separate-host evidence is collected for the same commit before the broader
  Milestone 28 bounded production release claim is used in a release note or
  handoff.

For the current Milestone 28 production claim, this acceptance document is
necessary but not sufficient on its own. Also require:

- `./devnet/run-production-gate.sh` on the same commit;
- the separate-host evidence run from `docs/PILOT_RUNBOOK.md`;
- the release-note limitations carried forward from `docs/KNOWN_LIMITATIONS.md`.

## Out of scope

The current first-user-ready claim does not include:

- public bootstrap-provider infrastructure or HTTPS trust distribution;
- a general distributed control plane or autonomous discovery;
- broad durable protocol-state persistence;
- arbitrary relay graphs or public-network relay closure;
- bounded production release packaging, install verification, or release-note
  requirements by themselves; those live in `docs/PRODUCTION_CHECKLIST.md`
  and `docs/PRODUCTION_RELEASE_TEMPLATE.md`;
- public-production beyond the bounded Milestone 28 claim, or
  hostile-environment deployment readiness.
