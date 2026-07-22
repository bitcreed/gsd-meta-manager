use std::collections::HashSet;
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
    pub has_uat: bool,
    pub has_spec: bool,
    pub has_eval_review: bool,
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

/// Detect whether a plan file's YAML frontmatter declares `status: superseded`.
///
/// GSD 1.8.0 (#2349): a plan marked `status: superseded` was deliberately
/// reassigned or never executed — its work moved to a later plan, so it can
/// never gain a matching `*-SUMMARY.md`. Such a plan is excluded from BOTH the
/// plan and summary counts. We parse only the leading `---`…`---` frontmatter
/// block via a cheap line scan (no YAML dependency); a plan without the marker
/// is counted exactly as before. Fail-safe: a file with no frontmatter, or a
/// closed block with no `status: superseded`, is treated as a normal plan.
fn plan_frontmatter_superseded(content: &str) -> bool {
    let mut lines = content.lines();
    // Frontmatter must open on the very first line with a bare `---`.
    if lines.next().map(str::trim) != Some("---") {
        return false;
    }
    for line in lines {
        if line.trim() == "---" {
            // End of frontmatter block without a superseded marker.
            return false;
        }
        if let Some((key, value)) = line.split_once(':') {
            if key.trim() == "status" && value.trim().eq_ignore_ascii_case("superseded") {
                return true;
            }
        }
    }
    false
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

    // GSD 1.8.0 counting is a two-pass scan (#1988, #2349):
    //   Pass 1 collects the IDs of surviving (non-superseded) plans.
    //   Pass 2 counts only summaries whose ID matches a surviving plan.
    // Both passes run inside the single directory iteration below: plan IDs are
    // gathered into `plan_ids` and candidate summary names into `summary_names`,
    // then paired after the loop.
    let mut plan_ids: HashSet<String> = HashSet::new();
    let mut summary_names: Vec<String> = Vec::new();
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
    let mut has_uat = false;
    let mut has_spec = false;
    let mut has_eval_review = false;

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
        if name == "EVAL-REVIEW.md" || name.ends_with("-EVAL-REVIEW.md") {
            has_eval_review = true;
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
        if name == "SPEC.md" || name.ends_with("-SPEC.md") {
            has_spec = true;
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
        // PLAN-REVIEW artifacts are review notes, not plans (#2349): skip before
        // the PLAN match so they never count toward plan_count or set has_plans.
        if name == "PLAN-REVIEW.md" || name.ends_with("-PLAN-REVIEW.md") {
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
        if name == "UAT.md" || name.ends_with("-UAT.md") {
            has_uat = true;
            continue;
        }

        // Pass 1 — Match PLAN.md or *-PLAN.md (after PLAN-CHECK/PLAN-REVIEW are
        // filtered above). Derive the plan ID (filename minus the PLAN suffix;
        // standalone PLAN.md → empty-string ID) and record it, unless the plan's
        // frontmatter marks it `status: superseded`.
        if name == "PLAN.md" || name.ends_with("-PLAN.md") {
            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                if plan_frontmatter_superseded(&content) {
                    continue;
                }
            }
            let id = name
                .strip_suffix("-PLAN.md")
                .map(str::to_string)
                .unwrap_or_default();
            plan_ids.insert(id);
            continue;
        }

        // Pass 2 (collection) — Match SUMMARY.md or *-SUMMARY.md, excluding FIX and
        // GAPCLOSURE summaries which are never plan partners (#1988). The ID→plan
        // pairing happens after the loop once all surviving plan IDs are known.
        if name == "SUMMARY.md" || name.ends_with("-SUMMARY.md") {
            if name.contains("-FIX-") || name.ends_with("-GAPCLOSURE-SUMMARY.md") {
                continue;
            }
            summary_names.push(name);
            continue;
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

    // Pass 2 (pairing) — a summary counts only if its ID matches a surviving
    // (non-superseded) plan ID (matched-summary rule, #1988). Standalone
    // SUMMARY.md derives the empty-string ID and pairs with standalone PLAN.md.
    let plan_count: u32 = plan_ids.len() as u32;
    let summary_count: u32 = summary_names
        .iter()
        .filter(|name| {
            let id = name
                .strip_suffix("-SUMMARY.md")
                .map(str::to_string)
                .unwrap_or_default();
            plan_ids.contains(&id)
        })
        .count() as u32;

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
        has_uat,
        has_spec,
        has_eval_review,
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

/// Test whether a phase directory name belongs to the given phase number.
///
/// GSD 1.8.0 phase directories are no longer always `NN-slug`: they may be
/// decimal (`0.3-slug`), milestone-prefixed (`M1-2-slug`), project-code-prefixed
/// (`AB-29-slug`), or year-prefixed multi-segment (`14-2026-foo`). Matching must
/// also be pad-insensitive (`3-foo` and `03-foo` both match phase `3`) while
/// still rejecting bare-number-vs-longer-number collisions (`1` must not match
/// `14-foo` or `1.2-foo`).
///
/// A candidate list is built from `phase_number` — always the raw string, plus
/// (for all-digit numbers) the zero-padded-to-2 and leading-zeros-stripped forms.
/// A directory matches a candidate `C` when it equals `C` or starts with `C-`;
/// the trailing `-` is the boundary guard against longer-number collisions.
fn phase_dir_matches(dir_name: &str, phase_number: &str) -> bool {
    let mut candidates: Vec<String> = vec![phase_number.to_string()];
    if !phase_number.is_empty() && phase_number.chars().all(|c| c.is_ascii_digit()) {
        if phase_number.len() < 2 {
            candidates.push(format!("{:0>2}", phase_number));
        }
        let stripped = phase_number.trim_start_matches('0');
        candidates.push(if stripped.is_empty() {
            "0".to_string()
        } else {
            stripped.to_string()
        });
    }
    candidates
        .iter()
        .any(|c| dir_name == c || dir_name.starts_with(&format!("{c}-")))
}

/// Find a phase directory by its number, checking both active phases/ and archived milestones/.
///
/// Search order:
/// 1. planning_dir/phases/ for directories starting with zero-padded phase number (e.g., "05-")
/// 2. planning_dir/milestones/*/ for archived phases
pub fn find_phase_dir(planning_dir: &Path, phase_number: &str) -> Option<PathBuf> {
    // Check phases/ directory first
    let phases_dir = planning_dir.join("phases");
    if let Ok(entries) = std::fs::read_dir(&phases_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    if phase_dir_matches(name, phase_number) {
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
                                if phase_dir_matches(name, phase_number) {
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
                                if phase_dir_matches(name, phase_number) {
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
    fn test_uat_md_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-UAT.md"), "uat").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_uat, "has_uat should be true for *-UAT.md");
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
        assert_eq!(result.status, DiskStatus::Empty);
    }

    #[test]
    fn test_standalone_uat_md_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("UAT.md"), "uat").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_uat, "has_uat should be true for bare UAT.md");
    }

    #[test]
    fn test_empty_dir_has_no_uat() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert!(!result.has_uat);
    }

    #[test]
    fn test_spec_md_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-SPEC.md"), "spec").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_spec);
        assert!(!result.has_ui_spec);
        assert!(!result.has_ai_spec);
        assert_eq!(result.status, DiskStatus::Empty);
    }

    #[test]
    fn test_ai_spec_does_not_trigger_spec() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-AI-SPEC.md"), "ai-spec").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_ai_spec);
        assert!(!result.has_spec, "AI-SPEC.md must not be misclassified as SPEC.md");
    }

    #[test]
    fn test_eval_review_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-EVAL-REVIEW.md"), "eval").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_eval_review);
        assert!(!result.has_review, "EVAL-REVIEW.md must not be misclassified as REVIEW.md");
    }

    #[test]
    fn test_empty_dir_has_no_spec_or_eval_review() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert!(!result.has_spec);
        assert!(!result.has_eval_review);
    }

    // ── Task 1: GSD 1.8.0 counting — exclusions, superseded, matched-summary ──

    #[test]
    fn test_fix_summary_not_counted() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        // A FIX summary must not inflate the count or flip status.
        fs::write(dir.path().join("05-01-FIX-01-SUMMARY.md"), "fix").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(
            result.summary_count, 1,
            "FIX summary must not be counted in summary_count"
        );
        assert_eq!(result.status, DiskStatus::Complete);
    }

    #[test]
    fn test_gapclosure_summary_not_counted() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        // GAPCLOSURE summary must not flip a still-incomplete phase to Complete.
        fs::write(dir.path().join("05-GAPCLOSURE-SUMMARY.md"), "gap").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 2);
        assert_eq!(
            result.summary_count, 1,
            "GAPCLOSURE summary must not be counted"
        );
        assert_eq!(result.status, DiskStatus::Partial);
    }

    #[test]
    fn test_plan_review_not_counted_as_plan() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-01-PLAN-REVIEW.md"), "review").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.plan_count, 1,
            "PLAN-REVIEW.md must not be counted as a plan"
        );
        assert_eq!(result.status, DiskStatus::Planned);
    }

    #[test]
    fn test_superseded_plan_excluded_from_counts() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        // A superseded plan is dropped from the denominator, and its summary
        // (if any) is not counted either.
        fs::write(
            dir.path().join("05-02-PLAN.md"),
            "---\nstatus: superseded\n---\nbody",
        )
        .unwrap();
        fs::write(dir.path().join("05-02-SUMMARY.md"), "summary2").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.plan_count, 1,
            "superseded plan must be excluded from plan_count"
        );
        assert_eq!(
            result.summary_count, 1,
            "summary of a superseded plan must not be counted"
        );
        assert_eq!(result.status, DiskStatus::Complete);
    }

    #[test]
    fn test_unmatched_summary_not_counted() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        // A summary with no matching plan ID must not be counted.
        fs::write(dir.path().join("05-99-SUMMARY.md"), "orphan").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(
            result.summary_count, 0,
            "summary without a matching plan must not be counted"
        );
        assert_eq!(result.status, DiskStatus::Planned);
    }

    #[test]
    fn test_normal_two_plan_two_summary_still_complete() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        fs::write(dir.path().join("05-02-SUMMARY.md"), "summary2").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 2);
        assert_eq!(result.status, DiskStatus::Complete);
    }

    #[test]
    fn test_standalone_summary_requires_standalone_plan() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("PLAN.md"), "plan").unwrap();
        fs::write(dir.path().join("SUMMARY.md"), "summary").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(result.summary_count, 1);
        assert_eq!(result.status, DiskStatus::Complete);
    }

    #[test]
    fn test_standalone_summary_without_plan_not_counted() {
        let dir = tempdir().unwrap();
        // Standalone SUMMARY.md with no PLAN.md must not be counted.
        fs::write(dir.path().join("SUMMARY.md"), "summary").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
        assert_eq!(result.status, DiskStatus::Empty);
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

    // ── Task 2: robust phase-token / directory matching ──

    #[test]
    fn test_phase_dir_matches_year_prefixed() {
        assert!(phase_dir_matches("14-2026-foo", "14"));
    }

    #[test]
    fn test_phase_dir_matches_decimal() {
        assert!(phase_dir_matches("0.3-slug", "0.3"));
    }

    #[test]
    fn test_phase_dir_matches_milestone_prefixed() {
        assert!(phase_dir_matches("M1-2-slug", "M1-2"));
    }

    #[test]
    fn test_phase_dir_matches_project_code_prefixed() {
        assert!(phase_dir_matches("AB-29-slug", "AB-29"));
    }

    #[test]
    fn test_phase_dir_matches_pad_insensitive() {
        assert!(phase_dir_matches("3-foo", "3"));
        assert!(phase_dir_matches("03-foo", "3"));
    }

    #[test]
    fn test_phase_dir_matches_rejects_longer_number() {
        assert!(!phase_dir_matches("14-foo", "1"));
    }

    #[test]
    fn test_phase_dir_matches_rejects_decimal_boundary() {
        assert!(!phase_dir_matches("1.2-foo", "1"));
    }

    #[test]
    fn test_find_phase_dir_year_prefixed() {
        let dir = tempdir().unwrap();
        let phase_dir = dir.path().join("phases").join("14-2026-foo");
        fs::create_dir_all(&phase_dir).unwrap();

        let found = find_phase_dir(dir.path(), "14");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), phase_dir);
    }

    #[test]
    fn test_find_phase_dir_decimal() {
        let dir = tempdir().unwrap();
        let phase_dir = dir.path().join("phases").join("0.3-foo");
        fs::create_dir_all(&phase_dir).unwrap();

        let found = find_phase_dir(dir.path(), "0.3");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), phase_dir);
    }
}
