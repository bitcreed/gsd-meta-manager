---
phase: 28-sandboxed-driver-launcher
verified: 2026-10-28T12:00:00Z
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
covered_digest: "v4:sha256:425a1c65e6c6bf544a56fff3004eb20ebe352cfc5407e63ea87b563a3713425a"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 6/6
  gaps_closed: []
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "WR-08: a malformed host managed `hooks`/`permissions` value (non-object) replaces the launcher's gate in managed settings (config_dir.rs managed_settings)"
    category: security
    reason: "Requires a malformed admin-owned managed-settings file on the host; Claude Code itself refuses such a file. Recorded open in 28-REVIEW-DISPOSITION.md, not a gap against any success criterion. [inferred — cleanup 2026-10-28]"
    evidence_status: "reviewer read the code, did not run it"
  - finding: "WR-09: a launcher binary under /tmp or /run is unreachable inside the sandbox (snapshot_exe)"
    category: other
    reason: "Fail-closed robustness defect (bwrap exec 127), not an exposure. [inferred — cleanup 2026-10-28]"
    evidence_status: "reviewer read the code, did not run it"
  - finding: "IN-05 (UTS namespace does not hide /etc/hostname, machine-id) and IN-06 (failed record_launch_posture leaves launch half recorded)"
    category: other
    reason: "Info-level; recorded open. [inferred — cleanup 2026-10-28]"
    evidence_status: "reviewer read the code, did not run it"
optional_human_items:
  - test: "Real Apple Silicon (aarch64-apple-darwin) run of the degraded launcher"
    why_optional: "Hardware coverage only. The Darwin behaviour is already proven on a real macOS 26.6.2 VM (Intel) and CI check+clippy builds aarch64-apple-darwin. Non-blocking."
  - test: "Anthropic's own browser OAuth inside `claude setup-token` (UAT 1 remainder)"
    why_optional: "Not gsd-mm behaviour; same non-blocking class as Phase 28.1 D-11. Deferred."
---

# Phase 28: Sandboxed Driver Launcher Verification Report

