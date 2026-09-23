# Quick 260923-md1: Roadmap dependency graph + milestone boundaries - Research

**Researched:** 2026-09-23
**Domain:** Rust / ratatui 0.30 TUI, ROADMAP.md parsing, escape censuses
**Confidence:** HIGH (all findings read from this worktree or the local cargo registry this session)

CONTEXT.md decisions are LOCKED; this doc only answers "where/how", it does not revisit them.
No new crates are needed, so the Package Legitimacy Audit is N/A.

## Summary / primary recommendations

1. **Milestone membership:** add `roadmap_md::roadmap_milestones(content) -> Vec<RoadmapMilestone>`. Parse **explicit `Phases A-B` ranges** from three line shapes (the `## Milestones` bullets, `<summary>` lines, and milestone headings). Add a heading-scope fallback for phases that no range covers. Store the result on a **new `ProjectState` field `milestones: Vec<roadmap_md::RoadmapMilestone>`**, filled at `src/state_reader/mod.rs:591-608` right beside `parse_roadmap_phases`. The text fields should be `crate::text::Untrusted`, not `String` (see Pitfall P1).
2. **Toggle:** put `pub roadmap_box_view: bool` on `ProjectViewCache` (`src/ui/screens/mod.rs:906-907`, `#[derive(Default)]`, so the default is false and the graph shows). Do **not** put it on `DetailScreen`, because the escape probe can only arrange state through `AppContext` (P4). The `v` key is unbound on the Detail screen, and nowhere under `src/` matches `Char('v')`.
3. **Rendering:** turn `GraphLayout` rows into ratatui `Line`/`Span`s and draw them with `Paragraph::new(..).scroll((v, h))`. Do not use raw `Buffer::set_string` math. Paragraph clips and handles horizontal offset. `set_string` panics on an out-of-area `y` and bleeds past the widget's `Rect` (P5).
4. **Escaping:** send every phase id, dependency id and milestone label through `crate::text::render_for_terminal(..)` or `Untrusted::shown()`. **Never call `display_identity(` directly in `src/ui/`**, because that trips the `MEASURED_REACH` pin (P3). The behavioural probe covers the new sink automatically once the graph is the default (P4).

## 1. Milestone membership

### Formats that actually occur
Sources: the GSD templates, plus a survey of 14 real `.planning/ROADMAP.md` files under `~/projects`.

| Shape | Example (verbatim) | Source |
|---|---|---|
| `## Milestones` bullet with range | `- 🚧 **v2.0 Autonomous Orchestration** - Phases 14-23 (in progress)` | this repo `.planning/ROADMAP.md:12` |
| same, em-dash separator | `- ✅ **v1.0 MVP** — Phases 1-4 (shipped YYYY-MM-DD)` | `~/.claude/gsd-core/workflows/complete-milestone.md:489` |
| same, `--` separator / en-dash range | `- ✅ **v1.0 MVP** -- Phases 1-3 (...)` (daily-vow); `— Phases 1–7` (hitchmatch) | survey |
| bullet with NO range | `- ✅ **v1.3 Configuration & Pipeline Visibility** - 10 quick tasks (...)` | this repo `ROADMAP.md:8` |
| `<summary>` with range | `<summary>v1.0 MVP (Phases 01-04) - SHIPPED 2026-03-26</summary>` | this repo `ROADMAP.md:24` |
| milestone heading with range | `### v2.0 Autonomous Orchestration (Phases 14-23)` / `### v2.0 Customer Intelligence (Phases 14-16) — IN PROGRESS` / `### ✅ v1.2 Folder Organization Pipeline (Phases 4-8, shipped …)` | this repo `:86`, shopify, aiFlowAgent |
| milestone heading, NO range (phases nested beneath) | `### 🚧 v1.1 [Name] (In Progress)` then `#### Phase 5:` | template `~/.claude/gsd-core/templates/roadmap.md:167-171` |
| roadmapper heading form | `## vX.Y — [Name]` | `~/.claude/agents/gsd-roadmapper.md:342` |
| "Milestone N" wording | `- 🚧 **Milestone 1: Reassessment and decision records** - Phases 1-7 (in progress)`; `### Milestone 3 "Supervised live booking and unattended runs"` | ttbook |

