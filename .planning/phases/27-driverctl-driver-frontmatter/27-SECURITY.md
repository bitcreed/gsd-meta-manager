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

Register source: the `<threat_model>` blocks of 27-01..27-06-PLAN.md (authored at plan time). All six SUMMARY `## Threat Flags` sections report no new surface beyond the register. ASVS level 1 and 0 open threats at plan-register time, so per secure-phase step 3 this was an L1 (grep-depth plus test-run) verification and no auditor was spawned. Evidence was gathered on HEAD `32fa9a1`.

Test evidence (`cargo test --no-fail-fast`, 2026-10-19): driverctl_client 19/19, driverctl_frontmatter 11/11 (1 ignored: golden regenerator), driverctl_liveness 11/11, driverctl_lock_race 5/5, driverctl_race 4/4 (1 ignored: the 15 s `editor_race_full`; the default smoke variant asserting `lost_rename == 0` ran), driverctl_replay 4/4, driverctl_serve 11/11, driverctl_store 29/29, plus lib unit tests filtered to driverctl/direction/lock/driver_frontmatter 111/111. 0 failures.

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
| T-27-02 | Denial of Service | DIRECTION.lock held from inside sandbox | medium | mitigate | `DIRECTION_LOCK_BUDGET` = 2 s (`src/driver/direction.rs:76`); timeout proceeds and reports `SpliceReport::lock_timed_out` (`:251`) | closed |
| T-27-03 | Spoofing | false "already running" refusal | medium | mitigate | Readers probe `NonBlockingLockShared` (`src/driver/lock.rs:377`); acquirers retry for `ACQUIRE_RETRY_BUDGET` (`:79`); `tests/driverctl_lock_race.rs` 5/5 | closed |
| T-27-04 | Tampering | power loss surfacing empty DIRECTION.md | low | mitigate | temp `sync_all()` before exchange (`src/driver/direction.rs:477`) | closed |
| T-27-05 | Tampering | gap count inflated by verifier re-write | high | mitigate | `COALESCE_S` = 45.0 coalescing (`src/driverctl/counters.rs:22`); Run 1 replay test in `tests/driverctl_replay.rs` 4/4 | closed |
| T-27-06 | Tampering | LOG grammar injection via phase dir/file name | medium | mitigate | `scan` skips paths containing `\|`, CR, LF or > 200 chars (`src/driverctl/scan.rs:95`); `log::one_line` (`src/driverctl/log.rs:154`) | closed |
| T-27-07 | Denial of Service | non-numeric phase dir aborting scan tick | low | mitigate | `phase_key` returns `None` (`src/driverctl/counters.rs:155`), skipped at `scan.rs:92`; test `non_numeric_phase_dir_is_skipped_not_fatal` (`scan.rs:238`) | closed |
| T-27-08 | Repudiation | driver under-reporting its own gap rounds | high | mitigate | counts derived only from on-disk `gaps_found` bytes (digest + mtime) in `scan`; no wire op sets them (`no_request_can_set_crashed_or_stalled_or_rewrite_a_counter`, `tests/driverctl_store.rs:1341`) | closed |
| T-27-09 | Tampering | YAML injection in `DriverFrontmatter::emit` | medium | mitigate | `quote()` escapes `\\`, `"` and control chars (`src/state_reader/driver_frontmatter.rs:459`); test `quote_escapes_control_and_quotes` (`:789`) | closed |
| T-27-10 | Denial of Service | hostile block reaching TUI reader | low | mitigate | typed `FrontmatterError`, major-version gate, unknown keys ignored; `tests/driverctl_frontmatter.rs` 11/11 | closed |
| T-27-11 | Elevation of Privilege | in-sandbox panic (rc 101) failing the gate open | high | mitigate | `driverctl` dispatched before tracing/color-eyre/config (`src/main.rs:103-108`); fallible `init_tracing`; tests spawn with `XDG_DATA_HOME=/proc/nonexistent-gsdmm` (driverctl_client/serve/replay) | closed |
| T-27-12 | Elevation of Privilege | transport error / `ok:false` letting dispatch through | high | mitigate | `hook_main` denies the gate on `ClientError`, unset socket, non-`ok` reply (`src/driverctl_cli.rs:437-464`); tests `gate_fails_closed_when_unreachable` (`tests/driverctl_client.rs:372`), `gate_fails_closed_without_socket_env` (`:385`), `server_error_on_gate_denies` (`:424`) | closed |
| T-27-13 | Spoofing | dead run shown live / live run shown crashed | medium | mitigate | LOCK_SH probe, D-22 table, `LivenessUnknown` (`src/state_reader/driver_frontmatter.rs:651`); `tests/driverctl_liveness.rs` R1-R6 11/11 | closed |
| T-27-14 | Denial of Service | client hanging on stalled server | medium | mitigate | `CLIENT_TIMEOUT` = 15 s (`src/driverctl_cli.rs:55`) applied to socket I/O (`:196`); timeout is a `ClientError` -> gate deny | closed |
| T-27-15 | Tampering | LOG grammar injection via header values | high | mitigate | every header value through `log::one_line`; report control chars rejected; rejection-table tests in `tests/driverctl_store.rs` 29/29 | closed |
| T-27-16 | Elevation of Privilege | sandbox forging crashed/stalled or counters | high | mitigate | `set_terminal`/`begin_launch`/`observe_verification` have no wire op; `tests/driverctl_store.rs:1341` | closed |
| T-27-17 | Tampering | store rewrite dropping launcher-owned keys | medium | mitigate | `#[serde(flatten)] extra` (`src/driverctl/store.rs:105`, `:129`); r261a forward-compat test passing | closed |
| T-27-18 | Elevation of Privilege | host-side git in sandbox-writable repo | medium | mitigate | only `rev-parse --short HEAD` via `git_read_raw_within`, output held to hex (`src/driverctl/store.rs:1034-1036`) | closed |
| T-27-19 | Tampering | rejected request leaving partial write | medium | mitigate | validate, apply on clone, append LOG, commit; byte-identity assertions in rejection table (`tests/driverctl_store.rs`) | closed |
| T-27-20 | Denial of Service | oversized report or state growth | low | mitigate | `MAX_REPORT_LINES` = 15 (`src/driverctl/protocol.rs:43`, truncation `:431`); `MAX_REQUEST_BYTES` = 1 MiB (`:40`, enforced `:310` and in `serve.rs:439-446`) | closed |
| T-27-21 | Tampering | second serve stealing a live socket | high | mitigate | run lock acquired (`src/driverctl/serve.rs:564`) before unlink/bind (`:615`); test `second_serve_is_refused_and_does_not_steal_the_socket` (`tests/driverctl_serve.rs:339`) | closed |
| T-27-22 | Spoofing | another user pre-creating socket dir | high | mitigate | no /tmp fallback (`serve.rs:259`); dir must be a real directory (`:280`), owned by our uid (`:283`), no group/other bits (`:289`); `default_socket_path_requires_xdg_runtime_dir` + unit tests (symlinked dir, wrong mode) | closed |
| T-27-23 | Tampering | run id / run dir path escape | high | mitigate | `RUN_ID_RE` (`serve.rs:94`) + canonical parent equals `<project>/.planning/meta-manager/drivers` in `validate_run_dir` (`:182-215`); `run_dir_outside_drivers_is_refused` | closed |
| T-27-24 | Denial of Service | stalled connection starving the gate hook | high | mitigate | thread per connection, `MAX_CONNECTIONS` = 32 (`serve.rs:81`) with `busy` reply (`:494-511`), `CONN_TIMEOUT` 10 s (`:84`, `:473`), 1 MiB cap; `stalled_connection_does_not_block_the_gate` (`tests/driverctl_serve.rs:669`) | closed |
| T-27-25 | Spoofing | two projects colliding on one socket | medium | mitigate | socket name carries `sha256(canonical run dir)[:8]` (`serve.rs:269-271`); hash stability/difference unit test | closed |
| T-27-26 | Information Disclosure | socket reachable by other users | medium | mitigate | umask 0o177 around bind then chmod 0600 (`serve.rs:339-343`); parent dir 0700 (`:274`); mode assertions in `tests/driverctl_serve.rs` | closed |
| T-27-SC | Tampering | npm/pip/cargo installs | high | accept | No package installs: `git log 3ca76e0^..HEAD -- Cargo.toml Cargo.lock` is empty for the phase | closed |

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

- `[inferred — audit]` `driverctl hook` exits 0 silently when stdin is unreadable or not a JSON object (`src/driverctl_cli.rs:423-428`), before the gate predicate is evaluated. Claude Code always sends a JSON payload, so this is not a reachable bypass of T-27-12 under the plan's model, but it is a fail-open path for a malformed gate payload. Phase 28's planned `onFailure: block` hand-off does not cover it because the exit code is 0. Recorded per the "verify, do not scan for new threats" constraint; not counted.
- The 27-06 socket-dir design records "no /tmp fallback" as `[inferred — audit]` (`serve.rs:32`); confirmed implemented.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-19 | 27 | 27 | 0 | gsd-secure-phase (orchestrator, L1 short-circuit; no auditor per ASVS 1 rule) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-19
