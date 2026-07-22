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
}
