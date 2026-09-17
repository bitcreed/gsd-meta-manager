---
quick_id: 260917-fko
phase: quick-260917-fko
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/lib.rs
  - src/experimental.rs
  - src/app.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/help.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/render_escape_guard.rs
  - docs/CONFIGURATION.md
autonomous: true
requirements: [D1, D2, D3, D4, D5, D6]
estimate:
  tokens: 95000
  raw_tokens: 95000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "D1: `GSDMM_EXPERIMENTAL_FEATURES` is read exactly once, at startup, in `App::from_config`, and its resolved bool lives on `AppContext.experimental`; no other production site reads that variable."
    - "D1: the truthy set is `1`/`true`/`yes`/`on`, ASCII-case-insensitive, whitespace-trimmed; unset, empty, `0`, `false`, `no`, `off` and any unrecognised value are OFF."
    - "D2: with the flag off, a user cannot learn a driver exists from the TUI — no Driver tab at any width tier, no Shift+D, no D in the shared footer hint, no driver rows or driver heading in help, no driven badge on the dashboard, and no dashboard r/x/o driver action."
    - "D3: with the flag on, the literal word EXPERIMENTAL is visible on the Driver tab's own pane title and on the help screen's Driver section heading."
    - "D4: `driver_opt_in` still defaults to `None` and `driver_max_concurrent` admission is unchanged in BOTH flag states — the env flag is a new outer layer, never a bypass."
    - "D5: the `drive` CLI subcommand works identically regardless of the flag, so the TUI's own `current_exe() drive` respawn cannot be broken by env scrubbing."
    - "D6: `reconcile_all` and `last_ended_outcomes` still run at startup in both flag states; only the surfaces that display their result are gated."
    - "With the flag off the ten remaining tabs still render whole at their tier widths — the tab-bar cell constants are re-derived for ten tabs, not reused from the eleven-tab values."
  artifacts:
    - "src/experimental.rs — `EXPERIMENTAL_FEATURES_ENV`, `experimental_features_enabled_from(Option<&str>) -> bool`, `experimental_features_enabled() -> bool`"
    - "src/ui/screens/mod.rs — `AppContext.experimental: bool`"
    - "src/ui/screens/detail.rs — `visible_tab_count(bool)`, `tab_bar_full_cells(bool)`, `tab_bar_compact_cells(bool)`, `effective_sub_view(DetailSubView, bool)`"
    - "src/ui/screens/help.rs — `help_lines(experimental: bool)`"
    - "docs/CONFIGURATION.md — a `GSDMM_EXPERIMENTAL_FEATURES` row in the environment-variable table"
  key_links:
    - "std::env -> experimental_features_enabled() -> App::from_config -> AppContext.experimental -> every gate point (ONE read, ONE field, no second env lookup)"
    - "AppContext.experimental -> effective_sub_view() -> the three DetailSubView read sites (handle_key, render, backdrop render) -> the Driver render arms and the Driver key arms all become unreachable from ONE coercion"
    - "AppContext.experimental -> row_badge()'s driven_and_live -> alias_badge() rank 1 -> the attention-first sort ordering (gated at the single source, never per consumer)"
    - "visible_tab_count(experimental) -> tab_titles / windowed_tab_titles / the Right-arrow guard (one count, no second `TAB_COUNT - 1` literal left behind)"
---

<objective>
Put the driver / autonomous-orchestration surface behind a `GSDMM_EXPERIMENTAL_FEATURES`
startup flag, defaulting OFF, and mark what remains EXPERIMENTAL when the flag is on.

Purpose: the driver drives real repos with a real agent. A user who never asked for that
must not discover it by pressing a key. Today the Driver tab, its eleven-tab bar, the
Shift+D hint on every footer, the dashboard's `r`/`x`/`o` keys, the driven badge and nine
help rows are all unconditionally present.

Output: one env-var read, one `AppContext` field, roughly twenty-five gate points threaded
from it, two EXPERIMENTAL markers, the seven test groups `260917-fko-CONTEXT.md` requires,
and one documentation row.

No external API integration, no new crate, no package install: this reads one env var with
`std::env::var` and threads a bool.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/quick/260917-fko-gate-driver-autonomous-features-behind-a/260917-fko-CONTEXT.md
@src/ui/screens/detail.rs
@src/ui/screens/normal.rs
@src/ui/screens/help.rs
</context>

