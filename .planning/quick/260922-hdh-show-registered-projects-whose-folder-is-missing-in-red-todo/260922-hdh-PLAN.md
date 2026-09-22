---
phase: quick-260922-hdh
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/mod.rs
  - src/app.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/delete_confirm.rs
autonomous: true
requirements: [QUICK-260922-hdh]

must_haves:
  truths:
    - "A registered project whose folder no longer exists is drawn on the dashboard with a red foreground (Color::Red) on its alias cell and its Phase cell."
    - "That row's Phase cell reads the literal '(missing)', so a user without color can still read the state."
    - "A registered project whose folder exists but has no .planning/ directory is also drawn red, and its Phase cell reads '(no .planning)'. That keeps it distinguishable from a folder that is gone."
    - "Presence is decided inside state_reader::parse_project_state, which runs at startup and on every watcher reparse. It is never decided at render time: a row built from ProjectState::default() with a /nonexistent fixture path stays non-red and shows no marker."
    - "Typing /missing in the dashboard filter lists the rows whose folder is gone, because the marker comes from format_phase_display, which the filter already matches against."
    - "A project whose folder is gone can be unregistered through the existing delete flow ('y' on DeleteConfirmScreen) with no error. Its config entry and its project_states entry are both removed."
    - "cargo test --no-fail-fast is green except for the known local-only failure the_config_section_constants_record_the_git_version_they_were_derived_against (src/envelope/policy.rs). tests/spawn_seam_guard.rs, which counts ProjectState String fields, still passes."
  artifacts:
    - path: "src/state_reader/mod.rs"
      provides: "the ProjectPresence enum (Present default, NoPlanning, FolderMissing), the ProjectState.presence field, the presence computation in parse_project_state, and the three presence unit tests"
      contains: "pub enum ProjectPresence"
    - path: "src/app.rs"
      provides: "the MISSING_FOLDER_LABEL and NO_PLANNING_LABEL constants and the presence branch at the top of format_phase_display"
      contains: "MISSING_FOLDER_LABEL"
    - path: "src/ui/screens/normal.rs"
      provides: "row_color forced to Color::Red for any presence other than Present, plus the end-to-end, NoPlanning and filter tests"
      contains: "ProjectPresence::Present"
    - path: "src/ui/screens/delete_confirm.rs"
      provides: "a test that pins removal of a project whose folder is gone"
      contains: "folder_presence_unregistering_a_project_whose_folder_is_gone_removes_it"
  key_links:
    - from: "state_reader::parse_project_state (src/state_reader/mod.rs:375)"
      to: "ProjectState.presence"
      via: "project_root.is_dir() then planning_dir.is_dir(), evaluated right after project_root is resolved"
      pattern: "ProjectPresence::FolderMissing"
    - from: "app::format_phase_display (src/app.rs:405)"
      to: "MISSING_FOLDER_LABEL / NO_PLANNING_LABEL"
      via: "the first statement of the function returns the marker when presence is not Present, ahead of the unreadable and recovered branches"
      pattern: "NO_PLANNING_LABEL"
    - from: "NormalScreen::render_main (src/ui/screens/normal.rs:808)"
      to: "Row style fg"
      via: "row_color = Color::Red when the state's presence is not Present. The Status fallback cell (l.835-838) and the Span::raw alias cell inherit it."
      pattern: "Color::Red"
    - from: "AppContext::recompute_filtered_aliases (src/ui/screens/mod.rs:1827,1836,1847)"
      to: "format_phase_display"
      via: "an existing call, left unchanged. It is what makes '/missing' find the rows."
      pattern: "format_phase_display"
---

<objective>
Draw registered projects whose folder has gone missing in red on the dashboard, with a "(missing)" marker in the Phase cell that can be read without color. Projects whose folder exists but has no `.planning/` get red and "(no .planning)". Users can then spot these stale entries and remove them with the existing delete flow (`src/ui/screens/delete_confirm.rs`).

