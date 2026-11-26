---
quick_id: 261116-kft
mode: quick
phase: quick-261116-kft
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/config_set.rs
  - src/state_reader/mod.rs
  - src/state_reader/config_write.rs
  - src/ui/screens/owner_actions.rs
  - src/ui/screens/detail.rs
  - tests/config_set_roundtrip.rs
  - README.md
  - docs/CONFIGURATION.md
  - docs/ARCHITECTURE.md
  - docs/GSD-CORE-SYNC.md
autonomous: true
requirements: [QUICK-261116-kft]

estimate:
  tokens: 140000
  raw_tokens: 140000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "A project-scope Config-tab edit (dropdown, integer, string, secret, `x` clear) saves by running the user's own gsd-tools: `config-set <key.path> <value>`, `config-set <key.path> null` for a clear, `config-set-model-profile <profile>` for model_profile. The child runs with cwd = the project root and without the GSD workstream/session variables. This build never opens `.planning/config.json` or `.planning/.lock` for writing (D-01, D-04, D-05)."
    - "After every project-scope save attempt that read the file, the Defaults tab shows what is on disk (re-read and parsed). Success says `Config saved`. Failure says `Config NOT saved: <reason>`, or `Config partly saved (k of n changes): <reason>`, where the reason is gsd-tools' last stderr line, capped at 120 characters and escaped with `shown()`. It never contains a secret value (D-06)."
    - "A value that gsd-tools would store as a different JSON type (phase_naming `12`, a secret key `12345678`, response_language `true`), or a value or key segment that starts with `-`, is refused before any child runs. The status line names the key and a static reason, never the value."
    - "With no user gsd-core install, no `node` on PATH, an active workstream pointer in `.planning/active-workstream`, an unparseable `config.json`, or a target other than Linux/macOS, the save is refused with a status line and nothing is written (D-05, D-10)."
    - "The Global scope (`$GSD_HOME/.gsd/defaults.json`) still saves exactly as before: installer lock plus atomic write, `Defaults NOT saved` on refusal. README and docs/CONFIGURATION.md name it as the one GSD file this build still writes directly, because gsd-core has no command that writes it (D-02)."
    - "src/state_reader/config_write.rs no longer contains the `.planning/.lock` (Planning-flavor) lock replication (D-03)."
    - "Against the real installed gsd-tools, the argv the product plans leaves a config.json that the product's own diff sees as identical to the intended edit. The Rust replica of gsd-core's value parser agrees with `config-set --dry-run` on a fixed conformance table (D-08)."
  artifacts:
    - path: src/state_reader/config_set.rs
      provides: "Pure planner: GsdConfig before/after -> ordered gsd-tools calls (Set/Unset/ModelProfile), cli_parse replica, refusals that carry no value"
      contains: "pub fn plan_config_set"
    - path: src/ui/screens/detail.rs
      provides: "save_project_config: fresh read, plan, run gsd-tools per call under one budget, re-read, status; persist_active_config routes the Project scope to it"
      contains: "fn save_project_config"
    - path: src/ui/screens/owner_actions.rs
      provides: "Shared bounded gsd-tools child helper used by acknowledge and config saves"
      contains: "fn run_gsd_tools"
    - path: src/state_reader/config_write.rs
      provides: "Global-defaults-only direct writer (installer lock + atomic_write)"
      contains: "fn update_global_defaults"
    - path: tests/config_set_roundtrip.rs
      provides: "Oracle round trip and cli_parse conformance against real gsd-tools"
      contains: "GSD_META_MANAGER_ALLOW_MISSING_ORACLE"
  key_links:
    - from: src/ui/screens/detail.rs
      to: src/state_reader/config_set.rs
      via: "save_project_config calls plan_config_set(&base, &edited) and runs each ConfigSetCall::args()"
      pattern: "plan_config_set"
    - from: src/ui/screens/detail.rs
      to: src/ui/screens/owner_actions.rs
      via: "resolve_launcher() (user installs only, test seam force_launcher), on_path, active_workstream_is_set, run_gsd_tools"
      pattern: "run_gsd_tools"
    - from: src/ui/screens/owner_actions.rs
      to: src/launcher/process.rs
      via: "run_gsd_tools -> try_run_bounded_with_stderr_env(program, args, Some(cwd), ACK_ENV_REMOVE, &[], budget)"
      pattern: "try_run_bounded_with_stderr_env"
    - from: tests/config_set_roundtrip.rs
      to: src/state_reader/config_set.rs
      via: "plan_config_set / ConfigSetCall::args / cli_parse driven through the installed gsd-tools"
      pattern: "cli_parse"
---

<!-- planner-discipline-allow: ConfigWriteTarget::Project -->
<!-- planner-discipline-allow: ConfigWriteTarget -->
<!-- planner-discipline-allow: LockFlavor -->
<!-- planner-discipline-allow: PlanningLockBody -->
<!-- planner-discipline-allow: planning_on_exists -->
<!-- planner-discipline-allow: planning_verified_live -->
<!-- planner-discipline-allow: is_transient_create_error -->
<!-- planner-discipline-allow: STALE_COUNTER -->
<!-- planner-discipline-allow: update_gsd_config -->
<!-- Positive gates (EXPECT>=1) on new identifiers: these literals must appear in actions. -->
<!-- planner-discipline-allow: plan_config_set -->
<!-- planner-discipline-allow: fn run_gsd_tools -->
<!-- planner-discipline-allow: active_workstream_is_set -->
<!-- planner-discipline-allow: config-set-model-profile -->
<!-- planner-discipline-allow: config-set -->
<!-- planner-discipline-allow: config_set.rs -->
<!-- planner-discipline-allow: cmdline -->
<!-- planner-discipline-allow: config_set_roundtrip -->

<objective>
Route the Config tab's project-scope `.planning/config.json` saves through gsd-core's own writer, `gsd-tools config-set`, instead of writing the file from Rust. This enforces the user decision of 2026-11-16 recorded in CLAUDE.md and PROJECT.md (commit cf7a246c): GSD files are read-only for this tool, and every change goes through gsd-tools or a GSD command.

