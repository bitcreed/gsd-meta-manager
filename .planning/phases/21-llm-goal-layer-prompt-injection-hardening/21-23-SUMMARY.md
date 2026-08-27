---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 23
subsystem: security
tags: [unicode, trojan-source, ratatui, newtype, render-escaping, autoref-specialization]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "text::display_identity and is_invisible_formatting_char (the derived invisible class); registry::LegacyRegistryKey (the carrier SHAPE, applied to one call site); render_escape_guard's Screen census and behavioural probe"
provides:
  - "crate::text::Untrusted — the general carrier for a string this build did not author: no Display, AsRef<str>, Deref, Borrow<str>, Into<Cow<'_, str>> and no derived Debug"
  - "crate::text::Rendered — the only string type in the tree that is both escaped and Into<Cow<'static, str>>, making the escaped path the ergonomic one"
  - "crate::text::render_for_terminal — the ONE composition of the control class and the invisible-formatting class (WR-01, WR-02)"
  - "crate::text::strip_terminal_controls — the ONE production spelling of the ESC/C0/DEL/C1 control class, moved out of ui::screens and separated from the display cap"
  - "GitLogEntry's four fields retyped to Untrusted, wrapped at the one producer; six render/logic consumers named by the COMPILER"
  - "the raw-absence probe assertion LIMIT 4 declined, reinstated per state and observed red"
  - "a rendered-buffer survivorship precondition, so assertion 3's teeth are a property of the fixture rather than of an index"
  - "a runtime autoref-specialization control certifying Untrusted's three absent conversions, observed red twice by planting"
  - "the per-widget ratatui correction: the zero-width drop is a Paragraph property, NOT a Buffer property"
affects: [21-24, 21-25, 21-26, verification-pass-10]

actuals:
  tokens: 20826
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Carrier newtype with two purpose-named accessors (as_raw_for_logic_only / shown), generalising LegacyRegistryKey"
    - "Ergonomic asymmetry as a security mechanism: the escaped type carries the std conversions, the raw carrier carries none"
    - "Autoref specialization as a runtime control for an ABSENT trait impl"
    - "Survivorship-by-rendering as a fixture precondition instead of an index pin"

key-files:
  created: []
  modified:
    - src/text.rs
    - src/ui/screens/mod.rs
    - src/state_reader/git_ops.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/render_escape_guard.rs
    - src/registry.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md

key-decisions:
  - "D-21-7: the inversion lives in the CARRIER types and their accessors, never in a ban on a ratatui API — Span::raw takes Into<Cow<'_, str>> and String: Into<Cow> is a std impl, so no global forbid exists and none is claimed"
  - "D-21-8: Untrusted withholds Display/AsRef<str>/Deref/Borrow<str>/Into<Cow>; Rendered HAS Into<Cow<'static, str>>, so the escaped path is the short one"
  - "D-21-9: strip_terminal_controls moves into text.rs and sanitize_render_line becomes it plus the display cap — the round-8 review's proposed one-liner would have capped every CLI echo at 512 chars"
  - "D-21-10 (costly): GitLogEntry is the tracer's carrier; its pub fields make this a semver-breaking public-API change landing in the next minor"
  - "D-21-11: the raw-absence assertion is reinstated, not merely reworded — LIMIT 4's premise held only for Paragraph"
  - "D-21-12: the probe's teeth are asserted by RENDERING, not by pinning TAG_PAIR == 4"

patterns-established:
  - "Retype the carrier, then resolve ONLY the sites the compiler names, writing one line at each saying which question it answers"
  - "Correct a falsified premise in the same commit as the measurement that falsifies it, append-only, quoting the sentence corrected"
  - "Certify a negative type property with a both-directions runtime control observed red by planting, never with a comment quoting a compile error"

requirements-completed: [SAFE-07, SAFE-08, DRIVE-01]

