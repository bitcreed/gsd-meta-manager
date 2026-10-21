---
phase: 30-driver-tab-on-direction-md
reviewed: 2026-10-20T00:00:00Z
depth: standard
files_reviewed: 22
files_reviewed_list:
  - src/action.rs
  - src/app.rs
  - src/cli.rs
  - src/driver/direction.rs
  - src/driver/mod.rs
  - src/driver/steer.rs
  - src/journal/mod.rs
  - src/main.rs
  - src/state_reader/driver_frontmatter.rs
  - src/ui/mod.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/driver_launch.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/driver_run.rs
  - src/ui/screens/driver_steer.rs
  - src/ui/screens/help.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/render_escape_guard.rs
  - src/watcher.rs
  - tests/driver_tab_launch_e2e.rs
findings:
  critical: 0
  warning: 6
  info: 3
  total: 9
status: issues_found
---

# Phase 30: Code Review Report

**Reviewed:** 2026-10-20
**Depth:** standard
**Files Reviewed:** 22
**Status:** issues_found

## Summary

Reviewed the Phase 30 production code (the 36 commits from `38251d5` to `HEAD`): the body transforms and the no-clobber editor flow in `driver/steer.rs`, the launch form and new-run flow in `driver_launch.rs`, the steering screens, the `App` handlers and watcher routing, the log-tail reader and the tiered renderer in `driver_run.rs`. I traced the byte-offset splices, the `write_body` closure semantics, the edit-session lifecycle (`edit_sessions` / `new_run_flow`), the launch argv path and the stale-result guards on the refresh actions.

The core is sound. The transforms re-check themselves against the store's own parser, offsets are validated before slicing, answers are validated before any byte moves, and stale or duplicated async results are dropped by run id and cursor offset. I found no data-loss or injection defect, so there are no BLOCKERs. The warnings are places where the tab promises more than the code does, or where a hardening the phase applied in one reader was not applied in its siblings.

`cargo test --no-fail-fast` ran 2815 passed, 1 failed. The failure is the expected git-version witness in `src/envelope/policy.rs`.

## Warnings

### WR-01: The launch form accepts a model name that the detached launch then rejects

**File:** `src/ui/screens/driver_launch.rs:608-618` (also `src/cli.rs:606-611`, `src/launcher/run.rs:1147`)
**Issue:** `commit_edit` validates the model by calling `parse_launch_field("--model", text)`, which joins the pair as `--model=<text>`. Clap accepts a value such as `-x` or `--foreground` in that form. The launch itself re-executes the binary with `launch_argv`, which passes `--model` and the value as two separate words. `LaunchArgs::model` has no `allow_hyphen_values`, so clap rejects `--model -x` there. The form therefore validates one parse and launches another, which breaks the module's own claim that the form "accepts exactly what `driver launch` accepts". The user gets a failed launch (reported as "Could not launch ... Nothing was started") instead of a refusal in the form. The prefill path is stricter: `recorded_model_ok` requires a leading alphanumeric, so the typed path is the only gap. The unit test `a_field_value_that_looks_like_a_flag_is_a_value` pins the `=` join and so does not catch it.
**Fix:**
```rust
// commit_edit, Field::Model branch
let ok = !text.is_empty()
    && text.chars().count() <= 100
    && !text.starts_with('-')            // launch_argv passes it as a separate word
    && !text.chars().any(|c| c.is_control())
    && parse_launch_field(field.flag(), text).is_ok();
```
Better: apply `recorded_model_ok` (or a shared predicate) to typed and recorded models alike.

### WR-02: Run-directory files are opened without a file-type guard, although the phase hardens one reader against exactly that

