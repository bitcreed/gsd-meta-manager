---
phase: quick-261020-vxg
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/launcher/host.rs
  - src/launcher/mod.rs
  - src/launcher/prompt.rs
  - src/launcher/sandbox.rs
  - src/driverctl/mod.rs
  - src/driverctl/scan.rs
  - src/driverctl_cli.rs
  - src/driver_cli.rs
  - src/driver_templates/mod.rs
  - src/driver/direction.rs
  - src/driver/liveness.rs
  - src/driver/mod.rs
  - src/state_reader/config_write.rs
  - src/cli.rs
  - src/experimental.rs
  - src/ui/screens/driver_start.rs
  - .github/workflows/release.yml
  - README.md
autonomous: true
requirements: [QUICK-261020-vxg]

estimate:
  tokens: 90000
  raw_tokens: 90000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "`cargo check --target x86_64-apple-darwin --all-targets --locked` exits 0, and `cargo clippy --target x86_64-apple-darwin --lib --bins --locked -- -D warnings` exits 0."
    - "On macOS, `driver launch` without `--unsandboxed` is refused with SandboxUnavailable naming bwrap and systemd-run, and the sandboxed (BwrapScope) posture cannot be reached off Linux whatever is on PATH. With `--unsandboxed` it goes down the real degraded launcher path, no longer the old 'run store needs Linux' refusal."
    - "`cargo check --target x86_64-pc-windows-gnu --lib --bins --locked` exits 0. On Windows the features refuse at runtime: a config save returns ErrorKind::Unsupported before any I/O, the verification scan skips files, and the driver subcommands report an unsupported platform."
    - "Linux behaves exactly as before. The full `cargo test --no-fail-fast` passes the baseline count plus the new host.rs tests, and its only failure is the known envelope/policy.rs git-version witness. `cargo clippy --all-targets -- -D warnings` is clean. src/launcher/sandbox.rs has zero deleted lines in the diff."
    - "release.yml has a cross-target-check job that runs `cargo check` (and clippy -D warnings on lib+bins) for the macOS and Windows targets, and the publish job lists it under `needs`."
  artifacts:
    - path: "src/launcher/host.rs"
      provides: "SANDBOX_SUPPORTED const and off-platform missing list; detect() forces bwrap/systemd-run/user-systemd missing off Linux"
      contains: "SANDBOX_SUPPORTED"
    - path: "src/state_reader/config_write.rs"
      provides: "cfg(unix) lock-aware writer plus a cfg(not(unix)) update_gsd_config/atomic_write that refuse with Unsupported"
      contains: "ErrorKind::Unsupported"
    - path: ".github/workflows/release.yml"
      provides: "cross-target-check matrix job gating publish"
      contains: "cross-target-check"
  key_links:
    - from: "src/launcher/host.rs detect()"
      to: "src/launcher/host.rs decide()"
      via: "off Linux, caps.missing is never empty, so decide(_, false) always returns SandboxUnavailable"
      pattern: "SANDBOX_SUPPORTED"
    - from: "src/driver_cli.rs launch_main (linux|macos arm)"
      to: "src/launcher/run.rs"
      via: "cfg(any(target_os = \"linux\", target_os = \"macos\")) on the launcher module tree"
      pattern: "target_os = \"macos\""
    - from: "src/ui/screens/detail.rs Defaults-tab persist"
      to: "src/state_reader/config_write.rs update_gsd_config"
      via: "same signature on every platform; detail.rs unchanged"
      pattern: "pub fn update_gsd_config"
    - from: ".github/workflows/release.yml publish.needs"
      to: ".github/workflows/release.yml jobs.cross-target-check"
      via: "needs list"
      pattern: "cross-target-check"
---

<objective>
The crate builds on Linux only. This plan makes it build for macOS and Windows and adds a CI check so it stays that way.

