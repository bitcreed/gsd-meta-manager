---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 28
subsystem: ui
tags: [ratatui, prompt-injection, unicode, render-escape, untrusted-carrier, census]

requires:
  - phase: 21 (plans 21-23, 21-25)
    provides: "`crate::text::Untrusted` / `Rendered` carriers, `render_for_terminal`, `shown_capped`, the render-escape probe and its per-state `DETAIL_TAB_ARRIVAL` record"
provides:
  - "`DriverOutputLine::text` retyped to `crate::text::Untrusted` — the driver output pane's escape is held by the compiler, not by a call"
  - "Both classes composed at the injection rows, the dry-run preview and the opt-in disclosure"
  - "`probe_ctx` populates `ctx.driver_output`, `cache.driver_journal`, `cache.driver_inbox`; a `driver_dry_run` probe state; branch-reached tokens for both"
  - "A committed census asserting no executable control-class call in `driver.rs`/`driver_confirm.rs` stands outside a composition"
  - "Two false doc claims corrected in the commits that made them true, each quoting what it replaced"
affects: [21-29, 21-30, render-escape-guard, driver-tab]

actuals:
  tokens: 167000
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Carrier-at-the-struct-field: retype the one field, let the compiler enumerate every consumer"
    - "Balanced-delimiter logical-line census (parens/brackets only; braces are boundaries)"
    - "Probe arrange functions must DERIVE untrusted values from populated state, never spell them, or the chrome baseline cancels arrival"

key-files:
  created: []
  modified:
    - src/ui/screens/mod.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/render_escape_guard.rs
    - src/app.rs

key-decisions:
  - "D-21-37: the output pane is closed at the TYPE, not at the render call — semver-breaking on a published crate, accepted"
  - "D-21-38: control class + cap at APPEND time, invisible class at RENDER time; the split is pinned equal to `shown_capped` in both directions with a non-vacuity arm"
  - "D-21-39: `InboxMessage::text` and `DryRunPreview::report` deliberately NOT retyped (outside the wave fence); disclosed as call-held with direction `under-protection, silent`"
  - "WR-06's control belongs at `render_disclosure`, not at `DriverConfirmScreen` — the live screen path passes authored paths and hex digests, so no screen fixture could go red there"
  - "The census judges structure (balanced-delimiter logical lines), not proximity — proximity could not go red against a planted call"

patterns-established:
  - "A census must be observed red by PLANTING before it is trusted: two real defects in this one were invisible to reading"
  - "A probe arrange that spells the untrusted value itself puts it in the chrome baseline too and reports its own state as not-arriving"

requirements-completed: [SAFE-07, DRIVE-01]

coverage:
  - id: D1
    description: "The driver output pane cannot render an unescaped invisible-class character, and the render site is held by the compiler"
    requirement: DRIVE-01
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#the_wrapped_line_composition_equals_shown_capped"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#the_retype_left_the_rings_accounting_where_it_was"
        status: pass
    human_judgment: false
  - id: D2
    description: "The injection rows and the dry-run preview compose both classes, each observed red separately"
    requirement: DRIVE-01
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped"
        status: pass
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_two_new_driver_states_reach_their_branches"
        status: pass
    human_judgment: false
  - id: D3
    description: "The Driver tab's output pane, injection rows and dry-run preview render under probe on every run, with arrival recorded per state"
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped (DETAIL_TAB_ARRIVAL set equality, both directions)"
        status: pass
    human_judgment: false
  - id: D4
    description: "WR-06 — the opt-in disclosure composes both classes over path and digest"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#the_disclosure_escapes_both_classes_over_the_path_and_the_digest"
        status: pass
    human_judgment: false
  - id: D5
    description: "IN-01 — `shown_capped`'s completeness claim replaced by a census that checks it, observed red by planting"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#no_executable_control_class_call_in_these_two_files_stands_outside_a_composition"
        status: pass
    human_judgment: false
  - id: D6
    description: "SAFE-07 boundary and precision reconfirmed by re-run: this plan's diff did not move the structural boundary"
    requirement: SAFE-07
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs (38 passed / 0 failed / 0 ignored, file unedited)"
        status: pass
      - kind: integration
        ref: "tests/driver_injection_corpus.rs (13 passed / 0 failed / 10 ignored, file unedited)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The two corrected doc claims read as candour rather than as a softer restatement of the same overclaim"
    verification: []
    human_judgment: true
    rationale: "Whether a rewritten disclosure is genuinely NARROWER than what it replaced, and whether the residual's stated failure direction reads as an admission, is a reading judgement no test asserts. The verbatim old wording is quoted beside each replacement so a human can check the narrowing directly."

