---
phase: 28-sandboxed-driver-launcher
verified: 2026-10-29T06:00:00Z
status: passed
score: 6/6 must-haves verified
covered_files:
  - ".planning/phases/28-sandboxed-driver-launcher/28-01-PLAN.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-01-SUMMARY.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-02-PLAN.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-02-SUMMARY.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-03-PLAN.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-03-SUMMARY.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-04-PLAN.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-04-SUMMARY.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-05-PLAN.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-05-SUMMARY.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-06-PLAN.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-06-SUMMARY.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-07-PLAN.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-07-SUMMARY.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-08-PLAN.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-08-SUMMARY.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-09-PLAN.md"
  - ".planning/phases/28-sandboxed-driver-launcher/28-09-SUMMARY.md"
  - "scripts/launcher-acceptance.sh"
  - "src/driver_cli.rs"
  - "src/driverctl/serve.rs"
  - "src/launcher/auth.rs"
  - "src/launcher/bridge.rs"
  - "src/launcher/config_dir.rs"
  - "src/launcher/gate.rs"
  - "src/launcher/home.rs"
  - "src/launcher/host.rs"
  - "src/launcher/mod.rs"
  - "src/launcher/outcome.rs"
  - "src/launcher/process.rs"
  - "src/launcher/prompt.rs"
  - "src/launcher/push.rs"
  - "src/launcher/run.rs"
  - "src/launcher/sandbox.rs"
  - "src/launcher/stop.rs"
  - "src/ui/screens/driver_launch.rs"
  - "src/ui/screens/driver_run.rs"
  - "tests/launcher_acceptance.rs"
  - "tests/launcher_auth.rs"
  - "tests/launcher_bridge.rs"
  - "tests/launcher_caps.rs"
  - "tests/launcher_config_dir.rs"
  - "tests/launcher_degraded.rs"
  - "tests/launcher_e2e.rs"
  - "tests/launcher_egress.rs"
  - "tests/launcher_live_egress.rs"
  - "tests/launcher_live_isolation.rs"
  - "tests/launcher_live_managed.rs"
  - "tests/launcher_live_scope.rs"
  - "tests/launcher_live_token_reach.rs"
  - "tests/launcher_push.rs"
  - "tests/launcher_reconcile.rs"
  - "tests/launcher_sandbox.rs"
  - "tests/launcher_stop.rs"
covered_digest: "v4:sha256:b4a14569619aba956e7592605ecade5d1811c90d718bdf176124d2bb08a2fe6d"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 6/6
  gaps_closed: []
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "WR-10: a wrong-shape host managed policy value is dropped with no diagnostic (config_dir.rs managed_settings), though a non-object file is refused"
    category: other
    reason: "New in the 415ba4a review. The drop is fail-closed (the launcher's own gate and deny list stay); only the diagnostic is missing. Recorded open in 28-REVIEW-DISPOSITION.md; advisory by decision of this run, not a gap against any success criterion. [inferred — cleanup 2026-10-28]"
    evidence_status: "reviewer read the code; fail-closed behaviour covered by tests/launcher_config_dir.rs (28 pass)"
  - finding: "IN-07: host managed `permissions` scalar settings are first-wins while every other host key is last-wins"
    category: other
    reason: "Info-level consistency nit, recorded open. [inferred — cleanup 2026-10-28]"
    evidence_status: "reviewer read the code, did not run it"
  - finding: "WR-09: a launcher binary under /tmp or /run is unreachable inside the sandbox (snapshot_exe)"
    category: other
    reason: "Fail-closed robustness defect (bwrap exec 127), not an exposure. Still open. [inferred — cleanup 2026-10-28]"
    evidence_status: "reviewer read the code, did not run it"
  - finding: "IN-05 (UTS namespace does not hide /etc/hostname, machine-id) and IN-06 (failed record_launch_posture leaves launch half recorded)"
    category: other
    reason: "Info-level; recorded open. [inferred — cleanup 2026-10-28]"
    evidence_status: "reviewer read the code, did not run it"
optional_human_items:
  - test: "Real Apple Silicon (aarch64-apple-darwin) run of the degraded launcher"
    why_optional: "Hardware coverage only; settled and optional. Darwin behaviour is proven on a real macOS 26.6.2 VM (28-UAT.md test 2) and CI check+clippy builds aarch64-apple-darwin."
---

# Phase 28: Sandboxed Driver Launcher Verification Report

**Phase Goal:** `gsd-meta-manager driver launch` starts a headless master inside a sandbox it cannot write out of, enforces the hard caps from outside, and pushes on the driver's behalf. On Linux this is the default. Hosts without bwrap or systemd-run get an explicit degraded mode.
**Verified:** 2026-10-29 at HEAD aae0816 (source tree identical to 66e3d94; only `.planning` docs commits since)
**Status:** passed
**Re-verification:** Yes. The prior report (2026-10-28, passed) read `stale` because quick batch 261028-t3c/t3f and quick 261028-v2z changed covered files: `src/driverctl/serve.rs`, `src/launcher/config_dir.rs`, `src/launcher/run.rs`, `src/launcher/stop.rs`, `tests/launcher_config_dir.rs`. Every truth was re-checked against HEAD, citations refreshed, live evidence re-run in this session, and the covered-input fingerprint regenerated with `verification.fingerprint`.

