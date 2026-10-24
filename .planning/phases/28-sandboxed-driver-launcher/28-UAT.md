---
status: complete
phase: 28-sandboxed-driver-launcher
source: [28-VERIFICATION.md]
started: 2026-10-20T00:47:15Z
updated: 2026-10-24T18:33:41.627Z
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
result: pass
source: agent-evidence
evidence: |
  Settled by the agent on 2026-10-24 on a real macOS host (human unavailable). [inferred — audit]
  Host: the local `docker-osx` VM (QEMU/KVM, Intel x86_64), macOS 26.6.2 (25G83), Darwin 25.6.0, reached as `ssh osx`; no bwrap, no
  systemd-run on PATH. HEAD b170d64 (`git archive`) built natively there with a throwaway rustup toolchain under /tmp
  (rustc 1.99.1, Xcode linker): `cargo build --locked` finished clean. Hermetic fixture mirroring tests/launcher_support (env -i,
  private HOME/XDG_RUNTIME_DIR, registry alias p28, a `claude` shim at 2.1.303 running the WELL_MANNERED / GROUP scenarios,
  host SSH_AUTH_SOCK/GH_TOKEN/ANTHROPIC_API_KEY set).
  - Token: `driver auth setup` stored at $HOME/Library/Application Support/gsd-meta-manager/driver-oauth-token (0600), rc 0.
  - No flag, foreground AND detached: rc 1, stderr "driver launch: sandbox unavailable: missing bwrap, systemd-run, user-systemd;
    rerun with --unsandboxed to launch without bwrap and a systemd scope (degraded)". No state.json, no LOG.md, driver never ran.
  - `--unsandboxed`: rc 0, "run r28 ended status=stopped reason=driver:stop"; LOG.md "RUN | launch | n=1 | mode=degraded";
    DIRECTION.md `sandbox: "degraded"`; state.json sandbox=degraded, hook_dispatches=1, launcher connection mode=degraded,
    scope_unit=null; driver env had no SSH_AUTH_SOCK/GH_TOKEN/ANTHROPIC_API_KEY and CLAUDE_CODE_OAUTH_TOKEN present.
  - Group supervision on Darwin: GROUP scenario r29, leader pid = pgid with a `sleep 600` child; `driver stop p28 r29` rc 0
    ("stopped the launcher ... the run is now stopped"); launcher rc 0, "ended status=stopped reason=stop:user"; leader and
    child both dead afterwards. (Push step refused no_recorded_branch: the fixture is not a git repo; expected.)
  - Driver tab badge: the real TUI in a 160x45 pty (GSDMM_EXPERIMENTAL_FEATURES=1, which gates the Driver tab), Enter then `D`:
    run rows "r28 !" and "r29 !" with the `!` posture cell in bold yellow (SGR 1 + 38;5;3, src/ui/screens/driver_run.rs:97,327),
    and the detail line "sandbox UNSANDBOXED missing: bwrap, systemd-run, user-systemd" (driver_run.rs:1054).
  - Linux regression on the current tree (2026-10-24): launcher_degraded 16/16, launcher_auth 11/11, posture/launch-screen 4/4.
  VM temp dir removed and the container stopped again afterwards.
  Residual [inferred — audit]: Apple Silicon (aarch64-apple-darwin) was not run, only built-checked by CI
  (.github/workflows/release.yml:65-97); the VM is Intel. The TUI-initiated [U] unsandboxed launch was not driven on Darwin
  (covered by driver_launch::tests::launcher_launch_a_degraded_host_* on Linux).

## Summary

total: 2
passed: 2
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none]

Note: the 2026-10-24 sandbox-isolation audit findings (open network, $HOME denylist, abstract sockets, planted files) are out of
scope for this UAT and owned by Phase 28.1 (.planning/phases/28.1-sandbox-egress-agent-channel-isolation/28.1-AUDIT-FINDINGS.md).