duration: 33min
completed: 2026-08-27
status: complete
---

# Phase 21 Plan 28: Driver Output Pane Escape at the Type Summary

**`DriverOutputLine::text` becomes `crate::text::Untrusted` so the pane showing the LLM's own prose cannot render an unescaped tag character; the other three sites on that path compose both classes at the call, and the difference between the two is disclosed rather than smoothed.**

## Performance

- **Duration:** 33 min
- **Started:** 2026-08-27T20:31:02Z
- **Completed:** 2026-08-27T21:04:31Z
- **Tasks:** 3
- **Files modified:** 5

## Accomplishments

- **CR-02 closed at the TYPE on the surface that matters most.** `DriverOutputLine::text` is `crate::text::Untrusted`; `output_line`'s `Span::styled(line.text.clone(), ..)` stopped compiling until it became `shown()`. Observed red first: with `ctx.driver_output` populated and nothing else changed, the probe reported four `\u{e0041}` reaching cells in the Driver tab.
- **The other three sites compose both classes at the call**, each observed red separately against a fixture that provably reaches its branch.
- **The probe hole that hid all of it for nine rounds is closed.** `probe_ctx` populates `ctx.driver_output`, `cache.driver_journal` and `cache.driver_inbox`; a `Driver tab, dry-run preview` state drives the preview. Arrival is recorded per state and asserted as a set equality against the `chrome_ctx` baseline.
- **Two false doc claims corrected in the commits that made them true**, each quoting the falsified text verbatim and dated.
- **IN-01's completeness claim replaced by a census** that was observed red by planting — after two real defects in the census itself were found the same way.
- **Seven verbatim REDs**, one more than the plan's six, because the census needed two iterations to acquire teeth.

## Task Commits

1. **Task 1 (tracer): output pane held by the compiler** — `4801187` (feat)
2. **Task 2: injection rows, dry-run preview, fixture, doc + LIMIT 1** — `9107cf9` (fix)
3. **Task 3: WR-06 and IN-01's census** — `2fcf41b` (fix)

Tracer feedback gate: the tracer's `<verify>` was re-run end-to-end after `4801187` and passed before any expansion task began.

## Files Created/Modified

- `src/ui/screens/mod.rs` — `DriverOutputLine::text` retyped; wrapped once in `push_record`; `sanitize_render_line`'s composition claim corrected.
- `src/ui/screens/driver.rs` — `output_line` renders `shown()`; injection rows and dry-run preview compose through `shown_capped`; composition-equality pin, ring-accounting pin, the census and its logical-line splitter; `shown_capped`'s completeness claim replaced.
- `src/ui/screens/driver_confirm.rs` — `render_disclosure` composes both classes over `path` and `digest`; the bare `display_identity` becomes `render_for_terminal`; the WR-06 control.
- `src/ui/screens/render_escape_guard.rs` — `probe_ctx` populates the three driver caches; `Driver tab, dry-run preview` state; two arrival rows; branch-reached test; LIMIT 1 rewritten narrower.
- `src/app.rs` — one test helper's accessor (compiler-forced; see Deviations).

## Re-measured gates (all under `rtk proxy`)

| Gate | Plan's figure | Re-measured |
|---|---|---|
| `cargo build` | exit 0 | exit 0 |
| `cargo test --workspace --no-fail-fast` | 1387 / 0 / 13 | **1392 passed / 0 failed / 13 ignored** |
| `cargo clippy -- -D warnings` | exit 0 | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exactly 4 lints | exactly **4**, same two kinds, same two files |
| `cargo test --test spawn_seam_guard` | quote HEAD count | 38 / 0 / 0, file unedited |
| `cargo test --test driver_injection_corpus` | 13 / 0 / 10 | 13 / 0 / 10, file unedited |

