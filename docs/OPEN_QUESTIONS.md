# Open Questions

This file exists so Codex does not silently invent protocol details.
All currently known MVP ambiguities affecting the current Milestone 1-7
baseline are resolved below and should be reused as the conservative defaults.

## Resolved conservative choices for MVP

### 1. Canonical binary encoding for all records and messages

For MVP, all protocol bodies use:

- deterministic UTF-8 JSON bytes for the message/record body;
- fixed field order equal to Rust struct field declaration order;
- no pretty-printing;
- no maps in signed protocol bodies;
- arrays preserve declared order unless a field explicitly requires sorting;
- the transport frame header stays binary and big-endian.

The wire format is therefore:

- frame header: binary big-endian;
- frame body: canonical JSON UTF-8 bytes.

For byte arrays inside JSON bodies, MVP uses arrays of `u8` values, not hex strings.

Rationale:
- this matches the current identity, record, wire, and handshake vectors;
- it is the smallest conservative choice;
- it avoids inventing a second canonical body codec before Milestone 3+.

### 2. Exact transcript hash layout for the handshake

Handshake transcript hashing uses ordered transcript entries.

Each entry is encoded as:

`u32_be(label_len) || label_utf8 || u32_be(body_len) || body_bytes`

The transcript hash is:

`BLAKE3(entry_0 || entry_1 || ... || entry_n)`

Transcript labels are fixed ASCII strings.

MVP labels:

- `client_hello`
- `server_hello_unsigned`
- `server_hello`
- `client_finish_unsigned`

Coverage rules:

- server signature covers:
- `client_hello`
- `server_hello_unsigned`

- hello transcript for key derivation covers:
- `client_hello`
- `server_hello`

- client finish signature covers:
- `client_hello`
- `server_hello`
- `client_finish_unsigned`

### 3. Full key schedule details beyond MVP wrappers

MVP key schedule:

1. compute X25519 shared secret;
2. reject all-zero shared secret;
3. compute `hello_transcript_hash`;
4. run HKDF-SHA256 extract with:
- `salt = hello_transcript_hash`
- `ikm = x25519_shared_secret`
5. expand from the resulting PRK using:

- `overlay-mvp/client-to-server` -> 32 bytes
- `overlay-mvp/server-to-client` -> 32 bytes
- `overlay-mvp/client-finish-key` -> 32 bytes
- `overlay-mvp/client-finish-nonce` -> 12 bytes

No additional traffic keys are derived in MVP.

`ClientFinish.confirmation` is:

- plaintext: `hello_transcript_hash`
- AEAD: ChaCha20-Poly1305
- key: `client-finish-key`
- nonce: `client-finish-nonce`
- AAD: `overlay-mvp-client-finish`

### 4. Exact relay quota defaults for tiny/std/relay profiles

These are local defaults, not protocol guarantees.

#### tiny profile
- `relay_mode = false` by default
- if enabled:
- max concurrent relay tunnels: 2
- max intro requests per minute: 30
- max bytes relayed per peer per hour: 64 MiB
- max total relay bytes per hour: 256 MiB

#### standard profile
- `relay_mode = false` by default
- if enabled:
- max concurrent relay tunnels: 8
- max intro requests per minute: 120
- max bytes relayed per peer per hour: 256 MiB
- max total relay bytes per hour: 2 GiB

#### relay profile
- `relay_mode = true` by default
- max concurrent relay tunnels: 128
- max intro requests per minute: 2000
- max bytes relayed per peer per hour: 1 GiB
- max total relay bytes per hour: 64 GiB

When any quota is exceeded:
- reject new relay binds first;
- keep existing sessions until local policy closes them.

### 5. Concrete constants for path score weights and hysteresis

MVP route selection uses deterministic integer scoring.

Base score:

`score =`
- `8 * obs_rtt_ms`
- `+ 2 * est_rtt_ms`
- `+ 1 * jitter_ms`
- `+ (loss_ppm / 1000) * 25`
- `+ 25 * relay_hops`
- `+ 100 * censorship_risk_level`
- `- 20 * diversity_bonus`

Lower score is better.

EWMA defaults:
- RTT alpha: `0.2`
- loss alpha: `0.1`
- jitter alpha: `0.1`

