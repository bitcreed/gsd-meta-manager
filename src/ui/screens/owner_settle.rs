//! Settle (`x`): record an outcome for TEST and BLOCKED checks by launching
//! `/gsd-verify-work` with a plain-language instruction (39, D-07).
//!
//! **The command is typed.** It is a [`RunCommand`] whose phase and workstream are
//! re-validated with `is_command_token` / `is_workstream_name` (37 D-11, CWE-88),
//! and it travels with the instruction as ONE argv element (see
//! `detail::settle_mux_argv`). Only `RunCommand`'s `Display` spells the slash
//! command.
//!
//! **The instruction is built only from typed values passed through [`clean`]**,
//! so the text the popup draws equals the argv text (`shown(prompt) == prompt`)
//! and nothing in a check name or the owner's reason can break verify-work's
//! `sed | xargs` argument parse or smuggle a second `--ws`.
//!
//! **Nothing is typed into a live session** (37 D-12): a project that already runs
//! Claude gets that session focused, never a second one.
//!
//! **This module and `settle_popup.rs` write no file** (D-13a). gsd-core records
//! the outcome (and, for a move, the todo) when the launched command runs.

use crate::session_detector::ClaudeSession;
use crate::state_reader::owner_items::{
    is_command_token, is_workstream_name, Kind, Lane, OwnerDigest, OwnerGroup, OwnerItem,
    RunCommand, RunVerb, Scope,
};
use crate::state_reader::ProjectState;
use crate::ui::screens::{AppContext, ScreenAction};
use std::path::PathBuf;

use super::ack_confirm::AckHost;
use super::owner_actions::active_workstream_is_set;
use super::owner_block::{self, GroupKey};
use super::settle_popup::SettleChooserScreen;

/// The prefix of the reason gsd-core treats as a non-blocking deferral (RESEARCH
/// Pitfall 1): the only `result: skipped` that neither gsd-core nor the reader
/// counts as an open check.
pub const DEFERRED_PREFIX: &str = "Deferred follow-up:";
/// The most characters of the owner's reason (D-02).
pub const REASON_MAX_CHARS: usize = 200;
/// The most characters of a check name or file name in the instruction (D-08).
pub const NAME_MAX_CHARS: usize = 80;
/// The most check numbers of one file listed before `all N open tests` (D-08).
pub const LIST_MAX_NUMBERS: usize = 20;

/// What the owner decides about the checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The owner will not do these checks.
    WontDo,
    /// The owner defers these checks for good.
    DeferPermanently,
}

impl Outcome {
    /// The popup's `Outcome` row.
    pub fn label(self) -> &'static str {
        match self {
            Outcome::WontDo => "Won't do",
            Outcome::DeferPermanently => "Deferred permanently",
        }
    }

    /// The status line's wording.
    pub fn phrase(self) -> &'static str {
        match self {
            Outcome::WontDo => "won't do",
            Outcome::DeferPermanently => "deferred permanently",
        }
    }
}

/// Whether the settle covers one check or every check of the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettleScope {
    /// One check, by its `OwnerItem::id`.
    Check(String),
    /// Every check behind the row.
    Phase,
}

/// One check as the instruction names it. Every text field already went through
/// [`clean`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckRef {
    /// The basename of the check's file.
    pub file: String,
    /// `test` for a UAT file, `item` for every other kind.
    pub noun: &'static str,
    /// The check's own number, else its 1-based position in its group.
    pub number: String,
    /// The check's name.
    pub name: String,
}

/// What a settle is about, fully resolved and validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleTarget {
    pub alias: String,
    /// The row's identity, so Yes can find it again.
    pub key: GroupKey,
    pub command: RunCommand,
    pub scope: SettleScope,
    pub checks: Vec<CheckRef>,
}

impl SettleTarget {
    /// The phase token of the command.
    pub fn phase(&self) -> &str {
        self.command.phase.as_deref().unwrap_or_default()
    }

    /// `phase {N}` for a phase target, `{noun} {number}` for one check.
    pub fn short(&self) -> String {
        match (&self.scope, self.checks.first()) {
            (SettleScope::Check(_), Some(check)) => format!("{} {}", check.noun, check.number),
            _ => format!("phase {}", self.phase()),
        }
    }
}

/// The owner's choice. The reason is already cleaned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleChoice {
    pub outcome: Outcome,
    pub reason: String,
}

/// A resolved target with the project facts a launch needs.
#[derive(Debug, Clone)]
pub(crate) struct SettleDraft {
    pub target: SettleTarget,
    /// The project path: the child's cwd, never part of the argv.
    pub cwd: PathBuf,
    /// A live session whose working directory IS the project path (37 D-12).
    pub live_session: Option<ClaudeSession>,
}

