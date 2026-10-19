//! The push step: a driven run's only way off the machine (Phase 28; LNCH-06,
//! D-24, D-25, RESEARCH §4).
//!
//! The driver commits inside its sandbox, but it holds no credential and cannot
//! reach a remote. Its commits leave through here, on the host, outside the
//! sandbox, and only through these rules:
//!
//! 1. **Never from the driven checkout.** The driver can rewrite `.git/config` and
//!    the hooks of the checkout it drives, and a read-only bind of `.git/config` is
//!    no defence because it can rename `.git`. So the push runs from a bare
//!    **mirror the launcher owns**, `<driver-root>/<hash8(project)>/mirror.git`:
//!    the tip is fetched from the checkout into a private ref of the mirror and
//!    pushed from there. The mirror has `core.hooksPath=/dev/null`, and its push
//!    URL is **pinned** (`gsdmm.pushUrl`) when the mirror is first created at a
//!    launch, before any sandbox of that launch starts. It is never read from the
//!    checkout again.
//! 2. **One refspec.** `<tip of the branch recorded at the first launch>:refs/heads/gsd-auto/<alias>/<run-id>`,
//!    the reserved namespace of Phase 19 ([`DEFAULT_NAMESPACE_ROOT`]). No `--force`,
//!    no `+`, and the destination is checked with [`classify_push_ref`] first.
//! 3. **Scanned.** The full tree of the tip is materialized from the mirror and
//!    passed through the SAFE-03 credential scan; a hit pushes nothing and parks
//!    the run.
//! 4. **Only when the run is finished** (`done`, `parked` or `stopped`) and nobody
//!    holds its lock. The launcher runs [`push_run`] once at run end while it still
//!    holds the lock; `driver push` is the same step for a finished run.
//!
//! Every `git` and `gh` this module starts goes through [`super::process`], the one
//! launcher spawn file. Synchronous, like the rest of the launcher: never call it
//! from an `async fn`.

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::Duration;

use chrono::{SecondsFormat, Utc};
use regex::Regex;
use serde_json::Value;

use super::process::run_bounded_with_stderr;
use super::LauncherError;
use crate::config::DriverOptIn;
use crate::driver::lock::ACQUIRE_RETRY_BUDGET;
use crate::driver::lock::{acquire_at_with_retry, probe_at, run_lock_path, LockProbe};
use crate::driverctl::serve::{hash8, valid_run_id, validate_run_dir, DRIVERS_REL};
use crate::driverctl::store::{PushRecord, RunStore, EXTRA_PUSH, STATE_FILE};
use crate::envelope::policy::{
    classify_push_ref, default_namespace, validate_namespace, EnvelopePolicy, PushVerdict,
    DEFAULT_NAMESPACE_ROOT, DEFAULT_PR_CAP_PER_24H, DEFAULT_PR_CAP_PER_RUN,
    REASON_PUSH_OUTSIDE_NAMESPACE, REASON_SECRET_DETECTED,
};
use crate::envelope::scan::{scan_with_external, ScanLimits};
use crate::error::LockError;
use crate::state_reader::driver_frontmatter::RunStatus;

/// The bare mirror's directory name under `<driver-root>/<hash8(project)>/`.
pub const MIRROR_DIR: &str = "mirror.git";

/// The `state.json` key holding the branch the first launch recorded.
pub const KEY_LAUNCH_BRANCH: &str = "launch_branch";

/// The `state.json` key holding the commit the first launch started from.
pub const KEY_LAUNCH_START_SHA: &str = "launch_start_sha";

/// The mirror config key that pins the push URL.
const PUSH_URL_KEY: &str = "gsdmm.pushUrl";

/// The transports a launcher `git` may use. `ext::` (run a command) is not one.
const ALLOWED_PROTOCOLS: &str = "file:git:http:https:ssh";

/// Reads and local writes (`config`, `rev-parse`, `init`, `worktree`).
const GIT_QUICK: Duration = Duration::from_secs(30);

/// Fetching the tip from the checkout into the mirror.
const GIT_FETCH: Duration = Duration::from_secs(120);

