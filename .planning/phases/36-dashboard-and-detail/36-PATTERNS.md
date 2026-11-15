# Phase 36: Dashboard and detail - Pattern Map

**Mapped:** 2026-11-14
**Files analyzed:** 16 (2 new, 14 modified)
**Analogs found:** 16 / 16 (every new surface has an in-repo model; line numbers read live at HEAD `d36f6733`)

All analogs below are git-tracked source in this worktree (`src/`, `tests/`, `README.md`); no
gitignored mirror path is referenced. The binary crate (`src/main.rs`) and the lib crate
(`src/lib.rs`: `pub mod inbox`, `pub mod ui`, `pub mod app`, ...) are separate: `inbox` and `ui`
share the lib crate, so `pub(crate)` on `inbox.rs` helpers is enough for the TUI; anything
`main.rs` calls must be `pub`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/ui/screens/owner.rs` (NEW, pure) | utility (presentation predicates + span builders) | transform | `normal.rs` `agent_summary_line` (519-530) + `summary_spans` (1351-1401); `mod.rs` `needs_human` / `attention_rank` (1459-1491) | role-match |
| `src/ui/screens/owner_block.rs` (NEW) | component (detail sub-block: model, render, keys, view state) | event-driven (keys) + transform (render) | Waves pane in `detail.rs`: `render_waves_pane` (7691-7790), `handle_waves_pane_key` (2060-2098), `waves_move` (1806-1824), `waves_activate` (1830-1855); state in `mod.rs` `ProjectViewCache.waves_cursor/waves_toggles` (1022-1032) | exact |
| `src/action.rs` | model (message enum) | event-driven | `Action::LauncherMarkerScanned` (176-179) and its clone test (483-490); `ProjectStateLoaded` (45-49) | exact |
| `src/app.rs` | controller (update loop, filter grammar, schedulers, prune) | event-driven / request-response | itself: `schedule_reparse` (1000-1019), `ProjectStateLoaded` handler (2832-2854), `LauncherMarkerScanned` handler (3546-3555), `parse_filter` (62-104), `prune_driver_maps` (3574-3605), OBS tests (4531-4657), destructure test (5783-5844) | exact |
| `src/ui/screens/mod.rs` | store (AppContext, ProjectViewCache) + sort/filter service | CRUD (in-memory maps) | itself: `AppContext` fields (1323-1415), `schedule_launcher_marker` (1817-1835), `attention_rank_for` (1870-1878), `recompute_filtered_aliases` NeedsHuman arm (1950-1957), `ctx_with_aliases` (2394-2447), rank test (2857-2880) | exact |
| `src/ui/screens/normal.rs` | component (dashboard screen) | transform (render) + event-driven (keys) | itself: `dashboard_columns` (439-471), `status_column_cells` (486-497), cell vector (1200-1225), `render` split (866), `render_normal_footer` (1403-1433), `s` key (749-762), test helpers (3074-3117, 3346-3349, 3476-3507) | exact |
| `src/ui/screens/detail.rs` | component (detail screen) | event-driven + transform | itself: `DetailFocus` (1161-1181), Pane dispatch (3939-3945), `↑` arm (4255-4267), `render_pipeline_tab` (7456-7526), footer chain (6305-6318), `footer_spans` lead (11335-11344), `waves_pane_footer_spans` (11466-11483), `waves_viewport_rows: Cell<u16>` (838, 1205) | exact |
| `src/inbox.rs` | utility (wording) | transform | itself: `plural` (255-257), `blocked_tally` (260-288), `detail` (290-381), `push_closes_after` (496-543), `Filters::lists_group` (69-77), `by_location` (473-479) | exact (visibility bump + one extraction) |
| `src/main.rs` | config (startup wiring) | request-response | itself 526-538 (`load_project_states` then `event_tx = Some(..)`) | exact |
| `src/ui/screens/add_project.rs` | controller (registration screen) | CRUD | itself 204-212 (insert state, recompute) | exact |
| `src/ui/screens/help.rs` | component (static copy) | transform | itself: `heading`/`row` (189-198), Keybindings (215-234), Filter Syntax (448-462), whole-row test (679-...) | exact |
| `src/ui/screens/render_escape_guard.rs` | test (safety guard) | batch | itself: `probe_ctx` (1093-...), `"NormalScreen"` states (2048-2093), `"DetailScreen"` (2114-2118), `PROBE_WIDTH = 200` (2700) | exact |
| `src/ui/screens/driver_confirm.rs`, `delete_confirm.rs` (test literals), `app.rs:932`, `detail.rs:20030`, `normal.rs:1642`, `mod.rs:2411` | test fixture / config | n/a | the six `AppContext { .. reparse_dispatches: 0, .. }` literals | exact (mechanical) |
| `README.md` | config (docs) | n/a | `README.md` 249-362 (Key bindings tables), 540 `#### Owner work: \`inbox\`` | exact |
| `tests/async_blocking_guard.rs` | test | n/a | already lists `"read_owner_digest("` (132-137) | no change needed |
| `src/status_cli.rs` | (reference only) | n/a | `owner_json` (213-223): the numbers the dashboard must agree with | reference |

