---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 34
subsystem: record
tags: [flake, driver-reattach, record-correction, staleness-obligation, coverage, merged-tree-gate, proc-scan]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening (21-31)
    provides: "the argv fusion whose dependency-behaviour residual this plan opens as a standing obligation"
  - phase: 21-llm-goal-layer-prompt-injection-hardening (21-32)
    provides: "the two repaired censuses whose narrowed claims and directed residuals this plan records"
  - phase: 21-llm-goal-layer-prompt-injection-hardening (21-33)
    provides: "the narrowed src/ui census and its reach pin, whose bookkeeping-red direction this plan records"
provides:
  - "the driver_reattach flake correction placed AT the stale --test-threads=1 sentence, not only after it"
  - "a committed 388-hit grep inventory with a stated, re-runnable classification predicate and no hit unclassified"
  - "a comment-only module note in tests/driver_reattach.rs, asserted comment-only"
  - "the claude CLI option-parser staleness obligation, six probes re-measured at 2.1.248"
  - "three deliberate non-closures with promote conditions and directions; all 14 anti-pattern rows dispositioned"
  - "the Record-corrections table for the four round-10 truths pass 11 measured FALSE"
  - "ROADMAP criterion 4 verbatim, fourth round, with the permanently-agent-unclosable statement in words"
  - "M3 — a third candidate flake mechanism, named not measured, which makes M1 insufficient on its own"
  - "the merged-tree gate, every figure re-measured, none inherited from a wave-1 worktree"
affects: [phase-21-verification-pass-12, secure-phase-21, future-claude-cli-upgrade, future-driver-reattach-fix]

actuals:
  tokens: 24000
  tokens_note: "13942 = chars/4 over the realized 55766-byte diff (git diff f5b548b HEAD); the balance is this SUMMARY file itself"
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "A record correction is placed AT the sentence it corrects, because a reader who stops at the first section is the reader the wrong sentence reaches"
    - "A correction's completeness is established by a committed inventory with a STATED, RE-RUNNABLE classification predicate — not by an author's sweep"
    - "A dated per-run observation is left byte-unedited and NAMED; only standing claims are corrected, because editing a record of what one run did falsifies history to make a general point"
    - "A plan that measures its own freshly-written claim false corrects it in a separate, legible commit rather than smoothing it"

key-files:
  created: []
  modified:
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/COVERAGE.md
    - tests/driver_reattach.rs

key-decisions:
  - "D-21-61: the flake correction's completeness is established by a committed grep inventory with a stated re-runnable predicate, not by an author's sweep"
  - "D-21-62: dated per-run observations are NAMED and left unedited; only STANDING claims are corrected in place"
  - "D-21-63: a comment-only note is added to tests/driver_reattach.rs, the one file under tests/ this round touches, with the comment-only property ASSERTED rather than promised"
  - "D-21-64: a new standing staleness obligation is opened for the claude CLI's option parser, in the shape of the ratatui one"
  - "D-21-65: COVERAGE.md is amended append-only and its existing declaration is left byte-identical"
  - "Deviation D-21-66: the mechanism record is NOT settled — M1 is measured insufficient on its own and M3 is added, named not measured"

patterns-established:
  - "M1/M2/M3 candidate-mechanism tables with an explicit discriminating experiment, rather than one asserted mechanism"
  - "A green sample under a flag is reported WITH the sample size and WITH the reason it is not a fix, because the stale claim being corrected was itself born of n=3"

requirements-completed: [DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08]

coverage:
  - id: D1
    description: "The driver_reattach flake record says the same true thing in every place a reader will look — corrected AT the stale sentence, with the completeness checkable from a committed inventory"
    requirement: SAFE-07
    verification:
      - kind: other
        ref: "rtk proxy grep -n 'test-threads' deferred-items.md — line 20 (stale) immediately followed by the correction block at lines 26-73"
        status: pass
      - kind: other
        ref: "388-hit inventory: 133 test-threads + 255 driver_reattach; 34 LIVING + 354 DATED = 388, none unclassified"
        status: pass
    human_judgment: false
  - id: D2
    description: "The correction is also where the reader of the FAILING TEST finds it, and that file's diff is comment-only"
    requirement: SAFE-07
    verification:
      - kind: other
        ref: "rtk proxy git diff -- tests/driver_reattach.rs | grep -cvE '^[+-][[:space:]]*//' == 0, both times; git diff --stat -- tests/ names that file and no other"
        status: pass
    human_judgment: false
  - id: D3
    description: "Whether the corrected prose actually stops the next reader reaching for --test-threads=1"
    verification: []
    human_judgment: true
    rationale: "A doc's persuasive adequacy is a prose judgment. The placement, the quoted supersession and the inventory are the parts automation can carry; whether a human reads them is not."
  - id: D4
    description: "Round 11's four closures recorded with residuals and DIRECTIONS, quoted from the delivering SUMMARYs rather than from any plan's intent"
    requirement: SAFE-07
    verification:
      - kind: other
        ref: "deferred-items.md round-11 closures section; every residual traced to 21-31/32/33-SUMMARY.md wording"
        status: pass
    human_judgment: false
  - id: D5
    description: "The claude CLI staleness obligation carries the six probe commands, the measured version, the property, the direction and the re-measure instruction"
    requirement: SAFE-08
    verification:
      - kind: manual_procedural
        ref: "six probes re-run on the merged tree against claude 2.1.248 (Claude Code); probe D again byte-identical to probe C"
        status: pass
    human_judgment: false
  - id: D6
    description: "Every row of pass 11's anti-pattern table dispositioned — closed with citation, or a deliberate non-closure with a reason and a direction"
    requirement: SAFE-07
    verification:
      - kind: other
        ref: "deferred-items.md disposition table: 14 rows, 14 dispositions, IN-05 cited as CLOSED by 21-32 T2"
        status: pass
    human_judgment: false
  - id: D7
    description: "DRIVE-04 boundary and precision reconfirmed as explicit evidence rather than inferred from the round's scope"
    requirement: DRIVE-04
    verification:
      - kind: integration
        ref: "tests/driver_escalation_cap.rs — 8 passed / 0 failed; git diff --stat 2c13fcf HEAD on that file is empty"
        status: pass
    human_judgment: false
  - id: D8
    description: "The round's gate measured on the MERGED tree with every delta attributed, the file fence confirmed, and REQUIREMENTS.md untouched"
    requirement: DRIVE-01
    verification:
      - kind: integration
        ref: "cargo build exit 0; workspace 1422/2/13 across 35 binaries reconciling to 1424 = 1417 + 7; lib clippy 0 errors; --all-targets exactly 4 pre-existing lints"
        status: pass
    human_judgment: false
  - id: D9
    description: "ROADMAP criterion 4 re-surfaced verbatim with no work claimed, and the permanently-agent-unclosable / 4-of-5-ceiling statement made in words"
    verification:
      - kind: integration
        ref: "tests/driver_injection_corpus.rs — 13 passed / 0 failed / 10 ignored; file empty in git diff --stat 2c13fcf HEAD"
        status: pass
    human_judgment: true
    rationale: "The structural half (counts, file unchanged, no work claimed) is measured and passing. The BEHAVIOURAL half needs a human with an authenticated Claude subscription and is permanently agent-unclosable; it is NOT claimed here."
  - id: D10
    description: "The mechanism record's honesty: three candidate mechanisms with a discriminating experiment, rather than one asserted cause"
    verification:
      - kind: other
        ref: "6 isolated runs at default threads on an idle machine: 2 green / 4 red — measured, refuting M1 as a necessary condition"
        status: pass
    human_judgment: true
    rationale: "The RATES are measured. Which mechanism actually causes the failure is not established by this plan and is deliberately not claimed; the discriminating experiment is written down as the promote condition instead."

