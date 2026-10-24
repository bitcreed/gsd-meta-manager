---
phase: 28-sandboxed-driver-launcher
verified: 2026-10-23T12:00:00Z
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
  - "src/launcher/prompt.rs"
  - "src/launcher/push.rs"
  - "src/launcher/run.rs"
  - "src/launcher/sandbox.rs"
  - "src/launcher/stop.rs"
  - "src/ui/screens/driver_launch.rs"
  - "src/ui/screens/driver_run.rs"
  - "tests/launcher_acceptance.rs"
  - "tests/launcher_auth.rs"
  - "tests/launcher_caps.rs"
  - "tests/launcher_config_dir.rs"
  - "tests/launcher_degraded.rs"
  - "tests/launcher_e2e.rs"
  - "tests/launcher_live_isolation.rs"
  - "tests/launcher_live_scope.rs"
  - "tests/launcher_push.rs"
  - "tests/launcher_reconcile.rs"
  - "tests/launcher_sandbox.rs"
  - "tests/launcher_stop.rs"
covered_digest: "v3:sha256:07d0bf1fdfb3970b35dca3fc0485227b7f89b19eb88eee9c20fb2e2df9079ed4"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 6/6
  gaps_closed: []
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "On a real macOS host (no bwrap, no systemd-run): run `gsd-meta-manager driver launch` without a flag, then with `--unsandboxed`"
    expected: "Refuses by default naming the missing pieces; with the flag it runs unsandboxed, records `sandbox: degraded` on the run, and the Driver tab shows the badge"
    why_human: "Needs a physical Darwin host (28-UAT.md test 2, blocked: physical-device). Everything observable without one passes on Linux: tests/launcher_degraded.rs 16/16 with bwrap/systemd-run hidden by shims, TUI posture tests, and the cross-target check recorded in the UAT. Not observable: a real Darwin build linking and running (macOS process groups, no /proc, launchd environment)."
---

# Phase 28: Sandboxed Driver Launcher Verification Report

