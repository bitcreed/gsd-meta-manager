//! Landing a finished one-off: merge, push or discard (C-18..C-21, I-4, I-10, I-11).
//!
//! * **merge** fast-forwards the user's checkout to the recorded head, only when the
//!   checkout is clean, on a branch and an ancestor of the head, and only after the
//!   planted-file gate found nothing git-relevant in the clone (the fetch starts
//!   `upload-pack` inside the agent's repository). The fetch is hardened; the merge
//!   itself is the user's own `git merge --ff-only`, hooks included (C-19).
//! * **push** sends the head only to `refs/heads/gsd-auto/<alias>/<run-id>`, from
//!   the launcher's mirror whose URL was pinned before any sandbox, after the SAFE-03
//!   scan, through the store-free core the driver's push uses. No pull request (I-11).
//! * **discard** deletes the run dir once the run is not live.
//!
//! Every refusal changes nothing in the checkout.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;

use crate::launcher::process::run_bounded_with_stderr;
use crate::launcher::push::{
    self, check_destination, dest_ref, mirror_path, pinned_push_url, planted, push_tip, TipPush,
    TipPushed,
};

use super::runtime::AgentRuntime;
use super::store::{
    project_dir_for, read_view, rfc3339, write_record, Landing, LandingKind, OneoffView,
    RecordStatus,
};
use super::BRANCH_PREFIX;
use super::WORK_DIR;

/// The paths whose change the merge confirmation calls out (I-10), besides the
/// agent's own project config dir ([`AgentRuntime::project_config_dir`]).
const SENSITIVE_DIRS: &[&str] = &[".github/", ".husky/"];
const SENSITIVE_FILES: &[&str] = &[".gitattributes", ".gitmodules", ".envrc"];

/// The plain `git merge --ff-only` budget.
const MERGE_BUDGET: Duration = Duration::from_secs(300);

/// Everything the merge decision looks at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeFacts {
    /// The run may still be live.
    pub live: bool,
    /// The run is finished and its head was recorded.
    pub finished_with_head: bool,
    pub commits: u32,
    /// Git-relevant planted findings in the clone.
    pub planted: usize,
    /// `git status --porcelain` printed something.
    pub dirty: bool,
    /// The checkout's branch; `None` on a detached HEAD.
    pub branch: Option<String>,
    /// The checkout's HEAD is an ancestor of the recorded head.
    pub ancestor: bool,
}

/// Whether the merge may run (pure). Each refusal has its own reason.
pub fn merge_decision(facts: &MergeFacts) -> Result<(), String> {
    if facts.live {
        return Err("the run is still live; stop it or wait for it to finish".to_owned());
    }
    if !facts.finished_with_head {
        return Err("the run has no recorded head to merge".to_owned());
    }
    if facts.commits == 0 {
        return Err("the run made no commits; there is nothing to merge".to_owned());
    }
    if facts.planted > 0 {
        return Err(format!(
            "{} git-relevant planted finding(s) in the run's clone; it is not landed (discard it)",
            facts.planted
        ));
    }
    if facts.dirty {
        return Err(
            "the checkout has uncommitted or untracked changes; commit or stash them first"
                .to_owned(),
        );
    }
    if facts.branch.is_none() {
        return Err("the checkout's HEAD is detached; check out a branch first".to_owned());
    }
    if !facts.ancestor {
        return Err(
            "not a fast-forward: the project moved since the run started; nothing was merged"
                .to_owned(),
        );
    }
    Ok(())
}

/// The changed paths the merge confirmation calls out (I-10).
pub fn sensitive_paths(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .filter(|p| {
            p.starts_with(AgentRuntime::default().project_config_dir())
                || SENSITIVE_DIRS.iter().any(|d| p.starts_with(d))
                || SENSITIVE_FILES
                    .iter()
                    .any(|f| p.as_str() == *f || p.ends_with(&format!("/{f}")))
        })
        .cloned()
        .collect()
}

