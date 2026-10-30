---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 12
subsystem: infra
tags: [security, envelope, pretooluse-guard, property-testing, generative-corpus, threat-t-19-60, threat-t-19-74]

# Dependency graph
requires:
  - phase: 19-gitsafe-git-blast-radius-envelope
    provides: "`policy::resolve_program`, `policy::GOVERNED_PROGRAMS`, `classify_segments` on the resolved program index, and the six enumerated bypass rows in `tests/envelope_wrapper_bypass.rs` — all delivered by 19-11"
provides:
  - "`tests/envelope_wrapper_class.rs` — the control over the CLASS T-19-60 belongs to, rather than over the six lines `19-SECURITY.md` measured"
  - "a hand-rolled fixed-seed LCG generator (no crate) composing assignment prefixes x wrapper chains x path spellings x quoting x optional `-c` nesting"
  - "the invariance property `verdict(wrap(base)) == verdict(base)` over 1680 refused cases, compared on exit code AND D-24 reason identifier"
  - "the paired allow corpus over 1080 permitted cases through the same generator (D-32)"
  - "the alphabet-absence control: 12 wrapper names mechanically proved absent from the production logic of `policy.rs` and `hooks.rs`"
  - "a positive control proving each `include_str!` file is the file the scanner thinks it is — an ADDITION made during execution, not a plan requirement"
  - "the two-sided pin over the accepted `T-19-74` residual, including the command-line boundary where it begins and the doc-disclosure pin"
affects: [gsd-secure-phase-19, phase-20-router]

# Actuals (#2632)
actuals:
  # chars/4 over the realized diff, which is the whole of the one created file
  # (51 383 chars / 4). The plan estimated 80 000 on the same scale.
  tokens: 12846
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "An invariance property whose right-hand side is MEASURED from the unwrapped command in the same run, so the property tightens rather than loosens if the bases stop being refused"
    - "Non-vacuity floors (case count, distinct-chain count, distinct-reason count, non-empty corpus) asserted BEFORE the loop they bound"
    - "A generative alphabet deliberately containing names that appear nowhere in the production logic, plus a mechanical control asserting their absence — so a fix that became a list turns its own corpus red"
    - "A POSITIVE control beside every absence control: an absence assertion cannot tell 'the name is not in this file' from 'this is not the file I think it is'"
    - "Wrapper-name absence measured over PRODUCTION CODE (comments and `#[cfg(test)]` stripped), because the doc comments that name wrappers exist precisely to record that they must not be enumerated"

key-files:
  created:
    - tests/envelope_wrapper_class.rs
  modified: []

key-decisions:
  - "The alphabet-absence control runs over production code, not raw bytes: 8 of the 12 designated names appear in `policy.rs`'s doc comments and unit tests, where they are the disclosure and the control rather than the failure mode"
  - "The positive control (anchor per included file) is ordered BEFORE the absence assertions, so the file is proved to be the right file before anything is concluded from what is missing from it"
  - "`X=git; env $X push --force` (semicolon) is PERMITTED and is pinned as such: step 7 closes the SAME-command-line binding, and a `;` makes it a different command line — which is the T-19-74 residual by definition"
  - "The full-suite number requires `--no-fail-fast`: a plain `cargo test` stops at `driver_reattach` and never reaches any `envelope_*` binary"

requirements-completed: [SAFE-02, SAFE-06]

