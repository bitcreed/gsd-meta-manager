---
phase: quick-260729-vmp
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/help.rs
autonomous: true
requirements: [QUICK-260729-vmp, CTRL-03]

must_haves:
  truths:
    - "Pressing `o` on the Driver tab of a project that is not opted in opens the SAME DriverConfirmScreen the dashboard's `o` opens (DriverAction::ToggleOptIn), and confirming it with `y` records the opt-in and persists it to config.json."
    - "Pressing `o` on the Driver tab and NOT confirming leaves the project not opted in — the confirmation is never bypassed, and opt-in is never a side effect of visiting or rendering the tab."
    - "A project with no opt-in record still yields Err(OptInError::NotOptedIn) from DrivableProject::from_registry; the same project yields Ok only after the confirmation has been accepted."
    - "`o` pressed on any non-Driver detail tab pushes no driver screen."
    - "The Driver tab's not-opted-in empty state names a key the user can press on that tab, in the same two lines and no wider than the copy it replaces."
    - "The help popup's Driver Tab section documents `o` on its own row, asserted as a whole row by the existing help test."
    - "src/registry.rs and src/executor/mod.rs are byte-identical to HEAD — the opt-in write path and the spawn seam are unchanged."
  artifacts:
    - "src/ui/screens/detail.rs — tab-scoped `KeyCode::Char('o') if current_view == DetailSubView::Driver` arm pushing DriverConfirmScreen with DriverAction::ToggleOptIn"
    - "src/ui/screens/detail.rs — driver_optin_fixture + end-to-end regression test spanning keypress -> confirmation -> config.json -> spawn seam"
    - "src/ui/screens/detail.rs — the driver key-scoping test extended to cover `o`"
    - "src/ui/screens/driver.rs — NOT_OPTED_IN_NEXT re-pointed at the Driver tab's own key, with its copy test strengthened"
    - "src/ui/screens/help.rs — a Driver Tab section row for `o`, plus the matching whole-row entry in the help test"
  key_links:
    - "detail.rs `o` arm -> DriverConfirmScreen::new(alias, DriverAction::ToggleOptIn) -> driver_confirm.rs:370 do_toggle_opt_in -> registry::record_opt_in -> save_config -> config.json"
    - "config.json driver_opt_in -> src/executor/mod.rs:159 DrivableProject::from_registry — the process-spawn seam, read-only in this change"
    - "driver.rs:228 NOT_OPTED_IN_NEXT -> driver.rs:858 no_runs_lines -> driver.rs:915 render_driver_tab empty state"
    - "help.rs:216 Driver Tab heading -> help.rs:408 help_lines_documents_every_key_this_phase_binds"
---

<objective>
Let the user opt a project in to driving from the Driver tab itself, by binding `o` there to
the exact confirmation screen the dashboard's `o` already opens — instead of showing a dead
end that sends them back to the dashboard.

Purpose: the Driver tab currently renders `"proj" is not opted in to driving. / Press [o] on
the dashboard to allow it.` (`src/ui/screens/driver.rs:870-872`). The user is already looking
at the project, already knows what they want, and the only thing between them and it is a
navigation round-trip that exists for no reason: `o` is bound on the dashboard
(`src/ui/screens/normal.rs:444-454`) and unbound on every detail tab.

Output: one tab-scoped key binding, one re-pointed sentence of copy, one help row, and the
regression tests that prove the safety gate did not move — only the place the user reaches it.

**The safety contract is the point, not a caveat.** CTRL-03 says a project is driven only when
explicitly opted in, enforced at the process-spawn seam. This change moves WHERE the deliberate
opt-in action is performed and nothing else. It does not make opt-in implicit, does not add a
second registry write path, and does not weaken `DrivableProject`. `src/registry.rs` and
`src/executor/mod.rs` MUST NOT be modified by any task in this plan.
</objective>

<execution_context>
@$HOME/.claude/gsd-core/workflows/execute-plan.md
@$HOME/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md

@src/ui/screens/detail.rs
@src/ui/screens/driver.rs
@src/ui/screens/normal.rs
@src/ui/screens/driver_confirm.rs
@src/ui/screens/help.rs
</context>

<interface_context>
Verified at HEAD `ded839b`. Line numbers are accurate as of planning; re-locate by symbol if
an earlier task in this plan shifts them.

