# Phase 14: UI Fixes - Pattern Map

**Mapped:** 2026-07-28
**Files analyzed:** 4 modified (3 source + 1 state struct)
**Analogs found:** 3 / 4 (test-analog coverage; 1 has no analog anywhere in repo)

This phase's deliverable is four *regression tests* against already-located root causes. The
root causes are in `14-UI-SPEC.md` and are NOT re-derived here. This document maps **how to
write the tests**, because that is where the repo's conventions are non-obvious.

---

## Dev Dependencies (authoritative — do not plan around crates that are absent)

`Cargo.toml`:

```toml
[dev-dependencies]
assert_fs = "1"
```

**Also available to tests via regular `[dependencies]`** (this repo uses `tempfile` as a
*runtime* dep — `config.rs` uses `NamedTempFile` for atomic writes — so unit tests import it
directly):

| Crate | Version | Test usage in repo |
|-------|---------|--------------------|
| `tempfile` | 3 | **Dominant fixture crate** — used by 7 unit-test modules |
| `assert_fs` | 1 (dev) | Used by exactly one file: `tests/registry_test.rs` |
| `serde_json` | 1 | JSON fixture strings |
| `ratatui` / `crossterm` | 0.30 / 0.29 | `KeyCode`, `Line`, `Span`, `Style`, `Color` |

**Not available — do NOT plan a test needing these:** `insta` (snapshot), `rstest`,
`pretty_assertions`, `proptest`, `ratatui::backend::TestBackend` is available via ratatui but
is used **nowhere** in this repo today — introducing it is a new pattern, not a copy.

**Absent capability worth flagging:** there is **no** test in the repo that renders a
ratatui frame or constructs an `AppContext`. See "No Analog Found" below.

---

## File Classification

| Modified File | Role | Data Flow | Test module exists? | Closest Analog | Match |
|---|---|---|---|---|---|
| `src/state_reader/mod.rs` (UIFIX-01) | model / state reader | file-I/O | **YES — line 257** | itself (`mod tests` at 257) | exact |
| `src/ui/screens/normal.rs` (UIFIX-02) | component (render) | transform (pure fn) | **NO — must be created** | `src/app.rs:490` tests | role-match |
| `src/ui/screens/detail.rs` (UIFIX-03) | component (key handler) | event-driven | YES — line 4484 (parse tests only) | **none in repo** | no analog |
| `src/ui/screens/detail.rs` (UIFIX-04) | component (scroll state) | event-driven | YES — line 4484 | **none in repo** | no analog |
| `src/ui/screens/mod.rs` (UIFIX-04 plumbing) | model (`ProjectViewCache`) | state | n/a — plain struct, no tests | n/a | n/a |

---

## Pattern Assignments

### 1. `src/state_reader/mod.rs` — UIFIX-01 HANDOFF pause detection (filesystem fixture)

**Analog: the file's own `#[cfg(test)] mod tests` at `src/state_reader/mod.rs:257-406`.**
This is an *exact* analog — same module, same function-under-test family
(`parse_project_state`), same fixture need (a temp `.planning/` tree).

**The fixture helper already exists — reuse it verbatim, do not write a new one**
(`src/state_reader/mod.rs:263-277`):

```rust
use std::fs;
use tempfile::TempDir;

/// Build a temp project with a `.planning/` dir and the given files
/// (relative paths under `.planning/`). Returns the TempDir (keep it alive).
fn make_planning(files: &[(&str, &str)]) -> TempDir {
    let td = TempDir::new().unwrap();
    let planning = td.path().join(".planning");
    fs::create_dir_all(&planning).unwrap();
    for (rel, content) in files {
        let path = planning.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }
    td
}
```

**Closest existing test to copy in shape** — `test_async_jobs_sets_external_job_waiting`
(`mod.rs:330-345`). This is the same *boolean-flag-from-file-presence* assertion UIFIX-01
needs, and it already covers the negative case in a sibling test:

```rust
#[test]
fn test_async_jobs_sets_external_job_waiting() {
    let td = make_planning(&[
        ("STATE.md", "---\nstatus: executing\n---\n"),
        ("async-jobs/job1.json", "{\"id\":\"job1\"}"),
    ]);
    let state = parse_project_state(&td.path().join(".planning"));
    assert!(state.external_job_waiting);
}

#[test]
fn test_no_async_jobs_means_not_waiting() {
    let td = make_planning(&[("STATE.md", "---\nstatus: executing\n---\n")]);
    let state = parse_project_state(&td.path().join(".planning"));
    assert!(!state.external_job_waiting);
}
```

