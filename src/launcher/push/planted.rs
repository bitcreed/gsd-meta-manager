//! Planted-file detection for the push step (Phase 28.1; LNCH-11, D-21, D-22,
//! D-23, RESEARCH Pattern 7).
//!
//! The push step runs `git` on the host against a checkout the driver wrote. A
//! driver that planted a `core.fsmonitor`, a hook, a filter or a worktree pointer
//! there would get it executed by that `git`. This module is how the launcher
//! finds such a plant **without running anything**: every fact comes from a file
//! read (a `symlink_metadata` call, the bytes of a regular file, a text parse of a
//! git config). It starts no process and imports nothing from `super::process`;
//! `tests/spawn_seam_guard.rs` keeps that true.
//!
//! Files are never reverted, deleted or rewritten here: the guard reports, the
//! push refuses, and the operator decides (the non-intrusive constraint).

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Files whose content is hashed (D-21). Paths are relative to the checkout.
pub const TRACKED_FILES: &[&str] = &[
    ".git/config",
    ".git/info/attributes",
    ".gitattributes",
    ".claude/settings.json",
    ".claude/settings.local.json",
    "CLAUDE.md",
    ".envrc",
    ".vscode/tasks.json",
    ".mcp.json",
];

/// Directories whose regular files are each hashed (D-21).
pub const TRACKED_DIRS: &[&str] = &[];

/// Directory names the nested-repository walk never enters. A plant inside one of
/// them is an accepted residual `[inferred — audit]`.
pub const SKIP_DIR_NAMES: &[&str] = &["target", "node_modules", ".venv", "venv", "__pycache__"];

/// The most directory entries one walk visits before it gives up and reports a
/// truncation (a flood of files must not hide a plant silently).
pub const MAX_WALK_ENTRIES: usize = 200_000;

/// The largest git config file that is parsed.
pub const MAX_CONFIG_BYTES: u64 = 1_048_576;

/// The most findings [`render_bounded`] lists.
pub const MAX_RECORDED: usize = 20;

/// The most bytes of one tracked file that are hashed.
const MAX_HASH_BYTES: u64 = 64 * 1024 * 1024;

/// The baseline file inside the run's launcher state directory.
pub const BASELINE_FILE: &str = "planted-baseline.json";

/// What one path was when it was looked at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Entry {
    /// Nothing there.
    Absent,
    /// A regular file; the payload is the lowercase hex sha256 of its bytes.
    File(String),
    /// A directory.
    Dir,
    /// A symlink; the payload is its target, never followed.
    Symlink(String),
    /// Anything else (a socket, a device, an unreadable file).
    Other,
}

/// Everything the guard looks at, as of one moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    /// [`TRACKED_FILES`] and the files below [`TRACKED_DIRS`], by relative path.
    pub files: BTreeMap<String, Entry>,
    /// `.git` entries and worktree pointers, by relative path.
    pub git_entries: BTreeMap<String, Entry>,
    /// A walk hit its entry limit: the snapshot is incomplete.
    pub truncated: bool,
}

impl Snapshot {
    const VERSION: u32 = 1;

    fn new() -> Snapshot {
        Snapshot {
            version: Self::VERSION,
            files: BTreeMap::new(),
            git_entries: BTreeMap::new(),
            truncated: false,
        }
    }
}

/// What a finding is about.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingKind {
    Changed,
    Created,
    Removed,
    NestedGit,
    WorktreePointer,
    /// A git config key that runs a program (`core.fsmonitor`, `alias.x`, ...).
    GitConfigKey(String),
    ScanTruncated,
}

/// One thing the guard found. There is nowhere to put file content.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Finding {
    pub path: String,
    pub kind: FindingKind,
}

/// `text` as one safe line: control characters (a driver chose these names)
/// become `?`.
fn sanitize(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect()
}

impl Finding {
    /// Whether this finding blocks the push (D-23): anything that can make the
    /// host's own git run something, or that hides that it could.
    pub fn is_git_relevant(&self) -> bool {
        match &self.kind {
            FindingKind::NestedGit
            | FindingKind::WorktreePointer
            | FindingKind::GitConfigKey(_)
            | FindingKind::ScanTruncated => true,
            FindingKind::Changed | FindingKind::Created | FindingKind::Removed => {
                self.path.starts_with(".git/")
                    || self.path.contains("/.git/")
                    || self.path == ".gitattributes"
            }
        }
    }