Switch hysteresis defaults:
- minimum absolute improvement: `10 ms`
- minimum relative improvement: `15%`
- minimum dwell time on current path: `30 s`
- max path switches per minute: `2`

If thresholds disagree, require both absolute and relative improvement.

### 6. Concrete bootstrap response schema beyond the current skeleton

MVP logical bootstrap response schema:

- `version: u8`
- `generated_at_unix_s: u64`
- `expires_at_unix_s: u64`
- `network_params`
- `epoch_duration_s: u64`
- `presence_ttl_s: u64`
- `max_frame_body_len: u32`
- `handshake_version: u8`
- `peers: Vec<BootstrapPeer>`
- `bridge_hints: Vec<BridgeHint>`

`BootstrapPeer`:
- `node_id`
- `transport_classes`
- `capabilities`
- `dial_hints`
- `observed_role`

`BridgeHint`:
- `transport_class`
- `dial_hint`
- `capabilities`
- `expires_at_unix_s`

For MVP:
- providers may deliver this schema over HTTPS, DNS-derived data, or static config;
- bootstrap responses are advisory;
- a peer entry must not be trusted as sufficient on its own.
- `epoch_duration_s`, `presence_ttl_s`, and `max_frame_body_len` must be
  non-zero during local bootstrap-response validation;
- `max_frame_body_len` must not exceed the MVP frame-body limit.

### 7. Final encoding of transport classes and capabilities

For MVP, transport classes and capabilities are encoded as lowercase ASCII string enums in canonical JSON.

Allowed transport classes:
- `tcp`
- `quic`
- `ws`
- `relay`

Allowed capability strings:
- `relay-forward`
- `relay-intro`
- `rendezvous-helper`
- `bridge`
- `service-host`

Rules:
- arrays must be deduplicated;
- arrays must be sorted lexicographically before signing or hashing;
- unknown values must be rejected in signed protocol records;
- unknown values may be ignored only in unsigned local config.

### 8. Repository stage lock-in

For current work, treat the repository stage as:

- Milestone 0 complete;
- Milestone 1 foundations implemented, vectorized, and validated;
- Milestone 2 handshake surface implemented, vectorized, validated, and considered closed;
- Milestone 3 transport/session layer implemented, validated, and considered closed;
- Milestone 4 peer/bootstrap layer implemented, validated, and considered closed;
- Milestone 5 rendezvous/presence publish and exact lookup work is implemented
  and considered closed in
  `crates/overlay-core/src/rendezvous/mod.rs` with deterministic placement key
  derivation, bounded in-memory publish/lookup flows, canonical wire-body
  helpers, deterministic message vectors, freshness and epoch/sequence conflict
  handling, bounded lookup state, negative cache, and verified-signature
  handoff at the store boundary;
- Milestone 6 relay intro and fallback work is implemented in
  `crates/overlay-core/src/relay/mod.rs` with profile-based bounded relay quota
  defaults, an explicit local relay role model, canonical `ResolveIntro` /
  `IntroResponse` wire-body helpers with deterministic relay intro message
  vectors, intro/tunnel/byte quota enforcement, verified `IntroTicket` usage,
  direct-first/relay-second reachability planning, and relay fallback
  integration coverage. Milestone 6 is considered closed;
- Milestone 7 routing metrics and path switching work is implemented in
  `crates/overlay-core/src/routing/mod.rs` with deterministic path-score
  weights, integer EWMA observation updates, hysteresis-gated route selection,
  anti-flapping unit coverage, and routing stage-boundary integration coverage.
  Milestone 7 is considered closed;
- Milestone 8 service-layer work is implemented in
  `crates/overlay-core/src/service/mod.rs` with canonical service wire bodies,
  verified `ServiceRecord` registration, a bounded local service registry and
  open-session store, exact `app_id` resolution, `reachability_ref` binding
  checks, allow/deny local policy enforcement, and integration coverage.
  Milestone 8 is considered closed;
- Milestone 9 hardening and polish is implemented with bounded observability
  groundwork in `crates/overlay-core/src/metrics/mod.rs`, a validated top-level
  config baseline in `crates/overlay-core/src/config.rs` with explicit
  transport-buffer projection in `crates/overlay-core/src/transport/mod.rs`, a
  bounded handshake transcript replay cache in
  `crates/overlay-core/src/session/manager.rs`, explicit observability hooks in
  bootstrap provider fetch/validation, peer/bootstrap ingest, rendezvous
  publish/lookup, relay bind and rate-limit handling, routing probe/switch
  paths, service registry flows, and session event export, with malformed-input
  coverage explicitly exercising relay, routing, and service wire-body
  rejection paths;
