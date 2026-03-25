use regex::Regex;

#[derive(Debug, Clone)]
pub struct RoadmapPhase {
    pub number: String,
    pub name: String,
    pub description: String,
    pub completed: bool,
}

/// Parse ROADMAP.md content and extract phase checklist items.
pub fn parse_roadmap_phases(content: &str) -> Vec<RoadmapPhase> {
    let re = Regex::new(
        r"- \[([ xX])\] \*\*Phase ([0-9.]+): (.+?)\*\*\s*[-\x{2014}]\s*(.*)"
    ).unwrap();

    content
        .lines()
        .filter_map(|line| {
            re.captures(line).map(|caps| RoadmapPhase {
                completed: &caps[1] != " ",
                number: caps[2].to_string(),
                name: caps[3].to_string(),
                description: caps[4].trim().to_string(),
            })
        })
        .collect()
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
}
