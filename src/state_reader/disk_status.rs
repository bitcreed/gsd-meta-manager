use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum DiskStatus {
    #[default]
    NoDirectory,
    Empty,
    Discussed,
    Researched,
    Planned,
    Partial,
    Complete,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiskInference {
    pub status: DiskStatus,
    pub plan_count: u32,
    pub summary_count: u32,
    pub has_plans: bool,
    pub has_summaries: bool,
    pub has_context: bool,
    pub has_research: bool,
    pub has_verification: bool,
    pub has_security: bool,
    /// Sub-stage artifacts (per /gsd-settings Planning + Execution toggles).
    pub has_patterns: bool,
    pub has_plan_check: bool,
    pub has_validation: bool,
    pub has_ui_spec: bool,
    pub has_ui_check: bool,
    pub has_ai_spec: bool,
    pub has_review: bool,
    pub has_ui_review: bool,
}

/// Infer the GSD status of a phase directory by scanning its file artifacts.
///
/// Follows GSD's algorithm (from roadmap.cjs:127-166):
/// 1. If not a directory, return NoDirectory
/// 2. Count files matching *-PLAN.md or PLAN.md
/// 3. Count files matching *-SUMMARY.md or SUMMARY.md
/// 4. Check for *-CONTEXT.md or CONTEXT.md
/// 5. Check for *-RESEARCH.md or RESEARCH.md
/// 6. Check for *-VERIFICATION.md or VERIFICATION.md
/// 7. Derive status from artifact presence
pub fn infer_disk_status(phase_dir: &Path) -> DiskInference {
    if !phase_dir.is_dir() {
        return DiskInference {
            status: DiskStatus::NoDirectory,
            ..Default::default()
        };
    }

    let entries = match std::fs::read_dir(phase_dir) {
        Ok(entries) => entries,
        Err(_) => {
            return DiskInference {
                status: DiskStatus::NoDirectory,
                ..Default::default()
            }
        }
    };

    let mut plan_count: u32 = 0;
    let mut summary_count: u32 = 0;
    let mut has_context = false;
    let mut has_research = false;
    let mut has_verification = false;
    let mut has_patterns = false;
    let mut has_plan_check = false;
    let mut has_validation = false;
    let mut has_ui_spec = false;
    let mut has_ui_check = false;
    let mut has_ai_spec = false;
    let mut has_review = false;
    let mut has_ui_review = false;
    let mut has_security = false;

    for entry in entries.flatten() {
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Only consider files (not directories)
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(true) {
            continue;
        }

        // Skip review (UI-REVIEW vs REVIEW) and validate before generic SUMMARY/PLAN
        // matches so we don't double-count.
        if name == "UI-REVIEW.md" || name.ends_with("-UI-REVIEW.md") {
            has_ui_review = true;
            continue;
        }
        if name == "REVIEW.md" || name.ends_with("-REVIEW.md") {
            has_review = true;
            continue;
        }
        if name == "UI-SPEC.md" || name.ends_with("-UI-SPEC.md") {
            has_ui_spec = true;
            continue;
        }
        if name == "UI-CHECK.md" || name.ends_with("-UI-CHECK.md") {
            has_ui_check = true;
            continue;
        }
        if name == "AI-SPEC.md" || name.ends_with("-AI-SPEC.md") {
            has_ai_spec = true;
            continue;
        }
        if name == "PATTERNS.md" || name.ends_with("-PATTERNS.md") {
            has_patterns = true;
            continue;
        }
        if name == "PLAN-CHECK.md" || name.ends_with("-PLAN-CHECK.md") {
            has_plan_check = true;
            continue;
        }
        if name == "VALIDATION.md" || name.ends_with("-VALIDATION.md") {
            has_validation = true;
            continue;
        }
        if name == "SECURITY.md" || name.ends_with("-SECURITY.md") {
            has_security = true;
            continue;
        }

        // Match PLAN.md or *-PLAN.md (after PLAN-CHECK is filtered above)
        if name == "PLAN.md" || name.ends_with("-PLAN.md") {
            plan_count += 1;
        }

        // Match SUMMARY.md or *-SUMMARY.md
        if name == "SUMMARY.md" || name.ends_with("-SUMMARY.md") {
            summary_count += 1;
        }

        // Match CONTEXT.md or *-CONTEXT.md
        if name == "CONTEXT.md" || name.ends_with("-CONTEXT.md") {
            has_context = true;
        }

        // Match RESEARCH.md or *-RESEARCH.md
        if name == "RESEARCH.md" || name.ends_with("-RESEARCH.md") {
            has_research = true;
        }

        // Match VERIFICATION.md or *-VERIFICATION.md
        if name == "VERIFICATION.md" || name.ends_with("-VERIFICATION.md") {
            has_verification = true;
        }
    }

    // Determine status following GSD's priority order
    let status = if summary_count >= plan_count && plan_count > 0 {
        DiskStatus::Complete
    } else if summary_count > 0 {
        DiskStatus::Partial
    } else if plan_count > 0 {
        DiskStatus::Planned
    } else if has_research {
        DiskStatus::Researched
    } else if has_context {
        DiskStatus::Discussed
    } else {
        DiskStatus::Empty
    };

    DiskInference {
        status,
        plan_count,
        summary_count,
        has_plans: plan_count > 0,
        has_summaries: summary_count > 0,
        has_context,
        has_research,
        has_verification,
        has_security,
        has_patterns,
        has_plan_check,
        has_validation,
        has_ui_spec,
        has_ui_check,
        has_ai_spec,
        has_review,
        has_ui_review,
    }
}

/// Find a phase directory by its number, checking both active phases/ and archived milestones/.
///
/// Search order:
/// 1. planning_dir/phases/ for directories starting with zero-padded phase number (e.g., "05-")
/// 2. planning_dir/milestones/*/ for archived phases
pub fn find_phase_dir(planning_dir: &Path, phase_number: &str) -> Option<PathBuf> {
    // Zero-pad to 2 digits for prefix matching
    let padded = if phase_number.len() == 1 {
        format!("0{}", phase_number)
    } else {
        phase_number.to_string()
    };
    let prefix = format!("{}-", padded);

    // Check phases/ directory first
    let phases_dir = planning_dir.join("phases");
    if let Ok(entries) = std::fs::read_dir(&phases_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(&prefix) {
                        return Some(entry.path());
                    }
                }
            }
        }
    }

    // Check milestones/*/ directories for archived phases
    let milestones_dir = planning_dir.join("milestones");
    if let Ok(milestone_entries) = std::fs::read_dir(&milestones_dir) {
        for milestone_entry in milestone_entries.flatten() {
            if milestone_entry
                .file_type()
                .map(|t| t.is_dir())
                .unwrap_or(false)
            {
                if let Ok(phase_entries) = std::fs::read_dir(milestone_entry.path()) {
                    for phase_entry in phase_entries.flatten() {
                        if phase_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            if let Some(name) = phase_entry.file_name().to_str() {
                                if name.starts_with(&prefix) {
                                    return Some(phase_entry.path());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Infer a phase's status by locating its directory and scanning artifacts.
/// Archived phases (in milestones/) are always Complete per Pitfall 4.
pub fn infer_phase_status(planning_dir: &Path, phase_number: &str) -> DiskInference {
    let padded = if phase_number.len() == 1 {
        format!("0{}", phase_number)
    } else {
        phase_number.to_string()
    };
    let prefix = format!("{}-", padded);

    // Check if phase is in milestones/ (archived = Complete)
    let milestones_dir = planning_dir.join("milestones");
    if let Ok(milestone_entries) = std::fs::read_dir(&milestones_dir) {
        for milestone_entry in milestone_entries.flatten() {
            if milestone_entry
                .file_type()
                .map(|t| t.is_dir())
                .unwrap_or(false)
            {
                if let Ok(phase_entries) = std::fs::read_dir(milestone_entry.path()) {
                    for phase_entry in phase_entries.flatten() {
                        if phase_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            if let Some(name) = phase_entry.file_name().to_str() {
                                if name.starts_with(&prefix) {
                                    // Archived phase -- always Complete
                                    return DiskInference {
                                        status: DiskStatus::Complete,
                                        ..Default::default()
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check phases/ directory
    match find_phase_dir(planning_dir, phase_number) {
        Some(dir) => infer_disk_status(&dir),
        None => DiskInference {
            status: DiskStatus::NoDirectory,
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_empty_directory_returns_empty() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Empty);
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
    }

    #[test]
    fn test_context_only_returns_discussed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-CONTEXT.md"), "context").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Discussed);
        assert!(result.has_context);
    }

    #[test]
    fn test_context_and_research_returns_researched() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-CONTEXT.md"), "context").unwrap();
        fs::write(dir.path().join("05-RESEARCH.md"), "research").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Researched);
        assert!(result.has_context);
        assert!(result.has_research);
    }

    #[test]
    fn test_plans_only_returns_planned() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Planned);
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 0);
        assert!(result.has_plans);
        assert!(!result.has_summaries);
    }

    #[test]
    fn test_partial_summaries_returns_partial() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Partial);
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 1);
    }

    #[test]
    fn test_all_summaries_returns_complete() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        fs::write(dir.path().join("05-02-SUMMARY.md"), "summary2").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Complete);
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 2);
        assert!(result.has_plans);
        assert!(result.has_summaries);
    }

    #[test]
    fn test_standalone_plan_counted() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("PLAN.md"), "plan").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Planned);
        assert_eq!(result.plan_count, 1);
    }

    #[test]
    fn test_nonexistent_directory_returns_no_directory() {
        let result = infer_disk_status(Path::new("/nonexistent/path/does/not/exist"));
        assert_eq!(result.status, DiskStatus::NoDirectory);
    }

    #[test]
    fn test_has_plans_and_has_summaries_booleans() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(
            result.has_plans,
            "has_plans should be true when plans exist"
        );
        assert!(
            result.has_summaries,
            "has_summaries should be true when summaries exist"
        );
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 1);
    }

    #[test]
    fn test_empty_dir_has_no_plans_or_summaries() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert!(!result.has_plans);
        assert!(!result.has_summaries);
    }

    #[test]
    fn test_disk_status_ordering() {
        assert!(DiskStatus::NoDirectory < DiskStatus::Empty);
        assert!(DiskStatus::Empty < DiskStatus::Discussed);
        assert!(DiskStatus::Discussed < DiskStatus::Researched);
        assert!(DiskStatus::Researched < DiskStatus::Planned);
        assert!(DiskStatus::Planned < DiskStatus::Partial);
        assert!(DiskStatus::Partial < DiskStatus::Complete);
    }

    #[test]
    fn test_archived_phase_returns_complete() {
        let dir = tempdir().unwrap();
        // Create milestone archive structure: milestones/v1.0/05-something/
        let milestone_dir = dir
            .path()
            .join("milestones")
            .join("v1.0")
            .join("05-state-reader");
        fs::create_dir_all(&milestone_dir).unwrap();
        // Put some content in the archived phase
        fs::write(milestone_dir.join("05-01-PLAN.md"), "plan").unwrap();

        // Also create phases/ directory (empty, no phase 05 there)
        fs::create_dir_all(dir.path().join("phases")).unwrap();

        let result = infer_phase_status(dir.path(), "05");
        assert_eq!(result.status, DiskStatus::Complete);
    }

    #[test]
    fn test_find_phase_dir_in_phases() {
        let dir = tempdir().unwrap();
        let phase_dir = dir.path().join("phases").join("05-state-reader");
        fs::create_dir_all(&phase_dir).unwrap();

        let found = find_phase_dir(dir.path(), "05");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), phase_dir);
    }

    #[test]
    fn test_security_md_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-SECURITY.md"), "security").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_security, "has_security should be true for *-SECURITY.md");
        // SECURITY.md is informational — must not affect plan/summary/status counts.
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
        assert_eq!(result.status, DiskStatus::Empty);
    }

    #[test]
    fn test_standalone_security_md_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("SECURITY.md"), "security").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_security, "has_security should be true for bare SECURITY.md");
    }

    #[test]
    fn test_empty_dir_has_no_security() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert!(!result.has_security);
    }

    #[test]
    fn test_find_phase_dir_in_milestones() {
        let dir = tempdir().unwrap();
        let milestone_phase = dir
            .path()
            .join("milestones")
            .join("v1.0")
            .join("03-something");
        fs::create_dir_all(&milestone_phase).unwrap();
        fs::create_dir_all(dir.path().join("phases")).unwrap();

        let found = find_phase_dir(dir.path(), "03");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), milestone_phase);
    }
}
