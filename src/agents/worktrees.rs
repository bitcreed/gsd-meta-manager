//! The runtime-agnostic core of the agent observer (D-A01a): which of a
//! project's git worktrees belong to agents, and what git alone can say about
//! each of them.
//!
//! **Read-only, and every git call goes through `git_ops`.** This file builds
//! no git command of its own; it asks [`git_ops::worktree_list_porcelain`],
//! [`git_ops::commits_ahead`], [`git_ops::dirty_count`] and
//! [`git_ops::log_subjects`], which all route
//! through `git_read_raw` and therefore carry `--no-optional-locks` plus
//! `GIT_OPTIONAL_LOCKS=0` (D-B02). A live agent owns these worktrees; a read
//! that refreshed one of their indexes would be the observer perturbing the run
//! it observes.
//!
//! Nothing here knows about Claude, Codex or any other runtime. Runtime facts
//! arrive later, through [`crate::agents::adapters`].
//!
//! Every text value git reports about a worktree — its path aside — is
//! attacker-controllable (a hostile clone names its own branches and lock
//! reasons), so branch names and lock reasons leave this module wrapped in
//! [`Untrusted`], and no branch name is ever placed into git's argv (D-C04).

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;

use crate::state_reader::git_ops;
use crate::text::Untrusted;

/// One worktree block of `git worktree list --porcelain`, as git reported it.
///
/// Plain strings rather than [`Untrusted`]: this is the parser's output, and it
/// is consumed only by [`scan_worktrees`] and tests, which wrap before anything
/// leaves the module.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawWorktree {
    /// The worktree's path, verbatim (C-unquoted in the non-`-z` fallback).
    pub path: PathBuf,
    /// The `HEAD` sha, when git reported one (a bare main has none).
    pub head: Option<String>,
    /// The checked-out branch with `refs/heads/` stripped; `None` when detached.
    pub branch: Option<String>,
    /// Whether git reported `detached`.
    pub detached: bool,
    /// `None` = not locked; `Some(None)` = locked without a reason;
    /// `Some(Some(r))` = locked with reason `r`.
    pub locked: Option<Option<String>>,
    /// Whether git reported `prunable` (typically: the directory is gone).
    pub prunable: bool,
    /// Whether git reported `bare` (main worktree of a bare repository only).
    pub bare: bool,
}

/// A plan id and spawn time read from a Codex-style agent branch name,
/// `worktree-agent-p{plan}-{unix_ts}` (D-A06).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BranchPlan {
    /// The plan id, e.g. `13-13` or `07.1-02`.
    pub plan: String,
    /// The Unix timestamp the branch name carries.
    pub spawned_unix: u64,
}

/// One NON-main worktree, as the core found it. Read-only to adapters.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoreWorktree {
    /// The worktree's path, as git reported it.
    pub path: PathBuf,
    /// The branch short name, wrapped because a hostile clone chooses it.
    pub branch: Option<Untrusted>,
    /// `None` = not locked; `Some(None)` = locked without a reason;
    /// `Some(Some(r))` = locked with reason `r`.
    pub locked: Option<Option<Untrusted>>,
    /// Whether git reported the worktree `prunable`. Such a worktree keeps its
    /// row and is never pruned from here (D-C16, D-B02).
    pub prunable: bool,
    /// The D-C03 predicate: under `<project>/.claude/worktrees/`, or on a
    /// branch in GSD's agent families. See [`is_agent_worktree`].
    pub agent_pattern: bool,
    /// The agent id from the path (`agent-<id>`) or branch, kept only when it
    /// passes [`valid_agent_id`].
    pub agent_id: Option<String>,
    /// The plan id and spawn time from a Codex-style branch name (D-A06).
    pub branch_plan: Option<BranchPlan>,
    /// The plan id from GSD's `gsd-plan-head-before-<plan>` ledger file in the
    /// worktree's admin dir — confirmation only (D-C10 tier 4); usually absent.
    pub ledger_plan: Option<String>,
}

/// Everything [`scan_worktrees`] learned about one project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoreScan {
    /// The main worktree's path, from the first porcelain block.
    pub main_worktree: Option<PathBuf>,
    /// The main worktree's HEAD, kept only when it is a full lowercase hex sha.
    pub base_sha: Option<String>,
    /// Every non-main worktree, agent or not, in git's order.
    pub worktrees: Vec<CoreWorktree>,
}

