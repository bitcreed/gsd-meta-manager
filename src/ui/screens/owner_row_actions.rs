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
//! `owner_actions::ack_refusal` and `owner_settle::settle_refusal`, which run the
//! very target builders the keys run, minus their one file read,
//! [`settle_pending_refusal`]).
//!
//! **Hidden or blocked.** An action that never applies to the row's kind is absent
//! from the result; one that applies but cannot run right now is present as
//! [`Availability::Blocked`] with the reason. The footer lists the ready ones only.
//!
//! **Screen differences are listed, not accidental.** [`INBOX_ONLY`] and
//! [`OWNER_BLOCK_ONLY`] name every action only one screen offers, each with its
//! reason; a test asserts the difference between the two screens equals them.

use super::detail::{fit_cells, fit_footer, fit_spans, shown, FooterToken};
use super::owner_actions;
use super::owner_block::{self, Fold, GroupKey, RowSrc, SubStop};
use super::owner_settle;
use super::AppContext;
use crate::state_reader::owner_items::{Kind, Lane, OwnerGroup};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;

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
    // settle, an unreadable file and every other pure refusal of the target builder
    // (unsafe token, unlisted check, no state) block it with the text the key prints.
    let fold_days = ctx.config.preferences.owner_fold_days();
    let groups = ctx
        .owner_digest(alias)
        .map(|digest| owner_settle::groups_behind(digest, &src.key, fold_days))
        .unwrap_or_default();
    let refusal = groups
        .iter()
        .find_map(|group| owner_settle::settle_group_refusal(group).err());
    if !groups.is_empty() && !refusal.is_some_and(|refusal| !refusal.unreadable) {
        let item = match sub_stop {
            Some(SubStop::Item(id)) => Some(id.as_str()),
            _ => None,
        };
        let availability = match settle_pending_refusal(ctx, alias, src) {
            Some(message) => Availability::Blocked(message.to_owned()),
            None => availability_of(owner_settle::settle_refusal(ctx, alias, &src.key, item)),
        };
        let detail = match &availability {
            Availability::Blocked(message) => message.clone(),
            Availability::Ready if item.is_some() => "this check, asks first".to_owned(),
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
        let availability = availability_of(owner_actions::ack_refusal(ctx, alias, &group));
        let detail = match &availability {
            Availability::Blocked(message) => message.clone(),
            Availability::Ready => format!("{} via gsd-core, asks first", shown(file_name)),
        };
        out.push(action(
            ActionId::Acknowledge,
            "A",
            "acknowledge",
            "ck",
            detail,
            availability,
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
/// fold is open): Old debt copies its checklist, Space opens or closes every fold
/// the line can open. Both screens offer the same: the inbox/Owner-block split of
/// what `y` copies (the fleet's, this project's) lives in the copy path, not here.
pub(crate) fn fold_line_actions(fold: Option<Fold>, open: bool) -> Vec<RowAction> {
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
    /// A group or debug row: Enter opens its action menu.
    Actions,
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
/// (1; `[v]` carries its command until the line must shrink), Space on a group row,
/// check or file line (2), the screen's own `[Esc]` (2) and, in the Owner block, `[q]uit`
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
        Enter::Actions => Some("actions"),
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
        // Space is named on rows and sub-stops (Enter opens the menu or the file
        // there, so Space is the one expand/collapse key); on the fold line Enter
        // itself opens or closes, so the fold's Space entry is not repeated.
        if action.id == ActionId::Toggle && !matches!(enter, Enter::Actions | Enter::View) {
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

// ---------------------------------------------------------------------------
// The action menu (Enter on a group or debug row)
// ---------------------------------------------------------------------------

/// The open action menu of a screen: which row it belongs to and which entry is
/// selected. Its entries are NOT kept: every frame and every key recomputes them
/// from [`row_actions`], so the menu never shows a stale command or a reason that
/// no longer applies, and running an entry is the host's own hotkey path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActionMenu {
    /// The project the row belongs to.
    pub(crate) alias: String,
    /// The row's identity (never an index: a refresh may move it).
    pub(crate) key: GroupKey,
    selected: ActionId,
}

/// What the screen does with a key the open menu looked at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MenuKey {
    /// Swallowed: the menu stays as it is (or moved its selection).
    Stay,
    /// The menu closes and nothing else happens.
    Close,
    /// The menu closes and the host handles this key as if it had been typed.
    Run(KeyCode),
}

impl ActionMenu {
    /// Open the menu on the row `key` of `alias`: `v` is preselected when ready,
    /// else the first ready entry (I-6). `None` when the row has no entry at all.
    pub(crate) fn open(alias: &str, key: GroupKey, actions: &[RowAction]) -> Option<Self> {
        let selected = actions
            .iter()
            .find(|action| action.id == ActionId::Run && action.is_ready())
            .or_else(|| actions.iter().find(|action| action.is_ready()))
            .or_else(|| actions.first())?
            .id;
        Some(Self {
            alias: alias.to_owned(),
            key,
            selected,
        })
    }

    /// Keep the selection on an entry the menu shows (review WR-02): the same
    /// action while it is listed and ready, else the ready entry nearest to it in
    /// action order (the earlier one on a tie), else, with nothing ready, the same
    /// action while it is listed, else the nearest listed one. Called before every
    /// key and every frame, so the marker is always drawn and Enter runs the entry
    /// it marks, whatever a refresh did to the row since the last key.
    pub(crate) fn reconcile(&mut self, actions: &[RowAction]) {
        let ready = actions.iter().any(RowAction::is_ready);
        let pool = |action: &&RowAction| action.is_ready() || !ready;
        if actions
            .iter()
            .filter(pool)
            .any(|action| action.id == self.selected)
        {
            return;
        }
        let from = self.selected as i16;
        if let Some(near) = actions
            .iter()
            .filter(pool)
            .min_by_key(|action| (i16::abs(action.id as i16 - from), action.id as i16))
        {
            self.selected = near.id;
        }
    }

    /// The selected entry.
    pub(crate) fn selected(&self) -> ActionId {
        self.selected
    }

    /// One key while the menu is open. `j`/`k`/Down/Up walk the ready entries
    /// (clamped); Enter runs the selected one; a plain letter, or Space, that names
    /// an entry runs THAT entry, ready or blocked (a blocked one's hotkey prints its
    /// reason, I-5); Esc and `q` close; everything else is swallowed.
    pub(crate) fn key(
        &mut self,
        actions: &[RowAction],
        code: KeyCode,
        modifiers: KeyModifiers,
    ) -> MenuKey {
        let plain = owner_actions::is_plain_action_key(modifiers);
        self.reconcile(actions);
        match code {
            KeyCode::Esc => MenuKey::Close,
            KeyCode::Char('q') if plain => MenuKey::Close,
            KeyCode::Char('j') | KeyCode::Down => {
                self.step(actions, true);
                MenuKey::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.step(actions, false);
                MenuKey::Stay
            }
            KeyCode::Enter => actions
                .iter()
                .find(|action| action.id == self.selected)
                .map_or(MenuKey::Stay, |action| MenuKey::Run(action.key)),
            KeyCode::Char(_) if plain => actions
                .iter()
                .find(|action| action.key == code)
                .map_or(MenuKey::Stay, |action| MenuKey::Run(action.key)),
            _ => MenuKey::Stay,
        }
    }

    /// Move the selection over the ready entries ([`reconcile`](Self::reconcile)
    /// has already put it on one); with none ready it stays.
    fn step(&mut self, actions: &[RowAction], forward: bool) {
        let ready: Vec<ActionId> = actions
            .iter()
            .filter(|action| action.is_ready())
            .map(|action| action.id)
            .collect();
        let at = ready.iter().position(|id| *id == self.selected);
        let to = match (at, forward) {
            (Some(at), true) => (at + 1).min(ready.len().saturating_sub(1)),
            (Some(at), false) => at.saturating_sub(1),
            (None, true) => 0,
            (None, false) => ready.len().saturating_sub(1),
        };
        if let Some(id) = ready.get(to) {
            self.selected = *id;
        }
    }
}

/// Cells the key and label columns of an entry take: the marker, the key padded
/// to six and the label padded to sixteen.
const MENU_LEAD: usize = 2 + 6 + 16;

/// `text` broken at spaces into lines of at most `room` cells (a word wider than
/// that is cut by the caller's `fit_cells`).
fn wrap_words(text: &str, room: usize) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    let mut row = String::new();
    for word in text.split_whitespace() {
        let wide =
            owner_block::cells(&row) + usize::from(!row.is_empty()) + owner_block::cells(word);
        if !row.is_empty() && wide > room {
            rows.push(std::mem::take(&mut row));
        }
        if !row.is_empty() {
            row.push(' ');
        }
        row.push_str(word);
    }
    rows.push(row);
    rows
}

/// The menu's text lines inside a box `inner` cells wide: one per ready entry
/// (the detail cut to the line), several per blocked one (its reason wrapped under
/// the detail column, so the whole reason is always readable at 80 columns), and
/// the line range of the selected entry.
fn menu_lines(
    actions: &[RowAction],
    selected: ActionId,
    inner: usize,
) -> (Vec<Line<'static>>, std::ops::Range<usize>) {
    let mut out = Vec::new();
    let mut at = 0..0;
    for action in actions {
        let pick = action.id == selected;
        let first = out.len();
        let marker = if pick { "> " } else { "  " };
        let lead = format!(
            "{marker}{:<6}{:<16}",
            fit_cells(action.key_label, 6),
            fit_cells(action.label, 16)
        );
        let mut style = Style::default();
        if !action.is_ready() {
            style = style.add_modifier(Modifier::DIM);
        }
        if pick {
            style = style
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan)
                .remove_modifier(Modifier::DIM);
        }
        let room = inner.saturating_sub(MENU_LEAD);
        match &action.availability {
            Availability::Ready => out.push(Line::from(Span::styled(
                fit_cells(&format!("{lead}{}", action.detail), inner),
                style,
            ))),
            Availability::Blocked(reason) => {
                let rows = wrap_words(&format!("({})", shown(reason)), room.max(8));
                for (n, row) in rows.into_iter().enumerate() {
                    let head = if n == 0 {
                        lead.clone()
                    } else {
                        " ".repeat(MENU_LEAD)
                    };
                    out.push(Line::from(Span::styled(
                        fit_cells(&format!("{head}{row}"), inner),
                        style,
                    )));
                }
            }
        }
        if pick {
            at = first..out.len();
        }
    }
    (out, at)
}

/// The first line of the window of `rows` lines that keeps the whole `selected`
/// range in view (review WR-03): none when everything fits, else the least
/// scroll that shows the range's end, never past its start (an entry taller than
/// the window shows its head).
fn scroll_to(selected: &std::ops::Range<usize>, total: usize, rows: usize) -> usize {
    if total <= rows {
        return 0;
    }
    selected
        .end
        .saturating_sub(rows)
        .min(selected.start)
        .min(total - rows)
}

/// Draw the menu over `area` (the whole frame): `Clear`, a bordered box titled
/// ` Actions ` with ` [Enter]run  [Esc]close ` under it, one entry per action.
/// It sits below `anchor` (the cursor row), flipped above when it does not fit,
/// clamped to the frame and centered when no anchor was recorded; when the frame is
/// shorter than the entries they scroll so the selected one stays in view. Draw it
/// LAST.
pub(crate) fn render_menu(
    frame: &mut Frame,
    area: Rect,
    anchor: Option<Rect>,
    actions: &[RowAction],
    selected: ActionId,
) {
    if actions.is_empty() || area.width < 8 || area.height < 3 {
        return;
    }
    let max_width = usize::from(area.width.saturating_sub(2)).clamp(1, 76);
    let longest = actions
        .iter()
        .map(|action| MENU_LEAD + owner_block::cells(&shown(&action.detail)))
        .max()
        .unwrap_or(0);
    let width = (longest + 4).clamp(30, 76).min(max_width.max(8));
    let inner = width.saturating_sub(2);
    let (lines, at) = menu_lines(actions, selected, inner);
    let width = u16::try_from(width).unwrap_or(u16::MAX).min(area.width);
    let height = u16::try_from(lines.len() + 2)
        .unwrap_or(u16::MAX)
        .min(area.height);
    // A frame shorter than the menu scrolls the entries, never the box.
    let scroll = scroll_to(&at, lines.len(), usize::from(height.saturating_sub(2)));
    let (x, y) = match anchor {
        Some(row) => {
            let x = (area.x + 2)
                .max(row.x + 2)
                .min(area.right().saturating_sub(width));
            let below = row.y.saturating_add(1);
            let y = if below + height <= area.bottom() {
                below
            } else if row.y >= area.y + height {
                row.y - height
            } else {
                area.bottom().saturating_sub(height)
            };
            (x, y)
        }
        None => (
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
        ),
    };
    let rect = Rect {
        x,
        y: y.max(area.y),
        width,
        height,
    };
    frame.render_widget(Clear, rect);
    let block = Block::bordered()
        .title(" Actions ")
        .title_bottom(" [Enter]run  [Esc]close ");
    let scroll = u16::try_from(scroll).unwrap_or(u16::MAX);
    frame.render_widget(Paragraph::new(lines).block(block).scroll((scroll, 0)), rect);
}

/// Draw the open menu of a screen from `actions`, the entries of its row now
/// (`None`: the row vanished, the cursor left it or the block lost the keyboard).
/// The one place a render pass touches the menu: a menu with no row is cleared
/// from `slot` right here, so it can neither draw again when the row comes back
/// nor swallow or misdirect the next key (review WR-04), and a live one has its
/// selection reconciled first, so the marker is always on a shown entry (WR-02).
pub(crate) fn render_open_menu(
    frame: &mut Frame,
    area: Rect,
    anchor: Option<Rect>,
    slot: &std::cell::RefCell<Option<ActionMenu>>,
    actions: Option<Vec<RowAction>>,
) {
    let mut slot = slot.borrow_mut();
    let Some(menu) = slot.as_mut() else {
        return;
    };
    let Some(actions) = actions else {
        *slot = None;
        return;
    };
    menu.reconcile(&actions);
    render_menu(frame, area, anchor, &actions, menu.selected());
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

    /// Give `alias` a milestone version, as every real project state has one: `A`
    /// refuses a project with none ("no known milestone version").
    pub(crate) fn known_milestone(ctx: &mut crate::ui::screens::AppContext, alias: &str) {
        if let Some(state) = ctx.project_states.get_mut(alias) {
            state.milestone = "v1.0".to_owned();
        }
    }

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
    use crate::state_reader::owner_items::OwnerDigest;
    use crate::ui::screens::inbox::{self, InboxRow, LaneFilter};
    use crate::ui::screens::owner_block::{GroupKey, OwnerBlockState, Stop};
    use crate::ui::screens::settle_pending::{PendingSettle, Stage};
    use crate::ui::screens::tests::{ctx_with_aliases, owe};
    use ratatui::style::{Color, Modifier};

    /// A context over the fixture digest with every lane listed and both folds open.
    fn ctx() -> AppContext {
        let mut ctx = ctx_with_aliases(&[ALIAS]);
        fixture::known_milestone(&mut ctx, ALIAS);
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
        // The fold line takes no host: both screens offer the same fold actions.
        assert_eq!(INBOX_ONLY, [ActionId::GoToProject]);
        assert!(OWNER_BLOCK_ONLY.is_empty());
    }

    #[test]
    fn the_fold_line_offers_copy_on_old_debt_only() {
        let copy_ids = |fold| ids(&fold_line_actions(fold, false));
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
            Enter::Actions,
            false,
            Chrome::Back,
            250,
        ));
        assert_eq!(
            footer,
            "  [j/k]move  [Enter]actions  [o]pen  [A]ck  [y]copy  [g]oto  [Space]expand  [Esc]back  [?]help"
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
                Enter::Actions,
                false,
                Chrome::Phases,
                w,
            ))
        };
        let full = "  [j/k]move  [Enter]actions  [v]run /gsd-verify-work 24  [x]settle  [o]pen  [A]ck  [y]copy  [Space]expand  [Esc]phases  [q]uit  [?]help";
        assert_eq!(at(250), full);
        let no_q = full.replace("[q]uit  ", "");
        assert_eq!(at(no_q.chars().count()), no_q, "[q]uit goes first");
        let no_jk = no_q.replace("[j/k]move  ", "");
        assert_eq!(at(no_jk.chars().count()), no_jk, "then [j/k]");
        let no_esc = no_jk.replace("[Esc]phases  ", "");
        assert_eq!(at(no_esc.chars().count()), no_esc, "then [Esc]");
        let no_space = no_esc.replace("[Space]expand  ", "");
        assert_eq!(at(no_space.chars().count()), no_space, "then [Space]");
        let short = no_space.replace(" /gsd-verify-work 24", "");
        assert_eq!(at(no_space.chars().count() - 1), short, "then v's command");
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
        // A row names Space beside Enter's menu; the fold line's Enter already
        // opens and closes, so its Space entry is not repeated.
        let a = of_owner(&ctx, test, None, false);
        let line = text(&footer_spans(
            Host::OwnerBlock,
            &a,
            Enter::Actions,
            false,
            Chrome::Phases,
            250,
        ));
        assert!(line.contains("[Enter]actions"), "{line}");
        assert!(line.contains("[Space]expand"), "{line}");
        let a = of_owner(&ctx, test, None, true);
        let line = text(&footer_spans(
            Host::OwnerBlock,
            &a,
            Enter::Actions,
            false,
            Chrome::Phases,
            250,
        ));
        assert!(line.contains("[Space]collapse"), "{line}");
        let folds = fold_line_actions(Some(Fold::OldDebt), true);
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

    // -----------------------------------------------------------------
    // The action menu (quick 261119-f5b)
    // -----------------------------------------------------------------

    fn menu_on(ctx: &AppContext, pick: impl Fn(&RowSrc) -> bool) -> (ActionMenu, Vec<RowAction>) {
        let rows = owner_rows(ctx);
        let (key, src) = rows
            .iter()
            .find(|(_, src)| pick(src))
            .expect("the fixture row");
        let actions = of_owner(ctx, src, None, false);
        let menu = ActionMenu::open(ALIAS, key.clone(), &actions).expect("a menu");
        (menu, actions)
    }

    fn press(menu: &mut ActionMenu, actions: &[RowAction], code: KeyCode) -> MenuKey {
        menu.key(actions, code, KeyModifiers::NONE)
    }

    /// `v` is preselected when ready, else the first ready entry.
    #[test]
    fn the_menu_preselects_v_else_the_first_ready_entry() {
        let ctx = ctx();
        let (menu, _) = menu_on(&ctx, |s| s.key.phase == "24");
        assert_eq!(menu.selected(), ActionId::Run);
        // No command: the first ready entry is `o`.
        let (menu, actions) = menu_on(&ctx, |s| s.group.kind == Kind::ContextQuestions);
        assert_eq!(ids(&actions)[0], ActionId::Open);
        assert_eq!(menu.selected(), ActionId::Open);
        // v blocked (unreadable): the first ready entry, `o`.
        let (menu, actions) = menu_on(&ctx, |s| s.key.phase == "28");
        assert!(!get(&actions, ActionId::Run).expect("v").is_ready());
        assert_eq!(menu.selected(), ActionId::Open);
    }

    /// j/k walk the ready entries only and clamp; Enter runs the selection.
    #[test]
    fn the_menu_walks_ready_entries_and_enter_runs_the_selection() {
        let ctx = ctx();
        // The unreadable TEST row: v and x blocked, o A y Space ready.
        let (mut menu, actions) = menu_on(&ctx, |s| s.key.phase == "28");
        assert_eq!(menu.selected(), ActionId::Open);
        assert_eq!(
            press(&mut menu, &actions, KeyCode::Char('k')),
            MenuKey::Stay
        );
        assert_eq!(menu.selected(), ActionId::Open, "clamped at the top");
        for (code, want) in [
            (KeyCode::Char('j'), ActionId::Acknowledge),
            (KeyCode::Down, ActionId::Copy),
            (KeyCode::Char('j'), ActionId::Toggle),
            (KeyCode::Char('j'), ActionId::Toggle),
            (KeyCode::Up, ActionId::Copy),
        ] {
            assert_eq!(press(&mut menu, &actions, code), MenuKey::Stay);
            assert_eq!(menu.selected(), want, "{code:?}");
        }
        assert_eq!(
            press(&mut menu, &actions, KeyCode::Enter),
            MenuKey::Run(KeyCode::Char('y'))
        );
    }

    /// A letter or Space naming an entry runs THAT entry, ready or blocked; Esc and
    /// q close; every other key is swallowed.
    #[test]
    fn the_menu_letters_run_and_everything_else_is_swallowed() {
        let ctx = ctx();
        let (mut menu, actions) = menu_on(&ctx, |s| s.key.phase == "28");
        for (code, want) in [
            (KeyCode::Char('v'), KeyCode::Char('v')),
            (KeyCode::Char('x'), KeyCode::Char('x')),
            (KeyCode::Char('o'), KeyCode::Char('o')),
            (KeyCode::Char('A'), KeyCode::Char('A')),
            (KeyCode::Char('y'), KeyCode::Char('y')),
            (KeyCode::Char(' '), KeyCode::Char(' ')),
        ] {
            assert_eq!(press(&mut menu, &actions, code), MenuKey::Run(want));
        }
        assert_eq!(press(&mut menu, &actions, KeyCode::Esc), MenuKey::Close);
        assert_eq!(
            press(&mut menu, &actions, KeyCode::Char('q')),
            MenuKey::Close
        );
        for code in [
            KeyCode::Char('1'),
            KeyCode::Char('a'),
            KeyCode::Char('?'),
            KeyCode::Char('g'),
            KeyCode::Char('h'),
            KeyCode::Char('l'),
            KeyCode::Char('D'),
            KeyCode::Char('['),
            KeyCode::Tab,
            KeyCode::Left,
            KeyCode::Right,
            KeyCode::Home,
        ] {
            assert_eq!(press(&mut menu, &actions, code), MenuKey::Stay, "{code:?}");
        }
        // A chorded letter (the Ctrl-V paste reflex) is swallowed, never run, and
        // Ctrl-q does not close.
        for code in ['v', 'x', 'o', 'y', 'q'] {
            assert_eq!(
                menu.key(&actions, KeyCode::Char(code), KeyModifiers::CONTROL),
                MenuKey::Stay,
                "Ctrl-{code}"
            );
        }
        // The Owner block has no `g` entry; the inbox does.
        let inbox = of_inbox(&ctx, &owner_rows(&ctx)[0].1.clone(), None, false);
        assert_eq!(
            menu.key(&inbox, KeyCode::Char('g'), KeyModifiers::NONE),
            MenuKey::Run(KeyCode::Char('g'))
        );
    }

    fn draw(
        actions: &[RowAction],
        selected: ActionId,
        anchor: Option<Rect>,
        w: u16,
        h: u16,
    ) -> (Vec<String>, ratatui::buffer::Buffer) {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let mut terminal = Terminal::new(TestBackend::new(w, h)).expect("terminal");
        terminal
            .draw(|frame| render_menu(frame, frame.area(), anchor, actions, selected))
            .expect("draw");
        let buffer = terminal.backend().buffer().clone();
        let rows = (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buffer[(x, y)].symbol().to_owned())
                    .collect::<String>()
            })
            .collect();
        (rows, buffer)
    }

    /// At 80x24 the TEST row's seven entries fit inside the frame unclipped, the
    /// selection carries `> `, and the box hangs below the anchor row.
    #[test]
    fn the_menu_fits_80x24_with_every_entry_and_the_selection_marker() {
        let ctx = ctx();
        let rows = owner_rows(&ctx);
        let test = find(&rows, |s| s.key.phase == "24");
        let actions = of_inbox(&ctx, test, None, false);
        assert_eq!(actions.len(), 7);
        let anchor = Rect::new(0, 5, 80, 1);
        let (lines, _) = draw(&actions, ActionId::Run, Some(anchor), 80, 24);
        let top = lines
            .iter()
            .position(|line| line.contains("Actions"))
            .expect("title");
        assert_eq!(top, 6, "below the anchor row: {lines:#?}");
        let body: Vec<&String> = lines[top + 1..top + 8].iter().collect();
        for (line, want) in body.iter().zip([
            "> v     run             /gsd-verify-work 24",
            "  x     settle          whole phase, asks first",
            "  o     open            24-UAT.md, read-only",
            "  A     acknowledge     24-UAT.md via gsd-core, asks first",
            "  y     copy checklist  as gsd-meta-manager inbox prints it",
            "  g     go to project   dune-app: Phases tab, same group",
            "  Space expand          checks and source file",
        ]) {
            assert!(line.contains(want), "{line:?} lacks {want:?}");
        }
        assert!(
            lines[top + 8].contains("[Enter]run  [Esc]close"),
            "{lines:#?}"
        );
        // Nothing above the box or beside it is touched.
        assert!(lines[..top].iter().all(|line| line.trim().is_empty()));
    }

    /// A blocked entry is DIM, shows its whole reason in text (wrapped under the
    /// detail column) and is skipped by selection; near the bottom the box flips
    /// above the row.
    #[test]
    fn a_blocked_entry_is_dim_with_its_whole_reason_and_the_menu_flips_above() {
        let ctx = ctx();
        let inbox = inbox_rows(&ctx);
        let merged = inbox
            .iter()
            .find(|row| row.src.parts.len() > 1)
            .expect("merged row");
        let actions = of_inbox(&ctx, &merged.src, None, false);
        let anchor = Rect::new(0, 21, 80, 1);
        let (lines, buffer) = draw(&actions, ActionId::Run, Some(anchor), 80, 24);
        let flat = lines.join("\n");
        let top = lines
            .iter()
            .position(|line| line.contains("Actions"))
            .expect("title");
        assert!(top + actions.len() + 2 <= 21 + 1, "flipped above: {top}");
        // The full reason survives at 80 columns, across wrapped lines.
        let squashed: String = flat
            .split_whitespace()
            .filter(|word| !word.starts_with('\u{2502}') || word.len() > 3)
            .collect::<Vec<_>>()
            .join(" ");
        for word in ["behind", "Owner", "block,", "then", "A)"] {
            assert!(squashed.contains(word), "{word} lost: {flat}");
        }
        // Its row carries DIM.
        let row = lines
            .iter()
            .position(|line| line.contains("(2 files behind"))
            .expect("blocked row");
        let at = lines[row].find('(').expect("paren");
        let x = u16::try_from(lines[row][..at].chars().count()).expect("x");
        let cell = &buffer[(x, u16::try_from(row).expect("row"))];
        assert!(cell.modifier.contains(Modifier::DIM), "{cell:?}");
        // The selected entry is BOLD Cyan and not dim.
        let row = lines
            .iter()
            .position(|line| line.contains("> v"))
            .expect("selected row");
        let at = lines[row].find("> v").expect("marker");
        let x = u16::try_from(lines[row][..at].chars().count()).expect("x");
        let cell = &buffer[(x + 2, u16::try_from(row).expect("row"))];
        assert_eq!(cell.fg, Color::Cyan);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    /// Without an anchor the box is centered; on a frame narrower than the box it
    /// shrinks to fit and still draws every key.
    #[test]
    fn the_menu_centers_without_an_anchor_and_shrinks_on_a_narrow_frame() {
        let ctx = ctx();
        let rows = owner_rows(&ctx);
        let test = find(&rows, |s| s.key.phase == "24");
        let actions = of_owner(&ctx, test, None, false);
        let (lines, _) = draw(&actions, ActionId::Run, None, 80, 24);
        let top = lines.iter().position(|l| l.contains("Actions")).expect("t");
        let height = actions.len() + 2;
        assert_eq!(top, (24 - height) / 2);
        let (lines, _) = draw(&actions, ActionId::Run, None, 40, 24);
        for line in &lines {
            assert!(line.chars().count() <= 40);
        }
        assert!(lines.iter().any(|l| l.contains("Space")));
    }

    /// A context whose digest `edit` has changed, before any row is derived.
    fn ctx_edited(edit: impl FnOnce(&mut OwnerDigest, &mut AppContext)) -> AppContext {
        let mut ctx = ctx();
        let mut d = digest();
        edit(&mut d, &mut ctx);
        owe(&mut ctx, ALIAS, d);
        ctx
    }

    /// The row of the TEST group of phase `phase` as the Owner block lists it.
    fn row_of<'a>(rows: &'a [(GroupKey, RowSrc)], phase: &str) -> &'a RowSrc {
        find(rows, |s| s.key.phase == phase && s.group.lane == Lane::Test)
    }

    /// Quick 261119-f5b review WR-01: `x` and `A` are blocked, with the text the key
    /// prints, for every pure refusal their target builders can make, so the footer
    /// never lists them as ready on such a row.
    #[test]
    fn x_and_a_are_blocked_with_the_key_text_for_every_pure_refusal() {
        // An unsafe phase token (the group's own phase).
        let ctx = ctx_edited(|d, _| {
            for group in &mut d.groups {
                if group.phase == "24" {
                    group.phase = "24; rm -rf".to_owned();
                }
            }
        });
        let rows = owner_rows(&ctx);
        let row = row_of(&rows, "24; rm -rf");
        let a = of_owner(&ctx, row, None, false);
        let refusal = "Refused: 24; rm -rf is not a safe phase token; nothing started";
        blocked(&a, ActionId::Settle, refusal);
        blocked(&a, ActionId::Acknowledge, refusal);
        assert_eq!(
            owner_settle::resolve_draft(&ctx, ALIAS, &row.key, None).err(),
            Some(refusal.to_owned()),
            "the key prints the model's text"
        );
        assert_eq!(
            owner_actions::ack_request(&ctx, ALIAS, &row.group).err(),
            Some(refusal.to_owned())
        );
        let footer = text(&footer_spans(
            Host::Inbox,
            &of_inbox(&ctx, row, None, false),
            Enter::Actions,
            false,
            Chrome::Back,
            250,
        ));
        assert!(
            !footer.contains("[x]") && !footer.contains("[A]"),
            "{footer}"
        );

        // An unsafe workstream name.
        let ctx = ctx_edited(|d, _| {
            for group in &mut d.groups {
                if group.phase == "24" {
                    group.workstream = Some("bad ws!".to_owned());
                }
            }
        });
        let rows = owner_rows(&ctx);
        let row = row_of(&rows, "24");
        let a = of_owner(&ctx, row, None, false);
        let refusal = "Refused: bad ws! is not a safe workstream name; nothing started";
        blocked(&a, ActionId::Settle, refusal);
        blocked(&a, ActionId::Acknowledge, refusal);
        assert_eq!(
            owner_settle::resolve_draft(&ctx, ALIAS, &row.key, None).err(),
            Some(refusal.to_owned())
        );
        assert_eq!(
            owner_actions::ack_request(&ctx, ALIAS, &row.group).err(),
            Some(refusal.to_owned())
        );

        // No known milestone version: only `A` needs it.
        let ctx = ctx_edited(|_, ctx| {
            if let Some(state) = ctx.project_states.get_mut(ALIAS) {
                state.milestone = String::new();
            }
        });
        let rows = owner_rows(&ctx);
        let row = row_of(&rows, "24");
        let a = of_owner(&ctx, row, None, false);
        let refusal = "Cannot acknowledge: dune-app has no known milestone version";
        blocked(&a, ActionId::Acknowledge, refusal);
        ready(&a, ActionId::Settle, "whole phase, asks first");
        assert_eq!(
            owner_actions::ack_request(&ctx, ALIAS, &row.group).err(),
            Some(refusal.to_owned())
        );

        // A check that is no longer listed refuses `x` on its own line.
        let ctx = self::ctx();
        let rows = owner_rows(&ctx);
        let row = row_of(&rows, "24");
        let gone = SubStop::Item("check 99".to_owned());
        let a = of_owner(&ctx, row, Some(&gone), true);
        blocked(
            &a,
            ActionId::Settle,
            "Check is no longer listed; nothing started",
        );
        assert_eq!(
            owner_settle::resolve_draft(&ctx, ALIAS, &row.key, Some("check 99")).err(),
            Some("Check is no longer listed; nothing started".to_owned())
        );
    }

    /// Review WR-02: a selection that vanished is reconciled to a visible entry (the
    /// same id when it is still there, else the nearest by action order), so Enter
    /// runs the entry the marker shows.
    #[test]
    fn a_vanished_selection_is_reconciled_to_a_visible_entry() {
        let ctx = ctx();
        let (mut menu, _) = menu_on(&ctx, |s| s.key.phase == "24");
        assert_eq!(menu.selected(), ActionId::Run);
        // A refresh left the row without `v` (a CONTEXT-questions row has no command).
        let (_, without_run) = menu_on(&ctx, |s| s.group.kind == Kind::ContextQuestions);
        assert!(get(&without_run, ActionId::Run).is_none());
        assert_eq!(
            press(&mut menu, &without_run, KeyCode::Enter),
            MenuKey::Run(KeyCode::Char('o')),
            "Enter runs the nearest visible entry, never nothing"
        );
        assert_eq!(menu.selected(), ActionId::Open);

        // The same id is kept while it is listed and ready.
        let (_, with_run) = menu_on(&ctx, |s| s.key.phase == "24");
        menu.reconcile(&with_run);
        assert_eq!(menu.selected(), ActionId::Open, "still listed: kept");

        // Nearest by action order, the earlier one on a tie: `g` is not offered by
        // the Owner block, `y` and Space are equally near.
        menu.selected = ActionId::GoToProject;
        menu.reconcile(&with_run);
        assert_eq!(menu.selected(), ActionId::Copy);

        // A selection that is listed but not ready moves to a ready entry.
        let (_, bad) = menu_on(&ctx, |s| s.key.phase == "28");
        menu.selected = ActionId::Run;
        menu.reconcile(&bad);
        assert_eq!(menu.selected(), ActionId::Open, "v is blocked there");
        // With nothing ready the listed entry stays.
        let all_blocked: Vec<RowAction> = bad
            .iter()
            .filter(|a| a.id == ActionId::Run)
            .cloned()
            .collect();
        menu.selected = ActionId::Run;
        menu.reconcile(&all_blocked);
        assert_eq!(menu.selected(), ActionId::Run);
    }

    /// Review WR-03: on a frame shorter than the menu the entry window scrolls so the
    /// selected entry is always drawn, and the hint under the box stays.
    #[test]
    fn a_short_frame_scrolls_the_menu_to_keep_the_selection_visible() {
        let ctx = ctx();
        let rows = owner_rows(&ctx);
        let test = find(&rows, |s| s.key.phase == "24");
        let actions = of_inbox(&ctx, test, None, false);
        assert_eq!(actions.len(), 7);
        for height in [5_u16, 7, 9] {
            for action in &actions {
                let anchor = Rect::new(0, 1, 80, 1);
                let (lines, _) = draw(&actions, action.id, Some(anchor), 80, height);
                let want = format!("> {}", action.key_label);
                assert!(
                    lines.iter().any(|line| line.contains(&want)),
                    "{want} hidden at height {height}: {lines:#?}"
                );
                assert!(
                    lines
                        .iter()
                        .any(|line| line.contains("[Enter]run  [Esc]close")),
                    "the hint stays at height {height}: {lines:#?}"
                );
            }
        }
        // A tall blocked entry (a wrapped reason) is shown whole when it is selected.
        let merged = inbox_rows(&ctx)
            .into_iter()
            .find(|row| row.src.parts.len() > 1)
            .expect("merged row");
        let actions = of_inbox(&ctx, &merged.src, None, false);
        let (lines, _) = draw(&actions, ActionId::Acknowledge, None, 80, 6);
        let flat = lines.join("\n");
        assert!(flat.contains("> A"), "{flat}");
        assert!(flat.contains("then A)"), "the whole reason: {flat}");
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
