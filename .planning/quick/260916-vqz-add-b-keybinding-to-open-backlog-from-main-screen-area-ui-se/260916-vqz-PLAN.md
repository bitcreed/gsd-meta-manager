---
quick_id: 260916-vqz
phase: quick-260916-vqz
plan: 01
type: execute
wave: 1
depends_on: [260916-vr0]
files_modified:
  - src/ui/screens/normal.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/help.rs
autonomous: true
requirements: [TODO-2026-09-15-backlog-keybinding]
estimate:
  tokens: 45000
  raw_tokens: 45000
  tasks: 2
  confidence: low

must_haves:
  truths:
    - "Pressing `b` on the dashboard with a project selected opens that project's detail view already on the Backlog tab."
    - "The Backlog tab reached by `b` is populated on first paint — identical cache contents to reaching it with Enter then `3`."
    - "Pressing `b` with no project selected does nothing: no screen pushed, no per-project sub-view recorded."
    - "`b` is documented on its own row in the help screen, the one place this codebase documents keys."
  artifacts:
    - "src/ui/screens/detail.rs — `DetailScreen::opened_on(alias, sub_view, ctx)` constructor that runs the target tab's arrival work"
    - "src/ui/screens/normal.rs — `KeyCode::Char('b')` arm in `NormalScreen::handle_key`"
    - "src/ui/screens/help.rs — a `row(\"b\", …)` entry in the Keybindings section, asserted as a whole row"
  key_links:
    - "normal.rs `b` arm -> DetailScreen::opened_on -> detail.rs `switch_to_tab` -> its Backlog arm (the SINGLE backlog load path; 260916-vr0 repairs that arm and this entry path must inherit the repair rather than carry a copy)"
    - "detail.rs `tab_index(&DetailSubView::Backlog)` -> `switch_to_tab` index -> `sub_view_from_index` round trip (no hardcoded `2` at the new call site)"
    - "help.rs `row()` helper -> `help_lines()` -> the whole-row assertion in `help_lines_documents_every_key_this_phase_binds`"
---

<objective>
Bind `b` on the dashboard (`NormalScreen`) to open the selected project's detail
view landed directly on the Backlog tab, with the tab's data already loaded.

Purpose: the overview already shows a per-project backlog count in its Backlog
column, so the count is visible exactly where one-key access to the items behind
it belongs. Today that costs `Enter`, then `3`, and lands on whatever tab was
last active for that project.

Output: one new key binding, one reusable `DetailScreen` constructor, one help
row, and tests that pin the new entry path to the SAME load path the existing
`3` key uses.

No external API integration: this is a local TUI key binding over files the app
already reads; no SDK, service or package install is added.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/todos/pending/2026-09-15-add-b-keybinding-to-open-backlog-from-main-screen.md
@src/ui/screens/normal.rs
@src/ui/screens/detail.rs
@src/ui/screens/help.rs
</context>

<interface_context>
Read before writing — measured, not assumed:

- `src/ui/screens/normal.rs:399-566` — `impl Screen for NormalScreen::handle_key`.
  The `KeyCode::Enter` arm (`:520-528`) is the shape to follow: guard on
  `ctx.selected_alias()`, set `ctx.detail_scroll_offset = 0`, set
  `ctx.needs_redraw = true`, return `ScreenAction::Push(Box::new(DetailScreen::…))`,
  else `ScreenAction::None`.
- `src/ui/screens/normal.rs:449-455` and `:489-494` — the two collision-check
  comments that enumerate the claimed dashboard keys. Measured set today:
  `q j k a c d r x o s / ? Tab Enter Up Down`. `b` is free. The search sub-mode
  short-circuits at `:406-408`, so `b` typed into the filter is unaffected.
- `src/ui/screens/detail.rs:462-528` — `impl DetailScreen`, holding `NAME` and
  `pub fn new(alias: String) -> Self`. The struct's fields are `alias`,
  `scroll_offset`, and four `Cell` viewports.
- `src/ui/screens/detail.rs:534-571` — `tab_index` / `sub_view_from_index`
  (`DetailSubView::Backlog` is index 2), both `pub(crate)`.
