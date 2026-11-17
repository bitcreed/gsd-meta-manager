//! The three screens of settle (`x`, 39): the one-row chooser, the reason field
//! and the default-No confirm popup that shows the exact argv.
//!
//! **No screen here writes a file or starts a process.** Yes hands the plan to
//! `owner_settle::launch_settle` after `settle_recheck` found the row unchanged;
//! No, Esc and a clipped popup launch nothing.
//!
//! The chooser is replaced by the reason field and the reason field by the popup
//! (`ScreenAction::Replace`, never `Push`: WR-03), so Esc and No return straight
//! to the host.

use super::ack_confirm::{wrap_at_separators, AckHost};
use super::confirm_popup::{self, ConfirmOutcome, ConfirmPopup};
use super::detail::{fit_spans, settle_mux_argv, shown};
use super::driver_steer::tail_cells;
use super::inbox::InboxScreen;
use super::owner_settle::{
    self, clean, Outcome, SettleChoice, SettleDraft, SettlePlan, SettleScope, REASON_MAX_CHARS,
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
/// [Esc] cancel`. A key replaces it with the reason field; Esc cancels.
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
        spans.extend(key("[Esc]", " cancel"));
        let line = fit_spans(spans, usize::from(chunks[1].width));
        frame.render_widget(Paragraph::new(line), chunks[1]);
    }

    fn name(&self) -> &str {
        "settle_chooser"
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
    buffer: String,
    error: Option<String>,
}

impl SettleReasonScreen {
    pub(crate) fn new(host: AckHost, draft: SettleDraft, outcome: Outcome) -> Self {
        Self {
            host,
            draft,
            outcome,
            buffer: String::new(),
            error: None,
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
        let prompt = format!(
            "  {} {}: reason{optional}> ",
            self.outcome.label(),
            shown(&self.draft.target.short())
        );
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

    /// Yes: the status line says why nothing launched, or what launched.
    fn confirm(&self, ctx: &mut AppContext) {
        let message = match owner_settle::settle_recheck(ctx, &self.plan) {
            Ok(fresh) => owner_settle::launch_settle(&fresh),
            Err(message) => message,
        };
        ctx.status_message = Some((message, std::time::Instant::now()));
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
    body.extend(field("Outcome", plan.choice.outcome.label().to_owned()));
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
    body.push(Line::raw(if plan.draft.live_session.is_some() {
        "Claude already runs here: Yes switches to it, no second session."
    } else {
        "Yes starts a new session; GSD records it. This app writes nothing."
    }));
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
            ConfirmOutcome::Confirm => {
                self.confirm(ctx);
                ScreenAction::Pop
            }
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
    pub(crate) struct Stack(Vec<Box<dyn Screen>>);

    impl Stack {
        pub(crate) fn inbox() -> Self {
            Stack(vec![Box::new(InboxScreen::new())])
        }

        pub(crate) fn key_with(
            &mut self,
            ctx: &mut AppContext,
            code: KeyCode,
            modifiers: KeyModifiers,
        ) -> bool {
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
            chooser.contains("Settle phase 01: [w] won't do  [p] defer permanently  [Esc] cancel"),
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
            text.contains("Claude already runs here: Yes switches to it, no second session."),
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
}