Consequences:
- In this repo, `## Phase Details` (`ROADMAP.md:105`) is a separate `##` section, so the `### Phase 14:` detail headings are **not** nested under their milestone heading. Heading nesting alone gives the wrong answer here. **Ranges must be the primary source.**
- The user's sketch project (ttbook) writes build phases 8-18 as `#### Build phase 8 (Milestone 2): …`. `heading_re` (`roadmap_md.rs:165-169`, needs `#{2,4}\s+Phase <id>:`) does **not** parse those, so they will not appear in the graph at all. That is an existing parser limit and out of scope; see Open Questions.
- Milestone-prefixed ids (`Phase 1-01`, `gsd-roadmapper.md:360-366`) also fail today's `PHASE_ID` (`roadmap_md.rs:48`, prefix is `[A-Za-z]{1,4}-`). Out of scope.

### Reusable code (`src/state_reader/roadmap_md.rs`, read this session)
- `active_milestone` (`:447-478`) already scans the `## Milestones` section (heading regex `(?i)^##[ \t]+Milestones\b`, boundary `^#{1,2}[ \t]`, bold `\*\*(.+?)\*\*`, in-progress = `'\u{1F6A7}'` or `"(in progress)"`). Copy its section and bold logic, and mark `in_progress` the same way.
- `parse_roadmap_phases` (`:155-258`) + `merge_duplicate_phases` (`:286-319`) key phases with `phase_num::phase_key`. Membership lookups must go through `phase_num` too.
- `roadmap_progress` (`:355-436`) reads the Progress table by column NAME but **ignores** the `Milestone` column (only `Phase`, `Plans Complete`, `Status` are read, `:396-398`). Its `split_table_row` (`:335-341`) and header or delimiter detection can be reused. The Progress `Milestone` cell (`| 1. Foundation | v1.0 | …`, template `:190-194`) is an optional third source; I would skip it for this quick task.
- `extract_phase_id` (`:67-91`) normalises `Phase 19` / `**19**` / `#19`, so reuse it for range endpoints.

### `ProjectState` today (`src/state_reader/mod.rs:43-58`, read)
- `pub milestone: String` (`:56`) = STATE.md `milestone:` (for example `v1.7.2`, `v2.0`). If that is empty it falls back to `roadmap_md::active_milestone(&content)` (`:593-601`). STATE.md's `milestone_name` is parsed into the frontmatter (`state_md.rs:27,641`) but **not** copied onto `ProjectState`.
- There is **no** raw roadmap content and **no** milestone list on `ProjectState`. ROADMAP.md is read once at `mod.rs:590-591`, and that is the plug point.
- `ProjectState` derives `Debug, Clone, Default, PartialEq` (`:42`). Every `ProjectState {..}` literal in `src/` and `tests/` uses struct-update `..` (I checked all 48 sites with a scan script), so adding a field breaks none of them.

### Recommended parse strategy and signature [INFERRED design]
```rust
// src/state_reader/roadmap_md.rs
#[derive(Debug, Clone, PartialEq)]
pub struct RoadmapMilestone {
    /// Full label as written, e.g. "v2.0 Autonomous Orchestration". Third-party.
    pub label: crate::text::Untrusted,
    /// Inclusive range, when the roadmap declares one. Numeric only.
    pub first: Option<PhaseNum>,
    pub last: Option<PhaseNum>,
    /// Phase keys (phase_key) assigned by heading scope when no range covers them.
    pub scoped_phases: Vec<String>,
    /// 🚧 / "(in progress)" / "IN PROGRESS" marker seen on any of its lines.
    pub in_progress: bool,
}
pub fn roadmap_milestones(content: &str) -> Vec<RoadmapMilestone>;
impl RoadmapMilestone { pub fn contains(&self, phase_id: &str) -> bool; }
/// First milestone (roadmap order) containing the id; None when none do.
pub fn milestone_of<'a>(ms: &'a [RoadmapMilestone], phase_id: &str) -> Option<&'a RoadmapMilestone>;
```
Algorithm:
1. **Range pass** over every line. Match a range regex such as `(?i)\bphases?\s+(\S+?)\s*[-\u{2013}\u{2014}]\s*([0-9][0-9.]*)` on bullets inside `## Milestones`, on `<summary>` lines, and on `#{2,4}` headings that are not `Phase <id>:` headings.
   - Label = the bold text for bullets. For summary and heading lines, it is the text before ` (`, with emoji, `<summary>` tags and leading `#`s trimmed.
   - Parse the endpoints with `extract_phase_id` then `PhaseNum::parse`.
   - Dedupe by label, keeping the first seen and OR-ing `in_progress`. The `## Milestones` bullet and its `<summary>` twin carry the same label.
   - A bullet with no range (`10 quick tasks`) still yields a milestone entry with `first/last = None`, so the header band can list it. Or drop it; that is the planner's call.