coverage:
  - id: D1
    description: "An unescaped GitLogEntry field cannot reach a Span: the four fields are text::Untrusted, wrapped at the one producer, and every consumer was named by the compiler"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped"
        status: pass
      - kind: unit
        ref: "src/text.rs#an_untrusted_carrier_implements_none_of_the_string_conversions"
        status: pass
    human_judgment: false
  - id: D2
    description: "CR-04 (the GitHistory tab's live Trojan Source leak through List/ListItem) fires 20/20 before the fix and 0/20 after it"
    requirement: SAFE-07
    verification:
      - kind: integration
        ref: "20 runs of target/debug/deps/gsd_meta_manager-* --exact ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped, before (20 failed) and after (0 failed)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The two-class question (WR-01/WR-02) is resolved once, in render_for_terminal, and sanitize_render_line is behaviour-preservingly redefined as the class plus the cap"
    requirement: SAFE-08
    verification:
      - kind: unit
        ref: "src/ui/screens/mod.rs#the_capped_and_uncapped_compositions_agree_below_the_cap"
        status: pass
      - kind: other
        ref: "git diff -U0 f6dc06e -- src/ui/screens/mod.rs: zero hunks inside any pre-existing #[test] fn"
        status: pass
    human_judgment: false
  - id: D4
    description: "Assertion 3's teeth are a property of the fixture's rendered behaviour, not of TAG_PAIR's value; the probe refuses to run when they are gone"
    requirement: DRIVE-01
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell"
        status: pass
    human_judgment: false
  - id: D5
    description: "The raw-absence assertion LIMIT 4 declined is present, gated on arrival, and observed red for DetailScreen [GitHistory tab]"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped (assertion 4)"
        status: pass
    human_judgment: false
  - id: D6
    description: "Untrusted's Debug is hand-written and prints shown(), so {:?} cannot put a raw invisible character into a log line, panic message or anyhow chain"
    requirement: SAFE-08
    verification:
      - kind: unit
        ref: "src/text.rs#a_carrier_debug_never_carries_an_invisible_character"
        status: pass
    human_judgment: false
  - id: D7
    description: "The ratatui premise is corrected per widget family and the correction rides the commit that falsifies it (probe doc, LIMIT 4, deferred-items.md)"
    verification:
      - kind: other
        ref: "scratch crate outside the repo against ratatui 0.30.2, run and deleted; re-derived in-tree by the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell arms 3 and 4"
        status: pass
    human_judgment: false
  - id: D8
    description: "The reach of the lever is stated with its uncovered remainder named carrier by carrier — six carrier types keep bare String fields with measured reasons"
    verification: []
    human_judgment: true
    rationale: "A claim about what is NOT covered cannot be discharged by a passing test; it is a disclosure whose accuracy a reader must judge against the tree. The measurements behind it are quoted below."

duration: 55 min
completed: 2026-08-27
status: complete
---

# Phase 21 Plan 23: Make an Unescaped Untrusted String Unrenderable Summary

**Retyped `GitLogEntry`'s four third-party fields to a carrier that implements none of the string conversions, so the compiler — not a reader working through a list — named all six consumers and the live `List`/`ListItem` Trojan Source leak in the GitHistory pane became a compile error.**

## Performance

- **Duration:** 55 min
- **Started:** 2026-08-27T16:03Z (approx — first gate measurement)
- **Completed:** 2026-08-27T16:58Z
- **Tasks:** 3 of 3
- **Files modified:** 7 (6 source, 1 planning doc)

## Accomplishments

