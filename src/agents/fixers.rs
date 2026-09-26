//! The code-review fix-run estimate (AGENT-07, D-C12): while parallel
//! `gsd-code-fixer` agents work through a phase's `NN-REVIEW.md`, roughly how
//! many of its findings are fixed out of the total.
//!
//! **An estimate, never a count of record.** The numerator is the number of
//! distinct finding ids (`CR-`, `WR-`, `IN-` plus digits) named in `fix(NN): …`
//! commit subjects — a fixer that fixes two findings in one commit naming only
//! one, or names an id it did not fix, moves it. Every rendering therefore
//! carries `~`, and the widest one keeps the word `fixed`
//! ([`super::waves::AgentView::summary_forms`]).
//!
//! What it reads, and nothing else:
//!
//! * one `*-REVIEW.md`'s leading frontmatter, through the existing nested
//!   reader [`disk_status::leading_frontmatter_nested_value`] (no second YAML
//!   reader, D-C11), capped at [`REVIEW_READ_CAP`] bytes;
//! * the entry NAMES of that phase's directory in the main worktree;
//! * commit subjects, through [`git_ops::log_subjects`] only — whose range is
//!   `HEAD` or `<hex>..HEAD` and is refused otherwise (D-B02).
//!
//! It runs only while an active fixer exists, and it writes nothing (D-B01).
//! Agent-authored text (a description, a commit subject) only ever selects a
//! phase number or a finding id by regex; it never reaches argv or a path
//! other than through `find_phase_dir` over main's own listing.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;

use super::{AgentLiveness, AgentRow};
use crate::state_reader::phase_num::{same_phase, PhaseNum};
use crate::state_reader::{disk_status, git_ops};
use crate::text::Untrusted;

/// The runtime agent type a code-fix run's agents report.
pub const FIXER_AGENT_TYPE: &str = "gsd-code-fixer";

/// At most this many of each fixer worktree's own commit subjects are read.
const FIXER_SUBJECTS: u32 = 200;

/// At most this many of the main worktree's commit subjects are read.
const MAIN_SUBJECTS: u32 = 300;

/// At most this many bytes of a `*-REVIEW.md` are read (T-25-27). The leading
/// frontmatter is at the top, so a truncated tail loses nothing that counts.
pub const REVIEW_READ_CAP: u64 = 256 * 1024;

/// One project's fix-run estimate, as of one scan.
///
/// `fixed` and `total` are shown only when BOTH are `Some`; either is `None`
/// when no phase was determined, the phase has no readable `findings.total`,
/// or its `*-REVIEW-FIX.md` already exists (the run is over).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FixerEstimate {
    /// Active (`Live`, `Idle` or `Finished`) unattributed fixer agents.
    pub fixers: u32,
    /// The phase whose REVIEW.md the fixers are working through.
    pub phase: Option<PhaseNum>,
    /// Distinct finding ids named in that phase's `fix(NN…)` subjects.
    pub fixed: Option<u32>,
    /// `findings.total` from the phase's REVIEW.md frontmatter, verbatim.
    pub total: Option<u32>,
}

/// `fix(NN): …` or `fix(NN-anything): …`, capturing the phase.
fn fix_subject_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^fix\((\d+(?:\.\d+)?)(?:-[^)]*)?\):").expect("static regex"))
}

/// A GSD review finding id: `CR-01`, `WR-8`, `IN-003`.
fn finding_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\b(?:CR|WR|IN)-\d+\b").expect("static regex"))
}

/// `phase 12`, `Phase 07.1`, `FIX PHASE 12` in a fixer's description.
fn description_phase_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\bphase (\d+(?:\.\d+)?)\b").expect("static regex"))
}

/// The distinct finding ids named in `subjects` that are `fix` commits of
/// `phase`.
///
/// Only a subject opening `fix(NN…):` whose `NN` is the same phase
/// (pad-insensitively: `05` and `5` are one) counts; ids are collected from the
/// rest of the subject, verbatim. `feat(12): WR-03`, `fix(13): WR-02` and
/// `fix(12): XX-01` add nothing for phase 12.
pub fn finding_ids(subjects: &[String], phase: &PhaseNum) -> BTreeSet<String> {
    let target = phase.to_string();
    let mut ids = BTreeSet::new();
    for subject in subjects {
        let Some(caps) = fix_subject_re().captures(subject) else {
            continue;
        };
        if !same_phase(&caps[1], &target) {
            continue;
        }
        let rest = &subject[caps.get(0).map_or(0, |m| m.end())..];
        ids.extend(finding_id_re().find_iter(rest).map(|m| m.as_str().to_string()));
    }
    ids
}

/// The phase one fixer works on: `phase NN` in its description
/// (case-insensitive), else the scope phase of the first `fix(NN…)` subject
/// among its own commits, newest first. `None` when neither names one — the
/// fixer then shows `N fixers` without a count until its first commit
/// [inferred — replaces RESEARCH's "else the active phase", which the blocking
/// scan cannot see].
pub fn fixer_phase(description: Option<&str>, own_subjects: &[String]) -> Option<PhaseNum> {
    description
        .and_then(|d| description_phase_re().captures(d))
        .and_then(|caps| PhaseNum::parse(&caps[1]))
        .or_else(|| {
            own_subjects.iter().find_map(|s| {
                fix_subject_re()
                    .captures(s)
                    .and_then(|caps| PhaseNum::parse(&caps[1]))
            })
        })
}

