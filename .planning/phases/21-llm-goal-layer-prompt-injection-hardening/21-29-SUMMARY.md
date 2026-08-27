---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 29
subsystem: ui
tags: [ratatui, unicode, escaping, census, total-order, tui]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "crate::text::render_for_terminal — the ONE composition of the control class and the invisible-formatting class (round 9, 21-23)"
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "crate::test_support::LOOK_ALIKE_PAIRS — the shared look-alike fixture list every seam pin consumes"
provides:
  - "Every executable render site in the nine `src/ui/` files this plan owns composes both classes through `render_for_terminal`"
  - "A committed census in `src/ui/mod.rs` asserting the one-composition claim over `src/ui/` as an equality on a count, with two named exemptions, a non-ban control arm, a self-match control and a self-cleaning stale-exemption report"
  - "The ratatui grapheme-filtering residual stated with its failure direction AND measured in a buffer-level control rather than quoted"
  - "`parse_backlog_items` sorting on `f64::total_cmp` over a finite-filtered key — a total order by construction, with antisymmetry and transitivity swept"
  - "The Debug-notation control asserting per invisible character, and a two-invisible-character fixture in the shared list that closes the concatenation trap for every consumer"
affects: [21-27, 21-28, 21-30, phase-21-verification, phase-21-close]

actuals:
  tokens: 19880
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "Composition census over a directory: an equality on the count of un-composed constructions, needle assembled at runtime from halves meaningless apart, with a non-ban arm and a self-match control"
    - "WAVE_PENDING: a violation owned by a parallel plan is SUBTRACTED (merge-safe in both directions) rather than exempted, with its failure direction stated"
    - "Two-direction conversion pin: every direction compares against an oracle that is the PRE-conversion formulation, never against the new code itself"

key-files:
  created: []
  modified:
    - src/ui/mod.rs
    - src/ui/screens/normal.rs
    - src/ui/roadmap_widget.rs
    - src/ui/screens/add_project.rs
    - src/ui/screens/create_project.rs
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/driver_start.rs
    - src/ui/screens/driver_inject.rs
    - src/ui/screens/enqueue.rs
    - src/ui/screens/queue_delete_confirm.rs
    - src/state_reader/backlog.rs
    - src/error.rs
    - src/test_support.rs

key-decisions:
  - "D-21-42: WR-05 closed by converting every site AND landing a census, not by narrowing render_for_terminal's doc"
  - "D-21-43: the census lives in src/ui/mod.rs — the module root of the directory it walks, and the only census-shaped home not fenced to a parallel plan this wave"
  - "D-21-44: the census's rule is about the un-composed CONSTRUCTION, with a non-ban control arm, not about the identifier"
  - "D-21-45: WR-07 fixed with f64::total_cmp over a finite-filtered key and tested on ORDER; the reviewer's panic claim recorded as REFUTED and attributed to verification pass 10"
  - "D-21-46: WR-08's two-invisible-character fixture added to the SHARED LOOK_ALIKE_PAIRS; every consumer re-run and named, and the absence of a second concatenation-shaped control is a MEASURED result"
  - "New, taken at execution time: the census is committed AFTER both conversion commits so that every commit in this plan is green; its RED is captured verbatim against the pre-conversion tree instead of being left uncommitted"
  - "New, taken at execution time: WAVE_PENDING subtracts driver_confirm.rs:275 rather than exempting driver_confirm.rs, so the equality holds both in this worktree and on the post-merge tree"

patterns-established:
  - "A claim in a doc is closed by a control that goes red against the unfixed tree, not by rewording the doc"
  - "An oracle for a 'behaviour-preserving' assertion must be the OLD formulation; comparing the new code to itself proves nothing"
  - "A fixture list that every control consumes is where a shape-of-control defect is closed once for the whole tree"

requirements-completed: [SAFE-07, DRIVE-03]

coverage:
  - id: D1
    description: "Every executable render site in src/ui/screens/normal.rs composes both classes through render_for_terminal (6 sites: phase display, workstream name, milestone, status, alias cell, filter footer)"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/ui/mod.rs#every_render_site_under_ui_composes_both_classes"
        status: pass
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every executable render site in the eight remaining ui files (roadmap_widget, add_project, create_project, delete_confirm, driver_start, driver_inject, enqueue, queue_delete_confirm) composes both classes"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/ui/mod.rs#every_render_site_under_ui_composes_both_classes"
        status: pass
    human_judgment: false
  - id: D3
    description: "The one-composition claim is a committed control: an equality on a count over src/ui/, observed RED at 23 sites against the unconverted tree and RED again by planting a bare call in help.rs"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/ui/mod.rs#every_render_site_under_ui_composes_both_classes"
        status: pass
      - kind: unit
        ref: "src/ui/mod.rs#the_census_cannot_report_itself"
        status: pass
      - kind: unit
        ref: "src/ui/mod.rs#the_census_does_not_report_a_composed_call"
        status: pass
      - kind: unit
        ref: "src/ui/mod.rs#a_stale_exemption_is_reported"
        status: pass
    human_judgment: false
  - id: D4
    description: "The conversion is proven behaviour-preserving on clean values and proven CHANGED on control-carrying values, at three levels: function results, a real ratatui buffer, and queue_delete_confirm's exact prompt string"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/ui/mod.rs#the_conversion_is_a_no_op_on_clean_values_and_is_not_on_control_values"
        status: pass
      - kind: unit
        ref: "src/ui/mod.rs#the_roadmap_widget_renders_a_clean_phase_name_unchanged_and_a_control_one_differently"
        status: pass
      - kind: unit
        ref: "src/ui/mod.rs#the_queue_delete_prompt_is_unchanged_for_a_clean_command_and_changed_for_a_control_one"
        status: pass
    human_judgment: false
  - id: D5
    description: "parse_backlog_items sorts on a total order by construction; a .planning/phases/ directory name with a non-numeric suffix can no longer make the Backlog tab's order depend on read_dir's order"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/state_reader/backlog.rs#a_non_numeric_suffix_cannot_make_the_order_depend_on_the_input_permutation"
        status: pass
      - kind: unit
        ref: "src/state_reader/backlog.rs#the_backlog_comparator_is_antisymmetric_and_transitive_including_over_a_nan_suffix"
        status: pass
      - kind: unit
        ref: "src/state_reader/backlog.rs#well_formed_backlog_numbers_keep_the_order_they_had_before_the_total_order_fix"
        status: pass
      - kind: unit
        ref: "src/state_reader/backlog.rs#a_non_finite_or_unparsable_suffix_takes_the_same_fallback_key"
        status: pass
    human_judgment: false
  - id: D6
    description: "The Debug-notation control can no longer go red for a correct implementation, proven by the two-invisible-character fixture that made it do so; every consumer of the shared list re-run"
    requirement: "DRIVE-03"
    verification:
      - kind: unit
        ref: "src/error.rs#the_debug_route_of_every_alias_carrying_variant_uses_this_projects_own_notation"
        status: pass
      - kind: integration
        ref: "cargo test --workspace --no-fail-fast (every LOOK_ALIKE_PAIRS consumer)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The ratatui grapheme-filtering mitigation is written down as DEPENDENCY BEHAVIOUR with its failure direction, and measured rather than quoted"
    verification:
      - kind: unit
        ref: "src/ui/mod.rs#the_roadmap_widget_renders_a_clean_phase_name_unchanged_and_a_control_one_differently"
        status: pass
    human_judgment: true
    rationale: "The buffer-level control measures that ratatui drops a C1 grapheme TODAY. Whether the residual is adequately DISCLOSED — the prose in the census doc, its direction, and its pointer to the standing deferred-items entry — is a judgment about a written record, not something a test can assert. A reviewer must read the doc."
  - id: D8
    description: "The two-direction spot-check on an INPUT-ECHO screen (as distinct from a .planning/-derived value)"
    verification: []
    human_judgment: true
    rationale: "NOT DELIVERED. Both spot-checks landed on .planning/-derived values (ROADMAP.md via roadmap_widget, queue.md via queue_delete_confirm). Screen::render needs a Frame and an AppContext; the tree's only AppContext fixture is `pub(super) fn ctx_with_aliases` inside `ui::screens::tests`, a module fenced to 21-28 this wave, and AppContext has no constructor — a sixth full-field literal is the anti-pattern that fixture's own doc exists to prevent. Follow-up named in Deferred Issues."

