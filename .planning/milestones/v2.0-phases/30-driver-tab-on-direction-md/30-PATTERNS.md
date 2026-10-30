# Phase 30: Driver Tab on DIRECTION.md - Pattern Map

**Mapped:** 2026-10-20
**Files analyzed:** 17 (6 new, 11 modified)
**Analogs found:** 17 / 17 (every file has an in-tree analog; RESEARCH.md already verified the APIs)

All analog paths below are git-tracked `src/` / `tests/` sources. Line numbers were read on 2026-10-20 at
`f8ab657`. `src/launcher/` is being edited in parallel (auth.rs): treat launcher line numbers as approximate and
re-grep before quoting.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/driver/steer.rs` (NEW) | utility (pure transforms) | transform | `src/driver/direction.rs` (`parse_direction`, `blank_comments`, tests :674+) | exact |
| `src/driver/direction.rs` (MOD: `pub(crate)` on `control_re`/`item_re`, edit-temp const) | utility | transform | itself (:144-169) | exact |
| `src/ui/screens/driver_run.rs` (NEW: view model + render, tiers, tokens, footer) | component | request-response (render over cache) | `src/ui/screens/driver.rs` (`render_driver_tab`, `driver_panes` :723, `shown_capped` :654, `output_header_line` :1685) | exact |
| `src/ui/screens/driver_steer.rs` (NEW: control chooser, answer input, conflict chooser) | component (footer-input screen) | event-driven | `src/ui/screens/enqueue.rs` (whole, 140 lines) | exact |
| `src/ui/screens/driver_launch.rs` (NEW: slug input, lint popup, launch form, unsandboxed confirm, kill/polite-stop confirms) | component (confirm popup) | event-driven → Action | `src/ui/screens/driver_confirm.rs` (:196-525) + `confirm_popup.rs` | exact |
| `src/ui/screens/driver.rs` (MOD: delete 28-07 launcher section; legacy detail no-orphan at H 8..=13) | component | render | itself | exact |
| `src/ui/screens/detail.rs` (MOD: keymap, `e` Driver branch, footer replaces `driver_footer_spans` :10456) | component (key router) | event-driven | itself (`e` arm, driver keys ~:5380-5570; `fit_cells` :10083) | exact |
| `src/ui/screens/normal.rs` (MOD: badge ranks crashed/stale/unknown) | component | transform | itself `alias_badge` :202-251, `row_badge` :264 | exact |
| `src/ui/screens/mod.rs` (MOD: `ProjectViewCache` fields, marker types, `needs_human` split, constants) | store / model | CRUD (cache) | itself `needs_human_for` :1803-1811, `ProjectViewCache` :1124-1220, `schedule_run_list_scan` :1836-1951 | exact |
| `src/ui/screens/help.rs` (MOD) | config (static text) | — | itself :371-396 | exact |
| `src/ui/screens/render_escape_guard.rs` (MOD: fixtures for every new screen/site) | test | — | itself `probe_ctx` :1085, `fixture_for` list :368-378 | exact |
| `src/journal/mod.rs` (MOD: `ChangeKind::{DriverRun, Ignored}`, `classify_change`) | utility (pure router) | event-driven | itself :1088-1157 | exact |
| `src/watcher.rs` (MOD only if needed; dedup key `(root, kind)` :59-77) | service | event-driven | itself | exact |
| `src/action.rs` (MOD: new `Action::Driver*` variants) | model (message) | event-driven | itself `LauncherStopRequested`/`LauncherStopped` :376-394 | exact |
| `src/app.rs` (MOD: Action arms, `after_editor`, `launch_fn` seam, marker scan in 20-tick task, LOG tail) | controller | request-response via `spawn_blocking` | itself `stop_launcher_run` :985-1028; 20-tick scan :1281-1310; `refresh_after_editor` :565-571 | exact |
| `src/main.rs` (MOD: pass editor exit status) | controller (binary) | request-response | itself :833-886 | exact |
| `tests/driver_tab_launch_e2e.rs` (NEW, zero-spend relaunch) | test (integration) | batch | `tests/launcher_e2e.rs` + `tests/launcher_support/mod.rs` (`Fixture`) | exact |

## Pattern Assignments

### `src/driver/steer.rs` (utility, pure transform)

**Analog:** `src/driver/direction.rs`

**Shared regexes — make `pub(crate)`, do not copy** (direction.rs:144-169):
```rust
fn control_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?mi)^[ \t]*-[ \t]+control[ \t]*:[ \t]*(\w+)").expect("control regex")
    })
}
pub fn control_hold(word: &str) -> Option<&'static str> { /* pause|stop, case-insensitive */ }
fn item_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?m)^### (I-\d+)\b").expect("item regex"))
}
```
**Comment-aware matching** (direction.rs:176-182): run regexes over `blank_comments(body)` (same length and
char boundaries), then splice into the original body at the same byte offsets. Item segments are scoped with
`section_heading` → `Section::Items`, like `parse_direction` (:237).

**Write contract** (direction.rs:421 `pub fn write_body<F: FnMut(&str) -> String>(run_dir, edit)`): the closure
may run more than once; return the live body unchanged to signal refusal (RESEARCH Pattern 3 example). Map
`SpliceOutcome::{Ok, SkippedInsaneBody, GaveUp, NoDirection}` + `lock_timed_out` (Pitfall 1: timeout = written
without the lock, NOT an error).

**Answer validator:** model on `driverctl::log::one_line` (`src/driverctl/log.rs:159-172`) but allow `|`; also
reject `<!--`, `-->`, U+2028/2029, > 500 chars (Pitfall 4).

**Test pattern** (direction.rs:674-723): `#[cfg(test)] mod tests { use super::*; const BODY: &str = "...## Direction\n\n- control: run\n\n## Items\n\n### I-01 first\n..."; }`
with table-style asserts. Also test against `driver_templates::DIRECTION_TEMPLATE`. Post-conditions:
`parse_direction(&new).control == Some(word)`; item-id set unchanged after answer.

