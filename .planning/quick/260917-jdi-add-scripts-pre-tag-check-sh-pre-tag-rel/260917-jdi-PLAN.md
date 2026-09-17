---
phase: quick-260917-jdi
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - scripts/pre-tag-check.sh
  - CLAUDE.md
autonomous: true
requirements: [QUICK-260917-jdi]

estimate:
  tokens: 45000
  raw_tokens: 45000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Invoking the script with a tag whose version disagrees with Cargo.toml exits non-zero and stops before any compilation happens."
    - "Invoking the script with no argument reports the crate version and does not fail on that account."
    - "Every run prints a loud reconciliation block comparing the installed git against the constant READ OUT OF src/envelope/policy.rs at runtime, and repeats that comparison in the final summary."
    - "A git/constant MISMATCH is an advisory: it never by itself changes the script's exit code, which reflects only real gate failures."
    - "If the constant cannot be extracted from src/envelope/policy.rs, the script hard-fails instead of silently continuing."
    - "The MSRV gate never installs a toolchain; when the declared floor is not installed locally it prints an explicit SKIPPED line."
    - "CLAUDE.md release step 3 invokes scripts/pre-tag-check.sh and states the git-version caveat."
  artifacts:
    - "scripts/pre-tag-check.sh (new, mode 0755)"
    - "CLAUDE.md (Release Process step 3 rewritten)"
  key_links:
    - "src/envelope/policy.rs constant declaration -> runtime extraction inside the script (no hardcoded version literal anywhere in the script)"
    - "Cargo.toml version, read via cargo metadata + jq -> gate 1 tag comparison (same ${ARG#v} strip CI uses)"
    - "Cargo.toml rust-version, read via cargo metadata -> gate 2 toolchain selection (mirrors the release.yml msrv job's own manifest read)"
    - "CLAUDE.md Release Process step 3 -> scripts/pre-tag-check.sh"
---

<objective>
Add a local pre-tag release dry-run, `scripts/pre-tag-check.sh`, that reproduces the gates
`.github/workflows/release.yml` runs on a `v*` tag push, and point CLAUDE.md's Release Process
step 3 at it.

Purpose: the v1.7.0 release CI failed at the publish gate on a git-version witness test whose
drift was invisible locally. A pre-tag dry-run makes the same gates runnable before the tag
exists, and — where a local run provably CANNOT stand in for CI — says so loudly instead of
returning a green that means nothing.

Output: one new executable shell script and one edited section of CLAUDE.md. No CI change,
no version bump, no tag, no Rust source change.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@.github/workflows/release.yml
</context>

<locked_decisions>

These were decided at planning time from live observation. Implement them as written; do not
re-open them during execution.

- **DEC-1 — Gate order follows the task brief (tag/version, MSRV, build, test, clippy), not the
  CI job graph.** In `release.yml` the `msrv` job is a `needs:` prerequisite of `publish`, so CI
  reaches the tag check only after a full MSRV compile. Locally the tag check costs a
  `cargo metadata` call, so it runs FIRST and hard-stops. Record this inversion in a script
  comment so a reader does not "fix" it back.

- **DEC-2 — Gate 1 (tag/version) hard-stops; gates 2-5 run to completion and collect statuses.**
  Everything after gate 1 is minutes of compilation that cannot be meaningful if the tag names a
  version the manifest does not carry. Gates 2-5 each record PASS/FAIL/SKIPPED and the script
  exits non-zero at the end if any of them FAILED — the operator gets the whole picture from one
  run, which is the same reason gate 4 uses `--no-fail-fast`.

- **DEC-3 — The MSRV floor is DERIVED from `cargo metadata`'s `rust_version`, not hardcoded.**
  The `msrv` job in release.yml reads exactly that field to certify its toolchain, so deriving it
  is faithful to CI and cannot go stale when the floor moves. MEASURED at planning time: this
  manifest declares `rust-version = "1.88"` (two-component), while `rustup toolchain list` on this
  machine holds BOTH `1.88-x86_64-unknown-linux-gnu` and `1.88.0-x86_64-unknown-linux-gnu`. A
  naive `^1\.88\.0` match against a two-component declared floor would wrongly report SKIPPED, and
  a naive `^1\.88` match would wrongly miss a machine that only has the three-component spelling.
  Resolution: match the declared floor followed by `.` or `-` or end-of-name, and then invoke the
  FULL installed toolchain name that matched. Mirror CI's own failure mode: if the manifest
  declares no `rust-version`, gate 2 FAILS (CI errors out in that case too) rather than skipping.