<authority>
`260917-fko-CONTEXT.md` carries six LOCKED decisions (D1..D6) and a verified surface map
(sections A..J). It is the edit authority for this plan. Do not revisit D1..D6.

Its `file:line` references were spot-checked at planning time and a few drifted by a
handful of lines — `App::from_config` is at `src/app.rs:468` (not 469),
`sub_view_from_index` at `detail.rs:688` (not 700), `switch_to_tab` at `detail.rs:1141`
(not 1256). Locate by SYMBOL NAME, not by line number; every symbol the map names exists.

**One correction, already re-derived — use these numbers, not the map's.** Section A asks
for the ten-tab bar widths to be re-derived rather than guessed, and flags its own compact
figure as unverified. Derived here from the documented formula `sum(len + 2) + (n - 1)`,
with the Driver tab's reserved marker cell counted separately:

| tier | 11 tabs (today) | 10 tabs (flag off) | what removal costs |
|---|---|---|---|
| full | 74 + 1 + 22 + 10 = 107 | 67 + 20 + 9 = **96** | -7 label, -1 marker, -2 pads, -1 divider |
| compact | 44 + 1 + 22 + 10 = 77 | 40 + 20 + 9 = **69** | -4 label, -1 marker, -2 pads, -1 divider |

The compact figure is **69**, not the 66 the map guessed. Task 3 pins all four numbers
against the label arrays so none can drift again.
</authority>

<commit_discipline>
The executor runs UNISOLATED on the primary checkout, and the working tree already carries
unrelated dirty paths (`.gsd/`, `.planning/state.json`, two unrelated
`.planning/quick/*-SUMMARY.md`). Those belong to other work and must not be swept in.

- Stage ONLY the files this plan edits, BY EXPLICIT PATH, e.g.
  `git add src/experimental.rs src/lib.rs src/app.rs`.
- **Never** `git add -A`, `git add .`, `git add -u`, or `git commit -a`.
- Before each commit, list the staged set with `rtk proxy git diff --cached --name-only`
  and confirm every path on it appears in this plan's `files_modified`.
- Land on `dev`. Do NOT bump `Cargo.toml` `version`, do NOT tag, do NOT merge to master.
</commit_discipline>

<build_gates>
Run raw. `rtk` strips `warning:` and `test result:` lines, so a piped grep succeeds
vacuously and a filtered run fakes a green gate.

```
rtk proxy cargo build
rtk proxy cargo test --no-fail-fast
rtk proxy cargo clippy -- -D warnings
```

Known-environmental, NOT regressions (from `260917-fko-CONTEXT.md`):
- `envelope::policy` git-version-constants test — installed git 2.53 against constants
  derived from 2.43. This is the single pre-existing `--lib` failure. Baseline: **2087
  passed / 1 failed**.
- `driver::run::tests::the_current_group_agrees_with_the_proc_parse` — a real flake under
  parallel load. Re-run it isolated before calling it a failure.
- `cargo clippy --all-targets` has 4 PRE-EXISTING lint errors (`tests/envelope_carrier_reach.rs`,
  `tests/envelope_config_resolution.rs`, `src/browser.rs`, `src/project_creator.rs`). Do
  not fix them. The plain `cargo clippy -- -D warnings` gate is the one that must stay green.
</build_gates>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: The flag exists, reaches AppContext, and changes one visible string end-to-end</name>
  <files>src/experimental.rs, src/lib.rs, src/app.rs, src/ui/screens/mod.rs, src/ui/screens/detail.rs, src/ui/screens/delete_confirm.rs, src/ui/screens/driver_confirm.rs, src/ui/screens/normal.rs, src/ui/screens/render_escape_guard.rs</files>
  <behavior>
    - `experimental_features_enabled_from` is true for `Some("1")`, `Some("true")`, `Some("TRUE")`, `Some("  yes  ")`, `Some("On")`.
    - It is false for `None`, `Some("")`, `Some("0")`, `Some("false")`, `Some("no")`, `Some("off")`, `Some("maybe")`, `Some("1 1")`.
    - `EXPERIMENTAL_FEATURES_ENV` is the one literal naming the variable; tests reference it rather than respelling the name.
    - With `experimental: false` the detail-view footer's shared tabs hint reads `[1-0]`; with `experimental: true` it reads `[1-0/D]`. Both measured through the real footer builder, never a restated string.
  </behavior>
  <action>