    /// One line, for stderr and the record.
    pub fn render(&self) -> String {
        let path = sanitize(&self.path);
        match &self.kind {
            FindingKind::Changed => format!("changed {path}"),
            FindingKind::Created => format!("created {path}"),
            FindingKind::Removed => format!("removed {path}"),
            FindingKind::NestedGit => format!("nested-git {path}"),
            FindingKind::WorktreePointer => format!("worktree-pointer {path}"),
            FindingKind::GitConfigKey(key) => format!("git-config-key:{} {path}", sanitize(key)),
            FindingKind::ScanTruncated => "scan-truncated .".to_owned(),
        }
    }
}

// ---------------------------------------------------------------- snapshot

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The sha256 of at most [`MAX_HASH_BYTES`] of `path`.
fn hash_file(path: &Path) -> io::Result<String> {
    let file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut reader = file.take(MAX_HASH_BYTES);
    let mut buf = [0u8; 16 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

/// What `path` is, without following a symlink.
fn entry_of(path: &Path) -> Entry {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return Entry::Absent;
    };
    let kind = meta.file_type();
    if kind.is_symlink() {
        return match fs::read_link(path) {
            Ok(target) => Entry::Symlink(target.to_string_lossy().into_owned()),
            Err(_) => Entry::Other,
        };
    }
    if kind.is_file() {
        return hash_file(path).map_or(Entry::Other, Entry::File);
    }
    if kind.is_dir() {
        return Entry::Dir;
    }
    Entry::Other
}

/// Look at the checkout. File reads only.
pub fn snapshot(project: &Path) -> Snapshot {
    snapshot_with_limit(project, MAX_WALK_ENTRIES)
}

/// [`snapshot`] with the walk limit a test can lower.
pub fn snapshot_with_limit(project: &Path, limit: usize) -> Snapshot {
    let _ = limit;
    let mut snap = Snapshot::new();
    for rel in TRACKED_FILES {
        snap.files
            .insert((*rel).to_owned(), entry_of(&project.join(rel)));
    }
    snap
}

// ---------------------------------------------------------------- git config

/// One key of a git config file.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfigKey {
    /// Lowercase.
    section: String,
    sub: Option<String>,
    /// Lowercase.
    name: String,
    value: String,
}

impl ConfigKey {
    fn full(&self) -> String {
        match &self.sub {
            Some(sub) => format!("{}.{}.{}", self.section, sub, self.name),
            None => format!("{}.{}", self.section, self.name),
        }
    }

    /// Whether this key can make git run a program or load more configuration.
    fn runs_something(&self) -> bool {
        matches!(
            self.name.as_str(),
            "fsmonitor" | "hookspath" | "sshcommand" | "pager" | "editor" | "textconv"
        ) || matches!(
            self.section.as_str(),
            "alias" | "filter" | "credential" | "include" | "includeif"
        )
    }
}

