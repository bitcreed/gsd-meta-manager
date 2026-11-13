---
phase: quick-261113-dck
plan: 01
type: execute
wave: 1
depends_on: []
coupling_justified:
  - "261113-dcj: this plan only CALLS agents::adapters::claude::resolve_config_root (pub, signature (Option<OsString>, Option<PathBuf>) -> Option<PathBuf>) and never edits src/agents/adapters/claude.rs; dcj is told to keep and reuse that resolver, so either merge order compiles"
files_modified:
  - src/launcher/host_dirs.rs
  - src/launcher/mod.rs
  - src/launcher/auth.rs
  - src/launcher/run.rs
  - src/launcher/config_dir.rs
  - src/launcher/home.rs
  - src/launcher/sandbox.rs
  - src/oneoff/runtime.rs
  - src/oneoff/launch.rs
  - tests/launcher_caps.rs
  - tests/launcher_auth.rs
  - tests/launcher_config_dir.rs
  - tests/launcher_live_isolation.rs
  - tests/launcher_live_managed.rs
  - tests/launcher_sandbox.rs
  - tests/launcher_sandbox_data_mask.rs
  - docs/DRIVER-SANDBOX.md
  - docs/CONFIGURATION.md
autonomous: true
requirements: [QUICK-261113-dck]

must_haves:
  truths:
    - "With CLAUDE_CONFIG_DIR set to a non-empty absolute directory, a driver launch takes its host-token fallback from $CLAUDE_CONFIG_DIR/.credentials.json, the auth relaunch and the launcher-side egress server re-read that same file, one-off runs do the same, and `driver auth status` reports that token. A blank or relative value counts as unset and ~/.claude is used, exactly as agents::adapters::claude::resolve_config_root decides (I-1, I-2)."
    - "The per-run config dir links the GSD entries and transforms settings.json from $CLAUDE_CONFIG_DIR. Hook `~`/`$HOME` expansion and the keep-root check use the real host $HOME, not the parent of the Claude dir, and a GSD hook whose script lies under the relocated Claude root is recognised as a GSD hook (I-3, I-9)."
    - "The sandbox $HOME allowlist re-binds the GSD entries (agents, commands, skills, gsd-core, get-shit-done, hooks, scripts, .gsd-profile) of BOTH the configured Claude root and ~/.claude, rtk config at both ~/.config/rtk and $XDG_CONFIG_HOME/rtk, and per-run tmpfs at both the default and the relocated cache and rtk-data dirs. Relocated XDG dirs and every Claude root are refused as whole-directory install roots, and their credential stores join the no-overlap invariant (I-4)."
    - "Every credential mask is derived from the resolver that writes or reads that file: the stored run token from auth::token_path() (the resolver token_path() itself uses), the Claude credential files from the configured Claude root, and git, gh, gcloud, glab-cli and keyrings from the resolved XDG dirs. The default ~/... locations stay masked too, so no default is ever dropped (deny-by-default)."
    - "Outside $HOME: a relocated Claude root becomes an empty tmpfs with only its GSD entries re-bound read-only. Its .credentials.json and .claude.json are /dev/null-masked whenever they exist outside $HOME and /tmp. Relocated gh, gcloud, glab-cli, git and keyrings directories that exist outside $HOME and /tmp become empty tmpfs mounts. No tmpfs is ever placed on `/` or on an ancestor of $HOME, /tmp, a toolchain or this binary. The residual (other contents of relocated XDG dirs outside $HOME stay readable read-only, as before) is documented (I-5, I-6, I-7)."
    - "With none of CLAUDE_CONFIG_DIR / XDG_* set, every existing golden argv and allowlist test passes unchanged. Every test injects its environment through HostDirs::resolve or an explicit path; no test writes the process environment, and the targeted suites pass both with the developer's real CLAUDE_CONFIG_DIR set and with it unset (I-8)."
    - "Gates: `cargo test --no-fail-fast` exits 0 with at least 3852 passed and 0 failed, read from a log file inside `rtk bypass`. `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `cargo check` plus `cargo clippy -D warnings` with `--locked --all-targets` for aarch64-apple-darwin, x86_64-apple-darwin and x86_64-pc-windows-gnu all exit 0. `git status` is clean."
  artifacts:
    - path: src/launcher/host_dirs.rs
      provides: "HostDirs: the one injected-env resolver for CLAUDE_CONFIG_DIR and XDG_CONFIG_HOME/XDG_DATA_HOME/XDG_CACHE_HOME/XDG_RUNTIME_DIR, the run-token path, the credential lists and the Placement classifier"
      contains: "pub struct HostDirs"
    - path: src/launcher/auth.rs
      provides: "resolve_launch_token_in and host_claude_dir: the host-token fallback and auth status follow the configured Claude root"
      contains: "pub fn resolve_launch_token_in"
    - path: src/launcher/config_dir.rs
      provides: "SeedSpec/OneoffSeedSpec host_home; settings_transform_in / hook_is_kept_in with Claude-root markers; sandbox_dir_masks"
      contains: "pub fn sandbox_dir_masks"
    - path: src/launcher/home.rs
      provides: "allowlist over every Claude root and the resolved XDG dirs; outside-$HOME Claude-root tmpfs plus re-binds; HostFacts::from_host_with / HomeSetup::for_host_with"
      contains: "pub fn for_host_with"
    - path: src/launcher/sandbox.rs
      provides: "SandboxSpec::from_host_with with null masks from HostDirs; masks_outside never masks an ancestor"
      contains: "pub fn from_host_with"
    - path: docs/DRIVER-SANDBOX.md
      provides: "Relocated host directories section plus a residual-risk bullet"
      contains: "CLAUDE_CONFIG_DIR"
  key_links:
    - from: src/launcher/host_dirs.rs
      to: src/agents/adapters/claude.rs
      via: "HostDirs::resolve calls resolve_config_root for CLAUDE_CONFIG_DIR (reused, not modified)"
      pattern: "resolve_config_root"
    - from: src/launcher/host_dirs.rs
      to: src/launcher/auth.rs
      via: "HostDirs::from_host sets run_token from auth::token_path() on linux/macos"
      pattern: "token_path"
    - from: src/launcher/run.rs
      to: src/launcher/host_dirs.rs
      via: "HostDirs::from_host once per launch, kept in LaunchContext.host_dirs and used by auth_relaunch, EgressGuard::start, seed, HomeSetup::for_host_with, SandboxSpec::from_host_with and sandbox_dir_masks"
      pattern: "host_dirs"
    - from: src/oneoff/launch.rs
      to: src/oneoff/runtime.rs
      via: "run_session resolves HostDirs once and passes &HostDirs to egress_token and seed_config (the agent-specific field is read only inside the runtime seam)"
      pattern: "HostDirs::from_host"
---

<objective>
Make the launcher's host-side paths follow `CLAUDE_CONFIG_DIR` and the XDG base-directory variables instead of hard-coded `~/.claude` and `~/.config` / `~/.local/share` / `~/.cache`. This covers the host-token fallback and auth status, the auth relaunch and egress re-read, per-run config seeding, the sandbox's `$HOME` allowlist, and every credential mask. One resolver is reused everywhere. Defaults stay unchanged and masked. A relocated path outside `$HOME` gets a decided, documented, deny-by-default treatment.

Purpose: on a claude-profile host (`CLAUDE_CONFIG_DIR=~/.local/share/claude-profiles/<name>`, as on this machine) the driver today reads the wrong account's token from `~/.claude`. And with a Claude or XDG dir relocated outside `$HOME`, the sandbox's read-only root leaves the host's OAuth refresh token and gh/gcloud/glab credentials readable to the sandboxed agent.

Output:
- a new `src/launcher/host_dirs.rs` (`HostDirs`, `Placement`) with unit tests
- auth, run, config_dir, home, sandbox and one-off runtime/launch wired through it
- end-to-end harness tests (token, auth status, seeding)
- allowlist, mask and argv tests
- migrated env-dependent tests
- updated docs/DRIVER-SANDBOX.md and docs/CONFIGURATION.md
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md

