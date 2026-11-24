//! The screens of settle (`x`, 39): the two-row chooser, the phase step of a Move,
//! the reason field and the default-No confirm popup: a summary cut from the exact
//! argv, with the whole argv one `d` away.
//!
//! **No screen here writes a file or starts a process.** Yes hands the plan to
//! `owner_settle::launch_settle` after `settle_recheck` found the row unchanged;
//! No, Esc and a clipped popup launch nothing.
//!
//! The chooser is replaced by the phase step (Move only) or the reason field, and
//! each of those by the next screen and finally the popup (`ScreenAction::Replace`,
//! never `Push`: WR-03), so Esc and No return straight to the host.

use super::ack_confirm::{wrap_at_separators, AckHost};
use super::confirm_popup::{self, ConfirmOutcome, ConfirmPopup};
use super::detail::fit_cells;
use super::detail::{fit_spans, settle_mux_argv, shown};
use super::driver_steer::tail_cells;
use super::inbox::InboxScreen;
use super::owner_actions;
use super::owner_settle::{
    self, clean, MoveTarget, Outcome, SettleChoice, SettleDraft, SettlePlan, SettleScope,
    REASON_MAX_CHARS,
};
use super::{AppContext, Screen, ScreenAction};
use crate::ui::mouse::MouseInput;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

fn bold() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

fn muted() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// The host screen the chooser, the reason field and the popup draw over.
fn render_host(host: &AckHost, frame: &mut Frame, area: Rect, ctx: &AppContext) {
    match host {
        AckHost::Inbox => InboxScreen::new().render(frame, area, ctx),
        AckHost::Detail(alias) => {
            super::detail::DetailScreen::new(alias.clone()).render_main_only(frame, area, ctx)
        }
    }
}

// ── The chooser ─────────────────────────────────────────────────────────────

/// `x`: the two-row chooser `Settle {target}: record it as skipped, so the phase can
/// close`, then `[w] won't do  [p] defer permanently  [m] move to phase M  [Esc]
/// cancel`. `w` and `p` replace it with the reason field, `m` with the phase step;
/// Esc cancels.
pub struct SettleChooserScreen {
    host: AckHost,
    draft: SettleDraft,
}

impl SettleChooserScreen {
    pub(crate) fn new(host: AckHost, draft: SettleDraft) -> Self {
        Self { host, draft }
    }

    fn choose(&self, outcome: Outcome, ctx: &mut AppContext) -> ScreenAction {
        // Replacing the chooser (not pushing over it) makes Esc and No return to
        // the host (WR-03).
        ctx.needs_redraw = true;
        ScreenAction::Replace(Box::new(SettleReasonScreen::new(
            self.host.clone(),
            self.draft.clone(),
            outcome,
        )))
    }
}

crate::ui::screens::adjudicate_screen!(
    SettleChooserScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints the inbox or the detail view behind one footer row that names the \
     target: the phase token or the check number, both read out of `.planning/` \
     and drawn through `shown`. Fixture states: the hostile identity as the phase \
     token, the check number and the alias, over the inbox and the detail view.",
);

impl Screen for SettleChooserScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Char('w') => self.choose(Outcome::WontDo, ctx),
            KeyCode::Char('p') => self.choose(Outcome::DeferPermanently, ctx),
            KeyCode::Char('m') => {
                ctx.needs_redraw = true;
                ScreenAction::Replace(Box::new(SettlePhaseScreen::open(
                    ctx,
                    self.host.clone(),
                    self.draft.clone(),
                )))
            }
            KeyCode::Esc => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).split(area);
        render_host(&self.host, frame, chunks[0], ctx);
        let key = |key: &'static str, rest: &'static str| {
            [Span::styled(key, bold()), Span::styled(rest, muted())]
        };
        let width = usize::from(chunks[1].width);
        let what = vec![
            Span::raw(format!("  Settle {}: ", shown(&self.draft.target.short()))),
            Span::raw("record it as skipped, so the phase can close"),
        ];
        let mut keys = vec![Span::raw("  ")];
        keys.extend(key("[w]", " won't do  "));
        keys.extend(key("[p]", " defer permanently  "));
        keys.extend(key("[m]", " move to phase M  "));
        keys.extend(key("[Esc]", " cancel"));
        frame.render_widget(
            Paragraph::new(vec![fit_spans(what, width), fit_spans(keys, width)]),
            chunks[1],
        );
    }

    fn name(&self) -> &str {
        "settle_chooser"
    }
}

// ── The phase step of a Move ────────────────────────────────────────────────

/// Most characters of a typed phase number (a token is digits, an optional
/// letter and `.digits` groups).
const TYPED_PHASE_MAX: usize = 16;

/// `m` in the chooser: pick the phase the checks move to (D-01, D-06). The
/// candidates are the scope's open roadmap phases after the source, then its
/// backlog phases; `j`/`k` move the pick while the entry is empty, a typed number
/// overrides it. Enter opens the optional reason field; Esc launches nothing.
pub struct SettlePhaseScreen {
    host: AckHost,
    draft: SettleDraft,
    candidates: Vec<MoveTarget>,
    pick: usize,
    typed: String,
    error: Option<String>,
}

impl SettlePhaseScreen {
    /// The step for `draft`. **Reads one directory** (the scope's backlog) and is
    /// called from the chooser's key arm only.
    pub(crate) fn open(ctx: &AppContext, host: AckHost, draft: SettleDraft) -> Self {
        let workstream = draft.target.command.workstream.as_deref();
        let (candidates, pick) = match ctx.project_states.get(&draft.target.alias) {
            Some(state) => {
                let backlog = owner_settle::backlog_for(&draft.cwd, workstream);
                owner_settle::move_candidates(state, workstream, draft.target.phase(), &backlog)
            }
            None => (Vec::new(), 0),
        };
        Self::with_candidates(host, draft, candidates, pick)
    }

    pub(crate) fn with_candidates(
        host: AckHost,
        draft: SettleDraft,
        candidates: Vec<MoveTarget>,
        pick: usize,
    ) -> Self {
        Self {
            host,
            draft,
            candidates,
            pick,
            typed: String::new(),
            error: None,
        }
    }

    fn step(&mut self, forward: bool) {
        if forward {
            self.pick = (self.pick + 1).min(self.candidates.len().saturating_sub(1));
        } else {
            self.pick = self.pick.saturating_sub(1);
        }
    }

    fn enter(&mut self, ctx: &mut AppContext) -> ScreenAction {
        ctx.needs_redraw = true;
        let target = if self.typed.is_empty() {
            match self.candidates.get(self.pick) {
                Some(candidate) => Ok(candidate.clone()),
                None => Err("Type a phase number".to_owned()),
            }
        } else {
            owner_settle::move_target(&self.candidates, &self.typed, self.draft.target.phase())
        };
        match target {
            Ok(target) => ScreenAction::Replace(Box::new(SettleReasonScreen::new_move(
                self.host.clone(),
                self.draft.clone(),
                target,
            ))),
            Err(message) => {
                self.error = Some(message);
                ScreenAction::None
            }
        }
    }
}

crate::ui::screens::adjudicate_screen!(
    SettlePhaseScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints the inbox or the detail view behind a footer that lists the candidate \
     phases (roadmap phase names and backlog slugs read out of `.planning/`), the \
     target (a phase token or a check number) and the phase number the owner typed, \
     all through `shown`. Fixture states: the hostile identity as candidate number \
     and name, as the target and as the typed entry.",
);

impl Screen for SettlePhaseScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Enter => self.enter(ctx),
            KeyCode::Esc => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            KeyCode::Down => {
                self.step(true);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Up => {
                self.step(false);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Backspace => {
                self.typed.pop();
                self.error = None;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // A phase token starts with a digit, so a letter first can only be
            // movement.
            KeyCode::Char('j') if self.typed.is_empty() => {
                self.step(true);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char('k') if self.typed.is_empty() => {
                self.step(false);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char(c)
                if (c.is_ascii_digit()
                    || (!self.typed.is_empty() && (c.is_ascii_alphabetic() || c == '.')))
                    && self.typed.chars().count() < TYPED_PHASE_MAX =>
            {
                self.typed.push(c);
                self.error = None;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let width = usize::from(area.width);
        let hint = "   [j/k] pick  [Enter] next  [Esc] cancel";
        let prompt = format!("  Move {} to phase> ", shown(&self.draft.target.short()));
        let room = width
            .saturating_sub(Span::raw(prompt.as_str()).width())
            .saturating_sub(1)
            .saturating_sub(Span::raw(hint).width());
        let input_row = Line::from(vec![
            Span::styled(prompt, bold()),
            Span::raw(tail_cells(&shown(&self.typed), room)),
            Span::raw("_"),
            Span::styled(hint, muted()),
        ]);
        let error = self.error.as_ref().map(|error| {
            Line::from(Span::styled(
                fit_cells(&format!("    {}", shown(error)), width),
                Style::default().fg(Color::Red),
            ))
        });
        // One row per candidate (or the empty-list line), the input row and an error
        // row, at most half the screen.
        let cap = usize::from((area.height / 2).max(3));
        let list_room = cap.saturating_sub(1 + usize::from(error.is_some())).max(1);
        let mut rows: Vec<Line<'static>> = Vec::new();
        if self.candidates.is_empty() {
            rows.push(Line::from(Span::styled(
                "  No other open phase: type a phase number",
                muted(),
            )));
        } else {
            let first = self
                .pick
                .saturating_sub(list_room - 1)
                .min(self.candidates.len().saturating_sub(list_room));
            for (at, candidate) in self
                .candidates
                .iter()
                .enumerate()
                .skip(first)
                .take(list_room)
            {
                let name = candidate.name.as_deref().unwrap_or_default();
                let text = fit_cells(
                    &shown(&format!("{}  {name}", candidate.number)),
                    width.saturating_sub(2),
                );
                rows.push(if at == self.pick {
                    Line::from(Span::styled(format!("> {text}"), bold().fg(Color::Cyan)))
                } else {
                    Line::raw(format!("  {text}"))
                });
            }
        }
        rows.push(input_row);
        rows.extend(error);
        let height = u16::try_from(rows.len().min(cap + 1)).unwrap_or(1);
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(height)]).split(area);
        render_host(&self.host, frame, chunks[0], ctx);
        frame.render_widget(Paragraph::new(rows), chunks[1]);
    }

    fn name(&self) -> &str {
        "settle_phase"
    }
}

// ── The reason field ────────────────────────────────────────────────────────

/// The one-line reason field `{label} {target}: reason> {buffer}_`. Won't do needs
/// a reason, Deferred permanently takes an empty one (D-02); the field stops
/// taking characters at [`REASON_MAX_CHARS`] and the reason is cleaned to one
/// line before it reaches the popup.
pub struct SettleReasonScreen {
    host: AckHost,
    draft: SettleDraft,
    outcome: Outcome,
    /// The phase a Move goes to (its reason is optional, D-02).
    move_to: Option<MoveTarget>,
    buffer: String,
    error: Option<String>,
}

impl SettleReasonScreen {
    pub(crate) fn new(host: AckHost, draft: SettleDraft, outcome: Outcome) -> Self {
        Self {
            host,
            draft,
            outcome,
            move_to: None,
            buffer: String::new(),
            error: None,
        }
    }

    /// The reason field of a Move to `target`.
    pub(crate) fn new_move(host: AckHost, draft: SettleDraft, target: MoveTarget) -> Self {
        Self {
            move_to: Some(target),
            ..Self::new(host, draft, Outcome::Move)
        }
    }

    fn at_cap(&self) -> bool {
        self.buffer.chars().count() >= REASON_MAX_CHARS
    }

    fn needs_reason(&self) -> bool {
        self.outcome == Outcome::WontDo
    }
}

crate::ui::screens::adjudicate_screen!(
    SettleReasonScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints the inbox or the detail view behind its footer rows, names the target \
     (a phase token or a check number read out of `.planning/`) and echoes the \
     reason the owner typed or pasted, all through `render_for_terminal`. Fixture \
     states: the hostile identity as the target and as the buffer.",
);

impl Screen for SettleReasonScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Enter => {
                let reason = clean(&self.buffer, REASON_MAX_CHARS);
                if reason.is_empty() && self.needs_reason() {
                    self.error = Some("Won't do needs a reason".to_owned());
                    ctx.needs_redraw = true;
                    return ScreenAction::None;
                }
                ctx.needs_redraw = true;
                ScreenAction::Replace(Box::new(SettleConfirmScreen::new(
                    self.host.clone(),
                    SettlePlan {
                        draft: self.draft.clone(),
                        choice: SettleChoice {
                            outcome: self.outcome,
                            reason,
                            move_to: self.move_to.clone(),
                        },
                    },
                )))
            }
            KeyCode::Esc => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            KeyCode::Backspace => {
                self.buffer.pop();
                self.error = None;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char(c) => {
                if !self.at_cap() {
                    self.buffer.push(c);
                    self.error = None;
                    ctx.needs_redraw = true;
                }
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let hint = if self.at_cap() {
            format!("   [Enter] next ({REASON_MAX_CHARS} max)  [Esc] cancel")
        } else {
            "   [Enter] next  [Esc] cancel".to_owned()
        };
        let optional = if self.needs_reason() {
            ""
        } else {
            " (optional)"
        };
        let prompt = match &self.move_to {
            Some(to) => format!(
                "  Move {} to phase {}: reason{optional}> ",
                shown(&self.draft.target.short()),
                shown(&to.number)
            ),
            None => format!(
                "  {} {}: reason{optional}> ",
                self.outcome.label(),
                shown(&self.draft.target.short())
            ),
        };
        let room = usize::from(area.width)
            .saturating_sub(Span::raw(prompt.as_str()).width())
            .saturating_sub(1)
            .saturating_sub(Span::raw(hint.as_str()).width());
        let echoed = tail_cells(&shown(&self.buffer), room);
        let input_row = Line::from(vec![
            Span::styled(prompt, bold()),
            Span::raw(echoed),
            Span::raw("_"),
            Span::styled(hint, muted()),
        ]);
        let mut notes: Vec<Line<'static>> = Vec::new();
        if let Some(error) = &self.error {
            notes.push(Line::from(Span::styled(
                format!("    {error}"),
                Style::default().fg(Color::Red),
            )));
        }
        let note_rows = if notes.is_empty() {
            0
        } else {
            Paragraph::new(notes.clone())
                .wrap(Wrap { trim: false })
                .line_count(area.width.max(1))
        };
        let cap = usize::from((area.height / 2).max(2));
        let height = u16::try_from((1 + note_rows).min(cap)).unwrap_or(1);
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(height)]).split(area);
        render_host(&self.host, frame, chunks[0], ctx);
        let footer = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(chunks[1]);
        frame.render_widget(Paragraph::new(input_row), footer[0]);
        frame.render_widget(Paragraph::new(notes).wrap(Wrap { trim: false }), footer[1]);
    }

    fn name(&self) -> &str {
        "settle_reason"
    }
}

