# Phase 33: Terminal multiplexer backend seam + herdr support - Pattern Map

**Mapped:** 2026-11-10
**Files analyzed:** 11 (4 new, 6 modified, 1 deleted)
**Analogs found:** 11 / 11 (all analogs are git-tracked source)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/multiplexer/mod.rs` (new) | service (trait + pure core + seam fns) | request-response / transform | `src/terminal_switch.rs` (LaunchPlan, plan_launch) + `src/session_detector.rs:1005-1120` (ProcessProbe trait, cfg swap) | exact (code moves) + role-match (trait) |
| `src/multiplexer/tmux.rs` (new) | service (backend) | request-response (CLI) | `src/terminal_switch.rs:188-316` | exact (verbatim move) |
| `src/multiplexer/herdr.rs` (new) | service (backend) | request-response (CLI + socket NDJSON) | `src/terminal_switch.rs:271-316` (CLI edge + pure parser) + `src/driverctl_cli.rs:204-340` (UnixStream one-line request) | role-match |
| `src/multiplexer/gui.rs` (new) | utility (probe edge) | request-response | `src/terminal_switch.rs:28-43, 127-~175` | exact (verbatim move) |
| `src/terminal_switch.rs` (deleted) | — | — | — | — |
| `src/lib.rs` (mod decl, line 34) | config | — | itself | exact |
| `src/ui/screens/detail.rs` (call sites 2400-2410, 2658-2672, ~4936-5025, 5446-5452, 5483-5530; tests ~23850, 23972, 24093) | controller (key handler) | request-response | itself | exact |
| `src/ui/screens/normal.rs:840-843` | controller | request-response | `detail.rs:5446-5452` (same shape) | exact |
| `src/session_detector.rs:53-54` (doc only) | model | — | itself | exact |
| `tests/spawn_seam_guard.rs:50-70` (allowlist) | test | — | itself | exact |
| ancestry collector (inside `mod.rs`) | utility | file-I/O (/proc) | `src/session_detector.rs:963-969` (`parse_stat_starttime`) + `process_probe()` cfg swap | role-match |

## Pattern Assignments

### `src/multiplexer/mod.rs` (trait, EnvSnapshot, ExecArgv, detect_ordered, LaunchPlan, seam fns)

**Analog A — pure plan over injected probes:** `src/terminal_switch.rs:28-117`. Move `LaunchPlan` / `plan_launch` with the D-01/D-02 rustdoc **verbatim** (lines 45-92). Only change: `try_tmux: bool` → ordered backend list.
```rust
pub struct LaunchPlan {
    /// Try the running tmux server first.
    pub try_tmux: bool,
    /// The GUI terminal to use — either because tmux was not applicable, or
    /// because it was tried and failed.
    pub gui: Option<String>,
}
pub fn plan_launch(probes: &TerminalProbes) -> LaunchPlan {
    let gui = probes.terminal_env.as_ref().filter(|t| !t.is_empty()).cloned()
        .or_else(|| probes.xdg_terminal_exec_ok.then(|| "xdg-terminal-exec".to_string()))
        .or_else(|| probes.x_terminal_emulator_present.then(|| "x-terminal-emulator".to_string()))
        .or_else(|| probes.fallback_present.first().cloned());
    LaunchPlan { try_tmux: probes.in_tmux, gui }
}
```

**Analog B — object-safe trait + "knows-nothing" default + one cfg swap point:** `src/session_detector.rs:1005-1029, 1105-1120`. Copy this shape for `MultiplexerBackend` default `Unsupported` methods and for `collect_ancestry()` (Linux `/proc`, else empty → static order).
```rust
pub trait ProcessProbe {
    fn observe(&self, pid: u32) -> PidObservation;
    fn codex_cwds(&self) -> Option<&[PathBuf]>;
}
#[derive(Debug, Clone, Copy, Default)]
pub struct NoProcessProbe;
...
#[cfg(target_os = "linux")]
pub fn process_probe() -> Box<dyn ProcessProbe> { Box::new(ProcfsProbe::new()) }
#[cfg(not(target_os = "linux"))]
pub fn process_probe() -> Box<dyn ProcessProbe> { ... }
```
Doc convention there: "Facts only: `None`/`Unknown` mean 'could not say', never 'absent'" — reuse for ancestry ("unreadable /proc → stop walk, static order").

**Analog C — ExecArgv refusals:** `src/terminal_switch.rs:188-205`. Move both checks into `ExecArgv::new`, text byte-identical (tests assert substrings `directly` and `option`):
```rust
if program_argv.len() < 2 {
    return Err(format!(
        "refusing to open a tmux window from a {}-element program vector: \
         tmux execs a shell-command given as MULTIPLE arguments directly, \
         but hands a SINGLE argument to a command interpreter — which would \
         put a parser back in a path that deliberately has none",
        program_argv.len()
    ));
}
if program_argv[0].starts_with('-') {
    return Err(format!(
        "refusing to open a tmux window whose program vector begins with \
         {:?}: tmux's own option parser would consume it as one of its \
         options instead of treating it as the program",
        program_argv[0]
    ));
}
```
Carry the preceding rustdoc (lines ~150-187, D-03, T-W0D-01/03/04) onto `ExecArgv`.

**Analog D — ppid parse (last `)` rule):** `src/session_detector.rs:963-969`. ppid is `rest.split_whitespace().nth(1)` (field 4 = index 1 after comm); comm is between first `(` and last `)`, or read `/proc/<pid>/comm`.
```rust
pub(crate) fn parse_stat_starttime(stat: &str) -> Option<u64> {
    let after_comm = stat.rfind(')')?.checked_add(1)?;
    let rest = stat.get(after_comm..)?;
    let field = rest.split_whitespace().nth(19)?;
    field.parse::<u64>().ok()
}
```
Write a sibling `parse_stat_ppid` (pure, unit-tested) rather than `split(' ')`. Cap walk depth (64).

**Seam launch fn (D-10):** collapses the two duplicated blocks in `detail.rs` (see below). Must reproduce exactly these strings: `format!("{backend}: {} — opened in {}", e, term)`, `format!("{backend}: {e}; no terminal emulator found (set $TERMINAL)")`, `"No terminal emulator found (set $TERMINAL)"`, `format!("Failed to launch: {}", e)`; success text is caller-supplied (`"Resumed session {short_id}"` / `"Launched new Claude session"`).

**Seam focus fn (D-09):** no-backend text generalizes `"Tab-switching requires running inside tmux (no $TMUX set)"` (terminal_switch.rs:273).

---

### `src/multiplexer/tmux.rs` (backend, CLI)

**Analog:** `src/terminal_switch.rs:18-26, 188-316, tests 318-686`. Move verbatim:
- `in_tmux` (24-26) → `detect` env gate (`$TMUX` non-empty).
- `open_new_window` body after the refusals (lines ~207-226) → `open(cwd, &ExecArgv)`:
```rust
let output = Command::new("tmux")
    .arg("new-window").arg("-c").arg(cwd).args(program_argv)
    .output()
    .map_err(|e| format!("tmux new-window failed: {e}"))?;