Substitute `HANDOFF.md` / `HANDOFF.json` for `async-jobs/job1.json` and `state.paused` /
`state.pause_context` for `state.external_job_waiting`. UI-SPEC rows 1-4 and 8 map 1:1 onto
five tests of this exact shape.

**Function under test is private** (`fn detect_handoff`, `mod.rs:57`) — but `mod tests` uses
`use super::*;` so it is directly callable. Prefer testing through the public
`parse_project_state` (as every existing test does) so the wiring `detect_handoff → state.paused`
is also covered; the ground truth is `mod.rs:135`-ish where the tuple is assigned.

**Naming convention in this module:** `test_<subject>_<expected>` snake_case, `test_` prefix.
(Note `browser.rs` uses no `test_` prefix — follow the *local* module's convention.)

---

### 2. `src/ui/screens/normal.rs` — UIFIX-02 D-R-P-E-V leading blank (pure transform)

**No `#[cfg(test)] mod tests` exists in `normal.rs` — one must be created.** There is no test
module anywhere under `src/ui/`, except `detail.rs:4484`.

**Analog: `src/app.rs:490-577`** — the only test module in the repo that tests *pure display
helpers* that feed the dashboard (`format_phase_display`, `classify_status`). Same layer, same
"build a `ProjectState`, assert a rendered string" shape:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::roadmap_md::RoadmapPhase;

    #[test]
    fn test_format_phase_display_completed_milestone() {
        let state = ProjectState {
            completed_phases: 4,
            total_phases: 4,
            milestone: "v1.0".to_string(),
            phases: vec![],
            ..Default::default()
        };
        assert_eq!(format_phase_display(&state), "v1.0 Complete");
    }

    #[test]
    fn test_classify_status_ordinary_unchanged() {
        assert_eq!(classify_status("executing"), StatusCategory::Active);
        assert_eq!(classify_status("blocked"), StatusCategory::Blocked);
    }
}
```

Note the `..Default::default()` struct-literal fixture idiom — used in `app.rs`, `browser.rs`
(`browser.rs:170-176`), and elsewhere. Use it to build `DiskStatus` cases.

**Function under test** (`normal.rs:55-79`) is private and returns `Line<'static>`, not a
`String`:

```rust
fn compact_pipeline(status: &DiskStatus) -> Line<'static> {
    let stages: [(&str, DiskStatus); 5] = [ ("D", DiskStatus::Discussed), ... ];
    let spans: Vec<Span> = stages.iter().map(|(label, threshold)| {
            let color = if *status >= *threshold { Color::Green }
                        else if *status == prev_status(*threshold) { Color::Yellow }
                        else { Color::DarkGray };
            Span::styled(format!(" {} ", label), Style::default().fg(color))   // ← UIFIX-02
        }).collect();
    Line::from(spans)
}
```

