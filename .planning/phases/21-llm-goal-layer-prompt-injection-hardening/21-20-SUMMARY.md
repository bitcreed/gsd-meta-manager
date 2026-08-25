---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 20
subsystem: testing
tags: [guard-scans, anti-recurrence, planted-defect-controls, self-scan, census, prompt-injection, record-correction]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "21-19's grown test_support::DEGENERATE (ten members, four outside the pre-round-7 ranges) — the two new uniqueness witnesses are drawn from it; and the derived invisible class whose own membership pins are two of the adjudicated witness sites"
provides:
  - "is_field_opener sees a field declared with NO visibility modifier — the scan and the non-vacuity floor share the fn, so they cannot disagree about it"
  - "judge_declaration judges payload types INSIDE the OsString branch, so an allowlisted name suppresses the OsString-presence report only"
  - "An OSSTRING_ALLOWED integrity pin that compares the parsed declared type for EQUALITY, not containment"
  - "degenerate_witnesses() — three witnesses from three different DEGENERATE members, with WITNESS_ALLOWED_ELSEWHERE adjudicating every legitimate non-home site by exact hit count"
  - "every_census_row_names_a_variant_that_still_exists (guard ten) with a permanent planted stale row"
  - "the_ignored_set_is_seven_arms_two_controls_and_their_own_meta_check — an ACTIVE census making SAFE-07's arithmetic a mechanism"
  - "is_comment_line / is_ignore_attribute_line — one self-scan mechanism in tests/driver_injection_corpus.rs where there were two"
  - "A dated append-only correction to 21-18-SUMMARY and a standing SAFE-07 deferred item with the exact human-run command"
affects: [phase-21 verification pass 8, any future guard header in this repository, the SAFE-07 human verification item]

actuals:
  tokens: 81198
  tasks: 3
  commits: 6

tech-stack:
  added: []
  patterns:
    - "A doc that names a bound must name the control that goes red without it; where no control can exist, name the residual WITH its failure direction"
    - "Widening a scan's witness set buys detection at the price of over-detection — pay it in the open with an adjudicated per-site table carrying exact counts, never with a blanket exemption"
    - "One self-scan mechanism per file: two independent scans of the same source drift apart about what a comment is"
    - "Count DECLARATIONS, never mentions: prose about a thing is not the thing"
    - "Correcting a shipped record is append-only and dated — the history stays legible"

key-files:
  created: []
  modified:
    - tests/spawn_seam_guard.rs
    - tests/driver_injection_corpus.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-18-SUMMARY.md
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md

key-decisions:
  - "D-20-1: is_field_opener widens to bare identifiers before `:`; over-detection is bounded by construction (the scan runs only inside the struct's braces) and is loud rather than silent"
  - "D-20-2: judge_declaration judges payload types inside the OsString branch; the integrity pin becomes equality on the parsed declared type text"
  - "D-20-3: corrections to 21-18-SUMMARY are append-only and dated — 51 insertions, 0 deletions"
  - "D-20-4: SAFE-07's boundary-never-executed fact becomes a standing deferred item; it does NOT close criterion 4 and says so in its own text"
  - "D-20-5 (executor): the three-witness scan carries WITNESS_ALLOWED_ELSEWHERE — two of the three witnesses have legitimate non-home sites after 21-19, and each is adjudicated with an EXACT hit count rather than exempted wholesale"

patterns-established:
  - "Planted-defect controls are committed RED against the unfixed scanner, with verbatim output in the doc comment AND the commit message, and stay permanently beside the live assertion"
  - "An exact-positive count is structurally non-vacuous where an emptiness assertion is not — a scanner that stopped matching reports zero and fails"

requirements-completed: [DRIVE-04, SAFE-07]

