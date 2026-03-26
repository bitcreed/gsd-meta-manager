use regex::Regex;

#[derive(Debug, Clone)]
pub struct RoadmapPhase {
    pub number: String,
    pub name: String,
    pub description: String,
    pub completed: bool,
    pub total_plans: u32,
    pub completed_plans: u32,
}

/// Parse ROADMAP.md content and extract phase checklist items with per-phase plan counts.
pub fn parse_roadmap_phases(content: &str) -> Vec<RoadmapPhase> {
    let phase_re = Regex::new(
        r"- \[([ xX])\] \*\*Phase ([0-9.]+): (.+?)\*\*\s*[-\x{2014}]\s*(.*)"
    ).unwrap();
    let plan_re = Regex::new(
        r"^\s*- \[([ xX])\] (?:\d+-\d+-)?PLAN\.md"
    ).unwrap();

    let lines: Vec<&str> = content.lines().collect();
    let mut phases: Vec<RoadmapPhase> = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        if let Some(caps) = phase_re.captures(lines[i]) {
            let mut phase = RoadmapPhase {
                completed: &caps[1] != " ",
                number: caps[2].to_string(),
                name: caps[3].to_string(),
                description: caps[4].trim().to_string(),
                total_plans: 0,
                completed_plans: 0,
            };

            // Scan subsequent lines for plan items
            let mut j = i + 1;
            while j < lines.len() {
                let line = lines[j];
                // Stop at next phase header
                if phase_re.is_match(line) {
                    break;
                }
                // Match plan checklist items
                if let Some(plan_caps) = plan_re.captures(line) {
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
