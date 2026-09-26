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

use super::{AdapterReport, AgentAdapter, CoreSnapshot, Enrichment};
use crate::agents::worktrees::{valid_agent_id, CoreWorktree};
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
        enrich_by_id(snap, &sessions, &mut per_worktree);
        AdapterReport {
            per_worktree: per_worktree
                .into_iter()
                .enumerate()
                .filter_map(|(index, facts)| facts.map(|facts| (index, facts)))
                .collect(),
            worktreeless: Vec::new(),
        }
    }
}