/// The push to the remote (D-25: 300 s).
const GIT_PUSH: Duration = Duration::from_secs(300);

/// What the push step did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushOutcome {
    /// The tip was sent to `dest`; `sha` is the full commit id.
    Pushed { dest: String, sha: String },
    /// The tip is the one the last push already sent: nothing was done.
    AlreadyPushed,
    /// The branch is still at the commit the run started from.
    NothingToPush,
    /// The step declined. `reason` is a stable identifier (a SAFE-03 / SAFE-01
    /// reason where one exists).
    Refused {
        reason: &'static str,
        detail: String,
    },
    /// The step tried and a `git` call failed; nothing was recorded as pushed.
    Failed { detail: String },
}

impl PushOutcome {
    /// Whether `driver push` exits `0`: a push, an already-done push or nothing to do.
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            PushOutcome::Pushed { .. } | PushOutcome::AlreadyPushed | PushOutcome::NothingToPush
        )
    }
}

impl std::fmt::Display for PushOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PushOutcome::Pushed { dest, sha } => {
                let short: String = sha.chars().take(12).collect();
                write!(f, "pushed {short} to {dest}")
            }
            PushOutcome::AlreadyPushed => {
                f.write_str("already pushed: the tip is unchanged since the last push")
            }
            PushOutcome::NothingToPush => {
                f.write_str("nothing to push: the run added no commits to its branch")
            }
            PushOutcome::Refused { reason, detail } => write!(f, "refused ({reason}): {detail}"),
            PushOutcome::Failed { detail } => write!(f, "failed: {detail}"),
        }
    }
}

/// Everything the push step needs about one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushContext {
    /// The project root, canonical (the mirror is keyed by its hash).
    pub project: PathBuf,
    /// The registry alias (the namespace segment).
    pub alias: String,
    pub run_id: String,
    /// `dirs::data_local_dir()/gsd-meta-manager/driver`.
    pub driver_root: PathBuf,
    /// `preferences.driver.open_pr`: open a pull request after a push (D-26).
    pub open_pr: bool,
    /// The PR caps (SAFE-06) the pull request is checked against.
    pub policy: EnvelopePolicy,
}

// ---------------------------------------------------------------- pure helpers

/// The envelope settings the push step runs under: the project's opt-in resolved
/// (D-30) when it has one, else the defaults with no credential. Only the PR caps
/// are read from it; the destination is always the run's own branch.
pub fn policy_for(alias: &str, opt_in: Option<&DriverOptIn>) -> EnvelopePolicy {
    match opt_in {
        Some(opt_in) => EnvelopePolicy::resolve(alias, opt_in),
        None => EnvelopePolicy {
            namespace: default_namespace(alias),
            pr_cap_per_24h: DEFAULT_PR_CAP_PER_24H,
            pr_cap_per_run: DEFAULT_PR_CAP_PER_RUN,
            credential: None,
        },
    }
}

/// The one destination a run ever pushes to: `refs/heads/gsd-auto/<alias>/<run-id>`.
pub fn dest_ref(alias: &str, run_id: &str) -> String {
    format!("{DEFAULT_NAMESPACE_ROOT}{alias}/{run_id}")
}

/// `owner/repo` of a GitHub remote URL, or `None` for any other remote.
///
/// Recognized: `git@github.com:o/r(.git)`, `ssh://git@github.com/o/r(.git)` and
/// `https://[user[:password]@]github.com/o/r(.git)`, with an optional trailing `/`.
pub fn github_repo(url: &str) -> Option<String> {
    static GITHUB: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"^(?:git@github\.com:|ssh://git@github\.com/|https://(?:[^/@\s]+@)?github\.com/)([A-Za-z0-9._-]+)/([A-Za-z0-9._-]+?)(?:\.git)?/?$",
        )
        .expect("static regex")
    });
    let caps = GITHUB.captures(url.trim())?;
    let (owner, repo) = (&caps[1], &caps[2]);
    if owner.starts_with('.') || repo.starts_with('.') || repo.is_empty() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

