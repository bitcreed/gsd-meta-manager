---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 32
subsystem: testing
tags: [rust, census, prompt-injection, static-analysis, anti-self-match]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "21-27's interpreter census (`text::tests::interpreter_program_sites`) and 21-28's composition census (`driver::tests::composition_census`) — the two mechanisms this plan repairs"
provides:
  - "The interpreter census's join budget counts EXECUTABLE lines only; comment lines are free at any count"
  - "`interpreter_sites_in(path, lines)` — the per-file walk as a pure function, so the live assertion and its boundary control consume the same code"
  - "A two-sided comment/executable window boundary control, captured RED against the pre-fix increment placement"
  - "The interpreter census's narrowed claim: three named bounds, five residuals each with a failure direction"
  - "The composition verdict taken at the innermost call, so no joined unit can launder an occurrence inside it"
  - "The composition census's non-vacuity total computed over the slice it actually scans"
  - "Truncation cut at a column-zero test MODULE, with exactly one asserted per census file"
affects: [21-33, 21-34, phase-21-verification, future-interpreter-reintroduction, future-render-site-conversion]

actuals:
  tokens: 82000
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Per-occurrence verdicts over per-unit verdicts: a decision taken over a joined window answers a question about the window, not about the calls inside it"
    - "Anti-self-match extended to SIBLING censuses: a fixture token spelled whole becomes a hit in whichever census reads the file"
    - "Two-sided boundary controls: a window asserted at N and N+1 in both directions is its own certificate"

key-files:
  created: []
  modified:
    - src/text.rs
    - src/ui/screens/driver.rs

key-decisions:
  - "D-21-51: gaps[1] closed by a bounded mechanism FIX plus a narrowed claim, not by disclosure alone and not by a new census"
  - "D-21-52: the join budget counts EXECUTABLE lines; comments are free at any count"
  - "D-21-53: IN-05 closed by NARROWING the doc, not by widening the marker set"
  - "D-21-54: gaps[2] closed by moving the verdict to the INNERMOST call rather than by making the joining smarter"
  - "D-21-55: truncation cuts at a column-zero `#[cfg(test)]` whose following declaration is a MODULE, exactly one asserted per file"

patterns-established:
  - "A mechanism repaired and its claim narrowed in the SAME commit — never repaired and simultaneously re-sold as broader"
  - "Every residual named WITH its failure direction and, where the direction flips, the flip stated explicitly"

requirements-completed: [SAFE-07, SAFE-08]

coverage:
  - id: D1
    description: "A re-introduced command-interpreter call is reported by the census no matter how many comment lines separate the interpreter name from the interpolation"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/text.rs#text::tests::the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments"
        status: pass
      - kind: unit
        ref: "src/text.rs#text::tests::no_executable_line_under_src_hands_an_interpreter_an_interpolated_program"
        status: pass
    human_judgment: false
  - id: D2
    description: "The executable-line join window is measured unchanged by the comment fix, asserted at N and N+1 in both directions"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/text.rs#text::tests::the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments (executable-filler arm, 11 reported / 12 missed)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The interpreter census's doc says exactly as much as the mechanism does: the word `any` is gone, three bounds are named, five residuals carry directions, and `has_an_unclosed_delimiter`'s cross-reference resolves"
    requirement: SAFE-07
    verification: []
    human_judgment: true
    rationale: "A doc's honesty is a prose judgment. No test can assert that a claim is not wider than its mechanism; the reviewer must read the three bounds against the measured window. The mechanism half is covered by D1/D2."
  - id: D4
    description: "A composed sibling cannot launder an un-composed call sharing its joined logical unit; the laundering was reproduced before it was closed"
    requirement: SAFE-08
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#ui::screens::driver::tests::a_composed_arm_does_not_launder_its_un_composed_sibling"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#ui::screens::driver::tests::no_executable_control_class_call_in_these_two_files_stands_outside_a_composition"
        status: pass
    human_judgment: false
  - id: D5
    description: "The composition census's non-vacuity total is computed over the slice it scans, and its truncation cuts at a test MODULE with exactly one asserted per file"
    requirement: SAFE-08
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#ui::screens::driver::tests::no_executable_control_class_call_in_these_two_files_stands_outside_a_composition (marker arm + non-vacuity arm)"
        status: pass
    human_judgment: false

duration: 30 min
completed: 2026-08-28
status: complete
---

# Phase 21 Plan 32: Comments Stop Spending the Census's Join Budget, and the Composition Verdict Moves to the Innermost Call Summary

**Two round-10 censuses that pass 11 measured over-claiming are repaired at the mechanism and have their claims narrowed in the same commit: comment lines no longer exhaust the interpreter census's 16-line join budget, and the composition verdict moves off the joined unit onto the characters immediately preceding each occurrence, so a composed `match` arm can no longer launder its bare sibling.**