// ── The confirm popup ───────────────────────────────────────────────────────

/// The popup in front of Yes: default No, Yellow accent. It opens on a summary (project,
/// target, outcome, the command that runs, the start of the instruction), every part
/// cut from the exact argv Yes executes; `d` shows the whole argv and back. Only Yes
/// launches anything, and only after `settle_recheck` found the row as it was shown. A
/// cut-off view, either one, cannot be confirmed (the `confirm_popup` guarantee).
pub struct SettleConfirmScreen {
    host: AckHost,
    plan: SettlePlan,
    popup: ConfirmPopup,
    /// Whether `d` switched the popup to the full command.
    full: bool,
}

impl SettleConfirmScreen {
    pub(crate) fn new(host: AckHost, plan: SettlePlan) -> Self {
        Self {
            host,
            plan,
            popup: ConfirmPopup::default(),
            full: false,
        }
    }

    /// The popup already switched to the full command, for the render-escape guard's
    /// fixture states.
    #[cfg(test)]
    pub(crate) fn showing_full_command(mut self) -> Self {
        self.full = true;
        self
    }

    /// Yes: the status line says why nothing launched, or what launched. With a
    /// live session in the project it is focused (never typed into) and the
    /// prompt goes to the clipboard for the owner to paste there (D-11); the
    /// OSC 52 branch closes the popup and hands over the sequence in one action.
    fn confirm(&self, ctx: &mut AppContext) -> ScreenAction {
        let fresh = match owner_settle::settle_recheck(ctx, &self.plan) {
            Ok(fresh) => fresh,
            Err(message) => {
                ctx.status_message = Some((message, std::time::Instant::now()));
                return ScreenAction::Pop;
            }
        };
        let focus = owner_settle::launch_and_track(ctx, &fresh);
        if fresh.draft.live_session.is_none() {
            ctx.status_message = Some((focus, std::time::Instant::now()));
            return ScreenAction::Pop;
        }
        let lead = live_settle_lead(&focus, &fresh.draft.target.alias);
        match owner_actions::copy_led(ctx, lead, fresh.prompt(), 1) {
            ScreenAction::WriteOsc52(sequence) => ScreenAction::PopWriteOsc52(sequence),
            _ => ScreenAction::Pop,
        }
    }
}

/// The status lead for a live-session settle. "paste the command there" only
/// holds when the focus actually switched (the `v` path's `focus_status` rule);
/// on a failed focus the reason stays and the owner is told where to paste.
fn live_settle_lead(focus: &str, alias: &str) -> String {
    if focus.starts_with(crate::multiplexer::FOCUS_OK_PREFIX) {
        format!("{focus} \u{b7} paste the command there")
    } else {
        format!("{focus} \u{b7} paste the command into the Claude session in {alias}")
    }
}

/// Width of the label column of the popup's header rows.
const LABEL_W: usize = 11;

/// The one footer both views end with, as the popup shows it for a new session.
const FOOTER_NEW: &str =
    "Yes starts a new session \u{b7} GSD records it \u{b7} this app writes nothing";

/// The footer when Claude already runs in the project.
const FOOTER_LIVE: &str =
    "Claude runs here \u{b7} Yes focuses it, copies the command \u{b7} no new session";

/// `label` (BOLD, in the label column) and `value` wrapped at separators under it.
fn field(label: &str, value: &str, inner_w: usize) -> Vec<Line<'static>> {
    wrap_at_separators(value, inner_w.saturating_sub(LABEL_W))
        .into_iter()
        .enumerate()
        .map(|(i, part)| {
            let head = if i == 0 {
                Span::styled(format!("{label:<LABEL_W$}"), bold())
            } else {
                Span::raw(" ".repeat(LABEL_W))
            };
            Line::from(vec![head, Span::raw(part)])
        })
        .collect()
}

/// `label` and `value` on ONE row: the value goes through `shown` first and is then cut
/// to the column with an ellipsis, so the cut never splits an escape.
fn one_row(label: &str, value: &str, inner_w: usize) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label:<LABEL_W$}"), bold()),
        Span::raw(fit_cells(&shown(value), inner_w.saturating_sub(LABEL_W))),
    ])
}

/// What the popup's Target row says: `phase 01: tests 2, 3 in 01-UAT.md` for a phase,
/// the check's own words for one check. Not yet through `shown`.
fn target_label(plan: &SettlePlan) -> String {
    let target = &plan.draft.target;
    let refs = owner_settle::target_text(target);
    match target.scope {
        SettleScope::Phase => format!("phase {}: {refs}", target.phase()),
        SettleScope::Check(_) => refs,
    }
}

/// What the popup's Outcome row says. Not yet through `shown`.
fn outcome_label(plan: &SettlePlan) -> String {
    match &plan.choice.move_to {
        Some(to) => {
            let name = to
                .name
                .as_deref()
                .map(|name| format!(" \u{201c}{name}\u{201d}"))
                .unwrap_or_default();
            format!("{} {}{name}", plan.choice.outcome.label(), to.number)
        }
        None => plan.choice.outcome.label().to_owned(),
    }
}

/// D-06: a phase neither the roadmap nor the backlog lists is accepted, with this
/// warning (Yellow) and a blank row after it. Empty for every other plan.
fn unknown_phase_warning(plan: &SettlePlan, inner_w: usize) -> Vec<Line<'static>> {
    let Some(to) = plan.choice.move_to.as_ref().filter(|to| !to.known) else {
        return Vec::new();
    };
    let warning = format!(
        "Phase {} is not in the roadmap or backlog: GSD may create or reject it.",
        shown(&to.number)
    );
    let mut rows: Vec<Line<'static>> = wrap_at_separators(&warning, inner_w)
        .into_iter()
        .map(|part| Line::from(Span::styled(part, Style::default().fg(Color::Yellow))))
        .collect();
    rows.push(Line::raw(""));
    rows
}

/// The one short dim row both views end with, wrapped so a narrow popup still draws
/// every word.
fn footer(plan: &SettlePlan, inner_w: usize) -> Vec<Line<'static>> {
    let text = if plan.draft.live_session.is_some() {
        FOOTER_LIVE
    } else {
        FOOTER_NEW
    };
    wrap_at_separators(text, inner_w)
        .into_iter()
        .map(|part| Line::from(Span::styled(part, muted())))
        .collect()
}

/// The facts of the argv the launcher executes that the summary draws, every one
/// sliced from `settle_mux_argv(plan)` and already through `shown` (D-02, D-03).
struct RunsSummary {
    /// The program words and the slash command, e.g. `claude /gsd-verify-work 01`.
    program: String,
    /// `env -u (N session vars)`: `env`, then N = how many `-u` elements the argv
    /// carries. `None` when the argv unsets nothing.
    scrub: Option<String>,
    /// The prompt after the slash command and its separating space.
    instruction: String,
}

/// Read the summary out of the exact argv Yes launches. `Err` carries the escaped
/// reason the argv could not be built.
fn runs_summary(plan: &SettlePlan) -> Result<RunsSummary, String> {
    let argv = settle_mux_argv(plan).map_err(|error| shown(&error.to_string()))?;
    let elements = argv.as_slice();
    let (prompt, head) = elements
        .split_last()
        .ok_or_else(|| "the command line is empty".to_owned())?;
    let program_name = head.first().map(String::as_str).unwrap_or_default();
    let unset = elements.iter().filter(|e| e.as_str() == "-u").count();
    // The elements that are neither `-u` nor the variable it names.
    let mut words: Vec<String> = Vec::new();
    let mut operand = false;
    for element in head.iter().skip(1) {
        if operand {
            operand = false;
        } else if element == "-u" {
            operand = true;
        } else {
            words.push(shown(element));
        }
    }
    let scrub = if unset == 0 {
        words.insert(0, shown(program_name));
        None
    } else {
        Some(format!("{} -u ({unset} session vars)", shown(program_name)))
    };
    // The prompt is `{command} {instruction}`; when it is not (it always is), none of it
    // is invented: the whole element is the instruction and no command is drawn.
    let command = plan.draft.target.command.to_string();
    let instruction = match prompt
        .strip_prefix(command.as_str())
        .and_then(|rest| rest.strip_prefix(' '))
    {
        Some(rest) => {
            words.push(shown(&command));
            rest
        }
        None => prompt.as_str(),
    };
    Ok(RunsSummary {
        program: words.join(" "),
        scrub,
        instruction: shown(instruction),
    })
}

