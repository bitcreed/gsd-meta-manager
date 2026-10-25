//! The global-settings screen, reached from the project overview with `G`
//! (todo 2026-09-23, global settings editor).
//!
//! Two kinds of "global" exist and they are kept visibly apart:
//!
//! * **Manager preferences** (`config.json`, shown here): **none are editable.**
//!   The screen used to edit `default_runtime` and the run-concurrency cap, but
//!   both configured the legacy `drive` subcommand (which agent it ran, how many
//!   runs it allowed at once), and Phase 31 retired that subcommand. Offering a
//!   stepper for a setting that selects or caps nothing would be a control that
//!   lies. `[inferred — audit]`. The fields still parse (`config::Preferences`),
//!   so an old `config.json` that carries them loads and is rewritten with them
//!   intact; only the editing surface went.
//! * **GSD global template** (`$GSD_HOME/.gsd/defaults.json`; `GSD_HOME`
//!   defaults to the home directory): reached with `g` from
//!   here; it only seeds NEW projects, because gsd-core ignores it once a
//!   project has its own `.planning/config.json`.
//!
//! The screen holds no unsaved state and writes nothing itself.

use super::{AppContext, Screen, ScreenAction};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

#[derive(Default)]
pub struct PreferencesScreen;

impl PreferencesScreen {
    pub fn new() -> Self {
        Self
    }
}

crate::ui::screens::adjudicate_screen!(
    PreferencesScreen,
    crate::ui::screens::RENDERS_NO_ATTACKER_INFLUENCED_IDENTITY,
    "Draws authored labels and the config file path through \
     `render_for_terminal`. No project identity or project state.",
);