**The path to reuse — do not build a second one:**
- `src/ui/screens/normal.rs:444-454` — the dashboard's `o`: `ScreenAction::Push(Box::new(
  DriverConfirmScreen::new(alias, DriverAction::ToggleOptIn)))`, nothing else.
- `src/ui/screens/driver_confirm.rs:72` — `DriverAction::ToggleOptIn`.
- `src/ui/screens/driver_confirm.rs:216` — the confirm screen routes `ToggleOptIn` to
  `do_toggle_opt_in`.
- `src/ui/screens/driver_confirm.rs:370-405` — `do_toggle_opt_in`: `registry::is_opted_in` ->
  `record_opt_in`/`clear_opt_in` -> `save_config`, with a revert on save failure. It dispatches
  no `Action`. **This is the only opt-in write path and it stays the only one.**
- `src/ui/screens/driver_confirm.rs:266` — `DriverConfirmScreen::name()` returns
  `"driver_confirm"` for every `DriverAction`, so a name assertion alone cannot distinguish
  `Stop` from `ToggleOptIn`. The new test must confirm and observe the effect.

**Where the new arm goes:**
- `src/ui/screens/detail.rs:2179-2181` — the `s` arm; `2192-2198` — the `x` arm with its
  reuse rationale comment at `2183-2191` (the comment shape to imitate).
- `src/ui/screens/detail.rs:2199` — `KeyCode::Char('e')`, the next arm.
- `src/ui/screens/detail.rs:2446` — a generic `KeyCode::Char(c)` arm. The new arm MUST be
  above it or it is unreachable.
- `src/ui/screens/detail.rs:696` — `let current_view = ...`, the guard's binding.
- Grep confirmed: **zero** `Char('o')` / `Char('O')` bindings exist anywhere in `detail.rs`.

**Test harness:**
- `src/ui/screens/driver_confirm.rs:416` — `pub(crate) const ALIAS: &str = "proj"`.
- `src/ui/screens/detail.rs:5852` — `const TEST_ALIAS: &str = "proj"`. **The two are equal**,
  which is what lets the shared fixture drive a `DetailScreen`.
- `src/ui/screens/driver_confirm.rs:426-476` — `pub(crate) fn ctx_with_project(root: &Path)
  -> (AppContext, UnboundedReceiver<Action>)`: registers `ALIAS` with `driver_opt_in: None`
  and sets `config_path` to `root.join("config.json")`, so `save_config` is a real write.
  Reached from `detail.rs` tests as `super::super::driver_confirm::tests::ctx_with_project`.
- `src/ui/screens/detail.rs:5856-5893` — `test_ctx()`: `Config::new()` with **no registered
  project** and `config_path` = the relative `"gsd-meta-manager-test-config.json"`.
- `src/ui/screens/detail.rs:6227-6239` — `driver_action_fixture()`, built on `test_ctx()`.
- `src/ui/screens/detail.rs:6246-6251` — `pushed_screen_name(&ScreenAction) -> Option<String>`.
- `src/ui/screens/detail.rs:6386-6390` — `pressing_x_on_the_driver_tab_pushes_the_stop_confirmation`.
- `src/ui/screens/detail.rs:6396-6421` — `the_three_driver_keys_are_scoped_to_the_driver_tab`.
- `src/ui/screens/mod.rs:33-42` — `trait Screen { fn handle_key(&mut self, ...) -> ScreenAction; ... }`.
  `ScreenAction::Push(Box<dyn Screen>)` at `mod.rs:46`; `ScreenAction` has no `Debug` impl.

**The seam (read-only):**
- `src/executor/mod.rs:155-172` — `pub fn DrivableProject::from_registry(alias, project)`;
  refuses `OptInError::NotOptedIn` at `159` before examining the path.
- `src/executor/mod.rs:764-776` — the existing refusal test, the shape to assert against.

**Symbol paths:** `crate::registry::is_opted_in`, `crate::executor::DrivableProject`,
`crate::error::OptInError`, `crate::config::load_config`.

**Copy:**
- `src/ui/screens/driver.rs:226-229` — the empty-state constants.
- `src/ui/screens/driver.rs:858-877` — `no_runs_lines(alias, opted_in)`, two `Line`s in both
  arms, alias run through `sanitize_render_line` at `859`.
- `src/ui/screens/driver.rs:915` — the call site, `opted_in` read from
  `config.projects[alias].driver_opt_in.is_some()`.
- `src/ui/screens/driver.rs:1978-1989` — `the_empty_run_list_copy_differs_by_opt_in_state`.

**Help:**
- `src/ui/screens/help.rs:181` — `fn row(key: &str, description: &str) -> Line<'static>`.
- `src/ui/screens/help.rs:207` — `row("o", "Toggle driver opt-in (dashboard)")`.
- `src/ui/screens/help.rs:216-225` — the `Driver Tab (detail view)` heading and its rows,
  ending at `row("x", "Stop the live run (asks first)")`; `226` is the closing blank line.
