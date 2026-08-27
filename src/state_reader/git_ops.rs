use crate::text::Untrusted;
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

// ============================================================================
// The dry-run preview half (D-22)
//
// Three read-only questions a preview has to answer before an autonomous agent
// with push rights starts: what branch is checked out, what a commit from this
// working tree would capture, and what refspecs a push would produce. All three
// are answered from local reads alone.
//
// These extend this module rather than starting a second git wrapper, and they
// follow the conventions the module already documents: `-C <root>` rather than
// `current_dir`, plain values rather than `Result`, never panics. They are
// synchronous for the same reason `head_sha` is, and inherit the discipline
// `src/executor/outcome.rs:60` states: a caller on an async runtime puts them on
// a blocking thread. The dry-run path is a CLI foreground invocation (D-24), so
// it calls them directly.
// ============================================================================

/// Run `git --no-optional-locks -C <root> <args>` and hand back its raw stdout.
///
/// **`--no-optional-locks` is load-bearing, not hygiene.** `git diff` calls
/// `refresh_index_quietly()`, which will opportunistically *write* `.git/index`
/// when it finds entries that are stat-dirty but content-identical. That is a
/// git write performed by a read — precisely what D-23 promises never happens
/// during a preview — and because such a rewrite leaves the index the same
/// length, a fingerprint that compared only file sizes would not even catch it.
/// Setting the flag makes "zero git writes" true by construction rather than by
/// luck.
///
/// Raw rather than trimmed because `git diff --stat` renders aligned columns
/// with a leading space per line; trimming the blob would silently unalign the
/// first line of a preview whose whole job is to be read by a human.
///
/// `None` on a non-zero exit, an unreadable repository, or a missing git.
/// Never panics.
///
/// `pub(crate)` rather than private because [`crate::envelope::hooks`] reads the
/// staged and pushed path lists and must do it with **this** shape — the
/// `--no-optional-locks` flag and the failure-as-data return are the two
/// properties that keep a read from mutating the repository it is judging.
/// A second copy of this three-line function in the envelope would be a second
/// place for one of those properties to go missing, and it would not be the
/// spawn-seam allowlist entry that noticed.
pub(crate) fn git_read_raw(project_root: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// [`git_read_raw`] trimmed, with an empty answer reported as `None`.
///
/// git exits 0 with empty stdout for plenty of questions whose honest answer is
/// "nothing" — an unset config key under `--get-all`, a ref namespace with no
/// entries — so an empty string is folded into `None` at the one place rather
/// than at every call site.
fn git_read(project_root: &Path, args: &[&str]) -> Option<String> {
    let raw = git_read_raw(project_root, args)?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

/// One `git config --get <key>` value, or `None` when the key is unset.
fn config_get(project_root: &Path, key: &str) -> Option<String> {
    git_read(project_root, &["config", "--get", key])
}

/// Every `git config --get-all <key>` value, in the order git reports them.
///
/// Empty rather than `None` for an unset key: the caller's question is always
/// "how many are configured", and zero is a perfectly good answer.
fn config_get_all(project_root: &Path, key: &str) -> Vec<String> {
    git_read(project_root, &["config", "--get-all", key])
        .map(|out| {
            out.lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The short name of the branch `HEAD` points at.
///
/// **`None` is a real answer here, not a failure.** A detached `HEAD` genuinely
/// has no branch to push, and reporting that as "no refspec" is the honest
/// preview — the alternative, guessing a branch name, would show the user a push
/// that cannot happen. `None` equally covers an empty repository and a directory
/// that is not a repository at all. Never panics.
pub fn current_branch(project_root: &Path) -> Option<String> {
    git_read(project_root, &["symbolic-ref", "--quiet", "--short", "HEAD"])
}

/// What a commit from this working tree would capture, in git's own words.
///
/// Two lists, kept separate on purpose. `git diff --stat HEAD` renders the
/// tracked delta and **does not list untracked files at all**, so a preview that
/// merged the two — or reported only the first — would silently under-report
/// exactly the files a fresh agent is most likely to have created.
///
/// Deliberately **not** folded into [`GitDiffStat`]. That type models a *parsed*
/// numeric summary of a commit range for the dashboard; this one carries git's
/// own rendered stat lines verbatim for a human-readable preview. Parsing them
/// and re-rendering them would only add a way to be wrong.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkingTreeStat {
    /// `git diff --stat HEAD` output, one entry per line, trailing whitespace
    /// stripped and blank lines dropped. Verbatim otherwise.
    pub stat_lines: Vec<String>,
    /// Paths reported by `git ls-files --others --exclude-standard` — untracked
    /// and *not* gitignored, because a gitignored file is not something a
    /// `git add -A` would sweep up.
    pub untracked: Vec<String>,
}

/// The working-tree state a run would inherit and could commit (D-22.2).
///
/// You cannot know the diff of a command you have not run, so what is previewed
/// is the state that is actually computable: what is already modified and what
/// is already untracked. That answers the question the user has — *"if this run
/// does `git add -A && git commit`, what goes in?"*
///
/// Both reads are read-only. An empty struct — never an error — for a
/// non-repository, an empty repository, or an unavailable git. Never panics.
pub fn working_tree_stat(project_root: &Path) -> WorkingTreeStat {
    let stat_lines = git_read_raw(project_root, &["diff", "--stat", "HEAD"])
        .map(|raw| {
            raw.lines()
                .map(|line| line.trim_end().to_string())
                .filter(|line| !line.trim().is_empty())
                .collect()
        })
        .unwrap_or_default();

    let untracked = git_read_raw(
        project_root,
        &["ls-files", "--others", "--exclude-standard"],
    )
    .map(|raw| {
        raw.lines()
            .map(|line| line.trim_end().to_string())
            .filter(|line| !line.trim().is_empty())
            .collect()
    })
    .unwrap_or_default();

    WorkingTreeStat {
        stat_lines,
        untracked,
    }
}

/// What a push from this repository would send, as far as local config knows.
///
/// Every field is advisory *preview* data. `refspecs` empty with `note` set is a
/// complete and honest answer — "nothing would be pushed, and here is why" — and
/// is never the same thing as "we could not work it out".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PushPreview {
    /// The remote a push would target, or `None` when none could be resolved.
    pub remote: Option<String>,
    /// That remote's URL **verbatim**, never normalised or rewritten — the user
    /// needs to recognise their own remote.
    pub url: Option<String>,
    /// Fully-qualified `<src>:<dst>` lines, never a bare branch name: the whole
    /// point of the preview is that the destination is visible.
    pub refspecs: Vec<String>,
    /// Why the list looks the way it does, when that is not self-evident.
    pub note: Option<String>,
}

/// The refspecs a `git push` from this state would produce — computed
/// **entirely from local git config, never by invoking git's push subcommand in
/// any form, including `--dry-run`** (D-22.3).
///
/// That prohibition is the first thing stated here because it is the single most
/// likely thing a later reader will "improve". `git push --dry-run` contacts the
/// network, needs credentials, and makes the preview non-deterministic and
/// untestable in CI — three properties this function exists to avoid.
///
/// The computation, in order:
///
/// 1. [`current_branch`]. A detached `HEAD` has no refspec, with a note saying so.
/// 2. The remote: `branch.<b>.remote`, then `remote.pushDefault`, then `origin`
///    **only if** `git remote` actually lists it. An unresolvable remote yields
///    `remote: None` and a note — guessing a remote that does not exist would
///    report a push that cannot happen.
/// 3. The URL, verbatim.
/// 4. `remote.<r>.push`, which when configured is **authoritative** and ends the
///    computation; `push.default` is not consulted.
/// 5. Otherwise `push.default`, defaulting to `simple`.
///
/// Never panics; every failure degrades to a note.
pub fn push_refspecs(project_root: &Path) -> PushPreview {
    let mut preview = PushPreview::default();

    let Some(branch) = current_branch(project_root) else {
        preview.note = Some(
            "HEAD is detached (or this is not a repository with commits), so there is no \
             branch to push and no refspec to preview."
                .to_string(),
        );
        return preview;
    };

    let remote = config_get(project_root, &format!("branch.{branch}.remote"))
        .or_else(|| config_get(project_root, "remote.pushDefault"))
        .or_else(|| {
            // `origin` is a convention, not a guarantee, so it is only used when
            // the repository actually has a remote by that name.
            let remotes = git_read(project_root, &["remote"])?;
            remotes
                .lines()
                .any(|line| line.trim() == "origin")
                .then(|| "origin".to_string())
        });

    let Some(remote) = remote else {
        preview.note = Some(format!(
            "No remote is configured for branch `{branch}` (no `branch.{branch}.remote`, no \
             `remote.pushDefault`, and no remote named `origin`), so a push from this state \
             would fail."
        ));
        return preview;
    };

    preview.url = git_read(project_root, &["remote", "get-url", &remote]);
    preview.remote = Some(remote.clone());

    let explicit = config_get_all(project_root, &format!("remote.{remote}.push"));
    if !explicit.is_empty() {
        preview.note = Some(format!(
            "These refspecs come from `remote.{remote}.push`, which is authoritative — \
             `push.default` is not consulted when it is set."
        ));
        preview.refspecs = explicit;
        return preview;
    }

    // `simple` has been git's own default since 2.0, so this is git's documented
    // behaviour for an unset key rather than a guess of ours.
    let mode = config_get(project_root, "push.default").unwrap_or_else(|| "simple".to_string());
    let upstream = config_get(project_root, &format!("branch.{branch}.merge"));
    let upstream_short = upstream
        .as_deref()
        .map(|reference| reference.trim_start_matches("refs/heads/").to_string());

    match mode.as_str() {
        "simple" => match (&upstream, &upstream_short) {
            (Some(reference), Some(short)) if *short == branch => {
                preview.refspecs = vec![format!("refs/heads/{branch}:{reference}")];
                preview.note = Some(format!(
                    "`push.default` is `simple` and the upstream `{reference}` has the same \
                     name as the local branch, which is the case `simple` allows."
                ));
            }
            (Some(reference), _) => {
                preview.note = Some(format!(
                    "`push.default` is `simple` and the upstream is `{reference}`, whose name \
                     differs from the local branch `{branch}`. `simple` refuses this case — \
                     that refusal is the safety property the mode exists for — so nothing \
                     would be pushed."
                ));
            }
            (None, _) => {
                preview.refspecs = vec![format!("refs/heads/{branch}:refs/heads/{branch}")];
                preview.note = Some(format!(
                    "`push.default` is `simple` and branch `{branch}` has no configured \
                     upstream, so this is the central-workflow arm: the same name on the \
                     remote."
                ));
            }
        },
        // `tracking` is git's deprecated alias for `upstream`; both are accepted
        // in a config file, so both are understood here.
        "upstream" | "tracking" => match &upstream {
            Some(reference) => {
                preview.refspecs = vec![format!("refs/heads/{branch}:{reference}")];
                preview.note = Some(format!(
                    "`push.default` is `{mode}`, so the push follows branch `{branch}`'s \
                     configured upstream."
                ));
            }
            None => {
                preview.note = Some(format!(
                    "`push.default` is `{mode}` but branch `{branch}` has no configured \
                     upstream, so a push would be refused and nothing would be sent."
                ));
            }
        },
        "current" => {
            preview.refspecs = vec![format!("refs/heads/{branch}:refs/heads/{branch}")];
            preview.note = Some(format!(
                "`push.default` is `current`, so branch `{branch}` goes to the same name on \
                 `{remote}` regardless of any upstream."
            ));
        }
        "matching" => {
            let locals = git_read(
                project_root,
                &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
            )
            .map(|out| {
                out.lines()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

            let remote_prefix = format!("refs/remotes/{remote}/");
            let tracked = git_read(
                project_root,
                &[
                    "for-each-ref",
                    "--format=%(refname)",
                    remote_prefix.trim_end_matches('/'),
                ],
            )
            .map(|out| {
                out.lines()
                    .filter_map(|line| line.trim().strip_prefix(&remote_prefix).map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

            preview.refspecs = locals
                .iter()
                .filter(|local| tracked.iter().any(|name| name == *local))
                .map(|local| format!("refs/heads/{local}:refs/heads/{local}"))
                .collect();
            preview.note = Some(format!(
                "`push.default` is `matching`, which pushes **every** local branch that also \
                 exists on `{remote}` — not only the current one. That is the blast radius, \
                 so all of them are listed."
            ));
        }
        "nothing" => {
            preview.note = Some(
                "`push.default` is `nothing`, so a bare `git push` sends no refspec at all."
                    .to_string(),
            );
        }
        other => {
            // Forward compatibility, the same posture the config and journal
            // readers take: carry an unrecognised value through verbatim rather
            // than failing on it.
            preview.note = Some(format!(
                "`push.default` is `{other}`, which this build does not recognise, so no \
                 refspec is previewed. The value is reported verbatim rather than guessed at."
            ));
        }
    }

    preview
}

/// One row of a THIRD-PARTY repository's `git log`, in a type that cannot reach
/// a terminal cell unescaped.
///
/// **Every field is [`crate::text::Untrusted`], and that is the mechanism**
/// (D-21-10, T-21-23-01). This tool exists to watch other people's
/// repositories, so a commit subject here is attacker-controlled in the
/// strongest sense available — and a commit subject is the canonical Trojan
/// Source carrier (CVE-2021-42574). Until round 9 these were bare `String`s
/// rendered raw through a `List`/`ListItem`, which (measured per widget family)
/// PRESERVES `U+202E` and `U+00AD` all the way into a cell.
///
/// The retype is what makes the render sites compile errors rather than sites a
/// reader has to find: `Untrusted` implements no `Display`, `AsRef<str>`,
/// `Deref`, `Borrow<str>` or `Into<Cow<'_, str>>`, so `Span::raw(&entry.message)`
/// does not compile and `entry.message.shown()` is the only ergonomic
/// resolution. Where a field is genuinely wanted raw — the `git show` argument —
/// the call reads `as_raw_for_logic_only()` and is visible in a diff.
///
/// **Public-API note (D-21-10, `costly`).** These fields are `pub` on a crate
/// published to crates.io, so this is a semver-breaking change and lands in the
/// next minor. Fully revertible from git; no on-disk format changes.
#[derive(Debug, Clone)]
pub struct GitLogEntry {
    pub hash: crate::text::Untrusted,
    pub date: crate::text::Untrusted,
    pub author: crate::text::Untrusted,
    pub message: crate::text::Untrusted,
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
                // THE one producer. Wrapping here rather than at each consumer
                // is what makes the carrier's guarantee structural: there is no
                // other route from `git log` stdout into a `GitLogEntry`.
                Some(GitLogEntry {
                    hash: Untrusted::from_untrusted_source(parts[0].to_string()),
                    date: Untrusted::from_untrusted_source(parts[1].to_string()),
                    author: Untrusted::from_untrusted_source(parts[2].to_string()),
                    message: Untrusted::from_untrusted_source(parts[3].to_string()),
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

    // ========================================================================
    // The dry-run preview half (D-22)
    //
    // Each test builds a real temporary repository and sets the specific config
    // keys under test with `git -C <root> config`, never by hand-editing
    // `.git/config` — the point is to exercise the same reader git itself would.
    // ========================================================================

    /// Run one git command in `dir`, reporting only whether it succeeded.
    fn git_ok(dir: &Path, args: &[&str]) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .ok()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    /// A repository with one commit, or `None` when the sandbox forbids git.
    fn repo_with_commit() -> Option<tempfile::TempDir> {
        let dir = tempfile::TempDir::new().ok()?;
        if !try_init_repo_with_commit(dir.path()) {
            return None;
        }
        Some(dir)
    }

    #[test]
    fn push_refspecs_renders_the_upstream_pair_under_push_default_simple() {
        let Some(repo) = repo_with_commit() else {
            return;
        };
        let root = repo.path();
        let branch = current_branch(root).expect("a fresh repo with a commit is on a branch");

        assert!(git_ok(
            root,
            &["remote", "add", "origin", "https://example.invalid/demo.git"]
        ));
        assert!(git_ok(
            root,
            &["config", &format!("branch.{branch}.remote"), "origin"]
        ));
        assert!(git_ok(
            root,
            &[
                "config",
                &format!("branch.{branch}.merge"),
                &format!("refs/heads/{branch}"),
            ]
        ));
        assert!(git_ok(root, &["config", "push.default", "simple"]));

        let preview = push_refspecs(root);

        assert_eq!(
            preview.remote.as_deref(),
            Some("origin"),
            "the remote is read from branch.{branch}.remote"
        );
        assert_eq!(
            preview.url.as_deref(),
            Some("https://example.invalid/demo.git"),
            "the URL is carried verbatim so the user recognises their own remote"
        );
        assert_eq!(
            preview.refspecs,
            vec![format!("refs/heads/{branch}:refs/heads/{branch}")],
            "a same-named upstream under `simple` renders the fully-qualified pair"
        );
        assert!(
            preview.refspecs[0].starts_with("refs/heads/") && preview.refspecs[0].contains(':'),
            "never a bare branch name — the destination has to be visible"
        );
    }

    #[test]
    fn push_refspecs_reports_a_detached_head_as_having_no_refspec() {
        let Some(repo) = repo_with_commit() else {
            return;
        };
        let root = repo.path();
        assert!(git_ok(
            root,
            &["remote", "add", "origin", "https://example.invalid/demo.git"]
        ));
        assert!(git_ok(root, &["checkout", "--detach"]));

        assert!(
            current_branch(root).is_none(),
            "a detached HEAD has no branch, and None is the honest answer"
        );

        let preview = push_refspecs(root);
        assert!(
            preview.refspecs.is_empty(),
            "there is no branch to push, so there is no refspec: {:?}",
            preview.refspecs
        );
        assert!(
            preview.note.is_some(),
            "an empty list without a reason reads as a missing section"
        );
    }

    #[test]
    fn push_refspecs_reports_no_remote_rather_than_guessing_origin() {
        let Some(repo) = repo_with_commit() else {
            return;
        };
        let root = repo.path();
        // Deliberately no `git remote add`.

        let preview = push_refspecs(root);
        assert_eq!(
            preview.remote, None,
            "guessing `origin` when no remote exists would report a push that cannot happen"
        );
        assert!(preview.url.is_none(), "no remote means no URL");
        assert!(preview.refspecs.is_empty(), "and no refspec");
        assert!(
            preview
                .note
                .as_deref()
                .is_some_and(|note| note.contains("no remote") || note.contains("No remote")),
            "the note has to say a push would fail, got: {:?}",
            preview.note
        );
    }

    #[test]
    fn push_refspecs_prefers_an_explicit_remote_push_refspec_over_push_default() {
        let Some(repo) = repo_with_commit() else {
            return;
        };
        let root = repo.path();
        let branch = current_branch(root).expect("a fresh repo with a commit is on a branch");

        assert!(git_ok(
            root,
            &["remote", "add", "origin", "https://example.invalid/demo.git"]
        ));
        assert!(git_ok(
            root,
            &["config", &format!("branch.{branch}.remote"), "origin"]
        ));
        assert!(git_ok(
            root,
            &[
                "config",
                &format!("branch.{branch}.merge"),
                &format!("refs/heads/{branch}"),
            ]
        ));
        // `simple` would have rendered `<b>:<b>`; the explicit refspec must win.
        assert!(git_ok(root, &["config", "push.default", "simple"]));
        assert!(git_ok(
            root,
            &[
                "config",
                "--add",
                "remote.origin.push",
                &format!("refs/heads/{branch}:refs/heads/deploy"),
            ]
        ));

        let preview = push_refspecs(root);
        assert_eq!(
            preview.refspecs,
            vec![format!("refs/heads/{branch}:refs/heads/deploy")],
            "an explicit remote.<r>.push is authoritative and push.default is not consulted"
        );
        assert!(
            preview
                .note
                .as_deref()
                .is_some_and(|note| note.contains("remote.origin.push")),
            "the note has to say where the refspecs came from, got: {:?}",
            preview.note
        );
    }

    #[test]
    fn working_tree_stat_lists_untracked_files_separately_from_the_stat() {
        let Some(repo) = repo_with_commit() else {
            return;
        };
        let root = repo.path();

        std::fs::write(root.join("file.txt"), "hello, and then some more").unwrap();
        std::fs::write(root.join("untracked.txt"), "brand new").unwrap();

        let stat = working_tree_stat(root);
        let rendered = stat.stat_lines.join("\n");

        assert!(
            rendered.contains("file.txt"),
            "the tracked modification belongs in the stat, got: {rendered:?}"
        );
        assert!(
            !rendered.contains("untracked.txt"),
            "`git diff --stat` does not list untracked files, and this test is what proves \
             the preview does not silently rely on it doing so; got: {rendered:?}"
        );
        assert_eq!(
            stat.untracked,
            vec!["untracked.txt".to_string()],
            "the untracked list is where a file a `git add -A` would sweep up shows up"
        );
    }

    #[test]
    fn working_tree_stat_is_empty_rather_than_erroring_for_a_non_git_dir() {
        let tmp = tempfile::TempDir::new().expect("temp dir");

        let stat = working_tree_stat(tmp.path());
        assert_eq!(
            stat,
            WorkingTreeStat::default(),
            "a directory that is not a repository has an empty preview, never an error"
        );
        assert!(
            current_branch(tmp.path()).is_none(),
            "and no branch either"
        );
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