- Milestone 10 minimal runtime is implemented in
  `crates/overlay-core/src/runtime.rs` and `crates/overlay-cli/src/main.rs`;
- Milestone 11 local devnet is implemented in `devnet/` and
  `crates/overlay-cli/src/devnet.rs`;
- Milestone 12 launch hardening is implemented with bounded cleanup, degraded
  bootstrap retry, runtime health snapshots, and the logical soak path;
- the current repository stage marker is `milestone-16-network-bootstrap`;
- Milestone 16 network bootstrap and multi-host devnet is now the current
  stage, with `docs/LAUNCH_CHECKLIST.md`, `docs/PILOT_RELEASE_TEMPLATE.md`,
  the documented green-path validation and launch sequence, the checked-in
  `overlay-cli bootstrap-serve` surface, host-style devnet layouts, and
  explicit pilot-only limitations;

That means:

- do not restart from Milestone 0;
- do not re-implement Milestones 1-4 from scratch;
- touch Milestones 1-4 only for regression fixes, spec mismatches,
  vector maintenance, or validation maintenance;
- keep status docs and prompts synchronized to this baseline as protocol logic evolves;
- lock missing conservative defaults here before inventing new wire or session behavior;
- Milestone 5 is closed and should be touched only for regressions,
  vectors, or spec mismatches;
- Milestone 6 is closed and should be touched only for regressions,
  vectors, or spec mismatches;
- Milestone 7 is closed and should be touched only for regressions,
  vectors, or spec mismatches;
- Milestone 8 is closed and should be touched only for regressions,
  vectors, or spec mismatches;
- Milestone 16 is the current stage and should stay limited to network
  bootstrap, multi-host devnet maintenance, regression fixes, validation
  maintenance, and documentation synchronization unless a task explicitly
  reopens scope;
- public bootstrap infrastructure, discovery expansion, and simulation-focused
  expansion remain out of scope for current work.

### 9. Final encoding of `supported_kex` and `supported_signatures`

For MVP, `NodeRecord.supported_kex` and `NodeRecord.supported_signatures`
are encoded as lowercase ASCII string enums in canonical JSON.

Allowed key exchange values:
- `x25519`

Allowed signature values:
- `ed25519`

Rules:
- arrays must be deduplicated;
- arrays must be sorted lexicographically before signing or hashing;
- unknown values must be rejected in signed protocol records;
- unknown values may be ignored only in unsigned local config outside the
  signed record path.

### 10. Final encoding of `ServiceRecord.auth_mode` and `IntroTicket.scope`

For MVP, these fields are also locked to lowercase ASCII string enums in
canonical JSON.

Allowed `ServiceRecord.auth_mode` values:
- `none`

Allowed `IntroTicket.scope` values:
- `relay-intro`

Rules:
- unknown values must be rejected in signed protocol records;
- do not invent additional auth or intro scope modes until later specs land.

### 11. Conservative lookup scope and freshness defaults for MVP

For MVP lookup behavior, keep the scope exact and freshness rules strict.

Rules:
- `LookupNode` operates on a full `node_id` only;
- no prefix scan, range scan, wildcard lookup, or open enumeration;
- expired records must not be returned as fresh lookup results;
- when multiple candidate `PresenceRecord` values exist, higher epoch wins, then
  higher sequence, and equal epoch plus sequence must be byte-identical to be
  treated as the same record.

### 12. Conservative direct-first reachability policy for MVP

For MVP session establishment and service reachability:

- prefer direct transport attempts first;
- use relay only as fallback when direct reachability is unavailable or fails;
- do not make any single relay mandatory for the connection policy;
- keep this as the local default for the current Milestone 6 relay baseline.

### 13. Conservative keepalive and timeout scaffolding for the Milestone 3 session skeleton

For the current Milestone 3 session skeleton, use explicit polled timers only.

There is no background scheduler yet.
The session layer evaluates deadlines only when `SessionManager::poll_timers(now_ms)`
is called.

