//! The Claude Code enrichment adapter (D-A04): what Claude Code's own session
//! files say about the agents running in a project's worktrees.
//!
//! **The format is undocumented.** Everything here was measured against Claude
//! Code 2.1.283 (25-RESEARCH.md § Pattern 4), and any release may change it. So
//! every read is tolerant, and every failure is "no data for this row this
//! scan" (D-A03): a missing config root, a missing project directory, a meta
//! file that is garbage, truncated mid-rewrite, oversized or of an unexpected
//! shape each cost only the row or field concerned, and the row falls back to
//! what the git core already knows.
//!
//! **What this module reads — exactly two kinds of thing:**
//!
//! * the contents of `*.meta.json` files, through [`read_meta_capped`] only,
//!   capped at [`META_READ_CAP`] bytes, and refused for any other file name;
//! * file and directory metadata (existence, type, mtime).
//!
//! **It never reads a transcript's contents** (D-C09). A subagent's
//! `agent-<id>.jsonl` holds the user's conversation; liveness needs its mtime
//! and nothing more, so the transcript is statted and never opened.
//!
//! **It never parses the lock-reason pid** (D-A05). Claude Code writes
//! `claude agent agent-<id> (pid N start T)` as the worktree lock reason, but
//! that pid is the top-level session every agent of that session shares: it is
//! alive for as long as the user's terminal is, whatever the agent is doing.
//! The only lock fact reported is whether the worktree is locked at all.
//! (Quick 260926-06g: the core's `processes` module does read that pid, but
//! solely as a DEATH signal — a gone owner means `Ended` — never as liveness,
//! so a live owner never upgrades a row and D-A05 holds. This adapter still
//! parses nothing of it.)
//!
//! **Where the files are.** The config root is `$CLAUDE_CONFIG_DIR` when set,
//! non-empty and absolute, else `<home>/.claude` ([`resolve_config_root`]).
//! Sessions live under `<root>/projects/<encoded project path>/<session>/`, with
//! every subagent of that session — at any spawn depth — flat in its
//! `subagents/` directory as `agent-<id>.meta.json` beside `agent-<id>.jsonl`.
//! The directory name is Claude Code's lossy encoding of the project path
//! ([`encode_project_dir`]); it is only ever computed forwards, never decoded.
//!
//! **Only the user's own config root is read**, never a `.claude/` directory
//! inside a project: a cloned repository can ship any files it likes there.

use std::collections::HashSet;
use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::{AdapterReport, AgentAdapter, ChildAgent, CoreSnapshot, Enrichment};
use crate::agents::worktrees::{valid_agent_id, CoreWorktree};
use crate::agents::{LIVE_SECS, MAX_AGENT_AGE_SECS};
use crate::text::Untrusted;

/// The most bytes of one `*.meta.json` that are read. Measured metas are a few
/// hundred bytes; anything past this is not a meta this adapter understands,
/// and is no data rather than a large allocation (T-25-09).
pub const META_READ_CAP: u64 = 64 * 1024;

/// Claude Code's cap on an encoded project-directory name, in UTF-16 units
/// (`Kne` in the 2.1.283 binary). Longer encodings are cut here and suffixed
/// with a hash.
const ENCODED_NAME_LIMIT: usize = 200;

/// The file-name suffix [`read_meta_capped`] insists on.
const META_SUFFIX: &str = ".meta.json";

/// Enriches agent worktrees from Claude Code's session metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeCodeAdapter {
    /// The Claude config root; `None` when neither the environment nor a home
    /// directory names one, in which case every scan reports nothing.
    config_root: Option<PathBuf>,
}

impl ClaudeCodeAdapter {
    /// An adapter rooted at `config_root`. Tests use this, with a tempdir, so
    /// no test ever reads the real home directory (D-C17).
    pub fn new(config_root: PathBuf) -> Self {
        Self {
            config_root: Some(config_root),
        }
    }

    /// An adapter rooted where Claude Code itself looks: `$CLAUDE_CONFIG_DIR`,
    /// else `<home>/.claude`. Resolves a path only; reads no file.
    pub fn from_env() -> Self {
        Self {
            config_root: resolve_config_root(
                std::env::var_os("CLAUDE_CONFIG_DIR"),
                dirs::home_dir(),
            ),
        }
    }
}

/// The Claude config root (D-B03, D-C05).
///
/// `env_value` — the value of `CLAUDE_CONFIG_DIR` — wins when it is non-empty
/// and absolute. Claude Code itself refuses a relative one ("the configuration
/// home (CLAUDE_CONFIG_DIR) is not an absolute path"), so a relative or empty
/// value is ignored rather than resolved against whatever this process's cwd
/// happens to be (T-25-12). Otherwise `<home>/.claude`; `None` when there is no
/// home either.
pub fn resolve_config_root(env_value: Option<OsString>, home: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(value) = env_value {
        let path = PathBuf::from(value);
        if !path.as_os_str().is_empty() && path.is_absolute() {
            return Some(path);
        }
    }
    home.map(|home| home.join(".claude"))
}