Operator goals, cited below as G1 to G3:
- **G1, macOS.** `cargo check` and `cargo clippy -D warnings` pass for the darwin target. Linux-only code stays gated. The degraded (`--unsandboxed`) launcher path is what compiles and runs on macOS. The operator's Phase 28 UAT item 2 (`.planning/phases/28-sandboxed-driver-launcher/28-UAT.md:23`) needs a macOS `driver launch` that refuses by default and runs with `--unsandboxed`.
- **G2, Windows.** At least `cargo check` passes. Unix-only code is gated with `cfg(unix)`. On Windows the features refuse at runtime and nothing is implemented for them.
- **G3, CI.** Cross-target `cargo check`, plus clippy where it is cheap, in the existing `.github/workflows/release.yml`.

Measured at planning (HEAD 8035409):
- **macOS:** two errors.
  - `src/launcher/sandbox.rs:233`: `super::auth` is gated to Linux.
  - `src/driverctl/scan.rs:344`: `rustix::fs::mknodat` is absent on Apple. This one is test-only.
- **Windows (`--lib`):** 25 errors.
  - `src/state_reader/config_write.rs` accounts for most of them (std::os::unix, mode, custom_flags, from_mode, dev/ino, rustix fs/process, unix Errno constants).
  - `src/driverctl/scan.rs`.
  - `src/launcher/sandbox.rs:233`.
- **Windows (`--all-targets`):** the 25 above, plus test-only errors in:
  - `src/driverctl/scan.rs` tests
  - `src/driver/liveness.rs` tests (`process_group`, `arg0`, `std::os::unix`)
  - `src/cli.rs:908`, `src/experimental.rs:144` and `src/ui/screens/driver_start.rs:592` (all call `driver::spawn`, which is cfg(unix))
  - `src/state_reader/config_write.rs` tests
  - plus a Windows-only unused-variable warning at `src/driver/mod.rs:1210`.
- **rustix:** it compiled for x86_64-pc-windows-gnu here, without its fs/process modules. The operator's measured "rustix lib.rs errors" did not reproduce, so `Cargo.toml` stays unchanged `[inferred — audit]`.

A measurement experiment (since reverted) widened the launcher stack gates from Linux to Linux+macOS. The files were src/launcher/mod.rs, src/launcher/prompt.rs, src/driverctl/mod.rs, src/driverctl_cli.rs, src/driver_cli.rs, src/driver_templates/mod.rs and src/driver/direction.rs, plus a Linux gate on the scan.rs FIFO test and its helper. The results:
- `cargo check --target x86_64-apple-darwin --all-targets` was clean apart from two dead-code warnings, at `src/app.rs:8460` and `src/ui/screens/detail.rs:20043`. Those are test helpers that another agent is fixing on master. DO NOT TOUCH THEM.
- `cargo clippy --target x86_64-apple-darwin --lib --bins -- -D warnings` was clean.
- rustix 1.1.5 maps `RenameFlags::EXCHANGE` to `renameatx_np(RENAME_SWAP)` on Apple (`rustix-1.1.5/src/backend/libc/fs/types.rs:545`). That is why direction.rs's exchange splice, and with it the run store, compiles on macOS. It is also why the "no renamex_np splice yet" refusal is obsolete there.

Purpose: macOS and Windows users can `cargo install` again. The operator can run the Phase 28 macOS UAT. CI catches the next non-Linux break before a publish.

Output:
- macOS and Windows compile fixes. On Linux every cfg predicate stays true, so Linux compiles the same code as before.
- A fail-closed host probe for platforms other than Linux.
- A cross-target CI job.
- A short README note on Windows.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@src/launcher/host.rs
@src/launcher/mod.rs
@src/driver_cli.rs
@src/state_reader/config_write.rs
@.github/workflows/release.yml