**Phase Goal:** `gsd-meta-manager driver launch` starts a headless master inside a sandbox it cannot write out of, enforces the hard caps from outside, and pushes on the driver's behalf. On Linux this is the default. Hosts without bwrap or systemd-run get an explicit degraded mode.
**Verified:** 2026-10-23
**Status:** human_needed (one item, a physical macOS host)
**Re-verification:** Yes. The prior report was stale because covered sources changed after it ran. Every truth was re-checked against the current tree and live evidence was re-run, not carried over.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Launch runs master in bwrap under `systemd-run --user --scope`; stop kills the whole scope; a second launcher is refused by the flock; relaunch logs `crash-detected` | VERIFIED | `run.rs:1349` builds `systemd_run_argv(unit, runtime_max, &bwrap_argv(&spec, &driver_argv))`. `sandbox.rs:263` `bwrap_argv`, `:334` `systemd_run_argv` (`--collect`). Live this session, `launcher_live_scope -- --ignored`: `stop_kills_every_process_of_the_scope` listed 5 cgroup pids (bwrap x2, claude, sleep x2), ran `driver stop`, state `inactive`, none of the 5 pids alive. `runtime_max_sec_backstop_stops_a_scope`: `RuntimeMaxSec=3` ended the scope in 3.20 s. `acceptance check l5` PASS (`crash-detected` at line 39 precedes launch n=2 at line 40). `tests/launcher_stop.rs` 5 pass. |
| 2 | L1-L5 misbehaving-driver cases pass against the Rust launcher: dispatch cap, wall clock + RuntimeMaxSec, gap cap deny limit+1 / kill limit+2, rate-limit park | VERIFIED | Re-derived from committed evidence with `scripts/launcher-acceptance.sh check`: l1, l2, l3, l4, l5, l7, l8, d33 all RESULT PASS. l3 now passes (deny after count 3 at line 41; none before count 3). l4: one `cap-tripped gap_rounds used=4 limit=2 kill-scope`, 11 pids none survive. l7: deny at count 3, trip at count 4. Gate logic: `gate.rs:71` `decide_dispatch`, unit test `the_gap_cap_allows_at_count_l_and_denies_at_l_plus_1`. `tests/launcher_caps.rs` 17 pass. Rate-limit park and auth-failure ordering are unit-tested in `outcome.rs` (`a_terminal_reason_naming_a_rate_limit_parks_with_an_unknown_window`, `an_auth_failure_is_decided_before_a_rate_limit`); no live rate-limit case was run in the campaign. A real rate-limit cannot be provoked on demand, and the verdict logic is a pure function exercised by tests. |
| 3 | Inside the sandbox: host `~/.claude` RO; `.credentials.json` and `~/.claude.json` unreadable; `~/.ssh` empty; `SSH_AUTH_SOCK`/`GH_TOKEN`/`ANTHROPIC_API_KEY` absent; run dir RO; per-run `CLAUDE_CONFIG_DIR` keeps hooks, no global CLAUDE.md | VERIFIED | Re-ran `launcher_live_isolation -- --ignored` under real bwrap this session: `claude_touch=EROFS`, `creds_read=1`, `claude_json_read=1`, `ssh_entries=0`, `gh_entries=0`, `env_ssh_auth_sock=` / `env_gh_token=` / `env_anthropic_api_key=` / `env_anthropic_auth_token=` (empty), `run_dir_touch=EROFS`, `lock_bytes=0`, `settings_touch=EROFS`, `bin_touch=EROFS`, `sibling_visible=absent`. Also masked: `npmrc`, `cargo credentials`, `~/.aws`, `~/.config/gcloud`. Inside `push --dry-run` fails over ssh and https while the host control over ssh succeeds. `tests/launcher_config_dir.rs` 11 pass. |
| 4 | Auth: stored `claude setup-token` token as `CLAUDE_CODE_OAUTH_TOKEN`; guided onboarding; host-token fallback detects `OAuth token revoked` and relaunches; no refresh token in sandbox | VERIFIED | `auth.rs` `resolve_launch_token` (called `run.rs:558`), `outcome.rs:175` `is_auth_failure`, `run.rs` `auth_relaunch`. `driver auth setup|status` wired at `driver_cli.rs:74-77`. `tests/launcher_auth.rs` 11 pass, including `stored_token_wins_over_the_host_token` (sandboxed fake driver records exactly one `CLAUDE_CODE_OAUTH_TOKEN` equal to the stored token; host access/refresh tokens appear nowhere) and `setup_refuses_an_unusable_paste_and_stores_nothing`. UAT test 1 PASSED on agent evidence (isolated HOME, `claude` shim, file mode 0600, dir 0700, value never echoed). Only the browser OAuth inside Anthropic's own `claude setup-token` was not driven; that is not gsd-mm behaviour. |
| 5 | Commits leave only via gsd-mm's push step outside the sandbox: fixed refspec into the reserved namespace, secret scan, protected-branch refusal, PR cap; push inside the sandbox fails for lack of credentials | VERIFIED | `push.rs:168` `dest_ref` -> `refs/heads/gsd-auto/<alias>/<run-id>`, `:193` `check_destination` (rejects sibling alias and prefix-collision `p280`), `:591` `scan_with_external`, `pr_cap_*`; `driver push` wired at `driver_cli.rs:65` -> `push_finished_run`. `tests/launcher_push.rs` 10 pass. Live: inside-sandbox push fails over ssh and https; host control dry-run to `gsd-auto/probe` succeeds. |
| 6 | Without bwrap/systemd-run (macOS) the launcher refuses by default; explicit opt-in degraded mode runs unsandboxed with process-supervision caps; posture recorded and shown in TUI; headless never needs tmux | VERIFIED on Linux (macOS run is the one human item) | `host.rs:185` `decide(caps, unsandboxed)`: refuses unless the flag is set, no auto fallback; called at `run.rs:579` and `:1261`. `tests/launcher_degraded.rs` 16/16 pass (refusal naming the missing tool, `the_flag_is_never_implied_by_a_failed_probe`, degraded posture recorded, group kill, lock held, TERM->KILL escalation). TUI: `driver_run::tests::the_posture_cell_marks_a_degraded_run_and_only_that` and `driver_launch::tests::launcher_launch_a_degraded_host_*` (3 tests) pass; `[U] launch UNSANDBOXED instead (asks again)` second confirmation at `driver_launch.rs:832`. No `tmux` argv on any control path (`launcher/mod.rs:36`, `process.rs:14`, `run.rs:542`; stdin `/dev/null`). |

**Score:** 6/6 truths verified (0 behavior-unverified). No behavior-dependent truth rests on presence alone: each cancellation/cleanup invariant (scope kill, cap trip, lock) has a passing live or named test above.

### Requirements Coverage

