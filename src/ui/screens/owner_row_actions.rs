//! The one action model behind the Owner rows of both screens (quick 261119-f5b).
//!
//! The Owner inbox and the Phases tab's Owner block list the same rows and take the
//! same keys (`v` run, `x` settle, `o` open, `A` acknowledge, `y` copy, `g` go to
//! the project, Space expand). Which of them works on the row under the cursor, what
//! each would run and why one cannot right now is decided HERE, once, from the row
//! alone, so the two footers say the same thing and neither lists a key that does
//! nothing.
//!
//! **Pure (36 D-02, OWN-12).** [`row_actions`] is callable from a render pass: it
//! reads `AppContext` and the row and nothing else. No file, no `PATH`, no process
//! (the refusals that need I/O, such as a missing launcher, a pointer file or an
//! unreadable source at key time, stay in the key arms: their action is listed
//! ready and the key prints the refusal, as before). Every refusal text the model
//! shows is the text the hotkey prints: they come from the same pure helpers
//! (`owner_actions::run_plan`, `open_files_refusal`, `merged_ack_refusal`,
//! `owner_settle::settle_group_refusal`, [`settle_pending_refusal`]).
//!
//! **Hidden or blocked.** An action that never applies to the row's kind is absent
//! from the result; one that applies but cannot run right now is present as
//! [`Availability::Blocked`] with the reason. The footer lists the ready ones only.
//!
//! **Screen differences are listed, not accidental.** [`INBOX_ONLY`] and
//! [`OWNER_BLOCK_ONLY`] name every action only one screen offers, each with its
//! reason; a test asserts the difference between the two screens equals them.

use super::detail::{fit_footer, fit_spans, shown, FooterToken};
use super::owner_actions;
use super::owner_block::{self, Fold, RowSrc, SubStop};
use super::owner_settle;
use super::AppContext;
use crate::state_reader::owner_items::{Kind, Lane, OwnerGroup};
use crossterm::event::KeyCode;
use ratatui::text::Line;

/// The screen a row is listed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Host {
    /// The global Owner inbox.
    Inbox,
    /// The Owner block on a project's Phases tab.
    OwnerBlock,
}

/// One action a row can offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum ActionId {
    Run,
    Settle,
    Open,
    Acknowledge,
    Copy,
    GoToProject,
    Toggle,
}

/// The actions only the inbox offers, each with its reason.
///
/// `GoToProject`: the inbox lists every registered project, so `g` opens the row's
/// project; in the Owner block you are already in it and `g` means "first row"
/// (cursor navigation, not a row action).
pub(crate) const INBOX_ONLY: &[ActionId] = &[ActionId::GoToProject];

/// The actions only the Owner block offers, each with its reason. None: every
/// action of the Owner block's rows is also offered by the inbox.
pub(crate) const OWNER_BLOCK_ONLY: &[ActionId] = &[];

/// What `x` says on a row whose settle is still running (the `settling...` badge):
/// one message for the hotkey and the model (quick 261119-f5b, I-14).
pub(crate) const SETTLE_PENDING_REFUSAL: &str = "Settle already pending for this row";

/// What `x` says on the fold line, in both screens (quick 261119-f5b, I-16).
pub(crate) const SETTLE_FOLD_LINE_REFUSAL: &str = "Select a check or a group row to settle it";

/// Whether an action can run on the row right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Availability {
    Ready,
    /// Applies to the kind but cannot run now; the text is what the hotkey prints.
    Blocked(String),
}

/// How an action reads in the footer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FooterHint {
    /// The bracketed key, `[v]`.
    pub(crate) key: String,
    /// The label after the key, `run`.
    pub(crate) short: String,
    /// The label when there is room, `run /gsd-verify-work 16`.
    pub(crate) long: Option<String>,
    /// Footer drop priority (1 is kept longest).
    pub(crate) priority: u8,
}

/// One action of the row under the cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RowAction {
    pub(crate) id: ActionId,
    /// The key the hotkey path takes.
    pub(crate) key: KeyCode,
    /// The key as the menu spells it: `v`, `A`, `Space`.
    pub(crate) key_label: &'static str,
    /// The label both screens use for this action.
    pub(crate) label: &'static str,
    /// The command or target, or, when blocked, the reason.
    pub(crate) detail: String,
    pub(crate) footer: FooterHint,
    pub(crate) availability: Availability,
}

impl RowAction {
    pub(crate) fn is_ready(&self) -> bool {
        self.availability == Availability::Ready
    }
}

/// The group a key acts on for the row `src` of `alias`, looked up by identity as
/// the key is pressed: the digest's own group, else the entry a merged row stands
/// for, else the row's own group (a BLOCKED aggregate has no single group; the run
/// path refuses it by lane).
pub(crate) fn row_group(ctx: &AppContext, alias: &str, src: &RowSrc) -> OwnerGroup {
    let fold_days = ctx.config.preferences.owner_fold_days();
    ctx.owner_digest(alias)
        .and_then(|digest| {
            owner_block::group_for(digest, &src.key).or_else(|| {
                owner_block::entry_for(digest, &src.key, fold_days).map(|entry| entry.group)
            })
        })
        .unwrap_or_else(|| src.group.clone())
}

