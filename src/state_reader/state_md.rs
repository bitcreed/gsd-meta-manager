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
    /// ADR-2207: current phase number (GSD 1.8.0 frontmatter). Stored as a
    /// string but accepts a YAML string OR number so a numeric value never
    /// aborts frontmatter parsing.
    #[serde(default, deserialize_with = "de_opt_scalar_string")]
    pub current_phase: Option<String>,
    /// ADR-2207: human-readable current phase name (GSD 1.8.0 frontmatter).
    #[serde(default)]
    pub current_phase_name: Option<String>,
    /// ADR-2207: current plan identifier (GSD 1.8.0 frontmatter). Accepts a
    /// YAML string OR number (e.g. `"0.3"` or `14`).
    #[serde(default, deserialize_with = "de_opt_scalar_string")]
    pub current_plan: Option<String>,
    #[serde(default)]
    pub progress: ProgressInfo,
}

/// Deserialize a YAML scalar (string, integer, float, or null) into
/// `Option<String>`.
///
/// GSD 1.8.0 may write `current_phase` / `current_plan` either quoted
/// (`"14"`, `"0.3"`) or bare (`14`). serde_yml would otherwise reject a bare
/// number for a `String`/`Option<String>` field and abort the *entire*
/// frontmatter parse (`parse_state_md` returning `None` zeroes every field).
/// This visitor normalizes any scalar shape to its string form; null/absent
/// yields `None`.
fn de_opt_scalar_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct ScalarStringVisitor;

    impl<'de> serde::de::Visitor<'de> for ScalarStringVisitor {
        type Value = Option<String>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a string, integer, float, or null")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }
        fn visit_string<E>(self, v: String) -> Result<Self::Value, E> {
            Ok(Some(v))
        }
        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }
        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }
        fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E> {
            Ok(Some(v.to_string()))
        }
        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(self)
        }
    }

    deserializer.deserialize_option(ScalarStringVisitor)
}

/// ADR-2207: true when `status` denotes milestone termination — the milestone
/// is fully done (`<version> milestone complete`) or the project is between
/// milestones (`Awaiting next milestone`). Case-insensitive.
pub fn is_milestone_terminal(status: &str) -> bool {
    let s = status.to_lowercase();
    s.contains("milestone complete") || s.contains("awaiting next milestone")
}

/// ADR-2207: true when `status` is the intermediate `All phases complete`
/// state — every phase is done but the milestone has NOT been terminated
/// (the project awaits `/gsd:complete-milestone`). Case-insensitive.
pub fn is_all_phases_complete(status: &str) -> bool {
    status.to_lowercase().contains("all phases complete")
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

    #[test]
    fn test_frontmatter_current_phase_keys_string_form() {
        let content = "---\nstatus: executing\ncurrent_phase: \"14\"\ncurrent_phase_name: Dashboard\ncurrent_plan: \"0.3\"\n---\n";
        let fm = parse_state_md(content).unwrap();
        assert_eq!(fm.current_phase.as_deref(), Some("14"));
        assert_eq!(fm.current_phase_name.as_deref(), Some("Dashboard"));
        assert_eq!(fm.current_plan.as_deref(), Some("0.3"));
    }

    #[test]
    fn test_frontmatter_current_phase_numeric_form() {
        // A bare (unquoted) numeric current_phase / current_plan must NOT abort
        // frontmatter parsing — it is coerced to its string form.
        let content = "---\nstatus: executing\ncurrent_phase: 14\ncurrent_plan: 2\n---\n";
        let fm = parse_state_md(content).unwrap();
        assert_eq!(fm.status, "executing");
        assert_eq!(fm.current_phase.as_deref(), Some("14"));
        assert_eq!(fm.current_plan.as_deref(), Some("2"));
    }

    #[test]
    fn test_frontmatter_current_phase_absent_is_none() {
        let content = "---\nstatus: active\n---\n";
        let fm = parse_state_md(content).unwrap();
        assert_eq!(fm.current_phase, None);
        assert_eq!(fm.current_phase_name, None);
        assert_eq!(fm.current_plan, None);
    }

    #[test]
    fn test_is_milestone_terminal() {
        assert!(is_milestone_terminal("v1.5.0 milestone complete"));
        assert!(is_milestone_terminal("Awaiting next milestone"));
        assert!(is_milestone_terminal("V1.5.0 MILESTONE COMPLETE")); // case-insensitive
        assert!(!is_milestone_terminal("All phases complete"));
        assert!(!is_milestone_terminal("executing"));
    }

    #[test]
    fn test_is_all_phases_complete() {
        assert!(is_all_phases_complete("All phases complete"));
        assert!(is_all_phases_complete("all phases complete")); // case-insensitive
        assert!(!is_all_phases_complete("v1.5.0 milestone complete"));
        assert!(!is_all_phases_complete("Awaiting next milestone"));
        assert!(!is_all_phases_complete("executing"));
    }
}