**Assertion pattern for a ratatui `Line`** — no existing analog in this repo, so use the
ratatui-native accessors. Recommended (concrete, matches the UI-SPEC's "13 cells" contract):

```rust
let line = compact_pipeline(&DiskStatus::Planned);
let rendered: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
assert!(!rendered.starts_with(char::is_whitespace));   // the actual defect
assert_eq!(rendered, "D  R  P  E  V");                 // exact contract, 13 cells
assert_eq!(line.width(), 13);
```

Per-letter color must survive the fix; assert it the same way the color logic is written:

```rust
// spans[0] is the "D" stage span; colour assertions preserve the Green/Yellow/DarkGray rule
assert_eq!(line.spans[0].style.fg, Some(Color::Green));
```

Note that `Line::width()` and `Span.content` are stable ratatui 0.30 API; `Style.fg` is a
public `Option<Color>` field.

---

### 3. `src/ui/screens/detail.rs` — UIFIX-03 `e` key routing (event-driven)

**No analog exists.** `detail.rs`'s `mod tests` (line 4484) contains only three pure JSON-parse
tests (`parse_waves_manifest`). No test in the repo constructs a `KeyCode`, calls `handle_key`,
or asserts a `ScreenAction`. Verified by grep across `src/` and `tests/`.

**Two hard constraints the planner must design around:**

**(a) `handle_key` takes `KeyCode`, not `KeyEvent`** (`src/ui/screens/mod.rs:26-35`):

```rust
pub trait Screen {
    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers, ctx: &mut AppContext) -> ScreenAction;
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext);
    fn name(&self) -> &str;
}
```

So a test calls `screen.handle_key(KeyCode::Char('e'), KeyModifiers::NONE, &mut ctx)`. No
`KeyEvent::new` needed — this is simpler than the phase brief assumed.

**(b) `ScreenAction` derives nothing** (`src/ui/screens/mod.rs:37-48`) — no `Debug`, no
`PartialEq`. `assert_eq!` on it will **not compile**. Assert with `matches!`:

```rust
let action = screen.handle_key(KeyCode::Char('e'), KeyModifiers::NONE, &mut ctx);
assert!(matches!(&action, ScreenAction::SuspendAndEdit(p) if p == &expected_path));
assert!(matches!(&action, ScreenAction::SetStatusMessage(m) if m == "Archived files are read-only"));
```

Adding `#[derive(Debug)]` to `ScreenAction` is *not* free — `Push(Box<dyn Screen>)` and
`DispatchAction(Box<Action>)` would need `Debug` too. Use `matches!`; do not touch the enum.

**(c) `AppContext` has 18 fields and no `Default`** (`src/ui/screens/mod.rs:111-132`). A test
helper must be written. The **only** construction site in the codebase is `src/app.rs:126-147`
— copy it verbatim into the test module:

```rust
fn test_ctx() -> AppContext {
    AppContext {
        config: Config::new(),                 // Config::new() / Config::default() exist (config.rs)
        config_path: PathBuf::from("/tmp/none.json"),
        project_states: HashMap::new(),
        table_state: TableState::default(),
        filtered_aliases: Vec::new(),
        filter_text: String::new(),
        change_tracker: ChangeTracker::new(),
        detail_sub_view_per_project: HashMap::new(),
        view_cache: HashMap::new(),
        status_message: None,
        error_message: None,
        event_tx: None,
        watcher: None,
        last_refresh: HashMap::new(),
        detail_scroll_offset: 0,
        suggestion_index: 0,
        input_buffer: String::new(),
        needs_redraw: true,
        active_sessions: Vec::new(),
        archive_cache: HashMap::new(),
    }
}
```

`ProjectViewCache` **does** derive `Default` (`mod.rs:57`), so per-tab state is cheap:

```rust
let mut ctx = test_ctx();
ctx.detail_sub_view_per_project.insert("proj".into(), DetailSubView::Browse);
let cache = ctx.view_cache.entry("proj".to_string()).or_default();
cache.browser_depth = BrowserDepth::View;
cache.browser_file_name = Some(md_path.to_string_lossy().into());
let mut screen = DetailScreen::new("proj".to_string());   // detail.rs:41 — 2 fields, trivial
```

**Escape hatch if `test_ctx()` proves heavy:** extract the Browse branch of the `e` arm into a
free function `fn browse_edit_target(cache: &ProjectViewCache) -> Result<PathBuf, &'static str>`
and unit-test *that* purely, calling it from the `KeyCode::Char('e')` arm. This mirrors how
`detail.rs` already factors testable logic out of the render path
(`parse_waves_manifest` / `build_waves_lines`, tested at `detail.rs:4488-4518`).

**Copy the guard + return shape verbatim from the Archive arm** (`detail.rs:1629-1640`) — the
UI-SPEC mandates reusing the same `/milestones/` read-only guard and the same string:

```rust
if let Some(path) = file_path {
    let path_str = path.to_string_lossy();
    if path_str.contains("/milestones/") {
        ctx.needs_redraw = true;
        return ScreenAction::SetStatusMessage("Archived files are read-only".to_string());
    }
    ctx.needs_redraw = true;
    return ScreenAction::SuspendAndEdit(path);
}
```

**Footer** — copy the Archive arm's `[e]dit` construction (`detail.rs:3635-3640`) into the
Browse arm (`detail.rs:3641-3650`), inserted after `[Enter]open  `:

```rust
spans.push(Span::styled("[e]", b));
spans.push(Span::raw("dit  "));
```

`build_footer(sub_view: &DetailSubView) -> Paragraph<'static>` (`detail.rs:3590`) is a private
free function — testable directly from `mod tests` via `use super::*`, no `AppContext` needed.
This is the **cheapest UIFIX-03 assertion available**; consider it the primary footer test.

---

### 4. `src/ui/screens/detail.rs` — UIFIX-04 scroll-offset clamp (state mutation)

**Quick task 260403-p84 landed with ZERO tests.** Confirmed: `PAGE_SCROLL_LINES` (`detail.rs:20`)
has 27 references in `detail.rs` and **none** in any test module. There is no scroll test to copy.

**Nearest in-file analog is the already-clamped sibling code in the same `match` arms** — this is
the pattern the fix should mirror, and it is 20 lines above each defect site. PageDown,
`ArchiveDepth::FileList` (`detail.rs:770-777`):

```rust
ArchiveDepth::FileList { milestone, phase_idx } => {
    if let Some(data) = ctx.archive_cache.get(milestone) {
        if let Some(phase) = data.phases.get(*phase_idx) {
            let max = phase.files.len().saturating_sub(1);
            cache.archive_selected[2] =
                (cache.archive_selected[2] + PAGE_SCROLL_LINES as usize).min(max);
        }
    }
}
```

The `let max = ...; x = (x + N).min(max);` idiom is the established convention in this handler
(also at `detail.rs:540`, `548`, `577`, `589`, `801`, `790`). The four defect sites are the only
ones that use bare `saturating_add` with no `.min(max)`:

```rust
// detail.rs:807-811 (PageDown, Browse/View) — the defect
BrowserDepth::View => {
    cache.browser_scroll_offset = cache
        .browser_scroll_offset
        .saturating_add(PAGE_SCROLL_LINES);
}
```

**The bound to reuse — verbatim from the render path** (`detail.rs:2943-2945`, Browse; identical
formula at `detail.rs:2803-2805`, Archive):

```rust
let visible_height = text_area.height;
let max_scroll = total_lines.saturating_sub(visible_height);
let scroll = cache.browser_scroll_offset.min(max_scroll);
```

**Plumbing pattern for the new cache fields.** `ProjectViewCache` (`mod.rs:57-109`) is a flat
`#[derive(Default)]` struct of plain fields with `///` doc comments. Add the recorded viewport
metrics next to the existing offsets (`archive_scroll_offset` at `mod.rs:75`,
`browser_scroll_offset` at `mod.rs:106`), following the existing doc-comment style:

```rust
pub browser_scroll_offset: u16,
/// Last-rendered viewport metrics for the Browse file view. Recorded by the
/// render pass so the key handler can clamp the stored offset. Zero before
/// the first render — yields max_scroll = 0, a safe floor.
pub browser_total_lines: u16,
pub browser_visible_height: u16,
```

`ProjectViewCache` derives `Default`, so new `u16` fields need no other change.

**Test shape (no analog — new pattern).** The pure part is trivially extractable; strongly prefer
a free helper so the test needs no `AppContext`:

```rust
/// Clamp a scroll offset to the last-rendered viewport.
fn clamp_scroll(offset: u16, total_lines: u16, visible_height: u16) -> u16 {
    offset.min(total_lines.saturating_sub(visible_height))
}

#[test]
fn test_page_down_clamps_at_content_end() {
    // 100-line doc, 30-line viewport → max_scroll = 70
    assert_eq!(clamp_scroll(60 + PAGE_SCROLL_LINES, 100, 30), 70);
    // idempotent past the end
    assert_eq!(clamp_scroll(70 + PAGE_SCROLL_LINES, 100, 30), 70);
    // short document never scrolls
    assert_eq!(clamp_scroll(PAGE_SCROLL_LINES, 10, 30), 0);
    // pre-first-render floor
    assert_eq!(clamp_scroll(PAGE_SCROLL_LINES, 0, 0), 0);
}
```

This mirrors the `parse_waves_manifest` factoring already used in `detail.rs` (pure fn extracted
from the render path, tested directly). Then the four call sites each become
`cache.browser_scroll_offset = clamp_scroll(cache.browser_scroll_offset.saturating_add(PAGE_SCROLL_LINES), t, v);`

The "first PageUp visibly scrolls" property (UI-SPEC backstop) is a corollary of the clamp being
idempotent — assert it as the second line above, not with a rendered fixture.

---

## Shared Patterns

### Test module placement
**Source:** `src/state_reader/mod.rs:257`, `src/app.rs:490`, `src/browser.rs:113`, `src/detail.rs:4484`
**Apply to:** all four fixes
Inline `#[cfg(test)] mod tests { use super::*; ... }` at the **bottom of the module under test**.
CONTEXT.md mandates this. `tests/` (integration) holds only `registry_test.rs` and
`state_reader_test.rs` — do **not** add to `tests/` for this phase.

Caveat: in `src/state_reader/mod.rs` the `mod tests` block sits at line 257 with
`count_backlog_items` *below* it (line 410). Keep new HANDOFF tests inside the existing block at
257; do not create a second one.

### Filesystem fixtures
**Source:** `src/state_reader/mod.rs:263-277` (`make_planning`), `src/browser.rs:118` (`tempfile::tempdir`)
**Apply to:** UIFIX-01, and UIFIX-03 if a real `.md` path is needed
Two co-existing idioms; both are `tempfile`:
- `TempDir::new().unwrap()` + a `make_planning(&[(rel, content)])` helper — `state_reader/mod.rs`
- `tempfile::tempdir().unwrap()` + inline `fs::write` — `browser.rs`

Use the local module's idiom. `assert_fs` is used **only** in `tests/registry_test.rs`; do not
introduce it into unit tests.

### Struct fixtures
**Source:** `src/app.rs:497-503`, `src/browser.rs:170-174`
**Apply to:** UIFIX-01, UIFIX-02
```rust
let state = ProjectState { completed_phases: 4, total_phases: 4, ..Default::default() };
```

### Clamped-selection idiom
**Source:** `src/ui/screens/detail.rs:540-542, 548-551, 577-578, 589-591, 800-804`
**Apply to:** UIFIX-04
`let max = <bound>; x = (x + delta).min(max);` — never bare `saturating_add` on a bounded index.

### Clippy fence
**Source:** CLAUDE.md + CONTEXT.md
`cargo clippy -- -D warnings` (lib target) must stay clean. `--all-targets` currently fails on 5
pre-existing lints in `browser.rs`, `project_creator.rs`, `state_reader/mod.rs` — **new test code
must not add a sixth**. Watch for `clippy::bool_assert_comparison`: `browser.rs:131` already
writes `assert_eq!(entries[0].is_dir, true)`, which is one of the existing `--all-targets` lints.
Write `assert!(x)` / `assert!(!x)` in new tests, not `assert_eq!(x, true)`.

---

## No Analog Found

| Need | Role | Data Flow | Reason |
|---|---|---|---|
| Key-handler test (`handle_key` → `ScreenAction`) — UIFIX-03 | component | event-driven | **No test in the repo constructs a `KeyCode`/`KeyEvent` or calls `handle_key`.** `AppContext` (18 fields, no `Default`) has exactly one construction site: `src/app.rs:126`. `ScreenAction` (`mod.rs:37`) derives neither `Debug` nor `PartialEq`. Mitigations above: copy `app.rs:126-147` into a `test_ctx()` helper, assert with `matches!`, or extract a pure `browse_edit_target` helper. |
| Scroll-offset mutation test — UIFIX-04 | component | event-driven | Quick task 260403-p84 (PageUp/PageDown) shipped with **no** tests; `PAGE_SCROLL_LINES` appears 27× in `detail.rs`, 0× in tests. Recommend extracting a pure `clamp_scroll` helper (mirrors the `parse_waves_manifest` factoring already in this file). |
| Rendered-frame / terminal assertion | component | render | `ratatui::backend::TestBackend` is used nowhere in this repo. The UI-SPEC's two 🧪 backstop rows should be asserted against recorded viewport metrics (`ProjectViewCache`) and `Line`/`Span` inspection, **not** a live terminal. Do not introduce `TestBackend` for this phase. |
| `normal.rs` test module | component | transform | The file has **no** `#[cfg(test)] mod tests`; UIFIX-02 creates the first one. Closest template is `src/app.rs:490`. |

---

## Metadata

**Analog search scope:** `src/` (all modules), `tests/`, `Cargo.toml`
**Test modules found:** 17 inline `#[cfg(test)]` blocks + 2 integration test files
**Key line references verified:** `state_reader/mod.rs:57,257,263,330`; `normal.rs:55,74,421-441`;
`detail.rs:20,41,330,555,595,780,808,1602,1629,2804,2944,3590,3635,3641,4484`;
`ui/screens/mod.rs:26,37,57,75,106,111`; `app.rs:126,490`; `browser.rs:113,131`; `config.rs:9,50`
**Pattern extraction date:** 2026-07-28