Execution is sequential, one commit per task: conventional, path-scoped (`git commit -- <paths>`), never `git add -A`. The user is unavailable. Every decision marked I-n was made by the planner from code and measurements and is `[inferred — audit]`. Do not touch `/etc/claude-code`, `/dev/null` or `/bin/sh` handling. Do not edit `src/agents/adapters/claude.rs` (sibling item 261113-dcj owns it); only call its `resolve_config_root`.

## Facts measured at planning (HEAD 7c857d5, 2026-11-13)

- Hard-coded host Claude dir, production code:
  - `src/launcher/auth.rs` ~L241 (`resolve_launch_token` host fallback) and ~L474 (`auth_status`).
  - `src/launcher/run.rs` ~L740 (`SeedSpec.host_claude_dir`), ~L1135 (`auth_relaunch`) and ~L1725 (`EgressGuard::start`, `TokenProvider::Host`).
  - `src/oneoff/runtime.rs` ~L737 (`egress_token`) and ~L774 (`seed_config`). This is where `src/oneoff/launch.rs:261` and `src/oneoff/resolve.rs:916` lead: the `home` those two lines resolve IS `$HOME` and is correct, so resolve.rs needs no change (I-11).
  - `src/launcher/config_dir.rs` `seed` (~L836) and `seed_oneoff` (~L932) derive the host `$HOME` as the parent of the Claude dir. That breaks the moment the Claude dir is `$CLAUDE_CONFIG_DIR`.
  - `GSD_HOOK_MARKERS` (~L445-450) recognise GSD hooks only by `.claude/hooks/`-style text.
- `src/launcher/home.rs`:
  - `HOME_RO_REL` (~L45-57) hard-codes eight `.claude/...` entries plus `.config/rtk`.
  - `CACHE_REL` / `RTK_REL` (~L82-86) and `REFUSED_ROOT_REL` (~L92-99) are `$HOME`-relative.
  - `touches_mask_dir` / `root_refused` use only the `$HOME`-relative `TMPFS_MASK_DIRS` / `NULL_MASK_FILES`.
  - `dangling_within` recognises the skills dir only when its parent is named `.claude`.
- `src/launcher/sandbox.rs`:
  - `TMPFS_MASK_DIRS` (~L232) and `NULL_MASK_FILES` (~L251) are `$HOME`-relative.
  - `SandboxSpec::from_host` (~L424-462) already appends `auth::token_path()` to the null masks when it lies inside a bind (WR-02), and it reads the runtime dir straight from the environment.
  - `masks_outside` (~L567) drops only paths under `$HOME` and `/tmp`.
- `config_dir.rs` ~L151: the app-root mask helper already masks `dirs::config_dir()/gsd-meta-manager` (where the token lives) when it is outside `$HOME`.
- **The task's security premise is partly stale.** The stored token itself is already masked in both cases. When it sits in a bind, WR-02 null-masks it. When it is outside `$HOME`, the app-root mask covers it. The real gaps are:
  - (a) a relocated `$XDG_CONFIG_HOME` under `$HOME` is not a refused install root, so a tool at `$XDG_CONFIG_HOME/bin/x` binds the whole dir, gh/gcloud/glab tokens included;
  - (b) relocated gh/gcloud/glab-cli/keyrings/git-credentials outside `$HOME` are readable through the read-only `/`;
  - (c) a Claude root outside `$HOME` exposes its `.credentials.json` (refresh token), `.claude.json`, transcripts and shell snapshots;
  - (d) `$XDG_CONFIG_HOME/git/config` outside `$HOME` reaches the driver's git, breaking T-28.1-33 (no host alias, include or credential.helper).
  Record this in the SUMMARY.
- The `dirs` crate on Linux honours `XDG_*_HOME` only when it is absolute, so blank or relative counts as unset. That matches `resolve_config_root`, which also refuses a relative value. `InstallRoots::config_home` in `src/state_reader/gsd_install.rs` accepts relative values and expands `~`, which is unsafe for mount paths, so it is not reused (I-1).
- Test harness (`tests/launcher_support/mod.rs` `hermetic_command`):
  - It calls `env_clear`, so `CLAUDE_CONFIG_DIR` is unset in children.
  - It sets `HOME=<scratch>/home`, `XDG_CONFIG_HOME=<scratch>/config` and `XDG_DATA_HOME=<scratch>/data`, both outside the fixture HOME. `XDG_CACHE_HOME` is unset.
  - The stored token lives at `<scratch>/config/gsd-meta-manager/driver-oauth-token`.
  - Consequence: the rules below must add no argv entry for that layout. Concretely, no outside-`$HOME` null mask for the token (the app-root dir mask covers it), and no relocated tmpfs/ro entry for a directory that does not exist.
- **This host has `CLAUDE_CONFIG_DIR=/home/blk/.local/share/claude-profiles/player-health` in the executor's own shell.** Its `agents`, `hooks`, `settings.json` and so on are symlinks into `~/.claude`, `.credentials.json` is a real file there, and its hooks name `/home/blk/.claude/hooks/...`. Any test that hands a temp `$HOME` to an env-reading entry will now pick up these real paths (outside the temp home), which breaks it. Those tests are migrated to injected `HostDirs` (Task 1 and Task 3 list them). CI has no `CLAUDE_CONFIG_DIR`. Task 3 runs the targeted suites both ways.
- `tests/oneoff_runtime_seam_guard.rs` fails if `src/oneoff/launch.rs` or `src/oneoff/resolve.rs` contain the agent's name (case-insensitive) on any non-`//` line. launch.rs may therefore pass `&HostDirs` around but must never name its Claude-root field or say the word. Only `src/oneoff/runtime.rs` (the seam) may.
- Baseline: `cargo test --no-fail-fast` gives 3827 passed, 0 failed, 19 ignored. Per-module lib counts:

  | Module | Tests |
  |--------|-------|
  | launcher::auth | 23 |
  | launcher::config_dir | 19 |
  | launcher::home | 17 |
  | launcher::sandbox | 15 |
  | oneoff::runtime | 18 |

  `cargo clippy --locked --target x86_64-pc-windows-gnu --all-targets -- -D warnings` is clean (17 s). All three release.yml cross targets are installed locally.

## Interfaces (Task 1 creates them; Tasks 2 and 3 consume them)

`src/launcher/host_dirs.rs`, not cfg-gated. Register it with `pub mod host_dirs;` next to `pub mod home;` in `src/launcher/mod.rs`.

- `#[derive(Debug, Clone, PartialEq, Eq)] pub struct HostDirs` with these `pub` fields:
  - `home: PathBuf`
  - `claude_root: PathBuf`: `resolve_config_root(env("CLAUDE_CONFIG_DIR"), Some(home))`, falling back to `<home>/.claude` without `unwrap`.
  - `config_home: PathBuf`: `XDG_CONFIG_HOME` when non-empty and absolute, else `<home>/.config`.
  - `data_home: PathBuf`: `XDG_DATA_HOME`, else `<home>/.local/share`.
  - `cache_home: PathBuf`: `XDG_CACHE_HOME`, else `<home>/.cache`.
  - `runtime_dir: Option<PathBuf>`: `XDG_RUNTIME_DIR` when non-empty and absolute, else `None`.
  - `run_token: Option<PathBuf>`: the stored run token's path.