## Performance

- **Duration:** 30 min
- **Started:** 2026-08-28T00:09:10Z
- **Completed:** 2026-08-28T00:39:43Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- **gaps[1] closed at the line the verifier named.** `taken += 1` moved from immediately after `ahead += 1` to immediately after the comment `continue`. Captured RED first, at exactly twelve comment lines.
- **The interpreter census's claim narrowed in the same commit.** The word `any` is gone; three bounds are named; the residual block goes from three residuals to five, each with a direction word, including the method-chain residual another doc had been promising for a whole round.
- **gaps[2] closed by deleting the window from the decision.** The verdict is now taken per occurrence from the preceding characters. All five real production sites still judge composed; the two-arm laundering fixture goes from `ok` to reported.
- **Both of the composition census's secondary defects closed alongside it** — the non-vacuity slice mismatch and the silent truncation kill switch — so the innermost fix is not a claim wider than its certificate.
- **Four REDs captured verbatim**, each against the unfixed code, each exercising the same extracted function the live assertion consumes.

## Task Commits

1. **Task 1: comments stop spending join budget + two-sided boundary test** — `b644c32` (fix)
2. **Task 2: the census stops claiming it reports "any" such line** — `e18019c` (docs)
3. **Deviation: the CR-01 fixture stops spelling the spawn seam's needle** — `a64005b` (fix)
4. **Task 3: the composition verdict moves to the innermost call** — `c84c16d` (fix)

## Re-measurement first — every planner figure checked against this tree

| Gate | Planner at `343c408` | This executor at `2c13fcf` | Agrees |
|---|---|---|---|
| `rtk proxy cargo clippy -- -D warnings` (lib) | exit 0 | exit 0 | yes |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | 4 lints: `bool_assert_comparison` ×3 at `src/browser.rs:155,156,157`, `cmp_owned` ×1 at `src/project_creator.rs:146` | identical — 4 lints, same two kinds, same two files, same four line numbers | yes |
| `INTERPRETER_JOIN_LINES` | 16 | 16 | yes |
| live interpreter census over `src/` | 0 sites | 0 sites | yes |
| live composition census | 0 sites | 0 sites | yes |
| `driver.rs` column-zero `#[cfg(test)]` | one, line 2030, followed by `mod tests {` | one, line 2030, followed by `mod tests {` | yes |
| `driver_confirm.rs` column-zero `#[cfg(test)]` | one, line 661, followed by `pub(crate) mod tests {` | one, line 661, followed by `pub(crate) mod tests {` | yes |
| `detail.rs` column-zero `#[cfg(test)]` (the live hazard) | two: `:5342` a `pub(super) fn`, `:5764` `mod tests` | two: `:5342` `pub(super) fn first_string_entry`, `:5764` `mod tests` | yes |
| whole-file needle lines | `driver.rs` 3, `driver_confirm.rs` 4 | 3 (645, 944, 2349) and 4 (178, 180, 304, 344) | yes |
| production-slice needle lines | 2 and 4 | 2 and 4 | yes |

**One difference from the planner's figure, reported rather than smoothed:** the `driver_reattach` flake landed on a *different test* in this run — see "Issues Encountered".

## Task 1 — the comment window

### The RED, verbatim

```text
thread 'text::tests::the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments' (2948088) panicked at src/text.rs:2014:13:
the census MISSED the CR-01 shape with 12 comment line(s) between the interpreter name and the interpolation. A comment cannot carry a call, so it must not spend the join budget: `taken += 1` has to run AFTER the comment `continue`, not before it.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test text::tests::the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments ... FAILED
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 0.00s
```

**The first failure fired at TWELVE comment lines** — the arm having passed at zero, four, ten and eleven immediately before it. Stated in words because that number is the whole finding: twelve comment lines were enough to silence the phase's sharpest execution-sink control on the exact CR-01 construction it exists to catch.

### The comment arm, re-measured by this executor in both states

Measured by driving the **committed** `interpreter_sites_in` — not a re-implementation — with the increment temporarily restored to its pre-fix position and then moved back.

| N comment lines | 0 | 4 | 10 | 11 | 12 | 13 | 20 | 40 | 100 |
|---|---|---|---|---|---|---|---|---|---|
| **pre-fix** | REPORTED | REPORTED | REPORTED | REPORTED | **MISSED** | **MISSED** | **MISSED** | **MISSED** | **MISSED** |
| **fixed** | REPORTED | REPORTED | REPORTED | REPORTED | REPORTED | REPORTED | REPORTED | REPORTED | REPORTED |

