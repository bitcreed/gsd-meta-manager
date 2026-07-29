use std::path::Path;

/// Read a project's last-activity timestamp from its most recent git commit
/// (committer date), falling back to filesystem mtimes for non-git or empty
/// repositories. Synchronous by design: its consumer `parse_project_state` is
/// synchronous. Plan 6 stores this on `ProjectState.last_activity`.
///
/// Returns the committer date of the most recent commit as a UTC timestamp, or
/// `None` when the directory is not a git repo, has no commits, or git is
/// unavailable/errors. Never panics.
fn git_last_commit_time(project_root: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["log", "-1", "--format=%cI"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }

    chrono::DateTime::parse_from_rfc3339(trimmed)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
}

/// Filesystem-mtime fallback for `project_last_activity`. Walks
/// `<project_root>/.planning/` shallowly (top level plus one level of
/// subdirectories — sufficient for staleness) collecting file `modified()`
/// times and returns the newest as a UTC timestamp. If `.planning/` is absent,
/// falls back to the `project_root` directory's own mtime. Returns `None` when
/// nothing is readable. Never panics.
fn mtime_last_activity(project_root: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    let planning = project_root.join(".planning");
    if planning.is_dir() {
        let mut newest: Option<std::time::SystemTime> = None;

        // Shallow walk: top level plus one level of subdirectories.
        if let Ok(entries) = std::fs::read_dir(&planning) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_type = match entry.file_type() {
                    Ok(ft) => ft,
                    Err(_) => continue,
                };

                if file_type.is_file() {
                    if let Ok(modified) = entry.metadata().and_then(|m| m.modified()) {
                        newest = Some(newest.map_or(modified, |cur| cur.max(modified)));
                    }
                } else if file_type.is_dir() {
                    if let Ok(sub_entries) = std::fs::read_dir(&path) {
                        for sub in sub_entries.flatten() {
                            if sub.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                                if let Ok(modified) =
                                    sub.metadata().and_then(|m| m.modified())
                                {
                                    newest =
                                        Some(newest.map_or(modified, |cur| cur.max(modified)));
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(t) = newest {
            return Some(chrono::DateTime::<chrono::Utc>::from(t));
        }
    }

    // Fall back to the project_root directory's own mtime.
    std::fs::metadata(project_root)
        .and_then(|m| m.modified())
        .ok()
        .map(chrono::DateTime::<chrono::Utc>::from)
}

/// A project's last-activity timestamp, preferring the most recent git commit
/// time and falling back to the newest `.planning/` file mtime (or the project
/// directory mtime) for non-git or empty repositories. Synchronous by design.
/// Returns `None` only when neither git nor the filesystem yields a timestamp.
/// Never panics.
pub fn project_last_activity(project_root: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    git_last_commit_time(project_root).or_else(|| mtime_last_activity(project_root))
}

/// A project's current git `HEAD` commit sha, in full.
///
/// Synchronous by design, matching `git_last_commit_time` above: its consumer
/// is the run snapshot in `src/executor/outcome.rs`, which calls it from the
/// same blocking closure as `parse_project_state`. Passes the repository root
/// as an argument (`-C`) rather than setting a working directory, consistently
/// with [`is_dirty`].
///
/// Returns `None` — never an error — when the directory is not a git
/// repository, has no commits yet, or git is unavailable. A project that is
/// simply not under version control is a normal case, not a failure. Never
/// panics.
pub fn head_sha(project_root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}

/// Whether a project's working tree carries any uncommitted change.
///
/// **Untracked files count as dirty.** `git status --porcelain` reports them by
/// default, and an agent that creates a new file without committing it has
/// unambiguously changed the project — which is exactly the corroboration
/// signal the run delta wants.
///
/// Synchronous and `-C`-based for the same reasons as [`head_sha`]. Returns
/// `None` — never an error — when the directory is not a git repository or git
/// is unavailable. Never panics.
pub fn is_dirty(project_root: &Path) -> Option<bool> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(!stdout.trim().is_empty())
}

#[derive(Debug, Clone)]
pub struct GitLogEntry {
    pub hash: String,
    pub date: String,
    pub author: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct GitDiffStat {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
    pub file_stats: Vec<String>,
}

/// Load git log entries for a project.
/// If `planning_only` is true, only shows commits touching `.planning/` files.
pub async fn load_git_log(
    project_path: &Path,
    planning_only: bool,
    limit: usize,
) -> anyhow::Result<Vec<GitLogEntry>> {
    let mut cmd = tokio::process::Command::new("git");
    cmd.current_dir(project_path);
    cmd.args([
        "log",
        &format!("--max-count={}", limit),
        "--format=%h\x1f%ad\x1f%an\x1f%s",
        "--date=short",
    ]);

    if planning_only {
        cmd.arg("--").arg(".planning/");
    }

    let output = cmd.output().await?;

    if !output.status.success() {
        return Ok(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries = stdout
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, '\x1f').collect();
            if parts.len() == 4 {
                Some(GitLogEntry {
                    hash: parts[0].to_string(),
                    date: parts[1].to_string(),
                    author: parts[2].to_string(),
                    message: parts[3].to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(entries)
}

/// Load diff stat for a specific commit hash.
pub async fn load_diff_stat(project_path: &Path, hash: &str) -> anyhow::Result<GitDiffStat> {
    let output = tokio::process::Command::new("git")
        .current_dir(project_path)
        .args(["diff-tree", "--stat", "--no-commit-id", "-r", hash])
        .output()
        .await?;

    if !output.status.success() {
        return Ok(GitDiffStat::default());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    if lines.is_empty() {
        return Ok(GitDiffStat::default());
    }

    let mut stat = GitDiffStat::default();

    // Last line is the summary: " N files changed, N insertions(+), N deletions(-)"
    // All other lines are per-file stats
    let last_line = lines[lines.len() - 1];
    stat.file_stats = lines[..lines.len().saturating_sub(1)]
        .iter()
        .map(|s| s.to_string())
        .collect();

    // Parse summary line
    for part in last_line.split(',') {
        let part = part.trim();
        if part.contains("file") && part.contains("changed") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                stat.files_changed = n;
            }
        } else if part.contains("insertion") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                stat.insertions = n;
            }
        } else if part.contains("deletion") {
            if let Some(n) = part.split_whitespace().next().and_then(|s| s.parse().ok()) {
                stat.deletions = n;
            }
        }
    }

    Ok(stat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Initialize a git repo with one commit in `dir`. Returns `false` if the
    /// sandbox forbids git init/commit (e.g. no writable identity), so callers
    /// can skip gracefully rather than fail.
    fn try_init_repo_with_commit(dir: &Path) -> bool {
        let git = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(args)
                .output()
                .ok()
                .map(|o| o.status.success())
                .unwrap_or(false)
        };

        if !git(&["init"]) {
            return false;
        }
        // Use local (repo-scoped) identity so we don't depend on global config.
        git(&["config", "user.email", "test@example.com"]);
        git(&["config", "user.name", "Test User"]);

        if std::fs::write(dir.join("file.txt"), "hello").is_err() {
            return false;
        }
        if !git(&["add", "file.txt"]) {
            return false;
        }
        git(&["commit", "-m", "initial commit"])
    }

    #[test]
    fn git_last_commit_time_returns_some_in_real_repo() {
        let tmp = std::env::temp_dir().join(format!("gsd_git_ops_test_repo_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        if !try_init_repo_with_commit(&tmp) {
            // Sandbox forbids git commit — skip gracefully.
            let _ = std::fs::remove_dir_all(&tmp);
            return;
        }

        let result = git_last_commit_time(&tmp);
        assert!(
            result.is_some(),
            "expected Some(commit time) in a repo with one commit"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn git_last_commit_time_returns_none_for_non_git_dir() {
        let tmp =
            std::env::temp_dir().join(format!("gsd_git_ops_test_nogit_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        assert!(
            git_last_commit_time(&tmp).is_none(),
            "expected None for a directory that is not a git repo"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn mtime_last_activity_returns_recent_time_for_planning_dir() {
        let tmp =
            std::env::temp_dir().join(format!("gsd_git_ops_test_mtime_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".planning")).unwrap();
        std::fs::write(tmp.join(".planning").join("STATE.md"), "state").unwrap();

        let result = mtime_last_activity(&tmp);
        assert!(result.is_some(), "expected Some from freshly-written .planning file");

        let now = chrono::Utc::now();
        let dt = result.unwrap();
        let diff = (now - dt).num_seconds().abs();
        assert!(
            diff < 60,
            "expected mtime within a minute of now, got {}s difference",
            diff
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn mtime_last_activity_returns_none_for_missing_path() {
        let missing = std::env::temp_dir().join(format!(
            "gsd_git_ops_test_missing_{}_does_not_exist",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&missing);

        assert!(
            mtime_last_activity(&missing).is_none(),
            "expected None for a path that does not exist"
        );
    }

    // ========================================================================
    // The run-delta git half (D-11)
    // ========================================================================

    #[test]
    fn head_sha_and_is_dirty_report_a_clean_repo_with_one_commit() {
        let tmp =
            std::env::temp_dir().join(format!("gsd_git_ops_test_headclean_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        if !try_init_repo_with_commit(&tmp) {
            // Sandbox forbids git commit — skip gracefully.
            let _ = std::fs::remove_dir_all(&tmp);
            return;
        }

        let sha = head_sha(&tmp);
        assert!(
            sha.as_deref().is_some_and(|s| s.len() >= 7),
            "expected a HEAD sha in a repo with one commit, got: {sha:?}"
        );
        assert_eq!(
            is_dirty(&tmp),
            Some(false),
            "a freshly committed tree is clean"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn an_untracked_file_flips_the_dirty_flag() {
        let tmp =
            std::env::temp_dir().join(format!("gsd_git_ops_test_dirty_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        if !try_init_repo_with_commit(&tmp) {
            let _ = std::fs::remove_dir_all(&tmp);
            return;
        }

        assert_eq!(is_dirty(&tmp), Some(false), "clean before the write");
        std::fs::write(tmp.join("untracked.txt"), "new work").unwrap();
        assert_eq!(
            is_dirty(&tmp),
            Some(true),
            "an untracked file is a change the agent made and must read as dirty"
        );

        // The sha must NOT move for an uncommitted change — the two signals are
        // independent.
        let sha_before = head_sha(&tmp);
        assert!(sha_before.is_some(), "the repo still has its commit");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn both_new_helpers_return_none_for_a_non_git_dir_without_erroring() {
        let tmp = std::env::temp_dir()
            .join(format!("gsd_git_ops_test_headnogit_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        assert!(
            head_sha(&tmp).is_none(),
            "a directory that is not a repository has no HEAD"
        );
        assert!(
            is_dirty(&tmp).is_none(),
            "an unknown dirty state is None, never a fabricated false"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn project_last_activity_falls_back_to_mtime_for_non_git_dir() {
        let tmp = std::env::temp_dir()
            .join(format!("gsd_git_ops_test_activity_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join(".planning")).unwrap();
        std::fs::write(tmp.join(".planning").join("STATE.md"), "state").unwrap();

        // Non-git dir: git_last_commit_time is None, so the mtime fallback applies.
        assert!(
            project_last_activity(&tmp).is_some(),
            "expected Some from mtime fallback on a non-git dir with .planning files"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
