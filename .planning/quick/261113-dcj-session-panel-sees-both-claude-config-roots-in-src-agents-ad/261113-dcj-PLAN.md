---
phase: quick-261113-dcj
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/agents/adapters/claude.rs
  - tests/agents_claude.rs
autonomous: true
requirements: [QUICK-261113-dcj]

must_haves:
  truths:
    - "Take CLAUDE_CONFIG_DIR=<profile> (absolute) and a home of <home>, where <profile> is not <home>/.claude. The Claude Code adapter then reads sessions from BOTH <profile>/projects/<encoded>/... and <home>/.claude/projects/<encoded>/... in one scan. A worktree agent filed only under the home root is enriched (agentType, description, last activity, Live). A live worktree-less agent filed only under the profile root is listed. Both reach `scan_project_with`, which is what feeds the Sessions panel's Agents data."
    - "The same path is scanned once. Raw-equal roots resolve to one root, a trailing separator included. If CLAUDE_CONFIG_DIR is a symlink to <home>/.claude, or its `projects/` is a symlink to the home root's `projects/`, each project directory is still scanned once. That dedupe is by canonical path through the existing `same_path` helper, and the first spelling, CLAUDE_CONFIG_DIR's, is kept [inferred — audit: the dedupe point is the project directory, which covers the aliased-root case and also the aliased-`projects/` case]."
    - "A blank, whitespace-only or relative CLAUDE_CONFIG_DIR counts as unset, so only <home>/.claude is scanned. This is the existing `resolve_config_root` behaviour (T-25-12), reused unchanged. An absolute CLAUDE_CONFIG_DIR with no home scans only that root. With neither, nothing is scanned. There are never more than two roots."
    - "Every safety property holds under the second root, and a test proves it. The only content read is `*.meta.json` through `read_meta_capped`, capped at META_READ_CAP. A symlinked meta is refused. A symlinked session entry is skipped. A transcript is statted and never opened, even when its bytes look like a valid meta. The LIVE_SECS and MAX_AGENT_AGE_SECS bounds are unchanged."
    - "Tests inject CLAUDE_CONFIG_DIR and home through `ClaudeCodeAdapter::with_env`, with tempdir paths. No test calls the process-environment constructor, mutates the process environment or reads the real home directory."
    - "Gates: `rtk bypass cargo test --no-fail-fast` exits 0 with at least 3832 passed and 0 failed (HEAD: 3827 passed), read from a log file. `cargo fmt --all -- --check` exits 0. `cargo clippy --all-targets -- -D warnings` exits 0. For every release.yml matrix target, x86_64-pc-windows-gnu included, `cargo check --locked --target T --all-targets` and `cargo clippy --locked --target T --all-targets -- -D warnings` both exit 0."
  artifacts:
    - path: src/agents/adapters/claude.rs
      provides: "multi-root adapter: `config_roots` field, `with_env` injection seam, `resolve_config_roots` (reuses `resolve_config_root`), `project_dirs_across` with canonical dedupe; unit tests for resolution, blank/relative, aliasing and second-root safety"
      contains: "pub fn resolve_config_roots("
    - path: tests/agents_claude.rs
      provides: "end-to-end tracer: both roots reach `scan_project_with` through a real git worktree"
      contains: "both_claude_config_roots_reach_the_scan"
  key_links:
    - from: src/agents/adapters/mod.rs
      to: src/agents/adapters/claude.rs
      via: "registered_adapters -> ClaudeCodeAdapter::from_env -> Self::with_env(CLAUDE_CONFIG_DIR, home) -> resolve_config_roots"
      pattern: "Self::with_env("
    - from: tests/agents_claude.rs
      to: src/agents/adapters/claude.rs
      via: "ClaudeCodeAdapter::with_env(profile, home) fed to scan_project_with"
      pattern: "with_env("
---

<objective>
Make the Claude Code session adapter read sessions from both Claude config roots: the `$CLAUDE_CONFIG_DIR` root and `<home>/.claude`. Each project directory is scanned once even when the two roots alias each other, and every existing safety property of the adapter stays.

Purpose: under a claude-profile setup (`CLAUDE_CONFIG_DIR=~/.local/share/claude-profiles/<name>` with its own real `projects/` directory), today the adapter sees only the root this process was started under. The other account's agents are then missing from the Sessions panel. This machine runs that setup: `~/.local/share/claude-profiles/player-health/projects` is a real directory, separate from `~/.claude/projects`.

