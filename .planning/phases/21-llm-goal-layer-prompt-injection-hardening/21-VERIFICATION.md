---
phase: 21-llm-goal-layer-prompt-injection-hardening
verified: 2026-08-25T19:36:27Z
head: 8dc8c98
status: human_needed
score: 19/20 must-haves verified (4/5 ROADMAP success criteria; 0 FAILED, 1 behavior-unverified)
behavior_unverified: 1
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 11/20 must-haves verified (3/5 ROADMAP success criteria)
  gaps_closed:
    - "pass-7 gap 1 (criterion 1 — `carries_visible_content` returning true for a single U+202E / U+00AD / U+E0041 / U+FE0F, so `from_argv` accepted an argv payload with no visible instruction): CLOSED, and closed by DERIVATION rather than by a seventh enumeration. `is_invisible_formatting_char` (`src/text.rs:144-153`) is now `GENERAL_CATEGORY.get(c) == GeneralCategory::Format || DEFAULT_IGNORABLE.contains(c)` over `icu_properties` compiled data, with no literal range in the function body — I read it. **Measured with an oracle independent of the implementation AND of the tree's own test oracle** (see Judgment 1 for provenance): all 170 `General_Category=Cf` code points that CPython's `unicodedata` 15.0.0 names are refused by BOTH `carries_visible_content` and `carries_invisible_formatting`; the accepted set is EMPTY in both directions. Pass 7's own eight witnesses (U+202E, U+00AD, U+034F, U+E0041, U+FE0F, U+13430, U+180E, U+FFF9) are each refused in all four argv string positions with the correctly typed error, and `run_paths` returns `None` for every one."
    - "pass-7 gap 2 (the identity class — nine registry keys rendering as `demo`, two envelope roots, two run directories, an accepted look-alike phase token, `gsd-\\u{202e}nur` printing as `gsd-run`): CLOSED by INVERTING the direction, which is the only move in this phase's seven rounds that terminates the regress rather than moving it. `text::is_identity_char` (`src/text.rs:204-206`) is `[A-Za-z0-9._-]`, and it is a clause of `journal::is_plain_path_component` (`:339`) and of `registry::Alias::new` (`:145`). REPRODUCED IN REVERSE end to end against `target/debug/gsd-meta-manager` built from HEAD: sixteen `add` invocations with one scratch config and one fixture project; **exactly one exit 0** (`demo`), fifteen exit 1 — including the seven pass 7 registered, plus U+2800, the PUA, a Cyrillic homoglyph (`dem\\u{43e}`), and the two non-ASCII aliases the recorded trade refuses (`d\\u{e9}mo`, CJK). `config.json` afterwards holds ONE key. Pass 7's nine-keys-rendering-as-`demo` reproduction is gone."
    - "pass-7 gap 2, the `LOOK_ALIKE_PAIRS` tautology (round-3 WR-03 at its third level): CLOSED by moving the SAMPLING, which pass 7 correctly identified as the only fix that could work. `every_format_character_the_standard_names_is_inside_the_class` (`src/text.rs:377-406`) sweeps all 0x110000 code points with `unicode-properties` (unicode-rs) as its oracle, against a production class that reads `icu_properties` (ICU4X) and nothing else — two independently maintained derivations, neither reading the other. It ships a committed `format_seen >= 150` non-vacuity floor against a measured 170, and `Cargo.toml` names the `general-category` feature explicitly with a comment saying why it must never be defaulted. Both fixture consts also had their docs CORRECTED to disclaim the anti-tautology property they never had (`src/test_support.rs:23-43`, `:83-95`) rather than being quietly grown — the honest move."
    - "pass-7 gap 2's missing item 3 (the two seams carrying no look-alike pin of their own): CLOSED. `--target-phase` has `src/driver/mod.rs:1769` consuming `LOOK_ALIKE_PAIRS` with a DISCRIMINATING negative arm (`!matches!(err, TargetPhaseInvalid)` at `:1811`), and `journal::writer`'s re-read has `an_active_pointer_naming_a_look_alike_run_is_refused_not_followed` (`:1095-1130`), which sweeps four suffixes including two from outside the pre-round-7 ranges and asserts BOTH directions with the visible twin's directory really on disk so a `None` cannot be the stale-pointer rule firing instead. I read both."
    - "pass-7 coincidental-reliance item (the `--target-phase` token safe only by roadmap CONTENTS): CLOSED, and this is now a property of the value. Measured by me: `is_plain_path_component` returns false for `\"2\\u{202e}0\"`, `\"2\\u{2800}0\"` and `\"2\\u{43e}0\"` — the last a Cyrillic homoglyph, which no deny-list over invisible characters could ever have closed. 21-17 truth 6's contract is finally true."
    - "pass-7 gap 3, WR-01 (guard nine blind to a field declared with NO visibility modifier, in both the scan and the floor): CLOSED. `is_field_opener` (`tests/spawn_seam_guard.rs:3069-3087`) now accepts a bare identifier before a `:`. **I did not take this from the SUMMARY or from the control: I planted the defect myself.** Inserting `pass8_bare_plant: String,` into the real `DriveArgs` body took `drive_args_declares_no_raw_argv_string_field` from green to RED. Reverted; `git status` clean. Pass 7's `offenders=[]` silent pass does not reproduce."
    - "pass-7 gap 3, WR-02 (the `OsString` early return and the `contains` integrity pin): CLOSED, verified by reading the control flow rather than the doc. `judge_declaration` (`:2894-2912`) no longer returns from the `OsString` branch — the comment at `:2906-2907` says the payload judgment still runs, and the code does. `declared_type_text` (`:2922-2929`) was extracted so the live pin (`:3690-3702`) compares for EQUALITY; `declared_type_text(\"pub claude_args: (Vec<OsString>, String),\")` yields `\"(Vec<OsString>, String)\"`, which is not equal to `\"Vec<OsString>\"`, so the pin now goes red on exactly pass 7's example. The planted control (`:3515-3559`) consumes `raw_string_argv_fields`, the same fn the live assertion consumes, and carries its red output verbatim at `:3500-3513`. (I could not plant this one in the real body — the compound type does not typecheck against `claude_args`' consumers — and I say so rather than claiming a measurement I did not make.)"
    - "pass-7 gap 3, WR-04 (`DEGENERATE` uniqueness scanned via ONE witness literal): CLOSED. `degenerate_witnesses()` (`:3804-3811`) returns THREE, assembled pairwise from two half-arrays, and the scan (`:3948-3970`) runs a per-path census compared BOTH ways against an adjudicated table — an extra path is an unadjudicated copy, a missing path is a stale exemption. A per-witness `home_hits == 1` assertion carries its own non-vacuity floor."
    - "pass-7 gap 3, the guard-ten Warning (no row-variant existence check): CLOSED. `census_row_offence` backs a permanent stale-row plant (`:4294-4340`) shared with the live assertion, so renaming a variant while holding the count at 8 goes red."
    - "pass-7 gap 3, WR-05 (the SAFE-07 arithmetic): CLOSED as a MECHANISM, not prose. `the_ignored_set_is_seven_arms_two_controls_and_their_own_meta_check` (`tests/driver_injection_corpus.rs:1374`) is ACTIVE, counts `fn` DECLARATIONS with the prefix and suffix assembled at runtime so it cannot count its own source, and shares `is_comment_line`/`is_ignore_attribute_line` with the file's pre-existing self-scan so the two cannot disagree. I confirmed its arithmetic independently from my own suite run: `driver_injection_corpus` = 13 passed / 10 ignored, and the ten enumerate as seven `corpus_*` arms, two suppression controls, and `both_arms_of_every_class_comparison_were_really_executed`."
    - "pass-7's binary-level Trojan Source reproduction in `list`: CLOSED at the render seam. `display_identity` (`src/text.rs:259-269`) escapes every invisible-class character to `U+XXXX`, and `list` consumes it (`src/main.rs:146`), as do the TUI project table (`src/ui/project_list.rs:156`) and `judged_alias_or_exit`'s refusal echo (`src/main.rs:43`). MEASURED against the binary with a hand-built legacy `config.json`: the row prints `gsd-U+202Enur`, not `gsd-run`. A legacy `d\\u{e9}mo` still prints as itself, so this is a legibility defence and not a transliteration, exactly as documented."
    - "The ACCEPTING direction did NOT regress, which is the half a refuse-everything fix would have broken. Measured by me at every identity seam simultaneously (`is_plain_path_component`, `Alias::new`, `run_paths`, argv `--run-id`, argv `--target-phase`): `\"20\"`, `\"2.1\"`, `\"99\"`, `\"demo\"`, `\"gsd-meta-manager\"`, `\"my_project.v2\"`, `\"2026-08-19T12-00-00Z-aaaa\"` and `\"2026-07-28T14-03-11Z-a3f9\"` all pass at all five. Free text remains UNJUDGED by the alphabet: `--goal` and `--command` accept `\"/gsd:progress\"` (a `:` outside the alphabet), `\"réparer le café\"`, CJK, Cyrillic and an embedded ZWJ beside visible content."
    - "The legacy recovery route is REAL, not asserted. Measured end to end: four legacy aliases an older build accepted (`gsd-\\u{202e}nur`, `d\\u{e9}mo`, `demo\\u{200b}`, CJK) are all LISTED by `list` and all four `remove` with exit 0, leaving only the clean key. The product trade is therefore reversible in the way `src/text.rs:185-197` claims it is."
    - "Process: `.planning/REQUIREMENTS.md` untouched for the FIFTH consecutive round. `git log -- .planning/REQUIREMENTS.md` still ends at `0c4f712`; all five phase-21 requirements read `[ ]` (lines 62, 63, 83, 85, 86) and `Gaps Found` (152, 153, 164, 166, 167). The prohibition held again."
    - "Process: the record-correction practice pass 7 called the best thing round 6 did was CONTINUED and widened. `21-18-SUMMARY.md:525-570` carries a dated, append-only round-7 correction of its own eight-arms miscount, and `deferred-items.md:129-181` now carries SAFE-07's unexecuted boundary as a STANDING item with the exact command, the expected output, and the ten tests enumerated — moved out of a SUMMARY qualification that is read once and into a file that is read every round. This is pass 7's recommendation 4, implemented."
  gaps_remaining: []
  regressions: []
