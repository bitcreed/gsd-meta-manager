//! The Driver tab's steering inputs (Phase 30, plan 30-04): the control chooser, and
//! the copy the App shows after a steering write.
//!
//! **A key handler here never touches the filesystem** (D-25). Each screen collects
//! one decision and dispatches an [`Action`] on `ctx.event_tx`; the App runs the
//! blocking write on `spawn_blocking` ([`crate::driver::steer::steer_run`], a
//! `write_body` closure under `DIRECTION.lock`) and answers with
//! [`Action::DriverSteered`]. The file contains no file-write call at all, and a test
//! pins that.
//!
//! **No copy here says a control or an answer took effect.** The driver has not
//! echoed it in the frontmatter yet, so "written to DIRECTION.md" is the strongest
//! thing true (D-28): never `paused`, `stopped` or `sent`, and no spinner.
//!
//! Every value drawn or interpolated that came from a file goes through
//! [`crate::text::render_for_terminal`].

use super::confirm_popup::{
    click_as_key, dim_background, render_confirm_popup, strip_key_hint, ConfirmOutcome,
    ConfirmPopup,
};
use super::driver_confirm::dispatch;
use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::driver::steer::{AnswerRule, SteerOp, SteerOutcome, DIRECTION_ANSWER_MAX_CHARS};
use crate::ui::mouse::MouseInput;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// What every steering key says on a build that cannot write `DIRECTION.md` safely
/// (D-14): the writer is `renameat2(RENAME_EXCHANGE)`, which is Linux-only.
pub const STEERING_NEEDS_LINUX: &str = "read-only: steering needs Linux";

/// Whether this build can steer. A constant, not a probe: the write primitive exists
/// on Linux and nowhere else.
pub const fn can_steer() -> bool {
    cfg!(target_os = "linux")
}

fn bold() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

fn muted() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// The appended sentence when the write went ahead without `DIRECTION.lock` (27 D-20,
/// orchestrator decision 3 `[inferred — audit]`).
pub const LOCK_TIMEOUT_NOTE: &str =
    " Written without DIRECTION.lock (another writer held it); the exchange check kept every save.";

/// The park rule that needs a human answer; any other rule is not lifted by an
/// answer alone (the DIRECTION template's stop-rules block, D-16).
pub const NEEDS_HUMAN_RULE: &str = "needs-human";

/// The second row of the answer input, and the tail of the written message, when the
/// item is parked by a rule an answer does not lift (OQ-R1, D-16): `None` for
/// `needs-human` and for an unknown rule, because showing it would claim a rule nobody
/// recorded (`[inferred — audit]`).
pub fn park_warning(item: &str, rule: Option<&str>) -> Option<String> {
    let rule = rule.filter(|rule| *rule != NEEDS_HUMAN_RULE)?;
    Some(format!(
        "{} is parked by {}: an answer alone will not resume it. Raise its budget with [e] or relaunch with higher caps with [R].",
        crate::text::render_for_terminal(item),
        crate::text::render_for_terminal(rule),
    ))
}

