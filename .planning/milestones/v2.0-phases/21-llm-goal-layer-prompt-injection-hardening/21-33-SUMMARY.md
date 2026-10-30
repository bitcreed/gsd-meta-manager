---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 33
subsystem: testing
tags: [rust, ratatui, unicode, autoref-specialization, total-order, census, prompt-injection]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "21-26's sealed RenderAdjudicated supertrait; 21-29's src/ui census and LOOK_ALIKE_PAIRS; 21-30's EditBuffer type; 21-27's Untrusted trait-probe pattern"
provides:
  - "gaps[3] closed by NARROWING the census's claim to what it checks, disclosing the residual, and pinning the disclosed reach as a checked two-direction equality"
  - "WR-04 closed: EditBuffer's three claimed trait absences certified by a local autoref-specialization probe with String presence arms, each observed RED by planting its impl"
  - "T-21-33-03 closed: the doc's buffer.clone() example, which named a method the type does not have, replaced with a measured construction and two verbatim compiler errors"
  - "WR-05 closed: backlog_number_ordering made total over ELEMENTS by an appended string tiebreak"
  - "21-29's S1 delivered: a two-direction spot-check on an input-echo screen through the real Screen::render"
  - "21-29's F3 CLOSED AS UNNECESSARY by measurement, not deferred a third time"
affects: [21-34, phase-21 verification pass 12]

actuals:
  tokens: 41000
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Coverage PIN: a control's disclosed reach committed as a per-file distribution asserted by Vec equality, so prose and measurement cannot drift"
    - "Narrower-claim-plus-disclosure as the close for a false completeness claim, with the completeness handed by name to the mechanisms that carry it"
    - "Widget-family-aware fixture selection: choose the LOOK_ALIKE_PAIRS entry the rendering family PRESERVES, or the absence assertion is vacuous forever"

key-files:
  created: []
  modified:
    - src/ui/mod.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/render_escape_guard.rs
    - src/state_reader/backlog.rs

key-decisions:
  - "D-21-56: gaps[3] closed by narrowing the claim and disclosing the remainder, NOT by a fourth census — a needle census's coverage shrinks as the conversion succeeds, so it is the wrong shape for a completeness claim"
  - "D-21-57: the disclosed reach is PINNED as a checked per-file equality — and it fired on its own author inside this plan, which is its justification rather than an embarrassment"
  - "D-21-58: WR-04's probe replicated locally in src/ui/screens/mod.rs rather than imported from src/text.rs, which 21-32 owns this wave"
  - "D-21-59: WR-05 closed by appending a string tiebreak rather than changing the key function"
  - "D-21-60: S1 housed in render_escape_guard.rs; F3's visibility widening DECLINED as unnecessary"
  - "Deviation: the S1 fixture is TAG_PAIR, not a zero-width pair — the plan's fallback guidance would have produced a vacuous control for the Paragraph family"

patterns-established:
  - "A coverage pin's red is a BOOKKEEPING red with a one-line repair stated in its own failure message: re-measure, update the constant, update the disclosed sentence, one commit"
  - "Every residual named WITH its failure direction (under-detection/over-detection, silent/loud, and whether it grows over time)"
  - "A doc correction is dated and left legible — the superseded claim is quoted as wrong rather than silently replaced"

requirements-completed: [SAFE-07, DRIVE-03]

