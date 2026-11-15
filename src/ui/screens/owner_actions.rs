//! The actions the Owner inbox and the Owner block share (37-03, D-11, D-12).
//!
//! `v` runs a group's GSD command in its project. Both surfaces reach it through
//! [`run_group`], so the refusal table, the live-session rule and the argv shape
//! exist once.
//!
//! **Where the command comes from (D-11, T-37-04, CWE-88).** Only from the typed
//! [`RunCommand`] the reader put on the group, never from the display `command`
//! string or from `phase`. At launch the phase token and the workstream name are
//! checked again with the reader's own `is_command_token` / `is_workstream_name`,
//! so a value that reached the digest by any other path still cannot become an
//! argv element. The command travels as ONE argv element behind `env claude`
//! (see `detail::owner_mux_argv`), and the project path as the child's cwd.
//!
//! **Nothing is injected (D-12, T-37-07).** A project with a live session gets
//! that session focused and the command shown for the owner to type; the branch
//! calls only `multiplexer::focus_message` and writes nothing to the session.

use crate::session_detector::ClaudeSession;
use crate::state_reader::owner_items::{
    is_command_token, is_workstream_name, Lane, OwnerGroup, RunCommand, RunVerb,
};
use crate::ui::screens::AppContext;
use std::path::PathBuf;

use super::detail::{owner_mux_argv, owner_terminal_argv, resolve_launch_plan};

/// What `v` is about to do, fully resolved and validated.
#[derive(Debug, Clone)]
pub(crate) struct RunPlan {
    pub alias: String,
    /// The project path: the child's cwd, never part of the argv.
    pub cwd: PathBuf,
    pub command: RunCommand,
    /// A live session whose working directory IS the project path (D-12).
    pub live_session: Option<ClaudeSession>,
}

/// Whether `verb` spells a phase in its command.
fn takes_phase(verb: RunVerb) -> bool {
    matches!(verb, RunVerb::VerifyWork | RunVerb::PlanPhaseGaps)
}

/// Resolve `group` of `alias` into a [`RunPlan`], or the status line that says why
/// nothing starts (UI-SPEC "Status line").
pub(crate) fn run_plan(
    ctx: &AppContext,
    alias: &str,
    group: &OwnerGroup,
) -> Result<RunPlan, String> {
    if group.lane == Lane::Blocked {
        return Err("No command for BLOCKED work: unblock it first".to_owned());
    }
    if group.counts.unreadable > 0 {
        return Err("No command: the source file is unreadable".to_owned());
    }
    let no_command = || "No command for this row".to_owned();
    let command = group.run.clone().ok_or_else(no_command)?;
    if takes_phase(command.verb) {
        let token = command.phase.as_deref().ok_or_else(no_command)?;
        if !is_command_token(token) {
            return Err(format!(
                "Refused: {token} is not a safe phase token; nothing started"
            ));
        }
    }
    if let Some(name) = command.workstream.as_deref() {
        if !is_workstream_name(name) {
            return Err(format!(
                "Refused: {name} is not a safe workstream name; nothing started"
            ));
        }
    }
    let project = ctx.config.projects.get(alias).ok_or_else(no_command)?;
    // Exact path equality (T-37-06): a sibling directory that merely shares the
    // project path as a string prefix is a different project.
    let live_session = ctx
        .active_sessions
        .iter()
        .find(|session| session.working_dir == project.path)
        .cloned();
    Ok(RunPlan {
        alias: alias.to_owned(),
        cwd: project.path.clone(),
        command,
        live_session,
    })
}

/// The status line after a live session was focused: the focus message, and the
/// command to type when the focus worked (`Switched to ...`). Any other text (a
/// failure, a missing multiplexer) passes through unchanged.
pub(crate) fn focus_status(focus_message: String, command: &RunCommand) -> String {
    if focus_message.starts_with("Switched to") {
        format!("{focus_message} \u{b7} type {command}")
    } else {
        focus_message
    }
}

/// The real action behind `v` (D-12): focus the live session or launch a new one.
/// **Called only from key arms**; tests drive [`run_group`] with a recording
/// closure instead, so none of them touches a multiplexer or a terminal.
pub(crate) fn launch_real(plan: &RunPlan) -> String {
    if let Some(session) = &plan.live_session {
        let message = crate::multiplexer::focus_message(session, &plan.alias);
        return focus_status(message, &plan.command);
    }
    let launch_plan = resolve_launch_plan();
    crate::multiplexer::launch(
        &launch_plan,
        &plan.cwd,
        owner_mux_argv(&plan.command),
        &|term| owner_terminal_argv(term, &plan.command),
        &format!("Launched {} in {}", plan.command, plan.alias),
    )
}

