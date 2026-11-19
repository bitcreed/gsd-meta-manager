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
    command_phase, is_command_token, is_workstream_name, scope_state, Kind, Lane, OwnerDigest,
    OwnerGroup, OwnerItem, RunCommand, RunVerb, Scope,
};
use crate::state_reader::phase_num::same_phase;
use crate::state_reader::ProjectState;
use crate::ui::screens::{AppContext, ScreenAction};
use std::path::{Path, PathBuf};

use super::ack_confirm::AckHost;
use super::owner_actions::active_workstream_is_set;
use super::owner_block::{self, GroupKey};
use super::settle_pending;
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
    /// The owner moves these checks to another phase; GSD files a todo for it.
    Move,
}

impl Outcome {
    /// The popup's `Outcome` row.
    pub fn label(self) -> &'static str {
        match self {
            Outcome::WontDo => "Won't do",
            Outcome::DeferPermanently => "Deferred permanently",
            Outcome::Move => "Move to phase",
        }
    }

    /// The status line's wording.
    pub fn phrase(self) -> &'static str {
        match self {
            Outcome::WontDo => "won't do",
            Outcome::DeferPermanently => "deferred permanently",
            Outcome::Move => "moved",
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

/// The phase a Move goes to (D-01, D-06). Its name already went through [`clean`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveTarget {
    /// The phase token, as the roadmap spells it (a typed unknown phase: as typed).
    /// Always passes `is_command_token`.
    pub number: String,
    /// The roadmap name, or the backlog slug as words.
    pub name: Option<String>,
    /// Whether the roadmap or the backlog lists the phase (D-06: an unknown one is
    /// accepted and warned about in the popup).
    pub known: bool,
}

/// The owner's choice. The reason is already cleaned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettleChoice {
    pub outcome: Outcome,
    pub reason: String,
    /// The phase a Move goes to; `Some` exactly for [`Outcome::Move`].
    pub move_to: Option<MoveTarget>,
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

/// The status line `x` prints on a row whose source file is unreadable.
pub(crate) const UNREADABLE_REFUSAL: &str = "Cannot settle: the source file is unreadable";

/// Why one group's checks cannot be settled: the text `x` prints, and whether the
/// refusal is permanent for the group's kind (`Never`: the row model hides `x`) or
/// a state of this one file (`Unreadable`: the model shows `x` dim, with the text).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettleRefusal {
    pub(crate) message: &'static str,
    pub(crate) unreadable: bool,
}

/// The lane and scope rule of [`settle_target_for`] for one group, pure: FIX,
/// DECIDE and deferred lanes, archived and acknowledged files, then an unreadable
/// file, in that order. The one copy, shared by the target builder and the row
/// action model, so a hint and the key cannot disagree.
pub(crate) fn settle_group_refusal(group: &OwnerGroup) -> Result<(), SettleRefusal> {
    let never = |message: &'static str| {
        Err(SettleRefusal {
            message,
            unreadable: false,
        })
    };
    match group.lane {
        Lane::Fix => return never("Cannot settle a FIX row: it needs code; v plans the fix"),
        Lane::Decide => return never("Cannot settle a DECIDE row: it is not a check"),
        Lane::Deferred => return never("Cannot settle a deferred row: it is already deferred"),
        Lane::Test | Lane::Blocked => {}
    }
    if matches!(group.scope, Scope::Archived { .. }) {
        return never(
            "Cannot settle old debt: an archived phase is not reachable by /gsd-verify-work",
        );
    }
    if group.acknowledged {
        return never("Cannot settle an acknowledged file: it is already handled");
    }
    if group.counts.unreadable > 0 {
        return Err(SettleRefusal {
            message: UNREADABLE_REFUSAL,
            unreadable: true,
        });
    }
    Ok(())
}