- `src/ui/screens/detail.rs:1006-1150` — `fn switch_to_tab(alias: &str,
  new_index: usize, scroll_offset: &mut u16, ctx: &mut AppContext) -> ScreenAction`.
  Private today, called only from `:1898-1926` (the digit keys, `Shift+D`,
  `Left`/`Right`). Its Backlog arm (`:1019-1029`) is the single place
  `backlog::parse_backlog_items` feeds `cache.backlog_items`. It returns
  `ScreenAction::None` unconditionally (`:1148`).
- `src/ui/screens/normal.rs:1123-1177` — `ctx_with_aliases`, the full-field
  `AppContext` fixture; `:1210-1212` `press`; `:1238-1246` `make_planning`
  (writes files into a `TempDir`'s `.planning/`).
- `src/ui/screens/detail.rs:7932` — the idiom for reading a pushed screen out of
  a `ScreenAction`: `ScreenAction::Push(pushed) => pushed.name()`.
- `src/state_reader/backlog.rs:34-98` — `parse_backlog_items` reads
  `.planning/phases/999.N-slug/` directories that contain at least one `.md`
  file, and returns items sorted by number. `BacklogItem` fields are `Untrusted`;
  compare them in tests via `as_raw_for_logic_only()`.
- `src/ui/screens/help.rs:181-211` — `row(key, description)` and the Keybindings
  section of `help_lines()`; `:426-462` — the existing whole-row key test.
</interface_context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: `b` on the dashboard opens the Backlog tab, populated, through the existing load path</name>
  <files>src/ui/screens/detail.rs, src/ui/screens/normal.rs</files>
  <precondition>260916-vr0 ("Backlog tab shows empty despite non-zero count") has merged into this worktree's base — its repair lives in the same `switch_to_tab` Backlog arm this task routes through. If it has not, the equivalence test below still passes (both paths are equally broken) but the feature is not finished; record that in the SUMMARY rather than adding a second load path.</precondition>
  <behavior>
    Three tests in `src/ui/screens/normal.rs`'s `mod tests`, driven through the
    real `handle_key` (never by calling a helper directly — a test that calls the
    constructor cannot catch a key that was never bound):

    - Test 1 (the tracer, end-to-end): a `TempDir` project whose `.planning/`
      holds `phases/999.1-queue-editor/999.1-BACKLOG.md` with a first heading;
      note that `ctx_with_aliases` hardcodes `/nonexistent/<alias>` as the
      project path, so the test must overwrite that entry's `path` with the
      `TempDir`'s path (and keep the `TempDir` alive for the whole test, per the
      `make_planning` doc comment); press `b`; assert (a) the
      returned action is `ScreenAction::Push` whose `name()` is
      `DetailScreen::NAME`, (b) `ctx.detail_sub_view_per_project[alias] ==
      DetailSubView::Backlog`, (c) `ctx.view_cache[alias].backlog_items` is
      non-empty.
    - Test 2 (equivalence — the property that survives 260916-vr0 changing the
      Backlog arm): run the same fixture twice into two separate `AppContext`s —
      one pressing `b` on `NormalScreen`, one pressing `3` on a `DetailScreen`
      built with `DetailScreen::new(alias)` — and assert both contexts end with
      the same `detail_sub_view_per_project` entry and the same
      `backlog_items` (equal length, and equal `dir_name` strings in order via
      `as_raw_for_logic_only()`). A count-only comparison would pass on two
      empty vectors, so this test must also assert the vector is non-empty.
    - Test 3 (the empty-selection guard): a ctx with no registered projects;
      press `b`; assert the action is `ScreenAction::None` and that
      `ctx.detail_sub_view_per_project` gained no entry.

    One test in `src/ui/screens/detail.rs`'s `mod tests`:

    - Test 4 (the pin the new constructor depends on): `switch_to_tab` for the
      Backlog index returns `ScreenAction::None`. The constructor added below
      discards that return value, so this test is what makes the discard safe
      instead of assumed — it goes red the day `switch_to_tab` learns to return
      a `Push`, which is the day the constructor has to stop dropping it.
  </behavior>
  <action>
    In `src/ui/screens/detail.rs`: add `pub(crate) fn opened_on(alias: String,
    sub_view: DetailSubView, ctx: &mut AppContext) -> Self` to the existing
    `impl DetailScreen` block. It builds `Self::new(alias)`, then calls
    `switch_to_tab` with `tab_index(&sub_view)` and the screen's own
    `scroll_offset`, then returns the screen. Derive the index through
    `tab_index` — never write the literal index at the call site, which is the
    whole reason `tab_index`/`sub_view_from_index` are `pub(crate)` and
    round-trip-asserted. Doc-comment it with the reason it exists: the arrival
    work for a tab (backlog parse, git spawn, defaults load, browser init, run
    scan) lives in `switch_to_tab` and nowhere else, so any caller that wants to
    LAND on a tab has to go through it or ship a second, drifting copy. Keep
    `switch_to_tab` private if the borrow works from inside the same module —
    it does, both new callers are in `detail.rs` — and widen its visibility only
    if the compiler forces it.

    In `src/ui/screens/normal.rs`: add a `KeyCode::Char('b')` arm to
    `NormalScreen::handle_key`, placed next to the `KeyCode::Enter` arm because
    it is a second way to open the same screen, not a fourth driver key. It
    guards on `ctx.selected_alias()` exactly as `Enter` does, sets
    `ctx.detail_scroll_offset = 0` and `ctx.needs_redraw = true`, and returns
    `ScreenAction::Push(Box::new(DetailScreen::opened_on(alias,
    DetailSubView::Backlog, ctx)))`. `DetailSubView` is declared in
    `src/app.rs:16`; `normal.rs:8` already imports two items from that module, so
    extend that existing `use crate::app::{…}` line rather than adding a second
    one (`detail.rs:9` is the precedent for importing the enum from there).

    Update the two collision-check comments in `normal.rs` (the driver-keys block
    and the sort-toggle block) so each claimed-key list names `b` too. Those
    lists are how the next person checks a key is free; a list that silently goes
    stale is worse than no list. State in the new arm's own comment that `b` was
    free at the time of writing and that this screen's binding is dashboard-
    scoped.

    Inferred decision, recorded here for audit (the human was unavailable):
    reaching the Backlog tab by `b` does NOT restore the project's previously
    active tab afterwards — `switch_to_tab` records Backlog as that project's
    sub-view, so a later `Enter` lands on Backlog. That matches every other tab
    switch in the app (the sub-view is sticky per project) and the todo asks to
    land "directly on the Backlog sub-view instead of whatever tab was last
    active". A non-sticky variant would need a second, differently-shaped state
    field and is not worth it for a shortcut.
  </action>
  <verify>
    <automated>cargo test --lib --no-fail-fast ui::screens::normal::tests:: ui::screens::detail::tests::</automated>
  </verify>
  <done>
    `cargo test --lib --no-fail-fast ui::screens::normal::tests::` reports 35
    tests (32 today plus the 3 above), all passing, and
    `ui::screens::detail::tests::` reports 75 (74 today plus the pin), all
    passing. `cargo clippy -- -D warnings` exits 0.
  </done>
  <reversibility rating="reversible">A key binding plus a constructor; removing both is a clean revert with no data or on-disk format implications.</reversibility>