coverage:
  - id: D1
    description: "The guard's verdict is invariant under any wrapper chain, assignment prefix, path spelling, quoting or `-c` nesting, over 1680 generated refused cases from a named fixed seed, across 773 distinct wrapper chains and 3 distinct D-24 reasons"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#the_verdict_is_invariant_under_every_generated_wrapping_of_a_refused_command"
        status: pass
      - kind: other
        ref: "measured with `rtk proxy cargo test --test envelope_wrapper_class -- --nocapture`: `refused corpus: 1680 generated cases, 773 distinct wrapper chains, 3 distinct D-24 reasons, seed 0x1912c0de5eed0060`"
        status: pass
    human_judgment: false
  - id: D2
    description: "At least ten wrapper names the fix must not know are mechanically proved absent from the production logic of `policy.rs` and `hooks.rs`, four of them from the raw bytes as well"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic"
        status: pass
      - kind: other
        ref: "fail-first proof — adding `GOVERNED_PROGRAMS` (a string that IS in production code) to the designated list turns the control red naming that string and `src/envelope/policy.rs`; observed and reverted"
        status: pass
    human_judgment: false
  - id: D3
    description: "Each `include_str!` file is proved to be the file the scanner thinks it is, before any absence is concluded from it — the ADDITION made during execution"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic (the positive-control block, ordered first)"
        status: pass
      - kind: other
        ref: "fail-first proof — repointing `HOOKS_SOURCE` at `../src/envelope/policy.rs` (a wrong-but-EXISTING path, which `include_str!` accepts) turns the control red naming `fn classify_segments(`; observed and reverted"
        status: pass
    human_judgment: false
  - id: D4
    description: "The fix cannot be a blanket denial: 1080 wrapped permitted cases exit 0 with empty stdout, and `rg \"git status\" src/` runs while `rg \"git push --force\" src/` is refused"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#the_permitted_corpus_survives_every_generated_wrapping_and_still_answers_nothing"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#a_search_for_an_allowed_git_command_runs_and_a_search_for_a_refused_one_does_not"
        status: pass
      - kind: other
        ref: "measured: `permitted corpus: 1080 generated cases, 533 distinct wrapper chains, seed 0x1912c0de5eed0060`"
        status: pass
    human_judgment: false
  - id: D5
    description: "The behaviours 19-11 generalised rather than replaced still hold, including `bash -lc` — the bundled short-flag spelling the deleted exact-`-c` list did not cover"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#a_force_push_inside_a_shell_payload_is_still_refused_in_every_spelling_of_the_flag"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#a_command_the_guard_cannot_see_the_program_of_is_refused_naming_why"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#a_dash_c_payload_whose_quoting_cannot_be_recovered_is_permitted_when_it_governs_nothing"
        status: pass
    human_judgment: false
  - id: D6
    description: "The accepted `T-19-74` residual is pinned as permitted with its identifier, its three bounds are pinned as refused, and the command-line boundary where the residual begins is pinned on both sides"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#the_t_19_74_residual_is_permitted_and_the_cost_of_closing_it_is_measured_beside_it"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#the_bounds_of_the_t_19_74_residual_are_refused_which_is_what_makes_it_narrow"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#the_residual_begins_exactly_at_the_command_line_boundary"
        status: pass
    human_judgment: false
  - id: D7
    description: "The disclosure in the code and the disclosure in the test cannot drift apart: `resolve_program`'s doc still names `T-19-74`, the shape, that it is permitted, and `T-19-75`"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover"
        status: pass
      - kind: other
        ref: "anti-vacuity measured — the extracted doc region is 3364 bytes; temporarily raising the floor to 100 000 printed the whole extracted disclosure, confirming the extractor reads the real paragraph and not an empty string; reverted"
        status: pass
    human_judgment: false
  - id: D8
    description: "SAFE-06's wrapped-PR-creation coverage is deliberately NOT re-asserted generatively — write-before-permit would make invariance pass for the wrong reason — and remains covered by the enumerated ledger row 19-11 delivered"
    requirement: SAFE-06
    verification:
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#a_wrapped_pull_request_is_counted_written_to_the_ledger_and_parked_on_the_second_attempt (pre-existing, unmodified, still green)"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_class.rs#the_permitted_corpus_survives_every_generated_wrapping_and_still_answers_nothing — `gh pr list --limit 5` is wrapped 120 times and stays a permit that writes no ledger line"
        status: pass
    human_judgment: false

duration: 41 min
completed: 2026-08-29
status: complete
---

# Phase 19 Plan 12: The Control Over the T-19-60 Class Summary

**A fixed-seed generative property now asserts that the `PreToolUse` guard's verdict is INVARIANT under 1680 machine-composed wrappings of twelve refused git commands — thirty wrapper spellings chained 0–3 deep, four assignment prefixes, basename and absolute-path forms, three quoting styles and an optional `sh -c` / `bash -lc` payload layer — compared against the unwrapped verdict measured in the same run, paired with 1080 permitted cases through the identical generator, and fenced by a control that proves twelve wrapper names appear nowhere in the production logic of `policy.rs` or `hooks.rs`.**

## Performance

- **Duration:** 41 min
- **Started:** 2026-08-29T05:05:00Z
- **Completed:** 2026-08-29T05:46:00Z
- **Tasks:** 3 of 3
- **Files modified:** 1 (1 created, 0 modified)

## The measured numbers

