use regex::Regex;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct RoadmapPhase {
    pub number: String,
    pub name: String,
    pub description: String,
    pub completed: bool,
    pub total_plans: u32,
    pub completed_plans: u32,
}

/// Aggregate phase/plan counts parsed from a ROADMAP.md `## Progress` table.
///
/// The `## Progress` table is the authoritative source of progress counts for a
/// GSD 1.8.0 roadmap. plan 6 (mod.rs) prefers this over STATE.md frontmatter
/// when present; when the section is absent `roadmap_progress` returns `None` and
/// the caller falls back to STATE.md.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RoadmapProgress {
    pub total_phases: u32,
    pub completed_phases: u32,
    pub total_plans: u32,
    pub completed_plans: u32,
}

/// Phase-ID token used inside the phase recognizers.
///
/// Accepts:
/// - bare numeric / decimal IDs (`4`, `0.3`, `14`, `26`)
/// - project-code / milestone-prefixed IDs (`M-2`, `AB-29`)
/// - a trailing letter (covers backlog sentinels like `999.x`)
///
/// The optional leading `[A-Za-z]{1,4}-` is the project-code/milestone prefix;
/// the numeric body is `[0-9][0-9.]*` with an optional trailing `[A-Za-z]`.
const PHASE_ID: &str = r"(?:[A-Za-z]{1,4}-)?[0-9][0-9.]*[A-Za-z]?";

/// Returns true when a phase number is a backlog sentinel that must be excluded
/// from the returned phase list (and every count).
///
/// Sentinels: `Phase 0` (pre-milestone) and `Phase 999` / `999.x` (backlog).
/// A leading alphabetic project-code prefix (`M-`, `AB-`) is stripped first so
/// only the numeric body is inspected. Ordinary decimals like `0.3` are kept.
fn is_sentinel_phase(number: &str) -> bool {
    let n = match number.split_once('-') {
        Some((prefix, rest)) if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_alphabetic()) => {
            rest
        }
        _ => number,
    };
    n == "0" || n == "999" || n.starts_with("999.")
}

/// Parse ROADMAP.md content and extract phase checklist items with per-phase plan counts.
///
/// Recognizes several heading shapes used by GSD 1.8.0 roadmaps:
/// - checklist form: `- [ ] **Phase N: Title** - desc`
/// - parenthetical cluster tags: `- [ ] **Phase 26 (Cluster B): Title** - desc`
/// - project-code / milestone-prefixed IDs: `Phase M-2`, `Phase AB-29`
/// - markdown headings: `### Phase 4: Visualization` (no checkbox → `completed: false`)
/// - `<details>` / `</details>` / `<summary>` wrapper lines are transparent, so
///   phases and plans nested inside a `<details>` block are still counted.
pub fn parse_roadmap_phases(content: &str) -> Vec<RoadmapPhase> {
    // Checklist form. Groups: 1=checkbox, 2=strike-open (`~~`), 3=id, 4=name,
    // 5=strike-close (`~~`), 6=description. The `(~~)?` groups let a retired
    // (strikethrough) phase still match so it can be filtered out explicitly.
    let checklist_re = Regex::new(&format!(
        r"- \[([ xX])\] (~~)?\*\*Phase ({id})(?:\s*\([^)]*\))?:\s*(.+?)\*\*(~~)?\s*[-\x{{2014}}]\s*(.*)",
        id = PHASE_ID
    ))
    .unwrap();
    // Markdown-heading form (no checkbox, no `**`). Groups: 1=id, 2=name.
    let heading_re = Regex::new(&format!(
        r"^\s*#{{2,4}}\s+Phase ({id})(?:\s*\([^)]*\))?:\s+(.+?)\s*$",
        id = PHASE_ID
    ))
    .unwrap();
    let plan_re = Regex::new(r"^\s*- \[([ xX])\] (?:\d+-\d+-)?PLAN\.md").unwrap();

    let is_header = |line: &str| checklist_re.is_match(line) || heading_re.is_match(line);

    let lines: Vec<&str> = content.lines().collect();
    let mut phases: Vec<RoadmapPhase> = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let parsed: Option<RoadmapPhase> = if let Some(caps) = checklist_re.captures(line) {
            let number = caps[3].to_string();
            // Strikethrough (`~~...~~`) marks a retired phase → exclude entirely.
            let retired = caps.get(2).is_some() || caps.get(5).is_some() || line.contains("~~");
            if retired || is_sentinel_phase(&number) {
                None
            } else {
                Some(RoadmapPhase {
                    completed: &caps[1] != " ",
                    number,
                    name: caps[4].trim().to_string(),
                    description: caps[6].trim().to_string(),
                    total_plans: 0,
                    completed_plans: 0,
                })
            }
        } else if let Some(caps) = heading_re.captures(line) {
            // `###`-style headings carry no checkbox and no inline description.
            let number = caps[1].to_string();
            if line.contains("~~") || is_sentinel_phase(&number) {
                None
            } else {
                Some(RoadmapPhase {
                    completed: false,
                    number,
                    name: caps[2].trim().to_string(),
                    description: String::new(),
                    total_plans: 0,
                    completed_plans: 0,
                })
            }
        } else {
            None
        };

        if let Some(mut phase) = parsed {
            // Scan subsequent lines for plan items. `<details>`/`</details>`/
            // `<summary>` lines don't match `plan_re` or the phase recognizers,
            // so they are transparent and never break the scan.
            let mut j = i + 1;
            while j < lines.len() {
                let l = lines[j];
                // Stop at the next phase header (either heading shape).
                if is_header(l) {
                    break;
                }
                if let Some(plan_caps) = plan_re.captures(l) {
                    phase.total_plans += 1;
                    if &plan_caps[1] != " " {
                        phase.completed_plans += 1;
                    }
                }
                j += 1;
            }

            phases.push(phase);
            i += 1;
        } else {
            i += 1;
        }
    }

    merge_duplicate_phases(phases)
}