</task>

<task type="auto">
  <name>Task 2: document `b` in the help screen and assert it as a whole row</name>
  <files>src/ui/screens/help.rs</files>
  <action>
    Add `row("b", "Open the backlog tab (dashboard)")` to the Keybindings section
    of `help_lines()`, immediately after the `Enter` row, so the two ways to open
    a project's detail view read together. The description must not be
    byte-identical to any other row in the body — the whole-row assertions cannot
    tell two identical rows apart, which the `o` rows' comment already records.

    Extend `help_lines_documents_every_key_this_phase_binds` with a whole-row
    assertion for this pair, built through the same `row()` helper the body uses.
    Do NOT assert it with `text.contains("b")`: that substring is satisfied by
    the word "backlog" itself, and by "Toggle", and by most of the file — the
    exact vacuity this test's own comment was written about. If the existing
    array in that test is the wrong home (it is the Driver-tab section's list),
    add a second array or a single row assertion beside the dashboard-specific
    `Toggle sort` assertion rather than widening the Driver-tab list.
  </action>
  <verify>
    <automated>cargo test --lib --no-fail-fast ui::screens::help::tests::</automated>
  </verify>
  <done>
    `ui::screens::help::tests::help_lines_documents_every_key_this_phase_binds`
    passes, and fails when the new `row("b", …)` line is commented out of
    `help_lines()` (prove that once, by hand, before committing — an assertion
    that cannot fail documents nothing).
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| `.planning/phases/999.*` on disk -> Backlog render | Directory names, headings and file bodies from a repository the user cloned; not authored by this build (SAFE-07). Carried by `Untrusted`. |
| keyboard -> `NormalScreen::handle_key` | Local, trusted input; no new parsing or shell surface is introduced. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-vqz-01 | Tampering | new `b` entry path into the Backlog tab | medium | mitigate | The new path calls `switch_to_tab`, whose Backlog arm calls `backlog::parse_backlog_items` — the ONE site that wraps `dir_name`/`number`/`description` in `Untrusted`. Task 1 forbids a second load path, and the equivalence test (Test 2) is what proves the two entry paths read the same producer. A hand-rolled load in `normal.rs` would produce raw `String`s outside the carrier and is explicitly out of scope. |
| T-vqz-02 | Information disclosure | `DetailScreen::opened_on` for a non-selected alias | low | accept | The constructor is `pub(crate)` and every caller passes `ctx.selected_alias()`; the alias is a registry key the user themself registered. No cross-project read is reachable from this change. |
| T-vqz-03 | Denial of service | synchronous backlog parse on the render thread | low | accept | Unchanged risk, not a new one: the `3` key already performs this same synchronous `read_dir` + small-file read, which `switch_to_tab` documents as "fast filesystem reads". `b` adds a second trigger for identical work, not heavier work. |

No package-manager install is performed by this plan (no `Cargo.toml` change), so
no `T-vqz-SC` supply-chain row and no legitimacy checkpoint apply.
</threat_model>

<verification>
Project gate, run from the repo root after both tasks:

1. `cargo build` exits 0.
2. `cargo test --no-fail-fast` — use `--no-fail-fast`. A plain `cargo test` stops
   at the first failing test BINARY (`driver_reattach`, a documented pre-existing
   pair) and never reaches the suites after it, so the totals it reports are
   unrelated to what this change did. Do not pipe the run through a filtering
   proxy either; use the raw command, because a filtered stream can drop the
   result line a count is read from.
3. `cargo clippy -- -D warnings` exits 0. (`--all-targets` carries 5 documented
   pre-existing lints; the lib-target gate is the project's gate.)

Known-failing baseline this change must not grow: the `driver_reattach` pair and
the environment-dependent git-version-constants test. Record the measured
passed/failed numbers in the SUMMARY against the pre-change baseline taken in the
same worktree — a number quoted from STATE.md was measured on a different tree.

Manual confirmation (optional, not a gate): run the TUI, select a project whose
Backlog column shows a non-zero count, press `b`, and see the Backlog tab open
with items listed rather than blank.
</verification>

<success_criteria>
- `b` on the dashboard pushes the detail screen already on Backlog, with items
  loaded, and the equivalence test proves it shares the `3` key's load path.
- `b` with nothing selected is inert, like every other alias-scoped dashboard key.
- The help screen documents `b` on its own row, asserted as a whole row.
- Both collision-check comment blocks in `normal.rs` list `b` among the claimed
  keys.
- Build, test (`--no-fail-fast`) and `clippy -- -D warnings` all clean, with the
  failing set no larger than the pre-change baseline measured in this worktree.
</success_criteria>

<output>
Create `.planning/quick/260916-vqz-add-b-keybinding-to-open-backlog-from-main-screen-area-ui-se/260916-vqz-SUMMARY.md` when done.
</output>
