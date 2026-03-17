# IMPLEMENT.md

This file is the execution plan for Codex.

## Current repository stage

The repository has a closed Milestone 1-12 baseline, a landed Milestone 14
pilot launch gate, a landed Milestone 16 network-bootstrap stage, a landed
Milestone 17 operator-runtime stage, a landed Milestone 18 real-pilot stage,
and the current Milestone 20 regular-distributed-use-closure stage.
The current repository stage marker is
`milestone-20-regular-distributed-use-closure`.

## Current regular-distributed-use green path

Treat the current stage as having one localhost sign-off flow:

1. run the applicable commands in `VALIDATION.md`;
2. run `./devnet/run-launch-gate.sh`;
3. run `./devnet/run-distributed-pilot-checklist.sh` on the same commit;
4. use `docs/PILOT_RUNBOOK.md` for the separate-host evidence run.

`./devnet/run-pilot-checklist.sh` remains a retained Milestone 18 localhost
rehearsal only. It is not the current sign-off path.

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
- Milestone 9 hardening and polish is implemented with initial bounded
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
  helper while keeping aggregation caller-invoked, and the current validation
  boundary is the existing regression suites,
  stage-boundary integration tests, and Milestone 9 unit coverage in
  `bootstrap::tests`, `config::tests`, `metrics::tests`, `peer::tests`,
  `rendezvous::tests`, `relay::tests`, `routing::tests`, `service::tests`, and
  `session::manager::tests`, plus `transport::tests`.
- Milestone 10 minimal runtime work is also implemented in
  `crates/overlay-core/src/runtime.rs` and `crates/overlay-cli/src/main.rs`,
  with config-path loading, local bootstrap-file startup, and a bounded
  runtime tick loop exposed as `overlay-cli run`.
- Milestone 11 local-devnet work is now implemented in `devnet/` and
  `crates/overlay-cli/src/devnet.rs`, with four sample node configs,
  deterministic key material, local bootstrap seed files, a reproducible
  smoke flow, and one documented relay-fallback path. This remains a
  local-only orchestration layer on top of the existing runtime and subsystem
  boundaries; it did not advance the protocol stage marker before Milestone 14.
- Milestone 12 launch-hardening work is now implemented in
  `crates/overlay-core/src/runtime.rs`, `crates/overlay-cli/src/main.rs`, and
  `crates/overlay-cli/src/devnet.rs`, with bounded stale-state cleanup for the
  replay cache, rendezvous expiry, service-open sessions, relay tunnels, and
  expired path probes, a conservative degraded-runtime bootstrap retry policy,
  runtime health/status snapshots plus `overlay-cli run --status-every ...`
  dumps, and a logical long-run devnet soak path exposed through
  `overlay-cli smoke --soak-seconds ...` / `devnet/run-soak.sh`. This is still
  runtime/devnet hardening on top of the current local-only boundary and did
  not advance the protocol stage marker before Milestone 14.
- Milestone 14 launch-gate and pilot-tag work is now implemented with
  `docs/LAUNCH_CHECKLIST.md`, `docs/PILOT_RELEASE_TEMPLATE.md`,
  `devnet/run-launch-gate.sh`, `devnet/run-restart-smoke.sh`, the documented
  green-path validation and launch sequence in `docs/RUNBOOK.md` /
  `docs/DEVNET.md`, a frozen current MVP launch surface, and explicit pilot-only
  limitations. The repository stage marker now advances to
  `milestone-14-launch-gate`.
- Milestone 16 network bootstrap and multi-host devnet work is now implemented
  in `crates/overlay-core/src/runtime.rs`, `crates/overlay-cli/src/main.rs`,
  `crates/overlay-cli/src/bootstrap_server.rs`, and `crates/overlay-cli/src/devnet.rs`,
  with minimal static `http://` bootstrap fetch, a bounded static bootstrap
  seed server command, host-style devnet configs under `devnet/hosts/`,
  distributed and multi-host smoke scripts, and the current validation/docs
  updates for that surface. The repository stage marker now advances to
  `milestone-16-network-bootstrap`.
