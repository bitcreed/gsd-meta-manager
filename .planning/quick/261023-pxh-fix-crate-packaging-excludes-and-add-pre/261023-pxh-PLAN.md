---
phase: quick-261023-pxh
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - scripts/pre-tag-check.sh
  - CLAUDE.md
  - .github/workflows/release.yml
autonomous: true
requirements: [QUICK-261023-pxh]

estimate:
  tokens: 70000
  raw_tokens: 70000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "`cargo package --list --locked --allow-dirty` lists src/driver_templates/assets/DIRECTION.md, src/driver_templates/assets/LOG.md and src/launcher/assets/MASTER-PROMPT.md. It also lists every git-tracked file under src/ and tests/, which covers all 58 distinct `include_str!`/`include_bytes!` targets the planner resolved: every one of them lies under src/ or tests/. It lists no path under the ROOT assets/ or ROOT .planning/ directories, and no ROOT CLAUDE.md."
    - "`cargo package --locked --allow-dirty` exits 0 on the fixed tree. That is the build-and-verify phase `cargo publish --locked` runs before uploading. At base commit 6c805ad the same command dies with `failed to verify package tarball`, because the packaged crate lacks the three asset files."
    - "scripts/pre-tag-check.sh runs a gate 6, `cargo package --locked --allow-dirty`, after gate 5. It runs in bare mode, and in container mode through the existing inner re-run, which is the same script. Its result is the sixth line of the PRE-TAG SUMMARY. A FAILED gate 6 makes the exit code 1. It never publishes and never reads or forwards CARGO_REGISTRY_TOKEN."
    - "Gate 6 always passes `--allow-dirty`. Release step 3 runs before the step-5 release commit, so Cargo.toml and Cargo.lock are uncommitted by design. Container mode bind-mounts the same working tree rather than copying it, so both modes see the same dirty state. [INFERRED — audit]"
    - "After gate 6, the script prints a dirty-tree advisory: the `git status --porcelain --untracked-files=all` lines for src, tests, Cargo.toml and Cargo.lock. On a PASS with dirty paths, the summary note counts them. The advisory never changes the gate verdict. [INFERRED — audit]"
    - "Gate 6 uses neither `--offline` nor an offline fallback. CARGO_NET_OFFLINE=true is documented as cargo's own switch for a bare run on a warm cache, in the script header Environment section and in usage(). [INFERRED — audit]"
    - "The gate count reads six everywhere it is stated: the script header, the `set -u` rationale, the trap-placement comment, usage() (which gains a run-order gate list) and CLAUDE.md Release Process step 3. In release.yml only a comment changes: the Publish step gains a cross-reference to gate 6."
    - "Live evidence: `./scripts/pre-tag-check.sh --container v1.15.0` exits 0. The inner summary shows all six gates PASS, and gate 6 carries no dirty-tree note."
  artifacts:
    - path: "Cargo.toml"
      provides: "root-anchored package excludes"
      contains: "\"/assets/\""
    - path: "scripts/pre-tag-check.sh"
      provides: "gate 6 (cargo package --locked --allow-dirty), dirty-tree advisory, six-line summary, docs"
      contains: "gate6-package.log"
    - path: "CLAUDE.md"
      provides: "Release Process step 3 updated to six gates, with the package-verify gate described"
      contains: "six gates"
    - path: ".github/workflows/release.yml"
      provides: "comment-only cross-reference from the Publish step to pre-tag-check gate 6"
      contains: "pre-tag-check.sh gate 6"
  key_links:
    - from: "Cargo.toml exclude"
      to: "cargo package file list, then the verify build of the include_str! targets"
      via: "gitignore-style patterns; a leading / anchors an entry to the package root"
      pattern: "\"/assets/\""
    - from: "run_gate \"gate6-package.log\""
      to: "print_summary gate_line 6 and the OVERALL_RC loop"
      via: "G6_STATUS / G6_NOTE, initialised to NOT REACHED before the EXIT trap is installed"
      pattern: "G6_STATUS"
    - from: "container dispatch CONTAINER_INNER"
      to: "gate 6 inside the ubuntu-24.04 lookalike"
      via: "the inner run is this same script in bare mode, so gate 6 needs no container-specific code"
      pattern: "CONTAINER_INNER"
