---
quick_id: 261118-u5i
status: complete
commits:
  - 59930419 test: give the debug-sessions workstream the phases/ dir gsd-core always scaffolds
---

# Quick 261118-u5i Summary: debug-sessions fixture workstream phases/

## Root cause (verified)
- The fixture's `workstreams/a/` had ROADMAP.md and STATE.md but no `phases/`.
  The harness runs the oracle once per workstream (`GSD_WORKSTREAM=a`), and
  `cmdAuditUat` (gsd-core 1.21.0 `bin/lib/uat.cjs:106-107`) errors when the
  planning dir has neither `phases/` nor archived phase dirs. Reproduced by hand:
  the root run returns JSON, the `GSD_WORKSTREAM=a` run fails with that message.
- Real projects never have that layout: `workstream create` always ensures
  `<ws>/phases` (`bin/lib/workstream.cjs:163`); the `handoff-workstreams` fixture
  already models it.
- Not oracle drift: the invoked oracle is `~/.claude/gsd-core/bin/gsd-tools.cjs`
  (VERSION 1.21.0), its `bin/lib/` is byte-identical to the published
  `@opengsd/gsd-core@1.21.0` tarball (only a vendor README differs), and the
  baseline was already 1.21.0 when the fixture landed in 948c4f55.
- Not a product divergence: our scan already matches the oracle once the
  fixture is realistic, so no `no-phases-dir`-style declared divergence applies.

## Fix
Added `workstreams/a/phases/03-stream/03-01-PLAN.md`. Test unchanged.

## Results
- owner_audit_conformance 3/3, owner_open_conformance 4/4
- `cargo test --no-fail-fast`: 4588 passed, 0 failed, 19 ignored (69 suites)
- `cargo clippy --all-targets -- -D warnings`: clean
