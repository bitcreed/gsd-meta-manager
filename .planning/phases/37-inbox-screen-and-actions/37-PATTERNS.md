# Phase 37: Inbox screen and actions - Pattern Map

**Mapped:** 2026-11-14
**Files analyzed:** 30 (7 new source/test files, 3 new fixture trees, 20 modified files)
**Analogs found:** 30 / 30 (every new file has at least a role-match analog; no file needs RESEARCH-only patterns)

All analog paths below were checked with `git ls-files` (tracked source, no install mirrors). Line
numbers are from the worktree HEAD on 2026-11-14.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/ui/screens/inbox.rs` (NEW) | component (screen + pure model) | request-response (render from cache, key -> view state) | `src/ui/screens/owner_block.rs` (model, grid, cursor ops) + `src/ui/screens/normal.rs` (pushed screen, footer/status) | exact (model) / role-match (screen) |
| `src/ui/screens/owner_actions.rs` (NEW) | service (shared UI actions) | request-response + subprocess | `src/ui/screens/detail.rs` 5586-5604 (`n` launch) and 5549-5562 / `normal.rs` 879-898 (focus) | exact |
| `src/ui/screens/ack_confirm.rs` (NEW) | component (modal confirm screen) | event-driven (key -> Action) | `src/ui/screens/queue_delete_confirm.rs` (whole file) + `driver_launch.rs` 1076-1088 (Yellow accent) | exact |
| `src/ui/screens/owner_file_view.rs` (NEW) | component (read-only viewer screen) | file-I/O (read once on key press) | `src/ui/screens/help.rs` `HelpScreen` 109-140, 558-646 + `archive::render_markdown_lines` (`src/archive.rs:525`) | role-match |
| `src/clipboard.rs` (NEW) | utility (tool choice + bounded child + OSC 52) | subprocess / transform | `src/multiplexer/gui.rs` 1-119 (probes struct + pure rank + thin impure edge) + `src/bounded_output.rs` (try_wait budget) | role-match |
| `tests/owner_acknowledge_roundtrip.rs` (NEW) | test (oracle-gated integration) | subprocess + file-I/O | `tests/owner_open_conformance.rs` 44-150 + `tests/owner_digest.rs` 16-68 | exact |
| `tests/fixtures/owner/context-planned/`, `context-summary-only/`, `context-entries/` (NEW) | test fixture | file-I/O | `tests/fixtures/owner/context-questions/` | exact |
| `src/state_reader/owner_items/context.rs` | service (Layer A scan) | file-I/O, cached parse | itself: `scan_with` 252-339, `derive_open_questions` 141-184 | self |
| `src/state_reader/owner_items/rules.rs` | service (Layer B rules) | transform | itself: `context_groups` 793-859, `command_for` 113-135, `QUESTION_LABEL` 748-751 | self |
| `src/state_reader/owner_items/mod.rs` | model | — | itself: `ContextFile` 333-354, `OwnerGroup` 469-487, `Hidden` 491-501 | self |
| `src/inbox.rs` | service (CLI JSON + text) | transform | itself: `project_json` 153-212, `build_document` 215-243, `push_section` 455-470 | self |
| `src/ui/screens/owner_block.rs` | component (frozen layout) | — | itself (visibility bumps only) | self |
| `src/ui/screens/detail.rs` | component | request-response | itself: `handle_owner_block_key` 2148-2230, argv builders 2784-2869 | self |
| `src/ui/screens/normal.rs` | component | request-response | itself: `b` arm 850-862 (push), interim `i` arm 803-814 | self |
| `src/ui/screens/help.rs` | component (static copy) | — | itself: `heading`/`row` 192-201, Owner Work section 486-509 | self |
| `src/ui/screens/render_escape_guard.rs` | test (census) | — | itself: `SCREEN_IDENTITY_DISPOSITIONS` 363-391, `fixture_for` arm 2288-2296 | self |
| `src/ui/screens/mod.rs` | model/store (`AppContext`, `ScreenAction`) | event-driven | itself: `owner_digests` 1355-1371, `schedule_owner_digest` 1896-1930, `ScreenAction` 349-365 | self |
| `src/action.rs` | model (Action enum) | event-driven | itself: `OwnerDigestLoaded` 186-196, `LauncherStopped` | self |
| `src/app.rs` | controller (update loop) | event-driven | itself: `ProjectStateLoaded` 2993-3000, `OwnerDigestLoaded` 3706-3714, `stop_launcher_run` 1213-1254, `LauncherStopped` 3277-3291, `prune_driver_maps` 3746-3791, `process_screen_action` 3860-3908 | self |
| `src/main.rs` | controller (main loop) | event-driven | itself: mouse-capture apply 649-658, `pending_editor` 681-700 | self |
| `src/launcher/process.rs` | utility (bounded child) | subprocess | itself: `run_bounded_with_stderr` 444-476, `GIT_REPO_ENV` 392-400 | self |
| `src/state_reader/queue_md.rs` | utility (gsd-tools resolver) | — | itself: `GsdToolsCmd` 17-33, `resolve_gsd_tools_from` 153-169, `gsd_tools_on_path` 172-180 | self |
| `src/multiplexer/mod.rs` (+ `fixtures.rs`) | utility (seam) | — | `launch_with` 967-1012, `launch` 1014-1032, `FakeBackend` 1291-1331, `fixtures.rs` | self |
| `src/lib.rs` | config (module list) | — | `pub mod inbox;` line 21 | self |
| `tests/owner_open_conformance.rs` | test (source guard) | — | itself: `no_source_file_names_the_open_audit_verb` 492-520 | self |
| `tests/focus_entry_guard.rs` | test (source guard) | — | itself: `FOCUS_CALLERS` 29-31 | self |
| `tests/spawn_seam_guard.rs` | test (source guard) | — | itself: `SPAWN_ALLOWLIST` 28-78 | self |
| `tests/inbox_cli.rs` | test (schema pin) | — | itself: `hidden_keys` 247-254, `INBOX_PATHS` 633-712 | self |
| `tests/owner_digest.rs` | test (integration) | — | itself: `context_questions_fill_decide` 523-599 | self |
| `README.md`, `src/agent_skill.md` | docs | — | README `### Owner inbox` 368+, key table 259, schema rows 693/748; `agent_skill.md:71` | self |