Hard constraints (operator):
- Do NOT touch the unused test helpers at src/app.rs:8460 and src/ui/screens/detail.rs:20043. Make zero edits to src/app.rs and src/ui/screens/detail.rs.
- The Linux sandbox must stay behaviourally identical. Off Linux, a sandboxed launch must fail closed unless `--unsandboxed` is given.
- No push, no rebase, no force-push. Commit on the current worktree branch.
- Use `rtk proxy` for any output you count or grep. To keep a pipeline raw, wrap it in `rtk proxy sh -c '...'` or read the log file directly with `rtk proxy awk`.
- No `#[allow(dead_code)]`-style blanket suppression. Fix cfg-induced dead code by giving the helper the same cfg as its only user.
- Gate predicates:
  - Launcher stack (Linux+macOS): `any(target_os = "linux", target_os = "macos")`, and its negation `not(any(target_os = "linux", target_os = "macos"))`.
  - Unix-API code needed for Windows: `unix` / `not(unix)`.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: macOS end-to-end: fail-closed host probe, launcher stack widened to Linux+macOS, darwin check and clippy green</name>
  <files>src/launcher/host.rs, src/launcher/mod.rs, src/launcher/prompt.rs, src/driverctl/mod.rs, src/driverctl_cli.rs, src/driver_cli.rs, src/driver_templates/mod.rs, src/driver/direction.rs, src/driverctl/scan.rs</files>
  <precondition>`rustup target list --installed` lists x86_64-apple-darwin and x86_64-pc-windows-gnu.</precondition>
  <behavior>
    - Test 1: `SANDBOX_SUPPORTED == cfg!(target_os = "linux")`.
    - Test 2: `decide(&HostCaps { missing: off_platform_missing(), claude_version: Some(MIN_CLAUDE_VERSION) }, false)` is `Err(LauncherError::SandboxUnavailable { missing })`, where `missing` contains "bwrap" and "systemd-run". This is the macOS refuse-by-default contract, proven on Linux.
    - Test 3: the same caps with `unsandboxed = true` give `Ok(Posture::Degraded { .. })`, and `as_str()` is "degraded".
    - Test 4: `off_platform_missing()` is non-empty and equals the three MISSING_* constants in probe order (bwrap, systemd-run, user-systemd). Because the list is never empty, BwrapScope is unreachable off Linux.
  </behavior>
  <action>
**Step 0: baseline.** Before ANY edit, record the Linux baseline:
- Run `mkdir -p -m700 /tmp/claude-1000/t; umask 077; TMPDIR=/tmp/claude-1000/t rtk proxy cargo test --no-fail-fast > /tmp/claude-1000/t/linux-before.log 2>&1`.
- Sum it with `rtk proxy awk '/^test result:/ {p+=$4; f+=$6} END {print p, f}' /tmp/claude-1000/t/linux-before.log`.
- Note the name of the single expected failure (the envelope/policy.rs git-version witness).
- Record BASE=$(git rev-parse HEAD).

**Step 1: the host.rs fail-closed guard (G1, security).** Write it test-first.
- In src/launcher/host.rs, add a public const `SANDBOX_SUPPORTED: bool = cfg!(target_os = "linux")`. Model it on `LIVENESS_SUPPORTED` in src/driver/liveness.rs:56: a const rather than a cfg block, so both arms are compiled and testable on Linux.
- Add a pub fn `off_platform_missing() -> Vec<String>` that returns MISSING_BWRAP, MISSING_SYSTEMD_RUN and MISSING_USER_MANAGER, in that order.
- In `detect()`:
  - When `SANDBOX_SUPPORTED`, run the existing three probes (bwrap, systemd-run, systemctl --user) verbatim and in the same order.
  - Otherwise skip those probes entirely and set missing to `off_platform_missing()`.
  - The `claude --version` probe still runs on every platform.
- Write the four behavior tests in the existing `mod tests`, see them pass, and commit.
- Rationale, for a doc comment on the const: on macOS a `bwrap` or `systemd-run` shim on PATH must never yield BwrapScope. decide() is the only producer of Posture (run.rs:579 and :1261 call `decide(&detect(), ..)`), so this one chokepoint makes the sandboxed posture unreachable off Linux by construction.

