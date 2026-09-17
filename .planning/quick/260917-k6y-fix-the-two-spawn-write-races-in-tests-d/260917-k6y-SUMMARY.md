---
phase: quick-260917-k6y
plan: 01
subsystem: testing
status: complete
tags: [flake, test-synchronisation, driver-reattach, race, deferred-items, m2]

requires:
  - "tests/driver_reattach.rs (the two flaking tests and live_within/gone_within)"
  - "src/driver/liveness.rs::is_run_alive -> probe -> cmdline_names_run (read-only, unchanged)"
  - "src/journal/reader.rs::tail_lines missing-file branch (read-only, unchanged)"
  - "src/journal/writer.rs run.json atomic NamedTempFile::persist (read-only, unchanged)"
provides:
  - "a deterministic tests/driver_reattach.rs: 10/10 isolated green, 5/5 full-suite green"
  - "three reusable bounded artifact-wait helpers in the file's existing poll idiom"
  - "a closed paper record: deferred-items.md driver_reattach half RESOLVED at 5 sites + 1 canonical entry"
  - "pre-tag-check.sh gate 4 down from three known reds to one"
affects:
  - ".planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md"
  - ".planning/todos/completed/2026-08-18-driver-reattach-spawn-artifact-race.md"

tech-stack:
  added: []
  patterns:
    - "wait on the ARTIFACT an assertion reads, never on the process that will eventually write it"
    - "a wait that precedes an assertion must be strictly WEAKER than that assertion (run_json_within is existence-only)"
    - "wall time as a vacuity control: a ~0.5s run of this binary is a failure signature even when it reports ok"

key-files:
  created: []
  modified:
    - "tests/driver_reattach.rs (+263/-69: three helpers, three call sites, one import, rewritten module header)"
    - ".planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md (+221/-0, append-only honoured)"
  moved:
    - ".planning/todos/pending/2026-08-18-driver-reattach-spawn-artifact-race.md -> .planning/todos/completed/ (0 insertions, 0 deletions)"

decisions:
  - "M2 is the mechanism and is now confirmed: /proc/<pid>/cmdline is populated by the kernel at execve, so live_within answered true microseconds after spawn() — before the envelope, the lock, run.json or the first journal record."
  - "The three artifact waits are layered AFTER live_within rather than replacing it: 'the driver came up at all' is still worth proving and its message is the one that should fire when the binary is broken rather than merely slow."
  - "run_json_within waits on EXISTENCE only. Any content predicate could be satisfied by a COMPLETED run and would hollow out the ended_at-is-null assertion two lines later."
  - "journal_record_within tests for a PARSED ParsedLine::Record, never for Ok-ness: tail_lines returns Ok with zero lines for a missing file, so an is_ok() wait would have fixed nothing while looking like it had."
  - "observed_within matches the run ID, not len() == 1, so the wait cannot be satisfied by a different run and the existing exactly-one assertion still does its own work."
  - "Wall time is reported per run as a vacuity control: 10/10 after-runs at ~6.24s, the paced stand-in's full duration, not the ~0.5s failure signature."
  - "M1 and M3 are NOT closed, were never measured to be causes, and are not claimed live."

metrics:
  duration: ~50min
  completed: 2026-09-17

actuals:
  tokens: 49129
  tasks: 3
  commits: 3
  plan_head_before: 018cbb599dd127ebb664d4b1ec0bc71110b9a34d
---

# Quick 260917-k6y: Fix the two spawn/write races in tests/driver_reattach.rs — Summary

The two `driver_reattach` tests no longer race the driver's first write. They now wait on the
artifact each assertion reads instead of on the driver's process, and the record that told every
reader a red here was expected has been closed against this quick id — with its history intact and
with M1 and M3 explicitly left open.

**`actuals.tokens` method, stated so the scale is auditable:** chars/4 over the FULL content of the
files actually changed (`tests/driver_reattach.rs` 37,462 + `deferred-items.md` 157,047 + the moved
todo 2,006 = 196,515 chars → 49,129), matching the scale the plan's `estimate.tokens: 55000` was
derived on. The realized-diff-only figure, for reference, is 39,157 chars → 9,789 — a different
instrument, recorded here rather than substituted so a later calibration is not comparing two
measurement methods.

## Commits

