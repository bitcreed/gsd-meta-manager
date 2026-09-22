# Quick 260922-hdh: Show registered projects whose folder is missing in red - Research

**Researched:** 2026-09-22
**Domain:** Rust / ratatui dashboard row rendering + state_reader
**Confidence:** HIGH (all findings read from source this session; no new crates)

## Summary

Right now a registered project whose folder is gone does not error, and nothing skips it. `parse_project_state` quietly produces a placeholder: `status: "unknown"` (magenta), the phase text from `format_phase_display` falls back to `P1: Unknown`, and progress reads `0/0 phases`. That row looks the same as a project whose STATE.md is simply unreadable. The fix needs **no new crates and no registry schema change**. Add a typed presence field to `ProjectState`. `parse_project_state` computes it, and that function already runs on startup load, every watcher reparse, auto-register and create-project. `render_main` then reads the field to color the row red and replace the phase text with a fixed marker.

**Primary recommendation:** Add `pub presence: ProjectPresence { #[default] Present, NoPlanning, FolderMissing }` to `ProjectState`, set in `parse_project_state`. In `render_main`, when it is not `Present`, force `row_color = Color::Red` and render the Phase cell as the `&'static str` `"(missing)"` / `"(no .planning)"`. Optional second task: a presence sweep on the existing 20-tick counter so a folder that disappears (or comes back) while the TUI runs gets noticed.

## Project Constraints (from CLAUDE.md)
- Read state from files only. Do not interfere with running GSD instances. Keep it portable (no hardcoded paths).
- Work goes through a GSD workflow. Logs go to a file (never stdout). Use `tokio::sync::Mutex` in async code.
- Codebase conventions seen here: no new timers (everything rides the 20-tick block in `app.rs:1049-1112`). Blocking fs work goes through `spawn_blocking`. Every reparse goes through `App::schedule_reparse` (`app.rs:599`), which is the only code that increments `reparse_dispatches` (checked by the OBS-06 test). Text drawn in a dashboard badge or cell that the code itself wrote is a `&'static str`.

## Findings

