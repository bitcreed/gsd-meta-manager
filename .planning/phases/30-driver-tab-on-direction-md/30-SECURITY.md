---
phase: "30"
slug: "driver-tab-on-direction-md"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
created: "2026-10-20"
---

# Phase 30 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| run dir files -> TUI | DIRECTION.md frontmatter and run-dir names written by driverctl, the human, or (for an `--unsandboxed` run) another run's driver | untrusted display text |
| filesystem events -> App | notify events for paths under `.planning/meta-manager/drivers/` | event paths (DoS surface) |
| DIRECTION body / frontmatter -> TUI | titles, answers, outcomes, connection fields, control word | untrusted display text |
| LOG.md -> TUI | sandbox-originated fields reach LOG.md verbatim; file may be truncated or replaced | untrusted display text, unbounded size |
| every registered project's run dirs -> dashboard | marker scan reads up to 20 DIRECTION.md per project and probes their locks | untrusted frontmatter, I/O cost |
| user input -> DIRECTION.md body | answers, control words and editor saves become the plan the driver follows | plan text (integrity) |
| tab writes <-> driverctl splice / human editor saves | concurrent writers of the same file | file integrity |
| run dir -> edit temp | temp lives in a directory an `--unsandboxed` run could write | file path (symlink/TOCTOU) |
| TUI -> launcher library -> detached daemon | confirmed form becomes a `driver launch` daemon that spends money | launch args, sandbox flag |
| run frontmatter -> launch form prefill | recorded caps and model prefilled for confirmation | untrusted numbers / model string |
| slug input -> filesystem | slug becomes a directory under `.planning/meta-manager/drivers/` | path component |
| popup body -> user decision | the text a confirmation shows is the basis for a destructive or costly `y` (30-06) | display text |
| run dir files -> blocking readers | DIRECTION.md, LOG.md and the edit temp may be FIFOs/symlinks planted by an `--unsandboxed` run (30-07) | file handles |
| run dir name -> editor argv | the edit temp path contains a run id an attacker can choose (30-08) | argv element |
| user environment -> shell | `$VISUAL` / `$EDITOR` is the user's own configuration (30-08) | shell script |
| typed form values -> launcher argv | the model string becomes a separate argv word in `launch_argv` (30-09) | model string |
| DIRECTION.md -> launch preflight | `control` word and lint findings from a file a sibling may write (30-09) | control verdict |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-30-01 | Spoofing | `driver_run::status_cell`, header rows | high | mitigate | status/liveness only from `derive_status` / `DerivedStatus` (`src/state_reader/driver_frontmatter.rs:712`, `:795`) via `status_cell_at` (`src/ui/screens/driver_run.rs:187-250`); tests `launcher_view_a_free_lock_reads_crashed_and_a_held_lock_reads_running` (`src/app.rs:4142`), `status_cell_follows_the_ui_spec_table_for_every_derived_status` (`driver_run.rs:3300`) (re-checked 2026-10-28) | closed |
| T-30-02 | Tampering | run list rows, header rows, kill prompt | high | mitigate | `shown_capped` before `fit_cells` (`driver_run.rs:317`, `:859`, `:876`, `:883`, `:893`); static glyphs (`:56-75`); kill prompt `render_for_terminal` (`src/ui/screens/driver_confirm.rs:115`, `:124`); escape-guard fixtures (`src/ui/screens/render_escape_guard.rs`, 18 tests green 2026-10-28) | closed |
| T-30-03 | Denial of Service | `classify_change` / `batch_actions` | medium | mitigate | allowlist `classify_change` (`src/journal/mod.rs:342-368`); `Ignored` dropped (`src/watcher.rs:75`); `DriverRun` arm refreshes the selected run / list only, no `schedule_reparse` (`src/app.rs:1914-1935`; `schedule_reparse` call only at `:1974` on the STATE arm) | closed |
| T-30-04 | Denial of Service | `read_driver_run` | medium | mitigate | `DIRECTION_READ_MAX_BYTES` + `take` (`driver_frontmatter.rs:814`, `:893-901`, `:940-950`); symlink skip (`:1006`, test `:1128`) and cap `LAUNCHER_RUNS_LISTED` 20 (`src/ui/screens/mod.rs:391`); cap extended to launcher/CLI reads by dc812c0 (28 WR-03) | closed |
| T-30-05 | Tampering | LOG pane, item rows, header rows 3-6, connected rows | high | mitigate | `push_record` -> `Untrusted` (`src/ui/screens/mod.rs:484-496`, `:587`); render via `shown()` (`driver_run.rs:2045`); `shown_capped` on header/item/connected rows (`driver_run.rs:560`, `:729`, `:769-796`, `:1067-1208`); escape-guard fixtures green | closed |
| T-30-06 | Spoofing | LOG styling, item and run glyphs | medium | mitigate | closed `ItemState`, `DerivedStatus`, `PARK_RULES` closed set (`driver_run.rs:1779`), structural LOG styling; `launcher_marker` reduces lock-probed status only (`driver_run.rs:521`) | closed |
| T-30-07 | Denial of Service | `read_log_delta`, LOG ring | medium | mitigate | `MAX_TAIL_BYTES` 4 MiB (`src/journal/mod.rs:53`, `src/journal/reader.rs:143`, `:168`); `read_log_delta` single bounded open (`driver_run.rs:1852-1886`); ring 2000 x 512 cells (`mod.rs:387`, `:404`, record cap `:415`); `from_offset` stale discard (`src/app.rs:2516`, test `:4299-4339`) | closed |
| T-30-08 | Spoofing | `launcher_marker` | low | accept | see Accepted Risks AR-30-01 | closed |
| T-30-09 | Denial of Service | 20-tick marker scan | medium | mitigate | existing blocking task on the 20-tick counter, flag-gated (`src/app.rs:1790-1805`); `launcher_markers_for` bounded by `LAUNCHER_RUNS_LISTED` and the 1 MiB read (`app.rs:145-170`) (re-anchored 2026-10-28 after 31-03 moved the scan from `mod.rs`) | closed |
| T-30-10 | Tampering | `steer_run`, `apply_edit` | high | mitigate | `write_body` only (`src/driver/steer.rs:555` `steer_run`, `:753` `apply_edit`); DIRECTION.lock + RENAME_EXCHANGE verify (`src/driver/direction.rs:655` `locked`, `:807` `run_loop`); newest-wins `give_back_newest` bounded at `GIVE_BACK_ROUNDS` 8 (`:556`, `:737-802`); absent-file give-back restores by no-replace rename (44eef32, 30 WR-10); DIRECTION.lock opened `O_NOFOLLOW\|O_NONBLOCK` (`src/driver/lock.rs:170-174`, 670e099, 30 WR-08); write capped at 1 MiB (`direction.rs:843`, 0f85617, 30 WR-09); tests `a_save_landing_right_after_the_exchange_back_is_not_reverted`, `a_newer_save_found_at_the_exchange_back_is_put_in_place`, `a_save_landing_before_the_exchange_is_given_back_and_built_on` (`direction.rs:1387`, `:1414`, `:1431`), `steer_run_with_a_symlinked_direction_lock_fails_and_leaves_the_target_alone` (`steer.rs:1878`) (re-checked 2026-10-28) | closed |
| T-30-11 | Tampering | `validate_answer`, `answer` | high | mitigate | refusal set `validate_answer` (`steer.rs:319`), called from `answer` before the edit (`:513`); `check_item` inside closure (`:480`) | closed |
| T-30-12 | Tampering | `prepare_edit`, `apply_edit` | medium | mitigate | `create_new` + 0600 (`steer.rs:662-674`); regular-file and ≤1 MiB checks on the open handle via `open_run_file` (`:753-758`); bounded UTF-8 read | closed |
| T-30-13 | Repudiation | `write_body` lock-timeout fallback | low | accept | see Accepted Risks AR-30-02 | closed |
| T-30-14 | Elevation of Privilege | steering keys on legacy rows / non-Linux | medium | mitigate | `steer_refusal` (`src/ui/screens/driver_run.rs:2747`) gated on `can_steer()` (`src/ui/screens/driver_steer.rs:48-50`, true exactly where the exchange primitive exists: Linux `renameat2`, macOS `renameatx_np`, `direction.rs:58-59`, `:416-452`); callers `src/ui/screens/detail.rs:5714`, `:5744`, `:5786`, `:6003`; App arms `cfg(any(linux, macos))` (`src/app.rs:1129`, `:1172`, `:2253-2255`); legacy rows deleted in 31-03 (77908eb), so only launcher runs reach the guard; grep gate: no file writes in non-test `src/ui/**` (0 hits 2026-10-28) `[inferred — cleanup 2026-10-28]` macOS enablement 76a9ecb keeps the threat's intent (no steer without the atomic exchange) | closed |
| T-30-15 | Elevation of Privilege | `LaunchFormScreen`, `UnsandboxedConfirmScreen` | high | mitigate | sole non-test `unsandboxed = true` at `src/ui/screens/driver_launch.rs:1180` (re-checked 2026-10-28); form forces false (`:571`); clip guard (T-30-21) also covers this popup; default No (`src/ui/screens/confirm_popup.rs:35-39`); tests `launcher_launch_u_then_n_dispatches_nothing` (`driver_launch.rs:2279`), `launcher_launch_u_then_y_launches_unsandboxed_once_and_remembers_nothing` (`:2295`) | closed |
| T-30-16 | Denial of Service (wallet) | launch form values, spend copy | medium | mitigate | clap `parse_launch_field` (`driver_launch.rs:643-649`; `src/cli.rs:368`); `check_limits` (`src/launcher/run.rs:260`, `:635`, `:1462`); spend copy in `essential_lines` (`driver_launch.rs:810`) | closed |
| T-30-17 | Tampering | `relaunch_prefill` | medium | mitigate | caps re-parsed (`driver_launch.rs:275-287`); model charset gate `model_name_ok` (`:232`, `:269`) | closed |
| T-30-18 | Tampering | `SlugInputScreen`, `create_run_dir` | medium | mitigate | `SLUG_RE` + 80-byte bound (`src/driver_templates/mod.rs:48`, `:113`, `:144`; `driver_launch.rs:1243`); symlink/outside-project refusals (`mod.rs:128`, `:164`, `:206`); `AlreadyExists` -> `Exists` (`:242`); `create_new` files (`:181`) | closed |
| T-30-19 | Denial of Service | run lifetime | medium | mitigate | grep gate: 0 hits for `RunStore`, `driverctl::serve`, `UnixListener` in `src/ui` and `src/app.rs` (2026-10-28); only `launch_detached` seam (`app.rs:499`; `src/launcher/run.rs:1450`) | closed |
| T-30-20 | Information Disclosure / Tampering | `launch_error_copy` | low | mitigate | `capped_detail` 3 lines / 300 chars via `render_for_terminal` (`driver_launch.rs:432`, `:460`) | closed |
| T-30-21 | Spoofing | `confirm_popup::render_confirm_popup`, `ConfirmPopup::on_key` / `on_mouse` | medium | mitigate | `wrapped_rows` = `Paragraph::line_count` of the drawn wrap (`src/ui/screens/confirm_popup.rs:194`, `:233`); `clipped` (`:106`) shows `CLIPPED_HINT` (`:67`, `:285`); `confirm_unless_clipped` gates y / Enter-on-Yes / Yes click (`:112`, `:122`, `:125`, `:91`); used by every confirm screen; tests `a_clipped_body_disarms_yes_and_says_so` (`:511`), `launcher_steer_a_clipped_polite_stop_writes_nothing_on_y` (`driver_steer.rs:1705`) | closed |
| T-30-22 | Tampering | wrapped answer footer rows, D-16 status copy | low | mitigate | `render_for_terminal` before `Span` (`src/ui/screens/driver_steer.rs:547`, `:554`, `:77-78`); `steered_message` escapes (`:89-95`); `AnswerScreen` fixture (`render_escape_guard.rs:365`) | closed |
| T-30-23 | Denial of Service | `read_driver_run_text`, `steer::read_direction`, `read_log_delta` | medium | mitigate | `open_run_file` `O_NOFOLLOW\|O_NONBLOCK`, refuses non-regular handles (`src/state_reader/driver_frontmatter.rs:822-830`); callers `read_driver_run_text` (`:929`), `steer.rs:623`, `:758`, `driver_run.rs:1872`, `direction.rs:604` `read_run_file`; FIFO tests `open_run_file_refuses_a_fifo_without_blocking` (`driver_frontmatter.rs:1231`), `read_driver_run_text_refuses_a_fifo_direction_without_blocking` (`:1306`), `read_direction_refuses_a_fifo_without_blocking` (`steer.rs:1836`), `read_log_delta_skips_a_fifo_log_without_blocking` (`driver_run.rs:5880`) | closed |
| T-30-24 | Tampering | `apply_edit` temp read, `read_log_delta` tail | medium | mitigate | single open, checks and reads on the handle (`steer.rs:753-758`; `driver_run.rs:1868-1886`, `(dev, ino)` at `:1886`; `src/journal/reader.rs` `tail_file`); tests `launcher_edit_a_temp_that_is_a_symlink_is_refused` (`steer.rs:1936`), `tail_file_reads_from_an_open_handle` (`reader.rs:212`) | closed |
| T-30-25 | Tampering | `write_body` no-op rewrites | low | mitigate | `Plan::Keep` (`src/driver/direction.rs:641`), early `Ok` before temp/exchange (`:833`; re-anchored 2026-10-28); tests `write_body_leaves_the_file_alone_when_the_body_is_unchanged` (`direction.rs:1475`), `steer_run_unchanged_and_refused_leave_the_inode_alone` (`steer.rs:1113`), `apply_edit_conflict_leaves_the_inode_alone` (`steer.rs:1159`) | closed |
| T-30-26 | Tampering | `set_item_field` on an unterminated last heading | low | mitigate | `terminated` check and leading eol in `set_item_field` (`steer.rs:434`, `:454-465`); `check_item` (`:480`); test `answer_on_an_unterminated_last_heading_writes_a_line_of_its_own` (`steer.rs:1182`) | closed |
| T-30-27 | Elevation of Privilege | `app::editor_command` `sh -c` form | high | mitigate | script is exactly `{editor} "$@"`, path/line only as argv after `$0` (`src/app.rs:604-618`); sole spawn `src/main.rs:596-597`; tests `editor_command_never_puts_the_path_in_the_shell_script` (`app.rs:2979`), `a_hostile_path_reaches_the_editor_as_one_literal_argument` (`tests/editor_command.rs:178`) | closed |
| T-30-28 | Elevation of Privilege | `$VISUAL` / `$EDITOR` interpreted by `sh` | low | accept | see Accepted Risks AR-30-04 | closed |
| T-30-29 | Spoofing | `LaunchFormScreen` body on short terminals | medium | mitigate | `essential_lines` puts spend/detach/sandbox first (`driver_launch.rs:810`); `fitted_body_lines` (`:985`); clip guard keeps `y` inert; tests `launcher_launch_the_spend_and_sandbox_rows_outlive_the_findings` (`:2739`, 80x18) and the 80x24 / 80x12 siblings | closed |
| T-30-30 | Tampering | `commit_edit` model value | medium | mitigate | `model_name_ok` (`driver_launch.rs:232`) + clap on typed path (`:643`) and prefill (`:269`); only writers of `form.args.model`; tests `launcher_launch_a_typed_model_that_looks_like_a_flag_is_refused` (`:2912`), `..._prefill_distrusts_a_hostile_model_and_a_forged_cap` | closed |
| T-30-31 | Spoofing | `launch_preflight` control verdict | low | mitigate | uses launcher's `driver_templates::lint::control_line` (`driver_launch.rs:380-414`; `src/launcher/run.rs:91`, `:313`); exact `!= "run"` (`driver_launch.rs:499`); test `launcher_launch_preflight_reads_control_like_the_launcher` (`:2958`) | closed |
| T-30-SC | Tampering | npm/pip/cargo installs | high | accept | see Accepted Risks AR-30-03; no package added by any phase-30 plan (dependency changes since e485c07 belong to 28.1/31 and the v1.15.0 release, not phase 30) | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

