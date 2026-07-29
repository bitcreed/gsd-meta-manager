//! **The only driver surface Phase 17 ships** (D-25, D-26).
//!
//! Read the minimalism as a fence, not as an oversight. Everything a driver
//! *looks* like belongs to **Phase 18**, and the ROADMAP says so twice: the
//! Driver tab, the live output stream, the ring-buffered output, the dashboard
//! driver badges, the three-state injection UI, and the rich opt-in disclosure
//! flow that lists every file entering the prompt and makes the user type the
//! project name. D-26 is explicit that a bare toggle with a clear confirmation
//! is sufficient *here* and must not block on a richer flow that has no screen
//! to live in yet.
//!
//! What Phase 17 needs is only enough surface for a human to reach the phase's
//! own success criteria: start a run, stop a live one, toggle the opt-in. That
//! is three keys on the dashboard and this one confirmation, modelled file for
//! file on [`super::delete_confirm`] — the same `[y/n]` one-line footer, the
//! same `Color::Red` for a destructive direction, the same free functions doing
//! the work and reporting through `ctx.error_message` / `ctx.status_message`,
//! and the same `ScreenAction::Pop` on both arms. No new widget, no new tab, no
//! second screen.
//!
//! Driver state is read from the **sibling maps on `AppContext`** — never from
//! `ProjectState`, whose derived `PartialEq` drives the v1.4 unchanged-state
//! status suppression that a multi-hour run would otherwise defeat for its whole
//! duration (D-25, ARCHITECTURE AP1).

use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::config::save_config;
use crate::registry;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// The single GSD command a Phase 17 start issues.
///
/// **Hardcoded on purpose, and the hardcoding is a scope fence rather than a
/// stub.** There is no command picker because the picker is **Phase 18's**, and
/// there is no computed sequence because the D-R-P-E-V decision router is
/// **Phase 20's** (D-22 says the honest "sequence" for this phase is the single
/// `--command` argument, and the driver's own dry-run output says so too). A
/// read-only progress command is the right default for the one command a user
/// can reach from a keypress before either of those phases exists.
pub const DEFAULT_DRIVE_COMMAND: &str = "/gsd-progress";

/// Which of the three driver actions this confirmation is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverAction {
    /// Start a run on the selected project.
    Start,
    /// Stop the live run on the selected project.
    Stop,
    /// Toggle the driver opt-in record for the selected project.
    ToggleOptIn,
}

/// One confirmation screen serving all three driver actions.
pub struct DriverConfirmScreen {
    alias: String,
    action: DriverAction,
}

impl DriverConfirmScreen {
    pub fn new(alias: String, action: DriverAction) -> Self {
        Self { alias, action }
    }
}

/// The prompt for one action, stating the **consequence** rather than the
/// mechanism, and naming the alias.
///
/// `opted_in` is the project's opt-in state *right now*, which is what decides
/// the direction of a toggle. Free function rather than a method so the whole
/// prompt matrix is testable without constructing a screen.
///
/// The disabling direction deliberately says no **new** run can start. It does
/// not claim to stop a live one, because the code does not stop one — and a
/// safety claim the implementation does not back is worse than no claim at all
/// (T-17-48).
fn prompt_text(alias: &str, action: DriverAction, opted_in: bool) -> String {
    match action {
        DriverAction::Start => format!(
            "Drive \"{alias}\"? An autonomous agent will run {DEFAULT_DRIVE_COMMAND} in that \
             project with full autonomy, including git operations. [y/n]"
        ),
        DriverAction::Stop => format!(
            "Stop the run on \"{alias}\"? The whole process tree is terminated, including the \
             agent's own grandchildren. [y/n]"
        ),
        DriverAction::ToggleOptIn if opted_in => format!(
            "Withdraw the driver opt-in for \"{alias}\"? No NEW run can start afterwards; a run \
             already live keeps going. [y/n]"
        ),
        DriverAction::ToggleOptIn => format!(
            "Allow \"{alias}\" to be driven? This is the record the spawn gate checks before any \
             agent is started against it. [y/n]"
        ),
    }
}

/// `Color::Red` for the two directions that take something away — stopping a
/// run terminates a process tree, and withdrawing an opt-in removes a
/// capability. The default foreground for starting a run and for granting one,
/// following `delete_confirm.rs`'s use of red for a destructive action.
fn prompt_color(action: DriverAction, opted_in: bool) -> Color {
    match action {
        DriverAction::Stop => Color::Red,
        DriverAction::ToggleOptIn if opted_in => Color::Red,
        _ => Color::Reset,
    }
}

