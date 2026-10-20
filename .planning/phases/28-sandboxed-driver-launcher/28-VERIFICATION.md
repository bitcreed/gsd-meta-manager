---
phase: 28-sandboxed-driver-launcher
verified: 2026-10-20T01:10:00Z
status: human_needed
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
  - "src/launcher/config_dir.rs"
  - "src/launcher/gate.rs"
  - "src/launcher/host.rs"
  - "src/launcher/mod.rs"
  - "src/launcher/outcome.rs"
  - "src/launcher/process.rs"
  - "src/launcher/push.rs"
  - "src/launcher/run.rs"
  - "src/launcher/sandbox.rs"
  - "src/launcher/stop.rs"
  - "tests/launcher_acceptance.rs"
  - "tests/launcher_auth.rs"
  - "tests/launcher_caps.rs"
  - "tests/launcher_config_dir.rs"
  - "tests/launcher_degraded.rs"
  - "tests/launcher_e2e.rs"
  - "tests/launcher_live_isolation.rs"
  - "tests/launcher_live_scope.rs"
  - "tests/launcher_push.rs"
  - "tests/launcher_sandbox.rs"
  - "tests/launcher_stop.rs"
covered_digest: "v3:sha256:f45ad024a80d6a296a25b893c773a000832891df9d7e8f8c1dc9b8a7883dacb9"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Run `gsd-meta-manager driver auth setup` and complete the browser login (`claude setup-token`)"
    expected: "A long-lived token is stored at ~/.config/gsd-meta-manager/driver-oauth-token (mode 0600); `driver auth status` then reports 'stored token' and the next launch uses it"
    why_human: "Browser OAuth login cannot be driven here. Not-stored state and host-token fallback ARE observed (`driver auth status` printed 'stored token: none ... next launch uses: the host access token'); token validation/storage is unit-tested (tests/launcher_auth.rs)."
  - test: "On a real macOS host (no bwrap, no systemd-run): `driver launch` without a flag, then with `--unsandboxed`"
    expected: "Refuses by default naming the missing pieces; with the flag runs unsandboxed, records `sandbox: degraded` on the run, and the Driver tab shows the badge"
    why_human: "No Mac available. The refuse/degrade logic is exercised on Linux with missing-tool shims (tests/launcher_degraded.rs, 16 pass) and the TUI badge is unit-tested (src/ui/screens/driver.rs:1181), but a real Darwin build and run is unobservable here."
---

# Phase 28: Sandboxed Driver Launcher Verification Report