duration: 40 min
completed: 2026-08-27
status: complete
---

# Phase 21 Plan 29: One Composition, One Total Order, One Fixture Summary

**`render_for_terminal`'s ONE-composition claim made true of `src/ui/` at 23 render sites and enforced by a committed census that goes red for a planted bare call; the Backlog sort made a total order by construction with the reviewer's panic claim refuted in the record; and the Debug-notation control stopped from going red for a correct implementation by the two-invisible-character fixture that made it do so.**

## Performance

- **Duration:** 40 min
- **Started:** 2026-08-27T20:31Z
- **Completed:** 2026-08-27T21:11Z
- **Tasks:** 3
- **Files modified:** 13

## Accomplishments

- **23 executable render sites** across nine `src/ui/` files converted from `display_identity` alone to `crate::text::render_for_terminal`. Every one of them draws a value this build did not author — `.planning/`-derived project state, registry keys, operator keystrokes, `.planning/queue.md` commands, `ROADMAP.md` phase names.
- **The claim stopped being a sentence.** A census in `src/ui/mod.rs` walks `src/ui/` recursively, drops comment lines, joins wrapped calls into one logical unit and asserts an EQUALITY on the count of un-composed executable calls. It was observed RED three ways.
- **The mitigation the old state depended on is now written down AND measured.** ratatui's control-grapheme filtering is stated in the census's doc as dependency behaviour with its failure direction, and a buffer-level control measures it directly instead of quoting a changelog.
- **`parse_backlog_items`' comparator is a total order by construction** — `f64::total_cmp` over a finite-filtered key, with no fallback arm — and antisymmetry and transitivity are swept over every pair and triple of a fixture set including the hostile input.
- **WR-08's trap is closed for the whole tree, not for one test**, by adding the two-invisible-character fixture to the shared `LOOK_ALIKE_PAIRS` and re-running every consumer.

## Task Commits

1. **Task 1 — normal.rs conversions** — `b3dbe4b` (feat)
2. **Task 2 — the eight remaining ui files** — `44fa6c2` (feat)
3. **Task 1 — the census + the two-direction pins** — `91b271f` (test)
4. **Task 3 — the backlog total order** — `91c27c2` (fix)
5. **Task 3 — the Debug notation per character + the shared fixture** — `f1d0b1f` (test)

## Files Created/Modified

- `src/ui/mod.rs` — the composition census (+743 lines), its two named exemptions, the WAVE_PENDING register, the non-ban arm, the self-match control, the self-cleaning stale-exemption report, and three two-direction conversion pins
- `src/ui/screens/normal.rs` — 6 render sites converted
- `src/ui/roadmap_widget.rs` — 2 sites converted; width arithmetic deliberately untouched with a pointer to the IN-02/IN-03 deferral
- `src/ui/screens/add_project.rs`, `create_project.rs`, `delete_confirm.rs`, `driver_start.rs`, `driver_inject.rs`, `enqueue.rs` — 15 sites converted
- `src/ui/screens/queue_delete_confirm.rs` — 1 site converted; prompt extracted into `prompt_text` so the pin can drive the exact rendered string
- `src/state_reader/backlog.rs` — `backlog_sort_key` (finite-filtered) and `backlog_number_ordering` (`total_cmp`), plus four order controls
- `src/error.rs` — the Debug-notation assertion rewritten per character
- `src/test_support.rs` — `LOOK_ALIKE_PAIRS` index 6: `("demo", "d\u{200b}emo\u{00ad}")`

---

## Re-measured gates — this plan's figures beside the plan's own

Every command under `rtk proxy`. **No number below is inherited from the plan's text, `21-REVIEW.md` or `21-VERIFICATION.md`.**

| Gate | Plan said (at `b1d0478`) | Measured at base `80bc4c1` | Measured at `f1d0b1f` |
|---|---|---|---|
| `cargo build` | exit 0 | exit 0 | exit 0 |
| `cargo test --workspace --no-fail-fast` | 1387 / 0 / 13 | 1387 / 0 / 13 | **1398 / 0 / 13** |
| `cargo clippy -- -D warnings` | exit 0 | exit 0 | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | 4 lints: `bool_assert_comparison` ×3 at `browser.rs:131,132,133`; `cmp_owned` ×1 at `project_creator.rs:146` | 4 lints, same two kinds, **`browser.rs:155,156,157`**, `project_creator.rs:146` | 4 lints, same two kinds, same two files, same lines |