**Step 2: widen the launcher stack to Linux+macOS (G1)** `[inferred — audit]`.
- In src/launcher/mod.rs, src/driverctl/mod.rs, src/driverctl_cli.rs, src/driver_cli.rs, src/driver_templates/mod.rs and src/driver/direction.rs:
  - replace every `cfg(target_os = "linux")` with `cfg(any(target_os = "linux", target_os = "macos"))`;
  - replace every `cfg(not(target_os = "linux"))` with `cfg(not(any(target_os = "linux", target_os = "macos")))`;
  - replace `cfg(all(test, target_os = "linux"))` with `cfg(all(test, any(target_os = "linux", target_os = "macos")))`.
- Do not widen gates anywhere else. In particular, leave src/driver/steer.rs, src/state_reader/driver_frontmatter.rs, src/session_detector.rs, src/registry.rs, the ui/screens files, app.rs and detail.rs alone. Their remaining Linux gates are test-only or TUI, and leaving them compiled cleanly in the experiment.
- Why `any(linux, macos)` and not `unix`: other unixes are unverified and keep the refusal stubs, which fails closed.
- Correct the user-facing text that the widening makes false. These stubs are now Windows/other-only:
  - the "--unsandboxed" refusal in driver_cli.rs `launch_main` ("the run store needs Linux (no renamex_np splice yet)") and its doc comment;
  - the driverctl_cli.rs `serve_main` stub message and doc;
  - the direction.rs `unsupported()` detail ("renameat2(RENAME_EXCHANGE) is Linux-only");
  - the module doc in src/launcher/prompt.rs:18 and the gating prose near src/launcher/mod.rs:45.
  Each should name Linux or macOS as the supported platforms. Keep the error variants and exit codes unchanged.

**Step 3: the scan.rs FIFO test (G1).** In the src/driverctl/scan.rs test module:
- Put `#[cfg(target_os = "linux")]` on `cr01_a_fifo_named_like_a_verification_does_not_hang_the_scan`. rustix has no `mknodat` on Apple.
- Put the same cfg on its only helper, `within_a_generous_bound`.

**Step 4: iterate on the darwin builds.**
- Run `rtk proxy cargo check --target x86_64-apple-darwin --all-targets --locked` until it exits 0.
- Run `rtk proxy cargo clippy --target x86_64-apple-darwin --lib --bins --locked -- -D warnings` until it exits 0.
- Run `rtk proxy cargo clippy --target x86_64-apple-darwin --all-targets --message-format short -- -D warnings`. Every error it reports must be at src/app.rs:8460 or src/ui/screens/detail.rs:20043 (foreign, owned by the master fix). Anything else is yours to fix.
- If a widened file needs more than a gate swap to compile on darwin, stop and record it in SUMMARY rather than porting logic.

Commit with message `fix(quick-261020-vxg): build on macOS; launcher degraded path on Linux+macOS, sandbox fail-closed off Linux`.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib launcher::host && rtk proxy cargo check --target x86_64-apple-darwin --all-targets --locked && rtk proxy cargo clippy --target x86_64-apple-darwin --lib --bins --locked -- -D warnings && rtk proxy sh -c 'n=$(cargo clippy --target x86_64-apple-darwin --all-targets --message-format short -- -D warnings 2>&1 | grep -E "^src/.*: error" | grep -v -E "^src/app.rs:8460:|^src/ui/screens/detail.rs:20043:" | wc -l); test "$n" -eq 0'</automated>
  </verify>
  <done>
- The new host tests pass on Linux.
- The darwin `--all-targets` check exits 0.
- darwin clippy on lib+bins exits 0.
- darwin clippy `--all-targets` reports nothing outside the two foreign helpers.
- `grep -c SANDBOX_SUPPORTED src/launcher/host.rs` is at least 2 (it was 0 at HEAD).
- The task is committed.
  </done>
</task>