This reproduces pass 11's measurement exactly and independently — in Rust, against the committed function, rather than against a Python re-implementation of its published algorithm.

### The executable-filler arm — the window that must NOT move

| M executable filler lines | 0 | 5 | 9 | 10 | 11 | 12 | 13 | 16 | 20 |
|---|---|---|---|---|---|---|---|---|---|
| **pre-fix** | REPORTED | REPORTED | REPORTED | REPORTED | REPORTED | MISSED | MISSED | MISSED | MISSED |
| **fixed** | REPORTED | REPORTED | REPORTED | REPORTED | REPORTED | MISSED | MISSED | MISSED | MISSED |

**The pair is identical either side of the move — last reported 11, first missed 12 — so the executable window is measured unchanged while the comment behaviour is measured to have changed.** Both numbers are committed as literals with the arithmetic beside them: the fixture spends four lines of budget before the filler (`.args([` at `taken` = 1, then the exec flag, the interpreter name and the command flag at `taken` = 4) and one further line is needed for the interpolation itself, so against `INTERPRETER_JOIN_LINES = 16` the last reportable count is `16 - 5 = 11` and the first missed is `16 - 4 = 12`. Both relations are themselves asserted, so a change to the constant fails the test rather than silently moving the census's reach.

### Acceptance evidence

- `rtk proxy cargo test --lib -- text::tests --nocapture` → `test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 1083 filtered out`
- Live census green: `no_executable_line_under_src_hands_an_interpreter_an_interpolated_program ... ok` — **zero sites over the real `src/`**, unchanged by the fix.
- Anti-self-match green: `the_interpreter_census_does_not_report_the_module_that_builds_its_needle ... ok` — the module that builds the needle is still not a hit in its own census, and the assembled names still differ from the stems that build them.
- `rtk proxy grep -c "interpreter_sites_in" src/text.rs` → **6** (required ≥ 3)
- `rtk proxy grep -c "temp_dir" src/text.rs` → **0** — the boundary control creates no temporary directory and performs no filesystem read.
- `rtk proxy cargo clippy -- -D warnings` → exit 0

## Task 2 — the narrowed claim

### The overclaim, quoted old and new side by side

**Old (deleted):**

> It reports every executable line under `src/` where the doc's THIRD question would have to be asked, and the answer must be none.

**New:**

> **Exactly what it reports — three bounds, not a completeness claim.** It reports every executable line under `src/` at which **all three** of these hold, and the answer must be none: (1) a command-interpreter binary is named by a **QUOTED LITERAL** — an interpreter named through a variable, a const or a config value is not seen; (2) one of the **five** markers in `INTERPOLATION_MARKERS` appears; (3) both fall inside **ONE joined logical unit** of at most `INTERPRETER_JOIN_LINES` **executable** lines, joined only while delimiters stay open. That is the claim. It is deliberately narrower than "any such line": round 10's version of this doc said **any**, pass 11 measured that false (twelve comment lines defeated it), and 21-32 repaired the mechanism and cut the word in the same commit rather than leaving a claim wider than its certificate.

The words `any` and `every` no longer appear in that doc without the three bounds attached.

### The residual block — three residuals became five, each with a direction

Quoted in full:

> **What it does NOT see, with the direction.** Five residuals, each with its failure direction. None of them is bounded by this census; what bounds them is `Untrusted::as_raw_for_logic_only`'s rule and code review.
>
> 1. **Assembled across statements.** It is a SOURCE SCAN over one logical call at a time, so a program string built into a local on one line and handed to an interpreter three lines later is invisible. **Under-detection, silent.**
> 2. **Interpreter named by a variable.** `names_an_interpreter` wants a quoted literal; a binary chosen through a variable, a const or a config value is invisible. **Under-detection, silent.**
> 3. **Interpolation only into a neighbouring non-program string.** `project_creator::execute_hook` spawns an interpreter on one line and interpolates only into its ERROR message on another, and is not reported — correctly: the hook command is a shell command by design, supplied by the operator's own config. **Deliberate exclusion**, not a gap.
> 4. **Method chains are not followed** — the residual `has_an_unclosed_delimiter`'s doc promises is stated here. The join advances only while `(` or `[` stay unclosed; it does NOT chase `.method()` onto the next line. So a construction whose interpreter name sits on a line with balanced delimiters and whose interpolation sits on a later chained call is never joined to it and is invisible. **Under-detection, silent.** The price is paid on purpose: a chain-follower would join `Command::new("/bin/sh")` in `driver::liveness`'s fixture to an unrelated `.args([.., &format!(..)])` three lines below and report correct code — and a census that cries wolf is a census the next author deletes.
> 5. **Four of five string-building forms are unseen.** `interpolates_into_a_string` looks for five substrings; a string built by `concat!`, `join`, an owned `+`, a `replace` or an owned `push_str` carries none of them. **Under-detection, silent.** Widening the set is rejected in that doc, with the reason.

