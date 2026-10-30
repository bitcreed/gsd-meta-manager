---
phase: 30-driver-tab-on-direction-md
plan: 02
subsystem: ui
tags: [rust, ratatui, tui, driver-tab, frontmatter, log-tail, heartbeat, byte-offset-tail]

requires:
  - phase: 30-driver-tab-on-direction-md
    provides: 30-01 launcher-first Driver tab (driver_run.rs, LauncherRunsRefreshed, selection pinned to the run id, DriverKeys footer)
  - phase: 27-driverctl-driver-frontmatter
    provides: read_driver_run, DerivedStatus, parse_direction, blank_comments/section_heading, driverctl::log::parse_log
provides:
  - "driver::steer::read_items: pure DIRECTION body item reader (id, title, hold, answer) sharing the comment/section/item readers"
  - "read_driver_run_text: one capped read of DIRECTION.md returning the view and the text"
  - "RunDetail/load_run_detail, merged_items, item cursor ([ and ]) and the items section"
  - "header rows 3-6, heartbeat_stale (the one stale rule), caps bars and audit row, connected rows"
  - "LOG.md live tail by byte offset (read_log_delta/apply_log_read), park_rules map, LOG pane with follow/scroll"
affects: [30-03, 30-04, 30-05, 31]

actuals:
  tokens: 80000
  tasks: 3
  commits: 5
plan_head_before: d520cdb7eb23c22a47f5e8f9cc5fe60bdf6f2750
plan_head_after: ebd69579fd58c86be4bb1ac0cad68b70c7d3ef80

tech-stack:
  added: []
  patterns:
    - "one blocking read per concern on spawn_blocking, applied by a handler that drops a result for any run but the selected one"
    - "byte-offset tail with a from_offset applicability check, so a duplicated or stale read shows no line twice"
    - "stale = live AND age > the file's own threshold; the lock stays the liveness authority"
    - "status/glyph/colour only from DerivedStatus, the closed ItemState enum, numeric caps and the lock probe; LOG text styling is structural only"

key-files:
  created:
    - src/driver/steer.rs
  modified:
    - src/ui/screens/driver_run.rs
    - src/state_reader/driver_frontmatter.rs
    - src/driver/direction.rs
    - src/driver/mod.rs
    - src/action.rs
    - src/app.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs

key-decisions:
  - "heartbeat_stale(fm: &DriverFrontmatter, derived: &DerivedStatus, now: SystemTime) -> bool is the one stale rule; 30-03 reuses it for the status tokens and SUMMARY rows"
  - "The park-rule map for 30-04 is ProjectViewCache::launcher_log.park_rules: BTreeMap<item id, rule>, valid only while launcher_log.run_id equals the selected run id"
  - "A failed single-run read is answered by a full re-listing (typed unreadable row) instead of synthesising a FrontmatterError from a String"
  - "The LOG pane shares driver_scroll_offset/driver_follow/driver_viewport with the legacy output pane; the shipped PgUp/PgDn/f/G arms already drive it"

patterns-established:
  - "pinned_index: one selection rule shared by the blocking task (which run's detail/log to read) and the handler (which run to select)"
  - "status_cell_at/run_row_at take `now`; the clock-reading wrappers are for tests and one-shot callers"

requirements-completed: [DTAB-01, DTAB-02, DTAB-04]