Create `src/experimental.rs` and register it in `src/lib.rs` (alphabetically among the
existing `pub mod` list, between `error` and `executor`). Per locked decision D1 it holds
exactly three public items:

- `pub const EXPERIMENTAL_FEATURES_ENV: &str = "GSDMM_EXPERIMENTAL_FEATURES";`
- `pub fn experimental_features_enabled_from(raw: Option<&str>) -> bool` — the whole parse.
  Trim surrounding whitespace, compare ASCII-case-insensitively against the closed set
  `1`, `true`, `yes`, `on`; everything else, including `None` and the empty string, is
  false. Document on the function that this is deliberately not a presence test, because
  setting the variable to `0` has to mean off and a presence test cannot express that.
- `pub fn experimental_features_enabled() -> bool` — reads `std::env::var(EXPERIMENTAL_FEATURES_ENV)`
  and hands `.ok().as_deref()` to the `_from` variant. This is the ONLY production site
  that touches the process environment for this flag.

Write the unit tests in this module's own `#[cfg(test)] mod tests`, in the repo's
descriptive sentence style, driving ONLY the pure `_from` variant. Do not call
`std::env::set_var` anywhere: it is unsound under the parallel test harness, and the
truthy set is fully observable through the pure function.

Add `pub experimental: bool` to `AppContext` in `src/ui/screens/mod.rs`, with a doc comment
recording that it is resolved once at startup and is the single source every gate point
reads. Set it in the one production construction, `App::from_config` in `src/app.rs`, from
`crate::experimental::experimental_features_enabled()`.

Adding the field breaks nine construction sites (CONTEXT section J). Fix them:
- `src/app.rs`'s exhaustive-destructure guard (find the `let crate::ui::screens::AppContext {`
  destructure beside the `prune_driver_maps` assertions) — add `experimental,` to the
  destructure and classify it in the surrounding comment as a startup-resolved scalar that
  is not alias-keyed and therefore is not pruned.
- The seven test construction sites — `ctx_with_aliases` in `src/ui/screens/mod.rs` and in
  `src/ui/screens/normal.rs`, plus `delete_confirm.rs`, `driver_confirm.rs`, `test_ctx` in
  `detail.rs`, and `probe_ctx` and `chrome_ctx` in `render_escape_guard.rs` — default
  `experimental: true`, so the roughly thirty existing driver tests keep asserting exactly
  what they always asserted. Add one small `with_experimental(self, on: bool) -> Self`
  helper beside the existing test helpers so the new tests flip one field instead of
  rewriting a construction site.

Then wire ONE gate end to end, to prove the threading before the wide edit: in
`src/ui/screens/detail.rs` give `footer_spans` and `build_footer` an `experimental: bool`
parameter, make the shared tabs-hint span read `[1-0]` when it is false and `[1-0/D]` when
true, and pass `ctx.experimental` at the `build_footer` call site in the render method.
Leave `driver_footer_spans` untouched — it is reached only from the Driver tab, which task
2 makes unreachable. Update the `footer_text_at` test helper to take the flag and pass
`true` at its existing call sites, so no existing footer assertion changes meaning.

Do not touch `driver_opt_in`, `driver_max_concurrent`, `admit(...)`, `Commands::Drive`, its
`main.rs` dispatch arm, `reconcile_all` or `last_ended_outcomes` in this task or any other —
D4, D5 and D6 hold all of them fixed.

Commit by explicit path per `<commit_discipline>`.
  </action>
  <verify>
    <automated>rtk proxy cargo build</automated>
    <automated>rtk proxy cargo test --lib experimental</automated>
  </verify>
  <done>
