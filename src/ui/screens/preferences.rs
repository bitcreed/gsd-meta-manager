//! The manager-preferences screen, reached from the project overview with `G`
//! (todo 2026-09-23, global settings editor).
//!
//! Two kinds of "global" exist and they are kept visibly apart:
//!
//! * **Manager preferences** (this screen): `default_runtime` and
//!   `driver_max_concurrent` in the manager's own `config.json`. They apply
//!   to the manager across every project.
//! * **GSD global template** (`~/.gsd/defaults.json`): reached with `g` from
//!   here; it only seeds NEW projects, because gsd-core ignores it once a
//!   project has its own `.planning/config.json`.
//!
//! Every change is written immediately; there is no unsaved state.

use super::{AppContext, Screen, ScreenAction};
use crate::config::{save_config, Preferences, DEFAULT_RUNTIME_KEY};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use serde_json::Value;

/// Upper bound for the concurrency stepper: the shared 5h/7d quota makes
/// anything higher a footgun ([INFERRED] bound; the file may hold more).
pub(crate) const MAX_CONCURRENT_STEP: usize = 16;

const ROW_RUNTIME: usize = 0;
const ROW_CONCURRENT: usize = 1;
const ROWS: usize = 2;

#[derive(Default)]
pub struct PreferencesScreen {
    selected: usize,
}

impl PreferencesScreen {
    pub fn new() -> Self {
        Self::default()
    }
}

crate::ui::screens::adjudicate_screen!(
    PreferencesScreen,
    crate::ui::screens::RENDERS_NO_ATTACKER_INFLUENCED_IDENTITY,
    "Draws authored labels, the validated runtime name (`claude`/`codex`) or \
     a fixed 'invalid in file' marker, a number, and the config file path \
     through `render_for_terminal`. No project identity or project state.",
);

/// The runtime choices in cycle order; `None` is "unset" (falls back to Claude).
const RUNTIME_CYCLE: [Option<&str>; 3] = [None, Some("claude"), Some("codex")];

/// Step `default_runtime` through unset -> claude -> codex (or backwards).
/// An unrecognized value in the file is treated as "unset" for stepping, so a
/// keypress always lands on something valid.
pub(crate) fn step_runtime(prefs: &mut Preferences, forward: bool) {
    let current = prefs
        .extra
        .get(DEFAULT_RUNTIME_KEY)
        .and_then(Value::as_str)
        .and_then(|s| RUNTIME_CYCLE.iter().position(|c| *c == Some(s)))
        .unwrap_or(0);
    let n = RUNTIME_CYCLE.len();
    let next = if forward { (current + 1) % n } else { (current + n - 1) % n };
    match RUNTIME_CYCLE[next] {
        Some(name) => {
            prefs
                .extra
                .insert(DEFAULT_RUNTIME_KEY.to_string(), Value::from(name));
        }
        None => {
            prefs.extra.remove(DEFAULT_RUNTIME_KEY);
        }
    }
}

/// Step `driver_max_concurrent`, clamped to `1..=MAX_CONCURRENT_STEP`
/// (0 would deny every run — see the note on [`Preferences`]).
pub(crate) fn step_concurrency(prefs: &mut Preferences, up: bool) {
    let cur = prefs.driver_max_concurrent.max(1);
    prefs.driver_max_concurrent = if up {
        (cur + 1).min(MAX_CONCURRENT_STEP)
    } else {
        cur.saturating_sub(1).max(1)
    };
}

