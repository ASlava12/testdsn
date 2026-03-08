# Open Questions

This file exists so Codex does not silently invent protocol details.
All currently known MVP ambiguities affecting Milestones 1 and 2 are resolved
below and should be reused as the conservative defaults.

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
- this matches the current handshake vectors;
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
- Milestone 1 foundations implemented;
- Milestone 2 handshake surface implemented;
- Milestone 3+ not started beyond placeholders.

That means:

- do not restart from Milestone 0;
- do not re-implement Milestone 1 or 2 from scratch;
- finish missing artifacts, vectors, and validation for the existing
  Milestone 1-2 baseline before starting Milestone 3.

## Rule

If a task requires an area still not fully specified:
- choose the smallest conservative implementation;
- document the choice in the final report;
- update this file if a new gap is discovered.