/// Apply one porcelain record to the block being built.
///
/// Unknown keys are ignored, so a newer git's additional attributes do not
/// break the parse.
fn apply_record(current: &mut RawWorktree, record: &str) {
    let (key, value) = match record.split_once(' ') {
        Some((key, value)) => (key, Some(value)),
        None => (record, None),
    };
    match key {
        "HEAD" => current.head = value.map(str::to_string),
        "branch" => {
            current.branch = value.map(|v| v.strip_prefix("refs/heads/").unwrap_or(v).to_string())
        }
        "detached" => current.detached = true,
        "locked" => current.locked = Some(value.filter(|v| !v.is_empty()).map(str::to_string)),
        "prunable" => current.prunable = true,
        "bare" => current.bare = true,
        _ => {}
    }
}

/// Parse a sequence of porcelain records into worktree blocks.
///
/// A `worktree <path>` record opens a block and an empty record closes it. The
/// first block is the main worktree. `unquote` is applied to the path only.
fn parse_records<'a>(
    records: impl Iterator<Item = &'a str>,
    unquote: fn(&str) -> String,
) -> Vec<RawWorktree> {
    let mut out = Vec::new();
    let mut current: Option<RawWorktree> = None;
    for record in records {
        if record.is_empty() {
            if let Some(done) = current.take() {
                out.push(done);
            }
            continue;
        }
        if let Some(path) = record.strip_prefix("worktree ") {
            if let Some(done) = current.take() {
                out.push(done);
            }
            current = Some(RawWorktree {
                path: PathBuf::from(unquote(path)),
                ..RawWorktree::default()
            });
            continue;
        }
        if let Some(block) = current.as_mut() {
            apply_record(block, record);
        }
    }
    if let Some(done) = current.take() {
        out.push(done);
    }
    out
}

/// Parse `git worktree list --porcelain -z` output. The first block is the
/// main worktree. Paths with spaces or other unusual bytes arrive verbatim
/// under `-z`. Empty input yields an empty vec.
pub fn parse_porcelain_z(raw: &str) -> Vec<RawWorktree> {
    parse_records(raw.split('\0'), str::to_string)
}

/// Parse the newline-separated `git worktree list --porcelain` output older git
/// produces (no `-z` before 2.36). A blank line ends a block.
///
/// Without `-z`, git C-quotes an unusual path: surrounding `"`, with `\\`,
/// `\"`, `\t`, `\n` and three-digit octal byte escapes (how git spells
/// non-ASCII bytes under the default `core.quotePath`). Those are undone; a
/// path with any other escape, or whose bytes are not UTF-8 once unescaped, is
/// kept as the raw string — best effort, never a failed scan.
pub fn parse_porcelain_lines(raw: &str) -> Vec<RawWorktree> {
    parse_records(raw.lines(), unquote_c_style)
}

/// Undo git's C-style path quoting, or return the input unchanged.
fn unquote_c_style(quoted: &str) -> String {
    try_unquote_c_style(quoted).unwrap_or_else(|| quoted.to_string())
}

/// `None` when `quoted` is not quoted, or carries an escape this cannot undo.
fn try_unquote_c_style(quoted: &str) -> Option<String> {
    let inner = quoted.strip_prefix('"')?.strip_suffix('"')?;
    let bytes = inner.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b != b'\\' {
            out.push(b);
            i += 1;
            continue;
        }
        let next = *bytes.get(i + 1)?;
        match next {
            b'\\' => out.push(b'\\'),
            b'"' => out.push(b'"'),
            b't' => out.push(b'\t'),
            b'n' => out.push(b'\n'),
            b'0'..=b'3' => {
                let digits = bytes.get(i + 1..i + 4)?;
                if !digits.iter().all(|d| (b'0'..=b'7').contains(d)) {
                    return None;
                }
                let value = digits
                    .iter()
                    .fold(0u16, |acc, d| acc * 8 + u16::from(d - b'0'));
                out.push(u8::try_from(value).ok()?);
                i += 4;
                continue;
            }
            _ => return None,
        }
        i += 2;
    }
    String::from_utf8(out).ok()
}

/// GSD's own agent-branch regex, `WORKTREE_AGENT_BRANCH_RE` in
/// `gsd-core/bin/lib/worktree-safety.cjs` (RESEARCH Pattern 1).
fn agent_branch_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^((worktree-)?agent-|worktree-wf_)[A-Za-z0-9._/-]+$").unwrap())
}