<task type="auto">
  <name>Task 2: Windows: gate Unix-only code with cfg(unix) and add fail-closed stubs so lib+bins compile (all-targets if it stays in budget)</name>
  <files>src/state_reader/config_write.rs, src/driverctl/scan.rs, src/launcher/sandbox.rs, src/driver/liveness.rs, src/driver/mod.rs, src/cli.rs, src/experimental.rs, src/ui/screens/driver_start.rs</files>
  <action>
**config_write.rs (G2)** `[inferred — audit]`.
- These stay portable and unchanged: LockFlavor, LockPolicy, ConfigWriteTarget (including `config_path`, `flavor`, `lock_path` and `dir`), ConfigWriteError and its `status_text`.
- Put `#[cfg(unix)]` on every item that only the Unix lock-and-write implementation uses:
  - the `std::os::unix` and `rustix::io::Errno` imports;
  - the caps/consts/statics it alone reads;
  - the lock-body structs;
  - `now_iso`, `read_bounded`, `body_pid`, `probe_pid`, `planning_verified_live`, `is_transient_create_error`;
  - LockGuard and its Drop;
  - `try_create`, `lock_body`, `planning_on_exists`, `install_on_exists`, `acquire`;
  - the existing `atomic_write` and `update_gsd_config`.
- Add `#[cfg(not(unix))]` versions of `atomic_write` and `update_gsd_config`. Their signatures must be identical to the Unix ones.
  - They return `Err(std::io::Error::from(std::io::ErrorKind::Unsupported))` and `Err(ConfigWriteError::Io { kind: std::io::ErrorKind::Unsupported })` respectively.
  - Both return before any file I/O, and update_gsd_config never calls `mutate`. A save without gsd-core's lock protocol must never happen. The Defaults tab then shows "Config NOT saved: unsupported" through the existing `status_text`. src/ui/screens/detail.rs needs no edit.
  - Document this on the stub.
- Change the test module gate from `#[cfg(test)]` to `#[cfg(all(test, unix))]`. The tests exercise the Unix implementation.
- Any item left dead on Windows gets the cfg of its only user.

**scan.rs.**
- Gate the `OpenOptionsExt` import at line 10 and the existing `read_regular_bounded` body with `#[cfg(unix)]`.
- Add a `#[cfg(not(unix))]` `read_regular_bounded` with the same signature that returns None, so every file is skipped. Without O_NOFOLLOW|O_NONBLOCK and the fstat re-check, the CR-01 guard cannot be reproduced, so the scan fails closed `[inferred — audit]`.
- Gate the symlink tests (they use `std::os::unix::fs::symlink`, near lines 366 and 379) with `#[cfg(unix)]`.

**sandbox.rs.**
- Gate only the stored-token mask statement in `SandboxSpec::from_host` (the `if let Some(token) = super::auth::token_path()` block, line 233) with `#[cfg(any(target_os = "linux", target_os = "macos"))]`. That is exactly the cfg of `launcher::auth`.
- Add no other change. On Linux the predicate is true and the code is identical. The deleted-lines count for this file must be 0.

**Tests that call Unix-only APIs.** Put `#[cfg(unix)]` on:
- the liveness.rs tests that use `std::os::unix`, `process_group` or `arg0` (around lines 343, 373 and 400);
- the tests in src/cli.rs (around line 907), src/experimental.rs (around line 143) and src/ui/screens/driver_start.rs (around line 586) that call `crate::driver::spawn`, which is cfg(unix).

**driver/mod.rs:1210.** Fix the Windows-only unused `decomposed` binding the way the file's own comment at :1011 prescribes: annotate, don't gate. Use a targeted `cfg_attr(not(unix), allow(unused_variables))` or an underscore rename on the non-unix binding. The unix arm must stay byte-identical.

