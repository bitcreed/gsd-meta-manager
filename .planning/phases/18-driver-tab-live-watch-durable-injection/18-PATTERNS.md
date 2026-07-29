# Phase 18: Driver Tab, Live Watch & Durable Injection — Pattern Map

**Mapped:** 2026-07-29
**Files analyzed:** 19 (5 new, 14 modified)
**Analogs found:** 18 / 19 (one genuine greenfield: the bounded ring buffer)

> Every anchor below was read directly against the current tree during this pass.
> **Where a line number here disagrees with `18-CONTEXT.md`, this file is the corrected one** —
> CONTEXT was gathered against a slightly earlier read. The three known drifts are called out
> inline (`recompute_filtered_aliases`, `sorted_aliases`, `run.rs` close_input).

---

## The house convention this phase must imitate

This codebase does not write terse doc comments. **A doc comment here names the decision id, states
the failure the code prevents, and forbids the "obvious" refactor by name.** Two calibration
samples, quoted so the executor has the register:

`src/driver/kill.rs:159-174`:

```rust
/// Send `sig` to every process in the group led by `pgid`.
///
/// **Through `rustix`, never a shell-out to `/bin/kill` and never a hand-written
/// negative-pid call** (D-08). Both alternatives are ways to get the sign
/// convention wrong silently — `kill(pid, …)` and `kill(-pid, …)` differ by one
/// character and by the entire blast radius …
///
/// **A pgid of zero, or one that does not fit a `pid_t`, is a hard refusal and
/// never a fall-through.** This is the single most dangerous mistake reachable
/// from this function: `kill(0, sig)` signals *the caller's own process group*,
/// which under a TUI is the user's terminal session …
```

`src/driver/liveness.rs:41-62`:

```rust
/// **A `const` value rather than a `#[cfg]` block, and that is the whole design
/// decision.** Every downstream refusal CR-05 introduced … branches on the
/// *non-Linux* answer, which is unreachable at runtime on the only platform CI
/// runs. Expressed as an attribute those branches could never be executed, let
/// alone tested; expressed as a value they are pure functions that a Linux test
/// can hand `false` to.
```

Three structural tics to copy:

1. **"The order of the body is the decision."** Numbered lists in the fn doc explaining why each
   step is where it is — `driver::drive` (`src/driver/mod.rs:158-170`), `App::start_driver_run`
   (`src/app.rs:877-902`), `App::stop_driver_run` (`src/app.rs:982-1003`).
2. **Record what the code deliberately does *not* do.** `src/app.rs:693-704` is the model, and it
   is literally this phase's seam: *"It does NOT set `needs_redraw`. … Read the omission as a
   choice, not as a bug — Phase 18 is what adds the surface and the flag together."*
3. **Name the anti-refactor.** `src/app.rs:466-472` (*"Do not tidy this into its own interval."*),
   `src/journal/writer.rs:24-28` (*"One cosmetic consequence worth writing down so nobody 'fixes'
   it"*).

---

## File Classification

| File | New/Mod | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|---|
| `src/ui/screens/driver.rs` | NEW | screen/render module | request-response (render) | `detail.rs::render_pipeline_tab` `:2499-2637` + `render_browser_tab` `:2957` | exact |
| `src/ui/screens/driver_inject.rs` | NEW | screen (pushed input) | request-response | `src/ui/screens/enqueue.rs` (whole, 120 lines) | exact |
| `src/ui/screens/driver_start.rs` | NEW | screen (2-step wizard) | request-response | `enqueue.rs` (input) + `driver_confirm.rs` (confirm/dispatch) | role-match |
| `src/journal/inbox.rs` | NEW | I/O module | file-I/O, append + tail | `journal/writer.rs` (append) + `journal/reader.rs::tail_lines` `:140` | exact |
| ring buffer (in `screens/driver.rs` or `src/driver/output.rs`) | NEW | in-memory state | streaming/bounded | **none — greenfield** | none |
| `src/app.rs` | MOD | controller/dispatcher | event-driven | itself (`DriverJournalAppended` `:705`, `Tick` `:445`) | exact |
| `src/ui/screens/detail.rs` | MOD | screen | request-response | itself (six 11th-tab sites) | exact |
| `src/ui/screens/normal.rs` | MOD | screen | request-response | `alias_badge` `:62-78` | exact |
| `src/ui/screens/mod.rs` | MOD | state container | — | existing sibling maps `:139-226` | exact |
| `src/ui/screens/help.rs` | MOD | screen (overlay) | — | `detail.rs` viewport-scroll idiom `:3055-3086` | role-match |
| `src/action.rs` | MOD | message type | event-driven | `DriverJournalAppended` doc `:56-83` | exact |
| `src/journal/mod.rs` | MOD | model + path helper | file-I/O | `classify_change` `:263-295` (component check) | exact |
| `src/journal/writer.rs` | MOD | I/O | file-I/O | `read_active_run` `:449-472` | exact |
| `src/driver/run.rs` | MOD | run lifecycle | streaming/event-driven | drain loop `:620-658` | exact |
| `src/driver/mod.rs` | MOD | orchestration | async | `drive` `:171-220` | exact |
| `src/driver/reconcile.rs` | MOD | reader | batch | `reconcile_one` `:261-292` | exact |
| `src/cli.rs` | MOD | config/CLI | — | `claude_program` `:71-91` | exact |
| `src/executor/stream_json.rs` | MOD | model | transform | `TurnMessage` `:103-129` | exact |
| `tests/driver_inbox.rs` (new) | NEW | integration test | process | `tests/driver_reattach.rs` `:1-45` | exact |

---

## Pattern Assignments

### 1. NEW `src/ui/screens/driver.rs` (screen/render module, request-response)

**Module-layout decision (CONTEXT "Claude's Discretion"):** put the Driver tab's render + key
logic in its own module and have `detail.rs` delegate. `detail.rs` is 5,206 lines; every other
sub-tab renders inline, but every other sub-tab is 40 lines and this one is eight width/height
tiers, a ring-buffer renderer and a four-state injection widget. **Record the decision in the
module's header doc**, following the register of `src/driver/liveness.rs:35-41`.

**Analog A — two-pane split with `ListState`:** `detail.rs:2499-2637` (`render_pipeline_tab`).

```rust
// detail.rs:2521-2552 — the 40/60 split, the list, the selection, the highlight
let panes = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
    .split(area);
let left_area = panes[0];
let right_area = panes[1];

let selected = cache
    .map(|c| c.pipeline_selected.min(state.phases.len().saturating_sub(1)))
    .unwrap_or(0);

let list = List::new(items)
    .block(Block::default().borders(Borders::RIGHT).title(" Phases "))
    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
    .highlight_symbol("> ");