---

<objective>
Fix the crate packaging that killed the v1.15.0 publish job, and add a pre-tag gate that would have caught it.

The publish job's `cargo publish --locked` failed at `failed to verify package tarball`. The packaged crate lacked three `include_str!` targets: src/driver_templates/assets/DIRECTION.md, src/driver_templates/assets/LOG.md and src/launcher/assets/MASTER-PROMPT.md. The cause is Cargo.toml line 23, `exclude = [".planning/", "CLAUDE.md", "assets/"]`. Cargo reads these as gitignore-style patterns, so the unanchored `assets/` matches EVERY directory named `assets`, not only the root screenshots directory.

Purpose: the next tag publishes. scripts/pre-tag-check.sh, in bare and container modes, then rehearses the publish job's build-and-verify-the-tarball step too, not only build, test and clippy.

Output: anchored excludes in Cargo.toml; gate 6 in scripts/pre-tag-check.sh, with summary, help and header docs; CLAUDE.md step 3 updated; a comment-only cross-reference in .github/workflows/release.yml; and a green `--container v1.15.0` run as evidence.

Planner measurements (cargo 1.99.1, taken in an isolated `git archive HEAD` copy; the repo was not mutated):
- At HEAD the package list has 323 files and misses exactly the three assets above. With root-anchored entries it has 335 files: those three, plus nine fixture files that were also being dropped — the eight files under tests/fixtures/injection-corpus/.planning/ and tests/fixtures/injection-corpus/CLAUDE.md.
- The anchored package is 13.0 MiB unpacked and 3.4 MiB compressed, under crates.io's 10 MB limit.
- Root assets/ (screenshots), root .planning/ (1372 tracked files) and root CLAUDE.md all stay out.
- 58 distinct `include_str!`/`include_bytes!` targets resolve, covering single-line literals, multi-line calls and the `concat!(env!("CARGO_MANIFEST_DIR"), ...)` form in tests/executor_transport.rs. All 58 lie under src/ or tests/, so "every tracked src/ and tests/ file is packaged" is a sufficient (superset) check.
- `cargo package` refuses a dirty tree, even with `--list`. It packages untracked files that are not gitignored. It contacts the crates.io index even with `--no-verify` on a warm cache. `CARGO_NET_OFFLINE=true cargo package --locked --no-verify` succeeds on this machine's warm cache.
- Docker is usable here. No gsdmm-pre-tag-check image or gsdmm-pretag-* volume exists (quick 261023-p51 finished with storage clean), so Task 3's container run is fully COLD.

Execution note [INFERRED — audit]: run this quick task WITHOUT worktree isolation. It is a single plan with no parallelism to gain. Task 3's container run bind-mounts only the repo root, so from a linked worktree (whose `.git` is a file pointing outside the mount) git, cargo's VCS file listing and gate 6's advisory cannot work inside the container.

Constraints: human unavailable. Decide from code and mark inferred decisions `[INFERRED — audit]` in the SUMMARY. Do NOT push and do NOT create, move or delete any tag; a local v1.15.0 tag exists and stays where it is. Every commit carries the trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

RTK note: this machine's Bash-tool hook rewrites cargo and grep and filters their output. Every automated verify command below is therefore wrapped in `rtk proxy bash -c '...'`, so raw output reaches the checks. Drop the `rtk proxy` prefix only if rtk is not installed. When a count looks wrong, re-run under `rtk proxy` before believing it.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@Cargo.toml
@scripts/pre-tag-check.sh
@.github/workflows/release.yml

