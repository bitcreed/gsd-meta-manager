---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 26
subsystem: ui-render-honesty
status: complete
tags: [CR-05, WR-05, IN-01, sealed-supertrait, source-census, record]
wave: 3
gap_closure: true
requires: ["21-23", "21-24", "21-25"]
provides:
  - "ui::screens::RenderAdjudicated — a sealed, object-safe supertrait of Screen, so an unadjudicated screen is error[E0277] rather than a silent census miss"
  - "ui::screens::adjudicate_screen! — the only route to an adjudication, keeping the disposition vocabulary to two constants"
  - "the two stable snake_case disposition constants, promoted from render_escape_guard to production vocabulary"
  - "a screen census whose floor is raised: joined logical lines, reported unnameable impls, (name, path) keys"
  - "an alphabet census that matches a normalized line rather than one exact byte string"
  - "the round-9 record in deferred-items.md"
affects:
  - "every present and future implementor of ui::screens::Screen, whatever its source formatting"
  - "downstream consumers of the crate: Screen is now un-implementable outside it (semver-breaking, deliberate)"
tech-stack:
  added: []
  patterns:
    - "sealed supertrait with pub(crate) mod sealed — a compile-time obligation where a source scan was formatting-bounded"
    - "object-safe adjudication via &'static str METHODS, never associated consts, because the whole UI is Box<dyn Screen>"
    - "normal-form matching (sorted bare char-literal atoms) instead of one exact byte string"
key-files:
  created: []
  modified:
    - src/ui/screens/mod.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/screens/add_project.rs
    - src/ui/screens/create_project.rs
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/driver_inject.rs
    - src/ui/screens/driver_start.rs
    - src/ui/screens/enqueue.rs
    - src/ui/screens/help.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/queue_delete_confirm.rs
    - src/text.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
key-decisions:
  - "D-21-25: CR-05 closed with a sealed supertrait on Screen, not a better source scan — a scan's completeness is bounded by source FORMATTING, which is the level CR-05 found; a trait bound has no spelling to be short of. COSTLY: semver-breaking for downstream implementors."
  - "D-21-26: RenderAdjudicated uses METHODS returning &'static str, never associated consts — an associated const makes Screen dyn-incompatible and the whole UI is Box<dyn Screen>."
  - "D-21-27: the adjudication is SEALED and reachable only through adjudicate_screen!. The seal is pub(crate) rather than fully private, because a private mod sealed is reachable only from ui::screens and its descendants — MEASURED as error[E0603] against a plant in src/driver/liveness.rs."
  - "D-21-28: the disposition is PROMOTED onto the screen; SCREEN_IDENTITY_DISPOSITIONS is DEMOTED to a (type name, path) fixture map with its narrowed job documented in the same commit."
  - "D-21-29: the source walk is KEPT with a narrowed job and a raised floor, not deleted."
  - "D-21-30: WR-05 closed by NORMALIZING the needle, not by softening the residual paragraph — and the residual paragraph corrected anyway."
  - "D-21-31: IN-02 and IN-03 RECORDED in deferred-items.md, not fixed."
requirements-completed: [SAFE-07, SAFE-08, DRIVE-04]
duration: "~2h"
completed: 2026-08-27
actuals:
  tokens: 128000
  tasks: 3
  commits: 3