/// A settle ready to confirm: what the popup shows is what Yes launches.
#[derive(Debug, Clone)]
pub(crate) struct SettlePlan {
    pub draft: SettleDraft,
    pub choice: SettleChoice,
}

impl SettlePlan {
    /// The ONE argv element after `claude`: the slash command and the instruction.
    pub(crate) fn prompt(&self) -> String {
        prompt(&self.draft.target, &self.choice)
    }
}

/// `text` made safe to put in the prompt and in the popup: no control or invisible
/// formatting character, whitespace runs become one space, ASCII `'` and `"`
/// become typographic quotes and `\` becomes `/` (verify-work parses its arguments
/// with `sed | xargs`, which breaks on all three; RESEARCH Pitfall 2), runs of `-`
/// collapse to one (so free text never spells `--ws`, Pitfall 3), and a text longer
/// than `cap` characters is cut to `cap` characters plus `\u{2026}` (Pitfall 9).
/// Every cap counts characters, never bytes.
pub fn clean(text: &str, cap: usize) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last: Option<char> = None;
    for c in text.chars() {
        let mapped = if c.is_whitespace() {
            ' '
        } else if c.is_control() || crate::text::is_invisible_formatting_char(c) {
            continue;
        } else {
            match c {
                '\'' => '\u{2019}',
                '"' => '\u{201d}',
                '\\' => '/',
                other => other,
            }
        };
        if mapped == ' ' && matches!(last, None | Some(' ')) {
            continue;
        }
        if mapped == '-' && last == Some('-') {
            continue;
        }
        out.push(mapped);
        last = Some(mapped);
    }
    let out = out.trim_end();
    if out.chars().count() > cap {
        let mut cut: String = out.chars().take(cap).collect();
        cut.push('\u{2026}');
        cut
    } else {
        out.to_owned()
    }
}

/// One check of `group`, named for the instruction.
fn check_ref(group: &OwnerGroup, item: &OwnerItem, position: usize) -> CheckRef {
    let basename = group.file.rsplit('/').next().unwrap_or(&group.file);
    let number = item
        .test
        .as_deref()
        .map(|test| clean(test, 16))
        .filter(|number| !number.is_empty())
        .unwrap_or_else(|| (position + 1).to_string());
    CheckRef {
        file: clean(basename, NAME_MAX_CHARS),
        noun: if group.kind == Kind::Uat {
            "test"
        } else {
            "item"
        },
        number,
        name: clean(&item.name, NAME_MAX_CHARS),
    }
}

/// The groups behind the row `key`: a merged entry's parts, a BLOCKED row's
/// groups, else the one group.
fn groups_behind(digest: &OwnerDigest, key: &GroupKey, fold_days: u32) -> Vec<OwnerGroup> {
    if key.kind == "entry" {
        owner_block::entry_for(digest, key, fold_days)
            .map(|entry| entry.parts)
            .unwrap_or_default()
    } else {
        owner_block::groups_for_copy(digest, key)
    }
}