| Commit | Task | What |
|---|---|---|
| `b9f1795` | 1 | `fix(quick-260917-k6y)`: three bounded artifact waits + three call sites + `HashMap` import |
| `b2bcec0` | 2 | `docs(quick-260917-k6y)`: module header rewritten as a closed case, history preserved |
| `7b5e3e8` | 3 | `docs(quick-260917-k6y)`: five RESOLVED pointers + canonical entry; todo `git mv`d to `completed/` |

`git rev-list --count 018cbb5..HEAD` = **3**. `git tag --points-at HEAD` is **empty**.

## The mechanism, confirmed

Both flaking tests spawned the driver and waited on `live_within`, which polls
`liveness::is_run_alive` → `probe` → `cmdline_names_run(pid, run_id)` — a read of
`/proc/<pid>/cmdline` checking it carries `gsd-meta-manager` and the matching `--run-id`, and
nothing else. **The kernel populates `/proc/<pid>/cmdline` at `execve`.** The wait returned `true`
within microseconds of `spawn()`, long before the driver had established its envelope, taken its
lock, persisted `run.json` or emitted one journal record — and the assertions then read those
artifacts. Process liveness is a NECESSARY precondition for "the run record is on disk" and a wildly
INSUFFICIENT one.

## What changed in `tests/driver_reattach.rs`

Three bounded-wait helpers beside `live_within` / `gone_within`, each reusing that loop's exact
shape (`Instant::now() + limit` deadline, 25ms sleep between polls) and each failing with a message
naming the artifact that never arrived. Every call site passes `Duration::from_secs(30)` inline,
matching the three existing `live_within` sites — generous enough for a contended two-core GitHub
`ubuntu-latest` runner, and costing wall time only on a genuine failure.

| Helper | Waits for | Call site |
|---|---|---|
| `observed_within` | a `reconcile_all` scan observing a run whose **id matches** | test 1, between `let fresh = registry(...)` and `reconcile_all` |
| `journal_record_within` | a journal line that **PARSES** as `ParsedLine::Record` | test 1, between the `journal` path binding and `reader::tail_lines` |
| `run_json_within` | the run record to **EXIST** — existence only, no content predicate | test 2, between the `run_json` path binding and `read_to_string` |

Test 3, `a_run_outlives_the_process_that_spawned_it`, is **unchanged**: it asserts on process
liveness and process group and nothing else, so liveness IS its correct synchronisation point. It
was green in every one of the 6 before-runs, 10 after-runs and 5 full-suite runs below.

**Nothing was weakened.** Every assertion that existed in the two tests before this change exists
after it, verbatim in meaning. No `#[ignore]` (`grep -cE '^[[:space:]]*#\[ignore' = 0`), no
`--test-threads` flag, no fixed `sleep`, no `NotFound` swallowed as success. Nothing under `src/`,
`Cargo.toml`, `Cargo.lock`, `.github/` or `tests/envelope_tracer.rs` was touched — verified by
`git diff --stat 018cbb5..HEAD -- src/ Cargo.toml Cargo.lock .github/ tests/envelope_tracer.rs`,
which is empty.

Task 2 was proved comment-only mechanically: the Task-2 diff has **zero** changed lines that do not
begin with `//`.

## Measurement 1 — the fail-first baseline (six isolated runs, before one byte was edited)

`rtk proxy cargo test --test driver_reattach`, back to back, output redirected to a file per run and
read back with the Read tool (never piped through `grep`/`awk`).

| Run | Result | Wall time | Failing tests |
|---|---|---|---|
| 1 | **FAILED. 1 passed; 2 failed; 0 ignored** | **0.53s** | `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired` (`the run record is on disk: Os { code: 2, kind: NotFound }`, line 542) **and** `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` (`exactly one project has a run to observe — left: 0, right: 1`, line 450) — **both arms firing together** |
| 2 | ok. 3 passed; 0 failed; 0 ignored | 6.24s | — |
| 3 | ok. 3 passed; 0 failed; 0 ignored | 6.23s | — |
| 4 | ok. 3 passed; 0 failed; 0 ignored | 6.24s | — |
| 5 | ok. 3 passed; 0 failed; 0 ignored | 6.24s | — |
| 6 | ok. 3 passed; 0 failed; 0 ignored | 6.23s | — |
| **total** | **1 red / 5 green of 6** | reds at ~0.53s, greens at ~6.24s | the recorded signature, exactly |