**Delta on the test total: +11, fully attributed** — 7 new `ui::tests` (census equality, self-match, non-ban, stale-exemption, generic two-direction pin, roadmap-widget buffer pin, queue-delete prompt pin) + 4 new `state_reader::backlog::tests` (permutation-independence, total-order sweep, no-regression, fallback key). The WR-08 fixture adds loop ITERATIONS inside existing tests, not new test functions, so it contributes 0 to the count.

**Correction to a number in the plan's own text.** The plan's `<tooling_note>` places the three `bool_assert_comparison` lints at `src/browser.rs:131,132,133`. Re-measured at `80bc4c1` and at `f1d0b1f` they are at **`155,156,157`**. `src/browser.rs` is not in this plan's diff (see `git diff --stat` below), so this is a stale figure in the plan, not a change this plan made. The load-bearing property — exactly four, same two kinds, same two files, before and after — holds.

### Per-file `display_identity(` under `src/ui/`, split into executable calls and doc mentions

**Split method:** a line whose content after `trim_start()` begins with `//` (which covers `//`, `///` and `//!`) is a doc/comment mention; every other line is executable. This is the same rule the census uses to drop comment lines, so the table and the control agree by construction. The plan's `<tooling_note>` table did NOT make this split and quoting it unsplit would have been inheriting the plan's own number.

| File | BEFORE (`80bc4c1`) exe / doc | AFTER (`f1d0b1f`) exe / doc | Owner |
|---|---|---|---|
| `src/ui/screens/normal.rs` | 6 / 0 | **0 / 0** | this plan |
| `src/ui/screens/create_project.rs` | 4 / 0 | **0 / 0** | this plan |
| `src/ui/screens/delete_confirm.rs` | 3 / 0 | **0 / 0** | this plan |
| `src/ui/screens/driver_start.rs` | 3 / 0 | **0 / 0** | this plan |
| `src/ui/roadmap_widget.rs` | 2 / 0 | **0 / 0** | this plan |
| `src/ui/screens/add_project.rs` | 2 / 0 | **0 / 0** | this plan |
| `src/ui/screens/driver_inject.rs` | 1 / 0 | **0 / 0** | this plan |
| `src/ui/screens/enqueue.rs` | 1 / 0 | **0 / 0** | this plan |
| `src/ui/screens/queue_delete_confirm.rs` | 1 / 0 | **0 / 0** | this plan |
| `src/ui/screens/driver.rs` | 2 / 0 | 2 / 0 (both COMPOSED) | 21-28 |
| `src/ui/screens/driver_confirm.rs` | 3 / 0 | 3 / 0 (2 composed, **1 bare at `:275`**) | 21-28 |
| `src/ui/screens/render_escape_guard.rs` | 1 / 0 | 1 / 0 (**exempt** probe) | 21-28 |
| `src/ui/screens/mod.rs` | 0 / 3 | 0 / 3 | 21-28 |
| `src/ui/screens/detail.rs` | 0 / 1 | 0 / 1 | 21-27 |

**Total executable un-composed calls in files this plan owns: 23 → 0.**

Note the plan's table quoted `driver_confirm.rs` at 6 occurrences and `mod.rs`/`detail.rs` counts that include mentions without a following `(`; the numbers above count the call construction `display_identity(` specifically, which is what the census matches.

### `render_for_terminal` under `src/ui/`, split, with the delta attributed site by site

| File | BEFORE exe | AFTER exe | Delta, attributed |
|---|---|---|---|
| `src/ui/screens/normal.rs` | 1 | **7** | +6: `:697` phase display, `:709` workstream name, `:749` milestone, `:763` status, `:779` alias cell, `:1003` filter footer. The pre-existing 1 is `render_footer`'s status branch, landed by 21-25 and untouched. |
| `src/ui/screens/create_project.rs` | 0 | 4 | +4: `:88`/`:89` confirm prompt name and path, `:265` input echo, `:274` error echo |
| `src/ui/screens/delete_confirm.rs` | 0 | 3 | +3: `:90` alias prompt, `:129` liveness refusal, `:183` removal toast |
| `src/ui/screens/driver_start.rs` | 0 | 3 | +3: `:340`/`:356` input echoes, `:352` committed-command hint |
| `src/ui/roadmap_widget.rs` | 0 | 2 | +2: phase number prefix, phase name |
| `src/ui/screens/add_project.rs` | 0 | 2 | +2: input echo, error echo |
| `src/ui/screens/driver_inject.rs` | 0 | 1 | +1: input echo |
| `src/ui/screens/enqueue.rs` | 0 | 1 | +1: input echo |
| `src/ui/screens/queue_delete_confirm.rs` | 0 | 1 | +1: queued command (now inside `prompt_text`) |
| `src/ui/mod.rs` | 0 | 5 | +5: the two-direction pins' own calls (test code) |
| `src/ui/screens/detail.rs`, `driver.rs`, `mod.rs`, `render_escape_guard.rs` | unchanged | unchanged | not this plan's |

`rtk proxy grep -c "render_for_terminal" src/ui/screens/normal.rs` = **8** (7 executable + 1 doc mention), against the acceptance floor of 7.

### `git diff --stat 80bc4c1 HEAD`

```
 src/error.rs                           |  50 ++-
 src/state_reader/backlog.rs            | 276 +++++++++++-
 src/test_support.rs                    |  27 +-
 src/ui/mod.rs                          | 743 +++++++++++++++++++++++++++++++++
 src/ui/roadmap_widget.rs               |  29 +-
 src/ui/screens/add_project.rs          |   8 +-
 src/ui/screens/create_project.rs       |  10 +-
 src/ui/screens/delete_confirm.rs       |  11 +-
 src/ui/screens/driver_inject.rs        |   5 +-
 src/ui/screens/driver_start.rs         |  11 +-
 src/ui/screens/enqueue.rs              |   8 +-
 src/ui/screens/normal.rs               |  36 +-
 src/ui/screens/queue_delete_confirm.rs |  63 ++-
 13 files changed, 1202 insertions(+), 75 deletions(-)
```

Exactly the thirteen declared files. **None of prohibition 4's fenced files** (`detail.rs`, `text.rs`, `driver.rs`, `driver_confirm.rs`, `src/ui/screens/mod.rs`, `render_escape_guard.rs`), **neither `src/browser.rs` nor `src/project_creator.rs`**, and `git diff --stat 80bc4c1 HEAD -- tests/driver_injection_corpus.rs .planning/REQUIREMENTS.md src/browser.rs src/project_creator.rs` is **empty**.

