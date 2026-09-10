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
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs, Wrap};
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

/// Terminal emulators probed by NAME, as a **preference order of last resort**.
///
/// This list is the LOWEST rank of the launch decision (D-02), and that demotion
/// is the fix for the bug it caused. Being order-dependent is precisely how the
/// reported defect happened: on a machine whose real default terminal is
/// `ptyxis`, `gnome-terminal` was picked purely because it sits earlier here.
/// The answer to that is the discovery ranks ABOVE this list —
/// `xdg-terminal-exec`, which knows what the desktop actually designates — not
/// surgery on the order of these five names.
///
/// | entry | note |
/// |---|---|
/// | `kitty`, `alacritty`, `xterm` | one process per window; `current_dir` is honoured |
/// | `ptyxis` | **Residual (D-04):** bare `ptyxis` may open a TAB in an existing instance rather than a new window, and a single-instance GUI application may not honour the `current_dir` of the client process. Documented rather than fixed: `gnome-terminal` below is already the same class (it is backed by `gnome-terminal-server`), so this is a pre-existing property of this list rather than something `ptyxis` introduces. It is also reachable only when `xdg-terminal-exec` is absent — which is rare, since `xdg-terminal-exec` is how `ptyxis` gets designated as the default in the first place. |
/// | `gnome-terminal` | same single-instance class; needs the `--` separator, not `-e` |
///
/// Every name here must have a row in [`terminal_program_separator`]'s table;
/// `tests::the_separator_table_covers_every_launcher_the_discovery_consts_name`
/// parses this const out of the source text and fails if one does not.
const FALLBACK_TERMINAL_CANDIDATES: &[&str] =
    &["kitty", "alacritty", "ptyxis", "gnome-terminal", "xterm"];

/// The launchers that RESOLVE a terminal rather than being one, in rank order
/// (D-02). They outrank [`FALLBACK_TERMINAL_CANDIDATES`] because they answer
/// "which terminal does this desktop designate?" instead of "which terminal is
/// installed?".
///
/// They still receive the resumed program's argv, so each needs a separator
/// decision exactly as an emulator does — which is why they are parsed by the
/// same source-derived guard.
///
/// # Not read by the runtime, and pinned so that cannot rot
///
/// The names themselves live in [`crate::terminal_switch::plan_launch`], which
/// returns them, and in `probe_terminals`, which probes for them. This const is
/// the REGISTRY the separator guard adjudicates against — the same role
/// `session_detector`'s `CLAUDE_ARGV_SITES` plays for the option-literal
/// census. A registry the runtime does not read can drift from the runtime, so
/// `tests::the_discovery_consts_name_every_launcher_the_plan_can_actually_return`
/// drives the real `plan_launch` and fails if it ever answers with a name that
/// is not listed here.
#[allow(dead_code)]
const DISCOVERY_LAUNCHERS: &[&str] = &["xdg-terminal-exec", "x-terminal-emulator"];

/// The launch decision for this build: `plan_launch` over the probes, with this
/// file's own fallback list.
///
/// **Replaces `find_terminal`**, which tried `$TERMINAL` and then the hardcoded
/// candidate list and never looked at `$TMUX` at all. The ordering now lives in
/// [`crate::terminal_switch::plan_launch`], where it is a pure function of
/// injected probes and can be asserted for environments nobody is running in —
/// including the exact one the bug was reported from.
fn resolve_launch_plan() -> crate::terminal_switch::LaunchPlan {
    crate::terminal_switch::plan_launch(&crate::terminal_switch::probe_terminals(
        FALLBACK_TERMINAL_CANDIDATES,
    ))
}

/// The token that separates a terminal emulator's OWN options from the program
/// it is being asked to run.
///
/// **This function exists because the security fix would otherwise delete the
/// feature** (T-21-27-06). While the resume was one opaque program string
/// handed to an interpreter, the emulator never saw the resumed program's own
/// options — the interpreter did. Handing the emulator a real argv makes those
/// options visible to it for the first time, and the separator is not the same
/// token for every launcher [`resolve_launch_plan`] can return:
///
/// | the plan's `gui` is | Separator | Why |
/// |---|---|---|
/// | `kitty` | `-e` | takes the program and its arguments after `-e` |
/// | `alacritty` | `-e` | `-e`/`--command` consumes the remainder |
/// | `gnome-terminal` | `--` | its `-e` is deprecated and takes a SINGLE string it re-parses; with a real argv it consumes `--resume` as one of its OWN options |
/// | `xterm` | `-e` | `-e` consumes the remainder |
/// | `ptyxis` | `--` | `ptyxis --help`: `Usage: ptyxis [OPTION…] [-- COMMAND ARGUMENTS]`. **NOT `-e`.** Ptyxis does not follow the `-e` convention at all: its `-x` takes a SINGLE re-parsed string — the same trap that made `gnome-terminal`'s deprecated `-e` wrong. Getting this row wrong deletes the resume SILENTLY rather than loudly. |
/// | `xdg-terminal-exec` | `--` | usage line `xdg-terminal-exec [options] [--] [command [arguments ...]]`; confirmed by measuring `--print-cmd -- claude --resume=abc123`, which printed `ptyxis` / `--new-window` / `--` / `claude` / `--resume=abc123` — landing on the desktop's real default AND preserving the fused element intact |
/// | `x-terminal-emulator` | `-e` | the interface spec in the POD of `/usr/bin/gnome-terminal.wrapper` (lines 139-143): `-e COMMAND [ARGUMENTS...]`, "Equivalent to `xterm -e` COMMAND ARGUMENTS", and it "stops parsing options after -e" |
/// | anything else (`$TERMINAL`) | `-e` | the xterm-compatible convention, which is what an unknown emulator most likely follows |
///
/// Getting `gnome-terminal` wrong does not fail loudly: the emulator swallows
/// `--resume` as an unrecognised option of its own and the operator sees a
/// terminal that did not resume anything. That is a feature deletion wearing a
/// security fix's clothes, which is exactly what this table is here to stop.
///
/// The match is on the FILE NAME, so `$TERMINAL=/usr/bin/gnome-terminal` is
/// recognised rather than falling through to the default.
///
/// **Its residual, with the direction.** An emulator that is not in this table
/// and does not follow the `-e` convention gets the wrong separator.
/// **Under-detection of unsupported emulators, and it is LOUD rather than
/// silent** — the emulator rejects the flag and the operator sees the failure —
/// which is the direction prohibition 3 asks for: an emulator that cannot be
/// driven by an argv is reported, not silently handled by quoting.
fn terminal_program_separator(term: &str) -> &'static str {
    let stem = term.rsplit('/').next().unwrap_or(term);
    match stem {
        "gnome-terminal" => "--",
        "ptyxis" => "--",
        "xdg-terminal-exec" => "--",
        "x-terminal-emulator" => "-e",
        _ => "-e",
    }
}

/// The resume long option **fused to its value separator** — the single
/// spelling of the prefix [`resume_terminal_argv`] builds its untrusted element
/// from, so the option name and the `=` that binds its value cannot drift apart
/// between the builder and the controls that check it (T-21-31-01).
///
/// The trailing `=` is load-bearing, not cosmetic: without it the id becomes a
/// separate argv element and `-r, --resume [value]`'s OPTIONAL value lets an
/// id beginning with `-` be read as a new option (CWE-88).
const RESUME_OPTION_FUSED_PREFIX: &str = "--resume=";

/// The argv for resuming a Claude session, with **no command interpreter in
/// it** (CR-01, T-21-27-01, T-21-27-02).
///
/// # CORRECTED 2026-08-27 (21-27): the round-9 comment at the spawn site called this value an argv element, and it was not one
///
/// **The sentence this corrects, verbatim:** *"A SUBPROCESS ARGUMENT: the raw
/// id is what `claude --resume` must receive, and an escaped one would resume
/// nothing."*
///
/// Its second clause was true and its first was false, and the false half is
/// the dangerous one. The value was NOT a subprocess argument. It was
/// interpolated into `"cd '{}' && claude --resume '{}'"`, a program handed to a
/// command interpreter through `-c` — so it was a *fragment of a program a
/// parser reads*, and a single quote in it closed the quoting and ran arbitrary
/// code with the operator's privileges. The session id is scraped verbatim from
/// another process's `/proc/<pid>/cmdline`
/// ([`crate::session_detector`]), so nothing about it was authored here and no
/// privilege is needed to plant one. A comment that misclassifies a sink is
/// worse than no comment: it is what tells the next reader not to look, and
/// round 9's own diff touched these lines.
///
/// # Which of the three sink kinds this site is now
///
/// Every element of this vector is an **argv element**: the kernel hands it to
/// `execve` unparsed. It is not a lookup and it is not a program fragment.
/// **The third kind no longer exists here** because there is no interpreter
/// left in the path to parse anything — that is why a quote, a semicolon, a
/// backtick, a dollar-parenthesis or a newline in the session id is now data.
/// Deleting the interpreter is strictly stronger than quoting for it: quoting
/// keeps a parser in the path and makes correctness depend on the escaper being
/// right about every metacharacter of every interpreter an operator's
/// `$TERMINAL` might name.
///
/// The working directory does NOT travel in this vector. It is set through
/// [`std::process::Command::current_dir`] at the call site, which the process
/// API passes to the child directly rather than as a `cd` written into a
/// program.
///
/// Proven at the argv rather than at the spawn — a test cannot start a terminal
/// emulator in CI — by
/// `tests::the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element`,
/// observed RED against the construction this replaces.
///
/// # CORRECTED 2026-08-27 (21-31): the paragraph above says no parser remained in the path, and one did (CWE-88)
///
/// **The sentence this corrects, verbatim:** *"**The third kind no longer
/// exists here** because there is no interpreter left in the path to parse
/// anything."*
///
/// Its second clause is what made the first one false, and the failure is
/// worth naming precisely rather than patching over. Round 10 deleted the
/// command interpreter, and that was real: no `-c` program string survives
/// here, so CR-01 is genuinely closed. But "no interpreter" is not "no
/// parser". **`claude`'s own option parser was in the path the whole time.**
///
/// The distinction the original sentence collapsed: `execve` hands argv
/// elements to the child **unparsed** — but the PROGRAM `execve` starts parses
/// them itself, and that is the entire purpose of an argv. Deleting the shell
/// removed one parser from the path. It did not remove the last one, because
/// the last one is the program being run.
///
/// So what a hostile session id could reach changed KIND rather than
/// disappearing. It stopped being a shell metacharacter problem (CWE-78) and
/// became an **argument injection** problem (**CWE-88**): `claude --help`
/// documents
///
/// ```text
/// -r, --resume [value]   Resume a conversation by session ID, or
///                        open interactive picker with optional search term
/// ```
///
/// — an option whose value is **optional**. When the id travelled as its own
/// trailing argv element, an id beginning with `-` was read by `claude` as a
/// NEW OPTION rather than as this option's argument. Measured at `claude`
/// 2.1.248 with stdin at `/dev/null`, `claude --resume --version` printed
/// `2.1.248 (Claude Code)` and exited 0 — the injection firing, with a harmless
/// flag standing in for `--dangerously-skip-permissions`. The id is scraped
/// verbatim from another local process's `/proc/<pid>/cmdline`, so planting one
/// needs no privilege.
///
/// # Why FUSION (`--resume=<id>`) and not a `--` end-of-options separator
///
/// The obvious fix is the wrong one, and it was measured rather than reasoned
/// about — at the same binary, same conditions:
///
/// | Probe | Command | Observed |
/// |---|---|---|
/// | C | `claude --resume -- --version` | `Error: --resume requires a valid session ID or session title when used with --print.` |
/// | D | `claude --resume -- 550e8400-e29b-41d4-a716-446655440000` | **byte-identical to C** |
/// | E | `claude --resume=550e8400-e29b-41d4-a716-446655440000` | `No conversation found with session ID: 550e8400-e29b-41d4-a716-446655440000` |
///
/// **D is the finding that decides this function.** A `--` separator closes the
/// injection (C) and, in the same stroke, **deletes the resume** (D): `--`
/// terminates option parsing, so the id lands as a POSITIONAL operand and
/// `--resume` receives nothing at all. A valid UUID and a hostile flag produce
/// the same error, which means the failure is silent on every legitimate
/// session — a feature deletion wearing a security fix's clothes, the exact
/// shape [`terminal_program_separator`]'s own doc exists to prevent.
///
/// Fusion has neither cost. In `--resume=<id>` the value is bound to the option
/// by the argv element itself, so its first byte stops being syntax (probe B:
/// `claude --resume=--version` reports `"--version" is not a UUID and does not
/// match any session title` — the flag arriving as DATA), and a real id still
/// resumes (probe E). The id reaching the child stays byte-identical to
/// `as_raw_for_logic_only()`, which is the capability half: an id that does not
/// arrive whole resumes nothing.
///
/// Note what fusion is NOT: it is not escaping and not validation. It removes
/// the receiving parser's ability to reinterpret ANY byte of the value, which
/// is why [`crate::session_detector::read_session_id`] can and does keep its
/// unvalidated pass-through — see that function's doc for the other half of
/// this decision.
///
/// The shape is asserted, not asserted-about, by
/// `tests::the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program`,
/// observed RED against the construction this replaces.
fn resume_terminal_argv(term: &str, sid: &Untrusted) -> Vec<String> {
    let mut argv = vec![terminal_program_separator(term).to_string()];
    argv.extend(claude_resume_args(sid));
    argv
}

/// The resumed program and its arguments — **the ONE construction site of the
/// fused `--resume=` element** (T-W0D-02, T-21-31-01, CWE-88).
///
/// Both carriers build from here: the GUI path through
/// [`resume_terminal_argv`], the tmux path through [`tmux_resume_argv`]. Two
/// separate constructions would agree on the day they were written and diverge
/// on the day one of them is edited — and the entire CWE-88 fix lives in the
/// bytes of that one element, so a divergence is a silently reopened
/// vulnerability on whichever path was not edited.
/// `tests::both_resume_paths_carry_the_same_fused_element` is what makes
/// "one site" checkable rather than a claim.
fn claude_resume_args(sid: &Untrusted) -> Vec<String> {
    vec![
        "claude".to_string(),
        // ONE element, with the untrusted id FUSED to the option name it
        // belongs to. `execve` hands this over unparsed and `claude`'s own
        // option parser then reads everything after the `=` as this option's
        // VALUE — so an id beginning with `-` is data rather than a new
        // option of its own (CWE-88). The raw id is what must arrive, and it
        // arrives whole: an escaped or truncated one would resume nothing.
        format!("{RESUME_OPTION_FUSED_PREFIX}{}", sid.as_raw_for_logic_only()),
    ]
}

/// The program for a NEW session. The sibling of [`claude_resume_args`], and
/// the same single-site reasoning: [`launch_terminal_argv`] and
/// [`tmux_launch_argv`] both build from here.
fn claude_launch_args() -> Vec<String> {
    vec!["claude".to_string()]
}

/// The program vector handed to `tmux new-window` when resuming (D-03).
///
/// # Why it begins with the literal `env`
///
/// `man tmux`, verbatim: *"the new-window ... and respawn-pane commands allow
/// shell-command to be given as multiple arguments and executed directly
/// (without 'sh -c'). This can avoid issues with shell quoting."* A
/// shell-command given as a **single** argument still goes through `sh -c`.
/// The new-session vector would naturally be the single element `claude`,
/// landing exactly on that edge and reintroducing an interpreter into a path
/// CR-01 cleared. The leading `env` makes `len() >= 2` true **by
/// construction** for every vector, present and future, instead of by a rule
/// each caller has to remember, and it fixes the head to a literal that
/// provably does not start with `-`, so tmux's own option parser stops there
/// rather than consuming part of this vector.
///
/// **`env` is not a command interpreter.** It has no program-string mode: it
/// consumes leading `NAME=VALUE` and option words, then `execvp`s the first
/// non-option word with the remainder passed through **unparsed**. No byte of
/// the fused `--resume=` element can be reinterpreted by it. This is the same
/// structural reading of `env` that the envelope's `resolve_program` already
/// uses. The rejected alternative was padding with a `claude` flag: no `claude`
/// flag is provably inert, so that would trade a structural guarantee for a
/// behaviour change.
///
/// **No separator appears in a tmux vector.** A separator exists to stop an
/// EMULATOR consuming the resumed program's own options; tmux takes the program
/// vector directly rather than after an emulator's own options, so there is
/// nothing here for a separator to protect.
///
/// The working directory does not travel in this vector either: it goes as
/// `tmux new-window -c <dir>`, an argv element, never as a `cd` written into a
/// program string (T-W0D-03).
fn tmux_resume_argv(sid: &Untrusted) -> Vec<String> {
    let mut argv = vec!["env".to_string()];
    argv.extend(claude_resume_args(sid));
    argv
}

/// The program vector handed to `tmux new-window` for a NEW session. Same
/// leading `env` and the same reason — see [`tmux_resume_argv`]. This is the
/// vector D-03 was actually written about: without the `env` it would be the
/// single element `claude`, which tmux hands to `sh -c`.
fn tmux_launch_argv() -> Vec<String> {
    let mut argv = vec!["env".to_string()];
    argv.extend(claude_launch_args());
    argv
}