The red reproduced the documented failure verbatim — the same two panic messages, at the recorded
~0.53s. The rate (1/6) was lighter than the 4/6 round 11 and round 13 each measured, which is
within what a run-to-run-varying flake does; **the sample was not tuned until it went redder.**

## Measurement 2 — the ten isolated after-runs

| Run | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
|---|---|---|---|---|---|---|---|---|---|---|
| Result | ok. 3 passed | ok. 3 passed | ok. 3 passed | ok. 3 passed | ok. 3 passed | ok. 3 passed | ok. 3 passed | ok. 3 passed | ok. 3 passed | ok. 3 passed |
| Wall time | 6.24s | 6.24s | 6.24s | 6.24s | 6.25s | 6.24s | 6.25s | 6.23s | 6.24s | 6.24s |

**10 green of 10. Every run at the paced stand-in's full ~6.24s; not one at the ~0.5s failure
signature.** The wall time is the vacuity control and is why it is reported per run: a ~0.5s run
that reported `ok` would mean a wait had been removed or defeated, and would be reportable as a
failure regardless of exit code. No such run occurred.

## Measurement 3 — five full-suite `rtk proxy cargo test --no-fail-fast` runs

Plain `cargo test` is not acceptable here and was not used: it stops at the first failing BINARY and
the lib target carries the version-witness failure, so the later suites never run.

| Run | passed | failed | ignored | Failing test names (full) |
|---|---|---|---|---|
| 1 | 2120 | 1 | 15 | `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` |
| 2 | 2120 | 1 | 15 | `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` |
| 3 | 2120 | 1 | 15 | `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` |
| 4 | 2119 | **2** | 15 | `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` **and** `a_relocated_copy_of_the_stub_refuses_instead_of_acting` (`tests/envelope_tracer.rs`) |
| 5 | 2120 | 1 | 15 | `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` |

**Both target tests green in all five runs**, by name — as was test 3:

| Run | `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` | `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired` | `a_run_outlives_the_process_that_spawned_it` |
|---|---|---|---|
| 1 | ok | ok | ok |
| 2 | ok | ok | ok |
| 3 | ok | ok | ok |
| 4 | ok | ok | ok |
| 5 | ok | ok | ok |

**The git-version witness is the expected and acceptable red.** Local git is 2.53.0 against
constants re-derived at 2.55.0; it is a schedule aimed at the CI runner's ambient git, not a
regression. It was not touched, not re-derived, and not made to pass.

**Run 4's second failure is REPORTED, NOT ABSORBED.** `a_relocated_copy_of_the_stub_refuses_instead_of_acting`
in `tests/envelope_tracer.rs` is the `Text file busy` write-then-exec race — the OTHER half of the
original two-binary deferred item, a different mechanism in a file this task was forbidden to open
and did not open. It fired once in five full-suite runs (~20%). **It remains OPEN**, is unchanged in
`deferred-items.md`, and the RESOLVED pointer placed at the two-binary table says so explicitly in
its first sentence. It is not a finding produced by this change: it is a known open flake that
happened to fire during this task's verification, and it is recorded here so a reader of the "5/5
green" claim is not misled about what was green.

The failed count is now **1 per run** where the tree's own `--no-fail-fast` baseline (recorded in
STATE.md for quick `260917-ii4`) was **2**.

## Measurement 4 — `./scripts/pre-tag-check.sh`, gate by gate

**Exit code: 1. That is EXPECTED and is not a failure of this task.** The script's gate 4 runs the
test suite, and the git-version witness fails locally by design. The relevant number is that gate 4
now fails on **exactly one** test, down from the **three** recorded for quick `260917-jdi` on this
tree (the version witness plus the two `driver_reattach` flakes). The script was not edited and no
green exit was chased.

| Gate | Result | Detail |
|---|---|---|
| 1 — tag vs `Cargo.toml` version | **SKIPPED** | `Crate version (Cargo.toml): 1.7.0` — "no tag argument given, so there is nothing to compare it to." The script's documented no-argument behaviour; its own summary states plainly that **"SKIPPED is not a failure."** No tag was created. |
| 2 — MSRV (declared floor compiles) | **PASS** | |
| 3 — `cargo build --release` | **PASS** | |
| 4 — `cargo test --no-fail-fast` | **FAILED** | 2120 passed / **1 failed** / 15 ignored. The single failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (`left: "git version 2.53.0"`, `right: "git version 2.55.0"`). `tests/driver_reattach.rs` reported **ok. 3 passed; 0 failed** in 6.24s inside this run. |
| 5 — `cargo clippy -- -D warnings` | **PASS** | |

