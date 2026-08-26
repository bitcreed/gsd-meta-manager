---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 22
subsystem: testing
tags: [unicode, identity-alphabet, guard-docs, posix-quoting, deny-list, disclosure, clippy]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening (round 7, plan 21-19)
    provides: "text::is_identity_char (the finite alphabet), text::is_invisible_formatting_char (the derived class), journal::is_plain_path_component's alphabet clause (D-19-2)"
  - phase: 21-llm-goal-layer-prompt-injection-hardening (round 8, plan 21-21)
    provides: "the merged wave-1 tree this plan re-measured against; text::is_invisible_formatting_char widened to pub(crate); the ratatui 0.30 Buffer measurement quoted by this plan's staleness obligation"
provides:
  - "`narrow_visible`'s assertion message now names BOTH causes of a count difference with the LIKELIER one first, and states which repair belongs to which — so an executor repairing under it widens the narrow expectation instead of reverting round 7's `is_field_opener` widening."
  - "`WITNESS_ALLOWED_ELSEWHERE`'s doc replaced its unmeasured bound with the two residuals it hides, each named with its failure direction and each MEASURED by a plant that left the census green. The table, its four rows, its reason column and its both-ways `assert_eq!` are byte-identical."
  - "The tree carries ONE spelling of the identity alphabet: `envelope::advisory::is_plain_component` delegates its character clause to `text::is_identity_char`, certified by a source census committed RED at two occurrences and green at one."
  - "`envelope::hooks::stub_body`'s security rationale corrected from a false premise to the post-D-19-2 truth, with a defence-in-depth reason and a new committed control (`the_path_component_predicate_refuses_every_shell_metacharacter`) behind it. `sh_quote` and the generated stub untouched."
  - "`carries_visible_content`'s doc discloses the free-text emptiness residual with its direction, the reason it cannot reach an identity, and the two bounds on the deny-list's completeness — pinned by a committed test so the disclosure cannot go stale in either direction."
  - "`deferred-items.md` gains the pre-existing clippy todo with its empty-log evidence, a standing ratatui-version staleness obligation for invisible-character rendering, and ROADMAP criterion 4 re-surfaced verbatim and unclosed."
affects: [round 9, any future ratatui upgrade, any future widening of the identity alphabet, any executor repairing guard nine]

actuals:
  tokens: 10140    # chars/4 over the realized diff: 40560 chars / 4
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "A residual named in a doc must be demonstrated by a plant that leaves the guard GREEN, captured verbatim and reverted — a doc asserting an unmeasured residual is the same defect as a doc asserting an unmeasured bound, pointed the other way."
    - "An assertion message is an INSTRUCTION an executor acts on under a red test, so it names every cause it can have, likeliest first, with the repair for each."
    - "A one-spelling claim is closed by DELEGATION plus a source census asserted as an EQUALITY on a count, not by softening the claim."
    - "A behaviour-preserving refactor is proven by exhaustive set comparison over the whole domain, not by the suite staying green."

key-files:
  created: []
  modified:
    - tests/spawn_seam_guard.rs
    - src/text.rs
    - src/envelope/advisory.rs
    - src/envelope/hooks.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md

key-decisions:
  - "D-22-1: WITNESS_ALLOWED_ELSEWHERE is KEPT; only its doc changes. Both-ways assert_eq! over a BTreeMap fails loudly, it governs a hygiene scan rather than a production predicate, and all four rows re-measured correct at this tree."
  - "D-22-2: each residual the rewritten doc names is MEASURED by planting, not argued. Both plants left the census green; both were reverted."
  - "D-22-3: WR-03 closed by DELEGATION, making the one-spelling claim true, rather than by narrowing the claim. The two sets were measured byte-for-byte identical over all 1,112,064 Unicode scalar values."
  - "D-22-5: the pre-existing clippy lints are RECORDED, not fixed; `git log 2074595..HEAD` is empty for both files."
  - "D-22-6: the ratatui-version dependence of invisible-character rendering becomes a standing staleness obligation beside the pinned-Unicode-version one."
  - "New, forced by prohibition 6: the corrected `stub_body` premise and the disclosed free-text residual each gained a COMMITTED control, because a doc stating a bound with no control that goes red for it is the exact defect this plan exists to end."

patterns-established:
  - "A plant-and-revert whose expected result is GREEN is as much evidence as one whose expected result is red — it is how an under-detection residual is measured rather than predicted."
  - "A refactor claimed behaviour-preserving is measured exhaustively over its whole input domain in a throwaway probe, because a suite staying green proves only that the suite did not cover the difference."

requirements-completed: [DRIVE-04, SAFE-07, SAFE-08]
# Copied verbatim from this plan's `requirements` frontmatter, as the template
# requires. **This field is a declaration of the plan's SCOPE, not a completion
# verdict.** Requirement status in .planning/REQUIREMENTS.md is decided
# exclusively by a passed verification (828d7cc, 0c4f712 — twice reverted, six
# rounds held), and no commit of this plan touches that file (proved below).

