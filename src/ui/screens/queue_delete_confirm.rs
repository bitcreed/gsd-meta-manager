use super::{AppContext, Screen, ScreenAction};
use crate::state_reader::{self, queue_md};
use crossterm::event::{KeyCode, KeyModifiers};
use super::confirm_popup::{self, ConfirmOutcome, ConfirmPopup};
use crate::ui::mouse::MouseInput;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

pub struct QueueDeleteConfirmScreen {
    pub alias: String,
    pub index: usize,
    pub command_text: String,
    popup: ConfirmPopup,
}

impl QueueDeleteConfirmScreen {
    pub fn new(alias: String, index: usize, command_text: String) -> Self {
        Self {
            alias,
            index,
            command_text,
            popup: ConfirmPopup::default(),
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
        match self.popup.on_key(code) {
            ConfirmOutcome::Confirm => {
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
        // Render the detail view content in the background, dimmed, with the
        // confirmation as a modal popup over it.
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, area, ctx);
        confirm_popup::dim_background(frame, area);

        self.popup.record(confirm_popup::render_confirm_popup(
            frame,
            area,
            "Remove from queue",
            vec![Line::from(Span::styled(
                confirm_popup::strip_key_hint(&prompt_text(&self.command_text)).to_string(),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ))],
            Color::Red,
            self.popup.focus,
        ));
    }

    fn handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction {
        confirm_popup::click_as_key(&self.popup, input)
            .map_or(ScreenAction::None, |code| self.handle_key(code, KeyModifiers::NONE, ctx))
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn rows(screen: &QueueDeleteConfirmScreen, ctx: &AppContext) -> String {
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| screen.render(f, f.area(), ctx)).unwrap();
        let buf = t.backend().buffer();
        (0..30u16)
            .map(|y| (0..100u16).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn queue_delete_renders_a_popup_with_buttons() {
        let ctx = crate::ui::screens::tests::ctx_with_aliases(&["demo"]);
        let screen =
            QueueDeleteConfirmScreen::new("demo".into(), 0, "/gsd:execute-phase 21".into());
        let text = rows(&screen, &ctx);
        assert!(text.contains("Remove from queue"));
        assert!(text.contains("Remove \"/gsd:execute-phase 21\" from queue?"));
        assert!(text.contains("[ Yes ]") && text.contains("[ No ]"));
        assert!(!text.contains("[y/n]"));
    }

    #[test]
    fn queue_delete_enter_defaults_to_cancel() {
        let mut ctx = crate::ui::screens::tests::ctx_with_aliases(&["demo"]);
        let mut screen = QueueDeleteConfirmScreen::new("demo".into(), 0, "x".into());
        let act = screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(act, ScreenAction::Pop));
        assert!(ctx.status_message.is_none(), "No-by-default Enter must not delete");
    }

    #[test]
    fn clicking_a_button_acts_and_a_stray_click_does_not() {
        let mut ctx = crate::ui::screens::tests::ctx_with_aliases(&["demo"]);
        let mut screen = QueueDeleteConfirmScreen::new("demo".into(), 0, "x".into());
        let text = rows(&screen, &ctx);
        let click = |text: &str, label: &str| {
            let (column, row) = confirm_popup::locate(text, label).expect("button drawn");
            MouseInput::Click { column, row, double: false }
        };
        let act = screen.handle_mouse(
            MouseInput::Click { column: 0, row: 0, double: false },
            &mut ctx,
        );
        assert!(matches!(act, ScreenAction::None));
        let act = screen.handle_mouse(click(&text, "[ No ]"), &mut ctx);
        assert!(matches!(act, ScreenAction::Pop));
        assert!(ctx.status_message.is_none(), "No must not delete");
        let act = screen.handle_mouse(click(&text, "[ Yes ]"), &mut ctx);
        assert!(matches!(act, ScreenAction::Pop), "Yes confirms (the aliased project has no queue to edit)");
    }
}
