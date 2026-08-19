use super::add_project::AddProjectScreen;
use super::create_project::CreateProjectScreen;
use super::delete_confirm::DeleteConfirmScreen;
use super::detail::DetailScreen;
use super::driver_confirm::{DriverAction, DriverConfirmScreen};
use super::help::HelpScreen;
use super::{AppContext, Screen, ScreenAction, SortMode};
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
///
/// "One step below" is the *declaration-order predecessor*, so this function has
/// to move whenever a variant is inserted into `DiskStatus`. `Executed` was
/// inserted between `Partial` and `Complete`, so `Complete`'s predecessor is now
/// `Executed` and not `Partial`. Leaving it at `Partial` would light the `V`
/// stage yellow for a partially-executed phase — claiming verification is next
/// up when execution has not finished — and leave it dark for the `Executed`
/// phase where verification genuinely IS the next step.
fn prev_status(threshold: DiskStatus) -> DiskStatus {
    match threshold {
        DiskStatus::Discussed => DiskStatus::Empty,
        DiskStatus::Researched => DiskStatus::Discussed,
        DiskStatus::Planned => DiskStatus::Researched,
        DiskStatus::Partial => DiskStatus::Planned,
        DiskStatus::Executed => DiskStatus::Partial,
        DiskStatus::Complete => DiskStatus::Executed,
        _ => DiskStatus::NoDirectory,
    }
}

// ── Dashboard badge glyphs (Phase 18 UI-SPEC `## Surface 6`, D-24) ─────────
//
// Every glyph is a fixed `&'static str` written as a `\u{…}` escape rather than
// a raw glyph in source (PATTERNS S7). Both properties are load-bearing:
//
// * `&'static str` is the *mechanism* enforcing "a badge is never derived from
//   file content". A HANDOFF body, an agent's prose summary, or any other byte
//   read off disk cannot be assigned to one of these, so it cannot reach a
//   dashboard row through the badge (D-13, D-24, T-18-32).
// * The escape form keeps the source readable in editors and diffs that render
//   these codepoints ambiguously, and makes the intended codepoint checkable by
//   eye against the UI-SPEC table.
//
// Each is one glyph plus one space — two terminal cells — and every glyph is
// East-Asian-Width Ambiguous (narrow), matching the shipped `\u{25b6}`. That is
// what keeps the Alias column aligned at all three dashboard width tiers.

/// Rank 1 — an agent is driving this repo *right now*. `◆` FILLED DIAMOND.
///
/// Shape-distinct from every other badge at a glance. `▲` (up triangle) was
/// rejected: it is the session `▶` rotated, and two glyphs that differ only by
/// rotation fail the at-a-glance test that is this badge's whole purpose.
pub(super) const BADGE_DRIVEN: &str = "\u{25C6} ";

/// Rank 2 — this project is waiting on a human. `⚑` BLACK FLAG.
///
/// A flag is the conventional "planted here, come look" mark and shares no
/// outline with the diamond, the two pause bars, the hourglass or the triangle.
pub(super) const BADGE_NEEDS_HUMAN: &str = "\u{2691} ";

/// Rank 3 — a non-empty HANDOFF. `⏸` DOUBLE VERTICAL BAR. Shipped in v1.4.
pub(super) const BADGE_PAUSED: &str = "\u{23F8} ";

/// Rank 4 — blocked on an external/async job, not stuck. `⏳` HOURGLASS.
/// Shipped in GSD 1.8.0.
pub(super) const BADGE_EXTERNAL_JOB: &str = "\u{23F3} ";

/// Rank 5 — an active Claude session in this project's directory. `▶`.
/// Shipped in v1.0.
pub(super) const BADGE_SESSION: &str = "\u{25b6} ";

/// The single badge a dashboard alias cell leads with.
///
/// The glyph field is `&'static str` **by design, not by convenience** — see the
/// constants above. Widening it to `String` would silently remove the guarantee
/// that no file content can reach a dashboard row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AliasBadge {
    pub glyph: &'static str,
    pub color: Color,
    pub modifier: Modifier,
}

/// The five independent facts that select a badge, named rather than positional.
///
/// A five-`bool` parameter list is a five-way ordering hazard at every call site
/// and in every test; a `Default`-able struct makes each fact self-labelling and
/// lets a test state one condition and mean it.
#[derive(Debug, Clone, Copy, Default)]
struct BadgeInputs {
    /// A run exists for this alias **and** `ObservedRun::is_live()` says its pid
    /// and cmdline both still check out. `Liveness` is tri-state and
    /// `Liveness::Unknown` is never a synonym for dead (CR-05) — `is_live()`
    /// owns that distinction, and this flag is its answer, not a re-derivation.
    driven_and_live: bool,
    /// 18-04's `needs_human` predicate over evidence that exists today (D-14).
    needs_human: bool,
    /// `ProjectState::paused` — a non-empty HANDOFF.
    is_paused: bool,
    /// `ProjectState::external_job_waiting`.
    external_job_waiting: bool,
    /// An active Claude session whose working dir is this project.
    has_session: bool,
}