Landmarks in scripts/pre-tag-check.sh at base 6c805ad:
- lines 3-8: the header sentence listing the reproduced gates
- line 63: the header Usage line
- lines 65-70: the `set -u` rationale, which names the gate range that must all run
- lines 74-116: `usage()`
- lines 171-185: the container-dispatch comment explaining why the dispatch exits before the EXIT trap; it states the trap's summary gate count
- lines 494-527: CONTAINER_INNER and the inner `docker run`; the inner run is this script in bare mode, with CARGO_TARGET_DIR=/cargo-target
- lines 563-572: status initialisation
- lines 580-629: `gate_line` and `print_summary`
- lines 870-887: gate 5 and the OVERALL_RC loop
- each existing gate uses the form `if run_gate "gateN-name.log" <cmd>; then G<N>_STATUS="PASS" else G<N>_STATUS="FAILED"; G<N>_NOTE="see gateN-name.log" fi`

scripts/pre-tag-check.Dockerfile is NOT modified by this plan.
</context>

<tasks>

<task type="tracer">
  <name>Task 1: Anchor the crate excludes, prove the publish verify passes, and wire gate 6 into the pre-tag summary</name>
  <files>Cargo.toml, scripts/pre-tag-check.sh</files>
  <action>
End-to-end slice: manifest fix, then the CI-equivalent package verify, then the gate wired into the pre-tag script's verdict.

1. Cargo.toml line 23. Change the three exclude entries to root-anchored form: `"/.planning/"`, `"/CLAUDE.md"` and `"/assets/"`, in that order.
   - Above the line, add a short `#` comment in the manifest's existing explanatory style. It states that cargo matches exclude entries gitignore-style, so a leading `/` anchors an entry to the package root.
   - The comment records that the unanchored `assets/` also matched src/driver_templates/assets/ and src/launcher/assets/. Those hold `include_str!` targets, and the v1.15.0 publish died at `failed to verify package tarball`.
   - It notes that anchoring also starts shipping the tests/fixtures/injection-corpus/.planning/ files and that directory's CLAUDE.md. Those are synthetic test fixtures, already public in the GitHub repo, which the crate's own tests read. [INFERRED — audit]
   - Do NOT switch to an `include = [...]` allow-list. It would change what ships beyond this bug and would need upkeep every time a top-level directory is added. [INFERRED — audit]

2. Prove the list. Run `cargo package --list --locked --allow-dirty`. The flag is needed only because Cargo.toml is uncommitted at this moment.
   - Confirm the three asset files are present.
   - Confirm every `git ls-files src tests` path is present.
   - Confirm no listed path starts with `assets/` or `.planning/`, and none equals `CLAUDE.md`.
   - Optional cross-check: resolve every `include_str!`/`include_bytes!` literal relative to its source file's directory. Include the multi-line calls and the CARGO_MANIFEST_DIR `concat!` form in tests/executor_transport.rs. Confirm the result is still 58 targets, all packaged. Record the count in the SUMMARY.

3. Prove the verify phase. Run `cargo package --locked --allow-dirty`. It builds the .crate and then compiles the unpacked tarball, exactly the step that failed in CI. It must exit 0, and its output must show the Verifying and Packaged lines.
   - It may recompile much of the dependency graph, so allow up to 20 minutes. Run it in the background if needed.
   - Record the `Packaged N files` line in the SUMMARY.

4. Commit Cargo.toml: `fix(quick-261023-pxh): anchor crate excludes so src/**/assets ship in the package`, with the trailer.