Output:
- `src/agents/adapters/claude.rs`: multi-root resolution, an injected-environment constructor, canonical dedupe and updated module docs, plus unit tests.
- `tests/agents_claude.rs`: one end-to-end tracer test.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@.planning/STATE.md
@src/agents/adapters/claude.rs
@src/agents/adapters/mod.rs
@tests/agents_claude.rs

Interfaces the executor works against (measured at HEAD 7c857d5):

- `src/agents/adapters/claude.rs`
  - `ClaudeCodeAdapter` derives `Debug, Clone, PartialEq, Eq`. It has ONE private field today, a single optional root (L69-73).
  - `ClaudeCodeAdapter::new(config_root: PathBuf) -> Self` (L78) is used by every existing in-source test and by `tests/agents_claude.rs`. Its signature stays.
  - The process-environment constructor (L86-93) passes the process's `CLAUDE_CONFIG_DIR` and the `dirs` crate's home directory to `resolve_config_root`.
  - `pub fn resolve_config_root(env_value: Option<OsString>, home: Option<PathBuf>) -> Option<PathBuf>` (L104-112): an env value wins only when it is non-empty and absolute; otherwise `<home>/.claude`. It stays unchanged, along with its test `a_relative_or_empty_config_dir_falls_back_to_home`.
  - `fn same_path(a, b) -> bool` (L244): equal raw, or equal once both are canonicalized. A path that cannot be canonicalized matches only raw.
  - `fn project_dirs(root, snap) -> Vec<PathBuf>` (L282) handles ONE root. `fn session_dirs(&[PathBuf])` (L311) sorts the union and skips symlinked entries. `enrich` (L584-610) handles one root.
  - `read_meta_capped` (L215) is the only content read. `place_live_subagents` keeps a `seen` id set, so a duplicated session never duplicates an agent; dedupe exists for scan cost and for the "scans once" requirement.
  - The test module starts at the column-0 line `mod tests {` (L617). Its `Env` fixture (L627-695) has `root = <base>/claude`, `subagents` under it, `write_meta_bytes` and `write_meta` (L668-675), `transcript(id, now, age_secs)` (L678) and `enrich(worktrees, now)`, which uses `new(self.root)`.
- `src/agents/adapters/mod.rs` L110-115: `registered_adapters()` builds the adapter through the process-environment constructor. `scan_projects_guarded` calls it once per scan (src/agents/mod.rs:640).
- `tests/agents_claude.rs`:
  - `claude_fixture() -> Option<ClaudeFixture>` (L131) returns `None` when `git init` is forbidden, and the test returns early. Fields: `work`, `root` (= `<base>/claude`), `subagents`.
  - Helpers: `add_agent_worktree(id, lock_pid)`, `write_meta_in(dir, id, &Value)`, `write_transcript_in(dir, id, SystemTime)`, `executor_meta(&wt)` (L217), `loose_meta(type, desc)`, `loose(&scan)`, `only_row(&scan)` and `shown`, plus the constants `AGENT_ID` and `SESSION`.
  - `scan_project_with(&work, &[Box<dyn AgentAdapter>], now)`.
- `crate::agents::worktrees::valid_agent_id`: 1-64 bytes of `[A-Za-z0-9_-]`. Every test agent id must pass it.

Baselines (HEAD):
- `cargo test --lib agents::adapters::claude::` gives 12 passed.
- `cargo test --test agents_claude` gives 13 passed.
- The full suite gives 3827 passed, 0 failed.
- Installed targets: aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-gnu, x86_64-unknown-linux-gnu. The release.yml matrix lists the first three.

