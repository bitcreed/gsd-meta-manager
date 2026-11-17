# Phase 39: Check actions through GSD - Pattern Map

**Mapped:** 2026-11-17
**Files analyzed:** 15 (2 new source modules, 2 new integration tests, 1 in-crate flow test, 10 modified files)
**Analogs found:** 15 / 15 (every file has a Phase 37 or driver-steer precedent in the tree)

All analog paths below are git-tracked sources (verified with `git ls-files`). Line numbers are from
the worktree at `/home/blk/projects/rust/gsd-meta-manager-owner-inbox` on 2026-11-17.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/ui/screens/owner_settle.rs` (NEW) | service (pure: target, refusals, sanitizer, instruction, plan, launch seam) | request-response (key -> typed plan -> launch) | `src/ui/screens/owner_actions.rs` (`ack_target_for`, `run_plan`, `launch_real`, `run_group`, `ack_recheck`) | exact |
| `src/ui/screens/settle_popup.rs` (NEW): `SettleChooserScreen` | component (screen) | event-driven (key -> Replace) | `src/ui/screens/driver_steer.rs` `ControlChooserScreen` (185-299) | exact |
| `src/ui/screens/settle_popup.rs`: `SettleReasonScreen` (+ phase picker) | component (screen, single-line input) | event-driven | `src/ui/screens/driver_steer.rs` `AnswerScreen` (444-599); `driver_launch.rs` `SlugInputScreen` (1317-1483) | exact |
| `src/ui/screens/settle_popup.rs`: `SettleConfirmScreen` | component (default-No confirm popup) | request-response (Yes -> recheck -> launcher) | `src/ui/screens/ack_confirm.rs` `AckConfirmScreen` (whole file) | exact |
| `src/ui/screens/ack_confirm.rs` (MOD) | component | transform | itself: `wrap_at_separators` (106-144) -> `pub(crate)`; reuse `AckHost` (33-39) | exact |
| `src/ui/screens/detail.rs` (MOD) | controller (key dispatch) + utility (argv builders) | request-response | itself: `handle_owner_block_key` (2192-2297), `owner_ack` (2345-2386), `owner_mux_argv`/`owner_terminal_argv` (3085-3122), pin test (24646-24690) | exact |
| `src/ui/screens/inbox.rs` (MOD) | controller (key dispatch, cursor, footer) | event-driven | itself: `handle_key` (1903-1976), `ack_key` (1742-1776), `InboxViewState` (126-142), `FOOTER_KEYS` (1201-1256) | exact |
| `src/ui/screens/owner_block.rs` (MOD) | component (model/cursor) | transform | itself: `OwnerBlockState.fold_cursor` sub-cursor (161-172), `item_lines` (1175-1218), `block_model` (1273-1364), `move_cursor`/`clamp` (1598-1688) | exact |
| `src/state_reader/owner_items/rules.rs` (MOD) | utility (pure predicate/lookup) | transform | itself: `phase_labels` (369-378) + `scope_state` (111-115); `is_command_token`/`is_workstream_name` (74-107) | exact |
| `src/ui/screens/mod.rs` (MOD) | config (module registry, `ScreenAction`) | n/a | itself: `pub mod` list (1-23), `ScreenAction` (330-376) | exact |
| `src/app.rs` (MOD, only if `PopThen` is chosen, Pitfall 8) | controller (action dispatch) | event-driven | itself: `process_screen_action` (3932-3974) | exact |
| `src/ui/screens/help.rs` (MOD) | config (help rows) | n/a | itself: rows 375 and 532-550 | exact |
| `src/ui/screens/render_escape_guard.rs` (MOD) | test (census fixtures) | n/a | itself: `SCREEN_IDENTITY_DISPOSITIONS` (363-384), `"AckConfirmScreen"` fixture (2288-2356) | exact |
| `tests/owner_settle_guard.rs` (NEW) | test (source scan) | batch (file walk) | `tests/owner_open_conformance.rs` (465-572) + `tests/async_blocking_guard.rs` `scan` cfg(test) skip (425-508) | exact |
| `tests/owner_settle_oracle.rs` (NEW, optional) | test (oracle round trip) | file-I/O (temp copy) | `tests/owner_acknowledge_roundtrip.rs` (1-130) | exact |
| In-crate flow test (tree snapshot, D-13b) in `settle_popup.rs` or `owner_settle.rs` `mod tests` | test | file-I/O + event-driven | `src/ui/screens/ack_confirm.rs` tests (264-358) + `tests/owner_acknowledge_roundtrip.rs` `copy_dir`/`temp_project` (103-130) | role-match |
| `README.md` (MOD) | docs | n/a | itself: "Owner inbox" (373-382, 404-410), key table row 340 | exact |

## Pattern Assignments

### `src/ui/screens/owner_settle.rs` (service, request-response) - NEW

**Analog:** `src/ui/screens/owner_actions.rs`

**Module doc + imports pattern** (lines 1-40): a `//!` block stating where the command comes from
(CWE-88), that nothing is injected (D-12), and that the app writes nothing. Imports:
```rust
use crate::session_detector::ClaudeSession;
use crate::state_reader::owner_items::{
    is_command_token, is_workstream_name, Kind, Lane, OwnerDigest, OwnerGroup, RunCommand, RunVerb,
    Scope,
};
use crate::state_reader::ProjectState;
use crate::ui::screens::{AppContext, ScreenAction};
use std::path::{Path, PathBuf};

use super::ack_confirm::{AckConfirmScreen, AckHost};
use super::detail::{owner_mux_argv, owner_terminal_argv, resolve_launch_plan, shown};
use super::inbox::InboxKey;
use super::owner_block::GroupKey;
```
For settle: swap `owner_mux_argv`/`owner_terminal_argv` for the new `settle_mux_argv`/`settle_terminal_argv`.