### The cross-reference now resolves

**`has_an_unclosed_delimiter`'s doc, old final sentence:**

> The price is stated as a residual on the census itself.

**New:**

> The price is paid, and it is stated as the **method-chain residual** in `no_executable_line_under_src_hands_an_interpreter_an_interpolated_program`'s "What it does NOT see" block. Pass 11 recorded that this sentence used to point at prose that did not exist — the block named three residuals and this was not one of them. It is now written there, with its direction.

**In one sentence: the cross-reference now resolves** — `has_an_unclosed_delimiter`'s doc points at residual 4, residual 4 exists, and it states the same method-chain price with the same `driver::liveness` justification and an explicit direction word.

### IN-05, closed by narrowing the doc rather than the detector

**`interpolates_into_a_string`'s doc, old:**

> Does `logical` INTERPOLATE a value into a string? The formatting macros this codebase builds strings with, plus `&`-string concatenation.

**New, quoted in full:**

> **Does `logical` contain one of five interpolation markers?** Not "does it interpolate". The markers are exactly `INTERPOLATION_MARKERS` — `format!`, `write!`, `writeln!`, `+ &` and `push_str(&` — and nothing else is looked for.
>
> Both halves must hold for a line to be reported: naming an interpreter with a FIXED command is not the defect, and a census that reported it would be a ban on a word rather than a check on a construction — at which point the next author works around it by renaming.
>
> **What the five markers miss, with the direction.** A string assembled by any OTHER means is invisible here: a `push_str` of an already-owned value (no `&`), a `concat!`, a `join`, an owned `+` without the reference marker, a `replace`, a `String::from` fed a previously-built variable. **Under-detection, silent.**
>
> **Why the set is not widened** (IN-05, 21-32 T2). Every additional substring marker adds false positives to a control whose entire value is that its zero can be trusted, and a census that reports correct code is a census the next author disables. The honest move is to say what the five are and what they miss — not to enumerate the next level down and call the claim repaired.

### The measured numbers the narrowed claim rests on

The doc's "The reach, MEASURED" block states **this executor's own numbers, not the planner's**: comment lines cost nothing at any count, with **100** named as the largest N actually tested by this executor, and the executable window given as **11 reported / 12 missed**, measured identical before and after the fix. Both figures come from the tables above, produced by driving the committed function in this tree.

### Doc-only, verified

`rtk proxy git diff -- src/text.rs` for this task, filtered to lines that are neither `///` doc lines nor blank, produced **no output**. **No executable line changed in Task 2.**

- `rtk proxy cargo test --lib -- text::tests --nocapture` → `ok. 21 passed; 0 failed`
- `rtk proxy cargo clippy -- -D warnings` → exit 0

## Task 3 — the innermost verdict

### The laundering, reproduced verbatim against the committed rule

```text
thread 'ui::screens::driver::tests::a_composed_arm_does_not_launder_its_un_composed_sibling' (2971679) panicked at src/ui/screens/driver.rs:2255:9:
assertion `left == right` failed: the census did not report the BARE arm of a two-arm match whose sibling is composed. A verdict taken over the joined unit answers a question about the unit, not about the calls inside it, so the composed arm launders its sibling and `U+202E`, `U+00AD` and the `U+E0000..U+E007F` tag block reach a terminal cell from a site the census calls clean. The verdict has to be taken at each occurrence, from the characters immediately preceding it.
  left: []
 right: ["fixture.rs:3"]
```

`left: []` **is** the laundering: the same fixture, driven through the same extracted `census_sites_in`, is judged **clean by the committed joined rule** and **a site (`fixture.rs:3`) by the innermost rule**. The unit-shape assertion passed in that same run — the fixture genuinely put both arms in one logical unit — so the red is the verdict's, not the join's.

### The five real production sites, each judged composed by the new rule

Measured by driving `occurrence_is_composed` over the production slice of both census files:

| Site | needle occurrences in the unit | composed under the innermost rule | note |
|---|---|---|---|
| `src/ui/screens/driver.rs:645` | 1 | **yes** | unqualified call inside `shown_capped` |
| `src/ui/screens/driver.rs:944` | 1 | **yes** | unqualified, in `no_runs_lines` |
| `src/ui/screens/driver_confirm.rs:176` | **2** | **yes** | both `super::`-qualified, both reached through a `format!` argument list — the multi-occurrence unit the innermost rule exists for |
| `src/ui/screens/driver_confirm.rs:303` | 1 | **yes** | `super::`-qualified |
| `src/ui/screens/driver_confirm.rs:341` | 1 | **yes** | `super::`-qualified, inside a closure |