coverage:
  - deliverable: "An unadjudicated Screen implementation does not compile"
    human_judgment: false
    verification:
      - kind: compile-error
        ref: "planted impl Screen for TwelfthScreenNobodyAdjudicated in src/driver/liveness.rs -> error[E0277]"
        status: pass
      - kind: test
        ref: "cargo build (11 adjudicated implementors) exit 0"
        status: pass
  - deliverable: "Both of the round-8 reviewer's plants are covered by that one bound"
    human_judgment: false
    verification:
      - kind: compile-error
        ref: "macro_rules!-generated implementor WITH adjudication compiles ('and 4 others'); WITHOUT it is E0277"
        status: pass
      - kind: test
        ref: "src/ui/screens/render_escape_guard.rs#a_wrapped_impl_header_is_one_logical_unit"
        status: pass
  - deliverable: "Screen remains object-safe and the probe reads disposition() off the instance"
    human_judgment: false
    verification:
      - kind: test
        ref: "Vec<Box<dyn Screen>> built and both supertrait methods called through &dyn Screen (planted, printed, reverted)"
        status: pass
      - kind: test
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped"
        status: pass
  - deliverable: "The screen census's floor is raised against three planted defects"
    human_judgment: false
    verification:
      - kind: test
        ref: "src/ui/screens/render_escape_guard.rs#the_census_reports_an_unadjudicated_screen_and_a_stale_row (4 directions + clean)"
        status: pass
      - kind: test
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_census_matches_the_tree"
        status: pass
  - deliverable: "The alphabet needle survives a reorder, a re-spacing and a wrap"
    human_judgment: false
    verification:
      - kind: test
        ref: "src/text.rs#the_alphabet_needle_survives_a_reorder_and_a_respacing"
        status: pass
      - kind: test
        ref: "src/text.rs#the_normalized_needle_still_excludes_the_branch_name_set"
        status: pass
      - kind: test
        ref: "src/text.rs#exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src"
        status: pass
  - deliverable: "DRIVE-04's boundary and precision, reconfirmed by re-run"
    human_judgment: false
    verification:
      - kind: test
        ref: "tests/driver_escalation_cap.rs (8 passed / 0 failed, both cap directions)"
        status: pass
  - deliverable: "deferred-items.md carries round 9's unmechanisable knowledge"
    human_judgment: true
    rationale: "A record's value is whether a future reader can act on it, which no test asserts. Its checkable properties — append-only, every count traced to a SUMMARY, criterion 4 quoted verbatim — are verified below, but its adequacy as a record is a human judgment."
  - deliverable: "ROADMAP criterion 4's behavioural half"
    human_judgment: true
    rationale: "PERMANENTLY AGENT-UNCLOSABLE. The ten driver_injection_corpus arms spawn the real claude binary and need an authenticated subscription. Round 9 claimed no work against it and edited neither test file; presence and wiring verified for the tenth consecutive pass, behaviour never."
---

# Phase 21 Plan 26: The Two Mechanism Defects, and the Record — Summary

CR-05 is closed by making an unadjudicated screen **fail to build** — a sealed
`RenderAdjudicated` supertrait on `ui::screens::Screen` — rather than by a better
source scan; WR-05 is closed by normalizing the alphabet census's needle so arm
order and spacing stop mattering; and round 9's unmechanisable knowledge is
recorded, including the six carrier types the round did not retype and the one
criterion no agent can ever close.

**Duration:** ~2h · **Tasks:** 3/3 · **Commits:** 3 · **Files modified:** 15

| Task | Commit | What |
|---|---|---|
| 1 | `d28e408` | The sealed supertrait; the disposition promoted onto the screen; the table demoted to a fixture map |
| 2 | `08605b3` | Both censuses' floors raised — joined lines, reported unnameables, `(name, path)` keys, a normalized needle |
| 3 | `58b2de6` | The round-9 record in `deferred-items.md` |

---

## Task 1 — an unadjudicated screen fails to BUILD

### The mechanism

`ui::screens::Screen` now reads `pub trait Screen: RenderAdjudicated`.
`RenderAdjudicated` is sealed above a `pub(crate) mod sealed` and carries two
object-safe **methods** — `disposition(&self) -> &'static str` and
`adjudication_reason(&self) -> &'static str`. `adjudicate_screen!` emits both the
`Sealed` impl and the `RenderAdjudicated` impl and is the only route to an
adjudication.

**Why a trait bound and not a better regex.** The round-8 review proposed joining
lines and reporting unnameable `impl`s. That narrows the residual and it is worth
doing — it is Task 2 — but it leaves the deny-by-default property resting on
**source formatting**, which is one level below where anybody was looking and is
exactly the level CR-05 found. A trait bound is a property of the TYPE, so there
is no spelling for it to be short of.

### The RED, verbatim

Observed by planting a throwaway implementor in `src/driver/liveness.rs` — a file
two directory levels down, with nothing to do with the UI, and one this plan does
not otherwise touch:

```text
error[E0277]: the trait bound `TwelfthScreenNobodyAdjudicated: RenderAdjudicated` is not satisfied
   --> src/driver/liveness.rs:274:37
    |
274 | impl crate::ui::screens::Screen for TwelfthScreenNobodyAdjudicated {
    |                                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ unsatisfied trait bound
    |
help: the trait `RenderAdjudicated` is not implemented for `TwelfthScreenNobodyAdjudicated`
   --> src/driver/liveness.rs:272:1
    |
272 | pub struct TwelfthScreenNobodyAdjudicated;
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    = help: the following other types implement trait `RenderAdjudicated`:
              AddProjectScreen
              CreateProjectScreen
              DeleteConfirmScreen
              DetailScreen
              DriverConfirmScreen
              DriverInjectScreen
              DriverStartScreen
              EnqueueScreen
            and 3 others
note: required by a bound in `Screen`
   --> src/ui/screens/mod.rs:179:19
    |
179 | pub trait Screen: RenderAdjudicated {
    |                   ^^^^^^^^^^^^^^^^^ required by this bound in `Screen`
```

**The red is a compile error rather than a failing test, and that is stronger.** A
failing test can be filtered out, marked `#[ignore]`, or simply not run. A build
that does not build cannot ship.

### Both of the round-8 reviewer's plants, covered by one bound

The second plant is a `macro_rules!`-generated implementor whose expansion
includes `adjudicate_screen!`. It **compiles**, in the same file, at the same
time. The evidence is the compiler's own list in the error above: with both
plants present it read `… and 4 others` where with only the unadjudicated one it
read `… and 3 others`. The macro-generated screen is a `RenderAdjudicated`
implementor; the unadjudicated one is not, and only it is E0277.

The third spelling — a **wrapped `impl` header** — is not a text pattern at all
under a type bound, so it needs no separate demonstration here. (It gets one
anyway in Task 2, against the walk.)

Both plants removed; `rtk proxy git status --porcelain` clean afterwards, naming
only this plan's own files.

### Object safety, proven at runtime rather than argued

A `Vec<Box<dyn Screen>>` was built, an element taken as `&dyn Screen`, and both
supertrait methods called through it:

```text
disposition        = "renders_no_attacker_influenced_identity"
adjudication_reason = "A macro-generated plant. Draws nothing at all."
```

The permanent form of that call is in the probe:

```rust
let screen_disposition: &'static str = {
    let as_dyn: &dyn Screen = clean_states[0].screen.as_ref();
    as_dyn.disposition()
};
```

Every one of the 1387 tests passes, which includes every screen transition in the
tree.

### The one number this plan corrected by measuring

The standing brief and the round-8 review both speak of **13** `Screen` impls. At
HEAD, `rtk proxy grep -rn "impl.*Screen for " src/ --include=*.rs` returns
**ELEVEN**:

```
src/ui/screens/driver_inject.rs:122      src/ui/screens/driver_start.rs:193
src/ui/screens/add_project.rs:31         src/ui/screens/normal.rs:374
src/ui/screens/enqueue.rs:20             src/ui/screens/help.rs:274
src/ui/screens/queue_delete_confirm.rs:26  src/ui/screens/delete_confirm.rs:21
src/ui/screens/create_project.rs:30      src/ui/screens/driver_confirm.rs:336
src/ui/screens/detail.rs:787
```

The thirteen was **eleven real implementors plus the round-8 reviewer's own two
plants**, counted while the plants were in the tree. A plan that inherited it
would have spent the round hunting a twelfth and thirteenth screen that do not
exist. (`detail.rs` is at `:787`, not the plan's `:720` — 21-25's edits shifted
it.)

### The seal is `pub(crate)`, and that was measured too

The first draft used a fully private `mod sealed`. The eleven real screens are
descendants of `ui::screens`, so they compiled; the macro-generated plant in
`src/driver/liveness.rs` did not:

```text
error[E0603]: module `sealed` is private
   --> src/ui/screens/mod.rs:154:35
    |
154 |         impl $crate::ui::screens::sealed::Sealed for $type {}
    |                                   ^^^^^^  ------ trait `Sealed` is not publicly re-exported
```

A `Screen` may be added in **any** module of this crate — that is what the census
walking all of `src/` is for — so a seal that only sealed the directory today's
screens happen to live in would push the next screen outside the mechanism rather
than into it. `pub(crate)` keeps the downstream property exactly intact: nothing
outside this crate can name it, so nothing outside this crate can declare its own
screen adjudicated.

### The promote, and the demote

Each screen now carries its adjudication beside its `impl Screen`, with the reason
text moved **verbatim** from `SCREEN_IDENTITY_DISPOSITIONS`'s fourth column —
including the "Fixture states: …" sentences, which are part of what a reader
inherits.

