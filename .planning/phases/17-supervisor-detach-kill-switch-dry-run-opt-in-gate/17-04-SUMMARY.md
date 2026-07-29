---
phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
plan: 04
subsystem: infra
tags: [driver, dry-run, git, refspec, preview, ctrl-02, blast-radius, portability]

requires:
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    plan: 01
    provides: "src/driver/{mod,run}.rs, DriveArgs.dry_run, DrivableProject::from_registry, drive()'s gate-then-branch ordering, tests/fixtures/fake-claude.sh banner conventions"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    plan: 02
    provides: "src/driver/lock.rs::lock_path — the path a preview must be proved never to create"
  - phase: 15-transport-foundation
    provides: "DrivableProject, the capability token the report builder reads a root from"
provides:
  - "`src/driver/dry_run.rs`: the portable (no `cfg`) three-section report builder and renderer"
  - "`SECTION_COMMANDS` / `SECTION_DIFFSTAT` / `SECTION_REFSPECS` — the pinned output contract"
  - "`DryRunReport`, `build_report`, `render`"
  - "`git_ops::current_branch` — `None` as the honest answer for a detached HEAD"
  - "`git_ops::WorkingTreeStat` / `working_tree_stat` — `git diff --stat HEAD` plus a separate untracked list"
  - "`git_ops::PushPreview` / `push_refspecs` — refspecs computed entirely from local git config"
  - "tests/driver_dry_run.rs — the reflog/refs/`.git` zero-write proof and the tripwire zero-spawn proof"
  - "tests/fixtures/fake-claude-tripwire.sh — a stand-in that leaves evidence if it is ever executed"
affects: [17-05 spawn and reattach, 17-07 UI, 18 TUI dry-run surfacing, 19 push policy, 20 decision router]

tech-stack:
  added: []
  patterns:
    - "Pinned output contract: user-visible section headers are `pub const`s a test asserts appear in order, so a disappearing section fails the build rather than passing a smoke test"
    - "`--no-optional-locks` on every read in a path that promises zero writes, because `git diff` opportunistically rewrites `.git/index`"
    - "Three-part before/after fingerprint (reflog, ref listing, `.git` walk with content digests) as the mechanical form of 'wrote nothing'"
    - "Tripwire fixture with a positive control: absence proves nothing until the tripwire has been seen to fire"
    - "Portable module deliberately outside a sibling `#[cfg(unix)]` block, with the reason stated at the declaration"

key-files:
  created:
    - src/driver/dry_run.rs
    - tests/driver_dry_run.rs
    - tests/fixtures/fake-claude-tripwire.sh
  modified:
    - src/state_reader/git_ops.rs
    - src/driver/mod.rs
    - src/journal/mod.rs
    - src/error.rs

key-decisions:
  - "`--no-optional-locks` is set on every preview read: `git diff` calls `refresh_index_quietly()`, which writes `.git/index` when entries are stat-dirty but content-identical — a git write performed by a read, and one that leaves the file the same length"
  - "The `.git` fingerprint carries a content digest as well as a byte length, because the most likely unwanted write is exactly the one a length comparison cannot see"
  - "`DriveError::DryRunUnavailable` was removed rather than left unconstructed: its text said '--dry-run is not implemented yet', which stopped being true"
  - "The section headers are multi-line constants carrying their own explanatory sentences, so pinning the header also pins the honesty statement about why each section is shaped the way it is"
  - "`src/driver/dry_run.rs` sits outside the `#[cfg(unix)]` block: gating it would make the one mode needing no platform facility the one mode a Windows build could not check"

patterns-established:
  - "A negative grep over a doc-comment prohibition is self-defeating — the doc that states 'never do X' contains X; filter comments or the criterion contradicts the instruction that produced it"
  - "A safety fingerprint must be observed detecting a planted write before it is trusted reporting none"

requirements-completed: []