**Iterate.**
- Run `rtk proxy cargo check --target x86_64-pc-windows-gnu --lib --bins --locked` until it exits 0. Expect more type-check errors to appear once the resolution errors are gone.
- Then try `--all-targets`. For integration test files under tests/ that are inherently Unix (sockets, process groups, std::os::unix), add a file-level `#![cfg(unix)]` as the first non-comment line. That is one line per file, a no-op on Linux.
- Stop at lib+bins and record what is left in SUMMARY, marked `[inferred — audit]`, if `--all-targets` would need any edit to src/app.rs or src/ui/screens/detail.rs, or more than about 25 test files.
- Also run `rtk proxy cargo clippy --target x86_64-pc-windows-gnu --lib --bins --locked -- -D warnings`. Fix cheap Windows-only lints with the same cfg discipline. Record whether it ends green: Task 3 enables Windows clippy in CI only if it is.

**Re-check macOS and Linux.** Re-run the Task 1 darwin check, since scan.rs and sandbox.rs changed. Run `rtk proxy cargo clippy --all-targets -- -D warnings` on Linux.

Commit with message `fix(quick-261020-vxg): build on Windows; unix-only code gated, config save and scan fail closed`.
  </action>
  <verify>
    <automated>rtk proxy cargo check --target x86_64-pc-windows-gnu --lib --bins --locked && rtk proxy cargo check --target x86_64-apple-darwin --all-targets --locked && rtk proxy cargo clippy --all-targets -- -D warnings && rtk proxy sh -c 'out=$(git diff --numstat 8035409 -- src/launcher/sandbox.rs) || exit 1; test -n "$out" && test "$(printf "%s" "$out" | cut -f2)" -eq 0' && rtk proxy grep -c 'ErrorKind::Unsupported' src/state_reader/config_write.rs</automated>
  </verify>
  <done>
- The Windows lib+bins check exits 0 (it had 25 errors at HEAD).
- The macOS check is still green.
- Linux clippy `--all-targets -D warnings` is clean.
- sandbox.rs shows 0 deleted lines against 8035409.
- config_write.rs contains the Unsupported stubs.
- The SUMMARY scratch notes record:
  - whether Windows `--all-targets` was reached, and what is left if not;
  - whether Windows clippy lib+bins is green.
- The task is committed.
  </done>
</task>

<task type="auto">
  <name>Task 3: CI cross-target-check job gating publish, README Windows note, full Linux regression</name>
  <files>.github/workflows/release.yml, README.md</files>
  <action>
**CI (G3)** `[inferred — audit]`. In .github/workflows/release.yml, add a job `cross-target-check`:
- **Name:** "Check non-Linux targets compile (${{ matrix.target }})".
- **Runner:** `runs-on: ubuntu-24.04`, matching the file's pinning rationale. Update the header comment that says "Both jobs pin ubuntu-24.04" so it covers every job.
- **Strategy:** `fail-fast: false`.
- **Matrix:** a matrix `include` list with three entries:
  - target `aarch64-apple-darwin`, check_args `--all-targets`, clippy true;
  - target `x86_64-apple-darwin`, check_args `--all-targets`, clippy true;
  - target `x86_64-pc-windows-gnu`, check_args `--all-targets` if Task 2 reached it, otherwise `--lib --bins`; clippy true only if Task 2 measured Windows clippy lib+bins green.
- **Steps:**
  1. `actions/checkout@v7`.
  2. `dtolnay/rust-toolchain@stable` with `targets: ${{ matrix.target }}` and `components: clippy`.
  3. `Swatinem/rust-cache@v2` with `key: ${{ matrix.target }}`.
  4. A Check step: `cargo check --locked --target "${{ matrix.target }}" ${{ matrix.check_args }}`.
  5. A Clippy step guarded by `if: matrix.clippy`: `cargo clippy --locked --target "${{ matrix.target }}" --lib --bins -- -D warnings`.