- `pub fn resolve(home: &Path, env: impl Fn(&str) -> Option<OsString>) -> HostDirs`. Pure; leaves `run_token` as `None`. The one place the XDG rule lives: a private helper "non-empty absolute value wins, else default", the same semantics as `resolve_config_root` and as `dir_from_env` in home.rs.
- `pub fn from_host(home: &Path) -> HostDirs`. Calls `resolve(home, std::env::var_os)` and sets `run_token` from `super::auth::token_path()` on linux/macos, `None` elsewhere. Write it so the Windows build has no unused `mut` and no dead private helper: either a cfg'd one-line helper that returns the token path, or `#[cfg_attr(not(any(target_os = "linux", target_os = "macos")), allow(unused_mut))]` as sandbox.rs already does.
- `pub fn claude_roots(&self) -> Vec<PathBuf>` returns `[claude_root, <home>/.claude]`, lexically deduplicated (one entry when equal).
- `pub fn config_homes(&self) -> Vec<PathBuf>` returns `[<home>/.config, config_home]`, deduplicated.
- `pub fn credential_dirs(&self) -> Vec<PathBuf>` returns `TMPFS_MASK_DIRS` joined to home, in order, then `config_home/{gh,gcloud,glab-cli}` and `data_home/keyrings`, deduplicated with first occurrence kept.
- `pub fn claude_credential_files(&self) -> Vec<PathBuf>` returns `[claude_root/.credentials.json, claude_root/.claude.json]`. Claude Code keeps `.claude.json` inside `$CLAUDE_CONFIG_DIR` when that is set.
- `pub fn credential_files(&self) -> Vec<PathBuf>` returns `NULL_MASK_FILES` joined to home, in order, then `claude_credential_files()`, `config_home/git/credentials` and `run_token`, deduplicated with first occurrence kept.
- `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum Placement { UnderHome, UnderTmp, Maskable, Unmaskable }`.
- `pub fn placement(path: &Path, home: &Path, protected: &[PathBuf]) -> Placement`. Lexical and component-wise, checked in this order:
  - path starts with home (equal included): `UnderHome`;
  - path starts with `/tmp`: `UnderTmp`;
  - path is not absolute, or is `/`, or is an ancestor-or-equal of home, of `/tmp` or of any `protected` path: `Unmaskable`;
  - otherwise `Maskable`.

The constants `TMPFS_MASK_DIRS` / `NULL_MASK_FILES` stay in sandbox.rs as the `$HOME`-relative defaults; host_dirs.rs imports them. The new relocation tables live in host_dirs.rs:
- `XDG_CONFIG_CREDENTIAL_DIRS = ["gh", "gcloud", "glab-cli"]`
- `XDG_DATA_CREDENTIAL_DIRS = ["keyrings"]`
- `XDG_CONFIG_CREDENTIAL_FILES = ["git/credentials"]`

Each carries a doc line pointing at its `$HOME`-relative twin.

## Decisions (all `[inferred — audit]`)

- **I-1. One resolver for the launcher: `HostDirs`.**
  - `CLAUDE_CONFIG_DIR` goes through the existing `resolve_config_root` (reused, never modified).
  - `XDG_*` go through one private helper with identical "non-empty absolute, else default" semantics. The `dirs` crate is not reused for these because it cannot be injected in tests and ignores XDG on macOS, while gh/gcloud honour it. `InstallRoots::config_home` is not reused because of its relative and `~` semantics.
  - The run token keeps `auth::token_path()`, the resolver its writer uses.
  - A literal `~/x` in `CLAUDE_CONFIG_DIR` counts as relative, so unset, as Claude Code refuses it.
- **I-2. Host token comes from the configured Claude root only.** When `CLAUDE_CONFIG_DIR` is set, there is no fallback to `~/.claude`. That mirrors Claude Code and avoids launching on another account's token.
- **I-3. Seeding links and transforms from the configured Claude root only.** `~/.claude` is not merged, just as Claude Code does not merge it.
- **I-4. Binds and masks are the union of the default and the relocated locations.**
  - Read-only GSD entries and rtk config (non-credential) are bound at both, so claude-profile hosts whose hooks name `~/.claude/...` keep working.
  - Masks never drop a default.
  - Per-run tmpfs dirs keep the default and add the relocated one.
- **I-5. Outside-`$HOME` policy:**

  | What | Placement | Treatment |
  |------|-----------|-----------|
  | A Claude root | `UnderHome` | Its GSD entries are bound like any `$HOME` entry. |
  | A Claude root | `UnderTmp` | Its GSD entries are re-bound; /tmp is already empty, so no extra tmpfs. |
  | A Claude root | `Maskable` (protected = every tool hit and resolved path, the exe, `cargo_home`, `rustup_home`) | `HomeBind::Tmpfs` of `CLAUDE_ROOT_TMPFS_SIZE` = 1 MiB (the driver never writes there) plus `HomeBind::Ro` of each existing GSD entry. |
  | A Claude root | `Unmaskable` | Nothing. |
  | Its `.credentials.json` / `.claude.json` | anything but `UnderHome` / `UnderTmp` | `/dev/null`-masked when they exist. |
  | Relocated gh / gcloud / glab-cli / git / keyrings dirs | outside `$HOME` and /tmp, existing | `--tmpfs` masks through the existing masks list. |
  | Relocated cache / rtk-data | `UnderHome` | Per-run tmpfs. |
  | Relocated cache / rtk-data | `Maskable` and existing | Per-run tmpfs. bwrap cannot mkdir on the read-only root. |

  Residual, documented: other contents of relocated XDG dirs outside `$HOME` stay readable read-only, as today. Masking them whole would hide toolchains installed under them.
- **I-6. `config_home/git` outside `$HOME` is masked as a directory.** This also stops the host git config from reaching the driver, restoring T-28.1-33 for relocated XDG. It is a bug fixed along the way.
- **I-7. `masks_outside` keeps only `Placement::Maskable` candidates**, so it now also drops `/` and any ancestor of `$HOME` or `/tmp`. This is defensive and unchanged for every existing case.
- **I-8. Impure entries keep their signatures as one-line wrappers.** These are `HostFacts::from_host`, `HomeSetup::for_host` and `SandboxSpec::from_host`. Each resolves `HostDirs::from_host(home)` and calls a new `_with(&HostDirs, ...)` variant. The live tests keep calling the wrappers with the real home and env. Production (`run.rs`, `oneoff/launch.rs`) and every hermetic test use the `_with` variants with one `HostDirs` per launch or session.
- **I-9. GSD-hook recognition.** A hook counts as GSD when it names `.claude/<dir>/` (existing markers) OR `<claude root>/<dir>/` for any Claude root, where `<dir>` is one of `hooks`, `gsd-core`, `get-shit-done`, `scripts`.
- **I-10. Out of scope.** `XDG_STATE_HOME`, `GSD_HOME`, and the `.gsd` / `.gsd-capabilities.json` binds stay `$HOME`-relative. Record `GSD_HOME` as an observation in the SUMMARY: those binds ignore it. The XDG_RUNTIME_DIR handling in `driverctl/serve.rs` is out of scope, as is the sibling-owned `claude.rs`.
- **I-11.** `src/oneoff/resolve.rs` is checked and stays unchanged.
- **I-12.** A one-off session resolves `HostDirs` once in `run_session` and passes `&HostDirs` to the runtime seam.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: End-to-end "host token follows CLAUDE_CONFIG_DIR": the HostDirs resolver, auth, driver relaunch and egress re-read</name>
  <files>src/launcher/host_dirs.rs, src/launcher/mod.rs, src/launcher/auth.rs, src/launcher/run.rs, tests/launcher_caps.rs, tests/launcher_auth.rs, tests/launcher_config_dir.rs</files>
  <read_first>src/launcher/auth.rs (lines 1-60, 175-245, 420-480), src/launcher/run.rs (lines 440-470, 630-660, 765-800, 1125-1150, 1712-1735), src/agents/adapters/claude.rs (lines 84-112, read only), src/launcher/home.rs (lines 455-466, dir_from_env), src/launcher/sandbox.rs (lines 225-265), tests/launcher_caps.rs (lines 430-470, 585-605), tests/launcher_auth.rs (lines 40-80, 215-250), tests/launcher_config_dir.rs (lines 1-30, 594-625), tests/launcher_support/mod.rs (lines 380-405)</read_first>
  <behavior>
    - A `driver launch --unsandboxed`, run with CLAUDE_CONFIG_DIR set to a profile dir that holds `.credentials.json` (`at-host-1`) while `$HOME/.claude/.credentials.json` holds a decoy, gives the driver `at-host-1`. Its two auth relaunches re-read the profile file (`at-host-2`, `at-host-3`). The decoy never appears.
    - `driver auth status` with CLAUDE_CONFIG_DIR=profile prints the profile token's expiry. With CLAUDE_CONFIG_DIR set to a relative path it prints `host token: none found`: relative counts as unset, and the default dir is empty.
    - `HostDirs::resolve` behaves as follows:
      - With no env it yields exactly the defaults.
      - Absolute values win for each variable.
      - "", "   ", "rel/dir" and "~/x" all count as unset.
      - `claude_root` equals `resolve_config_root(value, Some(home))` for every one of those inputs.
      - `claude_roots` deduplicates.
      - The credential lists start with the `$HOME`-relative defaults in their constant order, then add the relocated paths and `run_token`.
      - `placement` covers UnderHome (equal included), UnderTmp, `/`, an ancestor of home, a protected ancestor, and Maskable.
  </behavior>
  <action>
