---
phase: quick-261113-epo
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/config_json.rs
  - src/launcher/host_dirs.rs
  - src/launcher/home.rs
  - tests/launcher_sandbox.rs
  - docs/DRIVER-SANDBOX.md
  - docs/CONFIGURATION.md
autonomous: true
requirements: [QUICK-261113-epo]

estimate:
  tokens: 75000
  raw_tokens: 75000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "With GSD_HOME set to a non-empty absolute directory under $HOME, the driver/one-off bwrap argv carries `--ro-bind P P` for `$GSD_HOME/.gsd` and `$GSD_HOME/.gsd-capabilities.json` when they exist, and still carries the `~/.gsd` and `~/.gsd-capabilities.json` binds (union rule, I-3)."
    - "The GSD home is decided by ONE resolver, `state_reader::config_json::resolve_gsd_home_from` (gsd-core's `resolveGsdHome`: empty or unset means $HOME, any other value is used as given). `user_defaults_path` and `HostDirs::resolve` both call it; `src/launcher/host_dirs.rs` holds no second copy of the rule (I-1)."
    - "Outside $HOME, an absolute GSD home's `.gsd` and `.gsd-capabilities.json` are re-bound read-only when they exist, whether under /tmp or elsewhere, and no tmpfs is ever placed over a GSD home. A relative GSD_HOME adds no bind. An entry inside a credential directory is never named, and a relocated GSD home is never bound as a whole install root (I-4, I-5, I-6)."
    - "With GSD_HOME unset, `the_golden_allowlist_with_overlay` and every existing launcher test pass unchanged. Every new test injects its environment through `HostDirs::resolve` or `relocated_facts`. No test writes the process environment, and the launcher suites pass with GSD_HOME set in the process environment."
    - "docs/DRIVER-SANDBOX.md documents the GSD_HOME rule and its placement rows, and the old sentence saying these binds ignore GSD_HOME is gone. The GSD_HOME row in docs/CONFIGURATION.md names `HostDirs::resolve` as a consumer."
    - "Gates: `cargo test --no-fail-fast` and `cargo clippy --all-targets --locked -- -D warnings` exit 0, `cargo fmt --all -- --check` is clean, and `cargo clippy --locked --target T --all-targets -- -D warnings` exits 0 for each of aarch64-apple-darwin, x86_64-apple-darwin and x86_64-pc-windows-gnu that is installed."
  artifacts:
    - path: src/state_reader/config_json.rs
      provides: "pub fn resolve_gsd_home_from: the one GSD-home rule, shared by user_defaults_path and the launcher"
      contains: "pub fn resolve_gsd_home_from"
    - path: src/launcher/host_dirs.rs
      provides: "HostDirs.gsd_home resolved through resolve_gsd_home_from; HostDirs::gsd_homes() = [$HOME, absolute $GSD_HOME] deduplicated"
      contains: "pub fn gsd_homes"
    - path: src/launcher/home.rs
      provides: "GSD_RO_REL bound under every GSD home; outside-$HOME re-binds without a tmpfs; relocated GSD home refused as install root"
      contains: "pub const GSD_RO_REL"
    - path: tests/launcher_sandbox.rs
      provides: "end-to-end argv tests for a GSD home under $HOME and outside $HOME and /tmp"
      contains: "a_gsd_home_under_home_rebinds_its_store_and_keeps_the_default"
    - path: docs/DRIVER-SANDBOX.md
      provides: "GSD_HOME paragraph, union bullet and placement rows under Relocated host directories"
      contains: "GSD_HOME"
  key_links:
    - from: src/launcher/host_dirs.rs
      to: src/state_reader/config_json.rs
      via: "HostDirs::resolve calls resolve_gsd_home_from(env(GSD_HOME_ENV).as_deref(), Some(home)), the same resolver behind user_defaults_path"
      pattern: "resolve_gsd_home_from"
    - from: src/launcher/home.rs
      to: src/launcher/host_dirs.rs
      via: "candidates() iterates facts.dirs.gsd_homes() x GSD_RO_REL; root_refused() refuses every gsd_homes() entry"
      pattern: "gsd_homes()"
    - from: src/launcher/home.rs
      to: src/launcher/sandbox.rs
      via: "HomeSetup::for_host_with -> HomeSetup.binds -> SandboxSpec::from_host_with -> bwrap_argv emits the --ro-bind triples (sandbox.rs itself unchanged)"
      pattern: "HomeBind::Ro"
---

<objective>
The sandbox's read-only binds of `.gsd` and `.gsd-capabilities.json` honour `GSD_HOME` the way gsd-core does (`${GSD_HOME:-$HOME}`). Quick 261113-dck left them hard-coded to `$HOME` (its decision I-10, recorded as a follow-up). A host with `GSD_HOME` set binds `~/.gsd`. The driver, though, inherits `GSD_HOME` (it is not in the scrub lists) and reads `$GSD_HOME/.gsd` and `$GSD_HOME/.gsd-capabilities.json`, which the `$HOME` tmpfs hides when they sit under `$HOME`. So its capability overlays, consent store, defaults and global capability ledger go missing inside the sandbox.