coverage:
  - id: D1
    description: "Guard nine sees a field declared with NO visibility modifier, in both the offender scan and the non-vacuity floor, and the widening reports nothing new against the real DriveArgs body"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "tests/spawn_seam_guard.rs#the_scanner_reports_a_bare_private_field"
        status: pass
      - kind: unit
        ref: "tests/spawn_seam_guard.rs#drive_args_declares_no_raw_argv_string_field (widened-vs-narrow opener control)"
        status: pass
      - kind: other
        ref: "red arm committed in 8ff3d99 before the fix in 0ede247; verbatim output quoted below"
        status: pass
    human_judgment: false
  - id: D2
    description: "An OSSTRING_ALLOWED entry can no longer be silently repurposed: a payload type riding beside an allowlisted OsString is reported, and the integrity pin certifies by equality rather than containment"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "tests/spawn_seam_guard.rs#an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring"
        status: pass
      - kind: other
        ref: "rtk proxy grep -n 'contains(expected_type' tests/spawn_seam_guard.rs -> one hit, in the comment recording what it USED to be; the pin itself is assert_eq!(actual_type, expected_type)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The DEGENERATE uniqueness scan draws three witnesses from three different members including one 21-19 added, adjudicates every legitimate non-home site by exact count, and its message claims only what it performs"
    verification:
      - kind: unit
        ref: "tests/spawn_seam_guard.rs#the_degenerate_payload_set_is_spelled_in_exactly_one_place"
        status: pass
      - kind: other
        ref: "the under-detection direction is named in the guard's own doc; quoted verbatim below"
        status: pass
    human_judgment: false
  - id: D4
    description: "Guard ten's census rows cannot go stale silently — each row's variant token must still be declared in src/cli.rs"
    verification:
      - kind: unit
        ref: "tests/spawn_seam_guard.rs#every_census_row_names_a_variant_that_still_exists"
        status: pass
      - kind: other
        ref: "red arm committed in 2b92588 before the fix in ba8d3d3; verbatim output quoted below"
        status: pass
    human_judgment: false
  - id: D5
    description: "SAFE-07's ignored-set arithmetic is a mechanism counting declarations, sharing one self-scan with the file's pre-existing scanner"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "tests/driver_injection_corpus.rs#the_ignored_set_is_seven_arms_two_controls_and_their_own_meta_check"
        status: pass
      - kind: unit
        ref: "tests/driver_injection_corpus.rs#every_named_class_has_exactly_one_ignored_arm_and_every_arm_names_a_class (expected values unchanged by the extraction)"
        status: pass
      - kind: other
        ref: "falsifiability demonstrations 1 and 2 below — containment predicate -> 14 vs 10; broken arm suffix -> 0 vs 7. Both reverted, neither committed."
        status: pass
    human_judgment: false
  - id: D6
    description: "SAFE-07's behavioural boundary is tracked where it is read every round, with the exact command and expected output, and is NOT claimed closed"
    requirement: "SAFE-07"
    verification: []
    human_judgment: true
    rationale: "The item's whole content is that the ten behavioural tests have never executed under any verification pass of this phase and cannot without an authenticated `claude` subscription. Only a human with that subscription can run `cargo test --test driver_injection_corpus -- --ignored --nocapture` and record the result. The mechanised half — the composition of the ignored set — is D5 and does run every round."
  - id: D7
    description: "The DRIVE-04 escalation-cap boundary and precision hold against this plan's final tree"
    requirement: "DRIVE-04"
    verification:
      - kind: integration
        ref: "cargo test --test driver_escalation_cap -> 8 passed, 0 failed"
        status: pass
    human_judgment: false
  - id: D8
    description: "The record is corrected where round 6 shipped falsehoods: seven falsified truths named FALSE AS SHIPPED with pass 7's measurement and the statement true after round 7"
    verification:
      - kind: manual_procedural
        ref: "the Record-corrections table below, cross-checked line by line against 21-VERIFICATION.md's must-have verdict table (rows 6, 7, 11, 13, 14, 15, 18)"
        status: pass
    human_judgment: false

duration: 26 min
completed: 2026-08-25
status: complete
---

# Phase 21 Plan 20: Guard Nine's Two Blind Spots, an Honest Uniqueness Scan, and the Ignored-Set Census Summary

**Four claims the anti-recurrence machinery was making without a control behind them became four committed controls — two guard-nine blind spots planted red and then fixed, a one-witness scan whose message promised tree-wide coverage narrowed to three witnesses and an adjudicated exemption table, a census row check that can no longer describe a variant nobody declares — and SAFE-07's arithmetic stopped being a SUMMARY sentence and became an active test.**

## Performance

- **Duration:** 26 min
- **Started:** 2026-08-25T18:40:10Z
- **Completed:** 2026-08-25T19:06:32Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- **Both of pass 7's guard-nine blind spots are closed, and each was committed RED first.** A field declared with no visibility modifier (`goal_hint: String,`) was skipped by the offender scan AND by the non-vacuity floor, because they share `is_field_opener` — reproducing pass 6's exact silent signature `field_lines=12 protected=6 offenders=[]` in a spelling that appeared in neither of guard nine's lists. And an allowlisted name suppressed the whole judgment rather than the `OsString`-presence report alone, so `pub claude_args: (Vec<OsString>, String),` was silent while *also* passing the `contains`-based integrity pin that exists to catch exactly that.
- **The uniqueness guard's message shrank to what its scan performs.** Three witnesses from three different `DEGENERATE` members, two of them from the ones 21-19 added, and the under-detection direction stated in words rather than implied. This is the fifth-consecutive-round defect the plan was written to end, and the correction is a **narrowing of the claim**, not a widening of the scan.
- **Guard ten's rows can no longer describe a tree that no longer exists.** `census_row_offence` asserts each row's variant token is still declared in `src/cli.rs`; the plant that proves it stays permanently beside the live assertion.
- **SAFE-07's arithmetic became a mechanism.** `21-18-SUMMARY` said eight class arms and omitted the tenth ignored test entirely. There are seven arms, and the tenth is the arms' own non-vacuity meta-check. An active census now asserts ten line-anchored `#[ignore]` attributes and seven arm declarations, sharing the file's ONE self-scan mechanism, so the next miscount has to make a test go red first.
- **The record is corrected where round 6 was wrong**, append-only and dated, and the one thing no test in this tree can establish — that SAFE-07's boundary has never executed — is now a standing item read every round rather than a sentence read once.

