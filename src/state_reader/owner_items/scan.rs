//! Layer A's file selection and read path: which UAT, VERIFICATION and
//! `deferred-items.md` files of the phase directories are read, and how they are read
//! safely.
//!
//! # The read path (D-17, OWN-11)
//!
//! A project may be a clone of somebody else's repository, so every name under
//! `phases/` is untrusted:
//!
//! - a file is read only when its canonical path stays inside its canonical phase
//!   directory and that directory stays inside its canonical parent (upstream's
//!   `isReportContained`): a symlink that stays inside is followed, one that
//!   escapes makes the file absent;
//! - the open is non-blocking (a FIFO cannot park the reader), the handle must be a
//!   regular file, and at most [`READ_CAP`] bytes are read, so a larger file is
//!   marked unreadable instead of being parsed or reported clean.

use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use std::sync::Arc;

use super::{
    deferred, frontmatter, uat, verification, AuditFile, AuditKind, CacheKey, ParseCache,
    ParsedFile, Scope, Unreadable,
};
use crate::state_reader::phase_num::same_phase;
use crate::state_reader::ProjectState;

/// The largest audit file read: 1 MiB.
const READ_CAP: u64 = 1 << 20;

/// The leading phase token of a directory name, `^(\d+[A-Z]?(?:\.\d+)*)` ignoring
/// case, else the whole name.
pub(super) fn dir_token(name: &str) -> String {
    let bytes = name.as_bytes();
    let mut end = bytes.iter().take_while(|b| b.is_ascii_digit()).count();
    if end == 0 {
        return name.to_owned();
    }
    if bytes.get(end).is_some_and(u8::is_ascii_alphabetic) {
        end += 1;
    }
    while bytes.get(end) == Some(&b'.') && bytes.get(end + 1).is_some_and(u8::is_ascii_digit) {
        end += 1;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
    }
    name[..end].to_owned()
}

/// Whether a name starts with a phase token: a digit, or one to three letters then
/// a digit (upstream's `derivePhaseTokenSegments` first segment).
fn has_token(name: &str) -> bool {
    let first = name.split('-').next().unwrap_or("");
    let bytes = first.as_bytes();
    if bytes.first().is_some_and(u8::is_ascii_digit) {
        return true;
    }
    let letters = bytes.iter().take_while(|b| b.is_ascii_alphabetic()).count();
    (1..=3).contains(&letters) && bytes.get(letters).is_some_and(u8::is_ascii_digit)
}

/// The legacy `isPhaseArtifact` rule: whether `file` belongs to the phase directory
/// `dir`. A directory without a token admits every file, a file without a token is
/// admitted, and otherwise the file's name must start with the directory's token
/// (padded or de-padded) followed by `-`, `.` or `_`.
fn is_phase_artifact(file: &str, dir: &str) -> bool {
    if !has_token(dir) {
        return true;
    }
    let token = dir_token(dir);
    let digits = token.bytes().take_while(u8::is_ascii_digit).count();
    let lead = &token[..digits];
    let mut candidates = vec![token.clone()];
    if digits > 0 {
        candidates.push(lead.to_owned());
        candidates.push(format!("{:0>2}{}", lead, &token[digits..]));
        if digits == token.len() {
            let depadded = lead.trim_start_matches('0');
            candidates.push(if depadded.is_empty() { "0" } else { depadded }.to_owned());
        }
    }
    let upper = file.to_ascii_uppercase();
    let hit = candidates.iter().any(|candidate| {
        upper
            .strip_prefix(&candidate.to_ascii_uppercase())
            .is_some_and(|rest| ["-", ".", "_"].iter().any(|sep| rest.starts_with(sep)))
    });
    hit || !has_token(file) || dir.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
}

/// Whether `file` is a UAT document name: contains `-UAT` and ends in `.md`.
fn is_uat_name(file: &str) -> bool {
    file.contains("-UAT") && file.ends_with(".md")
}

/// Whether the canonical `file` stays in the canonical `phase_dir`, which stays in
/// its canonical parent. Anything unresolvable is not contained.
fn contained(phase_dir: &Path, file: &Path) -> Option<PathBuf> {
    let real_dir = phase_dir.canonicalize().ok()?;
    let real_parent = phase_dir.parent()?.canonicalize().ok()?;
    let real_file = file.canonicalize().ok()?;
    (real_dir.starts_with(&real_parent) && real_file.starts_with(&real_dir)).then_some(real_file)
}

/// A regular file opened for reading, its size within the cap.
struct Opened {
    file: std::fs::File,
    modified: Option<DateTime<Utc>>,
    key: CacheKey,
}

/// Open `path` without blocking and without following a final symlink, and require a
/// regular file of at most [`READ_CAP`] bytes. Everything the cache must not
/// short-circuit happens here, on the open handle.
fn open_checked(path: &Path) -> Result<Opened, Unreadable> {
    #[cfg(unix)]
    let opened = {
        use std::os::unix::fs::OpenOptionsExt;
        let flags = rustix::fs::OFlags::NOFOLLOW.bits() | rustix::fs::OFlags::NONBLOCK.bits();
        let custom = i32::try_from(flags).map_err(|_| Unreadable::Io)?;
        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(custom)
            .open(path)
    };
    #[cfg(not(unix))]
    let opened = std::fs::File::open(path);
    let file = opened.map_err(|_| Unreadable::Io)?;
    let metadata = file.metadata().map_err(|_| Unreadable::Io)?;
    if !metadata.file_type().is_file() {
        return Err(Unreadable::NotRegular);
    }
    if metadata.len() > READ_CAP {
        return Err(Unreadable::TooLarge);
    }
    let mtime = metadata.modified().ok();
    Ok(Opened {
        file,
        modified: mtime.map(DateTime::<Utc>::from),
        key: CacheKey {
            mtime,
            len: metadata.len(),
        },
    })
}

