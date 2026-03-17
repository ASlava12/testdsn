Read `AGENTS.md`, `IMPLEMENT.md`, `README.md`, `HANDOFF.md`, `VALIDATION.md`,
`docs/LAUNCH_CHECKLIST.md`, `docs/RUNBOOK.md`, `docs/DEVNET.md`,
`docs/OPEN_QUESTIONS.md`, `spec/mvp-scope.md`, `spec/architecture.md`,
`spec/wire-protocol.md`, `spec/records.md`, and `spec/state-machines.md`.

Goal:
Audit or repair the closed Milestone 14 launch-gate surface from the current
`milestone-22-first-user-acceptance-pack` repository stage.

Current repository baseline:
- The current repository stage marker is
  `milestone-22-first-user-acceptance-pack`.
- Milestones 0-8 are implemented, vectorized where applicable, validated, and
  considered closed.
- Milestone 9 hardening is implemented and part of the frozen baseline.
- Milestone 10 minimal runtime, Milestone 11 local devnet, and Milestone 12
  launch hardening are implemented and part of the frozen baseline.
- Milestone 14 launch gate and pilot tag are implemented and remain part of the
  landed baseline with a frozen MVP launch surface, a reproducible launch
  checklist, a pilot release template, a documented green-path launch flow,
  and explicit known limitations.

Requirements:
- preserve the frozen launch surface documented in `docs/LAUNCH_CHECKLIST.md`;
- prefer pilot-readiness fixes, validation maintenance, launch-doc updates, and
  conservative regressions over new features;
- keep explicit layering between identity, transport/session, peer/bootstrap,
  rendezvous, relay, routing, and service code;
- keep `README.md`, `HANDOFF.md`, `IMPLEMENT.md`, affected prompts, and
  `docs/OPEN_QUESTIONS.md` synchronized if the launch-gate baseline changes;
- keep known limitations explicit and avoid public-production claims.

Constraints:
- do not add major new protocol features by default;
- do not redesign protocol layers in this stage;
- do not broaden scope beyond the frozen pilot launch surface unless explicitly
  requested;
- prefer reproducible local validation and bounded smoke flows over new
  orchestration layers.

Validation:
- run the applicable commands from `VALIDATION.md`;
- rerun the documented launch gate whenever `REPOSITORY_STAGE`, launch docs, or
  launch scripts change;
- report exactly what passed, what failed, and whether any failure is an
  environment issue rather than a code regression.