**The fix regresses nothing:** all five are composed under the innermost rule exactly as they are under the joined rule, including both `super::`-qualified spellings, and the live census still reports **ZERO** sites over both files.

### NON-VACUITY 2 — before and after, both re-measured

| | total |
|---|---|
| **before** — needle-bearing non-comment lines over the **whole file** | **7** |
| **after** — needle occurrences over the **production slice the census walks** | **6** (across 5 logical units) |

**Why they differ, in one sentence:** the whole-file count included `src/ui/screens/driver.rs:2349`, this module's own test-module call to the sanitiser, which sits below the `#[cfg(test)]` truncation and which the census therefore never scans — so the old guarantee "there was something to find" was about a strictly larger set than the zero it was standing behind. All three totals (5 units, 6 occurrences, 7 whole-file lines) are committed as measured literals with a failure message telling the next reader to re-measure rather than edit.

### The truncation RED, from a planted second module marker

```text
thread 'ui::screens::driver::tests::no_executable_control_class_call_in_these_two_files_stands_outside_a_composition' (2985743) panicked at src/ui/screens/driver.rs:2457:13:
assertion `left == right` failed: src/ui/screens/driver.rs carries 2 column-zero `#[cfg(test)]` markers whose following declaration is a MODULE, at line(s) [2030, 4199]. The census cuts at the FIRST one, so a second means every line after it is dropped from the scan without anything saying so — the silent kill switch this assertion exists to make loud. Either fold the modules together or state, here, which one the census is meant to stop at and why.
  left: 2
 right: 1
```

The plant was `#[cfg(test)] mod planted_second_test_module {}` appended to `src/ui/screens/driver.rs`. It was removed immediately and **`rtk proxy git status --porcelain` showed ` M src/ui/screens/driver.rs` alone** — no stray file, no leftover marker — and the test returned green. The failure **names both marker lines**, which is the difference between a kill switch and a control: the reader is sent to the markers, not to the census.

**The hazard is live in this tree, verified here:** `src/ui/screens/detail.rs` carries two column-zero `#[cfg(test)]` markers, at `:5342` (followed by `pub(super) fn first_string_entry`) and `:5764` (followed by `mod tests`). Under the old first-`#[cfg(test)]`-line rule, adding that file to `CENSUS_FILES` would have silently dropped more than two thousand lines. Under the new rule the cut is at `:5764`, and the `pub(super) fn` helper stays **inside** the scanned slice — the stricter direction.

### The rewritten residual block, and its flipped direction

Quoted in full:

> **Residuals, each with its direction.**
>
> 1. **A composition assembled across separate statements** — the sanitiser bound to a local on one line, wrapped three lines later — reads as UN-composed and is reported. Under the old whole-unit verdict this was under-detection and silent; since 21-32 moved the verdict to the innermost call it is **over-detection, and LOUD** — it fails the build and names the file and line. Loud is the safe direction, and the direction changed on purpose. The repair for a loud false positive is to inline the composition, or to record an exemption with a reason — **never to soften the rule**.
> 2. **A third file added to this path tomorrow is simply not looked at.** `CENSUS_FILES` is two named files, and nothing here notices a new one. **Under-detection, silent.** What bounds that is the render-escape probe, which renders all four of this path's sites, and NOT this census.

**The direction changed from under-detection to over-detection, and over-detection here is loud** — a false positive fails the build and names the site, so it cannot be missed, only argued with. The old residual's under-detection was silent; that is the trade this repair deliberately makes.

### Anti-self-match, re-run and quoted as evidence

Every needle, composer and fixture token in both files is assembled at runtime. `rtk proxy grep -c "CENSUS_COMPOSERS" src/ui/screens/driver.rs` → **5** (required ≥ 3), and the fixture is built by `format!` from `CENSUS_NEEDLE_HEAD` / `CENSUS_NEEDLE_TAIL` / `CENSUS_COMPOSERS[0]`, so no source line spells the escape call with its opening parenthesis. The census's own anti-self-match arm (the needle-half declaration check) is inside `no_executable_control_class_call_in_these_two_files_stands_outside_a_composition`, which is green.

**21-33's coverage pin is protected:** `src/ui/screens/driver.rs` still contributes exactly **2** executable `display_identity(` occurrences (`:645`, `:944`) — this plan added none, because the composer is only ever referenced through `CENSUS_COMPOSERS`.

### Acceptance evidence