## Task Commits

1. **Task 1 (TRACER) RED arm: guard nine's two measured blind spots** — `8ff3d99` (test)
2. **Task 1 fix: make the scanner see, un-ignore both plants** — `0ede247` (fix)
3. **Task 2 (a)(b) + (c) RED arm: three-witness scan, guard ten's stale-row plant** — `2b92588` (test)
4. **Task 2 (c) fix: the variant-existence clause** — `ba8d3d3` (fix)
5. **Task 3: the census, the dated correction, the deferred item** — `4aec514` (feat)

**Plan metadata:** this SUMMARY (docs).

## The red arms, verbatim

Every one committed BEFORE its fix, `#[ignore]`d so the tree stayed green while the evidence landed in history, with the `#[ignore]` coming off in the fix commit. Each plant calls the SAME extracted fn the live assertion consumes.

### WR-01 — a field with no visibility modifier (`8ff3d99`)

```text
running 1 test
test the_scanner_reports_a_bare_private_field ... FAILED

---- the_scanner_reports_a_bare_private_field stdout ----

thread 'the_scanner_reports_a_bare_private_field' (664715) panicked at tests/spawn_seam_guard.rs:3315:5:
assertion `left == right` failed: a field declared with NO visibility modifier is still a field, and a `String` on it is still raw argv text. `is_field_opener` accepted only `pub `/`pub(`, so this spelling was skipped by the scan AND by the floor that shares the fn — pass 6's exact silent signature (`field_lines=12 protected=6 offenders=[]`) reproduced by a spelling in neither the SEEN nor the SILENT list. Got: []
  left: []
 right: ["goal_hint: String,"]

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 36 filtered out; finished in 0.00s
```

### WR-02 — an allowlist entry repurposed (`8ff3d99`)

```text
running 1 test
test an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring ... FAILED

---- an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring stdout ----

thread 'an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring' (665429) panicked at tests/spawn_seam_guard.rs:3386:5:
assertion `left == right` failed: an allowlisted NAME suppresses the `OsString`-presence report only. It must never suppress a raw payload TYPE riding beside it: `judge_declaration` returned from the OsString branch before `names_string_payload` ever ran, so this declaration was silent. Got: []
  left: []
 right: ["pub claude_args: (Vec<OsString>, String),"]

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 36 filtered out; finished in 0.00s
```

**The second half of WR-02, recorded because it is the sharper finding.** The integrity pin was `declaration.contains(expected_type)`, and `"pub claude_args: (Vec<OsString>, String),"` **contains** `"Vec<OsString>"` — so the pin written to catch a repurposed entry passed over the exact repurposing. It is now `assert_eq!(actual_type, expected_type)` over a parsed `declared_type_text`.

### Guard ten — a census row naming a variant nobody declares (`2b92588`)

```text
running 1 test
test every_census_row_names_a_variant_that_still_exists ... FAILED

---- every_census_row_names_a_variant_that_still_exists stdout ----

thread 'every_census_row_names_a_variant_that_still_exists' (720027) panicked at tests/spawn_seam_guard.rs:4276:5:
a census row naming a variant `src/cli.rs` does not declare must be REPORTED. Pass 7's warning was that the census bounds a COUNT and nothing else: rename `Scan` to `Sweep`, or delete one variant and add a different one, and the count stays at eight while the table describes a tree that no longer exists. The rows would go on naming judges for variants nobody can invoke. Got: None

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.03s
```

The red state is the extracted `census_row_offence` in its pre-round-7 behaviour: the fn exists and the control feeds it, so the control and the live assertion share one code path from the red commit onward.

## Pass 7's two guard reproductions, re-run against the FIXED scanner

Both are now the live behaviour of the committed tests, not a one-time probe:

| Pass 7 measurement | Against the fixed scanner |
|---|---|
| `goal_hint: String,` (bare) -> `offenders=[]`, `field_lines=12` | REPORTED as `["goal_hint: String,"]`; the floor's own filter now counts 13 on the same body — asserted in the same test, so scan and floor cannot diverge again |
| `pub claude_args: (Vec<OsString>, String),` -> silent, and the `contains` pin PASSES | REPORTED; and the pin is an equality on the parsed type, so the repurposed declaration fails it |

## Falsifiability demonstrations for the census (Task 3)

Performed once in this worktree, each reverted, **neither committed** — `git status` and `git diff --numstat` are both empty afterwards. Recorded because prohibition 1 asks for evidence rather than a claim, and because an exact-positive count is the only assertion shape in this plan that does not carry a planted control (a scanner that stopped matching reports zero and fails, which is why one is not needed).

**1 — replacing the line-anchored predicate with containment.** This is the defect the anchoring exists to prevent, and it is measurable:

```text
thread 'the_ignored_set_is_seven_arms_two_controls_and_their_own_meta_check' (754554) panicked at tests/driver_injection_corpus.rs:1409:5:
assertion `left == right` failed: this file must carry exactly TEN line-anchored `#[ignore]` attributes: ... Found 14.
  left: 14
 right: 10
```

**Recorded as measured, not as predicted.** The plan predicted a containment census would measure 18 against the raw `grep -c`. It measures **14**, because `is_comment_line` drops four of the eighteen mentions before the ignore check ever runs. The shared comment filter is doing real work; the raw `grep -c` figure is not the number a containment census would see.

**2 — breaking the arm suffix.** Proves the arm counter is not vacuous:

```text
assertion `left == right` failed: this file must declare exactly SEVEN corpus class arms; found 0.
  left: 0
 right: 7
```

## The two measurements side by side (Task 3 acceptance)

| Measurement | Command | Result |
|---|---|---|
| Raw substring count of the ignore attribute | `rtk proxy grep -c '#\[ignore' tests/driver_injection_corpus.rs` | **18** (it was **14** before this census's own prose landed) |
| Line-anchored count — what the census asserts | `rtk proxy grep -cE '^[[:space:]]*#\[ignore' tests/driver_injection_corpus.rs` | **10** |
| Corpus arm `fn` declarations | `rtk proxy grep -cE '^fn corpus_.*_arrives_and_leaves_the_command_unchanged\(\)' tests/driver_injection_corpus.rs` | **7** |

Adding four sentences of prose about the attribute moved the raw count from 14 to 18 and the anchored count by zero. That is the whole argument for anchoring, measured on this plan's own diff.

**Honest statement about the two independent grounds, as the criterion requires.** `fn`-declaration anchoring **alone would suffice** — the arm count is over declaration lines, so no mention in prose or in a string can inflate it. The runtime assembly of the arm suffix and the meta-check name from separate consts is a **second, independent** reason the census cannot match its own source, valuable if a later change moved any of these counts back onto containment. Neither is claimed to do the other's work.

## Record corrections

Seven truths round 6 shipped that pass 7 measured FALSE. A reader of round 8 inherits measurements, not certifications.

| Truth | As shipped | Pass 7's measurement | True after round 7 |
|---|---|---|---|
| **21-17 t1** — one production spelling of the invisible-character CLASS, shared by both judgments | Claimed delivered | **FALSE AS SHIPPED.** The *sharing* was real; the *class* was a 22-code-point subset of what its own doc named. "Sharing a wrong class perfectly is how one hand-enumeration reached five seams at once." | **True via 21-19.** `text::is_invisible_formatting_char` is DERIVED from `icu_properties` (`Cf` union `Default_Ignorable_Code_Point`); no literal range survives in the body; both judgments consume it. Falsified by an all-code-points sweep whose oracle is `unicode-properties`, an independent derivation, carrying a committed `format_seen >= 150` floor. |
| **21-17 t2** — `is_plain_path_component` refuses embedded invisible formatting; two identities that render identically cannot resolve to two paths | Claimed delivered | **FALSE AS SHIPPED.** Measured `is_plain_path_component("demo\u{202e}")` -> **true**; `envelope_dir_in` -> two roots; `run_paths` -> two directories. Pass 5's sentence, third pass running. | **True via 21-19.** Identities inverted to the finite allow-list `[A-Za-z0-9._-]` through one shared `text::is_identity_char`. Pinned by `an_identity_outside_the_alphabet_names_nothing_at_any_seam`. |
| **21-17 t6** — the phase token's look-alike safety is a PROPERTY, not a precondition | Claimed delivered | **FALSE AS SHIPPED.** `"2\u{200b}0"` refused, but `"2\u{202e}0"`, `"2\u{ad}0"`, `"2\u{e0041}0"`, `"2\u{fe0f}0"` all ACCEPTED — still relying on roadmap membership, which is a property of today's roadmap contents rather than of the value. | **True via 21-19.** The refusal is a property of the value, pinned against a fixture roadmap that DOES declare the visible twin, so `None` cannot be the staleness rule. |
| **21-17 t8** — backstop: no registry key, envelope root, credential scope, run directory or phase token differs from another only by invisible formatting | Claimed delivered | **FALSE AS SHIPPED.** Directly falsified end to end against the built binary: **nine** registry keys all rendering as `demo`, two envelope roots, two run directories, an accepted look-alike phase token. | **True via 21-19.** The binary-level nine-add reproduction now yields **ONE** key; the other eight refuse at `Alias::new`. |
| **21-18 t1** — guard nine reports every measured silent spelling; an allowlist entry cannot be silently repurposed | Claimed delivered | **FALSE AS SHIPPED (the allowlist half).** `judge_declaration`'s `return` fires before `names_string_payload`, and the integrity pin is a `contains`, so `claude_args: (Vec<OsString>, String)` is silent **and** passes the pin that exists to catch it. | **True via THIS plan, Task 1.** The payload judgment runs inside the OsString branch; the pin is an equality on the parsed declared type. Plant: `an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring`, committed red in `8ff3d99`. |
| **21-18 t2** — guard nine's limits block claims exactly what its committed controls measure | Claimed delivered | **FALSE AS SHIPPED.** `is_field_opener` accepted only `pub `/`pub(`, so a bare field was skipped by BOTH the scan and the floor — pass 6's exact signature in a spelling named in neither list, bounded only by thirteen fixture crates the SUMMARY itself reported retired. | **True via THIS plan, Task 1.** The opener sees the spelling; the SEEN list names it with its plant; and a RETIRED-RELIANCE paragraph states plainly that the thirteen-fixture bound was a coincidence of the fixture tree rather than a property, and is relied on in neither direction. Plant: `the_scanner_reports_a_bare_private_field` (`8ff3d99`). |
| **21-18 t5** — `DEGENERATE` spelled in exactly one place tree-wide and the guard's message finally true | Claimed delivered | **FALSE AS SHIPPED (the message half).** The consumption half was closed; the scan detected a hand copy only via ONE witness literal while the message claimed every blank-shape pin consumes the const. An overclaiming message, not a hole. | **True via THIS plan, Task 2.** Three witnesses from three different members; every legitimate non-home site adjudicated with an exact count in `WITNESS_ALLOWED_ELSEWHERE`; and the message no longer claims what it cannot check — the under-detection direction is stated in words. |

## The under-detection direction, quoted verbatim

Prohibition 2 requires the residual named WITH its failure direction wherever a scan cannot deliver a bound. From `the_degenerate_payload_set_is_spelled_in_exactly_one_place`'s doc:

> **Under-detection, silent, and this is the residual to know about.** A hand copy that carries only members OTHER than the three witnesses — say `["", "   ", "\t"]`, three real members of the set and none of them a witness — **is invisible to this scan and always will be.** Nothing in this file bounds it. Three witnesses make such a copy less likely than one did; they do not make it impossible, and the failure message no longer says otherwise.

And from guard ten's limits block, the new third approximation:

> What it does NOT catch: a variant REMOVED and RE-ADDED under the same name with a different meaning — the row still resolves, the judge named in it may no longer be the judge in the arm. **Under-detection, silent**, bounded only by the judge strings' own tests named in limit 2.

And guard nine's rewritten header, on the reliance that is now retired:

> That is a **coincidence of the fixture tree, not a property of the code** — delete or restructure those thirteen crates and the spelling compiles unprotected. [...] It is no longer relied on in either direction: `is_field_opener` now sees the spelling, and `the_scanner_reports_a_bare_private_field` is the committed control that goes red if it stops seeing it. Nothing in this header claims a bound beyond the arms named in it.

## Prohibition audit

Every row a mechanism check with its command and output. No prose greps.

| # | Prohibition | Command | Output / result |
|---|---|---|---|
| 1 | MUST NOT certify a guard claim with a grep for prose; every claim is certified by a planted-defect control committed red-first against the unfixed scanner, calling the SAME extracted fns | `rtk proxy git log --oneline 3f17818..HEAD` | Two `test(21-20)` commits precede their `fix(21-20)` commits: `8ff3d99` -> `0ede247`, `2b92588` -> `ba8d3d3`. Verbatim red output in all three plants' doc comments and in both commit messages. `rtk proxy grep -n "raw_string_argv_fields\|judge_declaration" tests/spawn_seam_guard.rs` shows the plants among the callers (`:3434`, `:3480`, `:3531`, `:3556`, `:3572`) — no re-implemented loop. The census, which has no plant, carries two reverted falsifiability demonstrations above instead, and its non-vacuity is structural (exact positive counts). |
| 2 | MUST NOT leave any guard doc, limits block or plan truth claiming a bound no committed control goes red for; residuals named WITH direction | Read of all three rewritten blocks, quoted verbatim in the section above | Guard nine's header: SEEN list names both new spellings with their arms; RETIRED-RELIANCE paragraph; "Nothing in this header claims a bound beyond the arms named in it." Uniqueness guard: under-detection direction in words; over-detection adjudicated per site. Guard ten: third approximation added, header now says "No claim beyond these three." |
| 3 | MUST NOT rewrite or delete any line of a prior SUMMARY; corrections append-only, dated, quoting the text they correct | `rtk proxy git diff --numstat 3f17818..HEAD -- .planning/.../21-18-SUMMARY.md` | `51  0` — **51 insertions, 0 deletions.** The section is headed `## Correction (round 7, 2026-08-25)` and blockquotes the eight-arms sentence verbatim before correcting it. |
| 4 | MUST NOT paste a raw invisible, bidi, tag or variation-selector character into any file; escapes only | Programmatic scan of all 4 changed files; class computed from `unicodedata` plus integer ranges, never spelled; the scanner's own source is pure ASCII | `files scanned: 4` / `raw invisible characters found: 0` |
| 5 | MUST NOT flip any REQUIREMENTS.md requirement | `rtk proxy git log --format=%H 3f17818..HEAD -- .planning/REQUIREMENTS.md` | *(empty)* — no commit of this plan touches the file. The three commits that historically did (`828d7cc`, `0c4f712`, `760d71f`) all predate this plan's base. |

**On `requirements-completed` in this SUMMARY's frontmatter.** The IDs are copied verbatim from the plan's `requirements` field, as the SUMMARY template requires. **This SUMMARY does not certify them** — requirement status is decided by a passed verification, `.planning/REQUIREMENTS.md` is untouched by every commit of this plan, and SAFE-07 in particular carries an open standing item (D6) saying its behavioural half has never executed.

## Named-shape audit

| # | Named shape (source) | Owner | Final state |
|---|---|---|---|
| 1 | Bare (no-`pub`) field silent to scan AND floor (pass-7 WR-01) | 21-20 T1 | **CLOSED.** `the_scanner_reports_a_bare_private_field` (`tests/spawn_seam_guard.rs`), committed red in `8ff3d99`, green in `0ede247`. Opener widened; floor shares the fn; limits block rewritten and the thirteen-fixture reliance named as retired. |
| 2 | Compound `OsString` declaration suppressed wholesale; `contains` pin passes (pass-7 WR-02) | 21-20 T1 | **CLOSED.** `an_allowlisted_name_cannot_carry_a_raw_payload_beside_its_osstring`, red in `8ff3d99`, green in `0ede247`. In-branch payload judgment plus `declared_type_text` equality pin. |
| 3 | One-witness `DEGENERATE` scan under an every-pin message (pass-7 WR-04) | 21-20 T2 | **CLOSED AS A CLAIM, DISCLOSED AS A SCAN.** `degenerate_witnesses()` returns three witnesses from three members; `the_degenerate_payload_set_is_spelled_in_exactly_one_place` iterates all three; `WITNESS_ALLOWED_ELSEWHERE` adjudicates each legitimate non-home site with an exact count; the under-detection residual is named with its direction rather than claimed away. |
| 4 | Guard-ten rows can describe removed/renamed variants at a stable count (pass-7 Warning) | 21-20 T2 | **CLOSED.** `every_census_row_names_a_variant_that_still_exists` + permanent planted `Commands::Adopt` row, red in `2b92588`, green in `ba8d3d3`. Third limits entry added with its own under-detection direction. |
| 5 | SAFE-07 qualification miscounts arms and omits the meta-check (pass-7 WR-05) | 21-20 T3 | **CLOSED.** `the_ignored_set_is_seven_arms_two_controls_and_their_own_meta_check` (ACTIVE, `tests/driver_injection_corpus.rs`); dated append-only correction at the end of `21-18-SUMMARY.md`. |
| 6 | SAFE-07 boundary never executed under verification (pass-7 behavior_unverified) | 21-20 T3 | **TRACKED, NOT CLOSED — and the item says so in its own text.** Standing round-7 item in `deferred-items.md` with the exact command, expected output, provenance, an empty run-history table, and the census named by identifier as the part that DOES run. Coverage entry D6 is `human_judgment: true`. |
| 7 | 21-17 truths 1/2/6/8 and 21-18 truths 1/2/5 false as shipped | 21-20 T3 | **CLOSED.** The Record-corrections table above, cross-checked row by row against `21-VERIFICATION.md`'s must-have verdict table. |
| 8 | The class subset and sampling defects themselves | 21-19 | **DELEGATED** — see `21-19-SUMMARY.md`'s audit. Not this plan's work; its output is this plan's base. |

## Probe reconfirmations (truths 8-11), run against this plan's final tree

`rtk proxy cargo test --test driver_escalation_cap --test driver_injection_corpus --test spawn_seam_guard -- --test-threads=2`

| Truth | Suite | Result |
|---|---|---|
| 8 — DRIVE-04 boundary (a cap at/above the resolved step cap refused, below accepted) | `driver_escalation_cap` | **8 passed / 0 failed** |
| 9 — DRIVE-04 precision (the decomposition consultation counted against the same cap; no exempt path) | `driver_escalation_cap`, unmodified by this plan | **8 passed / 0 failed** |
| 10 — SAFE-07 boundary (third-party text reaches a prompt only inside the labelled untrusted boundary; arrival asserted before influence) | `driver_injection_corpus` | **13 passed / 0 failed / 10 ignored** — the **structural** half only. The behavioural half is the standing deferred item and is stated as unverified, not certified. |
| 11 — SAFE-07 precision (only the enumerated strings cross, never a whole file — guard seven) | `spawn_seam_guard` | **38 passed / 0 failed** |

Truth 10's qualification is the one 21-18 got wrong and this plan corrects: **12 active pins became 13** (the census is the new one), and the ignored set is seven arms + two suppression controls + one meta-check, not eight arms + two controls.

## Files Created/Modified

- `tests/spawn_seam_guard.rs` (+680 / -74) — `is_field_opener` widened to bare declarations with its over-detection direction documented; `judge_declaration` judging payloads inside the OsString branch; `declared_type_text` + the equality integrity pin; the widened-vs-narrow opener control against the real body; the two guard-nine plants; guard nine's SEEN list and RETIRED-RELIANCE paragraph; `degenerate_witnesses()` + `WITNESS_ALLOWED_ELSEWHERE` + the rewritten uniqueness guard and its honest message; `census_row_offence` + `variant_is_declared` + `every_census_row_names_a_variant_that_still_exists`; guard ten's third limits entry.
- `tests/driver_injection_corpus.rs` (+173 / -6) — `is_comment_line` and `is_ignore_attribute_line` extracted from `assert_class_call_sites` (behaviour-identical) and consumed by both self-scans; `the_ignored_set_is_seven_arms_two_controls_and_their_own_meta_check` with its arm-name and meta-check-name halves.
- `.planning/phases/21-.../21-18-SUMMARY.md` (+51 / -0) — the dated append-only round-7 correction.
- `.planning/phases/21-.../deferred-items.md` (+71 / -0) — the round-7 standing SAFE-07 item.

## Decisions Made

The plan's D-20-1 through D-20-4 were implemented as specified. One further decision was forced by the tree as 21-19 left it:

**D-20-5 — the three-witness scan needs an adjudicated exemption table, and the honest way to pay for the widening is to name every site.** The plan directs the two new witnesses to be drawn from `"\u{200b}"` and `"\u{202e}"`. Unlike `"\n  \n"`, neither is distinctive: both are ordinary hostile fixtures that legitimately appear in the invisible class's own membership pins (`src/text.rs`) and in the look-alike suffix list 21-19 added (`src/journal/writer.rs`). A plain uniqueness assertion on either would have reported four legitimate sites as offenders. The plan anticipates this — it says the scan fails on a hit "outside `src/text.rs`'s additive list **and the allowed sites it names today**" — so `WITNESS_ALLOWED_ELSEWHERE` names each site with the reason it is not a hand-copied blank-shape list, and pins an **exact hit count** rather than exempting the file: an adjudicated site that grows a second member of the set, the first step of becoming the copy this guard exists to catch, breaks the census loudly. The comparison is `assert_eq!` on the whole per-path map, so a **stale** row (a file that stopped spelling the witness) fails too and cannot go on exempting a file for a reason that no longer holds.

Two smaller implementation choices, recorded rather than glossed:

- **`is_field_opener` is implemented with `chars()`, not a regex.** The plan specified the identifier pattern; this crate has no regex dependency and is not gaining one for a test scanner.
- **`census_row_offence` returns `Option<String>` rather than being a `bool` predicate**, so the row check the control shares with the live assertion carries its own diagnostic. The live assertion `panic!`s with it; the control asserts it is `Some`.

## Deviations from Plan

### Auto-fixed / adjusted

**1. [Rule 3 - Blocking] Task 2's witness set could not be plain-unique against the tree 21-19 left**

- **Found during:** Task 2(a)
- **Issue:** The plan's three named witnesses include two that occur legitimately outside `DEGENERATE_HOME` — `"\u{200b}"` at `src/journal/writer.rs:1096`, `src/text.rs:531` and `src/text.rs:663`; `"\u{202e}"` at `src/journal/writer.rs:1096` and `src/text.rs:618`. A scan asserting plain uniqueness would have failed on four legitimate sites.
- **Fix:** `WITNESS_ALLOWED_ELSEWHERE`, per D-20-5 above — which is the mechanism the plan's own wording ("the allowed sites it names today") calls for.
- **Verification:** `the_degenerate_payload_set_is_spelled_in_exactly_one_place` passes; the map comparison is exact in both directions, so it is non-vacuous by construction.
- **Committed in:** `2b92588`

**2. [Criterion wording] The integrity pin is `assert_eq!`, which the criterion's literal grep does not match**

- **Found during:** Task 1 acceptance
- **Issue:** Task 1's criterion asks for `rtk proxy grep -n "contains(expected_type\|== expected" tests/spawn_seam_guard.rs` to show "the equality form". The pin is written as `assert_eq!(actual_type, expected_type, …)`, which is the equality form with better diagnostics and no `== expected` text.
- **Fix:** Kept `assert_eq!` and satisfied the criterion's **intent** with two measurements instead: `rtk proxy grep -n "contains(expected_type" tests/spawn_seam_guard.rs` returns exactly one hit, at `:3666`, inside the comment recording what the pin USED to be — there is no `contains` on the declared type text in the pin itself; and `rtk proxy grep -n "actual_type, expected_type\|actual_type = declared_type_text"` returns `:3690` and `:3692`, the parse and the equality.
- **Committed in:** `0ede247`

**3. [Rule 1 - Measurement, not prediction] The census's containment demonstration measures 14, not the predicted 18**

- **Found during:** Task 3 falsifiability demonstration
- **Issue:** The plan reasons that a containment census would be "green over the wrong set" of 14 (the raw `grep -c` at the time). Run for real after the census's own prose landed, the raw `grep -c` reads 18 but the containment demonstration reports **14** — because `is_comment_line` drops four of the eighteen mentions before the ignore check runs.
- **Fix:** Recorded as measured. The census's doc and failure message were rewritten to say "strictly higher" rather than a hand-maintained delta, because a number that has to be updated by hand is the same failure mode this file's own consts avoid. The plan's *conclusion* is unaffected and if anything strengthened: 14 against 10 is still green over the wrong set.
- **Committed in:** `4aec514`

---

**Total deviations:** 3 (1 blocking-issue fix, 1 criterion-wording adaptation, 1 measured-vs-predicted correction).
**Impact on plan:** No scope creep. Deviation 1 is the mechanism the plan's own wording specifies; deviation 2 changes no behaviour; deviation 3 replaces a predicted number with a measured one, which is this phase's entire method.

## Issues Encountered

- **`rtk` output filtering was avoided throughout**, per the phase's blocking constraint. Every count-bearing, presence-bearing, grep-bearing and build/test-output check in this SUMMARY was run as `rtk proxy <cmd>`.
- **One self-inflicted edit error, caught immediately.** A `replace_all` un-ignoring both plants dropped the space in `fn ` and produced `fnthe_scanner_reports…`; fixed in two follow-up edits before any test run or commit. Nothing broken reached history.
- **`cargo clippy --all-targets -- -D warnings` is NOT clean**, and it was not clean before this plan either. Four pre-existing lints live in `src/project_creator.rs` (`bool_assert_comparison`, `cmp_owned`) inside its in-module tests. That file is outside this plan's `files_modified` and outside its scope boundary, so it was not touched. The plan's specified clippy gate — `cargo clippy --lib -- -D warnings` — is clean, and `cargo clippy --test spawn_seam_guard --test driver_injection_corpus -- -D warnings` is clean.

## Verification results

| Gate | Command | Result |
|---|---|---|
| Build | `rtk proxy cargo build --all-targets` | clean |
| Clippy (plan's gate) | `rtk proxy cargo clippy --lib -- -D warnings` | clean |
| Clippy (touched targets) | `rtk proxy cargo clippy --test spawn_seam_guard --test driver_injection_corpus -- -D warnings` | clean |
| Guard suite | `rtk proxy cargo test --test spawn_seam_guard` | **38 passed / 0 failed / 0 ignored** (was 35 + 2 ignored at base) |
| Corpus suite | `rtk proxy cargo test --test driver_injection_corpus` | **13 passed / 0 failed / 10 ignored** (was 12 + 10) |
| Escalation cap | `rtk proxy cargo test --test driver_escalation_cap` | **8 passed / 0 failed** |
| Whole suite | `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2` | **35 binaries, 1362 passed, 0 failed, 13 ignored** |

Baseline at the worktree base `3f17818` was 35 binaries / 1358 passed / 0 failed / 13 ignored. The four new passes are the two guard-nine plants, the guard-ten stale-row control and the corpus census. **The ignored count is unchanged at 13** — every `#[ignore]` this plan added came off in the same task's fix commit, and the ten availability-gated `driver_injection_corpus` tests are untouched. **No flake was encountered in any run**; the three documented flaky tests passed on every execution.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Pass-7 gap 3 is closed at the level the plan specifies: every claim these three guards make about what they report is now backed by a control that goes red without it, and the two places where no scan can deliver a bound (a hand copy avoiding all three witnesses; a variant re-added under the same name) are named in the code with their failure directions rather than certified away.
- **This plan's five task commits touch no `.planning/REQUIREMENTS.md`.** The two `.planning/` files it does write are a prior SUMMARY's append-only correction and the deferred-items register. `STATE.md` and `ROADMAP.md` are the orchestrator's to write and are untouched.
- **Still open, deliberately, and not to be read as closed:** SAFE-07's behavioural boundary has never executed under any verification pass of this phase. It is now a standing item in `deferred-items.md` with the exact command and an empty run-history table. It requires a human with an authenticated `claude` subscription; nothing in this tree can close it.
- **For the verifier**, the three claims worth re-deriving independently: (a) that both guard-nine plants call `raw_string_argv_fields`/`judge_declaration` rather than re-implementing the scan (readable from `git show 8ff3d99`); (b) that `WITNESS_ALLOWED_ELSEWHERE`'s four rows each describe a purpose-specific fixture list rather than a hand-copied blank-shape subset — this is an adjudication, and it is the one place this plan asks to be trusted about a judgement rather than a measurement; and (c) that the census counts declarations, which is checkable by adding a sentence of prose about an arm and observing the count not move.

## Self-Check: PASSED

**Files claimed, verified present on disk:** `tests/spawn_seam_guard.rs`, `tests/driver_injection_corpus.rs`, `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-18-SUMMARY.md`, `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md` — all found. `key-files.created` is empty.

**Commits claimed, verified in `git log 3f17818..HEAD`:** `8ff3d99`, `0ede247`, `2b92588`, `ba8d3d3`, `4aec514` — all five present, in the stated order, with each red arm preceding its fix.

**Acceptance criteria re-run:** all three tasks' criteria pass against the final tree, with the two wording adaptations recorded as deviations 2 and 3 rather than silently skipped. Plan-level `<verification>` steps 1-5 all executed and recorded above. `git log --format=%H 3f17818..HEAD -- .planning/REQUIREMENTS.md` returns empty. `git diff --numstat` for `21-18-SUMMARY.md` reads `51 0`. `git status --short` is clean of the two reverted falsifiability demonstrations.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-25*