/// The Runs row: the program words and the command in BOLD, then the dim `· env -u (N
/// session vars)` after them when it fits the row, else on a row of its own.
fn runs_lines(summary: &RunsSummary, inner_w: usize) -> Vec<Line<'static>> {
    let room = inner_w.saturating_sub(LABEL_W);
    let mut rows: Vec<Vec<Span<'static>>> = wrap_at_separators(&summary.program, room)
        .into_iter()
        .map(|part| vec![Span::styled(part, bold())])
        .collect();
    if let Some(scrub) = &summary.scrub {
        let tail = format!(" \u{b7} {scrub}");
        let used = rows
            .last()
            .map_or(0, |row| row.iter().map(Span::width).sum::<usize>());
        if used + Span::raw(tail.as_str()).width() <= room {
            if let Some(row) = rows.last_mut() {
                row.push(Span::styled(tail, muted()));
            }
        } else {
            rows.push(vec![Span::styled(fit_cells(scrub, room), muted())]);
        }
    }
    rows.into_iter()
        .enumerate()
        .map(|(i, mut spans)| {
            let head = if i == 0 {
                Span::styled(format!("{:<LABEL_W$}", "Runs"), bold())
            } else {
                Span::raw(" ".repeat(LABEL_W))
            };
            spans.insert(0, head);
            Line::from(spans)
        })
        .collect()
}

/// Most rows of the instruction the summary draws before it cuts it with an ellipsis.
const INSTRUCTION_ROWS: usize = 3;

/// `text` wrapped to `width` cells with the `needles` found in it in BOLD and the rest
/// dim, at most `max_rows` rows; a cut ends in a dim ellipsis. Emphasis changes no
/// character: the spans' contents, joined, are the wrapped text.
///
/// A hit is dropped when an ASCII letter or digit follows it (`phase 1` is not the head
/// of `phase 10`), and overlapping hits keep the earlier, longer one.
fn emphasized_rows(
    text: &str,
    needles: &[String],
    width: usize,
    max_rows: usize,
) -> Vec<Vec<Span<'static>>> {
    let mut hits: Vec<(usize, usize)> = Vec::new();
    for needle in needles.iter().filter(|needle| !needle.is_empty()) {
        for (start, matched) in text.match_indices(needle.as_str()) {
            let end = start + matched.len();
            let glued = text[end..]
                .chars()
                .next()
                .is_some_and(|next| next.is_ascii_alphanumeric());
            if !glued {
                hits.push((start, end));
            }
        }
    }
    hits.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut covered = 0;
    for (start, end) in hits {
        if start >= covered {
            ranges.push((start, end));
            covered = end;
        }
    }

    let mut rows: Vec<Vec<Span<'static>>> = Vec::new();
    let mut cursor = 0;
    for part in wrap_at_separators(text, width) {
        // The rows are in-order substrings of the text; if one is ever not found it is
        // drawn dim and whole, so no text is dropped or changed.
        let Some(offset) = text[cursor..].find(part.as_str()) else {
            rows.push(vec![Span::styled(part, muted())]);
            continue;
        };
        let (start, end) = (cursor + offset, cursor + offset + part.len());
        cursor = end;
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut at = start;
        for &(from, to) in ranges
            .iter()
            .filter(|(from, to)| *from < end && *to > start)
        {
            let (from, to) = (from.max(start), to.min(end));
            if from > at {
                spans.push(Span::styled(text[at..from].to_owned(), muted()));
            }
            spans.push(Span::styled(text[from..to].to_owned(), bold()));
            at = to;
        }
        if at < end {
            spans.push(Span::styled(text[at..end].to_owned(), muted()));
        }
        rows.push(spans);
    }

    if rows.len() > max_rows {
        rows.truncate(max_rows);
        if let Some(last) = rows.last_mut() {
            last.push(Span::styled(" \u{2026}", muted()));
            *last = fit_spans(std::mem::take(last), width).spans;
        }
    }
    rows
}

/// The popup's default body: a summary cut from the exact argv (D-01..D-03). BOLD
/// labels, every dynamic value through `shown` before it is cut. `d` swaps it for
/// [`settle_full_body`]. `inner_w` is the popup's inner width.
pub(crate) fn settle_body(plan: &SettlePlan, inner_w: u16) -> Vec<Line<'static>> {
    let inner_w = usize::from(inner_w);
    let mut body: Vec<Line<'static>> = Vec::new();
    body.extend(field("Project", &shown(&plan.draft.target.alias), inner_w));
    body.push(one_row("Target", &target_label(plan), inner_w));
    body.push(one_row("Outcome", &outcome_label(plan), inner_w));
    if !plan.choice.reason.is_empty() {
        body.push(one_row("Reason", &plan.choice.reason, inner_w));
    }
    body.push(Line::raw(""));
    match runs_summary(plan) {
        Ok(summary) => {
            body.extend(runs_lines(&summary, inner_w));
            let target = &plan.draft.target;
            let mut needles = vec![
                shown(&owner_settle::target_text(target)),
                shown(&format!("phase {}", target.phase())),
                "result: skipped".to_owned(),
            ];
            if !plan.choice.reason.is_empty() {
                needles.push(shown(&plan.choice.reason));
            }
            if let Some(to) = &plan.choice.move_to {
                needles.push(shown(&format!("phase {}", to.number)));
            }
            let rows = emphasized_rows(
                &summary.instruction,
                &needles,
                inner_w.saturating_sub(LABEL_W),
                INSTRUCTION_ROWS,
            );
            for mut spans in rows {
                spans.insert(0, Span::raw(" ".repeat(LABEL_W)));
                body.push(Line::from(spans));
            }
        }
        Err(message) => body.extend(field("Runs", &message, inner_w)),
    }
    body.push(Line::raw(""));
    body.extend(unknown_phase_warning(plan, inner_w));
    body.extend(footer(plan, inner_w));
    body
}