All nine IDs in the ROADMAP (LNCH-01..08, DRIVE-01) are declared across the PLAN frontmatter (28-01..28-09) and appear in REQUIREMENTS.md. No orphaned Phase 28 IDs.

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| LNCH-01 | 28-01, 28-02, 28-07 | SATISFIED | SC1; live scope tests |
| LNCH-02 | 28-02, 28-09 | SATISFIED | l5 PASS; `launcher_stop`/`launcher_e2e` |
| LNCH-03 | 28-05, 28-06, 28-09 | SATISFIED | l1/l2/l3/l4/l7 PASS; `launcher_caps` 17; gate/outcome unit tests |
| LNCH-04 | 28-03, 28-09 | SATISFIED | live isolation probe; `launcher_config_dir` 11 |
| LNCH-05 | 28-03, 28-05, 28-06 | SATISFIED | `launcher_auth` 11; UAT test 1 pass |
| LNCH-06 | 28-03, 28-08 | SATISFIED | `launcher_push` 10; live dry-run |
| LNCH-07 | 28-04, 28-07 | SATISFIED on Linux (macOS run human) | `launcher_degraded` 16; TUI posture tests |
| LNCH-08 | 28-01 | SATISFIED | no tmux on control path; detached log-file launch |
| DRIVE-01 (re-mapped) | 28-02, 28-09 | SATISFIED for the Phase 28 launcher half | d33 PASS: real Opus driver, injected second `gaps_found`, `cap-tripped gap_rounds kill-scope`, $9.00 (campaign total about $9.93 of $20). REQUIREMENTS.md already records the Phase 29 half as closed by operator sign-off. |

### Behavioral Spot-Checks (this session)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Live isolation and scope | `cargo test --test launcher_live_isolation --test launcher_live_scope -- --ignored --nocapture` | 3/3 ok (1.27 s + 3.24 s) | PASS |
| All launcher integration binaries | `cargo test --no-fail-fast --test launcher_{acceptance,auth,caps,config_dir,degraded,e2e,live_isolation,live_scope,push,reconcile,sandbox,stop}` | rc 0: 7 + 11 + 17 + 11 + 16 + 29 + 10 + 7 + 5 + 5 passed, 0 failed (live and one bwrap-gated test are `#[ignore]`d in a plain run and were run live above) | PASS |
| Launcher unit tests | `cargo test --lib -- launcher::outcome launcher::gate launcher::host` | 52 passed, 0 failed | PASS |
| TUI posture | `cargo test --lib -- the_posture_cell_marks_a_degraded_run_and_only_that launcher_launch_a_degraded_host` | 4 passed | PASS |
| Acceptance verdicts re-derived | `bash scripts/launcher-acceptance.sh check <case>` for l1 l2 l3 l4 l5 l7 l8 d33 | all RESULT PASS | PASS |
| Targeted regression gate (earlier this session, orchestrator) | 44 integration binaries | 496 passed, 0 failed, 17 ignored | PASS |

The full `cargo test` was deliberately not run (another agent is running timing-sensitive stress tests). The known expected local failure is the git-version witness in `src/envelope/policy.rs` (installed git differs from the pinned 2.55.0); it is not a regression.

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes are declared by the plans. The acceptance harness (`scripts/launcher-acceptance.sh check`) is the phase's probe equivalent and was re-run above.

### Anti-Patterns / Code Review / Security

- Debt-marker scan (`TBD|FIXME|XXX|todo!|unimplemented!`) over `src/launcher/*.rs`, `src/driver_cli.rs`, `scripts/launcher-acceptance.sh`, `tests/launcher_*.rs`: none.
- `28-REVIEW.md` (re-run): 0 critical, 6 warning, 4 info, all advisory. The prior critical (CR-01 process-wide umask in `bind_private`) and WR-07 are `fixed` in the disposition. The remaining warnings are hardening and polish (credential hiding is still a denylist with the network open, an unbounded `DIRECTION.md` read, a symlinked-drivers-dir tmpfs skip, a `bind_private` replace race, half-applied macOS enablement in the TUI). None contradicts a success criterion as written; they are worth triage but are not goal gaps. WR-06 is the same reduced hardening note as before and the live probe measures the criterion list as satisfied.
- `28-SECURITY.md`: `threats_open: 0`, status verified.
- `28-UI-REVIEW.md`: 18/24, no blockers.
- Working tree note: `src/driver/direction.rs` is modified and `tests/driverctl_race_probe.rs` is untracked. Both belong to another agent's in-flight quick task and are outside this phase's covered files.

### Deferred Items

None.

### Human Verification Required

#### 1. macOS degraded-mode run

**Test:** On a real macOS host with neither bwrap nor systemd-run, run `gsd-meta-manager driver launch` without a flag, then with `--unsandboxed`.
**Expected:** The first refuses and names the missing pieces. The second runs unsandboxed, records `sandbox: degraded` on the run, and the Driver tab shows the badge.
**Why human:** Needs a physical Darwin host (28-UAT.md test 2, `blocked_by: physical-device`). The refuse/degrade logic, process-group supervision and TUI badge are all proven on Linux through shims and unit tests, but a real Darwin build and run is unobservable here.

(The earlier `driver auth setup` item is settled: UAT test 1 passed on agent evidence, see truth 4.)

### Gaps Summary

No gaps. All six roadmap success criteria and all nine requirement IDs are backed by evidence from this session, including live bwrap isolation and live systemd scope kill. The single open item is a real macOS run, which cannot be done without a physical device.

---

_Verified: 2026-10-23_
_Verifier: Claude (gsd-verifier)_