**Every delta attributed.** 1392 − 1387 = **+5**, all new tests: T1 +2 (`the_wrapped_line_composition_equals_shown_capped`, `the_retype_left_the_rings_accounting_where_it_was`), T2 +1 (`the_two_new_driver_states_reach_their_branches`), T3 +2 (`the_disclosure_escapes_both_classes_over_the_path_and_the_digest`, `no_executable_control_class_call_in_these_two_files_stands_outside_a_composition`).

**The four pre-existing clippy lints are untouched and that is MEASURED:** `browser.rs:155,156,157` (`bool_assert_comparison` ×3) and `project_creator.rs:146` (`cmp_owned` ×1). `git diff --stat` names neither file. **Note the plan's line numbers were stale** — it said `browser.rs:131,132,133`; re-measured they are `155,156,157`. Same three lints, same kind, same file.

### Site counts, re-measured against plan HEAD `80bc4c1`

| Measurement | At `80bc4c1` | Now |
|---|---|---|
| `sanitize_render_line` in `driver.rs` | 10 | — (2 executable non-composition calls at HEAD; **0** now) |
| `sanitize_render_line` in `driver_confirm.rs` | **8** (plan said 6) | — (2 executable non-composition calls at HEAD; **0** now) |
| `shown_capped` in `driver.rs` | 8 | 22 |
| `display_identity(` in `driver_confirm.rs` | 5 (one BARE, at `prompt_text`) | 4, **all composed** |
| `display_identity(` in `driver.rs` | 2 | 2, both already compositions |
| `driver_output` / `driver_journal` / `driver_inbox` in guard | 0 / 0 / 0 | non-zero |
| `driver_dry_run` in guard | 1 (LIMIT 1's own mention of it as unprobed) | non-zero |

**A plan figure that did not survive re-measurement:** the plan states `sanitize_render_line` has "**six**" occurrences in `driver_confirm.rs`. Measured at `80bc4c1` it is **eight**. The plan's split (two executable non-composition calls at `:164`/`:166`) was correct; only the total was wrong.

## The seven REDs

Each is quoted verbatim in its task's commit message.

| # | What | Evidence |
|---|---|---|
| 1 | Output pane, before the retype | `[Driver tab] rendered ['\u{e0041}', '\u{e0041}', '\u{e0041}', '\u{e0041}']` |
| 2 | Output pane, single-site revert of `shown()` | same four characters |
| 3 | Injection rows | `[Driver tab] rendered ['\u{e0041}']` |
| 4 | Dry-run preview | `[Driver tab, dry-run preview] rendered ['\u{e0041}', '\u{e0041}', '\u{e0041}']` |
| 5 | Disclosure, before either site composed | `demo\u{200b}` survives into the rendered disclosure |
| 6 | Disclosure, single-site revert of the **digest** composition | `driver_confirm.rs:902` |
| 7 | Census, against a planted unconverted call | `Sites: ["src/ui/screens/driver_confirm.rs:186"]`, left 1 / right 0 |

**On "clean tree after each red", stated precisely rather than claimed loosely.** REDs 2, 6 and 7 were plant-and-restore: after restoring, `git status --porcelain` was empty. REDs 1, 3, 4 and 5 were produced by fixture/test additions that are KEPT and ship in the task commit, so the tree was not empty at that moment — it carried exactly the fixture under test. Saying otherwise would be false. RED 4 was **re-captured against the final committed fixture** after that fixture was corrected, so the red and the shipped fixture are the same artifact.

## Per-state probe results

| State | Cache populated | Branch-reached token | Arrival |
|---|---|---|---|
| `Driver tab` (output pane) | `ctx.driver_output`, 4 lines via `push_record`, one per `DriverLineKind` | n/a — the pane is the tab's default body | recorded, set equality holds |
| `Driver tab` (injection rows) | `cache.driver_inbox` + `cache.driver_journal` | `\u{25CB} queued`, absent when `driver_inbox` is emptied | recorded |
| `Driver tab, dry-run preview` | `cache.driver_dry_run`, report derived from the populated run | run detail's `── steps ` rule asserted ABSENT here and PRESENT in the ordinary tab | recorded |

No state failed to arrive in the final tree. One did during development — see Issues.

## Decisions Made

- **D-21-37 (costly, taken):** the output pane is closed at the type. `DriverOutputLine` is a `pub struct` with `pub` fields on a crate published to crates.io, so this is semver-breaking and lands in the next minor. No on-disk format, no parse behaviour, no migration.
- **D-21-39 (accepted debt, disclosed):** `InboxMessage::text` and `DryRunPreview::report` are not retyped. They reach `src/journal/inbox.rs` and `src/app.rs`, outside this wave's fence. Held by a call plus a probe fixture. **Direction: under-protection, silent** — a new render of either compiles and draws. What would force the promote: a third render of either value, or any change to those files already open for another reason.
- **WR-06's control is at the function, not the screen** — see Issues, this is a correction to the plan's premise.
- **The census judges structure, not proximity** — forced by two planted-defect failures.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] `src/app.rs` edited, a fifth file beyond the plan's declared four**
- **Found during:** Task 1, step (d)
- **Issue:** The retype made the compiler name six consumers. Five were test-side; one of those, `App`'s `buffered` test helper at `src/app.rs:2822`, is outside the plan's `files_modified` list. The plan's acceptance criterion says `git diff --stat` must name only its four files.
- **Fix:** One accessor — `line.text.clone()` → `line.text.as_raw_for_logic_only().to_string()` — with a one-line reason. No carrier retyped, so prohibition 5 is not engaged; `src/app.rs` is not in prohibition 3's list.
- **Verification:** `git diff --stat 80bc4c1 HEAD` names exactly 5 files, none of them prohibition 3's.
- **Committed in:** `4801187`