`SCREEN_IDENTITY_DISPOSITIONS` is demoted to a `(type name, path)` fixture map.
Its doc states the narrowed job **in the same commit that narrows it**, quoting
the wording it replaces. What it still answers is a different question from what
makes adjudication mandatory: *which adjudicated screens have a probe fixture*.
Prohibition 4 forbade deleting it and the reason holds — deleting it would trade a
bounded residual for an unbounded one.

The probe reads `screen.disposition()` off the **constructed instance**, so the
disposition it checks and the disposition the screen declares cannot drift. The
both-ways set equality could only ever catch that for *membership*; **content was
never checked** until the disposition became one statement instead of two.

### The module's headline sentence, corrected with the falsified text quoted

It claimed a filesystem walk discovers a screen added tomorrow *"without anybody
editing anything here"*. That is quoted verbatim in the correction block and
replaced with what is true after this task, and LIMIT 6 was added stating the
walk's residual **as it stood at that commit** rather than as Task 2 intended it
to be.

### Measurements

| Check | Result |
|---|---|
| `rtk proxy grep -rn "adjudicate_screen!" src/ui/screens/` | **11**, one per implementor, one per file |
| `rtk proxy grep -rn "impl.*RenderAdjudicated for" src/` | **1** — the macro's own definition site. No hand-written implementation exists. |
| `rtk proxy grep -c "renders_attacker_influenced_identity\|renders_no_attacker_influenced_identity" src/ui/screens/mod.rs` | **2** — the vocabulary is exactly two constants, and the probe still `panic!`s on any third value |
| Test totals after Task 1 | 1384 passed / 0 failed / 13 ignored — **identical to the base** |
| `git diff --stat` | names neither `src/browser.rs` nor `src/project_creator.rs` |

---

## Task 2 — both censuses' floors raised, each against a planted defect

### The screen census

Three raises, three plants, each in `src/driver/liveness.rs` and each with its
BEFORE captured by temporarily restoring the old behaviour for one run.

**(1) `UNNAMEABLE IMPLEMENTATION`** — an `impl` whose type name the scan cannot
extract used to be a silent `continue`. Planted a `macro_rules!`-generated
adjudicated implementor.

```text
BEFORE (silent `continue` restored):
  test ui::screens::render_escape_guard::tests::the_screen_census_matches_the_tree ... ok

AFTER:
  UNNAMEABLE IMPLEMENTATION: an implementation of the trait at
  src/driver/liveness.rs:282 has a type name this scan cannot extract — it is
  macro-generated, or carries a generic parameter, or is spelled in some way the
  extractor does not handle. It COMPILES, so it is adjudicated: the sealed
  supertrait saw to that. What nobody has checked is whether it has a probe
  fixture, which is this walk's whole remaining job.
```

**The difference from Task 1, stated because it is the point:** that screen
**compiled**, because Task 1's bound was satisfied. The walk is reporting that it
has no probe fixture, which is the walk's *narrowed* job. The two mechanisms'
jobs are now visibly different, and the offence message says which one failed.

**(2) `(name, path)` keys (IN-01)** — planted a second `HelpScreen` in a
different file.

```text
BEFORE (bare-name key emulated for one run):
  test … the_screen_census_matches_the_tree ... ok
  ← COLLAPSED. Two implementors in the tree, one silently unchecked, census green.

AFTER:
  RELOCATED IMPLEMENTOR: `HelpScreen` has a fixture row at
  ["src/ui/screens/help.rs"] but the walk found an implementation at
  src/driver/liveness.rs. … The census keys on (type name, path) precisely so the
  second case cannot collapse into the first and go unchecked (IN-01)
```

**(3) Joined logical lines** — planted an `impl` header wrapped across two lines.

```text
BEFORE (join disabled for one run):
  test … the_screen_census_matches_the_tree ... ok
  ← a twelfth implementor in the tree, reported green

AFTER:
  UNFIXTURED IMPLEMENTOR: `WrappedHeaderScreen` (in src/driver/liveness.rs)
  implements the trait but no row gives it a probe fixture …
```

**`UNADJUDICATED IMPLEMENTOR` is renamed to `UNFIXTURED IMPLEMENTOR`**, and the
message itself says so and why: since Task 1 an unadjudicated screen does not
compile at all, so that heading could no longer be about adjudication without
lying to the reader about which mechanism failed.

