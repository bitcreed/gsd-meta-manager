---
phase: 37-inbox-screen-and-actions
reviewed: 2026-11-15T00:00:00Z
depth: standard
files_reviewed: 34
files_reviewed_list:
  - README.md
  - src/action.rs
  - src/agent_skill.md
  - src/app.rs
  - src/clipboard.rs
  - src/inbox.rs
  - src/launcher/process.rs
  - src/lib.rs
  - src/main.rs
  - src/multiplexer/mod.rs
  - src/state_reader/owner_items/context.rs
  - src/state_reader/owner_items/gate.rs
  - src/state_reader/owner_items/mod.rs
  - src/state_reader/owner_items/rules.rs
  - src/state_reader/queue_md.rs
  - src/ui/screens/ack_confirm.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/help.rs
  - src/ui/screens/inbox.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/owner_actions.rs
  - src/ui/screens/owner_block.rs
  - src/ui/screens/owner_file_view.rs
  - src/ui/screens/owner.rs
  - src/ui/screens/render_escape_guard.rs
  - tests/focus_entry_guard.rs
  - tests/inbox_cli.rs
  - tests/owner_acknowledge_roundtrip.rs
  - tests/owner_digest.rs
  - tests/owner_open_conformance.rs
  - tests/spawn_seam_guard.rs
findings:
  critical: 0
  warning: 6
  info: 6
  total: 12
  necessity: 0
status: issues_found
---

# Phase 37: Code Review Report

**Reviewed:** 2026-11-15
**Depth:** standard
**Files Reviewed:** 34 (fixtures under `tests/fixtures/owner/context-*` and `owner_golden` skimmed; no private real-project text found)

## Summary

The security-critical paths are mostly sound. The `v` argv carries the command as one element behind `env claude`, and the phase token and workstream name are re-validated at launch. The `A` argv is built one element per flag and value, with `is_flag_value` guarding most values. The acknowledge child is bounded (10 s, stdin null, stderr to a file) and has the workstream and session selectors removed from its environment. OSC 52 carries base64 only. Clipboard tool argv are constants and the text goes on stdin. The clipboard text and every status line go through `terminal_capped`, `shown` or `render_for_terminal`. The file viewer canonicalizes and contains the target inside `.planning/`. The unicode width math I traced (`blanks(width - ...)`, `rule_w`, `scroll_for`, `grid_for_inbox`) is guarded against underflow, and I found no render-path slicing panic.

The defects are mostly behavioral:
- The side-effecting action keys ignore key modifiers.
- Spawn failures are reported as timeouts.
- The cursor re-anchor contract is broken at the fold line, and the test that should catch it reads the cursor through a test-only function.
- The `--ws` value skips the flag-shape guard that every other value gets.
- The acknowledge launcher is resolved project-local first.

No Critical issues. `necessity-review.md` was read; I found no necessity findings. The re-validations in `run_plan` and `ack_target_for` sit on a trust boundary (digest data) and are defense in depth, so they are not necessity findings.

## Warnings

### WR-01: Action keys `v`, `o`, `A`, `y` and `g` ignore key modifiers; Ctrl-V, key repeat or an unbracketed paste launches sessions with no confirm

**File:** `src/ui/screens/inbox.rs:1697-1716` (also `src/ui/screens/detail.rs:2243-2246`, `src/app.rs:3906-3917`)
**Issue:** `InboxScreen::handle_key` discards `_modifiers`, and so does the Owner block key handler. `App::handle_key` filters only Ctrl-C. A terminal in raw mode delivers Ctrl-V (0x16, the common paste reflex) as `Char('v')` with CONTROL, so it runs `v`. Ctrl-O, Ctrl-Y and Ctrl-G likewise trigger `o`, `y` and `g`. `v` is immediate: no confirmation and no guard against repeats. `run_key` creates a new multiplexer tab or GUI terminal on every press.

- Holding `v` (key auto-repeat) opens one session per repeat. The live-session rule only helps after the periodic session poll notices the first one.
- `grep -rn BracketedPaste src` finds nothing, so a pasted string is replayed as individual key events. Every `v` in pasted text launches a session. A pasted `A...y` opens the acknowledge popup and confirms it, because `ConfirmPopup::on_key` treats `y` as an immediate Yes (`confirm_popup.rs:122`).

**Fix:** In the inbox and Owner block key arms, act on `v`, `o`, `A`, `y` and `g` only when `!modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)`. SHIFT must stay allowed because `A` arrives shifted. Also enable bracketed paste (crossterm `EnableBracketedPaste` plus `Event::Paste` swallowed in `main.rs`). For `v`, add a short per-`(alias, group)` cooldown, for example ignoring a repeat within 1 s and saying so in the status line.

```rust
KeyCode::Char('v') if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
    self.run_key(ctx, &mut owner_actions::launch_real)
}
```

### WR-02: A failed spawn, an unwritable temp file or an unreadable stderr sink is reported as "Acknowledge timed out after 10 s"