/// A Codex-style branch: `(worktree-)agent-p{plan}-{unix_ts}` (D-A06).
fn plan_branch_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^(?:worktree-)?agent-p(?P<plan>[0-9][0-9A-Za-z.]*(?:-[0-9]+)?)-(?P<ts>[0-9]{9,11})$",
        )
        .unwrap()
    })
}

/// A plan id: `13`, `13-02`, `07.1`, `07.1-02`.
fn plan_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9]+(\.[0-9]+)?(-[0-9]+)?$").unwrap())
}

/// Whether `id` is a plan id (`^[0-9]+(\.[0-9]+)?(-[0-9]+)?$`).
pub(crate) fn valid_plan_id(id: &str) -> bool {
    plan_id_re().is_match(id)
}

/// Whether `raw` is the directory `.claude/worktrees/<something>` under `root`.
fn under_claude_worktrees(root: &Path, raw: &RawWorktree) -> bool {
    raw.path
        .strip_prefix(root.join(".claude").join("worktrees"))
        .is_ok_and(|rest| rest.components().next().is_some())
}

/// The D-C03 agent-worktree predicate, widened to GSD's branch families.
///
/// True when the worktree's path is under `<project_root>/.claude/worktrees/`,
/// or its branch matches `^((worktree-)?agent-|worktree-wf_)[A-Za-z0-9._/-]+$`.
/// A path that merely contains `.claude` elsewhere does not qualify.
pub fn is_agent_worktree(project_root: &Path, raw: &RawWorktree) -> bool {
    under_claude_worktrees(project_root, raw)
        || raw
            .branch
            .as_deref()
            .is_some_and(|branch| agent_branch_re().is_match(branch))
}

/// The plan id and spawn time a Codex-style agent branch carries (D-A06).
///
/// Runtime-agnostic: the branch shape is what counts, not who created it.
/// Claude's `agent-a<16 hex>` ids never parse, because the plan must start
/// with a digit and satisfy the plan-id rule.
pub fn parse_agent_branch(branch: &str) -> Option<BranchPlan> {
    let caps = plan_branch_re().captures(branch)?;
    let plan = caps.name("plan")?.as_str();
    if !valid_plan_id(plan) {
        return None;
    }
    let spawned_unix = caps.name("ts")?.as_str().parse::<u64>().ok()?;
    Some(BranchPlan {
        plan: plan.to_string(),
        spawned_unix,
    })
}

/// Whether `id` may be used as an agent id — and therefore, later, as a path
/// component: `^[A-Za-z0-9_-]{1,64}$`, so no separator, no dot, no traversal.
pub fn valid_agent_id(id: &str) -> bool {
    (1..=64).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// The agent id: from a path component `agent-<id>` directly under
/// `.claude/worktrees/`, else from a branch `worktree-agent-<id>` or
/// `agent-<id>`; kept only when [`valid_agent_id`].
fn agent_id_of(roots: &[&Path], raw: &RawWorktree) -> Option<String> {
    let from_path = roots.iter().find_map(|root| {
        let rest = raw
            .path
            .strip_prefix(root.join(".claude").join("worktrees"))
            .ok()?;
        let first = rest.components().next()?.as_os_str().to_str()?;
        first.strip_prefix("agent-").map(str::to_string)
    });
    let from_branch = || {
        let branch = raw.branch.as_deref()?;
        branch
            .strip_prefix("worktree-agent-")
            .or_else(|| branch.strip_prefix("agent-"))
            .map(str::to_string)
    };
    from_path
        .filter(|id| valid_agent_id(id))
        .or_else(|| from_branch().filter(|id| valid_agent_id(id)))
}

/// The prefix of GSD's per-plan commit ledger in a worktree's admin dir
/// (`gsd-plan-head-before-<phase>-<plan>`, written by GSD's executor).
const LEDGER_PREFIX: &str = "gsd-plan-head-before-";

/// How much of a worktree's `.git` file is read. The file is one
/// `gitdir: <path>` line; a larger one is not a worktree pointer.
const DOT_GIT_FILE_CAP: u64 = 4096;

/// The plan id GSD's commit ledger names, when one is present (D-C10 tier 4).
///
/// Confirmation only, and usually absent (it was missing on 9 of 13 live
/// agents measured), so absence is normal and yields `None`. The worktree's
/// `.git` must be a FILE reading `gitdir: <admin dir>`; the admin dir must be
/// absolute and exist. Entries are read in sorted order so the answer does not
/// depend on directory iteration order, and a ledger id is accepted only when
/// it passes the plan-id rule — it never becomes a path.
fn read_ledger_plan(worktree: &Path) -> Option<String> {
    use std::io::Read;

    let dot_git = worktree.join(".git");
    if !std::fs::symlink_metadata(&dot_git).ok()?.is_file() {
        return None;
    }
    let mut text = String::new();
    std::fs::File::open(&dot_git)
        .ok()?
        .take(DOT_GIT_FILE_CAP)
        .read_to_string(&mut text)
        .ok()?;
    let admin = PathBuf::from(text.lines().next()?.strip_prefix("gitdir: ")?.trim());
    if !admin.is_absolute() || !admin.is_dir() {
        return None;
    }
    let mut ids: Vec<String> = std::fs::read_dir(&admin)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let id = name.strip_prefix(LEDGER_PREFIX)?;
            valid_plan_id(id).then(|| id.to_string())
        })
        .collect();
    ids.sort();
    ids.into_iter().next()
}

