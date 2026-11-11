---
phase: quick-261110-tbn
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/ui/screens/detail.rs
  - scripts/pre-tag-check.sh
  - scripts/pre-tag-check.Dockerfile
  - .github/workflows/release.yml
  - CLAUDE.md
  - docs/DEVELOPMENT.md
autonomous: true
requirements: [QUICK-261110-tbn]

estimate:
  tokens: 45000
  raw_tokens: 45000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "For every target in release.yml's cross-target-check matrix (aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-gnu), the job's exact commands `cargo check --locked --target T --all-targets` and `cargo clippy --locked --target T --all-targets -- -D warnings` exit 0 (U-1)."
    - "`assert_dispatched_for_item_999_1` in src/ui/screens/detail.rs carries exactly `#[cfg(any(target_os = \"linux\", target_os = \"macos\"))]`, the cfg of its three callers. No `allow(dead_code)` was added and nothing under src/multiplexer/ changed (U-1)."
    - "On Linux the seven `backlog_one_off_t_` lib tests, including the three helper callers, still run and pass, and `cargo clippy --all-targets -- -D warnings` exits 0."
    - "`./scripts/pre-tag-check.sh` has a gate 7 that reads the target list (target, check_args, clippy) from release.yml's cross-target-check matrix at run time and runs that job's exact Check and Clippy commands for each target. The summary prints seven gate lines, and a gate-7 FAILED makes the exit status 1 (U-2)."
    - "Gate 7 refuses to certify a matrix it cannot model. Zero entries, an unknown matrix key, an invalid value, or Check/Clippy/if lines in the job that no longer match what the gate runs all give FAILED, never PASS or SKIPPED. In --container mode the same refusal is a HARD ERROR before any image build (U-2)."
    - "`--container` passes the derived target list to the image build as the build arg PRE_TAG_CROSS_TARGETS, and the image installs exactly those rustup targets for stable. Inside the container a target that is not installed makes gate 7 FAILED (image drift). In a bare run it gives SKIPPED with a `rustup target add` hint. Gate 7 never installs anything (U-2)."
    - "`./scripts/pre-tag-check.sh --container`, run with no tag on the fixed tree, exits 0 with gate 7 PASS. CLAUDE.md Release Process describes seven gates, including the cross-target gate. In release.yml only comment lines changed (U-2)."
  artifacts:
    - path: "src/ui/screens/detail.rs"
      provides: "the helper gated to its callers' cfg"
      contains: "fn assert_dispatched_for_item_999_1"
    - path: "scripts/pre-tag-check.sh"
      provides: "the release.yml matrix parser, gate 7, the build-arg handoff and the seven-line summary"
      contains: "G7_STATUS"
    - path: "scripts/pre-tag-check.Dockerfile"
      provides: "ARG PRE_TAG_CROSS_TARGETS and rustup target add for stable"
      contains: "PRE_TAG_CROSS_TARGETS"
    - path: "CLAUDE.md"
      provides: "the seven-gate Release Process description"
      contains: "seven gates"
  key_links:
    - from: "scripts/pre-tag-check.sh gate 7"
      to: ".github/workflows/release.yml cross-target-check matrix include entries + Check/Clippy run lines"
      via: "the parser function, resolved from the script's own location (never cwd)"
      pattern: "cross-target-check"
    - from: "scripts/pre-tag-check.sh --container dispatch (build step)"
      to: "scripts/pre-tag-check.Dockerfile ARG PRE_TAG_CROSS_TARGETS"
      via: "--build-arg PRE_TAG_CROSS_TARGETS=<validated, space-separated triples>"
      pattern: "build-arg"
    - from: "scripts/pre-tag-check.Dockerfile"
      to: "stable toolchain rust-std for each target"
      via: "rustup target add --toolchain stable"
      pattern: "rustup target add"
---

<objective>
The v2.2.0 and v2.3.0 tags both died in release.yml's `cross-target-check` job. On x86_64-pc-windows-gnu, clippy rejected a test helper as dead code, because its only callers are gated to linux/macos. No local gate could see this.

