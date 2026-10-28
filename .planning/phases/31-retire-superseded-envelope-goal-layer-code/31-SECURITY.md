---
phase: "31"
slug: "retire-superseded-envelope-goal-layer-code"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
threats_open_nonblocking: 0
asvs_level: 1
block_on: high
created: "2026-10-25"
updated: "2026-10-27"
---

# Phase 31 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Scope: plans 31-01..31-06 and gap-closure plans 31-08, 31-09 (re-audited 2026-10-27). Plan 31-07 (D-04, removal of the experimental Codex executor / in-process Claude transport) is HELD pending a user decision and was not executed. Its threats are **deferred** and not audited here; they are not open gaps of this phase.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| child stderr (git, gh) -> LOG.md | bytes chosen by a remote or credential helper are persisted by the push step | possibly credential-bearing text (high) |
| corpus tests -> kept tests | evidence for ledger bounds T-19-117 / T-19-120 moved off deleted corpora | test evidence (integrity) |
| CLI argument -> terminal | untrusted `--target-phase` echoed in a refusal | untrusted text (low) |
| confirm prompt text -> user decision | the user records/withdraws an opt-in based on the prompt's claims | consent statements (medium) |
| `GSDMM_EXPERIMENTAL_FEATURES` -> visible surface | flag keeps launcher surfaces off a default install | feature gating |
| registry/run identity -> rendered rows | aliases and run ids are attacker-influenced text | untrusted text |
| `.planning/meta-manager/runs/**` on disk -> watcher | leftover legacy run dirs reach the classifier | untrusted files |
| launcher run dirs -> dashboard markers | slow tick reads launcher state for badges | status integrity |
| source tree -> census tests | spawn/async allowlists prove no new spawn/blocking site | test evidence |
| docs -> user expectations | users decide what is gated/consented from docs | claims |
| running legacy `drive` process -> new binary | a detached legacy run may survive the upgrade | process with git/push rights |
| acceptance evidence dir -> `envelope scan` | campaign evidence passes the scan before being kept | possibly secrets |
| policy.rs reduction -> launcher push step | push.rs compiles against what policy.rs keeps | namespace/cap semantics |
| release script -> source tree | pre-tag-check read the removed git-version constant | release gate integrity |
| old envelope dirs on disk -> new binary | stale stubs exec removed `envelope <hook-arm>` subcommands | exec path |
| closure record -> maintainers/auditors | Phase 19/21 closure rows decide residual ownership | audit record |
| 28.1 verification -> Phase 19 closures | N-class closures lean on the pending 28.1 D-11 live probe | audit record |
| detached launcher run -> TUI liveness view / removal | the TUI learns of a live run only through the cached launcher marker (31-08) | process with git/push rights |
| driver-written checkout -> host push step (planted-file guard) | the sandboxed driver controls tree names, sizes and layout; the guard runs on the host (31-09) | untrusted files |
| host process -> PR ledger file | user-writable file outside the sandbox (31-09) | ledger bytes |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-31-01 | Information disclosure | `push.rs` `git_with_env` / `open_pull_request` -> `log_event` | high | mitigate | `redact_captured` (`src/launcher/push.rs:247-249`) applied at both capture sites (`push.rs:332`, `push.rs:890`), the only callers of `run_bounded_with_stderr`; tests `push.rs:1156`, `tests/launcher_push.rs:598`. Residual R-31-01 (WR-05) recorded below | closed |
| T-31-02 | Tampering (evidence) | ledger T-19-117 / T-19-120 bound pins | medium | mitigate | seven bound tests in `src/envelope/ledger.rs:1088-1254`; slice pin `tests/envelope_pr_cap.rs:279`; bound code intact `ledger.rs:330-370` | closed |
| T-31-03 | Spoofing | `next_cli.rs` target-phase refusal | low | mitigate | `render_for_terminal(target)` at `src/next_cli.rs:149-156`; `tests/next_cli.rs:249` passes | closed |
| T-31-04 | Repudiation | `driver_confirm.rs` opt-in prompts | medium | mitigate | spawn-gate / no-new-run claims removed (pinned by `driver_confirm.rs:1073-1090`). **Not delivered:** prompts (`driver_confirm.rs:237,252`), module doc (`:12-14`) and `docs/CONFIGURATION.md:143-145` claim the push step reads a branch namespace from the opt-in, but `policy_for` reads only PR caps and `dest_ref`/`check_destination` hard-code `default_namespace` (review WR-02); grant popup still renders retired model-seam / re-confirmation sections (review WR-01). **Closed by 31-08:** prompts now say "PR caps" only (`src/ui/screens/driver_confirm.rs:13-17,87-88,118,132`), `docs/CONFIGURATION.md:143-147` says `branch_namespace` is accepted but unused; retired-seam disclosure removed, pinned by `the_grant_popup_shows_no_disclosure_of_retired_seams` | closed |
| T-31-05 | Elevation of privilege | `ctx.experimental` gate sites | medium | mitigate | surviving gates `src/app.rs:989,1041,1169,1411,2620`, `src/ui/screens/detail.rs:4728`; diff vs 036426d removes only the two gates inside deleted functions | closed |
| T-31-06 | Information disclosure | render_escape_guard coverage | medium | mitigate | `src/ui/screens/render_escape_guard.rs:374`; census, probe-fixture and identity-escape tests pass | closed |
| T-31-07 | Denial of service | `journal::classify_change` on legacy run dirs | low | mitigate | `src/journal/mod.rs:364-370` returns `Ignored`; tests `journal/mod.rs:396,440`, `src/watcher.rs:229` | closed |
| T-31-08 | Tampering (status integrity) | 20-tick marker scan | medium | mitigate | single `spawn_blocking` + flag short-circuit `src/app.rs:1840-1853`; `apply_launcher_markers` `src/ui/screens/mod.rs:1648`; `launcher_marker_` tests pass | closed |
| T-31-09 | Elevation of privilege | delete_confirm live-legacy-run refusal | low | accept | see Accepted Risks Log | closed |
| T-31-10 | Tampering | spawn/async census allowlists | medium | mitigate | removed entries all name deleted files or files with no `Command::`; bidirectional `vanished` check kept `tests/spawn_seam_guard.rs:345-350`; floors recorded `31-04-SUMMARY.md:169` | closed |
| T-31-11 | Elevation of privilege | detached legacy `drive` process | low | accept | see Accepted Risks Log | closed |
| T-31-12 | Tampering | router / `next --json` contract | medium | mitigate | `tests/driver_router_conformance.rs` 0 commits since 036426d; `src/driver/router.rs` diff comment-only; 6 tests pass | closed |
| T-31-13 | Repudiation | README / CONFIGURATION / GSD-CORE-SYNC claims | low | mitigate | `README.md:42,60`, `docs/CONFIGURATION.md:84,143-149`, `docs/GSD-CORE-SYNC.md:344` consistent with removals | closed |
| T-31-14 | Tampering | release gate (`pre-tag-check.sh` + constant) | medium | mitigate | no witness variable survives in `scripts/` or `src/envelope/policy.rs`; `bash -n scripts/pre-tag-check.sh` clean | closed |
| T-31-15 | Information disclosure | `envelope scan` evidence gate | high | mitigate | `EnvelopeAction::Scan` `src/cli.rs:469-484`, arm `src/main.rs:364-395`; binary run: PEM key -> exit 1 `rule=pem`, clean dir -> exit 0; `push.rs:708` still scans before push | closed |
| T-31-16 | Elevation of privilege | stale hook/askpass stubs | low | accept | see Accepted Risks Log | closed |
| T-31-17 | Tampering | namespace/scan/ledger semantics in policy.rs rebuild | high | mitigate | `classify_push_ref`, `validate_namespace`, `default_namespace` byte-identical to 036426d; `scan.rs` no diff; 10 policy tests pass; only push.rs edit is 31-01 `8ed63d2` | closed |
| T-31-18 | Tampering | `cargo package --locked` (gate 6) | medium | mitigate | gate 6 at `scripts/pre-tag-check.sh:824-825`; every `include_str!`/`include_bytes!` target resolves; gate 6 PASS in `31-05-SUMMARY.md` | closed |
| T-31-19 | Repudiation | Phase 19/21 closure rows | high | mitigate | Phase 19 `deferred-items.md:3232` table, 27 rows; K rows T-19-72/73 stay open with todo owners (todos exist); Phase 21 `deferred-items.md:2963` marks CR-01/CR-02 NOT superseded; 27 vs 26 reconciled by name (T-19-105) | closed |
| T-31-20 | Repudiation | N-class closures pending 28.1 D-11 probe | medium | mitigate | T-19-86/112/116/121 and Phase 21 criterion-4 closure carry the literal reopen conditional; awk check clean | closed |
| T-31-21 | Repudiation | earlier audit text | medium | mitigate | only removed lines since 036426d: `status: blocked`, `threats_open: 6`, `updated: 2026-09-04` | closed |
| T-31-25 | Elevation of privilege | `do_remove_project` live-launcher-run refusal (WR-03) | medium | mitigate | guard on cached `launcher_marker.any_live` at `src/ui/screens/delete_confirm.rs:121-140`; refusal/control tests `delete_confirm.rs:326,356` | closed |
| T-31-26 | Spoofing | opt-in grant popup disclosure (WR-01) | medium | mitigate | retired-seam sections removed; `the_grant_popup_shows_no_disclosure_of_retired_seams` in `src/ui/screens/driver_confirm.rs` pins absence on the rendered buffer | closed |
| T-31-27 | Tampering | opt-in prompts / doc / CONFIGURATION.md namespace claims (WR-02) | low | mitigate | "PR caps only" wording `driver_confirm.rs:13-17,118,132`, `docs/CONFIGURATION.md:143-147`; push-step destination logic untouched | closed |
| T-31-28 | Information disclosure | live-run refusal message alias | low | mitigate | `crate::text::render_for_terminal(alias)` at `src/ui/screens/delete_confirm.rs:137` | closed |
| T-31-29 | Information disclosure | `run_bounded_with_stderr` tail window -> `redact_captured` (WR-05) | medium | mitigate | partial first line dropped / over-long line -> `OVERLONG_STDERR_NOTE` via `last_line_of_tail` (`src/launcher/process.rs:195-238`); tests `process.rs:261,278`; cap after redaction `src/launcher/push.rs:253-255` (`MAX_CAPTURED_LINE_CHARS`), used at `push.rs:339,897` | closed |
| T-31-30 | Tampering | planted-file walk under truncation | low | mitigate | name-ordered walk, whole-directory truncation `src/launcher/push/planted.rs:309,344`; `a_truncated_walk_records_the_same_entries_in_any_directory_order` `planted.rs:1421` | closed |
| T-31-31 | Denial of service | planted-file directory listing memory | low | mitigate | at most `MAX_WALK_ENTRIES + 1` entries held (`src/launcher/push/planted.rs:62-63`) | closed |
| T-31-32 | Tampering | release gate certification (SC4) | medium | mitigate | both container-only tests made host-independent without new `#[ignore]` (diff 36e0805~1..165a80c adds none); `./scripts/pre-tag-check.sh --container v1.15.0` exit 0, gates 1-6 PASS (`31-09-SUMMARY.md:16,107`) | closed |
| T-31-33 | Repudiation | PR-ledger size refusal text (WR-04) | low | mitigate | message states the byte bound `src/envelope/ledger.rs:423-431`; `MAX_LEDGER_BYTES` unchanged `ledger.rs:128` | closed |
| T-31-SC | Tampering | npm/pip/cargo installs | high | accept | see Accepted Risks Log | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

