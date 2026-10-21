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

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-30-01 | Spoofing | `driver_run::status_cell`, header rows | high | mitigate | status/liveness only from `DerivedStatus` (`src/ui/screens/driver_run.rs:199-248`, `src/state_reader/driver_frontmatter.rs:704-724`, `:822-825`); test `launcher_view_a_free_lock_reads_crashed_and_a_held_lock_reads_running` (`src/app.rs:5691`) | closed |
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
| T-30-12 | Tampering | `prepare_edit`, `apply_edit` | medium | mitigate | `create_new` + 0600 (`steer.rs:661-674`); `symlink_metadata` regular file ≤1 MiB (`:745-754`); bounded UTF-8 read (`:621-631`) | closed |
| T-30-13 | Repudiation | `write_body` lock-timeout fallback | low | accept | see Accepted Risks AR-30-02 | closed |
| T-30-14 | Elevation of Privilege | steering keys on legacy rows / non-Linux | medium | mitigate | `steer_refusal` (`src/ui/screens/detail.rs:5360`, `:5390`, `:5432`, `:5688`; `driver_run.rs:2502`, `:2630-2641`; `src/ui/screens/driver_steer.rs:43`); App guard (`app.rs:1575-1605`, `:731`, `:1356`); grep gate: no file writes in non-test `src/ui/**` | closed |
| T-30-15 | Elevation of Privilege | `LaunchFormScreen`, `UnsandboxedConfirmScreen` | high | mitigate | sole `unsandboxed = true` at `src/ui/screens/driver_launch.rs:1090`; form forces false (`:541`, `:725`); default No (`src/ui/screens/confirm_popup.rs:34-35`); tests `launcher_launch_u_then_n_dispatches_nothing`, `launcher_launch_u_then_y_launches_unsandboxed_once_and_remembers_nothing` | closed |
| T-30-16 | Denial of Service (wallet) | launch form values, spend copy | medium | mitigate | clap `parse_launch_field` (`driver_launch.rs:608-636`; `src/cli.rs:9-35`, `:551`, `:557`); `check_limits` (`src/launcher/run.rs:201`, `:555`, `:1223`); spend copy (`driver_launch.rs:916-927`) | closed |
| T-30-17 | Tampering | `relaunch_prefill` | medium | mitigate | caps re-parsed (`driver_launch.rs:270-287`); model charset gate (`:229-237`, `:260-269`) | closed |
| T-30-18 | Tampering | `SlugInputScreen`, `create_run_dir` | medium | mitigate | `SLUG_RE` + 80-byte bound (`driver_launch.rs:1169-1181`; `src/driver_templates/mod.rs:145-154`); symlink/outside-project refusals (`:205-237`); `EEXIST` -> `Exists` (`:240-244`); `create_new` files (`:178-187`) | closed |
| T-30-19 | Denial of Service | run lifetime | medium | mitigate | grep gate: 0 hits for `RunStore`, `driverctl::serve`, `UnixListener` in `src/ui` and `src/app.rs`; only `launch_detached` seam (`app.rs:603`, `:1318`) | closed |
| T-30-20 | Information Disclosure / Tampering | `launch_error_copy` | low | mitigate | `capped_detail` 3 lines / 300 chars via `render_for_terminal` (`driver_launch.rs:49-51`, `:396-462`) | closed |
| T-30-SC | Tampering | npm/pip/cargo installs | high | accept | see Accepted Risks AR-30-03; `Cargo.toml`/`Cargo.lock` unchanged across the phase | closed |

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

### Auditor Notes (informational, non-blocking)

1. T-30-01/T-30-06 wording vs code: three styling decisions compare closed frontmatter literals (`sandbox == "degraded"` at `driver_run.rs:1042`, `custom:` prefix at `:1064`, `- hold: true` at `:1462`), and the `stale` glyph uses heartbeat age only while the lock is held (`:427-432`). None lets free text pick a glyph/colour or produce `crashed`/`running`. The module doc at `driver_run.rs:8-9` ("heartbeat freshness picks no glyph") is stale against the D-08 stale rule — documentation drift only.
2. Follow-up for 28-SECURITY (outside the declared register): `read_driver_run_text`, `read_direction`, `apply_edit`'s `File::open` and `tail_lines` open by path with no regular-file / non-blocking guard. Beyond the flagged TOCTOU windows, a FIFO planted in a run dir would block one blocking-pool worker. Only an `--unsandboxed` run's own driver (already full user permissions) can do this; sibling run dirs are tmpfs-masked in the sandbox. `[inferred — audit]` recorded, not opened as a threat.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-30-01 | T-30-08 | An `--unsandboxed` driver could forge a sibling's frontmatter to show a false parked marker; liveness/crash still come only from the lock probe (`driver_run.rs:489-516`); sibling run dirs tmpfs-masked in the sandbox (`src/launcher/sandbox.rs:13-14`, `:258-259`, quick 261020-f80). | 30-03-PLAN threat model `[inferred — audit]` | 2026-10-20 |
| AR-30-02 | T-30-13 | Phase 27 D-20: after 2 s the write proceeds without DIRECTION.lock; exchange verify still prevents loss; `lock_timed_out` surfaced (`src/driver/direction.rs:76`, `:549-553`; `src/ui/screens/driver_steer.rs:58`, `:590-594`). | 30-04-PLAN threat model `[inferred — audit]` | 2026-10-20 |
| AR-30-03 | T-30-SC | No package added; `Cargo.toml`/`Cargo.lock` unchanged since 258053d (before phase). | 30-0x-PLAN threat models `[inferred — audit]` | 2026-10-20 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-20 | 21 | 21 | 0 | gsd-security-auditor (sonnet), /gsd-secure-phase 30; 143 lib tests across 13 security filters green |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-20
