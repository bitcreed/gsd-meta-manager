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

mod context;
mod debug;
mod deferred;
mod frontmatter;
mod handoff;
mod markdown;
mod rules;
mod scan;
mod uat;
mod verification;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};
use std::time::SystemTime;

use chrono::{DateTime, Utc};

use super::ProjectState;

/// What kind of document an [`AuditFile`] was read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// What parsing one document produced: everything of an [`AuditFile`] that depends on
/// the text alone, not on where the file lives.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParsedFile {
    pub status: String,
    pub source: Vec<String>,
    pub updated: Option<DateTime<Utc>>,
    pub acknowledged: bool,
    pub unparsed_blocks: usize,
    pub unreadable: Option<Unreadable>,
    pub items: Vec<AuditItem>,
}

/// What identifies one version of a file's content: its modification time and length,
/// read from the handle the text was (or would be) read from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CacheKey {
    pub mtime: Option<SystemTime>,
    pub len: u64,
}

/// A parse result as the cache holds it: `None` for a document that contributes
/// nothing (a passed VERIFICATION), which is worth remembering too.
pub(crate) type Parsed = Option<ParsedFile>;

type CacheMap<K, V> = HashMap<(PathBuf, K), (CacheKey, Arc<V>)>;

/// The most entries the process-wide cache holds before it starts over.
const CACHE_MAX_ENTRIES: usize = 8192;

#[cfg(test)]
thread_local! {
    /// Cache hits seen by this thread, so a test can assert that a second scan read
    /// nothing from disk.
    pub(crate) static CACHE_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Parsed documents by canonical path, valid while the file's [`CacheKey`] is
/// unchanged (D-16, OWN-12).
///
/// The lock is held for one map operation and never across file I/O or parsing.
/// A poisoned lock turns lookups and inserts into no-ops, so the scan reads the
/// file instead; it is never a panic.
pub(crate) struct ParseCache<K, V> {
    map: Mutex<CacheMap<K, V>>,
    max_entries: usize,
}

impl<K: Copy + Eq + std::hash::Hash, V> ParseCache<K, V> {
    pub(crate) fn new(max_entries: usize) -> Self {
        ParseCache {
            map: Mutex::new(HashMap::new()),
            max_entries,
        }
    }

    /// The cached parse of `kind` at `path` when its key still equals `key`.
    pub(crate) fn get(&self, path: &Path, kind: K, key: CacheKey) -> Option<Arc<V>> {
        let map = self.map.lock().ok()?;
        let (stored, parsed) = map.get(&(path.to_path_buf(), kind))?;
        let hit = (*stored == key).then(|| Arc::clone(parsed));
        #[cfg(test)]
        if hit.is_some() {
            CACHE_HITS.with(|hits| hits.set(hits.get() + 1));
        }
        hit
    }

    /// Remember a parse. An insert into a full map clears it first.
    pub(crate) fn insert(&self, path: PathBuf, kind: K, key: CacheKey, parsed: Arc<V>) {
        let Ok(mut map) = self.map.lock() else {
            return;
        };
        let entry = (path, kind);
        if map.len() >= self.max_entries && !map.contains_key(&entry) {
            map.clear();
        }
        map.insert(entry, (key, parsed));
    }

    /// Drop every entry whose path is inside `root`.
    pub(crate) fn evict_under(&self, root: &Path) {
        let mut map = self.map.lock().unwrap_or_else(PoisonError::into_inner);
        map.retain(|(path, _), _| !path.starts_with(root));
    }

    /// Poison the lock the way a panicking holder would.
    #[cfg(test)]
    pub(crate) fn hold_lock_and_panic(&self) {
        let _held = self.map.lock().ok();
        panic!("poisoning the parse cache lock on purpose");
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.map
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

static PARSE_CACHE: LazyLock<ParseCache<AuditKind, Parsed>> =
    LazyLock::new(|| ParseCache::new(CACHE_MAX_ENTRIES));

/// Which open-signal reader a cache entry belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OpenKind {
    Debug,
    Context,
}

/// Parsed CONTEXT documents by canonical path (same rules as [`OPEN_CACHE`]; a second
/// map because the parsed value differs).
pub(crate) static CONTEXT_CACHE: LazyLock<ParseCache<OpenKind, context::ParsedContext>> =
    LazyLock::new(|| ParseCache::new(CACHE_MAX_ENTRIES));

/// Parsed debug sessions (and, later, context files) by canonical path, with the same
/// validity rule and lock discipline as [`PARSE_CACHE`].
pub(crate) static OPEN_CACHE: LazyLock<ParseCache<OpenKind, debug::ParsedSession>> =
    LazyLock::new(|| ParseCache::new(CACHE_MAX_ENTRIES));

/// Forget everything the parse cache holds about the project at `root` (D-16).
///
/// Called when the watcher reports a change under the project. It is a map removal:
/// nothing is read or parsed, and a path that was never cached costs nothing.
pub fn evict_project(root: &Path) {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    PARSE_CACHE.evict_under(&root);
    OPEN_CACHE.evict_under(&root);
    CONTEXT_CACHE.evict_under(&root);
}

/// Layer A: every audit file of the project, process-free, sorted by `rel_path`.
pub fn scan(planning_dir: &Path, state: &ProjectState) -> Vec<AuditFile> {
    scan::scan_planning(&PARSE_CACHE, planning_dir, state)
}

/// One debug session as Layer A reads it (`.planning/debug/<slug>.md`).
#[derive(Debug, Clone, PartialEq)]
pub struct DebugSession {
    /// POSIX path relative to the project root.
    pub rel_path: String,
    /// The file name without `.md`.
    pub slug: String,
    /// Lower-cased frontmatter `status`, not trimmed; `unknown` when absent.
    pub status: String,
    pub trigger: Option<String>,
    /// First line of the `## Current Focus` section.
    pub hypothesis: Option<String>,
    pub updated: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub acknowledged: bool,
    pub unreadable: Option<Unreadable>,
    /// The frontmatter fence is closed but its YAML was rejected (Layer B input).
    pub fm_unparseable: bool,
    /// A column-0 `status:` line recovered from such a fence (Layer B input, never
    /// compared with the oracle).
    pub recovered_status: Option<String>,
}

/// One CONTEXT document with open questions as Layer A reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextFile {
    /// POSIX path relative to the project root.
    pub rel_path: String,
    /// `Current` for an active phase directory (of any milestone), else the archive.
    pub scope: Scope,
    /// The workstream directory name, `None` for the project's own tree.
    pub workstream: Option<String>,
    /// The phase directory's leading token.
    pub phase: String,
    pub phase_dir: String,
    /// Whether the phase belongs to the current milestone of its own tree (false for
    /// an archived file).
    pub in_current_milestone: bool,
    /// The complete, sanitized question list (the digest input).
    pub questions: Vec<String>,
    /// Lower-cased frontmatter `status`, `unknown` when absent.
    pub status: String,
    pub updated: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub acknowledged: bool,
    pub unreadable: Option<Unreadable>,
}

/// Layer A for the open signals: what `audit-open` reports besides UAT files.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OpenScan {
    pub debug_sessions: Vec<DebugSession>,
    pub context_files: Vec<ContextFile>,
}