/// Whether `dest` is allowed for `alias`: the SAFE-01 namespace refusal of Phase 19,
/// reused. `Err` carries the D-24 reason.
pub fn check_destination(dest: &str, alias: &str) -> Result<(), &'static str> {
    let namespace = default_namespace(alias);
    if validate_namespace(&namespace).is_none() {
        return Err(REASON_PUSH_OUTSIDE_NAMESPACE);
    }
    match classify_push_ref(dest, &namespace) {
        PushVerdict::Allow => Ok(()),
        PushVerdict::Refuse { reason, .. } => Err(reason),
    }
}

/// A branch name safe to put inside a fetch refspec: what git itself would accept
/// from `symbolic-ref`, minus every character that could end the ref or start an
/// option.
fn plausible_branch(branch: &str) -> bool {
    !branch.is_empty()
        && branch.len() <= 200
        && !branch.starts_with(['-', '+', '/'])
        && !branch.contains("..")
        && !branch.contains("@{")
        && !branch
            .chars()
            .any(|c| c.is_control() || " :?*[\\^~".contains(c))
}

/// A remote URL that may be pinned and pushed to: not option-shaped, not the `ext::`
/// command transport, one line.
fn plausible_url(url: &str) -> bool {
    !url.is_empty()
        && url.len() <= 2048
        && !url.starts_with('-')
        && !url.to_ascii_lowercase().starts_with("ext::")
        && !url.chars().any(char::is_control)
}

/// `text` with the user information of every URL in it removed
/// (`https://user:token@host/x` becomes `https://host/x`). git's own error lines
/// quote the URL, and the push URL may carry a credential.
pub fn redact_urls(text: &str) -> String {
    static USERINFO: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"://[^/@\s]+@").expect("static regex"));
    USERINFO.replace_all(text, "://").into_owned()
}

fn is_hex_sha(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.chars().all(|c| c.is_ascii_hexdigit())
}

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

// ---------------------------------------------------------------- git

/// What one `git` call came to.
struct GitRun {
    success: bool,
    stdout: String,
    /// The last non-empty stderr line, or why the call produced none.
    last_error: String,
}

/// Run `git` with `args` (they name their repository), within `budget`.
///
/// `GIT_TERMINAL_PROMPT=0` so a credential prompt cannot hang the launcher, and
/// `GIT_ALLOW_PROTOCOL` so the `ext::` transport is out of reach.
fn git(args: Vec<OsString>, budget: Duration) -> GitRun {
    match run_bounded_with_stderr(
        "git",
        &args,
        None,
        &[
            ("GIT_TERMINAL_PROMPT", "0"),
            ("GIT_ALLOW_PROTOCOL", ALLOWED_PROTOCOLS),
        ],
        budget,
    ) {
        Some((out, line)) => GitRun {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).trim().to_owned(),
            last_error: redact_urls(&line),
        },
        None => GitRun {
            success: false,
            stdout: String::new(),
            last_error: format!("git did not start or did not finish within {budget:?}"),
        },
    }
}

/// `git --git-dir <mirror> <args...>`.
fn git_mirror(mirror: &Path, args: &[&str], budget: Duration) -> GitRun {
    let mut full = vec![OsString::from("--git-dir"), mirror.as_os_str().to_owned()];
    full.extend(os(args));
    git(full, budget)
}

/// `git -C <dir> <args...>`.
fn git_in(dir: &Path, args: &[&str], budget: Duration) -> GitRun {
    let mut full = vec![OsString::from("-C"), dir.as_os_str().to_owned()];
    full.extend(os(args));
    git(full, budget)
}

// ---------------------------------------------------------------- the mirror

/// `<driver-root>/<hash8(project)>/mirror.git`.
pub fn mirror_path(driver_root: &Path, canonical_project: &Path) -> PathBuf {
    driver_root.join(hash8(canonical_project)).join(MIRROR_DIR)
}