/// The groups behind the row `key`: a merged entry's parts, a BLOCKED row's
/// groups, else the one group.
pub(crate) fn groups_behind(
    digest: &OwnerDigest,
    key: &GroupKey,
    fold_days: u32,
) -> Vec<OwnerGroup> {
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
    state: &ProjectState,
    digest: &OwnerDigest,
    key: &GroupKey,
    item: Option<&str>,
    fold_days: u32,
    active_workstream_set: bool,
) -> Result<SettleTarget, String> {
    let groups = groups_behind(digest, key, fold_days);
    let first = groups
        .first()
        .ok_or_else(|| "Row is no longer listed; nothing started".to_owned())?;
    // D-03: only TEST and BLOCKED checks of the current milestone can be settled.
    for group in &groups {
        settle_group_refusal(group).map_err(|refusal| refusal.message.to_owned())?;
    }
    let workstream = match first.workstream.as_deref() {
        Some(name) if !is_workstream_name(name) => {
            return Err(format!(
                "Refused: {name} is not a safe workstream name; nothing started"
            ));
        }
        other => other.map(str::to_owned),
    };
    let token = command_phase(state, workstream.as_deref(), &first.phase);
    if !is_command_token(&token) {
        return Err(format!(
            "Refused: {token} is not a safe phase token; nothing started"
        ));
    }
    // T-37-18 analog: a root-scoped command while the pointer names a workstream
    // would record into that workstream.
    if workstream.is_none() && active_workstream_set {
        return Err(format!(
            "Cannot settle phase {token}: {alias} has an active workstream set; run it in the project"
        ));
    }
    let command = RunCommand {
        verb: RunVerb::VerifyWork,
        phase: Some(token),
        workstream,
    };
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

/// The owner's reason as a trailing sentence: a leading space and a closing
/// period unless it already ends in one. Empty when there is no reason.
fn reason_sentence(reason: &str) -> String {
    if reason.is_empty() {
        return String::new();
    }
    let stop = if reason.ends_with(['.', '!', '?']) {
        ""
    } else {
        "."
    };
    format!(" {reason}{stop}")
}

/// The plain-language instruction after the slash command (D-08).
pub fn instruction(target: &SettleTarget, choice: &SettleChoice) -> String {
    let r = target_text(target);
    let n = target.phase();
    let (each, it, upper_it) = match (target.checks.len(), target.checks.first()) {
        (1, Some(check)) => (format!("{} {}", check.noun, check.number), "it", "It"),
        _ => ("each of them".to_owned(), "them", "They"),
    };
    let reason = reason_sentence(&choice.reason);
    let (lead, outcome) = match choice.outcome {
        Outcome::WontDo => (format!("The user will not perform {r}."), "will not do."),
        Outcome::DeferPermanently => (
            format!("The user defers {r} permanently."),
            "deferred permanently.",
        ),
        Outcome::Move => return move_instruction(target, choice, &r, (&each, it, upper_it)),
    };
    format!(
        "{lead} Resume the UAT session of phase {n} without restarting it and record {each} as result: skipped, reason: {DEFERRED_PREFIX} {outcome}{reason} {upper_it} must not block phase {n} or be asked again. Keep {it} in the UAT file and do not promote {it} to the 999.x backlog."
    )
}

/// The Move template (D-07, D-08, D-09; RESEARCH Pitfalls 5 and 12): the deferral
/// record gsd-core counts as non-blocking, then ONE todo requested the way
/// `/gsd-capture` files it, titled with phase M's number and roadmap name so
/// gsd-core's keyword match finds it when M runs. The app runs no todo command.
fn move_instruction(
    target: &SettleTarget,
    choice: &SettleChoice,
    r: &str,
    (each, it, upper_it): (&str, &str, &str),
) -> String {
    let n = target.phase();
    let to = choice
        .move_to
        .as_ref()
        .expect("a Move choice carries its target");
    let m = &to.number;
    let (quoted, bare) = match &to.name {
        Some(name) => (format!(" \u{201c}{name}\u{201d}"), format!(" {name}")),
        None => (String::new(), String::new()),
    };
    let from = match (&target.scope, target.checks.first()) {
        (SettleScope::Check(_), Some(check)) => format!("{} {}", check.noun, check.number),
        _ => {
            let k = target.checks.len();
            format!("{k} {}", if k == 1 { "check" } else { "checks" })
        }
    };
    let reason = reason_sentence(&choice.reason);
    let comes = if it == "it" { "it comes" } else { "they come" };
    format!(
        "The user moves {r} to phase {m}{quoted}. Resume the UAT session of phase {n} without restarting it and record {each} as result: skipped, reason: {DEFERRED_PREFIX} moved to phase {m}.{reason} {upper_it} must not block phase {n}. Then add one todo as /gsd-capture does, titled \u{201c}Phase {m}{bare}: rerun {from} from phase {n}\u{201d}, so {comes} up again when phase {m} runs. Do not promote {it} to the 999.x backlog."
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

/// What every settle draft reads: the project's path, its state and its Owner
/// digest, or the status line that says which one is missing.
fn settle_inputs<'a>(
    ctx: &'a AppContext,
    alias: &str,
) -> Result<(&'a Path, &'a ProjectState, &'a OwnerDigest), String> {
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
    Ok((project.path.as_path(), state, digest))
}

/// Why `x` would refuse the row `key` (and, for one check, `item`) of `alias`,
/// from the digest and state alone: the pure part of [`resolve_draft`], the same
/// lookups and the same [`settle_target_for`] with the one file read (the
/// `active-workstream` pointer) left out. `None` when only that read or the Yes
/// recheck could still refuse. The row action model calls it from a render pass
/// (OWN-12), so the footer and the menu cannot list a key this refuses.
pub(crate) fn settle_refusal(
    ctx: &AppContext,
    alias: &str,
    key: &GroupKey,
    item: Option<&str>,
) -> Option<String> {
    let (_, state, digest) = match settle_inputs(ctx, alias) {
        Ok(inputs) => inputs,
        Err(message) => return Some(message),
    };
    settle_target_for(
        alias,
        state,
        digest,
        key,
        item,
        ctx.config.preferences.owner_fold_days(),
        false,
    )
    .err()
}

/// The draft for the row `key` of `alias` under the current registry, state,
/// digest, pointer file and session list.
pub(crate) fn resolve_draft(
    ctx: &AppContext,
    alias: &str,
    key: &GroupKey,
    item: Option<&str>,
) -> Result<SettleDraft, String> {
    let (path, state, digest) = settle_inputs(ctx, alias)?;
    let target = settle_target_for(
        alias,
        state,
        digest,
        key,
        item,
        ctx.config.preferences.owner_fold_days(),
        active_workstream_is_set(path),
    )?;
    // Exact path equality (T-37-06): a sibling directory that merely shares the
    // project path as a string prefix is a different project.
    let live_session = ctx
        .active_sessions
        .iter()
        .find(|session| session.working_dir == path)
        .cloned();
    Ok(SettleDraft {
        target,
        cwd: path.to_path_buf(),
        live_session,
    })
}

/// The backlog phases (`999.N`) of the project, or of workstream `workstream`:
/// `(dir_name, number, slug)` from one `read_dir` of its `phases/` directory
/// through the Backlog tab's own rule. **Called only from key handlers** (the
/// chooser's `m` and the Yes recheck), never from a render path (OWN-12).
pub(crate) fn backlog_for(cwd: &Path, workstream: Option<&str>) -> Vec<(String, String, String)> {
    let planning = cwd.join(".planning");
    match workstream {
        Some(name) => {
            crate::state_reader::backlog::backlog_dirs(&planning.join("workstreams").join(name))
        }
        None => crate::state_reader::backlog::backlog_dirs(&planning),
    }
}

/// The phases a Move can go to (D-06) and the default pick: the scope's roadmap
/// phases that are neither completed nor the `source` phase, in roadmap order, then
/// its backlog phases by number. The pick is the first roadmap candidate after the
/// source phase in roadmap order, else the first. Every number passes
/// `is_command_token`; names go through [`clean`].
pub(crate) fn move_candidates(
    state: &ProjectState,
    workstream: Option<&str>,
    source: &str,
    backlog: &[(String, String, String)],
) -> (Vec<MoveTarget>, usize) {
    let scoped = scope_state(state, workstream);
    let source_at = scoped
        .phases
        .iter()
        .position(|phase| same_phase(&phase.number, source));
    let named = |name: &str| Some(clean(name, NAME_MAX_CHARS)).filter(|name| !name.is_empty());
    let mut out: Vec<MoveTarget> = Vec::new();
    let mut pick: Option<usize> = None;
    for (at, phase) in scoped.phases.iter().enumerate() {
        if phase.completed || same_phase(&phase.number, source) || !is_command_token(&phase.number)
        {
            continue;
        }
        if pick.is_none() && source_at.is_some_and(|source_at| at > source_at) {
            pick = Some(out.len());
        }
        out.push(MoveTarget {
            number: phase.number.clone(),
            name: named(&phase.name),
            known: true,
        });
    }
    let order = |number: &str| -> Vec<u64> {
        number
            .split('.')
            .map(|part| part.parse().unwrap_or(u64::MAX))
            .collect()
    };
    let mut parked: Vec<&(String, String, String)> = backlog
        .iter()
        .filter(|(_, number, _)| is_command_token(number) && !same_phase(number, source))
        .collect();
    parked.sort_by_key(|(_, number, _)| order(number));
    for (_, number, slug) in parked {
        out.push(MoveTarget {
            number: number.clone(),
            name: named(&slug.replace('-', " ")),
            known: true,
        });
    }
    (out, pick.unwrap_or(0))
}

/// The target a typed phase number names (D-06): a candidate when one has that
/// number (pad-insensitively), else an unknown phase the popup warns about. A
/// token that is not a phase number and the source phase itself are refused.
pub(crate) fn move_target(
    candidates: &[MoveTarget],
    typed: &str,
    source: &str,
) -> Result<MoveTarget, String> {
    let typed = typed.trim();
    if !is_command_token(typed) {
        return Err(format!("Not a phase number: {}", clean(typed, 16)));
    }
    if same_phase(typed, source) {
        return Err("That is the source phase; pick another".to_owned());
    }
    Ok(candidates
        .iter()
        .find(|candidate| same_phase(&candidate.number, typed))
        .cloned()
        .unwrap_or_else(|| MoveTarget {
            number: typed.to_owned(),
            name: None,
            known: false,
        }))
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
    // A known Move target must still be listed under the same name (T-39-13).
    if let Some(to) = plan.choice.move_to.as_ref().filter(|to| to.known) {
        let workstream = target.command.workstream.as_deref();
        let listed = ctx.project_states.get(&target.alias).is_some_and(|state| {
            let backlog = backlog_for(&fresh.draft.cwd, workstream);
            let (candidates, _) = move_candidates(state, workstream, target.phase(), &backlog);
            candidates.iter().any(|c| c.known && c == to)
        });
        if !listed {
            return Err(format!(
                "Phase {} changed since the popup opened; nothing launched",
                to.number
            ));
        }
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
    let phrase = match &plan.choice.move_to {
        Some(to) => format!("{} to phase {}", plan.choice.outcome.phrase(), to.number),
        None => plan.choice.outcome.phrase().to_owned(),
    };
    format!(
        "Launched {} in {}: {phrase} for {what}",
        target.command, target.alias
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

/// What the badge says the settle is (I-5): the outcome, and for one check the
/// check first (`test 2 won't do`).
fn pending_words(plan: &SettlePlan) -> String {
    let outcome = match (&plan.choice.outcome, &plan.choice.move_to) {
        (Outcome::WontDo, _) => "won't do".to_owned(),
        (Outcome::DeferPermanently, _) => "defer permanently".to_owned(),
        (Outcome::Move, Some(to)) => format!("move to phase {}", to.number),
        (Outcome::Move, None) => "move".to_owned(),
    };
    match plan.draft.target.scope {
        SettleScope::Check(_) => format!("{} {outcome}", plan.draft.target.short()),
        SettleScope::Phase => outcome,
    }
}

/// What the check line says once the checks are gone: GSD records Won't do and
/// Deferred permanently as `result: skipped`, so both read `skipped`.
fn pending_done_words(plan: &SettlePlan) -> String {
    match (&plan.choice.outcome, &plan.choice.move_to) {
        (Outcome::Move, Some(to)) => format!("moved to phase {}", to.number),
        _ => format!("{} skipped", plan.draft.target.short()),
    }
}

/// The pending entry for `plan`, built from the digest as it is before the launch
/// (quick 261118-v6l, I-12: the fingerprints are taken first so a fast write is
/// never missed). `None` without a digest or without a group behind the row.
fn pending_for(ctx: &AppContext, plan: &SettlePlan) -> Option<settle_pending::PendingSettle> {
    let target = &plan.draft.target;
    let digest = ctx.owner_digest(&target.alias)?;
    let fold_days = ctx.config.preferences.owner_fold_days();
    let groups = groups_behind(digest, &target.key, fold_days);
    let mut files: Vec<String> = Vec::new();
    let mut checks: Vec<(String, String)> = Vec::new();
    for group in &groups {
        for item in &group.items {
            let wanted = match &target.scope {
                SettleScope::Phase => true,
                SettleScope::Check(id) => item.id == *id && checks.is_empty(),
            };
            if wanted {
                checks.push((item.id.clone(), item.name.clone()));
                if !files.contains(&group.file) {
                    files.push(group.file.clone());
                }
            }
        }
        if matches!(target.scope, SettleScope::Phase) && !files.contains(&group.file) {
            files.push(group.file.clone());
        }
    }
    if checks.is_empty() {
        return None;
    }
    let fingerprints = files
        .iter()
        .map(|file| settle_pending::fingerprint(&plan.draft.cwd, file))
        .collect();
    let mut entry = settle_pending::PendingSettle {
        alias: target.alias.clone(),
        root: plan.draft.cwd.clone(),
        lane: target.key.lane,
        scope_label: target.key.scope_label.clone(),
        workstream: target.key.workstream.clone(),
        phase: target.key.phase.clone(),
        files,
        checks,
        words: pending_words(plan),
        done_words: pending_done_words(plan),
        started: std::time::Instant::now(),
        stage: settle_pending::Stage::Launched,
        fingerprints,
        inbox_rows: Vec::new(),
        block_rows: Vec::new(),
    };
    entry.snapshot_rows(
        owner_block::inbox_sources(digest, fold_days).0,
        owner_block::current_sources(digest),
    );
    Some(entry)
}

/// Whether Yes started something worth waiting on (I-10): a live session always
/// (the paste happens outside the app), a new session only when `status` is the
/// launched line or reports a GUI terminal. The multiplexer's failures
/// (`Failed to launch: ...`, `... no terminal emulator found ...`) are neither.
pub(crate) fn launch_started(plan: &SettlePlan, status: &str) -> bool {
    plan.draft.live_session.is_some()
        || status == launched_status(plan)
        || status.contains("opened in")
}

/// Yes: launch the plan (or focus the live session) and leave a pending entry on
/// the context so the settled row shows `settling…` until its checks leave
/// (quick 261118-v6l, I-10). A new session registers only when it started (the
/// status is the launched line or reports a GUI terminal); a live session always
/// registers, because the paste happens outside the app. Returns the status line
/// of [`launch_settle`] unchanged.
pub(crate) fn launch_and_track(ctx: &mut AppContext, plan: &SettlePlan) -> String {
    let entry = pending_for(ctx, plan);
    let status = launch_settle(plan);
    if let Some(entry) = entry.filter(|_| launch_started(plan, &status)) {
        ctx.settle_pending.insert(entry);
    }
    status
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

/// Helpers the settle tests share: a throwaway copy of a fixture project, a byte
/// map of a tree and a context over it.
#[cfg(test)]
pub(crate) mod test_support {
    use crate::state_reader::gsd_install::InstallRoots;
    use crate::state_reader::owner_items::read_owner_digest;
    use crate::state_reader::parse_project_state_with;
    use crate::ui::screens::tests::ctx_with_aliases;
    use crate::ui::screens::AppContext;
    use std::collections::BTreeMap;
    use std::path::{Path, PathBuf};

    fn copy_dir(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), &target).unwrap();
            }
        }
    }

    /// A throwaway copy of `tests/fixtures/owner/{name}/.planning`.
    pub(crate) fn temp_fixture(name: &str) -> (tempfile::TempDir, PathBuf) {
        let src = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/owner")
            .join(name)
            .join(".planning");
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        copy_dir(&src, &root.join(".planning"));
        (dir, root)
    }

    /// Every file and directory under `root`, relative path to bytes (directories
    /// as empty vectors).
    pub(crate) fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                let rel = path.strip_prefix(root).unwrap().to_path_buf();
                if path.is_dir() {
                    out.insert(rel, Vec::new());
                    walk(root, &path, out);
                } else {
                    out.insert(rel, std::fs::read(&path).unwrap());
                }
            }
        }
        let mut out = BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    /// A context with `alias` registered at `root`, its state and owner digest read
    /// from the copy at a fixed `now`.
    pub(crate) fn fixture_ctx(root: &Path, alias: &str) -> AppContext {
        let mut ctx = ctx_with_aliases(&[alias]);
        ctx.config.projects.get_mut(alias).unwrap().path = root.to_path_buf();
        let planning = root.join(".planning");
        let state = parse_project_state_with(&planning, &InstallRoots::default());
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-12T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let digest = read_owner_digest(&planning, &state, now, 30);
        ctx.project_states.insert(alias.to_owned(), state);
        ctx.owner_digests.insert(alias.to_owned(), digest);
        ctx
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{fixture_ctx, temp_fixture};
    use super::*;
    use crate::state_reader::owner_items::{GroupCounts, OwnerDigest};
    use crate::ui::screens::detail::shown;
    use crate::ui::screens::inbox::stop_rows;
    use crate::ui::screens::owner_block::sample_owner_digest;

    const ALIAS: &str = "alder-api";

    /// A refusal row: a label, how to break a group, the status line expected.
    type Case = (&'static str, Box<dyn Fn(&mut OwnerGroup)>, &'static str);

    /// The skeleton every template follows, so a table row names only what varies.
    fn expect(
        cmd: &str,
        lead: &str,
        each: &str,
        outcome: &str,
        clause: &str,
        pronoun: (&str, &str),
    ) -> String {
        let n = cmd.split(' ').nth(1).unwrap();
        let (up, lc) = pronoun;
        format!(
            "{cmd} {lead} Resume the UAT session of phase {n} without restarting it and record {each} as result: skipped, reason: Deferred follow-up: {outcome}.{clause} {up} must not block phase {n} or be asked again. Keep {lc} in the UAT file and do not promote {lc} to the 999.x backlog."
        )
    }

    fn target(
        ctx: &AppContext,
        key: &GroupKey,
        item: Option<&str>,
        pointer: bool,
    ) -> Result<SettleTarget, String> {
        settle_target_for(
            ALIAS,
            &ctx.project_states[ALIAS],
            ctx.owner_digest(ALIAS).unwrap(),
            key,
            item,
            90,
            pointer,
        )
    }

    fn choice(outcome: Outcome, reason: &str) -> SettleChoice {
        SettleChoice {
            outcome,
            reason: clean(reason, REASON_MAX_CHARS),
            move_to: None,
        }
    }

    fn group_key(ctx: &AppContext, lane: Lane, workstream: Option<&str>) -> GroupKey {
        let group = ctx
            .owner_digest(ALIAS)
            .unwrap()
            .groups
            .iter()
            .find(|g| g.lane == lane && g.workstream.as_deref() == workstream)
            .expect("a group");
        if lane == Lane::Blocked {
            GroupKey::aggregate(group)
        } else {
            GroupKey::of(group)
        }
    }

    #[test]
    fn settle_prompts_for_every_outcome_scope_and_workstream() {
        // Root: uat-pending, tests 2 and 3 are TEST, test 4 is BLOCKED.
        let (_d1, root) = temp_fixture("uat-pending");
        let ctx = fixture_ctx(&root, ALIAS);
        let key = group_key(&ctx, Lane::Test, None);
        let check_id = target(&ctx, &key, None, false).unwrap().checks.len();
        assert_eq!(check_id, 2);
        let item_id = ctx.owner_digest(ALIAS).unwrap().groups[0]
            .items
            .iter()
            .find(|i| i.test.as_deref() == Some("2"))
            .unwrap()
            .id
            .clone();
        let phase = |o, r| prompt(&target(&ctx, &key, None, false).unwrap(), &choice(o, r));
        let check = |o, r| {
            prompt(
                &target(&ctx, &key, Some(&item_id), false).unwrap(),
                &choice(o, r),
            )
        };
        let each = "each of them";
        assert_eq!(
            phase(Outcome::WontDo, "no device here"),
            expect(
                "/gsd-verify-work 01",
                "The user will not perform tests 2, 3 in 01-UAT.md.",
                each,
                "will not do",
                " no device here.",
                ("They", "them")
            )
        );
        assert_eq!(
            phase(Outcome::DeferPermanently, "later"),
            expect(
                "/gsd-verify-work 01",
                "The user defers tests 2, 3 in 01-UAT.md permanently.",
                each,
                "deferred permanently",
                " later.",
                ("They", "them")
            )
        );
        let named = "test 2 (\u{201c}Saving a note\u{201d}) in 01-UAT.md";
        assert_eq!(
            check(Outcome::WontDo, "no device here"),
            expect(
                "/gsd-verify-work 01",
                &format!("The user will not perform {named}."),
                "test 2",
                "will not do",
                " no device here.",
                ("It", "it")
            )
        );
        assert_eq!(
            check(Outcome::DeferPermanently, ""),
            expect(
                "/gsd-verify-work 01",
                &format!("The user defers {named} permanently."),
                "test 2",
                "deferred permanently",
                "",
                ("It", "it")
            )
        );

        // Workstream: alpha phase 03 has tests 1 and 2.
        let (_d2, root) = temp_fixture("workstreams");
        let ctx = fixture_ctx(&root, ALIAS);
        let key = group_key(&ctx, Lane::Test, Some("alpha"));
        let phase = target(&ctx, &key, None, false).unwrap();
        assert_eq!(
            prompt(&phase, &choice(Outcome::WontDo, "r")),
            expect(
                "/gsd-verify-work 03 --ws alpha",
                "The user will not perform tests 1, 2 in 03-UAT.md.",
                each,
                "will not do",
                " r.",
                ("They", "them")
            )
        );
        let id = ctx
            .owner_digest(ALIAS)
            .unwrap()
            .groups
            .iter()
            .find(|g| g.workstream.is_some())
            .unwrap()
            .items[1]
            .id
            .clone();
        let one = target(&ctx, &key, Some(&id), false).unwrap();
        assert_eq!(
            prompt(&one, &choice(Outcome::DeferPermanently, "r")),
            expect(
                "/gsd-verify-work 03 --ws alpha",
                "The user defers test 2 (\u{201c}Stream check two\u{201d}) in 03-UAT.md permanently.",
                "test 2",
                "deferred permanently",
                " r.",
                ("It", "it")
            )
        );
        // The same digest twice gives the same text (the Yes recheck compares it).
        let again = target(&ctx, &key, None, false).unwrap();
        assert_eq!(
            prompt(&phase, &choice(Outcome::WontDo, "r")),
            prompt(&again, &choice(Outcome::WontDo, "r"))
        );
        // The root group of the same project, with no workstream, has no `--ws`.
        let root_key = group_key(&ctx, Lane::Test, None);
        let root_target = target(&ctx, &root_key, None, false).unwrap();
        assert!(!prompt(&root_target, &choice(Outcome::WontDo, "r")).contains("--ws"));
    }

    #[test]
    fn blocked_rows_use_command_phase_and_test_rows_match_their_run() {
        let (_d, root) = temp_fixture("uat-pending");
        let ctx = fixture_ctx(&root, ALIAS);
        let digest = ctx.owner_digest(ALIAS).unwrap();
        let blocked = group_key(&ctx, Lane::Blocked, None);
        let t = target(&ctx, &blocked, None, false).unwrap();
        assert_eq!(t.command.to_string(), "/gsd-verify-work 01");
        assert_eq!(
            (t.checks.len(), t.checks[0].number.as_str()),
            (1, "4"),
            "{t:?}"
        );
        for fixture in ["uat-pending", "workstreams"] {
            let (_d, root) = temp_fixture(fixture);
            let ctx = fixture_ctx(&root, ALIAS);
            let digest = ctx.owner_digest(ALIAS).unwrap();
            let mut seen = 0;
            for group in digest
                .groups
                .iter()
                .filter(|g| g.lane == Lane::Test && matches!(g.scope, Scope::Current))
            {
                let t = target(&ctx, &GroupKey::of(group), None, false).unwrap();
                assert_eq!(Some(t.command), group.run.clone(), "{fixture}");
                seen += 1;
            }
            assert!(seen > 0, "{fixture} has TEST groups");
        }
        assert!(digest.groups.iter().any(|g| g.lane == Lane::Blocked));
    }

    /// A TEST UAT group of phase 24 with six checks, and a digest holding only it.
    fn base() -> OwnerGroup {
        sample_owner_digest()
            .groups
            .into_iter()
            .find(|g| g.kind == Kind::Uat && g.lane == Lane::Test && g.phase == "24")
            .unwrap()
    }

    fn lone(group: &OwnerGroup, item: Option<&str>, pointer: bool) -> Result<SettleTarget, String> {
        let digest = OwnerDigest {
            groups: vec![group.clone()],
            ..OwnerDigest::default()
        };
        settle_target_for(
            ALIAS,
            &ProjectState::default(),
            &digest,
            &GroupKey::of(group),
            item,
            90,
            pointer,
        )
    }

    #[test]
    fn settle_refusals() {
        assert!(lone(&base(), None, false).is_ok());
        let cases: Vec<Case> = vec![
            (
                "FIX",
                Box::new(|g| g.lane = Lane::Fix),
                "Cannot settle a FIX row: it needs code; v plans the fix",
            ),
            (
                "DECIDE",
                Box::new(|g| g.lane = Lane::Decide),
                "Cannot settle a DECIDE row: it is not a check",
            ),
            (
                "deferred",
                Box::new(|g| g.lane = Lane::Deferred),
                "Cannot settle a deferred row: it is already deferred",
            ),
            (
                "archived",
                Box::new(|g| {
                    g.scope = Scope::Archived {
                        label: "v1.0".to_owned(),
                        rank: 0,
                    }
                }),
                "Cannot settle old debt: an archived phase is not reachable by /gsd-verify-work",
            ),
            (
                "acknowledged",
                Box::new(|g| g.acknowledged = true),
                "Cannot settle an acknowledged file: it is already handled",
            ),
            (
                "unreadable",
                Box::new(|g| {
                    g.counts = GroupCounts {
                        unreadable: 1,
                        ..g.counts
                    }
                }),
                "Cannot settle: the source file is unreadable",
            ),
            (
                "phase token",
                Box::new(|g| g.phase = "1;rm".to_owned()),
                "Refused: 1;rm is not a safe phase token; nothing started",
            ),
            (
                "fullwidth digits",
                Box::new(|g| g.phase = "\u{ff11}\u{ff12}".to_owned()),
                "Refused: \u{ff11}\u{ff12} is not a safe phase token; nothing started",
            ),
        ];
        for (label, mutate, want) in cases {
            let mut group = base();
            mutate(&mut group);
            assert_eq!(
                lone(&group, None, false).err().as_deref(),
                Some(want),
                "{label}"
            );
        }
        for name in ["a b", "../x", "-h", "."] {
            let mut group = base();
            group.workstream = Some(name.to_owned());
            assert_eq!(
                lone(&group, None, false).err(),
                Some(format!(
                    "Refused: {name} is not a safe workstream name; nothing started"
                )),
                "{name}"
            );
        }
        // A vanished row and a vanished check.
        let group = base();
        let digest = OwnerDigest::default();
        let gone = settle_target_for(
            ALIAS,
            &ProjectState::default(),
            &digest,
            &GroupKey::of(&group),
            None,
            90,
            false,
        );
        assert_eq!(
            gone.err().as_deref(),
            Some("Row is no longer listed; nothing started")
        );
        assert_eq!(
            lone(&group, Some("no such id"), false).err().as_deref(),
            Some("Check is no longer listed; nothing started")
        );
    }

    #[test]
    fn the_pointer_refuses_only_root_groups() {
        let (_d, root) = temp_fixture("workstreams");
        let ctx = fixture_ctx(&root, ALIAS);
        let root_key = group_key(&ctx, Lane::Test, None);
        let ws_key = group_key(&ctx, Lane::Test, Some("alpha"));
        assert!(target(&ctx, &root_key, None, false).is_ok());
        assert_eq!(
            target(&ctx, &root_key, None, true).err().as_deref(),
            Some("Cannot settle phase 1: alder-api has an active workstream set; run it in the project")
        );
        assert!(target(&ctx, &ws_key, None, true).is_ok());
    }

    #[test]
    fn hostile_text_cannot_break_the_argument_parse_or_the_popup() {
        let hostile = [
            "x\u{1b}[2J\u{7}\u{202e}y\u{200b}z\u{85}",
            "won't",
            "a \"b\"",
            "c\\d",
            "x --ws evil",
            "-\u{200b}-ws evil",
            "tab\there\nnewline",
            "\u{201c}curly\u{201d} and \u{2019}",
        ];
        for ws in [None, Some("alpha")] {
            for text in hostile {
                let mut group = base();
                group.workstream = ws.map(str::to_owned);
                group.file = format!("phases/24/{text}-UAT.md");
                for item in &mut group.items {
                    item.name = text.to_owned();
                    item.test = Some(text.to_owned());
                }
                let t = lone(&group, None, false).unwrap();
                let c = lone(&group, Some(&group.items[0].id), false).unwrap();
                for target in [t, c] {
                    for outcome in [Outcome::WontDo, Outcome::DeferPermanently] {
                        let p = prompt(&target, &choice(outcome, text));
                        for bad in ['\'', '"', '\\'] {
                            assert!(!p.contains(bad), "{bad:?} in {p:?} for {text:?}");
                        }
                        let count = p.matches("--ws").count();
                        assert_eq!(count, usize::from(ws.is_some()), "{p:?} for {text:?}");
                        assert!(!p.chars().any(|c| c.is_control()), "{p:?}");
                        assert_eq!(shown(&p), p, "the popup would differ from the argv");
                        assert!(p.starts_with("/gsd-verify-work 24"));
                        if ws.is_some() {
                            assert!(p.starts_with("/gsd-verify-work 24 --ws alpha "));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn caps_count_characters_at_each_boundary() {
        let a = |n: usize| "a".repeat(n);
        assert_eq!(clean(&a(200), REASON_MAX_CHARS), a(200));
        assert_eq!(
            clean(&a(201), REASON_MAX_CHARS),
            format!("{}\u{2026}", a(200))
        );
        assert_eq!(clean(&a(80), NAME_MAX_CHARS), a(80));
        assert_eq!(clean(&a(81), NAME_MAX_CHARS), format!("{}\u{2026}", a(80)));
        // Characters, never bytes: 201 two-byte characters are cut at 200.
        let e = "\u{e9}".repeat(201);
        let cut = clean(&e, REASON_MAX_CHARS);
        assert_eq!(cut.chars().count(), 201);
        assert!(cut.starts_with(&"\u{e9}".repeat(200)) && cut.ends_with('\u{2026}'));
        assert_eq!(
            clean(&"\u{e9}".repeat(200), REASON_MAX_CHARS)
                .chars()
                .count(),
            200
        );

        // 20 numbers are listed, 21 become `all 21 open tests`.
        for (n, want) in [(20usize, false), (21, true)] {
            let mut group = base();
            group.items = (0..n)
                .map(|i| {
                    let mut item = base().items[0].clone();
                    item.id = format!("id{i}");
                    item.test = Some((i + 1).to_string());
                    item
                })
                .collect();
            let t = lone(&group, None, false).unwrap();
            let text = target_text(&t);
            if want {
                assert_eq!(text, "all 21 open tests in 24-UAT.md");
            } else {
                assert!(
                    text.starts_with("tests 1, 2, 3") && text.contains(", 20 in 24-UAT.md"),
                    "{text}"
                );
            }
        }
    }

    #[test]
    fn phase_targets_list_files_in_part_order() {
        let uat = base();
        let mut verification = base();
        verification.kind = Kind::Verification;
        verification.file = "phases/24/24-VERIFICATION.md".to_owned();
        verification.items.truncate(1);
        for order in [[0usize, 1], [1, 0]] {
            let both = [uat.clone(), verification.clone()];
            let digest = OwnerDigest {
                groups: order.iter().map(|&i| both[i].clone()).collect(),
                ..OwnerDigest::default()
            };
            let key = GroupKey::entry(&digest.groups[0]);
            let t = settle_target_for(
                ALIAS,
                &ProjectState::default(),
                &digest,
                &key,
                None,
                90,
                false,
            )
            .unwrap();
            let again = settle_target_for(
                ALIAS,
                &ProjectState::default(),
                &digest,
                &key,
                None,
                90,
                false,
            )
            .unwrap();
            assert_eq!(t, again);
            let text = target_text(&t);
            let uat_at = text.find("24-UAT.md").expect("the UAT file");
            let ver_at = text
                .find("24-VERIFICATION.md")
                .expect("the VERIFICATION file");
            assert_eq!(
                uat_at < ver_at,
                order[0] == 0,
                "files follow the row's part order: {text}"
            );
            assert!(
                text.contains("tests 1, 2, 3, 4, 5, 6 in 24-UAT.md"),
                "{text}"
            );
            assert!(text.contains("item 1 in 24-VERIFICATION.md"), "{text}");
            assert_eq!(
                text.matches(" and ").count(),
                1,
                "one target, not two: {text}"
            );
        }
    }

    #[test]
    fn one_check_reads_singular() {
        let (_d, root) = temp_fixture("uat-pending");
        let ctx = fixture_ctx(&root, ALIAS);
        let key = group_key(&ctx, Lane::Blocked, None);
        let t = target(&ctx, &key, None, false).unwrap();
        let p = prompt(&t, &choice(Outcome::WontDo, "no handset"));
        assert!(p.contains("will not perform test 4 in 01-UAT.md."), "{p}");
        assert!(p.contains("record test 4 as result: skipped"), "{p}");
        assert!(p.contains(" It must not block phase 01"), "{p}");
        assert!(
            p.contains("Keep it in the UAT file and do not promote it to"),
            "{p}"
        );
        assert!(!p.contains("They") && !p.contains("them"), "{p}");
    }

    #[test]
    fn deferred_without_a_reason_has_no_empty_clause() {
        let (_d, root) = temp_fixture("uat-pending");
        let ctx = fixture_ctx(&root, ALIAS);
        let key = group_key(&ctx, Lane::Test, None);
        let t = target(&ctx, &key, None, false).unwrap();
        let p = prompt(&t, &choice(Outcome::DeferPermanently, ""));
        assert!(
            p.contains("deferred permanently. They must not block"),
            "{p}"
        );
        assert!(!p.contains(". ."), "{p}");
        assert!(!p.contains("  "), "{p}");
        // A reason that ends in punctuation is not given a second full stop.
        let p = prompt(&t, &choice(Outcome::WontDo, "no device?"));
        assert!(p.contains("will not do. no device? They must not"), "{p}");
        // Every phase row of the fixture is a real row.
        assert!(!stop_rows(&ctx).is_empty());
    }

    // ── Move (39-02, D-01, D-06, D-09) ──────────────────────────────────────

    fn numbers(candidates: &[MoveTarget]) -> Vec<&str> {
        candidates.iter().map(|c| c.number.as_str()).collect()
    }

    fn names(candidates: &[MoveTarget]) -> Vec<&str> {
        candidates
            .iter()
            .map(|c| c.name.as_deref().unwrap_or_default())
            .collect()
    }

    #[test]
    fn move_candidates_list_open_phases_after_the_source() {
        let (_d, root) = temp_fixture("heuristic-labels");
        let ctx = fixture_ctx(&root, ALIAS);
        let state = &ctx.project_states[ALIAS];
        let (list, pick) = move_candidates(state, None, "01", &[]);
        assert_eq!(
            (numbers(&list), names(&list), pick),
            (vec!["02", "03"], vec!["Sync", "Plans"], 0)
        );
        assert!(list.iter().all(|c| c.known));
        // From 02 the first phase after it is 03; 01 is listed before it.
        let (list, pick) = move_candidates(state, None, "02", &[]);
        assert_eq!((numbers(&list), pick), (vec!["01", "03"], 1));
        // Pad-insensitive: `2` is the source phase `02`.
        let (list, _) = move_candidates(state, None, "2", &[]);
        assert_eq!(numbers(&list), ["01", "03"]);
        // One phase: nothing else to move to.
        let (_d2, root2) = temp_fixture("uat-pending");
        let ctx2 = fixture_ctx(&root2, ALIAS);
        let (none, pick) = move_candidates(&ctx2.project_states[ALIAS], None, "01", &[]);
        assert!(none.is_empty());
        assert_eq!(pick, 0);
    }

    #[test]
    fn move_candidates_skip_completed_source_and_bad_tokens() {
        let (_d, root) = temp_fixture("heuristic-labels");
        let mut ctx = fixture_ctx(&root, ALIAS);
        ctx.project_states
            .get_mut(ALIAS)
            .unwrap()
            .phases
            .iter_mut()
            .find(|p| p.number == "02")
            .unwrap()
            .completed = true;
        let state = &ctx.project_states[ALIAS];
        let (list, _) = move_candidates(state, None, "01", &[]);
        assert_eq!(
            numbers(&list),
            ["03"],
            "a completed phase is not a candidate"
        );
        // A phase whose number is not a command token is dropped, so is a bad
        // backlog number.
        let mut odd = ctx.project_states[ALIAS].clone();
        odd.phases[2].number = "3;rm".to_owned();
        let backlog = vec![("999.x-bad".to_owned(), "999.x".to_owned(), "bad".to_owned())];
        let (list, _) = move_candidates(&odd, None, "01", &backlog);
        assert!(list.is_empty(), "{list:?}");
    }

    #[test]
    fn move_candidates_append_the_backlog() {
        let (_d, root) = temp_fixture("heuristic-labels");
        let ctx = fixture_ctx(&root, ALIAS);
        let backlog = vec![
            (
                "999.2-later-idea".to_owned(),
                "999.2".to_owned(),
                "later-idea".to_owned(),
            ),
            ("999.1-x".to_owned(), "999.1".to_owned(), "x".to_owned()),
            (
                "999.10-tail".to_owned(),
                "999.10".to_owned(),
                "tail".to_owned(),
            ),
        ];
        let (list, pick) = move_candidates(&ctx.project_states[ALIAS], None, "01", &backlog);
        assert_eq!(numbers(&list), ["02", "03", "999.1", "999.2", "999.10"]);
        assert_eq!(names(&list)[2..], ["x", "later idea", "tail"]);
        assert_eq!(pick, 0);
        // backlog_for reads the real directory names through the Backlog rule.
        std::fs::create_dir_all(root.join(".planning/phases/999.3-parked-thing")).unwrap();
        std::fs::create_dir_all(root.join(".planning/phases/999.x-nope-not-a-number-ok")).unwrap();
        let listed = backlog_for(&root, None);
        assert!(listed
            .iter()
            .any(|(d, n, s)| d == "999.3-parked-thing" && n == "999.3" && s == "parked-thing"));
        assert!(backlog_for(&root, Some("alpha")).is_empty());
    }

    #[test]
    fn move_candidates_in_a_workstream_list_its_roadmap_only() {
        let (_d, root) = temp_fixture("workstreams");
        let ctx = fixture_ctx(&root, ALIAS);
        let state = &ctx.project_states[ALIAS];
        let (list, _) = move_candidates(state, Some("alpha"), "99", &[]);
        assert_eq!(numbers(&list), ["03"]);
        let (root_list, _) = move_candidates(state, None, "99", &[]);
        assert_eq!(numbers(&root_list), ["1"]);
    }

    #[test]
    fn move_target_resolves_typed_phases() {
        let (_d, root) = temp_fixture("heuristic-labels");
        let ctx = fixture_ctx(&root, ALIAS);
        let (list, _) = move_candidates(&ctx.project_states[ALIAS], None, "01", &[]);
        let known = move_target(&list, "2", "01").unwrap();
        assert_eq!(
            known,
            MoveTarget {
                number: "02".to_owned(),
                name: Some("Sync".to_owned()),
                known: true
            }
        );
        let unknown = move_target(&list, "7", "01").unwrap();
        assert_eq!(
            unknown,
            MoveTarget {
                number: "7".to_owned(),
                name: None,
                known: false
            }
        );
        assert_eq!(
            move_target(&list, "1", "01").unwrap_err(),
            "That is the source phase; pick another"
        );
        assert_eq!(
            move_target(&list, "1;rm", "01").unwrap_err(),
            "Not a phase number: 1;rm"
        );
        assert_eq!(
            move_target(&list, "\u{ff11}", "01").unwrap_err(),
            "Not a phase number: \u{ff11}"
        );
        assert_eq!(
            move_target(&list, "", "01").unwrap_err(),
            "Not a phase number: "
        );
        // A decimal phase and a lettered one are tokens.
        assert!(move_target(&list, "7.1", "01").is_ok());
        assert!(move_target(&list, "3A", "01").is_ok());
        assert!(move_target(&list, "-1", "01").is_err());
    }

    fn move_choice(reason: &str, to: MoveTarget) -> SettleChoice {
        SettleChoice {
            outcome: Outcome::Move,
            reason: clean(reason, REASON_MAX_CHARS),
            move_to: Some(to),
        }
    }

    fn sync() -> MoveTarget {
        MoveTarget {
            number: "02".to_owned(),
            name: Some("Sync".to_owned()),
            known: true,
        }
    }

    #[test]
    fn move_prompts_for_phase_and_check_targets() {
        let (_d, root) = temp_fixture("heuristic-labels");
        let ctx = fixture_ctx(&root, ALIAS);
        let key = group_key(&ctx, Lane::Test, None);
        let phase = target(&ctx, &key, None, false).unwrap();
        assert_eq!(phase.checks.len(), 7);
        assert_eq!(
            prompt(&phase, &move_choice("", sync())),
            "/gsd-verify-work 01 The user moves tests 1, 2, 3, 4, 5, 6, 7 in 01-UAT.md to phase 02 \u{201c}Sync\u{201d}. Resume the UAT session of phase 01 without restarting it and record each of them as result: skipped, reason: Deferred follow-up: moved to phase 02. They must not block phase 01. Then add one todo as /gsd-capture does, titled \u{201c}Phase 02 Sync: rerun 7 checks from phase 01\u{201d}, so they come up again when phase 02 runs. Do not promote them to the 999.x backlog."
        );
        let id = ctx.owner_digest(ALIAS).unwrap().groups[0].items[3]
            .id
            .clone();
        let one = target(&ctx, &key, Some(&id), false).unwrap();
        let unknown = MoveTarget {
            number: "7".to_owned(),
            name: None,
            known: false,
        };
        assert_eq!(
            prompt(&one, &move_choice("after the redesign", unknown)),
            "/gsd-verify-work 01 The user moves test 4 (\u{201c}Looks OK on a phone\u{201d}) in 01-UAT.md to phase 7. Resume the UAT session of phase 01 without restarting it and record test 4 as result: skipped, reason: Deferred follow-up: moved to phase 7. after the redesign. It must not block phase 01. Then add one todo as /gsd-capture does, titled \u{201c}Phase 7: rerun test 4 from phase 01\u{201d}, so it comes up again when phase 7 runs. Do not promote it to the 999.x backlog."
        );
        // A backlog phase is named by its slug words, and never promoted.
        let parked = MoveTarget {
            number: "999.1".to_owned(),
            name: Some("later idea".to_owned()),
            known: true,
        };
        let text = prompt(&one, &move_choice("", parked));
        assert!(
            text.contains("to phase 999.1 \u{201c}later idea\u{201d}."),
            "{text}"
        );
        assert!(
            text.contains(
                "titled \u{201c}Phase 999.1 later idea: rerun test 4 from phase 01\u{201d}"
            ),
            "{text}"
        );
        // The app runs no todo command: the instruction only asks.
        assert!(!text.contains("gsd-tools"), "{text}");
    }

    #[test]
    fn move_prompts_carry_the_workstream_once() {
        let (_d, root) = temp_fixture("workstreams");
        let ctx = fixture_ctx(&root, ALIAS);
        let key = group_key(&ctx, Lane::Test, Some("alpha"));
        let phase = target(&ctx, &key, None, false).unwrap();
        let hostile = MoveTarget {
            number: "9".to_owned(),
            name: Some(clean("it's a \"name\" --ws beta \\ x", NAME_MAX_CHARS)),
            known: true,
        };
        for to in [sync(), hostile] {
            let text = prompt(&phase, &move_choice("r --ws beta", to));
            assert!(
                text.starts_with("/gsd-verify-work 03 --ws alpha The user moves "),
                "{text}"
            );
            assert_eq!(text.matches("--ws").count(), 1, "{text}");
            assert!(!text.contains(['\'', '"', '\\']), "{text}");
            assert_eq!(shown(&text), text);
        }
    }

    #[test]
    fn move_recheck_refuses_a_renamed_phase() {
        take_settle_launches();
        let (_d, root) = temp_fixture("heuristic-labels");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let key = group_key(&ctx, Lane::Test, None);
        let draft = resolve_draft(&ctx, ALIAS, &key, None).unwrap();
        let plan = SettlePlan {
            draft,
            choice: move_choice("", sync()),
        };
        // Unchanged: the recheck passes and the status names the phase.
        let fresh = settle_recheck(&ctx, &plan).unwrap();
        assert_eq!(
            launched_status(&fresh),
            "Launched /gsd-verify-work 01 in alder-api: moved to phase 02 for phase 01 (7 checks)"
        );
        // Renamed between the popup and Yes.
        ctx.project_states
            .get_mut(ALIAS)
            .unwrap()
            .phases
            .iter_mut()
            .find(|p| p.number == "02")
            .unwrap()
            .name = "Search".to_owned();
        assert_eq!(
            settle_recheck(&ctx, &plan).unwrap_err(),
            "Phase 02 changed since the popup opened; nothing launched"
        );
        // An unknown phase is not rechecked against the roadmap.
        let unknown = MoveTarget {
            number: "7".to_owned(),
            name: None,
            known: false,
        };
        let plan = SettlePlan {
            choice: move_choice("", unknown),
            ..plan
        };
        assert!(settle_recheck(&ctx, &plan).is_ok());
        assert!(take_settle_launches().is_empty());
    }

    /// Quick 261119-f5b: the lane and scope rule is one pure function, ordered lane,
    /// scope, acknowledged, unreadable; only the last is a state of one file.
    #[test]
    fn the_settle_group_rule_separates_kinds_from_unreadable_files() {
        use crate::ui::screens::owner_row_actions::fixture;
        let digest = fixture::digest();
        let of = |lane: Lane, pick: &dyn Fn(&OwnerGroup) -> bool| {
            let group = digest
                .groups
                .iter()
                .find(|g| g.lane == lane && pick(g))
                .expect("fixture group");
            settle_group_refusal(group)
        };
        let current = |g: &OwnerGroup| matches!(g.scope, Scope::Current) && !g.acknowledged;
        assert!(of(Lane::Test, &|g| current(g) && g.counts.unreadable == 0).is_ok());
        assert!(of(Lane::Blocked, &current).is_ok());
        let fix = of(Lane::Fix, &current).unwrap_err();
        assert!(!fix.unreadable && fix.message.starts_with("Cannot settle a FIX row"));
        let decide = of(Lane::Decide, &current).unwrap_err();
        assert!(!decide.unreadable && decide.message.starts_with("Cannot settle a DECIDE row"));
        let old = of(Lane::Test, &|g| matches!(g.scope, Scope::Archived { .. })).unwrap_err();
        assert!(!old.unreadable && old.message.starts_with("Cannot settle old debt"));
        let acked = of(Lane::Test, &|g| g.acknowledged).unwrap_err();
        assert!(!acked.unreadable && acked.message.contains("acknowledged"));
        let bad = of(Lane::Test, &|g| current(g) && g.counts.unreadable > 0).unwrap_err();
        assert!(bad.unreadable);
        assert_eq!(bad.message, UNREADABLE_REFUSAL);
    }
}
