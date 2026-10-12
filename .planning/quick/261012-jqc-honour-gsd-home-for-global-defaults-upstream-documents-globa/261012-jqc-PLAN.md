---
quick_id: 261012-jqc
mode: quick-batch
phase: quick-261012-jqc
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/config_json.rs
  - tests/state_reader_gsd_home.rs
  - src/ui/screens/help.rs
  - src/ui/screens/preferences.rs
  - src/ui/screens/detail.rs
  - README.md
  - docs/CONFIGURATION.md
autonomous: true
requirements: [QUICK-261012-jqc]

estimate:
  tokens: 90000
  raw_tokens: 90000
  tasks: 2
  confidence: low

must_haves:
  truths:
    - "With GSD_HOME set to a non-empty value, `user_defaults_path()` returns `$GSD_HOME/.gsd/defaults.json`, so the Config tab's global scope loads and writes that file and not `$HOME/.gsd/defaults.json` (upstream gsd-core fd1bfd09e, rule `GSD_HOME || homedir()`) [ID-1]"
    - "With GSD_HOME unset or set to the empty string, resolution falls back to `$HOME/.gsd/defaults.json` exactly as before. With neither usable, it returns None and the defaults file is not loaded [ID-1, ID-2]"
    - "No lib unit test mutates the process environment. The only env mutation is in `tests/state_reader_gsd_home.rs`, a test binary with exactly one #[test] [ID-3]"
    - "The help row, the preferences screen header, the Config tab's global title, its empty-state line and its scope-switch status message, the README keys table and docs/CONFIGURATION.md all name the global defaults file `$GSD_HOME/.gsd/defaults.json`. Each drawn label is a static literal, and the resolved path is never drawn (I-12, quick 260927-t3s) [ID-4, ID-5]"
    - "README.md lines 15-20 (GSD compatibility note) are byte-identical to HEAD 7303b41. The sibling-owned regions of detail.rs and config_json.rs are untouched [ID-6]"
    - "`cargo build` and `cargo clippy -- -D warnings` exit 0, `cargo clippy --all-targets` reports no warning located in a file this plan touches, and `rtk proxy cargo test --no-fail-fast` fails only the known git-version witness in src/envelope/policy.rs"
  artifacts:
    - path: src/state_reader/config_json.rs
      provides: "pub const GSD_HOME_ENV; private pure resolver user_defaults_path_from(gsd_home, home); pub fn user_defaults_path() as the single env-reading wrapper; three pure unit tests placed directly after load_user_defaults_from_reads_a_given_path"
    - path: tests/state_reader_gsd_home.rs
      provides: "one #[test] that sets GSD_HOME/HOME to tempdir stores holding contradicting defaults.json files and proves the production chain user_defaults_path() -> load_user_defaults_from() through set, empty and unset GSD_HOME"
  key_links:
    - from: "detail.rs Config-tab arrival (~3224) and reload (~5027)"
      to: "config_json::user_defaults_path"
      via: "cache.defaults_user_path = user_defaults_path(); the same cached path feeds load_user_defaults_from and config_write_path(Global), so the file read is the file written (I-7)"
    - from: "config_json::user_defaults_path"
      to: "config_json::user_defaults_path_from"
      via: "user_defaults_path_from(var_os(GSD_HOME_ENV).as_deref(), var_os(\"HOME\").as_deref()) with GSD_HOME first, and an empty GSD_HOME treated as unset"
---

<objective>
Honour `GSD_HOME` when resolving GSD's global defaults file. gsd-core now documents the global defaults store as `$GSD_HOME/.gsd/defaults.json` (commit fd1bfd09e, shipped in v1.16.0). Its config loader already resolved it as `process.env.GSD_HOME || os.homedir()` at both sites (src/config-loader.cts 1013 and 1076). The meta-manager still reads `$HOME/.gsd/defaults.json` unconditionally. A user with `GSD_HOME` set therefore sees, edits and writes a different template than the one GSD seeds new projects from.

Purpose: the Config tab's global scope shows and edits the file gsd-core actually uses.
Output: a GSD_HOME-aware `user_defaults_path` with race-free tests, plus labels and docs that name the store the way upstream does.