`the_census_reports_an_unadjudicated_screen_and_a_stale_row` now drives four
synthetic directions (unfixtured, relocated-same-name-twin, stale, unnameable)
plus the clean direction, and still drives the **same** `census_offences` the live
assertion consumes — `rtk proxy grep -n "census_offences"` shows one definition at
`:491` and three callers (`:1689`, `:1739`, `:1882`).
`a_wrapped_impl_header_is_one_logical_unit` drives `join_logical_impl_header` with
the header split at each of the three places it can wrap, and pins the join's
four-line bound so one unclosed brace cannot make the census read a whole file as
one header.

### The alphabet census (WR-05)

The census matched **one exact byte string**, so its power depended on the order
the author happened to type the arms in. It now normalises each line — whitespace
stripped, bare character literals collected, sorted, deduplicated, `..=` range
endpoints excluded — and compares for **equality** against the same normalisation
of the identity alphabet's own clause.

Planted a reordered copy, `matches!(c, '-' | '_' | '.')`:

```text
OLD NEEDLE — rtk proxy grep -rn "'.' | '_' | '-')" src/ --include=*.rs, plant present:
  src/envelope/advisory.rs:566   ← a doc comment, which the census skips
  src/text.rs:251                ← the one real site
  …the plant is NOT named. The census would have reported ONE and stayed green
  with a second boundary in the tree.

NEW NEEDLE:
  … spelled 2 times in executable lines under src/ …
  Sites: ["src/driver/liveness.rs:275", "src/text.rs:251"]
```

The same plant re-spelled so one arm ends a line is also counted, **exactly once**
(`Sites: ["src/driver/liveness.rs:276", "src/text.rs:251"]`) — clauses are joined
into logical units first, and the subset-gated join is what stops double-counting.

### The self-match that joining introduced, and how it was found

Joining **broke the anti-self-match property**, and that was measured rather than
reasoned about. With an unconditional join the census reported TWO sites,
`src/text.rs:251` and `src/text.rs:1086`, because `ALPHABET_CLAUSE_HEAD`'s own line
(atoms `['.']`) joined onto `ALPHABET_CLAUSE_TAIL`'s (atoms `['-', '_']`) and the
union is exactly the clause. **The two halves are meaningless apart as strings;
they were not meaningless apart as adjacent source lines.**

`continues_onto_the_next_line` is the fix: a join requires the line to be
syntactically unfinished — an open `|` or an unclosed paren — and a `const … ;` is
neither. The new control's own fixture strings hit the same rock (eight sites
reported where one exists) and are assembled at runtime from one-atom fragments
for exactly the same reason. Both incidents are recorded in the code's doc,
because the shape is going to recur.

### The exclusion, asserted rather than assumed

Normalising loosens the needle, so **over-matching is the new failure direction**.
`the_normalized_needle_still_excludes_the_branch_name_set` pins that
`advisory::default_branch_of`'s set — which admits `'/'` — normalises to four
atoms rather than three, and additionally runs the live census and requires that
no line of `envelope/advisory.rs` is counted.

`exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src` still
asserts an **equality on a count**, still names `src/text.rs` as the one surviving
site, and its committed RED-at-two block is **unchanged**.

### The residual paragraph, corrected

`src/text.rs`'s under-detection paragraph named only NON-textual constructions.
The census's real blind spot was narrower and worse: **five of the six orderings
of a textual copy.** The falsified paragraph is quoted verbatim beside the
correction, and what remains is named with its direction: (1) a genuinely
different construction, and (2) a line that spells the clause *and* an unrelated
character literal beside it, which normalises to a larger set. Both
under-detection, silent; both bounded by the delegation rather than by this
census.

---

## Task 3 — the record

`deferred-items.md` gains 263 lines and loses none —
`rtk proxy git diff -- …/deferred-items.md | grep -c "^-[^-]"` is **0**.

* **The six carrier types round 9 did NOT retype**, one row each with the
  measurement that excluded it, the SUMMARY it came from, and its failure
  direction. Where a re-measurement disagrees with the SUMMARY, BOTH are given:
  `21-23-SUMMARY.md:289` reports 135 mentions for `ProjectState` where `.status`
  alone now measures **175** (different things counted, both recorded), and 93 for
  `filtered_aliases`/`selected_alias` where HEAD measures **94** — round 9's own
  diff added one.
* **The per-widget ratatui obligation** as a standing item in its own right,
  pointing at 21-23's table row rather than duplicating it, and stating explicitly
  that the previous generalisation was wrong in one direction and that a future
  upgrade must be measured **per widget family**.
