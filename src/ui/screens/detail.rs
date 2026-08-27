use super::driver_confirm::{DriverAction, DriverConfirmScreen};
use super::driver_inject::DriverInjectScreen;
use super::driver_start::DriverStartScreen;
use super::enqueue::EnqueueScreen;
use super::help::HelpScreen;
use super::queue_delete_confirm::QueueDeleteConfirmScreen;
use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::app::{classify_status, DetailSubView, StatusCategory};
use crate::change_tracker::ChangeTracker;
use crate::state_reader::disk_status::{DiskInference, DiskStatus};
use crate::state_reader::git_ops;
use crate::state_reader::queue_md;
use crate::state_reader::{self, backlog};
use crate::text::Untrusted;
use crate::ui::roadmap_widget::RoadmapWidget;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs};
use ratatui::Frame;
use std::cell::Cell;

pub(super) const PAGE_SCROLL_LINES: u16 = 20;

/// What a human READS, escaped — the render-side half of the identity split,
/// spelled once for this file (CR-01, D-19-5).
///
/// **The rule every call site below follows.** A value this build did not author
/// — a registry key, or a status, milestone, phase number, phase name, HANDOFF
/// context line, queued command, backlog title, session id, git subject or file
/// name parsed out of the project's `.planning/` — goes through here on its way
/// to a terminal cell. The SAME value goes to a map lookup, a comparison, a path
/// segment or a write **raw and untouched**, because escaping is a legibility
/// defence and not a transformation of the data.
///
/// It is a named function rather than an inline call at each site so the rule is
/// stated in one place and a reader of the diff sees the rule rather than fifty
/// instances of it. It is deliberately NOT a wrapper type: a type would be the
/// UI-wide retype D-21-4 measured at 25 direct compile errors plus an unbounded
/// cascade, and this round chose the source-derived `Screen` census and its
/// behavioural probe instead. What holds these call sites honest is therefore
/// `render_escape_guard::the_screen_renders_identity_escaped`, which renders
/// every tab and asserts that no invisible-class character reaches a cell — not
/// the discipline of whoever adds the next one.
///
/// `display_identity` is idempotent over its own output (pinned in
/// `text::tests`), so a value that passes through here twice is unchanged.
///
/// # WR-02: this file answers BOTH classes now, and it does not decide which
///
/// This used to be `display_identity` alone, which answers only the
/// invisible-formatting class (`Cf` ∪ `Default_Ignorable`). Everything this file
/// draws is parsed out of a project's `.planning/` directory, and a `.planning/`
/// file can carry a raw `ESC`, a C0 control, or a C1 introducer just as easily
/// as a `U+202E` — those are the CONTROL class, `Cc`, and `display_identity`
/// does not touch them. The tree's own statement of that split is at
/// `src/ui/screens/driver.rs:874-878`; what was missing was a site that composed
/// the two rather than picking one.
///
/// So this delegates to [`crate::text::render_for_terminal`], the ONE
/// composition, and this file no longer decides which halves apply — it inherits
/// the resolution. The deliberate second composition,
/// `display_identity(&sanitize_render_line(..))` in `driver.rs` and
/// `driver_confirm.rs`, differs from it ONLY by the
/// `DRIVER_OUTPUT_LINE_CELLS` display cap those two want because they draw agent
/// prose; that difference is pinned by
/// `ui::screens::tests::the_capped_and_uncapped_compositions_agree_below_the_cap`.
///
/// It stays a `String`-returning free function rather than becoming
/// [`crate::text::Rendered`]-returning: its ~50 call sites in this file all
/// interpolate the result, and the type-level lever this round introduces lives
/// on the CARRIER ([`crate::text::Untrusted`]) rather than on the escape helper.
/// A `GitLogEntry` field reaches a cell through `entry.field.shown()` and never
/// through here.
fn shown(value: &str) -> String {
    crate::text::render_for_terminal(value).to_string()
}

/// How many CHARACTERS of a session id the Sessions tab and the resume toast
/// show.
const SESSION_ID_DISPLAY_CHARS: usize = 8;

/// A session id shortened for display — **by characters, never by bytes**
/// (T-21-25-05).
///
/// # The panic this replaced
///
/// Both call sites used to read `if sid.len() > 8 { &sid[..8] }`. `str::len`
/// counts BYTES and `sid[..8]` slices BYTES, so any session id longer than
/// eight bytes whose eighth byte falls inside a multi-byte character panics.
/// The id is scraped verbatim out of another process's `--resume` argument
/// (`session_detector::read_session_id`), so nothing about it was authored here
/// and nothing constrains it to ASCII — a five-character CJK id is fifteen
/// bytes and lands mid-character at byte eight. A panic inside a render pass
/// takes the whole TUI down.
///
/// **This is a family, not an incident.** It is the same defect class as the
/// `&s[..n]` panic round 8 fixed in `ui::roadmap_widget` — a byte index used
/// where a character count was meant, on a value read from outside this build.
/// `a_multibyte_session_id_does_not_panic_the_render` and
/// `a_long_multibyte_session_id_is_truncated_by_characters_not_bytes` were both
/// observed red against the byte slice, and the panic is quoted verbatim in the
/// first one's doc.
///
/// # Order of operations, and why it is this way round
///
/// The raw value is truncated FIRST and escaped SECOND. Escaping expands each
/// invisible-class character into a six-character `U+XXXX` marker, so escaping
/// first and then cutting at eight could slice a marker in half and print a
/// fragment that reads like data. Truncating first means the cap always means
/// "the first eight characters of the id" and the escape is applied whole.
fn shorten_session_id(sid: &Untrusted) -> String {
    let raw = sid.as_raw_for_logic_only();
    let shortened: String = raw.chars().take(SESSION_ID_DISPLAY_CHARS).collect();
    shown(&shortened)
}

/// Viewport metrics recorded by the last render pass of a markdown file view.
///
/// Both fields are zero before the first render, which yields a max scroll of
/// zero — the correct pre-first-render floor, not a crash.
///
/// `pub(super)` since plan 18-09 so `driver.rs` — a sibling module rather than a
/// second mechanism — records its output pane's metrics through **this** type
/// and clamps through [`clamp_scroll`]. One clamp formula, shared; three copies
/// of it is how the UIFIX-04 defect would return in a new place.
#[derive(Clone, Copy, Default)]
pub(super) struct ViewportMetrics {
    pub(super) total_lines: u16,
    pub(super) visible_height: u16,
}

/// Clamp a stored scroll offset to the last-rendered viewport.
///
/// Uses the identical `total_lines - visible_height` formula the render path
/// already applies for display, so the two cannot drift apart.
///
/// `pub(super)` for the reason given on [`ViewportMetrics`].
pub(super) fn clamp_scroll(offset: u16, total_lines: u16, visible_height: u16) -> u16 {
    offset.min(total_lines.saturating_sub(visible_height))
}

/// The largest offset the last render pass could display: the tail.
///
/// `u16::MAX` means *"as far down as this pane goes"*, and [`clamp_scroll`]
/// resolves it. Written this way on purpose: spelling
/// `total_lines - visible_height` out a second time is precisely how UIFIX-04
/// would come back in a new place, and every caller that needs the tail — the
/// Driver pane's renderer, its `PageDown` arm and its `G` arm — goes through
/// this one function and therefore through the one clamp formula.
pub(super) fn tail_offset(vp: ViewportMetrics) -> u16 {
    clamp_scroll(u16::MAX, vp.total_lines, vp.visible_height)
}

/// The Driver output pane's scroll offset as a key press must see it.
///
/// **While the follow bit is set the stored offset is stale by design**: the
/// pane renders the tail, whatever the tail currently is, so a key press has to
/// start from the tail rather than from the number parked in the cache. Both
/// branches resolve through [`clamp_scroll`] against the metrics the render pass
/// recorded, so the up-direction arms below can clamp **first** and subtract
/// second without a second formula existing anywhere (UIFIX-04, D-19).
pub(super) fn driver_offset_now(stored: u16, following: bool, vp: ViewportMetrics) -> u16 {
    if following {
        tail_offset(vp)
    } else {
        clamp_scroll(stored, vp.total_lines, vp.visible_height)
    }
}

/// How many tabs the detail view has, including the Driver tab at index 10.
pub(crate) const TAB_COUNT: usize = 11;

/// Full tab labels, one per index. The Driver entry omits its live-marker cell,
/// which [`tab_titles`] always appends as a span of its own (see
/// [`DRIVER_LIVE_MARKER`]).
const TAB_LABELS_FULL: [&str; TAB_COUNT] = [
    "1:Phases",
    "2:Roadmap",
    "3:Backlog",
    "4:Git",
    "5:Pipe",
    "6:Queue",
    "7:Sess",
    "8:Arch",
    "9:Cfg",
    "0:Docs",
    "D:Drive",
];

/// Compact tab labels, same order. The digit is the real handle in every case,
/// so a two-letter mnemonic loses nothing that matters; the pairs are mutually
/// unambiguous.
const TAB_LABELS_COMPACT: [&str; TAB_COUNT] = [
    "1:Ph", "2:Rd", "3:Bk", "4:Gt", "5:Pp", "6:Qu", "7:Ss", "8:Ar", "9:Cf", "0:Dc", "D:Dr",
];

/// The Driver tab's index. Named because five sites compare against it.
pub(crate) const DRIVER_TAB_INDEX: usize = 10;

/// Rendered width of the full label set, in terminal cells.
///
/// **Derived from the render, not chosen:** `Tabs` draws `1 pad + label + 1 pad`
/// per tab and a one-cell `"|"` divider between adjacent tabs, so the bar costs
/// `Σ(len + 2) + (n − 1)`. For [`TAB_LABELS_FULL`] plus the Driver tab's
/// always-present marker cell that is `75 + 1 + 22 + 10 = 107`.
///
/// The number matters because **the ten shipped tabs already occupied 96 cells**:
/// at an 80-column terminal `9:Cfg` and `0:Docs` were silently dropped off the
/// right edge, which is the "tab bar overflow at 80 columns" defect carried in
/// `STATE.md` since Phase 12. Appending an 11th tab at the end without tiering
/// would have made the Driver tab the one that never renders, at every common
/// width.
pub(crate) const TAB_BAR_FULL_CELLS: u16 = 107;

/// Rendered width of the compact label set, by the same arithmetic:
/// `45 + 1 + 22 + 10 = 77`.
pub(crate) const TAB_BAR_COMPACT_CELLS: u16 = 77;

/// The Driver label's reserved final cell while a run is live: `◆`.
///
/// A fixed `&'static str` written as a `\u{…}` escape, per the house rule that no
/// raw glyph appears in source (`normal.rs:55-70`). Magenta is the phase's
/// driven-and-live colour.
const DRIVER_LIVE_MARKER: &str = "\u{25C6}";

/// The Driver label's reserved final cell while nothing is driving: one space.
///
/// **The cell is always present**, so the bar's width never changes when a run
/// starts or ends — no tab shifts sideways under the user's eye mid-run. Inside
/// a project's detail view the dashboard is not visible, and the user must never
/// be unsure whether something is driving their repo.
const DRIVER_IDLE_MARKER: &str = " ";

/// Left overflow marker: `‹`. DarkGray, one cell.
const TAB_OVERFLOW_LEFT: &str = "\u{2039}";

/// Right overflow marker: `›`. DarkGray, one cell.
const TAB_OVERFLOW_RIGHT: &str = "\u{203A}";

/// The cells one `Tabs` entry costs: the label plus its two pads.
fn tab_entry_cells(label_cells: usize) -> usize {
    label_cells + 2
}

/// Build the tab-bar titles for `width`, and the select index into them.
///
/// **A tab bar that silently drops the active tab is a defect, not a tier.** That
/// sentence is the whole specification of this function; the three tiers below
/// exist to honour it, and the windowed tier exists because at 40 columns no
/// eleven-tab bar can be shown whole.
///
/// | Tier | Condition | Labels |
/// |---|---|---|
/// | Full | `width >= `[`TAB_BAR_FULL_CELLS`] | [`TAB_LABELS_FULL`] |
/// | Compact | `width >= `[`TAB_BAR_COMPACT_CELLS`] | [`TAB_LABELS_COMPACT`] |
/// | Windowed | below that | a contiguous run of compact labels **containing `active`**, with `‹`/`›` markers on whichever side is truncated |
///
/// Returning the labels *and* the adjusted select index from one function is what
/// makes the tiering assertable without a terminal — the same reason
/// [`footer_spans`] is split out of [`build_footer`]. Both tab-bar render sites
/// call this; before plan 18-09 the bar was constructed twice, verbatim, and a
/// tier applied to only one of them would have left the Driver tab visible on one
/// path and invisible on the other.
///
/// `driver_live` drives only the Driver label's reserved marker cell, which is
/// present either way (see [`DRIVER_IDLE_MARKER`]).
pub(crate) fn tab_titles(
    width: u16,
    active: usize,
    driver_live: bool,
) -> (Vec<Line<'static>>, usize) {
    let active = active.min(TAB_COUNT - 1);

    if width >= TAB_BAR_FULL_CELLS {
        return (
            (0..TAB_COUNT)
                .map(|i| tab_label_line(&TAB_LABELS_FULL, i, driver_live))
                .collect(),
            active,
        );
    }
    if width >= TAB_BAR_COMPACT_CELLS {
        return (
            (0..TAB_COUNT)
                .map(|i| tab_label_line(&TAB_LABELS_COMPACT, i, driver_live))
                .collect(),
            active,
        );
    }

    windowed_tab_titles(width, active, driver_live)
}

/// One tab's `Line`. The Driver entry gets its marker cell as a second span so
/// the label's width is identical live and idle.
fn tab_label_line(labels: &[&'static str; TAB_COUNT], index: usize, driver_live: bool) -> Line<'static> {
    if index == DRIVER_TAB_INDEX {
        let marker = if driver_live {
            Span::styled(DRIVER_LIVE_MARKER, Style::default().fg(Color::Magenta))
        } else {
            Span::raw(DRIVER_IDLE_MARKER)
        };
        Line::from(vec![Span::raw(labels[index]), marker])
    } else {
        Line::from(labels[index])
    }
}

/// The width in cells of one compact label, marker cell included.
fn compact_label_cells(index: usize) -> usize {
    TAB_LABELS_COMPACT[index].chars().count() + usize::from(index == DRIVER_TAB_INDEX)
}

/// The windowed tier: the widest contiguous run of compact labels that contains
/// `active` and fits, with overflow markers on whichever side was truncated.
///
/// The window is grown outward from the active tab — right first, then left — so
/// the active label is the one thing that is never given up. If even the active
/// label alone overflows the bar it is still rendered: a clipped label the user
/// can see beats a correct one they cannot.
fn windowed_tab_titles(width: u16, active: usize, driver_live: bool) -> (Vec<Line<'static>>, usize) {
    let cost = |start: usize, end: usize| -> usize {
        let mut entries: Vec<usize> = (start..end).map(compact_label_cells).collect();
        if start > 0 {
            entries.insert(0, 1);
        }
        if end < TAB_COUNT {
            entries.push(1);
        }
        let cells: usize = entries.iter().copied().map(tab_entry_cells).sum();
        cells + entries.len().saturating_sub(1)
    };

    let budget = usize::from(width);
    let mut start = active;
    let mut end = active + 1;
    loop {
        let mut grew = false;
        if end < TAB_COUNT && cost(start, end + 1) <= budget {
            end += 1;
            grew = true;
        }
        if start > 0 && cost(start - 1, end) <= budget {
            start -= 1;
            grew = true;
        }
        if !grew {
            break;
        }
    }

    let marker_style = Style::default().fg(Color::DarkGray);
    let mut titles: Vec<Line<'static>> = Vec::new();
    if start > 0 {
        titles.push(Line::from(Span::styled(TAB_OVERFLOW_LEFT, marker_style)));
    }
    let select = titles.len() + (active - start);
    for i in start..end {
        titles.push(tab_label_line(&TAB_LABELS_COMPACT, i, driver_live));
    }
    if end < TAB_COUNT {
        titles.push(Line::from(Span::styled(TAB_OVERFLOW_RIGHT, marker_style)));
    }

    (titles, select)
}

/// Whether the selected project has a run this scan positively knows is running.
///
/// `Liveness` is a tri-state and [`crate::driver::reconcile::ObservedRun::is_live`]
/// owns the distinction; `Liveness::Unknown` is **never** a synonym for dead
/// (CR-05) and is resolved there, never re-derived here.
fn driver_live_for(ctx: &AppContext, alias: &str) -> bool {
    ctx.observed_runs
        .get(alias)
        .is_some_and(|run| run.is_live())
}

pub struct DetailScreen {
    pub alias: String,
    pub scroll_offset: u16,
    /// Last-rendered viewport metrics for the Docs (Browse) file view.
    /// Interior mutability: `Screen::render` takes `&self`, so the render pass
    /// cannot write into the view cache (see plan 14-02 CD-01).
    browser_viewport: Cell<ViewportMetrics>,
    /// Last-rendered viewport metrics for the Archive file view.
    archive_viewport: Cell<ViewportMetrics>,
    /// Last-rendered viewport metrics for the generic text panes — the Phases
    /// and Roadmap tabs, the only two sub-views that reach the `_ =>` scroll
    /// fallback. Same interior-mutability reason as the two Cells above
    /// (plan 14-04 decision GD-01, closing code review IN-07 / CD-03).
    generic_viewport: Cell<ViewportMetrics>,
    /// Last-rendered viewport metrics for the Driver tab's live-output pane.
    ///
    /// A fourth `Cell` alongside the three above rather than a fourth
    /// *mechanism*: the output pane clamps through the same [`clamp_scroll`]
    /// formula the Browse and Archive viewers already use, which is what keeps
    /// the UIFIX-04 fix from having to be made a second time in a new place.
    driver_viewport: Cell<ViewportMetrics>,
}

impl DetailScreen {
    /// This screen's [`Screen::name`], as a constant.
    ///
    /// `app.rs`'s elapsed-time redraw gate has to ask "is a detail view on top
    /// of the stack?", and the stack holds `Box<dyn Screen>` — so the only
    /// answer available is the name. Naming it here rather than repeating the
    /// literal there is what keeps the two in agreement: `name()` returns this
    /// constant, so a rename cannot leave the gate comparing against a string
    /// no screen answers to, which would silently disable the redraw (D-21).
    pub const NAME: &'static str = "detail";

    pub fn new(alias: String) -> Self {
        Self {
            alias,
            scroll_offset: 0,
            browser_viewport: Cell::default(),
            archive_viewport: Cell::default(),
            generic_viewport: Cell::default(),
            driver_viewport: Cell::default(),
        }
    }

    /// Move the Driver tab's run selection by `delta`, clamped to the list.
    ///
    /// **Selecting a different run resets the output pane** — the offset goes
    /// back to the tail and the follow bit comes back on (D-19) — and
    /// reschedules the run-list scan, because the inbox and the journal the scan
    /// reads are the *selected* run's. Leaving the previous run's messages under
    /// a new selection would attribute one run's steering history to another,
    /// which is the same error `DriverRunTally`'s run id exists to prevent.
    ///
    /// Extracted rather than inlined twice because `j`/Down and `k`/Up differ
    /// only in the sign, and two copies of a clamp are two things to keep in
    /// agreement.
    fn move_driver_selection(&self, ctx: &mut AppContext, delta: isize) {
        let len = ctx
            .view_cache
            .get(&self.alias)
            .map_or(0, |cache| cache.driver_runs.len());
        let mut moved = false;
        if len > 0 {
            let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
            let last = len - 1;
            let next = if delta < 0 {
                cache.driver_selected_run.saturating_sub(delta.unsigned_abs())
            } else {
                cache
                    .driver_selected_run
                    .saturating_add(delta as usize)
                    .min(last)
            }
            .min(last);
            if next != cache.driver_selected_run {
                cache.driver_selected_run = next;
                cache.driver_scroll_offset = 0;
                cache.driver_follow = true;
                moved = true;
            }
        }
        if moved {
            if let Some(path) = ctx.config.projects.get(&self.alias).map(|p| p.path.clone()) {
                ctx.schedule_run_list_scan(&self.alias, &path);
            }
        }
        ctx.needs_redraw = true;
    }
}

/// `pub(crate)` so the index mapping is assertable from `app.rs`, which owns the
/// enum: a tab whose index does not round-trip lands the user on a different tab
/// than the one they asked for, and that is a logic-level property rather than a
/// rendering one.
pub(crate) fn tab_index(sub_view: &DetailSubView) -> usize {
    match sub_view {
        DetailSubView::PhaseList => 0,
        DetailSubView::RoadmapViz => 1,
        DetailSubView::Backlog => 2,
        DetailSubView::GitHistory => 3,
        DetailSubView::Pipeline => 4,
        DetailSubView::Queue => 5,
        DetailSubView::Sessions => 6,
        DetailSubView::Archive => 7,
        DetailSubView::Defaults => 8,
        DetailSubView::Browse => 9,
        // Index 10, per D-15 — the 11th tab, reachable by `Left`/`Right`, by
        // `Shift+D`, and rendered by `tab_titles` at every width.
        DetailSubView::Driver => DRIVER_TAB_INDEX,
    }
}

/// `pub(crate)` for the same reason as [`tab_index`]: the round trip is the
/// property worth asserting, and it takes both halves.
pub(crate) fn sub_view_from_index(index: usize) -> DetailSubView {
    match index {
        0 => DetailSubView::PhaseList,
        1 => DetailSubView::RoadmapViz,
        2 => DetailSubView::Backlog,
        3 => DetailSubView::GitHistory,
        4 => DetailSubView::Pipeline,
        5 => DetailSubView::Queue,
        6 => DetailSubView::Sessions,
        7 => DetailSubView::Archive,
        8 => DetailSubView::Defaults,
        9 => DetailSubView::Browse,
        10 => DetailSubView::Driver,
        // Unchanged fallback: an out-of-range index still lands on the first
        // tab rather than on the newest one.
        _ => DetailSubView::PhaseList,
    }
}

/// Find a terminal emulator to use for launching Claude sessions.
/// Tries $TERMINAL env var first, then common terminal emulators.
fn find_terminal() -> Option<String> {
    if let Ok(term) = std::env::var("TERMINAL") {
        if !term.is_empty() {
            return Some(term);
        }
    }
    for candidate in &["kitty", "alacritty", "gnome-terminal", "xterm"] {
        if std::process::Command::new("which")
            .arg(candidate)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(candidate.to_string());
        }
    }
    None
}

fn status_color(category: &StatusCategory) -> Color {
    match category {
        StatusCategory::Active => Color::Green,
        StatusCategory::Idle => Color::Yellow,
        StatusCategory::Blocked => Color::Red,
        StatusCategory::Complete => Color::DarkGray,
        StatusCategory::Unknown => Color::Magenta,
    }
}

/// Compute disk-inferred status suffix spans for a phase line, e.g. " [Executing 2/3]".
/// When `show_badges` is true, appends a [verified] or [inferred] badge based on artifact presence.
fn disk_suffix_spans(
    phase_number: &str,
    phase_disk_statuses: &std::collections::HashMap<
        String,
        crate::state_reader::disk_status::DiskInference,
    >,
    show_badges: bool,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let disk_inf = phase_disk_statuses.get(phase_number);
    let disk_label = match disk_inf {
        Some(inf) => match inf.status {
            DiskStatus::NoDirectory => "Not started",
            DiskStatus::Empty => "Empty",
            DiskStatus::Discussed => "Discussed",
            DiskStatus::Researched => "Researched",
            DiskStatus::Planned => "Planned",
            DiskStatus::Partial => "Executing",
            // Implementation done, verification not passed. Deliberately NOT
            // "Complete": a phase awaiting a human's verification judgement
            // must not read as finished on the one screen a human reads.
            DiskStatus::Executed => "Executed",
            DiskStatus::Complete => "Complete",
        },
        None => "",
    };

    if !disk_label.is_empty() {
        let label = if disk_label == "Executing" {
            match disk_inf {
                Some(inf) if inf.plan_count > 0 => {
                    format!(" [Executing {}/{}]", inf.summary_count, inf.plan_count)
                }
                _ => format!(" [{}]", disk_label),
            }
        } else if disk_label == "Planned" {
            match disk_inf {
                Some(inf) if inf.plan_count > 0 => {
                    format!(" [Planned ({} plans)]", inf.plan_count)
                }
                _ => format!(" [{}]", disk_label),
            }
        } else {
            format!(" [{}]", disk_label)
        };
        spans.push(Span::raw(label));
    }

    // Append verified/inferred badge when gsd_integration is enabled
    if show_badges {
        if let Some(inf) = phase_disk_statuses.get(phase_number) {
            if inf.has_summaries || inf.has_verification {
                spans.push(Span::styled(
                    " [verified]",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::DIM),
                ));
            } else {
                spans.push(Span::styled(
                    " [inferred]",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                ));
            }
        }
    }

    spans
}

/// Switch to a new tab, handling scroll reset and data loading for backlog/git tabs.
fn switch_to_tab(
    alias: &str,
    new_index: usize,
    scroll_offset: &mut u16,
    ctx: &mut AppContext,
) -> ScreenAction {
    let new_view = sub_view_from_index(new_index);
    ctx.detail_sub_view_per_project
        .insert(alias.to_string(), new_view.clone());
    *scroll_offset = 0;
    ctx.needs_redraw = true;

    // Load data for backlog tab synchronously (fast filesystem reads)
    if new_view == DetailSubView::Backlog {
        let cache = ctx.view_cache.entry(alias.to_string()).or_default();
        if cache.backlog_items.is_empty() && !cache.loading_backlog {
            if let Some(project) = ctx.config.projects.get(alias) {
                let planning_dir = project.path.join(".planning");
                let items = backlog::parse_backlog_items(&planning_dir);
                cache.backlog_items = items;
            }
        }
    }

    // Load data for git tab asynchronously
    if new_view == DetailSubView::GitHistory {
        let cache = ctx.view_cache.entry(alias.to_string()).or_default();
        if cache.git_entries.is_empty() && !cache.loading_git {
            cache.loading_git = true;
            if let (Some(project), Some(tx)) = (ctx.config.projects.get(alias), &ctx.event_tx) {
                let tx = tx.clone();
                let project_path = project.path.clone();
                let alias_owned = alias.to_string();
                let planning_only = cache.git_planning_only;
                tokio::spawn(async move {
                    match git_ops::load_git_log(&project_path, planning_only, 50).await {
                        Ok(entries) => {
                            let _ = tx.send(Action::GitLogLoaded {
                                alias: alias_owned,
                                entries,
                                planning_only,
                            });
                        }
                        Err(_) => {
                            let _ = tx.send(Action::GitLogLoaded {
                                alias: alias_owned,
                                entries: Vec::new(),
                                planning_only,
                            });
                        }
                    }
                });
            }
        }
    }

    // Load config.json + ~/.gsd/defaults.json for defaults tab
    if new_view == DetailSubView::Defaults {
        let cache = ctx.view_cache.entry(alias.to_string()).or_default();
        if let Some(project) = ctx.config.projects.get(alias) {
            let config_path = project.path.join(".planning/config.json");
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                cache.defaults_config =
                    crate::state_reader::config_json::parse_gsd_config(&content);
            } else {
                cache.defaults_config = None;
            }
        }
        cache.defaults_user_config =
            crate::state_reader::config_json::load_user_defaults();
        cache.defaults_selected = 0;
        cache.defaults_editing = None;
        cache.defaults_dropdown_selected = 0;
        cache.defaults_text_buffer.clear();
    }

    // Lazy-init the docs browser on first visit: resolve the active phase
    // directory (or `.planning/` root if milestone complete) and populate
    // the entry list. Re-running this on subsequent visits would wipe any
    // navigation state, so we only run it when `browser_current_dir` is None.
    if new_view == DetailSubView::Browse {
        let project_path = ctx
            .config
            .projects
            .get(alias)
            .map(|p| p.path.clone());
        let project_state = ctx.project_states.get(alias).cloned();
        let cache = ctx.view_cache.entry(alias.to_string()).or_default();
        if cache.browser_current_dir.is_none() {
            if let (Some(path), Some(state)) = (project_path, project_state) {
                let planning_dir = path.join(".planning");
                let entry_dir = crate::browser::resolve_active_phase_dir(&planning_dir, &state);
                cache.browser_entries = crate::browser::list_dir(&entry_dir);
                cache.browser_root = Some(planning_dir);
                cache.browser_entry_dir = Some(entry_dir.clone());
                cache.browser_current_dir = Some(entry_dir);
                cache.browser_selected = 0;
                cache.browser_scroll_offset = 0;
                cache.browser_depth = crate::browser::BrowserDepth::List;
            }
        }
    }

    // Load this project's run list for the Driver tab, off the render thread.
    //
    // Modelled on the git arm above — the file's async-load shape — but through
    // `AppContext::schedule_run_list_scan` rather than a second inline closure,
    // because the scan is `read_dir` plus one `run.json` per run plus the
    // selected run's inbox tail and there must be exactly one copy of it (D-28).
    //
    // Unconditional on each visit rather than "only when empty": unlike the
    // backlog or the git log, this list changes while the user is looking away —
    // a run they started from the dashboard finishes, a new one begins — and a
    // cached-once run list would show a stale set of runs indefinitely. The scan
    // is bounded by the retention cap and runs on `spawn_blocking`.
    if new_view == DetailSubView::Driver {
        if let Some(path) = ctx.config.projects.get(alias).map(|p| p.path.clone()) {
            ctx.schedule_run_list_scan(alias, &path);
        }
    }

    // Load milestone list for archive tab on first visit
    if new_view == DetailSubView::Archive {
        let cache = ctx.view_cache.entry(alias.to_string()).or_default();
        if cache.archive_milestones.is_empty() && !cache.archive_loading {
            cache.archive_loading = true;
            if let (Some(project), Some(tx)) = (ctx.config.projects.get(alias), &ctx.event_tx) {
                let tx = tx.clone();
                let milestones_dir = project.path.join(".planning/milestones");
                let alias_owned = alias.to_string();
                tokio::task::spawn_blocking(move || {
                    let milestones = crate::archive::discover_milestones(&milestones_dir);
                    let _ = tx.send(Action::ArchiveMilestonesDiscovered {
                        alias: alias_owned,
                        milestones,
                    });
                });
            }
        }
    }

    ScreenAction::None
}

/// Load queue, apply a mutation, save back to disk, and reload project state.
/// Returns Ok(()) on success or an error message string.
fn queue_mutate_and_save(
    alias: &str,
    ctx: &mut AppContext,
    mutate: impl FnOnce(&mut Vec<queue_md::QueuedAction>),
) -> Result<(), String> {
    let project = ctx
        .config
        .projects
        .get(alias)
        .ok_or_else(|| "Project not found".to_string())?;
    let planning_dir = project.path.join(".planning");
    let mut actions = queue_md::load_queue(&planning_dir);
    mutate(&mut actions);
    queue_md::save_queue(&planning_dir, &actions).map_err(|e| format!("Queue error: {}", e))?;
    let new_state = state_reader::parse_project_state(&planning_dir);
    ctx.project_states.insert(alias.to_string(), new_state);
    Ok(())
}

