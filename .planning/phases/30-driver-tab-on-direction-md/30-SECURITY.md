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
| T-30-01 | Spoofing | `driver_run::status_cell`, header rows | high | mitigate | status/liveness only from `DerivedStatus` (`src/ui/screens/driver_run.rs:199-248`, `src/state_reader/driver_frontmatter.rs:704-724`, `:822-825`); test `launcher_view_a_free_lock_reads_crashed_and_a_held_lock_reads_running` (`src/app.rs:5919`; re-checked 2026-10-21, `driver_run.rs:191-250`) | closed |
| T-30-02 | Tampering | run list rows, header rows, kill prompt | high | mitigate | `shown_capped` before `fit_cells` (`driver_run.rs:321`, `:863`, `:880`, `:897`); static glyphs (`:57-75`); kill prompt `render_for_terminal` (`src/ui/screens/driver_confirm.rs:347`); escape-guard states (`src/ui/screens/render_escape_guard.rs:1219-1278`, `:2564`) | closed |
| T-30-03 | Denial of Service | `classify_change` / `batch_actions` | medium | mitigate | allowlist (`src/journal/mod.rs:1181-1196`); `Ignored` dropped (`src/watcher.rs:74-76`); no `schedule_reparse` on `DriverRun` (`src/app.rs:2261-2298`) | closed |
| T-30-04 | Denial of Service | `read_driver_run` | medium | mitigate | `DIRECTION_READ_MAX_BYTES` + `take` (`driver_frontmatter.rs:770`, `:801-818`); symlink skip and cap 20 (`:882`, `:887`; `src/ui/screens/mod.rs:397`) | closed |
| T-30-05 | Tampering | LOG pane, item rows, header rows 3-6, connected rows | high | mitigate | `push_record` -> `Untrusted` (`driver_run.rs:1815`, `:1839`; `mod.rs:605`); render via `shown()` (`driver_run.rs:1912`); `shown_capped` at `:564`, `:1052-1127`, `:1452`, `:1475`, `:773-800`; escape-guard states (`render_escape_guard.rs:1228-1299`) | closed |
| T-30-06 | Spoofing | LOG styling, item and run glyphs | medium | mitigate | closed `ItemState` (`driver_run.rs:1418-1430`), `DerivedStatus` (`:489-516`), `PARK_RULES` closed set (`:1651`, `:1822`), structural LOG styling (`:1913-1927`) | closed |
| T-30-07 | Denial of Service | `read_log_delta`, LOG ring | medium | mitigate | `MAX_TAIL_BYTES` 4 MiB (`src/journal/mod.rs:132`, `src/journal/reader.rs:177`); 4 reads/refresh (`driver_run.rs:1668`, `:1767`); ring 2000 x 512 cells (`mod.rs:393`, `:410`, `:607`, `:766`); `from_offset` stale discard (`driver_run.rs:1853-1860`) | closed |
| T-30-08 | Spoofing | `launcher_marker` | low | accept | see Accepted Risks AR-30-01 | closed |
| T-30-09 | Denial of Service | 20-tick marker scan | medium | mitigate | existing blocking task and tick counter, flag-gated (`src/app.rs:2091-2143`); early return and 20-run cap (`mod.rs:2237`, `:2247-2250`) | closed |
| T-30-10 | Tampering | `steer_run`, `apply_edit` | high | mitigate | `write_body` only (`src/driver/steer.rs:548-576`, `:787`); DIRECTION.lock + RENAME_EXCHANGE verify (`src/driver/direction.rs:523-676`); base compare before and inside closure (`steer.rs:780-788`) | closed |
| T-30-11 | Tampering | `validate_answer`, `answer` | high | mitigate | refusal set (`steer.rs:319-337`), called before edit (`:510`); `check_item` inside closure (`:476-496`) | closed |
| T-30-12 | Tampering | `prepare_edit`, `apply_edit` | medium | mitigate | `create_new` + 0600 (`steer.rs:662-692`); regular-file and ≤1 MiB checks on the open handle via `open_run_file` (`:758-772`, strengthened by 30-07); bounded UTF-8 read (`:629-639`) | closed |
| T-30-13 | Repudiation | `write_body` lock-timeout fallback | low | accept | see Accepted Risks AR-30-02 | closed |
| T-30-14 | Elevation of Privilege | steering keys on legacy rows / non-Linux | medium | mitigate | `steer_refusal` (`src/ui/screens/detail.rs:5360`, `:5390`, `:5432`, `:5688`; `driver_run.rs:2502`, `:2630-2641`; `src/ui/screens/driver_steer.rs:43`); App guard (`app.rs:1575-1605`, `:731`, `:1356`); grep gate: no file writes in non-test `src/ui/**` | closed |
| T-30-15 | Elevation of Privilege | `LaunchFormScreen`, `UnsandboxedConfirmScreen` | high | mitigate | sole non-test `unsandboxed = true` at `src/ui/screens/driver_launch.rs:1120` (re-checked 2026-10-21); form forces false (`:729`); clip guard (T-30-21) also covers this popup; default No (`src/ui/screens/confirm_popup.rs:34-35`); tests `launcher_launch_u_then_n_dispatches_nothing`, `launcher_launch_u_then_y_launches_unsandboxed_once_and_remembers_nothing` | closed |
| T-30-16 | Denial of Service (wallet) | launch form values, spend copy | medium | mitigate | clap `parse_launch_field` (`driver_launch.rs:608-636`; `src/cli.rs:9-35`, `:551`, `:557`); `check_limits` (`src/launcher/run.rs:201`, `:555`, `:1223`); spend copy (`driver_launch.rs:916-927`) | closed |
| T-30-17 | Tampering | `relaunch_prefill` | medium | mitigate | caps re-parsed (`driver_launch.rs:272-289`); model charset gate `model_name_ok` (`:231-239`, `:267`; renamed from `recorded_model_ok`, identical body) | closed |
| T-30-18 | Tampering | `SlugInputScreen`, `create_run_dir` | medium | mitigate | `SLUG_RE` + 80-byte bound (`driver_launch.rs:1169-1181`; `src/driver_templates/mod.rs:145-154`); symlink/outside-project refusals (`:205-237`); `EEXIST` -> `Exists` (`:240-244`); `create_new` files (`:178-187`) | closed |
| T-30-19 | Denial of Service | run lifetime | medium | mitigate | grep gate: 0 hits for `RunStore`, `driverctl::serve`, `UnixListener` in `src/ui` and `src/app.rs`; only `launch_detached` seam (`app.rs:603`, `:1318`) | closed |
| T-30-20 | Information Disclosure / Tampering | `launch_error_copy` | low | mitigate | `capped_detail` 3 lines / 300 chars via `render_for_terminal` (`driver_launch.rs:49-51`, `:396-462`) | closed |
| T-30-21 | Spoofing | `confirm_popup::render_confirm_popup`, `ConfirmPopup::on_key` / `on_mouse` | medium | mitigate | `wrapped_rows` = `Paragraph::line_count` of the drawn wrap (`src/ui/screens/confirm_popup.rs:194-199`, `:241-267`); `clipped` (`:283`) shows `CLIPPED_HINT` (`:67`, `:284-285`); `confirm_unless_clipped` gates y / Enter-on-Yes / Yes click (`:112-118`, `:122`, `:125`, `:91`); used by every confirm screen; tests `a_clipped_body_disarms_yes_and_says_so`, `launcher_steer_a_clipped_polite_stop_writes_nothing_on_y` | closed |
| T-30-22 | Tampering | wrapped answer footer rows, D-16 status copy | low | mitigate | `render_for_terminal` before `Span` (`src/ui/screens/driver_steer.rs:542`, `:549`, `:68-75`); wrap after sanitising (`:577-581`); `steered_message` escapes (`:90`, `:99`, `:112`, `:123-124`); `AnswerScreen` fixture (`render_escape_guard.rs:369`, `:2475`) | closed |
| T-30-23 | Denial of Service | `read_driver_run_text`, `steer::read_direction`, `read_log_delta` | medium | mitigate | `open_run_file` `O_NOFOLLOW\|O_NONBLOCK`, refuses non-regular handles (`src/state_reader/driver_frontmatter.rs:786-825`); callers `:856`, `steer.rs:623`, `driver_run.rs:1764`, `steer.rs:758`; FIFO tests with 5 s `recv_timeout` (`open_run_file_refuses_a_fifo_without_blocking`, `read_driver_run_text_refuses_a_fifo_direction_without_blocking`, `read_direction_refuses_a_fifo_without_blocking`, `read_log_delta_skips_a_fifo_log_without_blocking`) | closed |
| T-30-24 | Tampering | `apply_edit` temp read, `read_log_delta` tail | medium | mitigate | single open, checks and reads on the handle (`steer.rs:753-772`; `driver_run.rs:1764-1808`, `(dev, ino)` at `:1775-1778`; `src/journal/reader.rs:159-186` `tail_file`); tests `launcher_edit_a_temp_that_is_a_symlink_is_refused`, `tail_file_reads_from_an_open_handle` | closed |
| T-30-25 | Tampering | `write_body` no-op rewrites | low | mitigate | `Plan::Keep` (`src/driver/direction.rs:532-535`), early `Ok` before temp/exchange (`:616`, `:630`, `:637`); tests `write_body_leaves_the_file_alone_when_the_body_is_unchanged`, `steer_run_unchanged_and_refused_leave_the_inode_alone`, `apply_edit_conflict_leaves_the_inode_alone` | closed |
| T-30-26 | Tampering | `set_item_field` on an unterminated last heading | low | mitigate | `terminated` check and leading eol (`steer.rs:454-455`, `:465-469`); `check_item` (`:480-500`); test `answer_on_an_unterminated_last_heading_writes_a_line_of_its_own` | closed |
| T-30-27 | Elevation of Privilege | `app::editor_command` `sh -c` form | high | mitigate | script is exactly `{editor} "$@"`, path/line only as argv after `$0` (`src/app.rs:719-727`); sole spawn `src/main.rs:849-850`; tests `editor_command_never_puts_the_path_in_the_shell_script` (`app.rs:4136`), `a_hostile_path_reaches_the_editor_as_one_literal_argument` (`tests/editor_command.rs:178`) | closed |
| T-30-28 | Elevation of Privilege | `$VISUAL` / `$EDITOR` interpreted by `sh` | low | accept | see Accepted Risks AR-30-04 | closed |
| T-30-29 | Spoofing | `LaunchFormScreen` body on short terminals | medium | mitigate | `essential_lines` puts spend/detach/sandbox first (`driver_launch.rs:785-907`, `:793-835`); `fitted_body_lines` (`:943-961`); clip guard keeps `y` inert (`:1019`); tests `launcher_launch_the_spend_and_sandbox_rows_outlive_the_findings` (80x18), `..._a_degraded_host_at_80x24_...`, `..._a_form_cut_off_at_80x12_launches_nothing` | closed |
| T-30-30 | Tampering | `commit_edit` model value | medium | mitigate | `model_name_ok` (`driver_launch.rs:231-239`) + clap on typed path (`:619`, `:622`) and prefill (`:262-269`); only writers of `form.args.model`; tests `launcher_launch_a_typed_model_that_looks_like_a_flag_is_refused`, `..._prefill_distrusts_a_hostile_model_and_a_forged_cap` | closed |
| T-30-31 | Spoofing | `launch_preflight` control verdict | low | mitigate | uses launcher's `driver_templates::lint::control_line` (`driver_launch.rs:368-397`, `:390`; `src/launcher/run.rs:82`, `:246`); exact `!= "run"` (`:474-476`); test `launcher_launch_preflight_reads_control_like_the_launcher` | closed |
| T-30-SC | Tampering | npm/pip/cargo installs | high | accept | see Accepted Risks AR-30-03; `Cargo.toml`/`Cargo.lock` unchanged across the phase incl. 30-06..30-09 | closed |

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
3. Residual from 30-07-SUMMARY (outside the declared register): `src/driver/direction.rs:601` `run_loop` still reads DIRECTION.md by path with `std::fs::read`; the steering path (`app.rs:1724` -> `steer::steer_run` -> `write_body` -> `run_loop`) and driverctl's splice have no `open_run_file` pre-check. A FIFO planted by an `--unsandboxed` run could park one blocking-pool worker if the user steers that run; not a TUI hang. Medium at most, below `block_on: high`; the Driver-tab reads already refuse a FIFO first. Candidate for a later `direction.rs` hardening plan. `[inferred — audit]` recorded, not opened as a threat.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-30-01 | T-30-08 | An `--unsandboxed` driver could forge a sibling's frontmatter to show a false parked marker; liveness/crash still come only from the lock probe (`driver_run.rs:489-516`); sibling run dirs tmpfs-masked in the sandbox (`src/launcher/sandbox.rs:13-14`, `:258-259`, quick 261020-f80). | 30-03-PLAN threat model `[inferred — audit]` | 2026-10-20 |
| AR-30-02 | T-30-13 | Phase 27 D-20: after 2 s the write proceeds without DIRECTION.lock; exchange verify still prevents loss; `lock_timed_out` surfaced (`src/driver/direction.rs:76`, `:549-553`; `src/ui/screens/driver_steer.rs:58`, `:590-594`). | 30-04-PLAN threat model `[inferred — audit]` | 2026-10-20 |
| AR-30-03 | T-30-SC | No package added; `Cargo.toml`/`Cargo.lock` unchanged since 258053d (before phase), re-checked through e485c07 (30-06..30-09). | 30-0x-PLAN threat models `[inferred — audit]` | 2026-10-20 |
| AR-30-04 | T-30-28 | `$VISUAL`/`$EDITOR` is interpreted by `sh` only when it holds shell-special characters; it is the user's own environment (`src/main.rs:842-844`), already executed as a program before this phase, and git treats it the same way. The path is never in the script (T-30-27). | 30-08-PLAN threat model `[inferred — audit]` | 2026-10-21 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-20 | 21 | 21 | 0 | gsd-security-auditor (sonnet), /gsd-secure-phase 30; 143 lib tests across 13 security filters green |
| 2026-10-21 | 32 | 32 | 0 | gsd-security-auditor (sonnet), /gsd-secure-phase 30 re-run after gap-closure 30-06..30-09; 11 new threats closed, 21 existing regression-checked (none weakened); 17 + 189 lib tests and 5 `tests/editor_command.rs` tests green |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-20; re-verified 2026-10-21 (30-06..30-09)
