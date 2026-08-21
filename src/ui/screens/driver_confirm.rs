//! **The one driver confirmation** — Phase 17's only driver surface, now the
//! last step of Phase 18's start flow (D-25, D-26, D-23).
//!
//! Phase 17 shipped this as a deliberate fence rather than an oversight:
//! everything a driver *looks* like belonged to **Phase 18**. That map is kept
//! accurate here rather than left to go stale, so the entries below are the
//! ones that are still someone else's:
//!
//! | Still future work | Owner |
//! |---|---|
//! | The Driver tab and its live, ring-buffered output stream | 18-09 |
//! | The dashboard driver badges | 18-06 |
//! | The four-state injection **display** | 18-09 |
//! | The rich opt-in disclosure flow that lists every file entering the prompt and makes the user type the project name | a later milestone (D-26) |
//!
//! **Discharged by plan 18-07, which is why this file changed:** the command
//! picker and the goal field. [`DEFAULT_DRIVE_COMMAND`] is no longer the only
//! command a run can be started with — it is the default *selection* offered by
//! [`super::driver_start::DriverStartScreen`], and this confirmation names the
//! command the user actually chose. The injection **input** is
//! [`super::driver_inject`].
//!
//! What Phase 17 needed was only enough surface for a human to reach its own
//! success criteria: start a run, stop a live one, toggle the opt-in. That is
//! three keys on the dashboard and this one confirmation, modelled file for
//! file on [`super::delete_confirm`] — the same `[y/n]` one-line footer, the
//! same `Color::Red` for a destructive direction, the same free functions doing
//! the work and reporting through `ctx.error_message` / `ctx.status_message`,
//! and the same `ScreenAction::Pop` on both arms. No new widget, no new tab, no
//! second screen. Phase 18 adds one row above the prompt and changes no part of
//! that shape.
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

/// The GSD command a start is **pre-selected** with.
///
/// **No longer the only value a run can carry.** The command picker landed in
/// plan 18-07 ([`super::driver_start::DriverStartScreen`], Step A), and this
/// constant is now what an empty command field commits — the default
/// *selection*, exactly as D-23 specifies. A read-only progress command is the
/// right thing to default to: it is the one command that is safe to start
/// without reading the field.
///
/// There is still no computed **sequence**, because the D-R-P-E-V decision
/// router is **Phase 20's**; a Phase 18 run executes exactly one GSD command per
/// `drive` invocation, so the picker chooses one command and the driver's own
/// dry-run output says the same.
pub const DEFAULT_DRIVE_COMMAND: &str = "/gsd-progress";

/// The disclosed-file-set heading (**pinned contract** — see
/// [`SECTION_RESIDUAL_EXPOSURE`]).
///
/// The list that follows it is the one **recorded on the `DriverOptIn`**, never
/// a set recomputed at render time, so what the user reads is what was actually
/// approved. A file that did not exist at opt-in is shown as absent rather than
/// omitted — its later appearance is drift, and a list that quietly dropped it
/// would be the under-broad disclosure this text exists to avoid.
pub const SECTION_PROMPT_INPUTS: &str = "== Files whose bytes can reach a model prompt ==\n\
    These are the files this build reads from the project, and the digest each\n\
    had when the opt-in was recorded. If any of them changes, the opt-in is\n\
    re-confirmed at the spawn gate before a run starts.";

/// What the two model seams actually see (**pinned contract** — see
/// [`SECTION_RESIDUAL_EXPOSURE`]).
///
/// **Deliberately narrower than the file list above**, because disclosing a
/// broader set than is true is as dishonest as disclosing a narrower one. The
/// seam spawn suppresses `CLAUDE.md` auto-discovery and carries an empty tool
/// set, so the repository's `CLAUDE.md` does not enter the seams' prompts; what
/// does enter is the enumerated third-party strings, inside a labelled untrusted
/// boundary.
pub const SECTION_MODEL_SEAM: &str = "== What the two model seams see ==\n\
    The goal-decomposition and ambiguity seams run with an empty tool set and\n\
    with CLAUDE.md auto-discovery suppressed, so this project's CLAUDE.md does\n\
    NOT enter their prompts. What does enter is a fixed set of enumerated\n\
    strings read from STATE.md, ROADMAP.md and HANDOFF — phase names, statuses\n\
    and summaries — passed inside a labelled untrusted-content boundary and\n\
    never concatenated into instructions.";