/// The warning lines for `paths`.
pub fn sensitive_warning(paths: &[String]) -> Vec<String> {
    let found = sensitive_paths(paths);
    if found.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![format!(
        "warning: the run changed {} sensitive path(s); review them before you rely on them:",
        found.len()
    )];
    lines.extend(found.into_iter().map(|p| format!("  {p}")));
    lines
}

/// The view of `run_id`, or why not.
fn view_of(project: &Path, run_id: &str) -> Result<(PathBuf, OneoffView), String> {
    if !super::store::valid_oneoff_run_id(run_id) {
        return Err(format!("{run_id:?} is not a one-off run id"));
    }
    let project_dir = project_dir_for(project)
        .ok_or_else(|| "no one-off directory for this project".to_owned())?;
    let view = read_view(&project_dir, run_id)
        .ok_or_else(|| format!("no one-off run {run_id} for this project"))?;
    Ok((project_dir, view))
}

/// Git-relevant planted findings in the clone against its launch baseline (strict
/// without one). File reads only.
fn planted_count(view: &OneoffView) -> usize {
    let work = view.run_dir.join(WORK_DIR);
    let baseline = planted::load_baseline(&view.run_dir).ok().flatten();
    let found: Vec<planted::Finding> =
        planted::findings(baseline.as_ref(), &planted::snapshot(&work), &work)
            .into_iter()
            .filter(planted::Finding::is_git_relevant)
            .collect();
    for line in planted::render_bounded(&found) {
        eprintln!("task land: planted: {line}");
    }
    found.len()
}

fn git_in(dir: &Path, args: &[&str]) -> push::GitRun {
    push::git_in(dir, args, push::GIT_QUICK)
}

/// Record a landing on the run.
fn record_landing(view: &OneoffView, kind: LandingKind, detail: String) {
    let mut record = view.record.clone();
    record.landed.push(Landing {
        kind,
        at: rfc3339(Utc::now()),
        detail,
    });
    if let Err(e) = write_record(&view.run_dir, &record) {
        eprintln!("task land: the landing succeeded but could not be recorded: {e}");
    }
}

/// Fast-forward the project's checkout to the run's recorded head.
pub fn merge(project: &Path, run_id: &str) -> Result<String, String> {
    let project = fs::canonicalize(project).map_err(|e| format!("project: {e}"))?;
    let (_, view) = view_of(&project, run_id)?;
    let record = &view.record;
    let head = record.head.clone();
    let mut facts = MergeFacts {
        live: view.live.may_be_live(),
        finished_with_head: record.status == RecordStatus::Finished && head.is_some(),
        commits: record.commits.unwrap_or(0),
        planted: 0,
        dirty: false,
        branch: Some(String::new()),
        ancestor: true,
    };
    merge_decision(&facts)?;
    let head = head.unwrap_or_default();
    // I-4: the planted gate reads files only, and runs before any git touches the clone.
    facts.planted = planted_count(&view);
    let status = git_in(&project, &["status", "--porcelain"]);
    if !status.success {
        return Err(format!("git status failed: {}", status.last_error));
    }
    facts.dirty = !status.stdout.is_empty();
    let branch = git_in(&project, &["symbolic-ref", "-q", "--short", "HEAD"]);
    facts.branch =
        (branch.success && push::plausible_branch(&branch.stdout)).then_some(branch.stdout);
    merge_decision(&facts)?;

    let work = view.run_dir.join(WORK_DIR);
    let branch_ref = format!("refs/heads/{BRANCH_PREFIX}{run_id}");
    let fetch = push::git(
        vec![
            "-C".into(),
            project.as_os_str().to_owned(),
            "fetch".into(),
            "--quiet".into(),
            "--no-tags".into(),
            "--no-recurse-submodules".into(),
            work.as_os_str().to_owned(),
            branch_ref.into(),
        ],
        push::GIT_FETCH,
    );
    if !fetch.success {
        return Err(format!(
            "could not fetch the run's branch: {}",
            fetch.last_error
        ));
    }
    let fetched = git_in(
        &project,
        &["rev-parse", "--verify", "--quiet", "FETCH_HEAD^{commit}"],
    );
    if !fetched.success || fetched.stdout != head {
        return Err("the fetched tip is not the recorded head; nothing was merged".to_owned());
    }
    let ancestor = git_in(&project, &["merge-base", "--is-ancestor", "HEAD", &head]);
    facts.ancestor = ancestor.success;
    merge_decision(&facts)?;

    // C-19: the user's own git, hooks included.
    let args: Vec<OsString> = [
        "-C",
        &project.to_string_lossy(),
        "merge",
        "--ff-only",
        "--quiet",
        &head,
    ]
    .iter()
    .map(OsString::from)
    .collect();
    match run_bounded_with_stderr(
        "git",
        &args,
        None,
        &[("GIT_TERMINAL_PROMPT", "0")],
        MERGE_BUDGET,
    ) {
        Some((out, _)) if out.status.success() => {}
        Some((_, line)) => {
            return Err(format!(
                "git merge --ff-only failed: {}",
                push::redact_urls(&line)
            ))
        }
        None => return Err("git merge did not finish".to_owned()),
    }
    let branch = facts.branch.unwrap_or_default();
    record_landing(&view, LandingKind::Merged, branch.clone());
    let mut lines = vec![format!(
        "merged {} commit(s) of {run_id} into {branch} (now at {})",
        facts.commits,
        &head[..head.len().min(12)]
    )];
    lines.extend(sensitive_warning(&record.changed_files));
    Ok(lines.join("\n"))
}