5. Wire gate 6 into scripts/pre-tag-check.sh, as the minimal skeleton Task 2 expands.
   - Add `G6_STATUS="NOT REACHED"` and `G6_NOTE=""` to the status-initialisation block, beside G5. The EXIT trap reads them, so they must be set before it is installed.
   - Add `gate_line 6 "cargo package --locked" "$G6_STATUS" "$G6_NOTE"` directly after gate 5's line in `print_summary`. The label is 22 characters and fits the existing `%-26s` column.
   - After the gate 5 block and before the OVERALL_RC loop, add a `# GATE 6` comment banner in the existing style. Then add `section "GATE 6 — cargo package --locked (the publish step's tarball verify; never uploads)"`.
   - Then add `if run_gate "gate6-package.log" cargo package --locked --allow-dirty; then`, setting G6_STATUS to PASS. Otherwise set it to FAILED, with G6_NOTE "see gate6-package.log". This mirrors the other gates exactly.
   - Append `"$G6_STATUS"` to the `for st in ...` OVERALL_RC loop.
   - The banner comment states:
     - it mirrors the publish job's Publish step, `cargo publish --locked`, up to and excluding the upload;
     - `cargo package` assembles the .crate from the manifest's file list and then builds the unpacked tarball, which is the step that failed v1.15.0;
     - the subcommand has no upload path, needs no token and no crates.io account;
     - it sits after clippy because Publish follows Clippy in release.yml;
     - its output (the .crate and the unpacked tree) lands under the target dir's package/ subtree. That is target/ in bare mode and /cargo-target/ in container mode, inside the capped cache volume. The script does not delete it: release.yml drops target/package only for the rust-cache cleaner, which never runs here; cargo overwrites it on the next run; and the .crate is useful evidence. [INFERRED — audit]

6. Run `bash -n scripts/pre-tag-check.sh`. Commit: `feat(quick-261023-pxh): add pre-tag-check gate 6 — cargo package --locked`, with the trailer.
  </action>
  <verify>
    <automated>rtk proxy bash -c 'set -eo pipefail; export LC_ALL=C; L="${TMPDIR:-/tmp}/pxh-pkg.txt"; cargo package --list --locked --allow-dirty > "$L.raw"; sort "$L.raw" > "$L"; T=$(git ls-files src tests) || exit 1; M=$(printf "%s\n" "$T" | sort | comm -23 - "$L"); if [ -n "$M" ]; then echo "MISSING FROM PACKAGE: $M"; exit 1; fi; for f in src/driver_templates/assets/DIRECTION.md src/driver_templates/assets/LOG.md src/launcher/assets/MASTER-PROMPT.md; do grep -qxF "$f" "$L"; done; if grep -nE "^(assets/|\.planning/|CLAUDE\.md$)" "$L"; then echo "ROOT-EXCLUDED PATH PACKAGED"; exit 1; fi; S=scripts/pre-tag-check.sh; bash -n "$S"; grep -qE "^[[:space:]]*if run_gate \"gate6-package\.log\" cargo package --locked --allow-dirty; then" "$S"; grep -qF "gate_line 6 \"cargo package --locked\"" "$S"; grep -qE "^G6_STATUS=\"NOT REACHED\"" "$S"; grep -qE "^for st in .*G6_STATUS" "$S"; cargo package --locked --allow-dirty'</automated>
    Baseline at 6c805ad (planner-run): the `comm` step prints 12 missing paths and exits 1. They are the three asset files plus the nine tests/fixtures/injection-corpus files that the unanchored `.planning/` and `CLAUDE.md` entries also dropped. The gate-6 greps find nothing, and the final command fails with `failed to verify package tarball`. Positive control (planner-run, in an anchored `git archive` copy): every list check passes.
  </verify>
  <done>
- Cargo.toml excludes are root-anchored, with an explanatory comment.
- The package list contains every tracked src/ and tests/ file, including the three asset files, and no root assets/, .planning/ or CLAUDE.md path.
- `cargo package --locked --allow-dirty` exits 0.
- Gate 6 exists in the script, initialised to NOT REACHED, printed as the sixth summary line and counted in the exit code.
- Two commits exist, each with the trailer.
  </done>
</task>