Default local timing values:
- `open_timeout_ms = 10_000`
- `keepalive_interval_ms = 15_000`
- `idle_timeout_ms = 45_000`
- `degraded_timeout_ms = 30_000`
- `close_timeout_ms = 5_000`

Rules:
- `idle_timeout_ms` must be strictly greater than `keepalive_interval_ms`;
- entering `opening` schedules only the open deadline;
- entering `established` schedules keepalive and idle deadlines;
- entering `degraded` schedules keepalive and degraded deadlines;
- degraded recovery is explicit only; local activity alone does not auto-promote a
  degraded session back to `established`;
- `mark_recovered(...)` is the conservative local transition back to
  `established` and reschedules established keepalive/idle deadlines;
- entering `closing` schedules only the close deadline;
- `open`, `degraded`, and `close` timeout expiry closes the session;
- `idle` timeout expiry degrades an established session before closing it later;
- keepalive expiry emits a structured event only and does not send network traffic
  by itself in the current skeleton;
- observed local activity refreshes keepalive and liveness deadlines but does not
  invent new routing or transport behavior.

Handshake binding rule:
- when `mark_established_with_handshake` is used, the session manager stores the
  peer `node_id`, transcript hash, and derived session keys as local session
  context;
- structured session events may reference the peer `node_id`, but must not expose
  session keys.

### 14. Final encoding of `PresenceRecord.reachability_mode` and `intro_policy`

For MVP, these fields are also locked to lowercase ASCII string enums in
canonical JSON.

Allowed `PresenceRecord.reachability_mode` values:
- `direct`
- `hybrid`

Allowed `PresenceRecord.intro_policy` values:
- `allow`

Rules:
- unknown values must be rejected in signed protocol records;
- do not invent additional reachability or intro policy modes until later
  specs land.

### 15. Conservative runner boundary for the closed Milestone 3 session layer

The session layer still does not perform real network I/O directly.
Instead, the session/transport boundary is explicit:

- `SessionManager::drain_io_actions()` exposes queued session-originated actions;
- `SessionManager::handle_runner_input(...)` consumes runner-originated session inputs;
- `transport::TransportRunner` defines the placeholder adapter-side boundary for
  `begin_open`, `send_frame`, `begin_close`, `abort`, and `poll_event`.

Current action kinds:
- `begin_handshake`
- `send_keepalive`
- `start_close`
- `abort_transport`

Rules:
- `begin_open` queues `begin_handshake` for the selected placeholder transport;
- keepalive timer expiry queues `send_keepalive`;
- `begin_close` queues `start_close` as the conservative graceful-close attempt;
- failures and timeout-driven closes queue `abort_transport`;
- queued actions may include transport binding and peer `node_id`;
- queued actions must not expose derived session keys.

Current runner inputs:
- `frame_received`
- `handshake_succeeded`
- `transport_closed`
- `transport_failed`

Rules:
- handshake completion reaches the session manager only through a runner-delivered
  handshake outcome;
- session-frame activity refreshes liveness only after the session is established
  or degraded;
- placeholder transports may expose the runner contract while still returning
  unsupported operations until a real runner lands.

### 16. Bounded local stores for the closed Milestone 3 session layer

The session manager keeps explicit bounded local stores instead of unbounded logs.

Limits:
- `MAX_SESSION_EVENT_LOG_LEN = 64`
- `MAX_SESSION_IO_ACTION_QUEUE_LEN = 32`

Rules:
- when a store reaches its limit, the oldest entry is dropped first;
- these limits apply only to the local session-manager buffers, not to future
  network byte budgets;
- if later work needs different limits, change the docs and validation together.

### 17. Conservative bootstrap response details for the closed Milestone 4 layer

For the closed Milestone 4 bootstrap baseline, use the smallest explicit schema
that satisfies the advisory bootstrap response contract.

`BootstrapNetworkParams`:
- `network_id: String`

Allowed `BootstrapPeer.observed_role` values:
- `standard`
- `relay`

Rules:
- bootstrap schema version is `1`;
- `generated_at_unix_s` must not exceed `expires_at_unix_s`;
- bootstrap responses must be fresh when validated;
- `handshake_version` must match the current MVP handshake version;
- `max_frame_body_len` must not exceed the wire-layer MVP body limit;
- duplicate `BootstrapPeer.node_id` entries are rejected during local
  bootstrap-response validation instead of being merged implicitly;