/// Push the run's recorded head to `refs/heads/gsd-auto/<alias>/<run-id>`.
pub fn push(
    project: &Path,
    alias: &str,
    run_id: &str,
    driver_root: &Path,
) -> Result<String, String> {
    let project = fs::canonicalize(project).map_err(|e| format!("project: {e}"))?;
    let (_, view) = view_of(&project, run_id)?;
    let record = &view.record;
    if view.live.may_be_live() {
        return Err("the run is still live; stop it or wait for it to finish".to_owned());
    }
    if record.status != RecordStatus::Finished || record.head.is_none() {
        return Err("the run has no recorded head to push".to_owned());
    }
    if record.commits.unwrap_or(0) == 0 {
        return Err("the run made no commits; there is nothing to push".to_owned());
    }
    let mirror = mirror_path(driver_root, &project);
    let url = pinned_push_url(&mirror).ok_or_else(|| {
        "no push remote was pinned when the mirror was created (the project had no \
         remote.origin.url when the run started)"
            .to_owned()
    })?;
    let dest = dest_ref(alias, run_id);
    let private_ref = format!("refs/gsdmm/{run_id}");
    for name in [&dest, &private_ref] {
        let check = push::git_mirror(&mirror, &["check-ref-format", name], push::GIT_QUICK);
        if !check.success {
            return Err(format!("{name:?} is not a valid ref name"));
        }
    }
    check_destination(&dest, alias).map_err(|reason| format!("{dest} is refused: {reason}"))?;
    let planted = planted_count(&view);
    if planted > 0 {
        return Err(format!(
            "{planted} git-relevant planted finding(s) in the run's clone; nothing was pushed (discard it)"
        ));
    }
    let work = view.run_dir.join(WORK_DIR);
    let branch = format!("{BRANCH_PREFIX}{run_id}");
    let prior = record.last_pushed_sha();
    let scan_dir = mirror
        .parent()
        .unwrap_or(driver_root)
        .join(format!("scan-{run_id}"));
    match push_tip(&TipPush {
        mirror: &mirror,
        url: &url,
        checkout: &work,
        branch: &branch,
        private_ref: &private_ref,
        dest: &dest,
        start_sha: record.base.as_deref(),
        prior_pushed_sha: prior.as_deref(),
        scan_dir: &scan_dir,
    }) {
        TipPushed::Pushed { sha } => {
            record_landing(&view, LandingKind::Pushed, format!("{dest} {sha}"));
            Ok(format!("pushed {} to {dest}", &sha[..sha.len().min(12)]))
        }
        TipPushed::AlreadyPushed => Ok(format!("already pushed: {dest} has this run's head")),
        TipPushed::NothingToPush => Ok("nothing to push: the branch is at its start".to_owned()),
        TipPushed::SecretFound { findings, rendered } => {
            eprintln!("{rendered}");
            Err(format!(
                "the credential scan found {findings} finding(s) in the tip; nothing was pushed"
            ))
        }
        TipPushed::FetchFailed { detail }
        | TipPushed::ScanFailed { detail }
        | TipPushed::PushFailed { detail } => Err(detail),
    }
}