/// Claude Code's directory name for a project path (D-C05), byte for byte.
///
/// The algorithm extracted from the 2.1.283 binary (25-RESEARCH.md § Pattern 4)
/// works on JavaScript strings, so on UTF-16 code units: every unit that is not
/// ASCII alphanumeric becomes `-`, which means a character outside the Basic
/// Multilingual Plane (two units) becomes TWO dashes. An encoding longer than
/// 200 units is cut to 200, then `-` and the base-36 absolute value of a 32-bit
/// string hash of the ORIGINAL path are appended. The result is pure ASCII, so
/// its byte length equals its UTF-16 length.
///
/// The encoding is lossy; nothing here ever decodes it.
pub fn encode_project_dir(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            out.extend(std::iter::repeat_n('-', c.len_utf16()));
        }
    }
    if out.len() <= ENCODED_NAME_LIMIT {
        return out;
    }
    let mut h: i32 = 0;
    for unit in path.encode_utf16() {
        h = (h << 5).wrapping_sub(h).wrapping_add(i32::from(unit));
    }
    // `Math.abs` in i64, so `i32::MIN` maps to 2147483648 rather than overflowing.
    let hash = i64::from(h).unsigned_abs();
    format!("{}-{}", &out[..ENCODED_NAME_LIMIT], to_base36(hash))
}

/// `n` in lowercase base 36, as JavaScript's `Number.prototype.toString(36)`.
fn to_base36(mut n: u64) -> String {
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    if n == 0 {
        return "0".to_string();
    }
    let mut reversed = Vec::new();
    while n > 0 {
        reversed.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    reversed.iter().rev().map(|&b| char::from(b)).collect()
}

/// The fields of one `agent-<id>.meta.json` this adapter uses.
///
/// Built field by field from a `serde_json::Value`, never derived: a field of an
/// unexpected type loses only itself, and unknown fields — `spawnDepth`,
/// `requestShape`, whatever a later release adds — are simply never looked at.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Meta {
    agent_type: Option<String>,
    description: Option<String>,
    worktree_path: Option<PathBuf>,
    inherited_worktree_path: Option<PathBuf>,
    parent_agent_id: Option<String>,
    worktree_cleanly_removed: Option<bool>,
    stopped_by_user: Option<bool>,
}

impl Meta {
    /// Build from parsed JSON; `None` unless the top level is an object.
    fn from_value(value: &serde_json::Value) -> Option<Self> {
        let object = value.as_object()?;
        let text = |key: &str| object.get(key)?.as_str().map(str::to_string);
        let path = |key: &str| {
            object
                .get(key)?
                .as_str()
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
        };
        let flag = |key: &str| object.get(key)?.as_bool();
        Some(Self {
            agent_type: text("agentType"),
            description: text("description"),
            worktree_path: path("worktreePath"),
            inherited_worktree_path: path("inheritedWorktreePath"),
            parent_agent_id: text("parentAgentId"),
            worktree_cleanly_removed: flag("worktreeCleanlyRemoved"),
            stopped_by_user: flag("stoppedByUser"),
        })
    }

    /// Whether Claude Code recorded the agent as over (D-C08).
    fn ended(&self) -> bool {
        self.worktree_cleanly_removed == Some(true) || self.stopped_by_user == Some(true)
    }
}

