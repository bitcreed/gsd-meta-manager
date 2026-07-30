use super::{AppContext, Screen, ScreenAction};
use crate::config::save_config;
use crate::registry;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct DeleteConfirmScreen {
    pub alias: String,
}

impl DeleteConfirmScreen {
    pub fn new(alias: String) -> Self {
        Self { alias }
    }
}

impl Screen for DeleteConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Char('y') => {
                do_remove_project(ctx, &self.alias);
                ScreenAction::Pop
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            _ => ScreenAction::None,
        }
    }

    // The confirmation prompt below is deliberately UNCHANGED by the guard in
    // `do_remove_project` (CR-06). Its reassurance — "This only unregisters it —
    // project files are not deleted" — is true of every removal the guard now
    // permits, and restating the guard here would put the rule in a place that
    // cannot enforce it. The refusal is a message the user sees *after* pressing
    // `y`, which is where it belongs: this screen is reached from more than one
    // route, and only the removal function is on all of them.
    fn render(&self, frame: &mut Frame, area: Rect, _ctx: &AppContext) {
        let chunks = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);

        // Render project list background
        use ratatui::widgets::{Block, Borders};
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" GSD Manager ");
        frame.render_widget(block, chunks[0]);

        // Footer with delete confirmation
        let prompt = format!(
            "Remove \"{}\"? This only unregisters it \u{2014} project files are not deleted. [y/n]",
            self.alias
        );
        let line = Line::from(Span::styled(prompt, Style::default().fg(Color::Red)));
        frame.render_widget(Paragraph::new(line), chunks[1]);
    }

    fn name(&self) -> &str {
        "delete_confirm"
    }
}

