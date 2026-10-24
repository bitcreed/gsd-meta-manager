---
status: partial
phase: 28-sandboxed-driver-launcher
source: [28-VERIFICATION.md]
started: 2026-10-20T00:47:15Z
updated: 2026-10-24T03:58:42.032Z
---

## Current Test

[testing complete]

## Tests

### 1. Run `gsd-meta-manager driver auth setup` and complete the browser login (`claude setup-token`)
expected: A long-lived token is stored at ~/.config/gsd-meta-manager/driver-oauth-token (mode 0600); `driver auth status` then reports 'stored token' and the next launch uses it
result: pass
source: agent-evidence
evidence: |
  Settled by the agent on 2026-10-23 (human unavailable). [INFERRED — audit]
  - Real binary (target/debug/gsd-meta-manager, v1.15.0) in an isolated HOME/XDG_CONFIG_HOME with a `claude` shim on PATH standing in for
    the browser step: `driver auth status` before -> "stored token: none ... next launch uses: none: run `gsd-meta-manager driver auth setup`";
    `printf '<sk-ant-oat01-...>\n' | driver auth setup` -> ran `claude setup-token`, prompted, "stored at .../driver-oauth-token (0600)", rc 0,
    value never echoed by gsd-mm; `stat` -> file 600, dir 700; `driver auth status` after -> "stored token: present ... (mode 0600)",
    "next launch uses: the stored setup-token".
  - "Next launch uses it": `cargo test --test launcher_auth` 11/11 pass, incl. stored_token_wins_over_the_host_token (real launch, the
    sandboxed fake driver records exactly one CLAUDE_CODE_OAUTH_TOKEN equal to the stored token; host access/refresh tokens and
    ANTHROPIC_AUTH_TOKEN appear nowhere), stored_token_with_group_bits_is_refused_at_launch, setup_refuses_an_unusable_paste_and_stores_nothing.
  - Code path: src/driver_cli.rs:89-179 (auth_setup), src/launcher/auth.rs:300-360 (store_token, O_CREAT mode 0o600), src/launcher/run.rs:1369.
  Residual (not a phase-28 behaviour): the browser OAuth inside Anthropic's own `claude setup-token` was not driven, since it needs the
  operator's account in a browser. Everything gsd-mm does before and after that step was observed. Optional operator smoke only.

### 2. On a real macOS host (no bwrap, no systemd-run): `driver launch` without a flag, then with `--unsandboxed`
expected: Refuses by default naming the missing pieces; with the flag runs unsandboxed, records `sandbox: degraded` on the run, and the Driver tab shows the badge
result: blocked
blocked_by: physical-device
reason: "Needs a real macOS (Darwin) host; none is available to the agent. Agent evidence for everything observable without one (2026-10-23): `cargo test --test launcher_degraded` 16/16 pass on Linux with bwrap/systemd-run hidden via shims (refusal naming the missing tool, the flag never implied by a failed probe, degraded posture recorded, group kill, lock held, TERM->KILL escalation); `cargo check --locked --target x86_64-apple-darwin --all-targets` clean locally (CI also checks + clippies aarch64/x86_64-apple-darwin, .github/workflows/release.yml:65-97); TUI degraded posture cell/launch-screen tests pass (ui::screens::driver_run::tests::the_posture_cell_marks_a_degraded_run_and_only_that, driver_launch::tests::launcher_launch_a_degraded_host_*). Unobserved: a real Darwin build linking and running (macOS process groups, no /proc, launchd environment)."

## Summary

total: 2
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 1

## Gaps

[none]
