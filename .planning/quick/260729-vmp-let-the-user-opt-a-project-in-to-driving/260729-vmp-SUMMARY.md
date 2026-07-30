---
phase: quick-260729-vmp
plan: 01
subsystem: ui/driver
status: complete
tags: [driver, opt-in, keybinding, ctrl-03, help, tui]
requires:
  - src/ui/screens/driver_confirm.rs (DriverConfirmScreen, DriverAction::ToggleOptIn, do_toggle_opt_in)
  - src/executor/mod.rs (DrivableProject::from_registry — read-only)
  - src/registry.rs (is_opted_in / record_opt_in — read-only)
provides:
  - "`o` on the Driver tab opens the same opt-in confirmation the dashboard's `o` opens"
  - "an empty-state instruction that names a key working on the surface it is displayed on"
  - "an enforced tab-scoping property for the four driver action keys"
affects:
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/help.rs
tech-stack:
  added: []
  patterns:
    - "tab-scoped key arms guarded by `if current_view == DetailSubView::Driver`, above the generic `Char(c)` arm"
    - "keys documented as WHOLE help rows, never by contains(key)"
    - "a single opt-in write path; new affordances push the existing confirmation rather than mutating the registry"
key-files:
  created: []
  modified:
    - src/ui/screens/detail.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/help.rs
decisions:
  - "`o` was chosen because the dashboard already binds `o` for this exact action; verb/key consistency across the two surfaces beats a unique letter"
  - "the `current_view` guard is present despite there being NO collision on any detail tab, so `o` cannot silently become a global detail-screen opt-in"
  - "the new arm pushes DriverConfirmScreen and nothing else — `do_toggle_opt_in` stays the sole opt-in write path"
  - "the end-to-end test observes the spawn seam's answer on both sides of the transition, read off the RELOADED config, because DriverConfirmScreen::name() cannot distinguish Stop from ToggleOptIn"
metrics:
  duration: 7min
  tasks: 3
  files: 3
  tests_added: 1
  completed: 2026-07-30
---

# Quick Task 260729-vmp: Let the User Opt a Project In to Driving from the Driver Tab

Bound `o` on the Driver tab to the *same* `DriverConfirmScreen::new(alias,
DriverAction::ToggleOptIn)` the dashboard's `o` already pushes, removing a navigation
round-trip without moving the CTRL-03 safety gate.

## What Was Built

**Task 1 — the binding and the end-to-end proof** (`src/ui/screens/detail.rs`, TDD)

A single match arm in `DetailScreen::handle_key`, placed immediately after the `x` arm and
well above the generic `KeyCode::Char(c)` arm:

```rust
KeyCode::Char('o') if current_view == DetailSubView::Driver => {
    ctx.needs_redraw = true;
    ScreenAction::Push(Box::new(DriverConfirmScreen::new(
        self.alias.clone(),
        DriverAction::ToggleOptIn,
    )))
}
```

It calls no registry function and no `save_config`. The only difference from
`normal.rs:444-454` is that the alias comes from `self.alias` (the project whose tab is on
screen) rather than `ctx.selected_alias()`.

Alongside it, `driver_optin_fixture()` — built on `driver_confirm::tests::ctx_with_project`
rather than `driver_action_fixture()`, because the latter registers no project and points
`config_path` at a relative path (a real toggle would fail `UnknownAlias`, or write a config
file into the repository working directory) — and the regression test
`opting_in_from_the_driver_tab_flips_the_spawn_seam`, which runs:

1. `DrivableProject::from_registry` is `Err(NotOptedIn)` — the fixture starts refused, so the
   rest cannot pass vacuously (verified as a real RED: the test failed on the *push*
   assertion, meaning assertion 1 had already passed).
2. `o` pushes a screen named `driver_confirm`.
3. **`is_opted_in` is still false** — the load-bearing safety assertion. A binding that opted
   in and *then* asked would satisfy every other assertion in the test.
4. `y` on the pushed screen writes `config.json`.
5. `load_config` shows `driver_opt_in.is_some()`.
6. `DrivableProject::from_registry` on the **reloaded** entry is now `Ok`.

**Task 2 — scoping and copy** (`src/ui/screens/driver.rs`, `src/ui/screens/detail.rs`, TDD)

`NOT_OPTED_IN_NEXT` went from `"Press [o] on the dashboard to allow it."` to
`"Press [o] to allow it."` — strictly shorter, still two `Line`s in both arms of
`no_runs_lines`, so the empty state's height and width budget only shrank.
`the_empty_run_list_copy_differs_by_opt_in_state` gained
`assert!(!not_opted_in[1].contains("dashboard"), ...)`, which failed RED before the constant
changed. It asserts against the **rendered** `Line`, not the source, so it stays true
regardless of surrounding doc comments.

`the_three_driver_keys_are_scoped_to_the_driver_tab` became
`the_driver_action_keys_are_scoped_to_the_driver_tab` and now covers `o` alongside `i`/`s`/`x`
across PhaseList, Pipeline and Defaults. This is what turns the guard from a comment into an
enforced property: a future refactor that drops the `if current_view == ...` clause compiles
clean and would otherwise silently make `o` a global opt-in on every detail tab.

**Task 3 — documentation** (`src/ui/screens/help.rs`)

`row("o", "Toggle driver opt-in for this project")` added to the `Driver Tab (detail view)`
section, with the matching entry in `help_lines_documents_every_key_this_phase_binds`. The
dashboard's `row("o", "Toggle driver opt-in (dashboard)")` was left untouched — both surfaces
bind `o` and the help lists both, which is what makes the consistency visible instead of
coincidental. The two descriptions are deliberately not byte-identical, since the test matches
whole rows and could otherwise not tell which section it matched.