/// The residual-exposure statement, and the pinned-contract rule for all three.
///
/// **These three constants are a contract, not decoration**, in exactly the
/// register `crate::driver::dry_run::SECTION_REFSPECS` establishes:
/// `the_disclosure_names_the_file_set_the_seams_and_what_is_not_closed` asserts
/// all three appear in this order by comparing byte offsets, so there is exactly
/// **one** place pinning the order rather than two that can disagree.
///
/// **This block says what the phase does NOT close, and that is its whole job.**
/// The executor profile — the spawn that actually runs a GSD command — does
/// still load the target repository's `CLAUDE.md`, because GSD commands are
/// skills and suppressing it would degrade every honest run in order to defend
/// against a dishonest one. It is disclosed rather than fixed quietly or
/// omitted (research Q2).
///
/// It also states that the seam-side suppression has **no behavioural proof on
/// this transport**: no field in the init envelope reports whether it took
/// effect. Claiming a verified control here would be the overstated safety
/// claim D-27 refuses.
pub const SECTION_RESIDUAL_EXPOSURE: &str = "== What this does NOT close ==\n\
    The executor profile — the spawn that actually runs the GSD command — DOES\n\
    still load this project's CLAUDE.md. This phase does not change that, and\n\
    that exposure is the reason the digest and the re-confirmation above exist.\n\
    The seam-side suppression is also asserted on the spawn's arguments only:\n\
    nothing on this transport reports back whether it took effect, so it is not\n\
    a verified control. Treat this boundary as incomplete.";

/// Render the full opt-in disclosure: the recorded file set, what the seams
/// see, and what this does not close.
///
/// `prompt_inputs` is the list **recorded on the record** (or, when granting a
/// fresh opt-in, the list that is about to be recorded). It is never recomputed
/// from disk here — a disclosure that recomputed would show a set the user never
/// approved, which is the replay hazard research Q4 names.
///
/// An empty list renders as an explicit statement that it is empty, never as an
/// absent section: a blank space where a file list belongs reads as "nothing
/// enters a prompt", which is the opposite of what an empty list means.
pub fn render_disclosure(prompt_inputs: &[crate::config::PromptInput]) -> String {
    let mut out = String::new();
    out.push_str(SECTION_PROMPT_INPUTS);
    out.push('\n');

    if prompt_inputs.is_empty() {
        out.push_str(
            "  (this opt-in recorded no file list, so it will be re-confirmed\n   \
             before any run starts)\n",
        );
    } else {
        for input in prompt_inputs {
            // The profile label comes from the code that does the reading, not
            // from the record — see `config::PromptInput`.
            let profile = registry::DISCLOSED_PROMPT_INPUTS
                .iter()
                .find(|(path, _)| *path == input.path)
                .map(|(_, profile)| match profile {
                    registry::PromptProfile::ModelSeam => "model seam",
                    registry::PromptProfile::Executor => "executor",
                })
                // A recorded path this build no longer reads is shown rather
                // than hidden: the user approved it, so it belongs in the list.
                .unwrap_or("not read by this build");

            let digest = input
                .digest
                .as_deref()
                .unwrap_or("(absent at opt-in — appearing later re-confirms)");

            out.push_str(&format!(
                "  {} [{}]\n    {}\n",
                super::sanitize_render_line(&input.path),
                profile,
                super::sanitize_render_line(digest),
            ));
        }
    }

    out.push('\n');
    out.push_str(SECTION_MODEL_SEAM);
    out.push_str("\n\n");
    out.push_str(SECTION_RESIDUAL_EXPOSURE);
    out
}

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
///
/// `command` and `goal` are meaningful only for [`DriverAction::Start`], and
/// they live on the screen rather than inside the enum variant on purpose: the
/// enum stays `Copy` and keeps working as a plain discriminator in
/// [`prompt_text`] and [`prompt_color`], and the dashboard's existing `r`
/// binding keeps compiling against the two-argument [`Self::new`].
///
/// **One field feeds both the prompt and the dispatch** (T-18-41). The command
/// rendered in the confirmation and the command sent in
/// [`Action::DriverStartRequested`] are the *same* `String`, so a confirmation
/// that named one command while starting another is not expressible here.
pub struct DriverConfirmScreen {
    alias: String,
    action: DriverAction,
    /// The command a `Start` will run. [`DEFAULT_DRIVE_COMMAND`] unless the
    /// start flow chose otherwise.
    command: String,
    /// The human's goal, **verbatim**, or `None` when none was given.
    ///
    /// Never `Some("")`: the empty string is a different and worse thing than
    /// no goal — `drive_argv` omits the flag entirely for `None`, and an empty
    /// goal would be recorded verbatim in `RunRecord.goal` and rendered as if
    /// the user had said something. [`super::driver_start`] is what enforces
    /// that mapping.
    goal: Option<String>,
}