/// The status line (or error line) for a finished steering write, and whether it is
/// an error.
///
/// `live` is whether the run's lock was held when the result arrived: a write to a run
/// that is not running applies at its next launch, not before its next dispatch.
/// `park_rule` is the rule the LOG recorded for the answered item, when it is known.
/// Nothing here claims the driver has applied anything (D-28).
pub fn steered_message(
    op: &SteerOp,
    outcome: &SteerOutcome,
    live: bool,
    park_rule: Option<&str>,
) -> (String, bool) {
    let render = |text: &str| crate::text::render_for_terminal(text).to_string();
    let with_note = |mut text: String, lock_timed_out: bool| {
        if lock_timed_out {
            text.push_str(LOCK_TIMEOUT_NOTE);
        }
        (text, false)
    };
    match (op, outcome) {
        (SteerOp::SetControl(word), SteerOutcome::Written { lock_timed_out }) => {
            let word = render(word);
            let text = if live {
                format!(
                    "control: {word} written to DIRECTION.md. The driver applies it before its next dispatch."
                )
            } else {
                format!("control: {word} written to DIRECTION.md. It applies at the next launch.")
            };
            with_note(text, *lock_timed_out)
        }
        (SteerOp::SetControl(word), SteerOutcome::Unchanged) => (
            format!(
                "control is already {} in DIRECTION.md. Nothing was written.",
                render(word)
            ),
            false,
        ),
        (SteerOp::Answer { item, .. }, SteerOutcome::Written { lock_timed_out }) => {
            let text = match park_rule.filter(|rule| *rule != NEEDS_HUMAN_RULE) {
                Some(rule) => format!(
                    "Answer written to {}, but it is parked by {}: an answer alone will not resume it. Raise its budget with [e] or relaunch with higher caps with [R].",
                    render(item),
                    render(rule)
                ),
                None => format!(
                    "Answer written to {} and hold set to false. The driver re-reads it before its next dispatch.",
                    render(item)
                ),
            };
            with_note(text, *lock_timed_out)
        }
        (SteerOp::Answer { item, .. }, SteerOutcome::Unchanged) => (
            format!(
                "{} already has that answer and is off hold. Nothing was written.",
                render(item)
            ),
            false,
        ),
        (SteerOp::Unhold { item }, SteerOutcome::Written { lock_timed_out }) => with_note(
            format!(
                "{}: hold set to false. The driver re-reads it before its next dispatch.",
                render(item)
            ),
            *lock_timed_out,
        ),
        (SteerOp::Unhold { item }, SteerOutcome::Unchanged) => (
            format!("{} is already off hold. Nothing was written.", render(item)),
            false,
        ),
        (_, SteerOutcome::Refused(error)) => (
            format!("Nothing was written: {}.", render(&error.to_string())),
            true,
        ),
        (_, SteerOutcome::NotWritten(reason)) => (
            format!(
                "Could not write DIRECTION.md: {}. The file was not changed. Try again.",
                render(reason)
            ),
            true,
        ),
        (_, SteerOutcome::Failed(error)) => (
            format!(
                "Could not write DIRECTION.md: {}. The file was not changed. Try again.",
                render(error)
            ),
            true,
        ),
    }
}

// ── The control chooser (`c`, D-15) ─────────────────────────────────────────

/// `c`: the one-line chooser `control (now: {word})>  [r]un  [p]ause  [s]top
/// [Esc] cancel`.
///
/// `r` and `p` write at once; choosing the word the body already has writes nothing
/// and says so. `s` opens the polite-stop confirmation (the stop is the one choice
/// that ends the run, so it asks). `Esc` cancels.
pub struct ControlChooserScreen {
    alias: String,
    run_id: String,
    /// The control word the body holds now (the cached detail's `body_control`);
    /// `None` when the body has no `- control:` line.
    current: Option<String>,
}

impl ControlChooserScreen {
    pub fn new(alias: String, run_id: String, current: Option<String>) -> Self {
        Self {
            alias,
            run_id,
            current,
        }
    }

    /// `word` is what the body already says.
    fn is_current(&self, word: &str) -> bool {
        self.current
            .as_deref()
            .is_some_and(|current| current.eq_ignore_ascii_case(word))
    }

    /// Choose `word`: nothing is dispatched when the body already says it.
    fn choose(&self, word: &str, ctx: &mut AppContext) -> ScreenAction {
        if self.is_current(word) {
            let (text, _) = steered_message(
                &SteerOp::SetControl(word.to_string()),
                &SteerOutcome::Unchanged,
                false,
                None,
            );
            ctx.status_message = Some((text, std::time::Instant::now()));
            ctx.needs_redraw = true;
            return ScreenAction::Pop;
        }
        dispatch(
            ctx,
            Action::DriverSteerRequested {
                alias: self.alias.clone(),
                run_id: self.run_id.clone(),
                op: SteerOp::SetControl(word.to_string()),
            },
            &self.alias,
        );
        ScreenAction::Pop
    }
}

