---
phase: 30-driver-tab-on-direction-md
plan: 07
subsystem: driver-tab
tags: [rust, filesystem, hardening, direction-md, ratatui, gap-closure]
status: complete
gap_closure: true

requires:
  - phase: 30-driver-tab-on-direction-md
    provides: 30-05 Driver tab, steering write path and run-directory readers
provides:
  - "open_run_file: one O_NOFOLLOW | O_NONBLOCK open with a regular-file check on the handle, shared by every run-dir reader"
  - "tail_file: tail over an already-open handle; tail_lines delegates"
  - "Plan::Keep: an unchanged body is never rewritten (same inode, mtime, no temp, no watcher event)"
  - "answer / un-hold on an unterminated last `### I-NN` heading"
  - "BodyControl { Loading, NoLine, Word }: FULL header row 3 says Loading... until the body is read"
  - "INJECTION_RETIRED shortened to 54 cells"
affects: [30-08, 30-09]

actuals:
  tokens: 7400
  tasks: 3
  commits: 6
plan_head_before: 5d3c617a731525b9a4030343e77e5d11eb62191f
plan_head_after: d41e23d7bb7078e02ef7d587ba1a49caaadba147
commits: 6

tech-stack:
  added: []
  patterns:
    - "one guarded open per untrusted run-dir name; every check and read uses that handle"
    - "a write closure that returns the live body unchanged short-circuits the exchange-verify loop"

key-files:
  created: []
  modified:
    - src/state_reader/driver_frontmatter.rs
    - src/driver/steer.rs
    - src/driver/direction.rs
    - src/journal/reader.rs
    - src/ui/screens/driver_run.rs
    - src/ui/screens/detail.rs

key-decisions:
  - "[inferred — audit] open_run_file maps ELOOP (O_NOFOLLOW on a link) to InvalidData `not a regular file (a symbolic link)`, so every caller, apply_edit included, reports a planted link and a planted FIFO with the same words."
  - "[inferred — audit] Plan::Keep is decided before the sanity check, as the plan says, so an unchanged body that already lacks a section header is now Ok/Unchanged/Refused rather than InsaneNewBody/Failed."
  - "[inferred — audit] The IN-01 append opens with the segment's line end, which is LF for a heading with no terminator (eol_of looks at that one line); a CRLF body whose last heading is unterminated therefore gets LF on the appended lines."
  - "[inferred — audit] Loading... uses muted_style (DarkGray + DIM), the DRY_RUN_LOADING idiom used elsewhere on the tab; Yellow stays reserved for a read body with no control line."
  - "[inferred — audit] New INJECTION_RETIRED copy: `Injection is retired: use [c]ontrol, [a]nswer, [e]dit.` (54 cells; fits the 62-cell status room at 80 columns for alias `proj`). Plan 30-09 updates the UI-SPEC row."

requirements-completed: [DTAB-01, DTAB-02, DTAB-03, DTAB-04]

coverage:
  - id: D1
    description: "A FIFO or symlink at DIRECTION.md is refused promptly by read_driver_run_text, list_driver_runs and steer::read_direction"
    requirement: DTAB-01
    verification:
      - kind: unit
        ref: "src/state_reader/driver_frontmatter.rs#read_driver_run_text_refuses_a_fifo_direction_without_blocking"
        status: pass
      - kind: unit
        ref: "src/state_reader/driver_frontmatter.rs#open_run_file_refuses_a_fifo_without_blocking"
        status: pass
      - kind: unit
        ref: "src/driver/steer.rs#read_direction_refuses_a_fifo_without_blocking"
        status: pass
    human_judgment: false
  - id: D2
    description: "The edit temp and LOG.md are each opened once and checked and read on that handle"
    requirement: DTAB-03
    verification:
      - kind: unit
        ref: "src/driver/steer.rs#launcher_edit_a_temp_that_is_a_symlink_is_refused"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#read_log_delta_skips_a_fifo_log_without_blocking"
        status: pass
      - kind: unit
        ref: "src/journal/reader.rs#tail_file_reads_from_an_open_handle"
        status: pass
    human_judgment: false
  - id: D3
    description: "A steering write or edit apply that changes nothing does not touch DIRECTION.md"
    requirement: DTAB-03
    verification:
      - kind: unit
        ref: "src/driver/direction.rs#write_body_leaves_the_file_alone_when_the_body_is_unchanged"
        status: pass
      - kind: unit
        ref: "src/driver/steer.rs#steer_run_unchanged_and_refused_leave_the_inode_alone"
        status: pass
      - kind: unit
        ref: "src/driver/steer.rs#apply_edit_conflict_leaves_the_inode_alone"
        status: pass
    human_judgment: false
  - id: D4
    description: "Answer and un-hold work on a heading that is the last line with no newline"
    requirement: DTAB-03
    verification:
      - kind: unit
        ref: "src/driver/steer.rs#answer_on_an_unterminated_last_heading_writes_a_line_of_its_own"
        status: pass
    human_judgment: false
  - id: D5
    description: "FULL header row 3 reads `control Loading...` (DarkGray) until the body is read; (no control line) stays Yellow for a read body"
    requirement: DTAB-02
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#launcher_view_full_header_says_loading_until_the_detail_arrives"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#launcher_view_full_header_flags_a_body_without_a_control_line"
        status: pass
    human_judgment: false
  - id: D6
    description: "The retired-i copy keeps the three steering keys visible at 80 columns"
    requirement: DTAB-04
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#launcher_steer_the_retired_copy_fits_an_80_column_status_bar"
        status: pass
    human_judgment: false