coverage:
  - id: R1
    description: "A dry-run prints three sections — the GSD command sequence, the working-tree diffstat a commit would capture, and the push refspecs — and their order is pinned so a section cannot silently disappear"
    requirement: "CTRL-02"
    verification:
      - kind: integration
        ref: "tests/driver_dry_run.rs#the_dry_run_output_names_the_command_the_diffstat_and_the_refspecs"
        status: pass
      - kind: unit
        ref: "src/driver/dry_run.rs#the_rendered_report_carries_all_three_section_headers_in_order"
        status: pass
      - kind: manual_procedural
        ref: "cargo run -- --config <tmp> drive demo --command '/gsd:progress' --dry-run — exit 0, all three headers, stdout recorded verbatim below"
        status: pass
    human_judgment: false
  - id: R2
    description: "The command section states that one command is the complete and honest sequence for this build rather than implying a sequence it cannot compute"
    requirement: "CTRL-02"
    verification:
      - kind: unit
        ref: "src/driver/dry_run.rs#the_command_section_states_that_one_command_is_the_complete_sequence_for_this_phase"
        status: pass
    human_judgment: false
  - id: R3
    description: "The refspec list is computed locally from git config and no code path invokes git's push subcommand in any form"
    requirement: "CTRL-02"
    verification:
      - kind: unit
        ref: "src/state_reader/git_ops.rs#push_refspecs_renders_the_upstream_pair_under_push_default_simple"
        status: pass
      - kind: unit
        ref: "src/state_reader/git_ops.rs#push_refspecs_prefers_an_explicit_remote_push_refspec_over_push_default"
        status: pass
      - kind: manual_procedural
        ref: "grep -v '^[[:space:]]*//' src/state_reader/git_ops.rs | grep -c 'push --dry-run|\"push\"' == 0"
        status: pass
    human_judgment: false
  - id: R4
    description: "A detached HEAD and a repository with no remote each report no refspec plus a note, never a guessed branch or a guessed `origin`"
    verification:
      - kind: unit
        ref: "src/state_reader/git_ops.rs#push_refspecs_reports_a_detached_head_as_having_no_refspec"
        status: pass
      - kind: unit
        ref: "src/state_reader/git_ops.rs#push_refspecs_reports_no_remote_rather_than_guessing_origin"
        status: pass
    human_judgment: false
  - id: R5
    description: "The untracked list is reported separately from the stat, because `git diff --stat` never lists untracked files"
    verification:
      - kind: unit
        ref: "src/state_reader/git_ops.rs#working_tree_stat_lists_untracked_files_separately_from_the_stat"
        status: pass
    human_judgment: false
  - id: R6
    description: "A dry-run performs zero git writes, proved by comparing reflog output, the full ref listing and the whole `.git` directory listing before and after"
    requirement: "CTRL-02"
    verification:
      - kind: integration
        ref: "tests/driver_dry_run.rs#a_dry_run_leaves_the_git_directory_byte_identical"
        status: pass
      - kind: manual_procedural
        ref: "planted `git tag planted-write` between the two fingerprints; the test failed naming the REF LISTING as the diverging capture, then the plant was reverted"
        status: pass
    human_judgment: false
  - id: R7
    description: "A dry-run spawns no agent, proved by a tripwire program whose evidence file is asserted absent, with a positive control proving the tripwire fires when executed"
    requirement: "CTRL-02"
    verification:
      - kind: integration
        ref: "tests/driver_dry_run.rs#a_dry_run_never_executes_the_agent_program"
        status: pass
    human_judgment: false
  - id: R8
    description: "A dry-run creates no run directory, no journal and no lock file"
    verification:
      - kind: integration
        ref: "tests/driver_dry_run.rs#a_dry_run_writes_no_run_directory_and_no_journal"
        status: pass
      - kind: manual_procedural
        ref: "grep -v '^[[:space:]]*//' src/driver/dry_run.rs | grep -c 'JournalRun|RunLock|ClaudeExecutor' == 0; from_registry at src/driver/mod.rs:111, dry-run branch at :113"
        status: pass
    human_judgment: false

duration: 16 min
completed: 2026-07-29
status: complete
---

# Phase 17 Plan 04: The Load-Bearing Dry-Run Summary

**`drive <alias> --dry-run` now prints what would be issued, what a commit would capture, and what a push would send — the third computed from local git config with no network contact — and the claim that it wrote nothing is settled by comparing the reflog, every ref and the whole `.git` directory before and after, not by asserting it.**

## Performance

- **Duration:** 16 min
- **Started:** 2026-07-29T17:34:00Z
- **Completed:** 2026-07-29T17:50:22Z
- **Tasks:** 2
- **Files modified:** 7 (3 created, 4 modified)

## Accomplishments

