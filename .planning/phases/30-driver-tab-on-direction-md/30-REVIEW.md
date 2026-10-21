---
phase: 30-driver-tab-on-direction-md
reviewed: 2026-10-21T00:00:00Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - src/app.rs
  - src/driver/direction.rs
  - src/driver/steer.rs
  - src/journal/reader.rs
  - src/main.rs
  - src/state_reader/driver_frontmatter.rs
  - src/ui/screens/confirm_popup.rs
  - src/ui/screens/create_project.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/driver_launch.rs
  - src/ui/screens/driver_run.rs
  - src/ui/screens/driver_steer.rs
  - tests/editor_command.rs
  - tests/launcher_config_dir.rs
findings:
  critical: 0
  warning: 2
  info: 2
  total: 4
status: issues_found
---

# Phase 30: Code Review Report (re-review of gap-closure plans 30-06..30-09)

**Reviewed:** 2026-10-21
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

Incremental review of `git diff dc5934f HEAD` over the listed files: the word-wrap-accurate popup sizing and the clipped-popup guard (30-06), the guarded run-directory open, `Plan::Keep` and the unterminated-heading fix (30-07), `editor_command` (30-08), and the launch form reordering, `model_name_ok`, body-relative lint numbering and `control_line` reuse (30-09).

Most of the prior round is genuinely resolved. The new code is mostly careful. Highlights I traced and found sound:

- `open_run_file` (`O_NOFOLLOW | O_NONBLOCK`, type check on the handle, ELOOP mapped to "not a regular file") and its three callers `read_driver_run_text`, `read_direction` and `apply_edit`. `apply_edit` now opens the temp once and reads, sizes and caps that handle.
- `read_log_delta` and `tail_file`: the `(dev, ino)` is taken from the same handle the loop tails, `tail_file` seeks every call, and the restart and reset logic is unchanged.
- `Plan::Keep` in `run_loop` and the `steer_run` and `apply_edit` closures that feed it. A conflict, a refusal and an already-set control word all return the live body, so no temp, no exchange and no inode change happen. The retry-after-race arms still resolve correctly.
- `fit_list`, `body_room`, `wrapped_rows` and `popup_rect`. `body_room.1 == popup body_h`, so the `clipped = body_rows > body_h` test agrees with what the fitted callers computed.
- `editor_command`. The path and line ride as separate argv elements after `$0` and never enter the script text. `tests/editor_command.rs` exercises hostile names against a real `sh`.

`rtk proxy cargo test --no-fail-fast --test editor_command --test launcher_config_dir` passes (5 and 11 tests).

No critical findings. Two warnings remain: one prior finding only partly closed, and one regression introduced by the new model rule.

## Prior findings status

