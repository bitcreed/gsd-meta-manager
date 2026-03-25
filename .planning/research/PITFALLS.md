# Pitfalls Research

**Domain:** TUI multi-project management dashboard (Rust/ratatui or Python/Textual)
**Researched:** 2026-03-24
**Confidence:** HIGH (ratatui pitfalls), MEDIUM (Textual pitfalls), HIGH (file-watching pitfalls)

---

## Critical Pitfalls

### Pitfall 1: Terminal Not Restored on Panic

**What goes wrong:**
The application crashes (panic in Rust, unhandled exception in Python) while in raw mode and on the alternate screen. The terminal is left in a broken state — no echo, garbled output, cursor missing. The user's shell is unusable until they run `reset` or open a new terminal.

**Why it happens:**
Raw mode and alternate screen must be explicitly restored. Developers write the happy path but forget that panics and signals bypass normal cleanup. In Rust, `Drop` implementations are not called during `abort` or certain signals. In Python, `atexit` handlers may not run under SIGKILL.

**How to avoid:**
- **Rust/ratatui:** Use `ratatui::init()` and `ratatui::restore()` (available since 0.28.1) — these automatically install panic hooks. If on an older version, install a custom panic hook that calls `restore()` before the original hook runs. Wrap the main loop in a closure that restores on error.
- **Python/Textual:** Textual handles this internally for normal exits, but test explicitly that `Ctrl+C` and unhandled exceptions leave the terminal clean. Use `try/finally` blocks around the `app.run()` call.

**Warning signs:**
- Terminal looks corrupt after killing the app during development
- Tests kill the process mid-run and leave CI shells broken
- The app handles `Ctrl+C` but does not handle `SIGTERM`

**Phase to address:** Foundation / Setup phase — install before writing any application logic.

---

### Pitfall 2: Blocking the Event Loop with File I/O

**What goes wrong:**
Reading `.planning/` files for N projects happens synchronously in the render or event-handling thread. With even 10 projects, a slow disk, NFS mount, or unresponsive filesystem causes the UI to freeze noticeably. The TUI stops responding to keypresses while reads are in-flight.

**Why it happens:**
State reads feel "fast" on a local SSD during development. The problem surfaces on spinning disks, network shares, Docker volumes, or when one `.planning/` directory is inside a Git repository being indexed by an IDE. File reads are never truly instant and always unbounded in latency.

**How to avoid:**
- **Rust:** Run all file reads on a dedicated Tokio task or `rayon` thread pool. Send results back via an `mpsc` channel to the main event loop. Never call `std::fs::read` or `serde_json::from_reader` inside the draw closure or the input handler.
- **Python/Textual:** Use `@work(thread=True)` for all file system operations. Never `await` a file read on the main async loop — Python's asyncio file I/O is still blocking unless you use `aiofiles` or a thread worker.
- **Both:** Cache parsed state in memory. File reads should only refresh the cache, not block widget rendering.

**Warning signs:**
- Keypresses feel laggy when many projects are registered
- Adding a project on a slow path causes a visible stutter
- The draw function or event handler calls any filesystem API directly

**Phase to address:** State reading / architecture phase — establish the async file-read pattern before connecting it to any UI widget.

---

### Pitfall 3: File Watcher Event Storms and Debounce Neglect

**What goes wrong:**
A file watcher (`notify` in Rust, `watchfiles` in Python) is set up on `.planning/` directories. When GSD writes state (multiple files written in rapid succession), the watcher fires 5–20 events per save. Each event triggers a full state reload. CPU spikes, the UI redraws constantly, and the application may deadlock or drop events if the watcher's event queue overflows.

**Why it happens:**
Editors and tools write files in stages (truncate, write, fsync, rename). Each stage generates an inotify event. Without debouncing, every intermediate write triggers a reload. The `notify` crate's raw watcher has no built-in debounce; developers assume one save = one event.

