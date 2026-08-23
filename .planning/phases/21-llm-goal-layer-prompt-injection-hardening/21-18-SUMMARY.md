---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 18
subsystem: guard suite / driver argv seam / record honesty
tags: [gap-closure, round-6, wave-2, planted-defect-controls, record-correction]
requires: ["21-17"]
provides:
  - "guard nine seeing pub(-opened fields, str-family payloads, OsString deny-by-default, trailing comments"
  - "is_field_opener shared by the offender scan and the non-vacuity floor"
  - "names_token / names_string_payload; OSSTRING_ALLOWED suppress-only allowlist with type pins"
  - "one_of_each with an honest doc and pin message; the residual named"
  - "the acceptance matrix sweeping 7 argv positions x 4 padded payloads, no exemption"
  - "the DEGENERATE uniqueness scan covering tests/ as well as src/"
  - "the CR-01 tracer's envelope-half assertion"
affects:
  - src/driver/mod.rs
  - src/driver/run.rs
  - tests/spawn_seam_guard.rs
  - tests/driver_dry_run.rs
  - tests/driver_goal_seam.rs
tech-stack:
  added: []
  patterns:
    - "planted-defect control consuming the same extracted fn as the live assertion"
    - "deny-by-default with a suppress-only allowlist whose entries are pinned to their real declared types"
    - "residual named in the artifact rather than papered over with a fictitious mechanism"
    - "observe the resolved global root rather than inject one, when injection would race"
key-files:
  created: []
  modified:
    - src/driver/mod.rs
    - src/driver/run.rs
    - tests/spawn_seam_guard.rs
    - tests/driver_dry_run.rs
    - tests/driver_goal_seam.rs
key-decisions:
  - "D-18-1 one_of_each's false compile-error claim DELETED and the residual named"
  - "D-18-2 guard nine widened via the extracted fns, control-per-claim, red-first; OsString deny-by-default with a two-entry suppress allowlist"
  - "D-18-3 the padded acceptance matrix sweeps all 7 columns x 4 payloads"
  - "D-18-4 the uniqueness guard scans tests/ as well as src/"
  - "D-18-5 21-16's falsified truths corrected in THIS SUMMARY, not by editing 21-16"
  - "D-18-6 (new, this execution) the CR-01 envelope half is ASSERTED by reading the resolved root, not by injecting one"
requirements-completed: []
requirements: [DRIVE-04, SAFE-07]
actuals:
  tokens: 13721
  raw_tokens: 13721
  tasks: 3
  commits: 4
duration: "~35m across two executors (the first was killed by an API quota limit after task 2)"
completed: 2026-08-22
coverage:
  - deliverable: "Guard nine reports every spelling pass 6 measured it silent on"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#the_raw_argv_field_scanner_sees_every_measured_silent_spelling"
        status: pass
      - kind: test
        ref: "tests/spawn_seam_guard.rs#the_raw_argv_field_scanner_reports_a_planted_string_field"
        status: pass
      - kind: test
        ref: "tests/spawn_seam_guard.rs#drive_args_declares_no_raw_argv_string_field"
        status: pass
    human_judgment: false
  - deliverable: "Guard nine's limits block claims exactly what its controls measure"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#no_type_alias_hides_a_string_from_guard_nine"
        status: pass
      - kind: command
        ref: "the SEEN/SILENT/FLOOR mapping table in this SUMMARY's prohibition audit, one control per claim"
        status: pass
    human_judgment: false
  - deliverable: "one_of_each claims exactly what it delivers; the residual is named"
    verification:
      - kind: test
        ref: "src/driver/mod.rs#all_variant_names_matches_the_variant_set_in_both_directions"
        status: pass
    human_judgment: true
  - deliverable: "The acceptance matrix sweeps all 7 argv positions x 4 padded payloads"
    verification:
      - kind: test
        ref: "src/driver/mod.rs#one_visible_character_is_accepted_in_every_argv_position"
        status: pass
      - kind: command
        ref: "rtk proxy sh -c 'grep -c \"position.starts_with\" src/driver/mod.rs' -> 0"
        status: pass
    human_judgment: false
  - deliverable: "DEGENERATE is spelled in exactly one place TREE-WIDE"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#the_degenerate_payload_set_is_spelled_in_exactly_one_place"
        status: pass
      - kind: command
        ref: "rtk proxy sh -c 'grep -rn \"for blank in \\[\" tests/ | wc -l' -> 0; grep -rln DEGENERATE tests/ -> 3 files"
        status: pass
    human_judgment: false
  - deliverable: "The CR-01 tracer's envelope half is observed, not assumed"
    verification:
      - kind: test
        ref: "src/driver/run.rs#a_run_with_no_command_source_writes_nothing_before_refusing"
        status: pass
      - kind: command
        ref: "planted-ordering scratch probe: command_source moved below establish_envelope -> the new assertion FAILS; reverted, green"
        status: pass
    human_judgment: false
  - deliverable: "DRIVE-04 boundary and precision reconfirmed against this tree"
    verification:
      - kind: test
        ref: "tests/driver_escalation_cap.rs (whole binary)"
        status: pass
    human_judgment: false
  - deliverable: "SAFE-07 boundary and precision reconfirmed against this tree"
    verification:
      - kind: test
        ref: "tests/driver_injection_corpus.rs (12 passed / 0 failed / 10 ignored-by-design — see the qualification under truth 10)"
        status: partial
      - kind: test
        ref: "tests/spawn_seam_guard.rs (guard seven, inside the 35/0 run)"
        status: pass
    human_judgment: true