/// Run `group` of `alias`: the refusal text, or whatever `launch` reports.
pub(crate) fn run_group(
    ctx: &AppContext,
    alias: &str,
    group: &OwnerGroup,
    launch: &mut dyn FnMut(&RunPlan) -> String,
) -> String {
    match run_plan(ctx, alias, group) {
        Ok(plan) => launch(&plan),
        Err(message) => message,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::owner_items::{GroupCounts, Kind, Scope};
    use crate::ui::screens::tests::ctx_with_aliases;

    fn group(lane: Lane, run: Option<RunCommand>) -> OwnerGroup {
        OwnerGroup {
            lane,
            kind: Kind::Uat,
            scope: Scope::Current,
            workstream: None,
            phase: "24".to_owned(),
            phase_name: "clock-faces".to_owned(),
            file: "phases/24/24-UAT.md".to_owned(),
            file_status: "partial".to_owned(),
            closes_phase: true,
            bookkeeping_only: false,
            acknowledged: false,
            age_days: Some(2),
            stale: false,
            command: run.as_ref().map(ToString::to_string),
            run,
            supersedes: Vec::new(),
            counts: GroupCounts::default(),
            items: Vec::new(),
        }
    }

    fn verify(phase: Option<&str>, ws: Option<&str>) -> RunCommand {
        RunCommand {
            verb: RunVerb::VerifyWork,
            phase: phase.map(str::to_owned),
            workstream: ws.map(str::to_owned),
        }
    }

    #[test]
    fn run_plan_refusals() {
        let ctx = ctx_with_aliases(&["alder-api"]);
        let mut unreadable = group(Lane::Fix, Some(verify(Some("24"), None)));
        unreadable.counts.unreadable = 1;
        let mut debt = group(Lane::Test, None);
        debt.scope = Scope::Archived {
            label: "v1.0".to_owned(),
            rank: 0,
        };
        let plan_gaps_no_phase = group(
            Lane::Fix,
            Some(RunCommand {
                verb: RunVerb::PlanPhaseGaps,
                phase: None,
                workstream: None,
            }),
        );
        let cases: Vec<(OwnerGroup, &str)> = vec![
            (
                group(Lane::Blocked, Some(verify(Some("24"), None))),
                "No command for BLOCKED work: unblock it first",
            ),
            (unreadable, "No command: the source file is unreadable"),
            (debt, "No command for this row"),
            (
                group(Lane::Test, Some(verify(None, None))),
                "No command for this row",
            ),
            (plan_gaps_no_phase, "No command for this row"),
            (
                group(Lane::Test, Some(verify(Some("1;rm"), None))),
                "Refused: 1;rm is not a safe phase token; nothing started",
            ),
            (
                group(Lane::Test, Some(verify(Some("\u{ff12}\u{ff14}"), None))),
                "Refused: \u{ff12}\u{ff14} is not a safe phase token; nothing started",
            ),
            (
                group(Lane::Test, Some(verify(Some("24"), Some("a b")))),
                "Refused: a b is not a safe workstream name; nothing started",
            ),
            (
                group(Lane::Test, Some(verify(Some("24"), Some("../x")))),
                "Refused: ../x is not a safe workstream name; nothing started",
            ),
        ];
        for (group, want) in cases {
            let mut launched = 0;
            let got = run_group(&ctx, "alder-api", &group, &mut |_| {
                launched += 1;
                "launched".to_owned()
            });
            assert_eq!(got, want);
            assert_eq!(launched, 0, "{want}: nothing may start");
        }
        // The happy path reaches the closure with the validated command.
        let ok = group(Lane::Test, Some(verify(Some("24"), Some("v14"))));
        let got = run_group(&ctx, "alder-api", &ok, &mut |plan| plan.command.to_string());
        assert_eq!(got, "/gsd-verify-work 24 --ws v14");
        // A debug command takes no phase and runs without one.
        let debug = group(
            Lane::Fix,
            Some(RunCommand {
                verb: RunVerb::Debug,
                phase: None,
                workstream: None,
            }),
        );
        assert!(run_plan(&ctx, "alder-api", &debug).is_ok());
        // An unknown alias has no project path to run in.
        assert_eq!(
            run_plan(&ctx, "nobody", &ok).unwrap_err(),
            "No command for this row"
        );
    }

    fn session_in(dir: &str) -> ClaudeSession {
        ClaudeSession {
            pid: 4242,
            kind: crate::session_detector::SessionKind::Claude,
            session_id: None,
            working_dir: PathBuf::from(dir),
            start_time: Some(1),
            tty: None,
        }
    }

    #[test]
    fn a_session_in_a_sibling_directory_is_not_focused() {
        let mut ctx = ctx_with_aliases(&["alder-api"]);
        let project = ctx.config.projects["alder-api"].path.clone();
        let sibling = format!("{}-old", project.display());
        ctx.active_sessions = vec![session_in(&sibling)];
        let g = group(Lane::Test, Some(verify(Some("24"), None)));
        let plan = run_plan(&ctx, "alder-api", &g).expect("a plan");
        assert!(plan.live_session.is_none(), "{sibling} is not {project:?}");
        // The project's own directory is focused.
        ctx.active_sessions
            .push(session_in(&project.to_string_lossy()));
        let plan = run_plan(&ctx, "alder-api", &g).expect("a plan");
        assert_eq!(
            plan.live_session.map(|s| s.working_dir),
            Some(project.clone())
        );
        assert_eq!(plan.cwd, project);
    }

    #[test]
    fn fullwidth_digits_are_not_a_phase_token() {
        assert!(is_command_token("24"));
        assert!(!is_command_token("\u{ff12}\u{ff14}"));
        assert!(!is_command_token("\u{661}"));
        let ctx = ctx_with_aliases(&["alder-api"]);
        let g = group(Lane::Test, Some(verify(Some("\u{ff12}\u{ff14}"), None)));
        assert!(run_plan(&ctx, "alder-api", &g)
            .unwrap_err()
            .starts_with("Refused: "));
    }

    #[test]
    fn the_focus_status_shows_the_command_only_after_a_switch() {
        let cmd = verify(Some("24"), None);
        assert_eq!(
            focus_status("Switched to tmux pane %3".to_owned(), &cmd),
            "Switched to tmux pane %3 \u{b7} type /gsd-verify-work 24"
        );
        // A failure passes through unchanged.
        let failed = "Tab-switching requires running inside tmux or herdr (none detected)";
        assert_eq!(focus_status(failed.to_owned(), &cmd), failed);
    }
}