/// The text of an opened file, at most [`READ_CAP`] bytes, line endings normalized.
fn read_text(file: std::fs::File) -> Result<String, Unreadable> {
    let mut bytes = Vec::new();
    file.take(READ_CAP + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Unreadable::Io)?;
    if bytes.len() as u64 > READ_CAP {
        return Err(Unreadable::TooLarge);
    }
    Ok(uat::normalize_line_endings(&String::from_utf8_lossy(
        &bytes,
    )))
}

/// A path relative to `root` spelled with `/`.
fn posix_relative(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// The current milestone's phase directories: real directories (a symlinked one is
/// skipped) whose token names a roadmap phase of the current milestone.
fn current_phase_dirs(phases_dir: &Path, state: &ProjectState) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(phases_dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            let token = dir_token(name);
            state.phases.iter().any(|phase| {
                same_phase(&phase.number, &token) && state.in_current_milestone(&phase.number)
            })
        })
        .collect();
    names.sort();
    names
}

/// Whether `file` is a VERIFICATION document name: contains `-VERIFICATION` and ends
/// in `.md`.
fn is_verification_name(file: &str) -> bool {
    file.contains("-VERIFICATION") && file.ends_with(".md")
}

/// The one deferred-items document name.
const DEFERRED_NAME: &str = "deferred-items.md";

/// Parse one document's text. `None` when the file contributes nothing: a
/// VERIFICATION that passed or has no usable status.
pub(super) fn parse_document(kind: AuditKind, text: &str) -> Option<ParsedFile> {
    let lines: Vec<&str> = text.split('\n').collect();
    match kind {
        AuditKind::Uat => {
            let map = frontmatter::extract(text);
            let status = frontmatter::fm_str(&map, "status")
                .filter(|status| !status.is_empty())
                .map_or_else(|| "unknown".to_owned(), |status| status.to_lowercase());
            let parse = uat::parse_uat_items_with_stats(text);
            Some(ParsedFile {
                acknowledged: frontmatter::is_acknowledged(
                    &map,
                    "gap_snapshot",
                    &frontmatter::uat_gap_snapshot(&status, text),
                ),
                source: frontmatter::source(&map),
                updated: frontmatter::updated(&map),
                unparsed_blocks: parse.headings_seen,
                unreadable: None,
                items: parse.items,
                status,
            })
        }
        AuditKind::Verification => {
            let map = frontmatter::extract(text);
            match verification::report_status(&map) {
                verification::Gate::Omit => None,
                verification::Gate::OutOfSet(value) => Some(ParsedFile {
                    status: value.clone(),
                    source: Vec::new(),
                    updated: None,
                    acknowledged: false,
                    unparsed_blocks: 0,
                    unreadable: Some(Unreadable::OutOfSetStatus(value)),
                    items: Vec::new(),
                }),
                verification::Gate::Open(status) => Some(ParsedFile {
                    acknowledged: frontmatter::is_acknowledged(&map, "status", &status),
                    source: frontmatter::source(&map),
                    updated: frontmatter::updated(&map),
                    unparsed_blocks: 0,
                    unreadable: None,
                    items: verification::parse_verification_items(&lines, &map, &status),
                    status,
                }),
            }
        }
        AuditKind::Deferred => Some(ParsedFile {
            status: "unresolved".to_owned(),
            source: Vec::new(),
            updated: None,
            acknowledged: false,
            unparsed_blocks: 0,
            unreadable: None,
            items: deferred::parse_deferred_items(&lines),
        }),
    }
}

/// One document as an [`AuditFile`]; `None` when it escapes its phase directory or,
/// for a VERIFICATION, has a status that contributes nothing.
fn read_audit(
    cache: &ParseCache,
    root: &Path,
    target: &Target,
    file_name: &str,
    kind: AuditKind,
) -> Option<AuditFile> {
    let path = target.path.join(file_name);
    let real = contained(&target.path, &path)?;
    let mut audit = AuditFile {
        rel_path: posix_relative(root, &path),
        kind,
        scope: target.scope.clone(),
        workstream: target.workstream.clone(),
        phase: dir_token(&target.dir),
        phase_dir: target.dir.clone(),
        status: "unknown".to_owned(),
        source: Vec::new(),
        updated: None,
        modified: None,
        acknowledged: false,
        unparsed_blocks: 0,
        unreadable: None,
        items: Vec::new(),
    };
    match open_checked(&real) {
        Ok(opened) => {
            audit.modified = opened.modified;
            // The cache sits after containment, the regular-file check and the size
            // check: a hostile path never gets as far as a lookup.
            let parsed = match cache.get(&real, kind, opened.key) {
                Some(hit) => hit,
                None => match read_text(opened.file) {
                    Ok(text) => {
                        let parsed = Arc::new(parse_document(kind, &text));
                        cache.insert(real, kind, opened.key, Arc::clone(&parsed));
                        parsed
                    }
                    Err(reason) => {
                        audit.unreadable = Some(reason);
                        return Some(audit);
                    }
                },
            };
            let parsed = parsed.as_ref().as_ref()?;
            audit.status = parsed.status.clone();
            audit.source = parsed.source.clone();
            audit.updated = parsed.updated;
            audit.acknowledged = parsed.acknowledged;
            audit.unparsed_blocks = parsed.unparsed_blocks;
            audit.unreadable = parsed.unreadable.clone();
            audit.items = parsed.items.clone();
        }
        Err(reason) => audit.unreadable = Some(reason),
    }
    Some(audit)
}

