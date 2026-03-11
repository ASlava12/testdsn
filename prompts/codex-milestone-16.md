Read `AGENTS.md`, `IMPLEMENT.md`, `README.md`, `HANDOFF.md`, `VALIDATION.md`,
`docs/LAUNCH_CHECKLIST.md`, `docs/RUNBOOK.md`, `docs/DEVNET.md`,
`docs/OPEN_QUESTIONS.md`, `devnet/hosts/README.md`, `spec/mvp-scope.md`,
`spec/architecture.md`, `spec/wire-protocol.md`, `spec/records.md`,
`spec/state-machines.md`, and `spec/config.md`.

Goal:
Work within the current repository stage, `milestone-16-network-bootstrap`,
from the landed Milestone 1-14 baseline plus the current minimal network
bootstrap and multi-host devnet surface.

Current repository baseline:
- The current repository stage marker is `milestone-16-network-bootstrap`.
- Milestones 0-12 are implemented, validated, and considered closed.
- Milestone 14 launch gate and pilot tag are implemented and remain part of the
  pilot baseline.
- Milestone 16 network bootstrap and multi-host devnet are implemented with
  minimal static `http://` bootstrap fetch, `overlay-cli bootstrap-serve`,
  host-style devnet configs under `devnet/hosts/`, `run-distributed-smoke.sh`,
  and `run-multihost-smoke.sh`.

Requirements:
- preserve the current launch surface documented in
  `docs/LAUNCH_CHECKLIST.md`, `docs/RUNBOOK.md`, `docs/DEVNET.md`, and
  `devnet/hosts/README.md`;
- keep Milestones 1-12 limited to regression fixes, validation maintenance,
  vector maintenance, or conservative spec-conformance fixes;
- keep explicit layering between identity, bootstrap, transport/session, peer,
  rendezvous, relay, routing, and service code;
- keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, prompts, and
  `docs/OPEN_QUESTIONS.md` synchronized if the current stage boundary changes;
- prefer minimal, static, operator-comprehensible bootstrap flows over broad
  infrastructure.

Constraints:
- do not add public bootstrap-provider infrastructure;
- do not add global discovery or anonymity features;
- do not redesign bootstrap message semantics;
- do not broaden scope beyond the documented Milestone 16 surface unless
  explicitly requested.

Validation:
- run the applicable commands from `VALIDATION.md`;
- rerun the documented gate whenever `REPOSITORY_STAGE`, launch docs, or launch
  scripts change;
- report exactly what passed, what failed, and whether any failure is an
  environment issue rather than a code regression.