**Typed plan struct** (lines 42-51, `RunPlan`) - shape for `SettlePlan`/`SettleTarget`:
```rust
#[derive(Debug, Clone)]
pub(crate) struct RunPlan {
    pub alias: String,
    /// The project path: the child's cwd, never part of the argv.
    pub cwd: PathBuf,
    pub command: RunCommand,
    /// A live session whose working directory IS the project path (D-12).
    pub live_session: Option<ClaudeSession>,
}
```
Add `key: InboxKey` for the Yes recheck exactly like `AckRequest.key` (lines 452-464).

**Refusal table** (lines 322-421, `ack_target_for`) - mirror for `settle_target`. Key excerpts:
```rust
if group.lane == Lane::Blocked {
    return Err("Cannot acknowledge a BLOCKED row: gsd-core has no category for it".to_owned());
}
if group.acknowledged {
    return Err(format!("Already acknowledged: {name}"));
}
...
let refused =
    |what: &str, value: &str| format!("Refused: {value} is not a safe {what}; nothing started");
if let Some(ws) = group.workstream.as_deref() {
    if !is_workstream_name(ws) {
        return Err(refused("workstream name", ws));
    }
}
...
if group.workstream.is_none() && active_workstream_set {
    return Err(format!(
        "Cannot acknowledge {name}: {alias} has an active workstream set; run it in the project"
    ));
}
```
Settle inverts the lane rule: accept `Lane::Test | Lane::Blocked`, refuse `Fix | Decide | Deferred`,
`Scope::Archived { .. }`, `acknowledged`, `counts.unreadable > 0` (copy from `run_plan` lines 68-70).
Pass `active_workstream_set: bool` as a parameter (pure, table-testable) like `ack_target_for`.

**Validation + live-session lookup** (lines 60-102, `run_plan`):
```rust
if takes_phase(command.verb) {
    let token = command.phase.as_deref().ok_or_else(no_command)?;
    if !is_command_token(token) {
        return Err(format!(
            "Refused: {token} is not a safe phase token; nothing started"
        ));
    }
}
...
let project = ctx.config.projects.get(alias).ok_or_else(no_command)?;
// Exact path equality (T-37-06)
let live_session = ctx
    .active_sessions
    .iter()
    .find(|session| session.working_dir == project.path)
    .cloned();
```
Do NOT call `run_plan` itself (it refuses BLOCKED, lines 65-67, and BLOCKED groups have `run: None`,
`rules.rs:138`). Build `RunCommand { verb: RunVerb::VerifyWork, phase: Some(..), workstream }`.

**Launch / focus** (lines 107-131, `focus_status` + `launch_real`) - template for `launch_settle_real`:
```rust
pub(crate) fn launch_real(plan: &RunPlan) -> String {
    if let Some(session) = &plan.live_session {
        let message = crate::multiplexer::focus_message(session, &plan.alias);
        return focus_status(message, &plan.command);
    }
    let launch_plan = resolve_launch_plan();
    crate::multiplexer::launch(
        &launch_plan,
        &plan.cwd,
        owner_mux_argv(&plan.command),
        &|term| owner_terminal_argv(term, &plan.command),
        &format!("Launched {} in {}", plan.command, plan.alias),
    )
}
```
Settle's live-session branch also copies `plan.prompt()` (slash command + instruction, not `claude ...`)
through `owner_actions::copy` (lines 585-599), see Shared Patterns "Clipboard".

**Closure seam** (lines 133-144, `run_group`):
```rust
pub(crate) fn run_group(
    ctx: &AppContext, alias: &str, group: &OwnerGroup,
    launch: &mut dyn FnMut(&RunPlan) -> String,
) -> String {
    match run_plan(ctx, alias, group) {
        Ok(plan) => launch(&plan),
        Err(message) => message,
    }
}
```

**Recheck on Yes** (lines 779-804, `ack_recheck`): re-resolve by identity under current state and
active-workstream pointer; compare with the shown target; on mismatch
`Err("The file changed since the popup opened; nothing acknowledged")`. Settle: plan (prompt) must be
identical; wording `...; nothing launched`.

**Pointer read** (lines 477-482, reuse as-is): `owner_actions::active_workstream_is_set(&project.path)`.

