# Launch Checklist

This checklist defines the Milestone 17 operator-runtime gate that remains the
prerequisite launch gate for the current Milestone 19 pilot-closure stage.

It is a pilot gate, not a public-production or hostile-Internet readiness
claim.

For current Milestone 19 sign-off, the validation green path is
`./devnet/run-launch-gate.sh` followed by
`./devnet/run-distributed-pilot-checklist.sh` on the same commit.

## Launchable surface freeze

The current launchable MVP surface is frozen to:

- node identity and key handling;
- canonical wire framing and the current message catalog;
- session handshake and real TCP session establishment;
- local bootstrap-file startup plus minimal static `http://` bootstrap fetch
  with optional `#sha256=<pin>` integrity checks;
- exact presence publish and exact lookup by `node_id`;
- direct-first reachability planning with relay fallback;
- bounded path metrics, deterministic scoring, and hysteresis;
- exact service resolution by `app_id` and `OpenAppSession`;
- structured logs, runtime health snapshots, and bounded counters;
- `overlay-cli run` / `overlay-cli status` for single-node inspection;
- `overlay-cli run --service` for bounded local service registration;
- `overlay-cli publish`, `lookup`, `open-service`, and `relay-intro` as
  one-shot distributed operator surfaces;
- `overlay-cli bootstrap-serve` for static devnet seed serving;
- the checked-in devnet layouts, launch gate, and pilot-closure checklist.

Anything outside that list is out of the Milestone 17 gate unless a later task
explicitly reopens scope.

## Not in this gate

The current gate does not claim:

- broad public bootstrap-provider infrastructure;
- HTTPS, DNS-derived bootstrap, or a public bootstrap trust framework;
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
- the multi-host smoke includes networked `publish_presence`, `lookup_node`,
  `open_service`, `relay_fallback_planned`, and `relay_fallback_bound`;
- the bounded soak completes with cleanup and presence-refresh checks;
- the restart smoke proves a `SIGTERM`-driven clean shutdown, a readable
  `overlay-cli status` surface, and a second clean startup against the same
  config.

## Milestone 19 follow-on

After the launch gate stays green on the target commit, run the current
pilot-closure checklist:

```bash
./devnet/run-distributed-pilot-checklist.sh
```

Pass criteria:

- the baseline distributed operator flow succeeds;
- the `node-c-down` scenario still completes;
- the primary-relay-down scenario falls back to the alternate relay path;
- the primary relay may emit one expected connection failure during the
  `relay-unavailable` scenario, but the checklist still passes only if the
  alternate relay path succeeds and the final summary reaches
  `pilot_checklist_complete`;
- the bootstrap-seed-unavailable scenario still completes;
- the service-host restart scenario reports a clean later startup;
- the tampered-bootstrap scenario is rejected by the new integrity check;
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
   network bootstrap plus networked `publish`, `lookup`, `open-service`, and
   relay fallback on the host-style devnet layout.
8. Run `./devnet/run-distributed-pilot-checklist.sh` and collect the resulting
   pilot-closure scenario evidence plus summary fields.
9. Run the actual off-box pilot on separate hosts and attach the collected
   operator-command logs and per-host status snapshots.
10. Cut the pilot tag only after the gate stays green on the exact commit being
    tagged and the off-box report is attached.

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
- Bootstrap remains static JSON served over `http://`; integrity comes from
  SHA-256-pinned artifact URLs rather than HTTPS or a public trust root.
- `overlay-cli bootstrap-serve` is a devnet seed server, not a public bootstrap
  service or trust framework.
- The distributed operator commands are bounded one-shot CLI surfaces, not a
  general distributed control plane, discovery layer, or rollout system.
- The multi-host smoke and the distributed pilot checklist prove point-to-point
  networked operator flows only; they do not imply autonomous routing of those
  control messages through arbitrary peers.
- Relay fallback is now proven for two documented paths only:
  `node-a -> node-relay -> node-b` and
  `node-a -> node-relay-b -> node-b`.
- Lookup is exact-by-`node_id` only, and service resolution is exact-by-`app_id`
  only.
- Relay quotas and most service-open policy are still code-level defaults rather
  than a rich operator-configurable surface.

## Closure items before regular distributed use

- Run and attach the off-box pilot report for the exact commit being signed
  off; the localhost checklist is necessary but not sufficient evidence.
- Keep bootstrap artifacts and their pinned `#sha256=<hex>` URLs synchronized
  manually; the current repo still has no HTTPS bootstrap or public trust root.
- Treat the distributed operator commands as explicit proof surfaces only; they
  are not a general distributed control plane, orchestration layer, or
  discovery system.
- Expect restart loss of peers, presence, service-open state, relay tunnels,
  and path probes until durable protocol-state persistence is explicitly added.
- Treat the documented two-relay fallback paths as the only proven relay
  closure paths for the current stage.