- **`crate::text::Untrusted`** — the general carrier. No `Display`, `AsRef<str>`, `Deref`, `Borrow<str>`, `Into<Cow<'_, str>>`, no `serde`, and **no derived `Debug`**. Two accessors named after the questions they answer. It generalises `registry::LegacyRegistryKey`, whose shape was right in round 8 and whose application was one call site.
- **`crate::text::Rendered`** — the escaped result, and the only string type in this tree that is both escaped and `Into<Cow<'static, str>>`. The ergonomic asymmetry is the mechanism: `Span::raw(v.shown())` is short, `as_raw_for_logic_only()` is deliberate and greppable.
- **`render_for_terminal`** — the one composition of the control class and the invisible-formatting class. `strip_terminal_controls` moved into `text.rs` as the single production spelling of `ESC`/C0/`DEL`/C1; `sanitize_render_line` is now that class plus the display cap, behaviour-preserving.
- **`GitLogEntry` retyped**, wrapped at the one producer. Six consumers named by the compiler, resolved with a one-line justification at each.
- **The ratatui premise corrected per widget family**, re-derived outside this tree, with all three falsified sites fixed in the same commit as the measurement.
- **The probe gained teeth and an assertion**: a rendered-buffer survivorship precondition (so a `LOOK_ALIKE_PAIRS` reorder cannot silently empty assertion 3) and the raw-absence assertion LIMIT 4 declined.
- **The carrier's absent traits are certified by a control observed red twice**, not by a doc comment.

## Task Commits

1. **RED: populate `probe_ctx`'s `git_entries`** — `46cfdf7` (test) — committed deliberately red
2. **Task 1 (TRACER): the carrier, the one composition, and the git-history pane** — `f0d5d6a` (feat)
3. **Task 2: probe teeth + the reinstated raw-absence assertion** — `e45d0cd` (test)
4. **Task 3: the carrier's absent traits, certified by a control that goes red** — `881a348` (test)

## Files Created/Modified

- `src/text.rs` — `Untrusted`, `Rendered`, `render_for_terminal`, `strip_terminal_controls`, the `ESC`/`CONTROL_REPLACEMENT`/`TAB_WIDTH` consts (moved), the hand-written `Debug`, the autoref-specialization probe and its two controls
- `src/ui/screens/mod.rs` — `sanitize_render_line` redefined as the moved class plus the cap; the capped/uncapped agreement pin; the consts re-imported `#[cfg(test)]` only
- `src/state_reader/git_ops.rs` — `GitLogEntry`'s four fields retyped; wrapping at the one producer inside `load_git_log`
- `src/ui/screens/detail.rs` — six compiler-named sites resolved; `shown()` delegates to `render_for_terminal` (WR-02)
- `src/ui/screens/render_escape_guard.rs` — the populated `git_entries` fixture, `survives_a_rendered_buffer` + `ProbeSink`, assertion 0, assertion 4, the corrected probe doc and LIMITS block (4 rewritten, 5 added, ledger appended)
- `src/registry.rs` — `LegacyRegistryKey`'s doc correction (append-only, dated, quoting the sentence corrected); the derive is deliberately NOT removed
- `.planning/.../deferred-items.md` — the STANDING ratatui entry corrected per widget family, append-only, with a new dated table row

## Re-measured counts (every number below re-measured under `rtk proxy` against the FINAL tree)

| Quantity | Plan's figure (at `dfa11c6`) | Measured here (final tree) | Note |
|---|---|---|---|
| `Span::raw` under `src/` | 129 | **139** | +10, all added by this plan's own test/doc-free code and the git-history rewrite; the plan's figure verified at 129 against the pre-plan tree before any edit |
| `Span::styled` under `src/` | 198 | **201** | +3, same cause; verified at 198 pre-plan |
| sink calls under `src/ui/` | 502 | **569** | **NOT reproducible.** The plan does not state its spelling set. With `Span::raw\|Span::styled\|Line::from\|Paragraph::new\|\.title\(` the pre-plan tree measured 567 and the final tree 569. The plan's 502 is treated as unverifiable, not as evidence the tree changed. |
| `impl ... Screen for ` | 11 | **11** | matches; the standing brief's "13" remains the reviewer's 11 plus their own 2 plants |
| `GitLogEntry` references | 7 | **16** | +9: the retype's doc, the producer's four wraps, and the probe fixture |
| `git_entries` references | 14 | **18** | +4: the populated fixture and its doc |
| ratatui (this tree, `Cargo.lock`) | 0.30 | **0.30.2** | the scratch measurement crate resolved the same 0.30.2 |