`src/experimental.rs` exists with the three public items and its own passing unit tests;
`AppContext.experimental` compiles at all nine construction sites; `App::from_config` is
the only production caller of `experimental_features_enabled()`; the footer's shared tabs
hint is flag-dependent and asserted both ways through `footer_spans`; `cargo build` exits 0.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Gate every driver surface off the one field, and mark what remains EXPERIMENTAL</name>
  <files>src/ui/screens/detail.rs, src/ui/screens/normal.rs, src/ui/screens/help.rs, src/ui/screens/driver.rs, src/app.rs</files>
  <behavior>
    - Flag off: `visible_tab_count` is 10, `tab_titles` emits no Driver label at any of the three width tiers, `sub_view_from_index(10, false)` is `PhaseList`, Shift+D does not reach the Driver tab, and Right from tab 9 stays at 9.
    - Flag on: `visible_tab_count` is 11, the Driver label is present, Shift+D reaches it, Right from 9 reaches 10.
    - Flag off: `row_badge` never yields the driven badge, and the dashboard `r`/`x`/`o` keys push no driver screen.
    - Flag off: `help_lines(false)` has no driver row and no Driver heading. Flag on: it has both, and the heading carries EXPERIMENTAL.
    - Flag on: the Driver tab's pane title carries EXPERIMENTAL.
  </behavior>
  <action>
Thread `AppContext.experimental` into every surface the CONTEXT surface map lists. Prefer
one gate at the source over many gates at consumers; where the map says "belt and braces",
add the guard only where it costs one clause.

**A — tab plumbing (`detail.rs`).** Keep `TAB_COUNT: usize = 11` as the ARRAY CAPACITY of
`TAB_LABELS_FULL` / `TAB_LABELS_COMPACT` and re-document it as such. Add three accessors
beside it:

- `pub(crate) fn visible_tab_count(experimental: bool) -> usize` — 11 or 10.
- `pub(crate) fn tab_bar_full_cells(experimental: bool) -> u16` — 107 or **96**.
- `pub(crate) fn tab_bar_compact_cells(experimental: bool) -> u16` — 77 or **69**.

Keep the existing `TAB_BAR_FULL_CELLS` / `TAB_BAR_COMPACT_CELLS` consts as the eleven-tab
values the accessors return when experimental, add the two ten-tab constants beside them,
and extend the existing derivation doc comments with the ten-tab arithmetic from
`<authority>` so the numbers stay self-explaining. Do not delete the existing doc comments;
the "tab bar overflow at 80 columns" history they record is still true.

Then:
- `tab_titles(width, active, driver_live)` gains `experimental: bool`. Clamp `active` to
  `visible_tab_count(experimental) - 1`, compare `width` against the accessor values rather
  than the consts, and build from `&LABELS[..visible_tab_count(experimental)]` in both
  whole-bar tiers.
- `tab_label_line` takes `&[&'static str]` instead of `&[&'static str; TAB_COUNT]` so a
  slice can be passed.
- `compact_label_cells` and `windowed_tab_titles` take the flag; every `TAB_COUNT` bound
  inside the windowing cost/growth loop becomes `visible_tab_count(experimental)`.
- `sub_view_from_index(index)` gains the flag; the index-10 arm yields `Driver` only when
  it is set and otherwise falls through to the existing `PhaseList` fallback.
- Leave `tab_index(&DetailSubView)` alone: with the driver unreachable it never sees
  `Driver`, and the round-trip property it exists for is still worth asserting.
- The Right-arrow guard's `current_idx < TAB_COUNT - 1` becomes
  `current_idx < visible_tab_count(ctx.experimental) - 1`. Leave the Left arm as is.
- The `KeyCode::Char('D')` arm gains an `if ctx.experimental` match guard so the key falls
  through unhandled when the flag is off.

**B and C — one coercion covers the Driver key arms and both render dispatches.** Add
`pub(crate) fn effective_sub_view(stored: DetailSubView, experimental: bool) -> DetailSubView`
to `detail.rs`: it returns `PhaseList` for `Driver` when the flag is off and is the identity
otherwise. Document that the Driver sub-view does not exist when the flag is off, so a
stored one has to read as the default tab. Apply it at the THREE sites that read
`ctx.detail_sub_view_per_project` for the current view — the `handle_key` read that feeds
`current_idx`, the main render read, and the `EnqueueScreen` / `DriverInjectScreen` backdrop
render read. That makes `DetailSubView::Driver` unreachable, which is what disarms all ten
`current_view == DetailSubView::Driver` key arms (`j`/`k`/PageUp/PageDown/`f`/`G`/`i`/`s`/`x`/`o`),
both `render_driver_tab` dispatches, and the `switch_to_tab` Driver-entry scan schedule,
without a guard per arm.