- Milestone 17 operator-grade runtime hardening is now implemented in
  `crates/overlay-cli/src/main.rs`, `crates/overlay-cli/src/operator_state.rs`,
  `crates/overlay-cli/src/signal.rs`, `crates/overlay-cli/src/bootstrap_server.rs`,
  and `crates/overlay-core/src/config.rs`, with signal-aware graceful
  shutdown, restart-safe operator lock/status state under `.overlay-runtime/`,
  `overlay-cli status`, stricter startup/config validation, an upgraded
  restart smoke, and the bounded soak added to the current launch gate. The
  repository stage marker advanced to `milestone-17-operator-runtime`.
- Milestone 18 real pilot support is now implemented in
  `crates/overlay-cli/src/devnet.rs`, `crates/overlay-cli/src/main.rs`,
  `devnet/pilot/`, `devnet/run-pilot-checklist.sh`,
  `docs/PILOT_RUNBOOK.md`, and `docs/PILOT_REPORT_TEMPLATE.md`, with a
  dedicated pilot topology/config pack, smoke fault rehearsals for `node-c`
  down and relay unavailable, lookup-latency plus relay-usage reporting in the
  smoke output, a bootstrap-seed-unavailable checklist path, and synchronized
  current-stage docs. The repository stage marker now advances to
  `milestone-18-real-pilot`.
- Milestone 19 pilot-closure support is now implemented in
  `crates/overlay-cli/src/main.rs`, `crates/overlay-cli/src/operator_client.rs`,
  `crates/overlay-core/src/runtime.rs`, `crates/overlay-core/src/config.rs`,
  `devnet/run-multihost-smoke.sh`, `devnet/run-distributed-pilot-checklist.sh`,
  and the current pilot docs, with bounded operator commands for networked
  `publish`, `lookup`, `open-service`, and `relay-intro` flows over real
  runtime sessions, `overlay-cli run --service` local service registration, a
  second relay-capable pilot path, conservative `http://...#sha256=<pin>`
  bootstrap-artifact integrity checks, the expanded pilot fault matrix, and
  synchronized post-pilot-closure docs. The repository stage marker now
  advances to `milestone-19-pilot-closure`.
- Milestone 20 regular-distributed-use-closure support is now implemented in
  `crates/overlay-core/src/runtime.rs`, `crates/overlay-cli/src/main.rs`,
  `devnet/run-multihost-smoke.sh`, `devnet/run-distributed-pilot-checklist.sh`,
  `docs/PILOT_RUNBOOK.md`, `docs/LAUNCH_CHECKLIST.md`, and the current status
  docs, with per-source bootstrap diagnostics on the runtime status surface,
  preferred retry/fallback ordering across configured bootstrap sources,
  localhost proof for unavailable/integrity/stale/empty bootstrap-source
  outcomes, stronger relay-bind evidence across node-down, primary-relay-down,
  and service-restart scenarios in the checked-in two-relay topology, and
  reproducible `--evidence-dir` support for the distributed smoke and pilot
  checklist. The repository stage marker now advances to
  `milestone-20-regular-distributed-use-closure`.

Treat Milestones 0-8 as a closed baseline. Prefer regression fixes,
spec-conformance fixes, vector maintenance, and validation maintenance there
over refactoring the already present work.

## Recommended next Codex task

Use `prompts/codex-milestone-20.md` as the recommended next-task prompt for the
current `milestone-20-regular-distributed-use-closure` stage and keep work
conservative from the current regular-distributed-use boundary:

1. preserve the current launch surface documented in
   `docs/LAUNCH_CHECKLIST.md`, `docs/RUNBOOK.md`, `docs/DEVNET.md`,
   `docs/PILOT_RUNBOOK.md`, and `devnet/pilot/README.md`;
2. keep Milestones 1-12 limited to regression fixes, launch-maintenance
   updates, vector maintenance, or conservative spec-conformance fixes;
3. prefer pilot execution support, validation maintenance, documentation sync,
   and operator-surface hardening over feature expansion;
4. rerun the documented launch gate and distributed pilot checklist whenever
   `REPOSITORY_STAGE`, `README.md`, `HANDOFF.md`, `IMPLEMENT.md`,
   `VALIDATION.md`, launch docs, pilot docs, or current-stage scripts change;