All from `rtk proxy` runs. The global RTK hook strips `warning:` and `test result:` lines, so an unproxied grep for them succeeds having read nothing (D-34, and 19-08's `T-19-51`); no count below comes from a filtered run.

| Quantity | Measured |
|---|---|
| **Fixed seed** | `const SEED: u64 = 0x1912_C0DE_5EED_0060` (printed as `0x1912c0de5eed0060`) |
| Refused generated cases | **1680** (12 bases × 140 variants; floor 1500) |
| Distinct wrapper chains, refused corpus | **773** (floor 200) |
| Distinct D-24 reasons across the refused bases | **3** — `force_push_blocked`, `hook_bypass_blocked`, `push_outside_namespace` (floor 3) |
| Permitted generated cases | **1080** (9 bases × 120 variants; floor 1000) |
| Distinct wrapper chains, permitted corpus | **533** |
| Wrapper alphabet size | 30 spellings, chained 0–3 deep |
| Extracted `resolve_program` doc region | **3364 bytes** (anti-vacuity floor 400) |
| `tests/envelope_wrapper_class.rs` | 1181 lines, 11 tests, runs in **0.2 s** |

The two `println!` lines the property emits, verbatim from `rtk proxy cargo test --test envelope_wrapper_class -- --nocapture`:

```
refused corpus: 1680 generated cases, 773 distinct wrapper chains, 3 distinct D-24 reasons, seed 0x1912c0de5eed0060
permitted corpus: 1080 generated cases, 533 distinct wrapper chains, seed 0x1912c0de5eed0060
```

## The designated absent wrapper names

Twelve, asserted absent from the **production logic** of both `src/envelope/policy.rs` and `src/envelope/hooks.rs`:

`stdbuf`, `setsid`, `ionice`, `chrt`, `taskset`, `doas`, `runuser`, `unshare`, `firejail`, `torsocks`, `catchsegv`, `made-up-wrapper-9000`

Four of those are additionally asserted absent from the **raw bytes** of both files — comments and test module included — so the control does not rest entirely on the comment stripper being correct:

`unshare`, `firejail`, `torsocks`, `catchsegv`

The short names in the wider alphabet (`env`, `command`, `time`, `nice`, `sudo`, `proot`, `flock`) are deliberately excluded and the exclusion is stated in the file's own doc: they are substrings of ordinary English and ordinary Rust (`environment`, `command line`, `timestamp`), so asserting their absence would be asserting something about prose rather than about logic.

## THE ADDITION MADE DURING EXECUTION — the positive control

**This was NOT called for by `19-12-PLAN.md`.** The plan specifies only the absence assertions. The orchestrator required this addition on dispatch, and it is recorded here as an addition rather than as a plan requirement.

**The hole it closes.** `include_str!` fails the build on a MISSING path but not on a WRONG-BUT-EXISTING one. Point it at a different source file — or at the same file twice — and every absence assertion passes having certified nothing. Absence assertions alone cannot distinguish *"the name is not in this file"* from *"this is not the file I think it is."*

**What was added.** For each included file, an assertion that a string known to be present in THAT file IS found, ordered strictly BEFORE the absence assertions so the file is proved to be the right file before anything is concluded from what is missing from it. The exact anchors, both verified against the live tree before use:

| Included file | Anchor string | Where it lives | Occurrences in the OTHER included file |
|---|---|---|---|
| `src/envelope/policy.rs` | `fn resolve_program` | `policy.rs:1387`, `pub fn resolve_program(segment: &[Token]) -> ProgramResolution {` | 0 |
| `src/envelope/hooks.rs` | `fn classify_segments(` | `hooks.rs:950`, `fn classify_segments(` — the function 19-11 rewired onto the resolver | 0 |

Neither anchor had moved, so no substitute was needed. Each anchor is asserted four ways:

1. present in the file's **raw** `include_str!` content (proves the path);
2. **absent** from the other included file (proves the two are distinguishable — the wrong-but-existing-path mistake);
3. present in the **production-stripped** text (proves the stripper did not eat the logic the absence assertions then run over);
4. the stripped text is more than a quarter of the raw bytes (proves the stripper did not truncate the file).

**What a red here means, in the failure message itself:** the `include_str!` path is wrong or the file was renamed or the anchor moved — and therefore EVERY absence assertion in that test is vacuous, because a name is trivially absent from a file that is not the one being examined. The message says to fix the path or the anchor and not to delete the assertion.

**Proved fail-first.** `HOOKS_SOURCE` was temporarily repointed at `../src/envelope/policy.rs` — a wrong path that exists, which `include_str!` accepts silently. The control went red:

```
POSITIVE CONTROL FAILED for `src/envelope/hooks.rs`: its `include_str!` content does not
contain `fn classify_segments(`, which that file is known to define. A red here means the
`include_str!` path is wrong or the file was renamed or the anchor moved — and therefore
that EVERY absence assertion in this test is vacuous …
```

Observed, then reverted. Without this control that same misconfiguration would have produced a green run.

## Accomplishments

- **The control is over the CLASS, and it is generative.** Thirty wrapper spellings — including `/usr/bin/env`, `busybox env`, `env -i`, `runuser -u me --`, `nsenter --` and `made-up-wrapper-9000 --flag`, a program that does not exist — chained 0 to 3 deep, drawn by a five-line hand-rolled LCG from one named seed, crossed with four assignment prefixes, three quoting styles and an optional shell-payload layer.
- **The invariance right-hand side is measured, not written down.** Each base's verdict is observed unwrapped in the same run and asserted to be a refusal carrying a `REASON_*` identifier BEFORE the wrapping loop. A suite that quietly stopped refusing the bases fails here rather than passing more easily.
- **The verdict identity is exit code AND D-24 reason identifier**, the identifier extracted from the `(reason: …)` marker in `permissionDecisionReason` rather than the whole message — so a detail that legitimately quotes a token out of the command it refused is not mistaken for a change of verdict.
- **Four non-vacuity floors are asserted before the loop they bound**: ≥1500 cases, ≥200 distinct chains, a non-empty base corpus with every base refused unwrapped, and ≥3 distinct D-24 reasons.
- **The paired allow corpus is what makes the refusal half mean anything** (D-32, `T-19-78`): 1080 wrapped cases over 9 bases, each asserting exit 0 **and empty stdout** — a permit answers nothing at all, which is what keeps a deny-only control from becoming an approval authority.
- **`bash -lc "git push --force origin main"` is pinned refused.** This is the row that proves 19-11's deletion of `NESTED_SHELLS` WIDENED coverage rather than merely moving it: the five-name list matched an exact `-c` and did not cover the bundled login-flag spelling.
- **The `T-19-74` residual is bounded on both sides, and the cost of closing it is measured beside it.** `echo $(git rev-parse HEAD)` and `cd "$HOME"` are asserted still permitted in the same test as the residual, so a future decision to close it meets the bill in the same place as the benefit.
- **The residual's exact edge is now pinned.** `X=git env $X push --force` (same command line) is refused; `X=git; env $X push --force` (different command line) is permitted. Pinning only one side would let the boundary move unnoticed in either direction.
- **No crate was added.** `git diff 15b406a..HEAD -- Cargo.toml Cargo.lock` is empty, so `T-19-SC` still holds phase-wide.

## Task Commits

1. **Task 1: the fixed-seed generator, the invariance property, the alphabet-absence control and the positive control** — `9543da7` (test)
2. **Task 2: the paired allow corpus, the `T-19-75` discrimination pair, and the pins over the generalised behaviours** — `af9d2e0` (test)
3. **Task 3: the `T-19-74` residual bounded on both sides, its command-line boundary, and the doc-disclosure pin** — `fea96bc` (test)

`git diff --stat 15b406a..HEAD` → `tests/envelope_wrapper_class.rs | 1181 insertions(+)`, **one file, nothing else**.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — internal inconsistency in the plan text] The alphabet-absence control runs over PRODUCTION CODE, not raw file bytes**

- **Found during:** Task 1, before writing the control, by measuring the twelve designated names against the live files.
- **Issue:** The plan's must-have says the control "reads `src/envelope/policy.rs` and `src/envelope/hooks.rs` and asserts that at least ten of the alphabet's names … are absent from both files." Taken literally over the raw bytes, that control is **RED today** — and for the exact opposite of the reason it exists. Measured occurrences in `policy.rs`:
  - `stdbuf`, `setsid`, `ionice`, `chrt`, `taskset`, `doas`, `runuser` appear in the **doc comments** of `GOVERNED_PROGRAMS` (lines 1166–1167) and `resolve_program` (line 1339), where they are listed *precisely to record that the set of things which can precede a program is open and must therefore NOT be enumerated*;
  - `made-up-wrapper-9000` appears four times inside the `#[cfg(test)]` unit-test module (lines 2573–2676), as 19-11's own class-level fixture.
  - Only 4 of the 12 (`unshare`, `firejail`, `torsocks`, `catchsegv`) are absent from the raw bytes — not the ten the plan requires.
  A disclosure and a control are the opposite of the failure mode this test defends against, and no ten-name subset of the plan's own list satisfies the literal reading.
- **Fix:** The control runs over the file's production logic — `//`-prefixed lines dropped, everything from the trailing `#[cfg(test)]` attribute onward dropped. That is the same fact 19-11's audit measured with `grep -v '^\s*//'` ("no wrapper-name literal in non-comment `src/` or `tests/` code"), made mechanical. All twelve names are absent from the stripped text of both files. The reasoning is written into `production_code`'s own doc comment rather than left here. **The control was additionally strengthened**, not merely relaxed: the four raw-absent names are asserted against the *unprocessed* bytes, so the control does not rest entirely on the stripper being correct, and the positive control (above) proves the stripper preserved the logic.
- **Verification:** Proved fail-first — adding `GOVERNED_PROGRAMS` (a string that IS in production code) to the designated list turned the control red naming that string and `src/envelope/policy.rs` with the full "adding a name to a list is the failure mode" message; observed and reverted.
- **Committed in:** `9543da7`

**2. [Rule 1 — internal inconsistency in the plan text] `X=git; env $X push --force origin main` is PERMITTED, and the plan's own rationale says why**

- **Found during:** Task 3 preparation, by measuring the candidate rows against the built guard before writing them.
- **Issue:** The plan's Task 3 and its success criterion 4 require `X=git; env $X push --force origin main` to be REFUSED. **Measured: exit 0, permitted.** The plan's own stated rationale for that bound is *"binding a governed program name in a command the guard cannot otherwise resolve is refused, so the **same-command-line** route is closed and only a binding from outside remains"* — and `resolve_program`'s doc says the same: *"step 7 refuses binding a governed program name to a variable **in the same command line**."* A `;` makes the binding a *different* command line, which is by definition the `T-19-74` residual rather than a bound on it. The two readings cannot both hold.
- **Fix:** Followed the normative rationale. Bound 2 is pinned as `X=git env $X push --force origin main` — **no semicolon** — which is measured REFUSED under `policy::REASON_ENVELOPE_ASSERTION_FAILED`. That is the bound the plan's own sentence describes. **Then the semicolon form was pinned too**, in a dedicated test (`the_residual_begins_exactly_at_the_command_line_boundary`) asserting it PERMITTED and explaining that a `;` is a different command line and the guard has no memory of the last one. This is strictly *stronger* than the plan asked for: it pins BOTH sides of the boundary, so the edge cannot move outward unnoticed (which pinning only the refusal would allow) and cannot move inward unnoticed (which pinning only the permit would allow).
- **Verification:** Both rows measured against `hooks::guard_in` before being written, and both green in the committed file. The three bounds the plan asks for are all present and refused: `GIT_CONFIG_COUNT=0 env $X push --force` (`hook_bypass_blocked`), `X=git env $X push --force` (`envelope_assertion_failed`), `unset GIT_CONFIG_COUNT && git push --force` (`hook_bypass_blocked`).
- **Committed in:** `fea96bc`

**3. [Rule 1 — measurement correction] The full-suite number requires `--no-fail-fast`**

- **Found during:** Task 3's project gate.
- **Issue:** `cargo test` stops at the first failing test *binary*. `tests/driver_reattach.rs` fails (the documented pre-existing pair), and `envelope_*` sorts alphabetically after `driver_*` — so a plain `cargo test` **never runs this plan's new file at all** and reports `1245 passed / 2 failed / 13 ignored`, numerically identical to the pre-19-12 baseline. Reporting that number as "the suite after 19-12" would have been reporting a run that never executed the work.
- **Fix:** The gate number is taken from `rtk proxy cargo test --no-fail-fast`: **1470 passed / 2 failed / 13 ignored**, with all 33 test binaries and the doc-tests executed. The 2 failures are the same documented `driver_reattach` pair and nothing else.
- **Verification:** `grep -E "^    [a-z_:]+$" | sort -u` over the full output returns exactly `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` and `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`.
- **Committed in:** `fea96bc` (the fact is recorded in the commit body)

---

**Total deviations:** 3 auto-fixed (2× Rule 1 correcting an internal inconsistency in the plan text, 1× Rule 1 correcting a measurement method).
**Impact on plan:** No scope creep, no relaxation. Deviations 1 and 2 each replace a literal instruction that measurement showed to be false with the plan's own stated rationale, and each ends up asserting MORE than the literal instruction would have (a raw-bytes subset in one case, both sides of a boundary in the other). Deviation 3 changes only how a number is obtained, and the corrected number is worse-looking and truer. **No production file was touched, no pre-existing test was modified, no crate was added, and no wrapper name entered `src/`.**

## Scope discipline — what was deliberately NOT touched

- **`src/` is byte-identical to `15b406a`.** `git diff --stat 15b406a..HEAD` lists one file: `tests/envelope_wrapper_class.rs`.
- **`tests/envelope_wrapper_bypass.rs` and every other pre-existing test are unmodified.** This plan adds controls over behaviour 19-11 already delivered; no control went red, so no finding about the fix arose.
- **No crate.** `Cargo.toml` / `Cargo.lock` untouched (`T-19-SC`). The generator is a five-line LCG against `std`.
- **No wrapper name entered `src/`.** Re-verified after all three commits: all twelve designated names return zero hits against the production logic of both files.
- **A refspec-less `push` is deliberately absent from the generative corpus** (`T-19-80`). `policy::push_needs_resolved_dests` is true only for that shape and makes the guard shell out to `git`; at 1680 cases that is minutes of runtime and a verdict that depends on the repository the test happens to run in. The exclusion is stated in the file, and the shape stays covered by an enumerated row in `tests/envelope_wrapper_bypass.rs`.
- **No PR-creating forge command is in the generative corpus.** Write-before-permit (D-20) parks the second `gh pr create` in a run under `pr_cap_exceeded` regardless of wrapping, which would make invariance pass for entirely the wrong reason. `gh pr list --limit 5` is in the allow corpus instead; `env gh pr create` remains an enumerated ledger row in `tests/envelope_wrapper_bypass.rs`.
- **`19-SECURITY.md` was not edited.** Re-running the audit is what should flip `threats_open` / `status`, not the plan that added the control writing its own verdict into the audit (D-25).

## Verification / Final gate numbers

All measured with `rtk proxy`.

| Gate | Result |
|---|---|
| `rtk proxy cargo build` | Finished, **exit 0** |
| `rtk proxy cargo test --test envelope_wrapper_class` | **11 passed / 0 failed / 0 ignored**, exit 0, 0.20 s |
| `rtk proxy cargo test --no-fail-fast` | **1470 passed / 2 failed / 13 ignored** — the 2 are the documented pre-existing `driver_reattach` pair |
| `rtk proxy cargo test` (fail-fast, for the record) | stops at `driver_reattach`, reports 1245/2/13, never reaches any `envelope_*` binary |
| `rtk proxy cargo clippy -- -D warnings` | **exit 0** |
| `rtk proxy cargo clippy --all-targets` | **4 lints, unchanged** — 3× `assert_eq!` with a literal bool (`src/browser.rs:155–157`), 1× owned-instance-for-comparison (`src/project_creator.rs:146`). None in this plan's file. (`grep -c '^warning: '` returns 5 because it also matches the `generated N warnings` summary line — the phase's recorded "5" is that grep count.) |
| `git diff --stat 15b406a..HEAD` | `tests/envelope_wrapper_class.rs \| 1181 +++++`, 1 file changed |
| `git diff 15b406a..HEAD -- Cargo.toml Cargo.lock` | empty |
| wrapper names in production logic of `policy.rs` / `hooks.rs` | **0 of 12** |