## Pattern Assignments

### `src/ui/screens/inbox.rs` (NEW: screen + pure model)

**Analogs:** `src/ui/screens/owner_block.rs` (pure model, grid, identities, cursor ops),
`src/ui/screens/normal.rs` (footer with status message, test render helper),
`src/ui/screens/help.rs` (pushed screen, `Cell` viewport for the key handler).

**Imports pattern** (`owner_block.rs` 16-30), copy and extend:
```rust
use crate::inbox::{self, ClauseForm};
use crate::state_reader::owner_items::{
    ClosesAfter, Kind, Lane, OwnerDigest, OwnerGroup, OwnerItem, Scope,
};
use crate::ui::glyphs;
use crate::ui::screens::owner::OWNER_SEP;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::collections::HashSet;

use super::detail::{fit_cells, fit_spans, shown};
use super::driver::muted_style;
```
Plus `use super::{AppContext, Screen, ScreenAction}; use crossterm::event::{KeyCode, KeyModifiers};`
(`queue_delete_confirm.rs` 1-9).

**Identity-keyed view state** (`owner_block.rs` 52-123): `GroupKey` (lane, kind, scope_label,
workstream, phase, file), `GroupKey::aggregate` for BLOCKED rows (77-84), `Fold`, `Stop::{Row, FoldLine}`,
`OwnerBlockState { expanded, open_folds, cursor, fold_cursor, cursor_index }`. The inbox stop is
`(alias, GroupKey)`; the inbox view state follows the same "identities, never indices" rule and lives on
`AppContext` (RESEARCH Pattern 6) so `AckConfirmScreen` can re-render it as background.

**Column grid** (`owner_block.rs` 555-628, `grid_for`): copy the narrowing loop shape:
```rust
// The phrase gives way first (down to 12), then the phase cell (down to 5).
while fixed(phase_w, phrase_w) + needed > width && phrase_w > 12 {
    phrase_w -= 1;
}
while fixed(phase_w, phrase_w) + needed > width && phase_w > 5 {
    phase_w -= 1;
}
```
`grid_for_inbox` differs per UI-SPEC: no lane-tag column, an alias column clamp(8, 18), phrase clamp(12, 32),
Command only on FIX rows and only when it fits unsqueezed, widths over all lanes (not the active filter).
Reuse `phase_cell` (653-674) for the ws-prefix truncation; reuse `age_spans` (638-648) for `stale` Yellow.

**Row text: reuse, never re-spell** (`owner_block.rs` 318-382, `row_cells`): bump `RowCells`, `row_cells`,
`phase_text`, `test_phrase`, `RowSrc`, `current_sources`, `old_debt_sources`, `acknowledged_sources`,
`item_text`, `detail_text`, `gap_text`, `expansion_lines` to `pub(crate)`. The inbox clears the
`closes phase` phrase (section title says it) and adds the alias column.

**Cursor ops** (`owner_block.rs` 1278-1409): `Move`, `resolve_cursor`, `set_cursor`, `move_cursor`,
`resolved_fold`, `toggle`, `fold_step`, `clamp`. Copy shape for the inbox stops:
```rust
pub(crate) fn resolve_cursor(view: &OwnerBlockState, stops: &[Stop]) -> Option<usize> {
    if stops.is_empty() {
        return None;
    }
    let found = view
        .cursor
        .as_ref()
        .and_then(|stop| stops.iter().position(|s| s == stop));
    Some(found.unwrap_or_else(|| view.cursor_index.min(stops.len() - 1)))
}
```
Cursor row style (`owner_block.rs` 1227-1242): `cursor_style()` = Cyan + BOLD, DIM removed;
`mark_cursor_row` swaps the `"  "` gutter for `"> "`.

**Key handler shape** (`detail.rs` 2148-2230, `handle_owner_block_key`): clamp, skeleton, then a `match code`
over `j/k/Down/Up`, `PageDown/PageUp` with `Move::By(page)`, `Enter | ' '` -> `toggle`, `Left/Right if
on_fold_line` -> `fold_step`. The inbox maps `Home/End` (not `g/G`) to `Move::First/Last`, adds `1-4`/`a`
filters, `Esc`/`q` -> `ScreenAction::Pop`, `?` -> `ScreenAction::Push(Box::new(HelpScreen::new()))`
(`normal.rs` 875-878), and delegates `g o v A y` to `owner_actions`.

**Digest access** (`ui/screens/mod.rs` 2008-2016): read only through `ctx.owner_digest(alias)` (returns
`None` for a gone folder), never `ctx.owner_digests` directly; classify loaded / unreadable
(`ctx.owner_state_unavailable(alias)` 1985-1989 or the new failed set) / loading.

**Header totals** (`owner.rs` 195-236): `owner_numbers(digest)` per project, summed like `owner_totals`
(but over every registered alias, not `filtered_aliases`); `OWNER_SEP` (` · `) as the clause separator.

**Footer with status message** (`normal.rs` 1346-1402): status replaces hints, escaped at the consumer:
```rust
if let Some((msg, _)) = &ctx.status_message {
    let line = Line::from(Span::styled(
        crate::text::render_for_terminal(msg),
        Style::default().fg(color),
    ));
    frame.render_widget(Paragraph::new(line), area);
} else {
    render_normal_footer(frame, area, ctx);
}
```
Hint tokens (`normal.rs` 1485-1501): `Span::styled("[j/k]", bold), Span::raw("move  ")`, measured with
`Span::width()`.

**Screen adjudication** (`queue_delete_confirm.rs` 29-35):
```rust
crate::ui::screens::adjudicate_screen!(
    InboxScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws every registered alias and every digest string ...",
);
```

