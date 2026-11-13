---
phase: quick-261113-dck
plan: 01
subsystem: launcher
tags: [sandbox, credentials, claude-config-dir, xdg, auth]
status: complete
requires: []
provides:
  - HostDirs resolver (CLAUDE_CONFIG_DIR and XDG_* injected, blank or relative counts as unset)
  - host-token fallback, auth relaunch, egress re-read, auth status and per-run seeding follow ${CLAUDE_CONFIG_DIR:-~/.claude}
  - sandbox allowlist, null masks and outside-HOME dir masks derived from HostDirs
affects: [src/launcher, src/oneoff, docs]
key-files:
  created:
    - src/launcher/host_dirs.rs
  modified:
    - src/launcher/auth.rs
    - src/launcher/run.rs
    - src/launcher/config_dir.rs
    - src/launcher/home.rs
    - src/launcher/sandbox.rs
    - src/launcher/mod.rs
    - src/oneoff/runtime.rs
    - src/oneoff/launch.rs
    - tests/launcher_caps.rs
    - tests/launcher_auth.rs
    - tests/launcher_config_dir.rs
    - tests/launcher_sandbox.rs
    - tests/launcher_sandbox_data_mask.rs
    - tests/launcher_live_isolation.rs
    - tests/launcher_live_managed.rs
    - docs/DRIVER-SANDBOX.md
    - docs/CONFIGURATION.md
commits: 3
plan_head_before: 7c857d5ba76e548469575a218a4971c1fd0d031e
plan_head_after: 577f27e9adac4503e0249b1199c5e06ffa29f4db
---

# Phase quick-261113-dck Plan 01: Launcher honours CLAUDE_CONFIG_DIR and XDG dirs Summary

One `HostDirs` resolver now drives the host-token fallback, per-run seeding, the sandbox `$HOME` allowlist and every credential mask, with the default locations kept (union rule) and degenerate values unable to tmpfs `/`, an ancestor of `$HOME`, `/tmp` or a toolchain.

## Commits

| Task | Commit | Subject |
|------|--------|---------|
| 1 | 68eaac0 | feat(quick-261113-dck): host token follows CLAUDE_CONFIG_DIR through one HostDirs resolver |
| 2 | 06ff731 | feat(quick-261113-dck): seed per-run config from CLAUDE_CONFIG_DIR with the real host home |
| 3 | 577f27e | fix(quick-261113-dck): sandbox binds and credential masks follow CLAUDE_CONFIG_DIR and XDG dirs |

## Gate results

