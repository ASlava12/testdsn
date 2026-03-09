# Wire Protocol

## Frame header
- `version: u8`
- `msg_type: u16`
- `flags: u16`
- `body_len: u32`
- `correlation_id: u64`

## Encoding
- big-endian integers
- max frame size for MVP: 64 KiB

## Message catalog
### Session
- `ClientHello`
- `ServerHello`
- `ClientFinish`
- `Ping`
- `Pong`
- `Close`

### Bootstrap
- `BootstrapRequest`
- `BootstrapResponse`

### Presence / lookup
- `PublishPresence`
  - `record`
- `PublishAck`
  - `node_id`
  - `placement_key`
  - `disposition`
  - `accepted_epoch`
  - `accepted_sequence`
- `LookupNode`
  - `node_id`
- `LookupResult`
  - `node_id`
  - `placement_key`
  - `record`
  - `remaining_budget`
- `LookupNotFound`
  - `node_id`
  - `placement_key`
  - `reason`
  - `remaining_budget`
- `ResolveIntro`
- `IntroResponse`

Presence / lookup rules for the current Milestone 5 baseline:
- `PublishPresence.record` carries a full `PresenceRecord`;
- `LookupNode` stays exact-by-`node_id` only;
- `LookupResult.record` carries the fresh winning `PresenceRecord`;
- `PublishAck.disposition` values are `stored`, `replaced`, `duplicate`, `stale`;
- `LookupNotFound.reason` values are `missing`, `negative_cache_hit`, `budget_exhausted`;
- these message bodies use the same canonical JSON UTF-8 body rules as the rest
  of the MVP body encoding;
- a presence/lookup body must still fit within the MVP frame body limit.

### Routing
- `PathProbe`
- `PathProbeResult`

### Service
- `GetServiceRecord`
- `ServiceRecordResponse`
- `OpenAppSession`
- `OpenAppSessionResult`
