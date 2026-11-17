//! The screens of settle (`x`, 39): the one-row chooser, the phase step of a Move,
//! the reason field and the default-No confirm popup that shows the exact argv.
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

/// `x`: the one-row chooser `Settle {target}: [w] won't do  [p] defer permanently
/// [m] move  [Esc] cancel`. `w` and `p` replace it with the reason field, `m` with
/// the phase step; Esc cancels.
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
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        render_host(&self.host, frame, chunks[0], ctx);
        let key = |key: &'static str, rest: &'static str| {
            [Span::styled(key, bold()), Span::styled(rest, muted())]
        };
        let mut spans = vec![Span::raw(format!(
            "  Settle {}: ",
            shown(&self.draft.target.short())
        ))];
        spans.extend(key("[w]", " won't do  "));
        spans.extend(key("[p]", " defer permanently  "));
        spans.extend(key("[m]", " move  "));
        spans.extend(key("[Esc]", " cancel"));
        let line = fit_spans(spans, usize::from(chunks[1].width));
        frame.render_widget(Paragraph::new(line), chunks[1]);
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

/// The popup in front of Yes: default No, Yellow accent, the exact argv drawn.
/// Only Yes launches anything, and only after `settle_recheck` found the row as
/// it was shown. A cut-off popup cannot be confirmed (the `confirm_popup`
/// guarantee).
pub struct SettleConfirmScreen {
    host: AckHost,
    plan: SettlePlan,
    popup: ConfirmPopup,
}

impl SettleConfirmScreen {
    pub(crate) fn new(host: AckHost, plan: SettlePlan) -> Self {
        Self {
            host,
            plan,
            popup: ConfirmPopup::default(),
        }
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
        let focus = owner_settle::launch_settle(&fresh);
        if fresh.draft.live_session.is_none() {
            ctx.status_message = Some((focus, std::time::Instant::now()));
            return ScreenAction::Pop;
        }
        let lead = format!("{focus} \u{b7} paste the command there");
        match owner_actions::copy_led(ctx, lead, fresh.prompt(), 1) {
            ScreenAction::WriteOsc52(sequence) => ScreenAction::PopWriteOsc52(sequence),
            _ => ScreenAction::Pop,
        }
    }
}

/// The popup body (D-10): BOLD labels, every dynamic value through `shown`, then
/// the exact argv (the prompt element in double quotes, wrapped at `/` and spaces)
/// and one consequence row. `inner_w` is the popup's inner width.
pub(crate) fn settle_body(plan: &SettlePlan, inner_w: u16) -> Vec<Line<'static>> {
    const LABEL_W: usize = 11;
    let inner_w = usize::from(inner_w);
    let target = &plan.draft.target;
    let field = |label: &str, value: String| -> Vec<Line<'static>> {
        wrap_at_separators(&value, inner_w.saturating_sub(LABEL_W))
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
    };
    let refs = owner_settle::target_text(target);
    let shown_target = match target.scope {
        SettleScope::Phase => format!("phase {}: {refs}", target.phase()),
        SettleScope::Check(_) => refs,
    };
    let mut body: Vec<Line<'static>> = Vec::new();
    body.extend(field("Project", shown(&target.alias)));
    body.extend(field("Target", shown(&shown_target)));
    let outcome = match &plan.choice.move_to {
        Some(to) => {
            let name = to
                .name
                .as_deref()
                .map(|name| format!(" \u{201c}{name}\u{201d}"))
                .unwrap_or_default();
            format!("{} {}{name}", plan.choice.outcome.label(), to.number)
        }
        None => plan.choice.outcome.label().to_owned(),
    };
    body.extend(field("Outcome", shown(&outcome)));
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
    // D-06: a phase neither the roadmap nor the backlog lists is accepted, with this
    // warning above the consequence row.
    if let Some(to) = plan.choice.move_to.as_ref().filter(|to| !to.known) {
        let warning = format!(
            "Phase {} is not in the roadmap or backlog: GSD may create or reject it.",
            shown(&to.number)
        );
        for part in wrap_at_separators(&warning, inner_w) {
            body.push(Line::from(Span::styled(
                part,
                Style::default().fg(Color::Yellow),
            )));
        }
        body.push(Line::raw(""));
    }
    let consequence = if plan.draft.live_session.is_some() {
        "Claude already runs here: Yes switches to it and copies the command to paste; no second session."
    } else {
        "Yes starts a new session; GSD records it. This app writes nothing."
    };
    for part in wrap_at_separators(consequence, inner_w) {
        body.push(Line::raw(part));
    }
    body
}