/// Enumerate a project's worktrees, classify each against the agent predicate,
/// and resolve the base the commit counts are measured from.
///
/// The base is the main worktree's HEAD from the first porcelain block of the
/// same listing, kept only when it is a full lowercase hex sha (D-C04).
///
/// Any failure — not a repository, no git, unparseable output — yields an
/// empty [`CoreScan`], never an error.
///
/// **Prefilter (D-C09 cost bound) [inferred].** When `<project_root>/.git` is
/// a directory with no `worktrees/` inside it, the repository has no linked
/// worktree and git is not run at all: the scan reports `project_root` as the
/// main worktree, no base, and no worktrees. Most registered projects are in
/// this state most of the time. A `.git` file (the project is itself a linked
/// worktree or a submodule) or a missing `.git` goes through the listing as
/// usual.
pub fn scan_worktrees(project_root: &Path) -> CoreScan {
    let dot_git = project_root.join(".git");
    if dot_git.is_dir() && !dot_git.join("worktrees").exists() {
        return CoreScan {
            main_worktree: Some(project_root.to_path_buf()),
            base_sha: None,
            worktrees: Vec::new(),
        };
    }
    let Some(listing) = git_ops::worktree_list_porcelain(project_root) else {
        return CoreScan::default();
    };
    let blocks = if listing.nul_separated {
        parse_porcelain_z(&listing.raw)
    } else {
        parse_porcelain_lines(&listing.raw)
    };
    let mut blocks = blocks.into_iter();
    let Some(main) = blocks.next() else {
        return CoreScan::default();
    };
    let base_sha = main
        .head
        .clone()
        .filter(|sha| git_ops::is_full_hex_sha(sha));

    // Git reports canonical paths; the registered root may be spelled
    // differently (a symlink, a trailing component), so both are tried.
    let main_path = main.path.clone();
    let roots: [&Path; 2] = [project_root, main_path.as_path()];

    let worktrees = blocks
        .map(|raw| {
            let agent_pattern = roots.iter().any(|root| is_agent_worktree(root, &raw));
            CoreWorktree {
                agent_id: agent_id_of(&roots, &raw),
                branch_plan: raw.branch.as_deref().and_then(parse_agent_branch),
                branch: raw.branch.clone().map(Untrusted::from_untrusted_source),
                locked: raw
                    .locked
                    .clone()
                    .map(|reason| reason.map(Untrusted::from_untrusted_source)),
                prunable: raw.prunable,
                agent_pattern,
                ledger_plan: read_ledger_plan(&raw.path),
                path: raw.path,
            }
        })
        .collect();

    CoreScan {
        main_worktree: Some(main_path),
        base_sha,
        worktrees,
    }
}

/// Commits ahead of `base_sha` and the dirty-file count for one worktree.
///
/// Both `None` for a `prunable` worktree, whose directory git already says is
/// gone — the helpers are not called for it (D-C16). A git failure for one
/// worktree affects only that worktree's counts.
pub(crate) fn worktree_counts(
    worktree: &CoreWorktree,
    base_sha: Option<&str>,
) -> (Option<u32>, Option<u32>) {
    if worktree.prunable {
        return (None, None);
    }
    let ahead = base_sha.and_then(|base| git_ops::commits_ahead(&worktree.path, base));
    let dirty = git_ops::dirty_count(&worktree.path);
    (ahead, dirty)
}