/// The text between the quotes of a subsection, with `\"` and `\\` resolved.
fn unquote_sub(raw: &str) -> String {
    let mut out = String::new();
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(next);
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `[name]`, `[name "sub"]` or `[name.sub]` -> (section, sub), and the rest of the line.
fn parse_header(line: &str) -> Option<(String, Option<String>, &str)> {
    let inner_start = line.strip_prefix('[')?;
    // The closing bracket outside any quoted subsection.
    let mut in_quote = false;
    let mut escaped = false;
    let mut end = None;
    for (i, c) in inner_start.char_indices() {
        if escaped {
            escaped = false;
        } else if c == '\\' && in_quote {
            escaped = true;
        } else if c == '"' {
            in_quote = !in_quote;
        } else if c == ']' && !in_quote {
            end = Some(i);
            break;
        }
    }
    let end = end?;
    let (header, rest) = (&inner_start[..end], &inner_start[end + 1..]);
    let header = header.trim();
    if let Some((name, quoted)) = header.split_once(char::is_whitespace) {
        let quoted = quoted.trim();
        let quoted = quoted.strip_prefix('"')?.strip_suffix('"')?;
        return Some((name.to_ascii_lowercase(), Some(unquote_sub(quoted)), rest));
    }
    match header.split_once('.') {
        Some((name, sub)) => Some((
            name.to_ascii_lowercase(),
            Some(sub.to_ascii_lowercase()),
            rest,
        )),
        None => Some((header.to_ascii_lowercase(), None, rest)),
    }
}

/// Every key of a git config text. A line that does not parse is skipped: this is
/// a detector, not git.
fn parse_config(text: &str) -> Vec<ConfigKey> {
    let mut keys = Vec::new();
    let mut current: Option<(String, Option<String>)> = None;
    for raw in text.lines() {
        let mut line = raw.trim();
        while !line.is_empty() {
            if line.starts_with('#') || line.starts_with(';') {
                break;
            }
            if line.starts_with('[') {
                match parse_header(line) {
                    Some((section, sub, rest)) => {
                        current = Some((section, sub));
                        line = rest.trim();
                        continue;
                    }
                    None => break,
                }
            }
            let Some((section, sub)) = &current else {
                break;
            };
            let name_end = line
                .find(|c: char| c == '=' || c.is_whitespace())
                .unwrap_or(line.len());
            let name = line[..name_end].to_ascii_lowercase();
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                break;
            }
            let value = line[name_end..]
                .trim_start()
                .strip_prefix('=')
                .map(|v| v.trim().trim_matches('"').to_owned())
                .unwrap_or_default();
            keys.push(ConfigKey {
                section: section.clone(),
                sub: sub.clone(),
                name,
                value,
            });
            break;
        }
    }
    keys
}

/// The keys of a git config text that can run a program or pull in more
/// configuration (the D-22 list), as `section[.sub].key` with the section and key
/// names lowercase.
pub fn exec_keys_in(config_text: &str) -> Vec<String> {
    parse_config(config_text)
        .iter()
        .filter(|key| key.runs_something())
        .map(ConfigKey::full)
        .collect()
}

/// The text of a config file, or `None` when it cannot be read as one: absent, a
/// symlink (never followed), too large (the bool says it was) or unreadable.
fn read_config(path: &Path) -> Option<Result<String, FindingKind>> {
    let meta = fs::symlink_metadata(path).ok()?;
    if meta.file_type().is_symlink() {
        return Some(Err(FindingKind::GitConfigKey(
            "symlinked-config".to_owned(),
        )));
    }
    if !meta.is_file() {
        return None;
    }
    if meta.len() > MAX_CONFIG_BYTES {
        return Some(Err(FindingKind::GitConfigKey("oversized".to_owned())));
    }
    let bytes = fs::read(path).ok()?;
    Some(Ok(String::from_utf8_lossy(&bytes).into_owned()))
}

/// Whether `rel` names a git config file.
fn is_config_path(rel: &str) -> bool {
    rel == ".git/config"
}

/// The exec-key findings of one config file.
fn scan_config(project: &Path, rel: &str, out: &mut Vec<Finding>) {
    match read_config(&project.join(rel)) {
        Some(Ok(text)) => {
            for key in exec_keys_in(&text) {
                out.push(Finding {
                    path: rel.to_owned(),
                    kind: FindingKind::GitConfigKey(key),
                });
            }
        }
        Some(Err(kind)) => out.push(Finding {
            path: rel.to_owned(),
            kind,
        }),
        None => {}
    }
}

// ---------------------------------------------------------------- findings

#[derive(Clone, Copy)]
enum Change {
    Created,
    Removed,
    Changed,
}

/// The paths whose entry differs between two maps (a missing path is `Absent`).
fn diff(base: &BTreeMap<String, Entry>, now: &BTreeMap<String, Entry>) -> Vec<(String, Change)> {
    let mut paths: Vec<&String> = base.keys().chain(now.keys()).collect();
    paths.sort();
    paths.dedup();
    let absent = Entry::Absent;
    paths
        .into_iter()
        .filter_map(|path| {
            let (was, is) = (
                base.get(path).unwrap_or(&absent),
                now.get(path).unwrap_or(&absent),
            );
            if was == is {
                return None;
            }
            let change = match (was, is) {
                (Entry::Absent, _) => Change::Created,
                (_, Entry::Absent) => Change::Removed,
                _ => Change::Changed,
            };
            Some((path.clone(), change))
        })
        .collect()
}