let mut list_state = ListState::default();
list_state.select(Some(selected));
frame.render_stateful_widget(list, left_area, &mut list_state);
```

**Deviation UI-SPEC mandates:** `Constraint::Percentage(60)` becomes
`Constraint::Min(DRIVER_DETAIL_MIN_CELLS)` (39). The convention that authorises the deviation is
`normal.rs:112-117`:

```rust
/// The exact number of terminal cells `compact_pipeline` produces:
/// five one-cell stage letters plus four two-cell inter-stage gaps
/// (`D  R  P  E  V`). The Status column must never be allocated fewer
/// cells than this, or the trailing `V` is clipped and a fully-verified
/// project renders identically to a mid-pipeline one (UIFIX-02 / CR-01).
const STATUS_COLUMN_MIN_CELLS: u16 = 13;
```

**Convention:** a named `const` whose doc *derives* the number from the render and names the bug
its absence caused. `PIPELINE_LINE_MAX_CELLS` / `DRIVER_DETAIL_MIN_CELLS` / `TAB_BAR_*_CELLS` all
get this treatment.

**Analog B — the scrolling viewer + `Cell<ViewportMetrics>` capture:** `detail.rs:3055-3086`
(`BrowserDepth::View`). This is the exact shape the output pane copies.

```rust
// detail.rs:3056-3086
if let Some(content) = &cache.browser_file_content {
    let styled_lines = crate::archive::render_markdown_lines(content);
    let total_lines = styled_lines.len() as u16;
    let gutter_width = (total_lines as usize).max(1).to_string().len() as u16 + 1;
    let file_chunks =
        Layout::horizontal([Constraint::Length(gutter_width), Constraint::Min(0)])
            .split(content_area);
    let text_area = file_chunks[1];

    let visible_height = text_area.height;
    self.browser_viewport.set(ViewportMetrics { total_lines, visible_height });
    let max_scroll = total_lines.saturating_sub(visible_height);
    let scroll = cache.browser_scroll_offset.min(max_scroll);

    let paragraph = Paragraph::new(styled_lines).scroll((scroll, 0));
    frame.render_widget(paragraph, text_area);
} else {
    let loading = Paragraph::new("Loading...").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(loading, content_area);
}
```

The `Cell` lives on the screen struct, with the interior-mutability reason spelled out
(`detail.rs:56-68`):

```rust
pub struct DetailScreen {
    pub alias: String,
    pub scroll_offset: u16,
    /// Last-rendered viewport metrics for the Docs (Browse) file view.
    /// Interior mutability: `Screen::render` takes `&self`, so the render pass
    /// cannot write into the view cache (see plan 14-02 CD-01).
    browser_viewport: Cell<ViewportMetrics>,
    …
}
```

**Add `driver_viewport: Cell<ViewportMetrics>` alongside these three** (not a fourth mechanism).
`ViewportMetrics` and `clamp_scroll` are at `detail.rs:27-39`.

**Analog C — the clamp-ordering invariant (UIFIX-04, non-negotiable):**

```rust
// detail.rs:878-885 — PageDown: ADD then CLAMP
BrowserDepth::View => {
    let vp = self.browser_viewport.get();
    cache.browser_scroll_offset = clamp_scroll(
        cache.browser_scroll_offset.saturating_add(PAGE_SCROLL_LINES),
        vp.total_lines,
        vp.visible_height,
    );
}

// detail.rs:976-985 — PageUp: CLAMP FIRST, subtract second
BrowserDepth::View => {
    // Clamp FIRST, subtract second — see the `k`/Up sibling.
    let vp = self.browser_viewport.get();
    cache.browser_scroll_offset = clamp_scroll(
        cache.browser_scroll_offset,
        vp.total_lines,
        vp.visible_height,
    )
    .saturating_sub(PAGE_SCROLL_LINES);
}
```

`clamp_scroll` (`detail.rs:33-39`) is the shared formula and **must not be re-derived**:

```rust
/// Clamp a stored scroll offset to the last-rendered viewport.
///
/// Uses the identical `total_lines - visible_height` formula the render path
/// already applies for display, so the two cannot drift apart.
fn clamp_scroll(offset: u16, total_lines: u16, visible_height: u16) -> u16 {
    offset.min(total_lines.saturating_sub(visible_height))
}
```

`PAGE_SCROLL_LINES: u16 = 20` is at `detail.rs:21`.

**Analog D — the D-R-P-E-V reuse (D-17).** Free functions over `&DiskInference`, no lifting:
`derive_all_stage_statuses` `detail.rs:3411`, `stage_color` `:3461`, `build_pipeline_line` `:3471`,
`build_stage_detail_lines` `:3503`. Call them exactly as `render_pipeline_tab` does at `:2568-2570`:

```rust
let stage_statuses = derive_all_stage_statuses(inf);
let pipeline_line = build_pipeline_line(inf, &stage_statuses);
```

If `driver.rs` is a sibling module these must become `pub(super)` or `pub(crate)` in `detail.rs` —
a visibility widen, **not** a move.

**Analog E — the "no state" / "no data" early-return guard:** `detail.rs:2504-2519`. Copy the shape
for the four empty states the Copywriting Contract names.

---

### 2. NEW `src/ui/screens/driver_inject.rs` (screen, request-response)

**Analog:** `src/ui/screens/enqueue.rs`, all 120 lines. Copy it file-for-file and change the footer
copy, the Enter body, and drop the `Tab` arm.

**Struct + trait shape** (`enqueue.rs:10-26`):

```rust
pub struct EnqueueScreen { pub alias: String }

impl EnqueueScreen { pub fn new(alias: String) -> Self { Self { alias } } }

impl Screen for EnqueueScreen {
    fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers, ctx: &mut AppContext)
        -> ScreenAction { … }
```

**Enter / Esc / Backspace / Char** (`enqueue.rs:27-87`) — note `Enter` on an empty buffer returns
`ScreenAction::None` and the screen stays open, which UI-SPEC Surface 4 mandates verbatim:

```rust
KeyCode::Enter => {
    if !ctx.input_buffer.is_empty() {
        …
        ctx.input_buffer.clear();
        ctx.needs_redraw = true;
        ScreenAction::Pop
    } else {
        ScreenAction::None
    }
}
KeyCode::Esc => { ctx.input_buffer.clear(); ctx.needs_redraw = true; ScreenAction::Pop }
KeyCode::Backspace => { ctx.input_buffer.pop(); ctx.suggestion_index = 0;
                        ctx.needs_redraw = true; ScreenAction::None }
KeyCode::Char(c) => { ctx.input_buffer.push(c); ctx.suggestion_index = 0;
                      ctx.needs_redraw = true; ScreenAction::None }