Purpose: `add_project` checks the path only once, at registration (`src/registry.rs:396-462`). After that, a moved or deleted project reads as a magenta "P1: Unknown / 0/0 phases" row, which looks exactly like a project whose STATE.md is unreadable. Source: todo `.planning/todos/pending/2026-09-22-show-registered-projects-whose-folder-is-missing-in-red.md`.

Output: a typed `ProjectPresence` on `ProjectState`, computed where state is parsed. Two `&'static str` markers are routed through the existing `format_phase_display`. One row-color override. Four new test groups (state_reader, app, normal, delete_confirm). No new crates, no registry schema change, no new timer.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@.planning/todos/pending/2026-09-22-show-registered-projects-whose-folder-is-missing-in-red.md
@.planning/quick/260922-hdh-show-registered-projects-whose-folder-is-missing-in-red-todo/260922-hdh-RESEARCH.md

<interfaces>
Extracted from source during planning. Use these directly; you do not need to explore the codebase for them.

- `src/state_reader/mod.rs:14-142`: `#[derive(Debug, Clone, Default, PartialEq)] pub struct ProjectState { ... }`. Every construction site in src/ and tests/ uses struct-update syntax (`..Default::default()` / `..ProjectState::default()`); this was checked during planning, including `src/ui/screens/driver.rs:3982`. A new field with a `Default` impl therefore compiles everywhere.
- `src/state_reader/mod.rs:375-387`: `pub fn parse_project_state(planning_dir: &Path) -> ProjectState` builds `ProjectState { status: "unknown".to_string(), ..Default::default() }`, then sets `state.project_root = planning_dir.parent()...unwrap_or_else(|| planning_dir.to_path_buf())`, then calls `git_ops::project_last_activity(&state.project_root)`. Every later read is an `if let Ok(..) = read_to_string` and fails silently on a missing folder. The workstream sub-parse (`src/state_reader/workstreams.rs:56`) calls this with `.planning/workstreams/<ws>`. Its parent exists, so it will read Present.
- `src/state_reader/mod.rs:611-631`: test module with `use tempfile::TempDir;` and the `make_planning(files) -> TempDir` fixture, which creates `<td>/.planning/...`.
- `tests/spawn_seam_guard.rs:708-745`: `UNTRUSTED_STRUCTS` counts every `String` / `Option<String>` / `Vec<String>` field of `ProjectState`. A field typed as an enum is not counted, and that is the reason `presence` must be an enum.
- `src/app.rs:368-416`: `pub const UNREADABLE_STATE_LABEL: &str = "! STATE.md unreadable";`, `pub const RECOVERED_STATE_MARKER: &str = "~ ";`, and `pub fn format_phase_display(state: &ProjectState) -> String`. The function checks `state_md_unreadable` first, then `phase_display_label`, then the recovered prefix. Its tests are at `src/app.rs:2165+` (`use super::*;`, `ProjectState { .., ..Default::default() }` literals).
- `src/ui/screens/normal.rs:767-772`: the phase cell is `crate::text::render_for_terminal(&format_phase_display(s)).into()`, which is already escaped. l.808: `let row_color = status_color(&status_str);`. It is computed BEFORE the status cell. l.835-838: the Status fallback span uses `.fg(row_color)`. l.867: the alias is `Span::raw(alias_read)` and inherits the row style. l.889: `Row::new(cells).style(Style::default().fg(row_color))`. Badge spans carry their own fg and stay as they are.
- `src/ui/screens/normal.rs:1124-1310`: tests use `use super::*;`, `use crate::state_reader::{parse_project_state, ProjectState};`, `TempDir`, `ctx_with_aliases(&[..]) -> AppContext`. The fixtures register `path: PathBuf::from("/nonexistent").join(alias)` with `ProjectState::default()`, event_tx None, and call `recompute_filtered_aliases`. `search(&mut NormalScreen, &mut AppContext, text)` types `/text` + Enter through the real handler. `NormalScreen::render(frame, area, ctx)` (l.627) needs area >= 40x8.
- `src/ui/screens/render_escape_guard.rs:1989-2020`: the pattern for rendering a real `Screen::render` into a `ratatui::backend::TestBackend` and reading `buffer.cell((x, y))`.
- `src/ui/screens/delete_confirm.rs:212-305`: tests have `ALIAS = "proj"`, `ctx_with_project(root) -> (AppContext, rx)` (registers ALIAS at `root` with a REAL `config_path = root.join("config.json")`, so `save_config` writes), and `confirm_removal(&mut ctx)`, which presses 'y'. `do_remove_project` (l.106-210) touches only config and in-memory maps. `watcher.unwatch` errors are ignored.
</interfaces>
</context>