/// Read one meta file — the ONLY content read in this module.
///
/// `None` (no data this scan) unless: the file name ends in `.meta.json` (so a
/// transcript can never be opened through here, whatever it contains —
/// T-25-08); the path is a regular file and not a symlink (a FIFO would block
/// the scan, a symlink could point anywhere); at most [`META_READ_CAP`] bytes
/// are present; and those bytes parse as a JSON object. A meta that Claude Code
/// is rewriting at this instant typically fails the parse, and the next scan
/// reads it again.
fn read_meta_capped(path: &Path) -> Option<Meta> {
    let name = path.file_name()?.to_str()?;
    if !name.ends_with(META_SUFFIX) {
        return None;
    }
    if !std::fs::symlink_metadata(path).ok()?.file_type().is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(META_READ_CAP + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > META_READ_CAP {
        return None;
    }
    let value = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;
    Meta::from_value(&value)
}

/// A file's mtime, or `None` when it cannot be statted. Metadata only.
fn mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// Whether two spellings name the same path: equal as given, or equal once
/// both are canonicalized (a symlinked parent, `/home` vs `/var/home` —
/// RESEARCH Pitfall 7). A path that cannot be canonicalized matches only raw.
fn same_path(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// The names of the entries of `dir`, or none when it cannot be listed.
fn entry_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect()
}

/// This project's directories under `<root>/projects/`, deduplicated.
///
/// Candidates, in order: the registered path, its canonical form, and the main
/// worktree path git reported — Claude Code files sessions under the canonical
/// working-copy root, which the registered spelling may not be. Each is kept
/// when its encoded directory exists. When an encoding exceeded 200 units and
/// its exact directory is absent (a hash drift, or a path spelled differently
/// than when Claude hashed it), `<root>/projects/` is listed once and the
/// single entry starting with the 200-unit prefix plus `-` is accepted; zero or
/// several such entries mean no directory.
fn project_dirs(root: &Path, snap: &CoreSnapshot<'_>) -> Vec<PathBuf> {
    let projects = root.join("projects");
    let mut encodings: Vec<String> = Vec::new();
    let mut push = |path: &Path| {
        let encoded = encode_project_dir(&path.to_string_lossy());
        if !encodings.contains(&encoded) {
            encodings.push(encoded);
        }
    };
    push(snap.project_root);
    if let Ok(canonical) = std::fs::canonicalize(snap.project_root) {
        push(&canonical);
    }
    if let Some(main) = snap.main_worktree {
        push(main);
    }

    let mut listing: Option<Vec<String>> = None;
    let mut dirs: Vec<PathBuf> = Vec::new();
    for encoded in encodings {
        let exact = projects.join(&encoded);
        let found = if exact.is_dir() {
            Some(exact)
        } else if encoded.len() > ENCODED_NAME_LIMIT {
            let prefix = format!("{}-", &encoded[..ENCODED_NAME_LIMIT]);
            let names = listing.get_or_insert_with(|| entry_names(&projects));
            let mut matches = names.iter().filter(|name| name.starts_with(&prefix));
            match (matches.next(), matches.next()) {
                (Some(only), None) => Some(projects.join(only)).filter(|dir| dir.is_dir()),
                _ => None,
            }
        } else {
            None
        };
        if let Some(dir) = found {
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    dirs
}

/// Every session directory of every project directory, sorted so a scan does
/// not depend on directory iteration order. Symlinked entries are skipped.
fn session_dirs(project_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut sessions: Vec<PathBuf> = project_dirs
        .iter()
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flat_map(|entries| entries.flatten())
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
        .map(|entry| entry.path())
        .collect();
    sessions.sort();
    sessions
}

/// The facts one accepted meta contributes to its worktree's row.
fn worktree_enrichment(
    meta: &Meta,
    last_activity: Option<SystemTime>,
    worktree: &CoreWorktree,
) -> Enrichment {
    Enrichment {
        agent_type: meta
            .agent_type
            .clone()
            .map(Untrusted::from_untrusted_source),
        description: meta
            .description
            .clone()
            .map(Untrusted::from_untrusted_source),
        last_activity,
        ended: meta.ended(),
        // Claude Code releases the lock when the agent finishes (RESEARCH
        // Pitfall 4). The reason text — and the pid in it — is never read.
        lock_released: Some(worktree.locked.is_none()),
        children: Vec::new(),
    }
}

/// Whether `meta` may enrich `worktree`: its `worktreePath` names that
/// worktree, or it carries no `worktreePath` at all (D-C06 fallback — the
/// `agent-<id>` file stem already matched, and a meta rewritten after
/// `worktreeCleanlyRemoved` drops the path).
fn meta_joins(meta: &Meta, worktree: &CoreWorktree) -> bool {
    meta.worktree_path
        .as_deref()
        .is_none_or(|path| same_path(path, &worktree.path))
}

/// Pass one: each worktree's own agent, looked up by its id (unbounded — a
/// worktree agent is found however old its session is).
///
/// For each worktree whose `agent_id` passes [`valid_agent_id`] — re-checked
/// here, because the id becomes part of a file name (T-25-10) — every session's
/// `subagents/agent-<id>.meta.json` is statted. Among the sessions that have
/// one, the one whose transcript is newest is taken; its meta is read, and
/// accepted per [`meta_joins`]. Returns the ids that were accepted.
fn enrich_by_id(
    snap: &CoreSnapshot<'_>,
    sessions: &[PathBuf],
    per_worktree: &mut [Option<Enrichment>],
) -> HashSet<String> {
    let mut matched = HashSet::new();
    for (index, worktree) in snap.worktrees.iter().enumerate() {
        let Some(id) = worktree.agent_id.as_deref().filter(|id| valid_agent_id(id)) else {
            continue;
        };
        let mut best: Option<(Option<SystemTime>, PathBuf)> = None;
        for session in sessions {
            let subagents = session.join("subagents");
            let meta_path = subagents.join(format!("agent-{id}{META_SUFFIX}"));
            if !meta_path.is_file() {
                continue;
            }
            let transcript = mtime(&subagents.join(format!("agent-{id}.jsonl")));
            if best.as_ref().is_none_or(|(newest, _)| transcript > *newest) {
                best = Some((transcript, meta_path));
            }
        }
        let Some((last_activity, meta_path)) = best else {
            continue;
        };
        let Some(meta) = read_meta_capped(&meta_path) else {
            continue;
        };
        if !meta_joins(&meta, worktree) {
            continue;
        }
        per_worktree[index] = Some(worktree_enrichment(&meta, last_activity, worktree));
        matched.insert(id.to_string());
    }
    matched
}

/// Seconds from `then` to `now`; a `then` in the future is age 0.
fn age_secs(then: SystemTime, now: SystemTime) -> u64 {
    now.duration_since(then).map(|d| d.as_secs()).unwrap_or(0)
}

/// The index of the snapshot worktree at `path`: a raw match across every
/// worktree first, then a canonical one (RESEARCH Pitfall 7).
fn worktree_at(snap: &CoreSnapshot<'_>, path: &Path) -> Option<usize> {
    snap.worktrees
        .iter()
        .position(|wt| wt.path == path)
        .or_else(|| {
            snap.worktrees
                .iter()
                .position(|wt| same_path(path, &wt.path))
        })
}

/// A meta as a sub-agent.
fn child_agent(meta: &Meta, last_activity: SystemTime) -> ChildAgent {
    ChildAgent {
        agent_type: meta
            .agent_type
            .clone()
            .map(Untrusted::from_untrusted_source),
        description: meta
            .description
            .clone()
            .map(Untrusted::from_untrusted_source),
        last_activity: Some(last_activity),
        ended: meta.ended(),
        ..ChildAgent::default()
    }
}

/// The per-worktree results of both passes. `primary[i]` records that worktree
/// `i` carries its OWN agent's facts, as opposed to an enrichment created only
/// to hold children.
struct Placement<'s, 'a> {
    snap: &'s CoreSnapshot<'a>,
    per_worktree: Vec<Option<Enrichment>>,
    primary: Vec<bool>,
}

impl Placement<'_, '_> {
    /// Attach `child` under worktree `index`, creating an enrichment that
    /// carries only the child when the worktree has none.
    fn attach_child(&mut self, index: usize, child: ChildAgent) {
        self.per_worktree[index]
            .get_or_insert_with(Enrichment::default)
            .children
            .push(child);
    }

    /// Make `meta` worktree `index`'s own agent, keeping any children already
    /// attached there.
    fn set_primary(&mut self, index: usize, meta: &Meta, last_activity: SystemTime) {
        let children = self.per_worktree[index]
            .take()
            .map(|facts| facts.children)
            .unwrap_or_default();
        let mut facts = worktree_enrichment(meta, Some(last_activity), &self.snap.worktrees[index]);
        facts.children = children;
        self.per_worktree[index] = Some(facts);
        self.primary[index] = true;
    }

    /// Place one live meta the id pass did not match; `Some(child)` when it
    /// joins no worktree at all.
    ///
    /// In order: its `worktreePath` names a worktree with no agent of its own →
    /// it becomes that worktree's agent (a worktree whose path carries no id);
    /// names a worktree that already has one → a child there [inferred];
    /// otherwise its `inheritedWorktreePath` names a worktree, or its
    /// `parentAgentId` equals a worktree's agent id → a child there (D-C06).
    fn place(&mut self, meta: &Meta, last_activity: SystemTime) -> Option<ChildAgent> {
        if let Some(index) = meta
            .worktree_path
            .as_deref()
            .and_then(|path| worktree_at(self.snap, path))
        {
            if self.primary[index] {
                self.attach_child(index, child_agent(meta, last_activity));
            } else {
                self.set_primary(index, meta, last_activity);
            }
            return None;
        }
        let parent = meta
            .inherited_worktree_path
            .as_deref()
            .and_then(|path| worktree_at(self.snap, path))
            .or_else(|| {
                let parent_id = meta.parent_agent_id.as_deref()?;
                self.snap
                    .worktrees
                    .iter()
                    .position(|wt| wt.agent_id.as_deref() == Some(parent_id))
            });
        let child = child_agent(meta, last_activity);
        match parent {
            Some(index) => {
                self.attach_child(index, child);
                None
            }
            None => Some(child),
        }
    }
}

/// Pass two: live subagents the id pass did not match — worktrees whose path
/// carries no agent id, nested agents, and agents with no worktree at all.
///
/// Per session, `subagents/` is listed and every `agent-<id>.jsonl` whose id
/// passes [`valid_agent_id`] and was not matched is statted. Only a transcript
/// at most [`LIVE_SECS`] old (a future mtime is age 0) gets its meta read, so
/// the metas of the thousands of finished agents in a long history are never
/// opened. Each id is handled once, in sorted session-then-id order. Returns
/// the live metas that joined no worktree — the worktree-less group (D-C07),
/// `agentType` verbatim, with no `gsd-*` filter.
///
/// **The cost bound (D-C09, amended by RESEARCH Pitfall 2).** D-C09 proposed
/// skipping sessions whose `subagents/` mtime is older than [`LIVE_SECS`]. That
/// mtime is a SPAWN clock, not an activity clock: it moves when an agent is
/// created in the directory, never while one works, so that prefilter would
/// hide every agent running longer than two minutes. Every transcript is
/// statted instead (hundreds in ~10 ms, measured), and the only bound is
/// coarse: a session whose `subagents/` has seen no spawn for
/// [`MAX_AGENT_AGE_SECS`] (a day) is skipped by this pass. The consequence
/// (RESEARCH Assumption A5): an agent running for more than a day in a session
/// with no newer spawn is hidden from the worktree-less group, and is not
/// attached as a child or path-joined either [inferred — the bound covers the
/// whole pass, or it bounds nothing]. A worktree agent's own row is
/// unaffected: the id pass ([`enrich_by_id`]) is not bounded.
fn place_live_subagents(
    placement: &mut Placement<'_, '_>,
    sessions: &[PathBuf],
    matched: &HashSet<String>,
) -> Vec<ChildAgent> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut unjoined = Vec::new();
    for session in sessions {
        let subagents = session.join("subagents");
        let spawned = mtime(&subagents);
        if spawned.is_none_or(|at| age_secs(at, placement.snap.now) > MAX_AGENT_AGE_SECS) {
            continue;
        }
        let mut ids: Vec<String> = entry_names(&subagents)
            .into_iter()
            .filter_map(|name| {
                let id = name.strip_prefix("agent-")?.strip_suffix(".jsonl")?;
                (valid_agent_id(id) && !matched.contains(id)).then(|| id.to_string())
            })
            .collect();
        ids.sort();
        for id in ids {
            if !seen.insert(id.clone()) {
                continue;
            }
            let Some(last_activity) = mtime(&subagents.join(format!("agent-{id}.jsonl"))) else {
                continue;
            };
            if age_secs(last_activity, placement.snap.now) > LIVE_SECS {
                continue;
            }
            let Some(meta) = read_meta_capped(&subagents.join(format!("agent-{id}{META_SUFFIX}")))
            else {
                continue;
            };
            if let Some(child) = placement.place(&meta, last_activity) {
                unjoined.push(child);
            }
        }
    }
    unjoined
}

impl AgentAdapter for ClaudeCodeAdapter {
    fn name(&self) -> &'static str {
        "claude-code"
    }

    fn enrich(&self, snap: &CoreSnapshot<'_>) -> AdapterReport {
        let Some(root) = self.config_root.as_deref() else {
            return AdapterReport::default();
        };
        let dirs = project_dirs(root, snap);
        if dirs.is_empty() {
            return AdapterReport::default();
        }
        let sessions = session_dirs(&dirs);
        let mut per_worktree: Vec<Option<Enrichment>> = vec![None; snap.worktrees.len()];
        let matched = enrich_by_id(snap, &sessions, &mut per_worktree);
        let mut placement = Placement {
            snap,
            primary: per_worktree.iter().map(Option::is_some).collect(),
            per_worktree,
        };
        let worktreeless = place_live_subagents(&mut placement, &sessions, &matched);
        AdapterReport {
            per_worktree: placement
                .per_worktree
                .into_iter()
                .enumerate()
                .filter_map(|(index, facts)| facts.map(|facts| (index, facts)))
                .collect(),
            worktreeless,
        }
    }
}

// Tempdir fixtures only: a hand-built `CoreSnapshot` over tempdir paths, no git
// and no process of any kind (this directory is not on the spawn allowlist,
// test code included). Every adapter here is `ClaudeCodeAdapter::new(<tmp>)`.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::adapters::ChildAgent;
    use serde_json::json;
    use std::time::Duration;
    use tempfile::TempDir;

    const SESSION: &str = "0b6f7a3e-1111-4222-8333-944455556666";

    /// A project dir, a fake config root and one empty session under it.
    struct Env {
        _tmp: TempDir,
        base: PathBuf,
        project: PathBuf,
        root: PathBuf,
        subagents: PathBuf,
    }

    fn env() -> Env {
        let tmp = TempDir::new().expect("tempdir");
        let base = std::fs::canonicalize(tmp.path()).expect("canonical tempdir");
        let project = base.join("work");
        std::fs::create_dir_all(&project).expect("project dir");
        let root = base.join("claude");
        let subagents = root
            .join("projects")
            .join(encode_project_dir(project.to_str().expect("utf-8")))
            .join(SESSION)
            .join("subagents");
        std::fs::create_dir_all(&subagents).expect("subagents dir");
        Env {
            _tmp: tmp,
            base,
            project,
            root,
            subagents,
        }
    }

    impl Env {
        /// A worktree directory `<base>/wts/<name>` with the given agent id.
        fn worktree(&self, name: &str, agent_id: Option<&str>) -> CoreWorktree {
            let path = self.base.join("wts").join(name);
            std::fs::create_dir_all(&path).expect("worktree dir");
            CoreWorktree {
                path,
                agent_id: agent_id.map(str::to_string),
                ..Default::default()
            }
        }

        fn write_meta_bytes(&self, id: &str, bytes: &[u8]) {
            std::fs::write(self.subagents.join(format!("agent-{id}.meta.json")), bytes)
                .expect("meta write");
        }

        fn write_meta(&self, id: &str, meta: serde_json::Value) {
            self.write_meta_bytes(id, &serde_json::to_vec(&meta).expect("json"));
        }

        /// `agent-<id>.jsonl`, modified `age_secs` before `now`.
        fn transcript(&self, id: &str, now: SystemTime, age_secs: u64) {
            let path = self.subagents.join(format!("agent-{id}.jsonl"));
            std::fs::write(&path, "{}\n").expect("transcript write");
            std::fs::File::open(&path)
                .and_then(|f| f.set_modified(now - Duration::from_secs(age_secs)))
                .expect("transcript mtime");
        }

        fn enrich(&self, worktrees: &[CoreWorktree], now: SystemTime) -> AdapterReport {
            let snap = CoreSnapshot {
                project_root: &self.project,
                main_worktree: None,
                worktrees,
                now,
            };
            ClaudeCodeAdapter::new(self.root.clone()).enrich(&snap)
        }
    }

    fn facts(report: &AdapterReport, index: usize) -> Option<&Enrichment> {
        report
            .per_worktree
            .iter()
            .find(|(i, _)| *i == index)
            .map(|(_, facts)| facts)
    }

    fn raw(value: &Option<Untrusted>) -> Option<&str> {
        value.as_ref().map(Untrusted::as_raw_for_logic_only)
    }

    // -----------------------------------------------------------------------
    // Encoding (D-C05)
    //
    // Every expected string below was printed by Claude Code 2.1.283's own
    // functions (`k`, `qx`, `Le`, `KJ`, quoted in 25-RESEARCH.md § Pattern 4)
    // under node, NOT by this implementation:
    //
    //   node -e 'function k(e){return e.replace(/[^a-zA-Z0-9]/g,"-")};function KJ(t){let e=0;for(let n=0;n<t.length;n++)e=(e<<5)-e+t.charCodeAt(n)|0;return e};function Le(e){return Math.abs(KJ(e)).toString(36)};function qx(e){let n=k(e);if(n.length<=200)return n;return`${n.slice(0,200)}-${Le(e)}`};for(const p of ["/home/blk/.local/state","/srv/media/My.Show - S01E02 [1080p]","/tmp/a\u{1F600}b","/tmp/café","/"+"a".repeat(249),"/"+"b".repeat(249)])console.log(qx(p), KJ(p));console.log(Math.abs(-2147483648).toString(36))'
    //
    // printed (the 200-unit bodies abbreviated here as `-` + 199 × the letter):
    //   -home-blk--local-state                 456616014
    //   -srv-media-My-Show---S01E02--1080p-    1327698200
    //   -tmp-a--b                              -1782071675
    //   -tmp-caf-                              ...
    //   -aaa…a-wkpu26                          -1969715310  (negative accumulator)
    //   -bbb…b-3d6hoz                          203536403    (positive accumulator)
    //   zik0zk                                 (abs(i32::MIN) in base 36)
    // -----------------------------------------------------------------------

    #[test]
    fn encoding_matches_claude_code_for_ascii_dots_and_spaces() {
        assert_eq!(
            encode_project_dir("/home/blk/.local/state"),
            "-home-blk--local-state"
        );
        assert_eq!(
            encode_project_dir("/srv/media/My.Show - S01E02 [1080p]"),
            "-srv-media-My-Show---S01E02--1080p-"
        );
    }

    #[test]
    fn encoding_emits_two_dashes_for_a_non_bmp_char() {
        assert_eq!(encode_project_dir("/tmp/a\u{1F600}b"), "-tmp-a--b");
        assert_eq!(
            encode_project_dir("/tmp/café"),
            "-tmp-caf-",
            "a BMP char is one UTF-16 unit, so one dash"
        );
    }

    #[test]
    fn encoding_over_200_units_appends_the_base36_hash() {
        let a = format!("/{}", "a".repeat(249));
        assert_eq!(
            encode_project_dir(&a),
            format!("-{}-wkpu26", "a".repeat(199)),
            "negative accumulator"
        );
        let b = format!("/{}", "b".repeat(249));
        assert_eq!(
            encode_project_dir(&b),
            format!("-{}-3d6hoz", "b".repeat(199)),
            "positive accumulator"
        );
        let exactly_200 = format!("/{}", "c".repeat(199));
        assert_eq!(
            encode_project_dir(&exactly_200),
            format!("-{}", "c".repeat(199)),
            "200 units is not over the limit"
        );
        assert_eq!(to_base36(i64::from(i32::MIN).unsigned_abs()), "zik0zk");
        assert_eq!(to_base36(0), "0");
        assert_eq!(to_base36(35), "z");
        assert_eq!(to_base36(36), "10");
    }

    #[test]
    fn a_relative_or_empty_config_dir_falls_back_to_home() {
        let home = PathBuf::from("/home/someone");
        let fallback = Some(home.join(".claude"));
        assert_eq!(
            resolve_config_root(Some("relative/dir".into()), Some(home.clone())),
            fallback
        );
        assert_eq!(
            resolve_config_root(Some("".into()), Some(home.clone())),
            fallback
        );
        assert_eq!(resolve_config_root(None, Some(home.clone())), fallback);
        assert_eq!(
            resolve_config_root(Some("/abs".into()), Some(home)),
            Some(PathBuf::from("/abs"))
        );
        assert_eq!(
            resolve_config_root(Some("/abs".into()), None),
            Some(PathBuf::from("/abs"))
        );
        assert_eq!(resolve_config_root(Some("relative".into()), None), None);
        assert_eq!(resolve_config_root(None, None), None);
    }

    // -----------------------------------------------------------------------
    // Tolerance and caps (D-A03, T-25-08, T-25-09, T-25-10)
    // -----------------------------------------------------------------------

    #[test]
    fn garbage_truncated_and_mistyped_metas_degrade_field_by_field() {
        let env = env();
        let now = SystemTime::now();
        let ids = [
            "garbage",
            "truncated",
            "mistyped",
            "nodescription",
            "unknownkeys",
        ];
        let worktrees: Vec<CoreWorktree> =
            ids.iter().map(|id| env.worktree(id, Some(id))).collect();
        env.write_meta_bytes("garbage", b"\x00\xffnot json at all");
        env.write_meta_bytes("truncated", br#"{"agentType":"gsd-executor","descr"#);
        env.write_meta("mistyped", json!({"agentType": 42, "description": "d"}));
        env.write_meta("nodescription", json!({"agentType": "gsd-executor"}));
        env.write_meta(
            "unknownkeys",
            json!({"agentType": "t", "description": "d", "brandNew": {"x": [1, 2]}, "spawnDepth": "two"}),
        );
        for id in ids {
            env.transcript(id, now, 5);
        }

        let report = env.enrich(&worktrees, now);
        assert!(facts(&report, 0).is_none(), "garbage is no data");
        assert!(facts(&report, 1).is_none(), "a torn rewrite is no data");

        let mistyped = facts(&report, 2).expect("a mistyped field costs only itself");
        assert_eq!(raw(&mistyped.agent_type), None);
        assert_eq!(raw(&mistyped.description), Some("d"));
        assert!(mistyped.last_activity.is_some());

        let no_description = facts(&report, 3).expect("description is optional");
        assert_eq!(raw(&no_description.agent_type), Some("gsd-executor"));
        assert_eq!(raw(&no_description.description), None);

        let unknown = facts(&report, 4).expect("unknown keys are ignored");
        assert_eq!(raw(&unknown.agent_type), Some("t"));
        assert_eq!(raw(&unknown.description), Some("d"));
        assert_eq!(report.per_worktree.len(), 3, "{:?}", report.per_worktree);
    }

    #[test]
    fn a_meta_larger_than_the_cap_is_no_data() {
        let env = env();
        let now = SystemTime::now();
        let body = br#"{"agentType":"gsd-executor"}"#;
        let padded = |len: u64| {
            let mut bytes = body.to_vec();
            bytes.resize(usize::try_from(len).expect("fits"), b' ');
            bytes
        };

        env.write_meta_bytes("fits", &padded(META_READ_CAP));
        let at_cap = env.subagents.join("agent-fits.meta.json");
        assert!(
            read_meta_capped(&at_cap).is_some(),
            "exactly the cap is read"
        );

        env.write_meta_bytes("toolarge", &padded(META_READ_CAP + 1));
        let over = env.subagents.join("agent-toolarge.meta.json");
        assert_eq!(read_meta_capped(&over), None, "one byte over is no data");

        env.transcript("toolarge", now, 5);
        let worktrees = [env.worktree("toolarge", Some("toolarge"))];
        assert!(env.enrich(&worktrees, now).per_worktree.is_empty());
    }

    #[test]
    fn the_meta_reader_refuses_a_transcript_path() {
        let env = env();
        let valid = br#"{"agentType":"gsd-executor","description":"secret conversation"}"#;
        let transcript = env.subagents.join("agent-x.jsonl");
        std::fs::write(&transcript, valid).expect("write");
        assert_eq!(
            read_meta_capped(&transcript),
            None,
            "a .jsonl is never opened, whatever it holds"
        );

        env.write_meta_bytes("x", valid);
        assert!(
            read_meta_capped(&env.subagents.join("agent-x.meta.json")).is_some(),
            "control: the same bytes under the meta name are read"
        );

        #[cfg(unix)]
        {
            let link = env.subagents.join("agent-y.meta.json");
            std::os::unix::fs::symlink(&transcript, &link).expect("symlink");
            assert_eq!(
                read_meta_capped(&link),
                None,
                "a .meta.json symlink to a transcript is refused"
            );
        }
    }

    #[test]
    fn an_unsafe_agent_id_never_reaches_the_filesystem() {
        let env = env();
        let now = SystemTime::now();
        // Files exist exactly where an unchecked id would lead, so any lookup
        // through them would find an acceptable meta (no `worktreePath`).
        let meta = br#"{"agentType":"gsd-executor"}"#;
        let nested = env.subagents.join("agent-a");
        std::fs::create_dir_all(&nested).expect("dir");
        std::fs::write(nested.join("b.meta.json"), meta).expect("write");
        std::fs::write(nested.join("b.jsonl"), "{}\n").expect("write");
        std::fs::write(env.subagents.join("agent-...meta.json"), meta).expect("write");
        std::fs::write(env.subagents.join("agent-...jsonl"), "{}\n").expect("write");

        let worktrees = [
            env.worktree("one", Some("a/b")),
            env.worktree("two", Some("..")),
        ];
        let report = env.enrich(&worktrees, now);
        assert!(report.per_worktree.is_empty(), "{:?}", report.per_worktree);
        assert!(report.worktreeless.is_empty(), "{:?}", report.worktreeless);
    }

    // -----------------------------------------------------------------------
    // Joins and children (D-C06, RESEARCH Pitfall 7)
    // -----------------------------------------------------------------------

    #[cfg(unix)]
    #[test]
    fn a_symlinked_worktree_path_still_joins() {
        let env = env();
        let now = SystemTime::now();
        let worktree = env.worktree("real", Some("abc"));
        let alias = env.base.join("alias");
        std::os::unix::fs::symlink(env.base.join("wts"), &alias).expect("symlink");
        env.write_meta(
            "abc",
            json!({"agentType": "gsd-executor", "worktreePath": alias.join("real")}),
        );
        env.transcript("abc", now, 5);

        let report = env.enrich(&[worktree], now);
        let joined = facts(&report, 0).expect("the canonical compare joins");
        assert_eq!(raw(&joined.agent_type), Some("gsd-executor"));
    }

    #[test]
    fn a_meta_naming_another_worktree_is_rejected() {
        let env = env();
        let now = SystemTime::now();
        let worktree = env.worktree("mine", Some("abc"));
        let other = env.worktree("other", None);
        env.write_meta(
            "abc",
            json!({"agentType": "gsd-executor", "worktreePath": other.path}),
        );
        env.transcript("abc", now, 5);

        let report = env.enrich(std::slice::from_ref(&worktree), now);
        assert!(facts(&report, 0).is_none(), "{:?}", report.per_worktree);
    }

    #[test]
    fn a_nested_agent_attaches_to_its_worktree_as_a_child() {
        let env = env();
        let now = SystemTime::now();
        let worktree = env.worktree("exec", Some("parent1"));
        env.write_meta(
            "parent1",
            json!({"agentType": "gsd-executor", "worktreePath": worktree.path}),
        );
        env.transcript("parent1", now, 5);
        env.write_meta(
            "child1",
            json!({
                "agentType": "gsd-code-fixer",
                "description": "fix finding 3",
                "inheritedWorktreePath": worktree.path,
                "spawnDepth": 3,
            }),
        );
        env.transcript("child1", now, 3);

        let report = env.enrich(&[worktree], now);
        let row = facts(&report, 0).expect("the worktree is enriched");
        assert_eq!(raw(&row.agent_type), Some("gsd-executor"));
        assert_eq!(row.children.len(), 1, "{:?}", row.children);
        let child: &ChildAgent = &row.children[0];
        assert_eq!(raw(&child.agent_type), Some("gsd-code-fixer"));
        assert_eq!(raw(&child.description), Some("fix finding 3"));
        assert_eq!(child.last_activity, Some(now - Duration::from_secs(3)));
        assert!(
            report.worktreeless.is_empty(),
            "a child is not worktree-less"
        );
    }

    #[test]
    fn a_parent_agent_id_attaches_a_child_without_an_inherited_path() {
        let env = env();
        let now = SystemTime::now();
        let with_meta = env.worktree("exec", Some("parent1"));
        let without_meta = env.worktree("bare", Some("parent2"));
        env.write_meta("parent1", json!({"agentType": "gsd-executor"}));
        env.transcript("parent1", now, 5);
        env.write_meta(
            "child1",
            json!({"agentType": "Explore", "parentAgentId": "parent1"}),
        );
        env.transcript("child1", now, 3);
        env.write_meta(
            "child2",
            json!({"agentType": "general-purpose", "parentAgentId": "parent2"}),
        );
        env.transcript("child2", now, 3);

        let report = env.enrich(&[with_meta, without_meta], now);
        let first = facts(&report, 0).expect("first worktree enriched");
        assert_eq!(first.children.len(), 1);
        assert_eq!(raw(&first.children[0].agent_type), Some("Explore"));

        let second = facts(&report, 1).expect("an enrichment carrying only the child");
        assert_eq!(raw(&second.agent_type), None);
        assert_eq!(second.last_activity, None);
        assert_eq!(second.children.len(), 1);
        assert_eq!(raw(&second.children[0].agent_type), Some("general-purpose"));
        assert!(report.worktreeless.is_empty());
    }
}
