use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub struct RoadmapPhase {
    pub number: String,
    pub name: String,
    pub description: String,
    pub completed: bool,
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
            Some(RoadmapPhase {
                completed: &caps[1] != " ",
                number: caps[3].to_string(),
                name: caps[4].trim().to_string(),
                description: caps[6].trim().to_string(),
                total_plans: 0,
                completed_plans: 0,
            })
        } else if let Some(caps) = heading_re.captures(line) {
            // `###`-style headings carry no checkbox and no inline description.
            Some(RoadmapPhase {
                completed: false,
                number: caps[1].to_string(),
                name: caps[2].trim().to_string(),
                description: String::new(),
                total_plans: 0,
                completed_plans: 0,
            })
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

    phases
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
    fn test_parse_roadmap_decimal_id_preserved() {
        // Existing decimal IDs still parse.
        let content = "- [ ] **Phase 0.3: Spike** - explore\n";
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].number, "0.3");
        assert_eq!(phases[0].name, "Spike");
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
}