/// Make sure the project's launcher-owned mirror exists, and return its path.
///
/// **A mirror that already exists is left as it is** (only `core.hooksPath` is
/// asserted again, which can only harden it): its pinned push URL is never
/// refreshed from the checkout, because by the second launch the checkout may have
/// been rewritten by a driver. To change remotes the user deletes the mirror
/// `[inferred — audit]`.
///
/// A new mirror is `git init --bare` with `core.hooksPath=/dev/null` and, when the
/// checkout has a usable `remote.origin.url`, that URL pinned as `gsdmm.pushUrl`.
/// The caller runs this at launch start, before any sandbox of the launch exists:
/// the checkout is the user's until the driver gets it.
pub fn prepare_mirror(
    driver_root: &Path,
    canonical_project: &Path,
) -> Result<PathBuf, LauncherError> {
    let mirror = mirror_path(driver_root, canonical_project);
    let io_error = |context: &str, detail: String| LauncherError::Io {
        detail: format!("{context}: {detail}"),
    };
    if !mirror.join("HEAD").is_file() {
        let parent = mirror.parent().ok_or_else(|| LauncherError::Io {
            detail: "the push mirror has no parent directory".to_owned(),
        })?;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)
            .map_err(|e| io_error(&format!("create {}", parent.display()), e.to_string()))?;
        let init = git(
            vec![
                "init".into(),
                "--bare".into(),
                "--quiet".into(),
                mirror.as_os_str().to_owned(),
            ],
            GIT_QUICK,
        );
        if !init.success {
            return Err(LauncherError::Spawn {
                detail: format!("git init --bare {}: {}", mirror.display(), init.last_error),
            });
        }
        let origin = git_in(
            canonical_project,
            &["config", "--get", "remote.origin.url"],
            GIT_QUICK,
        );
        if origin.success && plausible_url(&origin.stdout) {
            let pinned = git_mirror(
                &mirror,
                &["config", PUSH_URL_KEY, &origin.stdout],
                GIT_QUICK,
            );
            if !pinned.success {
                return Err(LauncherError::Spawn {
                    detail: format!("pin the push URL: {}", pinned.last_error),
                });
            }
        }
    }
    let hooks = git_mirror(
        &mirror,
        &["config", "core.hooksPath", "/dev/null"],
        GIT_QUICK,
    );
    if !hooks.success {
        return Err(LauncherError::Spawn {
            detail: format!("disable the mirror's hooks: {}", hooks.last_error),
        });
    }
    Ok(mirror)
}

/// The branch and commit to record at the first launch: `(branch, sha)`. The
/// branch is `None` on a detached `HEAD`; the whole pair is `None` when the project
/// has no commit.
pub fn current_branch_and_sha(canonical_project: &Path) -> Option<(Option<String>, String)> {
    let head = git_in(canonical_project, &["rev-parse", "HEAD"], GIT_QUICK);
    if !head.success || !is_hex_sha(&head.stdout) {
        return None;
    }
    let branch = git_in(
        canonical_project,
        &["symbolic-ref", "--short", "-q", "HEAD"],
        GIT_QUICK,
    );
    let branch = (branch.success && plausible_branch(&branch.stdout)).then_some(branch.stdout);
    Some((branch, head.stdout))
}

// ---------------------------------------------------------------- the step

/// Log one launcher event, reporting (not failing on) a store error.
fn log_event(store: &mut RunStore, event: &'static str, kv: Vec<(&'static str, Option<String>)>) {
    if let Err(e) = store.log_launcher_event(event, kv) {
        eprintln!("driver push: could not log {event}: {e}");
    }
}

fn refused(reason: &'static str, detail: impl Into<String>) -> PushOutcome {
    PushOutcome::Refused {
        reason,
        detail: detail.into(),
    }
}

/// Remove a scan worktree (and the mirror's record of it), best effort.
fn remove_scan_dir(mirror: &Path, scan_dir: &Path) {
    if scan_dir.exists() {
        let _ = git(
            vec![
                "--git-dir".into(),
                mirror.as_os_str().to_owned(),
                "worktree".into(),
                "remove".into(),
                "--force".into(),
                scan_dir.as_os_str().to_owned(),
            ],
            GIT_QUICK,
        );
    }
    if scan_dir.exists() {
        let _ = fs::remove_dir_all(scan_dir);
    }
    let _ = git_mirror(mirror, &["worktree", "prune"], GIT_QUICK);
}

