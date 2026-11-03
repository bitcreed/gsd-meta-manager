//! Resuming a paused, stopped or killed one-off (quick 261103-klf, L-3, L-4, C-09..C-12,
//! I-17..I-24).
//!
//! A resume never reopens a run: it starts a NEW run id (new run dir, lock, stream,
//! budget) whose clone begins at the old run's head (`--from branch`) or at a host-made
//! merge of the checkout's tip into that head (`--from master`, a sandboxed resolver on
//! conflict). The new session is a FRESH one primed with the old task, report and commit
//! log (C-10); the old record gets `resumed_by`, the new one `resumed_from`. The pure
//! half (refusals, mode, budget) is here ungated; the serialized start is linux/macos.
//! Nothing here names the agent.

use super::merge::MIN_RESOLVER_BUDGET_USD;
use super::store::{BranchState, LandingKind, OneoffView};
use super::{cancel::StopKind, AgentOutcome};

/// Where the new run's clone starts (`--from`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ResumeFrom {
    /// At the old run's head.
    Branch,
    /// At the old head with the checkout's current tip merged in first.
    Master,
}

impl ResumeFrom {
    /// The word shown and passed on the daemon's command line.
    pub fn word(self) -> &'static str {
        match self {
            ResumeFrom::Branch => "branch",
            ResumeFrom::Master => "master",
        }
    }
}

/// The resume a run is started as (`OneoffOptions.resume`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeSpec {
    /// The old run's id.
    pub from_run: String,
    pub start: ResumeFrom,
}

/// What `task resume` does with a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResumeMode {
    /// A new agent session from the old head.
    Agent(ResumeFrom),
    /// The run's landing was paused: retry it (`task land merge`).
    RetryLanding,
}

/// Whether a run's latest landing failure is a paused one (I-15): detected by the
/// recorded `stop`, never by text.
pub fn landing_paused(view: &OneoffView) -> bool {
    view.record
        .land_failure
        .as_ref()
        .is_some_and(|f| f.stop == Some(StopKind::Pause))
}

/// What `task resume` does for `view` given `--from` (I-17): a paused landing is
/// retried and refuses `--from`; otherwise `--from` is required.
pub fn resume_mode(view: &OneoffView, from: Option<ResumeFrom>) -> Result<ResumeMode, String> {
    if landing_paused(view) {
        return match from {
            Some(_) => Err(
                "this run's landing was paused: task resume retries the landing, so omit --from"
                    .to_owned(),
            ),
            None => Ok(ResumeMode::RetryLanding),
        };
    }
    match from {
        Some(from) => Ok(ResumeMode::Agent(from)),
        None => Err("pass --from branch or --from master".to_owned()),
    }
}

/// Why the run cannot be resumed as an agent session, if it cannot (pure, C-12, I-18,
/// I-19). `successor_blocks`: the run named by `resumed_by` still exists and did not fail
/// its resume merge (a failed resume never strands its predecessor).
pub fn resume_refusal(view: &OneoffView, successor_blocks: bool) -> Result<(), String> {
    let record = &view.record;
    if view.live.may_be_live() {
        return Err("the run is still live; stop or pause it first".to_owned());
    }
    if view.landing {
        return Err("a landing of this run is in progress; wait for it to end".to_owned());
    }
    if record.landed.iter().any(|l| l.kind == LandingKind::Merged) {
        return Err("the run was already merged; there is nothing to resume".to_owned());
    }
    match &record.outcome {
        Some(AgentOutcome::Paused | AgentOutcome::Stopped | AgentOutcome::Killed) => {}
        Some(other) => {
            return Err(format!(
                "the run finished on its own ({}); only a paused, stopped or killed run \
                 resumes, land it instead",
                other.word()
            ))
        }
        None => return Err("the run has no outcome yet".to_owned()),
    }
    if record.head.is_none() {
        return Err("the run has no recorded head to resume from".to_owned());
    }
    if record.commits.unwrap_or(0) > 0
        && !matches!(record.branch_state(), BranchState::Tracked { .. })
    {
        return Err(
            "its work is not in the project (no tracked branch), so it cannot be resumed"
                .to_owned(),
        );
    }
    if let (Some(id), true) = (&record.resumed_by, successor_blocks) {
        return Err(format!("the run was already resumed as {id}"));
    }
    Ok(())
}