/// Collapse repeated sightings of the same phase number into one entry.
///
/// A standard GSD 1.8.0 ROADMAP.md describes every phase twice: once in the
/// summary checklist near the top (`- [x] **Phase 4: Title** - desc`) and once
/// under `## Phase Details` (`### Phase 4: Title`, with the plan items beneath
/// it). Without this step the Phases pane lists each phase twice, and the two
/// copies disagree — the checklist copy reports `total_plans: 0` because the
/// next line is another header, while the detail copy carries the real counts.
///
/// Phases are keyed by `number` alone; that is already the identity key the
/// caller uses when building `phase_disk_statuses`. Merge rule per field:
/// - `completed` — logical OR (any checked copy marks the phase complete)
/// - `name`, `description` — first non-empty value wins (the heading form
///   carries no description, so the checklist text survives)
/// - `total_plans`, `completed_plans` — max (the copy that actually scanned the
///   plan list wins over the one that stopped at the next header)
///
/// First-seen order is preserved, and the pass is O(n) — no nested scan.
fn merge_duplicate_phases(phases: Vec<RoadmapPhase>) -> Vec<RoadmapPhase> {
    let mut merged: Vec<RoadmapPhase> = Vec::with_capacity(phases.len());
    let mut index: HashMap<String, usize> = HashMap::new();

    for phase in phases {
        match index.get(&phase.number) {
            Some(&at) => {
                let existing = &mut merged[at];
                existing.completed |= phase.completed;
                if existing.name.is_empty() {
                    existing.name = phase.name;
                }
                if existing.description.is_empty() {
                    existing.description = phase.description;
                }
                existing.total_plans = existing.total_plans.max(phase.total_plans);
                existing.completed_plans = existing.completed_plans.max(phase.completed_plans);
            }
            None => {
                index.insert(phase.number.clone(), merged.len());
                merged.push(phase);
            }
        }
    }

    merged
}

/// Returns true when a `## Progress` table Phase cell is a backlog sentinel
/// (`Phase 0` / `Phase 999` / `999.x`) that must not be counted. The cell may
/// carry a trailing label (e.g. `999. Backlog`), so only the leading token is
/// inspected.
fn is_progress_sentinel(phase_cell: &str) -> bool {
    let token = phase_cell
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches('.');
    is_sentinel_phase(token)
}

