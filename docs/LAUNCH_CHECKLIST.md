# Launch Checklist

This checklist defines the landed Milestone 17 operator-runtime gate that
remains the first component of the current Milestone 27
relay-topology-generalization stage.

It is a pilot gate, not a public-production or hostile-Internet readiness
claim.

For current Milestone 27 sign-off, the bounded acceptance flow is
`./devnet/run-first-user-acceptance.sh` on the same commit after the
applicable workspace validation commands.

This checklist still matters because `./devnet/run-first-user-acceptance.sh`
reuses `./devnet/run-launch-gate.sh` as a landed prerequisite component.

`./devnet/run-pilot-checklist.sh` remains a retained Milestone 18 localhost
rehearsal. Do not treat it as the current sign-off path.

Use [docs/FIRST_USER_ACCEPTANCE.md](FIRST_USER_ACCEPTANCE.md)
for the current first-user-ready acceptance scenarios and boundary.

## Launchable surface freeze

The current launchable MVP surface is frozen to:

- node identity and key handling;
- canonical wire framing and the current message catalog;
- session handshake and real TCP session establishment;
- local bootstrap-file startup plus static signed `http://` bootstrap fetch
  with pinned `#ed25519=<pin>` trust roots and optional `#sha256=<pin>`
  integrity checks;
- exact presence publish and exact lookup by `node_id`;
- direct-first reachability planning with relay fallback;
- bounded path metrics, deterministic scoring, and hysteresis;
- exact service resolution by `app_id` and `OpenAppSession`;
- structured logs, runtime health snapshots, and bounded counters;
- `overlay-cli run`, `overlay-cli status`, `overlay-cli status --summary`, and
  `overlay-cli doctor` for single-node inspection;
- `overlay-cli inspect` for one machine-readable operator report that combines
  local persisted status/doctor data with explicit requested remote probes;
- `overlay-cli run --service` for bounded local service registration;
- `overlay-cli publish`, `lookup`, `open-service`, and `relay-intro` as
  explicit distributed operator surfaces;
- `overlay-cli bootstrap-serve` for static devnet seed serving;
- the checked-in devnet layouts, launch gate, and regular-distributed-use
  checklist.

Anything outside that list is out of the Milestone 17 gate unless a later task
explicitly reopens scope.

## Not in this gate

The current gate does not claim:

- broad public bootstrap-provider infrastructure;
- HTTPS, DNS-derived bootstrap, or a public bootstrap trust framework;
- broad persistent on-disk peers, presence, services, sessions, or relay state
  beyond bounded operator metadata, last-known health, persisted
  bootstrap-source state, and the last-known active bootstrap peers plus local
  service registration intent;
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
./devnet/run-doctor-smoke.sh
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
  `open_service`, `relay_fallback_planned`, `relay_fallback_bound`, and an
  `operator_inspect` report with `result=ok`;
- the bounded soak completes with cleanup and presence-refresh checks;
- the doctor smoke proves `overlay-cli doctor` returns `0` and reports a
  healthy running runtime;
- the restart smoke proves a `SIGTERM`-driven clean shutdown, a readable
  `overlay-cli status` surface, bootstrap-source plus peer-cache recovery,
  local service-intent recovery on the second startup, and a second clean
  shutdown against the same config.

## Current distributed follow-on

After the launch gate stays green on the target commit, the current acceptance
flow continues with the distributed checklist:

```bash
./devnet/run-distributed-pilot-checklist.sh
```

Optional evidence-preserving form:

```bash
./devnet/run-distributed-pilot-checklist.sh --evidence-dir /tmp/overlay-pilot-evidence
```

Pass criteria:

- the baseline distributed operator flow succeeds;
- the `node-c-down` scenario still completes and all three relay paths bind
  again;
- the primary-relay-down scenario falls back to the alternate relay path;
- the repeated-relay-bind-failure-recovery scenario reaches the tertiary relay
  path after two explicit relay-intro failures;
- the primary relay may emit one expected connection failure during the
  `relay-unavailable` scenario, but the checklist still passes only if the
  alternate relay path succeeds and the final summary reaches
  `pilot_checklist_complete`;
- the bootstrap-seed-unavailable scenario still completes;
- the integrity-mismatch-fallback scenario reports one integrity mismatch and
  one accepted source while startup still reaches `running`;
- the trust-verification-fallback scenario reports one trust verification
  failure and one accepted source while startup still reaches `running`;
- the stale-bootstrap-fallback scenario reports one stale source and one
  accepted source while startup still reaches `running`;
- the empty-bootstrap-fallback scenario reports one empty-peer-set source and
  one accepted source while startup still reaches `running`;
- the service-host restart scenario reports a clean later startup, restored
  local service intent on the restarted host, and another alternate-relay
  bind;
- the tampered-bootstrap scenario is rejected by the new integrity check;
- the final output reaches `pilot_checklist_complete`.

## Operator assumptions for the current checklist path

- `./devnet/run-distributed-pilot-checklist.sh` uses the checked-in
  `devnet/pilot/localhost/` configs and bootstrap artifacts; do not mix those
  ports or pins with `devnet/pilot/examples/`.
- The checked-in pilot pack assumes three static bootstrap seed servers and one
  service host: `node-b` must start with `--service devnet:terminal`.