- `src/ui/screens/help.rs:407-433` — `help_lines_documents_every_key_this_phase_binds`, whose
  expected-row array is at `417-426`.
</interface_context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: Bind `o` on the Driver tab and prove the whole path end-to-end</name>
  <files>src/ui/screens/detail.rs</files>

  <behavior>
    - The seam refuses the fixture project before any key is pressed (Err NotOptedIn), so the
      test cannot pass vacuously.
    - `o` on the Driver tab pushes a screen named `driver_confirm`.
    - After that keypress and before any confirmation, the project is STILL not opted in.
    - Feeding `y` to the pushed screen records the opt-in and writes it to config.json.
    - Re-reading config.json from disk, the seam now returns Ok for the same alias.
  </behavior>

  <action>
Insert one new match arm in `DetailScreen::handle_key`, immediately after the `x` arm that
ends at `src/ui/screens/detail.rs:2198` and before the `KeyCode::Char('e')` arm at `2199`. It
must sit above the generic `KeyCode::Char(c)` arm at `2446`, or it never runs.

The arm is `KeyCode::Char('o') if current_view == DetailSubView::Driver =>`, and its body sets
`ctx.needs_redraw = true` then returns
`ScreenAction::Push(Box::new(DriverConfirmScreen::new(self.alias.clone(), DriverAction::ToggleOptIn)))`.
That is the same construction as `src/ui/screens/normal.rs:447-450`; the only difference is
that the alias comes from `self.alias` rather than `ctx.selected_alias()`.

Do NOT call `registry::record_opt_in`, `registry::clear_opt_in` or `save_config` from this
arm. Do NOT add anything to `src/registry.rs`. Do NOT touch `src/executor/mod.rs`. The
confirmation screen owns the single write path, and a second one would be a second thing that
has to stay in agreement with the spawn seam.

Write a doc comment above the arm in the shape of the `x` arm's comment at `2183-2191`,
recording three things: (a) `o` is what the dashboard already binds for this exact action, so
the verb/key mapping stays consistent across the two surfaces — the same argument that comment
already makes for reusing `x`; (b) a grep for `Char('o')` and `Char('O')` across this file
found zero existing bindings, so no collision was resolved on any detail tab; (c) the
`current_view` guard is present **despite** there being no collision, so the key stays scoped
to the tab that explains it rather than silently becoming a global detail-screen opt-in on
tabs where it is undiscoverable and unannounced.

Then add the regression test, placed after
`pressing_x_on_the_driver_tab_pushes_the_stop_confirmation` (`6386-6390`).

First a fixture, `driver_optin_fixture(root: &std::path::Path) -> (DetailScreen, AppContext,
tokio::sync::mpsc::UnboundedReceiver<crate::action::Action>)`. It builds
`DetailScreen::new(TEST_ALIAS.to_string())`, takes its `AppContext` and receiver from
`super::super::driver_confirm::tests::ctx_with_project(root)`, and inserts
`DetailSubView::Driver` into `ctx.detail_sub_view_per_project` under `TEST_ALIAS`. It returns
the receiver so the sender in `ctx.event_tx` stays live for the duration of the test. Fully
qualify the tokio and crate paths at the signature the way `driver_action_fixture` does at
`6227-6231` rather than adding file-level imports.

Document on the fixture WHY it does not reuse `driver_action_fixture()`: that fixture's
`test_ctx()` registers no project and points `config_path` at a relative path, so a real
toggle would fail `UnknownAlias` and, if it did not, would write a config file into the
repository working directory. `ctx_with_project` registers `ALIAS` — the same string as
`TEST_ALIAS` — with no opt-in record and a `config_path` inside a tempdir, which is exactly
the starting state this test needs.

The test is `opting_in_from_the_driver_tab_flips_the_spawn_seam`, and it runs in this order:

1. `let dir = tempfile::tempdir().expect("temp dir");` then build the fixture on `dir.path()`.
2. Assert `crate::executor::DrivableProject::from_registry(TEST_ALIAS, &ctx.config.projects[TEST_ALIAS])`
   is `Err(crate::error::OptInError::NotOptedIn { .. })`, with a message saying the fixture
   must start refused or the rest of the test proves nothing.
3. `let action = screen.handle_key(KeyCode::Char('o'), KeyModifiers::NONE, &mut ctx);` and
   assert `pushed_screen_name(&action).as_deref() == Some("driver_confirm")`.
4. Assert `!crate::registry::is_opted_in(&ctx.config, TEST_ALIAS)` — the load-bearing safety
   assertion. The keypress alone must change nothing; opening the tab and reaching for the key
   is not consent, the confirmation is. A binding that opted in and then asked would pass every
   other assertion in this test.
5. Destructure the pushed screen out and confirm it:
   `let ScreenAction::Push(mut confirm) = action else { panic!("the `o` arm must push") };`
   then `confirm.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);`
6. Assert `crate::config::load_config(&ctx.config_path)` yields a config whose
   `projects[TEST_ALIAS].driver_opt_in.is_some()`, with a message that an opt-in which never
   reached disk is invisible to the driver, which is a different process reading config.json.
7. Assert `crate::executor::DrivableProject::from_registry(TEST_ALIAS,
   &saved.projects[TEST_ALIAS])` is now `Ok` — read off the **reloaded** entry, so the
   assertion is about the bytes on disk and not about in-memory state. The registered path is
   the tempdir root, a real directory, so the `RootUnusable` refusal cannot fire and mask this.

Steps 2 and 7 are one pair: the only thing that happened between them is the keypress and the
confirmation, which is what makes this a proof that the affordance — and nothing else — moved
the seam's answer.
  </action>

  <verify>
    <automated>rtk proxy cargo test opting_in_from_the_driver_tab_flips_the_spawn_seam 2>&amp;1 | grep -qE '^test result: ok\. 1 passed'</automated>
    <automated>test -z "$(git diff --name-only -- src/registry.rs src/executor/mod.rs)"</automated>
  </verify>

  <done>
`o` on the Driver tab pushes `DriverConfirmScreen` with `DriverAction::ToggleOptIn` and
nothing else; the new test passes and covers refused-seam -> keypress -> unconfirmed-still-refused
-> confirmation -> persisted -> accepted-seam; `src/registry.rs` and `src/executor/mod.rs` show
no diff against HEAD.
  </done>

  <reversibility rating="reversible">A single match arm and a test; reverting is deleting both, and no data model, file format or persisted record changed.</reversibility>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Scope the key to the tab, and re-point the refusal copy at it</name>
  <files>src/ui/screens/detail.rs, src/ui/screens/driver.rs</files>

  <behavior>
    - `o` on PhaseList, Pipeline and Defaults pushes no screen whose name starts with `driver_`.
    - The not-opted-in empty state still names `[o]`, and no longer sends the reader elsewhere
      to press it.
    - Both arms of `no_runs_lines` still return exactly two lines.
  </behavior>

  <action>
Two edits, both about making the new binding honest.

**(a) `src/ui/screens/driver.rs:228`.** Change the constant to exactly:

`const NOT_OPTED_IN_NEXT: &str = "Press [o] to allow it.";`

Add or extend its doc comment to record that the key is now bound on this tab, so the sentence
points at where the reader is standing. The replacement is shorter than what it replaces, and
`no_runs_lines` (`858-877`) still builds exactly two `Line`s in both arms, so the empty state's
height and width budget only shrinks. Do NOT touch `NO_RUNS_OPTED_IN_NEXT` at `227`, the alias
line at `870`, or the number of lines either arm returns. Do NOT attempt anything about the
known WR-05 blank-pipeline-row defect at terminal heights 8-13; it is out of scope here.

**(b) `src/ui/screens/driver.rs:1978-1989`.** Strengthen
`the_empty_run_list_copy_differs_by_opt_in_state`. Keep every existing assertion, including
`not_opted_in[1].contains("[o]")`, and add one more:
`assert!(!not_opted_in[1].contains("dashboard"), ...)` — the line must not send the reader to
another surface to press a key that works right here. Word the failure message as: an empty
state whose only instruction is to go somewhere else is the dead end this change removes.

This assertion runs against the **rendered** `Line`, not against the source file, so it stays
true regardless of what the surrounding doc comments say.