coverage:
  - id: D1
    description: "The selected run's items are one merged list: DIRECTION body items in body order (id, title, hold, answer), then frontmatter-only items, each with state/dispatches/outcome from item_states; body-only items read 'unseen', frontmatter-only items carry '[not in DIRECTION]'; the cursor starts on the first parked item."
    requirement: "DTAB-01"
    verification:
      - kind: e2e
        ref: "src/app.rs#launcher_view_items_merge_body_and_frontmatter"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#merged_items_keep_body_order_then_frontmatter_only_in_item_states_order"
        status: pass
      - kind: unit
        ref: "src/driver/steer.rs#the_template_yields_one_item_with_hold_false_and_no_answer"
        status: pass
    human_judgment: false
  - id: D2
    description: "Header rows 3-6 show the live control word from the body ('(pending)' or '(at next launch)' when the frontmatter has not echoed it, never 'paused'), sandbox posture and prompt, current position and last event, each with its named placeholder."
    requirement: "DTAB-01"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#the_control_table_follows_the_ui_spec"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#the_full_header_has_six_rows_with_the_ui_spec_content"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#empty_header_fields_render_their_named_placeholders"
        status: pass
    human_judgment: false
  - id: D3
    description: "A live run (lock held) whose heartbeat is older than the file's own heartbeat_stale_after_s reads '◈ stale' in the list and 'stale (after ...)' on the heartbeat row and is never called crashed; exactly the threshold is not stale, one second over is; a run that is not live is never judged. The age advances on the existing tick without file I/O."
    requirement: "DTAB-02"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#the_stale_boundary_is_the_files_threshold_and_the_age_must_exceed_it"
        status: pass
      - kind: integration
        ref: "src/ui/screens/driver_run.rs#a_held_lock_with_an_old_heartbeat_reads_stale_and_a_free_lock_reads_crashed"
        status: pass
      - kind: unit
        ref: "src/app.rs#the_launcher_heartbeat_gate_truth_table"
        status: pass
    human_judgment: false
  - id: D4
    description: "Caps show used/limit with a 10-cell floor-division bar, at-or-over-limit rows Yellow with a BOLD value, 'no limit' instead of a bar when unlimited, and a limit of 0 as a full at-limit bar; the audit row flags logged != observed in Red with MISMATCH (surviving the 39-cell pane) and shows denied > 0 in Yellow; connected[] lists at most three rows then '... k more'."
    requirement: "DTAB-01"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#the_audit_row_flags_logged_not_equal_to_observed_and_only_that"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#mismatch_survives_the_narrowest_detail_pane"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#connections_list_three_rows_then_a_more_row_and_omit_absent_fields"
        status: pass
    human_judgment: false
  - id: D5
    description: "LOG.md is tailed live by byte offset on spawn_blocking: an append reads only the new bytes, a torn last line waits for its newline, truncation and (dev, ino) replacement reset the ring with the reset notice, a read that did not start at the tail's cursor is discarded, the template preamble is skipped, and only LOG.md is read."
    requirement: "DTAB-04"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#launcher_log_an_append_reads_only_the_new_bytes_and_never_a_line_twice"
        status: pass
      - kind: integration
        ref: "src/app.rs#launcher_log_appends_arrive_once_through_the_refresh_actions"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#launcher_log_a_replacement_is_detected_by_device_and_inode"
        status: pass
    human_judgment: false
  - id: D6
    description: "Hostile frontmatter, body and LOG text reaches no terminal cell unescaped at any new site (header rows 2-6, item rows, cursor-item row, connection rows, LOG header and detail lines)."
    requirement: "DTAB-01"
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_selected_run_view_sites_are_reached"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#launcher_log_a_hostile_header_and_detail_line_render_escaped"
        status: pass
    human_judgment: false
  - id: D7
    description: "Layout quality of the stacked selected-run view (header, items, caps, connected, LOG) at real terminal sizes: the sections are stacked without height tiers, which plan 30-03 owns."
    verification: []
    human_judgment: true
    rationale: "Whether the stacked layout reads well, and where it should give way, is the 30-03 height-tier design; this plan asserts each section's content, not the composition's look."

duration: 34min
completed: 2026-10-21
status: complete
---

# Phase 30 Plan 02: Selected-Run View From the Run's Own Files Summary

**The Driver tab's selected run now shows its merged DIRECTION items, live body control, sandbox/prompt/position/last-event header, a heartbeat age with a lock-honest stale flag, caps with bars and an audit MISMATCH row, `connected[]`, and a LOG.md tail read by byte offset, all from the run's files with no Claude call.**

## Performance

- **Duration:** 34 min
- **Started:** 2026-10-21T00:31:45Z
- **Completed:** 2026-10-21T01:06:00Z
- **Tasks:** 3 (5 commits: Task 1 in three groups)
- **Files modified:** 12 (1 created)

## Accomplishments