Decisions (from the orchestrator; [inferred] ones are recorded as I-kft-N in the SUMMARY):
- D-01: Project-scope saves run `config-set <key.path> <value>` once per changed leaf. The edit closure is applied to a clone of a FRESH disk read, the two are diffed as `serde_json::Value`, and each changed leaf becomes one call. A removed leaf becomes `config-set <key.path> null`, which gsd-core unsets (`cmdConfigSet`, bin/lib/config.cjs:1006). The cache is then refreshed from a bounded re-read.
- D-02 [inferred, locked by orchestrator]: `$GSD_HOME/.gsd/defaults.json` has no gsd-tools writer. Only the installer writes it, under `acquireInstallMigrationLock` (bin/lib/installer-migrations.cjs:527 at 1.21.0), and the /gsd-settings workflow has the agent write it. The Global-scope edit keeps today's direct path (installer-lock replication + `atomic_write`). The gap is documented and reported to the user. Do not remove the Global edit feature.
- D-03: Delete the Planning-flavor `.planning/.lock` replication and everything else that becomes unused. Keep what Global needs.
- D-04: `model_profile` is set with `config-set-model-profile <profile>`, which validates against VALID_PROFILES and lowercases.
- D-05: Scrub the workstream/session selector variables (`ACK_ENV_REMOVE`). Refuse when `.planning/active-workstream` names a workstream [inferred I-kft-2: gsd-core has no spelling for "the root", same refusal as Phase 37's acknowledge]. cwd = the project root, without `--project-dir` [inferred I-kft-1: Phase 37 precedent; gsd-core's `legacyDispatchCwd` keeps a cwd that holds `.planning/`].
- D-06: The status line and the log never carry a secret value. A secret travels to the short-lived child as argv, because config-set has no stdin form. This is an accepted, documented risk [inferred I-kft-4].
- D-07 [inferred I-kft-3]: The save stays synchronous in the key handler, with one total budget of 3 s per edit. Measured: one `config-set` takes about 150 ms (node startup) on this machine. Moving it to spawn_blocking would need a new Action, a busy guard and stale-edit handling, which is not trivial.
- D-08: Unit tests (diff to argv, outcome lines, refusals through the forced launcher seam) plus an oracle integration test with REAL gsd-tools on throwaway temp projects. It uses the `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` policy from tests/owner_acknowledge_roundtrip.rs. Real projects under ~/projects are read-only. Old config_write tests for removed code go; Global-scope tests stay.
- D-09: Update README/docs only where they describe the old write path or the gsd-tools surface.
- D-10: Use the Phase 37 runner pattern. `resolve_user_gsd_tools()` resolves user installs only (honors CLAUDE_CONFIG_DIR/CODEX_HOME, never a project-shipped gsd-tools), with a node-on-PATH check and `try_run_bounded_with_stderr_env`. Linux/macOS only; elsewhere the save fails visibly. Factor a shared helper instead of duplicating the runner.

Measured by the planner against the installed gsd-core 1.21.0 (re-measure if in doubt):
- The value parser (`parseConfigArgValue`, bin/lib/config.cjs:189-209, `DECIMAL_LITERAL` at :180) coerces `true`/`false`/`null`, decimal literals (`1.`, `+1`, `007` -> 7), and a leading `[`/`{` that JSON-parses. Everything else stays a string; `١٢`, `0x10`, `TRUE`, `[bad` and the empty string all stay strings. `project_code` keeps its text verbatim (:1032), except that a bare `null` still unsets it. Consequence: `config-set phase_naming 12` stores the NUMBER 12, and `parse_gsd_config` then fails on the whole file. That is why D-01 needs the fidelity guard.
- gsd-tools consumes global flags anywhere in argv: `--cwd`/`--cwd=`, `--project-dir`, `--ws`, `--pick`, `--default`, `--raw`, `--json-errors`, `--exit-contract=`, help tokens `-h --help -? --h --usage`, and rejects `--version -v` (bin/lib/cli-surface.cjs GLOBAL_FLAGS). Every one starts with `-`.
- Errors are `Error: <text>` on stderr with exit 1, and the text echoes the value (`Invalid mode 'bogus'`). The unknown-key error is one very long line. gsd-core masks only `brave_search`, `firecrawl`, `exa_search` in its output (bin/lib/secrets.cjs). `config-set tavily_search <key>` echoes the plaintext key on stdout.
- `config-set <key> null --dry-run` checks the key (`assertKnownConfigKey`) and stops before every value validator. `--dry-run` prints `{"dry_run":true,"would_update":true,"key":..,"value":..}`, or `would_unset` for null.

Purpose: honor the read-only constraint for the file GSD itself owns most visibly, without losing the Config tab's ability to edit project settings.
Output: src/state_reader/config_set.rs, a slimmer src/state_reader/config_write.rs, the rewired save path in src/ui/screens/detail.rs, a shared child helper in src/ui/screens/owner_actions.rs, tests/config_set_roundtrip.rs, doc updates.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@src/state_reader/config_write.rs
@tests/owner_acknowledge_roundtrip.rs

