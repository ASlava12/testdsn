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
- Milestone 9 hardening and polish is now the active feature stage with the
  current regression suites and stage-boundary integration tests as the entry
  boundary until Milestone 9-specific hardening work lands.

For normal work, touch Milestones 1-8 only for regression fixes, spec
mismatches, vector maintenance, or validation maintenance unless the task
explicitly reopens that stage. Milestone 9 is now the next active feature
stage.

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