if !output.status.success() {
    return Err(format!("tmux new-window exited {}: {}", output.status,
        String::from_utf8_lossy(&output.stderr).trim()));
}
```
- `strip_dev_prefix` / `pane_tty_matches` / `find_pane_target` (229-266) unchanged; make `pane_tty_matches` `pub(super)` only if herdr reuses it (research says herdr has no tty — likely not needed).
- `switch_to_session` (271-316) → `focus`; "TTY … not found in any tmux pane" maps to `MuxError::NotFound`.
- Tests 348-596 move as-is; the `-d`/one-element refusal test (442-466) retargets `ExecArgv::new`; the `#[ignore]` live test (597+, `Drop` guard killing a `-L` private server) is the pattern for an optional `#[ignore]` herdr smoke test.

---

### `src/multiplexer/herdr.rs` (backend, CLI + socket)

**CLI edge + pure parser analog:** `src/terminal_switch.rs:271-316` (spawn `.output()`, map non-zero exit with stderr, hand stdout to a pure fn). For herdr, stdout is JSON: use `serde` derive with `#[serde(default)]` and tolerant unknown fields (pattern: `src/config.rs:27`; `deny_unknown_fields` is forbidden in `src/` by `tests/spawn_seam_guard.rs`). Pure functions: `parse_pane_list(&str)`, `parse_process_info(&str)`, `join(panes, session) -> Result<Match, NotFound|Ambiguous>`; refusal of ambiguous cwd mirrors `pane_tty_matches` rationale doc (terminal_switch.rs:231-240).

**Socket `layout.apply` analog:** `src/driverctl_cli.rs:204-340` (`connect_within`, `request`, `send_and_read`, `read_reply`). Copy:
- connect on a helper thread with `recv_timeout` (lines 211-234) — `UnixStream::connect` has no timeout;
- encode with `serde_json::to_vec` + push `b'\n'` (lines 247-251);
- `set_write_timeout` then `write_all` (265-271);
- bounded read loop until `\n`, size cap, deadline-derived `set_read_timeout`, `WouldBlock|TimedOut` → timeout error (292-334).
Gate with `#[cfg(unix)]` exactly as driverctl_cli does. Map transport errors → `MuxError::Failed`, `{"error":{"code","message"}}` → `Failed` with message.

