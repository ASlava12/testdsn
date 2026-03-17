# AGENTS.md

This repository contains a specification-first implementation of a censorship-resistant overlay network.

Codex must follow these rules before making changes.

## 1. Primary sources of truth

Use these files in this order:

1. `spec/mvp-scope.md`
2. `spec/architecture.md`
3. `spec/wire-protocol.md`
4. `spec/records.md`
5. `spec/state-machines.md`
6. `IMPLEMENT.md`
7. `VALIDATION.md`

If code and spec disagree, treat the spec as the source of truth unless the task explicitly says to update the spec.

## 2. Scope discipline

Do not expand scope without being asked.

For MVP, implement only:
- node identity and key handling
- wire framing and message catalog
- session handshake
- transport abstraction
- peer manager
- bootstrap
- `PresenceRecord`
- exact lookup by `node_id`
- relay fallback reachability
- basic RTT/loss probes
- path score and hysteresis
- `ServiceRecord`
- `OpenAppSession`
- metrics and structured logs

Do not add by default:
- onion routing
- full post-quantum handshake
- global service discovery
- tokenomics/payment systems
- store-and-forward messaging
- distributed consensus not required by the specs

### Current repository baseline

- Milestone 0 bootstrap is complete.
- Milestone 1 identities, records, and wire foundations are implemented, vectorized, and validated.
- Milestone 2 crypto wrappers and handshake surface are implemented, vectorized, and validated.
- Milestone 2 is considered closed.
- Milestone 3 transport/session layer is implemented with a minimal runner boundary, explicit session-runner input surface, bounded local session stores, and an integration-level handshake-to-session scenario.
- Milestone 3 is considered closed.
- Milestone 4 peer/bootstrap layer is implemented with validated bootstrap responses, provider abstractions, a bounded peer store, and deterministic diversity-preserving rebalance.
- Milestone 4 is considered closed.
- Milestone 5 rendezvous/presence publish and exact lookup work is implemented,
  vectorized, and validated.
- Milestone 5 is considered closed.
- Milestone 6 relay intro and fallback work is implemented, vectorized, and
  validated with local quota enforcement, an explicit local role model,
  canonical relay-intro wire bodies, verified intro-ticket usage, and
  direct-first fallback planning.
- Milestone 6 is considered closed.
- Milestone 7 path metrics, deterministic scoring, and switch hysteresis work
  is implemented, vectorized, and validated with canonical path-probe wire
  bodies, a bounded local probe tracker, integer EWMA observation updates, and
  anti-flapping route selection tests.
- Milestone 7 is considered closed.
- Milestone 8 service-layer work is implemented, vectorized, and validated
  with canonical service wire bodies, verified `ServiceRecord` registration, a
  bounded local service registry and open-session store, exact `app_id`
  resolution, `reachability_ref` binding checks, and integration coverage.
- Milestone 8 is considered closed.
- Milestone 9 hardening and polish is implemented with bounded observability
  surfaces, validated config projection, a bounded handshake replay cache,
  explicit subsystem observability hooks, and the current regression plus
  stage-boundary validation suites.
- Milestone 10 minimal runtime is implemented with `overlay-cli run`, local
  config loading, local bootstrap-file startup, and bounded runtime ticks.
- Milestone 11 local devnet is implemented with the checked-in four-node
  `devnet/` and `overlay-cli smoke`.
- Milestone 12 launch hardening is implemented with bounded cleanup,
  conservative bootstrap retry, runtime health snapshots, status dumps, and the
  logical soak path.
- The current repository stage marker is
  `milestone-26-bounded-operator-control-plane`.
- Milestone 14 launch gate and pilot tag remains part of the landed pilot
  baseline.
- Milestone 16 network bootstrap and multi-host devnet is implemented with
  minimal static `http://` bootstrap fetch, `overlay-cli bootstrap-serve`,
  host-style devnet configs and smoke paths, the documented green-path
  validation and launch sequence, and explicit pilot-only limitations.
- Milestone 17 operator-grade runtime hardening is implemented with
  signal-aware graceful shutdown, restart-safe operator lock/status files under
  `.overlay-runtime/`, `overlay-cli status`, stricter startup/config
  validation, the bounded soak in the launch gate, and explicit pilot-only
  limitations.
- Milestone 18 real pilot remains part of the landed baseline with the
  dedicated `devnet/pilot/` topology pack, `devnet/run-pilot-checklist.sh`,
  and the first separate-host rehearsal/reporting docs.