**Test render helper** (`normal.rs` 3732-3767, `render_dashboard_cells` + `find_in_row` + `rendered_text`):
copy as `render_inbox_cells(ctx, w, h)` returning `(symbol, fg, modifier)` cells. Fixture context starts
from `crate::ui::screens::tests::ctx_with_aliases(&[...])` (`ui/screens/mod.rs:2564`) with digests
inserted like `normal.rs` 2705-2745 (`owner_fleet_ctx`, `owe(...)`) and `owner_block::sample_owner_digest`
(`owner_block.rs` 1415-1417).

---

### `src/ui/screens/owner_actions.rs` (NEW: shared `v`/`A`/`y`/`o`/`g` functions)

**Analog:** `src/ui/screens/detail.rs` 5586-5604 (new-session launch) and `normal.rs` 879-898 (focus).

**Launch call** (`detail.rs` 5586-5604), the exact seam to reuse:
```rust
KeyCode::Char('n') if current_view == DetailSubView::Sessions => {
    if let Some(project) = ctx.config.projects.get(&self.alias) {
        let plan = resolve_launch_plan();
        ScreenAction::SetStatusMessage(crate::multiplexer::launch(
            &plan,
            &project.path,
            mux_launch_argv(),
            &launch_terminal_argv,
            "Launched new Claude session",
        ))
    } else {
        ScreenAction::None
    }
}
```
For `v`: `crate::multiplexer::launch(&resolve_launch_plan(), &cwd, owner_mux_argv(&cmd),
&|t| owner_terminal_argv(t, &cmd), &format!("Launched {cmd} in {alias}"))`. `resolve_launch_plan`
(`detail.rs:2567`) and `terminal_program_separator` (`detail.rs:2611`) are private: bump to `pub(crate)`.

**Focus branch** (`normal.rs` 884-897), session matched by `working_dir`:
```rust
let project_path = ctx.config.projects.get(&alias).map(|p| p.path.clone());
let session = project_path.and_then(|path| {
    ctx.active_sessions
        .iter()
        .find(|s| s.working_dir == path)
        .cloned()
});
let msg = match session {
    Some(s) => crate::multiplexer::focus_message(&s, &alias),
    None => no_active_session_status(&alias),
};
```
Inbox variant: when `Some(s)`, append ` · type {command}` only if the message starts with `Switched to`
(UI-SPEC status table). Calling `crate::multiplexer::focus_message(` from this file requires the
`tests/focus_entry_guard.rs` edit below.

**Testable split** (`multiplexer/mod.rs` 967-1012, `launch_with`): keep a pure
`run_plan(ctx, alias, &OwnerGroup) -> Result<RunPlan, Refusal>`; tests call
`crate::multiplexer::launch_with(&plan, &cwd, argv, &gui_argv, &mut |term, args, dir| { record; Ok(()) },
success)` with a `LaunchPlan { backends: vec![], gui: Some("kitty".into()), .. }` (GUI branch) or a
`FakeBackend` moved to `multiplexer/fixtures.rs` as `#[cfg(test)] pub(crate)` (today private at
`multiplexer/mod.rs` 1291-1331).

**Validation** (`owner_items/rules.rs` 73-102): re-validate typed tokens with `is_command_token`
(`pub(super)` today) and `is_workstream_name` (private today), bumped to `pub(crate)` and re-exported
through `owner_items`; file basename / slug with `crate::text::is_identity_char`.

**Acknowledge child: off-thread with an Action back** (`app.rs` 1213-1254, `stop_launcher_run`):
```rust
let Some(tx) = &self.ctx.event_tx else {
    self.ctx.error_message = Some(format!(/* refuse visibly */));
    return;
};
let tx = tx.clone();
tokio::task::spawn_blocking(move || {
    let report = /* run the bounded child */;
    let _ = tx.send(Action::LauncherStopped { alias, run_id, report });
});
self.ctx.status_message = Some((
    crate::text::render_for_terminal(&format!("Stopping launcher run {run_id} on {alias}")).to_string(),
    std::time::Instant::now(),
));
```
Copy as `Acknowledging {file} via gsd-core...` + `Action::OwnerAcknowledged { alias, file, outcome }`.
The child is `launcher::process::run_bounded_with_stderr` (or its env-removing sibling, below) with cwd =
project root and `Duration::from_secs(10)`; program/prefix from `queue_md::resolve_gsd_tools_from`.

**Verb spelling** (Pitfall 1): spell the verb once, e.g. `const ACK_VERB: [&str; 2] = ["audit-open",
"acknowledge"];`, in this one file, and narrow `tests/owner_open_conformance.rs` in the same commit.

---

### `src/ui/screens/ack_confirm.rs` (NEW: `AckConfirmScreen`)

**Analog:** `src/ui/screens/queue_delete_confirm.rs` (entire file, 233 lines).

**Struct + popup** (lines 11-27):
```rust
pub struct QueueDeleteConfirmScreen {
    pub alias: String,
    pub index: usize,
    pub command_text: String,
    popup: ConfirmPopup,
}
// new(): popup: ConfirmPopup::default()   // focus defaults to No
```

**Key handling** (lines 38-85): `match self.popup.on_key(code) { Confirm => ..., Cancel => Pop,
Pending => None }`. On `Confirm` the ack screen re-resolves the group by identity (gone ->
`Group is no longer listed; nothing acknowledged`), then starts the child and returns `ScreenAction::Pop`.

**Render: host screen as background, dimmed, then the popup** (lines 87-105):
```rust
fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
    let detail = super::detail::DetailScreen::new(self.alias.clone());
    detail.render_main_only(frame, area, ctx);
    confirm_popup::dim_background(frame, area);
    self.popup.record(confirm_popup::render_confirm_popup(
        frame, area, "Remove from queue", /* body lines, pre-escaped */,
        Color::Red, self.popup.focus,
    ));
}
```
Ack variant: background = inbox model (from `AppContext` view state) or `DetailScreen::render_main_only`
(`detail.rs:8768`) when opened from the Owner block; title `Acknowledge via gsd-core`; accent
`Color::Yellow` as in `driver_launch.rs` 1080-1087. Clipping safety is the component's own
(`confirm_popup.rs` 110-118, `confirm_unless_clipped`; `CLIPPED_HINT` line 67).