coverage:
  - id: D1
    description: "The assertion message that would have got round 7's `is_field_opener` widening reverted now names the bare-declaration cause FIRST and states the repair for each cause."
    requirement: "DRIVE-04"
    verification:
      - kind: manual_procedural
        ref: "re-planted a bare field in the REAL DriveArgs body twice; OLD and NEW messages captured verbatim; `rtk proxy git status --porcelain` clean after each revert"
        status: pass
      - kind: manual_procedural
        ref: "`git diff -U0 -- tests/spawn_seam_guard.rs | grep '^-'` — every deleted line is a comment, doc line or message-string fragment; zero compared expressions changed"
        status: pass
    human_judgment: false
  - id: D2
    description: "`WITNESS_ALLOWED_ELSEWHERE`'s doc states what the census delivers and names both residuals with their directions, each measured by a plant; the table is byte-identical."
    verification:
      - kind: manual_procedural
        ref: "plant 1 (seven non-witness DEGENERATE members in an allowed file) -> spawn_seam_guard 38 passed / 0 failed, GREEN; reverted"
        status: pass
      - kind: manual_procedural
        ref: "plant 2 (occurrence deleted and hand-copy occurrence added in the same allowed file) -> the_degenerate_payload_set_is_spelled_in_exactly_one_place 1 passed / 0 failed, GREEN; reverted"
        status: pass
      - kind: manual_procedural
        ref: "const extraction from HEAD vs worktree: 31 lines each, BYTE-IDENTICAL True, 4 rows"
        status: pass
    human_judgment: false
  - id: D3
    description: "`stub_body`'s doc no longer states that the path-component predicate accepts a shell metacharacter; the replacement premise is a measurement with a committed control behind it, and `sh_quote` plus the generated stub are unchanged."
    verification:
      - kind: unit
        ref: "src/envelope/hooks.rs#the_path_component_predicate_refuses_every_shell_metacharacter"
        status: pass
      - kind: manual_procedural
        ref: "`git diff abcb367..HEAD -- src/envelope/hooks.rs | grep -cE '^[+-].*sh_quote\\('` -> 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "The tree carries exactly one executable spelling of the identity alphabet's character clause under `src/`, and the delegation changed no accepted or refused segment."
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/text.rs#exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src (RED at 2 in bce8e89, green at 1 in b38451c)"
        status: pass
      - kind: integration
        ref: "cargo test --test envelope_advisory — 9 passed / 0 failed BEFORE (old clause temporarily restored) and AFTER"
        status: pass
      - kind: other
        ref: "exhaustive set comparison over 1,112,064 Unicode scalar values: OLD clause 65 accepted, is_identity_char 65 accepted, 0 disagreements"
        status: pass
    human_judgment: false
  - id: D5
    description: "`carries_visible_content`'s doc discloses the free-text residual with its direction, its identity bound, and the two bounds on the deny-list's completeness."
    verification:
      - kind: unit
        ref: "src/text.rs#the_free_text_emptiness_residual_is_accepted_and_cannot_reach_an_identity — four witnesses, both halves"
        status: pass
    human_judgment: false
  - id: D6
    description: "The register carries the pre-existing clippy todo with measured locations and empty-log evidence, the ratatui-version staleness obligation, and criterion 4 re-surfaced verbatim and unclosed."
    verification:
      - kind: manual_procedural
        ref: "`git diff --numstat -- deferred-items.md` -> 135 additions, 0 deletions; `cargo clippy --all-targets` -> 4 errors, locations quoted; `git log --oneline 2074595..HEAD -- src/browser.rs src/project_creator.rs` -> empty"
        status: pass
    human_judgment: false
  - id: D7
    description: "The four authored edge-probe rows reconfirmed against this plan's final tree by their named suites."
    requirement: "DRIVE-04"
    verification:
      - kind: integration
        ref: "tests/driver_escalation_cap.rs — 8 passed / 0 failed (DRIVE-04 boundary and precision)"
        status: pass
      - kind: integration
        ref: "tests/driver_injection_corpus.rs — 13 passed / 0 failed / 10 ignored, file unmodified (SAFE-07 boundary, STRUCTURAL half only)"
        status: pass
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#the_arrival_evidence_field_is_named_only_where_the_schema_declares_it — 1 passed / 0 failed (SAFE-07 precision, guard seven)"
        status: pass
    human_judgment: false
  - id: D8
    description: "SAFE-07's BEHAVIOURAL half — that a `.planning/` file carrying injected instructions does not change which command the driver executes when the real model reads it (ROADMAP criterion 4)."
    requirement: "SAFE-07"
    verification: []
    human_judgment: true
    rationale: "Permanently agent-unclosable: all ten `driver_injection_corpus` tests spawn the real `claude` binary and need an authenticated subscription. NO AGENT CAN CLOSE THIS. Round 8 did not touch it (prohibition 4) and does not claim it — it re-surfaced the exact command and expected output in `deferred-items.md`. Stated as unverified, never certified."
  - id: D9
    description: "Whether the rewritten guard docs actually read as candour to a maintainer — i.e. whether naming two under-detection residuals in `WITNESS_ALLOWED_ELSEWHERE`'s doc makes a future reader keep the table or delete it."
    verification: []
    human_judgment: true
    rationale: "This is a reading judgment about prose, not a property. The plants prove the residuals are real; nothing can prove the sentences describing them will be read the way they are meant. A human has to decide whether the doc now reads as an adjudication a maintainer would trust."

duration: 6h 21m
completed: 2026-08-25
status: complete
---

# Phase 21 Plan 22: What the Tree Says Summary

**The five sentences that would have got a correct mechanism deleted or a correct fix reverted are now true: `narrow_visible`'s message names the bare-field cause first with the repair for each, `WITNESS_ALLOWED_ELSEWHERE`'s doc names two under-detection residuals that were PLANTED rather than predicted, `stub_body`'s false shell-safety premise is replaced by a measurement with a committed control, `advisory::is_plain_component` delegates to `text::is_identity_char` so the one-spelling claim became TRUE (census committed red at two, green at one), and `carries_visible_content` finally discloses the free-text residual its own standard required.**

## Performance

- **Duration:** 6h 21m (first commit `0d55523` 2026-08-25T17:55:14-06:00; last task commit `59df5a3` 2026-08-25T18:13:04-06:00; wall time is dominated by measurement work between commits)
- **Tasks:** 3 (one tracer, two execute)
- **Commits:** 4 task commits + this document
- **Files changed:** 5 — 595 insertions, 30 deletions

## Accomplishments