<task type="auto">
  <name>Task 2: Dirty-tree advisory, offline policy, and every doc that states the gate count</name>
  <files>scripts/pre-tag-check.sh, CLAUDE.md, .github/workflows/release.yml</files>
  <action>
1. Dirty-tree advisory, in scripts/pre-tag-check.sh, directly after gate 6's run_gate if/else. It runs whatever the verdict is. [INFERRED — audit]
   - Capture `git status --porcelain --untracked-files=all -- src tests Cargo.toml Cargo.lock`. Use exactly that pathspec: those are the packaged paths whose content changes what compiles.
   - If git itself fails (no repository, or a linked worktree seen from inside the container), print one line saying the dirty state is unknown because git status failed. Leave the verdict and note alone.
   - If the output is non-empty:
     - print it under a short heading. The heading says `--allow-dirty` packaged these uncommitted paths, and that the publish job packages only what is COMMITTED at the tag.
     - point out that an untracked (`??`) file the crate needs passes this gate and fails the publish job.
     - when G6_STATUS is PASS, set G6_NOTE to `packaged N uncommitted path(s); see gate output`, where N is the line count.
   - If the output is empty, print nothing extra.
   - Add a comment saying the advisory never alters the verdict, the same principle as the DEC-5 comment under gate 4.

2. Add a comment block in gate 6 explaining the `--allow-dirty` decision. [INFERRED — audit]
   - CLAUDE.md Release Process step 3 runs BEFORE the step-5 release commit, so the step-1 version bump and the step-2 `cargo update` are uncommitted by design. Without the flag, the gate would fail on every correctly run release.
   - The container dispatch bind-mounts the same working tree at /work and does not copy it, so the dirty state is identical in both modes and one flag serves both.
   - Every other gate also tests the working tree, not HEAD.

3. Add a comment block in gate 6 explaining the network decision. [INFERRED — audit]
   - No `--offline` flag and no retry-offline fallback.
   - Measured: `cargo package` contacts the crates.io index even on a warm cache, unlike the check, build, test and clippy gates, and the publish job runs online. A fallback would be a second code path whose green CI never takes.
   - The escape hatch for a bare offline run on a warm cache is cargo's own `CARGO_NET_OFFLINE=true`, which every cargo gate honours. The container passes only explicit `-e` variables and needs the network anyway for its image build and oracle install.

4. Script header and usage(). Update every place that states the gate count or list.
   - Header lines 3-8: add "package verify" to the parenthesised list of reproduced gates.
   - The `set -u` rationale: the gate range that must all run becomes 2 through 6.
   - The container-dispatch comment above the trap: the trap's summary gate count becomes six.
   - Header Environment section: add a CARGO_NET_OFFLINE entry. Say it is cargo's own variable, passed through in bare mode only, and why (step 3).
   - usage(): add a `Gates (run order):` block after the opening sentence, one line per gate 1-6. Gate 6's line says it builds and verifies the packaged .crate the way the publish job's `cargo publish --locked` does before uploading, that it never publishes, and that it needs no token.
   - usage() Environment list: add the CARGO_NET_OFFLINE entry.
   - Leave the line ~506 comment about which gates can touch Cargo.lock unchanged. Gate 6 uses `--locked`, so it cannot write Cargo.lock, and that comment stays accurate.
   - Finally, scan the whole script for any other gate count or range that the sixth gate makes stale, and fix it.

5. CLAUDE.md, Release Process step 3 only. In the "Why a bare ..." paragraph, the gate sentence must read: `It reproduces the same six gates (tag/version match, MSRV floor, release build, tests, clippy, package verify)`. The rest of the sentence stays as it is.
   - Add one sentence to step 3 about gate 6. It runs `cargo package --locked`, which builds and verifies the packaged crate the way `cargo publish --locked` does before uploading. That is the step that killed the v1.15.0 publish, when an unanchored `assets/` exclude dropped three `include_str!` targets from the tarball. It never publishes and needs no token.
   - Touch nothing else in CLAUDE.md.