Purpose: a sandboxed GSD run sees the same GSD store and capability ledger as the host GSD. Placement follows the outside-`$HOME` bind policy that 261113-dck documented.
Output: a shared GSD-home resolver in `src/state_reader/config_json.rs`, a `HostDirs.gsd_home` field plus `gsd_homes()`, the allowlist and install-root rules in `src/launcher/home.rs`, end-to-end argv tests, and corrected docs.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@.planning/quick/261113-dck-launcher-honours-claude-config-dir-and-xdg-dirs-for-host-pat/261113-dck-SUMMARY.md
@src/launcher/host_dirs.rs
@src/launcher/home.rs
@src/state_reader/config_json.rs
@tests/launcher_sandbox.rs
@docs/DRIVER-SANDBOX.md

<interfaces>
Measured at HEAD 7b7cb89. Use these directly; do not re-explore.

src/state_reader/config_json.rs (L733-774):
- `pub const GSD_HOME_ENV: &str = "GSD_HOME";`
- `pub fn user_defaults_path() -> Option<PathBuf>`: the env wrapper; it calls `user_defaults_path_from(var_os(GSD_HOME_ENV), var_os("HOME"))`.
- `fn user_defaults_path_from(gsd_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf>` (private). Body: `gsd_home.filter(|g| !g.is_empty()).or(home)?`, then push `.gsd` and `defaults.json`. Three unit tests pin it (L812-830).
- tests/state_reader_gsd_home.rs imports `load_user_defaults_from, user_defaults_path, GSD_HOME_ENV`. Keep those names and signatures.

src/launcher/host_dirs.rs:
- `pub struct HostDirs { home, claude_root, config_home, data_home, cache_home, runtime_dir: Option<PathBuf>, run_token: Option<PathBuf> }`, derives Debug, Clone, PartialEq and Eq. It is built in exactly one struct literal, in `HostDirs::resolve`.
- `pub fn resolve(home: &Path, env: impl Fn(&str) -> Option<OsString>) -> HostDirs` (pure); `pub fn from_host(home: &Path) -> HostDirs` (the impure entry).
- `fn dedup(paths: Vec<PathBuf>) -> Vec<PathBuf>` keeps the first occurrence. `config_homes()` returns `[<home>/.config, config_home]`, deduplicated.
- The tests use helpers `env_of(&[(k, v)])` and `home()` (= /home/u).

src/launcher/home.rs:
- `pub const HOME_RO_REL: &[&str] = &[".gsd", ".gsd-capabilities.json"];` (L74). Its only uses are the module doc on L14 and `candidates()` on L315.
- `fn candidates(facts: &HostFacts, overlay: bool) -> Vec<HomeBind>`. Its local `ro` closure pushes `HomeBind::Ro` only when `admissible` holds (strictly under $HOME and clear of every credential dir). L315-317 holds `for rel in HOME_RO_REL { ro(home.join(rel), &mut out); }`. The Claude-root loop (L321-346) is the placement model to mirror.
- `fn root_refused(root: &Path, facts: &HostFacts) -> bool` (L238) already refuses `dirs.claude_roots()` and the XDG homes.
- `fn touches_mask_dir(path: &Path, facts: &HostFacts) -> bool` (L186).
- `pub fn home_allowlist(facts, overlay)` names a non-tmpfs entry only when it is in `facts.existing`, sorts parents first, and drops entries nested in an earlier ro or overlay entry.
- In `mod tests`, `facts()` builds a host with no env. `relocated_facts(env: &[(&str, &str)]) -> HostFacts` resolves the dirs from env and marks every candidate as existing. `paths(&binds)` renders `"ro /p"` / `"tmpfs /p"` strings. `the_golden_allowlist_with_overlay` pins `ro /home/u/.gsd` and `ro /home/u/.gsd-capabilities.json`.

src/launcher/sandbox.rs (unchanged by this plan):
- `child_env` passes the host env minus the strip lists, and GSD_HOME is not stripped, so the driver inherits it.
- `bwrap_argv_masking` order: `--ro-bind / /`, the `/tmp` tmpfs, the `/etc` block, the `/run` tmpfs, the `$HOME` tmpfs, home entries outside the project, the dir masks, the project `--bind`, then home entries inside the project (WR-05).

tests/launcher_sandbox.rs (`#![cfg(target_os = "linux")]`):
- Helpers: `paths()`, `strings(&argv)` and `RUNTIME_DIR`. Imports already in place: `HomeBind, HomeSetup`, `HostDirs`, `SandboxSpec`, `bwrap_argv`, `TempDir`, `fs`.
- `a_claude_root_outside_home_is_hidden_and_its_gsd_entries_rebound` (L278) is the model for both new end-to-end tests. Its outside-`$HOME`-and-`/tmp` scratch dir uses `tempfile::Builder::new().prefix(..).tempdir_in(env!("CARGO_TARGET_TMPDIR"))` and returns early with an `eprintln!` when that dir is under /tmp.