- **Publish dependency:** change publish's `needs: msrv` to `needs: [msrv, cross-target-check]`. A crate that does not build on macOS or Windows must not reach crates.io.
- **Comment block above the job.** Write it in the file's existing explanatory style, stating:
  - Why ubuntu with rustup targets: `cargo check` and clippy need no linker, so no macOS/Windows runner minutes are spent.
  - Why windows-gnu: the crate has no `target_env` cfg, so it is cfg-equivalent to msvc, and it is the target verified locally.
  - Why clippy covers lib+bins only: two test-only helpers (app.rs, detail.rs) are dead under macOS cfg until the master fix lands. Widen clippy to `--all-targets` once it does.
  - That, like the rest of this workflow, it runs only on a tag push.
  - That scripts/pre-tag-check.sh does not yet mirror this gate. That is a follow-up.
- Do not touch the msrv or publish steps otherwise. Add no new actions beyond the three already used in this file.

**README** `[inferred — audit]`. In README.md `### Platform support`, add a short paragraph after the session-detection paragraph:
- Windows builds (checked in CI) but is not exercised at runtime.
- Saving config from the Defaults tab is refused there.
- The driver launcher is unavailable there.
Do not document the driver launcher otherwise. README does not cover it today.

**Final Linux regression.**
- Run `mkdir -p -m700 /tmp/claude-1000/t; umask 077; TMPDIR=/tmp/claude-1000/t rtk proxy cargo test --no-fail-fast > /tmp/claude-1000/t/linux-after.log 2>&1`.
- Sum it with `rtk proxy awk '/^test result:/ {p+=$4; f+=$6} END {print p, f}' /tmp/claude-1000/t/linux-after.log`.
- Expected: passed equals the Task 1 baseline plus the new host.rs tests (4, or however many Task 1 added); failed is 1, and it is the same envelope/policy.rs git-version witness as in the baseline. Any other delta is a regression. Fix it before committing.
- Run `rtk proxy cargo clippy --all-targets -- -D warnings` (exit 0).

**SUMMARY.** It must list every `[inferred — audit]` decision:
1. The launcher stack is widened to `any(linux, macos)` rather than `unix`.
2. SANDBOX_SUPPORTED forces the three sandbox pieces missing off Linux.
3. Windows config save refuses with Unsupported, and the scan skips files.
4. Cargo.toml and rustix are unchanged, because the rustix-on-Windows failure did not reproduce.
5. CI runs on ubuntu with the gnu target, on tags only, gates publish, and pre-tag-check.sh parity is a follow-up.
6. TUI-side driver launch on macOS is still Linux-gated in app.rs and ui/screens/driver_launch.rs, untouched per constraint. That is a follow-up.
7. macOS runtime behaviour of `driver launch --unsandboxed` is unverified here. The operator's 28-UAT item 2 on a real Mac is the check. Note what to watch: liveness reports Unknown (LIVENESS_SUPPORTED is false), store host_name is None, and the RENAME_SWAP splice needs APFS.
8. Windows `--all-targets` and clippy status, from Task 2.

Commit with message `ci(quick-261020-vxg): cross-target check for macOS and Windows gates publish`.
  </action>
  <verify>
    <automated>rtk proxy python3 -I -c "import yaml; d=yaml.safe_load(open('.github/workflows/release.yml')); j=d['jobs']; c=j['cross-target-check']; ts=[e['target'] for e in c['strategy']['matrix']['include']]; assert {'aarch64-apple-darwin','x86_64-apple-darwin','x86_64-pc-windows-gnu'} <= set(ts), ts; n=j['publish']['needs']; n=[n] if isinstance(n,str) else n; assert 'msrv' in n and 'cross-target-check' in n, n; print('ok')" && rtk proxy cargo clippy --all-targets -- -D warnings && rtk proxy grep -c -i 'windows' README.md</automated>
  </verify>
  <done>