/// Send the finished run's recorded branch tip to its reserved branch.
///
/// The order is the decision; each step refuses before the next one can act:
/// finished status, recorded branch, mirror and pinned URL, fetch of the tip into a
/// private mirror ref, "nothing new" / "already sent", destination check, credential
/// scan of the tip's whole tree, then the push itself, then the record.
///
/// `store` is the run's store, held by the caller together with the run lock.
pub fn push_run(ctx: &PushContext, store: &mut RunStore) -> PushOutcome {
    let (branch, start_sha, prior) = {
        let state = store.state();
        if !matches!(
            state.status,
            RunStatus::Done | RunStatus::Parked | RunStatus::Stopped
        ) {
            return refused(
                "run_not_finished",
                format!(
                    "the run is {} (only done, parked or stopped runs are pushed)",
                    state.status.as_str()
                ),
            );
        }
        let text = |key: &str| {
            state
                .extra
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        let prior = state.extra.get(EXTRA_PUSH).map(|push| {
            let field = |name: &str| push.get(name).and_then(Value::as_str).map(str::to_owned);
            (field("sha"), field("outcome"))
        });
        (text(KEY_LAUNCH_BRANCH), text(KEY_LAUNCH_START_SHA), prior)
    };
    let Some(branch) = branch.filter(|b| plausible_branch(b)) else {
        return refused(
            "no_recorded_branch",
            "the first launch recorded no branch (a detached HEAD, or not a git repository)",
        );
    };

    let mirror = mirror_path(&ctx.driver_root, &ctx.project);
    let pinned = git_mirror(&mirror, &["config", "--get", PUSH_URL_KEY], GIT_QUICK);
    if !mirror.join("HEAD").is_file() || !pinned.success || !plausible_url(&pinned.stdout) {
        return refused(
            "no_push_remote",
            "no push remote was pinned when the mirror was created (the project had no \
             remote.origin.url at its first launch)",
        );
    }
    let url = pinned.stdout;

    // The ref names the push will use have to be real ref names before anything is
    // fetched into them.
    let dest = dest_ref(&ctx.alias, &ctx.run_id);
    let private_ref = format!("refs/gsdmm/{}", ctx.run_id);
    for name in [&dest, &private_ref] {
        let check = git_mirror(&mirror, &["check-ref-format", name], GIT_QUICK);
        if !check.success {
            return refused(
                REASON_PUSH_OUTSIDE_NAMESPACE,
                format!("{name:?} is not a valid ref name"),
            );
        }
    }
    if let Err(reason) = check_destination(&dest, &ctx.alias) {
        log_event(
            store,
            "push-refused",
            vec![
                ("reason", Some(reason.to_owned())),
                ("dest", Some(dest.clone())),
            ],
        );
        return refused(
            reason,
            format!("{dest} is outside {}", default_namespace(&ctx.alias)),
        );
    }

    let refspec = format!("+refs/heads/{branch}:{private_ref}");
    let fetch = git(
        vec![
            "--git-dir".into(),
            mirror.as_os_str().to_owned(),
            "fetch".into(),
            "--quiet".into(),
            "--no-tags".into(),
            "--no-recurse-submodules".into(),
            ctx.project.as_os_str().to_owned(),
            refspec.into(),
        ],
        GIT_FETCH,
    );
    if !fetch.success {
        return PushOutcome::Failed {
            detail: format!(
                "could not fetch branch {branch} from the checkout: {}",
                fetch.last_error
            ),
        };
    }
    let tip = git_mirror(
        &mirror,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{private_ref}^{{commit}}"),
        ],
        GIT_QUICK,
    );
    if !tip.success || !is_hex_sha(&tip.stdout) {
        return PushOutcome::Failed {
            detail: format!("the fetched tip of {branch} is not a commit"),
        };
    }
    let tip = tip.stdout;

    if start_sha.as_deref() == Some(tip.as_str()) {
        return PushOutcome::NothingToPush;
    }
    if let Some((Some(sha), Some(outcome))) = &prior {
        if sha == &tip && outcome == "pushed" {
            return PushOutcome::AlreadyPushed;
        }
    }

    // SAFE-03: the whole tree of the tip, not the diff.
    let scan_dir = mirror
        .parent()
        .unwrap_or(&ctx.driver_root)
        .join(format!("scan-{}", ctx.run_id));
    remove_scan_dir(&mirror, &scan_dir);
    let add = git(
        vec![
            "--git-dir".into(),
            mirror.as_os_str().to_owned(),
            "worktree".into(),
            "add".into(),
            "--detach".into(),
            "--force".into(),
            "--quiet".into(),
            scan_dir.as_os_str().to_owned(),
            tip.clone().into(),
        ],
        GIT_FETCH,
    );
    if !add.success {
        remove_scan_dir(&mirror, &scan_dir);
        return PushOutcome::Failed {
            detail: format!(
                "could not materialize the tip for the credential scan (nothing was pushed): {}",
                add.last_error
            ),
        };
    }
    let report = scan_with_external(&scan_dir, ScanLimits::default());
    remove_scan_dir(&mirror, &scan_dir);
    if !report.is_clean() {
        // The report cannot hold a secret (`Finding` has nowhere to put one).
        eprintln!("{}", report.render());
        if let Err(e) = store.park_after_push(&format!("push-refused:{REASON_SECRET_DETECTED}")) {
            eprintln!("driver push: could not park the run: {e}");
        }
        log_event(
            store,
            "push-refused",
            vec![
                ("reason", Some(REASON_SECRET_DETECTED.to_owned())),
                ("findings", Some(report.findings.len().to_string())),
            ],
        );
        return refused(
            REASON_SECRET_DETECTED,
            format!(
                "the credential scan found {} finding(s) in the tip; nothing was pushed and \
                 the run is parked",
                report.findings.len()
            ),
        );
    }

    // The one push: a fixed refspec, no force, from the mirror, to the pinned URL.
    let pushed = git(
        vec![
            "--git-dir".into(),
            mirror.as_os_str().to_owned(),
            "push".into(),
            "--quiet".into(),
            "--no-recurse-submodules".into(),
            url.clone().into(),
            format!("{tip}:{dest}").into(),
        ],
        GIT_PUSH,
    );
    if !pushed.success {
        let detail = redact_urls(&pushed.last_error);
        log_event(
            store,
            "push-failed",
            vec![
                ("dest", Some(dest.clone())),
                ("detail", Some(detail.clone())),
            ],
        );
        return PushOutcome::Failed {
            detail: format!("git push to {dest} failed: {detail}"),
        };
    }

    let record = PushRecord {
        at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        dest: dest.clone(),
        sha: tip.clone(),
        outcome: "pushed".to_owned(),
        pr: None,
    };
    if let Err(e) = store.record_push(&record) {
        eprintln!("driver push: the push succeeded but could not be recorded: {e}");
    }
    PushOutcome::Pushed { dest, sha: tip }
}