impl Screen for DetailScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        let current_view = ctx
            .detail_sub_view_per_project
            .get(&self.alias)
            .cloned()
            .unwrap_or_default();
        let current_idx = tab_index(&current_view);

        // Text-input intercept: when the Defaults tab has a String entry being
        // edited, route all keystrokes to the input buffer so character keys
        // ('q', 'x', 'r', etc.) don't trigger their global shortcuts.
        if current_view == DetailSubView::Defaults {
            let editing_text_idx = {
                let cache = ctx.view_cache.get(&self.alias);
                let editing = cache.and_then(|c| c.defaults_editing);
                editing.and_then(|idx| {
                    cache
                        .map(entries_for_cache)
                        .and_then(|entries| entries.into_iter().nth(idx))
                        .filter(|e| matches!(e.kind, ConfigValueKind::String))
                        .map(|_| idx)
                })
            };
            if let Some(editing_idx) = editing_text_idx {
                return self.handle_text_input_key(code, ctx, editing_idx);
            }
        }

        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Archive: pop depth level before popping screen
                if current_view == DetailSubView::Archive {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    use crate::archive::ArchiveDepth;
                    match &cache.archive_depth {
                        ArchiveDepth::MilestoneList => {
                            // At root level -- fall through to pop screen
                        }
                        ArchiveDepth::PhaseList { .. } => {
                            cache.archive_depth = ArchiveDepth::MilestoneList;
                            ctx.needs_redraw = true;
                            return ScreenAction::None;
                        }
                        ArchiveDepth::FileList { milestone, .. } => {
                            cache.archive_depth =
                                ArchiveDepth::PhaseList { milestone: milestone.clone() };
                            ctx.needs_redraw = true;
                            return ScreenAction::None;
                        }
                        ArchiveDepth::FileView {
                            milestone,
                            phase_idx,
                            ..
                        } => {
                            match phase_idx {
                                Some(idx) => {
                                    // Came from FileList -- return to FileList
                                    cache.archive_depth = ArchiveDepth::FileList {
                                        milestone: milestone.clone(),
                                        phase_idx: *idx,
                                    };
                                }
                                None => {
                                    // Came from PhaseList (top-level file) -- return to PhaseList
                                    cache.archive_depth = ArchiveDepth::PhaseList {
                                        milestone: milestone.clone(),
                                    };
                                }
                            }
                            cache.archive_file_content = None;
                            cache.archive_scroll_offset = 0;
                            ctx.needs_redraw = true;
                            return ScreenAction::None;
                        }
                    }
                }
                // If diff stat pane is showing on Git tab, dismiss it first
                if current_view == DetailSubView::GitHistory {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    if cache.git_diff_stat.is_some() {
                        cache.git_diff_stat = None;
                        cache.loading_diff = false;
                        ctx.needs_redraw = true;
                        return ScreenAction::None;
                    }
                }
                // Browse: drop View→List, walk up one dir, or fall through to pop
                if current_view == DetailSubView::Browse {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    use crate::browser::BrowserDepth;
                    match cache.browser_depth {
                        BrowserDepth::View => {
                            cache.browser_depth = BrowserDepth::List;
                            cache.browser_file_content = None;
                            cache.browser_file_name = None;
                            cache.browser_scroll_offset = 0;
                            ctx.needs_redraw = true;
                            return ScreenAction::None;
                        }
                        BrowserDepth::List => {
                            let current = cache.browser_current_dir.clone();
                            let root = cache.browser_root.clone();
                            if let (Some(current), Some(root)) = (current, root) {
                                if current != root {
                                    if let Some(parent) = current.parent() {
                                        let parent = parent.to_path_buf();
                                        cache.browser_entries = crate::browser::list_dir(&parent);
                                        cache.browser_current_dir = Some(parent);
                                        cache.browser_selected = 0;
                                        cache.browser_scroll_offset = 0;
                                        ctx.needs_redraw = true;
                                        return ScreenAction::None;
                                    }
                                }
                            }
                            // At the .planning/ root: fall through to pop screen
                        }
                    }
                }
                // If a config dropdown is open on the Defaults tab, close it first
                if current_view == DetailSubView::Defaults {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    if cache.defaults_editing.is_some() {
                        cache.defaults_editing = None;
                        cache.defaults_dropdown_selected = 0;
                        cache.defaults_text_buffer.clear();
                        ctx.needs_redraw = true;
                        return ScreenAction::None;
                    }
                }
                self.scroll_offset = 0;
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match current_view {
                    DetailSubView::GitHistory => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.git_entries.is_empty() {
                            let max = cache.git_entries.len().saturating_sub(1);
                            cache.git_selected = (cache.git_selected + 1).min(max);
                            cache.git_diff_stat = None;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Backlog => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.backlog_items.is_empty() {
                            let max = cache.backlog_items.len().saturating_sub(1);
                            cache.backlog_selected = (cache.backlog_selected + 1).min(max);
                            cache.backlog_expanded = false;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Pipeline => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(state) = ctx.project_states.get(&self.alias) {
                            if !state.phases.is_empty() {
                                let max = state.phases.len().saturating_sub(1);
                                cache.pipeline_selected = (cache.pipeline_selected + 1).min(max);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Queue => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(state) = ctx.project_states.get(&self.alias) {
                            if !state.queued_actions.is_empty() {
                                let max = state.queued_actions.len().saturating_sub(1);
                                cache.queue_selected = (cache.queue_selected + 1).min(max);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Sessions => {
                        let session_count = ctx
                            .config
                            .projects
                            .get(&self.alias)
                            .map(|proj| {
                                ctx.active_sessions
                                    .iter()
                                    .filter(|s| s.working_dir == proj.path)
                                    .count()
                            })
                            .unwrap_or(0);
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if session_count > 0 {
                            let max = session_count.saturating_sub(1);
                            cache.sessions_selected = (cache.sessions_selected + 1).min(max);
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Archive => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::archive::ArchiveDepth;
                        match &cache.archive_depth {
                            ArchiveDepth::MilestoneList => {
                                let max = cache.archive_milestones.len().saturating_sub(1);
                                cache.archive_selected[0] =
                                    (cache.archive_selected[0] + 1).min(max);
                            }
                            ArchiveDepth::PhaseList { milestone } => {
                                if let Some(data) = ctx.archive_cache.get(milestone) {
                                    let total = data.top_level_files.len() + data.phases.len();
                                    let max = total.saturating_sub(1);
                                    cache.archive_selected[1] =
                                        (cache.archive_selected[1] + 1).min(max);
                                }
                            }
                            ArchiveDepth::FileList { milestone, phase_idx } => {
                                if let Some(data) = ctx.archive_cache.get(milestone) {
                                    if let Some(phase) = data.phases.get(*phase_idx) {
                                        let max = phase.files.len().saturating_sub(1);
                                        cache.archive_selected[2] =
                                            (cache.archive_selected[2] + 1).min(max);
                                    }
                                }
                            }
                            ArchiveDepth::FileView { .. } => {
                                let vp = self.archive_viewport.get();
                                cache.archive_scroll_offset = clamp_scroll(
                                    cache.archive_scroll_offset.saturating_add(1),
                                    vp.total_lines,
                                    vp.visible_height,
                                );
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Defaults => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(editing_idx) = cache.defaults_editing {
                            // Move dropdown cursor down
                            let entries = entries_for_cache(cache);
                            if let Some(entry) = entries.get(editing_idx) {
                                let options = dropdown_options(&entry.kind);
                                if !options.is_empty() {
                                    let max = options.len() - 1;
                                    cache.defaults_dropdown_selected =
                                        (cache.defaults_dropdown_selected + 1).min(max);
                                }
                            }
                        } else {
                            let entry_count = entries_count_for_cache(cache);
                            if entry_count > 0 {
                                let max = entry_count.saturating_sub(1);
                                cache.defaults_selected = (cache.defaults_selected + 1).min(max);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Browse => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::browser::BrowserDepth;
                        match cache.browser_depth {
                            BrowserDepth::List => {
                                if !cache.browser_entries.is_empty() {
                                    let max = cache.browser_entries.len().saturating_sub(1);
                                    cache.browser_selected =
                                        (cache.browser_selected + 1).min(max);
                                }
                            }
                            BrowserDepth::View => {
                                let vp = self.browser_viewport.get();
                                cache.browser_scroll_offset = clamp_scroll(
                                    cache.browser_scroll_offset.saturating_add(1),
                                    vp.total_lines,
                                    vp.visible_height,
                                );
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    // `j`/Down move the **run selection**, matching the Pipeline
                    // tab exactly. The output pane is scrolled with
                    // PageUp/PageDown and there is deliberately no pane-focus
                    // mode: the pane's default is to follow its tail, so
                    // scrolling is the exception rather than a second mode the
                    // user has to track.
                    DetailSubView::Driver => {
                        self.move_driver_selection(ctx, 1);
                    }
                    _ => {
                        // Phases and Roadmap: add the delta, then clamp.
                        let vp = self.generic_viewport.get();
                        self.scroll_offset = clamp_scroll(
                            self.scroll_offset.saturating_add(1),
                            vp.total_lines,
                            vp.visible_height,
                        );
                        ctx.needs_redraw = true;
                    }
                }
                ScreenAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match current_view {
                    DetailSubView::GitHistory => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.git_entries.is_empty() {
                            cache.git_selected = cache.git_selected.saturating_sub(1);
                            cache.git_diff_stat = None;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Backlog => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.backlog_selected = cache.backlog_selected.saturating_sub(1);
                        cache.backlog_expanded = false;
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Pipeline => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.pipeline_selected = cache.pipeline_selected.saturating_sub(1);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Queue => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.queue_selected = cache.queue_selected.saturating_sub(1);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Sessions => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.sessions_selected = cache.sessions_selected.saturating_sub(1);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Archive => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::archive::ArchiveDepth;
                        match &cache.archive_depth {
                            ArchiveDepth::MilestoneList => {
                                cache.archive_selected[0] =
                                    cache.archive_selected[0].saturating_sub(1);
                            }
                            ArchiveDepth::PhaseList { .. } => {
                                cache.archive_selected[1] =
                                    cache.archive_selected[1].saturating_sub(1);
                            }
                            ArchiveDepth::FileList { .. } => {
                                cache.archive_selected[2] =
                                    cache.archive_selected[2].saturating_sub(1);
                            }
                            ArchiveDepth::FileView { .. } => {
                                // Clamp FIRST, subtract second. The reverse order lands a
                                // stale-high offset exactly on max_scroll — the value the
                                // renderer was already displaying — so the first press would
                                // not visibly move the viewport (UIFIX-04 / WR-02).
                                let vp = self.archive_viewport.get();
                                cache.archive_scroll_offset = clamp_scroll(
                                    cache.archive_scroll_offset,
                                    vp.total_lines,
                                    vp.visible_height,
                                )
                                .saturating_sub(1);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Defaults => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if cache.defaults_editing.is_some() {
                            cache.defaults_dropdown_selected =
                                cache.defaults_dropdown_selected.saturating_sub(1);
                        } else {
                            cache.defaults_selected = cache.defaults_selected.saturating_sub(1);
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Browse => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::browser::BrowserDepth;
                        match cache.browser_depth {
                            BrowserDepth::List => {
                                cache.browser_selected =
                                    cache.browser_selected.saturating_sub(1);
                            }
                            BrowserDepth::View => {
                                // Clamp FIRST, subtract second — see the Archive sibling.
                                let vp = self.browser_viewport.get();
                                cache.browser_scroll_offset = clamp_scroll(
                                    cache.browser_scroll_offset,
                                    vp.total_lines,
                                    vp.visible_height,
                                )
                                .saturating_sub(1);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    // The run selection again — see the `j`/Down sibling.
                    DetailSubView::Driver => {
                        self.move_driver_selection(ctx, -1);
                    }
                    _ => {
                        // Phases and Roadmap: clamp FIRST, subtract second —
                        // same load-bearing order as the file-view siblings.
                        let vp = self.generic_viewport.get();
                        self.scroll_offset =
                            clamp_scroll(self.scroll_offset, vp.total_lines, vp.visible_height)
                                .saturating_sub(1);
                        ctx.needs_redraw = true;
                    }
                }
                ScreenAction::None
            }
            KeyCode::PageDown => {
                match current_view {
                    DetailSubView::GitHistory => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.git_entries.is_empty() {
                            let max = cache.git_entries.len().saturating_sub(1);
                            cache.git_selected = (cache.git_selected + PAGE_SCROLL_LINES as usize).min(max);
                            cache.git_diff_stat = None;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Backlog => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.backlog_items.is_empty() {
                            let max = cache.backlog_items.len().saturating_sub(1);
                            cache.backlog_selected = (cache.backlog_selected + PAGE_SCROLL_LINES as usize).min(max);
                            cache.backlog_expanded = false;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Pipeline => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(state) = ctx.project_states.get(&self.alias) {
                            if !state.phases.is_empty() {
                                let max = state.phases.len().saturating_sub(1);
                                cache.pipeline_selected = (cache.pipeline_selected + PAGE_SCROLL_LINES as usize).min(max);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Queue => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(state) = ctx.project_states.get(&self.alias) {
                            if !state.queued_actions.is_empty() {
                                let max = state.queued_actions.len().saturating_sub(1);
                                cache.queue_selected = (cache.queue_selected + PAGE_SCROLL_LINES as usize).min(max);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Sessions => {
                        let session_count = ctx
                            .config
                            .projects
                            .get(&self.alias)
                            .map(|proj| {
                                ctx.active_sessions
                                    .iter()
                                    .filter(|s| s.working_dir == proj.path)
                                    .count()
                            })
                            .unwrap_or(0);
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if session_count > 0 {
                            let max = session_count.saturating_sub(1);
                            cache.sessions_selected = (cache.sessions_selected + PAGE_SCROLL_LINES as usize).min(max);
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Archive => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::archive::ArchiveDepth;
                        match &cache.archive_depth {
                            ArchiveDepth::MilestoneList => {
                                let max = cache.archive_milestones.len().saturating_sub(1);
                                cache.archive_selected[0] =
                                    (cache.archive_selected[0] + PAGE_SCROLL_LINES as usize).min(max);
                            }
                            ArchiveDepth::PhaseList { milestone } => {
                                if let Some(data) = ctx.archive_cache.get(milestone) {
                                    let total = data.top_level_files.len() + data.phases.len();
                                    let max = total.saturating_sub(1);
                                    cache.archive_selected[1] =
                                        (cache.archive_selected[1] + PAGE_SCROLL_LINES as usize).min(max);
                                }
                            }
                            ArchiveDepth::FileList { milestone, phase_idx } => {
                                if let Some(data) = ctx.archive_cache.get(milestone) {
                                    if let Some(phase) = data.phases.get(*phase_idx) {
                                        let max = phase.files.len().saturating_sub(1);
                                        cache.archive_selected[2] =
                                            (cache.archive_selected[2] + PAGE_SCROLL_LINES as usize).min(max);
                                    }
                                }
                            }
                            ArchiveDepth::FileView { .. } => {
                                let vp = self.archive_viewport.get();
                                cache.archive_scroll_offset = clamp_scroll(
                                    cache.archive_scroll_offset.saturating_add(PAGE_SCROLL_LINES),
                                    vp.total_lines,
                                    vp.visible_height,
                                );
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Defaults => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        let entry_count = entries_count_for_cache(cache);
                        if entry_count > 0 {
                            let max = entry_count.saturating_sub(1);
                            cache.defaults_selected = (cache.defaults_selected + PAGE_SCROLL_LINES as usize).min(max);
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Browse => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::browser::BrowserDepth;
                        match cache.browser_depth {
                            BrowserDepth::List => {
                                if !cache.browser_entries.is_empty() {
                                    let max = cache.browser_entries.len().saturating_sub(1);
                                    cache.browser_selected = (cache.browser_selected
                                        + PAGE_SCROLL_LINES as usize)
                                        .min(max);
                                }
                            }
                            BrowserDepth::View => {
                                let vp = self.browser_viewport.get();
                                cache.browser_scroll_offset = clamp_scroll(
                                    cache.browser_scroll_offset.saturating_add(PAGE_SCROLL_LINES),
                                    vp.total_lines,
                                    vp.visible_height,
                                );
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    // The Driver output pane: **add the page step, THEN clamp**
                    // — the same order as every sibling above, and it was a
                    // shipped bug in the other direction (UIFIX-04).
                    //
                    // Reaching the bottom re-arms the follow bit, because
                    // reaching the bottom *is* the request to follow (D-19).
                    DetailSubView::Driver => {
                        let vp = self.driver_viewport.get();
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        let from =
                            driver_offset_now(cache.driver_scroll_offset, cache.driver_follow, vp);
                        cache.driver_scroll_offset = clamp_scroll(
                            from.saturating_add(PAGE_SCROLL_LINES),
                            vp.total_lines,
                            vp.visible_height,
                        );
                        cache.driver_follow = cache.driver_scroll_offset >= tail_offset(vp);
                        ctx.needs_redraw = true;
                    }
                    _ => {
                        // Phases and Roadmap: add the delta, then clamp.
                        let vp = self.generic_viewport.get();
                        self.scroll_offset = clamp_scroll(
                            self.scroll_offset.saturating_add(PAGE_SCROLL_LINES),
                            vp.total_lines,
                            vp.visible_height,
                        );
                        ctx.needs_redraw = true;
                    }
                }
                ScreenAction::None
            }
            KeyCode::PageUp => {
                match current_view {
                    DetailSubView::GitHistory => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.git_entries.is_empty() {
                            cache.git_selected = cache.git_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                            cache.git_diff_stat = None;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Backlog => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.backlog_selected = cache.backlog_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                        cache.backlog_expanded = false;
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Pipeline => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.pipeline_selected = cache.pipeline_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Queue => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.queue_selected = cache.queue_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Sessions => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.sessions_selected = cache.sessions_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Archive => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::archive::ArchiveDepth;
                        match &cache.archive_depth {
                            ArchiveDepth::MilestoneList => {
                                cache.archive_selected[0] =
                                    cache.archive_selected[0].saturating_sub(PAGE_SCROLL_LINES as usize);
                            }
                            ArchiveDepth::PhaseList { .. } => {
                                cache.archive_selected[1] =
                                    cache.archive_selected[1].saturating_sub(PAGE_SCROLL_LINES as usize);
                            }
                            ArchiveDepth::FileList { .. } => {
                                cache.archive_selected[2] =
                                    cache.archive_selected[2].saturating_sub(PAGE_SCROLL_LINES as usize);
                            }
                            ArchiveDepth::FileView { .. } => {
                                // Clamp FIRST, subtract second — see the `k`/Up sibling.
                                let vp = self.archive_viewport.get();
                                cache.archive_scroll_offset = clamp_scroll(
                                    cache.archive_scroll_offset,
                                    vp.total_lines,
                                    vp.visible_height,
                                )
                                .saturating_sub(PAGE_SCROLL_LINES);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Defaults => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.defaults_selected = cache.defaults_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Browse => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::browser::BrowserDepth;
                        match cache.browser_depth {
                            BrowserDepth::List => {
                                cache.browser_selected = cache
                                    .browser_selected
                                    .saturating_sub(PAGE_SCROLL_LINES as usize);
                            }
                            BrowserDepth::View => {
                                // Clamp FIRST, subtract second — see the `k`/Up sibling.
                                let vp = self.browser_viewport.get();
                                cache.browser_scroll_offset = clamp_scroll(
                                    cache.browser_scroll_offset,
                                    vp.total_lines,
                                    vp.visible_height,
                                )
                                .saturating_sub(PAGE_SCROLL_LINES);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    // The Driver output pane: **clamp FIRST, subtract second.**
                    // The reverse order lands a stale-high offset exactly on
                    // `max_scroll` — the value the renderer was already
                    // displaying — so the first press would not visibly move the
                    // viewport (UIFIX-04 / WR-02). `driver_offset_now` performs
                    // the clamp for both the following and the scrolled case.
                    //
                    // **Any upward scroll clears the follow bit**, automatically
                    // and without a key of its own (D-19).
                    DetailSubView::Driver => {
                        let vp = self.driver_viewport.get();
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.driver_scroll_offset =
                            driver_offset_now(cache.driver_scroll_offset, cache.driver_follow, vp)
                                .saturating_sub(PAGE_SCROLL_LINES);
                        cache.driver_follow = false;
                        ctx.needs_redraw = true;
                    }
                    _ => {
                        // Phases and Roadmap: clamp FIRST, subtract second —
                        // same load-bearing order as the file-view siblings.
                        let vp = self.generic_viewport.get();
                        self.scroll_offset =
                            clamp_scroll(self.scroll_offset, vp.total_lines, vp.visible_height)
                                .saturating_sub(PAGE_SCROLL_LINES);
                        ctx.needs_redraw = true;
                    }
                }
                ScreenAction::None
            }
            // Tab switching via number keys
            KeyCode::Char('1') => switch_to_tab(&self.alias, 0, &mut self.scroll_offset, ctx),
            KeyCode::Char('2') => switch_to_tab(&self.alias, 1, &mut self.scroll_offset, ctx),
            KeyCode::Char('3') => switch_to_tab(&self.alias, 2, &mut self.scroll_offset, ctx),
            KeyCode::Char('4') => switch_to_tab(&self.alias, 3, &mut self.scroll_offset, ctx),
            KeyCode::Char('5') => switch_to_tab(&self.alias, 4, &mut self.scroll_offset, ctx),
            KeyCode::Char('6') => switch_to_tab(&self.alias, 5, &mut self.scroll_offset, ctx),
            KeyCode::Char('7') => switch_to_tab(&self.alias, 6, &mut self.scroll_offset, ctx),
            KeyCode::Char('8') => switch_to_tab(&self.alias, 7, &mut self.scroll_offset, ctx),
            KeyCode::Char('9') => switch_to_tab(&self.alias, 8, &mut self.scroll_offset, ctx),
            KeyCode::Char('0') => switch_to_tab(&self.alias, 9, &mut self.scroll_offset, ctx),
            // The 11th tab (D-15). All ten digits are taken, uppercase is
            // entirely unclaimed in the detail view, and `KeyCode::Char('D')`
            // arrives without needing the `_modifiers` parameter this handler
            // ignores — so `Shift+D` costs no new plumbing and collides with
            // nothing.
            KeyCode::Char('D') => {
                switch_to_tab(&self.alias, DRIVER_TAB_INDEX, &mut self.scroll_offset, ctx)
            }
            // Tab switching via arrow keys
            KeyCode::Left => {
                if current_idx > 0 {
                    switch_to_tab(&self.alias, current_idx - 1, &mut self.scroll_offset, ctx)
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Right => {
                if current_idx < TAB_COUNT - 1 {
                    switch_to_tab(&self.alias, current_idx + 1, &mut self.scroll_offset, ctx)
                } else {
                    ScreenAction::None
                }
            }
            // Enter/Space: expand backlog item, load diff stat, or mark queue item done
            KeyCode::Enter | KeyCode::Char(' ') => {
                match current_view {
                    DetailSubView::Queue => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        let selected = cache.queue_selected;
                        // Get the command text before mutation
                        let command_text = ctx
                            .project_states
                            .get(&self.alias)
                            .and_then(|s| s.queued_actions.get(selected))
                            .map(|a| a.command.clone());
                        if let Some(cmd) = command_text {
                            match queue_mutate_and_save(&self.alias, ctx, |actions| {
                                if selected < actions.len() {
                                    actions.remove(selected);
                                }
                            }) {
                                Ok(()) => {
                                    // Clamp selection
                                    let cache =
                                        ctx.view_cache.entry(self.alias.clone()).or_default();
                                    let new_len = ctx
                                        .project_states
                                        .get(&self.alias)
                                        .map(|s| s.queued_actions.len())
                                        .unwrap_or(0);
                                    if new_len == 0 {
                                        cache.queue_selected = 0;
                                    } else if cache.queue_selected >= new_len {
                                        cache.queue_selected = new_len - 1;
                                    }
                                    ctx.status_message =
                                        Some((format!("Done: {}", cmd), std::time::Instant::now()));
                                }
                                Err(e) => {
                                    ctx.status_message = Some((e, std::time::Instant::now()));
                                }
                            }
                            ctx.needs_redraw = true;
                        }
                        ScreenAction::None
                    }
                    DetailSubView::Backlog => {
                        // Space does nothing on backlog; Enter toggles expand
                        if code == KeyCode::Char(' ') {
                            return ScreenAction::None;
                        }
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.backlog_items.is_empty() {
                            cache.backlog_expanded = !cache.backlog_expanded;
                            // Load content if expanding and not yet loaded
                            if cache.backlog_expanded {
                                let selected = cache.backlog_selected;
                                if let Some(item) = cache.backlog_items.get(selected) {
                                    if item.content.is_none() {
                                        if let Some(project) = ctx.config.projects.get(&self.alias)
                                        {
                                            let planning_dir = project.path.join(".planning");
                                            // A PATH SEGMENT — the directory to
                                            // read from — so the raw bytes are
                                            // what the filesystem needs.
                                            let content = backlog::load_backlog_content(
                                                &planning_dir,
                                                item.dir_name.as_raw_for_logic_only(),
                                            );
                                            if let Some(content) = content {
                                                // A file BODY read off disk;
                                                // the carrier travels with it.
                                                cache.backlog_items[selected].content = Some(
                                                    crate::text::Untrusted::from_untrusted_source(
                                                        content,
                                                    ),
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            ctx.needs_redraw = true;
                        }
                        ScreenAction::None
                    }
                    DetailSubView::GitHistory => {
                        if code == KeyCode::Char(' ') {
                            return ScreenAction::None;
                        }
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(entry) = cache.git_entries.get(cache.git_selected) {
                            // RAW: this hash becomes an argv element of
                            // `git diff-tree ... <hash>`. A subprocess argument
                            // is a lookup, not something a human reads, and an
                            // escaped hash would name no commit.
                            let hash = entry.hash.as_raw_for_logic_only().to_string();
                            cache.loading_diff = true;
                            if let (Some(project), Some(tx)) =
                                (ctx.config.projects.get(&self.alias), &ctx.event_tx)
                            {
                                let tx = tx.clone();
                                let project_path = project.path.clone();
                                let alias = self.alias.clone();
                                tokio::spawn(async move {
                                    match git_ops::load_diff_stat(&project_path, &hash).await {
                                        Ok(stat) => {
                                            let _ =
                                                tx.send(Action::GitDiffStatLoaded { alias, stat });
                                        }
                                        Err(_) => {
                                            let _ = tx.send(Action::GitDiffStatLoaded {
                                                alias,
                                                stat: Default::default(),
                                            });
                                        }
                                    }
                                });
                            }
                            ctx.needs_redraw = true;
                        }
                        ScreenAction::None
                    }
                    DetailSubView::Sessions => {
                        // Resume selected session in a new terminal
                        let filtered_sessions: Vec<_> = ctx
                            .config
                            .projects
                            .get(&self.alias)
                            .map(|proj| {
                                ctx.active_sessions
                                    .iter()
                                    .filter(|s| s.working_dir == proj.path)
                                    .cloned()
                                    .collect()
                            })
                            .unwrap_or_default();
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(session) = filtered_sessions.get(cache.sessions_selected) {
                            if let Some(ref sid) = session.session_id {
                                match find_terminal() {
                                    Some(term) => {
                                        // READ BY A HUMAN (it lands in a status
                                        // message below), and shortened by
                                        // CHARACTERS — see
                                        // `shorten_session_id`'s doc for why
                                        // the byte slice this replaced was a
                                        // panic.
                                        let short_id = shorten_session_id(sid);
                                        match std::process::Command::new(&term)
                                            .args([
                                                "-e",
                                                "sh",
                                                "-c",
                                                // A SUBPROCESS ARGUMENT: the
                                                // raw id is what `claude
                                                // --resume` must receive, and
                                                // an escaped one would resume
                                                // nothing.
                                                &format!(
                                                    "cd '{}' && claude --resume '{}'",
                                                    session.working_dir.display(),
                                                    sid.as_raw_for_logic_only()
                                                ),
                                            ])
                                            .spawn()
                                        {
                                            Ok(_) => {
                                                return ScreenAction::SetStatusMessage(format!(
                                                    "Resumed session {}",
                                                    short_id
                                                ))
                                            }
                                            Err(e) => {
                                                return ScreenAction::SetStatusMessage(format!(
                                                    "Failed to launch: {}",
                                                    e
                                                ))
                                            }
                                        }
                                    }
                                    None => {
                                        return ScreenAction::SetStatusMessage(
                                            "No terminal emulator found (set $TERMINAL)"
                                                .to_string(),
                                        )
                                    }
                                }
                            } else {
                                return ScreenAction::SetStatusMessage(
                                    "No session ID to resume".to_string(),
                                );
                            }
                        }
                        ScreenAction::None
                    }
                    DetailSubView::Archive => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::archive::ArchiveDepth;
                        match cache.archive_depth.clone() {
                            ArchiveDepth::MilestoneList => {
                                let selected = cache.archive_selected[0];
                                // A NAVIGATION KEY: it becomes the
                                // `ArchiveDepth` discriminant, a
                                // `ctx.archive_cache` map key and a path
                                // segment under `.planning/milestones`, so the
                                // raw bytes are what all three need. Nothing
                                // below draws it; the render reads it back out
                                // of the cache and escapes it there.
                                if let Some(milestone) = cache
                                    .archive_milestones
                                    .get(selected)
                                    .map(|m| m.as_raw_for_logic_only().to_string())
                                {
                                    cache.archive_depth = ArchiveDepth::PhaseList {
                                        milestone: milestone.clone(),
                                    };
                                    cache.archive_selected[1] = 0;
                                    // Trigger async load if not cached
                                    if !ctx.archive_cache.contains_key(&milestone) {
                                        cache.archive_loading = true;
                                        if let Some(project) = ctx.config.projects.get(&self.alias)
                                        {
                                            let milestones_dir =
                                                project.path.join(".planning/milestones");
                                            if let Some(tx) = &ctx.event_tx {
                                                let tx = tx.clone();
                                                let alias = self.alias.clone();
                                                let ms = milestone.clone();
                                                tokio::task::spawn_blocking(move || {
                                                    let data =
                                                        crate::archive::load_milestone_archive(
                                                            &milestones_dir,
                                                            &ms,
                                                        );
                                                    let _ = tx.send(Action::ArchiveLoaded {
                                                        alias,
                                                        milestone: ms,
                                                        data,
                                                    });
                                                });
                                            }
                                        }
                                    }
                                    ctx.needs_redraw = true;
                                }
                            }
                            ArchiveDepth::PhaseList { milestone } => {
                                let selected = cache.archive_selected[1];
                                if let Some(data) = ctx.archive_cache.get(&milestone) {
                                    let top_count = data.top_level_files.len();
                                    if selected < top_count {
                                        // Selected a top-level file
                                        let file = &data.top_level_files[selected];
                                        let path = file.path.clone();
                                        cache.archive_file_name = Some(file.name.clone());
                                        cache.archive_depth = ArchiveDepth::FileView {
                                            milestone: milestone.clone(),
                                            phase_idx: None,
                                            file_idx: selected,
                                        };
                                        cache.archive_scroll_offset = 0;
                                        cache.archive_file_content =
                                            Some(crate::archive::read_archive_file(&path));
                                    } else {
                                        // Selected a phase
                                        let phase_idx = selected - top_count;
                                        cache.archive_depth = ArchiveDepth::FileList {
                                            milestone: milestone.clone(),
                                            phase_idx,
                                        };
                                        cache.archive_selected[2] = 0;
                                    }
                                    ctx.needs_redraw = true;
                                }
                            }
                            ArchiveDepth::FileList {
                                milestone,
                                phase_idx,
                            } => {
                                let selected = cache.archive_selected[2];
                                if let Some(data) = ctx.archive_cache.get(&milestone) {
                                    if let Some(phase) = data.phases.get(phase_idx) {
                                        if let Some(file) = phase.files.get(selected) {
                                            let path = file.path.clone();
                                            cache.archive_file_name = Some(file.name.clone());
                                            cache.archive_depth = ArchiveDepth::FileView {
                                                milestone: milestone.clone(),
                                                phase_idx: Some(phase_idx),
                                                file_idx: selected,
                                            };
                                            cache.archive_scroll_offset = 0;
                                            cache.archive_file_content =
                                                Some(crate::archive::read_archive_file(&path));
                                            ctx.needs_redraw = true;
                                        }
                                    }
                                }
                            }
                            ArchiveDepth::FileView { .. } => {
                                // No action on Enter in file view
                            }
                        }
                        ScreenAction::None
                    }
                    DetailSubView::Defaults => {
                        let project_path = ctx
                            .config
                            .projects
                            .get(&self.alias)
                            .map(|p| p.path.clone());
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        let target = cache.defaults_edit_target;
                        if let Some(editing_idx) = cache.defaults_editing {
                            // Dropdown is open — Enter applies the selected option.
                            let entries = entries_for_cache(cache);
                            if let Some(entry) = entries.get(editing_idx).cloned() {
                                let options = dropdown_options(&entry.kind);
                                let dropdown_idx = cache.defaults_dropdown_selected.min(options.len().saturating_sub(1));
                                if let Some(value) = options.get(dropdown_idx).cloned() {
                                    if let Some(active) = active_config_mut(cache) {
                                        if set_config_value(active, entry.key, &value) {
                                            persist_active_config(target, project_path.as_deref(), active, &mut ctx.status_message);
                                        }
                                    }
                                }
                            }
                            cache.defaults_editing = None;
                            cache.defaults_dropdown_selected = 0;
                        } else {
                            let entries = entries_for_cache(cache);
                            let selected = cache.defaults_selected;
                            if let Some(entry) = entries.get(selected).cloned() {
                                let options = dropdown_options(&entry.kind);
                                if !options.is_empty() {
                                    let current_idx = options.iter().position(|o| o == &entry.value).unwrap_or(0);
                                    cache.defaults_editing = Some(selected);
                                    cache.defaults_dropdown_selected = current_idx;
                                } else if matches!(entry.kind, ConfigValueKind::String) {
                                    cache.defaults_editing = Some(selected);
                                    cache.defaults_text_buffer =
                                        if entry.value == "(unset)" {
                                            String::new()
                                        } else {
                                            entry.value.clone()
                                        };
                                } else if matches!(entry.kind, ConfigValueKind::Integer) {
                                    if let Some(active) = active_config_mut(cache) {
                                        if mutate_config_entry(active, entry.key, &entry.kind) {
                                            persist_active_config(target, project_path.as_deref(), active, &mut ctx.status_message);
                                        }
                                    }
                                }
                            }
                        }
                        ctx.needs_redraw = true;
                        ScreenAction::None
                    }
                    DetailSubView::Browse => {
                        // Space does nothing; only Enter descends/opens
                        if code == KeyCode::Char(' ') {
                            return ScreenAction::None;
                        }
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if cache.browser_depth != crate::browser::BrowserDepth::List {
                            return ScreenAction::None;
                        }
                        let entry = cache
                            .browser_entries
                            .get(cache.browser_selected)
                            .cloned();
                        if let Some(entry) = entry {
                            if entry.is_dir {
                                cache.browser_entries = crate::browser::list_dir(&entry.path);
                                cache.browser_current_dir = Some(entry.path);
                                cache.browser_selected = 0;
                                cache.browser_scroll_offset = 0;
                            } else {
                                cache.browser_file_content =
                                    Some(crate::browser::read_md_file(&entry.path));
                                cache.browser_file_name = Some(entry.name.clone());
                                cache.browser_depth = crate::browser::BrowserDepth::View;
                                cache.browser_scroll_offset = 0;
                            }
                            ctx.needs_redraw = true;
                        }
                        ScreenAction::None
                    }
                    _ => ScreenAction::None,
                }
            }
            // Docs browser: 'g' jumps to .planning/ root
            KeyCode::Char('g') if current_view == DetailSubView::Browse => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                if let Some(root) = cache.browser_root.clone() {
                    cache.browser_entries = crate::browser::list_dir(&root);
                    cache.browser_current_dir = Some(root);
                    cache.browser_selected = 0;
                    cache.browser_scroll_offset = 0;
                    cache.browser_depth = crate::browser::BrowserDepth::List;
                    cache.browser_file_content = None;
                    cache.browser_file_name = None;
                    ctx.needs_redraw = true;
                }
                ScreenAction::None
            }
            // Docs browser: 'p' jumps back to the entry phase directory
            KeyCode::Char('p') if current_view == DetailSubView::Browse => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                if let Some(entry_dir) = cache.browser_entry_dir.clone() {
                    cache.browser_entries = crate::browser::list_dir(&entry_dir);
                    cache.browser_current_dir = Some(entry_dir);
                    cache.browser_selected = 0;
                    cache.browser_scroll_offset = 0;
                    cache.browser_depth = crate::browser::BrowserDepth::List;
                    cache.browser_file_content = None;
                    cache.browser_file_name = None;
                    ctx.needs_redraw = true;
                }
                ScreenAction::None
            }
            // 'x' key: clear (unset) the value of the selected config row
            KeyCode::Char('x') if current_view == DetailSubView::Defaults => {
                let project_path = ctx
                    .config
                    .projects
                    .get(&self.alias)
                    .map(|p| p.path.clone());
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                let target = cache.defaults_edit_target;
                if cache.defaults_editing.is_none() {
                    let entries = entries_for_cache(cache);
                    let selected = cache.defaults_selected;
                    if let Some(entry) = entries.get(selected).cloned() {
                        let key = entry.key;
                        if let Some(active) = active_config_mut(cache) {
                            let cleared = clear_config_value(active, key);
                            if cleared {
                                persist_active_config(
                                    target,
                                    project_path.as_deref(),
                                    active,
                                    &mut ctx.status_message,
                                );
                                ctx.status_message = Some((
                                    format!("Cleared {}", key),
                                    std::time::Instant::now(),
                                ));
                            } else {
                                ctx.status_message = Some((
                                    format!("{} cannot be cleared", key),
                                    std::time::Instant::now(),
                                ));
                            }
                        }
                    }
                }
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // 'r' key: reload config (Defaults tab only)
            KeyCode::Char('r') if current_view == DetailSubView::Defaults => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                if let Some(project) = ctx.config.projects.get(&self.alias) {
                    let config_path = project.path.join(".planning/config.json");
                    if let Ok(content) = std::fs::read_to_string(&config_path) {
                        cache.defaults_config =
                            crate::state_reader::config_json::parse_gsd_config(&content);
                    } else {
                        cache.defaults_config = None;
                    }
                }
                cache.defaults_user_config =
                    crate::state_reader::config_json::load_user_defaults();
                ctx.status_message = Some(("Config reloaded".to_string(), std::time::Instant::now()));
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // Tab: switch the host terminal to a Claude session for this project.
            // From the Sessions tab, jump to the highlighted session.
            // From any other tab, jump to the first session matching the project's path.
            KeyCode::Tab => {
                let project_path = ctx
                    .config
                    .projects
                    .get(&self.alias)
                    .map(|p| p.path.clone());
                let session = if current_view == DetailSubView::Sessions {
                    let filtered: Vec<_> = project_path
                        .as_ref()
                        .map(|path| {
                            ctx.active_sessions
                                .iter()
                                .filter(|s| s.working_dir == *path)
                                .cloned()
                                .collect()
                        })
                        .unwrap_or_default();
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    filtered.get(cache.sessions_selected).cloned()
                } else {
                    project_path.and_then(|path| {
                        ctx.active_sessions
                            .iter()
                            .find(|s| s.working_dir == path)
                            .cloned()
                    })
                };
                let msg = match session {
                    Some(s) => match crate::terminal_switch::switch_to_session(&s) {
                        Ok(()) => format!("Switched to {}", self.alias),
                        Err(e) => e,
                    },
                    None => format!("No active Claude session for {}", self.alias),
                };
                ctx.status_message = Some((msg, std::time::Instant::now()));
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // 'd' key: toggle between editing project config and ~/.gsd/defaults.json
            KeyCode::Char('d') if current_view == DetailSubView::Defaults => {
                use super::DefaultsEditTarget;
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                if cache.defaults_editing.is_some() {
                    return ScreenAction::None;
                }
                cache.defaults_edit_target = match cache.defaults_edit_target {
                    DefaultsEditTarget::Project => DefaultsEditTarget::Global,
                    DefaultsEditTarget::Global => DefaultsEditTarget::Project,
                };
                // Bootstrap an empty global defaults so toggling into the
                // Global view always has something editable, even before
                // ~/.gsd/defaults.json exists on disk.
                if cache.defaults_edit_target == DefaultsEditTarget::Global
                    && cache.defaults_user_config.is_none()
                {
                    cache.defaults_user_config =
                        Some(crate::state_reader::config_json::GsdConfig::default());
                }
                cache.defaults_selected = 0;
                let label = match cache.defaults_edit_target {
                    DefaultsEditTarget::Project => "Editing project config",
                    DefaultsEditTarget::Global => "Editing ~/.gsd/defaults.json",
                };
                ctx.status_message = Some((label.to_string(), std::time::Instant::now()));
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // 'n' key: launch new Claude session (Sessions tab only)
            KeyCode::Char('n') if current_view == DetailSubView::Sessions => {
                if let Some(project) = ctx.config.projects.get(&self.alias) {
                    match find_terminal() {
                        Some(term) => {
                            match std::process::Command::new(&term)
                                .args([
                                    "-e",
                                    "sh",
                                    "-c",
                                    &format!("cd '{}' && claude", project.path.display()),
                                ])
                                .spawn()
                            {
                                Ok(_) => ScreenAction::SetStatusMessage(
                                    "Launched new Claude session".to_string(),
                                ),
                                Err(e) => ScreenAction::SetStatusMessage(format!(
                                    "Failed to launch: {}",
                                    e
                                )),
                            }
                        }
                        None => ScreenAction::SetStatusMessage(
                            "No terminal emulator found (set $TERMINAL)".to_string(),
                        ),
                    }
                } else {
                    ScreenAction::None
                }
            }
            // Toggle planning-only filter for git tab
            KeyCode::Char('p') if current_view == DetailSubView::GitHistory => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                cache.git_planning_only = !cache.git_planning_only;
                cache.git_entries.clear();
                cache.git_diff_stat = None;
                cache.git_selected = 0;
                cache.loading_git = true;
                if let (Some(project), Some(tx)) =
                    (ctx.config.projects.get(&self.alias), &ctx.event_tx)
                {
                    let tx = tx.clone();
                    let project_path = project.path.clone();
                    let alias = self.alias.clone();
                    let planning_only = cache.git_planning_only;
                    tokio::spawn(async move {
                        match git_ops::load_git_log(&project_path, planning_only, 50).await {
                            Ok(entries) => {
                                let _ = tx.send(Action::GitLogLoaded {
                                    alias,
                                    entries,
                                    planning_only,
                                });
                            }
                            Err(_) => {
                                let _ = tx.send(Action::GitLogLoaded {
                                    alias,
                                    entries: Vec::new(),
                                    planning_only,
                                });
                            }
                        }
                    });
                }
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // 'a' key: add new queue item (Queue tab only)
            KeyCode::Char('a') if current_view == DetailSubView::Queue => {
                let alias = self.alias.clone();
                let has_planning = ctx
                    .config
                    .projects
                    .get(&alias)
                    .map(|p| p.path.join(".planning").is_dir())
                    .unwrap_or(false);
                if !has_planning {
                    ctx.needs_redraw = true;
                    ScreenAction::SetStatusMessage(
                        "Run GSD in this project first to enable queue".to_string(),
                    )
                } else {
                    ctx.input_buffer.clear();
                    ctx.suggestion_index = 0;
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(EnqueueScreen::new(alias)))
                }
            }
            // 'd' or 'x' key: delete queue item with confirmation (Queue tab only)
            KeyCode::Char('d') | KeyCode::Char('x') if current_view == DetailSubView::Queue => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                let selected = cache.queue_selected;
                let command_text = ctx
                    .project_states
                    .get(&self.alias)
                    .and_then(|s| s.queued_actions.get(selected))
                    .map(|a| a.command.clone());
                if let Some(cmd) = command_text {
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(QueueDeleteConfirmScreen::new(
                        self.alias.clone(),
                        selected,
                        cmd,
                    )))
                } else {
                    ScreenAction::None
                }
            }
            // Shift+J: move queue item down
            KeyCode::Char('J') if current_view == DetailSubView::Queue => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                let selected = cache.queue_selected;
                let len = ctx
                    .project_states
                    .get(&self.alias)
                    .map(|s| s.queued_actions.len())
                    .unwrap_or(0);
                if len > 1 && selected < len - 1 {
                    match queue_mutate_and_save(&self.alias, ctx, |actions| {
                        if selected < actions.len() - 1 {
                            actions.swap(selected, selected + 1);
                        }
                    }) {
                        Ok(()) => {
                            let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                            cache.queue_selected = selected + 1;
                        }
                        Err(e) => {
                            ctx.status_message = Some((e, std::time::Instant::now()));
                        }
                    }
                    ctx.needs_redraw = true;
                }
                ScreenAction::None
            }
            // Shift+K: move queue item up
            KeyCode::Char('K') if current_view == DetailSubView::Queue => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                let selected = cache.queue_selected;
                if selected > 0 {
                    match queue_mutate_and_save(&self.alias, ctx, |actions| {
                        if selected > 0 && selected < actions.len() {
                            actions.swap(selected, selected - 1);
                        }
                    }) {
                        Ok(()) => {
                            let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                            cache.queue_selected = selected - 1;
                        }
                        Err(e) => {
                            ctx.status_message = Some((e, std::time::Instant::now()));
                        }
                    }
                    ctx.needs_redraw = true;
                }
                ScreenAction::None
            }
            // `f`: toggle the output pane's follow bit (Driver tab only).
            //
            // Turning it **on** jumps to the tail, which is the only thing
            // "follow" can mean; turning it off leaves the viewport exactly
            // where it is, so the key never moves the text out from under the
            // reader (D-19).
            KeyCode::Char('f') if current_view == DetailSubView::Driver => {
                let vp = self.driver_viewport.get();
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                if cache.driver_follow {
                    cache.driver_scroll_offset = tail_offset(vp);
                    cache.driver_follow = false;
                } else {
                    cache.driver_follow = true;
                }
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // `G`: jump to the tail **and** re-enable follow (Driver tab only).
            KeyCode::Char('G') if current_view == DetailSubView::Driver => {
                let vp = self.driver_viewport.get();
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                cache.driver_scroll_offset = tail_offset(vp);
                cache.driver_follow = true;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // `i`: open the injection input — **only when there is a live run**
            // (STEER-01, D-22).
            //
            // `DriverInjectScreen`'s contract is that it is constructed with the
            // id of a run that is live *now*, so the refusal belongs here rather
            // than inside it. The refusal sets the pinned message and
            // **dispatches nothing**: a screen that set a message and dispatched
            // anyway would pass a test that only checked the message, which is
            // the shape `driver_confirm.rs:479-486` records as the load-bearing
            // half.
            //
            // Liveness is `ObservedRun::is_live` — positively observed running —
            // and never `LivenessUnknown`, because opening an input aimed at a
            // run nothing can speak for would queue a message that may already
            // be undeliverable.
            //
            // **The target is the SELECTED run, not merely the observed one**
            // (WR-01). `j`/`k` move the selection freely across the whole run
            // history while `observed_runs` holds only the run this session is
            // tailing, so reading the id out of that map alone aimed the input
            // at today's live run while the user was looking at last week's.
            // Worse, `cache.driver_inbox` is the **selected** run's inbox, so
            // the queued message did not appear anywhere on the surface they
            // were looking at. Requiring the two to agree is what keeps the
            // target and the pane the same run.
            KeyCode::Char('i') if current_view == DetailSubView::Driver => {
                let selected_run_id = ctx.view_cache.get(&self.alias).and_then(|cache| {
                    cache
                        .driver_runs
                        .get(cache.driver_selected_run)
                        .map(|run| run.run_id.clone())
                });
                let live_run_id = ctx
                    .observed_runs
                    .get(&self.alias)
                    .filter(|observed| observed.is_live())
                    .map(|observed| observed.run_id.clone());
                ctx.needs_redraw = true;
                match live_run_id {
                    // The one case that opens the input: a live run, and it is
                    // the one on screen.
                    Some(run_id) if Some(&run_id) == selected_run_id.as_ref() => {
                        ScreenAction::Push(Box::new(DriverInjectScreen::new(
                            self.alias.clone(),
                            run_id,
                        )))
                    }
                    // A live run exists, but not the one being reviewed. Saying
                    // "no live run" here would contradict a run list that
                    // visibly contains one.
                    Some(_) => {
                        ctx.status_message = Some((
                            super::driver_inject::not_the_selected_run_message(&self.alias),
                            std::time::Instant::now(),
                        ));
                        ScreenAction::None
                    }
                    None => {
                        ctx.status_message = Some((
                            super::driver_inject::no_live_run_message(&self.alias),
                            std::time::Instant::now(),
                        ));
                        ScreenAction::None
                    }
                }
            }
            // `s`: the start flow (Surface 5), which asks for the command and
            // the goal before it reaches the confirmation the dashboard's `r`
            // reaches directly.
            KeyCode::Char('s') if current_view == DetailSubView::Driver => {
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(DriverStartScreen::new(self.alias.clone())))
            }
            // `x`: stop, **through the existing confirmation** and never
            // directly.
            //
            // Tab-scoped, and the collision is deliberate: `x` is bound on the
            // Defaults tab (clear a value) and on the Queue tab (delete), both
            // untouched, and per-tab reuse is the established pattern — `d` is
            // already bound twice (D-15). Using `x` here matches the dashboard's
            // `x` = stop run, and that verb/key consistency across the two
            // surfaces is worth more than a unique letter.
            KeyCode::Char('x') if current_view == DetailSubView::Driver => {
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(DriverConfirmScreen::new(
                    self.alias.clone(),
                    DriverAction::Stop,
                )))
            }
            // `o`: opt in, **through the same confirmation the dashboard's `o`
            // opens** and never by writing the registry from here.
            //
            // `o` is already what the dashboard binds for exactly this action
            // (`normal.rs:444-454`), so the verb/key mapping stays consistent
            // across the two surfaces — the same argument the `x` comment above
            // makes for reusing `x`.
            //
            // Unlike `i`/`s`/`x` there was no collision to resolve: a grep for
            // `Char('o')` and `Char('O')` across this file found zero existing
            // bindings on any detail tab. The `current_view` guard is present
            // **despite** that, so the key stays scoped to the tab that
            // explains it rather than silently becoming a global
            // detail-screen opt-in on tabs where it is undiscoverable and
            // unannounced.
            //
            // `DriverConfirmScreen`'s `do_toggle_opt_in` stays the ONE opt-in
            // write path (CTRL-03). A second one here would be a second thing
            // that has to stay in agreement with the spawn seam in
            // `executor::DrivableProject::from_registry`.
            KeyCode::Char('o') if current_view == DetailSubView::Driver => {
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(DriverConfirmScreen::new(
                    self.alias.clone(),
                    DriverAction::ToggleOptIn,
                )))
            }
            KeyCode::Char('e') => {
                // Archive FileView: open file in $EDITOR (read-only for milestones)
                if current_view == DetailSubView::Archive {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    use crate::archive::ArchiveDepth;
                    if let ArchiveDepth::FileView {
                        milestone,
                        phase_idx,
                        file_idx,
                    } = &cache.archive_depth
                    {
                        // Resolve the file path from archive cache
                        let file_path = ctx.archive_cache.get(milestone).and_then(|data| {
                            match phase_idx {
                                None => {
                                    // Top-level file (entered from PhaseList)
                                    data.top_level_files.get(*file_idx).map(|f| f.path.clone())
                                }
                                Some(idx) => {
                                    data.phases
                                        .get(*idx)
                                        .and_then(|p| p.files.get(*file_idx))
                                        .map(|f| f.path.clone())
                                }
                            }
                        });

                        if let Some(path) = file_path {
                            // Check if file is under milestones/ (archived = read-only)
                            let path_str = path.to_string_lossy();
                            if path_str.contains("/milestones/") {
                                ctx.needs_redraw = true;
                                return ScreenAction::SetStatusMessage(
                                    "Archived files are read-only".to_string(),
                                );
                            }
                            ctx.needs_redraw = true;
                            return ScreenAction::SuspendAndEdit(path);
                        }
                    }
                    // Not in FileView depth -- no-op for 'e'
                    return ScreenAction::None;
                }

                // Docs (Browse) tab: open the selected markdown file in $EDITOR.
                // Resolved in a scope so the view-cache borrow ends before the
                // surrounding context fields are touched.
                if current_view == DetailSubView::Browse {
                    let target = {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        browse_edit_target(cache)
                    };
                    ctx.needs_redraw = true;
                    return match target {
                        Ok(path) => ScreenAction::SuspendAndEdit(path),
                        Err(msg) => ScreenAction::SetStatusMessage(msg.to_string()),
                    };
                }

                let alias = self.alias.clone();
                let has_planning = ctx
                    .config
                    .projects
                    .get(&alias)
                    .map(|p| p.path.join(".planning").is_dir())
                    .unwrap_or(false);
                if !has_planning {
                    ctx.needs_redraw = true;
                    ScreenAction::SetStatusMessage(
                        "Run GSD in this project first to enable queue".to_string(),
                    )
                } else if current_view == DetailSubView::Queue {
                    // Queue tab: edit selected item (pre-fill + remove old)
                    let cache = ctx.view_cache.entry(alias.clone()).or_default();
                    let selected = cache.queue_selected;
                    let command_text = ctx
                        .project_states
                        .get(&alias)
                        .and_then(|s| s.queued_actions.get(selected))
                        .map(|a| a.command.clone());
                    if let Some(cmd) = command_text {
                        ctx.input_buffer = cmd.clone();
                        ctx.suggestion_index = 0;
                        // Remove the item first; EnqueueScreen will re-add on Enter
                        match queue_mutate_and_save(&alias, ctx, |actions| {
                            if selected < actions.len() {
                                actions.remove(selected);
                            }
                        }) {
                            Ok(()) => {
                                // Clamp selection after removal
                                let cache = ctx.view_cache.entry(alias.clone()).or_default();
                                let new_len = ctx
                                    .project_states
                                    .get(&alias)
                                    .map(|s| s.queued_actions.len())
                                    .unwrap_or(0);
                                if new_len == 0 {
                                    cache.queue_selected = 0;
                                } else if cache.queue_selected >= new_len {
                                    cache.queue_selected = new_len - 1;
                                }
                                ctx.status_message = Some((
                                    format!("Editing: {} (Esc cancels and removes)", cmd),
                                    std::time::Instant::now(),
                                ));
                            }
                            Err(e) => {
                                ctx.status_message = Some((e, std::time::Instant::now()));
                            }
                        }
                        ctx.needs_redraw = true;
                        ScreenAction::Push(Box::new(EnqueueScreen::new(alias)))
                    } else {
                        ScreenAction::None
                    }
                } else if current_view == DetailSubView::Backlog {
                    // Backlog tab: if expanded, open file in $EDITOR; otherwise enqueue
                    let cache = ctx.view_cache.entry(alias.clone()).or_default();
                    if cache.backlog_expanded {
                        if let Some(item) = cache.backlog_items.get(cache.backlog_selected) {
                            if let Some(ref path) = item.path {
                                ctx.needs_redraw = true;
                                return ScreenAction::SuspendAndEdit(path.clone());
                            }
                        }
                        return ScreenAction::SetStatusMessage(
                            "No file found for this backlog item".to_string(),
                        );
                    }
                    // Not expanded: enqueue as before
                    if let Some(item) = cache.backlog_items.get(cache.backlog_selected) {
                        // A COMMAND ARGUMENT that is then echoed into the
                        // enqueue footer and written to `.planning/queue.md`.
                        // The raw directory name is what the command must
                        // carry, and the ECHO of `ctx.input_buffer` is escaped
                        // at its own render site (EnqueueScreen's footer),
                        // which the probe covers.
                        ctx.input_buffer = format!(
                            "/gsd:review-backlog {}",
                            item.dir_name.as_raw_for_logic_only()
                        );
                    } else {
                        ctx.input_buffer.clear();
                    }
                    ctx.suggestion_index = 0;
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(EnqueueScreen::new(alias)))
                } else {
                    // Generic: use suggest_next_commands
                    if let Some(state) = ctx.project_states.get(&alias) {
                        let suggestions = queue_md::suggest_next_commands(state);
                        if !suggestions.is_empty() {
                            ctx.input_buffer = suggestions[0].clone();
                        } else {
                            ctx.input_buffer.clear();
                        }
                    } else {
                        ctx.input_buffer.clear();
                    }
                    ctx.suggestion_index = 0;
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(EnqueueScreen::new(alias)))
                }
            }
            KeyCode::Char('?') => {
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(HelpScreen::new()))
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;

        let sub_view = ctx
            .detail_sub_view_per_project
            .get(alias)
            .cloned()
            .unwrap_or_default();
        let tab_idx = tab_index(&sub_view);

        // Split into: tab bar, content, footer
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
        let tab_area = chunks[0];
        let content_area = chunks[1];
        let footer_area = chunks[2];

        // Render tab bar. Both this site and its duplicate in
        // `render_main_only` take their titles from `tab_titles`; a tier applied
        // to only one of them would leave the Driver tab visible on one render
        // path and invisible on the other.
        let (titles, select) = tab_titles(tab_area.width, tab_idx, driver_live_for(ctx, alias));
        let tabs_widget = Tabs::new(titles)
            .select(select)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )
            .divider("|");
        let tab_block = Block::default()
            .borders(Borders::BOTTOM)
            .title(format!(" Project: {} ", shown(alias)));
        frame.render_widget(tabs_widget.block(tab_block), tab_area);

        // Render content based on active tab
        match sub_view {
            DetailSubView::PhaseList => self.render_phase_list(frame, content_area, ctx),
            DetailSubView::RoadmapViz => self.render_roadmap(frame, content_area, ctx),
            DetailSubView::Backlog => self.render_backlog_tab(frame, content_area, ctx),
            DetailSubView::GitHistory => self.render_git_tab(frame, content_area, ctx),
            DetailSubView::Pipeline => self.render_pipeline_tab(frame, content_area, ctx),
            DetailSubView::Queue => self.render_queue_tab(frame, content_area, ctx),
            DetailSubView::Sessions => self.render_sessions_tab(frame, content_area, ctx),
            DetailSubView::Archive => self.render_archive_tab(frame, content_area, ctx),
            DetailSubView::Defaults => self.render_defaults_tab(frame, content_area, ctx),
            DetailSubView::Browse => self.render_browser_tab(frame, content_area, ctx),
            // The Driver tab renders from its own module — the one sub-tab that
            // does. `driver.rs`'s header doc records why.
            DetailSubView::Driver => super::driver::render_driver_tab(
                frame,
                content_area,
                ctx,
                alias,
                ctx.view_cache.get(alias),
                &self.driver_viewport,
            ),
        }

        // Render footer with tab-appropriate hints
        let footer = build_footer(&sub_view, footer_area.width);
        frame.render_widget(footer, footer_area);
    }

    fn name(&self) -> &str {
        Self::NAME
    }
}

impl DetailScreen {
    /// Handle a keystroke while a text-input editor is open on the Defaults
    /// tab. Char/Backspace edit the buffer, Enter persists, Esc cancels.
    fn handle_text_input_key(
        &self,
        code: KeyCode,
        ctx: &mut AppContext,
        editing_idx: usize,
    ) -> ScreenAction {
        match code {
            KeyCode::Char(c) => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                cache.defaults_text_buffer.push(c);
                ctx.needs_redraw = true;
            }
            KeyCode::Backspace => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                cache.defaults_text_buffer.pop();
                ctx.needs_redraw = true;
            }
            KeyCode::Esc => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                cache.defaults_editing = None;
                cache.defaults_text_buffer.clear();
                ctx.needs_redraw = true;
            }
            KeyCode::Enter => {
                let project_path = ctx
                    .config
                    .projects
                    .get(&self.alias)
                    .map(|p| p.path.clone());
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                let target = cache.defaults_edit_target;
                let buffer = std::mem::take(&mut cache.defaults_text_buffer);
                cache.defaults_editing = None;
                let entries = entries_for_cache(cache);
                if let Some(entry) = entries.get(editing_idx).cloned() {
                    let key = entry.key;
                    if let Some(active) = active_config_mut(cache) {
                        let applied = if buffer.is_empty() {
                            clear_config_value(active, key)
                        } else {
                            set_string_value(active, key, &buffer)
                        };
                        if applied {
                            persist_active_config(
                                target,
                                project_path.as_deref(),
                                active,
                                &mut ctx.status_message,
                            );
                        }
                    }
                }
                ctx.needs_redraw = true;
            }
            _ => {}
        }
        ScreenAction::None
    }

    /// Render the phase list tab content.
    fn render_phase_list(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;
        let state = ctx.project_states.get(alias);
        let project_path = ctx
            .config
            .projects
            .get(alias)
            .map(|p| p.path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let mut lines: Vec<Line> = Vec::new();

        lines.push(Line::from(vec![
            Span::styled("  Path: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(shown(&project_path)),
        ]));

        if let Some(state) = state {
            // `classify_status` reads the RAW status — it is a comparison, not
            // a render — while the cell beside it carries the escaped form.
            let cat = classify_status(&state.status);
            let color = status_color(&cat);
            lines.push(Line::from(vec![
                Span::styled("  Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(shown(&state.status), Style::default().fg(color)),
                Span::raw("    "),
                Span::styled("Milestone: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(shown(&state.milestone)),
            ]));

            if state.paused {
                let pause_line = if let Some(ref ctx_text) = state.pause_context {
                    Line::from(vec![
                        Span::styled(
                            "  Paused: ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(shown(ctx_text), Style::default().fg(Color::Cyan)),
                    ])
                } else {
                    Line::from(Span::styled(
                        "  Paused (HANDOFF file present)",
                        Style::default().fg(Color::Cyan),
                    ))
                };
                lines.push(pause_line);
            }

            lines.push(Line::from(""));

            if let Some(event) = ctx.change_tracker.latest_change(alias) {
                let elapsed = ChangeTracker::format_elapsed(event.timestamp);
                let banner = format!("  [ {} -- {} ]", shown(&event.description), elapsed);
                lines.push(Line::from(Span::styled(
                    banner,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                lines.push(Line::from(""));
            }

            lines.push(Line::from(Span::styled(
                "  Phases:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "  Legend: + done  * current  o future  [stage] = disk-inferred  (N plans) = plan count",
                Style::default().fg(Color::DarkGray),
            )));

            if state.phases.is_empty() {
                lines.push(Line::from("  No roadmap data available"));
            } else {
                let current_phase_num = (state.completed_phases + 1).to_string();

                for phase in &state.phases {
                    let (icon, is_current) = if phase.completed {
                        ("+", false)
                    } else if phase.number == current_phase_num {
                        ("*", true)
                    } else {
                        ("o", false)
                    };

                    let plan_display = if phase.total_plans == 0 {
                        "0/? plans".to_string()
                    } else {
                        format!("{}/{} plans", phase.completed_plans, phase.total_plans)
                    };

                    let show_badges = ctx.config.preferences.gsd_integration;
                    let badge_spans =
                        disk_suffix_spans(&phase.number, &state.phase_disk_statuses, show_badges);

                    // `phase.number` above is COMPARED raw against
                    // `current_phase_num` and looked up raw in
                    // `phase_disk_statuses`; here it is read by a human.
                    let line_text = format!(
                        "  {} P{}: {}  {}",
                        icon,
                        shown(&phase.number),
                        shown(&phase.name),
                        plan_display
                    );

                    if is_current {
                        let cat = classify_status(&state.status);
                        let color = status_color(&cat);
                        let mut spans = vec![Span::styled(
                            line_text,
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        )];
                        spans.extend(badge_spans);
                        lines.push(Line::from(spans));
                    } else if phase.completed {
                        let mut spans = vec![Span::styled(
                            line_text,
                            Style::default().fg(Color::DarkGray),
                        )];
                        spans.extend(badge_spans);
                        lines.push(Line::from(spans));
                    } else {
                        let mut spans = vec![Span::raw(line_text)];
                        spans.extend(badge_spans);
                        lines.push(Line::from(spans));
                    }
                }
            }

            lines.push(Line::from(""));

            if state.backlog_count > 0 {
                lines.push(Line::from(format!(
                    "  Backlog: {} items",
                    state.backlog_count
                )));
            }

            if !state.queued_actions.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  Queued:",
                    Style::default().add_modifier(Modifier::BOLD),
                )));
                for (i, action) in state.queued_actions.iter().enumerate() {
                    lines.push(Line::from(vec![
                        Span::raw(format!("    {}. ", i + 1)),
                        Span::styled(shown(&action.command), Style::default().fg(Color::Cyan)),
                    ]));
                }
            }
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from("  No state data available for this project."));
        }

        let block = Block::default().borders(Borders::ALL);
        // Clamp scroll so content can't scroll past the end
        let content_height = lines.len() as u16;
        let viewport_height = area.height.saturating_sub(2); // borders

        // Record for the `_ =>` scroll handlers, which cannot see this pass.
        self.generic_viewport.set(ViewportMetrics {
            total_lines: content_height,
            visible_height: viewport_height,
        });
        let max_scroll = content_height.saturating_sub(viewport_height);
        let clamped_offset = self.scroll_offset.min(max_scroll);

        let paragraph = Paragraph::new(lines)
            .block(block)
            .scroll((clamped_offset, 0));

        frame.render_widget(paragraph, area);
    }

    /// Render the roadmap visualization tab content.
    fn render_roadmap(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;
        let state = ctx.project_states.get(alias);
        let project_path = ctx
            .config
            .projects
            .get(alias)
            .map(|p| p.path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if let Some(state) = state {
            let mut header_lines: Vec<Line> = Vec::new();
            header_lines.push(Line::from(vec![
                Span::styled("  Path: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(shown(&project_path)),
            ]));

            let cat = classify_status(&state.status);
            let color = status_color(&cat);
            header_lines.push(Line::from(vec![
                Span::styled("  Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(shown(&state.status), Style::default().fg(color)),
                Span::raw("    "),
                Span::styled("Milestone: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(shown(&state.milestone)),
            ]));

            if state.paused {
                let pause_line = if let Some(ref ctx_text) = state.pause_context {
                    Line::from(vec![
                        Span::styled(
                            "  Paused: ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(shown(ctx_text), Style::default().fg(Color::Cyan)),
                    ])
                } else {
                    Line::from(Span::styled(
                        "  Paused (HANDOFF file present)",
                        Style::default().fg(Color::Cyan),
                    ))
                };
                header_lines.push(pause_line);
            }

            header_lines.push(Line::from(""));

            if let Some(event) = ctx.change_tracker.latest_change(alias) {
                let elapsed = ChangeTracker::format_elapsed(event.timestamp);
                let banner = format!("  [ {} -- {} ]", shown(&event.description), elapsed);
                header_lines.push(Line::from(Span::styled(
                    banner,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                header_lines.push(Line::from(""));
            }

            let header_height = header_lines.len() as u16 + 2;

            let content_chunks =
                Layout::vertical([Constraint::Length(header_height), Constraint::Min(0)])
                    .split(area);

            let header_area = content_chunks[0];
            let roadmap_area = content_chunks[1];

            let header_block =
                Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);

            let header_paragraph = Paragraph::new(header_lines).block(header_block);
            frame.render_widget(header_paragraph, header_area);

            let current_phase_num = state.completed_phases + 1;
            // Clamp roadmap scroll — estimate content height from phase count
            let phase_block_h: u16 = 5; // BOX_HEIGHT(3) + connector(1) + spacing(1)
            let total_content = if state.phases.is_empty() {
                0
            } else {
                3 + (state.phases.len() as u16 - 1) * phase_block_h
            };
            let viewport_h = roadmap_area.height;
            // Record for the `_ =>` scroll handlers, which cannot see this pass.
            self.generic_viewport.set(ViewportMetrics {
                total_lines: total_content,
                visible_height: viewport_h,
            });
            let max_scroll = total_content.saturating_sub(viewport_h);
            let clamped_offset = self.scroll_offset.min(max_scroll);

            let roadmap_widget = RoadmapWidget {
                phases: &state.phases,
                current_phase_num,
                scroll_offset: clamped_offset,
            };
            frame.render_widget(roadmap_widget, roadmap_area);
        } else {
            let block = Block::default().borders(Borders::ALL);
            let paragraph = Paragraph::new("  No state data available for roadmap.").block(block);
            frame.render_widget(paragraph, area);
        }
    }

    /// Render the backlog tab with list selection and optional split-pane content preview.
    fn render_backlog_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let cache = ctx.view_cache.get(&self.alias);

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        let cache = match cache {
            Some(c) => c,
            None => {
                let msg =
                    Paragraph::new("  Loading...").style(Style::default().fg(Color::DarkGray));
                frame.render_widget(msg, inner);
                return;
            }
        };

        if cache.loading_backlog {
            let msg = Paragraph::new("  Loading...").style(Style::default().fg(Color::DarkGray));
            frame.render_widget(msg, inner);
            return;
        }

        if cache.backlog_items.is_empty() {
            let msg = Paragraph::new("  No backlog items found.");
            frame.render_widget(msg, inner);
            return;
        }

        // Build list items
        let items: Vec<ListItem> = cache
            .backlog_items
            .iter()
            .map(|item| {
                // READ BY A HUMAN, through a `ListItem` — the widget family
                // 21-23 measured as PRESERVING the entire invisible class,
                // `U+202E` included. This is the site verification pass 9
                // named by reading; the compiler named it here.
                ListItem::new(Line::from(format!(
                    "{} - {}",
                    item.number.shown(),
                    item.description.shown()
                )))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(cache.backlog_selected));

        if cache.backlog_expanded {
            // Split-pane: 50/50 list on top, content on bottom
            let chunks = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(inner);

            frame.render_stateful_widget(list, chunks[0], &mut list_state);

            // Content pane for selected item
            let selected_item = cache.backlog_items.get(cache.backlog_selected);
            // READ BY A HUMAN, through `Block::title` — the family that
            // preserves the invisible class most completely of the four
            // measured. Pass 9 did not name this site; the compiler did.
            let title = selected_item
                .map(|item| format!(" Content: {} ", item.dir_name.shown()))
                .unwrap_or_else(|| " Content ".to_string());
            let content_block = Block::default().borders(Borders::ALL).title(title);

            // READ BY A HUMAN: the body of a `.md` file inside a `999.*`
            // backlog directory. It reaches a `Paragraph`, which drops the
            // zero-width half of the class but passes the tag block through
            // intact — so escaping here is load-bearing, not belt-and-braces.
            let escaped_content = selected_item.and_then(|item| item.content.as_ref()).map(
                |content| content.shown().to_string(),
            );
            let content_text = escaped_content
                .as_deref()
                .unwrap_or("  Empty — no .md files in this backlog directory");

            let style = if selected_item
                .and_then(|item| item.content.as_ref())
                .is_none()
            {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            let content_paragraph = Paragraph::new(content_text)
                .style(style)
                .block(content_block);
            frame.render_widget(content_paragraph, chunks[1]);
        } else {
            // Full-height list, no content pane
            frame.render_stateful_widget(list, inner, &mut list_state);
        }
    }

    /// Render the git history tab with scrollable log, mode indicator, and diff stat pane.
    fn render_git_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let cache = ctx.view_cache.get(&self.alias);

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        let cache = match cache {
            Some(c) => c,
            None => {
                let msg = Paragraph::new("  Press 4 to load git history")
                    .style(Style::default().fg(Color::DarkGray));
                frame.render_widget(msg, inner);
                return;
            }
        };

        if cache.loading_git {
            let msg = Paragraph::new("  Loading git history...")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(msg, inner);
            return;
        }

        if cache.git_entries.is_empty() {
            let msg = Paragraph::new("  No commits found (or not a git repository)");
            frame.render_widget(msg, inner);
            return;
        }

        // Mode indicator line
        let mode_line = if cache.git_planning_only {
            Line::from(Span::styled(
                "  [.planning/ only]  Press 'p' to show full repo",
                Style::default().fg(Color::Yellow),
            ))
        } else {
            Line::from(Span::styled(
                "  [Full repo]  Press 'p' to show .planning/ only",
                Style::default().fg(Color::DarkGray),
            ))
        };

        // Layout: mode indicator (1 line), then log (and optionally diff stat)
        let has_diff = cache.git_diff_stat.is_some() || cache.loading_diff;
        let content_chunks = if has_diff {
            Layout::vertical([
                Constraint::Length(1),
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ])
            .split(inner)
        } else {
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner)
        };

        let mode_area = content_chunks[0];
        let log_area = content_chunks[1];

        frame.render_widget(Paragraph::new(mode_line), mode_area);

        // Build log list items
        let items: Vec<ListItem> = cache
            .git_entries
            .iter()
            .map(|entry| {
                // SHOWN: every field here is a human-readable cell drawn from a
                // THIRD-PARTY repository's `git log`. This is the site whose raw
                // form was live Trojan Source (T-21-23-01): a `List`/`ListItem`
                // PRESERVES `U+202E` and `U+00AD` into a cell (measured per
                // widget family), so a hostile commit subject reordered what the
                // operator read. `Untrusted` has no `Into<Cow<str>>`, so the raw
                // spelling of these four lines is a compile error rather than a
                // site a reader has to notice.
                ListItem::new(Line::from(vec![
                    Span::styled(entry.hash.shown(), Style::default().fg(Color::Yellow)),
                    Span::raw(" -- "),
                    Span::raw(entry.date.shown()),
                    Span::raw(" -- "),
                    Span::raw(entry.message.shown()),
                    Span::raw("  "),
                    Span::styled(entry.author.shown(), Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(cache.git_selected));
        frame.render_stateful_widget(list, log_area, &mut list_state);

        // Render diff stat pane if present
        if has_diff {
            let diff_area = content_chunks[2];
            if cache.loading_diff {
                let diff_block = Block::default()
                    .borders(Borders::ALL)
                    .title(" Diff: loading... ");
                let loading = Paragraph::new("  Loading diff...")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(diff_block);
                frame.render_widget(loading, diff_area);
            } else if let Some(stat) = &cache.git_diff_stat {
                // SHOWN: the title is what a human reads, and `Block::title` is
                // the widget family that PRESERVES the invisible class most
                // completely (measured — even `U+202E` reaches a cell through
                // it). The same hash goes to `load_diff_stat` RAW, above, which
                // is the split this carrier exists to make the compiler ask
                // about separately.
                let selected_hash = cache
                    .git_entries
                    .get(cache.git_selected)
                    .map(|e| e.hash.shown().to_string())
                    .unwrap_or_else(|| "???".to_string());
                let diff_block = Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Diff: {} ", selected_hash));

                let mut diff_lines: Vec<Line> = stat
                    .file_stats
                    .iter()
                    .map(|s| Line::from(format!("  {}", s)))
                    .collect();

                // Summary line with colored insertions/deletions
                diff_lines.push(Line::from(vec![
                    Span::raw(format!("  {} files changed, ", stat.files_changed)),
                    Span::styled(
                        format!("+{}", stat.insertions),
                        Style::default().fg(Color::Green),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("-{}", stat.deletions),
                        Style::default().fg(Color::Red),
                    ),
                ]));

                let diff_paragraph = Paragraph::new(diff_lines).block(diff_block);
                frame.render_widget(diff_paragraph, diff_area);
            }
        }
    }

    fn render_pipeline_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;
        let state = ctx.project_states.get(alias);
        let cache = ctx.view_cache.get(alias);

        let state = match state {
            Some(s) => s,
            None => {
                let block = Block::default().borders(Borders::ALL).title(" Pipeline ");
                let msg = Paragraph::new("  No state data available.").block(block);
                frame.render_widget(msg, area);
                return;
            }
        };

        if state.phases.is_empty() {
            let block = Block::default().borders(Borders::ALL).title(" Pipeline ");
            let msg = Paragraph::new("  No phases found").block(block);
            frame.render_widget(msg, area);
            return;
        }

        // Split into left (phase list) and right (pipeline detail)
        let panes = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
        let left_area = panes[0];
        let right_area = panes[1];

        let selected = cache
            .map(|c| {
                c.pipeline_selected
                    .min(state.phases.len().saturating_sub(1))
            })
            .unwrap_or(0);

        // Left pane: phase list
        let items: Vec<ListItem> = state
            .phases
            .iter()
            .map(|phase| {
                ListItem::new(format!(
                    "P{}: {}",
                    shown(&phase.number),
                    shown(&phase.name)
                ))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::RIGHT).title(" Phases "))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(selected));
        frame.render_stateful_widget(list, left_area, &mut list_state);

        // Right pane: pipeline detail for selected phase
        let phase = &state.phases[selected];
        let inference = state.phase_disk_statuses.get(&phase.number);

        let right_block = Block::default().borders(Borders::NONE).title(" Pipeline ");
        let inner = right_block.inner(right_area);
        frame.render_widget(right_block, right_area);

        match inference {
            None => {
                let msg = Paragraph::new("  No disk data");
                frame.render_widget(msg, inner);
            }
            Some(inf) => {
                let stage_statuses = derive_all_stage_statuses(inf);
                let pipeline_line = build_pipeline_line(inf, &stage_statuses);
                let detail_lines = build_stage_detail_lines(inf, &stage_statuses);
                let substage_lines = build_substage_lines(inf);

                let mut lines: Vec<Line> = Vec::new();
                // `phase.number` is the RAW key into `phase_disk_statuses` and
                // into `find_phase_dir` above and below; only this row is read.
                lines.push(Line::from(format!(
                    "  Phase {}: {}",
                    shown(&phase.number),
                    shown(&phase.name)
                )));
                lines.push(Line::from(""));
                lines.push(pipeline_line);
                lines.push(Line::from(""));
                for dl in detail_lines {
                    lines.push(dl);
                }
                if !substage_lines.is_empty() {
                    lines.push(Line::from(""));
                    for sl in substage_lines {
                        lines.push(sl);
                    }
                }

                // External-job indicator: distinguish a legitimately blocked
                // phase (waiting on an async job) from a stuck one.
                if state.external_job_waiting {
                    lines.push(Line::from(""));
                    lines.push(Line::from(vec![
                        Span::styled(
                            "  \u{23F3} external job waiting",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            "  (blocked on an async job, not stuck)",
                            Style::default().fg(Color::DarkGray),
                        ),
                    ]));
                }

                // Waves manifest (GSD 1.8.0 parallelism), when present on disk
                // for the selected phase. Absent/unparsable → render nothing.
                if let Some(proj) = ctx.config.projects.get(alias) {
                    let planning_dir = proj.path.join(".planning");
                    if let Some(phase_dir) =
                        crate::state_reader::disk_status::find_phase_dir(
                            &planning_dir,
                            &phase.number,
                        )
                    {
                        let waves_path = phase_dir.join("waves.json");
                        if let Ok(raw) = std::fs::read_to_string(&waves_path) {
                            if let Some(manifest) = parse_waves_manifest(&raw) {
                                if !manifest.waves.is_empty() {
                                    lines.push(Line::from(""));
                                    for wl in build_waves_lines(&manifest) {
                                        lines.push(wl);
                                    }
                                }
                            }
                        }
                    }
                }

                let paragraph = Paragraph::new(lines);
                frame.render_widget(paragraph, inner);
            }
        }
    }

    /// Render the queue tab with selectable list of queued actions.
    fn render_queue_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;
        let state = ctx.project_states.get(alias);
        let cache = ctx.view_cache.get(alias);

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        let queued_actions = state.map(|s| &s.queued_actions).filter(|q| !q.is_empty());

        match queued_actions {
            None => {
                let msg = Paragraph::new("  Queue empty -- press 'a' to add").style(
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                );
                frame.render_widget(msg, inner);
            }
            Some(actions) => {
                let selected = cache.map(|c| c.queue_selected).unwrap_or(0);

                let items: Vec<ListItem> = actions
                    .iter()
                    .map(|action| {
                        ListItem::new(Line::from(format!("  > {}", shown(&action.command))))
                    })
                    .collect();

                let title = format!(" Queue ({} items) ", actions.len());
                let list_block = Block::default().borders(Borders::ALL).title(title);
                let list = List::new(items)
                    .block(list_block)
                    .highlight_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                            .add_modifier(Modifier::UNDERLINED),
                    )
                    .highlight_symbol("> ");

                let mut list_state = ListState::default();
                list_state.select(Some(selected));
                frame.render_stateful_widget(list, inner, &mut list_state);
            }
        }
    }

    /// Render the sessions tab listing active Claude sessions for this project.
    ///
    /// The session id is drawn through [`shorten_session_id`], which is a
    /// CHARACTER operation. See its doc for the byte-slice panic it replaced.
    fn render_sessions_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        // Filter sessions by project path
        let filtered_sessions: Vec<_> = ctx
            .config
            .projects
            .get(alias)
            .map(|proj| {
                ctx.active_sessions
                    .iter()
                    .filter(|s| s.working_dir == proj.path)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if filtered_sessions.is_empty() {
            let lines = vec![
                Line::from(""),
                Line::from("  No active Claude sessions"),
                Line::from(""),
                Line::from(Span::styled(
                    "  [n] Launch new session",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            let msg = Paragraph::new(lines);
            frame.render_widget(msg, inner);
            return;
        }

        let selected = ctx
            .view_cache
            .get(alias)
            .map(|c| {
                c.sessions_selected
                    .min(filtered_sessions.len().saturating_sub(1))
            })
            .unwrap_or(0);

        let items: Vec<ListItem> = filtered_sessions
            .iter()
            .map(|session| {
                // READ BY A HUMAN, so escaped — and shortened by CHARACTERS,
                // never by bytes (T-21-25-05).
                let sid_display = session
                    .session_id
                    .as_ref()
                    .map(shorten_session_id)
                    .unwrap_or_else(|| "new session".to_string());
                let time_display = session
                    .start_time
                    .map(|_| "active".to_string())
                    .unwrap_or_else(|| "active".to_string());
                ListItem::new(Line::from(format!(
                    "  PID {} | Session: {} | {}",
                    session.pid, sid_display, time_display
                )))
            })
            .collect();

        let title = format!(" Sessions ({}) ", filtered_sessions.len());
        let list_block = Block::default().borders(Borders::ALL).title(title);
        let list = List::new(items)
            .block(list_block)
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(selected));
        frame.render_stateful_widget(list, inner, &mut list_state);
    }

    /// Render the archive tab with 4-level drill-down navigation.
    fn render_archive_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        use crate::archive::ArchiveDepth;

        let cache = if let Some(c) = ctx.view_cache.get(&self.alias) {
            c
        } else {
            let loading = Paragraph::new("Loading...")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(loading, area);
            return;
        };

        // Split: breadcrumb (1 row) + content (remaining)
        let chunks =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);
        let breadcrumb_area = chunks[0];
        let content_area = chunks[1];

        // Render breadcrumb
        let breadcrumb = Self::archive_breadcrumb(&cache.archive_depth, cache, ctx);
        frame.render_widget(Paragraph::new(breadcrumb), breadcrumb_area);

        if cache.archive_loading {
            let loading = Paragraph::new("Loading...")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(loading, content_area);
            return;
        }

        match &cache.archive_depth {
            ArchiveDepth::MilestoneList => {
                if cache.archive_milestones.is_empty() {
                    let empty = Paragraph::new(
                        "No archived milestones found. Complete a milestone to see it here.",
                    )
                    .style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(empty, content_area);
                    return;
                }
                let items: Vec<ListItem> = cache
                    .archive_milestones
                    .iter()
                    .map(|v| {
                        // READ BY A HUMAN, through a `ListItem` (preserves the
                        // whole class). A milestone version string scanned out
                        // of `.planning/archive/` directory names.
                        ListItem::new(Line::from(Span::styled(
                            v.shown(),
                            Style::default().fg(Color::Yellow),
                        )))
                    })
                    .collect();
                let list = List::new(items)
                    .highlight_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");
                let mut list_state = ListState::default();
                list_state.select(Some(cache.archive_selected[0]));
                frame.render_stateful_widget(list, content_area, &mut list_state);
            }
            ArchiveDepth::PhaseList { milestone } => {
                if let Some(data) = ctx.archive_cache.get(milestone) {
                    let mut items: Vec<ListItem> = Vec::new();
                    // Top-level milestone files first
                    for f in &data.top_level_files {
                        // READ BY A HUMAN, through a `ListItem`: an archive
                        // file name read off disk.
                        items.push(ListItem::new(Line::from(Span::styled(
                            format!("  {}", f.name.shown()),
                            Style::default().fg(Color::DarkGray),
                        ))));
                    }
                    // Then phases
                    for phase in &data.phases {
                        // READ BY A HUMAN, through a `ListItem`: a phase
                        // directory name, Title-Cased into a sentence whose
                        // untrusted half is the directory slug.
                        items.push(ListItem::new(Line::from(Span::raw(
                            phase.display_name.shown(),
                        ))));
                    }
                    if items.is_empty() {
                        let empty =
                            Paragraph::new("No phase artifacts found for this milestone.")
                                .style(Style::default().fg(Color::DarkGray));
                        frame.render_widget(empty, content_area);
                        return;
                    }
                    let list = List::new(items)
                        .highlight_style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol("> ");
                    let mut list_state = ListState::default();
                    list_state.select(Some(cache.archive_selected[1]));
                    frame.render_stateful_widget(list, content_area, &mut list_state);
                } else {
                    let loading = Paragraph::new("Loading...")
                        .style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(loading, content_area);
                }
            }
            ArchiveDepth::FileList {
                milestone,
                phase_idx,
            } => {
                if let Some(data) = ctx.archive_cache.get(milestone) {
                    if let Some(phase) = data.phases.get(*phase_idx) {
                        if phase.files.is_empty() {
                            let empty = Paragraph::new("No files found for this phase.")
                                .style(Style::default().fg(Color::DarkGray));
                            frame.render_widget(empty, content_area);
                            return;
                        }
                        let items: Vec<ListItem> = phase
                            .files
                            .iter()
                            // READ BY A HUMAN, through a `ListItem`: a phase
                            // artifact file name read off disk.
                            .map(|f| ListItem::new(Line::from(Span::raw(f.name.shown()))))
                            .collect();
                        let list = List::new(items)
                            .highlight_style(
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            )
                            .highlight_symbol("> ");
                        let mut list_state = ListState::default();
                        list_state.select(Some(cache.archive_selected[2]));
                        frame.render_stateful_widget(list, content_area, &mut list_state);
                    }
                }
            }
            ArchiveDepth::FileView { .. } => {
                if let Some(content) = &cache.archive_file_content {
                    let styled_lines = crate::archive::render_markdown_lines(content);
                    let total_lines = styled_lines.len() as u16;
                    // Split content area into gutter + main content
                    let gutter_width = (total_lines as usize).max(1).to_string().len() as u16 + 1;
                    let file_chunks = Layout::horizontal([
                        Constraint::Length(gutter_width),
                        Constraint::Min(0),
                    ])
                    .split(content_area);
                    let gutter_area = file_chunks[0];
                    let text_area = file_chunks[1];

                    let visible_height = text_area.height;
                    self.archive_viewport.set(ViewportMetrics {
                        total_lines,
                        visible_height,
                    });
                    let max_scroll = total_lines.saturating_sub(visible_height);
                    let scroll = cache.archive_scroll_offset.min(max_scroll);

                    // Render line number gutter
                    let gutter_lines = crate::archive::line_number_lines(
                        total_lines as usize,
                        scroll,
                        visible_height,
                    );
                    let gutter = Paragraph::new(gutter_lines);
                    frame.render_widget(gutter, gutter_area);

                    // Render styled markdown content
                    let paragraph = Paragraph::new(styled_lines).scroll((scroll, 0));
                    frame.render_widget(paragraph, text_area);
                } else {
                    let loading = Paragraph::new("Loading...")
                        .style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(loading, content_area);
                }
            }
        }
    }

    /// Render the docs browser tab: a drill-down list of `.planning/` entries
    /// (dirs first, then `.md` files) with a rendered markdown preview when a
    /// file is open.
    fn render_browser_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        use crate::browser::BrowserDepth;

        let cache = match ctx.view_cache.get(&self.alias) {
            Some(c) => c,
            None => {
                let p = Paragraph::new("Loading...")
                    .style(Style::default().fg(Color::DarkGray));
                frame.render_widget(p, area);
                return;
            }
        };

        // Header: Docs > <relative path from .planning/ root>
        let root = cache.browser_root.as_deref();
        let current = cache.browser_current_dir.as_deref();
        let rel_path = match (root, current) {
            (Some(r), Some(c)) => c
                .strip_prefix(r)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| c.to_string_lossy().to_string()),
            _ => String::new(),
        };
        let mut header_spans = vec![
            Span::styled("Docs", Style::default().fg(Color::Cyan)),
            Span::raw(" > "),
            Span::styled(".planning", Style::default().fg(Color::Yellow)),
        ];
        if !rel_path.is_empty() {
            // READ BY A HUMAN: the breadcrumb is the browsed directory's path
            // relative to `.planning/`, so every segment of it is a directory
            // name read off disk. The compiler cannot name this site — the
            // value is a `PathBuf`, not a carrier — and it was found by
            // POPULATING `browser_current_dir` in the probe fixture (21-25 T2).
            // The red is quoted in `probe_ctx`'s doc.
            header_spans.push(Span::raw("/"));
            header_spans.push(Span::styled(
                shown(&rel_path),
                Style::default().fg(Color::Yellow),
            ));
        }
        if cache.browser_depth == BrowserDepth::View {
            if let Some(name) = &cache.browser_file_name {
                // READ BY A HUMAN: the breadcrumb of the browsed file.
                header_spans.push(Span::raw(" / "));
                header_spans.push(Span::styled(
                    name.shown(),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));
            }
        }

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);
        let header_area = chunks[0];
        let content_area = chunks[1];

        let header = Paragraph::new(Line::from(header_spans));
        frame.render_widget(header, header_area);

        match cache.browser_depth {
            BrowserDepth::List => {
                if cache.browser_entries.is_empty() {
                    let msg = Paragraph::new("(empty directory)")
                        .style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(msg, content_area);
                    return;
                }
                let items: Vec<ListItem> = cache
                    .browser_entries
                    .iter()
                    .map(|e| {
                        // READ BY A HUMAN, through a `ListItem`: a directory
                        // listing entry from the project's `.planning/`.
                        // T-21-25-06 — the site verification pass 9 could NOT
                        // see, because the Browse tab's cache was empty in the
                        // probe fixture and the tab rendered its
                        // `(empty directory)` branch. Named by the compiler.
                        if e.is_dir {
                            ListItem::new(Line::from(vec![
                                Span::styled(
                                    "[DIR] ",
                                    Style::default()
                                        .fg(Color::Blue)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::raw(e.name.shown()),
                            ]))
                        } else {
                            ListItem::new(Line::from(Span::raw(e.name.shown())))
                        }
                    })
                    .collect();
                let list = List::new(items)
                    .block(Block::default().borders(Borders::NONE))
                    .highlight_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");
                let mut list_state = ListState::default();
                list_state.select(Some(cache.browser_selected));
                frame.render_stateful_widget(list, content_area, &mut list_state);
            }
            BrowserDepth::View => {
                if let Some(content) = &cache.browser_file_content {
                    let styled_lines = crate::archive::render_markdown_lines(content);
                    let total_lines = styled_lines.len() as u16;
                    let gutter_width =
                        (total_lines as usize).max(1).to_string().len() as u16 + 1;
                    let file_chunks = Layout::horizontal([
                        Constraint::Length(gutter_width),
                        Constraint::Min(0),
                    ])
                    .split(content_area);
                    let gutter_area = file_chunks[0];
                    let text_area = file_chunks[1];

                    let visible_height = text_area.height;
                    self.browser_viewport.set(ViewportMetrics {
                        total_lines,
                        visible_height,
                    });
                    let max_scroll = total_lines.saturating_sub(visible_height);
                    let scroll = cache.browser_scroll_offset.min(max_scroll);

                    let gutter_lines = crate::archive::line_number_lines(
                        total_lines as usize,
                        scroll,
                        visible_height,
                    );
                    let gutter = Paragraph::new(gutter_lines);
                    frame.render_widget(gutter, gutter_area);

                    let paragraph = Paragraph::new(styled_lines).scroll((scroll, 0));
                    frame.render_widget(paragraph, text_area);
                } else {
                    let loading = Paragraph::new("Loading...")
                        .style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(loading, content_area);
                }
            }
        }
    }

    /// Build breadcrumb line for the archive tab showing navigation path.
    fn archive_breadcrumb(
        depth: &crate::archive::ArchiveDepth,
        cache: &super::ProjectViewCache,
        ctx: &AppContext,
    ) -> Line<'static> {
        use crate::archive::ArchiveDepth;
        let mut spans = vec![Span::styled(
            "Archive".to_string(),
            Style::default().fg(Color::Cyan),
        )];

        // `ArchiveDepth::*::milestone` is a bare `String` and the compiler does
        // NOT name these three sites — the value is `.planning/`-derived (it is
        // `archive_milestones`' raw form, taken as a navigation key and a map
        // key) but the carrier does not travel with it through `ArchiveDepth`.
        // Found by reading the sites the compiler DID name in this function,
        // and escaped at the render with `shown` for that reason. **Residual:
        // `ArchiveDepth`'s `milestone` field is not typed; direction is
        // under-protection, silent, and it is bounded only by the probe.**
        match depth {
            ArchiveDepth::MilestoneList => {}
            ArchiveDepth::PhaseList { milestone } => {
                spans.push(Span::raw(" > "));
                spans.push(Span::styled(
                    shown(milestone),
                    Style::default().fg(Color::Yellow),
                ));
            }
            ArchiveDepth::FileList {
                milestone,
                phase_idx,
            } => {
                spans.push(Span::raw(" > "));
                spans.push(Span::styled(
                    shown(milestone),
                    Style::default().fg(Color::Yellow),
                ));
                if let Some(data) = ctx.archive_cache.get(milestone) {
                    if let Some(phase) = data.phases.get(*phase_idx) {
                        spans.push(Span::raw(" > "));
                        spans.push(Span::raw(phase.display_name.shown()));
                    }
                }
            }
            ArchiveDepth::FileView {
                milestone,
                phase_idx,
                ..
            } => {
                spans.push(Span::raw(" > "));
                spans.push(Span::styled(
                    shown(milestone),
                    Style::default().fg(Color::Yellow),
                ));
                if let Some(idx) = phase_idx {
                    if let Some(data) = ctx.archive_cache.get(milestone) {
                        if let Some(phase) = data.phases.get(*idx) {
                            spans.push(Span::raw(" > "));
                            spans.push(Span::raw(phase.display_name.shown()));
                        }
                    }
                }
                if let Some(ref name) = cache.archive_file_name {
                    spans.push(Span::raw(" > "));
                    spans.push(Span::styled(
                        name.shown(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                }
            }
        }
        Line::from(spans)
    }

    /// Render just the main content area (without footer), used by EnqueueScreen overlay.
    pub fn render_main_only(&self, frame: &mut Frame, main_area: Rect, ctx: &AppContext) {
        let alias = &self.alias;

        let sub_view = ctx
            .detail_sub_view_per_project
            .get(alias)
            .cloned()
            .unwrap_or_default();
        let tab_idx = tab_index(&sub_view);

        // Split into tab bar and content
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(main_area);
        let tab_area = chunks[0];
        let content_area = chunks[1];

        // Render tab bar — the duplicate of the site in `render`, and the reason
        // `tab_titles` exists as one function rather than two constructions.
        let (titles, select) = tab_titles(tab_area.width, tab_idx, driver_live_for(ctx, alias));
        let tabs_widget = Tabs::new(titles)
            .select(select)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )
            .divider("|");
        let tab_block = Block::default()
            .borders(Borders::BOTTOM)
            .title(format!(" Project: {} ", shown(alias)));
        frame.render_widget(tabs_widget.block(tab_block), tab_area);

        // Render content based on active tab
        match sub_view {
            DetailSubView::PhaseList => self.render_phase_list(frame, content_area, ctx),
            DetailSubView::RoadmapViz => self.render_roadmap(frame, content_area, ctx),
            DetailSubView::Backlog => self.render_backlog_tab(frame, content_area, ctx),
            DetailSubView::GitHistory => self.render_git_tab(frame, content_area, ctx),
            DetailSubView::Pipeline => self.render_pipeline_tab(frame, content_area, ctx),
            DetailSubView::Queue => self.render_queue_tab(frame, content_area, ctx),
            DetailSubView::Sessions => self.render_sessions_tab(frame, content_area, ctx),
            DetailSubView::Archive => self.render_archive_tab(frame, content_area, ctx),
            DetailSubView::Defaults => self.render_defaults_tab(frame, content_area, ctx),
            DetailSubView::Browse => self.render_browser_tab(frame, content_area, ctx),
            // The duplicate of the dispatch above, used by `EnqueueScreen` and
            // `DriverInjectScreen` to paint the body behind their footers. The
            // same delegation: a Driver tab that renders on one path and not the
            // other is the classic half-landing D-15 names.
            DetailSubView::Driver => super::driver::render_driver_tab(
                frame,
                content_area,
                ctx,
                alias,
                ctx.view_cache.get(alias),
                &self.driver_viewport,
            ),
        }
    }

    fn render_defaults_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        use super::DefaultsEditTarget;
        let cache = ctx.view_cache.get(&self.alias);
        let selected = cache.map(|c| c.defaults_selected).unwrap_or(0);
        let edit_target = cache
            .map(|c| c.defaults_edit_target)
            .unwrap_or_default();

        let entries = match cache {
            Some(c) => entries_for_cache(c),
            None => Vec::new(),
        };

        if entries.is_empty() {
            let msg_text = match edit_target {
                DefaultsEditTarget::Project => {
                    "  No config loaded (project may not have .planning/config.json)"
                }
                DefaultsEditTarget::Global => {
                    "  No global defaults loaded (~/.gsd/defaults.json missing — saving will create it)"
                }
            };
            let msg =
                Paragraph::new(msg_text).style(Style::default().fg(Color::DarkGray));
            frame.render_widget(msg, area);
            return;
        }

        let items: Vec<ListItem> = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let cat_span = if entry.show_category {
                    Span::styled(
                        format!("{:<18}", entry.category),
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    Span::raw("                  ")
                };
                let key_span = Span::styled(
                    format!("{:<30}", entry.key),
                    Style::default().fg(Color::White),
                );
                let val_style = match entry.kind {
                    ConfigValueKind::Bool => {
                        if entry.value == "true" {
                            Style::default().fg(Color::Green)
                        } else if entry.value == "false" {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        }
                    }
                    ConfigValueKind::Null => Style::default().fg(Color::DarkGray),
                    ConfigValueKind::ReadOnly => Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                    _ => Style::default().fg(Color::Yellow),
                };
                // READ BY A HUMAN, through a `ListItem`: `entry.value` is the
                // value of a key parsed out of the project's
                // `.planning/config.json`, and its string-valued keys (`mode`,
                // `granularity`, `project_code`, `phase_naming`,
                // `response_language`) are free-form text this build did not
                // author. `category` and `key` beside it are `&'static str`
                // literals from `build_defaults_entries` and need nothing.
                //
                // Found by POPULATING the fixture, not by reading (21-25 T2):
                // `defaults_config` was `None` under probe, so this tab painted
                // "No config loaded" and this `Span` was exercised by no
                // committed control. The red is quoted in the fixture's doc.
                let val_span = Span::styled(shown(&entry.value), val_style);
                let mut spans = vec![
                    Span::raw("  "),
                    cat_span,
                    Span::raw(" "),
                    key_span,
                    val_span,
                ];
                if entry.from_defaults {
                    spans.push(Span::styled(
                        " *",
                        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    ));
                }
                let line = Line::from(spans);
                let item = ListItem::new(line);
                if i == selected {
                    item.style(Style::default().bg(Color::DarkGray).fg(Color::Cyan))
                } else {
                    item
                }
            })
            .collect();

        let title = match edit_target {
            DefaultsEditTarget::Project => " Config Settings ".to_string(),
            DefaultsEditTarget::Global => " Global Defaults (~/.gsd/defaults.json) ".to_string(),
        };
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::TOP)
                .title(title),
        );

        let mut list_state = ListState::default();
        list_state.select(Some(selected));
        frame.render_stateful_widget(list, area, &mut list_state);

        // Render dropdown OR text-input overlay if editing
        if let Some(cache) = cache {
            if let Some(editing_idx) = cache.defaults_editing {
                if let Some(entry) = entries.get(editing_idx) {
                    if matches!(entry.kind, ConfigValueKind::String) {
                        let title = format!(" {} (Enter to save, Esc to cancel) ", entry.key);
                        let buffer = &cache.defaults_text_buffer;
                        let inner_w = buffer.len().max(title.len()).max(30) as u16;
                        let popup_w = (inner_w + 4).min(area.width.saturating_sub(2));
                        let popup_h = 3u16;
                        let popup_x = area.x + (area.width.saturating_sub(popup_w)) / 2;
                        let popup_y = area.y + (area.height.saturating_sub(popup_h)) / 2;
                        let popup_area = Rect {
                            x: popup_x,
                            y: popup_y,
                            width: popup_w,
                            height: popup_h,
                        };
                        frame.render_widget(ratatui::widgets::Clear, popup_area);
                        let text = Paragraph::new(Line::from(vec![
                            Span::styled(buffer.clone(), Style::default().fg(Color::White)),
                            Span::styled("█", Style::default().fg(Color::Cyan)),
                        ]))
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::Cyan))
                                .title(title),
                        );
                        frame.render_widget(text, popup_area);
                        return;
                    }
                    let options = dropdown_options(&entry.kind);
                    if !options.is_empty() {
                        let dropdown_idx =
                            cache.defaults_dropdown_selected.min(options.len() - 1);
                        let title = format!(" {} ", entry.key);
                        let max_opt_width = options.iter().map(|s| s.len()).max().unwrap_or(0);
                        let inner_w = max_opt_width.max(title.len() + 2).max(20) as u16;
                        let popup_w = (inner_w + 4).min(area.width.saturating_sub(2));
                        let popup_h = (options.len() as u16 + 2).min(area.height.saturating_sub(2));
                        let popup_x =
                            area.x + (area.width.saturating_sub(popup_w)) / 2;
                        let popup_y =
                            area.y + (area.height.saturating_sub(popup_h)) / 2;
                        let popup_area = Rect {
                            x: popup_x,
                            y: popup_y,
                            width: popup_w,
                            height: popup_h,
                        };

                        // Clear underneath the popup so the list doesn't bleed through
                        frame.render_widget(ratatui::widgets::Clear, popup_area);

                        let opt_items: Vec<ListItem> = options
                            .iter()
                            .enumerate()
                            .map(|(i, opt)| {
                                let style = if i == dropdown_idx {
                                    Style::default()
                                        .bg(Color::Cyan)
                                        .fg(Color::Black)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(Color::White)
                                };
                                let marker = if opt == &entry.value { "● " } else { "  " };
                                ListItem::new(Line::from(vec![
                                    Span::raw(marker),
                                    Span::raw(opt.clone()),
                                ]))
                                .style(style)
                            })
                            .collect();
                        let popup = List::new(opt_items).block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::Cyan))
                                .title(title),
                        );
                        frame.render_widget(popup, popup_area);
                    }
                }
            }
        }
    }
}

// --- Pipeline stage status logic ---

/// `pub(super)` since plan 18-09 because it appears in
/// [`derive_all_stage_statuses`]'s and [`build_pipeline_line`]'s signatures and
/// the Driver tab calls both. A **visibility widen, not a move**: `ARCHITECTURE`
/// M5's proposal to lift this logic into `state_reader/` is declined for this
/// phase — the driver process does not render, Phase 20's `decide()` is the
/// first genuine second consumer, and moving 130 lines across modules now would
/// churn the largest file in the repository to buy nothing (D-17).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum StageStatus {
    Complete,
    Current,
    Skipped,
    NotStarted,
}

/// Stage names for the 5 GSD pipeline stages.
const STAGE_LABELS: [&str; 5] = ["D", "R", "P", "E", "V"];
const STAGE_NAMES: [&str; 5] = ["Discuss", "Research", "Plan", "Execute", "Verify"];

/// Determine the status of each of the 5 pipeline stages from DiskInference.
///
/// `pub(super)` for [`StageStatus`]'s reason: the Driver tab reuses this widget
/// rather than growing a second progress display, which D-17 makes mandatory and
/// FEATURES.md names as an anti-feature.
pub(super) fn derive_all_stage_statuses(inf: &DiskInference) -> [StageStatus; 5] {
    // Whether each stage's artifact is present
    let present = [
        inf.has_context,       // D: Discuss
        inf.has_research,      // R: Research
        inf.has_plans,         // P: Plan
        inf.summary_count > 0, // E: Execute (at least one)
        inf.has_verification,  // V: Verify
    ];

    // Execute is "complete" only if summary_count >= plan_count and plan_count > 0
    let execute_complete = inf.plan_count > 0 && inf.summary_count >= inf.plan_count;

    let mut statuses = [StageStatus::NotStarted; 5];

    for i in 0..5 {
        if present[i] {
            if i == 3 && !execute_complete {
                // Execute stage: present but not fully complete
                statuses[i] = StageStatus::Current;
            } else {
                statuses[i] = StageStatus::Complete;
            }
        } else {
            // Check if any later stage is present (skip detection per D-10)
            let any_later = present[i + 1..].iter().any(|&p| p);
            if any_later {
                statuses[i] = StageStatus::Skipped;
            } else {
                // Check if this is the next expected stage after the last complete one
                let last_complete = (0..i).rev().find(|&j| present[j]);
                if let Some(lc) = last_complete {
                    if lc == i - 1 {
                        // Immediately after a complete stage
                        statuses[i] = StageStatus::Current;
                    }
                } else if i == 0 {
                    // First stage, nothing complete yet -- it's the current one
                    // only if the overall status isn't NoDirectory/Empty
                    if inf.status != DiskStatus::NoDirectory && inf.status != DiskStatus::Empty {
                        statuses[i] = StageStatus::Current;
                    }
                }
            }
        }
    }

    statuses
}

fn stage_color(status: StageStatus) -> Color {
    match status {
        StageStatus::Complete => Color::Green,
        StageStatus::Current => Color::Yellow,
        StageStatus::Skipped => Color::Magenta,
        StageStatus::NotStarted => Color::DarkGray,
    }
}

/// Build the horizontal pipeline line: [D]---[R]---[P]---[E 2/3]---[V]
///
/// `pub(super)` for [`StageStatus`]'s reason. The Driver tab calls this
/// **unmodified**: same two-cell indent, same [`stage_color`] per stage, same
/// `[--]` for a skipped one. Nothing about the widget is wrapped, restyled or
/// duplicated there, and `the_driver_tab_pipeline_line_matches_the_pipeline_tabs`
/// asserts the two produce the identical line for the same inference.
pub(super) fn build_pipeline_line(
    inf: &DiskInference,
    statuses: &[StageStatus; 5],
) -> Line<'static> {
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::raw("  "));

    for (i, &status) in statuses.iter().enumerate() {
        let color = stage_color(status);
        let label = if status == StageStatus::Skipped {
            "[--]".to_string()
        } else if i == 3 && inf.plan_count > 0 {
            // Execute stage with plan fraction
            format!("[E {}/{}]", inf.summary_count, inf.plan_count)
        } else {
            format!("[{}]", STAGE_LABELS[i])
        };

        let style = if status == StageStatus::Skipped {
            Style::default().fg(color).add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(color)
        };

        spans.push(Span::styled(label, style));

        if i < 4 {
            spans.push(Span::styled("---", Style::default().fg(Color::DarkGray)));
        }
    }

    Line::from(spans)
}

/// Build detail lines showing each stage's status text.
fn build_stage_detail_lines(
    inf: &DiskInference,
    statuses: &[StageStatus; 5],
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for (i, &status) in statuses.iter().enumerate() {
        let color = stage_color(status);
        let detail = match (i, status) {
            (3, StageStatus::Complete) => {
                format!("{}/{} complete", inf.summary_count, inf.plan_count)
            }
            (3, StageStatus::Current) => {
                format!("{}/{} complete", inf.summary_count, inf.plan_count)
            }
            (2, StageStatus::Complete) if inf.plan_count > 0 => format!("{} plans", inf.plan_count),
            (_, StageStatus::Complete) => "Complete".to_string(),
            (_, StageStatus::Current) => "Current".to_string(),
            (_, StageStatus::Skipped) => "Skipped".to_string(),
            (_, StageStatus::NotStarted) => "Not started".to_string(),
        };

        lines.push(Line::from(vec![
            Span::raw(format!("  {:<12}", format!("{}:", STAGE_NAMES[i]))),
            Span::styled(detail, Style::default().fg(color)),
        ]));
    }

    lines
}

/// A parsed `waves.json` parallelism manifest (GSD 1.8.0 claude-orchestration).
///
/// On-disk shape (see gsd-core `enable-claude-orchestration-workflow-backend`):
/// `{ "waves": [ { "id": "w1", "plans": [ { "id": "p1", "files_modified": [..] } ] } ] }`.
/// Deserialization is intentionally lenient: unknown fields are ignored and any
/// missing field defaults, so a partial or evolving manifest still renders.
#[derive(Debug, serde::Deserialize)]
struct WavesManifest {
    #[serde(default)]
    waves: Vec<WaveEntry>,
}

#[derive(Debug, serde::Deserialize)]
struct WaveEntry {
    /// Wave identifier — usually a string id (`"w1"`) but tolerated as a number too.
    #[serde(default)]
    id: Option<serde_json::Value>,
    /// Alternate wave key some manifests use instead of `id`.
    #[serde(default)]
    wave: Option<serde_json::Value>,
    #[serde(default)]
    plans: Vec<WavePlan>,
}

#[derive(Debug, serde::Deserialize)]
struct WavePlan {
    #[serde(default)]
    id: Option<String>,
    /// Alternate plan-identifier key.
    #[serde(default)]
    plan: Option<String>,
    #[serde(default)]
    files_modified: Vec<String>,
}

impl WaveEntry {
    /// A short display label for the wave, falling back to a 1-based index.
    fn label(&self, index: usize) -> String {
        match self.id.as_ref().or(self.wave.as_ref()) {
            Some(serde_json::Value::String(s)) if !s.is_empty() => s.clone(),
            Some(serde_json::Value::Null) | None => format!("wave {}", index + 1),
            Some(other) => other.to_string(),
        }
    }
}

impl WavePlan {
    /// The plan's identifier, if the manifest carried one.
    fn label(&self) -> Option<String> {
        self.id
            .clone()
            .or_else(|| self.plan.clone())
            .filter(|s| !s.is_empty())
    }
}

/// Deserialize a `waves.json` manifest. Returns `None` on unparsable input so
/// callers can silently omit the section rather than surface parse noise.
fn parse_waves_manifest(raw: &str) -> Option<WavesManifest> {
    serde_json::from_str::<WavesManifest>(raw).ok()
}

/// Render a compact "Waves" section: one line per wave listing its plans and a
/// small parallelism hint (plan count). Assumes `manifest.waves` is non-empty.
fn build_waves_lines(manifest: &WavesManifest) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "  Waves (parallelism):",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )));
    for (i, wave) in manifest.waves.iter().enumerate() {
        let plan_count = wave.plans.len();
        let files_touched: usize = wave.plans.iter().map(|p| p.files_modified.len()).sum();
        let mut hint = if plan_count == 1 {
            "1 plan".to_string()
        } else {
            format!("{} parallel", plan_count)
        };
        if files_touched > 0 {
            hint.push_str(&format!(", {}f", files_touched));
        }
        let plans: Vec<String> = wave.plans.iter().filter_map(|p| p.label()).collect();
        let plans_str = if plans.is_empty() {
            "(no plans)".to_string()
        } else {
            plans.join(", ")
        };
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled(
                format!("{:<8}", wave.label(i)),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!("{:<14}", hint),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(plans_str, Style::default().fg(Color::White)),
        ]));
    }
    lines
}

/// Build sub-stage status lines for the Plan and Execute parent stages.
/// Each sub-stage is detected by the presence of a specific artifact.
/// We only render a parent group's lines once that parent has any artifact
/// on disk — otherwise the section stays collapsed to avoid noise on
/// not-yet-touched phases.
fn build_substage_lines(inf: &DiskInference) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    let plan_touched = inf.has_plans
        || inf.has_spec
        || inf.has_patterns
        || inf.has_plan_check
        || inf.has_validation
        || inf.has_ui_spec
        || inf.has_ui_check
        || inf.has_ai_spec
        || inf.has_security
        || inf.has_skeleton
        || inf.has_windows
        || inf.has_deferred_items;
    if plan_touched {
        lines.push(Line::from(Span::styled(
            "  Plan sub-stages:",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
        )));
        push_substage(&mut lines, "Spec", inf.has_spec);
        push_substage(&mut lines, "Skeleton", inf.has_skeleton);
        push_substage(&mut lines, "Security", inf.has_security);
        push_substage(&mut lines, "Patterns", inf.has_patterns);
        push_substage(&mut lines, "UI-Spec", inf.has_ui_spec);
        push_substage(&mut lines, "AI-Spec", inf.has_ai_spec);
        push_substage(&mut lines, "Plan-Check", inf.has_plan_check);
        push_substage(&mut lines, "UI-Check", inf.has_ui_check);
        push_substage(&mut lines, "Nyquist", inf.has_validation);
        push_substage(&mut lines, "Windows", inf.has_windows);
        push_substage(&mut lines, "Deferred", inf.has_deferred_items);
    }

    let exec_touched = inf.summary_count > 0
        || inf.has_review
        || inf.has_ui_review
        || inf.has_eval_review
        || inf.has_uat
        || inf.has_coverage;
    if exec_touched {
        if plan_touched {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            "  Execute sub-stages:",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
        )));
        push_substage(&mut lines, "Code Review", inf.has_review);
        push_substage(&mut lines, "UI Review", inf.has_ui_review);
        push_substage(&mut lines, "Eval Review", inf.has_eval_review);
        push_substage(&mut lines, "UAT", inf.has_uat);
        push_substage(&mut lines, "Coverage", inf.has_coverage);
    }

    lines
}

fn push_substage(lines: &mut Vec<Line<'static>>, label: &'static str, present: bool) {
    let (marker, color) = if present {
        ("✓", Color::Green)
    } else {
        ("○", Color::DarkGray)
    };
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled(marker.to_string(), Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(format!("{:<14}", label), Style::default().fg(Color::White)),
        Span::styled(
            if present { "done" } else { "not run" },
            Style::default().fg(color),
        ),
    ]));
}

/// Resolve the markdown file the Docs (Browse) tab's `[e]dit` key should open.
///
/// Pure path logic — no filesystem I/O — so the whole UIFIX-03 contract is unit
/// testable without constructing an `AppContext`. Returns the user-facing status
/// message on the error path; the caller turns it into a `SetStatusMessage`.
fn browse_edit_target(cache: &super::ProjectViewCache) -> Result<std::path::PathBuf, &'static str> {
    const NO_FILE: &str = "Select a markdown file to edit";
    const READ_ONLY: &str = "Archived files are read-only";

    // 1. Resolve a candidate path from the current browse depth.
    let candidate = match cache.browser_depth {
        crate::browser::BrowserDepth::View => {
            // `None` content is the Phase 12 `Loading...` window; `e` is inert
            // there rather than editing a file the user cannot yet see.
            if cache.browser_file_content.is_none() {
                return Err(NO_FILE);
            }
            let dir = cache.browser_current_dir.as_ref().ok_or(NO_FILE)?;
            let name = cache.browser_file_name.as_ref().ok_or(NO_FILE)?;
            // A PATH SEGMENT handed to `$EDITOR`, so the raw bytes are what the
            // filesystem needs. The root fence below is what bounds it, and it
            // compares the same raw form.
            dir.join(name.as_raw_for_logic_only())
        }
        crate::browser::BrowserDepth::List => {
            let entry = cache
                .browser_entries
                .get(cache.browser_selected)
                .ok_or(NO_FILE)?;
            if entry.is_dir {
                return Err(NO_FILE);
            }
            entry.path.clone()
        }
    };

    // 2. Root fence: never hand $EDITOR a path outside the browse root.
    if let Some(root) = cache.browser_root.as_ref() {
        if !candidate.starts_with(root) {
            return Err(NO_FILE);
        }
    }

    // 3. Archived milestones stay immutable regardless of which tab reached
    //    them — the same guard the Archive tab applies.
    if candidate.to_string_lossy().contains("/milestones/") {
        return Err(READ_ONLY);
    }

    Ok(candidate)
}

/// Width at or above which the Driver footer shows every hint.
///
/// **101, not the UI-SPEC's 100.** The spec's own table gives the full form's
/// measured width as 101 cells and its condition as `width >= 100`, which are
/// inconsistent by one: at exactly 100 columns the trailing `p` of `[?]help`
/// falls off the right edge. That is the `STATUS_COLUMN_MIN_CELLS` bug in
/// miniature — a threshold that does not match the render — so the threshold is
/// the measured width, and `each_driver_footer_form_fits_the_width_that_selects_it`
/// keeps the two equal.
const DRIVER_FOOTER_FULL_CELLS: u16 = 101;

/// Width at or above which the Driver footer shows the medium form (69 cells).
const DRIVER_FOOTER_MEDIUM_CELLS: u16 = 70;

/// The Driver tab's footer hints, in three measured width forms.
///
/// The hints are ordered by how often they are needed, so truncation drops the
/// least useful first: the tab-switching and paging hints go before the four
/// verbs that only this tab offers. Measured against the shipped
/// `Span::styled("[k]", BOLD)` + `Span::raw("ey-word  ")` convention: 101 cells
/// full, 69 medium, 48 short.
fn driver_footer_spans(width: u16) -> Vec<Span<'static>> {
    let b = Style::default().add_modifier(Modifier::BOLD);
    let mut spans: Vec<Span<'static>> = Vec::new();

    if width >= DRIVER_FOOTER_MEDIUM_CELLS {
        spans.push(Span::raw("  "));
        spans.push(Span::styled("[Esc]", b));
        spans.push(Span::raw("back  "));
    } else {
        spans.push(Span::raw("  "));
    }
    if width >= DRIVER_FOOTER_FULL_CELLS {
        spans.push(Span::styled("[1-0/D]", b));
        spans.push(Span::raw("tabs  "));
    }
    spans.push(Span::styled("[j/k]", b));
    spans.push(Span::raw("runs  "));
    if width >= DRIVER_FOOTER_FULL_CELLS {
        spans.push(Span::styled("[PgUp/PgDn]", b));
        spans.push(Span::raw("output  "));
    }
    if width >= DRIVER_FOOTER_MEDIUM_CELLS {
        spans.push(Span::styled("[f]", b));
        spans.push(Span::raw("ollow  "));
    }
    spans.push(Span::styled("[i]", b));
    spans.push(Span::raw("nject  "));
    spans.push(Span::styled("[s]", b));
    spans.push(Span::raw("tart  "));
    spans.push(Span::styled("[x]", b));
    spans.push(Span::raw("stop  "));
    spans.push(Span::styled("[?]", b));
    spans.push(Span::raw("help"));

    spans
}

/// Build the footer key-hint spans for a tab.
///
/// Split out of `build_footer` so the hint set is assertable: `Paragraph`
/// exposes no public text accessor, but a `Vec<Span>` concatenates cleanly.
///
/// `width` is the footer row's width. Only the Driver tab tiers on it today; the
/// other ten keep their single shipped form, with the shared prefix's tabs hint
/// changed once, for every tab, so `Shift+D` is discoverable from anywhere in
/// the detail view.
fn footer_spans(sub_view: &DetailSubView, width: u16) -> Vec<Span<'static>> {
    if matches!(sub_view, DetailSubView::Driver) {
        return driver_footer_spans(width);
    }

    let b = Style::default().add_modifier(Modifier::BOLD);
    let mut spans = vec![
        Span::raw("  "),
        Span::styled("[Esc]", b),
        Span::raw("back  "),
        Span::styled("[1-0/D]", b),
        Span::raw("tabs  "),
        Span::styled("[j/k]", b),
        Span::raw("scroll  "),
    ];

    match sub_view {
        DetailSubView::Backlog => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("xpand  "));
            spans.push(Span::styled("[e]", b));
            spans.push(Span::raw("nqueue  "));
        }
        DetailSubView::GitHistory => {
            spans.push(Span::styled("[p]", b));
            spans.push(Span::raw("lanning-only  "));
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("diff  "));
        }
        DetailSubView::Queue => {
            spans.push(Span::styled("[a]", b));
            spans.push(Span::raw("dd  "));
            spans.push(Span::styled("[e]", b));
            spans.push(Span::raw("dit  "));
            spans.push(Span::styled("[d]", b));
            spans.push(Span::raw("el  "));
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("done  "));
            spans.push(Span::styled("[J/K]", b));
            spans.push(Span::raw("reorder  "));
        }
        DetailSubView::Sessions => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("resume  "));
            spans.push(Span::styled("[Tab]", b));
            spans.push(Span::raw("switch  "));
            spans.push(Span::styled("[n]", b));
            spans.push(Span::raw("ew session  "));
        }
        DetailSubView::Archive => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("open  "));
            spans.push(Span::styled("[e]", b));
            spans.push(Span::raw("dit  "));
        }
        DetailSubView::Browse => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("open  "));
            spans.push(Span::styled("[e]", b));
            spans.push(Span::raw("dit  "));
            spans.push(Span::styled("[Esc]", b));
            spans.push(Span::raw("up  "));
            spans.push(Span::styled("[g]", b));
            spans.push(Span::raw("root  "));
            spans.push(Span::styled("[p]", b));
            spans.push(Span::raw("hase  "));
        }
        DetailSubView::Defaults => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("edit  "));
            spans.push(Span::styled("[x]", b));
            spans.push(Span::raw(" clear  "));
            spans.push(Span::styled("[d]", b));
            spans.push(Span::raw(" defaults  "));
            spans.push(Span::styled("[r]", b));
            spans.push(Span::raw("eload  "));
        }
        _ => {
            spans.push(Span::styled("[e]", b));
            spans.push(Span::raw("nqueue  "));
        }
    }

    spans.push(Span::styled("[?]", b));
    spans.push(Span::raw("help"));

    spans
}

/// Build the footer line with tab-appropriate key hints.
fn build_footer(sub_view: &DetailSubView, width: u16) -> Paragraph<'static> {
    Paragraph::new(Line::from(footer_spans(sub_view, width)))
}