Start red.
- In tests/launcher_caps.rs:
  - Generalise `write_host_credentials` into `write_credentials_at(dir, access, expires_in_s)` and keep `write_host_credentials` delegating with `fx.home().join(".claude")` as the dir.
  - Generalise `revoked_scenario` into `revoked_scenario_writing(next_expiry_s, creds_file)`, which writes the rotated token to the given absolute path. Keep `revoked_scenario` delegating with the existing `$HOME`-based path text.
  - Add `an_unsandboxed_launch_takes_the_host_token_from_claude_config_dir`:
    - set `profile = fx.home().join(".local/share/claude-profiles/work")`;
    - remove the stored token with `fx.write_token(None)`;
    - write `at-host-1` credentials into profile and a decoy `at-decoy-default` into `fx.home()/.claude`;
    - use the scenario that rewrites the profile file;
    - run `fx.launch_command(RUN_ID, &["--unsandboxed"])` with `.env("CLAUDE_CONFIG_DIR", &profile)`;
    - assert success, three launch lines, `tokens.txt` holding exactly the three lines `at-host-1`, `at-host-2` and `at-host-3` (each newline-terminated, in that order), and the decoy absent from `tokens.txt` and the run log.
- In tests/launcher_auth.rs:
  - Generalise `host_credentials` into `host_credentials_at(fx, dir, minutes)`, keeping the old helper delegating.
  - Add `status_reads_the_host_token_from_claude_config_dir`. Store no token and write credentials only into the profile dir. Then:
    - `auth(&fx, "status")` with `.env("CLAUDE_CONFIG_DIR", &profile)` shows `host token: expires` and the returned expiry prefix;
    - the same command with `.env("CLAUDE_CONFIG_DIR", "relative/profile")` shows `host token: none found`.
- Run both tests and confirm they FAIL before the fix: the decoy token is used, and the profile token is not found.

Then implement, per I-1 and I-2:
- Create src/launcher/host_dirs.rs exactly as the context's Interfaces section specifies. Give it a module doc that names quick 261113-dck, the blank/relative rule and why each resolver was chosen (I-1). Add unit tests covering every behavior bullet above; at least 7 `#[test]` fns, none reading or writing the process environment. Register the module in src/launcher/mod.rs, not cfg-gated.
- In src/launcher/auth.rs:
  - Add `pub fn host_claude_dir() -> Option<PathBuf>`: `HostDirs::from_host` of `dirs::home_dir()`, taking `claude_root`.
  - Add `pub fn resolve_launch_token_in(token_path: Option<&Path>, host_claude_dir: Option<&Path>, now: DateTime<Utc>) -> Result<LaunchToken, AuthError>`. It is the stored-token-first logic that `resolve_launch_token` has today. `None` for the Claude dir means `AuthError::Missing`.
  - Make `resolve_launch_token(now)` delegate to it with `token_path()` and `host_claude_dir()`.
  - Make `auth_status(now)` pass `host_claude_dir()` to `auth_status_in`.
  - Remove every hard-coded `<home>/.claude` path from auth.rs's production code.
  - Update the module doc's D-22 sentence to say the host token comes from `${CLAUDE_CONFIG_DIR:-~/.claude}/.credentials.json`.
  - Add a unit test `resolve_launch_token_in_falls_back_to_the_given_claude_dir`, using a tempdir profile and a missing token path.
- In src/launcher/run.rs:
  - Right after the home directory is resolved in the launch entry (~L652), compute `let host_dirs = HostDirs::from_host(&home);`.
  - Add a `host_dirs: HostDirs` field to `LaunchContext`, with a doc line citing quick 261113-dck and set in the context literal (~L769).
  - `auth_relaunch` passes `&ctx.host_dirs.claude_root` to `host_access_token`. `EgressGuard::start` builds `TokenProvider::Host` with `claude_dir: ctx.host_dirs.claude_root.clone()`.
  - Leave the `SeedSpec` literal (~L740) for Task 2. Changing its Claude dir before the seed knows the real `$HOME` would mis-expand hooks.
- In tests/launcher_config_dir.rs, `no_refresh_token_material_reaches_the_sandbox` mutates the process environment and would now read this host's real profile:
  - Replace its `resolve_launch_token` call with `resolve_launch_token_in(Some(&home.path().join("no-stored-token")), Some(&host_claude), Utc::now())`.
  - Delete its two process-environment writes (the HOME override and the XDG_CONFIG_HOME removal) and the mutex static that only serialised them. Remove any import that becomes unused.
  - Leave the test's own assertions unchanged.

