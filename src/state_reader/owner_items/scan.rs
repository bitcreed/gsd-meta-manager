//! Layer A's file selection and read path: which UAT files of the current
//! milestone's phase directories are read, and how they are read safely.
//!
//! Only the project's own active tree is scanned here; plan 34-03 adds workstreams,
//! archives, VERIFICATION and deferred files.
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

use super::{frontmatter, uat, AuditFile, AuditKind, Scope, Unreadable};
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

/// The text and modification time of an audit file, or why it could not be read.
fn read_capped(path: &Path) -> Result<(String, Option<DateTime<Utc>>), Unreadable> {
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
    let modified = metadata.modified().ok().map(DateTime::<Utc>::from);
    if metadata.len() > READ_CAP {
        return Err(Unreadable::TooLarge);
    }
    let mut bytes = Vec::new();
    file.take(READ_CAP + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Unreadable::Io)?;
    if bytes.len() as u64 > READ_CAP {
        return Err(Unreadable::TooLarge);
    }
    Ok((
        uat::normalize_line_endings(&String::from_utf8_lossy(&bytes)),
        modified,
    ))
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

/// One UAT file as an [`AuditFile`]; `None` when it escapes its phase directory.
fn read_uat(root: &Path, phase_dir: &Path, dir: &str, file_name: &str) -> Option<AuditFile> {
    let path = phase_dir.join(file_name);
    let real = contained(phase_dir, &path)?;
    let mut audit = AuditFile {
        rel_path: posix_relative(root, &path),
        kind: AuditKind::Uat,
        scope: Scope::Current,
        workstream: None,
        phase: dir_token(dir),
        phase_dir: dir.to_owned(),
        status: "unknown".to_owned(),
        source: Vec::new(),
        updated: None,
        modified: None,
        acknowledged: false,
        unparsed_blocks: 0,
        unreadable: None,
        items: Vec::new(),
    };
    match read_capped(&real) {
        Ok((text, modified)) => {
            let map = frontmatter::extract(&text);
            audit.status = frontmatter::fm_str(&map, "status")
                .filter(|status| !status.is_empty())
                .map_or_else(|| "unknown".to_owned(), |status| status.to_lowercase());
            audit.source = frontmatter::source(&map);
            audit.updated = frontmatter::updated(&map);
            audit.modified = modified;
            let parse = uat::parse_uat_items_with_stats(&text);
            audit.items = parse.items;
            audit.unparsed_blocks = parse.headings_seen;
        }
        Err(reason) => audit.unreadable = Some(reason),
    }
    Some(audit)
}

/// Layer A over the project's own tree, sorted by `rel_path`.
pub(super) fn scan_planning(planning_dir: &Path, state: &ProjectState) -> Vec<AuditFile> {
    let root = planning_dir.parent().unwrap_or(planning_dir);
    let phases_dir = planning_dir.join("phases");
    let mut files = Vec::new();
    for dir in current_phase_dirs(&phases_dir, state) {
        let phase_dir = phases_dir.join(&dir);
        let Ok(entries) = std::fs::read_dir(&phase_dir) else {
            continue;
        };
        let mut names: Vec<String> = entries
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| is_uat_name(name) && is_phase_artifact(name, &dir))
            .collect();
        names.sort();
        for name in names {
            if let Some(file) = read_uat(root, &phase_dir, &dir, &name) {
                files.push(file);
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
        scan_planning(&planning, &parse_project_state(&planning))
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
            ROW,
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
}