// --- Defaults tab helpers ---

#[derive(Debug, Clone)]
enum ConfigValueKind {
    Bool,
    Enum(&'static [&'static str]),
    String,
    Integer,
    Null,
    /// Display-only: shape-varying keys (JSON value could be int/string/array)
    /// that we surface as a formatted string but never make editable.
    ReadOnly,
}

#[derive(Clone)]
struct ConfigEntry {
    category: &'static str,
    key: &'static str,
    value: String,
    kind: ConfigValueKind,
    show_category: bool,
    /// True when the value was inherited from ~/.gsd/defaults.json
    /// because the project config had it unset.
    from_defaults: bool,
}

fn opt_bool_layered(project: Option<bool>, defaults: Option<bool>) -> (String, ConfigValueKind, bool) {
    if let Some(v) = project {
        (v.to_string(), ConfigValueKind::Bool, false)
    } else if let Some(v) = defaults {
        (v.to_string(), ConfigValueKind::Bool, true)
    } else {
        ("(unset)".to_string(), ConfigValueKind::Null, false)
    }
}

fn opt_str_layered(project: Option<&str>, defaults: Option<&str>) -> (String, ConfigValueKind, bool) {
    if let Some(s) = project {
        (s.to_string(), ConfigValueKind::String, false)
    } else if let Some(s) = defaults {
        (s.to_string(), ConfigValueKind::String, true)
    } else {
        ("(unset)".to_string(), ConfigValueKind::Null, false)
    }
}

