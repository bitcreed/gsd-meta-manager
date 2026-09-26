---
phase: quick-260926-fcp
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/browser.rs
  - src/project_creator.rs
  - tests/envelope_config_resolution.rs
  - tests/envelope_control_carrier.rs
  - tests/envelope_carrier_reach.rs
  - tests/envelope_wrapper_class.rs
autonomous: true
requirements: [QUICK-260926-fcp]

must_haves:
  truths:
    - "`cargo clippy -- -D warnings` (the release gate's command, scripts/pre-tag-check.sh GATE 5 and .github/workflows/release.yml Clippy step) still exits 0"
    - "`cargo clippy --all-targets -- -D warnings` exits 0 (it exits 101 today)"
    - "No test changes what it asserts: every rewritten assertion checks the same condition and keeps its original message"
    - "`cargo test --no-fail-fast` shows no new failures; the only acceptable failure is the known git-version witness `the_config_section_constants_record_the_git_version_they_were_derived_against` in src/envelope/policy.rs"
  artifacts:
    - path: "tests/envelope_wrapper_class.rs"
      provides: "4 lint sites rewritten (unused_mut, manual_ignore_case_cmp, chars_last_cmp, manual_contains)"
    - path: "src/browser.rs"
      provides: "3 bool_assert_comparison sites rewritten to assert!/assert!(!..)"
  key_links:
    - from: "scripts/pre-tag-check.sh GATE 5"
      to: "cargo clippy -- -D warnings"
      via: "unchanged command; must stay green"
---

<objective>
Clear every clippy error that `cargo clippy --all-targets -- -D warnings` reports, so that the
all-targets lint run is clean alongside the release gate's lib+bin run (which is already clean).

Purpose: the release gate is `cargo clippy -- -D warnings` and already passes; the all-targets
run fails with exit 101 on lints in test code and `#[cfg(test)]` modules. Clearing them keeps
lint debt from building up and lets anyone tighten the gate to `--all-targets` later without a
cleanup first. This plan does NOT change the gate command (out of scope).

Output: lint-only code edits in the 6 files above (plus any further files a later clippy round
surfaces), with no behavior change.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md

Established facts (orchestrator, live, 2026-09-26, clippy 0.1.98):
- `cargo clippy -- -D warnings` currently exits 0. `cargo clippy --all-targets -- -D warnings`
  exits 101.
- Rustc stops at the first error in each crate, and each integration-test file is its own crate,
  so more lints may appear once the first round is fixed. Iterate until the run is clean.