## Pattern Assignments

### `src/ui/screens/owner.rs` (NEW: utility, transform)

**Analogs:** `src/ui/screens/normal.rs` (ladder + span builder), `src/ui/screens/mod.rs` (pure predicates).

**Pure predicate style** (`mod.rs` 1474-1491): doc comment says "Pure, so the ordering is testable without a dashboard"; plain bools in, `u8` out.
```rust
pub fn attention_rank(needs_human: bool, driven_and_live: bool) -> u8 {
    if needs_human {
        0
    } else if driven_and_live {
        1
    } else {
        2
    }
}
```
Put `OwnerNumbers`, `owner_numbers(&OwnerDigest)`, `has_owner_work`, `OwnerLane { Test, Decide, Fix }`
(new 3-variant enum, NOT `owner_items::Lane`, which has `Blocked`/`Deferred`, mod.rs 373-379) and
`owner_lane_count(nums, lane)` here so the cell, summary, rank (`mod.rs`) and filter (`mod.rs`) call
one function (D-11). Field sources (`owner_items/mod.rs` 537-553, 504-510):
`test, decide, fix_phases, fix_debug, blocked, closeable_phases, archived.{test,blocked,fix,decide}`.
Counts must match `status_cli::owner_json` (status_cli.rs 213-223).

**Ladder pattern for the `You` cell** (`normal.rs` 519-530): first whole form whose
`Line::width() <= cells` wins; never clip mid-segment.
```rust
fn agent_summary_line(view: &crate::agents::waves::AgentView, cells: u16) -> Option<Line<'static>> {
    let form = view
        .summary_forms()
        .into_iter()
        .find(|form| Line::from(form.as_str()).width() <= usize::from(cells))?;
    let color = if form.ends_with("stalled") { Color::Yellow } else { Color::Cyan };
    Some(Line::from(Span::styled(form, Style::default().fg(color))))
}
```
Differences required by UI-SPEC: every span gets `Style::default().fg(Color::Reset)` explicitly
(the row style `fg(row_color)` at normal.rs 1223-1225 would otherwise tint it); DIM pieces
(`·`, ` (+N old)`) use `fg(Color::Reset).add_modifier(Modifier::DIM)` (DIM idiom =
`driver::muted_style`, driver.rs 120-122, `pub(super)`, reachable from a sibling screen module as
`super::driver::muted_style()`). Final fallback rung is `super::detail::fit_cells(text, cells)`
(detail.rs 10797, `pub(super)`). Separator from one `const OWNER_SEP: &str = " \u{b7} ";`.

**Span-builder pattern for the `You:` row** (`normal.rs` 1351-1401, `summary_spans`): returns
`Vec<Span<'static>>` so tests assert spans, not a buffer; doc explains "nothing derived from prose".
```rust
let mut left_spans: Vec<Span> = vec![Span::raw(format!("{} projects ", all_count))];
if active > 0 {
    left_spans.push(Span::styled(format!("{} active ", active), Style::default().fg(Color::Green)));
}
```
Iterate `ctx.filtered_aliases` (visible rows, D-11), not `ctx.project_states.values()` as
`summary_spans` does. Loading copy reuses `super::driver::DRY_RUN_LOADING` (driver.rs 98,
`pub(super)`, value `"Loading..."`).

**P computation:** `crate::state_reader::phase_num::phase_key(&str) -> String` (phase_num.rs 169);
count only current, unacknowledged `Lane::Test` groups with `!items.is_empty()` (RESEARCH
Correction 6).

**Tests:** in-file `#[cfg(test)] mod tests`, digests built as struct literals with
`..Default::default()` (`OwnerDigest` derives `Default`, owner_items/mod.rs 537), the shape used by
`inbox.rs` tests 762-773:
```rust
digest: OwnerDigest { groups, test: 1, ..Default::default() },
```

---

### `src/ui/screens/owner_block.rs` (NEW: component, event-driven + render)

**Analog:** the Phases-tab Waves pane in `src/ui/screens/detail.rs`.