6. .github/workflows/release.yml: a COMMENT-ONLY change. Directly above the `- name: Publish` step, add `#` lines at the step's indentation, in the style of the existing Test-step comment.
   - They say `scripts/pre-tag-check.sh gate 6` runs `cargo package --locked`, this step's pre-upload build-and-verify, before the tag exists.
   - They say v1.15.0 died here at `failed to verify package tarball`.
   - No step, key or value changes. [INFERRED — audit]

7. Run `bash -n scripts/pre-tag-check.sh` and confirm `./scripts/pre-tag-check.sh --help` exits 0. Commit: `docs(quick-261023-pxh): gate 6 dirty-tree advisory, offline policy and six-gate docs`, with the trailer.
  </action>
  <verify>
    <automated>rtk proxy bash -c 'set -eo pipefail; S=scripts/pre-tag-check.sh; H="${TMPDIR:-/tmp}/pxh-help.txt"; bash -n "$S"; "./$S" --help > "$H"; grep -qF "cargo package --locked" "$H"; grep -qF CARGO_NET_OFFLINE "$H"; grep -qF CARGO_NET_OFFLINE "$S"; if grep -nE "five-gate|gates 2-5" "$S"; then echo "STALE GATE COUNT IN SCRIPT"; exit 1; fi; grep -qF "six gates" CLAUDE.md; if grep -nF "five gates" CLAUDE.md; then echo "STALE GATE COUNT IN CLAUDE.md"; exit 1; fi; grep -qF "pre-tag-check.sh gate 6" .github/workflows/release.yml; grep -v "^[[:space:]]*#" "$S" | grep -qF "git status --porcelain --untracked-files=all -- src tests Cargo.toml Cargo.lock"; D=$(git diff 6c805ad -- .github/workflows/release.yml) || exit 1; NC=$(printf "%s\n" "$D" | grep -E "^[+-][^+-]" | grep -vE "^[+-][[:space:]]*#" || true); if [ -n "$NC" ]; then echo "NON-COMMENT release.yml CHANGE: $NC"; exit 1; fi'</automated>
    Baseline at 6c805ad: --help mentions neither cargo package nor CARGO_NET_OFFLINE. The script still carries the old gate count (line 175) and range (line 66). CLAUDE.md line 135 carries the old count and lacks "six gates". release.yml lacks the cross-reference. Every deliverable check here is red at HEAD.
  </verify>
  <done>
- Gate 6 prints the dirty-tree advisory and notes it in the summary on PASS, without ever changing the verdict.
- The `--allow-dirty` and offline decisions are explained in comments.
- `--help` lists all six gates and CARGO_NET_OFFLINE.
- No stale gate count remains in the script or in CLAUDE.md step 3.
- release.yml changed in comments only.
- One commit exists, with the trailer.
  </done>
</task>

<task type="auto">
  <name>Task 3: Live acceptance — `./scripts/pre-tag-check.sh --container v1.15.0` passes all six gates</name>
  <files>scripts/pre-tag-check.sh (only if gate 6's own block needs a fix), Cargo.toml (only if the package list itself is wrong)</files>
  <precondition>`docker info` succeeds, `.git` at the repo root is a directory (main checkout, not a linked worktree), and `git status --porcelain -- src tests Cargo.toml Cargo.lock` is empty because Tasks 1-2 are committed.</precondition>
  <action>
1. Before the run, record `docker system df -v` lines for gsdmm-pretag-target, gsdmm-pretag-cargo-home and the gsdmm-pre-tag-check image. The planner expects none to exist.

2. Run `./scripts/pre-tag-check.sh --container v1.15.0` from the repo root.
   - Send its FULL output to `${TMPDIR:-/tmp}/pxh-container.log`. Never read the whole log back: grep and tail it.
   - This run is COLD: image build, then the cold cargo caches for MSRV check, release build, about 2100 tests, clippy and the package verify. Start it as a background command with a timeout of at least 120 minutes, wait for it to exit, then inspect.
   - Record the exit code and the wall-clock time.

3. Expected results:
   - the inner reconciliation banner reads `MATCH: this tree ...`;
   - the inner summary has six lines `  gate N  ...  PASS`, with gate 1 PASS against v1.15.0, which matches Cargo.toml 1.15.0;
   - gate 6's streamed output shows cargo's Verifying and Packaged lines;
   - gate 6's summary line carries NO dirty-tree note, because the tree is clean;
   - the final banner reads `inner run exited 0`, and the exit code is 0;
   - afterwards, `git status --porcelain -- Cargo.toml Cargo.lock` is still empty.

4. If gate 6 FAILS:
   - diagnose it from the gate-6 section of the log;
   - fixes are allowed only in gate 6's block of scripts/pre-tag-check.sh, or in the Cargo.toml exclude (after a Cargo.toml fix, re-run Task 1's verify);
   - commit any fix as `fix(quick-261023-pxh): ...` with the trailer, then re-run step 2.