## Gates

All through `rtk proxy` (bare `cargo`/`grep` never used for a count- or presence-bearing check).

| Gate | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test --workspace --no-fail-fast -- --test-threads=2` | **1376 passed / 0 failed / 13 ignored**, 35 result lines, 0 `FAILED` |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 101 with **exactly** the 4 pre-existing lints, at `src/browser.rs:131,132,133` and `src/project_creator.rs:146` — untouched per prohibition 4 |
| `git diff --stat` | names neither `src/browser.rs` nor `src/project_creator.rs` |
| `.planning/REQUIREMENTS.md` | untouched; `git log --oneline` for it still ends at `0c4f712` |
| `tests/driver_injection_corpus.rs` | untouched; `git diff --stat dfa11c6..HEAD` for it is empty |

**Test total delta, every unit attributed:** baseline 1372 → **1376**, +4.

| + | Test | Task |
|---|---|---|
| 1 | `ui::screens::tests::the_capped_and_uncapped_compositions_agree_below_the_cap` | 1 |
| 1 | `render_escape_guard::tests::the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell` | 2 |
| 1 | `text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions` | 3 |
| 1 | `text::tests::a_carrier_debug_never_carries_an_invisible_character` | 3 |

## The four reds, verbatim

**1. CR-04 made deterministic** (`46cfdf7`, before any escape landed):

```text
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (333264) panicked at src/ui/screens/render_escape_guard.rs:1157:17:
DetailScreen (src/ui/screens/detail.rs) [GitHistory tab] rendered ['\u{e0041}', '\u{ad}', '\u{e0041}', '\u{ad}', '\u{e0041}', '\u{ad}', '\u{e0041}', '\u{ad}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.
```

**Reproduction rate, measured in both directions against the compiled lib test binary:** **20 failures in 20 runs** before the fix; **0 failures in 20 runs** after it. **A defect that fires 20 out of 20 is not a flake.**

**2. Assertion 4 (the one LIMIT 4 declined)**, against a tree with exactly one `shown()` reverted:

```text
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (432171) panicked at src/ui/screens/render_escape_guard.rs:1399:29:
DetailScreen (src/ui/screens/detail.rs) [GitHistory tab] rendered the RAW hostile identity "demo\u{e0041}r\u{ad}un" into the terminal buffer. What the operator reads is therefore a value the terminal may reorder, hide characters in, or render as a different string entirely — this is Trojan Source (CVE-2021-42574) in a cell. Route what a human READS through crate::text::render_for_terminal; the raw value belongs only in lookups, map keys, path segments, subprocess arguments and persistence.
```

**3. `impl std::fmt::Display for Untrusted` planted:**

```text
thread 'text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions' (459964) panicked at src/text.rs:1361:9:
`Untrusted` implements `Display`. The whole mechanism is that a carrier cannot be interpolated: with `Display` present, `format!("{}", untrusted)` and `Span::raw(untrusted.to_string())` compile again at every site in the tree, and the compiler stops naming the consumers. Remove the impl; if a human needs to read the value, that is `shown()`.
```

**4. `impl AsRef<str> for Untrusted` planted:**

```text
thread 'text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions' (460706) panicked at src/text.rs:1370:9:
`Untrusted` implements `AsRef<str>`. A carrier that can be coerced to `&str` can be handed to any sink that takes one, which is the raw path restored everywhere at once and invisible in a diff. Remove the impl; the raw path is `as_raw_for_logic_only()` and it is meant to be conspicuous.
```

Each plant was reverted immediately and `git status --porcelain` confirmed clean. A **fifth** red was observed and is recorded under Issues Encountered: the trait probe's own control arm caught the probe being broken.

## The per-widget ratatui measurement (re-derived, not quoted)

A throwaway crate depending only on `ratatui = "0.30"` (resolved **0.30.2**) and `unicode-width` was created outside this repository, run, and deleted; `git status --porcelain` clean. Verbatim:

```text
ratatui per-widget cell survivorship
cp          width | Paragraph   Block::title  ListItem    Paragraph-in-Block
U+202E     2 | dropped     SURVIVES      SURVIVES    dropped
U+200B     2 | dropped     SURVIVES      SURVIVES    dropped
U+00AD     2 | dropped     SURVIVES      SURVIVES    dropped
U+2062     2 | dropped     SURVIVES      SURVIVES    dropped
U+2065     2 | dropped     SURVIVES      SURVIVES    dropped
U+FEFF     2 | dropped     SURVIVES      SURVIVES    dropped
U+E0041    2 | SURVIVES    SURVIVES      SURVIVES    SURVIVES

C0 controls (probe `a<CTRL>b`)
cp          | Paragraph   Block::title  ListItem    Paragraph-in-Block
U+001B      | dropped     dropped       dropped     dropped
U+000D      | dropped     dropped       dropped     dropped
U+0007      | dropped     dropped       dropped     dropped
```

The `width` column reads 2 rather than the plan's 7 because the probe string is `a<CP>b` rather than the plan's longer one. The verdicts, which are the load-bearing part, agree.

**Two things this adds that the plan's table did not have.** The C0 rows are new: every C0 tested is dropped by every family, so a probe asserting C0 absence in a `Buffer` would be vacuous in exactly the way the raw-absence assertion was thought to be. `strip_terminal_controls` is therefore justified by what reaches the **terminal**, not by what reaches a `Buffer` cell — the `Buffer` is not the boundary the `ESC` rule defends. That is now written into `deferred-items.md` with its direction.

Landed in the same commit as the measurement, append-only, each quoting the sentence it corrects: `render_escape_guard.rs`'s probe doc, its LIMIT 4, and `deferred-items.md`'s STANDING entry plus a new dated table row.

## The compiler-named consumers

The plan's central claim is that the compiler, not a reader, names the sites. It did. `cargo build` after the retype reported **six** errors, all in `detail.rs`, and `action.rs` / `app.rs` needed no change at all:

| Site | Question it answers | Resolution |
|---|---|---|
| `detail.rs:1533` | subprocess argument to `git diff-tree` | `as_raw_for_logic_only()` |
| `detail.rs:3009` | `List` row hash, read by a human | `shown()` |
| `detail.rs:3011` | `List` row date | `shown()` |
| `detail.rs:3013` | `List` row message — **the live Trojan Source** | `shown()` |
| `detail.rs:3015` | `List` row author | `shown()` |
| `detail.rs:3047` | diff-block title hash, read by a human | `shown()` |

No site in this plan's diff was found by reading. Prohibition 1 is satisfied by construction.

## The reach of the lever, and the uncovered remainder

Re-measured here: **139** `Span::raw` and **201** `Span::styled` calls under `src/`. **The lever gates none of them**, and cannot: `Span::raw` takes `Into<Cow<'_, str>>` and `String: Into<Cow<'_, str>>` is a standard-library impl this project cannot un-implement. What it gates is the carriers.

This plan retyped **one** carrier type (`GitLogEntry`, 4 fields). The round-9 set is 21 fields across 9 carrier types; `21-24`, `21-25` and `21-26` take the rest. **The six carrier types round 9 does NOT retype keep bare `String` fields** — `state_reader::ProjectState` (5 fields, 135 mentions), `roadmap_md::RoadmapPhase` (4), `queue_md::QueuedAction` (1), `AppContext.filtered_aliases`/`selected_alias`/per-screen `alias` (93 mentions), `AppContext.status_message` (the carrier boundary is inside a `format!`), and `config_json::GsdConfig`. **What bounds those is the census plus the behavioural probe — a sampling control, and saying so is the point. Direction: under-protection, silent.**

## Residuals, each with its failure direction

| # | Residual | Direction | Bounded by a committed control? |
|---|---|---|---|
| 1 | Six carrier types keep bare `String` fields | under-protection, silent | **No** — census + probe only; disclosed |
| 2 | The probe sees only states its fixtures construct | under-detection, silent | Partially — dispositions name their states; Backlog/Sessions/Archive/Browse/Defaults/Driver populated branches still unexercised |
| 3 | Homoglyphs are outside the judged class | under-detection, by design | **No** — closed at identity seams by the finite alphabet only |
| 4 | A render surface not reached through `Screen::render` | under-detection, silent | Partially — growth is bounded by the census, not the probe |
| 5 | Assertion 4 at a `Paragraph` site passes for a reason unrelated to the code | under-detection, silent | Bounded by assertions 2 and 3, which have different blind spots |
| 6 | The trait probe answers for the type as the test binary sees it | under-detection | Bounded by coherence: the orphan rule leaves any such impl nowhere to live but this crate |
| 7 | Four widget families measured, not all | under-detection | Bounded by the probe, which renders through the REAL `Screen::render` and so sees whatever family a screen used |
| 8 | The `\u{ad}` sighting's mechanism in verification pass 9 | unexplained | **No** — see Issues Encountered |

Every bound claimed in `render_escape_guard.rs`'s LIMITS block now names its control, and the block closes with that ledger explicitly.

## Decisions Made

D-21-7 through D-21-12 as planned; see key-decisions frontmatter. **D-21-10 is `costly`, not free:** `GitLogEntry`'s fields are `pub` on a crate published to crates.io, so the retype is a semver-breaking public-API change that lands in the next minor. Fully revertible from git; no data destroyed, no on-disk format changed.

## Deviations from Plan

### 1. [Rule 3 - Blocker] `clippy::explicit_counter_loop` in the refactored `sanitize_render_line`

- **Found during:** Task 1 (step e)
- **Issue:** The first behaviour-preserving rewrite used a manual `n` counter, which `-D warnings` rejects.
- **Fix:** Rewrote as `if expanded.chars().count() <= cap { return expanded; }` then `chars().take(cap - 1)` plus the ellipsis. Equivalence re-derived by hand for the three boundary cases (below cap, exactly cap, above cap) and pinned by `the_capped_and_uncapped_compositions_agree_below_the_cap`.
- **Verification:** `cargo clippy -- -D warnings` exit 0; every pre-existing `sanitize_render_line` test passes unmodified.
- **Committed in:** `f0d5d6a`

### 2. [Rule 3 - Blocker] Unused-import warning from moving the control-class consts

- **Found during:** Task 1 (step e)
- **Issue:** After moving `ESC`/`CONTROL_REPLACEMENT`/`TAB_WIDTH` to `text.rs`, an unconditional re-import in `ui::screens` was an unused import in non-test builds (`-D warnings` failure), because nothing in that module's production path refers to them any more.
- **Fix:** `#[cfg(test)] use crate::text::{CONTROL_REPLACEMENT, ESC};` — narrowed to what the pre-existing tests actually assert with, so their bodies stay unmodified. `TAB_WIDTH` is not re-imported because nothing in `mod.rs` uses it.
- **Verification:** `cargo clippy -- -D warnings` exit 0; `grep -rn CONTROL_REPLACEMENT src/` names `src/text.rs` for the const and its one production use.
- **Committed in:** `f0d5d6a`

### 3. [Deviation - the plan contradicted itself] The diff-block title's hash

- **Found during:** Task 1 (step f)
- **Issue:** The plan's `must_haves.truths` says the diff-block title's hash at `:3045` resolves to `as_raw_for_logic_only()`; the plan's own `<action>` (f) says the same site resolves to `shown()`.
- **Resolution:** `shown()`. `Block::title` is the widget family that preserves the invisible class most completely (measured — even `U+202E` reaches a cell through it), and the title is read by a human. The `must_haves` wording is the erroneous half. Recorded rather than silently picked.
- **Committed in:** `f0d5d6a`

### 4. [Deviation - the plan asked for something that does not exist] The teeth precondition's `false` direction

- **Found during:** Task 2 (step b)
- **Issue:** The plan asked the `false` direction be built from "characters the measurement shows are dropped by every family under probe". **No such identity exists for the invisible class**: the per-widget measurement shows `ListItem` preserving all seven code points tested. The only universally dropped code points are C0 controls, which are `Cc` — outside the class the helper judges — so an identity of them would answer `false` for two reasons at once and certify neither.
- **Resolution:** The `false` direction is delivered by the two mechanisms that actually make the precondition fire, both asserted: (a) a fixture carrying no class member at all (`clean_identity()`), and (b) a fixture whose class members are dropped by the family under measurement (`zero_width_only_identity()` through `ProbeSink::Paragraph`). Arm (b) is **strictly stronger** than what was asked, because the SAME value answers `true` through `ProbeSink::ListItem` — one value, two answers, which proves the helper measures the render rather than the string, and re-derives the per-widget correction inside this tree.
- **Verification:** `the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell`, four arms, passing.
- **Committed in:** `e45d0cd`

### 5. [Deviation - an acceptance criterion is self-defeating as literally spelled]

- **Found during:** Task 1 verification
- **Issue:** The criterion `grep -c "impl std::fmt::Display for Untrusted\|..." src/text.rs` returns **2**, not 0. Both hits are **doc-comment lines** at `src/text.rs:1333` and `:1341` — the verbatim planting-red descriptions that Task 3's own acceptance criteria required be quoted. A naive text grep cannot tell a comment from an implementation, so the criterion as spelled forbids the evidence another criterion demands.
- **Resolution:** Measured the way this tree's own census idiom measures (`exactly_one_executable_spelling_...` skips lines whose trimmed form opens a line comment): **0 executable-line matches, 2 comment-line matches.** The criterion itself says Task 3's runtime control is what certifies the absence rather than the grep, and that control exists and was observed red twice.
- **Committed in:** `881a348`

---

**Total deviations:** 5 — 2 auto-fixed blockers (Rule 3), 3 plan-defect resolutions recorded rather than patched quietly.
**Impact on plan:** None on scope. Deviations 3, 4 and 5 are defects in the plan text, not in the tree; each is recorded with the reasoning rather than resolved silently, which is what prohibitions 1 and 6 require.

## Recurrence check — did this round enumerate one level down?

Phase standing constraint 5 requires this be answered plainly. **No.** The test:

- The GitHistory leak was closed by changing a **type**, and the six sites were produced by `cargo build`, not by reading.
- The two-class question was closed by **one composition** consumed by every site, not by adding `display_identity(...)` at more sites.
- The probe's teeth were closed by a **rendered-behaviour property**, not by pinning an index or adding characters to a fixture.
- The absent-trait claim was closed by a **runtime control observed red**, not by adding a stronger sentence to a doc.

**Where this round DID add to a list, said plainly:** `zero_width_only_identity` names three `LOOK_ALIKE_PAIRS` indices (`ZERO_WIDTH_PAIRS = [0, 2, 5]`). That is a hand-maintained list of the same kind the phase keeps paying for. Its direction is benign — a wrong index makes the control itself go red rather than making a guard vacuous, because the test asserts the value answers `false` through `Paragraph` **and** `true` through `ListItem` — but it is a list and it is disclosed as one.

## Issues Encountered

**1. The trait probe was broken on first write, and its own control arm caught it.** The autoref-specialization call was written `(&Wrap(v)).implements_display()` with one `&`. Because every method takes `&self`, the first candidate step solves `Self = Wrap<T>` and matches the `No` half **for every type** — the probe answered `false` unconditionally. Had the test asserted only the three absences, it would have passed forever while certifying nothing. The `String` control arm went red instead:

```text
`String` implements `Display` and the probe said otherwise, so the probe is broken and every absence asserted above proved nothing
```

This is the both-directions shape working, and it is recorded in the probe's doc rather than quietly fixed. Resolution: `(&&Wrap(v))`, as the plan in fact specified.

**2. Verification pass 9's sighting is NOT fully explained, and this plan does not pretend otherwise.** The populated fixture makes the GitHistory leak fire 20/20, so reading (a) of pass 9's dichotomy — a real leak in this tab — is settled by construction. But the character counts do not match: pass 9 reported **four** characters, this fixture reports **eight** (one hostile pair per rendered field; four is two fields' worth). The class is reproduced; the exact count is not. **The mechanism of that single sighting remains unexplained and is recorded as an open discrepancy** in `probe_ctx`'s doc, in `46cfdf7`'s message, and here. Any residual intermittency after this plan is reading (b) and must be reported as that rather than absorbed.

**3. One documented flake fired once during a full-suite run** — `envelope_tracer::a_relocated_copy_of_the_stub_refuses_instead_of_acting` (`Text file busy`, the documented write-then-exec race). Re-run alone: 6 passed, 0 failed. Not a regression, not re-diagnosed.

## Prohibition compliance

| # | Prohibition | Status |
|---|---|---|
| 1 | No site closed by reading | **Met** — all six named by `cargo build`; the table above is the compiler's output |
| 2 | No work against ROADMAP criterion 4 | **Met** — `tests/driver_injection_corpus.rs` untouched, verified by `git diff --stat` |
| 3 | `sanitize_render_line`'s return unchanged | **Met** — `git diff -U0` shows zero hunks inside any pre-existing `#[test] fn`; the only test-region hunk is a pure 86-line insertion |
| 4 | The 4 pre-existing clippy lints not fixed | **Met** — `--all-targets` still reports exactly those 4, at the same lines; `git diff --stat` names neither file |
| 5 | The ratatui table re-derived, not quoted | **Met** — scratch crate outside the repo, run, deleted, tree clean |
| 6 | No unbacked bound claim left in the diff | **Met** — LIMITS block closes with a bounds/residuals ledger; every residual above carries its direction |
| 7 | No raw invisible character in any file | **Met** — a Cf-class scan of the whole plan diff (83,306 chars) returns **0**, and the scan is non-vacuous: the same function reports 5 offenders on a planted string built from `chr()` code points |
| 8 | No prior plan/SUMMARY/deferred line rewritten | **Met** — the `deferred-items.md` change is append-only and quotes the sentence it corrects |
| 9 | `.planning/REQUIREMENTS.md` untouched | **Met** — its `git log` still ends at `0c4f712` |
| 10 | Every number re-measured, none inherited | **Met** — see the re-measured counts table, including the one figure that did not reproduce |

## Known Stubs

None. No hardcoded empty value, placeholder string, TODO or unwired component was introduced.

## Next Phase Readiness

`21-24` is unblocked and inherits `crate::text::Untrusted` as the carrier to demote `LegacyRegistryKey` onto (and, with it, the `#[derive(Debug)]` removal this plan deliberately did not make). `21-25` and `21-26` inherit the same carrier plus `render_for_terminal`.

**Ready for 21-24.**

## Self-Check: PASSED

- `src/text.rs`, `src/ui/screens/mod.rs`, `src/state_reader/git_ops.rs`, `src/ui/screens/detail.rs`, `src/ui/screens/render_escape_guard.rs`, `src/registry.rs`, `deferred-items.md` — all present on disk and all modified.
- Commits `46cfdf7`, `f0d5d6a`, `e45d0cd`, `881a348` all present in `git log --oneline`.
- Every task-level acceptance criterion re-run against the final tree; results recorded above, including the one criterion (deviation 5) that could not be satisfied as literally spelled and the executable-line measurement substituted for it.
- Plan-level `<verification>` re-run: `cargo build` exit 0; `cargo test --workspace --no-fail-fast` 1376/0/13 with every delta attributed; `cargo clippy -- -D warnings` exit 0; four reds recorded verbatim, each followed by a clean tree; the ratatui measurement re-derived and quoted in three places with the version; `REQUIREMENTS.md` and `tests/driver_injection_corpus.rs` untouched.