- **DEC-4 — A git/constant MISMATCH is a prominent ADVISORY, never a script-level failure.**
  MEASURED at planning time: local git is `2.53.0` while the constant on `src/envelope/policy.rs:1099`
  was re-derived against `2.55.0` (commits ef4a2a8..d70845c). That mismatch is the CURRENT AND
  CORRECT state of a properly-prepared release, so hard-failing on it would paint the script red on
  a good tree and train the operator to ignore it. It is instead made impossible to miss: a banner
  before the gates run AND an advisory line in the final summary. The exit code reflects only real
  gate failures. State this rationale in a script comment — it is the one decision a future reader
  is most likely to second-guess.

- **DEC-5 — Gate 4's known collateral failure is EXPLAINED, never suppressed.** While the advisory
  is active, the witness test
  `the_config_section_constants_record_the_git_version_they_were_derived_against` is expected to
  FAIL locally, which makes gate 4 FAIL and the script exit non-zero. The script does not filter,
  skip or special-case that failure (a filter there could hide a real regression). It detects the
  test name in gate 4's captured output and, if the advisory is active, prints a summary line
  saying the witness is among the failures and that any OTHER failing test is the thing to look at.
  If the test is ever renamed the hint simply goes quiet — the gate still fails correctly.

</locked_decisions>

<tasks>

<task type="tracer">
  <name>Task 1: scripts/pre-tag-check.sh — the whole gate chain end to end</name>
  <files>scripts/pre-tag-check.sh</files>
  <precondition>`jq` is on PATH (the script parses `cargo metadata` with it, exactly as release.yml does) and `src/envelope/policy.rs` still declares `pub const CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION: &str = "...";`. Both were observed present at planning time; if either is gone, halt rather than improvising around it.</precondition>
  <action>
Create the `scripts/` directory (it does not exist yet) and write `scripts/pre-tag-check.sh`.
Shebang `#!/usr/bin/env bash`, `set -u` only — do NOT add `set -e` (gates 2-5 must all run per
DEC-2) and do NOT add `pipefail` (the constant-extraction pipelines end in `head -1`, whose SIGPIPE
would be read as failure). `chmod +x` it.

Header comment block, a few lines: what this is (a local dry-run of `.github/workflows/release.yml`
before a tag exists), the gate-order inversion per DEC-1, and a pointer saying the workflow is the
authority whenever the two disagree.

Usage: one optional positional argument, the tag, e.g. `./scripts/pre-tag-check.sh v1.7.1`.
Reject a second positional argument with a usage message and exit 2.

Structure the script as: extract-constant, print-banner, then five gate blocks in order, with an
EXIT trap that always prints the summary. Concretely:

CONSTANT EXTRACTION (before anything else). Read the quoted literal out of
`src/envelope/policy.rs` at runtime — the version string must NEVER appear in this script. Match
the declaration line only, not the several doc-comment lines that also name the constant: an
extended-regexp anchored at line start allowing leading whitespace, `pub const`, the constant name,
`:`, `&str`, `=`; take the first match; then pull the quoted literal out of it with a
sed capture of the form `.*"(.*)".*`. If the file is missing, or the declaration does not match, or
the captured literal is empty, print a loud hard-error explaining that the script has lost its
subject (the constant it exists to reconcile against) and `exit 1` immediately. This is the one
constant-related condition that IS a hard failure — per DEC-4 the mismatch is not.

RECONCILIATION BANNER (printed on EVERY run, before gate 1, so it is still visible when gate 1
hard-stops). A banner-style block, full-width rule lines above and below, whose heading line is the
literal `!!! GIT VERSION RECONCILIATION !!!`. Inside it print, on labelled lines, the output of
`git --version` and the extracted constant. Then:
  - if they are EQUAL: one line saying the local tree can validate the witness test the way CI will.
  - if they DIFFER: the loud form, stating plainly, one claim per line, that (1) THIS LOCAL RUN
    CANNOT VALIDATE THAT TEST THE WAY CI WILL, (2) the GitHub `ubuntu-latest` runner's ambient git
    is the AUTHORITY for it, and (3) a green local gate therefore DOES NOT GUARANTEE the publish
    job passes. Set an advisory flag variable consumed by the summary.
Add the DEC-4 rationale as a comment right here: local git and the constant legitimately differ on
a correctly-prepared release, so this warns and does not fail.