**Mouse** (lines 107-111): `confirm_popup::click_as_key(&self.popup, input).map_or(ScreenAction::None,
|code| self.handle_key(code, KeyModifiers::NONE, ctx))`.

**Prompt escaping** (lines 146-156, `prompt_text`): body built by a pure `pub(crate) fn ack_body(..) ->
Vec<Line<'static>>` so the escape census can drive it; each project string through
`crate::text::render_for_terminal(..).into()` / `shown()`.

**Tests** (lines 158-233): `rows(screen, ctx)` TestBackend helper, `enter_defaults_to_cancel`,
`clicking_a_button_acts_and_a_stray_click_does_not` (uses `confirm_popup::locate`).

---

### `src/ui/screens/owner_file_view.rs` (NEW: read-only viewer for `o`)

**Analog:** `src/ui/screens/help.rs` `HelpScreen` (pushed, scrollable, pops on `Esc`).

**State with a render-written viewport** (`help.rs` 109-140):
```rust
pub struct HelpScreen {
    scroll_offset: u16,
    viewport: Cell<ViewportMetrics>,
}
```
Viewer adds `alias`, relative `path` (title), and `lines: Vec<Line<'static>>` computed ONCE on key press
from `archive::render_markdown_lines(&text)` (`src/archive.rs:525`; it escapes every line with
`render_for_terminal`, line 530). The read happens in `owner_actions` on the key press with the
containment guards (regular file, <= 1 MiB, canonical path under `<project>/.planning/`), never in
`render` (OWN-12 / `async_blocking_guard`). Do not use `browser::read_md_file` (`browser.rs:193`) as is:
it formats an error into the content and has no size cap.

**Key handler** (`help.rs` 559-609): down = add then clamp, up = clamp then subtract (UIFIX-04 order):
```rust
KeyCode::Char('j') | KeyCode::Down => {
    self.scroll_offset = clamp_scroll(
        self.scroll_offset.saturating_add(1),
        vp.total_lines,
        vp.visible_height,
    );
    ctx.needs_redraw = true;
    ScreenAction::None
}
KeyCode::Char('k') | KeyCode::Up => {
    self.scroll_offset =
        clamp_scroll(self.scroll_offset, vp.total_lines, vp.visible_height)
            .saturating_sub(1);
    ...
}
```
Pop on `Esc` and `q` (help pops on `?`/`Esc`).

**Render** (`help.rs` 611-641): record `ViewportMetrics` into the `Cell`, then
`Paragraph::new(lines).block(block).scroll((offset, 0))`. Full-area, title = `shown(alias)` + path.

**Adjudication**: `RENDERS_ATTACKER_INFLUENCED_IDENTITY` (file content and alias), plus a
`SCREEN_IDENTITY_DISPOSITIONS` row and `fixture_for` arm (see render_escape_guard below).

---

### `src/clipboard.rs` (NEW: `y` tool choice, bounded stdin child, OSC 52)

**Analog:** `src/multiplexer/gui.rs` (whole file): facts gathered at a thin impure edge, decision in a
pure function, spawn in one small function.

**Probe struct + pure ranking** (`gui.rs` 13-58):
```rust
pub struct TerminalProbes {
    pub terminal_env: Option<String>,
    pub xdg_terminal_exec_ok: bool,
    pub x_terminal_emulator_present: bool,
    pub fallback_present: Vec<String>,
}

pub fn rank_gui(probes: &TerminalProbes) -> Option<String> {
    probes.terminal_env.as_ref().filter(|term| !term.is_empty()).cloned()
        .or_else(|| probes.xdg_terminal_exec_ok.then(|| "xdg-terminal-exec".to_string()))
        ...
}
```
Clipboard: `ClipboardProbes { os: &'static str, wayland_display: bool, display: bool, on_path:
Vec<&'static str> }` -> `pick_tool(&probes) -> Option<Tool>` (`wl-copy`, `xclip -selection clipboard`,
`xsel --clipboard --input`, `pbcopy`, `clip.exe`), `None` -> OSC 52. Taking `os` as data (from
`std::env::consts::OS`) keeps every branch compiled on every target (cross-target clippy gate; no
`#[cfg(target_os)]` dead code).

**Thin impure edge** (`gui.rs` 70-95, `probe_terminals`): PATH lookup should be a pure `split_paths` scan
like `queue_md.rs` 172-180 (`gsd_tools_on_path`), not a `which` child.

**Bounded child** (`bounded_output.rs` 29-40 poll constants, 49-80): spawn with `stdin(piped)`,
`stdout(null)`, `stderr(null)` (Pitfall 4: forking tools hold piped stdout), write the text, drop stdin,
then `try_wait` with the doubling poll until exit or a 2 s budget, kill on expiry. Run on
`tokio::task::spawn_blocking`, report through a new `Action` (same shape as `stop_launcher_run` above).

**OSC 52**: hand-rolled RFC 4648 base64 (test vectors `"" -> ""`, `"f" -> "Zg=="`, `"foobar" ->
"Zm9vYmFy"`); `format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))`. Written only by the main loop (see
`src/main.rs` below), never from `render` or a key handler.

**Spawn allowlist**: `Command::new(` here requires a `tests/spawn_seam_guard.rs` row in the same commit.

---

### `tests/owner_acknowledge_roundtrip.rs` (NEW: oracle-gated integration)

**Analog:** `tests/owner_open_conformance.rs` 1-150 (oracle resolve/skip policy) + `tests/owner_digest.rs`
16-68 (temp copy of a fixture, digest of the copy).