/// `driver push`: the same step for a run whose launcher is gone.
///
/// Refuses (`run_live`) while a launcher holds the run lock: the launcher pushes
/// once at its own run end. Otherwise takes the run lock for the push, opens the
/// store and calls [`push_run`].
pub fn push_finished_run(ctx: &PushContext) -> Result<PushOutcome, LauncherError> {
    if !valid_run_id(&ctx.run_id) {
        return Err(LauncherError::InvalidRun {
            detail: format!("run id {:?} is not a plain run id", ctx.run_id),
        });
    }
    let project = fs::canonicalize(&ctx.project).map_err(|e| LauncherError::Io {
        detail: format!("project {}: {e}", ctx.project.display()),
    })?;
    let requested = project.join(DRIVERS_REL).join(&ctx.run_id);
    let (run_dir, run_id) = validate_run_dir(&project, &requested)?;
    let live = || {
        Ok(refused(
            "run_live",
            "the run is live; the launcher pushes once at run end",
        ))
    };
    let lock_path = run_lock_path(&run_dir);
    match probe_at(&lock_path) {
        LockProbe::Held => return live(),
        LockProbe::Unknown(errno) => {
            return Err(LauncherError::Io {
                detail: format!("could not probe the run lock: {errno}"),
            })
        }
        LockProbe::Free | LockProbe::Absent => {}
    }
    if !run_dir.join(STATE_FILE).is_file() {
        return Ok(refused(
            "run_not_finished",
            "the run has never been launched",
        ));
    }
    let lock = match acquire_at_with_retry(
        &lock_path,
        &run_id,
        std::process::id(),
        ACQUIRE_RETRY_BUDGET,
    ) {
        Ok(lock) => lock,
        Err(LockError::Unavailable { detail }) => {
            return Err(LauncherError::Io {
                detail: format!("run lock: {detail}"),
            })
        }
        Err(_held) => return live(),
    };
    let mut store = RunStore::open(&run_dir, &project, lock)?;
    let ctx = PushContext {
        project,
        ..ctx.clone()
    };
    Ok(push_run(&ctx, &mut store))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `git` in `dir` through the module's own spawn path (the launcher spawns
    /// only from `process.rs`, tests included).
    fn git_ok(dir: &Path, args: &[&str]) {
        let run = git_in(dir, args, GIT_QUICK);
        assert!(run.success, "git {args:?}: {}", run.last_error);
    }

    #[test]
    fn the_destination_is_the_run_branch_in_the_reserved_namespace() {
        assert_eq!(dest_ref("p28", "r28"), "refs/heads/gsd-auto/p28/r28");
        assert!(dest_ref("a", "b").starts_with(DEFAULT_NAMESPACE_ROOT));
    }

    #[test]
    fn github_remotes_in_every_common_spelling_give_owner_and_repo() {
        for url in [
            "git@github.com:o/r.git",
            "git@github.com:o/r",
            "ssh://git@github.com/o/r.git",
            "ssh://git@github.com/o/r",
            "https://github.com/o/r.git",
            "https://github.com/o/r",
            "https://github.com/o/r/",
            "https://user:tok@github.com/o/r.git",
        ] {
            assert_eq!(github_repo(url).as_deref(), Some("o/r"), "{url}");
        }
        assert_eq!(
            github_repo("https://github.com/my-org/my.repo.git").as_deref(),
            Some("my-org/my.repo")
        );
    }

    #[test]
    fn other_remotes_are_not_github() {
        for url in [
            "file:///tmp/x.git",
            "/srv/git/x.git",
            "https://gitlab.com/o/r.git",
            "git@gitlab.com:o/r.git",
            "https://github.com.evil.example/o/r.git",
            "https://github.com/o",
            "https://github.com/o/r/extra",
            "https://notgithub.com/o/r",
            "",
        ] {
            assert_eq!(github_repo(url), None, "{url}");
        }
    }

    #[test]
    fn only_the_runs_own_branch_in_its_own_alias_namespace_is_allowed() {
        assert_eq!(
            check_destination("refs/heads/gsd-auto/p28/r28", "p28"),
            Ok(())
        );
        for refused in [
            "refs/heads/main",
            "refs/heads/master",
            "refs/heads/develop",
            "refs/heads/gsd-auto/p28",
            "refs/heads/gsd-auto/other/r28",
            "refs/heads/gsd-auto/p280/r28",
            "refs/tags/v1",
        ] {
            assert_eq!(
                check_destination(refused, "p28"),
                Err(REASON_PUSH_OUTSIDE_NAMESPACE),
                "{refused}"
            );
        }
    }

    #[test]
    fn a_branch_name_that_could_end_a_refspec_or_start_an_option_is_not_plausible() {
        assert!(plausible_branch("work"));
        assert!(plausible_branch("feature/x-1.2"));
        for bad in [
            "", "-x", "+x", "/x", "a..b", "a b", "a:b", "a?b", "a*b", "a[b", "a\\b", "a^b", "a~b",
            "a@{b", "a\nb",
        ] {
            assert!(!plausible_branch(bad), "{bad:?}");
        }
    }

    #[test]
    fn a_pinned_url_is_never_option_shaped_or_the_command_transport() {
        assert!(plausible_url("https://github.com/o/r.git"));
        assert!(plausible_url("git@github.com:o/r.git"));
        assert!(plausible_url("file:///srv/x.git"));
        for bad in ["", "-oProxyCommand=x", "ext::sh -c id", "EXT::sh", "a\nb"] {
            assert!(!plausible_url(bad), "{bad:?}");
        }
    }

    #[test]
    fn credentials_in_a_url_never_survive_redaction() {
        assert_eq!(
            redact_urls("fatal: unable to access 'https://user:ghp_secret@github.com/o/r.git/'"),
            "fatal: unable to access 'https://github.com/o/r.git/'"
        );
        assert_eq!(
            redact_urls("git@github.com:o/r.git"),
            "git@github.com:o/r.git"
        );
        assert!(!redact_urls("ssh://deploy@host/x").contains("deploy"));
    }

    #[test]
    fn a_new_mirror_pins_the_checkouts_origin_and_has_no_hooks() {
        let root = tempfile::tempdir().expect("root");
        let project = tempfile::tempdir().expect("project");
        git_ok(project.path(), &["init", "--quiet"]);
        git_ok(
            project.path(),
            &["remote", "add", "origin", "https://github.com/o/r.git"],
        );
        let mirror = prepare_mirror(root.path(), project.path()).expect("mirror");
        assert_eq!(mirror, mirror_path(root.path(), project.path()));
        let get = |key: &str| git_mirror(&mirror, &["config", "--get", key], GIT_QUICK).stdout;
        assert_eq!(get(PUSH_URL_KEY), "https://github.com/o/r.git");
        assert_eq!(get("core.hooksPath"), "/dev/null");
    }

    #[test]
    fn an_existing_mirror_keeps_its_pinned_url_when_the_checkout_is_rewritten() {
        let root = tempfile::tempdir().expect("root");
        let project = tempfile::tempdir().expect("project");
        git_ok(project.path(), &["init", "--quiet"]);
        git_ok(
            project.path(),
            &["remote", "add", "origin", "https://github.com/o/r.git"],
        );
        let mirror = prepare_mirror(root.path(), project.path()).expect("first");
        git_ok(
            project.path(),
            &["remote", "set-url", "origin", "https://evil.example/x.git"],
        );
        let again = prepare_mirror(root.path(), project.path()).expect("second");
        assert_eq!(again, mirror);
        let pinned = git_mirror(&mirror, &["config", "--get", PUSH_URL_KEY], GIT_QUICK).stdout;
        assert_eq!(pinned, "https://github.com/o/r.git");
    }

    #[test]
    fn a_project_without_a_remote_gets_a_mirror_with_no_pinned_url() {
        let root = tempfile::tempdir().expect("root");
        let project = tempfile::tempdir().expect("project");
        git_ok(project.path(), &["init", "--quiet"]);
        let mirror = prepare_mirror(root.path(), project.path()).expect("mirror");
        let pinned = git_mirror(&mirror, &["config", "--get", PUSH_URL_KEY], GIT_QUICK);
        assert!(!pinned.success, "{}", pinned.stdout);
    }

    #[test]
    fn outcomes_say_what_they_are() {
        assert!(PushOutcome::AlreadyPushed.is_success());
        assert!(PushOutcome::NothingToPush.is_success());
        assert!(PushOutcome::Pushed {
            dest: "d".into(),
            sha: "0123456789abcdef".into()
        }
        .is_success());
        assert!(!refused("x", "y").is_success());
        assert!(!PushOutcome::Failed { detail: "z".into() }.is_success());
        assert!(refused("secret_detected", "found")
            .to_string()
            .contains("secret_detected"));
    }
}