### Summary Threat Flags (all map to registered threats)

| Flag | Source | Maps to | Note |
|------|--------|---------|------|
| residual TOCTOU in `read_log_delta` | 30-02-SUMMARY, `src/ui/screens/driver_run.rs` | T-30-05, T-30-07 | worst case: another regular file's lines displayed, escaped and bounded |
| event-driven marker scan, no in-flight guard | 30-03-SUMMARY, `src/ui/screens/mod.rs` | T-30-09 | bounded by 200 ms debounce and flag gate |
| residual TOCTOU in `apply_edit` | 30-04-SUMMARY, `src/driver/steer.rs` | T-30-12 | user sees lint / unchanged-body result first; tool block never written |
| `request_launch` new caller of `launch_detached` | 30-05-SUMMARY, `src/app.rs` | T-30-15, T-30-16, T-30-19 | — |
| new `sh` route for `$EDITOR` with arguments | 30-08-SUMMARY, `src/app.rs` | T-30-27, T-30-28 | planned surface, covered by tests |

### Auditor Notes (informational, non-blocking)

1. T-30-01/T-30-06 wording vs code: three styling decisions compare closed frontmatter literals (`sandbox == "degraded"` at `driver_run.rs:1042`, `custom:` prefix at `:1064`, `- hold: true` at `:1462`), and the `stale` glyph uses heartbeat age only while the lock is held (`:427-432`). None lets free text pick a glyph/colour or produce `crashed`/`running`. The module doc at `driver_run.rs:8-9` ("heartbeat freshness picks no glyph") is stale against the D-08 stale rule — documentation drift only.
2. Follow-up for 28-SECURITY (outside the declared register): `read_driver_run_text`, `read_direction`, `apply_edit`'s `File::open` and `tail_lines` open by path with no regular-file / non-blocking guard. Beyond the flagged TOCTOU windows, a FIFO planted in a run dir would block one blocking-pool worker. Only an `--unsandboxed` run's own driver (already full user permissions) can do this; sibling run dirs are tmpfs-masked in the sandbox. `[inferred — audit]` recorded, not opened as a threat. **Resolved 2026-10-21** for all four named sites by 30-07 (`open_run_file` / `tail_file`, T-30-23, T-30-24).
3. Residual from 30-07-SUMMARY (outside the declared register): `src/driver/direction.rs:601` `run_loop` still reads DIRECTION.md by path with `std::fs::read`; the steering path (`app.rs:1724` -> `steer::steer_run` -> `write_body` -> `run_loop`) and driverctl's splice have no `open_run_file` pre-check. A FIFO planted by an `--unsandboxed` run could park one blocking-pool worker if the user steers that run; not a TUI hang. Medium at most, below `block_on: high`; the Driver-tab reads already refuse a FIFO first. Candidate for a later `direction.rs` hardening plan. `[inferred — audit]` recorded, not opened as a threat. **Resolved 2026-10-21** by quick 261021-101 (64f73e7): `run_loop` reads through `read_run_file` -> `open_run_file` (`direction.rs:588-593`, `:723`); the 717d724 give-back path reads the temp name the same way (`seen`, `:558-565`).
4. 2026-10-24 re-audit after 717d724 (quick 261023-v1s, `src/driver/direction.rs` exchange-verify loop): the plain `rename` give-back in the newest-wins branch could revert a third rename-save landing at that instant (an integrity loss under T-30-10). It is replaced by `give_back_newest`: RENAME_EXCHANGE repeated while what comes back is not, by `(dev, ino, bytes)`, the older displaced file; after 8 unsettled rounds it returns an error and leaves the human bytes at the temp name. Net effect: T-30-10 strengthened, no registered mitigation weakened; the new `race_hook` seam is `#[cfg(test)]`-only. Other post-2026-10-21 commits touching cited files (39dad19, 98c20c1, 91b19ff, bf36f2d, d1b1457, 0fd27fa, 699ae75, 450713f) are test fixes, workstream features outside this phase's surface, crate-exclude anchoring and the v1.15.0 `cargo update`; AR-30-03's "Cargo.lock unchanged" holds for the phase itself, the later lock refresh is release-process, not a phase-30 install. `[inferred — audit]`

