---
phase: 30-driver-tab-on-direction-md
verified: 2026-10-21T06:00:00Z
status: human_needed
score: 6/6 must-haves verified
covered_files:
  - .planning/phases/30-driver-tab-on-direction-md/30-01-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-01-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-02-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-02-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-03-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-03-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-04-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-04-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-05-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-05-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-06-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-06-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-07-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-07-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-08-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-08-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-09-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-09-SUMMARY.md
  - src/app.rs
  - src/driver/direction.rs
  - src/driver/steer.rs
  - src/journal/reader.rs
  - src/main.rs
  - src/state_reader/driver_frontmatter.rs
  - src/ui/screens/confirm_popup.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/driver_launch.rs
  - src/ui/screens/driver_run.rs
  - src/ui/screens/driver_steer.rs
  - tests/driver_tab_launch_e2e.rs
  - tests/editor_command.rs
  - tests/launcher_config_dir.rs
covered_digest: "v3:sha256:a69c8d6624d5fe55230d44e3ae8530291b1323b2a6def42bf2d47870d9a6048a"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 5/6
  gaps_closed:
    - "D-15/D-22: the polite-stop confirmation shows 'Nothing is killed.' (closed by 30-06)"
  gaps_remaining: []
  regressions: []
gaps: []
deferred: []
human_verification:
  - test: "Read the DarkGray labels and the glyphs U+2016, U+25C8, U+2691 on the real terminal theme and font"
    expected: "Labels are legible; each glyph renders as a single cell"
    why_human: "Terminal theme and font are unobservable from a TestBackend"
  - test: "Judge the density of the FULL tier at 100x30"
    expected: "Header, items, caps, connected and log read as a coherent view"
    why_human: "Subjective layout quality"
---

# Phase 30: Driver Tab on DIRECTION.md Verification Report (re-verification)

**Phase Goal:** The TUI observes and steers driver runs only through the run's files. Status, caps, heartbeat and crash detection come from the tool-written frontmatter, and steering is an edit to DIRECTION.md.
**Verified:** 2026-10-21
**Status:** human_needed
**Re-verification:** Yes, after gap-closure plans 30-06..30-09

## Prior gap: closed

| Prior gap | Status | Evidence |
|-----------|--------|----------|
| D-15/D-22: polite-stop confirm clipped "Nothing is killed." | CLOSED | `wrapped_rows` (`src/ui/screens/confirm_popup.rs:194-199`) now calls `Paragraph::line_count` with `Wrap { trim: false }`, the same wrapper `render_confirm_popup` draws the body with (line 270), instead of `ceil(chars/width)`. `popup_rect` is sized from it (line 262). `driver_steer::launcher_steer_the_polite_stop_sentence_survives_every_width` (`driver_steer.rs:1201`) uses the 18-char run id and asserts the whole stripped prompt plus a blank spacer row at 50x24, 60x16, 80x12, 80x24, 100x30, 120x40 and 160x40. Ran it: passes. The prompt string at `driver_steer.rs:320` still ends "Nothing is killed. [y/n]". |