/// Why `x` refuses the row `src` of `alias` while its settle runs: the covering
/// [`PendingSettle`](super::settle_pending::PendingSettle) is not done, i.e.
/// exactly while the row shows the `settling...` badge (I-14).
pub(crate) fn settle_pending_refusal(
    ctx: &AppContext,
    alias: &str,
    src: &RowSrc,
) -> Option<&'static str> {
    ctx.settle_pending
        .for_row(alias, src)
        .filter(|entry| !entry.is_done())
        .map(|_| SETTLE_PENDING_REFUSAL)
}

fn action(
    id: ActionId,
    key_label: &'static str,
    label: &'static str,
    footer_rest: &str,
    detail: String,
    availability: Availability,
) -> RowAction {
    let key = if id == ActionId::Toggle {
        KeyCode::Char(' ')
    } else {
        KeyCode::Char(key_label.chars().next().unwrap_or(' '))
    };
    RowAction {
        id,
        key,
        key_label,
        label,
        detail,
        footer: FooterHint {
            key: format!("[{key_label}]"),
            short: footer_rest.to_owned(),
            long: None,
            priority: if id == ActionId::Toggle { 2 } else { 1 },
        },
        availability,
    }
}

fn availability_of(refusal: Option<String>) -> Availability {
    refusal.map_or(Availability::Ready, Availability::Blocked)
}

/// The actions of the row `src` of `alias` on `host`, in the fixed order `v`, `x`,
/// `o`, `A`, `y`, `g`, Space. `sub_stop` is the check or `file PATH` line the
/// sub-cursor rests on (the row's own actions apply there too; `x` then settles
/// that one check), `expanded` whether the row shows its lines.
pub(crate) fn row_actions(
    ctx: &AppContext,
    host: Host,
    alias: &str,
    src: &RowSrc,
    sub_stop: Option<&SubStop>,
    expanded: bool,
) -> Vec<RowAction> {
    let group = row_group(ctx, alias, src);
    let blocked = group.lane == Lane::Blocked || src.group.lane == Lane::Blocked;
    let file_name = src
        .files
        .iter()
        .find(|file| !file.is_empty())
        .map_or("", |file| owner_actions::basename(file));
    let mut out: Vec<RowAction> = Vec::new();

    // v: BLOCKED work and a row with no command never carry the action; a command
    // that cannot start now (unreadable file, unsafe token) is blocked with the
    // reason `run_plan` prints.
    if !blocked && group.run.is_some() {
        match owner_actions::run_plan(ctx, alias, &group) {
            Ok(plan) => {
                let command = shown(&plan.command.to_string());
                let detail = if plan.live_session.is_some() {
                    format!("focus the live session, type {command}")
                } else {
                    command.clone()
                };
                let mut run = action(
                    ActionId::Run,
                    "v",
                    "run",
                    "run",
                    detail,
                    Availability::Ready,
                );
                run.footer.long = Some(format!("run {command}"));
                out.push(run);
            }
            Err(message) => out.push(action(
                ActionId::Run,
                "v",
                "run",
                "run",
                message.clone(),
                Availability::Blocked(message),
            )),
        }
    }

    // x: the lane and scope rule `settle_target_for` applies hides it; a pending
    // settle and an unreadable file block it.
    let fold_days = ctx.config.preferences.owner_fold_days();
    let groups = ctx
        .owner_digest(alias)
        .map(|digest| owner_settle::groups_behind(digest, &src.key, fold_days))
        .unwrap_or_default();
    let refusal = groups
        .iter()
        .find_map(|group| owner_settle::settle_group_refusal(group).err());
    if !groups.is_empty() && !refusal.is_some_and(|refusal| !refusal.unreadable) {
        let availability = match (settle_pending_refusal(ctx, alias, src), refusal) {
            (Some(message), _) => Availability::Blocked(message.to_owned()),
            (None, Some(refusal)) => Availability::Blocked(refusal.message.to_owned()),
            (None, None) => Availability::Ready,
        };
        let detail = match &availability {
            Availability::Blocked(message) => message.clone(),
            Availability::Ready if matches!(sub_stop, Some(SubStop::Item(_))) => {
                "this check, asks first".to_owned()
            }
            Availability::Ready => "whole phase, asks first".to_owned(),
        };
        out.push(action(
            ActionId::Settle,
            "x",
            "settle",
            "settle",
            detail,
            availability,
        ));
    }

    // o: one source file opens; none or several are blocked with the count text.
    let refusal = owner_actions::open_files_refusal(&src.files);
    let detail = match &refusal {
        Some(message) => message.clone(),
        None => format!("{}, read-only", shown(file_name)),
    };
    out.push(action(
        ActionId::Open,
        "o",
        "open",
        "pen",
        detail,
        availability_of(refusal),
    ));

    // A: a same-phase merged row is file-level and never picks one (blocked); the
    // kinds gsd-core has no category for, BLOCKED work and an acknowledged file
    // never carry the action.
    if src.parts.len() > 1 {
        let message = owner_actions::merged_ack_refusal(src.parts.len());
        out.push(action(
            ActionId::Acknowledge,
            "A",
            "acknowledge",
            "ck",
            message.clone(),
            Availability::Blocked(message),
        ));
    } else if !blocked
        && !group.acknowledged
        && !matches!(
            group.kind,
            Kind::Handoff | Kind::Checkpoint | Kind::Deferred
        )
    {
        out.push(action(
            ActionId::Acknowledge,
            "A",
            "acknowledge",
            "ck",
            format!("{} via gsd-core, asks first", shown(file_name)),
            Availability::Ready,
        ));
    }

    // y: every row copies its checklist.
    out.push(action(
        ActionId::Copy,
        "y",
        "copy checklist",
        "copy",
        "as gsd-meta-manager inbox prints it".to_owned(),
        Availability::Ready,
    ));

    // g: offered to the inbox only ([`INBOX_ONLY`] removes it from the Owner block).
    out.push(action(
        ActionId::GoToProject,
        "g",
        "go to project",
        "oto",
        format!("{}: Phases tab, same group", shown(alias)),
        Availability::Ready,
    ));

    // Space: every row and sub-stop expands or collapses.
    let collapse = expanded || sub_stop.is_some();
    out.push(action(
        ActionId::Toggle,
        "Space",
        if collapse { "collapse" } else { "expand" },
        if collapse { "collapse" } else { "expand" },
        "checks and source file".to_owned(),
        Availability::Ready,
    ));
    out.retain(|action| offered(host, action.id));
    out
}

