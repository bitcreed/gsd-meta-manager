---
phase: "27"
slug: "driverctl-driver-frontmatter"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-19"
---

# Phase 27 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register source: the `<threat_model>` blocks of 27-01..27-06-PLAN.md (authored at plan time). All six SUMMARY `## Threat Flags` sections report no new surface beyond the register. ASVS level 1 and 0 open threats at plan-register time, so per secure-phase step 3 this was an L1 (grep-depth plus test-run) verification and no auditor was spawned. Evidence was first gathered on HEAD `32fa9a1` (2026-10-19) and refreshed on HEAD `dd1a08c` (2026-10-28) and HEAD `4c7a46a` (2026-10-29), after Phases 28, 28.1, 30 and 31, the quick tasks and cleanup batch 261028-n0u touched covered files.

Test evidence (`cargo test --no-fail-fast`, 2026-10-19): driverctl_client 19/19, driverctl_frontmatter 11/11 (1 ignored: golden regenerator), driverctl_liveness 11/11, driverctl_lock_race 5/5, driverctl_race 4/4 (1 ignored: the 15 s `editor_race_full`; the default smoke variant asserting `lost_rename == 0` ran), driverctl_replay 4/4, driverctl_serve 11/11, driverctl_store 29/29, plus lib unit tests filtered to driverctl/direction/lock/driver_frontmatter 111/111. 0 failures.

---

Refreshed test evidence (`cargo test --no-fail-fast`, 2026-10-28, HEAD `dd1a08c`): driverctl_client 20/20, driverctl_frontmatter 12/12 (1 ignored: golden regenerator), driverctl_liveness 11/11, driverctl_lock_race 5/5, driverctl_race 4/4 (1 ignored: `editor_race_full`; `editor_race_smoke` ran), driverctl_replay 4/4, driverctl_serve 13/13, driverctl_store 60/60, lib unit tests filtered to driverctl/direction/lock/driver_frontmatter 307/307. 0 failures.

Phase 31 retirement check `[inferred — cleanup 2026-10-28]`: Phase 31 deleted `src/driver/{goal,bounds,escalate,untrusted,run,spawn,reconcile,dry_run}.rs` (commits `2a23d9e`, `9caf162`) and the executor/envelope layers. No Phase 27 threat's mitigation or files-to-check lived in those files (grep of 27-0*-PLAN.md finds none), and every Phase 27 component (`src/driverctl/*`, `src/driverctl_cli.rs`, `src/driver/{direction,lock}.rs`, `src/state_reader/driver_frontmatter.rs`) is still present and live. Retired-by-31: 0.

---

Refreshed test evidence (`cargo test --no-fail-fast`, 2026-10-29, HEAD `4c7a46a`): driverctl_client 20/20, driverctl_frontmatter 12/12 (1 ignored: golden regenerator), driverctl_liveness 11/11, driverctl_lock_race 5/5, driverctl_race 4/4 (1 ignored: `editor_race_full`; `editor_race_smoke` ran), driverctl_replay 4/4, driverctl_serve 16/16, driverctl_store 61/61, lib unit tests filtered to `driverctl::` 135, `direction` 47, `lock` 131, `driver_frontmatter` 27 (filters may overlap). 0 failures.