duration: 30 min
completed: 2026-08-28
status: complete
---

# Phase 21 Plan 34: The Record, Corrected Completely Summary

**The `driver_reattach` flake's superseded mitigation no longer stands first in the file: the correction is placed at the stale sentence, its completeness is bounded by a committed 388-hit inventory with a re-runnable classification predicate, and the note is mirrored into the failing test file with a comment-only diff — and then this plan's own merged-tree gate contradicted a claim it had written hours earlier, which is corrected in a separate commit rather than smoothed.**

## Performance

- **Duration:** ~30 min
- **Tasks:** 3 (plus one in-plan deviation correction)
- **Files modified:** 3
- **Commits:** 4

---

## Task 1 — the flake record, corrected everywhere it is wrong

### (a) The grep inventory, committed

Both commands run under `rtk proxy`, because RTK's global hook strips
`test result:` and `warning:` lines and a count taken through the filtered path
reads low. This mattered more here than anywhere: the whole job of this plan is
to get a count right.

```
rtk proxy grep -rn "test-threads"     .planning/ tests/ src/   ->  133 hits
rtk proxy grep -rn "driver_reattach"  .planning/ tests/ src/   ->  255 hits
                                                        TOTAL  ->  388 hits
```

**The classification predicate, stated as a command rather than as a judgment,**
so the next verifier re-runs it instead of trusting it. A hit is a **DATED
OBSERVATION** if it sits in an executed or superseded artifact — `*-PLAN.md`,
`*-SUMMARY.md`, `*-VERIFICATION.md`, `*-REVIEW.md`, `*-FIXES.md`,
`*-RESEARCH.md`, `*-UAT.md`, `*-PATTERNS.md`, `.continue-here.md` — and
**LIVING** otherwise:

```
rtk proxy grep -vE '(-PLAN\.md|-SUMMARY\.md|-VERIFICATION\.md|-REVIEW\.md|-FIXES\.md|-RESEARCH\.md|-UAT\.md|-PATTERNS\.md|\.continue-here\.md):'
```

| Class | Hits | Files |
|---|---|---|
| **LIVING** — a future reader consults it for how to run or mitigate the binary | **34** | 8 |
| **DATED OBSERVATION** — a record of what one run did on one day | **354** | 95 |
| **TOTAL** | **388** | 103 |

**34 + 354 = 388**, the measured total, so the classification accounts for every
hit and the arithmetic is checkable against the grep rather than against my word.

### The LIVING set — all 34 hits, read individually and classified

Line numbers are as they stood **before** this plan's edits (the pre-edit tree at
`f5b548b`), so the inventory can be reproduced by checking out that commit.

