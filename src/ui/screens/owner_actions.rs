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
//!
//! **`A` writes nothing itself (D-14..D-16, D7).** The only writer is gsd-core's
//! own acknowledge command, run as a bounded direct child after an explicit Yes
//! in a popup whose default is No. [`ack_target_for`] decides whether a group can
//! be acknowledged at all and names the file, [`ack_args`] spells the argv (every
//! value one element), [`ack_request`] adds the launcher, and [`run_ack`] runs it
//! with the workstream and session selectors removed from the environment.

use crate::session_detector::ClaudeSession;
use crate::state_reader::owner_items::{
    is_command_token, is_workstream_name, Kind, Lane, OwnerDigest, OwnerGroup, RunCommand, RunVerb,
    Scope,
};
use crate::state_reader::queue_md::{resolve_user_gsd_tools, GsdToolsCmd};
use crate::state_reader::ProjectState;
use crate::ui::screens::{AppContext, ScreenAction};
use std::path::{Path, PathBuf};

use super::ack_confirm::{AckConfirmScreen, AckHost};
use super::detail::{owner_mux_argv, owner_terminal_argv, resolve_launch_plan, shown};
use super::inbox::InboxKey;
use super::owner_block::GroupKey;
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
    if focus_message.starts_with(crate::multiplexer::FOCUS_OK_PREFIX) {
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

/// Whether a key press may run an owner action (`v`, `o`, `A`, `y`, `g`): no
/// Control and no Alt. A terminal in raw mode delivers Ctrl-V (the paste reflex)
/// as `Char('v')` with CONTROL, and Ctrl-O, Ctrl-Y and Ctrl-G likewise. Shift is
/// allowed because `A` arrives shifted (WR-01).
pub(crate) fn is_plain_action_key(modifiers: crossterm::event::KeyModifiers) -> bool {
    use crossterm::event::KeyModifiers as M;
    !modifiers.intersects(M::CONTROL | M::ALT)
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
    // One handle for the check and the read: a file swapped or grown after the
    // size check cannot be read past the cap (WR-06, T-37-14).
    let file = std::fs::File::open(&target).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let bytes = read_capped(file, MAX_VIEW_BYTES)?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// At most `cap` bytes of `source`; `None` when it holds more, or cannot be read.
/// Reads one byte past the cap and no further, however long the source is.
fn read_capped(source: impl std::io::Read, cap: u64) -> Option<Vec<u8>> {
    use std::io::Read as _;
    let mut bytes = Vec::new();
    source
        .take(cap.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    (u64::try_from(bytes.len()).ok()? <= cap).then_some(bytes)
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

/// The gsd-core command that acknowledges an owner file: the verb and its
/// subcommand, spelled **once** in `src/` (37 D-14, RESEARCH Pitfall 1). Every
/// argv and every expectation, tests included, is built from this constant; the
/// source guard in `tests/owner_open_conformance.rs` allows this one file, and
/// only on lines that also name the acknowledge subcommand, so the read verb
/// stays unreachable from the product.
pub const ACK_VERB: [&str; 2] = ["audit-open", "acknowledge"];

/// How long the acknowledge child may run (D-15, T-37-21). Defined on every
/// target because the timeout status line spells it.
const ACK_BUDGET: std::time::Duration = std::time::Duration::from_secs(10);

/// The longest failure line the status shows, in characters (D-16).
const ACK_LINE_CAP: usize = 120;

/// What gsd-core's workstream resolution reads from the environment, removed from
/// the acknowledge child (37 D-15, RESEARCH Pitfall 2, T-37-18). gsd-core resolves
/// the workstream as "`--ws` flag, then `GSD_WORKSTREAM`, then a session or shared
/// pointer", and keys the session pointer on the first of these variables that
/// is set (`active-workstream-store.cjs`, `SESSION_KEY_ENV_VARS`). An inherited
/// value would send a root-scoped write into a workstream's file. `GSD_PROJECT`
/// is removed as well: the child's cwd is the project, never an inherited path.
pub const ACK_ENV_REMOVE: &[&str] = &[
    "GSD_WORKSTREAM",
    "GSD_PROJECT",
    "GSD_SESSION_KEY",
    "CODEX_THREAD_ID",
    "CLAUDE_SESSION_ID",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_SSE_PORT",
    "OPENCODE_SESSION_ID",
    "GEMINI_SESSION_ID",
    "CURSOR_SESSION_ID",
    "WINDSURF_SESSION_ID",
    "TERM_SESSION_ID",
    "WT_SESSION",
    "TMUX_PANE",
    "ZELLIJ_SESSION_NAME",
];

/// What `A` is about to acknowledge, fully validated: every field is a value that
/// became (or will become) one argv element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AckTarget {
    /// gsd-core's category: `uat_gaps`, `verification_gaps`, `context_questions`
    /// or `debug_sessions`.
    pub category: &'static str,
    /// The marker's milestone: the project's (or the workstream's) current one.
    pub milestone: String,
    /// The phase directory token exactly as the reader carries it (`03`, `18.1`),
    /// never the roadmap number. `None` for a debug session.
    pub phase: Option<String>,
    /// The file's basename. `None` for a debug session.
    pub file: Option<String>,
    /// A debug session's file stem. `None` for the phase-scoped categories.
    pub slug: Option<String>,
    /// The archived milestone's label for an Old-debt group.
    pub archived_milestone: Option<String>,
    /// The workstream a workstream group lives in.
    pub workstream: Option<String>,
    /// The file as the digest names it (project-relative, POSIX): shown to the
    /// owner, never an argv element.
    pub rel_file: String,
    /// The items the marker hides: every group of this file in every lane.
    pub items: usize,
}

impl AckTarget {
    /// The name the status lines use: the file's basename.
    pub fn name(&self) -> &str {
        self.file
            .as_deref()
            .or(self.slug.as_deref())
            .unwrap_or(&self.rel_file)
    }
}

/// The last `/`-separated segment of a project-relative path.
fn basename(rel: &str) -> &str {
    rel.rsplit('/').next().unwrap_or(rel)
}

/// Whether `value` may become the value of an acknowledge flag: non-empty, only
/// the identity alphabet, and not shaped like a flag itself (CWE-88).
fn is_flag_value(value: &str) -> bool {
    !value.is_empty() && !value.starts_with('-') && value.chars().all(crate::text::is_identity_char)
}

/// Resolve `group` of `alias` into the one file gsd-core would acknowledge, or the
/// status line that says why nothing starts (UI-SPEC "Status line", D-14).
///
/// `active_workstream_set` is whether `<project>/.planning/active-workstream`
/// holds a name: gsd-core has no spelling for "the root", so a root-scoped group
/// is refused while a pointer could redirect the write (T-37-18).
pub fn ack_target_for(
    alias: &str,
    state: &ProjectState,
    digest: &OwnerDigest,
    group: &OwnerGroup,
    active_workstream_set: bool,
) -> Result<AckTarget, String> {
    let name = basename(&group.file);
    if group.lane == Lane::Blocked {
        return Err("Cannot acknowledge a BLOCKED row: gsd-core has no category for it".to_owned());
    }
    if group.acknowledged {
        return Err(format!("Already acknowledged: {name}"));
    }
    let category = match group.kind {
        Kind::Uat => "uat_gaps",
        Kind::Verification => "verification_gaps",
        Kind::ContextQuestions => "context_questions",
        Kind::Debug => "debug_sessions",
        Kind::Handoff => {
            return Err("Cannot acknowledge a handoff: gsd-core has no category for it".to_owned())
        }
        Kind::Checkpoint => {
            return Err(
                "Cannot acknowledge a checkpoint: gsd-core has no category for it".to_owned(),
            )
        }
        Kind::Deferred => {
            return Err(
                "Cannot acknowledge deferred items: gsd-core has no category for it".to_owned(),
            )
        }
    };
    let refused =
        |what: &str, value: &str| format!("Refused: {value} is not a safe {what}; nothing started");
    if let Some(ws) = group.workstream.as_deref() {
        if !is_workstream_name(ws) {
            return Err(refused("workstream name", ws));
        }
    }
    let milestone = match group.workstream.as_deref() {
        Some(ws) => state
            .workstreams
            .iter()
            .find(|candidate| candidate.name == ws)
            .map(|candidate| candidate.state.milestone.trim().to_owned()),
        None => Some(state.milestone.trim().to_owned()),
    }
    .filter(|version| !version.is_empty())
    .ok_or_else(|| format!("Cannot acknowledge: {alias} has no known milestone version"))?;
    if !is_flag_value(&milestone) {
        return Err(refused("milestone version", &milestone));
    }
    if !is_flag_value(name) {
        return Err(refused("file name", name));
    }
    let archived_milestone = match &group.scope {
        Scope::Current => None,
        Scope::Archived { label, .. } => {
            if !is_flag_value(label) {
                return Err(refused("milestone label", label));
            }
            Some(label.clone())
        }
    };
    let (phase, file, slug) = if category == "debug_sessions" {
        let stem = name.strip_suffix(".md").unwrap_or(name);
        if !is_flag_value(stem) {
            return Err(refused("file name", stem));
        }
        (None, None, Some(stem.to_owned()))
    } else {
        if !is_command_token(&group.phase) {
            return Err(refused("phase token", &group.phase));
        }
        (Some(group.phase.clone()), Some(name.to_owned()), None)
    };
    if group.workstream.is_none() && active_workstream_set {
        return Err(format!(
            "Cannot acknowledge {name}: {alias} has an active workstream set; run it in the project"
        ));
    }
    let items = digest
        .groups
        .iter()
        .filter(|other| other.file == group.file && other.workstream == group.workstream)
        .map(|other| other.counts.checks)
        .sum();
    Ok(AckTarget {
        category,
        milestone,
        phase,
        file,
        slug,
        archived_milestone,
        workstream: group.workstream.clone(),
        rel_file: group.file.clone(),
        items,
    })
}

/// The acknowledge arguments (after the launcher's own prefix): the verb pair,
/// then one element per flag and per value. Never one interpreter string (T-37-19).
pub fn ack_args(target: &AckTarget) -> Vec<String> {
    let mut args: Vec<String> = ACK_VERB.iter().map(|part| (*part).to_owned()).collect();
    let mut flag = |name: &str, value: &str| {
        args.push(format!("--{name}"));
        args.push(value.to_owned());
    };
    flag("category", target.category);
    flag("milestone", &target.milestone);
    if let Some(slug) = &target.slug {
        flag("slug", slug);
    }
    if let Some(phase) = &target.phase {
        flag("phase", phase);
    }
    if let Some(file) = &target.file {
        flag("file", file);
    }
    if let Some(label) = &target.archived_milestone {
        flag("archived-milestone", label);
    }
    if let Some(ws) = &target.workstream {
        flag("ws", ws);
    }
    args
}

/// An acknowledge ready to run: what the popup shows is what [`run_ack`] runs.
#[derive(Debug, Clone)]
pub(crate) struct AckRequest {
    pub alias: String,
    /// The project path: the child's cwd.
    pub cwd: PathBuf,
    /// `node`, or a `gsd-tools` found on PATH.
    pub program: String,
    /// The launcher's prefix (the `.cjs` path), then [`ack_args`].
    pub argv: Vec<String>,
    pub target: AckTarget,
    /// The group's identity, so Yes can find it again (T-37-20).
    pub key: InboxKey,
}

impl AckRequest {
    /// The whole command line the popup shows, program first.
    pub(crate) fn command_line(&self) -> Vec<&str> {
        std::iter::once(self.program.as_str())
            .chain(self.argv.iter().map(String::as_str))
            .collect()
    }
}

/// Whether the active-workstream pointer file names a workstream. An unreadable
/// file counts as set: the safe answer is the refusal.
fn active_workstream_is_set(project: &Path) -> bool {
    match std::fs::read_to_string(project.join(".planning").join("active-workstream")) {
        Ok(text) => !text.trim().is_empty(),
        Err(err) => err.kind() != std::io::ErrorKind::NotFound,
    }
}

/// Whether a program called `name` is a file in one of the `path` directories.
fn on_path(path: Option<&std::ffi::OsStr>, name: &str) -> bool {
    let Some(path) = path else {
        return false;
    };
    std::env::split_paths(path)
        .any(|dir| dir.join(name).is_file() || dir.join(format!("{name}.exe")).is_file())
}

// ---------------------------------------------------------------------------
// `y`: copy the checklist (37-07, D-09)
// ---------------------------------------------------------------------------

/// `1 line` / `{n} lines`.
pub(crate) fn line_count(lines: usize) -> String {
    if lines == 1 {
        "1 line".to_owned()
    } else {
        format!("{lines} lines")
    }
}

/// The text `y` copies for `groups` of `alias`: exactly the lines
/// `gsd-meta-manager inbox` prints for them (`inbox::group_checklist`, one line
/// per entry, a final newline) and their count, or the refusal over 64 KiB.
pub(crate) fn copy_text(alias: &str, groups: &[OwnerGroup]) -> Result<(String, usize), String> {
    let lines: Vec<String> = groups
        .iter()
        .flat_map(|group| crate::inbox::group_checklist(alias, group))
        .collect();
    finish_copy_text(lines)
}

/// The Old debt checklist: the `OLD DEBT` heading and every archived group of
/// every registered project that has a digest, in the CLI's order.
pub(crate) fn old_debt_text(ctx: &AppContext) -> Result<(String, usize), String> {
    let mut aliases: Vec<&String> = ctx.config.projects.keys().collect();
    aliases.sort();
    let mut groups: Vec<(String, &OwnerGroup)> = Vec::new();
    for alias in aliases {
        if let Some(digest) = ctx.owner_digest(alias) {
            groups.extend(digest.groups.iter().map(|g| (alias.clone(), g)));
        }
    }
    let lines = crate::inbox::old_debt_checklist(&groups);
    if lines.is_empty() {
        return Err("Nothing to copy: no old debt is listed".to_owned());
    }
    finish_copy_text(lines)
}

fn finish_copy_text(lines: Vec<String>) -> Result<(String, usize), String> {
    if lines.is_empty() {
        return Err("Nothing to copy: the group is no longer listed".to_owned());
    }
    let mut text = lines.join("\n");
    text.push('\n');
    if text.len() > crate::clipboard::MAX_COPY_BYTES {
        return Err(format!(
            "Too large to copy ({} KiB): narrow the lane with 1-4",
            text.len().div_ceil(1024)
        ));
    }
    Ok((text, lines.len()))
}

/// What `y` does on the fold line (UI-SPEC "Folds"): Old debt copies, the other
/// two say why not.
pub(crate) fn fold_copy(
    ctx: &mut AppContext,
    fold: Option<crate::ui::screens::owner_block::Fold>,
) -> Result<ScreenAction, String> {
    use crate::ui::screens::owner_block::Fold;
    match fold {
        Some(Fold::OldDebt) => {
            let (text, lines) = old_debt_text(ctx)?;
            Ok(copy(ctx, text, lines))
        }
        Some(Fold::Acknowledged) => {
            Err("Nothing to copy: acknowledged items are already handled".to_owned())
        }
        Some(Fold::Todos) => Err("Nothing to copy: todos are only counted here".to_owned()),
        None => Err("Select a group row to copy its checklist".to_owned()),
    }
}

/// Put `text` (`lines` lines) on the clipboard: a native tool on a blocking task
/// when one is present (the status line is set by its answer, never before), else
/// an OSC 52 sequence handed to the main loop. The OSC 52 status says what is
/// true: the sequence was sent and the terminal must allow it.
pub(crate) fn copy(ctx: &mut AppContext, text: String, lines: usize) -> ScreenAction {
    ctx.needs_redraw = true;
    if let Some(tool) = crate::clipboard::pick_tool(&clipboard_probes()) {
        start_native_copy(ctx, tool, text, lines);
        return ScreenAction::None;
    }
    ctx.status_message = Some((
        format!(
            "Sent {} to the terminal clipboard (OSC 52; the terminal must allow it)",
            line_count(lines)
        ),
        std::time::Instant::now(),
    ));
    ScreenAction::WriteOsc52(crate::clipboard::osc52(&text))
}

#[cfg(not(test))]
fn clipboard_probes() -> crate::clipboard::ClipboardProbes {
    crate::clipboard::probe()
}

#[cfg(not(test))]
fn start_native_copy(
    ctx: &mut AppContext,
    tool: crate::clipboard::Tool,
    text: String,
    lines: usize,
) {
    ctx.start_copy(tool, text, lines);
}

// Test seam: no unit test reads this machine's environment or starts a real
// clipboard tool. The probes default to "no tool" (OSC 52); a test sets them and
// reads what would have been run.
#[cfg(test)]
thread_local! {
    static SEAM_PROBES: std::cell::RefCell<Option<crate::clipboard::ClipboardProbes>> =
        const { std::cell::RefCell::new(None) };
    static SEAM_RAN: std::cell::RefCell<Vec<(crate::clipboard::Tool, String, usize)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn force_clipboard(probes: Option<crate::clipboard::ClipboardProbes>) {
    SEAM_PROBES.with(|p| *p.borrow_mut() = probes);
    SEAM_RAN.with(|r| r.borrow_mut().clear());
}

/// The `(tool, text, lines)` the seam received, oldest first.
#[cfg(test)]
pub(crate) fn seam_ran() -> Vec<(crate::clipboard::Tool, String, usize)> {
    SEAM_RAN.with(|r| r.borrow().clone())
}

#[cfg(test)]
fn clipboard_probes() -> crate::clipboard::ClipboardProbes {
    SEAM_PROBES.with(|p| {
        p.borrow()
            .clone()
            .unwrap_or(crate::clipboard::ClipboardProbes {
                os: "linux",
                wayland_display: false,
                display: false,
                on_path: Vec::new(),
            })
    })
}

#[cfg(test)]
fn start_native_copy(
    _ctx: &mut AppContext,
    tool: crate::clipboard::Tool,
    text: String,
    lines: usize,
) {
    SEAM_RAN.with(|r| r.borrow_mut().push((tool, text, lines)));
}

#[cfg(test)]
thread_local! {
    /// What the next `ack_request` on this thread resolves as the launcher, when a
    /// test set one: `Some(None)` is "none installed". Per thread, so parallel
    /// tests never see each other's, and no test depends on this machine's install.
    static FORCED_LAUNCHER: std::cell::RefCell<Option<Option<GsdToolsCmd>>> =
        const { std::cell::RefCell::new(None) };
}

/// Make `ack_request` on this thread resolve `launcher` instead of searching the
/// machine (`None` is "no gsd-tools installed").
#[cfg(test)]
pub(crate) fn force_launcher(launcher: Option<GsdToolsCmd>) {
    FORCED_LAUNCHER.with(|cell| *cell.borrow_mut() = Some(launcher));
}

/// A launcher that is a plain `gsd-tools` (no `node`, so no PATH probe).
#[cfg(test)]
pub(crate) fn test_launcher() -> GsdToolsCmd {
    GsdToolsCmd {
        program: "gsd-tools".to_owned(),
        prefix_args: Vec::new(),
    }
}

/// The launcher the acknowledge writer runs: the user's own gsd-core install
/// only, never one the registered project ships (WR-05: the project is untrusted
/// input, and the child runs with the TUI's environment).
fn resolve_launcher() -> Option<GsdToolsCmd> {
    #[cfg(test)]
    if let Some(forced) = FORCED_LAUNCHER.with(|cell| cell.borrow().clone()) {
        return forced;
    }
    resolve_user_gsd_tools()
}

/// Everything `A` needs before it may open the popup, or the status line that
/// says why not (D-14, D-15). Reads one small pointer file on the key press.
pub(crate) fn ack_request(
    ctx: &AppContext,
    alias: &str,
    group: &OwnerGroup,
) -> Result<AckRequest, String> {
    let project = ctx
        .config
        .projects
        .get(alias)
        .ok_or_else(|| format!("Cannot acknowledge: {alias} is not a registered project"))?;
    let no_milestone = || format!("Cannot acknowledge: {alias} has no known milestone version");
    let state = ctx.project_states.get(alias).ok_or_else(no_milestone)?;
    let digest = ctx
        .owner_digest(alias)
        .ok_or_else(|| format!("Cannot acknowledge: {alias} has no owner data yet"))?;
    let target = ack_target_for(
        alias,
        state,
        digest,
        group,
        active_workstream_is_set(&project.path),
    )?;
    let launcher = resolve_launcher().ok_or_else(|| {
        "gsd-tools.cjs not found. Install: npx -y @opengsd/gsd-core@latest --claude --local"
            .to_owned()
    })?;
    if launcher.program == "node" && !on_path(std::env::var_os("PATH").as_deref(), "node") {
        return Err("Cannot acknowledge: node not found on PATH".to_owned());
    }
    let mut argv = launcher.prefix_args.clone();
    argv.extend(ack_args(&target));
    Ok(AckRequest {
        alias: alias.to_owned(),
        cwd: project.path.clone(),
        program: launcher.program,
        argv,
        target,
        key: InboxKey {
            alias: alias.to_owned(),
            key: GroupKey::of(group),
        },
    })
}

/// `A` (D-14, D-16) on `group` of `alias`: the popup over `host`, or the status
/// line that says why not. Both the inbox and the Owner block call it, so the
/// refusals, the busy rule and the popup are the same on both surfaces. **Called
/// from key arms only**: it reads one pointer file and starts nothing.
pub(crate) fn ack_open(
    ctx: &AppContext,
    alias: &str,
    group: &OwnerGroup,
    host: AckHost,
) -> Result<ScreenAction, String> {
    let request = ack_request(ctx, alias, group)?;
    let name = request.target.name().to_owned();
    if ctx
        .owner_ack_busy
        .contains(&(alias.to_owned(), request.target.rel_file.clone()))
    {
        return Err(format!("Already acknowledging {name}"));
    }
    Ok(ScreenAction::Push(Box::new(AckConfirmScreen::new(
        host, request,
    ))))
}

/// Yes re-checks what the popup showed (T-37-20): the group, found again by its
/// identity, must still resolve to the same target under the project's current
/// state and active-workstream pointer. Only the item count may differ, because a
/// refresh while the popup is open can change it without changing the file.
pub(crate) fn ack_recheck(
    ctx: &AppContext,
    req: &AckRequest,
    group: &OwnerGroup,
) -> Result<(), String> {
    let gone = || "Group is no longer listed; nothing acknowledged".to_owned();
    let project = ctx.config.projects.get(&req.alias).ok_or_else(gone)?;
    let state = ctx.project_states.get(&req.alias).ok_or_else(gone)?;
    let digest = ctx.owner_digest(&req.alias).ok_or_else(gone)?;
    let now = ack_target_for(
        &req.alias,
        state,
        digest,
        group,
        active_workstream_is_set(&project.path),
    )?;
    let same = AckTarget {
        items: req.target.items,
        ..now
    } == req.target;
    if same {
        Ok(())
    } else {
        Err("The file changed since the popup opened; nothing acknowledged".to_owned())
    }
}

/// The status line for a finished child (D-16, RESEARCH Pitfall 7): done on exit
/// 0, else the **last** non-empty stderr line (gsd-tools prints a defaults
/// warning first), else the first stdout line, capped at [`ACK_LINE_CAP`].
pub(crate) fn ack_outcome(name: &str, success: bool, stdout: &[u8], stderr_line: &str) -> String {
    if success {
        return format!("Acknowledged {name} via gsd-core (uncommitted)");
    }
    let from_stdout = || {
        String::from_utf8_lossy(stdout)
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or_default()
            .to_owned()
    };
    let line = if stderr_line.trim().is_empty() {
        from_stdout()
    } else {
        stderr_line.trim().to_owned()
    };
    let line: String = line.chars().take(ACK_LINE_CAP).collect();
    format!("Acknowledge failed: {line}")
}

/// What a finished child left: whether it succeeded, its stdout, and the last
/// non-empty stderr line.
type AckExit = (bool, Vec<u8>, String);

/// The bounded child's result, or why there is none: it could not start, it did
/// not finish within `budget` (it was killed), or waiting for it failed.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn spawn_ack(
    req: &AckRequest,
    budget: std::time::Duration,
) -> Result<AckExit, crate::bounded_output::BoundedFailure> {
    let args: Vec<std::ffi::OsString> = req.argv.iter().map(std::ffi::OsString::from).collect();
    let (output, stderr_line) = crate::launcher::process::try_run_bounded_with_stderr_env(
        &req.program,
        &args,
        Some(&req.cwd),
        ACK_ENV_REMOVE,
        &[],
        budget,
    )?;
    Ok((output.status.success(), output.stdout, stderr_line))
}

/// The bounded runner is Linux and macOS only, like the launcher module it lives
/// in: elsewhere the acknowledge fails visibly and runs nothing.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn spawn_ack(
    req: &AckRequest,
    _budget: std::time::Duration,
) -> Result<AckExit, crate::bounded_output::BoundedFailure> {
    tracing::warn!(cwd = %req.cwd.display(), "owner acknowledge is not supported here");
    Ok((
        false,
        Vec::new(),
        "running gsd-core is not supported on this platform".to_owned(),
    ))
}

/// The status line for an acknowledge that produced no exit status (WR-02): a
/// start failure says nothing ran, a budget overrun says nothing is confirmed.
fn ack_failure_line(
    req: &AckRequest,
    failure: crate::bounded_output::BoundedFailure,
    budget: std::time::Duration,
) -> String {
    use crate::bounded_output::BoundedFailure as F;
    match failure {
        F::TimedOut => format!(
            "Acknowledge timed out after {} s: nothing is confirmed, check git status in {}",
            budget.as_secs(),
            req.alias
        ),
        F::Spawn(kind) | F::Sink(kind) => {
            format!("Acknowledge failed: could not start gsd-tools ({kind}); nothing was run")
        }
        F::Io(kind) => format!(
            "Acknowledge failed: lost gsd-tools while it ran ({kind}); nothing is confirmed, check git status in {}",
            req.alias
        ),
    }
}

/// Run gsd-core's acknowledge writer for `req` as a bounded direct child and
/// return the status line (D-15). **Blocks up to [`ACK_BUDGET`]**: it is called
/// only on `spawn_blocking`. The full capped output goes to the tracing log.
pub(crate) fn run_ack(req: &AckRequest) -> String {
    run_ack_within(req, ACK_BUDGET)
}

/// [`run_ack`] with the budget injected, so a test can stand a short one in.
fn run_ack_within(req: &AckRequest, budget: std::time::Duration) -> String {
    let (success, stdout, stderr_line) = match spawn_ack(req, budget) {
        Ok(done) => done,
        Err(failure) => {
            tracing::warn!(alias = %req.alias, file = %req.target.rel_file, ?failure, "owner acknowledge did not finish");
            return ack_failure_line(req, failure, budget);
        }
    };
    if success {
        tracing::info!(alias = %req.alias, file = %req.target.rel_file, "owner acknowledge done");
    } else {
        let output: String = String::from_utf8_lossy(&stdout)
            .chars()
            .take(2000)
            .collect();
        tracing::warn!(
            alias = %req.alias,
            file = %req.target.rel_file,
            stderr = %stderr_line,
            stdout = %output,
            "owner acknowledge failed"
        );
    }
    ack_outcome(req.target.name(), success, &stdout, &stderr_line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::owner_items::GroupCounts;
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
            (
                group(Lane::Test, Some(verify(Some("24"), Some("-h")))),
                "Refused: -h is not a safe workstream name; nothing started",
            ),
            (
                group(Lane::Test, Some(verify(Some("24"), Some(".")))),
                "Refused: . is not a safe workstream name; nothing started",
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

    /// WR-06: the read is bounded by the cap itself, not by an earlier size check
    /// a growing or swapped file can outrun.
    #[test]
    fn read_capped_stops_one_byte_past_the_cap() {
        assert_eq!(read_capped(&b"abc"[..], 3), Some(b"abc".to_vec()));
        assert_eq!(read_capped(&b"abcd"[..], 3), None);
        assert_eq!(read_capped(&b""[..], 0), Some(Vec::new()));
        // A source with no end is refused after cap + 1 bytes, not read forever.
        assert_eq!(read_capped(std::io::repeat(b'a'), 1024), None);
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
    // ---- `A`: the acknowledge argv, its refusals and its child (37-06) ----

    use crate::state_reader::workstreams::WorkstreamState;

    fn ack_state(milestone: &str) -> ProjectState {
        ProjectState {
            milestone: milestone.to_owned(),
            workstreams: vec![WorkstreamState {
                name: "ws1".to_owned(),
                state: ProjectState {
                    milestone: "v2.1".to_owned(),
                    ..ProjectState::default()
                },
                ..WorkstreamState::default()
            }],
            ..ProjectState::default()
        }
    }

    /// A group of `kind` in `file`, phase directory token `phase`.
    fn ack_group(kind: Kind, phase: &str, file: &str) -> OwnerGroup {
        let mut g = group(Lane::Test, None);
        g.kind = kind;
        g.phase = phase.to_owned();
        g.file = file.to_owned();
        g.counts.checks = 2;
        g
    }

    /// `ACK_VERB` followed by `rest`: expectations never respell the verb.
    fn expect_args(rest: &[&str]) -> Vec<String> {
        ACK_VERB
            .iter()
            .chain(rest)
            .map(|part| (*part).to_owned())
            .collect()
    }

    fn args_for(state: &ProjectState, group: &OwnerGroup) -> Vec<String> {
        let digest = OwnerDigest {
            groups: vec![group.clone()],
            ..OwnerDigest::default()
        };
        let target = ack_target_for("alder-api", state, &digest, group, false)
            .unwrap_or_else(|err| panic!("{err}"));
        ack_args(&target)
    }

    #[test]
    fn ack_args_per_category() {
        let state = ack_state("v2.0");
        let uat = ack_group(Kind::Uat, "03", ".planning/phases/03-x/03-UAT.md");
        assert_eq!(
            args_for(&state, &uat),
            expect_args(&[
                "--category",
                "uat_gaps",
                "--milestone",
                "v2.0",
                "--phase",
                "03",
                "--file",
                "03-UAT.md"
            ])
        );
        // HUMAN-UAT is a UAT file in any lane; VERIFICATION and CONTEXT have their own.
        let human = ack_group(Kind::Uat, "03", ".planning/phases/03-x/03-HUMAN-UAT.md");
        assert!(args_for(&state, &human).contains(&"uat_gaps".to_owned()));
        let verification = ack_group(
            Kind::Verification,
            "03",
            ".planning/phases/03-x/03-VERIFICATION.md",
        );
        assert!(args_for(&state, &verification).contains(&"verification_gaps".to_owned()));
        let context = ack_group(
            Kind::ContextQuestions,
            "03",
            ".planning/phases/03-x/03-CONTEXT.md",
        );
        assert!(args_for(&state, &context).contains(&"context_questions".to_owned()));
        // Old debt names its archived milestone.
        let mut old = ack_group(
            Kind::Uat,
            "00",
            ".planning/milestones/v1.0-phases/00-a/00-UAT.md",
        );
        old.scope = Scope::Archived {
            label: "v1.0".to_owned(),
            rank: 0,
        };
        assert_eq!(
            args_for(&state, &old),
            expect_args(&[
                "--category",
                "uat_gaps",
                "--milestone",
                "v2.0",
                "--phase",
                "00",
                "--file",
                "00-UAT.md",
                "--archived-milestone",
                "v1.0"
            ])
        );
        // A workstream group takes the workstream's milestone and names `--ws`.
        let mut stream = ack_group(
            Kind::Uat,
            "03",
            ".planning/workstreams/ws1/phases/03-x/03-UAT.md",
        );
        stream.workstream = Some("ws1".to_owned());
        assert_eq!(
            args_for(&state, &stream),
            expect_args(&[
                "--category",
                "uat_gaps",
                "--milestone",
                "v2.1",
                "--phase",
                "03",
                "--file",
                "03-UAT.md",
                "--ws",
                "ws1"
            ])
        );
        // A debug session is named by its file stem.
        let debug = ack_group(Kind::Debug, "", ".planning/debug/tap-no-open-app.md");
        assert_eq!(
            args_for(&state, &debug),
            expect_args(&[
                "--category",
                "debug_sessions",
                "--milestone",
                "v2.0",
                "--slug",
                "tap-no-open-app"
            ])
        );
        // The directory token stays as the reader carries it.
        let dotted = ack_group(Kind::Uat, "18.1", ".planning/phases/18.1-x/18.1-UAT.md");
        let args = args_for(&state, &dotted);
        assert!(args.windows(2).any(|pair| pair == ["--phase", "18.1"]));
    }

    #[test]
    fn the_item_count_is_every_group_of_the_file_in_every_lane() {
        let state = ack_state("v2.0");
        let file = ".planning/phases/03-x/03-UAT.md";
        let mut a = ack_group(Kind::Uat, "03", file);
        a.counts.checks = 3;
        let mut b = ack_group(Kind::Uat, "03", file);
        b.lane = Lane::Fix;
        b.counts.checks = 2;
        let mut blocked = ack_group(Kind::Uat, "03", file);
        blocked.lane = Lane::Blocked;
        blocked.counts.checks = 4;
        let other = ack_group(Kind::Uat, "04", ".planning/phases/04-x/04-UAT.md");
        let mut other_ws = ack_group(Kind::Uat, "03", file);
        other_ws.workstream = Some("ws1".to_owned());
        let digest = OwnerDigest {
            groups: vec![a.clone(), b, blocked, other, other_ws],
            ..OwnerDigest::default()
        };
        let target = ack_target_for("alder-api", &state, &digest, &a, false).unwrap();
        assert_eq!(target.items, 9);
    }

    #[test]
    fn ack_refusals() {
        let state = ack_state("v2.0");
        let no_milestone = ack_state("");
        let phase_file = ".planning/phases/03-x/03-UAT.md";
        let mut blocked = ack_group(Kind::Uat, "03", phase_file);
        blocked.lane = Lane::Blocked;
        let mut done = ack_group(Kind::Uat, "03", phase_file);
        done.acknowledged = true;
        let mut bad_ws = ack_group(Kind::Uat, "03", phase_file);
        bad_ws.workstream = Some("a b".to_owned());
        let mut unknown_ws = ack_group(Kind::Uat, "03", phase_file);
        unknown_ws.workstream = Some("nope".to_owned());
        let mut flag_ws = ack_group(Kind::Uat, "03", phase_file);
        flag_ws.workstream = Some("--raw".to_owned());
        let mut dots_ws = ack_group(Kind::Uat, "03", phase_file);
        dots_ws.workstream = Some("..".to_owned());
        let mut bad_label = ack_group(Kind::Uat, "03", phase_file);
        bad_label.scope = Scope::Archived {
            label: "v1 0".to_owned(),
            rank: 0,
        };
        let cases: Vec<(OwnerGroup, &ProjectState, &str)> = vec![
            (
                ack_group(Kind::Handoff, "03", ".planning/HANDOFF.json"),
                &state,
                "Cannot acknowledge a handoff: gsd-core has no category for it",
            ),
            (
                ack_group(
                    Kind::Checkpoint,
                    "03",
                    ".planning/phases/03-x/.continue-here.md",
                ),
                &state,
                "Cannot acknowledge a checkpoint: gsd-core has no category for it",
            ),
            (
                blocked,
                &state,
                "Cannot acknowledge a BLOCKED row: gsd-core has no category for it",
            ),
            (done, &state, "Already acknowledged: 03-UAT.md"),
            (
                ack_group(Kind::Uat, "03", phase_file),
                &no_milestone,
                "Cannot acknowledge: alder-api has no known milestone version",
            ),
            (
                unknown_ws,
                &state,
                "Cannot acknowledge: alder-api has no known milestone version",
            ),
            (
                ack_group(Kind::Uat, "3;x", phase_file),
                &state,
                "Refused: 3;x is not a safe phase token; nothing started",
            ),
            (
                bad_ws,
                &state,
                "Refused: a b is not a safe workstream name; nothing started",
            ),
            (
                flag_ws,
                &state,
                "Refused: --raw is not a safe workstream name; nothing started",
            ),
            (
                dots_ws,
                &state,
                "Refused: .. is not a safe workstream name; nothing started",
            ),
            (
                ack_group(Kind::Uat, "03", ".planning/phases/03-x/03 UAT.md"),
                &state,
                "Refused: 03 UAT.md is not a safe file name; nothing started",
            ),
            (
                ack_group(Kind::Debug, "", ".planning/debug/a$b.md"),
                &state,
                "Refused: a$b.md is not a safe file name; nothing started",
            ),
            (
                ack_group(Kind::Uat, "03", ".planning/phases/03-x/-x.md"),
                &state,
                "Refused: -x.md is not a safe file name; nothing started",
            ),
            (
                bad_label,
                &state,
                "Refused: v1 0 is not a safe milestone label; nothing started",
            ),
        ];
        for (group, state, want) in cases {
            let digest = OwnerDigest {
                groups: vec![group.clone()],
                ..OwnerDigest::default()
            };
            let got = ack_target_for("alder-api", state, &digest, &group, false);
            assert_eq!(got.err().as_deref(), Some(want), "{want}");
        }
        // A root group is refused while a workstream pointer is set, and allowed
        // once the pointer is empty or absent; a workstream group is unaffected.
        let root = ack_group(Kind::Uat, "03", phase_file);
        let digest = OwnerDigest {
            groups: vec![root.clone()],
            ..OwnerDigest::default()
        };
        assert_eq!(
            ack_target_for("alder-api", &state, &digest, &root, true)
                .err()
                .as_deref(),
            Some("Cannot acknowledge 03-UAT.md: alder-api has an active workstream set; run it in the project")
        );
        assert!(ack_target_for("alder-api", &state, &digest, &root, false).is_ok());
        let mut stream = root.clone();
        stream.workstream = Some("ws1".to_owned());
        assert!(ack_target_for("alder-api", &state, &digest, &stream, true).is_ok());
    }

    #[test]
    fn ack_request_refuses_without_a_launcher_or_a_registered_project() {
        let (_dir, mut ctx) = project_with_uat("alder-api");
        ctx.project_states.get_mut("alder-api").unwrap().milestone = "v2.0".to_owned();
        let mut digest = OwnerDigest::default();
        let g = ack_group(Kind::Uat, "24", ".planning/phases/24-x/24-UAT.md");
        digest.groups.push(g.clone());
        ctx.owner_digests.insert("alder-api".to_owned(), digest);
        force_launcher(None);
        assert_eq!(
            ack_request(&ctx, "alder-api", &g).err().as_deref(),
            Some("gsd-tools.cjs not found. Install: npx -y @opengsd/gsd-core@latest --claude --local")
        );
        assert_eq!(
            ack_request(&ctx, "nobody", &g).err().as_deref(),
            Some("Cannot acknowledge: nobody is not a registered project")
        );
        force_launcher(Some(test_launcher()));
        let request = ack_request(&ctx, "alder-api", &g).expect("a launcher resolves");
        assert_eq!(request.program, "gsd-tools");
        assert_eq!(request.cwd, ctx.config.projects["alder-api"].path);
        assert_eq!(request.argv[..2], expect_args(&[])[..]);
    }

    #[test]
    fn node_is_looked_for_on_the_given_path_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = std::env::join_paths([dir.path()]).unwrap();
        assert!(!on_path(Some(&path), "node"));
        assert!(!on_path(None, "node"));
        std::fs::write(dir.path().join("node"), "").unwrap();
        assert!(on_path(Some(&path), "node"));
    }

    #[test]
    fn the_pointer_file_decides_whether_an_active_workstream_is_set() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".planning")).unwrap();
        assert!(!active_workstream_is_set(dir.path()), "absent");
        let pointer = dir.path().join(".planning/active-workstream");
        std::fs::write(&pointer, "\n  \n").unwrap();
        assert!(!active_workstream_is_set(dir.path()), "empty");
        std::fs::write(&pointer, "ws1\n").unwrap();
        assert!(active_workstream_is_set(dir.path()), "named");
    }

    #[test]
    fn run_ack_maps_outcomes() {
        assert_eq!(
            ack_outcome("03-UAT.md", true, b"", ""),
            "Acknowledged 03-UAT.md via gsd-core (uncommitted)"
        );
        // The last stderr line, not the defaults warning before it.
        assert_eq!(
            ack_outcome(
                "03-UAT.md",
                false,
                b"",
                "no phase directory found for phase \"3\""
            ),
            "Acknowledge failed: no phase directory found for phase \"3\""
        );
        // No stderr: the first stdout line.
        assert_eq!(
            ack_outcome("03-UAT.md", false, b"\n  refused here\nmore\n", ""),
            "Acknowledge failed: refused here"
        );
        // Capped at 120 characters.
        let long = "x".repeat(300);
        let line = ack_outcome("03-UAT.md", false, b"", &long);
        assert_eq!(line, format!("Acknowledge failed: {}", "x".repeat(120)));
    }

    /// The real bounded child, with `sh` standing in for gsd-core: exit 0, a failure
    /// whose stderr starts with a warning, and a child that outlives its budget is
    /// covered by `process.rs`; here only the mapping over real exits.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn run_ack_runs_a_bounded_child_and_maps_its_exit() {
        let (_dir, mut ctx) = project_with_uat("alder-api");
        ctx.project_states.get_mut("alder-api").unwrap().milestone = "v2.0".to_owned();
        let g = ack_group(Kind::Uat, "24", ".planning/phases/24-x/24-UAT.md");
        ctx.owner_digests.insert(
            "alder-api".to_owned(),
            OwnerDigest {
                groups: vec![g.clone()],
                ..OwnerDigest::default()
            },
        );
        force_launcher(Some(test_launcher()));
        let mut request = ack_request(&ctx, "alder-api", &g).unwrap();
        request.program = "sh".to_owned();
        request.argv = vec!["-c".to_owned(), "exit 0".to_owned()];
        assert_eq!(
            run_ack(&request),
            "Acknowledged 24-UAT.md via gsd-core (uncommitted)"
        );
        request.argv = vec![
            "-c".to_owned(),
            "echo 'gsd-tools: warning: defaults' >&2; echo 'no phase directory found' >&2; exit 1"
                .to_owned(),
        ];
        assert_eq!(
            run_ack(&request),
            "Acknowledge failed: no phase directory found"
        );
    }

    /// WR-02: a child that never started is not "timed out", and the real
    /// timeout names the budget it was given.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn run_ack_tells_a_spawn_failure_from_a_timeout() {
        let (_dir, mut ctx) = project_with_uat("alder-api");
        ctx.project_states.get_mut("alder-api").unwrap().milestone = "v2.0".to_owned();
        let g = ack_group(Kind::Uat, "24", ".planning/phases/24-x/24-UAT.md");
        ctx.owner_digests.insert(
            "alder-api".to_owned(),
            OwnerDigest {
                groups: vec![g.clone()],
                ..OwnerDigest::default()
            },
        );
        force_launcher(Some(test_launcher()));
        let mut request = ack_request(&ctx, "alder-api", &g).unwrap();
        request.program = "/nonexistent/program".to_owned();
        let line = run_ack(&request);
        assert!(
            line.starts_with("Acknowledge failed: could not start gsd-tools"),
            "{line}"
        );
        assert!(line.ends_with("nothing was run"), "{line}");
        assert!(!line.contains("timed out"), "{line}");
        // A project folder that vanished fails the same way, as a spawn error.
        request.program = "sh".to_owned();
        request.argv = vec!["-c".to_owned(), "exit 0".to_owned()];
        request.cwd = request.cwd.join("moved-away");
        let line = run_ack(&request);
        assert!(
            line.starts_with("Acknowledge failed: could not start"),
            "{line}"
        );
        // A child that outlives a short budget really is a timeout.
        request.cwd = _dir.path().to_path_buf();
        request.argv = vec!["-c".to_owned(), "exec sleep 30".to_owned()];
        assert_eq!(
            run_ack_within(&request, std::time::Duration::from_secs(1)),
            "Acknowledge timed out after 1 s: nothing is confirmed, check git status in alder-api"
        );
    }

    fn item(name: &str) -> crate::state_reader::owner_items::OwnerItem {
        crate::state_reader::owner_items::OwnerItem {
            id: name.to_owned(),
            test: None,
            name: name.to_owned(),
            expected: None,
            result: "pending".to_owned(),
            category: crate::state_reader::owner_items::Category::HumanUat,
            reason: None,
            blocked_by: None,
            gaps: Vec::new(),
            why_human: None,
        }
    }

    /// `y` copies exactly the lines `gsd-meta-manager inbox` prints for the group:
    /// they are a contiguous run inside `render_text` of the same project.
    #[test]
    fn copy_text_equals_the_cli_checklist() {
        let mut g = group(Lane::Test, Some(verify(Some("24"), None)));
        g.items = vec![item("first check"), item("second \u{1b}[2Jcheck")];
        let (text, lines) = copy_text("dune-app", std::slice::from_ref(&g)).expect("text");
        assert_eq!(lines, 3);
        assert!(text.ends_with('\n'));
        assert!(!text.contains('\u{1b}'), "the CLI's terminal_capped text");
        let entry = crate::inbox::ProjectEntry {
            target: None,
            alias: Some("dune-app".to_owned()),
            path: None,
            error: None,
            milestone_version: None,
            phases_done: 0,
            phases_total: 0,
            digest: crate::state_reader::owner_items::OwnerDigest {
                groups: vec![g],
                ..Default::default()
            },
            todos: Vec::new(),
        };
        let printed = crate::inbox::render_text(&[entry], &crate::inbox::Filters::default());
        let printed: Vec<&str> = printed.lines().collect();
        let copied: Vec<&str> = text.lines().collect();
        assert!(
            printed
                .windows(copied.len())
                .any(|w| w == copied.as_slice()),
            "{printed:?} does not hold {copied:?}"
        );
    }

    #[test]
    fn too_large_to_copy() {
        let mut g = group(Lane::Test, None);
        g.items = (0..2000)
            .map(|i| item(&format!("check number {i:04} {}", "x".repeat(60))))
            .collect();
        let message = copy_text("p", &[g]).expect_err("over 64 KiB");
        assert!(
            message.starts_with("Too large to copy (")
                && message.ends_with(" KiB): narrow the lane with 1-4"),
            "{message}"
        );
        // IN-05: one byte over the limit never prints as the limit itself.
        let just_over = finish_copy_text(vec!["x".repeat(crate::clipboard::MAX_COPY_BYTES)]);
        assert_eq!(
            just_over,
            Err("Too large to copy (65 KiB): narrow the lane with 1-4".to_owned())
        );
        force_clipboard(None);
        assert!(seam_ran().is_empty(), "nothing was sent");
        assert_eq!(
            copy_text("p", &[]),
            Err("Nothing to copy: the group is no longer listed".to_owned())
        );
    }

    #[test]
    fn line_count_pluralizes() {
        assert_eq!(line_count(1), "1 line");
        assert_eq!(line_count(7), "7 lines");
    }

    /// The native path claims nothing until the tool answers, then reports through
    /// the event channel; a failure says which tool and why, with no OSC 52 retry.
    #[tokio::test]
    async fn start_copy_reports_the_tools_answer() {
        use crate::clipboard::Tool;
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let mut ctx = ctx_with_aliases(&["p"]);
        ctx.event_tx = Some(tx);
        ctx.start_copy_with(Tool::WlCopy, "a\nb\n".to_owned(), 2, |tool, text| {
            assert_eq!((tool, text), (Tool::WlCopy, "a\nb\n"));
            Ok(())
        });
        assert!(ctx.status_message.is_none(), "no claim before the answer");
        let Some(crate::action::Action::ClipboardCopied { outcome }) = rx.recv().await else {
            panic!("the task answers");
        };
        assert_eq!(outcome, "Copied 2 lines via wl-copy");
        ctx.start_copy_with(Tool::Xclip, "a\n".to_owned(), 1, |_, _| {
            Err("exited 3".to_owned())
        });
        let Some(crate::action::Action::ClipboardCopied { outcome }) = rx.recv().await else {
            panic!("the task answers");
        };
        assert_eq!(outcome, "Copy failed: xclip exited 3");
        ctx.start_copy_with(Tool::Xsel, "a\n".to_owned(), 1, |_, _| {
            Err("did not finish".to_owned())
        });
        let Some(crate::action::Action::ClipboardCopied { outcome }) = rx.recv().await else {
            panic!("the task answers");
        };
        assert_eq!(outcome, "Copy failed: xsel did not finish");
        ctx.start_copy_with(Tool::Pbcopy, "a\n".to_owned(), 1, |_, _| panic!("boom"));
        let Some(crate::action::Action::ClipboardCopied { outcome }) = rx.recv().await else {
            panic!("the task answers even when the runner panics");
        };
        assert!(outcome.starts_with("Copy failed: pbcopy"), "{outcome}");
        // No event channel: refused visibly, nothing started.
        ctx.event_tx = None;
        ctx.start_copy_with(Tool::WlCopy, "a\n".to_owned(), 1, |_, _| panic!("not run"));
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Copy failed: this session has no event channel")
        );
    }
}