---

### `src/multiplexer/gui.rs` (probe edge)

**Analog:** `src/terminal_switch.rs:28-43` (`TerminalProbes`) and `118-~175` (`probe_terminals`, with `which` closure and `xdg-terminal-exec --print-id`). Move verbatim incl. rustdoc; drop `in_tmux` field from `TerminalProbes` if detection supplies the backend list (update the `probes(...)` test helper at 325-342 accordingly).

---

### `src/ui/screens/detail.rs` (call sites)

- `resolve_launch_plan` (2400-2410): repoint doc links + path to `crate::multiplexer`.
- `tmux_resume_argv` / `tmux_launch_argv` (2658-2672): rename backend-neutral, return `ExecArgv` (keep leading `vec!["env".to_string()]`).
- Resume block (~4936-5025) and new-session block (5483-5530): both shaped
```rust
let plan = resolve_launch_plan();
let mut tmux_err: Option<String> = None;
if plan.try_tmux {
    match crate::terminal_switch::open_new_window(&cwd, &tmux_..._argv()) {
        Ok(()) => return ScreenAction::SetStatusMessage(<success>),
        Err(e) => tmux_err = Some(e),
    }
}
match plan.gui { Some(term) => Command::new(&term).args(..).current_dir(..).spawn() ... }
```
→ replace with one seam call; keep `resumable_session_id` gate before it (T-lr9-01). The GUI `Command::new(&term)` spawn may stay in detail.rs (it is already allowlisted) or move into the seam via a GUI-argv builder closure.
- Focus (5446-5452):
```rust
Some(s) => match crate::terminal_switch::switch_to_session(&s) {
    Ok(()) => format!("Switched to {}", self.alias),
    Err(e) => e,
},
```
→ `crate::multiplexer::focus(&s)` (same `Result<_, String>`-to-status shape).
- Tests ~23850/23972 (doc links), 24093 (`use crate::terminal_switch::{plan_launch, TerminalProbes}`) → new paths; do not weaken. Composed-flow tests must not reach the real registry (RESEARCH P3: `cargo test` inside herdr inherits `HERDR_ENV=1`); keep using Codex rows / injected backend list.

### `src/ui/screens/normal.rs:840-843`
Same replacement as detail.rs focus site (`format!("Switched to {}", alias)`).

### `tests/spawn_seam_guard.rs:50-70`
Replace entry
```rust
    // tmux, for switching the user to an existing session. No agent.
    "src/terminal_switch.rs",
```
with one entry per spawning file (`src/multiplexer/tmux.rs`, `herdr.rs`, `gui.rs`; `mod.rs` only if it spawns), each with a one-line "No agent." reason, in the same commit as the move (guard fails both on unexpected and vanished entries, lines ~220-233).

### `src/lib.rs:34`, `src/session_detector.rs:53-54`
`pub mod terminal_switch;` → `pub mod multiplexer;`; update `ClaudeSession` doc reference.

## Shared Patterns

### Pure core / thin impure edge
**Source:** `src/terminal_switch.rs:113-127` ("The thin impure edge: it holds no ordering of its own"). Apply to: detection (`EnvSnapshot` + ancestry fixture → pure `order_innermost`), herdr JSON join, launch plan. Fail-closed refusals happen before any `Command` is built (test at terminal_switch.rs:440-466).

### Error surfacing
Status line receives `Result<_, String>` and renders `Err(e)` directly (`detail.rs:5447-5450`, `normal.rs:840-843`). `MuxError` should `Display` into those strings; tmux strings byte-identical.

### Exact-match join rationale docs
**Source:** `src/terminal_switch.rs:231-247` (T-LR8-01 wording). Apply to herdr session-id/pid/unique-cwd join.

### Long "why" rustdoc with decision/threat IDs
Keep D-01/D-02/D-03, T-W0D-01/03/04/06 text when moving; design-note table (D-14) goes in `mod.rs` module doc — content from RESEARCH.md § Design-note table.

## No Analog Found

| File/part | Role | Data Flow | Reason |
|---|---|---|---|
| nearest-first ancestry walk (`/proc/<pid>/stat` ppid chain) | utility | file-I/O | No existing ppid walker; build on `parse_stat_starttime`'s last-`)` rule |
| herdr `layout.apply` request body | — | — | Use RESEARCH.md § "herdr open via one socket request" schema |

## Metadata
**Analog search scope:** `src/terminal_switch.rs`, `src/ui/screens/{detail,normal}.rs`, `src/session_detector.rs`, `src/driverctl_cli.rs`, `src/launcher/bridge.rs`, `src/config.rs`, `tests/spawn_seam_guard.rs`
**Files scanned:** 8
**Pattern extraction date:** 2026-11-10