/// Split a markdown table row into trimmed cells, dropping the outer pipes.
fn split_table_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

/// Parse the authoritative `## Progress` table from ROADMAP.md content.
///
/// Columns are matched by NAME (case-insensitive), not position, so both the
/// flat 4-column layout (`Phase | Plans Complete | Status | Completed`) and the
/// milestone-grouped 5-column layout (with an extra `Milestone` column) — as
/// well as any column reordering — are handled. Returns `None` when the
/// `## Progress` section is absent or its table header cannot be interpreted
/// (missing the required `Phase` / `Plans Complete` columns).
///
/// Pure: takes `&str`, returns `Option<RoadmapProgress>`, mutates nothing.
///
/// plan 6 (mod.rs) prefers this over STATE.md frontmatter when present.
pub fn roadmap_progress(content: &str) -> Option<RoadmapProgress> {
    let lines: Vec<&str> = content.lines().collect();

    // Locate the `## Progress` section heading.
    let progress_heading = Regex::new(r"(?i)^##[ \t]+Progress\b").unwrap();
    // A following level-1 or level-2 heading closes the section scope.
    let heading_boundary = Regex::new(r"^#{1,2}[ \t]").unwrap();

    let start = lines.iter().position(|l| progress_heading.is_match(l))?;
    let mut end = lines.len();
    for (offset, l) in lines.iter().enumerate().skip(start + 1) {
        if heading_boundary.is_match(l) {
            end = offset;
            break;
        }
    }
    let scope = &lines[(start + 1)..end];

    // A markdown table header row is a `|`-delimited line immediately followed
    // by a delimiter row (`| --- | ... |`).
    let is_row = |l: &str| l.trim().starts_with('|');
    let is_delimiter = |l: &str| {
        let t = l.trim();
        t.starts_with('|')
            && t.contains('-')
            && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'))
    };

    let mut header_idx = None;
    for k in 0..scope.len() {
        if is_row(scope[k]) && k + 1 < scope.len() && is_delimiter(scope[k + 1]) {
            header_idx = Some(k);
            break;
        }
    }
    let header_idx = header_idx?;

    let headers = split_table_row(scope[header_idx]);
    let col = |name: &str| headers.iter().position(|h| h.eq_ignore_ascii_case(name));

    // Required columns; without these the header cannot be interpreted.
    let phase_col = col("Phase")?;
    let plans_col = col("Plans Complete")?;
    let status_col = col("Status");

    let plans_re = Regex::new(r"(\d+)\s*/\s*(\d+)").unwrap();

    let mut progress = RoadmapProgress::default();

    for l in scope.iter().skip(header_idx + 2) {
        if !is_row(l) {
            break; // table ended
        }
        let cells = split_table_row(l);
        if cells.len() < headers.len() {
            continue;
        }
        let phase_cell = cells.get(phase_col).map(String::as_str).unwrap_or("").trim();
        if phase_cell.is_empty() || is_progress_sentinel(phase_cell) {
            continue;
        }
        progress.total_phases += 1;

        if let Some(sc) = status_col {
            if let Some(status) = cells.get(sc) {
                let status = status.trim();
                if status.eq_ignore_ascii_case("complete") || status.eq_ignore_ascii_case("done") {
                    progress.completed_phases += 1;
                }
            }
        }

        if let Some(cell) = cells.get(plans_col) {
            if let Some(m) = plans_re.captures(cell.trim()) {
                progress.completed_plans += m[1].parse::<u32>().unwrap_or(0);
                progress.total_plans += m[2].parse::<u32>().unwrap_or(0);
            }
        }
    }

    Some(progress)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_roadmap_phases_real() {
        let content = r#"# Roadmap

- [ ] **Phase 1: Core Infrastructure** - Async TUI foundation
- [ ] **Phase 2: Dashboard and Navigation** - Main project list
- [x] **Phase 3: Live State** - File-watcher auto-refresh
- [ ] **Phase 4: Visualization** - ASCII roadmap
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 4);
        assert!(!phases[0].completed);
        assert_eq!(phases[0].number, "1");
        assert_eq!(phases[0].name, "Core Infrastructure");
        assert!(phases[2].completed);
        assert_eq!(phases[1].name, "Dashboard and Navigation");
        // No plan items listed, so counts should be 0
        assert_eq!(phases[0].total_plans, 0);
        assert_eq!(phases[0].completed_plans, 0);
    }

    #[test]
    fn test_parse_roadmap_empty() {
        let phases = parse_roadmap_phases("");
        assert!(phases.is_empty());
    }

    #[test]
    fn test_parse_roadmap_uppercase_x() {
        let content = "- [X] **Phase 1: Done** - Completed phase\n";
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert!(phases[0].completed);
    }

    #[test]
    fn test_parse_roadmap_with_plan_items() {
        let content = r#"# Roadmap

- [x] **Phase 1: Core Infrastructure** - Foundation

Plans:
- [x] 01-01-PLAN.md -- Project scaffold
- [x] 01-02-PLAN.md -- State reader
- [ ] 01-03-PLAN.md -- Event loop

- [ ] **Phase 2: Dashboard** - Main UI

Plans:
- [x] 02-01-PLAN.md -- Dashboard table
- [ ] 02-02-PLAN.md -- Help overlay
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 2);

        assert_eq!(phases[0].total_plans, 3);
        assert_eq!(phases[0].completed_plans, 2);

        assert_eq!(phases[1].total_plans, 2);
        assert_eq!(phases[1].completed_plans, 1);
    }

    #[test]
    fn test_parse_roadmap_standalone_plan_md() {
        let content = r#"# Roadmap

- [x] **Phase 1: Solo Plan** - Single standalone plan

Plans:
- [x] PLAN.md -- solo plan
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].total_plans, 1);
        assert_eq!(phases[0].completed_plans, 1);
    }

    #[test]
    fn test_parse_roadmap_mixed_standalone_and_numbered() {
        let content = r#"# Roadmap

- [ ] **Phase 3: Mixed** - Both formats

Plans:
- [x] 03-01-PLAN.md -- numbered plan
- [ ] PLAN.md -- standalone plan
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].total_plans, 2);
        assert_eq!(phases[0].completed_plans, 1);
    }

    #[test]
    fn test_parse_roadmap_heading_form() {
        // `### Phase N: Title` (no checkbox) parses as a phase with completed=false.
        let content = "### Phase 4: Visualization\n";
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].number, "4");
        assert_eq!(phases[0].name, "Visualization");
        assert!(!phases[0].completed);
        assert_eq!(phases[0].description, "");
    }

    #[test]
    fn test_parse_roadmap_heading_levels_and_plans() {
        // `##` and `####` heading levels both parse; plan items after a heading count.
        let content = r#"## Phase 2: Dashboard

Plans:
- [x] 02-01-PLAN.md -- table
- [ ] 02-02-PLAN.md -- overlay

#### Phase 3: Live State
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].number, "2");
        assert_eq!(phases[0].name, "Dashboard");
        assert_eq!(phases[0].total_plans, 2);
        assert_eq!(phases[0].completed_plans, 1);
        assert_eq!(phases[1].number, "3");
        assert_eq!(phases[1].name, "Live State");
    }

    #[test]
    fn test_parse_roadmap_parenthetical_tag() {
        // A parenthetical cluster tag is not part of the number or name.
        let content = "- [ ] **Phase 26 (Cluster B): Title** - desc\n";
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].number, "26");
        assert_eq!(phases[0].name, "Title");
        assert_eq!(phases[0].description, "desc");
    }

    #[test]
    fn test_parse_roadmap_prefixed_ids() {
        // Project-code / milestone-prefixed IDs parse with the prefix intact.
        let content = r#"- [ ] **Phase M-2: Milestone Two** - desc
- [x] **Phase AB-29: Big One** - done
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].number, "M-2");
        assert_eq!(phases[0].name, "Milestone Two");
        assert_eq!(phases[1].number, "AB-29");
        assert_eq!(phases[1].name, "Big One");
        assert!(phases[1].completed);
    }

    #[test]
    fn test_parse_roadmap_details_wrapped() {
        // Phase + plans nested in a <details> block are still parsed; the
        // <details>/<summary>/</details> wrapper lines are transparent.
        let content = r#"<details>
<summary>Completed phases</summary>

- [x] **Phase 1: Core** - Foundation

Plans:
- [x] 01-01-PLAN.md -- scaffold
- [x] 01-02-PLAN.md -- reader

</details>

- [ ] **Phase 2: Dashboard** - Main UI
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].number, "1");
        assert_eq!(phases[0].total_plans, 2);
        assert_eq!(phases[0].completed_plans, 2);
        assert_eq!(phases[1].number, "2");
    }

    #[test]
    fn test_parse_roadmap_dedupes_summary_and_details() {
        // A standard GSD 1.8.0 roadmap describes each phase twice: once in the
        // summary checklist near the top, once under `## Phase Details`. The two
        // copies must merge into a single entry per phase number.
        let content = r#"# Roadmap

- [x] **Phase 4: Visualization** - ASCII roadmap rendering
- [ ] **Phase 5: Queue** - Batch execution
- [ ] **Phase 6: Sessions** - tmux attach

## Phase Details

### Phase 4: Visualization

Plans:
- [x] 04-01-PLAN.md -- canvas shapes
- [x] 04-02-PLAN.md -- arrow routing

### Phase 5: Queue

Plans:
- [x] 05-01-PLAN.md -- queue model
- [ ] 05-02-PLAN.md -- executor trait

### Phase 6: Sessions
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 3);

        // First-seen order preserved.
        let numbers: Vec<&str> = phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(numbers, vec!["4", "5", "6"]);

        // The literal `## Phase Details` heading is not itself a phase.
        assert!(phases.iter().all(|p| p.name != "Details"));

        // Description survives from the checklist form (heading form has none).
        assert_eq!(phases[0].description, "ASCII roadmap rendering");

        // Checkbox survives the merge (heading form always reports false).
        assert!(phases[0].completed);
        assert!(!phases[1].completed);

        // Plan counts come from the detail section, where the plan items live.
        assert_eq!(phases[0].total_plans, 2);
        assert_eq!(phases[0].completed_plans, 2);
        assert_eq!(phases[1].total_plans, 2);
        assert_eq!(phases[1].completed_plans, 1);
        assert_eq!(phases[2].total_plans, 0);
    }

    #[test]
    fn test_parse_roadmap_decimal_id_preserved() {
        // Existing decimal IDs still parse.
        let content = "- [ ] **Phase 0.3: Spike** - explore\n";
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].number, "0.3");
        assert_eq!(phases[0].name, "Spike");
    }

    #[test]
    fn test_parse_roadmap_excludes_strikethrough() {
        // A strikethrough (retired) phase is absent from the returned Vec and
        // does not contribute to any count; real phases are unaffected.
        let content = r#"- [ ] **Phase 1: Real One** - live
- [ ] ~~**Phase 7: Old idea**~~ - abandoned
~~Phase 8: Bare strike~~
- [ ] **Phase 2: Real Two** - live
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].number, "1");
        assert_eq!(phases[1].number, "2");
        assert!(phases.iter().all(|p| p.number != "7" && p.number != "8"));
    }

    #[test]
    fn test_parse_roadmap_excludes_backlog_sentinels() {
        // Phase 0 and Phase 999 / 999.x sentinels are excluded; real phases stay.
        let content = r#"- [ ] **Phase 0: Backlog** - pre-milestone
- [ ] **Phase 1: Alpha** - real
- [ ] **Phase 999: Someday** - backlog
- [ ] **Phase 999.2: Later** - backlog
- [ ] **Phase 0.3: Spike** - real decimal
- [ ] **Phase 2: Beta** - real
"#;
        let phases = parse_roadmap_phases(content);
        let numbers: Vec<&str> = phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(numbers, vec!["1", "0.3", "2"]);
        assert!(!numbers.contains(&"0"));
        assert!(!numbers.contains(&"999"));
        assert!(!numbers.contains(&"999.2"));
    }

    #[test]
    fn test_parse_roadmap_three_real_phases_unaffected() {
        // A normal roadmap of 3 real phases is unchanged by the new filters.
        let content = r#"- [ ] **Phase 1: One** - a
- [x] **Phase 2: Two** - b
- [ ] **Phase M-2: Three** - c
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 3);
        assert_eq!(phases[0].number, "1");
        assert_eq!(phases[1].number, "2");
        assert_eq!(phases[2].number, "M-2");
    }

    #[test]
    fn test_parse_roadmap_mixed_plan_formats() {
        let content = r#"
- [ ] **Phase 3: Live State** - File watching

Plans:
- [ ] 03-01-PLAN.md -- File watcher
- [ ] 03-02-PLAN.md -- Detail view

**UI hint**: yes
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].total_plans, 2);
        assert_eq!(phases[0].completed_plans, 0);
    }

    #[test]
    fn test_roadmap_progress_flat_four_column() {
        let content = r#"# Roadmap

## Progress

| Phase | Plans Complete | Status | Completed |
| --- | --- | --- | --- |
| 1. Alpha | 2/2 | Complete | ✅ |
| 2. Beta | 1/2 | In Progress | |
"#;
        let p = roadmap_progress(content).expect("progress table parses");
        assert_eq!(p.total_phases, 2);
        assert_eq!(p.completed_phases, 1);
        assert_eq!(p.total_plans, 4);
        assert_eq!(p.completed_plans, 3);
    }

    #[test]
    fn test_roadmap_progress_milestone_grouped_five_column() {
        let content = r#"## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|---|---|---|---|---|
| 1. Alpha | v1.0 | 2/2 | Complete | ✅ |
| 2. Beta | v1.1 | 0/3 | Planned | |
"#;
        let p = roadmap_progress(content).expect("5-column table parses");
        assert_eq!(p.total_phases, 2);
        assert_eq!(p.completed_phases, 1);
        assert_eq!(p.total_plans, 5);
        assert_eq!(p.completed_plans, 2);
    }

    #[test]
    fn test_roadmap_progress_column_reordered() {
        // Columns matched by NAME, not position — a reordered header still parses.
        let content = r#"## Progress

| Status | Completed | Plans Complete | Phase |
| --- | --- | --- | --- |
| Complete | ✅ | 3/3 | 1. Alpha |
| Planned | | 0/2 | 2. Beta |
"#;
        let p = roadmap_progress(content).expect("reordered table parses");
        assert_eq!(p.total_phases, 2);
        assert_eq!(p.completed_phases, 1);
        assert_eq!(p.total_plans, 5);
        assert_eq!(p.completed_plans, 3);
    }

    #[test]
    fn test_roadmap_progress_excludes_backlog_row() {
        // A 999.x backlog row is not counted as a phase or plans.
        let content = r#"## Progress

| Phase | Plans Complete | Status | Completed |
| --- | --- | --- | --- |
| 1. Alpha | 2/2 | Complete | ✅ |
| 999. Backlog | 0/9 | Planned | |
"#;
        let p = roadmap_progress(content).expect("table parses");
        assert_eq!(p.total_phases, 1);
        assert_eq!(p.completed_phases, 1);
        assert_eq!(p.total_plans, 2);
        assert_eq!(p.completed_plans, 2);
    }

    #[test]
    fn test_roadmap_progress_absent_section_is_none() {
        let content = r#"# Roadmap

- [ ] **Phase 1: Alpha** - no progress table here
"#;
        assert_eq!(roadmap_progress(content), None);
    }

    #[test]
    fn test_roadmap_progress_uninterpretable_header_is_none() {
        // A `## Progress` section whose table lacks the required columns → None.
        let content = r#"## Progress

| Foo | Bar |
| --- | --- |
| a | b |
"#;
        assert_eq!(roadmap_progress(content), None);
    }
}