---

# Phase 21 Plan 18: Mechanisms That Say What They Do — Guard Nine's Blind Spellings, `one_of_each`'s Residual, and the Corrected Record

Pass 6 graded gap 2 — two anti-recurrence mechanisms constraining materially less than
their docs certified — as more important than the round's Critical, because those
artifacts are what a sixth reader trusts instead of re-deriving. This plan closes it by
making every mechanism state what it delivers and certifying every structural claim with
a planted-defect control that goes red without the fix.

## Execution provenance — read this before auditing the commits

This plan was executed by **two executors**. The first was killed by an API quota limit
after task 2 landed; the second (this one) audited the tree, executed task 3, and wrote
this SUMMARY. The audit result:

| Task | State found | Commits |
|---|---|---|
| Task 1 (TRACER) | **already committed** — red arm and fix both present, in that order | `9f9b6d2` (red), `704e95a` (fix) |
| Task 2 | **already committed** — all four sub-parts (a)-(d) present | `7cf4533` |
| Task 3 | **not started** — executed here | `899c6b4` + this SUMMARY commit |

Nothing from tasks 1-2 was re-done. Every acceptance criterion of tasks 1 and 2 was
**re-run against the final tree** by this executor and is quoted below, so their green is
this tree's green rather than an inherited claim. The first executor also left ~35
uncommitted lines in `src/driver/run.rs`; those were stashed by the orchestrator and
**were not applied** — task 3(a) was written from scratch, and it took a different and
stronger branch than that draft had (see D-18-6).

## Red-arm evidence (the tracer contract)

Task 1's red arm is committed at `9f9b6d2`, **before** the fix at `704e95a`. The
`#[ignore]`d control reproduced pass 6's WR-02 table exactly, against the unfixed scanner
and through the same extracted fns the live assertion consumes:

```
=== WR-02 reproduction against the UNFIXED scanner ===
  pub(crate) opener                  -> reported=0 []
  pub(super) opener                  -> reported=0 []
  Option<Box<str>>                   -> reported=0 []
  Option<Cow<'static, str>>          -> reported=0 []
  Option<&'static str>               -> reported=0 []
  Option<OsString>                   -> reported=0 []
  trailing // last field             -> reported=0 []
  trailing // mid-struct             -> reported=1 ["pub goal_file: Option<String>, // seventh pub dry_run: bool,"]
  Vec<OsString> (must NOT report)    -> reported=0 []
  Option<PathBuf> (must NOT report)  -> reported=0 []
  FLOOR PROBE: field_lines=12 protected=6 offenders=[]
```

and the control itself failed at the first plant:

```
assertion `left == right` failed: a `pub(crate)`-opened raw argv field must
be REPORTED. ... Got: []
  left: 0
 right: 1
```

Seven silent, one misattributed with the following declaration swallowed into its text
(so `dry_run` was never judged at all), and the floor reading exactly the
`field_lines=12 protected=6` pass 6 recorded while `offenders` was empty.