impl DriverConfirmScreen {
    /// A confirmation with the default command selection and no goal.
    ///
    /// This is the dashboard's `r` / `x` / `o` path. For a `Start` that came
    /// through the command picker, use [`Self::new_start`].
    pub fn new(alias: String, action: DriverAction) -> Self {
        Self {
            alias,
            action,
            command: DEFAULT_DRIVE_COMMAND.to_string(),
            goal: None,
        }
    }

    /// A `Start` confirmation for the command and goal the user chose.
    ///
    /// `goal` is stored and dispatched **verbatim, never paraphrased** — goal
    /// decomposition and prompt-injection hardening are Phase 21's, and this
    /// phase interprets nothing. Only the *rendering* is sanitised.
    pub fn new_start(alias: String, command: String, goal: Option<String>) -> Self {
        Self {
            alias,
            action: DriverAction::Start,
            command,
            goal,
        }
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
///
/// `command` is the one the run will actually be started with, **not**
/// [`DEFAULT_DRIVE_COMMAND`] — the last thing a user sees before an autonomous
/// agent starts on their repo has to name the real thing. It is sanitised
/// before it is interpolated: the picker's suggestion list can come from a
/// `gsd-tools smart-entry` subprocess, so a command string is not necessarily
/// something the user typed, and an escape sequence reaching a rendered prompt
/// can repaint the screen or forge a line (T-18-38).
fn prompt_text(alias: &str, action: DriverAction, opted_in: bool, command: &str) -> String {
    match action {
        DriverAction::Start => {
            let command = super::sanitize_render_line(command);
            format!(
                "Drive \"{alias}\" with {command}? An autonomous agent will run it in that \
                 project with full autonomy, including git operations. [y/n]"
            )
        }
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

/// The `Goal: {goal}` row that precedes a `Start` prompt when a goal was given.
///
/// `None` when there is none — the row is **absent** rather than empty, because
/// an empty `Goal:` label reads as a goal the user failed to give rather than as
/// a goal they chose not to give (the "(none given)" copy belongs to the run
/// detail, not to a confirmation).
///
/// Sanitised, single line, ellipsis-truncated: [`super::sanitize_render_line`]
/// strips `ESC` unconditionally, replaces every other C0 control and `DEL`, and
/// caps by `char`. Without it a goal is a straight path from free text to a
/// rendered terminal line (T-18-38).
fn goal_row(goal: Option<&str>) -> Option<String> {
    goal.map(|g| format!("Goal: {}", super::sanitize_render_line(g)))
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
                    DriverAction::Start => {
                        do_start_run(ctx, &self.alias, &self.command, self.goal.as_deref())
                    }
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
        // The goal, when there is one, takes a row of its own above the prompt.
        // A `Start` with no goal — and every `Stop` and every toggle — renders
        // the same single-row footer this screen has always had.
        let is_start = matches!(self.action, DriverAction::Start);
        let goal = goal_row(self.goal.as_deref().filter(|_| is_start));
        let goal_rows = u16::from(goal.is_some());

        let chunks = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(goal_rows),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);

        use ratatui::widgets::{Block, Borders};
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" GSD Manager ");
        let inner = block.inner(chunks[0]);
        frame.render_widget(block, chunks[0]);

        // The disclosure is shown only when GRANTING an opt-in. Withdrawing one
        // takes a capability away, and reading a file list is not something a
        // user needs to do in order to revoke.
        if matches!(self.action, DriverAction::ToggleOptIn)
            && !registry::is_opted_in(&ctx.config, &self.alias)
        {
            // What is about to be recorded, so the user approves the same list
            // `record_opt_in` will write.
            let prompt_inputs = ctx
                .config
                .projects
                .get(&self.alias)
                .map(|entry| registry::current_prompt_inputs(&entry.path))
                .unwrap_or_default();
            frame.render_widget(
                Paragraph::new(render_disclosure(&prompt_inputs))
                    .style(Style::default().fg(Color::DarkGray)),
                inner,
            );
        }

        if let Some(goal) = goal {
            let line = Line::from(Span::styled(goal, Style::default().fg(Color::DarkGray)));
            frame.render_widget(Paragraph::new(line), chunks[1]);
        }

        // The opt-in state comes from the registry, which is the same read the
        // toggle itself performs — so the prompt cannot describe a direction
        // other than the one `y` will take.
        let opted_in = registry::is_opted_in(&ctx.config, &self.alias);
        let prompt = prompt_text(&self.alias, self.action, opted_in, &self.command);
        let line = Line::from(Span::styled(
            prompt,
            Style::default().fg(prompt_color(self.action, opted_in)),
        ));
        frame.render_widget(Paragraph::new(line), chunks[2]);
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
fn do_start_run(ctx: &mut AppContext, alias: &str, command: &str, goal: Option<&str>) {
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
            // The command the prompt just named, not the constant. Verbatim:
            // this is an argv value, and the sanitising in `prompt_text` is a
            // rendering concern that must not reach what gets executed.
            command: command.to_string(),
            // Verbatim or absent — never a fabricated placeholder. OBS-03
            // renders an absent goal as `(none given)`, and inventing a summary
            // here is exactly what D-13 forbids.
            goal: goal.map(str::to_string),
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
pub(super) fn dispatch(ctx: &mut AppContext, action: Action, alias: &str) {
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
///
/// **A revert restores; it never approves (WR-04).** The withdrawal direction
/// puts back a *clone of the record the user already granted*, captured before
/// anything is applied. It deliberately does not call
/// [`registry::record_opt_in`] — see the comment at the revert itself for why
/// that is a laundering rather than a restore.
fn do_toggle_opt_in(ctx: &mut AppContext, alias: &str) {
    let was_opted_in = registry::is_opted_in(&ctx.config, alias);

    // Captured BEFORE anything is applied, because after the toggle it is gone.
    // `None` in the grant direction, where there is nothing to put back.
    let previous = ctx
        .config
        .projects
        .get(alias)
        .and_then(|entry| entry.driver_opt_in.clone());

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
        // **The withdrawal revert is an assignment, not a construction, and that
        // is not a violation of `record_opt_in`'s sole-constructor property
        // (D-14) — it is what preserves it.** The value put back is a clone of a
        // record the user constructed by opting in; nothing new comes into
        // existence, so a `Some(record)` in a `config.json` is still proof of a
        // deliberate user action, which is the claim `registry.rs:96-99` makes
        // and which this call site must not become a counterexample to.
        //
        // Calling `record_opt_in` here instead — which is what this did — mints
        // a record with a fresh `opted_in_at` and a fresh `current_prompt_inputs`
        // snapshot. That approves whatever the disclosed files contain *right
        // now*, with no disclosure shown and no user act: a project whose
        // `CLAUDE.md` was rewritten under a `git pull` since the opt-in, and
        // which the spawn gate would have refused with `PromptInputsDrifted`,
        // comes back already approved and the next successful save persists it.
        // That is exactly the drift plan 21-03's SHA-256 digests were added to
        // catch (WR-04, T-21-10-02).
        //
        // The grant direction keeps `clear_opt_in`, which is correct and
        // unchanged: withdrawing a record that never reached disk needs no prior
        // value.
        let restored = if was_opted_in {
            match ctx.config.projects.get_mut(alias) {
                Some(entry) => {
                    entry.driver_opt_in = previous;
                    true
                }
                // Only reachable if the alias vanished from the config between
                // the toggle and here. Nothing in this process can do that —
                // both `registry` calls above bail on an unknown alias before
                // the save is attempted — but the note below is what a user
                // would need if it ever became reachable, and deriving it from
                // an outcome rather than asserting the outcome is what keeps it
                // honest.
                None => false,
            }
        } else {
            registry::clear_opt_in(&mut ctx.config, alias).is_ok()
        };
        let note = if restored {
            ""
        } else {
            " (and the in-memory registry could not be restored)"
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
pub(crate) mod tests {
    use super::*;
    use crate::config::{Config, RegisteredProject};
    use std::path::Path;
    use tokio::sync::mpsc::UnboundedReceiver;

    pub(crate) const ALIAS: &str = "proj";

    /// An `AppContext` with one registered project, a live event channel, and a
    /// config path inside `root` so `save_config` is a real write.
    ///
    /// `pub(crate)` so the sibling driver screens — `driver_inject` and
    /// `driver_start` — assert against the **same** `Action` receiver rather
    /// than each standing up a fifth and sixth full-field `AppContext` fixture.
    /// Every field of `AppContext` is named here, so this is one of the sites
    /// that breaks on any addition to it; concentrating that cost is the point.
    pub(crate) fn ctx_with_project(root: &Path) -> (AppContext, UnboundedReceiver<Action>) {
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
            last_outcomes: HashMap::new(),
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
            "the dashboard's `r` path gives no goal, and an absent goal must \
             travel as absent rather than as an invented summary"
        );
    }

    /// Two entries, one present and one absent, so both digest renderings are
    /// exercised.
    fn two_inputs() -> Vec<crate::config::PromptInput> {
        vec![
            crate::config::PromptInput {
                path: "CLAUDE.md".to_string(),
                digest: Some("sha256:aaaa".to_string()),
                extra: Default::default(),
            },
            crate::config::PromptInput {
                path: ".planning/STATE.md".to_string(),
                digest: Some("sha256:bbbb".to_string()),
                extra: Default::default(),
            },
        ]
    }

    #[test]
    fn the_disclosure_names_the_file_set_the_seams_and_what_is_not_closed() {
        let rendered = render_disclosure(&two_inputs());

        // Byte offsets, so ONE place pins the order rather than two that can
        // disagree — following `dry_run`'s section-ordering assertion.
        let files = rendered
            .find(SECTION_PROMPT_INPUTS)
            .expect("the disclosed file set must appear");
        let seams = rendered
            .find(SECTION_MODEL_SEAM)
            .expect("what the seams see must appear");
        let residual = rendered
            .find(SECTION_RESIDUAL_EXPOSURE)
            .expect("what this does not close must appear");

        assert!(
            files < seams && seams < residual,
            "the three blocks must render in declaration order: file set, then \
             what the seams see, then what is not closed. Got offsets \
             {files}/{seams}/{residual}"
        );
    }

    #[test]
    fn the_residual_block_says_the_executor_still_loads_claude_md_and_is_not_verified() {
        let rendered = render_disclosure(&two_inputs());

        assert!(
            rendered.contains("executor profile"),
            "the residual block must name the profile that is still exposed"
        );
        assert!(
            rendered.contains("still load this project's CLAUDE.md"),
            "the load-bearing admission must be present verbatim, not implied"
        );
        assert!(
            rendered.contains("This phase does not change that"),
            "the text must say the exposure is open, not merely describe it"
        );
        assert!(
            rendered.contains("not\na verified control"),
            "the seam-side suppression has no behavioural proof on this \
             transport, and the text must say so rather than implying the \
             control is verified"
        );

        // The weaker predecessor wording, spelled out VERBATIM rather than
        // referenced. A test comparing the constant with itself could not
        // detect the stale claim's return — that is the Phase 20 Critical.
        for stale in [
            "this project's CLAUDE.md never reaches the model",
            "CLAUDE.md is never loaded",
            "the boundary is complete",
            "CLAUDE.md suppression is verified",
        ] {
            assert!(
                !rendered.contains(stale),
                "the disclosure must never claim `{stale}` — the executor profile \
                 does load it, and the seam-side suppression is unproven"
            );
        }
    }

    #[test]
    fn the_disclosure_renders_every_recorded_path_and_digest() {
        let rendered = render_disclosure(&two_inputs());

        assert!(rendered.contains("CLAUDE.md"));
        assert!(rendered.contains("sha256:aaaa"));
        assert!(rendered.contains(".planning/STATE.md"));
        assert!(rendered.contains("sha256:bbbb"));

        // The profile attribution distinguishes the two spawns, because the
        // exposure is different and a list that flattened them would mislead.
        assert!(rendered.contains("[executor]"), "CLAUDE.md is executor-side");
        assert!(
            rendered.contains("[model seam]"),
            "STATE.md is read by the seams"
        );
    }

    #[test]
    fn an_empty_recorded_list_says_so_rather_than_rendering_silence() {
        let rendered = render_disclosure(&[]);

        assert!(
            rendered.contains("recorded no file list"),
            "an empty section reads as 'nothing enters a prompt', which is the \
             opposite of what an empty list means. Got:\n{rendered}"
        );
        assert!(
            rendered.contains("re-confirmed"),
            "an empty list must say what happens next: it re-confirms"
        );
        // The other two blocks are still present — an empty file list does not
        // excuse dropping the residual-exposure statement.
        assert!(rendered.contains(SECTION_RESIDUAL_EXPOSURE));
    }

    #[test]
    fn the_rendered_list_is_the_recorded_one_not_a_recomputed_set() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("CLAUDE.md"), b"original").expect("write");

        let recorded = registry::current_prompt_inputs(dir.path());
        let before = render_disclosure(&recorded);

        // Change the file the recomputed set would cover. If the disclosure
        // recomputed at render time, the digest on screen would move — and the
        // user would be shown a set they never approved.
        std::fs::write(dir.path().join("CLAUDE.md"), b"rewritten by a git pull").expect("write");

        let after = render_disclosure(&recorded);
        assert_eq!(
            before, after,
            "the disclosure renders the RECORDED list; recomputing it at render \
             time would show bytes the user never approved"
        );
        assert_ne!(
            recorded,
            registry::current_prompt_inputs(dir.path()),
            "precondition: the on-disk set really did change, so the assertion \
             above is not vacuous"
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

    // ========================================================================
    // A failed save reverts to what existed; it never approves (WR-04, SAFE-07)
    //
    // `registry::record_opt_in` is the ONLY function outside tests that
    // constructs a `DriverOptIn`, and that uniqueness is what makes a
    // `Some(record)` in a `config.json` proof of a deliberate user act (D-14).
    // Calling it on the withdrawal-direction revert broke exactly that: it
    // stamps a fresh `opted_in_at` and takes a fresh `current_prompt_inputs`
    // snapshot, so a project whose `CLAUDE.md` was rewritten under a `git pull`
    // — one the spawn gate would have refused with `PromptInputsDrifted` —
    // came back with the NEW bytes already approved, with no disclosure shown
    // and no user act, and the next successful save persisted it.
    //
    // The equality assertions below are what proves the sole constructor is no
    // longer called on a revert: `record_opt_in` cannot reproduce a record it
    // did not write, because it re-stamps the timestamp even when nothing on
    // disk has moved.
    // ========================================================================

    /// A config path `save_config` cannot write.
    ///
    /// Its parent **component is a regular file**, so `create_dir_all` fails
    /// with `NotADirectory` before a temp file is ever created. Deterministic
    /// and portable, and deliberately not a read-only directory: a permission
    /// bit does not stop a process running as root, which some CI does.
    fn unwritable_config_path(root: &Path) -> std::path::PathBuf {
        let blocker = root.join("not-a-directory");
        std::fs::write(&blocker, b"a regular file standing where a directory must be")
            .expect("the blocker is written");
        blocker.join("config.json")
    }

    /// A record a user granted, at a moment and over a file set both
    /// distinguishable from anything `record_opt_in` would mint from a temp
    /// directory right now.
    ///
    /// That distinguishability is the whole assertion: an implementation that
    /// re-minted the record would differ in `opted_in_at` even if the disclosed
    /// files had not moved at all.
    fn granted_record() -> crate::config::DriverOptIn {
        crate::config::DriverOptIn {
            opted_in_at: "2020-01-01T00:00:00Z".to_string(),
            claude_md_digest: None,
            prompt_inputs: two_inputs(),
            extra: Default::default(),
            branch_namespace: None,
            credential: None,
            pr_cap_per_24h: None,
            pr_cap_per_run: None,
        }
    }

    /// Put `record` on the fixture project, as a prior opt-in the user granted.
    fn grant(ctx: &mut AppContext, record: crate::config::DriverOptIn) {
        ctx.config
            .projects
            .get_mut(ALIAS)
            .expect("the fixture project")
            .driver_opt_in = Some(record);
    }

    /// Press `y` on the opt-in toggle, which is the only way a user reaches it.
    fn toggle(ctx: &mut AppContext) {
        let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::ToggleOptIn);
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, ctx);
    }

    #[test]
    fn a_failed_save_after_a_withdrawal_restores_the_record_the_user_granted() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        ctx.config_path = unwritable_config_path(dir.path());

        let granted = granted_record();
        grant(&mut ctx, granted.clone());

        toggle(&mut ctx);

        let after = ctx.config.projects[ALIAS]
            .driver_opt_in
            .clone()
            .expect("a withdrawal whose save failed must be rolled back in memory");
        assert_eq!(
            after, granted,
            "the revert must restore the record that EXISTED, field for field — \
             same opted_in_at, same prompt_inputs, same digests. Against the \
             unfixed build this fails on `opted_in_at` alone, because the revert \
             called `registry::record_opt_in`, which is a constructor rather than \
             a restore: it mints a new approval covering whatever the disclosed \
             files contain at that instant, with no disclosure shown and no user \
             act (WR-04)"
        );
    }

    #[test]
    fn a_failed_save_after_a_withdrawal_does_not_rebaseline_a_drifted_disclosure() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        ctx.config_path = unwritable_config_path(dir.path());

        // The drift the SHA-256 digests were added in plan 21-03 to catch: the
        // approval covers bytes that are no longer the bytes on disk.
        std::fs::write(
            dir.path().join("CLAUDE.md"),
            b"# rewritten under a git pull, after the user opted in\n",
        )
        .expect("the drifted file is written");

        let granted = granted_record();
        grant(&mut ctx, granted.clone());

        // Non-vacuity, asserted BEFORE the act rather than inferred after: the
        // recorded set and the on-disk set really do disagree, so "restored"
        // and "re-baselined" are distinguishable outcomes here.
        assert_ne!(
            granted.prompt_inputs,
            registry::current_prompt_inputs(dir.path()),
            "precondition: the recorded digests must differ from what the files \
             hash to now, or this test cannot tell a restore from a re-baseline"
        );

        toggle(&mut ctx);

        let after = ctx.config.projects[ALIAS]
            .driver_opt_in
            .clone()
            .expect("the withdrawal is rolled back");
        assert_eq!(
            after.prompt_inputs, granted.prompt_inputs,
            "the restored record must still carry the OLD digests, so the spawn \
             gate still refuses this project with PromptInputsDrifted. A revert \
             that re-snapshots the files launders exactly the drift the digests \
             exist to catch — and the next successful save persists the laundered \
             approval (T-21-10-02)"
        );
        assert_ne!(
            after.prompt_inputs,
            registry::current_prompt_inputs(dir.path()),
            "and it must not have adopted the new bytes"
        );
    }

    #[test]
    fn a_failed_save_after_a_grant_leaves_no_record_at_all() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        ctx.config_path = unwritable_config_path(dir.path());
        assert!(
            !registry::is_opted_in(&ctx.config, ALIAS),
            "precondition: the fixture starts NOT opted in"
        );

        toggle(&mut ctx);

        assert!(
            ctx.config.projects[ALIAS].driver_opt_in.is_none(),
            "`clear_opt_in` is the correct revert in this direction and is \
             unchanged: withdrawing a record that never reached disk needs no \
             prior value, and leaving one in memory would make the TUI report a \
             project as drivable while the gate — which reads config.json in a \
             different process — refuses every run against it"
        );
    }

    #[test]
    fn a_failed_save_names_the_save_failure_in_both_directions() {
        for (direction, prior) in [
            ("withdrawal", Some(granted_record())),
            ("grant", None),
        ] {
            let dir = tempfile::tempdir().expect("temp dir");
            let (mut ctx, _rx) = ctx_with_project(dir.path());
            ctx.config_path = unwritable_config_path(dir.path());
            if let Some(record) = prior {
                grant(&mut ctx, record);
            }

            toggle(&mut ctx);

            let message = ctx
                .error_message
                .as_deref()
                .unwrap_or_else(|| panic!("{direction}: the save failure must be visible"));
            assert!(
                message.starts_with("Failed to save config:"),
                "{direction}: the message must name the SAVE as what failed — it \
                 is the only thing the user can act on. Got: {message}"
            );
            assert!(
                !message.contains("could not be restored"),
                "{direction}: the in-memory registry WAS restored here, so the \
                 note must not appear. It is reserved for an alias that vanished \
                 from the config between the toggle and the revert, which no \
                 caller can currently produce — `record_opt_in` and \
                 `clear_opt_in` both bail on an unknown alias before the save is \
                 attempted. Got: {message}"
            );
            assert!(
                ctx.status_message.is_none(),
                "{direction}: a failed save must not also report success"
            );
        }
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
            let prompt = prompt_text(ALIAS, action, opted_in, DEFAULT_DRIVE_COMMAND);
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
        let withdraw = prompt_text(ALIAS, DriverAction::ToggleOptIn, true, DEFAULT_DRIVE_COMMAND);
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

    const CHOSEN: &str = "/gsd:execute-phase 18";

    /// The last thing a user sees before an autonomous agent starts on their
    /// repo has to name the exact command (T-18-41).
    #[test]
    fn the_start_prompt_names_the_chosen_command_and_not_the_default() {
        let prompt = prompt_text(ALIAS, DriverAction::Start, true, CHOSEN);
        assert!(prompt.contains(CHOSEN), "got: {prompt}");
        assert!(
            !prompt.contains(DEFAULT_DRIVE_COMMAND),
            "the constant is a default selection, not the prompt's content — a \
             confirmation naming a command other than the one that will run is \
             worse than no confirmation. Got: {prompt}"
        );
        assert!(prompt.contains(ALIAS) && prompt.ends_with("[y/n]"), "got: {prompt}");

        // And the default path still says the default, so the assertion above
        // is not passing because the prompt named nothing at all.
        let defaulted = prompt_text(ALIAS, DriverAction::Start, true, DEFAULT_DRIVE_COMMAND);
        assert!(defaulted.contains(DEFAULT_DRIVE_COMMAND), "got: {defaulted}");
    }

    #[test]
    fn the_goal_row_renders_only_when_a_goal_was_given() {
        assert_eq!(goal_row(None), None, "an absent goal takes no row at all");
        assert_eq!(
            goal_row(Some("ship the driver tab")).as_deref(),
            Some("Goal: ship the driver tab"),
            "the goal is shown verbatim — this phase paraphrases nothing"
        );
    }

    /// T-18-38: a goal is free text on a straight path to a rendered terminal
    /// line, and the picker's commands can come from a subprocess.
    #[test]
    fn an_escape_bearing_goal_and_command_are_sanitised_before_they_reach_the_prompt() {
        let nasty = "finish\u{1b}[2Jthe\u{1b}]0;pwned\u{7}phase";

        let row = goal_row(Some(nasty)).expect("a goal was given");
        assert!(
            !row.contains('\u{1b}'),
            "no ESC may survive into a rendered row, got: {row:?}"
        );
        assert!(
            row.contains("finish") && row.contains("phase"),
            "the prose itself must still be shown — a sanitiser that passes by \
             deleting everything is not a sanitiser. Got: {row:?}"
        );

        let prompt = prompt_text(ALIAS, DriverAction::Start, true, nasty);
        assert!(
            !prompt.contains('\u{1b}'),
            "the command is interpolated into the prompt too, got: {prompt:?}"
        );
    }

    /// The prompt and the dispatched action must agree, and the goal must reach
    /// `run.json` byte-identically, including multi-byte input (OBS-03).
    #[test]
    fn a_start_from_the_picker_dispatches_the_chosen_command_and_the_verbatim_goal() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        registry::record_opt_in(&mut ctx.config, ALIAS).expect("opt in");

        let typed = "livrer l'onglet 🚀 — 日本語";
        let mut screen = DriverConfirmScreen::new_start(
            ALIAS.to_string(),
            CHOSEN.to_string(),
            Some(typed.to_string()),
        );
        let action = screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::Pop));

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
        assert_eq!(
            command, CHOSEN,
            "the dispatched command must be the one the prompt named — and \
             verbatim, because it is an argv value and the sanitising is a \
             rendering concern"
        );
        assert_eq!(
            goal.as_deref().map(str::as_bytes),
            Some(typed.as_bytes()),
            "the goal reaches RunRecord.goal exactly as typed; interpretation is \
             Phase 21's and this phase interprets nothing"
        );
    }
}