fn opt_u32_layered(project: Option<u32>, defaults: Option<u32>) -> (String, ConfigValueKind, bool) {
    if let Some(n) = project {
        (n.to_string(), ConfigValueKind::Integer, false)
    } else if let Some(n) = defaults {
        (n.to_string(), ConfigValueKind::Integer, true)
    } else {
        ("(unset)".to_string(), ConfigValueKind::Null, false)
    }
}

fn opt_enum_layered(
    project: Option<&str>,
    defaults: Option<&str>,
    options: &'static [&'static str],
) -> (String, ConfigValueKind, bool) {
    if let Some(s) = project {
        (s.to_string(), ConfigValueKind::Enum(options), false)
    } else if let Some(s) = defaults {
        (s.to_string(), ConfigValueKind::Enum(options), true)
    } else {
        ("(unset)".to_string(), ConfigValueKind::Null, false)
    }
}

/// Format a shape-varying JSON value for read-only display.
fn json_display(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Layered accessor for shape-varying keys rendered read-only.
fn opt_json_readonly(
    project: Option<&serde_json::Value>,
    defaults: Option<&serde_json::Value>,
) -> (String, ConfigValueKind, bool) {
    if let Some(v) = project {
        (json_display(v), ConfigValueKind::ReadOnly, false)
    } else if let Some(v) = defaults {
        (json_display(v), ConfigValueKind::ReadOnly, true)
    } else {
        ("(unset)".to_string(), ConfigValueKind::Null, false)
    }
}

fn build_defaults_entries(
    config: &crate::state_reader::config_json::GsdConfig,
    defaults: Option<&crate::state_reader::config_json::GsdConfig>,
) -> Vec<ConfigEntry> {
    use crate::state_reader::config_json::GsdConfig;

    let mut entries = Vec::new();
    let mut push = |cat: &'static str,
                    key: &'static str,
                    value: String,
                    kind: ConfigValueKind,
                    first: bool,
                    from_defaults: bool| {
        entries.push(ConfigEntry {
            category: cat,
            key,
            value,
            kind,
            show_category: first,
            from_defaults,
        });
    };

    // Layered accessors — fall back to ~/.gsd/defaults.json when the
    // project's Option is None.
    let bool_l = |proj: Option<bool>, def: Option<bool>| opt_bool_layered(proj, def);
    let u32_l = |proj: Option<u32>, def: Option<u32>| opt_u32_layered(proj, def);
    let str_l = |proj: Option<&str>, def: Option<&str>| opt_str_layered(proj, def);
    let enum_l = |proj: Option<&str>,
                  def: Option<&str>,
                  options: &'static [&'static str]| { opt_enum_layered(proj, def, options) };

    // ── Planning ───────────────────────────────────────────────
    let cat = "Planning";
    let pwf = config.workflow.as_ref();
    let dwf = defaults.and_then(|d: &GsdConfig| d.workflow.as_ref());
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.research), dwf.and_then(|w| w.research));
    push(cat, "research", v, k, true, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.plan_check), dwf.and_then(|w| w.plan_check));
    push(cat, "plan_check", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.pattern_mapper), dwf.and_then(|w| w.pattern_mapper));
    push(cat, "pattern_mapper", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.nyquist_validation), dwf.and_then(|w| w.nyquist_validation));
    push(cat, "nyquist_validation", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ui_phase), dwf.and_then(|w| w.ui_phase));
    push(cat, "ui_phase", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ui_safety_gate), dwf.and_then(|w| w.ui_safety_gate));
    push(cat, "ui_safety_gate", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ai_integration_phase), dwf.and_then(|w| w.ai_integration_phase));
    push(cat, "ai_integration_phase", v, k, false, fd);
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.subagent_timeout), dwf.and_then(|w| w.subagent_timeout));
    push(cat, "subagent_timeout", v, k, false, fd);
    // GSD 1.4–1.8 planning gates
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.specless_probe_fallback), dwf.and_then(|w| w.specless_probe_fallback));
    push(cat, "workflow.specless_probe_fallback", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.assumption_delta), dwf.and_then(|w| w.assumption_delta));
    push(cat, "workflow.assumption_delta", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.plan_drift_precheck), dwf.and_then(|w| w.plan_drift_precheck));
    push(cat, "workflow.plan_drift_precheck", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.plan_chunked), dwf.and_then(|w| w.plan_chunked));
    push(cat, "workflow.plan_chunked", v, k, false, fd);
    let (v, k, fd) = str_l(pwf.and_then(|w| w.context_guard_mode.as_deref()), dwf.and_then(|w| w.context_guard_mode.as_deref()));
    push(cat, "workflow.context_guard_mode", v, k, false, fd);

    // ── Execution ──────────────────────────────────────────────
    let cat = "Execution";
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.verifier), dwf.and_then(|w| w.verifier));
    push(cat, "verifier", v, k, true, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.tdd_mode), dwf.and_then(|w| w.tdd_mode));
    push(cat, "tdd_mode", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.code_review), dwf.and_then(|w| w.code_review));
    push(cat, "code_review", v, k, false, fd);
    let (v, k, fd) = enum_l(
        pwf.and_then(|w| w.code_review_depth.as_deref()),
        dwf.and_then(|w| w.code_review_depth.as_deref()),
        &["quick", "standard", "deep"],
    );
    push(cat, "code_review_depth", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ui_review), dwf.and_then(|w| w.ui_review));
    push(cat, "ui_review", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.node_repair), dwf.and_then(|w| w.node_repair));
    push(cat, "node_repair", v, k, false, fd);
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.node_repair_budget), dwf.and_then(|w| w.node_repair_budget));
    push(cat, "node_repair_budget", v, k, false, fd);
    // GSD 1.8 execution gates
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.api_coverage_gate), dwf.and_then(|w| w.api_coverage_gate));
    push(cat, "workflow.api_coverage_gate", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.windows_enforce), dwf.and_then(|w| w.windows_enforce));
    push(cat, "workflow.windows_enforce", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.mvp_mode), dwf.and_then(|w| w.mvp_mode));
    push(cat, "workflow.mvp_mode", v, k, false, fd);
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.test_gate_timeout), dwf.and_then(|w| w.test_gate_timeout));
    push(cat, "workflow.test_gate_timeout", v, k, false, fd);
    let (v, k, fd) = str_l(pwf.and_then(|w| w.code_review_command.as_deref()), dwf.and_then(|w| w.code_review_command.as_deref()));
    push(cat, "workflow.code_review_command", v, k, false, fd);
    // Shape-varying security keys — read-only display.
    let (v, k, fd) = opt_json_readonly(
        pwf.and_then(|w| w.security_asvs_level.as_ref()),
        dwf.and_then(|w| w.security_asvs_level.as_ref()),
    );
    push(cat, "workflow.security_asvs_level", v, k, false, fd);
    let (v, k, fd) = opt_json_readonly(
        pwf.and_then(|w| w.security_block_on.as_ref()),
        dwf.and_then(|w| w.security_block_on.as_ref()),
    );
    push(cat, "workflow.security_block_on", v, k, false, fd);

    // ── Docs & Output ─────────────────────────────────────────
    let cat = "Docs & Output";
    let (v, k, fd) = bool_l(config.commit_docs, defaults.and_then(|d| d.commit_docs));
    push(cat, "commit_docs", v, k, true, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.skip_discuss), dwf.and_then(|w| w.skip_discuss));
    push(cat, "skip_discuss", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.use_worktrees), dwf.and_then(|w| w.use_worktrees));
    push(cat, "use_worktrees", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.text_mode), dwf.and_then(|w| w.text_mode));
    push(cat, "text_mode", v, k, false, fd);
    let (v, k, fd) = str_l(
        config.response_language.as_deref(),
        defaults.and_then(|d| d.response_language.as_deref()),
    );
    push(cat, "response_language", v, k, false, fd);

    // ── Features ──────────────────────────────────────────────
    let cat = "Features";
    let (v, k, fd) = bool_l(
        config.intel.as_ref().and_then(|i| i.enabled),
        defaults.and_then(|d| d.intel.as_ref().and_then(|i| i.enabled)),
    );
    push(cat, "intel_enabled", v, k, true, fd);
    let (v, k, fd) = bool_l(
        config.graphify.as_ref().and_then(|g| g.enabled),
        defaults.and_then(|d| d.graphify.as_ref().and_then(|g| g.enabled)),
    );
    push(cat, "graphify_enabled", v, k, false, fd);
    let (v, k, fd) = u32_l(
        config.graphify.as_ref().and_then(|g| g.build_timeout),
        defaults.and_then(|d| d.graphify.as_ref().and_then(|g| g.build_timeout)),
    );
    push(cat, "graphify_build_timeout", v, k, false, fd);
    let (v, k, fd) = str_l(
        config.graphify.as_ref().and_then(|g| g.graph_path.as_deref()),
        defaults.and_then(|d| d.graphify.as_ref().and_then(|g| g.graph_path.as_deref())),
    );
    push(cat, "graphify.graph_path", v, k, false, fd);
    let (v, k, fd) = bool_l(config.brave_search, defaults.and_then(|d| d.brave_search));
    push(cat, "brave_search", v, k, false, fd);
    let (v, k, fd) = bool_l(config.firecrawl, defaults.and_then(|d| d.firecrawl));
    push(cat, "firecrawl", v, k, false, fd);
    let (v, k, fd) = bool_l(config.exa_search, defaults.and_then(|d| d.exa_search));
    push(cat, "exa_search", v, k, false, fd);

    // ── Model & Pipeline ──────────────────────────────────────
    let cat = "Model & Pipeline";
    push(cat, "mode", config.mode.clone(), ConfigValueKind::Enum(&["interactive", "yolo"]), true, false);
    push(cat, "granularity", config.granularity.clone(), ConfigValueKind::Enum(&["coarse", "standard", "fine"]), false, false);
    push(cat, "model_profile", config.model_profile.clone(), ConfigValueKind::Enum(&["quality", "balanced", "budget", "adaptive", "inherit"]), false, false);
    let (v, k, fd) = bool_l(config.parallelization, defaults.and_then(|d| d.parallelization));
    push(cat, "parallelization", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.auto_advance), dwf.and_then(|w| w.auto_advance));
    push(cat, "auto_advance", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.auto_chain_active), dwf.and_then(|w| w.auto_chain_active));
    push(cat, "auto_chain_active", v, k, false, fd);
    let pgit = config.git.as_ref();
    let dgit = defaults.and_then(|d| d.git.as_ref());
    let (v, k, fd) = enum_l(
        pgit.and_then(|g| g.branching_strategy.as_deref()),
        dgit.and_then(|g| g.branching_strategy.as_deref()),
        &["none", "phase", "milestone"],
    );
    push(cat, "branching_strategy", v, k, false, fd);
    let (v, k, fd) = str_l(
        pgit.and_then(|g| g.base_branch.as_deref()),
        dgit.and_then(|g| g.base_branch.as_deref()),
    );
    push(cat, "base_branch", v, k, false, fd);
    let (v, k, fd) = str_l(
        pgit.and_then(|g| g.phase_branch_template.as_deref()),
        dgit.and_then(|g| g.phase_branch_template.as_deref()),
    );
    push(cat, "phase_branch_template", v, k, false, fd);
    let (v, k, fd) = str_l(
        pgit.and_then(|g| g.milestone_branch_template.as_deref()),
        dgit.and_then(|g| g.milestone_branch_template.as_deref()),
    );
    push(cat, "milestone_branch_template", v, k, false, fd);
    let qbt_proj = pgit.and_then(|g| g.quick_branch_template.as_ref()).map(|v| v.to_string());
    let qbt_def = dgit.and_then(|g| g.quick_branch_template.as_ref()).map(|v| v.to_string());
    let (qbt_val, qbt_kind, qbt_fd) = match (qbt_proj, qbt_def) {
        (Some(s), _) => (s, ConfigValueKind::String, false),
        (None, Some(s)) => (s, ConfigValueKind::String, true),
        (None, None) => ("(unset)".to_string(), ConfigValueKind::Null, false),
    };
    push(cat, "quick_branch_template", qbt_val, qbt_kind, false, qbt_fd);

    // ── Misc ──────────────────────────────────────────────────
    let cat = "Misc";
    let (v, k, fd) = bool_l(
        config.hooks.as_ref().and_then(|h| h.context_warnings),
        defaults.and_then(|d| d.hooks.as_ref().and_then(|h| h.context_warnings)),
    );
    push(cat, "context_warnings", v, k, true, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.research_before_questions), dwf.and_then(|w| w.research_before_questions));
    push(cat, "research_before_questions", v, k, false, fd);
    let (v, k, fd) = enum_l(
        pwf.and_then(|w| w.discuss_mode.as_deref()),
        dwf.and_then(|w| w.discuss_mode.as_deref()),
        &["discuss", "assumptions"],
    );
    push(cat, "discuss_mode", v, k, false, fd);
    let (v, k, fd) = bool_l(config.search_gitignored, defaults.and_then(|d| d.search_gitignored));
    push(cat, "search_gitignored", v, k, false, fd);
    let (v, k, fd) = str_l(config.project_code.as_deref(), defaults.and_then(|d| d.project_code.as_deref()));
    push(cat, "project_code", v, k, false, fd);
    let (v, k, fd) = str_l(config.phase_naming.as_deref(), defaults.and_then(|d| d.phase_naming.as_deref()));
    push(cat, "phase_naming", v, k, false, fd);
    let (v, k, fd) = str_l(config.phase_id_convention.as_deref(), defaults.and_then(|d| d.phase_id_convention.as_deref()));
    push(cat, "phase_id_convention", v, k, false, fd);
    let (v, k, fd) = str_l(config.claude_md_path.as_deref(), defaults.and_then(|d| d.claude_md_path.as_deref()));
    push(cat, "claude_md_path", v, k, false, fd);
    let (v, k, fd) = opt_json_readonly(config.sub_repos.as_ref(), defaults.and_then(|d| d.sub_repos.as_ref()));
    push(cat, "sub_repos", v, k, false, fd);

    // ── Orchestration ─────────────────────────────────────────
    let cat = "Orchestration";
    let pco = config.claude_orchestration.as_ref();
    let dco = defaults.and_then(|d: &GsdConfig| d.claude_orchestration.as_ref());
    let (v, k, fd) = bool_l(pco.and_then(|c| c.enabled), dco.and_then(|c| c.enabled));
    push(cat, "claude_orchestration.enabled", v, k, true, fd);
    let (v, k, fd) = str_l(pco.and_then(|c| c.execution_backend.as_deref()), dco.and_then(|c| c.execution_backend.as_deref()));
    push(cat, "claude_orchestration.execution_backend", v, k, false, fd);
    let (v, k, fd) = str_l(pco.and_then(|c| c.min_agent_sdk_version.as_deref()), dco.and_then(|c| c.min_agent_sdk_version.as_deref()));
    push(cat, "claude_orchestration.min_agent_sdk_version", v, k, false, fd);

    // ── Statusline ────────────────────────────────────────────
    let cat = "Statusline";
    let psl = config.statusline.as_ref();
    let dsl = defaults.and_then(|d: &GsdConfig| d.statusline.as_ref());
    let (v, k, fd) = bool_l(psl.and_then(|s| s.show_context_tokens), dsl.and_then(|s| s.show_context_tokens));
    push(cat, "statusline.show_context_tokens", v, k, true, fd);
    let (v, k, fd) = str_l(psl.and_then(|s| s.state_format.as_deref()), dsl.and_then(|s| s.state_format.as_deref()));
    push(cat, "statusline.state_format", v, k, false, fd);
    let (v, k, fd) = bool_l(psl.and_then(|s| s.show_git), dsl.and_then(|s| s.show_git));
    push(cat, "statusline.show_git", v, k, false, fd);

    // ── Routing ───────────────────────────────────────────────
    let cat = "Routing";
    let pdr = config.dynamic_routing.as_ref();
    let ddr = defaults.and_then(|d: &GsdConfig| d.dynamic_routing.as_ref());
    let (v, k, fd) = bool_l(pdr.and_then(|r| r.provider_escalation), ddr.and_then(|r| r.provider_escalation));
    push(cat, "dynamic_routing.provider_escalation", v, k, true, fd);
    let (v, k, fd) = u32_l(pdr.and_then(|r| r.max_escalations), ddr.and_then(|r| r.max_escalations));
    push(cat, "dynamic_routing.max_escalations", v, k, false, fd);

    // ── External Job ──────────────────────────────────────────
    let cat = "External Job";
    let pej = config.external_job.as_ref();
    let dej = defaults.and_then(|d: &GsdConfig| d.external_job.as_ref());
    let (v, k, fd) = u32_l(pej.and_then(|e| e.submit_timeout_ms), dej.and_then(|e| e.submit_timeout_ms));
    push(cat, "external_job.submit_timeout_ms", v, k, true, fd);
    let (v, k, fd) = u32_l(pej.and_then(|e| e.poll_timeout_ms), dej.and_then(|e| e.poll_timeout_ms));
    push(cat, "external_job.poll_timeout_ms", v, k, false, fd);
    let (v, k, fd) = str_l(pej.and_then(|e| e.artifact_dir.as_deref()), dej.and_then(|e| e.artifact_dir.as_deref()));
    push(cat, "external_job.artifact_dir", v, k, false, fd);

    // ── Capabilities ──────────────────────────────────────────
    let cat = "Capabilities";
    let pcap = config.capabilities.as_ref();
    let dcap = defaults.and_then(|d: &GsdConfig| d.capabilities.as_ref());
    let (v, k, fd) = bool_l(pcap.and_then(|c| c.strict_known_registries), dcap.and_then(|c| c.strict_known_registries));
    push(cat, "capabilities.strict_known_registries", v, k, true, fd);
    let (v, k, fd) = bool_l(pcap.and_then(|c| c.auto_update), dcap.and_then(|c| c.auto_update));
    push(cat, "capabilities.auto_update", v, k, false, fd);

    // ── Review ────────────────────────────────────────────────
    let cat = "Review";
    let (v, k, fd) = opt_json_readonly(
        config.review.as_ref().and_then(|r| r.reviewer_instances.as_ref()),
        defaults.and_then(|d| d.review.as_ref().and_then(|r| r.reviewer_instances.as_ref())),
    );
    push(cat, "review.reviewer_instances", v, k, true, fd);

    entries
}