- **CTRL-02 ships with all three outputs, not one.** PITFALLS:63 names the failure this plan existed to avoid — *"a dry-run that prints 'would run: /gsd:execute-phase' tells the user nothing about blast radius"* — and PITFALLS:69 names the tell: *"the dry-run output does not include a refspec list"*. The refspec section is present, carries fully-qualified `refs/heads/<src>:refs/<dst>` lines, and its disappearance is now a build failure rather than an unnoticed regression.
- **The preview is honest about what it cannot know.** The commands section says one command *is* the complete sequence for this build because the router is Phase 20 — stated in the pinned header, not buried in a comment. The diffstat section says out loud that it is the working tree, not the diff of an unrun command. Neither is presented as a shortfall to paper over.
- **"Zero git writes" is settled mechanically and was observed detecting a real write.** Three independent captures — reflog, ref listing, and a recursive `.git` walk carrying a content digest per file — and a planted `git tag` between them made the test fail naming the ref listing as the diverging capture. A fingerprint that has only ever been seen agreeing proves nothing.
- **A latent write in the read path was found and closed.** `git diff` calls `refresh_index_quietly()`, which *writes* `.git/index` when it finds entries that are stat-dirty but content-identical. That is a git write performed by a read, in the one code path that promises never to perform one — and because such a rewrite leaves the index the same length, the plan's specified length-only fingerprint would not have caught it. `--no-optional-locks` on every preview read makes D-23 true by construction, and the fingerprint now digests contents as well.
- **The tripwire fired before it was trusted silent.** `a_dry_run_never_executes_the_agent_program` asserts the evidence file is absent, then runs the same fixture directly and asserts it appears. Without that positive control, a fixture with a typo in its path would have "proved" the same thing.

## Task Commits

1. **Task 1: Local-only git reads — working-tree stat and push refspecs** — `937a8d0` (feat)
2. **Task 2: The three-section report, and mechanical proof it wrote and spawned nothing** — `7143d81` (feat)

## Files Created/Modified

**Created**

- `src/driver/dry_run.rs` — Module doc stating D-22's three outputs with the constraint that shapes each, D-24 (stdout, not the journal, not the TUI) and D-23 (proved, not asserted). Three pinned section constants, `DryRunReport`, `build_report`, `render`, and four unit tests.
- `tests/driver_dry_run.rs` — `git_fingerprint` plus `walk`, and the four proofs. Deliberately not `#![cfg(unix)]`; only the tripwire test carries a `#[cfg(unix)]`, with the reason in a comment.
- `tests/fixtures/fake-claude-tripwire.sh` — Writes its evidence file and exits 1. Its banner says it is never supposed to run and that its *absence* is the proof.

**Modified**

- `src/state_reader/git_ops.rs` — A `--no-optional-locks`-based read helper pair, `config_get`/`config_get_all`, `current_branch`, `WorkingTreeStat`/`working_tree_stat`, `PushPreview`/`push_refspecs`, and six unit tests over real temporary repositories.
- `src/driver/mod.rs` — `pub mod dry_run;` outside the `#[cfg(unix)]` block with the portability reason stated; the module doc's "later plans add `dry_run`" sentence retired; the refusal arm replaced by the preview with its ordering rationale.
- `src/journal/mod.rs` — The `dry_run: false` comment rewritten to name plan 17-04 and D-23.
- `src/error.rs` — `DriveError::DryRunUnavailable` removed (variant, `Display` arm) with a comment recording why, and the enum doc updated.

## Decisions Made

- **`--no-optional-locks` on every preview read.** Documented at the helper as load-bearing rather than hygiene, because the next reader will otherwise see it as noise and delete it.
- **The `.git` fingerprint digests contents.** The plan specified `(relative path, byte length)`. An index rewrite that only updates stat data is byte-length-identical, and it is the single most likely unwanted write here — so length alone would have missed precisely the failure the test exists to catch. Still one comparison over a three-part tuple, as the acceptance criterion requires.
- **`DryRunUnavailable` was deleted, not repurposed.** Plan 17-01 introduced it saying "--dry-run is not implemented yet and this build refuses". A variant nothing can construct, whose text is false, is worse than no variant; a preview is now a success path returning `Ok(())`.
- **Section headers are multi-line constants.** Pinning a bare `== Push refspecs ==` line would leave the *explanation* — that the list was computed locally with no network contact — free to drift or vanish. Carrying it inside the constant means the pinning test protects the honesty statement too.
- **`build_report` is exercised in-source through `from_registry`, never the escape hatch.** `tests/spawn_seam_guard.rs` fences `for_testing_bypassing_opt_in` out of `src/` absolutely; the in-source test builds a real temporary directory and a `RegisteredProject` with an opt-in record instead. The integration test, being in `tests/`, uses the hatch legitimately.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] The preview's own git reads could write to `.git`**