/// Whether `host` offers `id`: the one place the documented screen differences
/// ([`INBOX_ONLY`], [`OWNER_BLOCK_ONLY`]) take effect.
fn offered(host: Host, id: ActionId) -> bool {
    match host {
        Host::Inbox => !OWNER_BLOCK_ONLY.contains(&id),
        Host::OwnerBlock => !INBOX_ONLY.contains(&id),
    }
}

/// The actions of the fold line when the sub-cursor rests on `fold` (`open`: the
/// fold is open): Old debt copies its checklist (the inbox's the fleet's, the Owner
/// block's this project's), Space opens or closes every fold the line can open.
pub(crate) fn fold_line_actions(_host: Host, fold: Option<Fold>, open: bool) -> Vec<RowAction> {
    let mut out = Vec::new();
    if fold == Some(Fold::OldDebt) {
        out.push(action(
            ActionId::Copy,
            "y",
            "copy checklist",
            "copy",
            "as gsd-meta-manager inbox prints it".to_owned(),
            Availability::Ready,
        ));
    }
    if matches!(fold, Some(Fold::OldDebt | Fold::Stale | Fold::Acknowledged)) {
        let word = if open { "close" } else { "open" };
        out.push(action(
            ActionId::Toggle,
            "Space",
            word,
            word,
            "the fold's rows".to_owned(),
            Availability::Ready,
        ));
    }
    out
}

/// What Enter does under the cursor, as the footer words it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Enter {
    /// A collapsed row: Enter expands it.
    Expand,
    /// An expanded row: Enter collapses it.
    Collapse,
    /// A check or `file PATH` line: Enter opens the file there.
    View,
    /// The fold line over a closed fold.
    OpenFold,
    /// The fold line over an open fold.
    CloseFold,
    /// Enter does nothing here (the Triage fold, an empty screen).
    Nothing,
}

/// Where Esc goes, in the footer's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Chrome {
    /// The inbox: `[Esc]back`.
    Back,
    /// The Owner block, phase list below: `[Esc]phases` and `[q]uit`.
    Phases,
    /// The Owner block with an empty phase list: `[Esc]tab bar` and `[q]uit`.
    TabBar,
}

/// The footer for the cursor's row: only keys that act on it, within `width`.
///
/// Tokens left to right, `[key]` BOLD: `[j/k]move` (priority 3), on the fold line
/// `[←/→]fold`, the Enter token (1), every ready action of `actions` in model order
/// (1; `[v]` carries its command until the line must shrink), Space on a check or
/// file line (2), the screen's own `[Esc]` (2) and, in the Owner block, `[q]uit`
/// (3); `[?]help` is appended after fitting and never dropped. The fit order is
/// [`fit_footer`]'s: priority 3, priority 2, short labels, priority 1.
pub(crate) fn footer_spans(
    host: Host,
    actions: &[RowAction],
    enter: Enter,
    fold_nav: bool,
    chrome: Chrome,
    width: usize,
) -> Line<'static> {
    let mut tokens = vec![FooterToken::owned(
        "[j/k]".to_owned(),
        "move  ".to_owned(),
        3,
    )];
    if fold_nav {
        tokens.push(FooterToken::owned(
            "[\u{2190}/\u{2192}]".to_owned(),
            "fold  ".to_owned(),
            1,
        ));
    }
    let word = match enter {
        Enter::Expand => Some("expand"),
        Enter::Collapse => Some("collapse"),
        Enter::View => Some("view"),
        Enter::OpenFold => Some("open"),
        Enter::CloseFold => Some("close"),
        Enter::Nothing => None,
    };
    if let Some(word) = word {
        tokens.push(FooterToken::owned(
            "[Enter]".to_owned(),
            format!("{word}  "),
            1,
        ));
    }
    for action in actions.iter().filter(|action| action.is_ready()) {
        // Space is named only where Enter does not already do its job (a check or
        // file line: Enter views there and Space collapses).
        if action.id == ActionId::Toggle && enter != Enter::View {
            continue;
        }
        let hint = &action.footer;
        let token = match &hint.long {
            Some(long) => FooterToken::owned(hint.key.clone(), format!("{long}  "), hint.priority)
                .with_short(hint.key.clone(), format!("{}  ", hint.short)),
            None => {
                FooterToken::owned(hint.key.clone(), format!("{}  ", hint.short), hint.priority)
            }
        };
        tokens.push(token);
    }
    match chrome {
        Chrome::Back => tokens.push(FooterToken::owned(
            "[Esc]".to_owned(),
            "back  ".to_owned(),
            2,
        )),
        Chrome::Phases => {
            tokens.push(FooterToken::owned(
                "[Esc]".to_owned(),
                "phases  ".to_owned(),
                2,
            ));
        }
        Chrome::TabBar => {
            tokens.push(FooterToken::owned(
                "[Esc]".to_owned(),
                "tab bar  ".to_owned(),
                2,
            ));
        }
    }
    if host == Host::OwnerBlock {
        tokens.push(FooterToken::owned("[q]".to_owned(), "uit  ".to_owned(), 3));
    }
    fit_spans(fit_footer(tokens, width), width)
}