/// Build defaults entries respecting the cache's edit target.
/// When target=Project: project config is primary, defaults_user_config
/// fills in unset Optional fields. When target=Global: defaults_user_config
/// is primary with no fallback.
fn entries_for_cache(cache: &super::ProjectViewCache) -> Vec<ConfigEntry> {
    use super::DefaultsEditTarget;
    let (primary, fallback) = match cache.defaults_edit_target {
        DefaultsEditTarget::Project => {
            (cache.defaults_config.as_ref(), cache.defaults_user_config.as_ref())
        }
        DefaultsEditTarget::Global => (cache.defaults_user_config.as_ref(), None),
    };
    primary
        .map(|p| build_defaults_entries(p, fallback))
        .unwrap_or_default()
}

fn entries_count_for_cache(cache: &super::ProjectViewCache) -> usize {
    entries_for_cache(cache).len()
}

/// Get a mutable reference to whichever config the user is currently
/// editing (project or ~/.gsd/defaults.json).
fn active_config_mut(
    cache: &mut super::ProjectViewCache,
) -> Option<&mut crate::state_reader::config_json::GsdConfig> {
    use super::DefaultsEditTarget;
    match cache.defaults_edit_target {
        DefaultsEditTarget::Project => cache.defaults_config.as_mut(),
        DefaultsEditTarget::Global => cache.defaults_user_config.as_mut(),
    }
}