- duplicate bridge hints with the same canonical `transport_class` and
  `dial_hint` are rejected during local bootstrap-response validation instead
  of being merged implicitly;
- peer and bridge transport classes use the same lowercase string enums as the
  record layer;
- bootstrap capabilities use the same lowercase string enums as the record layer;
- peer `dial_hints[]` and `bridge_hints[].dial_hint` are trimmed, deduplicated,
  and must remain non-empty;
- bootstrap responses are advisory only and do not, by themselves, establish trust.

### 18. Conservative peer-store defaults for the closed Milestone 4 layer

For the closed Milestone 4 peer baseline, keep the local peer store bounded and
rebalance deterministically.

`NeighborState` values:
- `candidate`
- `active`

Selection defaults:
- `max_neighbors = 16`
- `max_relay_neighbors = 4`
- `max_neighbors_per_transport = 8`

Rebalance phases:
1. reserve relay-capable peers first;
2. preserve transport diversity before filling more slots;
3. fill remaining slots using deterministic hash-ordered randomization;
4. leave overflow peers in `candidate`.

Rules:
- relay-capable peers are identified by `observed_role == relay`, relay
  capability strings, or relay transport support;
- do not collapse the active set to a single dominant transport class if valid
  alternatives exist;
- do not require runtime RNG for the Milestone 4 fill phase;
- keep bootstrap and peer management advisory and separate from Milestone 5
  presence publication and lookup.

### 19. Deterministic rendezvous placement key derivation for the Milestone 5 baseline

For the current Milestone 5 exact-lookup baseline, derive the local placement
key from the full `node_id` using a domain-separated BLAKE3 hash:

`placement_key = BLAKE3("overlay-mvp-rendezvous-placement" || node_id)`

Rules:
- derive one placement key from the full 32-byte `node_id`;
- do not truncate the `node_id` or placement key for prefix/range behavior;
- use the placement key only for local rendezvous addressing, not as a public
  replacement for `node_id`.

### 20. Conservative bounded rendezvous defaults for the Milestone 5 baseline

The current in-memory Milestone 5 store stays explicitly bounded.

Defaults:
- `max_published_records = 1024`
- `max_negative_cache_entries = 256`
- `negative_cache_ttl_s = 60`
- `max_lookup_budget = 8`
- `max_lookup_seen_helpers = 16`

Rules:
- clamp requested lookup budgets to the local maximum before a lookup starts;
- consume one budget unit per local lookup attempt;
- keep the seen-set as local helper state rather than an enumerable remote
  surface;
- evict the oldest entry first when a bounded published-record or negative-cache
  store is full;
- clear any negative-cache entry for a node when a fresher presence record is
  accepted for that node.
- `PublishAck.placement_key`, `LookupResult.placement_key`, and
  `LookupNotFound.placement_key` must equal the derived placement key for the
  corresponding `node_id`;
- `LookupResult.record.node_id` must match `LookupResult.node_id`.

### 21. Signature-verification handoff for the conservative Milestone 5 store

The current rendezvous store does not independently verify `PresenceRecord`
signatures because the local source of trusted node public keys lives outside
this module boundary.

Rules:
- call the publish path only with records whose signatures were already
  validated upstream;
- still enforce freshness plus epoch/sequence conflict handling locally inside
  the rendezvous store;
- do not treat this handoff as permission to accept unchecked signed records
  silently anywhere else in the node pipeline.

### 22. Conservative relay fallback planning defaults for the current Milestone 6 baseline

For the current Milestone 6 baseline, relay planning remains explicit and local.

Rules:
- verify `IntroTicket` signatures before relay planning begins;
- require a fresh `IntroTicket` whose `target_node_id` matches the requested
  node and whose `requester_binding` matches the local requester binding;
- build direct attempts first from non-`relay` transport classes on the target
  `PresenceRecord`;
- reject relay hints whose relay-dial transport class is `relay`; recursive
  relay-on-relay fallback remains out of scope for the current baseline;
- keep relay candidates as fallback only, sorted by higher `relay_score` first
  with deterministic tie-breaking;
- preserve secondary relay candidates instead of collapsing to one mandatory
  relay.

### 23. Conservative local relay role defaults for the current Milestone 6 baseline

For the current Milestone 6 baseline, the local relay role model stays minimal.