Task 3 carries no red-arm obligation of its own (it is not a tracer task), but its one
code change was nonetheless certified by a planted defect — see below.

### Task 3's planted-defect probe (scratch, never committed)

The CR-01 ordering regression was planted for real: `command_source` /
`iteration_source` moved below the `establish_envelope` block in `execute_run`. The new
assertion went red while the pre-existing project-half assertion stayed green — which is
precisely the direction that had been silent:

```
panicked at src/driver/run.rs:3842:13:
a refused run must have created NOTHING OUTSIDE the project either — no hook stubs, no
settings file, no generated gitconfig, no askpass responder. This directory existing
means the command-source resolution ran BELOW `establish_envelope` instead of above it,
which is the CR-01 ordering itself, and it writes into the developer's real data
directory rather than into a temporary one. Found:
  .../probe-envelope/cr01-envelope-probe/gh/
  .../probe-envelope/cr01-envelope-probe/askpass: #!/bin/sh ...
  .../probe-envelope/cr01-envelope-probe/gitconfig: ...
  .../probe-envelope/cr01-envelope-probe/settings.json: { "permissions": { "deny": [ ...
  .../probe-envelope/cr01-envelope-probe/hooks/pre-commit: ...
  .../probe-envelope/cr01-envelope-probe/hooks/pre-push: ...
```

Run under `GSD_MM_ENVELOPE_ROOT` pointed at a scratch directory, so the probe never
touched the real one. Reverted; tree green.

## Accomplishments

1. **Guard nine reads the type, and now sees the spellings the tree already uses.**
   One shared `is_field_opener` (`pub ` OR `pub(`) consumed by both the offender scan
   and the non-vacuity floor's filter — pass 6 measured those two *disagreeing*, floor
   counting twelve while the scan reported nothing with the offender in the body.
   `names_bare_string` generalised to `names_token`, with `names_string_payload`
   matching the `String` OR `str` tokens, covering `Box<str>`, `Cow<'_, str>`,
   `&'static str` and `&str` without firing inside `OsString`.
   `without_trailing_comment` strips a trailing `//` before the `,` test, fixing both
   measured comment positions, and a declaration still buffered at end-of-body is
   flushed and judged.

2. **`OsString` is deny-by-default over the whole scanned body**, suppressed only by
   NAME through the two-entry `OSSTRING_ALLOWED = ["claude_args", "claude_program"]`.
   Deny-by-default rather than a protected-name rule because the threat is a **seventh**
   argv field, which carries a new name by definition — a by-name rule binds nothing
   against it. The allowlist can only suppress, never widen, and the live assertion pins
   each entry to its real declared type (`claude_args: Vec<OsString>` at
   `src/driver/mod.rs:302`, `claude_program: Option<PathBuf>` at `:296`), so an entry
   cannot be silently repurposed for a payload field.

3. **Guard nine's limits block states only what its controls measure** — a SEEN list
   with one planted arm per bullet, a SILENT list with directions and per-item bounds
   (type alias: bounded by `no_type_alias_hides_a_string_from_guard_nine`; macro
   expansion: *bounded by nothing in this file*, stated rather than mitigated; the
   column-zero extraction: cross-referenced), and a floor sentence that says what the
   floors actually bound (extraction breakage and wholesale style drift) and what they
   provably do **not** (a single added field).

4. **`one_of_each` stopped claiming a compile error it does not cause** (D-18-1). The
   only compile-time anchor on `CommandSource`'s arity is `variant_name`'s wildcard-free
   match, and it forces classification, not sweep growth. The residual is named in the
   doc rather than replaced with a fictitious second mechanism.

5. **The acceptance matrix sweeps all seven argv positions with all four padded
   payloads.** The `--alias`/`--run-id` narrowing is gone, and so is its rationale, which
   pass 6 measured false (`from_argv` never calls `is_plain_path_component` for either
   field). The replacing comment is true only because 21-17 landed first.

6. **`DEGENERATE` is spelled once tree-wide.** The three hand-picked subsets consume
   `test_support::DEGENERATE`, and the uniqueness scan walks `tests/` as well as `src/`
   via `source_and_test_files`, which asserts both roots produced files.

7. **The CR-01 tracer observes both halves of "nothing".** New this execution — and it
   is an assertion, not a disclosure. See D-18-6.

