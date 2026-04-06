---
phase: quick-260405-oum
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/config_json.rs
  - src/app.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/detail.rs
autonomous: true
must_haves:
  truths:
    - "User can switch to the 9th Defaults tab via key '9' or arrow keys"
    - "User sees all config.json settings organized by category with human-readable labels"
    - "User can toggle booleans with Enter/Space"
    - "User can cycle enum values (model_profile, mode, granularity, branching_strategy, discuss_mode) with Enter"
    - "Changes are written back to the project's .planning/config.json on each edit"
  artifacts:
    - path: "src/state_reader/config_json.rs"
      provides: "Full GsdConfig struct with all config.json fields, Serialize support"
    - path: "src/ui/screens/detail.rs"
      provides: "render_defaults_tab(), handle_defaults_key(), tab wiring"
  key_links:
    - from: "detail.rs render_defaults_tab"
      to: "GsdConfig struct"
      via: "ProjectViewCache.defaults_config"
    - from: "detail.rs handle_defaults_key"
      to: ".planning/config.json on disk"
      via: "serde_json::to_string_pretty + std::fs::write"
---

<objective>
Add a 9th "Defaults" tab to the project detail view that reads, displays, and allows inline editing of all `.planning/config.json` settings for the selected project.

Purpose: Let users view and modify GSD project configuration without leaving the TUI or manually editing JSON.
Output: Fully functional Defaults tab with categorized display, boolean toggling, and enum cycling.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@src/state_reader/config_json.rs
@src/app.rs
@src/ui/screens/mod.rs
@src/ui/screens/detail.rs

<interfaces>
From src/app.rs:
```rust
#[derive(Debug, Clone, PartialEq, Default)]
pub enum DetailSubView {
    #[default]
    PhaseList, RoadmapViz, Backlog, GitHistory,
    Pipeline, Queue, Sessions, Archive,
    // ADD: Defaults
}
```

From src/ui/screens/mod.rs:
```rust
#[derive(Default)]
pub struct ProjectViewCache {
    pub backlog_items: Vec<BacklogItem>,
    pub backlog_selected: usize,
    // ... existing fields ...
    pub archive_file_name: Option<String>,
    // ADD: defaults_config, defaults_selected, defaults_editing
}
```

From src/ui/screens/detail.rs:
```rust
const TAB_TITLES: [&str; 8] = [ ... ];  // Expand to 9
fn tab_index(sub_view: &DetailSubView) -> usize  // Add Defaults => 8
fn sub_view_from_index(index: usize) -> DetailSubView  // Add 8 => Defaults
fn switch_to_tab(...)  // Add config.json loading for Defaults tab
fn build_footer(sub_view: &DetailSubView) -> Paragraph  // Add Defaults footer, update [1-8] to [1-9]
```
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Expand GsdConfig to full schema and add tab plumbing</name>
  <files>src/state_reader/config_json.rs, src/app.rs, src/ui/screens/mod.rs</files>
  <action>