- `driver::steer::read_items` reads each body item's id, title, `hold` and `answer` through the shared comment, section and item readers (`item_re`/`control_re` made `pub(crate)`, no regex copy); `read_driver_run_text` returns the view and the text from a single capped read. `launcher_view_items_merge_body_and_frontmatter` proves I-01 parked + I-02 `· unseen` + I-09 `done [not in DIRECTION]`, the cursor on I-01, and that a later `DIRECTION.md` event re-reads only the selected run (`LauncherRunRefreshed`), with no project re-parse.
- Header rows 3-6, `heartbeat_stale`, `format_age`, `cap_bar`, `control_shown`: a held lock with a weeks-old driver heartbeat renders `◈ stale` and `stale (after 5:00)` and never `crashed`; freeing the lock turns the same bytes into `✗ crashed`. Exactly 300 s is not stale, 301 s is; the threshold is the file's `heartbeat_stale_after_s`.
- Caps (floor-division bars, at-limit Yellow/BOLD, `no limit`, limit 0), the audit row (Red + BOLD `MISMATCH` on `logged != observed`, kept on the 39-cell pane by a narrowing ladder), connected rows (3 then `… k more`), and `launcher_heartbeat_redraw_wanted` so the age advances on the existing tick only while a live launcher run is on screen.
- LOG.md tail: `read_log_delta` (4 reads of at most 4 MiB, `symlink_metadata` guard, `(dev, ino)` identity under `#[cfg(unix)]`), `apply_log_read` (applicability by `from_offset`), preamble skip, header rewrite to local `HH:MM:SS`, `push_record` for every line, bounded `park_rules` from the closed rule set, a `[following]`/`[scrolled +N]`/`[not live]` rule, and footer `[[/]]item`, the `Driver Item States` legend and the LOG help rows.

## Task Commits

1. **Task 1 group A: pure readers** - `d0e469c` (feat)
2. **Task 1 group B: merged items and items section** - `dd6a0ad` (feat)
3. **Task 1 group C: refresh path and item cursor keys** - `9ed7761` (feat)
4. **Task 2: header 3-6, stale rule, caps, audit, connected** - `26c10bc` (feat)
5. **Task 3: LOG.md tail, park rules, footer, help** - `ebd6957` (feat)

**Plan metadata:** committed with this file (docs: complete plan)

## Files Created/Modified

- `src/driver/steer.rs` (new) - `BodyItem`, `read_items`
- `src/ui/screens/driver_run.rs` - `RunDetail`, `load_run_detail`, `merged_items`, items/caps/connected sections, header rows, stale rule, LOG tail (`LogRead`, `read_log_delta`, `apply_log_read`, `render_log_section`), `DriverKeys.item`, `ITEM_STATES_LEGEND`
- `src/state_reader/driver_frontmatter.rs` - `read_driver_run_text`, `is_plain_run_id`
- `src/driver/direction.rs`, `src/driver/mod.rs` - `item_re`/`control_re` `pub(crate)`, `pub mod steer`
- `src/action.rs` - `LauncherRunsRefreshed { detail, log }`, `LauncherRunRefreshed`
- `src/app.rs` - reducers, `DriverRun` event routing, `launcher_heartbeat_redraw_wanted`, tests
- `src/ui/screens/mod.rs` - `LauncherLogTail`, cache fields, `schedule_selected_run_refresh`, `refresh_selected_run_if_missing`
- `src/ui/screens/detail.rs` - `[`/`]` item cursor keys, footer token, selection refresh
- `src/ui/screens/driver.rs` - `pane_header_line`, `ring_overflow_notice` visibility
- `src/ui/screens/help.rs` - LOG rows, `[ / ]`, `Driver Item States` legend
- `src/ui/screens/render_escape_guard.rs` - probe seeds every new site, plus a reach test

## Decisions Made