<inferred_decisions>
The human is unavailable. The decisions below were inferred from the artifacts and are marked **[INFERRED - audit]** for later review.

- **ID-1: where the marker goes: the Phase cell, not a suffix on the alias.** The todo says "Consider a short '(missing)' suffix". The marker text is kept exactly; only the column differs. Evidence:
  (a) `dashboard_columns` (`normal.rs:331-363`) gives Alias 25% at >=80 cols, which is about 19 cells on an 80-col terminal, so `<alias> (missing)` clips the marker for any alias longer than about 9 chars. The Phase column is 30-35%, at least 13 cells even at the 40-col floor, which always fits the 9-char `(missing)`.
  (b) `format_phase_display` also feeds the dashboard filter (`screens/mod.rs:1827,1836,1847`), so `/missing` lists every missing project. A suffix drawn only at render time would not be searchable.
  (c) Precedent: `UNREADABLE_STATE_LABEL` (`app.rs:373`) already replaces the phase cell when the phase is unknown.
  (d) The alias cell stays untouched, so its `render_for_terminal` escape and `row_badge` raw-key lookup (CR-01) are unaffected.
- **ID-2: `NoPlanning` is red as well, with its own marker `(no .planning)`.** The todo says "(and ideally its `.planning/` dir)". The different text keeps it from reading as "folder gone". Known transient: after create-project, the row shows `(no .planning)` for up to about 60s, until GSD writes `.planning/` and the existing 30x2s poll (`app.rs:1264-1289`) triggers the reparse.
- **ID-3: no periodic presence sweep.** "On load/refresh" maps to `App::load_project_states` at startup (`main.rs:497`) plus every watcher-driven `schedule_reparse` (`app.rs:599`). The app has no manual refresh key. A folder moved away mid-session with no inotify event under its `.planning/` shows red on the next restart or the next event. A sweep on the 20-tick block would change `reparse_dispatches` in tick-driven app tests (OBS-06), which is out of proportion for this item. It is a follow-up candidate; see research, Recommended Design 4.
- **ID-4: no "N missing" count in the footer summary.** The todo does not ask for one.
- **Side issue, noted and deliberately NOT fixed:** auto-registration (`registry::auto_register_from_sessions`, `registry.rs:877`, driven from `app.rs:972-1015` every ~5s) can re-register a removed entry if a live Claude session sits in a re-created folder that has `.planning/`. Record it in the SUMMARY; do not change `registry.rs` or `app.rs` auto-register code.
</inferred_decisions>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: End to end: a deleted project folder renders a red "(missing)" dashboard row</name>
  <files>src/state_reader/mod.rs, src/app.rs, src/ui/screens/normal.rs</files>
  <behavior>
    - Test `folder_presence_missing_folder_row_renders_red_end_to_end` in the `normal.rs` test module. Create `<TempDir>/vanished/.planning/STATE.md`, then `fs::remove_dir_all(<TempDir>/vanished)` so the folder genuinely disappears. Build `ctx_with_aliases(&["vanished", "intact"])`, point `ctx.config.projects["vanished"].path` at the removed folder, and set `ctx.project_states["vanished"] = parse_project_state(&removed.join(".planning"))`. Render `NormalScreen::new()` through `Screen::render` into a `TestBackend` of 100x12 (the render_escape_guard pattern).
    - Expected: the buffer row holding "vanished" also holds "(missing)", and every cell of "vanished" and of "(missing)" on that row has foreground `Color::Red`.
    - Control on the same buffer: the "intact" row (its state is `ProjectState::default()` and its fixture path `/nonexistent/intact` does not exist) contains no "(missing)", and its alias cells are NOT `Color::Red`. This proves presence comes from parse and not from a stat at render time.
    - Locate text by CELL COLUMN, never by byte offset: the border glyph is multi-byte. Build a per-row Vec of (symbol, fg) from `buffer.cell((x, y))`, and read the fg from the cell's `fg` field (or `cell.style().fg` if the pinned ratatui hides the field).
    - RED: the test uses only APIs that exist today (`parse_project_state`, `ctx_with_aliases`, `Screen::render`), so it compiles and fails at runtime, because the row currently shows "P1: Unknown" in magenta. Commit it alone as `test(quick-260922-hdh): pin deleted folder -> red (missing) dashboard row, RED`.
  </behavior>
  <action>
    GREEN, three layers, one path:
    (1) `src/state_reader/mod.rs`: declare `pub enum ProjectPresence` deriving Debug, Clone, Copy, Default, PartialEq, Eq, with variants `#[default] Present`, `NoPlanning`, `FolderMissing`. Give it a doc comment explaining why it is an enum and not a String or bool: it is a classification this crate authors, and the spawn_seam_guard String census must stay unchanged. Add `pub presence: ProjectPresence` to `ProjectState` with a doc line. In `parse_project_state`, immediately after `state.project_root` is resolved and before the `git_ops::project_last_activity` call, set `state.presence` to FolderMissing when `!state.project_root.is_dir()`, else NoPlanning when `!planning_dir.is_dir()`, else Present. Do NOT add an early return: the later reads already fail silently, and a single code path means a future field cannot be skipped by accident. `Default = Present` keeps every existing fixture unchanged. Because `PartialEq` covers the field, a presence flip counts as a state change in `Action::ProjectStateLoaded` (`app.rs:1174-1196`) with no extra wiring.
    (2) `src/app.rs`: next to `UNREADABLE_STATE_LABEL`, add `pub const MISSING_FOLDER_LABEL: &str = "(missing)";` with a doc comment in the style of its neighbours: why a marker and not a count-derived phase guess; readable without color; and matched by the dashboard filter. Make the first statement of `format_phase_display` return `MISSING_FOLDER_LABEL.to_string()` when `state.presence == ProjectPresence::FolderMissing`, ahead of the unreadable and recovered branches.
    (3) `src/ui/screens/normal.rs` `render_main` l.808: compute `row_color` as `Color::Red` when the looked-up state exists and its presence is not `ProjectPresence::Present`, otherwise keep `status_color(&status_str)`. Nothing else changes. The phase cell already goes through `format_phase_display` + `render_for_terminal`, the Status fallback span already uses `row_color`, and the alias `Span::raw` inherits the row style. Do not call `exists()`/`is_dir()` anywhere in `normal.rs`; see the research Pitfall about the `/nonexistent` fixtures. Import `ProjectPresence` where it is used.
    Commit: `feat(quick-260922-hdh): missing project folders render red with a (missing) phase marker`.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib folder_presence</automated>
  </verify>
  <done>`folder_presence_missing_folder_row_renders_red_end_to_end` failed at the RED commit and passes now. The whole lib builds. The RED and GREEN commits exist in that order.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Expand to the "(no .planning)" marker, presence unit tests, and the /missing filter</name>
  <files>src/state_reader/mod.rs, src/app.rs, src/ui/screens/normal.rs</files>
  <behavior>
    - RED (fail until the NoPlanning marker exists; the tests assert the literal text so they compile before the constant does):
      - `app.rs` test `folder_presence_no_planning_shows_its_own_marker`: a `ProjectState` with `presence: ProjectPresence::NoPlanning, ..Default::default()` makes `format_phase_display` return exactly "(no .planning)".
      - `normal.rs` test `folder_presence_no_planning_row_renders_red_with_its_marker`: `ctx_with_aliases(&["bare"])` with `project_states["bare"].presence = NoPlanning`, rendered via `Screen::render` into a TestBackend. The row contains "(no .planning)" and does NOT contain "(missing)", and the alias cells are `Color::Red`.
      Commit these alone: `test(quick-260922-hdh): pin (no .planning) marker for folders without .planning, RED`.
    - Pins. These pass as soon as Task 1 is in; they lock down behavior Task 1 already delivered:
      - `state_reader` tests: `folder_presence_is_folder_missing_when_root_is_gone` (a TempDir child that was never created: presence FolderMissing, `status == "unknown"`, `project_root` equal to that child); `folder_presence_is_no_planning_when_root_exists_without_planning` (TempDir root exists with no `.planning`: NoPlanning); `folder_presence_is_present_for_a_real_planning_dir` (`make_planning(&[("STATE.md", ...)])`: Present).
      - `app.rs` test `folder_presence_marker_outranks_unreadable_and_recovered`: FolderMissing with `state_md_unreadable: true` gives "(missing)"; FolderMissing with `state_md_recovered: true` gives "(missing)" with no "~ " prefix; a default (Present) state gives neither marker.
      - `normal.rs` test `folder_presence_filter_slash_missing_lists_only_missing_rows`: `ctx_with_aliases(&["vanished", "intact"])` with `vanished` set to FolderMissing, then `search(&mut NormalScreen::new(), &mut ctx, "missing")` leaves `ctx.filtered_aliases == ["vanished"]`.
  </behavior>
  <action>
    GREEN: in `src/app.rs`, add `pub const NO_PLANNING_LABEL: &str = "(no .planning)";` beside `MISSING_FOLDER_LABEL`, with a doc comment covering: distinct from "folder gone"; the known about 60s transient after create-project (ID-2); and still red, because the entry is not a working GSD project. Extend the first statement of `format_phase_display` to a match on `state.presence`: FolderMissing returns MISSING_FOLDER_LABEL, NoPlanning returns NO_PLANNING_LABEL, Present falls through to the existing logic unchanged. Leave `render_main` untouched beyond Task 1: its red override already covers NoPlanning. Commit: `feat(quick-260922-hdh): (no .planning) marker for registered folders without .planning`. The pin tests may ride in the same GREEN commit, or go in a following `test(quick-260922-hdh): pin presence classification and /missing filter` commit.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib folder_presence</automated>
  </verify>
  <done>All `folder_presence_*` tests in state_reader, app and normal pass. The two NoPlanning tests were RED at their own commit before the constant existed.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Pin the removal path for a missing folder, then run the full-suite gates</name>
  <files>src/ui/screens/delete_confirm.rs</files>
  <behavior>
    - Test `folder_presence_unregistering_a_project_whose_folder_is_gone_removes_it` in the `delete_confirm.rs` test module. `let dir = tempfile::tempdir()`, then `ctx_with_project(dir.path())`, which keeps `config_path` inside the real temp root so `save_config` really writes. Set `ctx.config.projects[ALIAS].path = dir.path().join("vanished")` (never created) and insert `ctx.project_states[ALIAS] = parse_project_state(&that.join(".planning"))`. Assert as a precondition that its presence is FolderMissing. Then `confirm_removal(&mut ctx)`.
    - Expected: `ALIAS` is gone from `ctx.config.projects` and from `ctx.project_states`, `ctx.error_message == None`, and `ctx.status_message` contains "Removed".
    - This is a pin (research found `do_remove_project` touches only config and in-memory maps). If it fails, fix `do_remove_project` so a missing path never blocks removal, keeping the driver-run guard as its first statement (CR-06), and record the fix as a deviation.
  </behavior>
  <action>
    Add the test above. Import `crate::state_reader::{parse_project_state, ProjectPresence}` inside the test module. Commit: `test(quick-260922-hdh): pin removal of a registered project whose folder is gone`.
    Then run the gates, reading raw output. Redirect to a log file under `target/` and Read it, because piping through grep/tail can let rtk truncate the output and fake a pass-count change (see memory notes):
    (a) `rtk proxy cargo test --no-fail-fast`. The only acceptable failure is `the_config_section_constants_record_the_git_version_they_were_derived_against` in `src/envelope/policy.rs` (local git differs from the pinned 2.55.0). Every other suite, including `tests/spawn_seam_guard.rs` (ProjectState String census) and the envelope suites, must run and pass.
    (b) `rtk proxy cargo clippy --all-targets` must add no new warnings in the four touched files. (c) `rtk proxy cargo clippy -- -D warnings`, the release gate 5, must pass.
    Fix any fallout inside the four declared files only. If fallout lands elsewhere, stop and report it as a deviation rather than widening scope.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib folder_presence_unregistering && rtk proxy cargo test --test spawn_seam_guard && rtk proxy cargo clippy -- -D warnings</automated>
  </verify>
  <done>The removal pin passes. The full `cargo test --no-fail-fast` run's only failure is the known git-version witness. clippy -D warnings is clean. The SUMMARY records ID-1..ID-4 as [INFERRED - audit] and the auto-registration side issue as not fixed.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| registry config -> state_reader | Registered project paths come from the user's config file and are stat'ed with `is_dir()` |
