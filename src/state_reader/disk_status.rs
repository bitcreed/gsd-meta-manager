use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// GSD's eight-state phase vocabulary, in workflow order.
///
/// **Declaration order IS the ordering.** `Ord` is derived, so the position of a
/// variant in this list is its rank, and more than thirty comparisons across the
/// UI, the reader and the router read that rank. A new variant is therefore
/// *inserted at its semantic position*, never appended: an appended variant
/// sorts above [`DiskStatus::Complete`] and silently changes the meaning of
/// every one of those comparisons without failing to compile.
/// `test_disk_status_ordering` pins the relation rather than this comment
/// claiming it.
///
/// The vocabulary matches `init.cjs:1875-1888` one-for-one, which is what lets a
/// router rule keyed on a variant mean what the runtime means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum DiskStatus {
    #[default]
    NoDirectory,
    Empty,
    Discussed,
    Researched,
    Planned,
    Partial,
    /// **Implementation complete, verification not yet passed** — GSD's own
    /// `executed` (`init.cjs:1876`, predicate at `:181`).
    ///
    /// Inserted between [`DiskStatus::Partial`] and [`DiskStatus::Complete`],
    /// which is the only correct position: it outranks a partially-summarised
    /// phase and is outranked by one whose verification passed. Before this
    /// variant existed, `Complete` carried this meaning, and a phase whose
    /// verification was `human_needed` therefore read as finished — the exact
    /// collapse DRIVE-05 exists to prevent.
    Executed,
    /// **Implementation complete AND verification passed** — GSD's `complete`
    /// (`init.cjs:194-195`: `implementationComplete && verificationPassed`).
    Complete,
}

/// The status read from a phase's `*-VERIFICATION.md` leading frontmatter.
///
/// Six values, matching `verification.cjs:72-113`'s `VERIFICATION_ROUTING_TABLE`
/// keys exactly. Only three are ever *written* by GSD's verifier
/// (`VERIFIER_STATUSES = ['passed', 'gaps_found', 'human_needed']`,
/// `verification.cjs:50`); `stale`, `missing` and `unknown` are constructed
/// internally. All six are modelled here because a driven agent, a hand edit or
/// a future GSD version can put any of them on disk.
///
/// **Matched as a string with an explicit fallback that keeps the value**, in
/// the tolerant-wire-enum posture `executor::outcome` and `executor::stream_json`
/// already use: a typed parse that discarded an unrecognised value would lose the
/// one fact a human needs to see when GSD ships a seventh status.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum VerificationStatus {
    /// No `*-VERIFICATION.md`, no leading frontmatter block, or no `status` key.
    ///
    /// The default, and fail-safe by construction: an unreadable or malformed
    /// artifact yields "nothing observed", never an error and never a claim that
    /// verification passed.
    #[default]
    Missing,
    /// Verification passed. The **only** value that admits `DiskStatus::Complete`.
    Passed,
    /// The verifier found gaps. Takes precedence over staleness upstream
    /// (`verification.cjs:333-342`).
    GapsFound,
    /// The verifier needs a human judgement. The definitional DRIVE-05 gate.
    HumanNeeded,
    /// A `*-SUMMARY.md` is newer than the `*-VERIFICATION.md`.
    Stale,
    /// A value outside the table, carried **verbatim**.
    ///
    /// Never mapped onto a known arm and never dropped: an unrecognised status
    /// is not a passing one, and the observed bytes are what tells a reader
    /// which unknown it was.
    Unknown(String),
}

impl VerificationStatus {
    /// Classify a raw frontmatter value. Case-insensitive on the known arms;
    /// anything else is carried verbatim by [`VerificationStatus::Unknown`].
    pub fn from_raw(raw: &str) -> Self {
        let trimmed = raw.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "passed" => VerificationStatus::Passed,
            "gaps_found" => VerificationStatus::GapsFound,
            "human_needed" => VerificationStatus::HumanNeeded,
            "stale" => VerificationStatus::Stale,
            "missing" => VerificationStatus::Missing,
            _ => VerificationStatus::Unknown(trimmed.to_string()),
        }
    }

    /// The stable identifier a later reader greps for. Exhaustive, no wildcard.
    pub fn as_str(&self) -> &str {
        match self {
            VerificationStatus::Missing => "missing",
            VerificationStatus::Passed => "passed",
            VerificationStatus::GapsFound => "gaps_found",
            VerificationStatus::HumanNeeded => "human_needed",
            VerificationStatus::Stale => "stale",
            VerificationStatus::Unknown(observed) => observed,
        }
    }

    /// Whether this status is the one that admits `DiskStatus::Complete`.
    ///
    /// Spelled as a predicate rather than an `== Passed` at each call site so
    /// the completion rule has exactly one definition.
    pub fn is_passed(&self) -> bool {
        matches!(self, VerificationStatus::Passed)
    }
}

