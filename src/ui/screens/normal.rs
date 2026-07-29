use super::add_project::AddProjectScreen;
use super::create_project::CreateProjectScreen;
use super::delete_confirm::DeleteConfirmScreen;
use super::detail::DetailScreen;
use super::help::HelpScreen;
use super::{AppContext, Screen, ScreenAction};
use crate::app::{classify_status, format_phase_display, StatusCategory};
use crate::state_reader::disk_status::DiskStatus;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

pub struct NormalScreen {
    pub searching: bool,
}

impl Default for NormalScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl NormalScreen {
    pub fn new() -> Self {
        Self { searching: false }
    }
}

fn status_color(status: &str) -> Color {
    match classify_status(status) {
        StatusCategory::Active => Color::Green,
        StatusCategory::Idle => Color::Yellow,
        StatusCategory::Blocked => Color::Red,
        StatusCategory::Complete => Color::DarkGray,
        StatusCategory::Unknown => Color::Magenta,
    }
}

/// Return the DiskStatus one step below the given threshold.
fn prev_status(threshold: DiskStatus) -> DiskStatus {
    match threshold {
        DiskStatus::Discussed => DiskStatus::Empty,
        DiskStatus::Researched => DiskStatus::Discussed,
        DiskStatus::Planned => DiskStatus::Researched,
        DiskStatus::Partial => DiskStatus::Planned,
        DiskStatus::Complete => DiskStatus::Partial,
        _ => DiskStatus::NoDirectory,
    }
}

/// Select the single leading badge for a dashboard alias cell.
///
/// Badge priority (Phase 14 UI-SPEC, `### Badge Priority Rule`):
/// pause > external-job-waiting > active session. At most one badge ever
/// renders, so the alias column stays aligned. The glyph is always a fixed
/// `&'static str` — never text derived from a HANDOFF file, so no handoff
/// body can leak onto the dashboard row.
fn alias_badge(
    is_paused: bool,
    external_job_waiting: bool,
    has_session: bool,
) -> Option<(&'static str, Color)> {
    if is_paused {
        // Pause badge takes priority over all other indicators
        Some(("\u{23F8} ", Color::Cyan))
    } else if external_job_waiting {
        // Hourglass: waiting on an async job, not stuck
        Some(("\u{23F3} ", Color::Yellow))
    } else if has_session {
        Some(("\u{25b6} ", Color::Green))
    } else {
        None
    }
}

/// Render the compact D-R-P-E-V pipeline for unfocused dashboard rows.
fn compact_pipeline(status: &DiskStatus) -> Line<'static> {
    let stages: [(&str, DiskStatus); 5] = [
        ("D", DiskStatus::Discussed),
        ("R", DiskStatus::Researched),
        ("P", DiskStatus::Planned),
        ("E", DiskStatus::Partial),
        ("V", DiskStatus::Complete),
    ];

    // Build `D  R  P  E  V` with the two-cell inter-stage gap only *between*
    // stages — never before `D` or after `V`, so the cell aligns with sibling
    // Status values such as `executing` and `v1.0 Complete` (UIFIX-02).
    // Each stage letter stays its own span so per-letter color survives.
    let mut spans: Vec<Span> = Vec::with_capacity(stages.len() * 2 - 1);
    for (i, (label, threshold)) in stages.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("  "));
        }
        let color = if *status >= *threshold {
            Color::Green
        } else if *status == prev_status(*threshold) {
            Color::Yellow
        } else {
            Color::DarkGray
        };
        spans.push(Span::styled(*label, Style::default().fg(color)));
    }

    Line::from(spans)
}

/// The exact number of terminal cells `compact_pipeline` produces:
/// five one-cell stage letters plus four two-cell inter-stage gaps
/// (`D  R  P  E  V`). The Status column must never be allocated fewer
/// cells than this, or the trailing `V` is clipped and a fully-verified
/// project renders identically to a mid-pipeline one (UIFIX-02 / CR-01).
const STATUS_COLUMN_MIN_CELLS: u16 = 13;

