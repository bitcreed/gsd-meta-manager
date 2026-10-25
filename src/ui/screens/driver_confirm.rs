//! **The driver confirmation** — the one `[y/n]` popup behind the Driver tab's
//! two confirmed actions: toggling a project's driver opt-in record (`o`) and
//! killing a launcher run (`x`).
//!
//! Phase 31 (D-03) retired the legacy `drive` start and stop actions and the
//! dashboard keys that pushed this screen, so [`DriverAction`] is exactly
//! `ToggleOptIn` and `StopLauncher`. The shape is the one Phase 17 set: modelled
//! file for file on [`super::delete_confirm`] — the same `[y/n]` footer, the same
//! `Color::Red` for a direction that takes something away, the same free
//! functions doing the work and reporting through `ctx.error_message` /
//! `ctx.status_message`, and the same `ScreenAction::Pop` on both arms.
//!
//! **What the opt-in record does now.** The launcher's push step reads its PR caps
//! and branch namespace from it (`launcher::push::policy_for`) and falls back to
//! the defaults when there is none. The prompts say that and nothing more: no
//! safety claim the code does not back (T-17-48).
//!
//! Driver state is read from the **sibling maps on `AppContext`** — never from
//! `ProjectState`, whose derived `PartialEq` drives the v1.4 unchanged-state
//! status suppression that a multi-hour run would otherwise defeat for its whole
//! duration (D-25, ARCHITECTURE AP1).

use super::confirm_popup::{ConfirmOutcome, ConfirmPopup};
use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::config::save_config;
use crate::registry;
use crate::ui::mouse::MouseInput;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

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
    had when the opt-in was recorded. The digest is a snapshot of that moment.";

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
        out.push_str("  (this opt-in recorded no file list)\n");
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

            // Two classes, two predicates, composed (WR-06) — the same shape
            // `prompt_text` uses 100 lines below, and for the same
            // reason: `sanitize_render_line` answers the CONTROL class, and the
            // invisible-formatting class (`Cf` union `Default_Ignorable`) is a
            // different question that neither predicate subsumes. This screen is
            // the last thing an operator reads before an irreversible act.
            //
            // Held by a CALL, not by a type: `PromptInput::path` and `digest`
            // are bare `String`s in `src/config.rs`, outside this plan's wave
            // fence, so a NEW render of either would not fail to compile.
            // **Under-protection, silent** — see
            // `the_disclosure_escapes_both_classes_over_the_path_and_the_digest`,
            // which is the committed control, and which also records why this
            // one is at the function rather than at the screen.
            out.push_str(&format!(
                "  {} [{}]\n    {}\n",
                crate::text::display_identity(&super::sanitize_render_line(&input.path)),
                profile,
                crate::text::display_identity(&super::sanitize_render_line(digest)),
            ));
        }
    }

    out.push('\n');
    out.push_str(SECTION_MODEL_SEAM);
    out.push_str("\n\n");
    out.push_str(SECTION_RESIDUAL_EXPOSURE);
    out
}

/// Which of the two driver actions this confirmation is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverAction {
    /// Toggle the driver opt-in record for the selected project.
    ToggleOptIn,
    /// Stop a launcher run (28-07, D-11). The run id lives on the screen so this
    /// enum stays `Copy`.
    StopLauncher,
}

/// One confirmation screen serving both driver actions.
pub struct DriverConfirmScreen {
    alias: String,
    action: DriverAction,
    /// The launcher run a [`DriverAction::StopLauncher`] will stop; empty for the
    /// opt-in toggle. One field feeds both the prompt and the dispatch, so a
    /// confirmation that named one run while stopping another is not expressible
    /// (T-18-41, T-28-18).
    run_id: String,
    popup: ConfirmPopup,
}

impl DriverConfirmScreen {
    /// A confirmation for `action` on `alias`, with no run attached.
    pub fn new(alias: String, action: DriverAction) -> Self {
        Self {
            alias,
            action,
            run_id: String::new(),
            popup: ConfirmPopup::default(),
        }
    }