**File:** `src/ui/screens/owner_actions.rs:816-823` (cause in `src/launcher/process.rs:471-496`)
**Issue:** `run_bounded_with_stderr_env` returns `None` for several reasons: `tempfile()` failed, `spawn()` failed, the budget expired, stdout did not reach EOF, or the stderr sink could not be read back. `run_ack` maps every `None` to `"Acknowledge timed out after 10 s: nothing is confirmed, check git status in {alias}"`.

- A project folder that was moved or deleted, so the cwd is invalid, produces this.
- `node` or `gsd-tools` that became non-executable between the `on_path` check and the spawn produces this.
- `ENOMEM`, `EMFILE` or a full `/tmp` produce this.

The owner is told a timeout happened and is sent to check git status for an operation that never started. A child that did finish but whose stderr read failed also reads as a timeout, hiding a success or a gsd-core refusal.

**Fix:** Distinguish the reasons. Spawn the child so an `io::Error` can be surfaced, or have the bounded runner return a small enum (`Spawn(io::ErrorKind) | TimedOut | Done(..)`). Report `Acknowledge failed: could not start gsd-tools ({kind})` for spawn errors and keep the timeout text for the real timeout. Take the `10 s` in the message from `ACK_BUDGET` rather than hard-coding it.

### WR-03: Cursor re-anchor lands on the fold line instead of the previous row when the last row vanishes (and the test cannot see it)

**File:** `src/ui/screens/inbox.rs:387-402` (`cursor_pos`), `:467-480` (`reanchor`), test helper `:300-310`
**Issue:** The `reanchor` doc and the D-10 test name promise "the next row, else the previous one, else the fold line, else nothing". `cursor_pos` clamps the stale index against `count = rows.len() + has_fold`. When the cursor row was the last row and a fold line exists, the clamped index equals `rows.len()`. That is the fold line, so the cursor jumps to the fold line and skips the previous row.

`selection_falls_to_the_next_row_then_previous_then_none` reads the result through the `#[cfg(test)]`-only `resolve_cursor`, which clamps to `rows.len() - 1` and knows nothing of the fold line. The assertion therefore checks a function production never calls. A fleet fixture with a fold would fail it.

**Fix:** In `cursor_pos`, clamp a vanished row's fallback index to `rows.len().saturating_sub(1)` when `rows` is non-empty, and reach the fold line only when no row remains. Make the test read the cursor through `cursor_pos`, and delete the test-only `resolve_cursor`.

```rust
Some(InboxStop::Row(key)) => rows.iter().position(|r| r.key() == *key).unwrap_or_else(|| {
    if rows.is_empty() { count - 1 } else { view.cursor_index.min(rows.len() - 1) }
}),
```

### WR-04: `--ws` value skips the flag-shape guard that every other acknowledge argument gets

**File:** `src/ui/screens/owner_actions.rs:328-332` and `:416-418`
**Issue:** `ack_target_for` checks milestone, file name, label and slug with `is_flag_value`, which refuses a leading `-`. The workstream name is checked only with `is_workstream_name`, whose alphabet (`[A-Za-z0-9._-]+`) admits `-x`, `--raw`, `.` and `..`. `ack_args` then emits `--ws` followed by that value as a separate element. A workstream directory named `--category` or `-h` (any registered project can contain one) becomes a flag-shaped token in gsd-tools' argv (CWE-88, the exact class the `is_flag_value` doc cites). `.` and `..` are accepted as workstream names too.

**Fix:** Use the same guard, and also refuse dot-only names.

```rust
if !is_workstream_name(ws) || !is_flag_value(ws) || ws.chars().all(|c| c == '.') {
    return Err(refused("workstream name", ws));
}
```

Apply the dot-only refusal to `is_workstream_name` itself so `v` (`--ws {name}` inside the slash command) gets it too.

### WR-05: The acknowledge launcher is resolved project-local first, so a registered project can supply the code that runs after Yes

**File:** `src/ui/screens/owner_actions.rs:681-687` (resolver `src/state_reader/queue_md.rs:113-116`, candidates `:88-111`)
**Issue:** `resolve_launcher(&project.path)` uses `resolve_gsd_tools`, whose first candidates are `<project>/gsd-core/bin/gsd-tools.cjs`, `<project>/.claude/gsd-core/bin/...` and `<project>/.codex/gsd-core/bin/...`. `A` then runs `node <that file> ...` with the TUI's full environment (tokens, API keys). A cloned repository that ships `.claude/gsd-core/bin/gsd-tools.cjs` gets arbitrary code executed on the owner's machine once the owner presses `A` and `y`. The popup shows the path, but it is one long DIM line among others, and the owner has no reason to distrust a path ending in `gsd-tools.cjs`. The project-local order mirrors GSD, but this is the first place in the product where the resolved launcher is run as a write path after an owner action.

**Fix:** For the acknowledge writer, prefer the user-level installs (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `PATH`) and use a project-local shim only if no user-level one exists. In that case say so in the popup (`Runs the project's own gsd-tools.cjs`) and draw that line in Yellow or Red. Also consider `env_clear` plus an allowlist for the child. Add a test with a hostile `.claude/gsd-core/bin/gsd-tools.cjs` next to a user-level one.