**How to avoid:**
- **Rust:** Use `notify-debouncer-full` or `notify-debouncer-mini` instead of the raw `notify` watcher. Set a debounce window of 100–500ms. Handle the `EventKind::Any` overflow case gracefully by scheduling a full refresh rather than crashing.
- **Python:** Use `watchfiles` with `awatch`, which batches events within a cycle. Apply an explicit debounce delay (100–300ms) before acting on changes.
- **Both:** Treat the file watcher as a "cache invalidation hint" only. The watcher says "something changed" — a separate read task does the actual file I/O. Never do file I/O inside the watcher callback.
- inotify watch limits default to 8192 on Linux. With many projects, each containing multiple files, you can hit this limit. Check `/proc/sys/fs/inotify/max_user_watches` and document the limit in user-facing error messages.

**Warning signs:**
- CPU is elevated even when no projects are actively running
- A single GSD write triggers many rapid redraws
- Application slows down as more projects are registered

**Phase to address:** File-watching / state sync phase.

---

### Pitfall 4: Rendering Everything on Every Tick (The Spinning Loop)

**What goes wrong:**
The app renders at a fixed tick rate (e.g., 60 FPS or every 16ms) regardless of whether anything changed. With 10+ projects displayed, the render function walks the project list, formats strings, and pushes buffer diffs on every frame. CPU usage is 10–40% idle — comparable to a video game. Users and CI pipelines notice immediately.

**Why it happens:**
Early ratatui examples and templates default to a "tick + draw on every iteration" pattern because it is simple. It works fine for a clock widget. It does not work for a dashboard with stable state.