/// Resolve the dashboard's header cells and column constraints for a terminal width.
///
/// Single source of truth for the three width tiers, so the render-level tests
/// cannot drift from what `render_main` actually lays out.
///
/// `terminal_width` is the **outer** area width (borders included), which is what
/// selects the tier — but the returned constraints are resolved by `Table` against
/// the block's **inner** width, two cells narrower. That two-cell gap is exactly why
/// a percentage-based Status column clipped at width 80; the `Constraint::Min` floor
/// below is immune to it.
///
/// The `<60` tier is deliberately left on percentages (decision GD-02): its
/// `Percentage(30)` already yields 13+ cells from width 44 upward, and adding a floor
/// there would starve the Alias column at very narrow widths.
fn dashboard_columns(terminal_width: u16) -> (Vec<&'static str>, Vec<Constraint>) {
    if terminal_width >= 80 {
        (
            vec!["Alias", "Phase", "Status", "Progress", "Backlog"],
            vec![
                Constraint::Percentage(25),
                Constraint::Percentage(30),
                Constraint::Min(STATUS_COLUMN_MIN_CELLS),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
            ],
        )
    } else if terminal_width >= 60 {
        (
            vec!["Alias", "Phase", "Status", "Progress"],
            vec![
                Constraint::Percentage(30),
                Constraint::Percentage(35),
                Constraint::Min(STATUS_COLUMN_MIN_CELLS),
                Constraint::Percentage(15),
            ],
        )
    } else {
        (
            vec!["Alias", "Phase", "Status"],
            vec![
                Constraint::Percentage(35),
                Constraint::Percentage(35),
                Constraint::Percentage(30),
            ],
        )
    }
}

/// Build the dashboard `Table` for a terminal width from already-built rows.
///
/// Pairs with [`dashboard_columns`] so the header cells and the constraint vector
/// can never disagree. `terminal_width` is the outer area width — see
/// [`dashboard_columns`] for why.
fn dashboard_table<'a>(rows: Vec<Row<'a>>, terminal_width: u16) -> Table<'a> {
    let (header_cells, widths) = dashboard_columns(terminal_width);

    let header = Row::new(header_cells)
        .style(Style::default().add_modifier(Modifier::BOLD))
        .bottom_margin(0);

    Table::new(rows, widths)
        .header(header)
        .row_highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED))
        .highlight_symbol("> ")
}