1. **src/state_reader/config_json.rs** — Replace the minimal GsdConfig with the full schema. Add `Serialize` derive alongside `Deserialize`. Use `serde_json::Value` for the top-level to preserve unknown keys on round-trip, but also parse into typed sub-structs for display:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GsdConfig {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub granularity: String,
    #[serde(default)]
    pub model_profile: String,
    #[serde(default)]
    pub commit_docs: Option<bool>,
    #[serde(default)]
    pub parallelization: Option<bool>,
    #[serde(default)]
    pub search_gitignored: Option<bool>,
    #[serde(default)]
    pub brave_search: Option<bool>,
    #[serde(default)]
    pub firecrawl: Option<bool>,
    #[serde(default)]
    pub exa_search: Option<bool>,
    #[serde(default)]
    pub project_code: Option<String>,
    #[serde(default)]
    pub phase_naming: Option<String>,
    #[serde(default)]
    pub response_language: Option<String>,
    #[serde(default)]
    pub git: Option<GitConfig>,
    #[serde(default)]
    pub workflow: Option<WorkflowConfig>,
    #[serde(default)]
    pub hooks: Option<HooksConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GitConfig {
    #[serde(default)]
    pub branching_strategy: Option<String>,
    #[serde(default)]
    pub base_branch: Option<String>,
    #[serde(default)]
    pub phase_branch_template: Option<String>,
    #[serde(default)]
    pub milestone_branch_template: Option<String>,
    #[serde(default)]
    pub quick_branch_template: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct WorkflowConfig {
    #[serde(default)]
    pub research: Option<bool>,
    #[serde(default)]
    pub plan_check: Option<bool>,
    #[serde(default)]
    pub verifier: Option<bool>,
    #[serde(default)]
    pub nyquist_validation: Option<bool>,
    #[serde(default)]
    pub auto_advance: Option<bool>,
    #[serde(default)]
    pub node_repair: Option<bool>,
    #[serde(default)]
    pub node_repair_budget: Option<u32>,
    #[serde(default)]
    pub ui_phase: Option<bool>,
    #[serde(default)]
    pub ui_safety_gate: Option<bool>,
    #[serde(default)]
    pub text_mode: Option<bool>,
    #[serde(default)]
    pub research_before_questions: Option<bool>,
    #[serde(default)]
    pub discuss_mode: Option<String>,
    #[serde(default)]
    pub skip_discuss: Option<bool>,
    #[serde(rename = "_auto_chain_active", default)]
    pub auto_chain_active: Option<bool>,
    #[serde(default)]
    pub use_worktrees: Option<bool>,
    #[serde(default)]
    pub subagent_timeout: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct HooksConfig {
    #[serde(default)]
    pub context_warnings: Option<bool>,
}
```

Keep `parse_gsd_config()` as-is (it will work with the expanded struct). Add a helper:
```rust
pub fn serialize_gsd_config(config: &GsdConfig) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(config)
}
```

Update existing tests and add a test that round-trips the full config JSON from the detailed_context example.

2. **src/app.rs** — Add `Defaults` variant to `DetailSubView` enum after `Archive`.

3. **src/ui/screens/mod.rs** — Add three fields to `ProjectViewCache`:
```rust
pub defaults_config: Option<crate::state_reader::config_json::GsdConfig>,
pub defaults_selected: usize,
pub defaults_editing: Option<usize>,  // Some(row_index) when editing a string field inline
```
  </action>
  <verify>
    <automated>cd /home/blk/projects/rust/gsd-manager && cargo test --lib state_reader::config_json 2>&1 | tail -5</automated>
  </verify>
  <done>GsdConfig parses all config.json fields, serializes back to JSON, round-trip test passes. DetailSubView::Defaults exists. ProjectViewCache has defaults fields.</done>
</task>

<task type="auto">
  <name>Task 2: Wire Defaults tab rendering, key handling, and editing</name>
  <files>src/ui/screens/detail.rs</files>
  <action>
Make all the following changes in detail.rs:

1. **TAB_TITLES** — Change to 9 entries: `["1:Phases", "2:Roadmap", "3:Backlog", "4:Git", "5:Pipe", "6:Queue", "7:Sess", "8:Arch", "9:Cfg"]`. Note: shortened "Archive" to "Arch" and "Defaults" to "Cfg" to fit 9 tabs in 80 columns.

2. **tab_index()** — Add `DetailSubView::Defaults => 8`.

3. **sub_view_from_index()** — Add `8 => DetailSubView::Defaults`.

4. **Key binding** — Add `KeyCode::Char('9') => switch_to_tab(&self.alias, 8, ...)` after the '8' binding.

5. **switch_to_tab()** — Add block for `DetailSubView::Defaults`:
   - Read `project.path.join(".planning/config.json")` synchronously via `std::fs::read_to_string`
   - Parse with `parse_gsd_config()`, store in `cache.defaults_config`
   - Reset `cache.defaults_selected = 0` and `cache.defaults_editing = None`

6. **Render dispatch** — Add `DetailSubView::Defaults => self.render_defaults_tab(frame, content_area, ctx)`.

7. **build_footer()** — Change `"[1-8]"` to `"[1-9]"`. Add `DetailSubView::Defaults` match arm with hints: `[Enter]toggle/cycle  [r]eload`.

8. **Implement `render_defaults_tab()`** — New method on `DetailScreen`. Builds a list of `(category, key, display_value, value_type)` rows from the cached `GsdConfig`. Categories:
   - "General": mode, granularity, model_profile, commit_docs, parallelization, project_code, phase_naming, response_language
   - "Search": search_gitignored, brave_search, firecrawl, exa_search
   - "Git": branching_strategy, base_branch, phase_branch_template, milestone_branch_template, quick_branch_template
   - "Workflow": research, plan_check, verifier, nyquist_validation, auto_advance, node_repair, node_repair_budget, ui_phase, ui_safety_gate, text_mode, research_before_questions, discuss_mode, skip_discuss, auto_chain_active, use_worktrees, subagent_timeout
   - "Hooks": context_warnings

   Render as a scrollable list. Each row: category label (only on first item of group, DarkGray), key name (White), value (Green for true, Red for false, Yellow for strings/enums, DarkGray for null/unset). Highlight selected row with Cyan background. Show "No config loaded" if defaults_config is None.

   Define an enum or vec of `ConfigEntry` structs internally to map row index back to field for editing:
   ```rust
   enum ConfigValueKind { Bool, Enum(&'static [&'static str]), String, Integer, Null }
   struct ConfigEntry { category: &'static str, key: &'static str, value: String, kind: ConfigValueKind, show_category: bool }
   ```

   Build entries from the GsdConfig. For fields that are `None`, show "(unset)" in DarkGray.

9. **Handle Defaults key events** — In the existing `handle_input` match on `KeyCode::Enter | KeyCode::Char(' ')`, add a branch for `DetailSubView::Defaults`:
   - Build the same config entries vec
   - Get entry at `cache.defaults_selected`
   - If Bool: toggle the value in `cache.defaults_config`, write to disk
   - If Enum: cycle to next value in the allowed list
     - model_profile: ["quality", "balanced", "budget"]
     - mode: ["yolo", "normal"]  
     - granularity: ["coarse", "standard", "fine"]
     - branching_strategy: ["none", "phase", "milestone"]
     - discuss_mode: ["discuss", "skip", "auto"]
   - If Integer (node_repair_budget, subagent_timeout): increment by 1, wrap at reasonable max
   - After mutation, serialize with `serialize_gsd_config()` and write to project's `.planning/config.json` via `std::fs::write`. Set status message "Config saved".

   For j/k navigation within Defaults tab, add handling in the existing j/k section: increment/decrement `cache.defaults_selected`, clamped to entry count.

   Add 'r' key for Defaults tab to reload config from disk (re-read and re-parse).

10. **PageUp/PageDown** — In the existing PageUp/PageDown handling section, add `DetailSubView::Defaults` to adjust `defaults_selected` by PAGE_SCROLL_LINES, clamped.
  </action>
  <verify>
    <automated>cd /home/blk/projects/rust/gsd-manager && cargo build 2>&1 | tail -10</automated>
  </verify>
  <done>
    - Pressing '9' switches to Defaults tab showing all config settings by category
    - j/k navigates settings, Enter toggles booleans and cycles enums
    - Changes write back to .planning/config.json immediately
    - Footer shows [1-9]tabs and Defaults-specific hints
    - All 9 tabs render without panic, arrow key navigation works across all 9
  </done>
</task>

</tasks>

<verification>
- `cargo build` succeeds with no errors
- `cargo test` passes (including updated config_json tests)
- `cargo clippy` shows no new warnings
- Manual: run the TUI, navigate to a project with .planning/config.json, press 9, verify settings display, toggle a boolean, confirm file is updated on disk
</verification>

<success_criteria>
- 9th tab "Cfg" appears in the tab bar and is accessible via '9' key and arrow keys
- All config.json fields are displayed organized by category with human-readable formatting
- Boolean values toggle on Enter/Space and save to disk
- Enum values cycle through valid options on Enter and save to disk
- Tab works gracefully when config.json is missing (shows "No config loaded")
- No regression in existing 8 tabs
</success_criteria>

<output>
After completion, create `.planning/quick/260405-oum-add-defaults-tab-to-display-and-edit-pla/260405-oum-SUMMARY.md`
</output>