### Residuals on closed threats

| Residual | Threat Ref | Description | Tracking |
|----------|------------|-------------|----------|
| R-31-01 | T-31-01 | WR-05: `run_bounded_with_stderr` (`src/launcher/process.rs:192-203`) reads only the last 4096 bytes; a final stderr line longer than the window can start mid-token, defeating the prefix-anchored redactor (`journal/redact.rs:155`) so a token suffix could reach LOG.md / `PushOutcome::Failed.detail`; the line has no length cap. Requires a >4 KiB single-line stderr. Declared control is present at both sites, so T-31-01 is closed at ASVS L1 `[inferred — audit]`; at L2+ this would reopen T-31-01 at high (blocking). Fix: drop a partial first line and cap the logged line | **Resolved by 31-09** (T-31-29, commit 21b94df) |
| R-31-02 | T-31-09 | WR-03: unregistering a project is not guarded against a live **launcher** run; accepted as out of scope by plan 31-03 | **Resolved by 31-08** (T-31-25) |

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-31-01 | T-31-09 | After plan 31-03 the TUI cannot observe a legacy run and none can be started; a legacy `drive` still alive at upgrade is an operator kill (`31-03-SUMMARY.md:157-159`); the removed guard read only `ctx.observed_runs`, so launcher runs never had it | plan 31-03 threat model | 2026-10-25 |
| AR-31-02 | T-31-11 | Nothing can start a new legacy `drive`; `drive` now parses as a positional TARGET; operator told to kill survivors (`31-04-SUMMARY.md:286-288`) | plan 31-04 threat model | 2026-10-25 |
| AR-31-03 | T-31-16 | Stale stubs exec removed subcommands; `envelope pre-push/pre-commit/askpass/guard` exit 2 "unrecognized subcommand" (fail closed); nothing persisted in repo config (T-19-02) | plan 31-05 threat model | 2026-10-25 |
| AR-31-04 | T-31-SC | No package added or upgraded: `Cargo.lock` unchanged since 036426d, `Cargo.toml` comment-only edits; re-checked for 31-08/31-09: `Cargo.lock`/`Cargo.toml` untouched in 36e0805~1..165a80c | plans 31-01..31-06, 31-08, 31-09 threat models | 2026-10-27 |