---

### `src/ui/screens/driver_steer.rs` (footer-input screens)

**Analog:** `src/ui/screens/enqueue.rs` (entire file)

**Imports** (enqueue.rs:1-8):
```rust
use super::{AppContext, Screen, ScreenAction};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
```
**Adjudication (mandatory, sealed supertrait)** (enqueue.rs:20-28):
```rust
crate::ui::screens::adjudicate_screen!(
    EnqueueScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "…why + fixture states…",
);
```
**Key handling shape** (enqueue.rs:37-97): `Esc` → clear + `Pop`; `Backspace` → pop char; `Char(c)` push;
`Enter` on empty → `ScreenAction::None`. **Deviation required:** enqueue.rs:40-57 does file I/O in the key
handler — do NOT copy that; instead return `ScreenAction::DispatchAction(Action::DriverSteer…)` (D-25).

**Render: body behind, input footer** (enqueue.rs:105-134):
```rust
let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
let detail = super::detail::DetailScreen::new(self.alias.clone());
detail.render_main_only(frame, chunks[0], ctx);
let footer = Paragraph::new(Line::from(vec![
    Span::styled("  Enqueue> ", Style::default().add_modifier(Modifier::BOLD)),
    Span::raw(crate::text::render_for_terminal(&ctx.input_buffer)),
    Span::styled("  [Tab] suggestions  [Enter] queue  [Esc] cancel", Style::default().fg(Color::DarkGray)),
]));
```
Answer input needs `Constraint::Length(2)` for the OQ-R1 second row (UI-SPEC §Answer).

---

### `src/ui/screens/driver_launch.rs` (confirm popups + launch form)

**Analog:** `src/ui/screens/driver_confirm.rs` :196-525; primitive `src/ui/screens/confirm_popup.rs`
(`ConfirmPopup` :54, `ConfirmFocus` :31, `render_confirm_popup` :194).