/// How many of a worktree's own commit subjects the commit-scope tier reads.
const COMMIT_SCOPE_SUBJECTS: u32 = 50;

/// The subjects of the commits `worktree`'s HEAD carries beyond `base_sha`,
/// newest first — the input to the commit-scope attribution tier
/// (`waves::commit_scope_plan`).
///
/// Empty for a `prunable` worktree (its directory is gone), and whenever
/// `git_ops::log_subjects` refuses the range (a base that is not a full hex
/// sha never reaches argv) or git fails.
pub(crate) fn commit_subjects(worktree: &CoreWorktree, base_sha: &str) -> Vec<String> {
    if worktree.prunable {
        return Vec::new();
    }
    git_ops::log_subjects(
        &worktree.path,
        &format!("{base_sha}..HEAD"),
        COMMIT_SCOPE_SUBJECTS,
    )
}

// Pure tests only: string fixtures, no git, no process of any kind. This
// directory is not on the spawn allowlist, and that holds for test code too.
#[cfg(test)]
mod tests {
    use super::*;

    const SHA_A: &str = "1111111111111111111111111111111111111111";
    const SHA_B: &str = "2222222222222222222222222222222222222222";

    fn raw(path: &str, branch: Option<&str>) -> RawWorktree {
        RawWorktree {
            path: PathBuf::from(path),
            branch: branch.map(str::to_string),
            ..RawWorktree::default()
        }
    }