Create a log directory with `mktemp -d` and announce its path in the header; gates 2-5 each tee
their output to a file inside it so the operator can go read a failure in full.

GATE 1 — tag/version (mirrors the publish job's "Check tag matches Cargo.toml version"). Read the
crate version with `cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version'`.
With NO argument: print the crate version, mark the gate SKIPPED with the reason "no tag argument",
and continue — this is not a failure. With an argument: strip a leading `v` using the SAME
`${ARG#v}` parameter expansion CI uses, compare to the crate version, and on disagreement print the
mismatch (both values) and `exit 1` immediately per DEC-2, before any compilation.

GATE 2 — MSRV (mirrors the `msrv` job) per DEC-3. Read `.packages[0].rust_version` from the same
`cargo metadata` shape. If it is empty or the string `null`, mark the gate FAILED with CI's own
reasoning (a floor that does not exist cannot be certified). If `rustup` is not on PATH, mark
SKIPPED with that reason. Otherwise select the toolchain by NAME: take `rustup toolchain list`,
keep the first whitespace-delimited field of each line (the list annotates the active/default entry),
and find the first whose name is the declared floor followed by `.`, `-`, or end of name — escape
the dots when building that pattern. If nothing matches, mark the gate SKIPPED with a clearly
marked line naming the floor that is absent; it must be unmistakable that the gate did not pass.
Do NOT probe with `cargo +<floor> --version` under any circumstances — rustup AUTO-DOWNLOADS AND
INSTALLS a missing toolchain on that probe, which both lies about the skip condition and mutates
the developer's machine; say exactly that in a comment. When a toolchain matched, run
`cargo +"$TOOLCHAIN" check --all-targets --locked` with the full matched name, and record PASS/FAILED.

GATE 3 — `cargo build --release`. Record PASS/FAILED.

GATE 4 — `cargo test --no-fail-fast`. Add a comment stating this DELIBERATELY differs from CI's
plain `cargo test` and why: fail-fast stops at the first failing test BINARY and hides every later
suite behind it, which is exactly how the v1.7.0 breakage stayed invisible. Tee the output; take
the cargo exit status from `${PIPESTATUS[0]}`, not from tee. Record PASS/FAILED. Then per DEC-5,
grep the captured output for the witness test name
`the_config_section_constants_record_the_git_version_they_were_derived_against` and set a flag if
it is present; do not filter, skip or otherwise modify the gate's verdict.

GATE 5 — `cargo clippy -- -D warnings`. Record PASS/FAILED.

SUMMARY + EXIT TRAP. Install `trap 'rc=$?; print_summary; exit "$rc"' EXIT` near the top, after the
status variables are initialised, so the summary prints on EVERY exit path — including gate 1's
hard stop and the constant-extraction hard error — and the original exit status is preserved.
Initialise all five gate statuses to the literal `NOT REACHED` so an early exit reports honestly.
`print_summary` prints a rule, the heading `=== PRE-TAG SUMMARY ===`, one aligned line per gate
(number, short name, status), the log directory path, and then:
  - if the advisory flag is set: repeat the git-vs-constant comparison as an `ADVISORY:` block —
    both values again, plus the one-line restatement that the runner is the authority and a green
    local gate does not guarantee publish. This repetition is the point: it must survive a long
    scrollback.
  - if the advisory flag is set AND gate 4 FAILED AND the witness-test flag is set: a line saying
    the version witness is among the failing tests and is EXPECTED to fail while the advisory
    stands, so the operator should read the gate-4 log for any OTHER failure before concluding a
    regression.
  - a closing line stating that the exit code reflects gate failures only and that the advisory,
    on its own, does not fail the run.
Final exit status: 0 when no gate is FAILED, 1 when any gate is FAILED. SKIPPED is not a failure.
  </action>
  <verify>
    <automated><![CDATA[
set -e
bash -n scripts/pre-tag-check.sh
test -x scripts/pre-tag-check.sh
# no version literal baked into the script (comment lines excluded so prose cannot self-invalidate)
test "$(grep -v '^[[:space:]]*#' scripts/pre-tag-check.sh | grep -Ec '"git version [0-9]')" = "0"
LOG=$(mktemp)
if ./scripts/pre-tag-check.sh v0.0.0-nope >"$LOG" 2>&1; then echo "FAIL: mismatched tag accepted"; exit 1; fi
CONST=$(grep -E '^[[:space:]]*pub const CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION[[:space:]]*:' src/envelope/policy.rs | head -1 | sed -E 's/.*"(.*)".*/\1/')
test -n "$CONST"
grep -qF "$CONST" "$LOG"          # constant was extracted at runtime, not hardcoded
grep -qF 'GIT VERSION RECONCILIATION' "$LOG"
grep -qF 'NOT REACHED' "$LOG"     # summary trap fired on the early exit
./scripts/pre-tag-check.sh --help >/dev/null 2>&1 || true
echo "TASK1 OK (transcript: $LOG)"
]]></automated>
  </verify>
  <done>`scripts/pre-tag-check.sh` exists, is executable, parses under `bash -n`, contains no hardcoded version literal, and a run with a deliberately wrong tag exits non-zero having printed the reconciliation banner and a summary in which the later gates read NOT REACHED.</done>
</task>

<task type="auto">
  <name>Task 2: point CLAUDE.md Release Process step 3 at the script</name>
  <files>CLAUDE.md</files>
  <action>
In CLAUDE.md's `## Release Process` list, rewrite item 3 (`**Verify clean**`). It stays item 3 and
keeps the surrounding numbering 1-6 untouched. Delete the inline three-command verification chain
(the build/test/clippy commands joined by shell `&&`) and replace it with an invocation of
`./scripts/pre-tag-check.sh <tag>` run after the bumps from steps 1-2, passing the tag about to be
created. Keep the existing terse imperative voice — no headings, no sub-bullets, no code fence;
one list item.

Append one or two sentences, no more, carrying the git-version caveat so the checklist itself
warns: the script reconciles the locally installed git against the constant in
`src/envelope/policy.rs`, the GitHub `ubuntu-latest` runner is the authority for that witness test,
and a mismatch there is an advisory the operator must read rather than a script failure.
The sentence must contain the word `runner` so the authority is named explicitly.

Touch nothing else in CLAUDE.md.
  </action>
  <verify>
    <automated><![CDATA[
set -e
STEP3=$(mktemp)
sed -n '/^3\. \*\*Verify clean\*\*/,/^4\. \*\*/p' CLAUDE.md > "$STEP3"
test -s "$STEP3"
grep -qF 'scripts/pre-tag-check.sh' "$STEP3"
grep -qi 'runner' "$STEP3"
test "$(grep -Ec 'cargo build &&' "$STEP3")" = "0"
test "$(grep -Ec '^[0-9]+\. \*\*' CLAUDE.md)" = "6"
echo "TASK2 OK"
]]></automated>
  </verify>
  <done>CLAUDE.md step 3 is still item 3 of a 6-item list, invokes `scripts/pre-tag-check.sh` with the tag, no longer carries the inline command chain, and names the runner as the authority for the git-version witness.</done>
</task>

<task type="auto">
  <name>Task 3: exercise the script end to end against the real tree</name>
  <files>scripts/pre-tag-check.sh</files>
  <action>
Run the script once for real, deriving the tag from the manifest rather than hardcoding it, so
gates 2-5 are actually exercised. Expected shape on this machine, MEASURED at planning time — use
it to judge the output, not to decide the script is broken:
  - gate 1 PASSES (the derived tag necessarily matches the crate version);
  - gate 2 RUNS rather than skipping — the declared floor `1.88` is installed here, so this is a
    full `cargo check --all-targets --locked` on a second toolchain and is the slowest step;
  - the advisory IS active (local git differs from the constant);
  - gate 4 is EXPECTED to FAIL, carrying the version witness plus the known per-run-flaky
    `driver_reattach` pair, so a non-zero exit code here is the correct result and NOT a script
    defect. What is being verified is that the script ran every gate, classified each one, and
    explained the witness failure in the summary.

CONTENTION: another `cargo test` was running against this target directory at planning time and
holds the cargo build lock. If the run parks on the cargo lock and the timeout expires, do NOT
treat that as a failure and do NOT retry in a loop — record the deferral in the SUMMARY with the
observed blocking line as evidence, and let Task 1's shell-level checks stand as the gate for the
script's structure. Fix the script only if the transcript shows a defect of the script's own
(missing gate line, missing summary, wrong classification, a hard error from the extraction).
  </action>
  <!-- planner-discipline-allow: NOT REACHED -->
  <!-- The negative grep below targets a runtime transcript produced by mktemp, not a source file,
       so the literal appearing in Task 1's action text cannot satisfy or invalidate it. -->
  <verify>
    <automated><![CDATA[
set -u
V=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')
LOG=$(mktemp)
timeout 2700 ./scripts/pre-tag-check.sh "v$V" >"$LOG" 2>&1
RC=$?
echo "exit=$RC transcript=$LOG"
if [ "$RC" = "124" ]; then echo "TIMEOUT — check for cargo lock contention:"; grep -i 'waiting for file lock' "$LOG" | head -3; exit 0; fi
grep -qF 'GIT VERSION RECONCILIATION' "$LOG" || { echo "FAIL: no reconciliation banner"; exit 1; }
grep -qF 'PRE-TAG SUMMARY' "$LOG" || { echo "FAIL: no summary"; exit 1; }
test "$(grep -cF 'NOT REACHED' "$LOG")" = "0" || { echo "FAIL: a gate was never reached"; exit 1; }
grep -qF 'ADVISORY' "$LOG" || { echo "FAIL: advisory not repeated in summary"; exit 1; }
sed -n '/PRE-TAG SUMMARY/,$p' "$LOG"
echo "TASK3 OK"
]]></automated>
  </verify>
  <done>Either the transcript shows all five gates classified (none left NOT REACHED), the reconciliation banner, and the advisory repeated in the summary — with gate 4's failure explained rather than hidden — or the run is recorded in the SUMMARY as blocked on the cargo target-directory lock, quoting the observed blocking line, with Task 1's checks standing as the structural gate.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| operator argv -> script | the tag argument is operator-supplied; it is only `${ARG#v}`-stripped and string-compared, never evaluated or used to build a path |
| repo working tree -> script | `src/envelope/policy.rs` and `Cargo.toml` are read at runtime; their content shapes the script's reported verdicts |
| script -> developer machine | the script invokes cargo/rustup subcommands that could mutate local toolchain state |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-jdi-01 | Tampering | rustup toolchain state | medium | mitigate | DEC-3 forbids the `cargo +<floor> --version` probe, which auto-installs a toolchain; detection is read-only via `rustup toolchain list` |
| T-jdi-02 | Spoofing | the local gate standing in for CI | high | mitigate | the reconciliation banner plus the summary advisory state that the runner is the authority and a green local gate does not guarantee publish |
| T-jdi-03 | Information disclosure | mktemp log directory | low | accept | logs hold cargo output only, no credentials; created by `mktemp -d` with default 0700 |
| T-jdi-04 | Elevation of privilege | operator-supplied tag argument | low | mitigate | argument is never `eval`'d, never interpolated into a path, and every expansion is quoted |
| T-jdi-05 | Repudiation | a suppressed gate-4 failure | medium | mitigate | DEC-5 forbids filtering the witness failure; the gate verdict stands and only an explanatory line is added |

No package-manager install step is introduced (no new dependency is added to Cargo.toml), so no
package-legitimacy checkpoint applies.
</threat_model>

<verification>
- `bash -n scripts/pre-tag-check.sh` parses clean and the file is mode 0755.
- `shellcheck` is NOT installed on this machine (observed at planning time). If the executor finds
  it available, running `shellcheck scripts/pre-tag-check.sh` is a bonus signal; its absence is not
  a gate failure and it must not be installed for this task.
- No version literal is baked into the script: the constant is extracted from
  `src/envelope/policy.rs` on every run, and removing the declaration makes the script hard-fail.
- A wrong-tag run exits non-zero before any compilation, with the banner and the summary printed.
- CLAUDE.md step 3 invokes the script, keeps the 6-item numbering, and carries the caveat.
- Nothing under `.github/`, `Cargo.toml`, or `src/` is modified — confirm with `git status` before
  committing.
</verification>

<success_criteria>
- `scripts/pre-tag-check.sh` reproduces release.yml's gates in the DEC-1 order and exits non-zero
  when any of them fails.
- The git-version reconciliation is impossible to miss (banner before the gates, advisory in the
  summary) and never by itself changes the exit code.
- The MSRV gate never installs a toolchain and never silently passes when the floor is absent.
- CLAUDE.md's release checklist routes step 3 through the script and carries the caveat.
- Exactly two files changed.
</success_criteria>

<output>
Create `.planning/quick/260917-jdi-add-scripts-pre-tag-check-sh-pre-tag-rel/260917-jdi-SUMMARY.md` when done.
Record in it: the observed exit code and gate classifications from Task 3 (or the deferral and its
evidence), whether gate 2 ran or skipped, and which tests were in gate 4's failing set.
</output>
</content>
</invoke>