* **IN-04's process lesson**, quoted verbatim, plus round 9's adopted rule and the
  two places it paid inside this plan alone.
* **IN-02 and IN-03 deferred** with their reasons and with what would promote
  each.
* **ROADMAP criterion 4 re-surfaced unchanged**, command/expected/why-human quoted
  verbatim, status MEASURED: diff-stat since `dfa11c6` empty,
  `13 passed / 0 failed / 10 ignored`.
* **DRIVE-04's two backstops re-run** — `8 passed / 0 failed`, both cap directions,
  the arms named, framed as a reconfirmation rather than new evidence.
* **The round-9 self-audit** with its controls and its scope stated exactly.
* **One item recorded as OPEN**: pass 9's four-character sighting.

---

## The round-9 self-audit

Scope: `git diff 98610bf..08605b3`, **6244 added lines** — round 9's four plans
plus the orchestrator's `Display for Rendered` fix and both tracking commits.
Task 3's own commit is excluded from the debt-marker half by construction and the
entry says so; the `Cf` half IS extended to it.

| Check | Result | The control that makes it non-vacuous |
|---|---|---|
| Raw `General_Category=Cf` in added lines | **0** | Same scan, one `U+200B` appended: returns **1**. Delta `+1`. |
| `Cf` in Task 3's own 256 added lines | **0** | Same control, same `+1` delta. |
| `TBD`/`FIXME`/`XXX` | **0** (3 raw hits, all false positives) | Control needle `the` returns **1819**. The three are the literal `U+XXXX` escape-marker format string. |
| `TODO`/`HACK`/`PLACEHOLDER` | **0** (3 raw hits, all false positives) | Same control. The three are the SUMMARY template's own "no … TODO … was introduced" sentence, once per wave SUMMARY. |
| `.planning/REQUIREMENTS.md` last touched | `0c4f712` | Seventh consecutive round untouched. |

The false positives are **reported rather than filtered away**. A grep tuned until
it returns zero is a grep that has been taught not to look.

---

## Verification

| Gate | Result |
|---|---|
| `rtk proxy cargo build` | exit 0 |
| `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2` | **1387 passed / 0 failed / 13 ignored** (base 1384; +3 = the three new controls) |
| `rtk proxy cargo clippy -- -D warnings` | exit 0 |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | exactly **four** pre-existing lints, same two kinds, same two files — `bool_assert_comparison` ×3 at `src/browser.rs:155,156,157`, `cmp_owned` ×1 at `src/project_creator.rs:146`. Line numbers shifted across the round's waves exactly as the standing constraint predicted; kinds and files unchanged. |
| `rtk proxy cargo test --test driver_escalation_cap` | 8 passed / 0 failed |
| `rtk proxy cargo test --test driver_injection_corpus` | 13 passed / 0 failed / 10 ignored |
| `git diff --name-only 12d7549..HEAD` | names none of `STATE.md`, `ROADMAP.md`, `REQUIREMENTS.md`, `browser.rs`, `project_creator.rs`, `spawn_seam_guard.rs`, `driver_injection_corpus.rs`, `driver_escalation_cap.rs`, `roadmap_widget.rs` |

**Five plants, each with verbatim output and a clean tree after each:** the
unadjudicated screen (compile error), the macro-generated adjudicated screen
(compiles; walk reports the missing fixture), two same-named screens, a wrapped
`impl` header, a reordered alphabet copy (and its wrapped variant).

---

## Deviations from Plan

### [Rule 3 - Blocking] The seal had to be `pub(crate)`, not fully private

- **Found during:** Task 1, step (e), while planting the macro-generated screen.
- **Issue:** The plan specifies "a crate-private sealing module". Implemented as a
  fully private `mod sealed`, the macro is only invocable from `ui::screens` and
  its descendants — `error[E0603]: module 'sealed' is private` at the plant in
  `src/driver/liveness.rs`. The eleven real screens compiled, which is exactly how
  this would have gone unnoticed until the twelfth screen was added elsewhere.
- **Fix:** `pub(crate) mod sealed`. The downstream property — the whole point of
  the seal — is unchanged: nothing outside this crate can name it. The doc records
  the measurement and the reasoning.
- **Files modified:** `src/ui/screens/mod.rs`
- **Commit:** `d28e408`

### [Rule 2 - Missing critical] `continues_onto_the_next_line` for the alphabet join