/// The budget of the resumed run (I-22): the override when given (positive and finite),
/// otherwise what the old run had left, refused below the resolver's floor.
pub fn budget(old_max: f64, old_spend: f64, override_usd: Option<f64>) -> Result<f64, String> {
    if let Some(usd) = override_usd {
        return if usd.is_finite() && usd > 0.0 {
            Ok(usd)
        } else {
            Err("--budget must be a positive number of dollars".to_owned())
        };
    }
    let left = old_max - old_spend.max(0.0);
    if left.is_finite() && left >= MIN_RESOLVER_BUDGET_USD {
        Ok(left)
    } else {
        Err("no budget left to resume; pass --budget".to_owned())
    }
}

/// What `u` means for a run in the TUI (C-01, C-12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyPlan {
    /// Ask `b` / `M`.
    Resume,
    /// The landing was paused: ask the merge question.
    RetryLanding,
    Refused(String),
}

/// `u` on `view`: retry a paused landing, resume an agent session, or say why not.
pub fn key_plan(view: &OneoffView) -> KeyPlan {
    if landing_paused(view) {
        return if view.live.may_be_live() || view.landing {
            KeyPlan::Refused("the run or its landing is still running".to_owned())
        } else {
            KeyPlan::RetryLanding
        };
    }
    match resume_refusal(view, view.record.resumed_by.is_some()) {
        Ok(()) => KeyPlan::Resume,
        Err(why) => KeyPlan::Refused(why),
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use self::io::*;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod io {
    use std::path::Path;

    use super::super::land::{self, take_land_lock, LandOptions};
    use super::super::launch::{claim_new_run, run_detached, run_foreground, OneoffOptions};
    use super::super::store::{
        project_dir_for, read_view, resolve_run, write_record, OneoffView, RunPick,
    };
    use super::super::stream::spend_of_file;
    use super::super::STREAM_FILE;
    use super::{budget, resume_mode, resume_refusal, ResumeFrom, ResumeMode, ResumeSpec};

    /// What `start` did.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum ResumeStarted {
        /// A new run was started (`detached`) or has finished (in the foreground).
        Run {
            old: String,
            new: String,
            from: ResumeFrom,
            detached: bool,
            /// The foreground run's outcome word.
            outcome: Option<String>,
        },
        /// The run's paused landing was retried; this is its result text.
        Landing { run_id: String, message: String },
    }

    /// Whether the run `old.resumed_by` names still blocks another resume: its dir exists
    /// and it did not fail its resume merge (I-18).
    fn successor_blocks(project_dir: &Path, old: &OneoffView) -> bool {
        let Some(id) = &old.record.resumed_by else {
            return false;
        };
        match read_view(project_dir, id) {
            None => false,
            Some(successor) => successor
                .record
                .resume_merge
                .as_ref()
                .is_none_or(|m| m.failure.is_none()),
        }
    }

    /// Resume `run_id` of `project` (the one run picked like every other verb does when
    /// `None`). `detach` starts the new run as a daemon; otherwise it runs in this
    /// process. `land_opts` makes the options of a retried paused landing (only called then,
    /// so a CLI's signal handlers are installed once).
    ///
    /// The start is serialized (I-18, T-klf-12): under the OLD run's land lock the record
    /// is read again, the refusals applied, the new run dir claimed and `resumed_by`
    /// written; only then is the lock released and the new run started. If the start
    /// fails, `resumed_by` is cleared again.
    #[allow(clippy::too_many_arguments)]
    pub fn start(
        project: &Path,
        alias: &str,
        run_id: Option<&str>,
        from: Option<ResumeFrom>,
        budget_override: Option<f64>,
        unsandboxed: bool,
        egress_allow: Vec<String>,
        detach: bool,
        config: Option<&Path>,
        land_opts: &dyn Fn() -> Result<LandOptions, String>,
    ) -> Result<ResumeStarted, String> {
        let project = std::fs::canonicalize(project).map_err(|e| format!("project: {e}"))?;
        let project_dir = project_dir_for(&project)
            .ok_or_else(|| "no one-off directory for this project".to_owned())?;
        let picked = match resolve_run(&project_dir, run_id, true) {
            Ok(view) => view,
            Err(RunPick::NoRuns) => return Err("this project has no one-off runs".to_owned()),
            Err(pick) => return Err(pick.detail()),
        };
        let old_id = picked.record.run_id.clone();
        match resume_mode(&picked, from)? {
            ResumeMode::RetryLanding => {
                let message = land::merge(&project, &old_id, &land_opts()?)?;
                Ok(ResumeStarted::Landing {
                    run_id: old_id,
                    message,
                })
            }
            ResumeMode::Agent(from) => {
                let (new_id, options) = {
                    // Under the old run's land lock: no landing, discard or second resume
                    // can interleave.
                    let _lock = take_land_lock(&picked.run_dir, &old_id)?;
                    let mut old = read_view(&project_dir, &old_id)
                        .ok_or_else(|| format!("no one-off run {old_id} for this project"))?;
                    // The land lock is ours: a landing in progress would have kept us
                    // from taking it, so what the view reads as "landing" is this lock.
                    old.landing = false;
                    resume_refusal(&old, successor_blocks(&project_dir, &old))?;
                    if from == ResumeFrom::Master
                        && !matches!(
                            crate::launcher::push::current_branch_and_sha(&project),
                            Some((Some(_), _))
                        )
                    {
                        return Err(
                            "the checkout's HEAD is detached; check out a branch to merge first"
                                .to_owned(),
                        );
                    }
                    let spent = old
                        .record
                        .spend_usd
                        .unwrap_or_else(|| spend_of_file(&old.run_dir.join(STREAM_FILE)));
                    let usd = budget(old.record.max_budget_usd, spent, budget_override)?;
                    let (new_id, _dir) = claim_new_run(&project).map_err(|e| e.to_string())?;
                    let mut marked = old.record.clone();
                    marked.resumed_by = Some(new_id.clone());
                    write_record(&old.run_dir, &marked)
                        .map_err(|e| format!("could not record the resume: {e}"))?;
                    let options = OneoffOptions {
                        project: project.clone(),
                        alias: alias.to_owned(),
                        mode: old.record.mode,
                        task: old.record.task.clone(),
                        model: old.record.model.clone(),
                        max_budget_usd: usd,
                        cap_minutes: old.record.cap_minutes,
                        unsandboxed,
                        run_id: Some(new_id.clone()),
                        egress_allow,
                        resume: Some(ResumeSpec {
                            from_run: old_id.clone(),
                            start: from,
                        }),
                    };
                    (new_id, options)
                };
                let clear = |project_dir: &Path| {
                    if let Some(old) = read_view(project_dir, &old_id) {
                        let mut record = old.record.clone();
                        if record.resumed_by.as_deref() == Some(new_id.as_str()) {
                            record.resumed_by = None;
                            let _ = write_record(&old.run_dir, &record);
                        }
                    }
                };
                if detach {
                    match run_detached(&options, config) {
                        Ok(started) => Ok(ResumeStarted::Run {
                            old: old_id,
                            new: started.run_id,
                            from,
                            detached: true,
                            outcome: None,
                        }),
                        Err(e) => {
                            clear(&project_dir);
                            Err(e.to_string())
                        }
                    }
                } else {
                    match run_foreground(options) {
                        Ok(record) => Ok(ResumeStarted::Run {
                            old: old_id,
                            new: record.run_id.clone(),
                            from,
                            detached: false,
                            outcome: record.outcome.as_ref().map(|o| o.word().to_owned()),
                        }),
                        Err(e) => {
                            clear(&project_dir);
                            Err(e.to_string())
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oneoff::store::{
        LandFailure, Landing, LandingKind, LiveState, MergeHow, OneoffView,
    };
    use std::path::PathBuf;

    fn view(outcome: AgentOutcome) -> OneoffView {
        let mut view = crate::ui::screens::oneoff::tests::finished_view("x");
        view.record.outcome = Some(outcome);
        view.record.head = Some("a".repeat(40));
        view.record.commits = Some(2);
        view.record.tracked_branch = Some("refs/heads/gsd-oneoff/oneoff-x".to_owned());
        view.record.landed.clear();
        view.record.land_failure = None;
        view.live = LiveState::Finished;
        view.landing = false;
        view.run_dir = PathBuf::from("/x");
        view
    }

    #[test]
    fn paused_stopped_and_killed_runs_with_a_head_resume() {
        for outcome in [
            AgentOutcome::Paused,
            AgentOutcome::Stopped,
            AgentOutcome::Killed,
        ] {
            assert!(resume_refusal(&view(outcome), false).is_ok());
        }
        // A zero-commit pause resumes from its base.
        let mut empty = view(AgentOutcome::Paused);
        empty.record.commits = Some(0);
        empty.record.tracked_branch = None;
        empty.record.tracking_note = Some("the run made no commits; no branch tracked".to_owned());
        assert!(resume_refusal(&empty, false).is_ok());
    }

    #[test]
    fn each_refusal_has_its_own_reason() {
        let cases: Vec<(OneoffView, bool, &str)> = vec![
            (
                {
                    let mut v = view(AgentOutcome::Paused);
                    v.live = LiveState::Running;
                    v
                },
                false,
                "still live",
            ),
            (
                {
                    let mut v = view(AgentOutcome::Paused);
                    v.landing = true;
                    v
                },
                false,
                "landing",
            ),
            (
                {
                    let mut v = view(AgentOutcome::Paused);
                    v.record.landed.push(Landing {
                        how: Some(MergeHow::Ff),
                        ..Landing::plain(LandingKind::Merged, "t".to_owned(), "main".to_owned())
                    });
                    v
                },
                false,
                "already merged",
            ),
            (view(AgentOutcome::Finished), false, "finished on its own"),
            (view(AgentOutcome::TimedOut), false, "finished on its own"),
            (
                {
                    let mut v = view(AgentOutcome::Paused);
                    v.record.head = None;
                    v
                },
                false,
                "no recorded head",
            ),
            (
                {
                    let mut v = view(AgentOutcome::Killed);
                    v.record.tracked_branch = None;
                    v
                },
                false,
                "not in the project",
            ),
            (
                {
                    let mut v = view(AgentOutcome::Paused);
                    v.record.resumed_by = Some("oneoff-y".to_owned());
                    v
                },
                true,
                "already resumed as oneoff-y",
            ),
        ];
        for (view, blocks, needle) in cases {
            let why = resume_refusal(&view, blocks).unwrap_err();
            assert!(why.contains(needle), "{needle}: {why}");
        }
        // A successor that failed its merge (or is gone) does not block.
        let mut v = view(AgentOutcome::Paused);
        v.record.resumed_by = Some("oneoff-y".to_owned());
        assert!(resume_refusal(&v, false).is_ok());
    }

    #[test]
    fn a_paused_landing_is_retried_and_refuses_from() {
        let mut v = view(AgentOutcome::Finished);
        v.record.land_failure = Some(LandFailure {
            at: "t".to_owned(),
            reason: "paused".to_owned(),
            conflicted: Vec::new(),
            attempts: Vec::new(),
            stop: Some(StopKind::Pause),
        });
        assert_eq!(resume_mode(&v, None), Ok(ResumeMode::RetryLanding));
        assert!(resume_mode(&v, Some(ResumeFrom::Branch))
            .unwrap_err()
            .contains("retries the landing"));
        assert_eq!(key_plan(&v), KeyPlan::RetryLanding);
        // A stopped landing is not a paused one.
        v.record.land_failure.as_mut().unwrap().stop = Some(StopKind::Hard);
        assert!(resume_mode(&v, None).unwrap_err().contains("--from"));
        assert_eq!(
            resume_mode(&view(AgentOutcome::Paused), Some(ResumeFrom::Master)),
            Ok(ResumeMode::Agent(ResumeFrom::Master))
        );
    }

    #[test]
    fn the_budget_is_the_override_the_remainder_or_a_refusal() {
        assert_eq!(budget(10.0, 4.0, None), Ok(6.0));
        assert_eq!(budget(10.0, 4.0, Some(2.5)), Ok(2.5));
        assert_eq!(budget(0.2, 0.0, Some(1.0)), Ok(1.0));
        assert!(budget(10.0, 9.9, None)
            .unwrap_err()
            .contains("pass --budget"));
        assert!(budget(10.0, 0.0, Some(f64::NAN)).is_err());
        assert!(budget(10.0, 0.0, Some(0.0)).is_err());
    }

    #[test]
    fn the_key_plan_resumes_retries_or_refuses() {
        assert_eq!(key_plan(&view(AgentOutcome::Paused)), KeyPlan::Resume);
        assert!(matches!(
            key_plan(&view(AgentOutcome::Finished)),
            KeyPlan::Refused(_)
        ));
    }
}