Inferred decisions. The human was unavailable during planning. The executor MUST copy this list into SUMMARY.md under a heading marked "inferred — for later audit":
- **ID-1 Upstream parity on emptiness.** Mirror `GSD_HOME || homedir()`. An unset GSD_HOME and an empty-string GSD_HOME both fall back to HOME. Any non-empty value is used verbatim, with no trimming and no relative-path normalization, because JS `||` treats `" "` as truthy and upstream joins whatever it gets. Record this in the resolver's doc comment.
- **ID-2 HOME fallback unchanged.** The fallback stays `std::env::var_os("HOME")`, exactly as the item specifies ("falling back to $HOME"). Upstream's `os.homedir()` also consults the passwd entry when HOME is unset. That is a pre-existing divergence and is out of this item's scope, so None stays None.
- **ID-3 Race-free env tests.** Resolution semantics are tested through a private pure function that takes the two values as arguments. This follows the crate's convention in src/experimental.rs: "`std::env::set_var` is unsound under the parallel test harness". The env wiring itself is proven in a dedicated integration-test binary holding exactly ONE #[test], following the tests/driver_*.rs precedent of mutating the environment only inside its own process. Lib unit tests never call set_var or remove_var.
- **ID-4 Label wording.** Every label uses upstream's documented static literal `$GSD_HOME/.gsd/defaults.json`. Where a surface has room to explain it (help row, preferences header, README, docs), add that GSD_HOME defaults to the home directory. Labels stay static literals per I-12 (quick 260927-t3s, T-t3s-05): the resolved path contains the username and is never drawn.
- **ID-5 detail.rs labels included.** The item's list names help.rs and preferences.rs but not detail.rs. The Config tab's own three user-visible labels are still in scope: without them, the tab would contradict the help screen once GSD_HOME is honoured. Edits are confined to those exact string literals and the tests that pin them. They are hundreds of lines away from sibling 261012-jqb's gates-help region. There is no depends_on, because there is no semantic dependency. config_json.rs already overlaps with the sibling, so the batch's file partition behaves the same either way.
- **ID-6 Out of scope (guardrails, do not edit):** the `*_api_key` help strings in detail.rs (~11440-11461), which sit in the config-key help region that sibling 261012-jqb refreshes; code comments that use the tilde shorthand (detail.rs, src/ui/screens/mod.rs ~1061/~1066, src/executor/runtime.rs ~107, config_json.rs ~84), which are not user-visible; README.md lines 15-20, which a later task owns; anything near `GatesConfig` (config_json.rs ~244).
- **ID-7 CONFIGURATION.md made self-consistent.** Besides the env table (~107), the three other defaults-path mentions in the same file (~198, ~225, ~235) are updated. They become inaccurate under GSD_HOME, and the file is already in scope.
- **ID-8 Env-name constant.** `pub const GSD_HOME_ENV: &str = "GSD_HOME";` follows the crate precedent (`EXPERIMENTAL_FEATURES_ENV`, `ENVELOPE_ROOT_ENV`), so production code and the integration test reference one literal.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@src/state_reader/config_json.rs
@src/experimental.rs