- `cargo test --no-fail-fast`: exit 0, 3857 passed, 0 failed, 19 ignored (baseline 3827; required >= 3852). Run with this host's `CLAUDE_CONFIG_DIR` set.
- `env -u CLAUDE_CONFIG_DIR cargo test` for `--lib launcher::`, `--lib oneoff::` and the launcher_sandbox, launcher_caps, launcher_auth, launcher_config_dir and launcher_sandbox_data_mask suites: exit 0.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --all -- --check`: clean.
- `cargo check --locked --all-targets` and `cargo clippy --locked --all-targets -- -D warnings` for aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-gnu: all exit 0.

## Measured premise correction

The task's security premise was partly stale. The stored run token was already masked in both cases: WR-02 null-masks it when it sits in a bind, and the app-root mask covers it when it is outside `$HOME`. The real gaps, and how each was closed:

- (a) A relocated `$XDG_CONFIG_HOME` under `$HOME` was not a refused install root, so a tool at `$XDG_CONFIG_HOME/bin/x` bound the whole dir with the gh/gcloud/glab tokens. Closed: `root_refused` refuses the XDG homes and every Claude root; `touches_mask_dir` iterates `HostDirs::credential_dirs()`.
- (b) Relocated gh/gcloud/glab-cli/keyrings outside `$HOME` were readable through the read-only `/`. Closed: `sandbox_dir_masks_in` tmpfs-masks each existing one.
- (c) A Claude root outside `$HOME` exposed `.credentials.json`, `.claude.json`, transcripts and shell snapshots. Closed: empty 1 MiB tmpfs plus read-only re-binds of its GSD entries; its two credential files are `/dev/null`-masked.
- (d) `$XDG_CONFIG_HOME/git/config` outside `$HOME` reached the driver's git (T-28.1-33). Closed: `config_home/git` is a tmpfs mask candidate.

## Decisions (all `[inferred — audit]`)

Plan decisions I-1 to I-12 were implemented as written. In summary: I-1 one resolver (`resolve_config_root` for the Claude root, one private helper for XDG, not the `dirs` crate and not `InstallRoots::config_home`); I-2 host token only from the configured root, no fallback to `~/.claude`; I-3 seeding only from the configured root; I-4 union of default and relocated binds and masks; I-5 outside-`$HOME` policy table (see docs/DRIVER-SANDBOX.md); I-6 `config_home/git` masked as a dir; I-7 `masks_outside` keeps only `Placement::Maskable`; I-8 impure entries kept as wrappers over `_with` variants; I-9 GSD hooks recognised under every Claude root; I-10 `XDG_STATE_HOME`, `GSD_HOME` and the `.gsd` binds stay `$HOME`-relative; I-11 `oneoff/resolve.rs` unchanged; I-12 one `HostDirs` per one-off session.

New inferred decisions made while executing:

- `credential_files()` includes `<claude_root>/.claude.json`, so on a default host a `~/.claude/.claude.json` is now also a null-mask candidate (masked only if it exists inside a read-only bind). This only adds a mask; no default is dropped. The unit test pins it.
- A tempdir in `/tmp` cannot stand in for "outside `$HOME` and `/tmp`", so the three tests that need that placement create their scratch dir under `target/` (`CARGO_MANIFEST_DIR/target` or `CARGO_TARGET_TMPDIR`) and return early with a printed note if the build tree itself is under `/tmp`.
- The tmpfs over a relocated per-run cache or rtk-data dir outside `$HOME` is emitted only when that path exists (bwrap cannot mkdir on the read-only root), via the same rule as the Claude-root tmpfs.
- `HostFacts.dirs` is a new public field; `HomeSetup::for_host_with` / `HostFacts::from_host_with` / `SandboxSpec::from_host_with` are the hermetic entries, the old names are wrappers.

## Deviations from Plan

### Auto-fixed issues

**1. [Rule 3 - Blocking] I-12 names the wrong function.**
- **Found during:** Task 2.
- **Issue:** `run_session` in `src/oneoff/launch.rs` does not touch `spec.home`; the code that seeds, builds the egress token and the sandbox is `run_sandboxed_session`.
- **Fix:** `HostDirs::from_host(spec.home)` is resolved once at the top of the `run_sandboxed_session` home-setup block and used for seeding, egress token, `HomeSetup`, `SandboxSpec` and `sandbox_dir_masks`. The seam guard still passes (no agent name in launch.rs).
- **Commit:** 06ff731, 577f27e.

**2. [Rule 1 - Dead code] `LaunchContext.home` became unread.**
- **Issue:** After the migration nothing read `ctx.home`; `-D warnings` would fail.
- **Fix:** removed the field; `ctx.host_dirs.home` carries the host `$HOME`.
- **Commit:** 577f27e.

**3. [Process] Red-first only observed for Task 1.**
- The Task 1 end-to-end tests were run and seen to fail before the fix. For Tasks 2 and 3 the new tests were written together with the implementation and verified green; I did not stage a separate failing run.

### Auth gates

None.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model: no new network endpoint or auth path. T-261113-dck-01 to -06 are mitigated and pinned by tests (`a_claude_root_outside_home_is_hidden_and_its_gsd_entries_rebound`, `from_host_with_masks_the_run_token_where_token_path_puts_it`, `sandbox_dir_masks_in_masks_relocated_credential_dirs_outside_home`, `masks_outside_never_masks_an_ancestor_of_home_or_tmp`, the placement and home tests, the two Task 1 end-to-end tests). T-07 and T-08 are accepted and documented.

## Follow-up candidates

- `GSD_HOME`: the `.gsd` and `.gsd-capabilities.json` allowlist binds ignore it (I-10). A host with `GSD_HOME` set binds `~/.gsd` rather than `$GSD_HOME/.gsd`.
- `driverctl/serve.rs` reads `XDG_RUNTIME_DIR` on its own, outside this change.

## Self-Check: PASSED

- src/launcher/host_dirs.rs: found.
- docs/DRIVER-SANDBOX.md: found.
- Commits 68eaac0, 06ff731, 577f27e: in `git log`; `git rev-list --count 7c857d5..HEAD` = 3.