5. If any of gates 1-5 FAILS:
   - do NOT modify gates, tests, the Dockerfile or the git-version constant. That is out of this plan's scope.
   - extract the failing test names, or the failing command, from the log and stop with a blocker report in the SUMMARY.
   - one known way this cold rebuild can fail [INFERRED — audit]: the image now pulls whatever git ppa:git-core/ppa currently ships. If that is no longer 2.55.0, the banner reads MISMATCH and gate 4's version witness fails. Report that as the cause rather than working around it.

6. Record in the SUMMARY:
   - the six inner summary lines;
   - the gate-6 `Packaged N files` line;
   - the `post-run cache size` lines;
   - the before/after `docker system df -v` lines;
   - the exit code and wall-clock time.

7. Storage decision [INFERRED — audit].
   - Leave the two pretag volumes and the image in place: they are bounded by the 20 GB default cap, and the operator's next release run benefits from a warm cache.
   - Exception: if a post-run size line says a volume is over the cap, it will be wiped on the next run anyway, so keeping it buys nothing. In that case run `./scripts/pre-tag-check.sh --clean` and record that you did.
   - Never touch gsdmm-target, gsdmm-cargo or any other volume.

8. Do not push, and do not create, move or delete any tag. The SUMMARY's next-steps section must say that the local v1.15.0 tag still points at the pre-fix release commit 450713f. Republishing needs an operator decision between moving that tag and cutting v1.15.1, and that is outside this task.
  </action>
  <verify>
    <automated>rtk proxy bash -c 'L="${TMPDIR:-/tmp}/pxh-container.log"; test -s "$L" || exit 1; for n in 1 2 3 4 5 6; do grep -qE "^  gate $n  .*PASS" "$L" || { echo "gate $n not PASS"; exit 1; }; done; grep -qE "^  gate 6  cargo package --locked +PASS" "$L" || exit 1; if grep -qE "^  gate 6  cargo package --locked +PASS +\(" "$L"; then echo "UNEXPECTED DIRTY NOTE"; exit 1; fi; grep -qF "MATCH: this tree" "$L" || exit 1; grep -qF "inner run exited 0" "$L" || exit 1; GS=$(git status --porcelain -- Cargo.toml Cargo.lock) || exit 1; test -z "$GS"'</automated>
    Baseline: the log does not exist before this task, so the check is red until the run completes green.
  </verify>
  <done>