**File:** `src/state_reader/driver_frontmatter.rs:801`, `src/driver/steer.rs:614-618` and `745-761`, `src/ui/screens/driver_run.rs:1739-1768`
**Issue:** `read_log_delta` calls `symlink_metadata` and refuses anything that is not a regular file, with the comment "a FIFO or device is not opened (it could block this worker forever)". The other readers of the same attacker-writable directory do not. `read_driver_run_text` (`File::open(&path)`, behind every list row, the selected-run detail and the 20-tick marker scan) and `steer::read_direction` (behind the launch preflight, the new-run check, `prepare_edit` and `live_body`) open `DIRECTION.md` blocking and follow symlinks. A FIFO placed at `DIRECTION.md` by an `--unsandboxed` run or a sibling sandbox, the threat T-30-04 names, blocks a `spawn_blocking` worker forever, and `list_driver_runs` stalls on it for the whole project. Each refresh the watcher or the slow tick schedules strands another worker. There are two smaller TOCTOUs of the same kind:
- `apply_edit` checks `symlink_metadata(temp)` and then reopens `temp` by path, so a symlink swapped in between is followed.
- `read_log_delta` checks the type and then `tail_lines` reopens the path.
**Fix:** Open once, without following links or blocking, and check the type on the handle.
```rust
use std::os::unix::fs::OpenOptionsExt;
let file = std::fs::OpenOptions::new()
    .read(true)
    .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
    .open(&path)?;
if !file.metadata()?.file_type().is_file() { return Err(/* not a regular file */); }
```
Put this in one helper that `read_driver_run_text`, `read_direction` and `apply_edit` share. Have `tail_lines` take the already-checked handle.

### WR-03: `steer_run` and `apply_edit` rewrite DIRECTION.md even when they changed nothing, then say "Nothing was written"

**File:** `src/driver/steer.rs:551-562` and `787-795`; `src/driver/direction.rs:519-532`
**Issue:** `write_body` always returns `Plan::Write`, so the closure's "unchanged" and "refused" results still create a temp, `renameat2(RENAME_EXCHANGE)` it over `DIRECTION.md` and delete the displaced inode. `SteerOutcome::Unchanged` and `Refused` (and `EditOutcome::Conflict`, where the closure returns `live`) therefore each replace the file with an identical copy. That has four effects:
- It changes the inode and mtime.
- It emits a `DriverRun` watcher event (an avoidable refresh).
- It opens a window to race a human's in-place save, with the retry or `GaveUp` machinery in play for a write that did not need to happen.
- The copy ("control is already X in DIRECTION.md. Nothing was written.", "Nothing was written: ...") is not literally true.
**Fix:** Let the closure signal "keep as is" so `run_loop` returns `SpliceOutcome::Ok` without touching the file, for example `FnMut(&str) -> Option<String>`, or a `Plan::Keep` variant. `steer_run` and `apply_edit` then return `None` when `new == live` or when they refuse or conflict.

### WR-04: Two line-number bases are both labelled `DIRECTION.md:<line>`

**File:** `src/ui/screens/driver_launch.rs:378-383`, `src/app.rs:1455-1458`, `src/driver/steer.rs:771`
**Issue:** `lint_direction(text)` numbers lines from the start of whatever it is given. The launch preflight and the new-run check pass the whole file (`read_direction`), so line numbers include the tool-written frontmatter block. The edit-apply lint passes the body only (`lint_direction(&edited)`), and the `.DIRECTION.md.edit` temp the user actually edits is body-only. A problem shown in the launch form or the `Check DIRECTION.md` popup as `DIRECTION.md:57: ...` is on line 57 of the file, but the editor the user opens next (`y` on that popup, or `e`) shows it on a much lower line. The mismatch is the length of the frontmatter block, 30-odd lines in the shipped fixtures.
**Fix:** Number both the same way. Either lint `split_frontmatter(&text).1` in the two file-level callers, so the numbers match the editor, or open the editor on a line-offset temp. The first is the smaller change.

### WR-05: The launch form puts the spend and "keeps running" disclosure last, where short terminals clip it first

