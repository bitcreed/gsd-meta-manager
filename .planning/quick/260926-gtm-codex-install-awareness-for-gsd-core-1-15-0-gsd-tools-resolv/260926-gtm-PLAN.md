---
phase: quick-260926-gtm
plan: 01
type: execute
wave: 3
depends_on: ["260926-gtl"]
files_modified:
  - src/state_reader/queue_md.rs
  - tests/driver_router_conformance.rs
  - src/executor/codex.rs
  - tests/driver_codex_runtime.rs
  - tests/fixtures/fake-codex.sh
  - README.md
autonomous: true
requirements: [QUICK-260926-gtm]

must_haves:
  truths:
    - "On a machine whose only GSD install is a Codex one (`~/.codex/gsd-core`, or `$CODEX_HOME/gsd-core` when CODEX_HOME is set and non-empty), the TUI's gsd-tools resolver (src/state_reader/queue_md.rs) finds it and runs `smart-entry` through it instead of silently falling back to the keyword heuristic"
    - "Resolver order mirrors upstream 1.15.0 gsd-core/references/gsd-run-resolver.md: <root>/gsd-core, <root>/.claude/gsd-core, <root>/.codex/gsd-core, ~/.claude/gsd-core, ${CODEX_HOME:-~/.codex}/gsd-core, then `gsd-tools` on PATH; on a dual-install machine the Claude install still wins"
    - "tests/driver_router_conformance.rs resolves a Codex-only install as its oracle AND passes against it, because the oracle is invoked with GSD_RUNTIME=claude so it answers in the canonical `/gsd-…` spelling the rule table stores"
    - "An inherited GSD_RUNTIME in the test process (e.g. GSD_RUNTIME=codex) no longer breaks the conformance test (today it fails at the safe-alphabet assert because the oracle emits `$gsd-…`)"
    - "A driven `codex exec` child never inherits GSD_RUNTIME from the manager's environment (exact name only; GSD_RUNTIME_* siblings and other GSD_* variables are kept), per the scrub-not-set decision recorded in this plan"
    - "`rtk proxy cargo test --no-fail-fast` fails on nothing except the src/envelope/policy.rs git-version witness, and `rtk proxy cargo clippy --all-targets -- -D warnings` is clean"
  artifacts:
    - path: "src/state_reader/queue_md.rs"
      provides: "gsd_tools_candidates (pure) + resolve_gsd_tools_from (injectable home / CODEX_HOME) + resolve_gsd_tools wrapper, with tests for order, CODEX_HOME honouring and on-disk Codex-only resolution"
      contains: ".codex/gsd-core/bin/gsd-tools.cjs"
    - path: "tests/driver_router_conformance.rs"
      provides: "oracle_candidates (Claude then Codex home), oracle command pinned to GSD_RUNTIME=claude, updated SKIP/failure text, pure tests for both"
      contains: "GSD_RUNTIME"
    - path: "src/executor/codex.rs"
      provides: "scrubbed_from_codex_child drops exactly GSD_RUNTIME, doc comment carries the scrub-not-set justification"
      contains: "GSD_RUNTIME"
    - path: "tests/driver_codex_runtime.rs"
      provides: "PLANTED_ENV plants GSD_RUNTIME; env test asserts it is absent in the child with GSD_MM_ENVELOPE_ROOT as positive control"
      contains: "GSD_RUNTIME"
    - path: "tests/fixtures/fake-codex.sh"
      provides: "env log also records the NAMES of GSD_* variables"
      contains: "GSD_"
    - path: "README.md"
      provides: "Codex bullet names Codex-only install discovery and the GSD_RUNTIME scrub"
      contains: "CODEX_HOME"
  key_links:
    - from: "src/state_reader/queue_md.rs::suggest_next_commands"
      to: "resolve_gsd_tools -> resolve_gsd_tools_from(dirs::home_dir(), env CODEX_HOME)"
      via: "smart_entry_commands"
      pattern: "resolve_gsd_tools_from"
    - from: "tests/driver_router_conformance.rs::Oracle::recommended_action"
      to: "GSD's formatGsdSlash(resolveRuntime(cwd)) in init.manager (upstream src/init.cts:2865,3230-3252)"
      via: "GSD_RUNTIME=claude on the oracle child"
      pattern: "GSD_RUNTIME"
    - from: "src/executor/codex.rs::start_run env closure (lines ~213-232)"
      to: "scrubbed_from_codex_child"
      via: "scrub loop runs before envelope entries are applied"
      pattern: "scrubbed_from_codex_child"