| `.planning/` contents -> dashboard cells | Third-party text (SAFE-07). The new markers are authored `&'static str`, not repository text |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-hdh-01 | Tampering | `format_phase_display` markers / `normal.rs` phase cell | low | mitigate | Markers are `&'static str` constants and still pass through the existing `render_for_terminal` call at `normal.rs:769`. No path or other repository text is added to any cell. The alias cell and its escape stay as they are. |
| T-hdh-02 | Information disclosure | `ProjectState.presence` vs the untrusted-text census | low | mitigate | The field is an enum, so the `tests/spawn_seam_guard.rs` count of ProjectState String fields is unchanged. Task 3 runs that suite explicitly. |
| T-hdh-03 | Denial of service | `is_dir()` on a hung or network-mounted path | low | accept | The stat runs only inside `parse_project_state`. That function already does blocking reads of the same paths at startup and inside `spawn_blocking` on reparse, and the render path never stats. Exposure is one extra syscall per parse. |
| T-hdh-04 | Elevation of privilege | delete flow on a missing folder | low | mitigate | The removal path still touches only config and in-memory maps (it never deletes files on disk), and the driver-run refusal stays the first statement of `do_remove_project` (CR-06). The Task 3 pin asserts that removal succeeds without extending any filesystem reach. |
| T-hdh-05 | Spoofing | dangling symlink as the project root | low | accept | `is_dir()` follows symlinks, so a dangling link correctly reads as FolderMissing, and a live link reads as whatever it points to, which is today's behaviour. |
</threat_model>