deferred:
  - truth: "General Unicode CONFUSABLES / homoglyph defence in FREE TEXT (a Cyrillic `а` beside a Latin `a` in a `--goal`)"
    addressed_in: "Not phase 21 — recommend a new roadmap item"
    evidence: "The carve-out at `src/text.rs:219-229` is unchanged and still correct, and it now states honestly where the harm IS closed and where it is not: at identity seams homoglyphs are closed by the finite alphabet (I measured `dem\\u{43e}` and `2\\u{43e}0` both refused), and in free text they remain open by design. TR39 skeleton/confusables is a separate data table with a separate false-positive profile. Round 7 did not let the class fix grow into it, which was prohibition 5 of plan 21-19."
  - truth: "`registry::current_prompt_inputs` absent from `tests/async_blocking_guard.rs`'s BLOCKING_HELPERS"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md, unchanged across rounds 4-7)"
    evidence: "Async-hygiene class, not this phase's class; the fix forces production `spawn_blocking` rewiring in `approve_plan` and `execute_run`."
  - truth: "The spawn-gate plan-half argument lives in a comment rather than a checked property"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md)"
    evidence: "The comment states plainly that the plan half is a no-op and why that is sound."
  - truth: "Dead `PlanStep::rationale` field in src/driver/goal.rs"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md)"
    evidence: "No production reader; cosmetic dead field with no security or honesty bearing."
  - truth: "`tests/driver_reattach.rs` and `tests/envelope_tracer.rs` flake under parallel execution"
    addressed_in: "Next phase backlog (deferred-items.md)"
    evidence: "Neither fired in my own full-workspace run (1362 passed / 0 failed / 13 ignored)."
  - truth: "The degenerate matrix's ROW table stays hand-maintained (`positions()`)"
    addressed_in: "Disclosed residual, not a gap (deferred-items.md 'OUT — by design')"
    evidence: "21-15 truth 6 discloses this in those words; the disclosure is still accurate."
behavior_unverified_items:
  - truth: "A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes (ROADMAP success criterion 4 / SAFE-07)"
    test: "With an authenticated `claude` CLI available, run `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the repository root and record the CLI version beside the result."
    expected: "10 passed, 0 failed. Every `corpus_*_arrives_and_leaves_the_command_unchanged` arm asserts the payload ARRIVED at the model before asserting the command was unchanged; the two suppression controls show the positive/negative `CLAUDE_CODE_DISABLE_CLAUDE_MDS` pair diverging; and `both_arms_of_every_class_comparison_were_really_executed` confirms the hostile and clean arms both really ran."
    why_human: "All ten spawn the real `claude` binary and need an authenticated subscription, so they cannot run inside verification. NO AGENT CAN CLOSE THIS ITEM. I confirmed the ignored set myself from my own suite run — `driver_injection_corpus` reports 13 passed / 10 ignored, and the ten enumerate as seven class arms, two suppression controls and the arms' own non-vacuity meta-check. The thirteen ACTIVE structural pins prove the corpus is planted where the shipped reader reads, that every payload survives the production bound whole, and that the typed state carries no marker — i.e. that the labelled boundary is the only channel. They cannot prove the model's behaviour ON that channel. The last recorded live run remains `21-05-SUMMARY.md:514` (10 passed against `claude` 2.1.238), an executor claim from twenty-eight commits ago that no verification pass of this phase has ever reproduced. Round 7 did the only thing an agent could do about it: it made the arithmetic a red-capable mechanism and moved the standing fact into `deferred-items.md:129-181`. Presence and wiring verified; behaviour not."
coincidental_reliance_items: []
human_verification:
  - test: "With an authenticated `claude` CLI available, run `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the repository root and record the CLI version beside the result."
    expected: "10 passed, 0 failed, with arrival asserted before influence in every class arm and `both_arms_of_every_class_comparison_were_really_executed` green."
    why_human: "Requires an authenticated subscription and spawns the real model binary; cannot run inside verification. This is criterion 4's only behavioural evidence and no verification pass of this phase has ever produced it."
---

# Phase 21: LLM Goal Layer & Prompt-Injection Hardening Verification Report

**Phase Goal:** A user states a goal once and the run pursues it, with the model confined to two narrow, bounded, hardened seams
**Verified:** 2026-08-25T19:36:27Z
**HEAD:** `8dc8c98`
**Status:** human_needed
**Re-verification:** Yes — eighth verification pass, after the seventh gap-closure cycle

## Goal Achievement

**Pass 6 declared criterion 1 verified and was wrong, because it attacked the
character class with U+200B and U+FEFF — two code points drawn from INSIDE the
implementation's own list. Pass 7 retracted it. My single most important
obligation this pass was not to repeat that, so I will state my provenance
before I state my verdict.**