| Question | Answer | Evidence |
|---|---|---|
| Registry load | `config.projects` (`HashMap<String, RegisteredProject{path, added, driver_opt_in, extra}>`). `add_project` checks the path only at registration: `if !path.exists() { bail!("Path does not exist: …") }` and `if !planning_dir.is_dir() { bail!("No .planning/ directory found at: …") }`. `add_project_unchecked` skips the `.planning` check, and the create-project flow relies on that. | [VERIFIED: src/registry.rs:396-462] |
| State load / refresh | `App::load_project_states` (`app.rs:952-959`) calls `state_reader::parse_project_state(&project.path.join(".planning"))` for every project at startup (`main.rs:497`). Refreshes come only from watcher `FileChanged` → `schedule_reparse` (`app.rs:1161`, `spawn_blocking`) → `Action::ProjectStateLoaded`, which uses `ProjectState: PartialEq` to detect change (`app.rs:1174-1196`). **No manual refresh key exists** (`'r'` in `normal.rs:472` is an experimental driver key). | [VERIFIED: src/app.rs:599-614, 952-959, 1120-1196] |
| Missing path today | Nothing guards it. `parse_project_state` sets `status: "unknown".to_string()`, and every `read_to_string` just fails silently, leaving defaults. `project_root = planning_dir.parent()`. | [VERIFIED: src/state_reader/mod.rs:375-387] |
| Row render site | `NormalScreen::render_main` (`normal.rs:697-900`). The row style is `Row::new(cells).style(Style::default().fg(row_color))` (l.889) with `row_color = status_color(&status_str)` (l.808). The Status fallback cell uses `.fg(row_color)` (l.835-838). The alias cell is `Span::raw(render_for_terminal(alias))`, so it inherits the row color unless a badge span is present. `format_phase_display` (`app.rs:405`) + `render_for_terminal` produce the Phase cell. | [VERIFIED: src/ui/screens/normal.rs:738-891] |
| Color / theme | **There is no theme module.** Colors are written inline as `Color::…`. Red means blocked or destructive: `StatusCategory::Blocked => Color::Red` (`normal.rs:37`), the needs-human badge `color: Color::Red` (l.201), and the delete prompt `.fg(Color::Red)` (`delete_confirm.rs:97`). Magenta is used for Unknown. `row_highlight_style` = `BOLD \| UNDERLINED` only (l.379), so the selected row keeps its red fg. | [VERIFIED: normal.rs:33-41, 370-381] |
| Render test patterns | `normal.rs` tests use `ctx_with_aliases(&[..])` (l.1183) with **`path: PathBuf::from("/nonexistent").join(alias)`** and `ProjectState::default()`. `render_dashboard_interior/rows(width, rows)` (l.1882-1935) draw with `TestBackend` and read `buffer.cell((x,y)).symbol()`. Color can be asserted through `cell.fg`. `make_planning(files)` (l.1302) builds a temp `.planning/`. | [VERIFIED: normal.rs:1183-1241, 1302-1310, 1882-1935] |
| Delete flow on a missing folder | Works. `do_remove_project` only touches config and in-memory maps: `registry::remove_project` → `save_config` → `let _ = watcher.unwatch(&planning_dir)` (error ignored) → maps cleared. It refuses only while a non-`Dead` driver run is observed. | [VERIFIED: src/ui/screens/delete_confirm.rs:106-200] |
| Watcher on a nonexistent dir | notify 8.2.0 recursive `add_watch` calls `metadata(&path).map_err(Error::io_watch)?`, so a missing path returns `Err`. Every call site already guards with `is_dir()` (`main.rs:526`, `app.rs:1165`, `app.rs:1260`) or logs a warning (`app.rs:992`, auto-register only runs when `.planning` is a directory). **Consequence:** a project that is missing at startup is never watched, so it will not refresh by itself if the folder comes back. | [VERIFIED: ~/.cargo/registry/.../notify-8.2.0/src/inotify.rs:400-404; src/watcher.rs:113-120; src/main.rs:522-535] |
| Auto-registration (noted, not fixed) | **Yes, the app auto-registers projects.** At startup (`main.rs:544-547`) and every ~5s (`Action::SessionsDetected` → `auto_register_new_sessions`, `app.rs:1221-1225`, `972-1015`), `registry::auto_register_from_sessions` (`registry.rs:877`) registers any active Claude session `working_dir` that contains `.planning/` and saves config. It de-duplicates by canonical path (`canon_or_raw`, which falls back to the raw path when canonicalize fails). A deleted-then-removed entry can therefore come back if a Claude session is still open in a re-created folder. | [VERIFIED: src/registry.rs:867-930; src/app.rs:968-1015] |

## Recommended Design (minimal)