**File:** `src/ui/screens/driver_launch.rs:916-928` (rendered by `src/ui/screens/confirm_popup.rs:139-148, 219-223`)
**Issue:** `popup_rect` clamps the popup to `area.height` and the body paragraph is cut at the bottom (`body_h = inner.height - 3`). The form's body grows with the preflight: sandbox rows, up to 3 lint problems plus `… k more`, up to 3 warnings plus `… k more`. The paragraph that says each session "may spend up to $X ... runs detached and keeps running after you close this TUI", and the `fresh-per-item` multiplier line, are appended after all of that. On a 24-row terminal a form with warnings loses them while `y` stays live. The tests that assert the line (`driver_launch.rs:2370`) evidently use a tall buffer. The ceiling value is also shown in the `spend ceiling` field row, but the detach and multiplier statements are not.
**Fix:** Emit the spend and detach paragraph (and the `fresh-per-item` line) directly after the six field rows, before the preflight rows, so the findings are what gets clipped. Cap the findings to the rows actually available. Add a render test at 80x24 with three warnings.

### WR-06: `$VISUAL` / `$EDITOR` with arguments cannot be launched, and Phase 30 now depends on this path for every edit

**File:** `src/main.rs:847` (the line is pre-existing; Phase 30 makes it load-bearing)
**Issue:** `Command::new(&editor)` treats the whole variable as a program name, so `EDITOR="code -w"`, `"emacs -nw"` or `"nvim -u NONE"`, all common, fail with ENOENT. The new code turns that spawn error into `EditorExit::SpawnError`, which ends the new-run flow with the run left unlaunched and the user's body untouched. The help text documents "a terminal editor or a blocking wrapper script" but not that arguments are unsupported.
**Fix:** Split the variable into program and arguments before spawning, for example with `shlex::split` or `shell_words::split` (or `sh -c '"$EDITOR" "$@"'`), then append `editor_args(...)`. Failing that, say in the help row and the spawn-error message that `$EDITOR` must be a single executable.

## Info

### IN-01: `set_item_field` mishandles a heading on an unterminated last line

**File:** `src/driver/steer.rs:449-466`
**Issue:** When the item's `### I-NN` heading is the last line of the body with no trailing newline, `segment.start == body.len()` and `terminated` starts as `true`, so the new line is appended directly after the heading text (`### I-05 title- answer: foo`). `answer` is saved from corruption only because `check_item` then fails to read the answer back. The user gets a `PostCondition("answer")` refusal for a valid request instead of a written answer. The `terminated` flag only tracks non-blank body lines, never the heading's own terminator.
**Fix:** Initialise `terminated` from the heading line (`segment.eol` is known, so also record whether the heading ended with `\n`) or prepend `eol` when `segment.start` is not preceded by a newline.

### IN-02: The slug input stays "pending" when the request is refused before it starts

**File:** `src/ui/screens/driver_launch.rs:1306-1316`, `src/app.rs:1352-1394`
**Issue:** Enter sets `pending = true` and dispatches. `request_new_run` returns early (unregistered alias, no event channel) after only setting `error_message`, and nothing answers, so `pending` stays true. Enter is then dead until a character is typed. Only the `Exists` answer clears it.
**Fix:** Clear `pending` when `ctx.error_message` changes, or have the early-return paths send a `DriverNewRunCreated { result: Err(Other(..)) }` so the single answer path resets state and pops the screen.

### IN-03: The launch form and the launcher read the `control` word with different readers

**File:** `src/ui/screens/driver_launch.rs:383, 466-471`; `src/launcher/run.rs:246`
**Issue:** The form's `control_blocking` uses `parse_direction(...).control`: the loose, whole-body matcher where `pause` or `stop` on any line wins. The launcher's `preflight_direction` uses `driver_templates::lint::control_line`: the first `- control:` line inside `## Direction`. They agree on well-formed files. For a stray control line outside `## Direction` or an indented one, the form can block a launch the launcher would accept (or the reverse), while its copy says "the launcher refuses...". This is the two-readers drift WR-01 of Phase 27 warned about.
**Fix:** Have `launch_preflight` call `control_line(&text)`, the function the launcher itself uses, so the form predicts the launcher's verdict exactly.

---

_Reviewed: 2026-10-20_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