### WR-06: The file viewer silently drops everything past line 65,535 and reads past its size cap on a swapped file

**File:** `src/ui/screens/owner_file_view.rs:146` (`u16::try_from(self.lines.len()).unwrap_or(u16::MAX)`), `src/ui/screens/owner_actions.rs:164-168`
**Issue:**
1. `MAX_VIEW_BYTES` allows 1 MiB, which is more than 65,535 short lines. The scroll offset and `Paragraph::scroll` are `u16`, so lines beyond 65,535 can never be reached. The viewer shows no notice, so a long checklist looks complete while hiding its tail.
2. `read_owner_file` checks `meta.len() <= MAX_VIEW_BYTES` and then `std::fs::read`s the path again. A file swapped (or a log growing) between the `metadata` and `read` calls is read in full, which defeats the T-37-14 cap. The canonicalized target is also re-resolved by path rather than read from an opened handle.

**Fix:** Open once and bound the read: `File::open(&target)?.take(MAX_VIEW_BYTES + 1).read_to_end(..)` and refuse if more than `MAX_VIEW_BYTES` came back. In the viewer, cap the line count at `u16::MAX` explicitly and append a DIM `… N more lines not shown` line.

## Info

### IN-01: `owner_ack_busy` is keyed by file basename, so same-named files in different workstreams or milestones block each other

**File:** `src/ui/screens/mod.rs:1912-1940`, `src/ui/screens/owner_actions.rs:714-720`, `src/app.rs` (`OwnerAcknowledged` arm)
**Issue:** The busy set holds `(alias, target.name())`, the basename. Workstreams restart phase numbering, so `alpha/.../01-UAT.md` and `beta/.../01-UAT.md` (or a current and an archived `03-UAT.md`) share a basename. A concurrent `A` on the second file is refused with "Already acknowledging 01-UAT.md" for up to 10 s. The success line `Acknowledged 03-UAT.md via gsd-core (uncommitted)` is likewise ambiguous across workstreams.
**Fix:** Key the busy set and the answer on `target.rel_file` (already carried), and show the workstream or label in the status line when present.

### IN-02: `v` does not scrub `GSD_WORKSTREAM` and the session-pointer variables from the launched session

**File:** `src/ui/screens/owner_actions.rs:118-131`
**Issue:** The acknowledge child removes `ACK_ENV_REMOVE` so an inherited `GSD_WORKSTREAM` cannot redirect a root-scoped write (T-37-18). The `v` launch of a root-scoped `/gsd-verify-work N` into a GUI terminal inherits the TUI's environment unchanged. An exported `GSD_WORKSTREAM` would send that command to a different workstream than the row shows. It is a lower-risk path because the owner types into the session, but the two paths disagree.
**Fix:** Either document that `v` inherits the environment, or have the GUI terminal spawn remove `GSD_WORKSTREAM` and `GSD_PROJECT` for root-scoped commands.

### IN-03: `focus_status` keys off the literal prefix `"Switched to"` of another module's message

**File:** `src/ui/screens/owner_actions.rs:107-113`
**Issue:** The "type {command}" hint appears only if `multiplexer::focus_message` starts with `Switched to`. A wording change in the multiplexer silently drops the hint, and nothing couples the two.
**Fix:** Have `focus_message` return a typed outcome (`Focused(String) | Failed(String)`), or expose a `pub const FOCUS_OK_PREFIX` used by both.

### IN-04: The "10 s" in the acknowledge timeout text and a second `ACK_BUDGET` definition are not tied together

**File:** `src/ui/screens/owner_actions.rs:207-209` and `:819-822`
**Issue:** `ACK_BUDGET` is `cfg`-gated to Linux and macOS, so the message hard-codes `10 s`. Changing the budget leaves the message stale.
**Fix:** Define the constant for all targets (the non-supported platform simply never reads it) and `format!` the seconds from it.

### IN-05: The OSC 52 and copy-size messages are slightly inaccurate or unbounded for terminals

**File:** `src/ui/screens/owner_actions.rs:512-516`, `src/clipboard.rs:26`
**Issue:** `Too large to copy ({} KiB)` uses integer division, so a 64 KiB + 1 byte text prints `64 KiB` against a 64 KiB limit. The OSC 52 payload is about 4/3 of 64 KiB (about 87 KB), which some terminals and tmux builds cap lower and drop silently, while the status says `Sent ... to the terminal clipboard`.
**Fix:** Round the displayed size up (`div_ceil`), and cap the OSC 52 path lower (for example 48 KiB of source text) or say that large copies may be dropped by the terminal.

### IN-06: `start_copy_with` has no guard against overlapping copies

**File:** `src/ui/screens/mod.rs:1958-1990`
**Issue:** Each `y` starts a new blocking task and tool process. Holding `y` stacks `wl-copy` or `xclip` children (up to the 2 s budget each), and the status line flips between answers in arrival order.
**Fix:** Track an `in_flight` flag like `owner_ack_busy`, and refuse a second copy until the answer arrives.

---

_Reviewed: 2026-11-15_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