Sibling batch items own other files. 261113-dck owns `src/launcher/*` and `src/oneoff/*`. 261113-dcl owns `tests/driver_router_conformance.rs` and `scripts/install-conformance-oracle.sh`. Touch none of them.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: End-to-end "both Claude config roots reach the agents scan" (one path only)</name>
  <files>src/agents/adapters/claude.rs, tests/agents_claude.rs</files>
  <behavior>
    - `resolve_config_roots_lists_the_env_root_then_the_home_root` (in-source, pure values, no filesystem):
      - `(Some("/profiles/a"), Some("/home/someone"))` gives `["/profiles/a", "/home/someone/.claude"]`, in that order.
      - `(Some("/home/someone/.claude"), Some("/home/someone"))` and `(Some("/home/someone/.claude/"), Some("/home/someone"))` each give `["/home/someone/.claude"]`, one entry.
      - `(None, Some("/home/someone"))` gives `["/home/someone/.claude"]`.
      - `(Some("/abs"), None)` gives `["/abs"]`.
      - `(Some("relative"), None)` and `(None, None)` give `[]`.
    - `both_claude_config_roots_reach_the_scan` (tests/agents_claude.rs, real git worktree):
      - The profile root is `fx.root`; home is `<base>/home`.
      - A worktree agent's meta lives only in the home root. `scan_project_with` over `ClaudeCodeAdapter::with_env(Some(profile), Some(home))` enriches that row: agentType `gsd-executor`, adapter `claude-code`, Live.
      - A live worktree-less researcher lives only in the profile root, and the same scan lists it.
    - Control, same test (non-vacuity, and the bug as it stands):
      - `ClaudeCodeAdapter::new(profile)` alone leaves the row unenriched (`agent_type` None) but lists the researcher.
      - `ClaudeCodeAdapter::new(<home>/.claude)` alone enriches the row but lists no worktree-less agent.
  </behavior>
  <action>
RED first:
- Write both tests named in `<behavior>`.
- Run them and record that they fail. Here they fail to compile, because the constructor and resolver do not exist yet.
- Commit path-scoped: `test(quick-261113-dcj): both Claude config roots reach the agents scan`.

GREEN: make these changes in `src/agents/adapters/claude.rs`.

1. Replace the adapter's single optional root field with a field named `config_roots`, of type `Vec<PathBuf>`.
   - An empty list means nothing to scan, the role the old "no root" state had.
   - Field doc: the CLAUDE_CONFIG_DIR root first, then `<home>/.claude`, at most two.
   - `new(config_root: PathBuf)` keeps its signature and stores a one-element list. Every existing test keeps using it unchanged.
2. Add a public associated function `with_env`, with parameters `claude_config_dir: Option<OsString>` and `home: Option<PathBuf>`, returning `Self`.
   - This is the injected-environment seam. It only resolves paths and reads no file.
   - The existing process-environment constructor `from_env` becomes a one-line delegation to `with_env` (called as `Self::with_env`), passing exactly what it passes today: the process's `CLAUDE_CONFIG_DIR` and the `dirs` crate's home directory.
   - That constructor stays the module's only process-environment read.
3. Add a public free function `resolve_config_roots`, with parameters `env_value: Option<OsString>` and `home: Option<PathBuf>`, returning `Vec<PathBuf>`. Build it entirely from the existing resolver.
   - First entry: `resolve_config_root(env_value, home.clone())`. A blank, whitespace-only or relative value already falls through to the home root there (T-25-12).
   - Second entry: `resolve_config_root(None, home)`, appended only when it differs from the first. `Path` equality is component-wise, so a trailing separator counts as the same path.
   - This function stays pure: no canonicalize, no stat.
   - Do NOT change `resolve_config_root` or its test.
   - Do NOT use `InstallRoots::claude_home`. It expands `~` and accepts relative values, which contradicts the T-25-12 semantics this task mandates [inferred — audit].
4. Add a private function `project_dirs_across`, with parameters `roots: &[PathBuf]` and `snap: &CoreSnapshot<'_>`, returning `Vec<PathBuf>`.
   - It calls the existing per-root `project_dirs` for each root in order and concatenates the results.
   - For now it skips a directory only when it is raw-equal to one already kept. Task 2 makes this canonical.
   - `enrich` uses it in place of the single-root lookup and returns the default report when the list is empty.
   - Everything after that point is untouched and runs once over the merged session list: `session_dirs`, `enrich_by_id`, `place_live_subagents`. `session_dirs` already sorts the union, so scan order stays deterministic.
5. Update the docs to the new behaviour.
   - Module doc, "Where the files are": both roots, and why. Claude-profile setups point CLAUDE_CONFIG_DIR at a profile directory with its own `projects/`, while the main account keeps `<home>/.claude`.
   - "Only the user's own config root is read" becomes plural, still never a `.claude/` inside a project.
   - Update the struct and constructor docs.
   - No README or docs/ file describes this read path, so none changes.

