Read `AGENTS.md`, `IMPLEMENT.md`, `VALIDATION.md`, `README.md`,
`docs/FIRST_USER_ACCEPTANCE.md`, `docs/PILOT_RUNBOOK.md`,
`docs/PRODUCTION_CHECKLIST.md`, `docs/KNOWN_LIMITATIONS.md`, and
`docs/OPEN_QUESTIONS.md`.

Goal:
Work conservatively from the current repository stage,
`milestone-28-production-gates-packaging-safety-hardening`.

Current repository baseline:
- Milestones 0-27 are already landed baseline work.
- Milestone 28 adds a bounded production gate above the existing acceptance
  pack, reproducible release packaging, install verification, a longer bounded
  soak, and synchronized bounded production claim docs.

Constraints:
- do not add new protocol layers;
- do not widen the bounded production claim beyond what the current gate and
  off-box evidence actually validate;
- do not hide limitations;
- keep changes minimal and local.

Current green path:
- run the applicable commands from `VALIDATION.md`;
- run `./devnet/run-production-gate.sh` on the same commit;
- use `docs/PILOT_RUNBOOK.md` for separate-host evidence;
- generate the ship artifact with `./devnet/package-release.sh` only after the
  same commit is validated.