Code facts the planner verified at HEAD 7303b41. Cite these and do not re-derive them:
- `src/state_reader/config_json.rs`: `user_defaults_path()` is at 648-657. It reads only `std::env::var_os("HOME")` and pushes `.gsd` then `defaults.json`. `load_user_defaults()` (659-665) has no production caller. `load_user_defaults_from(path)` is at 667-674. `#[cfg(test)] mod tests` opens at 676, and its first test is `load_user_defaults_from_reads_a_given_path` (680-688). Sibling 261012-jqb edits `GatesConfig` (~244) and adds an `AuditConfig`. Stay away from both, and do not append tests at the END of the tests module, because that is the sibling's likely insertion point.
- Production consumers of `user_defaults_path()` exist only in `src/ui/screens/detail.rs`: arrival ~3224, reload ~5027, and the read-only test `arrival_resolves_the_global_defaults_path` ~22096-22104, which compares against `user_defaults_path()` and stays valid. No signature change is needed.
- `src/experimental.rs` 1-100 is the crate's pattern for env parsing: a pure function plus a one-line env wrapper, with tests that only drive the pure function.
- Integration tests already mutate env once per binary (tests/driver_inbox.rs ~108-115, driver_optin.rs ~224-231). The crate is edition 2021, so `std::env::set_var` is a safe fn here. `tempfile = "3"` is already a dependency.
- Upstream (`~/projects/node/gsd-core`, fd1bfd09e): `const gsdHome = process.env['GSD_HOME'] || os.homedir();` then `path.join(gsdHome, '.gsd', 'defaults.json')`. Upstream's regression tests point GSD_HOME at one store and HOME at a decoy store with contradicting values. Mirror that design.
- Labels at HEAD:
  - help.rs 297 is the production row: `row("g", "Config tab: switch between project and global (~/.gsd/defaults.json) settings; d is an alias (detail view)")`. Test `the_g_config_scope_switch_is_documented` (~1157-1175) pins the same description string at ~1161, and the two must stay byte-identical.
  - preferences.rs 9 (module doc) reads ``* **GSD global template** (`~/.gsd/defaults.json`): reached with `g` from``. 192 (header) reads `" GSD GLOBAL TEMPLATE (~/.gsd/defaults.json)"`. Test `render_labels_both_scopes` (294-310) renders at 100x14.
  - detail.rs 7942 (Global empty state): `"  No global template loaded (~/.gsd/defaults.json missing — saving will create it)"`. 8080 (Global block title): `" Global: template for NEW projects (~/.gsd/defaults.json) "`, pinned by the test at 22017. 12387 (`config_scope_status`, whose doc says "Static text only … (I-12)"): `"Editing global template for NEW projects (~/.gsd/defaults.json); existing projects ignore it"`, pinned verbatim at 22367, 22431 and 27374. 22992 asserts only the substring `Global: template for NEW projects`, which still holds.
  - README.md 275: `| Config | \`g\` (or \`d\`) | Switch between the project's config and \`~/.gsd/defaults.json\` |`. A test in detail.rs (~16002-16040) reads README.md but checks only the compatibility paragraph (lines 15-20), which this plan must not touch.
  - docs/CONFIGURATION.md 107 is the `HOME` row of the environment-variable table (`user_defaults_path` consumer). Lines 198, 225 and 235 mention the defaults path in prose.
