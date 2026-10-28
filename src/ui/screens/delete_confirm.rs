use super::confirm_popup::{self, ConfirmOutcome, ConfirmPopup};
use super::{AppContext, Screen, ScreenAction};
use crate::config::save_config;
use crate::registry;
use crate::ui::mouse::MouseInput;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

pub struct DeleteConfirmScreen {
    pub alias: String,
    popup: ConfirmPopup,
}

impl DeleteConfirmScreen {
    pub fn new(alias: String) -> Self {
        Self {
            alias,
            popup: ConfirmPopup::default(),
        }
    }
}

crate::ui::screens::adjudicate_screen!(
    DeleteConfirmScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws the registry key of the project about to be unregistered into a \
     destructive [y/n] prompt, again into the removal toast, and again into \
     the live-run refusal. This is the highest-consequence identity render \
     in the tree: the operator confirms the name they READ, so a rendered \
     name that is not the key is a confirmation of a different thing than \
     was asked (T-21-21-01). Fixture state: the confirm prompt for a \
     registered hostile key.",
);

impl Screen for DeleteConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match self.popup.on_key(code) {
            ConfirmOutcome::Confirm => {
                do_remove_project(ctx, &self.alias);
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

    // The confirmation prompt below is deliberately UNCHANGED by the guard in
    // `do_remove_project` (CR-06). Its reassurance — "This only unregisters it —
    // project files are not deleted" — is true of every removal the guard now
    // permits, and restating the guard here would put the rule in a place that
    // cannot enforce it. The refusal is a message the user sees *after* pressing
    // `y`, which is where it belongs: this screen is reached from more than one
    // route, and only the removal function is on all of them.
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        // The dashboard the user was just looking at stays rendered, dimmed,
        // behind the popup (todo 2026-08-18: never an empty block).
        super::normal::NormalScreen::new().render(frame, area, ctx);
        confirm_popup::dim_background(frame, area);

        // Popup body for the delete confirmation.
        //
        // **The split, stated here so a maintainer reading the diff sees the
        // rule rather than the instance:** the value a HUMAN READS goes through
        // `crate::text::display_identity`; the value used for the removal, the
        // map keys and the config lookup — `self.alias`, untouched, in
        // `do_remove_project` below — stays RAW.
        //
        // This is the highest-consequence identity render in the tree
        // (T-21-21-01). The operator confirms the name they READ, so a rendered
        // name that is not the key is a confirmation of a different thing than
        // was asked. Measured: ratatui's buffer DROPS the zero-width bytes of a
        // legacy key, so `"gsd-\u{200b}run"` reads as `gsd-run` and can collide
        // with a real project of that name.
        //
        // Both halves, through the ONE composition (WR-05): a registry key an
        // older build accepted can carry a raw `ESC` as easily as a `U+200B`,
        // and this is the last thing a human reads before an irreversible act.
        // See `crate::ui::tests` for the census that keeps this true.
        let prompt = format!(
            "Remove \"{}\"? This only unregisters it \u{2014} project files are not deleted. [y/n]",
            crate::text::render_for_terminal(&self.alias)
        );
        self.popup.record(confirm_popup::render_confirm_popup(
            frame,
            area,
            "Remove project",
            vec![Line::from(Span::styled(
                confirm_popup::strip_key_hint(&prompt).to_string(),
                Style::default().fg(Color::Red),
            ))],
            Color::Red,
            self.popup.focus,
        ));
    }

    fn handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction {
        confirm_popup::click_as_key(&self.popup, input).map_or(ScreenAction::None, |code| {
            self.handle_key(code, KeyModifiers::NONE, ctx)
        })
    }

    fn name(&self) -> &str {
        "delete_confirm"
    }
}