### Judgment 1 — Where my criterion-1 witnesses came from, and why they are independent

I used two sources, neither of which appears anywhere in this repository.

**Source 1 — CPython's `unicodedata` module, `unidata_version` 15.0.0.** This is
a third, independently maintained derivation of the Unicode Character Database.
It is not `icu_properties` (ICU4X), which the production class at
`src/text.rs:144-153` reads. It is not `unicode-properties` (unicode-rs), which
the tree's own sweep at `src/text.rs:377-406` reads as its oracle. Nothing in
`Cargo.toml`, `src/`, `tests/` or the round-7 plans mentions Python. I enumerated
every code point in `range(0x110000)` whose `unicodedata.category()` is `Cf` —
**170 of them** — emitted them as a Rust `const`, and fed each one, as a
one-character string, through the production predicates from a scratch
integration crate. This is a cross-oracle completeness check: it catches a
derivation bug, a wrong accessor, or a stale pinned Unicode version.

**Source 2 — code points chosen for "renders as nothing" that are OUTSIDE the
class the implementation defines.** Source 1 shares a *definition* with the
implementation (`Cf`), so on its own it could only catch a derivation bug, not a
scope bug. So I derived a second set from the opposite direction: characters I
know render as blank or as no glyph, whose category I then confirmed with
CPython, and which are neither `Cf` nor default-ignorable — **U+2800 BRAILLE
PATTERN BLANK (`So`), U+E000 (Private Use Area, `Co`), U+0378 (unassigned,
`Cn`), U+0301 COMBINING ACUTE ACCENT (`Mn`), U+FFFC (`So`)** — plus controls
that ARE inside the class (U+1160, U+3164, U+1D173) so a uniform answer could
not be mistaken for a working predicate. None of these appears in `DEGENERATE`,
in `LOOK_ALIKE_PAIRS`, in any plan's cited examples, or in pass 7's twenty
values. For the identity half I additionally used a **Cyrillic homoglyph**
(U+043E), which is by construction outside any invisible-character deny-list —
the one shape that can distinguish a deny-list fix from an allow-list fix.

Everything below was measured against the tree at `8dc8c98`. Scratch integration
crates and a scratch registry were created, run and deleted; `git status`
afterwards shows only the pre-existing untracked `.gsd/` and
`.planning/milestone.lock`, plus `21-REVIEW.md` which the concurrently-running
code reviewer is writing and I did not touch. Every count, presence and grep
check ran under `rtk proxy`.

### Judgment 2 — Does criterion 1 hold now? Yes, and this time it was attacked from outside.

**The class is derived, and it is complete against a third oracle.** All 170
`Cf` code points CPython names are refused by `carries_visible_content` AND by
`carries_invisible_formatting`. The accepted list is **empty in both
directions**. Pass 7's eight measured witnesses each produce the correctly typed
refusal in all four argv string positions, and `run_paths` returns `None` for
every one.

**The regress is terminated for identities, not merely moved down another
level.** This is the finding that matters most, and it is why pass 8 differs in
kind from passes 2 through 7. Every one of my out-of-class witnesses — Braille
blank, PUA, unassigned, combining mark, Cyrillic homoglyph — is refused at every
identity seam:

```
                       carries_visible_content   is_plain_path_component   Alias::new
U+2800 (So, blank)              true                     false             OutsideIdentityAlphabet
U+E000 (Co, PUA)                true                     false             OutsideIdentityAlphabet
U+0378 (Cn, unassigned)         true                     false             OutsideIdentityAlphabet
U+0301 (Mn, combining)          true                     false             OutsideIdentityAlphabet
U+043E (Cyrillic homoglyph)      —                       false             OutsideIdentityAlphabet
```

Note the shape of that table. The deny-list column (`carries_visible_content`)
says *true* — it is one code point short, as a deny-list over a growing standard
always can be. The allow-list column says *false* anyway. **For the first time
in this phase, a witness the deny-list misses causes no harm**, because the
identity judgment does not consult a deny-list. `[A-Za-z0-9._-]` cannot be one
item short; the accepted set is finite and printable.

**Reproduced in reverse against the binary.** Sixteen `add` invocations against
`target/debug/gsd-meta-manager` built from HEAD, one scratch config, one fixture
project: **one exit 0**, fifteen exit 1. `config.json` afterwards holds one key.
Pass 7's nine-keys-all-rendering-as-`demo` is gone, and so is the
`gsd-\u{202e}nur` spoof — `list` now prints `gsd-U+202Enur`.

**The residual, stated plainly rather than absorbed.** `--goal` and `--command`
are FREE TEXT, judged by the deny-list because an ASCII allow-list there would
refuse legitimate script — a trade `src/text.rs:178-183` argues for and I agree
with. So a `--goal` of one U+2800, U+E000, U+0378 or U+0301 is still accepted
and can land verbatim in `run.json`'s `goal` field, where it renders as blank.
**I weighed calling criterion 1 FAILED on that and decided against it, and here
is the reasoning so a ninth reader can overturn it if they disagree.** Pass 7's
defect was that the function's own doc named a WIDER class than the code
implemented — an undisclosed subset, with a reproduced end-to-end harm (run
directories, registry keys, envelope roots, terminal spoofing). At HEAD the doc
and the code name the same class, that class is complete against an independent
oracle, and none of my out-of-class witnesses reaches any identity, any
directory, any registry key or any envelope root. What remains is a
display-honesty residual confined to a string the user typed themselves. That is
a different and much smaller thing, and calling it a criterion failure would be
demanding a complete solution to a problem the code correctly documents as
having none (`src/text.rs:199-203`). **It is recorded as a Warning below, and
its one real defect is that the doc does not disclose it** — this phase's own
standard is that a residual is named with its direction, and this one is not.

### Judgment 3 — Is the six-round enumeration pattern broken, or moved?

**Broken for identities. Moved — honestly, and with the move disclosed — for
free text.**

The pattern pass 7 named was: each round closed a level structurally and
hand-enumerated the next level down. Arms → fields → character class → and every
round wrote its verifying fixture out of the same head that wrote the
implementation.

Round 7 attacked both halves of that, and only one of the two attacks is a
structural guarantee:

1. **The SAMPLING moved, which pass 7 said was the only item that addresses the
   real recurrence.** The falsifying property no longer lives in a literal
   fixture. It lives in an exhaustive sweep whose oracle is a different crate
   from the production derivation, with a committed non-vacuity floor and an
   explicitly pinned feature flag so it cannot vanish silently. My third oracle
   agrees with both. Both fixture consts had their false anti-tautology claims
   DELETED rather than quietly grown — the doc correction is the part I trust
   most, because it costs the round something.
2. **The DIRECTION inverted for identities, which is the structural half.** An
   allow-list cannot be one item short. My homoglyph witness proves this is not
   a rhetorical claim: no widening of any invisible-character deny-list, however
   derived, would ever have refused `dem\u{43e}`. The alphabet refuses it in the
   same clause that refuses bidi, tags and variation selectors.