coverage:
  - id: D1
    description: "The src/ui census claims exactly what it checks: renamed to no_display_identity_call_under_ui_stands_outside_a_composition, with the completeness claim handed by name to the sealed RenderAdjudicated bound and the behavioural probes"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/mod.rs#no_display_identity_call_under_ui_stands_outside_a_composition"
        status: pass
      - kind: unit
        ref: "rtk proxy grep -c 'every_render_site_under_ui_composes_both_classes' src/ui/mod.rs == 0"
        status: pass
    human_judgment: false
  - id: D2
    description: "The census's disclosed reach is a checked number, not a sentence — a per-file distribution asserted by Vec equality in both directions, with a file-count floor"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/mod.rs#no_display_identity_call_under_ui_stands_outside_a_composition (MEASURED_REACH + UI_SOURCE_FLOOR assertions)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The shrinking-coverage property disclosed in words with its direction (under-detection, silent, growing over time), plus pass 11's 63-site hand-trace and its 'not a live leak' conclusion"
    verification: []
    human_judgment: true
    rationale: "A prose disclosure's adequacy is a judgment about whether a future reader is correctly warned; no test can assert that the wording conveys the residual. The NUMBERS it states are pinned by D2, which is the part automation can carry."
  - id: D4
    description: "WR-04: EditBuffer's three claimed trait absences (Display, AsRef<str>, Into<Cow<'static, str>>) certified by a local autoref-specialization probe with a String presence arm per absence"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/screens/mod.rs#an_edit_buffer_implements_none_of_the_string_conversions"
        status: pass
      - kind: unit
        ref: "rtk proxy grep -c 'implements_' src/ui/screens/mod.rs == 31 (was 0)"
        status: pass
    human_judgment: false
  - id: D5
    description: "EditBuffer's doc example corrected: the buffer.clone() construction named a method the type does not have; replaced with the construction that actually fails at the trait bound, both compiler errors committed verbatim"
    requirement: SAFE-07
    verification:
      - kind: other
        ref: "measured by scratch plant: E0599 'no method named clone' for the old example, E0277 'Cow<'_, str>: From<EditBuffer>' for the corrected one"
        status: pass
    human_judgment: false
  - id: D6
    description: "WR-05: backlog_number_ordering is total over ELEMENTS — tied keys no longer inherit read_dir's order"
    requirement: DRIVE-03
    verification:
      - kind: unit
        ref: "src/state_reader/backlog.rs#tied_keys_are_ordered_by_the_element_so_read_dir_cannot_decide_the_display_order"
        status: pass
      - kind: unit
        ref: "src/state_reader/backlog.rs (all 9 tests, incl. the 3 pre-existing ordering controls)"
        status: pass
    human_judgment: false
  - id: D7
    description: "21-29's S1 delivered: a two-direction spot-check on EnqueueScreen (an input-echo screen) through its real Screen::render, arrival asserted before property, widget family named"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#an_input_echo_screen_escapes_a_hostile_value_and_leaves_a_clean_one_alone"
        status: pass
    human_judgment: false
  - id: D8
    description: "21-29's F3 closed as UNNECESSARY by measurement: no visibility widened anywhere"
    verification:
      - kind: unit
        ref: "rtk proxy grep -c 'pub(super) fn ctx_with_aliases' src/ui/screens/mod.rs == 1; src/ui/screens/mod.rs absent from Task 3's diff"
        status: pass
    human_judgment: false

duration: 44 min
completed: 2026-08-27
status: complete
---

# Phase 21 Plan 33: Narrowing the src/ui Census and Delivering Three Carried Items Summary

**The census whose name promised every render site now claims only what it checks — six executable escape calls in two of sixteen files — with its reach pinned as a checked equality that fired on its own author mid-plan; plus WR-04's trait certificate, WR-05's element-total comparator, and 21-29's S1 delivered with F3 declined by measurement.**

## Performance

- **Duration:** 44 min
- **Tasks:** 3 (plus one in-plan pin repair)
- **Files modified:** 4
- **Commits:** 4

## The pattern call for gaps[3], in my own words

A needle-driven census structurally **cannot** carry the completeness its old name claimed, and the reason is that its coverage moves the wrong way: a render site that applies **neither** escape class carries no needle and is invisible to it, and every render site successfully converted to the single composed call **removes** a needle. Its reach therefore shrinks monotonically as the work it certifies succeeds. Making the old name true would require enumerating render **sites** rather than escape **calls** — a structurally different mechanism — and that mechanism already exists in this tree twice over: the sealed `RenderAdjudicated` supertrait (21-26), which makes `impl Screen for X` on an unadjudicated `X` a compile error, and the per-screen behavioural probes in `render_escape_guard.rs`. Building a fourth census here would have moved the over-claim, not ended it.

So: the test is renamed to what it checks, its reach is disclosed file by file, the shrinking property is stated with its direction, and the completeness claim is **handed by name** to the two mechanisms that carry it. The one bounded mechanism addition is the reach **pin**, because a disclosure that is only prose is exactly what eleven passes have taught this phase not to trust.