    /// A confirmation for stopping launcher run `run_id` of `alias` (28-07, D-11).
    pub fn new_launcher_stop(alias: String, run_id: String) -> Self {
        Self {
            run_id,
            ..Self::new(alias, DriverAction::StopLauncher)
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
/// **Both opt-in directions say only what the record feeds** (C12, T-17-48): the
/// push step reads its PR caps and branch namespace from it
/// (`launcher::push::policy_for`) and uses the defaults without one. Neither
/// claims a gate or says that a run can or cannot start, because nothing
/// here gates a run — and a safety claim the implementation does not back is
/// worse than no claim at all.
///
/// The alias is escaped through [`crate::text::render_for_terminal`]: this prompt
/// is the last thing a human reads before an irreversible act, and
/// `sanitize_render_line` answers a DIFFERENT question — it strips `ESC` and
/// C0/DEL, the *control* class, and says nothing about the invisible-formatting
/// class (`General_Category=Cf` union `Default_Ignorable_Code_Point`). The two
/// compose; neither replaces the other. Nothing dispatched changes:
/// `do_stop_launcher` and `do_toggle_opt_in` receive `&self.alias` raw.
fn prompt_text(alias: &str, action: DriverAction, opted_in: bool, run_id: &str) -> String {
    // `render_for_terminal`, not a bare `display_identity` (21-28 T3, WR-05).
    // This was the one BARE invisible-class call left in either of this plan's
    // render files: it answered `Cf`/`Default_Ignorable` and said nothing about
    // `ESC`/C0/DEL, on the prompt a human reads immediately before an
    // irreversible act. `render_for_terminal` is the ONE composition of both
    // classes, so this stops being a site a reader has to check.
    //
    // Cross-plan contract: 21-29 lands a census in this same wave asserting
    // that no executable `display_identity` call under `src/ui/` stands outside
    // a composition. After the worktree merge it cannot edit this file, so this
    // conversion has to happen here.
    let alias = &crate::text::render_for_terminal(alias);
    match action {
        DriverAction::ToggleOptIn if opted_in => format!(
            "Withdraw the driver opt-in for \"{alias}\"? Its PR caps and branch namespace stop \
             applying; the push step falls back to the defaults. [y/n]"
        ),
        DriverAction::StopLauncher => {
            // The run id is a directory name and so third-party text: both classes
            // are escaped, through the one composition.
            let run_id = crate::text::render_for_terminal(run_id);
            format!(
                "Kill launcher run {run_id} on \"{alias}\"? Its whole sandbox scope is \
                 stopped now, mid-dispatch, and the in-flight work is not reported. To let \
                 the driver finish its current dispatch and end, use [c] then stop. [y/n]"
            )
        }
        DriverAction::ToggleOptIn => format!(
            "Record a driver opt-in for \"{alias}\"? The push step reads its PR caps and \
             branch namespace from it; without one it uses the defaults. [y/n]"
        ),
    }
}

/// `Color::Red` for the two directions that take something away — killing a
/// launcher run stops its whole sandbox scope, and withdrawing an opt-in removes
/// a record. The default foreground for granting one, following
/// `delete_confirm.rs`'s use of red for a destructive action.
fn prompt_color(action: DriverAction, opted_in: bool) -> Color {
    match action {
        DriverAction::StopLauncher => Color::Red,
        DriverAction::ToggleOptIn if opted_in => Color::Red,
        DriverAction::ToggleOptIn => Color::Reset,
    }
}

crate::ui::screens::adjudicate_screen!(
    DriverConfirmScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws the registry key into three prompts — kill launcher run, grant opt-in, \
     withdraw opt-in — each of which precedes an irreversible act, and draws the \
     launcher run id and the recorded prompt-input paths and digests of the \
     opt-in disclosure. Fixture states: all three prompts.",
);

impl Screen for DriverConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match self.popup.on_key(code) {
            ConfirmOutcome::Confirm => {
                match self.action {
                    DriverAction::StopLauncher => do_stop_launcher(ctx, &self.alias, &self.run_id),
                    DriverAction::ToggleOptIn => do_toggle_opt_in(ctx, &self.alias),
                }
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
        // The screen it was opened from stays rendered, dimmed, behind the popup. The
        // kill confirmation (`x`) is opened from the Driver tab, so it keeps that tab in
        // view (UI D6 `[inferred — audit]`); the opt-in toggle keeps the dashboard, as
        // it did when the dashboard also opened it.
        if matches!(self.action, DriverAction::StopLauncher) {
            super::detail::DetailScreen::new(self.alias.clone()).render_main_only(frame, area, ctx);
        } else {
            super::normal::NormalScreen::new().render(frame, area, ctx);
        }
        super::confirm_popup::dim_background(frame, area);
        let mut body: Vec<Line<'static>> = Vec::new();
        let mut disclosure: Option<String> = None;

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
            // Rendered after the prompt below (disclosure is supporting text).
            disclosure = Some(render_disclosure(&prompt_inputs));
        }

        // The opt-in state comes from the registry, which is the same read the
        // toggle itself performs — so the prompt cannot describe a direction
        // other than the one `y` will take.
        let opted_in = registry::is_opted_in(&ctx.config, &self.alias);
        let prompt = prompt_text(&self.alias, self.action, opted_in, &self.run_id);
        let color = prompt_color(self.action, opted_in);
        body.push(Line::from(Span::styled(
            super::confirm_popup::strip_key_hint(&prompt).to_string(),
            Style::default().fg(color),
        )));
        if let Some(text) = disclosure {
            body.push(Line::from(""));
            body.extend(text.lines().map(|l| {
                Line::from(Span::styled(
                    l.to_string(),
                    Style::default().fg(Color::DarkGray),
                ))
            }));
        }
        let (title, accent) = match self.action {
            DriverAction::StopLauncher => ("Kill driver run", Color::Red),
            DriverAction::ToggleOptIn if opted_in => ("Withdraw driver opt-in", Color::Red),
            DriverAction::ToggleOptIn => ("Allow driver", Color::Yellow),
        };
        self.popup
            .record(super::confirm_popup::render_confirm_popup(
                frame,
                area,
                title,
                body,
                accent,
                self.popup.focus,
            ));
    }

    fn handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction {
        super::confirm_popup::click_as_key(&self.popup, input).map_or(ScreenAction::None, |code| {
            self.handle_key(code, KeyModifiers::NONE, ctx)
        })
    }

    fn name(&self) -> &str {
        "driver_confirm"
    }
}

/// Ask the app to stop launcher run `run_id` of `alias` (28-07, D-11).
///
/// The message carries the run id: a launcher project can hold many runs, and
/// the id sent is the same `String` the prompt named (T-28-18).
/// `launcher::stop::stop_run` re-validates it against the project.
fn do_stop_launcher(ctx: &mut AppContext, alias: &str, run_id: &str) {
    dispatch(
        ctx,
        Action::LauncherStopRequested {
            alias: alias.to_string(),
            run_id: run_id.to_string(),
        },
        alias,
    );
}

/// Send one action, reporting visibly if there is nowhere to send it.
///
/// Without a channel there is no way to reach the app, and an action the user
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
/// not need to do and this does. The launcher's push step reads `config.json`
/// *from disk*, in a different process; an in-memory opt-in that never reached
/// the file would make the TUI show a record the push step never sees. Reverting
/// keeps the two in agreement, so the error message is the only thing the user
/// has to act on.
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
        // whose recorded digest therefore no longer matches (`PromptInputsDrifted`),
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
        format!("\"{alias}\": driver opt-in withdrawn")
    } else {
        format!("\"{alias}\": driver opt-in recorded")
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
    /// `pub(crate)` so the sibling driver screens assert against the **same**
    /// `Action` receiver rather than each standing up its own full-field
    /// `AppContext` fixture.
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
            agent_views: HashMap::new(),
            agents_scan_in_flight: false,
            sort_mode: crate::ui::screens::SortMode::default(),
            watcher: None,
            last_refresh: HashMap::new(),
            detail_scroll_offset: 0,
            suggestion_index: 0,
            input_buffer: String::new(),
            needs_redraw: false,
            active_sessions: Vec::new(),
            archive_cache: crate::archive::ArchiveCache::default(),
            // Fixtures default the experimental flag ON, so every driver test
            // written before 260917-fko keeps asserting what it always did;
            // the flag-off tests call `with_experimental(false)`.
            experimental: true,
        };
        (ctx, rx)
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