- **Found during:** Task 2, while designing the zero-write fingerprint
- **Issue:** `git diff` calls `refresh_index_quietly()`, which writes `.git/index` when it finds entries that are stat-dirty but content-identical. The dry-run path calls `git diff --stat HEAD`, so the mode that promises zero git writes could perform one — non-deterministically, depending on the timestamps of the working tree it was pointed at. The plan's specified fingerprint compares `(path, byte length)`, and an index refresh of that kind is length-identical, so the test as specified would have reported success.
- **Fix:** `--no-optional-locks` on every read in `git_read_raw`, documented at the helper as load-bearing with the mechanism spelled out; and the `.git` walk now carries a content digest per file alongside the length.
- **Files modified:** `src/state_reader/git_ops.rs`, `tests/driver_dry_run.rs`
- **Verification:** `a_dry_run_leaves_the_git_directory_byte_identical` passes, and was observed failing on a planted `git tag` naming the diverging capture.
- **Committed in:** `7143d81`

**2. [Rule 1 - Bug] Task 1's negative-grep criterion contradicts the doc it also requires**

- **Found during:** Task 1 verification
- **Issue:** The criterion is `grep -rn 'push --dry-run|"push"' src/state_reader/git_ops.rs | wc -l == 0`, while the same task requires the function's doc to *state the prohibition on `git push --dry-run` as its first line*. Satisfying the doc requirement necessarily produces one grep hit — the prohibition itself. Raw count is 1, at the doc line.
- **Fix:** No code change; the property the criterion reaches for holds. Verified with the comment-filtering form the phase already uses elsewhere: `grep -v '^[[:space:]]*//' src/state_reader/git_ops.rs | grep -c 'push --dry-run|"push"'` outputs `0` — no executable line invokes git's push subcommand in any form.
- **Files modified:** none
- **Verification:** the comment-filtered grep, plus the four `push_refspecs` unit tests exercising every branch without a network.
- **Committed in:** n/a (verification-only)

**3. [Rule 1 - Bug] The `current_dir` criterion counts two pre-existing call sites**

- **Found during:** Task 1 verification
- **Issue:** `grep -v '^[[:space:]]*//' src/state_reader/git_ops.rs | grep -c 'current_dir' == 0` cannot hold: the pre-existing async `load_git_log` and `load_diff_stat` both use `cmd.current_dir(project_path)`. Actual count is 2, and neither line is this plan's.
- **Fix:** No code change. The intended property — the *new* helpers follow the module's `-C` convention — was verified by scoping to the new section: exactly one `Command::new("git")` in the added code, and it uses `.arg("-C")`.
- **Files modified:** none
- **Verification:** `awk 'NR>=155 && NR<=470' src/state_reader/git_ops.rs | grep -n 'Command::new|arg("-C")|current_dir'` shows one spawn site with `-C` and no `current_dir` outside the section's own explanatory comment.
- **Committed in:** n/a (verification-only)

**4. [Rule 1 - Bug] `DriveError::DryRunUnavailable` became an unconstructible false claim**

- **Found during:** Task 2
- **Issue:** `src/error.rs` is not in this plan's `files_modified`, but replacing the refusal arm leaves a public variant whose doc and message both say `--dry-run` is not implemented. Nothing can construct it and its text is false — the same class of stale claim plans 17-01 and 17-02 each retired.
- **Fix:** Variant and `Display` arm removed; a comment records what lived there and why it went; the enum's own doc now notes that 17-04 removed a placeholder rather than replacing it. Checked first that no sibling plan depends on it — 17-05 is wave 4 and 17-06 wave 5, and neither references it.
- **Files modified:** `src/error.rs`
- **Verification:** `cargo build` clean, full suite green, `grep -rn 'DryRunUnavailable' src/ tests/` empty.
- **Committed in:** `7143d81`

**5. [Rule 2 - Missing Critical] Two extra tests beyond the plan's list**

