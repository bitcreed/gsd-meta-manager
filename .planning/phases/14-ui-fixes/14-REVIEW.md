---
phase: 14-ui-fixes
reviewed: 2026-07-28T00:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - src/state_reader/mod.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/normal.rs
findings:
  critical: 1
  warning: 5
  info: 10
  total: 16
status: issues_found
---

# Phase 14: Code Review Report

**Reviewed:** 2026-07-28
**Depth:** standard
**Files Reviewed:** 3 (diff vs `fc24f012e8dc87a6bb58d04d6477168e7e7653a6`)
**Status:** issues_found

## Summary

Phase 14 shipped four UI fixes (UIFIX-01..04) across `normal.rs`, `detail.rs`, and a
test-only addition to `state_reader/mod.rs`. Build, `cargo test` (241 pass), and
`cargo clippy -- -D warnings` (lib) are green; `cargo clippy --all-targets` fails only
on the five pre-existing debt items declared out of scope.

The refactors themselves are sound in isolation — `alias_badge`, `clamp_scroll`, and
`browse_edit_target` are pure, well-documented, and unit-tested. The defects are at the
seams between those pure functions and the surrounding render/terminal reality, where
the new unit tests cannot see:

- **UIFIX-02 is not actually fixed at two of the three width tiers it claims.** I rendered
  the real dashboard table through `ratatui::backend::TestBackend` with the production
  `Constraint` set: at terminal widths 60–63, 66, and 80–85 the Status column is only 11–12
  cells wide, so `D  R  P  E  V` is clipped to `D  R  P  E ` — the `V` (verified) stage is
  invisible at exactly the `>=60` and `>=80` breakpoints the UI-SPEC enumerates as covered.
  The new test asserts `line.width() == 13` but nothing asserts the column can hold 13.
- **UIFIX-01's alignment claim is contradicted by Unicode display width.** `⏳` (U+23F3) is
  East_Asian_Width=Wide; the badge occupies 3 terminal cells while `⏸`/`▶` occupy 2. The
  guard test asserts `glyph.chars().count() == 2`, which cannot catch this.
- **UIFIX-04 clamps only downward motion.** The up/PageUp sites were left unclamped, so the
  original "first PageUp does nothing" symptom still reproduces after the viewport grows
  (terminal resize).
- **UIFIX-03 leaves the viewer showing pre-edit content** after `$EDITOR` exits, and its
  root fence fails open when `browser_root` is `None`.

No secrets, injection sinks, `unsafe`, or panicking paths were introduced. All `unwrap()`/
`expect()` in the diff are inside `#[cfg(test)]` code.

## Critical Issues

### CR-01: D-R-P-E-V cell is still clipped at the 60 and 80 width tiers — UIFIX-02 acceptance criterion unmet

**File:** `src/ui/screens/normal.rs:80-110` (cell construction) and `src/ui/screens/normal.rs:313-343` (column constraints)

**Issue:** UIFIX-02 shrank the pipeline cell from 15 to 13 columns, and `14-UI-SPEC.md`
(lines 235 and 329) accepts it as "identical at `>=80`, `>=60`, and `<60`". That is false.
The Status column is `Percentage(15)` in the `>=80` tier and `Percentage(20)` in the `>=60`
tier; neither yields 13 cells at the low end of its range.

Reproduced by rendering the production table (same `Constraint` vectors, same
`.highlight_symbol("> ")`, inside the bordered block) with `ratatui 0.30` `TestBackend`:

```
w= 59 |│myproj           Phase 14: UI Fix D  R  P  E  V │|      <- <60 tier, ok
w= 60 |│myproj            Phase 14: UI Fixes D  R  P  E  13/14 pha│|   <- V CLIPPED
w= 70 |│myproj               Phase 14: UI Fixes    D  R  P  E  V  13/14 phas│|
w= 80 |│myproj              Phase 14: UI Fixes   D  R  P  E   13/14 phase 2  │|  <- V CLIPPED
w= 86 |│myproj                Phase 14: UI Fixes    D  R  P  E  V 13/14 phases 2 │|
```

Clipped at widths **60, 61, 62, 63, 66, 80, 81, 82, 83, 84, 85**. 80 columns is the classic
default terminal size, so this is the common case, not an exotic one. A fully verified
phase and an executing phase render identically in the Status column at those widths,
which is a wrong status read, not merely a cosmetic trim.

**Fix:** Give the Status column a hard floor equal to the cell it must hold, instead of a
percentage:

```rust
// >=80 tier
vec![
    Constraint::Percentage(25),
    Constraint::Percentage(30),
    Constraint::Min(13),          // D  R  P  E  V never clips
    Constraint::Percentage(15),
    Constraint::Percentage(15),
],
// >=60 tier
vec![
    Constraint::Percentage(30),
    Constraint::Percentage(35),
    Constraint::Min(13),
    Constraint::Percentage(15),
],
```

and add a render-level regression test (the project has no `TestBackend` test yet) that
asserts the substring `"D  R  P  E  V"` is present in the rendered buffer row at widths
60, 80, and 120 — a unit test on `compact_pipeline` alone cannot catch this class of bug.

## Warnings

### WR-01: Badge glyphs have unequal display width — the alias column the docstring promises to keep aligned is misaligned

**File:** `src/ui/screens/normal.rs:55-76` (`alias_badge`), test at `src/ui/screens/normal.rs:713-728`

**Issue:** The docstring states "At most one badge ever renders, so the alias column stays
aligned." Only the first clause holds. `⏳` U+23F3 is `East_Asian_Width=W` (2 cells);
`⏸` U+23F8 is Neutral and `▶` U+25B6 is Ambiguous (1 cell each). Measured with
ratatui 0.30's own `Span::width()`: pause = 2, hourglass = 3, session = 2. Rendered:

```
|⏸ proj-a   |
|⏳  proj-b  |   <- alias starts one column later
|▶ proj-c   |
|proj-d     |
```

`test_badge_is_never_two_glyphs` asserts `glyph.chars().count() == 2`, which is char count,
not display width, so it certifies an invariant that is not the one the code needs. Many
terminals also render `⏳` with an extra advance beyond what `unicode-width` reports,
widening the drift.

**Fix:** Normalise on display width and assert on it:

```rust
// pad the narrow glyphs so every badge occupies exactly 3 cells
if is_paused {
    Some(("\u{23F8}  ", Color::Cyan))     // 1 + 2 spaces
} else if external_job_waiting {
    Some(("\u{23F3} ", Color::Yellow))    // 2 + 1 space
} else if has_session {
    Some(("\u{25b6}  ", Color::Green))    // 1 + 2 spaces
} else { None }
```

and change the guard test to
`assert_eq!(Span::raw(glyph).width(), 3)` (ratatui re-exports the width impl) instead of
counting chars.

### WR-02: UIFIX-04 clamps only downward scrolling — the "dead PageUp" symptom still reproduces after the viewport grows

**File:** `src/ui/screens/detail.rs:691-693`, `716-718`, `912-914`, `932-936`

**Issue:** The four patched sites all sit on `j`/`Down`/`PageDown`. The matching up paths
still do a bare `saturating_sub` against a stored offset that the renderer only clamps for
*display* (`detail.rs:2866`, `detail.rs:3010`). The stored offset can therefore exceed
`max_scroll` whenever the viewport grows after the offset was set:

1. 100-line doc, terminal is short: `visible_height = 10`, `max_scroll = 90`; page down to 90.
2. Maximise the terminal: `visible_height = 60`, `max_scroll = 40`. Render clamps the *display*
   to 40 but leaves `browser_scroll_offset = 90`.
3. PageUp → 70 (display still 40, nothing moves). PageUp → 50 (still nothing). Only the
   third press moves the viewport.

That is the exact defect UIFIX-04 set out to remove, and
`test_clamp_scroll_first_page_up_moves_viewport` does not cover it because it only walks
the already-clamped path.

**Fix:** Clamp the stored offset before subtracting, at all four up sites:

```rust
BrowserDepth::View => {
    let vp = self.browser_viewport.get();
    let cur = clamp_scroll(cache.browser_scroll_offset, vp.total_lines, vp.visible_height);
    cache.browser_scroll_offset = cur.saturating_sub(PAGE_SCROLL_LINES);
}
```

(and the `saturating_sub(1)` equivalents). Add a test:
`assert_eq!(clamp_scroll(90, 100, 60).saturating_sub(PAGE_SCROLL_LINES), 20)`.

### WR-03: Docs viewer keeps showing pre-edit content after `$EDITOR` exits

**File:** `src/ui/screens/detail.rs:1688-1701`; resume path at `src/main.rs:156-196`

**Issue:** `ScreenAction::SuspendAndEdit` is drained in `main.rs`, the editor runs, the TUI is
re-initialised, and `needs_redraw` is set — but nothing invalidates
`cache.browser_file_content`, which is only ever populated on `Enter`
(`detail.rs:1316-1318`). So the sequence "open a plan in Docs → `e` → edit → save → quit
editor" returns the user to a viewer displaying the *old* text, with a status message
("Editor closed: 14-02-PLAN.md") that actively implies the change landed. The filesystem
watcher does not repopulate this cache either (`browser_file_content` has no other writer).