**Opt-out constant and policy** (`owner_open_conformance.rs` 48-61):
```rust
const ALLOW_MISSING_ORACLE: &str = "GSD_META_MANAGER_ALLOW_MISSING_ORACLE";

fn skip_permitted(opt_out: Option<&str>) -> bool {
    matches!(opt_out, Some(value) if !value.trim().is_empty())
}
```

**Oracle resolution** (`owner_open_conformance.rs` 63-107): `install_candidates(None,
&InstallRoots::from_env())` mapped to `bin/gsd-tools.cjs`, `node --version` probe, else `gsd-tools` on PATH,
else `None` with an `eprintln!("SKIP: ...")`. A `None` must FAIL the test unless `skip_permitted(..)`.

**Child env hygiene** (`owner_open_conformance.rs` 122-131):
```rust
command
    .current_dir(root)
    .env_remove("GSD_WORKSTREAM")
    .env_remove("GSD_PROJECT");
```
The round trip should run the PRODUCT's argv builder from `owner_actions` (not a re-spelled argv) against
the temp copy, then re-read with `read_owner_digest` and assert the group is gone and
`hidden.acknowledged_files` grew. Add the active-workstream pointer case (Pitfall 2).

**Fixture copy** (`owner_digest.rs` 20-68): `copy_dir`, `copy_fixture(name) -> (TempDir, PathBuf)`,
`entry_of(name, alias)`, `now()` = `"2026-11-14T00:00:00Z"`. Never run against the repo tree.

---

### `tests/fixtures/owner/context-planned/`, `context-summary-only/`, `context-entries/` (NEW fixtures)

**Analog:** `tests/fixtures/owner/context-questions/` (tree: `.planning/ROADMAP.md`, `.planning/STATE.md`,
`.planning/phases/NN-name/NN-CONTEXT.md`, `audit-open.expect` = `same`, `oracle.expect` = `same`).

**CONTEXT shape to copy** (`context-questions/.planning/phases/02-alpha/02-CONTEXT.md`):
```markdown
---
status: draft
updated: 2026-10-01
---

# Phase 2 context

## Open Questions

- **Q-01 — RESOLVED by the second review.** Was the export format fixed?
...
## Decisions
```
`context-planned`: add `NN-01-PLAN.md` + `NN-01-SUMMARY.md` next to the CONTEXT; `context-summary-only`:
SUMMARY files only; `context-entries`: `<open_questions>` tags around `## Open Questions`, an intro
sentence, 4 wrapped entries, a `---` rule, one `**OQ-n** — RESOLVED by ...` entry. Carry
`audit-open.expect: same` so Layer A stays oracle-pinned.

---

### `src/state_reader/owner_items/context.rs` (MODIFY: `phase_planned`, top-level entries)

**Self-analog, listing loop** (`context.rs` 263-272): the directory is already listed once per target:
```rust
let Ok(entries) = std::fs::read_dir(&target.path) else {
    continue;
};
let mut names: Vec<String> = entries
    .flatten()
    .filter_map(|entry| entry.file_name().into_string().ok())
    .filter(|name| name.contains("-CONTEXT") && name.ends_with(".md"))
    .collect();
```
Collect all names first (skip non-files via `entry.file_type()`), compute `phase_planned` from the full
list (`PLAN.md`, `*-PLAN.md`, `SUMMARY.md`, `*-SUMMARY.md`), then filter the CONTEXT names. Set it on the
`ContextFile` literal (292-305).

**Cached parse** (`context.rs` 45-52, 200-218): add `entries: Vec<String>` to `ParsedContext` and compute it
in `parse_context` next to `derive_open_questions` (141-184), so it is cached in `CONTEXT_CACHE` with the
rest. `questions` (Layer A, oracle input, `questions_digest`) must stay byte-identical. Entries go through
`sanitize_for_display` (118-139) exactly as questions do. Section lookup reuses the
`markdown::collect_section(.. heading.level == 2 && starts_with("open questions") ..)` call (156-169).

---

### `src/state_reader/owner_items/rules.rs` (MODIFY: D-17 gate, D-18 units, typed `RunCommand`)

**Self-analog, `context_groups`** (793-859): insert the planned gate after `items` is built and before the
`acknowledged_files` increment / push:
```rust
if items.is_empty() && !unreadable {
    continue;
}
// D-17 goes here: planned + not acknowledged + readable -> count and continue
if file.acknowledged {
    digest.hidden.acknowledged_files += 1;
}
```
Item ids keep the `{base}#q{position}` form (812-814) over the new entry list.

**`QUESTION_LABEL` extension** (748-751): add an optional emphasis closer before the separator:
```rust
regex::Regex::new(r"^[A-Za-z]{1,6}[-.]?[0-9][A-Za-z0-9.-]*(?:[ \t]*[—–:][—–:\s-]*|[ \t]+-+\s+)")
```
Extend the table test `is_answered_reads_explicit_markers_only` (positive `- **OQ-02** — RESOLVED by
H-02`, `1. **Q3** – CLOSED`; negative `- **OQ-04** — [OPEN] keep?`).

**Typed command** (113-135 `command_for`, 609 `DEBUG_COMMAND`, 871-877 `resume_command`): every builder
produces a `RunCommand` and sets `command: run.as_ref().map(ToString::to_string)` so the JSON `command`
string does not change. The `OwnerGroup` literal to extend is shown at 658-685 (`debug_groups`) and
834-857 (`context_groups`).

**Visibility**: `is_command_token` (74, `pub(super)`) and `is_workstream_name` (100, private) ->
`pub(crate)`.

---

### `src/state_reader/owner_items/mod.rs` (MODIFY: model fields)

**Self-analog:** `ContextFile` (331-354), `OwnerGroup` (467-487), `Hidden` (489-501). Add
`ContextFile.phase_planned: bool`, `ContextFile.entries: Vec<String>` (Layer B inputs, like
`DebugSession.fm_unparseable` 324-325, documented "never compared with the oracle"),
`Hidden.planned_questions: usize` (struct derives `Copy, Default`), `OwnerGroup.run: Option<RunCommand>`
and the `RunVerb`/`RunCommand` types (derive `Debug, Clone, PartialEq, Eq` like `OwnerGap` 431-436).
`OwnerGroup` has no `Default`: every struct literal must add `run` (2 in `inbox.rs`, 2 `gate.rs`, 1
`mod.rs`, 6 `rules.rs`, 1 `owner_block.rs`, 2 `owner.rs`, 1 `render_escape_guard.rs`).