**Open-entry wiring** (lines 752-773, `ack_open`): returns `Result<ScreenAction, String>`; both hosts call
it and print the `Err` as status. Settle: `settle_open(ctx, alias, row_key, item_id, host) ->
Result<ScreenAction, String>` pushing `SettleChooserScreen`.

**Sanitizer building blocks:** `crate::text::is_invisible_formatting_char` (`src/text.rs:190`,
`pub(crate)`), `crate::text::strip_terminal_controls` (`src/text.rs:374`), cap idiom
`line.chars().take(ACK_LINE_CAP).collect()` (owner_actions.rs:826). Sketch in 39-RESEARCH.md "Code Examples".

**Test pattern** (owner_actions.rs tests from line 942; `run_plan_refusals` at 979, `ack_refusals` at 1407):
table of mutated groups, `ack_target_for("alder-api", state, &digest, &group, false)` and
`(..., &root, true)` for the pointer case (lines 1505-1524).

---

### `src/ui/screens/settle_popup.rs` - `SettleChooserScreen` (component, event-driven) - NEW

**Analog:** `src/ui/screens/driver_steer.rs` `ControlChooserScreen` (lines 177-299)

**Key handling with Replace** (lines 245-274):
```rust
match code {
    KeyCode::Char('r') => self.choose("run", ctx),
    KeyCode::Char('p') => self.choose("pause", ctx),
    KeyCode::Char('s') => {
        // Replacing the chooser (not pushing over it) makes `n` and `Esc` return to the tab.
        ctx.needs_redraw = true;
        ScreenAction::Replace(Box::new(PoliteStopScreen::new(
            self.alias.clone(),
            self.run_id.clone(),
        )))
    }
    KeyCode::Esc => {
        ctx.needs_redraw = true;
        ScreenAction::Pop
    }
    _ => ScreenAction::None,
}
```
Settle: `w` / `p` / `m` -> `ScreenAction::Replace(Box::new(SettleReasonScreen::new(..)))`; `Esc` -> `Pop`.
WR-03 rationale for `Replace`: `src/ui/screens/mod.rs:333-353`.

**One-line hint render** (lines 276-294): host painted first, then a single footer row
`[w]on't do  [p]ermanently defer  [m]ove  [Esc] cancel` with `bold()`/`muted()` key spans. For the inbox
host, paint `InboxScreen::new().render(..)`; for detail use `render_main_only` (see host switch in
ack_confirm.rs 235-241). A small popup instead of a footer is UI-SPEC discretion.

**Adjudication** (lines 235-243): every new screen needs `crate::ui::screens::adjudicate_screen!(Name,
crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY, "<what it draws and where it comes from>");`.

---

### `src/ui/screens/settle_popup.rs` - `SettleReasonScreen` + phase picker (component, input) - NEW

**Analog:** `src/ui/screens/driver_steer.rs` `AnswerScreen` (lines 436-599). Prefer it over
`SlugInputScreen` (driver_launch.rs 1317-1483): it has the cap, the inline Red error row and Enter
validation, and does not depend on `view_cache.new_run_status`.

**State + cap** (lines 444-469):
```rust
pub struct AnswerScreen {
    alias: String, run_id: String, item: String,
    park_rule: Option<String>,
    buffer: String,
    error: Option<String>,
}
fn at_cap(&self) -> bool {
    self.buffer.chars().count() >= DIRECTION_ANSWER_MAX_CHARS
}
```
Settle cap = 200 chars (D-02).