- **The highest-harm item first, and it was certified by re-planting the defect in the REAL body.** `narrow_visible`'s assertion caught the right defect while its message pointed the repairing executor at `is_field_opener` — i.e. at round 7's fix. Both the old and the new message were captured under an actual bare-field plant, verbatim, side by side below. **The assertion itself is unchanged.** The bound is real; only the sentence pointed the wrong way, and that separation is the whole finding.
- **WR-01's two residuals were MEASURED, not asserted.** Both plants were expected to leave the census GREEN, and both did. A doc naming a residual nobody measured is the same overclaim as a doc naming a bound nobody measured — this is the round that stopped writing either.
- **The table survives, and the doc now says why.** `WITNESS_ALLOWED_ELSEWHERE` is byte-identical to HEAD (31 lines, 4 rows, verified by extraction, not by eye), and its doc carries the adjudication a future reader needs in order not to delete it.
- **One spelling of the identity alphabet, certified by a census committed red at two.** And the delegation's behaviour-preservation was measured **exhaustively** — 1,112,064 scalar values, 65 accepted by each, **0 disagreements** — rather than inferred from the suite staying green.
- **A false security rationale replaced by a measurement plus a control.** Sixteen shell metacharacters, each answered `false`, in a committed test that goes red if the alphabet is ever widened.
- **The free-text residual disclosed with its direction, its bound, and its two completeness limits** — and pinned, so the disclosure and the behaviour cannot drift apart in either direction.
- **Three register additions, 135 lines, zero deletions**, including a standing obligation nobody had written down: which invisible characters reach a terminal cell is a property of ratatui's VERSION, not of this code.

## Task Commits

1. **Task 1 (TRACER): the message that would revert round 7, and two residuals measured by planting** — `0d55523` (docs)
2. **Task 2 RED arm: the alphabet spelling census, observed RED at TWO spellings** — `bce8e89` (test)
3. **Task 2 GREEN: one alphabet spelling, a true security rationale, the free-text residual disclosed** — `b38451c` (fix)
4. **Task 3: the register — pre-existing lints, the ratatui obligation, criterion 4 re-surfaced** — `59df5a3` (docs)

## The four plants, verbatim, each reverted

Every plant below was made against the REAL tree, run, captured, and reverted with
`rtk proxy git status --porcelain` confirming the file clean afterwards.

### Plants 1 and 2 — a bare field in the real `DriveArgs` body

**The plant.** `#[cfg(any())]` + `goal_hint: String,` inserted into the real
`DriveArgs` body in `src/driver/mod.rs`, immediately after `pub dry_run: bool,`.

> **Why the `cfg` gate, stated rather than glossed.** `DriveArgs` has **69**
> construction sites across `src/` and `tests/` (`grep -rn "DriveArgs {" src/
> tests/ | grep -c .`), so an ungated bare field does not compile and the guard
> could not run at all. Guard nine is a **textual scan over source**, so the
> attribute line does not change what it reads — proved by the guard going red
> with exactly the counts a real thirteenth field would produce (13 vs 12). This
> is disclosed rather than presented as an ordinary field.

**BEFORE the rewrite — the message an executor would read** (`cargo test --test spawn_seam_guard drive_args_declares_no_raw_argv_string_field`):

```
thread 'drive_args_declares_no_raw_argv_string_field' (1735476) panicked at tests/spawn_seam_guard.rs:3647:5:
assertion `left == right` failed: the widened `is_field_opener` sees 13 field declarations in the real `DriveArgs` body where the pre-round-7 `pub`-only rule sees 12. Every real field carries `pub`, so the two must agree; a difference means the widening is matching something that is not a field declaration.
  left: 13
 right: 12
```

**That sentence is an instruction to revert round 7.** It says the widening is
matching a non-field. It was not: a real field was added in exactly the spelling
round 7 widened `is_field_opener` to see. An executor repairing under it narrows
`is_field_opener` back to `pub `/`pub(` and restores pass 6's silent signature.

**AFTER the rewrite — the message an executor now reads**, under the same plant:

```
thread 'drive_args_declares_no_raw_argv_string_field' (1739181) panicked at tests/spawn_seam_guard.rs:3657:5:
assertion `left == right` failed: the widened `is_field_opener` sees 13 field declarations in the real `DriveArgs` body where the pre-round-7 `pub`-only rule respelled just above this assertion sees 12. TWO different things produce that difference and they are repaired in OPPOSITE directions. Read both before changing anything.

(1) LIKELIER, and it is what verification pass 8 measured when it planted one: a REAL field was added to `DriveArgs` with NO visibility modifier — `goal_hint: String,` rather than `pub goal_hint: String,`. The widened opener SEES it, which is round 7's fix working: a private field is still a field, `from_argv` still has to destructure it, and pass 7 measured that exact spelling slipping past the old `pub`-only rule with pass 6's silent signature. The respelled `pub`-only expectation above CANNOT see it. THE REPAIR IS TO WIDEN THAT EXPECTATION, here, so it agrees with `is_field_opener` again — and then to check that guard nine's offender scan classifies the new field. DO NOT narrow `is_field_opener` back to `pub `/`pub(`: that reverts round 7 and reopens the hole a bare field slipped through.

(2) ONLY IF NO SUCH FIELD EXISTS: an over-detection in `is_field_opener` — a line inside the extracted region that matches the bare `identifier:` shape without being a field declaration (a match arm, a struct-literal initialiser, a labelled loop). None of those is in `DriveArgs`'s body by construction today, so this cause requires that the extracted region itself widened. THE REPAIR FOR THAT ONE is to narrow `is_field_opener`, and it is correct only after you have read the offending line and confirmed it is not a field.

Tell the two apart by listing the lines the widened opener accepts that the narrow rule rejects, and reading them.
  left: 13
 right: 12
```

`rtk proxy git status --porcelain` after each revert did not name `src/driver/mod.rs`.

**The assertion itself is unchanged.** `rtk proxy git diff -U0 -- tests/spawn_seam_guard.rs | grep "^-"` returns twenty-three deleted lines and **every one of them is a comment line, a doc line, or a fragment of a message string**. Neither `field_lines.len()` nor `narrow_visible` was touched, and the `narrow_visible` filter closure is byte-identical.

### Plant 3 — the WITNESS-COUNTING residual (expected GREEN, and it was)

**The plant.** All **seven** members of `test_support::DEGENERATE` that are NOT
witnesses (`""`, `"   "`, `"\t"`, `"\u{feff}"`, `"\u{00ad}"`, `"\u{e0041}"`,
`"\u{fe0f}"`) added as one hand-copy-shaped array inside `src/text.rs` — a file
`WITNESS_ALLOWED_ELSEWHERE` already allows.

```
$ rtk proxy cargo test --test spawn_seam_guard the_degenerate_payload_set_is_spelled_in_exactly_one_place
running 1 test
test the_degenerate_payload_set_is_spelled_in_exactly_one_place ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.22s

$ rtk proxy cargo test --test spawn_seam_guard
test result: ok. 38 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
```

**CONFIRMS the predicted residual.** Seven of ten members hand-copied into an
allowed file, and not one of the 38 guards in the file moved. Reverted; tree clean.

### Plant 4 — the PER-FILE residual (expected GREEN, and it was)

**The plant.** Inside `src/text.rs`, the legitimate `"\u{200b}"` occurrence was
deleted from the look-alike fixture list and one added in a hand-copy-shaped
array in the same file, leaving the file's executable hit count unmoved:

```
$ rtk proxy grep -n '"\\u{200b}"' src/text.rs
src/text.rs:29://! ...                        <- comment, dropped by executable_hits
src/text.rs:220:/// ...                       <- comment, dropped
src/text.rs:576:        let _plant_hand_copy = ["", "   ", "\u{200b}"];    <- the PLANT
src/text.rs:675:        for invisible in ["\u{200b}", "\u{feff}", ...      <- the legitimate one

$ rtk proxy cargo test --test spawn_seam_guard the_degenerate_payload_set_is_spelled_in_exactly_one_place
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 37 filtered out; finished in 0.22s
```

**CONFIRMS the predicted residual.** Two executable hits before, two after; the
`actual == expected` comparison holds while one occurrence has been replaced by a
hand copy. Reverted; tree clean.

Neither plant refuted its prediction. Had either gone red, the doc would have
recorded what the measurement showed instead — that is the reason for measuring.

## The RED arm, verbatim

The spelling census was committed at `bce8e89` carrying
`#[ignore = "red: WR-03 second spelling; un-ignored in the fix commit"]`, its red
output quoted in both its own doc comment and the commit message, against the tree
where the count is **two**:

```
running 1 test
test text::tests::exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src ... FAILED

---- text::tests::exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src stdout ----

thread 'text::tests::exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src' (1779910) panicked at src/text.rs:810:9:
assertion `left == right` failed: the identity alphabet's character clause is spelled 2 times in executable lines under src/, and `is_identity_char`'s doc claims to be THE one spelling. Sites: ["src/envelope/advisory.rs:569", "src/text.rs:213"]. A second spelling is a boundary that can stop agreeing with the boundary: WR-03 measured exactly that, in `envelope::advisory::is_plain_component`, gating the GitHub owner/repo segments that are interpolated into a request path. The repair is DELEGATION to `crate::text::is_identity_char`, not a softening of the claim.
  left: 2
 right: 1

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1055 filtered out; finished in 0.08s
```

Green at **one** after the delegation, in `b38451c`:

```
test text::tests::exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1056 filtered out; finished in 0.07s
```

**The count re-measured directly, after the fix**, and it demonstrates why dropping
comment lines is load-bearing rather than decorative:

```
$ rtk proxy grep -rn "'.' | '_' | '-')" src/ --include=*.rs
src/text.rs:213:    matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | '-')
src/envelope/advisory.rs:566:/// `c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')` — while
```

A naive grep reads **2**. The census reads **1**, because `advisory.rs:566` is the
doc comment that quotes the old spelling in order to record what changed.

## The delegation, measured exhaustively rather than inferred

The plan warns that a behavioural difference here would mean the byte-for-byte
measurement was wrong. The suite staying green proves only that the suite did not
cover the difference, so the sets were compared over their **whole domain** in a
throwaway `tests/` probe (a committed copy of the old clause would have
re-introduced the second spelling the census now forbids):

```
scalar values examined : 1112064
accepted by OLD clause : 65
accepted by is_identity_char : 65
disagreements          : 0
```

And the suite, both ways:

| `cargo test --test envelope_advisory` | Result |
|---|---|
| BEFORE (old clause temporarily restored) | 9 passed; 0 failed |
| AFTER (delegating) | 9 passed; 0 failed |

**Only the CHARACTER clause moved.** The emptiness check and the two traversal
tokens are unchanged and are **not** redundant: `.` and `..` consist entirely of
characters the alphabet admits, so nothing in the delegated clause refuses them —
which is the same fact 21-21's D-21-5 pinned from the other side.
`default_branch_of`'s nearby set, which additionally admits `'/'`, is left alone
with its difference stated: a branch name legitimately carries a separator and is
not an identity.

## The corrected premises, measured rather than asserted

### WR-02 — `is_plain_path_component` and the shell metacharacters

The old sentence: *"`is_plain_path_component` accepts a quote character."*
Measured at this tree by the new committed control
`the_path_component_predicate_refuses_every_shell_metacharacter`, run with
`--nocapture`:

```
single quote   '\''  is_plain_path_component -> false
double quote   '"'  is_plain_path_component -> false
backtick       '`'  is_plain_path_component -> false
dollar         '$'  is_plain_path_component -> false
semicolon      ';'  is_plain_path_component -> false
space          ' '  is_plain_path_component -> false
backslash      '\\'  is_plain_path_component -> false
pipe           '|'  is_plain_path_component -> false
ampersand      '&'  is_plain_path_component -> false
open paren     '('  is_plain_path_component -> false
close paren    ')'  is_plain_path_component -> false
asterisk       '*'  is_plain_path_component -> false
question mark  '?'  is_plain_path_component -> false
less than      '<'  is_plain_path_component -> false
greater than   '>'  is_plain_path_component -> false
newline        '\n'  is_plain_path_component -> false
```

**Sixteen for sixteen refused.** The doc now states that truth and gives the
reason the quoting stays anyway — it is the only defence that survives a later
widening of the alphabet (whose own doc calls reverting the clause "costly, NOT
one-way"), and a script git execs in a separate process is not a place to depend
on a predicate defined three modules away. `sh_quote` and the generated stub are
untouched.

### The free-text residual — all four witnesses re-measured against the built library

```
BRAILLE PATTERN BLANK (So, a symbol - not a format char)
  carries_visible_content   -> true
  is_plain_path_component   -> false
the first private-use code point (Co)
  carries_visible_content   -> true
  is_plain_path_component   -> false
an unassigned code point (Cn) at the pinned version
  carries_visible_content   -> true
  is_plain_path_component   -> false
COMBINING ACUTE ACCENT (Mn), with no base character
  carries_visible_content   -> true
  is_plain_path_component   -> false
```

Four out of four **accepted in free text** and four out of four **unable to reach
an identity**. Direction: over-permissive, in free text only. The disclosure names
that direction, names the alphabet as what bounds it, and names the two things
that bound the deny-list's own completeness — the pinned Unicode version, and the
thirteen hand-named default-ignorable members with no second machine oracle. The
predicate is deliberately unchanged.

## Prohibition audit — all NINE, against THIS plan's own diff

Every row is a command and its output. **No row is a grep for a sentence of
prose.** WR-01 exists precisely because 21-20 did not do this.

| # | Prohibition | Command | Output | Verdict |
|---|---|---|---|---|
| 1 | MUST NOT delete or shrink `WITNESS_ALLOWED_ELSEWHERE`, its rows, or its both-ways `assert_eq!` | extraction of the const from `git show HEAD:...` vs the worktree, compared line by line | `HEAD const lines: 31` / `WORK const lines: 31` / `BYTE-IDENTICAL: True` / `rows: 4` | **HELD.** Not one byte between the const's brackets changed. `grep -c WITNESS_ALLOWED_ELSEWHERE` is now `7` (was `5`) — the two extra are the doc cross-references this plan added, both outside the const. |
| 2 | MUST NOT close WR-03 by softening `src/text.rs`'s one-spelling claim | `git diff abcb367..HEAD -- src/text.rs \| grep -c "ONE spelling of the identity alphabet"` | `0` | **HELD.** The claim's text does not appear in the diff at all; it was made TRUE by delegation. The census is the certificate: red at 2, green at 1. |
| 3 | MUST NOT remove `sh_quote` or weaken the POSIX quoting | `git diff abcb367..HEAD -- src/envelope/hooks.rs \| grep -cE "^[+-].*sh_quote\("` | `0` | **HELD.** Every deleted line in that file's diff is one of the three doc lines carrying the false premise. `a_quote_in_an_alias_cannot_escape_the_generated_stub` still passes. |
| 4 | MUST NOT plan, execute or claim any work against ROADMAP criterion 4 | `git diff abcb367..HEAD --stat -- tests/driver_injection_corpus.rs` | *(empty)* | **HELD.** File untouched; its ten `#[ignore]`d tests still report `10 ignored`. The register re-surfacing is RECORDING, and the SUMMARY states the behavioural half as unverified (coverage D8, `human_judgment: true`). |
| 5 | MUST NOT fix the four pre-existing `--all-targets` lints | `git diff abcb367..HEAD --stat` | names only `src/envelope/advisory.rs`, `src/envelope/hooks.rs`, `src/text.rs`, `tests/spawn_seam_guard.rs` (+ `deferred-items.md`) — neither `src/browser.rs` nor `src/project_creator.rs` | **HELD.** `cargo clippy --all-targets -- -D warnings` still ends `due to 4 previous errors`, unchanged. |
| 6 | MUST NOT claim a bound no committed control goes red for, or name a residual nobody measured | every residual and bound written in this diff, listed with its control | see the table below | **HELD** — and it forced two controls this plan would otherwise not have written. |
| 7 | MUST NOT certify a claim with a prose grep; every count re-measured under `rtk proxy` against this plan's final tree | *(this SUMMARY)* | every number here is a command output, a test result, a planted red/green, or a byte comparison | **HELD.** No number is inherited from `21-REVIEW.md`, `21-VERIFICATION.md`, `21-21-SUMMARY.md` or the plan's own text — the one number quoted from 21-21 (the ratatui buffer measurement) is quoted as 21-21's measurement, explicitly, because the plan directs that it be quoted rather than re-derived. |
| 8 | MUST NOT paste a raw invisible, bidi, tag or variation-selector character | scanner (`Cf` union the VS/tag/filler ranges) over all five files this plan writes, **then re-run against this SUMMARY itself** | `tests/spawn_seam_guard.rs: 0` / `src/text.rs: 0` / `src/envelope/advisory.rs: 0` / `src/envelope/hooks.rs: 0` / `deferred-items.md: 0` / `TOTAL: 0`; this SUMMARY: **0** | **HELD.** Re-run against this document specifically because 21-21 caught a violation of this prohibition *inside the artifact auditing it*. |
| 9 | MUST NOT flip any `.planning/REQUIREMENTS.md` requirement | `git log --format=%H abcb367..HEAD -- .planning/REQUIREMENTS.md` | *(empty)* | **HELD** — sixth consecutive round. |

### Prohibition 6 in detail — every bound and residual this diff writes, with its control

| Statement written in this diff | Kind | What goes red for it |
|---|---|---|
| `narrow_visible`'s two causes and their repairs | bound + diagnosis | The assertion itself, re-planted twice; both messages captured verbatim above. |
| "an allowed site cannot grow a second occurrence **of that witness** unnoticed" | bound (narrowed) | `the_degenerate_payload_set_is_spelled_in_exactly_one_place`'s both-ways `assert_eq!`, unchanged and passing. |
| Residual 1 — the census counts witnesses, not members | residual, silent under-detection | **Plant 3**, captured green above, reverted. |
| Residual 2 — the census is per-file, not per-line | residual, silent under-detection | **Plant 4**, captured green above, reverted. |
| Residual 3 — a copy carrying none of the three witnesses | residual, silent under-detection | Unchanged from round 7; disclosed only, and said to be disclosed only. |
| "`is_plain_path_component` refuses every shell metacharacter" | bound | **NEW control** `the_path_component_predicate_refuses_every_shell_metacharacter` (16 rows). |
| "the quoting is what survives a widening of the alphabet" | rationale, not a bound | `a_quote_in_an_alias_cannot_escape_the_generated_stub` pins the quoting itself, independently of the rationale. |
| "exactly one executable spelling of the alphabet under `src/`" | bound | `exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src`, committed red at 2. |
| The census's own under-detection (a third, differently-constructed spelling) | residual, silent | Named in its own doc as bounded by the DELEGATION, not by the scan. No control claimed. |
| "a lone out-of-class blank-rendering character is accepted in free text" | residual, over-permissive | **NEW control** `the_free_text_emptiness_residual_is_accepted_and_cannot_reach_an_identity`, four witnesses, both halves. |
| "none of them can reach an identity" | bound | Same control, second assertion per row. |
| "the deny-list's completeness is bounded by the pinned Unicode version and the thirteen hand-named members" | residual | Both already disclosed at their own sites (`is_invisible_formatting_char`, `named_default_ignorable_members_beyond_cf_are_inside_the_class`); referenced, not newly claimed. |
| "ratatui 0.30 drops zero-width graphemes; the tag block survives" | dependency-version fact | 21-21's measurement, quoted as 21-21's; the standing obligation says to re-measure on upgrade. No control, and it says so. |

## Named-shape audit

| # | Named shape | Owner | Resolved by |
|---|---|---|---|
| 1 | An assertion catching the right defect with a message diagnosing the opposite cause | 21-22 T1 | `tests/spawn_seam_guard.rs` — message rewritten, both causes named, likelier first, repair stated for each; certified by two re-plants in the REAL `DriveArgs` body. **RESOLVED.** |
| 2 | `WITNESS_ALLOWED_ELSEWHERE`'s doc claiming a bound the census does not perform (WR-01) | 21-22 T1 | Doc rewritten at BOTH claim sites plus the assertion message; both residuals named with direction and each MEASURED by a plant; table byte-identical. **RESOLVED.** |
| 3 | A false security rationale a maintainer would act on (WR-02) | 21-22 T2 | `src/envelope/hooks.rs` — premise corrected, measured by a new committed control, defence-in-depth reason given; `sh_quote` untouched. **RESOLVED.** |
| 4 | A second spelling of the identity alphabet at a request-path seam (WR-03) | 21-22 T2 | `src/envelope/advisory.rs` delegates; census committed red at 2, green at 1; equivalence measured over all 1,112,064 scalar values. **RESOLVED.** |
| 5 | The free-text emptiness residual undisclosed in a doc whose own standard is disclosure | 21-22 T2 | `carries_visible_content`'s doc, with direction, identity bound, and the two completeness bounds; pinned by a committed test. **RESOLVED.** |
| 6 | Pre-existing clippy failures readable as round-8 breakage | 21-22 T3 | `deferred-items.md` — four lints with file:line and kind, plus the empty `git log 2074595..HEAD`. **RESOLVED (recorded, not fixed).** |
| 7 | Invisible-character rendering believed to be a property of this code | 21-22 T3 | `deferred-items.md` — standing ratatui-version staleness obligation with a version table. **RESOLVED (recorded).** |
| 8 | The render surface, `project_list.rs`, the `remove` echo, the refusal echoes, `NotPlainComponent`, CR-01's falsified records | 21-21 | **DELEGATED** — see `21-21-SUMMARY.md`'s own audit. |
| 9 | Criterion 4's behavioural half | **nobody, deliberately** | **UNOWNED BY DESIGN.** Permanently agent-unclosable; re-surfaced in the register; never claimed. Carried in this SUMMARY as coverage D8 with `human_judgment: true`. |

## Measurement corrections — every count this plan asserted that moved between `8dc8c98` and the tree it landed on

The plan measured at `8dc8c98`, before wave 1 (`21-21`) landed. Wave 1 rewrote
`src/text.rs`'s class doc and widened `is_invisible_formatting_char` to
`pub(crate)`, which moved everything below it. Every number was re-measured here.

| What the plan asserts | Measured at this tree | Moved? |
|---|---|---|
| `src/text.rs:155` — the one-spelling claim | The doc for `is_identity_char` begins at **`src/text.rs:201`** and the fn is at **`:250`** after this plan's own additions; it was at `:163`/`:212` on the tree this plan landed on, and `21-VERIFICATION.md` recorded `:204-206` at `8dc8c98` | **YES — moved twice.** The claim itself is unchanged and is the one this plan made true. Cited by NAME rather than by line in every artifact this plan writes. |
| `src/text.rs` "lines 128-160: `is_invisible_formatting_char`'s pinned-version disclosure" | `pub(crate) fn is_invisible_formatting_char` at **`:190`** after this plan; the disclosure paragraph is inside its doc | **YES.** Wave 1 rewrote that doc and added the `pub(crate)` rationale paragraph. |
| `src/text.rs` "lines 78-108: `carries_visible_content` and its doc" | doc opens at **`:80`**, fn at **`:142`** after this plan's disclosure was added | **Partly** — the doc's start was right; the fn moved because this plan lengthened the doc. |
| `tests/spawn_seam_guard.rs:3646-3656` — the `assert_eq!` whose message is the subject | The `assert_eq!(` token was at **`:3647`** before this plan (panic reported `3647:5`), and is at **`:3657`** after | **Off by one at the start.** The range covered the right assertion. |
| `advisory.rs:569` — the second spelling | **`src/envelope/advisory.rs:569`**, exactly (the census's own red output names it) | **NO.** |
| `hooks.rs:155` — the false premise | **`src/envelope/hooks.rs:155`**, exactly | **NO.** |
| `src/journal/mod.rs:339` — D-19-2's alphabet clause | **`:339`**, exactly | **NO.** |
| "an allowed file can grow any of the **seven** members that are not witnesses" | `DEGENERATE` has **10** members; `degenerate_witnesses()` returns **3**; **7** non-witnesses | **NO — confirmed.** |
| `WITNESS_ALLOWED_ELSEWHERE`'s four rows | witness 1 -> `src/journal/writer.rs` **1**, `src/text.rs` **2**; witness 2 -> `src/journal/writer.rs` **1**, `src/text.rs` **1**. All four correct; the census's own both-ways `assert_eq!` passes | **NO — all four confirmed at this tree.** |
| "four PRE-EXISTING clippy lints" | **4**, at `src/browser.rs:131,132,133` and `src/project_creator.rs:146` | **NO.** |
| `grep -c WITNESS_ALLOWED_ELSEWHERE` (21-21 measured `5`) | **7** after this plan | **YES — caused by this plan**, and both extra mentions are the doc cross-references it added. Recorded so a future cross-check does not read the change as growth in the table. |

**The table is non-empty**, as the plan requires it to be if anything moved: five
of eleven asserted locators moved, all of them in `src/text.rs` or by one line in
the guard, and none of them changed what the work is.

## Decisions Made

Followed the plan's decision table (D-22-1 … D-22-6) as written. One decision was
forced during execution and is recorded because the plan did not cover it:

1. **Two NEW committed controls were added that the plan asked only to
   "measure".** Prohibition 6 forbids writing a bound with no control that goes
   red for it. `stub_body`'s corrected premise ("the predicate refuses every shell
   metacharacter") and `carries_visible_content`'s disclosure ("these four are
   accepted and none can reach an identity") are both *bounds a reader will act
   on*. A one-off measurement quoted in a SUMMARY satisfies "measure" and
   violates prohibition 6 the moment the tree moves — which is the exact way
   `hooks.rs`'s original sentence became false. Both measurements are therefore
   committed tests. The cost is +2 tests in `#[cfg(test)]` modules; the
   `carries_visible_content` one deliberately pins *over-permissive* behaviour and
   its doc says in as many words that it records the residual rather than blessing
   it, and that closing the residual must change the test and the disclosure in
   the same commit.

## Deviations from Plan

### 1. [Rule 3 — Blocking] The bare-field plant needed a `#[cfg(any())]` gate

- **Found during:** Task 1(a).
- **Issue:** the plan says to re-plant a bare private field "in the REAL `DriveArgs` body". `DriveArgs` has **69** construction sites across `src/` and `tests/`; an ungated bare field does not compile, so the integration test binary could not build and the guard could not run at all.
- **Fix:** the plant carries `#[cfg(any())]` (never compiled in). Guard nine is a **textual** scan over source, so the attribute line changes nothing it reads — proved by the guard producing exactly the counts a real thirteenth field would (13 vs 12), which is only possible if the scan saw the field.
- **Disclosed rather than glossed:** the SUMMARY states the gate, the reason, and the evidence that it does not weaken the plant. Both messages were captured under it.
- **Committed in:** the plant was reverted; only the message rewrite is in `0d55523`.

### 2. [Rule 2 — Missing critical] `narrow_visible`'s preceding COMMENT carried the same misdiagnosis

- **Found during:** Task 1(a).
- **Issue:** the plan names the assertion *message* as the subject. The comment block immediately above it said the same wrong thing — "if the widened count ever exceeds it, the extra lines are not field declarations" — and it is the text a reader browsing the file (rather than reading a panic) sees first.
- **Fix:** the comment was rewritten alongside the message, with the same both-causes structure and an explicit note that pass 8 measured the old diagnosis wrong. Leaving it would have been the defect surviving in the artifact written to remove it.
- **Verification:** the deleted-line inspection shows the change is entirely inside comment/doc/message text; no compared expression moved.
- **Committed in:** `0d55523`.

### 3. [Rule 2 — Missing critical] `src/envelope/hooks.rs` gained a test, not only doc lines

- **Found during:** Task 2(a).
- **Issue:** the plan's acceptance criterion says `git diff -- src/envelope/hooks.rs` "touches only doc lines". Prohibition 6 says a bound must have a control. The corrected premise IS a bound.
- **Fix:** the diff touches doc lines **plus** one new test inside the existing `#[cfg(test)] mod tests`. `sh_quote` and the generated stub are provably unchanged (`grep -cE "^[+-].*sh_quote\("` -> `0`), which is what the criterion's second half actually protects.
- **Committed in:** `b38451c`.

### 4. [Rule 2 — Missing critical] The assertion message inside the census also overclaimed

- **Found during:** Task 1(c).
- **Issue:** beyond the two places the plan names, the `assert_eq!` message itself said "a path in the table with a HIGHER count is an adjudicated site that grew another **member of the set**" and listed only one under-detection direction.
- **Fix:** corrected to "another occurrence OF THIS WITNESS", and the "what this scan does NOT check" paragraph now lists all three directions with a note that (b) and (c) were each planted.
- **Committed in:** `0d55523`.

### 5. The tracer feedback gate was satisfied by re-running the verify rather than by halting

- **Found during:** end of Task 1.
- **Issue:** Task 1 is `type="tracer"`, and the executor contract says an interactive run STOPS at a `checkpoint:human-verify` after the tracer.
- **Reason for continuing:** the plan declares `autonomous: true` and contains **zero** `checkpoint:*` tasks; Tasks 2 and 3 are independent workstreams, not expansions of Task 1; and the orchestrator dispatched this executor in a worktree it force-removes on return, so halting would have stranded the work. The gate's substance was performed: the tracer's `<verify>` was re-run end to end and passed (`spawn_seam_guard` 38/38, whole workspace green modulo the documented flake) before any expansion task began.
- **Recorded here rather than silently.**

### Deviations from the plan's stated facts, reported rather than absorbed

| Plan states | Measured | Consequence |
|---|---|---|
| `src/text.rs:155` is the one-spelling claim | The claim is at `:163` on the tree this plan landed on and `:201` after this plan; `21-VERIFICATION.md` recorded `:204-206` at `8dc8c98` | The claim, the file and the work are all right; only the locator is stale — wave 1 moved it, as this plan's own tooling note predicted. Every artifact this plan writes cites it by NAME. |
| `tests/spawn_seam_guard.rs:3646-3656` is the assertion | The `assert_eq!` token is at `:3647` | Off by one at the start of the range; the right assertion. |

---

**Total deviations:** 4 auto-fixed (1 blocking, 3 missing-critical) + 1 protocol
deviation recorded + 2 corrections to the plan's stated locators.
**Impact on plan:** none to its structure. Every mechanism the plan specifies was
built, every plant it requires was performed and captured verbatim, and the red
arm was committed red. Three of the four auto-fixes make the diff *more*
compliant with the plan's own prohibitions than the letter of its acceptance
criteria required.

## Issues Encountered

- **`driver_reattach` fired once, then did not.** During Task 1's whole-suite run under `--test-threads=2` it failed on `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`; `cargo test --test driver_reattach -- --test-threads=1` immediately returned `3 passed; 0 failed`. Both later whole-suite runs (after Tasks 2 and 3) returned **0 failures across all 35 binaries**. This is exactly the shape `deferred-items.md` and `21-21-SUMMARY.md` characterise, and this plan touches no file it could affect.
- No authentication gates. No architectural decisions requiring escalation.

## Known Stubs

None. Every mechanism this plan introduces is a committed test wired to real
production code, and each was observed producing the result its doc claims —
either red (the census, both bare-field plants) or green-under-plant (the two
WR-01 residuals, whose green IS the finding). The residuals named in the docs are
not stubs: they are measured limits of real, shipped scans, each named with its
direction.

## Verification results

Every number below was re-measured under `rtk proxy` against this plan's own final
tree. None is inherited from `21-REVIEW.md`, `21-VERIFICATION.md`,
`21-21-SUMMARY.md` or the plan's text.

| Gate | Result |
|---|---|
| `rtk proxy cargo build` | clean |
| `rtk proxy cargo test --lib` | **1058 passed, 0 failed, 0 ignored** |
| `rtk proxy cargo test --test spawn_seam_guard` | **38 passed, 0 failed** |
| `rtk proxy cargo test --test envelope_advisory` | **9 passed, 0 failed** |
| `rtk proxy cargo test --test driver_escalation_cap` | **8 passed, 0 failed** |
| `rtk proxy cargo test --test driver_injection_corpus` | **13 passed, 0 failed, 10 ignored** (unmodified) |
| `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2` | **35 binaries, 1372 passed, 0 failed, 13 ignored** |
| `rtk proxy cargo clippy --lib -- -D warnings` | clean |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | **4 errors** — the pre-existing lints in `src/browser.rs` and `src/project_creator.rs`, untouched (prohibition 5), now recorded in `deferred-items.md` |

### The four authored edge-probe rows, reconfirmed against this plan's FINAL tree

| Row | Suite | Result |
|---|---|---|
| DRIVE-04 boundary — `--max-escalations` at/above the RESOLVED step cap refused, below it accepted | `tests/driver_escalation_cap.rs` | **8 passed, 0 failed**, file unmodified |
| DRIVE-04 precision — the decomposition consultation counts against the same cap, no path exempt | `tests/driver_escalation_cap.rs` (same run) | **8 passed, 0 failed** |
| SAFE-07 boundary — arrival asserted before influence | `tests/driver_injection_corpus.rs` | **13 passed, 0 failed, 10 ignored.** STRUCTURAL half only. **The behavioural half remains UNVERIFIED and is stated as such, not certified** — the ten ignored tests need an authenticated Claude subscription and no agent can run them. |
| SAFE-07 precision — only the enumerated strings cross, never a whole file | `tests/spawn_seam_guard.rs#the_arrival_evidence_field_is_named_only_where_the_schema_declares_it` | **1 passed, 0 failed** |

## Next Phase Readiness

- The five sentences that could have caused a correct mechanism to be deleted or a correct fix to be reverted are now true, and each is backed by a control that goes red rather than by prose.
- The tree carries ONE spelling of the identity alphabet, and a census keeps it that way. Its own residual — a third, differently-constructed spelling — is named and attributed to the delegation rather than to the scan.
- Round 9 inherits, unchanged: criterion 4's behavioural half (permanently agent-unclosable); the three flagged edge-probe assumptions SAFE-08, DRIVE-01, DRIVE-03 that 21-21 carried forward and this plan did not auto-resolve; 21-21's residual 1 (unexercised populated-cache render branches) and residual 5 (a fixture in the wrong state passes by silence); TR39 confusables in free text; and the four pre-existing clippy lints, now recorded with their locations.
- New standing obligation for whoever upgrades ratatui: re-measure which invisible characters reach a terminal cell, and re-check that the render probe's assertions are still the non-vacuous ones.
- `.planning/REQUIREMENTS.md` is untouched; requirement status remains for a passed verification to decide.
- STATE.md and ROADMAP.md are deliberately not modified — this executor ran in a worktree and the orchestrator owns those writes.

## Self-Check: PASSED

Files this SUMMARY claims were modified, checked on disk:

```
$ rtk proxy git diff abcb367..HEAD --stat
 src/envelope/advisory.rs  |  27 +++++-
 src/envelope/hooks.rs     |  84 ++++++++++++++++-
 src/text.rs               | 236 ++++++++++++++++++++++++++++++++++++++++++++++
 tests/spawn_seam_guard.rs | 143 +++++++++++++++++++++++-----
 4 files changed, 460 insertions(+), 30 deletions(-)
```

(plus `deferred-items.md`, +135/-0, in `59df5a3`; 5 files and 595/30 in total.)

Commits this SUMMARY claims, checked in `git log`:

```
$ rtk proxy git log --format='%h %s' abcb367..HEAD
59df5a3 docs(21-22): the register — the pre-existing lints, the ratatui obligation, criterion 4 re-surfaced
b38451c fix(21-22): one alphabet spelling, a true security rationale, and the free-text residual disclosed
bce8e89 test(21-22): the alphabet spelling census, observed RED at TWO spellings
0d55523 docs(21-22): the message that would revert round 7, and two residuals measured by planting
```

All four task commits present, in the stated order, with the red arm (`bce8e89`)
preceding its fix (`b38451c`).

Prohibition-8 scanner re-run over every file this plan writes **and over this
document itself**, because 21-21 caught a violation of exactly this prohibition
inside the artifact auditing it:

```
tests/spawn_seam_guard.rs: 0 raw invisible/bidi/tag/VS characters
src/text.rs: 0 raw invisible/bidi/tag/VS characters
src/envelope/advisory.rs: 0 raw invisible/bidi/tag/VS characters
src/envelope/hooks.rs: 0 raw invisible/bidi/tag/VS characters
deferred-items.md: 0 raw invisible/bidi/tag/VS characters
21-22-SUMMARY.md: 0 raw invisible/bidi/tag/VS characters
TOTAL: 0
```

Working tree clean of every plant: `rtk proxy git status --porcelain` names no
file under `src/driver/` and no untracked probe under `tests/`.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-25*