crate::ui::screens::adjudicate_screen!(
    ControlChooserScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints its body with `DetailScreen::render_main_only` (the registry alias and the \
     run list) and its footer names the control word the run's DIRECTION.md body holds \
     now, which is file content a sibling run or a cloned repository can set. The word \
     is drawn through `render_for_terminal`; the run id is carried for the dispatch and \
     never drawn. Fixture states: the hostile identity as the current word.",
);

impl Screen for ControlChooserScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Char('r') => self.choose("run", ctx),
            KeyCode::Char('p') => self.choose("pause", ctx),
            KeyCode::Char('s') => {
                // The stop is the one choice that ends the run, so it asks. Replacing
                // the chooser (not pushing over it) makes `n` and `Esc` return to the
                // tab. A body that already says `stop` has nothing to confirm.
                if self.is_current("stop") {
                    return self.choose("stop", ctx);
                }
                ctx.needs_redraw = true;
                ScreenAction::Replace(Box::new(PoliteStopScreen::new(
                    self.alias.clone(),
                    self.run_id.clone(),
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
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, chunks[0], ctx);

        let now = match self.current.as_deref() {
            Some(word) => crate::text::render_for_terminal(word).to_string(),
            None => "no control line".to_string(),
        };
        let key = |key: &'static str, rest: &'static str| {
            [Span::styled(key, bold()), Span::styled(rest, muted())]
        };
        let mut spans = vec![Span::styled(format!("  control (now: {now})> "), bold())];
        spans.extend(key("[r]", "un  "));
        spans.extend(key("[p]", "ause  "));
        spans.extend(key("[s]", "top    "));
        spans.push(Span::styled("[Esc] cancel", muted()));
        frame.render_widget(Paragraph::new(Line::from(spans)), chunks[1]);
    }

    fn name(&self) -> &str {
        "driver_control"
    }
}

// ── The polite stop (`c` then `s`, D-22) ────────────────────────────────────

/// `c` then `s`: the confirmation before `control: stop` is written. Polite means
/// the driver finishes its current dispatch, records the report and ends; nothing is
/// killed (that is `x`, with its own Red confirmation). Default focus is No, so a
/// stray Enter is safe.
pub struct PoliteStopScreen {
    alias: String,
    run_id: String,
    popup: ConfirmPopup,
}

impl PoliteStopScreen {
    pub fn new(alias: String, run_id: String) -> Self {
        Self {
            alias,
            run_id,
            popup: ConfirmPopup::default(),
        }
    }

    /// The prompt the popup shows, with the run id escaped.
    fn prompt(&self) -> String {
        format!(
            "Set control: stop for {}? The driver dispatches nothing new, records the in-flight report, then ends. Nothing is killed. [y/n]",
            crate::text::render_for_terminal(&self.run_id)
        )
    }
}

crate::ui::screens::adjudicate_screen!(
    PoliteStopScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints the Driver tab behind a dimmed popup (the registry alias and the run list) \
     and names the run id in its prompt. The run id is a directory name under the \
     project's drivers directory, so a cloned repository or a sibling sandbox chooses \
     it. Fixture states: the hostile identity as the run id.",
);

impl Screen for PoliteStopScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match self.popup.on_key(code) {
            ConfirmOutcome::Confirm => {
                dispatch(
                    ctx,
                    Action::DriverSteerRequested {
                        alias: self.alias.clone(),
                        run_id: self.run_id.clone(),
                        op: SteerOp::SetControl("stop".to_string()),
                    },
                    &self.alias,
                );
                ScreenAction::Pop
            }
            ConfirmOutcome::Cancel => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            ConfirmOutcome::Pending => {
                ctx.needs_redraw = true;
                ScreenAction::None
            }
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, area, ctx);
        dim_background(frame, area);
        let prompt = self.prompt();
        let body = vec![Line::from(Span::styled(
            strip_key_hint(&prompt).to_string(),
            Style::default().fg(Color::Yellow),
        ))];
        self.popup.record(render_confirm_popup(
            frame,
            area,
            "Stop driver run",
            body,
            Color::Yellow,
            self.popup.focus,
        ));
    }

    fn handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction {
        click_as_key(&self.popup, input).map_or(ScreenAction::None, |code| {
            self.handle_key(code, KeyModifiers::NONE, ctx)
        })
    }

    fn name(&self) -> &str {
        "driver_stop_confirm"
    }
}