---

### `src/inbox.rs` (MODIFY: `planned_questions`, shared `group_checklist`)

**JSON** (153-212, 215-243): add `"planned_questions"` to both `hidden` objects (error entry 159-166 with
`0`, read entry 182-189 with `entry.digest.hidden.planned_questions`) and to `summary`
(`"planned_questions": sum(|d| d.hidden.planned_questions)`, next to `acknowledged_files` line 237).

**Checklist extraction** (455-470, `push_section`): pull the per-group body out:
```rust
for row in rows {
    out.push(format!("  {}", group_line(&row.alias, row.group)));
    for item in &row.group.items {
        out.push(format!("    - [ ] {}", terminal_capped(&item.name)));
        for gap in &item.gaps {
            out.push(format!("        - {}", terminal_capped(&gap.name)));
        }
    }
}
```
into `pub(crate) fn group_checklist(alias: &str, group: &OwnerGroup) -> Vec<String>` called by
`push_section`, so `y` and `inbox` text are one formatter; the owner goldens prove byte identity. Do not
change `render_text` ordering (638-646; Pitfall 9).

---

### `src/ui/screens/owner_block.rs` (MODIFY: visibility only, byte-identical output)

`pub(crate)` bumps for the items listed under `inbox.rs` above. One new parameter at most:
`expansion_lines(src, width, shift)` (952-979) where the Owner block passes `0`. Add `run: None` to the
`OwnerGroup` literal in `sample_owner_digest` (1415+) and `planned_questions` to the full `Hidden {` literal
at 1536. The 36 snapshot tests in this file must pass unedited.

---

### `src/ui/screens/detail.rs` (MODIFY: argv builders, D-21 keys)

**Argv builders** (2832-2869): add next to them, same doc style (CWE-88 reasoning):
```rust
fn mux_launch_argv() -> Result<crate::multiplexer::ExecArgv, crate::multiplexer::MuxError> {
    let mut argv = vec!["env".to_string()];
    argv.extend(claude_launch_args());
    crate::multiplexer::ExecArgv::new(argv)
}

fn launch_terminal_argv(term: &str) -> Vec<String> {
    let mut argv = vec![terminal_program_separator(term).to_string()];
    argv.extend(claude_launch_args());
    argv
}
```
New: `pub(crate) fn owner_mux_argv(cmd: &RunCommand)` = `["env", "claude", cmd.to_string()]` and
`pub(crate) fn owner_terminal_argv(term, cmd)` = `[separator, "claude", cmd]`. Pin test modeled on
`launch_terminal_argv_carries_no_untrusted_element_and_is_pinned_at_two` (24081-24112): same terminal table,
assert exact length 3 and element 2 equals the validated command string.

**D-21 keys** (2148-2230, `handle_owner_block_key`): the catch-all comment at 2220 (`// e o v A y ...`) is
where `v`, `o`, `A`, `y` gain arms that call `owner_actions` with the selected `Stop::Row(key)`. Do not
touch `owner_block_footer_spans` (11736). Narrow `owner_block_swallows_action_keys` (36276-36299,
`for c in ['e', 'o', 'v', 'A', 'y', 'x']`) to `['e', 'x']`. Test helpers: `owner_ctx` (35823),
`owner_screen` (36170), `render_detail_to_text_at` (22418).

**`g` target** (1569-1573): `DetailScreen::opened_on(alias, DetailSubView::Pipeline, ctx)` is the entry
the inbox pushes (NOT `App::open_project_view`, which refuses unless the stack is 1 deep, `app.rs`
3831-3834).

---

### `src/ui/screens/normal.rs` (MODIFY: `i` pushes `InboxScreen`)

**Replace** the interim arm (803-814) with the push shape of the `b` arm (850-862):
```rust
KeyCode::Char('b') => {
    if let Some(alias) = ctx.selected_alias() {
        ctx.detail_scroll_offset = 0;
        ctx.needs_redraw = true;
        ScreenAction::Push(Box::new(DetailScreen::opened_on(alias, DetailSubView::Backlog, ctx)))
    } else {
        ScreenAction::None
    }
}
```
(`i` needs no selected alias; reset the inbox view state, then `ScreenAction::Push(Box::new(InboxScreen::new()))`.)
Replace the test `the_i_key_points_at_the_inbox_command` (2976-2991) with a push assertion; keep its
"a bare `y` stays unbound" half. Update the bound-keys comment (770-777) that lists `i`.

---

### `src/ui/screens/help.rs` (MODIFY: rows)

**Helpers** (192-201): `heading(text)`, `row(key, description)` (`format!("  {key:<13} {description}")`).
Replace the `i` row (232-235). Add `heading("Owner Inbox")` after the "Owner Work" block (486-509),
mirroring its `Line::from("")` spacing. Update `help_lines_documents_every_key_this_phase_binds` (732+,
the 36 `i` assertion at 892) with whole-row assertions; descriptions must stay unique in the body.

---

### `src/ui/screens/render_escape_guard.rs` (MODIFY: census rows + fixtures)

**Rows** (363-391): add `("InboxScreen", "src/ui/screens/inbox.rs")`, `("AckConfirmScreen",
"src/ui/screens/ack_confirm.rs")`, `("OwnerFileViewScreen", "src/ui/screens/owner_file_view.rs")` in
alphabetical order.

