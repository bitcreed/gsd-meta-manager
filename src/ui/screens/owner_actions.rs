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
use crate::ui::screens::{AppContext, ScreenAction};
use std::path::{Path, PathBuf};

use super::detail::{owner_mux_argv, owner_terminal_argv, resolve_launch_plan, shown};
use super::owner_file_view::OwnerFileViewScreen;

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

/// The most bytes `o` reads from a source file (D-08, T-37-14): the viewer is for
/// the owner's own checklists, not for a log someone left in `.planning/`.
pub(crate) const MAX_VIEW_BYTES: u64 = 1024 * 1024;

/// The text of the project file `rel` (a POSIX path relative to the project
/// root), or `None` when it may not or cannot be shown (T-37-13, T-37-14).
///
/// `rel` comes from a project file, so it is untrusted: both the project's
/// `.planning/` directory and the target are canonicalized (symlinks and `..`
/// resolved), the target must lie inside the first, be a regular file and be at
/// most [`MAX_VIEW_BYTES`] long. Any failure is `None`; the caller says only
/// that the file is not readable.
pub(crate) fn read_owner_file(project_root: &Path, rel: &str) -> Option<String> {
    let planning = project_root.join(".planning").canonicalize().ok()?;
    let target = project_root.join(rel).canonicalize().ok()?;
    if !target.starts_with(&planning) {
        return None;
    }
    let meta = std::fs::metadata(&target).ok()?;
    if !meta.is_file() || meta.len() > MAX_VIEW_BYTES {
        return None;
    }
    let bytes = std::fs::read(&target).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// `o` (D-08): read the file behind a row and push the read-only viewer, or the
/// status line that says why not. `files` are the row's distinct source files
/// (one for a group, several for a BLOCKED aggregate). **Called from key arms
/// only**: the read happens on the key press, never on the render path, and
/// nothing is spawned (T-37-16).
pub(crate) fn open_target(
    ctx: &AppContext,
    alias: &str,
    files: &[String],
) -> Result<ScreenAction, String> {
    let files: Vec<&String> = files.iter().filter(|f| !f.is_empty()).collect();
    let [file] = files.as_slice() else {
        return match files.len() {
            0 => Err("Cannot open this row: no source file".to_owned()),
            n => Err(format!(
                "{n} files behind this row: expand it with Enter, or press g"
            )),
        };
    };
    let not_readable = || format!("Cannot open {}: not readable", shown(file));
    let project = ctx.config.projects.get(alias).ok_or_else(not_readable)?;
    let text = read_owner_file(&project.path, file).ok_or_else(not_readable)?;
    Ok(ScreenAction::Push(Box::new(OwnerFileViewScreen::new(
        alias, file, &text,
    ))))
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

    // --- `o`: the contained read (37-05) -----------------------------------

    /// A project root with `.planning/phases/24-x/24-UAT.md` and a registry entry.
    fn project_with_uat(alias: &str) -> (tempfile::TempDir, AppContext) {
        let dir = tempfile::tempdir().expect("tempdir");
        let phase = dir.path().join(".planning/phases/24-x");
        std::fs::create_dir_all(&phase).unwrap();
        std::fs::write(phase.join("24-UAT.md"), "# UAT\n\n- a check\n").unwrap();
        let mut ctx = ctx_with_aliases(&[alias]);
        ctx.config.projects.get_mut(alias).unwrap().path = dir.path().to_path_buf();
        (dir, ctx)
    }

    #[test]
    fn read_owner_file_stays_inside_planning() {
        let (dir, _ctx) = project_with_uat("alder-api");
        let root = dir.path();
        let ok = ".planning/phases/24-x/24-UAT.md";
        assert_eq!(
            read_owner_file(root, ok).as_deref(),
            Some("# UAT\n\n- a check\n")
        );
        // Missing, a directory, and a path that climbs out with `..`.
        assert_eq!(read_owner_file(root, ".planning/phases/24-x/nope.md"), None);
        assert_eq!(read_owner_file(root, ".planning/phases"), None);
        std::fs::write(root.join("outside.md"), "secret").unwrap();
        assert_eq!(read_owner_file(root, ".planning/../outside.md"), None);
        assert_eq!(read_owner_file(root, "../outside.md"), None);
        assert_eq!(read_owner_file(root, "outside.md"), None);
        // No `.planning/` at all.
        let bare = tempfile::tempdir().unwrap();
        std::fs::write(bare.path().join("x.md"), "x").unwrap();
        assert_eq!(read_owner_file(bare.path(), "x.md"), None);
        // One byte over the cap is refused, the cap itself is read.
        let big = root.join(".planning/big.md");
        std::fs::write(&big, vec![b'a'; MAX_VIEW_BYTES as usize]).unwrap();
        assert!(read_owner_file(root, ".planning/big.md").is_some());
        std::fs::write(&big, vec![b'a'; MAX_VIEW_BYTES as usize + 1]).unwrap();
        assert_eq!(read_owner_file(root, ".planning/big.md"), None);
        // Invalid UTF-8 is decoded lossily, not refused.
        std::fs::write(root.join(".planning/odd.md"), [b'o', 0xff, b'k']).unwrap();
        assert_eq!(
            read_owner_file(root, ".planning/odd.md").as_deref(),
            Some("o\u{fffd}k")
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_planning_is_not_followed() {
        let (dir, _ctx) = project_with_uat("alder-api");
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret.md"), "secret").unwrap();
        let link = dir.path().join(".planning/link.md");
        std::os::unix::fs::symlink(outside.path().join("secret.md"), &link).unwrap();
        assert_eq!(read_owner_file(dir.path(), ".planning/link.md"), None);
        // A symlink to a file inside `.planning/` is fine.
        let inner = dir.path().join(".planning/inner.md");
        std::os::unix::fs::symlink(dir.path().join(".planning/phases/24-x/24-UAT.md"), &inner)
            .unwrap();
        assert!(read_owner_file(dir.path(), ".planning/inner.md").is_some());
        // The whole `.planning/` pointing elsewhere is not inside itself either.
        let swapped = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("a.md"), "a").unwrap();
        std::os::unix::fs::symlink(outside.path(), swapped.path().join(".planning")).unwrap();
        // The target resolves under the (canonical) `.planning/`, so it is read:
        // the containment is relative to wherever `.planning/` really is.
        assert!(read_owner_file(swapped.path(), ".planning/a.md").is_some());
    }

    #[test]
    fn open_target_pushes_the_viewer_or_says_why_not() {
        let (_dir, ctx) = project_with_uat("alder-api");
        let one = vec![".planning/phases/24-x/24-UAT.md".to_owned()];
        let Ok(ScreenAction::Push(screen)) = open_target(&ctx, "alder-api", &one) else {
            panic!("a readable file pushes the viewer");
        };
        assert_eq!(screen.name(), "owner_file_view");
        let missing = vec![".planning/phases/24-x/gone.md".to_owned()];
        assert_eq!(
            open_target(&ctx, "alder-api", &missing).err().as_deref(),
            Some("Cannot open .planning/phases/24-x/gone.md: not readable")
        );
        assert_eq!(
            open_target(&ctx, "nobody", &one).err().as_deref(),
            Some("Cannot open .planning/phases/24-x/24-UAT.md: not readable")
        );
        // A hostile file name is escaped in the message.
        let hostile = vec![".planning/\u{1b}[2J.md".to_owned()];
        let message = open_target(&ctx, "alder-api", &hostile).err().unwrap();
        assert!(!message.chars().any(char::is_control), "{message:?}");
    }

    #[test]
    fn a_blocked_row_over_two_files_points_at_expand() {
        let (_dir, ctx) = project_with_uat("alder-api");
        let two = vec![
            ".planning/phases/24-x/24-UAT.md".to_owned(),
            ".planning/phases/24-x/24-VERIFICATION.md".to_owned(),
        ];
        assert_eq!(
            open_target(&ctx, "alder-api", &two).err().as_deref(),
            Some("2 files behind this row: expand it with Enter, or press g")
        );
        // Duplicates and blanks do not count as files.
        let dup = vec![".planning/phases/24-x/24-UAT.md".to_owned(), String::new()];
        assert!(open_target(&ctx, "alder-api", &dup).is_ok());
    }
}