/// Persist the active config to disk and surface a status message.
fn persist_active_config(
    target: super::DefaultsEditTarget,
    project_path: Option<&std::path::Path>,
    config: &crate::state_reader::config_json::GsdConfig,
    status_message: &mut Option<(String, std::time::Instant)>,
) {
    use super::DefaultsEditTarget;
    let path = match target {
        DefaultsEditTarget::Project => project_path.map(|p| p.join(".planning/config.json")),
        DefaultsEditTarget::Global => crate::state_reader::config_json::user_defaults_path(),
    };
    let Some(path) = path else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = crate::state_reader::config_json::serialize_gsd_config(config) {
        if std::fs::write(&path, &json).is_ok() {
            let label = match target {
                DefaultsEditTarget::Project => "Config saved",
                DefaultsEditTarget::Global => "Defaults saved",
            };
            *status_message = Some((label.to_string(), std::time::Instant::now()));
        }
    }
}

/// Returns the list of selectable values for an entry's kind, or empty if the
/// list is not statically known (e.g. free-form strings, integers).
fn dropdown_options(kind: &ConfigValueKind) -> Vec<String> {
    match kind {
        ConfigValueKind::Bool => vec!["true".to_string(), "false".to_string()],
        ConfigValueKind::Enum(opts) => opts.iter().map(|s| s.to_string()).collect(),
        _ => Vec::new(),
    }
}

/// Set a bool/enum config field to a specific value (selected from a dropdown).
/// Returns true if the field was recognized and updated.
fn set_config_value(
    config: &mut crate::state_reader::config_json::GsdConfig,
    key: &str,
    value: &str,
) -> bool {
    use crate::state_reader::config_json::*;

    let parse_bool = |v: &str| match v {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    };

    if let Some(b) = parse_bool(value) {
        match key {
            "commit_docs" => { config.commit_docs = Some(b); return true; }
            "parallelization" => { config.parallelization = Some(b); return true; }
            "search_gitignored" => { config.search_gitignored = Some(b); return true; }
            "brave_search" => { config.brave_search = Some(b); return true; }
            "firecrawl" => { config.firecrawl = Some(b); return true; }
            "exa_search" => { config.exa_search = Some(b); return true; }
            "research" => { config.workflow.get_or_insert_with(WorkflowConfig::default).research = Some(b); return true; }
            "plan_check" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_check = Some(b); return true; }
            "verifier" => { config.workflow.get_or_insert_with(WorkflowConfig::default).verifier = Some(b); return true; }
            "nyquist_validation" => { config.workflow.get_or_insert_with(WorkflowConfig::default).nyquist_validation = Some(b); return true; }
            "auto_advance" => { config.workflow.get_or_insert_with(WorkflowConfig::default).auto_advance = Some(b); return true; }
            "node_repair" => { config.workflow.get_or_insert_with(WorkflowConfig::default).node_repair = Some(b); return true; }
            "ui_phase" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_phase = Some(b); return true; }
            "ui_safety_gate" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_safety_gate = Some(b); return true; }
            "text_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).text_mode = Some(b); return true; }
            "research_before_questions" => { config.workflow.get_or_insert_with(WorkflowConfig::default).research_before_questions = Some(b); return true; }
            "skip_discuss" => { config.workflow.get_or_insert_with(WorkflowConfig::default).skip_discuss = Some(b); return true; }
            "auto_chain_active" => { config.workflow.get_or_insert_with(WorkflowConfig::default).auto_chain_active = Some(b); return true; }
            "use_worktrees" => { config.workflow.get_or_insert_with(WorkflowConfig::default).use_worktrees = Some(b); return true; }
            "context_warnings" => { config.hooks.get_or_insert_with(HooksConfig::default).context_warnings = Some(b); return true; }
            "intel_enabled" => { config.intel.get_or_insert_with(IntelConfig::default).enabled = Some(b); return true; }
            "graphify_enabled" => { config.graphify.get_or_insert_with(GraphifyConfig::default).enabled = Some(b); return true; }
            "pattern_mapper" => { config.workflow.get_or_insert_with(WorkflowConfig::default).pattern_mapper = Some(b); return true; }
            "ai_integration_phase" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ai_integration_phase = Some(b); return true; }
            "tdd_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).tdd_mode = Some(b); return true; }
            "code_review" => { config.workflow.get_or_insert_with(WorkflowConfig::default).code_review = Some(b); return true; }
            "ui_review" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_review = Some(b); return true; }
            // GSD 1.4–1.8 workflow gates
            "workflow.specless_probe_fallback" => { config.workflow.get_or_insert_with(WorkflowConfig::default).specless_probe_fallback = Some(b); return true; }
            "workflow.assumption_delta" => { config.workflow.get_or_insert_with(WorkflowConfig::default).assumption_delta = Some(b); return true; }
            "workflow.plan_drift_precheck" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_drift_precheck = Some(b); return true; }
            "workflow.plan_chunked" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_chunked = Some(b); return true; }
            "workflow.api_coverage_gate" => { config.workflow.get_or_insert_with(WorkflowConfig::default).api_coverage_gate = Some(b); return true; }
            "workflow.windows_enforce" => { config.workflow.get_or_insert_with(WorkflowConfig::default).windows_enforce = Some(b); return true; }
            "workflow.mvp_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).mvp_mode = Some(b); return true; }
            // GSD 1.8 top-level blocks
            "claude_orchestration.enabled" => { config.claude_orchestration.get_or_insert_with(ClaudeOrchestrationConfig::default).enabled = Some(b); return true; }
            "statusline.show_context_tokens" => { config.statusline.get_or_insert_with(StatuslineConfig::default).show_context_tokens = Some(b); return true; }
            "statusline.show_git" => { config.statusline.get_or_insert_with(StatuslineConfig::default).show_git = Some(b); return true; }
            "dynamic_routing.provider_escalation" => { config.dynamic_routing.get_or_insert_with(DynamicRoutingConfig::default).provider_escalation = Some(b); return true; }
            "capabilities.strict_known_registries" => { config.capabilities.get_or_insert_with(CapabilitiesConfig::default).strict_known_registries = Some(b); return true; }
            "capabilities.auto_update" => { config.capabilities.get_or_insert_with(CapabilitiesConfig::default).auto_update = Some(b); return true; }
            _ => {}
        }
    }

    // Enum string values
    match key {
        "mode" => { config.mode = value.to_string(); true }
        "granularity" => { config.granularity = value.to_string(); true }
        "model_profile" => { config.model_profile = value.to_string(); true }
        "branching_strategy" => {
            config.git.get_or_insert_with(GitConfig::default).branching_strategy = Some(value.to_string());
            true
        }
        "discuss_mode" => {
            config.workflow.get_or_insert_with(WorkflowConfig::default).discuss_mode = Some(value.to_string());
            true
        }
        "code_review_depth" => {
            config.workflow.get_or_insert_with(WorkflowConfig::default).code_review_depth = Some(value.to_string());
            true
        }
        _ => false,
    }
}

/// Set a String-kind config field by key. Returns true if recognized.
/// `value` is assumed to be non-empty (callers route empty input through
/// `clear_config_value` instead).
fn set_string_value(
    config: &mut crate::state_reader::config_json::GsdConfig,
    key: &str,
    value: &str,
) -> bool {
    use crate::state_reader::config_json::*;

    match key {
        "project_code" => { config.project_code = Some(value.to_string()); true }
        "phase_naming" => { config.phase_naming = Some(value.to_string()); true }
        "response_language" => { config.response_language = Some(value.to_string()); true }
        "base_branch" => {
            config.git.get_or_insert_with(GitConfig::default).base_branch = Some(value.to_string());
            true
        }
        "phase_branch_template" => {
            config.git.get_or_insert_with(GitConfig::default).phase_branch_template = Some(value.to_string());
            true
        }
        "milestone_branch_template" => {
            config.git.get_or_insert_with(GitConfig::default).milestone_branch_template = Some(value.to_string());
            true
        }
        "quick_branch_template" => {
            config.git.get_or_insert_with(GitConfig::default).quick_branch_template =
                Some(serde_json::Value::String(value.to_string()));
            true
        }
        // GSD 1.4–1.8 string keys
        "workflow.context_guard_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).context_guard_mode = Some(value.to_string()); true }
        "workflow.code_review_command" => { config.workflow.get_or_insert_with(WorkflowConfig::default).code_review_command = Some(value.to_string()); true }
        "graphify.graph_path" => { config.graphify.get_or_insert_with(GraphifyConfig::default).graph_path = Some(value.to_string()); true }
        "phase_id_convention" => { config.phase_id_convention = Some(value.to_string()); true }
        "claude_md_path" => { config.claude_md_path = Some(value.to_string()); true }
        "claude_orchestration.execution_backend" => { config.claude_orchestration.get_or_insert_with(ClaudeOrchestrationConfig::default).execution_backend = Some(value.to_string()); true }
        "claude_orchestration.min_agent_sdk_version" => { config.claude_orchestration.get_or_insert_with(ClaudeOrchestrationConfig::default).min_agent_sdk_version = Some(value.to_string()); true }
        "statusline.state_format" => { config.statusline.get_or_insert_with(StatuslineConfig::default).state_format = Some(value.to_string()); true }
        "external_job.artifact_dir" => { config.external_job.get_or_insert_with(ExternalJobConfig::default).artifact_dir = Some(value.to_string()); true }
        _ => false,
    }
}

/// Clear (set to None / unset) the config field identified by `key`.
/// Required scalar fields (`mode`, `granularity`, `model_profile`) are NOT
/// clearable — those rows ignore the clear shortcut.
fn clear_config_value(
    config: &mut crate::state_reader::config_json::GsdConfig,
    key: &str,
) -> bool {
    use crate::state_reader::config_json::*;

    match key {
        // Required (non-Option) fields — refuse to clear.
        "mode" | "granularity" | "model_profile" => false,

        // Top-level Options
        "commit_docs" => { config.commit_docs = None; true }
        "parallelization" => { config.parallelization = None; true }
        "search_gitignored" => { config.search_gitignored = None; true }
        "brave_search" => { config.brave_search = None; true }
        "firecrawl" => { config.firecrawl = None; true }
        "exa_search" => { config.exa_search = None; true }
        "project_code" => { config.project_code = None; true }
        "phase_naming" => { config.phase_naming = None; true }
        "response_language" => { config.response_language = None; true }

        // Git
        "branching_strategy" => { config.git.get_or_insert_with(GitConfig::default).branching_strategy = None; true }
        "base_branch" => { config.git.get_or_insert_with(GitConfig::default).base_branch = None; true }
        "phase_branch_template" => { config.git.get_or_insert_with(GitConfig::default).phase_branch_template = None; true }
        "milestone_branch_template" => { config.git.get_or_insert_with(GitConfig::default).milestone_branch_template = None; true }
        "quick_branch_template" => { config.git.get_or_insert_with(GitConfig::default).quick_branch_template = None; true }

        // Workflow
        "research" => { config.workflow.get_or_insert_with(WorkflowConfig::default).research = None; true }
        "plan_check" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_check = None; true }
        "verifier" => { config.workflow.get_or_insert_with(WorkflowConfig::default).verifier = None; true }
        "nyquist_validation" => { config.workflow.get_or_insert_with(WorkflowConfig::default).nyquist_validation = None; true }
        "auto_advance" => { config.workflow.get_or_insert_with(WorkflowConfig::default).auto_advance = None; true }
        "node_repair" => { config.workflow.get_or_insert_with(WorkflowConfig::default).node_repair = None; true }
        "node_repair_budget" => { config.workflow.get_or_insert_with(WorkflowConfig::default).node_repair_budget = None; true }
        "ui_phase" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_phase = None; true }
        "ui_safety_gate" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_safety_gate = None; true }
        "text_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).text_mode = None; true }
        "research_before_questions" => { config.workflow.get_or_insert_with(WorkflowConfig::default).research_before_questions = None; true }
        "discuss_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).discuss_mode = None; true }
        "skip_discuss" => { config.workflow.get_or_insert_with(WorkflowConfig::default).skip_discuss = None; true }
        "auto_chain_active" => { config.workflow.get_or_insert_with(WorkflowConfig::default).auto_chain_active = None; true }
        "use_worktrees" => { config.workflow.get_or_insert_with(WorkflowConfig::default).use_worktrees = None; true }
        "subagent_timeout" => { config.workflow.get_or_insert_with(WorkflowConfig::default).subagent_timeout = None; true }

        // Hooks
        "context_warnings" => { config.hooks.get_or_insert_with(HooksConfig::default).context_warnings = None; true }

        // Intel / Graphify
        "intel_enabled" => { config.intel.get_or_insert_with(IntelConfig::default).enabled = None; true }
        "graphify_enabled" => { config.graphify.get_or_insert_with(GraphifyConfig::default).enabled = None; true }
        "graphify_build_timeout" => { config.graphify.get_or_insert_with(GraphifyConfig::default).build_timeout = None; true }

        // Workflow toggles added with the /gsd-settings six-section layout
        "pattern_mapper" => { config.workflow.get_or_insert_with(WorkflowConfig::default).pattern_mapper = None; true }
        "ai_integration_phase" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ai_integration_phase = None; true }
        "tdd_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).tdd_mode = None; true }
        "code_review" => { config.workflow.get_or_insert_with(WorkflowConfig::default).code_review = None; true }
        "code_review_depth" => { config.workflow.get_or_insert_with(WorkflowConfig::default).code_review_depth = None; true }
        "ui_review" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_review = None; true }

        // GSD 1.4–1.8 workflow gates
        "workflow.specless_probe_fallback" => { config.workflow.get_or_insert_with(WorkflowConfig::default).specless_probe_fallback = None; true }
        "workflow.assumption_delta" => { config.workflow.get_or_insert_with(WorkflowConfig::default).assumption_delta = None; true }
        "workflow.plan_drift_precheck" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_drift_precheck = None; true }
        "workflow.plan_chunked" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_chunked = None; true }
        "workflow.context_guard_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).context_guard_mode = None; true }
        "workflow.api_coverage_gate" => { config.workflow.get_or_insert_with(WorkflowConfig::default).api_coverage_gate = None; true }
        "workflow.windows_enforce" => { config.workflow.get_or_insert_with(WorkflowConfig::default).windows_enforce = None; true }
        "workflow.mvp_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).mvp_mode = None; true }
        "workflow.test_gate_timeout" => { config.workflow.get_or_insert_with(WorkflowConfig::default).test_gate_timeout = None; true }
        "workflow.code_review_command" => { config.workflow.get_or_insert_with(WorkflowConfig::default).code_review_command = None; true }
        "graphify.graph_path" => { config.graphify.get_or_insert_with(GraphifyConfig::default).graph_path = None; true }

        // GSD 1.8 top-level blocks
        "phase_id_convention" => { config.phase_id_convention = None; true }
        "claude_md_path" => { config.claude_md_path = None; true }
        "claude_orchestration.enabled" => { config.claude_orchestration.get_or_insert_with(ClaudeOrchestrationConfig::default).enabled = None; true }
        "claude_orchestration.execution_backend" => { config.claude_orchestration.get_or_insert_with(ClaudeOrchestrationConfig::default).execution_backend = None; true }
        "claude_orchestration.min_agent_sdk_version" => { config.claude_orchestration.get_or_insert_with(ClaudeOrchestrationConfig::default).min_agent_sdk_version = None; true }
        "statusline.show_context_tokens" => { config.statusline.get_or_insert_with(StatuslineConfig::default).show_context_tokens = None; true }
        "statusline.state_format" => { config.statusline.get_or_insert_with(StatuslineConfig::default).state_format = None; true }
        "statusline.show_git" => { config.statusline.get_or_insert_with(StatuslineConfig::default).show_git = None; true }
        "dynamic_routing.provider_escalation" => { config.dynamic_routing.get_or_insert_with(DynamicRoutingConfig::default).provider_escalation = None; true }
        "dynamic_routing.max_escalations" => { config.dynamic_routing.get_or_insert_with(DynamicRoutingConfig::default).max_escalations = None; true }
        "external_job.submit_timeout_ms" => { config.external_job.get_or_insert_with(ExternalJobConfig::default).submit_timeout_ms = None; true }
        "external_job.poll_timeout_ms" => { config.external_job.get_or_insert_with(ExternalJobConfig::default).poll_timeout_ms = None; true }
        "external_job.artifact_dir" => { config.external_job.get_or_insert_with(ExternalJobConfig::default).artifact_dir = None; true }
        "capabilities.strict_known_registries" => { config.capabilities.get_or_insert_with(CapabilitiesConfig::default).strict_known_registries = None; true }
        "capabilities.auto_update" => { config.capabilities.get_or_insert_with(CapabilitiesConfig::default).auto_update = None; true }

        _ => false,
    }
}