**Fixture arm** (2288-2296):
```rust
"QueueDeleteConfirmScreen" => Some(|identity| {
    states_over_sub_views(identity, &|identity, _ctx| {
        Box::new(super::queue_delete_confirm::QueueDeleteConfirmScreen::new(
            identity.to_string(), 0, identity.to_string(),
        ))
    })
}),
```
The inbox arm builds its context with `probe_ctx(identity)` (1189) and inserts a digest whose alias,
phase name, item names, file, command and the status message carry `identity`. Add `run: None` to the
`OwnerGroup` literal in this file.

---

### `src/ui/screens/mod.rs` (MODIFY: module list, `AppContext`, `ScreenAction`)

- Module list (1-19): `pub mod inbox; pub mod owner_actions; pub mod ack_confirm; pub mod owner_file_view;`.
- New field next to `owner_digests` (1355-1371), with a doc comment of the same style:
  `pub owner_digest_failed: HashSet<String>` and the inbox view state; initialize in `ctx_with_aliases`
  (2564-2598).
- `schedule_owner_digest` (1896-1930): the panic branch currently `return`s (1915-1924); send
  `Action::OwnerDigestFailed { alias }` instead:
```rust
let Some(digest) = Self::guarded_owner_digest(&alias, || { ... }) else {
    return;   // becomes: let _ = tx.send(Action::OwnerDigestFailed { alias }); return;
};
let _ = tx.send(crate::action::Action::OwnerDigestLoaded { alias, digest: Box::new(digest) });
```
- `ScreenAction` (349-365): add a variant for the OSC 52 hand-off, documented like
  `ToggleMouseCapture` (360-364): "`App` owns ..., the binary's main loop applies it to the terminal".

---

### `src/action.rs` (MODIFY: new actions)

**Analog** (186-196):
```rust
/// ... Plain data, so `Action` stays `Clone`.
OwnerDigestLoaded {
    alias: String,
    digest: Box<OwnerDigest>,
},
```
Add `OwnerDigestFailed { alias: String }`, `OwnerAcknowledged { alias: String, file: String, outcome:
String }` (already-rendered outcome, like `LauncherStopped.report`), and a clipboard result action. Keep
the clone test pattern at 509-513.

---

### `src/app.rs` (MODIFY: handlers, prune, OSC 52 hand-off)

- `ProjectStateLoaded` digest branch (2993-3000): on `Some` stored -> `owner_digest_failed.remove(&alias)`;
  on `None` while `owner_digest_storable` -> insert and redraw.
- `OwnerDigestLoaded` (3706-3714): remove from the failed set when it stores.
- New handlers modeled on `LauncherStopped` (3277-3291): escape with `render_for_terminal`, set
  `status_message`, `needs_redraw = true`. Full capped child output to `tracing::warn!`.
- `prune_driver_maps` (3778-3782): add `self.ctx.owner_digest_failed.retain(|a| registered.contains_key(a));`
  with the same one-line justification comment.
- `process_screen_action` (3860-3908): new arm storing the OSC 52 payload in an `App` field, like
  `SuspendAndEdit` -> `self.pending_editor = Some(..)` (3890-3892); field declared beside
  `pending_editor` (561) and initialized at 1010.
- Tests: `#[tokio::test]` with `obs_app(dir.path())` and `app.update(Action::...)` (5929-5951).

---

### `src/main.rs` (MODIFY: write OSC 52 after the frame)

**Analog:** `pending_editor` take (681) and the mouse-capture apply (649-658), both outside
`terminal.draw`:
```rust
if app.mouse_capture != tui::mouse_captured() {
    if let Err(e) = tui::set_mouse_capture(app.mouse_capture) { ... }
}
```
Write `app.pending_osc52.take()` to `std::io::stdout()` after `terminal.draw` (667), using `execute!`/`write_all`
+ flush as `tui.rs` 52-60 does for capture. `src/main.rs` is already on the spawn allowlist; a stdout write
needs no allowlist change.

---

### `src/launcher/process.rs` (MODIFY: env-removing bounded runner)

**Self-analog** (444-476, `run_bounded_with_stderr`) and the env-removal precedent (392-400, 459-463):
```rust
if program == "git" {
    for key in GIT_REPO_ENV {
        cmd.env_remove(key);
    }
}
for (key, value) in extra_env {
    cmd.env(key, value);
}
let output = stdout_within_with_stderr(&mut cmd, budget, Stdio::from(sink.try_clone().ok()?))?;
```
Add a sibling (or a `remove_env: &[&str]` parameter on a new fn) that removes `GSD_WORKSTREAM` and the
gsd-core session-key variables (RESEARCH Pitfall 2 list) before `extra_env`. It returns `None` for both
"could not start" and "timed out" (`bounded_output.rs` 49-60): check `node` on PATH first. The returned
stderr line is the LAST non-empty line (411-431), which is what Pitfall 7 needs. Stays in this file, so no
allowlist change.

---

### `src/state_reader/queue_md.rs` (MODIFY: visibility)

`GsdToolsCmd` (17-23, fields `program`, `prefix_args`) and `resolve_gsd_tools_from` (153-169) ->
`pub(crate)`; `resolve_gsd_tools` (116-118) is the env-reading wrapper to call from `owner_actions`.
Argv = `program` + `prefix_args` + the ack args, each one element.

---

### `src/multiplexer/mod.rs` (+ `fixtures.rs`) (MODIFY: docs, test fake)

Update "called only from `detail.rs`" (1014-1016) and "called only from `normal.rs` and `detail.rs`"
(1215-1216). If the SC1 test needs a backend fake, move `FakeBackend` (1291-1331) into `fixtures.rs` (already
`pub(crate) mod fixtures;` at line 120, `#[cfg(test)]`-gated helpers like `exec_lock` 26-32).

---

### `src/lib.rs` (MODIFY)

`pub mod clipboard;` in alphabetical position (after `pub mod cli;`, line 9).

---

### Guard tests (MODIFY, same commit as the code they allow)

- `tests/owner_open_conformance.rs` (492-520): allow exactly `src/ui/screens/owner_actions.rs`, and require
  every allowed hit line to also contain `"acknowledge"`; keep the `executable > 1000` floor.