| # | `file:line` | Quoted | Verdict |
|---|---|---|---|
| 1 | `21/deferred-items.md:16` | table row naming the two flaking tests and their symptoms | DATED OBSERVATION — describes what `21-10` saw; makes no run/mitigate claim |
| 2 | `21/deferred-items.md:19` | *"`cargo test --test driver_reattach` returned FAILED, FAILED, then ok on three consecutive runs"* | DATED OBSERVATION — a true record of that session |
| 3 | `21/deferred-items.md:20` | *"the same binary with `-- --test-threads=1` returned ok three times out of three"* | **STANDING — THE ONE. Corrected in place.** Presented as evidence and read as a mitigation; round 10 proved it does not work |
| 4 | `21/deferred-items.md:28` | *"failing `driver_reattach` assertions are about the run record's existence and observability at write one"* | DATED OBSERVATION — `21-10` non-causation analysis |
| 5 | `21/deferred-items.md:33` | *"`rtk proxy cargo test -- --test-threads=2` over the whole suite exits **0** with **1193 passing tests**"* | **STANDING — corrected.** No longer the gate; the 1193 figure is a round-2 tree |
| 6 | `21/deferred-items.md:37` | *"Neither `tests/envelope_tracer.rs` nor `tests/driver_reattach.rs` is in any `21-*` plan's `<files>`"* | **STANDING — corrected. Now FALSE:** `21-34` puts it in its `<files>` (comment-only) |
| 7 | `21/deferred-items.md:42` | *"the honest whole-suite gate is `--test-threads=2` with `--no-fail-fast`"* | **STANDING — corrected.** Rounds 9-11 use the default thread count |
| 8 | `21/deferred-items.md:43` | *"The round-5 orchestrator re-measured the two `driver_reattach` tests"* | DATED OBSERVATION — round-5 run |
| 9 | `21/deferred-items.md:49` | *"Every gate in `21-15` and `21-16` is measured as `... --test-threads=2`"* | DATED OBSERVATION — a statement about two executed plans |
| 10 | `21/deferred-items.md:52` | *"`driver_reattach` is now flakier than recorded above, and its documented mitigation no longer works"* | **STANDING — correct already; kept and cited.** Round 4 got this right |
| 11 | `21/deferred-items.md:54` | *"Under `--test-threads=4` it fails intermittently in whole-suite runs"* | DATED OBSERVATION — round-4 run |
| 12 | `21/deferred-items.md:55` | *"under `-- --test-threads=1` … it now also fails intermittently"* | **STANDING — correct already; kept and cited.** The measurement that refutes hit #3 |
| 13 | `21/deferred-items.md:62` | *"`--test-threads=2` remains reliably green for the whole workspace"* | **STANDING — corrected.** Superseded by the default-thread gate |
| 14 | `21/deferred-items.md:972` | `21-30` section heading | DATED OBSERVATION — heading of round 10's record |
| 15 | `21/deferred-items.md:974` | *"failed its two documented tests in round 10's pre-work baseline run"* | DATED OBSERVATION — round-10 runs |
| 16 | `21/deferred-items.md:980` | *"The variable is not `--test-threads=1`, which does not reliably fix it."* | **STANDING — correct; this is the entry the corrections point at.** Extended (not edited) by the round-11 entry |
| 17 | `todos/pending/2026-08-18-…-race.md:3` | frontmatter `title:` | LIVING reference — no run/mitigate claim, left as is |
| 18 | `todos/pending/…:8` | `files: - tests/driver_reattach.rs:340-375` | LIVING reference — a file pointer |
| 19 | `todos/pending/…:11` | document title | LIVING reference |
| 20 | `todos/pending/…:28` | *"do not serialise the tests with `--test-threads=1` — that hides the race rather than closing it"* | **STANDING — correct already, and it gives the better reason.** Cited in the correction |
| 21 | `.planning/ROADMAP.md:593` (`test-threads` match) | the `21-34` plan row, which QUOTES the stale sentence as the thing to be corrected | DATED OBSERVATION — a quotation-in-role. Editing it would falsify the plan record. **Also orchestrator-owned this wave; not touched** |
| 22 | `.planning/ROADMAP.md:593` (`driver_reattach` match) | same line, second grep term | as #21 |
| 23 | `19/deferred-items.md:6` | *"`tests/driver_reattach.rs` is intermittently flaky (found during 19-04)"* | LIVING, and **correct** — the pre-existing finding this phase inherited |
| 24 | `19/deferred-items.md:21` | *"into a clean directory and `cargo test --test driver_reattach` reproduces the same two failures"* | DATED OBSERVATION — phase-19 `git archive` reproduction. **Load-bearing evidence for M2** |
| 25 | `19/deferred-items.md:90` | *"the third showed only the two known `driver_reattach` flakes"* | DATED OBSERVATION |
| 26 | `19/deferred-items.md:91` | *"It is therefore rarer than the documented `driver_reattach` pair."* | DATED OBSERVATION — a comparative rate from that run |
| 27 | `19/deferred-items.md:103` | *"unlike the `driver_reattach` pair which is …"* | DATED OBSERVATION |
| 28 | `19/deferred-items.md:106` | *"Whoever fixes the `driver_reattach` race — the fix direction is the…"* | LIVING ownership note, **still accurate**; consistent with the promote condition |
| 29 | `.planning/WINDOWS.md:27` | *"it now fails 3/3 in ISOLATION (0.53s) … the isolation-passes assumption in deferred-items.md is now stale"* | DATED OBSERVATION — a true record of the phase-20 gate. **FLAGGED in the record** because a reader could mistake it for the current rate |
| 30 | `.planning/WINDOWS.md:149` | the same entry's JSON `"file"` field | DATED OBSERVATION — machine mirror of #29 |
| 31 | `.planning/WINDOWS.md:151` | the same entry's JSON `"description"` field | DATED OBSERVATION — machine mirror of #29 |
| 32 | `.planning/codebase/TESTING.md:52` | `driver_reattach.rs` named in a list of files whose `#[test]` count a literal grep misses | LIVING reference — a test-counting note, no flake or mitigation claim |
| 33 | `.planning/codebase/STRUCTURE.md:76` | `driver_reattach.rs` in a directory listing | LIVING reference |
| 34 | `src/driver/spawn.rs:55` | *"`driver_reattach.rs` have been using against the real binary since plan 17-06"* | LIVING reference in a doc comment; **no claim about the flake. `src/` is untouchable this round** (prohibition 3) |