## Plan success criteria, re-run

1. ✅ Invariance passes over **1680** refused and **1080** permitted generated cases from the named seed `0x1912_C0DE_5EED_0060`, across **773** distinct wrapper chains and **3** distinct D-24 reasons.
2. ✅ **12** wrapper names mechanically proved absent from the production logic of `policy.rs` and `hooks.rs`; 4 of them from the raw bytes too.
3. ✅ `rg "git status" src/` permitted and `rg "git push --force" src/` refused under `force_push_blocked`, asserted as a pair in one test.
4. ✅ `T-19-74` pinned as permitted with its identifier and the cost of closing it measured beside it; its three bounds pinned as refused — with bound 2 spelled as the same-command-line binding the plan's own rationale describes, and the semicolon form pinned as the permitted other side of the boundary (deviation 2).
5. ✅ (orchestrator addition) The positive control proves each included file is the file the scanner thinks it is, ordered before the absence assertions, and was proved fail-first.

## Issues Encountered

**The 2 suite failures are the pre-existing `driver_reattach` pair and nothing else.** `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` and `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired` are documented in this phase's `deferred-items.md`, proved pre-existing at `0a84023` (the commit immediately before Phase 19) and re-proved by 19-11 by reverting `hooks.rs`. Out of scope here: this plan touches no production file at all, so it cannot be their cause. Not fixed, not edited.