/// Delete the run's directory, clone included, once the run is not live.
pub fn discard(project: &Path, run_id: &str) -> Result<String, String> {
    let (project_dir, view) = view_of(project, run_id)?;
    if view.live.may_be_live() {
        return Err("the run is still live; stop it first".to_owned());
    }
    let run_dir = project_dir.join(run_id);
    let meta = fs::symlink_metadata(&run_dir).map_err(|e| format!("{}: {e}", run_dir.display()))?;
    if !meta.file_type().is_dir() {
        return Err(format!("{} is not a directory", run_dir.display()));
    }
    let canonical = fs::canonicalize(&run_dir).map_err(|e| e.to_string())?;
    let parent = fs::canonicalize(&project_dir).map_err(|e| e.to_string())?;
    if canonical.parent() != Some(parent.as_path()) {
        return Err(format!(
            "{} does not sit directly under the project's one-off directory",
            run_dir.display()
        ));
    }
    fs::remove_dir_all(&canonical).map_err(|e| format!("remove {}: {e}", canonical.display()))?;
    Ok(format!("discarded {run_id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_facts() -> MergeFacts {
        MergeFacts {
            live: false,
            finished_with_head: true,
            commits: 2,
            planted: 0,
            dirty: false,
            branch: Some("main".to_owned()),
            ancestor: true,
        }
    }

    #[test]
    fn a_clean_branch_ancestor_with_commits_merges() {
        assert!(merge_decision(&ok_facts()).is_ok());
    }

    #[test]
    fn each_refusal_has_its_own_reason() {
        let cases: Vec<(MergeFacts, &str)> = vec![
            (
                MergeFacts {
                    live: true,
                    ..ok_facts()
                },
                "live",
            ),
            (
                MergeFacts {
                    commits: 0,
                    ..ok_facts()
                },
                "no commits",
            ),
            (
                MergeFacts {
                    dirty: true,
                    ..ok_facts()
                },
                "uncommitted",
            ),
            (
                MergeFacts {
                    branch: None,
                    ..ok_facts()
                },
                "detached",
            ),
            (
                MergeFacts {
                    ancestor: false,
                    ..ok_facts()
                },
                "not a fast-forward",
            ),
            (
                MergeFacts {
                    planted: 1,
                    ..ok_facts()
                },
                "planted",
            ),
            (
                MergeFacts {
                    finished_with_head: false,
                    ..ok_facts()
                },
                "no recorded head",
            ),
        ];
        for (facts, needle) in cases {
            let why = merge_decision(&facts).unwrap_err();
            assert!(why.contains(needle), "{needle}: {why}");
        }
    }

    #[test]
    fn sensitive_paths_are_called_out() {
        let agent_settings = format!(
            "{}settings.json",
            AgentRuntime::default().project_config_dir()
        );
        let paths: Vec<String> = [
            agent_settings.as_str(),
            ".github/workflows/x.yml",
            ".gitattributes",
            ".envrc",
            "src/a.rs",
            ".husky/pre-commit",
            "sub/.gitmodules",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
        let found = sensitive_paths(&paths);
        assert!(!found.contains(&"src/a.rs".to_owned()));
        assert_eq!(found.len(), 6, "{found:?}");
    }
}