**D — footer.** Task 1 already made the shared prefix flag-dependent. Give `footer_spans`
and `build_footer` the flag if task 1 has not already, and confirm the Driver-tab
early-return into `driver_footer_spans` is now reachable only when the flag is on.

**E — dashboard (`normal.rs`).** Gate `driven_and_live` inside `row_badge` at its single
computation site: it resolves false unless `ctx.experimental`. That one clause removes the
driven badge, its rank-1 position, and therefore the attention-first sort's driver-first
ordering, without touching `alias_badge`, `BadgeInputs`, the rank table, or any consumer.
Add an `if ctx.experimental` match guard to each of the three `KeyCode::Char` arms that push
a `DriverConfirmScreen` (`r` start, `x` stop, `o` toggle opt-in), so each key falls through
to its existing non-driver behaviour or to unhandled.

**F — tab-strip live marker.** Nothing to do: the magenta marker span lives inside
`tab_label_line`'s Driver branch, which the sliced label list no longer reaches.

**G — help (`help.rs`).** Change `help_lines()` to `help_lines(experimental: bool)` and have
`HelpScreen::render` pass `ctx.experimental` (the parameter is currently bound as `_ctx` —
rename it). When the flag is off, omit: the three dashboard driver rows (`r` start, `x` stop,
`o` toggle opt-in), the entire Driver Tab section including its heading, blank line and all
eight rows plus the detail-view `o` row, the driven entry while iterating `BADGE_LEGEND`
(skip the tuple whose glyph is `BADGE_DRIVEN`), and the whole Injected Message States
section with its heading and its `INJECTION_LEGEND` / `INJECTION_LEGEND_CONT` loop. Build
the line list so the omissions leave no double blank line and no trailing blank run. When
the flag is on, the Driver Tab heading text gains an em-dash-separated EXPERIMENTAL suffix,
satisfying D3's help-screen half. Keep the existing comments that explain why the two `o`
rows must not be byte-identical — that constraint survives, and the whole-row assertions
still depend on it.

**H — driver pane title (`driver.rs`).** `render_driver_tab` is reachable only when the flag
is on, so its title carries the marker unconditionally. Change the pane identity title from
`" Driver "` to `" Driver — EXPERIMENTAL "` at BOTH sites that spell it — the empty-runs
early-return block and the two-pane detail block — so the marker is present whether or not
the project has runs. Leave `" Runs "` and `run_list_title(...)` alone: the narrow tier
already gives the detail pane the selected run's identity, and overwriting that would cost
the user the information the tier exists to preserve. Use the em dash literal, which this
codebase already ships in `app.rs` status strings; the no-raw-glyph house rule the CONTEXT
cites covers badge and box-drawing glyphs, not punctuation.

**I — defensive early returns (`app.rs`).** Add `if !self.ctx.experimental { return; }` as
the first statement of `start_driver_run` and of `stop_driver_run`, each with a one-line
comment saying the TUI must not be able to start or stop a run while the surface that offers
it does not exist. Adjust the receiver path to whatever the methods actually hold. Do not
change the status-message strings themselves.

**J — do not touch.** `driver_opt_in`, `driver_max_concurrent` and `admit(...)` keep their
exact current semantics and defaults (D4). `Commands::Drive` and its `main.rs` dispatch arm
stay ungated (D5). `reconcile_all` and `last_ended_outcomes` still run (D6). The Defaults tab
option table has no driver key and needs no edit.

Fix every resulting call site and every compile error in the existing tests by passing
`true`, so this task changes no existing assertion's meaning; task 3 adds the flag-off
assertions. Commit by explicit path per `<commit_discipline>`.
  </action>
  <verify>
    <automated>rtk proxy cargo build</automated>
    <automated>rtk proxy cargo test --lib ui::screens</automated>
  </verify>
  <done>