/// One phase directory to read, with where it came from.
struct Target {
    /// The directory's name.
    dir: String,
    path: PathBuf,
    scope: Scope,
    workstream: Option<String>,
}

/// The names of the real subdirectories of `path` (a symlinked one is skipped),
/// sorted ascending.
fn subdirectories(path: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

/// The version of a `v<digits and dots>-phases` directory name, as numeric segments
/// (`compareArchiveVersionDesc`); `None` for any other name.
fn archive_version(name: &str) -> Option<Vec<f64>> {
    let version = name.strip_prefix('v')?.strip_suffix("-phases")?;
    if version.is_empty() || !version.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }
    Some(
        version
            .split('.')
            .map(|segment| segment.parse::<f64>().unwrap_or(0.0))
            .collect(),
    )
}

/// Newest first: segments compared numerically, a missing one as 0.
fn compare_versions_desc(a: &[f64], b: &[f64]) -> std::cmp::Ordering {
    for index in 0..a.len().max(b.len()) {
        let left = a.get(index).copied().unwrap_or(0.0);
        let right = b.get(index).copied().unwrap_or(0.0);
        if left != right {
            return right
                .partial_cmp(&left)
                .unwrap_or(std::cmp::Ordering::Equal);
        }
    }
    std::cmp::Ordering::Equal
}

/// `listArchiveVersionDirs` for one planning tree: `(label, directory holding the
/// phase directories)`, flat `vX.Y-phases` directories newest first, then
/// `ws-*` directories (their phase directories one level down, under `phases/`) by
/// name, descending.
fn archive_dirs(tree: &Path) -> Vec<(String, PathBuf)> {
    let milestones = tree.join("milestones");
    if std::fs::symlink_metadata(&milestones).is_ok_and(|meta| meta.file_type().is_symlink()) {
        return Vec::new();
    }
    let names = subdirectories(&milestones);
    let mut flat: Vec<(&String, Vec<f64>)> = names
        .iter()
        .filter_map(|name| archive_version(name).map(|version| (name, version)))
        .collect();
    flat.sort_by(|a, b| compare_versions_desc(&a.1, &b.1));
    let mut out: Vec<(String, PathBuf)> = flat
        .into_iter()
        .map(|(name, _)| {
            let label = name.strip_suffix("-phases").unwrap_or(name).to_owned();
            (label, milestones.join(name))
        })
        .collect();
    let mut streams: Vec<&String> = names
        .iter()
        .filter(|name| name.starts_with("ws-"))
        .collect();
    streams.sort();
    streams.reverse();
    out.extend(
        streams
            .into_iter()
            .map(|name| (name.clone(), milestones.join(name).join("phases"))),
    );
    out
}

/// Every phase directory Layer A reads, in upstream's enumeration order: the active
/// phases of the project's own tree, then of each workstream, then every archive
/// tree (root first, then the workstreams by name descending), unfiltered by
/// milestone.
fn targets(planning_dir: &Path, state: &ProjectState) -> Vec<Target> {
    let mut out = Vec::new();
    for (workstream, scope_state) in state.scopes() {
        let tree = match workstream {
            None => planning_dir.to_path_buf(),
            Some(stream) => planning_dir.join("workstreams").join(&stream.name),
        };
        let phases_dir = tree.join("phases");
        for dir in current_phase_dirs(&phases_dir, scope_state) {
            out.push(Target {
                path: phases_dir.join(&dir),
                dir,
                scope: Scope::Current,
                workstream: workstream.map(|stream| stream.name.clone()),
            });
        }
    }
    let mut trees: Vec<(Option<String>, PathBuf)> = vec![(None, planning_dir.to_path_buf())];
    let mut streams = subdirectories(&planning_dir.join("workstreams"));
    streams.reverse();
    trees.extend(streams.into_iter().map(|name| {
        let path = planning_dir.join("workstreams").join(&name);
        (Some(name), path)
    }));
    let mut rank = 0usize;
    for (workstream, tree) in trees {
        for (label, archive) in archive_dirs(&tree) {
            let label = match &workstream {
                Some(stream) => format!("{stream}/{label}"),
                None => label,
            };
            for dir in subdirectories(&archive) {
                out.push(Target {
                    path: archive.join(&dir),
                    dir,
                    scope: Scope::Archived {
                        label: label.clone(),
                        rank,
                    },
                    workstream: workstream.clone(),
                });
            }
            rank += 1;
        }
    }
    out
}

