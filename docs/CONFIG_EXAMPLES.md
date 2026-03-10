# Config Examples

Role-based runnable examples live in
[docs/config-examples](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/config-examples).

These examples intentionally reuse the checked-in `devnet/` keys and bootstrap
seed files so they can be loaded and validated directly in this repository.

## Accepted top-level config fields

The current `OverlayConfig` JSON schema accepts only these fields:

- `node_key_path`
- `bootstrap_sources`
- `max_total_neighbors`
- `max_presence_records`
- `max_service_records`
- `presence_ttl_s`
- `epoch_duration_s`
- `path_probe_interval_ms`
- `max_transport_buffer_bytes`
- `relay_mode`
- `log_level`

Unknown operator knobs are not available yet. In particular:

- relay quota values are not JSON-configurable;
- service open allow or deny policy is not JSON-configurable;
- lookup budget, negative cache size, and open-service-session limits are not
  JSON-configurable.

## Field notes

- `node_key_path`: relative to the config file directory unless absolute.
- `bootstrap_sources`: each entry must be:
  - a local `.json` path; or
  - a `file:<path>` URI.
- `bootstrap_sources`: network URLs are not supported by the current runtime and
  will be treated as unavailable.
- `max_total_neighbors`: drives the peer-store cap and the runtime's managed
  session and tracked-path caps.
- `max_presence_records`: projects into the local rendezvous published-record
  store.
- `max_service_records`: projects into the local service registration store.
- `presence_ttl_s`: drives local presence refresh cadence, stale service session
  age, and stale relay tunnel age.
- `path_probe_interval_ms`: controls scheduling cadence and derived probe
  timeout.
- `max_transport_buffer_bytes`: must stay within the runtime's accepted
  transport frame budget.
- `relay_mode`: `true` enables the relay profile locally; `false` leaves relay
  roles disabled even though relay limits still appear in status output.
- `log_level`: accepted values are lowercase `error`, `warn`, `info`, `debug`,
  or `trace`, but the current runtime does not yet apply this field as a stdout
  log filter.

## Role examples

- Bootstrap anchor:
  [bootstrap-node.json](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/config-examples/bootstrap-node.json)
- Standard node:
  [standard-node.json](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/config-examples/standard-node.json)
- Relay-enabled node:
  [relay-enabled-node.json](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/config-examples/relay-enabled-node.json)
- Service-host node:
  [service-host-node.json](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/config-examples/service-host-node.json)

All four are loadable with:

```bash
TMPDIR=/tmp cargo run -p overlay-cli -- run --config <example-path> --max-ticks 0 --status-every 1
```

## Bootstrap seed files

Bootstrap seed JSON files are separate from node config JSON files. The checked
in working examples live under
[devnet/bootstrap](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/devnet/bootstrap).

The current bootstrap seed schema contains:

- `version`
- `generated_at_unix_s`
- `expires_at_unix_s`
- `network_params.network_id`
- `epoch_duration_s`
- `presence_ttl_s`
- `max_frame_body_len`
- `handshake_version`
- `peers[]`
- `bridge_hints[]`

Each `peers[]` entry carries:

- `node_id`
- `transport_classes`
- `capabilities`
- `dial_hints`
- `observed_role`

The runtime validates schema version, freshness, handshake version, frame-size
limits, duplicate peer IDs, duplicate bridge hints, and blank or unsupported
transport and capability values before peer ingest.

## Current limitations documented by these examples

- The examples are local and repository-relative on purpose.
- A bootstrap anchor is still just a node plus a seed file, not a bootstrap
  service process.
- Service-host behavior in the stock repo comes from the smoke harness and
  caller-side service registration, not from extra JSON fields.
- Relay-enabled behavior is controlled only by `relay_mode`; other relay limits
  come from built-in profile defaults.