Post-`dd1a08c` delta check `[inferred — cleanup 2026-10-28]`: quick batch 261028-t3d (`65233b8`, `4977318`) hardened the run-start `DIRECTION.lock` creation (`serve.rs:938-939`, via `ensure_lock_file`), the run-start `state.json` read (`store.rs:637-672`), and the `lock.rs` probe/holder opens (`O_NOFOLLOW | O_NONBLOCK`, single-link regular file, `lock.rs:155-177`) per 27-REVIEW WR-03/WR-04. These strengthen T-27-02, T-27-03 and T-27-17 and remove no existing mitigation; new tests `a_fifo_direction_lock_fails_the_start_instead_of_hanging`, `a_dangling_symlinked_direction_lock_fails_the_start_and_creates_nothing`, `a_fifo_state_json_fails_the_start_instead_of_hanging` (`tests/driverctl_serve.rs`), `a_crashed_run_with_direction_md_absent_is_still_refused_dispatch` (`tests/driverctl_store.rs`) and the `probe_at_*` symlink/hard-link/FIFO unit tests pass. `7f4fc49` (261028-t3e) changed the swap-back path of `splice_frontmatter` (T-27-01) to restore by no-replace rename when DIRECTION.md is absent; the race smoke still asserts `lost_rename == 0` and passes. Cargo.toml/Cargo.lock untouched by these commits. No new surface outside the register; line anchors in the register re-pinned to HEAD `4c7a46a`.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| human editor -> DIRECTION.md | human-owned body, saved by rename or in place at any moment | user prose (integrity-sensitive) |
| sandboxed driver -> run dir lock files | flock works on O_RDONLY fds of a read-only bind | lock state |
| TUI readers -> launcher.lock | concurrent LOCK_SH probes vs. a starting launcher | liveness signal |
| sandboxed project tree -> scan | VERIFICATION.md bytes, phase dir names and paths are sandbox-writable | gap counts, paths that become LOG header values |
| run dir DIRECTION.md -> TUI reader | reader parses a possibly hand-edited frontmatter block | untrusted YAML |
| store state -> emitter | outcomes, questions and session ids reach YAML text | untrusted strings |
| sandboxed driver / hooks -> driverctl client -> unix socket -> serve | every request field and connection's pacing is untrusted; the client's answer decides whether a dispatch proceeds | JSON requests, gate decisions |
| store -> LOG.md / state.json / DIRECTION.md | store is the sole writer | run state read by Phases 28 and 30 |
| store -> host git | `rev-parse` on the host against a sandbox-writable repo | commit sha |
| other local users -> socket dir | a shared runtime or /tmp dir could be pre-created or squatted | socket access |
| CLI args -> run dir path | `--run-dir` names where the store writes | filesystem path |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-27-01 | Tampering | `direction::splice_frontmatter` | high | mitigate | Exchange-verify with swap-back on displaced body bytes; `tests/driverctl_race.rs` smoke asserts `lost_rename == 0` (default run, passing) | closed |
| T-27-02 | Denial of Service | DIRECTION.lock held from inside sandbox | medium | mitigate | `DIRECTION_LOCK_BUDGET` = 2 s (`src/driver/direction.rs:88`); timeout proceeds and reports `SpliceReport::lock_timed_out` (`:362`); run-start `DIRECTION.lock` creation goes through the guarded `ensure_lock_file` (`src/driverctl/serve.rs:938-939`, 261028-t3d), so a planted FIFO/symlink is a refused start, not a hang | closed |
| T-27-03 | Spoofing | false "already running" refusal | medium | mitigate | Readers probe `NonBlockingLockShared` (`probe_at`, `src/driver/lock.rs:437`, flock `:450`); probe/holder opens carry `O_NOFOLLOW \| O_NONBLOCK` and a single-link regular-file check (`:155-177`, 261028-t3d), so a symlink/hard link/FIFO reads `Unknown`, never `Held` (unit tests `:720`, `:737`, `:753`); acquirers retry for `ACQUIRE_RETRY_BUDGET` (`:82`); `tests/driverctl_lock_race.rs` 5/5 | closed |
| T-27-04 | Tampering | power loss surfacing empty DIRECTION.md | low | mitigate | temp `sync_all()` before exchange (`src/driver/direction.rs:732`) | closed |
| T-27-05 | Tampering | gap count inflated by verifier re-write | high | mitigate | `COALESCE_S` = 45.0 coalescing (`src/driverctl/counters.rs:22`); Run 1 replay test in `tests/driverctl_replay.rs` 4/4 | closed |
| T-27-06 | Tampering | LOG grammar injection via phase dir/file name | medium | mitigate | `scan` skips paths containing `\|`, CR, LF or > 200 chars (`src/driverctl/scan.rs:146`); `log::one_line` (`src/driverctl/log.rs:159`) | closed |
| T-27-07 | Denial of Service | non-numeric phase dir aborting scan tick | low | mitigate | `phase_key` returns `None` (`src/driverctl/counters.rs:161`); test `non_numeric_phase_dir_is_skipped_not_fatal` (`scan.rs:289`) | closed |
| T-27-08 | Repudiation | driver under-reporting its own gap rounds | high | mitigate | counts derived only from on-disk `gaps_found` bytes (digest + mtime) in `scan`; no wire op sets them (`no_request_can_set_crashed_or_stalled_or_rewrite_a_counter`, `tests/driverctl_store.rs:1521`) | closed |
| T-27-09 | Tampering | YAML injection in `DriverFrontmatter::emit` | medium | mitigate | `quote()` escapes `\\`, `"` and control chars (`src/state_reader/driver_frontmatter.rs:506`); test `quote_escapes_control_and_quotes` (`:1527`) | closed |
| T-27-10 | Denial of Service | hostile block reaching TUI reader | low | mitigate | typed `FrontmatterError` (`driver_frontmatter.rs:438`), major-version gate (`UnsupportedVersion`), unknown keys ignored; `tests/driverctl_frontmatter.rs` 12/12 | closed |
| T-27-11 | Elevation of Privilege | in-sandbox panic (rc 101) failing the gate open | high | mitigate | `driverctl` dispatched before tracing/color-eyre/config (`src/main.rs:56`, first arm of `main`); fallible `init_tracing`; tests spawn with `XDG_DATA_HOME=/proc/nonexistent-gsdmm` (driverctl_client/serve/replay) | closed |
| T-27-12 | Elevation of Privilege | transport error / `ok:false` letting dispatch through | high | mitigate | `hook_main` denies the gate on `ClientError`, unset socket, non-`ok` reply (`hook_main`, `src/driverctl_cli.rs:548-600`); tests `gate_fails_closed_when_unreachable` (`tests/driverctl_client.rs:372`), `gate_fails_closed_without_socket_env` (`:385`), `gate_fails_closed_on_an_unreadable_reply` (`:397`), `server_error_on_gate_denies` (`:424`) | closed |
| T-27-13 | Spoofing | dead run shown live / live run shown crashed | medium | mitigate | LOCK_SH probe, D-22 table, `LivenessUnknown` (`src/state_reader/driver_frontmatter.rs:727`); `tests/driverctl_liveness.rs` R1-R6 11/11 | closed |
| T-27-14 | Denial of Service | client hanging on stalled server | medium | mitigate | `CLIENT_TIMEOUT` = 15 s (`src/driverctl_cli.rs:60`) applied as a total deadline to connect and socket I/O (`:245-247`); timeout is a `ClientError` -> gate deny | closed |
| T-27-15 | Tampering | LOG grammar injection via header values | high | mitigate | every header value through `log::one_line`; report control chars rejected; rejection-table tests in `tests/driverctl_store.rs` 61/61 | closed |
| T-27-16 | Elevation of Privilege | sandbox forging crashed/stalled or counters | high | mitigate | `set_terminal`/`begin_launch`/`observe_verification` have no wire op; `tests/driverctl_store.rs:1521` | closed |
| T-27-17 | Tampering | store rewrite dropping launcher-owned keys | medium | mitigate | `#[serde(flatten)] extra` (`src/driverctl/store.rs:105`, `:129`); r261a forward-compat test passing; the run-start `state.json` read is the guarded, 1 MiB-capped open (`store.rs:637-672`, 261028-t3d WR-03), so a FIFO/symlink at the name refuses the start instead of hanging or reading through the link | closed |
| T-27-18 | Elevation of Privilege | host-side git in sandbox-writable repo | medium | mitigate | only `rev-parse --short HEAD` via `git_read_raw_within`, output held to hex (`src/driverctl/store.rs:1659-1665`) | closed |
| T-27-19 | Tampering | rejected request leaving partial write | medium | mitigate | validate, apply on clone, append LOG, commit; byte-identity assertions in rejection table (`tests/driverctl_store.rs`) | closed |
| T-27-20 | Denial of Service | oversized report or state growth | low | mitigate | `MAX_REPORT_LINES` = 15 (`src/driverctl/protocol.rs:43`, truncation `:431`); `MAX_REQUEST_BYTES` = 1 MiB (`:40`, enforced in `serve.rs:731-738`) | closed |
| T-27-21 | Tampering | second serve stealing a live socket | high | mitigate | run lock acquired first in `serve` (`src/driverctl/serve.rs:921`) before `replace_and_bind` (`:527`, unlink `:537`, bind `:578`); test `second_serve_is_refused_and_does_not_steal_the_socket` (`tests/driverctl_serve.rs:344`) | closed |
| T-27-22 | Spoofing | another user pre-creating socket dir | high | mitigate | no /tmp fallback (`serve.rs:336-358`; macOS-only per-user `TMPDIR` base, still subject to the checks); dir created 0700 (`:371`), owned by our uid (`:380`), no group/other bits (`:386`); `default_socket_path_requires_xdg_runtime_dir` + unit tests (symlinked dir, wrong mode) | closed |
| T-27-23 | Tampering | run id / run dir path escape | high | mitigate | `RUN_ID_RE` (`serve.rs:123`) + canonical parent equals `<project>/.planning/meta-manager/drivers` in `validate_run_dir` (`:212`, parent check `:248-262`); `run_dir_outside_drivers_is_refused` (`tests/driverctl_serve.rs:633`) | closed |
| T-27-24 | Denial of Service | stalled connection starving the gate hook | high | mitigate | thread per connection, `MAX_CONNECTIONS` = 32 (`serve.rs:110`) with `busy` reply, `CONN_TIMEOUT` 10 s total read deadline (`:113`), 1 MiB cap; `stalled_connection_does_not_block_the_gate` (`tests/driverctl_serve.rs:844`), `a_flood_of_idle_connections_is_answered_busy_and_the_gate_denies` (`:872`) | closed |
| T-27-25 | Spoofing | two projects colliding on one socket | medium | mitigate | socket name carries `sha256(canonical run dir)[:8]` (`serve.rs:268-275`); hash stability/difference unit test | closed |
| T-27-26 | Information Disclosure | socket reachable by other users | medium | mitigate | bind in a private 0700 staging dir, chmod 0600, then rename into place (`serve.rs:551-580`; CR-01 replaced the process-wide umask swap); parent dir 0700 (`:371`); mode assertions (`serve.rs:1162`, `:1321`, `:1631`, `tests/driverctl_serve.rs`) | closed |
| T-27-SC | Tampering | npm/pip/cargo installs | high | accept | No package installs: `git log 3ca76e0^..HEAD -- Cargo.toml Cargo.lock` is empty for the phase (later dependency changes, e.g. 28.1, belong to those phases' registers) | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-27-01 | T-27-SC | Phase adds no dependencies; all crates used (rustix, sha2, regex, serde_json, serde_yml, chrono, clap, tracing-appender, tokio) were already in Cargo.lock. Verified: Cargo.toml/Cargo.lock untouched across phase commits. | plan-time disposition (27-01..27-06 PLAN); confirmed by secure-phase run | 2026-10-19 |

*Accepted risks do not resurface in future audit runs.*

### Notes for later audit (outside the register, not scored)

- `[inferred — audit]` (2026-10-19) `driverctl hook` exited 0 silently when stdin was unreadable or not a JSON object. **Resolved at HEAD `dd1a08c`**: `hook_main` now prints the deny JSON in both cases (`src/driverctl_cli.rs:553`, `:560`; module doc `:29-32`, SEC-01), covered by `gate_fails_closed_on_an_unreadable_reply` (`tests/driverctl_client.rs:397`).
- The 27-06 socket-dir design records "no /tmp fallback" as `[inferred — audit]` (`serve.rs:37`); confirmed implemented at HEAD `dd1a08c`.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-19 | 27 | 27 | 0 | gsd-secure-phase (orchestrator, L1 short-circuit; no auditor per ASVS 1 rule) |
| 2026-10-28 | 27 | 27 (0 retired-by-31) | 0 | gsd-secure-phase re-run at HEAD `dd1a08c` (orchestrator, L1 short-circuit; evidence anchors refreshed, tests re-run) |
| 2026-10-29 | 27 | 27 | 0 | gsd-secure-phase re-run at HEAD `4c7a46a` after 261028-t3d/t3e (orchestrator, L1 short-circuit; anchors re-pinned, tests re-run) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-19; re-verified 2026-10-28 at HEAD `dd1a08c`; re-verified 2026-10-29 at HEAD `4c7a46a` (post 261028-t3d)