**Phase Goal:** `gsd-meta-manager driver launch` starts a headless master inside a sandbox it cannot write out of, enforces the hard caps from outside, and pushes on the driver's behalf. On Linux this is the default. Hosts without bwrap or systemd-run get an explicit degraded mode.
**Verified:** 2026-10-20
**Status:** human_needed (all automated checks pass; two items are unobservable on this host)
**Re-verification:** No, initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Launch runs master in bwrap under `systemd-run --user --scope`; stop kills whole scope; second launcher refused by flock; relaunch logs `crash-detected` | VERIFIED | `src/launcher/sandbox.rs` `bwrap_argv`/`systemd_run_argv`. Re-run live this session: `launcher_live_scope` (`stop_kills_every_process_of_the_scope`: 5 pids incl. bwrap/claude/sleep all dead, scope inactive; `runtime_max_sec_backstop_stops_a_scope`: scope ended 3.3 s). Acceptance `check l5` PASS (second launcher rc 3; after SIGKILL lock free, `crash-detected` precedes launch n=2). |
| 2 | L1-L5 misbehaving-driver cases pass against the Rust launcher (dispatch cap, wall clock + RuntimeMaxSec, gap cap deny limit+1 / kill limit+2, rate-limit park) | VERIFIED (with caveat: l3 live FAIL carried, compensating evidence) | `check l1` PASS (deny at 3/3), `l2` PASS (cap-tripped wall_clock_min, scope gone), `l4` PASS (kill at count 4, 11 pids none survive), `l7` PASS (first deny after count 3, one cap-tripped gap_rounds used=4 limit=2, none before count 4 - this is exactly the limit+1 deny / limit+2 kill sequence, observed live). `check l3` re-derives **RESULT FAIL** (L3-2: no deny after count 3): `acceptance/l3/LOG.md` shows the Haiku driver exited rc=0 ("without RUN end") 1 s after count=3, so no dispatch was ever attempted - a driver-pacing fault in the harness, not a launcher fault. Gate logic itself: `src/launcher/gate.rs:56-87` `decide_dispatch` denies when `count > gap_limit` and the dispatch prompt mentions the phase; `run.rs:871-889` trips at `>= limit+2`; unit tests in both. Rate-limit park: `src/launcher/outcome.rs:213-235` + tests; not live-exercised (no live rate-limit case was in the campaign). [inferred - audit] |
| 3 | Sandbox: host `~/.claude` RO; creds and `~/.claude.json` unreadable; `~/.ssh` empty; SSH_AUTH_SOCK/GH_TOKEN/ANTHROPIC_API_KEY absent; run dir RO; per-run CLAUDE_CONFIG_DIR keeps hooks, no global CLAUDE.md | VERIFIED | Ran `launcher_live_isolation -- --ignored` this session (real bwrap): `claude_touch=EROFS`, `creds_read=1`, `claude_json_read=1`, `ssh_entries=0`, `env_ssh_auth_sock=` `env_gh_token=` `env_anthropic_api_key=` empty, `run_dir_touch=EROFS`, `lock_bytes=0`, `settings_touch=EROFS`, `bin_touch=EROFS`; inside `push --dry-run` over ssh and https both fail while host control over ssh succeeds. `tests/launcher_config_dir.rs` passes. |
| 4 | Auth: stored `claude setup-token` token as `CLAUDE_CODE_OAUTH_TOKEN`; guided onboarding; host-token fallback detects `OAuth token revoked` and relaunches; no refresh token in sandbox | VERIFIED (onboarding browser step human) | `src/launcher/auth.rs` (`resolve_launch_token`, `validate_pasted_token`, `store_token`), `outcome.rs:85,175-213` `AUTH_FAILURE_MARKERS`/`is_auth_failure`, `run.rs:728,761` `auth_relaunch`. `driver auth status` runs live (host-token fallback shown). `tests/launcher_auth.rs` 7 pass. |
| 5 | Commits leave only via gsd-mm push step outside sandbox: fixed refspec into reserved namespace, secret scan, protected-branch refusal, PR cap; push inside sandbox fails for lack of credentials | VERIFIED | `src/launcher/push.rs` (`dest_ref`, `check_destination`, `scan_with_external`, `pr_cap_*`, `push_run`), `driver push` subcommand exists. `tests/launcher_push.rs` 9 pass. Live: inside-sandbox push over ssh/https fails (see 3); host control push to `refs/heads/gsd-auto/...` dry-run succeeds. |
| 6 | Without bwrap/systemd-run (macOS) refuses by default; explicit opt-in degraded mode runs unsandboxed with process-supervision caps; posture recorded and shown in TUI; headless never needs tmux | VERIFIED on Linux (macOS run human) | `src/launcher/host.rs:152-190` `decide` (refuse unless `--unsandboxed`; no auto fallback). `tests/launcher_degraded.rs` 16 pass (refusal naming missing tools, degraded record, group kill, lock held, SIGTERM escalation). TUI badge `src/ui/screens/driver.rs:1181-1224`. No tmux argv anywhere (`launcher/mod.rs:36`, `process.rs:14`; stdin /dev/null). |