    /// **WR-06: the disclosure composes BOTH classes over both values it draws
    /// from its argument** (21-28 T3).
    ///
    /// `render_disclosure` drew `input.path` and `digest` through
    /// `sanitize_render_line` alone, which answers only the CONTROL class — so
    /// `U+202E`, `U+00AD` and the `U+E0000..U+E007F` tag block passed through
    /// into the `Paragraph` on the screen an operator reads immediately before
    /// an irreversible act. This is the committed control for that, and it was
    /// observed RED against the one-class code (the panic is quoted in
    /// `21-28-SUMMARY.md`).
    ///
    /// # Why this control is at the FUNCTION and not at the screen — a
    /// correction to what 21-28's plan assumed
    ///
    /// The plan expected a probe fixture on `DriverConfirmScreen` to go red
    /// here, on the premise that `input.path` and `digest` are "read out of the
    /// recorded opt-in block in the user's `config.json`". **Measured, that is
    /// not what the live screen path does.** `DriverConfirmScreen::render` calls
    /// `registry::current_prompt_inputs(&entry.path)`, which builds every
    /// `path` from the authored `&'static str`s in
    /// [`registry::DISCLOSED_PROMPT_INPUTS`] and every `digest` from
    /// `sha256_digest`, i.e. hex. **Neither value on the currently rendered path
    /// can carry an invisible-class character**, so no fixture on that screen
    /// could go red, and one that appeared to would be proving something else.
    ///
    /// The untrusted vector is this function's ARGUMENT, which is exactly what
    /// its own doc contracts for: *"the list **recorded on the record**"* — the
    /// opt-in block in a `config.json` this build does not exclusively own. The
    /// function is `pub`, and its recorded-list caller is the one the doc
    /// describes. So the control belongs where the value enters: here.
    ///
    /// **Residual, with its direction.** This bounds `render_disclosure` for any
    /// caller. It does NOT bound a future caller that formats a `PromptInput`
    /// somewhere else. **Under-detection, silent** — what would bound that is
    /// retyping `PromptInput::path`/`digest` as `crate::text::Untrusted`, which
    /// this plan could not do because `src/config.rs` is outside its wave fence.
    #[test]
    fn the_disclosure_escapes_both_classes_over_the_path_and_the_digest() {
        for (clean, hostile) in crate::test_support::LOOK_ALIKE_PAIRS {
            let rendered = render_disclosure(&[crate::config::PromptInput {
                path: hostile.to_string(),
                digest: Some(hostile.to_string()),
                extra: Default::default(),
            }]);
            assert!(
                !rendered
                    .chars()
                    .any(crate::text::is_invisible_formatting_char),
                "no invisible-class character may survive into the disclosure — \
                 this is the screen an operator reads immediately before an \
                 irreversible act. clean={clean:?} hostile={hostile:?} \
                 rendered={rendered:?}"
            );
        }

        // NON-VACUITY: the escaped spelling must actually be present, or an
        // implementation that dropped both values entirely would pass above.
        let rendered = render_disclosure(&[crate::config::PromptInput {
            path: "demo\u{e0041}".to_string(),
            digest: Some("sha256:d\u{202e}f".to_string()),
            extra: Default::default(),
        }]);
        assert!(
            rendered.contains("demoU+E0041"),
            "the path must be shown in its ESCAPED form, not dropped: {rendered:?}"
        );
        assert!(
            rendered.contains("sha256:dU+202Ef"),
            "and so must the digest: {rendered:?}"
        );
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
        assert!(
            rendered.contains("[executor]"),
            "CLAUDE.md is executor-side"
        );
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

        // It persisted, which is what a different process (the push step) reads.
        let saved = crate::config::load_config(&ctx.config_path).expect("the toggle must save");
        assert!(
            saved.projects[ALIAS].driver_opt_in.is_some(),
            "an opt-in that never reached disk is invisible to the push step, which is \
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
    // — whose recorded digest would then no longer match (`PromptInputsDrifted`) —
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
        std::fs::write(
            &blocker,
            b"a regular file standing where a directory must be",
        )
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
            "the restored record must still carry the OLD digests, so a drift check \
             still refuses this project with PromptInputsDrifted. A revert \
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
             project as opted in while the push step — which reads config.json in a \
             different process — never sees the record"
        );
    }