**Zero gates reported `NOT REACHED`.**

The advisory banner, reported as the advisory it is rather than filtered: the script printed
`installed git (this machine): git version 2.53.0` against
`derived-against constant: git version 2.55.0` from
`CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION` in `src/envelope/policy.rs`, before the gates
and again in the EXIT-trap summary, stating that this run cannot validate that test the way CI will,
that the GitHub `ubuntu-latest` runner's ambient git is the AUTHORITY for it, and that a green local
gate does not guarantee the publish job passes. The summary also records that "the exit code
reflects GATE FAILURES ONLY. The git-version advisory, on its own, never fails this run."

## Measurement 5 — `rtk proxy cargo clippy -- -D warnings`

**Exit 0, no diagnostics.** (`cargo build --tests` also exits 0; the one warning it emits is the
pre-existing `unused_mut` at `tests/envelope_wrapper_class.rs:6127`, which this task does not own
and did not touch.)

## What this does NOT close — M1 and M3

**M1 and M3 remain OPEN, UNMEASURED, and UNCLAIMED.** Neither was ever measured to be a cause of
these failures, neither is claimed live, and neither is closed by this change:

- **M1** — the tests discover runs through a scan that does not stop at a process-group or worktree
  boundary, so a concurrently running sibling driver-spawning binary could in principle be visible
  to them. The `/proc` scan was left exactly as it was. The `21-30` promote condition that aimed at
  M1 ("scoping the scan to the test's own process group or worktree") is **not satisfied and not
  withdrawn** — it simply is not the route that closed this, and its RESOLVED pointer says so.
- **M3** — `isolate_envelope_root()` sets a process-wide environment variable via
  `std::env::set_var` from whichever of the three test threads reaches it first, while the siblings
  are already executing. Untouched.

If a red ever returns to `tests/driver_reattach.rs`, these are the remaining candidates and the
discriminating experiment recorded in round 11's STANDING entry still applies. Both facts are stated
in the rewritten module header AND in the canonical `deferred-items.md` entry, so a reader arriving
from either direction gets the same disclosure.

## The paper record

`deferred-items.md` is append-only by its own repeatedly-stated discipline, and that was honoured
mechanically: **`git diff --numstat` reports 221 insertions, 0 deletions.** Five dated
`> **RESOLVED (2026-09-17, quick 260917-k6y)**` blockquotes were inserted AT the sentences a reader
would otherwise act on, each quoting verbatim what it supersedes, plus one canonical entry appended
at the end of the file:

1. **The two-binary table** — states in its first sentence that exactly ONE row is closed; the
   `tests/envelope_tracer.rs` `Text file busy` row stays OPEN and untouched. The heading is
   unchanged.
2. **The `21-30` promote condition** — aimed at M1, not what closed it.
3. **Round 11's STANDING entry** ("the one place to edit next round") — its discriminating
   experiment predicted this outcome (*"If the isolated rate is non-zero, M2 is live independently
   of M1 and the fix is to synchronise on the written artifact"*), the isolated rate WAS non-zero,
   and the prediction is quoted and honoured.
4. **Round 12's RE-AFFIRMED entry** — its "a green run is NOT evidence the flake is fixed" still
   stands as a general rule, which is why this task measured a fail-first red plus ten greens.
5. **Round 13's RE-AFFIRMED, UNCHANGED entry** — its varying failure count (2/2/1/2/0/2 with no code
   change) is now explained rather than re-recorded.

The pending todo `2026-08-18-driver-reattach-spawn-artifact-race.md` was `git mv`d to
`completed/` with a **0-insertion, 0-deletion** diff, the project's convention for retiring a
satisfied todo. It retires satisfied rather than abandoned: its "Fix direction" section prescribed
*"Synchronise on the artifact rather than the process... Do not add a fixed `sleep`, and do not
serialise the tests with `--test-threads=1`"* — precisely and only what shipped. The canonical entry
discloses that round 11's two path references to it are now stale by one directory, left as written
rather than edited.

