# Launch Checklist

This checklist defines the Milestone 17 operator-runtime gate that remains the
prerequisite launch gate for the current Milestone 18 pilot stage.

It is a pilot gate, not a public-production or hostile-Internet readiness
claim.

## Launchable surface freeze

The current launchable MVP surface is frozen to:

- node identity and key handling;
- canonical wire framing and the current message catalog;
- session handshake and minimal real TCP session establishment;
- local bootstrap-file startup plus minimal `http://` bootstrap fetch;
- exact presence publish and exact lookup by `node_id`;
- direct-first reachability planning with relay fallback;
- bounded path metrics, deterministic scoring, and hysteresis;
- exact service resolution by `app_id` and `OpenAppSession`;
- structured logs, runtime health snapshots, and bounded counters;
- `overlay-cli run` for single-node inspection with signal-aware graceful
  shutdown;
- `overlay-cli status` for last-known operator health and lifecycle state;
- `overlay-cli bootstrap-serve` for static devnet seed serving;
- `overlay-cli smoke` plus the checked-in devnet layouts for local and
  multi-host-style proof paths.

Anything outside that list is out of the Milestone 17 gate unless a later task
explicitly reopens scope.

## Not in this gate

The current gate does not claim:

- broad public bootstrap-provider infrastructure;
- HTTPS, DNS-derived bootstrap, or bootstrap trust roots;
- persistent on-disk peers, presence, services, sessions, or relay state
  beyond bounded operator metadata and last-known health;
- global node or service discovery;
- onion routing or stronger anonymity;
- full post-quantum handshake;
- upgrade orchestration or rolling deploy automation;
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
./devnet/run-distributed-smoke.sh
./devnet/run-multihost-smoke.sh
./devnet/run-soak.sh
./devnet/run-restart-smoke.sh
```

CI-friendly wrapper:

```bash
./devnet/run-launch-gate.sh
```

Pass criteria:

- every command exits `0`;
- the local smoke reaches `smoke_complete`;
- the distributed smoke reaches `distributed_smoke_complete`;
- the multi-host smoke reaches `smoke_complete`;
- the multi-host smoke includes `publish_presence`, `lookup_node`,
  `open_service`, `relay_fallback_planned`, and `relay_fallback_bound`;
- the bounded soak completes with cleanup and presence-refresh checks;
- the restart smoke proves a `SIGTERM`-driven clean shutdown, a readable
  `overlay-cli status` surface, and a second clean startup against the same
  config.

## Milestone 18 follow-on

After the launch gate stays green on the target commit, run the current pilot
checklist:

```bash
./devnet/run-pilot-checklist.sh
```

Pass criteria:

- the baseline pilot rehearsal reaches `smoke_complete`;
- the `node-c-down` scenario still reaches `smoke_complete`;
- the `relay-unavailable` scenario reaches `relay_fallback_unavailable` and
  reports a degraded-but-expected result;
- the bootstrap-seed-unavailable scenario still reaches `smoke_complete`;
- the pilot-config restart check reports a clean second startup;
- the final output reaches `pilot_checklist_complete`.

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
5. Confirm `overlay-cli status --config <path>` returns the same node's last
   known `health` plus `lifecycle.clean_shutdown` / `lifecycle.startup_count`.
6. Use `./devnet/run-distributed-smoke.sh` as the real-process proof path for
   network bootstrap, listener bind, outbound dial, accept, and handshake-backed
   session establishment.
7. Use `./devnet/run-multihost-smoke.sh` as the repo-local proof path for
   network bootstrap plus publish, lookup, service open, and relay fallback on
   the host-style devnet layout.
8. Run `./devnet/run-pilot-checklist.sh` and collect the resulting pilot
   scenario evidence plus summary fields.
9. Cut the pilot tag only after the gate stays green on the exact commit being
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
- The current on-disk state is bounded to operator lock/status metadata under
  `.overlay-runtime/`; it is not protocol-state persistence.
- The new network bootstrap path is intentionally minimal: it fetches static
  bootstrap JSON over plain `http://` only.
- `overlay-cli bootstrap-serve` is a devnet seed server, not a public bootstrap
  service or trust framework.
- The current CLI still does not expose standalone publish, lookup,
  relay-intro, or service-open operator commands outside the smoke harness.
- The checked-in multi-host smoke uses host-style configs and real TCP session
  establishment, but publish, lookup, service open, and relay fallback remain
  harness-coordinated proof steps rather than a distributed control-plane
  implementation.
- The Milestone 18 pilot checklist adds first-pilot rehearsal and reporting,
  but it still uses the current smoke harness for the publish, lookup,
  service-open, and relay-fallback proof path.
- Relay fallback is still proven for the documented path only:
  `node-a -> node-relay -> node-b`.
- Lookup is exact-by-`node_id` only, and service resolution is exact-by-`app_id`
  only.
- Relay quotas and service-open policy remain code-level defaults rather than a
  full operator-configurable control surface.