Upstream semantics, verified in the installed gsd-core `bin/lib/` sources (not guessed):
- `runtime-homes.cjs` `resolveGsdHome(fallbackHome = os.homedir())` returns `process.env['GSD_HOME'] || fallbackHome`. A blank value counts as unset; any other value is used verbatim (#5017).
- `capability-loader.cjs` L245-246: the global overlay root is `(gsdHome || GSD_HOME || homedir())/.gsd/capabilities`. At L342 the ledger is `<root>/../../.gsd-capabilities.json`, which is `<GSD home>/.gsd-capabilities.json`.
- `capability-command-router.cjs` `capResolveScope`: for the global scope, `runtimeDir = process.env.GSD_HOME || os.homedir()` is "the scope ROOT that holds .gsd/capabilities/<id> and the .gsd-capabilities.json ledger". `capability-ledger.cjs` joins `LEDGER_FILE_NAME = '.gsd-capabilities.json'` to that runtimeDir.
- Conclusion: both entries sit directly in the GSD home: `${GSD_HOME:-$HOME}/.gsd` and `${GSD_HOME:-$HOME}/.gsd-capabilities.json`. The ledger is NOT inside `.gsd`.
</interfaces>
</context>

<decisions>
The user is unavailable. Decisions come from the coordinator brief, the 261113-dck plan and SUMMARY, docs/DRIVER-SANDBOX.md and the upstream sources. All are `[inferred — audit]` unless marked verified.

- **I-1 Reuse the resolver.** Extract the rule now inlined in the private `user_defaults_path_from` into `pub fn resolve_gsd_home_from(gsd_home: Option<&OsStr>, home: Option<&OsStr>) -> Option<PathBuf>` in `src/state_reader/config_json.rs`. `user_defaults_path_from` and `HostDirs::resolve` both call it, so the rule is not duplicated (coordinator: REUSE). GSD_HOME keeps gsd-core's rule: empty means unset, any other value is used as given. It does not take HostDirs' "relative counts as unset" rule. The bind effect is the same either way, because only absolute GSD homes are bound (I-4).
- **I-2 Ledger location (verified, not inferred).** `.gsd-capabilities.json` lives at `${GSD_HOME:-$HOME}/.gsd-capabilities.json`, a sibling of `.gsd`. See the upstream citations in `<interfaces>`.
- **I-3 Union, defaults kept.** `~/.gsd` and `~/.gsd-capabilities.json` stay bound when GSD_HOME is set. This is the same as 261113-dck I-4: no default bind is dropped.
- **I-4 Placement.** Under `$HOME`, a GSD-home entry is bound read-only like any `$HOME` entry and goes through `admissible`. Any other absolute location (`/tmp`, or outside both) gets a read-only re-bind when the entry exists. That is redundant over the read-only `/`, but it survives the `/tmp` tmpfs, the `/run` tmpfs and a Claude-root tmpfs. A GSD home never gets a tmpfs. Unlike a Claude root it holds no Claude credentials, and it may be, or contain, the project, so a tmpfs over it could hide the project. A relative GSD_HOME adds no bind, because it names no fixed host path.
- **I-5 Install root.** A relocated GSD home joins `root_refused`, like the relocated XDG homes and every Claude root: a tool under `$GSD_HOME/bin/` gets only its own file bound.
- **I-6 No credential overlap.** A GSD-home entry inside a credential directory (`HostDirs::credential_dirs()`) is never named, whether it lies under `$HOME` or outside it.
- **I-7 GSD_HOME at the project root.** In that case `<project>/.gsd` becomes read-only inside the sandbox, because WR-05 re-binds allowlisted entries over the writable project bind. This is accepted and documented. The read-only store is the bind's intent, and upstream itself treats a GSD_HOME aliasing the project root as hostile (capability-loader's symlinked-GSD_HOME guard).
- **I-8 Rename.** `HOME_RO_REL` becomes `GSD_RO_REL`, relative to each GSD home, in parallel with `CLAUDE_RO_REL`. The old name and doc ("Entries under `$HOME`") would be wrong, and nothing outside `src/launcher/home.rs` names it.
- **I-9 Dated sync records untouched.** docs/GSD-CORE-SYNC.md (its v1.20.1 note on `.gsd` paths) describes the build at that sync and is not edited. This omission is logged in the musk ledger as M-0048.
</decisions>

<tasks>

<!-- The three literals below are POSITIVE gates (grep -c ... -ge N / = 1): the deliverable must contain them, so the actions name them on purpose. -->
<!-- planner-discipline-allow: gsd_homes() -->
<!-- planner-discipline-allow: gsd-capabilities.json -->
<!-- planner-discipline-allow: HostDirs::resolve -->