---

## The three REDs, verbatim

### RED 1 — the one-composition claim measured FALSE of the tree

The strongest of the three, because it is the defect itself rather than a plant. Census run against `80bc4c1` before any conversion (`rtk proxy cargo test --lib -- ui::tests::every_render_site --nocapture`):

```text
thread 'ui::tests::every_render_site_under_ui_composes_both_classes' (1996514) panicked at src/ui/mod.rs:374:9:
assertion `left == right` failed: 23 executable call sites under src/ui/ apply the invisible-formatting half alone, and `crate::text::render_for_terminal`'s doc claims to be THE one composition of the two classes. Sites: ["src/ui/roadmap_widget.rs:131", "src/ui/roadmap_widget.rs:136", "src/ui/screens/add_project.rs:243", "src/ui/screens/add_project.rs:256", "src/ui/screens/create_project.rs:88", "src/ui/screens/create_project.rs:89", "src/ui/screens/create_project.rs:265", "src/ui/screens/create_project.rs:274", "src/ui/screens/delete_confirm.rs:90", "src/ui/screens/delete_confirm.rs:129", "src/ui/screens/delete_confirm.rs:183", "src/ui/screens/driver_inject.rs:197", "src/ui/screens/driver_start.rs:340", "src/ui/screens/driver_start.rs:352", "src/ui/screens/driver_start.rs:356", "src/ui/screens/enqueue.rs:124", "src/ui/screens/normal.rs:697", "src/ui/screens/normal.rs:709", "src/ui/screens/normal.rs:749", "src/ui/screens/normal.rs:763", "src/ui/screens/normal.rs:779", "src/ui/screens/normal.rs:1003", "src/ui/screens/queue_delete_confirm.rs:104"]. A site that applies one half is open in the other direction: `General_Category=Cf` union `Default_Ignorable_Code_Point` is not `Cc`, and `ESC` is `Cc` and in neither of the first two. The repair is to call `crate::text::render_for_terminal`, or to compose on the same logical line with `sanitize_render_line` where the display cap is also wanted — not to soften the claim.
  left: 23
 right: 0
```

Twenty-three sites, in exactly the nine files this plan converted.

`rtk proxy git status --porcelain` after restoring the working copy: only ` M src/ui/mod.rs` (the census itself, not yet committed).

### RED 2 — the planted bare call

One bare call planted in `src/ui/screens/help.rs`, a file this plan does not otherwise touch:

```text
CENSUS raw un-composed sites under src/ui/ (3): ["src/ui/screens/driver_confirm.rs:275", "src/ui/screens/help.rs:182", "src/ui/screens/render_escape_guard.rs:2019"]

thread 'ui::tests::every_render_site_under_ui_composes_both_classes' (2088034) panicked at src/ui/mod.rs:377:9:
assertion `left == right` failed: 1 executable call sites under src/ui/ apply the invisible-formatting half alone, and `crate::text::render_for_terminal`'s doc claims to be THE one composition of the two classes. Sites: ["src/ui/screens/help.rs:182"]. ...
  left: 1
 right: 0
```

The same plant was then REWRITTEN as a **wrapped composition** at the same site — the call on one physical line, `sanitize_render_line` on the next — and the census did NOT report it:

```text
CENSUS raw un-composed sites under src/ui/ (2): ["src/ui/screens/driver_confirm.rs:275", "src/ui/screens/render_escape_guard.rs:2019"]
test ui::tests::every_render_site_under_ui_composes_both_classes ... ok
```

That single pair of runs measures the join and the non-ban direction together: the same identifier, at the same line, reported when un-composed and not reported when composed.

Both plants removed. `rtk proxy git status --porcelain`:

```text
 M src/ui/mod.rs
```

`src/ui/screens/help.rs` is absent from that output — the plant left no trace.

### RED 3 — the backlog order under the pre-fix comparator

```text
thread 'state_reader::backlog::tests::a_non_numeric_suffix_cannot_make_the_order_depend_on_the_input_permutation' (2153211) panicked at src/state_reader/backlog.rs:299:13:
assertion `left == right` failed: sorting the same set from rotation 1 produced ["999.NaN", "999.1", "999.2", "999.3", "999.4", "999.10"], but from rotation 0 it produced ["999.1", "999.NaN", "999.2", "999.3", "999.4", "999.10"]. A comparator whose answer depends on the input permutation is not a total order, and the Backlog tab's order is then a function of whatever `read_dir` happened to return first.
  left: ["999.NaN", "999.1", "999.2", "999.3", "999.4", "999.10"]
 right: ["999.1", "999.NaN", "999.2", "999.3", "999.4", "999.10"]
```

and, from the total-order sweep, the shape named exactly:

```text
thread 'state_reader::backlog::tests::the_backlog_comparator_is_antisymmetric_and_transitive_including_over_a_nan_suffix' (2153216) panicked at src/state_reader/backlog.rs:392:25:
assertion `left == right` failed: transitivity of equality failed: "999.1" == "999.NaN" == "999.2" but "999.1" vs "999.2" is Less. This is the exact shape the pre-fix comparator had — `NaN` compared Equal to everything while the finite keys around it did not compare equal to each other.
  left: Less
 right: Equal
```

**This is a wrong ORDER and NOT a panic**, and that is not this executor's opinion. Verification pass 10, quoted verbatim from `21-VERIFICATION.md` (and reproduced in `backlog_number_ordering`'s doc so it travels with the code):

> "I built and ran a standalone Rust program (rustc 1.97.1, matching this toolchain) sorting a `Vec<f64>` containing multiple `NaN` values with the exact comparator shape used in `backlog.rs:88-106` (`partial_cmp(...).unwrap_or(Equal)`), at both small (5-element) and larger (2000-element, 1/3 NaN) sizes. Neither run panicked; both produced a silently-wrong order with NaNs interspersed. Rust's stable `slice::sort_by` does NOT panic on a non-total-order comparator on this toolchain — WR-07's specific claim ('Rust's current slice::sort_by detects total-order violations and panics') is not reproducible and is likely incorrect, possibly confusing Rust with Java's TimSort."

`rtk proxy git status --porcelain` after restoring `total_cmp`: `M src/state_reader/backlog.rs` only.