<verification>
- `rtk proxy cargo test --lib folder_presence`: all new tests pass (state_reader x3, app x2, normal x3, delete_confirm x1).
- `rtk proxy cargo test --no-fail-fast`: the only failure is the known git-version witness in `src/envelope/policy.rs`.
- `rtk proxy cargo clippy -- -D warnings` is clean, and `rtk proxy cargo clippy --all-targets` adds no new warnings.
- Git log shows the RED test commits ahead of their GREEN feat commits for Task 1 and Task 2.
</verification>

<success_criteria>
- A registered project whose folder was deleted renders red with "(missing)" in the Phase cell. This is proven through a real `parse_project_state` on a removed TempDir and a real `Screen::render` buffer.
- A folder without `.planning/` renders red with "(no .planning)".
- The existing `/nonexistent` render fixtures stay green and non-red, because presence is computed in parse and not at render.
- `/missing` filters the dashboard to the missing rows.
- Such a project is removable via `d` then `y` with no error.
- No new crates, timers, registry schema changes or auto-registration changes.
</success_criteria>

<output>
Create `.planning/quick/260922-hdh-show-registered-projects-whose-folder-is-missing-in-red-todo/260922-hdh-SUMMARY.md` when done. Include the inferred decisions ID-1..ID-4 marked [INFERRED - audit], the auto-registration side issue (noted, not fixed), and the exact test-gate results.
</output>