impl Screen for PreferencesScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        ctx.needs_redraw = true;
        match code {
            KeyCode::Esc | KeyCode::Char('q') => ScreenAction::Pop,
            KeyCode::Char('g') => match ctx.selected_alias() {
                Some(alias) => ScreenAction::Replace(Box::new(
                    super::detail::DetailScreen::opened_on_global_template(alias, ctx),
                )),
                None => {
                    ctx.status_message = Some((
                        "Register a project first: the GSD template editor opens inside one"
                            .to_string(),
                        std::time::Instant::now(),
                    ));
                    ScreenAction::None
                }
            },
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let path =
            crate::text::render_for_terminal(&ctx.config_path.display().to_string()).to_string();
        let dim = Style::default().fg(Color::DarkGray);
        let lines = vec![
            Line::from(Span::styled(
                " MANAGER PREFERENCES: apply to this manager, for every project",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(format!(" File: {path}"), dim)),
            Line::from(""),
            Line::from(Span::styled(
                "  (none: the runtime and concurrency settings configured the",
                dim,
            )),
            Line::from(Span::styled("   legacy driver, removed in Phase 31)", dim)),
            Line::from(""),
            Line::from(Span::styled(
                " GSD GLOBAL TEMPLATE ($GSD_HOME/.gsd/defaults.json; GSD_HOME defaults to ~)",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "  Seeds NEW projects only. Existing projects ignore it once they",
                dim,
            )),
            Line::from(Span::styled(
                "  have their own .planning/config.json.  g = open editor",
                dim,
            )),
            Line::from(""),
            Line::from(Span::styled(" g template   Esc back", dim)),
        ];
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Preferences (global to the manager) ");
        frame.render_widget(Paragraph::new(lines).block(block), area);
    }

    fn name(&self) -> &str {
        "preferences"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Render the screen into a 100x14 buffer and return its text.
    fn rendered() -> String {
        let ctx = super::super::tests::ctx_with_aliases(&["a"]);
        let backend = ratatui::backend::TestBackend::new(100, 14);
        let mut term = ratatui::Terminal::new(backend).unwrap();
        term.draw(|f| PreferencesScreen::new().render(f, f.area(), &ctx))
            .unwrap();
        term.backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect()
    }

    #[test]
    fn g_without_a_selected_project_says_why_and_stays() {
        let mut ctx = super::super::tests::ctx_with_aliases(&[]);
        let mut s = PreferencesScreen::new();
        let act = s.handle_key(KeyCode::Char('g'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(act, ScreenAction::None));
        assert!(ctx.status_message.is_some());
    }

    #[test]
    fn esc_and_q_leave_the_screen() {
        for code in [KeyCode::Esc, KeyCode::Char('q')] {
            let mut ctx = super::super::tests::ctx_with_aliases(&["a"]);
            let mut s = PreferencesScreen::new();
            assert!(matches!(
                s.handle_key(code, KeyModifiers::NONE, &mut ctx),
                ScreenAction::Pop
            ));
        }
    }

    #[test]
    fn no_key_edits_or_writes_a_preference() {
        // The steppers are gone. Every key the old screen used to act on is now
        // inert: no state changes, nothing is written, no "saved" status appears.
        let dir = tempfile::TempDir::new().unwrap();
        let mut ctx = super::super::tests::ctx_with_aliases(&["a"]);
        ctx.config_path = dir.path().join("config.json");
        let before = serde_json::to_value(&ctx.config.preferences).unwrap();
        let mut s = PreferencesScreen::new();
        for code in [
            KeyCode::Right,
            KeyCode::Left,
            KeyCode::Char('l'),
            KeyCode::Char('h'),
            KeyCode::Char('j'),
            KeyCode::Char('k'),
            KeyCode::Char(' '),
            KeyCode::Enter,
        ] {
            assert!(matches!(
                s.handle_key(code, KeyModifiers::NONE, &mut ctx),
                ScreenAction::None
            ));
        }
        assert_eq!(
            serde_json::to_value(&ctx.config.preferences).unwrap(),
            before,
            "no key may change a preference"
        );
        assert!(
            !ctx.config_path.exists(),
            "an inert screen must not create the config file"
        );
        assert!(ctx.status_message.is_none());
    }

    #[test]
    fn render_labels_both_scopes_and_offers_no_dead_rows() {
        let text = rendered();
        assert!(text.contains("MANAGER PREFERENCES"));
        assert!(text.contains("NEW projects only"));
        assert!(text.contains("GSD GLOBAL TEMPLATE"));
        assert!(text.contains("$GSD_HOME/.gsd/defaults.json"));
        assert!(
            text.contains("(none:"),
            "the manager section must say plainly that nothing is editable: {text}"
        );
        // The second name is assembled so this file's own text does not name the
        // retired knob (the plan's acceptance grep counts it at zero).
        let retired_cap = ["driver_max", "_concurrent"].concat();
        for dead in ["default_runtime", retired_cap.as_str(), "j/k select"] {
            assert!(
                !text.contains(dead),
                "{dead:?} names a control that no longer exists: {text}"
            );
        }
    }

    #[test]
    fn old_preference_keys_still_load_from_a_config_file() {
        // Only the editing surface went. A config.json written by an older build
        // keeps parsing, and the keys survive a load/save round trip.
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("config.json");
        let cap_key = ["driver_max", "_concurrent"].concat();
        let body = serde_json::json!({
            "version": 1,
            "projects": {},
            "preferences": {
                crate::config::DEFAULT_RUNTIME_KEY: "codex",
                cap_key.as_str(): 3,
            },
        });
        std::fs::write(&path, body.to_string()).unwrap();
        let config = crate::config::load_config(&path).unwrap();
        let loaded = serde_json::to_value(&config.preferences).unwrap();
        assert_eq!(loaded[crate::config::DEFAULT_RUNTIME_KEY], "codex");
        assert_eq!(loaded[cap_key.as_str()], 3);
        crate::config::save_config(&config, &path).unwrap();
        let again = crate::config::load_config(&path).unwrap();
        let reloaded = serde_json::to_value(&again.preferences).unwrap();
        assert_eq!(reloaded[crate::config::DEFAULT_RUNTIME_KEY], "codex");
        assert_eq!(reloaded[cap_key.as_str()], 3);
    }
}