- **WR-01** (launch form accepts a model the launch rejects): **Resolved.** `commit_edit` now requires `model_name_ok(text)` (`src/ui/screens/driver_launch.rs:619`). `model_name_ok` demands an alphanumeric first character, so `-x` and `--foreground` are refused in the form (`:231-240`). The rule is too narrow in the other direction, see new WR-07.
- **WR-02** (run-directory files opened without a file-type guard): **Partly resolved, still present.** `read_driver_run_text`, `read_direction`, `apply_edit` and `read_log_delta` all go through `open_run_file` now (`src/state_reader/driver_frontmatter.rs:772-825`, `src/driver/steer.rs:622-625` and `751-770`, `src/ui/screens/driver_run.rs:1764`). `DIRECTION.md` is still read by path, unguarded, inside `run_loop` (`src/driver/direction.rs:601`, `651`, `672`). Re-reported below.
- **WR-03** (no-op writes rewrite `DIRECTION.md`): **Resolved.** `Plan::Keep` (`src/driver/direction.rs:501-503`, `:532-534`, `:616`) returns `SpliceOutcome::Ok` before the mode read, the temp and the exchange. Tests assert the same inode and mtime and no temp (`direction.rs` `write_body_leaves_the_file_alone_when_the_body_is_unchanged`, `steer.rs` `steer_run_unchanged_and_refused_leave_the_inode_alone`, `apply_edit_conflict_leaves_the_inode_alone`).
- **WR-04** (two line-number bases): **Resolved for the TUI paths.** `launch_preflight` (`driver_launch.rs:388`) and `request_new_run_check` (`src/app.rs:1559-1561`) lint `split_frontmatter(&text).1`, as `apply_edit` does. A smaller cross-tool remainder is recorded as IN-05.
- **WR-05** (spend and "keeps running" disclosure clipped first): **Resolved.** `essential_lines` puts the title, the spend and detach paragraph, the `fresh-per-item` line and the sandbox rows before the six fields. `fitted_body_lines` (`driver_launch.rs:943-966`) adds findings only into the rows left, and the lint popups fit their lists after the header and question. A clipped popup is inert on `y` (`confirm_popup.rs` `confirm_unless_clipped`).
- **WR-06** (`$EDITOR` with arguments): **Resolved.** `editor_command` (`src/app.rs:708-730`) wraps a string with shell-special characters in `sh -c '<editor> "$@"'`. `src/main.rs:849` spawns the returned pair. The only remainder is that a missing wrapped editor now surfaces as `sh` exit 127 (`EditorExit::Failed`) instead of `SpawnError`. That is cosmetic and not reported.
- **IN-01** (`set_item_field` on an unterminated last heading): **Resolved.** `terminated` is initialised from the byte before `segment.start` (`src/driver/steer.rs:455-456`), so the new line opens with `eol`. A test covers the `answer` path.
- **IN-02** (slug input stays pending when the request is refused): **Resolved.** The unregistered-alias and no-event-channel early returns in `request_new_run` now route through `finish_new_run(Err(NewRunFailure::Other(..)))` (`src/app.rs:1454-1471`), which pops the slug input.
- **IN-03** (form and launcher read `control` differently): **Resolved.** `launch_preflight` calls `control_line(&text)` (`driver_launch.rs:390`), the function `launcher::run::preflight_direction` uses, and `control_blocking` compares the word exactly (`!= "run"`), as the launcher does.

## Warnings

### WR-02: Run-directory files are opened without a file-type guard, although the phase hardens one reader against exactly that

**File:** `src/driver/direction.rs:601`, `651`, `672` (reached from `src/app.rs:1718-1724` through `src/driver/steer.rs:551`)
**Issue:** This is the residual of the prior finding. The guarded open now covers every read the Driver tab makes before a write. It does not cover the read inside the write itself. `run_loop`, the engine behind `write_body`, still does `std::fs::read(&target)` by path, and the same for the displaced inode after `renameat2(RENAME_EXCHANGE)` (`std::fs::read(&temp_path)`) and the exchange-back (`std::fs::read(&temp_path)`). Those opens block on a FIFO with no writer and follow symlinks.