- **Park-rule map for plan 30-04:** `ProjectViewCache::launcher_log.park_rules` (`BTreeMap<String, String>`, item id to rule, rules only from `driver_run::PARK_RULES`). It is valid only while `launcher_log.run_id` equals the selected run id; `items_section` already passes it to the cursor-item row (`I-03 parked by needs-human: ...`). A reset (truncation/replacement) clears it; it holds at most 256 ids.
- **`heartbeat_stale` signature for plan 30-03:** `pub fn heartbeat_stale(fm: &DriverFrontmatter, derived: &DerivedStatus, now: SystemTime) -> bool` in `src/ui/screens/driver_run.rs`; companions `is_live(&DerivedStatus)`, `heartbeat_source`, `heartbeat_age_secs`, `format_age`, `control_shown`, `status_cell_at`/`run_row_at`.
- `[inferred — audit]` A failed single-run read triggers a full re-listing rather than building a `FrontmatterError` from the action's `String`: the re-listing yields the typed unreadable row.
- `[inferred — audit]` Item row spacing follows the UI-SPEC mock (one gap before the `hold` flag, two after), which is what makes the documented 27 fixed cells true; the id column is cut to 5 cells (`I-1000` would read `I-10…`).
- `[inferred — audit]` Cap at-limit is judged on the displayed whole numbers (minutes floored), so what is shown is what is judged; limit 0 is a full at-limit bar.
- `[inferred — audit]` Header row 3 sheds its context tags before its flags: the heartbeat narrows `Full` -> `Short` (drops `(launcher start)`/`(after ...)`) -> `Tiny` (`hb 3:10 stale`) and the control word is cut, so `stale` survives the 39-cell pane. The audit row narrows the same way so `MISMATCH` survives.
- `[inferred — audit]` `[not live]` is shown for any run whose derived status is not `AsWritten(Running | Starting)`.
- `[inferred — audit]` A tail is bounded to 4 reads x 4 MiB per refresh; a path at LOG.md that is not a regular file (a symlink, a directory, a FIFO) is not opened.
- `[inferred — audit]` A park rule is recorded only for item keys shaped `I-...` (at most 32 chars); `RUN` and arbitrary keys are ignored.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `status_cell`/`run_row` fixtures read stale against the real clock**
- **Found during:** Task 2
- **Issue:** the shared fixture file's heartbeats are fixed dates (2026-10-17); once `status_cell` judges staleness against `now`, every `running` fixture reads `◈ stale` and the 30-01 tests that assert `◆ running` fail.
- **Fix:** `refresh_heartbeats`/`stamp_ago` test helpers (`#[cfg(test)]`, `pub(crate)`), applied in the `driver_run`, `detail` and `app` fixtures; the fixture's `last_event.summary` (`status=crashed`) is neutralised where a test asserts a run is never called crashed. Production code unchanged.
- **Files modified:** `src/ui/screens/driver_run.rs`, `src/ui/screens/detail.rs`, `src/app.rs`
- **Committed in:** `26c10bc`