- `rtk proxy cargo test --lib -- ui::screens::driver::tests --nocapture` → `test result: ok. 49 passed; 0 failed; 0 ignored; 0 measured; 1056 filtered out`
- Live composition census green — **ZERO sites over both census files**.
- `rtk proxy cargo clippy -- -D warnings` → exit 0
- `rtk proxy cargo clippy --all-targets -- -D warnings` → exactly **FOUR** lints, before and after, quoted below.

## Whole-plan verification

### Clippy, before and after

**Before (`2c13fcf`) and after (`c84c16d`) — identical:**

```text
  1    --> src/browser.rs:155:9
  1    --> src/browser.rs:156:9
  1    --> src/browser.rs:157:9
  1    --> src/project_creator.rs:146:27
```

Three `bool_assert_comparison` ("used `assert_eq!` with a literal bool") and one `cmp_owned` ("this creates an owned instance just for comparison") — **exactly four, same two kinds, same two files, same four line numbers.** This plan edits neither `src/browser.rs` nor `src/project_creator.rs`. A count of three would have been a failure of this plan exactly as five would be.

The lib gate `rtk proxy cargo clippy -- -D warnings` exits **0**. `rtk proxy cargo build` exits **0**.

### Workspace test totals

`rtk proxy cargo test --workspace --no-fail-fast` → **1418 passed / 1 failed / 13 ignored** across 35 binaries.

**Every delta against the measured 1417 / 0 / 13 green baseline attributed:**

| Delta | Attribution |
|---|---|
| +1 passed | `text::tests::the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments` — added by Task 1 |
| +1 passed | `ui::screens::driver::tests::a_composed_arm_does_not_launder_its_un_composed_sibling` — added by Task 3 |
| −1 passed, +1 failed | `driver_reattach` — the documented flake, green on an isolated re-run (see below) |

Total accounted: 1417 + 2 = 1419 tests; 1418 passed + 1 failed = 1419. No unattributed delta.

### The flake, named and re-run

The single failure was `driver_reattach::a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`. Re-run alone:

```text
rtk proxy cargo test --test driver_reattach
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.12s
```

### Criterion 4 — no work claimed

`rtk proxy cargo test --test driver_injection_corpus` → **`ok. 13 passed; 0 failed; 10 ignored`**, matching the HEAD counts exactly, with `rtk proxy git diff --stat -- tests/driver_injection_corpus.rs` **empty** (file unchanged). No task in this plan claimed work against ROADMAP success criterion 4, which is permanently agent-unclosable by explicit user decision.

### Scope

`rtk proxy git diff --stat 2c13fcf HEAD`:

```text
 src/text.rs              | 399 +++++++++++++++++++++++++++++++++++++++++------
 src/ui/screens/driver.rs | 386 ++++++++++++++++++++++++++++++++++++++++-----
 2 files changed, 691 insertions(+), 94 deletions(-)
```

**Only the two declared `files_modified`.** `.planning/REQUIREMENTS.md` untouched. `tests/driver_injection_corpus.rs` untouched. `src/ui/screens/driver_confirm.rs` read by the census and edited by nobody. None of `21-31`'s or `21-33`'s files touched. `rtk proxy git status --porcelain` clean after the final commit.

## The pattern call, in this executor's own words

**gaps[1] — bounded mechanism fix plus a narrowed claim, and why disclosure alone would not have done.** The misplaced increment was not the census over-selling itself; it was the mechanism failing to honour the constant its own doc explains. `INTERPRETER_JOIN_LINES` is documented as a budget for *wrapped call* lines, and a comment is not one. Fixing it is *terminal* — it cannot need certifying next round — because what the repair leaves behind is a window with a **two-sided committed boundary test**: asserted at 11 and at 12, in both directions, in the comment arm and the executable arm. A window asserted at N and N+1 in both directions *is* its own certificate; there is no further measurement a later round could take that this one has not. Disclosure alone was rejected on consequence, not on taste: this census is the only committed thing standing between the tree and a re-introduced CR-01, and publishing "it stops looking after twelve comment lines" would have left the phase's sharpest execution-sink control knowingly blind while the disclosure sat in a doc nobody reads at the moment of the edit. The claim was narrowed anyway, in the same commit, because a repaired mechanism re-sold as broader is the exact defect this round exists to end.

**gaps[2] — a bounded fix that DELETES the window rather than widening it.** Over-joining here is a **laundering** defect, categorically worse than a reach that is merely narrower than its name: the census does not fail to look at a construction it never claimed, it looks directly at the construction it exists to catch and calls it clean because something *else* in the same joined unit was fine. The repair is terminal for a structural reason, not an optimistic one: the verdict moves off the joined text entirely and onto the characters immediately preceding each occurrence, so **there is no window left to over-join**. No future round can find a bigger unit that launders something, because no unit is consulted at all. A "smarter join" would have been the wrong shape — a smarter window is still a window. And the residual flips direction: a composition assembled across separate statements now reads as un-composed and is reported. That is over-detection, and it is **loud** — it fails the build and names the site, so the next author must argue with it rather than never learn of it. Loud is the safe direction to be wrong in.