    #[test]
    fn a_failed_save_names_the_save_failure_in_both_directions() {
        for (direction, prior) in [("withdrawal", Some(granted_record())), ("grant", None)] {
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

    /// A launcher run id for the 28-07 tests.
    const RUN: &str = "20261019-0042-abcd";

    #[test]
    fn every_prompt_names_the_alias_and_offers_the_same_two_keys() {
        let cases = [
            (DriverAction::ToggleOptIn, false),
            (DriverAction::ToggleOptIn, true),
            (DriverAction::StopLauncher, false),
            (DriverAction::StopLauncher, true),
        ];

        for (action, opted_in) in cases {
            let prompt = prompt_text(ALIAS, action, opted_in, RUN);
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

        // The two directions that take something away are red; the one that
        // grants something is not. A prompt that warned in the wrong direction
        // would be worse than an unstyled one.
        assert_eq!(prompt_color(DriverAction::StopLauncher, false), Color::Red);
        assert_eq!(prompt_color(DriverAction::ToggleOptIn, true), Color::Red);
        assert_eq!(prompt_color(DriverAction::ToggleOptIn, false), Color::Reset);
    }

    /// C12 / D-13 (T-31-04): neither opt-in prompt claims a gate or says a
    /// run can or cannot start. Each states what the record feeds now, which is
    /// what `launcher::push::policy_for` reads, and each still names the alias
    /// (through `render_for_terminal`).
    #[test]
    fn the_opt_in_prompts_say_what_the_record_feeds_and_claim_no_gate() {
        let withdraw = prompt_text(ALIAS, DriverAction::ToggleOptIn, true, "");
        assert_eq!(
            withdraw,
            format!(
                "Withdraw the driver opt-in for \"{ALIAS}\"? Its PR caps and branch namespace \
                 stop applying; the push step falls back to the defaults. [y/n]"
            )
        );
        let allow = prompt_text(ALIAS, DriverAction::ToggleOptIn, false, "");
        assert_eq!(
            allow,
            format!(
                "Record a driver opt-in for \"{ALIAS}\"? The push step reads its PR caps and \
                 branch namespace from it; without one it uses the defaults. [y/n]"
            )
        );
        for prompt in [&withdraw, &allow] {
            for claim in ["gate", "new run", "be driven"] {
                assert!(
                    !prompt.to_lowercase().contains(&claim.to_lowercase()),
                    "an opt-in prompt claims {claim:?}, which nothing backs any more: {prompt}"
                );
            }
        }
    }

    /// The prompt escapes a hostile alias through the one composition, in both
    /// opt-in directions.
    #[test]
    fn the_opt_in_prompts_escape_a_hostile_alias() {
        let hostile = "x\u{1b}[2J\u{202e}y\u{200b}z";
        for opted_in in [true, false] {
            let prompt = prompt_text(hostile, DriverAction::ToggleOptIn, opted_in, "");
            assert!(
                !prompt.chars().any(|c| c.is_control() && c != '\n'),
                "a control character reached the prompt: {prompt:?}"
            );
            assert!(
                !prompt.contains('\u{202e}') && !prompt.contains('\u{200b}'),
                "an invisible formatting character reached the prompt: {prompt:?}"
            );
        }
    }

    // ── 28-07: the launcher stop confirmation ────────────────────────────

    #[test]
    fn launcher_stop_prompt_names_the_run_the_alias_and_the_whole_scope() {
        let prompt = prompt_text(ALIAS, DriverAction::StopLauncher, false, RUN);
        assert_eq!(
            prompt,
            format!(
                "Kill launcher run {RUN} on \"{ALIAS}\"? Its whole sandbox scope is stopped \
                 now, mid-dispatch, and the in-flight work is not reported. To let the driver \
                 finish its current dispatch and end, use [c] then stop. [y/n]"
            )
        );
    }

    #[test]
    fn launcher_stop_prompt_escapes_a_hostile_alias_and_run_id() {
        let hostile = "x\u{1b}[2J\u{202e}y\u{200b}z";
        let prompt = prompt_text(hostile, DriverAction::StopLauncher, false, hostile);
        assert!(
            prompt.starts_with("Kill launcher run "),
            "the prompt was not built at all: {prompt:?}"
        );
        assert!(
            !prompt.chars().any(|c| c.is_control() && c != '\n'),
            "a control character reached the prompt: {prompt:?}"
        );
        assert!(
            !prompt.contains('\u{202e}') && !prompt.contains('\u{200b}'),
            "an invisible formatting character reached the prompt: {prompt:?}"
        );
    }

    #[test]
    fn launcher_stop_confirming_sends_the_confirmed_run_id_and_cancelling_sends_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());

        for key in [KeyCode::Char('n'), KeyCode::Esc] {
            let mut screen = DriverConfirmScreen::new_launcher_stop(ALIAS.to_string(), RUN.into());
            let action = screen.handle_key(key, KeyModifiers::NONE, &mut ctx);
            assert!(matches!(action, ScreenAction::Pop), "{key:?} must pop");
            assert!(rx.try_recv().is_err(), "{key:?} must dispatch nothing");
        }

        let mut screen = DriverConfirmScreen::new_launcher_stop(ALIAS.to_string(), RUN.into());
        let action = screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::Pop));
        let sent = rx.try_recv().expect("a confirmed stop must dispatch");
        let Action::LauncherStopRequested { alias, run_id } = sent else {
            panic!("expected LauncherStopRequested, got {sent:?}");
        };
        assert_eq!((alias.as_str(), run_id.as_str()), (ALIAS, RUN));
        assert!(rx.try_recv().is_err(), "exactly one request");
    }

    #[test]
    fn launcher_stop_is_not_gated_on_the_driver_opt_in() {
        // Withdrawing an opt-in must never strand a live sandboxed run.
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        assert!(!registry::is_opted_in(&ctx.config, ALIAS));
        let mut screen = DriverConfirmScreen::new_launcher_stop(ALIAS.to_string(), RUN.into());
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);
        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn declining_dispatches_nothing_and_pops() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        registry::record_opt_in(&mut ctx.config, ALIAS).expect("opt in");

        for key in [KeyCode::Char('n'), KeyCode::Esc] {
            let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::ToggleOptIn);
            let action = screen.handle_key(key, KeyModifiers::NONE, &mut ctx);
            assert!(matches!(action, ScreenAction::Pop), "{key:?} must pop");
            assert!(
                rx.try_recv().is_err(),
                "{key:?} must dispatch nothing — a confirmation that acts on a \
                 decline is not a confirmation"
            );
        }

        // Any other key is consumed and does nothing at all.
        let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::ToggleOptIn);
        let action = screen.handle_key(KeyCode::Char('z'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::None));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn launcher_kill_confirm_keeps_the_driver_tab_behind_it() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let dir = tempfile::tempdir().expect("temp dir");
        let (ctx, _rx) = ctx_with_project(dir.path());
        let screen = DriverConfirmScreen::new_launcher_stop(ALIAS.to_string(), RUN.to_string());
        let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
        t.draw(|f| screen.render(f, f.area(), &ctx)).unwrap();
        let buf = t.backend().buffer();
        let text: String = (0..24u16)
            .map(|y| (0..80u16).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Kill driver run"), "{text}");
        assert!(
            text.contains("Project: proj"),
            "the Driver tab is behind the kill prompt:\n{text}"
        );
        assert!(
            !text.contains(" GSD Manager "),
            "not the dashboard:\n{text}"
        );
    }

    #[test]
    fn every_driver_confirmation_renders_as_a_popup_with_buttons() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let dir = tempfile::tempdir().expect("temp dir");
        let (ctx, _rx) = ctx_with_project(dir.path());
        for (action, title) in [
            (DriverAction::ToggleOptIn, "Allow driver"),
            (DriverAction::StopLauncher, "Kill driver run"),
        ] {
            let screen = DriverConfirmScreen::new_launcher_stop(ALIAS.to_string(), RUN.to_string());
            let screen = if action == DriverAction::StopLauncher {
                screen
            } else {
                DriverConfirmScreen::new(ALIAS.to_string(), action)
            };
            let mut t = Terminal::new(TestBackend::new(110, 40)).unwrap();
            t.draw(|f| screen.render(f, f.area(), &ctx)).unwrap();
            let buf = t.backend().buffer();
            let text: String = (0..40u16)
                .map(|y| {
                    (0..110u16)
                        .map(|x| buf[(x, y)].symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert!(text.contains(title), "{action:?}: title missing");
            assert!(text.contains(ALIAS), "{action:?}: alias missing");
            assert!(
                text.contains("[ Yes ]") && text.contains("[ No ]"),
                "{action:?}: buttons missing"
            );
            assert!(!text.contains("[y/n]"), "{action:?}: legacy hint remains");
        }
    }

    #[test]
    fn enter_on_the_default_focus_declines_and_dispatches_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        registry::record_opt_in(&mut ctx.config, ALIAS).expect("opt in");
        let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::ToggleOptIn);
        let action = screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::Pop));
        assert!(
            rx.try_recv().is_err(),
            "a stray Enter must dispatch nothing"
        );
    }

    #[test]
    fn clicking_no_declines_and_a_stray_click_is_inert() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        registry::record_opt_in(&mut ctx.config, ALIAS).expect("opt in");
        let mut screen = DriverConfirmScreen::new(ALIAS.to_string(), DriverAction::ToggleOptIn);
        let mut t = Terminal::new(TestBackend::new(110, 40)).unwrap();
        t.draw(|f| screen.render(f, f.area(), &ctx)).unwrap();
        let buf = t.backend().buffer();
        let text: String = (0..40u16)
            .map(|y| {
                (0..110u16)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        let at = |label: &str| {
            let (column, row) =
                crate::ui::screens::confirm_popup::locate(&text, label).expect("button drawn");
            MouseInput::Click {
                column,
                row,
                double: false,
            }
        };
        let stray = MouseInput::Click {
            column: 0,
            row: 0,
            double: false,
        };
        assert!(matches!(
            screen.handle_mouse(stray, &mut ctx),
            ScreenAction::None
        ));
        assert!(matches!(
            screen.handle_mouse(at("[ No ]"), &mut ctx),
            ScreenAction::Pop
        ));
        assert!(rx.try_recv().is_err(), "clicking No must dispatch nothing");
    }
}