**2. [Rule 2 - Missing Critical] Watcher-supplied run id validated before it names a directory**
- **Found during:** Task 1 group C
- **Issue:** `schedule_selected_run_refresh` joins a run id taken from a watcher path onto the drivers directory.
- **Fix:** `is_plain_run_id` (the listing's own plain-component pattern) gates it; anything else schedules nothing.
- **Files modified:** `src/state_reader/driver_frontmatter.rs`, `src/ui/screens/mod.rs`
- **Committed in:** `9ed7761`

**3. [Rule 3 - Blocking] `output_header_line` hard-coded the word `output`**
- **Found during:** Task 3
- **Issue:** the plan reuses `output_header_line` for the LOG rule, which would read `── output ──`.
- **Fix:** `pane_header_line(name, ...)` with `output_header_line` as its `output` instance; `ring_overflow_notice` made `pub(super)`.
- **Files modified:** `src/ui/screens/driver.rs`
- **Committed in:** `ebd6957`

**4. [Rule 1 - Bug] rustfmt diffs in group A's `steer.rs`**
- **Found during:** Task 1 group B
- **Issue:** `cargo fmt --check` flagged three hunks in the file committed in group A.
- **Fix:** formatted in the group B commit (the only file group A touched that the repo's fmt gate flagged).
- **Committed in:** `dd6a0ad`

### Plan-vs-implementation notes (not defects)

- **Step 6 key handlers:** the plan asks `PgUp`/`PgDn`/`f`/`G` to act on the LOG pane "reusing `driver_scroll_offset`, `driver_follow` and `driver_viewport`". The shipped Driver arms already call `scroll_driver_output`/the `f`/`G` bodies against exactly those fields whatever row is selected, so no handler needed to change; the LOG renderer records `driver_viewport` and honours the follow bit. The new test `launcher_view_the_scroll_keys_drive_the_log_tail` proves the behaviour on a launcher row. `move_selection` also resets scroll/follow when the selected launcher run changes or the highlight leaves a legacy row.
- **`from_offset` in `src/app.rs`:** the applicability rule lives in `driver_run::apply_log_read` (pure, so unit-testable); the app arms call it and carry the comment, and `launcher_log_appends_arrive_once_through_the_refresh_actions` proves the stale-delivery discard at the action level, satisfying the plan's `from_offset` acceptance check.
- **`status_cell`/`run_row`** keep clock-reading wrappers beside the new `status_cell_at`/`run_row_at`.
- **No height tiers:** the stacked layout clips the lower sections on short panes and gives the LOG rule + 3 rows when there is room; no-orphan and tiering are plan 30-03.

---

**Total deviations:** 4 auto-fixed (1 bug, 1 missing critical, 2 blocking). **Impact on plan:** none on scope; fixtures, a security guard, a rule-reuse generalisation and a formatting hunk.

## Issues Encountered

None.

## Verification

- `mkdir -p -m700 /tmp/claude-1000/t; umask 077; TMPDIR=/tmp/claude-1000/t rtk proxy cargo test --no-fail-fast`: **3827 passed, 1 failed, 24 ignored** (baseline after 30-01: 3749 passed, 1 failed, 24 ignored; +78 tests). The one failure is the expected git-version witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- Per-task filters: Task 1 `driver::steer:: driver_frontmatter screens::driver_run:: launcher_view_ driver::direction::` pass (including `launcher_view_items_merge_body_and_frontmatter`); Task 2 `screens::driver_run:: launcher_view_ render_escape_guard app::tests::` 213 passed; Task 3 `screens::driver_run:: launcher_log_ launcher_view_ screens::detail:: screens::help:: render_escape_guard` 525 passed, 16 `launcher_log_*` tests.
- Acceptance greps hold: `pub fn read_items` 1, `Regex::new` in steer.rs 0, `fn heartbeat_stale` 1, `HEARTBEAT_STALE_AFTER_S` in driver_run.rs 0, `fn read_log_delta` 1, `tail_lines(` >= 1, `from_offset` in app.rs >= 1, `park_rules` in mod.rs >= 1, `Driver Item States` in help.rs >= 1, `launcher.log` in driver_run.rs 0.
- Cfg hygiene (RESEARCH Pitfall 13): `read_log_delta`'s `(dev, ino)` read is `#[cfg(unix)]` with a `None` arm elsewhere; `read_driver_run_text` keeps the reader's existing unix probe / non-unix `LivenessUnknown` split; `steer.rs`, `driver_run.rs` and the new action fields are platform-neutral. Reviewed by hand (only the Linux target is installed).
- Group commits: one hash per group A-C of Task 1 (`d0e469c`, `dd6a0ad`, `9ed7761`); no group was merged into a neighbour, and the group A formatting hunk landed in B (deviation 4).

## Known Stubs

None. `LOG_MIN_ROWS` is consumed by the composition; `ITEMS_MAX_ROWS` and `CONNECTED_MAX_ROWS` are consumed by their sections.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: file-access (residual TOCTOU) | src/ui/screens/driver_run.rs | `read_log_delta` checks `LOG.md` with `symlink_metadata` (regular file only) and then `tail_lines` opens it by path; a sibling run directory writable from a sandbox could swap a link in between. The worst outcome is that another regular file's lines are displayed as content (escaped, 4 MiB x 4 bound); no write, no liveness or status effect. Within T-30-05/T-30-07, noted for the 28-SECURITY follow-up. |

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Ready for 30-03: `heartbeat_stale` (above), `status_cell_at`/`run_row_at`, `header_rows`, `items_section`, `caps_section`, `connected_section`, `log_body_lines` and `render_log_section` are the section builders to tier by height; `detail_lines` is the stacked composition to replace, and `LOG_MIN_ROWS`/`CAP_ROW_FIXED_CELLS` are in place.
- Ready for 30-04: `launcher_log.park_rules` (above), `merged_items`/`launcher_item_cursor` (the cursor item for `a`/`u`), `BodyItem::is_on_hold`.
- 30-05 owns `s`, `R` and the retired `i` row (the help `i` row still reads as shipped).

## Self-Check: PASSED

- Created/modified files exist (12 of 12 checked); commit hashes `d0e469c dd6a0ad 9ed7761 26c10bc ebd6957` are ancestors of HEAD; `check evaluation-scope --plan 30-02 --commits-only` resolves with no missing, outside-union or unreachable entries; acceptance greps for all three tasks hold; no deletions outside the intended code (post-commit deletion check empty for every commit).

---
*Phase: 30-driver-tab-on-direction-md*
*Completed: 2026-10-21*