5. keep public bootstrap infrastructure, protocol redesign, and scope expansion
   out of work unless explicitly requested.

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

Status: closed in this repository; use only for regression fixes, validation
maintenance, or conservative hardening repairs.

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

---

## Milestone 14 — launch gate and pilot tag

Status: landed and considered closed for the current baseline.

### Goal
Freeze a reproducible pilot launch gate without claiming public-production
readiness.

### Tasks
1. Add a reproducible launch checklist.
2. Add a pilot release note and tag template.
3. Freeze the current launchable MVP surface.
4. Document the green-path validation and launch sequence.
5. Keep status docs, prompts, and launch-facing docs synchronized to the new
   stage marker.

### Important constraints
- Do not add major new features in this milestone.
- Do not redesign protocol layers in this milestone.
- Do not describe the result as GA or public hostile-environment ready.

### Done when
- the launch checklist and pilot release template exist
- the documented launch gate has one reproducible command order
- the current network can be raised and checked through the documented green
  path
- status docs and prompts report the same Milestone 14 stage marker

---

## Milestone 16 — network bootstrap and multi-host devnet

Status: landed and considered closed for the current baseline.

### Goal
Move the devnet from local-file-only bootstrap to a minimal network-reachable
bootstrap flow and add a reproducible host-style devnet validation path.

### Tasks
1. Add minimal network bootstrap fetch support without redesigning bootstrap
   message semantics.
2. Add a bounded static bootstrap seed server command for devnet or lab use.
3. Add host-style multi-host config examples with real dial hints.
4. Add distributed and multi-host smoke scripts.
5. Document the host-to-host bootstrap runbook and current limits.

### Important constraints
- Keep bootstrap transport minimal and static.
- Do not add broad public bootstrap-provider infrastructure.
- Do not add global discovery or anonymity features.
- Keep publish, lookup, service-open, and relay-fallback proof paths explicit
  about any remaining smoke-harness coordination.

### Done when
- nodes can bootstrap from static `http://` seed URLs;
- the repo includes host-style localhost and example multi-host configs;
- the distributed smoke proves network bootstrap plus real TCP session
  establishment;
- the multi-host smoke proves bootstrap, publish, lookup, service open, and
  relay fallback against the host-style devnet layout;
- status docs and prompts report the same Milestone 16 stage marker.

---

## Milestone 17 — operator-grade runtime hardening

Status: landed and considered closed for the current baseline.

### Goal
Make the current pilot runtime predictable under service-style operation
without changing the protocol surface.

### Tasks
1. Add signal-aware graceful shutdown for `overlay-cli run`.
2. Add restart-safe operator metadata and bounded stale-lock recovery.
3. Add an operator-facing status surface that survives process restarts.
4. Tighten startup/config validation for operator use.
5. Update the launch gate, restart smoke, soak path, and runbooks.

### Important constraints
- Do not add a database or persist protocol-layer state.
- Do not redesign protocol layers or claim public-production readiness.
- Keep persisted state bounded to operator metadata and last-known health.

### Done when
- `overlay-cli run` handles `SIGINT` and `SIGTERM` through the existing runtime
  shutdown path;
- `overlay-cli status --config ...` exposes the last known health plus
  lifecycle state from `.overlay-runtime/`;
- the restart smoke proves signal-driven clean shutdown and restart-safe reuse
  of the same config;
- the launch gate includes the bounded soak and status docs report the same
  Milestone 17 stage marker.

---

## Milestone 18 — real pilot network on separate hosts

Status: landed baseline.

### Goal
Prepare and rehearse the first real pilot network stage on separate hosts
without widening the protocol surface.

### Tasks
1. Add a dedicated pilot topology/config pack for localhost rehearsal and
   separate-host copy-and-edit use.
2. Add a pilot runbook for 3-5 host execution.
3. Add a pilot checklist script that captures baseline, restart, and fault
   rehearsal evidence.
4. Add pilot report structure for success/failure, lookup latency, relay usage,
   restart outcomes, and remaining blockers.
