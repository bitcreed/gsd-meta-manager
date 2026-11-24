---
phase: quick-261124-k1s
plan: 01
subsystem: ui
tags: [owner-inbox, wording, acknowledge, settle]
status: complete
key-files:
  modified:
    - src/ui/screens/owner_row_actions.rs
    - src/ui/screens/owner_settle.rs
    - src/ui/screens/inbox.rs
    - src/ui/screens/owner_actions.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/ack_confirm.rs
    - src/ui/screens/settle_popup.rs
    - src/ui/screens/help.rs
    - README.md
    - docs/GETTING-STARTED.md
actuals:
  tokens: 30000
  tasks: 3
  commits: 3
plan_head_before: d79e0c2367b2dc02030c9934167abff9a39fe31a
plan_head_after: fae6e3f083f6abc504d46eb6d73db5140f8822a9
completed: 2026-11-24
---

# Quick 261124-k1s: Clarify the in-app meaning of Owner-row A and x

One-liner: the Enter menu, the A popup, the x chooser, the help overlay and the docs now say that
`A` hides a file until it changes (nothing recorded, phase stays open) and `x` records checks as
skipped so the phase can close; no status line calls an acknowledged file "handled" any more.

## Commits

- ca937698 fix(quick-261124-k1s): the action menu says A hides and x records as skipped; no refusal calls an acknowledged file handled
- d453e421 fix(quick-261124-k1s): the acknowledge popup, settle chooser and help say A hides and x records as skipped
- fae6e3f0 docs(quick-261124-k1s): README and getting started contrast A (hide) with x (record as skipped)
- c473a9cd test(quick-261124-k1s): pin verify-work's gsd-core 1.23.0 argument parse in the settle oracle (orchestrator, deviation)

## New user-facing strings

- Menu x: `record whole phase as skipped, asks first` / `record this check as skipped, asks first`
- Menu A: `hide this file until it changes, asks first`
- A popup title: `Acknowledge (hide) via gsd-core`
- A popup line: `Hiding resolves nothing: no result is recorded, the phase stays open.` (debug: `Hiding resolves nothing: the investigation stays open.`)
- x chooser row 1: `  Settle {target}: record it as skipped, so the phase can close`
- x chooser row 2: `  [w] won't do  [p] defer permanently  [m] move to phase M  [Esc] cancel`
- x refusal: `Cannot settle an acknowledged file: it is hidden until it changes` (shared const `owner_settle::ACKNOWLEDGED_REFUSAL`)
- y refusal: `Nothing to copy: acknowledged items are hidden until their file changes`
- Help A: `Acknowledge = hide, not resolve: gsd-core marks the group's file and all its items leave the lanes until the file changes; no result is recorded and the phase stays open; uncommitted, asks first`
- Help x: `Settle = record as skipped: the selected check or phase becomes won't do, deferred permanently or moved to phase M (via /gsd-verify-work), so the phase can close; asks first`

## Verification

- `cargo fmt --check`: clean. `cargo clippy --all-targets -- -D warnings`: clean.
- Executor's run (`target/k1s-final.log`): 4690 passed, 1 failed: `tests/owner_settle_oracle.rs::the_installed_verify_work_still_parses_arguments_this_way`,
  because the installed gsd-core 1.23.0 rewrote verify-work's `--ws`/phase-argument parse and the verbatim pin still held 1.21.0's.
- After c473a9cd (orchestrator): full `cargo test --no-fail-fast` (`target/k1s-final2.log`): 69 suites, 4691 passed, 0 failed;
  `cargo clippy --all-targets -- -D warnings` clean.

## Deviations from Plan

- [Rule 1, orchestrator] The settle oracle's verbatim pin of verify-work's argument parse was stale against the installed gsd-core 1.23.0. Updated both pinned lines to 1.23.0's text (c473a9cd); the oracle's parse-and-init cases pass through the new parse, so the settle prompt encoding still holds. The rest of the branch remains synced to 1.21.0 (`GSD_CORE_SYNCED_VERSION`); the full 1.23.0 sync arrives with master's v2.6.0.

## Known Stubs

None.

## Self-Check: PASSED

Commits ca937698, d453e421, fae6e3f0, c473a9cd exist on feat/v2.5.0-owner-inbox; nothing pushed; src, tests, README.md and docs are clean.