**Screen struct — one field feeds prompt and dispatch** (driver_confirm.rs:219-262, T-18-41/T-28-18):
```rust
pub struct DriverConfirmScreen { alias: String, action: DriverAction, run_id: String, popup: ConfirmPopup, /*…*/ }
pub fn new_launcher_stop(alias: String, run_id: String) -> Self {
    Self { run_id, ..Self::new(alias, DriverAction::StopLauncher) }
}
```
**Prompt text with sanitised identity** (driver_confirm.rs:327, 344-352):
```rust
let alias = &crate::text::render_for_terminal(alias);
DriverAction::StopLauncher => {
    let run_id = crate::text::render_for_terminal(run_id);
    format!("Stop launcher run {run_id} on \"{alias}\"? This stops its whole sandbox scope. [y/n]")
}
```
(Replace copy with UI-SPEC "Kill driver run" text.)

**Key handling** (driver_confirm.rs:413-433):
```rust
match self.popup.on_key(code) {
    ConfirmOutcome::Confirm => { do_stop_launcher(ctx, &self.alias, &self.run_id); ScreenAction::Pop }
    ConfirmOutcome::Cancel => { ctx.needs_redraw = true; ScreenAction::Pop }
    ConfirmOutcome::Pending => { ctx.needs_redraw = true; ScreenAction::None }
}
```
**Render** (driver_confirm.rs:443-514): background screen + `confirm_popup::dim_background`, body
`Vec<Line<'static>>`, `strip_key_hint(&prompt)`, then
`self.popup.record(render_confirm_popup(frame, area, title, body, accent, self.popup.focus))`.
For the Driver tab, render `DetailScreen::render_main_only` behind instead of `NormalScreen`.
**Mouse** (driver_confirm.rs:517-521): `confirm_popup::click_as_key(&self.popup, input)`.

Launch form: while editing a field, `y`/`n` are text (input-intercept rule); `y` inert until host check
returns; `y` blocked when body `control != run` (Pitfall 2). Defaults from clap `LaunchArgs` (`src/cli.rs:526-574`)
via `#[command(flatten)]` + `try_parse_from`; validate with `cli::positive_finite` (`src/cli.rs:16`).

---

### `src/ui/screens/driver_run.rs` (render/view model)

**Analog:** `src/ui/screens/driver.rs`
- Panes: `driver_panes` (:723), `DRIVER_TWO_PANE_MIN_CELLS`/`DRIVER_DETAIL_MIN_CELLS` constants with
  doc-commented arithmetic (:71-110 style).
- List selection idiom: `"> "` + Cyan + BOLD with persisted `driver_list_offset` (:1110-1122).
- Sanitise: `shown_capped` (:654) / `crate::text::render_for_terminal`; then truncate with
  `super::detail::fit_cells` (`detail.rs:10083`, `pub(super)`). Do NOT use `driver.rs::ellipsize` (:793, char count).
- Width-room pattern (detail.rs:9619-9627):
```rust
let room = cells.saturating_sub(base + word_w + hint_w + tokens_w);
if room > 1 { let cut = fit_cells(&title, room - 1); /* push */ }
```
- LOG pane: `output_header_line` (:1685), `follow_indicator`, `tail_offset`, `clamp_scroll`; ring =
  `DriverOutput` (`mod.rs:531-561`), bounds `DRIVER_OUTPUT_RING_LINES`/`DRIVER_OUTPUT_LINE_CELLS` (`mod.rs:390-407`).
- Footer spans: `Span::styled("[c]", BOLD)` + `Span::raw("ontrol  ")`, following `driver_footer_spans` (`detail.rs:10456`).
- Tests: `TestBackend` sweep H 1..=40 × W 40/60/80/120 (D-29), mirroring existing driver.rs tier tests.

---

### `src/journal/mod.rs` (pure watcher router)

