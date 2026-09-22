---
phase: quick-260922-hdh
plan: 01
subsystem: dashboard / state_reader
tags: [dashboard, state_reader, presence, ux]
status: complete
requires: []
provides:
  - ProjectPresence enum (Present default, NoPlanning, FolderMissing) on ProjectState
  - MISSING_FOLDER_LABEL "(missing)" / NO_PLANNING_LABEL "(no .planning)" phase-cell markers
  - red dashboard row for any presence other than Present
affects: [src/state_reader/mod.rs, src/app.rs, src/ui/screens/normal.rs, src/ui/screens/delete_confirm.rs]
tech-stack:
  added: []
  patterns: [presence decided at parse time never at render time, authored &'static str markers routed through format_phase_display]
key-files:
  created: []
  modified:
    - src/state_reader/mod.rs
    - src/app.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/delete_confirm.rs
decisions:
  - "ID-1 [INFERRED - audit]: marker goes in the Phase cell, not as an alias suffix"
  - "ID-2 [INFERRED - audit]: NoPlanning is also red, with its own (no .planning) marker"
  - "ID-3 [INFERRED - audit]: no periodic presence sweep; presence refreshes at startup and on watcher reparse"
  - "ID-4 [INFERRED - audit]: no 'N missing' count in the footer summary"
completed: 2026-09-22
commits: 7
plan_head_before: 9ef0f7f
actuals:
  tasks: 3
  commits: 7
---

# Quick 260922-hdh: Show registered projects whose folder is missing in red Summary

`parse_project_state` now sets a typed `ProjectPresence` using two `is_dir()` checks. The dashboard draws a registered project whose folder is gone as a red row with `(missing)` in the Phase cell. A folder that exists but has no `.planning/` gets a red row with `(no .planning)`. Typing `/missing` filters the dashboard to the missing rows, and the existing `d` then `y` delete flow removes them.

## Commits

| Hash | Subject |
|------|---------|
| c8d79f4 | test(quick-260922-hdh): pin deleted folder -> red (missing) dashboard row, RED |
| ad46015 | feat(quick-260922-hdh): missing project folders render red with a (missing) phase marker |
| 33b973a | test(quick-260922-hdh): pin (no .planning) marker for folders without .planning, RED |
| 9ab818b | feat(quick-260922-hdh): (no .planning) marker for registered folders without .planning |
| 96d482e | test(quick-260922-hdh): pin presence classification and /missing filter |
| f8c9098 | test(quick-260922-hdh): pin removal of a registered project whose folder is gone |
| 0e112b2 | style(quick-260922-hdh): rustfmt the new dashboard render test helpers |

Each RED commit comes before its GREEN feat commit, for both Task 1 and Task 2.

## Tests added (9, all `folder_presence_*`)

- `state_reader`: `folder_presence_is_folder_missing_when_root_is_gone`, `folder_presence_is_no_planning_when_root_exists_without_planning`, `folder_presence_is_present_for_a_real_planning_dir`
- `app`: `folder_presence_no_planning_shows_its_own_marker` (RED at 33b973a), `folder_presence_marker_outranks_unreadable_and_recovered`
- `normal`: `folder_presence_missing_folder_row_renders_red_end_to_end` (RED at c8d79f4; real removed TempDir, then real `parse_project_state`, then real `Screen::render` into a TestBackend, with cells checked by column and fg; a control row built from `ProjectState::default()` stays non-red), `folder_presence_no_planning_row_renders_red_with_its_marker` (RED at 33b973a), `folder_presence_filter_slash_missing_lists_only_missing_rows`
- `delete_confirm`: `folder_presence_unregistering_a_project_whose_folder_is_gone_removes_it`. It passed on the first run, so `do_remove_project` needed no change.

## Gate results

- `rtk proxy cargo test --no-fail-fast`: 2131 passed, 1 failed, 15 ignored across 48 test binaries. The single failure is the known local-only witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. The lib suite ran 1356 passed / 1 failed / 1 ignored. `tests/spawn_seam_guard.rs` passed 38 of 38, so the ProjectState String census is unchanged. All envelope suites ran and passed.
- `rtk proxy cargo clippy --all-targets`: exit 0, with no warnings in the four touched files. The remaining warnings are all pre-existing, in `tests/envelope_*` and `src/browser.rs`.
- `rtk proxy cargo clippy -- -D warnings`: exit 0.
- `cargo fmt --check`: the three hunks from this item are fixed (commit 0e112b2). The repo has about 950 pre-existing fmt diffs that this item did not touch, which is out of scope.

## Inferred decisions (for audit)

- **ID-1 [INFERRED - audit]:** the marker goes in the Phase cell, not as a suffix on the alias. The Phase column is wider (at least 13 cells even at 40 columns), the marker becomes searchable through `format_phase_display`, and it follows the `UNREADABLE_STATE_LABEL` precedent. The alias escape and the badge lookup are left untouched.
- **ID-2 [INFERRED - audit]:** `NoPlanning` is also red, with its own marker `(no .planning)`. One transient is known: right after create-project, the row can show `(no .planning)` for up to about 60s, until GSD writes `.planning/` and the post-create poll reparses. This is documented on `NO_PLANNING_LABEL`.
- **ID-3 [INFERRED - audit]:** there is no periodic presence sweep. Presence refreshes at startup and on every watcher-driven reparse. A folder moved away mid-session with no inotify event under `.planning/` turns red on the next restart or the next event. This is a follow-up candidate (research, Recommended Design 4).
- **ID-4 [INFERRED - audit]:** no "N missing" count was added to the footer summary.
- **Executor-inferred [INFERRED - audit]:** the missing-folder marker comes before the `state_md_unreadable` and `state_md_recovered` branches, as the plan says. `NoPlanning` uses the same order, since it also means there is no STATE.md to be unreadable.

## Side issue, noted and deliberately NOT fixed

Auto-registration (`registry::auto_register_from_sessions`, `registry.rs:877`, driven from `app.rs` roughly every 5s) can re-register an entry the user removed. This happens if a live Claude session is still sitting in a re-created folder that has `.planning/`. No auto-register code was changed.

## Deviations from Plan

- **[Rule 3 - Blocking, trivial]:** the delete_confirm test first used `status_message.as_deref()`, but the field is `Option<(String, Instant)>`. It now uses `as_ref()` and destructures, before the test was committed. The plan was not changed.
- **Formatting:** I added a `style` commit that rustfmts the new test helpers (0e112b2). It is test-only and changes no behavior.

Otherwise the plan was executed as written. No new crates, timers, registry schema changes or auto-registration changes.

## Threat surface

Nothing new beyond the plan's threat model. The markers are `&'static str` and still pass through `render_for_terminal`. `presence` is an enum, so the census is unchanged, and the `is_dir()` stats happen only inside `parse_project_state`.

## Self-Check: PASSED

- All 7 commits are present in `git log` (c8d79f4, ad46015, 33b973a, 9ab818b, 96d482e, f8c9098, 0e112b2).
- The four modified files exist and contain `pub enum ProjectPresence`, `MISSING_FOLDER_LABEL`/`NO_PLANNING_LABEL`, the `ProjectPresence::Present` red override, and `folder_presence_unregistering_a_project_whose_folder_is_gone_removes_it`.