// ── The answer input (`a`, D-15, D-16) ──────────────────────────────────────

/// What an answer refused for `rule` says, with nothing written.
pub fn answer_rejected_copy(rule: AnswerRule) -> &'static str {
    match rule {
        AnswerRule::CommentMarker => {
            "An answer cannot contain <!-- or --> (they would hide the rest of DIRECTION.md). Nothing was written."
        }
        AnswerRule::Empty | AnswerRule::ControlChar | AnswerRule::TooLong => {
            "An answer must be one line with no control characters. Nothing was written."
        }
    }
}

/// The last cells of `text` that fit `cols` terminal cells, led by `…` when cut, so
/// the caret end of a long answer stays visible.
fn tail_cells(text: &str, cols: usize) -> String {
    if Span::raw(text).width() <= cols {
        return text.to_string();
    }
    let mut kept: Vec<char> = Vec::new();
    let mut used = 1usize; // the leading ellipsis
    let mut buf = [0u8; 4];
    for ch in text.chars().rev() {
        let width = Span::raw(&*ch.encode_utf8(&mut buf)).width();
        if used + width > cols {
            break;
        }
        kept.push(ch);
        used += width;
    }
    kept.reverse();
    format!("\u{2026}{}", kept.into_iter().collect::<String>())
}

/// `a`: the one-line input `Answer {id}> {buffer}_   [Enter] write  [Esc] cancel`,
/// and under it, for an item parked by a rule an answer does not lift, the Yellow
/// OQ-R1 row (D-16).
///
/// No movable cursor and no text-area widget (18-UI-SPEC D-22): typing appends,
/// Backspace removes, characters past [`DIRECTION_ANSWER_MAX_CHARS`] are ignored.
/// Enter on an empty buffer does nothing; Enter on a refused one keeps the buffer and
/// the screen and says why; otherwise the request is dispatched and the screen pops.
pub struct AnswerScreen {
    alias: String,
    run_id: String,
    item: String,
    /// The rule the LOG recorded for the item's last park, when known.
    park_rule: Option<String>,
    buffer: String,
    error: Option<String>,
}

impl AnswerScreen {
    pub fn new(alias: String, run_id: String, item: String, park_rule: Option<String>) -> Self {
        Self {
            alias,
            run_id,
            item,
            park_rule,
            buffer: String::new(),
            error: None,
        }
    }

    fn at_cap(&self) -> bool {
        self.buffer.chars().count() >= DIRECTION_ANSWER_MAX_CHARS
    }
}

crate::ui::screens::adjudicate_screen!(
    AnswerScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints the Driver tab behind its footer rows (the registry alias and the run \
     list), echoes the buffer the operator typed or pasted, and names the item id and \
     the park rule: the id is read from the run's DIRECTION.md body or its \
     frontmatter, and the rule from the run's LOG.md, both files a cloned repository \
     or a sibling sandbox can write. Fixture states: the hostile identity as the item, \
     the rule and the buffer.",
);