**Seven of the 34 are STANDING claims requiring correction** (#3, #5, #6, #7,
#13 corrected; #10, #12, #16, #20 stand as already-correct and are cited).
**Nothing was left unclassified.**

### The DATED OBSERVATION set — 354 hits, enumerated at file granularity

**Named honestly: these 354 are enumerated by `file` with an exact per-file
count, not quoted sentence by sentence.** Quoting 354 sentences would produce a
document nobody reads, which would defeat the purpose; the partition predicate is
mechanical and the counts sum exactly, so the classification is checkable in one
command. **This is a stated shortfall of the inventory, not a hidden one.** The
line-level list regenerates from the two greps plus the predicate above.

95 files. Highest counts: `21-34-PLAN.md` **48** (this plan's own text, which
quotes the stale sentence repeatedly *in the role of the thing to correct*),
`17-05-SUMMARY.md` **17**, `21-31-PLAN.md` **10**, `21-13-SUMMARY.md` **10**,
`21-33-PLAN.md` / `21-32-PLAN.md` / `21-18-PLAN.md` / `21-17-PLAN.md` **8** each,
`21-32-SUMMARY.md` / `21-21-SUMMARY.md` / `21-19-PLAN.md` / `21-16-PLAN.md` /
`21-15-PLAN.md` / `17-08-PLAN.md` / `17-05-PLAN.md` **7** each, then a tail of 6,
5, 4, 3, 2 and 1 across phases 17, 18, 19, 20 and 21. The counts sum to **354**.

### (c) Why they are left unedited — D-21-62, recorded in the file itself

A sentence in an executed SUMMARY recording that a particular run passed 3/3
under a particular setting **is a true record of that run**. Editing it would
falsify a historical observation in order to make a general point — the same
move, pointed the other way, as leaving the stale claim standing. **The
distinction the inventory turns on is not "old versus new"; it is "a claim a
future reader would act on" versus "a report of what happened once."**

### (b) The correction, and where it sits

`rtk proxy grep -n "test-threads" .../deferred-items.md` after the edit, with
each line's class named:

```
20:then ok on three consecutive runs; the same binary with `-- --test-threads=1`   <- STALE (hit #3)
30:> > "the same binary with `-- --test-threads=1` returned ok three times ..."     <- the correction quoting it
42:> 2. **`--test-threads=1` does NOT reliably fix it.** Round 4 already measured   <- the correction
43:>    failing intermittently *under* `-- --test-threads=1` (see the round-4 ...   <- the correction
60:> * **line 33's** `rtk proxy cargo test -- --test-threads=2` **is no longer ...  <- the correction (hit #5)
64:>   1424). The `--test-threads=2` recommendation is retained below as the dated  <- the correction
85:`rtk proxy cargo test -- --test-threads=2` over the whole suite exits **0** with <- hit #5, corrected above
94:`--test-threads=2` with `--no-fail-fast`, and the `--test-threads=1` note above  <- hit #7, corrected below
101:`rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2`.          <- hit #9, DATED
106:`--test-threads=4` it fails intermittently in whole-suite runs; under           <- hit #11, DATED
107:`-- --test-threads=1` — which this file records as passing three times out of   <- hit #12, correct, kept
114:`--test-threads=2` remains reliably green for the whole workspace.              <- hit #13, corrected below
122:> > "the honest whole-suite gate is `--test-threads=2` with `--no-fail-fast`"   <- the correction
124:> > "`--test-threads=2` remains reliably green for the whole workspace."        <- the correction
130:> `--test-threads` setting applied at all. `--no-fail-fast` is kept ...         <- the correction
136:> round-2 `--test-threads=1` line is corrected at its own sentence at the top   <- the correction
1054:The variable is not `--test-threads=1`, which does not reliably fix it.        <- hit #16, correct, cited
1081:2. **`--test-threads=1` does NOT fix it.** Round 4 measured it failing         <- round-11 standing entry
1125:**never** to serialise with `--test-threads=1`, which hides it either way      <- round-11 standing entry
1194:| `deferred-items.md`, immediately after the stale `--test-threads=1` ...       <- round-11 placement table
1195:| `deferred-items.md`, after the round-5/round-4 updates | the ...              <- round-11 placement table
```

**The stale sentence at line 20 is immediately followed by its correction, which
opens at line 26.** A reader who stops at the first section now reads the
correction.

**The correction block, abridged, quoting what it supersedes:**

> ### CORRECTION (2026-08-28, `21-34`, round 11) — READ THIS BEFORE ACTING ON THE PARAGRAPH ABOVE
>
> **The superseded sentence, quoted verbatim from the paragraph immediately above:**
>
> > "the same binary with `-- --test-threads=1` returned ok three times out of three"
>
> **That mitigation does NOT work and must not be reached for.** … 1. **PRE-EXISTING, NOT A REGRESSION** — round 10's orchestrator measured the untouched base and the round's HEAD at the SAME rate, **3/5 red at the base, 3/5 red at HEAD**. … 2. **`--test-threads=1` does NOT reliably fix it.** … 3. **The mechanism is recorded in the `21-30` entry at the END of this file** … pointed at rather than duplicated here, so there is one place to correct next time.

### (d) The note in the failing test file

**`tests/driver_reattach.rs` carried no note about its own documented flake.** It
now opens with one, in the module header block's existing idiom, headed:

```
// THIS FILE IS FLAKY, AND `--test-threads=1` WILL NOT HELP YOU (21-34, round 11)
```

It names **both** flaking tests
(`a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` and
`a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`),
states explicitly that `--test-threads=1` does not fix it and that serialising
hides the race, names the `/proc`-scan mechanism (M1) alongside M2 and M3, points
at `deferred-items.md` by file name for the standing item, and carries the data
points including **the planner's pre-round-11 baseline of 1416 passed / 1 failed
/ 13 ignored at `343c408` with no `--test-threads` setting and before any file
was edited.**

### (e) The comment-only assertion — measured, not promised

| Check | Task 1 | After the deviation correction |
|---|---|---|
| `git diff -- tests/driver_reattach.rs`, `+`/`-` lines | 54 | 31 |
| of those, **non-comment** (`grep -cvE '^[+-][[:space:]]*//'`) | **0** | **0** |
| `git diff --stat -- tests/` | `tests/driver_reattach.rs \| 54 ++++` — **and no other file** | `tests/driver_reattach.rs \| 31 +++-` — and no other file |

**`tests/driver_reattach.rs` is the ONE file under `tests/` round 11 touches, its
diff is comment-only, and that is asserted rather than promised.** The next
verifier should replace the convenient "no file under `tests/` in the diff"
shortcut with this narrower, checkable statement.
**`tests/driver_injection_corpus.rs` is untouched** — empty in
`git diff --stat 2c13fcf HEAD`.

### Task 1 verification

`rtk proxy cargo test --test driver_reattach` →
**`ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.12s`**

The note is comment-only, so the binary's behaviour is unchanged either way —
this run's green is a fact about that run, not evidence the note did anything.
(The same command later went red; see the deviation.)

### (f) The fix was NOT attempted

Prohibition 4 holds. The promote condition — scoping the run discovery to the
test's own process group or worktree, or synchronising on the written artifact —
stays a standing item, because its direction is **false-red under parallelism,
LOUD**, which is the safe direction to be wrong in.

**Commit:** `2be6055`

---

## Task 2 — the standing record

**Commit:** `a1342a1` — 342 insertions, **0 deletions**. Append-only, verified by
`git diff --stat`.

### Round 11's four closures, with residuals and DIRECTIONS

Every residual quoted from the SUMMARY of the plan that **delivered** it — which
is the whole reason this plan ran in wave 2.

| Gap | Closed by | Residual | Direction |
|---|---|---|---|
| gaps[0] argv fusion | `21-31` T1-T3 (`4ca622c`, `3638fd9`, `8964b19`) | the fix rests on the `claude` CLI's option-parser behaviour; nothing here goes red if it changes | **under-detection, SILENT** |
| gaps[0] second half | `21-31` T3 | `read_session_id` keeps its pass-through — *"the CLI resumes by session TITLE, so a rule tight enough to refuse `-h` deletes legitimate sessions silently"* | **under-detection, SILENT** |
| gaps[1] comment budget | `21-32` T1-T2 (`b644c32`, `e18019c`) | five residuals: cross-statement assembly; interpreter named by a variable; method chains unfollowed; four of five string-building forms unseen | **under-detection, SILENT** ×4, plus one **deliberate exclusion** |
| gaps[2] innermost verdict | `21-32` T3 (`c84c16d`) | a composition assembled across statements now reads as un-composed and is reported — **the direction FLIPPED on purpose** | **over-detection, LOUD** (plus one silent: a third file added is not looked at) |
| gaps[3] narrowed census + pin | `21-33` T1 (`9eacc82`, `a23a4c9`) | the census's reach **shrinks monotonically as the work it certifies succeeds** | **under-detection, silent, and growing over time**; the pin's own red is a **BOOKKEEPING red** with a one-line repair |

The measured windows are recorded as literals, not prose: gaps[1]'s executable
join window is **last reported 11, first missed 12**, identical before and after
the fix, with comments free at any count up to 100. gaps[3]'s pinned reach is
**six non-exempt occurrences in two of sixteen files**, and the pin **fired on
its own author inside `21-33`** — which is its justification, not an
embarrassment.

### The `claude` CLI staleness obligation — opened, in the shape of the ratatui one

Re-measured on the merged tree rather than inherited: **`claude --version` →
`2.1.248 (Claude Code)`**, agreeing with `21-31`'s figure. All six probes re-run
in a scratch directory outside the repository, stdin at `/dev/null`, with a
timeout:

```
claude --help | grep -A2 -- '-r, --resume'
claude --resume --version
claude --resume=--version
claude --resume -- --version
claude --resume -- 550e8400-e29b-41d4-a716-446655440000
claude --resume=550e8400-e29b-41d4-a716-446655440000
```

| Probe | Output, verbatim from this run |
|---|---|
| spec | `-r, --resume [value]   Resume a conversation by session ID, or open interactive picker with optional search term` |
| A | `2.1.248 (Claude Code)` — the injection FIRING |
| B | `Error: --resume requires a valid session ID or session title when used with --print. … Provided value "--version" is not a UUID and does not match any session title.` |
| C | `Error: --resume requires a valid session ID or session title when used with --print. Usage: claude -p --resume <session-id\|title>` |
| D | **byte-identical to C** — the `--` separator **deletes the capability** |
| E | `No conversation found with session ID: 550e8400-e29b-41d4-a716-446655440000` — the fused id **BOUND** |

**Independently reconfirmed:** probe D matching probe C is the measurement that
refuted the verification pass's recommended `--` separator fix. Had `21-31`
applied it, the injection would have closed and the resume would have been
silently deleted for every legitimate session.

**The property recorded:** `-r, --resume [value]` is an optional-value option;
the `=` form binds the value regardless of its first byte; the `--` form does NOT
bind. **Direction: under-detection, SILENT.** The obligation carries a dated
version table with two rows (`21-31`, and `21-34` on the merged tree) and the
instruction to re-run the six probes on any CLI upgrade rather than infer from a
version number.

### The three deliberate non-closures

| Item | Measurement making it inert / out of scope today | Promote condition | Direction |
|---|---|---|---|
| `src/executor/claude.rs`'s three `--option value` pairs (`--resume`, `--model`, `--name`) | all three read `Option` fields that are `None` at `ExecutionOptions::default()` and are **set by no caller** | the **FIRST caller that sets one from a value not authored in this crate** | under-detection, **SILENT** until that caller appears |
| `IN-01`…`IN-04` | all `Info` severity; one line of reason each in the record | per-item, recorded | one direction each, recorded per item |
| `WR-06` (per-emulator working-directory column) | a **capability** question, not a security one — no untrusted value travels through `current_dir` | a per-emulator table measured on a machine with a desktop session | under-detection, **SILENT** — a session landing in `$HOME` fails quietly |

**`IN-05` is CLOSED, not outstanding** — `21-32` T2 (`e18019c`) closed it by
narrowing the doc rather than widening the marker set (D-21-53). Cited as closed.

### Every anti-pattern row dispositioned — 14 rows, 14 dispositions

| Id | Severity | Disposition |
|---|---|---|
| gaps[0] CWE-88 argv | Blocker | CLOSED — `21-31` T2 |
| gaps[0] complicit corpus | Blocker | CLOSED — `21-31` T1 |
| gaps[1] comment budget | Blocker | CLOSED — `21-32` T1 |
| gaps[1] dangling cross-reference | Warning | CLOSED — `21-32` T2 |
| gaps[2] over-join / non-vacuity / truncation | Blocker | CLOSED — `21-32` T3 |
| gaps[3] census named for a property it does not check | Blocker | CLOSED — `21-33` T1 |
| `WR-04` | Warning | CLOSED — `21-33` T2 (`grep -c "implements_"` 0 → 31, three planted-impl REDs) |
| `WR-05` | Warning | CLOSED — `21-33` T2 (`.then_with(\|\| a.cmp(b))`, RED captured) |
| `WR-06` | Warning | **DELIBERATE NON-CLOSURE** |
| `IN-01` | Info | **DELIBERATE NON-CLOSURE** |
| `IN-02` | Info | **DELIBERATE NON-CLOSURE** |
| `IN-03` | Info | **DELIBERATE NON-CLOSURE** |
| `IN-04` | Info | **DELIBERATE NON-CLOSURE** |
| `IN-05` | Info | CLOSED — `21-32` T2 |

**Total 14; none dropped.** Also delivered without being on the table: `21-29`'s
carried **S1** (`21-33` T3, `5400e44`) and **F3**, closed as **UNNECESSARY by
measurement** — no visibility widened anywhere.

### The Record-corrections table — the four round-10 truths pass 11 measured FALSE

| Truth | Claimed | Measured | Closed by |
|---|---|---|---|
| `21-27` t1 | *"no parser left in the path"* | **FALSE** — `claude`'s own option parser remained; CWE-78 traded for **CWE-88**, undisclosed in every round-10 artifact | `21-31` T2 (`3638fd9`) — doc correction quoting the falsified clause, argv fused to three elements |
| `21-27` t5 | the census reports **"any"** such executable line | **FALSE at twelve comment lines** | `21-32` T1+T2 — RED at exactly 12, increment moved, two-sided boundary control, "any" deleted **in the same commit** |
| `21-28` t7 | an executable call appears **only** inside a composition | **FALSE** — sibling arms launder | `21-32` T3 (`c84c16d`) — verdict moved to the innermost call, so no window is consulted at all |
| `21-29` t2 | the claim becomes a committed control over `src/ui/` | **FALSE** — completeness not checked; 6 lines in 2 of 16 files; **no live leak** found in a 63-site hand trace | `21-33` T1 — renamed, reach disclosed **and PINNED** as a two-direction equality |

The full four-column table with the verbatim claims and each closing plan's
committed evidence is in `deferred-items.md`, append-only and dated.

### ROADMAP criterion 4 — verbatim, fourth round, no work claimed

Re-surfaced **byte-identical** to the round-10 entry (command, expected, and
why_human quoted from `21-VERIFICATION.md`'s `behavior_unverified_items`
frontmatter). Measured on the merged tree:

- `rtk proxy cargo test --test driver_injection_corpus` → **`ok. 13 passed; 0 failed; 10 ignored`**
- `rtk proxy git diff --stat 2c13fcf HEAD -- tests/driver_injection_corpus.rs` → **empty**

**The sentence added this round, so the next verifier does not re-litigate it,
quoted from the record:**

> **ROADMAP success criterion 4 is PERMANENTLY AGENT-UNCLOSABLE BY CONSTRUCTION.**
> It requires a human with a live, authenticated Claude subscription to run the
> ten `#[ignore]`d arms of `tests/driver_injection_corpus.rs` against the real
> model binary. No agent has such a subscription and no agent can acquire one, so
> no amount of further planning, further rounds or cleverer test design will close
> it. It is a standing item **by explicit user decision**.
>
> **4/5 is therefore the EXPECTED AND CORRECT CEILING for this phase, and is not a
> failure.** A future verification pass that scores this phase 4/5 has scored it
> correctly. A future round that opens a plan against criterion 4 is doing work
> that cannot succeed.

### DRIVE-04 — both backstops re-run as explicit evidence

`rtk proxy cargo test --test driver_escalation_cap` → **`ok. 8 passed; 0 failed; 0 ignored`**

| Row | Direction | Arms |
|---|---|---|
| edge-probe 6 (**boundary**) | a cap AT or ABOVE the resolved step cap is refused at the seam | `a_budget_equal_to_the_resolved_step_cap_refuses_above_the_run`, `a_budget_of_zero_refuses_above_the_run` |
| edge-probe 6 (**boundary**) | a cap BELOW it is accepted and parks on the cap | `a_budget_one_below_the_resolved_step_cap_runs_and_parks_on_the_cap` |
| edge-probe 7 (**precision**) | the decomposition consultation counts against the SAME cap; exceeding it parks with a typed reason | `a_run_that_spends_its_budget_parks_and_says_so_rather_than_continuing`, `the_three_boundaries_are_measured_against_the_resolved_step_cap` |

`rtk proxy git diff --stat 2c13fcf HEAD -- tests/driver_escalation_cap.rs` →
**empty.** So the reconfirmation is explicit evidence, not an inference from the
round's scope.

**DRIVE-04 precision, in words:** the escalation counter is an **integer count
compared by equality against an integer cap** — no rounding, no tie-breaking, no
overflow, no precision-loss contract to state.

---

## Task 3 — `COVERAGE.md` amended, and the merged-tree gate

**Commit:** `47ef05e` — 74 insertions, **0 deletions**, **0 removed content
lines**. The existing round-10 no-external-API declaration comes out
**byte-identical**, as prohibition 2 requires.

### The appended round-11 paragraph, abridged

> **Round 11 of phase 21 (`21-31`, `21-32`, `21-33`, `21-34`) likewise integrates
> no external API, SDK or service.** Its entire diff changes: **one argv
> element's shape**; **two census algorithms and the claims that rest on them**;
> **one sort comparator clause**; **a set of controls, fixtures and doc
> corrections**; and **three documentation records**. … Nothing above adds a
> client, a base URL, an auth exchange, a response schema or a retry policy,
> because round 11 adds no interface at all — it changes how this tree hands an
> argument to a program it already spawned, and what its own censuses are
> permitted to claim.

The amendment also records that the `claude` subprocess seam stays deliberately
off the matrix and **sharpens the reason**: `21-31` measured the receiving
binary's option parser precisely because it is a program this phase **hardens**,
not an interface this round **integrates**. That measurement exposed a real
obligation a coverage matrix would never have caught — the parser's behaviour
belongs to the CLI **version** — which is why the staleness obligation exists.

### The no-dependency claim, MEASURED

Both empty, both under `rtk proxy`:

```
rtk proxy git diff 2c13fcf HEAD -- Cargo.toml Cargo.lock     (no output)
rtk proxy git diff 343c408 HEAD -- Cargo.toml Cargo.lock     (no output)
```

`2c13fcf` is round 11's base; `343c408` is the pre-round-11 gate baseline.
**The `## Package Legitimacy Audit` gate does not fire:** no `cargo add`, no new
dependency, no manifest change in any of round 11's four plans.

### The MERGED-TREE gate — every figure re-measured here, none inherited

| Gate | Planner at `343c408` | Orchestrator, post-merge quiet tree | **`21-34` on the merged tree** | Agrees |
|---|---|---|---|---|
| `cargo build` | — | exit 0 | **exit 0** | yes |
| `cargo test --workspace --no-fail-fast` | 1416 / 1 / 13 (flake fired) | 1424 / 0 / 13 | **1422 passed / 2 failed / 13 ignored**, 35 binaries | **total agrees at 1424; the split differs — reported, not smoothed** |
| `cargo clippy -- -D warnings` (lib, the project gate) | exit 0 | exit 0 | **0 errors → exit 0** | yes |
| `cargo clippy --all-targets -- -D warnings` | exactly 4 | exactly 4 | **exactly 4** | yes |
| `cargo test --test driver_injection_corpus` | 13 / 0 / 10 | — | **13 / 0 / 10** | yes |
| `cargo test --test driver_escalation_cap` | 8 / 0 / 0 (round 10) | — | **8 / 0 / 0** | yes |

**The four `--all-targets` lints, verbatim from this run:**

```
error: used `assert_eq!` with a literal bool
   --> src/browser.rs:155:9
error: used `assert_eq!` with a literal bool
   --> src/browser.rs:156:9
error: used `assert_eq!` with a literal bool
   --> src/browser.rs:157:9
error: this creates an owned instance just for comparison
   --> src/project_creator.rs:146:27
error: could not compile `gsd-meta-manager` (lib test) due to 4 previous errors
```

**Exactly four, same two kinds, same two files, same four line numbers.** Round
11's diff names neither file. **A count of three would have been a failure of
this round exactly as five would be.**

### Every delta attributed — the arithmetic reconciles exactly

```
1422 passed + 2 failed = 1424 tests
1417 (green baseline) + 7 (added by wave 1) = 1424
```

| Delta | Attribution |
|---|---|
| +2 | `21-31` — `the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program`, `launch_terminal_argv_carries_no_untrusted_element_and_is_pinned_at_two` |
| +2 | `21-32` — `the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments`, `a_composed_arm_does_not_launder_its_un_composed_sibling` |
| +3 | `21-33` — the backlog tiebreak control, the `EditBuffer` trait probe, the S1 input-echo spot-check |
| −2 passed, +2 failed | `driver_reattach` — the documented flake, **both** arms firing together |

**No unattributed delta.** The total agrees exactly with the orchestrator's
quiet-tree 1424; only the pass/fail split differs, and that difference is the
flake, reported with both numbers as prohibition 6 requires.

### The file fence — held, with the plan's own count corrected

`rtk proxy git diff --stat 2c13fcf HEAD` names **16** files:

| File | Owner |
|---|---|
| `src/ui/screens/detail.rs`, `src/session_detector.rs` | `21-31` |
| `src/text.rs`, `src/ui/screens/driver.rs` | `21-32` |
| `src/ui/mod.rs`, `src/ui/screens/mod.rs`, `src/ui/screens/render_escape_guard.rs`, `src/state_reader/backlog.rs` | `21-33` |
| `tests/driver_reattach.rs`, `deferred-items.md`, `COVERAGE.md` | `21-34` |
| `21-31-SUMMARY.md`, `21-32-SUMMARY.md`, `21-33-SUMMARY.md` | the three wave-1 plans' own outputs |
| `.planning/STATE.md`, `.planning/ROADMAP.md` | **the orchestrator's** post-wave-1 tracking commit `f5b548b` — not written by any executor |

**No fence breach.** Every source file is one of the eight wave 1 owns; the only
file under `tests/` is `driver_reattach.rs`.

**The plan's own arithmetic corrected rather than smoothed.** Task 3's action (d)
says the diff must name *"exactly the ten files the four plans own"* and then
lists **eight `src/` files + `tests/driver_reattach.rs` + the two `.planning/`
records = eleven**, not ten. The list is right and the number is wrong. Reported
here rather than padded or trimmed to make the count come out.

### `.planning/REQUIREMENTS.md` — untouched

**Absent from `git diff --stat 2c13fcf HEAD`.** Ninth consecutive round.
Requirement status stays where it belongs: decided by a passed verification, not
by an executor.

---

## Deviations from Plan

### 1. [Rule 1 — Bug in this plan's own freshly-written claim] `21-34` measured its own Task 1 record FALSE hours after committing it

- **Found during:** Task 3's merged-tree gate.
- **Issue:** Task 1 recorded wave 1's *"every isolated re-run was green 3/3"* and the orchestrator's fully-green quiet-tree run — both true records of those runs — and let them stand as the current picture. `21-34`'s own gate then contradicted the generalisation they invite:

| Run | Result |
|---|---|
| workspace gate | **1422 / 2 / 13** — BOTH flaking arms fired together, a **third** distinct pattern |
| six back-to-back isolated runs, default threads, **idle machine, nothing else running** | **2 green (6.12s) / 4 red (0.53s)** |

- **What it refutes:** **M1 cannot be the whole story.** Round 10's mechanism sentence names *"other driver-spawning test binaries running concurrently"* as the variable. Four of those six reds happened with **no other test binary running at all**, so a concurrent sibling is **not necessary** for the failure — and a fix that only scopes the `/proc` scan may leave the rate where it is.
- **What it adds — M3, NAMED NOT MEASURED:** the parallelism that remains when nothing else runs is **inside this process**. The three tests run on separate threads by default, and `isolate_envelope_root()` sets a **process-wide** environment variable (`std::env::set_var`) from whichever thread reaches it first while the siblings are already executing. **No experiment here attributes the failure to M3**; it is written down so the discriminating experiment covers it, not to claim a cause.
- **The awkward measurement, reported because suppressing it would be the same failure as inflating one:** `-- --test-threads=1` ran **5/5 green** at ~6.67s each. **This does NOT reinstate the mitigation, and the correction at the top of `deferred-items.md` stands unchanged**, for three reasons: (i) round 4 measured this binary **failing** under that flag; (ii) **n = 5 is the same kind of evidence that produced the original false claim, which was n = 3** — recording it as a fix would be committing the round-2 error again with a slightly larger sample, inside the entry that exists to correct it; (iii) serialising **hides** the race rather than closing it. The **only** claim drawn from the pairing is that **parallelism matters even with no other binary running**, which is a fact about the three tests in this one process.
- **Fix:** the correction was made in `deferred-items.md` and mirrored into the test file's note, in a **separate, legible commit** rather than folded into Task 3, and the discriminating experiment was upgraded to a 2×2 — (default vs serialised) × (idle vs concurrent load) — at HEAD and at `0a84023`, with N large enough that a five-run streak cannot decide it.
- **Files modified:** `deferred-items.md` (+73, 0 deletions), `tests/driver_reattach.rs` (+31/−2, **still 0 non-comment diff lines**).
- **Committed in:** `80bc488`.

### 2. [Rule 1 — Arithmetic corrected, not smoothed] The plan's "ten owned files" is eleven

- **Found during:** Task 3, step (d). See the file-fence section: the enumerated list is correct, the stated count is not. No file was dropped or invented to make the number come out.
- **Committed in:** `47ef05e` (reported in the commit message).

### 3. [Reported, not fixed] The inventory quotes 34 of 388 hits, and says so

- The LIVING set is quoted and classified hit by hit. The 354 DATED OBSERVATIONS are enumerated at **file granularity with exact per-file counts summing to 354**, not sentence by sentence. The partition predicate is mechanical and re-runnable, so the classification is checkable in one command — but **the quoted-sentence half of the plan's acceptance criterion is met for 34 hits, not 388, and that shortfall is stated in the SUMMARY rather than hidden.** Quoting 354 sentences would produce a document nobody reads, which defeats the purpose the inventory serves.

---

**Total deviations:** 2 auto-fixed (both measurement/arithmetic corrections), 1
reported-only shortfall.
**Impact on plan:** None on scope; all three sharpen the record. Deviation 1 is
the significant one — a record-correction plan measured its own freshly-written
claim false and corrected it in a separate commit, which is the behaviour this
phase exists to produce rather than a failure of it.

---

## Issues Encountered

**The `driver_reattach` flake fired in this plan's own gate and did NOT go green
on every isolated re-run**, unlike wave 1's experience. Fully documented above
and in `deferred-items.md`. Reported, not absorbed, and not fixed.

**No architectural decision was reached, no checkpoint was hit, no auth gate
fired, and the fix-attempt limit was never approached.**

## Known Stubs

None. No placeholder values, no skipped tests, no unrun `<verify>` commands — all
three task-level `<verify>` commands were executed under `rtk proxy` and their
counts are quoted verbatim above.

## Threat Flags

None. This plan changed **no executable line anywhere in the tree**: two
`.planning/` records gained append-only sections and one test file gained
comment lines, asserted comment-only. No new network endpoint, auth path,
file-access pattern or schema change at a trust boundary.

## Residuals this plan leaves, each with its direction

| Residual | Direction |
|---|---|
| The `driver_reattach` flake itself — fix not attempted | **false-red under parallelism, LOUD** (the safe direction) |
| Which mechanism causes it — M1 measured **insufficient alone**, M2 and M3 undiscriminated | **unknown cause; the failure direction is LOUD either way**, so the deferral stays defensible |
| The `claude` CLI's `--resume` arity and `=`-binding | **under-detection, SILENT** — bounded only by the new staleness obligation |
| `src/executor/claude.rs`'s three `--option value` pairs | **under-detection, SILENT** until a caller sets one from a value not authored in this crate |
| `WR-06`'s per-emulator working directory | **under-detection, SILENT** — a session landing in `$HOME` fails quietly |
| `IN-01`…`IN-04` | recorded per item |
| The inventory quotes 34 of 388 hits | **stated shortfall**; the partition is mechanical and re-runnable, so the completeness is checkable even where the quoting is not exhaustive |
| Whether the corrected prose stops the next reader | **unbounded by any test** — a prose judgment, flagged as `human_judgment: true` |
| ROADMAP criterion 4's behavioural half | **permanently agent-unclosable**; 4/5 is the correct ceiling |

## User Setup Required

None — no external service configuration, no new dependency, no `Cargo.toml` or
`Cargo.lock` change.

## Next Phase Readiness

- **The record is corrected and its completeness is checkable from a committed inventory rather than trusted.** The stale mitigation no longer stands unqualified at the top of the file.
- **For verification pass 12:** the one claim here no test can assert is whether the corrected prose actually stops the next reader reaching for `--test-threads=1`. The `5/5 green` serialised sample recorded in the deviation is the specific thing a careless reader could seize on; the record states three reasons it is not a fix, immediately beside it.
- **New standing item for whoever fixes the flake:** M1 is measured **insufficient on its own**, so scoping the `/proc` scan alone may not move the rate. Run the 2×2 discriminating experiment first.
- **`WINDOWS.md` was NOT edited** — it is outside this plan's `files_modified`. Its entry `27` (*"fails 3/3 in ISOLATION"*) is a true record that round 11's data both contradicts (wave 1's green isolated re-runs) and corroborates (this plan's 4-of-6 red isolated runs). It is named and flagged in `deferred-items.md` instead.
- **`.planning/STATE.md`, `.planning/ROADMAP.md` and `.planning/REQUIREMENTS.md` deliberately untouched** — the orchestrator owns those writes.

## Self-Check

**Files claimed as modified, verified on disk:**

- `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md` — FOUND
- `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/COVERAGE.md` — FOUND
- `tests/driver_reattach.rs` — FOUND

**Commits claimed, verified in `git log`:**

- `2be6055` — FOUND
- `a1342a1` — FOUND
- `80bc488` — FOUND
- `47ef05e` — FOUND

**Plan-level `<verification>` re-run:** `cargo build` exit 0; workspace
1422/2/13 across 35 binaries with every delta attributed and the total
reconciling to 1424; the 2 failures named as the documented flake and **not**
shown green on an isolated re-run this time, which is reported rather than
smoothed; lib clippy 0 errors; `--all-targets` exactly four pre-existing lints
quoted verbatim; the 388-hit inventory committed with every hit classified; both
`.planning/` records additions-only; `tests/driver_reattach.rs` comment-only,
asserted twice; both `Cargo` diffs empty; the file fence naming only owned files;
corpus 13/0/10 unchanged with no work claimed against criterion 4; escalation cap
8/0 with the file unchanged; `.planning/REQUIREMENTS.md` untouched.

## Self-Check: PASSED

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-28*