**(c) `src/ui/screens/detail.rs:6396-6421`.** Extend
`the_three_driver_keys_are_scoped_to_the_driver_tab`: add `KeyCode::Char('o')` to the inner key
array (currently `i`, `s`, `x`), and rename the test to
`the_driver_action_keys_are_scoped_to_the_driver_tab` since it is no longer three. Leave its
loop body, its fixture choice and its `!pushed.starts_with("driver_")` assertion exactly as
they are — `driver_action_fixture()` registers no project, which is correct here because this
test only reads the pushed screen's name and never confirms anything. Update the doc comment at
`6392-6395` to cover `o`, noting that unlike `i`/`s`/`x` it had no collision to resolve and is
scoped anyway, so the guard is doing real work rather than dodging a conflict.

This is the test that turns the guard from a comment into an enforced property: without it, a
future refactor that drops the `if current_view == ...` clause compiles clean and silently
makes `o` a global opt-in on every detail tab.
  </action>

  <verify>
    <automated>grep -q 'const NOT_OPTED_IN_NEXT: &amp;str = "Press \[o\] to allow it.";' src/ui/screens/driver.rs</automated>
    <automated>rtk proxy cargo test the_empty_run_list_copy_differs_by_opt_in_state 2>&amp;1 | grep -qE '^test result: ok\. 1 passed'</automated>
    <automated>rtk proxy cargo test the_driver_action_keys_are_scoped_to_the_driver_tab 2>&amp;1 | grep -qE '^test result: ok\. 1 passed'</automated>
  </verify>

  <done>
The empty-state constant reads `Press [o] to allow it.`; the copy test asserts both that `[o]`
is named and that no other surface is; the key-scoping test covers four keys across three
non-Driver tabs and passes.
  </done>
</task>

<task type="auto">
  <name>Task 3: Document `o` where keys are documented</name>
  <files>src/ui/screens/help.rs</files>

  <action>
Add one row to the `Driver Tab (detail view)` section of `help_lines()`: place
`row("o", "Toggle driver opt-in for this project")` immediately after
`row("x", "Stop the live run (asks first)")` at `src/ui/screens/help.rs:225`, before the blank
`Line::from("")` at `226`.

Leave the dashboard row at `207` — `row("o", "Toggle driver opt-in (dashboard)")` — exactly as
it is. Both surfaces bind `o` for the same action and the help lists both; that is what makes
the consistency visible instead of coincidental. The two descriptions must not be
byte-identical, or the whole-row assertion cannot tell which section it matched — the dashboard
row is already disambiguated by its trailing parenthetical, and the new row's wording differs
throughout.

Then add `("o", "Toggle driver opt-in for this project")` to the expected-row array in
`help_lines_documents_every_key_this_phase_binds` at `417-426`. Keep the test's existing shape
untouched: it builds each expected row through the same `row()` helper and matches whole lines
with `text.lines().any(|line| line == expected)`, never `contains(key)`. That convention exists
because a substring check on a one-letter key is vacuous — "next" contains "x" — and it matters
more for `o` than for any key already listed, since `o` appears inside almost every description
string in the file.

Do not add a heading, reorder sections, or change any scrolling behaviour: the help screen's
viewport arithmetic (`more_below`, `clamp_scroll`, `PAGE_SCROLL_LINES`) is shared with every
other scrolling pane and is out of scope. One extra row is one extra line of scrollable
content, which that arithmetic already handles.
  </action>

  <verify>
    <automated>rtk proxy cargo test help_lines_documents_every_key_this_phase_binds 2>&amp;1 | grep -qE '^test result: ok\. 1 passed'</automated>
    <automated>rtk proxy cargo test --lib ui::screens::help 2>&amp;1 | grep -qE '^test result: ok\.'</automated>
  </verify>

  <done>