**No control went red.** Every generated case, every pin and every disclosure assertion passed on first run against the tree 19-11 delivered. There is therefore **no finding about the fix** to report — which is the outcome a green class-level control is supposed to mean, and the two fail-first proofs above are what establish that it means it.

## Known Stubs

None. No hardcoded empty value, placeholder string, `TODO` or `FIXME` was introduced, and every test in the new file asserts against a measured verdict rather than a constant.

## Threat Flags

No new attack surface. This plan adds one test file and touches no production code, no network path, no filesystem path outside a `TempDir`, and no crate.

The plan's own threat register is discharged as follows:

| Threat | Disposition | Evidence |
|---|---|---|
| `T-19-76` — a corpus incapable of failing on its own class | mitigated | 12 names asserted absent from production logic; fail-first proved with `GOVERNED_PROGRAMS` |
| `T-19-77` — a property that passes by generating nothing | mitigated | 4 floors asserted before the loop; RHS measured from the unwrapped base in the same run |
| `T-19-78` — a fix certified by a suite that would pass if everything were denied | mitigated | 1080 wrapped permits with empty stdout, plus the `rg` discrimination pair |
| `T-19-79` — a disclosed residual deleted without notice | mitigated | doc-disclosure pin over a 3364-byte extracted region, anti-vacuity floor asserted first |
| `T-19-80` — a property that consults a repository per case | mitigated | refspec-less push excluded, one `TempDir` for the whole property, 0.2 s total |
| `T-19-SC` — package-manager installs | mitigated | hand-rolled LCG; `Cargo.toml`/`Cargo.lock` byte-identical |