---

<objective>
Make the manager aware of gsd-core 1.15.0 Codex installs and of 1.15.0's runtime-identity ladder, for this batch item only (catalog item D, quick id 260926-gtm).

1. Both gsd-tools resolvers (the TUI's production resolver in src/state_reader/queue_md.rs and the conformance-oracle resolver in tests/driver_router_conformance.rs) also find `${CODEX_HOME:-~/.codex}/gsd-core/bin/gsd-tools.cjs`, after the `.claude` locations. The production resolver also gains upstream's project-local `<root>/.codex/gsd-core/bin/gsd-tools.cjs` candidate.
2. The `codex exec` child no longer inherits `GSD_RUNTIME` (decision: SCRUB, justified below).
3. Tests cover both, and README states the user-visible change.

Purpose: 1.15.0's #4667 makes Codex installs self-contained under the Codex home (their @-includes now point at `~/.codex/gsd-core`, not `~/.claude/gsd-core`), so a Codex-only GSD user has no `~/.claude/gsd-core`. Today such a user gets keyword-heuristic suggestions instead of GSD's own `smart-entry`, and the conformance test fails as "oracle missing". 1.15.0's #4717 ranks `GSD_RUNTIME` above the per-install `.gsd-runtime` marker, so a value leaked from the launching shell overrides the identity of the GSD the driven Codex agent actually loads.

Output: the six files in `files_modified`, green gates, and a SUMMARY that carries the INFERRED decisions below.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@src/state_reader/queue_md.rs
@src/executor/codex.rs
@src/executor/runtime.rs

## Upstream facts (verified by the planner against `~/projects/node/gsd-core`, ref `upstream/release-1.15.0` = ec81d0d; do not check out branches there, use `git -C ~/projects/node/gsd-core show upstream/release-1.15.0:<path>`)

- **#4667 (58c7bbb16)**: the Codex installer rewrites `@~/.claude/gsd-core/...` includes to the Codex install root. After it, Codex agents load `~/.codex/gsd-core` (or `$CODEX_HOME/gsd-core`), and a Codex-only machine has no `~/.claude/gsd-core`.
- **Launcher order, 1.15.0** (`gsd-core/references/gsd-run-resolver.md`, reworked by #4834 b956bb7c6):
  1. Runtime-root candidates: `<root>/gsd-core/bin`, `<root>/.claude/gsd-core/bin`, `<root>/.codex/gsd-core/bin`.
  2. The `_gsd_homes` arm, which starts with `${CLAUDE_CONFIG_DIR:-$HOME/.claude}` and has `${CODEX_HOME:-$HOME/.codex}` fourth; it comes before the PATH arm.
  3. A PATH `gsd_run`, gated on `runtime-identity`.
  The shell `:-` means an EMPTY `CODEX_HOME` falls back to `$HOME/.codex`.
- **#4717 (9a41a9521)**: `fillRuntimeIdentity` in src/config-loader.cts fills an empty `config.runtime` from `GSD_RUNTIME`, then from the per-install `.gsd-runtime` marker. `resolveReportedRuntime` in src/host-runtime-detection.cts becomes explicit (GSD_RUNTIME > config.runtime) > install marker > host detection > claude. `resolveExplicitRuntime` in src/runtime-slash.cts:133-171 is `env.GSD_RUNTIME > config.runtime > null`. docs/how-to/control-the-reported-host-runtime.md says `GSD_RUNTIME` "outranks everything", and that a project's `runtime` is explicit intent GSD "does not second-guess". #4717 also states that project and workstream configs "are explicit operator intent and are never touched".
- **Command spelling follows runtime identity.** Measured by the planner with the local 1.14.0 installs: `~/.claude/gsd-core/.gsd-runtime` = `claude` and `~/.codex/gsd-core/.gsd-runtime` = `codex`. `query init.manager` on a one-phase fixture returns:
  - `/gsd-discuss-phase 1` for the Claude install, whether GSD_RUNTIME is unset or `claude`;
  - `$gsd-discuss-phase 1` for the Claude install under `GSD_RUNTIME=codex`;
  - `$gsd-discuss-phase 1` for the Codex install with GSD_RUNTIME unset, and `/gsd-…` under `GSD_RUNTIME=claude`.
  In 1.15.0 src/init.cts the runtime feeds only `formatGsdSlash` (lines 2865, 3230-3252), so pinning it changes spelling only, not routing.
- **`smart-entry` is runtime-independent.** 1.15.0 src/smart-entry.cts:632-647 hardcodes `/gsd:…`, and the planner measured identical output from both installs under every GSD_RUNTIME value. So the production resolver needs NO GSD_RUNTIME pin — do not add one.
- **Measured RED today:**
  - (a) With HOME set to a temp home containing only `.codex/gsd-core`, `the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state` fails as "oracle could not be resolved" (driver_router_conformance.rs:377).
  - (b) With the real HOME and `GSD_RUNTIME=codex` exported, the same test fails at the safe-alphabet assert (driver_router_conformance.rs:423), because the oracle emitted `$gsd-…`.

## GSD_RUNTIME decision: SCRUB from the codex child (INFERRED — human unavailable; record in SUMMARY for audit)

Chosen: remove an inherited `GSD_RUNTIME` from the `codex exec` child. Do NOT set it to `codex`. Reasons, in upstream-semantics order:

1. **An inherited value describes the launching shell or session, not the driven one.** That is exactly the ID-8 class `scrubbed_from_codex_child` already removes (CODEX_THREAD_ID, CODEX_SESSION_ID, …). A user who exports `GSD_RUNTIME=claude` globally (the upstream how-to's own remedy for a globally exported CODEX_HOME) would otherwise make every driven Codex child resolve Claude spelling, the Claude agents dir and the Claude tier map.
2. **Once it is removed, the child's own GSD answers from its own 1.15.0 ladder**, which yields codex without the manager asserting anything:
   - the project's explicit `.planning/config.json` `runtime`;
   - then the `.gsd-runtime` marker of the GSD install the Codex agent actually loads (post-#4667 that is the Codex install, marker `codex`);
   - then host detection. Per upstream docs, Codex injects `CODEX_SANDBOX_NETWORK_DISABLED` into the tool subprocesses of its network-off `workspace-write` sandbox, which is the sandbox this transport always uses. This comes from upstream docs and was not measured here.
3. **Setting `GSD_RUNTIME=codex` would mint a rung-1 identity that overrides an explicit project `runtime`.** #4717 deliberately never overrides that. It would also switch model-tier resolution, not just reporting.
4. **The manager's own ID-2** (src/executor/runtime.rs:103-111) keeps the manager's runtime choice decoupled from GSD's runtime keys. Writing `GSD_RUNTIME` into the child would be the inverse coupling.
5. **Reversible.** The envelope's entries are applied AFTER the scrub (codex.rs:216-232), so an explicit future envelope entry would still win.

The conformance ORACLE is the opposite case, and pinning `GSD_RUNTIME=claude` there is consistent with this decision. The oracle is a comparison harness asked for the canonical spelling the rule table stores. `runtime::codex_command` translates to `$gsd-…` only at the executor boundary (runtime.rs:133-134). The pin is spelling-only for `init.manager` (fact above).

## Other INFERRED scope calls (record in SUMMARY)

- **Honour `CODEX_HOME`** (non-empty) for the Codex home candidate in both resolvers. It mirrors upstream `${CODEX_HOME:-$HOME/.codex}`, where #4667's installer puts a Codex install.
- **Add the project-local `<root>/.codex/gsd-core` candidate** to the production resolver only. It is upstream's third runtime-root candidate. Conformance fixtures are temp trees with no project-local install, so that resolver keeps its home-only list.
- **Out of scope, observed, NOT fixed** (list as follow-ups in SUMMARY):
  - `CLAUDE_CONFIG_DIR` is not honoured for the Claude candidate (pre-existing).
  - The other 14 upstream runtime homes are not probed.
  - The PATH arm is not identity-gated (#4834).
  - src/executor/claude.rs scrubs only `CLAUDE*` (around line 504), so an inherited `GSD_RUNTIME=codex` still reaches Claude children. This is the mirror of this item, but the catalog scopes item D to the codex child.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: Production resolver finds a Codex-only GSD install end to end (queue_md.rs)</name>
  <files>src/state_reader/queue_md.rs</files>
  <read_first>src/state_reader/queue_md.rs lines 1-140 (GsdToolsCmd, resolve_gsd_tools, gsd_tools_on_path, smart_entry_commands) and 289-420 (existing test module style; tempfile is a regular dependency, see src/state_reader/backlog.rs:727 for TempDir use in lib tests)</read_first>
  <behavior>
    - gsd_tools_candidates(root, Some(home), None) returns exactly, in order: root/gsd-core/bin/gsd-tools.cjs, root/.claude/gsd-core/bin/gsd-tools.cjs, root/.codex/gsd-core/bin/gsd-tools.cjs, home/.claude/gsd-core/bin/gsd-tools.cjs, home/.codex/gsd-core/bin/gsd-tools.cjs
    - with codex_home = Some("/x/codex") the last candidate is /x/codex/gsd-core/bin/gsd-tools.cjs (replaces, not appends to, home/.codex)
    - with codex_home = Some("") the last candidate is home/.codex/... (mirrors shell `:-`)
    - with home = None and codex_home = None only the three project-local candidates are returned; with home = None and codex_home = Some("/x/codex") the codex candidate is still present and is last
    - on disk: a TempDir home containing ONLY .codex/gsd-core/bin/gsd-tools.cjs (a plain file) and an empty TempDir project root makes resolve_gsd_tools_from return program "node" with prefix_args == [that .codex path]
    - on disk: when the same home also contains .claude/gsd-core/bin/gsd-tools.cjs, the .claude path wins
    - on disk: a project-local <root>/.codex/gsd-core/bin/gsd-tools.cjs beats a home install
  </behavior>
  <action>
Split the resolver into three pieces, keeping behaviour identical for today's candidates.

1. Add a pure fn, `gsd_tools_candidates(project_root: &Path, home: Option<&Path>, codex_home: Option<&OsStr>) -> Vec<PathBuf>`. It builds the ordered list from the behavior block. The Codex home is the `codex_home` value when it is non-empty, otherwise `home.join(".codex")`, and the candidate is that path joined with `gsd-core/bin/gsd-tools.cjs`.
2. Add `resolve_gsd_tools_from(project_root: &Path, home: Option<&Path>, codex_home: Option<&OsStr>) -> Option<GsdToolsCmd>`. It walks those candidates with the existing is_file check and node program, then falls through to the unchanged `gsd_tools_on_path` arm.
3. Reduce `resolve_gsd_tools(project_root)` to a thin wrapper passing `dirs::home_dir()` and `std::env::var_os("CODEX_HOME")`. Do not read process env anywhere else, so tests never have to mutate HOME or CODEX_HOME in a shared test process.

Rewrite the doc comment on the resolver (currently lines 75-84):
- List the new order and cite upstream 1.15.0 gsd-core/references/gsd-run-resolver.md, #4667 and #4834.
- State the known deliberate differences: CLAUDE_CONFIG_DIR not honoured, the other runtime homes not probed, the PATH arm not identity-gated.
- State that no GSD_RUNTIME pin is applied to the `smart-entry` call because 1.15.0 src/smart-entry.cts hardcodes the `/gsd:` spelling regardless of runtime (so a future reader does not "fix" it).

Add the seven behavior-block tests in the existing `mod tests`:
- Use `tempfile::TempDir` and create the stub .cjs files with `std::fs::create_dir_all` + `std::fs::write`.
- Assert only positive resolutions. The PATH arm consults the host PATH, so never assert a `None`.
- Write the tests first and watch the new ones fail to compile or fail (RED), then implement (GREEN).

This is the tracer: it proves the production path (candidate list, real files on disk, resolved launcher) for a Codex-only install before the harness and env work build on it.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib state_reader::queue_md</automated>
  </verify>
  <done>All seven new tests pass alongside the existing queue_md tests. `resolve_gsd_tools` reads HOME and CODEX_HOME only through the wrapper, and a Codex-only temp home resolves to its .codex gsd-tools.cjs through node.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Conformance oracle resolves a Codex install and asks for the canonical spelling</name>
  <files>tests/driver_router_conformance.rs</files>
  <read_first>tests/driver_router_conformance.rs lines 40-181 (ALLOW_MISSING_ORACLE, skip_permitted, Oracle::resolve, Oracle::recommended_action) and 360-440 (the main test's missing-oracle assert text at ~385 and the SameCommand safe-alphabet assert at ~423)</read_first>
  <behavior>
    - oracle_candidates(Some(home), None) == [home/.claude/gsd-core/bin/gsd-tools.cjs, home/.codex/gsd-core/bin/gsd-tools.cjs] (Claude first)
    - oracle_candidates(Some(home), Some("/x/codex")) ends with /x/codex/gsd-core/bin/gsd-tools.cjs; Some("") falls back to home/.codex
    - the Command the oracle builds for a query carries GSD_RUNTIME=claude (checked via std::process::Command::get_envs, no oracle needed)
    - end to end: with HOME pointing at a temp home holding ONLY a Codex install (marker `codex`), the conformance test resolves it and PASSES
    - end to end: with the real HOME and GSD_RUNTIME=codex exported, the conformance test PASSES (was RED at the safe-alphabet assert)
  </behavior>
  <action>
Mirror Task 1's order inside the test file (an integration test cannot call the lib's private fn; keep the existing "mirrors queue_md.rs" doc claim true).

1. Add a pure `oracle_candidates(home: Option<PathBuf>, codex_home: Option<OsString>) -> Vec<PathBuf>` returning the `.claude` candidate, then the `${CODEX_HOME:-home/.codex}` candidate. Use the same non-empty rule as Task 1.
2. Make `Oracle::resolve` use it, reading `HOME` and `CODEX_HOME` via `std::env::var_os`.
3. Factor the command construction in `recommended_action` into a helper, e.g. `Oracle::query_command(&self, verb: &str) -> Command`, that sets `.env("GSD_RUNTIME", "claude")`. Give it a comment citing the planner's measurement:
   - the oracle's `init.manager` spells commands via `formatGsdSlash(resolveRuntime(cwd))` (upstream src/init.cts);
   - a Codex install's `.gsd-runtime` marker, or an inherited GSD_RUNTIME, yields `$gsd-…`;
   - the rule table stores canonical `/gsd-…` and `runtime::codex_command` translates only at the executor edge;
   - the pin is spelling-only for this verb.
4. Update the SKIP eprintln in `resolve` (~line 125) and the missing-oracle assert message (~line 385) so both name `~/.claude/gsd-core/bin/gsd-tools.cjs` AND `${CODEX_HOME:-~/.codex}/gsd-core/bin/gsd-tools.cjs`.
5. Add plain `#[test]` fns for the first three behavior bullets. Write them first (RED), then implement.

End-to-end verification uses a temp HOME. Rustup and cargo must keep their real homes, so pass CARGO_HOME and RUSTUP_HOME explicitly; the planner verified this runs without a rebuild. Build the Codex-only home as follows:
- Create a fresh temp dir (bash `mktemp -d`) and inside it the `.codex` dir.
- Symlink `$HOME/.codex/gsd-core` to `<tmp>/.codex/gsd-core`.
- If `~/.codex/gsd-core` does not exist, copy `~/.claude/gsd-core` there instead and write `codex` into the copy's `.gsd-runtime`, so the marker still says codex.
Before the fix, the first e2e run is RED ("could not be resolved"). After adding the candidate but before the pin, it is RED at the safe-alphabet assert with `$gsd-discuss-phase`. After the pin, it is GREEN. Record the three observations in the SUMMARY.
  </action>
  <verify>
    <automated>rtk proxy cargo test --test driver_router_conformance && FH=$(mktemp -d) && mkdir -p "$FH/.codex" && ln -s "$HOME/.codex/gsd-core" "$FH/.codex/gsd-core" && env -u GSD_META_MANAGER_ALLOW_MISSING_ORACLE -u CODEX_HOME -u GSD_RUNTIME HOME="$FH" CARGO_HOME="$HOME/.cargo" RUSTUP_HOME="$HOME/.rustup" rtk proxy cargo test --test driver_router_conformance && env -u GSD_META_MANAGER_ALLOW_MISSING_ORACLE GSD_RUNTIME=codex rtk proxy cargo test --test driver_router_conformance</automated>
  </verify>
  <precondition>node is on PATH and `~/.claude/gsd-core/bin/gsd-tools.cjs` and `~/.codex/gsd-core/bin/gsd-tools.cjs` exist (both true on the planning machine; if the Codex one is missing use the copy-and-mark fallback in the action)</precondition>
  <done>All three runs report the conformance test `ok`: real HOME, Codex-only temp HOME, and inherited GSD_RUNTIME=codex. None of them sets GSD_META_MANAGER_ALLOW_MISSING_ORACLE, so a pass means the oracle actually ran. The new pure tests pass.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Scrub inherited GSD_RUNTIME from the codex child, cover it end to end, document it, run the gates</name>
  <files>src/executor/codex.rs, tests/driver_codex_runtime.rs, tests/fixtures/fake-codex.sh, README.md</files>
  <read_first>src/executor/codex.rs lines 205-233 (env closure) and 337-352 (scrubbed_from_codex_child) and 615-633 (its unit test); tests/driver_codex_runtime.rs lines 45-70 (PLANTED_ENV, isolate_envelope_root) and 448-472 (the_codex_child_inherits_codex_home_and_no_other_agent_variable); tests/fixtures/fake-codex.sh (env logging line + header comment); README.md lines 30-34 (Initial Codex support bullet)</read_first>
  <behavior>
    - scrubbed_from_codex_child("GSD_RUNTIME") is true
    - scrubbed_from_codex_child is false for "GSD_RUNTIME_ROOT", "GSD_RUNTIMES", "GSD_TOOLS", "GSD_MM_ENVELOPE_ROOT", "PATH", "HOME", "CODEX_HOME" (exact-name scrub, no prefix match)
    - end to end: with GSD_RUNTIME planted in the test binary's env, the fake codex child's logged env names include GSD_MM_ENVELOPE_ROOT (positive control that GSD_* names are logged) and exclude GSD_RUNTIME
  </behavior>
  <action>
1. In `scrubbed_from_codex_child`, return true for the exact key `GSD_RUNTIME` before the existing CLAUDE/CODEX checks. Extend its doc comment with a compact form of the SCRUB justification from this plan's context:
   - the value describes the parent session;
   - 1.15.0 ranks it above config.runtime and the install marker (#4717);
   - removing it lets the child's own GSD resolve config.runtime, then marker, then host detection;
   - setting it to codex would override an explicit project runtime and couple the manager into GSD's ladder (ID-2);
   - envelope entries still apply after the scrub.
   Leave the scrub-then-envelope ordering in `start_run` untouched.
2. Add a dedicated unit test next to the existing scrub test covering the first two behavior bullets.
3. In tests/driver_codex_runtime.rs, add `("GSD_RUNTIME", "claude")` to `PLANTED_ENV` (bump the array length). In `the_codex_child_inherits_codex_home_and_no_other_agent_variable`, assert `GSD_RUNTIME` is absent and `GSD_MM_ENVELOPE_ROOT` (the value of `gsd_meta_manager::envelope::ENVELOPE_ROOT_ENV`, planted by `isolate_envelope_root`) is present. Reference the constant rather than the literal.
4. In tests/fixtures/fake-codex.sh, extend the env sed so it also logs the NAMES of variables matching `^GSD_`. Update the header comment that documents the `env` log file. Names only, never values, as today.
5. In README.md's "Initial Codex support" bullet (lines 30-34), add one short sentence:
   - next-command suggestions also use a Codex-only GSD install (`~/.codex/gsd-core`, or `$CODEX_HOME/gsd-core`);
   - a driven `codex exec` child does not inherit the launching shell's `GSD_RUNTIME`, so GSD inside it takes its runtime from the project's config and its own install.
   Keep the bullet's existing wrap style.
6. Run the full gates. Redirect raw output to a file under the executor's scratch directory and read that file (Read tool), never through a pipe: rtk filters downstream of `rtk proxy`, and a piped grep can fake a regression. Commands:
   - `rtk proxy cargo test --no-fail-fast`. The only permitted failure is the git-version witness in src/envelope/policy.rs (local git differs from the pinned 2.55.0); every other suite must report 0 failed. Compare the passed count with the pre-change baseline plus the new tests.
   - `rtk proxy cargo clippy --all-targets -- -D warnings`, which must be clean.
Write the SUMMARY with the INFERRED decisions (SCRUB rationale, CODEX_HOME honouring, project-local .codex candidate), the Task 2 RED/GREEN observations, and the out-of-scope follow-ups listed in context. Do not commit (the batch coordinator owns commits).
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib executor::codex && rtk proxy cargo test --test driver_codex_runtime && rtk proxy cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <done>
- The new unit test and the extended integration test pass.
- `rtk proxy cargo test --no-fail-fast` shows only the src/envelope/policy.rs git-version witness failing.
- clippy with `-D warnings` is clean.
- README's Codex bullet names Codex-only install discovery and the GSD_RUNTIME scrub.
- The SUMMARY records every INFERRED call.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| manager env -> codex child env | Variables inherited from whatever shell launched the TUI cross into the driven agent |
| filesystem -> spawned `node <gsd-tools.cjs>` | The resolver executes the first gsd-tools.cjs it finds under the project root or the user's homes |
| test process env -> conformance oracle | Inherited GSD_RUNTIME / HOME / CODEX_HOME steer which oracle runs and what it spells |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-gtm-01 | Spoofing | src/executor/codex.rs scrubbed_from_codex_child | medium | mitigate | An inherited GSD_RUNTIME would make the child's GSD claim a runtime identity it does not have (wrong agents dir, wrong tier map, wrong command spelling). Task 3 scrubs the exact name. Covered by a unit test and by the end-to-end env-name log in tests/driver_codex_runtime.rs |
| T-gtm-02 | Elevation of Privilege | src/state_reader/queue_md.rs new `<root>/.codex/gsd-core/bin/gsd-tools.cjs` candidate | medium | accept | Same trust level as the two existing project-local candidates (`<root>/gsd-core`, `<root>/.claude/gsd-core`), which already execute repo-shipped JS for a user-registered project on explicit user action. Upstream's own resolver runs the identical candidate. No new class of exposure |
| T-gtm-03 | Tampering | CODEX_HOME honoured by both resolvers | low | accept | CODEX_HOME is user-controlled environment, the same authority that already chooses PATH (the existing fallback arm). An empty value falls back to ~/.codex, mirroring upstream `:-` |
| T-gtm-04 | Information Disclosure | tests/fixtures/fake-codex.sh env log | low | mitigate | Task 3 logs variable NAMES only (sed captures the name group), never values, preserving the fixture's existing guarantee |
| T-gtm-05 | Tampering | tests/driver_router_conformance.rs oracle | low | mitigate | Task 2 pins GSD_RUNTIME=claude on the oracle child so an inherited value cannot silently change what the rule table is compared against |
</threat_model>

<verification>
- `rtk proxy cargo test --lib state_reader::queue_md`: resolver order, CODEX_HOME honouring and on-disk Codex-only resolution.
- `rtk proxy cargo test --test driver_router_conformance`, run three ways (real HOME; Codex-only temp HOME; `GSD_RUNTIME=codex`). The oracle runs and agrees each time, with no ALLOW_MISSING_ORACLE opt-out.
- `rtk proxy cargo test --lib executor::codex` and `rtk proxy cargo test --test driver_codex_runtime`: GSD_RUNTIME is scrubbed exactly, and GSD_* names are observably logged.
- `rtk proxy cargo test --no-fail-fast`: the only failure allowed is the src/envelope/policy.rs git-version witness. Read raw output from a redirected file, not a pipe.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean.
</verification>

<success_criteria>
- A Codex-only GSD install is found by the TUI's resolver and by the conformance oracle. On a dual install, `.claude` still wins.
- The conformance test is immune to an inherited GSD_RUNTIME and to a Codex-marked oracle.
- A driven codex child never inherits GSD_RUNTIME, and the SCRUB-over-SET decision is justified in code comments and in the SUMMARY (marked INFERRED).
- README states the user-visible change.
- Full test suite and clippy gates are green, apart from the known git-version witness.
</success_criteria>

<output>
Create `.planning/quick/260926-gtm-codex-install-awareness-for-gsd-core-1-15-0-gsd-tools-resolv/260926-gtm-SUMMARY.md` when done
</output>