*Accepted risks do not resurface in future audit runs.*

---

## Notes

- Deferred (not audited): all threats of plan 31-07 (held for user decision D-04).
- Conditional: four high N-class Phase 19 closures (T-19-86, T-19-112, T-19-116, T-19-121) reopen if the Phase 28.1 D-11 live probe fails — a declared condition, not a gap.
- Superseded: the container dry run was red at gate 4 on two pre-existing 28.1 launcher tests (`31-05-SUMMARY.md`); 31-09 fixed both and the container run is green, gates 1-6 (T-31-32).
- Orchestrator decisions taken unattended: Step 4 "Verify all open threats" chosen `[inferred — audit]`; WR-05 classified residual rather than open at ASVS L1 `[inferred — audit]`; T-31-04 left open (non-blocking) rather than accepted, so the WR-01/WR-02 rewording stays visible `[inferred — audit]`.
- 2026-10-27 re-audit (verify:post after 31-08/31-09): all new threats closed at L1 grep depth with ASVS L1 and a plan-time register, so the auditor was skipped per the short-circuit rule; T-31-04 closed on 31-08's delivered rewording `[inferred — audit]`.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-25 | 22 | 21 | 1 (non-blocking, medium) | gsd-security-auditor (sonnet) via /gsd-secure-phase 31 |
| 2026-10-27 | 31 | 31 | 0 | orchestrator L1 grep check (auditor skipped: ASVS L1 short-circuit) via /gsd-secure-phase 31 after 31-08/31-09 |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-25 (one non-blocking medium open: T-31-04); re-verified 2026-10-27 after 31-08/31-09 — 31 threats, all closed

## Security Audit 2026-10-28

| Metric | Count |
|---|---|
| Threats found | 31 |
| Closed | 31 |
| Open | 0 |