/// The popup's `d` body: project, target and outcome wrapped, then the exact argv, every
/// element through `shown`, the prompt element in double quotes, wrapped at `/` and
/// spaces, nothing elided (D-04). `inner_w` is the popup's inner width.
pub(crate) fn settle_full_body(plan: &SettlePlan, inner_w: u16) -> Vec<Line<'static>> {
    let inner_w = usize::from(inner_w);
    let mut body: Vec<Line<'static>> = Vec::new();
    body.extend(field("Project", &shown(&plan.draft.target.alias), inner_w));
    body.extend(field("Target", &shown(&target_label(plan)), inner_w));
    body.extend(field("Outcome", &shown(&outcome_label(plan)), inner_w));
    body.push(Line::raw(""));
    let argv = match settle_mux_argv(plan) {
        Ok(argv) => {
            let elements = argv.as_slice();
            let last = elements.len().saturating_sub(1);
            elements
                .iter()
                .enumerate()
                .map(|(i, element)| {
                    if i == last {
                        format!("\"{}\"", shown(element))
                    } else {
                        shown(element)
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
        Err(error) => shown(&error.to_string()),
    };
    for (i, part) in wrap_at_separators(&argv, inner_w.saturating_sub(2))
        .into_iter()
        .enumerate()
    {
        let indent = if i == 0 { "" } else { "  " };
        body.push(Line::raw(format!("{indent}{part}")));
    }
    body.push(Line::raw(""));
    body.extend(unknown_phase_warning(plan, inner_w));
    body.extend(footer(plan, inner_w));
    body
}

crate::ui::screens::adjudicate_screen!(
    SettleConfirmScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws the registered alias, the phase token, the check file and check names, \
     the workstream and the owner's reason (all read out of `.planning/` or typed) \
     in the summary (the header rows, the Runs row and the capped instruction, all \
     cut from the argv after `shown`) and, behind `d`, in the whole argv, each \
     element through `shown`, over the inbox or the detail view as its background. \
     Fixture states: the summary and the full-command view with hostile values at \
     the sizes the guard renders.",
);

impl Screen for SettleConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        ctx.needs_redraw = true;
        // `d` only changes what is drawn: it launches nothing and sets no status.
        if code == KeyCode::Char('d')
            && !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            self.full = !self.full;
            return ScreenAction::None;
        }
        match self.popup.on_key(code) {
            ConfirmOutcome::Confirm => self.confirm(ctx),
            ConfirmOutcome::Cancel => ScreenAction::Pop,
            ConfirmOutcome::Pending => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        render_host(&self.host, frame, area, ctx);
        confirm_popup::dim_background(frame, area);
        let inner_w = confirm_popup::body_room(area).0;
        let (body, hint) = if self.full {
            (settle_full_body(&self.plan, inner_w), "d summary")
        } else {
            (settle_body(&self.plan, inner_w), "d full command")
        };
        self.popup
            .record(confirm_popup::render_confirm_popup_hinted(
                frame,
                area,
                "Settle via /gsd-verify-work",
                body,
                Color::Yellow,
                self.popup.focus,
                hint,
            ));
    }

    fn handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction {
        confirm_popup::click_as_key(&self.popup, input).map_or(ScreenAction::None, |code| {
            self.handle_key(code, KeyModifiers::NONE, ctx)
        })
    }

    fn name(&self) -> &str {
        "settle_confirm"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::inbox::stop_rows;
    use crate::ui::screens::owner_settle::take_settle_launches;
    use crate::ui::screens::owner_settle::test_support::{fixture_ctx, temp_fixture, tree};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    const ALIAS: &str = "alder-api";

    /// The prompt of the Won't do tracer flow on `uat-pending` (D-08).
    const TRACER_PROMPT: &str = "/gsd-verify-work 01 The user will not perform tests 2, 3 in 01-UAT.md. Resume the UAT session of phase 01 without restarting it and record each of them as result: skipped, reason: Deferred follow-up: will not do. no device here. They must not block phase 01 or be asked again. Keep them in the UAT file and do not promote them to the 999.x backlog.";

    /// A small screen stack: `Push`, `Replace` and `Pop` as the App applies them.
    pub(crate) struct Stack(Vec<Box<dyn Screen>>, Option<String>);

    impl Stack {
        pub(crate) fn inbox() -> Self {
            Stack(vec![Box::new(InboxScreen::new())], None)
        }

        /// The detail view with the Owner block focused on the row `key`.
        pub(crate) fn detail(
            ctx: &mut AppContext,
            alias: &str,
            key: &crate::ui::screens::owner_block::GroupKey,
        ) -> Self {
            let screen = crate::ui::screens::detail::DetailScreen::opened_on_owner(
                alias.to_owned(),
                key,
                ctx,
            );
            assert!(screen.owner_focused(), "the Owner block holds the keyboard");
            let stack = Stack(vec![Box::new(screen)], None);
            stack.render(ctx, 100, 30);
            stack
        }

        pub(crate) fn key_with(
            &mut self,
            ctx: &mut AppContext,
            code: KeyCode,
            modifiers: KeyModifiers,
        ) -> bool {
            // A draw first: the detail view learns its Owner block rows from it.
            self.render(ctx, 100, 30);
            let action = self.0.last_mut().unwrap().handle_key(code, modifiers, ctx);
            match action {
                ScreenAction::Push(screen) => self.0.push(screen),
                ScreenAction::Replace(screen) => {
                    self.0.pop();
                    self.0.push(screen);
                }
                ScreenAction::Pop => {
                    self.0.pop();
                }
                // As `App` applies it: pop and hold the sequence.
                ScreenAction::PopWriteOsc52(sequence) => {
                    self.0.pop();
                    self.1 = Some(sequence);
                }
                ScreenAction::None => return false,
                _ => {}
            }
            true
        }

        pub(crate) fn key(&mut self, ctx: &mut AppContext, code: KeyCode) -> bool {
            self.key_with(ctx, code, KeyModifiers::NONE)
        }

        pub(crate) fn keys(&mut self, ctx: &mut AppContext, text: &str) {
            for ch in text.chars() {
                self.key(ctx, KeyCode::Char(ch));
            }
        }

        /// The OSC 52 sequence a `PopWriteOsc52` handed over, drained.
        pub(crate) fn take_osc52(&mut self) -> Option<String> {
            self.1.take()
        }

        pub(crate) fn depth(&self) -> usize {
            self.0.len()
        }

        pub(crate) fn name(&self) -> String {
            self.0
                .last()
                .map(|s| s.name().to_owned())
                .unwrap_or_default()
        }

        pub(crate) fn render(&self, ctx: &AppContext, width: u16, height: u16) -> String {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|f| self.0.last().unwrap().render(f, f.area(), ctx))
                .unwrap();
            dump(&terminal, width, height)
        }
    }

    fn dump(terminal: &Terminal<TestBackend>, width: u16, height: u16) -> String {
        let buffer = terminal.backend().buffer();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn status(ctx: &AppContext) -> String {
        ctx.status_message
            .as_ref()
            .map(|(m, _)| m.clone())
            .unwrap_or_default()
    }

    /// The Won't do flow up to the popup: `x`, `w`, the reason, Enter.
    fn to_popup(stack: &mut Stack, ctx: &mut AppContext) {
        stack.key(ctx, KeyCode::Char('x'));
        stack.key(ctx, KeyCode::Char('w'));
        stack.keys(ctx, "no device here");
        stack.key(ctx, KeyCode::Enter);
        assert_eq!(stack.name(), "settle_confirm");
    }

    /// The tracer plan of `uat-pending` as the popup holds it.
    fn tracer_plan(ctx: &AppContext, outcome: Outcome, reason: &str) -> SettlePlan {
        let row = stop_rows(ctx).remove(0);
        let draft = owner_settle::resolve_draft(ctx, ALIAS, &row.src.key, None).unwrap();
        SettlePlan {
            draft,
            choice: SettleChoice {
                outcome,
                reason: reason.to_owned(),
                move_to: None,
            },
        }
    }

    #[test]
    fn the_inbox_wont_do_flow_launches_the_exact_prompt_and_writes_nothing() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let before = tree(&root);
        let mut ctx = fixture_ctx(&root, ALIAS);
        let first = stop_rows(&ctx).remove(0);
        assert_eq!(
            (first.src.group.lane, first.src.group.phase.as_str()),
            (crate::state_reader::owner_items::Lane::Test, "01")
        );
        let mut stack = Stack::inbox();
        assert!(stack.key(&mut ctx, KeyCode::Char('x')));
        assert_eq!(stack.name(), "settle_chooser");
        assert!(stack.key(&mut ctx, KeyCode::Char('w')));
        assert_eq!((stack.name().as_str(), stack.depth()), ("settle_reason", 2));
        stack.keys(&mut ctx, "no device here");
        assert!(stack.key(&mut ctx, KeyCode::Enter));
        assert_eq!(
            (stack.name().as_str(), stack.depth()),
            ("settle_confirm", 2)
        );
        assert!(
            take_settle_launches().is_empty(),
            "nothing launches before Yes"
        );
        assert!(stack.key(&mut ctx, KeyCode::Char('y')));
        assert_eq!(stack.depth(), 1);
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1, "exactly one launch");
        assert_eq!(launches[0].prompt(), TRACER_PROMPT);
        assert_eq!(launches[0].draft.cwd, root);
        assert_eq!(
            status(&ctx),
            "Launched /gsd-verify-work 01 in alder-api: won't do for phase 01 (2 checks)"
        );
        let after = stop_rows(&ctx);
        assert_eq!(after[0].src.key, first.src.key, "the row is still listed");
        assert_eq!(tree(&root), before, "the copy is byte-identical");
    }

    /// The row `x` settles on `uat-pending`, and the id of its first drawn item.
    fn uat_row_and_first_item(
        ctx: &AppContext,
    ) -> (crate::ui::screens::owner_block::GroupKey, String) {
        let row = stop_rows(ctx).remove(0);
        let id = crate::ui::screens::owner_block::inbox_item_ids(&row.src).remove(0);
        (row.src.key, id)
    }

    /// 39-02 SC1 tracer: in the detail Owner block `Space`, `j`, `x`, `w`, a reason,
    /// Enter and `y` settle ONE check, and the copy of the tree stays identical.
    #[test]
    fn the_owner_block_check_flow_launches_one_check_and_writes_nothing() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let before = tree(&root);
        let mut ctx = fixture_ctx(&root, ALIAS);
        let (key, id) = uat_row_and_first_item(&ctx);
        assert_eq!(id, "01-UAT.md#2");
        let mut stack = Stack::detail(&mut ctx, ALIAS, &key);
        stack.key(&mut ctx, KeyCode::Char(' '));
        stack.key(&mut ctx, KeyCode::Char('j'));
        let view = &ctx.view_cache[ALIAS].owner_block;
        assert_eq!(view.item_cursor.as_deref(), Some("01-UAT.md#2"));
        assert!(stack.key(&mut ctx, KeyCode::Char('x')));
        assert_eq!(stack.name(), "settle_chooser");
        stack.key(&mut ctx, KeyCode::Char('w'));
        stack.keys(&mut ctx, "no device here");
        stack.key(&mut ctx, KeyCode::Enter);
        assert_eq!(stack.name(), "settle_confirm");
        assert!(take_settle_launches().is_empty(), "nothing before Yes");
        stack.key(&mut ctx, KeyCode::Char('y'));
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1);
        assert_eq!(
            launches[0].draft.target.scope,
            SettleScope::Check("01-UAT.md#2".to_owned())
        );
        let prompt = launches[0].prompt();
        assert!(
            prompt.starts_with("/gsd-verify-work 01 The user will not perform test 2 (\u{201c}"),
            "{prompt}"
        );
        assert!(
            prompt.contains("in 01-UAT.md. Resume the UAT session of phase 01"),
            "{prompt}"
        );
        assert!(
            prompt.contains("record test 2 as result: skipped"),
            "{prompt}"
        );
        assert_eq!(
            status(&ctx),
            "Launched /gsd-verify-work 01 in alder-api: won't do for test 2"
        );
        assert_eq!(tree(&root), before, "the copy is byte-identical");
    }

    /// Quick 261119-f5b: `x` picked in the Enter action menu runs the hotkey path,
    /// so the chooser, the reason field and the default-No confirm all still stand
    /// between the menu and a launch, in the inbox and in the Owner block.
    #[test]
    fn x_through_the_menu_still_asks_and_the_default_is_no() {
        for host in ["inbox", "owner block"] {
            take_settle_launches();
            let (_dir, root) = temp_fixture("uat-pending");
            let before = tree(&root);
            let mut ctx = fixture_ctx(&root, ALIAS);
            let (key, _) = uat_row_and_first_item(&ctx);
            let mut stack = if host == "inbox" {
                Stack::inbox()
            } else {
                Stack::detail(&mut ctx, ALIAS, &key)
            };
            stack.key(&mut ctx, KeyCode::Enter); // the menu
            assert_eq!(stack.depth(), 1, "{host}: the menu is not a screen");
            assert!(stack.render(&ctx, 100, 30).contains(" Actions "), "{host}");
            // j to the `x` entry, Enter runs it.
            stack.key(&mut ctx, KeyCode::Char('j'));
            stack.key(&mut ctx, KeyCode::Enter);
            assert_eq!(stack.name(), "settle_chooser", "{host}");
            assert!(
                !stack.render(&ctx, 100, 30).contains(" Actions "),
                "{host}: no menu behind the chooser"
            );
            stack.key(&mut ctx, KeyCode::Char('w'));
            stack.keys(&mut ctx, "no device here");
            stack.key(&mut ctx, KeyCode::Enter);
            assert_eq!(stack.name(), "settle_confirm", "{host}");
            assert!(
                take_settle_launches().is_empty(),
                "{host}: nothing before Yes"
            );
            // Enter is the default No: the popup closes and nothing launches.
            stack.key(&mut ctx, KeyCode::Enter);
            assert_eq!(stack.depth(), 1, "{host}: the popup closed");
            assert!(take_settle_launches().is_empty(), "{host}: default No");
            assert_eq!(tree(&root), before, "{host}: nothing written");
        }
    }

    #[test]
    fn the_owner_block_row_flow_settles_the_phase() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let (key, _) = uat_row_and_first_item(&ctx);
        let mut stack = Stack::detail(&mut ctx, ALIAS, &key);
        // Enter opens the action menu on the row; `x` there is the hotkey.
        stack.key(&mut ctx, KeyCode::Enter);
        // On the row, no item selected: the phase target.
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('w'));
        stack.keys(&mut ctx, "no device here");
        stack.key(&mut ctx, KeyCode::Enter);
        stack.key(&mut ctx, KeyCode::Char('y'));
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1);
        assert_eq!(launches[0].draft.target.scope, SettleScope::Phase);
        assert!(
            launches[0]
                .prompt()
                .contains("The user will not perform tests 2, 3 in 01-UAT.md."),
            "{}",
            launches[0].prompt()
        );
    }

    /// 39-02, D-04: in the inbox, `x` on an item line settles that ONE check, on
    /// the row the whole phase; a merged row's second file's item is reachable.
    #[test]
    fn x_on_an_item_targets_one_check() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let before = tree(&root);
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut stack = Stack::inbox();
        stack.key(&mut ctx, KeyCode::Char(' '));
        stack.key(&mut ctx, KeyCode::Char('j'));
        stack.key(&mut ctx, KeyCode::Char('j'));
        assert_eq!(ctx.inbox_view.item_cursor.as_deref(), Some("01-UAT.md#3"));
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('w'));
        stack.keys(&mut ctx, "no device here");
        stack.key(&mut ctx, KeyCode::Enter);
        stack.key(&mut ctx, KeyCode::Char('y'));
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1);
        assert_eq!(
            launches[0].draft.target.scope,
            SettleScope::Check("01-UAT.md#3".to_owned())
        );
        assert!(
            launches[0]
                .prompt()
                .contains("record test 3 as result: skipped"),
            "{}",
            launches[0].prompt()
        );
        // `k` back to the row: `x` is the phase. The first settle still shows its
        // `settling...` badge on the row, and a second `x` is refused while it runs
        // (quick 261119-f5b, I-14), so the pending entry is cleared first.
        stack.key(&mut ctx, KeyCode::Char('k'));
        stack.key(&mut ctx, KeyCode::Char('k'));
        assert_eq!(ctx.inbox_view.item_cursor, None);
        ctx.settle_pending = Default::default();
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('p'));
        stack.key(&mut ctx, KeyCode::Enter);
        stack.key(&mut ctx, KeyCode::Char('y'));
        let launches = take_settle_launches();
        assert_eq!(launches[0].draft.target.scope, SettleScope::Phase);
        assert_eq!(tree(&root), before);
    }

    /// WR-02 (39 review): `j` past the last item of the last stop is a no-op; it
    /// must not drop the item selection, or a following `x` settles the phase.
    #[test]
    fn j_past_the_last_item_of_the_last_stop_keeps_the_item() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let rows = stop_rows(&ctx);
        let last = rows.last().cloned().expect("a row");
        let ids = crate::ui::screens::owner_block::inbox_item_ids(&last.src);
        let last_id = ids.last().cloned().expect("an item");
        ctx.inbox_view.cursor = Some(crate::ui::screens::inbox::InboxStop::Row(last.key()));
        ctx.inbox_view.expanded.insert(last.key());
        let mut stack = Stack::inbox();
        for _ in 0..ids.len() {
            stack.key(&mut ctx, KeyCode::Char('j'));
        }
        assert_eq!(ctx.inbox_view.item_cursor.as_ref(), Some(&last_id));
        stack.key(&mut ctx, KeyCode::Char('j'));
        assert!(ctx.inbox_view.file_cursor.is_some() && ctx.inbox_view.item_cursor.is_none());
        let on_file = ctx.inbox_view.file_cursor.clone();
        stack.key(&mut ctx, KeyCode::Char('j'));
        assert_eq!(ctx.inbox_view.file_cursor, on_file);
        stack.key(&mut ctx, KeyCode::Char('k'));
        assert_eq!(ctx.inbox_view.item_cursor.as_ref(), Some(&last_id));
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('w'));
        stack.keys(&mut ctx, "r");
        stack.key(&mut ctx, KeyCode::Enter);
        stack.key(&mut ctx, KeyCode::Char('y'));
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1);
        assert_eq!(launches[0].draft.target.scope, SettleScope::Check(last_id));
    }

    #[test]
    fn x_on_a_merged_rows_second_file_names_that_file() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("same-phase-merge");
        let mut ctx = fixture_ctx(&root, "market-app");
        let merged = stop_rows(&ctx)
            .into_iter()
            .find(|row| row.src.parts.len() > 1)
            .expect("a merged row");
        let ids = crate::ui::screens::owner_block::inbox_item_ids(&merged.src);
        let last = ids.last().cloned().expect("an item");
        assert!(last.contains("VERIFICATION"), "{ids:?}");
        ctx.inbox_view.cursor = Some(crate::ui::screens::inbox::InboxStop::Row(merged.key()));
        ctx.inbox_view.expanded.insert(merged.key());
        let mut stack = Stack::inbox();
        for _ in 0..crate::ui::screens::owner_block::inbox_sub_stops(&merged.src).len() {
            stack.key(&mut ctx, KeyCode::Char('j'));
        }
        assert_eq!(ctx.inbox_view.item_cursor.as_ref(), Some(&last));
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('w'));
        stack.keys(&mut ctx, "r");
        stack.key(&mut ctx, KeyCode::Enter);
        stack.key(&mut ctx, KeyCode::Char('y'));
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1, "{}", status(&ctx));
        let prompt = launches[0].prompt();
        assert!(
            prompt.contains("The user will not perform item 1 (\u{201c}")
                && prompt.contains(") in 43-VERIFICATION.md."),
            "{prompt}"
        );
    }

    #[test]
    fn a_vanished_check_is_not_launched() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let (key, id) = uat_row_and_first_item(&ctx);
        let mut stack = Stack::detail(&mut ctx, ALIAS, &key);
        stack.key(&mut ctx, KeyCode::Char(' '));
        stack.key(&mut ctx, KeyCode::Char('j'));
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('w'));
        stack.keys(&mut ctx, "r");
        stack.key(&mut ctx, KeyCode::Enter);
        for group in &mut ctx.owner_digests.get_mut(ALIAS).unwrap().groups {
            group.items.retain(|i| i.id != id);
        }
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(status(&ctx), "Check is no longer listed; nothing launched");
        assert!(take_settle_launches().is_empty());
    }

    /// The prompt of the Move flow on `heuristic-labels` phase 01 to phase 02 (D-08).
    const MOVE_PROMPT: &str = "/gsd-verify-work 01 The user moves tests 1, 2, 3, 4, 5, 6, 7 in 01-UAT.md to phase 02 \u{201c}Sync\u{201d}. Resume the UAT session of phase 01 without restarting it and record each of them as result: skipped, reason: Deferred follow-up: moved to phase 02. They must not block phase 01. Then add one todo as /gsd-capture does, titled \u{201c}Phase 02 Sync: rerun 7 checks from phase 01\u{201d}, so they come up again when phase 02 runs. Do not promote them to the 999.x backlog.";

    #[test]
    fn the_inbox_move_flow_launches_the_move_prompt_and_writes_nothing() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("heuristic-labels");
        let before = tree(&root);
        let mut ctx = fixture_ctx(&root, "pricing-app");
        let mut stack = Stack::inbox();
        assert!(stack.key(&mut ctx, KeyCode::Char('x')));
        assert!(stack.key(&mut ctx, KeyCode::Char('m')));
        assert_eq!((stack.name().as_str(), stack.depth()), ("settle_phase", 2));
        let step = stack.render(&ctx, 100, 24);
        assert!(step.contains("> 02  Sync"), "{step}");
        assert!(step.contains("  03  Plans"), "{step}");
        assert!(step.contains("Move phase 01 to phase> _"), "{step}");
        // `j` picks 03, `k` goes back to 02.
        stack.key(&mut ctx, KeyCode::Char('j'));
        assert!(stack.render(&ctx, 100, 24).contains("> 03  Plans"));
        stack.key(&mut ctx, KeyCode::Char('k'));
        assert!(stack.render(&ctx, 100, 24).contains("> 02  Sync"));
        assert!(stack.key(&mut ctx, KeyCode::Enter));
        assert_eq!(stack.name(), "settle_reason");
        let field = stack.render(&ctx, 100, 24);
        assert!(
            field.contains("Move phase 01 to phase 02: reason (optional)>"),
            "{field}"
        );
        assert!(
            stack.key(&mut ctx, KeyCode::Enter),
            "an empty reason is allowed"
        );
        assert_eq!(
            (stack.name().as_str(), stack.depth()),
            ("settle_confirm", 2)
        );
        let popup = stack.render(&ctx, 100, 40);
        assert!(
            popup.contains("Outcome    Move to phase 02 \u{201c}Sync\u{201d}"),
            "{popup}"
        );
        assert!(take_settle_launches().is_empty(), "nothing before Yes");
        assert!(stack.key(&mut ctx, KeyCode::Char('y')));
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1);
        assert_eq!(launches[0].prompt(), MOVE_PROMPT);
        assert_eq!(
            status(&ctx),
            "Launched /gsd-verify-work 01 in pricing-app: moved to phase 02 for phase 01 (7 checks)"
        );
        assert_eq!(tree(&root), before, "the app wrote nothing");
    }

    #[test]
    fn the_default_pick_follows_the_source_and_a_typed_phase_overrides_it() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("heuristic-labels");
        let mut ctx = fixture_ctx(&root, "pricing-app");
        // From phase 02 (second row) the first phase after it is 03.
        let rows = stop_rows(&ctx);
        let second = rows
            .iter()
            .find(|r| r.src.group.phase == "02")
            .expect("phase 02")
            .key();
        ctx.inbox_view.cursor = Some(crate::ui::screens::inbox::InboxStop::Row(second));
        let mut stack = Stack::inbox();
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('m'));
        assert!(stack.render(&ctx, 100, 24).contains("> 03  Plans"));
        // Typing overrides the pick: `1` is a known roadmap phase (01, pad-insensitive).
        stack.keys(&mut ctx, "1");
        stack.key(&mut ctx, KeyCode::Enter);
        stack.key(&mut ctx, KeyCode::Enter);
        stack.key(&mut ctx, KeyCode::Char('y'));
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1);
        let to = launches[0].choice.move_to.clone().expect("a target");
        assert_eq!(
            (to.number.as_str(), to.name.as_deref(), to.known),
            ("01", Some("Pricing"), true)
        );
    }

    #[test]
    fn an_unknown_phase_is_warned_in_the_popup() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("heuristic-labels");
        let mut ctx = fixture_ctx(&root, "pricing-app");
        let mut stack = Stack::inbox();
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('m'));
        stack.keys(&mut ctx, "7");
        stack.key(&mut ctx, KeyCode::Enter);
        stack.keys(&mut ctx, "later");
        stack.key(&mut ctx, KeyCode::Enter);
        assert_eq!(stack.name(), "settle_confirm");
        let popup = stack.render(&ctx, 100, 40);
        assert!(
            popup
                .contains("Phase 7 is not in the roadmap or backlog: GSD may create or reject it."),
            "{popup}"
        );
        assert!(popup.contains("Outcome    Move to phase 7"), "{popup}");
        stack.key(&mut ctx, KeyCode::Char('y'));
        let prompt = take_settle_launches().remove(0).prompt();
        assert!(prompt.contains("moved to phase 7. later."), "{prompt}");
        // A known phase carries no warning.
        let mut stack = Stack::inbox();
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('m'));
        stack.key(&mut ctx, KeyCode::Enter);
        stack.key(&mut ctx, KeyCode::Enter);
        assert!(!stack
            .render(&ctx, 100, 40)
            .contains("is not in the roadmap"));
    }

    #[test]
    fn the_phase_step_refuses_the_source_and_bad_entries() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("heuristic-labels");
        let mut ctx = fixture_ctx(&root, "pricing-app");
        let mut stack = Stack::inbox();
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('m'));
        stack.keys(&mut ctx, "1");
        assert!(
            !stack.key(&mut ctx, KeyCode::Enter),
            "the source phase stays on the step"
        );
        assert!(stack
            .render(&ctx, 100, 24)
            .contains("That is the source phase; pick another"));
        stack.key(&mut ctx, KeyCode::Backspace);
        stack.keys(&mut ctx, "1.");
        stack.key(&mut ctx, KeyCode::Enter);
        assert_eq!(stack.name(), "settle_phase");
        assert!(stack
            .render(&ctx, 100, 24)
            .contains("Not a phase number: 1."));
        // Only digits, letters and dots are taken, and a letter cannot start a number.
        stack.key(&mut ctx, KeyCode::Backspace);
        stack.key(&mut ctx, KeyCode::Backspace);
        stack.keys(&mut ctx, "\u{1b};-x");
        stack.key(&mut ctx, KeyCode::Char('3'));
        stack.keys(&mut ctx, ";\u{1b}");
        stack.key(&mut ctx, KeyCode::Enter);
        assert_eq!(stack.name(), "settle_reason", "only the `3` was taken");
        assert!(take_settle_launches().is_empty());
        // No candidate at all: the step says so, and Enter needs a number.
        let (_d2, root2) = temp_fixture("uat-pending");
        let mut ctx2 = fixture_ctx(&root2, ALIAS);
        let mut stack = Stack::inbox();
        stack.key(&mut ctx2, KeyCode::Char('x'));
        stack.key(&mut ctx2, KeyCode::Char('m'));
        assert!(stack
            .render(&ctx2, 100, 24)
            .contains("No other open phase: type a phase number"));
        stack.key(&mut ctx2, KeyCode::Enter);
        assert_eq!(stack.name(), "settle_phase");
        assert!(stack.render(&ctx2, 100, 24).contains("Type a phase number"));
    }

    #[test]
    fn every_move_cancel_launches_nothing() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("heuristic-labels");
        let before = tree(&root);
        let mut ctx = fixture_ctx(&root, "pricing-app");
        let mut stack = Stack::inbox();
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('m'));
        stack.key(&mut ctx, KeyCode::Esc);
        assert_eq!(
            (stack.depth(), stack.name().as_str()),
            (1, "inbox"),
            "Esc at the phase step"
        );
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('m'));
        stack.key(&mut ctx, KeyCode::Enter);
        stack.keys(&mut ctx, "later");
        stack.key(&mut ctx, KeyCode::Esc);
        assert_eq!(stack.depth(), 1, "Esc at the reason field");
        for code in [KeyCode::Esc, KeyCode::Char('n'), KeyCode::Enter] {
            stack.key(&mut ctx, KeyCode::Char('x'));
            stack.key(&mut ctx, KeyCode::Char('m'));
            stack.key(&mut ctx, KeyCode::Enter);
            stack.key(&mut ctx, KeyCode::Enter);
            assert_eq!(stack.name(), "settle_confirm");
            stack.key(&mut ctx, code);
            assert_eq!(stack.depth(), 1, "{code:?} at the popup");
        }
        assert!(take_settle_launches().is_empty());
        assert_eq!(tree(&root), before);
    }

    /// A Move plan on `fixture` for the row of its first stop (a check when
    /// `item` is `Some`).
    fn move_plan(
        fixture: &str,
        alias: &str,
        item: Option<&str>,
    ) -> (SettlePlan, AppContext, tempfile::TempDir) {
        let (dir, root) = temp_fixture(fixture);
        let ctx = fixture_ctx(&root, alias);
        let row = stop_rows(&ctx).remove(0);
        let draft = owner_settle::resolve_draft(&ctx, alias, &row.src.key, item).unwrap();
        let target = owner_settle::MoveTarget {
            number: "02".to_owned(),
            name: Some("Sync".to_owned()),
            known: true,
        };
        let plan = SettlePlan {
            draft,
            choice: SettleChoice {
                outcome: Outcome::Move,
                reason: String::new(),
                move_to: Some(target),
            },
        };
        (plan, ctx, dir)
    }

    /// The rows `body` takes at `inner_w`, and whether that is exactly its own line
    /// count (no `Paragraph` re-wrap).
    fn rows_of_body(body: &[Line<'static>], inner_w: u16) -> (u16, bool) {
        let rows = confirm_popup::wrapped_rows(body, inner_w);
        (rows, usize::from(rows) == body.len())
    }

    #[test]
    fn the_move_popup_fits_for_a_check_at_80x24() {
        let (plan, ctx, _dir) = move_plan("heuristic-labels", "pricing-app", Some("01-UAT.md#4"));
        let (inner_w, room) = confirm_popup::body_room(Rect::new(0, 0, 80, 24));
        let (rows, plain) = rows_of_body(&settle_body(&plan, inner_w), inner_w);
        assert!(rows <= room && plain, "{rows} rows, room {room}");
        let (rows, _) = rows_of_body(&settle_full_body(&plan, inner_w), inner_w);
        assert!(rows <= room, "full: {rows} rows > {room}");
        let mut screen = SettleConfirmScreen::new(AckHost::Inbox, plan);
        for full in [false, true] {
            screen.full = full;
            let text = draw_screen(&screen, &ctx, 80, 24);
            assert!(!screen.popup.clipped(), "full {full}\n{text}");
        }
    }

    #[test]
    fn the_move_popup_fits_for_a_phase_at_100x30() {
        let (plan, ctx, _dir) = move_plan("heuristic-labels", "pricing-app", None);
        let (inner_w, room) = confirm_popup::body_room(Rect::new(0, 0, 100, 30));
        for body in [
            settle_body(&plan, inner_w),
            settle_full_body(&plan, inner_w),
        ] {
            let rows = confirm_popup::wrapped_rows(&body, inner_w);
            assert!(rows <= room, "{rows} rows > {room}");
        }
        let mut screen = SettleConfirmScreen::new(AckHost::Inbox, plan.clone());
        for full in [false, true] {
            screen.full = full;
            let text = draw_screen(&screen, &ctx, 100, 30);
            assert!(!screen.popup.clipped(), "full {full}\n{text}");
        }
        // The 80x24 counts of the same plan, measured for the SUMMARY.
        let (w80, room80) = confirm_popup::body_room(Rect::new(0, 0, 80, 24));
        let compact = confirm_popup::wrapped_rows(&settle_body(&plan, w80), w80);
        let full = confirm_popup::wrapped_rows(&settle_full_body(&plan, w80), w80);
        eprintln!(
            "phase-level Move popup at 80x24: compact {compact} rows, full {full} rows of {room80}"
        );
        assert!(compact <= room80);
    }

    #[test]
    fn every_cancel_launches_nothing() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let before = tree(&root);
        let mut ctx = fixture_ctx(&root, ALIAS);

        let mut stack = Stack::inbox();
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Esc);
        assert_eq!(stack.depth(), 1, "Esc at the chooser");

        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('w'));
        stack.keys(&mut ctx, "no device");
        stack.key(&mut ctx, KeyCode::Esc);
        assert_eq!(stack.depth(), 1, "Esc at the reason field");

        for code in [KeyCode::Esc, KeyCode::Char('n'), KeyCode::Enter] {
            to_popup(&mut stack, &mut ctx);
            stack.key(&mut ctx, code);
            assert_eq!(stack.depth(), 1, "{code:?} at the popup");
            assert_eq!(stack.name(), "inbox");
        }
        assert!(take_settle_launches().is_empty());
        assert_eq!(tree(&root), before);
    }

    #[test]
    fn chorded_x_does_nothing() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut stack = Stack::inbox();
        for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
            assert!(!stack.key_with(&mut ctx, KeyCode::Char('x'), modifiers));
            assert_eq!(stack.depth(), 1);
        }
        assert!(ctx.status_message.is_none());
        assert!(take_settle_launches().is_empty());
    }

    #[test]
    fn wont_do_needs_a_reason() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut stack = Stack::inbox();
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('w'));
        assert!(
            !stack.key(&mut ctx, KeyCode::Enter),
            "Enter on an empty reason"
        );
        assert_eq!(stack.name(), "settle_reason");
        assert!(stack
            .render(&ctx, 80, 24)
            .contains("Won't do needs a reason"));
        // A reason of control characters only is empty too.
        stack.keys(&mut ctx, "\u{7}\u{1b}");
        assert!(!stack.key(&mut ctx, KeyCode::Enter));
        assert_eq!(stack.name(), "settle_reason");
        // Typing clears the error and a real reason goes on.
        stack.keys(&mut ctx, "no device");
        assert!(!stack.render(&ctx, 80, 24).contains("needs a reason"));
        assert!(stack.key(&mut ctx, KeyCode::Enter));
        assert_eq!(stack.name(), "settle_confirm");
        assert!(take_settle_launches().is_empty());
    }

    #[test]
    fn enter_defaults_to_no() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut screen = SettleConfirmScreen::new(
            AckHost::Inbox,
            tracer_plan(&ctx, Outcome::WontDo, "no device here"),
        );
        let action = screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::Pop));
        assert!(ctx.status_message.is_none(), "No says nothing");
        assert!(take_settle_launches().is_empty());
    }

    /// The popup's inner text with every whitespace removed, so a wrap point never
    /// matters when an element is looked up.
    fn squashed(text: &str) -> String {
        text.lines()
            .filter_map(|l| {
                let c: Vec<char> = l.chars().collect();
                let from = c.iter().position(|ch| *ch == '\u{2502}')?;
                let to = c.iter().rposition(|ch| *ch == '\u{2502}')?;
                (to > from).then(|| c[from + 1..to].iter().collect::<String>())
            })
            .collect::<String>()
            .split_whitespace()
            .collect()
    }

    /// Draw `screen` once and hand back the dump.
    fn draw_screen(screen: &SettleConfirmScreen, ctx: &AppContext, w: u16, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| screen.render(f, f.area(), ctx)).unwrap();
        dump(&terminal, w, h)
    }

    /// The argv as the full view draws it: through `shown`, the prompt in quotes.
    fn drawn_argv(plan: &SettlePlan) -> String {
        let argv = settle_mux_argv(plan).unwrap();
        let elements = argv.as_slice();
        let last = elements.len() - 1;
        elements
            .iter()
            .enumerate()
            .map(|(i, e)| {
                if i == last {
                    format!("\"{}\"", shown(e))
                } else {
                    shown(e)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// How many `-u` the argv carries, and the variable names after them.
    fn unset_in(plan: &SettlePlan) -> (usize, Vec<String>) {
        let argv = settle_mux_argv(plan).unwrap();
        let elements = argv.as_slice();
        let names = elements
            .windows(2)
            .filter(|pair| pair[0] == "-u")
            .map(|pair| pair[1].clone())
            .collect::<Vec<_>>();
        (elements.iter().filter(|e| *e == "-u").count(), names)
    }

    #[test]
    fn the_popup_fits_80x24_as_a_summary_and_as_the_exact_argv() {
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let plan = tracer_plan(&ctx, Outcome::WontDo, "no device here");
        let (n, names) = unset_in(&plan);
        assert!(n > 0, "the launcher unsets session variables");
        let mut screen = SettleConfirmScreen::new(AckHost::Inbox, plan.clone());
        let text = draw_screen(&screen, &ctx, 80, 24);
        assert!(!screen.popup.clipped(), "{text}");
        assert!(!text.contains("Cut off"), "{text}");
        for want in [
            "Settle via /gsd-verify-work",
            "Project    alder-api",
            "Target     phase 01: tests 2, 3 in 01-UAT.md",
            "Outcome    Won't do",
            "Reason     no device here",
            &format!("Runs       claude /gsd-verify-work 01 \u{b7} env -u ({n} session vars)"),
            "Yes starts a new session \u{b7} GSD records it \u{b7} this app writes nothing",
            "d full command \u{b7} y/n \u{b7} \u{2190}/\u{2192} move \u{b7} Enter select \u{b7} Esc cancel",
            "[ Yes ]",
            "[ No ]",
        ] {
            assert!(text.contains(want), "{want:?} missing from\n{text}");
        }
        for name in &names {
            assert!(!text.contains(name.as_str()), "{name} drawn in\n{text}");
        }
        assert_eq!(screen.popup.focus, confirm_popup::ConfirmFocus::No);

        // `d`: the exact argv, every element on screen, in order, the prompt quoted.
        assert!(matches!(
            screen.handle_key(KeyCode::Char('d'), KeyModifiers::NONE, &mut ctx),
            ScreenAction::None
        ));
        let full = draw_screen(&screen, &ctx, 80, 24);
        assert!(!screen.popup.clipped(), "{full}");
        let joined: String = drawn_argv(&plan).split_whitespace().collect();
        assert!(
            squashed(&full).contains(&joined),
            "the argv is not drawn whole in\n{full}"
        );
        let argv = settle_mux_argv(&plan).unwrap();
        for element in argv.as_slice() {
            let element: String = shown(element).split_whitespace().collect();
            assert!(squashed(&full).contains(&element), "{element} missing");
        }
        assert!(full.contains("d summary \u{b7} y/n"), "{full}");
        assert!(!full.contains("d full command"), "{full}");
        assert!(
            full.contains("Yes starts a new session \u{b7} GSD records it"),
            "{full}"
        );
    }

    #[test]
    fn d_toggles_the_full_command_and_launches_nothing() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let before = tree(&root);
        let mut ctx = fixture_ctx(&root, ALIAS);
        let plan = tracer_plan(&ctx, Outcome::WontDo, "no device here");
        let joined: String = drawn_argv(&plan).split_whitespace().collect();
        let mut screen = SettleConfirmScreen::new(AckHost::Inbox, plan);
        let compact = draw_screen(&screen, &ctx, 80, 24);
        assert!(!squashed(&compact).contains(&joined));
        // A chorded `d` is not the toggle.
        for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT] {
            screen.handle_key(KeyCode::Char('d'), modifiers, &mut ctx);
            assert_eq!(draw_screen(&screen, &ctx, 80, 24), compact);
        }
        assert!(matches!(
            screen.handle_key(KeyCode::Char('d'), KeyModifiers::NONE, &mut ctx),
            ScreenAction::None
        ));
        assert!(squashed(&draw_screen(&screen, &ctx, 80, 24)).contains(&joined));
        assert!(matches!(
            screen.handle_key(KeyCode::Char('d'), KeyModifiers::NONE, &mut ctx),
            ScreenAction::None
        ));
        assert_eq!(draw_screen(&screen, &ctx, 80, 24), compact, "and back");
        assert!(ctx.status_message.is_none(), "d sets no status");
        assert!(take_settle_launches().is_empty(), "d launches nothing");
        assert_eq!(tree(&root), before);

        // The stack flow: `d`, then `y` from the (unclipped) full view launches once.
        let mut stack = Stack::inbox();
        to_popup(&mut stack, &mut ctx);
        assert!(!stack.key(&mut ctx, KeyCode::Char('d')));
        assert!(squashed(&stack.render(&ctx, 100, 30)).contains(&joined));
        assert!(take_settle_launches().is_empty());
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(stack.depth(), 1);
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1);
        assert_eq!(launches[0].prompt(), TRACER_PROMPT);
        assert_eq!(tree(&root), before);
    }

    /// The tracer plans fit the 19 body rows an 80x24 terminal gives the popup, in both
    /// views, and the summary draws exactly the rows it has lines (no re-wrap).
    #[test]
    fn the_tracer_popups_fit_the_body_room_at_80x24() {
        let (_dir, root) = temp_fixture("uat-pending");
        let ctx = fixture_ctx(&root, ALIAS);
        let (inner_w, room) = confirm_popup::body_room(Rect::new(0, 0, 80, 24));
        for (outcome, reason) in [
            (Outcome::WontDo, "no device here"),
            (Outcome::DeferPermanently, ""),
        ] {
            let plan = tracer_plan(&ctx, outcome, reason);
            let (rows, plain) = rows_of_body(&settle_body(&plan, inner_w), inner_w);
            assert!(rows <= room, "{outcome:?}: {rows} rows > {room}");
            assert!(plain, "{outcome:?}: the summary is re-wrapped");
            let (rows, _) = rows_of_body(&settle_full_body(&plan, inner_w), inner_w);
            assert!(rows <= room, "{outcome:?} full: {rows} rows > {room}");
        }
    }

    /// A live Claude session whose cwd is exactly `root`.
    fn session_in(root: &std::path::Path) -> crate::session_detector::ClaudeSession {
        crate::session_detector::ClaudeSession {
            pid: 4242,
            kind: crate::session_detector::SessionKind::Claude,
            session_id: None,
            working_dir: root.to_path_buf(),
            start_time: None,
            tty: None,
        }
    }

    #[test]
    fn p_opens_an_optional_reason() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut stack = Stack::inbox();
        stack.key(&mut ctx, KeyCode::Char('x'));
        let chooser = stack.render(&ctx, 100, 24);
        assert!(
            chooser.contains("Settle phase 01: record it as skipped, so the phase can close"),
            "{chooser}"
        );
        let keys = "[w] won't do  [p] defer permanently  [m] move to phase M  [Esc] cancel";
        assert!(chooser.contains(keys), "{chooser}");
        let narrow = stack.render(&ctx, 80, 24);
        assert!(narrow.contains(keys), "{narrow}");
        stack.key(&mut ctx, KeyCode::Char('p'));
        assert_eq!(stack.name(), "settle_reason");
        let field = stack.render(&ctx, 100, 24);
        assert!(
            field.contains("Deferred permanently phase 01: reason (optional)>"),
            "{field}"
        );
        assert!(
            stack.key(&mut ctx, KeyCode::Enter),
            "an empty reason is allowed"
        );
        assert_eq!(stack.name(), "settle_confirm");
        assert!(stack.key(&mut ctx, KeyCode::Char('y')));
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1);
        let prompt = launches[0].prompt();
        assert!(
            prompt.contains(
                "reason: Deferred follow-up: deferred permanently. They must not block phase 01"
            ),
            "{prompt}"
        );
        assert_eq!(
            status(&ctx),
            "Launched /gsd-verify-work 01 in alder-api: deferred permanently for phase 01 (2 checks)"
        );
    }

    #[test]
    fn yes_rechecks_a_vanished_row() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut stack = Stack::inbox();
        to_popup(&mut stack, &mut ctx);
        // The refresh between `x` and Yes dropped the row.
        ctx.owner_digests.insert(
            ALIAS.to_owned(),
            crate::state_reader::owner_items::OwnerDigest::default(),
        );
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(status(&ctx), "Row is no longer listed; nothing launched");
        assert!(take_settle_launches().is_empty());
    }

    #[test]
    fn yes_rechecks_a_changed_row() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut stack = Stack::inbox();
        to_popup(&mut stack, &mut ctx);
        // A check was added to the group while the popup was open.
        let group = ctx
            .owner_digests
            .get_mut(ALIAS)
            .unwrap()
            .groups
            .iter_mut()
            .find(|g| g.lane == crate::state_reader::owner_items::Lane::Test)
            .unwrap();
        let mut extra = group.items[0].clone();
        extra.id = "added".to_owned();
        extra.test = Some("9".to_owned());
        group.items.push(extra);
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(
            status(&ctx),
            "The row changed since the popup opened; nothing launched"
        );
        assert!(take_settle_launches().is_empty());
    }

    #[test]
    fn yes_rechecks_a_new_live_session() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut stack = Stack::inbox();
        to_popup(&mut stack, &mut ctx);
        ctx.active_sessions.push(session_in(&root));
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(
            status(&ctx),
            "A session started or ended in alder-api since the popup opened; nothing launched"
        );
        assert!(take_settle_launches().is_empty());
        // And the other direction: a session that vanished.
        let mut stack = Stack::inbox();
        to_popup(&mut stack, &mut ctx);
        ctx.active_sessions.clear();
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(
            status(&ctx),
            "A session started or ended in alder-api since the popup opened; nothing launched"
        );
        assert!(take_settle_launches().is_empty());
    }

    #[test]
    fn a_live_session_is_focused_not_relaunched() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        ctx.active_sessions.push(session_in(&root));
        let mut stack = Stack::inbox();
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('w'));
        stack.keys(&mut ctx, "no device here");
        stack.key(&mut ctx, KeyCode::Enter);
        let text = stack.render(&ctx, 100, 30);
        for want in [
            "Claude runs here",
            "Yes focuses it, copies the command",
            "no new session",
        ] {
            assert!(text.contains(want), "{want:?} missing from\n{text}");
        }
        stack.key(&mut ctx, KeyCode::Char('y'));
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1);
        assert!(
            launches[0].draft.live_session.is_some(),
            "the real path focuses it"
        );
        assert!(
            status(&ctx).starts_with(crate::multiplexer::FOCUS_OK_PREFIX),
            "{}",
            status(&ctx)
        );
        // A prefix-sharing sibling directory is a different project.
        let mut ctx = fixture_ctx(&root, ALIAS);
        let sibling = root.with_file_name(format!(
            "{}-sibling",
            root.file_name().unwrap().to_string_lossy()
        ));
        ctx.active_sessions.push(session_in(&sibling));
        let mut stack = Stack::inbox();
        to_popup(&mut stack, &mut ctx);
        stack.key(&mut ctx, KeyCode::Char('y'));
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1);
        assert!(launches[0].draft.live_session.is_none());
    }

    #[test]
    fn a_cut_off_view_cannot_confirm_and_d_returns_to_the_summary() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut screen = SettleConfirmScreen::new(
            AckHost::Inbox,
            tracer_plan(&ctx, Outcome::WontDo, "no device here"),
        );
        let key = |screen: &mut SettleConfirmScreen, ctx: &mut AppContext, code| {
            screen.handle_key(code, KeyModifiers::NONE, ctx)
        };
        // The full view at 60x20 does not fit.
        key(&mut screen, &mut ctx, KeyCode::Char('d'));
        let text = draw_screen(&screen, &ctx, 60, 20);
        assert!(screen.popup.clipped(), "{text}");
        assert!(
            text.contains(confirm_popup::CLIPPED_HINT.trim_end()),
            "{text}"
        );
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('y')),
            ScreenAction::None
        ));
        key(&mut screen, &mut ctx, KeyCode::Left);
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Enter),
            ScreenAction::None
        ));
        assert!(take_settle_launches().is_empty());
        // `d` back to the summary, which fits 60x20 and confirms.
        key(&mut screen, &mut ctx, KeyCode::Char('d'));
        let text = draw_screen(&screen, &ctx, 60, 20);
        assert!(!screen.popup.clipped(), "{text}");
        assert!(!text.contains("Cut off"), "{text}");
        // A summary the terminal cuts off cannot be confirmed either.
        let text = draw_screen(&screen, &ctx, 60, 12);
        assert!(screen.popup.clipped(), "{text}");
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('y')),
            ScreenAction::None
        ));
        assert!(take_settle_launches().is_empty());
        for code in [KeyCode::Char('n'), KeyCode::Esc] {
            assert!(matches!(
                key(&mut screen, &mut ctx, code),
                ScreenAction::Pop
            ));
        }
        // The summary at 60x20 launches through Yes.
        draw_screen(&screen, &ctx, 60, 20);
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('y')),
            ScreenAction::Pop
        ));
        assert_eq!(take_settle_launches().len(), 1);
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect()
    }

    fn is_bold(span: &Span<'_>) -> bool {
        span.style.add_modifier.contains(Modifier::BOLD)
    }

    fn squash(text: &str) -> String {
        text.split_whitespace().collect()
    }

    fn the_instruction() -> (&'static str, Vec<String>) {
        let text = TRACER_PROMPT
            .strip_prefix("/gsd-verify-work 01 ")
            .expect("the slash command leads");
        let needles = [
            "tests 2, 3 in 01-UAT.md",
            "phase 01",
            "result: skipped",
            "no device here",
        ]
        .map(str::to_owned)
        .to_vec();
        (text, needles)
    }

    #[test]
    fn emphasized_rows_change_no_character_and_stay_in_the_width() {
        let (text, needles) = the_instruction();
        let rows = emphasized_rows(text, &needles, 59, usize::MAX);
        assert!(rows.len() > INSTRUCTION_ROWS, "{} rows", rows.len());
        let joined: String = rows.iter().flatten().map(|s| s.content.as_ref()).collect();
        assert_eq!(squash(&joined), squash(text));
        let mut bold_seen = 0;
        for row in &rows {
            assert!(Line::from(row.clone()).width() <= 59);
            for span in row.iter().filter(|span| is_bold(span)) {
                bold_seen += 1;
                assert!(
                    needles.iter().any(|n| n.contains(span.content.as_ref())),
                    "{:?} is not part of a needle",
                    span.content
                );
            }
            for span in row.iter().filter(|span| !is_bold(span)) {
                assert_eq!(span.style.fg, Some(Color::DarkGray));
            }
        }
        assert!(bold_seen >= 4, "{bold_seen} bold spans");
        // Capped: exactly three rows, the last ending in an ellipsis, none too wide.
        let capped = emphasized_rows(text, &needles, 59, INSTRUCTION_ROWS);
        assert_eq!(capped.len(), INSTRUCTION_ROWS);
        let last: String = capped[2].iter().map(|s| s.content.as_ref()).collect();
        assert!(last.ends_with('\u{2026}'), "{last}");
        for row in &capped {
            assert!(Line::from(row.clone()).width() <= 59);
        }
        // A short text is not cut and gets no ellipsis.
        let short = emphasized_rows("phase 01 only", &needles, 59, INSTRUCTION_ROWS);
        let short: String = short.iter().flatten().map(|s| s.content.as_ref()).collect();
        assert_eq!(short, "phase 01 only");
    }

    #[test]
    fn emphasis_ignores_absent_and_glued_needles_and_splits_on_char_boundaries() {
        let none = emphasized_rows("nothing to see", &["absent".to_owned()], 20, 3);
        assert!(none.iter().flatten().all(|span| !is_bold(span)));
        let glued = emphasized_rows("phase 10 and phase 1.", &["phase 1".to_owned()], 40, 3);
        let bolds: Vec<&str> = glued
            .iter()
            .flatten()
            .filter(|span| is_bold(span))
            .map(|span| span.content.as_ref())
            .collect();
        assert_eq!(bolds, ["phase 1"], "only the one not followed by a digit");
        let quoted = "test 2 (\u{201c}caf\u{e9} \u{201c}x\u{201d}\u{201d}) in 01-UAT.md. Next.";
        let needle = "test 2 (\u{201c}caf\u{e9} \u{201c}x\u{201d}\u{201d}) in 01-UAT.md".to_owned();
        for width in 3..30 {
            let rows = emphasized_rows(quoted, std::slice::from_ref(&needle), width, usize::MAX);
            let joined: String = rows.iter().flatten().map(|s| s.content.as_ref()).collect();
            assert_eq!(squash(&joined), squash(quoted), "width {width}");
        }
    }

    #[test]
    fn the_summary_draws_the_command_bold_and_the_key_facts_bold() {
        let (_dir, root) = temp_fixture("uat-pending");
        let ctx = fixture_ctx(&root, ALIAS);
        let plan = tracer_plan(&ctx, Outcome::WontDo, "no device here");
        let body = settle_body(&plan, 70);
        let runs = body
            .iter()
            .position(|line| line_text(line).starts_with("Runs"))
            .expect("a Runs row");
        let rows: Vec<&Line<'_>> = body[runs + 1..]
            .iter()
            .take_while(|line| !line_text(line).is_empty())
            .collect();
        assert_eq!(rows.len(), INSTRUCTION_ROWS, "{rows:?}");
        let mut bold_text = String::new();
        for row in &rows {
            assert_eq!(
                line_text(row).chars().take(11).collect::<String>(),
                " ".repeat(11)
            );
            for span in row.spans.iter().skip(1) {
                if is_bold(span) {
                    bold_text.push_str(&span.content);
                } else {
                    assert_eq!(span.style.fg, Some(Color::DarkGray), "{span:?}");
                }
            }
        }
        let bold_text = squash(&bold_text);
        for want in ["tests2,3in01-UAT.md", "phase01"] {
            assert!(bold_text.contains(want), "{want} not bold in {bold_text}");
        }
        let runs = &body[runs];
        let bold_runs: String = runs
            .spans
            .iter()
            .filter(|s| is_bold(s))
            .map(|s| s.content.as_ref())
            .collect();
        assert!(
            bold_runs.contains("claude /gsd-verify-work 01"),
            "{bold_runs}"
        );
        assert!(
            runs.spans
                .iter()
                .any(|s| !is_bold(s) && s.content.contains("env -u (")),
            "{runs:?}"
        );
    }

    #[test]
    fn the_compact_popup_fits_80x24_for_every_outcome_and_scope() {
        let (inner_w, room) = confirm_popup::body_room(Rect::new(0, 0, 80, 24));
        let (_dir, root) = temp_fixture("uat-pending");
        let uat = fixture_ctx(&root, ALIAS);
        let mut live = tracer_plan(&uat, Outcome::WontDo, "no device here");
        live.draft.live_session = Some(session_in(&root));
        let (check, ctx_check, _d1) =
            move_plan("heuristic-labels", "pricing-app", Some("01-UAT.md#4"));
        let (phase, ctx_phase, _d2) = move_plan("heuristic-labels", "pricing-app", None);
        let unknown = |mut plan: SettlePlan| {
            plan.choice.move_to = Some(MoveTarget {
                number: "7".to_owned(),
                name: None,
                known: false,
            });
            plan
        };
        let reasoned = |mut plan: SettlePlan| {
            plan.choice.reason = "no device here".to_owned();
            plan
        };
        let cases: Vec<(&str, SettlePlan, &AppContext)> = vec![
            (
                "won't do with a reason",
                tracer_plan(&uat, Outcome::WontDo, "no device here"),
                &uat,
            ),
            (
                "deferred permanently, no reason",
                tracer_plan(&uat, Outcome::DeferPermanently, ""),
                &uat,
            ),
            ("move, known check", check.clone(), &ctx_check),
            ("move, known phase", phase.clone(), &ctx_phase),
            ("move, unknown check", unknown(check), &ctx_check),
            ("move, phase with a reason", reasoned(phase), &ctx_phase),
            ("live session", live, &uat),
        ];
        for (label, plan, ctx) in cases {
            let body = settle_body(&plan, inner_w);
            let (rows, plain) = rows_of_body(&body, inner_w);
            assert!(rows <= room, "{label}: {rows} rows > {room}");
            assert!(plain, "{label}: the summary is re-wrapped");
            let mut screen = SettleConfirmScreen::new(AckHost::Inbox, plan.clone());
            let text = draw_screen(&screen, ctx, 80, 24);
            assert!(!screen.popup.clipped(), "{label}\n{text}");
            eprintln!("{label}: {rows} rows of {room}");
            // The full view fits too, except an unknown phase's, which needs one row
            // more than 80x24 has (as it always did): it clips and `d` goes back.
            screen.full = true;
            let text = draw_screen(&screen, ctx, 80, 24);
            let unknown_phase = plan.choice.move_to.as_ref().is_some_and(|to| !to.known);
            assert_eq!(
                screen.popup.clipped(),
                unknown_phase,
                "{label} (full)\n{text}"
            );
            // The unknown-phase warning keeps its Yellow and its words in both views.
            if unknown_phase {
                let warning =
                    "Phase 7 is not in the roadmap or backlog: GSD may create or reject it.";
                for body in [
                    settle_body(&plan, inner_w),
                    settle_full_body(&plan, inner_w),
                ] {
                    let yellow: String = body
                        .iter()
                        .filter(|line| {
                            !line.spans.is_empty()
                                && line.spans.iter().all(|s| s.style.fg == Some(Color::Yellow))
                        })
                        .map(line_text)
                        .collect();
                    assert_eq!(squash(&yellow), squash(warning), "{label}");
                }
                assert!(squash(&text).contains(&squash(warning)), "{label}\n{text}");
            }
        }
    }

    /// The popup flow with a live session in the project: `x w <reason> Enter y`.
    fn live_flow(root: &std::path::Path) -> (AppContext, Stack) {
        take_settle_launches();
        let mut ctx = fixture_ctx(root, ALIAS);
        ctx.active_sessions.push(session_in(root));
        let mut stack = Stack::inbox();
        to_popup(&mut stack, &mut ctx);
        (ctx, stack)
    }

    const FOCUSED: &str = "alder-api \u{b7} paste the command there";

    #[test]
    fn a_failed_focus_never_says_paste_there() {
        let ok = format!("{} alder-api", crate::multiplexer::FOCUS_OK_PREFIX);
        assert_eq!(
            live_settle_lead(&ok, "alder-api"),
            format!("{} {FOCUSED}", crate::multiplexer::FOCUS_OK_PREFIX)
        );
        let failed = "Tab-switching requires running inside tmux or herdr (none detected)";
        let lead = live_settle_lead(failed, "alder-api");
        assert!(lead.starts_with(failed), "{lead}");
        assert!(!lead.contains("paste the command there"), "{lead}");
        assert!(lead.ends_with("Claude session in alder-api"), "{lead}");
    }

    #[test]
    fn a_live_session_gets_the_prompt_by_osc52() {
        crate::ui::screens::owner_actions::force_clipboard(None);
        let (_dir, root) = temp_fixture("uat-pending");
        let before = tree(&root);
        let (mut ctx, mut stack) = live_flow(&root);
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(stack.depth(), 1, "the popup closed in the same transition");
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1, "one focus, no second session");
        let prompt = launches[0].prompt();
        assert_eq!(prompt, TRACER_PROMPT);
        let sequence = stack.take_osc52().expect("the sequence reached the stack");
        assert_eq!(sequence, crate::clipboard::osc52(&prompt));
        assert!(sequence.starts_with("\u{1b}]52;c;") && sequence.ends_with('\u{7}'));
        assert!(!sequence.contains("claude"), "never the argv");
        assert_eq!(
            status(&ctx),
            format!(
                "{} {FOCUSED}; Sent 1 line to the terminal clipboard (OSC 52; the terminal must allow it)",
                crate::multiplexer::FOCUS_OK_PREFIX
            )
        );
        assert!(owner_actions::seam_ran().is_empty());
        assert_eq!(tree(&root), before);
    }

    #[test]
    fn a_live_session_gets_the_prompt_by_a_native_tool() {
        use crate::clipboard::{ClipboardProbes, Tool};
        crate::ui::screens::owner_actions::force_clipboard(Some(ClipboardProbes {
            os: "linux",
            wayland_display: true,
            display: false,
            on_path: vec!["wl-copy"],
        }));
        let (_dir, root) = temp_fixture("uat-pending");
        let (mut ctx, mut stack) = live_flow(&root);
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(stack.depth(), 1);
        assert!(stack.take_osc52().is_none(), "the native tool is used");
        assert_eq!(
            owner_actions::seam_ran(),
            vec![(Tool::WlCopy, TRACER_PROMPT.to_owned(), 1)]
        );
        assert_eq!(
            status(&ctx),
            format!("{} {FOCUSED}", crate::multiplexer::FOCUS_OK_PREFIX)
        );
        assert_eq!(take_settle_launches().len(), 1);
        crate::ui::screens::owner_actions::force_clipboard(None);
    }

    #[test]
    fn an_owner_block_live_session_copies_the_check_prompt() {
        crate::ui::screens::owner_actions::force_clipboard(None);
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        ctx.active_sessions.push(session_in(&root));
        let (key, _) = uat_row_and_first_item(&ctx);
        let mut stack = Stack::detail(&mut ctx, ALIAS, &key);
        stack.key(&mut ctx, KeyCode::Char(' '));
        stack.key(&mut ctx, KeyCode::Char('j'));
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('w'));
        stack.keys(&mut ctx, "no device here");
        stack.key(&mut ctx, KeyCode::Enter);
        stack.key(&mut ctx, KeyCode::Char('y'));
        let launches = take_settle_launches();
        assert_eq!(launches.len(), 1);
        assert_eq!(
            launches[0].draft.target.scope,
            SettleScope::Check("01-UAT.md#2".to_owned())
        );
        assert_eq!(
            stack.take_osc52(),
            Some(crate::clipboard::osc52(&launches[0].prompt()))
        );
    }

    #[test]
    fn no_live_session_copies_nothing() {
        crate::ui::screens::owner_actions::force_clipboard(None);
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut stack = Stack::inbox();
        to_popup(&mut stack, &mut ctx);
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(stack.depth(), 1);
        assert!(stack.take_osc52().is_none());
        assert!(owner_actions::seam_ran().is_empty());
        assert!(status(&ctx).starts_with("Launched "), "{}", status(&ctx));
        assert_eq!(take_settle_launches().len(), 1);
    }

    /// D-13b (SC5): every outcome x host x target, on a throwaway copy each, through
    /// the real key path to Yes. Each cell launches exactly the prompt the product
    /// builds from the resolved target and leaves its copy byte-identical.
    #[test]
    fn the_flow_matrix_launches_every_expected_prompt_and_writes_nothing() {
        const PRICING: &str = "pricing-app";
        let outcomes = [
            ('w', Outcome::WontDo),
            ('p', Outcome::DeferPermanently),
            ('m', Outcome::Move),
        ];
        let mut cells = 0;
        for (letter, outcome) in outcomes {
            for host in ["inbox", "owner block"] {
                for check in [false, true] {
                    let label = format!(
                        "{outcome:?} / {host} / {}",
                        if check { "check" } else { "phase" }
                    );
                    take_settle_launches();
                    let (_dir, root) = temp_fixture("heuristic-labels");
                    let before = tree(&root);
                    let mut ctx = fixture_ctx(&root, PRICING);
                    let row = stop_rows(&ctx).remove(0);
                    let item = check.then(|| {
                        crate::ui::screens::owner_block::inbox_item_ids(&row.src).remove(0)
                    });
                    let mut stack = if host == "inbox" {
                        Stack::inbox()
                    } else {
                        Stack::detail(&mut ctx, PRICING, &row.src.key)
                    };
                    if check {
                        stack.key(&mut ctx, KeyCode::Char(' '));
                        stack.key(&mut ctx, KeyCode::Char('j'));
                    }
                    assert!(stack.key(&mut ctx, KeyCode::Char('x')), "{label}: x");
                    assert_eq!(stack.name(), "settle_chooser", "{label}");
                    stack.key(&mut ctx, KeyCode::Char(letter));
                    if outcome == Outcome::Move {
                        assert_eq!(stack.name(), "settle_phase", "{label}");
                        stack.key(&mut ctx, KeyCode::Enter);
                    }
                    assert_eq!(stack.name(), "settle_reason", "{label}");
                    stack.keys(&mut ctx, "no device here");
                    stack.key(&mut ctx, KeyCode::Enter);
                    assert_eq!(stack.name(), "settle_confirm", "{label}");
                    assert!(
                        take_settle_launches().is_empty(),
                        "{label}: nothing before Yes"
                    );
                    stack.key(&mut ctx, KeyCode::Char('y'));
                    assert_eq!(stack.depth(), 1, "{label}: the popup closed");

                    let draft =
                        owner_settle::resolve_draft(&ctx, PRICING, &row.src.key, item.as_deref())
                            .unwrap_or_else(|why| panic!("{label}: {why}"));
                    let choice = SettleChoice {
                        outcome,
                        reason: "no device here".to_owned(),
                        move_to: (outcome == Outcome::Move).then(|| owner_settle::MoveTarget {
                            number: "02".to_owned(),
                            name: Some("Sync".to_owned()),
                            known: true,
                        }),
                    };
                    let expected = owner_settle::prompt(&draft.target, &choice);
                    assert_eq!(draft.target.scope == SettleScope::Phase, !check, "{label}");
                    let launches = take_settle_launches();
                    assert_eq!(launches.len(), 1, "{label}: exactly one launch");
                    assert_eq!(launches[0].prompt(), expected, "{label}: the prompt");
                    assert_eq!(
                        launches[0].draft.cwd, root,
                        "{label}: the project is the cwd"
                    );
                    assert!(
                        status(&ctx).starts_with("Launched "),
                        "{label}: {}",
                        status(&ctx)
                    );
                    assert_eq!(tree(&root), before, "{label}: the copy is byte-identical");
                    cells += 1;
                }
            }
        }
        assert_eq!(cells, 12);
    }
}
