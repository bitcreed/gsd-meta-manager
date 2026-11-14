//! The owner's open items: the checks a human has to run, read from the `.planning/`
//! trees of a GSD project (Phase 34).
//!
//! # Numbered facts
//!
//! 1. **Process-free.** Nothing in this module spawns a process: gsd-core, gsd-tools
//!    and every other binary are out of reach of the reader. The `audit-uat` oracle
//!    runs only inside `tests/`.
//! 2. **Layer A is a port.** [`scan`] ports gsd-core 1.21.0's `cmdAuditUat`: which
//!    files it reads, how it parses their rows, how it categorizes them.
//!    `tests/owner_audit_conformance.rs` pins the port against the real oracle on
//!    committed fixtures.
//! 3. **Layer B is the owner's rules.** [`read_owner_digest`] turns Layer A's files
//!    into the lanes, groups, ages and commands the inbox shows. Those rules are the
//!    owner's own, unit-tested here and never compared with the oracle.
//! 4. **Fail-safe.** A file that cannot be read, is too large, is not a regular file
//!    or holds rows the parser could not read is reported as unreadable checks, never
//!    as clean.
//! 5. **The caller supplies `now`.** No function here reads the clock, so a digest
//!    over a fixed tree is reproducible (D-14).
//!
//! `parse_project_state` never computes the digest (D-15): it is a separate entry
//! point, so a refresh that does not need the inbox never pays for it.

mod frontmatter;
mod rules;
mod scan;
mod uat;

use std::path::Path;

use chrono::{DateTime, Utc};

use super::ProjectState;

/// What kind of document an [`AuditFile`] was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditKind {
    Uat,
    Verification,
    Deferred,
}

/// Which milestone's phase directories a file was found in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    /// The current milestone's active phase directories.
    Current,
    /// An archived milestone. `rank` is the upstream archive enumeration order.
    Archived { label: String, rank: usize },
}

/// Why a file's rows could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unreadable {
    /// Larger than the 1 MiB read cap.
    TooLarge,
    /// Not a regular file (a FIFO, a device, a directory).
    NotRegular,
    /// A VERIFICATION status outside the closed set (carries the value).
    OutOfSetStatus(String),
    /// Any other I/O failure.
    Io,
}

/// Where an [`AuditItem`] came from inside its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Row,
    Gap,
    GapTable,
    HumanVerification,
    VerificationGap,
    Deferred,
}

/// Upstream's `categorizeItem` tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    ServerBlocked,
    DeviceNeeded,
    BuildNeeded,
    ThirdParty,
    Blocked,
    Deferred,
    SkippedUnresolved,
    Pending,
    HumanUat,
    Issue,
    Unknown,
}

impl Category {
    /// The token upstream's `audit-uat` prints.
    pub fn as_str(&self) -> &'static str {
        match self {
            Category::ServerBlocked => "server_blocked",
            Category::DeviceNeeded => "device_needed",
            Category::BuildNeeded => "build_needed",
            Category::ThirdParty => "third_party",
            Category::Blocked => "blocked",
            Category::Deferred => "deferred",
            Category::SkippedUnresolved => "skipped_unresolved",
            Category::Pending => "pending",
            Category::HumanUat => "human_uat",
            Category::Issue => "issue",
            Category::Unknown => "unknown",
        }
    }
}

/// One open check as upstream reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditItem {
    pub origin: Origin,
    pub test: Option<String>,
    pub name: String,
    pub expected: Option<String>,
    pub result: String,
    pub category: Category,
    pub reason: Option<String>,
    pub blocked_by: Option<String>,
}

/// One document read by Layer A.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditFile {
    /// POSIX path relative to the project root.
    pub rel_path: String,
    pub kind: AuditKind,
    pub scope: Scope,
    /// The workstream directory name, `None` for the project's own tree.
    pub workstream: Option<String>,
    /// The phase directory's leading token.
    pub phase: String,
    pub phase_dir: String,
    /// Lower-cased frontmatter `status`, `unknown` when absent.
    pub status: String,
    /// Basenames the frontmatter `source` names.
    pub source: Vec<String>,
    pub updated: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub acknowledged: bool,
    /// Rows the parser could not turn into items (upstream's `unparsed_blocks`).
    pub unparsed_blocks: usize,
    pub unreadable: Option<Unreadable>,
    pub items: Vec<AuditItem>,
}

/// Layer A: every audit file of the project, process-free, sorted by `rel_path`.
pub fn scan(planning_dir: &Path, state: &ProjectState) -> Vec<AuditFile> {
    scan::scan_planning(planning_dir, state)
}