**Phase Goal:** `gsd-meta-manager driver launch` starts a headless master inside a sandbox it cannot write out of, enforces the hard caps from outside, and pushes on the driver's behalf. On Linux this is the default. Hosts without bwrap or systemd-run get an explicit degraded mode.
**Verified:** 2026-10-28 at HEAD 64bf15e
**Status:** passed
**Re-verification:** Yes. The prior report (2026-10-23) went stale: Phase 28.1, Phase 31 deletions and quick batch 261028-n0u/n11/n12 touched covered files. Every truth was re-checked against the current tree and live evidence was re-run in this session.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Master runs in bwrap under `systemd-run --user --scope`; stop kills the whole scope; a second launcher is refused by the flock; relaunch logs `crash-detected` | VERIFIED | `run.rs:1597` `systemd_run_argv(unit, runtime_max, &bwrap_argv(&spec, &bridged))`; `sandbox.rs:506` `bwrap_argv`, `:681` `systemd_run_argv`. Live this session (`launcher_live_scope -- --ignored`): `stop_kills_every_process_of_the_scope` listed 6 cgroup pids, `driver stop` left the scope `inactive`, none of the 6 alive; `runtime_max_sec_backstop_stops_a_scope` ok (3.14 s). `acceptance check l5` RESULT PASS. `launcher_stop` 5 pass, `launcher_e2e` 26 pass. |
| 2 | L1-L5 misbehaving-driver cases pass: dispatch cap, wall clock + RuntimeMaxSec, gap cap deny limit+1 / kill limit+2, rate-limit park | VERIFIED | `scripts/launcher-acceptance.sh check` for l1 l2 l3 l4 l5 l7 l8 d33: all RESULT PASS (re-run this session from committed evidence). `gate.rs:71` `decide_dispatch` (unit tests in gate.rs), `outcome.rs:200` `decide`. `launcher_caps` 17 pass. Rate-limit park is a pure function exercised by `outcome.rs` unit tests; a real rate limit cannot be provoked on demand. |
| 3 | Inside the sandbox: host `~/.claude` RO; `.credentials.json` and `~/.claude.json` unreadable; `~/.ssh` empty; `SSH_AUTH_SOCK`/`GH_TOKEN`/`ANTHROPIC_API_KEY` absent; run dir RO; per-run `CLAUDE_CONFIG_DIR` | VERIFIED | Re-ran `launcher_live_isolation -- --ignored` under real bwrap this session (2/2 ok, 52.9 s): `creds_read=absent`, `claude_json_read=absent`, `ssh_entries=0`, `gh_entries=absent`, `npmrc/cargo_credentials/git_credentials/netrc=absent`, `env_ssh_auth_sock=`, `env_gh_token=`, `env_anthropic_api_key=`, `env_anthropic_auth_token=` (all empty), `run_dir_touch=EROFS`, `settings_touch=EROFS`, `bin_touch=EROFS`, `claude_agents_touch=EROFS`, `lock_bytes=0`, `sibling_visible=absent`. Phase 28.1 tightened this (`--unshare-net/ipc/uts`, tmpfs `/run` and `$HOME` allowlist); the 28 criteria still hold and are now stronger. `launcher_config_dir` 26 pass. |
| 4 | Auth: stored `claude setup-token` token as `CLAUDE_CODE_OAUTH_TOKEN`; guided onboarding; host-token fallback detects `OAuth token revoked` and relaunches; no refresh token in sandbox | VERIFIED | `auth.rs:228` `resolve_launch_token` (called `run.rs:638`), `outcome.rs:175` `is_auth_failure`, `run.rs:1132` `auth_relaunch`. `launcher_auth` 13 pass incl. `stored_token_wins_over_the_host_token`. UAT 1 passed on agent evidence (isolated HOME, `claude` shim, mode 0600). Residual browser OAuth is Anthropic's, not gsd-mm's (non-blocking, deferred). |
| 5 | Commits leave only via gsd-mm's push step outside the sandbox: fixed refspec into the reserved namespace, secret scan, protected-branch refusal, PR cap; push inside the sandbox fails | VERIFIED | `push.rs:178` `dest_ref`, `:203` `check_destination`, `:918` `push_finished_run` (called `driver_cli.rs:532`). `launcher_push` 15 pass (1 ignored is not part of this criterion). Live isolation: no credentials, no network (`--unshare-net`), so an in-sandbox push cannot leave. |
| 6 | Without bwrap/systemd-run (macOS) the launcher refuses by default; explicit opt-in degraded mode runs unsandboxed with process-supervision caps; posture recorded and shown in TUI; headless needs no tmux | VERIFIED | `host.rs:286` `decide(caps, unsandboxed)` refuses unless the flag is set (called `run.rs:660`, `:1500`). `launcher_degraded` 17 pass incl. `the_flag_is_never_implied_by_a_failed_probe`. TUI posture and `[U]` second confirmation tests pass. No tmux on any control path (only doc mentions in `mod.rs`, `process.rs`, `run.rs`). **Real Darwin run: SETTLED by agent evidence**, 28-UAT.md test 2 on a docker-osx macOS 26.6.2 VM (Darwin 25.6.0), run twice (4563fd5/5797842/d1ff8e6, re-run on 2ea7d77) and again after 261028-n0u/n11 (HEAD 9b68124): no flag refuses naming `bwrap, systemd-run, user-systemd`; `--unsandboxed` records `mode=degraded`, group kill on `driver stop` verified, TUI badge and, after n11, the TUI launch/steer/stop path end to end. CI `cross-target-check` (`.github/workflows/release.yml:65-97`) check+clippy `aarch64-apple-darwin`. |

**Score:** 6/6 truths verified (0 behavior-unverified). Each cancellation/cleanup invariant (scope kill, cap trip, lock, group kill) has a passing live or named test.

### Requirements Coverage

