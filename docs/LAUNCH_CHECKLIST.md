# Launch Checklist

This checklist defines the Milestone 14 launch gate for the current
pilot-ready network surface.

It is a pilot gate, not a public-production or hostile-Internet readiness
claim.

## Launchable surface freeze

The current launchable MVP surface is frozen to:

- node identity and key handling;
- canonical wire framing and the current message catalog;
- session handshake and placeholder session establishment;
- local bootstrap-file startup and peer ingest;
- exact presence publish and exact lookup by `node_id`;
- direct-first reachability planning with relay fallback;
- bounded path metrics, deterministic scoring, and hysteresis;
- exact service resolution by `app_id` and `OpenAppSession`;
- structured logs, runtime health snapshots, and bounded counters;
- `overlay-cli run` for single-node inspection;
- `overlay-cli smoke` plus the checked-in four-node devnet for the green path.

Anything outside that list is out of the Milestone 14 launch gate unless a
later task explicitly reopens scope.

## Not in this gate

The current launch gate does not claim:

- public bootstrap-provider fetch over the network;
- real sockets, listeners, or distributed multi-process transport;
- persistent on-disk peers, presence, services, sessions, or relay state;
- global node or service discovery;
- onion routing or stronger anonymity;
- full post-quantum handshake;
- daemon supervision, signal-driven graceful shutdown, or upgrade orchestration;
- public hostile-environment deployment readiness.

## Required command order

Run the gate from the repository root:

```bash
cargo fmt --all --check
TMPDIR=/tmp cargo clippy --workspace --all-targets --all-features -- -D warnings
TMPDIR=/tmp cargo check --workspace
TMPDIR=/tmp cargo test --workspace
TMPDIR=/tmp cargo test -p overlay-core --test integration_bootstrap
TMPDIR=/tmp cargo test -p overlay-core --test integration_publish_lookup
TMPDIR=/tmp cargo test -p overlay-core --test integration_relay_fallback
TMPDIR=/tmp cargo test -p overlay-core --test integration_routing
TMPDIR=/tmp cargo test -p overlay-core --test integration_service_open
./devnet/run-smoke.sh
./devnet/run-restart-smoke.sh
```

CI-friendly wrapper:

```bash
./devnet/run-launch-gate.sh
```

Pass criteria:

- every command exits `0`;
- the devnet smoke reaches `smoke_complete`;
- the devnet smoke includes `relay_fallback_planned` and
  `relay_fallback_bound`;
- the restart smoke completes two consecutive bounded `overlay-cli run`
  startups against the same config.

## Green path launch sequence

1. Confirm a clean repository state and record the target commit SHA.
2. Run the launch gate in the required order above.
3. Run a bounded single-node inspection if you want operator-facing logs before
   tagging:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config docs/config-examples/bootstrap-node.json --max-ticks 2 --status-every 1
   ```

4. Confirm the first logs include `bootstrap_fetch`, `bootstrap_ingest`, and
   `state_transition`, and that `runtime_status.health.runtime.state` becomes
   `running` or `degraded`.
5. Use the devnet smoke as the end-to-end proof path for bootstrap, session,
   publish, lookup, service open, and relay fallback.
6. Cut the pilot tag only after the gate stays green on the exact commit being
   tagged.

## Pilot tag workflow

Use the crate version already checked into the workspace and append a pilot RC
suffix. For the current workspace version, the first candidate tag is:

```text
pilot-v0.1.0-rc1
```

Workflow:

1. Pick the next unused `pilot-v<crate-version>-rcN` tag.
2. Run the full launch gate on the exact commit you intend to tag.
3. Copy [docs/PILOT_RELEASE_TEMPLATE.md](/mnt/c/Users/Noki1/OneDrive/Documents/testdsn/docs/PILOT_RELEASE_TEMPLATE.md)
   into a release note for that candidate and fill in the command results,
   commit SHA, and limitations.
4. Create an annotated tag on the validated commit:

   ```bash
   git tag -a pilot-v0.1.0-rc1 -m "Pilot release candidate v0.1.0 rc1"
   ```

5. If any code, config, vector, or documentation changes after tagging, do not
   move the old tag. Rerun the gate and cut `rcN+1`.

## Known limitations to carry into every pilot note

- The current runtime is in-memory only and loses peers, sessions, presence,
  service-open state, relay tunnels, and path probes on restart.
- The checked-in green path is local-file and in-process; it does not prove a
  distributed network data plane.
- Bootstrap sources are local `.json` files or `file:` URIs only.
- The current CLI does not expose standalone publish, lookup, relay-intro, or
  service-open operator commands outside the smoke harness.
- Relay fallback is proven for the documented local path only:
  `node-a -> node-relay -> node-b`.
- Lookup is exact-by-`node_id` only, and service resolution is exact-by-`app_id`
  only.
- Relay quotas and service-open policy remain code-level defaults rather than a
  full operator-configurable control surface.