/// The inbox lane an item or group is listed under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    Test,
    Decide,
    Fix,
    Blocked,
    Deferred,
}

impl Lane {
    pub fn as_str(&self) -> &'static str {
        match self {
            Lane::Test => "test",
            Lane::Decide => "decide",
            Lane::Fix => "fix",
            Lane::Blocked => "blocked",
            Lane::Deferred => "deferred",
        }
    }

    /// The D-20 sort position.
    pub(crate) fn order(&self) -> u8 {
        match self {
            Lane::Test => 0,
            Lane::Decide => 1,
            Lane::Fix => 2,
            Lane::Blocked => 3,
            Lane::Deferred => 4,
        }
    }
}

/// The document kind of a group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Uat,
    Verification,
    Deferred,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Uat => "uat",
            Kind::Verification => "verification",
            Kind::Deferred => "deferred",
        }
    }
}

/// A gap merged into an item (R3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerGap {
    pub name: String,
    pub result: String,
    pub reason: Option<String>,
}

/// One check as the inbox lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnerItem {
    pub id: String,
    pub test: Option<String>,
    pub name: String,
    pub expected: Option<String>,
    pub result: String,
    pub category: Category,
    pub reason: Option<String>,
    pub blocked_by: Option<String>,
    pub gaps: Vec<OwnerGap>,
}

/// The counts a group reports about itself.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GroupCounts {
    /// Listed items.
    pub checks: usize,
    /// Items whose result is issue, failed or partial.
    pub failed: usize,
    /// Merged plus standalone gap items.
    pub gaps: usize,
    /// Unparsed blocks, or 1 for a file that could not be read or classified.
    pub unreadable: usize,
}

/// One (file, lane) pair of the inbox.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnerGroup {
    pub lane: Lane,
    pub kind: Kind,
    pub scope: Scope,
    pub workstream: Option<String>,
    pub phase: String,
    pub phase_name: String,
    pub file: String,
    pub file_status: String,
    pub closes_phase: bool,
    pub bookkeeping_only: bool,
    pub acknowledged: bool,
    pub age_days: Option<i64>,
    pub stale: bool,
    pub command: Option<String>,
    pub supersedes: Vec<String>,
    pub counts: GroupCounts,
    pub items: Vec<OwnerItem>,
}

/// What the lanes do not list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Hidden {
    pub deferred: usize,
    pub acknowledged_files: usize,
    pub noise: usize,
}

/// Item counts of archived groups.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArchivedCounts {
    pub test: usize,
    pub blocked: usize,
    pub fix: usize,
}

/// The owner's view of one project.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OwnerDigest {
    pub groups: Vec<OwnerGroup>,
    pub test: usize,
    pub fix_phases: usize,
    pub blocked: usize,
    pub closeable_phases: usize,
    pub archived: ArchivedCounts,
    pub superseded: usize,
    pub hidden: Hidden,
}

/// Layer A plus Layer B: the digest of one project's `.planning/` tree.
///
/// Never called from `parse_project_state` (D-15). `now` and `stale_days` come from
/// the caller (D-13, D-14).
pub fn read_owner_digest(
    planning_dir: &Path,
    state: &ProjectState,
    now: DateTime<Utc>,
    stale_days: u32,
) -> OwnerDigest {
    rules::build_digest(scan(planning_dir, state), state, now, stale_days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_tokens_are_upstreams() {
        let all = [
            (Category::ServerBlocked, "server_blocked"),
            (Category::DeviceNeeded, "device_needed"),
            (Category::BuildNeeded, "build_needed"),
            (Category::ThirdParty, "third_party"),
            (Category::Blocked, "blocked"),
            (Category::Deferred, "deferred"),
            (Category::SkippedUnresolved, "skipped_unresolved"),
            (Category::Pending, "pending"),
            (Category::HumanUat, "human_uat"),
            (Category::Issue, "issue"),
            (Category::Unknown, "unknown"),
        ];
        for (category, token) in all {
            assert_eq!(category.as_str(), token);
        }
    }

    #[test]
    fn lane_order_matches_the_d20_sort() {
        let mut lanes = [
            Lane::Deferred,
            Lane::Blocked,
            Lane::Fix,
            Lane::Decide,
            Lane::Test,
        ];
        lanes.sort_by_key(Lane::order);
        let names: Vec<&str> = lanes.iter().map(Lane::as_str).collect();
        assert_eq!(names, ["test", "decide", "fix", "blocked", "deferred"]);
    }
}