## What changed since the last report (delta review)

- `config_dir.rs`: host managed `hooks`/`permissions` of the wrong shape no longer replace the launcher's gate (WR-08 fixed; `managed_settings` keeps the launcher's `hooks`/`permissions` objects and merges only `well_formed_hooks` / `well_formed_permissions`, lines ~664-701). Strictly tightens truth 3 and the gate half of truth 2.
- `run.rs` / `serve.rs`: `DIRECTION.lock` is created through the guarded `ensure_lock_file`; the wall-clock pre-check reads `state.json` via `read_run_file_capped` (a FIFO or symlink is refused, not a hang). Test `the_wall_clock_check_does_not_block_on_a_fifo_state_json`.
- `stop.rs`: orphan-scope stop reads `state.json` through `read_run_file_capped`. Test `an_orphan_stop_does_not_block_on_a_fifo_state_json`.
- No regression in any behaviour a success criterion depends on (evidence below).

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Master runs in bwrap under `systemd-run --user --scope`; stop kills the whole scope; a second launcher is refused by the flock; relaunch logs `crash-detected` | VERIFIED | `run.rs:1598` `systemd_run_argv(unit, runtime_max, &bwrap_argv(&spec, &bridged))`; `sandbox.rs:506` `bwrap_argv`, `:681` `systemd_run_argv`. Live this session (`launcher_live_scope -- --ignored`): `stop_kills_every_process_of_the_scope` ok, `runtime_max_sec_backstop_stops_a_scope` ok (2/2, 3.34 s). `acceptance check l5` RESULT PASS. `launcher_stop` 5 pass, `launcher_e2e` 41 pass. |
| 2 | L1-L5 (and L7/L8/D33) misbehaving-driver cases pass: dispatch cap, wall clock + RuntimeMaxSec, gap cap deny limit+1 / kill limit+2, rate-limit park | VERIFIED | `scripts/launcher-acceptance.sh check` for l1 l2 l3 l4 l5 l7 l8 d33: all `RESULT PASS` (re-run this session). `gate.rs:71` `decide_dispatch`, `outcome.rs:200` `decide`. `launcher_caps` 18 pass. Rate-limit park is a pure function with unit tests in `outcome.rs`; a real rate limit cannot be provoked on demand. |
| 3 | Inside the sandbox: host `~/.claude` RO; `.credentials.json` and `~/.claude.json` unreadable; `~/.ssh` empty; `SSH_AUTH_SOCK`/`GH_TOKEN`/`ANTHROPIC_API_KEY` absent; run dir RO; per-run `CLAUDE_CONFIG_DIR` | VERIFIED | Re-ran `launcher_live_isolation -- --ignored` under real bwrap this session: `sandbox_isolates_host_credentials_and_services` ok and `the_final_profile_closes_every_audited_channel` ok (2/2, 68.8 s). `launcher_config_dir` 28 pass (was 26; +2 tests for the fail-closed managed merge). `launcher_egress` 21 pass, `launcher_sandbox` 15 pass. |
| 4 | Auth: stored `claude setup-token` token as `CLAUDE_CODE_OAUTH_TOKEN`; guided onboarding; host-token fallback detects `OAuth token revoked` and relaunches; no refresh token in sandbox | VERIFIED | `auth.rs:228` `resolve_launch_token` (called `run.rs:639`), `outcome.rs:175` `is_auth_failure`, `run.rs:1130` `auth_relaunch`. `launcher_auth` 13 pass incl. `stored_token_wins_over_the_host_token`. UAT 1 passed on agent evidence (28-UAT.md); the Anthropic browser OAuth login was done and is settled. |
| 5 | Commits leave only via gsd-mm's push step outside the sandbox: fixed refspec into the reserved namespace, secret scan, protected-branch refusal, PR cap; push inside the sandbox fails | VERIFIED | `push.rs:178` `dest_ref`, `:203` `check_destination`, `:918` `push_finished_run` (called `driver_cli.rs:532`). `launcher_push` 15 pass. No credentials and `--unshare-net` in the sandbox (live isolation above), so an in-sandbox push cannot leave. |
| 6 | Without bwrap/systemd-run (macOS) the launcher refuses by default; explicit opt-in degraded mode runs unsandboxed with process-supervision caps; posture recorded and shown in TUI; headless needs no tmux | VERIFIED | `host.rs:286` `decide(caps, unsandboxed)` refuses unless the flag is set (called `run.rs:661`, `:1501`). `launcher_degraded` 17 pass incl. `the_flag_is_never_implied_by_a_failed_probe`. No tmux on any control path (`sandbox.rs:194-195` only scrubs `TMUX`/`TMUX_PANE` from the child env). Real Darwin run: SETTLED, 28-UAT.md test 2 (macOS 26.6.2 VM; no flag refuses naming `bwrap, systemd-run, user-systemd`; `--unsandboxed` records `mode=degraded`; group kill and TUI badge verified). CI `cross-target-check` builds and lints `aarch64-apple-darwin`. |