**2. [Rule 1 — Bug] The dry-run probe arrange cancelled its own arrival measurement**
- **Found during:** Task 2, step (d)
- **Issue:** The arrange spelled the report from `identity` directly. Every arrange also runs against `chrome_ctx` to build the baseline, so the value landed on both sides, the difference was zero, and the probe reported `Driver tab, dry-run preview` as not arriving — while it was visibly leaking three tag characters. This is exactly the failure prohibition 2 names, one level down.
- **Fix:** The arrange derives the report from the run `probe_ctx` populated and returns early when there are no runs, so `chrome_ctx` draws chrome only.
- **Verification:** set equality passes in both directions; RED #4 re-captured against the corrected fixture.
- **Committed in:** `9107cf9`

**3. [Rule 1 — Bug] The census could not go red — twice**
- **Found during:** Task 3, step (e)
- **Issue:** (a) The first version joined lines forward and backward until it found a composer; `render_disclosure` composes correctly two statements above the plant, so the backward walk found a `display_identity` belonging to a different statement and passed the plant. (b) The second version counted `{` in the delimiter depth, so `fn f(..) {` left depth above zero and an entire function body accumulated into one logical line, again swallowing the plant.
- **Fix:** Logical lines are built on balanced **paren/bracket** depth with braces as boundaries; proximity dropped entirely.
- **Verification:** RED #7 names the planted file and line exactly.
- **Committed in:** `2fcf41b`

**4. [Rule 2 — Transparency] Acceptance criterion `grep -c "pub text: String" src/ui/screens/mod.rs == 0` reports 1**
- **Found during:** Task 1
- **Issue:** The criterion's literal reading conflicts with this phase's own rule that a corrected doc must quote the falsified text verbatim. The one match is the doc quote of the old declaration.
- **Fix:** Neither. **Both numbers are reported** rather than deleting the evidence to make a number look right: raw grep = **1** (the doc quote), non-comment grep = **0** (no field). The criterion's intent — the field is retyped — holds.
- **Verification:** `grep -n "pub text: String" src/ui/screens/mod.rs` returns only line 337, a `///` line.

---

**Total deviations:** 4 (1 blocking, 2 bugs, 1 transparency).
**Impact on plan:** Deviations 2 and 3 are the plan's own controls catching real defects in this plan's own new mechanisms before they shipped as false green — the outcome the plant-and-observe discipline exists to produce. Deviation 1 is unavoidable and bounded. No scope creep.

## Issues Encountered

**WR-06's premise did not survive measurement, and this is the most consequential finding here.** The plan states that `render_disclosure` draws `input.path` and `digest` "read out of the recorded opt-in block in the user's `config.json`". The live screen path does not do that: `DriverConfirmScreen::render` calls `registry::current_prompt_inputs(&entry.path)`, which builds every `path` from the authored `&'static str`s in `DISCLOSED_PROMPT_INPUTS` and every `digest` from `sha256_digest` — hex. **Neither value on the currently rendered path can carry an invisible-class character.** A `DriverConfirmScreen` fixture therefore *could not* go red for WR-06, and one that appeared to would have been proving something else.