<task type="tracer" tdd="true">
  <name>Task 1: End-to-end: GSD_HOME under $HOME reaches the bwrap argv through the shared resolver</name>
  <files>src/state_reader/config_json.rs, src/launcher/host_dirs.rs, src/launcher/home.rs, tests/launcher_sandbox.rs</files>
  <read_first>src/state_reader/config_json.rs (L733-830), src/launcher/host_dirs.rs, src/launcher/home.rs (L1-80, L280-360, L830-1070), tests/launcher_sandbox.rs (L1-60, L278-334)</read_first>
  <behavior>
    - config_json `resolve_gsd_home_from_is_gsd_cores_rule`: (Some("/g"), Some("/h")) gives Some("/g"). (Some(""), Some("/h")) and (None, Some("/h")) give Some("/h"). (Some("rel/dir"), Some("/h")) gives Some("rel/dir"), used verbatim as gsd-core does. (None, None) and (Some(""), None) give None. The three existing `user_defaults_path_from` tests pass unchanged.
    - host_dirs: with no env, `gsd_home == /home/u` and `gsd_homes() == [/home/u]`. GSD_HOME=/g gives `[/home/u, /g]`. GSD_HOME="" gives `[/home/u]`. GSD_HOME=/home/u gives `[/home/u]` (deduplicated). For "rel/dir" and "~/x", `gsd_home` holds the value as given and `gsd_homes() == [/home/u]`. Parity: for each of "", "/g", "rel/dir", "~/x", "/home/u", `Some(d.gsd_home) == resolve_gsd_home_from(Some(v), Some(/home/u))`.
    - home.rs `a_gsd_home_under_home_binds_the_store_of_both_homes`: `relocated_facts(&[("GSD_HOME", "/home/u/gsd")])` yields `ro /home/u/gsd/.gsd`, `ro /home/u/gsd/.gsd-capabilities.json`, `ro /home/u/.gsd` and `ro /home/u/.gsd-capabilities.json`. `the_golden_allowlist_with_overlay` passes with its want list untouched.
    - tests/launcher_sandbox.rs `a_gsd_home_under_home_rebinds_its_store_and_keeps_the_default`: the real argv carries all four `--ro-bind P P` triples, and no `--tmpfs` names the GSD home.
  </behavior>
  <action>
    Red first: write the end-to-end test (step 4) and the home.rs unit test before the implementation, run them, and record the failure in the SUMMARY. The store entries will be missing from the argv or allowlist.

    1. `src/state_reader/config_json.rs` (per I-1). Add `pub fn resolve_gsd_home_from(gsd_home: Option<&std::ffi::OsStr>, home: Option<&std::ffi::OsStr>) -> Option<std::path::PathBuf>`, holding exactly the rule now inside `user_defaults_path_from`: an empty `gsd_home` counts as unset, any other value is used verbatim with no trim and no normalization, the fallback is `home`, and the result is None when neither is usable. Move the gsd-core citation (runtime-homes `resolveGsdHome`, #5017, `process.env.GSD_HOME || os.homedir()`) and the verbatim-value rationale from the docstrings onto the new function. Say that it is the one GSD-home rule, shared by `user_defaults_path` and `launcher::host_dirs::HostDirs::resolve`. Rewrite `user_defaults_path_from` as `resolve_gsd_home_from(..)?` joined with `.gsd` and `defaults.json`. Its signature and visibility and `user_defaults_path` stay as they are, because tests/state_reader_gsd_home.rs depends on them. Add the unit test `resolve_gsd_home_from_is_gsd_cores_rule` with the cases in `<behavior>`, using the existing `os()` helper.

    2. `src/launcher/host_dirs.rs`. Add the field `pub gsd_home: PathBuf`. Its doc: "the GSD home as gsd-core resolves it (`config_json::resolve_gsd_home_from`: non-empty `$GSD_HOME` used as given, else `home`); may be relative, `gsd_homes()` keeps it only when absolute". In `HostDirs::resolve`, set it from `resolve_gsd_home_from(env(GSD_HOME_ENV).as_deref(), Some(home.as_os_str()))`, falling back to `home.to_path_buf()`. Import `GSD_HOME_ENV` and `resolve_gsd_home_from` from `crate::state_reader::config_json`. Do not write the env-var name as a string literal and do not re-implement the empty check here. Add `pub fn gsd_homes(&self) -> Vec<PathBuf>`: `[home, gsd_home]` through `dedup`, with `gsd_home` included only when `is_absolute()`. The default comes first, as in `config_homes()`. Extend the module doc with a short paragraph, "GSD_HOME follows gsd-core, not I-1 `[inferred — audit]`", stating I-1 and I-3 of this plan. Extend `no_env_yields_exactly_the_defaults` with the two gsd assertions. Add `gsd_home_follows_the_shared_resolver_and_keeps_the_default`, covering every host_dirs case in `<behavior>` through `env_of`. Never call std::env::set_var or remove_var.

    3. `src/launcher/home.rs` (per I-8 and I-3). Rename `HOME_RO_REL` to `GSD_RO_REL` and keep the same two entries. New doc: "Entries of each GSD home (`$HOME` and an absolute `$GSD_HOME`, gsd-core's `resolveGsdHome`) re-bound read-only when they exist (D-16): GSD's store and its global capability ledger. Both sit directly in the GSD home; the ledger is not inside `.gsd` (quick 261113-epo)." Update the module-doc bullet on L14 to name `GSD_RO_REL` under every GSD home. In `candidates()`, replace the L315-317 loop with nested loops over `facts.dirs.gsd_homes()` and `GSD_RO_REL`, going through the existing `ro` closure. In this task only `$HOME` paths are admitted; Task 2 adds the branch for other locations. Add the unit test `a_gsd_home_under_home_binds_the_store_of_both_homes` using `relocated_facts`. Do not edit the want list of `the_golden_allowlist_with_overlay`.

    4. `tests/launcher_sandbox.rs`. Add `a_gsd_home_under_home_rebinds_its_store_and_keeps_the_default`, modelled on L278-334. Use a TempDir root with `home = root/home`, `state = root/state` and `store = home/gsd-store`. Create the directories `home/.gsd` and `store/.gsd` and the files `home/.gsd-capabilities.json` and `store/.gsd-capabilities.json` (content "{}"). The env closure returns `store` for "GSD_HOME" only. Then call `HostDirs::resolve(&home, env)`, `HomeSetup::for_host_with(&dirs, Path::new("/nonexistent/exe"), true, &state)` and `SandboxSpec::from_host_with(paths(), &dirs, &setup)`, set `spec.runtime_dir = Some(PathBuf::from(RUNTIME_DIR))`, and build `bwrap_argv(&spec, &[OsString::from("true")])`. Assert that the four `--ro-bind P P` triples are present via `argv.windows(3)`, and that no `--tmpfs` window names `store`.

    Gate baselines, measured at HEAD: config_json lib tests [HEAD=32 EXPECT>=33]; host_dirs lib tests [HEAD=9 EXPECT>=10]; home lib tests [HEAD=24 EXPECT>=25]; `resolve_gsd_home_from` in host_dirs.rs production code [HEAD=0 EXPECT>=1]; `GSD_HOME_ENV` in host_dirs.rs production code [HEAD=0 EXPECT>=1]; `pub fn resolve_gsd_home_from` in config_json.rs [HEAD=0 EXPECT==1]; `pub const GSD_RO_REL` in home.rs [HEAD=0 EXPECT==1]; `gsd_homes()` in home.rs production code [HEAD=0 EXPECT>=1].
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail(){ echo "FAIL-$1"; exit 1; }; mkdir -p target; L=target/epo-t1.log; ran(){ cargo test --lib "$1" > "$L" 2>&1; sed -nE "s/^test result: ok\. ([0-9]+) passed; 0 failed.*/\1/p" "$L"; }; prod(){ sed -E "/^ *#\[cfg\((all\()?test/,\$d" "$1" | grep -v "^\s*//"; }; n=$(ran state_reader::config_json::); [ -n "$n" ] && [ "$n" -ge 33 ] || fail config-json; n=$(ran launcher::host_dirs::); [ -n "$n" ] && [ "$n" -ge 10 ] || fail host-dirs; n=$(ran launcher::home::); [ -n "$n" ] && [ "$n" -ge 25 ] || fail home; cargo test --test launcher_sandbox --test state_reader_gsd_home > "$L" 2>&1 || fail integration; for t in a_gsd_home_under_home_rebinds_its_store_and_keeps_the_default a_claude_root_outside_home_is_hidden_and_its_gsd_entries_rebound; do grep -q "^test $t \.\.\. ok$" "$L" || fail "$t"; done; [ "$(prod src/launcher/host_dirs.rs | grep -c resolve_gsd_home_from)" -ge 1 ] || fail reuse-resolver; [ "$(prod src/launcher/host_dirs.rs | grep -c GSD_HOME_ENV)" -ge 1 ] || fail reuse-env-name; [ "$(prod src/state_reader/config_json.rs | grep -c "pub fn resolve_gsd_home_from")" = 1 ] || fail resolver-pub; [ "$(prod src/launcher/home.rs | grep -c "pub const GSD_RO_REL")" = 1 ] || fail rename; [ "$(prod src/launcher/home.rs | grep -c "gsd_homes()")" -ge 1 ] || fail home-loop; echo verify-ok'</automated>
  </verify>
  <done>With GSD_HOME pointing at a directory under $HOME, the real bwrap argv re-binds `$GSD_HOME/.gsd` and `$GSD_HOME/.gsd-capabilities.json` read-only next to the `~/` defaults. The GSD home comes from the one shared resolver, the golden allowlist is unchanged, the failing-first run is recorded, and the verify command prints `verify-ok`.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Outside-$HOME placement, install-root refusal and the no-overlap invariant for the GSD home</name>
  <files>src/launcher/home.rs, tests/launcher_sandbox.rs</files>
  <read_first>src/launcher/home.rs (L1-45, L183-260, L280-392, L1004-1170), tests/launcher_sandbox.rs (L278-334)</read_first>
  <behavior>
    - `a_gsd_home_outside_home_is_rebound_without_a_tmpfs`: for each GSD_HOME in "/srv/gsd" and "/tmp/gsd", the allowlist from `relocated_facts` contains `ro <root>/.gsd` and `ro <root>/.gsd-capabilities.json`, no entry starts with `tmpfs <root>`, and `ro /home/u/.gsd` is still present. After `<root>/.gsd` is removed from `existing`, that entry is not named.
    - `a_relative_or_degenerate_gsd_home_never_gets_a_tmpfs`: for "", "rel/dir" and "~/x", `home_allowlist` equals the no-env `home_allowlist(&facts(), true)`. For "/", "/home" and "/home/u/.cargo/bin", every `HomeBind::Tmpfs` lies under /home/u and is not in `f.dirs.gsd_homes()`.
    - `a_relocated_gsd_home_is_never_an_install_root`: with GSD_HOME=/home/u/gsd and a tool at /home/u/gsd/bin/x, `install_root(..) == None`, the allowlist has `ro /home/u/gsd/bin/x`, and it does not have `ro /home/u/gsd`.
    - `no_allowlist_entry_overlaps_a_mask_dir` also holds for `[("XDG_CONFIG_HOME","/home/u/cfg"),("GSD_HOME","/home/u/cfg/gh")]` and `[("XDG_CONFIG_HOME","/x/cfg"),("GSD_HOME","/x/cfg/gh")]`.
    - tests/launcher_sandbox.rs `a_gsd_home_outside_home_and_tmp_is_rebound_read_only_without_a_tmpfs`: in the real argv, a store outside $HOME and /tmp gets the two `--ro-bind P P` triples and no `--tmpfs` naming it, and the default `<home>/.gsd` triple is still present.
  </behavior>
  <action>
    Write the four home.rs tests and the end-to-end test first, and run them red; the outside entries are not yet named. Then implement:

    1. `candidates()` (I-4, I-6). For each GSD-home entry from Task 1's loop, a path under `facts.home` keeps going through the `ro` closure. Any other path is pushed as `HomeBind::Ro(path)` only when `!touches_mask_dir(&path, facts)`. `home_allowlist` keeps naming it only when it exists, through the existing `facts.existing` filter, which is unchanged. Never push a `HomeBind::Tmpfs` for a GSD home. `gsd_homes()` already drops a relative value.
    2. `root_refused()` (I-5). Add `dirs.gsd_homes().iter().any(|g| root == g)` next to the `claude_roots()` clause. `$HOME` itself is already refused by `strictly_under`, so this adds only a relocated GSD home. Extend its docstring.
    3. Docs in `src/launcher/home.rs`. In the `home_allowlist` docstring, add GSD-home entries to the exception sentence: outside `$HOME` they are re-bound read-only when they exist, never under a tmpfs. In the module doc's "Relocated host directories" section, add one or two sentences covering `GSD_HOME` per I-3 to I-7: the union with `~/.gsd`; bound like any `$HOME` entry under `$HOME`; re-bound read-only elsewhere so the `/tmp`, `/run` or a Claude-root tmpfs cannot hide it; no tmpfs over the GSD home; a relative value adds nothing; never a whole install root.
    4. Tests in home.rs `mod tests`: add the three new tests from `<behavior>`, modelled on `a_claude_root_outside_home_is_an_empty_tmpfs_with_its_gsd_entries_rebound`, `a_degenerate_claude_root_never_gets_a_tmpfs` and `a_relocated_claude_root_is_never_an_install_root`. Add the two relocated variants to the facts list of `no_allowlist_entry_overlaps_a_mask_dir`. All of them inject env through `relocated_facts`.
    5. `tests/launcher_sandbox.rs`: add `a_gsd_home_outside_home_and_tmp_is_rebound_read_only_without_a_tmpfs`, using the `CARGO_TARGET_TMPDIR` scratch pattern from L281-290 with prefix "epo-gsd-home-" and the same early return under /tmp. Create `store = outside/store` holding a `.gsd` dir and a `.gsd-capabilities.json` file. The home is under a TempDir and holds `.gsd`. The env closure returns `store` for "GSD_HOME" only. Build the argv as in Task 1 and assert the behavior above.

    Gate baselines: home lib tests [HEAD=24 EXPECT>=28]; launcher_sandbox new test names [HEAD=absent EXPECT=ok]; `gsd_homes()` in home.rs production code [HEAD=0 EXPECT>=2]; set_var or remove_var in non-comment lines of host_dirs.rs, home.rs and tests/launcher_sandbox.rs [HEAD=0 EXPECT==0, invariant guard].
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail(){ echo "FAIL-$1"; exit 1; }; mkdir -p target; L=target/epo-t2.log; ran(){ cargo test --lib "$1" > "$L" 2>&1; sed -nE "s/^test result: ok\. ([0-9]+) passed; 0 failed.*/\1/p" "$L"; }; prod(){ sed -E "/^ *#\[cfg\((all\()?test/,\$d" "$1" | grep -v "^\s*//"; }; n=$(ran launcher::home::); [ -n "$n" ] && [ "$n" -ge 28 ] || fail home; n=$(ran launcher::host_dirs::); [ -n "$n" ] && [ "$n" -ge 10 ] || fail host-dirs; cargo test --test launcher_sandbox > "$L" 2>&1 || fail sandbox-int; for t in a_gsd_home_under_home_rebinds_its_store_and_keeps_the_default a_gsd_home_outside_home_and_tmp_is_rebound_read_only_without_a_tmpfs a_claude_root_outside_home_is_hidden_and_its_gsd_entries_rebound; do grep -q "^test $t \.\.\. ok$" "$L" || fail "$t"; done; [ "$(prod src/launcher/home.rs | grep -c "gsd_homes()")" -ge 2 ] || fail refuse-and-bind; [ "$(cat src/launcher/host_dirs.rs src/launcher/home.rs tests/launcher_sandbox.rs | grep -v "^\s*//" | grep -cE "set_var|remove_var")" = 0 ] || fail env-mutation; G="$PWD/target/epo-gsd-home"; mkdir -p "$G/.gsd"; GSD_HOME="$G" cargo test --lib launcher:: > "$L" 2>&1 || fail hermetic-lib; GSD_HOME="$G" cargo test --test launcher_sandbox > "$L" 2>&1 || fail hermetic-int; echo verify-ok'</automated>
  </verify>
  <done>An absolute GSD home outside $HOME (under /tmp or elsewhere) has its `.gsd` and `.gsd-capabilities.json` re-bound read-only and never gets a tmpfs. A relative value changes nothing, no GSD-home entry overlaps a credential directory, and a relocated GSD home is never a whole install root. The launcher suites pass with GSD_HOME set in the process environment, and the verify command prints `verify-ok`.</done>
</task>

<task type="auto">
  <name>Task 3: Correct the sandbox and configuration docs, then run the full gates</name>
  <files>docs/DRIVER-SANDBOX.md, docs/CONFIGURATION.md</files>
  <read_first>docs/DRIVER-SANDBOX.md (L40-84), docs/CONFIGURATION.md (L112-124)</read_first>
  <action>
    1. `docs/DRIVER-SANDBOX.md`, section "Relocated host directories". Make these edits:
       - After the intro paragraph that lists `CLAUDE_CONFIG_DIR` and the `XDG_*` variables, add a paragraph saying that `GSD_HOME` follows gsd-core's own rule instead (I-1). Unset or empty means `$HOME`; any other value is used as given, by the same resolver the Config tab uses for `$GSD_HOME/.gsd/defaults.json`. GSD keeps its store at `$GSD_HOME/.gsd` and its global capability ledger at `$GSD_HOME/.gsd-capabilities.json` (I-2). The driver inherits `GSD_HOME`, and both entries are bound read-only next to `~/.gsd` and `~/.gsd-capabilities.json` (I-3). A relative value adds no bind.
       - In the union bullet list, add a bullet: `.gsd` and `.gsd-capabilities.json` of both `$HOME` and `$GSD_HOME` are bound read-only. Extend the "never bound as a whole" bullet with "a relocated GSD home" (I-5).
       - In the placement table, add two rows for "`.gsd` and `.gsd-capabilities.json` of `$GSD_HOME`". Under `$HOME`: bound read-only like any `$HOME` entry. Elsewhere: re-bound read-only when they exist, so the `/tmp` tmpfs or a Claude-root tmpfs cannot hide them; `GSD_HOME` itself never gets a tmpfs, because it may hold the project; when `GSD_HOME` is the project root, its `.gsd` is read-only inside the sandbox too (I-4, I-7).
       - Delete the sentence right after the table (L83) claiming that gsd-meta-manager's own GSD_HOME-relative binds stay put. The new rows supersede it. Keep L82 ("Under `$HOME` nothing needs masking ...").
    2. `docs/CONFIGURATION.md`, the `GSD_HOME` row of the environment table. Set the Consumer cell to `src/state_reader/config_json.rs` (`resolve_gsd_home_from`, `user_defaults_path`) and `src/launcher/host_dirs.rs` (`HostDirs::resolve`). Append to Purpose: "The driver and one-off sandboxes bind `$GSD_HOME/.gsd` and `$GSD_HOME/.gsd-capabilities.json` read-only next to the `~/` defaults when `GSD_HOME` is absolute (see `docs/DRIVER-SANDBOX.md`, "Relocated host directories")." Keep the existing text and the `\|` escaping in the cell.
    3. Do not edit docs/GSD-CORE-SYNC.md (I-9).
    4. Run the full gates (verify). If a cross-target clippy fails on a dead-code lint for a target-specific path, fix it in the code from Tasks 1 and 2. This is the class of failure that killed v2.2.0 and v2.3.0. The full `cargo test` run takes minutes; run the verify in the background or with a long timeout.

    Gate baselines: the stale sentence's text in docs/DRIVER-SANDBOX.md, with HTML-comment lines filtered [HEAD=1 EXPECT==0]; `GSD_HOME` in docs/DRIVER-SANDBOX.md [HEAD=1 EXPECT>=3]; `gsd-capabilities.json` in docs/DRIVER-SANDBOX.md [HEAD=1 EXPECT>=2]; the `GSD_HOME` table row of docs/CONFIGURATION.md naming `HostDirs::resolve` [HEAD=0 EXPECT==1]; `gsd-capabilities.json` in docs/CONFIGURATION.md [HEAD=0 EXPECT>=1].
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail(){ echo "FAIL-$1"; exit 1; }; mkdir -p target; L=target/epo-t3.log; D=docs/DRIVER-SANDBOX.md; C=docs/CONFIGURATION.md; [ "$(grep -v "^\s*<!--" "$D" | grep -c "binds (..gsd., ..gsd-capabilities.json.) are not relocated")" = 0 ] || fail stale-sentence; [ "$(grep -c GSD_HOME "$D")" -ge 3 ] || fail doc-gsd-home; [ "$(grep -c "gsd-capabilities.json" "$D")" -ge 2 ] || fail doc-ledger; [ "$(grep "^| .GSD_HOME. |" "$C" | grep -c "HostDirs::resolve")" = 1 ] || fail cfg-row; [ "$(grep -c "gsd-capabilities.json" "$C")" -ge 1 ] || fail cfg-ledger; cargo fmt --all -- --check > "$L" 2>&1 || fail fmt; cargo clippy --all-targets --locked -- -D warnings > "$L" 2>&1 || fail clippy; cargo test --no-fail-fast > "$L" 2>&1 || fail tests; if grep -q "^test result: FAILED" "$L"; then fail tests-failed; fi; for T in aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-gnu; do if rustup target list --installed | grep -qx "$T"; then cargo clippy --locked --target "$T" --all-targets -- -D warnings > "$L" 2>&1 || fail "clippy-$T"; else echo "SKIP $T (not installed)"; fi; done; echo verify-ok'</automated>
  </verify>
  <done>docs/DRIVER-SANDBOX.md documents the GSD_HOME rule, the union bullet and the two placement rows, and no longer says that these binds ignore GSD_HOME. The GSD_HOME row in docs/CONFIGURATION.md names the launcher consumer. The full test suite, host clippy, fmt and cross-target clippy all pass, and the verify command prints `verify-ok`.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| host environment -> launcher | `GSD_HOME` is operator-controlled and decides host paths that get bound into the sandbox |
| sandbox -> host filesystem | the driver reads the bound GSD store and ledger; any bind it should not have is readable, or writable if not read-only |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-261113-epo-01 | Tampering | `$GSD_HOME/.gsd` and `.gsd-capabilities.json` inside the sandbox (capability overlays, consent store, ledger) | medium | mitigate | Every GSD-home entry is a `HomeBind::Ro`, never Overlay or a writable bind. Inside the project, WR-05 re-establishes it read-only over the project bind. Pinned by the two end-to-end argv tests (`--ro-bind` triples). |
| T-261113-epo-02 | Information disclosure | a tool installed under a relocated GSD home would bind the whole GSD home | medium | mitigate | `root_refused` refuses every `gsd_homes()` entry (I-5). Pinned by `a_relocated_gsd_home_is_never_an_install_root`. |
| T-261113-epo-03 | Information disclosure | GSD_HOME pointed inside a credential directory (gh, gcloud, glab-cli, keyrings, git) | medium | mitigate | Entries under `$HOME` go through `admissible`, and other entries through `!touches_mask_dir`, so none is named (I-6). Outside `$HOME` the dir masks also follow the home entries in the argv. Pinned by the two variants added to `no_allowlist_entry_overlaps_a_mask_dir`. |
| T-261113-epo-04 | Denial of service | a degenerate GSD_HOME (relative, `/`, an ancestor of `$HOME`, the project) hiding the sandbox's footing or the project | medium | mitigate | A GSD home never gets a tmpfs, and a relative value adds no bind (I-4). Pinned by `a_relative_or_degenerate_gsd_home_never_gets_a_tmpfs`. |
| T-261113-epo-05 | Information disclosure | `$GSD_HOME/.gsd/*_api_key` (search-provider keys) readable inside the sandbox | low | accept | Unchanged from the existing `~/.gsd` bind: gsd-core inside the run needs them for its search providers, and egress stays on the allowlisted proxy. The bind only follows the store to where GSD actually keeps it. |
| T-261113-epo-06 | Denial of service | GSD_HOME equal to the project root makes `<project>/.gsd` read-only inside the sandbox | low | accept | Consistent with WR-05 and the bind's read-only intent. Upstream treats a GSD_HOME aliasing the project root as hostile. Documented in docs/DRIVER-SANDBOX.md (I-7). |
| T-261113-epo-SC | Tampering | npm/pip/cargo installs | low | accept | No dependency is added. Every gate runs `--locked`. |
</threat_model>

<verification>
- Task verifies 1-3 each print `verify-ok`.
- `the_golden_allowlist_with_overlay` passes with its want list unchanged (GSD_HOME unset gives the same allowlist).
- No test in the touched files writes the process environment; the launcher suites pass with GSD_HOME set to an existing scratch store.
- `git status` is clean after the executor's path-scoped commits; `target/epo-*` scratch stays under the ignored `target/`.
</verification>

<success_criteria>
- `.gsd` and `.gsd-capabilities.json` are bound at `${GSD_HOME:-$HOME}` as well as the `$HOME` defaults, with placement per I-4. The GSD home is decided only by `resolve_gsd_home_from`.
- New tests: 1 in config_json, 1 in host_dirs (plus one extended), 4 in home.rs (plus one extended) and 2 end-to-end in tests/launcher_sandbox.rs, all passing.
- `cargo test --no-fail-fast`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check` and the installed cross-target clippy runs all exit 0.
- docs/DRIVER-SANDBOX.md and docs/CONFIGURATION.md describe the new behavior, and the stale sentence is gone.
</success_criteria>

<output>
Create `.planning/quick/261113-epo-sandbox-binds-of-gsd-and-gsd-capabilitie/261113-epo-SUMMARY.md` when done
</output>