/// What the checkout holds now that a baseline (or its absence) makes suspect.
///
/// With a baseline: every path whose entry differs, and the exec keys of every
/// git config that is new or changed. Without one (a run older than 28.1, or a
/// lost baseline): strict mode, every git config found is scanned. Sorted by path,
/// then kind.
pub fn findings(baseline: Option<&Snapshot>, now: &Snapshot, project: &Path) -> Vec<Finding> {
    let mut out = Vec::new();
    if now.truncated {
        out.push(Finding {
            path: ".".to_owned(),
            kind: FindingKind::ScanTruncated,
        });
    }
    let mut configs: Vec<String> = Vec::new();
    match baseline {
        Some(base) => {
            for (path, change) in diff(&base.files, &now.files) {
                if is_config_path(&path) {
                    configs.push(path.clone());
                }
                out.push(Finding {
                    path,
                    kind: match change {
                        Change::Created => FindingKind::Created,
                        Change::Removed => FindingKind::Removed,
                        Change::Changed => FindingKind::Changed,
                    },
                });
            }
        }
        None => {
            configs.extend(now.files.keys().filter(|p| is_config_path(p)).cloned());
        }
    }
    configs.sort();
    configs.dedup();
    for rel in &configs {
        scan_config(project, rel, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

// ---------------------------------------------------------------- baseline

/// Write `planted-baseline.json` into `state_dir` unless one is there.
///
/// Through a temp file in the same directory and a no-clobber rename, so an
/// interrupted write leaves either no baseline (strict mode) or a complete one.
/// Returns whether this call wrote it.
pub fn save_baseline_if_absent(state_dir: &Path, snapshot: &Snapshot) -> io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    let target = state_dir.join(BASELINE_FILE);
    if fs::symlink_metadata(&target).is_ok() {
        return Ok(false);
    }
    fs::create_dir_all(state_dir)?;
    let json = serde_json::to_vec(snapshot).map_err(io::Error::other)?;
    let mut temp = tempfile::NamedTempFile::new_in(state_dir)?;
    io::Write::write_all(&mut temp, &json)?;
    temp.as_file().sync_all()?;
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o600))?;
    match temp.persist_noclobber(&target) {
        Ok(_) => Ok(true),
        Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e.error),
    }
}

/// The baseline in `state_dir`: `Ok(None)` when there is none or it does not
/// parse (the guard then runs in strict mode).
pub fn load_baseline(state_dir: &Path) -> io::Result<Option<Snapshot>> {
    let path: PathBuf = state_dir.join(BASELINE_FILE);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    Ok(serde_json::from_slice::<Snapshot>(&bytes)
        .ok()
        .filter(|s| s.version == Snapshot::VERSION))
}