**Key handler contract** (detail.rs 2053-2098): `Some(action)` consumes, `None` falls through;
digits/`D`/`[`/`]` reset focus to `Content` and fall through; everything else swallowed.
```rust
fn handle_waves_pane_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> Option<ScreenAction> {
    match code {
        KeyCode::Left | KeyCode::Esc => {
            self.focus = DetailFocus::Content;
            ctx.needs_redraw = true;
            Some(ScreenAction::None)
        }
        KeyCode::Char('q') => { ctx.needs_redraw = true; Some(ScreenAction::Pop) }
        KeyCode::Char('j') | KeyCode::Down => Some(self.waves_move(ctx, |i, _, _| i.saturating_add(1))),
        KeyCode::Char('k') | KeyCode::Up => Some(self.waves_move(ctx, |i, _, _| i.saturating_sub(1))),
        KeyCode::PageDown => Some(self.waves_move(ctx, |i, _, page| i.saturating_add(page))),
        KeyCode::PageUp => Some(self.waves_move(ctx, |i, _, page| i.saturating_sub(page))),
        KeyCode::Char('g') => Some(self.waves_move(ctx, |_, _, _| 0)),
        KeyCode::Char('G') => Some(self.waves_move(ctx, |_, n, _| n.saturating_sub(1))),
        KeyCode::Enter | KeyCode::Char(' ') => Some(self.waves_activate(ctx)),
        KeyCode::Char('e') => self.waves_edit(ctx),
        KeyCode::Right => Some(ScreenAction::None),
        KeyCode::Char('?') | KeyCode::Tab => None,
        KeyCode::Char(c) if c.is_ascii_digit() || matches!(c, 'D' | '[' | ']') => {
            self.focus = DetailFocus::Content;
            ctx.needs_redraw = true;
            None
        }
        _ => Some(ScreenAction::None),
    }
}
```
Owner-block deltas (UI-SPEC Interaction table): `Esc` → `Content`; `↓` past last stop → `Content`
(leave `pipeline_selected` at 0); `↑` from first stop → `TabBar`; `←`/`→` move the fold sub-cursor;
`e`, `o`, `v`, `A`, `y` must be `Some(ScreenAction::None)` (Pitfall 11; `e` would otherwise reach
Content's enqueue). Because the handler mutates `DetailScreen.focus`, either keep it as a
`DetailScreen` method in `detail.rs` that delegates model work to `owner_block.rs`, or have the
`owner_block.rs` helper return an enum the `detail.rs` method maps to focus changes.

**Cursor move with identity, clamped** (detail.rs 1806-1824): page size comes from a `Cell<u16>`
the render writes; the stored cursor is an identity, not an index.
```rust
let page = usize::from(self.waves_viewport_rows.get()).max(1);
let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
let current = waves_cursor_index(&model, &rows, cache.waves_cursor.as_ref());
let next = to(current, rows.len(), page).min(rows.len() - 1);
cache.waves_cursor = Some(rows[next].target.clone());
ctx.needs_redraw = true;
```

**Toggle set pattern** (detail.rs 1847-1850): `if !cache.waves_toggles.remove(&key) { insert }`.
Expanded-group set and open-fold set follow this. Group key = owned struct
`(lane, kind, scope label, workstream, phase, file)` deriving `Hash, Eq` (`OwnerGroup` itself does
not derive `Hash`, owner_items/mod.rs 468).

**View state on `ProjectViewCache`** (mod.rs 1022-1032): add one field, e.g.
`pub owner_block: OwnerBlockState` with `#[derive(Default)]`, documented like `waves_cursor`
("a row IDENTITY, never an index, so it survives refreshes"). `ProjectViewCache` is
`#[derive(Default)]` (mod.rs 986), so existing literals using `..ProjectViewCache::default()` keep
compiling.

**Render with scroll window and recorded viewport** (detail.rs 7699-7756): early return on zero
area; compute `(offset, visible)` from a stored offset `Cell` and the focus row; write the offset and
viewport back to `Cell`s on `&self`.
```rust
let (offset, visible) = waves_window(rows.len(), height, self.waves_offset.get(), focus_row, focused);
self.waves_offset.set(offset);
self.waves_viewport_rows.set(visible.max(1) as u16);
```
Escaping/truncation inside the block: every project string through `shown()` (detail.rs 81-83,
currently private: bump to `pub(super)`), width via `fit_cells`. Workstream cue: `Span::styled(..,
Style::default().fg(Color::Magenta))` as at detail.rs 7552-7557. Cursor row: `BOLD + Color::Cyan`
as the Phases list highlight (detail.rs 7568-7573). Fold glyphs: `crate::ui::glyphs::TRIANGLE_RIGHT_SMALL`
/ `TRIANGLE_DOWN_SMALL` (glyphs.rs 88, 91); never literal `▸`/`▾` under `src/ui/` (source scan,
glyphs.rs 483-503). Header rule `──` (U+2500, structural range).

**Wording:** call `crate::inbox::{detail, blocked_tally, plural}` and the extracted closes-after
clause builder (see `src/inbox.rs` below), never re-spell. Lane membership for body rows / folds
mirrors `render_text` (inbox.rs 602-629):
```rust
let archived = sorted(&|r| matches!(r.group.scope, Scope::Archived { .. }));
let acknowledged = sorted(&|r| {
    matches!(r.group.scope, Scope::Current)
        && r.group.lane != Lane::Deferred
        && r.group.acknowledged
});
```

**Tests:** follow `waves_pane_focus_in_and_out` (detail.rs 29277-29326) and
`waves_pane_keys_never_reach_the_phase_list` (29328-29349): `press(&mut screen, &mut ctx, code)`
(detail.rs 20131), `assert_eq!(screen.focus, DetailFocus::..)`, footer read as the last line of
`render_detail_to_text_at(&screen, &ctx, w, h)` (detail.rs 22144-22170).

---

### `src/action.rs` (model, event-driven)

**Analog:** `ProjectStateLoaded` (45-49) and `LauncherMarkerScanned` (174-179).
```rust
ProjectStateLoaded {
    alias: String,
    // Boxed: ProjectState dwarfs every other variant (clippy::large_enum_variant)
    state: Box<ProjectState>,
},
```
Add `digest: Box<OwnerDigest>` (boxed, same lint) and a new
`OwnerDigestLoaded { alias: String, digest: Box<OwnerDigest> }` with a doc comment in the
`LauncherMarkerScanned` style ("Plain data, so `Action` stays `Clone`"). Extend the clone test
(action.rs 483-490):
```rust
let scanned = Action::LauncherMarkerScanned { alias: "proj".to_string(), marker: ...::default() };
assert!(matches!(scanned.clone(), Action::LauncherMarkerScanned { .. }));
```

---

### `src/app.rs` (controller, event-driven)

**Filter grammar** (62-104): new variant documented like `NeedsHuman` ("a **predicate**, not a
column"); in `parse_filter`, check long suffixes before `/y` (`"/y test"`, `"/y decide"`, `"/y fix"`,
`"/yt"`, `"/yd"`, `"/yf"`, then `"/y"`), same `strip_suffix` chain:
```rust
} else if let Some(term) = input.strip_suffix("/h") {
    (term.to_string(), FilterColumn::NeedsHuman)
} else {
    (input.to_string(), FilterColumn::All)
}
```
`FilterColumn` derives `Debug, Clone, PartialEq` only, so `OwnerLane` needs at least those.

**Reparse task** (1000-1019): read `stale_days` before the closure; compute the digest inside the
existing `spawn_blocking` after `parse_project_state`; keep `reparse_dispatches += 1` before the
spawn (doc at 998-999 explains why).
```rust
self.ctx.reparse_dispatches += 1;
tokio::task::spawn_blocking(move || {
    let state = state_reader::parse_project_state(&planning_dir);
    let _ = tx.send(Action::ProjectStateLoaded {
        alias: alias_for_task,
        state: Box::new(state),
    });
});
```
`stale_days`: `self.ctx.config.preferences.owner_stale_days()` (config.rs 546; also used by
status_cli.rs 295).

**`ProjectStateLoaded` handler** (2832-2854): split `changed` into `state_changed` (existing logic,
drives `change_tracker` and "Updated:") and `digest_changed` (`OwnerDigest: PartialEq`); recompute +
redraw on either, status message only on `state_changed` (Pitfall 1).
```rust
let changed = match self.ctx.project_states.get(&alias) {
    Some(old_state) => {
        self.ctx.change_tracker.detect_changes(&alias, old_state, &state);
        *old_state != *state
    }
    None => true,
};
self.ctx.project_states.insert(alias.clone(), *state);
if changed {
    self.ctx.recompute_filtered_aliases();
    self.ctx.status_message = Some((format!("Updated: {}", alias), std::time::Instant::now()));
    self.needs_redraw = true;
}
```

**`OwnerDigestLoaded` handler (first fill only)** — copy the registered-alias guard and
change-only redraw of `LauncherMarkerScanned` (3546-3555), but insert only when absent
(Pitfall 2: a reparse digest is always newer):
```rust
Action::LauncherMarkerScanned { alias, marker } => {
    if self.ctx.experimental && self.ctx.config.projects.contains_key(&alias) {
        let cache = self.ctx.view_cache.entry(alias).or_default();
        if cache.launcher_marker != Some(marker) {
            cache.launcher_marker = Some(marker);
            self.ctx.recompute_filtered_aliases();
            self.needs_redraw = true;
        }
    }
}
```
(No `experimental` gate for owner digests.)

**First-fill call sites:** auto-register (2568-2570, after `project_states.insert`) and
`CreateProjectResult` success (2960-2961) call `self.ctx.schedule_owner_digest(&alias, &path)`.

**Prune** (`prune_driver_maps`, 3574-3605): add
`self.ctx.owner_digests.retain(|alias, _| registered.contains_key(alias));` next to the
`last_refresh` retain (3603-3605), with a one-line reason comment in the same voice.

**Exhaustive destructure test** (5783-5829): add `owner_digests` to the "Alias-keyed: every one of
these must be pruned" group, plus `assert!(!owner_digests.contains_key(GONE), "owner_digests leaked");`
and a control-arm `assert!(owner_digests.contains_key(OBS_ALIAS));` (seed one before
`app.prune_driver_maps()` at 5779).

**`AppContext` literal** at app.rs 932-950 (`App::from_config` / `new_for_test`): add
`owner_digests: HashMap::new(),`.

**D-04 test** — build on the OBS harness (4531-4636): `obs_app(root)` returns an `App` with a live
`event_tx` and the receiver; `planning_change(root)` builds the `FileChanged`; tests are
`#[tokio::test]` because `spawn_blocking` panics without a runtime (comment at 4528-4529).
```rust
#[tokio::test]
async fn a_planning_write_still_triggers_a_reparse() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    let (mut app, _rx) = obs_app(root);
    let before = app.ctx.reparse_dispatches;
    app.update(planning_change(root));
    assert_eq!(app.ctx.reparse_dispatches, before + 1, "...");
}
```
For D-04: keep `rx`, `rx.recv().await` the `ProjectStateLoaded`, feed it to `app.update`, assert the
cell numbers; flip a UAT row to `pass`, `app.ctx.last_refresh.remove(OBS_ALIAS)` (Pitfall 12: the
500 ms dedup at 2810-2817), send `planning_change` again, receive, update, assert the count dropped.
Fixture: copy a project from `tests/fixtures/owner/` (e.g. `uat-pending`) into the tempdir the way
`tests/owner_digest.rs` `copy_fixture` does (lines 20-43, `env!("CARGO_MANIFEST_DIR")`). The parse
caches are private statics (owner_items/mod.rs 278-290), so assert behavior, not cache length.

---

### `src/ui/screens/mod.rs` (store + sort/filter)

**New field on `AppContext`** (1323-1415): document it like `agent_views` (1346-1368): "A
**sibling map** ... it must not live on `ProjectState`. That type derives `PartialEq` and `app.rs`
uses the derived equality to suppress the 'Updated: {alias}' status message"; state that it is
pruned in `App::prune_driver_maps`.

**AppContext scheduler** — copy `schedule_launcher_marker` (1817-1835) for
`pub fn schedule_owner_digest(&self, alias, project_path)` (must be `pub`: `main.rs` is another
crate):
```rust
pub fn schedule_launcher_marker(&self, alias: &str, project_path: &std::path::Path) {
    if !self.experimental { return; }
    let Some(tx) = &self.event_tx else { return; };
    let tx = tx.clone();
    let alias = alias.to_string();
    let project = project_path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let entries = crate::state_reader::driver_frontmatter::list_driver_runs(&project, LAUNCHER_RUNS_LISTED);
        let marker = driver_run::launcher_marker(&entries, std::time::SystemTime::now());
        let _ = tx.send(crate::action::Action::LauncherMarkerScanned { alias, marker });
    });
}
```
(Drop the `experimental` gate; clone the `ProjectState` from `self.project_states` into the closure;
`read_owner_digest(` must stay inside `spawn_blocking` for `tests/async_blocking_guard.rs`.)

**Rank** (1474-1491 + 1870-1878): `attention_rank` gains a third bool and ranks 0..=3; keep the doc
style. NOTE the argument-order conflict: RESEARCH Pattern 5 says
`(needs_human, has_owner_work, driven_and_live)`, UI-SPEC "Sort" says
`(needs_human, driven_and_live, has_owner_work)`; the planner must pick one and use it in the
rank-table test. `attention_rank_for` keeps the `experimental` gate on `driven_and_live` only:
```rust
fn attention_rank_for(&self, alias: &str) -> u8 {
    let driven_and_live = self.experimental
        && self.view_cache.get(alias)
            .and_then(|cache| cache.launcher_marker)
            .is_some_and(|marker| marker.class == Some(driver_run::MarkerClass::Driven));
    attention_rank(self.needs_attention_for(alias), driven_and_live)
}
```

**Filter arm** (1950-1957): mirror `NeedsHuman` exactly; no digest → no match.
```rust
FilterColumn::NeedsHuman => {
    matches_any_column() && self.needs_attention_for(alias)
}
```

**Module declarations** (1-26): add `pub mod owner;` / `mod owner_block;` (or `pub(crate)`) in the
alphabetical list; neither implements `Screen`, so the `render_escape_guard` screen census is
unaffected.

**Test fixture literal** `ctx_with_aliases` (2394-2447): add `owner_digests: HashMap::new(),`.
Helpers `drive` / `pause` (2449-2468) are the model for a new `owe(ctx, alias, digest)` helper
that inserts into `ctx.owner_digests`.

**Existing rank test to edit** (2857-2880, Pitfall 4): only the three `attention_rank(..)` calls at
2873/2878/2879 change; the `sorted_aliases()` assertion (2864-2870) stays byte-identical.

---

### `src/ui/screens/normal.rs` (dashboard component)

**Width tier** (439-471): add a `terminal_width >= 100` branch BEFORE the `>= 80` one; Status stays
at index 2 (`DASHBOARD_STATUS_COLUMN`, 500); one named `const YOU_COLUMN_CELLS: u16 = 21;` next to
`STATUS_COLUMN_MIN_CELLS` (423).
```rust
fn dashboard_columns(terminal_width: u16) -> (Vec<&'static str>, Vec<Constraint>) {
    if terminal_width >= 80 {
        (
            vec!["Alias", "Phase", "Status", "Progress", "Backlog"],
            vec![
                Constraint::Percentage(25),
                Constraint::Percentage(30),
                Constraint::Min(STATUS_COLUMN_MIN_CELLS),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
            ],
        )
    } else if terminal_width >= 60 { ... }
```
UI-SPEC vector: `Percentage(20), Percentage(26), Min(13), Length(21), Percentage(12), Percentage(9)`.

**Second tier site — the cell vector** (1200-1221): add the matching `>= 100` branch with the `You`
cell at index 3; the cell width for the ladder comes from re-running the layout like
`status_column_cells` (486-497) for index 3.
```rust
let cells: Vec<TableCell> = if terminal_width >= 80 {
    vec![
        TableCell::from(alias_cell),
        TableCell::from(phase_line),
        TableCell::from(status_text),
        TableCell::from(Line::from(progress_cell)),
        TableCell::from(Line::from(backlog_cell)),
    ]
} else if terminal_width >= 60 { ... }
```

**Vertical split** (866): keep `[Min(0), Length(1)]` byte-identical below the gate; at
`area.width >= 100 && area.height >= 12` use `[Min(0), Length(1), Length(1)]` and draw the summary in
the middle row.

**Left/right footer row idiom** (1423-1432) for `You: ...` + `[i] inbox`:
```rust
let right_len: u16 = right_spans.iter().map(|s| s.width() as u16).sum();
let footer_chunks =
    Layout::horizontal([Constraint::Min(0), Constraint::Length(right_len + 1)]).split(area);
let left = Paragraph::new(Line::from(left_spans));
let right = Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right);
```
Bold key token as in `render_normal_footer`: `Span::styled("[i]", Style::default().add_modifier(Modifier::BOLD))`.

**`i` key** — copy the `s` arm (749-762): set `ctx.status_message`, `ctx.needs_redraw = true`,
return `ScreenAction::None`. Update the collision comment list at 784-788 (`i` is free).
```rust
KeyCode::Char('s') => {
    ...
    ctx.status_message = Some((msg.to_string(), std::time::Instant::now()));
    ctx.needs_redraw = true;
    ScreenAction::None
}
```

**Test helpers to reuse** (same `mod tests`): `render_dashboard_cells(ctx, width, height)`
(3074-3102, `(symbol, fg)` per cell), `find_in_row` (3105-3109), `rendered_text` (3112-3117),
`text_is_colored` (3346-3349), `render_selected(ctx, width)` (3476-3480, height 12),
`status_cell_text` (3485-3495, model for a `you_cell_text` between the `You` and `Progress`
headers), `line_text` (3505-3507). Extend `test_dashboard_columns_header_and_width_counts_match`
(2586-2604) with widths 100/99 and a row-cell-count check (Pitfall 6). Keep
`status_column_cells_matches_the_rendered_status_column` (3510-3531) and
`the_agent_summary_is_a_whole_ladder_form_at_every_width` (3534-...) green unmodified.
Fixture literal `ctx_with_aliases` (1642-1676): add `owner_digests: HashMap::new(),`.

---

### `src/ui/screens/detail.rs` (detail component)

**Focus enum** (1161-1181): add `Owner` with a doc comment in the `ScopeStrip` voice ("Modelled on
`Pane`: one enum value, never a view-cache flag ... It means nothing off the Phases tab;
`handle_key` resets it to `Content` anywhere else").

**Dispatch** (3939-3945): an identical block right after the `Pane` one:
```rust
if self.focus == DetailFocus::Pane {
    if current_view != DetailSubView::Pipeline {
        self.focus = DetailFocus::Content;
    } else if let Some(action) = self.handle_waves_pane_key(code, ctx) {
        return action;
    }
}
```

**Entry from the first phase row** (4255-4267): before the `TabBar` fallback, when
`current_view == DetailSubView::Pipeline` and the last frame drew a non-empty block, go to
`DetailFocus::Owner` with the cursor on the last stop.
```rust
KeyCode::Char('k') | KeyCode::Up => {
    if self.content_at_first_row(&current_view, ctx) {
        self.focus = if current_view == DetailSubView::Defaults {
            DetailFocus::ScopeStrip
        } else {
            DetailFocus::TabBar
        };
        ctx.needs_redraw = true;
        return ScreenAction::None;
    }
```
"Did the last frame draw a block" = a new `owner_block_rows: Cell<u16>` field on `DetailScreen`,
declared and documented like `waves_viewport_rows` (838) and initialized `Cell::new(0)` beside it
(1205); key tests that never render see 0 and keep today's `↑ → TabBar` (Pitfall 9).

**Block placement** (7456-7526): compute block height first, split `area` into
`[Length(h), Min(0)]`, render the block, pass the remainder into the unchanged body (including the
`entries.is_empty()` "No phases found" branch at 7483-7490). Skip the block when the focused Waves
pane takes the full tab (7498-7522). `h == 0` must leave `area` untouched.
```rust
let panes = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
    .split(area);
```

**Footers:** focus chain (6305-6318) gains
`} else if self.focus == DetailFocus::Owner && sub_view == DetailSubView::Pipeline { Paragraph::new(Line::from(owner_block_footer_spans())) }`;
`owner_block_footer_spans` copies `waves_pane_footer_spans` (11466-11483):
```rust
fn waves_pane_footer_spans() -> Vec<Span<'static>> {
    let b = Style::default().add_modifier(Modifier::BOLD);
    vec![
        Span::raw("  "),
        Span::styled("[\u{2190}/Esc]", b),
        Span::raw("phases  "),
        Span::styled("[j/k]", b),
        Span::raw("move  "),
        ...
    ]
}
```
Content footer lead (11335-11339): `up` becomes `"owner  "` when the Phases tab drew a block. Do it
with a parameter or wrapper; `footer_spans` has three test call sites (18233, 26921, 26932).
```rust
let up = if matches!(sub_view, DetailSubView::Defaults) { "scope  " } else { "tab bar  " };
```

**Escape helper** (81-83): bump `fn shown` to `pub(super)` so `owner_block.rs` and `owner.rs` use it.
`verification_hint_spans` (9455) stays untouched.

**Test fixture literal** `test_ctx` (20030-20049): add `owner_digests: HashMap::new(),`.

---

### `src/inbox.rs` (utility, wording)

**Visibility bump:** `plural` (255), `blocked_tally` (260), `detail` (290) → `pub(crate)`. They
already escape with `terminal_capped` (inbox.rs 21); passing their output through `shown()` again is
harmless (RESEARCH Correction 2).

**Extraction:** pull the lane-clause list out of `push_closes_after` (504-525) into
`pub(crate) fn closes_after_clauses(gate: &ClosesAfter) -> Vec<String>` and have
`push_closes_after` call it, so the CLI golden output (`tests/owner_digest.rs` 358-381,
`tests/fixtures/owner_golden/`) is byte-identical. Keep the CLI's `not built ph {list}` text
(RESEARCH Correction 4 / A3).
```rust
let lanes = [
    ("test", gate.test, &gate.lane_phases.test),
    ("decide", gate.decide, &gate.lane_phases.decide),
    ("unblock", gate.blocked, &gate.lane_phases.blocked),
    ("fix", gate.fix, &gate.lane_phases.fix),
];
let mut parts: Vec<String> = lanes.iter()
    .filter(|(_, count, _)| *count > 0)
    .map(|(label, count, phases)| format!("{label} {count} (ph {})", phase_list(phases)))
    .collect();
if parts.is_empty() && !gate.phases_open.is_empty() {
    parts.push(format!("no owner work (ph {})", phase_list(&gate.phases_open)));
}
if !gate.not_built.is_empty() {
    parts.push(format!("not built ph {}", phase_list(&gate.not_built)));
}
```
Row/phase labels for root signals follow `group_line` (383-389): `Kind::Debug` → `debug`,
`Kind::Handoff` → `handoff`. Test-builder for new tests: `inbox.rs` tests `group(lane)` (726-760)
is private; copy its literal shape.

---

### `src/main.rs` (startup wiring)

**Analog:** itself, 526-538. Insert the first-fill dispatch right after
`app.ctx.event_tx = Some(event_bus.tx.clone());` (538): loop `app.ctx.config.projects` and call
`app.ctx.schedule_owner_digest(alias, &project.path)` (collect `(alias, path)` first to avoid
borrowing `ctx` mutably/immutably at once, as the watcher loop at 540-544 iterates
`app.ctx.config.projects.values()`).
```rust
app.load_project_states();
app.init_change_tracker();
app.announce_gsd_install();
let event_bus = EventBus::new();
// Store event_tx on App context so creation flow can send actions back
app.ctx.event_tx = Some(event_bus.tx.clone());
```
`load_project_states` (app.rs 2458-2465) is synchronous and sends no action (RESEARCH Correction 3).

---

### `src/ui/screens/add_project.rs` (registration)

**Analog:** itself, 204-212. After `ctx.project_states.insert(alias.to_string(), state);` (206) add
`ctx.schedule_owner_digest(alias, &canonical);`; the screen holds only `ctx`, which is why the
scheduler lives on `AppContext`.

---

### `src/ui/screens/help.rs` (static copy)

**Analog:** itself. Helpers (189-198):
```rust
fn row(key: &str, description: &str) -> Line<'static> {
    Line::from(format!("  {key:<13} {description}"))
}
```
Insert rows per UI-SPEC "Help overlay entries": Keybindings `s` (228, replace its description) and
a new `i` row; Owner block key rows (each `(detail view)`); Filter Syntax after `row("//h", ...)`
(456); a new `heading("Owner Work")` + `Line::from("")` section between Filter Syntax (449-457)
and `heading("Dashboard Badges")` (460). Note the driver section already has an `s` row ("Start a
new run...", test at 702-705), so descriptions must stay unique (whole-row test at 679-...).
README 258 carries the same `s` text and changes with it. Unreadable files: document as fix work,
not test work (RESEARCH Correction 1).

---

### `src/ui/screens/render_escape_guard.rs` (safety test)

**Analog:** itself. `probe_ctx(identity)` (1093-...) builds on `super::tests::ctx_with_aliases` and
inserts hostile state; add a hostile `OwnerDigest` (group phase, phase_name, workstream, file,
file_status, command, item name/expected/reason/blocked_by/why_human, gap name/reason, archived
label all carrying `identity`) into `ctx.owner_digests`. `PROBE_WIDTH = 200` (2700), so the
dashboard probe renders the `>= 100` tier and the summary row. Add states in the `"NormalScreen"`
vec (2048-2093) via `one_state("dashboard with owner work", ctx, Box::new(NormalScreen::new()))`;
for `"DetailScreen"` (2114-2118) the Phases tab is covered by `states_over_sub_views`, but the
block draws only with a digest and a tab of at least 12 rows; update the "Fixture states" doc of
`adjudicate_screen!` for both screens.

---

### `README.md` (docs)

**Analog:** itself. Key tables at 249-362 (`| \`s\` | Toggle sort ... |` at 258, `↑ / k` row at
281, Tab-specific keys ending before `### Examples` at 363). The new `### Owner inbox` section text
is fixed verbatim by UI-SPEC "README snippet" and links `#owner-work-inbox` (the slug of
`#### Owner work: \`inbox\`` at 540). Replace "count as test work" wording per RESEARCH Correction 1.

## Shared Patterns

### Background load, never on the render path
**Source:** `src/app.rs` 1000-1019 (`schedule_reparse`), `src/ui/screens/mod.rs` 1817-1835
(`schedule_launcher_marker`), `tests/async_blocking_guard.rs` 132-137.
**Apply to:** `schedule_reparse`, `schedule_owner_digest`, every handler.
Rule: clone `tx` and owned inputs, `tokio::task::spawn_blocking(move || { ...; let _ = tx.send(Action::X { .. }); })`;
missing `event_tx` returns silently; render reads `ctx.owner_digests` only.

### Alias-keyed sibling map on `AppContext`
**Source:** `agent_views` doc (mod.rs 1346-1368), `prune_driver_maps` (app.rs 3574-3605),
destructure test (app.rs 5783-5844).
**Apply to:** `owner_digests`. Not on `ProjectState`; pruned; classified in the destructure; added
to all six literals (app.rs 932, mod.rs 2411, normal.rs 1642, detail.rs 20030,
driver_confirm.rs ~409, delete_confirm.rs ~240; find them with `reparse_dispatches: 0,`).

### Escaping and width
**Source:** `detail.rs` 81-83 (`shown`), 10797 (`fit_cells`); `normal.rs` 519-530 (`Line::width`
ladder).
**Apply to:** every digest string in the `You` cell (none expected: numbers only), the summary row,
and the Owner block. Never `str::len`, never byte slicing.

### Muted and quiet styling
**Source:** `driver.rs` 113-122 (`label_style` / `muted_style` = DIM on default fg).
**Apply to:** `·`, `(+N old)`, `You: ·`, `Loading...`, block detail lines, age, Todos fold. The
`You` cell sets `fg(Color::Reset)` explicitly on every span (row style at normal.rs 1223-1225).

### Focus level with `Option<ScreenAction>` handler
**Source:** `detail.rs` 1161-1181, 2053-2160, 3931-3945.
**Apply to:** `DetailFocus::Owner`.

### Predicate filter over the shared column matcher
**Source:** `mod.rs` 1931-1957 (`matches_any_column` closure shared by `All` and `NeedsHuman`).
**Apply to:** `FilterColumn::OwnerWork(lane)`.

### Whole-row help assertions
**Source:** `help.rs` 679-... (`for (key, description) in [...] { assert!(body.lines().any(|l| l == row(key, description)...)) }`).
**Apply to:** every new help row.

## No Analog Found

| File / piece | Role | Data Flow | Reason |
|--------------|------|-----------|--------|
| Fold line with a horizontal sub-cursor (`←`/`→` across folds) in `owner_block.rs` | component | event-driven | No existing single-row multi-stop widget; the Config scope strip (`handle_config_scope_key`, detail.rs 2112-2160, `←`/`→` select between two values) is the nearest model. Planner uses UI-SPEC "Folds" + "Interaction" tables. |
| Summary-row fit ladder (drop `K of N`, then `(C close)`, then `in P phases`) | utility | transform | `agent_summary_line` ladders over precomputed forms; here the forms are built from clauses. Same "first whole form that fits" rule, new form builder. |

## Metadata

**Analog search scope:** `src/app.rs`, `src/action.rs`, `src/main.rs`, `src/lib.rs`, `src/inbox.rs`,
`src/status_cli.rs`, `src/config.rs`, `src/state_reader/owner_items/mod.rs`,
`src/ui/screens/{mod,normal,detail,help,driver,add_project,render_escape_guard}.rs`,
`src/ui/glyphs.rs`, `tests/async_blocking_guard.rs`, `tests/owner_digest.rs`,
`tests/fixtures/owner/`, `README.md`.
**Files scanned:** 18
**Pattern extraction date:** 2026-11-14