impl Screen for AnswerScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Enter => {
                if self.buffer.trim().is_empty() {
                    return ScreenAction::None;
                }
                match crate::driver::steer::validate_answer(&self.buffer) {
                    Err(rule) => {
                        self.error = Some(answer_rejected_copy(rule).to_string());
                        ctx.needs_redraw = true;
                        ScreenAction::None
                    }
                    Ok(text) => {
                        dispatch(
                            ctx,
                            Action::DriverSteerRequested {
                                alias: self.alias.clone(),
                                run_id: self.run_id.clone(),
                                op: SteerOp::Answer {
                                    item: self.item.clone(),
                                    text,
                                },
                            },
                            &self.alias,
                        );
                        ScreenAction::Pop
                    }
                }
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
            format!("   [Enter] write ({DIRECTION_ANSWER_MAX_CHARS} max)  [Esc] cancel")
        } else {
            "   [Enter] write  [Esc] cancel".to_string()
        };
        let prompt = format!(
            "  Answer {}> ",
            crate::text::render_for_terminal(&self.item)
        );
        let room = usize::from(area.width)
            .saturating_sub(Span::raw(prompt.as_str()).width())
            .saturating_sub(1)
            .saturating_sub(Span::raw(hint.as_str()).width());
        let echoed = tail_cells(
            crate::text::render_for_terminal(&self.buffer).as_ref(),
            room,
        );
        let mut rows = vec![Line::from(vec![
            Span::styled(prompt, bold()),
            Span::raw(echoed),
            Span::raw("_"),
            Span::styled(hint, muted()),
        ])];
        if let Some(error) = &self.error {
            rows.push(Line::from(Span::styled(
                format!("    {error}"),
                Style::default().fg(Color::Red),
            )));
        }
        if let Some(note) = park_warning(&self.item, self.park_rule.as_deref()) {
            rows.push(Line::from(Span::styled(
                format!("    {note}"),
                Style::default().fg(Color::Yellow),
            )));
        }
        let height = u16::try_from(rows.len()).unwrap_or(1);
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(height)]).split(area);
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, chunks[0], ctx);
        frame.render_widget(Paragraph::new(rows), chunks[1]);
    }

    fn name(&self) -> &str {
        "driver_answer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::driver_confirm::tests::{ctx_with_project, ALIAS};
    use tokio::sync::mpsc::UnboundedReceiver;

    const RUN: &str = "20261019-run0";

    fn chooser(
        current: Option<&str>,
    ) -> (ControlChooserScreen, AppContext, UnboundedReceiver<Action>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let (ctx, rx) = ctx_with_project(dir.path());
        (
            ControlChooserScreen::new(
                ALIAS.to_string(),
                RUN.to_string(),
                current.map(str::to_string),
            ),
            ctx,
            rx,
        )
    }

    fn press(screen: &mut ControlChooserScreen, ctx: &mut AppContext, key: char) -> ScreenAction {
        screen.handle_key(KeyCode::Char(key), KeyModifiers::NONE, ctx)
    }

    #[test]
    fn launcher_steer_r_and_p_dispatch_a_set_control_and_pop() {
        for (key, word) in [('r', "run"), ('p', "pause")] {
            let (mut screen, mut ctx, mut rx) = chooser(Some("stop"));
            let action = press(&mut screen, &mut ctx, key);
            assert!(matches!(action, ScreenAction::Pop), "{key}");
            let Some(Action::DriverSteerRequested { alias, run_id, op }) = rx.try_recv().ok()
            else {
                panic!("{key} must dispatch a steering request");
            };
            assert_eq!(alias, ALIAS);
            assert_eq!(run_id, RUN);
            assert_eq!(op, SteerOp::SetControl(word.to_string()));
            assert!(rx.try_recv().is_err(), "exactly one request");
        }
    }

    #[test]
    fn launcher_steer_choosing_the_current_word_writes_nothing_and_says_so() {
        let (mut screen, mut ctx, mut rx) = chooser(Some("Pause"));
        let action = press(&mut screen, &mut ctx, 'p');
        assert!(matches!(action, ScreenAction::Pop));
        assert!(rx.try_recv().is_err(), "nothing is dispatched");
        let (text, _) = ctx.status_message.clone().expect("a status message");
        assert_eq!(
            text,
            "control is already pause in DIRECTION.md. Nothing was written."
        );
    }

    #[test]
    fn launcher_steer_escape_pops_and_other_keys_are_inert() {
        let (mut screen, mut ctx, mut rx) = chooser(None);
        for key in ['x', 'a', 'q'] {
            assert!(matches!(
                press(&mut screen, &mut ctx, key),
                ScreenAction::None
            ));
        }
        assert!(matches!(
            screen.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx),
            ScreenAction::Pop
        ));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn launcher_steer_the_copy_never_claims_the_driver_applied_it() {
        let op = SteerOp::SetControl("pause".to_string());
        let written = SteerOutcome::Written {
            lock_timed_out: false,
        };
        let (live, error) = steered_message(&op, &written, true, None);
        assert!(!error);
        assert_eq!(
            live,
            "control: pause written to DIRECTION.md. The driver applies it before its next dispatch."
        );
        let (idle, _) = steered_message(&op, &written, false, None);
        assert_eq!(
            idle,
            "control: pause written to DIRECTION.md. It applies at the next launch."
        );
        for text in [&live, &idle] {
            for word in ["paused", "stopped", "sent"] {
                assert!(!text.contains(word), "{text} says {word}");
            }
        }
    }

    #[test]
    fn launcher_steer_a_lock_timeout_is_surfaced_not_hidden() {
        let op = SteerOp::SetControl("stop".to_string());
        let (text, error) = steered_message(
            &op,
            &SteerOutcome::Written {
                lock_timed_out: true,
            },
            true,
            None,
        );
        assert!(!error);
        assert!(text.contains("Written without DIRECTION.lock"), "{text}");
    }

    #[test]
    fn launcher_steer_failures_name_the_problem_and_that_nothing_changed() {
        let op = SteerOp::SetControl("run".to_string());
        for outcome in [
            SteerOutcome::Failed("DIRECTION.md I/O failed: disk full".to_string()),
            SteerOutcome::NotWritten("the run has no DIRECTION.md".to_string()),
        ] {
            let (text, error) = steered_message(&op, &outcome, true, None);
            assert!(error, "{text}");
            assert!(text.starts_with("Could not write DIRECTION.md: "), "{text}");
            assert!(
                text.ends_with("The file was not changed. Try again."),
                "{text}"
            );
        }
    }

    #[test]
    fn launcher_steer_the_screens_never_write_a_file() {
        let source = include_str!("driver_steer.rs");
        let needle = ["fs::", "write("].concat();
        let create = ["File::", "create("].concat();
        let (code, _) = source
            .split_once("#[cfg(test)]")
            .expect("the file has a test module");
        assert!(!code.contains(&needle) && !code.contains(&create));
    }

    // ── The polite stop and the answer input (Task 2) ───────────────────────

    /// Draw `screen` into a `TestBackend` and join the cells row by row.
    fn draw(screen: &dyn Screen, ctx: &AppContext, width: u16, height: u16) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
        terminal
            .draw(|frame| screen.render(frame, frame.area(), ctx))
            .expect("draw");
        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .map(|cell| cell.symbol().to_string())
                            .unwrap_or_default()
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn answer_screen(rule: Option<&str>) -> (AnswerScreen, AppContext, UnboundedReceiver<Action>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let (ctx, rx) = ctx_with_project(dir.path());
        (
            AnswerScreen::new(
                ALIAS.to_string(),
                RUN.to_string(),
                "I-03".to_string(),
                rule.map(str::to_string),
            ),
            ctx,
            rx,
        )
    }

    fn type_into(screen: &mut AnswerScreen, ctx: &mut AppContext, text: &str) {
        for ch in text.chars() {
            screen.handle_key(KeyCode::Char(ch), KeyModifiers::NONE, ctx);
        }
    }

    fn enter(screen: &mut AnswerScreen, ctx: &mut AppContext) -> ScreenAction {
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, ctx)
    }

    #[test]
    fn launcher_steer_s_in_the_chooser_replaces_itself_with_the_polite_stop() {
        let (mut screen, mut ctx, mut rx) = chooser(Some("run"));
        let action = press(&mut screen, &mut ctx, 's');
        assert!(
            matches!(&action, ScreenAction::Replace(next) if next.name() == "driver_stop_confirm"),
            "the confirmation replaces the chooser, so `n` returns to the tab"
        );
        assert!(rx.try_recv().is_err(), "choosing `s` writes nothing yet");

        // The body already says stop: nothing to confirm, nothing to write.
        let (mut screen, mut ctx, mut rx) = chooser(Some("stop"));
        let action = press(&mut screen, &mut ctx, 's');
        assert!(matches!(action, ScreenAction::Pop));
        assert!(rx.try_recv().is_err());
        let (text, _) = ctx.status_message.clone().expect("a status message");
        assert_eq!(
            text,
            "control is already stop in DIRECTION.md. Nothing was written."
        );
    }

    #[test]
    fn launcher_steer_only_y_on_the_polite_stop_writes_control_stop() {
        let dir = tempfile::tempdir().expect("temp dir");
        for (key, writes) in [
            (KeyCode::Char('y'), true),
            (KeyCode::Char('n'), false),
            (KeyCode::Esc, false),
            // Default focus is No: a stray Enter is safe.
            (KeyCode::Enter, false),
        ] {
            let (mut ctx, mut rx) = ctx_with_project(dir.path());
            let mut screen = PoliteStopScreen::new(ALIAS.to_string(), RUN.to_string());
            let action = screen.handle_key(key, KeyModifiers::NONE, &mut ctx);
            assert!(matches!(action, ScreenAction::Pop), "{key:?}");
            match (writes, rx.try_recv().ok()) {
                (true, Some(Action::DriverSteerRequested { alias, run_id, op })) => {
                    assert_eq!((alias.as_str(), run_id.as_str()), (ALIAS, RUN));
                    assert_eq!(op, SteerOp::SetControl("stop".to_string()));
                }
                (false, None) => {}
                (_, other) => panic!("{key:?}: unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn launcher_steer_the_polite_stop_prompt_names_the_run_and_kills_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (ctx, _rx) = ctx_with_project(dir.path());
        let screen = PoliteStopScreen::new(ALIAS.to_string(), RUN.to_string());
        let text = draw(&screen, &ctx, 100, 30);
        assert!(text.contains("Stop driver run"), "{text}");
        assert!(text.contains(RUN), "{text}");
        assert!(text.contains("Nothing is killed."), "{text}");
        assert!(
            !text.contains("[y/n]"),
            "the buttons carry the hint:\n{text}"
        );
    }

    #[test]
    fn launcher_steer_an_answer_enter_dispatches_the_trimmed_text_and_pops() {
        let (mut screen, mut ctx, mut rx) = answer_screen(Some("needs-human"));
        type_into(&mut screen, &mut ctx, "  use /run/user | ok  ");
        let action = enter(&mut screen, &mut ctx);
        assert!(matches!(action, ScreenAction::Pop));
        let Some(Action::DriverSteerRequested { alias, run_id, op }) = rx.try_recv().ok() else {
            panic!("Enter must dispatch the request");
        };
        assert_eq!((alias.as_str(), run_id.as_str()), (ALIAS, RUN));
        assert_eq!(
            op,
            SteerOp::Answer {
                item: "I-03".to_string(),
                text: "use /run/user | ok".to_string()
            }
        );
    }

    #[test]
    fn launcher_steer_enter_on_an_empty_answer_is_a_no_op_that_keeps_the_screen_open() {
        // Edge E8.
        let (mut screen, mut ctx, mut rx) = answer_screen(None);
        assert!(matches!(enter(&mut screen, &mut ctx), ScreenAction::None));
        type_into(&mut screen, &mut ctx, "   ");
        assert!(matches!(enter(&mut screen, &mut ctx), ScreenAction::None));
        assert!(rx.try_recv().is_err());
        let text = draw(&screen, &ctx, 100, 20);
        assert!(
            !text.contains("Nothing was written"),
            "no error for empty:\n{text}"
        );
    }

    #[test]
    fn launcher_steer_a_refused_answer_keeps_the_buffer_and_the_screen_and_writes_nothing() {
        for (typed, copy) in [
            (
                "a\tb",
                "An answer must be one line with no control characters. Nothing was written.",
            ),
            (
                "a --> b",
                "An answer cannot contain <!-- or --> (they would hide the rest of DIRECTION.md). Nothing was written.",
            ),
        ] {
            let (mut screen, mut ctx, mut rx) = answer_screen(None);
            type_into(&mut screen, &mut ctx, typed);
            let action = enter(&mut screen, &mut ctx);
            assert!(matches!(action, ScreenAction::None), "{typed:?}");
            assert!(rx.try_recv().is_err(), "nothing is dispatched");
            let text = draw(&screen, &ctx, 160, 20);
            assert!(text.contains(copy), "{typed:?}:\n{text}");
            assert!(text.contains("Answer I-03> "), "the buffer is kept:\n{text}");
            // Editing clears the error; the screen is still usable.
            screen.handle_key(KeyCode::Backspace, KeyModifiers::NONE, &mut ctx);
            let text = draw(&screen, &ctx, 160, 20);
            assert!(!text.contains("Nothing was written"), "{text}");
        }
    }

    #[test]
    fn launcher_steer_typing_past_five_hundred_chars_is_ignored_and_the_hint_says_so() {
        let (mut screen, mut ctx, mut rx) = answer_screen(None);
        type_into(&mut screen, &mut ctx, &"x".repeat(499));
        let text = draw(&screen, &ctx, 120, 20);
        assert!(text.contains("[Enter] write  [Esc] cancel"), "{text}");
        assert!(!text.contains("500 max"), "{text}");
        type_into(&mut screen, &mut ctx, "yz");
        let text = draw(&screen, &ctx, 120, 20);
        assert!(text.contains("[Enter] write (500 max)"), "{text}");
        enter(&mut screen, &mut ctx);
        let Some(Action::DriverSteerRequested {
            op: SteerOp::Answer { text, .. },
            ..
        }) = rx.try_recv().ok()
        else {
            panic!("a 500-char answer is dispatched");
        };
        assert_eq!(text.chars().count(), 500);
        assert!(
            text.ends_with('y') && !text.contains('z'),
            "the 501st char was ignored"
        );
    }

    #[test]
    fn launcher_steer_the_oq_r1_row_shows_only_for_a_known_rule_that_is_not_needs_human() {
        let note = |rule: Option<&str>| {
            let (screen, ctx, _rx) = answer_screen(rule);
            draw(&screen, &ctx, 160, 20)
        };
        let row = "I-03 is parked by gap-rounds: an answer alone will not resume it. Raise its budget with [e] or relaunch with higher caps with [R].";
        assert!(note(Some("gap-rounds")).contains(row));
        assert!(!note(Some("needs-human")).contains("an answer alone will not resume it"));
        assert!(
            !note(None).contains("an answer alone will not resume it"),
            "an unknown rule shows no second row"
        );
    }

    #[test]
    fn launcher_steer_the_answer_copy_follows_the_park_rule() {
        let op = SteerOp::Answer {
            item: "I-03".to_string(),
            text: "x".to_string(),
        };
        let written = SteerOutcome::Written {
            lock_timed_out: false,
        };
        assert_eq!(
            steered_message(&op, &written, true, Some("needs-human")).0,
            "Answer written to I-03 and hold set to false. The driver re-reads it before its next dispatch."
        );
        assert_eq!(
            steered_message(&op, &written, true, None).0,
            "Answer written to I-03 and hold set to false. The driver re-reads it before its next dispatch."
        );
        assert_eq!(
            steered_message(&op, &written, true, Some("gap-rounds")).0,
            "Answer written to I-03, but it is parked by gap-rounds: an answer alone will not resume it. Raise its budget with [e] or relaunch with higher caps with [R]."
        );
        let unhold = SteerOp::Unhold {
            item: "I-03".to_string(),
        };
        assert_eq!(
            steered_message(&unhold, &written, true, None).0,
            "I-03: hold set to false. The driver re-reads it before its next dispatch."
        );
        let (text, error) = steered_message(
            &op,
            &SteerOutcome::Refused(crate::driver::steer::SteerError::NoSuchItem(
                "I-03".to_string(),
            )),
            true,
            None,
        );
        assert!(
            error && text.starts_with("Nothing was written: I-03 is not an item"),
            "{text}"
        );
    }

    #[test]
    fn launcher_steer_a_hostile_buffer_and_item_render_escaped() {
        let (mut screen, mut ctx, _rx) = answer_screen(Some("gap-rounds"));
        type_into(&mut screen, &mut ctx, "boom \u{1b}[31m red \u{202e}rtl");
        let text = draw(&screen, &ctx, 160, 20);
        assert!(
            !text.contains('\u{1b}') && !text.contains('\u{202e}'),
            "{text:?}"
        );
    }
}