## Record corrections

Prior artifacts are **not edited** (D-18-5). This is the new record round 7 inherits.

**1. 21-16 truth 1 was FALSE as shipped.** It claimed a field declared outside the
`pub <name>:` style is "bounded by the ≥ 10-field / ≥ 6-`NonBlank` non-vacuity floor and
the zero-alias grep."

- *Measured at pass 6:* with a `pub(crate) goal_file: Option<String>` offender planted
  in a full twelve-field body, the scan produced `field_lines=12 protected=6
  offenders=[]` — **the floor passed and the guard was silent.** The floor was not a
  bound on that gap at all; it was a bound on the scanner still recognising the tree's
  style.
- *True now:* the offender scan itself catches it, certified by
  `the_raw_argv_field_scanner_sees_every_measured_silent_spelling` (nine planted
  spellings plus that exact twelve-plus-one floor probe, red at `9f9b6d2`, green at
  `704e95a`), and the rewritten floor sentence in guard nine's header states that the
  floors bound extraction breakage and wholesale style drift and **not** a single added
  field, citing the measurement.

**2. 21-16 truth 4 was FALSE as shipped.** It claimed a fourth `CommandSource` variant
"is a compile error in TWO places (`variant_name`, `one_of_each`)".

- *Measured at pass 6:* a fourth variant was built with its single `variant_name` arm
  added; `all_variant_names_matches_the_variant_set_in_both_directions` **passed with the
  variant unswept, and there were zero compile errors.** An array literal of constructed
  values carries no exhaustiveness obligation.
- *True now:* the doc and the pin message name `variant_name`'s wildcard-free match as
  the **only** compile-time anchor, and say what it forces — classification, not sweep
  growth. The residual is named in the same paragraph: a fourth variant classified in
  `variant_name` and constructed nowhere passes this pin, and what catches it is the
  reviewer reading that sentence. Nothing mechanical does.