In `tests/agents_claude.rs`, write the tracer test:
- Home is `fx.root.parent()` joined with `home`: a tempdir path, never the real home.
- The home root's session `subagents` directory is `<home>/.claude/projects/<encode_project_dir(work)>/<a second session uuid literal>/subagents`.
- Set up with `fx.add_agent_worktree(AGENT_ID, Some(1))`.
- Write `executor_meta(&wt)` and a 10 s transcript into the home root via `write_meta_in` and `write_transcript_in`.
- Write a `loose_meta("gsd-phase-researcher", "Research phase 26")` and a 30 s transcript into the profile root via `fx.write_meta` and `fx.write_transcript`.
- Build the adapter with `with_env(Some(fx.root.clone().into_os_string()), Some(home))` and run `scan_project_with` once. Assert with `only_row`, `shown` and `loose`. Then run both controls.
- Update the suite header comment to say that tests either root the adapter at a tempdir with `new` or inject CLAUDE_CONFIG_DIR and home with `with_env`.

Run the tests and confirm they pass. Commit path-scoped: `fix(quick-261113-dcj): read Claude sessions from both the CLAUDE_CONFIG_DIR root and ~/.claude`.

Never mutate the process environment in a test.
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail() { echo "FAIL-$1"; exit 1; }; L=target/dcj-t1.log; cnt() { sed -nE "s/^test result: ok\. ([0-9]+) passed; 0 failed.*/\1/p" "$L"; }; cargo test --lib resolve_config_roots_lists_the_env_root_then_the_home_root > "$L" 2>&1 || fail resolve-run; [ "$(cnt)" = 1 ] || fail resolve-count; cargo test --lib agents::adapters::claude:: > "$L" 2>&1 || fail lib-run; n=$(cnt); [ -n "$n" ] && [ "$n" -ge 13 ] || fail lib-count; cargo test --test agents_claude both_claude_config_roots_reach_the_scan > "$L" 2>&1 || fail tracer-run; [ "$(cnt)" = 1 ] || fail tracer-count; cargo test --test agents_claude > "$L" 2>&1 || fail int-run; n=$(cnt); [ -n "$n" ] && [ "$n" -ge 14 ] || fail int-count; F=src/agents/adapters/claude.rs; P() { awk "/^mod tests \\{/{exit} {print}" "$F" | grep -v "^[[:space:]]*//"; }; [ "$(P | grep -cF "pub fn with_env(")" = 1 ] || fail with-env; [ "$(P | grep -cF "pub fn resolve_config_roots(")" = 1 ] || fail resolve-fn; [ "$(P | grep -cF "config_roots: Vec<PathBuf>")" -ge 1 ] || fail roots-field; [ "$(P | grep -cF "Self::with_env(")" = 1 ] || fail from-env-delegates; [ "$(P | grep -cF "fn project_dirs_across(")" = 1 ] || fail across-fn; [ "$(P | grep -cF "config_root: Option<PathBuf>")" = 0 ] || fail old-field; echo verify-ok'</automated>
    Gate baselines, production region only (comment lines dropped):
    - claude lib suite [HEAD=12 EXPECT>=13]
    - agents_claude suite [HEAD=13 EXPECT>=14]
    - `pub fn with_env(` [HEAD=0 EXPECT==1]
    - `pub fn resolve_config_roots(` [HEAD=0 EXPECT==1]
    - `config_roots: Vec<PathBuf>` [HEAD=0 EXPECT>=1]
    - `Self::with_env(` [HEAD=0 EXPECT==1]
    - `fn project_dirs_across(` [HEAD=0 EXPECT==1]
    - the old single-root field declaration [HEAD=1 EXPECT==0]
  </verify>
  <done>Both tests were committed red and then pass. One `scan_project_with` call shows an agent from each root. The controls prove that a single root misses the other account's agent. The module docs describe both roots.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Same path scans once; blank/relative env and every safety property hold under the second root</name>
  <files>src/agents/adapters/claude.rs</files>
  <behavior>
    - `a_config_dir_aliasing_the_home_root_scans_once` (`#[cfg(unix)]`):
      - Case A: CLAUDE_CONFIG_DIR is a symlink to `<home>/.claude`, which holds one live worktree-less agent.
        - `config_roots` has 2 entries, because the raw spellings differ.
        - `project_dirs_across(&adapter.config_roots, &snap)` returns exactly 1 directory, and it starts with the CLAUDE_CONFIG_DIR spelling.
        - `enrich` lists the agent once.
      - Case B: CLAUDE_CONFIG_DIR is a real directory whose `projects` is a symlink to `<home>/.claude/projects`. `project_dirs_across` again returns exactly 1.
      - Against Task 1's raw dedupe both cases return 2, which is the RED.
    - `a_blank_or_relative_config_dir_scans_only_the_home_root` (every platform, no symlinks):
      - For each of `""`, `"   "` and `"rel/dir"` with home `<base>/home`, `config_roots == [<base>/home/.claude]`, and `enrich` lists the home root's one live agent.
      - Control: an absolute profile value gives 2 roots.
    - `the_home_root_keeps_every_safety_property` (`#[cfg(unix)]`). CLAUDE_CONFIG_DIR is `env.root`; the home root is the SECOND root. Only the control is reported:
      - a control live agent in the home root IS listed;
      - an `agent-<id>.meta.json` that is a symlink to a valid meta outside the roots, with a fresh transcript, is NOT;
      - a session entry that is a symlink to an outside directory holding a valid meta and a fresh transcript is NOT;
      - a meta of META_READ_CAP + 1 bytes with a fresh transcript is NOT;
      - a worktree agent with only a transcript, whose bytes are a valid-looking meta, gets no enrichment;
      - `worktreeless` is exactly the control, and `per_worktree` is empty.
  </behavior>
  <action>
RED first:
- Add the three tests named in `<behavior>` to the `mod tests` block of `src/agents/adapters/claude.rs`.
- Run them. The aliasing test must fail against Task 1's raw dedupe. Record that.
- Commit path-scoped: `test(quick-261113-dcj): same path scans once, blank/relative env, second-root safety`.

GREEN:
- In `project_dirs_across`, replace the raw-equality skip with the existing `same_path` helper. A directory is kept unless it is the same path as one already kept, raw or canonical. A directory that cannot be canonicalized matches only raw.
- Keep the first spelling seen, which is CLAUDE_CONFIG_DIR's because that root comes first.
- This is the one dedupe point. It collapses an aliased root and also a profile whose `projects/` is symlinked to the main account's, so each session directory is scanned once [inferred — audit].
- Do NOT canonicalize in `resolve_config_roots`, `with_env` or the process-environment constructor. Those stay pure path resolution, as their docs promise.
- Relax no existing check:
  - `session_dirs` still skips symlinked session entries (`DirEntry::file_type` does not follow links).
  - `read_meta_capped` still refuses non-`.meta.json` names, symlinks, and anything over META_READ_CAP.
  - Transcripts are only statted.
  - The LIVE_SECS and MAX_AGENT_AGE_SECS bounds are unchanged.
  - A symlinked config root or `projects/` directory is still followed, as it already was for the single root: it is the user's own configuration [inferred — audit]. Note this in the `project_dirs_across` doc.
- Commit path-scoped: `fix(quick-261113-dcj): scan each Claude project directory once across aliased config roots`.

Test helpers:
- Extend `Env` with helpers parameterized by root or directory. For example:
  - a helper that creates `<root>/projects/<encode_project_dir(project)>/<SESSION>/subagents` for any root and returns it;
  - meta and transcript writers that take that directory;
  - an enrich that takes a prepared adapter.
- Every agent id must pass `valid_agent_id`, so use ASCII alphanumerics, for example `acontrol0000000`.
- Build all paths under the tempdir, with home at `<base>/home`.
- Build adapters with `with_env` only. Never use the process-environment constructor, never read the real home, and never mutate the process environment.
- Windows dead-code trap: release.yml runs `cargo clippy --locked --target x86_64-pc-windows-gnu --all-targets -- -D warnings`, where `#[cfg(unix)]` tests do not exist. That exact lint killed v2.2.0 and v2.3.0. So every new helper must be called by the all-platform blank/relative test, or carry `#[cfg(unix)]` itself.

Run the full gate set below, then `cargo fmt --all`. Commit any formatting change path-scoped.
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail() { echo "FAIL-$1"; exit 1; }; L=target/dcj-t2.log; cnt() { sed -nE "s/^test result: ok\. ([0-9]+) passed; 0 failed.*/\1/p" "$L"; }; for t in a_config_dir_aliasing_the_home_root_scans_once a_blank_or_relative_config_dir_scans_only_the_home_root the_home_root_keeps_every_safety_property resolve_config_roots_lists_the_env_root_then_the_home_root a_relative_or_empty_config_dir_falls_back_to_home; do cargo test --lib "$t" > "$L" 2>&1 || fail "run-$t"; [ "$(cnt)" = 1 ] || fail "count-$t"; done; cargo test --lib agents::adapters::claude:: > "$L" 2>&1 || fail lib-run; n=$(cnt); [ -n "$n" ] && [ "$n" -ge 16 ] || fail lib-count; cargo test --test agents_claude > "$L" 2>&1 || fail int-run; n=$(cnt); [ -n "$n" ] && [ "$n" -ge 14 ] || fail int-count; F=src/agents/adapters/claude.rs; I=tests/agents_claude.rs; P() { awk "/^mod tests \\{/{exit} {print}" "$F" | grep -v "^[[:space:]]*//"; }; T() { awk "/^mod tests \\{/{f=1} f" "$F" | grep -v "^[[:space:]]*//"; }; [ "$(P | awk "/fn project_dirs_across\\(/,/^}/" | grep -cF "same_path(")" -ge 1 ] || fail canonical-dedupe; [ "$(P | grep -cF "from_env()")" = 1 ] || fail env-ctor-defined-once; [ "$(T | grep -cF "from_env()")" = 0 ] || fail unit-test-uses-env-ctor; [ "$(T | grep -cF "home_dir")" = 0 ] || fail unit-test-reads-home; [ "$(grep -v "^[[:space:]]*//" "$I" | grep -cF "home_dir")" = 0 ] || fail int-test-reads-home; [ "$(cat "$F" "$I" | grep -v "^[[:space:]]*//" | grep -cE "set_var|remove_var")" = 0 ] || fail env-mutation; cargo fmt --all -- --check > "$L" 2>&1 || fail fmt; cargo clippy --all-targets -- -D warnings > "$L" 2>&1 || fail clippy-host; TS=$(grep -E "^ +- target: " .github/workflows/release.yml | awk "{print \$3}"); [ -n "$TS" ] || fail no-matrix-targets; for t in $TS; do cargo check --locked --target "$t" --all-targets > "$L" 2>&1 || fail "check-$t"; cargo clippy --locked --target "$t" --all-targets -- -D warnings > "$L" 2>&1 || fail "clippy-$t"; done; FL=target/dcj-full-tests.log; cargo test --no-fail-fast > "$FL" 2>&1; rc=$?; PS=$(awk "/^test result:/ {p+=\$4} END {print p+0}" "$FL"); X=$(awk "/^test result:/ {f+=\$6} END {print f+0}" "$FL"); echo "full-suite exit=$rc passed=$PS failed=$X"; [ "$rc" -eq 0 ] || fail full-exit; [ "$X" -eq 0 ] || fail full-failed; [ "$PS" -ge 3832 ] || fail full-count; echo verify-ok'</automated>
    Gate baselines (comment lines dropped; P = production region, T = `mod tests` region):
    - claude lib suite [HEAD=12 EXPECT>=16]
    - agents_claude suite [HEAD=13 EXPECT>=14]
    - full suite passed [HEAD=3827 EXPECT>=3832 @7c857d5]
    - full suite failed [HEAD=0 EXPECT==0 @7c857d5 GUARD MUTATION: any test this change breaks, e.g. an existing ClaudeCodeAdapter::new caller]
    - `same_path(` inside `project_dirs_across` [HEAD=0 EXPECT>=1]
    Invariant guards, which already hold at HEAD and must not regress (each names the mutation that turns it red):
    - P `from_env()`, the definition only [HEAD=1 EXPECT==1 @7c857d5 GUARD MUTATION: a second production call site of the process-environment constructor inside claude.rs]
    - T `from_env()` [HEAD=0 EXPECT==0 @7c857d5 GUARD MUTATION: a unit test building its adapter through the process-environment constructor instead of with_env]
    - T `home_dir` [HEAD=0 EXPECT==0 @7c857d5 GUARD MUTATION: a unit test deriving its home from the real home directory instead of the tempdir]
    - tests/agents_claude.rs `home_dir` [HEAD=0 EXPECT==0 @7c857d5 GUARD MUTATION: the tracer test deriving home from the real home directory instead of fx.root's parent]
    - env mutation in both files [HEAD=0 EXPECT==0 @7c857d5 GUARD MUTATION: a test that sets or removes CLAUDE_CONFIG_DIR in the process environment]
  </verify>
  <done>Each project directory is scanned once across aliased roots, and the aliasing test went red and then green. Blank, whitespace-only or relative CLAUDE_CONFIG_DIR scans only the home root. The second root keeps every safety property, and a non-vacuous test with a control proves it. fmt, host clippy, and per-target check and clippy for every release.yml target are clean. The full suite passes with at least 3832 tests.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Claude session trees -> adapter | `<CLAUDE_CONFIG_DIR>/projects/...` and `<home>/.claude/projects/...` are written by Claude Code and by anything else running as the user. Meta content is untrusted: it is read only through `read_meta_capped` and shown through `Untrusted`. This change adds a second tree of the same trust level, not a new kind of input. |
| process environment -> adapter | `CLAUDE_CONFIG_DIR` is user-controlled. A blank or relative value must never resolve against the process's cwd, which could be a project carrying its own `.claude/`. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-261113-dcj-01 | Information disclosure | transcripts under the second root | medium | mitigate | `read_meta_capped` stays the only content read. A transcript is statted, never opened. `the_home_root_keeps_every_safety_property` plants a valid-looking meta body inside a `.jsonl` under the home root and asserts nothing from it surfaces. |
| T-261113-dcj-02 | Tampering | symlinked meta or session entry in the second root pointing outside it | medium | mitigate | `read_meta_capped`'s symlink refusal and `session_dirs`' non-following `file_type` are unchanged. The safety test plants both under the home root, beside a control that is reported. |
| T-261113-dcj-03 | Denial of service | scan cost across two roots; oversize meta | low | mitigate | `resolve_config_roots` builds at most two roots. Canonical dedupe in `project_dirs_across` means each project directory is scanned once. META_READ_CAP and the LIVE_SECS and MAX_AGENT_AGE_SECS bounds are unchanged. The safety test asserts that an over-cap meta in the home root is no data. |
| T-261113-dcj-04 | Spoofing | blank, whitespace-only or relative CLAUDE_CONFIG_DIR resolved against cwd | medium | mitigate | `resolve_config_root` is reused unchanged (T-25-12). `a_blank_or_relative_config_dir_scans_only_the_home_root` and `resolve_config_roots_lists_the_env_root_then_the_home_root` pin it. |
| T-261113-dcj-05 | Information disclosure | the other account's agent types and descriptions now appear in the Sessions panel | low | accept | Both roots belong to the same OS user and are that user's own configuration. Showing them is the requested behaviour. The fields shown are unchanged, and they are rendered through `Untrusted`. |
| T-261113-dcj-06 | Tampering | tests touching the real home or racing on the process environment | low | mitigate | `with_env` injection plus tempdirs. Invariant gates in Task 2 cover: no environment mutation; no home lookup in the unit-test region or in tests/agents_claude.rs; the process-environment constructor is defined once and called by no test. |
| T-261113-dcj-SC | Tampering | npm/pip/cargo installs | low | accept | No dependency is added. Every cross-target gate runs `--locked`. |
</threat_model>

<verification>
- Task 1's `<automated>` prints `verify-ok`: the end-to-end tracer plus resolution, with the structural gates.
- Task 2's `<automated>` prints `verify-ok`: aliasing, blank/relative and second-root safety, then fmt, host clippy, per-target check and clippy from release.yml, and the full suite at 3832 or more passed with 0 failed, read from `target/dcj-full-tests.log`.
- `git log --oneline -6` shows the test-then-fix commit pairs for both tasks, each scoped to `src/agents/adapters/claude.rs` and/or `tests/agents_claude.rs`. `git log -1 --stat` names only those paths.
</verification>

<success_criteria>
- With CLAUDE_CONFIG_DIR set to a separate profile directory, agents filed under either the profile root or `<home>/.claude` reach the agents scan the Sessions panel renders.
- An aliased root, or an aliased `projects/`, is scanned once.
- Blank, whitespace-only or relative CLAUDE_CONFIG_DIR behaves as unset.
- Every adapter safety property is proven under the second root.
- No test reads the real home or mutates the process environment.
- `git status --porcelain -- src/agents/adapters/claude.rs tests/agents_claude.rs` is empty after the final commit, and no sibling item's files are touched.
</success_criteria>

<output>
Create `.planning/quick/261113-dcj-session-panel-sees-both-claude-config-roots-in-src-agents-ad/261113-dcj-SUMMARY.md` when done
</output>
