# Phase 24: Roadmap Tab Redesign & Detail-Tab Consolidation - Pattern Map

**Mapped:** 2026-09-23
**Tree:** `master` @ `46d7e82` (debug fix `3c0e38f` + `581aa7d` already landed). **Line numbers drift: re-read before editing (D-B06).**
**Files analyzed:** 15 (new or modified)
**Analogs found:** 14 / 15 (the only gap is the lane assigner algorithm; it has no in-tree analog, see "No Analog Found")

All analog paths below are git-tracked source (`git ls-files` checked, 17/17 present). No `.gsd/` mirror paths are used.

---

## File Classification

| New/Modified File | Plan (RESEARCH) | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|---|
| `src/state_reader/roadmap_md.rs` (add `parse_phase_goals`, `parse_planned_build_phases`, build-dep grammar, widen milestone `phase_heading`) | 24-01 | parser/utility | transform (text → structs) | same file: `parse_roadmap_phases` (156-259), `extract_phase_id` (68-92), `roadmap_milestones` (628-778) | exact (self) |
| `src/state_reader/mod.rs` (`ProjectState` + `phase_goals`, `planned_phases`, `milestone_name`) | 24-01 | model | file-I/O → in-memory state | same file: `milestones` field (59-64) + assignment (598-599) | exact (self) |
| `tests/fixtures/roadmaps/{daily-vow,sentriq,ttbook}-ROADMAP.md` + `README.md` (NEW) | 24-01 | test fixture | file-I/O (include_str) | `tests/fixtures/codex/README.md` + `src/executor/codex_json.rs:242-243` | exact |
| `src/ui/roadmap_graph.rs` (keep dep/cycle/wave code; add transitive reduction + lanes + `RoadmapModel`; delete old placement/widget) | 24-02 | pure model | transform | same file: `resolve_deps`/`break_cycles`/`longest_path_layers` (111-288), `layout_graph` (771-...), test helpers (1162-1188) | exact (self) |
| `src/ui/roadmap_view.rs` (NEW widget: list + detail pane, 80/100-col) | 24-02 | component (widget) | request-response (state → Buffer) | `RoadmapGraphWidget` (`roadmap_graph.rs:1049-1156`) + `render_pipeline_tab` master/detail (`detail.rs:4301-4359`) | role-match |
| `src/ui/mod.rs` (`pub mod roadmap_view;`) | 24-02 | config | — | `src/ui/mod.rs:1-3` | exact |
| `src/ui/screens/detail.rs` — Roadmap wiring (render_roadmap rewrite, key arms, shared selection, footer arm, tests) | 24-03 | controller/screen | event-driven (keys) + render | same file: `render_roadmap` (3795-3932), Pipeline `j` arm (1674-1683), guarded `g` arm (2721-2734), `v` arm (3398-3404), `roadmap_graph_fixture` tests (14447-14521) | exact (self) |
| `src/ui/screens/detail.rs` — tab consolidation (constants, `tab_index`, `sub_view_from_index`, digits, footer, PhaseList removal, Docs sub-tab, `switch_to_sub_view`) | 24-04 | controller/screen | event-driven | same file: tab constants (347-453), mappings (782-847), `switch_to_tab` (1283-1428), digits (2219-2260), `footer_spans` (6145-6244), tab tests (9885-10090) | exact (self) |
| `src/ui/screens/mod.rs` (`ProjectViewCache` roadmap fields; docs sub-tab field) | 24-03 / 24-04 | store (view state) | in-memory | `roadmap_box_view` (936-939), Driver view-state block comment (1019-1038) | exact |
| `src/app.rs` (enum: drop `PhaseList`, `#[default] RoadmapViz`; rename index test) | 24-04 | model (enum) | — | `DetailSubView` (15-40), `the_driver_sub_view_is_index_ten_in_both_directions` (4248-4261) | exact |
| `src/app.rs` archive regression tests (4725-5040) — **must stay green, unmodified in behaviour** | 24-04 (guard) | test | event-driven end-to-end | itself | exact |
| `src/ui/screens/render_escape_guard.rs` (hostile goal/planned/milestone_name; `ALL_SUB_VIEWS`; arrival rows; labels) | 24-03 / 24-04 | test (probe) | render | same file: `hostile_project_state` (859-948), `ALL_SUB_VIEWS` (1355-1361), `DETAIL_TAB_ARRIVAL` (1382-1519), `sub_view_label` (1558-1573), `DETAIL_SUB_STATES` (1651-1683) | exact |
| `src/ui/screens/help.rs` (Roadmap key rows; drop stale `r` row) | 24-03 | config (text) | — | `help.rs:227-234` rows + `the_roadmap_graph_toggle_key_is_documented` (884-896) | exact |
| `README.md:47-49`, `docs/ARCHITECTURE.md:165` | 24-04 | docs | — | current text | exact |
| `tests/state_reader_test.rs` (optional parser integration tests) | 24-01 | test | file-I/O | `src/state_reader/mod.rs` `make_planning` tests (736, 815-829) | role-match |

---

## Pattern Assignments

### `src/state_reader/roadmap_md.rs` (parser, transform) — plan 24-01

**Analog:** same file.

**Imports** (lines 1-4) — no new crates:
```rust
use super::phase_num::{phase_key, PhaseNum};
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;
```

**Shared id grammar** (line 49) — reuse, never re-spell:
```rust
const PHASE_ID: &str = r"(?:[A-Za-z]{1,4}-)?[0-9][0-9.]*[A-Za-z]?";
```

**Cached-regex pattern for a helper called per line** (lines 68-71) — copy for `goal_re` / `build_heading_re` if they live in a per-line helper:
```rust
static ID_AT_START: OnceLock<Regex> = OnceLock::new();
let id_at_start =
    ID_AT_START.get_or_init(|| Regex::new(&format!(r"^({id})", id = PHASE_ID)).unwrap());
```

**Heading regex with the `(…)` parenthetical already tolerated** (lines 166-170) — the build-phase recogniser is this plus `(?i)` and `Build\s+phase`:
```rust
let heading_re = Regex::new(&format!(
    r"^\s*#{{2,4}}\s+Phase ({id})(?:\s*\([^)]*\))?:\s+(.+?)\s*$",
    id = PHASE_ID
))
.unwrap();
```

**"First line inside the entry wins, stop at the next header" scan** (lines 222-249) — copy verbatim shape for Goal extraction (`goal_re` in place of `depends_re`):
```rust
let mut j = i + 1;
while j < lines.len() {
    let l = lines[j];
    if is_header(l) {            // stop at the next phase header
        break;
    }
    ...
    if phase.depends_on.is_empty() {
        if let Some(dep_caps) = depends_re.captures(l) {
            phase.depends_on = parse_depends_on(&dep_caps[1]);
        }
    }
    j += 1;
}
```
Goal regex (RESEARCH A8): `(?i)^\s*\*\*Goal(?:\*\*\s*:|:\*\*)\s*(.*)$`. Both `**Goal**:` and `**Goal:**` must be unit-tested.