The help popup's Driver Tab section carries an `o` row distinct from the dashboard row, and the
whole-row help test asserts it and passes along with the rest of the help module's tests.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| user keypress -> registry mutation | A keystroke on a rendered tab crosses into a persisted authorization record |
| config.json -> process spawn | `driver_opt_in` on disk is what `DrivableProject::from_registry` reads before any child process exists |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-VMP-01 | Elevation of Privilege | new `o` arm in `detail.rs` | high | mitigate | The arm only pushes `DriverConfirmScreen::new(alias, DriverAction::ToggleOptIn)`; `do_toggle_opt_in` (driver_confirm.rs:370) stays the sole opt-in write path. Enforced by Task 1's `git diff --name-only -- src/registry.rs src/executor/mod.rs` gate returning empty. |
| T-VMP-02 | Elevation of Privilege | opt-in becoming implicit on tab visit/render | critical | mitigate | Opt-in requires a distinct keypress AND `y` on the confirmation. Asserted directly by step 4 of `opting_in_from_the_driver_tab_flips_the_spawn_seam`: after the keypress and before the confirmation, `is_opted_in` must still be false. |
| T-VMP-03 | Tampering | unscoped `o` acting as a global detail-screen opt-in | medium | mitigate | `if current_view == DetailSubView::Driver` guard, enforced by `the_driver_action_keys_are_scoped_to_the_driver_tab` across PhaseList, Pipeline and Defaults. |
| T-VMP-04 | Spoofing | confirmation naming one project while toggling another | medium | mitigate | The alias is `self.alias.clone()` — the detail screen's own project, the one whose tab is on screen — and the same `String` feeds the prompt and the mutation inside `DriverConfirmScreen`. |
| T-VMP-05 | Information Disclosure | alias rendered into the empty-state copy | low | accept | `no_runs_lines` already routes the alias through `sanitize_render_line` (driver.rs:859); this change edits a static constant on the adjacent line and leaves that path untouched. |

No package-manager installs occur in this change (no new Cargo dependencies), so no
supply-chain threat row applies and no Package Legitimacy Gate is required.
</threat_model>

<verification>
Run in order. Every command that needs raw cargo output uses `rtk proxy`, because the `rtk`
wrapper filters `warning:` and `test result:` lines out of plain `cargo` output and any gate
grepping for them would otherwise pass vacuously.

1. Project quality gate (must pass):
   `cargo build && cargo test && cargo clippy -- -D warnings`

2. Test count must go UP from the 772 passing at HEAD `ded839b`, never down:
   `rtk proxy cargo test 2>&1 | grep -E '^test result:' | awk '{s+=$4} END {print s}'`
   Expected: 773 (Task 1 adds one test; Tasks 2 and 3 extend existing ones in place).

3. The five pre-existing `--all-targets` lints must not become six (3x
   `clippy::bool_assert_comparison` in `browser.rs`, 1x `clippy::cmp_owned` in
   `project_creator.rs`, 1x `clippy::items_after_test_module` in `state_reader/mod.rs`):
   `rtk proxy cargo clippy --all-targets --message-format=json 2>/dev/null | grep -c '"code":{"code":"clippy::'`
   Expected: exactly `5`.

4. The safety seam and the registry are untouched:
   `git diff --name-only -- src/registry.rs src/executor/mod.rs` must print nothing.

5. The three new/changed tests, named:
   `rtk proxy cargo test opting_in_from_the_driver_tab_flips_the_spawn_seam 2>&1 | grep -E '^test result: ok\. 1 passed'`
   `rtk proxy cargo test the_driver_action_keys_are_scoped_to_the_driver_tab 2>&1 | grep -E '^test result: ok\. 1 passed'`
   `rtk proxy cargo test help_lines_documents_every_key_this_phase_binds 2>&1 | grep -E '^test result: ok\. 1 passed'`

6. The pre-existing spawn-seam refusal still fires unchanged:
   `rtk proxy cargo test from_registry_refuses_a_project_with_no_opt_in_record 2>&1 | grep -E '^test result: ok\. 1 passed'`
</verification>

<success_criteria>
- A user on the Driver tab of a not-opted-in project can press `o`, confirm, and drive it,
  without ever returning to the dashboard.
- The confirmation is not skippable: the keypress alone leaves the project not opted in, proven
  by an assertion between the press and the confirm.
- `DrivableProject::from_registry` still refuses a project with no opt-in record, and the new
  test observes both sides of that transition off the bytes on disk.
- `o` on a non-Driver detail tab does nothing.
- The empty-state copy names a key that works where it is displayed, in the same two lines.
- The help popup documents the key in its Driver Tab section, asserted as a whole row.
- `cargo build && cargo test && cargo clippy -- -D warnings` passes; 773 tests; `--all-targets`
  lint count still 5; `src/registry.rs` and `src/executor/mod.rs` unmodified.
</success_criteria>

<output>
Create `.planning/quick/260729-vmp-let-the-user-opt-a-project-in-to-driving/260729-vmp-SUMMARY.md` when done
</output>
