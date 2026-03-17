# Troubleshooting

Start every investigation with a bounded local run:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config <config-path> --max-ticks 2 --status-every 1
```

Then inspect:

- the first `bootstrap_fetch` and `bootstrap_ingest` log records;
- `health.runtime.state`;
- `lifecycle.clean_shutdown` and `lifecycle.recovered_from_unclean_shutdown`;
- `health.bootstrap.last_accepted_sources`;
- `health.bootstrap.last_attempt_summary`;
- `health.bootstrap.last_sources`;
- `health.metrics`;
- `health.relay`;
- `health.resource_limits`.

If the process is still running or has already exited, also read the persisted
status surface:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- status --config <config-path>
```

## Matrix

| Symptom | Real signals to inspect | Likely cause in the current repo | Action |
| --- | --- | --- | --- |
| Bootstrap failure | `health.runtime.state == "degraded"`, `health.bootstrap.last_accepted_sources == 0`, and `health.bootstrap.last_attempt_summary` or `last_sources` show `unavailable`, `integrity_mismatch`, `stale`, `empty_peer_set`, or `rejected` | missing file, invalid JSON, expired bootstrap seed, empty peer list, unsupported source such as `https://...`, schema rejection, or a SHA-256 pin mismatch on `http://...#sha256=<hex>` | fix the specific source called out in `last_sources`; use only local `.json`, `file:`, or static pinned `http://...` sources; rerun with `--max-ticks 0 --status-every 1` first |
| Lookup timeout or `not_found` | external caller waits, or `overlay-cli lookup` returns `not_found`; if you have observability around a lookup call, `lookup_node` results are `missing`, `negative_cache_hit`, or `budget_exhausted` | the target runtime does not have the record in its local rendezvous store, the publish was sent to a different runtime, or the exact `node_id` is wrong | first reproduce with `overlay-cli lookup --config <path> --target <tcp://host:port> --node-id <hex>` against the runtime that received the publish; if it still fails, fix publish target, exact ID, or record freshness first |
| Relay quota rejection | relay log `resolve_intro`=`rejected_rate_limited`, relay log `bind_tunnel`=`rejected`, `health.metrics.dropped_rate_limited_total` increments, `health.relay.active_tunnels` or `recent_intro_requests` approaches `health.resource_limits` | intro request or tunnel quota hit, or relay mode is disabled on that node | use a node with `relay_mode: true`; reduce concurrent relay load; confirm the relay-enabled config was actually the one started |
| Service policy denial | service log `open_app_session`=`rejected_policy` | local service policy is deny-all in the embedding or harness | the stock JSON schema cannot change this; inspect the caller code that registers the service because checked-in configs alone cannot produce allow or deny policy changes |
| Degraded route churn | `health.metrics.path_switch_total` rises, routing log `route_selection`=`switched`, routing `probe_feedback` shows `lost` or `expired`, `probe_loss_ratio` stays high | path quality oscillation in embedded runtime use; only `path_probe_interval_ms` is operator-configurable today, not hysteresis thresholds | reduce path volatility in the test setup, inspect the path candidates, and remember this is mostly a code-level tuning issue in the current repo rather than a rich operator-configurable surface |

## Common log meanings

- `bootstrap_fetch=accepted`: a bootstrap source loaded and validated.
- `bootstrap_fetch=unavailable`: file missing, parse failure, or unsupported
  source format.
- `bootstrap_fetch=rejected`: bootstrap JSON loaded but failed schema or
  freshness validation.
- `bootstrap_ingest=accepted`: validated peers were accepted into the peer
  store.
- `health.bootstrap.last_sources[].result=integrity_mismatch`: a pinned
  artifact hash did not match the downloaded body.
- `health.bootstrap.last_sources[].result=stale`: the bootstrap artifact was
  expired or timing-invalid.
- `health.bootstrap.last_sources[].result=empty_peer_set`: the artifact
  validated but contained zero peers.
- `state_transition=degraded`: no active peers are currently available.
- `state_transition=running`: at least one active peer is available.
- `publish_presence=stored|replaced|duplicate|stale`: local presence publish
  result.
- `lookup_node=found|missing|negative_cache_hit|budget_exhausted`: exact lookup
  result from the local rendezvous store.
- `resolve_intro=rejected_relay_disabled|rejected_role_disabled|rejected_rate_limited`:
  relay intro failure mode.
- `open_app_session=rejected_not_found|rejected_policy|rejected_reachability_mismatch|rejected_session_limit`:
  local service-open failure mode.

## Limits worth remembering during triage

- `overlay-cli run` is single-node inspection, not distributed orchestration.
- `overlay-cli smoke` is the supported repo-local proof path.
- `overlay-cli publish`, `lookup`, `open-service`, and `relay-intro` are
  explicit point-to-point operator commands; they do not provide discovery or
  autonomous distributed routing.
- A relay-enabled node still uses compiled default quotas; there is no JSON knob
  for per-minute intro rate or tunnel cap.
- `overlay-cli run` now handles `SIGINT` and `SIGTERM` gracefully, but any
  crash or hard kill will leave `.overlay-runtime/` marked as an unclean exit
  until the next startup recovers it.
