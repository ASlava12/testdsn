# IMPLEMENT.md

This file is the execution plan for Codex.

## Current repository stage

The repository has a closed Milestone 1-8 baseline.
The current repository stage marker is `milestone-9-hardening`.

- Milestone 0 bootstrap is complete.
- Milestone 1 foundations are implemented, vectorized, and validated in
  `crates/overlay-core/src/identity.rs`, `crates/overlay-core/src/records/mod.rs`,
  and `crates/overlay-core/src/wire/mod.rs`.
- Milestone 2 crypto wrappers and handshake surface are implemented, vectorized,
  and validated in `crates/overlay-core/src/crypto/*` and
  `crates/overlay-core/src/session/handshake.rs`.
- Milestone 2 is considered closed.
- Milestone 3 transport abstraction and session-manager work is implemented and
  considered closed in `crates/overlay-core/src/transport/mod.rs` and
  `crates/overlay-core/src/session/manager.rs`, including an explicit runner
  boundary, runner-facing session inputs, explicit polled keepalive/timeout
  scaffolding, handshake-bound session context, bounded event and I/O-action
  stores, and an integration-level handshake-to-session scenario.
- Milestone 4 peer manager and bootstrap work is implemented and considered
  closed in `crates/overlay-core/src/bootstrap/mod.rs` and
  `crates/overlay-core/src/peer/mod.rs`, including validated bootstrap
  responses, static provider abstractions, a bounded peer store, diversity-aware
  rebalance, and bootstrap integration coverage.
- Milestone 5 rendezvous/presence work is implemented and considered closed in
  `crates/overlay-core/src/rendezvous/mod.rs`, including deterministic placement
  key derivation, bounded in-memory publish/lookup flows, canonical
  publish/lookup wire-body helpers with frame-size enforcement, deterministic
  publish/lookup message vectors, freshness and epoch/sequence conflict
  handling, bounded lookup state, negative cache behavior, verified-signature
  handoff at the store boundary, and publish/lookup integration coverage.
- Milestone 6 relay intro and fallback work is implemented in
  `crates/overlay-core/src/relay/mod.rs`, including profile-based bounded relay
  quota defaults, an explicit local relay role model, canonical
  `ResolveIntro` / `IntroResponse` wire-body helpers with deterministic relay
  intro message vectors, intro/tunnel/byte quota enforcement, verified
  `IntroTicket` usage, direct-first/relay-second reachability planning, and
  relay fallback integration coverage. Milestone 6 is considered closed.
- Milestone 7 routing metrics and path switching work is implemented in
  `crates/overlay-core/src/routing/mod.rs`, including deterministic path
  scoring, canonical `PathProbe` / `PathProbeResult` wire-body helpers with
  deterministic path-probe message vectors, a bounded local path probe tracker,
  integer EWMA updates for observed path metrics and probe feedback,
  hysteresis-gated route selection, anti-flapping unit coverage, and a routing
  stage-boundary integration scenario. Milestone 7 is considered closed.
- Milestone 8 service-layer work is implemented in
  `crates/overlay-core/src/service/mod.rs`, with canonical
  `GetServiceRecord` / `ServiceRecordResponse` and
  `OpenAppSession` / `OpenAppSessionResult` wire-body helpers, deterministic
  service message vectors, verified `ServiceRecord` registration, a bounded
  local service registry and open-session store, exact `app_id` resolution,
  `reachability_ref` binding checks, allow/deny local policy enforcement, and
  integration coverage in
  `crates/overlay-core/tests/integration_service_open.rs`. Milestone 8 is
  considered closed.
- Milestone 9 hardening and polish is now active with initial bounded
  observability groundwork in `crates/overlay-core/src/metrics/mod.rs` and a
  validated top-level config baseline in `crates/overlay-core/src/config.rs`
  with explicit transport-buffer projection in
  `crates/overlay-core/src/transport/mod.rs` and runner-boundary
  `TransportPollEvent` validation in `crates/overlay-core/src/session/manager.rs`,
  with a bounded handshake transcript replay cache now landed in
  `crates/overlay-core/src/session/manager.rs`.
  Observability integration is now explicitly wired into bootstrap provider
  fetch/validation, peer bootstrap ingest, rendezvous publish/lookup, relay
  bind and rate-limit handling, routing probe/switch paths, service registry
  flows, and session event export, and malformed-input coverage now explicitly
  exercises bootstrap schema validation across schema/version, timing, frame
  limits, and duplicate peer/bridge-hint rejection, peer ingest rejection
  handling, rendezvous response-shape validation, and relay, routing, and
  service wire-body rejection paths.
  Session observability now also has an explicit established-session gauge sync
  helper while keeping aggregation caller-invoked. The remaining active
  Milestone 9 work is broader stale/malformed-input expansion, validation
  maintenance, and any remaining explicit observability aggregation
  boundaries, and the current validation boundary is the existing regression
  suites,
  stage-boundary integration tests, and Milestone 9 unit coverage in
  `bootstrap::tests`, `config::tests`, `metrics::tests`, `peer::tests`,
  `rendezvous::tests`, `relay::tests`, `routing::tests`, `service::tests`, and
  `session::manager::tests`, plus `transport::tests`.