- `tests/focus_entry_guard.rs` (29-31): `FOCUS_CALLERS` gains `"src/ui/screens/owner_actions.rs"` (sorted:
  the assertion compares a sorted, deduped path list).
- `tests/spawn_seam_guard.rs` (34-78): add `"src/clipboard.rs"` with a one-line reason in the house style:
  `// wl-copy / xclip / xsel / pbcopy / clip.exe with the checklist on stdin. No agent.`

### `tests/inbox_cli.rs` / `tests/owner_digest.rs` (MODIFY)

- `inbox_cli.rs`: `hidden_keys` (247-254), summary key set (~217-232), error-entry `hidden` literal
  (~315-324), `INBOX_PATHS` (633-712: add `"summary.planned_questions"`, `"projects[].hidden.planned_questions"`).
- `owner_digest.rs` `context_questions_fill_decide` (523-599): `decide` 6 -> 5, the 02-alpha ids become
  `["02-CONTEXT.md#q4", "02-CONTEXT.md#q5"]`, text `2 open questions in 02-CONTEXT.md` (RESEARCH Pattern 2).
  New tests follow `entry_of(fixture, alias)` + `build_document(.., &include_all())` (45-78).

### `README.md` / `src/agent_skill.md` (MODIFY)

README: key-table row 259, `### Owner inbox` paragraph 368-372, schema rows near 693 (`summary.*`) and 748
(`projects[].hidden.*`); `the_readme_inbox_fence_has_exactly_the_schema_v1_key_paths` and
`the_readme_inbox_table_documents_every_path` in `tests/inbox_cli.rs` enforce both. `agent_skill.md:71`:
"open CONTEXT questions" -> "open CONTEXT questions of phases not yet planned".

## Shared Patterns

### Terminal-safe text (OWN-11)
**Source:** `src/ui/screens/detail.rs:82` `shown`, `:11042` `fit_cells`; `crate::text::render_for_terminal`
(status line, `normal.rs` 1395-1398); `crate::text::terminal_capped` (`inbox.rs`).
**Apply to:** inbox.rs, ack_confirm.rs, owner_file_view.rs, owner_actions.rs status strings, app.rs result handlers.
Every project-authored string through `shown` before layout; widths via `Line::width()` / `fit_cells`, never
`len()`. Status messages are escaped at the consumer (footer), and producers that build status text from
child output also run `render_for_terminal` (`app.rs` 1249-1252, 3284-3285).

### Blocking work off the render thread, result as an Action
**Source:** `src/app.rs` 1213-1254 (`stop_launcher_run`) + 3277-3291 (`LauncherStopped`);
`src/ui/screens/mod.rs` 1896-1930 (`schedule_owner_digest`).
**Apply to:** acknowledge child, clipboard child, digest-failure signal.
Clone `ctx.event_tx`, `tokio::task::spawn_blocking`, `let _ = tx.send(Action::...)`; refuse visibly when
there is no channel; set the "in progress" status before returning.

### Argv safety (CWE-88)
**Source:** `src/ui/screens/detail.rs` 2788-2869 (doc + builders), `src/multiplexer/gui.rs` 97-118.
**Apply to:** `owner_mux_argv`, `owner_terminal_argv`, the ack argv builder.
Untrusted values only as single argv elements, leading `env` for multiplexer vectors, cwd via
`current_dir` / tmux `-c`, never a `cd` or interpreter string; typed tokens re-validated at the point of use;
pin tests assert exact vectors.

### Identity-pinned selection
**Source:** `src/ui/screens/owner_block.rs` 52-123, 1302-1409.
**Apply to:** inbox cursor, expansion set, `g` focus target, ack re-resolution on Yes.
Store `GroupKey`-style identities; resolve to an index per frame; a vanished identity clamps to
`cursor_index`.

### Screen adjudication and escape census
**Source:** `crate::ui::screens::adjudicate_screen!` (`ui/screens/mod.rs:275`), usage
`queue_delete_confirm.rs` 29-35; census `render_escape_guard.rs` 363-391 + `fixture_for` arms.
**Apply to:** every new `Screen` (`InboxScreen`, `AckConfirmScreen`, `OwnerFileViewScreen`).

### Source-guard allowlists edited deliberately
**Source:** `tests/spawn_seam_guard.rs` 28-34 ("Whoever adds a spawn site must add the entry here in the
same commit"), `tests/focus_entry_guard.rs` 29-31, `tests/owner_open_conformance.rs` 492-520.
**Apply to:** clipboard.rs (spawn), owner_actions.rs (focus, `audit-open` verb).

### Buffer snapshot tests (no insta)
**Source:** `normal.rs` 3732-3774 (`render_dashboard_cells`, `find_in_row`, `rendered_text`),
`queue_delete_confirm.rs` 164-176 (`rows`).
**Apply to:** inbox.rs, ack_confirm.rs, owner_file_view.rs tests. `TestBackend`, assert joined text plus
per-cell `fg`/`modifier`.

## No Analog Found

None. Two parts have only partial precedent and should lean on RESEARCH.md as well:

| File | Part | Reason |
|------|------|--------|
| `src/clipboard.rs` | stdin-fed child that may fork; OSC 52 base64 | No existing child is fed through stdin, and no code writes an escape sequence to the terminal outside ratatui; use RESEARCH Pitfall 4 and the OSC 52 example |
| `src/state_reader/owner_items/context.rs` | top-level entry folding (D-18) | No list-entry parser exists; Layer A's line filter (141-184) is the closest; use RESEARCH Pattern 2 algorithm |

## Metadata

**Analog search scope:** `src/ui/screens/`, `src/multiplexer/`, `src/launcher/`, `src/state_reader/`
(incl. `owner_items/`), `src/app.rs`, `src/action.rs`, `src/main.rs`, `src/tui.rs`, `src/inbox.rs`,
`src/bounded_output.rs`, `tests/`, `tests/fixtures/owner/`
**Files scanned:** 34
**Pattern extraction date:** 2026-11-14
