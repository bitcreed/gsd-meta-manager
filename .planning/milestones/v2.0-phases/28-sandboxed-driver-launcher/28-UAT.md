---
status: complete
phase: 28-sandboxed-driver-launcher
source: [28-VERIFICATION.md]
started: 2026-10-20T00:47:15Z
updated: 2026-10-27T14:45:00.000Z
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

  Re-run 2026-10-27 on HEAD 2ea7d77 (after Phases 28.1, 29, 30, 31), same host and method. [inferred — audit]
  `git archive HEAD` built natively on the VM with a throwaway rustup (rustc 1.99.1): `cargo build --locked` rc 0,
  `--version` 1.15.0. Same hermetic fixture (env -i, private HOME/XDG_*, alias p28, `claude` shim 2.1.303, host
  SSH_AUTH_SOCK/GH_TOKEN/ANTHROPIC_API_KEY set). Token: `driver auth setup` stored at
  $HOME/Library/Application Support/gsd-meta-manager/driver-oauth-token, -rw-------, rc 0.
  - No flag, foreground AND detached: rc 1, same "sandbox unavailable: missing bwrap, systemd-run, user-systemd; rerun with
    --unsandboxed ..." stderr; run dir still holds only DIRECTION.md (byte-identical), the driver never ran.
  - `--unsandboxed --foreground` (WELL_MANNERED, r28): rc 0, "ended status=stopped reason=driver:stop"; LOG.md
    "RUN | launch | n=1 | mode=degraded"; DIRECTION.md `sandbox: "degraded"`, `sandbox_missing` the three pieces,
    `protections_absent` all eight words (28.1-09 vocabulary, net-isolation ... registry-overlay); state.json the same,
    hook_dispatches=1, launcher connection mode=degraded, scope_unit=null. 28.1 egress on Darwin degraded: no proxy is
    started (no egress.log or egress socket anywhere, no *_PROXY in the driver env, no `egress_allow` recorded), which
    is the documented behaviour (src/launcher/host.rs:262-270, src/launcher/run.rs:964-966) and is what
    `egress-proxy`/`net-isolation` in protections_absent declare. Driver env: no SSH_AUTH_SOCK/GH_TOKEN/ANTHROPIC_API_KEY,
    CLAUDE_CODE_OAUTH_TOKEN present (also re-checked on the detached run below: 0 hits).
  - Group supervision, now launched DETACHED (`--unsandboxed`, no --foreground, GROUP scenario r29): "run r29 started
    (pid 2171)"; leader 2186 = pgid 2186 with `sleep 600` 2192 in that group; `driver stop p28 r29` rc 0 "stopped the
    launcher (pid 2171); the run is now stopped"; leader and child dead, no launcher left; LOG.md
    "launch-end | status=stopped | reason=stop:user".
  - TUI in a 160x45 pty (GSDMM_EXPERIMENTAL_FEATURES=1), Enter then `D`: rows "▮ stopped r29!" / "r28!", the `!` bold
    yellow (pyte: fg cdcd00, bold); detail "sandbox UNSANDBOXED · absent: all 8 · missing: bwrap, systemd-run,
    user-systemd". Dashboard smoke without the flag: renders the p28 row, `q` exits 0 and restores the terminal
    (?1049l, mouse modes off, ?25h).
  - [U] residual: NOT closable on Darwin; it is unreachable there, not untested. On macOS `R` and `s` on the Driver tab
    only post "read-only: steering needs Linux" (observed), because can_steer() is cfg!(target_os = "linux")
    (src/ui/screens/driver_steer.rs:43-45), checked at src/ui/screens/detail.rs:5846 (`s`) and :5874 (`R`); and even a
    dispatched DriverLaunchRequested is refused off Linux (src/app.rs:2488-2494). So the TUI launch, and with it the
    `[U] launch UNSANDBOXED instead (asks again)` confirmation, exists only on a Linux host lacking bwrap/systemd. This is
    the open 28-REVIEW WR-05 ("macOS enablement is half applied"), not a regression; the CLI path above is the only
    degraded launch on macOS. Closing the residual needs WR-05 fixed first (a product-scope call: widen the TUI launch to
    macOS or declare it Linux-only). Minor, same finding: the launcher connection records `host: null` on Darwin.
  VM temp dir removed afterwards; the VM was left running.

  Re-check 2026-10-28 after 261028-n11 (WR-05 enabled on macOS) [inferred — cleanup 2026-10-28]
  Same docker-osx VM (macOS 26.6.2, Darwin 25.6.0, Intel), HEAD 9b68124 (`git archive`), throwaway rustup under /tmp
  (rustc 1.99.1). `cargo build --locked` rc 0; `cargo test --locked --no-fail-fast --lib`: 2522 passed, 0 failed, 1 ignored.
  A first Darwin run found 5 failures, all test-side and fixed in 9b68124: four `driver::liveness` tests read /proc (now
  Linux-only) and `launcher_launch_r_on_a_new_run_opens_the_form_with_the_defaults` built its event path from the canonical
  `/private/var` path while the project path was `/var` (the test now builds the path from the project path).
  Real TUI in a 160x45 pty driven with pyte (hermetic fixture as above; `$EDITOR` a shim that writes a lint-clean body;
  fake `claude` scenario that logs start and waits), Enter, `D`, `s`:
  - Empty state shows "Press [s] to start one ..." (no "needs Linux"); slug `e2e` creates 20261028-e2e, the editor shim
    runs, the check passes and the launch form opens with the preflight: "Sandbox unavailable: missing bwrap, systemd-run,
    user-systemd. A sandboxed launch is refused on this host." and "[U] launch UNSANDBOXED instead (asks again)".
  - `y`: "Launch refused: this host cannot sandbox the driver (missing: ...). Press [U] to launch it unsandboxed instead."
    `U`: the "Launch UNSANDBOXED" second confirmation naming the eight absent protections. `y`: run started, row
    "running 20261028-e2e!", detail "sandbox UNSANDBOXED", LOG.md "RUN | launch | n=1 | mode=degraded"; DIRECTION.md
    `sandbox: "degraded"`.
  - state.json connections: launcher mode=degraded and driver mode=headless, both with `host: "docker-osx.local"` (was
    null before the uname() change).
  - Steer: `c` then `p` posts "control: pause written to DIRECTION.md"; the body line changed from `- control: run` to
    `- control: pause`; the detail shows "control pause (pending)".
  - Stop: `x` opens "Kill driver run", `y`: row "stopped", LOG.md "RUN | launch-end | status=stopped | reason=stop:user";
    no launcher or scenario process left.
  - `R` (relaunch) was not separately driven: a stopped run shows `[R]elaunch` in the footer; it shares the launch form,
    preflight and `[U]` path exercised through `s`.
  The "[U] residual" above is now CLOSED on Darwin for the Intel x86_64 host. Apple Silicon (aarch64-apple-darwin) is
  still clippy-checked only. VM temp dirs removed and the VM stopped again (it was inactive at the start).

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