This plan does two things:
- U-1: Fix that helper properly by gating it to its callers' cfg. Then prove every release.yml matrix target is clean under the job's exact commands.
- U-2: Close the gate gap. `scripts/pre-tag-check.sh` gets a gate 7 that mirrors the cross-target-check matrix in bare and `--container` mode. The image gets the rustup targets, and the release docs get the new gate count.

U-1 and U-2 label the two parts of the user's request in planning_context. There is no CONTEXT.md.

Out of scope: the release itself (version bump, cargo update, STATE milestone, release commit, tag). The orchestrator does that afterward. No push.

Purpose: the next release tag cannot fail on a foreign-target lint that a green local `--container` rehearsal did not already reject.

Output: one cfg attribute in src/ui/screens/detail.rs; gate 7 and the build-arg handoff in scripts/pre-tag-check.sh; an ARG/rustup layer in scripts/pre-tag-check.Dockerfile; comment-only edits in .github/workflows/release.yml; updated CLAUDE.md and docs/DEVELOPMENT.md.

Planner discovery, measured at HEAD cea3a75: `cargo clippy --locked --keep-going --target T --all-targets -- -D warnings` was run for all three targets. Exactly ONE error exists, src/ui/screens/detail.rs:20714 `function assert_dispatched_for_item_999_1 is never used`, Windows only. Both apple targets are already clean. No target has a `cc` build dependency (`cargo tree -e build -i cc` prints nothing), so `cargo check` needs no mingw or apple C toolchain, on the host or in the container. Run verbatim at HEAD, Task 1's `<automated>` block prints `FAIL-clippy-x86_64-pc-windows-gnu`. That is the RED baseline, and the block goes green only through the fix. Nothing under src/multiplexer/ is implicated. A parallel worktree is editing that directory: do NOT touch it. If, against this measurement, a cross-target error appears there, stop and report it instead of editing.