/// The argv for launching a NEW Claude session. Same shape, same reason — see
/// [`resume_terminal_argv`]. The project path used to be interpolated into the
/// same interpreter program string here and now travels through
/// [`std::process::Command::current_dir`] instead.
///
/// # EXAMINED and deliberately left unchanged by 21-31 (D-21-49)
///
/// When [`resume_terminal_argv`] was fused against CWE-88, this sibling was the
/// obvious next place to look. It was looked at, and the answer is that there is
/// nothing here to protect: **every element of this vector is authored in this
/// file.** The separator comes from [`terminal_program_separator`]'s table and
/// the program name is a literal. No untrusted value reaches it at all — the
/// project path travels through `current_dir`, outside the argv — so there is
/// no value an option parser could reinterpret, and neither a fusion nor a `--`
/// separator would have anything to bind. Adding one would be ceremony that
/// looks like a control.
///
/// That reasoning is only true while the vector stays two elements long, and
/// prose cannot enforce that. `tests::launch_terminal_argv_carries_no_untrusted
/// _element_and_is_pinned_at_two` asserts the returned vector EQUALS an
/// authored two-element vector for every terminal, so appending a third element
/// — the only way this builder could acquire an untrusted value — fails loudly
/// and forces this decision to be made explicitly again rather than inherited.
fn launch_terminal_argv(term: &str) -> Vec<String> {
    let mut argv = vec![terminal_program_separator(term).to_string()];
    argv.extend(claude_launch_args());
    argv
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

crate::ui::screens::adjudicate_screen!(
    DetailScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "The widest identity surface in the tree. Draws the registry key in its \
     tab-bar title, and in its eleven tabs the values parsed out of the \
     project's `.planning/`. Per tab, the values and where their bytes come \
     from: PhaseList and RoadmapViz draw each `RoadmapPhase`'s number, name \
     and description plus the status and milestone, all parsed from \
     `ROADMAP.md`/`STATE.md`; Pipeline draws the current phase name, status \
     and the HANDOFF pause context; Queue draws each `QueuedAction::command` \
     from `queue.md`; Backlog draws a `999.*` directory's number and \
     description in its collapsed state and that directory's NAME (through \
     `Block::title`) plus the BODY of the first `.md` file inside it when \
     expanded; GitHistory draws a third-party repository's commit hash, \
     date, author and subject; Sessions draws a session id scraped from \
     another process's `--resume` argument via `/proc`; Archive draws \
     milestone version strings, archive file names and phase display names \
     from `.planning/archive/` directory listings, at three different \
     depths that are three different renders of three different names; \
     Defaults draws the value of every key of the project's \
     `.planning/config.json`, of which `mode`, `granularity`, \
     `project_code`, `phase_naming` and `response_language` are free-form \
     strings; Browse draws the browsed directory's path relative to \
     `.planning/`, each listing entry's name, and — in its file view — the \
     file name and the whole markdown body; Driver draws the run id suffix, \
     goal, `gsd_command` and run directory read back out of a run's \
     committed `run.json`. All of it is third-party text under SAFE-07 and \
     none of it was authored by this build. Fixture states: one per \
     sub-view, all eleven, EACH RENDERING ITS POPULATED BRANCH (21-25), plus \
     four within-tab states for the fields that dispatch to a different \
     render — Backlog expanded, Archive at its phase list and file list \
     depths, Browse at its file view. Arrival is recorded per state by \
     DETAIL_TAB_ARRIVAL against the chrome baseline, so a populated cache \
     the render never reads is reported rather than counted.",
);

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
                                // READ BY A HUMAN (it lands in a status message
                                // below), and shortened by CHARACTERS — see
                                // `shorten_session_id`'s doc for why the byte
                                // slice this replaced was a panic.
                                let short_id = shorten_session_id(sid);
                                let plan = resolve_launch_plan();
                                let mut tmux_err: Option<String> = None;
                                if plan.try_tmux {
                                    // D-01: the session should appear WHERE THE
                                    // USER IS. `$TERMINAL` says which GUI
                                    // emulator they prefer, not where a session
                                    // belongs, and it is usually exported once
                                    // from a shell profile rather than meant
                                    // per invocation.
                                    match crate::terminal_switch::open_new_window(
                                        &session.working_dir,
                                        &tmux_resume_argv(sid),
                                    ) {
                                        Ok(()) => {
                                            return ScreenAction::SetStatusMessage(format!(
                                                "Resumed session {}",
                                                short_id
                                            ))
                                        }
                                        // Fall through to the GUI rather than
                                        // failing: no reachable tmux server is
                                        // exactly when `$TERMINAL` should win.
                                        Err(e) => tmux_err = Some(e),
                                    }
                                }
                                match plan.gui {
                                    Some(term) => {
                                        // ARGV ELEMENTS, every one of them —
                                        // the kernel hands them to `execve`
                                        // unparsed. NOT program fragments,
                                        // which is what round 9's comment here
                                        // wrongly claimed they already were:
                                        // "A SUBPROCESS ARGUMENT: the raw id is
                                        // what `claude --resume` must receive,
                                        // and an escaped one would resume
                                        // nothing." (round 9). The value was a
                                        // fragment of a program handed to an
                                        // interpreter through `-c`, so a quote
                                        // in it ran arbitrary code (CR-01).
                                        // The INTERPRETER is gone; a parser is
                                        // not. `claude`'s own option parser
                                        // reads this argv, and `--resume`
                                        // takes an OPTIONAL value — so the id
                                        // travels FUSED as `--resume=<id>`,
                                        // which binds it as that option's
                                        // value instead of letting a leading
                                        // `-` make it an option of its own
                                        // (CWE-88). The working directory
                                        // travels through `current_dir`, not
                                        // as a `cd` written into a program.
                                        // See `resume_terminal_argv`'s doc for
                                        // the measurement that chose fusion
                                        // over a `--` separator.
                                        match std::process::Command::new(&term)
                                            .args(resume_terminal_argv(&term, sid))
                                            .current_dir(&session.working_dir)
                                            .spawn()
                                        {
                                            Ok(_) => {
                                                // A fall-through is REPORTED
                                                // rather than silent: the user
                                                // asked for a session where
                                                // they are, and got one
                                                // somewhere else.
                                                return ScreenAction::SetStatusMessage(
                                                    match tmux_err {
                                                        Some(e) => format!(
                                                            "tmux: {} — opened in {}",
                                                            e, term
                                                        ),
                                                        None => format!(
                                                            "Resumed session {}",
                                                            short_id
                                                        ),
                                                    },
                                                );
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
                                        return ScreenAction::SetStatusMessage(match tmux_err {
                                            Some(e) => format!(
                                                "tmux: {e}; no terminal emulator found (set \
                                                 $TERMINAL)"
                                            ),
                                            None => "No terminal emulator found (set $TERMINAL)"
                                                .to_string(),
                                        })
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
                                    // THE SEED, and the site the laundering ran
                                    // through: `entry.value` is a free-form
                                    // string out of the project's
                                    // `.planning/config.json`, already escaped
                                    // by the list render one render away. It
                                    // enters the buffer as what it is.
                                    cache.defaults_text_buffer =
                                        if entry.value == "(unset)" {
                                            super::EditBuffer::default()
                                        } else {
                                            super::EditBuffer::seed_from_untrusted_source(
                                                entry.value.clone(),
                                            )
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
                    let plan = resolve_launch_plan();
                    let mut tmux_err: Option<String> = None;
                    if plan.try_tmux {
                        // D-01, same as the resume site: a new session belongs
                        // where the user is. The working directory travels as
                        // `tmux new-window -c <dir>`, an argv element, never as
                        // a `cd` written into a program string.
                        match crate::terminal_switch::open_new_window(
                            &project.path,
                            &tmux_launch_argv(),
                        ) {
                            Ok(()) => {
                                return ScreenAction::SetStatusMessage(
                                    "Launched new Claude session".to_string(),
                                )
                            }
                            Err(e) => tmux_err = Some(e),
                        }
                    }
                    match plan.gui {
                        Some(term) => {
                            // ARGV ELEMENTS, and the working directory travels
                            // through `current_dir`. Same construction and the
                            // same reason as the resume site above: the project
                            // path used to be interpolated into a program
                            // string an interpreter parsed, so a quote in a
                            // registered project path was code. See
                            // `resume_terminal_argv`'s doc for the sink kinds.
                            match std::process::Command::new(&term)
                                .args(launch_terminal_argv(&term))
                                .current_dir(&project.path)
                                .spawn()
                            {
                                Ok(_) => ScreenAction::SetStatusMessage(match tmux_err {
                                    Some(e) => format!("tmux: {} — opened in {}", e, term),
                                    None => "Launched new Claude session".to_string(),
                                }),
                                Err(e) => ScreenAction::SetStatusMessage(format!(
                                    "Failed to launch: {}",
                                    e
                                )),
                            }
                        }
                        None => ScreenAction::SetStatusMessage(match tmux_err {
                            Some(e) => {
                                format!("tmux: {e}; no terminal emulator found (set $TERMINAL)")
                            }
                            None => "No terminal emulator found (set $TERMINAL)".to_string(),
                        }),
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
                // A CHARACTER the operator typed, never a byte.
                cache.defaults_text_buffer.push_char(c);
                ctx.needs_redraw = true;
            }
            KeyCode::Backspace => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                // Backspace removes a whole CHARACTER — a byte-indexed pop is
                // the `&sid[..8]` panic family one file over.
                cache.defaults_text_buffer.pop_char();
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
                // PERSISTENCE — the one question the raw take answers. These
                // are the bytes the operator typed, going back to their
                // `.planning/config.json` byte-identical; a `U+XXXX` display
                // spelling reaching this line would rewrite their config file
                // with a rendering of itself. Pinned by
                // `ui::screens::tests::what_the_operator_types_is_what_is_persisted`.
                let buffer = cache.defaults_text_buffer.take_raw_for_persistence();
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

        // ID-04: the pane YIELDS to the list on a short terminal. Below the
        // floor the split is skipped entirely and the list keeps the full
        // `area`, so a small window never loses option rows to help text.
        let (list_area, help_area) = if area.height >= HELP_PANE_FLOOR {
            let chunks = Layout::vertical([
                Constraint::Min(1),
                Constraint::Length(HELP_PANE_HEIGHT),
            ])
            .split(area);
            (chunks[0], Some(chunks[1]))
        } else {
            (area, None)
        };

        let mut list_state = ListState::default();
        list_state.select(Some(selected));
        frame.render_stateful_widget(list, list_area, &mut list_state);

        if let Some(help_area) = help_area {
            // `selected` is the cache's own cursor and the list is non-empty
            // here, but the cache is not this function's to trust: an entry
            // count that shrank since the cursor was set would panic on a
            // bare index.
            let idx = selected.min(entries.len() - 1);
            frame.render_widget(build_config_help_pane(&entries[idx].help), help_area);
        }

        // Render dropdown OR text-input overlay if editing
        if let Some(cache) = cache {
            if let Some(editing_idx) = cache.defaults_editing {
                if let Some(entry) = entries.get(editing_idx) {
                    if matches!(entry.kind, ConfigValueKind::String) {
                        let title =
                            format!(" {} {DEFAULTS_EDIT_BRANCH_TOKEN} ", entry.key);
                        // READ BY A HUMAN, and the whole point of `EditBuffer`:
                        // there is no other route from the buffer to a cell.
                        // `Span::styled(buffer.clone(), ..)` does not compile.
                        let rendered: String = cache.defaults_text_buffer.shown().into();
                        // IN-02: CHARACTERS, not bytes. This read `.len()` on a
                        // value out of the project's `.planning/config.json`,
                        // so a CJK value (3 bytes/char) or an emoji one (4)
                        // sized the popup two to four times wider than the text
                        // needs. `chars().count()` matches `section_rule` here
                        // and `roadmap_widget.rs`'s width arithmetic.
                        //
                        // **A character count is still not DISPLAY width** — a
                        // CJK character occupies two terminal cells and a
                        // combining mark occupies none — and closing that would
                        // need `unicode-width`, which is a transitive dependency
                        // of ratatui rather than a direct one. That residual is
                        // recorded beside the existing IN-02/IN-03 entry in this
                        // phase's `deferred-items.md` rather than silently
                        // improved.
                        let inner_w =
                            rendered.chars().count().max(title.chars().count()).max(30) as u16;
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
                            Span::styled(rendered.clone(), Style::default().fg(Color::White)),
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
                        let popup_w =
                            dropdown_popup_width(&options, &entry.help, &title, area.width);
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
                                // Value FIRST, explanation after: the popup is
                                // clamped to the area, so when the clamp bites
                                // it is the explanation that gets cut and the
                                // selectable value that survives.
                                let mut spans = vec![
                                    Span::raw(marker),
                                    Span::raw(opt.clone()),
                                ];
                                // `&'static str` from `ConfigHelp`, never a
                                // value out of the project's config.json
                                // (T-S0N-01). An option with no recorded
                                // explanation renders exactly as before.
                                if let Some(explanation) = entry.help.explanation_for(opt) {
                                    spans.push(Span::styled(
                                        CHOICE_SEPARATOR,
                                        Style::default().fg(Color::DarkGray),
                                    ));
                                    spans.push(Span::styled(
                                        explanation,
                                        Style::default().fg(Color::DarkGray),
                                    ));
                                }
                                ListItem::new(Line::from(spans)).style(style)
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

/// What one Defaults-tab option MEANS, authored at the option's definition site.
///
/// **Why a field and not a lookup table** (ID-01, quick task 260909-s0n). The
/// Defaults tab lists 73 raw GSD config keys — `nyquist_validation`,
/// `workflow.specless_probe_fallback`, `granularity` — as bare identifiers next
/// to a value, and there is no second place in this TUI that says what any of
/// them does. A `HashMap<&str, &str>` keyed by config key would compile fine on
/// the day a 74th option is added and silently render a blank line at runtime.
/// Carrying the help as a REQUIRED field, supplied through
/// [`build_defaults_entries`]'s `push` closure, makes a new option with no help
/// a COMPILE ERROR instead. That breakage is the coverage mechanism.
///
/// **Everything in here is `&'static str` this build authored** (T-S0N-01). No
/// value parsed out of a project's `.planning/config.json` may be interpolated
/// into a `ConfigHelp` on its way to the help pane. The file's escaping rule
/// (`shown` / [`crate::text::render_for_terminal`], documented at the
/// `val_span` site in `render_defaults_tab`) exists because those values are
/// attacker-influenced text; the pane is outside that rule BY CONSTRUCTION
/// rather than by remembering to call a function, and it stays outside it only
/// while the two fields below remain `&'static`.
#[derive(Debug, Clone, Copy)]
struct ConfigHelp {
    /// One sentence saying what the option does — for a boolean, what turning
    /// it ON causes; for a duration or budget, the UNIT.
    summary: &'static str,
    /// `(value, one-clause explanation)` for an option with a discrete choice
    /// set. Empty for free-form strings, integers and read-only keys.
    ///
    /// Asserted equal to `dropdown_options(&entry.kind)` by
    /// `every_choice_bearing_entry_documents_exactly_its_dropdown_options`, so
    /// the list a human READS is the list the editor OFFERS.
    choices: &'static [(&'static str, &'static str)],
}

impl ConfigHelp {
    /// Help for an option with no discrete choice set.
    const fn new(summary: &'static str) -> Self {
        Self { summary, choices: &[] }
    }

    /// Help for an option whose values are enumerable, each with its own
    /// explanation.
    const fn with_choices(
        summary: &'static str,
        choices: &'static [(&'static str, &'static str)],
    ) -> Self {
        Self { summary, choices }
    }

    /// The recorded explanation for one value, or `None` when the value has
    /// none — in which case the dropdown renders that option exactly as it did
    /// before this help existed.
    fn explanation_for(&self, value: &str) -> Option<&'static str> {
        self.choices
            .iter()
            .find(|(v, _)| *v == value)
            .map(|(_, explanation)| *explanation)
    }
}

/// Rows the Defaults help pane occupies, INCLUDING its top border — one
/// summary row plus roughly two wrapped choice rows at 100 columns.
///
/// The pane deliberately does NOT grow to fit a long choice list: `Wrap`
/// truncates at the pane boundary and the dropdown popup is where the complete
/// list is guaranteed readable.
const HELP_PANE_HEIGHT: u16 = 4;

/// Below this content height `render_defaults_tab` draws no help pane at all
/// and gives the option list the whole area (ID-04).
///
/// A pane that keeps its four rows in a ten-row window would leave five rows of
/// a seventy-three-row list, which is the failure this floor exists to prevent.
const HELP_PANE_FLOOR: u16 = 12;

/// Between a choice's value and its explanation, in the help pane AND in the
/// dropdown popup.
///
/// Spelled once because [`dropdown_popup_width`] MEASURES it: a separator the
/// width arithmetic and the drawing disagreed about is a popup sized for text
/// it does not contain.
const CHOICE_SEPARATOR: &str = " — ";

/// Between two choices on the help pane's choice row.
const CHOICE_DELIMITER: &str = "  ·  ";

/// Width of the dropdown's current-value marker column (`"● "` / `"  "`).
const DROPDOWN_MARKER_WIDTH: usize = 2;

/// The Defaults dropdown popup's outer width, clamped to the content area.
///
/// A pure function so the clamp is drivable from a test at several area widths
/// (T-S0N-02) instead of only through a render.
///
/// **Characters, not bytes** — the same correction the text-input popup
/// already documents as IN-02. A byte-length measure over multi-byte
/// explanation text sizes the popup two to four times wider than the text
/// needs, and the `.min(area_width - 2)` clamp then bites at every width. The
/// clamp is what keeps the popup inside the area; when it bites, the
/// EXPLANATION is what gets cut rather than the value, which is why the
/// drawing below puts the value first.
fn dropdown_popup_width(
    options: &[String],
    help: &ConfigHelp,
    title: &str,
    area_width: u16,
) -> u16 {
    let max_opt_width = options
        .iter()
        .map(|opt| {
            let base = DROPDOWN_MARKER_WIDTH + opt.chars().count();
            match help.explanation_for(opt) {
                Some(explanation) => {
                    base + CHOICE_SEPARATOR.chars().count() + explanation.chars().count()
                }
                None => base,
            }
        })
        .max()
        .unwrap_or(0);
    let inner_w = max_opt_width.max(title.chars().count() + 2).max(20);
    let inner_w = u16::try_from(inner_w).unwrap_or(u16::MAX);
    inner_w
        .saturating_add(4)
        .min(area_width.saturating_sub(2))
}

/// The pane under the Defaults list: the selected option's summary, and its
/// choice explanations when it has any.
///
/// **Everything drawn here is `&'static str` this build authored** (T-S0N-01).
/// Nothing parsed out of a project's `.planning/config.json` — not
/// `entry.value`, not even `entry.key` — is interpolated in, so the pane sits
/// outside the file's `shown()` escaping rule by construction rather than by
/// remembering to call it. Keep it that way: the moment a project-supplied
/// string reaches this function it needs `crate::text::render_for_terminal`,
/// and the guarantee stops being structural.
fn build_config_help_pane(help: &ConfigHelp) -> Paragraph<'static> {
    let mut lines: Vec<Line<'static>> = vec![Line::from(Span::styled(
        help.summary,
        Style::default().fg(Color::Gray),
    ))];

    if !help.choices.is_empty() {
        let mut spans: Vec<Span<'static>> = Vec::new();
        for (i, (value, explanation)) in help.choices.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled(
                    CHOICE_DELIMITER,
                    Style::default().fg(Color::DarkGray),
                ));
            }
            // The value reads as the primary text so a scan finds it; the
            // explanation is dim beside it.
            spans.push(Span::styled(*value, Style::default().fg(Color::Yellow)));
            spans.push(Span::styled(
                CHOICE_SEPARATOR,
                Style::default().fg(Color::DarkGray),
            ));
            spans.push(Span::styled(
                *explanation,
                Style::default().fg(Color::DarkGray),
            ));
        }
        lines.push(Line::from(spans));
    }

    Paragraph::new(lines)
        .block(Block::default().borders(Borders::TOP))
        .wrap(Wrap { trim: true })
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
    /// What this option means. Required — see [`ConfigHelp`] for why it is a
    /// field rather than a side table.
    help: ConfigHelp,
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
    // `help` is REQUIRED and last (ID-01). A 74th option added below without a
    // `ConfigHelp` does not compile, which is the whole reason the help lives
    // here rather than in a key-indexed side table.
    let mut push = |cat: &'static str,
                    key: &'static str,
                    value: String,
                    kind: ConfigValueKind,
                    first: bool,
                    from_defaults: bool,
                    help: ConfigHelp| {
        entries.push(ConfigEntry {
            category: cat,
            key,
            value,
            kind,
            show_category: first,
            from_defaults,
            help,
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
    push(cat, "research", v, k, true, fd, ConfigHelp::new(
        "Runs a research agent before planning so plans are written against fetched docs rather than recall.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.plan_check), dwf.and_then(|w| w.plan_check));
    push(cat, "plan_check", v, k, false, fd, ConfigHelp::new(
        "Sends every finished PLAN.md to a checker agent that must pass it before the phase is executed.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.pattern_mapper), dwf.and_then(|w| w.pattern_mapper));
    push(cat, "pattern_mapper", v, k, false, fd, ConfigHelp::new(
        "Maps the codebase's existing conventions between research and planning so new work imitates them.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.nyquist_validation), dwf.and_then(|w| w.nyquist_validation));
    push(cat, "nyquist_validation", v, k, false, fd, ConfigHelp::new(
        "Adds sampling-rate validation gates, so a phase is checked often enough to catch drift between checks.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ui_phase), dwf.and_then(|w| w.ui_phase));
    push(cat, "ui_phase", v, k, false, fd, ConfigHelp::new(
        "Generates a UI-SPEC.md design contract before planning any phase the roadmap marks as frontend.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ui_safety_gate), dwf.and_then(|w| w.ui_safety_gate));
    push(cat, "ui_safety_gate", v, k, false, fd, ConfigHelp::new(
        "Blocks a UI-bearing phase from planning until its UI-SPEC.md exists and the UI checker approves it.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ai_integration_phase), dwf.and_then(|w| w.ai_integration_phase));
    push(cat, "ai_integration_phase", v, k, false, fd, ConfigHelp::new(
        "Generates an AI-SPEC.md design contract before planning a phase that builds prompts, agents or evals.",
    ));
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.subagent_timeout), dwf.and_then(|w| w.subagent_timeout));
    push(cat, "subagent_timeout", v, k, false, fd, ConfigHelp::new(
        "Milliseconds a spawned subagent may run before it is killed; GSD's default is 300000, five minutes.",
    ));
    // GSD 1.4–1.8 planning gates
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.specless_probe_fallback), dwf.and_then(|w| w.specless_probe_fallback));
    push(cat, "workflow.specless_probe_fallback", v, k, false, fd, ConfigHelp::with_choices(
        "With no SPEC edge or prohibition section, probes the codebase for those predicates instead of skipping.",
        &[
            ("true", "probe and author the missing predicates"),
            ("false", "skip the probe, but print a visible marker"),
        ],
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.assumption_delta), dwf.and_then(|w| w.assumption_delta));
    push(cat, "workflow.assumption_delta", v, k, false, fd, ConfigHelp::new(
        "Raises one identity-model question during planning when a phase turns a single thing plural or optional.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.plan_drift_precheck), dwf.and_then(|w| w.plan_drift_precheck));
    push(cat, "workflow.plan_drift_precheck", v, k, false, fd, ConfigHelp::new(
        "Re-checks the codebase against the plan just before execution and reports what moved since planning.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.plan_chunked), dwf.and_then(|w| w.plan_chunked));
    push(cat, "workflow.plan_chunked", v, k, false, fd, ConfigHelp::new(
        "Writes long plans as an outline plus one short task per plan, each committed, instead of one long run.",
    ));
    let (v, k, fd) = str_l(pwf.and_then(|w| w.context_guard_mode.as_deref()), dwf.and_then(|w| w.context_guard_mode.as_deref()));
    push(cat, "workflow.context_guard_mode", v, k, false, fd, ConfigHelp::new(
        "How execute-phase reacts to context pressure at a wave boundary: warn (default), auto to pause, or off.",
    ));

    // ── Execution ──────────────────────────────────────────────
    let cat = "Execution";
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.verifier), dwf.and_then(|w| w.verifier));
    push(cat, "verifier", v, k, true, fd, ConfigHelp::new(
        "Runs a verifier agent after execution to check the built work against the phase's must-have truths.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.tdd_mode), dwf.and_then(|w| w.tdd_mode));
    push(cat, "tdd_mode", v, k, false, fd, ConfigHelp::new(
        "Forces red-green-refactor: a behaviour-adding task must show a failing test before its code is written.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.code_review), dwf.and_then(|w| w.code_review));
    push(cat, "code_review", v, k, false, fd, ConfigHelp::new(
        "Runs the built-in reviewing agent over a phase's diff and records its blockers and warnings.",
    ));
    let (v, k, fd) = enum_l(
        pwf.and_then(|w| w.code_review_depth.as_deref()),
        dwf.and_then(|w| w.code_review_depth.as_deref()),
        &["quick", "standard", "deep"],
    );
    push(cat, "code_review_depth", v, k, false, fd, ConfigHelp::with_choices(
        "How deeply that code review reads the diff, trading findings against time and tokens.",
        &[
            ("quick", "skims the diff for obvious defects"),
            ("standard", "reads changed files in full, the default"),
            ("deep", "traces call paths and edge cases, slowest"),
        ],
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ui_review), dwf.and_then(|w| w.ui_review));
    push(cat, "ui_review", v, k, false, fd, ConfigHelp::new(
        "Runs the six-pillar visual audit of /gsd-ui-review over a phase's frontend code in autonomous mode.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.node_repair), dwf.and_then(|w| w.node_repair));
    push(cat, "node_repair", v, k, false, fd, ConfigHelp::new(
        "Lets the orchestrator retry a failed plan node automatically instead of halting the whole phase.",
    ));
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.node_repair_budget), dwf.and_then(|w| w.node_repair_budget));
    push(cat, "node_repair_budget", v, k, false, fd, ConfigHelp::new(
        "How many automatic retries one failed plan node gets before the phase stops and reports it.",
    ));
    // GSD 1.8 execution gates
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.api_coverage_gate), dwf.and_then(|w| w.api_coverage_gate));
    push(cat, "workflow.api_coverage_gate", v, k, false, fd, ConfigHelp::new(
        "Requires a COVERAGE.md matrix, with a reasoned opt-out per capability, before an API phase can seal.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.windows_enforce), dwf.and_then(|w| w.windows_enforce));
    push(cat, "workflow.windows_enforce", v, k, false, fd, ConfigHelp::new(
        "Blocks /gsd-ship while any stub, skipped test or unrun verify is still open in .planning/WINDOWS.md.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.mvp_mode), dwf.and_then(|w| w.mvp_mode));
    push(cat, "workflow.mvp_mode", v, k, false, fd, ConfigHelp::new(
        "Frames every phase as one vertical MVP slice of a user-visible capability rather than as a layer.",
    ));
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.test_gate_timeout), dwf.and_then(|w| w.test_gate_timeout));
    push(cat, "workflow.test_gate_timeout", v, k, false, fd, ConfigHelp::new(
        "Seconds the regression test gate may run before it aborts; the default is 600, and watch mode trips it.",
    ));
    let (v, k, fd) = str_l(pwf.and_then(|w| w.code_review_command.as_deref()), dwf.and_then(|w| w.code_review_command.as_deref()));
    push(cat, "workflow.code_review_command", v, k, false, fd, ConfigHelp::new(
        "External review command fed the diff on stdin; it must print JSON with a verdict of APPROVED or REVISE.",
    ));
    // Shape-varying security keys — read-only display.
    let (v, k, fd) = opt_json_readonly(
        pwf.and_then(|w| w.security_asvs_level.as_ref()),
        dwf.and_then(|w| w.security_asvs_level.as_ref()),
    );
    push(cat, "workflow.security_asvs_level", v, k, false, fd, ConfigHelp::new(
        "OWASP ASVS rigor of the security audit, 1 opportunistic to 3 comprehensive; shown here, edited in the file.",
    ));
    let (v, k, fd) = opt_json_readonly(
        pwf.and_then(|w| w.security_block_on.as_ref()),
        dwf.and_then(|w| w.security_block_on.as_ref()),
    );
    push(cat, "workflow.security_block_on", v, k, false, fd, ConfigHelp::new(
        "Lowest threat severity that blocks a phase — critical, high, medium, low or none; edited in the file.",
    ));

    // ── Docs & Output ─────────────────────────────────────────
    let cat = "Docs & Output";
    let (v, k, fd) = bool_l(config.commit_docs, defaults.and_then(|d| d.commit_docs));
    push(cat, "commit_docs", v, k, true, fd, ConfigHelp::new(
        "Commits .planning/ artifacts such as PLAN.md and SUMMARY.md; off keeps them out of git history.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.skip_discuss), dwf.and_then(|w| w.skip_discuss));
    push(cat, "skip_discuss", v, k, false, fd, ConfigHelp::new(
        "Skips the discussion round entirely and plans each phase straight from its roadmap entry.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.use_worktrees), dwf.and_then(|w| w.use_worktrees));
    push(cat, "use_worktrees", v, k, false, fd, ConfigHelp::new(
        "Runs executor agents in isolated git worktrees so agents in one wave cannot overwrite each other.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.text_mode), dwf.and_then(|w| w.text_mode));
    push(cat, "text_mode", v, k, false, fd, ConfigHelp::new(
        "Asks questions as plain numbered lists instead of interactive menus, for terminals that lack them.",
    ));
    let (v, k, fd) = str_l(
        config.response_language.as_deref(),
        defaults.and_then(|d| d.response_language.as_deref()),
    );
    push(cat, "response_language", v, k, false, fd, ConfigHelp::new(
        "Language GSD writes user-facing prose in, given as a name such as English, Portuguese or Japanese.",
    ));

    // ── Features ──────────────────────────────────────────────
    let cat = "Features";
    let (v, k, fd) = bool_l(
        config.intel.as_ref().and_then(|i| i.enabled),
        defaults.and_then(|d| d.intel.as_ref().and_then(|i| i.enabled)),
    );
    push(cat, "intel_enabled", v, k, true, fd, ConfigHelp::new(
        "Builds a queryable code index under .planning/intel/ so agents look symbols up instead of grepping.",
    ));
    let (v, k, fd) = bool_l(
        config.graphify.as_ref().and_then(|g| g.enabled),
        defaults.and_then(|d| d.graphify.as_ref().and_then(|g| g.enabled)),
    );
    push(cat, "graphify_enabled", v, k, false, fd, ConfigHelp::new(
        "Builds the project knowledge graph so decisions and phases can be queried by relation, not by reading.",
    ));
    let (v, k, fd) = u32_l(
        config.graphify.as_ref().and_then(|g| g.build_timeout),
        defaults.and_then(|d| d.graphify.as_ref().and_then(|g| g.build_timeout)),
    );
    push(cat, "graphify_build_timeout", v, k, false, fd, ConfigHelp::new(
        "Seconds a knowledge-graph build may run before it is abandoned; GSD's default is 300.",
    ));
    let (v, k, fd) = str_l(
        config.graphify.as_ref().and_then(|g| g.graph_path.as_deref()),
        defaults.and_then(|d| d.graphify.as_ref().and_then(|g| g.graph_path.as_deref())),
    );
    push(cat, "graphify.graph_path", v, k, false, fd, ConfigHelp::new(
        "Where graph.json lives, letting several projects share one umbrella graph instead of each building its own.",
    ));
    let (v, k, fd) = bool_l(config.brave_search, defaults.and_then(|d| d.brave_search));
    push(cat, "brave_search", v, k, false, fd, ConfigHelp::new(
        "Lets the research agent query Brave web search; without BRAVE_API_KEY in the environment it does nothing.",
    ));
    let (v, k, fd) = bool_l(config.firecrawl, defaults.and_then(|d| d.firecrawl));
    push(cat, "firecrawl", v, k, false, fd, ConfigHelp::new(
        "Lets the research agent scrape whole pages through Firecrawl; needs FIRECRAWL_API_KEY to have any effect.",
    ));
    let (v, k, fd) = bool_l(config.exa_search, defaults.and_then(|d| d.exa_search));
    push(cat, "exa_search", v, k, false, fd, ConfigHelp::new(
        "Lets the research agent use Exa semantic search; needs EXA_API_KEY in the environment to have any effect.",
    ));

    // ── Model & Pipeline ──────────────────────────────────────
    let cat = "Model & Pipeline";
    push(cat, "mode", config.mode.clone(), ConfigValueKind::Enum(&["interactive", "yolo"]), true, false, ConfigHelp::with_choices(
        "Whether GSD stops at gates and confirmations, or runs the whole workflow on its own judgement.",
        &[
            ("interactive", "stops at gates and asks you"),
            ("yolo", "runs autonomously, recording what it inferred"),
        ],
    ));
    push(cat, "granularity", config.granularity.clone(), ConfigValueKind::Enum(&["coarse", "standard", "fine"]), false, false, ConfigHelp::with_choices(
        "How finely a phase is cut into plans, trading fewer big plans against more small ones.",
        &[
            ("coarse", "few large plans, least orchestration overhead"),
            ("standard", "a plan per coherent subsystem, the default"),
            ("fine", "many small plans, easiest to resume mid-phase"),
        ],
    ));
    push(cat, "model_profile", config.model_profile.clone(), ConfigValueKind::Enum(&["quality", "balanced", "budget", "adaptive", "inherit"]), false, false, ConfigHelp::with_choices(
        "Which model each GSD agent gets, trading answer quality against token cost.",
        &[
            ("quality", "the strongest model for every agent, highest cost"),
            ("balanced", "strong models for planning, cheaper for mapping"),
            ("budget", "the cheapest capable model everywhere"),
            ("adaptive", "picks per agent from the work's routing tier"),
            ("inherit", "uses whatever model your session already runs"),
        ],
    ));
    let (v, k, fd) = bool_l(config.parallelization, defaults.and_then(|d| d.parallelization));
    push(cat, "parallelization", v, k, false, fd, ConfigHelp::new(
        "Runs plans that share no files at the same time instead of one after another.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.auto_advance), dwf.and_then(|w| w.auto_advance));
    push(cat, "auto_advance", v, k, false, fd, ConfigHelp::new(
        "Advances to the next phase on completion without stopping to ask you first.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.auto_chain_active), dwf.and_then(|w| w.auto_chain_active));
    push(cat, "auto_chain_active", v, k, false, fd, ConfigHelp::with_choices(
        "Runtime state GSD sets while an autonomous chain is in flight — read it, do not set it by hand.",
        &[
            ("true", "a chained autonomous run is in flight now"),
            ("false", "no chain running; the normal resting value"),
        ],
    ));
    let pgit = config.git.as_ref();
    let dgit = defaults.and_then(|d| d.git.as_ref());
    let (v, k, fd) = enum_l(
        pgit.and_then(|g| g.branching_strategy.as_deref()),
        dgit.and_then(|g| g.branching_strategy.as_deref()),
        &["none", "phase", "milestone"],
    );
    push(cat, "branching_strategy", v, k, false, fd, ConfigHelp::with_choices(
        "Which git branch phase work lands on, or whether it commits to the branch you are already on.",
        &[
            ("none", "commit on the current branch, no new branches"),
            ("phase", "a fresh branch per phase, merged when it closes"),
            ("milestone", "one branch for the whole milestone"),
        ],
    ));
    let (v, k, fd) = str_l(
        pgit.and_then(|g| g.base_branch.as_deref()),
        dgit.and_then(|g| g.base_branch.as_deref()),
    );
    push(cat, "base_branch", v, k, false, fd, ConfigHelp::new(
        "Branch that PRs target and that phase branches fork from; unset auto-detects it from origin/HEAD.",
    ));
    let (v, k, fd) = str_l(
        pgit.and_then(|g| g.phase_branch_template.as_deref()),
        dgit.and_then(|g| g.phase_branch_template.as_deref()),
    );
    push(cat, "phase_branch_template", v, k, false, fd, ConfigHelp::new(
        "Name pattern for a phase branch, with {phase} and {slug} substituted, e.g. gsd/phase-{phase}-{slug}.",
    ));
    let (v, k, fd) = str_l(
        pgit.and_then(|g| g.milestone_branch_template.as_deref()),
        dgit.and_then(|g| g.milestone_branch_template.as_deref()),
    );
    push(cat, "milestone_branch_template", v, k, false, fd, ConfigHelp::new(
        "Name pattern for a milestone branch, with {milestone} and {slug} substituted, e.g. gsd/{milestone}-{slug}.",
    ));
    let qbt_proj = pgit.and_then(|g| g.quick_branch_template.as_ref()).map(|v| v.to_string());
    let qbt_def = dgit.and_then(|g| g.quick_branch_template.as_ref()).map(|v| v.to_string());
    let (qbt_val, qbt_kind, qbt_fd) = match (qbt_proj, qbt_def) {
        (Some(s), _) => (s, ConfigValueKind::String, false),
        (None, Some(s)) => (s, ConfigValueKind::String, true),
        (None, None) => ("(unset)".to_string(), ConfigValueKind::Null, false),
    };
    push(cat, "quick_branch_template", qbt_val, qbt_kind, false, qbt_fd, ConfigHelp::new(
        "Optional name pattern, with {slug} substituted, for a quick task's branch; unset keeps quick work here.",
    ));

    // ── Misc ──────────────────────────────────────────────────
    let cat = "Misc";
    let (v, k, fd) = bool_l(
        config.hooks.as_ref().and_then(|h| h.context_warnings),
        defaults.and_then(|d| d.hooks.as_ref().and_then(|h| h.context_warnings)),
    );
    push(cat, "context_warnings", v, k, true, fd, ConfigHelp::new(
        "Warns in the statusline as a session's context budget runs out, before a compaction loses what was said.",
    ));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.research_before_questions), dwf.and_then(|w| w.research_before_questions));
    push(cat, "research_before_questions", v, k, false, fd, ConfigHelp::new(
        "Researches the topic before the discussion round, so the questions you are asked are already informed.",
    ));
    let (v, k, fd) = enum_l(
        pwf.and_then(|w| w.discuss_mode.as_deref()),
        dwf.and_then(|w| w.discuss_mode.as_deref()),
        &["discuss", "assumptions"],
    );
    push(cat, "discuss_mode", v, k, false, fd, ConfigHelp::with_choices(
        "Whether the discussion round interviews you or analyses the codebase and states its assumptions.",
        &[
            ("discuss", "asks you a round of questions before planning"),
            ("assumptions", "surfaces its own assumptions, asking nothing"),
        ],
    ));
    let (v, k, fd) = bool_l(config.search_gitignored, defaults.and_then(|d| d.search_gitignored));
    push(cat, "search_gitignored", v, k, false, fd, ConfigHelp::new(
        "Includes gitignored paths in broad ripgrep searches, so build output and vendored code are searched too.",
    ));
    let (v, k, fd) = str_l(config.project_code.as_deref(), defaults.and_then(|d| d.project_code.as_deref()));
    push(cat, "project_code", v, k, false, fd, ConfigHelp::new(
        "Short prefix for phase directories and requirement ids — CK gives CK-01-foundation and CK-01.",
    ));
    let (v, k, fd) = str_l(config.phase_naming.as_deref(), defaults.and_then(|d| d.phase_naming.as_deref()));
    push(cat, "phase_naming", v, k, false, fd, ConfigHelp::new(
        "Phase numbering: sequential auto-increments, while custom lets each phase carry an arbitrary string id.",
    ));
    let (v, k, fd) = str_l(config.phase_id_convention.as_deref(), defaults.and_then(|d| d.phase_id_convention.as_deref()));
    push(cat, "phase_id_convention", v, k, false, fd, ConfigHelp::new(
        "How ids are written in ROADMAP.md: sequential gives Phase 1, milestone-prefixed gives Phase 1-01.",
    ));
    let (v, k, fd) = str_l(config.claude_md_path.as_deref(), defaults.and_then(|d| d.claude_md_path.as_deref()));
    push(cat, "claude_md_path", v, k, false, fd, ConfigHelp::new(
        "Where the developer-profile writer puts its instructions section; GSD's default is ./.claude/CLAUDE.md.",
    ));
    let (v, k, fd) = opt_json_readonly(config.sub_repos.as_ref(), defaults.and_then(|d| d.sub_repos.as_ref()));
    push(cat, "sub_repos", v, k, false, fd, ConfigHelp::new(
        "Child directories with their own .git, auto-detected so commits route correctly; edited in the file.",
    ));

    // ── Orchestration ─────────────────────────────────────────
    let cat = "Orchestration";
    let pco = config.claude_orchestration.as_ref();
    let dco = defaults.and_then(|d: &GsdConfig| d.claude_orchestration.as_ref());
    let (v, k, fd) = bool_l(pco.and_then(|c| c.enabled), dco.and_then(|c| c.enabled));
    push(cat, "claude_orchestration.enabled", v, k, true, fd, ConfigHelp::new(
        "Lets GSD drive a phase's waves through the Claude Agent SDK rather than dispatching each plan itself.",
    ));
    let (v, k, fd) = str_l(pco.and_then(|c| c.execution_backend.as_deref()), dco.and_then(|c| c.execution_backend.as_deref()));
    push(cat, "claude_orchestration.execution_backend", v, k, false, fd, ConfigHelp::new(
        "Which backend runs the waves: auto to choose, workflow to force the Workflow tool, or inline to stay put.",
    ));
    let (v, k, fd) = str_l(pco.and_then(|c| c.min_agent_sdk_version.as_deref()), dco.and_then(|c| c.min_agent_sdk_version.as_deref()));
    push(cat, "claude_orchestration.min_agent_sdk_version", v, k, false, fd, ConfigHelp::new(
        "Lowest Agent SDK semver allowed to host the Workflow backend; below it GSD falls back to inline.",
    ));

    // ── Statusline ────────────────────────────────────────────
    let cat = "Statusline";
    let psl = config.statusline.as_ref();
    let dsl = defaults.and_then(|d: &GsdConfig| d.statusline.as_ref());
    let (v, k, fd) = bool_l(psl.and_then(|s| s.show_context_tokens), dsl.and_then(|s| s.show_context_tokens));
    push(cat, "statusline.show_context_tokens", v, k, true, fd, ConfigHelp::new(
        "Shows how much of the context window the running session has consumed, in the statusline.",
    ));
    let (v, k, fd) = str_l(psl.and_then(|s| s.state_format.as_deref()), dsl.and_then(|s| s.state_format.as_deref()));
    push(cat, "statusline.state_format", v, k, false, fd, ConfigHelp::new(
        "How much GSD state the statusline prints: full spells phase, plan and status; compact abbreviates them.",
    ));
    let (v, k, fd) = bool_l(psl.and_then(|s| s.show_git), dsl.and_then(|s| s.show_git));
    push(cat, "statusline.show_git", v, k, false, fd, ConfigHelp::new(
        "Shows the current git branch and its dirty-tree marker in the statusline.",
    ));

    // ── Routing ───────────────────────────────────────────────
    let cat = "Routing";
    let pdr = config.dynamic_routing.as_ref();
    let ddr = defaults.and_then(|d: &GsdConfig| d.dynamic_routing.as_ref());
    let (v, k, fd) = bool_l(pdr.and_then(|r| r.provider_escalation), ddr.and_then(|r| r.provider_escalation));
    push(cat, "dynamic_routing.provider_escalation", v, k, true, fd, ConfigHelp::new(
        "On a quota refusal, retries the step on an alternative provider's model instead of waiting for a reset.",
    ));
    let (v, k, fd) = u32_l(pdr.and_then(|r| r.max_escalations), ddr.and_then(|r| r.max_escalations));
    push(cat, "dynamic_routing.max_escalations", v, k, false, fd, ConfigHelp::new(
        "How many escalation hops one step may take before the resolver gives up and names every model it tried.",
    ));

    // ── External Job ──────────────────────────────────────────
    let cat = "External Job";
    let pej = config.external_job.as_ref();
    let dej = defaults.and_then(|d: &GsdConfig| d.external_job.as_ref());
    let (v, k, fd) = u32_l(pej.and_then(|e| e.submit_timeout_ms), dej.and_then(|e| e.submit_timeout_ms));
    push(cat, "external_job.submit_timeout_ms", v, k, true, fd, ConfigHelp::new(
        "Milliseconds the scheduler submit subprocess (sbatch) may run before it is killed; the default is 30000.",
    ));
    let (v, k, fd) = u32_l(pej.and_then(|e| e.poll_timeout_ms), dej.and_then(|e| e.poll_timeout_ms));
    push(cat, "external_job.poll_timeout_ms", v, k, false, fd, ConfigHelp::new(
        "Milliseconds the scheduler poll subprocess (squeue, then sacct) may run before it is killed; default 15000.",
    ));
    let (v, k, fd) = str_l(pej.and_then(|e| e.artifact_dir.as_deref()), dej.and_then(|e| e.artifact_dir.as_deref()));
    push(cat, "external_job.artifact_dir", v, k, false, fd, ConfigHelp::new(
        "Root directory each external job's artifacts land under, one subdirectory per job id.",
    ));

    // ── Capabilities ──────────────────────────────────────────
    let cat = "Capabilities";
    let pcap = config.capabilities.as_ref();
    let dcap = defaults.and_then(|d: &GsdConfig| d.capabilities.as_ref());
    let (v, k, fd) = bool_l(pcap.and_then(|c| c.strict_known_registries), dcap.and_then(|c| c.strict_known_registries));
    push(cat, "capabilities.strict_known_registries", v, k, true, fd, ConfigHelp::new(
        "Restricts which registries a capability pack may be installed from; an empty allowlist refuses them all.",
    ));
    let (v, k, fd) = bool_l(pcap.and_then(|c| c.auto_update), dcap.and_then(|c| c.auto_update));
    push(cat, "capabilities.auto_update", v, k, false, fd, ConfigHelp::new(
        "Refreshes installed capability packs to their latest published version instead of pinning what is there.",
    ));

    // ── Review ────────────────────────────────────────────────
    let cat = "Review";
    let (v, k, fd) = opt_json_readonly(
        config.review.as_ref().and_then(|r| r.reviewer_instances.as_ref()),
        defaults.and_then(|d| d.review.as_ref().and_then(|r| r.reviewer_instances.as_ref())),
    );
    push(cat, "review.reviewer_instances", v, k, true, fd, ConfigHelp::new(
        "Named cross-AI reviewer instances that /gsd-review dispatches to; shown here, edited in the config file.",
    ));

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

/// The Defaults string-edit popup's title suffix.
///
/// A named constant so the probe's branch-reached assertion compares against
/// the string the render actually draws instead of respelling it — the
/// `STATUS_BRANCH_TOKEN` discipline. It is drawn by this one branch and by
/// nothing else on the screen, which is what makes its presence evidence that
/// `render_defaults_tab` dispatched into the popup rather than merely that
/// `defaults_editing` was set.
pub(super) const DEFAULTS_EDIT_BRANCH_TOKEN: &str = "(Enter to save, Esc to cancel)";

/// The index and value of the first `ConfigValueKind::String` row of a cache's
/// Defaults list — **for the render-escape probe, and DERIVED rather than
/// spelled** (21-30 T1).
///
/// The probe's string-edit arrange has to set `defaults_editing` to an index
/// the render will actually dispatch on, and seed the buffer with the value the
/// operator would be editing. Both must come from the config the fixture
/// populated: 21-28 measured that an arrange which spells the untrusted value
/// itself puts that value in the chrome baseline too, so the arrival difference
/// is zero and the state reports "did not arrive" while visibly leaking. This
/// returns `None` for a cache with no config — which is exactly what
/// `chrome_ctx` is — so the baseline draws chrome only.
#[cfg(test)]
pub(super) fn first_string_entry(cache: &super::ProjectViewCache) -> Option<(usize, String)> {
    entries_for_cache(cache)
        .into_iter()
        .enumerate()
        .find(|(_, entry)| matches!(entry.kind, ConfigValueKind::String))
        .map(|(idx, entry)| (idx, entry.value))
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

    // ── Defaults tab help (quick task 260909-s0n) ─────────────────────────

    /// A `.planning/config.json` with EVERY key the Defaults tab reads set.
    ///
    /// **Populated, not `GsdConfig::default()`** — and the difference is not
    /// cosmetic. Under an all-`None` config every layered option resolves to
    /// `ConfigValueKind::Null`, so `dropdown_options` returns an empty list for
    /// all but the three plain-`String` enums and a coverage assertion over
    /// choices certifies nothing. That is the same trap the `val_span` comment
    /// in `render_defaults_tab` records for the render probe, one layer up:
    /// a fixture that forgets to populate passes having exercised the empty
    /// branch. MEASURED here — the first version of
    /// `every_choice_bearing_entry_documents_exactly_its_dropdown_options`
    /// built from `GsdConfig::default()` and failed with
    /// "`workflow.specless_probe_fallback` documents the choice "true", which
    /// its dropdown cannot select (it offers [])".
    ///
    /// Going through `parse_gsd_config` rather than a struct literal means the
    /// fixture is also a check that this build's serde shape still reads a
    /// config written in GSD's own spelling — `_auto_chain_active`, the
    /// shape-varying `security_asvs_level`, and the rest.
    fn populated_gsd_config() -> crate::state_reader::config_json::GsdConfig {
        let raw = r#"{
            "mode": "yolo",
            "granularity": "coarse",
            "model_profile": "quality",
            "commit_docs": true,
            "parallelization": true,
            "search_gitignored": false,
            "brave_search": false,
            "firecrawl": false,
            "exa_search": true,
            "project_code": "GMM",
            "phase_naming": "sequential",
            "phase_id_convention": "milestone-prefixed",
            "claude_md_path": "./.claude/CLAUDE.md",
            "response_language": "English",
            "sub_repos": [],
            "git": {
                "branching_strategy": "phase",
                "base_branch": "master",
                "phase_branch_template": "gsd/phase-{phase}-{slug}",
                "milestone_branch_template": "gsd/{milestone}-{slug}",
                "quick_branch_template": "gsd/quick-{slug}"
            },
            "workflow": {
                "research": true,
                "plan_check": true,
                "verifier": true,
                "nyquist_validation": false,
                "auto_advance": false,
                "_auto_chain_active": false,
                "node_repair": true,
                "node_repair_budget": 2,
                "ui_phase": true,
                "ui_safety_gate": true,
                "text_mode": false,
                "research_before_questions": false,
                "discuss_mode": "discuss",
                "skip_discuss": false,
                "use_worktrees": true,
                "subagent_timeout": 300000,
                "pattern_mapper": true,
                "ai_integration_phase": true,
                "tdd_mode": false,
                "code_review": true,
                "code_review_depth": "standard",
                "code_review_command": "my-review-tool --review",
                "ui_review": true,
                "api_coverage_gate": true,
                "windows_enforce": true,
                "specless_probe_fallback": true,
                "assumption_delta": true,
                "test_gate_timeout": 600,
                "context_guard_mode": "warn",
                "plan_drift_precheck": true,
                "plan_chunked": false,
                "mvp_mode": false,
                "security_asvs_level": 1,
                "security_block_on": "high"
            },
            "hooks": { "context_warnings": true },
            "intel": { "enabled": true },
            "graphify": {
                "enabled": true,
                "build_timeout": 300,
                "graph_path": ".planning/graphs"
            },
            "claude_orchestration": {
                "enabled": false,
                "execution_backend": "auto",
                "min_agent_sdk_version": "0.3.149"
            },
            "statusline": {
                "show_context_tokens": true,
                "state_format": "compact",
                "show_git": true
            },
            "dynamic_routing": {
                "provider_escalation": false,
                "max_escalations": 1
            },
            "review": { "reviewer_instances": {} },
            "external_job": {
                "submit_timeout_ms": 30000,
                "poll_timeout_ms": 15000,
                "artifact_dir": "Artifacts/jobs"
            },
            "capabilities": {
                "strict_known_registries": false,
                "auto_update": false
            }
        }"#;
        crate::state_reader::config_json::parse_gsd_config(raw)
            .expect("the populated Defaults fixture parses")
    }

    /// Every option the Defaults tab lists, built the way the tab builds them
    /// from a config with nothing unset.
    fn all_config_entries() -> Vec<ConfigEntry> {
        build_defaults_entries(&populated_gsd_config(), None)
    }

    /// The number of `push` call sites in `build_defaults_entries`, MEASURED at
    /// the time the help was authored. It is asserted rather than trusted so a
    /// 74th option cannot slip past the coverage assertions below by being
    /// added to a list nobody counted.
    const DEFAULTS_OPTION_COUNT: usize = 73;

    #[test]
    fn every_config_entry_carries_a_non_empty_summary() {
        let entries = all_config_entries();
        assert_eq!(
            entries.len(),
            DEFAULTS_OPTION_COUNT,
            "the Defaults tab's option count changed; every new option needs its own ConfigHelp \
             and this constant needs re-measuring"
        );
        for entry in &entries {
            assert!(
                entry.help.summary.chars().count() >= 20,
                "`{}` has no real help summary (got {:?}) — a placeholder that echoes the key \
                 teaches a reader nothing",
                entry.key,
                entry.help.summary
            );
        }
    }

    /// The link that keeps the list a human READS identical to the list the
    /// editor OFFERS.
    ///
    /// A `choices` entry naming a value `dropdown_options` does not have would
    /// document a choice nobody can pick — help that is worse than none,
    /// because it reads as authoritative. For an `Enum` the two lists must be
    /// equal AND in the same order, since the pane and the popup both present
    /// them in that order.
    ///
    /// Proved fail-first by renaming `model_profile`'s `"budget"` choice to
    /// `"cheap"`:
    ///
    /// ```text
    /// assertion `left == right` failed: `model_profile` documents
    /// ["quality", "balanced", "cheap", "adaptive", "inherit"] but its dropdown offers
    /// ["quality", "balanced", "budget", "adaptive", "inherit"]
    /// ```
    #[test]
    fn every_choice_bearing_entry_documents_exactly_its_dropdown_options() {
        let entries = all_config_entries();
        let mut enum_entries = 0usize;
        let mut choice_bearing = 0usize;

        for entry in &entries {
            let offered = dropdown_options(&entry.kind);
            let offered: Vec<&str> = offered.iter().map(String::as_str).collect();
            let documented: Vec<&str> = entry.help.choices.iter().map(|(v, _)| *v).collect();

            for (value, explanation) in entry.help.choices {
                assert!(
                    !explanation.trim().is_empty(),
                    "`{}` documents the choice {value:?} with an empty explanation",
                    entry.key
                );
            }

            if matches!(entry.kind, ConfigValueKind::Enum(_)) {
                enum_entries += 1;
                assert_eq!(
                    documented, offered,
                    "`{}` documents {documented:?} but its dropdown offers {offered:?} — the list \
                     a human READS must be the list the editor OFFERS",
                    entry.key
                );
            } else {
                for value in &documented {
                    assert!(
                        offered.contains(value),
                        "`{}` documents the choice {value:?}, which its dropdown cannot select \
                         (it offers {offered:?})",
                        entry.key
                    );
                }
            }

            if !documented.is_empty() {
                choice_bearing += 1;
            }
        }

        assert_eq!(
            enum_entries, 6,
            "the Defaults tab's Enum-kinded option count changed; each one needs a per-value \
             explanation"
        );
        // Without this the `else` branch above could be reached by nothing and
        // the membership rule would be asserted of no entry at all.
        assert!(
            choice_bearing > enum_entries,
            "no non-Enum option documents its choices, so the membership branch is vacuous"
        );
    }

    /// T-S0N-02: widening the popup for explanation text must not let it
    /// escape the content area, and must measure CHARACTERS rather than bytes.
    ///
    /// The lower bound is what makes this a control rather than a check a
    /// `return 0` would satisfy: at a wide area the popup has to be wide enough
    /// to actually hold the explanation the widening exists for.
    #[test]
    fn the_dropdown_popup_never_exceeds_the_area_width() {
        let entries = all_config_entries();
        let entry = entries
            .iter()
            .find(|e| e.key == "model_profile")
            .expect("model_profile is one of the Defaults tab's options");
        let options = dropdown_options(&entry.kind);
        assert_eq!(options.len(), 5, "model_profile offers five values");
        let title = format!(" {} ", entry.key);

        for area_width in [40u16, 80, 200] {
            let popup_w = dropdown_popup_width(&options, &entry.help, &title, area_width);
            assert!(
                popup_w <= area_width,
                "at area width {area_width} the popup computed {popup_w}, which is wider than the \
                 area it must sit inside"
            );
        }

        let widest_row = entry
            .help
            .choices
            .iter()
            .map(|(value, explanation)| {
                DROPDOWN_MARKER_WIDTH
                    + value.chars().count()
                    + CHOICE_SEPARATOR.chars().count()
                    + explanation.chars().count()
            })
            .max()
            .expect("model_profile documents its five choices");
        assert!(
            dropdown_popup_width(&options, &entry.help, &title, 200) as usize >= widest_row,
            "at a 200-column area the popup must be wide enough for its widest explained option \
             ({widest_row} characters)"
        );
    }

    /// Draw ONLY the Defaults tab through a `TestBackend` and join the
    /// resulting buffer's cell symbols, one line per terminal row — the same
    /// shape as `render_escape_guard.rs`'s `render_into_probe_buffer`, mirrored
    /// here rather than made public over there.
    ///
    /// The cache it builds is POPULATED (see [`populated_gsd_config`]). A probe
    /// that leaves `defaults_config` as `None` paints "No config loaded" and
    /// certifies the empty branch, which is why every caller below first
    /// asserts that a known option KEY reached a cell.
    fn render_defaults_to_text(width: u16, height: u16, selected: usize) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut ctx = test_ctx();
        {
            let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
            cache.defaults_config = Some(populated_gsd_config());
            cache.defaults_selected = selected;
        }
        let screen = DetailScreen::new(TEST_ALIAS.to_string());

        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("TestBackend terminal");
        terminal
            .draw(|frame| screen.render_defaults_tab(frame, frame.area(), &ctx))
            .expect("draw the Defaults tab");
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

    /// Collapse every run of whitespace to one space.
    ///
    /// The pane wraps with `Wrap { trim: true }`, so a summary that runs past
    /// the pane's width arrives in the buffer split across two rows with the
    /// space at the break replaced by a row boundary. Comparing squeezed text
    /// asserts the SENTENCE reached the terminal without also asserting where
    /// it happened to break.
    fn squeeze_ws(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn the_help_pane_renders_the_selected_entrys_summary() {
        let entries = all_config_entries();
        let first_summary = squeeze_ws(entries[0].help.summary);
        let model_idx = entries
            .iter()
            .position(|e| e.key == "model_profile")
            .expect("model_profile is one of the Defaults tab's options");
        let model_summary = squeeze_ws(entries[model_idx].help.summary);

        let at_first = squeeze_ws(&render_defaults_to_text(120, 40, 0));
        assert!(
            at_first.contains(entries[0].key),
            "the fixture is not populated — the tab painted its empty state, so nothing below \
             this line is about the help pane"
        );
        assert!(
            at_first.contains(&first_summary),
            "the pane did not draw entry 0's summary: {first_summary:?}"
        );

        let at_model = squeeze_ws(&render_defaults_to_text(120, 40, model_idx));
        assert!(
            at_model.contains("model_profile"),
            "the fixture is not populated at the model_profile cursor"
        );
        assert!(
            at_model.contains(&model_summary),
            "the pane did not draw model_profile's summary: {model_summary:?}"
        );
        // The control. Without it this test passes on a pane that draws entry
        // 0's summary forever and never follows the cursor.
        assert!(
            !at_model.contains(&first_summary),
            "entry 0's summary is still on screen with the cursor on model_profile — the pane \
             does not follow the selection"
        );
    }

    #[test]
    fn a_short_area_keeps_the_option_list() {
        let entries = all_config_entries();
        let first_summary = squeeze_ws(entries[0].help.summary);

        let short = render_defaults_to_text(120, 10, 0);
        let row = short
            .lines()
            .find(|line| line.contains(entries[0].key))
            .expect("below the ID-04 floor the option list keeps the whole area");
        assert!(
            row.contains(&entries[0].value),
            "the option row lost its value: {row:?}"
        );
        assert!(
            !squeeze_ws(&short).contains(&first_summary),
            "the help pane was drawn below the ID-04 floor, taking rows the list needs"
        );

        // Nothing panics at any height down to 3, on either side of the floor.
        for height in 3..=HELP_PANE_FLOOR + 2 {
            let _ = render_defaults_to_text(120, height, 0);
        }
    }

    #[test]
    fn every_summary_is_within_the_pane_budget_and_is_not_a_restatement_of_the_key() {
        use std::collections::HashSet;

        for entry in &all_config_entries() {
            let summary = entry.help.summary;
            assert!(
                summary.chars().count() <= 160,
                "`{}` has a {}-character summary; a paragraph in a TUI footer reads as noise",
                entry.key,
                summary.chars().count()
            );

            let key_tokens: HashSet<String> = entry
                .key
                .split(['.', '_'])
                .map(|token| token.to_ascii_lowercase())
                .collect();
            let novel_words = summary
                .split_whitespace()
                .map(|word| {
                    word.trim_matches(|c: char| !c.is_alphanumeric())
                        .to_ascii_lowercase()
                })
                .filter(|word| !word.is_empty() && !key_tokens.contains(word))
                .count();
            assert!(
                novel_words >= 4,
                "`{}` says only {novel_words} words its own key does not already say ({summary:?}) \
                 — a summary that re-spells the key teaches nothing",
                entry.key
            );
        }
    }

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

    // -----------------------------------------------------------------------
    // CR-01: the resume argv, proven at the argv because a test cannot spawn a
    // terminal emulator in CI
    // -----------------------------------------------------------------------

    /// Command interpreter binaries, spelled from halves so this array is not
    /// itself a hit for `text`'s interpreter census (which walks `src/`).
    const INTERPRETER_HEADS: [&str; 8] = ["s", "bas", "zs", "das", "ks", "fis", "cs", "tcs"];
    /// The tail every entry of [`INTERPRETER_HEADS`] takes. Meaningless alone.
    const INTERPRETER_TAIL: &str = "h";
    /// The flag by which an interpreter is handed a program to PARSE. Split for
    /// the same reason.
    const INTERPRETER_COMMAND_FLAG_HEAD: &str = "-";
    /// See [`INTERPRETER_COMMAND_FLAG_HEAD`].
    const INTERPRETER_COMMAND_FLAG_TAIL: &str = "c";

    fn interpreter_binaries() -> Vec<String> {
        INTERPRETER_HEADS
            .iter()
            .map(|head| format!("{head}{INTERPRETER_TAIL}"))
            .collect()
    }

    /// Session ids that are hostile in every way this codebase can name: the
    /// look-alike corpus by IMPORT (never respelled — D-21-6), one fixture per
    /// shell metacharacter class, and — since round 11 — one fixture per
    /// OPTION-LOOKALIKE class.
    ///
    /// # Why the option-lookalike block exists (gaps[0]'s second half, D-21-46)
    ///
    /// Until round 11 this corpus carried eighteen fixtures and **not one of
    /// them began with a hyphen**. Every fixture named a shell metacharacter
    /// class, because the defect the corpus was written against was a shell
    /// interpreter in the path. Round 10 deleted that interpreter and traded
    /// CWE-78 for CWE-88 — and this corpus could not tell, because
    /// `-r, --resume [value]` is an OPTIONAL-value option and only an
    /// id whose first byte is `-` can exercise it. The control it certified
    /// therefore passed against a build shipping the defect, which is not a
    /// certificate at all.
    ///
    /// The complicity was proven rather than asserted: these ten fixtures were
    /// added FIRST, with no production line touched, and
    /// `the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element` was
    /// captured still GREEN against the unfixed construction.
    fn hostile_session_ids() -> Vec<String> {
        let mut ids: Vec<String> = crate::test_support::LOOK_ALIKE_PAIRS
            .iter()
            .map(|(_, hostile)| (*hostile).to_string())
            .collect();
        ids.extend(
            [
                "a'b",                  // single quote — terminates the quoting
                "a\"b",                 // double quote
                "a;b",                  // command separator
                "a&&b",                 // conditional chain
                "a|b",                  // pipe
                "a`b`c",                // backtick substitution
                "a$(b)c",               // dollar-parenthesis substitution
                "a\nb",                 // newline — a statement separator
                "a\u{7}b",              // a NUL-free control character
                "a b",                  // a bare space: must stay ONE element
                "'; rm -rf / #",        // the whole escape, assembled
            ]
            .iter()
            .map(|raw| (*raw).to_string()),
        );
        ids.extend(
            [
                // --- Option lookalikes (round 11, CWE-88) -------------------
                // Each is a value that a receiving option parser reads as an
                // OPTION when it arrives as its own argv element.
                "-h",                             // the shortest possible option lookalike
                "--version",                      // probe A's payload: measured to FIRE at claude 2.1.248
                "--dangerously-skip-permissions", // the payload probe A stands in for
                "--print",                        // an option that changes the program's whole mode
                "-",                              // a bare hyphen: the degenerate case
                "-r",                             // the SHORT spelling of the option being injected into
                "--resume",                       // the option's own name, so the id can impersonate it
                "--settings=/tmp/x.json",         // an option that already carries a fused value
                "a=b",                            // NOT an option: the fusion character inside an id,
                // which must still arrive WHOLE (capability direction)
                "--add-dir", // an option taking a path the attacker chooses
            ]
            .iter()
            .map(|raw| (*raw).to_string()),
        );
        ids
    }

    /// **CR-01, proven where it can be proven.** A test cannot spawn a terminal
    /// emulator in CI, so the property is asserted at the ARGV: the vector
    /// [`resume_terminal_argv`] hands to `std::process::Command` carries the
    /// session id as one opaque element and contains no parser that could read
    /// a metacharacter in it as syntax.
    ///
    /// The four assertions are ordered so the first failure names the defect
    /// rather than a symptom of it.
    ///
    /// # Assertion (1) rewritten 2026-08-27 (21-31)
    ///
    /// It used to require an argv element **byte-identical to the raw id**.
    /// That is no longer the shape: `resume_terminal_argv` now FUSES the id to
    /// its option name (`--resume=<id>`) so that `claude`'s own option parser
    /// binds it as a value rather than reading a leading `-` as a new option
    /// (CWE-88). The id therefore never stands alone as an element, and the
    /// capability property it was really asserting — the id arrives WHOLE — is
    /// now checked on the suffix after the fused element's first `=`.
    /// Assertions (2), (3) and (4) are untouched: no interpreter binary, no
    /// interpreter command-string flag, constant arity. They still hold and
    /// they still name a real class, so they are kept rather than deleted.
    ///
    /// Keeping this control honest mattered here for a second reason. Under
    /// the OLD assertion (1), the round-11 fixture `"--resume"` made this test
    /// red — not because it detected the injection, but because the builder
    /// emitted a literal `--resume` element of its own and the id was spelled
    /// the same, so a count of string equalities read 2. That was a collision
    /// with this file's own literal masquerading as a finding; the fused shape
    /// removes it.
    #[test]
    fn the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element() {
        let interpreters = interpreter_binaries();
        let command_flag =
            format!("{INTERPRETER_COMMAND_FLAG_HEAD}{INTERPRETER_COMMAND_FLAG_TAIL}");

        // Non-vacuity: the corpus must actually be hostile, or every assertion
        // below would hold for an all-ASCII fixture set that proves nothing.
        assert!(
            hostile_session_ids()
                .iter()
                .any(|id| id.chars().any(|c| "'\";&|`$\n".contains(c))),
            "the fixture set carries no shell metacharacter at all, so this \
             control would pass against the construction it exists to reject"
        );

        let mut arities = std::collections::BTreeSet::new();
        for term in ["kitty", "alacritty", "gnome-terminal", "xterm", "/opt/wat"] {
            for raw in hostile_session_ids() {
                let sid = Untrusted::from_untrusted_source(raw.clone());
                let argv = resume_terminal_argv(term, &sid);
                arities.insert(argv.len());

                // (1) The id is carried EXACTLY ONCE and WHOLE, as the suffix
                //     of the fused option element. Byte-identity is the
                //     capability half: an escaped id resumes nothing.
                let carried = argv
                    .iter()
                    .filter(|element| element.split_once('=').map(|(_, v)| v) == Some(raw.as_str()))
                    .count();
                assert_eq!(
                    carried, 1,
                    "the session id {raw:?} must be carried by exactly ONE \
                     argv element as the suffix after that element's first \
                     `=`; it was carried {carried} times in {argv:?}. Zero \
                     means the id was interpolated into some larger string — \
                     which is a program a parser will read, not an argument \
                     `execve` hands over unread."
                );

                // (2) No element is a command interpreter binary.
                for element in &argv {
                    let stem = element.rsplit('/').next().unwrap_or(element);
                    assert!(
                        !interpreters.iter().any(|binary| binary == stem),
                        "argv element {element:?} is a command interpreter. \
                         While an interpreter sits in this path every byte of \
                         the session id is a candidate token, and quoting for \
                         it is the weaker answer: it makes correctness depend \
                         on the escaper being right about every metacharacter \
                         of every interpreter. Full argv: {argv:?}"
                    );
                }

                // (3) No element is an interpreter's command-string flag.
                assert!(
                    !argv.iter().any(|element| element == &command_flag),
                    "argv carries {command_flag:?}, the flag by which an \
                     interpreter is handed a program to PARSE. Its presence \
                     means the next element is a program, not an argument. \
                     Full argv: {argv:?}"
                );
            }
        }

        // (4) Arity is constant: a hostile id adds, removes or merges nothing.
        assert_eq!(
            arities.len(),
            1,
            "the argv arity varied across inputs ({arities:?}), so some id \
             changed the SHAPE of the vector rather than just one element of \
             it. A value that can change the arity is a value being parsed."
        );
    }

    /// **CWE-88 closed at the SHAPE of the vector** (T-21-31-01, T-21-31-03,
    /// D-21-47).
    ///
    /// # The property, and why it is not a list of forbidden characters
    ///
    /// The assertion is CONTENT INDEPENDENCE: the number of argv elements that
    /// begin with `-` must be the SAME for every session id in the corpus. A
    /// list of forbidden first bytes is an enumeration and can always be one
    /// entry short — that is the defect this control exists to close, arriving
    /// one level up — whereas a property over the whole vector cannot be. A
    /// value able to add an option-shaped element to a vector is a value the
    /// receiving parser will read as an option.
    ///
    /// # The parser this is about
    ///
    /// Not a shell — round 10 removed that one. `claude`'s OWN option parser,
    /// which was in the path the whole time. Measured at `claude` 2.1.248 with
    /// stdin at `/dev/null`:
    ///
    /// | Probe | Command | Observed |
    /// |---|---|---|
    /// | A | `claude --resume --version` | `2.1.248 (Claude Code)`, exit 0 — **the injection firing** |
    /// | E | `claude --resume=<uuid>` | `No conversation found with session ID: <uuid>` — **the id bound as a value** |
    ///
    /// # Non-vacuity is asserted FIRST, and that is the point
    ///
    /// The corpus this control consumes was, until round 11, incapable of
    /// failing it: eighteen fixtures, not one beginning with a hyphen. The
    /// non-vacuity arm is what stops that state recurring silently.
    #[test]
    fn the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program() {
        let corpus = hostile_session_ids();

        // --- Non-vacuity, asserted before anything else --------------------
        assert!(
            corpus.iter().any(|id| id.starts_with('-')),
            "the fixture set contains NO id beginning with a hyphen, so this \
             control cannot fail for the defect it names (CWE-88) and proves \
             nothing. That is the exact complicity round 11 found in the \
             committed corpus: eighteen hostile fixtures, every one of them a \
             shell metacharacter class, certifying a claim about \
             option-shaped inputs."
        );
        assert!(
            corpus
                .iter()
                .any(|id| id.chars().any(|c| "'\";&|`$\n".contains(c))),
            "the fixture set carries no shell metacharacter at all, so the \
             interpreter-absence class this same corpus certifies would be \
             proven by nothing."
        );

        // The corpus shape, asserted rather than counted by hand.
        assert_eq!(
            corpus.len(),
            28,
            "hostile_session_ids() must carry 28 fixtures: 7 imported \
             LOOK_ALIKE_PAIRS + 11 shell-metacharacter fixtures + 10 \
             option-lookalikes."
        );
        // MEASURED, not inherited: nine of the ten option lookalikes begin
        // with a hyphen. The tenth, `a=b`, is the fusion-character fixture and
        // deliberately does not — it exists for the CAPABILITY direction, to
        // prove an id containing `=` still arrives whole.
        let hyphen_leading = corpus.iter().filter(|id| id.starts_with('-')).count();
        assert!(
            hyphen_leading >= 9,
            "only {hyphen_leading} fixtures begin with a hyphen; the \
             option-lookalike block contributes nine and every one of them is \
             a value `claude`'s parser reads as an option when it arrives as \
             its own argv element."
        );

        // --- Content independence ------------------------------------------
        let mut by_count: std::collections::BTreeMap<usize, std::collections::BTreeSet<String>> =
            std::collections::BTreeMap::new();
        for term in ["kitty", "alacritty", "gnome-terminal", "xterm", "/opt/wat"] {
            for raw in &corpus {
                let sid = Untrusted::from_untrusted_source(raw.clone());
                let argv = resume_terminal_argv(term, &sid);
                let leading = argv.iter().filter(|e| e.starts_with('-')).count();
                by_count.entry(leading).or_default().insert(raw.clone());
            }
        }
        let observed: std::collections::BTreeSet<usize> = by_count.keys().copied().collect();
        assert_eq!(
            observed.len(),
            1,
            "the number of argv elements beginning with `-` DEPENDS ON THE \
             SESSION ID. Observed counts {observed:?}; the ids that produced \
             each: {by_count:?}. A value able to add an option-shaped element \
             to a vector is a value the receiving parser will read as an \
             option — measured at `claude` 2.1.248, `claude --resume \
             --version` prints `2.1.248 (Claude Code)` and exits 0, which is \
             CWE-88 firing. Fuse the id to its option name in ONE element \
             (`--resume=<id>`); do NOT insert a `--` separator, which was \
             measured to delete the resume capability entirely."
        );

        // --- The fused shape, in both directions ----------------------------
        // Ordered so the FIRST failure names the defect rather than a symptom.
        let mut arities = std::collections::BTreeSet::new();
        for term in ["kitty", "alacritty", "gnome-terminal", "xterm", "/opt/wat"] {
            for raw in &corpus {
                let sid = Untrusted::from_untrusted_source(raw.clone());
                let argv = resume_terminal_argv(term, &sid);
                arities.insert(argv.len());

                // (1) Exactly ONE element carries the fused option prefix.
                let fused: Vec<&String> = argv
                    .iter()
                    .filter(|e| e.starts_with(RESUME_OPTION_FUSED_PREFIX))
                    .collect();
                assert_eq!(
                    fused.len(),
                    1,
                    "exactly one element of {argv:?} must begin with \
                     {RESUME_OPTION_FUSED_PREFIX:?}. Zero means the session id \
                     {raw:?} is travelling as its own argv element again, \
                     where `-r, --resume [value]`'s OPTIONAL value lets a \
                     leading `-` make it a NEW OPTION of `claude` (CWE-88)."
                );

                // (2) CAPABILITY: the value arrives WHOLE. The suffix after
                //     the FIRST `=` is byte-identical to the raw id.
                let suffix = fused[0]
                    .split_once('=')
                    .expect("the fused element contains the fusion character")
                    .1;
                assert_eq!(
                    suffix,
                    sid.as_raw_for_logic_only(),
                    "the id `claude` will receive is {suffix:?} but the \
                     operator selected {raw:?}. An id that does not arrive \
                     WHOLE resumes nothing: measured at `claude` 2.1.248, \
                     probe E — `claude \
                     --resume=550e8400-e29b-41d4-a716-446655440000` reports \
                     `No conversation found with session ID: \
                     550e8400-e29b-41d4-a716-446655440000`, i.e. the value \
                     bound and looked up. Splitting on the LAST `=` rather \
                     than the first would truncate the id `a=b`, which is why \
                     that fixture is in the corpus. Full argv: {argv:?}"
                );

                // (3) The id never stands ALONE as its own element — the state
                //     this fix exists to remove.
                assert!(
                    !argv.iter().any(|e| e == raw),
                    "the session id {raw:?} appears as a standalone element of \
                     {argv:?}. Standing alone is exactly what lets an option \
                     parser reach it as syntax; fused to its option name it is \
                     that option's value and its first byte is data."
                );
            }
        }

        // (4) Arity is constant: a hostile id adds, removes or merges nothing.
        assert_eq!(
            arities.len(),
            1,
            "the argv arity varied across inputs ({arities:?}), so some id \
             changed the SHAPE of the vector rather than one element of it. A \
             value that can change the arity is a value being parsed."
        );
        assert_eq!(
            arities.iter().next().copied(),
            Some(3),
            "the fused argv is exactly three elements — separator, program \
             name, `--resume=<id>` — and {arities:?} is not that. A fourth \
             element is how the standalone untrusted id came back."
        );
    }

    /// The `/proc/<pid>/cmdline` wire encoding of an argv this build built.
    ///
    /// Element 0 of a terminal argv is the emulator's own program separator
    /// (`-e` / `--`). The EMULATOR consumes it and it never reaches the child,
    /// so it is dropped here. Element 1 — the literal program name — becomes
    /// the child's `argv[0]`, and it is taken from the PRODUCER'S OUTPUT rather
    /// than re-spelled, so this encoding cannot drift from what is emitted.
    ///
    /// The kernel presents `/proc/<pid>/cmdline` as NUL-separated with a
    /// trailing NUL, which is what is reproduced here.
    fn proc_cmdline_encoding(argv: &[String]) -> Vec<u8> {
        let elements: Vec<&[u8]> = argv.iter().skip(1).map(String::as_bytes).collect();
        nul_join_cmdline(&elements)
    }

    /// NUL-join argv elements into the `/proc/<pid>/cmdline` byte encoding,
    /// with the trailing NUL the kernel emits.
    ///
    /// Used by both round-trip arms: the FUSED arm reaches it through
    /// [`proc_cmdline_encoding`] with the producer's real output, and the SPLIT
    /// arm calls it directly with an explicitly constructed cmdline. Sharing it
    /// means both arms carry the same NUL-free precondition.
    ///
    /// # Why this takes `&[&[u8]]` and not `&[&str]` (21-36)
    ///
    /// It took `&[&str]` until round 13, and that TYPE was the defect rather
    /// than any fixture missing from a list. `/proc/<pid>/cmdline` is a byte
    /// string — the kernel imposes no encoding on it — and
    /// [`crate::session_detector::session_id_in_cmdline`] has taken `&[u8]` all
    /// along. A `&str`-typed harness cannot CONSTRUCT an ill-formed-UTF-8
    /// cmdline at all, so the whole encoding class was not "an unenumerated
    /// fixture": it was unrepresentable, and no amount of adding fixtures to a
    /// `&str` list could ever have reached it. The harness was the narrower
    /// type, and the harness is what changed.
    fn nul_join_cmdline(elements: &[&[u8]]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for element in elements {
            // A NUL-bearing element would TRUNCATE this encoding, and the
            // round trip would then assert on a value it never actually
            // round-tripped — passing vacuously (T-21-35-07). Fail loudly
            // instead, at the fixture that did it.
            assert!(
                !element.contains(&0),
                "the argv element {element:?} carries a NUL byte. NUL is the \
                 SEPARATOR of this encoding, so the element would be split in \
                 two and everything after the NUL would be read as a separate \
                 argument: the assertion below would then be checking a value \
                 this test invented rather than one that survived the wire."
            );
            bytes.extend_from_slice(element);
            bytes.push(0);
        }
        bytes
    }

    // ======================================================================
    // THE ROUND-TRIP CONTROLS — and why a one-sided control missed this class
    //
    // The property these two tests assert is a property of the PAIR, not of
    // either function: *what this build emits, this build must be able to read
    // back.* Neither `resume_terminal_argv` nor `session_id_in_cmdline` can be
    // wrong on its own here — each is individually correct — and that is
    // precisely why no control over either one could see the defect.
    //
    // What happened: round 11 FUSED the id to its option name at the producer
    // (`--resume=<id>`), correctly, to close CWE-88. That changed the WIRE
    // FORMAT the consumer parses. Every control in this repository lived on one
    // side or the other — the producer's controls asserted the argv's shape,
    // and the consumer had no parser control at all — so NO CONTROL SPANNED
    // BOTH. A build that had silently lost its resume detection passed the
    // whole gate green, for a full verification pass.
    //
    // The two modules spell the same option name INDEPENDENTLY:
    // `RESUME_OPTION_FUSED_PREFIX` here, and `RESUME_OPTION_NAME` /
    // `RESUME_OPTION_FUSED_PREFIX` in `crate::session_detector`. Nothing in the
    // type system couples them and nothing ever will — they are two byte
    // literals in two modules. These tests are the ONLY thing coupling them
    // (T-21-35-02), which is why they consume the real producer's output rather
    // than re-spelling the option name a third time.
    //
    // A control exercising only one side will miss the next instance of this.
    //
    // ----------------------------------------------------------------------
    // 2026-08-28 (21-37, WR-05): one of the two controls this block described
    // was a DUPLICATE, and it is gone. The safety of that was OBSERVED, not
    // argued.
    //
    // `the_argv_this_build_emits_is_an_argv_...` (named in truncated form here
    // and in `session_detector`'s two re-pointed citations, so that no whole
    // spelling of a test that no longer exists survives in the tree) iterated
    // five terminals x the 28 fixtures through the real producer, the real
    // encoder and the real consumer and asserted byte-identity. The FUSED arm
    // of `a_session_id_survives_the_round_trip_in_both_wire_forms` iterates the
    // same 28 fixtures x the same five terminals through the same three
    // functions and asserts the same thing, with the loop nesting swapped. The
    // first was entirely subsumed by the second.
    //
    // A subsumption ARGUMENT is not what removed it. Deleting a control in the
    // round that exists because controls could not fail is precisely the move
    // that requires proof, so the defect the pair exists to catch was PLANTED —
    // the parser's fused-long-resume branch made unreachable — and BOTH
    // survivors were observed red before anything was deleted:
    //
    //   * `a_session_id_survives_the_round_trip_in_both_wire_forms`
    //       FUSED form: the id "demo\u{200b}" ... could not be read back
    //       left: None   right: Some("demo\u{200b}")
    //   * `the_round_trip_property_holds_for_every_generated_byte_string`
    //       [refused-a-value-that-must-round-trip] 2655 occurrence(s)
    //
    // The defect was then restored and both went green again. The verbatim
    // output is in this round's SUMMARY.
    //
    // The two survivors certify DIFFERENT things and both are load-bearing:
    // the enumerated one pins NAMED shapes, and the generated one reaches
    // shapes nobody named. Neither replaces the other.
    // ======================================================================

    /// **Both wire forms, because both are legitimate on the wire.**
    ///
    /// The FUSED arm is driven from the REAL producer, unchanged — that is the
    /// shape this TUI emits. The SPLIT arm is constructed EXPLICITLY, because
    /// no producer in this build emits it any more: it is what a human typing
    /// `claude --resume <id>` by hand, or any launcher that is not this TUI,
    /// still produces. Inferring it from the fused arm would assert *about* the
    /// claim rather than asserting it, and dropping it would silently re-break
    /// detection for every session not started here (D-21-66).
    #[test]
    fn a_session_id_survives_the_round_trip_in_both_wire_forms() {
        let corpus = hostile_session_ids();

        // Non-vacuity: the corpus shape, asserted rather than assumed, so a
        // corpus that shrank to nothing could not make this pass silently.
        assert_eq!(
            corpus.len(),
            28,
            "hostile_session_ids() must carry 28 fixtures: 7 imported \
             LOOK_ALIKE_PAIRS + 11 shell-metacharacter fixtures + 10 \
             option-lookalikes. A corpus that shrank would make both arms below \
             pass over fewer shapes than they claim."
        );

        for raw in &corpus {
            // --- FUSED: the real producer's own output ----------------------
            for term in ["kitty", "alacritty", "gnome-terminal", "xterm", "/opt/wat"] {
                let sid = Untrusted::from_untrusted_source(raw.clone());
                let argv = resume_terminal_argv(term, &sid);
                let wire = proc_cmdline_encoding(&argv);
                let read_back = crate::session_detector::session_id_in_cmdline(&wire);

                assert_eq!(
                    read_back.as_ref().map(|id| id.as_raw_for_logic_only()),
                    Some(raw.as_str()),
                    "FUSED form: the id {raw:?} was emitted by \
                     resume_terminal_argv({term:?}) as {argv:?} and could not \
                     be read back out of the wire bytes it produces. This is \
                     the shape this TUI itself emits, so a failure here means \
                     the TUI cannot re-resume its own session."
                );
            }

            // --- SPLIT: constructed explicitly, not inferred ------------------
            // Program name, bare option name, then the fixture: the two-element
            // window a hand-typed `claude --resume <id>` puts on the wire.
            let wire =
                nul_join_cmdline(&[b"claude".as_slice(), b"--resume".as_slice(), raw.as_bytes()]);
            let read_back = crate::session_detector::session_id_in_cmdline(&wire);

            assert_eq!(
                read_back.as_ref().map(|id| id.as_raw_for_logic_only()),
                Some(raw.as_str()),
                "SPLIT form: the id {raw:?} on the wire as \
                 [\"claude\", \"--resume\", {raw:?}] could not be read back. \
                 This build no longer EMITS this shape, but it is still what a \
                 human typing the command by hand produces, and it is the shape \
                 every session not started by this TUI arrives in. Dropping it \
                 would delete those sessions from the Sessions tab with no \
                 message to the operator — under-detection, and SILENT."
            );
        }
    }

    // ======================================================================
    // THE GENERATED CERTIFICATE — the SHAPE of the corpus is what changed
    //
    // The two controls above are real and they stay. What they cannot be is
    // the thing that CERTIFIES the round-trip claim, because they consume an
    // ENUMERATION. An enumeration can only ever fail on a class somebody
    // thought to enumerate, and this phase has now watched that mechanism run
    // three times: round 10 certified a CWE-88 fix with a corpus containing no
    // leading-hyphen fixture; round 12 certified a byte-identity claim with a
    // corpus containing no whitespace-padded fixture; and the non-UTF-8 class
    // was not merely unenumerated but UNREPRESENTABLE, because the harness was
    // typed `&[&str]`. Each round the repair was to add the missing fixture,
    // which leaves the mechanism intact and guarantees the next round.
    //
    // So the claim below is not "these fixtures round-trip". It is a TOTAL
    // property with exactly two named refusal classes and no third outcome,
    // asserted over a deterministic GENERATOR rather than a list. The 28
    // fixtures are retained and are consumed by the generator as a seed
    // corpus — nothing that reads them today stops doing so.
    // ======================================================================

    /// The fixed seed. A generator whose inputs vary run to run makes a green
    /// gate un-trustworthy in the opposite direction from a vacuous one: it
    /// passes today over a space that is not the space it passed over
    /// yesterday. This seed is a literal so that the 4096 inputs below are
    /// byte-identical on every machine, in CI, forever.
    const GENERATOR_SEED: u64 = 0x21_36_5E_55_10_4E_C0_DE;

    /// How many byte strings the generator produces per run.
    const GENERATOR_CASES: usize = 4096;

    /// The length bound on a generated run of bytes.
    ///
    /// This is the mitigation for the one thing giving up a property-testing
    /// dependency actually costs: SHRINKING. A counterexample bounded at twelve
    /// bytes, printed as an explicit byte vector alongside its escaped
    /// rendering, is already small enough to read without a shrinker.
    const GENERATED_MAX_LEN: usize = 12;

    /// The whitespace units the `padded` shape draws from, spelled as BYTES so
    /// the multi-byte members are unambiguous and so no invisible character is
    /// pasted into this source file.
    ///
    /// Six are ASCII (space, tab, carriage return, line feed, vertical tab,
    /// form feed). The last two are the UTF-8 encodings of U+00A0 NO-BREAK
    /// SPACE (two bytes, `C2 A0`) and U+2028 LINE SEPARATOR (**three** bytes,
    /// `E2 80 A8`). Both carry Unicode `White_Space=yes`, which is the property
    /// `char::is_whitespace` — and therefore `str::trim` — is defined over, so
    /// both genuinely belong to the class the G1 floor counts.
    const GENERATED_WHITESPACE_UNITS: [&[u8]; 8] = [
        b" ",
        b"\t",
        b"\r",
        b"\n",
        b"\x0b",
        b"\x0c",
        b"\xc2\xa0",
        b"\xe2\x80\xa8",
    ];

    /// The byte prefixes the `prefixed` shape draws from. Five of the six begin
    /// with a hyphen — the class round 10's corpus missed entirely — and the
    /// sixth is the fusion character itself, which must arrive as data.
    const GENERATED_OPTION_PREFIXES: [&[u8]; 6] = [
        b"-",
        b"--",
        b"-r",
        b"=",
        b"--resume",
        b"--session-id",
    ];

    /// One xorshift64* step. Deterministic, seedable, and small enough to read
    /// in full — which is the point of not taking a dependency for it.
    ///
    /// The triple is shift-12-left, shift-25-right, shift-27-left over the
    /// state, followed by a multiply by Vigna's `0x2545F4914F6CDD1D` on the
    /// OUTPUT only, so the state sequence stays a pure xorshift. The state must
    /// never be zero; [`GENERATOR_SEED`] is not.
    fn next_random(state: &mut u64) -> u64 {
        let mut x = *state;
        x ^= x << 12;
        x ^= x >> 25;
        x ^= x << 27;
        *state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A run of unconstrained random bytes — the arm that reaches classes
    /// nobody enumerated.
    ///
    /// NUL is excluded, and only NUL: it is the SEPARATOR of the
    /// `/proc/<pid>/cmdline` encoding, so a NUL inside a value is not a value
    /// this wire can carry at all. That class is covered where it belongs, by
    /// [`nul_join_cmdline`]'s own precondition assertion.
    fn generated_uniform(state: &mut u64) -> Vec<u8> {
        let len = (next_random(state) % (GENERATED_MAX_LEN as u64 + 1)) as usize;
        (0..len)
            .map(|_| (next_random(state) % 255 + 1) as u8)
            .collect()
    }

    /// The `core` a compound shape wraps: one recursion, into a terminal arm
    /// only (`uniform` or a seed fixture). Bounded depth by construction.
    fn generated_core(state: &mut u64, seeds: &[String]) -> Vec<u8> {
        if next_random(state).is_multiple_of(2) {
            generated_uniform(state)
        } else {
            seeds[(next_random(state) % seeds.len() as u64) as usize]
                .as_bytes()
                .to_vec()
        }
    }

    /// One case of the mixture distribution.
    fn generated_shape(shape: u64, state: &mut u64, seeds: &[String]) -> Vec<u8> {
        match shape {
            // uniform — unrestricted bytes, no alphabet, no character class.
            0 => generated_uniform(state),
            // padded — whitespace run, core, whitespace run. The class G1 lives
            // in, and the class uniform bytes essentially never reach.
            1 => {
                let mut out = Vec::new();
                for _ in 0..(next_random(state) % 3 + 1) {
                    let unit = GENERATED_WHITESPACE_UNITS
                        [(next_random(state) % GENERATED_WHITESPACE_UNITS.len() as u64) as usize];
                    out.extend_from_slice(unit);
                }
                out.extend_from_slice(&generated_core(state, seeds));
                for _ in 0..(next_random(state) % 3 + 1) {
                    let unit = GENERATED_WHITESPACE_UNITS
                        [(next_random(state) % GENERATED_WHITESPACE_UNITS.len() as u64) as usize];
                    out.extend_from_slice(unit);
                }
                out
            }
            // prefixed — an option-shaped head on an arbitrary core.
            2 => {
                let mut out = GENERATED_OPTION_PREFIXES
                    [(next_random(state) % GENERATED_OPTION_PREFIXES.len() as u64) as usize]
                    .to_vec();
                out.extend_from_slice(&generated_core(state, seeds));
                out
            }
            // seed — one of the 28 retained fixtures, CONSUMED rather than
            // respelled (D-21-6).
            3 => seeds[(next_random(state) % seeds.len() as u64) as usize]
                .as_bytes()
                .to_vec(),
            // ascii — a run of printable ASCII, the boring shape a real id has.
            _ => {
                let len = (next_random(state) % (GENERATED_MAX_LEN as u64 + 1)) as usize;
                (0..len)
                    .map(|_| (0x20 + next_random(state) % 95) as u8)
                    .collect()
            }
        }
    }

    /// The input space of the round-trip claim: `count` byte strings drawn
    /// deterministically from a five-arm mixture at `seed`.
    ///
    /// `Vec<Vec<u8>>` and not `Vec<String>`, because the encoding class has to
    /// be expressible for the claim to be total — see [`nul_join_cmdline`].
    fn generated_session_id_bytes(seed: u64, count: usize) -> Vec<Vec<u8>> {
        let seeds = hostile_session_ids();
        let mut state = seed;
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            let shape = next_random(&mut state) % 5;
            out.push(generated_shape(shape, &mut state, &seeds));
        }
        out
    }

    /// One legitimate way a session id can appear on the `/proc` wire.
    ///
    /// A named, registered TABLE rather than two hand-written loops, so the
    /// property below is stated over a SET a later plan can extend by adding a
    /// row — rather than over whichever forms someone remembered.
    struct WireForm {
        name: &'static str,
        encode: fn(&[u8]) -> Vec<u8>,
    }

    /// The FUSED form: one argv element, prefix and value in the same element.
    ///
    /// The prefix is taken from this module's own
    /// [`RESUME_OPTION_FUSED_PREFIX`] rather than respelled, so a change to the
    /// producer's spelling reaches this table instead of drifting away from it.
    fn encode_fused(s: &[u8]) -> Vec<u8> {
        let mut element = RESUME_OPTION_FUSED_PREFIX.as_bytes().to_vec();
        element.extend_from_slice(s);
        nul_join_cmdline(&[b"claude".as_slice(), element.as_slice()])
    }

    /// The SPLIT form: bare option name, then the value as the next element.
    ///
    /// The bare name is DERIVED from the fused prefix by removing the fusion
    /// character, so this file still spells the option exactly once.
    fn encode_split(s: &[u8]) -> Vec<u8> {
        let bare = RESUME_OPTION_FUSED_PREFIX
            .strip_suffix('=')
            .expect("the fused prefix ends with the fusion character");
        nul_join_cmdline(&[b"claude".as_slice(), bare.as_bytes(), s])
    }

    /// The CLI's SHORT spelling of the resume option, measured at `claude`
    /// 2.1.250 where `--help` documents ONE option under two spellings,
    /// `-r, --resume [value]` (21-37, G2a).
    ///
    /// **Spelled here rather than imported from `crate::session_detector`, and
    /// that is deliberate.** The encoders in this table are the PRODUCER side
    /// of the round trip, and an encoder derived from the parser it checks can
    /// only ever agree with it — the same oracle-independence argument 21-36
    /// made for computing R1 and R2 from `std` rather than from a predicate the
    /// parser exports. The two long forms already derive from this module's own
    /// producer constant [`RESUME_OPTION_FUSED_PREFIX`] for exactly that
    /// reason. No producer in this build emits the short spelling — it is what
    /// a HUMAN types — so there is no producer constant to derive from, and the
    /// honest substitute is one constant carrying the measurement, with BOTH
    /// short encoders derived from it rather than respelling it twice.
    const RESUME_OPTION_SHORT_NAME: &str = "-r";

    /// The BARE SHORT form: `-r`, then the value as the next element.
    fn encode_short_split(s: &[u8]) -> Vec<u8> {
        nul_join_cmdline(&[
            b"claude".as_slice(),
            RESUME_OPTION_SHORT_NAME.as_bytes(),
            s,
        ])
    }

    /// The ATTACHED SHORT form: `-r<value>`, one element, no fusion character.
    fn encode_short_attached(s: &[u8]) -> Vec<u8> {
        let mut element = RESUME_OPTION_SHORT_NAME.as_bytes().to_vec();
        element.extend_from_slice(s);
        nul_join_cmdline(&[b"claude".as_slice(), element.as_slice()])
    }

    /// The registered wire forms the round-trip property is stated over.
    ///
    /// 21-36 made this a TABLE precisely so a later plan could extend it by
    /// adding a row rather than writing a second generator; 21-37 is that later
    /// plan, and the two short spellings are those rows. The generated property
    /// and the non-vacuity floor both iterate this function, so both cover the
    /// new forms with no further change.
    fn wire_forms() -> Vec<WireForm> {
        vec![
            WireForm {
                name: "fused",
                encode: encode_fused,
            },
            WireForm {
                name: "split",
                encode: encode_split,
            },
            WireForm {
                name: "short-split",
                encode: encode_short_split,
            },
            WireForm {
                name: "short-attached",
                encode: encode_short_attached,
            },
        ]
    }

    /// **The round-trip claim, stated so it has no third outcome** (21-36).
    ///
    /// For every generated byte string `s` and every registered [`WireForm`]:
    ///
    /// - if `s` is valid UTF-8 **and** trims to something non-empty, the parser
    ///   MUST return `Some(t)` with `t`'s bytes **byte-identical** to `s` — no
    ///   normalisation, no case folding, no trimming, no truncation, no cap;
    /// - otherwise the parser MUST return `None`, and `s` MUST fall in one of
    ///   exactly two named refusal classes: **R1**, not valid UTF-8; **R2**,
    ///   valid UTF-8 and empty after `trim`.
    ///
    /// Both directions are asserted. A non-refused `s` reading back `None`, a
    /// refused `s` reading back `Some`, and a byte mismatch each produce a
    /// distinct message naming the class. A disjunction with two named branches
    /// can be false; *"these 28 fixtures round-trip"* could not be.
    ///
    /// # The oracle is INDEPENDENT of the code it checks
    ///
    /// R1 and R2 are computed here from [`std::str::from_utf8`] and
    /// [`str::trim`] directly. The parser is deliberately NOT asked to export a
    /// shared `is_refused` predicate for this test to consume: an oracle that
    /// consumes the predicate it checks can only ever agree with it, which is
    /// this repository's own recorded lesson — see `Cargo.toml`'s two-crate
    /// rationale, where the production invisible-character class and its sweep
    /// oracle read two independently maintained derivations of the same
    /// standard for exactly this reason. `std` is the independent derivation
    /// here.
    ///
    /// # Why every violation is collected instead of panicking at the first
    ///
    /// This control's whole job is to be checked out at the commit that
    /// introduced it and observed RED. At that commit the parser violates the
    /// property in TWO distinct classes at once, and a test that panicked on
    /// the first one would show a reader only whichever class the corpus
    /// happened to reach first — the second would stay invisible until the
    /// first was fixed. So one exemplar per (class × wire form) is collected,
    /// with the rest counted, and the whole report is emitted in a single
    /// panic. Bounded by construction: at most six exemplars.
    #[test]
    fn the_round_trip_property_holds_for_every_generated_byte_string() {
        let corpus = generated_session_id_bytes(GENERATOR_SEED, GENERATOR_CASES);
        assert_eq!(
            corpus.len(),
            GENERATOR_CASES,
            "the generator produced {} cases, not {GENERATOR_CASES}; the claim \
             below would then be made over a space smaller than it names",
            corpus.len()
        );

        // class -> (first exemplar message, total count in that class)
        let mut violations: std::collections::BTreeMap<&'static str, (String, usize)> =
            std::collections::BTreeMap::new();
        let mut record = |class: &'static str, message: String| {
            let entry = violations.entry(class).or_insert((message, 0));
            entry.1 += 1;
        };

        for form in wire_forms() {
            for s in &corpus {
                let wire = (form.encode)(s);
                let read_back = crate::session_detector::session_id_in_cmdline(&wire);
                let read_back = read_back
                    .as_ref()
                    .map(|id| id.as_raw_for_logic_only().as_bytes().to_vec());

                // --- THE ORACLE, from `std`, in this test -------------------
                let decoded = std::str::from_utf8(s);
                let r1 = decoded.is_err();
                let r2 = decoded.map(|t| t.trim().is_empty()).unwrap_or(false);

                let name = form.name;
                let escaped = s.escape_ascii().to_string();

                match read_back {
                    None if !r1 && !r2 => record(
                        "refused-a-value-that-must-round-trip",
                        format!(
                            "wire form {name:?}: the byte string {s:?} \
                             (escaped: \"{escaped}\") is valid UTF-8 and does \
                             not trim to empty, so it is in NEITHER refusal \
                             class and the parser must return it byte-identically. \
                             It returned None. A value this build can put on the \
                             wire and cannot read back is a Sessions-tab row that \
                             answers `No session ID to resume` with no error \
                             anywhere — under-detection, and SILENT."
                        ),
                    ),
                    None => {}
                    Some(ref got) if r1 => record(
                        "fabricated-an-id-for-ill-formed-input",
                        format!(
                            "wire form {name:?}: the byte string {s:?} \
                             (escaped: \"{escaped}\") is NOT valid UTF-8, so it \
                             is in refusal class R1 and the parser must return \
                             None and keep scanning. It returned Some({got:?}). \
                             That is a FABRICATION: a non-empty id for a value no \
                             process carries, which puts a row in the Sessions \
                             tab offering to resume a conversation that does not \
                             exist. A lossy decode substituting U+FFFD is how \
                             this happens."
                        ),
                    ),
                    Some(ref got) if r2 => record(
                        "accepted-a-value-that-must-be-refused",
                        format!(
                            "wire form {name:?}: the byte string {s:?} \
                             (escaped: \"{escaped}\") is valid UTF-8 and trims to \
                             empty, so it is in refusal class R2 and the parser \
                             must return None. It returned Some({got:?})."
                        ),
                    ),
                    Some(ref got) if got.as_slice() != s.as_slice() => record(
                        "read-back-a-different-byte-string",
                        format!(
                            "wire form {name:?}: the byte string {s:?} \
                             (escaped: \"{escaped}\") went onto the wire and \
                             {got:?} came back. The claim is BYTE-IDENTITY, not \
                             equivalence: an id the TUI hands back rewritten is \
                             an id that resumes a different conversation, or \
                             none. Nothing in this parse may normalise, case \
                             fold, trim, truncate or cap."
                        ),
                    ),
                    Some(_) => {}
                }
            }
        }

        assert!(
            violations.is_empty(),
            "the round-trip property is FALSE over {GENERATOR_CASES} generated \
             byte strings in {} wire forms. {} violation class(es), one \
             exemplar each:\n\n{}",
            wire_forms().len(),
            violations.len(),
            violations
                .iter()
                .map(|(class, (message, count))| format!(
                    "[{class}] {count} occurrence(s)\n  {message}"
                ))
                .collect::<Vec<_>>()
                .join("\n\n")
        );
    }

    /// **The generator's reach is a COMMITTED FLOOR, asserted, not hoped for**
    /// (21-36).
    ///
    /// Replacing an enumeration with a generator buys reach and introduces
    /// exactly one new way to lie: the generator quietly stops producing a
    /// class, and the property above then passes over a space SMALLER than the
    /// one it names — green, and certifying nothing. That is the same failure
    /// shape as the enumerated corpus it replaces, one level up, and it would
    /// be invisible without this test.
    ///
    /// So the reach is committed as numbers. Each threshold gets its OWN
    /// assertion naming which class fell short, because a generator bug and a
    /// parser bug are different bugs with different fixes and one combined
    /// assertion would conflate them. That is also why this is a separate test
    /// function from
    /// [`the_round_trip_property_holds_for_every_generated_byte_string`]: a
    /// failure here means the INPUTS are wrong, not the parser.
    ///
    /// # The precedent this follows
    ///
    /// `src/text.rs`'s exhaustive Unicode sweep carries a committed
    /// `format_seen >= 150` floor for precisely this reason — an implication
    /// over a filtered set is vacuously TRUE when the filter matches nothing,
    /// and with the oracle's feature flag off that sweep would have passed
    /// green forever while asserting nothing. See the `unicode-properties`
    /// entry in `Cargo.toml`, whose comment names the floor as part of the
    /// two-crate rationale. This is the same device applied to a generated
    /// input space instead of a filtered one.
    ///
    /// # The honest limit, written where it has to be read
    ///
    /// **A generator does not abolish enumeration.** It MOVES the enumeration
    /// from VALUES to a GRAMMAR and a DISTRIBUTION, and a grammar can still
    /// miss a class — nobody should read the property above as covering every
    /// byte string a `/proc` cmdline could carry, because it does not.
    ///
    /// What it removes is narrower and is the thing that actually failed three
    /// times here: the ability of the claim to pass with **no input anywhere
    /// near the boundary**. Two devices do that work, and they are named so a
    /// later reader can tell whether a "simplification" deleted them:
    ///
    /// 1. the **`uniform` arm** — unrestricted random bytes, constrained to no
    ///    alphabet and no character class, which is what reaches classes nobody
    ///    enumerated; and
    /// 2. the **`>= 200` distinct-byte-values floor**, which fails if that arm
    ///    is ever narrowed to a restricted alphabet.
    ///
    /// A grammar arm that stops firing, or a uniform arm quietly replaced by a
    /// friendlier one, turns this test red rather than turning the property
    /// vacuous.
    #[test]
    fn the_generator_reaches_every_named_class_and_both_branches_of_the_property() {
        let corpus = generated_session_id_bytes(GENERATOR_SEED, GENERATOR_CASES);

        // --- The classes, with every predicate spelled from `std` -----------
        let empty = corpus.iter().filter(|s| s.is_empty()).count();

        let whitespace_padded = corpus
            .iter()
            .filter(|s| {
                let Ok(text) = std::str::from_utf8(s) else {
                    return false;
                };
                let edge_whitespace = text.chars().next().is_some_and(char::is_whitespace)
                    || text.chars().next_back().is_some_and(char::is_whitespace);
                edge_whitespace && !text.trim().is_empty()
            })
            .count();

        let whitespace_only = corpus
            .iter()
            .filter(|s| {
                !s.is_empty()
                    && std::str::from_utf8(s).is_ok_and(|text| text.trim().is_empty())
            })
            .count();

        let ill_formed = corpus
            .iter()
            .filter(|s| std::str::from_utf8(s).is_err())
            .count();

        let hyphen_leading = corpus.iter().filter(|s| s.first() == Some(&b'-')).count();

        let distinct_bytes: std::collections::BTreeSet<u8> =
            corpus.iter().flat_map(|s| s.iter().copied()).collect();

        assert!(
            empty >= 1,
            "the generator produced {empty} EMPTY inputs, below the floor of 1. \
             The property above is passing over a space that no longer contains \
             the empty input — the degenerate end of refusal class R2, and the \
             shape a fused `--resume=` with nothing after it puts on the wire."
        );
        assert!(
            whitespace_padded >= 20,
            "the generator produced {whitespace_padded} WHITESPACE-PADDED inputs \
             (leading or trailing `char::is_whitespace` around a non-whitespace \
             core), below the floor of 20. The property above is passing over a \
             space that no longer contains the class G1 lives in — the class \
             round 12's 28-fixture corpus did not contain either, which is how a \
             build whose parser rewrote every padded id certified itself green. \
             Uniform random bytes essentially never produce this shape; the \
             `padded` grammar arm is the only thing that does."
        );
        assert!(
            whitespace_only >= 5,
            "the generator produced {whitespace_only} non-empty WHITESPACE-ONLY \
             inputs, below the floor of 5. The property above is passing over a \
             space that no longer contains refusal class R2's interesting half \
             — the values that are non-empty on the wire and empty after trim."
        );
        assert!(
            ill_formed >= 100,
            "the generator produced {ill_formed} inputs that fail \
             `std::str::from_utf8`, below the floor of 100. The property above \
             is passing over a space that no longer contains refusal class R1 — \
             the encoding class that was not an unenumerated fixture but an \
             UNREPRESENTABLE one until the harness stopped being `&str`-typed. \
             Check the `uniform` arm first: it is what produces these."
        );
        assert!(
            hyphen_leading >= 50,
            "the generator produced {hyphen_leading} inputs whose FIRST BYTE is \
             `-`, below the floor of 50. The property above is passing over a \
             space that no longer contains the option-lookalike class — the \
             class round 10's corpus missed entirely, which let a CWE-88 fix be \
             certified by fixtures incapable of exercising it."
        );
        assert!(
            distinct_bytes.len() >= 200,
            "the generator emitted only {} DISTINCT byte values across all \
             inputs, below the floor of 200 (of 255 possible; NUL is excluded \
             because it is this encoding's separator). This is the floor that \
             fails if the `uniform` arm is ever narrowed to a restricted \
             alphabet or a character class — which would silently turn the \
             property above back into an enumeration with extra ceremony.",
            distinct_bytes.len()
        );

        // --- BOTH branches of the disjunction, actually exercised -----------
        // A disjunction whose second branch never fires passes vacuously, so
        // the property is run once more here purely to TALLY which branch each
        // case took.
        let mut round_tripped = 0_usize;
        let mut refused_r1 = 0_usize;
        let mut refused_r2 = 0_usize;

        for form in wire_forms() {
            for s in &corpus {
                let wire = (form.encode)(s);
                let read_back = crate::session_detector::session_id_in_cmdline(&wire);
                let decoded = std::str::from_utf8(s);

                match read_back {
                    Some(ref id) if id.as_raw_for_logic_only().as_bytes() == s.as_slice() => {
                        round_tripped += 1;
                    }
                    Some(_) => {}
                    None if decoded.is_err() => refused_r1 += 1,
                    None if decoded.is_ok_and(|text| text.trim().is_empty()) => refused_r2 += 1,
                    None => {}
                }
            }
        }

        assert!(
            round_tripped >= 1,
            "not one generated input ROUND-TRIPPED ({round_tripped} observed). \
             The property above is then satisfied entirely by its refusal \
             branch — a parser that returned None for everything would pass it. \
             The capability half of the claim is asserted by nothing."
        );
        assert!(
            refused_r1 >= 1,
            "not one generated input was refused as R1 ({refused_r1} observed), \
             so the ill-formed-UTF-8 branch of the disjunction never fired and \
             is certified by nothing. Either the generator stopped producing \
             ill-formed bytes or the parser stopped refusing them."
        );
        assert!(
            refused_r2 >= 1,
            "not one generated input was refused as R2 ({refused_r2} observed), \
             so the empty-after-trim branch of the disjunction never fired and \
             is certified by nothing. Either the generator stopped producing \
             whitespace-only inputs or the parser stopped refusing them."
        );
    }

    /// **`launch_terminal_argv` pinned rather than changed** (D-21-49).
    ///
    /// The sibling builder carries NO untrusted element: the separator comes
    /// from [`terminal_program_separator`]'s table and the program name is a
    /// literal, while the project path travels through `current_dir` outside
    /// the argv entirely. So there is nothing here for a fusion or a `--`
    /// separator to bind, and adding one would be ceremony shaped like a
    /// control.
    ///
    /// That is true only while the vector stays two elements long. This
    /// equality is what turns "examined and found safe" into something that
    /// FAILS when it stops being true: appending a third element — the only
    /// way this builder could acquire an untrusted value — breaks it and
    /// forces the decision to be made again explicitly.
    #[test]
    fn launch_terminal_argv_carries_no_untrusted_element_and_is_pinned_at_two() {
        for (term, expected_separator) in [
            ("kitty", "-e"),
            ("alacritty", "-e"),
            ("gnome-terminal", "--"),
            ("xterm", "-e"),
            // Newly reachable since 260908-w0d. `launch_terminal_argv` STAYS
            // AT TWO for them (D-04): no `terminal_leading_options` helper and
            // no prepended `--new-window`, because that would loosen this
            // pin's exact property — no third element, therefore no untrusted
            // element — to buy a window-vs-tab difference on a last-resort
            // path.
            ("ptyxis", "--"),
            ("xdg-terminal-exec", "--"),
            ("x-terminal-emulator", "-e"),
            ("wezterm", "-e"),
            ("/usr/bin/gnome-terminal", "--"),
        ] {
            assert_eq!(
                launch_terminal_argv(term),
                vec![expected_separator.to_string(), "claude".to_string()],
                "launch_terminal_argv({term:?}) must be exactly the separator \
                 and the program name. A THIRD element means this builder has \
                 acquired a value from somewhere, and if that value is \
                 untrusted it needs the same fusion `resume_terminal_argv` \
                 got — decide it explicitly rather than inheriting this \
                 function's `no untrusted element` reasoning, which was true \
                 only of the two-element shape."
            );
        }
    }

    /// **The capability half of the fix** (T-21-27-06, prohibition 2).
    ///
    /// Every launcher [`resolve_launch_plan`] can return, plus the unbounded
    /// `$TERMINAL` case, and in both directions: the separator is correct AND
    /// the resumed program's own long option lands AFTER it, where the emulator
    /// cannot consume it as one of its own.
    #[test]
    fn every_launcher_the_probe_can_return_gets_a_separator_that_keeps_the_program_options() {
        // Both discovery consts, plus `$TERMINAL`. If either const grows a
        // launcher, this arm must grow with it — and
        // `the_separator_table_covers_every_launcher_the_discovery_consts_name`
        // is what fails when it does not.
        for (term, expected) in [
            ("kitty", "-e"),
            ("alacritty", "-e"),
            ("gnome-terminal", "--"),
            ("xterm", "-e"),
            // Newly reachable since 260908-w0d: the fallback list gained
            // `ptyxis`, and discovery can now return either launcher by name.
            ("ptyxis", "--"),
            ("xdg-terminal-exec", "--"),
            ("x-terminal-emulator", "-e"),
            // An arbitrary `$TERMINAL`, and the same value behind a path.
            ("wezterm", "-e"),
            ("/usr/bin/gnome-terminal", "--"),
        ] {
            assert_eq!(
                terminal_program_separator(term),
                expected,
                "{term:?} must be handed {expected:?}. `gnome-terminal` is the \
                 one that first differed: its `-e` is deprecated and takes a \
                 single re-parsed string, so with a real argv it consumes \
                 `--resume` as one of its OWN options and the resume silently \
                 stops working — a feature deletion wearing a security fix's \
                 clothes. `ptyxis` is the same shape (its `-x` takes a single \
                 re-parsed string and it has no `-e` at all), which is why it \
                 gets `--` rather than inheriting the default."
            );

            let sid = Untrusted::from_untrusted_source("abc".to_string());
            let argv = resume_terminal_argv(term, &sid);
            assert_eq!(
                argv.first().map(String::as_str),
                Some(expected),
                "the separator must be the FIRST element of {argv:?}, or the \
                 emulator reads the program name as one of its own operands"
            );
            let separator_at = argv.iter().position(|e| e == expected);
            // Since 21-31 the resumed program's option is FUSED to its value
            // (`--resume=<id>`), so no element EQUALS `--resume` any more. The
            // lookup matches the fused PREFIX instead; the property being
            // asserted — the emulator must not consume the resumed program's
            // own option — is unchanged.
            let option_at = argv
                .iter()
                .position(|e| e.starts_with(RESUME_OPTION_FUSED_PREFIX));
            assert!(
                option_at.is_some(),
                "no element of {argv:?} carries {RESUME_OPTION_FUSED_PREFIX:?}, \
                 so this ordering assertion would pass vacuously"
            );
            assert!(
                separator_at < option_at,
                "the resumed program's own long option `--resume` must appear \
                 AFTER the separator in {argv:?}; before it, the emulator \
                 consumes it and resumes nothing"
            );
        }
    }

    /// The discovery consts and the separator table must not drift apart
    /// (D-05). This is the pin that makes the table's coverage claim checkable
    /// rather than a comment: a launcher added to either const without a
    /// decision here fails this.
    ///
    /// # Re-pointed 2026-09-08 (260908-w0d): the parse followed the code
    ///
    /// It used to grep the body of `fn find_terminal()` and split on the first
    /// `[`. That function is gone — the ordering now lives in
    /// [`crate::terminal_switch::plan_launch`] and the two candidate lists were
    /// hoisted to [`FALLBACK_TERMINAL_CANDIDATES`] and [`DISCOVERY_LAUNCHERS`]
    /// so this guard could keep its ESSENTIAL property: the table it
    /// adjudicates is DERIVED FROM SOURCE TEXT, so it cannot drift from the
    /// code the way a list somebody remembers can.
    ///
    /// # Two things this version does that the previous one did not
    ///
    /// **A positive control, first.** Both consts must be found and each must
    /// parse to a NON-EMPTY list. Without it, a parse that silently matched
    /// nothing would run zero adjudications and pass having certified nothing —
    /// an absence assertion cannot distinguish "no launcher lacks a row" from
    /// "no launcher was read".
    ///
    /// **A two-way expectations table, replacing a check that could never
    /// fail.** The old loop asserted `matches!(separator, "-e" | "--")`, which
    /// holds for ANY string whatsoever: [`terminal_program_separator`]'s `_` arm
    /// returns `-e`. So the guard's own claim — "a candidate added without a
    /// separator decision fails this" — was FALSE, and the only thing it
    /// actually enforced was the arity. The table below is checked in both
    /// directions (every parsed name has a row, every row's name was parsed)
    /// and the resolved separator must EQUAL the row's, so a launcher that
    /// inherits `-e` by silence is red.
    #[test]
    fn the_separator_table_covers_every_launcher_the_discovery_consts_name() {
        let source = include_str!("detail.rs");

        /// The slice literal declared under `name`, parsed back out of the
        /// source text. `None` when the const is not there at all.
        fn declared_names(source: &str, name: &str) -> Option<Vec<String>> {
            let after = source.split_once(&format!("const {name}: &[&str]"))?.1;
            let list = after.split_once("&[")?.1.split_once(']')?.0;
            Some(
                list.split(',')
                    .map(|piece| piece.trim().trim_matches('"').to_string())
                    .filter(|piece| !piece.is_empty())
                    .collect(),
            )
        }

        // --- POSITIVE CONTROL, before any adjudication ----------------------
        let fallback = declared_names(source, "FALLBACK_TERMINAL_CANDIDATES")
            .expect("`const FALLBACK_TERMINAL_CANDIDATES: &[&str]` must be findable in this file");
        let discovery = declared_names(source, "DISCOVERY_LAUNCHERS")
            .expect("`const DISCOVERY_LAUNCHERS: &[&str]` must be findable in this file");
        assert!(
            !fallback.is_empty(),
            "FALLBACK_TERMINAL_CANDIDATES parsed to an EMPTY list, so every \
             assertion below would pass having adjudicated nothing"
        );
        assert!(
            !discovery.is_empty(),
            "DISCOVERY_LAUNCHERS parsed to an EMPTY list, so every assertion \
             below would pass having adjudicated nothing"
        );

        let parsed: Vec<String> = fallback.iter().chain(discovery.iter()).cloned().collect();
        assert_eq!(
            parsed.len(),
            7,
            "the discovery consts name {parsed:?}; the separator table was \
             written against exactly seven launchers (5 fallback + 2 \
             discovery). A launcher added there without a separator decision \
             here inherits `-e` BY SILENCE — which is how `gnome-terminal` \
             would have been wrong, and `ptyxis` after it."
        );

        // --- The authored expectations, checked in BOTH directions ----------
        let expected: [(&str, &str); 7] = [
            ("kitty", "-e"),
            ("alacritty", "-e"),
            ("ptyxis", "--"),
            ("gnome-terminal", "--"),
            ("xterm", "-e"),
            ("xdg-terminal-exec", "--"),
            ("x-terminal-emulator", "-e"),
        ];

        for name in &parsed {
            let row = expected.iter().find(|(candidate, _)| candidate == name);
            let (_, separator) = row.unwrap_or_else(|| {
                panic!(
                    "the launcher {name:?} is named by a discovery const and has \
                     NO row in this test's expectations table, so no separator \
                     decision was ever made for it. It would inherit `-e` from \
                     `terminal_program_separator`'s `_` arm — silently, and \
                     wrongly for any emulator that re-parses a single string."
                )
            });
            assert_eq!(
                terminal_program_separator(name),
                *separator,
                "{name:?} must be handed {separator:?}"
            );
        }

        for (name, _) in &expected {
            assert!(
                parsed.iter().any(|parsed_name| parsed_name == name),
                "this test expects a separator for {name:?}, but no discovery \
                 const names it. Either the launcher was removed from the code \
                 and this row is stale, or the source-text parse above has \
                 stopped seeing the const it thinks it is reading."
            );
        }
    }

    // -----------------------------------------------------------------------
    // The tmux path (D-03, T-W0D-01, T-W0D-02)
    //
    // Asserted at the argv VALUES, never at the source text. A text grep for
    // interpreter names would be strictly weaker AND would be invalidated by
    // the very doc comments that explain why `env` is not one.
    // -----------------------------------------------------------------------

    /// **The tmux direct-exec boundary, pinned at the BUILDERS** (D-03).
    ///
    /// `man tmux`: a shell-command given as MULTIPLE arguments is executed
    /// directly, without `sh -c`; a SINGLE argument still goes through one. So
    /// `len() >= 2` is what keeps a command interpreter out of this path — the
    /// same property CR-01 / T-21-27-01 / T-21-27-02 bought at the GUI sites.
    ///
    /// [`crate::terminal_switch::open_new_window`] refuses a short vector too,
    /// and that refusal is the fail-closed sink every future caller inherits.
    /// This assertion lives here *as well* because the builders are the place a
    /// future edit would shorten a vector, and a sink that is never reached with
    /// a bad value is a sink nobody notices they broke.
    #[test]
    fn the_tmux_argvs_are_pinned_at_two_or_more_with_no_interpreter() {
        let interpreters = interpreter_binaries();
        let command_flag =
            format!("{INTERPRETER_COMMAND_FLAG_HEAD}{INTERPRETER_COMMAND_FLAG_TAIL}");
        let sid = Untrusted::from_untrusted_source("abc".to_string());

        for argv in [tmux_launch_argv(), tmux_resume_argv(&sid)] {
            assert!(
                argv.len() >= 2,
                "the tmux program vector {argv:?} has fewer than two elements. \
                 tmux hands a SINGLE-argument shell-command to a command \
                 interpreter, which puts a parser back in a path that \
                 deliberately has none."
            );
            assert_eq!(
                argv.first().map(String::as_str),
                Some("env"),
                "the tmux program vector {argv:?} must begin with the literal \
                 `env`. That is what makes the >= 2 arity true BY CONSTRUCTION \
                 for every vector, present and future, rather than by a rule \
                 each caller has to remember — and it fixes the head to a \
                 literal that provably does not start with `-`, so tmux's own \
                 option parser stops there."
            );
            assert!(
                !argv[0].starts_with('-'),
                "the head of {argv:?} starts with `-`, so tmux's option parser \
                 would consume it instead of treating it as the program"
            );
            for element in &argv {
                let stem = element.rsplit('/').next().unwrap_or(element);
                assert!(
                    !interpreters.iter().any(|binary| binary == stem),
                    "argv element {element:?} is a command interpreter. `env` is \
                     NOT one — it has no program-string mode, it consumes \
                     leading NAME=VALUE and option words and then `execvp`s the \
                     first non-option word with the remainder passed through \
                     UNPARSED. Full argv: {argv:?}"
                );
                assert_ne!(
                    element, &command_flag,
                    "argv {argv:?} carries the flag by which an interpreter is \
                     handed a program to PARSE"
                );
            }
        }
    }

    /// **CWE-88 / T-W0D-02 on the tmux path.** Modelled on
    /// [`the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element`],
    /// over the same adversarial corpus, because the tmux path reaches the same
    /// `claude` option parser through a different carrier.
    #[test]
    fn the_tmux_resume_argv_carries_a_hostile_session_id_as_one_opaque_element() {
        // Non-vacuity: the corpus must actually be hostile.
        assert!(
            hostile_session_ids()
                .iter()
                .any(|id| id.chars().any(|c| "'\";&|`$\n".contains(c))),
            "the fixture set carries no shell metacharacter at all, so this \
             control would pass against the construction it exists to reject"
        );

        let mut arities = std::collections::BTreeSet::new();
        for raw in hostile_session_ids() {
            let sid = Untrusted::from_untrusted_source(raw.clone());
            let argv = tmux_resume_argv(&sid);
            arities.insert(argv.len());

            let carried = argv
                .iter()
                .filter(|element| element.split_once('=').map(|(_, v)| v) == Some(raw.as_str()))
                .count();
            assert_eq!(
                carried, 1,
                "the session id {raw:?} must be carried by exactly ONE argv \
                 element as the suffix after that element's first `=`; it was \
                 carried {carried} times in {argv:?}"
            );

            // No OTHER element may carry any of the id's bytes: a value split
            // across elements is a value something parsed.
            let elsewhere = argv
                .iter()
                .filter(|element| {
                    element.split_once('=').map(|(_, v)| v) != Some(raw.as_str())
                        && element.contains(raw.as_str())
                })
                .count();
            assert_eq!(
                elsewhere, 0,
                "bytes of {raw:?} appear in an element other than the fused \
                 one, in {argv:?} — the id was split or duplicated by something \
                 reading it"
            );
        }

        assert_eq!(
            arities.len(),
            1,
            "the tmux argv arity varied across inputs ({arities:?}), so some id \
             changed the SHAPE of the vector rather than just one element of \
             it. A value that can change the arity is a value being parsed."
        );
    }

    /// **The registry cannot drift from the runtime** (D-05).
    ///
    /// [`DISCOVERY_LAUNCHERS`] is not read by the runtime — the names live in
    /// `plan_launch` and `probe_terminals` — so on its own it is a list somebody
    /// remembers, which is the artefact the source-derived guard exists to
    /// replace. This drives the REAL `plan_launch` at each discovery rank and
    /// asserts the answer is a name the registry carries.
    #[test]
    fn the_discovery_consts_name_every_launcher_the_plan_can_actually_return() {
        use crate::terminal_switch::{plan_launch, TerminalProbes};

        let at_rank = |xdg: bool, x_term: bool| -> String {
            plan_launch(&TerminalProbes {
                in_tmux: false,
                terminal_env: None,
                xdg_terminal_exec_ok: xdg,
                x_terminal_emulator_present: x_term,
                fallback_present: Vec::new(),
            })
            .gui
            .expect("a discovery rank must resolve to a launcher")
        };

        for answer in [at_rank(true, false), at_rank(false, true)] {
            assert!(
                DISCOVERY_LAUNCHERS.contains(&answer.as_str()),
                "`plan_launch` answered {answer:?} at a discovery rank, and \
                 {DISCOVERY_LAUNCHERS:?} does not list it. The separator guard \
                 adjudicates against that const, so a launcher the runtime can \
                 actually return but the const does not name gets NO separator \
                 decision and inherits `-e` by silence — which is how \
                 `gnome-terminal` would have been wrong."
            );
        }
    }

    /// **The drift pin across the two paths** (T-W0D-02).
    ///
    /// `claude_resume_args` is the single construction site of the fused
    /// element, and this is what makes that "single" checkable: the GUI builder
    /// and the tmux builder must hand the resumed program a byte-identical
    /// `--resume=<id>`. Two builders that construct it separately would agree on
    /// the day they were written and diverge on the day one of them is edited —
    /// and the CWE-88 fix lives entirely in the bytes of that element.
    #[test]
    fn both_resume_paths_carry_the_same_fused_element() {
        for raw in hostile_session_ids() {
            let sid = Untrusted::from_untrusted_source(raw.clone());
            let gui = resume_terminal_argv("gnome-terminal", &sid);
            let tmux = tmux_resume_argv(&sid);

            let fused = |argv: &[String]| -> Option<String> {
                argv.iter()
                    .find(|e| e.starts_with(RESUME_OPTION_FUSED_PREFIX))
                    .cloned()
            };
            let gui_fused = fused(&gui).unwrap_or_else(|| {
                panic!("the GUI argv {gui:?} carries no fused resume element at all")
            });
            let tmux_fused = fused(&tmux).unwrap_or_else(|| {
                panic!("the tmux argv {tmux:?} carries no fused resume element at all")
            });
            assert_eq!(
                gui_fused, tmux_fused,
                "the GUI path and the tmux path built DIFFERENT resume elements \
                 for the same id {raw:?}. They must come from the one \
                 construction site, or the CWE-88 fusion holds on whichever path \
                 was edited last."
            );
        }
    }
}