Rules:
- when `relay_mode` is disabled, all relay roles are disabled locally;
- when `relay_mode` is enabled for the current Milestone 6 baseline, enable only
  `forward` and `intro` roles by default;
- keep `rendezvous` and `bridge` roles disabled until a later milestone
  explicitly implements them;
- intro request handling requires the local `intro` role;
- relay tunnel binding and relayed-byte accounting require the local `forward`
  role.

### 24. Conservative relay intro message schema for the current Milestone 6 baseline

For the current Milestone 6 baseline, the relay intro wire surface stays small.

`ResolveIntro`:
- `relay_node_id`
- `intro_ticket`

`IntroResponse`:
- `relay_node_id`
- `target_node_id`
- `ticket_id`
- `status`

Rules:
- verify `ResolveIntro.intro_ticket` before local relay handling;
- use `IntroResponse.status` values `forwarded`, `rejected_relay_disabled`,
  `rejected_relay_mismatch`, `rejected_role_disabled`,
  `rejected_ticket_expired`, `rejected_requester_binding`, and
  `rejected_rate_limited`;
- keep relay intro message bodies on the same canonical JSON UTF-8 encoding
  rules and MVP frame-size limit as other current protocol bodies.

### 25. Conservative routing selector defaults for the closed Milestone 7 baseline

For the closed Milestone 7 baseline, route selection stays deterministic and local.

Rules:
- `PathMetrics.score()` uses the integer score formula locked in section 5,
  and lower score is better;
- `PathObservation` updates `obs_rtt_ms`, `loss_ppm`, and `jitter_ms` through
  integer EWMA rounding with the section 5 alpha defaults;
- when scores tie, choose the lower `path_id` deterministically;
- route switching requires both the absolute and relative improvement
  thresholds, the minimum dwell time, and the per-minute switch cap;
- if the current path disappears from the candidate set, switch immediately to
  the best remaining candidate and record that switch in local history.

### 26. Conservative path probe message schema and local defaults for the closed Milestone 7 baseline

For the closed Milestone 7 baseline, active probes stay small and bounded.

`PathProbe`:
- `path_id`
- `probe_id`
- `sent_at_unix_ms`

`PathProbeResult`:
- `path_id`
- `probe_id`

Defaults:
- `path_probe_interval_ms = 5_000`
- max in-flight probes per path: `4`
- loss window samples: `16`

Rules:
- issue at most one new probe for a path per local `path_probe_interval_ms`;
- derive RTT locally from the matching in-flight `PathProbe.sent_at_unix_ms`
  and the local receive time of `PathProbeResult`;
- derive loss locally from missing or expired probe results rather than from a
  field encoded in `PathProbeResult`;
- keep probe bookkeeping bounded per path and preserve deterministic `probe_id`
  assignment order;
- keep routing probe message bodies on the same canonical JSON UTF-8 encoding
  rules and MVP frame-size limit as other current protocol bodies.

### 27. Conservative service registry and open-session defaults for the closed Milestone 8 baseline

For the closed Milestone 8 baseline, service access stays exact, local, and bounded.

`GetServiceRecord`:
- `app_id`

`ServiceRecordResponse`:
- `app_id`
- `status`
- `record`

`OpenAppSession`:
- `app_id`
- `reachability_ref`

`OpenAppSessionResult`:
- `app_id`
- `status`
- `session_id`

Defaults:
- `max_registered_services = 256`
- `max_open_service_sessions = 1024`

Rules:
- `GetServiceRecord` operates on a full `app_id` only;
- no prefix scan, range scan, wildcard lookup, or global service enumeration;
- the local registry stores only `ServiceRecord` values that have already been
  signature-verified before `register_verified`;
- `ServiceRecord.policy` remains opaque signed bytes at this stage; local
  allow/deny access checks are enforced separately at the registry boundary;
- `ServiceRecordResponse.status` values are `found` and `not_found`;
- `ServiceRecordResponse.record` is present only when status is `found`, and
  its `app_id` must match the response `app_id`;
- `OpenAppSession` must echo the exact resolved
  `ServiceRecord.reachability_ref` or be rejected as
  `rejected_reachability_mismatch`;
- `OpenAppSessionResult.status` values are `opened`, `rejected_not_found`,
  `rejected_policy`, `rejected_reachability_mismatch`, and
  `rejected_session_limit`;