1. **`src/state_reader/mod.rs`:** add `#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)] pub enum ProjectPresence { #[default] Present, NoPlanning, FolderMissing }` and a field `pub presence: ProjectPresence` on `ProjectState`. At the top of `parse_project_state`, set it from `project_root.is_dir()` and then `planning_dir.is_dir()`. `Default = Present` keeps every `ProjectState::default()` / `..Default::default()` fixture unchanged. I checked all struct literals (queue_md, browser, app, change_tracker, router, render_escape_guard, driver) and they use `..Default::default()`, so adding the field compiles. `PartialEq` makes a presence change count as a state change in `ProjectStateLoaded`. Because it is an enum and not a `String`, the `tests/spawn_seam_guard.rs` `THIRD_PARTY_STRINGS` census is not affected.
2. **`src/ui/screens/normal.rs` `render_main`:** when `state.presence != Present`, set `row_color = Color::Red`. Replace `phase_line` with a `&'static str` const: `MISSING_MARKER = "(missing)"` or `NO_PLANNING_MARKER = "(no .planning)"`. Render the status cell as a red span. Skip the D-R-P-E-V pipeline, milestone and ws suffix, since those are stale defaults. **[INFERRED - audit]** The marker goes in the Phase column (30-35% wide, the widest column) and not as a suffix on the alias. At the <60 tier the alias column is 35%, and `alias (missing)` would clip. The escaped alias stays untouched.
3. **[INFERRED - audit]** `NoPlanning` is also red, but with its own marker. It is a real "not a GSD project any more" state, but it is also briefly true for up to 60s after the create-project flow (`add_project_unchecked` plus the 30×2s poll, `app.rs:1264-1289`). The different text keeps that case from reading as "folder gone".
4. **Optional task 2, live detection:** in the existing 20-tick block (`app.rs:1051`, **no new timer**), use `spawn_blocking` to stat each `(alias, path)` and send a new `Action` that lists the aliases whose presence differs from the cached value. The handler calls `schedule_reparse` for each of them. When the presence goes from missing to `Present`, it also calls `watcher.watch(planning_dir)`. Without this, a folder moved away while the TUI runs is only noticed if inotify happens to emit an event under `.planning/` (rm -rf does; moving a parent directory does not) [ASSUMED: inotify parent-move semantics].
5. Optional: add a red `N missing` span in `summary_spans` (`normal.rs:990`). Today missing projects fall into the uncounted `Unknown` bucket.

## Pitfalls
- **Do not compute `path.exists()` at render time.** Every `normal.rs` fixture uses `/nonexistent/<alias>`, so every existing render and badge test would suddenly show red or missing rows. Keep the check in `parse_project_state` so that fixtures built from `ProjectState::default()` stay `Present`.
- **Badge/row color precedence:** badge spans and the pipeline spans carry their own `fg`, and a span `fg` overrides the row style. The alias text stays red only if it remains `Span::raw`.
- **Task 2 and tests:** app tests that send `Action::Tick` ×20 with `event_tx` set and `/nonexistent` fixture paths would see every project flip to missing and schedule reparses, which moves `reparse_dispatches`. Re-run `cargo test --no-fail-fast` (memory note: fail-fast hides the envelope suites) and check the tick-driven tests in `app.rs` around l.3393-3550.
- **`rtk` filters output.** Use `rtk proxy cargo test …` whenever pass counts matter.
- **Sibling quick tasks in this batch:** hdi (config screen filter) and hdj (codex runtime, probably touching session detection/auto-register) may edit `ui/screens/mod.rs`/`app.rs`. If they run in parallel, expect merge conflicts in `app.rs`.

## Test Plan (existing patterns)
- `state_reader` unit tests: `TempDir` without a `.planning` → `NoPlanning`; a path that was removed → `FolderMissing`; `make_planning` → `Present`.
- `normal.rs`: `ctx_with_aliases(&["gone"])`, set `project_states["gone"].presence = FolderMissing`, render through `TestBackend`, assert that the row text contains `(missing)` and that the alias cells have `fg == Color::Red`. Also assert that a `Present` row is not red (regression guard).
- `delete_confirm.rs` already has a fixture `AppContext` (l.252). Add a test that removes a `FolderMissing` alias and succeeds.

## Security / Environment
No new dependencies, network or input surface. The markers are `&'static str`, and the alias still goes through `render_for_terminal`, so the escape census is not affected. Environment check skipped (code-only change; cargo toolchain already in use). Validation Architecture omitted (`workflow.nyquist_validation: false`).

## Assumptions Log
| # | Claim | Risk if wrong |
|---|---|---|
| A1 | Moving a project's parent directory produces no inotify event under the `.planning` watch | Low: it only decides whether task 2 is needed |
| A2 | Red for `NoPlanning` is acceptable despite the ≤60s create-project window | Low: UX preference, marked for audit |
| A3 | Phase column, not an alias suffix, holds the marker | Low: UX preference, marked for audit |