- **Found during:** Tasks 1 and 2
- **Issue:** `render`'s empty case is where a section would silently vanish, and the plan's named tests all use a populated report. Likewise nothing exercised the non-repository path of `working_tree_stat`, which is the case a preview against a project not under version control hits first.
- **Fix:** Added `every_section_prints_something_even_for_a_clean_tree_with_no_remote` and `working_tree_stat_is_empty_rather_than_erroring_for_a_non_git_dir`, plus `build_report_reads_a_real_project_root_and_carries_the_single_command` so the builder is covered in-source without the escape hatch.
- **Files modified:** `src/driver/dry_run.rs`, `src/state_reader/git_ops.rs`
- **Verification:** all pass; suite total 472.
- **Committed in:** `937a8d0`, `7143d81`

---

**Total deviations:** 5 (1 latent write in the code plus the weakened test that would have missed it, 2 unsatisfiable plan criteria, 1 stale-claim retirement, 1 coverage addition). **Impact:** deviation 1 is the substantive one — without it the plan's headline safety property would have been true only by luck and unverifiable by its own test. No scope creep: the public surface is the plan's symbol list plus nothing, minus one now-false error variant. No dependency added.

## Issues Encountered

**A doc-comment prohibition defeats a negative grep over the same file.** Recorded as deviation 2 and worth carrying forward: whenever a plan asks for both *"state the prohibition in the doc"* and *"grep proves the string is absent"*, the two criteria are in direct conflict unless the grep filters comments. Wave 1 and wave 2 both hit `rtk`-shaped grep problems; this is a different, plan-shaped one.

**`rtk proxy sh -c` for every pipeline, as waves 1–3 all recorded.** Every criterion here that pipes one filter into another was run as `rtk proxy sh -c '<whole pipeline>'`, and every cargo invocation needing raw `warning:`/`test result:` lines as `rtk proxy cargo …`. No criterion in this plan passed vacuously.

**The environment's Bash guard refuses compound `cd`-plus-git commands from a worktree agent.** Building the throwaway repository for the real CLI invocation had to go through a small checked-in-nowhere shell script in the scratchpad rather than an inline compound command. No workaround was needed for anything the tests do.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | clean |
| `rtk proxy cargo test` | **472 passed, 0 failed** (baseline 458: +6 `git_ops` unit, +4 `dry_run` unit, +4 integration) |
| `cargo clippy -- -D warnings` | exits 0 |
| `cargo clippy --all-targets` warning count | **exactly 5**, all pre-existing (`browser.rs` ×3, `project_creator.rs` ×1, `state_reader/mod.rs` ×1); none in `src/driver/`, `src/state_reader/git_ops.rs`, `src/error.rs` or `src/journal/` |
| `rtk proxy cargo test --lib state_reader::git_ops` | 14 passed — all five named tests present |
| `rtk proxy cargo test --lib driver::dry_run` | 4 passed — both named tests present |
| `rtk proxy cargo test --test driver_dry_run` | 4 passed — all four named tests present |
| `rtk proxy cargo test --test driver_tracer` | 4 passed, unchanged |
| `rtk proxy cargo test --test driver_lock` | 4 passed, unchanged |
| `grep -c 'push --dry-run\|"push"'` in `git_ops.rs`, comments filtered | `0` |
| `grep -c 'JournalRun\|RunLock\|ClaudeExecutor'` in `dry_run.rs`, comments filtered | `0` |
| `from_registry` vs the dry-run branch in `src/driver/mod.rs` | `:111` vs `:113` — the gate runs first |
| `grep -c '17-04\|D-23' src/journal/mod.rs` | `2` |
| `test -x tests/fixtures/fake-claude-tripwire.sh` | succeeds |
| `git diff --stat Cargo.toml Cargo.lock` | empty — no dependency added |
| `cargo run -- drive demo --command '/gsd:progress' --dry-run` | exit **0**, all three headers, stdout below |

### The deliverable, verbatim

Against a throwaway repository with one commit, one modified tracked file, one untracked file, `origin` at `https://example.invalid/demo.git`, an upstream and `push.default = simple`:

```
DRY RUN — nothing below was executed. No agent was spawned, no run was
journaled, and no git write was performed.

== GSD commands this run would issue ==
The decision router is Phase 20, so the single --command argument below is
the complete and honest sequence for this build — not a truncated one.
  1 command in the sequence:
    1. /gsd:progress

== Working tree a commit would capture ==
`git diff --stat HEAD` plus untracked files: the state this run would
inherit and could sweep into a `git add -A && git commit`. It is not the
diff of the command — that cannot be known without running it.
   tracked.txt | 1 +
   1 file changed, 1 insertion(+)
  Untracked:
    NOTES.md

== Push refspecs this state would produce ==
Computed locally from git config (branch.<b>.remote, remote.pushDefault,
remote.<r>.push, push.default). No network was contacted, no credential was
used, and git's push subcommand was never invoked in any form.
  Remote: origin
  URL:    https://example.invalid/demo.git
  refs/heads/master:refs/heads/master
  Note: `push.default` is `simple` and the upstream `refs/heads/master` has the same name as the local branch, which is the case `simple` allows.
```

After that invocation the repository's `.planning/` was still empty (no `meta-manager/`, no lock file) and `git status --porcelain` reported exactly the two changes the test had planted.

## Known Stubs

None. `DryRunReport.commands` carries one element because this phase issues exactly one command, and both the field's doc and the rendered header say so; Phase 20's router is what makes it longer. That is a stated scope boundary, not a placeholder.

## User Setup Required

None — no external service configuration required. Every test initialises its own git repository with repo-scoped identity and signing forced off, so they neither depend on nor disturb a developer's global git configuration, and the refspec computation contacts no network and needs no credential by construction.

## Next Phase Readiness

- **17-05 (detached spawn, reattach)** — unaffected by this plan's changes to `drive()`; the preview returns before the dispatch it extends. One note: `src/driver/dry_run.rs` contains no process-spawn marker, so it needs no `SPAWN_ALLOWLIST` entry. `src/driver/spawn.rs` still will.
- **17-06 (kill switch)** — `src/error.rs` is in its `files_modified`; be aware `DriveError::DryRunUnavailable` is gone and the `Display`/`source` matches are one arm shorter.
- **17-07 / Phase 18 (UI)** — `dry_run::build_report` and `render` are `pub` and portable, so a TUI surfacing path needs no new computation; it needs `spawn_blocking`, because both git helpers are synchronous by design.
- **Phase 19 (push policy)** — `push_refspecs` is the read this phase makes visible and Phase 19 makes enforceable. T-17-26 is accepted deliberately: a repository-local config can name any remote and any refspec, and faithfully reporting a configuration the user may not have chosen is exactly what a preview is for.
- **Phase 20 (decision router)** — `DryRunReport.commands` is already a `Vec` and `render` already numbers and counts its entries, so a sequence renders without touching the renderer. The `SECTION_COMMANDS` constant's second sentence is what needs rewriting then, and it is the pinned contract, so that rewrite is a deliberate user-visible change with a test to update.

One forward note: the three section constants are a user-visible output contract pinned by a test. The plan rated this reversibility "costly" and that assessment stands — anyone scripting against the preview reads these headers.

## Requirements Traceability

`CTRL-02` is declared by this plan alone among the phase's plans, and every clause of it — three sections, local computation, zero git writes, zero spawns — is proved above. `REQUIREMENTS.md` is nonetheless **deliberately untouched**, consistently with 17-01, 17-02 and 17-03: this executor runs in a worktree and the orchestrator owns all post-wave shared-file writes.

## Self-Check: PASSED

- All three created files verified present on disk: `src/driver/dry_run.rs`, `tests/driver_dry_run.rs`, `tests/fixtures/fake-claude-tripwire.sh` (the last one executable).
- Both commits verified present in `git log`: `937a8d0`, `7143d81`.
- All Task 1 and Task 2 acceptance criteria re-run after the final task commit; all pass, with the two unsatisfiable grep criteria evaluated in their corrected form and both corrections documented above.
- Plan-level verification re-run: build clean, **472 tests passing** (baseline 458), zero failures, `cargo clippy -- -D warnings` exits 0, `--all-targets` warning count still exactly 5 and all pre-existing, `Cargo.toml` and `Cargo.lock` unchanged.
- Both planted checks verified reverted: `grep -c PLANTED src/state_reader/git_ops.rs` and `grep -c planted-write tests/driver_dry_run.rs` both `0`, and the full suite was re-run green afterwards.
- No modification to `STATE.md` or `ROADMAP.md` — the orchestrator owns those writes.

---
*Phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate*
*Completed: 2026-07-29*