### RED 4 (WR-08) — the concatenation trap, fired by the two-character fixture

```text
thread 'error::tests::the_debug_route_of_every_alias_carrying_variant_uses_this_projects_own_notation' (2104544) panicked at src/error.rs:1248:17:
OptInError::UnknownAlias's `{:?}` does not carry "U+200BU+00AD" — this project's own notation for "d\u{200b}emo\u{ad}". It reads "UnknownAlias { alias: Untrusted(\"dU+200BemoU+00AD\") }" instead, which means the escape came from `core::char::is_printable` (an unpinned std table that is a second spelling of a class this project derives) rather than from `crate::text::Untrusted`'s hand-written `Debug`
```

Read what that says: **the implementation is correct.** `dU+200BemoU+00AD` is exactly the right rendering. The test demanded `U+200BU+00AD`, a form a correct rendering never produces because the markers are separated by the visible characters between them. A control that goes red for a right implementation is worse than no control, and this one had been green for six rounds only because every fixture happened to carry exactly one invisible character.

`rtk proxy git status --porcelain` at that point: ` M src/error.rs`, ` M src/test_support.rs`, ` M src/state_reader/backlog.rs`.

---

## The census, and what it actually reports

Full output of the live census, quoted (`rtk proxy cargo test --lib -- ui::tests --nocapture`):

```text
running 7 tests
test ui::tests::the_queue_delete_prompt_is_unchanged_for_a_clean_command_and_changed_for_a_control_one ... ok
test ui::tests::the_conversion_is_a_no_op_on_clean_values_and_is_not_on_control_values ... ok
test ui::tests::the_roadmap_widget_renders_a_clean_phase_name_unchanged_and_a_control_one_differently ... ok
CENSUS raw un-composed sites under src/ui/ (2): ["src/ui/screens/driver_confirm.rs:275", "src/ui/screens/render_escape_guard.rs:2019"]
STALE EXEMPTION: src/ui/screens/help.rs no longer contains an un-composed call, so the exemption shields nothing and must be removed. Its recorded reason was: a synthetic exemption on a file with nothing to exempt
test ui::tests::every_render_site_under_ui_composes_both_classes ... ok
test ui::tests::the_census_cannot_report_itself ... ok
test ui::tests::a_stale_exemption_is_reported ... ok
test ui::tests::the_census_does_not_report_a_composed_call ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 1073 filtered out; finished in 0.05s
```

**Zero for every file this plan owns.** The two raw sites are:

| Site | Disposition |
|---|---|
| `src/ui/screens/render_escape_guard.rs:2019` | **EXEMPT #1.** The probe module computes the EXPECTED escaped form of a hostile fixture and asserts a rendered screen buffer against it. Converting it would make the probe assert the implementation against itself. |
| `src/ui/screens/driver_confirm.rs:275` | **WAVE_PENDING — a post-merge finding, REPORTED not fixed.** `prompt_text`'s alias escape. `driver_confirm.rs` is fenced to **21-28**, which runs in a parallel worktree in this same wave and carries the matching acceptance criterion for exactly this site. |

The second exemption is `src/text.rs` — `display_identity`'s own definition and `render_for_terminal`'s composition — which is **outside the `src/ui/` walk**, listed in the census's doc so its absence is a stated exemption rather than something a reader has to rediscover.

**Why the zero is not vacuous, in one sentence:** the census asserts `!files.is_empty()` before counting, it reported 23 sites against the very tree this plan started from, and it reported a planted call at `help.rs:182` in a file this plan never edits — so the walk demonstrably looks, and the equality demonstrably fires.

**The non-ban arm's fixture, named.** `the_census_does_not_report_a_composed_call` does not spell a composition as a literal; it reads every real in-tree composition off disk and requires that none is reported. At `f1d0b1f` those are `src/ui/screens/driver.rs:615`, `src/ui/screens/driver.rs:914`, `src/ui/screens/driver_confirm.rs:279` and `src/ui/screens/driver_confirm.rs:319` — the deliberate SECOND composition (`display_identity(&sanitize_render_line(..))`), which additionally applies the `DRIVER_OUTPUT_LINE_CELLS` cap. Using the tree's own compositions rather than a copy means the arm cannot drift from what it certifies.

**The stale-exemption report, exercised.** Its output is quoted above. It is produced by driving the SAME extracted `stale_exemptions` function the live census consumes, with a synthetic exemption on `src/ui/screens/help.rs` — a file under the walk that contains no matching call. The same function is then run against the real `EXEMPTIONS` list and must return empty.

---

## The two-direction conversion pins

Three of them, at increasing levels of realism. **Every direction compares against an oracle that is the PRE-conversion formulation** — never against the new code itself, which was the mistake that would have made all three vacuous.

| Level | Clean fixture | Control fixture | Result |
|---|---|---|---|
| Function results (`the_conversion_is_a_no_op_..`) | all 14 members of `LOOK_ALIKE_PAIRS` + `"demo"`, `"2.1"`, `"20"`, `"/gsd:progress"`, `"2026-08-19T12-00-00Z-aaaa"`, `"clean /tmp/project"` | `"demo\u{1b}[31mred"`, `"a\tb"`, `"x\u{7f}y"`, `"gsd\u{9b}run"` | equal on clean, NOT equal on control |
| A real ratatui `Buffer`, through `RoadmapWidget` (`the_roadmap_widget_..`) | name `"Injection Hardening"`, number `"21"` | name `"Injection\u{7f} Hardening"`, number `"2\u{9b}1"` | clean cells carry the pre-conversion rendering verbatim; control cells differ AND carry the visible `CONTROL_REPLACEMENT` (`U+00B7`) |
| `queue_delete_confirm::prompt_text` (`the_queue_delete_prompt_..`) | `"/gsd:execute-phase 21"` | `"/gsd:execute-phase\u{9b}2K 21"` | byte-identical on clean; different on control; no `U+009B` survives |

The clean direction additionally asserts that each clean fixture is a **fixed point of `strip_terminal_controls`**, so the equality cannot pass because the added half quietly did something.

The middle pin also **measures the dependency behaviour** rather than quoting it: two one-line ratatui buffers, one filled with `"2\u{9b}1"` and one with `"21"`, are asserted EQUAL. That is the whole basis for calling an un-composed site a claim defect rather than a live leak, and it now lives in the tree as a control that will go red if ratatui ever changes.