`visible_tab_count`, `tab_bar_full_cells`, `tab_bar_compact_cells` and `effective_sub_view`
exist and are the only bounds used by the tab bar, tab navigation and sub-view reads; the
three dashboard driver keys, `row_badge`'s driven input, the help driver content and both
`start_driver_run` / `stop_driver_run` are gated on `ctx.experimental`; the Driver pane
title and the help Driver heading carry EXPERIMENTAL; `driver_opt_in`,
`driver_max_concurrent`, `Commands::Drive`, `reconcile_all` and `last_ended_outcomes` are
untouched; the full pre-existing test suite still passes with the helpers defaulting on.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Pin both flag states with the seven required test groups, document the variable, close out</name>
  <files>src/ui/screens/detail.rs, src/ui/screens/normal.rs, src/ui/screens/help.rs, src/experimental.rs, docs/CONFIGURATION.md</files>
  <behavior>
    - Group 1 (task 1, confirm still present): the truthy set parses as specified, purely.
    - Group 2, flag OFF: `visible_tab_count(false) == 10`; `tab_titles(w, active, live, false)` emits no Driver label at the full, compact and windowed tiers; `sub_view_from_index(10, false)` is not `Driver`; Shift+D leaves the stored sub-view unchanged; Right from index 9 stays at 9.
    - Group 3, flag ON: `visible_tab_count(true) == 11`; the Driver label is present at the full and compact tiers; Shift+D lands on `Driver`; Right from 9 reaches 10.
    - Group 4, flag ON: the rendered Driver pane title contains the literal EXPERIMENTAL.
    - Group 5, flag OFF: the non-driver footer prefix carries no `D`; `help_lines(false)` contains no driver row and no Driver heading. Flag ON: it contains them, with EXPERIMENTAL on the heading.
    - Group 6, flag OFF: dashboard `r`, `x` and `o` push no screen; `row_badge` never yields the driven badge even with a live observed run.
    - Group 7, both states: `RegisteredProject::driver_opt_in` defaults to `None` and `driver_max_concurrent` admission behaves identically.
    - Anti-drift: all four tab-bar cell constants equal the documented formula applied to the label arrays.
  </behavior>
  <action>
Write the seven test groups `260917-fko-CONTEXT.md` requires, in the repo's descriptive
sentence naming style, each beside the code it covers — tab, navigation and footer tests in
`detail.rs`, dashboard tests in `normal.rs`, help tests in `help.rs`, parse tests already in
`experimental.rs`. Use the `with_experimental` helper from task 1 to flip the flag; never
mutate process env.

Assert against MEASURED output, not restated strings: read the Driver pane title out of a
rendered `TestBackend` buffer the way this repo's existing render tests do, read footer text
through the existing `footer_text_at` helper, and read help rows out of `help_lines(flag)`.

Follow the two testing conventions this codebase already enforces:
- Keys are asserted as WHOLE ROWS, never `contains(key)` — a substring check on a one-letter
  key is vacuous. For the flag-off help assertions, assert the ABSENCE of each whole driver
  row and of the Driver heading line, not the absence of a letter.
- The existing help tests that assert driver rows must be THREADED with `true`, never
  deleted. They are the flag-on half of group 5.

For the flag-off footer assertion, assert the exact expected prefix span text rather than
negative-grepping the letter `D`: the ten non-driver tab names and the `[Esc]back` hint
contain `D`-adjacent text and a letter-level check would be either vacuous or falsely red.

Add the anti-drift test: derive all four tab-bar cell values from `TAB_LABELS_FULL` and
`TAB_LABELS_COMPACT` using `sum(len + 2) + (n - 1)`, adding one cell for the Driver marker
in the eleven-tab cases, and assert each equals its constant. That is what keeps 107 / 96 /
77 / 69 honest when a label is ever renamed.

Add the group-7 regression guard for D4 in whichever module already tests admission: assert
`RegisteredProject::default()`-shaped `driver_opt_in` is `None` and that `admit(...)` returns
the same verdict for the same inputs under both flag values, proving the env flag is an outer
layer rather than a bypass.

Then document the variable: add one row to the environment-variable table in
`docs/CONFIGURATION.md` (the table listing `VISUAL`, `EDITOR`, `TERMINAL`, `TMUX`, `HOME`),
naming `src/experimental.rs` as the consumer, spelling out the accepted truthy values, and
saying the default is OFF and that the Driver tab is hidden entirely when it is. Add a
matching line under the prose that follows the table if the surrounding text warrants it.

Run the three build gates raw and record the measured numbers. Expect the lib suite at the
2087-passed baseline plus the tests added here, still with the one environmental
git-version-constants failure and no other. If a number regresses, re-run the suspect test
isolated before treating it as a regression.