2. **Heading-scope fallback**, only for phases that no range covers. A `#{2,4}` heading whose text matches `\bv\d+(\.\d+)*\b` or `\bMilestone\s+\d` opens a scope. It closes at the next heading of the same or higher level. Phase header lines inside the scope (the checklist or `heading_re` shapes) push `phase_key(id)` into `scoped_phases`. Require the number, because mailbot has `### Milestone mapping (decision D10)`.
3. Never fail. No match means an empty `Vec`, and the graph then drops its milestone decorations, as CONTEXT locks.

Membership (`contains`) with decimals. `PhaseNum` ordering is segment-wise (`phase_num.rs:13-15`, tests `:119-127`: `7 < 7.1 < 7.2 < 8`), so a plain `first <= p <= last` **excludes** an inserted `7.1` from `Phases 1-7`. ttbook shows this is real: Milestone 1 = Phases 1-7, yet it contains `Phase 7.1 … (INSERTED)`. Rule: `first <= p && (p <= last || p.major() == last.major())`. Non-numeric ids (`M-2`) are only matched through `scoped_phases` / `same_phase`.

Active milestone for the header band. Pick the entry where `state.milestone` equals the label, **or** equals the label's first whitespace token (STATE writes `v2.0`, the roadmap writes `v2.0 Autonomous Orchestration`). Otherwise pick the first `in_progress` entry. This repo's STATE says `v1.7.2`, which no roadmap milestone names, so the fallback is needed.

Short id and name split for the row-end label `(M3 → M4: support chat)`. Short = first token (`v2.0`), or `M<n>` when the label starts with `Milestone <n>`. Name = the rest, with a leading `:`/`—`/`-` and surrounding quotes trimmed. Do this at render time on the **escaped** text.

## 2. Roadmap tab integration (`src/ui/screens/detail.rs`)

- **Render:** `fn render_roadmap` at `:3759`. It is dispatched from `:3430` (`render`) and `:5009` (`render_main_only`, which draws the backdrop behind the Enqueue and Driver overlays), so one edit covers both. The widget call is at `:3842-3865`.
  - The scroll estimate there is hard-coded for boxes: `phase_block_h = 5`, `total_content = 3 + (n-1)*5` (`:3844-3849`).
  - It writes `self.generic_viewport` (`:3852-3855`), then `clamped_offset = self.scroll_offset.min(max_scroll)`.
  - Branch here on `ctx.view_cache.get(&self.alias).is_some_and(|c| c.roadmap_box_view)`. The box branch stays unchanged. The graph branch sets `ViewportMetrics { total_lines: graph_rows as u16 (saturating), visible_height: graph_area.height }`.
- **Scroll:** the `_ =>` arms (`:1771-1780` j/Down, and the k/Up, PageDown and PageUp equivalents) clamp through `clamp_scroll(offset, total, visible) = offset.min(total.saturating_sub(visible))` (`:198-200`). Nothing new is needed; just record correct metrics. On toggle, reset `self.scroll_offset = 0`.
- **Keys bound in `DetailScreen::handle_key` (`:1463-3375`), top-level arms:** Esc/q, j/Down, k/Up, PageDown, PageUp, `1`-`9`/`0`, `D` (experimental), Left, Right, Enter/Space, Tab. Also the tab-guarded keys: `g` (Browse), `p` (Browse, Git), `/` `x` `r` `d` (Defaults), `n` (Sessions), `a` `d` `x` `J` `K` (Queue), `f` `G` `i` `s` `x` `o` (Driver), and globally `e` and `?`. **`v` is free.** Global keys handled in `app.rs:2141-2152` are only Ctrl+C.
  - Recommended arm, placed just before `KeyCode::Char('?')` at `:3369`: `KeyCode::Char('v') if current_view == DetailSubView::RoadmapViz => { toggle view_cache flag; self.scroll_offset = 0; ctx.needs_redraw = true; ScreenAction::None }`.