Treat Milestones 0-8 as a closed baseline. Prefer regression fixes,
spec-conformance fixes, vector maintenance, and validation maintenance there
over refactoring the already present work.

## Recommended next Codex task

Use `prompts/codex-milestone-9.md` as the recommended next-task prompt for the
current `milestone-9-hardening` stage and continue Milestone 9 conservatively
from the current hardening boundary:

1. keep hardening work aligned with `spec/observability.md`,
   `spec/threat-model.md`, `spec/config.md`, and `docs/OPEN_QUESTIONS.md`;
2. do not treat Milestone 9 as a broad umbrella task; take one concrete
   hardening slice at a time around local rate limits, byte budgets,
   replay-risk mitigation, structured metrics/logs, stale or malformed input
   coverage, or validation maintenance;
3. keep Milestones 1-8 limited to regression fixes, vector maintenance, or
   validation maintenance;
4. update status docs, prompts, and `docs/OPEN_QUESTIONS.md` whenever the
   documented baseline changes;
5. keep simulation-focused expansion or broader protocol scope out of work
   until Milestone 9 is materially complete.

## Milestone 0 — repository bootstrap

Status: already completed in this repository. Do not rerun from scratch.

### Goal
Create a clean Rust workspace and all spec files, with no protocol logic yet.

### Deliverables
- workspace `Cargo.toml`
- crate skeletons
- `spec/*.md`
- `AGENTS.md`
- `VALIDATION.md`
- test and simulation placeholders

### Done when
- `cargo check --workspace` passes for the skeleton
- repository layout matches `docs/REPO_LAYOUT.md`

---

## Milestone 1 — identities, records, and wire base

Status: closed in this repository. Use this milestone only for regression
fixes, fixture maintenance, or spec-conformance fixes.

### Goal
Implement the immutable foundations that other layers depend on.

### Tasks
1. Implement `NodeId` and `AppId` derivation.
2. Add canonical Rust types for:
   - `NodeRecord`
   - `PresenceRecord`
   - `ServiceRecord`
   - `RelayHint`
   - `IntroTicket`
3. Implement the common frame header.
4. Add message type enums and base message structures.
5. Add basic validation helpers for IDs and record freshness.

### Important constraints
- No network I/O in this milestone.
- Keep field names aligned with `spec/records.md`.
- Do not invent additional record fields.

### Done when
- unit tests and vectors exist for `node_id` / `app_id`
- record structs compile and serialize deterministically with record vectors
- frame header encode/decode round-trips and matches the vector fixture

---

## Milestone 2 — crypto wrappers and handshake

Status: closed in this repository. Treat this milestone as regression-fix,
fixture-maintenance, or spec-conformance work only.

### Goal
Implement a minimal secure handshake for session establishment.

### Tasks
1. Add crypto wrappers for:
   - BLAKE3
   - Ed25519 sign/verify
   - X25519 key exchange
   - HKDF-SHA256
   - ChaCha20-Poly1305
2. Implement:
   - `ClientHello`
   - `ServerHello`
   - `ClientFinish`
3. Implement transcript hashing and session key derivation.
4. Add handshake validation and error mapping.
5. Add handshake test vectors.

### Important constraints
- No hybrid/PQ suites in MVP.
- Bind the transcript to peer identity.
- Explicitly reject downgrade, invalid signatures, and replay-unsafe states.

### Done when
- handshake unit tests pass
- transcript test vectors exist
- invalid transcript cases fail cleanly, including identity-binding and replay-unsafe rejection

---

## Milestone 3 — transport abstraction and session manager

Status: closed in this repository. Reopen only for regression fixes,
runner-boundary adjustments, fixture maintenance, or conservative
spec-conformance fixes.

### Goal
Create a stable session layer independent from specific transports.

### Tasks
1. Define the `Transport` trait.
2. Add placeholder transport adapters:
   - TCP
   - QUIC
   - WebSocket/HTTPS tunnel
   - relay transport
3. Implement session states and transitions.
4. Add keepalive/timeout handling.
5. Add structured session events.
6. Define the minimal runner boundary between session and transport.
7. Bound local session event and I/O-action stores.
8. Add an integration-level handshake-to-session scenario.

### Important constraints
- Keep transport-specific logic behind the trait.
- Do not implement full real QUIC/WS protocol behavior yet unless required by the task.

### Done when
- session manager compiles
- session state machine matches `spec/state-machines.md`
- placeholder runner boundary exists for open/send/close/poll
- session tests cover open/close/error/degraded/recovery transitions, timer scaffolding, and bounded stores
- an integration test covers handshake outcome binding through the session runner surface