Inferred decisions (user unavailable; marked for audit):
- [inferred — audit] The new gate is APPENDED as gate 7, not inserted and renumbered. Existing references to "gate 6" (release.yml publish-step comment, CLAUDE.md, the script's env docs) stay valid. Local gate order does not matter because every gate always runs and reports.
- [inferred — audit] release.yml is the single source of the target list, for BOTH the gate and the image. The `--container` dispatch passes the parsed list into the image build as a build arg. The Dockerfile keeps no hardcoded copy, so the list cannot drift. The layer cache keys on the arg value.
- [inferred — audit] Sync is enforced at run time, not by a new Rust test. The parser fails if an unknown matrix key appears, if a value is malformed, if no entries are found, or if the job's literal Check, Clippy and `if: matrix.clippy` lines change. Inside the container, a missing target is FAILED.
- [inferred — audit] Clippy is not run for a target whose Check failed. This mirrors CI: the Clippy step's `if: matrix.clippy` carries an implicit `success()`. The gate also has no `--keep-going`: it runs the job's commands verbatim.
- [inferred — audit] In a bare run, a target not installed with rustup gives SKIPPED. This follows gate 2's precedent of never mutating the toolchain. In the container it gives FAILED.
- [inferred — audit] One full `./scripts/pre-tag-check.sh --container` run (no tag) is the end-to-end proof. The orchestrator's release run afterward then starts on warm cache volumes.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@.github/workflows/release.yml
@scripts/pre-tag-check.sh
@scripts/pre-tag-check.Dockerfile

Facts the executor needs (verified by the planner; do not re-derive):
- src/ui/screens/detail.rs:20714 `fn assert_dispatched_for_item_999_1(rx: &mut tokio::sync::mpsc::UnboundedReceiver<crate::action::Action>)`. Line 20713 above it is blank. Its only callers are `backlog_one_off_t_dispatches_the_selected_item_without_a_screen`, `backlog_one_off_t_works_with_the_content_pane_open` and `backlog_one_off_t_starts_another_run_while_one_is_live`. Each carries `#[cfg(any(target_os = "linux", target_os = "macos"))]` above `#[test]`. `backlog_one_off_fixture` stays ungated, because the ungated `backlog_one_off_t_is_inert_with_the_flag_off` uses it.
- release.yml cross-target-check matrix: lines 66-79, `include:` entries at 10-space indent `- target: T`, keys `check_args: --all-targets` and `clippy: true` at 12-space indent. Line 94 is the Check step `run: cargo check --locked --target "${{ matrix.target }}" ${{ matrix.check_args }}`. Line 97 is `if: matrix.clippy`. Line 98 is the Clippy step `run: cargo clippy --locked --target "${{ matrix.target }}" --all-targets -- -D warnings`. The only `- target:` lines in the file are the three matrix entries. The job's leading comment block ends with a sentence saying the script does not yet mirror the job and calling that a follow-up. Task 2 replaces that sentence.
- scripts/pre-tag-check.sh: the `set -u` design (no `set -e`, no pipefail), and the dispatch block (lines 207-570) that exits before the EXIT trap. Cap validation is step 1b at 230-259. Runtime resolution is step 2 at 261. The build is at 407-418. The `G*_STATUS`/`G*_NOTE` init is at 580-591. `gate_line`/`print_summary` are at 595-625. `run_gate` (tee + PIPESTATUS) is at 633-642. Gate 2's read-only `rustup toolchain list` precedent is at 705-743. Gate 6 and the dirty-tree advisory run to 851. The OVERALL_RC loop is at 853-859. The comment at line 307 mentions a "run-from-the-repo-root check further down" that no longer exists, so resolve paths from the script's own location, the way `CONTAINER_SCRIPT_DIR` does.
- Host: all three rustup targets are installed; docker is usable; shellcheck is NOT installed. Cache volumes gsdmm-pretag-target / gsdmm-pretag-cargo-home already exist.
- RTK filters shell output, and the hook rewrites top-level git/grep/cargo. So every `<automated>` block below is ONE `rtk bypass bash -c '...'` and prints `verify-ok` on success or `FAIL-<step>` on the first failed check. Run each block verbatim.
- The test filter `backlog_one_off_t_` selects exactly seven lib tests, all in src/ui/screens/detail.rs. The broader `backlog_one_off` also matches tests in help.rs and app.rs.
- Diff guards are scoped to THIS task's commits (`git log --grep=quick-261110-tbn cea3a75..HEAD`) plus the uncommitted tree. A parallel src/multiplexer/ branch merged into master in the meantime therefore cannot false-fail them.
</context>

<tasks>

<!-- planner-discipline-allow: seven gates -->
<!-- planner-discipline-allow: gate 7 -->
<!-- Both literals are POSITIVE deliverables (Task 3 greps them with -ge 1); no criterion requires their absence. -->

<task type="tracer">
  <name>Task 1: Cross-target clippy green end-to-end — gate the dead helper to its callers' cfg, prove every release.yml matrix target clean (U-1)</name>
  <files>src/ui/screens/detail.rs</files>
  <action>Per U-1, add exactly `#[cfg(any(target_os = "linux", target_os = "macos"))]` on its own line directly above `fn assert_dispatched_for_item_999_1(` in src/ui/screens/detail.rs (line ~20714, inside the test module). It must be character-identical to the attribute on its three callers, so the helper exists exactly when they do. Do NOT use `allow(dead_code)`, `cfg_attr` or any broader gate, and do not gate `backlog_one_off_fixture`, which an ungated test also uses. No other source change is expected: the planner's `--keep-going` clippy run at HEAD found this as the only error on all three targets.

Then run the job's two commands verbatim for every target listed under `- target:` in .github/workflows/release.yml. If a further error surfaces, fix it the same way: gate the item to the exact cfg of its users, never with a blanket allow. If it lies under src/multiplexer/, stop and report it, because a parallel worktree owns that directory. Also confirm Linux stays clean: `cargo clippy --all-targets -- -D warnings`, and the seven `backlog_one_off_t_` lib tests still run and pass.

Commit as `fix(quick-261110-tbn): cfg-gate assert_dispatched_for_item_999_1 to its callers`.</action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail() { echo "FAIL-$1"; exit 1; }; TS=$(grep -E "^ +- target: " .github/workflows/release.yml | awk "{print \$3}"); [ -n "$TS" ] || fail no-matrix-targets; for t in $TS; do cargo check --locked --target "$t" --all-targets || fail "check-$t"; cargo clippy --locked --target "$t" --all-targets -- -D warnings || fail "clippy-$t"; done; cargo clippy --all-targets -- -D warnings || fail clippy-linux; [ "$(grep -B1 "fn assert_dispatched_for_item_999_1" src/ui/screens/detail.rs | head -1 | sed "s/^ *//")" = "#[cfg(any(target_os = \"linux\", target_os = \"macos\"))]" ] || fail cfg; D=$(git log --grep=quick-261110-tbn -p cea3a75..HEAD -- src tests && git diff HEAD -- src tests) || fail git-diff; [ "$(printf "%s\n" "$D" | grep -c "^+.*allow(dead_code)")" -eq 0 ] || fail allow-dead-code; M=$(git log --grep=quick-261110-tbn --format= --name-only cea3a75..HEAD -- src/multiplexer && git status --porcelain -- src/multiplexer) || fail git-multiplexer; [ -z "$M" ] || fail multiplexer-touched; mkdir -p -m700 /tmp/claude-1000/t; (umask 077; TMPDIR=/tmp/claude-1000/t cargo test --lib backlog_one_off_t_ 2>&1) | grep -qE "^test result: ok\. 7 passed; 0 failed" || fail lib-tests; echo verify-ok'</automated>
  </verify>
  <done>Each matrix target passes the job's Check and Clippy commands with exit 0. Before this change the Windows clippy failed. Linux clippy also exits 0. The helper's preceding line is the callers' cfg. This task's diff adds no `allow(dead_code)` and touches nothing under src/multiplexer/. `cargo test --lib backlog_one_off_t_` reports 7 passed, 0 failed. The block prints verify-ok and the change is committed.</done>
</task>

<task type="auto">
  <name>Task 2: pre-tag-check gate 7 mirrors release.yml cross-target-check, and the image installs the derived rustup targets (U-2)</name>
  <files>scripts/pre-tag-check.sh, scripts/pre-tag-check.Dockerfile, .github/workflows/release.yml</files>
  <behavior>
    - Valid release.yml, `--container`, `PRE_TAG_CONTAINER_RUNTIME=gsdmm-no-such-runtime`: the parse passes and the run stops at the runtime probe. Exit 1, output contains "no usable container runtime".
    - Unknown matrix key (e.g. `features: x` added to an entry), same invocation: exit 1 with a parse refusal naming the key. Output does NOT contain "no usable container runtime", because the refusal comes before the probe.
    - The cross-target Clippy run line altered (e.g. `--all-targets` dropped from it), same invocation: exit 1 with a refusal saying the job's commands no longer match gate 7. No runtime message.
    - Every `- target:` line removed: exit 1 with a refusal (orphan keys or zero entries, whichever the parser hits first). No runtime message.
    - Bare run from a scratch copy that holds only scripts/ and .github/ (no Cargo.toml): exit 1. The output contains `+ cargo check --locked --target T --all-targets` for each of the three targets, and the summary has a `gate 7  cross-target check+clippy` line marked FAILED.
    - Image built without the build arg: the build fails with a message saying the image is built only via `./scripts/pre-tag-check.sh --container`. Built with it: `rustup target list --installed` in the image lists all three targets.
  </behavior>
  <action>Per U-2, all three files below change.

(1) scripts/pre-tag-check.sh: matrix parser. Add a function, defined BEFORE the `--container` dispatch block, that reads `.github/workflows/release.yml`. Resolve the file from the script's own directory (the `cd -- "$(dirname -- "${BASH_SOURCE[0]}")"` technique), never from cwd. Print one tab-separated line `target<TAB>check_args<TAB>clippy` per entry of the `cross-target-check` job's `strategy.matrix.include` list, using awk/grep/sed only (no yq or python: neither is guaranteed in the image).

Contract:
- The job block runs from the line `  cross-target-check:` to the next 2-space-indent non-comment key, or EOF. Skip comment and blank lines.
- An entry starts at `- target: V`. The keys `check_args` and `clippy` belong to the current entry. Any other key inside the include list is REFUSED, and the message names the key.
- Strip one surrounding pair of double quotes from a value.
- Validate each target against `^[a-z0-9_]+(-[a-z0-9_.]+){2,3}$`. Validate each whitespace token of check_args as a flag (`^--?[A-Za-z0-9][A-Za-z0-9_-]*$`). clippy must be `true` or `false`.
- An absent check_args means empty and an absent clippy means false. That is exactly how CI's `${{ matrix.check_args }}` and `if: matrix.clippy` treat missing keys.
- Zero entries is REFUSED.
- Also REFUSE unless the job block's NON-COMMENT lines still contain, as fixed strings, the three lines gate 7 models: the Check `run:` line, `if: matrix.clippy`, and the Clippy `run:` line, all quoted in the context section. Skip comment lines here so that a quoted copy in a comment can never satisfy the guard. This is the mechanical keep-in-sync guard: if CI's commands change, the gate fails until it is updated.
- On refusal, print the reason on stderr and return non-zero.

(2) scripts/pre-tag-check.sh: dispatch. When `CONTAINER=1`, add a step 1c after cap validation (1b) and BEFORE runtime resolution (step 2), so no side effect precedes it. It runs the parser and collects the targets into one space-separated list. A refusal is a HARD ERROR (exit 1) in the existing RULE/THIN style, saying THIS RUN HAS CERTIFIED NOTHING. At step 3, add `--build-arg "PRE_TAG_CROSS_TARGETS=<list>"` to the build and to its echoed `+ ... build` line, and add a cross-targets line to the CONTAINER MODE banner. Do not touch the clean, cap, prep or run logic.

(3) scripts/pre-tag-check.sh: gate 7. Add `G7_STATUS="NOT REACHED"` and `G7_NOTE=""` beside the others. Add `gate_line 7 "cross-target check+clippy"` to print_summary, and add `$G7_STATUS` to the OVERALL_RC loop. The gate section, "GATE 7 — cross-target check + clippy (release.yml cross-target-check matrix)", goes after the dirty-tree advisory. In order:
- A parser refusal gives FAILED, note "could not derive the matrix from release.yml".
- No rustup on PATH gives SKIPPED, as gate 2 does.
- Otherwise, read the installed targets once with the read-only `rustup target list --installed`. Never `rustup target add` and never anything that auto-installs.
- For each entry: if the target is not installed, the result is FAILED when `PRE_TAG_CHECK_IN_CONTAINER` is non-empty (the image drifted from release.yml) and SKIPPED otherwise. On SKIPPED, print the exact `rustup target add T` command.
- Otherwise, run `run_gate "gate7-T-check.log" cargo check --locked --target T` followed by the check_args tokens, split with `read -ra` so there is no globbing and no eval. If that passed and clippy is true, run `run_gate "gate7-T-clippy.log" cargo clippy --locked --target T --all-targets -- -D warnings`. If check failed, record that clippy was not run (CI skips the step too).
- Aggregate: any failure gives FAILED, with a note naming the target(s) and step. Otherwise any skip gives SKIPPED, with a note naming the uninstalled targets. Otherwise PASS, with a note giving the target count.

Explain each choice in the house style, saying WHY: the v2.2.0/v2.3.0 failure, derive-not-copy, the sync guard, no-install, container FAILED vs bare SKIPPED, and why clippy is skipped after a failed check. Update the header gate list (lines 6-7), the usage() gate list (add gate 7, how its list is derived, and the bare-run rustup requirement), the dispatch comment that counts the gates in the trap's summary (line ~196), and any other in-script gate count to seven.

(4) scripts/pre-tag-check.Dockerfile. Append at the END, after the gh layer, so a target change rebuilds only this layer: `ARG PRE_TAG_CROSS_TARGETS`, then one RUN that:
- fails with a clear message when the arg is empty (the image is built only by `./scripts/pre-tag-check.sh --container`, which derives the arg from release.yml);
- re-validates that the value contains only `[a-z0-9_.-]` and spaces, as a defence in depth before it is word-split in the shell;
- runs `rustup target add --toolchain stable $PRE_TAG_CROSS_TARGETS`, deliberately unquoted;
- re-applies `chmod -R a+w /usr/local/rustup`, for the same unprivileged-uid reason as the toolchain layer.
Give it a WHY comment in this file's style, saying it mirrors release.yml's cross-target-check toolchain step. Update the Rust layer's comment that lists which gates stable+clippy covers, so it includes gate 7.

(5) .github/workflows/release.yml, COMMENTS ONLY. Replace the cross-target-check comment block's final sentence (the one calling script mirroring a follow-up) with a note on gate 7:
- scripts/pre-tag-check.sh gate 7 now mirrors this job in bare and --container mode;
- it reads the include entries (target, check_args, clippy) from this file, refuses any other key, and refuses unless this job's Check, `if: matrix.clippy` and Clippy lines match what it runs;
- so changing those lines or adding a matrix key means updating gate 7;
- quick 261110-tbn added it after v2.2.0/v2.3.0 failed here.
No job, step, `run:`, `if:` or matrix value may change.

Commit as `feat(quick-261110-tbn): pre-tag-check gate 7 mirrors release.yml cross-target-check`.</action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail() { echo "FAIL-$1"; exit 1; }; bash -n scripts/pre-tag-check.sh || fail syntax; W=.github/workflows/release.yml; D=$(git log --grep=quick-261110-tbn -p cea3a75..HEAD -- "$W" && git diff HEAD -- "$W") || fail git-diff; [ "$(printf "%s\n" "$D" | grep -E "^[+-]" | grep -vE "^(\+\+\+|---)" | grep -vcE "^[+-] *#")" -eq 0 ] || fail yml-noncomment-change; [ "$(grep -c "does not mirror this gate yet" "$W")" -eq 0 ] || fail yml-followup-sentence; [ "$(grep -v "^ *#" scripts/pre-tag-check.sh | grep -c G7_STATUS)" -ge 4 ] || fail g7-wiring; [ "$(grep -v "^ *#" scripts/pre-tag-check.Dockerfile | grep -c PRE_TAG_CROSS_TARGETS)" -ge 2 ] || fail dockerfile-arg; S=$(mktemp -d); mkdir -p "$S/scripts" "$S/.github/workflows"; cp scripts/pre-tag-check.sh "$S/scripts/"; Y="$S/$W"; R=gsdmm-no-such-runtime; probe() { PRE_TAG_CONTAINER_RUNTIME=$R bash "$S/scripts/pre-tag-check.sh" --container >"$S/$1" 2>&1; echo $?; }; cp "$W" "$Y"; [ "$(probe a)" -eq 1 ] && grep -q "no usable container runtime" "$S/a" || fail a-valid; (cd "$S" && bash scripts/pre-tag-check.sh) >"$S/d" 2>&1; [ $? -eq 1 ] || fail d-rc; for t in aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-gnu; do grep -qF "+ cargo check --locked --target $t --all-targets" "$S/d" || fail "d-$t"; done; grep -qE "gate 7 +cross-target check\+clippy +FAILED" "$S/d" || fail d-summary; sed -i "s/^\(            check_args: --all-targets\)$/\1\n            features: x/" "$Y"; [ "$(probe b)" -eq 1 ] && grep -q features "$S/b" && ! grep -q "no usable container runtime" "$S/b" || fail b-unknown-key; cp "$W" "$Y"; sed -i "s/--all-targets -- -D warnings/-- -D warnings/" "$Y"; [ "$(probe c)" -eq 1 ] && ! grep -q "no usable container runtime" "$S/c" || fail c-runline-drift; cp "$W" "$Y"; sed -i "/- target: /d" "$Y"; [ "$(probe e)" -eq 1 ] && ! grep -q "no usable container runtime" "$S/e" || fail e-zero-entries; ! docker build -q -f scripts/pre-tag-check.Dockerfile scripts >/dev/null 2>&1 || fail build-without-arg-succeeded; docker build -q --label gsdmm.pre-tag-check=1 --build-arg "PRE_TAG_CROSS_TARGETS=aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-gnu" -t gsdmm-pre-tag-check:latest -f scripts/pre-tag-check.Dockerfile scripts >/dev/null || fail build-with-arg; [ "$(docker run --rm gsdmm-pre-tag-check:latest rustup target list --installed | grep -cE "^(aarch64-apple-darwin|x86_64-apple-darwin|x86_64-pc-windows-gnu)$")" -ge 3 ] || fail image-targets; echo verify-ok'</automated>
  </verify>
  <done>The block prints verify-ok. The script parses. Every behavior case holds: a valid matrix gets past the parse, while an unknown key, run-line drift and zero entries are each refused before the runtime probe. The bare scratch run derives all three exact Check lines and prints a gate 7 summary line. The image refuses to build without the arg and with it installs all three targets. In release.yml only comment lines changed, and the follow-up sentence is gone. The change is committed.</done>
</task>

<task type="auto">
  <name>Task 3: Release docs describe seven gates; one full --container rehearsal proves gate 7 green end-to-end (U-2)</name>
  <files>CLAUDE.md, docs/DEVELOPMENT.md</files>
  <action>Per U-2, update the docs.

CLAUDE.md, "Release Process" step 3 (line ~132): after the gate 6 sentence, add a sentence on gate 7. Gate 7 runs release.yml's `cross-target-check` matrix: `cargo check --locked --target T --all-targets` and `cargo clippy --locked --target T --all-targets -- -D warnings` for every target the matrix lists. The list is read from release.yml at run time, and container mode bakes those rustup targets into the image through a build arg derived from the same file. It exists because v2.2.0 and v2.3.0 both died in that job on a Windows-only dead-code lint that no other gate could see. In the "Why a bare ... cannot certify" paragraph (line ~134), replace the gate count and its parenthesised list with "seven gates (tag/version match, MSRV floor, release build, tests, clippy, package verify, cross-target check + clippy)". Add that a bare run SKIPs gate 7 for any target not installed with `rustup target add`. Change nothing else in CLAUDE.md.

docs/DEVELOPMENT.md:
- In Release Process step 3 (line ~308), add the cross-target check + clippy to the list of what the container rehearsal runs.
- In the bullet at line ~294 about cfg-gating Linux-only code, note that `./scripts/pre-tag-check.sh` gate 7 runs the same per-target check locally. Name the rule the fix applied: gate a test-only helper to exactly its callers' cfg, never a blanket `allow(dead_code)`.

Commit as `docs(quick-261110-tbn): seven-gate release process docs`.

Then run the end-to-end rehearsal ONCE. The `<automated>` block below contains it (`./scripts/pre-tag-check.sh --container` with no tag, so gate 1 is SKIPPED, logged to a mktemp file). Launch that block itself with run_in_background and wait for its completion notification: cold cross-target dependency checks plus the full suite can take 30-60 min. It must exit 0, and its inner summary must show gate 7 PASS.

If it exits non-zero only because of a gate unrelated to this plan, re-run once. If that failure persists, do NOT fix it here: report it in the SUMMARY with the gate log excerpt.

Afterward `git status --porcelain` must be empty. The bind mount is read-write, so a touched Cargo.lock would show up here. If anything is dirty, report it and do not fold it into a commit.</action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail() { echo "FAIL-$1"; exit 1; }; [ "$(grep -c "seven gates" CLAUDE.md)" -ge 1 ] || fail claude-md; [ "$(grep -c "gate 7" docs/DEVELOPMENT.md)" -ge 1 ] || fail development-md; LOG=$(mktemp); echo "log: $LOG"; ./scripts/pre-tag-check.sh --container >"$LOG" 2>&1; RC=$?; grep -E "^  gate [1-7]  " "$LOG"; [ "$(grep -cE "^  gate [1-7]  " "$LOG")" -ge 7 ] || fail seven-summary-lines; grep -qE "^  gate 7  cross-target check\+clippy +PASS" "$LOG" || fail gate7-not-pass; [ "$RC" -eq 0 ] || fail "container-rc-$RC"; ST=$(git status --porcelain) || fail git-status; [ -z "$ST" ] || fail dirty-tree; echo verify-ok'</automated>
  </verify>
  <done>CLAUDE.md says "seven gates" and describes gate 7. docs/DEVELOPMENT.md names gate 7. One full `./scripts/pre-tag-check.sh --container` run exits 0 with seven summary lines and gate 7 PASS, and the run's gate lines are quoted in the SUMMARY. The working tree is clean and the docs commit exists.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| .github/workflows/release.yml → pre-tag-check.sh | Repo file content is parsed in bash, then becomes cargo argv and a docker build-arg |
| pre-tag-check.sh → image build (RUN shell) | PRE_TAG_CROSS_TARGETS is word-split inside a root RUN step |
| local rehearsal verdict → release decision | A false-green gate 7 would send a tag into the CI failure it was meant to prevent |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-261110-tbn-01 | Tampering | matrix parser → cargo argv | medium | mitigate | Targets are validated against the triple regex and check_args tokens against the flag regex. check_args is split with `read -ra` (no glob, no eval) and every value goes to run_gate as a separate argv element. |
| T-261110-tbn-02 | Elevation of privilege | Dockerfile RUN with PRE_TAG_CROSS_TARGETS | medium | mitigate | The dispatch builds the arg only from parser-validated triples. The RUN re-validates the charset `[a-z0-9_.-]` plus space and fails on empty before the unquoted expansion. |
| T-261110-tbn-03 | Repudiation (false certificate) | gate 7 vs CI cross-target-check | high | mitigate | Gate 7 derives its list from release.yml instead of keeping a copy. It refuses an unknown key, zero entries, or drifted Check/if/Clippy lines. A missing target in the container is FAILED, never SKIPPED, and a bare SKIPPED never counts as PASS. |
| T-261110-tbn-04 | Denial of service | cache volume growth from three extra target trees | low | accept | Existing PRE_TAG_CONTAINER_CACHE_MAX_GB cap wipes over-cap volumes before each run; check artifacts only (no link). |
| T-261110-tbn-SC | Tampering | rustup target add in the image | low | accept | No npm/pip/cargo package is added. rust-std for official targets comes from the same rustup dist server the image already trusts for its toolchains. |
</threat_model>

<verification>
- Task 1: the release.yml-derived per-target loop of the job's exact Check and Clippy commands exits 0 for all three targets. Before the fix, the Windows clippy failed at src/ui/screens/detail.rs:20714. Linux clippy and the four backlog_one_off tests pass.
- Task 2: the scratch-copy behavior matrix passes. Valid input reaches the runtime probe; unknown-key, run-line-drift and zero-entry inputs are refused before it; the bare run derives the three exact Check lines. The image build is refused without the arg and with it installs all three targets. In release.yml only comment lines changed.
- Task 3: `./scripts/pre-tag-check.sh --container` exits 0 with gate 7 PASS; the docs grep gates hold; `git status --porcelain` is empty.
- Diff guards: no `allow(dead_code)` added; nothing under src/multiplexer/ changed; no version bump, no tag, no push.
</verification>

<success_criteria>
- All three release.yml cross-target matrix targets pass `cargo check --locked --target T --all-targets` and `cargo clippy --locked --target T --all-targets -- -D warnings`.
- `./scripts/pre-tag-check.sh`, bare and `--container`, runs a gate 7 that mirrors that matrix from release.yml, and a green `--container` run now implies a green cross-target-check job.
- The container image installs the derived rustup targets. CLAUDE.md and docs/DEVELOPMENT.md describe the seven gates.
- Three atomic commits (fix, feat, docs); clean working tree; nothing pushed.
</success_criteria>

<output>
Create `.planning/quick/261110-tbn-fix-windows-only-dead-code-clippy-error-/261110-tbn-SUMMARY.md` when done. Include the commit table, the per-target exit codes from Task 1, the Task 2 behavior-matrix results, the gate summary lines of the Task 3 `--container` run, and the inferred decisions copied from the objective, each still marked [inferred — audit].
</output>