fn do_remove_project(ctx: &mut AppContext, alias: &str) {
    // **The first statement of the body, before anything is read or removed
    // (CR-06, D-11, D-27).**
    //
    // *Why it lives here and not in the key handler.* Every route into removal
    // goes through this function; a guard in the handler would be one branch's
    // worth of protection. The failure it prevents — an autonomous agent with
    // git and push rights left running in a project the tool has forgotten, with
    // no path back to it in the TUI — is not a failure worth protecting one
    // branch from.
    //
    // *Why `Unknown` is refused as well as `Alive`.* An undeterminable liveness
    // is precisely the case where the tool cannot promise the agent is finished,
    // and a removal is irreversible from inside the TUI: `reconcile_all` iterates
    // only `config.projects`, so from the next tick the run is invisible, the `x`
    // key refuses with "No run is being observed", and the driver keeps its
    // `flock` for as long as it lives. Refusing on the uncertain answer costs the
    // user one keystroke; permitting it costs them a process they cannot find.
    // Only a positively `Dead` run is nothing left to abandon.
    if let Some(run) = ctx.observed_runs.get(alias) {
        if run.liveness != crate::driver::liveness::Liveness::Dead {
            ctx.error_message = Some(format!(
                "'{alias}' still has a driver run ({}) that is not known to be \
                 finished. Press 'x' to stop it first — unregistering now would \
                 leave it running with no way back to it from here",
                run.run_id
            ));
            ctx.needs_redraw = true;
            return;
        }
    }

    let project_path = ctx.config.projects.get(alias).map(|p| p.path.clone());

    match registry::remove_project(&mut ctx.config, alias) {
        Ok(()) => {
            if let Err(e) = save_config(&ctx.config, &ctx.config_path) {
                ctx.error_message = Some(format!("Failed to save config: {}", e));
                ctx.needs_redraw = true;
                return;
            }

            // Unwatch the project's .planning/ directory
            if let Some(ref path) = project_path {
                if let Some(ref mut watcher) = ctx.watcher {
                    let planning_dir = path.join(".planning");
                    let _ = watcher.unwatch(&planning_dir);
                }
            }

            ctx.project_states.remove(alias);
            ctx.detail_sub_view_per_project.remove(alias);
            ctx.last_refresh.remove(alias);

            // The driver maps join the three above (D-27). `registry::remove_project`
            // cleans none of them — it takes a `&mut Config` and has no access to
            // `AppContext` at all — so this is the **immediate** path, and
            // `App::prune_driver_maps` is the backstop for every removal that
            // does not come through this screen. `journal_cursors` is keyed
            // `(alias, run_id)`, so it is filtered rather than removed by key.
            ctx.run_states.remove(alias);
            ctx.observed_runs.remove(alias);
            ctx.journal_cursors
                .retain(|(cursor_alias, _), _| cursor_alias != alias);

            ctx.status_message =
                Some((format!("Removed \"{}\"", alias), std::time::Instant::now()));

            ctx.recompute_filtered_aliases();
            if ctx.filtered_aliases.is_empty() {
                ctx.table_state.select(None);
            } else {
                let selected = ctx
                    .table_state
                    .selected()
                    .unwrap_or(0)
                    .min(ctx.filtered_aliases.len() - 1);
                ctx.table_state.select(Some(selected));
            }
            ctx.needs_redraw = true;
        }
        Err(e) => {
            ctx.error_message = Some(e.to_string());
            ctx.needs_redraw = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::config::{Config, RegisteredProject};
    use crate::driver::liveness::Liveness;
    use crate::driver::reconcile::ObservedRun;
    use std::path::Path;
    use tokio::sync::mpsc::UnboundedReceiver;

    const ALIAS: &str = "proj";
    const RUN_ID: &str = "2026-07-29T12-00-00Z-aaaa";

    /// An `AppContext` with one registered project, a live event channel, and a
    /// config path inside `root` so `save_config` is a **real write** and the
    /// removal path is exercised end to end.
    ///
    /// Copied from `driver_confirm.rs`'s: the full `AppContext` literal is this
    /// module's convention, and a constructor shortcut does not exist.
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

    /// An `ObservedRun` with only the field these assertions read varied.
    fn observed(alias: &str, run_id: &str, liveness: Liveness) -> ObservedRun {
        ObservedRun {
            alias: alias.to_string(),
            run_id: run_id.to_string(),
            pid: 4242,
            pgid: 4242,
            started_at: "2026-07-29T12:00:00Z".to_string(),
            goal: "ship it".to_string(),
            gsd_command: "/gsd-progress".to_string(),
            liveness,
        }
    }

    /// Press `y` on the delete-confirmation screen for [`ALIAS`].
    fn confirm_removal(ctx: &mut AppContext) -> ScreenAction {
        let mut screen = DeleteConfirmScreen::new(ALIAS.to_string());
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, ctx)
    }

    #[test]
    fn unregistering_a_project_with_a_live_run_is_refused_and_leaves_every_map_intact() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        ctx.observed_runs
            .insert(ALIAS.to_string(), observed(ALIAS, RUN_ID, Liveness::Alive));

        let action = confirm_removal(&mut ctx);
        assert!(matches!(action, ScreenAction::Pop));

        assert!(
            ctx.config.projects.contains_key(ALIAS),
            "the registry entry must survive. Removing it here abandons a running \
             autonomous agent with git and push rights: the next reconciliation \
             scan iterates only config.projects, so the run becomes invisible, the \
             stop key refuses with 'No run is being observed', and the driver \
             keeps its flock for as long as it lives (CR-06, D-11)"
        );
        assert!(
            ctx.observed_runs.contains_key(ALIAS),
            "the sibling maps must survive too — a refusal that half-removed the \
             project would leave the TUI in a state neither the guard nor D-27's \
             cleanup describes"
        );

        let refusal = ctx
            .error_message
            .as_deref()
            .expect("the refusal must be visible, not silent");
        assert!(refusal.contains(ALIAS), "got: {refusal}");
        assert!(
            refusal.contains('x'),
            "the refusal must name the key that fixes it, or the user is told no \
             and not told what to do, got: {refusal}"
        );
    }

    #[test]
    fn unregistering_a_project_whose_liveness_is_undeterminable_is_refused_too() {
        // The case that cannot be reached at runtime on Linux, which is exactly
        // why it is asserted directly against the field rather than through a
        // probe. An undeterminable liveness is where the tool CANNOT promise the
        // agent is finished, and an irreversible removal is the wrong answer to
        // "I do not know" (CR-05, CR-06).
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        ctx.observed_runs
            .insert(ALIAS.to_string(), observed(ALIAS, RUN_ID, Liveness::Unknown));

        confirm_removal(&mut ctx);

        assert!(
            ctx.config.projects.contains_key(ALIAS),
            "an undeterminable liveness must be refused exactly like a live one"
        );
        assert!(ctx.error_message.is_some());
    }

    #[test]
    fn unregistering_a_project_with_no_run_still_removes_it() {
        // **The control arm, and it is not optional.** A blanket refusal would
        // pass both tests above while breaking the feature outright, and nothing
        // else here would notice.
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());

        confirm_removal(&mut ctx);

        assert!(
            !ctx.config.projects.contains_key(ALIAS),
            "a project with no observed run must still be removable"
        );
        assert_eq!(
            ctx.error_message, None,
            "an ordinary removal reports no error, got: {:?}",
            ctx.error_message
        );
    }

    #[test]
    fn unregistering_a_project_whose_run_crashed_still_removes_it() {
        // The second control arm: a crashed run is not something to abandon,
        // because there is nothing left running. Refusing on it would leave a
        // project permanently unremovable after one crash, with no way out but
        // editing config.json by hand.
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        ctx.observed_runs
            .insert(ALIAS.to_string(), observed(ALIAS, RUN_ID, Liveness::Dead));

        confirm_removal(&mut ctx);

        assert!(
            !ctx.config.projects.contains_key(ALIAS),
            "a crashed run must not block a removal"
        );
        assert_eq!(ctx.error_message, None);
        assert!(
            !ctx.observed_runs.contains_key(ALIAS),
            "and the sibling maps are still cleaned on the removal path (D-27)"
        );
    }
}