duration: ~30 min
completed: 2026-10-20
---

# Phase 30 Plan 07: Run-dir I/O hardening and honest "nothing was written" Summary

**One `O_NOFOLLOW | O_NONBLOCK` open with a handle-side type check now guards every Driver-tab read of a run directory, an unchanged steering write leaves `DIRECTION.md` untouched, and the FULL header says `Loading...` until the body arrives.**

## Accomplishments

- **WR-02 (tracer, Task 1).** `open_run_file` (`driver_frontmatter.rs`) is used by `read_driver_run_text` (run list, selected-run detail, marker scan), `steer::read_direction` (launch preflight, new-run check, `prepare_edit`, `live_body`), `apply_edit` (the temp is type-checked, size-checked and read through one handle) and `read_log_delta` (`LOG.md` is opened once, `(dev, ino)` comes from that handle, and the `LOG_READS_PER_REFRESH` loop tails it with the new `tail_file`). A FIFO or symlink is refused promptly instead of parking a `spawn_blocking` worker; the FIFO tests use a 5 s `recv_timeout`. `tail_lines` keeps its signature and delegates to `tail_file`.
- **WR-03 (Task 2).** `Plan::Keep` in `direction.rs`: when the edited body equals the body it was given, `run_loop` returns `Ok(SpliceOutcome::Ok)` before the mode read, the temp and the exchange. "Nothing was written." is now literally true for an already-set control word, a refused transform and an edit conflict.
- **IN-01 (Task 2).** `set_item_field` starts `terminated` from the byte before the segment, so a heading on an unterminated last line gets a line end before the appended `- answer:` / `- hold:` line.
- **UI D2 (Task 3).** `BodyControl { Loading, NoLine, Word }` replaces the `Option<&str>` of `header_rows` / `header_row_3`. No cached detail is `Loading` (`Loading...`, DarkGray), a detail with no `- control:` line is `NoLine` (Yellow), `Some(word)` is `Word`. MEDIUM and SUMMARY keep the frontmatter echo.
- **UI D4 retired half (Task 3).** `INJECTION_RETIRED` = `Injection is retired: use [c]ontrol, [a]nswer, [e]dit.` (54 cells).
- **Platform hygiene (Task 3).** `give_control` in `detail.rs` is `#[cfg(target_os = "linux")]`, like all its callers.

## Task Commits

| Task | RED | GREEN |
|------|-----|-------|
| 1 (tracer) guarded open | `82317ad` test | `7718033` fix |
| 2 no-op write, IN-01 | `82904b3` test | `4df4dd1` fix |
| 3 Loading header, retired copy, give_control | `73a9aef` test | `d41e23d` fix |

Tracer verified end-to-end before expansion: the Task 1 `<verify>` (lib tests for `state_reader::driver_frontmatter::`, `driver::steer::`, `journal::reader::`, `screens::driver_run::`; `spawn_seam_guard` and `async_blocking_guard`; clippy) passed.

## TDD Gate Compliance

Each task has a `test(30-07)` commit followed by a `fix(30-07)` commit. RED evidence is Rust `cargo test` text output, which the `tdd-red-evidence` classifier has no adapter for, so the semantic assessment is recorded here. As in 30-06, a compile-only skeleton (`open_run_file` doing a bare `File::open`, `tail_file` returning an empty read, `BodyControl` with unchanged rendering) was committed with the tests so they fail on assertions, not on the build.

- Task 1: six tests failed on the planned assertions (`open_run_file_refuses_a_fifo_without_blocking`, `read_driver_run_text_refuses_a_fifo_direction_without_blocking` and `read_direction_refuses_a_fifo_without_blocking` hit the 5 s timeout; the two symlink tests got `Ok`; `tail_file_reads_from_an_open_handle` got an empty read). `launcher_edit_a_temp_that_is_a_symlink_is_refused` and `read_log_delta_skips_a_fifo_log_without_blocking` passed in RED by design: the old `symlink_metadata` guards already refused these cases, and the tests pin the behaviour across the switch to one handle.
- Task 2: three tests failed (`write_body_leaves_the_file_alone_when_the_body_is_unchanged` inode changed; `steer_run_unchanged_and_refused_leave_the_inode_alone` inode changed; `answer_on_an_unterminated_last_heading_writes_a_line_of_its_own` got `PostCondition("answer")`). `apply_edit_conflict_leaves_the_inode_alone` passed in RED by design (see Deviations 2).
- Task 3: four tests failed (the old long copy, the old width, and `(no control line)` where `Loading...` is planned).