/// Resolve the row `key` (and, for one check, `item`) of `alias` into a
/// [`SettleTarget`], or the status line that says why nothing starts. Pure over
/// its arguments, so the refusal table is testable without a screen.
pub fn settle_target_for(
    alias: &str,
    _state: &ProjectState,
    digest: &OwnerDigest,
    key: &GroupKey,
    item: Option<&str>,
    fold_days: u32,
    _active_workstream_set: bool,
) -> Result<SettleTarget, String> {
    let groups = groups_behind(digest, key, fold_days);
    let first = groups
        .first()
        .ok_or_else(|| "Row is no longer listed; nothing started".to_owned())?;
    if first.lane != Lane::Test || !matches!(first.scope, Scope::Current) {
        return Err("Cannot settle this row".to_owned());
    }
    let command = first
        .run
        .clone()
        .filter(|run| run.verb == RunVerb::VerifyWork)
        .ok_or_else(|| "Cannot settle this row".to_owned())?;
    if let Some(token) = command.phase.as_deref() {
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
    let mut checks = Vec::new();
    let mut scope = SettleScope::Phase;
    for group in &groups {
        for (position, it) in group.items.iter().enumerate() {
            match item {
                Some(id) if it.id != id => {}
                Some(id) => {
                    if checks.is_empty() {
                        scope = SettleScope::Check(id.to_owned());
                        checks.push(check_ref(group, it, position));
                    }
                }
                None => checks.push(check_ref(group, it, position)),
            }
        }
    }
    if checks.is_empty() {
        return Err(if item.is_some() {
            "Check is no longer listed; nothing started"
        } else {
            "Row is no longer listed; nothing started"
        }
        .to_owned());
    }
    Ok(SettleTarget {
        alias: alias.to_owned(),
        key: key.clone(),
        command,
        scope,
        checks,
    })
}

/// How the instruction names the checks (`R`): one check as `test 2 (\u{201c}name\u{201d})
/// in FILE`, a phase as its files in row order, each with its numbers.
pub fn target_text(target: &SettleTarget) -> String {
    if let (SettleScope::Check(_), Some(check)) = (&target.scope, target.checks.first()) {
        let name = if check.name.is_empty() {
            String::new()
        } else {
            format!(" (\u{201c}{}\u{201d})", check.name)
        };
        return format!("{} {}{name} in {}", check.noun, check.number, check.file);
    }
    // (file, noun, numbers), files in first-seen order.
    let mut files: Vec<(&str, &'static str, Vec<&str>)> = Vec::new();
    for check in &target.checks {
        match files.iter_mut().find(|(file, _, _)| *file == check.file) {
            Some((_, _, numbers)) => numbers.push(&check.number),
            None => files.push((&check.file, check.noun, vec![&check.number])),
        }
    }
    files
        .iter()
        .map(|(file, noun, numbers)| {
            let plural = if *noun == "test" { "tests" } else { "items" };
            if numbers.len() > LIST_MAX_NUMBERS {
                format!("all {} open {plural} in {file}", numbers.len())
            } else {
                let noun = if numbers.len() == 1 { noun } else { plural };
                format!("{noun} {} in {file}", numbers.join(", "))
            }
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

/// The plain-language instruction after the slash command (D-08).
pub fn instruction(target: &SettleTarget, choice: &SettleChoice) -> String {
    let r = target_text(target);
    let n = target.phase();
    let (each, it, upper_it) = match (target.checks.len(), target.checks.first()) {
        (1, Some(check)) => (format!("{} {}", check.noun, check.number), "it", "It"),
        _ => ("each of them".to_owned(), "them", "They"),
    };
    let reason = if choice.reason.is_empty() {
        String::new()
    } else {
        let stop = if choice.reason.ends_with(['.', '!', '?']) {
            ""
        } else {
            "."
        };
        format!(" {}{stop}", choice.reason)
    };
    let (lead, outcome) = match choice.outcome {
        Outcome::WontDo => (format!("The user will not perform {r}."), "will not do."),
        Outcome::DeferPermanently => (
            format!("The user defers {r} permanently."),
            "deferred permanently.",
        ),
    };
    format!(
        "{lead} Resume the UAT session of phase {n} without restarting it and record {each} as result: skipped, reason: {DEFERRED_PREFIX} {outcome}{reason} {upper_it} must not block phase {n} or be asked again. Keep {it} in the UAT file and do not promote {it} to the 999.x backlog."
    )
}

/// The whole argument of the new session: the typed slash command, a space and the
/// instruction.
pub fn prompt(target: &SettleTarget, choice: &SettleChoice) -> String {
    format!("{} {}", target.command, instruction(target, choice))
}

/// The refusal text a Yes-time lookup shows: the same words, `launched` for
/// `started`.
fn at_yes(message: String) -> String {
    match message.strip_suffix("nothing started") {
        Some(head) => format!("{head}nothing launched"),
        None => message,
    }
}

/// The draft for the row `key` of `alias` under the current registry, state,
/// digest, pointer file and session list.
pub(crate) fn resolve_draft(
    ctx: &AppContext,
    alias: &str,
    key: &GroupKey,
    item: Option<&str>,
) -> Result<SettleDraft, String> {
    let project = ctx
        .config
        .projects
        .get(alias)
        .ok_or_else(|| format!("Cannot settle: {alias} is not a registered project"))?;
    let state = ctx
        .project_states
        .get(alias)
        .ok_or_else(|| format!("Cannot settle: {alias} has no project state yet"))?;
    let digest = ctx
        .owner_digest(alias)
        .ok_or_else(|| format!("Cannot settle: {alias} has no owner data yet"))?;
    let target = settle_target_for(
        alias,
        state,
        digest,
        key,
        item,
        ctx.config.preferences.owner_fold_days(),
        active_workstream_is_set(&project.path),
    )?;
    // Exact path equality (T-37-06): a sibling directory that merely shares the
    // project path as a string prefix is a different project.
    let live_session = ctx
        .active_sessions
        .iter()
        .find(|session| session.working_dir == project.path)
        .cloned();
    Ok(SettleDraft {
        target,
        cwd: project.path.clone(),
        live_session,
    })
}

/// `x` on the row `key` (and one check `item`, if any) of `alias`: the chooser over
/// `host`, or the status line that says why not. **Called from key arms only**: it
/// reads one pointer file and starts nothing.
pub(crate) fn settle_open(
    ctx: &AppContext,
    alias: &str,
    key: &GroupKey,
    item: Option<&str>,
    host: AckHost,
) -> Result<ScreenAction, String> {
    let draft = resolve_draft(ctx, alias, key, item)?;
    Ok(ScreenAction::Push(Box::new(SettleChooserScreen::new(
        host, draft,
    ))))
}

/// Yes re-checks what the popup showed (T-39-04): the row, found again by its
/// identity, must still resolve to the same prompt and project path, and a live
/// session must still be there (or still be absent). Anything else says why and
/// launches nothing. The fresh plan is the one launched against.
pub(crate) fn settle_recheck(ctx: &AppContext, plan: &SettlePlan) -> Result<SettlePlan, String> {
    let target = &plan.draft.target;
    let item = match &target.scope {
        SettleScope::Check(id) => Some(id.as_str()),
        SettleScope::Phase => None,
    };
    let gone = |message: String| {
        if message.starts_with("Row is no longer listed") {
            "Row is no longer listed; nothing launched".to_owned()
        } else {
            at_yes(message)
        }
    };
    let draft = resolve_draft(ctx, &target.alias, &target.key, item).map_err(gone)?;
    let fresh = SettlePlan {
        draft,
        choice: plan.choice.clone(),
    };
    if fresh.prompt() != plan.prompt() || fresh.draft.cwd != plan.draft.cwd {
        return Err("The row changed since the popup opened; nothing launched".to_owned());
    }
    if fresh.draft.live_session.is_some() != plan.draft.live_session.is_some() {
        return Err(format!(
            "A session started or ended in {} since the popup opened; nothing launched",
            target.alias
        ));
    }
    Ok(fresh)
}

/// The status line after a launch (D-12): `Launched {command} in {alias}: {phrase}
/// for {target}`.
pub(crate) fn launched_status(plan: &SettlePlan) -> String {
    let target = &plan.draft.target;
    let what = match target.scope {
        SettleScope::Check(_) => target.short(),
        SettleScope::Phase => {
            let n = target.checks.len();
            let noun = if n == 1 { "check" } else { "checks" };
            format!("{} ({n} {noun})", target.short())
        }
    };
    format!(
        "Launched {} in {}: {} for {what}",
        target.command,
        target.alias,
        plan.choice.outcome.phrase()
    )
}

/// Launch the plan, or focus the live session (37 D-12). The real launch is
/// reached only from here; unit tests record the plan on a thread-local instead,
/// so none of them touches a multiplexer or a terminal.
pub(crate) fn launch_settle(plan: &SettlePlan) -> String {
    #[cfg(not(test))]
    {
        launch_settle_real(plan)
    }
    #[cfg(test)]
    {
        SETTLE_LAUNCHES.with(|launches| launches.borrow_mut().push(plan.clone()));
        match &plan.draft.live_session {
            Some(_) => format!(
                "{} {}",
                crate::multiplexer::FOCUS_OK_PREFIX,
                plan.draft.target.alias
            ),
            None => launched_status(plan),
        }
    }
}

/// The real action behind Yes: focus the live session or launch a new one. The
/// project path is the child's cwd, the prompt is ONE argv element.
#[cfg(not(test))]
fn launch_settle_real(plan: &SettlePlan) -> String {
    use super::detail::{resolve_launch_plan, settle_mux_argv, settle_terminal_argv};
    if let Some(session) = &plan.draft.live_session {
        return crate::multiplexer::focus_message(session, &plan.draft.target.alias);
    }
    crate::multiplexer::launch(
        &resolve_launch_plan(),
        &plan.draft.cwd,
        settle_mux_argv(plan),
        &|term| settle_terminal_argv(term, plan),
        &launched_status(plan),
    )
}

#[cfg(test)]
thread_local! {
    static SETTLE_LAUNCHES: std::cell::RefCell<Vec<SettlePlan>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Every plan [`launch_settle`] recorded on this thread, drained.
#[cfg(test)]
pub(crate) fn take_settle_launches() -> Vec<SettlePlan> {
    SETTLE_LAUNCHES.with(|launches| std::mem::take(&mut *launches.borrow_mut()))
}