- `OpenAppSessionResult.session_id` is present only when status is `opened`;
- successful opens allocate monotonically increasing local `session_id` values
  from the bounded open-session store;
- keep service-layer message bodies on the same canonical JSON UTF-8 encoding
  rules and MVP frame-size limit as other current protocol bodies.

### 28. Conservative hardening scope for the current Milestone 9 baseline

For the current Milestone 9 baseline, hardening should stay local, bounded, and layered.

Rules:
- prefer extending existing bounded stores, quotas, and validation paths over
  adding new protocol scope;
- prefer explicit rejection of replay-risk, stale, malformed, or over-budget
  inputs over silent fallback;
- keep hardening changes within the existing identity, session, relay, routing,
  rendezvous, and service boundaries instead of collapsing layers together;
- when adding observability, use structured logs and metric names consistent
  with `spec/observability.md`;
- do not broaden into simulation-focused work until the Milestone 9 hardening
  checklist in `IMPLEMENT.md` is materially complete.

### 29. Conservative top-level node config defaults for the current Milestone 9 baseline

For the current Milestone 9 baseline, the top-level node config stays minimal
and projects into the existing bounded subsystem configs without inventing new
protocol behavior.

Rules:
- `bootstrap_sources[]` are stored as non-empty opaque local strings until a
  richer provider schema is explicitly specified;
- `log_level` uses lowercase local enums: `error`, `warn`, `info`, `debug`,
  `trace`;
- `max_total_neighbors` maps to `PeerStoreConfig.max_neighbors`, while
  `max_relay_neighbors` and `max_neighbors_per_transport` keep their existing
  local defaults capped to the total-neighbor limit;
- `max_presence_records` maps to
  `RendezvousConfig.max_published_records`;
- `max_service_records` maps to
  `ServiceConfig.max_registered_services`;
- `path_probe_interval_ms` maps directly to `PathProbeConfig`;
- `max_transport_buffer_bytes` maps directly to
  `TransportBufferConfig.max_buffer_bytes` and is enforced before
  `TransportPollEvent::FrameReceived` is converted into a session runner input;
- `relay_mode` maps directly to the existing `RelayConfig` role-toggle
  behavior;
- all other bounded subsystem knobs keep their current local defaults until the
  config spec expands further.

### 30. Conservative observability integration defaults for the current Milestone 9 baseline

For the current Milestone 9 baseline, observability integration stays explicit
at subsystem boundaries instead of introducing a new runtime orchestration layer.

Rules:
- keep the original subsystem methods available and add explicit
  `*_with_observability` wrappers when local metrics/log updates are needed;
- bootstrap providers may log validated fetch outcomes through an explicit
  `fetch_validated_response_with_observability` wrapper;
- `active_peers` is updated from peer-store bootstrap ingest outcomes;
- rendezvous publish/lookup wrappers update publish and lookup counters plus
  structured logs;
- relay wrappers update relay-bind counters and rate-limited-drop counters only
  on explicit rate-limit outcomes;
- routing wrappers update path-switch counters and probe RTT/loss samples from
  completed or lost probes;
- service wrappers log register/resolve/open/close outcomes without inventing
  new service-layer counters;
- session observability stays as explicit event-export helpers over
  `SessionEvent` values instead of implicit state-machine side effects;
- `established_sessions` remains a caller-managed gauge until broader session
  aggregation lands, but callers may now sync it explicitly from session
  states through the session-manager helper;
- the explicit established-session gauge counts `Established` and `Degraded`
  sessions, and excludes `Idle`, `Opening`, `Closing`, and `Closed`.

### 31. Conservative replay-cache defaults for the current Milestone 9 baseline

For the current Milestone 9 baseline, session replay-risk mitigation stays
local, bounded, and tied to handshake transcript outcomes instead of
introducing new protocol messages.

Rules:
- track successful handshake outcomes by `transcript_hash` in a bounded local
  replay cache keyed to the current node process;
- default replay-cache limits are `max_entries = 1024` and
  `replay_window_ms = 300000`;
- reject a repeated `transcript_hash` seen within the replay window as
  `ReplayCacheError::ReplayDetected`;
- prune expired entries before duplicate checks and evict the oldest surviving
  entry when the bounded cache is full;
- keep replay-cache enforcement explicit at the session-manager runner boundary
  through the existing replay-cache wrapper instead of adding implicit
  cross-layer state.