impl Screen for DriverConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Char('y') => {
                match self.action {
                    DriverAction::Start => do_start_run(ctx, &self.alias),
                    DriverAction::Stop => do_stop_run(ctx, &self.alias),
                    DriverAction::ToggleOptIn => do_toggle_opt_in(ctx, &self.alias),
                }
                ScreenAction::Pop
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let chunks = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);

        use ratatui::widgets::{Block, Borders};
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" GSD Manager ");
        frame.render_widget(block, chunks[0]);

        // The opt-in state comes from the registry, which is the same read the
        // toggle itself performs — so the prompt cannot describe a direction
        // other than the one `y` will take.
        let opted_in = registry::is_opted_in(&ctx.config, &self.alias);
        let prompt = prompt_text(&self.alias, self.action, opted_in);
        let line = Line::from(Span::styled(
            prompt,
            Style::default().fg(prompt_color(self.action, opted_in)),
        ));
        frame.render_widget(Paragraph::new(line), chunks[1]);
    }

    fn name(&self) -> &str {
        "driver_confirm"
    }
}

/// Ask the app to start a run, or refuse **visibly**.
///
/// **Two layers, and both are deliberate.** This screen checks
/// `registry::is_opted_in` and sets `ctx.error_message` naming the alias and the
/// `o` key, so a user who presses `r` on a project that has not opted in gets an
/// immediate answer instead of a run that silently never appears. And if this
/// check were ever deleted, the child process would still refuse, because the
/// real gate lives in the driver: `DrivableProject::from_registry` is the only
/// production constructor and `driver::drive` is its only caller, which is what
/// makes CTRL-03's "never" literally true for a hand-typed
/// `gsd-meta-manager drive foo` as well (D-16).
///
/// So: **the check here is an affordance, not the enforcement.** Nobody should
/// later "simplify" by treating it as one, in either direction — deleting it
/// costs the user the message, and treating it as the gate would put the gate
/// somewhere a `tests/spawn_seam_guard.rs`-style audit cannot see it.
///
/// The work itself is routed as an [`Action`] on `ctx.event_tx` rather than
/// called directly, because the seam lives on `App` and a `Screen` only ever
/// receives `&mut AppContext`. That is the route plan 17-06 already built for
/// [`Action::DriverStopRequested`], and this adds its sibling rather than a
/// second mechanism.
fn do_start_run(ctx: &mut AppContext, alias: &str) {
    if !registry::is_opted_in(&ctx.config, alias) {
        ctx.error_message = Some(format!(
            "\"{alias}\" has not opted in to being driven — press `o` on the dashboard to opt it in"
        ));
        ctx.needs_redraw = true;
        return;
    }

    dispatch(
        ctx,
        Action::DriverStartRequested {
            alias: alias.to_string(),
            command: DEFAULT_DRIVE_COMMAND.to_string(),
            // `None` until plan 18-07 gives the user a screen to type a goal
            // into. It is deliberately not a fabricated placeholder: OBS-03
            // renders an absent goal as `(none given)`, and inventing a summary
            // here is exactly what D-13 forbids.
            goal: None,
        },
        alias,
    );
}

/// Ask the app to stop the observed run on `alias`.
///
/// The message carries **only the alias**: the pid and the process group are
/// read from the reconciliation scan at the moment the stop is dispatched, never
/// captured at key-press time, because a pgid that has gone stale is a pgid that
/// may already name a stranger's process group (see [`Action::DriverStopRequested`]).
///
/// A refusal for an alias with nothing to stop belongs to `App::stop_driver_run`
/// and is deliberately not duplicated here — it reports through
/// `ctx.error_message` from there.
fn do_stop_run(ctx: &mut AppContext, alias: &str) {
    dispatch(
        ctx,
        Action::DriverStopRequested {
            alias: alias.to_string(),
        },
        alias,
    );
}

/// Send one action, reporting visibly if there is nowhere to send it.
///
/// The same guard `App::stop_driver_run` and `App::schedule_journal_tail` use:
/// without a channel there is no way to reach the seam, and an action the user
/// asked for that quietly went nowhere is indistinguishable from a hang.
fn dispatch(ctx: &mut AppContext, action: Action, alias: &str) {
    match &ctx.event_tx {
        Some(tx) => {
            let _ = tx.send(action);
        }
        None => {
            ctx.error_message = Some(format!(
                "Cannot reach the driver seam for \"{alias}\" — the event channel is closed"
            ));
        }
    }
    ctx.needs_redraw = true;
}