/// Whether `row` is an active, unattributed code fixer.
fn is_active_fixer(row: &AgentRow) -> bool {
    row.plan.is_none()
        && matches!(
            row.liveness,
            AgentLiveness::Live | AgentLiveness::Idle | AgentLiveness::Finished
        )
        && row
            .agent_type
            .as_ref()
            .is_some_and(|t| t.as_raw_for_logic_only() == FIXER_AGENT_TYPE)
}

/// What the phase directory says about the review: whether its fix report
/// exists, and which REVIEW.md to read. Entry names only.
fn review_files(phase_dir: &Path) -> (bool, Option<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(phase_dir) else {
        return (false, None);
    };
    let mut fix_report = false;
    let mut reviews: Vec<PathBuf> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.ends_with("-REVIEW-FIX.md") {
            fix_report = true;
        } else if name.ends_with("-REVIEW.md") {
            reviews.push(entry.path());
        }
    }
    reviews.sort();
    (fix_report, reviews.into_iter().next())
}

/// `findings.total` of the REVIEW.md at `path`, read through a
/// [`REVIEW_READ_CAP`]-byte cap. A trailing ` # comment` and surrounding quotes
/// are dropped; anything that is not a `u32` is `None`.
fn review_total(path: &Path) -> Option<u32> {
    let file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    file.take(REVIEW_READ_CAP).read_to_end(&mut bytes).ok()?;
    let content = String::from_utf8_lossy(&bytes);
    let value = disk_status::leading_frontmatter_nested_value(&content, "findings", "total")?;
    let value = value.split(" #").next().unwrap_or_default().trim();
    value.trim_matches(['"', '\'']).parse::<u32>().ok()
}

/// The fix-run estimate for one project's scanned rows, or `None` when no
/// active unattributed `gsd-code-fixer` row exists — in which case no git
/// call and no file read is made at all.
///
/// 1. The active fixers are rows whose raw agent type is `gsd-code-fixer`,
///    whose `plan` is `None`, and whose liveness is `Live`, `Idle` or
///    `Finished`.
/// 2. Each one's own subjects are `base_sha..HEAD` (at most 200), read only
///    when `base_sha` is known and the worktree is not prunable.
/// 3. The phase is the one most fixers yield ([`fixer_phase`]), ties to the
///    higher phase; with none, only the fixer count is returned.
/// 4. When the phase directory in the main worktree holds a
///    `*-REVIEW-FIX.md`, the run is over: no counts.
/// 5. Otherwise `total` is `findings.total` from its `*-REVIEW.md`.
/// 6. `fixed` is the size of the union of [`finding_ids`] over every fixer's
///    subjects plus the main worktree's last 300 — an id fixed twice, on two
///    worktrees or on a worktree and main, counts once.
/// 7. `fixed` and `total` are `Some` only when the total parsed.
///
/// **Assumption A7 (flagged for audit, not changed).** The denominator is
/// `findings.total` verbatim (D-C12). It includes the `info` findings fixers
/// often leave alone, so the estimate can read low — ttbook's run fixed 30 of
/// 48 — where `critical + warning` might read truer.
pub fn estimate(
    project_root: &Path,
    main_worktree: Option<&Path>,
    base_sha: Option<&str>,
    rows: &[AgentRow],
) -> Option<FixerEstimate> {
    let fixers: Vec<&AgentRow> = rows.iter().filter(|r| is_active_fixer(r)).collect();
    if fixers.is_empty() {
        return None;
    }
    let mut estimate = FixerEstimate {
        fixers: u32::try_from(fixers.len()).unwrap_or(u32::MAX),
        ..FixerEstimate::default()
    };

    let own_subjects: Vec<Vec<String>> = fixers
        .iter()
        .map(|row| match base_sha {
            Some(base) if !row.prunable => {
                git_ops::log_subjects(&row.path, &format!("{base}..HEAD"), FIXER_SUBJECTS)
            }
            _ => Vec::new(),
        })
        .collect();

    let mut votes: BTreeMap<PhaseNum, u32> = BTreeMap::new();
    for (row, subjects) in fixers.iter().zip(&own_subjects) {
        let description = row.description.as_ref().map(Untrusted::as_raw_for_logic_only);
        if let Some(phase) = fixer_phase(description, subjects) {
            *votes.entry(phase).or_default() += 1;
        }
    }
    // BTreeMap iterates ascending and `max_by_key` keeps the LAST maximum, so a
    // tie goes to the higher phase.
    let Some(phase) = votes
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(phase, _)| phase.clone())
    else {
        return Some(estimate);
    };
    estimate.phase = Some(phase.clone());

    let Some(phase_dir) =
        disk_status::find_phase_dir(&project_root.join(".planning"), &phase.padded())
    else {
        return Some(estimate);
    };
    let (fix_report, review) = review_files(&phase_dir);
    if fix_report {
        return Some(estimate);
    }
    let Some(total) = review.as_deref().and_then(review_total) else {
        return Some(estimate);
    };

    let mut ids: BTreeSet<String> = BTreeSet::new();
    for subjects in &own_subjects {
        ids.extend(finding_ids(subjects, &phase));
    }
    if let Some(main) = main_worktree {
        ids.extend(finding_ids(
            &git_ops::log_subjects(main, "HEAD", MAIN_SUBJECTS),
            &phase,
        ));
    }
    estimate.fixed = Some(u32::try_from(ids.len()).unwrap_or(u32::MAX));
    estimate.total = Some(total);
    Some(estimate)
}