`roadmap_widget.rs`'s truncation is measurably unchanged: the same input produces the same visible width and the same ellipsis placement. `git diff 80bc4c1 HEAD -- src/ui/roadmap_widget.rs` touches only the two escape calls and a comment block; `inner_width`, `name_max`, `prefix.chars().count()`, `suffix.chars().count()` and the `take(name_max.saturating_sub(3))` truncation are byte-for-byte identical. A comment at the site now points at the IN-02/IN-03 deferral entry so the untouched `chars().count()`-as-display-width arithmetic reads as a recorded deferral rather than an oversight.

---

## D-21-46's blast radius, MEASURED

Every consumer of `LOOK_ALIKE_PAIRS` was re-run with the two-character fixture in place **while `error.rs`'s old concatenated assertion was still standing** — the only configuration in which a second instance of the same trap could reveal itself.

| Consumer | Re-run result |
|---|---|
| `src/registry.rs` | pass |
| `src/error.rs` | **FAIL** — the WR-08 trap, quoted above |
| `src/driver/mod.rs` | pass |
| `src/text.rs` | pass |
| `src/envelope/mod.rs` | pass |
| `src/ui/screens/mod.rs` | pass |
| `src/ui/screens/render_escape_guard.rs` | pass |
| `src/ui/mod.rs` | pass |
| `src/journal/mod.rs` | pass |
| `tests/registry_test.rs` | pass |
| `tests/spawn_seam_guard.rs` | doc mention only — no iteration over the list, nothing to re-run |

`cargo test --workspace --no-fail-fast` in that configuration: **1393 passed / 1 failed / 13 ignored**, and the one failure was `error.rs`'s.

**There is no second instance of the concatenation shape in this tree. That is a measured result, not an assumption**, and producing it is what taking the shared-fixture path (rated `costly`) bought. Had a second existed, it would be reported here with file and line rather than quietly patched.