**Dependency parsing: strip parentheticals, then keyword match** (lines 133-145) — the build-phase dep grammar (`Build phase N`, `Build phases A, B`, `Build phases A-B`) must be a NEW private fn in this shape, applied only inside build-phase entries. Do NOT widen `parse_depends_on` (pinned by `test_depends_on_accepts_every_phase_id_form_and_rejects_a_plural_range`, lines 915-928):
```rust
fn parse_depends_on(text: &str) -> Vec<String> {
    let without_qualifiers = Regex::new(r"\([^)]*\)").unwrap().replace_all(text, " ");
    let phase_ref = Regex::new(&format!(r"Phase\s+({id})", id = PHASE_ID)).unwrap();
    let mut out: Vec<String> = Vec::new();
    for caps in phase_ref.captures_iter(&without_qualifiers) {
        let id = caps[1].trim_end_matches(['.', ',']).to_string();
        if !id.is_empty() && !out.contains(&id) { out.push(id); }
    }
    out
}
```

**Output struct literal** for planned phases (lines 208-216) — `RoadmapPhase` shape is UNCHANGED (adding a field breaks ~20 literals in 10 files):
```rust
Some(RoadmapPhase {
    completed: false,
    number,
    name: caps[2].trim().to_string(),
    description: String::new(),
    total_plans: 0,
    completed_plans: 0,
    depends_on: Vec::new(),
})
```

**Milestone detector fix site** (lines 639-643 and 712-713) — widen `phase_heading` to `(?:[Bb]uild\s+)?[Pp]hase` so `#### Build phase 14 (Milestone 3): …` is a phase heading, not a milestone (Pitfall 2), and is collected into the enclosing `### 📋 Milestone 3 …` scope by the existing `scoped.push` at 723-729:
```rust
let phase_heading = Regex::new(&format!(
    r"^\s*#{{2,4}}\s+Phase ({id})(?:\s*\([^)]*\))?:",
    id = PHASE_ID
))
.unwrap();
...
let is_phase = phase_heading.is_match(line);
if !is_phase && (2..=4).contains(&level) && (version.is_match(text) || numbered.is_match(text)) {
```

**Untrusted carrier construction** (line 684) — for `phase_goals` values:
```rust
label: crate::text::Untrusted::from_untrusted_source(label),
```