- Milestone 19 pilot closure is part of the landed baseline with minimal distributed
  operator commands for `publish`, `lookup`, `open-service`, and
  `relay-intro`, the two-relay `devnet/pilot/` topology, conservative
  `http://...#sha256=<pin>` bootstrap-artifact integrity checks,
  `devnet/run-distributed-pilot-checklist.sh`, and synchronized post-pilot
  closure docs.
- Milestone 20 regular distributed use closure is part of the landed baseline with
  per-source bootstrap diagnostics on the runtime status surface, preferred
  retry/fallback ordering across configured bootstrap sources, expanded
  localhost checklist proof for unavailable/integrity/stale/empty bootstrap
  cases, stronger relay-bind evidence across the checked-in two-relay pilot
  topology, and reproducible `--evidence-dir` wrappers for the distributed
  smoke and pilot checklist. This stage still does not claim
  hostile-environment or public-production readiness.
- Milestone 21 first-user runtime is part of the landed baseline with bounded restart
  recovery from the last-known active bootstrap peers, continued bootstrap
  retry after peer-cache recovery, `overlay-cli status --summary`,
  `overlay-cli doctor`, stable first-user example profiles, and more
  actionable config validation. This stage still does not claim
  hostile-environment or public-production readiness.
- Milestone 22 first-user acceptance pack remains part of the landed baseline
  with the bounded `./devnet/run-first-user-acceptance.sh` wrapper, explicit
  first-user-ready acceptance scenarios, fresh-node-join and
  relay-unavailable-service-open proof inside the distributed checklist, and
  synchronized first-user-ready boundary docs.
- Milestone 24 bootstrap trust and delivery hardening remains part of the
  landed baseline with signed bootstrap artifacts served over static `http://`
  sources, pinned `ed25519=<hex>` signer trust roots with optional
  `sha256=<hex>` integrity pins, richer bootstrap diagnostics for trust
  failures, and synchronized operator/bootstrap docs. This stage still does
  not claim hostile-environment or public-production readiness.
- Milestone 25 runtime persistence and recovery hardening remains part of the
  landed baseline
  with bounded restart recovery of persisted bootstrap-source preference,
  last-known active bootstrap peers, and local service registration intent,
  explicit recovery fields on the status/doctor surfaces, updated restart
  smoke/checklist proof, and synchronized recovery/runbook docs. This stage
  still does not claim hostile-environment or public-production readiness.
- Milestone 26 bounded operator control plane is the current stage with
  `overlay-cli inspect`, machine-readable bounded operator reports that bundle
  local status/doctor data with explicit remote lookup/service/relay probes,
  improved operator runbooks, and synchronized Milestone 26 docs. This stage
  still does not claim hostile-environment or public-production readiness.

For normal work, touch Milestones 1-12 only for regression fixes, spec
mismatches, vector maintenance, validation maintenance, or launch-maintenance
updates unless the task explicitly reopens that stage. Treat current-stage work as narrow
`milestone-26-bounded-operator-control-plane` work, not as a restart from
earlier milestones.

## 3. Change policy

Before large edits:
- read the relevant spec files;
- update docs if behavior changes;
- keep changes minimal and local;
- avoid renaming files or modules unless required.

When a behavior is underspecified:
- first check `docs/OPEN_QUESTIONS.md`;
- if the answer is still missing, implement the smallest conservative option and document it there.

## 4. Layering rules

Do not collapse these layers together:
- identity
- transport/session
- peer management
- rendezvous/presence
- relay
- routing/path scoring
- service layer

Keep APIs explicit between modules.

## 5. Coding rules

- Rust edition: 2021
- Prefer simple data structures over clever abstractions.
- Keep hot-path structures compact.
- Avoid unnecessary heap allocation in record parsing and path metrics.
- Use bounded stores and explicit limits.
- Add structured errors instead of stringly-typed failures.
- Do not silently ignore invalid signatures, stale records, or replay-risk states.

## 6. Testing rules

Every non-trivial change should include at least one of:
- unit test
- integration test
- test vector update

For protocol changes, update:
- `spec/wire-protocol.md`
- `spec/records.md`
- `tests/vectors/*` when applicable

## 7. Validation rules

Before finishing a task, run the commands listed in `VALIDATION.md` that apply to the changed code.
If a command cannot run yet because the repository is still incomplete, say so explicitly in the final report.

## 8. Reporting format

At the end of each task, report:
1. what changed;
2. what spec files were used;
3. what validation ran;
4. what remains blocked or underspecified.

## 9. Safe defaults for underspecified areas

Until a more detailed spec lands:
- use big-endian integers in wire headers;
- reject expired records as fresh lookup results;
- higher epoch wins, then higher sequence;
- exact lookup only, no prefix/range scan;
- prefer direct transport first, then relay fallback;
- use EWMA and hysteresis for route switching.