/// The UAT statuses `uat-predicate.cjs:30-32` treats as outstanding, plus the
/// tolerant arms either side of that set.
///
/// Same posture as [`VerificationStatus`]: a value outside the vocabulary is
/// carried verbatim rather than mapped onto a member of it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UatStatus {
    /// No `*-UAT.md`, no leading frontmatter block, or no `status` key.
    #[default]
    Missing,
    /// UAT is partially complete — outstanding.
    Partial,
    /// A UAT item was diagnosed but not closed — outstanding.
    Diagnosed,
    /// UAT has not been run — outstanding.
    Pending,
    /// UAT is blocked — outstanding.
    Blocked,
    /// UAT is underway — outstanding.
    InProgress,
    /// UAT failed — outstanding.
    Failed,
    /// Any other value, carried verbatim. **Not outstanding**: this repository's
    /// own phase 19 carries `status: deferred`, which is a human's explicit
    /// decision to proceed and must not read as an unanswered gate.
    Other(String),
}

impl UatStatus {
    /// Classify a raw frontmatter value. Case-insensitive on the known arms.
    pub fn from_raw(raw: &str) -> Self {
        let trimmed = raw.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "partial" => UatStatus::Partial,
            "diagnosed" => UatStatus::Diagnosed,
            "pending" => UatStatus::Pending,
            "blocked" => UatStatus::Blocked,
            "in_progress" => UatStatus::InProgress,
            "failed" => UatStatus::Failed,
            _ => UatStatus::Other(trimmed.to_string()),
        }
    }

    /// The stable identifier a later reader greps for. Exhaustive, no wildcard.
    pub fn as_str(&self) -> &str {
        match self {
            UatStatus::Missing => "missing",
            UatStatus::Partial => "partial",
            UatStatus::Diagnosed => "diagnosed",
            UatStatus::Pending => "pending",
            UatStatus::Blocked => "blocked",
            UatStatus::InProgress => "in_progress",
            UatStatus::Failed => "failed",
            UatStatus::Other(observed) => observed,
        }
    }

    /// Whether this status sets the outstanding-UAT gate (G7).
    ///
    /// The set is `uat-predicate.cjs:30-32`'s, taken as written. Everything
    /// outside it — including `passed` and `deferred` — is not a gate.
    pub fn is_outstanding(&self) -> bool {
        matches!(
            self,
            UatStatus::Partial
                | UatStatus::Diagnosed
                | UatStatus::Pending
                | UatStatus::Blocked
                | UatStatus::InProgress
                | UatStatus::Failed
        )
    }
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
    /// The `status` read from the phase's `*-VERIFICATION.md` frontmatter.
    ///
    /// **Presence and status answer different questions, and only the status can
    /// express a gate.** `has_verification` says an artifact exists; this says
    /// what it concluded. A phase with `has_verification: true` and
    /// `VerificationStatus::HumanNeeded` is *not* finished, and reading presence
    /// as completion is how an unattended run walks past the one gate DRIVE-05
    /// exists to stop at. `has_verification` is kept because the dashboard reads
    /// it as an artifact-presence badge.
    pub verification_status: VerificationStatus,
    pub has_security: bool,
    pub has_uat: bool,
    /// The `status` read from the phase's `*-UAT.md` frontmatter (G7).
    ///
    /// Presence is `has_uat`; whether anyone still owes an answer is this.
    /// [`UatStatus::is_outstanding`] is the gate predicate.
    pub uat_status: UatStatus,
    /// A phase-directory `.continue-here.md` carrying **at least one
    /// blocking-severity row** (G13).
    ///
    /// **Not a file-existence check, and it must never be simplified into one.**
    /// This repository carries a stale `.continue-here.md` in
    /// `.planning/phases/19-gitsafe-git-blast-radius-envelope/`, left behind by a
    /// completed phase; every one of its severity rows reads `advisory`. An
    /// existence test — or a substring search for the word "blocking", which its
    /// prose contains as part of the filename `tests/async_blocking_guard.rs` —
    /// would park every run against this project forever.
    /// `test_phase_19_stale_continue_here_marker_is_not_blocking` pins that.
    pub continue_here_blocking: bool,
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
    /// GSD 1.8.0 informational artifacts — never affect plan/summary counts.
    pub has_coverage: bool,
    pub has_windows: bool,
    pub has_deferred_items: bool,
    pub has_skeleton: bool,
}