- **Found during:** Task 2, step (e).
- **Issue:** The plan asks for line joining and for the anti-self-match property to
  be preserved, and did not anticipate that joining DESTROYS it: with an
  unconditional join the two runtime-assembled needle halves are adjacent source
  lines whose union is the clause, and the census reported itself.
- **Fix:** A join now requires the line to be syntactically unfinished. The
  incident is recorded verbatim in the helper's doc, because the same shape hit
  the new control's own fixtures immediately afterwards.
- **Files modified:** `src/text.rs`
- **Commit:** `08605b3`

### [Rule 2 - Honesty] `UNADJUDICATED IMPLEMENTOR` renamed to `UNFIXTURED IMPLEMENTOR`

- **Found during:** Task 2, step (c).
- **Issue:** The plan says to keep the three offence messages distinct. Keeping the
  *heading* would have left the census telling readers a screen was
  "unadjudicated" when since Task 1 an unadjudicated screen cannot compile — the
  message would send the reader to fix the wrong mechanism. Prohibition 9 forbids
  a doc claiming what is not true.
- **Fix:** Renamed, with the rename and its reason stated inside the message
  itself. Distinctness — which is what the plan actually required — is preserved
  across all four offences.
- **Files modified:** `src/ui/screens/render_escape_guard.rs`
- **Commit:** `08605b3`

**Total deviations:** 3 auto-fixed (1 × Rule 3, 2 × Rule 2). **Impact:** none
adverse. Each was discovered by a plant the plan itself required, which is the
mechanism working.

---

## Known Stubs

None. No hardcoded empty value, placeholder string, TODO or unwired component was
introduced.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or schema change at
a trust boundary. No `cargo add`, no `Cargo.toml` change — the `## Package
Legitimacy Audit` gate did not fire, as the plan's threat model predicted.

---

## What remains genuinely open at the end of round 9

Stated plainly, because this plan is the round's closing record and prohibition 9
forbids claiming a bound no committed control goes red for.

1. **ROADMAP criterion 4's behavioural half — permanently agent-unclosable.** The
   ten `#[ignore]`d `driver_injection_corpus` arms spawn the real `claude` binary
   and need an authenticated subscription. Round 9 claimed no work against it and
   edited neither test file. Presence and wiring verified for the tenth
   consecutive pass; behaviour never. **A human with a live Claude subscription
   must run `cargo test --test driver_injection_corpus -- --ignored --nocapture`
   and record the CLI version beside the result.**
2. **The six unretyped carrier types.** Under-protection, silent. Bounded by a
   census plus a SAMPLING probe. 21-25 made the sampling materially better; that
   is an improvement in the bound, not a closure of it.
3. **Verification pass 9's four-character sighting is still unexplained.** 21-23's
   populated fixture reports EIGHT and fires 20/20, so the CLASS is settled by
   construction — but the exact count is not reproduced. Recorded as an open
   discrepancy, not a resolved one.
4. **The walk's remaining residual (LIMIT 6).** An implementation whose source
   carries no `impl` token the walk recognises, or whose header wraps across more
   than four physical lines, is still invisible to the FIXTURE-COVERAGE check.
   Such a screen still cannot ship unadjudicated — the supertrait — and if any
   fixture renders it the probe's assertions still apply. Under-detection, silent.
5. **The probe's fixture residual (LIMIT 1).** States no fixture constructs: the
   Defaults tab's string-edit overlay and the Driver tab's `driver_dry_run`
   preview are the two named examples. Under-detection, silent.
6. **The alphabet census's two blind spots.** A non-textual construction, and a
   line spelling the clause beside an unrelated character literal. Both
   under-detection, silent; both bounded by the delegation rather than by the
   census.
7. **IN-02 and IN-03**, deferred with reasons and with what would promote each.
8. **The four pre-existing `--all-targets` clippy lints** in `src/browser.rs` and
   `src/project_creator.rs`. Untouched by design; they predate this phase.

## Self-Check: PASSED

- All 15 modified files exist on disk and are tracked.
- All three commits exist: `d28e408`, `08605b3`, `58b2de6`.
- All plan `<acceptance_criteria>` re-run and passing; all plan
  `<verification>` commands re-run at final HEAD.
- `STATE.md`, `ROADMAP.md` and `REQUIREMENTS.md` completion state untouched, per
  the wave contract and prohibition 12.