## Deviations from Plan

**1. [Scope note] Plan::Keep and an already-insane live body**
- `Plan::Keep` is decided before the sanity check, as the plan prescribes. An unchanged body that already lacks `## Direction` or `## Items` is therefore `Ok` (surfacing as `Unchanged` or `Refused`), not `InsaneNewBody` (`Failed`). The existing `launcher_steer_a_missing_file_and_an_insane_body_are_not_written` test accepts `Failed | Refused` and still passes; the file is never written either way.

**2. [Scope note] The apply_edit conflict test covers the pre-check path**
- The closure-side conflict (the body changing between the pre-check and the write) cannot be provoked deterministically without a hook in `apply_edit`. `apply_edit_conflict_leaves_the_inode_alone` exercises the pre-check `Conflict` (a body changed while the editor was open) and asserts the inode, the absence of a splice temp and the kept edit temp. The closure branch returns `live.to_string()`, which `Plan::Keep` handles exactly as the `steer_run` no-op case that `steer_run_unchanged_and_refused_leave_the_inode_alone` pins.

**3. [Scope note] CRLF in the IN-01 test**
- My first draft also asserted CRLF line ends for an unterminated last heading in a CRLF body. `eol_of` looks only at the heading line, which has no terminator, so the appended lines are LF. The plan specifies "the segment's `eol`", so I kept the plan's behaviour and dropped that assertion.

**Total deviations:** 0 auto-fixed bugs; 3 scope notes. **Impact:** none on behaviour.

## Inferred Decisions `[inferred — audit]`

See `key-decisions` above: the ELOOP mapping, `Plan::Keep` before the sanity check, the LF line end on an unterminated last heading, `muted_style` for `Loading...`, and the new `INJECTION_RETIRED` copy `Injection is retired: use [c]ontrol, [a]nswer, [e]dit.`. The plan's own `[inferred — audit]` choices (spec-less probe fallback skipped; assumption-delta no-change; IN-01 info finding included) were followed as written.

## Verification

- `rtk proxy cargo test --no-fail-fast`: 4048 passed, 1 failed, 24 ignored (baseline after 30-06: 4033 passed; 15 new tests). The one failure is the expected environment-only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check` clean.
- `cargo test --test driverctl_store` 53 passed; `--test spawn_seam_guard` 9 passed; `--test async_blocking_guard` 41 passed.
- `git diff --stat -- Cargo.toml Cargo.lock` over the plan's commits: empty.
- Acceptance greps: `pub fn open_run_file` 1; `open_run_file` in `steer.rs` 3 and in `driver_run.rs` 3; `pub fn tail_file` 1; `NOFOLLOW` in `driver_frontmatter.rs` 4; `Plan::Keep` in `direction.rs` 2; `pub enum BodyControl` 1; new `INJECTION_RETIRED` string 1; `#[cfg(target_os = "linux")]` directly above `fn give_control`; `write_body` signature unchanged.

## Threat Surface

T-30-23 (FIFO or device in a run dir), T-30-24 (re-open after check) and T-30-25 (no-op rewrites) are mitigated as planned; T-30-26 (unterminated heading) too. No package added.

One related observation, not changed (outside the plan's file list and its stated scope): `direction.rs` `run_loop` reads `DIRECTION.md` with `std::fs::read(&target)` by path, so a FIFO planted there would still block a steering write that reaches it. Every Driver-tab read that precedes a write (preflight, `prepare_edit`, `live_body`, `apply_edit`'s pre-check) now refuses a FIFO first, so a normal steering path does not get that far; a guard inside `run_loop` would also protect `driverctl`'s splice and belongs to a plan that owns `direction.rs` hardening.

## Known Stubs

None. No entries appended to `.planning/WINDOWS.md`.

## Next

Ready for 30-08.

## Self-Check: PASSED

- Files: `src/state_reader/driver_frontmatter.rs`, `src/driver/steer.rs`, `src/driver/direction.rs`, `src/journal/reader.rs`, `src/ui/screens/driver_run.rs`, `src/ui/screens/detail.rs` carry the changes above.
- Commits `82317ad`, `7718033`, `82904b3`, `4df4dd1`, `73a9aef`, `d41e23d` are ancestors of HEAD.
- All task `<acceptance_criteria>` greps and every `<verify>` command re-run green.