/// Select the single leading badge for a dashboard alias cell.
///
/// **Badge priority, highest first** (Phase 18 UI-SPEC `## Surface 6`, D-24;
/// extends the Phase 14 `### Badge Priority Rule`):
///
/// 1. driven **and** live — `BADGE_DRIVEN`, Magenta + BOLD
/// 2. needs a human — `BADGE_NEEDS_HUMAN`, Red + BOLD
/// 3. paused — `BADGE_PAUSED`, Cyan
/// 4. external job waiting — `BADGE_EXTERNAL_JOB`, Yellow
/// 5. active Claude session — `BADGE_SESSION`, Green
/// 6. none of the above — no badge; the alias renders flush
///
/// **Driven-and-live outranks everything, and the reason is the whole point of
/// the rank:** the user must never be unsure whether something is driving their
/// repo. Ranks 3–5 keep their v1.0/v1.4/1.8.0 order beneath the two new ones.
///
/// **At most one badge ever renders**, and the single `Option` return is the
/// *mechanism* that enforces it, not a stylistic choice — badges never stack, so
/// the Alias column stays aligned at all three width tiers. Every glyph is a
/// fixed `&'static str`, which is the mechanism enforcing that no HANDOFF body
/// or agent prose can leak onto a dashboard row (D-13, D-24).
///
/// **No meaning is carried by colour alone.** Every badge is a glyph whose shape
/// differs from every other badge's shape, so the five ranks remain
/// distinguishable in a monochrome terminal or to a colour-blind reader. The two
/// new badges are additionally the only ones carrying `BOLD`, which lifts them
/// above the three shipped badges without needing a second cell. Magenta for
/// rank 1 because Cyan, Yellow and Green are taken by ranks 3–5, Red by rank 2,
/// DarkGray means inert and Blue means directory — Magenta is this codebase's
/// "this is not one of the ordinary states" colour, which an autonomous agent
/// driving the user's repo precisely is.
///
/// **Rank 3 is reachable but production-superseded, and that is intentional.**
/// `needs_human` (rank 2) counts `state.paused` among its four sources (D-14),
/// so a paused project on the live dashboard renders the red flag rather than
/// the cyan pause bars — the stronger "waiting on you" signal, which is what
/// OBS-02 asks for. The rank-3 arm stays because this function is pure over its
/// inputs: a caller passing a narrower predicate still gets the shipped v1.4
/// badge, and deleting the arm would make that impossible without noticing.
fn alias_badge(inputs: BadgeInputs) -> Option<AliasBadge> {
    let BadgeInputs {
        driven_and_live,
        needs_human,
        is_paused,
        external_job_waiting,
        has_session,
    } = inputs;

    let bold = Modifier::BOLD;
    let plain = Modifier::empty();

    if driven_and_live {
        // An agent is driving this repo right now; nothing outranks that.
        Some(AliasBadge {
            glyph: BADGE_DRIVEN,
            color: Color::Magenta,
            modifier: bold,
        })
    } else if needs_human {
        // Waiting on the user, per 18-04's evidence predicate (D-14).
        Some(AliasBadge {
            glyph: BADGE_NEEDS_HUMAN,
            color: Color::Red,
            modifier: bold,
        })
    } else if is_paused {
        // Pause badge takes priority over the two lower indicators.
        Some(AliasBadge {
            glyph: BADGE_PAUSED,
            color: Color::Cyan,
            modifier: plain,
        })
    } else if external_job_waiting {
        // Hourglass: waiting on an async job, not stuck.
        Some(AliasBadge {
            glyph: BADGE_EXTERNAL_JOB,
            color: Color::Yellow,
            modifier: plain,
        })
    } else if has_session {
        Some(AliasBadge {
            glyph: BADGE_SESSION,
            color: Color::Green,
            modifier: plain,
        })
    } else {
        None
    }
}

