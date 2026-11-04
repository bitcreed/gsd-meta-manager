# Quick Task 261103-tn4: Fix editor/interactive child shell-outs competing with TUI input reader for stdin - Context

**Gathered:** 2026-11-03
**Status:** Ready for planning

<domain>
## Task Boundary

$EDITOR / vim launched from inside the TUI is nearly unusable: the user has to press almost
every key twice before vim reacts. Fix it at the root: while an interactive child owns the
terminal, the TUI's own input reader must be fully stopped (not reading stdin/tty at all), and
restored afterwards, with raw mode / alternate screen / mouse capture handled correctly. One
shared helper for every shell-out site. A regression test that would FAIL on the broken code.

</domain>

<decisions>
## Implementation Decisions

(Discussion answered by the orchestrating agent; the human was unavailable. All entries are
INFERRED decisions from code inspection — marked for later audit.)

### Root cause (verified by reading crossterm 0.29 source, `src/event/stream.rs`)
- `src/event.rs::spawn_crossterm_reader` runs a tokio task over `crossterm::event::EventStream`.
  `EventStream::default()` spawns a background std thread that loops
  `poll_internal(None, &EventFilter)` — a blocking poll that READS tty bytes into crossterm's
  global internal event buffer. The task is never stopped around the editor shell-out in
  `src/main.rs` (`app.pending_editor.take()` block, ~line 609), so that thread and vim race for
  every keystroke: each byte crossterm wins is swallowed by the TUI, so vim sees ~every other key.
- Dropping an `EventStream` only sets a shutdown flag and wakes the thread; the thread is not
  joined and there is no public way to observe it has stopped, so "drop the stream before the
  editor" is NOT a deterministic fix. [inferred]

### Fix mechanism [inferred]
- Replace the `EventStream`-based reader with an app-owned reader on a dedicated std thread that
  loops `crossterm::event::poll(short timeout, e.g. 50ms)` + `crossterm::event::read()`, and
  checks a suspend request between polls. A `suspend()` call must BLOCK until the reader thread
  has acknowledged it is parked (not inside poll/read); `resume()` unparks it. Since only that
  thread ever touches crossterm's reader, after the ack nothing in the process reads the tty.
- Keep the same Event → Action mapping (`map_event_to_action`: key Press only, Resize, routed
  mouse events) and the same unbounded mpsc channel into `pump`.
- If the planner finds a cleaner deterministic mechanism, it may use it, but the invariant is
  non-negotiable: when the child starts, no TUI thread is inside a tty read/poll, and none starts
  until the child has exited.
- Optional nicety (discretion): on resume, discard any stale events crossterm buffered before
  suspend that were not yet delivered, if cheap.

### Shared helper / call sites [inferred]
- Inventory found ONE in-TUI interactive shell-out: the `pending_editor` block in `src/main.rs`
  (serves backlog/archive editing, DIRECTION.md edits, one-off `e` SUMMARY and `o` full-diff,
  file browser edits — all route through `app.pending_editor`). `launcher::process::run_interactive`
  is only used by `driver_cli.rs` (CLI path, TUI not running) — out of scope; other `Command`
  uses (git, tmux, gsd-tools, sh in project_creator) are non-interactive / do not own the tty.
  The executor must re-grep to confirm nothing else hands the terminal to a child.
- Still extract the suspend sequence into ONE helper (e.g. `run_with_terminal_suspended(input,
  || child)`): suspend input reader (wait for ack) → remember mouse capture → `tui::restore`
  (capture off, leave alt screen, disable raw mode) → run child → `tui::init` (raw + alt screen)
  → restore mouse capture if it was on → resume input reader → force redraw. The editor block in
  main.rs calls this helper; any future shell-out must use it.
- Order matters: suspend the reader BEFORE leaving raw mode, resume it only AFTER re-entering
  raw mode.

### Regression test [inferred]
- Make the reader loop generic over a small input-source trait (poll/read) so a fake source can
  be injected. Test: start the reader with a fake source that records every poll/read call with
  a timestamp/flag; call the shared shell-out helper with a child closure that sleeps (e.g.
  200ms) and records the call count at start and end; assert ZERO poll/read calls happened while
  the child closure ran, and that polling resumes afterwards (events produced after resume
  arrive on the channel). Also assert `suspend()` returns only after the reader is parked.
- The test must exercise the same helper main.rs uses — a test of an unused helper would pass on
  the broken code. On the old design (reader never told to stop) the "zero reads during child"
  assertion fails, which is the point.
- A pty-based end-to-end test is Claude's discretion (only if it can be done without new heavy
  deps; rustix is already a dependency).

### Claude's Discretion
- Exact poll timeout, type/module names (e.g. `src/event.rs` vs a new `src/input.rs`), whether
  the shell-out helper lives in `tui.rs` or `event.rs`, and terminal-side helper signatures
  (the terminal re-init may need to stay in main.rs if `DefaultTerminal` ownership makes a
  closure awkward — the helper may take `&mut DefaultTerminal`).

</decisions>

<specifics>
## Specific Ideas

- Tests: `cargo test --no-fail-fast` (use `rtk proxy cargo test --no-fail-fast` for real counts),
  `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- Do not change user-visible editor behaviour otherwise (status messages, `App::after_editor`,
  `$VISUAL > $EDITOR > vi`, `editor_command`).

</specifics>

<canonical_refs>
## Canonical References

- crossterm 0.29 `src/event/stream.rs` (EventStream background poll thread) in ~/.cargo/registry
- `src/event.rs`, `src/tui.rs`, `src/main.rs` (`run_tui_loop`, `pending_editor` block)

</canonical_refs>