**Neither gap was closed by a new census.** No new needle, no new file set, no new tree-wide scan, no new completeness claim. Prohibition 1 forbade it by name and the constraint never bound: both repairs were corrections to mechanisms that already existed, and both came with the claim narrowed in the same commit.

**Anti-pattern 2 was not available here and was not reached for.** Neither fix is shaped like "add these values to the list". The comment fix moves one statement; the composition fix deletes a `contains` over a window and replaces it with a rule about adjacency. The one place where "add to the list" *was* the tempting move — IN-05's five-marker set — was explicitly declined (D-21-53): the doc was narrowed to enumerate the five and name what they miss, because widening a substring detector adds false positives to a control whose entire value is that its zero can be trusted.

## Files Created/Modified

- `src/text.rs` — `interpreter_sites_in` extracted as the shared per-file walk; `INTERPOLATION_MARKERS` hoisted; `taken += 1` moved after the comment `continue`; `cr01_fixture` + `FIXTURE_FLAG_STEMS` + `FIXTURE_SPAWN_HEAD`/`_TAIL` (all assembled at runtime); the two-sided boundary control; the narrowed claim with three bounds and five directed residuals; `interpolates_into_a_string`'s and `has_an_unclosed_delimiter`'s docs corrected.
- `src/ui/screens/driver.rs` — `census_sites_in` extracted; `occurrence_is_composed` + `strip_trailing_path_qualifiers` as the innermost verdict; `test_module_marker_lines` + `declares_a_module` + `production_slice` for the loud truncation; the exactly-one-marker assertion; NON-VACUITY 2 recomputed over the scanned slice with three committed literals; `laundering_fixture` and its control; the residual block rewritten to its new over-detection direction.

## Decisions Made

All five planned decisions were taken as written; none required re-opening, and none is `one-way`.

- **D-21-51 / D-21-52** — gaps[1] closed by fix + narrowed claim; the budget counts executable lines and comments are free at any count. Confirmed by measurement in both states, including at 100 comment lines.
- **D-21-53** — IN-05 closed by narrowing the doc rather than widening the marker set.
- **D-21-54** — gaps[2] closed at the innermost call rather than by a smarter join.
- **D-21-55** — truncation cuts at a column-zero test MODULE, exactly one asserted per file. Both census files carry exactly one; `detail.rs` proved the two-marker hazard is real.

Every change in this plan sits inside a `#[cfg(test)]` module or a doc comment. **No production line in either file changed**, so nothing here alters a shipped binary's behaviour, and everything is revertible from git with no migration.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The CR-01 fixture spelled the spawn seam's own needle**

- **Found during:** Task 3's whole-workspace verification (introduced by Task 1)
- **Issue:** Task 1's in-memory fixture opened with the raw literal `"match std::process::Command::new(&term)".to_string()`. That is an **executable** line in `src/text.rs` spelling `Command::new(` — the needle `tests/spawn_seam_guard.rs` walks `src/` for. That guard requires every file carrying it to be on `SPAWN_ALLOWLIST`, so the workspace run went red:
  ```text
  thread 'every_process_spawn_site_in_src_is_on_the_allowlist' (3044959) panicked at tests/spawn_seam_guard.rs:451:5:
  a process-spawn site appeared in a file that is not on the allowlist. Confirm it takes a capability type and then add it to `SPAWN_ALLOWLIST` in this file, in the same commit (PITFALLS:521). Unexpected: ["src/text.rs"]
  ```
- **Fix:** The opener is split across the sibling needle — `FIXTURE_SPAWN_HEAD` / `FIXTURE_SPAWN_TAIL`, meaningless apart — and assembled at runtime, the same anti-self-match idiom as `INTERPRETER_STEMS` but applied to a *sibling* census. The **wrong** repair, deliberately not taken, would have been adding `src/text.rs` to `SPAWN_ALLOWLIST`: that file spawns nothing, and the allowlist is the record of which files really do. Weakening a real guard to accommodate a test fixture is precisely the move this phase exists to stop.
- **Files modified:** `src/text.rs` (test module + consts only)
- **Verification:** `rtk proxy cargo test --test spawn_seam_guard` → `ok. 38 passed; 0 failed`; `rtk proxy cargo test --lib -- text::tests` → `ok. 21 passed; 0 failed`, boundary control still reporting the fixture at `fixture.rs:2`.
- **Committed in:** `a64005b` (its own commit, so the correction is legible rather than buried in Task 3)