- **Footer:** `footer_spans` `:6077-6168`. RoadmapViz currently falls into `_ =>` and gets `[e]nqueue`. Add a `DetailSubView::RoadmapViz` arm that pushes `[v]iew` plus the existing `[e]nqueue`. Tests pin exact footers only for Backlog and Defaults (`:9695-9704`), and the prefix for every tab (`:9709-9770`), so the new arm passes them.
- **Parallel-task collision map** (the worktree diffed against the main checkout's uncommitted files; the parallel task touches the following):
  - detail.rs `:138`, `:2378-2483` (Enter/Sessions), `:2804`, `:2839` (Tab), `:4462-4530` (sessions render), tests `:11453+`
  - help.rs `:84` and tests `:530`
  - render_escape_guard.rs `:1045-1048` (probe_ctx) and `:1411` (Sessions arrival row)
  - normal.rs, several places

  Our edit points (`:3369`, `:3842-3865`, `:6093-6160`, and `ProjectViewCache` in `screens/mod.rs`) do not overlap. Add new detail.rs tests at the end of the tests module, not near `:11453`.

## 3. Help screen (`src/ui/screens/help.rs`)

- `help_lines(experimental)` `:200-320`. The detail-view keys are the rows at `:227-233`: `row("e", "Enqueue next action (detail view)")` and `row("r", "Toggle roadmap visualization (detail view)")`. **That `r` row is stale:** `r` is bound only on the Defaults tab (`detail.rs:2785`). A test pins it (`:847`), so leave it alone.
- Minimal edit: insert `row("v", "Roadmap tab: graph / box view (detail view)"),` after `:229`. That is about 145 lines away from the parallel edit at `:84`. The description must not be byte-identical to any other row (`:207-210`).
- Tests that constrain help: whole-row lookup (`:482-532`), no double blanks and no trailing blank (`:804-829`), flag-off shorter than flag-on (`:834-858`), body > 19 rows (`:861-872`). None counts exact lines, so one added row is safe. Add a whole-row assertion for `v` in a **new** test at the file end, not inside `:482-532`, because the parallel task edits around `:530`.

## 4. Escaping contract and censuses

- **API** (`src/text.rs`, read):
  - `render_for_terminal(&str) -> Rendered` (`:487-489`, = `display_identity(&strip_terminal_controls(v))`).
  - `Rendered` implements `Display` (via `f.pad`, so `{:<w$}` works), `AsRef<str>`, `From<Rendered> for String` and `for Cow<'static, str>` (`:430-458`).
  - `Untrusted::from_untrusted_source(String)`, `.shown() -> Rendered`, `.as_raw_for_logic_only() -> &str` (`:568-644`). `Untrusted` derives `Clone, PartialEq, Eq, Hash, PartialOrd, Ord` and has a hand-written `Debug`, but **no `Default`**.
  - The precedent for `Untrusted` fields in a state reader is `BacklogItem` (`state_reader/backlog.rs:24-27`).
  - `roadmap_widget.rs:162-171` shows the pattern: escape **before** measuring or truncating, and truncate by `char`.
- **Census A: `src/ui/mod.rs` tests** (`:14-560`). A source walk flags any executable line under `src/ui/` that contains the `display_identity(` call without the `sanitize_render_line` composition. `MEASURED_REACH` (`:341-345`) pins **exact** per-file needle counts (`driver.rs` 2, `driver_confirm.rs` 4, `render_escape_guard.rs` 2) as a Vec equality in both directions. A new `src/ui/roadmap_graph.rs` must therefore contain **zero** `display_identity(` occurrences. `UI_SOURCE_FLOOR = 16` is a `>=` floor (`:535`), so a 17th file is fine. Also add `pub mod roadmap_graph;` next to `pub mod roadmap_widget;` (`src/ui/mod.rs:1`).
- **Census B: `render_escape_guard.rs`** is **behavioural**. `the_screen_renders_identity_escaped` (`:2693-2891`) renders every `DetailSubView` at 200x60 (`:1984-1985`) with `hostile_project_state` (`:859-938`). Wherever the clean identity arrives, the hostile render must contain the **full** `display_identity(hostile)` string and must not contain the raw hostile string (`:2816-2859`). Identity = `"demo\u{e0041}" + "r\u{00ad}un"` (`test_support.rs:124-132`, pairs 4 and 5).
  - The fixture sets `phases[0] = {number "1", name identity, depends_on [identity]}` and `milestone: identity`. So once the graph is the default, the new sink is probed automatically. Specifically: the current-phase line `▶ P1: <name>` and the `external deps` note (dep = identity, which is not in the phase list) both render there, and **neither may truncate the escaped identity at 200 columns.**
  - Required fixture edits:
    - (a) Populate the new `milestones` field in `hostile_project_state` with `label: Untrusted::from_untrusted_source(identity)` covering phase 1, so the header band and row-end label are exercised.
    - (b) Add a `DETAIL_SUB_STATES` entry `("RoadmapViz tab, box view", |id, ctx| ctx.view_cache.entry(id.to_string()).or_default().roadmap_box_view = true)` (`:1628+`) **and** a matching `DETAIL_TAB_ARRIVAL` row with `true` (`:1368-1496`). The arrival set equality runs in both directions (`:1365-1367`). Without this the box view drops out of probe coverage.
    - (c) Reword the `"RoadmapViz tab"` reason (`:1375-1379`) and the `DetailScreen` adjudication reason (`detail.rs:1432-1433`) to mention dependency ids and milestone labels. Reasons must not contain the words `escaped` or `safe` (`render_escape_guard.rs:2459-2478`).
  - **Census C: `tests/spawn_seam_guard.rs:712-1060` + `src/driver/untrusted.rs:136-203`.** Every `String` / `Option<String>` / `Vec<String>` field on `ProjectState` and `RoadmapPhase` must be listed in `THIRD_PARTY_STRINGS`, in both directions. The test `the_enumerated_prose_set_is_exactly_the_six_free_text_fields` (`untrusted.rs:438-460`) pins the prose set at six. A field typed `Vec<RoadmapMilestone>` whose text is `Untrusted` is not a free-string field, so neither test changes. The precedent is `queued_actions: Vec<QueuedAction>`. **Do not** add a `String` field to `ProjectState` or `RoadmapPhase` (P1).

## 5. `phase_num` helpers (`src/state_reader/phase_num.rs`, read)
- `PhaseNum::parse(&str) -> Option<PhaseNum>`: trims whitespace, accepts `7`, `07`, `7.1`, `07.1`; returns `None` for `""`, `M-2`, `AB-29`, `4a`, `7.`, `.1`, `7..1` (`:31-44`, `:130-137`).
- It derives `Ord`/`Hash` and orders segment-wise (`:24`). `.major()`, `.padded()`, `Display` gives the unpadded form, and `PartialEq<u32>` matches single-segment values only.
- `phase_key(id)` returns the canonical numeric spelling, or the raw trimmed text for non-numeric ids (`:94-99`). `same_phase(a,b)` = `phase_key(a) == phase_key(b)` (`:102-104`). Use `phase_key` as the `HashMap` key for the node index; this is O(1), where `same_phase` is an O(n) scan.
- No range helper exists. Implement `RoadmapMilestone::contains` as in §1.

## 6. Existing tests that must keep passing
- `roadmap_widget.rs:306-360`: two tests build `RoadmapWidget { phases, current_phase_num, disk_statuses, scroll_offset }` directly and read glyphs.
- `src/ui/mod.rs:810-922`: `the_roadmap_widget_renders_a_clean_phase_name_unchanged_and_a_control_one_differently` builds `RoadmapWidget` at 60x8.
- **Keep the `RoadmapWidget` struct fields and behaviour byte-identical** and all three stay green. Put the graph in a new widget or module; do not change `RoadmapWidget`.
- `RoadmapPhase {..}` literals appear in 11 files (23 sites, with no `..Default`). **Adding a field to `RoadmapPhase` would touch all of them, including `detail.rs`, which the parallel task is editing.** That is another reason milestone data goes on `ProjectState`.
- The `detail.rs:11377-11413` generic-scroll tests drive the `_ =>` arm through RoadmapViz using preset metrics. They are unaffected.

## 7. Pitfalls

- **P1: Census widening.** A `String` field on `ProjectState` or `RoadmapPhase` forces a `THIRD_PARTY_STRINGS` entry. If that entry is prose, it also breaks the pinned six-field prose test, which is a security-contract change. Use `Untrusted` inside a new struct instead. [INFERRED decision; record it in the SUMMARY]
- **P2: Decimal range membership.** See §1. `Phases 1-7` must include `7.1`.
- **P3: `display_identity(` in `src/ui/`** turns the `MEASURED_REACH` pin red. Use `render_for_terminal` or `Untrusted::shown()` only.
- **P4: Toggle on `DetailScreen`** cannot be reached by the probe's `SubStateArrange = fn(&str, &mut AppContext)` (`render_escape_guard.rs:1613`). If you put it there, the box view silently loses escape coverage.
- **P5: `Buffer::set_string`** (ratatui-core 0.1.2 `buffer/buffer.rs:324-370`, read):
  - It clips only at the **buffer's** right edge (`self.area.right()`, the whole frame when rendered through `Frame`), not at the widget `Rect`, so it bleeds into neighbouring panes.
  - A `y` outside the buffer, or an `x < area.x`, **panics** (`index_of` `:249-256`).
  - It silently drops control graphemes and zero-width graphemes.
  - Prefer `Paragraph::scroll((y, x))` (ratatui-widgets 0.3.2 `paragraph.rs:221-233`: the order is **(y, x)**, and the horizontal offset works when wrap is off). If you do write cells directly, use `set_stringn(x, y, s, area.right().saturating_sub(x) as usize, style)` and check `y < area.bottom()` first.
- **P6: Glyph widths.** I measured these this session with unicode-width 0.2.2; every one is width **1**: `─ │ ┬ └ ┘ ├ ┤ ┴ ┼ ┐ ┌ ► ◄ ▶ ◆ ◇ → ▼ …`. `Cargo.lock` resolves unicode-width 0.2.0 for ratatui-core; box-drawing widths are unchanged between those versions [ASSUMED for 0.2.0 specifically]. Measuring the **escaped** text by `chars().count()` is correct for ASCII ids and matches the existing widget's deliberate IN-02/IN-03 deferral (`roadmap_widget.rs:146-152`).
- **P7: u16 overflow.** Layout widths are `usize`. Convert with `u16::try_from(w).unwrap_or(u16::MAX)` or saturating ops before `Paragraph::scroll` / `ViewportMetrics`. `total_lines` is `u16`. Never use `as u16` on a sum that can exceed 65535.
- **P8: Tiny areas.** Guard `area.width == 0 || area.height == 0` and return early. Split the area into header band / graph / notes with `Layout`, which handles the zero heights. Test 1x1, 5x3, 10x3 and 20x5 per CONTEXT.
- **P9: MSRV and edition.** `edition = "2021"`, `rust-version = "1.88"` (`Cargo.toml:4,15`). **No let-chains** (`if let … && …` is edition 2024 only). Avoid APIs newer than 1.88.
- **P10: clippy `-D warnings`.** There is no custom lint config (no `clippy.toml`, no `[lints]`), so it is default clippy on rustc 1.98.1. Watch `needless_range_loop` (index loops over layer vectors), `too_many_arguments` (>7), `type_complexity` (nested `Vec<Vec<(..)>>`, so name the types), `new_without_default`, `collapsible_if`/`collapsible_else_if`, and `manual_saturating_arithmetic`.
- **P11: Cycle and recursion.** Use Kahn's algorithm or an iterative DFS, per CONTEXT. The ROADMAP is third-party, so the phase count is unbounded; never recurse on depth.
- **P12: Test runner.** Use `rtk proxy cargo test --no-fail-fast`. Plain `cargo test` fail-fast can hide the envelope suites, and a piped grep can truncate output even under `rtk proxy`. The known local failure is the git-version constant in `src/envelope/policy.rs`.

## Security domain (V5 output encoding only)

| Threat | Mitigation |
|---|---|
| Hostile ROADMAP text (ESC, bidi U+202E, tag chars) in a phase name, dependency id or milestone label reaching a cell | `render_for_terminal` / `Untrusted::shown()` at every sink; the behavioural probe (Census B) plus new unit tests with ESC and U+202E in the name and the milestone label |
| Panic from slicing third-party text | Escape first, then truncate by `char`, never `&s[..n]` (`roadmap_widget.rs:134-138`) |
| DoS through a cyclic or huge dependency graph | Iterative cycle detection; saturating u16 conversions |

## Assumptions log
| # | Claim | Risk if wrong |
|---|---|---|
| A1 | Box-drawing widths in unicode-width 0.2.0 (locked) equal the ones measured in 0.2.2 | Column misalignment only; the tests would show it |
| A2 | Heading-scope fallback is worth the complexity for a quick task; the planner may cut it to ranges only (which covers 12 of 14 surveyed roadmaps plus the template) | Phases in no-range heading-only roadmaps get no milestone decoration, which is an accepted degraded mode |

## Open questions
1. **ttbook `#### Build phase N (Milestone M)` entries** (the user's actual sketch topology) are not parsed as phases by `parse_roadmap_phases`, so the new graph will not show them. Extending the phase parser is out of scope per CONTEXT. Flag this in the SUMMARY so the user knows why their sketch project shows only phases 1-7.1.
2. The stale help row `r  Toggle roadmap visualization (detail view)` (`help.rs:229`, pinned at `:847`). Leave it, and note it as follow-up debt.
