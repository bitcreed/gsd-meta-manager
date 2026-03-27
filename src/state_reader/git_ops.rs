use std::path::Path;

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
pub async fn load_diff_stat(
    project_path: &Path,
    hash: &str,
) -> anyhow::Result<GitDiffStat> {
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