**One gap the orchestrator's required addition closed that the plan's register did not name:** an absence control certified by an `include_str!` pointed at the wrong-but-existing file. That is a `T-19-76`-shaped spoof one level up — the control certifying the control — and it is now itself controlled.

## Self-Check: PASSED

- `tests/envelope_wrapper_class.rs` — FOUND on disk (1181 lines, 11 tests).
- `9543da7` (Task 1) — FOUND in `git log`.
- `af9d2e0` (Task 2) — FOUND in `git log`.
- `fea96bc` (Task 3) — FOUND in `git log`.
- All 5 success criteria re-run and confirmed after the final commit.
- Plan `<verification>` re-run: build/test/clippy green; `git diff --stat` touches one file; no wrapper name in `src/`.

## Next Phase Readiness

**Phase 19 has all 12 plans summarised.** `T-19-60` is closed structurally by 19-11 and now certified at the class level rather than at the instance level.

**Blockers/concerns for whoever goes next:**

1. **Re-run `/gsd-secure-phase 19`.** `19-SECURITY.md` still carries `threats_open: 1` / `status: blocked` and a Sign-Off block that predates 19-11. Neither 19-11 nor 19-12 edited it, deliberately (D-25). Both `T-19-60` and the `T-19-74` pin it names as this plan's job are now done.
2. **`T-19-61` through `T-19-73` remain open and UNACCEPTED** below the `high` threshold — unremediated findings awaiting disposition, unchanged by this plan.
3. **Use `--no-fail-fast` for any phase-19 suite number.** A plain `cargo test` stops at `driver_reattach` and silently never reaches any `envelope_*` binary, so it will report a number that looks like a baseline no matter what was added.
4. **The `driver_reattach` pair still fails deterministically on this host.** Pre-existing, out of scope, documented in `deferred-items.md`.
5. **If a future change closes `T-19-74`**, three things must be deleted deliberately and together: the residual pin, the boundary test, and the disclosure paragraph in `resolve_program`'s doc — the doc-disclosure pin will go red until the last of those is done, which is the point of it.

---
*Phase: 19-gitsafe-git-blast-radius-envelope*
*Completed: 2026-08-29*