**Analog:** itself :1088-1157.
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]   // Hash is half the dedup key — keep run_id inside
pub enum ChangeKind { DriverJournal { run_id: String }, Planning }
...
if parts.len() >= 5 && parts[0] == ".planning" && parts[1] == "meta-manager" && parts[2] == "runs" {
    return ChangeKind::DriverJournal { run_id: parts[3].to_string() };
}
ChangeKind::Planning
```
Add the `drivers` arm before the fallback as an **allowlist** (RESEARCH "Watcher classification extension"):
`drivers/<id>` and `drivers/<id>/{DIRECTION.md,LOG.md}` → `DriverRun{run_id}`; anything else under
`drivers/` → `Ignored`. Keep the "no filesystem access" purity (doc :1103-1110) and the path-table test style.

---

### `src/action.rs` (messages)

**Analog:** :376-394 — doc comment states which handler runs it, that it goes through `spawn_blocking`, and
where identity comes from:
```rust
LauncherStopRequested { alias: String, run_id: String },
/// A launcher stop finished. `report` is the already-rendered outcome…
LauncherStopped { alias: String, run_id: String, report: String },
```
New variants: plain data, `Clone`, request/response pairs (`DriverRunRefresh{alias,run_id}` →
`DriverRunRefreshed{…}`, `DriverSteerRequested` → `DriverSteerDone`, `DriverEditPrepare` → `DriverEditReady`,
`DriverEditApply`, `DriverNewRun*`, `DriverLaunchRequested` → `DriverLaunched`), and extend
`RunsReconciled` with launcher markers.

---

### `src/app.rs` (controller)

**spawn_blocking → Action with visible refusal** (app.rs:985-1028):
```rust
#[cfg(target_os = "linux")]
fn stop_launcher_run(&mut self, alias: &str, run_id: &str) {
    if !self.ctx.experimental { return; }
    let Some(path) = self.project_path_for(alias) else { return; };
    let Some(tx) = &self.ctx.event_tx else {
        self.ctx.error_message = Some(format!("Could not stop the launcher run on '{alias}': this session has no event channel."));
        self.needs_redraw = true;
        return;
    };
    let tx = tx.clone();
    tokio::task::spawn_blocking(move || {
        let report = match crate::launcher::stop::stop_run(&path, &run_id_for_task, Duration::from_secs(60)) {
            Ok(report) => report.to_string(), Err(error) => error.to_string(),
        };
        let _ = tx.send(Action::LauncherStopped { alias: alias_for_task, run_id: run_id_for_task, report });
    });
    self.ctx.status_message = Some((crate::text::render_for_terminal(&format!("…")).to_string(), Instant::now()));
    self.needs_redraw = true;
}
```
Copy this for steer write, edit prepare/apply, new run, launch (each Linux-only arm needs a
`#[cfg(not(target_os = "linux"))]` refusal — Pitfall 13).

**Marker scan rides the 20-tick task — no second timer** (app.rs:1281-1310):
```rust
if let Some(ref tx) = self.ctx.event_tx {
    let tx: UnboundedSender<Action> = tx.clone();
    let projects = self.ctx.config.projects.clone();
    tokio::task::spawn_blocking(move || {
        let runs = crate::driver::reconcile::reconcile_all(&projects);
        let last_outcomes = crate::driver::reconcile::last_ended_outcomes(&projects);
        let _ = tx.send(Action::RunsReconciled { runs, last_outcomes });   // + launcher_markers
    });
}
```
**Editor hook** (app.rs:565-571 `refresh_after_editor(&path)`; `pending_editor` :509, set at :2614): add
`after_editor(&path, EditorExit)` that routes a known edit temp to `DriverEditApply` on spawn_blocking.

**Launch seam:** `launch_fn: fn(&LaunchOptions, Option<&Path>) -> Result<u32, LauncherError>` defaulting to
`launch_detached`; build `LaunchOptions` as `src/driver_cli.rs:286-314` does (Pitfall 5).

---

### `src/main.rs` (binary editor handoff)

**Analog:** itself :833-886; resolution :841-844 unchanged (`$VISUAL → $EDITOR → vi`). Only change: pass exit
status into the library hook. `tests/async_blocking_guard.rs` allowlists only `("src/main.rs", ".status()")` —
no new fs I/O in `run_tui_loop`.

---

### `src/ui/screens/normal.rs` (badge)