Commit as `feat(quick-261113-dck): host token follows CLAUDE_CONFIG_DIR through one HostDirs resolver`.
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail(){ echo "FAIL-$1"; exit 1; }; L=target/dck-t1.log; ran(){ cargo test --lib "$1" > "$L" 2>&1; sed -nE "s/^test result: ok\. ([0-9]+) passed; 0 failed.*/\1/p" "$L"; }; prod(){ sed -E "/^ *#\[cfg\((all\()?test/,\$d" "$1" | grep -v "^\s*//"; }; n=$(ran launcher::host_dirs::); [ -n "$n" ] && [ "$n" -ge 7 ] || fail host-dirs; n=$(ran launcher::auth::); [ -n "$n" ] && [ "$n" -ge 24 ] || fail auth; cargo test --test launcher_caps --test launcher_auth --test launcher_config_dir > "$L" 2>&1 || fail integration; grep -q "^test an_unsandboxed_launch_takes_the_host_token_from_claude_config_dir \.\.\. ok$" "$L" || fail caps-tracer; grep -q "^test status_reads_the_host_token_from_claude_config_dir \.\.\. ok$" "$L" || fail auth-status; [ "$(prod src/launcher/auth.rs | grep -c "join(\".claude\")")" = 0 ] || fail auth-join; [ "$(prod src/launcher/run.rs | grep -c "ctx.home.join(\".claude\")")" = 0 ] || fail run-join; [ "$(grep -c "set_var" tests/launcher_config_dir.rs)" = 0 ] || fail env-set; [ "$(grep -c "remove_var" tests/launcher_config_dir.rs)" = 0 ] || fail env-remove; echo verify-ok'</automated>
    Baselines, run against HEAD:
    - auth.rs production `join(".claude")`: [HEAD=2 EXPECT==0]
    - run.rs production `ctx.home.join(".claude")`: [HEAD=2 EXPECT==0]
    - tests/launcher_config_dir.rs `set_var`: [HEAD=1 EXPECT==0]
    - tests/launcher_config_dir.rs `remove_var`: [HEAD=1 EXPECT==0]
    - lib launcher::auth: [HEAD=23 EXPECT>=24]
    - lib launcher::host_dirs: [HEAD=0 EXPECT>=7]
  </verify>
  <done>Task 1 is done when all of the following hold:
- Both new end-to-end tests failed before the fix and pass after it.
- `HostDirs` exists with at least 7 env-free unit tests.
- The driver's initial host token, its auth relaunch, its egress Host provider and `driver auth status` all read `${CLAUDE_CONFIG_DIR:-~/.claude}` (absolute only).
- No test in tests/launcher_config_dir.rs writes the process environment.
- The block prints verify-ok, and the commit is made path-scoped.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Per-run config seeding and one-off sessions follow the configured Claude root, with the real $HOME for hooks</name>
  <files>src/launcher/config_dir.rs, src/launcher/run.rs, src/oneoff/runtime.rs, src/oneoff/launch.rs, tests/launcher_config_dir.rs, tests/launcher_live_isolation.rs, tests/launcher_live_managed.rs, tests/launcher_caps.rs</files>
  <read_first>src/launcher/config_dir.rs (lines 1-45, 165-200, 240-250, 440-460, 515-660, 795-870, 885-975, 975-1000, 1065-1085), src/oneoff/runtime.rs (lines 620-640, 720-800, 800-810), src/oneoff/launch.rs (lines 795-870), tests/oneoff_runtime_seam_guard.rs (lines 1-60), tests/launcher_config_dir.rs (lines 330-400, 620-640, 690-705), tests/launcher_live_isolation.rs (lines 320-332), tests/launcher_live_managed.rs (lines 186-196)</read_first>
  <behavior>
    - An unsandboxed `driver launch` with CLAUDE_CONFIG_DIR=profile, where profile has `agents/` and `settings.json` `{"gsdmmProbe":"profile"}` and `$HOME/.claude` has `{"gsdmmProbe":"default"}`, leaves the per-run config dir with an `agents` link whose target is `profile/agents`, and a settings.json whose `gsdmmProbe` is `"profile"`.
    - seed with `host_claude_dir = <root>/profiles/work` and `host_home = <root>/home` keeps a hook `bash $HOME/.claude/hooks/gsd-x.sh` when the keep roots contain `<root>/home/.claude/hooks`, so `$HOME` expands against `host_home`.
    - A hook naming `<profile>/hooks/gsd-y.js` is kept when `<profile>/hooks` is a keep root, even though the profile is not named `.claude`. A hook naming `<profile>/bin/other.sh` is dropped.
    - The one-off runtime's `seed_config` with a `HostDirs` whose `claude_root` is a tempdir profile links that profile's entries. Its `egress_token` with a host credential yields `TokenProvider::Host` whose `claude_dir` is that profile.
  </behavior>
  <action>
Start red.
- Add `an_unsandboxed_launch_seeds_from_claude_config_dir` to tests/launcher_caps.rs. Reuse Task 1's helpers and the simplest scenario in that file that ends one launch normally. Assert the link target with `fs::read_link(fx.config_dir().join("agents"))` and read `gsdmmProbe` from `fx.config_dir().join("settings.json")`.
- Add `seed_expands_home_against_host_home_not_the_claude_dirs_parent` and `a_gsd_hook_under_a_relocated_claude_root_is_kept` to tests/launcher_config_dir.rs. They fail to compile until `host_home` exists, which counts as red.

Then implement, per I-3, I-9 and I-12:
- In src/launcher/config_dir.rs:
  - Add `pub host_home: PathBuf` to `SeedSpec` and `OneoffSeedSpec`. Its doc says it is the host `$HOME` that hook `~`/`$HOME` expansion and the keep-root check use, and that it is NOT derived from `host_claude_dir`, which is `$CLAUDE_CONFIG_DIR` when that is set.
  - In `seed` and `seed_oneoff`, replace the line that derives the host home from the Claude dir's parent with `&spec.host_home`.
  - Pass the Claude roots `[spec.host_claude_dir, spec.host_home/.claude]`, lexically deduplicated, to a new `settings_transform_in(host, gate, home, claude_roots, keep_roots)`.
  - Keep `settings_transform(host, gate, home, keep_roots)` as a wrapper that passes only `<home>/.claude`.
  - Add `GSD_HOOK_DIRS = ["hooks", "gsd-core", "get-shit-done", "scripts"]` and `pub fn hook_is_kept_in(command, home, claude_roots, roots)`. A command names GSD when the existing `GSD_HOOK_MARKERS` match OR its expanded text contains `<root>/<dir>/` for any Claude root and any GSD hook dir.
  - Keep `hook_is_kept(command, home, roots)` as a wrapper with `<home>/.claude`.
  - Thread the Claude roots through `filter_user_hooks`.
  - Add the unit test `hook_is_kept_in_recognises_every_claude_root` to config_dir.rs's own test module. It shows that a GSD hook under a root not named `.claude` is kept only via `hook_is_kept_in`, and that the `hook_is_kept` wrapper still behaves as before.
  - In the module doc (lines 1-12), say the links and settings come from `${CLAUDE_CONFIG_DIR:-~/.claude}`.
- In src/launcher/run.rs, set the `SeedSpec` literal's `host_claude_dir` to `host_dirs.claude_root.clone()` and `host_home` to `home.clone()`. After this, run.rs's production code has no hard-coded `<home>/.claude` path left.
- In src/oneoff/runtime.rs:
  - `egress_token(self, credential, host: &HostDirs)` builds `TokenProvider::Host { claude_dir: host.claude_root.clone(), .. }`.
  - `seed_config(self, config_dir, host: &HostDirs, binary, report_dir, managed, keep_roots)` fills `OneoffSeedSpec` with `host_claude_dir: host.claude_root.clone()` and `host_home: host.home.clone()`.
  - Add the unit test `seed_config_and_egress_token_follow_the_relocated_claude_root` to the existing test module inside the gated `imp` block (~L804), using `HostDirs::resolve` with an injected `CLAUDE_CONFIG_DIR`.
  - No hard-coded `<home>/.claude` path may remain in runtime.rs's production code.
- In src/oneoff/launch.rs `run_session`, compute `let host_dirs = HostDirs::from_host(spec.home);` once at the top and pass `&host_dirs` to `seed_config` and `egress_token`. tests/oneoff_runtime_seam_guard.rs forbids the agent's name on any non-comment line of launch.rs, so never name the Claude-root field there. Only runtime.rs reads it.
- Add `host_home` to every remaining literal:
  - src/launcher/config_dir.rs unit fixtures: `oneoff_spec` ~L980 and `spec` ~L1071, each the tempdir root that is the parent of their `host` dir.
  - tests/launcher_config_dir.rs: `spec_in` ~L384 gets `root`. The literals ~L625 and ~L694 get `home.path()` and `root.path()` respectively, matching the parent their `host_claude_dir` had.
  - The test at ~L365 that reassigns `host_claude_dir` to `home.join(".claude")` must also set `host_home` to `home`.
  - tests/launcher_live_isolation.rs ~L325 and tests/launcher_live_managed.rs ~L189 get `home.clone()`. These live tests use the real home and are `#[ignore]`d; they only have to compile.

Commit as `feat(quick-261113-dck): seed per-run config from CLAUDE_CONFIG_DIR with the real host home`.
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail(){ echo "FAIL-$1"; exit 1; }; L=target/dck-t2.log; ran(){ cargo test --lib "$1" > "$L" 2>&1; sed -nE "s/^test result: ok\. ([0-9]+) passed; 0 failed.*/\1/p" "$L"; }; prod(){ sed -E "/^ *#\[cfg\((all\()?test/,\$d" "$1" | grep -v "^\s*//"; }; n=$(ran oneoff::runtime::); [ -n "$n" ] && [ "$n" -ge 19 ] || fail runtime; n=$(ran launcher::config_dir::); [ -n "$n" ] && [ "$n" -ge 20 ] || fail config-dir-lib; cargo test --test launcher_config_dir --test launcher_caps --test oneoff_runtime_seam_guard > "$L" 2>&1 || fail integration; for t in an_unsandboxed_launch_seeds_from_claude_config_dir seed_expands_home_against_host_home_not_the_claude_dirs_parent a_gsd_hook_under_a_relocated_claude_root_is_kept runtime_neutral_files_never_name_the_agent; do grep -q "^test $t \.\.\. ok$" "$L" || fail "$t"; done; cargo test --no-run --test launcher_live_isolation --test launcher_live_managed > "$L" 2>&1 || fail live-compile; [ "$(prod src/launcher/run.rs | grep -c "join(\".claude\")")" = 0 ] || fail run-join; [ "$(prod src/oneoff/runtime.rs | grep -c "join(\".claude\")")" = 0 ] || fail runtime-join; [ "$(prod src/launcher/config_dir.rs | grep -c "host_claude_dir.parent()")" = 0 ] || fail parent-home; echo verify-ok'</automated>
    Baselines, run against HEAD:
    - run.rs production `join(".claude")`: [HEAD=3 EXPECT==0]
    - runtime.rs production `join(".claude")`: [HEAD=2 EXPECT==0]
    - config_dir.rs production `host_claude_dir.parent()`: [HEAD=2 EXPECT==0]
    - lib oneoff::runtime: [HEAD=18 EXPECT>=19]
    - lib launcher::config_dir: [HEAD=19 EXPECT>=20]
  </verify>
  <done>Task 2 is done when all of the following hold:
- The per-run config of a driver launch and of a one-off session is seeded from `${CLAUDE_CONFIG_DIR:-~/.claude}`.
- Hook expansion uses the real `$HOME`, and GSD hooks under a relocated root are kept.
- The seam guard still passes, and every SeedSpec and OneoffSeedSpec literal compiles, live tests included.
- The block prints verify-ok, and the commit is made path-scoped.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Sandbox binds and masks honour the configured Claude root and the XDG dirs; outside-$HOME policy, tests and docs</name>
  <files>src/launcher/home.rs, src/launcher/sandbox.rs, src/launcher/config_dir.rs, src/launcher/run.rs, src/oneoff/launch.rs, tests/launcher_sandbox.rs, tests/launcher_sandbox_data_mask.rs, docs/DRIVER-SANDBOX.md, docs/CONFIGURATION.md</files>
  <read_first>src/launcher/home.rs (whole production region lines 1-560; tests lines 559-700, 754-800, 860-930), src/launcher/sandbox.rs (lines 1-75, 225-265, 340-470, 560-640, 1370-1395), src/launcher/config_dir.rs (lines 140-160), src/launcher/run.rs (lines 715-735, 1585-1605), src/oneoff/launch.rs (lines 805-875), tests/launcher_sandbox.rs (lines 195-305, 700-760), tests/launcher_sandbox_data_mask.rs (lines 1-60, 110-140), docs/DRIVER-SANDBOX.md (lines 10-45, 100-120, 160-195), docs/CONFIGURATION.md (lines 108-130, 415-430)</read_first>
  <behavior>
    - Default host (no CLAUDE_CONFIG_DIR/XDG): `the_golden_allowlist_with_overlay` and every existing allowlist, argv and data-mask test pass unchanged.
    - CLAUDE_CONFIG_DIR under $HOME: the allowlist has Ro entries for the existing GSD entries of both the profile and ~/.claude. A tool binary at `<profile>/bin/x` binds only the file, because the profile is a refused root.
    - CLAUDE_CONFIG_DIR outside $HOME and /tmp: the allowlist has `Tmpfs{profile, 1 MiB}` before `Ro(profile/agents)` and the other existing entries. In the argv, `--tmpfs <profile>` precedes `--ro-bind <profile>/agents <profile>/agents`, and `--ro-bind /dev/null <profile>/.credentials.json` is present.
    - CLAUDE_CONFIG_DIR under /tmp gives Ro entries only. CLAUDE_CONFIG_DIR equal to `/` or an ancestor of $HOME gives no tmpfs, only the /dev/null masks of its two credential files.
    - XDG_CONFIG_HOME relocated under $HOME (`~/cfg`):
      - `~/cfg/rtk` is bound when it exists;
      - `~/cfg` is a refused root;
      - `~/cfg/gh` counts as a mask dir, so no allowlist entry overlaps it;
      - a run token at `~/cfg/gsd-meta-manager/driver-oauth-token` inside a bind is /dev/null-masked;
      - XDG_CACHE_HOME `~/xcache` gets a per-run tmpfs next to the default `~/.cache`.
    - XDG dirs outside $HOME: `sandbox_dir_masks_in` returns the existing app roots, then existing `config_home/{gh,gcloud,glab-cli,git}` and `data_home/keyrings`. It returns nothing under $HOME or /tmp, and nothing that does not exist.
    - `masks_outside` drops `/`, `/home`, `/tmp` and any ancestor of home.
  </behavior>
  <action>
Start red: write the new unit and argv tests named in the verify block. They fail to compile or assert until the code below exists.

In src/launcher/home.rs, per I-4 and I-5:
- Add `pub dirs: HostDirs` to `HostFacts`, with the invariant `dirs.home == home`.
- Replace `HOME_RO_REL`'s eight Claude entries with `CLAUDE_RO_REL = ["agents", "commands", "skills", "gsd-core", "get-shit-done", "hooks", "scripts", ".gsd-profile"]`, named relative to a Claude root. `HOME_RO_REL` keeps `.gsd` and `.gsd-capabilities.json`.
- rtk config becomes `<each of dirs.config_homes()>/rtk`.
- Add `CLAUDE_ROOT_TMPFS_SIZE: u64 = 1_048_576` with an `[inferred — audit]` doc line.
- `touches_mask_dir` takes the facts and iterates `facts.dirs.credential_dirs()`.
- `root_refused` adds:
  - `dirs.config_home`, `dirs.data_home` and `dirs.cache_home` next to `REFUSED_ROOT_REL`;
  - every Claude root;
  - `facts.dirs.credential_files()` in place of the `$HOME`-relative null-mask list;
  - every per-run tmpfs path (default and relocated) in place of the two fixed ones.
- In `candidates`, for every `dirs.claude_roots()` entry, branch on `host_dirs::placement(root, home, protected)`, where protected is every tool `hit` and `real`, `exe`, `cargo_home` and `rustup_home`:
  - `UnderHome`: push `ro(root/e)` for each `CLAUDE_RO_REL` entry through `admissible` as today.
  - `UnderTmp`: push `HomeBind::Ro(root/e)` directly.
  - `Maskable`: push `HomeBind::Tmpfs { path: root, size: CLAUDE_ROOT_TMPFS_SIZE }`, then `HomeBind::Ro(root/e)` for each entry.
  - `Unmaskable`: push nothing.
- Per-run tmpfs: keep the default `~/.cache` and `~/.local/share/rtk` exactly as today. Add `dirs.cache_home` and `dirs.data_home/rtk` when they differ from the default and their placement is `UnderHome` or `Maskable`.
- In `home_allowlist`, keep a `Tmpfs` entry when it lies under `$HOME` or is in `facts.existing`. Ro and Overlay keep the existence rule.
- `dangling_within` also takes the Claude roots: the one-level skills scan applies to `<any Claude root>/skills` as well as to a parent named `.claude`. `dangling_symlinks` passes an empty list.
- In the gated `imp` module:
  - add `HostFacts::from_host_with(dirs: &HostDirs, exe)` and `HomeSetup::for_host_with(dirs, exe, overlay, state_dir)`;
  - turn `from_host` and `for_host` into one-line wrappers over `HostDirs::from_host(home)`.
- Update the module doc with a short "relocated host directories (quick 261113-dck)" paragraph.
- Update the test fixture `facts()` with `dirs: HostDirs::resolve(Path::new("/home/u"), |_| None)`.
- Extend `no_allowlist_entry_overlaps_a_mask_dir` to iterate `facts.dirs.credential_dirs()`, including a relocated-XDG facts value.
- Add at least five tests covering the behavior bullets.
- Migrate the three env-reading calls in the host_tests module (~L598, ~L655, ~L683) to the `_with` variants fed by `HostDirs::resolve(<temp home>, |_| None)`.

In src/launcher/sandbox.rs:
- Add `SandboxSpec::from_host_with(paths, dirs: &HostDirs, setup)` and make `from_host` a wrapper.
- Build the null masks from `dirs.credential_files()`. Keep a file when it exists AND it either lies inside a read-only bind, or it is one of `dirs.claude_credential_files()` with placement neither `UnderHome` nor `UnderTmp`. This keeps the harness argv unchanged: its token is covered by the app-root dir mask, not by a null mask.
- Delete the separate cfg-gated token block and its `allow(unused_mut)` attribute; `run_token` now arrives through `HostDirs`.
- `runtime_dir` comes from `dirs.runtime_dir`; delete the direct runtime-dir environment read.
- `masks_outside` keeps only `Placement::Maskable` candidates (empty protected list), deduplicated, in order (I-7). Update its doc.
- Update the module doc paragraph about the app-root mask helper so it names the renamed function.
- Migrate the unit test `from_host_keeps_only_null_masks_that_exist_inside_a_read_only_bind` to `from_host_with` with injected dirs.
- Add `from_host_with_masks_the_relocated_claude_credentials_outside_home` and `from_host_with_masks_the_run_token_where_token_path_puts_it`.

In src/launcher/config_dir.rs:
- Rename the app-root mask helper (~L151, used before `bwrap_argv_masking` in run.rs and oneoff/launch.rs) to `pub fn sandbox_dir_masks(dirs: &HostDirs) -> Vec<PathBuf>`. It takes the two app roots from `dirs::data_local_dir()` / `dirs::config_dir()` joined with `gsd-meta-manager` as today, and calls a new `pub fn sandbox_dir_masks_in(dirs: &HostDirs, app_roots: &[PathBuf]) -> Vec<PathBuf>`.
- That function builds the candidates in this order: app roots, `dirs.credential_dirs()`, `dirs.config_home/git` (I-6). It keeps existing directories and returns `masks_outside(&dirs.home, ...)`.
- Add the unit test `sandbox_dir_masks_in_masks_relocated_credential_dirs_outside_home`.

In src/launcher/run.rs:
- `HomeSetup::for_host_with(&host_dirs, ...)` (~L723);
- `SandboxSpec::from_host_with(paths, &ctx.host_dirs, &ctx.home_setup)` (~L1593);
- `sandbox_dir_masks(&ctx.host_dirs)` (~L1597);
- update the import.

In src/oneoff/launch.rs, use the session's `host_dirs` for `HomeSetup::for_host_with`, `SandboxSpec::from_host_with` and `sandbox_dir_masks`. Update the import. The agent's name still must not appear.

In tests/launcher_sandbox.rs:
- Migrate `null_masks_are_a_second_layer_inside_binds_only` and `no_flag_names_a_credential_directory_and_the_home_block_precedes_the_run_binds` to `HomeSetup::for_host_with` / `SandboxSpec::from_host_with` with `HostDirs::resolve(&home, |_| None)`.
- Migrate the ignored live test `runtime_dir_holds_only_the_driverctl_socket` to `from_host_with`, using dirs resolved from an env closure that returns only the real runtime dir for XDG_RUNTIME_DIR.
- Add `a_claude_root_outside_home_is_hidden_and_its_gsd_entries_rebound`. Use tempdirs for both a fake home and an outside profile with `agents/` and `.credentials.json`, inject CLAUDE_CONFIG_DIR, and assert the argv order from the behavior block.

In tests/launcher_sandbox_data_mask.rs, add `masks_outside_never_masks_an_ancestor_of_home_or_tmp`.

Docs:
- docs/DRIVER-SANDBOX.md:
  - In "Auth posture", say the host token is read from `${CLAUDE_CONFIG_DIR:-~/.claude}/.credentials.json`.
  - Add a section "Relocated host directories" after "What the sandbox does". It covers the CLAUDE_CONFIG_DIR and XDG_* rule (absolute only, blank or relative counts as unset), the union rule (I-4) and the outside-`$HOME` table from I-5/I-6.
  - Add a residual-risk bullet: other contents of relocated XDG dirs outside `$HOME` stay readable read-only.
- docs/CONFIGURATION.md:
  - Add an Environment variables row for `CLAUDE_CONFIG_DIR` (consumer `src/launcher/host_dirs.rs`, plus the existing state_reader and agents readers).
  - Add one row for `XDG_CONFIG_HOME` / `XDG_DATA_HOME` / `XDG_CACHE_HOME` / `XDG_RUNTIME_DIR` as the launcher uses them.
  - Extend the one-off "Sandbox" paragraph (~L422) to mention the relocated Claude root and credential stores.

Commit as `fix(quick-261113-dck): sandbox binds and credential masks follow CLAUDE_CONFIG_DIR and XDG dirs`. Commit the docs in the same commit or as `docs(quick-261113-dck): ...`.
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail(){ echo "FAIL-$1"; exit 1; }; L=target/dck-t3.log; ran(){ cargo test --lib "$1" > "$L" 2>&1; sed -nE "s/^test result: ok\. ([0-9]+) passed; 0 failed.*/\1/p" "$L"; }; prod(){ sed -E "/^ *#\[cfg\((all\()?test/,\$d" "$1" | grep -v "^\s*//"; }; n=$(ran launcher::home::); [ -n "$n" ] && [ "$n" -ge 22 ] || fail home; n=$(ran launcher::sandbox::); [ -n "$n" ] && [ "$n" -ge 17 ] || fail sandbox; n=$(ran launcher::config_dir::); [ -n "$n" ] && [ "$n" -ge 21 ] || fail config-dir; n=$(ran launcher::host_dirs::); [ -n "$n" ] && [ "$n" -ge 7 ] || fail host-dirs; cargo test --test launcher_sandbox --test launcher_sandbox_data_mask --test launcher_config_dir --test launcher_caps --test launcher_auth --test oneoff_runtime_seam_guard > "$L" 2>&1 || fail integration; for t in a_claude_root_outside_home_is_hidden_and_its_gsd_entries_rebound masks_outside_never_masks_an_ancestor_of_home_or_tmp runtime_neutral_files_never_name_the_agent; do grep -q "^test $t \.\.\. ok$" "$L" || fail "$t"; done; env -u CLAUDE_CONFIG_DIR cargo test --lib launcher:: > "$L" 2>&1 || fail hermetic-lib-unset; env -u CLAUDE_CONFIG_DIR cargo test --test launcher_sandbox --test launcher_caps --test launcher_auth --test launcher_config_dir > "$L" 2>&1 || fail hermetic-int-unset; [ "$(prod src/launcher/sandbox.rs | grep -c "var_os(\"XDG_RUNTIME_DIR\")")" = 0 ] || fail runtime-env-read; [ "$(grep -rn "app_root_masks" src | wc -l)" = 0 ] || fail masks-rename; [ "$(prod src/launcher/home.rs | grep -c "\"\.claude/")" = 0 ] || fail home-claude-rel; [ "$(grep -cE "(HomeSetup::for_host|SandboxSpec::from_host|HostFacts::from_host)\(" tests/launcher_sandbox.rs)" = 0 ] || fail sandbox-int-env; [ "$(sed -n "/^#\[cfg(all(test/,\$p" src/launcher/home.rs | grep -cE "(for_host|from_host)\(")" = 0 ] || fail home-tests-env; [ "$(sed -n "/^#\[cfg(test)\]/,\$p" src/launcher/sandbox.rs | grep -cE "from_host\(")" = 0 ] || fail sandbox-unit-env; grep -q "CLAUDE_CONFIG_DIR" docs/DRIVER-SANDBOX.md || fail doc-sandbox; grep -q "CLAUDE_CONFIG_DIR" docs/CONFIGURATION.md || fail doc-config; echo verify-ok'</automated>
    Baselines, run against HEAD:
    - sandbox.rs production `var_os("XDG_RUNTIME_DIR")`: [HEAD=1 EXPECT==0]
    - `app_root_masks` lines in src: [HEAD=6 EXPECT==0]
    - home.rs production `"\.claude/`: [HEAD=8 EXPECT==0]
    - tests/launcher_sandbox.rs env-reading entry calls: [HEAD=5 EXPECT==0]
    - home.rs test-region `(for_host|from_host)\(`: [HEAD=3 EXPECT==0]
    - sandbox.rs test-region `from_host\(`: [HEAD=1 EXPECT==0]
    - docs `CLAUDE_CONFIG_DIR` in DRIVER-SANDBOX.md: [HEAD=0 EXPECT>=1]
    - docs `CLAUDE_CONFIG_DIR` in CONFIGURATION.md: [HEAD=0 EXPECT>=1]
    - lib launcher::home: [HEAD=17 EXPECT>=22]
    - lib launcher::sandbox: [HEAD=15 EXPECT>=17]
    - lib launcher::config_dir: [HEAD=19 EXPECT>=21]
  </verify>
  <done>Task 3 is done when all of the following hold:
- The sandbox's `$HOME` allowlist, null masks and outside-`$HOME` dir masks are derived from `HostDirs`, for both the default and the relocated locations.
- A Claude root or credential store relocated outside `$HOME` is hidden per I-5/I-6. With no relocation, the argv is byte-identical to before.
- Every hermetic test injects `HostDirs`, and the targeted suites pass with `CLAUDE_CONFIG_DIR` set and with it unset.
- The docs describe the rule and the residual.
- The block prints verify-ok, and the commit is made path-scoped.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| sandboxed driver/agent -> host filesystem | Untrusted model-driven code runs under bwrap with `/` read-only. Anything readable there is exfiltratable through the inference proxy, commits and planning files. |
| host environment -> launcher path resolution | The operator's CLAUDE_CONFIG_DIR / XDG_* values decide which host paths are read, bound and masked. They may be blank, relative or degenerate (`/`, an ancestor of `$HOME`). |
| launcher -> Anthropic API (egress server) | The real OAuth token lives only in the launcher. Which file it comes from decides which account the run acts as. |

## STRIDE Threat Register (ASVS L1, block on high)

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-261113-dck-01 | Information disclosure | Claude root relocated outside `$HOME` (`.credentials.json` refresh token, `.claude.json`, transcripts, shell snapshots) | high | mitigate | home.rs places an empty 1 MiB tmpfs over a `Maskable` Claude root and re-binds only its GSD entries read-only. sandbox.rs `/dev/null`-masks its `.credentials.json` and `.claude.json` whenever they exist outside `$HOME`/`/tmp`. Every Claude root is a refused install root. Pinned by `a_claude_root_outside_home_is_hidden_and_its_gsd_entries_rebound` and the home.rs tests. |
| T-261113-dck-02 | Information disclosure | stored run token under a relocated `XDG_CONFIG_HOME` | high | mitigate | `HostDirs.run_token` comes from `auth::token_path()`, the token's own resolver. It is in `credential_files()`, so it feeds both the null masks and `root_refused`. The default `.config/...` entry stays. Outside `$HOME` the app-root dir mask (now `sandbox_dir_masks`) still covers it. Pinned by `from_host_with_masks_the_run_token_where_token_path_puts_it`. |
| T-261113-dck-03 | Information disclosure | gh / gcloud / glab-cli / keyrings / git-credentials under relocated XDG dirs | high | mitigate | Under `$HOME`: they join `credential_dirs()` and `credential_files()`, so no allowlist entry overlaps them and their parents are refused roots. Outside `$HOME` and `/tmp`: `sandbox_dir_masks_in` places an empty tmpfs over each existing dir. Pinned by `sandbox_dir_masks_in_masks_relocated_credential_dirs_outside_home` and the extended no-overlap invariant. |
| T-261113-dck-04 | Elevation of privilege | host git config (alias, include, credential.helper) via `$XDG_CONFIG_HOME/git` outside `$HOME` | medium | mitigate | `config_home/git` is a tmpfs mask candidate outside `$HOME` (I-6), restoring T-28.1-33. The per-run `~/.gitconfig` stays the only git config. |
| T-261113-dck-05 | Spoofing | host-token fallback read from `~/.claude` while Claude Code uses `$CLAUDE_CONFIG_DIR` (wrong account, or a token the operator thinks is logged out) | medium | mitigate | auth, auth_relaunch and the egress Host provider all read `HostDirs.claude_root` (I-2), with no fallback to `~/.claude` when the variable is set. Pinned by the two Task 1 end-to-end tests. |
| T-261113-dck-06 | Denial of service | a degenerate CLAUDE_CONFIG_DIR / XDG value (blank, relative, `/`, an ancestor of `$HOME`, `/tmp` or a toolchain) could tmpfs-mask the sandbox's `$HOME`, `/` or its tools | medium | mitigate | Blank or relative counts as unset (`resolve_config_root` semantics, unit-tested). `placement` refuses `/` and any ancestor of `$HOME`, `/tmp`, tool paths, `cargo_home`, `rustup_home` and the exe. `masks_outside` keeps only `Maskable`. Pinned by `masks_outside_never_masks_an_ancestor_of_home_or_tmp` and the placement tests. |
| T-261113-dck-07 | Tampering | GSD hooks recognised under a relocated root | low | accept | The roots are bound read-only, and a kept hook must still name only `$HOME` paths inside the keep roots (D-19 unchanged). Recognition widens only to the same trusted GSD install the operator's Claude Code already runs. |
| T-261113-dck-08 | Information disclosure | other contents of relocated XDG dirs outside `$HOME` stay readable read-only | low | accept | Masking `XDG_CONFIG_HOME`/`XDG_DATA_HOME` whole would hide toolchains installed under them. The named credential stores are masked, and the residual is documented in docs/DRIVER-SANDBOX.md. This is unchanged from today. |
| T-261113-dck-SC | Tampering | npm/pip/cargo installs | low | accept | No dependency is added. Every cross-target gate runs `--locked`. |
</threat_model>

<verification>
- Task 1 starts red: both end-to-end tests fail on the decoy or missing token before the fix.
- Tasks 2 and 3 start red: the new tests fail, or fail to compile, before the code exists.
- Full suite: `rtk bypass bash -c 'cargo test --no-fail-fast > target/dck-full.log 2>&1; echo EXIT=$?'` prints EXIT=0. `rtk bypass awk '/^test result:/ {p+=$4; f+=$6} END {print p, f}' target/dck-full.log` shows at least 3852 passed and 0 failed. Never pipe cargo output through grep without `rtk bypass`.
- `rtk bypass cargo clippy --all-targets -- -D warnings` exits 0.
- `rtk bypass cargo fmt --all -- --check` exits 0.
- For T in aarch64-apple-darwin, x86_64-apple-darwin and x86_64-pc-windows-gnu, both of these exit 0 (all three targets are installed locally; Windows takes about 17 s):
  - `rtk bypass cargo check --locked --target $T --all-targets`
  - `rtk bypass cargo clippy --locked --target $T --all-targets -- -D warnings`
  host_dirs.rs and the pure part of home.rs and sandbox.rs compile on every target, so no private helper may be used only from linux/macos-gated code.
- Hermeticity: Task 3's verify ran the launcher suites with the developer's CLAUDE_CONFIG_DIR set and with it unset. No test file writes the process environment for this feature.
- `git status` is clean after the last commit. `git log -1 --stat` shows only this plan's paths.
</verification>

<success_criteria>
- On this host (`CLAUDE_CONFIG_DIR=~/.local/share/claude-profiles/player-health`), a driver or one-off launch uses that profile's token, settings and GSD links, and keeps hooks that name `~/.claude/hooks/...`, because both roots' GSD entries are bound.
- With `XDG_CONFIG_HOME` or the Claude root relocated anywhere, no credential store named by the task (Claude credentials and `.claude.json`, gh, gcloud, glab-cli, keyrings, git credentials, the stored run token) is readable inside the sandbox. Default locations stay masked.
- The outside-`$HOME` decision is implemented and documented, and degenerate values cannot mask `$HOME`, `/`, `/tmp` or a toolchain.
- The SUMMARY records:
  - the measured stale part of the task premise (WR-02 and the app-root mask already covered the token);
  - the real gaps (a)-(d) and how each was closed;
  - I-1..I-12 for audit;
  - the GSD_HOME observation (I-10) as a follow-up candidate.
</success_criteria>

<output>
Create `.planning/quick/261113-dck-launcher-honours-claude-config-dir-and-xdg-dirs-for-host-pat/261113-dck-SUMMARY.md` when done
</output>