**Fix:** Reload the file when the editor returns. Minimal change in `main.rs` after
`*terminal = ratatui::init();`:

```rust
// invalidate the cached body of the file that was just edited
for cache in app.ctx.view_cache.values_mut() {
    let matches = cache.browser_current_dir.as_ref().zip(cache.browser_file_name.as_ref())
        .map(|(d, n)| d.join(n) == path).unwrap_or(false);
    if matches {
        cache.browser_file_content = Some(crate::browser::read_md_file(&path));
    }
}
```

(the same gap exists on the Archive tab's `e`, `detail.rs:1681` — fix both while here).

### WR-04: Root fence fails open when `browser_root` is `None`

**File:** `src/ui/screens/detail.rs:3684-3689`

**Issue:**

```rust
if let Some(root) = cache.browser_root.as_ref() {
    if !candidate.starts_with(root) { return Err(NO_FILE); }
}
```

The comment above it reads "Root fence: never hand `$EDITOR` a path outside the browse
root", but with `browser_root == None` (the `Default` value of `ProjectViewCache`) the check
is skipped entirely and *any* candidate is accepted. A guard whose default state is
"disabled" is the wrong polarity, and `test_browse_edit_target_outside_root_is_rejected`
only exercises the `Some(root)` arm, so a future change that stops setting `browser_root`
would silently disarm the fence with a green test suite.

**Fix:**

```rust
let root = cache.browser_root.as_ref().ok_or(NO_FILE)?;
if !candidate.starts_with(root) {
    return Err(NO_FILE);
}
```

plus a test with `browser_root: None` asserting `Err(NO_FILE)`.

### WR-05: `"/milestones/"` read-only guard is separator-specific and duplicated

**File:** `src/ui/screens/detail.rs:3691-3694` (new), duplicating `src/ui/screens/detail.rs:1673-1679`

**Issue:** Archive immutability is enforced with
`candidate.to_string_lossy().contains("/milestones/")`. Two problems:

1. On Windows the path renders as `...\.planning\milestones\v1.2\...`, the substring never
   matches, and archived milestone documents become editable through the Docs tab. `CLAUDE.md`
   commits this project to Windows support (crossterm was chosen over termion for exactly
   that reason), so this is a supported-platform bypass, not a theoretical one.
2. The predicate is now written twice with no shared definition — the classic way two copies
   of a guard drift apart. It also false-positives on any user directory literally named
   `milestones` anywhere under `.planning/`.

**Fix:** Extract one component-aware helper and call it from both sites:

```rust
fn is_archived_path(path: &std::path::Path) -> bool {
    path.components().any(|c| c.as_os_str() == "milestones")
}
```

## Info

### IN-01: Help overlay still documents `e` as "Enqueue next action"

**File:** `src/ui/screens/help.rs:54`
**Issue:** `"  e             Enqueue next action (detail view)"` is now wrong on three tabs
(Archive, Backlog-expanded, and — new in this phase — Docs), where `e` opens `$EDITOR`.
The tab footer was updated (`footer_spans`), the global help was not.
**Fix:** Split the line: `e  Enqueue next action / edit file (Docs, Archive, Backlog)`.

### IN-02: New handoff unit tests duplicate existing integration tests

**File:** `src/state_reader/mod.rs:347-412` vs `tests/state_reader_test.rs:282-342`
**Issue:** Four of the five tests added to `state_reader/mod.rs`
(`test_handoff_md_non_empty_sets_paused`, `test_handoff_json_non_empty_sets_paused_with_context`,
`test_handoff_md_whitespace_only_is_not_paused`, `test_no_handoff_file_is_not_paused`)
assert behaviour already covered verbatim in `tests/state_reader_test.rs`. Only
`test_handoff_json_invalid_is_paused_without_context` adds new coverage. Two suites now
have to be updated together whenever handoff parsing changes.
**Fix:** Keep the invalid-JSON case; delete the four duplicates or delete their integration
counterparts.

### IN-03: Third copy of the `make_planning` fixture, and it silently diverges

**File:** `src/ui/screens/normal.rs:647-657`
**Issue:** The doc comment says it "mirrors the `make_planning` fixture in
`state_reader::tests`", but it drops that fixture's `fs::create_dir_all(parent)` step, so a
nested relative path (`"workstreams/a/STATE.md"`) panics here while working there. Divergent
copies of a "mirror" fixture are a maintenance trap.
**Fix:** Move the fixture into a shared `#[cfg(test)]` helper module and use it from all
three sites.

### IN-04: Redundant second `project_states` lookup

**File:** `src/ui/screens/normal.rs:440-444`
**Issue:** `state` is already bound at `normal.rs:353` from the same map; `is_paused` re-does
`ctx.project_states.get(alias)`. Two lookups of the same key in one closure invite the two
to disagree after a refactor.
**Fix:** `let is_paused = state.map(|s| s.paused).unwrap_or(false);` (matching the
`external_job_waiting` line immediately below it).

### IN-05: Mutating map access for a read-only lookup in the `e` handler

**File:** `src/ui/screens/detail.rs:1692-1695`
**Issue:** `ctx.view_cache.entry(self.alias.clone()).or_default()` clones the alias and
inserts an empty cache entry on every `e` press, purely to read from it.
**Fix:** `let Some(cache) = ctx.view_cache.get(&self.alias) else { return ScreenAction::None };`

### IN-06: `browse_edit_target` does not enforce its own "markdown file" contract

**File:** `src/ui/screens/detail.rs:3659-3697`
**Issue:** Three loose ends: (a) the List branch accepts any `!is_dir` entry with no `.md`
check, relying entirely on `browser::list_dir`'s filter; (b) `list_dir` derives `is_dir`
from `entry.file_type()`, which does not follow symlinks, so a symlink-to-directory named
`x.md` is classified as a file and `e` would hand a directory to `$EDITOR`; (c)
`Path::starts_with` is lexical, so a symlink inside `.planning/` that points outside it
passes the root fence. Also, the root-fence rejection returns `NO_FILE`
("Select a markdown file to edit"), which misdescribes what happened.
**Fix:** Add an explicit
`candidate.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"))` check and a distinct
error string for the fence rejection; optionally `canonicalize()` both sides before
comparing.

### IN-07: The same unbounded-offset bug remains on every non-Docs/Archive tab

**File:** `src/ui/screens/detail.rs:637`, `723`, `858`, `941`
**Issue:** `self.scroll_offset` (Phases, Roadmap, Pipeline, Sessions, … text panes) still
grows without bound and is clamped only at render time (`detail.rs:2087`, `detail.rs:2183`) —
identical to the pre-fix Docs/Archive behaviour. Pre-existing and outside the phase diff,
but the phase's own framing ("scroll offset clamped at four sites") leaves the user-visible
symptom intact on the majority of tabs.
**Fix:** Apply the same `ViewportMetrics`/`clamp_scroll` pattern to `self.scroll_offset`, or
record the deferral explicitly in the phase summary.

### IN-08: `usize as u16` truncation now feeds the scroll clamp

**File:** `src/ui/screens/detail.rs:2849`, `src/ui/screens/detail.rs:2993`
**Issue:** `let total_lines = styled_lines.len() as u16;` silently wraps for documents over
65,535 rendered lines. Previously this only affected display; it now also caps
`clamp_scroll`, so a 70k-line document would refuse to scroll past ~4.4k. Pre-existing cast,
new consumer.
**Fix:** `let total_lines = u16::try_from(styled_lines.len()).unwrap_or(u16::MAX);`

### IN-09: New code is not rustfmt-clean

**File:** `src/ui/screens/detail.rs:4753`, `src/ui/screens/detail.rs:4832`
**Issue:** `cargo fmt --check` flags both new test hunks. The repository is already
non-conformant project-wide (126 diffs), so this is not a gate — but the phase adds to the
pile rather than landing formatted.
**Fix:** `cargo fmt` on the touched hunks; consider a separate quick task for the repo-wide
reformat so `cargo fmt --check` can become a gate.

### IN-10: HANDOFF-derived text is rendered unsanitised in the detail pane

**File:** `src/ui/screens/detail.rs:1968` (and `detail.rs:2125`)
**Issue:** `alias_badge`'s docstring makes a point of never rendering handoff-derived text
("no handoff body can leak onto the dashboard row"), while `pause_context` — the first
non-empty line of an arbitrary `HANDOFF.md`, unbounded in length and uncleaned of control
characters — is rendered straight into a `Span` in the detail pane. ratatui writes cell
symbols through to the terminal, so an ESC byte in that line can emit terminal escape
sequences. Pre-existing (not in this diff), but this phase's tests assert on that exact data
path, so the asymmetry is worth recording.
**Fix:** Truncate and strip control characters where `pause_context` is set
(`state_reader/mod.rs:63-66`, `state_reader/mod.rs:87`):
`s.chars().filter(|c| !c.is_control()).take(200).collect()`.

---

_Reviewed: 2026-07-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
