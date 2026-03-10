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
  - `relay_node_id`
  - `intro_ticket`
- `IntroResponse`
  - `relay_node_id`
  - `target_node_id`
  - `ticket_id`
  - `status`

Presence / lookup rules for the current Milestone 5 baseline:
- `PublishPresence.record` carries a full `PresenceRecord`;
- `LookupNode` stays exact-by-`node_id` only;
- `LookupResult.record` carries the fresh winning `PresenceRecord`;
- `PublishAck.placement_key` must equal the derived placement key for
  `PublishAck.node_id`;
- `LookupResult.record.node_id` must match `LookupResult.node_id`, and
  `LookupResult.placement_key` must equal the derived placement key for that
  `node_id`;
- `LookupNotFound.placement_key` must equal the derived placement key for
  `LookupNotFound.node_id`;
- `PublishAck.disposition` values are `stored`, `replaced`, `duplicate`, `stale`;
- `LookupNotFound.reason` values are `missing`, `negative_cache_hit`, `budget_exhausted`;
- these message bodies use the same canonical JSON UTF-8 body rules as the rest
  of the MVP body encoding;
- a presence/lookup body must still fit within the MVP frame body limit.

### Routing
- `PathProbe`
  - `path_id`
  - `probe_id`
  - `sent_at_unix_ms`
- `PathProbeResult`
  - `path_id`
  - `probe_id`

Routing probe rules for the current Milestone 7 baseline:
- `PathProbeResult` acknowledges an in-flight `PathProbe` by `path_id` and
  `probe_id`;
- the local sender computes RTT from the matching in-flight probe timestamp and
  its own receive time for `PathProbeResult`;
- loss is derived locally from missing or expired probe results rather than
  being encoded in `PathProbeResult`;
- these message bodies use the same canonical JSON UTF-8 body rules as the rest
  of the MVP body encoding;
- a routing probe body must still fit within the MVP frame body limit.

Relay intro rules for the current Milestone 6 baseline:
- `ResolveIntro.intro_ticket` carries a full `IntroTicket`;
- `ResolveIntro` must be verified against the target node signing key before the
  local relay handler uses it;
- `IntroResponse.status` values are `forwarded`, `rejected_relay_disabled`,
  `rejected_relay_mismatch`, `rejected_role_disabled`,
  `rejected_ticket_expired`, `rejected_requester_binding`,
  `rejected_rate_limited`;
- these message bodies use the same canonical JSON UTF-8 body rules as the rest
  of the MVP body encoding;
- a relay-intro body must still fit within the MVP frame body limit.

### Service
- `GetServiceRecord`
  - `app_id`
- `ServiceRecordResponse`
  - `app_id`
  - `status`
  - `record`
- `OpenAppSession`
  - `app_id`
  - `reachability_ref`
- `OpenAppSessionResult`
  - `app_id`
  - `status`
  - `session_id`

Service rules for the current Milestone 8 baseline:
- `GetServiceRecord` stays exact-by-`app_id` only;
- `ServiceRecordResponse.status` values are `found` and `not_found`;
- `ServiceRecordResponse.record` carries the exact matching `ServiceRecord`
  only when status is `found`;
- `OpenAppSession.reachability_ref` echoes the resolved
  `ServiceRecord.reachability_ref` for the target service binding;
- `OpenAppSessionResult.status` values are `opened`, `rejected_not_found`,
  `rejected_policy`, `rejected_reachability_mismatch`, and
  `rejected_session_limit`;
- `OpenAppSessionResult.session_id` is present only when status is `opened`;
- these message bodies use the same canonical JSON UTF-8 body rules as the rest
  of the MVP body encoding;
- a service-layer body must still fit within the MVP frame body limit.