/// One digest that holds every row kind the action table names, for the model,
/// footer and menu tests of both hosts.
#[cfg(test)]
pub(crate) mod fixture {
    use super::super::owner_block::sample_owner_digest;
    use crate::state_reader::owner_items::{
        GroupCounts, Kind, Lane, OwnerDigest, OwnerGroup, RunCommand, RunVerb, Scope,
    };

    /// The alias the module's own tests register the digest under.
    pub(crate) const ALIAS: &str = "dune-app";

    fn run(verb: RunVerb, phase: Option<&str>, workstream: Option<&str>) -> Option<RunCommand> {
        Some(RunCommand {
            verb,
            phase: phase.map(str::to_owned),
            workstream: workstream.map(str::to_owned),
        })
    }

    /// The synthetic Owner digest of the Phases-tab tests with its commands and
    /// files made faithful to what the reader produces, plus a CONTEXT-questions
    /// DECIDE row, an unreadable TEST row, a same-phase UAT + VERIFICATION pair
    /// (one merged row in the inbox) and a BLOCKED aggregate over two files.
    pub(crate) fn digest() -> OwnerDigest {
        let mut digest = sample_owner_digest();
        for group in &mut digest.groups {
            group.run = match (group.lane, group.kind, &group.scope) {
                (_, _, Scope::Archived { .. }) => None,
                (Lane::Blocked | Lane::Deferred, _, _) => None,
                (_, Kind::Debug, _) => run(RunVerb::Debug, None, None),
                (_, Kind::Handoff, _) => {
                    run(RunVerb::ResumeWork, None, group.workstream.as_deref())
                }
                (Lane::Fix, _, _) => run(RunVerb::PlanPhaseGaps, Some(&group.phase), None),
                (Lane::Test, _, _) => run(RunVerb::VerifyWork, Some(&group.phase), None),
                (Lane::Decide, _, _) => None,
            };
            group.command = group.run.as_ref().map(ToString::to_string);
            if group.kind == Kind::Debug {
                group.file = ".planning/debug/widget-tap-bug.md".to_owned();
            }
        }
        // The BLOCKED aggregate spans two files.
        for group in &mut digest.groups {
            if group.lane == Lane::Blocked && group.kind == Kind::Verification {
                group.file = format!("phases/{0}/{0}-VERIFICATION.md", group.phase);
            }
        }
        let template = digest
            .groups
            .iter()
            .find(|group| group.lane == Lane::Test && group.phase == "25")
            .cloned()
            .expect("a current TEST group");
        let make = |lane: Lane, kind: Kind, phase: &str, file: &str| OwnerGroup {
            lane,
            kind,
            phase: phase.to_owned(),
            file: file.to_owned(),
            workstream: None,
            scope: Scope::Current,
            acknowledged: false,
            counts: GroupCounts::default(),
            items: Vec::new(),
            run: None,
            command: None,
            closes_phase: false,
            bookkeeping_only: false,
            ..template.clone()
        };
        let mut context = make(
            Lane::Decide,
            Kind::ContextQuestions,
            "27",
            "phases/27/27-CONTEXT.md",
        );
        context.phase_name = "ask-first".to_owned();
        let mut unreadable = make(Lane::Test, Kind::Uat, "28", "phases/28/28-UAT.md");
        unreadable.phase_name = "garbled".to_owned();
        unreadable.run = run(RunVerb::VerifyWork, Some("28"), None);
        unreadable.command = unreadable.run.as_ref().map(ToString::to_string);
        unreadable.counts.unreadable = 1;
        let mut pair_uat = make(Lane::Test, Kind::Uat, "29", "phases/29/29-UAT.md");
        pair_uat.phase_name = "twin".to_owned();
        pair_uat.items = template.items.clone();
        pair_uat.counts.checks = template.items.len();
        pair_uat.run = run(RunVerb::VerifyWork, Some("29"), None);
        pair_uat.command = pair_uat.run.as_ref().map(ToString::to_string);
        let mut pair_ver = pair_uat.clone();
        pair_ver.kind = Kind::Verification;
        pair_ver.file = "phases/29/29-VERIFICATION.md".to_owned();
        digest
            .groups
            .extend([context, unreadable, pair_uat, pair_ver]);
        digest
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::{digest, ALIAS};
    use super::*;
    use crate::ui::screens::inbox::{self, InboxRow, LaneFilter};
    use crate::ui::screens::owner_block::{GroupKey, OwnerBlockState, Stop};
    use crate::ui::screens::settle_pending::{PendingSettle, Stage};
    use crate::ui::screens::tests::{ctx_with_aliases, owe};

    /// A context over the fixture digest with every lane listed and both folds open.
    fn ctx() -> AppContext {
        let mut ctx = ctx_with_aliases(&[ALIAS]);
        owe(&mut ctx, ALIAS, digest());
        ctx.inbox_view.filter = LaneFilter::All;
        ctx.inbox_view.open_folds.insert(Fold::OldDebt);
        ctx.inbox_view.open_folds.insert(Fold::Acknowledged);
        ctx
    }

    /// The Owner block's own rows over the same digest, both folds open.
    fn owner_rows(ctx: &AppContext) -> Vec<(GroupKey, RowSrc)> {
        let digest = ctx.owner_digest(ALIAS).expect("digest");
        let mut view = OwnerBlockState::default();
        view.open_folds.insert(Fold::OldDebt);
        view.open_folds.insert(Fold::Acknowledged);
        owner_block::skeleton(digest, &view)
            .stops
            .into_iter()
            .filter_map(|stop| match stop {
                Stop::Row(key) => owner_block::row_src_for(digest, &key).map(|src| (key, src)),
                Stop::FoldLine => None,
            })
            .collect()
    }

    fn inbox_rows(ctx: &AppContext) -> Vec<InboxRow> {
        inbox::stop_rows(ctx)
    }

    fn of_owner(
        ctx: &AppContext,
        src: &RowSrc,
        sub: Option<&SubStop>,
        expanded: bool,
    ) -> Vec<RowAction> {
        row_actions(ctx, Host::OwnerBlock, ALIAS, src, sub, expanded)
    }

    fn of_inbox(
        ctx: &AppContext,
        src: &RowSrc,
        sub: Option<&SubStop>,
        expanded: bool,
    ) -> Vec<RowAction> {
        row_actions(ctx, Host::Inbox, ALIAS, src, sub, expanded)
    }

    fn find(rows: &[(GroupKey, RowSrc)], pick: impl Fn(&RowSrc) -> bool) -> &RowSrc {
        rows.iter()
            .map(|(_, src)| src)
            .find(|src| pick(src))
            .expect("the fixture row")
    }

    fn get(actions: &[RowAction], id: ActionId) -> Option<&RowAction> {
        actions.iter().find(|action| action.id == id)
    }

    /// `Ready` with this detail, or `Blocked` with this reason.
    fn ready(actions: &[RowAction], id: ActionId, detail: &str) {
        let action = get(actions, id).unwrap_or_else(|| panic!("{id:?} is offered"));
        assert_eq!(action.availability, Availability::Ready, "{id:?}");
        assert_eq!(action.detail, detail, "{id:?}");
    }

    fn blocked(actions: &[RowAction], id: ActionId, reason: &str) {
        let action = get(actions, id).unwrap_or_else(|| panic!("{id:?} is offered"));
        assert_eq!(
            action.availability,
            Availability::Blocked(reason.to_owned()),
            "{id:?}"
        );
        assert_eq!(action.detail, reason, "{id:?}");
    }

    fn ids(actions: &[RowAction]) -> Vec<ActionId> {
        actions.iter().map(|action| action.id).collect()
    }

    /// The model, row by row, against the action table of the plan.
    #[test]
    fn the_model_matches_the_action_table_row_by_row() {
        use ActionId::{Acknowledge, Copy, Open, Run, Settle, Toggle};
        let ctx = ctx();
        let rows = owner_rows(&ctx);
        let copy = "as gsd-meta-manager inbox prints it";

        // TEST group, single file, current, readable.
        let test = find(&rows, |s| s.key.phase == "24");
        let a = of_owner(&ctx, test, None, false);
        assert_eq!(ids(&a), [Run, Settle, Open, Acknowledge, Copy, Toggle]);
        ready(&a, Run, "/gsd-verify-work 24");
        ready(&a, Settle, "whole phase, asks first");
        ready(&a, Open, "24-UAT.md, read-only");
        ready(&a, Acknowledge, "24-UAT.md via gsd-core, asks first");
        ready(&a, Copy, copy);
        ready(&a, Toggle, "checks and source file");
        assert_eq!(get(&a, Toggle).map(|t| t.label), Some("expand"));
        let inbox_a = of_inbox(&ctx, test, None, false);
        ready(
            &inbox_a,
            ActionId::GoToProject,
            "dune-app: Phases tab, same group",
        );
        // A sub-stop collapses; a check line settles that one check.
        let check = SubStop::Item("check 0".to_owned());
        let a = of_owner(&ctx, test, Some(&check), true);
        ready(&a, Settle, "this check, asks first");
        assert_eq!(get(&a, Toggle).map(|t| t.label), Some("collapse"));
        let a = of_owner(
            &ctx,
            test,
            Some(&SubStop::File(test.group.file.clone())),
            true,
        );
        ready(&a, Settle, "whole phase, asks first");

        // FIX group: command, no settle.
        let fix = find(&rows, |s| {
            s.group.lane == Lane::Fix && !s.group.acknowledged && s.key.scope_label.is_none()
        });
        let a = of_owner(&ctx, fix, None, false);
        assert_eq!(ids(&a), [Run, Open, Acknowledge, Copy, Toggle]);
        ready(&a, Run, "/gsd-plan-phase 25 --gaps");

        // DECIDE: CONTEXT questions have no command and no settle.
        let ctx_q = find(&rows, |s| s.group.kind == Kind::ContextQuestions);
        let a = of_owner(&ctx, ctx_q, None, false);
        assert_eq!(ids(&a), [Open, Acknowledge, Copy, Toggle]);

        // DECIDE: a handoff resumes, and cannot be acknowledged.
        let handoff = find(&rows, |s| s.group.kind == Kind::Handoff);
        let a = of_owner(&ctx, handoff, None, false);
        assert_eq!(ids(&a), [Run, Open, Copy, Toggle]);
        ready(&a, Run, "/gsd-resume-work --ws v14");

        // A debug investigation runs /gsd-debug and acknowledges by slug.
        let debug = find(&rows, |s| s.group.kind == Kind::Debug);
        let a = of_owner(&ctx, debug, None, false);
        assert_eq!(ids(&a), [Run, Open, Acknowledge, Copy, Toggle]);
        ready(&a, Run, "/gsd-debug");
        ready(
            &a,
            Acknowledge,
            "widget-tap-bug.md via gsd-core, asks first",
        );

        // BLOCKED aggregate over two files: settle the phase, no command, no ack,
        // and `o` is blocked with the count text.
        let blocked_row = find(&rows, |s| {
            s.group.lane == Lane::Blocked && s.key.scope_label.is_none()
        });
        let a = of_owner(&ctx, blocked_row, None, false);
        assert_eq!(ids(&a), [Settle, Open, Copy, Toggle]);
        ready(&a, Settle, "whole phase, asks first");
        blocked(
            &a,
            Open,
            "2 files behind this row: expand it and press Enter on a file line",
        );

        // An unreadable source blocks `v` and `x` with their own texts.
        let bad = find(&rows, |s| s.key.phase == "28");
        let a = of_owner(&ctx, bad, None, false);
        assert_eq!(ids(&a), [Run, Settle, Open, Acknowledge, Copy, Toggle]);
        blocked(&a, Run, "No command: the source file is unreadable");
        blocked(&a, Settle, owner_settle::UNREADABLE_REFUSAL);
        assert_eq!(get(&a, Open).map(RowAction::is_ready), Some(true));

        // Old debt rows carry no command and no settle; acknowledge targets the
        // archived milestone.
        let old = find(&rows, |s| {
            s.key.scope_label.is_some() && s.group.lane == Lane::Test
        });
        let a = of_owner(&ctx, old, None, false);
        assert_eq!(ids(&a), [Open, Acknowledge, Copy, Toggle]);

        // Acknowledged rows keep the group's command and offer neither settle nor ack.
        let acked = find(&rows, |s| {
            s.group.acknowledged && s.group.lane == Lane::Test
        });
        let a = of_owner(&ctx, acked, None, false);
        assert_eq!(ids(&a), [Run, Open, Copy, Toggle]);
    }

    /// An unsafe phase token is a pure refusal: `v` is blocked with it, never absent.
    #[test]
    fn an_unsafe_phase_token_blocks_the_run_action() {
        let mut ctx = ctx();
        let mut d = digest();
        for group in &mut d.groups {
            if group.phase == "24" {
                if let Some(run) = group.run.as_mut() {
                    run.phase = Some("24; rm -rf".to_owned());
                }
            }
        }
        owe(&mut ctx, ALIAS, d);
        let rows = owner_rows(&ctx);
        let row = find(&rows, |s| s.group.items.len() == 6 && s.key.phase == "24");
        let a = of_owner(&ctx, row, None, false);
        blocked(
            &a,
            ActionId::Run,
            "Refused: 24; rm -rf is not a safe phase token; nothing started",
        );
    }

    /// The inbox's merged same-phase pair: `o` and `A` are blocked with the count
    /// texts, `v` runs the entry's group, `x` settles the phase.
    #[test]
    fn a_merged_inbox_row_blocks_open_and_acknowledge() {
        use ActionId::{Acknowledge, Open, Run, Settle};
        let ctx = ctx();
        let merged: Vec<InboxRow> = inbox_rows(&ctx)
            .into_iter()
            .filter(|row| row.src.parts.len() > 1)
            .collect();
        assert_eq!(merged.len(), 1, "one merged row");
        let src = &merged[0].src;
        let a = of_inbox(&ctx, src, None, false);
        blocked(
            &a,
            Open,
            "2 files behind this row: expand it and press Enter on a file line",
        );
        blocked(
            &a,
            Acknowledge,
            "2 files behind this row: press g to pick a file in the Owner block, then A",
        );
        ready(&a, Run, "/gsd-verify-work 29");
        ready(&a, Settle, "whole phase, asks first");
        // The Owner block lists the two files as two rows, never the merged one.
        let rows = owner_rows(&ctx);
        assert!(rows.iter().all(|(key, _)| key.kind != "entry"));
    }

    /// Quick 261119-f5b: for the same item every action both screens offer has the
    /// same key, label, detail and availability; the actions only one screen offers
    /// are exactly the documented exceptions. Each screen resolves its rows through
    /// its own path (the inbox's stop rows, the Owner block's skeleton).
    #[test]
    fn both_screens_offer_the_same_action_under_the_same_key_and_label() {
        let ctx = ctx();
        let owner = owner_rows(&ctx);
        let inbox = inbox_rows(&ctx);
        let mut compared = 0;
        for (key, owner_src) in &owner {
            let Some(inbox_row) = inbox.iter().find(|row| row.src.key == *key) else {
                continue; // a row only one screen lists (data, not an exception)
            };
            compared += 1;
            let owner_actions = of_owner(&ctx, owner_src, None, false);
            let inbox_actions = of_inbox(&ctx, &inbox_row.src, None, false);
            for a in &owner_actions {
                let b = get(&inbox_actions, a.id)
                    .unwrap_or_else(|| panic!("{:?}: {:?} missing in the inbox", key, a.id));
                assert_eq!(a, b, "{key:?} {:?}", a.id);
            }
            let set = |actions: &[RowAction]| -> std::collections::HashSet<ActionId> {
                actions.iter().map(|a| a.id).collect()
            };
            let (o, i) = (set(&owner_actions), set(&inbox_actions));
            let only_inbox: std::collections::HashSet<ActionId> =
                i.difference(&o).copied().collect();
            let only_owner: std::collections::HashSet<ActionId> =
                o.difference(&i).copied().collect();
            assert_eq!(
                only_inbox,
                INBOX_ONLY.iter().copied().collect(),
                "{key:?}: the inbox-only actions are the documented ones"
            );
            assert_eq!(
                only_owner,
                OWNER_BLOCK_ONLY.iter().copied().collect(),
                "{key:?}: the Owner-block-only actions are the documented ones"
            );
        }
        assert!(
            compared >= 10,
            "the fixture covers every row kind: {compared}"
        );
        // The check and `file PATH` sub-stops of an expanded TEST row.
        let (key, src) = owner.iter().find(|(key, _)| key.phase == "24").expect("24");
        let inbox_src = &inbox
            .iter()
            .find(|row| row.src.key == *key)
            .expect("inbox 24")
            .src;
        for sub in [
            SubStop::Item("check 0".to_owned()),
            SubStop::File(src.group.file.clone()),
        ] {
            let a = of_owner(&ctx, src, Some(&sub), true);
            let b = of_inbox(&ctx, inbox_src, Some(&sub), true);
            for action in &a {
                assert_eq!(Some(action), get(&b, action.id), "{sub:?}");
            }
        }
        // The fold line: Old debt and Acknowledged offer the same actions.
        for fold in [Fold::OldDebt, Fold::Acknowledged, Fold::Stale] {
            for open in [false, true] {
                assert_eq!(
                    fold_line_actions(Host::OwnerBlock, Some(fold), open),
                    fold_line_actions(Host::Inbox, Some(fold), open),
                    "{fold:?}"
                );
            }
        }
        assert_eq!(INBOX_ONLY, [ActionId::GoToProject]);
        assert!(OWNER_BLOCK_ONLY.is_empty());
    }

    #[test]
    fn the_fold_line_offers_copy_on_old_debt_only() {
        let copy_ids = |fold| ids(&fold_line_actions(Host::Inbox, fold, false));
        assert_eq!(
            copy_ids(Some(Fold::OldDebt)),
            [ActionId::Copy, ActionId::Toggle]
        );
        assert_eq!(copy_ids(Some(Fold::Acknowledged)), [ActionId::Toggle]);
        assert_eq!(copy_ids(Some(Fold::Stale)), [ActionId::Toggle]);
        assert!(copy_ids(Some(Fold::Todos)).is_empty());
        assert!(copy_ids(None).is_empty());
    }

    /// While a settle covering the row runs, `x` is blocked with the shared message;
    /// once it is done (or gone) the action is ready again.
    #[test]
    fn x_is_blocked_while_the_settle_runs() {
        let mut ctx = ctx();
        let rows = owner_rows(&ctx);
        let row = find(&rows, |s| s.key.phase == "24").clone();
        assert_eq!(settle_pending_refusal(&ctx, ALIAS, &row), None);
        let mut entry = PendingSettle::for_alias_test(ALIAS);
        entry.phase = "24".to_owned();
        entry.files = row.files.clone();
        ctx.settle_pending.insert(entry);
        assert_eq!(
            settle_pending_refusal(&ctx, ALIAS, &row),
            Some(SETTLE_PENDING_REFUSAL)
        );
        let a = of_owner(&ctx, &row, None, false);
        blocked(&a, ActionId::Settle, SETTLE_PENDING_REFUSAL);
        // The other actions stay ready.
        assert!(get(&a, ActionId::Run).is_some_and(RowAction::is_ready));
        // A done settle no longer blocks.
        let mut done = PendingSettle::for_alias_test(ALIAS);
        done.phase = "24".to_owned();
        done.files = row.files.clone();
        done.stage = Stage::Done {
            since: std::time::Instant::now(),
        };
        ctx.settle_pending = Default::default();
        ctx.settle_pending.insert(done);
        assert_eq!(settle_pending_refusal(&ctx, ALIAS, &row), None);
        ready(
            &of_owner(&ctx, &row, None, false),
            ActionId::Settle,
            "whole phase, asks first",
        );
    }

    fn text(line: &Line<'static>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    /// The footer lists exactly the ready letter keys, in model order, and the Enter
    /// word of the row; a blocked key never appears.
    #[test]
    fn the_footer_names_the_ready_keys_only() {
        let ctx = ctx();
        let rows = owner_rows(&ctx);
        let bad = find(&rows, |s| s.key.phase == "28");
        let a = of_inbox(&ctx, bad, None, false);
        let footer = text(&footer_spans(
            Host::Inbox,
            &a,
            Enter::Expand,
            false,
            Chrome::Back,
            250,
        ));
        assert_eq!(
            footer,
            "  [j/k]move  [Enter]expand  [o]pen  [A]ck  [y]copy  [g]oto  [Esc]back  [?]help"
        );
        assert!(
            !footer.contains("[v]") && !footer.contains("[x]"),
            "{footer}"
        );
    }

    /// The fit order: priority 3, priority 2, short labels, then priority 1;
    /// `[?]help` is always last.
    #[test]
    fn the_footer_drops_in_the_documented_order() {
        let ctx = ctx();
        let rows = owner_rows(&ctx);
        let test = find(&rows, |s| s.key.phase == "24");
        let a = of_owner(&ctx, test, None, false);
        let at = |w| {
            text(&footer_spans(
                Host::OwnerBlock,
                &a,
                Enter::Expand,
                false,
                Chrome::Phases,
                w,
            ))
        };
        let full = "  [j/k]move  [Enter]expand  [v]run /gsd-verify-work 24  [x]settle  [o]pen  [A]ck  [y]copy  [Esc]phases  [q]uit  [?]help";
        assert_eq!(at(250), full);
        let no_q = full.replace("[q]uit  ", "");
        assert_eq!(at(no_q.chars().count()), no_q, "[q]uit goes first");
        let no_jk = no_q.replace("[j/k]move  ", "");
        assert_eq!(at(no_jk.chars().count()), no_jk, "then [j/k]");
        let no_esc = no_jk.replace("[Esc]phases  ", "");
        assert_eq!(at(no_esc.chars().count()), no_esc, "then [Esc]");
        let short = no_esc.replace(" /gsd-verify-work 24", "");
        assert_eq!(at(no_esc.chars().count() - 1), short, "then v's command");
        assert_eq!(
            at(short.chars().count() - 1),
            short.replace("[y]copy  ", ""),
            "then the rightmost priority-1 token"
        );
        // Below nine cells not even the lead and `[?]help` fit and the line is cut.
        for width in 9..250 {
            let line = at(width);
            assert!(line.ends_with("[?]help"), "{width}: {line}");
            assert!(line.chars().count() <= width, "{width}: {line}");
        }
    }

    #[test]
    fn the_enter_and_space_tokens_follow_the_cursor() {
        let ctx = ctx();
        let rows = owner_rows(&ctx);
        let test = find(&rows, |s| s.key.phase == "24");
        let check = SubStop::Item("check 0".to_owned());
        let a = of_owner(&ctx, test, Some(&check), true);
        let line = text(&footer_spans(
            Host::OwnerBlock,
            &a,
            Enter::View,
            false,
            Chrome::TabBar,
            250,
        ));
        assert!(line.contains("[Enter]view"), "{line}");
        assert!(line.contains("[Space]collapse"), "{line}");
        assert!(line.contains("[Esc]tab bar"), "{line}");
        let folds = fold_line_actions(Host::OwnerBlock, Some(Fold::OldDebt), true);
        let line = text(&footer_spans(
            Host::OwnerBlock,
            &folds,
            Enter::CloseFold,
            true,
            Chrome::Phases,
            250,
        ));
        assert_eq!(
            line,
            "  [j/k]move  [\u{2190}/\u{2192}]fold  [Enter]close  [y]copy  [Esc]phases  [q]uit  [?]help"
        );
        let line = text(&footer_spans(
            Host::Inbox,
            &[],
            Enter::Nothing,
            true,
            Chrome::Back,
            250,
        ));
        assert_eq!(
            line,
            "  [j/k]move  [\u{2190}/\u{2192}]fold  [Esc]back  [?]help"
        );
    }

    /// The model is pure: its non-test source reaches no file, process or launcher.
    #[test]
    fn the_model_source_reads_no_file_and_starts_nothing() {
        let source = include_str!("owner_row_actions.rs");
        let production = source
            .split("#[cfg(test)]\npub(crate) mod fixture")
            .next()
            .expect("production part");
        for forbidden in [
            "std::fs",
            "File::",
            "Command::new",
            "ack_request",
            "resolve_draft",
            "active_workstream_is_set",
            "resolve_launcher",
            "settle_open",
            "ack_open",
            "launch_real",
        ] {
            let hits = production
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .filter(|line| line.contains(forbidden))
                .count();
            assert_eq!(hits, 0, "`{forbidden}` in the model's production code");
        }
    }
}