/// Read a scalar key out of a file's **leading** YAML frontmatter block.
///
/// A cheap line scan, no YAML dependency, returning the first match's trimmed
/// value. Absence of a leading block, or of the key inside it, yields `None` —
/// fail-safe, never an error.
///
/// **The byte-zero anchor is load-bearing, not stylistic.** The block must open
/// on the very first line with a bare `---`, and the scan stops at the closing
/// `---`. GSD's own verification library records the defect this prevents
/// (`verification.cjs`'s `DEFECT.FRONTMATTER-SCALAR-BROAD-GREP`): a broad search
/// for `status:` false-matched the key inside a fenced code block further down
/// the file, so a document *describing* a status was read as *having* one.
/// Widening this to a whole-file search reintroduces that defect verbatim.
///
/// **The key must sit at column zero, and that is the same defect one level in**
/// (WR-05). Trimming the key made an indented mapping key indistinguishable from
/// a top-level one, so
///
/// ```text
/// ---
/// verification:
///   status: passed
/// status: human_needed
/// ---
/// ```
///
/// returned `passed` — the first match wins, and the nested one comes first.
/// This function is the single input to `VerificationStatus`, which is the
/// goal-met predicate and the whole DRIVE-05 gate set, so a nested
/// `status: passed` is a false `Decision::GoalMet` and a nested
/// `status: human_needed` a spurious park. A *nested* key is a different key.
fn leading_frontmatter_value(content: &str, key: &str) -> Option<String> {
    let mut lines = content.lines();
    // Frontmatter must open on the very first line with a bare `---`.
    if lines.next().map(str::trim) != Some("---") {
        return None;
    }
    for line in lines {
        if line.trim() == "---" {
            // End of the leading block. Nothing below it is frontmatter.
            return None;
        }
        // Column zero or it is somebody else's key. (List items were already
        // safe by accident — `- status` does not equal `status` — but a plain
        // indented mapping key was not.)
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        if let Some((found, value)) = line.split_once(':') {
            if found == key {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// Detect whether a plan file's YAML frontmatter declares `status: superseded`.
///
/// GSD 1.8.0 (#2349): a plan marked `status: superseded` was deliberately
/// reassigned or never executed — its work moved to a later plan, so it can
/// never gain a matching `*-SUMMARY.md`. Such a plan is excluded from BOTH the
/// plan and summary counts. A plan without the marker is counted exactly as
/// before. Fail-safe: a file with no frontmatter, or a closed block with no
/// `status: superseded`, is treated as a normal plan.
fn plan_frontmatter_superseded(content: &str) -> bool {
    leading_frontmatter_value(content, "status")
        .is_some_and(|value| value.eq_ignore_ascii_case("superseded"))
}

/// Read the `status` out of the phase directory's verification artifact.
///
/// `names` is every `*-VERIFICATION.md` seen in the directory. **They are sorted
/// and the first is read**, which is the tie-break GSD's own reader uses
/// (`verification.cjs:302-303`), so a directory holding more than one artifact
/// yields the same status on every read rather than whatever the filesystem
/// happened to hand back first.
///
/// Every failure mode — no artifact, an unreadable file, no leading block, no
/// `status` key — yields [`VerificationStatus::Missing`].
fn read_verification_status(phase_dir: &Path, mut names: Vec<String>) -> VerificationStatus {
    names.sort();
    names
        .first()
        .and_then(|name| std::fs::read_to_string(phase_dir.join(name)).ok())
        .and_then(|content| leading_frontmatter_value(&content, "status"))
        .map(|raw| VerificationStatus::from_raw(&raw))
        .unwrap_or_default()
}

/// Read the `status` out of the phase directory's UAT artifact.
///
/// Same sorted-first tie-break and the same byte-zero-anchored parse
/// [`read_verification_status`] uses, for the same reason: one directory, one
/// answer, on every read. Every failure mode yields [`UatStatus::Missing`].
fn read_uat_status(phase_dir: &Path, mut names: Vec<String>) -> UatStatus {
    names.sort();
    names
        .first()
        .and_then(|name| std::fs::read_to_string(phase_dir.join(name)).ok())
        .and_then(|content| leading_frontmatter_value(&content, "status"))
        .map(|raw| UatStatus::from_raw(&raw))
        .unwrap_or_default()
}

/// Normalize a markdown cell or value for comparison: strip emphasis and code
/// ticks, trim, lowercase.
fn normalize_cell(cell: &str) -> String {
    cell.trim()
        .trim_matches('*')
        .trim_matches('`')
        .trim()
        .to_ascii_lowercase()
}

/// Split a markdown table row into cells, dropping the outer pipes.
fn split_markdown_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

/// True when every cell of a table row is a `---` / `:--:` alignment marker.
fn is_alignment_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|c| {
            let t = c.trim();
            !t.is_empty() && t.chars().all(|ch| ch == '-' || ch == ':')
        })
}

/// Whether a `.continue-here.md` carries at least one blocking-severity ROW.
///
/// **A row, never the file.** GSD's `execute-phase.md:217-235` and
/// `discuss-phase.md:162-177` stop on a marker whose rows carry
/// `severity: blocking`; a marker whose rows are all advisory is a note, not a
/// gate. Two row shapes are recognised, both of which locate the severity as a
/// *field* rather than as text anywhere in the document:
///
/// 1. A markdown table with a `Severity` header column — the cell at that
///    column index is compared, so prose in a neighbouring column cannot match.
/// 2. A `severity: blocking` key line (optionally list-prefixed).
///
/// A substring search for "blocking" is deliberately NOT one of them. This
/// repository's own stale marker contains the word inside the filename
/// `tests/async_blocking_guard.rs`, and would fire on it.
fn continue_here_has_blocking_row(content: &str) -> bool {
    let mut severity_column: Option<usize> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('|') {
            let cells = split_markdown_row(trimmed);
            if is_alignment_row(&cells) {
                continue;
            }
            match severity_column {
                // Inside a table whose Severity column is known: compare that
                // cell, and only that cell.
                Some(index) => {
                    if cells.get(index).map(|c| normalize_cell(c)).as_deref() == Some("blocking") {
                        return true;
                    }
                }
                // Not yet in a severity table: is this row its header?
                None => {
                    severity_column = cells
                        .iter()
                        .position(|cell| normalize_cell(cell) == "severity");
                }
            }
            continue;
        }

        // A non-table line ends whatever table we were in, so a later table's
        // rows are never read against an earlier table's column index.
        severity_column = None;

        // Key-line shape: `severity: blocking`, `- severity: blocking`.
        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim_start_matches(['-', '*', ' ']).trim();
            if key.eq_ignore_ascii_case("severity") && normalize_cell(value) == "blocking" {
                return true;
            }
        }
    }

    false
}