All nine IDs in the ROADMAP (LNCH-01..08, DRIVE-01) are declared in the PLAN frontmatter (28-01..28-09) and are `[x]`/Complete in REQUIREMENTS.md (traceability rows 385-392, DRIVE-01 re-mapped Phase 28/29). No orphaned Phase 28 IDs. LNCH-09..15 belong to Phase 28.1 and are out of scope here.

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| LNCH-01 | 28-01, 28-02, 28-07 | SATISFIED | SC1; live scope tests |
| LNCH-02 | 28-02, 28-09 | SATISFIED | l5 PASS; `launcher_stop`, `launcher_e2e` |
| LNCH-03 | 28-05, 28-06, 28-09 | SATISFIED | l1/l2/l3/l4/l7 PASS; `launcher_caps` 17 |
| LNCH-04 | 28-03, 28-09 | SATISFIED | live isolation probe; `launcher_config_dir` |
| LNCH-05 | 28-03, 28-05, 28-06 | SATISFIED | `launcher_auth` 13; UAT 1 |
| LNCH-06 | 28-03, 28-08 | SATISFIED | `launcher_push` 15; SC5 |
| LNCH-07 | 28-04, 28-07 | SATISFIED | `launcher_degraded` 17; UAT 2 real macOS VM |
| LNCH-08 | 28-01 | SATISFIED | no tmux on control path; detached log-file launch |
| DRIVE-01 (re-mapped) | 28-02, 28-09 | SATISFIED (launcher half) | d33 PASS: real Opus driver, injected second `gaps_found`, `cap-tripped gap_rounds kill-scope`. Phase 29 half closed by operator sign-off per REQUIREMENTS.md. |

### Behavioral Spot-Checks and Regression

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full regression at a69de7a (orchestrator-supplied; `git diff a69de7a HEAD -- src tests scripts Cargo.toml` is empty, so it covers HEAD) | `rtk proxy cargo test --no-fail-fast` | 3183 passed, 0 failed, 17 ignored, 55 suites | PASS |
| Live isolation + scope (this session) | `cargo test --test launcher_live_isolation --test launcher_live_scope -- --ignored` | 2/2 + 2/2 ok | PASS |
| Launcher integration binaries (this session) | acceptance, auth, caps, config_dir, degraded, e2e, push, reconcile, sandbox, stop | 7, 13, 18, 26, 17, 41, 15, 7, 15, 5 passed; 0 failed | PASS |
| Acceptance verdicts re-derived | `launcher-acceptance.sh check` l1 l2 l3 l4 l5 l7 l8 d33 | all RESULT PASS | PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes are declared by the plans. `scripts/launcher-acceptance.sh check` is the phase's probe equivalent and was run above.

### Anti-Patterns / Code Review / Security

- `28-REVIEW.md` re-review (80b200e): 0 critical, 4 warning, 2 info. Disposition (64bf15e): WR-02 and WR-04 deferred to todos (hardening / protocol-wording design choices, neither contradicts a success criterion); WR-05 fixed by 261028-n0u/n11; the 4 new findings WR-08, WR-09, IN-05, IN-06 are open advisories. Checked against must-haves: WR-08 needs a malformed admin-owned managed-settings file and does not affect any tested criterion; WR-09 is fail-closed; IN-05/IN-06 are cosmetic or consistency. None breaks a must-have, so none is a gap. They are listed under `advisory`.
- `28-SECURITY.md` (8852f92): `threats_open: 0`, 36/36 closed.
- `28-UI-REVIEW.md` (03107c1): 18/24, unchanged, no blockers.
- Debt markers: none blocking; `tmux` appears only in doc comments.

### Deferred Items

None.

### Human Verification

No blocking items; status is `passed`. Optional, non-blocking:

1. **Apple Silicon run** (hardware coverage only). Darwin behaviour is proven on a real macOS 26.6.2 VM and CI builds and lints `aarch64-apple-darwin`.
2. **Anthropic browser OAuth inside `claude setup-token`** (UAT 1 remainder). Not gsd-mm behaviour; same non-blocking class as Phase 28.1 D-11, deferred.

### Gaps Summary

No gaps. All six roadmap success criteria and all nine requirement IDs are backed by evidence from this session, including live bwrap isolation and live systemd scope kill, plus the Darwin degraded-mode run settled by agent evidence.

---

_Verified: 2026-10-28T12:00:00Z_
_Verifier: Claude (gsd-verifier)_