**What is still an enumeration, and where it now sits.** Free text keeps the
deny-list, so its completeness is bounded by `icu_properties`' pinned Unicode
version — disclosed at `src/text.rs:131-140` with the staleness obligation and
the release-process refresh named. And one half of the class has no second
machine oracle at all: `named_default_ignorable_members_beyond_cf_are_inside_the_class`
(`src/text.rs:419-442`) is thirteen hand-named members, because the
`unicode-properties` dev-dependency does not expose `Default_Ignorable`. The doc
says so in those words (`:408-417`): *"a subset bug in a default-ignorable code
point NOT listed here is invisible to every test in this tree."* That is a real
residual, correctly disclosed, and it is the last hand-enumeration standing. It
cannot reach an identity — identities are version-independent — so what it
bounds is free-text emptiness only.

**The honest bottom, which round 7 recorded rather than re-derived**
(`src/text.rs:199-203`): the move with zero enumeration left is to stop letting
user bytes BE an identity at all. Round 7 declined it deliberately and said so.
I agree with the decline and with the disclosure.

### Observable Truths

#### ROADMAP success criteria (the contract)

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs | ✓ **VERIFIED** | **Attacked from outside the implementation, per Judgment 1.** All 170 CPython-derived `Cf` code points refused in both judgments (accepted set empty). Pass 7's eight witnesses refused in all four argv positions with correctly typed errors; `run_paths` → `None` for each. Five out-of-class witnesses (U+2800, U+E000, U+0378, U+0301, U+043E) refused at every identity seam despite the deny-list judging four of them visible — the allow-list holds where the deny-list is short. Structural machinery re-read and intact: twelve-field no-`..` destructure (`driver/mod.rs:414-429`), six `NonBlank` fields, `from_argv` pure and above `drive`, record builders total and verbatim (`run.rs:797-816`, `:910-915`). Acceptance did not regress (Probe D, 8 values × 5 seams). Residual: free-text `--goal`/`--command` accept out-of-class blank-rendering characters — Warning, not a gap. |
| 2 | An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input | ✓ VERIFIED (reconfirmed) | `recheck_approval` at `run.rs:2410`, above `iteration_source` (:2447), `establish_own_group` (:2449), `establish_envelope` (:2493) and `JournalRun::start` (:2626) — re-read, ordering unchanged. `driver_goal_seam` 22 passed / 0 failed in my own full-workspace run; decomposition → plan-digest → approval → recheck-at-spawn intact. |
| 3 | Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing | ✓ VERIFIED (reconfirmed) | `driver_escalation_cap` **8 passed / 0 failed** in my own run — both directions (a cap at or above the *resolved* step cap refused at the seam; a cap below it accepted) plus the typed park reason on the journal. Untouched by the round-7 diff. |
| 4 | A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes | ⚠️ **PRESENT_BEHAVIOR_UNVERIFIED** | Counted independently from my own suite run: `driver_injection_corpus` = **13 passed / 10 ignored**, the ten being seven `corpus_*` class arms, two suppression controls, and `both_arms_of_every_class_comparison_were_really_executed`. All ten spawn the real `claude` binary and need an authenticated subscription. **No agent can close this**; I am not manufacturing a pass. Round 7 did what was available: the arithmetic is now an ACTIVE red-capable census (`driver_injection_corpus.rs:1374`) sharing its scanners with the file's pre-existing self-scan, and the standing fact moved into `deferred-items.md:129-181`. Present and wired; behaviour unexercised. → human verification. |
| 5 | Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string | ✓ **VERIFIED** (pass 7's coincidental-reliance flag CLEARED) | `parse_action` (`goal.rs:343-350`) returns `Result<RouterAction, UnknownCommand>` — an enum lookup, never a string. `driver_refusal_record` 9/9 and `driver_model_seam` 4/4 green in my full run; no shell string is constructed from model output (`escalate.rs:45`, `:153`; `run.rs:2873`). **The flag pass 7 raised is gone**: 21-17 truth 6's contract is now met as a property. Measured by me — `is_plain_path_component("2\u{202e}0")`, `("2\u{2800}0")` and `("2\u{43e}0")` all **false**, so the model-supplied phase token is refused by its own value rather than by what today's roadmap happens to contain. |

**ROADMAP score: 4/5 verified, 0 FAILED, 1 behavior-unverified.**

#### Round-7 plan must-have truths (what the round contracted to deliver)

| # | Truth (source) | Status | Evidence |
|---|---|---|---|
| 6 | The invisible class is DERIVED, not enumerated; both judgments consume it (21-19 t1) | ✓ VERIFIED | `src/text.rs:144-153` — two `const fn` borrowed statics and one boolean expression; no literal range in the body. Both predicates call it (`:107`, `:235`). Confirmed complete against my third oracle. |
| 7 | The falsifying corpus no longer shares a derivation source with the implementation (21-19 t2) | ✓ VERIFIED | Sweep at `:377-406` over all 0x110000 code points with `unicode-properties` as oracle against an `icu_properties` implementation; `format_seen >= 150` floor against a measured 170; `general-category` feature pinned explicitly in `Cargo.toml` with the reason. Both fixture docs corrected to DISCLAIM the property they never had (`test_support.rs:23-43`, `:83-95`). |
| 8 | Identity is judged by an allow-list that cannot be one code point short (21-19 t3) | ✓ VERIFIED | `is_identity_char` (`:204-206`), one spelling, consumed at `journal/mod.rs:339` and `registry.rs:145`. Verified against a homoglyph, which no deny-list could close. Sixteen-alias binary reproduction: 1 accepted, 15 refused. |
| 9 | The two pin-free seams carry their own look-alike pins (21-19 t4) | ✓ VERIFIED | `driver/mod.rs:1769` with a discriminating `!matches!(TargetPhaseInvalid)` negative arm at `:1811`; `journal/writer.rs:1095-1130` sweeping four suffixes, both directions, with the visible twin's directory really on disk. Read both. |
| 10 | The tool's identity displays can no longer be reordered by what they render (21-19 t5) | ✓ VERIFIED | The truth names exactly three surfaces — `list`, the refusal echo path, the TUI list — and all three consume `display_identity` (`main.rs:146`, `main.rs:43`, `ui/project_list.rs:156`). Measured at the binary: `gsd-U+202Enur`. (A FOURTH surface the truth does not claim is unescaped — Warning, below.) |
| 11 | Acceptance did not narrow where the tree depends on it (21-19 t6) | ✓ VERIFIED | Probe D: 8 legitimate values × 5 seams, all accept. Probe E: free text accepts `/gsd:progress`, French, CJK, Cyrillic and an embedded ZWJ. Over-detection bounded past ASCII by ten named script members (`text.rs:469-488`). |
| 12 | Guard nine sees a field declared with NO visibility modifier (21-20 t1) | ✓ VERIFIED | **Planted by me in the real body; the guard went RED.** `is_field_opener` widened at `:3069-3087`. The limits block at `:2800-2805` names the thirteen-fixture struct-literal bound as retired and a coincidence. (Diagnosis-message Warning, below.) |
| 13 | An allowlist entry can no longer be silently repurposed; the pin is an EQUALITY (21-20 t2) | ✓ VERIFIED | `judge_declaration` `:2894-2912` — no early return, payload judgment runs inside the branch. `declared_type_text` `:2922-2929` feeds an `assert_eq!` at `:3690-3702`. Control at `:3515-3559` consumes the same `raw_string_argv_fields`, with its red output committed verbatim at `:3500-3513`. |
| 14 | The `DEGENERATE` uniqueness guard scans ≥2 witnesses and names its under-detection direction (21-20 t3) | ✓ VERIFIED | `degenerate_witnesses()` `:3804-3811` returns three; the scan `:3948-3970` is a both-ways per-path census with a per-witness `home_hits == 1` non-vacuity floor and an explicit stale-exemption message. |
| 15 | Guard ten's census rows cannot go stale silently (21-20 t4) | ✓ VERIFIED | `census_row_offence` shared between the live assertion and a permanent stale-row plant (`:4294-4340`); count tripwire retained at `:4278`. |
| 16 | The SAFE-07 arithmetic is a mechanism counting DECLARATIONS, not prose (21-20 t5) | ✓ VERIFIED | `the_ignored_set_is_seven_arms_two_controls_and_their_own_meta_check` (`driver_injection_corpus.rs:1374`) is ACTIVE, assembles its prefix/suffix at runtime so it cannot count its own source, and shares `is_comment_line`/`is_ignore_attribute_line` with the file's pre-existing self-scan. I independently confirmed 7 / 2 / 1 from my own suite run. |
| 17 | The unexecutable half is tracked where it is READ, not stated once (21-20 t6) | ✓ VERIFIED | `deferred-items.md:129-181` — a STANDING item with the ten tests enumerated, the exact command, the expected output, and an instruction to re-surface it every round. `21-18-SUMMARY.md:525-570` carries the dated append-only correction; no prior line was rewritten. |
| 18 | The record is corrected where round 6 shipped falsehoods (21-20 t7) | ✓ VERIFIED | `21-18-SUMMARY.md:262` — "Prior artifacts are **not edited**" — plus the append-only round-7 correction block. The practice pass 7 called the best thing round 6 did is now two rounds old and widened. |
| 19 | DRIVE-04 boundary and precision reconfirmed (21-20 t8, t9) | ✓ VERIFIED | `driver_escalation_cap` 8/8 in my own run, both cap directions, decomposition consultation counted against the same cap. |
| 20 | SAFE-07 boundary and precision — structural half only (21-20 t10, t11) | ⚠️ **PRESENT_BEHAVIOR_UNVERIFIED** | The thirteen active structural pins pass and guard seven is green inside my own `spawn_seam_guard` run (38/38). The behavioural half is truth 4 above. The plan states this as unverified rather than certified, which is correct. |

**Score: 19/20 must-haves verified; 1 present-but-behavior-unverified; 0 failed.**

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/text.rs` (`is_invisible_formatting_char`) | The one derived spelling of the invisible class | ✓ VERIFIED | `:144-153`, two const borrowed statics and one boolean. No literal range. Complete against CPython `unicodedata` 15.0.0 for all 170 `Cf` code points. |
| `src/text.rs` (`is_identity_char`) | The finite alphabet, one spelling | ✓ VERIFIED | `:204-206`. Consumed at `journal/mod.rs:339` and `registry.rs:145`; `registry.rs:559`/`:845` use it for alias derivation. Carries the recorded product trade and the reversibility statement at `:185-197`. |
| `src/text.rs` (`carries_visible_content`) | The one emptiness judgment | ✓ VERIFIED | `:104-108`. Refuses every pass-7 witness. Free-text residual for out-of-class blank-rendering characters — Warning. |
| `src/text.rs` (`carries_invisible_formatting`) | The identity deny-list, second over the same class | ✓ VERIFIED | `:234-236`. Now backstopped by the alphabet at every seam, so its extent is no longer load-bearing for identity harm. |
| `src/text.rs` (`display_identity`) | Render-side defence for legacy rows | ✓ VERIFIED | `:259-269`. Measured at the binary: `gsd-U+202Enur`, and `d\u{e9}mo` passes through as itself. |
| `src/text.rs` (independent sweep) | A fixture shape that can go red on a SUBSET | ✓ VERIFIED | `:377-406`, all code points, second-crate oracle, committed non-vacuity floor, feature pinned with its reason. Agrees with my third oracle. |
| `src/text.rs` (default-ignorable half) | The half with no second oracle | ⚠️ **DISCLOSED HAND LIST** | `:419-442`, thirteen named members. `:408-417` states the residual in exactly the right words. Correctly disclosed; cannot reach an identity. |
| `src/test_support.rs` (`DEGENERATE`, `LOOK_ALIKE_PAIRS`) | Named seam fixtures, docs no longer overclaiming | ✓ VERIFIED | 10 members / 6 pairs, four and three of them from outside the pre-round-7 ranges. Both docs now say plainly that literal spelling is not independence. |
| `src/journal/mod.rs` (`is_plain_path_component`) | Blank + control + identity + ALPHABET + structural | ✓ VERIFIED | `:309-349`, five clauses. Refuses every witness I could construct, including a homoglyph. Redundancy of clauses 1-3 is stated honestly at `:295-303` with the reason they are kept. |
| `src/registry.rs` (`Alias`, `add_project`) | Registration closed at the entry, alphabet clause included | ✓ VERIFIED | `Alias::new` `:119-160`, five delegating clauses, none judging locally except the documented whitespace usability rule. Sixteen-alias binary reproduction: 1 in, 15 out. |
| `src/main.rs` / `src/ui/project_list.rs` | Escaped identity rendering | ⚠️ **THREE OF FOUR SURFACES** | `list` (`:146`), refusal echo (`:43`) and the TUI table (`project_list.rs:156`) escape. `Removed project '{}'` (`:128`) and two `eprintln!("Error: {refusal}")` sites (`:91`, `:359`) do not — Warnings. |
| `src/driver/mod.rs` (`DriveArgs`, `from_argv`) | Six `NonBlank` fields, twelve-field no-`..` destructure, pure | ✓ VERIFIED | Re-read `:314-341` and `:410-460`. Purity and position above `drive` unchanged. |
| `tests/spawn_seam_guard.rs` (guard nine) | Bare declarations seen; allowlist not repurposable | ✓ VERIFIED | Bare-field blindness closed — **verified by planting the defect and observing red**. `OsString` early return deleted; integrity pin is an equality on parsed type text. |
| `tests/spawn_seam_guard.rs` (guard ten, DEGENERATE scan) | Stale rows caught; ≥2 witnesses | ✓ VERIFIED | `census_row_offence` plant; three witnesses with a both-ways per-path census. |
| `tests/driver_injection_corpus.rs` (ignored-set census) | An ACTIVE red-capable arithmetic | ✓ VERIFIED | `:1374`, declaration-counting, runtime-assembled needles, shared scanners. Arithmetic independently confirmed from my suite run. |
| `.planning/.../deferred-items.md` | SAFE-07's unexecuted boundary tracked where it is read | ✓ VERIFIED | `:129-181`, standing item, ten tests enumerated, command and expected output given. |
| `.planning/REQUIREMENTS.md` | Accurate, untouched by the round | ✓ VERIFIED | Last touch `0c4f712`; all five phase-21 entries read `[ ]` / `Gaps Found`. Fifth round holding. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `text::is_invisible_formatting_char` | `icu_properties` compiled data | `GeneralCategory::Format` ∪ `DefaultIgnorableCodePoint` | ✓ WIRED | Read the body; no literal range. Verified complete against a THIRD oracle. |
| `text.rs` test sweep | `unicode-properties` dev-dependency | Exhaustive all-codepoint implication with a non-vacuity floor | ✓ WIRED | Two crates, neither reading the other. My CPython oracle agrees with both. |
| `journal::is_plain_path_component` | `text::is_identity_char` | The allow-list clause at `:339` | ✓ WIRED | The link that closes the seven-round regress. Verified with a homoglyph. |
| `registry::Alias::new` | `text::is_identity_char` | Clause 4 at `:145`, above the path-component clause so the message carries the trade | ✓ WIRED | Measured: `OutsideIdentityAlphabet` with the recovery route in the message. |
| `payload::NonBlank::new` | `text::carries_visible_content` | Delegation, not re-implementation | ✓ WIRED | The link that carried the subset in pass 7; the subset is gone. |
| `main.rs list` / TUI / refusal echo | `text::display_identity` | Render-side escaping of the invisible class | ✓ WIRED | Three surfaces measured or read. |
| `main.rs Remove` / `Add` refusal | `text::display_identity` | — | ✗ **NOT WIRED** | `:128`, `:91`, `:359` print raw. Measured `Removed project 'gsd-<RLO>nur'`. Warning. |
| `driver/mod.rs:877` (`--target-phase`) | `journal::is_plain_path_component` | The model-selected identity seam | ✓ WIRED | Refuses `2\u{202e}0`, `2\u{2800}0`, `2\u{43e}0`. |
| `driver/mod.rs:1087` / `journal/writer.rs:491` | `journal::is_plain_path_component` | The run-id seams, argv and re-read | ✓ WIRED | Both pinned, both directions, with outside-class members. |
| guard nine's scan | every `DriveArgs` field declaration | `is_field_opener` token match, widened | ✓ WIRED | Planted and measured red. |
| `OSSTRING_ALLOWED` integrity pin | the declared type | `assert_eq!` on `declared_type_text` | ✓ WIRED | Equality, not containment. |
| `driver_injection_corpus` arms | the real model boundary | Live `claude` spawn | ⚠️ **IGNORED** | All ten, including the arms' own non-vacuity meta-check. Unchanged and unclosable by an agent. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `config.json` | registry key (alias) | `Alias::new` | ✓ | ✓ **FLOWING** — 16 alias variants, 1 accepted, measured against the binary |
| `<envelope>/<alias>/` | envelope root | `envelope_dir_in` → `is_plain_path_component` → alphabet | ✓ | ✓ **FLOWING** — no look-alike or out-of-class alias resolves |
| `run.json` / run directory | `run_id` | `NonBlank` + `is_plain_path_component` + alphabet | ✓ | ✓ **FLOWING** — `run_paths` `None` for every witness; real timestamped ids still resolve |
| `run.json` | `target_phase` | `Option<NonBlank>` + `is_plain_path_component` + alphabet | ✓ | ✓ **FLOWING** — now safe by the VALUE, not by roadmap contents |
| `runs/active` (re-read) | `run_id` | `read_active_run` → `is_plain_path_component` | ✓ | ✓ **FLOWING** — look-alike pointer refused while the visible twin resolves |
| `run.json` | `gsd_command` | `recorded_command(&IterationSource)` | ⚠️ | ⚠️ **AMBIGUOUS (narrowed)** — cannot be `""` and cannot be any `Cf`/default-ignorable value; a lone U+2800 / PUA / unassigned still passes |
| `run.json` | `goal` | `args.goal.as_ref().map(NonBlank::as_str)` | ⚠️ | ⚠️ **AMBIGUOUS (narrowed)** — same class; free text by design |
| `list` / TUI ALIAS column | alias | `display_identity` on the key | ✓ | ✓ **FLOWING** — `gsd-U+202Enur`, spoof made visible |
| `Removed project '…'` | alias | raw `Display` | ✗ | ✗ **SPOOFABLE** — measured; the one identity display the round did not escape |

### Behavioral Spot-Checks

Every count-, presence- or grep-bearing check ran under `rtk proxy`, including
all `cargo` output. The full workspace suite was run exactly once and its output
saved for grepping.

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full workspace suite (run once) | `cargo test --workspace --no-fail-fast -- --test-threads=2` | **1362 passed, 0 failed, 13 ignored** | ✓ PASS |
| Gate lint | `cargo clippy -- -D warnings` | exit 0 | ✓ PASS |
| Unfiltered severity | `cargo clippy --all-targets` | 4 warnings, all pre-existing (`browser.rs:131-133`, `project_creator.rs:146`) | ✓ PASS (info) |
| **PROBE A — cross-oracle sweep** | 170 CPython-derived `Cf` code points × both judgments | **accepted set EMPTY in both directions** | ✓ **VERIFIES the derivation** |
| **PROBE B — definition-scope attack** | 8 witnesses (5 outside `Cf ∪ DI`, 3 inside as controls) × 4 argv positions + 2 predicates | Inside-class controls refused everywhere. Outside-class: `carries_visible_content` **true**, `from_argv` Ok — but `is_plain_path_component` **false** in every case | ⚠️ **residual is free-text-only** |
| **PROBE C — identity seams** | 8 hostile identities incl. a Cyrillic homoglyph × 4 seams | `is_plain_path_component` false, `Alias::new` Err, `run_paths` None — **all 8, all seams** | ✓ **VERIFIES the allow-list** |
| **PROBE D — acceptance** | 8 legitimate values × 5 seams (`"20"`, `"2.1"`, `"99"`, `"demo"`, two real timestamped run ids, …) | 40/40 accept | ✓ **NO REGRESSION** |
| **PROBE E — free text unjudged** | 6 free-text values × `--goal` and `--command` | 12/12 accept, incl. `/gsd:progress`, CJK, Cyrillic, embedded ZWJ | ✓ **NO REGRESSION** |
| **Binary-level registration** | 16 × `gsd-meta-manager --config <scratch> add <fixture> "<alias>"` | **1 exit 0 (`demo`), 15 exit 1.** `config.json` → ONE key | ✓ **pass-7 gap 2 CLOSED** |
| **Trojan Source at the render seam** | legacy `config.json` + `list \| cat -v` | prints `gsd-U+202Enur`; `d\u{e9}mo` prints as itself | ✓ **CLOSED** |
| **Legacy recovery route** | 4 legacy non-ASCII/invisible aliases × `list` then `remove` | all 4 listed, all 4 removed exit 0, only the clean key left | ✓ **TRADE IS REVERSIBLE** |
| **Guard nine WR-01, planted by me** | inserted `pass8_bare_plant: String,` into the real `DriveArgs` body | `drive_args_declares_no_raw_argv_string_field` **FAILED** (13 ≠ 12); reverted, tree clean | ✓ **WR-01 CLOSED** |
| Guard nine WR-02 | read `judge_declaration` `:2894-2912` and `declared_type_text` `:2922-2929` | no early return; `assert_eq!` on parsed type; `"(Vec<OsString>, String)" != "Vec<OsString>"` | ✓ CLOSED (read, not planted — see gaps_closed) |
| Ignored-set arithmetic | `driver_injection_corpus` result line from my own run | 13 passed / **10 ignored** = 7 arms + 2 controls + 1 meta-check | ✓ CONFIRMS the census |
| Escalation cap | `driver_escalation_cap` in my own run | 8 passed / 0 failed | ✓ PASS |
| Debt markers in the round-7 diff | `git diff f1faa3e..HEAD -- src/ tests/` filtered on `^+` | **0** `TBD/FIXME/XXX`, **0** `TODO/HACK/PLACEHOLDER` over 2108 added lines; control needle `identity` → **52**, so neither zero is vacuous | ✓ PASS |
| Raw-invisible-character prohibition | scanned every tracked file under `src/` and `tests/` for `Cf` + the four named default-ignorables | **2 hits, both U+200C**, both inside `tests/fixtures/injection-corpus/.planning/REQUIREMENTS.md` — a hostile-payload FIXTURE, not code | ✓ PASS (fixture, by design) |
| REQUIREMENTS.md untouched | `git log -- .planning/REQUIREMENTS.md` | Last commit `0c4f712`, five rounds ago | ✓ PASS |
| Documented flakes | `driver_reattach`, `envelope_tracer` | Neither fired | ✓ NOT A GAP |

### Probe Execution

Not applicable — this phase is not a migration/tooling phase and declares no
`scripts/*/tests/probe-*.sh`.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| DRIVE-01 | 21-01, 04, 07, 09, 11, 13, 15, 17, 19 | User states a goal once; driver pursues it without further input | ✓ SATISFIED | Criteria 1 and 2 verified. The alias selecting which project the goal is pursued against can no longer name two projects that render alike — measured at the binary, 1 of 16 accepted. |
| DRIVE-03 | 21-01, 03, 04, 07, 08, 09, 11, 13, 15, 17, 19 | Goal decomposed into a structured, machine-checkable, reviewable plan | ✓ SATISFIED | Criterion 1 verified; `driver_goal_seam` 22/22. The record's `goal`/`gsd_command` can no longer carry any `Cf` or default-ignorable value, so the D-30 absent-vs-present guarantee holds for the whole standard-defined invisible class. |
| DRIVE-04 | 21-02, 04, 06, 10, 12, 14, 16, 18, 20 | Escalation capped per run; exceeding it parks | ✓ SATISFIED | Criterion 3 verified; both cap directions pinned and green (8/8) in my own run. |
| SAFE-07 | 21-01, 03, 05, 06, 08, 10, 12, 14, 16, 18, 20 | `.planning/` content passed inside an explicit untrusted boundary | ? NEEDS HUMAN | Criterion 4 behavior-unverified. Thirteen structural pins execute and pass; all ten behavioural tests are `#[ignore]`d and have never run under any verification pass. Now tracked as a standing item rather than a one-time qualification. |
| SAFE-08 | 21-01, 05, 06, 12, 14, 16, 17, 19 | Model's action constrained to a fixed enum; no free-form shell strings | ✓ SATISFIED | Criterion 5 verified and pass 7's hold-back removed: the tag block U+E0000–U+E007F is now refused at every identity seam by the alphabet, and the phase-token check is a property of the value rather than of roadmap contents. |

**No orphaned requirements.** The union of `requirements:` across all twenty plans
is exactly {DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08}, matching
REQUIREMENTS.md's phase-21 mapping and the ROADMAP `Requirements:` line.

**Note for whoever closes this phase:** four of the five requirements are now
SATISFIED and REQUIREMENTS.md still reads `[ ]` / `Gaps Found` for all five. That
is correct today — the prohibition reserves status changes for a *passed*
verification, and this is `human_needed`. It flips when SAFE-07's human run is
recorded.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/main.rs` | `:128` | `Removed project '{}'` prints the alias RAW while `list` escapes it | ⚠️ Warning | MEASURED: `Removed project 'gsd-<U+202E>nur'`, which in a bidi-aware terminal reads `Removed project 'gsd-run'` — the operator is told they removed a different project than they did. This is the confirmation for the exact command `src/text.rs:192-193` and the refusal messages name as the recovery route for these rows. 21-19 truth 5 names three surfaces and all three are escaped, so the truth is TRUE AS STATED; this is a fourth surface the truth does not claim. Fix is one call to `display_identity`. |
| `src/main.rs` | `:91`, `:359` | `eprintln!("Error: {refusal}")` raw, while `judged_alias_or_exit` (`:43`) escapes the same refusal type | ⚠️ Warning | MEASURED: the `OutsideIdentityAlphabet` message echoes the raw alias. Not exploitable for bidi (clause 2 catches every `Cf` value first and its own `Display` escapes), so the harm is inconsistency rather than spoofing — but the D-19-5 rule stated at `:35-42` is applied at one of three refusal sites. |
| `src/text.rs` | `:80-103` | The free-text emptiness residual is real and UNDISCLOSED | ⚠️ Warning | MEASURED with witnesses derived independently of the implementation: `carries_visible_content` returns **true** for a lone U+2800 (BRAILLE PATTERN BLANK), U+E000 (PUA), U+0378 (unassigned) and U+0301 (lone combining mark), so `--goal` and `--command` accept a payload that renders as nothing. None can reach an identity — the alphabet refuses all four — so this is display honesty in a user-supplied string, not the pass-7 harm. It is a Warning rather than a gap for the reason argued in Judgment 2. What makes it a finding at all is that this phase's own standard is to name a residual WITH its direction, and this doc names the class it covers without naming what falls outside it. One sentence closes it. |
| `tests/spawn_seam_guard.rs` | `:3644-3654` | The `narrow_visible` equality catches a bare field but DIAGNOSES the opposite cause | ⚠️ Warning | When I planted `pass8_bare_plant: String,` the guard went red — correct — with the message "a difference means the widening is matching something that is not a field declaration." It was not: a real field declaration was added in a spelling the narrow rule cannot see. An executor repairing under that message would narrow `is_field_opener` back, undoing round 7's fix. The bound is real; the sentence points the wrong way. (The offender assertion at `:3705-3716` would also have fired, but it is downstream of this `assert_eq!` and never runs.) |
| `src/text.rs` | `:419-442` | The default-ignorable half of the class is thirteen hand-named members | ℹ️ Info | The last hand-enumeration standing, and it is correctly disclosed at `:408-417` in the strongest available words. Cannot reach an identity. Recorded so a ninth reader inherits it as a known residual rather than discovering it. |
| `src/main.rs` | `:145-147` | `list` shows the escaped form; `remove` requires the raw bytes | ℹ️ Info | A user who sees `gsd-U+202Enur` in `list` cannot copy it into `remove`. The recovery route the docs name is not round-trippable from the tool's own output. Low severity — the entries are rare and legacy-only — but worth a line in whatever closes the display work. |

**No unreferenced debt markers were introduced.** `git diff f1faa3e..HEAD -- src/ tests/`
adds zero `TBD`/`FIXME`/`XXX` and zero `TODO`/`HACK`/`PLACEHOLDER` across 2108
added lines; the control needle `identity` returns 52 added lines, so neither
zero is vacuous.

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
|---|---|---|---|---|---|---|
| `src/text.rs` `mod tests` | DRIVE-01, DRIVE-03, SAFE-08 | 9 | 0 | **No — fixed** | Value + exhaustive property | ✓ The corpus is no longer sampled from inside the predicate. The sweep's oracle is a second crate; a third oracle (mine) agrees with both. The `format_seen >= 150` floor is committed, not an executor ritual. |
| `tests/driver_injection_corpus.rs` | SAFE-07 | 13 | **10** | No | Behavioral (in the skipped arms) | ⚠️ Unchanged: the requirement's behavioural proof is entirely in the ignored set. Now with an ACTIVE red-capable census of its own arithmetic. |
| `tests/spawn_seam_guard.rs` | SAFE-07, SAFE-08 | 38 | 0 | No | Value, with planted-defect controls | ✓ Both pass-7 holes closed; I planted one myself and watched it go red. One message diagnoses the wrong cause (Warning). |
| `tests/driver_escalation_cap.rs` | DRIVE-04 | 8 | 0 | No | Behavioral, both directions | ✓ |
| `tests/driver_goal_seam.rs` | DRIVE-01, DRIVE-03 | 22 | 0 | No | Value + behavioral | ✓ |
| `tests/registry_test.rs` | DRIVE-01, SAFE-08 | 15 | 0 | No | Behavioral, incl. legacy removability | ✓ `a_legacy_alias_the_alphabet_refuses_is_still_removable` exists at `:365` and I reproduced its claim against the binary. |

**Disabled tests on requirements:** 10, all on SAFE-07 → the requirement's only
behavioural arms; unclosable inside verification. **Circular patterns detected:
0** — pass 7's finding is fixed, and fixed at the sampling rather than by adding
literals. **Insufficient assertions:** 0.

### Decision Coverage

`21-CONTEXT.md`'s trackable decisions plus the round-7 additions are honoured
across the plans and the diff: D-19-1 (the derived class), D-19-2 (the finite
alphabet and its recorded product trade), D-19-3 (the independent oracle and the
restated `visibly_empty_numbered_entry` exception), D-19-4 (the fixture docs
corrected rather than grown), D-19-5 (render-side escaping), D-19-6 (free text
keeps its joiners), D-18-2 (`OsString` deny-by-default) and D-18-5 (prior
artifacts not edited). Each appears in code, in a SUMMARY disclosure list, or in
both. No decision vanished during execution. Non-blocking, as this gate always is.

### Human Verification Required

One item, and it is the same item pass 7 raised. Everything else in this report
is a code-level fact — measured against the built library with oracles I derived
myself, measured end to end against `target/debug/gsd-meta-manager` at HEAD, or
read directly from the source. Nothing is taken from a SUMMARY, from
`21-REVIEW.md`, or from a previous verification pass.

#### 1. Execute SAFE-07's boundary live

**Test:** With an authenticated `claude` CLI available, run
`cargo test --test driver_injection_corpus -- --ignored --nocapture` from the
repository root. Record the CLI version beside the result.

**Expected:** 10 passed, 0 failed. Each `corpus_*_arrives_and_leaves_the_command_unchanged`
arm asserts the hostile payload ARRIVED at the model before asserting the command
was unchanged; the two suppression controls show the positive/negative
`CLAUDE_CODE_DISABLE_CLAUDE_MDS` pair diverging; and
`both_arms_of_every_class_comparison_were_really_executed` confirms both arms
really ran.

**Why human:** All ten spawn the real model binary and require an authenticated
subscription, so they cannot execute inside verification. **No agent can close
this, and I am not going to score it as though one could.** The thirteen
structural pins that do execute prove the channel is the only channel — they
cannot prove the model's behaviour on it. The last recorded live run is an
executor claim from twenty-eight commits ago that no verification pass has
reproduced. Round 7 did the only thing available to it: the arithmetic is now a
red-capable mechanism and the standing fact lives in `deferred-items.md` where
it is read every round instead of in a SUMMARY where it was read once.

### Gaps Summary

**No gaps.** For the first time in this phase, every ROADMAP success criterion
that an agent can verify is verified, and the one that cannot be is honestly
carried as unverified with a tracked human item.

**What closed, and why I believe it rather than merely reading it.** The
character class is derived from Unicode's own data and I confirmed its
completeness with a third oracle — CPython's `unicodedata`, which nothing in this
tree touches. All 170 `Cf` code points it names are refused in both judgments;
the accepted set is empty. Pass 7's eight witnesses are refused in all four argv
positions and `run_paths` returns `None` for each. The identity judgment inverted
to a finite alphabet, and I attacked it with the one shape that distinguishes an
allow-list from any deny-list — a Cyrillic homoglyph — and it held, as did a
Braille blank, a private-use character and an unassigned code point. At the
binary level, sixteen `add` invocations produced one accepted key where pass 7
produced nine indistinguishable ones, and the `gsd-run` spoof now prints as
`gsd-U+202Enur`. Both guard-nine holes are closed; I planted one of them in the
real source myself and watched the guard go red, then reverted. The acceptance
direction did not regress at any of forty seam-value combinations, free text is
still unjudged by the alphabet, and legacy non-ASCII entries are still listable
and removable — so the recorded product trade is reversible in the way the doc
claims. REQUIREMENTS.md is untouched for the fifth round.

**What I want a ninth reader to inherit as OPEN, none of it a gap.** Four
Warnings, all small and all named above with their direction: `Removed project
'…'` prints an alias raw and I measured the bidi spoof through it; two `Add`
refusal sites echo raw while a third escapes; the free-text emptiness residual
(U+2800, PUA, unassigned, lone combining mark) is real, is confined to free text,
and is **undisclosed** in a doc whose own standard is to disclose residuals; and
one guard assertion catches the right defect with a message that diagnoses the
opposite cause, which is the kind of sentence that gets a correct fix reverted.
Plus one Info: the default-ignorable half of the class is still thirteen
hand-named members, correctly disclosed, and it is the last hand-enumeration
standing.

**On the recurrence, since it is the question this phase has been asking itself
for six rounds.** It is genuinely broken for identities and genuinely moved for
free text, and the difference is structural rather than rhetorical. Identities
are judged by a finite printable set, which cannot be one item short; my
homoglyph witness is the proof, because no widening of any invisible-character
deny-list would ever have refused it. Free text keeps a deny-list because an
ASCII allow-list there would refuse legitimate script, so its completeness is
bounded by a pinned Unicode version and by thirteen hand-named default-ignorable
members — both disclosed at the site, with the staleness obligation and the
refresh path named. The sampling fix is what makes even the moved half
trustworthy: the falsifying property left the fixtures and became an exhaustive
sweep against a second crate, and the two fixture consts had their false
anti-tautology claims deleted rather than quietly grown. That deletion is the
part I trust most, because it cost the round something and because it is what my
own independent oracle then confirmed.

### Recommendation

**No round 8 is required to close a criterion.** Criterion 4 needs a human with a
`claude` subscription and nothing else; run
`cargo test --test driver_injection_corpus -- --ignored --nocapture` and record
the version and result in `deferred-items.md`, and the phase closes at 5/5.

**Four Warnings are worth one small follow-up commit, not a gap-closure round.**
Route `Removed project '{}'` and the two `Add`-arm refusal echoes through
`display_identity`, so all four identity display surfaces agree. Add one sentence
to `carries_visible_content`'s doc naming the free-text residual and its
direction — that a blank-rendering character outside `Cf ∪ Default_Ignorable`
(U+2800, private use, unassigned, a lone combining mark) is accepted in free text
and cannot reach an identity. Rewrite the `narrow_visible` assertion message so
it names both causes, since a bare field is the likelier one. None of these
changes behaviour a criterion depends on, and bundling them into a seventh
gap-closure round would be treating polish as failure.

**What is NOT in scope, unchanged from pass 7:** TR39 confusables in free text.
The carve-out at `src/text.rs:219-229` is correct and now states honestly where
the homoglyph harm IS closed (identity seams, by the alphabet — I verified it)
and where it is not (free text, by design). Open it as its own roadmap item if
it is wanted.

**One thing to carry forward and one to stop carrying.** Carry forward the
sampling discipline: the property that can falsify a predicate belongs in the
sampling, never in the literals, and this round finally implemented that. Stop
carrying the assumption that the next level down must also be hand-enumerated —
round 7 showed there is a third option, which is to make the accepted set finite
so there is no next level down.

---

_Verified: 2026-08-25T19:36:27Z_
_Verifier: Claude (gsd-verifier), adversarial stance_
_HEAD `8dc8c98` · eighth verification pass · pass 7 preserved at `84143bb`_