- The distributed operator surfaces are explicit CLI calls to chosen runtime
  listeners; `overlay-cli inspect` may bundle multiple requested probes, but
  it still does not auto-discover targets or route a control plane through the
  overlay.
- If any bootstrap JSON changes, every referencing
  `http://...#ed25519=<pin>` config entry must still trust the intended
  signer, and every `#sha256=<pin>` value must be recomputed before the pilot
  checklist is honest again.
- The current checklist may be run with `--evidence-dir <dir>` to preserve the
  raw logs and status snapshots that back the final summary.
- The current checklist proves the documented relay paths
  `node-a -> node-relay -> node-b` and
  `node-a -> node-relay-b -> node-b` and
  `node-a -> node-relay-c -> node-b` across the checked-in node-down,
  primary-relay-down, repeated-relay-failure, and service-restart scenarios
  only.

## Green path launch sequence

1. Confirm a clean repository state and record the target commit SHA.
2. Run the launch gate in the required order above.
3. Run a bounded single-node inspection if you want operator-facing logs before
   tagging:

   ```bash
   TMPDIR=/tmp cargo run -p overlay-cli -- run --config docs/config-examples/bootstrap-seed.json --max-ticks 2 --status-every 1
   ```

4. Confirm the first logs include `bootstrap_fetch`, `bootstrap_ingest`, and
   `state_transition`, and that `runtime_status.health.runtime.state` becomes
   `running` or `degraded`.
5. Confirm `overlay-cli status --config <path>` returns the same node's last
   known `health` plus `lifecycle.clean_shutdown` / `lifecycle.startup_count`,
   and that `overlay-cli status --config <path> --summary` exposes the concise
   peer/bootstrap/presence/service/relay summary.
6. Use `./devnet/run-distributed-smoke.sh` as the real-process proof path for
   network bootstrap, listener bind, outbound dial, accept, and handshake-backed
   session establishment.
7. Use `./devnet/run-multihost-smoke.sh` as the repo-local proof path for
   network bootstrap plus networked `publish`, `lookup`, `open-service`, and
   relay fallback on the host-style devnet layout.
8. Run `./devnet/run-distributed-pilot-checklist.sh` and collect the resulting
   scenario evidence plus summary fields. Use `--evidence-dir <dir>` when you
   want the wrapper to preserve the raw logs and status files automatically.
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
3. Copy [docs/PILOT_RELEASE_TEMPLATE.md](PILOT_RELEASE_TEMPLATE.md)
   into a release note for that candidate and fill in the command results,
   commit SHA, and limitations.
4. Create an annotated tag on the validated commit:

   ```bash
   git tag -a pilot-v0.1.0-rc1 -m "Pilot release candidate v0.1.0 rc1"
   ```

5. If any code, config, vector, or documentation changes after tagging, do not
   move the old tag. Rerun the gate and cut `rcN+1`.

## Known limitations to carry into every pilot note

- The current runtime recovers persisted bootstrap-source preference,
  last-known active bootstrap peers, and local service registration intent on
  restart; sessions, presence, service-open state, relay tunnels, and path
  probes are rebuilt.
- The current on-disk state is bounded to operator lock/status metadata and
  the embedded bootstrap-source, peer-cache, and local-service-intent
  recovery payload under `.overlay-runtime/`; it is not broad
  protocol-state persistence.
- Bootstrap remains static signed JSON served over `http://`; trust comes from
  pinned signer keys with optional SHA-256 artifact pins rather than HTTPS or
  a public trust root.
- `overlay-cli bootstrap-serve` is a devnet seed server, not a public bootstrap
  service or trust framework.
- The distributed operator surfaces remain bounded explicit CLI flows.
  `overlay-cli inspect` bundles local state plus requested remote probes, but
  it is not a general distributed control plane, discovery layer, or rollout
  system.
- The multi-host smoke and the distributed pilot checklist prove point-to-point
  networked operator flows only; they do not imply autonomous routing of those
  control messages through arbitrary peers.
- Relay fallback is now proven for the checked-in three-relay topology only:
  `node-a -> node-relay -> node-b` and
  `node-a -> node-relay-b -> node-b` and
  `node-a -> node-relay-c -> node-b`.
- Lookup is exact-by-`node_id` only, and service resolution is exact-by-`app_id`
  only.
- Relay quotas and most service-open policy are still code-level defaults rather
  than a rich operator-configurable surface.

## Remaining limitations after Milestone 27

- Run and attach the off-box pilot report for the exact commit being signed
  off; the localhost checklist is necessary but not sufficient evidence for a
  release note.
- Keep bootstrap artifacts, signer pins, and any `#sha256=<hex>` URLs
  synchronized manually; the current repo still has no HTTPS bootstrap or
  public trust root.
- Keep the current localhost sign-off path anchored on
  `./devnet/run-distributed-pilot-checklist.sh`; the older
  `./devnet/run-pilot-checklist.sh` remains a historical rehearsal only.
- Treat the distributed operator surfaces as bounded proof flows only;
  `overlay-cli inspect` improves repeatable operator checks, but it is not a
  general distributed control plane, orchestration layer, or discovery
  system.
- Expect restart loss of presence, service-open state, relay tunnels, and path
  probes until durable protocol-state persistence is explicitly added.
- Treat the checked-in three-relay topology as the only proven relay closure
  layout for the current stage.