_ => ScreenAction::None,
```

**Render — body delegated, footer painted over** (`enqueue.rs:90-115`):

```rust
fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
    let footer_area = chunks[1];

    let detail = super::detail::DetailScreen::new(self.alias.clone());
    let main_chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
    detail.render_main_only(frame, main_chunks[0], ctx);

    let footer = Paragraph::new(Line::from(vec![
        Span::styled("  Enqueue> ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(&ctx.input_buffer),
        Span::styled("  [Tab] suggestions  [Enter] queue  [Esc] cancel",
                     Style::default().fg(Color::DarkGray)),
    ]));
    frame.render_widget(footer, footer_area);
}
```

`render_main_only` is `detail.rs:3164`; it contains the **duplicate render dispatch** at `:3195-3206`
— that is the second of the six 11th-tab sites, and it is the reason this screen shows the Driver
tab behind its footer at all.

**Convention this demonstrates:** the caret is a literal `Span::raw("_")` — there is no cursor
model. UI-SPEC's footer adds it as span 3; `enqueue.rs` currently omits it, so take the caret from
`driver_confirm.rs` / the Defaults text input rather than from here.

**What must NOT be copied:** `enqueue.rs:36-46` performs the filesystem write inline in the key
handler (`queue_md::save_queue`). D-06/D-28 forbid that for the inbox. The Enter arm dispatches an
`Action` instead — the model is `driver_confirm.rs:191-204`:

```rust
fn do_start_run(ctx: &mut AppContext, alias: &str) {
    …
    Action::DriverStartRequested { alias: …, command: DEFAULT_DRIVE_COMMAND.to_string() }
```

---

### 3. NEW `src/ui/screens/driver_start.rs` (screen, 2-step wizard)

**Analogs:** `enqueue.rs` for the input model (above) and `driver_confirm.rs` for the dispatch and
the y/n confirmation. `DEFAULT_DRIVE_COMMAND` is `driver_confirm.rs:46`; `DriverAction` is `:50`;
the prompt text that must now interpolate the *actual* command is `driver_confirm.rs:85`:

```rust
"Drive \"{alias}\"? An autonomous agent will run {DEFAULT_DRIVE_COMMAND} in that \
 project with full autonomy, including git operations. [y/n]"
```

**Tab-completion seed** — reuse, don't re-derive (`enqueue.rs:56-68`):

```rust
KeyCode::Tab => {
    if ctx.config.projects.contains_key(&self.alias) {
        if let Some(state) = ctx.project_states.get(&self.alias) {
            let suggestions = queue_md::suggest_next_commands(state);
            if !suggestions.is_empty() {
                ctx.suggestion_index = (ctx.suggestion_index + 1) % suggestions.len();
                ctx.input_buffer = suggestions[ctx.suggestion_index].clone();
                ctx.needs_redraw = true;
            }
        }
    }
    ScreenAction::None
}
```

**Two-field state:** `ctx.input_buffer` is a single shared `String` (`screens/mod.rs:231`). A
two-step wizard needs the committed command held on the screen struct itself
(`struct DriverStartScreen { alias: String, step: Step, command: String }`), with `input_buffer`
serving only the *active* field — that is the minimal extension and adds no `AppContext` field
(which matters: an `AppContext` field breaks two test fixtures, see §Tests).

**Dry-run preview (CUTTABLE, D-26):** `dry_run::build_report` is `src/driver/dry_run.rs:97`,
`render` is `:111`. It shells out to `git` twice synchronously — dispatch it via the
`spawn_blocking` + `Action` idiom in §6 below, never inline.

---

### 4. NEW `src/journal/inbox.rs` (I/O module, file-I/O append + tail)

**Analog A — the tail. Do not write a second tailer (D-04).** `reader::tail_lines`
(`src/journal/reader.rs:135-219`) already handles the missing file, the shrunk file, the concurrent
append, the non-UTF-8 fragment, and the oversize line without a newline. Call it with a
`TailCursor` the driver holds in its run state:

```rust
// reader.rs:135-140 — the doc that says which edge cases are covered
/// Read every complete line after `cursor`, leaving a torn tail unconsumed.
///
/// Four edge cases are handled deliberately, each marked at its branch:
/// a missing file, a file shorter than the cursor, a file growing during the
/// read, and a non-UTF-8 fragment.
pub fn tail_lines(path: &Path, mut cursor: TailCursor) -> io::Result<TailRead> {
```

`TailRead { lines, cursor, restarted, skipped_oversize }` is `reader.rs:100-133`; **both diagnostic
flags must be surfaced, not swallowed** — the doc at `:106-132` explains why in detail, and
`App::schedule_journal_tail` (`app.rs:314-329`) is the model for handling them:

```rust
if read.restarted {
    tracing::warn!(alias = %…, run_id = %…, restarted = true,
                   "journal tail: the file shrank and the cursor was reset");
}
if read.skipped_oversize {
    tracing::warn!(alias = %…, run_id = %…, skipped_oversize = true,
                   "journal tail: stepped over a line that exceeded the read bound");
}
```

**Analog B — the append.** `journal/writer.rs:66-84` for the handle, `:97-118` for the one-write
discipline:

```rust
// writer.rs:67-73
let file = OpenOptions::new()
    .create(true)
    .append(true)
    .open(journal_path)
    .with_context(|| format!("Failed to open journal at {}", journal_path.display()))?;

// writer.rs:113-118 — ONE write_all of a buffer that already ends in '\n'
let mut buffer = String::with_capacity(line.as_line().len() + 1);
buffer.push_str(line.as_line());
buffer.push('\n');
self.file
    .write_all(buffer.as_bytes())
    .with_context(|| format!("Failed to append to {}", self.path.display()))?;
```

The rule and its rationale are `writer.rs:5-9`:

> 1. **One record is one `write_all` of one buffer that already ends in `\n`.** Never a write of
>    the line followed by a write of the newline: two calls reintroduce exactly the torn-line
>    window that one call closes.

**The deliberate deviation D-06 requires:** the journal writer has **no** per-event durability
syscall (`writer.rs:13-15` says so and says why). The inbox append **must** add `sync_data()` after
`write_all`, and the doc comment on the inbox append must say *why it differs from its sibling* —
STEER-03's criterion is durability across TUI death, which the journal's rationale explicitly does
not claim. This is exactly the kind of divergence this codebase writes down.

**Analog C — path helper doc placement.** `journal/mod.rs:153-178` puts the layout rationale on the
path helper, not at the call sites, "following the `queue_md.rs:170-176` precedent". The `inbox`
field added to `RunPaths` (`mod.rs:133-146`) gets the same treatment. No `.gitignore` change is
needed — `RUNS_GITIGNORE_BODY` (`writer.rs:~245`) is `*` / `!*/` / `!.gitignore` / `!*/run.json`,
so any new per-run file is ignored by default; `tests/journal_gitignore.rs:142,194` already asserts
`inbox.jsonl` specifically.

---

### 5. NEW — the bounded ring buffer: **NO ANALOG EXISTS**

`grep -rn VecDeque src/` returns **zero hits**. This is genuinely greenfield; do not pretend a
precedent exists. Two things in the tree are the nearest *bounded-thing* precedents and supply the
convention, not the mechanism:

**Precedent A — the named-const-with-untuned-doc idiom** (`src/main_loop.rs:41-64`). This is the
exact register the three new constants must be written in:

```rust
/// The maximum number of executor events a single [`pump`] iteration applies.
///
/// This bound is what stops a burst starving a redraw: once `EXEC_BATCH` events
/// have been applied, control returns to the loop head, which draws a frame and
/// re-polls the input arm.
///
/// 64 is a defensible starting value with no tuning data behind it (RESEARCH
/// assumption A6, D-13). It is a named constant so tuning is a one-line change.
pub const EXEC_BATCH: usize = 64;
```

and the same phrasing at `journal/mod.rs:128-131`:

```rust
/// Like the three growth constants above it, this is a defensible starting
/// value with **no tuning data behind it** — it is a named constant so tuning is
/// a one-line change.
pub const MAX_TAIL_BYTES: u64 = 4 * 1024 * 1024;
```

→ `DRIVER_OUTPUT_RING_LINES = 2_000`, `DRIVER_OUTPUT_LINE_CELLS = 512`,
`DRIVER_OUTPUT_RECORD_MAX_LINES = 64` each carry this sentence.

**Precedent B — the byte-cap asymmetry** (`journal/writer.rs:97-118`): when the cap is hit, emit
**one** notice and then keep dropping, with the flag making "exactly once" structural rather than a
comment. The ring's "… {N} earlier lines dropped" affordance is the same shape: a counter on the
buffer, rendered as the first line, never a per-drop log.

**Precedent C — where it lives.** `AppContext`, keyed by alias, as a sibling map. The invariant is
stated four times (`screens/mod.rs:139-148`, `:159-183`, `:184-206`, `:207-226`); copy the
`journal_cursors` doc's bullet structure exactly:

```rust
/// Per-alias live driver state (D-19).
///
/// A **sibling map**, shaped exactly like `last_refresh` and
/// `archive_cache` above. Driver state deliberately does NOT live on
/// `ProjectState`: that type derives `PartialEq`, and `app.rs` uses the
/// derived equality to suppress the "Updated: {alias}" status message.
/// Driver state changes every few seconds — one 68-second spike turn
/// emitted 22 `thinking_tokens` events — so putting it there would flood
/// the status bar for an entire multi-hour run.
pub run_states: HashMap<String, RunState>,
```

Two bullets are **mandatory** for the new map, both quoted from `journal_cursors`:

- *"It holds offsets, sequence numbers, run ids and counts — **never a file handle or a join
  handle**. `Action` derives `Clone` and a handle is not `Clone` (D-20)."*
- *"**The map is pruned**, on the same 20-tick block as the reconciliation probe."* — CONTEXT's
  only remaining carry-forward obligation is a negative one; see §6.

**View state** (scroll offset, follow bit, selected run index) goes on `ProjectViewCache`
(`screens/mod.rs:60-112`), which is `#[derive(Default)]` and therefore additive with zero
constructor churn — unlike `AppContext`.

---

### 6. MODIFIED `src/app.rs` (controller, event-driven)

**Site A — `Action::DriverJournalAppended` handler, `app.rs:693-739`.** This is the named seam.
The comment above it *is* the spec:

```rust
// One tail read landed. This handler touches its own cursor entry
// and nothing else — modelled on `apply_exec_event`, which
// likewise records what it deliberately does *not* do.
//
// It does NOT set `needs_redraw`. This phase ships no surface that
// renders journal content (D-36), so a redraw here would schedule
// a frame that cannot differ from the one already on screen. Read
// the omission as a choice, not as a bug — Phase 18 is what adds
// the surface and the flag together.
//
// It also does NOT touch `ProjectState` (D-18) or `last_refresh` (D-14).
Action::DriverJournalAppended { alias, run_id, records, cursor } => {
    let key = (alias, run_id);
    let previous_last_seq =
        self.ctx.journal_cursors.get(&key).map_or(0, |stored| stored.last_seq);
    let gaps = crate::journal::reader::seq_gaps_from(previous_last_seq, &records).len();
    if gaps > 0 { tracing::warn!(…, count = gaps, "journal tail: sequence gaps observed"); }
    self.ctx.journal_cursors.insert(key, cursor);
}
```

**Change:** append `records` into the ring buffer, set `needs_redraw`, and **rewrite the comment**
— leaving the "Phase 18 is what adds the surface" prose above code that now does add it is exactly
the drift this codebase's comment discipline exists to prevent. The `gaps` count now feeds the
Yellow `! ` diagnostic row rather than only a `tracing::warn!`.

**Site B — `App::schedule_journal_tail`, `app.rs:282-367`.** The canonical
`spawn_blocking` → `Action` idiom, and the model for the TUI's inbox append, the driver's
run-list directory scan, and the dry-run report:

```rust
/// The read runs on `spawn_blocking` with its result returned as an
/// `Action` on a cloned sender, following the idiom this file already uses
/// for the re-parse and for session detection. No file I/O on the render
/// thread (D-16).
fn schedule_journal_tail(&mut self, alias: &str, project_path: &Path, run_id: &str) {
    let Some(tx) = &self.ctx.event_tx else { return; };
    let tx = tx.clone();
    …
    tokio::task::spawn_blocking(move || {
        …
        let _ = tx.send(Action::DriverJournalAppended { … });
    });
}
```

Note `crate::journal::run_paths(&planning_dir, run_id).journal` at `:296` — **this call becomes
fallible under D-27** and is one of the five sites the compiler will enumerate.

**Site C — the 20-tick block, `app.rs:445-492`.** Elapsed-time redraw (D-21) rides the existing
250 ms `Tick`, gated. The forbiddance of a second timer is at `:466-472`:

```rust
// The driver reconciliation probe rides THIS counter and
// must never get one of its own (D-13, ARCHITECTURE §4.4(c)
// are both explicit). Two timers polling `/proc` and
// `run.json` at slightly different phases would double the
// syscall load for no extra freshness and would make "how
// stale can the dashboard be?" a question with two answers.
// Do not tidy this into its own interval.
```

The gate goes in the `Action::Tick` arm before `session_poll_counter`: top screen is a
`DetailScreen`, its sub-view is `Driver`, and `observed_runs[alias].is_live()`. An unconditional
per-tick redraw is the failure mode named in D-21.

**Site D — `prune_driver_maps`, `app.rs:811-875`.** The new ring-buffer map **must** be added here
or the Phase 16 leak is reintroduced under a new name. Two passes, and the "newest" rule is a
string sort:

```rust
fn prune_driver_maps(&mut self) {
    let registered = &self.ctx.config.projects;
    self.ctx.journal_cursors.retain(|(alias, _), _| registered.contains_key(alias));
    self.ctx.run_states.retain(|alias, _| registered.contains_key(alias));
    self.ctx.observed_runs.retain(|alias, _| registered.contains_key(alias));
    …
    run_ids.sort_unstable();
    let excess = run_ids.len() - crate::journal::RETAIN_RUNS;
```

The doc at `:833-837` names why the sort needs no file read — carry that reasoning into the run-list
sort function too (D-16, UI-SPEC "record that in the sort function's doc comment").

**Site E — `App::start_driver_run`, `app.rs:877-980`.** `goal: Option<&str>` is already threaded;
only the caller changes. The `None` and its rationale live in the `DriverStartRequested` handler at
`:767-781`:

```rust
// The goal is `None`: … there is no screen to type
// one into — that is Phase 18's. An empty string is deliberately not
// passed instead, because `drive_argv` omits the flag entirely for
// `None` and would otherwise record an empty goal verbatim.
Action::DriverStartRequested { alias, command } => {
    #[cfg(unix)]
    self.start_driver_run(&alias, &command, None);
```

**The empty-string trap is live:** the goal picker's Step B accepts an empty buffer, and it must
map to `None`, not `Some("")`.

**Site F — `Action::DriverStopped` handler, `app.rs:791-807` (WR-15).** Today:

```rust
Action::DriverStopped { alias, run_id, outcome } => {
    self.ctx.observed_runs.remove(&alias);
    self.ctx.session_spawned_runs.remove(&run_id);
    self.ctx.status_message = Some((format!("{alias}: {outcome}"), std::time::Instant::now()));
    self.needs_redraw = true;
}
```

`outcome` is already a `String` (stringified at `app.rs:1039`, `outcome.to_string()`), so the
information is lost at the send site. The typed source is `StopOutcome`
(`src/driver/kill.rs:111-140`), which is `#[cfg(unix)]` — hence D-29's "small portable enum". The
model for a portable state type is `kill.rs:111-116`:

```rust
/// How a stop ended.
///
/// A **state**, not a message: `src/error.rs`'s module header states the house
/// rule that a driver UI needs something it can render rather than a string it
/// must parse, and this is the surface the stop returns through an `Action`.
```

Copy that doc verbatim in spirit for the new portable variant, and keep the rendered text alongside
it for the status line.

---

### 7. MODIFIED `src/ui/screens/detail.rs` — the six 11th-tab sites

All six verified against the current tree. **Missing one is the classic half-landing.**

| # | Site | Lines | Change |
|---|---|---|---|
| 1 | `TAB_TITLES` | `:41-52` | `[&str; 10]` → the `tab_titles(compact, driver_live) -> Vec<Line>` fn UI-SPEC Rule 1 mandates. The literal length `10` is a compile error waiting to help you. |
| 2 | `tab_index` | `:82-95` | `DetailSubView::Driver => 10` |
| 3 | `sub_view_from_index` | `:97-111` | `10 => DetailSubView::Driver` (keep `_ => PhaseList` fallback) |
| 4 | `switch_to_tab` | `:215-341` | Add a `Driver` load arm if the run list needs a scan; the git arm `:240-268` is the async-load model |
| 5a | render dispatch | `:1899-1910` | `DetailSubView::Driver => self.render_driver_tab(...)` |
| 5b | **duplicate** dispatch in `render_main_only` | `:3195-3206` | same arm — this is the one `EnqueueScreen`/`DriverInjectScreen` paints behind its footer |
| 6 | `footer_spans` | `:3772-3855` | `[1-9]tabs` → `[1-0/D]tabs` in the shared prefix `:3778-3779`, plus a `DetailSubView::Driver` match arm |

`DetailSubView` itself is `src/app.rs:16`.

**Footer-hint construction convention** (`detail.rs:3773-3782`) — `Span::styled("[k]", BOLD)` then
`Span::raw("ey-word  ")`, two trailing spaces:

```rust
let b = Style::default().add_modifier(Modifier::BOLD);
let mut spans = vec![
    Span::raw("  "),
    Span::styled("[Esc]", b),
    Span::raw("back  "),
    Span::styled("[1-9]", b),
    Span::raw("tabs  "),
    …
];
```

`footer_spans` is split out of `build_footer` (`:3768-3771`) precisely so the hint set is
assertable — *"`Paragraph` exposes no public text accessor, but a `Vec<Span>` concatenates
cleanly."* The three width forms UI-SPEC specifies should follow the same "assertable pure
function" shape, as should `tab_titles`.

The tab bar's current render is `detail.rs:1883-1896`:

```rust
let titles: Vec<Line> = TAB_TITLES.iter().map(|t| Line::from(*t)).collect();
let tabs_widget = Tabs::new(titles)
    .select(tab_idx)
    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
    .divider("|");
let tab_block = Block::default()
    .borders(Borders::BOTTOM)
    .title(format!(" Project: {} ", alias));
frame.render_widget(tabs_widget.block(tab_block), tab_area);
```

Duplicated verbatim at `:3178-3192`. Both must move to `tab_titles(...)`.

---

### 8. MODIFIED `src/ui/screens/normal.rs` (screen, request-response)

**Analog — `alias_badge`, `normal.rs:55-78`:**

```rust
/// Select the single leading badge for a dashboard alias cell.
///
/// Badge priority (Phase 14 UI-SPEC, `### Badge Priority Rule`):
/// pause > external-job-waiting > active session. At most one badge ever
/// renders, so the alias column stays aligned. The glyph is always a fixed
/// `&'static str` — never text derived from a HANDOFF file, so no handoff
/// body can leak onto the dashboard row.
fn alias_badge(
    is_paused: bool,
    external_job_waiting: bool,
    has_session: bool,
) -> Option<(&'static str, Color)> {
    if is_paused {
        // Pause badge takes priority over all other indicators
        Some(("\u{23F8} ", Color::Cyan))
    } else if external_job_waiting {
        // Hourglass: waiting on an async job, not stuck
        Some(("\u{25b6} ", Color::Yellow))   // (actual: \u{23F3})
    } else if has_session {
        Some(("\u{25b6} ", Color::Green))
    } else { None }
}
```

**Conventions to preserve:** escaped `\u{…}` literals (never a raw glyph in source), `&'static str`
return type (that is the mechanism enforcing "never derived from file content"), one `Option`
return (that is the mechanism enforcing "at most one badge"), and a doc comment naming the priority
order *and its reason*.

**Change:** two params → five (or a small `BadgeInputs` struct), two new arms at the top with
`Modifier::BOLD`. The return type must widen to carry the modifier —
`Option<(&'static str, Color, Modifier)>` or a small struct; keep `&'static str` for the glyph.

**`needs_human(..)` (D-14):** a new free function beside `alias_badge`, pure, taking `&ProjectState`
+ `Option<&ObservedRun>` + the run outcome. Purity is what makes it testable; the register to copy
is `driver/mod.rs:118-141` (`platform_refusal`), whose doc says *"Pure, and that is what makes it
testable at all."* The doc must name the D-14 fence: **do not wire it to `JournalEvent::Parked`
alone, nothing emits that until Phase 20.**

**`sorted_aliases` — actual line `screens/mod.rs:239-243`** (CONTEXT says `:218`; corrected):

```rust
pub fn sorted_aliases(&self) -> Vec<String> {
    let mut aliases: Vec<String> = self.config.projects.keys().cloned().collect();
    aliases.sort_by_key(|a| a.to_lowercase());
    aliases
}
```

Note: `sort_by_key`, not `sort_by` — commit `1984a6c` changed this for clippy 1.97
`unnecessary_sort_by`. **Keep `sort_by_key` when adding the mode**, or the lint returns.

**`recompute_filtered_aliases` — actual line `screens/mod.rs:252-…`** (CONTEXT says `:233`):

```rust
pub fn recompute_filtered_aliases(&mut self) {
    use crate::app::{format_phase_display, parse_filter, FilterColumn};
    let all = self.sorted_aliases();
    if self.filter_text.is_empty() { self.filtered_aliases = all; return; }
    let (term, column) = parse_filter(&self.filter_text);
    let term_lower = term.to_lowercase();
    self.filtered_aliases = all.into_iter().filter(|alias| {
        let state = self.project_states.get(alias);
        match column {
            FilterColumn::Name => alias.to_lowercase().contains(&term_lower),
            …
```

`FilterColumn` is `src/app.rs:31`; `parse_filter` is `src/app.rs:47`. Add
`FilterColumn::NeedsHuman` + the `/h` suffix + one match arm. UI-SPEC's `//h` case falls out of the
existing grammar with no special case (empty term matches everything) — **assert that in a test
rather than adding a branch for it.**

---

### 9. MODIFIED `src/ui/screens/mod.rs` (state container)

`AppContext` is `:114-235`; `ProjectViewCache` is `:60-112`.

**The additive/breaking split matters:** `ProjectViewCache` is `#[derive(Default)]`, so new fields
there cost nothing. `AppContext` is constructed field-by-field in **three** places — the production
site in `app.rs` and two test fixtures (§Tests) — so every new `AppContext` field breaks both
fixtures. Budget for it; do not discover it.

New-field doc template — copy the bullet structure of `journal_cursors` (`:159-183`) or
`session_spawned_runs` (`:207-226`), which is the most complete example: what it is, why it is a
sibling and not on `ProjectState`, what it may not hold (`Action` is `Clone`), and whether/how it
is pruned.

---

### 10. MODIFIED `src/ui/screens/help.rs` (screen, overlay)

Current: `centered_rect(area, 60, 70)`, a hardcoded `Vec<Line>` at `:40-77`, `Paragraph` in a
`Block`, **no scroll state at all** and a `handle_key` (`:25-33`) that consumes everything except
`?`/`Esc`.

```rust
let popup_area = centered_rect(area, 60, 70);
frame.render_widget(Clear, popup_area);
let help_text = vec![
    Line::from(Span::styled("Keybindings", Style::default().add_modifier(Modifier::BOLD))),
    Line::from(""),
    Line::from("  j / Down      Move down"),
    …
];
let block = Block::default().borders(Borders::ALL).title(" Help ");
let paragraph = Paragraph::new(help_text).block(block);
frame.render_widget(paragraph, popup_area);
```

**Adding scroll = importing the §1 Analog B+C pattern wholesale:** a `Cell<ViewportMetrics>` on
`HelpScreen`, a `scroll_offset: u16`, `Paragraph::new(lines).scroll((offset, 0))`, and the two
`clamp_scroll` orderings. `clamp_scroll` and `ViewportMetrics` are private to `detail.rs:27-39` —
widening them to `pub(super)` in `screens/mod.rs` or hoisting them into `screens/mod.rs` is the
right move, and it also serves `driver.rs`. **One copy of the clamp formula, not three.**

The existing filter-syntax section (`:64-71`) is where the `/term/h` and `//h` rows go; the
`Modifier::DIM` footer line (`:75-78`) is the model for the `▾ more` indicator.

---

### 11. MODIFIED `src/action.rs` (message type, event-driven)

**Analog — the `DriverJournalAppended` doc, `action.rs:56-83`.** New variants must carry the same
three-part justification: sizing, `Clone`-safety, and why the payload is shaped as it is.

```rust
/// One byte-offset tail of a run journal completed (D-12).
///
/// The records travel in a `Vec` rather than by value, and that is a
/// deliberate sizing choice rather than a habit: a `Vec` is a fixed 24
/// bytes regardless of what it holds, so this variant is 24 + 24 + 24 + 8
/// = 80 bytes. RESEARCH §8.3 measured `Action` at 120 bytes and measured
/// `clippy::large_enum_variant` as firing on a 200-byte *difference*
/// between the largest and second-largest variants — so **nothing here
/// needs boxing** and nothing should be boxed reflexively. …
///
/// Every field is plain data — `String`, `Vec`, and a `Copy` cursor of two
/// `u64`s — so `Action` stays `Clone` and no file handle or join handle
/// leaks into a message type (D-20).
```

`DriverStartRequested` (`:98-113`) already names this phase:

> Today the only production sender fills it from `driver_confirm::DEFAULT_DRIVE_COMMAND`; **Phase
> 18's command picker is what makes the field carry more than one value.**

→ add `goal: Option<String>` to that variant and update the prose. New variants likely needed:
`DriverInjectRequested { alias, run_id, id, text }`, `DriverInjectWritten { alias, id, result }`,
`DriverDryRunLoaded { alias, report }`. All plain data.

---

### 12. MODIFIED `src/journal/mod.rs`

**WR-02 fix — the in-repo model is `classify_change`, `mod.rs:263-295`:**

```rust
let mut parts: Vec<&str> = Vec::new();
for component in relative.components() {
    match component {
        Component::Normal(name) => match name.to_str() {
            Some(name) => parts.push(name),
            // Non-UTF-8 component: unrecognised, so it takes the default.
            None => return ChangeKind::Planning,
        },
        // `..`, a root, or a prefix inside the relative remainder means the
        // path is not a plain descendant. Do not try to normalise it here —
        // normalising is exactly the filesystem access this function forbids.
        _ => return ChangeKind::Planning,
    }
}
```

**The exact shape for `run_paths`** — `Path::new(run_id).components()` must yield exactly one
`Component::Normal` whose `OsStr` equals `run_id`. Signature becomes
`pub fn run_paths(planning_dir: &Path, run_id: &str) -> Option<RunPaths>`, which conscripts the
compiler to find all five callers: `JournalRun::start` (`mod.rs:608`), `writer::read_active_run`
(`writer.rs:449`), `reconcile::reconcile_one` (`reconcile.rs:264`), `App::schedule_journal_tail`
(`app.rs:296`), and the new inbox path. The existing doc `mod.rs:153-167` stays and gains the
validation rationale.

**`JournalEvent::Interjected` — `mod.rs:379-387`:**

```rust
/// A message the user injected into a running agent.
///
/// **Schema only in this phase — Phase 18 emits it** (D-36).
Interjected {
    /// The injected text.
    text: String,
    /// Whether it reached the agent, as opposed to being queued or dropped.
    delivered: bool,
},
```

Widen with `#[serde(default)] id: Option<String>`. Schema-safety is structural: no
`deny_unknown_fields` anywhere in `src/journal/` (mechanically guarded), `kind` is a plain `String`
(`reader.rs:232`), unknown payload survives in the flattened `Map` (`reader.rs:234-235`).

**Three coupled edits, all guarded by existing tests:**

- `EMITTED_KINDS` `mod.rs:523-533` — add `"interjected"` **and** the new acted-on kind.
- `RESERVED_KINDS` `mod.rs:543` — remove `"interjected"`. The two lists are complements and
  `every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase` proves it.
- `is_content()` `mod.rs:454-459` — `Interjected` is already content; **keep that property** and
  decide deliberately whether the acted-on variant is content (it carries only an id → lifecycle,
  so probably not; say which and why).

**`from_exec_event` text projection — `mod.rs:812-867`. The two lines to replace:**

```rust
ExecutionEvent::Message(message) => JournalEvent::ExecEvent {
    stream: stream_label(message).to_string(),
    text: format!("{message:?}"),          // ← mod.rs:824
},
…
ExecutionEvent::TurnCompleted(result) => JournalEvent::ExecEvent {
    stream: "turn_completed".to_string(),
    text: format!("{result:?}"),           // ← mod.rs:848
},
```

Every sibling arm already carries a **real string** (`raw.clone()`, `line.clone()`), so the
readable projection is the arm that matches its neighbours, not a novelty. The `LineTruncated` arm
(`:838-843`) is the model for a *composed* projection:

```rust
// Mirrors the field shape the executor already uses for this condition
// (`ExecutionEvent::LineTruncated { bytes, prefix }`), so the journal
// reports the same two facts the stream did rather than inventing a
// third rendering of them.
ExecutionEvent::LineTruncated { bytes, prefix } => JournalEvent::Diagnostic {
    code: "line_truncated".to_string(),
    detail: format!("{bytes} bytes; prefix: {prefix}"),
},
```

`JournalRun::record(&JournalEvent)` (`mod.rs:731`) is the append escape hatch for the interjection
records.

---

### 13. MODIFIED `src/journal/writer.rs` — `read_active_run` (`:449-472`)

```rust
/// The run id the `active` pointer names, **only if that run directory exists**.
///
/// The existence check is the pointer's authority rule made mechanical: a
/// pointer naming a directory that is not there is stale … and the listing wins.
/// A disagreement is logged, because it is a real anomaly even though it is
/// recoverable.
pub fn read_active_run(planning_dir: &Path) -> Option<String> {
    let root = runs_root(planning_dir);
    let raw = std::fs::read_to_string(root.join("active")).ok()?;
    let run_id = raw.trim();
    if run_id.is_empty() { return None; }
    if !root.join(run_id).is_dir() {
        tracing::warn!(
            "the active pointer under {} names a run directory that does not exist; \
             the directory listing is authoritative, so the pointer is ignored",
            root.display()
        );
        return None;
    }
    Some(run_id.to_string())
}
```

**`root.join(run_id).is_dir()` is the read-side WR-02 hole**: a traversing id satisfies `is_dir()`,
and the `active` file lives inside the driven project, so **the agent controls it**. Add the
component check *before* the `is_dir()` call, with a `tracing::warn!` in the same register as the
existing one ("names a run directory that does not exist" → "names a path that is not a single
directory component; refusing"). The existing `Option`-returning shape means no signature change.

---

### 14. MODIFIED `src/driver/run.rs` — stdin lifetime + the drain loop

**The line that makes STEER-01 impossible — `run.rs:597-618`** (CONTEXT says `:605`; the comment
starts at `:597`):

```rust
// One command means one message, so signal end-of-input immediately. EOF is
// "no more input", not "stop": the CLI drains what is queued, finishes and
// exits on its own. Without it a real `claude` would wait for a second turn
// that this phase never sends.
//
// Raced the same way, and the second-longest await in the window. By this
// point there **is** a handle, so a stop here takes the ordinary layer-2 path
// through `Executor::cancel` and no second teardown is written (D-06.2).
tokio::select! {
    biased;
    _ = term.recv() => { shutdown_on_terminate(&executor, &mut handle, &mut run.journal).await;
                         return Ok(()); }
    result = handle.close_input() => {
        if let Err(err) = result {
            tracing::warn!(kind = ?err, "could not signal end-of-input to the agent");
        }
    }
}
```

D-11 moves this whole `select!` **into** the drain loop, fired at a `TurnCompleted` boundary with an
empty inbox. The `biased` + terminate-first + `return Ok(())` structure travels with it unchanged;
the comment must be rewritten from "one command means one message" to the D-11 four-step rule.

**The drain loop — `run.rs:620-658`. The new inbox arm goes here, and arm order is the decision:**

```rust
// `biased`, with the terminate arm FIRST, following `src/main_loop.rs:130`.
//
// Arm order is the decision, not a formality. Without `biased` the macro
// picks a ready arm at random, and with a fast agent the event arm is
// essentially always ready — so a stop request would lose the race for as
// long as the stream kept producing … A stop that loses to a busy event queue is a
// stop the user experiences as ignored (D-06.1).
//
// `tokio::select!` drops the other arms' futures before it runs the chosen
// arm's body, which is what lets the terminate arm take `&mut handle` while
// the event arm's future borrowed it.
loop {
    tokio::select! {
        biased;
        _ = term.recv() => { shutdown_on_terminate(…).await; return Ok(()); }
        event = handle.events.recv() => {
            match event {
                Some(event) => {
                    if let Err(err) = run.journal.record_exec(&event) {
                        // The error KIND only. Never a message body, which
                        // could carry agent output (T-17-05).
                        tracing::warn!(kind = ?err.kind(), "journal write failed");
                    }
                }
                None => break,
            }
        }
    }
}
```

**Rules the new arm inherits:** terminate stays first; the poll arm (`tokio::time::interval`,
500 ms–1 s, D-discretion) goes **last**; the inbox read itself is blocking fs work and goes on
`spawn_blocking` (D-28); the `tracing::warn!(kind = ?…)` "error kind only, never a body" discipline
applies to every new log line in this file.

**Echo correlation (D-08):** the driver holds `{id → text}` for delivered messages and watches its
own `handle.events` for `ExecutionEvent::Message` with a `User` `TurnMessage` whose
`is_replay == true`. `is_replay` is `stream_json.rs:121-122`; the semantics doc `:113-120` is the
spec:

```rust
/// The `--replay-user-messages` echo marker — the only delivery ack the
/// design has.
///
/// It is camelCase on the wire **and absent rather than `false`** on
/// non-replay messages, so it needs both a rename and a default. It is also
/// emitted at *dequeue*, not at receipt: a message written mid-turn is
/// echoed after the preceding turn's `result`, which makes this a "started
/// processing" ack rather than a "received" ack (D-31).
#[serde(default, rename = "isReplay")]
pub is_replay: bool,
```

---

### 15. MODIFIED `src/driver/mod.rs` — WR-10 `spawn_blocking` boundaries

**Analog for a `spawn_blocking` wrap that returns a value:** `app.rs:298-366` (§6 Site B). Inside
the driver there is no `Action` channel, so the shape is
`tokio::task::spawn_blocking(move || …).await` — and the **critical constraint** is `RunLock`:
it holds a `File` whose descriptor **is** the lock (`driver/lock.rs:112-122`, no `Drop` impl), so
it must be *moved back out* of the blocking task and held for the run's duration, never dropped
inside it. `lock::acquire` is `driver/lock.rs:151`.

**The register for the doc comment** — `driver/mod.rs:158-170`, "the order of the body is the
decision":

```rust
/// Run one GSD command against `alias`, or refuse.
///
/// The order of the body is the decision:
///
/// 1. Look the alias up, refusing an unregistered one with
///    [`OptInError::UnknownAlias`].
/// 2. Pass the entry through the **single production call** to the capability
///    constructor. That call site is unique on purpose, and the uniqueness is a
///    property `tests/spawn_seam_guard.rs` can check while "every branch
///    remembers to gate" is not.
```

`drive` is `:171-220`; `dispatch` is `:224-230`; the dry-run branch that shells out to `git`
synchronously is `:181-199` and is one of the named WR-10 sites.

---

### 16. MODIFIED `src/driver/reconcile.rs` — `reconcile_one` (`:261-292`)

```rust
pub fn reconcile_one(alias: &str, project_root: &Path) -> Option<ObservedRun> {
    let planning_dir = project_root.join(".planning");
    let run_id = writer::read_active_run(&planning_dir)?;
    let paths = run_paths(&planning_dir, &run_id);      // ← becomes `?` under D-27
    let facts = read_run_facts(&paths.dir)?;
```

Already `Option`-returning, so the fallible `run_paths` threads through as a single `?`. `ObservedRun`
is `:81-111` with `liveness: Liveness` (**not** the removed `live: bool`); `is_live()` is `:132`.
`reconcile_all` (`:307`) sorts by alias so the handler can equality-guard the redraw — the run-list
sort in the Driver tab is the mirror-image case and gets the "lexicographic == chronological" doc.

---

### 17. MODIFIED `src/cli.rs` — WR-16 (`:71-91`)

```rust
/// Test and development only: the program to spawn instead of `claude`
///
/// It exists because the driver is a separate process that constructs
/// its own executor, so `ClaudeExecutor::with_program` does not reach
/// it. The TUI's own spawn argv never emits this flag. A hidden flag
/// rather than an environment variable is deliberate: an env var is
/// inherited by children, so a stray `GSD_*` in the user's shell would
/// silently reach a TUI-spawned driver … whereas a flag must be passed on
/// purpose by a caller whose argv builder is itself unit-tested.
#[arg(long, hide = true)]
claude_program: Option<PathBuf>,
```

Add `#[cfg(debug_assertions)]` to both fields **and** extend the doc with the accepted consequence
(`cargo test --release` no longer builds the integration tests that pass them) — this codebase
records accepted consequences in the doc, e.g. `writer.rs:24-28`. The `Diagnostic
{ code: "agent_program_overridden", detail }` emission follows `mod.rs:838-843`'s
`code`/`detail` shape exactly.

---

### 18. MODIFIED `src/executor/stream_json.rs` — `TurnMessage` (`:103-129`)

```rust
/// An `assistant` or `user` turn message.
///
/// Both types share one shape. The message body itself is deliberately not
/// modelled: this phase routes envelopes, and the content blocks are Phase 18's
/// rendering concern.
#[derive(Debug, Clone, Deserialize)]
pub struct TurnMessage { … }
```

**That doc names this phase; update it rather than leaving it.** The field-level conventions to
imitate are all present in the same struct: `#[serde(default)]` on everything optional, `rename`
only where the wire is camelCase, and a comment where the wire *isn't*:

```rust
/// snake_case on the wire — do not "fix" this with a blanket rename.
#[serde(default)]
pub claude_code_version: Option<String>,
```

Minimal content-block modelling means: `message: Option<MessageBody>` with
`content: Vec<ContentBlock>`, each block `#[serde(default)]`-tolerant and carrying at least
`{ type, text }`. **Every added field must be `Option` or `#[serde(default)]`** — the file's whole
posture is envelope tolerance, and a required field turns an unknown block shape into a parse
failure that lands in `ExecutionEvent::Unparseable`.

---

## Shared Patterns

### S1 — `spawn_blocking` → `Action` (applies to: injection append, run-list scan, dry-run report)
**Source:** `src/app.rs:276-366`. Guard on `event_tx` with a `let … else { return; }`, clone the
sender, move owned data in, `let _ = tx.send(Action::…)`. The doc must say *"No file I/O on the
render thread (D-16)"* — that phrase is the searchable marker.

### S2 — The sibling-map invariant (applies to: the ring buffer, any new per-alias state)
**Source:** `src/ui/screens/mod.rs:139-148` + `:159-183`. Never on `ProjectState` (it derives
`PartialEq` and that equality suppresses "Updated: {alias}" spam). Never a handle (`Action` is
`Clone`). Always pruned in `prune_driver_maps`.

### S3 — Error/log hygiene (applies to: every new log line in `driver/` and `journal/`)
**Source:** `src/app.rs:301-309`, `src/driver/run.rs:649-652`. The error **kind** only — never a
path, never a message body, because a body can carry agent output (T-17-05, D-28). Counts are
content-free by construction and are safe (`writer.rs:~150`).

### S4 — Refuse visibly, never silently (applies to: `i` with no live run, opt-in, concurrency cap)
**Source:** `src/app.rs:906-910`, `:925-929`, `:1008-1012`.
```rust
let Some(project) = self.ctx.config.projects.get(alias) else {
    self.ctx.error_message = Some(format!("No registered project named '{alias}'"));
    self.needs_redraw = true;
    return;
};
```
UI-SPEC's Copywriting Contract supplies the exact strings; the *shape* is this.

### S5 — Named `const` whose doc derives the number and names the bug
**Sources:** `normal.rs:112-117` (`STATUS_COLUMN_MIN_CELLS`, UIFIX-02/CR-01),
`main_loop.rs:41-49` (`EXEC_BATCH`, untuned), `journal/mod.rs:128-131` (`MAX_TAIL_BYTES`, untuned).
Applies to all six new constants.

### S6 — Pure functions so the dangerous branch is testable
**Sources:** `driver/mod.rs:129-135`, `liveness.rs:41-52`, `journal/mod.rs:243-247`
(*"Purity is also what lets it be tested exhaustively against path shapes"*).
Applies to `needs_human`, `tab_titles`, `footer_spans`, the injection-state derivation, the run-list
sort, and the sanitiser.

### S7 — No raw glyphs in source
**Source:** `normal.rs:69-74`, `detail.rs:2597`. Always `"\u{23F8} "`, `"\u{25b6} "`. The new
`◆`/`⚑`/`○`/`◐`/`●`/`✗`/`◇`/`■`/`»`/`·`/`‹`/`›` all become `\u{…}` `&'static str` constants.

---

## Test Analogs

### T1 — Inline `#[cfg(test)] mod tests` for all UI logic
There are **no UI/render integration tests** in this repo. `detail.rs:4674-5206`,
`normal.rs:717-1102`, `driver_confirm.rs:299-523`.

**The key-press helper — `detail.rs:5041`:**
```rust
fn press(screen: &mut DetailScreen, ctx: &mut AppContext, code: KeyCode) {
    screen.handle_key(code, KeyModifiers::NONE, ctx);
}
```

**The lesson that produced these tests — `detail.rs:4931-4938`, quote it in any new scroll test:**
```rust
// Every scroll test above calls `clamp_scroll` directly and simulates the
// key press with hand-written arithmetic. That is exactly why the
// up-direction defect was invisible: `clamp_scroll` was always correct,
// and the Up/PageUp handlers never called it. The tests below drive the
// real `handle_key` through a constructed `AppContext`.
```

### T2 — **The two `AppContext` fixtures that break when a field is added**
- `driver_confirm.rs:310-357` — `ctx_with_project(root)`, the canonical **full-field** fixture,
  returns `(AppContext, UnboundedReceiver<Action>)` so a test can assert *no `Action` was sent*.
- `detail.rs:4943` — `test_ctx()`, the minimal one.

**Both enumerate every `AppContext` field by name.** Every new field breaks both. Plan for it.

The assertion discipline to copy (`driver_confirm.rs:359-370`):
```rust
/// CTRL-03 at the affordance layer.
///
/// The load-bearing half is the **second** assertion. A screen that set the
/// message and dispatched anyway would pass a test that only checked
/// `error_message` — and the run would start regardless of the message the
/// user was shown. So the absence of a sent `Action` is asserted directly.
```
→ the injection screen's "no live run" guard needs exactly this: assert the status message **and**
assert nothing was sent.

### T3 — `tests/spawn_seam_guard.rs` — the grep-guard idiom (for WR-16's `#[cfg]`)
```rust
// One rule governs every assertion below: **a comment is not a guard; the test
// is.** … Prose cannot enforce either. This file does.
//
// It is an integration test rather than an in-source one because it reads the
// source tree, and a test that walks `src/` has no business living inside it.
//
// **The walk covers `src/` only, never `tests/`.** That is what lets this file's
// own prose name the tokens it forbids without invalidating its own gate.

/// The tree under audit. Resolved at compile time, so the test is
/// cwd-independent — the idiom `tests/executor_lifecycle.rs:26-29` already uses.
const SRC_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
```
Plus the declared-allowlist idiom (`SPAWN_ALLOWLIST`, `:36-60`), whose doc says *"This is a declared
allowlist, not a habit … that deliberate edit is the entire point."* Extend this file with the
`claude_program`-must-be-`#[cfg(debug_assertions)]` assertion — do **not** start a second guard file.

### T4 — `tests/driver_reattach.rs` — the "real process required" integration test
Header (`:1-26`) is the model for the new `tests/driver_inbox.rs`, including `#![cfg(unix)]` and the
refusal to write an aspirational `#[ignore]`d test:
```rust
// **What this file deliberately does NOT attempt: live output re-streaming after
// a restart.** … There is no `#[ignore]`d aspirational
// test for the impossible half — an ignored test for something physics forbids
// is a promise, not a plan.
```
Fixture-path constants (`:38-45`) use `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/…")`.

### T5 — `tests/driver_lock.rs:201-215` — the WR-10 deadlock, recorded as observed
```rust
// `lock::acquire` is a **synchronous** syscall inside an async fn, so a blocking
// `flock` parks the OS thread rather than yielding. `tokio::time::timeout` can
// only fire when the task awaiting it is polled … so wrapping the call directly, on a
// current-thread runtime, produces a bound that cannot fire: the timer, run A, and the
// timeout itself all share the one parked thread, and the suite deadlocks instead of
// failing. That was observed, not theorised.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
```
Any new test that races a blocking call needs `flavor = "multi_thread"` and must await a
`tokio::spawn` **handle**, not the call.

### T6 — `tests/executor_transport.rs` — the wire-shape and no-buffering pins
`:321` pins the `send` wire shape byte-for-byte:
```rust
assert_eq!(
    written[1],
    r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"second message"}]}}"#,
    "send() must write the same shape as the released prompt"
);
```
`:364` `a_message_sent_mid_turn_is_not_buffered_by_the_driver` is the **D-02 regression guard** — if
a plan proposes an AP3 flush buffer, this test is what fails it. The `reacting(&stdin_log)` +
`DrivableProject::for_testing_bypassing_opt_in` fixture pair is the model for driving the injection
path end-to-end.

### T7 — `tests/journal_gitignore.rs:142,194` — `inbox.jsonl` already asserted ignored
The ignore posture exists; the file does not. When `inbox.jsonl` starts being written, these
assertions become non-vacuous rather than needing to change.

### T8 — Guard tests that will fail if the coupled edits are missed
`EMITTED_KINDS`/`RESERVED_KINDS` complement test
(`every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase`, `journal/mod.rs` tests) and
`the_journal_cursor_is_copy` (`reader.rs:85-88`). Neither needs writing — both need not being
broken silently.

---

## No Analog Found

| Item | Role | Data Flow | Reason |
|---|---|---|---|
| Bounded ring buffer for live output | in-memory state | streaming | `grep -rn VecDeque src/` returns **zero**. No bounded in-memory collection exists anywhere in `src/`. Nearest precedents are conventions, not mechanisms — see §5 (`main_loop.rs:41-64` named-const idiom, `journal/writer.rs:97-118` cap asymmetry, `journal/mod.rs` byte caps). |
| The auto-follow bit | view state | streaming | Explicitly does not exist (D-19). Every scrolling pane in the repo is a static document; none has a tail to follow. Build it on top of the `Cell<ViewportMetrics>` + `clamp_scroll` machinery, but the follow semantics themselves are new. |
| Tab-bar width tiering / windowing | render | — | No widget in the repo windows its own content. The three-tier *table* layout in `normal.rs:119-129` is the nearest thing and supplies the "resolve tiers in one function so render tests cannot drift" convention, not the algorithm. |
| ANSI/control-character sanitiser | utility | transform | Nothing in `src/` strips ESC today. `redact::RedactedLine` (`journal/redact.rs`, used at `writer.rs:110`) is the nearest — it caps payload bytes and redacts, but does not sanitise for terminal rendering. Build beside it, do not extend it. |

---

## Metadata

**Analog search scope:** `src/ui/screens/`, `src/journal/`, `src/driver/`, `src/executor/`,
`src/app.rs`, `src/action.rs`, `src/cli.rs`, `src/main_loop.rs`, `tests/`
**Files read this pass:** 18
**Line-number corrections vs `18-CONTEXT.md`:** `sorted_aliases` `:218`→`:239`;
`recompute_filtered_aliases` `:233`→`:252`; `run.rs` close_input block starts `:597` (not `:605`);
`normal.rs` inline tests start `:717` (not `~730`); `detail.rs` inline tests start `:4674`.
**Pattern extraction date:** 2026-07-29