**How to avoid:**
- **Rust/ratatui:** Decouple tick rate from render rate. Use `tokio::select!` to wait on either a state-change event, a user input event, or a low-frequency render tick (200–500ms for stable dashboards). Only call `terminal.draw()` when state has actually changed or on the slow background tick for clock updates. Never call `terminal.draw()` more than once per event-loop iteration (ratatui's double-buffer assumption).
- **Python/Textual:** Textual's reactive system handles this well when used correctly — only `refresh()` widgets when their data changes. Avoid polling loops that call `refresh()` unconditionally.
- **Both:** Target 10–30 FPS for a management dashboard, not 60. This is not a game.

**Warning signs:**
- `htop` shows the process consuming >5% CPU while idle
- The render function is called even when the user is not interacting
- `terminal.draw()` is called outside of event handling (e.g., in a background tick unconditionally)

**Phase to address:** Core event loop architecture — establish before any widgets are built.

---

### Pitfall 5: Immediate-Mode Rendering Misapplied (God App Struct)

**What goes wrong:**
All project state, UI state, input state, selected indices, and modal flags get merged into one `App` struct. As the application grows, every render function receives `&mut App` or `&App`, and every event handler needs mutable access to the same struct. Borrow checker errors proliferate in Rust. The struct becomes untestable.

**Why it happens:**
Ratatui's immediate-mode model makes it tempting to put "everything the render function needs" into one place. Early tutorials show a minimal `App` struct, and developers grow it organically until it has 30 fields.

**How to avoid:**
- Adopt The Elm Architecture (TEA) from the start: separate `Model` (pure data), `View` (render functions, no side effects), and `Update` (message-based state transitions). Ratatui's official documentation endorses TEA explicitly.
- Keep UI state (selected row, scroll offset, focused panel) separate from domain state (project list, file-read results).
- In Rust: use `Arc<RwLock<ProjectState>>` for the shared project data, passed to background tasks. UI-only state lives in the render context, not the shared state.
- In Python/Textual: use `reactive` attributes scoped to individual widgets, not one mega-reactive on the App class.

**Warning signs:**
- The `App` struct has more than ~15 fields
- Render functions take `&mut App` instead of read-only data
- Adding a new view requires touching the central state struct

**Phase to address:** Architecture phase — before writing the first widget.

---

### Pitfall 6: Unicode Width and Emoji Breaking Layout

**What goes wrong:**
Status indicators, project names, or phase labels containing emoji or East Asian characters cause columns to misalign. A "✓" or "🔄" symbol may render as 1 column wide in code but 2 columns wide in the terminal. Carefully laid-out tables become unreadable garbage.

**Why it happens:**
`unicode-width` (used by both ratatui and Textual internally) applies UAX#11 "narrow/wide" classification, but multi-codepoint emoji sequences (ZWJ sequences, variation selectors) are not handled consistently across terminal emulators. What looks correct in Alacritty may break in tmux or VS Code's integrated terminal.

**How to avoid:**
- Stick to Unicode 9.0 single-codepoint emoji (e.g., ✓ U+2713, ✗ U+2717) for status indicators. Avoid newer multi-codepoint emoji.
- Test in at least two terminal emulators: one modern (Alacritty, Kitty) and one common (gnome-terminal, tmux).
- Provide ASCII-only fallbacks for all status symbols as a configuration option.
- Audit the GSD `STATE.md` and `ROADMAP.md` formats for any emoji — the parser must handle them safely even if the TUI does not display them.

**Warning signs:**
- Columns misalign when running inside tmux vs. a native terminal
- Status icons look correct locally but break in CI output
- Any use of emoji in row labels or status cells

**Phase to address:** Display / rendering phase — as soon as tabular project lists are implemented.

---

### Pitfall 7: Reading Partially-Written State Files

**What goes wrong:**
GSD writes to `.planning/STATE.md` or `config.json` while the dashboard is reading them. The dashboard reads a truncated or mid-write file, fails to parse it, and either crashes or caches corrupted state that persists until the next successful read.

**Why it happens:**
GSD (or any editor/tool) does not write files atomically by default. A write operation involves truncating the file, then writing new content. If the reader observes the file between truncation and completion, it sees partial data.

**How to avoid:**
- Treat parse failures as "transient, retry shortly" rather than "fatal error." Log the failure, keep the last good state in cache, and schedule a re-read after 1–2 seconds.
- Implement exponential backoff for repeated parse failures on the same file.
- Never propagate a parse error to the UI as "project broken" — distinguish "temporarily unreadable" from "persistently malformed."
- If GSD ever writes these files itself: write to a `.planning/STATE.md.tmp` file and `rename()` atomically.

**Warning signs:**
- Occasional "invalid JSON/TOML" errors in logs that resolve on their own
- Project status briefly shows as "unknown" then recovers
- The parser panics or crashes rather than returning `Result::Err`

**Phase to address:** State reading / parsing phase.

---

### Pitfall 8: Textual Reactive Watcher Called Before Widget is Mounted

**What goes wrong:**
(Python/Textual only) A `reactive` attribute is assigned in `__init__` or the class body. Its watcher method (`watch_*`) tries to query child widgets via `self.query_one()`. The widget is not yet mounted, so `query_one` raises `NoMatches` and the app crashes on startup.

**Why it happens:**
Textual's reactive system fires watchers immediately when a reactive is set, even before the DOM is ready. Developers assume widget initialization order matches Python's usual `__init__` flow.

**How to avoid:**
- Use `self.set_reactive(ClassName.attr, value)` in `__init__` to set initial values without triggering watchers.
- Keep watchers defensive: check `if not self.is_mounted: return` before querying child widgets.
- Prefer setting reactives in `on_mount` rather than `__init__` when the watcher touches the DOM.

**Warning signs:**
- App crashes with `NoMatches` on startup before any user interaction
- Watcher methods reference `self.query_one()` or `self.query()`
- Reactive attributes are initialized in the class body with data-dependent initial values

**Phase to address:** Widget development phase — as each widget is built.

---

### Pitfall 9: Calling UI Methods Directly from Thread Workers (Textual)

**What goes wrong:**
(Python/Textual only) A `@work(thread=True)` worker reads files and directly calls `self.some_widget.update()` or sets a reactive from the worker thread. Textual's asyncio event loop is not thread-safe. The result is intermittent UI corruption, silent dropped updates, or crashes that are difficult to reproduce.

**Why it happens:**
`@work(thread=True)` runs in a real OS thread. It looks like a normal Python function. The worker can "see" the widget objects. Developers naturally call methods on them directly without realizing asyncio objects cannot be touched from outside the event loop thread.

**How to avoid:**
- Use `self.app.call_from_thread(fn, *args)` for any UI update from a thread worker. This schedules `fn` to run on the event loop thread.
- Alternatively, use `self.post_message(MyMessage(data))` from the worker — messages are thread-safe and handled on the event loop.
- Prefer async workers (`@work` without `thread=True`) for file I/O if using `aiofiles`, reserving thread workers only for truly blocking APIs.

**Warning signs:**
- Thread workers contain `self.query_one(...)` or directly set reactive attributes
- UI glitches that are intermittent and timing-dependent
- No `call_from_thread` or `post_message` in any worker that touches UI state

**Phase to address:** Background task / worker phase.

---

### Pitfall 10: No Config for Project Registry — Storing Paths in Application State Only

**What goes wrong:**
The list of registered project directories is kept only in memory or in a runtime state file that is not persisted between sessions. Restarting the TUI requires re-registering all projects. Alternatively, the registry is written to a file without atomic writes or schema versioning, so a single crash during a write corrupts the registry.

**Why it happens:**
Storing a path list seems trivial. Developers serialize it with a quick `serde_json::to_writer` or `json.dump()` directly to the file without considering concurrent writes, partial writes, or future schema changes.

**How to avoid:**
- Store the project registry in a dedicated file (e.g., `~/.config/gsd-manager/registry.toml`).
- Always write via atomic rename: write to `.registry.toml.tmp`, then `rename()` to `.registry.toml`.
- Include a schema version field from day one. Even version `1` allows future migrations.
- Validate the registry on load and surface actionable errors: "Project at `/path/foo` no longer has a `.planning/` directory — remove it from registry?"

**Warning signs:**
- Registry file is written with a direct `write()` call, not via temp-file-then-rename
- No version field in the registry format
- Removing an entry while the app is running could corrupt the file if the app crashes mid-write

**Phase to address:** Project registration / persistence phase.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Synchronous file reads in render loop | Simpler code, no channels | UI freezes as project count grows | Never — async from the start costs little extra |
| One monolithic `App` struct for all state | Faster initial build | Untestable, borrow-checker pain, hard to split into modules | MVP only if the struct is explicitly planned for decomposition |
| Fixed tick rate of 60 FPS | Simplest event loop | High idle CPU, visible in htop | Never for a management dashboard — use event-driven rendering |
| Skipping panic hook setup | 5 minutes saved | Corrupt terminal on any panic during development | Never — 5 minutes to set up, infinite annoyance if skipped |
| Polling files on a timer instead of using a file watcher | Simpler initial implementation | Misses rapid changes, wastes I/O on quiet periods | Acceptable for MVP; replace before v1 release |
| No debounce on file watcher | Simpler watcher setup | CPU spike on every GSD write, event queue overflow | Never — debounce is one line with the right crate |
| Direct reactive assignment in `__init__` (Textual) | Feels natural | Watcher fires before widget mounted, crashes on startup | Never — use `set_reactive()` for initialization |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| `.planning/STATE.md` parsing | Crash on malformed file | Return `Result::Err` / `None`, keep last good state, retry after delay |
| `.planning/ROADMAP.md` parsing | Assume stable format across GSD versions | Version-check or use lenient parsing; log unparseable fields, don't crash |
| `config.json` in project directories | Read once at startup | Watch for changes; GSD may update it during phase transitions |
| `notify` file watcher | Use raw watcher, assume one event per save | Use `notify-debouncer-full`; assume burst events per save |
| Atomic registry writes | Write directly to target file | Write to `.tmp`, then `fs::rename()` — atomic on POSIX |
| Terminal resize events | Ignore `SIGWINCH` / resize events | Handle `Event::Resize` from crossterm; re-layout on terminal resize |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Rendering every event including mouse moves | CPU 20-40% at idle while user moves mouse | Gate renders on state-change events only; use a render-interval channel | Immediately noticeable on any project with >5 widgets |
| Re-reading all project files on any file change | I/O burst when GSD writes state | Debounce + only re-read the changed project's files | Visible with >5 projects on spinning disk |
| Parsing Markdown on every render frame | High CPU even with event-driven rendering | Parse once, cache `ProjectState` structs, only re-parse on cache invalidation | With >3 projects and complex ROADMAP.md files |
| inotify watch limit exceeded | Watcher silently fails to track new projects | Limit watches to key files per project (STATE.md, config.json), not entire trees | Linux default limit: 8192 watches, easily hit with 100+ projects |
| String allocation in render hot path | Memory churn, GC pressure (Python) | Pre-format status strings when state changes, not per-frame | Mostly a Python concern; Rust allocator handles this better |

---

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| Executing shell commands derived from project paths stored in registry | Path traversal, command injection if paths are user-supplied | Never execute paths directly; validate all registered paths are real directories with `.planning/` |
| Storing registry world-readable if it contains sensitive project names | Information disclosure | Use `0600` permissions on the registry file |
| Parsing untrusted `.planning/` files with `eval` or unsafe deserialization | Code execution if a malicious project is registered | Use strict parsers (serde with `deny_unknown_fields`, Python's `json` module, not `eval`) |

---

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Showing "unknown" state with no explanation when a project file is unreadable | User can't tell if the project is broken or if the tool is broken | Show "unreadable (retrying...)" with the last-known state and a timestamp |
| No visual indication that background file reads are in progress | User thinks the dashboard is frozen or showing stale data | Show a subtle spinner or "last updated: Xs ago" timestamp per project |
| Crashing when a registered project directory is deleted | Loss of all other visible state | Detect missing directories on startup and on watch events; surface as a warning, not a crash |
| Keyboard shortcuts not discoverable | Power users use the keyboard, but cannot discover bindings | Persistent footer with current context shortcuts; `?` opens help overlay |
| Terminal size too small for dashboard layout | Layout breaks, widgets overflow each other | Detect minimum required size; show a clear "terminal too small" message rather than corrupt layout |
| Phase names truncated without indicator | User cannot tell if a phase name is complete | Truncate with ellipsis `…` (U+2026); never silently clip text |

---

## "Looks Done But Isn't" Checklist

- [ ] **Terminal cleanup:** Verify the terminal is restored after `Ctrl+C`, `SIGTERM`, and a deliberate panic — not just on normal exit
- [ ] **File watcher edge cases:** Verify behavior when a registered project directory is deleted, renamed, or made unreadable while the app is running
- [ ] **Parse resilience:** Verify the app continues running and shows last-known state when a `.planning/` file contains invalid content
- [ ] **Resize handling:** Verify the layout redraws correctly after resizing the terminal window, including minimum-size protection
- [ ] **Registry persistence:** Verify the registry survives a crash mid-write (test by killing with SIGKILL during a registration write)
- [ ] **Unicode in project names/paths:** Verify project paths and names with spaces, Unicode, and special characters render and parse correctly
- [ ] **High project count:** Verify performance and layout with 20+ registered projects, not just 2–3
- [ ] **inotify limit:** Verify an informative error is shown (not a silent failure) when the inotify watch limit is approached

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Terminal left in raw mode | LOW | User runs `reset` in shell; no data loss |
| God App struct requiring decomposition | HIGH | Refactor into TEA model; borrow-checker issues make this painful mid-project |
| Corrupt project registry | LOW | Delete registry file; re-register projects manually |
| Persistent stale state in cache | LOW | Implement a "force refresh" keybinding (e.g., `r`) to invalidate all caches |
| inotify watch limit hit | MEDIUM | Reduce per-project watches to key files only; document Linux configuration change |
| Textual reactive mount crash | LOW | Fix by switching `__init__` assignment to `set_reactive()` or moving to `on_mount` |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Terminal not restored on panic | Phase 1: Foundation | Kill app with SIGKILL; verify terminal is usable |
| Blocking event loop with file I/O | Phase 1: Architecture | Profile with >10 projects; confirm 0 UI freezes during file reads |
| File watcher event storms | Phase 2: State sync | Simulate rapid GSD writes; confirm CPU stays <5% idle |
| Spinning render loop CPU waste | Phase 1: Event loop | Measure idle CPU; must be <2% with no activity |
| God App struct | Phase 1: Architecture | Review struct field count; enforce TEA boundaries in code review |
| Unicode width layout breakage | Phase 2: Display | Test in tmux and two terminal emulators; check all status symbols |
| Partially-written file reads | Phase 2: State parsing | Inject truncated test files; confirm graceful degradation |
| Textual reactive mount crash | Phase 2: Widget build | Verify app startup is clean with all reactive attributes set |
| Thread worker UI mutation (Textual) | Phase 2: Background tasks | Code review: no direct widget access from thread workers |
| Registry corruption on write | Phase 3: Persistence | Test SIGKILL during registration; verify registry integrity after |

---

## Sources

- [Ratatui FAQ](https://ratatui.rs/faq/) — event handling on Windows, double-draw prohibition, async complexity warning
- [Ratatui Rendering Concepts](https://ratatui.rs/concepts/rendering/) — immediate mode pitfalls, programmer responsibility for render triggers
- [Ratatui Panic Hooks](https://ratatui.rs/recipes/apps/panic-hooks/) — terminal cleanup patterns, backend-specific requirements
- [Ratatui Async Counter App Tutorial](https://ratatui.rs/tutorials/counter-async-app/) — blocking vs. event-driven event loop patterns
- [Ratatui GitHub Issue #1338](https://github.com/ratatui/ratatui/issues/1338) — confirmed high CPU from unconditional draw at 60 FPS
- [Ratatui GitHub Discussion #220](https://github.com/ratatui/ratatui/discussions/220) — best practices for app architecture
- [Ratatui The Elm Architecture](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/) — recommended state architecture
- [notify-rs GitHub](https://github.com/notify-rs/notify) — file watcher behavior and debounce requirements
- [notify-debouncer-mini docs](https://docs.rs/notify-debouncer-mini/latest/notify_debouncer_mini/) — debounce implementation
- [inotify(7) Linux man page](https://man7.org/linux/man-pages/man7/inotify.7) — watch limits, non-recursive monitoring, event queue overflow
- [Textual Workers Guide](https://textual.textualize.io/guide/workers/) — `call_from_thread` requirement, thread vs. async workers
- [Textual Reactivity Guide](https://textual.textualize.io/guide/reactivity/) — watcher mount timing issue, `set_reactive()` fix, `recompose` state reset
- [7 Things Learned Building a Modern TUI Framework](https://www.textualize.io/blog/7-things-ive-learned-building-a-modern-tui-framework/) — unicode width, emoji unpredictability, floating-point layout rounding
- [Claude Code Issue #15608](https://github.com/anthropics/claude-code/issues/15608) — config file corruption from concurrent process writes
- [Ratatui Unicode Width Issue #1271](https://github.com/ratatui/ratatui/issues/1271) — confirmed unicode width calculation bugs
- [Ratatui Buffer Unicode/Emoji Discussion #1438](https://github.com/ratatui/ratatui/discussions/1438) — terminal inconsistency with emoji rendering

---
*Pitfalls research for: TUI multi-project management dashboard (gsd-manager)*
*Researched: 2026-03-24*