- Known initial lints (the line numbers are clippy's; they may drift by a line or two):
  - src/browser.rs:156-158 bool_assert_comparison x3 (`assert_eq!(entries[N].is_dir, true|false)`)
  - src/project_creator.rs:146 cmp_owned (`result != PathBuf::from("~")` in test_expand_tilde_home)
  - tests/envelope_config_resolution.rs:~2206 bool_assert_comparison (`assert_eq!(helper(..).0, false, "<long msg>")`)
  - tests/envelope_control_carrier.rs:978 single_element_loop (`for name in ["shred"] { assert!(..) }`)
  - tests/envelope_carrier_reach.rs:1711 useless_format (`format!("{ALIAS}")` in a tuple list)
  - tests/envelope_wrapper_class.rs:6127 unused_mut (`let mut push = |out: &mut Vec<..>, ..|`)
  - tests/envelope_wrapper_class.rs:6213 manual_ignore_case_cmp (`config_section(key).to_ascii_lowercase() == "alias"`)
  - tests/envelope_wrapper_class.rs:10795 chars_last_cmp (`word.text[..index].chars().next_back() != Some('=')`)
  - tests/envelope_wrapper_class.rs:11232 manual_contains (`.iter().any(|entry| *entry == "tar --create ...")`)

Tooling rules (from memory, they apply here):
- rtk filters build and test output, and a piped grep/awk is filtered too, even under `rtk proxy`.
  Never pipe clippy or test output into grep. Redirect the full output to a log file in your
  session scratchpad and use the Read tool on that file. In the verify commands below, `$SCRATCH`
  stands for that scratchpad directory. Replace it with the literal absolute path.
- Always run tests with `--no-fail-fast`. Otherwise one failing envelope suite hides the rest,
  and a green run with an unchanged pass count may mean suites never ran.
- The user's shell is fish. Chain commands with `&&` / `;` and do not rely on `$?`. Use a
  trailing `&& echo OK_MARKER` to observe success.
- Work directly on the current branch (master). No worktrees. Do not push.
</context>

<tasks>

<task type="auto">
  <name>Task 1: Rewrite the ten known lint sites, with no behavior change</name>
  <files>src/browser.rs, src/project_creator.rs, tests/envelope_config_resolution.rs, tests/envelope_control_carrier.rs, tests/envelope_carrier_reach.rs, tests/envelope_wrapper_class.rs</files>
  <read_first>Each lint site listed in the context block, with about 15 lines around it. Read only those ranges using offset/limit. The test files are 10k+ lines, so do not read them whole.</read_first>
  <action>
Apply the rewrite clippy suggests at each site. Keep every assertion message byte-identical, and
keep every condition logically identical:

- src/browser.rs test (bool_assert_comparison x3): `assert_eq!(X, true)` becomes `assert!(X)`,
  and `assert_eq!(X, false)` becomes `assert!(!X)`.
- src/project_creator.rs test_expand_tilde_home (cmp_owned): compare against a borrowed path
  instead of building a PathBuf, i.e. `result != Path::new("~")` or whatever form clippy suggests.
  Add `std::path::Path` to the test module's imports only if `use super::*` does not already
  bring it in. Leave the `|| dirs::home_dir().is_none()` arm alone.
- tests/envelope_config_resolution.rs (bool_assert_comparison): `assert_eq!(helper(..).0, false, MSG)`
  becomes `assert!(!helper(..).0, MSG)`, and the long multi-line message string literal stays
  exactly as it is. If the same pattern appears nearby, the iteration in Task 2 will flag it. Fix
  it the same way.
- tests/envelope_control_carrier.rs:978 (single_element_loop): replace the one-element
  `for name in ["shred"] { ... }` loop with a block that binds `let name = "shred";` and holds the
  same `assert!`. The `{name}` interpolations in the message stay unchanged. DECISION [inferred,
  for audit]: rewrite rather than `#[allow]`. The loop pins the one program name that
  `after_19_27_the_carrier_rule_reads_a_path_and_not_a_program_name` (line ~607) uses for a live
  carrier row, and that test uses only `shred` (line ~616). So this is not a list designed to
  grow.
- tests/envelope_carrier_reach.rs:1711 (useless_format): `format!("{ALIAS}")` becomes
  `ALIAS.to_string()`. The tuple element's type must stay `String` to match the sibling
  `format!(...)` entries.
- tests/envelope_wrapper_class.rs:
  - 6127 unused_mut: drop `mut` from the `push` closure binding.
  - 6213 manual_ignore_case_cmp: `config_section(key).eq_ignore_ascii_case("alias")`. This has
    the same ASCII semantics, and the doc comment above it about reading only the SECTION stays.
  - 10795 chars_last_cmp: `!word.text[..index].ends_with('=')`
  - 11232 manual_contains: `CONTROL_CARRIER_ORDINARY_OPERANDS.contains(&"tar --create --file /tmp/t -C/tmp/g")`,
    or the exact form clippy suggests for that slice's element type.

Do not touch production logic in src/envelope/ or anything outside these lint sites. Only use
`#[allow(clippy::...)]` for a genuine false positive, or where the suggested fix would change
behavior. In that case attach it to the narrowest item and add a one-line `// clippy: ...`
justification. Do not add a crate-level or file-level allow.
  </action>
  <verify>
    <automated>rtk proxy cargo clippy --all-targets -- -D warnings > "$SCRATCH/fcp-clippy-r1.log" 2>&1 && echo CLIPPY_ALL_OK   # then Read the log: none of the ten listed sites may appear in it (new sites are Task 2's job)</automated>
  </verify>
  <done>None of the ten listed sites shows up in the clippy log. Each rewrite keeps its original assertion message and condition.</done>
</task>

<task type="auto">
  <name>Task 2: Repeat clippy until --all-targets is clean, then prove both gates and the test suite</name>
  <files>Any further file the next clippy rounds flag (expected to be tests/*.rs or #[cfg(test)] modules in src/)</files>
  <action>
Run `rtk proxy cargo clippy --all-targets -- -D warnings` again, sending the output to a
scratchpad log, and Read the log. For each new error that surfaces now that rustc gets past the
earlier ones, apply the same rules as Task 1: use clippy's suggested rewrite, keep messages and
conditions identical, and allow only for genuine false positives or behavior-changing fixes, each
narrowly scoped with a one-line justification. Record every allow you add in the SUMMARY.
Repeat until the command exits 0.

Next, run both final gates, each redirected to its own log:
1. `rtk proxy cargo clippy -- -D warnings`. This is the release gate command and must still exit 0.
2. `rtk proxy cargo clippy --all-targets -- -D warnings`. Must exit 0.

Then run `rtk proxy cargo test --no-fail-fast`, sending the output to a log. Read the log's
per-suite `test result:` lines and the failures list. The only acceptable failure is
`the_config_section_constants_record_the_git_version_they_were_derived_against` in
src/envelope/policy.rs. It is environmental: local git differs from the pinned 2.55.0. Any other
failure is a regression from a rewrite. Find it and fix it. Record the suite count and the
passed/failed/ignored totals in the SUMMARY.

Commit on master with the message `chore(quick-260926-fcp): clear clippy --all-targets lints (no
behavior change)` and the attribution trailer. List each lint class and file in the body. Do not
push.
  </action>
  <verify>
    <automated>rtk proxy cargo clippy -- -D warnings > "$SCRATCH/fcp-clippy-gate.log" 2>&1 && echo CLIPPY_GATE_OK ; rtk proxy cargo clippy --all-targets -- -D warnings > "$SCRATCH/fcp-clippy-all.log" 2>&1 && echo CLIPPY_ALL_OK ; rtk proxy cargo test --no-fail-fast > "$SCRATCH/fcp-test.log" 2>&1 ; echo TEST_DONE   # both OK markers must print; Read fcp-test.log and confirm the only FAILED test is the git-version witness</automated>
  </verify>
  <done>Both clippy commands exit 0 (both OK markers print). The test log's only failure is the git-version witness. Pass count has not dropped against the pre-change baseline. The commit is on master and nothing was pushed.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| none new | Lint-only edits to test code and test modules; no input handling, I/O, or envelope policy logic changes |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-fcp-01 | Tampering | tests/envelope_*.rs guard assertions | medium | mitigate | A careless rewrite could weaken an envelope security assertion, e.g. by flipping a negation or dropping the `shred` absence check. Mitigation: every rewrite keeps its condition logically identical and its message byte-identical, and `cargo test --no-fail-fast` must show no new failures. |
| T-fcp-02 | Repudiation | blanket `#[allow]` hiding real lints | low | mitigate | No crate-level or file-level allows. Each item-level allow carries a one-line justification and is listed in the SUMMARY. |
</threat_model>

<verification>
- `rtk proxy cargo clippy -- -D warnings` exits 0 (release gate unchanged and green)
- `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0
- `rtk proxy cargo test --no-fail-fast`: the only failure is the known git-version witness
- `git diff --stat HEAD~1` touches only lint sites and changes no production logic under src/envelope/
</verification>

<success_criteria>
The all-targets clippy run is clean, the release-gate clippy run stays clean, and test behavior
is unchanged. Everything is in one atomic commit on master, not pushed.
</success_criteria>

<output>
Create `.planning/quick/260926-fcp-clear-pre-existing-clippy-errors-so-the-/260926-fcp-SUMMARY.md` when done. Include:
the lint classes and sites fixed across all iteration rounds, any `#[allow]` added with its
justification, the [inferred] single_element_loop decision, and the test totals.
</output>