The rewritten module header carries: the CLOSED banner dated against this quick id and naming M2;
the `execve` mechanism stated as a mechanism; the three waits and why `run_json_within` is
existence-only; one line each on why not a `sleep` and why not `--test-threads=1`; the
~0.53s-vs-~6.1s tell re-aimed as a DIAGNOSTIC; a labelled `HISTORY` block preserving the
pre-existing-not-a-regression fact, the `--test-threads=1` correction, the small-green-serialised-
sample warning, the which-arm-varies observation and every dated data point (`343c408`, round 11's
three concurrent worktrees, `21-34`'s 2-green/4-red-of-6 and its five green serialised runs, plus
this task's own before/after); the M1/M3 honesty line; and the superseded comment-only sentence
quoted verbatim, kept true OF `21-34` and corrected for today.

## Deviations from Plan

**1. [Reported, not fixed] The Task 3 verify threshold `grep -c 'envelope_tracer' deferred-items.md >= 8` measures 7.**

- **Found during:** Task 3 verification.
- **Measured:** 4 lines mentioned `envelope_tracer` at HEAD `018cbb5`; 7 do now (+3 from the site-1
  pointer and the canonical entry). The plan's `>= 8` threshold was mis-derived by the planner.
- **Why it was not "fixed":** the criterion that threshold stands for — *"the `envelope_tracer` row
  and every sentence about it survive untouched, and at least one pointer says explicitly that its
  half is still open"* — **is fully satisfied.** Survival is proved harder than by a count:
  `git diff --numstat` shows **0 deletions** on the file, so no pre-existing sentence could have
  been changed. The site-1 pointer states the still-open half in its first sentence. Padding the
  document with a further mention purely to clear an arbitrary count would fabricate the
  measurement, which is the exact failure mode this plan's own IN-04 note warns against.
- **Files modified:** none (this is a reporting deviation, not a code change).

**2. [Reported, not absorbed] `tests/envelope_tracer.rs` flaked once during verification.** Detailed
in Measurement 3 above. Out of scope by prohibition 4, left OPEN, file never opened.

**No deviation rules 1-4 fired.** No bug, missing functionality, blocking issue or architectural
question arose. No `src/` file was modified and none needed to be.

## Inferred decisions (human unavailable — audit these)

1. **Ran unisolated on the primary checkout (`master`)**, per the orchestrator's dispatch: the
   repo's `worktree.base-check` reported the known false-negative base divergence (`origin/HEAD`
   pinned at `d580edd` while local HEAD is ahead), so the quick workflow auto-degraded to sequential
   per #1941. No worktree was created, no branch was switched.
2. **Site-3 pointer placed at the top of round 11's STANDING entry** (immediately after its
   self-declared "it is the one place to edit next round" sentence) rather than beside the
   discriminating-experiment paragraph further down — so a reader landing on the entry header sees
   the closure first. The experiment's sentence is quoted verbatim inside the pointer either way.
3. **Gate 1 reported as SKIPPED rather than PASS.** The plan's verification text anticipated "gates
   1/2/3/5 PASS"; run without a tag argument (as required — no tag may be created), gate 1 reports
   `SKIPPED: no tag argument given`. The script's own summary states SKIPPED is not a failure and it
   does not contribute to the exit code. Reported as measured rather than as anticipated.

## Known Stubs

None. No stub, placeholder, `TODO`, `FIXME`, skipped test or unrun `<verify>` was introduced by this
task. Every verification command in the plan was executed and its real output is reported above.

## Threat Flags

None. `tests/driver_reattach.rs`'s three new helpers are read-only pollers
(`fs::read_to_string`, `reader::tail_lines`, `reconcile_all`) confined to the test's own `TempDir`
project root, with the envelope root already redirected by `isolate_envelope_root`. Zero packages
added, removed or version-changed — `git diff --stat` names neither `Cargo.toml` nor `Cargo.lock`,
so T-K6Y-SC's package-legitimacy gate had no input. T-K6Y-02 (an over-broad wait making a test pass
vacuously) was mitigated exactly as the register required: `run_json_within` carries no content
predicate, the fail-first baseline reproduced the red before the change, and the ten-run wall-time
control confirms no run finished at the ~0.5s signature.

## Self-Check: PASSED

All claimed files exist on disk (`tests/driver_reattach.rs`, `deferred-items.md`, the todo at its
new `completed/` path, this SUMMARY) and the pending path is confirmed absent. All three claimed
commits resolve in `git log --all`: `b9f1795`, `b2bcec0`, `7b5e3e8`.