---

**Total deviations:** 1 auto-fixed (1 bug).
**Impact on plan:** None on scope — the fix stayed inside `src/text.rs`, one of this plan's two declared files, and changed only test-module code. It is in fact the plan's own prohibition 3 generalising one step further than the plan stated it: prohibition 3 named this plan's *own* censuses and `21-33`'s pin, and the same hazard turned out to apply to `tests/spawn_seam_guard.rs`. Worth recording for round 12: **a fixture token spelled whole is a hit in whichever census reads that file, not merely in the census the fixture belongs to.**

## Issues Encountered

**The `driver_reattach` flake landed on a different test than the planner's baseline — reported, not smoothed.** The plan's `<tooling_note>` records the baseline failure as `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`. In this executor's run the failing test was `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired` (`tests/driver_reattach.rs:461`), and `a_fresh_scan_...` passed. Both are in the same binary and both are consistent with the corrected mechanism the plan states: a **system-wide `/proc` scan that does not stop at the process-group or worktree boundary**. With three sibling executors running in parallel worktrees this round, cross-contamination is expected and can land on either test. The isolated re-run was green (`3 passed; 0 failed`), so this is the documented flake and not a regression — but the fact that it is **not always the same test** is new information relative to the plan's record and belongs in `21-34`'s correction of the flake note.

**No other issues.** No architectural decision was reached, no checkpoint was hit, no auth gate fired, and the fix-attempt limit was never approached (one deviation, fixed on the first attempt).

## Known Stubs

None. Every mechanism this plan touches was observed RED before green, and every red is committed verbatim — in the source doc, in the commit message and in this SUMMARY.

## Residuals this plan leaves, each with its direction

Restated in one place so the next round does not have to reconstruct them:

| Residual | Direction |
|---|---|
| Interpreter census: construction assembled across statements | under-detection, **silent** |
| Interpreter census: interpreter named by a variable, not a literal | under-detection, **silent** |
| Interpreter census: method chains not followed | under-detection, **silent** |
| Interpreter census: four of five string-building forms unseen | under-detection, **silent** |
| Interpreter census: `execute_hook`'s error-message-only interpolation | **deliberate exclusion**, not a gap |
| Composition census: composition assembled across statements | **over-detection, LOUD** (direction flipped this round) |
| Composition census: a third file added to this path is not looked at | under-detection, **silent** |

**SAFE-07 precision** carries no arithmetic contract: neither census performs arithmetic on quantities. The only counts either produces are exact `usize` line and occurrence counts compared by equality, and every equality names its offending sites in the failure message rather than reporting a magnitude — so there is no rounding, tie-breaking, overflow or precision-loss behaviour to state. Recorded as a backstop marker per the plan's `<edge_probe_audit>`.

## User Setup Required

None — no external service configuration, no new dependency, no `Cargo.toml` or `Cargo.lock` change. The Package Legitimacy Audit gate does not fire.

## Next Phase Readiness

- **Ready.** Both mechanisms this plan owns are repaired, measured and claim-narrowed. `21-33`'s coverage pin over `src/ui/` is protected: `driver.rs` still contributes exactly 2 executable `display_identity(` occurrences.
- **For the wave merge:** if `21-33`'s per-file pin goes red after the merge, that is a finding to report — this plan did not change the count, and the number should not be quietly updated.
- **For `21-34`:** the flake note needs a second correction beyond the `--test-threads=1` one already planned — the failure is **not always the same test** in `driver_reattach`. Evidence is in "Issues Encountered".
- **For round 12:** the deviation above generalises prohibition 3 — a fixture token spelled whole is a hit in *whichever* census reads that file, including censuses in `tests/` that the plan never named.
- **ROADMAP criterion 4** remains permanently agent-unclosable by explicit user decision; 4/5 is the expected ceiling and no work here was claimed against it.

## Self-Check

**Files claimed as modified, verified on disk:**

- `src/text.rs` — FOUND
- `src/ui/screens/driver.rs` — FOUND
- `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-32-SUMMARY.md` — FOUND

**Commits claimed, verified in `git log`:**

- `b644c32` — FOUND
- `e18019c` — FOUND
- `a64005b` — FOUND
- `c84c16d` — FOUND

**Plan-level `<verification>` re-run:** `cargo build` exit 0; workspace 1418/1/13 with the single failure named as the documented flake and green in isolation; lib clippy exit 0; all-targets clippy exactly four pre-existing lints; four verbatim REDs captured with a clean `git status --porcelain` after the one plant; corpus at 13/0/10 and unchanged; `git diff --stat` naming only the two declared files; `.planning/REQUIREMENTS.md` untouched.

## Self-Check: PASSED

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-28*