Defense in depth added by 30-06: a clipped popup disarms `y`, Enter-on-Yes and a Yes click (`confirm_popup.rs` `confirm_unless_clipped`, lines 112-125) and shows `CLIPPED_HINT`. The same sizing is reused by the launch form (`driver_launch.rs:943-966`, `fit_list`) and the edit-lint confirmation (`driver_steer.rs:879-897`), which resolves the earlier WR-05 and UI D5 findings.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: Driver tab lists run dirs; selected run shows status, items, caps, audit, heartbeat, `connected[]`, from frontmatter, no Claude call | VERIFIED | Unchanged since prior verification; regression check: lib tests pass, and no `Command::new` in `driver_run.rs`, `driver_steer.rs`, `driver_launch.rs`, `driver_confirm.rs`, `steer.rs`, `driver_frontmatter.rs` (grep rc=1). 30-07 added `open_run_file` (`driver_frontmatter.rs:786`), now used by `read_driver_run_text` (856), so the reads are also non-following and non-blocking. |
| 2 | SC2: `running` + free lock shown crashed; stale heartbeat flagged; dashboard marks driven/parked/crashed | VERIFIED | Unchanged code path (derive/derive_status, `launcher_markers_for`); regression tests pass. 30-07 adds the "Loading..." state so the FULL header no longer shows a false Yellow "(no control line)" while the body loads (`launcher_view_full_header_says_loading_until_the_detail_arrives`). |
| 3 | SC3: set control, answer/un-hold, open in `$EDITOR`; every edit through the shared flock, frontmatter untouched | VERIFIED | Only `write_body` closures write DIRECTION.md. 30-07 `Plan::Keep` (`direction.rs:534`, `616`) makes a no-op write leave inode, mtime and temp untouched, so "Nothing was written" is true. 30-08 `editor_command` (`app.rs:708`, consumed at `main.rs:849`) opens `$EDITOR` values with arguments through `sh -c '<editor> "$@"'` with the path as a separate argv element; `tests/editor_command.rs` 5 passed, including the hostile-path test. |
| 4 | SC4: start a new run, relaunch a crashed one, tail LOG.md live; injection UI not the steering path | VERIFIED | `tests/driver_tab_launch_e2e.rs` 2 passed. `read_log_delta` now tails through one checked handle (`driver_run.rs:1764`, `open_run_file`). 30-09: a refused new-run request closes the slug input; launch preflight uses the launcher's `control_line` (`driver_launch.rs:390`) and body-relative lint lines. |
| 5 | Status words come from the lock probe, never the frontmatter `status:` word alone | VERIFIED | Unchanged. |
| 6 | D-15/D-22: polite stop is distinguishable from kill; confirmation says "Nothing is killed." | VERIFIED | See "Prior gap: closed". |

**Score:** 6/6 truths verified.

### Required Artifacts and Key Links (gap-closure plans)

`verify.artifacts` for 30-06..30-09: all 14 artifacts exist with no issues. `verify.key-links` could not parse the plans' prose `from`/`to` fields ("Source file not found"), so links were checked by grep:

| Link | Status | Evidence |
|------|--------|----------|
| `render_confirm_popup` -> `ConfirmPopup::record` (clipped guard) | WIRED | `driver_steer.rs:375,897`, `driver_launch.rs:1036,1152,1525`, `driver_confirm.rs:515` all `.record(render_confirm_popup(..))` |
| `PoliteStopScreen::render` -> `wrapped_rows` sizing | WIRED | `driver_steer.rs:375` via `render_confirm_popup` |
| `open_run_file` -> `read_direction`, `apply_edit`, `read_log_delta`, `read_driver_run_text` | WIRED | `steer.rs:623,758`, `driver_run.rs:1764`, `driver_frontmatter.rs:856` |
| `write_body` -> `Plan::Keep` early return | WIRED | `direction.rs:534,616` |
| `main.rs` -> `editor_command` | WIRED | `main.rs:849` |
| `LaunchFormScreen::render` -> `body_room`/`fit_list`/`wrapped_rows` | WIRED | `driver_launch.rs:944-950` |
| `launch_preflight` -> `control_line`/`lint_direction` | WIRED | `driver_launch.rs:370,390` |
| Kill confirm -> `DetailScreen::render_main_only` | WIRED | `driver_confirm.rs:449` |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Gap-closure and steering/launch tests (`launcher_steer`, `confirm_popup`, `launcher_launch`, `launcher_new_run`, `open_run_file`, `write_body`, `apply_edit`, `read_log_delta`, `editor_command`) | `cargo test --lib -- <filters>` | 151 passed, 0 failed | PASS |
| New-run / relaunch e2e and editor stub tests | `cargo test --test driver_tab_launch_e2e --test editor_command` | 2 passed; 5 passed | PASS |
| Source changed since the orchestrator's full run? | `git diff --stat f2b53ba..HEAD -- src tests Cargo.toml` | empty (docs-only commits after f2b53ba) | PASS |
| Full suite (orchestrator evidence, f2b53ba) | `cargo test --no-fail-fast` | 4068 passed, 1 failed (`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the known machine-local git-version witness), 24 ignored | PASS (accepted) |
| clippy (orchestrator evidence) | `cargo clippy --all-targets -- -D warnings` | clean | PASS |

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| DTAB-01 | 30-01, 02, 03, 07 | SATISFIED | Truth 1 |
| DTAB-02 | 30-01, 02, 03, 07 | SATISFIED | Truth 2 |
| DTAB-03 | 30-04, 06, 07, 08, 09 | SATISFIED | Truths 3 and 6 |
| DTAB-04 | 30-02, 04, 05, 07, 09 | SATISFIED | Truth 4 |

All four IDs appear in plan frontmatter, are checked `[x]` in REQUIREMENTS.md (lines 233-245) and are "Complete" in the traceability table (365-368). No orphans.

### Anti-Patterns Found

No unreferenced TBD/FIXME/XXX markers in the files changed by the phase (the only `XXXX` hits are `\uXXXX` in doc comments).

Open review findings (`30-REVIEW-DISPOSITION.md`: 3 open, 1 deferred, 8 fixed). Judged against the goal:

| Finding | Location | Severity | Defeats a must-have? |
|---------|----------|----------|----------------------|
| WR-07 (open): `model_name_ok` rejects `[`/`]`, so the launch form refuses `opus[1m]`-style typed models; a recorded `opus[1m]` is not prefilled on relaunch and falls back to the default `opus` with `defaulted.model` still true (`driver_launch.rs:231-239,262-270,619`) | regression from 30-09 `9ad47bd` | Warning | No. SC4 (start a new run, relaunch a crashed one) holds: the model field has a working default (`opus`), accepts any alphanumeric-led name with `. _ : -`, and the CLI path is unaffected. The loss is a convenience (1M-context selector via the TUI form); a relaunch reads the default as an unedited, visibly "defaulted" value rather than silently trusting a recorded one. The reviewer's fix is one line plus one test. Recommend fixing in a follow-up. |
| WR-02 (open): `run_loop` in `src/driver/direction.rs:600,651` reads DIRECTION.md and the displaced inode by path (`std::fs::read`), so a FIFO planted as DIRECTION.md could block the `spawn_blocking` steer worker | pre-existing shared engine (also `driverctl`'s) | Warning | No. It needs a hostile file already planted in the user's own run dir, blocks only a worker thread (the render loop is unaffected), and writes nothing. SC3's flock and frontmatter-untouched guarantees are intact. Security audit: SECURED, `threats_open: 0`. |
| IN-04 (open): lint popup whose findings get zero rows is forced into the clipped, unconfirmable state | `confirm_popup.rs` | Info | No (fails safe: `n`/Esc still work, `y` is inert). |
| IN-05 (deferred): body-relative vs file-relative `DIRECTION.md:<line>` across tools | cross-tool | Info | No. |
| UI audit remaining (22/24, no BLOCK) | `driver_steer.rs:102,127,142,610` generic success copies about 90 chars cut at 62 cells at 80 columns; D7 popup background tier; un-ellipsised LOG clip | Warning/minor | No. The key facts are in the first 62 cells. |

### Human Verification Required

Only subjective or real-terminal items remain; everything checkable was settled from code and tests.

1. **DarkGray label contrast and glyph rendering** - Test: view the Driver tab in the user's terminal theme and font. Expected: labels legible, `‖ ◈ ⚑` render as single cells. Why human: the terminal theme and font are unobservable from a TestBackend.
2. **FULL-tier density at 100x30** - Test: open a populated run at 100x30. Expected: header, items, caps, connected and log read as a coherent view. Why human: subjective layout quality.

### Gaps Summary

The one prior gap (polite-stop confirmation clipping "Nothing is killed.") is closed, with the sizing fix, a clip guard that makes any cut-off confirmation unconfirmable, and a width-matrix test at an 18-character run id. No regressions: all four ROADMAP success criteria and all four requirement IDs remain met and wired, and no source has changed since the full-suite run. No gaps remain. Warnings WR-07 and WR-02 do not defeat any must-have or the phase goal; fixing WR-07 is recommended before the next release. The status is `human_needed` only because the two subjective, real-terminal items above cannot be settled without the user's terminal.

---

_Verified: 2026-10-21_
_Verifier: Claude (gsd-verifier)_