---

## Milestone 4 — peer manager and bootstrap

Status: closed in this repository. Reopen only for regression fixes,
fixture maintenance, bootstrap-schema adjustments, or conservative
spec-conformance fixes.

### Goal
Allow a node to obtain peers and maintain a bounded neighbor set.

### Tasks
1. Implement `NeighborState` and bounded peer store.
2. Implement diversity filters.
3. Implement bootstrap provider abstractions.
4. Add bootstrap response validation.
5. Add neighbor rebalance policy.

### Important constraints
- Do not prefer only the lowest-latency peers.
- Preserve random and diversity-driven slots.

### Done when
- bootstrap smoke tests pass
- peer limits are enforced
- diversity filters are exercised in tests

---

## Milestone 5 — presence publish and exact lookup

Status: closed in this repository. Reopen only for regression fixes, vector
maintenance, or conservative spec-conformance fixes.

### Goal
Make nodes discoverable by exact `node_id` without open enumeration.

### Tasks
1. Implement rendezvous placement key derivation.
2. Implement `PublishPresence` / `PublishAck`.
3. Implement exact `LookupNode` / `LookupResult` / `LookupNotFound`.
4. Add TTL/epoch/sequence conflict handling.
5. Add bounded lookup budgets, seen-set, and negative cache.

### Important constraints
- Exact lookup only.
- No range or prefix scan.
- Expired records must not be returned as fresh.
- The current rendezvous store expects `PresenceRecord` signatures to be
  validated upstream before `publish_verified`.

### Done when
- publish/lookup integration smoke tests pass
- conflict resolution tests pass
- lookup terminates within configured budget

---

## Milestone 6 — relay intro and fallback connectivity

Status: closed in this repository. Reopen only for regression fixes, vector
maintenance, validation maintenance, or conservative spec-conformance fixes.

### Goal
Allow nodes to reach each other when direct transport is unavailable.

### Tasks
1. Implement relay role model and quotas.
2. Implement relay scoring.
3. Implement `IntroTicket` validation and usage.
4. Implement direct-first, relay-second connection policy.
5. Add relay fallback integration tests.

### Important constraints
- Do not make a single relay mandatory.
- Maintain secondary relay candidates.
- Prefer direct transport first and use relay only as bounded fallback.

### Done when
- relay fallback integration test passes
- expired/invalid tickets are rejected
- relay quotas are enforced locally

---

## Milestone 7 — routing metrics and path switching

Status: closed in this repository. Reopen only for regression fixes, vector
maintenance, validation maintenance, or conservative spec-conformance fixes.

### Goal
Add path quality measurement and stable route selection.

### Tasks
1. Implement `PathMetrics`.
2. Implement path observations and EWMA updates.
3. Implement path scoring.
4. Implement hysteresis and switch limits.
5. Add tests for anti-flapping behavior.

### Important constraints
- Do not switch on tiny metric changes.
- Keep route selection deterministic given the same inputs.

### Done when
- path score tests pass
- switching tests show no oscillation under small jitter
- a routing stage-boundary integration test passes

---

## Milestone 8 — service layer

Status: closed in this repository. Reopen only for regression fixes, vector
maintenance, validation maintenance, or conservative spec-conformance fixes.

### Goal
Open an application session after node reachability is resolved.

### Tasks
1. Implement `ServiceRecord` resolution by exact `app_id`.
2. Implement `OpenAppSession` flow.
3. Add local service registry.
4. Add service access policy checks.
5. Add integration test for service open.

### Important constraints
- No global service enumeration.
- Service access must remain separate from node reachability.

### Done when
- service integration test passes
- policy denial cases are covered

---

## Milestone 9 — hardening and polish

Status: active with observability/config groundwork, bounded replay-cache
hardening, and expanded malformed-input coverage landed in this repository.

### Goal
Close the highest-risk gaps before larger-scale simulation.

### Closeout path
1. Finish the remaining explicit observability aggregation helpers and keep
   them caller-invoked beyond the landed bootstrap fetch/validation logging and
   established-session gauge sync helper.
2. Complete stale/malformed rejection coverage across the remaining boundary
   message shapes and bounded local stores.
3. Keep replay-risk mitigation, bounded quotas, and bounded stores aligned with
   the regression and stage-boundary suites.
4. Stabilize the documented validation commands and rerun the stage-boundary
   smoke tests whenever status markers or hardening baselines move.

### Tasks
1. Add rate limits and byte budgets.
2. Broaden malformed/stale input rejection coverage and tests.
3. Finish integrating the current structured metrics and logs into the few
   remaining subsystem paths and aggregation boundaries.
4. Keep replay-risk mitigation and bounded local stores aligned with the
   current validation boundary.
5. Fill in missing validation commands.

### Done when
- malformed/stale record tests pass
- validation commands are stable
- repository is ready for simulation-focused work