The untrusted vector is `render_disclosure`'s **argument**, which is precisely what the function's own doc contracts for ("the list **recorded on the record**") and what its `pub` visibility exposes. The committed control is therefore at the function, where the value enters, and it does go red. The escape at both sites is real and needed for the documented recorded-list caller; what is *not* true is that today's screen render is a live leak. Recorded here rather than smoothed, because a fixture reporting green over an authored constant is the exact shape this phase keeps finding.

**Documented flake, reported not absorbed.** `envelope_tracer::a_relocated_copy_of_the_stub_refuses_instead_of_acting` failed once during the Task 2 workspace run and passed **6/6 on re-run**, both parallel and with `--test-threads=1`. It is named as a flake in the plan's own tooling note. `driver_reattach` failed its two documented tests once in the pre-work baseline run and passed **3/3 under `--test-threads=1`**; it passed cleanly in both later full runs. Neither is attributable to this plan — both binaries are untouched by its diff.

**ROADMAP criterion 4 was not attempted and nothing is claimed against it.** `tests/spawn_seam_guard.rs` and `tests/driver_injection_corpus.rs` were RUN as SAFE-07 backstops and edited under no circumstance; `git diff --stat` names neither. The 10 `#[ignore]`d `driver_injection_corpus` arms still need a human with a live Claude subscription.

## Known Stubs

None. No hardcoded empty values, placeholder text or unwired components were introduced.

## Limits carried forward (each with its direction)

1. **Three of the four sites on this path are held by a CALL, not by a type** — `InboxMessage::text`, `DryRunPreview::report`, `PromptInput::path`/`digest`. A new render of any of them compiles and draws. **Under-protection, silent.** Recorded in LIMIT 1's four-row table with what would force the promote.
2. **The census is a source scan over two named files.** A composition assembled across separate statements, or a third file added to this path tomorrow, is invisible to it. **Under-detection, silent.** Bounded by the probe, not by the census.
3. **The injection fixture probes only the `Queued` state.** The `message.text` row is drawn in every state so the escaped site is covered, but the elapsed counter and missed-reason gloss are not. **Under-detection, silent.**
4. **The dry-run fixture sets `report: Some(..)`**, so the `None` loading branch is unprobed. It draws an authored constant and no identity. **Under-detection, silent.**
5. **The Defaults tab's string-EDIT overlay remains the residual unprobed STATE**, unchanged from LIMIT 1's prior wording.

## User Setup Required

None.

## Next Phase Readiness

- Prohibition 4's cross-plan contract with **21-29** is discharged: no bare `crate::text::display_identity` call remains in `driver.rs` or `driver_confirm.rs`. All six calls across the two files compose with `sanitize_render_line` on the same logical line.
- **21-30** should record `journal::inbox::InboxMessage::text` and `ui::screens::DryRunPreview::report` in `deferred-items.md` with the direction `under-protection, silent` and the promote trigger stated in LIMIT 1. `PromptInput::path`/`digest` in `src/config.rs` belongs on that list too — it was not in the plan's carrier table and is a third call-held carrier.
- No file owned by 21-27 or 21-29 was touched; no wave conflict was encountered.
- `.planning/REQUIREMENTS.md` is untouched by every commit of this plan.

## Self-Check: PASSED

- All five modified files exist on disk.
- All four commits exist: `4801187`, `9107cf9`, `2fcf41b`, `bb159dc`.
- `git status --porcelain` empty — no uncommitted work, no plant left behind.
- Every task's `<acceptance_criteria>` re-run; the one that does not pass on its literal wording (`grep -c "pub text: String" == 0`) is reported with both numbers and its cause in Deviations #4 rather than silently skipped.
- Plan-level `<verification>` re-run: build exit 0; workspace 1392/0/13; clippy exit 0; `--all-targets` exactly 4 pre-existing lints; both SAFE-07 backstops green and unedited; `git diff --stat` names 5 files (the plan's 4 plus `src/app.rs`, Deviation #1) and none of prohibition 3's; `.planning/REQUIREMENTS.md` untouched.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-27*