`rtk proxy grep -o "u{" src/test_support.rs | wc -l`: **14 → 16** (+2, the new fixture's two escapes; `grep -c` counts lines and reads 14 → 15 because both escapes are on one line). The fixture is `("demo", "d\u{200b}emo\u{00ad}")` — written with two `\u{...}` escapes, never a raw character. It was **APPENDED at index 6**, because `render_escape_guard` addresses this list positionally (`TAG_PAIR = 4`, `SOFT_HYPHEN_PAIR = 5`, `ZERO_WIDTH_PAIRS = [0, 2, 5]`) and an insertion would have silently repointed all three.

`rtk proxy grep -c "is_invisible_formatting_char" src/error.rs`: **5 → 5** (unchanged; the rewrite replaced a `.map().collect()` over the same filter with a `.collect()` and a loop).

The rewritten assertion:

```rust
let invisible: Vec<char> = hostile
    .chars()
    .filter(|c| is_invisible_formatting_char(*c))
    .collect();
assert!(
    !invisible.is_empty(),
    "the fixture {hostile:?} carries no invisible-class character, \
     so this assertion would be vacuous — LOOK_ALIKE_PAIRS' \
     second member is supposed to be the hostile one"
);
for c in invisible {
    let expected = format!("U+{:04X}", c as u32);
    assert!(debug.contains(&expected), /* ... */);
}
```

**The non-vacuity assertion is still load-bearing, in one line:** without it, a fixture carrying no invisible character would make the loop body execute zero times and the test would pass by silence — which is the failure mode this whole phase is named after.

**Honest note on "unmodified".** Its message string is byte-identical and its meaning is unchanged, but `git diff src/error.rs` shows ONE changed line inside it: the predicate went from `!expected.is_empty()` to `!invisible.is_empty()`, because the intermediate is now a `Vec<char>` rather than a concatenated `String`. That is a strictly more direct spelling of the same property, not a softening — but the plan's acceptance criterion said "no change inside it", so it is reported rather than glossed.

---

## The backlog sort, as it now reads

```rust
fn backlog_sort_key(number: &str) -> f64 {
    number
        .strip_prefix("999.")
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
}

fn backlog_number_ordering(a: &str, b: &str) -> std::cmp::Ordering {
    backlog_sort_key(a).total_cmp(&backlog_sort_key(b))
}
```

**There is no fallback arm.** `total_cmp` is a total order over all `f64` including `NaN`, so an `unwrap_or(Equal)` would be dead code that tells the next reader the order might not be total. The finite filter routes `NaN`, `inf` and `-inf` to the same `0.0` a failed parse already used — two unrepresentable cases, one answer — and a one-line comment at the site names why: a `.planning/phases/` directory name is chosen by a third party and is exactly SAFE-07's declared trust boundary, so a non-finite parse is an INPUT, not an impossibility.

`rtk proxy grep -n "partial_cmp" src/state_reader/backlog.rs`: **0 executable, 4 doc mentions** at lines 124, 138, 266, 341 — the refutation quote and the record of what was replaced. The acceptance criterion asked for a raw count of 0; deleting those four would delete the record prohibition 3 requires be kept, so the split is reported instead of the raw number.

**No-regression, quoted:** `well_formed_backlog_numbers_keep_the_order_they_had_before_the_total_order_fix` asserts `["999.10","999.2","999.1","999.3","999.21"]` sorts to `["999.1","999.2","999.3","999.10","999.21"]` from every rotation — byte-identical to the pre-fix behaviour for well-formed input.

---

## Deviations from Plan

### Auto-fixed / execution-time decisions

**1. [Rule 3 - Blocking] The census is committed AFTER both conversion commits, not inside Task 1's**

- **Found during:** Task 1
- **Issue:** The plan places the census in Task 1's commit and simultaneously requires each task to commit green. Those are incompatible: at the end of Task 1 the eight files of Task 2 are still un-composed, so a census committed there is red by 17 sites.
- **Fix:** Commit order is `feat` (normal.rs) → `feat` (eight files) → `test` (census). Every commit is green as committed. The RED was not lost — it was captured against the FULL pre-conversion tree (23 sites, RED 1 above), which is stronger evidence than the 17-site intermediate the plan's ordering would have produced.
- **Files modified:** none beyond the plan's
- **Verification:** each of `b3dbe4b`, `44fa6c2`, `91b271f` is green in isolation
- **Committed in:** `91b271f`

**2. [Rule 2 - Missing Critical] `WAVE_PENDING` added to the census, because the plan's design was not merge-safe**

- **Found during:** Task 1
- **Issue:** The plan requires the census to return zero for "the tree this plan leaves" AND forbids editing `driver_confirm.rs:275` AND forbids widening the exemption list. In this worktree that bare call exists, so a plain `assert_eq!(violations, 0)` is red; exempting the file would be the forbidden widening; and hard-coding an expected count of 1 would go red the moment 21-28 merges.
- **Fix:** A separate `WAVE_PENDING` register, pinned to one exact `path:line` pair, SUBTRACTED from the count rather than exempted. Zero in this worktree (the site is a violation and is subtracted); still zero post-merge (the site is no longer a violation and there is nothing to subtract). Its failure direction is stated in its own doc: under-detection, one line wide, and a satisfied entry is REPORTED (`satisfied_pending`) rather than made red, because a red there would break the merged tree for a bookkeeping reason.
- **Verification:** census green in this worktree; the subtraction is a no-op once the site is composed
- **Committed in:** `91b271f`

**3. [Rule 1 - Bug] My first backlog order assertion was FALSE of a correct implementation — the same shape as WR-08**

- **Found during:** Task 3
- **Issue:** The first formulation asserted that the sorted ELEMENT sequence is permutation-independent, over a set containing three non-numeric entries. It failed **against the fixed comparator**: `slice::sort_by` is stable, tied keys keep the input's relative order, and rotating the input therefore reorders the tied group. That is correct behaviour, and asserting against it would have shipped a control that goes red for a right implementation — precisely the defect WR-08 exists to close, reintroduced one file over.
- **Fix:** Split into (a) element-sequence permutation-independence over a **distinct-key** set, (b) **KEY**-sequence permutation-independence over the tied set, with the stable-sort reason written at the site, and (c) an explicit antisymmetry + transitivity sweep over every pair and triple. All three go red against the pre-fix comparator (verified by flipping the body back), all three are green against `total_cmp`.
- **Files modified:** `src/state_reader/backlog.rs`
- **Verification:** pre-fix body → 2 failures (both quoted in RED 3); post-fix → 8/8 green
- **Committed in:** `91c27c2`

**4. [Rule 3 - Blocking] `queue_delete_confirm::prompt_text` extracted so the two-direction pin can drive it**

- **Found during:** Task 2
- **Issue:** `Screen::render` needs a `Frame` and an `AppContext`. The tree's only `AppContext` fixture is `pub(super) fn ctx_with_aliases` inside `ui::screens::tests`, unreachable from `crate::ui`, and `AppContext` has no constructor — building a sixth full-field literal is the exact anti-pattern that fixture's own doc exists to prevent.
- **Fix:** The prompt is extracted into `pub(crate) fn prompt_text(command_text: &str) -> String`, the same shape `driver_confirm::prompt_text` already has. Escape, `CAP`, `KEEP` and the `char`-wise truncation carried across verbatim; no behaviour change.
- **Verification:** `render_escape_guard`'s existing `QueueDeleteConfirmScreen` fixtures pass unmodified
- **Committed in:** `44fa6c2`

---

**Total deviations:** 4 (1 blocking-ordering, 1 missing-critical, 1 bug in this plan's own new test, 1 blocking-testability).
**Impact on plan:** No scope creep. Deviations 1 and 2 are consequences of executing a plan designed for a merged tree inside a pre-merge worktree. Deviation 3 caught a control that would have been wrong; finding it is the plan working. Deviation 4 is a structural change to one file this plan owns, mirroring a shape already in the tree.

---

## Findings REPORTED, not fixed

**F1 — `src/ui/screens/driver_confirm.rs:275` is an un-composed render site.** `prompt_text`'s `let alias = &crate::text::display_identity(alias);`. Fenced to **21-28**, which carries the matching acceptance criterion. Named in the census's `WAVE_PENDING` register with its owner. **Post-merge action: after 21-28 lands, confirm the census reports `SATISFIED WAVE-PENDING` and remove the entry.** Keeping a satisfied entry hides that one line.

**F2 — the plan's premise that all nine conversion sites are `Into<Cow>` sinks is false; two are not.** `src/ui/screens/queue_delete_confirm.rs` and `src/ui/roadmap_widget.rs` both MEASURE and TRUNCATE the escaped form by `char` before it becomes a prompt, so they need an owned `String`. Neither uses `.to_string()`: both take the `Rendered` through `From<Rendered> for String`, which `text.rs` provides and documents for exactly this. **No `.to_string()` was added at any of the 23 conversions.** This is reported as a finding about `Rendered`'s trait surface rather than worked around, as the plan requires — the observation is that "escape then truncate for display width" is a real second shape at render sites, and the carrier serves it through an existing impl rather than needing a new one.

**F3 — the tree's only `AppContext` fixture is unreachable from the module root.** `ui::screens::tests::ctx_with_aliases` is `pub(super)` inside a private `mod tests` in `src/ui/screens/mod.rs`, so `crate::ui::tests` cannot use it, and `AppContext` has no constructor. This is what prevented an input-echo screen from getting a buffer-level two-direction spot-check (see the shortfall below). **Suggested follow-up (owner: whoever next edits `src/ui/screens/mod.rs`): widen it to `pub(crate)`.** One-line change; no new fixture.

**F4 — a stale figure in this plan's own `<tooling_note>`.** The three `bool_assert_comparison` lints are at `src/browser.rs:155,156,157`, not `131,132,133`. `src/browser.rs` is untouched by this plan.

---

## Residuals, each with its failure direction

| # | Residual | Direction | What bounds it |
|---|---|---|---|
| R1 | **ratatui's control-grapheme filtering is the reason an un-composed site was a claim defect and not a live leak. It is a property of the DEPENDENCY, asserted nowhere else in this tree, and it does not travel** — `ctx.error_message` and `ctx.status_message` are also produced by non-TUI paths where no filter exists. | **under-detection if the dependency changes, silent** | Now partly bounded: `the_roadmap_widget_..` asserts the filtering behaviour directly in a buffer, so a ratatui change goes red HERE even though nothing else in the tree asserts it. Stated in the census's doc and tied to the standing ratatui obligation already in `deferred-items.md` rather than duplicated. |
| R2 | **A call made through a local alias or a re-export is invisible to the census.** This is not hypothetical: `src/ui/mod.rs`'s own test module imports the invisible-formatting half under an alias, which is a live demonstration of the gap. | **under-detection, silent** | The same thing that bounds `text.rs`'s census: DELEGATION — one composition every render site calls — not a textual scan. |
| R3 | **A composition assembled across more than 4 physical lines** reads as un-composed. | **over-detection, loud** | It fails the build and names the site. That is the safe direction. |
| R4 | **A satisfied `WAVE_PENDING` entry hides exactly one `path:line` pair** until it is removed. | **under-detection, one line wide, reported-not-red** | `satisfied_pending` prints a `SATISFIED WAVE-PENDING` line on every census run. Removal is F1's post-merge action. |
| R5 | **The exemption for `render_escape_guard.rs` is asserted stale-free**, so if 21-28 removes that file's probe call the census goes red for a bookkeeping reason rather than a real defect. | **over-detection, loud** | The probe call is that module's reason to exist; the failure message names the exemption and its recorded reason, so the fix is obvious. |

---

## Issues Encountered

**Documented flake, reported rather than absorbed.** `tests/driver_reattach.rs` failed in one early baseline run in this worktree — `cargo test --workspace --no-fail-fast` reported **1385 passed / 2 failed / 13 ignored**, and a targeted `cargo test --test driver_reattach -- --test-threads=1` then failed `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`. An immediate re-run of the same command was **3 passed / 0 failed**, and every subsequent workspace run — including the final one — reported 0 failures. This matches the flake documented for that binary. **It is a baseline flake, not a regression:** it occurred before any file in this plan had been edited.

**No other issues.** No auth gates. No architectural decisions required (`REVERSIBILITY_GATES`: no `one-way` decision in this plan; D-21-46's `costly` rating is flagged, not gated, and its blast radius was measured — see above).

---

## Known Shortfalls

**S1 — the two-direction spot-check on an INPUT-ECHO screen was NOT delivered.** The plan's Task 2(c) asks for the spot-check on two of the eight files, "one input echo and one `.planning/`-derived value". Both delivered spot-checks are `.planning/`-derived: `roadmap_widget.rs` (`ROADMAP.md`, at ratatui buffer level) and `queue_delete_confirm.rs` (`queue.md`, at exact-prompt-string level). The input-echo screens (`add_project`, `create_project`, `driver_start`, `driver_inject`, `enqueue`) render only through `Screen::render`, which needs an `AppContext` — see F3. Writing a sixth full-field `AppContext` literal is the anti-pattern `ctx_with_aliases`' own doc names, and `src/ui/screens/mod.rs` is fenced to 21-28 this wave, so the visibility widening that would fix it belongs to another plan's file.

**What IS covered for those five files:** the census (they contain zero un-composed executable calls), the generic function-level two-direction pin, and `render_escape_guard`'s existing per-screen probes, which render every screen with a hostile identity and assert the escaped form reaches the buffer — all green after the conversion. **What is NOT covered:** a per-screen, buffer-level assertion that a CONTROL-carrying input echo renders differently after the conversion. **Direction: under-detection, disclosed** — a conversion that was a no-op at one of those five specifically would be caught by the census (it would not be a `render_for_terminal` call at all) but not by a rendering assertion. Follow-up is F3.

**S2 — ROADMAP success criterion 4 is untouched, by design.** `tests/driver_injection_corpus.rs` was RUN (its 13 non-ignored arms pass; 10 remain `#[ignore]`d) and EDITED under no circumstance — `git diff --stat 80bc4c1 HEAD -- tests/driver_injection_corpus.rs` is empty. **No work is claimed against criterion 4.** It needs a human with an authenticated Claude subscription and is permanently agent-unclosable.

---

## Broken-windows ledger

`gsd-tools windows append` was not invoked: the shortfalls above are disclosed shortfalls with named follow-ups rather than stubs, skipped tests or unrun verifies. There are no `#[ignore]`d tests added by this plan, no `TODO`/`FIXME` introduced, and every `<verify>` command in the plan was run and is quoted.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **`.planning/REQUIREMENTS.md` is untouched** by every commit of this plan (`git diff --stat 80bc4c1 HEAD -- .planning/REQUIREMENTS.md` is empty). Requirement status is for the phase-close step to decide from a passed verification, not for this executor.
- **Wave hand-off:** this plan's diff names only its thirteen declared files. It edits none of `detail.rs`, `text.rs`, `driver.rs`, `driver_confirm.rs`, `src/ui/screens/mod.rs` or `render_escape_guard.rs`. `src/test_support.rs` is in this plan's `files_modified` and both sibling plans fence it read-only, so the index-6 append is not a wave conflict — but 21-27 and 21-28 will each gain one extra loop iteration from it wherever they consume `LOOK_ALIKE_PAIRS`.
- **Post-merge action (F1):** confirm the census reports `SATISFIED WAVE-PENDING: src/ui/screens/driver_confirm.rs:275` once 21-28 lands, then remove the entry from `WAVE_PENDING` in `src/ui/mod.rs`.
- **Post-merge check:** the census asserts an equality over the WHOLE of `src/ui/`, so it will see 21-27's and 21-28's changes. If either introduces a new un-composed executable call, the census names it by file and line.

## Self-Check: PASSED

- `src/ui/mod.rs`, `src/ui/screens/normal.rs`, `src/ui/roadmap_widget.rs`, `src/ui/screens/add_project.rs`, `src/ui/screens/create_project.rs`, `src/ui/screens/delete_confirm.rs`, `src/ui/screens/driver_start.rs`, `src/ui/screens/driver_inject.rs`, `src/ui/screens/enqueue.rs`, `src/ui/screens/queue_delete_confirm.rs`, `src/state_reader/backlog.rs`, `src/error.rs`, `src/test_support.rs` — all present on disk.
- `git log --oneline 80bc4c1..HEAD` returns 5 commits: `b3dbe4b`, `44fa6c2`, `91b271f`, `91c27c2`, `f1d0b1f`.
- All task `<acceptance_criteria>` re-run; every shortfall against one is named in "Known Shortfalls" or "Findings REPORTED" above rather than silently passed.
- Plan-level `<verification>` re-run at `f1d0b1f`: `cargo build` exit 0; `cargo test --workspace --no-fail-fast` **1398 / 0 / 13**; `cargo clippy -- -D warnings` exit 0; `cargo clippy --all-targets -- -D warnings` exactly 4 pre-existing lints, same two kinds, same two files.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-27*
