# Known Limitations

Carry these limits into every current Milestone 28 release note and handoff.

## Current bounded production claim

The repository supports only a bounded production claim for operator-managed
deployments that stay inside the validated release gate, static signed
bootstrap model, explicit operator surfaces, and checked-in three-relay proof
topology.

## Limits that still apply

- bootstrap remains static signed artifact delivery over `http://`; there is
  still no HTTPS bootstrap transport or public trust framework
- signer-key distribution, rotation, and optional SHA-256 pin maintenance are
  still manual operator tasks
- operator surfaces remain explicit and operator-directed; `overlay-cli
  inspect` improves repeatable checks but does not create a distributed
  control plane or discovery mesh
- lookup remains exact-by-`node_id` only, and service resolution remains
  exact-by-`app_id` only
- restart recovery remains bounded to persisted bootstrap-source preference,
  last-known active bootstrap peers, and local service registration intent;
  presence, service-open sessions, relay tunnels, and path probes still rebuild
- relay fallback is validated only for the checked-in bounded three-relay
  topology, not arbitrary relay graphs or public-network conditions
- release packages are validated tarball installs for operator-managed hosts;
  there is still no service-manager packaging, auto-updater, or platform-wide
  installer matrix
- separate-host evidence is still required on the exact validated commit
  before a release note is honest

## Still out of scope

- hostile-environment or censorship-at-scale deployment guarantees
- anonymity or onion-routing claims
- global discovery, public bootstrap-provider infrastructure, or a public
  admin/control plane
- broad durable protocol-state persistence or a distributed database
- rolling upgrades, fleet orchestration, or autonomous recovery beyond the
  current bounded model
