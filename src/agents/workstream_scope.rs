//! Workstream scopes for the agents scan (quick 261023-05i).
//!
//! **Read-only, names only.** Like [`super::unmerged`], this module lists
//! directory entries with `std::fs::read_dir` and nothing else: no file is
//! written, created or opened for its contents, no process is spawned and no
//! git is run.
//!
//! A *scope* is the project itself or one workstream directory under
//! `<project>/.planning/workstreams/`. The agents scan used to key everything
//! by bare phase number, so the project's Phase 16 and a workstream's Phase 16
//! were indistinguishable. This module supplies the scope vocabulary the scan
//! joins on: [`names`] (which workstreams exist), [`scope_planning`] (a
//! scope's planning directory under any base) and, for agent rows,
//! [`resolve`] (which scope a plan belongs to).

use std::path::{Path, PathBuf};

use super::unmerged;

/// The UTF-8 sub-directory names of `planning/workstreams`, sorted. Files are
/// skipped; an unreadable or missing directory gives an empty list.
///
/// Matches `load_workstreams`, which takes every sub-directory, so these names
/// equal `WorkstreamState.name`. A name is a single path component read from
/// `read_dir`, never text an agent wrote.
pub fn names(planning: &Path) -> Vec<String> {
    let mut out: Vec<String> = unmerged::subdirs(&planning.join("workstreams"))
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    out.sort();
    out
}

/// A scope's planning directory under `base` (a `.planning` directory, main's
/// or a worktree's): `base` for the project, `base/workstreams/<name>` for a
/// workstream.
pub fn scope_planning(base: &Path, workstream: Option<&str>) -> PathBuf {
    match workstream {
        None => base.to_path_buf(),
        Some(name) => base.join("workstreams").join(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_lists_workstream_directories_sorted_and_skips_files() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let ws = tmp.path().join("workstreams");
        std::fs::create_dir_all(ws.join("v0-14-voice")).unwrap();
        std::fs::create_dir_all(ws.join("v0-13-beta")).unwrap();
        std::fs::write(ws.join("README.md"), "").unwrap();
        assert_eq!(names(tmp.path()), vec!["v0-13-beta", "v0-14-voice"]);
        assert!(names(&tmp.path().join("missing")).is_empty());
    }

    #[test]
    fn scope_planning_joins_the_workstream_only_for_a_workstream() {
        let base = Path::new("/p/.planning");
        assert_eq!(scope_planning(base, None), base);
        assert_eq!(
            scope_planning(base, Some("v1")),
            Path::new("/p/.planning/workstreams/v1")
        );
    }
}