/// The findings as at most [`MAX_RECORDED`] lines; when there are more, the last
/// line is `+N more` and counts what was left out.
pub fn render_bounded(found: &[Finding]) -> Vec<String> {
    if found.len() <= MAX_RECORDED {
        return found.iter().map(Finding::render).collect();
    }
    let shown = MAX_RECORDED - 1;
    let mut lines: Vec<String> = found.iter().take(shown).map(Finding::render).collect();
    lines.push(format!("+{} more", found.len() - shown));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, text).expect("write");
    }

    #[test]
    fn exec_keys_cover_the_core_d22_list() {
        let text = "[core]\n\tfsmonitor = x\n\thooksPath = /h\n[alias]\n\tst = status\n[user]\n\tname = a\n";
        assert_eq!(
            exec_keys_in(text),
            ["core.fsmonitor", "core.hookspath", "alias.st"]
        );
    }

    #[test]
    fn a_changed_config_is_a_git_relevant_finding_with_its_keys() {
        let dir = tempfile::tempdir().expect("dir");
        write(dir.path(), ".git/config", "[core]\n\tbare = false\n");
        let base = snapshot(dir.path());
        write(
            dir.path(),
            ".git/config",
            "[core]\n\tbare = false\n\tfsmonitor = \"touch x; false\"\n",
        );
        let now = snapshot(dir.path());
        let found = findings(Some(&base), &now, dir.path());
        assert!(found.iter().all(Finding::is_git_relevant), "{found:?}");
        assert!(found.contains(&Finding {
            path: ".git/config".to_owned(),
            kind: FindingKind::GitConfigKey("core.fsmonitor".to_owned())
        }));
        assert!(found.iter().any(|f| f.kind == FindingKind::Changed));
    }

    #[test]
    fn strict_mode_scans_the_config_without_a_baseline() {
        let dir = tempfile::tempdir().expect("dir");
        write(dir.path(), ".git/config", "[core]\n\tfsmonitor = x\n");
        let found = findings(None, &snapshot(dir.path()), dir.path());
        assert_eq!(
            found,
            [Finding {
                path: ".git/config".to_owned(),
                kind: FindingKind::GitConfigKey("core.fsmonitor".to_owned())
            }]
        );
    }

    #[test]
    fn a_clean_checkout_has_no_findings_either_way() {
        let dir = tempfile::tempdir().expect("dir");
        write(dir.path(), ".git/config", "[core]\n\tbare = false\n");
        let snap = snapshot(dir.path());
        assert!(findings(Some(&snap), &snapshot(dir.path()), dir.path()).is_empty());
        assert!(findings(None, &snap, dir.path()).is_empty());
    }

    #[test]
    fn the_guard_changes_nothing_it_looks_at() {
        let dir = tempfile::tempdir().expect("dir");
        write(dir.path(), ".git/config", "[core]\n\tfsmonitor = x\n");
        let before = fs::read(dir.path().join(".git/config")).expect("read");
        let snap = snapshot(dir.path());
        let _ = findings(None, &snap, dir.path());
        assert_eq!(
            fs::read(dir.path().join(".git/config")).expect("read"),
            before
        );
    }

    #[test]
    fn a_non_git_change_does_not_block() {
        let dir = tempfile::tempdir().expect("dir");
        write(dir.path(), ".claude/settings.json", "{}");
        let base = snapshot(dir.path());
        write(dir.path(), ".claude/settings.json", "{\"a\":1}");
        let found = findings(Some(&base), &snapshot(dir.path()), dir.path());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(!found[0].is_git_relevant());
    }

    // ------------------------------------------------ 28.1-03 task 2: full detection

    fn kinds(found: &[Finding]) -> Vec<String> {
        found.iter().map(Finding::render).collect()
    }

    #[test]
    fn exec_keys_follow_the_d22_example_and_skip_the_rest() {
        let text = "# a comment\n; another\n\n[core]\n\tfsmonitor = x\n[filter \"lfs\"]\n\tclean = y\n[alias]\n\tst = status\n[includeIf \"gitdir:/x/\"]\n\tpath = /tmp/c\n[user]\n\tname = a\n";
        assert_eq!(
            exec_keys_in(text),
            [
                "core.fsmonitor",
                "filter.lfs.clean",
                "alias.st",
                "includeif.gitdir:/x/.path"
            ]
        );
    }

    #[test]
    fn every_key_in_the_extended_list_is_returned() {
        // (section, subsection, key)
        let table: &[(&str, Option<&str>, &str)] = &[
            ("core", None, "hooksPath"),
            ("core", None, "sshCommand"),
            ("core", None, "pager"),
            ("core", None, "editor"),
            ("core", None, "worktree"),
            ("core", None, "gitProxy"),
            ("core", None, "askPass"),
            ("core", None, "alternateRefsCommand"),
            ("sequence", None, "editor"),
            ("gpg", None, "program"),
            ("gpg", Some("ssh"), "program"),
            ("diff", None, "external"),
            ("diff", Some("x"), "textconv"),
            ("diff", Some("x"), "command"),
            ("merge", Some("x"), "driver"),
            ("credential", None, "helper"),
            ("credential", Some("https://h"), "helper"),
            ("include", None, "path"),
            ("uploadpack", None, "packObjectsHook"),
            ("remote", Some("origin"), "uploadpack"),
            ("remote", Some("origin"), "receivepack"),
            ("submodule", Some("x"), "update"),
            ("init", None, "templateDir"),
            ("http", None, "proxy"),
            ("http", Some("https://h"), "proxy"),
            ("extensions", None, "worktreeConfig"),
            ("x", Some("y"), "command"),
            ("x", None, "cmd"),
        ];
        for (section, sub, key) in table {
            let header = match sub {
                Some(sub) => format!("[{section} \"{sub}\"]"),
                None => format!("[{section}]"),
            };
            let text = format!("{header}\n\t{key} = v\n");
            assert_eq!(
                exec_keys_in(&text).len(),
                1,
                "{section} {sub:?} {key} -> {text:?}"
            );
        }
        for harmless in [
            "[user]\n\tname = a\n",
            "[core]\n\tbare = false\n",
            "[remote \"o\"]\n\turl = u\n",
        ] {
            assert!(exec_keys_in(harmless).is_empty(), "{harmless:?}");
        }
    }

    /// A checkout with a linked harness worktree, tracked dirs and git hooks.
    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("dir");
        let p = dir.path();
        write(p, ".git/config", "[core]\n\tbare = false\n");
        write(p, ".git/hooks/pre-commit", "#!/bin/sh\n");
        write(p, ".claude/agents/a.md", "agent a\n");
        write(
            p,
            ".claude/worktrees/w/.git",
            &format!("gitdir: {}/.git/worktrees/w\n", p.display()),
        );
        write(
            p,
            ".git/worktrees/w/gitdir",
            &format!("{}/.claude/worktrees/w/.git\n", p.display()),
        );
        write(p, ".git/worktrees/w/commondir", "../..\n");
        dir
    }

    #[test]
    fn the_tracked_dirs_are_hashed_and_a_new_file_is_not_git_relevant() {
        let dir = fixture();
        let base = snapshot(dir.path());
        assert!(
            base.files.contains_key(".git/hooks/pre-commit"),
            "{:?}",
            base.files
        );
        assert!(base.files.contains_key(".claude/agents/a.md"));
        write(dir.path(), ".claude/agents/b.md", "agent b\n");
        let found = findings(Some(&base), &snapshot(dir.path()), dir.path());
        assert_eq!(kinds(&found), ["created .claude/agents/b.md"]);
        assert!(!found[0].is_git_relevant());
        write(dir.path(), ".git/hooks/pre-push", "#!/bin/sh\n");
        let found = findings(Some(&base), &snapshot(dir.path()), dir.path());
        assert!(
            found
                .iter()
                .any(|f| f.render() == "created .git/hooks/pre-push" && f.is_git_relevant()),
            "{found:?}"
        );
    }

    #[test]
    fn a_rewritten_commondir_is_a_worktree_pointer() {
        let dir = fixture();
        let base = snapshot(dir.path());
        assert!(
            base.git_entries.contains_key(".git/worktrees/w/commondir"),
            "{:?}",
            base.git_entries
        );
        assert!(findings(Some(&base), &snapshot(dir.path()), dir.path()).is_empty());
        write(dir.path(), ".git/worktrees/w/commondir", "/tmp/evil\n");
        let found = findings(Some(&base), &snapshot(dir.path()), dir.path());
        assert!(
            kinds(&found).contains(&"worktree-pointer .git/worktrees/w/commondir".to_owned()),
            "{found:?}"
        );
        assert!(found.iter().all(Finding::is_git_relevant));
    }

    #[test]
    fn a_new_nested_git_is_found_with_its_config_keys() {
        let dir = fixture();
        let base = snapshot(dir.path());
        write(dir.path(), "sub/.git/config", "[core]\n\tfsmonitor = x\n");
        let found = findings(Some(&base), &snapshot(dir.path()), dir.path());
        let lines = kinds(&found);
        assert!(
            lines.contains(&"nested-git sub/.git".to_owned()),
            "{lines:?}"
        );
        assert!(
            lines.contains(&"git-config-key:core.fsmonitor sub/.git/config".to_owned()),
            "{lines:?}"
        );
    }

    #[test]
    fn a_nested_git_under_a_skipped_directory_is_not_walked() {
        let dir = fixture();
        let base = snapshot(dir.path());
        write(
            dir.path(),
            "target/x/.git/config",
            "[core]\n\tfsmonitor = x\n",
        );
        write(
            dir.path(),
            "node_modules/y/.git/config",
            "[core]\n\tfsmonitor = x\n",
        );
        assert!(findings(Some(&base), &snapshot(dir.path()), dir.path()).is_empty());
    }

    #[test]
    fn strict_mode_flags_pointers_that_leave_the_checkout() {
        let dir = fixture();
        assert!(
            findings(None, &snapshot(dir.path()), dir.path()).is_empty(),
            "the harness worktree and its commondir are the checkout's own"
        );
        write(dir.path(), "sub/.git", "gitdir: /elsewhere/x\n");
        write(dir.path(), ".git/worktrees/w/commondir", "/tmp/evil\n");
        let lines = kinds(&findings(None, &snapshot(dir.path()), dir.path()));
        assert!(
            lines.contains(&"worktree-pointer sub/.git".to_owned()),
            "{lines:?}"
        );
        assert!(
            lines.contains(&"worktree-pointer .git/worktrees/w/commondir".to_owned()),
            "{lines:?}"
        );
    }

    #[test]
    fn an_included_config_is_scanned_and_its_change_is_found() {
        let dir = fixture();
        write(dir.path(), "extra.cfg", "[user]\n\tname = a\n");
        write(
            dir.path(),
            ".git/config",
            "[include]\n\tpath = ../extra.cfg\n",
        );
        let base = snapshot(dir.path());
        write(dir.path(), "extra.cfg", "[core]\n\tfsmonitor = x\n");
        let lines = kinds(&findings(Some(&base), &snapshot(dir.path()), dir.path()));
        assert!(
            lines.contains(&"git-config-key:core.fsmonitor extra.cfg".to_owned()),
            "{lines:?}"
        );
    }

    #[test]
    fn a_flood_of_files_is_a_truncation_finding_not_silence() {
        let dir = fixture();
        for i in 0..40 {
            write(dir.path(), &format!("flood/f{i}"), "x");
        }
        let snap = snapshot_with_limit(dir.path(), 10);
        assert!(snap.truncated);
        let found = findings(None, &snap, dir.path());
        assert!(
            found
                .iter()
                .any(|f| f.render() == "scan-truncated ." && f.is_git_relevant()),
            "{found:?}"
        );
        assert!(!snapshot(dir.path()).truncated);
    }

    #[test]
    fn the_baseline_is_written_once_and_a_corrupt_one_is_none() {
        use std::os::unix::fs::PermissionsExt;
        let dir = fixture();
        let state = tempfile::tempdir().expect("state");
        let first = snapshot(dir.path());
        assert!(save_baseline_if_absent(state.path(), &first).expect("save"));
        let path = state.path().join(BASELINE_FILE);
        assert_eq!(
            fs::metadata(&path).expect("meta").permissions().mode() & 0o777,
            0o600
        );
        let bytes = fs::read(&path).expect("read");
        write(dir.path(), "CLAUDE.md", "changed\n");
        assert!(!save_baseline_if_absent(state.path(), &snapshot(dir.path())).expect("again"));
        assert_eq!(
            fs::read(&path).expect("read"),
            bytes,
            "the first baseline is kept"
        );
        assert_eq!(load_baseline(state.path()).expect("load"), Some(first));
        fs::write(&path, "{ not json").expect("corrupt");
        assert_eq!(load_baseline(state.path()).expect("load"), None);
        let empty = tempfile::tempdir().expect("empty");
        assert_eq!(load_baseline(empty.path()).expect("load"), None);
    }

    #[test]
    fn the_record_is_bounded() {
        let many: Vec<Finding> = (0..25)
            .map(|i| Finding {
                path: format!("p{i:02}"),
                kind: FindingKind::Changed,
            })
            .collect();
        let lines = render_bounded(&many);
        assert_eq!(lines.len(), MAX_RECORDED);
        assert_eq!(lines.last().map(String::as_str), Some("+6 more"));
        assert_eq!(render_bounded(&many[..3]).len(), 3);
    }

    #[test]
    fn a_symlinked_tracked_file_is_recorded_as_a_link_and_never_read_through() {
        let dir = fixture();
        fs::create_dir_all(dir.path().join(".claude")).expect("dir");
        std::os::unix::fs::symlink("/etc/passwd", dir.path().join(".claude/settings.json"))
            .expect("symlink");
        let snap = snapshot(dir.path());
        assert_eq!(
            snap.files.get(".claude/settings.json"),
            Some(&Entry::Symlink("/etc/passwd".to_owned()))
        );
    }
}