5. 2026-10-28 re-audit at 7902043 after Phase 28.1, Phase 31 deletions and cleanup batch 261028-n0u (n0z: 30 WR-08/09/10; n12: 30 IN-06/08/09). Every registered mitigation was re-located at HEAD and its named tests re-run. Changes to cited surfaces: (a) WR-08 670e099 opens DIRECTION.lock / launcher.lock `O_NOFOLLOW|O_NONBLOCK` (`src/driver/lock.rs:170-174`), WR-09 0f85617 caps the `run_loop` read and write at 1 MiB (`direction.rs:604-614`, `:843`), WR-10 44eef32 restores a given-back save by no-replace rename when DIRECTION.md is absent: all three strengthen T-30-10 / T-30-23, none weakens a mitigation; (b) 76a9ecb / de20ec3 (28 WR-05) turn on steering and launch on macOS: `can_steer()` is now true on Linux and macOS, exactly the targets with the atomic exchange (`renameatx_np` swap; HFS+ refuses cleanly), so T-30-14's intent holds; (c) 31-03 deleted the legacy run rows and moved the marker scan into `app::launcher_markers_for`: T-30-09 / T-30-14 re-anchored, legacy-row half of T-30-14 now vacuous; (d) dc812c0 (28 WR-03) routes launcher/CLI DIRECTION.md reads through `open_run_file` with the 1 MiB cap, widening T-30-04 / T-30-23 coverage. `[inferred — cleanup 2026-10-28]`

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-30-01 | T-30-08 | An `--unsandboxed` driver could forge a sibling's frontmatter to show a false parked marker; liveness/crash still come only from the lock probe (`driver_run.rs:489-516`); sibling run dirs tmpfs-masked in the sandbox (`src/launcher/sandbox.rs:13-14`, `:258-259`, quick 261020-f80). | 30-03-PLAN threat model `[inferred — audit]` | 2026-10-20 |
| AR-30-02 | T-30-13 | Phase 27 D-20: after 2 s the write proceeds without DIRECTION.lock; exchange verify still prevents loss; `lock_timed_out` surfaced (`src/driver/direction.rs:359`, `:666`, re-anchored 2026-10-28; `src/ui/screens/driver_steer.rs:58`, `:590-594`). | 30-04-PLAN threat model `[inferred — audit]` | 2026-10-20 |
| AR-30-03 | T-30-SC | No package added; `Cargo.toml`/`Cargo.lock` unchanged since 258053d (before phase), re-checked through e485c07 (30-06..30-09). | 30-0x-PLAN threat models `[inferred — audit]` | 2026-10-20 |
| AR-30-04 | T-30-28 | `$VISUAL`/`$EDITOR` is interpreted by `sh` only when it holds shell-special characters; it is the user's own environment (`src/main.rs:842-844`), already executed as a program before this phase, and git treats it the same way. The path is never in the script (T-30-27). | 30-08-PLAN threat model `[inferred — audit]` | 2026-10-21 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-20 | 21 | 21 | 0 | gsd-security-auditor (sonnet), /gsd-secure-phase 30; 143 lib tests across 13 security filters green |
| 2026-10-21 | 32 | 32 | 0 | gsd-security-auditor (sonnet), /gsd-secure-phase 30 re-run after gap-closure 30-06..30-09; 11 new threats closed, 21 existing regression-checked (none weakened); 17 + 189 lib tests and 5 `tests/editor_command.rs` tests green |
| 2026-10-24 | 32 | 32 | 0 | /gsd-secure-phase 30 (L1 short-circuit, ASVS 1) re-run after 717d724 changed `src/driver/direction.rs`; T-30-10 strengthened, T-30-25 / AR-30-02 re-anchored, auditor note 3 resolved; 334 `driver::` lib tests, 66 steer/write_body/open_run_file filter tests and `tests/driverctl_race.rs` (4 passed, 1 ignored) green |
| 2026-10-28 | 32 | 32 | 0 | /gsd-secure-phase 30 (L1 short-circuit, ASVS 1) re-run at 7902043 after 28.1, 31 deletions and 261028-n0u (30 WR-08/09/10); all 32 rows re-anchored, T-30-10/T-30-23 strengthened, T-30-14 re-scoped to Linux+macOS (auditor note 5); lib filters launcher_ 272, steer_ 62, model 23, answer_ 19, render_escape_guard 18, confirm_popup 12 + 13 smaller filters, `tests/editor_command.rs` 5, `tests/driverctl_race.rs` 4 (+1 ignored) green, 0 failed |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-20; re-verified 2026-10-21 (30-06..30-09); re-verified 2026-10-24 (717d724); re-verified 2026-10-28 (HEAD 7902043, after 28.1 / 31 / 261028-n0u)

## Security Audit 2026-10-28

| Metric | Count |
|---|---|
| Threats found | 32 |
| Closed | 32 |
| Open | 0 |