fn runtime_label(prefs: &Preferences) -> String {
    match prefs.extra.get(DEFAULT_RUNTIME_KEY) {
        None => "(unset: claude)".to_string(),
        Some(_) => match prefs.default_runtime() {
            Ok(Some(rt)) => format!("{rt:?}").to_lowercase(),
            _ => "(invalid in file; press an arrow to reset)".to_string(),
        },
    }
}

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
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected = (self.selected + 1).min(ROWS - 1);
                ScreenAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                ScreenAction::None
            }
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Char('h')
            | KeyCode::Char('l')
            | KeyCode::Char(' ')
            | KeyCode::Enter => {
                let forward = !matches!(code, KeyCode::Left | KeyCode::Char('h'));
                match self.selected {
                    ROW_RUNTIME => step_runtime(&mut ctx.config.preferences, forward),
                    ROW_CONCURRENT => step_concurrency(&mut ctx.config.preferences, forward),
                    _ => {}
                }
                let msg = match save_config(&ctx.config, &ctx.config_path) {
                    Ok(()) => "Manager preference saved".to_string(),
                    Err(e) => format!("Failed to save preferences: {e}"),
                };
                ctx.status_message = Some((msg, std::time::Instant::now()));
                ScreenAction::None
            }
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
        let path = crate::text::render_for_terminal(&ctx.config_path.display().to_string())
            .to_string();
        let prefs = &ctx.config.preferences;
        let sel = |row: usize| {
            if self.selected == row {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            }
        };
        let dim = Style::default().fg(Color::DarkGray);
        let lines = vec![
            Line::from(Span::styled(
                " MANAGER PREFERENCES: apply to this manager, for every project",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(format!(" File: {path}"), dim)),
            Line::from(""),
            Line::from(Span::styled(
                format!("  default_runtime         < {} >", runtime_label(prefs)),
                sel(ROW_RUNTIME),
            )),
            Line::from(Span::styled(
                format!("  driver_max_concurrent   < {} >", prefs.driver_max_concurrent),
                sel(ROW_CONCURRENT),
            )),
            Line::from(""),
            Line::from(Span::styled(
                " GSD GLOBAL TEMPLATE (~/.gsd/defaults.json)",
                Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                "  Seeds NEW projects only. Existing projects ignore it once they",
                dim,
            )),
            Line::from(Span::styled("  have their own .planning/config.json.  g = open editor", dim)),
            Line::from(""),
            Line::from(Span::styled(
                " j/k select   </> or Enter change (saved at once)   g template   Esc back",
                dim,
            )),
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

    #[test]
    fn runtime_cycles_through_unset_claude_codex_and_wraps() {
        let mut p = Preferences::default();
        step_runtime(&mut p, true);
        assert_eq!(p.extra[DEFAULT_RUNTIME_KEY], "claude");
        step_runtime(&mut p, true);
        assert_eq!(p.extra[DEFAULT_RUNTIME_KEY], "codex");
        step_runtime(&mut p, true);
        assert!(!p.extra.contains_key(DEFAULT_RUNTIME_KEY));
        step_runtime(&mut p, false);
        assert_eq!(p.extra[DEFAULT_RUNTIME_KEY], "codex");
    }

    #[test]
    fn invalid_runtime_in_file_is_reset_by_stepping() {
        let mut p = Preferences::default();
        p.extra.insert(DEFAULT_RUNTIME_KEY.into(), Value::from("gemini"));
        assert!(runtime_label(&p).contains("invalid"));
        step_runtime(&mut p, true);
        assert_eq!(p.default_runtime().unwrap().map(|r| format!("{r:?}")), Some("Claude".into()));
    }

    #[test]
    fn concurrency_never_leaves_one_to_cap() {
        let mut p = Preferences::default();
        step_concurrency(&mut p, false);
        assert_eq!(p.driver_max_concurrent, 1);
        p.driver_max_concurrent = 0;
        step_concurrency(&mut p, false);
        assert_eq!(p.driver_max_concurrent, 1);
        for _ in 0..40 {
            step_concurrency(&mut p, true);
        }
        assert_eq!(p.driver_max_concurrent, MAX_CONCURRENT_STEP);
    }

    #[test]
    fn keys_edit_and_persist_to_the_config_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut ctx = super::super::tests::ctx_with_aliases(&["a"]);
        ctx.config_path = dir.path().join("config.json");
        let mut s = PreferencesScreen::new();
        s.handle_key(KeyCode::Right, KeyModifiers::NONE, &mut ctx);
        s.handle_key(KeyCode::Char('j'), KeyModifiers::NONE, &mut ctx);
        s.handle_key(KeyCode::Right, KeyModifiers::NONE, &mut ctx);
        let saved = crate::config::load_config(&ctx.config_path).unwrap();
        assert_eq!(saved.preferences.extra[DEFAULT_RUNTIME_KEY], "claude");
        assert_eq!(saved.preferences.driver_max_concurrent, 2);
        assert!(matches!(
            s.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx),
            ScreenAction::Pop
        ));
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
    fn render_labels_both_scopes() {
        let ctx = super::super::tests::ctx_with_aliases(&["a"]);
        let backend = ratatui::backend::TestBackend::new(100, 14);
        let mut term = ratatui::Terminal::new(backend).unwrap();
        term.draw(|f| PreferencesScreen::new().render(f, f.area(), &ctx)).unwrap();
        let text: String = term
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|c| c.symbol())
            .collect();
        assert!(text.contains("MANAGER PREFERENCES"));
        assert!(text.contains("NEW projects only"));
        assert!(text.contains("driver_max_concurrent"));
    }
}
