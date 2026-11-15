---
phase: 37-inbox-screen-and-actions
plan: 03
subsystem: Owner inbox screen, owner actions, multiplexer seam
tags: [inbox-screen, run-command, focus-or-launch, cwe-88]
requires: ["37-02"]
provides:
  - InboxScreen (header, D-02 sections, rows, sort_groups, j/k/Home/End/Page keys, v)
  - AppContext.inbox_view
  - owner_actions (RunPlan, run_plan, run_group, launch_real, focus_status)
  - owner_mux_argv, owner_terminal_argv
  - D-21 v on the detail Owner block (DetailScreen::owner_run)
affects: [37-04, 37-05, 37-06, 37-07]
key-files:
  created:
    - src/ui/screens/inbox.rs
    - src/ui/screens/owner_actions.rs
  modified:
    - src/state_reader/owner_items/mod.rs
    - src/state_reader/owner_items/rules.rs
    - src/inbox.rs
    - src/ui/screens/owner_block.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/help.rs
    - src/multiplexer/mod.rs
    - src/app.rs
    - tests/focus_entry_guard.rs
    - README.md
key-decisions:
  - "D-11: v builds the command only from the typed RunCommand and re-validates the phase token (verbs that take one) and the workstream at launch"
  - "D-12: a live session at exactly the project path is focused (focus_message) and the command is shown to type; nothing is injected"
  - "D-13: the multiplexer argv is [env, claude, <command>] with the command as ONE element; the GUI argv is exactly three elements"
requirements-completed: [OWN-21]
requirements-advanced: [OWN-20, OWN-23]
duration: 75 min
completed: 2026-11-14
status: complete
commits: 2
plan_head_before: 6a3e30b0e8004e614d6478dea3621dcce9aa31d8
plan_head_after: ada374ec0a1c6c97b1644329163fd0bd9c7cf9d3
actuals:
  tokens: 50000
  tasks: 2
  commits: 2
---

# Phase 37 Plan 03: Inbox screen and v Summary

`i` opens the Owner inbox, `j` moves, and `v` starts `/gsd-verify-work N` in the right project through `multiplexer::launch` (or focuses the project's live session and shows the command), with the argv built from the typed, re-validated `RunCommand`; the same `v` runs on the detail Owner block.

## Accomplishments

- Task 1 (d5fa7ffc): `InboxScreen` over `AppContext.inbox_view` with the D-02 header and sections, rows built from the Owner block's own `current_sources` / `row_cells`, the total-order `sort_groups` (D-05), identity cursor, scroll that keeps a row's section title with it, and the status/footer line. `owner_actions` with the refusal table, exact-path live-session match and `launch_real`. `owner_mux_argv` / `owner_terminal_argv` beside `mux_launch_argv`. The `i` arm pushes the inbox (interim message and its test gone). `InboxScreen` census row and hostile fixtures in `render_escape_guard`; `owner_actions.rs` joins `FOCUS_CALLERS`.
- Task 2 (ada374ec): `DetailScreen::owner_run` and the `v` arm of `handle_owner_block_key` (fold line, BLOCKED and vanished rows get their own copy); GUI argv pin at three elements; `owner_block_swallows_action_keys` narrowed to `e o A y x`; help `i` row and README `i` row name the inbox.
- Owner block pin: `owner_block_output_is_pinned` green before and after every edit; its goldens were not touched (`git rev-list --count` over them prints 1).

## Deviations from Plan

None in behavior. Process notes:
- [Inferred] The header ladder is a three-rung first cut: `{N} projects` is dropped before the lane keys shorten past `[1]T [2]D [3]F [4]B [a]ll`, matching the UI-SPEC 100-column mockup; 37-04 owns the full ladders.
- [Inferred] `InboxScreen::run_key` looks the group up with `owner_block::group_for` as the key is pressed and falls back to the row's own group only for a BLOCKED aggregate, which `run_plan` refuses by lane. This gave `group_for` its production caller in Task 1 (clippy `-D warnings`).
- [Inferred] `AppContext.inbox_view` joined the `prune_driver_maps` destructuring test as a non-alias-keyed field (a vanished project's row just stops resolving).
- [Inferred] OWN-20 and OWN-23 are advanced, not checked off: filters, expansion, g/o/y and the README section land in 37-04..37-07.
- [Inferred] The Debug and ResumeWork verbs are validated for the workstream only; the phase token is checked for the verbs that spell one (`VerifyWork`, `PlanPhaseGaps`), because `Display` ignores a phase on the others.

**Total deviations:** 0 auto-fixed.

## Verification

- `cargo test --locked --no-fail-fast --all-targets`: 4311 passed, 0 failed, 19 ignored.
- `cargo clippy --all-targets -- -D warnings`: clean; also clean for `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`.
- Human UAT (D-23 item 2, in /gsd-verify-work): `v` from the inbox under tmux and herdr, and focusing a live session.

## Known Stubs

None. The Command column, the filters, the fold line, expansion and the empty states are the later plans' scope, not stubs of this one.

## Self-Check: PASSED

src/ui/screens/inbox.rs and src/ui/screens/owner_actions.rs exist; commits d5fa7ffc and ada374ec are ancestors of HEAD.
