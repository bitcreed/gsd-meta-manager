use super::{AppContext, Screen, ScreenAction};
use crate::state_reader::{self, queue_md};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct QueueDeleteConfirmScreen {
    pub alias: String,
    pub index: usize,
    pub command_text: String,
}

impl QueueDeleteConfirmScreen {
    pub fn new(alias: String, index: usize, command_text: String) -> Self {
        Self {
            alias,
            index,
            command_text,
        }
    }
}

crate::ui::screens::adjudicate_screen!(
    QueueDeleteConfirmScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints its body with `DetailScreen::render_main_only`, and its \
     destructive [y/n] footer draws the queued command text read from the \
     project's `.planning/queue.md`. Fixture states: one per sub-view.",
);

impl Screen for QueueDeleteConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Char('y') => {
                if let Some(project) = ctx.config.projects.get(&self.alias) {
                    let planning_dir = project.path.join(".planning");
                    let mut actions = queue_md::load_queue(&planning_dir);
                    if self.index < actions.len() {
                        actions.remove(self.index);
                        if let Err(e) = queue_md::save_queue(&planning_dir, &actions) {
                            ctx.status_message =
                                Some((format!("Queue error: {}", e), std::time::Instant::now()));
                        } else {
                            ctx.status_message = Some((
                                format!("Removed: {}", self.command_text),
                                std::time::Instant::now(),
                            ));
                            // Reload project state
                            let new_state = state_reader::parse_project_state(&planning_dir);
                            ctx.project_states.insert(self.alias.clone(), new_state);
                            // Clamp queue_selected
                            let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                            let new_len = actions.len();
                            if new_len == 0 {
                                cache.queue_selected = 0;
                            } else if cache.queue_selected >= new_len {
                                cache.queue_selected = new_len - 1;
                            }
                        }
                    }
                }
                ctx.needs_redraw = true;
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
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        let footer_area = chunks[1];

        // Render the detail view content in the background
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, chunks[0], ctx);

        // Red confirmation prompt in footer.
        let line = Line::from(Span::styled(
            prompt_text(&self.command_text),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(line), footer_area);
    }

    fn name(&self) -> &str {
        "queue_delete_confirm"
    }
}

/// The exact string the destructive confirm footer draws.
///
/// **Extracted so the two-direction conversion pin can drive it** (WR-05), in
/// the same shape `driver_confirm::prompt_text` already has: a `Screen::render`
/// needs a `Frame` and an `AppContext`, and the tree's only `AppContext` fixture
/// is `pub(super)` inside `ui::screens::tests`, so a prompt built inline in
/// `render` is not reachable from `crate::ui`'s census module. Nothing about the
/// behaviour changed in the extraction; the escape, the cap and the truncation
/// are carried across verbatim.
///
/// The split: `command_text` is only ever RENDERED — the removal in `handle_key`
/// is by `self.index` into the queue file, never by this string — so escaping it
/// changes nothing about what is deleted. It is read from the project's
/// `.planning/queue.md`, which is third-party text under SAFE-07, and it is the
/// name in a destructive [y/n] prompt.
///
/// Escape BEFORE truncating, and truncate by `char` rather than by byte:
/// `&s[..47]` panics when byte 47 is not a char boundary, and this string comes
/// off disk. That was a reachable panic — a denial of service driven by a file
/// the tool does not own — for as long as the slice was written that way.
///
/// Both halves, through the ONE composition (WR-05). This site is NOT an
/// `Into<Cow>` sink — it measures and truncates the escaped form by `char`
/// before it becomes a prompt — so the `Rendered` is taken into a `String`
/// through the `From<Rendered> for String` that `text.rs` provides for exactly
/// this. That is the carrier's documented trait surface, not a `.to_string()`
/// workaround. The census in `crate::ui::tests` is what keeps the composition
/// true of this file.
pub(crate) fn prompt_text(command_text: &str) -> String {
    const CAP: usize = 50;
    const KEEP: usize = 47;
    let escaped: String = crate::text::render_for_terminal(command_text).into();
    let display_text = if escaped.chars().count() > CAP {
        format!("{}...", escaped.chars().take(KEEP).collect::<String>())
    } else {
        escaped
    };
    format!("  Remove \"{}\" from queue? [y/n]", display_text)
}