## The Key Chosen, and Why

**`o`.** Three reasons, in order of weight:

1. **The dashboard already binds `o` for exactly this action** (`normal.rs:444-454`). The
   verb/key mapping now reads the same on both surfaces that can perform it — the same
   argument the existing `x` arm's comment makes for reusing `x` for stop.
2. **A grep for `Char('o')` and `Char('O')` across `detail.rs` found zero existing bindings**,
   so no collision had to be resolved on any detail tab. `o` was simply free.
3. **The empty-state copy already said `[o]`.** It just pointed somewhere else. Choosing any
   other letter would have meant rewriting the instruction rather than re-pointing it.

The `current_view == DetailSubView::Driver` guard is present *despite* reason 2. Scoping was
not needed to dodge a conflict; it is there so the key stays on the tab that explains it,
rather than becoming an undiscoverable, unannounced global opt-in on Browse, Queue, Defaults
and every other detail tab.

## Safety Contract: The Spawn Seam Is Untouched

**`src/executor/mod.rs` and `src/registry.rs` are byte-identical to HEAD `ded839b`.** Verified
directly, and the command prints nothing:

```
git diff --name-only ded839b -- src/registry.rs src/executor/mod.rs
```

Concretely:

- `DrivableProject::from_registry` still refuses `OptInError::NotOptedIn` before it examines
  the path, before any process is launched. Its own pre-existing test
  `from_registry_refuses_a_project_with_no_opt_in_record` still passes unchanged.
- `do_toggle_opt_in` (`driver_confirm.rs:370`) remains the **only** opt-in write path. The new
  arm adds no second one.
- Opt-in is not implicit, not automatic, and not a side effect of visiting or rendering the
  tab. It requires a distinct keypress **and** `y` on the confirmation, asserted directly by
  step 3 of the new test.
- The alias flowing into the confirmation is `self.alias` — the project whose tab is on
  screen — and the same `String` feeds both the prompt and the mutation, so the confirmation
  cannot name one project while toggling another.

Every `mitigate` row in the plan's threat register (T-VMP-01 through T-VMP-04) has a passing
assertion behind it; T-VMP-05 was `accept` and its `sanitize_render_line` path was not
touched.

## Deviations from Plan

None — the plan executed exactly as written.

One process note, not a code deviation: the plan's Task 1 is `type="tracer"`, and the
tracer feedback gate normally emits a `checkpoint:human-verify` in a non-auto run. Both of
that task's `<verify>` entries are `<automated>` (a `cargo test` grep and a `git diff`
emptiness check) with no visual or behavioural content a human could evaluate, and the plan
frontmatter declares `autonomous: true`. Both gates were re-run green before any expansion
task began, and execution continued rather than pausing on an empty checkpoint.

## Verification Results

All six steps of the plan's `<verification>` section, in order:

| # | Check | Result |
|---|-------|--------|
| 1 | `cargo build && cargo test && cargo clippy -- -D warnings` | PASS — 773 passed, clippy clean |
| 2 | Test count up from 772 | **773** (+1, as predicted) |
| 3 | `--all-targets` clippy lint count | **5** — unchanged, the known pre-existing set |
| 4 | `git diff --name-only -- src/registry.rs src/executor/mod.rs` | empty |
| 5a | `opting_in_from_the_driver_tab_flips_the_spawn_seam` | `ok. 1 passed` |
| 5b | `the_driver_action_keys_are_scoped_to_the_driver_tab` | `ok. 1 passed` |
| 5c | `help_lines_documents_every_key_this_phase_binds` | `ok. 1 passed` |
| 6 | `from_registry_refuses_a_project_with_no_opt_in_record` | `ok. 1 passed` |

The five pre-existing `--all-targets` lints are unchanged: 3x
`clippy::bool_assert_comparison` (`browser.rs`), 1x `clippy::cmp_owned`
(`project_creator.rs`), 1x `clippy::items_after_test_module` (`state_reader/mod.rs`). All
raw-output checks were run through `rtk proxy cargo ...`, since the `rtk` wrapper filters
`warning:` and `test result:` lines out of plain `cargo` output and any grep for them would
otherwise pass vacuously.

The known Phase 18 defect **WR-05** (blank pipeline row at terminal heights 8-13) was not
touched — explicitly out of scope per the plan.

## Known Stubs

None. No placeholder values, no TODO/FIXME, no skipped tests, no unrun `<verify>` steps.

## TDD Gate Compliance

Both TDD tasks show the full gate sequence in `git log`:

- Task 1: `test(...)` `b925da9` (RED — failed on the push assertion, with the
  refused-seam precondition already passing) then `feat(...)` `4f06cb9` (GREEN).
- Task 2: `test(...)` `7650908` (RED — the copy test failed on the new `dashboard`
  assertion) then `feat(...)` `1252c42` (GREEN).

No REFACTOR gate was needed; nothing warranted cleanup.

## Commits

| Commit | Message |
|--------|---------|
| `b925da9` | `test(quick-260729-vmp-01): add failing test for opting in from the Driver tab` |
| `4f06cb9` | `feat(quick-260729-vmp-01): bind o on the Driver tab to the opt-in confirmation` |
| `7650908` | `test(quick-260729-vmp-02): assert the refusal copy names no other surface` |
| `1252c42` | `feat(quick-260729-vmp-02): point the not-opted-in copy at the key on this tab` |
| `70157bd` | `docs(quick-260729-vmp-03): document o in the help popup's Driver Tab section` |

## Self-Check: PASSED

All three modified files exist and all five commit hashes resolve in `git log`.