- `./scripts/pre-tag-check.sh --container v1.15.0` exited 0, with six PASS lines in the inner summary, gate 6 PASS with no dirty note, and a MATCH banner.
- Cargo.toml and Cargo.lock are unchanged by the run.
- The evidence and the storage decision are recorded in the SUMMARY.
- No push happened and no tag was touched.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| repo working tree → crates.io tarball | Whatever the exclude list lets through is published publicly and immutably |
| pre-tag-check.sh → cargo package | A local rehearsal of the publish step; it must never reach the upload |
| host → rehearsal container | The repo is bind-mounted read-write; cargo output lands in the capped cache volume |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-pxh-01 | Information disclosure | Cargo.toml exclude (public, immutable crates.io tarball) | high | mitigate | The anchored entries keep root .planning/ (1372 tracked planning files), root CLAUDE.md and root assets/ out of the tarball. Task 1's verify fails if any packaged path starts with `assets/` or `.planning/`, or equals `CLAUDE.md`. |
| T-pxh-02 | Information disclosure | Newly shipped tests/fixtures/injection-corpus/.planning/ files and that directory's CLAUDE.md | low | accept | These are synthetic test fixtures, already public in the (public) GitHub repo. Shipping them lets the crate's own tests resolve their inputs. |
| T-pxh-03 | Elevation of privilege | Gate 6 invocation | high | mitigate | Gate 6 runs `cargo package` only, a subcommand with no upload path. The script neither reads nor forwards CARGO_REGISTRY_TOKEN, and the container receives only explicit `-e` variables. Task 1's verify asserts the exact `run_gate "gate6-package.log" cargo package --locked --allow-dirty` line. |
| T-pxh-04 | Repudiation (false green) | `--allow-dirty` packages uncommitted and untracked files that the tag checkout lacks | medium | mitigate | The dirty-tree advisory lists those paths in the gate output and in the summary note. Task 3 asserts that a clean-tree run carries no note. |
| T-pxh-05 | Denial of service | Container cache growth from the package verify build | low | mitigate | Writes land under /cargo-target, which PRE_TAG_CONTAINER_CACHE_MAX_GB already bounds (quick 261023-p51). Task 3 records the post-run sizes and applies the over-cap clean rule. |
| T-pxh-SC | Tampering | npm/pip/cargo installs | low | accept | No new dependency is added, and Cargo.lock is unchanged (`--locked` throughout; Task 3 asserts it stays clean). The container's oracle install is pre-existing and unchanged. |
</threat_model>

<verification>
- Task 1's verify, re-run on the final tree, is green. It covers the package list (every tracked src/ and tests/ file present, root assets/, .planning/ and CLAUDE.md absent) and a passing `cargo package --locked --allow-dirty`.
- Task 2's static checks are green: help text, advisory pathspec, no stale gate count in the script or in CLAUDE.md, release.yml changed in comments only.
- Task 3's container log shows six PASS lines, a MATCH banner and `inner run exited 0`.
- `git log --format=%B 6c805ad..HEAD` shows the Co-Authored-By trailer on every commit this plan created.
- No tag was created, moved or deleted (`git tag -l 'v1.15*'` is unchanged), and nothing was pushed.
</verification>

<success_criteria>
- The crate tarball contains every `include_str!`/`include_bytes!` target, and `cargo package --locked` (the publish job's verify phase) passes.
- scripts/pre-tag-check.sh gate 6 runs that same verify in bare and container modes, appears in the summary and the exit code, and is documented in the header, `--help` and CLAUDE.md step 3.
- A cold `./scripts/pre-tag-check.sh --container v1.15.0` exits 0 with all six gates PASS.
- Every inferred decision is marked `[INFERRED — audit]` in the SUMMARY.
</success_criteria>

<output>
Create `.planning/quick/261023-pxh-fix-crate-packaging-excludes-and-add-pre/261023-pxh-SUMMARY.md` when done. Include:
- the package file counts before and after, and the include-target count;
- the Task 3 evidence block;
- every `[INFERRED — audit]` decision: the exclude anchoring scope and the shipped fixtures, no `include` allow-list, always `--allow-dirty`, the dirty-tree advisory, no `--offline` or fallback, leaving target/package in place, the comment-only release.yml change, and the storage outcome;
- the next step that is the operator's call: re-pointing the local v1.15.0 tag versus cutting v1.15.1.
</output>
