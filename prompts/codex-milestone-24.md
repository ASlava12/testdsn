Read `AGENTS.md`, `IMPLEMENT.md`, `README.md`, `HANDOFF.md`, `VALIDATION.md`,
`docs/FIRST_USER_ACCEPTANCE.md`, `docs/LAUNCH_CHECKLIST.md`,
`docs/PILOT_RUNBOOK.md`, `docs/RUNBOOK.md`, `docs/DEVNET.md`,
`docs/OPEN_QUESTIONS.md`, `devnet/README.md`, `spec/mvp-scope.md`,
`spec/architecture.md`, `spec/wire-protocol.md`, `spec/records.md`, and
`spec/state-machines.md`.

Goal:
Work within the current repository stage,
`milestone-24-bootstrap-trust-delivery-hardening`, from the landed
Milestone 1-22 baseline and keep Milestone 24 focused on bounded bootstrap
trust hardening, operator diagnostics, acceptance evidence, and honest stage
docs without widening protocol scope.

Current repository baseline:
- The current repository stage marker is
  `milestone-24-bootstrap-trust-delivery-hardening`.
- Milestones 0-12 are implemented, validated, and considered closed.
- Milestone 14 launch gate, Milestone 16 network bootstrap, Milestone 17
  operator runtime hardening, Milestone 18 real pilot, Milestone 19 pilot
  closure, Milestone 20 regular distributed use closure, Milestone 21
  first-user runtime, and Milestone 22 first-user acceptance pack are landed
  baseline work.
- Milestone 24 bootstrap trust and delivery hardening adds signed bootstrap
  artifacts, pinned signer-key verification with optional SHA-256 integrity
  pins, `overlay-cli bootstrap-sign`, signed `bootstrap-serve`,
  trust-verification diagnostics, and synchronized bootstrap/operator docs.

Requirements:
- preserve the current launch surface and distributed pilot proof path;
- prefer reproducible bootstrap trust coverage, checklist clarity, and runbook
  honesty over new features;
- keep Milestones 1-12 limited to regression fixes, validation maintenance,
  vector maintenance, or conservative spec-conformance fixes;
- keep status docs, prompts, and runbooks synchronized if the stage boundary
  changes again.

Constraints:
- do not add major new features, transports, or discovery redesign;
- do not redesign wire, handshake, record, quota, or routing semantics unless
  a concrete bug requires it;
- do not claim public-production or hostile-environment readiness;
- do not widen the first-user-ready claim beyond what the current acceptance
  pack actually proves.

Validation:
- run the applicable commands from `VALIDATION.md`;
- rerun `./devnet/run-first-user-acceptance.sh` when current-stage scripts,
  stage docs, acceptance docs, or the distributed checklist change;
- report exactly what passed, what failed, and whether any failure is an
  environment issue rather than a code regression.