    #[test]
    fn porcelain_z_main_first_locked_with_and_without_reason() {
        let listing = format!(
            "worktree /repo\0HEAD {SHA_A}\0branch refs/heads/main\0\0\
             worktree /repo/.claude/worktrees/agent-x\0HEAD {SHA_B}\0branch refs/heads/worktree-agent-x\0locked\0\0\
             worktree /repo/wt2\0HEAD {SHA_B}\0branch refs/heads/b2\0locked claude agent agent-x (pid 1 start 2)\0\0"
        );
        let blocks = parse_porcelain_z(&listing);
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0].path, PathBuf::from("/repo"), "main comes first");
        assert_eq!(blocks[0].head.as_deref(), Some(SHA_A));
        assert_eq!(
            blocks[0].branch.as_deref(),
            Some("main"),
            "refs/heads/ stripped"
        );
        assert_eq!(blocks[0].locked, None);
        assert_eq!(blocks[1].locked, Some(None), "locked without a reason");
        assert_eq!(
            blocks[2].locked,
            Some(Some("claude agent agent-x (pid 1 start 2)".to_string())),
            "locked with a reason"
        );
    }

    #[test]
    fn porcelain_z_prunable_detached_bare_and_spaces() {
        let listing = format!(
            "worktree /repo\0bare\0\0\
             worktree /tmp/my dir/wt\0HEAD {SHA_B}\0detached\0prunable gitdir file points to non-existent location\0frobnicate yes\0\0"
        );
        let blocks = parse_porcelain_z(&listing);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].bare, "bare main");
        assert_eq!(blocks[0].head, None, "a bare main has no HEAD record");
        assert_eq!(
            blocks[1].path,
            PathBuf::from("/tmp/my dir/wt"),
            "spaces verbatim"
        );
        assert!(blocks[1].detached);
        assert_eq!(blocks[1].branch, None);
        assert!(blocks[1].prunable);
        assert!(!blocks[1].bare, "the unknown `frobnicate` key is ignored");
        assert!(parse_porcelain_z("").is_empty(), "empty input");
    }

    #[test]
    fn porcelain_lines_fallback_unquotes_c_style_paths() {
        let listing = format!(
            "worktree /repo\nHEAD {SHA_A}\nbranch refs/heads/main\n\n\
             worktree \"/tmp/a\\tb\"\nHEAD {SHA_B}\ndetached\n\n\
             worktree \"/tmp/a\\\"b\"\nHEAD {SHA_B}\nbranch refs/heads/x\n\n\
             worktree \"/tmp/a\\\\b\"\nHEAD {SHA_B}\nlocked\n\n\
             worktree \"/tmp/a\\nb\"\nHEAD {SHA_B}\n\n\
             worktree \"/tmp/caf\\303\\251\"\nHEAD {SHA_B}\n\n\
             worktree \"/tmp/bad\\q\"\nHEAD {SHA_B}\n\n"
        );
        let blocks = parse_porcelain_lines(&listing);
        assert_eq!(blocks.len(), 7);
        assert_eq!(blocks[0].path, PathBuf::from("/repo"));
        assert_eq!(blocks[0].branch.as_deref(), Some("main"));
        assert_eq!(blocks[1].path, PathBuf::from("/tmp/a\tb"), "\\t");
        assert!(blocks[1].detached);
        assert_eq!(blocks[2].path, PathBuf::from("/tmp/a\"b"), "\\\"");
        assert_eq!(blocks[3].path, PathBuf::from("/tmp/a\\b"), "\\\\");
        assert_eq!(blocks[3].locked, Some(None));
        assert_eq!(blocks[4].path, PathBuf::from("/tmp/a\nb"), "\\n");
        assert_eq!(
            blocks[5].path,
            PathBuf::from("/tmp/café"),
            "octal UTF-8 bytes"
        );
        assert_eq!(
            blocks[6].path,
            PathBuf::from("\"/tmp/bad\\q\""),
            "an escape it cannot undo keeps the raw string"
        );
    }

    #[test]
    fn agent_branch_grammar_accepts_dashed_bare_and_decimal_plans() {
        let plan = |plan: &str| {
            Some(BranchPlan {
                plan: plan.to_string(),
                spawned_unix: 1_790_386_422,
            })
        };
        assert_eq!(
            parse_agent_branch("worktree-agent-p13-13-1790386422"),
            plan("13-13")
        );
        assert_eq!(parse_agent_branch("agent-p22-1790386422"), plan("22"));
        assert_eq!(
            parse_agent_branch("worktree-agent-p07.1-02-1790386422"),
            plan("07.1-02")
        );
    }

    #[test]
    fn agent_branch_grammar_rejects_claude_hex_ids_and_hostile_values() {
        assert_eq!(parse_agent_branch("worktree-agent-a0123456789abcdef"), None);
        assert_eq!(parse_agent_branch("agent-p13-13"), None);
        assert_eq!(parse_agent_branch("agent-p13-13-12345678"), None);
        assert_eq!(parse_agent_branch("agent-p../x-1790386422"), None);
        assert_eq!(parse_agent_branch("agent-p13x-1790386422"), None);
        assert_eq!(parse_agent_branch("feature-p13-1790386422"), None);
        assert_eq!(parse_agent_branch(""), None);
    }

    #[test]
    fn agent_id_validation_refuses_traversal_and_separators() {
        assert!(valid_agent_id("a0123456789abcdef"));
        assert!(valid_agent_id("p13-13-1790386422"));
        assert!(valid_agent_id(&"a".repeat(64)));
        assert!(!valid_agent_id(""));
        assert!(!valid_agent_id(".."));
        assert!(!valid_agent_id("a/b"));
        assert!(!valid_agent_id("a.b"));
        assert!(!valid_agent_id(&"a".repeat(65)));
        assert!(!valid_agent_id(" a0123"));
    }

    #[test]
    fn agent_predicate_matches_gsd_branch_families() {
        let root = Path::new("/repo");
        assert!(is_agent_worktree(
            root,
            &raw("/repo/.claude/worktrees/x", None)
        ));
        assert!(is_agent_worktree(
            root,
            &raw("/elsewhere/a", Some("agent-abc"))
        ));
        assert!(is_agent_worktree(
            root,
            &raw("/elsewhere/b", Some("worktree-agent-abc"))
        ));
        assert!(is_agent_worktree(
            root,
            &raw("/elsewhere/c", Some("worktree-wf_abc"))
        ));
        assert!(!is_agent_worktree(root, &raw("/repo/side", Some("main"))));
        assert!(!is_agent_worktree(
            root,
            &raw("/repo/side", Some("feature/x"))
        ));
        assert!(!is_agent_worktree(
            root,
            &raw("/repo/sub/.claude/worktrees/x", Some("main"))
        ));
        assert!(!is_agent_worktree(
            root,
            &raw("/other/.claude/worktrees/x", None)
        ));
        assert!(!is_agent_worktree(
            root,
            &raw("/repo/.claude/worktrees", None)
        ));
    }
}