/// Mutate the config field identified by `key`. Returns true if a mutation was made.
fn mutate_config_entry(
    config: &mut crate::state_reader::config_json::GsdConfig,
    key: &str,
    kind: &ConfigValueKind,
) -> bool {
    use crate::state_reader::config_json::*;

    match kind {
        ConfigValueKind::Null => false, // Cannot toggle unset values without initializing parent
        ConfigValueKind::String => false, // String editing not supported via Enter
        ConfigValueKind::Bool => {
            // Toggle the boolean field
            match key {
                "commit_docs" => { config.commit_docs = Some(!config.commit_docs.unwrap_or(false)); true }
                "parallelization" => { config.parallelization = Some(!config.parallelization.unwrap_or(false)); true }
                "search_gitignored" => { config.search_gitignored = Some(!config.search_gitignored.unwrap_or(false)); true }
                "brave_search" => { config.brave_search = Some(!config.brave_search.unwrap_or(false)); true }
                "firecrawl" => { config.firecrawl = Some(!config.firecrawl.unwrap_or(false)); true }
                "exa_search" => { config.exa_search = Some(!config.exa_search.unwrap_or(false)); true }
                "research" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.research = Some(!wf.research.unwrap_or(false)); true }
                "plan_check" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.plan_check = Some(!wf.plan_check.unwrap_or(false)); true }
                "verifier" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.verifier = Some(!wf.verifier.unwrap_or(false)); true }
                "nyquist_validation" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.nyquist_validation = Some(!wf.nyquist_validation.unwrap_or(false)); true }
                "auto_advance" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.auto_advance = Some(!wf.auto_advance.unwrap_or(false)); true }
                "node_repair" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.node_repair = Some(!wf.node_repair.unwrap_or(false)); true }
                "ui_phase" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.ui_phase = Some(!wf.ui_phase.unwrap_or(false)); true }
                "ui_safety_gate" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.ui_safety_gate = Some(!wf.ui_safety_gate.unwrap_or(false)); true }
                "text_mode" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.text_mode = Some(!wf.text_mode.unwrap_or(false)); true }
                "research_before_questions" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.research_before_questions = Some(!wf.research_before_questions.unwrap_or(false)); true }
                "skip_discuss" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.skip_discuss = Some(!wf.skip_discuss.unwrap_or(false)); true }
                "auto_chain_active" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.auto_chain_active = Some(!wf.auto_chain_active.unwrap_or(false)); true }
                "use_worktrees" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.use_worktrees = Some(!wf.use_worktrees.unwrap_or(false)); true }
                "context_warnings" => { let hooks = config.hooks.get_or_insert_with(HooksConfig::default); hooks.context_warnings = Some(!hooks.context_warnings.unwrap_or(false)); true }
                "intel_enabled" => { let i = config.intel.get_or_insert_with(IntelConfig::default); i.enabled = Some(!i.enabled.unwrap_or(false)); true }
                "graphify_enabled" => { let g = config.graphify.get_or_insert_with(GraphifyConfig::default); g.enabled = Some(!g.enabled.unwrap_or(false)); true }
                "pattern_mapper" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.pattern_mapper = Some(!wf.pattern_mapper.unwrap_or(false)); true }
                "ai_integration_phase" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.ai_integration_phase = Some(!wf.ai_integration_phase.unwrap_or(false)); true }
                "tdd_mode" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.tdd_mode = Some(!wf.tdd_mode.unwrap_or(false)); true }
                "code_review" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.code_review = Some(!wf.code_review.unwrap_or(false)); true }
                "ui_review" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.ui_review = Some(!wf.ui_review.unwrap_or(false)); true }
                // GSD 1.4–1.8 workflow gates
                "workflow.specless_probe_fallback" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.specless_probe_fallback = Some(!wf.specless_probe_fallback.unwrap_or(false)); true }
                "workflow.assumption_delta" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.assumption_delta = Some(!wf.assumption_delta.unwrap_or(false)); true }
                "workflow.plan_drift_precheck" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.plan_drift_precheck = Some(!wf.plan_drift_precheck.unwrap_or(false)); true }
                "workflow.plan_chunked" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.plan_chunked = Some(!wf.plan_chunked.unwrap_or(false)); true }
                "workflow.api_coverage_gate" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.api_coverage_gate = Some(!wf.api_coverage_gate.unwrap_or(false)); true }
                "workflow.windows_enforce" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.windows_enforce = Some(!wf.windows_enforce.unwrap_or(false)); true }
                "workflow.mvp_mode" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.mvp_mode = Some(!wf.mvp_mode.unwrap_or(false)); true }
                // GSD 1.8 top-level blocks
                "claude_orchestration.enabled" => { let c = config.claude_orchestration.get_or_insert_with(ClaudeOrchestrationConfig::default); c.enabled = Some(!c.enabled.unwrap_or(false)); true }
                "statusline.show_context_tokens" => { let s = config.statusline.get_or_insert_with(StatuslineConfig::default); s.show_context_tokens = Some(!s.show_context_tokens.unwrap_or(false)); true }
                "statusline.show_git" => { let s = config.statusline.get_or_insert_with(StatuslineConfig::default); s.show_git = Some(!s.show_git.unwrap_or(false)); true }
                "dynamic_routing.provider_escalation" => { let r = config.dynamic_routing.get_or_insert_with(DynamicRoutingConfig::default); r.provider_escalation = Some(!r.provider_escalation.unwrap_or(false)); true }
                "capabilities.strict_known_registries" => { let c = config.capabilities.get_or_insert_with(CapabilitiesConfig::default); c.strict_known_registries = Some(!c.strict_known_registries.unwrap_or(false)); true }
                "capabilities.auto_update" => { let c = config.capabilities.get_or_insert_with(CapabilitiesConfig::default); c.auto_update = Some(!c.auto_update.unwrap_or(false)); true }
                _ => false,
            }
        }
        ConfigValueKind::Enum(options) => {
            let cycle = |current: &str| -> String {
                let idx = options.iter().position(|&o| o == current).unwrap_or(0);
                let next = (idx + 1) % options.len();
                options[next].to_string()
            };
            match key {
                "mode" => { config.mode = cycle(&config.mode); true }
                "granularity" => { config.granularity = cycle(&config.granularity); true }
                "model_profile" => { config.model_profile = cycle(&config.model_profile); true }
                "branching_strategy" => {
                    let git = config.git.get_or_insert_with(GitConfig::default);
                    let current = git.branching_strategy.as_deref().unwrap_or("none");
                    git.branching_strategy = Some(cycle(current));
                    true
                }
                "discuss_mode" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.discuss_mode.as_deref().unwrap_or("discuss");
                    wf.discuss_mode = Some(cycle(current));
                    true
                }
                _ => false,
            }
        }
        ConfigValueKind::Integer => {
            match key {
                "node_repair_budget" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.node_repair_budget.unwrap_or(0);
                    wf.node_repair_budget = Some(if current >= 10 { 0 } else { current + 1 });
                    true
                }
                "subagent_timeout" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.subagent_timeout.unwrap_or(0);
                    wf.subagent_timeout = Some(if current >= 600 { 60 } else { current + 30 });
                    true
                }
                "graphify_build_timeout" => {
                    let g = config.graphify.get_or_insert_with(GraphifyConfig::default);
                    let current = g.build_timeout.unwrap_or(0);
                    g.build_timeout = Some(if current >= 600 { 60 } else { current + 30 });
                    true
                }
                "dynamic_routing.max_escalations" => {
                    let r = config.dynamic_routing.get_or_insert_with(DynamicRoutingConfig::default);
                    let current = r.max_escalations.unwrap_or(0);
                    r.max_escalations = Some(if current >= 10 { 0 } else { current + 1 });
                    true
                }
                "workflow.test_gate_timeout" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.test_gate_timeout.unwrap_or(0);
                    wf.test_gate_timeout = Some(if current >= 600 { 60 } else { current + 30 });
                    true
                }
                "external_job.submit_timeout_ms" => {
                    let e = config.external_job.get_or_insert_with(ExternalJobConfig::default);
                    let current = e.submit_timeout_ms.unwrap_or(0);
                    e.submit_timeout_ms = Some(if current >= 120000 { 5000 } else { current + 5000 });
                    true
                }
                "external_job.poll_timeout_ms" => {
                    let e = config.external_job.get_or_insert_with(ExternalJobConfig::default);
                    let current = e.poll_timeout_ms.unwrap_or(0);
                    e.poll_timeout_ms = Some(if current >= 60000 { 1000 } else { current + 1000 });
                    true
                }
                _ => false,
            }
        }
        // Shape-varying keys are surfaced read-only; never mutated in place.
        ConfigValueKind::ReadOnly => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_waves_manifest_two_waves() {
        let raw = r#"{
            "waves": [
                {
                    "id": "w1",
                    "plans": [
                        { "id": "p1", "files_modified": ["src/foo.rs"] },
                        { "id": "p2", "files_modified": ["src/bar.rs"] }
                    ]
                },
                {
                    "id": "w2",
                    "plans": [
                        { "id": "p3", "files_modified": ["src/baz.rs"] }
                    ]
                }
            ]
        }"#;
        let manifest = parse_waves_manifest(raw).expect("valid manifest parses");
        assert_eq!(manifest.waves.len(), 2);
        assert_eq!(manifest.waves[0].label(0), "w1");
        assert_eq!(manifest.waves[0].plans.len(), 2);
        assert_eq!(manifest.waves[0].plans[0].label(), Some("p1".to_string()));
        assert_eq!(manifest.waves[1].label(1), "w2");
        assert_eq!(manifest.waves[1].plans.len(), 1);

        // Rendering produces a header + one line per wave.
        let lines = build_waves_lines(&manifest);
        assert_eq!(lines.len(), 3);
    }

    #[test]
    fn test_parse_waves_manifest_garbage_is_none() {
        assert!(parse_waves_manifest("not json at all").is_none());
        assert!(parse_waves_manifest("{ oops ]").is_none());
    }

    #[test]
    fn test_parse_waves_manifest_lenient_defaults() {
        // Numeric wave id and missing plan ids are tolerated.
        let raw = r#"{ "waves": [ { "wave": 1, "plans": [ { "files_modified": [] } ] } ] }"#;
        let manifest = parse_waves_manifest(raw).expect("lenient parse");
        assert_eq!(manifest.waves.len(), 1);
        assert_eq!(manifest.waves[0].label(0), "1");
        // A plan with no id falls back to None (rendered as "(no plans)").
        assert_eq!(manifest.waves[0].plans[0].label(), None);

        // Empty object → empty waves, still Some (render layer skips it).
        let empty = parse_waves_manifest("{}").expect("empty object parses");
        assert!(empty.waves.is_empty());
    }

    // ── UIFIX-03: Docs (Browse) tab `[e]dit` key routing ──────────────────

    use crate::browser::{BrowserDepth, BrowserEntry};
    use crate::ui::screens::ProjectViewCache;
    use std::path::PathBuf;

    const NO_FILE_MSG: &str = "Select a markdown file to edit";
    const READ_ONLY_MSG: &str = "Archived files are read-only";

    /// A Browse cache rooted at `/proj/.planning`, sitting in a phase dir.
    fn browse_cache() -> ProjectViewCache {
        ProjectViewCache {
            browser_root: Some(PathBuf::from("/proj/.planning")),
            browser_current_dir: Some(PathBuf::from("/proj/.planning/phases/14-ui-fixes")),
            ..Default::default()
        }
    }

    fn md_entry(name: &str) -> BrowserEntry {
        BrowserEntry {
            name: Untrusted::from_untrusted_source(name.to_string()),
            path: PathBuf::from("/proj/.planning/phases/14-ui-fixes").join(name),
            is_dir: false,
        }
    }

    #[test]
    fn test_browse_edit_target_view_depth_returns_file_path() {
        let mut cache = browse_cache();
        cache.browser_depth = BrowserDepth::View;
        cache.browser_file_name = Some(Untrusted::from_untrusted_source("14-02-PLAN.md".to_string()));
        cache.browser_file_content = Some("# Plan\n".to_string());

        assert_eq!(
            browse_edit_target(&cache),
            Ok(PathBuf::from(
                "/proj/.planning/phases/14-ui-fixes/14-02-PLAN.md"
            ))
        );
    }

    #[test]
    fn test_browse_edit_target_list_depth_md_file() {
        let mut cache = browse_cache();
        cache.browser_depth = BrowserDepth::List;
        cache.browser_entries = vec![
            BrowserEntry {
                name: Untrusted::from_untrusted_source("sub".to_string()),
                path: PathBuf::from("/proj/.planning/phases/14-ui-fixes/sub"),
                is_dir: true,
            },
            md_entry("14-02-PLAN.md"),
        ];
        cache.browser_selected = 1;

        assert_eq!(
            browse_edit_target(&cache),
            Ok(PathBuf::from(
                "/proj/.planning/phases/14-ui-fixes/14-02-PLAN.md"
            ))
        );
    }

    #[test]
    fn test_browse_edit_target_list_depth_directory_is_rejected() {
        let mut cache = browse_cache();
        cache.browser_depth = BrowserDepth::List;
        cache.browser_entries = vec![BrowserEntry {
            name: Untrusted::from_untrusted_source("sub".to_string()),
            path: PathBuf::from("/proj/.planning/phases/14-ui-fixes/sub"),
            is_dir: true,
        }];
        cache.browser_selected = 0;

        assert_eq!(browse_edit_target(&cache), Err(NO_FILE_MSG));
    }

    #[test]
    fn test_browse_edit_target_empty_listing_is_rejected() {
        let mut cache = browse_cache();
        cache.browser_depth = BrowserDepth::List;
        cache.browser_entries = Vec::new();
        cache.browser_selected = 0;

        assert_eq!(browse_edit_target(&cache), Err(NO_FILE_MSG));
    }

    #[test]
    fn test_browse_edit_target_milestones_path_is_read_only() {
        let mut cache = browse_cache();
        cache.browser_depth = BrowserDepth::View;
        cache.browser_current_dir =
            Some(PathBuf::from("/proj/.planning/milestones/v1.2-phases"));
        cache.browser_file_name = Some(Untrusted::from_untrusted_source("11-SUMMARY.md".to_string()));
        cache.browser_file_content = Some("archived".to_string());

        assert_eq!(browse_edit_target(&cache), Err(READ_ONLY_MSG));
    }

    #[test]
    fn test_browse_edit_target_outside_root_is_rejected() {
        let mut cache = browse_cache();
        cache.browser_depth = BrowserDepth::List;
        cache.browser_entries = vec![BrowserEntry {
            name: Untrusted::from_untrusted_source("passwd.md".to_string()),
            path: PathBuf::from("/etc/passwd.md"),
            is_dir: false,
        }];
        cache.browser_selected = 0;

        assert_eq!(browse_edit_target(&cache), Err(NO_FILE_MSG));
    }

    #[test]
    fn test_browse_edit_target_loading_content_is_inert() {
        let mut cache = browse_cache();
        cache.browser_depth = BrowserDepth::View;
        cache.browser_file_name = Some(Untrusted::from_untrusted_source("14-02-PLAN.md".to_string()));
        // Still in the Phase 12 `Loading...` window.
        cache.browser_file_content = None;

        assert_eq!(browse_edit_target(&cache), Err(NO_FILE_MSG));
    }

    /// The footer's hint set as one string, at a width where every tab except
    /// the Driver tab renders its single shipped form.
    fn footer_text(sub_view: &DetailSubView) -> String {
        footer_text_at(sub_view, 120)
    }

    fn footer_text_at(sub_view: &DetailSubView, width: u16) -> String {
        footer_spans(sub_view, width)
            .iter()
            .map(|s| s.content.as_ref())
            .collect()
    }

    #[test]
    fn test_browse_footer_has_edit_hint() {
        let text = footer_text(&DetailSubView::Browse);
        assert!(text.contains("[e]dit  "));

        // Ordering matches the Archive arm: [Enter]open  [e]dit  … [Esc]up
        let open = text.find("[Enter]open").expect("open hint present");
        let edit = text.find("[e]dit").expect("edit hint present");
        let up = text.find("[Esc]up").expect("up hint present");
        assert!(open < edit);
        assert!(edit < up);
    }

    #[test]
    fn test_other_footers_unchanged_by_browse_edit_hint() {
        assert_eq!(
            footer_text(&DetailSubView::Backlog),
            "  [Esc]back  [1-0/D]tabs  [j/k]scroll  [Enter]xpand  [e]nqueue  [?]help"
        );
        assert_eq!(
            footer_text(&DetailSubView::Defaults),
            "  [Esc]back  [1-0/D]tabs  [j/k]scroll  [Enter]edit  [x] clear  [d] defaults  \
             [r]eload  [?]help"
        );
    }

    // ── Plan 18-09: the 11th tab, at all six sites ────────────────────────

    /// The shared prefix's tabs hint changes **once, for every tab**, so
    /// `Shift+D` is discoverable from anywhere in the detail view — not only
    /// from the tab it opens, which the user has no reason to be on.
    #[test]
    fn the_tabs_hint_names_shift_d_on_every_tab() {
        for sub_view in [
            DetailSubView::PhaseList,
            DetailSubView::RoadmapViz,
            DetailSubView::Backlog,
            DetailSubView::GitHistory,
            DetailSubView::Pipeline,
            DetailSubView::Queue,
            DetailSubView::Sessions,
            DetailSubView::Archive,
            DetailSubView::Defaults,
            DetailSubView::Browse,
        ] {
            let text = footer_text(&sub_view);
            assert!(
                text.contains("[1-0/D]tabs"),
                "{sub_view:?} footer must advertise the Driver tab: {text}"
            );
            assert!(
                !text.contains("[1-9]tabs"),
                "{sub_view:?} still shows the pre-18-09 digits-only hint: {text}"
            );
        }
    }

    #[test]
    fn the_driver_footer_has_three_measured_width_forms() {
        assert_eq!(
            footer_text_at(&DetailSubView::Driver, 120),
            "  [Esc]back  [1-0/D]tabs  [j/k]runs  [PgUp/PgDn]output  [f]ollow  [i]nject  \
             [s]tart  [x]stop  [?]help"
        );
        assert_eq!(
            footer_text_at(&DetailSubView::Driver, 80),
            "  [Esc]back  [j/k]runs  [f]ollow  [i]nject  [s]tart  [x]stop  [?]help"
        );
        assert_eq!(
            footer_text_at(&DetailSubView::Driver, 50),
            "  [j/k]runs  [i]nject  [s]tart  [x]stop  [?]help"
        );
    }

    /// Each form must fit the narrowest width that selects it — the whole point
    /// of tiering is that the last hint is not clipped off the right edge.
    ///
    /// The short form (48 cells) is the floor the UI-SPEC defines: below 48
    /// columns there is no further tier, because the remaining five hints are
    /// the tab's whole vocabulary and dropping one would leave a key with no
    /// discoverable name at all. A terminal that narrow clips the `[?]help`
    /// hint, which is the least-bad outcome available and is not a defect this
    /// function can fix.
    #[test]
    fn each_driver_footer_form_fits_the_width_that_selects_it() {
        for width in [DRIVER_FOOTER_FULL_CELLS, DRIVER_FOOTER_MEDIUM_CELLS, 48] {
            let text = footer_text_at(&DetailSubView::Driver, width);
            assert!(
                text.chars().count() <= usize::from(width),
                "the driver footer at width {width} is {} cells: {text}",
                text.chars().count()
            );
        }
    }

    /// The tab index and the sub-view must agree in both directions, for all
    /// eleven tabs: a tab whose index does not round-trip lands the user on a
    /// different tab than the one they asked for.
    #[test]
    fn every_tab_index_round_trips_through_its_sub_view() {
        for index in 0..TAB_COUNT {
            let view = sub_view_from_index(index);
            assert_eq!(
                tab_index(&view),
                index,
                "index {index} mapped to {view:?}, which maps back to {}",
                tab_index(&view)
            );
        }
        assert_eq!(sub_view_from_index(DRIVER_TAB_INDEX), DetailSubView::Driver);
        assert_eq!(tab_index(&DetailSubView::Driver), DRIVER_TAB_INDEX);
        // The out-of-range fallback still lands on the first tab, not the newest.
        assert_eq!(sub_view_from_index(TAB_COUNT), DetailSubView::PhaseList);
    }

    /// The rendered width of one `Line`, in cells.
    fn line_cells(line: &Line<'static>) -> usize {
        line.spans.iter().map(|s| s.content.chars().count()).sum()
    }

    /// The width `Tabs` needs for a whole title vector: each entry costs its
    /// label plus two pads, plus a one-cell divider between adjacent entries.
    fn bar_cells(titles: &[Line<'static>]) -> usize {
        titles.iter().map(|t| line_cells(t) + 2).sum::<usize>() + titles.len().saturating_sub(1)
    }

    #[test]
    fn the_full_tier_renders_eleven_labels_at_its_measured_width() {
        let (titles, select) = tab_titles(TAB_BAR_FULL_CELLS, DRIVER_TAB_INDEX, false);
        assert_eq!(titles.len(), TAB_COUNT);
        assert_eq!(select, DRIVER_TAB_INDEX);
        assert_eq!(bar_cells(&titles), usize::from(TAB_BAR_FULL_CELLS));
    }

    #[test]
    fn the_compact_tier_renders_eleven_labels_at_its_measured_width() {
        let (titles, select) = tab_titles(TAB_BAR_COMPACT_CELLS, 4, false);
        assert_eq!(titles.len(), TAB_COUNT);
        assert_eq!(select, 4);
        assert_eq!(bar_cells(&titles), usize::from(TAB_BAR_COMPACT_CELLS));
        // One cell below the full width is already the compact tier.
        let (titles, _) = tab_titles(TAB_BAR_FULL_CELLS - 1, 4, false);
        assert_eq!(bar_cells(&titles), usize::from(TAB_BAR_COMPACT_CELLS));
    }

    /// Below the compact width the bar is windowed — and the window always
    /// contains the active tab, with the returned select index pointing at it.
    /// **A tab bar that silently drops the active tab is a defect, not a tier.**
    #[test]
    fn the_windowed_tier_always_contains_the_active_tab() {
        for active in [0usize, 5, DRIVER_TAB_INDEX] {
            for width in [20u16, 30, 40, 60, 76] {
                let (titles, select) = tab_titles(width, active, false);
                assert!(
                    select < titles.len(),
                    "select {select} is out of range for {} titles at width {width}",
                    titles.len()
                );
                let expected = TAB_LABELS_COMPACT[active];
                let selected_text: String = titles[select]
                    .spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect();
                assert!(
                    selected_text.starts_with(expected),
                    "at width {width} with tab {active} active the selected title was \
                     {selected_text:?}, not {expected:?}"
                );
            }
        }
    }

    /// The window fits the bar, and truncation is announced on whichever side
    /// was cut rather than being silent.
    #[test]
    fn the_windowed_tier_fits_and_marks_the_side_it_truncated() {
        let (titles, _) = tab_titles(40, DRIVER_TAB_INDEX, false);
        assert!(bar_cells(&titles) <= 40, "windowed bar overflows 40 columns");
        let first: String = titles[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(
            first, TAB_OVERFLOW_LEFT,
            "tabs were truncated on the left with no marker"
        );

        let (titles, _) = tab_titles(40, 0, false);
        assert!(bar_cells(&titles) <= 40);
        let last: String = titles[titles.len() - 1]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(
            last, TAB_OVERFLOW_RIGHT,
            "tabs were truncated on the right with no marker"
        );
        // Nothing is truncated on the left when the window starts at 0.
        let first: String = titles[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_ne!(first, TAB_OVERFLOW_LEFT);
    }

    /// The live marker occupies a reserved cell that is **always** present, so
    /// starting or stopping a run never shifts the bar sideways.
    #[test]
    fn the_driver_label_is_the_same_width_live_and_idle() {
        for width in [TAB_BAR_FULL_CELLS, TAB_BAR_COMPACT_CELLS, 40] {
            let (live, _) = tab_titles(width, DRIVER_TAB_INDEX, true);
            let (idle, _) = tab_titles(width, DRIVER_TAB_INDEX, false);
            assert_eq!(
                bar_cells(&live),
                bar_cells(&idle),
                "the bar changed width when a run started, at width {width}"
            );
        }

        let (live, select) = tab_titles(TAB_BAR_FULL_CELLS, DRIVER_TAB_INDEX, true);
        let text: String = live[select]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(text, format!("D:Drive{DRIVER_LIVE_MARKER}"));

        let (idle, select) = tab_titles(TAB_BAR_FULL_CELLS, DRIVER_TAB_INDEX, false);
        let text: String = idle[select]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(text, "D:Drive ");
    }

    /// Held-out render-buffer backstop (UI-SPEC `## UI Considerations`,
    /// overflow row). Not a width calculation: the detail screen is rendered
    /// into a `TestBackend` and the resulting cells are scraped, because the
    /// defect this guards — the tenth and eleventh tabs falling off the right
    /// edge at 80 columns — was invisible to every calculation the code had.
    #[test]
    fn the_active_tab_label_is_always_present_in_the_rendered_bar() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        for width in [40u16, 60, 80, 120] {
            for (active, full, compact) in [
                (0usize, "1:Phases", "1:Ph"),
                (DRIVER_TAB_INDEX, "D:Drive", "D:Dr"),
            ] {
                let mut ctx = test_ctx();
                let screen = DetailScreen::new("meta-mgr".to_string());
                ctx.detail_sub_view_per_project
                    .insert("meta-mgr".to_string(), sub_view_from_index(active));

                let mut terminal = Terminal::new(TestBackend::new(width, 24))
                    .expect("TestBackend terminal");
                terminal
                    .draw(|frame| screen.render(frame, frame.area(), &ctx))
                    .expect("draw the detail screen");

                let buffer = terminal.backend().buffer().clone();
                // Row 0 carries the block title; the tab bar is row 1.
                let bar: String = (0..width)
                    .map(|x| {
                        buffer
                            .cell((x, 1))
                            .map(|cell| cell.symbol())
                            .unwrap_or(" ")
                            .to_string()
                    })
                    .collect();

                assert!(
                    bar.contains(full) || bar.contains(compact),
                    "at {width} columns the active tab ({full}) is absent from the \
                     rendered bar: {bar:?}"
                );

                if width < TAB_BAR_COMPACT_CELLS {
                    assert!(
                        bar.contains(TAB_OVERFLOW_LEFT) || bar.contains(TAB_OVERFLOW_RIGHT),
                        "at {width} columns the bar is truncated with no overflow \
                         marker: {bar:?}"
                    );
                }
            }
        }
    }

    /// `Shift+D` reaches the Driver tab from an arbitrary other tab, and
    /// `Right` can now walk all the way to index 10 rather than stopping at 9.
    #[test]
    fn shift_d_and_right_both_reach_the_driver_tab() {
        let mut ctx = test_ctx();
        let mut screen = DetailScreen::new("meta-mgr".to_string());
        ctx.detail_sub_view_per_project
            .insert("meta-mgr".to_string(), DetailSubView::Pipeline);

        screen.handle_key(KeyCode::Char('D'), KeyModifiers::SHIFT, &mut ctx);
        assert_eq!(
            ctx.detail_sub_view_per_project.get("meta-mgr"),
            Some(&DetailSubView::Driver)
        );

        // Walk right from the last pre-18-09 tab into the new one.
        ctx.detail_sub_view_per_project
            .insert("meta-mgr".to_string(), DetailSubView::Browse);
        screen.handle_key(KeyCode::Right, KeyModifiers::NONE, &mut ctx);
        assert_eq!(
            ctx.detail_sub_view_per_project.get("meta-mgr"),
            Some(&DetailSubView::Driver)
        );

        // …and Right at the last tab is still a no-op rather than a wrap.
        screen.handle_key(KeyCode::Right, KeyModifiers::NONE, &mut ctx);
        assert_eq!(
            ctx.detail_sub_view_per_project.get("meta-mgr"),
            Some(&DetailSubView::Driver)
        );
    }

    // ── UIFIX-04: stored scroll offset is clamped to the rendered viewport ──

    #[test]
    fn test_clamp_scroll_page_down_stops_at_content_end() {
        // 100-line document in a 30-line viewport → max_scroll = 70, which
        // leaves the last content line on screen.
        assert_eq!(clamp_scroll(60 + PAGE_SCROLL_LINES, 100, 30), 70);
    }

    #[test]
    fn test_clamp_scroll_repeated_page_down_is_idempotent() {
        // Already at the end: further PageDown presses do not grow the offset.
        assert_eq!(clamp_scroll(70 + PAGE_SCROLL_LINES, 100, 30), 70);
        assert_eq!(clamp_scroll(clamp_scroll(90, 100, 30) + PAGE_SCROLL_LINES, 100, 30), 70);
    }

    #[test]
    fn test_clamp_scroll_short_document_never_scrolls() {
        // total_lines <= visible_height → max_scroll = 0, PageDown is a no-op.
        assert_eq!(clamp_scroll(PAGE_SCROLL_LINES, 10, 30), 0);
        assert_eq!(clamp_scroll(PAGE_SCROLL_LINES, 30, 30), 0);
    }

    #[test]
    fn test_clamp_scroll_pre_first_render_floor() {
        // Before the first render both metrics are zero — a safe floor.
        assert_eq!(clamp_scroll(PAGE_SCROLL_LINES, 0, 0), 0);
    }

    #[test]
    fn test_clamp_scroll_first_page_up_moves_viewport() {
        // Backstop for the UI-SPEC long-document row: after repeated PageDown
        // past the end, the *first* PageUp must move the viewport.
        let mut offset = 0u16;
        for _ in 0..10 {
            offset = clamp_scroll(offset.saturating_add(PAGE_SCROLL_LINES), 100, 30);
        }
        assert_eq!(offset, 70);

        let after_page_up = offset.saturating_sub(PAGE_SCROLL_LINES);
        assert_eq!(after_page_up, 50);
        assert!(after_page_up < offset);
    }

    // --- UIFIX-04: real key-handler tests (gap closure, 14-04) -------------
    //
    // Every scroll test above calls `clamp_scroll` directly and simulates the
    // key press with hand-written arithmetic. That is exactly why the
    // up-direction defect was invisible: `clamp_scroll` was always correct,
    // and the Up/PageUp handlers never called it. The tests below drive the
    // real `handle_key` through a constructed `AppContext`.

    const TEST_ALIAS: &str = "proj";

    /// A minimal `AppContext`, mirroring the single production construction
    /// site in `app.rs`. The repository had no such fixture before 14-04.
    fn test_ctx() -> AppContext {
        use crate::config::Config;
        use ratatui::widgets::TableState;
        use std::collections::HashMap;
        use std::path::PathBuf;

        AppContext {
            config: Config::new(),
            config_path: PathBuf::from("gsd-meta-manager-test-config.json"),
            project_states: HashMap::new(),
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
        }
    }

    /// A DetailScreen and AppContext parked on the Browse file view, with the
    /// given recorded viewport metrics and stored scroll offset.
    fn browse_fixture(
        total_lines: u16,
        visible_height: u16,
        stored_offset: u16,
    ) -> (DetailScreen, AppContext) {
        use crate::browser::BrowserDepth;

        let screen = DetailScreen::new(TEST_ALIAS.to_string());
        screen.browser_viewport.set(ViewportMetrics {
            total_lines,
            visible_height,
        });

        let mut ctx = test_ctx();
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::Browse);
        let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
        cache.browser_depth = BrowserDepth::View;
        cache.browser_scroll_offset = stored_offset;
        cache.browser_file_content = Some("line\n".repeat(total_lines as usize));

        (screen, ctx)
    }

    /// The Archive equivalent of [`browse_fixture`].
    fn archive_fixture(
        total_lines: u16,
        visible_height: u16,
        stored_offset: u16,
    ) -> (DetailScreen, AppContext) {
        use crate::archive::ArchiveDepth;

        let screen = DetailScreen::new(TEST_ALIAS.to_string());
        screen.archive_viewport.set(ViewportMetrics {
            total_lines,
            visible_height,
        });

        let mut ctx = test_ctx();
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::Archive);
        let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
        cache.archive_depth = ArchiveDepth::FileView {
            milestone: "v1.0".to_string(),
            phase_idx: None,
            file_idx: 0,
        };
        cache.archive_scroll_offset = stored_offset;

        (screen, ctx)
    }

    fn browse_offset(ctx: &AppContext) -> u16 {
        ctx.view_cache[TEST_ALIAS].browser_scroll_offset
    }

    fn archive_offset(ctx: &AppContext) -> u16 {
        ctx.view_cache[TEST_ALIAS].archive_scroll_offset
    }

    fn press(screen: &mut DetailScreen, ctx: &mut AppContext, code: KeyCode) {
        screen.handle_key(code, KeyModifiers::NONE, ctx);
    }

    #[test]
    fn test_page_up_handler_clamps_stale_browse_offset() {
        // Viewport grew to 60 visible lines while the stored offset was still
        // 90; max_scroll is now 40. Clamp-then-subtract yields 40 - 20 = 20.
        let (mut screen, mut ctx) = browse_fixture(100, 60, 90);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(browse_offset(&ctx), 20);
    }

    #[test]
    fn test_page_up_handler_clamps_stale_archive_offset() {
        let (mut screen, mut ctx) = archive_fixture(100, 60, 90);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(archive_offset(&ctx), 20);
    }

    #[test]
    fn test_up_handler_clamps_stale_browse_offset() {
        // Same stale offset, single-line delta: 40 - 1 = 39.
        let (mut screen, mut ctx) = browse_fixture(100, 60, 90);
        press(&mut screen, &mut ctx, KeyCode::Up);
        assert_eq!(browse_offset(&ctx), 39);

        // `k` is the vim-key alias for the same handler arm.
        let (mut screen, mut ctx) = browse_fixture(100, 60, 90);
        press(&mut screen, &mut ctx, KeyCode::Char('k'));
        assert_eq!(browse_offset(&ctx), 39);
    }

    #[test]
    fn test_up_handler_clamps_stale_archive_offset() {
        let (mut screen, mut ctx) = archive_fixture(100, 60, 90);
        press(&mut screen, &mut ctx, KeyCode::Up);
        assert_eq!(archive_offset(&ctx), 39);

        let (mut screen, mut ctx) = archive_fixture(100, 60, 90);
        press(&mut screen, &mut ctx, KeyCode::Char('k'));
        assert_eq!(archive_offset(&ctx), 39);
    }

    // ── The Driver output pane: the same invariant in its newest place ─────
    //
    // Every test below drives the real `handle_key`, following the lesson
    // recorded above: `clamp_scroll` was always correct and the up-direction
    // handlers simply never called it, so a test that computes the arithmetic
    // itself cannot see the defect it is meant to guard against.

    /// A DetailScreen and AppContext parked on the Driver tab, with the given
    /// recorded output-pane metrics, stored offset and follow bit.
    fn driver_fixture(
        total_lines: u16,
        visible_height: u16,
        stored_offset: u16,
        following: bool,
    ) -> (DetailScreen, AppContext) {
        let screen = DetailScreen::new(TEST_ALIAS.to_string());
        screen.driver_viewport.set(ViewportMetrics {
            total_lines,
            visible_height,
        });

        let mut ctx = test_ctx();
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::Driver);
        let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
        cache.driver_scroll_offset = stored_offset;
        cache.driver_follow = following;

        (screen, ctx)
    }

    fn driver_offset(ctx: &AppContext) -> u16 {
        ctx.view_cache[TEST_ALIAS].driver_scroll_offset
    }

    fn driver_following(ctx: &AppContext) -> bool {
        ctx.view_cache[TEST_ALIAS].driver_follow
    }

    fn test_run(run_id: &str) -> crate::journal::RunSummary {
        crate::journal::RunSummary {
            run_id: run_id.to_string(),
            started_at: "2026-07-29T21:40:00Z".to_string(),
            ended_at: None,
            goal: String::new(),
            gsd_command: "/gsd:progress".to_string(),
            outcome: None,
        }
    }

    #[test]
    fn driver_page_down_from_the_tail_clamps_and_does_not_overshoot() {
        // 100 lines in a 30-row body → max_scroll = 70.
        let (mut screen, mut ctx) = driver_fixture(100, 30, 70, false);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(driver_offset(&ctx), 70);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(driver_offset(&ctx), 70, "repeated PageDown must be idempotent");
    }

    #[test]
    fn driver_page_up_from_zero_stays_at_zero() {
        let (mut screen, mut ctx) = driver_fixture(100, 30, 0, false);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(driver_offset(&ctx), 0);

        // And the pre-first-render floor: both metrics still zero, no underflow.
        let (mut screen, mut ctx) = driver_fixture(0, 0, 0, false);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(driver_offset(&ctx), 0);
    }

    #[test]
    fn driver_page_up_clamps_first_and_lands_below_max_scroll() {
        // The buffer grew and the body is 60 rows: max_scroll is 40 while the
        // stored offset is still 90. Clamp-then-subtract yields 20.
        // Subtract-then-clamp would yield exactly 40 — the value the renderer
        // was already displaying — so the first press would not visibly move the
        // viewport. That is UIFIX-04, in the pane this plan adds.
        let max_scroll = 100u16 - 60;
        let (mut screen, mut ctx) = driver_fixture(100, 60, 90, false);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(driver_offset(&ctx), 20);
        assert!(driver_offset(&ctx) < max_scroll);
    }

    #[test]
    fn an_upward_scroll_clears_the_driver_follow_bit() {
        // Following, so the stored offset is stale by design: the press has to
        // start from the tail (70) rather than from the parked 0.
        let (mut screen, mut ctx) = driver_fixture(100, 30, 0, true);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(driver_offset(&ctx), 50);
        assert!(
            !driver_following(&ctx),
            "any upward scroll must clear the follow bit, with no key of its own"
        );
    }

    #[test]
    fn page_down_reaching_the_tail_re_arms_driver_follow() {
        let (mut screen, mut ctx) = driver_fixture(100, 30, 55, false);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(driver_offset(&ctx), 70);
        assert!(
            driver_following(&ctx),
            "reaching the bottom IS the request to follow"
        );

        // A PageDown that does NOT reach the tail leaves the bit alone.
        let (mut screen, mut ctx) = driver_fixture(100, 30, 0, false);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(driver_offset(&ctx), 20);
        assert!(!driver_following(&ctx));
    }

    #[test]
    fn f_toggles_the_driver_follow_bit() {
        let (mut screen, mut ctx) = driver_fixture(100, 30, 10, false);
        press(&mut screen, &mut ctx, KeyCode::Char('f'));
        assert!(driver_following(&ctx));

        // Turning it off leaves the viewport where the pane was rendering it —
        // the tail — rather than snapping back to a stale stored number.
        press(&mut screen, &mut ctx, KeyCode::Char('f'));
        assert!(!driver_following(&ctx));
        assert_eq!(driver_offset(&ctx), 70);
    }

    #[test]
    fn g_jumps_to_the_driver_tail_and_re_enables_follow() {
        let (mut screen, mut ctx) = driver_fixture(100, 30, 0, false);
        press(&mut screen, &mut ctx, KeyCode::Char('G'));
        assert_eq!(driver_offset(&ctx), 70);
        assert!(driver_following(&ctx));
    }

    #[test]
    fn changing_the_selected_run_resets_the_offset_and_the_follow_bit() {
        let (mut screen, mut ctx) = driver_fixture(100, 30, 45, false);
        {
            let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
            cache.driver_runs = vec![test_run("2026-07-29T21-40-00Z-3f2a"), test_run("2026-07-28T09-00-00Z-aa11")];
        }

        press(&mut screen, &mut ctx, KeyCode::Char('j'));
        assert_eq!(ctx.view_cache[TEST_ALIAS].driver_selected_run, 1);
        assert_eq!(driver_offset(&ctx), 0);
        assert!(
            driver_following(&ctx),
            "a different run is a different journal: the pane goes back to its tail"
        );

        // And the selection clamps at both ends of the list.
        press(&mut screen, &mut ctx, KeyCode::Char('j'));
        assert_eq!(ctx.view_cache[TEST_ALIAS].driver_selected_run, 1);
        press(&mut screen, &mut ctx, KeyCode::Char('k'));
        press(&mut screen, &mut ctx, KeyCode::Char('k'));
        assert_eq!(ctx.view_cache[TEST_ALIAS].driver_selected_run, 0);
    }

    #[test]
    fn test_first_page_up_after_viewport_grows_moves_viewport() {
        // The UI-SPEC contract row: the FIRST PageUp press must visibly move
        // the viewport. This is the test that distinguishes clamp-then-subtract
        // from subtract-then-clamp — the latter would land exactly on
        // max_scroll (40), which the renderer was already displaying, so the
        // strict inequality below would fail while the offset still "changed".
        let max_scroll = 100u16 - 60;
        let (mut screen, mut ctx) = browse_fixture(100, 60, 90);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert!(
            browse_offset(&ctx) < max_scroll,
            "first PageUp left the offset at {}, which is not below max_scroll {}",
            browse_offset(&ctx),
            max_scroll
        );

        // And when the offset already sits exactly on max_scroll, the first
        // PageUp still yields a strictly smaller offset.
        let (mut screen, mut ctx) = browse_fixture(100, 60, max_scroll);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert!(browse_offset(&ctx) < max_scroll);
    }

    #[test]
    fn test_page_up_at_zero_offset_is_inert() {
        // Zero-line document, and the pre-first-render state where both
        // recorded metrics are still zero: Up and PageUp are no-ops at the
        // zero floor, with no panic and no underflow.
        for code in [KeyCode::PageUp, KeyCode::Up] {
            let (mut screen, mut ctx) = browse_fixture(0, 0, 0);
            press(&mut screen, &mut ctx, code);
            assert_eq!(browse_offset(&ctx), 0);

            let (mut screen, mut ctx) = archive_fixture(0, 0, 0);
            press(&mut screen, &mut ctx, code);
            assert_eq!(archive_offset(&ctx), 0);
        }

        // A document shorter than the viewport never scrolls: max_scroll is 0.
        let (mut screen, mut ctx) = browse_fixture(30, 30, 0);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(browse_offset(&ctx), 0);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(browse_offset(&ctx), 0);

        // While the file content is still None (the Loading window), the
        // up-direction handlers stay inert at the floor.
        let (mut screen, mut ctx) = browse_fixture(100, 60, 0);
        ctx.view_cache
            .entry(TEST_ALIAS.to_string())
            .or_default()
            .browser_file_content = None;
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(browse_offset(&ctx), 0);
    }

    // ── The Driver tab's three action keys (STEER-01, OBS-05) ──────────────
    //
    // All three are tab-scoped and all three go through the screens 18-07
    // landed rather than doing anything themselves. The `i` pair is the one
    // that matters most: its refusal has to be visible **and** silent on the
    // wire, and only an assertion on the receiver can see the second half.

    /// The Driver tab with a run list, an event channel and no observed run.
    fn driver_action_fixture() -> (
        DetailScreen,
        AppContext,
        tokio::sync::mpsc::UnboundedReceiver<crate::action::Action>,
    ) {
        let (mut screen, mut ctx) = driver_fixture(100, 30, 0, true);
        let _ = &mut screen;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        ctx.event_tx = Some(tx);
        let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
        cache.driver_runs = vec![test_run("2026-07-29T21-40-00Z-3f2a")];
        (screen, ctx, rx)
    }

    /// The name of the screen a [`ScreenAction`] pushes, if it pushes one.
    ///
    /// `ScreenAction` has no `Debug` impl and a `Box<dyn Screen>` has nothing
    /// else to assert on, so the name is the identity — which is what
    /// `DetailScreen::NAME` already exists to make reliable.
    fn pushed_screen_name(action: &ScreenAction) -> Option<String> {
        match action {
            ScreenAction::Push(pushed) => Some(pushed.name().to_string()),
            _ => None,
        }
    }

    fn observed_live_run(run_id: &str) -> crate::driver::reconcile::ObservedRun {
        crate::driver::reconcile::ObservedRun {
            alias: TEST_ALIAS.to_string(),
            run_id: run_id.to_string(),
            pid: 4321,
            pgid: 4321,
            started_at: "2026-07-29T21:40:00Z".to_string(),
            goal: "ship the driver tab".to_string(),
            gsd_command: "/gsd:execute-phase 18".to_string(),
            liveness: crate::driver::liveness::Liveness::Alive,
        }
    }

    /// CTRL-03 at the affordance layer, in the shape `driver_confirm.rs`
    /// records: the **second** assertion is the load-bearing one. A binding that
    /// set the message and opened the screen anyway would pass a test that
    /// checked only the message, and the user would be typing into an input
    /// aimed at a run that is not there.
    #[test]
    fn pressing_i_with_no_live_run_sets_a_message_and_dispatches_nothing() {
        let (mut screen, mut ctx, mut rx) = driver_action_fixture();
        assert!(ctx.observed_runs.is_empty(), "the fixture has no live run");

        let action = screen.handle_key(KeyCode::Char('i'), KeyModifiers::NONE, &mut ctx);
        assert!(
            matches!(action, ScreenAction::None),
            "the injection screen must not open"
        );

        let message = ctx
            .status_message
            .as_ref()
            .map(|(text, _)| text.clone())
            .expect("the refusal must be visible, not silent");
        assert_eq!(
            message,
            crate::ui::screens::driver_inject::no_live_run_message(TEST_ALIAS),
            "the copy lives in one place so the screen and its opener cannot drift"
        );
        assert!(
            rx.try_recv().is_err(),
            "the refusal must dispatch nothing at all"
        );

        // A run that is merely *observed* is not enough: an undeterminable
        // liveness answer is not a live run (CR-05's narrow reading).
        let mut unknown = observed_live_run("2026-07-29T21-40-00Z-3f2a");
        unknown.liveness = crate::driver::liveness::Liveness::Unknown;
        ctx.observed_runs.insert(TEST_ALIAS.to_string(), unknown);
        ctx.status_message = None;
        let action = screen.handle_key(KeyCode::Char('i'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::None));
        assert!(ctx.status_message.is_some());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn pressing_i_with_a_live_run_pushes_the_injection_screen() {
        let (mut screen, mut ctx, _rx) = driver_action_fixture();
        ctx.observed_runs.insert(
            TEST_ALIAS.to_string(),
            observed_live_run("2026-07-29T21-40-00Z-3f2a"),
        );

        let action = screen.handle_key(KeyCode::Char('i'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(pushed_screen_name(&action).as_deref(), Some("driver_inject"));
        assert!(
            ctx.status_message.is_none(),
            "a run that IS live gets no refusal"
        );
    }

    /// WR-01: the target of `i` is the run the user is **looking at**.
    ///
    /// `j`/`k` move the selection freely across the whole run history while
    /// `observed_runs` holds only the run this session is tailing. Reading the
    /// id out of that map alone aimed the input at today's live run while the
    /// user was reviewing last week's — and because `cache.driver_inbox` is the
    /// **selected** run's inbox, the queued message then appeared nowhere on the
    /// surface they were looking at. Two facts silently disagreeing, with the
    /// user's own steering intent as the payload.
    #[test]
    fn pressing_i_while_reviewing_a_different_run_refuses_and_says_which_condition() {
        const LIVE: &str = "2026-07-29T21-40-00Z-3f2a";
        const OLD: &str = "2026-07-20T09-00-00Z-beef";

        let (mut screen, mut ctx, mut rx) = driver_action_fixture();
        ctx.observed_runs
            .insert(TEST_ALIAS.to_string(), observed_live_run(LIVE));
        // The pane is showing an older run — the state `j` leaves behind.
        let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
        cache.driver_runs = vec![test_run(OLD), test_run(LIVE)];
        cache.driver_selected_run = 0;

        let action = screen.handle_key(KeyCode::Char('i'), KeyModifiers::NONE, &mut ctx);
        assert!(
            matches!(action, ScreenAction::None),
            "the input must not open aimed at a run the pane is not showing"
        );
        let message = ctx
            .status_message
            .as_ref()
            .map(|(text, _)| text.clone())
            .expect("the refusal must be visible, not silent");
        assert_eq!(
            message,
            crate::ui::screens::driver_inject::not_the_selected_run_message(TEST_ALIAS),
            "and it must name THIS condition: a live run exists, so saying \
             \"no live run\" over a run list that visibly contains one would \
             read as a bug in the tool"
        );
        assert!(rx.try_recv().is_err(), "the refusal dispatches nothing");

        // Selecting the live run is what makes `i` work again.
        let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
        cache.driver_selected_run = 1;
        ctx.status_message = None;
        let action = screen.handle_key(KeyCode::Char('i'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(pushed_screen_name(&action).as_deref(), Some("driver_inject"));
        assert!(ctx.status_message.is_none());
    }

    #[test]
    fn pressing_s_on_the_driver_tab_pushes_the_start_flow() {
        let (mut screen, mut ctx, _rx) = driver_action_fixture();
        let action = screen.handle_key(KeyCode::Char('s'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(pushed_screen_name(&action).as_deref(), Some("driver_start"));
    }

    /// `x` stops **through the existing confirmation** and never directly: the
    /// whole process tree is torn down, and that is not a thing a single
    /// keystroke may do.
    #[test]
    fn pressing_x_on_the_driver_tab_pushes_the_stop_confirmation() {
        let (mut screen, mut ctx, _rx) = driver_action_fixture();
        let action = screen.handle_key(KeyCode::Char('x'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(pushed_screen_name(&action).as_deref(), Some("driver_confirm"));
    }

    /// The Driver tab with a **registered** project carrying no opt-in record.
    ///
    /// It deliberately does not reuse [`driver_action_fixture`]. That fixture's
    /// `test_ctx()` registers no project at all and points `config_path` at a
    /// *relative* path, so a real toggle would fail `UnknownAlias` — and if it
    /// somehow did not, it would write a config file into the repository
    /// working directory. `ctx_with_project` registers `ALIAS` — the same
    /// string as [`TEST_ALIAS`], which is what lets it drive a `DetailScreen` —
    /// with `driver_opt_in: None` and a `config_path` inside a tempdir, which
    /// is exactly the starting state a real opt-in needs.
    ///
    /// The receiver is returned so the sender in `ctx.event_tx` stays live for
    /// the duration of the test.
    fn driver_optin_fixture(
        root: &std::path::Path,
    ) -> (
        DetailScreen,
        AppContext,
        tokio::sync::mpsc::UnboundedReceiver<crate::action::Action>,
    ) {
        let screen = DetailScreen::new(TEST_ALIAS.to_string());
        let (mut ctx, rx) = super::super::driver_confirm::tests::ctx_with_project(root);
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::Driver);
        (screen, ctx, rx)
    }

    /// CTRL-03 end to end: the affordance moved, the gate did not.
    ///
    /// The pair that carries the proof is the **first** and **last**
    /// assertions — the seam refusing, then the seam accepting — with nothing
    /// between them but the keypress and the confirmation. `name()` cannot do
    /// this on its own: `DriverConfirmScreen` returns `"driver_confirm"` for
    /// every `DriverAction`, so a name assertion cannot tell a `Stop` from a
    /// `ToggleOptIn`. The effect has to be observed.
    #[test]
    fn opting_in_from_the_driver_tab_flips_the_spawn_seam() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut screen, mut ctx, _rx) = driver_optin_fixture(dir.path());

        assert!(
            matches!(
                crate::executor::DrivableProject::from_registry(
                    TEST_ALIAS,
                    &ctx.config.projects[TEST_ALIAS],
                ),
                Err(crate::error::OptInError::NotOptedIn { .. })
            ),
            "the fixture must start REFUSED by the spawn seam, or the rest of \
             this test proves nothing"
        );

        let action = screen.handle_key(KeyCode::Char('o'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(pushed_screen_name(&action).as_deref(), Some("driver_confirm"));

        assert!(
            !crate::registry::is_opted_in(&ctx.config, TEST_ALIAS),
            "the keypress alone must change nothing: opening the tab and \
             reaching for the key is not consent, the confirmation is. A \
             binding that opted in and *then* asked would satisfy every other \
             assertion in this test."
        );

        let ScreenAction::Push(mut confirm) = action else {
            panic!("the `o` arm must push")
        };
        confirm.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);

        let saved = crate::config::load_config(&ctx.config_path)
            .expect("the confirmation must have written config.json");
        assert!(
            saved.projects[TEST_ALIAS].driver_opt_in.is_some(),
            "an opt-in that never reached disk is invisible to the driver, \
             which is a different process reading config.json"
        );

        assert!(
            crate::executor::DrivableProject::from_registry(
                TEST_ALIAS,
                &saved.projects[TEST_ALIAS],
            )
            .is_ok(),
            "read off the RELOADED entry, so this is about the bytes on disk \
             and not about in-memory state; the registered path is the tempdir \
             root, a real directory, so `RootUnusable` cannot fire and mask it"
        );
    }

    /// The driver action keys are **tab-scoped**, so the tabs that already
    /// claim `s`, `i` or `x` keep them. `x` on the Defaults tab clears a value
    /// and `x` on the Queue tab deletes an entry; neither may start reaching
    /// for the driver.
    ///
    /// `o` is in this list for the opposite reason: unlike `i`/`s`/`x` it had
    /// no collision to resolve — nothing else on any detail tab binds it — and
    /// it is scoped anyway, so the guard is doing real work rather than dodging
    /// a conflict. This test is what turns that guard from a comment into an
    /// enforced property: without it, a future refactor that drops the
    /// `if current_view == DetailSubView::Driver` clause compiles clean and
    /// silently makes `o` a global opt-in on every detail tab.
    #[test]
    fn the_driver_action_keys_are_scoped_to_the_driver_tab() {
        for tab in [
            DetailSubView::PhaseList,
            DetailSubView::Pipeline,
            DetailSubView::Defaults,
        ] {
            for code in [
                KeyCode::Char('i'),
                KeyCode::Char('s'),
                KeyCode::Char('x'),
                KeyCode::Char('o'),
            ] {
                let (mut screen, mut ctx, _rx) = driver_action_fixture();
                ctx.detail_sub_view_per_project
                    .insert(TEST_ALIAS.to_string(), tab.clone());
                ctx.observed_runs.insert(
                    TEST_ALIAS.to_string(),
                    observed_live_run("2026-07-29T21-40-00Z-3f2a"),
                );
                let action = screen.handle_key(code, KeyModifiers::NONE, &mut ctx);
                let pushed = pushed_screen_name(&action);
                assert!(
                    !pushed
                        .as_deref()
                        .is_some_and(|name| name.starts_with("driver_")),
                    "{code:?} reached {pushed:?} from {tab:?}"
                );
            }
        }
    }

    // --- CD-03 / IN-07 closure: the generic `_ =>` fallback ---------------
    //
    // Reached by exactly two sub-views — PhaseList and RoadmapViz. Every other
    // sub-view has an explicit match arm. See plan 14-04 decision GD-01, which
    // corrects CD-03's "seven non-file tabs" cost estimate.

    /// A DetailScreen and AppContext parked on a tab that reaches the generic
    /// `_ =>` scroll fallback, with recorded metrics and a stored offset.
    fn generic_fixture(
        total_lines: u16,
        visible_height: u16,
        stored_offset: u16,
    ) -> (DetailScreen, AppContext) {
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
        screen.generic_viewport.set(ViewportMetrics {
            total_lines,
            visible_height,
        });
        screen.scroll_offset = stored_offset;

        let mut ctx = test_ctx();
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::PhaseList);

        (screen, ctx)
    }

    #[test]
    fn test_generic_page_up_clamps_stale_offset() {
        // Same defect and same fix shape as the file views: max_scroll is 40,
        // so clamp-then-subtract yields 40 - 20 = 20.
        let (mut screen, mut ctx) = generic_fixture(100, 60, 90);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(screen.scroll_offset, 20);
        assert!(screen.scroll_offset < 100 - 60);

        // The Roadmap tab reaches the same arm.
        let (mut screen, mut ctx) = generic_fixture(100, 60, 90);
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::RoadmapViz);
        press(&mut screen, &mut ctx, KeyCode::Up);
        assert_eq!(screen.scroll_offset, 39);
    }

    #[test]
    fn test_generic_page_down_clamps_at_content_end() {
        // PageDown stops at the content end and stays there.
        let (mut screen, mut ctx) = generic_fixture(100, 60, 30);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(screen.scroll_offset, 40);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(screen.scroll_offset, 40);

        // Down is clamped at the same bound.
        press(&mut screen, &mut ctx, KeyCode::Down);
        assert_eq!(screen.scroll_offset, 40);

        // A document shorter than the viewport never scrolls at all.
        let (mut screen, mut ctx) = generic_fixture(30, 30, 0);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(screen.scroll_offset, 0);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(screen.scroll_offset, 0);
    }

    // --- T-21-25-05: the session-id truncation is a CHAR operation ---------
    //
    // `render_sessions_tab` shortened a session id with `sid[..8]`, which
    // indexes BYTES. A session id is scraped from a `claude` process's
    // `--resume` argument (`session_detector::read_session_id`), so nothing
    // about it was authored here and nothing constrains it to ASCII. Any
    // non-ASCII id longer than eight bytes whose eighth byte is inside a
    // multi-byte character panics the render, and a panic in a render pass
    // takes the whole TUI down.
    //
    // This is the SAME defect class as the `&s[..n]` panic round 8 fixed in
    // `roadmap_widget.rs:138-146` — a family, not an incident.

    /// A `DetailScreen` and `AppContext` parked on the Sessions tab with one
    /// active session carrying `session_id`.
    ///
    /// The session's `working_dir` must equal the registered project's path or
    /// `render_sessions_tab`'s filter drops it and the tab renders its EMPTY
    /// branch — which would make every assertion below pass by silence.
    fn sessions_fixture(session_id: &str) -> (DetailScreen, AppContext) {
        use crate::config::RegisteredProject;
        use crate::session_detector::ClaudeSession;
        use std::path::PathBuf;

        let project_path = PathBuf::from("/nonexistent").join(TEST_ALIAS);
        let mut ctx = test_ctx();
        ctx.config.projects.insert(
            TEST_ALIAS.to_string(),
            RegisteredProject {
                path: project_path.clone(),
                added: "2026-08-27".to_string(),
                driver_opt_in: None,
                extra: Default::default(),
            },
        );
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::Sessions);
        ctx.active_sessions = vec![ClaudeSession {
            pid: 4242,
            session_id: Some(Untrusted::from_untrusted_source(session_id.to_string())),
            working_dir: project_path,
            start_time: Some(1),
            tty: None,
        }];

        (DetailScreen::new(TEST_ALIAS.to_string()), ctx)
    }

    /// Render the detail screen into a `TestBackend` and join the cells.
    fn render_detail_to_text(screen: &DetailScreen, ctx: &AppContext) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let (width, height) = (120u16, 30u16);
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("TestBackend terminal");
        terminal
            .draw(|frame| screen.render(frame, frame.area(), ctx))
            .expect("draw the detail screen");
        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .map(|cell| cell.symbol())
                            .unwrap_or(" ")
                            .to_string()
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A three-byte-per-character id whose glyphs are ONE cell wide.
    ///
    /// `U+0915` DEVANAGARI LETTER KA is three bytes and `unicode-width` 1, so
    /// five of them are fifteen bytes with a character boundary nowhere near
    /// byte eight — exactly the shape the byte slice got wrong. **The width
    /// matters as much as the byte count**: a CJK id was the first fixture
    /// here, and CJK glyphs are TWO cells wide, so the probe's cell-by-cell
    /// buffer scrape produced `KA<blank>KA<blank>…` and the assertion failed
    /// for a reason that had nothing to do with the truncation. A one-cell
    /// character makes the scraped text the id itself.
    const NARROW_MULTIBYTE_CHAR: &str = "\u{915}";

    /// The panic reproduction: a five-character three-byte-per-character id is
    /// fifteen BYTES, so `len() > 8` is true and byte index 8 lands inside the
    /// third character.
    ///
    /// **Committed RED, verbatim, before the truncation was rewritten**
    /// (`cargo test --lib -- ui::screens::detail::tests::a_multibyte_session_id_does_not_panic_the_render --exact --nocapture`,
    /// re-observed against this fixture by restoring `sid[..8]` at
    /// `shorten_session_id` and nothing else):
    ///
    /// ```text
    /// thread 'ui::screens::detail::tests::a_multibyte_session_id_does_not_panic_the_render' (658292) panicked at src/ui/screens/detail.rs:117:12:
    /// end byte index 8 is not a char boundary; it is inside '\u{915}' (bytes 6..9 of string)
    /// ```
    ///
    /// The first RED, against the ORIGINAL `render_sessions_tab` byte slice
    /// before any of this plan's edits (commit `235c3cc`, a CJK fixture):
    ///
    /// ```text
    /// thread 'ui::screens::detail::tests::a_multibyte_session_id_does_not_panic_the_render' (594330) panicked at src/ui/screens/detail.rs:3384:32:
    /// end byte index 8 is not a char boundary; it is inside '\u{4e2d}' (bytes 6..9 of string)
    /// ```
    ///
    /// (The panic text names the character with its literal glyph; it is
    /// written here as its `\u{...}` escape, following the house rule that no
    /// raw glyph appears in source.) The sibling
    /// `a_long_multibyte_session_id_is_truncated_by_characters_not_bytes`
    /// panicked identically at the same line in the same run.
    #[test]
    fn a_multibyte_session_id_does_not_panic_the_render() {
        let id = NARROW_MULTIBYTE_CHAR.repeat(5);
        let id = id.as_str();
        assert!(
            id.len() > 8 && id.chars().count() <= 8,
            "the fixture must be longer than eight BYTES and no longer than \
             eight CHARACTERS, or it does not exercise the boundary the byte \
             slice got wrong"
        );

        let (screen, ctx) = sessions_fixture(id);
        let text = render_detail_to_text(&screen, &ctx);

        assert!(
            text.contains("PID 4242"),
            "the Sessions tab rendered its EMPTY branch, so this test proved \
             nothing about the truncation. The session's working_dir must match \
             the registered project's path. Rendered:\n{text}"
        );
        assert!(
            text.contains(id),
            "a session id of {} characters is at or below the eight-character \
             cap and must render in full, not as a byte-sliced fragment. \
             Rendered:\n{text}",
            id.chars().count()
        );
    }

    /// The truncation itself, as a CHAR operation: a nine-character multibyte
    /// id renders as its first EIGHT characters.
    ///
    /// A byte slice cannot produce this answer even where it does not panic —
    /// `[..8]` of a three-byte-per-character string is at most two whole
    /// characters — so this pins the semantics and not merely the absence of a
    /// crash.
    #[test]
    fn a_long_multibyte_session_id_is_truncated_by_characters_not_bytes() {
        let id: String = NARROW_MULTIBYTE_CHAR.repeat(9);
        let expected: String = id.chars().take(8).collect();

        let (screen, ctx) = sessions_fixture(&id);
        let text = render_detail_to_text(&screen, &ctx);

        assert!(
            text.contains("PID 4242"),
            "the Sessions tab rendered its EMPTY branch, so this test proved \
             nothing. Rendered:\n{text}"
        );
        assert!(
            text.contains(&expected),
            "a nine-character id must be shortened to its first eight \
             CHARACTERS ({expected:?}), not to a prefix measured in bytes. \
             Rendered:\n{text}"
        );
        assert!(
            !text.contains(&id),
            "the id was not shortened at all — the cap is nine characters wide \
             here, so the eight-character branch was never taken and the \
             assertion above passed for the wrong reason"
        );
    }
}