**Test pattern** (lines 780-819): inline `&str` roadmaps with `\` line continuations, `assert_eq!` on `Vec<&str>` of numbers, message strings explain WHY:
```rust
#[test]
fn a_zero_padded_detail_heading_merges_with_its_checklist_entry() {
    // ttbook's shape: checklist `Phase 7.1`, details heading `Phase 07.1`.
    let roadmap = "## Phases\n\n\
        - [x] **Phase 7: Consolidation** - c\n\
        ...";
    let phases = parse_roadmap_phases(roadmap);
    let numbers: Vec<&str> = phases.iter().map(|p| p.number.as_str()).collect();
    assert_eq!(numbers, ["7", "7.1"], "one row per phase, first spelling kept");
```
Add: `parse_roadmap_phases` output on the ttbook fixture is UNCHANGED (8..13 only) — GSD-conformance guard (Pitfall 1).

---

### `src/state_reader/mod.rs` (model, file-I/O → state) — plan 24-01

**Analog:** the `milestones` field, added by quick 260923-md1.

**Field + doc pattern** (lines 59-64) — copy the "why not a free string" doc for each new field:
```rust
/// The milestones ROADMAP.md names, with their phase membership
/// ([`roadmap_md::roadmap_milestones`]); drawn by the Roadmap graph.
///
/// Their labels are `Untrusted`, which is why this is not a free-string
/// field: third-party text reaches a cell only through `shown()`.
pub milestones: Vec<roadmap_md::RoadmapMilestone>,
```
New fields: `phase_goals: HashMap<String /*phase_key*/, crate::text::Untrusted>`, `planned_phases: Vec<roadmap_md::RoadmapPhase>`, `milestone_name: Option<crate::text::Untrusted>`. **Never `String`/`Option<String>`/`Vec<String>`** — `tests/spawn_seam_guard.rs:723-745` counts those shapes and `untrusted.rs::the_enumerated_prose_set_is_exactly_the_six_free_text_fields` goes red (Pitfall 3). `ProjectState` derives `Default` (line 42) and every literal uses `..`, so additions are zero-churn.

**Assignment site** (lines 596-599) — add the new parses beside these two:
```rust
let roadmap_path = planning_dir.join("ROADMAP.md");
if let Ok(content) = std::fs::read_to_string(&roadmap_path) {
    state.phases = roadmap_md::parse_roadmap_phases(&content);
    state.milestones = roadmap_md::roadmap_milestones(&content);
```
`milestone_name` comes from the STATE.md frontmatter block ending at line 581 (`state.milestone = fm.milestone;`); `fm.milestone_name` exists (`state_md.rs:27,641`). Wrap: `(!fm.milestone_name.is_empty()).then(|| Untrusted::from_untrusted_source(fm.milestone_name))`.

**Test pattern** (lines 815-829) — `make_planning` (line 736) builds a tempdir `.planning/`:
```rust
#[test]
fn parse_project_state_reads_roadmap_milestones() {
    let roadmap = "# Roadmap\n\n## Milestones\n\n\
        - 🚧 **v2.0 Next** - Phases 1-3 (in progress)\n\n\
        ## Phases\n\n- [ ] **Phase 1: Alpha** - a\n";
    let td = make_planning(&[
        ("STATE.md", "---\nstatus: executing\n---\n"),
        ("ROADMAP.md", roadmap),
    ]);
    let state = parse_project_state(&td.path().join(".planning"));
    assert_eq!(state.milestones.len(), 1);
    assert_eq!(state.milestones[0].label.as_raw_for_logic_only(), "v2.0 Next");
```
The sentriq STATE shape (`milestone_name` AFTER `progress:`) already exists as `SENTRIQ_SHAPED_STATE_MD` at `state_md.rs:1106` — reuse it for the `milestone_name` test.

---

### `tests/fixtures/roadmaps/` (NEW fixture dir) — plan 24-01

**Analog:** `tests/fixtures/codex/README.md` (whole file, 27 lines) and its consumer.

**README convention** — provenance header (tool/source, date, what was captured, where quoted), a table of files, an explicit paragraph for anything RECONSTRUCTED, why the dir is separate, and who reads the files:
```markdown
# Codex `exec --json` transcripts

Captured from **codex-cli 0.155.1** on 2026-09-22 during the 260922-hdj
research probes (...). Quoted in `.planning/quick/.../260922-hdj-RESEARCH.md`, section "Codex CLI facts".

| File | Probe | Exit |
|---|---|---|
| `01-exec-success.jsonl` | ... All 7 lines verbatim. | 0 |

**Line 5 of `02-exec-failure.jsonl` (`turn.failed`) is RECONSTRUCTED.** ...

They are replayed by ... and read through `include_str!` by the unit tests in `src/executor/codex_json.rs`.
```
For roadmaps: list source repo, commit sha (`2dd223b` / `dfc6d2c` / `f420fd5`), date, and state explicitly that product prose was **sanitised** (headings, ids, separators, parentheticals, `**Goal**`/`**Depends on**` line shapes preserved byte-for-byte). Source repos are PRIVATE, this crate is PUBLIC (RESEARCH A9 — checkpoint before commit).

**Consumer pattern** (`src/executor/codex_json.rs:238-243`) — path is relative to the `.rs` file:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SUCCESS: &str = include_str!("../../tests/fixtures/codex/01-exec-success.jsonl");
```
From `src/state_reader/roadmap_md.rs` → `include_str!("../../tests/fixtures/roadmaps/ttbook-ROADMAP.md")`; from `src/ui/screens/detail.rs` → `include_str!("../../../tests/fixtures/roadmaps/...")`. Never read `/home/blk/...` at test time (portability).

---

### `src/ui/roadmap_graph.rs` (pure model, transform) — plan 24-02

**Analog:** same file — KEEP lines 111-288 unchanged (D-A15), rewrite the rest.

**Module doc to preserve / adapt** (lines 1-22) — especially the escaping contract:
```rust
//! **Every phase id, dependency id, phase name and milestone label is
//! third-party text** ... Each one is escaped through
//! `crate::text::render_for_terminal` (or `Untrusted::shown()`) BEFORE it is
//! measured, and only the escaped form is ever stored in a [`GraphLayout`].
```

**Keep (verbatim):** `esc` (113-115), `width_of` (118-120), `type ParentLists` (127), `resolve_deps` (135-173), `break_cycles` (182-230), `cycle_note` (239-263), `longest_path_layers` (266-288), `milestone_tags` (944-958), `phase_markers` (1022-1029).
```rust
fn esc(raw: &str) -> String {
    String::from(crate::text::render_for_terminal(raw))
}
fn width_of(text: &str) -> usize {
    text.chars().count()
}
```

**Pipeline entry to reuse** (`layout_graph`, lines 776-781) — the new `layout_list(...)` starts the same way, then inserts `transitive_reduction` (RESEARCH Pattern 1) before lane assignment; waves = `layer + 1` from the UNREDUCED parents:
```rust
let n = nodes.len();
let labels: Vec<String> = nodes.iter().map(|node| esc(node.id)).collect();
let (mut parents, externals, mut cycles) = resolve_deps(nodes, &labels);
break_cycles(&mut parents, &labels, &mut cycles);
let layer = longest_path_layers(&parents);
```

**Input adapter from state** (lines 1031-1043) — shape for the new `ListNode` adapter (plan 24-02 defines the input struct so it compiles without 24-01's fields):
```rust
pub fn layout_for_state(state: &ProjectState, markers: &[PhaseMarker]) -> GraphLayout {
    let active = roadmap_md::active_milestone_index(&state.milestones, &state.milestone);
    let tags = milestone_tags(&state.milestones, active);
    let mut nodes = nodes_from_phases(&state.phases);
    for (node, phase) in nodes.iter_mut().zip(&state.phases) {
        node.milestone = roadmap_md::milestone_index_of(&state.milestones, &phase.number);
    }
```

**Delete** (Anti-Patterns): `Segment::Reference`, `GapCell`/`Group`/`RowSlots`/`Placement`/`place_rows` (294-~760), `milestone_decorations` (889-939), `split_areas` (987-1006), `RoadmapGraphWidget` (1049-1156), and their tests once ported.

**Test helpers to copy** (lines 1162-1188) — spec tuples → nodes → `render_text`-style `Vec<String>` assertion:
```rust
type Spec<'a> = &'a [(&'a str, &'a str, &'a [&'a str])];

fn owned_deps(spec: Spec<'_>) -> Vec<Vec<String>> {
    spec.iter().map(|(_, _, deps)| deps.iter().map(|d| d.to_string()).collect()).collect()
}
fn lay(spec: Spec<'_>, current: Option<usize>) -> GraphLayout { ... layout_graph(&nodes, &[], current) }
fn text(spec: Spec<'_>, current: Option<usize>) -> Vec<String> { render_text(&lay(spec, current)) }

#[test]
fn roadmap_graph_o1_chain() {
    assert_eq!(text(&[("1", "", &[]), ("2", "", &["1"]), ("3", "", &["2"])], None),
               vec!["1 ─► 2 ─► 3"]);
}
```
Port O1-O10 topologies as lane tests; add `reduction_*` (daily-vow `23: [(20 via 21)]`, sentriq `11: [(9 via 10)]`, ttbook `10/11: [(8 via 9)]`, `12: [(9 via 10)]`) and `lanes_*` matching MOCKUPS.md A/B/C (RESEARCH Pattern 2 prototype output).

**Escape test pattern** (lines 1481-1511, 1526-1545) — hostile ESC + U+202E, assert no raw control char reaches any output line:
```rust
let raw = "v1.0 Evil\u{1b}[31m \u{202E}name";
...
for line in &lines {
    assert!(!line.contains('\u{1b}') && !line.contains('\u{202E}'), "{line:?}");
}
```

**End-to-end parser→model test** (lines 1420-1468, `o10_roadmap` builds a ROADMAP string, then `parse_roadmap_phases` + `roadmap_milestones` + layout) — copy for fixture-driven model tests.

---

### `src/ui/roadmap_view.rs` (NEW widget, state → Buffer) — plan 24-02

**Analog A (widget skeleton, clipping, styling):** `RoadmapGraphWidget` in `src/ui/roadmap_graph.rs:1049-1156`.

**Imports** (from `roadmap_graph.rs:24-32`, adjust):
```rust
use crate::state_reader::PhaseMarker;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};
```
(add `Block, Borders, List, ListItem, ListState, StatefulWidget, Wrap` as needed — same set `detail.rs:22` imports.)

**Zero-area guard + Paragraph rendering (never raw buffer index math)** (lines 1076-1080, 1125-1127):
```rust
impl Widget for RoadmapGraphWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        ...
        Paragraph::new(lines)
            .scroll((self.scroll_offset, h))
            .render(graph, buf);
```

**Marker → style mapping** (lines 1058-1066; also `roadmap_widget.rs:47-55`) — base for D-A04 (● dim / ◉ yellow bold / ○ green / ◌ default):
```rust
Segment::Node { idx, .. } => match self.markers.get(*idx).copied() {
    Some(PhaseMarker::Done) => Style::default().fg(Color::DarkGray),
    Some(PhaseMarker::Current) => Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD | Modifier::REVERSED),
    _ => Style::default(),
},
```

**Analog B (master/detail split + List selection):** `render_pipeline_tab`, `src/ui/screens/detail.rs:4323-4359`:
```rust
let panes = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
    .split(area);
...
let list = List::new(items)
    .block(Block::default().borders(Borders::RIGHT).title(" Phases "))
    .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan))
    .highlight_symbol("> ");
let mut list_state = ListState::default();
list_state.select(Some(selected));
frame.render_stateful_widget(list, left_area, &mut list_state);
```
For the widget (no `Frame`), use `StatefulWidget::render(list, rect, buf, &mut list_state)`. Breakpoint `ROADMAP_SIDE_BY_SIDE_MIN_COLS = 100` → `Layout::horizontal` (detail `clamp(w*2/5, 40, 56)`), else `Layout::vertical` (detail `min(9, h/2)`) — RESEARCH Pattern 5. Selected row: `Modifier::REVERSED` + `▶`.

**Glyph constants** — house rule `\u{…}` escapes in `&'static str` consts (`detail.rs:455-460`, `normal.rs:65-70`):
```rust
/// The Driver label's reserved final cell while a run is live: `◆`.
///
/// A fixed `&'static str` written as a `\u{…}` escape, per the house rule that no
/// raw glyph appears in source (`normal.rs:55-70`). ...
const DRIVER_LIVE_MARKER: &str = "\u{25C6}";
```
Constants list: RESEARCH "Status glyph consts" (●◉○◌▶↑↓·▾▸━║).

**Truncation** — copy `truncate_subject` (`detail.rs:301-317`) semantics (char-wise, `…`), applied to the ESCAPED string; the fn is private to `detail.rs`, so either lift it to a shared `pub(crate)` helper or re-state it here with a doc pointer:
```rust
fn truncate_subject(subject: &str, cols: usize) -> String {
    if cols == 0 { return String::new(); }
    if subject.chars().count() <= cols { return subject.to_string(); }
    let mut out: String = subject.chars().take(cols.saturating_sub(1)).collect();
    out.push('…');
    out
}
```

**Goal wrapping** — `Paragraph::new(..).wrap(Wrap { trim: true })` in the right column of `Layout::horizontal([Length(label_w), Min(0)])` (hanging indent, Mockup C).

**Test pattern — buffer to rows** (`roadmap_graph.rs:1558-1567`):
```rust
fn buffer_rows(buf: &Buffer) -> Vec<String> {
    let area = buf.area;
    (area.y..area.y + area.height)
        .map(|y| {
            (area.x..area.x + area.width)
                .map(|x| buf.cell((x, y)).map_or(" ", |c| c.symbol()).to_string())
                .collect::<String>()
        })
        .collect()
}
```
**Test pattern — no panic / no bleed** (`roadmap_graph.rs:1569-1603`) — port as `assert_no_panic_at_tiny_sizes` for the new widget: render at `(1,1) (5,3) (10,3) (20,5)`, then render into an inner `Rect::new(3,2,5,3)` of a 20x10 buffer pre-filled with `x` and assert every outside cell equals `before`:
```rust
let inner = Rect::new(3, 2, 5, 3);
...
for y in 0..10u16 { for x in 0..20u16 {
    let inside = (3..8).contains(&x) && (2..5).contains(&y);
    if !inside { assert_eq!(buf.cell((x, y)), before.cell((x, y)), "bled at ({x},{y})"); }
}}
```
Add width tests at 80 (stacked) and 120 (side-by-side); sentriq 12 detail pane contains `nothing declared` and `can run any time`.

---

### `src/ui/mod.rs` (config) — plan 24-02

**Analog:** lines 1-3:
```rust
pub mod roadmap_widget;
pub mod roadmap_graph;
pub mod screens;
```
Add `pub mod roadmap_view;` (one line; this is the only shared-file touch in 24-02).

---

### `src/ui/screens/detail.rs` — Roadmap tab wiring (screen, event-driven + render) — plan 24-03

**Analog:** same file.

**Imports already present** (lines 14-24): `PhaseMarker`, `Untrusted`, `roadmap_graph`, `RoadmapWidget`, `Layout/Margin/Rect`, `List/ListItem/ListState/Paragraph/Tabs/Wrap`, `std::cell::Cell`. Add `use crate::ui::roadmap_view;`.

**Escape helper for every third-party string** (lines 78-80):
```rust
fn shown(value: &str) -> String {
    crate::text::render_for_terminal(value).to_string()
}
```

**Header block to keep at the top of the new Roadmap render** (lines 3805-3876) — Path / Status+Milestone / `unreadable_state_line` / `recovered_state_line` / Paused / change-tracker banner are ALREADY here (D-B02, D-B08 satisfied by keeping this block; compress to the mockup's one summary line + conditional extras for 80x24):
```rust
if let Some(line) = unreadable_state_line(state) { header_lines.push(line); }
if let Some(line) = recovered_state_line(state) { header_lines.push(line); }
if state.paused { ... "  Paused: " / "  Paused (HANDOFF file present)" ... }
if let Some(event) = ctx.change_tracker.latest_change(alias) {
    let elapsed = ChangeTracker::format_elapsed(event.timestamp);
    let banner = format!("  [ {} -- {} ]", shown(&event.description), elapsed);
```

**Box-view branch to keep for `v`** (lines 3878-3902) — unchanged; the `else` branch (3903-3926) is replaced by the `roadmap_view` widget:
```rust
if ctx.view_cache.get(alias).is_some_and(|c| c.roadmap_box_view) {
    let current_phase_num = state.active_phase_number();
    ...
    let roadmap_widget = RoadmapWidget { phases: &state.phases, current_phase_num,
        disk_statuses: &state.phase_disk_statuses, scroll_offset: clamped_offset };
    frame.render_widget(roadmap_widget, roadmap_area);
} else {
    // NEW: roadmap_view widget
}
```

**Viewport recorded through a `Cell` because `render` takes `&self`** (lines 648-658, 3911-3914):
```rust
self.generic_viewport.set(ViewportMetrics {
    total_lines: total,
    visible_height: graph_rect.height,
});
```
If the list needs a persisted `ListState` offset, add a sixth `Cell` on `DetailScreen` with the same doc style as `git_commit_viewport` (666-672) and initialise it in `new` (686-696).

**Empty states to preserve** (D-B08): `"  No state data available for roadmap."` (3929), `"  No roadmap data available"` (PhaseList, 3689).

**Glyph decision source of truth (D-B12)** — `state.phase_marker(phase)` (`state_reader/mod.rs:283-290`) / `roadmap_graph::phase_markers` (1022-1029). Plans counts: `crate::state_reader::phase_plan_counts(phase, &state.phase_disk_statuses)` (mod.rs:406-417; PhaseList use at detail.rs 3700-3706). Stage badge: `disk_suffix_spans(&phase.number, &state.phase_disk_statuses, ctx.config.preferences.gsd_integration)` (1206-1280; PhaseList use at 3708-3710).

**Selection key arm pattern** — copy the Pipeline `j` arm (1674-1683); RoadmapViz currently falls to the `_ =>` generic scroll (1799-1808). Add an explicit arm guarded on `!roadmap_box_view` so the box view keeps generic scroll:
```rust
DetailSubView::Pipeline => {
    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
    if let Some(state) = ctx.project_states.get(&self.alias) {
        if !state.phases.is_empty() {
            let max = state.phases.len().saturating_sub(1);
            cache.pipeline_selected = (cache.pipeline_selected + 1).min(max);
        }
    }
    ctx.needs_redraw = true;
}
```
Same four sites for j / k / PgDn / PgUp (research inventory: 1674, 1828, 1954, 2118 for Pipeline; 1799, 1910, 2077, 2207 for the `_ =>` fallback). In the Pipeline arms also write `cache.roadmap_selected = Some(phase_key(&state.phases[cache.pipeline_selected].number))` (bidirectional share, D-B03).

**Guarded single-view key arm pattern** (2720-2734, `g` on Browse; 3396-3404, `v` on Roadmap) — use for `g`, `G`, `h`, `l`, `[`, `]`:
```rust
// Roadmap tab only: flip between the dependency graph (default) and
// the box list. `v` is bound nowhere else on this screen.
KeyCode::Char('v') if current_view == DetailSubView::RoadmapViz => {
    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
    cache.roadmap_box_view = !cache.roadmap_box_view;
    self.scroll_offset = 0;
    ctx.needs_redraw = true;
    ScreenAction::None
}
```
`Enter | Space` is ONE arm with a per-view `match` (2261-2263, `_ => ScreenAction::None` covers RoadmapViz today) — add a `DetailSubView::RoadmapViz` branch that distinguishes `code` (Space = fold, Enter = open in Phases / Docs›Milestones).

**Enter → Phases handoff** — never a literal index (`opened_on` doc, 715-720):
```rust
switch_to_tab(&self.alias, tab_index(&DetailSubView::Pipeline), &mut self.scroll_offset, ctx)
```
after writing `pipeline_selected` from `roadmap_selected`. Enter on a planned build phase → status message (RESEARCH Pattern 4).

**Footer arm** (6228-6233) — extend with Roadmap hints (`[j/k]`, `[h/l]`, `[Space]`, `[Enter]`, `[v]`); keep `[v]` and `[e]` (pinned by `roadmap_graph_tab_footer_advertises_v`, 14513-14521):
```rust
DetailSubView::RoadmapViz => {
    spans.push(Span::styled("[v]", b));
    spans.push(Span::raw("iew  "));
    spans.push(Span::styled("[e]", b));
    spans.push(Span::raw("nqueue  "));
}
```

**Screen-level test fixture to copy/extend** (14447-14479):
```rust
fn roadmap_graph_fixture() -> (DetailScreen, AppContext) {
    use crate::state_reader::roadmap_md::RoadmapPhase;
    let phase = |n: &str, deps: &[&str]| RoadmapPhase {
        number: n.to_string(), name: format!("Name {n}"), description: String::new(),
        completed: false, total_plans: 0, completed_plans: 0,
        depends_on: deps.iter().map(|d| d.to_string()).collect(),
    };
    let mut ctx = test_ctx();
    ctx.project_states.insert(TEST_ALIAS.to_string(), crate::state_reader::ProjectState {
        phases: vec![phase("1", &[]), phase("2", &["1"]), phase("3", &["2"])],
        ..Default::default()
    });
    ctx.detail_sub_view_per_project.insert(TEST_ALIAS.to_string(), DetailSubView::RoadmapViz);
    (DetailScreen::new(TEST_ALIAS.to_string()), ctx)
}
```
Key helpers: `TEST_ALIAS = "proj"` (10415), `test_ctx()` (10419-10460, `experimental: true` by default), `press(screen, ctx, code)` (10524-10526). For fixture tests, build `ProjectState` via `parse_project_state` on a tempdir holding the vendored fixture, or parse the `include_str!` content directly into `phases`/`milestones`/new fields.

**Rendered-text test pattern (TestBackend buffer → String)** (11630-11656) — add an 80x24 sibling (e.g. `render_detail_to_text_at(screen, ctx, w, h)`), keep the 120x30 one:
```rust
fn render_detail_to_text(screen: &DetailScreen, ctx: &AppContext) -> String {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let (width, height) = (120u16, 30u16);
    let mut terminal =
        Terminal::new(TestBackend::new(width, height)).expect("TestBackend terminal");
    terminal
        .draw(|frame| screen.render(frame, frame.area(), ctx))
        .expect("draw the detail screen");
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| (0..width)
            .map(|x| buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" ").to_string())
            .collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
```
Assertions per RESEARCH: `text.matches(" 20 ").count() == 1` (daily-vow), each milestone label once, `·` on row 20 when 23 selected, sentriq 12 `nothing declared` / `can run any time`. Rewrite `roadmap_graph_tab_draws_the_graph_by_default_and_v_toggles_the_box_list` (14481-14502; it asserts `"1 ─► 2 ─► 3"` and `generic_viewport.total_lines == 1`).

---

### `src/ui/screens/detail.rs` — Tab consolidation (screen, event-driven) — plan 24-04 (LAST, D-B06)

**Analog:** same file. Re-read before editing.

**Tab constants** (347-453) — new values per RESEARCH Pattern 6; keep the doc style that states the arithmetic `Σ(len + 2) + (n − 1)` (+1 marker with Driver):
```rust
pub(crate) const TAB_COUNT: usize = 11;
...
const TAB_LABELS_FULL: [&str; TAB_COUNT] = [
    "1:Phases", "2:Roadmap", "3:Backlog", "4:Git", "5:Pipe", "6:Queue",
    "7:Sess", "8:Arch", "9:Cfg", "0:Docs", "D:Drive",
];
const TAB_LABELS_COMPACT: [&str; TAB_COUNT] = [
    "1:Ph", "2:Rd", "3:Bk", "4:Gt", "5:Pp", "6:Qu", "7:Ss", "8:Ar", "9:Cf", "0:Dc", "D:Dr",
];
pub(crate) const DRIVER_TAB_INDEX: usize = 10;
pub(crate) const TAB_BAR_FULL_CELLS: u16 = 107;
pub(crate) const TAB_BAR_COMPACT_CELLS: u16 = 77;
pub(crate) const TAB_BAR_FULL_CELLS_NO_DRIVER: u16 = 96;
pub(crate) const TAB_BAR_COMPACT_CELLS_NO_DRIVER: u16 = 69;
```
→ 9 / 8 / 89 / 63 / 78 / 55 (verified by the anti-drift test `the_tab_bar_widths_are_the_label_arrays_own_arithmetic`, 10018-10055 — do not hand-edit the test's `derive`).

**Mappings** (782-847) — exhaustive `match` without wildcard in `tab_index` (compile error on a missing variant); fallback becomes `RoadmapViz` in `sub_view_from_index` (809, 825) and `effective_sub_view` (844). Under Pattern 6, `DetailSubView::Archive => 7` (same as `Browse`), `sub_view_from_index(7) == Browse`.

**`switch_to_tab` split** (1283-1428) — extract the body into `switch_to_sub_view(alias, view: DetailSubView, scroll_offset, ctx) -> ScreenAction`; `switch_to_tab` becomes:
```rust
fn switch_to_tab(alias: &str, new_index: usize, scroll_offset: &mut u16, ctx: &mut AppContext) -> ScreenAction {
    let new_view = sub_view_from_index(new_index, ctx.experimental);
    switch_to_sub_view(alias, new_view, scroll_offset, ctx)
}
```
The Archive arrival block (1407-1425) moves unchanged into `switch_to_sub_view`:
```rust
if new_view == DetailSubView::Archive {
    let cache = ctx.view_cache.entry(alias.to_string()).or_default();
    if cache.archive_milestones.is_empty() && !cache.archive_loading {
        cache.archive_loading = true;
        if let (Some(project), Some(tx)) = (ctx.config.projects.get(alias), &ctx.event_tx) {
            ...
            tokio::task::spawn_blocking(move || {
                let milestones = crate::archive::discover_milestones(&milestones_dir);
                let _ = tx.send(Action::ArchiveMilestonesDiscovered { alias: alias_owned, milestones });
            });
```

**`opened_on` MUST route through the sub-view form** (725-730). Today:
```rust
pub(crate) fn opened_on(alias: String, sub_view: DetailSubView, ctx: &mut AppContext) -> Self {
    let mut screen = Self::new(alias);
    let index = tab_index(&sub_view);
    let _ = switch_to_tab(&screen.alias, index, &mut screen.scroll_offset, ctx);
    screen
}
```
Once `tab_index(&Archive) == tab_index(&Browse) == 7`, this round-trips `Archive → 7 → Browse` and the app.rs archive regression tests (below) would open Docs › Files, schedule NO milestone discovery, and fail on `"Archive > v1.2"`. Replace the two middle lines with `let _ = switch_to_sub_view(&screen.alias, sub_view, &mut screen.scroll_offset, ctx);` and update the doc paragraph at 715-720 (the "index comes from `tab_index`" argument no longer holds for sub-tabs).

**Digit arms** (2219-2229) — `'1'..='8'` → 0..7; DELETE the `'9'` and `'0'` arms so they fall through to `_ => ScreenAction::None` (3409) — D-B10. `Shift+D` (2240-2242) and Left/Right (2244-2260, bounded by `visible_tab_count`) stay; update the comment "last tab is index 9".

**Render dispatch ×2** (3464-3485 in `render`, 5074-5097 in `render_main_only`) — drop the `PhaseList` arm in both (a tier applied to one path only is a known hazard, see 3440-3443 comment). Delete `render_phase_list` (3607-3792).

**Docs sub-tab strip** — analog breadcrumb row: `render_archive_tab` 4631-4639 and `render_browser_tab` 4817-4832 both reserve a 1-row header via `Layout::vertical([Constraint::Length(1), Constraint::Min(0)])`. Put the `Files | Milestones` strip in that header region (or a new 1-row slot above it) in BOTH renders. Keep `archive_breadcrumb`'s `"Archive"` literal (4964-4967) or update the three app.rs assertions in the same commit (never weaken).

**Docs sub-tab toggle key** (`m`, discretion) — copy the guarded-arm shape of `g` on Browse (2720-2734), guard `if matches!(current_view, DetailSubView::Browse | DetailSubView::Archive)`, body calls `switch_to_sub_view(.., DetailSubView::Archive | Browse, ..)`.

**Footer** (6093-6244) — `"[1-0/D]"` at 6105 and `if experimental { "[1-0/D]" } else { "[1-0]" }` at 6155 → `"[1-8/D]"` / `"[1-8]"`; add `[m]` hint to both `Archive` (6198-6203) and `Browse` (6204-6215) arms. Driver footer widths (`DRIVER_FOOTER_FULL_CELLS` etc.) are measured — re-check `the_driver_footer_has_three_measured_width_forms` (9848-9861).

**Tab tests to update** (inventory in RESEARCH §Tab-Index Consumer Inventory; key ones):
```rust
#[test]
fn every_tab_index_round_trips_through_its_sub_view() {
    for index in 0..TAB_COUNT {
        let view = sub_view_from_index(index, true);
        assert_eq!(tab_index(&view), index, "index {index} mapped to {view:?}, which maps back to {}", tab_index(&view));
    }
    assert_eq!(sub_view_from_index(DRIVER_TAB_INDEX, true), DetailSubView::Driver);
    assert_eq!(tab_index(&DetailSubView::Driver), DRIVER_TAB_INDEX);
    assert_eq!(sub_view_from_index(TAB_COUNT, true), DetailSubView::PhaseList);   // -> RoadmapViz
}
```
(9885-9903) — add `assert_eq!(tab_index(&DetailSubView::Archive), tab_index(&DetailSubView::Browse));`. Also 9936-9940 (`== 10` → 8), 9958 (`[0, 5, 9]` → within 0..8), 9996-10010 (ten-tab → eight-tab), 10058-10070 and 10073-10090 (`PhaseList` → `RoadmapViz`), 10127-10156, 10171-10186, 10282-10328 (`("1:Phases","1:Ph")` → `("1:Roadmap","1:Rd")`), 14504-14511 (`roadmap_graph_tab_v_is_inert_on_other_tabs` uses `PhaseList` — switch to e.g. `Backlog`).

**Post-edit grep (Pitfall 5)** — expect zero hits except `ArchiveDepth::PhaseList`:
`\[1-0\|1:Phases\|5:Pipe\|8:Arch\|0:Docs\|DetailSubView::PhaseList\|DRIVER_TAB_INDEX, 10\|, 10)` over `src tests README.md docs`.

---

### `src/ui/screens/mod.rs` — `ProjectViewCache` (store) — plans 24-03 / 24-04

**Analog:** `roadmap_box_view` (936-939) and the Driver view-state block comment (1019-1024).
```rust
#[derive(Default)]
pub struct ProjectViewCache {
    ...
    pub pipeline_selected: usize,
    /// The Roadmap tab's view: `false` (the default) draws the dependency
    /// graph (`ui::roadmap_graph`), `true` draws the box list
    /// (`ui::roadmap_widget`). Toggled per project by `v` on that tab.
    pub roadmap_box_view: bool,
```
```rust
// View state lives here rather than beside the ring buffer on
// `AppContext` because `ProjectViewCache` is `#[derive(Default)]`: these
// five fields are additive with zero constructor churn, while a field on
// `AppContext` costs an edit at all four construction sites.
```
New fields (24-03): `roadmap_selected: Option<String>` (phase_key), `roadmap_band_cursor: Option<usize>`, `roadmap_folded: HashSet<String>` (needs an "initialised" flag or `Option<HashSet>` so shipped bands start folded), `roadmap_edge_cycle: usize`. `roadmap_selected` holds a reader-generated key, not third-party prose — `ProjectViewCache` is not in the census, but doc it as a key. 24-04 needs no new field if Docs sub-tab state = the stored `DetailSubView` (`Browse` vs `Archive`) in `detail_sub_view_per_project` (1247); add one only if "last Docs sub-tab" must survive a trip to another tab.

---

### `src/app.rs` — `DetailSubView` enum (model) — plan 24-04

**Analog:** lines 15-40:
```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub enum DetailSubView {
    #[default]
    PhaseList,
    RoadmapViz,
    ...
    /// The Driver tab: run list, run detail, live output (D-15, OBS-04).
    ///
    /// **Index 10, and fully reachable.** `TAB_COUNT` is 11, ...
    Driver,
}
```
→ remove `PhaseList`, move `#[default]` to `RoadmapViz`, doc "Index 8 … `TAB_COUNT` is 9"; add a doc on `Archive` saying it is the Docs › Milestones sub-view sharing Docs' index.

**Index test** (4248-4261) — rename `..._index_ten_...` → `..._index_eight_...`; `10`→`8`, `11`→`9`, `PhaseList`→`RoadmapViz`:
```rust
assert_eq!(tab_index(&DetailSubView::Driver), 10);
assert_eq!(sub_view_from_index(10, true), DetailSubView::Driver);
assert_eq!(sub_view_from_index(11, true), DetailSubView::PhaseList);
```

---

### `src/app.rs` — Archive regression tests (debug fix `3c0e38f`, `581aa7d`) — GUARD for plan 24-04

**Location:** 4725-5040. These drive the real `App` (keys, channel actions, ticks, `TestBackend` render). They open the archive via `DetailScreen::opened_on(alias, DetailSubView::Archive, ..)` — today tab index 7, the one the dashboard's digit `8` reaches — and assert rendered text `"Archive > v1.2"`, `"Phase 01: Core Flow"`, no `"Loading..."`, and `">   v1.2-ROADMAP.md"`.

**Entry helper** (4748-4757) — this is the call that breaks if `opened_on` keeps routing through `tab_index` (see 24-04 above):
```rust
/// Push a detail screen already on the Archive tab, the way the dashboard's
/// `8` does, so its milestone discovery is scheduled on the real channel.
fn open_archive_tab(app: &mut App, alias: &str) {
    let screen = crate::ui::screens::detail::DetailScreen::opened_on(
        alias.to_string(),
        DetailSubView::Archive,
        &mut app.ctx,
    );
    app.screen_stack.push(Box::new(screen));
}
```
Only the doc comment wording ("the way the dashboard's `8` does" → Docs › Milestones via `8` then `m`) may change. The drill-in (`drill_into_v1_2`, 4845-4860: `open_archive_tab` → pump discovery → `Down` → `Enter` → pump load) depends on Archive-depth `Down`/`Enter` arms (1713, 2514) which stay unchanged under Pattern 6.

**`App`-level TestBackend renderer** (4812-4843) — same buffer-to-string shape as `render_detail_to_text`, but renders `app.screen_stack.last()`:
```rust
fn render_top_screen(app: &App) -> String {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    let (width, height) = (120u16, 30u16);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("TestBackend terminal");
    terminal.draw(|frame| {
        app.screen_stack.last().expect("a screen on the stack").render(frame, frame.area(), &app.ctx)
    }).expect("draw the top screen");
    ...
}
```
**Channel-driven fixture** — `obs_app(root)` (2557-2580) registers `OBS_ALIAS = "proj"` with a live `event_tx`; `pump_archive_until` (4766-4795) applies only archive actions with a 2 s timeout; tests are `#[tokio::test] async`. A new "Enter on the collapsed shipped-milestones row lands on Docs › Milestones" test should copy this harness (push a detail screen on `RoadmapViz`, press `Enter` on the band row, `pump_archive_until(.., is_discovery_for(OBS_ALIAS))`, assert `render_top_screen` contains `Archive` breadcrumb / `Milestones` strip).

Run: `rtk proxy cargo test --lib app::tests -- --nocapture` and confirm these five `archive_*` / `the_same_milestone_version_*` / `an_in_place_reload_*` tests pass by name.

---

### `src/ui/screens/render_escape_guard.rs` (probe) — plans 24-03 / 24-04

**Hostile fixture** (859-948) — add the identity to the new fields exactly like the `milestones` addition (929-938):
```rust
// The Roadmap graph's header band and row-end label (quick
// 260923-md1). Covering phase 1, and active because `milestone`
// above names it, so both decorations draw the label.
milestones: vec![crate::state_reader::roadmap_md::RoadmapMilestone {
    label: crate::text::Untrusted::from_untrusted_source(identity.to_string()),
    first: crate::state_reader::phase_num::PhaseNum::parse("1"),
    last: crate::state_reader::phase_num::PhaseNum::parse("1"),
    scoped_phases: Vec::new(),
    in_progress: true,
}],
```
→ `phase_goals: [(phase_key("1"), Untrusted::from_untrusted_source(identity))]`, one `planned_phases` entry named `identity`, `milestone_name: Some(Untrusted::from_untrusted_source(identity))`. Probe is 200x60 (`PROBE_WIDTH/HEIGHT`, 2016-2017) → side-by-side layout; the identity must arrive WHOLE once (detail-pane name/goal), not only truncated (Pitfall 4).

**`ALL_SUB_VIEWS`** (1355-1361) — `[_; 11]` → `[_; 10]`, drop `PhaseList`, keep `Archive`:
```rust
const ALL_SUB_VIEWS: [crate::app::DetailSubView; 11] = {
    use crate::app::DetailSubView::*;
    [PhaseList, RoadmapViz, Backlog, GitHistory, Pipeline, Queue, Sessions, Archive, Defaults, Browse, Driver]
};
```
Note: `states_over_sub_views_with` (1597-1615) inserts the view into `detail_sub_view_per_project` directly — it does not go through `tab_index` — so Archive stays probed after it shares Docs' index.

**`DETAIL_TAB_ARRIVAL`** (1382-1519) — drop the `"PhaseList tab"` row (1383-1388); rewrite the `"RoadmapViz tab"` reason (1389-1396) to name list rows, band label, detail-pane name/goal/Needs, planned-phase names. Row shape:
```rust
(
    "RoadmapViz tab",
    true,
    "The default graph view draws each `RoadmapPhase` id, ... all through `ui::roadmap_graph`.",
),
```
**`sub_view_label`** (1558-1573) — exhaustive match, drop `PhaseList`. **`DETAIL_SUB_STATES`** (1651-1683) — keep `"RoadmapViz tab, box view"` and the two Archive states; optionally add `"RoadmapViz tab, folded shipped band"` with the arrange-closure shape:
```rust
("RoadmapViz tab, box view", |identity, ctx| {
    ctx.detail_sub_view_per_project
        .insert(identity.to_string(), crate::app::DetailSubView::RoadmapViz);
    let cache = ctx.view_cache.entry(identity.to_string()).or_default();
    cache.roadmap_box_view = true;
}),
```
Prose counts ("eleven tabs", "fifteen states") at 110-128, 838, 1012-1013, 1240-1255, 1590, 1617 — update in the same commit.

---

### `src/ui/screens/help.rs` (text) — plan 24-03

**Analog:** rows at 227-234:
```rust
lines.extend([
    row("e", "Enqueue next action (detail view)"),
    row("r", "Toggle roadmap visualization (detail view)"),   // STALE: `r` is bound only on Config
    row("v", "Roadmap tab: graph / box view (detail view)"),
    row("q / Esc", "Quit / Back"),
    ...
]);
```
Replace the stale `r` row with Roadmap rows (`j/k`, `g/G`, `h/l`, `[ / ]`, `Space`, `Enter`); keep the `v` row text EXACTLY (pinned by `the_roadmap_graph_toggle_key_is_documented`, 884-896). Test pattern for new rows — whole-row match, not substring:
```rust
let expected = row("v", "Roadmap tab: graph / box view (detail view)").spans[0].content.to_string();
assert!(text.lines().any(|line| line == expected), "the row {expected:?} is missing:\n{text}");
```
Check the popup-scroll assertion (878-882, `total > 19`) still holds.

---

### `README.md` / `docs/ARCHITECTURE.md` (docs) — plan 24-04

Current `README.md:47-49`:
```
- 10-tab detail view: Phases, Roadmap (ASCII DAG), Backlog, Git History,
  Pipeline, Queue, Sessions, Archive, Config, Docs (rendered `.planning/`
  browser rooted at the active phase, with quick jumps to `.planning/` and back)
```
→ wording from RESEARCH "Elsewhere" table; also `README.md:28, 59` and `docs/ARCHITECTURE.md:165`.

---

## Shared Patterns

### Third-party text escaping (all render sites)
**Source:** `src/ui/screens/detail.rs:78-80` (`shown`), `src/ui/roadmap_graph.rs:111-115` (`esc`), `Untrusted::shown()` (`src/text.rs:641`), `render_for_terminal` (`src/text.rs:487`).
**Apply to:** `roadmap_view.rs`, `roadmap_graph.rs`, `detail.rs` Roadmap render. Escape BEFORE measuring/truncating; store only escaped text in the model; raw text only for comparisons/map keys (`phase_key`). Enforced by `render_escape_guard::the_screen_renders_identity_escaped`.

### No new free-string fields (census)
**Source:** `src/driver/untrusted.rs:136+` (`THIRD_PARTY_STRINGS`), `tests/spawn_seam_guard.rs:723-745` (counts `String | Option<String> | Vec<String>`; `HashMap<String, T>` exempt).
**Apply to:** every new `ProjectState` / `RoadmapPhase` field — use `Untrusted`, `HashMap<String, Untrusted>`, `Vec<RoadmapPhase>`. A plan that edits `untrusted.rs` is a security-surface widening (Pitfall 3).

### Phase identity
**Source:** `crate::state_reader::phase_num::{phase_key, PhaseNum}` (used at `roadmap_graph.rs:139-160`, `roadmap_md.rs:525`).
**Apply to:** dep lookup, goal-map key, `roadmap_selected`, Enter handoff index resolution. Never `==` on raw ids.

### Done/current decision (D-B12)
**Source:** `ProjectState::phase_marker` (`state_reader/mod.rs:283-290`) → `PhaseMarker::decide` (373-395).
**Apply to:** ●/◉ glyphs and dimming; ○ vs ◌ = `Future` + "all in-graph deps Done" (external deps count as satisfied, A12). Never the ROADMAP checkbox alone.

### Scroll clamping
**Source:** `ViewportMetrics` + `clamp_scroll` (`detail.rs:211-225`), recorded via `Cell` in render (3911-3914).
**Apply to:** Roadmap list offset / box view; never re-spell `total - visible`.

### Width math
**Source:** `width_of` (`roadmap_graph.rs:117-120`), `truncate_subject` (`detail.rs:301-317`).
**Apply to:** every column budget in `roadmap_view.rs`; `chars().count()` of escaped text, never `len()` / byte slicing.

### Glyph constants
**Source:** `detail.rs:455-460` (`DRIVER_LIVE_MARKER = "\u{25C6}"`), `normal.rs:65-70` rule text.
**Apply to:** ●◉○◌▶↑↓·▾▸━║⏎ — `const X: &str = "\u{…}";` with a one-line doc.

### Tab index discipline
**Source:** `opened_on` doc (`detail.rs:698-724`), `tab_index`/`sub_view_from_index` (782-827).
**Apply to:** every handoff (Roadmap → Phases, Roadmap → Docs › Milestones). Use `tab_index(&view)` or the new `switch_to_sub_view`, never a literal.

### Rendered-text tests (TestBackend)
**Source:** `detail.rs:11630-11656` (`render_detail_to_text`, 120x30), `app.rs:4812-4843` (`render_top_screen`), `roadmap_graph.rs:1558-1567` (`buffer_rows` for bare widgets), `render_escape_guard.rs:2016-2050` (200x60 probe).
**Apply to:** all new render assertions; add an 80x24 variant for SC-3.

### Test runs
`rtk proxy cargo test --no-fail-fast` (baseline 49 suites, 2251 pass, 1 known local fail `envelope::policy::...git_version...`); `rtk proxy cargo clippy -- -D warnings` must stay clean. Never grep rtk-filtered output for `test result:`.

---

## No Analog Found

| File / part | Role | Data Flow | Reason |
|---|---|---|---|
| Lane assigner + transitive reduction in `src/ui/roadmap_graph.rs` | model | transform | The existing placement engine (`Placement`/`GapCell`, 294-760) is left-to-right with reference rows — the very mechanism being removed. Use RESEARCH Pattern 1 (Rust sketch of `transitive_reduction`) and Pattern 2 (lane rules + prototype output matching Mockups A/B/C). Only the input plumbing (`resolve_deps` → `break_cycles` → `longest_path_layers`, 776-781) is copied. |
| Docs "Files \| Milestones" sub-tab strip | component | render | No existing sub-tab strip inside a detail tab. Nearest shape is the 1-row breadcrumb header in `render_archive_tab` (4631-4639) / `render_browser_tab` (4817-4832); `Tabs` widget usage for the top bar is at `detail.rs:3450-3461`. |

---

## Metadata

**Analog search scope:** `src/state_reader/`, `src/ui/`, `src/ui/screens/`, `src/app.rs`, `src/executor/` (include_str precedent), `tests/fixtures/`, `tests/spawn_seam_guard.rs`, `src/driver/untrusted.rs`, `README.md`.
**Files scanned:** 17 (all confirmed git-tracked).
**Pattern extraction date:** 2026-09-23 (against `46d7e82`).