impl Screen for NormalScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        if self.searching {
            return self.handle_search_key(code, ctx);
        }

        match code {
            KeyCode::Char('q') => ScreenAction::Quit,
            KeyCode::Char('j') | KeyCode::Down => {
                move_selection_down(ctx);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                move_selection_up(ctx);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char('a') => {
                ctx.input_buffer.clear();
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(AddProjectScreen::new_alias()))
            }
            KeyCode::Char('c') => {
                ctx.input_buffer.clear();
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(CreateProjectScreen::new_name()))
            }
            KeyCode::Char('d') => {
                if let Some(alias) = ctx.selected_alias() {
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(DeleteConfirmScreen::new(alias)))
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Enter => {
                if let Some(alias) = ctx.selected_alias() {
                    ctx.detail_scroll_offset = 0;
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(DetailScreen::new(alias)))
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Char('/') => {
                self.searching = true;
                ctx.input_buffer.clear();
                ctx.filter_text.clear();
                ctx.recompute_filtered_aliases();
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char('?') => {
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(HelpScreen))
            }
            KeyCode::Tab => {
                let alias = match ctx.selected_alias() {
                    Some(a) => a,
                    None => return ScreenAction::None,
                };
                let project_path = ctx.config.projects.get(&alias).map(|p| p.path.clone());
                let session = project_path.and_then(|path| {
                    ctx.active_sessions
                        .iter()
                        .find(|s| s.working_dir == path)
                        .cloned()
                });
                let msg = match session {
                    Some(s) => match crate::terminal_switch::switch_to_session(&s) {
                        Ok(()) => format!("Switched to {}", alias),
                        Err(e) => e,
                    },
                    None => format!("No active Claude session for {}", alias),
                };
                ctx.status_message = Some((msg, std::time::Instant::now()));
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        // Minimum terminal size guard
        if area.width < 40 || area.height < 8 {
            let msg = Paragraph::new("Terminal too small. Resize to at least 40x8.")
                .alignment(Alignment::Center);
            frame.render_widget(msg, area);
            return;
        }

        // Split into main area and footer
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        let main_area = chunks[0];
        let footer_area = chunks[1];

        self.render_main(frame, main_area, ctx);
        self.render_footer(frame, footer_area, ctx);
    }

    fn name(&self) -> &str {
        "normal"
    }
}

impl NormalScreen {
    fn handle_search_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        match code {
            KeyCode::Char(c) => {
                ctx.filter_text.push(c);
                ctx.recompute_filtered_aliases();
                if !ctx.filtered_aliases.is_empty() {
                    ctx.table_state.select(Some(0));
                } else {
                    ctx.table_state.select(None);
                }
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Backspace => {
                ctx.filter_text.pop();
                ctx.recompute_filtered_aliases();
                if !ctx.filtered_aliases.is_empty() {
                    ctx.table_state.select(Some(0));
                } else {
                    ctx.table_state.select(None);
                }
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Esc => {
                self.searching = false;
                ctx.filter_text.clear();
                ctx.recompute_filtered_aliases();
                if !ctx.filtered_aliases.is_empty() {
                    ctx.table_state.select(Some(0));
                } else {
                    ctx.table_state.select(None);
                }
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Enter => {
                // Confirm filter and return to normal mode (filter stays active)
                self.searching = false;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    fn render_main(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .title(" GSD Manager ");

        if ctx.config.projects.is_empty() && !self.searching {
            // Empty state
            let inner = outer_block.inner(area);
            frame.render_widget(outer_block, area);

            let empty_text = vec![
                Line::from(Span::styled(
                    "No projects registered",
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from("Press [a] to add your first GSD project"),
            ];

            let paragraph = Paragraph::new(empty_text).alignment(Alignment::Center);

            let y_offset = if inner.height > 3 {
                (inner.height - 3) / 2
            } else {
                0
            };
            let centered_area = Rect {
                x: inner.x,
                y: inner.y + y_offset,
                width: inner.width,
                height: 3.min(inner.height),
            };

            frame.render_widget(paragraph, centered_area);
        } else {
            // Project table
            let inner = outer_block.inner(area);
            frame.render_widget(outer_block, area);

            let terminal_width = area.width;

            let rows: Vec<Row> = ctx
                .filtered_aliases
                .iter()
                .map(|alias| {
                    let state = ctx.project_states.get(alias);

                    let status_str = match state {
                        Some(s) if !s.status.is_empty() => s.status.clone(),
                        _ => "unknown".to_string(),
                    };

                    let phase_cell = match state {
                        Some(s) => format_phase_display(s),
                        None => "?".to_string(),
                    };

                    // Workstream cue (GSD 1.8.0): show the active workstream name,
                    // else an "N ws" count, appended to the Phase column so
                    // multi-workstream projects are visible in every layout.
                    let ws_suffix = match state {
                        Some(s) if !s.workstreams.is_empty() => {
                            match s.workstreams.iter().find(|w| w.active) {
                                Some(active) => format!("  [ws:{}]", active.name),
                                None => format!("  [{} ws]", s.workstreams.len()),
                            }
                        }
                        _ => String::new(),
                    };
                    let phase_line: Line = if ws_suffix.is_empty() {
                        Line::from(phase_cell)
                    } else {
                        Line::from(vec![
                            Span::raw(phase_cell),
                            Span::styled(ws_suffix, Style::default().fg(Color::Magenta)),
                        ])
                    };

                    let progress_cell = match state {
                        Some(s) => format!("{}/{} phases", s.completed_phases, s.total_phases),
                        None => "?".to_string(),
                    };

                    let backlog_cell = match state {
                        Some(s) if s.backlog_count > 0 => s.backlog_count.to_string(),
                        _ => "-".to_string(),
                    };

                    let row_color = status_color(&status_str);

                    // Build status cell: milestone complete, pipeline, or expanded
                    let is_milestone_complete = match state {
                        Some(s) => s.completed_phases >= s.total_phases && s.total_phases > 0,
                        None => false,
                    };

                    let status_cell: Line = if is_milestone_complete {
                        // D-03: Show milestone name for completed milestones
                        let milestone_text = match state {
                            Some(s) if !s.milestone.is_empty() => {
                                format!("{} Complete", s.milestone)
                            }
                            _ => "Complete".to_string(),
                        };
                        Line::from(Span::styled(
                            milestone_text,
                            Style::default().fg(Color::DarkGray),
                        ))
                    } else {
                        // All rows: show compact pipeline
                        match state.and_then(|s| s.current_phase_status.as_ref()) {
                            Some(inference) => compact_pipeline(&inference.status),
                            None => Line::from(Span::styled(
                                status_str.clone(),
                                Style::default().fg(row_color),
                            )),
                        }
                    };

                    // Check if project has an active Claude session
                    let has_session = ctx
                        .config
                        .projects
                        .get(alias)
                        .map(|proj| {
                            ctx.active_sessions
                                .iter()
                                .any(|s| s.working_dir == proj.path)
                        })
                        .unwrap_or(false);

                    // Check if project is paused (from HANDOFF file detection)
                    let is_paused = ctx
                        .project_states
                        .get(alias)
                        .map(|s| s.paused)
                        .unwrap_or(false);

                    // Legitimately blocked on an external/async job (GSD 1.8.0).
                    let external_job_waiting = state
                        .map(|s| s.external_job_waiting)
                        .unwrap_or(false);

                    // Badge priority: pause > external-job-waiting > session.
                    let alias_cell: Line =
                        match alias_badge(is_paused, external_job_waiting, has_session) {
                            Some((glyph, color)) => Line::from(vec![
                                Span::styled(glyph, Style::default().fg(color)),
                                Span::raw(alias.clone()),
                            ]),
                            None => Line::from(alias.clone()),
                        };

                    let cells: Vec<Line> = if terminal_width >= 80 {
                        vec![
                            alias_cell,
                            phase_line,
                            status_cell,
                            Line::from(progress_cell),
                            Line::from(backlog_cell),
                        ]
                    } else if terminal_width >= 60 {
                        vec![
                            alias_cell,
                            phase_line,
                            status_cell,
                            Line::from(progress_cell),
                        ]
                    } else {
                        vec![alias_cell, phase_line, status_cell]
                    };

                    Row::new(cells).style(Style::default().fg(row_color))
                })
                .collect();

            let table = dashboard_table(rows, terminal_width);

            // We need a mutable table_state for rendering
            let mut table_state = ctx.table_state;
            frame.render_stateful_widget(table, inner, &mut table_state);
            // Note: table_state selection is managed by ctx directly through handle_key
        }
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        if self.searching {
            render_search_footer(frame, area, ctx);
            return;
        }

        // Check for status message first
        if let Some((msg, _)) = &ctx.status_message {
            let color = if msg.starts_with("Added") || msg.starts_with("Removed") {
                Color::Green
            } else {
                Color::default()
            };
            let line = Line::from(Span::styled(msg.clone(), Style::default().fg(color)));
            frame.render_widget(Paragraph::new(line), area);
        } else {
            render_normal_footer(frame, area, ctx);
        }
    }
}

fn render_normal_footer(frame: &mut Frame, area: Rect, ctx: &AppContext) {
    let all_count = ctx.config.projects.len();
    let mut active = 0u32;
    let mut blocked = 0u32;
    let mut idle = 0u32;
    let mut complete = 0u32;

    for state in ctx.project_states.values() {
        match classify_status(&state.status) {
            StatusCategory::Active => active += 1,
            StatusCategory::Blocked => blocked += 1,
            StatusCategory::Idle => idle += 1,
            StatusCategory::Complete => complete += 1,
            StatusCategory::Unknown => {}
        }
    }

    let mut left_spans: Vec<Span> = vec![Span::raw(format!("{} projects ", all_count))];
    if active > 0 {
        left_spans.push(Span::styled(
            format!("{} active ", active),
            Style::default().fg(Color::Green),
        ));
    }
    if blocked > 0 {
        left_spans.push(Span::styled(
            format!("{} blocked ", blocked),
            Style::default().fg(Color::Red),
        ));
    }
    if idle > 0 {
        left_spans.push(Span::styled(
            format!("{} idle ", idle),
            Style::default().fg(Color::DarkGray),
        ));
    }
    if complete > 0 {
        left_spans.push(Span::styled(
            format!("{} done ", complete),
            Style::default().fg(Color::Cyan),
        ));
    }

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let right_spans = vec![
        Span::styled("[/]", bold),
        Span::raw("search  "),
        Span::styled("[Tab]", bold),
        Span::raw("session  "),
        Span::styled("[?]", bold),
        Span::raw("help  "),
        Span::styled("[a]", bold),
        Span::raw("dd  "),
        Span::styled("[c]", bold),
        Span::raw("reate  "),
        Span::styled("[d]", bold),
        Span::raw("el  "),
        Span::styled("[q]", bold),
        Span::raw("uit"),
    ];
    let right_len: u16 = right_spans.iter().map(|s| s.width() as u16).sum();

    let footer_chunks =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(right_len + 1)]).split(area);

    let left = Paragraph::new(Line::from(left_spans));
    let right = Paragraph::new(Line::from(right_spans)).alignment(Alignment::Right);

    frame.render_widget(left, footer_chunks[0]);
    frame.render_widget(right, footer_chunks[1]);
}

fn render_search_footer(frame: &mut Frame, area: Rect, ctx: &AppContext) {
    let left_spans = vec![
        Span::raw("/ "),
        Span::styled(
            ctx.filter_text.clone(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        ),
        Span::raw("_"),
    ];

    let right_text = "[Esc]clear [Enter]keep";

    let footer_chunks = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(right_text.len() as u16 + 1),
    ])
    .split(area);

    let left = Paragraph::new(Line::from(left_spans));
    let right = Paragraph::new(Line::from(Span::raw(right_text))).alignment(Alignment::Right);

    frame.render_widget(left, footer_chunks[0]);
    frame.render_widget(right, footer_chunks[1]);
}

fn move_selection_down(ctx: &mut AppContext) {
    let count = ctx.filtered_aliases.len();
    if count == 0 {
        return;
    }
    let current = ctx.table_state.selected().unwrap_or(0);
    let next = if current >= count - 1 { 0 } else { current + 1 };
    ctx.table_state.select(Some(next));
}

fn move_selection_up(ctx: &mut AppContext) {
    let count = ctx.filtered_aliases.len();
    if count == 0 {
        return;
    }
    let current = ctx.table_state.selected().unwrap_or(0);
    let next = if current == 0 { count - 1 } else { current - 1 };
    ctx.table_state.select(Some(next));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::parse_project_state;
    use std::fs;
    use tempfile::TempDir;

    const PAUSE_BADGE: (&str, Color) = ("\u{23F8} ", Color::Cyan);
    const ASYNC_BADGE: (&str, Color) = ("\u{23F3} ", Color::Yellow);
    const SESSION_BADGE: (&str, Color) = ("\u{25b6} ", Color::Green);

    /// Build a temp project with a `.planning/` dir holding the given files.
    /// Returns the TempDir (keep it alive) — mirrors the `make_planning`
    /// fixture in `state_reader::tests`.
    fn make_planning(files: &[(&str, &str)]) -> TempDir {
        let td = TempDir::new().unwrap();
        let planning = td.path().join(".planning");
        fs::create_dir_all(&planning).unwrap();
        for (rel, content) in files {
            fs::write(planning.join(rel), content).unwrap();
        }
        td
    }

    // --- UIFIX-01: end-to-end tracer -------------------------------------

    #[test]
    fn test_paused_project_shows_pause_badge_end_to_end() {
        // A real HANDOFF.md on disk → state reader → ProjectState → badge.
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            (
                "HANDOFF.md",
                "# Handoff\n\nResume with /gsd-execute-phase 14\n",
            ),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));

        assert!(state.paused);
        assert_eq!(
            state.pause_context.as_deref(),
            Some("Resume with /gsd-execute-phase 14")
        );

        // The same flag the dashboard row reads selects the cyan pause badge.
        assert_eq!(alias_badge(state.paused, false, false), Some(PAUSE_BADGE));
    }

    // --- UIFIX-01: badge priority (flag -> badge) -------------------------

    #[test]
    fn test_pause_badge_wins_over_session() {
        // UI-SPEC UIFIX-01 row 5: pause replaces the session glyph.
        assert_eq!(alias_badge(true, false, true), Some(PAUSE_BADGE));
    }

    #[test]
    fn test_pause_badge_wins_over_async_job() {
        // UI-SPEC UIFIX-01 row 6: pause replaces the hourglass...
        assert_eq!(alias_badge(true, true, false), Some(PAUSE_BADGE));
        // ...and still wins when every lower-priority indicator is also set.
        assert_eq!(alias_badge(true, true, true), Some(PAUSE_BADGE));
    }

    #[test]
    fn test_async_job_badge_when_not_paused() {
        // UI-SPEC UIFIX-01 row 7: hourglass outranks the session glyph.
        assert_eq!(alias_badge(false, true, true), Some(ASYNC_BADGE));
        assert_eq!(alias_badge(false, false, true), Some(SESSION_BADGE));
    }

    #[test]
    fn test_no_badge_when_nothing_active() {
        // UI-SPEC UIFIX-01 row 4/9: the alias renders flush.
        assert!(alias_badge(false, false, false).is_none());
    }

    #[test]
    fn test_badge_is_never_two_glyphs() {
        // Zero-one-many: every Some badge is exactly one glyph plus one space,
        // so two glyphs can never appear in the alias cell.
        for (paused, async_job, session) in [
            (true, false, false),
            (true, true, true),
            (false, true, false),
            (false, false, true),
        ] {
            let (glyph, _) = alias_badge(paused, async_job, session).expect("expected a badge");
            assert_eq!(glyph.chars().count(), 2);
            assert!(glyph.ends_with(' '));
        }
    }

    // --- UIFIX-02: D-R-P-E-V has no leading blank -------------------------

    const ALL_DISK_STATUSES: [DiskStatus; 7] = [
        DiskStatus::NoDirectory,
        DiskStatus::Empty,
        DiskStatus::Discussed,
        DiskStatus::Researched,
        DiskStatus::Planned,
        DiskStatus::Partial,
        DiskStatus::Complete,
    ];

    /// Concatenate a rendered `Line`'s span contents into a plain String.
    fn rendered(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    /// Collect the (letter, color) pairs of the stage spans, located by content
    /// rather than by index so separator spans cannot shift the assertion.
    fn stage_colors(line: &Line<'_>) -> Vec<(String, Option<Color>)> {
        line.spans
            .iter()
            .filter(|s| matches!(s.content.as_ref(), "D" | "R" | "P" | "E" | "V"))
            .map(|s| (s.content.to_string(), s.style.fg))
            .collect()
    }

    #[test]
    fn test_compact_pipeline_has_no_leading_blank() {
        for status in ALL_DISK_STATUSES {
            let text = rendered(&compact_pipeline(&status));
            // Assert on the first *char*, not a byte index, so a multi-byte
            // glyph elsewhere in the row cannot invalidate the check.
            let first = text.chars().next().expect("pipeline cell is never empty");
            assert_eq!(first, 'D', "status {:?} rendered {:?}", status, text);
            assert!(
                !first.is_whitespace(),
                "status {:?} rendered a leading blank: {:?}",
                status,
                text
            );
        }
    }

    #[test]
    fn test_compact_pipeline_exact_cells_and_width() {
        for status in ALL_DISK_STATUSES {
            let line = compact_pipeline(&status);
            let text = rendered(&line);
            assert_eq!(text, "D  R  P  E  V", "status {:?}", status);
            assert_eq!(line.width(), 13, "status {:?}", status);
            assert_eq!(text.chars().next_back(), Some('V'), "status {:?}", status);
        }
    }

    #[test]
    fn test_compact_pipeline_stage_colors_preserved() {
        // Planned: D, R, P reached (Green); E is next up (Yellow); V not started.
        let line = compact_pipeline(&DiskStatus::Planned);
        assert_eq!(
            stage_colors(&line),
            vec![
                ("D".to_string(), Some(Color::Green)),
                ("R".to_string(), Some(Color::Green)),
                ("P".to_string(), Some(Color::Green)),
                ("E".to_string(), Some(Color::Yellow)),
                ("V".to_string(), Some(Color::DarkGray)),
            ]
        );
    }

    #[test]
    fn test_compact_pipeline_no_workflow_data_still_renders_five_stages() {
        // A project with no workflow data on disk never yields a short cell.
        let none = compact_pipeline(&DiskStatus::NoDirectory);
        assert_eq!(rendered(&none), "D  R  P  E  V");
        assert!(stage_colors(&none)
            .iter()
            .all(|(_, fg)| *fg == Some(Color::DarkGray)));

        let empty = compact_pipeline(&DiskStatus::Empty);
        assert_eq!(rendered(&empty), "D  R  P  E  V");
        let empty_colors = stage_colors(&empty);
        assert_eq!(empty_colors[0], ("D".to_string(), Some(Color::Yellow)));
        assert!(empty_colors[1..]
            .iter()
            .all(|(_, fg)| *fg == Some(Color::DarkGray)));
    }

    // --- UIFIX-02: rendered-frame regression (gap closure, 14-04) ----------
    //
    // Every test above asserts the *isolated* `Line` object. That is exactly
    // why the clipping defect was invisible: `compact_pipeline` was always
    // correct, and the column holding it was not. The tests below render a
    // real frame through the production `dashboard_table` and read the cells
    // back out of the buffer.

    /// The literal 13-cell string the Status column must never clip.
    const PIPELINE: &str = "D  R  P  E  V";

    /// Build the interior (inside the borders) text lines of a rendered
    /// dashboard, from the top border downward: index 0 is the header row,
    /// index 1 onward are the data rows.
    ///
    /// The x range is restricted to the columns strictly inside the left and
    /// right border, so every returned string is pure ASCII and byte offsets
    /// equal column offsets — which is what makes the alignment assertion in
    /// `test_status_column_aligns_with_plain_status_text` sound.
    fn render_dashboard_interior(width: u16, rows: Vec<Vec<Line<'static>>>) -> Vec<String> {
        use ratatui::backend::TestBackend;
        use ratatui::widgets::TableState;
        use ratatui::Terminal;

        let row_count = rows.len() as u16;
        // top border + header + data rows + bottom border, plus slack.
        let height = row_count + 4;
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("TestBackend terminal");

        terminal
            .draw(|frame| {
                let area = frame.area();
                // The same bordered block `render_main` builds, and the same
                // outer-width/inner-rect relationship: the tier is selected
                // from the OUTER width while the columns are laid out over
                // the INNER width, two cells narrower.
                let outer_block = Block::default()
                    .borders(Borders::ALL)
                    .title(" GSD Manager ");
                let inner = outer_block.inner(area);
                frame.render_widget(outer_block, area);

                let table_rows: Vec<Row> = rows.into_iter().map(Row::new).collect();
                let table = dashboard_table(table_rows, area.width);
                let mut table_state = TableState::default();
                frame.render_stateful_widget(table, inner, &mut table_state);
            })
            .expect("draw dashboard frame");

        let buffer = terminal.backend().buffer().clone();
        (0..row_count + 1)
            .map(|i| {
                let y = 1 + i;
                (1..width.saturating_sub(1))
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .map(|cell| cell.symbol())
                            .unwrap_or(" ")
                            .to_string()
                    })
                    .collect::<String>()
            })
            .collect()
    }

    /// The rendered data rows only — the header line dropped.
    fn render_dashboard_rows(width: u16, rows: Vec<Vec<Line<'static>>>) -> Vec<String> {
        let mut lines = render_dashboard_interior(width, rows);
        lines.remove(0);
        lines
    }

    /// One dashboard row whose Status cell is the full D-R-P-E-V pipeline.
    fn pipeline_row() -> Vec<Line<'static>> {
        vec![
            Line::from("proj"),
            Line::from("14 ui-fixes"),
            compact_pipeline(&DiskStatus::Complete),
            Line::from("3/7 phases"),
            Line::from("-"),
        ]
    }

    #[test]
    fn test_dashboard_columns_status_floor_at_upper_tiers() {
        // The Status column carries a hard 13-cell floor at both upper tiers,
        // so no percentage arithmetic can ever starve it below what
        // `compact_pipeline` produces. `Constraint` is `PartialEq`, so this is
        // a direct equality check against the production constraint.
        let (headers_80, widths_80) = dashboard_columns(80);
        assert_eq!(headers_80.len(), 5);
        assert_eq!(widths_80.len(), 5);
        assert_eq!(widths_80[2], Constraint::Min(STATUS_COLUMN_MIN_CELLS));

        let (headers_60, widths_60) = dashboard_columns(60);
        assert_eq!(headers_60.len(), 4);
        assert_eq!(widths_60.len(), 4);
        assert_eq!(widths_60[2], Constraint::Min(STATUS_COLUMN_MIN_CELLS));

        // The narrowest tier is deliberately left on percentages (GD-02): it
        // already holds all five stages from width 44 up, and a floor there
        // would starve the Alias column instead.
        let (headers_40, widths_40) = dashboard_columns(40);
        assert_eq!(headers_40.len(), 3);
        assert_eq!(widths_40.len(), 3);
        assert_eq!(widths_40[2], Constraint::Percentage(30));
    }

    #[test]
    fn test_dashboard_columns_header_and_width_counts_match() {
        // Header vector and width vector must stay the same length at every
        // tier, with Status always at index 2 — the invariant that keeps the
        // per-row cell vector in `render_main` from drifting out of step.
        for width in [200u16, 120, 80, 79, 60, 59, 40, 20] {
            let (headers, widths) = dashboard_columns(width);
            assert_eq!(
                headers.len(),
                widths.len(),
                "header/width count mismatch at width {}",
                width
            );
            assert_eq!(
                headers[2], "Status",
                "Status not at index 2 at width {}",
                width
            );
        }
    }

    #[test]
    fn test_status_column_renders_all_five_stages_from_44_to_200() {
        // The gap: a fully-verified project and a mid-pipeline project must
        // never render an identical Status cell. Sweep every supported width.
        let mut clipped: Vec<u16> = Vec::new();
        for width in 44u16..=200 {
            let rows = render_dashboard_rows(width, vec![pipeline_row()]);
            if !rows[0].contains(PIPELINE) {
                clipped.push(width);
            }
        }
        assert!(
            clipped.is_empty(),
            "Status column clipped at widths {:?}",
            clipped
        );
    }

    #[test]
    fn test_status_column_not_clipped_at_reproduced_widths() {
        // The eleven widths the verifier and the code reviewer independently
        // reproduced as clipping the trailing `V`, plus 59, 86 and 120 as
        // controls. Named for the reproduction so the gap traces to this test.
        const REPRODUCED: [u16; 11] = [60, 61, 62, 63, 66, 80, 81, 82, 83, 84, 85];
        const CONTROLS: [u16; 3] = [59, 86, 120];

        for width in REPRODUCED.iter().chain(CONTROLS.iter()) {
            let rows = render_dashboard_rows(*width, vec![pipeline_row()]);
            assert!(
                rows[0].contains(PIPELINE),
                "width {} clipped the pipeline: {:?}",
                width,
                rows[0]
            );
        }
    }

    #[test]
    fn test_status_column_aligns_with_plain_status_text() {
        // The pipeline cell must begin at the same buffer column as a sibling
        // plain-text Status value, at the default 80-column terminal.
        let mut plain_row = pipeline_row();
        plain_row[2] = Line::from("executing");

        let rows = render_dashboard_rows(80, vec![pipeline_row(), plain_row]);

        let pipeline_at = rows[0]
            .find(PIPELINE)
            .expect("pipeline present in the rendered row");
        let plain_at = rows[1]
            .find("executing")
            .expect("plain status present in the rendered row");
        assert_eq!(
            pipeline_at, plain_at,
            "pipeline starts at {} but plain status starts at {}",
            pipeline_at, plain_at
        );
    }

    #[test]
    fn test_dashboard_table_with_no_rows_renders_header() {
        // Zero registered projects must still lay out the header, at all
        // three tiers, without panicking.
        for width in [80u16, 60, 40] {
            let lines = render_dashboard_interior(width, Vec::new());
            assert!(
                lines[0].contains("Status"),
                "Status header missing at width {}: {:?}",
                width,
                lines[0]
            );
        }
    }
}