### 32. Conservative runtime config and node-key loading defaults for Milestone 10

For the minimal Milestone 10 runtime:

- runtime config files use UTF-8 JSON matching `OverlayConfig`;
- relative `node_key_path` values resolve relative to the config file
  directory;
- node key files may be either:
- raw 32-byte Ed25519 seed bytes; or
- 64 hex characters representing the same 32-byte seed.

Rationale:
- this stays aligned with the existing serde-based config surface;
- it avoids inventing PEM or keystore semantics that are not specified yet;
- it keeps key loading explicit and portable.

### 33. Conservative runtime bootstrap source handling for Milestone 10

Until network-backed runtime providers land:

- startup bootstrap accepts local bootstrap sources expressed as:
- `file:<path>`; or
- direct paths ending in `.json`;
- relative bootstrap paths resolve relative to the config file directory;
- unsupported bootstrap source strings are treated as unavailable local inputs,
  logged through observability, and do not abort startup on their own.

Rationale:
- the bootstrap provider abstraction already exists;
- local-file bootstrap is the smallest conservative runtime integration;
- degraded startup is preferable to inventing placeholder network fetch
  behavior.

### 34. Conservative presence refresh defaults for Milestone 10 runtime ticks

For the minimal long-running runtime:

- presence refresh runs only when an already verified local `PresenceRecord`
  has been installed into the runtime context;
- the runtime does not synthesize signed presence records from config alone;
- refresh may roll `expires_at_unix_s` forward to `now + presence_ttl_s` and
  re-sign the installed local record template with a strictly higher
  `sequence`, so long-running local runtime/devnet operation does not republish
  already-expired presence state;
- refresh is scheduled at half of `presence_ttl_s`, with a minimum interval of
  one second.

Rationale:
- current top-level config does not specify enough endpoint material to build a
  fresh signed presence record on its own;
- half-TTL refresh is the smallest conservative choice that refreshes before
  expiry without changing rendezvous semantics.

### 35. Conservative local-devnet orchestration defaults for Milestone 11

For the minimal Milestone 11 local devnet:

- sample nodes are started from on-disk `OverlayConfig` JSON files and local
  bootstrap seed files only;
- the smoke flow may drive multiple `NodeRuntime` instances in-process instead
  of requiring real network listeners or container orchestration;
- the session step should use the existing placeholder transport boundary with a
  real handshake outcome, not a fabricated established-session state;
- verified presence records and service requests may be handed to peer runtimes
  in-process after signature verification so the smoke flow exercises publish,
  exact lookup, service open, and relay fallback without inventing new runtime
  networking surfaces.

Rationale:
- this stays aligned with the existing runtime and subsystem boundaries;
- it keeps the devnet runnable on one machine without pretending Milestone 10
  already has full socket-level transport plumbing;
- it exposes the local-only assumption explicitly instead of hiding it in test
  harness code.

### 36. Conservative runtime launch-hardening defaults for Milestone 12

For the current local-runtime/devnet hardening slice:

- degraded runtime bootstrap retry stays local-file/provider based and runs only
  from the existing runtime tick loop;
- the retry interval is derived conservatively from existing config instead of a
  new top-level knob:
  `max(5000 ms, path_probe_interval_ms, presence_ttl_s / 4)`;
- stale service-open sessions are pruned after `presence_ttl_s`;
- stale relay tunnels are pruned after `presence_ttl_s`;
- expired path probes are treated as local loss after
  `3 * path_probe_interval_ms`, removed from the in-flight tracker, and fed back
  through the existing loss-observation path;
- operator health/status stays as local JSON snapshots emitted by the CLI/runtime
  boundary and reports existing bounded counts, observability counters, cleanup
  totals, relay usage, and effective local resource limits;
- do not add a runtime control socket, external metrics backend, persistence
  layer, or listener-specific accept policy until real listener surfaces are
  explicitly specified.

Rationale:
- this keeps Milestone 12 focused on launch resilience for the current local
  runtime instead of inventing a new deployment/control-plane architecture;
- it reuses existing bounded config surfaces and subsystem limits;
- it gives node operators enough local status to run the devnet without
  broadening protocol scope.

## Rule

If a task requires an area still not fully specified:
- choose the smallest conservative implementation;
- document the choice in the final report;
- update this file if a new gap is discovered.