**Analog:** `alias_badge` :202-251 — pure if/else rank chain over a `BadgeInputs` struct; each arm
`Some(AliasBadge { glyph: BADGE_*, color, modifier })`. Add `launcher_marker` input and arms for crashed (rank 1),
parked (2), driven (3), legacy needs_human (4), stale (5), unknown (6). Glyph constants as fixed `&'static str`
(`normal.rs:91-100`), covered by `badge_glyphs_are_width_stable`. Gate on `ctx.experimental` exactly as
`row_badge` gates `driven_and_live` (:276-280).

### `src/ui/screens/mod.rs` (attention)

**Analog:** `needs_human_for` :1803-1811:
```rust
pub fn needs_human_for(&self, alias: &str) -> bool {
    let last = self.last_outcomes.get(alias).map(|label| TerminalState::from_label(Some(label)));
    self.project_states.get(alias)
        .is_some_and(|state| needs_human(state, self.observed_runs.get(alias), last, false))
}
```
Keep this for badge rank 4; add `needs_attention_for` (legacy OR marker ∈ {crashed, parked, stale}) and route
the `h` filter / AttentionFirst (`mod.rs:2031-2037, 2109`) to it (Pitfall 12). New cache fields go on
`ProjectViewCache` (`#[derive(Default)]`, zero construction churn), not `AppContext`.

---

### `tests/driver_tab_launch_e2e.rs` (integration)

**Analog:** `tests/launcher_e2e.rs` with `tests/launcher_support::Fixture` (fake `claude`/`bwrap`/`systemd-run`
on the child's PATH, `CARGO_BIN_EXE_gsd-meta-manager`). Relaunch a fixture run left `running` with a free lock.
Any stub `$EDITOR` process must live under `tests/` (`tests/spawn_seam_guard.rs` SPAWN_ALLOWLIST :45 fails new
`Command::new` in `src/`).

## Shared Patterns

### No I/O on the render thread (D-25)
**Source:** `src/app.rs:985-1028`, `src/ui/screens/mod.rs:1836-1951` (`schedule_run_list_scan`).
**Apply to:** every key handler in driver_run/steer/launch; every App arm. Key handlers only return
`ScreenAction::Push` / `DispatchAction`. Guard: `tests/async_blocking_guard.rs`.

### Untrusted text rendering (D-09, T-28-19)
**Source:** `crate::text::render_for_terminal` (driver_confirm.rs:327), `shown_capped` (driver.rs:654), then
`detail::fit_cells`. Order: sanitise → truncate (Pitfall 8).
**Apply to:** run ids, frontmatter strings, titles, answers, LOG lines, lint findings, launcher error detail.
Every new screen: `adjudicate_screen!` + `SCREEN_IDENTITY_DISPOSITIONS` row + `fixture_for` entry in
`render_escape_guard.rs`.

### Liveness only from the lock
**Source:** `state_reader::driver_frontmatter::{derive_status, read_driver_run, list_driver_runs}` (:751-846),
`driver::lock::probe_at` (:361-382). Never derive from `connected[].state` or heartbeat.

### Linux-only behind cfg with typed refusal
**Source:** `#[cfg(target_os = "linux")] fn stop_launcher_run` (app.rs:985). Pair each with a non-Linux arm
producing the read-only copy.

### Experimental gate (D-03)
**Source:** `if !self.ctx.experimental { return; }` (app.rs:987), `row_badge` (normal.rs:276).

## No Analog Found

None. Closest gaps (use RESEARCH.md patterns):
| Item | Note |
|---|---|
| Edit-temp create with `O_EXCL` + 0600 and base-compare inside `write_body` closure | RESEARCH Pattern 4; no existing body-only temp flow |
| Launch form with editable fields inside a `ConfirmPopup` | No multi-field popup exists; compose `ConfirmPopup` + per-field cursor (Selection style) |

## Metadata

**Analog search scope:** `src/ui/screens/`, `src/driver/`, `src/journal/`, `src/app.rs`, `src/action.rs`,
`src/main.rs`, `tests/`
**Files scanned:** ~14 directly; remaining line refs carried from RESEARCH.md (verified there)
**Pattern extraction date:** 2026-10-20