crate::ui::screens::adjudicate_screen!(
    SettleConfirmScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws the registered alias, the phase token, the check file and check names, \
     the workstream and the owner's reason (all read out of `.planning/` or typed) \
     in the whole argv, each element through `shown`, over the inbox or the detail \
     view as its background. Fixture states: one popup with hostile values at the \
     sizes the guard renders.",
);

impl Screen for SettleConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        ctx.needs_redraw = true;
        match self.popup.on_key(code) {
            ConfirmOutcome::Confirm => self.confirm(ctx),
            ConfirmOutcome::Cancel => ScreenAction::Pop,
            ConfirmOutcome::Pending => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        render_host(&self.host, frame, area, ctx);
        confirm_popup::dim_background(frame, area);
        self.popup.record(confirm_popup::render_confirm_popup(
            frame,
            area,
            "Settle via /gsd-verify-work",
            settle_body(&self.plan, confirm_popup::body_room(area).0),
            Color::Yellow,
            self.popup.focus,
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

    /// 39-02 SC1 tracer: in the detail Owner block `Enter`, `j`, `x`, `w`, a reason,
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
        stack.key(&mut ctx, KeyCode::Enter);
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

    #[test]
    fn the_owner_block_row_flow_settles_the_phase() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let (key, _) = uat_row_and_first_item(&ctx);
        let mut stack = Stack::detail(&mut ctx, ALIAS, &key);
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
        stack.key(&mut ctx, KeyCode::Enter);
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
        // `k` back to the row: `x` is the phase.
        stack.key(&mut ctx, KeyCode::Char('k'));
        stack.key(&mut ctx, KeyCode::Char('k'));
        assert_eq!(ctx.inbox_view.item_cursor, None);
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('p'));
        stack.key(&mut ctx, KeyCode::Enter);
        stack.key(&mut ctx, KeyCode::Char('y'));
        let launches = take_settle_launches();
        assert_eq!(launches[0].draft.target.scope, SettleScope::Phase);
        assert_eq!(tree(&root), before);
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
        for _ in 0..ids.len() {
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
        stack.key(&mut ctx, KeyCode::Enter);
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

    #[test]
    fn the_move_popup_fits_for_a_check_at_80x24() {
        let (plan, ctx, _dir) = move_plan("heuristic-labels", "pricing-app", Some("01-UAT.md#4"));
        let (inner_w, room) = confirm_popup::body_room(Rect::new(0, 0, 80, 24));
        let rows = confirm_popup::wrapped_rows(&settle_body(&plan, inner_w), inner_w);
        assert!(rows <= room, "{rows} rows > {room}");
        let screen = SettleConfirmScreen::new(AckHost::Inbox, plan);
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| screen.render(f, f.area(), &ctx)).unwrap();
        assert!(!screen.popup.clipped(), "{}", dump(&terminal, 80, 24));
    }

    #[test]
    fn the_move_popup_fits_for_a_phase_at_100x30() {
        let (plan, ctx, _dir) = move_plan("heuristic-labels", "pricing-app", None);
        let (inner_w, room) = confirm_popup::body_room(Rect::new(0, 0, 100, 30));
        let rows = confirm_popup::wrapped_rows(&settle_body(&plan, inner_w), inner_w);
        assert!(rows <= room, "{rows} rows > {room}");
        let screen = SettleConfirmScreen::new(AckHost::Inbox, plan.clone());
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| screen.render(f, f.area(), &ctx)).unwrap();
        assert!(!screen.popup.clipped(), "{}", dump(&terminal, 100, 30));
        // The 80x24 count of the same plan, measured for the SUMMARY.
        let (w80, room80) = confirm_popup::body_room(Rect::new(0, 0, 80, 24));
        let rows80 = confirm_popup::wrapped_rows(&settle_body(&plan, w80), w80);
        eprintln!("phase-level Move popup at 80x24: {rows80} rows of {room80}");
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

    #[test]
    fn the_popup_fits_80x24_with_the_exact_argv() {
        let (_dir, root) = temp_fixture("uat-pending");
        let ctx = fixture_ctx(&root, ALIAS);
        let plan = tracer_plan(&ctx, Outcome::WontDo, "no device here");
        let argv = settle_mux_argv(&plan).unwrap();
        let screen = SettleConfirmScreen::new(AckHost::Inbox, plan);
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| screen.render(f, f.area(), &ctx)).unwrap();
        let text = dump(&terminal, 80, 24);
        assert!(!screen.popup.clipped(), "{text}");
        assert!(!text.contains("Cut off"), "{text}");
        for want in [
            "Settle via /gsd-verify-work",
            "Project    alder-api",
            "Target     phase 01: tests 2, 3 in 01-UAT.md",
            "Outcome    Won't do",
            "Yes starts a new session; GSD records it. This app writes nothing.",
            "[ Yes ]",
            "[ No ]",
        ] {
            assert!(text.contains(want), "{want:?} missing from\n{text}");
        }
        assert_eq!(screen.popup.focus, confirm_popup::ConfirmFocus::No);
        // The exact argv: every element on screen, in order, the prompt quoted.
        let elements = argv.as_slice();
        let last = elements.len() - 1;
        let expected: String = elements
            .iter()
            .enumerate()
            .map(|(i, e)| {
                if i == last {
                    format!("\"{e}\"")
                } else {
                    e.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect();
        assert!(
            squashed(&text).contains(&expected),
            "the argv is not drawn whole in\n{text}"
        );
    }

    /// The tracer plans fit the 19 body rows an 80x24 terminal gives the popup.
    #[test]
    fn the_tracer_popups_fit_the_body_room_at_80x24() {
        let (_dir, root) = temp_fixture("uat-pending");
        let ctx = fixture_ctx(&root, ALIAS);
        let (inner_w, room) = confirm_popup::body_room(Rect::new(0, 0, 80, 24));
        for (outcome, reason) in [
            (Outcome::WontDo, "no device here"),
            (Outcome::DeferPermanently, ""),
        ] {
            let body = settle_body(&tracer_plan(&ctx, outcome, reason), inner_w);
            let rows = confirm_popup::wrapped_rows(&body, inner_w);
            assert!(rows <= room, "{outcome:?}: {rows} rows > {room}");
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
            chooser.contains(
                "Settle phase 01: [w] won't do  [p] defer permanently  [m] move  [Esc] cancel"
            ),
            "{chooser}"
        );
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
        assert!(
            text.contains("Claude already runs here: Yes switches to it and copies the command to")
                && text.contains("paste; no second session."),
            "{text}"
        );
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
    fn the_popup_clips_at_60x20_and_cannot_confirm() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut screen = SettleConfirmScreen::new(
            AckHost::Inbox,
            tracer_plan(&ctx, Outcome::WontDo, "no device here"),
        );
        let mut terminal = Terminal::new(TestBackend::new(60, 20)).unwrap();
        terminal.draw(|f| screen.render(f, f.area(), &ctx)).unwrap();
        let text = dump(&terminal, 60, 20);
        assert!(screen.popup.clipped(), "{text}");
        assert!(
            text.contains(confirm_popup::CLIPPED_HINT.trim_end()),
            "{text}"
        );
        assert!(matches!(
            screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx),
            ScreenAction::None
        ));
        screen.handle_key(KeyCode::Left, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(
            screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx),
            ScreenAction::None
        ));
        assert!(take_settle_launches().is_empty());
        for code in [KeyCode::Char('n'), KeyCode::Esc] {
            assert!(matches!(
                screen.handle_key(code, KeyModifiers::NONE, &mut ctx),
                ScreenAction::Pop
            ));
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
        stack.key(&mut ctx, KeyCode::Enter);
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
}