/// Layer A over the project's own tree, its workstreams and every archive, sorted by
/// `rel_path`. The same `rel_path` reached from two targets is listed once, the first
/// in enumeration order.
pub(super) fn scan_planning(
    cache: &ParseCache,
    planning_dir: &Path,
    state: &ProjectState,
) -> Vec<AuditFile> {
    let root = planning_dir.parent().unwrap_or(planning_dir);
    let mut files = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for target in targets(planning_dir, state) {
        let Ok(entries) = std::fs::read_dir(&target.path) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        names.sort();
        let uats = names
            .iter()
            .filter(|name| is_uat_name(name) && is_phase_artifact(name, &target.dir))
            .map(|name| (name, AuditKind::Uat));
        let verifications = names
            .iter()
            .filter(|name| is_verification_name(name) && is_phase_artifact(name, &target.dir))
            .map(|name| (name, AuditKind::Verification));
        let deferred = names
            .iter()
            .filter(|name| name.as_str() == DEFERRED_NAME)
            .map(|name| (name, AuditKind::Deferred));
        for (name, kind) in uats.chain(verifications).chain(deferred) {
            if let Some(file) = read_audit(cache, root, &target, name, kind) {
                if seen.insert(file.rel_path.clone()) {
                    files.push(file);
                }
            }
        }
    }
    files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::parse_project_state;

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// A project whose roadmap lists phase 1 (`alpha`) and phase 2 (`beta`).
    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join(".planning/ROADMAP.md"),
            "# Roadmap\n\n## Phases\n\n- [ ] **Phase 1: Alpha** - a\n- [ ] **Phase 2: Beta** - b\n\n### Phase 1: Alpha\n\n**Goal**: a\n**Depends on**: Nothing\n\n### Phase 2: Beta\n\n**Goal**: b\n**Depends on**: Phase 1\n",
        );
        write(
            &dir.path().join(".planning/STATE.md"),
            "---\nstatus: in_progress\n---\n\n# State\n",
        );
        dir
    }

    const ROW: &str = "---\nstatus: testing\n---\n\n### 1. One\nresult: pending\n";

    fn scan_of(root: &Path) -> Vec<AuditFile> {
        let planning = root.join(".planning");
        crate::state_reader::owner_items::scan(&planning, &parse_project_state(&planning))
    }

    #[test]
    fn a_uat_of_a_roadmap_phase_is_read_and_others_are_not() {
        let dir = project();
        write(&dir.path().join(".planning/phases/01-alpha/01-UAT.md"), ROW);
        write(
            &dir.path().join(".planning/phases/02-beta/02-01-UAT.md"),
            ROW,
        );
        write(&dir.path().join(".planning/phases/09-gone/09-UAT.md"), ROW);
        write(
            &dir.path()
                .join(".planning/phases/01-alpha/01-VERIFICATION.md"),
            "---\nstatus: passed\n---\n",
        );
        let files = scan_of(dir.path());
        let paths: Vec<&str> = files.iter().map(|f| f.rel_path.as_str()).collect();
        assert_eq!(
            paths,
            [
                ".planning/phases/01-alpha/01-UAT.md",
                ".planning/phases/02-beta/02-01-UAT.md"
            ]
        );
        assert_eq!(files[0].phase, "01");
        assert_eq!(files[0].status, "testing");
        assert_eq!(files[0].items.len(), 1);
    }

    #[test]
    fn a_uat_of_another_phase_is_not_scoped_into_this_one_but_a_tokenless_one_is() {
        let dir = project();
        write(&dir.path().join(".planning/phases/01-alpha/02-UAT.md"), ROW);
        write(
            &dir.path().join(".planning/phases/01-alpha/HUMAN-UAT.md"),
            ROW,
        );
        write(
            &dir.path().join(".planning/phases/01-alpha/1-notes-UAT.md"),
            ROW,
        );
        let files = scan_of(dir.path());
        let names: Vec<&str> = files
            .iter()
            .map(|f| f.rel_path.rsplit('/').next().unwrap())
            .collect();
        assert_eq!(names, ["1-notes-UAT.md", "HUMAN-UAT.md"]);
    }

    #[test]
    fn the_token_rules_match_padded_and_unpadded_names() {
        assert!(is_phase_artifact("01-UAT.md", "01-alpha"));
        assert!(is_phase_artifact("1-UAT.md", "01-alpha"));
        assert!(is_phase_artifact("01-UAT.md", "1-alpha"));
        assert!(is_phase_artifact("01.1-UAT.md", "01.1-alpha"));
        assert!(!is_phase_artifact("011-UAT.md", "01-alpha"));
        assert!(!is_phase_artifact("02-UAT.md", "01-alpha"));
        assert!(is_phase_artifact("anything-UAT.md", "notoken"));
        assert!(
            is_phase_artifact("HUMAN-UAT.md", "01-alpha"),
            "no token: admitted"
        );
        assert_eq!(dir_token("12A.1-x"), "12A.1");
        assert_eq!(dir_token("notoken"), "notoken");
    }

    #[test]
    fn a_file_over_one_mib_is_unreadable_and_exactly_one_mib_is_read() {
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        let mut exact = String::from("### 1. One\nresult: pending\n");
        exact.push_str(&"x".repeat((READ_CAP as usize) - exact.len()));
        assert_eq!(exact.len() as u64, READ_CAP);
        write(&phase.join("01-UAT.md"), &exact);
        write(&phase.join("01-b-UAT.md"), &format!("{exact}y"));
        let files = scan_of(dir.path());
        let by_name = |name: &str| files.iter().find(|f| f.rel_path.ends_with(name)).unwrap();
        assert_eq!(by_name("01-UAT.md").unreadable, None);
        assert_eq!(by_name("01-UAT.md").items.len(), 1);
        assert_eq!(
            by_name("01-b-UAT.md").unreadable,
            Some(Unreadable::TooLarge)
        );
        assert!(by_name("01-b-UAT.md").items.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_contained_symlink_is_followed_and_an_escaping_one_is_absent() {
        use std::os::unix::fs::symlink;
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        write(&phase.join("01-real-UAT.md"), ROW);
        symlink(phase.join("01-real-UAT.md"), phase.join("01-UAT.md")).unwrap();
        let outside = tempfile::tempdir().unwrap();
        write(&outside.path().join("secret.md"), ROW);
        symlink(outside.path().join("secret.md"), phase.join("01-x-UAT.md")).unwrap();
        symlink(phase.join("absent.md"), phase.join("01-y-UAT.md")).unwrap();
        let files = scan_of(dir.path());
        let names: Vec<&str> = files
            .iter()
            .map(|f| f.rel_path.rsplit('/').next().unwrap())
            .collect();
        assert_eq!(names, ["01-UAT.md", "01-real-UAT.md"]);
        assert_eq!(files[0].items.len(), 1, "the contained link is read");
    }

    #[cfg(unix)]
    #[test]
    fn a_directory_named_like_a_uat_is_not_regular() {
        let dir = project();
        std::fs::create_dir_all(dir.path().join(".planning/phases/01-alpha/01-UAT.md")).unwrap();
        let files = scan_of(dir.path());
        assert_eq!(files[0].unreadable, Some(Unreadable::NotRegular));
    }

    #[test]
    fn a_symlinked_phase_directory_is_skipped() {
        #[cfg(unix)]
        {
            let dir = project();
            let real = tempfile::tempdir().unwrap();
            write(&real.path().join("01-UAT.md"), ROW);
            std::fs::create_dir_all(dir.path().join(".planning/phases")).unwrap();
            std::os::unix::fs::symlink(real.path(), dir.path().join(".planning/phases/01-alpha"))
                .unwrap();
            assert!(scan_of(dir.path()).is_empty());
        }
    }

    #[test]
    fn frontmatter_gives_status_updated_and_a_missing_one_is_unknown() {
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        write(
            &phase.join("01-UAT.md"),
            "---\nstatus: Partial\nupdated: 2026-01-10T12:00:00Z\n---\n### 1. One\nresult: pending\n",
        );
        write(&phase.join("01-b-UAT.md"), "### 1. One\nresult: pending\n");
        let files = scan_of(dir.path());
        assert_eq!(files[0].status, "partial");
        assert!(files[0].updated.is_some());
        assert_eq!(files[1].status, "unknown");
        assert!(files[1].modified.is_some());
    }

    fn by_name<'a>(files: &'a [AuditFile], name: &str) -> &'a AuditFile {
        files
            .iter()
            .find(|f| f.rel_path.ends_with(name))
            .unwrap_or_else(|| panic!("{name} was not scanned: {files:?}"))
    }

    #[test]
    fn a_file_of_exactly_one_mib_is_read() {
        let dir = project();
        let mut text = String::from(ROW);
        text.push_str(&"x".repeat(READ_CAP as usize - text.len()));
        assert_eq!(text.len() as u64, READ_CAP);
        write(
            &dir.path().join(".planning/phases/01-alpha/01-UAT.md"),
            &text,
        );
        let files = scan_of(dir.path());
        assert_eq!(files[0].unreadable, None);
        assert_eq!(files[0].items.len(), 1);
    }

    #[test]
    fn a_file_one_byte_over_the_cap_is_too_large() {
        let dir = project();
        let mut text = String::from(ROW);
        text.push_str(&"x".repeat(READ_CAP as usize + 1 - text.len()));
        assert_eq!(text.len() as u64, READ_CAP + 1);
        write(
            &dir.path().join(".planning/phases/01-alpha/01-UAT.md"),
            &text,
        );
        let files = scan_of(dir.path());
        assert_eq!(files[0].unreadable, Some(Unreadable::TooLarge));
        assert!(files[0].items.is_empty(), "never parsed, never clean");
    }

    #[cfg(unix)]
    #[test]
    fn a_sibling_dir_sharing_the_prefix_is_outside_the_phase_dir() {
        use std::os::unix::fs::symlink;
        let dir = project();
        let phases = dir.path().join(".planning/phases");
        write(&phases.join("01-alphabet/01-UAT.md"), ROW);
        std::fs::create_dir_all(phases.join("01-alpha")).unwrap();
        symlink(
            phases.join("01-alphabet/01-UAT.md"),
            phases.join("01-alpha/01-link-UAT.md"),
        )
        .unwrap();
        let files = scan_of(dir.path());
        let paths: Vec<&str> = files.iter().map(|f| f.rel_path.as_str()).collect();
        assert_eq!(
            paths,
            [".planning/phases/01-alphabet/01-UAT.md"],
            "the link out of 01-alpha is absent, not followed"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn an_escaping_symlink_to_a_fifo_is_absent_not_unregular() {
        use crate::state_reader::driver_frontmatter::fifo_test_support::{mkfifo, within_5s};
        let dir = project();
        let outside = tempfile::tempdir().unwrap();
        mkfifo(&outside.path().join("pipe"));
        let phase = dir.path().join(".planning/phases/01-alpha");
        std::fs::create_dir_all(&phase).unwrap();
        std::os::unix::fs::symlink(outside.path().join("pipe"), phase.join("01-UAT.md")).unwrap();
        let root = dir.path().to_path_buf();
        let files = within_5s(move || scan_of(&root));
        assert!(files.is_empty(), "{files:?}");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_fifo_uat_is_not_regular_and_returns_promptly() {
        use crate::state_reader::driver_frontmatter::fifo_test_support::{mkfifo, within_5s};
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        std::fs::create_dir_all(&phase).unwrap();
        mkfifo(&phase.join("01-UAT.md"));
        let root = dir.path().to_path_buf();
        let files = within_5s(move || scan_of(&root));
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].unreadable, Some(Unreadable::NotRegular));
        assert!(files[0].items.is_empty());
    }

    #[test]
    fn invalid_utf8_in_a_row_name_decodes_lossily() {
        let dir = project();
        let path = dir.path().join(".planning/phases/01-alpha/01-UAT.md");
        let mut bytes = b"### 1. Caf".to_vec();
        bytes.extend([0xff, 0xfe]);
        bytes.extend(b"\nresult: pending\n");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
        let files = scan_of(dir.path());
        assert_eq!(files[0].unreadable, None);
        assert!(files[0].items[0].name.contains('\u{fffd}'));
    }

    #[test]
    fn an_unterminated_frontmatter_reads_as_empty_and_rows_are_still_parsed() {
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        write(
            &phase.join("01-UAT.md"),
            "---\nstatus: complete\n\n### 1. One\nresult: pending\n",
        );
        write(
            &phase.join("01-b-UAT.md"),
            "---\n---\n### 1. One\nresult: pending\n",
        );
        let files = scan_of(dir.path());
        for name in ["01-UAT.md", "01-b-UAT.md"] {
            let file = by_name(&files, name);
            assert_eq!(file.status, "unknown", "{name}");
            assert_eq!(file.items.len(), 1, "{name}");
        }
    }

    #[test]
    fn a_zero_byte_uat_yields_no_items() {
        let dir = project();
        write(&dir.path().join(".planning/phases/01-alpha/01-UAT.md"), "");
        let files = scan_of(dir.path());
        assert_eq!(files[0].status, "unknown");
        assert!(files[0].items.is_empty());
        assert_eq!(files[0].unreadable, None);
        assert!(!files[0].acknowledged);
    }

    #[test]
    fn an_alias_bomb_is_refused_quickly() {
        let dir = project();
        let mut head = String::from("---\nstatus: complete\na0: &a0 [x, x, x, x, x, x, x, x, x]\n");
        for level in 1..=12 {
            let prev = level - 1;
            head.push_str(&format!(
                "a{level}: &a{level} [*a{prev}, *a{prev}, *a{prev}, *a{prev}, *a{prev}, *a{prev}, *a{prev}, *a{prev}, *a{prev}]\n"
            ));
        }
        head.push_str("---\n### 1. One\nresult: pending\n");
        write(
            &dir.path().join(".planning/phases/01-alpha/01-UAT.md"),
            &head,
        );
        let started = std::time::Instant::now();
        let files = scan_of(dir.path());
        assert!(
            started.elapsed() < std::time::Duration::from_secs(1),
            "took {:?}",
            started.elapsed()
        );
        assert_eq!(files[0].status, "unknown", "the block was refused");
        assert_eq!(files[0].items.len(), 1, "the rows are still read");
    }

    const HUMAN: &str =
        "---\nstatus: human_needed\nhuman_verification:\n  - first check\n  - second check\n---\n";

    #[test]
    fn a_human_needed_verification_yields_its_checks_and_passed_yields_nothing() {
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        write(&phase.join("01-VERIFICATION.md"), HUMAN);
        write(
            &phase.join("01-b-VERIFICATION.md"),
            "---\nstatus: passed\n---\n",
        );
        write(
            &phase.join("01-c-VERIFICATION.md"),
            "no frontmatter at all\n",
        );
        let files = scan_of(dir.path());
        assert_eq!(files.len(), 1, "{files:?}");
        assert_eq!(files[0].kind, AuditKind::Verification);
        assert_eq!(files[0].status, "human_needed");
        assert_eq!(files[0].items.len(), 2);
    }

    #[test]
    fn an_out_of_set_verification_status_degrades_that_file_only() {
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        write(
            &phase.join("01-VERIFICATION.md"),
            "---\nstatus: complete\n---\n",
        );
        write(&phase.join("01-UAT.md"), ROW);
        write(
            &phase.join("01-b-VERIFICATION.md"),
            "---\nstatus: Human_Needed\n---\n",
        );
        let files = scan_of(dir.path());
        assert_eq!(files.len(), 3);
        let bad = by_name(&files, "01-VERIFICATION.md");
        assert_eq!(
            bad.unreadable,
            Some(Unreadable::OutOfSetStatus("complete".to_owned()))
        );
        assert!(bad.items.is_empty());
        assert_eq!(
            by_name(&files, "01-b-VERIFICATION.md").unreadable,
            Some(Unreadable::OutOfSetStatus("Human_Needed".to_owned()))
        );
        assert_eq!(
            by_name(&files, "01-UAT.md").items.len(),
            1,
            "others still read"
        );
    }

    #[test]
    fn an_acknowledged_verification_keeps_its_items_and_is_flagged() {
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        write(
            &phase.join("01-VERIFICATION.md"),
            "---\nstatus: human_needed\nhuman_verification:\n  - one\naudit_acknowledged:\n  milestone: v1.0\n  at: \"2026-01-12T09:00:00Z\"\n  status: human_needed\n---\n",
        );
        write(
            &phase.join("01-b-VERIFICATION.md"),
            "---\nstatus: human_needed\nhuman_verification:\n  - one\naudit_acknowledged:\n  milestone: v1.0\n  at: \"2026-01-12T09:00:00Z\"\n  status: gaps_found\n---\n",
        );
        let files = scan_of(dir.path());
        assert!(by_name(&files, "01-VERIFICATION.md").acknowledged);
        assert_eq!(by_name(&files, "01-VERIFICATION.md").items.len(), 1);
        assert!(!by_name(&files, "01-b-VERIFICATION.md").acknowledged);
    }

    #[test]
    fn an_oversize_verification_or_deferred_file_is_unreadable_not_passed() {
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        let big = "x".repeat(READ_CAP as usize + 1);
        write(&phase.join("01-VERIFICATION.md"), &big);
        write(&phase.join("deferred-items.md"), &big);
        let files = scan_of(dir.path());
        assert_eq!(files.len(), 2);
        for file in &files {
            assert_eq!(
                file.unreadable,
                Some(Unreadable::TooLarge),
                "{}",
                file.rel_path
            );
            assert!(file.items.is_empty());
        }
        assert_eq!(
            by_name(&files, "01-VERIFICATION.md").kind,
            AuditKind::Verification
        );
        assert_eq!(
            by_name(&files, "deferred-items.md").kind,
            AuditKind::Deferred
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_fifo_verification_or_deferred_file_is_not_regular_and_returns_promptly() {
        use crate::state_reader::driver_frontmatter::fifo_test_support::{mkfifo, within_5s};
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        std::fs::create_dir_all(&phase).unwrap();
        mkfifo(&phase.join("01-VERIFICATION.md"));
        mkfifo(&phase.join("deferred-items.md"));
        let root = dir.path().to_path_buf();
        let files = within_5s(move || scan_of(&root));
        assert_eq!(files.len(), 2);
        assert!(files
            .iter()
            .all(|f| f.unreadable == Some(Unreadable::NotRegular)));
    }

    #[test]
    fn a_deferred_items_file_lists_its_open_entries_as_unresolved() {
        let dir = project();
        write(
            &dir.path()
                .join(".planning/phases/01-alpha/deferred-items.md"),
            "## Deferred Items\n\n- one\n- two\n  status: resolved\n- three\n",
        );
        write(
            &dir.path()
                .join(".planning/phases/02-beta/deferred-items.md"),
            "# Notes\n\nnothing to defer\n",
        );
        let files = scan_of(dir.path());
        let first = by_name(&files, "01-alpha/deferred-items.md");
        assert_eq!(first.kind, AuditKind::Deferred);
        assert_eq!(first.status, "unresolved");
        assert_eq!(first.items.len(), 2);
        assert!(by_name(&files, "02-beta/deferred-items.md")
            .items
            .is_empty());
    }

    #[test]
    fn an_archived_phase_and_a_current_phase_with_the_same_number_are_two_files() {
        let dir = project();
        write(&dir.path().join(".planning/phases/01-alpha/01-UAT.md"), ROW);
        write(
            &dir.path()
                .join(".planning/milestones/v0.9-phases/01-alpha/01-UAT.md"),
            ROW,
        );
        let files = scan_of(dir.path());
        assert_eq!(files.len(), 2);
        let current = by_name(&files, ".planning/phases/01-alpha/01-UAT.md");
        let archived = by_name(&files, "v0.9-phases/01-alpha/01-UAT.md");
        assert_eq!(current.scope, Scope::Current);
        assert_eq!(
            archived.scope,
            Scope::Archived {
                label: "v0.9".to_owned(),
                rank: 0
            }
        );
        assert_eq!(current.items.len(), 1);
        assert_eq!(archived.items.len(), 1);
    }

    #[test]
    fn archives_are_labeled_and_ranked_in_upstream_enumeration_order() {
        let dir = project();
        let planning = dir.path().join(".planning");
        for version in ["v1.9", "v1.10", "v1.2"] {
            write(
                &planning.join(format!("milestones/{version}-phases/01-x/01-UAT.md")),
                ROW,
            );
        }
        write(
            &planning.join("milestones/ws-a-2026/phases/01-x/01-UAT.md"),
            ROW,
        );
        write(
            &planning.join("milestones/ws-b-2026/phases/01-x/01-UAT.md"),
            ROW,
        );
        write(
            &planning.join("workstreams/zeta/milestones/v0.1-phases/01-x/01-UAT.md"),
            ROW,
        );
        write(&planning.join("milestones/notes/01-x/01-UAT.md"), ROW);
        let state = parse_project_state(&planning);
        let mut labels: Vec<(usize, String, Option<String>)> = targets(&planning, &state)
            .into_iter()
            .filter_map(|target| match target.scope {
                Scope::Archived { label, rank } => Some((rank, label, target.workstream)),
                Scope::Current => None,
            })
            .collect();
        labels.sort();
        let order: Vec<&str> = labels.iter().map(|(_, label, _)| label.as_str()).collect();
        assert_eq!(
            order,
            [
                "v1.10",
                "v1.9",
                "v1.2",
                "ws-b-2026",
                "ws-a-2026",
                "zeta/v0.1"
            ]
        );
        assert_eq!(labels.last().unwrap().2.as_deref(), Some("zeta"));
        assert_eq!(labels[0].0, 0);
    }

    #[test]
    fn a_workstreams_active_phases_are_read_with_its_own_roadmap() {
        let dir = project();
        let stream = dir.path().join(".planning/workstreams/alpha");
        write(
            &stream.join("ROADMAP.md"),
            "# Roadmap\n\n## Phases\n\n- [ ] **Phase 3: Stream** - s\n\n### Phase 3: Stream\n\n**Goal**: s\n**Depends on**: Nothing\n",
        );
        write(&stream.join("STATE.md"), "---\nstatus: in_progress\n---\n");
        write(&stream.join("phases/03-stream/03-UAT.md"), ROW);
        write(&stream.join("phases/09-gone/09-UAT.md"), ROW);
        write(&dir.path().join(".planning/phases/01-alpha/01-UAT.md"), ROW);
        let files = scan_of(dir.path());
        assert_eq!(files.len(), 2, "{files:?}");
        let ws = by_name(&files, "03-UAT.md");
        assert_eq!(ws.workstream.as_deref(), Some("alpha"));
        assert_eq!(ws.scope, Scope::Current);
        assert_eq!(by_name(&files, "01-UAT.md").workstream, None);
    }

    #[test]
    fn a_project_without_phases_or_archives_yields_nothing() {
        let dir = project();
        assert!(scan_of(dir.path()).is_empty());
        std::fs::create_dir_all(dir.path().join(".planning/phases/01-alpha")).unwrap();
        assert!(
            scan_of(dir.path()).is_empty(),
            "a phase directory with no documents"
        );
    }

    #[test]
    fn the_same_rel_path_reached_twice_is_listed_once() {
        let dir = project();
        let planning = dir.path().join(".planning");
        write(&planning.join("phases/01-alpha/01-UAT.md"), ROW);
        let state = parse_project_state(&planning);
        let mut doubled = targets(&planning, &state);
        let again = targets(&planning, &state);
        doubled.extend(again);
        let root = dir.path();
        let mut seen = std::collections::HashSet::new();
        let mut files = Vec::new();
        for target in &doubled {
            if let Some(file) = read_audit(
                &ParseCache::new(16),
                root,
                target,
                "01-UAT.md",
                AuditKind::Uat,
            ) {
                if seen.insert(file.rel_path.clone()) {
                    files.push(file);
                }
            }
        }
        assert_eq!(files.len(), 1);
        assert_eq!(scan_of(dir.path()).len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_archive_directory_is_not_entered() {
        let dir = project();
        let outside = tempfile::tempdir().unwrap();
        write(&outside.path().join("01-x/01-UAT.md"), ROW);
        let milestones = dir.path().join(".planning/milestones");
        std::fs::create_dir_all(&milestones).unwrap();
        std::os::unix::fs::symlink(outside.path(), milestones.join("v1.0-phases")).unwrap();
        assert!(scan_of(dir.path()).is_empty());
    }

    fn hits() -> usize {
        crate::state_reader::owner_items::CACHE_HITS.with(std::cell::Cell::get)
    }

    fn scan_with(cache: &ParseCache, root: &Path) -> Vec<AuditFile> {
        let planning = root.join(".planning");
        scan_planning(cache, &planning, &parse_project_state(&planning))
    }

    #[test]
    fn a_second_scan_of_an_unchanged_tree_reads_every_file_from_the_cache() {
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        write(&phase.join("01-UAT.md"), ROW);
        write(&phase.join("01-VERIFICATION.md"), HUMAN);
        write(
            &phase.join("01-b-VERIFICATION.md"),
            "---\nstatus: passed\n---\n",
        );
        write(&phase.join("deferred-items.md"), "- one\n");
        let cache = ParseCache::new(64);
        let before = hits();
        let first = scan_with(&cache, dir.path());
        assert_eq!(hits(), before, "a cold scan hits nothing");
        let second = scan_with(&cache, dir.path());
        assert_eq!(hits() - before, 4, "every document, a passed one included");
        assert_eq!(first, second);
    }

    #[test]
    fn a_changed_length_or_mtime_reparses() {
        let dir = project();
        let path = dir.path().join(".planning/phases/01-alpha/01-UAT.md");
        write(&path, ROW);
        let cache = ParseCache::new(64);
        assert_eq!(scan_with(&cache, dir.path())[0].items.len(), 1);
        write(
            &path,
            "---\nstatus: testing\n---\n\n### 1. One\nresult: pending\n### 2. Two\nresult: pending\n",
        );
        assert_eq!(
            scan_with(&cache, dir.path())[0].items.len(),
            2,
            "new length"
        );
        // Same length, different content, different mtime.
        let swapped = "---\nstatus: testing\n---\n\n### 1. One\nresult: pending\n### 2. Two\nresult: skippd\n";
        write(&path, swapped);
        let stamp = std::time::SystemTime::now() + std::time::Duration::from_secs(120);
        std::fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(stamp)
            .unwrap();
        let files = scan_with(&cache, dir.path());
        assert_eq!(files[0].items[1].result, "skippd", "new mtime");
    }

    #[test]
    fn the_same_file_read_as_two_kinds_is_cached_per_kind() {
        let dir = project();
        // Matches both the UAT and the VERIFICATION name rules.
        write(
            &dir.path().join(".planning/phases/01-alpha/01-UAT-VERIFICATION.md"),
            "---\nstatus: human_needed\nhuman_verification:\n  - x\n---\n### 1. One\nresult: pending\n",
        );
        let cache = ParseCache::new(64);
        let first = scan_with(&cache, dir.path());
        let second = scan_with(&cache, dir.path());
        assert_eq!(first, second);
    }

    #[test]
    fn evicting_a_project_drops_only_its_entries() {
        let one = project();
        let two = project();
        write(&one.path().join(".planning/phases/01-alpha/01-UAT.md"), ROW);
        write(&two.path().join(".planning/phases/01-alpha/01-UAT.md"), ROW);
        let scan_both = || {
            crate::state_reader::owner_items::scan(
                &one.path().join(".planning"),
                &parse_project_state(&one.path().join(".planning")),
            );
            crate::state_reader::owner_items::scan(
                &two.path().join(".planning"),
                &parse_project_state(&two.path().join(".planning")),
            );
        };
        scan_both();
        let before = hits();
        scan_both();
        assert_eq!(hits() - before, 2, "both are cached");
        crate::state_reader::owner_items::evict_project(one.path());
        let before = hits();
        scan_both();
        assert_eq!(hits() - before, 1, "only the other project still hits");
    }

    #[test]
    fn a_full_cache_starts_over_and_the_scan_stays_correct() {
        let dir = project();
        let phase = dir.path().join(".planning/phases/01-alpha");
        for n in 0..6 {
            write(&phase.join(format!("01-{n}-UAT.md")), ROW);
        }
        let cache = ParseCache::new(3);
        let first = scan_with(&cache, dir.path());
        assert_eq!(first.len(), 6);
        assert!(cache.len() <= 3, "{} entries", cache.len());
        assert_eq!(scan_with(&cache, dir.path()), first);
    }

    #[test]
    fn a_poisoned_cache_lock_degrades_to_an_uncached_scan() {
        let dir = project();
        write(&dir.path().join(".planning/phases/01-alpha/01-UAT.md"), ROW);
        let cache = std::sync::Arc::new(ParseCache::new(64));
        let poisoner = std::sync::Arc::clone(&cache);
        let joined = std::thread::spawn(move || {
            poisoner.hold_lock_and_panic();
        })
        .join();
        assert!(joined.is_err(), "the thread panicked holding the lock");
        let before = hits();
        let first = scan_with(&cache, dir.path());
        let second = scan_with(&cache, dir.path());
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].items.len(), 1);
        assert_eq!(first, second);
        assert_eq!(
            hits(),
            before,
            "nothing is cached while the lock is poisoned"
        );
        cache.evict_under(dir.path());
    }
}