- Release gate 5 is `cargo clippy -- -D warnings` (scripts/pre-tag-check.sh 612; .github/workflows/release.yml 100).
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: GSD_HOME resolves the global defaults file end to end (env -> user_defaults_path -> load_user_defaults_from)</name>
  <files>src/state_reader/config_json.rs, tests/state_reader_gsd_home.rs</files>
  <behavior>
    - Integration (tests/state_reader_gsd_home.rs, ONE #[test]): GSD_HOME points at store G, whose defaults.json has mode "yolo". HOME points at decoy store D, whose defaults.json has mode "interactive". `user_defaults_path()` must equal `G/.gsd/defaults.json`, and `load_user_defaults_from` on it must yield mode "yolo".
    - Same test, next step: with GSD_HOME set to "", the path must equal `D/.gsd/defaults.json` and the loaded mode must be "interactive" (upstream's `||` fallback).
    - Same test, next step: with GSD_HOME removed, the path must equal `D/.gsd/defaults.json` and the loaded mode must be "interactive".
    - Unit (pure, no env): from(Some("/g"), Some("/h")) and from(Some("/g"), None) both give `/g/.gsd/defaults.json`.
    - Unit (pure, no env): from(None, Some("/h")) and from(Some(""), Some("/h")) both give `/h/.gsd/defaults.json`.
    - Unit (pure, no env): from(None, None) and from(Some(""), None) both give None.
  </behavior>
  <action>
    Per ID-8, add `pub const GSD_HOME_ENV: &str = "GSD_HOME";` directly above `user_defaults_path` in src/state_reader/config_json.rs, with a one-line doc saying production code and tests reference this one literal.

    RED: create tests/state_reader_gsd_home.rs with the single #[test] described in `<behavior>`. Build two `tempfile::TempDir` stores, each with `.gsd/defaults.json` written via `create_dir_all` + `std::fs::write`. Set the env with `std::env::set_var` / `std::env::remove_var`, using `gsd_meta_manager::state_reader::config_json::GSD_HOME_ENV` for the GSD_HOME name and the literal "HOME" for home. Drive the PRODUCTION chain the Config tab uses: call `user_defaults_path()`, assert the exact PathBuf, then pass that path to `load_user_defaults_from` and assert `.mode`. Do NOT use the caller-less `load_user_defaults()`. Give the file a module doc comment saying it must hold exactly ONE #[test] because the test mutates process-global env, and that further GSD_HOME cases go inside that one function, never into a second test. Cite the tests/driver_*.rs once-per-binary precedent. Run it. Case 1 must FAIL (it resolves the decoy and loads "interactive"), and that failure is the red. Commit the red state as `test(261012-jqc): …`.

    GREEN (per ID-1, ID-2, ID-3): split the resolver as src/experimental.rs does.
    - Add a private `fn user_defaults_path_from(gsd_home: Option<&std::ffi::OsStr>, home: Option<&std::ffi::OsStr>) -> Option<std::path::PathBuf>`. It filters an empty `gsd_home` to None, takes the first of (filtered gsd_home, home), and pushes `.gsd` then `defaults.json`. Do not trim or normalize the value (ID-1). The HOME branch keeps the exact current semantics (ID-2).
    - Rewrite `user_defaults_path()` as the single env-reading wrapper: `user_defaults_path_from(std::env::var_os(GSD_HOME_ENV).as_deref(), std::env::var_os("HOME").as_deref())`.
    - Update both doc comments (resolver and `load_user_defaults`) to name `$GSD_HOME/.gsd/defaults.json`, the empty-falls-back rule, the HOME fallback, the None case, and upstream's rule (`GSD_HOME || homedir()`, gsd-core config loader; fd1bfd09e / v1.16.0).
    - Add the three pure unit tests from `<behavior>` to the existing `mod tests`, IMMEDIATELY AFTER `load_user_defaults_from_reads_a_given_path`. Not at the module end (sibling 261012-jqb), and not near GatesConfig. Name them for what they prove, e.g. `gsd_home_wins_over_home_for_the_global_defaults_path`, `an_unset_or_empty_gsd_home_falls_back_to_home`, `no_usable_root_means_no_global_defaults_path`. These tests call only `user_defaults_path_from`. No `set_var` and no `remove_var` may appear in config_json.rs.
    - Keep `user_defaults_path`'s public signature, so the detail.rs call sites need no change.
    - Commit as `feat(261012-jqc): …`.
  </action>
  <verify>
    <automated>rtk proxy cargo test --test state_reader_gsd_home</automated>
    <automated>rtk proxy cargo test --lib state_reader::config_json</automated>
    <automated>rtk proxy sh -c "grep -v '^[[:space:]]*//' src/state_reader/config_json.rs | grep -c 'env::set_var\|env::remove_var'"  (prints 0: comment lines are excluded so a doc comment citing the crate's set_var rule cannot trip the gate)</automated>
    <automated>rtk proxy sh -c "grep -v '^[[:space:]]*//' tests/state_reader_gsd_home.rs | grep -c '#\[test\]'"  (prints 1)</automated>
  </verify>
  <done>The integration test failed before GREEN and passes after it. The three pure unit tests pass. config_json.rs has no env mutation. The integration binary holds exactly one #[test]. `user_defaults_path()` returns `$GSD_HOME/.gsd/defaults.json` when GSD_HOME is non-empty, and `$HOME/.gsd/defaults.json` otherwise.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Labels and docs name the store as $GSD_HOME/.gsd/defaults.json</name>
  <files>src/ui/screens/help.rs, src/ui/screens/preferences.rs, src/ui/screens/detail.rs, README.md, docs/CONFIGURATION.md</files>
  <behavior>
    - help: `the_g_config_scope_switch_is_documented` pins the new row text and finds it exactly once, with the experimental flag both on and off.
    - preferences: `render_labels_both_scopes` additionally asserts the rendered buffer contains `$GSD_HOME/.gsd/defaults.json`.
    - detail: the pinned title test (~22017) and the three `config_scope_status` assertions (~22367, ~22431, ~27374) pin the new literals.
  </behavior>
  <action>
    All drawn labels stay static `&'static str` / literal text per I-12 and ID-4. Never format the resolved path into a cell.

    help.rs (per ID-4): change the `g` row description at 297 AND the identical pinned string in the test at ~1161 to exactly
    `Config tab: switch between project and global ($GSD_HOME/.gsd/defaults.json; GSD_HOME defaults to ~) settings; d is an alias (detail view)`.
    The two strings must stay byte-identical. Leave the surrounding comments alone.

    preferences.rs (per ID-4):
    - Module doc line 9: name the template as `$GSD_HOME/.gsd/defaults.json` and add that `GSD_HOME` defaults to the home directory.
    - Header at 192: change it to exactly `" GSD GLOBAL TEMPLATE ($GSD_HOME/.gsd/defaults.json; GSD_HOME defaults to ~)"`. It still fits the 98-column inner width of the 100x14 test render.
    - In `render_labels_both_scopes`, add one `assert!(text.contains("$GSD_HOME/.gsd/defaults.json"))`.
    - Do not add or remove rendered lines, because the 14-row test layout depends on the line count.

    detail.rs (per ID-5, ID-6). Edit ONLY these literals; use Edit with replace_all where one literal recurs:
    - The 7942 empty-state string becomes `"  No global template loaded ($GSD_HOME/.gsd/defaults.json missing — saving will create it)"`.
    - The 8080 block title and its pin at 22017 become `" Global: template for NEW projects ($GSD_HOME/.gsd/defaults.json) "`.
    - The 12387 `config_scope_status` Global string, and its three verbatim pins at 22367, 22431 and 27374, become `"Editing global template for NEW projects ($GSD_HOME/.gsd/defaults.json); existing projects ignore it"`.
    - Touch nothing else in detail.rs. Specifically not the `*_api_key` help strings (~11440-11461), the gates help/option code (~11834-13841, sibling 261012-jqb), or any code comment.

    README.md (per ID-4, ID-6): change ONLY the Config `g` row at 275 to read: Switch between the project's config and `$GSD_HOME/.gsd/defaults.json` (`GSD_HOME` defaults to your home directory). Lines 15-20 must stay byte-identical.

    docs/CONFIGURATION.md (per ID-1, ID-2, ID-7):
    - In the environment-variable table, add a `GSD_HOME` row directly above the `HOME` row (107). Consumer: `src/state_reader/config_json.rs` (`user_defaults_path`). Purpose: root of the GSD-owned store. GSD user defaults are read from, and the Config tab's global scope writes to, `$GSD_HOME/.gsd/defaults.json`. Unset or empty falls back to `$HOME`, matching gsd-core's own `GSD_HOME || homedir()` rule. Any other value is used as given.
    - Rewrite the `HOME` row as the fallback root used when `GSD_HOME` is unset or empty. If neither is set, the defaults file is not loaded.
    - At ~198, ~225 and ~235, name the file `$GSD_HOME/.gsd/defaults.json`. Keep every other word of those sentences.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib ui::screens</automated>
    <automated>rtk proxy grep -c "GSD_HOME/.gsd/defaults.json" src/ui/screens/help.rs src/ui/screens/preferences.rs src/ui/screens/detail.rs README.md  (expect help.rs 2, preferences.rs 3, detail.rs 7, README.md 1)</automated>
    <automated>git diff --quiet 7303b41 -- README.md || git diff -U0 7303b41 -- README.md  (the only hunk header must be at line 275)</automated>
  </verify>
  <done>All five files name the store `$GSD_HOME/.gsd/defaults.json` at the listed sites. The pinning tests pass with the new literals. README.md's only hunk is line 275. detail.rs's diff touches exactly the seven listed lines. CONFIGURATION.md documents GSD_HOME and the HOME fallback, including the empty-string rule.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| process environment -> resolver | `GSD_HOME` and `HOME` come from the user's own environment, with the same trust as HOME today. They choose the file the Config tab reads and writes. |
| resolver -> terminal cells | The resolved absolute path contains the username. Only static literals may reach a cell (I-12). |
| test harness -> process env | Env mutation in tests can race other tests that run in the same process. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-jqc-01 | Information disclosure | help/preferences/detail labels | low | mitigate | Every label is the static literal `$GSD_HOME/.gsd/defaults.json`. The resolved path, which contains the username and possibly attacker-chosen bytes from the env, is never formatted into a cell. This keeps I-12 / T-t3s-05 and leaves the SAFE-07 escape surface unchanged. |
| T-jqc-02 | Tampering | `user_defaults_path` write target | low | accept | GSD_HOME redirects where the global template is written. Anyone who can set it can already set HOME, so this grants no new capability. A relative or whitespace-only value is used verbatim to match upstream (ID-1). Normalizing it would make the TUI and gsd-core disagree about which file is live, and that disagreement is the bug this item fixes. |
| T-jqc-03 | Tampering | test suite integrity | low | mitigate | No lib test mutates the env (ID-3). The only set_var/remove_var calls live in a one-test integration binary, which runs as its own process. Verified by the zero-count grep on config_json.rs and the one-count `#[test]` grep. |
| T-jqc-04 | Information disclosure | test fixtures | low | mitigate | Fixture defaults.json files carry only `mode`, never an API key, and assertions compare `.mode` and the path rather than Debug-printing a GsdConfig. This keeps T-jnf-03's never-print-values posture. |
| T-jqc-SC | Tampering | npm/pip/cargo installs | high | mitigate | No new dependency. `tempfile` is already in Cargo.toml and no install task exists, so the package-legitimacy gate has nothing to admit. Any new crate added during execution is a deviation and must stop for review. |
</threat_model>

<verification>
Run each command on its own. Never pipe test or clippy output through grep, because rtk filtering fakes counts. Capture to a file in the scratch directory and inspect it with the Grep tool when a search is needed.
- `cargo build` exits 0.
- `rtk proxy cargo clippy -- -D warnings` exits 0 (release gate 5).
- `rtk proxy cargo clippy --all-targets > <scratch>/clippy-all.log 2>&1`, then Grep the log for the touched paths (config_json.rs, state_reader_gsd_home.rs, help.rs, preferences.rs, detail.rs). Zero warnings may point into them. Pre-existing warnings elsewhere are noted, not fixed.
- `cargo fmt --check` exits 0. The tree was fmt-clean at 7303b41.
- `rtk proxy cargo test --no-fail-fast` must end with exactly ONE failure: the git-version constants witness in src/envelope/policy.rs, which is expected locally because local git differs from the pinned 2.55.0. Record suites/passed/failed/ignored in SUMMARY. Any other failure must be investigated. If it is in a file this plan did not touch, re-run it on a clean checkout of 7303b41 before calling it pre-existing.
</verification>

<success_criteria>
- With GSD_HOME set, the 7:Cfg global scope (and the `G` -> `g` preferences route) opens `$GSD_HOME/.gsd/defaults.json`. With GSD_HOME unset or empty, it behaves exactly as at 7303b41.
- Semantics match upstream `GSD_HOME || homedir()` for set, empty and unset values, and are pinned by pure unit tests plus one env-wiring integration test that cannot race.
- Every user-facing label and doc site listed in Task 2 names `$GSD_HOME/.gsd/defaults.json`. README lines 15-20, the sibling's regions and code comments are unchanged.
- The build, clippy and test gates in `<verification>` hold.
- SUMMARY.md carries ID-1..ID-8 under "inferred — for later audit". It also carries a follow-up note for the batch coordinator: detail.rs `*_api_key` help strings (~11440-11461) still name the tilde key-file paths, which upstream fd1bfd09e re-documented under `$GSD_HOME/.gsd/`. That region belongs to 261012-jqb's help refresh or a later task.
</success_criteria>

<output>
Create `.planning/quick/261012-jqc-honour-gsd-home-for-global-defaults-upstream-documents-globa/261012-jqc-SUMMARY.md` when done.
</output>
