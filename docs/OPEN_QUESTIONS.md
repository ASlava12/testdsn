# Open Questions

This file exists so Codex does not silently invent protocol details.
All currently known MVP ambiguities affecting the current Milestone 1-6
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
- Milestone 6 relay intro and fallback work started in
  `crates/overlay-core/src/relay/mod.rs` with profile-based bounded relay quota
  defaults, local intro/tunnel/byte quota enforcement, verified `IntroTicket`
  usage, direct-first/relay-second reachability planning, and relay fallback
  integration coverage;
- Milestone 7+ not started beyond placeholders and remaining stage-boundary smoke tests.

That means:

- do not restart from Milestone 0;
- do not re-implement Milestones 1-4 from scratch;
- touch Milestones 1-4 only for regression fixes, spec mismatches,
  vector maintenance, or validation maintenance;
- keep status docs and prompts synchronized to this baseline as protocol logic evolves;
- lock missing conservative defaults here before inventing new wire or session behavior;
- Milestone 5 is closed and should be touched only for regressions,
  vectors, or spec mismatches;
- Milestone 6 is active in code and remains the current feature stage;
- Milestone 7+ remains out of scope for current work.

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
- keep relay candidates as fallback only, sorted by higher `relay_score` first
  with deterministic tie-breaking;
- preserve secondary relay candidates instead of collapsing to one mandatory
  relay.

## Rule

If a task requires an area still not fully specified:
- choose the smallest conservative implementation;
- document the choice in the final report;
- update this file if a new gap is discovered.