fn do_remove_project(ctx: &mut AppContext, alias: &str) {
    // WR-03 (D-13): refuse to unregister a project whose launcher run still
    // holds its run lock. After an unregister the project drops out of
    // `launcher_markers_for`, so the Driven badge and the Driver tab vanish
    // while the detached launcher and its sandbox scope keep running. The
    // marker is the cached one (at most one slow tick stale, and `None` until
    // the first scan lands); like every driver surface it is gated on the
    // experimental flag, which is also the only time the scan populates it.
    let has_live_run = ctx.experimental
        && ctx
            .view_cache
            .get(alias)
            .and_then(|cache| cache.launcher_marker)
            .is_some_and(|marker| marker.any_live);
    if has_live_run {
        ctx.error_message = Some(format!(
            "'{}' has a live driver run. Stop it from its Driver tab (x) first.",
            crate::text::render_for_terminal(alias)
        ));
        ctx.needs_redraw = true;
        return;
    }

    let project_path = ctx.config.projects.get(alias).map(|p| p.path.clone());

    // Wrapped at the CALL, not stored: this is a registry key being looked up
    // and thrown away, which is exactly what `LegacyRegistryKey` is for
    // (`21-24`). The raw bytes still reach the removal through
    // `as_raw_for_lookup_only` inside `remove_project`, so a legacy row an
    // older build accepted is still removable from this screen (D-17-3); what
    // the type buys is that the FAILURE echo cannot be written raw.
    match registry::remove_project(
        &mut ctx.config,
        &registry::LegacyRegistryKey::from_argv(alias.to_string()),
    ) {
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

            // `registry::remove_project` cleans none of the maps above — it takes
            // a `&mut Config` and has no access to `AppContext` at all — so this
            // is the **immediate** path, and `App::prune_driver_maps` is the
            // backstop for every removal that does not come through this screen
            // (D-27).

            // Read, not looked up: every map removal above keys off the RAW
            // `alias`; only this toast is escaped, because only this toast is
            // read by a person.
            ctx.status_message = Some((
                format!("Removed \"{}\"", crate::text::render_for_terminal(alias)),
                std::time::Instant::now(),
            ));

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
    use std::path::Path;
    use tokio::sync::mpsc::UnboundedReceiver;

    const ALIAS: &str = "proj";

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

    /// Press `y` on the delete-confirmation screen for [`ALIAS`].
    fn confirm_removal(ctx: &mut AppContext) -> ScreenAction {
        let mut screen = DeleteConfirmScreen::new(ALIAS.to_string());
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, ctx)
    }

    #[test]
    fn unregistering_a_project_removes_it_with_no_refusal() {
        // Phase 31: the TUI can no longer observe a legacy `drive` run, so the
        // live-run refusal that used to guard this path has no input and is gone.
        // This pins the plain path: removal succeeds and reports no error.
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());

        confirm_removal(&mut ctx);

        assert!(
            !ctx.config.projects.contains_key(ALIAS),
            "a registered project must be removable"
        );
        assert_eq!(
            ctx.error_message, None,
            "an ordinary removal reports no error, got: {:?}",
            ctx.error_message
        );
    }

    /// A view cache entry carrying only the given launcher marker.
    fn cache_with_marker(
        marker: crate::ui::screens::driver_run::LauncherMarker,
    ) -> crate::ui::screens::ProjectViewCache {
        crate::ui::screens::ProjectViewCache {
            launcher_marker: Some(marker),
            ..Default::default()
        }
    }

    #[test]
    fn unregistering_a_project_with_a_live_launcher_run_is_refused() {
        use crate::ui::screens::driver_run::{LauncherMarker, MarkerClass};

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        ctx.view_cache.insert(
            ALIAS.to_string(),
            cache_with_marker(LauncherMarker {
                class: Some(MarkerClass::Driven),
                any_live: true,
            }),
        );

        let act = confirm_removal(&mut ctx);

        assert!(matches!(act, ScreenAction::Pop));
        assert!(
            ctx.config.projects.contains_key(ALIAS),
            "a project with a live launcher run must stay registered"
        );
        let msg = ctx.error_message.as_deref().expect("a refusal is reported");
        assert!(msg.contains("has a live driver run"), "got: {msg}");
        assert!(msg.contains(ALIAS), "the refusal names the alias: {msg}");
        assert!(
            !ctx.config_path.exists(),
            "a refused removal must not write the config"
        );
    }

    #[test]
    fn a_project_whose_launcher_runs_are_not_live_is_still_removed() {
        use crate::ui::screens::driver_run::{LauncherMarker, MarkerClass};

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        ctx.view_cache.insert(
            ALIAS.to_string(),
            cache_with_marker(LauncherMarker {
                class: Some(MarkerClass::Crashed),
                any_live: false,
            }),
        );

        confirm_removal(&mut ctx);

        assert!(
            !ctx.config.projects.contains_key(ALIAS),
            "a crashed run holds no lock, so the project is removable"
        );
        assert_eq!(ctx.error_message, None);
        let (status, _) = ctx
            .status_message
            .as_ref()
            .expect("a removal reports what it did");
        assert!(status.contains("Removed \"proj\""), "got: {status}");
    }

    #[test]
    fn folder_presence_unregistering_a_project_whose_folder_is_gone_removes_it() {
        // quick-260922-hdh: a red "(missing)" row exists so the user can find
        // the stale entry and remove it. The removal must not need the folder.
        use crate::state_reader::{parse_project_state, ProjectPresence};

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let vanished = dir.path().join("vanished");
        ctx.config.projects.get_mut(ALIAS).unwrap().path = vanished.clone();
        ctx.project_states.insert(
            ALIAS.to_string(),
            parse_project_state(&vanished.join(".planning")),
        );
        assert_eq!(
            ctx.project_states[ALIAS].presence,
            ProjectPresence::FolderMissing,
            "precondition: the project's folder reads as gone"
        );

        confirm_removal(&mut ctx);

        assert!(
            !ctx.config.projects.contains_key(ALIAS),
            "a project whose folder is gone must be removable from the registry"
        );
        assert!(
            !ctx.project_states.contains_key(ALIAS),
            "and its cached state must go with it"
        );
        assert_eq!(ctx.error_message, None);
        let (status, _) = ctx
            .status_message
            .as_ref()
            .expect("a removal reports what it did");
        assert!(status.contains("Removed"), "got: {status}");
    }

    fn render_rows(screen: &DeleteConfirmScreen, ctx: &AppContext) -> Vec<String> {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| screen.render(f, f.area(), ctx)).unwrap();
        let buf = t.backend().buffer();
        (0..30u16)
            .map(|y| {
                (0..100u16)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }

    #[test]
    fn delete_confirmation_is_a_popup_with_buttons_over_the_dashboard() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (ctx, _rx) = ctx_with_project(dir.path());
        let rows = render_rows(&DeleteConfirmScreen::new(ALIAS.to_string()), &ctx);
        let text = rows.join("\n");
        assert!(text.contains("Remove project"), "popup title missing");
        assert!(text.contains("Remove \"proj\"?"), "prompt missing");
        // The body wraps inside the popup, so assert on a fragment that cannot
        // straddle a line break at this width.
        assert!(text.contains("unregisters"), "consequence missing");
        assert!(text.contains("[ Yes ]") && text.contains("[ No ]"));
        assert!(!text.contains("[y/n]"), "legacy footer hint must be gone");
        // The dashboard is still behind the popup: the registered alias appears
        // outside the popup line too (project row), not an empty block.
        assert!(
            rows.iter().filter(|r| r.contains("proj")).count() >= 2,
            "the project list must remain rendered behind the popup"
        );
    }

    #[test]
    fn enter_defaults_to_no_and_focus_can_move_to_yes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());

        // Stray Enter is safe: No is focused, nothing is removed.
        let mut screen = DeleteConfirmScreen::new(ALIAS.to_string());
        let act = screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(act, ScreenAction::Pop));
        assert!(ctx.config.projects.contains_key(ALIAS));

        // Tab moves focus to Yes without acting; Enter then confirms.
        let mut screen = DeleteConfirmScreen::new(ALIAS.to_string());
        let act = screen.handle_key(KeyCode::Tab, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(act, ScreenAction::None));
        assert!(ctx.config.projects.contains_key(ALIAS));
        let act = screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(act, ScreenAction::Pop));
        assert!(!ctx.config.projects.contains_key(ALIAS));
    }

    #[test]
    fn n_and_esc_cancel_without_removing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        for key in [KeyCode::Char('n'), KeyCode::Esc] {
            let mut screen = DeleteConfirmScreen::new(ALIAS.to_string());
            let act = screen.handle_key(key, KeyModifiers::NONE, &mut ctx);
            assert!(matches!(act, ScreenAction::Pop));
            assert!(ctx.config.projects.contains_key(ALIAS));
        }
    }

    fn click_at(
        screen: &mut DeleteConfirmScreen,
        ctx: &mut AppContext,
        label: &str,
    ) -> ScreenAction {
        let text = render_rows(screen, ctx).join("\n");
        let (column, row) =
            crate::ui::screens::confirm_popup::locate(&text, label).expect("button drawn");
        screen.handle_mouse(
            MouseInput::Click {
                column,
                row,
                double: false,
            },
            ctx,
        )
    }

    #[test]
    fn clicking_yes_removes_and_clicking_no_or_elsewhere_does_not() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());

        let mut screen = DeleteConfirmScreen::new(ALIAS.to_string());
        let act = click_at(&mut screen, &mut ctx, "[ No ]");
        assert!(matches!(act, ScreenAction::Pop));
        assert!(ctx.config.projects.contains_key(ALIAS));

        // A click away from the buttons is inert.
        let mut screen = DeleteConfirmScreen::new(ALIAS.to_string());
        render_rows(&screen, &ctx);
        let act = screen.handle_mouse(
            MouseInput::Click {
                column: 0,
                row: 0,
                double: false,
            },
            &mut ctx,
        );
        assert!(matches!(act, ScreenAction::None));
        assert!(ctx.config.projects.contains_key(ALIAS));

        let act = click_at(&mut screen, &mut ctx, "[ Yes ]");
        assert!(matches!(act, ScreenAction::Pop));
        assert!(!ctx.config.projects.contains_key(ALIAS));
    }
}