/// The badge for one dashboard row, gathered from the context the row renders
/// from.
///
/// Separated from [`alias_badge`] so the production wiring — *which* state feeds
/// *which* input — is exercised by tests rather than living inline in a closure
/// inside `render_main`, where the only way to reach it is to render a frame.
///
/// Every input is a typed flag or a typed predicate. **No string read from disk
/// is inspected here**, so nothing an agent writes can decide what a badge says
/// (D-13). An unregistered alias, an alias with no parsed state, and an alias
/// that has never been driven all fall through to `None` without panicking.
fn row_badge(ctx: &AppContext, alias: &str) -> Option<AliasBadge> {
    let state = ctx.project_states.get(alias);

    // Tri-state `Liveness` is resolved by `is_live()`, never re-derived here.
    let driven_and_live = ctx
        .observed_runs
        .get(alias)
        .is_some_and(|run| run.is_live());

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

    alias_badge(BadgeInputs {
        driven_and_live,
        needs_human: ctx.needs_human_for(alias),
        is_paused: state.map(|s| s.paused).unwrap_or(false),
        external_job_waiting: state.map(|s| s.external_job_waiting).unwrap_or(false),
        has_session,
    })
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
            // ── The three driver keys (D-25) ──────────────────────────────
            //
            // This is Phase 17's ENTIRE UI surface: three keys pushing one
            // confirmation. The Driver tab, the live stream, the dashboard
            // badges and the rich opt-in disclosure flow are Phase 18's — see
            // the module doc on `super::driver_confirm` for the full fence.
            //
            // Collision check, performed before these were written: this match
            // already claims `q`, `j`, `k`, `a`, `c`, `d`, `/`, `?`, `Tab`,
            // `Enter`, `Up` and `Down`, and the search sub-mode is entered by
            // `/` and handled separately. None of `r`, `x`, `o` is among them.
            // (`r` is bound in `detail.rs` for the roadmap toggle. That is a
            // different screen with its own `handle_key` match, so it is not a
            // collision — the help screen annotates both with their scope.)
            KeyCode::Char('r') => {
                if let Some(alias) = ctx.selected_alias() {
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(DriverConfirmScreen::new(
                        alias,
                        DriverAction::Start,
                    )))
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Char('x') => {
                if let Some(alias) = ctx.selected_alias() {
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(DriverConfirmScreen::new(
                        alias,
                        DriverAction::Stop,
                    )))
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Char('o') => {
                if let Some(alias) = ctx.selected_alias() {
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(DriverConfirmScreen::new(
                        alias,
                        DriverAction::ToggleOptIn,
                    )))
                } else {
                    ScreenAction::None
                }
            }
            // ── The dashboard sort toggle (D-25, OBS-07) ──────────────────
            //
            // One key, one indicator, no new screen. `s` is free on this
            // screen: the match above claims `q`, `j`, `k`, `a`, `c`, `d`,
            // `r`, `x`, `o`, `/`, `?`, `Tab`, `Enter`, `Up` and `Down`, and
            // the search sub-mode is entered by `/` and handled separately.
            //
            // **Alphabetical stays the default and that is load-bearing.** A
            // dashboard whose row order changes under the cursor while a run
            // progresses is a usability regression a demo will not catch, so
            // the alternative ordering is opt-in, announced in the status
            // message, and shown in the summary row for as long as it is on.
            //
            // The recompute is what applies the new order, and it is also what
            // re-pins the selection to the previously selected **alias** — so
            // toggling the sort does not move the cursor to a different
            // project (`AppContext::pin_selection_to_alias`, 18-04).
            KeyCode::Char('s') => {
                ctx.sort_mode = match ctx.sort_mode {
                    SortMode::Alphabetical => SortMode::AttentionFirst,
                    SortMode::AttentionFirst => SortMode::Alphabetical,
                };
                ctx.recompute_filtered_aliases();
                let msg = match ctx.sort_mode {
                    SortMode::AttentionFirst => "Sort: attention first",
                    SortMode::Alphabetical => "Sort: alphabetical",
                };
                ctx.status_message = Some((msg.to_string(), std::time::Instant::now()));
                ctx.needs_redraw = true;
                ScreenAction::None
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
                ScreenAction::Push(Box::new(HelpScreen::new()))
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

                    // Badge priority (D-24): driven-and-live > needs-human >
                    // pause > external-job-waiting > session. At most one.
                    let alias_cell: Line = match row_badge(ctx, alias) {
                        Some(badge) => Line::from(vec![
                            Span::styled(
                                badge.glyph,
                                Style::default().fg(badge.color).add_modifier(badge.modifier),
                            ),
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

/// The summary-row label for the non-default sort mode.
///
/// There is deliberately no constant for the default mode: it renders nothing.
const SORT_INDICATOR: &str = "sort: attention";

/// Build the dashboard summary row — the left half of the footer.
///
/// Split out of [`render_normal_footer`] so the sort indicator is assertable on
/// the spans themselves rather than only through a rendered buffer, where a
/// missing indicator and a clipped one look identical.
///
/// **Nothing here is derived from an agent's prose (D-13).** Every count comes
/// from `classify_status` over `ProjectState`, and the sort indicator is a fixed
/// literal selected by a typed enum. No `ResultMessage.result`, no HANDOFF body
/// and no journal text may ever reach this row: a summary the user reads as a
/// fleet-wide fact must be telemetry, not testimony.
///
/// **The indicator renders only in the non-default sort mode, and the silence in
/// the default mode is deliberate.** A permanent `sort: alphabetical` label
/// would be noise on every dashboard forever to communicate the state the user
/// already assumes; what needs saying is that the order is *not* the one they
/// assume (UI-SPEC Surface 7, mitigation 2). Do not add an `else` branch here.
fn summary_spans(ctx: &AppContext) -> Vec<Span<'static>> {
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
    if ctx.sort_mode == SortMode::AttentionFirst {
        left_spans.push(Span::styled(
            SORT_INDICATOR,
            Style::default().fg(Color::Cyan),
        ));
    }

    left_spans
}

fn render_normal_footer(frame: &mut Frame, area: Rect, ctx: &AppContext) {
    let left_spans = summary_spans(ctx);

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
    use crate::driver::liveness::Liveness;
    use crate::driver::reconcile::ObservedRun;
    use crate::state_reader::{parse_project_state, ProjectState};
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// The five badges, as `(glyph, color, modifier)` expectations.
    ///
    /// Spelled out here rather than reusing the production constants for the
    /// glyph *and* the colour together: an assertion built entirely from the
    /// thing it is checking cannot fail, and the badge table is exactly the
    /// place a silent swap (two ranks trading colours) would go unnoticed.
    const DRIVEN_BADGE: AliasBadge = AliasBadge {
        glyph: "\u{25C6} ",
        color: Color::Magenta,
        modifier: Modifier::BOLD,
    };
    const NEEDS_HUMAN_BADGE: AliasBadge = AliasBadge {
        glyph: "\u{2691} ",
        color: Color::Red,
        modifier: Modifier::BOLD,
    };
    const PAUSE_BADGE: AliasBadge = AliasBadge {
        glyph: "\u{23F8} ",
        color: Color::Cyan,
        modifier: Modifier::empty(),
    };
    const ASYNC_BADGE: AliasBadge = AliasBadge {
        glyph: "\u{23F3} ",
        color: Color::Yellow,
        modifier: Modifier::empty(),
    };
    const SESSION_BADGE: AliasBadge = AliasBadge {
        glyph: "\u{25b6} ",
        color: Color::Green,
        modifier: Modifier::empty(),
    };

    /// Every badge, highest rank first — the order [`alias_badge`] documents.
    const ALL_BADGES: [AliasBadge; 5] = [
        DRIVEN_BADGE,
        NEEDS_HUMAN_BADGE,
        PAUSE_BADGE,
        ASYNC_BADGE,
        SESSION_BADGE,
    ];

    /// An `AppContext` with `aliases` registered, default state, nothing driven.
    ///
    /// The sixth full-field `AppContext` construction in the tree. It lives here
    /// rather than being borrowed from `screens::tests` because that module is a
    /// sibling of this one, not an ancestor, so its private fixture is
    /// unreachable — and because the behaviour under test (badges, the `s`
    /// toggle, the dashboard's filter wiring) is this screen's.
    fn ctx_with_aliases(aliases: &[&str]) -> AppContext {
        use crate::change_tracker::ChangeTracker;
        use crate::config::{Config, RegisteredProject};
        use ratatui::widgets::TableState;

        let mut config = Config::new();
        for alias in aliases {
            config.projects.insert(
                (*alias).to_string(),
                RegisteredProject {
                    path: PathBuf::from("/nonexistent").join(alias),
                    added: "2026-07-29".to_string(),
                    driver_opt_in: None,
                    extra: Default::default(),
                },
            );
        }

        let mut ctx = AppContext {
            config,
            config_path: PathBuf::from("/nonexistent/config.json"),
            project_states: aliases
                .iter()
                .map(|a| ((*a).to_string(), ProjectState::default()))
                .collect(),
            table_state: TableState::default(),
            filtered_aliases: Vec::new(),
            filter_text: String::new(),
            change_tracker: ChangeTracker::new(),
            detail_sub_view_per_project: HashMap::new(),
            view_cache: HashMap::new(),
            status_message: None,
            error_message: None,
            event_tx: None,
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
        ctx.recompute_filtered_aliases();
        ctx
    }

    /// Mark `alias` as being driven right now by a live run.
    fn drive(ctx: &mut AppContext, alias: &str) {
        ctx.observed_runs.insert(
            alias.to_string(),
            ObservedRun {
                alias: alias.to_string(),
                run_id: format!("2026-07-29T12-00-00Z-{alias}"),
                pid: 4242,
                pgid: 4242,
                started_at: "2026-07-29T12:00:00Z".to_string(),
                goal: "ship it".to_string(),
                gsd_command: "/gsd-execute-phase".to_string(),
                liveness: Liveness::Alive,
            },
        );
    }

    /// Mark `alias` as paused — one of `needs_human`'s four evidence sources.
    fn pause(ctx: &mut AppContext, alias: &str) {
        ctx.project_states
            .get_mut(alias)
            .expect("the fixture registered this alias")
            .paused = true;
    }

    /// Drive the real key handler, exactly as the event loop does.
    ///
    /// The `detail.rs:5041` idiom, and the lesson that produced it applies here
    /// too: a test that calls the toggle helper directly cannot catch a key that
    /// was never bound, and a test that hand-computes the filtered set cannot
    /// catch a handler that forgot to recompute it.
    fn press(screen: &mut NormalScreen, ctx: &mut AppContext, code: KeyCode) {
        screen.handle_key(code, KeyModifiers::NONE, ctx);
    }

    /// Type `text` into the dashboard's search prompt, from a standing start.
    ///
    /// **The leading `/` is the prompt, not part of the filter.** `handle_key`
    /// clears `filter_text` on `/` and never stores it, so the UI-SPEC's `//h`
    /// is stored as `"/h"` and `/x/h` as `"x/h"` — which is what makes the
    /// all-rows form fall out of the `/term/x` grammar with no special case
    /// (18-04 deviation 6). Typing through this helper is the only way to assert
    /// that rather than assume it.
    fn search(screen: &mut NormalScreen, ctx: &mut AppContext, text: &str) {
        press(screen, ctx, KeyCode::Char('/'));
        for c in text.chars() {
            press(screen, ctx, KeyCode::Char(c));
        }
        press(screen, ctx, KeyCode::Enter);
    }

    /// The plain text of a span vector, concatenated.
    fn spans_text(spans: &[Span<'_>]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

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

        // The same flag the dashboard row reads selects the cyan pause badge
        // whenever the caller's needs-human answer is `false`.
        assert_eq!(
            alias_badge(BadgeInputs {
                is_paused: state.paused,
                ..Default::default()
            }),
            Some(PAUSE_BADGE)
        );

        // On the live dashboard the needs-human predicate counts `paused` among
        // its four sources (D-14), so the same project renders the *rank 2* red
        // flag — the stronger "waiting on you" signal OBS-02 asks for. Asserted
        // here so the supersession is a stated behaviour, not a surprise.
        assert!(crate::ui::screens::needs_human(&state, None, None, false));
        assert_eq!(
            alias_badge(BadgeInputs {
                needs_human: true,
                is_paused: state.paused,
                ..Default::default()
            }),
            Some(NEEDS_HUMAN_BADGE)
        );
    }

    // --- UIFIX-01: badge priority (flag -> badge) -------------------------

    #[test]
    fn test_pause_badge_wins_over_session() {
        // UI-SPEC UIFIX-01 row 5: pause replaces the session glyph.
        assert_eq!(
            alias_badge(BadgeInputs {
                is_paused: true,
                has_session: true,
                ..Default::default()
            }),
            Some(PAUSE_BADGE)
        );
    }

    #[test]
    fn test_pause_badge_wins_over_async_job() {
        // UI-SPEC UIFIX-01 row 6: pause replaces the hourglass...
        assert_eq!(
            alias_badge(BadgeInputs {
                is_paused: true,
                external_job_waiting: true,
                ..Default::default()
            }),
            Some(PAUSE_BADGE)
        );
        // ...and still wins when every lower-priority indicator is also set.
        assert_eq!(
            alias_badge(BadgeInputs {
                is_paused: true,
                external_job_waiting: true,
                has_session: true,
                ..Default::default()
            }),
            Some(PAUSE_BADGE)
        );
    }

    #[test]
    fn test_async_job_badge_when_not_paused() {
        // UI-SPEC UIFIX-01 row 7: hourglass outranks the session glyph.
        assert_eq!(
            alias_badge(BadgeInputs {
                external_job_waiting: true,
                has_session: true,
                ..Default::default()
            }),
            Some(ASYNC_BADGE)
        );
        assert_eq!(
            alias_badge(BadgeInputs {
                has_session: true,
                ..Default::default()
            }),
            Some(SESSION_BADGE)
        );
    }

    #[test]
    fn test_no_badge_when_nothing_active() {
        // UI-SPEC UIFIX-01 row 4/9: the alias renders flush.
        assert!(alias_badge(BadgeInputs::default()).is_none());
    }

    // --- OBS-02 / D-24: the five-rank badge table ------------------------

    #[test]
    fn each_badge_rank_selects_its_own_glyph_in_isolation() {
        // One condition at a time, so nothing about the priority chain can make
        // a rank pass for the wrong reason.
        let cases: [(BadgeInputs, AliasBadge); 5] = [
            (
                BadgeInputs {
                    driven_and_live: true,
                    ..Default::default()
                },
                DRIVEN_BADGE,
            ),
            (
                BadgeInputs {
                    needs_human: true,
                    ..Default::default()
                },
                NEEDS_HUMAN_BADGE,
            ),
            (
                BadgeInputs {
                    is_paused: true,
                    ..Default::default()
                },
                PAUSE_BADGE,
            ),
            (
                BadgeInputs {
                    external_job_waiting: true,
                    ..Default::default()
                },
                ASYNC_BADGE,
            ),
            (
                BadgeInputs {
                    has_session: true,
                    ..Default::default()
                },
                SESSION_BADGE,
            ),
        ];

        for (inputs, expected) in cases {
            assert_eq!(
                alias_badge(inputs),
                Some(expected),
                "rank mis-selected for {inputs:?}"
            );
        }
    }

    #[test]
    fn the_driven_badge_outranks_every_other_condition() {
        // D-24's top rank exists because the user must never be unsure whether
        // something is driving their repo. Every lower condition set at once
        // must still yield exactly the driven badge.
        assert_eq!(
            alias_badge(BadgeInputs {
                driven_and_live: true,
                is_paused: true,
                ..Default::default()
            }),
            Some(DRIVEN_BADGE),
            "ranks 1 and 3 together must yield rank 1"
        );
        assert_eq!(
            alias_badge(BadgeInputs {
                driven_and_live: true,
                needs_human: true,
                is_paused: true,
                external_job_waiting: true,
                has_session: true,
            }),
            Some(DRIVEN_BADGE),
            "every condition at once must still yield rank 1"
        );
    }

    #[test]
    fn the_needs_human_badge_outranks_an_active_session() {
        // Ranks 2 and 5 together yield rank 2 only: a project waiting on the
        // user must not be reported as merely "has a session open".
        assert_eq!(
            alias_badge(BadgeInputs {
                needs_human: true,
                has_session: true,
                ..Default::default()
            }),
            Some(NEEDS_HUMAN_BADGE)
        );
        assert_eq!(
            alias_badge(BadgeInputs {
                needs_human: true,
                external_job_waiting: true,
                has_session: true,
                ..Default::default()
            }),
            Some(NEEDS_HUMAN_BADGE)
        );
    }

    #[test]
    fn test_badge_is_never_two_glyphs() {
        // Zero-one-many: every Some badge is exactly one glyph plus one space,
        // so two glyphs can never appear in the alias cell, and every alias cell
        // that carries a badge is indented by the same two cells regardless of
        // which rank won.
        for badge in ALL_BADGES {
            assert_eq!(badge.glyph.chars().count(), 2, "{badge:?}");
            assert!(badge.glyph.ends_with(' '), "{badge:?}");
        }

        // The same property through the production selector, over the full
        // 2^5 input space: no combination can produce a wider cell.
        for bits in 0u8..32 {
            let inputs = BadgeInputs {
                driven_and_live: bits & 1 != 0,
                needs_human: bits & 2 != 0,
                is_paused: bits & 4 != 0,
                external_job_waiting: bits & 8 != 0,
                has_session: bits & 16 != 0,
            };
            if let Some(badge) = alias_badge(inputs) {
                assert_eq!(badge.glyph.chars().count(), 2, "{inputs:?}");
                assert!(
                    ALL_BADGES.contains(&badge),
                    "{inputs:?} produced a badge outside the documented table"
                );
            } else {
                assert_eq!(bits, 0, "only the all-false input may yield no badge");
            }
        }
    }

    #[test]
    fn every_badge_shape_is_distinct_so_no_meaning_rides_on_colour_alone() {
        // A monochrome terminal, or a colour-blind reader, must still be able to
        // tell the five ranks apart (UI-SPEC `## Colour`).
        let mut glyphs: Vec<&str> = ALL_BADGES.iter().map(|b| b.glyph).collect();
        glyphs.sort_unstable();
        let distinct = glyphs.len();
        glyphs.dedup();
        assert_eq!(glyphs.len(), distinct, "two badges share a glyph");

        let mut colors: Vec<String> = ALL_BADGES.iter().map(|b| format!("{:?}", b.color)).collect();
        colors.sort_unstable();
        let distinct_colors = colors.len();
        colors.dedup();
        assert_eq!(colors.len(), distinct_colors, "two badges share a colour");

        // Only the two new badges carry BOLD, which is what lifts them above the
        // three shipped badges without needing a second cell.
        assert!(DRIVEN_BADGE.modifier.contains(Modifier::BOLD));
        assert!(NEEDS_HUMAN_BADGE.modifier.contains(Modifier::BOLD));
        for badge in [PAUSE_BADGE, ASYNC_BADGE, SESSION_BADGE] {
            assert!(
                !badge.modifier.contains(Modifier::BOLD),
                "{badge:?} must not compete with the two new ranks"
            );
        }
    }

    #[test]
    fn an_empty_registry_and_a_never_driven_project_both_yield_no_badge() {
        // Zero-one-many at the wiring layer, through the real `row_badge`.
        let empty = ctx_with_aliases(&[]);
        assert!(empty.filtered_aliases.is_empty());
        assert!(
            row_badge(&empty, "absent").is_none(),
            "an alias that is not registered at all must not panic or badge"
        );

        let never_driven = ctx_with_aliases(&["alpha"]);
        assert!(
            !never_driven.observed_runs.contains_key("alpha"),
            "the fixture must genuinely have no observed run"
        );
        assert!(
            row_badge(&never_driven, "alpha").is_none(),
            "a registered project that has never been driven renders flush"
        );
    }

    #[test]
    fn the_driven_badge_is_wired_to_liveness_not_to_the_mere_presence_of_a_run() {
        // T-18-33: a badge claiming an agent is driving the repo when the run is
        // gone is the spoofing failure this rank exists to avoid. `is_live()`
        // owns the tri-state; `Liveness::Unknown` is never a synonym for dead.
        let mut ctx = ctx_with_aliases(&["alpha"]);
        drive(&mut ctx, "alpha");
        assert_eq!(row_badge(&ctx, "alpha"), Some(DRIVEN_BADGE));

        for (liveness, expected) in [(Liveness::Dead, None), (Liveness::Unknown, None)] {
            ctx.observed_runs
                .get_mut("alpha")
                .expect("driven above")
                .liveness = liveness;
            assert_eq!(
                row_badge(&ctx, "alpha"),
                expected,
                "{liveness:?} must not light the driven badge"
            );
        }
    }

    #[test]
    fn computing_badges_never_reorders_rows() {
        // Two projects with identical driven/parked state keep their relative
        // order: badge computation is a per-row read, not a sort.
        let mut ctx = ctx_with_aliases(&["alpha", "bravo", "charlie"]);
        drive(&mut ctx, "alpha");
        drive(&mut ctx, "bravo");
        pause(&mut ctx, "charlie");

        let before = ctx.filtered_aliases.clone();
        let badges: Vec<Option<AliasBadge>> = ctx
            .filtered_aliases
            .iter()
            .map(|alias| row_badge(&ctx, alias))
            .collect();

        assert_eq!(ctx.filtered_aliases, before, "row order must not move");
        assert_eq!(
            badges,
            vec![
                Some(DRIVEN_BADGE),
                Some(DRIVEN_BADGE),
                Some(NEEDS_HUMAN_BADGE)
            ]
        );
    }

    // --- UIFIX-02: D-R-P-E-V has no leading blank -------------------------

    const ALL_DISK_STATUSES: [DiskStatus; 8] = [
        DiskStatus::NoDirectory,
        DiskStatus::Empty,
        DiskStatus::Discussed,
        DiskStatus::Researched,
        DiskStatus::Planned,
        DiskStatus::Partial,
        DiskStatus::Executed,
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
    fn test_compact_pipeline_executed_lights_v_as_next_up() {
        // The state this whole vocabulary change exists for: implementation is
        // done, verification is not. `E` must read as reached and `V` as the
        // next step — never as reached, which is what `Complete` claimed before
        // `Executed` existed.
        let line = compact_pipeline(&DiskStatus::Executed);
        assert_eq!(
            stage_colors(&line),
            vec![
                ("D".to_string(), Some(Color::Green)),
                ("R".to_string(), Some(Color::Green)),
                ("P".to_string(), Some(Color::Green)),
                ("E".to_string(), Some(Color::Green)),
                ("V".to_string(), Some(Color::Yellow)),
            ],
            "an executed phase renders V green only if `prev_status` was left \
             pointing at Partial — which would also mean the dashboard shows a \
             phase awaiting human verification as verified"
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
    fn a_badged_row_and_an_unbadged_row_keep_every_later_column_aligned() {
        // D-24's reason for the "at most one badge" rule, asserted where it is
        // observable: whatever badge a row wins, the alias cell consumes the
        // same allocation and every column after it starts at the same buffer
        // column as an unbadged row's. Badge stacking is what would break this.
        let mut badged = pipeline_row();
        badged[0] = Line::from(vec![
            Span::styled(BADGE_DRIVEN, Style::default().fg(Color::Magenta)),
            Span::raw("proj"),
        ]);
        let mut flagged = pipeline_row();
        flagged[0] = Line::from(vec![
            Span::styled(BADGE_NEEDS_HUMAN, Style::default().fg(Color::Red)),
            Span::raw("proj"),
        ]);

        let rows = render_dashboard_rows(80, vec![pipeline_row(), badged, flagged]);

        // Column offsets, not byte offsets: a badged row is no longer pure
        // ASCII, and `\u{25C6}` is three bytes wide but one cell wide. Every
        // badge glyph is East-Asian-Width narrow, so one buffer cell is one
        // char and a char count *is* the column.
        fn column_of(row: &str, needle: &str) -> usize {
            let byte = row
                .find(needle)
                .unwrap_or_else(|| panic!("{needle:?} missing from rendered row {row:?}"));
            row[..byte].chars().count()
        }

        let phase_at: Vec<usize> = rows
            .iter()
            .map(|row| column_of(row, "14 ui-fixes"))
            .collect();
        assert_eq!(
            phase_at[0], phase_at[1],
            "a driven-badged row shifted the Phase column"
        );
        assert_eq!(
            phase_at[0], phase_at[2],
            "a needs-human-badged row shifted the Phase column"
        );

        // And the badge occupies exactly the two leading cells of the alias
        // cell, so the alias text itself is indented identically by either rank.
        let alias_at: Vec<usize> = rows.iter().map(|row| column_of(row, "proj")).collect();
        assert_eq!(alias_at[1], alias_at[0] + 2);
        assert_eq!(alias_at[2], alias_at[0] + 2);
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

    // --- OBS-07 / D-25: the sort toggle, its indicator, and the filter ----

    #[test]
    fn s_toggles_the_sort_mode_and_confirms_with_the_pinned_copy() {
        let mut screen = NormalScreen::new();
        let mut ctx = ctx_with_aliases(&["alpha", "bravo"]);

        assert_eq!(
            ctx.sort_mode,
            SortMode::Alphabetical,
            "alphabetical must stay the default: a dashboard whose row order \
             changes under the cursor mid-run is the regression D-25 names"
        );

        press(&mut screen, &mut ctx, KeyCode::Char('s'));
        assert_eq!(ctx.sort_mode, SortMode::AttentionFirst);
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Sort: attention first"),
            "the non-default mode must be announced, not silently entered"
        );
        assert!(ctx.needs_redraw);

        press(&mut screen, &mut ctx, KeyCode::Char('s'));
        assert_eq!(ctx.sort_mode, SortMode::Alphabetical);
        assert_eq!(
            ctx.status_message.as_ref().map(|(m, _)| m.as_str()),
            Some("Sort: alphabetical")
        );
    }

    #[test]
    fn the_sort_indicator_renders_only_in_the_non_default_mode() {
        // Asserted on the built spans rather than a rendered buffer: a missing
        // indicator and one clipped off the end of the row look identical once
        // painted.
        let mut screen = NormalScreen::new();
        let mut ctx = ctx_with_aliases(&["alpha"]);

        let quiet = summary_spans(&ctx);
        assert!(
            !spans_text(&quiet).contains("sort"),
            "the default mode must render nothing — a permanent \
             `sort: alphabetical` label is noise: {:?}",
            spans_text(&quiet)
        );

        press(&mut screen, &mut ctx, KeyCode::Char('s'));
        let loud = summary_spans(&ctx);
        let indicator = loud
            .iter()
            .find(|s| s.content.contains("sort:"))
            .expect("the non-default mode must be visible in the summary row");
        assert_eq!(indicator.content.as_ref(), "sort: attention");
        assert_eq!(indicator.style.fg, Some(Color::Cyan));
        assert!(
            !spans_text(&loud).contains("alphabetical"),
            "there is no label for the default mode anywhere"
        );

        // ...and toggling back removes it again.
        press(&mut screen, &mut ctx, KeyCode::Char('s'));
        assert!(!spans_text(&summary_spans(&ctx)).contains("sort"));
    }

    #[test]
    fn a_needs_human_filter_with_a_term_narrows_to_the_matching_needs_human_rows() {
        // `/x/h` — the term still applies across every column, so this is
        // "matches x AND needs a human".
        let mut screen = NormalScreen::new();
        let mut ctx = ctx_with_aliases(&["alpha", "xenon", "xylo"]);
        pause(&mut ctx, "alpha");
        pause(&mut ctx, "xenon");

        search(&mut screen, &mut ctx, "x/h");

        assert_eq!(ctx.filter_text, "x/h", "the prompt's `/` is not stored");
        assert_eq!(
            ctx.filtered_aliases,
            vec!["xenon".to_string()],
            "alpha needs a human but does not match the term; xylo matches the \
             term but needs nobody"
        );
    }

    #[test]
    fn a_bare_needs_human_filter_yields_every_needs_human_project() {
        // `//h` — the all-rows form, which falls out of the existing `/term/x`
        // grammar with no special case because an empty term matches everything.
        let mut screen = NormalScreen::new();
        let mut ctx = ctx_with_aliases(&["alpha", "xenon", "xylo"]);
        pause(&mut ctx, "alpha");
        pause(&mut ctx, "xenon");

        search(&mut screen, &mut ctx, "/h");

        assert_eq!(ctx.filter_text, "/h");
        assert_eq!(
            ctx.filtered_aliases,
            vec!["alpha".to_string(), "xenon".to_string()],
            "every needs-human project, still in alphabetical order"
        );
    }

    #[test]
    fn a_needs_human_filter_matching_nothing_leaves_an_empty_list_and_no_panic() {
        // Zero-one-many: the empty result is the one a fleet spends most of its
        // time in — nobody is waiting on the user — so it must be the calm case.
        let mut screen = NormalScreen::new();
        let mut ctx = ctx_with_aliases(&["alpha", "bravo"]);

        search(&mut screen, &mut ctx, "/h");

        assert!(ctx.filtered_aliases.is_empty());
        assert_eq!(
            ctx.table_state.selected(),
            None,
            "an empty list selects nothing rather than a dangling index"
        );
        assert_eq!(ctx.selected_alias(), None);

        // Every key that acts on a selection must be a no-op here.
        for code in [
            KeyCode::Char('j'),
            KeyCode::Char('k'),
            KeyCode::Down,
            KeyCode::Up,
            KeyCode::Enter,
            KeyCode::Char('s'),
        ] {
            press(&mut screen, &mut ctx, code);
            assert_eq!(ctx.selected_alias(), None, "{code:?} invented a selection");
        }
    }

    #[test]
    fn a_project_that_is_both_driven_and_needs_human_appears_exactly_once() {
        // The two new badge conditions are independent, so a project can hold
        // both. It is still one row, under either form of the filter.
        let mut screen = NormalScreen::new();
        let mut ctx = ctx_with_aliases(&["other", "solo"]);
        drive(&mut ctx, "solo");
        pause(&mut ctx, "solo");
        assert_eq!(
            row_badge(&ctx, "solo"),
            Some(DRIVEN_BADGE),
            "both conditions hold, and exactly one badge renders"
        );

        search(&mut screen, &mut ctx, "/h");
        assert_eq!(ctx.filtered_aliases, vec!["solo".to_string()]);

        press(&mut screen, &mut ctx, KeyCode::Esc);
        search(&mut screen, &mut ctx, "s/h");
        assert_eq!(ctx.filtered_aliases, vec!["solo".to_string()]);
    }

    #[test]
    fn an_attention_sort_keeps_the_cursor_on_the_same_alias_when_a_run_finishes() {
        // The mitigation that makes `AttentionFirst` safe rather than actively
        // dangerous (D-25). Without it, a run finishing re-ranks its project,
        // the row moves, and the next keystroke acts on a project the user
        // never chose. The logic lives in `screens/mod.rs`, but the failure the
        // user experiences is a dashboard failure, so it is asserted here.
        let mut screen = NormalScreen::new();
        let mut ctx = ctx_with_aliases(&["alpha", "mid", "zebra"]);
        drive(&mut ctx, "zebra");

        press(&mut screen, &mut ctx, KeyCode::Char('s'));
        assert_eq!(
            ctx.filtered_aliases,
            vec![
                "zebra".to_string(),
                "alpha".to_string(),
                "mid".to_string()
            ],
            "the driven project sorts first, the rest stay alphabetical"
        );

        // Put the cursor on the driven project.
        press(&mut screen, &mut ctx, KeyCode::Up);
        assert_eq!(ctx.selected_alias().as_deref(), Some("zebra"));
        assert_eq!(ctx.table_state.selected(), Some(0));

        // The run finishes and the rows are recomputed.
        ctx.observed_runs.remove("zebra");
        ctx.recompute_filtered_aliases();

        assert_eq!(
            ctx.filtered_aliases,
            vec![
                "alpha".to_string(),
                "mid".to_string(),
                "zebra".to_string()
            ],
            "the re-rank really did move the row"
        );
        assert_eq!(
            ctx.selected_alias().as_deref(),
            Some("zebra"),
            "the cursor is pinned to the alias, not to the index"
        );
        assert_eq!(
            ctx.table_state.selected(),
            Some(2),
            "index 0 would now be `alpha` — a project the user never chose"
        );
    }
}