**3. 21-16-SUMMARY's prohibition-audit row 1 certified an unmeasured bound.** Its
"Guard nine (new), limits 1–3" row entered "Limit 2: the `>= 10` field-lines and `>= 6`
NonBlank floors inside the live assertion" as the *behaviour that bounds* the
declaration-style limit, and marked prohibition 1 PASS on that basis. That was a
certification of correction 1's falsehood, one layer up. This plan's audit below
certifies only claims a committed control measures, and the one claim with no mechanical
bound (macro-expanded declarations; and `one_of_each`'s residual) is entered as
**unbounded and named**, not as PASS.

## Prohibition audit

Every row is a **mechanism** run against this plan's own diff (`168de36..899c6b4`). No
row's check is a grep for a sentence of prose — which is itself prohibition 1's evidence.

| # | Prohibition | Mechanical check | Result |
|---|---|---|---|
| 1 | MUST NOT certify a guard/mechanism claim with a prose grep | Every structural claim in this plan maps to a named control below; the acceptance criteria that *are* greps (`position.starts_with` → 0, `for blank in [` → 0, `is_field_opener` ≥ 3) count **identifiers in code**, never sentences. Task 1's controls were committed RED (`9f9b6d2`) before the fix (`704e95a`); Task 2's uniqueness widening was probed with a scratch re-add; Task 3's assertion was probed with a planted ordering regression. | **PASS** |
| 2 | MUST NOT hand-copy any subset of the degenerate payload set, src/ OR tests/ | `rtk proxy sh -c 'grep -rn "for blank in \[" tests/ \| wc -l'` → **0**; `rtk proxy sh -c 'grep -rln "DEGENERATE" tests/'` → **3 files** (`spawn_seam_guard.rs`, `driver_goal_seam.rs`, `driver_dry_run.rs`); the standing check `the_degenerate_payload_set_is_spelled_in_exactly_one_place` now walks both roots and passes, and its scratch probe reported a re-added hand list at `tests/driver_dry_run.rs:609` by name. | **PASS** |
| 3 | MUST NOT leave a doc, limits block or truth claiming a bound no committed control measures | The claim-to-control table below. Two residuals are entered as **named, not certified**: macro-expanded declarations (bounded by nothing textual — the header says so) and `one_of_each`'s unswept-variant residual (bounded by a reviewer — the doc says so). | **PASS** |
| 4 | MUST NOT flip any `.planning/REQUIREMENTS.md` requirement | `rtk proxy git log --format=%H 168de36..HEAD -- .planning/REQUIREMENTS.md` → **no commits**. Files touched by this plan's four commits: `src/driver/mod.rs`, `src/driver/run.rs`, `tests/driver_dry_run.rs`, `tests/driver_goal_seam.rs`, `tests/spawn_seam_guard.rs` — REQUIREMENTS.md, STATE.md and ROADMAP.md appear in none. | **PASS** |

### Prohibition 3 in detail — each surviving claim beside the control that measures it

| Claim in a doc / limits block | Control that goes red without it | State |
|---|---|---|
| `pub(crate)` / `pub(super)` openers are SEEN | `the_raw_argv_field_scanner_sees_every_measured_silent_spelling`, arms 1-2 (both `reported=0` before the fix) | measured |
| str-family payloads (`Box<str>`, `Cow<'_, str>`, `&'static str`, `&str`) are SEEN | same test, arms 3-5 | measured |
| `OsString` is SEEN deny-by-default over the whole body | same test, `Option<OsString>` arm on the non-allowlisted name `goal_file` | measured |
| The allowlist does not over-report | same test, `Vec<OsString>` / `Option<PathBuf>` arms asserted NOT reported | measured |
| The allowlist cannot be silently repurposed | `drive_args_declares_no_raw_argv_string_field` pins `claude_args: Vec<OsString>` and `claude_program: Option<PathBuf>` against the **real** body | measured |
| Trailing `//` comments in both positions are SEEN, with correct attribution | same control test, last-field and mid-struct arms (the mid-struct arm asserts the following line is NOT swallowed and `dry_run` is not reported) | measured |
| The scan and the floor cannot disagree about what a field declaration is | one `is_field_opener` consumed by both — `grep -c "is_field_opener" tests/spawn_seam_guard.rs` → **6** (criterion: ≥ 3) | measured |
| The floors bound extraction breakage and style drift, NOT a single added field | the twelve-plus-one floor probe inside the same control (`field_lines=12 protected=6 -> PASSES` reproduced in-tree) | measured |
| A `type` alias hiding `String` is silent | `no_type_alias_hides_a_string_from_guard_nine` (asserts the count is zero on its own line) | measured |
| The extraction never reached `RawDriveArgs` | the extraction control asserts the region by name | measured |
| **A macro-expanded field declaration is silent** | **none in this file** — the header states this rather than mitigating it; what remains is `from_argv`'s no-`..` destructure, which a macro-declared field must still be named in | **named, not certified** |
| **A fourth `CommandSource` variant constructed nowhere passes the pin** | **none** — the doc names the residual and says the reviewer is what catches it | **named, not certified** |

## Named-shape audit — 21-18's share (rows 9-16, 19)

Rows 1-8 and the two disclosed deviations (17-18) are closed/disclosed by 21-17's
SUMMARY and are not re-verified here.

| # | Shape | Final state |
|---|---|---|
| 9 | guard nine silent spellings | **CLOSED.** Nine planted spellings + the floor probe, red at `9f9b6d2`, green at `704e95a`; `spawn_seam_guard` 35 passed / 0 failed against the final tree. |
| 10 | `one_of_each` false compile claim | **CLOSED BY HONESTY (D-18-1).** The false "two ways / TWO places" claim is deleted from the doc and from the pin's failure message; the residual is disclosed at `src/driver/mod.rs`'s `one_of_each` doc and in Record correction 2 above. `all_variant_names_matches_the_variant_set_in_both_directions` passes. No mechanism was substituted — that was the point. |
| 11 | acceptance-matrix hand-exemption | **CLOSED.** `rtk proxy sh -c 'grep -c "position.starts_with" src/driver/mod.rs'` → **0**; `one_visible_character_is_accepted_in_every_argv_position` sweeps 7 positions × 4 padded payloads and passes. |
| 12 | tests/ hand-copied DEGENERATE subsets | **CLOSED.** Three subsets converted; `for blank in [` → 0 tree-wide in `tests/`; the uniqueness guard now walks both roots and asserts both produced files; scratch probe reported a re-added list by file and line. |
| 13 | CR-01 tracer blind to the envelope half | **CLOSED BY ASSERTION, not by disclosure** (D-18-6) — the branch the plan treated as the less likely one. See the disposition below. |
| 14 | 21-16 truths 1/4 false in the record | **CORRECTED** — Record corrections 1 and 2 above, with the pass-6 measurement and the now-true statement for each. |
| 15 | Thirteen-fixture-file coincidental compile bound | **CLOSED.** Pass 6's finding was that a seventh field spelled `pub(crate) goal_file: Option<Box<str>>` would be destructured happily and left unprotected, and that what actually prevented it was the accident that thirteen integration crates build `DriveArgs { … }` literally. That reliance is retired: those two spellings are now exactly arms 1 and 3 of guard nine's control, so the payload-TYPE bound is asserted rather than coincidental. |
| 16 | 21-16-SUMMARY's unmeasured-bound certification | **CORRECTED** — Record correction 3 above; this plan's own audit enters two residuals as *named, not certified* rather than PASS. |
| 19 | IN-02/IN-03 guard-eight numbering and lost line continuations | **CLOSED** in `7cf4533(d)`: guard eight's limits renumbered 1-6 (was 1,6,2,3,4,5), the one cross-reference to its old "limit 3" updated, guard nine's floor message de-referenced from a stale limit number, and six assertion messages that had lost their `\` continuations (2 in `spawn_seam_guard.rs`, 4 in `driver_goal_seam.rs`) re-wrapped. |
| 20 | IN-04 `load_config` before `from_argv` | **CITED, not re-verified** — adjudicated pre-existing / out of scope per review G.4. No covering task in either plan of this round. |

## Disposition of pass-6's `run.rs:3788` warning

**The assertion branch was taken.** The plan offered two: assert the envelope root is
untouched *if a race-safe injected root exists*, else name the blind half with its
direction in the test's doc. Neither was needed as written, because a third route was
available and is strictly stronger:

- Every write `establish_envelope` performs outside the project lands under
  `envelope_dir(alias)` — `hooks::install` → `install_in` and `cred::build_env` →
  `build_env_in` both resolve `<root>/<alias>`. So **one** existence assertion on the
  alias's envelope directory covers hook stubs, `settings.json`, the generated
  `gitconfig`, the `gh` directory and the askpass responder.
- The root is **read, never set**: `envelope::envelope_dir` resolves exactly what the run
  would have resolved (`GSD_MM_ENVELOPE_ROOT`, else the platform data directory). No
  `std::env::set_var` was added to any parallel-executed test — the prohibition the plan
  set is honoured, and reading is safe because no test in the library binary sets that
  variable (`grep -rn "set_var" src/` → one *test name*, no call).
- The test's alias moved from `"demo"` to a dedicated `CR01_ENVELOPE_PROBE_ALIAS =
  "cr01-envelope-probe"`, so a real envelope in a developer's data directory can never
  make the assertion fire on an unrelated condition.

The direction is now **loud** rather than silent, and the planted-ordering probe above
demonstrates it. What remains unobserved is named in the test's doc with its direction:
`establish_envelope` also calls `write_exclude_block(project_root)`, but only for a
project root carrying a `.git`, and this fixture has none — unreachable here rather than
merely unchecked, and a third write site if the fixture ever grows a repository.

## Probe-authored reconfirmations (truths 8-11), run against this plan's final tree

`rtk proxy cargo test --test driver_escalation_cap --test driver_injection_corpus --test spawn_seam_guard -- --test-threads=2`

| Truth | Suite | Result |
|---|---|---|
| 8 — DRIVE-04 boundary (a cap at/above the resolved step cap refused, below accepted) | `driver_escalation_cap` | **8 passed / 0 failed** |
| 9 — DRIVE-04 precision (the decomposition consultation counted against the same cap; no exempt path) | `driver_escalation_cap`, unmodified by this plan | **8 passed / 0 failed** |
| 10 — SAFE-07 boundary (third-party text reaches a prompt only inside the labelled untrusted boundary; arrival asserted before influence) | `driver_injection_corpus` | **12 passed / 0 failed / 10 ignored** — see the qualification below |
| 11 — SAFE-07 precision (only the enumerated strings cross, never a whole file — guard seven) | `spawn_seam_guard` | **35 passed / 0 failed** |

**Qualification on truth 10, stated rather than glossed.** Ten of that binary's tests are
`#[ignore]`d by design ("spawns the real `claude` binary; run with `--ignored`"), and the
ignored set is exactly the eight `corpus_*_arrives_and_leaves_the_command_unchanged`
arms plus the two suppression controls. **The arrival-vs-influence comparison arms did
not execute in this run.** What did execute and pass are the corpus-integrity pins that
make those arms non-vacuous when they are run — `the_corpus_is_planted_where_the_shipped_reader_actually_reads`,
`the_typed_state_carries_no_marker_so_the_boundary_is_the_only_channel`,
`every_payload_survives_the_production_bound_whole`,
`the_corpus_prompt_carries_every_class_payload_and_asks_for_the_evidence_field`,
`every_named_class_has_exactly_one_ignored_arm_and_every_arm_names_a_class` and
`an_unavailable_binary_fails_loudly_instead_of_skipping`. 21-16's SAFE-07 truth carried
the same wording ("reconfirmed green in this plan's full-suite run") with the same
qualification unstated; it is stated here. Truth 10 is therefore **reconfirmed for the
structure, not for the live model round-trip**, and its coverage entry is marked
`partial` rather than `pass`.

## Files Created/Modified

| File | Change |
|---|---|
| `tests/spawn_seam_guard.rs` | guard nine's controls, `is_field_opener`, `names_token`/`names_string_payload`, `OSSTRING_ALLOWED` + its type pins, `without_trailing_comment`, the rewritten limits block, `source_and_test_files`, `DEGENERATE_WITNESS`, guard eight renumbering |
| `src/driver/mod.rs` | `one_of_each`'s honest doc and pin message; the 7×4 acceptance matrix with the exemption deleted and its comment replaced |
| `src/driver/run.rs` | the CR-01 tracer's envelope-half assertion, `CR01_ENVELOPE_PROBE_ALIAS`, the extended doc |
| `tests/driver_dry_run.rs` | two blank-payload lists consume `test_support::DEGENERATE`; the in-place disclosure retired |
| `tests/driver_goal_seam.rs` | the hostile blank-token list consumes `test_support::DEGENERATE`; four assertion messages re-wrapped |

No file outside this plan's `files_modified` list was touched. `Cargo.toml` unchanged; no
dependency moved.

## Task Commits

| Task | Commit | Subject |
|---|---|---|
| 1 (red) | `9f9b6d2` | `test(21-18): red arm — guard nine's blind spellings, planted and reproduced` |
| 1 (fix) | `704e95a` | `feat(21-18): guard nine sees every measured silent spelling (D-18-2)` |
| 2 | `7cf4533` | `feat(21-18): one_of_each says what it does; the matrix loses its exemption; DEGENERATE is unique tree-wide` |
| 3(a) | `899c6b4` | `test(21-18): the CR-01 tracer's envelope half, observed rather than named` |
| 3(c)(d)(e) | this commit | `docs(21-18): complete the honest-mechanisms and corrected-record plan` |

## Criterion-1 shape: unchanged

Nothing in this plan disturbed the shape pass 6 verified for the first time. Task 3
touched only a `#[cfg(test)]` module in `src/driver/run.rs`; tasks 1-2 touched
`from_argv`'s *acceptance matrix* and `one_of_each`'s *doc*, never the parse itself.
Confirmed by the whole guard suite (35/0) and the 1037-test library binary passing:
`from_argv`'s purity and ordering, the six `NonBlank` fields, the twelve-field no-`..`
destructure, and `execute_run`'s resolve-above-every-write are all intact — the last of
these is now *asserted* on both sides rather than on one.

## Decisions Made

D-18-1 through D-18-5 as planned. One new:

**D-18-6 — the CR-01 envelope half is asserted by READING the resolved root, not by
injecting one.** *Reversible.* The plan's two branches both assumed observation required
an injected root, and correctly forbade `set_var` in a parallel test. Reading
`envelope_dir(alias)` needs no injection, resolves exactly what the run would resolve,
and — with a probe-specific alias — cannot collide with anything real. It is strictly
stronger than the doc-only branch and carries no race.

## Deviations from Plan

**[Rule 2 — a better mechanism was available than either branch the plan offered]
Task 3(a) asserts instead of disclosing, without an injected root.**
Found during: Task 3(a). The plan's acceptance criterion reads "either asserts the
envelope root untouched (**with a race-safe injected root**) or its doc names the
unobserved half with direction." Neither literal branch was taken: the assertion landed
*without* injection. Reason: injection was the obstacle, not the goal, and reading the
same resolution the run performs removes the obstacle entirely. Both of the criterion's
real conditions hold — the envelope root is asserted untouched, and no `set_var` entered
a parallel test — and the doc *additionally* names the one genuinely unobserved path
(`write_exclude_block`). Recorded as D-18-6. Net effect: row 13 closes by assertion
rather than by disclosure, which is the stronger of the two outcomes the plan admitted.

**[Rule 1 — orchestrator ownership] The SUMMARY commit carries the SUMMARY alone.**
`execute-plan.md`'s `git_commit_metadata` step commits SUMMARY.md together with
STATE.md, ROADMAP.md and REQUIREMENTS.md. This plan's prohibition 4 forbids touching
REQUIREMENTS.md, and STATE.md / ROADMAP.md are owned by the orchestrator for this
gap-closure round. The metadata commit therefore names only
`.planning/phases/21-.../21-18-SUMMARY.md`.

**[Recorded, not a deviation] Truth 10's reconfirmation is structural.** See the
qualification under the probe table — the arrival-before-influence arms are
`#[ignore]`d by design and did not execute. Reported rather than counted as green.

## Issues Encountered

1. **A prior executor was killed by an API quota limit mid-plan**, after task 2. Its
   uncommitted draft of task 3(a) was stashed by the orchestrator and deliberately not
   applied; task 3(a) was rewritten from scratch and took a different branch. Nothing
   from tasks 1-2 was re-executed — their commits were audited and their acceptance
   criteria re-run.
2. **`rtk` filters counts.** Every count-bearing and presence-bearing check in this
   SUMMARY was run through `rtk proxy` (and through `rtk proxy sh -c '…'` where a
   pipeline was involved), because a rewritten `grep -c` has returned a false `0` three
   times in this phase.
3. **The documented flakes did not fire.** `driver_reattach`'s two tests and
   `envelope_tracer`'s relocated-stub test passed in both full-suite runs. The earlier
   `failed=2` observation is consistent with the recorded flake; this executor did not
   reproduce it and does not claim to have disproved it.

## Verification results

| Gate | Command | Result |
|---|---|---|
| Build | `rtk proxy cargo build --all-targets` | clean |
| Clippy | `rtk proxy cargo clippy --lib -- -D warnings` | clean |
| Whole suite (run 1) | `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2` | every binary `ok`, 0 failed |
| Whole suite (run 2) | same | **1346 passed / 0 failed** |
| Guard suite | `rtk proxy cargo test --test spawn_seam_guard` | 35 passed / 0 failed |
| Cap suite | `rtk proxy cargo test --test driver_escalation_cap` | 8 passed / 0 failed |
| Corpus suite | `rtk proxy cargo test --test driver_injection_corpus` | 12 passed / 0 failed / 10 ignored |
| CR-01 tracer | `rtk proxy cargo test --lib a_run_with_no_command_source` | 2 passed / 0 failed |

Test-count movement across the round: **1335 → 1346 passed, 0 failed throughout**
(baseline `0352dda` → this tree). The +11 are wave 1's and task 1-2's controls; task 3
added assertions to an existing test rather than a new one, so the count is unchanged
across `899c6b4`.

## Self-Check: PASSED

- Every task's `<acceptance_criteria>` re-run against the final tree, including tasks 1
  and 2, which this executor did not author.
- `git log --oneline --grep="21-18"` returns four commits; the red arm precedes its fix.
- Plan-level `<verification>` steps 1-6 all executed and recorded above.
- `key-files.created` is empty; every `modified` entry exists on disk.

## Next

Wave 2 is complete; both round-6 gap-closure plans have SUMMARYs. The phase's open
must-haves are the orchestrator's to reconcile — this plan flipped no requirement and
touched no STATE/ROADMAP/REQUIREMENTS file. Two residuals leave this round **named**
rather than closed, by design: a macro-expanded `DriveArgs` field is invisible to any
textual scan (bounded only by `from_argv`'s destructure), and a fourth `CommandSource`
variant constructed nowhere is caught by a reviewer and nothing else.