**How the claim was narrowed, concretely.** Old name: `every_render_site_under_ui_composes_both_classes` — asserted every render site under `src/ui/`. New name: `no_display_identity_call_under_ui_stands_outside_a_composition` — asserts that every executable *occurrence of the escape call* is composed with the control-class call on the same logical unit. The scope dropped from "all render sites" (unbounded, unmeasured) to "all needle-carrying executable lines" (six non-exempt, in two of sixteen files, pinned).

## Re-measurement against the planner's figures

Every number re-measured under `rtk proxy` in this worktree. **All agreed with the planner exactly.**

| Gate | Planner at `343c408` | My measurement | Agrees |
|---|---|---|---|
| `cargo clippy -- -D warnings` (lib) | exit 0 | exit 0 | yes |
| `cargo clippy --all-targets -- -D warnings` | exactly 4 lints: `bool_assert_comparison` ×3 at `src/browser.rs:155,156,157`, `cmp_owned` ×1 at `src/project_creator.rs:146` | identical — 3 × "used `assert_eq!` with a literal bool", 1 × "this creates an owned instance just for comparison", same 4 line refs | yes |
| Needle distribution under `src/ui/` | 7 executable occurrences, 3 files: `driver_confirm.rs` 4, `driver.rs` 2, `render_escape_guard.rs` 1 | identical | yes |
| `.rs` files under `src/ui/` | 16 | 16 | yes |
| Census raw / outstanding sites | — | raw 1 (`render_escape_guard.rs:2615`), outstanding 0 | — |
| `EXEMPTIONS` / `WAVE_PENDING` | 2 / 0 | 2 / 0 | yes |
| `grep -c "implements_" src/ui/screens/mod.rs` | 0 | 0 | yes |
| `super::tests::ctx_with_aliases` call sites | `:982`, `:1191` | `:982`, `:1191` | yes |

**No differences to report.** The needle distribution was measured with an independent Python walk over all sixteen files, not by re-reading the plan.

## Task Commits

1. **Task 1: rename the census, pin its measured reach, disclose the shrink** — `9eacc82` (test)
2. **Task 2: certify EditBuffer's absences, correct its example, make the backlog order total over elements** — `43b7c93` (test)
3. **Task 3: deliver 21-29's S1, close F3 as unnecessary** — `5400e44` (test)
4. **In-plan pin repair: re-measure the reach after Task 3** — `a23a4c9` (test)

## Accomplishments

### Task 1 — the narrowed claim and the reach pin

- Renamed to `no_display_identity_call_under_ui_stands_outside_a_composition`. `rtk proxy grep -c "every_render_site_under_ui_composes_both_classes" src/ui/mod.rs` → **0**; new name → **2**.
- Added `needle_distribution()`, `MEASURED_REACH` and `UI_SOURCE_FLOOR`. The pin is a `Vec` equality, so it fails in **both** directions — a file leaving the distribution is as much a red as one entering.
- The census's printed raw site list, quoted from the run: `["src/ui/screens/render_escape_guard.rs:2615", "src/ui/screens/render_escape_guard.rs:3272"]` (final state; it was `["src/ui/screens/render_escape_guard.rs:2615"]` before Task 3). **`src/ui/mod.rs` does not appear in it**, and `the_census_cannot_report_itself` asserts that independently.
- `EXEMPTIONS` still exactly 2 entries, `WAVE_PENDING` still exactly 0, `stale_exemptions(&files, &EXEMPTIONS)` still empty (asserted green by `a_stale_exemption_is_reported`).

**The rewritten doc block** contains, in order: (i) what the mechanism checks; (ii) the measured reach as a file-by-file table with the "two of sixteen files" framing and a note that the numbers are pinned rather than asserted in prose; (iii) the shrinking-coverage property — *"a render site that applies NEITHER class carries no needle, so it is **invisible to this census** — silence, not clearance. And every render site successfully converted to the single composed call REMOVES a needle, so **this census's reach shrinks monotonically as the very work it certifies succeeds**. **Direction: under-detection, silent, and growing over time.**"*; (iv) an explicit "This census carries NO completeness claim. Here is what does." section naming the sealed `RenderAdjudicated` supertrait and the `render_escape_guard` behavioural probes; (v) pass 11's hand-trace with attribution — 63 `Span::raw`/`Span::styled` sites sampled, the risky ones enumerated by line, *"a false completeness claim with an undisclosed residual, not a live leak"*, plus the note that inflating it into a vulnerability would be the same overclaim pointing the other way. The two pre-existing residuals (alias/re-export invisibility; over-length composition) and all three committed REDs are kept.

