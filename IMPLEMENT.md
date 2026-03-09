# IMPLEMENT.md

This file is the execution plan for Codex.

## Current repository stage

The repository has a closed Milestone 1-2 baseline plus a minimal Milestone 3
transport/session skeleton.

- Milestone 0 bootstrap is complete.
- Milestone 1 foundations are implemented, vectorized, and validated in
  `crates/overlay-core/src/identity.rs`, `crates/overlay-core/src/records/mod.rs`,
  and `crates/overlay-core/src/wire/mod.rs`.
- Milestone 2 crypto wrappers and handshake surface are implemented, vectorized,
  and validated in `crates/overlay-core/src/crypto/*` and
  `crates/overlay-core/src/session/handshake.rs`.
- Milestone 2 is considered closed.
- Milestone 3 transport abstraction and session-manager work has started at a
  minimal compileable skeleton level in `crates/overlay-core/src/transport/mod.rs`
  and `crates/overlay-core/src/session/manager.rs`, including explicit polled
  keepalive/timeout scaffolding, handshake-bound session context, and a queued
  I/O-action surface for future session runners, plus explicit degraded-session
  recovery back to `established`.
- Milestone 4 and later are still placeholder modules and stage-boundary smoke tests.

Treat Milestones 0-2 as a closed baseline. Prefer regression fixes,
spec-conformance fixes, vector maintenance, and validation maintenance there
over refactoring the already present work.

## Recommended next Codex task

Before more Milestone 3 feature work, align the repository baseline:

1. sync status docs and prompts to the current repository state;
2. lock down conservative MVP defaults in `docs/OPEN_QUESTIONS.md`;
3. finish any remaining Milestone 1 fixture or validation gaps;
4. validate or complete remaining Milestone 2 handshake-surface gaps;
5. only then resume Milestone 3 transport/session work without adding Milestone 4+ behavior.

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

Status: in progress. A minimal compileable skeleton exists, but baseline
docs/vectors/validation should stay aligned before additional Milestone 3
feature work lands.

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

### Important constraints
- Keep transport-specific logic behind the trait.
- Do not implement full real QUIC/WS protocol behavior yet unless required by the task.

### Done when
- session manager compiles
- session state machine matches `spec/state-machines.md`
- session tests cover open/close/error/degraded/recovery transitions and timer scaffolding

---

## Milestone 4 — peer manager and bootstrap

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

### Done when
- publish/lookup integration smoke tests pass
- conflict resolution tests pass
- lookup terminates within configured budget

---

## Milestone 6 — relay intro and fallback connectivity

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

### Done when
- relay fallback integration test passes
- expired/invalid tickets are rejected
- relay quotas are enforced locally

---

## Milestone 7 — routing metrics and path switching

### Goal
Add path quality measurement and stable route selection.

### Tasks
1. Implement `PathMetric`.
2. Implement active probes and EWMA updates.
3. Implement path scoring.
4. Implement hysteresis and switch limits.
5. Add tests for anti-flapping behavior.

### Important constraints
- Do not switch on tiny metric changes.
- Keep route selection deterministic given the same inputs.

### Done when
- path score tests pass
- switching tests show no oscillation under small jitter

---

## Milestone 8 — service layer

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

### Goal
Close the highest-risk gaps before larger-scale simulation.

### Tasks
1. Add rate limits and byte budgets.
2. Add replay cache.
3. Add more structured metrics and logs.
4. Add stale/malformed record tests.
5. Fill in missing validation commands.

### Done when
- malformed/stale record tests pass
- validation commands are stable
- repository is ready for simulation-focused work