**Score:** 6/6 truths verified (0 behavior-unverified). Each cancellation/cleanup invariant (scope kill, cap trip, lock, group kill) has a passing live or named test.

### Requirements Coverage

All nine IDs in the ROADMAP (LNCH-01..08, DRIVE-01) are declared in PLAN frontmatter (28-01 [LNCH-01, LNCH-08]; 28-02 [LNCH-01, LNCH-02, DRIVE-01]; 28-03 [LNCH-04, LNCH-05, LNCH-06]; 28-04 [LNCH-07]; 28-05 [LNCH-03, LNCH-05]; 28-06 [LNCH-05, LNCH-03]; 28-07 [LNCH-01, LNCH-07]; 28-08 [LNCH-06]; 28-09 [LNCH-03, LNCH-02, LNCH-04, DRIVE-01]) and are `[x]`/Complete in REQUIREMENTS.md (traceability rows LNCH-01..08 = "Phase 28 | Complete"; DRIVE-01 "Phase 28/29 (re-mapped)", Complete). No orphaned Phase 28 IDs. LNCH-09..15 belong to Phase 28.1 and are out of scope here.

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| LNCH-01 | 28-01, 28-02, 28-07 | SATISFIED | SC1; live scope tests |
| LNCH-02 | 28-02, 28-09 | SATISFIED | l5 PASS; `launcher_stop`, `launcher_e2e` |
| LNCH-03 | 28-05, 28-06, 28-09 | SATISFIED | l1/l2/l3/l4/l7 PASS; `launcher_caps` 18 |
| LNCH-04 | 28-03, 28-09 | SATISFIED | live isolation probe; `launcher_config_dir` 28 |
| LNCH-05 | 28-03, 28-05, 28-06 | SATISFIED | `launcher_auth` 13; UAT 1 |
| LNCH-06 | 28-03, 28-08 | SATISFIED | `launcher_push` 15; SC5 |
| LNCH-07 | 28-04, 28-07 | SATISFIED | `launcher_degraded` 17; UAT 2 real macOS VM |
| LNCH-08 | 28-01 | SATISFIED | no tmux on control path; detached log-file launch |
| DRIVE-01 (re-mapped) | 28-02, 28-09 | SATISFIED (launcher half) | d33 PASS: real driver, injected second `gaps_found`, `cap-tripped gap_rounds kill-scope`. Phase 29 half closed per REQUIREMENTS.md. |

### Behavioral Spot-Checks and Regression

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full regression at HEAD 66e3d94 (orchestrator-supplied; source unchanged since) | `rtk proxy cargo test --no-fail-fast` | 55 suites, 3211 passed, 0 failed, 17 ignored | PASS |
| Live isolation + scope (this session) | `cargo test --no-fail-fast --test launcher_live_isolation --test launcher_live_scope -- --ignored` | 2/2 + 2/2 ok | PASS |
| Launcher integration binaries (this session) | acceptance 7, auth 13, bridge 8, caps 18, config_dir 28, degraded 17, e2e 41, egress 21 (+1 ignored), push 15, reconcile 7, sandbox 15 (+1 ignored), stop 5 | 0 failed | PASS |
| Acceptance verdicts re-derived | `launcher-acceptance.sh check` l1 l2 l3 l4 l5 l7 l8 d33 | all `RESULT PASS` | PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes are declared by the plans. `scripts/launcher-acceptance.sh check` is the phase's probe equivalent and was run above.

### Anti-Patterns / Code Review / Security

- `28-REVIEW.md` (415ba4a) with disposition (aae0816): 0 critical, 4 warning, 3 info. WR-08 is now fixed. WR-02 and WR-04 deferred; WR-09, WR-10, IN-05, IN-06, IN-07 open. None contradicts a success criterion or a must-have; WR-10 and IN-07 are diagnostics/consistency nits on a path that fails closed. They are advisories by decision of this run, not gaps. [inferred — cleanup 2026-10-28]
- `28-SECURITY.md` (37acc29): `status: verified`, `threats_open: 0`.
- `28-UI-REVIEW.md` (90516c0): 18/24, no BLOCK findings.
- Debt markers: grep for `TBD|FIXME|XXX` across covered `src/`, `scripts/` and `tests/` files finds none (one `XXXXXX` mktemp template in `tests/launcher_live_managed.rs:57`, not a marker). `tmux` appears only as an env scrub list.

### Deferred Items

None.

### Human Verification

No items. Apple Silicon hardware testing is optional and settled; the Darwin run and the OAuth browser login are settled per 28-UAT.md.

### Gaps Summary

No gaps. All six roadmap success criteria and all nine requirement IDs are backed by evidence from this session, including live bwrap isolation and live systemd scope kill, and the delta since the previous report only tightens fail-closed behaviour.

---

_Verified: 2026-10-29T06:00:00Z_
_Verifier: Claude (gsd-verifier)_