5. Keep the launch-gate docs and status markers synchronized to the new stage.

### Important constraints
- Do not claim hostile-environment or public-production readiness.
- Do not add public bootstrap infrastructure or rollout automation.
- Do not redesign protocol layers or invent a distributed operator control
  plane for publish, lookup, service open, or relay intro.
- Keep full-flow pilot proof honest about any remaining smoke-harness
  coordination.

### Done when
- the repo includes a pilot topology pack and runbook for a 3-5 host pilot;
- the pilot checklist captures baseline, restart, and documented fault
  scenarios;
- the current smoke output reports lookup latency and relay usage for pilot
  reporting;
- validation and status docs report the same Milestone 18 stage marker.

## Milestone 19 — pilot-closure blockers after the first real pilot

Status: current repository stage.

### Goal
Close the pilot blockers exposed by the first real pilot without widening the
scope into hostile-environment or public-Internet rollout work.

### Tasks
1. Add minimal distributed operator commands or equivalent bounded surfaces for
   `publish`, `lookup`, `open-service`, and `relay-intro` on top of the landed
   runtime.
2. Extend the pilot topology/config pack to 3-5 hosts with two relay-capable
   fallback paths.
3. Add the distributed pilot-closure checklist with the documented node-down,
   primary-relay-down, bootstrap-seed-down, service-restart, and tampered
   bootstrap-artifact scenarios.
4. Replace the plain-HTTP bootstrap blocker with conservative integrity checks
   for static bootstrap artifacts while keeping the existing pilot-only static
   bootstrap model.
5. Synchronize launch, pilot, validation, and status docs to the new
   post-pilot-closure stage marker.

### Important constraints
- Do not claim hostile-environment or public-production readiness.
- Do not add public bootstrap infrastructure, orchestration, or rollout
  automation.
- Do not redesign the protocol or collapse subsystem layering.
- Keep the operator surfaces explicit, bounded, and honest about their
  point-to-point, operator-directed nature.

### Done when
- the repo exposes minimal networked operator flows for `publish`, `lookup`,
  `open-service`, and relay fallback without local in-process injection;
- the pilot pack documents two relay-capable paths and the expanded fault
  matrix;
- the current checklist proves the closure work through
  `./devnet/run-distributed-pilot-checklist.sh`;
- validation and status docs report the same Milestone 19 stage marker and
  current limitations.

## Milestone 20 — regular distributed use closure

Status: current repository stage.

### Goal
Reduce the remaining gap between the landed distributed pilot path and a
network that can be used regularly by first users in the current trusted,
pilot-only operating model.

### Tasks
1. Keep the current launch surface and distributed operator commands, but make
   distributed bootstrap fallback and failure modes explicit on the runtime
   status surface.
2. Improve retry/fallback behavior between configured bootstrap sources
   without redesigning bootstrap semantics.
3. Expand the localhost distributed checklist and host-style smoke to preserve
   reproducible evidence bundles and cover unavailable, integrity-mismatch,
   stale-artifact, and empty-peer-set bootstrap cases.
4. Strengthen relay-fallback proof for the checked-in two-relay pilot pack
   across node-down, primary-relay-down, and service-host-restart scenarios.
5. Synchronize launch, pilot, validation, and stage-marker docs to the new
   regular-distributed-use closure stage.

### Important constraints
- Do not claim hostile-environment or public-production readiness.
- Do not add public bootstrap infrastructure, new transports, or a general
  distributed control plane.
- Do not redesign the wire format, handshake semantics, relay quotas, or path
  score constants.
- Keep the operator surfaces explicit, bounded, and honest about their
  point-to-point, operator-directed nature.

### Done when
- runtime status exposes per-source bootstrap outcomes and operators can
  distinguish unavailable, integrity-mismatch, stale, empty-peer-set, and
  accepted cases without reading source code;
- the current distributed checklist proves fallback behavior for the checked-in
  multi-source bootstrap configs and records the stronger relay-bind evidence;
- docs and report templates explain the exact green path, evidence collection
  order, and remaining limitations without source-code spelunking;
- validation and status docs report the same Milestone 20 stage marker and
  narrower remaining blockers.