/// Whether a phase directory's `.continue-here.md` sets the G13 gate.
///
/// Fail-safe: an absent or unreadable marker is "no gate observed".
fn phase_continue_here_blocking(phase_dir: &Path) -> bool {
    std::fs::read_to_string(phase_dir.join(".continue-here.md"))
        .map(|content| continue_here_has_blocking_row(&content))
        .unwrap_or(false)
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
    // Verification artifacts are COLLECTED, not flagged: the status lives inside
    // the file, and which file to read is decided after the scan by sorting.
    let mut verification_names: Vec<String> = Vec::new();
    // UAT artifacts are collected for the same reason, and read the same way.
    let mut uat_names: Vec<String> = Vec::new();
    let mut has_context = false;
    let mut has_research = false;
    let mut has_patterns = false;
    let mut has_plan_check = false;
    let mut has_validation = false;
    let mut has_ui_spec = false;
    let mut has_ui_check = false;
    let mut has_ai_spec = false;
    let mut has_review = false;
    let mut has_ui_review = false;
    let mut has_security = false;
    let mut has_spec = false;
    let mut has_eval_review = false;
    let mut has_coverage = false;
    let mut has_windows = false;
    let mut has_deferred_items = false;
    let mut has_skeleton = false;

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
            uat_names.push(name);
            continue;
        }
        // GSD 1.8.0 informational artifacts — flagged only, never counted.
        if name == "COVERAGE.md" || name.ends_with("-COVERAGE.md") {
            has_coverage = true;
            continue;
        }
        if name == "WINDOWS.md" || name.ends_with("-WINDOWS.md") {
            has_windows = true;
            continue;
        }
        if name == "deferred-items.md" || name.ends_with("-deferred-items.md") {
            has_deferred_items = true;
            continue;
        }
        if name == "SKELETON.md" || name.ends_with("-SKELETON.md") {
            has_skeleton = true;
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

        // Match VERIFICATION.md or *-VERIFICATION.md. Collected the way the
        // summary pass above collects, rather than setting a boolean and moving
        // on: presence cannot express a gate and the status can.
        if name == "VERIFICATION.md" || name.ends_with("-VERIFICATION.md") {
            verification_names.push(name);
        }
    }

    let has_verification = !verification_names.is_empty();
    let verification_status = read_verification_status(phase_dir, verification_names);
    let has_uat = !uat_names.is_empty();
    let uat_status = read_uat_status(phase_dir, uat_names);
    let continue_here_blocking = phase_continue_here_blocking(phase_dir);

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

    // Determine status following GSD's priority order (`init.cjs:1875-1888`).
    //
    // `implementation_complete` is GSD's predicate verbatim (`init.cjs:181`).
    // Before this plan it was the FIRST arm and yielded `Complete`, which made
    // this reader's `Complete` mean GSD's `executed` — so a phase whose
    // verification was `human_needed` read as finished everywhere in the tree.
    // It now yields `Executed`, and `Complete` requires the conjunct
    // `init.cjs:194-195` requires: implementation complete AND verification
    // passed.
    let implementation_complete = summary_count >= plan_count && plan_count > 0;
    let status = if implementation_complete && verification_status.is_passed() {
        DiskStatus::Complete
    } else if implementation_complete {
        DiskStatus::Executed
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
        verification_status,
        has_security,
        has_uat,
        uat_status,
        continue_here_blocking,
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
        has_coverage,
        has_windows,
        has_deferred_items,
        has_skeleton,
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
                                    // Archived phase -- always Complete.
                                    //
                                    // Deliberately NOT re-derived through the
                                    // verification conjunct: a phase archived
                                    // into a milestone shipped, and the
                                    // milestone archive is the corroboration.
                                    // Its `verification_status` stays `Missing`
                                    // because nothing was read, which is the
                                    // honest value — not a claim it passed.
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
    fn test_all_summaries_returns_executed_not_complete() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        fs::write(dir.path().join("05-02-SUMMARY.md"), "summary2").unwrap();
        let result = infer_disk_status(dir.path());
        // Every plan has a summary and there is no verification artifact: GSD
        // calls that `executed`, not `complete` (init.cjs:181 vs :194-195).
        assert_eq!(result.status, DiskStatus::Executed);
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

    /// The hazard this assertion message names, stated once and reused.
    const APPEND_HAZARD: &str = "DiskStatus derives Ord from DECLARATION ORDER. A variant \
         APPENDED rather than INSERTED at its semantic position still compiles, still passes \
         every equality test, and silently reorders every `<`/`>=` comparison in the tree — \
         including the dashboard pipeline's stage thresholds. Insert; never append.";

    #[test]
    fn test_disk_status_ordering() {
        assert!(DiskStatus::NoDirectory < DiskStatus::Empty, "{APPEND_HAZARD}");
        assert!(DiskStatus::Empty < DiskStatus::Discussed, "{APPEND_HAZARD}");
        assert!(DiskStatus::Discussed < DiskStatus::Researched, "{APPEND_HAZARD}");
        assert!(DiskStatus::Researched < DiskStatus::Planned, "{APPEND_HAZARD}");
        assert!(DiskStatus::Planned < DiskStatus::Partial, "{APPEND_HAZARD}");
        // The two that pin `Executed`'s inserted position. An appended
        // `Executed` would sort ABOVE `Complete` and fail the second.
        assert!(DiskStatus::Partial < DiskStatus::Executed, "{APPEND_HAZARD}");
        assert!(DiskStatus::Executed < DiskStatus::Complete, "{APPEND_HAZARD}");
    }

    // ── Plan 20-03 Task 1: GSD's vocabulary + the verification frontmatter ──

    /// A phase directory with `n` plans and `n` matching summaries — GSD's
    /// `implementation_complete` predicate satisfied, and nothing more.
    fn implementation_complete_dir() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("19-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("19-01-SUMMARY.md"), "summary1").unwrap();
        dir
    }

    #[test]
    fn test_implementation_complete_without_verification_is_executed() {
        let dir = implementation_complete_dir();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.status,
            DiskStatus::Executed,
            "every plan has a summary and no verification artifact exists — that is \
             GSD's `executed`, not its `complete`. Reading it as Complete is the \
             vocabulary collapse DRIVE-05 exists to prevent"
        );
        assert_ne!(result.status, DiskStatus::Complete);
        assert_eq!(result.verification_status, VerificationStatus::Missing);
        assert!(!result.has_verification);
    }

    #[test]
    fn test_passing_verification_makes_it_complete() {
        let dir = implementation_complete_dir();
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nphase: 19\nstatus: passed\n---\nbody\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Complete);
        assert_eq!(result.verification_status, VerificationStatus::Passed);
        assert!(result.has_verification);
    }

    #[test]
    fn test_gating_verification_statuses_stay_executed_and_carry_the_value() {
        for (raw, expected) in [
            ("human_needed", VerificationStatus::HumanNeeded),
            ("gaps_found", VerificationStatus::GapsFound),
            ("stale", VerificationStatus::Stale),
        ] {
            let dir = implementation_complete_dir();
            fs::write(
                dir.path().join("19-VERIFICATION.md"),
                format!("---\nstatus: {raw}\n---\nbody\n"),
            )
            .unwrap();
            let result = infer_disk_status(dir.path());
            assert_eq!(
                result.status,
                DiskStatus::Executed,
                "a `{raw}` verification is a gate; a phase carrying one must never \
                 read as Complete anywhere in the tree"
            );
            assert_eq!(result.verification_status, expected);
            // Presence and status answer different questions, and both are kept.
            assert!(result.has_verification);
        }
    }

    #[test]
    fn test_unrecognised_verification_status_is_carried_verbatim() {
        let dir = implementation_complete_dir();
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nstatus: reticulating_splines\n---\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.verification_status,
            VerificationStatus::Unknown("reticulating_splines".to_string()),
            "an unrecognised status must be carried verbatim, never mapped onto a \
             known arm and never dropped: it is not a passing status, and the bytes \
             are what tell a reader which unknown it was"
        );
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_status_key_below_the_leading_block_is_never_matched() {
        let dir = implementation_complete_dir();
        // No leading frontmatter at all, and a fenced code block further down
        // that *documents* a passing status. This is GSD's own
        // DEFECT.FRONTMATTER-SCALAR-BROAD-GREP, reproduced as a fixture.
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "# Verification\n\nThe verifier writes:\n\n```yaml\nstatus: passed\n```\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.verification_status,
            VerificationStatus::Missing,
            "the parse is anchored at byte zero and reads only the leading delimited \
             block. A broad search would read the fenced example as a real status and \
             mark an unverified phase Complete"
        );
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_closed_frontmatter_block_does_not_leak_into_the_body() {
        let dir = implementation_complete_dir();
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nphase: 19\n---\n\n```yaml\nstatus: passed\n```\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.verification_status,
            VerificationStatus::Missing,
            "the scan stops at the closing delimiter; a key below it is body text"
        );
    }

    #[test]
    fn test_a_nested_status_key_is_not_the_documents_status() {
        let dir = implementation_complete_dir();
        // A nested mapping key that sorts BEFORE the real one, which is the
        // whole shape: the first match used to win and the trimmed comparison
        // could not tell the two apart (WR-05). The same defect as the fenced
        // code block above, one level in.
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nphase: 19\nverification:\n  status: passed\nstatus: human_needed\n---\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.verification_status,
            VerificationStatus::HumanNeeded,
            "the document's status is the key at column zero. Reading the nested \
             one produces a false `passed`, which is a false Decision::GoalMet \
             and a run routed straight past the human_needed gate DRIVE-05 exists \
             to park at"
        );
        assert_eq!(
            result.status,
            DiskStatus::Executed,
            "and Complete stays out of reach, since it requires a passing \
             verification"
        );
    }

    #[test]
    fn test_a_top_level_status_after_a_nested_one_is_still_found() {
        // The non-vacuity half: skipping indented lines must not turn into
        // skipping the block. A `status` that follows nested keys is still the
        // document's.
        let dir = implementation_complete_dir();
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nprogress:\n  status: draft\n  items: 3\nstatus: passed\n---\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.verification_status, VerificationStatus::Passed);
        assert_eq!(result.status, DiskStatus::Complete);
    }

    #[test]
    fn test_two_verification_artifacts_always_yield_the_sorted_first() {
        let dir = implementation_complete_dir();
        fs::write(
            dir.path().join("19-01-VERIFICATION.md"),
            "---\nstatus: human_needed\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("19-02-VERIFICATION.md"),
            "---\nstatus: passed\n---\n",
        )
        .unwrap();
        // Repeated reads, because the defect this guards is directory-iteration
        // order — which is not stable and need not differ on any single read.
        for read in 0..8 {
            let result = infer_disk_status(dir.path());
            assert_eq!(
                result.verification_status,
                VerificationStatus::HumanNeeded,
                "read {read}: names are sorted and the first is read, the same \
                 tie-break GSD's own reader uses. Taking whichever the filesystem \
                 offered first would make the same directory report two different \
                 statuses on two different days"
            );
            assert_eq!(result.status, DiskStatus::Executed, "read {read}");
        }
    }

    #[test]
    fn test_unreadable_verification_artifact_is_fail_safe() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("19-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("19-01-SUMMARY.md"), "summary1").unwrap();
        // A *directory* named like the artifact: `read_to_string` fails.
        fs::create_dir(dir.path().join("19-VERIFICATION.md")).unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.verification_status,
            VerificationStatus::Missing,
            "an unreadable artifact yields the absent value, never an error and \
             never a panic — the reader runs against other people's repositories"
        );
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_verification_on_an_incomplete_phase_does_not_complete_it() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("19-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("19-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("19-01-SUMMARY.md"), "summary1").unwrap();
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nstatus: passed\n---\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.status,
            DiskStatus::Partial,
            "Complete is a CONJUNCTION: a passing verification over an unfinished \
             implementation is still unfinished"
        );
        assert_eq!(result.verification_status, VerificationStatus::Passed);
    }

    // ── Plan 20-03 Task 2: the rest of the disk-observable gate set ──

    #[test]
    fn test_uat_blocking_statuses_set_the_outstanding_gate() {
        // uat-predicate.cjs:30-32's set, taken as written.
        for raw in [
            "partial",
            "diagnosed",
            "pending",
            "blocked",
            "in_progress",
            "failed",
        ] {
            let dir = tempdir().unwrap();
            fs::write(
                dir.path().join("19-UAT.md"),
                format!("---\nstatus: {raw}\n---\n"),
            )
            .unwrap();
            let result = infer_disk_status(dir.path());
            assert!(
                result.uat_status.is_outstanding(),
                "`{raw}` is in GSD's blocking UAT set and must set the gate"
            );
            assert_eq!(result.uat_status.as_str(), raw);
            assert!(result.has_uat, "presence is still recorded alongside status");
        }
    }

    #[test]
    fn test_uat_statuses_outside_the_blocking_set_do_not_gate() {
        // The negative arm. `deferred` is this repository's own phase-19 value:
        // a human's explicit decision to proceed, not an unanswered question.
        for raw in ["passed", "deferred", "complete"] {
            let dir = tempdir().unwrap();
            fs::write(
                dir.path().join("19-UAT.md"),
                format!("---\nstatus: {raw}\n---\n"),
            )
            .unwrap();
            let result = infer_disk_status(dir.path());
            assert!(
                !result.uat_status.is_outstanding(),
                "`{raw}` is outside GSD's blocking set; a gate that fires on \
                 everything is as useless as one that fires on nothing"
            );
            assert_eq!(result.uat_status, UatStatus::Other(raw.to_string()));
        }
    }

    #[test]
    fn test_absent_uat_artifact_yields_the_missing_status() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.uat_status, UatStatus::Missing);
        assert!(!result.uat_status.is_outstanding());
        assert!(!result.has_uat);
    }

    #[test]
    fn test_continue_here_with_a_blocking_row_sets_the_phase_gate() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".continue-here.md"),
            "## Anti-Patterns\n\n| Pattern | Severity |\n|---|---|\n| Something | advisory |\n| \
             Something else | blocking |\n",
        )
        .unwrap();
        assert!(infer_disk_status(dir.path()).continue_here_blocking);
    }

    #[test]
    fn test_continue_here_key_line_form_sets_the_phase_gate() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".continue-here.md"),
            "---\ncontext: phase\n---\n\n- item: do the thing\n- severity: blocking\n",
        )
        .unwrap();
        assert!(infer_disk_status(dir.path()).continue_here_blocking);
    }

    #[test]
    fn test_continue_here_with_no_blocking_row_does_not_set_the_phase_gate() {
        let dir = tempdir().unwrap();
        // Rows exist; none is blocking. Plus the word "blocking" in prose and in
        // a filename — exactly the shape of this repository's stale marker.
        fs::write(
            dir.path().join(".continue-here.md"),
            "## Anti-Patterns\n\n| Pattern | Severity |\n|---|---|\n| Plan 19-08: \
             `tests/async_blocking_guard.rs` | advisory |\n| Another blocking-sounding note | \
             advisory |\n",
        )
        .unwrap();
        assert!(
            !infer_disk_status(dir.path()).continue_here_blocking,
            "the gate is a blocking-severity ROW, never the file's existence and \
             never the word appearing in prose. A substring match parks every run \
             against a project carrying a stale advisory marker forever"
        );
    }

    #[test]
    fn test_absent_or_unreadable_continue_here_does_not_set_the_phase_gate() {
        let empty = tempdir().unwrap();
        assert!(!infer_disk_status(empty.path()).continue_here_blocking);

        // A directory where the marker should be: read_to_string fails.
        let unreadable = tempdir().unwrap();
        fs::create_dir(unreadable.path().join(".continue-here.md")).unwrap();
        assert!(
            !infer_disk_status(unreadable.path()).continue_here_blocking,
            "an unreadable marker is 'no gate observed', never an error and never \
             a panic"
        );
    }

    #[test]
    fn test_blocking_row_in_a_table_without_a_severity_column_is_not_a_gate() {
        let dir = tempdir().unwrap();
        // "blocking" sits in a Description column of a table that has no
        // Severity header at all. Comparing by column index is what stops it.
        fs::write(
            dir.path().join(".continue-here.md"),
            "| Item | Description |\n|---|---|\n| One | blocking |\n",
        )
        .unwrap();
        assert!(!infer_disk_status(dir.path()).continue_here_blocking);
    }

    #[test]
    fn test_phase_19_stale_continue_here_marker_is_not_blocking() {
        // The concrete regression the stale-marker note describes, read from
        // this repository's own planning directory rather than a fixture.
        let planning = Path::new(env!("CARGO_MANIFEST_DIR")).join(".planning");
        let Some(phase_19) = find_phase_dir(&planning, "19") else {
            // Archived away by a future milestone: the fixtures above still
            // carry the property, so there is nothing left to regress here.
            return;
        };
        if !phase_19.join(".continue-here.md").exists() {
            return;
        }
        assert!(
            !infer_disk_status(&phase_19).continue_here_blocking,
            "phase 19's marker is left over from a completed phase and every one \
             of its severity rows reads `advisory`. If this fires, the gate has \
             been simplified back into an existence check or a substring search, \
             and every run against this project parks forever"
        );
    }

    #[test]
    fn test_verification_status_round_trips_through_its_identifier() {
        for status in [
            VerificationStatus::Missing,
            VerificationStatus::Passed,
            VerificationStatus::GapsFound,
            VerificationStatus::HumanNeeded,
            VerificationStatus::Stale,
        ] {
            assert_eq!(
                VerificationStatus::from_raw(status.as_str()),
                status,
                "as_str and from_raw must name the same value, or a park record \
                 and the state that produced it disagree"
            );
        }
        assert!(VerificationStatus::Passed.is_passed());
        assert!(!VerificationStatus::HumanNeeded.is_passed());
        assert!(!VerificationStatus::Unknown("passed_ish".to_string()).is_passed());
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
        assert_eq!(result.status, DiskStatus::Executed);
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
        assert_eq!(result.status, DiskStatus::Executed);
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
    fn test_normal_two_plan_two_summary_is_executed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        fs::write(dir.path().join("05-02-SUMMARY.md"), "summary2").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 2);
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_standalone_summary_requires_standalone_plan() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("PLAN.md"), "plan").unwrap();
        fs::write(dir.path().join("SUMMARY.md"), "summary").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(result.summary_count, 1);
        assert_eq!(result.status, DiskStatus::Executed);
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

    // ── Task 3: COVERAGE / WINDOWS / deferred-items / SKELETON artifacts ──

    #[test]
    fn test_coverage_md_detected() {
        let bare = tempdir().unwrap();
        fs::write(bare.path().join("COVERAGE.md"), "coverage").unwrap();
        assert!(infer_disk_status(bare.path()).has_coverage);

        let prefixed = tempdir().unwrap();
        fs::write(prefixed.path().join("05-COVERAGE.md"), "coverage").unwrap();
        assert!(infer_disk_status(prefixed.path()).has_coverage);
    }

    #[test]
    fn test_windows_md_detected() {
        let bare = tempdir().unwrap();
        fs::write(bare.path().join("WINDOWS.md"), "windows").unwrap();
        assert!(infer_disk_status(bare.path()).has_windows);

        let prefixed = tempdir().unwrap();
        fs::write(prefixed.path().join("05-WINDOWS.md"), "windows").unwrap();
        assert!(infer_disk_status(prefixed.path()).has_windows);
    }

    #[test]
    fn test_deferred_items_md_detected() {
        let bare = tempdir().unwrap();
        fs::write(bare.path().join("deferred-items.md"), "deferred").unwrap();
        assert!(infer_disk_status(bare.path()).has_deferred_items);

        let prefixed = tempdir().unwrap();
        fs::write(prefixed.path().join("05-deferred-items.md"), "deferred").unwrap();
        assert!(infer_disk_status(prefixed.path()).has_deferred_items);
    }

    #[test]
    fn test_skeleton_md_detected() {
        let bare = tempdir().unwrap();
        fs::write(bare.path().join("SKELETON.md"), "skeleton").unwrap();
        assert!(infer_disk_status(bare.path()).has_skeleton);

        let prefixed = tempdir().unwrap();
        fs::write(prefixed.path().join("05-01-SKELETON.md"), "skeleton").unwrap();
        assert!(infer_disk_status(prefixed.path()).has_skeleton);
    }

    #[test]
    fn test_new_artifacts_do_not_affect_counts_or_status() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("COVERAGE.md"), "coverage").unwrap();
        fs::write(dir.path().join("WINDOWS.md"), "windows").unwrap();
        fs::write(dir.path().join("deferred-items.md"), "deferred").unwrap();
        fs::write(dir.path().join("SKELETON.md"), "skeleton").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_coverage);
        assert!(result.has_windows);
        assert!(result.has_deferred_items);
        assert!(result.has_skeleton);
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
        assert_eq!(
            result.status,
            DiskStatus::Empty,
            "informational artifacts must not change status"
        );
    }

    #[test]
    fn test_per_plan_security_still_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-SECURITY.md"), "security").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(
            result.has_security,
            "05-01-SECURITY.md must still set has_security"
        );
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
    }

    #[test]
    fn test_empty_dir_has_no_new_artifacts() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert!(!result.has_coverage);
        assert!(!result.has_windows);
        assert!(!result.has_deferred_items);
        assert!(!result.has_skeleton);
    }
}