The 30-07 SUMMARY dismisses this as "a normal steering path does not get that far", on the grounds that the guarded reads in the preflight, `prepare_edit`, `live_body` and `apply_edit` come first. That is true for the editor flow and false for the `c` and `a` keys. `request_steer` calls `steering_target`, which only validates names and the channel, then `spawn_blocking(steer_run)`, and `steer_run` goes straight to `write_body`. No guarded read precedes it. Whenever the cached run row is stale (a watcher refresh has not landed), a `DIRECTION.md` swapped for a FIFO makes the worker block in `open(2)`. By then `locked()` has taken `DIRECTION.lock`, so the stuck worker also holds the lock, and the driver's own `driverctl` splices wait on it until they time out. This is the T-30-04 threat model (a sibling sandbox or an `--unsandboxed` run with write access to the run directory), which the phase claims to have closed ("one guarded open for every run-dir read").
**Fix:** Give `run_loop` the same guard. A shared helper in `driver_frontmatter` (or a `direction.rs`-local copy, since `direction.rs` is also `driverctl`'s) avoids duplicating the open flags.
```rust
// run_loop, instead of std::fs::read(&target)
let bytes = match crate::state_reader::driver_frontmatter::open_run_file(&target)
    .and_then(|mut f| { let mut b = Vec::new(); f.read_to_end(&mut b).map(|_| b) })
{
    Ok(b) => b,
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(SpliceOutcome::NoDirection),
    Err(e) => return Err(io_err(target.display(), e)),
};
```
Use the same helper for the two `temp_path` reads (treat a refusal as a mismatch, which the code already does for an unreadable displaced file). Add a test that `steer_run` over a FIFO `DIRECTION.md` returns within a few seconds, like the existing `read_direction_refuses_a_fifo_without_blocking`.

### WR-07: The model rule is narrower than `claude --model`, so the form refuses `opus[1m]`-style names that the CLI accepts

**File:** `src/ui/screens/driver_launch.rs:231-240` (applied at `:619`)
**Issue:** 30-09 replaced the old typed-model check (any non-control string clap accepts, up to 100 characters) with the recorded-model charset `[A-Za-z0-9][A-Za-z0-9._:-]{0,63}`. That fixes the flag-looking-value problem (WR-01), but it also refuses `]` and `[`. Claude Code's long-context selector is a bracket suffix (`opus[1m]`, `sonnet[1m]`, `claude-sonnet-4-5[1m]`). The `driver launch` CLI still accepts these, since `LaunchArgs::model` is any string. A user who wants a 1M-context driver run therefore cannot type it into the form, and the only message is "must be a model name". The new test even pins `claude-opus-4-7:1m` as the good form, which is not a Claude model syntax. The relaunch prefill has the same gap (a recorded `opus[1m]` is silently dropped for the default), but that is existing behaviour. The leading-alphanumeric rule alone is what closes the argv-injection hole, and clap's own parse is still run afterwards.
**Fix:** Allow the bracket suffix, keeping the alphanumeric first character and the length bound:
```rust
first.is_ascii_alphanumeric()
    && model.chars().count() <= 64
    && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-' | '[' | ']'))
```
Update `launcher_launch_a_typed_model_that_looks_like_a_flag_is_refused` to accept `opus[1m]`, and keep `-x`, `--foreground`, `a b` and `.opus` as the refused set.

## Info

### IN-04: A lint popup whose findings get zero rows is forced into the clipped, unconfirmable state

**File:** `src/ui/screens/driver_launch.rs:1519-1523`, `src/ui/screens/driver_steer.rs:895`
**Issue:** `LintPopupScreen` and `EditLintConfirmScreen` pass `room.saturating_sub(taken).min(..)` to `fit_list` without a `> 0` guard. When that budget is 0, which is the terminal height at which the header and the question exactly fill the room, `fit_list` returns a lone `… k more` row ("so the caller's popup is taller than `rows` and clips"). The popup is then clipped and `y` is inert, although the header and the question would have fit without it. `fitted_body_lines` in the launch form has the guard (`if left > 0`); these two popups do not. It affects one or two terminal heights, and the safe direction is a refused `y`.
**Fix:** Skip the list when the budget is 0, as the form does:
```rust
let rows = room.saturating_sub(taken).min(POPUP_PROBLEMS_SHOWN + 1);
if rows > 0 { body.extend(fit_list(problems, rows, inner)); }
```

### IN-05: Body-relative `DIRECTION.md:<line>` in the TUI now differs from `driver check` and the launcher's own refusal

**File:** `src/ui/screens/driver_launch.rs:388` (preflight), `src/app.rs:1559-1561`, against `src/launcher/run.rs:246` (`lint_direction(&text)`), whose `DirectionInvalid` problems feed `launch_error_copy` (`driver_launch.rs:456`)
**Issue:** WR-04's fix numbers TUI findings from the first body line, which matches the editor. The launcher and `driver check` still lint the whole file, with the tool block counted. If the file changes between the preflight and the launch, `launch_error_copy` prints "First problem: DIRECTION.md:57: ..." with a whole-file number, and the same problem is "DIRECTION.md:25" in the form. The label gives no hint which base is meant. The window is narrow (the preflight normally prevents it), so this is informational.
**Fix:** Either prefix the TUI numbers with a base-revealing label (`body line 25`), or have `launch_error_copy` re-base a `DirectionInvalid` problem by the frontmatter line count. Whichever is chosen, the message and the editor should name the same line.

---

_Reviewed: 2026-10-21_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