**Score:** 6/6 truths verified (0 behavior-unverified)

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| LNCH-01 | 28-01, 28-02, 28-07 | SATISFIED | SC1; live scope tests |
| LNCH-02 | 28-02, 28-09 | SATISFIED | l5 PASS (lock refusal rc 3, crash-detected) |
| LNCH-03 | 28-05, 28-06, 28-09 | SATISFIED (l3 caveat) | l1/l2/l4/l7 PASS; gate unit tests; see SC2 |
| LNCH-04 | 28-03, 28-09 | SATISFIED | live isolation probe; config_dir tests |
| LNCH-05 | 28-03, 28-05, 28-06 | SATISFIED (browser setup human) | auth.rs, outcome.rs, run.rs auth_relaunch |
| LNCH-06 | 28-03, 28-08 | SATISFIED | push.rs; launcher_push tests; live dry-run |
| LNCH-07 | 28-04, 28-07 | SATISFIED on Linux (macOS human) | host.rs; launcher_degraded |
| LNCH-08 | 28-01 | SATISFIED | no tmux on control path |
| DRIVE-01 (re-mapped) | 28-02, 28-09 | Launcher half SATISFIED; requirement remains Pending for Phase 29 | d33 PASS: real Opus driver, organic dispatch-denied at LOG line 72, injected 2nd gaps_found, `cap-tripped gap_rounds kill-scope`, 32 pids none survive, $9.00. REQUIREMENTS.md line 321 correctly keeps it Pending (master-prompt half is Phase 29). |

All 9 IDs declared across plans 28-01..28-09 match REQUIREMENTS.md; no orphaned Phase 28 IDs.

### Behavioral Spot-Checks (all read-only, no spend)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Acceptance verdicts re-derived | `bash scripts/launcher-acceptance.sh check <case>` for l1 l2 l3 l4 l5 l7 l8 d33 | l1 l2 l4 l5 l7 l8 d33 RESULT PASS; l3 RESULT FAIL | consistent with claims |
| Full suite | `cargo test --no-fail-fast` | 2425 lib pass / 1 fail (expected git-version witness: installed 2.53.0 vs pinned 2.55.0); every `tests/launcher_*` binary ok; 0 flakes this run | PASS |
| Live isolation / scope | `cargo test --test launcher_live_isolation --test launcher_live_scope -- --ignored` | 3/3 ok | PASS |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | PASS |
| CLI wiring | `driver --help`, `driver launch --help`, `driver auth status` | launch/stop/push/auth present and functional | PASS |

Campaign: $9.9996 of $20 (`acceptance/campaign.txt`).

### Anti-Patterns / Code Review

`28-REVIEW.md` records 12 findings, all `open` in the disposition (1 critical, 7 warning, 4 info). Judgment:

- **CR-01 (critical) `src/driverctl/serve.rs:402-413` `bind_private` flips the process-wide umask:** a real defect (interleaved binders can leave umask 0o177) and the credible cause of the bursty `PermissionDenied` test flakes. It does NOT contradict a Phase 28 success criterion: the review itself reports production exposure as latent (no other file-creating thread exists at bind time in `serve()` / `Session::start`), the sandbox, caps and push paths are unaffected, and the full suite was green apart from the expected witness on this run. Classified **WARNING, not a goal gap**; should be fixed (staging-dir + rename fix is in the review) before Phase 30 embeds `bind_run_socket` in the multithreaded TUI runtime, and it is a test-reliability hazard until then. [inferred - audit]
- **WR-02 (credential hiding is a denylist; network open)** and **WR-01 (no `--new-session`)** weaken the "cannot write out of" claim beyond what the measured probe covers (e.g. `~/.cargo/credentials.toml`, `AWS_*`, `~/.config/gsd-meta-manager/driver-oauth-token` are not masked). The success-criterion list is satisfied as written, but these are real hardening gaps worth triage; recommend a follow-up plan. WR-04 (`RERUN=1` deletes committed evidence before gates) is a harness hazard.
- No TBD/FIXME/XXX debt markers were introduced as blockers (not exhaustively scanned beyond launcher sources).

### Gaps Summary

No blocking gaps. One caveat: l3's live assertion (deny after count 3) was never observed; SC2 is met through l7 (same assertion, observed live) plus l4 (kill at limit+2) and the gate's unit tests. If strict case-for-case parity is wanted, closure is cheap: fix the l3 harness pacing (driver exhausted its 8 scripted reps 1 s after count=3) and rerun on Haiku for about $0.02.

### Human Verification Required

1. **`driver auth setup` browser login** - complete it; `driver auth status` must then show a stored token. Why human: browser OAuth.
2. **macOS degraded-mode run** - refuse by default, `--unsandboxed` degraded with badge. Why human: no Mac; Linux shim tests cover the logic only.

---

_Verified: 2026-10-20_
_Verifier: Claude (gsd-verifier)_