**Keys** (lines 482-537): `Enter` validates (empty is a no-op unless the outcome allows an empty reason;
Won't do requires one, D-02), `Esc` -> `Pop`, `Backspace` pops a char and clears `error`, `Char(c)` pushes
unless `at_cap()`. On valid Enter settle returns `ScreenAction::Replace(Box::new(SettleConfirmScreen::new(..)))`
instead of `dispatch(..); Pop`. For Move, `j`/`k`/Up/Down (or Tab) pick among `move_candidates`; free
entry is accepted only when `is_command_token` passes (D-06).

**Render** (lines 539-595): prompt + `tail_cells(render_for_terminal(&self.buffer), room)` + `_` + hint;
notes rows wrap with `Paragraph::new(notes).wrap(Wrap { trim: false })`, footer height capped at
`(area.height / 2).max(2)`. `tail_cells` is `pub(super)` in driver_steer.rs (417), reachable from a
sibling screen module.

---

### `src/ui/screens/settle_popup.rs` - `SettleConfirmScreen` (component, request-response) - NEW

**Analog:** `src/ui/screens/ack_confirm.rs` (entire file, 642 lines)

**Imports** (lines 19-30):
```rust
use super::confirm_popup::{self, ConfirmOutcome, ConfirmPopup};
use super::detail::shown;
use super::inbox::InboxScreen;
use super::owner_actions::{self, AckRequest};
use super::owner_block;
use super::{AppContext, Screen, ScreenAction};
use crate::ui::mouse::MouseInput;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;
```

**Screen struct with injectable launcher** (lines 41-64):
```rust
pub struct AckConfirmScreen {
    host: AckHost,
    request: AckRequest,
    popup: ConfirmPopup,
    /// `None` is the real one. Only tests set it.
    runner: Option<fn(&AckRequest) -> String>,
}
#[cfg(test)]
fn with_runner(mut self, runner: fn(&AckRequest) -> String) -> Self { self.runner = Some(runner); self }
```
Settle: `launcher: Option<fn(&SettlePlan) -> String>`; `None` -> `owner_settle::launch_settle_real`.

**Yes with recheck** (lines 66-90, `confirm`): look up group by `request.key.key` via
`owner_block::group_for` (or `entry_for` for merged rows), say `"... no longer listed; nothing launched"`
if gone, run recheck, then call the launcher and set `ctx.status_message`.

**Body builder** (lines 146-205, `ack_body`): `LABEL_W = 11` label column, `field(label, value)` with
`wrap_at_separators(&value, inner_w - LABEL_W)` and a hanging indent; every dynamic value through
`shown`. Settle rows: `Project`, `Target`, `Outcome`, blank, the command line, blank, instruction
(wrapped), blank, one consequence line.

**Screen impl** (lines 217-262):
```rust
fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers, ctx: &mut AppContext) -> ScreenAction {
    ctx.needs_redraw = true;
    match self.popup.on_key(code) {
        ConfirmOutcome::Confirm => { self.confirm(ctx); ScreenAction::Pop }
        ConfirmOutcome::Cancel => ScreenAction::Pop,
        ConfirmOutcome::Pending => ScreenAction::None,
    }
}
fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
    match &self.host {
        AckHost::Inbox => InboxScreen::new().render(frame, area, ctx),
        AckHost::Detail(alias) => {
            super::detail::DetailScreen::new(alias.clone()).render_main_only(frame, area, ctx)
        }
    }
    confirm_popup::dim_background(frame, area);
    self.popup.record(confirm_popup::render_confirm_popup(
        frame, area, "Acknowledge via gsd-core",
        ack_body(&self.request, confirm_popup::body_room(area).0),
        Color::Yellow, self.popup.focus,
    ));
}
fn handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction {
    confirm_popup::click_as_key(&self.popup, input).map_or(ScreenAction::None, |code| {
        self.handle_key(code, KeyModifiers::NONE, ctx)
    })
}
```
If the live-session path needs Pop plus OSC 52 (Pitfall 8), return the new combined action from the
`Confirm` arm instead of plain `Pop`.

**Popup geometry** (`confirm_popup.rs` 169-171, 303-307): width `area.width - 4` clamped to 72, inner 70;
body rows `area.height - 5` (19 at 80x24). Clipped popups refuse `y` (`confirm_popup.rs` 110-118).

**Tests to copy** (ack_confirm.rs 264-641): `setup()` (289-303) with `ctx_with_aliases(&[ALIAS])` and a
digest from `owner_block::sample_owner_digest()`; `rows()` TestBackend dump (315-327); `press()` (329-331);
per-test `static X_RUNS: AtomicUsize` + `fn x_runner` (333-357); `enter_defaults_to_no` (359-368);
`escape_cancels_and_starts_nothing` (370-384); `a_vanished_group_is_not_acknowledged` (416-433);
`the_popup_fits_80x24_with_the_exact_argv` (478-518); `the_popup_clips_at_60x20_and_cannot_confirm`
(579-613); `a_hostile_popup_draws_no_raw_control_cell` with
`"x\u{1b}[2J\u{7}\u{202e}y\u{200b}z\u{85}"` (614-641).

---

### `src/ui/screens/ack_confirm.rs` (MOD)

Change `fn wrap_at_separators(text: &str, width: usize) -> Vec<String>` (line 106) to `pub(crate)` so
`settle_popup.rs` wraps the instruction and command with it. Reuse `AckHost` (lines 33-39) as the
settle host enum rather than a second `Inbox | Detail(alias)` type (or rename it to a shared host if
the planner prefers; keep the two variants).

---

### `src/ui/screens/detail.rs` (MOD: argv builders, `x` arm, `owner_settle` method)

**Argv builders** - add beside lines 3073-3122:
```rust
pub(crate) fn owner_mux_argv(
    cmd: &crate::state_reader::owner_items::RunCommand,
) -> Result<crate::multiplexer::ExecArgv, crate::multiplexer::MuxError> {
    let mut argv = owner_scrubbed_program();
    argv.push(cmd.to_string());
    crate::multiplexer::ExecArgv::new(argv)
}
fn owner_scrubbed_program() -> Vec<String> {   // line 3098, private: settle builders must live in detail.rs
    let mut argv = vec!["env".to_string()];
    for var in super::owner_actions::ACK_ENV_REMOVE {
        argv.push("-u".to_string());
        argv.push((*var).to_string());
    }
    argv.extend(claude_launch_args());
    argv
}
pub(crate) fn owner_terminal_argv(term: &str, cmd: &RunCommand) -> Vec<String> {
    let mut argv = vec![terminal_program_separator(term).to_string()];
    argv.extend(owner_scrubbed_program());
    argv.push(cmd.to_string());
    argv
}
```
`settle_mux_argv(plan)` / `settle_terminal_argv(term, plan)` push `plan.prompt()` (ONE element) instead of
`cmd.to_string()`. Copy the CWE-88 doc comment (3078-3084).

**Pin test** (lines 24640-24690, `owner_terminal_argv_carries_no_untrusted_element_and_is_pinned_at_three`):
loop over `(term, separator)` rows and assert
`[vec![sep], scrubbed_program(), vec![plan.prompt()]].concat()`.

**`x` arm in `handle_owner_block_key`** (lines 2236-2294): add `let mut settle = false;` beside
`run/open/acknowledge/copy` (2238-2241), the arm `KeyCode::Char('x') if plain => settle = true,` beside
2245-2248, and `if settle { return Some(self.owner_settle(ctx)); }` beside 2283-2294. Because this
function returns `Some(..)` for every letter (2295-2296), the phase list's `x` is not reached.
`j`/`k` arms (2249-2254) are where the item sub-cursor movement hooks in (call a new
`owner_block::move_cursor` variant that walks item ids when the row is expanded).

**`owner_settle` method** - copy `owner_ack` (lines 2345-2386):
```rust
fn owner_ack(&mut self, ctx: &mut AppContext) -> ScreenAction {
    let outcome = {
        let Some(digest) = ctx.owner_digest(&self.alias) else { return ScreenAction::None; };
        let view = ctx.view_cache.get(&self.alias)
            .map(|cache| cache.owner_block.clone()).unwrap_or_default();
        let sk = owner_block::skeleton(digest, &view);
        let Some(at) = owner_block::resolve_cursor(&view, &sk.stops) else { return ScreenAction::None; };
        match &sk.stops[at] {
            owner_block::Stop::FoldLine => Err("Select a group row to acknowledge it".to_owned()),
            owner_block::Stop::Row(key) => match owner_block::group_for(digest, key) {
                Some(group) => owner_actions::ack_open(ctx, &self.alias, &group,
                    super::ack_confirm::AckHost::Detail(self.alias.clone())),
                None if key.lane == "blocked" => Err("...".to_owned()),
                None => Err("Group is no longer listed; nothing acknowledged".to_owned()),
            },
        }
    };
    ctx.needs_redraw = true;
    match outcome {
        Ok(action) => action,
        Err(message) => { ctx.status_message = Some((message, std::time::Instant::now())); ScreenAction::None }
    }
}
```
Settle differs: BLOCKED aggregate keys resolve through `owner_block::groups_for_copy(digest, key)`
(owner_block.rs 116-131) instead of refusing, and the item id comes from `view.item_cursor`.

`shown` (line 83) and `fit_cells` (line 11335) are `pub(super)`: usable from every sibling under
`src/ui/screens/`.

---

### `src/ui/screens/inbox.rs` (MOD: `x` arm, item sub-cursor, footer)

**Key arm** (lines 1909-1928):
```rust
let plain = owner_actions::is_plain_action_key(modifiers);
match code {
    ...
    KeyCode::Char('v') if plain => self.run_key(ctx, &mut owner_actions::launch_real),
    KeyCode::Char('o') if plain => self.open_key(ctx),
    KeyCode::Char('A') if plain => self.ack_key(ctx),
    KeyCode::Char('y') if plain => self.copy_key(ctx),
    KeyCode::Char('g') if plain => self.goto_key(ctx),
```
Add `KeyCode::Char('x') if plain => self.settle_key(ctx),`.

**`settle_key`** - copy `ack_key` (lines 1740-1776): `Self::selected(ctx)` (1650-1656) ->
`Err(()) => None`, `Ok(None)` (fold line) -> `Self::say(ctx, "Select a group row to settle it")`,
`Ok(Some(row))` -> `owner_settle::settle_open(ctx, &row.alias, .., AckHost::Inbox)`; on `Err(message)`
`Self::say(ctx, message)`. Unlike `ack_key`, merged rows (`row.src.parts.len() > 1`, 1751-1762) are
accepted (D-03 phase target = all parts). Fold rows: accept `None | Some(Fold::Stale)` like `copy_key`
(1800-1803); refuse `OldDebt`/`Acknowledged`.

**Group lookup by identity** (lines 953-966, `row_group`) and merged-entry lookup in `copy_key`
(1806-1814: `owner_block::entry_for(digest, &row.src.key, fold_days)`).

**Sub-cursor state** (lines 124-142): add `pub item_cursor: Option<String>` (an `OwnerItem::id`) to
`InboxViewState` exactly like `fold_cursor: Option<Fold>`. Clear it in `set_cursor` (411-433) when the
row changes, in `prune_view` (448-465) when the id vanished, and in `toggle_key` (1666-1694) on collapse.

**Movement** (`step` 1638-1646, `move_to` 1615-1619): `j`/`k` (1945-1952) must enter/leave item ids of
an expanded row before moving rows; PgUp/PgDn/Home/End (1953-1972) clear `item_cursor`.

**Highlight** (render loop 2043-2067 and `expansion_for` 971-989): `expansion_for` calls
`owner_block::inbox_expansion_lines`; change it to also get `(line index -> item id)` so the
`BodyItem::Extra` line of the selected item gets `owner_block::cursor_style()` patched (same patch as
line 875). Layout must not change (D-04).

**Footer** (lines 1201-1256, `FOOTER_KEYS: [FooterKey; 9]`): add an entry after the existing `[A]` entry (lines 1232-1237), same shape, labels are UI-SPEC territory:
```rust
FooterKey { key: "[x]", labels: [" settle", " settle", " settle"], rung3: true, rung4: false },
```
Bump the array length; the ladder test is `footer_ladder_matches_the_ui_spec` (3079) and the 80/65-col
asserts at 3053-3054. 38-UAT bounds the 80-column rung.

**Test helper:** `fn press(ctx: &mut AppContext, keys: &str)` (line 2881) drives key strings.

---

### `src/ui/screens/owner_block.rs` (MOD: item sub-cursor, item line indices)

**State** (lines 158-172): add `pub item_cursor: Option<String>` beside
`pub fold_cursor: Option<Fold>`.

**Item lines** (lines 1171-1218, `item_lines`): it iterates `items.iter().take(MAX_ITEMS)` (MAX_ITEMS=5,
line 49) and pushes 1 + detail + gap lines per item. Record `(lines.len(), item.id.clone())` before the
first push of each item so callers get a line-index map. `expansion_lines` (1222-1229) and
`inbox_expansion_lines` (1234-1258, merged parts at `shift + 2`) return/forward that map.

**Block model** (lines 1302-1330, `push_rows`): expansion lines are pushed at 1321-1327; record the
absolute body index of each item line there (beside `stop_lines`, 1303/1315) and add a field to
`BlockModel` (183-205) for the renderer.

**Cursor style** (lines 1506-1521): `cursor_style()` = Cyan + BOLD, `mark_cursor_row` patches the
line and swaps a leading `"  "` gutter for `"> "`. Item highlight = patch only (no gutter change, D-04).

**Cursor ops** (lines 1581-1688): `move_cursor` returns `Moved::{Within, PastLast, PastFirst}`; the
item walk wraps it (enter first item on Next when the row is expanded and has shown items). `clamp`
(1674-1688) is where a vanished `item_cursor` id is cleared. `toggle` (1633-1653) clears it on collapse.

**Fixture:** `#[cfg(test)] pub(crate) fn sample_owner_digest()` (line 1696) is the shared digest for
detail/ack tests.

---

### `src/state_reader/owner_items/rules.rs` (MOD: command phase helper, Pitfall 6)

**Analog:** `phase_labels` (lines 368-378) using private `scope_state` (111-115):
```rust
fn scope_state<'a>(state: &'a ProjectState, workstream: Option<&str>) -> &'a ProjectState {
    workstream
        .and_then(|name| state.workstreams.iter().find(|ws| ws.name == name))
        .map_or(state, |ws| &ws.state)
}
fn phase_labels(state: &ProjectState, file: &AuditFile) -> (String, String) {
    let roadmap_phase = scope_state(state, file.workstream.as_deref()).roadmap_phase(&file.phase);
    (
        roadmap_phase.map_or_else(|| slug_after_token(&file.phase_dir, &file.phase), |phase| phase.name.clone()),
        roadmap_phase.map_or_else(|| file.phase.clone(), |phase| phase.number.clone()),
    )
}
```
New `pub(crate) fn command_phase(state: &ProjectState, workstream: Option<&str>, phase: &str) -> String`
(name is a proposal) returning the roadmap number else the dir token; re-export it next to line 50 of
`src/state_reader/owner_items/mod.rs`:
`pub(crate) use rules::{is_command_token, is_workstream_name};`. Validate the result with
`is_command_token` (74-97). Move candidates (D-06) read `ProjectState.phases: Vec<RoadmapPhase>`
(`src/state_reader/mod.rs:64`; `RoadmapPhase { number, name, completed, .. }` in
`src/state_reader/roadmap_md.rs:7-24`) through `scope_state`; lookups go through
`ProjectState::roadmap_phase` (mod.rs:270), never `==` on `number`.

---

### `src/ui/screens/mod.rs` and `src/app.rs` (MOD)

**Module registry** (`src/ui/screens/mod.rs` lines 1-23, alphabetical `pub mod` list): add
`pub mod owner_settle;` after `owner_file_view` and `pub mod settle_popup;` in order.

**Only if Pitfall 8 picks a new variant:** `ScreenAction` (mod.rs 330-376) documents each variant with
its reason (see `WriteOsc52`, 368-375); `App::process_screen_action` (app.rs 3932-3974) handles it,
e.g. `Pop` body (3949-3954) then the `WriteOsc52` body (3971-3974). Test precedent: app.rs 6746-6752.
The native-copy completion overwrites the status (`mod.rs` 2008-2015, `Copied {n} via {tool}`); compose
the "focused, paste it there" hint into that outcome or set it after.

---

### `src/ui/screens/help.rs` (MOD)

Owner block row (line 375):
```rust
row("v o A y", "Owner block: run, open, acknowledge or copy the selected group, as in the inbox (detail view)"),
```
-> `"v o A y x"` and extend the description. The same tuple is asserted at 961-962 and 1037-1038.
Inbox section (lines 532-550): add `row("x", "Settle the selected check or phase: won't do, deferred permanently, or move to phase M; launches /gsd-verify-work after a confirm")`
after the `A` row (542). Descriptions must stay unique and fit `HELP_DESC_MAX` (591); rows wider are
wrapped by `wrap_owner_sections` (626).

---

### `src/ui/screens/render_escape_guard.rs` (MOD)

**Table** (lines 363-384, alphabetical): add `("SettleChooserScreen", "src/ui/screens/settle_popup.rs")`,
`("SettleConfirmScreen", ...)`, `("SettleReasonScreen", ...)`.

**Fixture arm** - copy `"AckConfirmScreen"` (lines 2288-2356): build the request/plan with `identity`
in every string field (alias, file, item name, phase name, reason, workstream, command), then
```rust
let mut over_inbox = probe_ctx(identity);
over_inbox.owner_digests.insert(identity.to_string(), hostile_owner_digest(identity));
vec![
    one_state("popup over the inbox", over_inbox, Box::new(SettleConfirmScreen::new(AckHost::Inbox, plan()))),
    one_state("popup over the detail view", probe_ctx(identity),
              Box::new(SettleConfirmScreen::new(AckHost::Detail(identity.to_string()), plan()))),
]
```
Input-screen precedent: `"SlugInputScreen"` arm (2868-2886) seeds the buffer with the hostile identity.

---

### `tests/owner_settle_guard.rs` (test, source scan) - NEW

**Analog:** `tests/owner_open_conformance.rs` lines 465-572.

**Walk** (465-490, `source_files`): recursive `read_dir` under `env!("CARGO_MANIFEST_DIR")/src`, returns
`(path, lines)` sorted, asserts non-empty. Restrict to the settle files by `Path::new(path).ends_with(..)`
(precedent: `ACK_SITE` check, 510).

**Hit finder** (503-517): skip `//` lines, collect `"{path}:{n}: {line}"`. Add a `#[cfg(test)]`
module skip copied from `tests/async_blocking_guard.rs` `scan` (lines 442-451 + 484-491):
```rust
if test_module.is_none() {
    if trimmed.starts_with("#[cfg(test)]") { pending_test = true; }
    else if pending_test && opens_module(trimmed) { test_module = Some(depth); pending_test = false; }
    else if pending_test && !trimmed.is_empty() { pending_test = false; }
}
...
depth += code.chars().filter(|c| *c == '{').count() as i32;
depth -= code.chars().filter(|c| *c == '}').count() as i32;
if let Some(base) = test_module { if depth <= base { test_module = None; } }
```
Markers list shape: `BLOCKING_MARKERS` (async_blocking_guard.rs 76-94) - use the write/spawn subset
(`fs::write(`, `File::create(`, `OpenOptions`, `create_dir`, `fs::rename(`, `remove_file(`,
`remove_dir`, `fs::copy(`, `set_permissions(`, `.write_all(`, `Command::new(`, `.spawn()`, `.output()`,
`.status()`).

**Anti-vacuity + planted-line tests** (519-572): `executable > N` floor (533-536), and
`the_narrowed_verb_guard_still_catches_the_read_verb` shape (549-572) with
`let planted = |path: &str, line: &str| vec![(path.to_owned(), vec![line.to_owned()])];` proving each
marker is caught and a comment is not.

---

### `tests/owner_settle_oracle.rs` (test, oracle round trip) - NEW, optional

**Analog:** `tests/owner_acknowledge_roundtrip.rs` lines 1-130.
Module doc states "temp copies only" and the fail-loud policy (1-15); `ALLOW_MISSING_ORACLE` const (29);
`Oracle::resolve`/`find` (41-77) via `install_candidates(None, &InstallRoots::from_env())`;
`copy_dir` (103-114); `temp_project(name)` (116-130) copying `tests/fixtures/owner/<name>/.planning`.
Run `query init.verify-work "<phase> <instruction>"` with `ACK_ENV_REMOVE` removed (81-100 shape).
Integration tests can only import `pub` items (line 26 imports `pub fn ack_target_for`/`ack_args`): make
the instruction/prompt builder `pub` if this test uses it.

---

### In-crate flow test with tree snapshot (D-13b)

Place in `settle_popup.rs` `mod tests` (needs `pub(crate)` items and `ctx_with_aliases`).
Context setup: ack_confirm.rs 289-303; fixture copy: owner_acknowledge_roundtrip.rs 103-130
(`tempfile` is already a dev-dependency). Launcher recording: `static RUNS: AtomicUsize` + `fn` pointer
(ack_confirm.rs 333-357), because the screen field is a `fn`, not a closure. Clipboard branch:
`owner_actions::force_clipboard(Some(probes))` + `owner_actions::seam_ran()` (owner_actions.rs 627-637).

---

### `README.md` (MOD)

- Key table row 340: `| Owner inbox | `1`-`4` `a` `j` `k` `Enter` `g` `o` `v` `A` `y` `Esc` | See Owner inbox |` -> add `` `x` ``.
- Intro 377-380 ("Three actions go further ... The app itself writes nothing into any project."): becomes
  four actions; `x` launches `/gsd-verify-work` with an instruction after a confirmation.
- Owner block paragraph 409-410 ("On a group row `v`, `o`, `A` and `y` work as in the inbox"): add `x`.
- Inbox key table starting at 423: add an `x` row.
- gsd-tools row 63 stays (settle runs no gsd-tools verb).

## Shared Patterns

### Plain-key gate (WR-01)
**Source:** `src/ui/screens/owner_actions.rs` 146-153
**Apply to:** the `x` arm in `inbox.rs` and `detail.rs`
```rust
pub(crate) fn is_plain_action_key(modifiers: crossterm::event::KeyModifiers) -> bool {
    use crossterm::event::KeyModifiers as M;
    !modifiers.intersects(M::CONTROL | M::ALT)
}
```

### Typed command, one argv element (D-11, CWE-88)
**Source:** `src/state_reader/owner_items/mod.rs` 504-529 (`RunCommand` + `Display`), `detail.rs` 3085-3122
**Apply to:** `owner_settle.rs` plan, `settle_*_argv`, popup command line, clipboard text
Only `RunCommand::fmt` spells `/gsd-verify-work {phase}` and ` --ws {ws}`; `prompt()` =
`format!("{} {}", command, instruction)`.

### Status line refusals
**Source:** `inbox.rs` 1658-1661 (`say`), `detail.rs` 2378-2385
**Apply to:** every `Err(String)` from `settle_target` / `settle_open`
```rust
ctx.status_message = Some((message, std::time::Instant::now()));
ctx.needs_redraw = true;
```
Wording precedent: `"Refused: {value} is not a safe {what}; nothing started"` (owner_actions.rs 355-356).
The inbox footer escapes status text at the consumer (`inbox.rs` 1323-1327, `fit_cells(&shown(message), width)`).

### Escaping project strings (OWN-11)
**Source:** `detail.rs` 83-85 (`shown`), 11335 (`fit_cells`); `crate::text::render_for_terminal` (text.rs 487)
**Apply to:** all three settle screens; register each with `adjudicate_screen!` and a
`render_escape_guard.rs` fixture. The instruction itself is sanitized first so `shown(instr) == instr`.

### Identity, not index (D-10/D-18)
**Source:** `inbox.rs` 44-49 (`InboxKey`), `owner_block.rs` 53-131 (`GroupKey`, `group_for`, `groups_for_copy`), `owner_block.rs` 658 (`entry_for`)
**Apply to:** item sub-cursor (`OwnerItem::id`), the plan's `key`, the Yes recheck.

### Clipboard (D-11 live session)
**Source:** `owner_actions.rs` 581-661 (`copy`, test seam `force_clipboard`/`seam_ran`), `mod.rs` 1966-2016 (`start_copy`)
**Apply to:** live-session branch of the settle launch.
```rust
pub(crate) fn copy(ctx: &mut AppContext, text: String, lines: usize) -> ScreenAction {
    ctx.needs_redraw = true;
    if let Some(tool) = crate::clipboard::pick_tool(&clipboard_probes()) {
        start_native_copy(ctx, tool, text, lines);
        return ScreenAction::None;
    }
    ctx.status_message = Some((format!("Sent {} to the terminal clipboard (OSC 52; ...)", line_count(lines)), Instant::now()));
    ScreenAction::WriteOsc52(crate::clipboard::osc52(&text))
}
```

### Default-No confirm with clip guard
**Source:** `src/ui/screens/confirm_popup.rs` 1-24 (contract), 110-142 (`on_key`), 233-300 (`render_confirm_popup`), 303-307 (`body_room`)
**Apply to:** `SettleConfirmScreen` (Yellow accent like ack, `ack_confirm.rs` 248).

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| (instruction sanitizer inside `owner_settle.rs`) | utility | transform | No existing function maps `'` `"` `\` and collapses `--` for gsd-core's `sed | xargs` parse; build from `is_invisible_formatting_char` + the sketch in 39-RESEARCH.md "Code Examples" (Pitfalls 2, 3, 9) |
| (item-level cursor walk) | component | event-driven | Only the fold-line sub-cursor exists (`fold_cursor`); the item walk over expansion lines is new. Pattern 2 in 39-RESEARCH.md defines it |

## Metadata

**Analog search scope:** `src/ui/screens/` (owner_actions, ack_confirm, confirm_popup, driver_steer, driver_launch, inbox, owner_block, detail, help, render_escape_guard, mod), `src/state_reader/owner_items/` (rules, mod), `src/state_reader/{mod,roadmap_md}.rs`, `src/text.rs`, `src/app.rs`, `tests/` (async_blocking_guard, owner_open_conformance, owner_acknowledge_roundtrip, owner_audit_conformance), `README.md`
**Files scanned:** 21
**Tracked-source gate:** all analog paths confirmed with `git ls-files` (no mirror paths)
**Pattern extraction date:** 2026-11-17