Read only these ranges of the large files (no whole-file reads):
- src/ui/screens/detail.rs: `shown` at :83; the save call sites at :5546-5619 (dropdown, Integer), :5751-5797 (`x` clear), :7290-7340 (string/secret Enter); `config_write_path` / `PersistOutcome` / `persist_active_config` at :13971-14056; edit helpers `set_config_value` :14070, `set_string_value` :14660, `set_secret_value` :14812, `clear_config_value` :14844, `mutate_config_entry` :15700; tests `config_save_fixture` .. `config_save_global_defaults_refuses_while_the_installer_lock_is_live` at :25700-25849 and `g_switches_the_config_scope_and_writes_land_in_that_scopes_file` at :25858-25997.
- src/ui/screens/owner_actions.rs: `ACK_ENV_REMOVE` :237-260, `active_workstream_is_set` / `on_path` :469-485, launcher seam `FORCED_LAUNCHER` / `force_launcher` / `test_launcher` / `resolve_launcher` :646-680, `ack_request` node check :706-712, `AckExit` / `spawn_ack` (both cfg variants) / `ack_failure_line` / `run_ack_within` :806-900, its tests :1535-1637.
- src/state_reader/config_json.rs:60-235 (`ExtraKeys`, `ApiKeySetting` with its hand-written Debug, `GsdConfig`).
- src/state_reader/config_secrets.rs:78-99 (`is_secret_key`).
- src/state_reader/queue_md.rs:15-31 (`GsdToolsCmd`), :171-213 (`resolve_user_gsd_tools`).
- src/launcher/process.rs:456-510 (`try_run_bounded_with_stderr_env`).
- tests/spawn_seam_guard.rs:86 (`SPAWN_MARKERS`): calling the bounded runner needs no allowlist entry. tests/async_blocking_guard.rs:76: the save must stay a sync fn.
- Installed gsd-core (outside the repo, `${CLAUDE_CONFIG_DIR:-~/.claude}/gsd-core/bin/lib/`): config.cjs:180-209, :978-1070; cli-surface.cjs:34-46; secrets.cjs:16-30.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: Tracer: one project-scope edit saved by gsd-tools config-set, end to end</name>
  <files>src/state_reader/config_set.rs, src/state_reader/mod.rs, src/state_reader/config_write.rs, src/ui/screens/owner_actions.rs, src/ui/screens/detail.rs, tests/config_set_roundtrip.rs</files>
  <precondition>The user's gsd-core install resolves (`${CLAUDE_CONFIG_DIR:-~/.claude}/gsd-core/bin/gsd-tools.cjs` is a file) and `node` is on PATH; the oracle test fails by design otherwise.</precondition>
  <behavior>
    - plan_config_set: commit_docs true -> false gives exactly one call whose args are `config-set`, `commit_docs`, `false`.
    - An absent `workflow` block becoming one with `plan_check = true` gives exactly `config-set workflow.plan_check true` (null siblings emit nothing).
    - Clearing a set leaf (commit_docs -> None, or the last field of a block) gives `config-set <path> null`; an unchanged config gives an empty plan; an unchanged `extra` key never appears.
    - Strings go verbatim (`response_language` = `Deutsch`). `project_code` = `007` is allowed. `phase_naming` = `12`, `response_language` = `true`, and a value `[1]` are refused as Coerced. `[x` is allowed. A secret slot `Key("12345678")` is refused, `Key("sk-abc")` is allowed.
    - A value starting with `-`, a value with NUL, and an extra key containing `.` or starting with `-` are refused. Refusals and the Debug of a call never contain the value.
    - cli_parse table: `true`, `false`, `null`; `1.` -> 1; `.5` -> 0.5; `+1` -> 1; `1e3` -> 1000; `1e400`, `Infinity`, `NaN`, `0x10`, ` 12`, `١٢` stay strings; `[1,2]` -> array; `[bad` stays a string; project_code `true` stays the string; project_code `null` -> null.
    - Defaults tab, project scope, fake launcher: flipping commit_docs runs `config-set commit_docs false` with cwd = project root. The cache equals what the fake wrote. Status is `Config saved`. The TUI itself creates nothing in `.planning/`.
    - Fake exits 1 after a defaults warning plus `Error: Invalid value`: status is `Config NOT saved: Error: Invalid value`, disk is byte-identical, cache equals disk. No launcher: the install-hint refusal, nothing written.
  </behavior>
  <action>
Implements D-01, D-08, D-10 for one path. Two of the six files are one-line edits: the module declaration, and a visibility change.