- release.yml parses, has a cross-target-check job covering the three targets, and publish needs both msrv and cross-target-check.
- README mentions Windows in Platform support. At HEAD it had 0 matches.
- The Linux full suite has the baseline pass count plus the new host tests and the single known witness failure.
- Linux clippy is clean.
- The task is committed.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| host PATH → launcher host probe | On macOS, a `bwrap`/`systemd-run` binary on PATH is untrusted and could fake a sandbox |
| TUI Defaults tab → GSD config files | On Windows, a write without gsd-core's lock protocol could corrupt a config GSD is writing |
| project tree (driver-plantable) → verification scan | Planted FIFOs and symlinks (CR-01) on a platform without O_NOFOLLOW/O_NONBLOCK |
| CI workflow → crates.io publish | A non-Linux build break must not ship |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-vxg-01 | Elevation of Privilege | src/launcher/host.rs detect()/decide() on macOS | high | mitigate | SANDBOX_SUPPORTED=cfg!(linux): off Linux the probes are skipped and bwrap/systemd-run/user-systemd are always missing, so decide(_, false) always refuses and BwrapScope is unreachable. Pinned by Linux-run tests over off_platform_missing() (Task 1). |
| T-vxg-02 | Tampering | Linux sandbox (sandbox.rs, run.rs BwrapScope path) | high | mitigate | Every new predicate is true on Linux, so the Linux code is unchanged. sandbox.rs diff has zero deleted lines (Task 2 verify). Full Linux test suite matches the baseline count plus new tests, and clippy is clean (Task 3). |
| T-vxg-03 | Tampering | src/state_reader/config_write.rs on Windows | medium | mitigate | The non-unix update_gsd_config/atomic_write return Unsupported before any I/O and never call mutate. No unlocked write path exists. |
| T-vxg-04 | Denial of Service | src/driverctl/scan.rs read_regular_bounded on Windows | medium | mitigate | The non-unix variant returns None (skips the file), so a planted FIFO or symlink cannot hang or redirect the scan. |
| T-vxg-05 | Repudiation | macOS degraded run behaviour (liveness Unknown, RENAME_SWAP splice) | medium | accept | Unobservable without a Mac. Recorded in SUMMARY and routed to the operator's 28-UAT item 2. A splice failure (ENOTSUP) surfaces as an error and is not a partial write. |
| T-vxg-06 | Tampering | .github/workflows/release.yml cross-target-check | low | mitigate | Reuses only actions already in the file, uses `--locked`, and gates publish via `needs`. |
| T-vxg-SC | Tampering | cargo installs | high | mitigate | No new crates. Cargo.toml and Cargo.lock are unchanged, and every cargo command uses `--locked` where it applies. No package-legitimacy checkpoint is needed. |
</threat_model>

<verification>
- macOS: `rtk proxy cargo check --target x86_64-apple-darwin --all-targets --locked` exits 0, and `rtk proxy cargo clippy --target x86_64-apple-darwin --lib --bins --locked -- -D warnings` exits 0.
- Windows: `rtk proxy cargo check --target x86_64-pc-windows-gnu --lib --bins --locked` exits 0 (`--all-targets` if reached).
- Linux: the full `cargo test --no-fail-fast` has the baseline passed count plus the new host tests, and the only failure is the envelope/policy.rs git-version witness. `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0.
- `git diff --stat 8035409 -- src/app.rs src/ui/screens/detail.rs` is empty.
- release.yml: publish needs cross-target-check.
</verification>

<success_criteria>
- G1: darwin check and clippy are green. Off Linux, `driver launch` refuses by default (SandboxUnavailable names bwrap and systemd-run), and `--unsandboxed` takes the real degraded path.
- G2: the Windows lib+bins check is green, and Windows features refuse at runtime.
- G3: the cross-target CI job exists and gates publish.
- Linux is unchanged.
- Zero edits to app.rs and detail.rs.
- Three atomic commits on the worktree branch, with nothing pushed.
</success_criteria>

<output>
Create `.planning/quick/261020-vxg-fix-non-linux-build-breaks-macos-windows/261020-vxg-SUMMARY.md` when done
</output>