/// Layer A: the open-signal sources of the project, process-free.
pub fn scan_open(planning_dir: &Path, state: &ProjectState) -> OpenScan {
    OpenScan {
        debug_sessions: debug::scan_debug_sessions(planning_dir),
        context_files: context::scan_context_questions(planning_dir, state),
    }
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
    Debug,
    Handoff,
    Checkpoint,
    ContextQuestions,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Uat => "uat",
            Kind::Verification => "verification",
            Kind::Deferred => "deferred",
            Kind::Debug => "debug",
            Kind::Handoff => "handoff",
            Kind::Checkpoint => "checkpoint",
            Kind::ContextQuestions => "context_questions",
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
    pub decide: usize,
}

/// The owner's view of one project.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OwnerDigest {
    pub groups: Vec<OwnerGroup>,
    pub test: usize,
    /// Open decisions: debug sessions awaiting confirmation, handoffs, checkpoints and
    /// open CONTEXT questions.
    pub decide: usize,
    pub fix_phases: usize,
    /// Open debug investigations (root-scoped, never counted as phases).
    pub fix_debug: usize,
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
    let pause = rules::PauseScan {
        handoffs: handoff::read_handoffs(planning_dir, state),
        checkpoints: handoff::read_checkpoints(planning_dir, state),
    };
    rules::build_digest_with(
        scan(planning_dir, state),
        scan_open(planning_dir, state),
        pause,
        state,
        now,
        stale_days,
    )
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
    fn kind_tokens_are_the_schema_tokens() {
        let all = [
            (Kind::Uat, "uat"),
            (Kind::Verification, "verification"),
            (Kind::Deferred, "deferred"),
            (Kind::Debug, "debug"),
            (Kind::Handoff, "handoff"),
            (Kind::Checkpoint, "checkpoint"),
            (Kind::ContextQuestions, "context_questions"),
        ];
        for (kind, token) in all {
            assert_eq!(kind.as_str(), token);
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