1. Create `src/state_reader/config_set.rs` and declare `pub mod config_set;` in `src/state_reader/mod.rs` (alphabetical, after `config_secrets`). The module is pure: no I/O, no process. The module doc says why it exists (D-01, the CLAUDE.md read-only rule, commit cf7a246c). It records the replica's provenance: gsd-core 1.21.0 bin/lib/config.cjs `DECIMAL_LITERAL` :180, `parseConfigArgValue` :189-209, bare-null unset :1006, `project_code` verbatim :1032, and the GLOBAL_FLAGS list in bin/lib/cli-surface.cjs. Public surface:
   - `pub enum ConfigSetCall { Set { key: String, arg: String }, Unset { key: String } }`, with `pub fn key(&self) -> &str` and `pub fn args(&self) -> Vec<String>`. Set gives `config-set`, key, arg. Unset gives `config-set`, key, `null`. Write Debug by hand with `write!` so it prints the key and never the arg (ApiKeySetting precedent; tests/spawn_seam_guard.rs rejects the tuple-builder's terminal call name in src/).
   - `pub struct ConfigSetRefusal { pub key: String, pub reason: RefusalReason }` and `pub enum RefusalReason { Serialize, KeyPath, FlagShaped, Nul, Coerced { stored_as: &'static str } }`. `pub fn reason_text(&self) -> String` uses static text only. Coerced: gsd-tools config-set would store this value as a {stored_as}, not as text. FlagShaped: the value starts with '-', which gsd-tools would read as a flag. KeyPath: this key path cannot be passed to gsd-tools config-set. Nul: the value contains a NUL byte. Serialize: the config could not be serialized. No variant carries a value.
   - `pub fn cli_parse(key: &str, arg: &str) -> serde_json::Value` replicates the parser. Check, in order: the literals `true`, `false`, `null`. Then DECIMAL_LITERAL, written with ASCII `[0-9]` and not the regex `\d`, because Rust's `\d` is Unicode while JS's is ASCII. Compile it once in a `static RE: OnceLock<Regex>` (src/agents/waves.rs precedent). A match whose `f64` parse is finite becomes a Number: an integral value within ±2^53 as an integer Number, otherwise f64. Then a leading `[` or `{` that `serde_json::from_str` accepts becomes that value. Anything else is the string. Finally, a non-null result for key `project_code` becomes the string `arg`.
   - `pub fn plan_config_set(before: &GsdConfig, after: &GsdConfig) -> Result<Vec<ConfigSetCall>, ConfigSetRefusal>`. Serialize both with `serde_json::to_value`; an error gives Serialize with an empty key. Walk the two values together, joining path segments with `.`, and treat null and absent the same. If one side is an object and the other is an object or unset, recurse over the union of keys (serde_json's Map is a BTreeMap here, so the order is sorted). Otherwise: equal values emit nothing; an unset `after` emits Unset; anything else emits Set. The Set arg is the string itself for a JSON string, `true`/`false` for a bool, serde_json's rendering for a number, and compact JSON for an array or object. Guards run before a call is pushed:
     - Every key segment must be non-empty and contain no `.` and no NUL, and must not start with `-`; otherwise KeyPath.
     - A Set arg that starts with `-` is FlagShaped; one that contains NUL is Nul.
     - `cli_parse(key, arg)` must equal the intended value, else Coerced. Numbers compare by f64, and an integer above 2^53 in magnitude counts as unequal. `stored_as` is one of: number, boolean, null (unset), JSON array, JSON object, text.
     - The first refusal aborts the whole plan; never return a partial plan.
   Write the in-module tests from the behavior list first (RED), then implement (GREEN).
2. In `src/state_reader/config_write.rs`, change only `read_bounded` and `CONFIG_READ_CAP` to `pub(crate)`. Task 2 restructures the rest of this file.
3. In `src/ui/screens/owner_actions.rs`, factor out the shared child helper (D-10):
   - Make `resolve_launcher`, `on_path` and `active_workstream_is_set` `pub(crate)`.
   - Rename `AckExit` to `pub(crate) type GsdToolsExit`.
   - Extract `pub(crate) fn run_gsd_tools(program: &str, argv: &[String], cwd: &Path, budget: Duration) -> Result<GsdToolsExit, BoundedFailure>` from `spawn_ack`, keeping both cfg variants. Linux/macOS calls `try_run_bounded_with_stderr_env` with `ACK_ENV_REMOVE` and no extra env. Every other target returns the existing unsupported-platform result. `spawn_ack` delegates to it, so the Phase 37 run_ack tests cover the helper.
   - Extend `ACK_ENV_REMOVE`'s doc to say config-set children use it too. Keep the name, because tests/owner_acknowledge_roundtrip.rs imports it.
   - Under `cfg(test)`, an unforced `resolve_launcher` returns None [inferred I-kft-11: no unit test may run this machine's gsd-core]. If an existing test relied on real resolution, force a launcher in that test.
4. In `src/ui/screens/detail.rs`:
   - Add a sync (never `async`) `fn save_project_config(project_root, cached: &GsdConfig, mutate) -> ProjectSave` next to `persist_active_config`, with `enum ProjectSave { Saved { fresh: Option<GsdConfig>, changed: bool }, NotApplied, Failed { status: String, fresh: Option<GsdConfig> } }`.
   - `persist_active_config` routes the Project scope with a known project path to `save_project_config`. The project variant of the config-write target is then never constructed. `cargo build` may warn that it is never constructed until Task 2 deletes it in this plan. The Global branch and the no-path in-memory branch stay byte-for-byte as they are. Saved sets the cache to `fresh` when it is Some, sets status `Config saved` (`Config unchanged` when `changed` is false) and returns Applied. Failed refreshes the cache from `fresh` when it is Some, sets `status` and returns Failed. Existing caller handling (the `x` path's `Cleared …`) is unchanged.
   - Steps inside `save_project_config`, on Linux/macOS. Other targets refuse before any I/O with `Config NOT saved: running gsd-tools is not supported on this platform`. Lay out the cfg so Windows clippy sees no dead code or unused import.
     - (a) Base = a bounded read of `<root>/.planning/config.json` (`config_write::read_bounded(path, CONFIG_READ_CAP, true)`). A read that does not parse gives Failed `Config NOT saved: the file on disk no longer parses — press r to reload`. NotFound gives base = clone of `cached`. Any other error gives Failed `Config NOT saved: {ErrorKind}`. Applying the edit to a fresh read keeps I-guw-4's semantics [I-kft-5].
     - (b) `edited = base.clone()`; if `mutate(&mut edited)` returns false, give NotApplied.
     - (c) Call `plan_config_set(&base, &edited)`. A refusal gives Failed `Config NOT saved: {shown(key)}: {reason_text}`, with `fresh` = the base when it came from disk. An empty plan gives Saved with `changed: false`.
     - (d) `resolve_launcher()` None gives Failed `Config NOT saved: gsd-tools.cjs not found. Install: npx -y @opengsd/gsd-core@latest --claude --local`.
     - (e) Run the calls in order through `run_gsd_tools`, with argv = launcher prefix args + `call.args()`, cwd = root, and one total `CONFIG_SET_BUDGET` of 3 s for the whole edit. Each call gets what is left of the budget. Stop at the first failure. The reason is the child's last non-empty stderr line (the bounded runner already returns the last line, past gsd-tools' leading defaults warning), else its first non-empty stdout line, else `gsd-tools failed without a message`. Cap the reason at 120 chars, then escape it with `shown()`. Status: `Config NOT saved: {reason}`. A BoundedFailure uses the ack wording with the `Config NOT saved:` prefix (Task 2 refines this).
     - (f) Whatever the outcome, re-read and parse; `fresh` = the parse when it succeeds [I-kft-6].
   - Tests, gated `#[cfg(any(target_os = "linux", target_os = "macos"))]`. Add a helper `fake_gsd_tools(script)` that returns `GsdToolsCmd { program: "sh", prefix_args: ["-c", script, "gsd-tools"] }`, forced with `owner_actions::force_launcher` (thread-local). The script logs `"$*"` to `$PWD/argv.log` (project root, not `.planning/`) and may rewrite `.planning/config.json` with known content.
     - Replace `config_save_takes_and_releases_the_planning_lock` with `config_save_runs_gsd_tools_config_set_and_reloads_from_disk`. Assert: argv.log is `config-set commit_docs false`; the cache equals the fake's file; status is `Config saved`; `.planning/` lists only `config.json`.
     - Replace `config_save_refuses_while_a_live_gsd_holds_the_planning_lock` with `config_save_reports_the_gsd_tools_refusal_and_keeps_the_cache_equal_to_disk`.
     - Add `config_save_without_gsd_tools_refuses_and_writes_nothing`, using `force_launcher(None)`.
     - In `g_switches_the_config_scope_and_writes_land_in_that_scopes_file`, force a fake that writes `{"mode":"yolo","commit_docs":false}` and keep every assertion.
     - `config_save_global_defaults_refuses_while_the_installer_lock_is_live` stays unchanged.
5. Create `tests/config_set_roundtrip.rs`, an oracle test:
   - Copy the `Oracle` resolve/find shape and the `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` policy from tests/owner_acknowledge_roundtrip.rs: a missing oracle fails unless the variable opts out.
   - The module doc says it uses temp projects only and never a registered project.
   - Helper `apply(root, before, after)`: call `plan_config_set`, run each call's `args()` through the oracle with cwd = root and every `ACK_ENV_REMOVE` variable removed, then re-read and parse. Assert that `plan_config_set(&after, &on_disk)` is `Ok` and empty, i.e. the product's own diff sees the intended config on disk.
   - Cases, each from a fresh fixture `{"mode":"interactive","commit_docs":true,"phase_naming":"sequential"}`: commit_docs -> false; response_language -> `Deutsch`; commit_docs -> None; `workflow.plan_check` -> true from an absent block; context_window -> 400000. Also phase_naming -> `12`: plan_config_set returns Coerced, nothing runs, and the file stays byte-identical.
  </action>
  <verify>
    <automated>rtk bypass cargo test --locked --lib -- config_set config_save g_switches_the_config_scope run_ack ack_request</automated>
    <automated>rtk bypass cargo test --locked --test config_set_roundtrip --test owner_acknowledge_roundtrip</automated>
    <automated>rtk bypass bash -c 'set -e; test -f src/state_reader/config_set.rs; test -f tests/config_set_roundtrip.rs; test "$(grep -c "^pub mod config_set;" src/state_reader/mod.rs)" -eq 1; test "$(grep -v "^\s*//" src/ui/screens/detail.rs | grep -c "ConfigWriteTarget::Project")" -eq 0; test "$(grep -v "^\s*//" src/ui/screens/detail.rs | grep -c "plan_config_set")" -ge 1; test "$(grep -c "fn run_gsd_tools" src/ui/screens/owner_actions.rs)" -ge 1; echo T1-OK'</automated>
    Gate baselines:
    - `ls src/state_reader/config_set.rs 2>/dev/null | wc -l`: [HEAD=0 EXPECT==1]
    - `ls tests/config_set_roundtrip.rs 2>/dev/null | wc -l`: [HEAD=0 EXPECT==1]
    - `^pub mod config_set;` in src/state_reader/mod.rs: [HEAD=0 EXPECT==1]
    - non-comment `ConfigWriteTarget::Project` in detail.rs: [HEAD=1 EXPECT==0]
    - non-comment `plan_config_set` in detail.rs: [HEAD=0 EXPECT>=1]
    - `fn run_gsd_tools` in owner_actions.rs: [HEAD=0 EXPECT>=1]
  </verify>
  <done>A commit_docs flip on the Defaults tab (project scope) is saved by the user's gsd-tools `config-set`, never written by this build. The cache is reloaded from disk. Refusals and a missing install leave the file untouched with a visible status. The oracle round trip passes against the installed gsd-core 1.21.0. Phase 37's acknowledge tests still pass through the shared helper. Committed as `feat(quick-261116-kft): save project config through gsd-tools config-set`.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Expansion: model_profile, secrets, refusals and partial saves; retire the Planning-flavor lock</name>
  <files>src/state_reader/config_set.rs, src/state_reader/config_write.rs, src/ui/screens/detail.rs, tests/config_set_roundtrip.rs</files>
  <behavior>
    - A model_profile change plans `config-set-model-profile quality`. A profile starting with `-` is refused.
    - Saving a secret (brave_search, tavily_search) reports `Config saved` with no value in the status. A failing child whose stderr echoes the secret yields a status with `<redacted>` and without the secret.
    - `.planning/active-workstream` naming `ws1` refuses the save before any child runs. So do `node` missing from PATH (launcher program `node`) and an unparseable config.json.
    - Two changed leaves where the second call fails give `Config partly saved (1 of 2 changes): Error: nope`, with cache equal to disk. A child that outlives an injected 1 s budget gives the `timed out after 1 s` line. A spawn failure says nothing was run.
    - Global scope unchanged: the installer-lock test still says `Defaults NOT saved … pid 1`, and global edits still land in the TempDir defaults file.
    - Oracle: model_profile and secret round trips leave an empty product diff. On every conformance-table row, cli_parse agrees with `config-set --dry-run`.
  </behavior>
  <action>
Implements D-02, D-03, D-04, D-05, D-06, D-07, D-10 on the Task 1 path.

1. Retire the Planning flavor in `src/state_reader/config_write.rs` (D-03). Do this first, so the build is warning-free again.
   - Delete `LockFlavor`, `PlanningLockBody`, `planning_on_exists`, `planning_verified_live`, `is_transient_create_error`, `STALE_COUNTER`, the whole `ConfigWriteTarget` enum, the Planning arms of `lock_body`, `acquire`, `LockPolicy::tui` and `status_text`, and the six `planning_lock_*` tests.
   - Rename the entry point to `update_global_defaults(path: &Path, policy: LockPolicy, fallback: &GsdConfig, mutate) -> Result<Option<GsdConfig>, ConfigWriteError>`. The lock is `<dir>/gsd-install-migration.lock`. `LockPolicy::tui()` takes no argument (2 s budget, 50 ms poll). `ConfigWriteError::status_text()` takes no argument and always says `Defaults NOT saved: …`. Keep the non-unix refusals for `atomic_write` and `update_global_defaults` [I-kft-8].
   - Keep `read_bounded`, `CONFIG_READ_CAP`, `body_pid`, `probe_pid`, `install_on_exists`, `LockGuard` and `atomic_write`.
   - Convert the surviving read-modify-write tests to the global target: `update_applies_the_edit_to_the_on_disk_config_not_the_cached_snapshot`, `update_refuses_an_unparseable_config_on_disk`, `update_creates_an_absent_file_from_the_fallback`, `release_does_not_delete_a_lock_it_no_longer_owns` (with an installer-lock body), `update_without_a_mutation_writes_nothing`, `status_text_carries_no_path_and_names_only_the_pid`. Keep the `atomic_write_*` tests and `install_lock_rules`.
   - Rewrite the module doc. This module is now the one place this build still writes a GSD file itself, because gsd-core has no command that writes defaults.json (only its installer, plus /gsd-settings workflow prose). That is a known gap against the read-only rule (D-02). Project saves go through `state_reader::config_set` (quick 261116-kft).
   - Point `persist_active_config`'s Global branch at the new call; its behavior is unchanged.
   - src/state_reader/driver_frontmatter.rs:829 names `config_write::read_bounded`, which stays. Leave that comment alone.
2. model_profile (D-04): add `ConfigSetCall::ModelProfile { profile: String }`, with args `config-set-model-profile`, profile. `plan_config_set` emits it for a changed, non-empty top-level `model_profile`; an unset still goes through `config-set model_profile null`. The `-`, NUL and fidelity guards apply to the profile like any value; fidelity is a plain string. gsd-core lowercases and validates the profile, and the re-read shows what it stored.
3. Secrets (D-06): in `save_project_config`, when `config_secrets::is_secret_key(call.key())` holds and the call carries a non-empty string value, replace every occurrence of that value in the stderr line and the stdout with `<redacted>`. Do this before redact-then-cap-then-`shown()`, and before anything is logged. Never log argv for any call. Log the key, success, and the redacted reason only. gsd-core masks only three of the seven secret keys in its own output, so `tavily_search` comes back in plaintext on stdout; the redaction is what keeps it out.
4. Refusals and failure lines (D-05, D-07, D-10). Check these before any child runs, in this order:
   - Platform.
   - `owner_actions::active_workstream_is_set(root)` gives `Config NOT saved: an active workstream is set (.planning/active-workstream), so gsd-tools would write that workstream's config, not this one` [I-kft-2].
   - The fresh read.
   - The plan.
   - The launcher.
   - If the launcher program is `node` and `owner_actions::on_path(PATH, "node")` is false: `Config NOT saved: node not found on PATH`.
   BoundedFailure lines:
   - TimedOut: `gsd-tools timed out after {n} s; nothing is confirmed — press r to reload`.
   - Spawn or Sink: `could not start gsd-tools ({kind}); nothing was run`.
   - Io: `lost gsd-tools while it ran ({kind}); nothing is confirmed — press r to reload`.
   A budget exhausted before a call starts gives the timeout line. When k ≥ 1 of n calls succeeded before the failure, the prefix is `Config partly saved ({k} of {n} changes): ` instead of `Config NOT saved: `. Inject the budget for tests through `save_project_config_within` (the `run_ack_within` precedent).
5. Tests in detail.rs, using the fake launcher and the Task 1 helper:
   - The model_profile dropdown edit writes argv.log `config-set-model-profile quality`.
   - A secret Enter on brave_search gives status `Config saved` without the value.
   - A failing fake that echoes its third positional (the value) to stderr on a secret key gives a status containing `<redacted>`, not the key.
   - The active-workstream refusal leaves no argv.log.
   - A two-field closure, with the fake failing on the second key, gives the partly-saved line, and the cache equals disk.
   - `exec sleep 30` with an injected 1 s budget gives the timeout line.
   - An unparseable config.json gives its refusal, with no argv.log.
   Add planner unit tests for ModelProfile emission and refusal.
6. In `tests/config_set_roundtrip.rs`:
   - Add model_profile -> `quality`. Add brave_search and tavily_search set to `ApiKeySetting::Key("sk-test-…")`: the plaintext lands on disk and the product diff is empty.
   - Add a conformance test. For each arg in a fixed table, run the oracle's `config-set response_language <arg> --dry-run`. Compare its stdout JSON `value` (or `would_unset` for `null`) with `cli_parse("response_language", arg)`, using the planner's equality.
   - The table includes `true`, `false`, `null`, `TRUE`, `0`, `12`, `007`, `1.5`, `.5`, `1.`, `+1`, `1e3`, `1E+3`, `1e400`, `Infinity`, `NaN`, `0x10`, `1_000`, ` 12`, `12 `, `١٢`, `[1,2]`, `{"a":1}`, `[bad`, `{`, the empty string and `plain text`, plus `project_code` rows `007`, `true`, `[1]`.
   - Assert that at least one row parses to a non-string, so the test cannot pass vacuously. The `١٢` row pins the ASCII-digit rule.
   - `response_language` has no per-key validator at 1.21.0 (measured). If a future install adds one, switch to another free-form key and say why in a comment.
  </action>
  <verify>
    <automated>rtk bypass cargo test --locked --lib -- config_set config_save config_write g_switches_the_config_scope run_ack ack_request</automated>
    <automated>rtk bypass cargo test --locked --test config_set_roundtrip --test owner_acknowledge_roundtrip</automated>
    <automated>rtk bypass cargo clippy --locked --all-targets -- -D warnings</automated>
    <automated>rtk bypass bash -c 'set -e; F=src/state_reader/config_write.rs; for t in LockFlavor PlanningLockBody planning_on_exists planning_verified_live is_transient_create_error STALE_COUNTER ConfigWriteTarget; do test "$(grep -v "^\s*//" $F | grep -c "$t")" -eq 0 || { echo "LEFT $t"; exit 1; }; done; test "$(grep -c "fn update_global_defaults" $F)" -ge 1; D=src/ui/screens/detail.rs; test "$(grep -v "^\s*//" $D | grep -c "update_gsd_config")" -eq 0; test "$(grep -v "^\s*//" $D | grep -c "ConfigWriteTarget")" -eq 0; test "$(grep -v "^\s*//" $D | grep -c "active_workstream_is_set")" -ge 1; test "$(grep -c "config-set-model-profile" src/state_reader/config_set.rs)" -ge 1; echo T2-OK'</automated>
    Gate baselines (non-comment lines):
    - config_write.rs `LockFlavor`: [HEAD=18 EXPECT==0]
    - config_write.rs `PlanningLockBody`: [HEAD=2 EXPECT==0]
    - config_write.rs `planning_on_exists`: [HEAD=2 EXPECT==0]
    - config_write.rs `planning_verified_live`: [HEAD=2 EXPECT==0]
    - config_write.rs `is_transient_create_error`: [HEAD=2 EXPECT==0]
    - config_write.rs `STALE_COUNTER`: [HEAD=2 EXPECT==0]
    - config_write.rs `ConfigWriteTarget`: [HEAD=18 EXPECT==0]
    - config_write.rs `fn update_global_defaults`: [HEAD=0 EXPECT>=1]
    - detail.rs `update_gsd_config`: [HEAD=2 EXPECT==0]
    - detail.rs `ConfigWriteTarget`: [HEAD=3 EXPECT==0]
    - detail.rs `active_workstream_is_set`: [HEAD=0 EXPECT>=1]
    - config_set.rs `config-set-model-profile` (file absent at HEAD): [HEAD=0 EXPECT>=1]
  </verify>
  <done>Every project-scope edit kind saves through gsd-tools: model_profile through `config-set-model-profile`, and secrets without leaking into the status or the log. Workstream, node, parse, budget and partial failures are visible and leave the cache equal to disk. The Planning-flavor lock code is gone, and the Global scope behaves as before through `update_global_defaults`. Clippy is clean. Committed as `feat(quick-261116-kft): cover every config edit kind via gsd-tools and drop the .planning lock replica`.</done>
</task>

<task type="auto">
  <name>Task 3: Docs, refused-row measurement, full gates</name>
  <files>README.md, docs/CONFIGURATION.md, docs/ARCHITECTURE.md, docs/GSD-CORE-SYNC.md</files>
  <action>
Implements D-09 and reports D-02.

1. README.md, two edits:
   - The `gsd-tools` row of the comparison table near line 63 still describes gsd-tools as used for reading only, which has been wrong since Phase 37's acknowledge. Rewrite it to list what the app runs: `smart-entry --json` for suggestions, `audit-open … acknowledge` for the owner inbox, and `config-set` / `config-set-model-profile` for Config-tab project saves. All of these come from the user's own gsd-core install.
   - In "What it writes" (near line 69), split the config bullet. `.planning/config.json` is changed only by running gsd-tools `config-set` (GSD's own writer and lock). `$GSD_HOME/.gsd/defaults.json` is still written directly, under GSD's installer lock and replaced atomically, because gsd-core has no command that writes it (D-02).
   - The Windows sentence near line 865 (Defaults-tab saves refused there) stays true; keep it.
2. docs/CONFIGURATION.md, section "Secret values in GSD project config":
   - Under "Editing a secret row", add one bullet. A key gsd-tools would store as another type is refused with the key named and no value shown: all digits, `true`/`false` typed as a key is still the override, `null`, a leading `[`/`{` that is valid JSON, or a leading `-`.
   - Under "Storage", add that a project save passes the key to `gsd-tools config-set` as a command-line argument, because gsd-tools has no stdin form. While that child runs (about 150 ms), other local processes can read it from `/proc/<pid>/cmdline` on Linux unless `/proc` is mounted with `hidepid`. GSD's own /gsd-settings flow does the same.
   - Add a short paragraph on how Config-tab project saves work: the user's gsd-tools, one `config-set` per changed key, a 3 s budget, and the refusals (no install, no node, active workstream, a value gsd-tools would retype, an unparseable file). Global saves still write defaults.json directly.
3. docs/ARCHITECTURE.md line 273: add `config_set.rs` to the state_reader listing, with a one-line role: plans the gsd-tools `config-set` calls for a Config-tab save.
4. docs/GSD-CORE-SYNC.md "Next sync", item 1: add one bullet. Re-run `cargo test --test config_set_roundtrip` against the new oracle. If `parseConfigArgValue`, `DECIMAL_LITERAL`, the `project_code` exception or GLOBAL_FLAGS changed, update the replica in src/state_reader/config_set.rs. Do not edit the historical v1.21.0 invocation audit.
5. Measure, and do not commit the scratch. Collect the key literals from the match arms of `set_config_value`, `set_string_value`, `set_secret_value`, `mutate_config_entry` and `clear_config_value` in src/ui/screens/detail.rs, and dedupe them. In a scratch project under the session scratchpad, run the installed `node <gsd-tools.cjs> config-set <key> null --dry-run` for each key; null stops after the key check and before every value validator. List the keys gsd-tools refuses in the SUMMARY under "Rows gsd-core refuses". Those rows now fail visibly on save. Report this to the user, together with the D-02 defaults.json gap and I-kft-1..11.
6. Run the full gates below and fix anything they surface. If a gate fails because of something this task did not cause, fix it in a separate commit.
  </action>
  <verify>
    <automated>rtk bypass cargo test --locked --no-fail-fast</automated>
    <automated>rtk bypass cargo clippy --locked --all-targets -- -D warnings</automated>
    <automated>rtk bypass cargo clippy --locked --target x86_64-pc-windows-gnu --all-targets -- -D warnings</automated>
    <automated>rtk bypass cargo clippy --locked --target aarch64-apple-darwin --all-targets -- -D warnings</automated>
    <automated>rtk bypass cargo fmt --check</automated>
    <automated>rtk bypass bash -c 'set -e; test "$(grep -c "Called read-only" README.md)" -eq 0; test "$(grep -c "config-set" README.md)" -ge 1; test "$(grep -c "config_set.rs" docs/ARCHITECTURE.md)" -ge 1; test "$(grep -c "config-set" docs/CONFIGURATION.md)" -ge 1; test "$(grep -c "cmdline" docs/CONFIGURATION.md)" -ge 1; test "$(grep -c "config_set_roundtrip" docs/GSD-CORE-SYNC.md)" -ge 1; echo T3-OK'</automated>
    Gate baselines:
    - README "Called read-only": [HEAD=1 EXPECT==0]
    - README `config-set`: [HEAD=0 EXPECT>=1]
    - ARCHITECTURE `config_set.rs`: [HEAD=0 EXPECT>=1]
    - CONFIGURATION `config-set`: [HEAD=0 EXPECT>=1]
    - CONFIGURATION `cmdline`: [HEAD=0 EXPECT>=1]
    - GSD-CORE-SYNC `config_set_roundtrip`: [HEAD=0 EXPECT>=1]
  </verify>
  <done>README and docs describe the gsd-tools save path, the remaining direct defaults.json write and the argv exposure of secrets. The full suite (oracle tests included, not skipped), host/Windows/macOS clippy and fmt are clean. The SUMMARY lists the refused rows, the D-02 gap and I-kft-1..11. Committed as `docs(quick-261116-kft): document config saves through gsd-tools`. `git status` shows only this quick task's PLAN/SUMMARY, which are left for the orchestrator.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Config-tab input -> gsd-tools argv | Operator-typed values, plus keys from an untrusted project file, become argv of a child that writes `.planning/config.json`. |
| gsd-tools stdout/stderr -> status line and log | Child output can echo values, including secrets, and arbitrary text. |
| Registered project -> launcher resolution | A project may ship its own gsd-tools; the child runs with the TUI's environment. |
| Environment / `.planning/active-workstream` -> gsd-core path resolution | Selector variables and pointers decide which config.json gsd-core writes. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-kft-01 | Tampering | config_set.rs argv building (CWE-88) | high | mitigate | A value or key segment starting with `-` is refused before spawn. Every gsd-tools global flag (`--cwd`, `--project-dir`, `--ws`, `--pick`, `--default`, `--raw`, help tokens) starts with `-`. NUL and `.`-bearing segments are refused too. Unit-tested. |
| T-kft-02 | Tampering | gsd-core value coercion | high | mitigate | `cli_parse` replicates `parseConfigArgValue`. A Set whose parsed value differs from the intended one is refused (Coerced), so `12` never becomes a number in a string key, which would break the whole-file parse. The oracle conformance table pins the replica against real `--dry-run`. |
| T-kft-03 | Information disclosure | status line / tracing log | high | mitigate | For `is_secret_key` calls, the value is replaced by `<redacted>` in stderr/stdout before status or log. argv is never logged. Refusal texts and `ConfigSetCall` Debug carry no value. Tested with a fake child that echoes the secret. |
| T-kft-04 | Information disclosure | secret in child argv (`/proc/<pid>/cmdline`) | medium | accept | gsd-tools has no stdin form, and /gsd-settings passes keys the same way. The exposure lasts about 150 ms. Documented in docs/CONFIGURATION.md (I-kft-4). |
| T-kft-05 | Tampering | write lands in another config.json | high | mitigate | `ACK_ENV_REMOVE` scrubs `GSD_WORKSTREAM`, `GSD_PROJECT` and the session-key variables. The save is refused when `.planning/active-workstream` names a workstream. cwd is the project root that holds `.planning/`, so gsd-core neither walks up nor remaps a linked worktree. |
| T-kft-06 | Elevation of privilege | launcher resolution | high | mitigate | Only `resolve_user_gsd_tools()` (user installs and absolute PATH entries) is used, never a project-shipped gsd-tools (WR-05). Unit tests can never resolve the machine's install. |
| T-kft-07 | Denial of service | synchronous save in the key handler | low | mitigate | One total budget of 3 s per edit. The bounded runner kills an overrunning child, and the status says nothing is confirmed and asks for a reload. |
| T-kft-08 | Tampering | direct write of `$GSD_HOME/.gsd/defaults.json` | medium | accept | gsd-core has no writer for it (D-02). The installer-lock replication and atomic_write stay unchanged. The gap is documented in README/CONFIGURATION and reported to the user. |
| T-kft-SC | Tampering | npm/pip/cargo installs | low | accept | No package is installed and no Cargo dependency is added (`regex`, `serde_json` and `tempfile` are already dependencies). |
</threat_model>

<verification>
Full regression after the last commit. Use `rtk bypass` so rtk cannot filter the results.
- `rtk bypass cargo test --locked --no-fail-fast` exits 0. tests/config_set_roundtrip.rs and tests/owner_acknowledge_roundtrip.rs run against the installed gsd-core 1.21.0 and do not skip. Report the summed raw `test result:` counts.
- `rtk bypass cargo clippy --locked --all-targets -- -D warnings` exits 0, and so do the `x86_64-pc-windows-gnu` and `aarch64-apple-darwin` targets (release.yml cross-target-check).
- `rtk bypass cargo fmt --check` is clean.
- `git log --oneline -3` shows the three commits (feat, feat, docs), each made with a path-scoped `git commit -- <paths>`. `git status --short` lists nothing except this quick task's PLAN.md/SUMMARY.md, which the quick orchestrator commits in its docs step. Do not commit them in the executor. Nothing is pushed.
</verification>

<success_criteria>
- No code path in this build writes `.planning/config.json` or `.planning/.lock`. Project-scope Config saves run the user's gsd-tools `config-set` / `config-set-model-profile`, and the tab reflects disk afterwards.
- Coercing, flag-shaped and NUL values are refused before gsd-tools runs. Secrets never reach the status line or the log.
- The Global scope still works unchanged, and its direct write is documented as the remaining gap.
- The oracle round trip and the replica conformance table pass against the real gsd-tools. The full suite, the three clippy targets and fmt are clean.
</success_criteria>

<output>
Create `.planning/quick/261116-kft-route-config-json-writes-through-gsd-too/261116-kft-SUMMARY.md` when done. Record I-kft-1..11 (the [inferred] decisions), the "Rows gsd-core refuses" measurement, the D-02 defaults.json gap (gsd-core has no writer; the user decides whether to drop or keep the Global edit), the argv-exposure acceptance, and the raw test counts.
</output>