/// Flip the opt-in record for `alias` and persist it.
///
/// `registry::record_opt_in` and `registry::clear_opt_in` deliberately do not
/// persist — every function in that module leaves saving to its caller — so this
/// calls `save_config` itself, exactly as `delete_confirm.rs` does.
///
/// **A failed save is rolled back in memory**, which `delete_confirm.rs` does
/// not need to do and this does. The gate that matters reads `config.json` *from
/// disk*, in a different process; an in-memory opt-in that never reached the
/// file would make the TUI report a project as drivable while every run against
/// it is refused by the driver. Reverting keeps the two in agreement, so the
/// error message is the only thing the user has to act on.
fn do_toggle_opt_in(ctx: &mut AppContext, alias: &str) {
    let was_opted_in = registry::is_opted_in(&ctx.config, alias);

    let applied = if was_opted_in {
        registry::clear_opt_in(&mut ctx.config, alias)
    } else {
        registry::record_opt_in(&mut ctx.config, alias)
    };
    if let Err(e) = applied {
        ctx.error_message = Some(e.to_string());
        ctx.needs_redraw = true;
        return;
    }

    if let Err(e) = save_config(&ctx.config, &ctx.config_path) {
        let reverted = if was_opted_in {
            registry::record_opt_in(&mut ctx.config, alias)
        } else {
            registry::clear_opt_in(&mut ctx.config, alias)
        };
        let note = if reverted.is_err() {
            " (and the in-memory registry could not be restored)"
        } else {
            ""
        };
        ctx.error_message = Some(format!("Failed to save config: {e}{note}"));
        ctx.needs_redraw = true;
        return;
    }

    let message = if was_opted_in {
        format!("\"{alias}\" can no longer be driven — no NEW run can start")
    } else {
        format!("\"{alias}\" can now be driven")
    };
    ctx.status_message = Some((message, std::time::Instant::now()));
    ctx.needs_redraw = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, RegisteredProject};
    use std::path::Path;
    use tokio::sync::mpsc::UnboundedReceiver;

    const ALIAS: &str = "proj";

    /// An `AppContext` with one registered project, a live event channel, and a
    /// config path inside `root` so `save_config` is a real write.
    fn ctx_with_project(root: &Path) -> (AppContext, UnboundedReceiver<Action>) {
        use crate::change_tracker::ChangeTracker;
        use ratatui::widgets::TableState;
        use std::collections::HashMap;

        let mut config = Config::new();
        config.projects.insert(
            ALIAS.to_string(),
            RegisteredProject {
                path: root.to_path_buf(),
                added: "2026-07-29".to_string(),
                driver_opt_in: None,
                extra: Default::default(),
            },
        );

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let ctx = AppContext {
            config,
            config_path: root.join("config.json"),
            project_states: HashMap::new(),
            table_state: TableState::default(),
            filtered_aliases: Vec::new(),
            filter_text: String::new(),
            change_tracker: ChangeTracker::new(),
            detail_sub_view_per_project: HashMap::new(),
            view_cache: HashMap::new(),
            status_message: None,
            error_message: None,
            event_tx: Some(tx),
            exec_tx: None,
            run_states: HashMap::new(),
            reparse_dispatches: 0,
            journal_cursors: HashMap::new(),
            observed_runs: HashMap::new(),
            session_spawned_runs: std::collections::HashSet::new(),
            driver_output: HashMap::new(),
            sort_mode: crate::ui::screens::SortMode::default(),
            watcher: None,
            last_refresh: HashMap::new(),
            detail_scroll_offset: 0,
            suggestion_index: 0,
            input_buffer: String::new(),
            needs_redraw: false,
            active_sessions: Vec::new(),
            archive_cache: HashMap::new(),
        };
        (ctx, rx)
    }

    /// CTRL-03 at the affordance layer.
    ///
    /// The load-bearing half is the **second** assertion. A screen that set the
    /// message and dispatched anyway would pass a test that only checked
    /// `error_message` — and the run would start regardless of the message the
    /// user was shown. So the absence of a sent `Action` is asserted directly.
    #[test]
    fn starting_a_run_on_a_project_that_has_not_opted_in_sets_an_error_and_spawns_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        assert!(
            !registry::is_opted_in(&ctx.config, ALIAS),
            "the fixture must start NOT opted in, or this test is vacuous"
        );

        let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::Start);
        let action = screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::Pop));

        let refusal = ctx
            .error_message
            .as_deref()
            .expect("the refusal must be visible, not silent");
        assert!(refusal.contains(ALIAS), "got: {refusal}");
        assert!(
            refusal.contains('o'),
            "the refusal must name the key that fixes it, got: {refusal}"
        );

        assert!(
            rx.try_recv().is_err(),
            "a refused start must dispatch NOTHING — a message plus a dispatch is a \
             run that starts anyway while telling the user it did not"
        );

        // The control arm: once opted in, the same keypress DOES dispatch, so
        // the assertion above is not passing because nothing ever dispatches.
        registry::record_opt_in(&mut ctx.config, ALIAS).expect("opt in");
        let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::Start);
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);
        let sent = rx.try_recv().expect("an opted-in start must dispatch");
        let Action::DriverStartRequested {
            alias,
            command,
            goal,
        } = sent
        else {
            panic!("expected DriverStartRequested, got {sent:?}");
        };
        assert_eq!(alias, ALIAS);
        assert_eq!(command, DEFAULT_DRIVE_COMMAND);
        assert_eq!(
            goal, None,
            "this screen has no goal field yet (18-07 adds one), and an absent \
             goal must travel as absent rather than as an invented summary"
        );
    }

    #[test]
    fn the_toggle_records_and_then_clears_an_opt_in_record() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());

        let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::ToggleOptIn);
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);
        assert!(
            registry::is_opted_in(&ctx.config, ALIAS),
            "the first toggle must record an opt-in"
        );
        assert!(ctx.error_message.is_none());

        // It persisted, which is what the gate in the child process reads.
        let saved = crate::config::load_config(&ctx.config_path).expect("the toggle must save");
        assert!(
            saved.projects[ALIAS].driver_opt_in.is_some(),
            "an opt-in that never reached disk is invisible to the driver, which is \
             a different process and reads config.json"
        );

        let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::ToggleOptIn);
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);
        assert!(
            !registry::is_opted_in(&ctx.config, ALIAS),
            "the second toggle must clear it"
        );
        let saved = crate::config::load_config(&ctx.config_path).expect("load");
        assert!(saved.projects[ALIAS].driver_opt_in.is_none());
    }

    #[test]
    fn every_prompt_names_the_alias_and_offers_the_same_two_keys() {
        let cases = [
            (DriverAction::Start, false),
            (DriverAction::Start, true),
            (DriverAction::Stop, false),
            (DriverAction::Stop, true),
            (DriverAction::ToggleOptIn, false),
            (DriverAction::ToggleOptIn, true),
        ];

        for (action, opted_in) in cases {
            let prompt = prompt_text(ALIAS, action, opted_in);
            assert!(
                prompt.contains(ALIAS),
                "{action:?}/{opted_in} must name the alias, got: {prompt}"
            );
            assert!(
                prompt.ends_with("[y/n]"),
                "{action:?}/{opted_in} must offer the same two keys as every other \
                 confirmation in the tree, got: {prompt}"
            );
        }

        // The two directions that take something away are red; the two that
        // grant something are not. A prompt that warned in the wrong direction
        // would be worse than an unstyled one.
        assert_eq!(prompt_color(DriverAction::Stop, false), Color::Red);
        assert_eq!(prompt_color(DriverAction::ToggleOptIn, true), Color::Red);
        assert_eq!(prompt_color(DriverAction::ToggleOptIn, false), Color::Reset);
        assert_eq!(prompt_color(DriverAction::Start, true), Color::Reset);

        // The withdraw prompt says NEW, because disabling does not stop a live
        // run and the code does not pretend it does (T-17-48).
        let withdraw = prompt_text(ALIAS, DriverAction::ToggleOptIn, true);
        assert!(
            withdraw.contains("NEW"),
            "the withdraw prompt must not claim to stop a live run, got: {withdraw}"
        );
    }

    #[test]
    fn declining_dispatches_nothing_and_pops() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        registry::record_opt_in(&mut ctx.config, ALIAS).expect("opt in");

        for key in [KeyCode::Char('n'), KeyCode::Esc] {
            let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::Start);
            let action = screen.handle_key(key, KeyModifiers::NONE, &mut ctx);
            assert!(matches!(action, ScreenAction::Pop), "{key:?} must pop");
            assert!(
                rx.try_recv().is_err(),
                "{key:?} must dispatch nothing — a confirmation that acts on a \
                 decline is not a confirmation"
            );
        }

        // Any other key is consumed and does nothing at all.
        let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::Start);
        let action = screen.handle_key(KeyCode::Char('z'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::None));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn a_stop_dispatches_the_alias_and_nothing_else() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());

        let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::Stop);
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);

        let sent = rx.try_recv().expect("a stop must dispatch");
        let Action::DriverStopRequested { alias } = sent else {
            panic!("expected DriverStopRequested, got {sent:?}");
        };
        assert_eq!(alias, ALIAS);

        // A stop is NOT gated on the opt-in record: withdrawing an opt-in stops
        // no live run (T-17-48), so a run started before a withdrawal must
        // still be stoppable afterwards.
        assert!(!registry::is_opted_in(&ctx.config, ALIAS));
    }
}