**Reach pin observed RED**, by planting one executable occurrence in `src/ui/screens/help.rs` (an unowned file this plan does not otherwise touch):

```
CENSUS reach — executable needle occurrences per file (4 files, 8 occurrences): [("src/ui/screens/driver.rs", 2), ("src/ui/screens/driver_confirm.rs", 4), ("src/ui/screens/help.rs", 1), ("src/ui/screens/render_escape_guard.rs", 1)]

thread 'ui::tests::no_display_identity_call_under_ui_stands_outside_a_composition' (2953298) panicked at src/ui/mod.rs:544:9:
assertion `left == right` failed: THE CENSUS'S REACH CHANGED. This is a BOOKKEEPING red with a one-line repair: re-measure the distribution, update `MEASURED_REACH`, and update the disclosed reach table in this test's doc — all in the SAME commit, so the sentence and the measurement cannot drift apart. Never relax this assertion.

A SHRINK here is the property this doc describes actually happening: a render site was converted to the single composed call, which removes a needle and narrows what this census can speak about at all. A GROWTH is a new escape call somewhere under src/ui/ that did not exist when the reach was measured.

The equality is checked in both directions on purpose, so a file LEAVING the distribution is as much a finding as one entering it.
  left: [("src/ui/screens/driver.rs", 2), ("src/ui/screens/driver_confirm.rs", 4), ("src/ui/screens/help.rs", 1), ("src/ui/screens/render_escape_guard.rs", 1)]
 right: [("src/ui/screens/driver.rs", 2), ("src/ui/screens/driver_confirm.rs", 4), ("src/ui/screens/render_escape_guard.rs", 1)]
```

Plant reverted; `rtk proxy git status --porcelain` showed only ` M src/ui/mod.rs`.

### The pin fired on its own author — the episode, reported not smoothed

During the plan-level `cargo test --workspace` run, the reach pin **failed**. Task 3's S1 spot-check had added an oracle line to `render_escape_guard.rs` (`:3272`, `let expected = display_identity(hostile);`) — an eighth executable occurrence — which made Task 1's disclosure stale **within the same plan**.

I applied the repair the pin's own failure message prescribes, in one commit (`a23a4c9`): re-measured (8 occurrences; `render_escape_guard.rs` 1 → 2), updated `MEASURED_REACH`, and updated the disclosed reach table in the doc. The episode is recorded **on the constant itself**, because it is the pin's entire justification: a prose-only disclosure would have shipped stale within one plan of being written.

Both `render_escape_guard.rs` occurrences (`:2615`, `:3272`) are **oracles** computing the expected escaped form of a hostile fixture — precisely why that file is exempt. Neither is a render site, so **the load-bearing figure is unchanged: six non-exempt occurrences, in two of sixteen files.**

### Task 2 — WR-04 and WR-05

