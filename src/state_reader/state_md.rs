use serde::Deserialize;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct StateFrontmatter {
    #[serde(default)]
    pub gsd_state_version: f64,
    #[serde(default)]
    pub milestone: String,
    #[serde(default)]
    pub milestone_name: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub stopped_at: String,
    #[serde(default)]
    pub last_updated: String,
    #[serde(default)]
    pub last_activity: Option<String>,
    #[serde(default)]
    pub progress: ProgressInfo,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ProgressInfo {
    #[serde(default)]
    pub total_phases: u32,
    #[serde(default)]
    pub completed_phases: u32,
    #[serde(default)]
    pub total_plans: u32,
    #[serde(default)]
    pub completed_plans: u32,
    #[serde(default)]
    pub percent: u32,
}

/// Extract YAML frontmatter from a GSD STATE.md file.
/// Returns None if the file doesn't start with `---`.
pub fn extract_frontmatter(content: &str) -> Option<&str> {
    let content = content.trim_start();
    if !content.starts_with("---") {
        return None;
    }
    // Skip the opening ---
    let after_open = &content[3..];
    let after_open = after_open.trim_start_matches(['\r', '\n']);

    // Find closing --- (must be on its own line)
    if let Some(end) = after_open.find("\n---") {
        Some(&after_open[..end])
    } else {
        None
    }
}

/// Parse STATE.md content into a StateFrontmatter struct.
/// Returns None on any failure (missing frontmatter, invalid YAML).
pub fn parse_state_md(content: &str) -> Option<StateFrontmatter> {
    let yaml = extract_frontmatter(content)?;
    match serde_yml::from_str(yaml) {
        Ok(fm) => Some(fm),
        Err(e) => {
            tracing::warn!("Failed to parse STATE.md frontmatter: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter_basic() {
        let content = "---\nstatus: planning\n---\n# Body";
        let fm = extract_frontmatter(content).unwrap();
        assert!(fm.contains("status: planning"));
    }

    #[test]
    fn test_extract_frontmatter_with_hr_in_body() {
        let content = "---\nstatus: planning\n---\n# Body\n\n---\n\nMore content";
        let fm = extract_frontmatter(content).unwrap();
        assert!(fm.contains("status: planning"));
        assert!(!fm.contains("More content"));
    }

    #[test]
    fn test_extract_frontmatter_empty() {
        assert!(extract_frontmatter("").is_none());
    }

    #[test]
    fn test_extract_frontmatter_no_delimiters() {
        assert!(extract_frontmatter("# Just markdown").is_none());
    }

    #[test]
    fn test_parse_state_md_real_content() {
        let content = "---\ngsd_state_version: 1.0\nmilestone: v1.0\nmilestone_name: milestone\nstatus: planning\nstopped_at: Phase 1 context gathered\nlast_updated: \"2026-03-25T04:22:49.224Z\"\nprogress:\n  total_phases: 4\n  completed_phases: 0\n  total_plans: 0\n  completed_plans: 0\n  percent: 0\n---\n# Project State\n\n---\n\nSome body content";
        let fm = parse_state_md(content).unwrap();
        assert_eq!(fm.status, "planning");
        assert_eq!(fm.progress.total_phases, 4);
        assert_eq!(fm.progress.completed_phases, 0);
        assert_eq!(fm.milestone, "v1.0");
    }

    #[test]
    fn test_parse_state_md_empty() {
        assert!(parse_state_md("").is_none());
    }

    #[test]
    fn test_parse_state_md_no_frontmatter() {
        assert!(parse_state_md("# Just a heading").is_none());
    }

    #[test]
    fn test_parse_state_md_missing_optional_fields() {
        let content = "---\nstatus: active\n---\n";
        let fm = parse_state_md(content).unwrap();
        assert_eq!(fm.status, "active");
        assert_eq!(fm.progress.total_phases, 0); // default
        assert_eq!(fm.milestone, ""); // default
    }
}