Commit by explicit path per `<commit_discipline>`. Then write
`.planning/quick/260917-fko-gate-driver-autonomous-features-behind-a/260917-fko-SUMMARY.md`
with a "Coordinator inferred decisions (for audit)" section recording, at minimum: that D1's
truthy set was an inferred decision because this codebase had no boolean env-var convention;
that D5 leaves the `drive` CLI subcommand ungated deliberately, because the TUI respawns this
same binary as `current_exe() drive` and the exec layer scrubs env; and that the CONTEXT
surface map's compact ten-tab width was corrected from 66 to 69 by re-derivation.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast</automated>
    <automated>rtk proxy cargo clippy -- -D warnings</automated>
  </verify>
  <done>
All seven required test groups exist and pass in both flag states; the anti-drift test pins
107 / 96 / 77 / 69 against the label arrays; no existing driver test was deleted;
`docs/CONFIGURATION.md` documents `GSDMM_EXPERIMENTAL_FEATURES`, its truthy set and its OFF
default; `cargo clippy -- -D warnings` exits 0; `cargo test --no-fail-fast` shows the
baseline plus the new tests with only the known-environmental failure; SUMMARY.md records
the inferred decisions.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| process environment to TUI | `GSDMM_EXPERIMENTAL_FEATURES` crosses here; whoever can set env for this process can already run the binary with any flag |
| TUI to driven repo | the gated surface is what spawns an agent against a real repository |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-fko-01 | Elevation of Privilege | the env flag treated as a security control | medium | accept | The flag is a DISCOVERY gate, not an authorisation boundary — anyone who can set env can also invoke `drive` directly (D5). D4 keeps `driver_opt_in` as the real consent gate, unweakened: with the flag on, opt-in is still required. Documented as such in `docs/CONFIGURATION.md`. |
| T-fko-02 | Tampering | `AppContext.experimental` re-read or re-derived elsewhere | low | mitigate | One read site (`App::from_config`), one field, one parse helper; no other production caller of `experimental_features_enabled()`. A second read site would let two surfaces disagree mid-session. |
| T-fko-03 | Denial of Service | gating breaks the TUI's own `current_exe() drive` respawn | medium | mitigate | D5 leaves the CLI arm ungated precisely because the envelope/exec layer scrubs env and propagation into the child is not guaranteed; gating it would break the users who DID opt in. |
| T-fko-SC | Tampering | npm/pip/cargo installs | n/a | accept | No package is added. This change introduces no dependency and touches no manifest; the package-legitimacy gate does not apply. |
</threat_model>

<verification>
- `rtk proxy cargo build` exits 0.
- `rtk proxy cargo test --no-fail-fast` shows the 2087-passed baseline plus the tests this
  plan adds, with only the known-environmental `envelope::policy` git-version-constants
  failure. No piped grep — `rtk` strips `test result:`.
- `rtk proxy cargo clippy -- -D warnings` exits 0. The 4 pre-existing `--all-targets` lints
  are out of scope and stay untouched.
- `rtk proxy git diff --cached --name-only` before each commit lists only paths from
  `files_modified`; `.gsd/`, `.planning/state.json` and the two unrelated
  `.planning/quick/*-SUMMARY.md` remain unstaged.
- `Cargo.toml` `version` is unchanged, no tag was created, and the branch is still `dev`.
</verification>

<success_criteria>
- With `GSDMM_EXPERIMENTAL_FEATURES` unset, nothing in the TUI reveals that a driver exists:
  ten tabs, no Shift+D, no `D` in the footer hint, no driver help content, no driven badge,
  no dashboard driver keys.
- With it set to any of `1`/`true`/`yes`/`on` (case- and whitespace-insensitive), the full
  driver surface returns and carries EXPERIMENTAL on the Driver pane title and the help
  Driver heading.
- `0`, `false`, `no`, `off`, the empty string and any unrecognised value are OFF.
- `driver_opt_in`, `driver_max_concurrent`, the `drive` subcommand, `reconcile_all` and
  `last_ended_outcomes` are provably unchanged.
- All three build gates green against the documented baseline.
</success_criteria>

<output>
Create `.planning/quick/260917-fko-gate-driver-autonomous-features-behind-a/260917-fko-SUMMARY.md`
when done, including a "Coordinator inferred decisions (for audit)" section per task 3.
</output>