**WR-04.** Pre-change `rtk proxy grep -c "implements_" src/ui/screens/mod.rs` returned **0** — the measurement WR-04 rests on. After: **31**. A local autoref-specialization probe (replicated from `src/text.rs`'s pattern, never imported — `src/text.rs` belongs to 21-32 this wave and was read only) drives `an_edit_buffer_implements_none_of_the_string_conversions`: three absences for `EditBuffer` and three matching presences for `String`, so a probe answering `false` to everything cannot pass.

**Three verbatim REDs, one per planted impl**, each removed with a clean tree before the next:

```
---- 1. impl std::fmt::Display for EditBuffer ----
thread 'ui::screens::tests::an_edit_buffer_implements_none_of_the_string_conversions' (2968942) panicked at src/ui/screens/mod.rs:1857:9:
EditBuffer implements Display. That restores the round trip CR-03 closed: the value becomes interpolable into a format!, a Span or a Paragraph with no escape, which is exactly the laundering path the type exists to make impossible. If this impl is wanted, the type's doc and this control must both be revised deliberately — not left green.

---- 2. impl AsRef<str> for EditBuffer ----
thread 'ui::screens::tests::an_edit_buffer_implements_none_of_the_string_conversions' (2970294) panicked at src/ui/screens/mod.rs:1866:9:
EditBuffer implements AsRef<str>. Anything taking `impl AsRef<str>` then accepts the raw buffer directly, so the escape is bypassed at every such call site at once.

---- 3. impl From<EditBuffer> for Cow<'static, str> ----
thread 'ui::screens::tests::an_edit_buffer_implements_none_of_the_string_conversions' (2971327) panicked at src/ui/screens/mod.rs:1872:9:
EditBuffer implements Into<Cow<'static, str>>. ratatui's Span, Line and Paragraph constructors take exactly that bound, so this impl alone would let the raw buffer reach a terminal cell.
```

`rtk proxy git status --porcelain` was clean of the plant after each. (Panic line numbers are offset by each planted impl's own lines; that is stated in the doc rather than left as an inconsistency.)

**The wrong example, measured.** The doc argued from `Span::styled(buffer.clone(), ..)`. **`EditBuffer` derives nothing at all — `Clone` included — so that example named a method this type does not have and never described a reachable call.** Measured by scratch plant, it fails at a completely different error, *before* the trait bound is ever consulted:

```
error[E0599]: no method named `clone` found for struct `screens::EditBuffer` in the current scope
    --> src/ui/screens/mod.rs:1778:44
     |
 787 | pub struct EditBuffer(crate::text::Untrusted);
     | --------------------- method `clone` not found for this struct
```

The corrected example is the construction that actually fails at the bound the three absences are about:

```
error[E0277]: the trait bound `Cow<'_, str>: std::convert::From<screens::EditBuffer>` is not satisfied
    --> src/ui/screens/mod.rs:1774:37
     |
1774 |         ratatui::text::Span::styled(buffer, ratatui::style::Style::default())
     |         --------------------------- ^^^^^^ the trait `std::convert::From<screens::EditBuffer>` is not implemented for `Cow<'_, str>`
     |         |
     |         required by a bound introduced by this call
     |
     = note: required for `screens::EditBuffer` to implement `Into<Cow<'_, str>>`
note: required by a bound in `ratatui::prelude::Span::<'a>::styled`
    --> ratatui-core-0.1.2/src/text/span.rs:163:12
     |
 161 |     pub fn styled<T, S>(content: T, style: S) -> Self
 162 |     where
 163 |         T: Into<Cow<'a, str>>,
     |            ^^^^^^^^^^^^^^^^^^ required by this bound in `Span::<'a>::styled`
```

Both errors are committed in the doc, `shown` is named as the one legitimate route to a cell, and the correction is **dated and left legible** ("Correction, dated and left legible rather than made silently"), because a maintainer who tested the old claim would have found the example wrong and might reasonably have concluded the claim was too.

**WR-05.** `backlog_number_ordering` gained `.then_with(|| a.cmp(b))`. **Verbatim RED against the pre-tiebreak comparator:**

```
thread 'state_reader::backlog::tests::tied_keys_are_ordered_by_the_element_so_read_dir_cannot_decide_the_display_order' (2961201) panicked at src/state_reader/backlog.rs:370:13:
assertion `left == right` failed: two items whose keys tie came out in a different ELEMENT order from rotation 1 than from rotation 0 (["999.alpha", "999.zebra"] vs ["999.zebra", "999.alpha"]). The key sequence being permutation-independent is not enough: the operator reads elements, and a tied group that inherits `read_dir`'s order is the filesystem deciding the display order.
  left: ["999.alpha", "999.zebra"]
 right: ["999.zebra", "999.alpha"]
```

**Every ordering control in `src/state_reader/backlog.rs`, before and after:**

| Control | Before tiebreak | After tiebreak | Asserted order changed? |
|---|---|---|---|
| `a_non_numeric_suffix_cannot_make_the_order_depend_on_the_input_permutation` | pass | pass | **No** |
| `the_backlog_comparator_is_antisymmetric_and_transitive_including_over_a_nan_suffix` | pass | pass | **No** |
| `well_formed_backlog_numbers_keep_the_order_they_had_before_the_total_order_fix` | pass | pass | **No** |
| `a_non_finite_or_unparsable_suffix_takes_the_same_fallback_key` | pass | pass | **No** |
| `tied_keys_are_ordered_by_the_element_...` (new) | **RED** | pass | n/a — new |

`rtk proxy cargo test --lib -- state_reader::backlog --nocapture` → **9 passed / 0 failed** (8 before, +1). **No asserted order changed**, so nothing was absorbed. Display ordering only; no persisted artifact.

### Task 3 — S1 delivered, F3 declined

**F3 closed as UNNECESSARY, with the measurement.** `src/ui/screens/render_escape_guard.rs` is a **child module of `ui::screens`**, so it can already reach that module's private `tests` and its `pub(super) fn ctx_with_aliases` — it does so at **`render_escape_guard.rs:982`** and **`:1191`**, both calling `super::tests::ctx_with_aliases`. The widening 21-29's F3 asked for and 21-30 deferred is therefore not merely deferred but **not needed**. Confirmed by measurement: `rtk proxy grep -c "pub(super) fn ctx_with_aliases" src/ui/screens/mod.rs` → **1**, and `src/ui/screens/mod.rs` is absent from Task 3's diff. No visibility anywhere widened.

**The subject, and the widget family.** `EnqueueScreen` (`src/ui/screens/enqueue.rs`), whose footer at `:126-133` echoes `ctx.input_buffer` — **the characters the operator pressed**, arriving through no file at all. That is the distinction S1 was about and that 21-29's two `.planning/`-derived spot-checks missed.

The footer is a **`Paragraph`**. This tree's own measurement (`ProbeSink`'s doc) records that against ratatui 0.30.2 a `Paragraph` **DROPS** `U+200B`, `U+FEFF`, `U+00AD`, `U+202E`, `U+2062` and `U+2065` before a cell exists, while both families preserve the **tag block**. The spot-check relies on the invisible-character-count assertion, and its fixture is `LOOK_ALIKE_PAIRS[TAG_PAIR]` (`("demo", "demo\u{e0041}")`) precisely because that family **preserves** `U+E0041`.

**Verbatim RED**, by temporarily reverting the escape at `enqueue.rs:128`:

```
thread 'ui::screens::render_escape_guard::tests::an_input_echo_screen_escapes_a_hostile_value_and_leaves_a_clean_one_alone' (3019460) panicked at src/ui/screens/render_escape_guard.rs:3252:9:
EnqueueScreen rendered 1 invisible-formatting character(s) into the buffer: ['\u{e0041}']. What the operator typed reached a terminal cell unescaped, through the Paragraph footer at src/ui/screens/enqueue.rs:126-133.
```

The plant's file was `src/ui/screens/enqueue.rs`; it was restored, `rtk proxy git status --porcelain` showed only ` M src/ui/screens/render_escape_guard.rs`, and `enqueue.rs` is **absent from the final diff**.

**The one-screen-one-pair limit, quoted from the test's doc:**

> "**One screen and one fixture pair.** `EnqueueScreen` was chosen because S1 named the input-echo CLASS, not because one screen stands for five: `AddProjectScreen`, `CreateProjectScreen`, `DriverInjectScreen` and `DriverStartScreen` are covered only by this module's per-screen behavioural probes, which assert arrival and escaping but not this test's clean-value no-over-escaping direction. **Direction: under-detection, silent, bounded by those probes and not by this test.** A single spot-check must not acquire a class-wide claim — that is this round's own subject, one level down."

`rtk proxy cargo test --lib -- ui::screens::render_escape_guard --nocapture` → **14 passed / 0 failed** (13 before; the +1 is this test).

## Verification

| Check | Result |
|---|---|
| `rtk proxy cargo build` | exit 0 |
| `rtk proxy cargo test --workspace --no-fail-fast` | **1420 passed / 0 failed / 13 ignored**, 35 binaries |
| Delta vs. the 1417/0/13 green baseline | **+3**, one per test added: the backlog tiebreak control, the `EditBuffer` probe, the S1 spot-check. The census rename is net 0. |
| `driver_reattach` | Failed 2/3 in the first full-workspace run; **3 passed / 0 failed** on an isolated re-run. The documented flake — a system-wide `/proc` scan that does not stop at the worktree boundary, which three sibling worktrees running concurrently is exactly the trigger for. Reported, not absorbed. |
| `rtk proxy cargo clippy -- -D warnings` | exit 0 |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | exactly **4** lints before and after: 3 × `bool_assert_comparison` (`src/browser.rs:155,156,157`), 1 × `cmp_owned` (`src/project_creator.rs:146`) |
| `driver_injection_corpus` | **13 passed / 0 failed / 10 ignored** — HEAD counts, file unchanged in the diff. No work claimed against ROADMAP criterion 4. |
| `rtk proxy git diff --stat` (whole plan) | `src/state_reader/backlog.rs`, `src/ui/mod.rs`, `src/ui/screens/mod.rs`, `src/ui/screens/render_escape_guard.rs` — **only the four declared files** |
| `.planning/REQUIREMENTS.md` | untouched |
| Fenced files (`detail.rs`, `session_detector.rs`, `text.rs`, `driver.rs`, `driver_injection_corpus.rs`, `enqueue.rs`, `help.rs`) | all absent from the diff |

**Six verbatim REDs delivered**, each with a clean tree after the plant: the reach pin (planted needle in `help.rs`), three trait absences (three planted impls), the backlog tiebreak (pre-tiebreak comparator), and the S1 spot-check (reverted escape). Plus one verbatim compiler error for the corrected example — and, unplanned, a **seventh** red: the reach pin against Task 3's own addition.

## Decisions Made

- **D-21-56 through D-21-60** applied as planned; see frontmatter.
- **The S1 fixture is `TAG_PAIR`, not a zero-width pair.** See deviations.

## Deviations from Plan

### 1. [Rule 1 — Bug in the plan's own guidance] The S1 fixture had to be the TAG pair, not a zero-width pair

- **Found during:** Task 3.
- **Issue:** The plan's action (b) directs that if the chosen screen's widget family drops zero-width graphemes, the spot-check should *"rely on the invisible-character-count assertion rather than on a raw-form absence assertion."* For `EnqueueScreen`'s `Paragraph` footer that fallback is **itself vacuous**: if the family drops zero-width graphemes before a cell exists, then a count of invisible characters in the buffer is zero **whether or not the escape ran**. The control could never go red — which is prohibition 5's own stated failure mode ("a control that cannot fail — this round's own subject"), reached by following the instruction meant to avoid it.
- **Fix:** Chose `LOOK_ALIKE_PAIRS[TAG_PAIR]`, whose hostile member carries `U+E0041` — a character this tree has already measured as **preserved** by both `Paragraph` and `ListItem`. The invisible-character-count assertion is then non-vacuous, which the RED proves: the reverted escape produced `['\u{e0041}']` in the buffer.
- **Also added:** a second hostile-direction arm asserting the expected escaped form is *present*, so a value that was **dropped** rather than escaped cannot pass as escaped. Without it, "zero invisible characters" is satisfiable by disappearance.
- **Verification:** RED captured verbatim; the surviving character is the tag character, confirming the family preserves it.
- **Committed in:** `5400e44`.

### 2. [Rule 2 — Missing critical] The old test name had to be elided from two verbatim committed REDs and one quoted filter command

- **Found during:** Task 1.
- **Issue:** Task 1's acceptance criterion requires `grep -c "every_render_site_under_ui_composes_both_classes" src/ui/mod.rs` → 0, but the old name appeared inside two **verbatim** committed panic outputs, which the same task is told to keep ("deleting evidence is not narrowing").
- **Fix:** The `thread '<path>'` token in each panic is elided to `<renamed by 21-33>`, with a **declared** note stating that this token is the only alteration and every site list, count and `left`/`right` value is byte-verbatim. The quoted filter command was updated to the new name and marked as updated. The narrowed-claim doc describes what the old name asserted ("every render site under `src/ui/` composes both classes") in words rather than as the identifier, and points at this SUMMARY for the exact old spelling.
- **The exact old identifier, recorded here so nothing is lost:** `every_render_site_under_ui_composes_both_classes`.
- **Verification:** `grep -c` → 0 for the old name, 2 for the new.
- **Committed in:** `9eacc82`.

### 3. [Rule 1 — Bug] A neighbouring control's comment became false when the tiebreak landed

- **Found during:** Task 2 step (g).
- **Issue:** `a_non_numeric_suffix_cannot_make_the_order_depend_on_the_input_permutation` carried a comment arguing that the tied group's **element** sequence could not be asserted, because `sort_by` is stable and rotation reorders ties. True before the tiebreak — and the defect itself. False after it.
- **Fix:** Comment corrected in place, dated to `21-33`, quoting what it corrects and pointing at the new control. The key-sequence assertion was **kept rather than widened**, so the two controls stay separable: one fails if the KEY order regresses, the other if the tiebreak is removed.
- **Verification:** all 9 backlog tests green; no asserted order changed.
- **Committed in:** `43b7c93`.

### 4. [Rule 3 — Blocking] `super::tests::ctx_with_aliases` does not resolve from the nested `tests` module

- **Found during:** Task 3.
- **Issue:** The existing call sites at `:982`/`:1191` are at the `render_escape_guard` module level, where `super::` is `ui::screens`. My helper lives inside `render_escape_guard::tests`, one level deeper, so `super::tests` resolved to `render_escape_guard::tests` and failed to compile (`E0425`).
- **Fix:** `super::super::tests::ctx_with_aliases`, with a comment naming the grandparent relationship. **No visibility changed** — the F3 decline holds exactly as measured.
- **Verification:** compiles and passes; `grep -c "pub(super) fn ctx_with_aliases"` still 1.
- **Committed in:** `5400e44`.

---

**Total deviations:** 4 auto-fixed (2 × Rule 1, 1 × Rule 2, 1 × Rule 3).
**Impact on plan:** No scope creep. Deviation 1 is the significant one — the plan's own fallback guidance would have produced a control that could not fail, and catching it required measuring the widget family rather than trusting the instruction. All four are inside the four declared files.

## Issues Encountered

- **The reach pin failed on its own author.** Documented in full above. Resolved by applying the repair the pin's failure message prescribes, in one commit. This is the mechanism working, not a defect.
- **`driver_reattach` failed 2/3 in the full-workspace run, green 3/3 in isolation.** The documented flake; three sibling executor worktrees running concurrently is exactly the `/proc`-scan trigger the corrected record describes. Reported rather than absorbed.

## Known Stubs

None. No placeholder values, no skipped tests, no unrun `<verify>` commands. All three task-level `<verify>` commands were executed under `rtk proxy` and their counts are quoted above.

## Residuals left standing, each with its failure direction

1. **The census's reach shrinks as the conversion succeeds** — a render site applying neither class is invisible to it. **Under-detection, silent, growing over time.** Bounded by the sealed `RenderAdjudicated` bound and the behavioural probes, both named in the doc; the reach itself is now pinned so the shrink goes red when it happens.
2. **Alias/re-export invisibility** in the census (pre-existing). **Under-detection, silent.**
3. **Over-length composition** read as un-composed (pre-existing). **Over-detection, loud.**
4. **The trait probe sees the test binary's view** — a feature-gated impl would be invisible. **Under-detection, disclosed**; bounded by coherence, since `EditBuffer` is crate-local and all three traits are foreign.
5. **The S1 spot-check is one screen and one fixture pair.** The other four input-echo screens are covered only by the weaker per-screen probes. **Under-detection, silent, bounded by those probes and not by this test.**
6. **ROADMAP criterion 4's behavioural half** — permanently agent-unclosable by explicit user decision. No work claimed against it; 4/5 remains the expected ceiling.

## Next Phase Readiness

- Wave 1 slice complete; `21-34` (wave 2) owns `deferred-items.md`, `COVERAGE.md` and `tests/driver_reattach.rs`, none of which this plan touched.
- **For the post-merge step:** the reach pin asserts the per-file distribution across all of `src/ui/`, which includes `21-31`'s `detail.rs` (0 at HEAD) and `21-32`'s `driver.rs` (2 at HEAD). A red in the pin after the merge is **informative, not an accident** — it says a sibling changed the census's reach and the disclosure is stale. The repair is to re-measure, update `MEASURED_REACH` and update the disclosed table in one commit; never to relax the assertion.
- `.planning/STATE.md`, `.planning/ROADMAP.md` and `.planning/REQUIREMENTS.md` deliberately untouched — the orchestrator owns those writes.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-27*
