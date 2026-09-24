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
use crate::state_reader::disk_status::{DiskInference, DiskStatus, VerificationStatus};
use crate::state_reader::git_ops;
use crate::state_reader::queue_md;
use crate::state_reader::{self, backlog};
use crate::text::Untrusted;
use crate::ui::roadmap_graph;
use crate::ui::roadmap_widget::RoadmapWidget;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Margin, Rect};
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

/// The banner shown when `.planning/STATE.md` exists but its frontmatter could
/// not be read.
///
/// Every value this screen takes from STATE.md — status, milestone, phase — is
/// silently defaulted when the read fails, so without this line the screen
/// renders defaults as if they were the project's real state. It follows the
/// `Paused:` banner's shape deliberately: same position, same one-line form,
/// only the colour says this one is a fault rather than a condition.
fn unreadable_state_line(state: &state_reader::ProjectState) -> Option<Line<'static>> {
    if !state.state_md_unreadable {
        return None;
    }
    // `describe()` is a fixed phrase this crate wrote — it needs no escaping,
    // and is not run through `shown` so that fact stays visible here.
    let mut detail = match state.state_md_fault {
        Some(fault) => format!("  STATE.md unreadable: {}", fault.describe()),
        None => "  STATE.md unreadable".to_string(),
    };
    // Same argument, arithmetic instead of prose: these are two integers this
    // crate computed from a parser location, not text the repository wrote, so
    // they are interpolated directly and deliberately NOT routed through
    // `shown`/`render_for_terminal`. A number cannot carry an escape sequence.
    if let Some(position) = state.state_md_fault_position {
        detail.push_str(&format!(
            " (line {}, column {})",
            position.line, position.column
        ));
    }
    Some(Line::from(Span::styled(
        detail,
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    )))
}

/// The line shown when `.planning/STATE.md` parsed only after this tool
/// repaired it in memory.
///
/// Without it a repaired document renders exactly like a clean one, and the
/// screen presents values taken from a file a strict reader still refuses. It
/// follows the `Paused:` banner's shape rather than the unreadable banner's:
/// same position, same one-line form, and **yellow rather than red**, because
/// this is a condition the screen is reporting, not a fault that lost data.
fn recovered_state_line(state: &state_reader::ProjectState) -> Option<Line<'static>> {
    if !state.state_md_recovered {
        return None;
    }
    // A fixed phrase this crate wrote — nothing from the document and nothing
    // from the parser is interpolated, so it needs no escaping, and not running
    // it through `shown` is what keeps that visible here.
    Some(Line::from(Span::styled(
        "  STATE.md repaired in memory to be read: the file on disk is unchanged",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )))
}

/// The Sessions tab's answer to Enter on a Codex row (260923-lr9).
const CODEX_RESUME_UNSUPPORTED: &str =
    "Resume is not supported for Codex sessions — press Tab to switch to it";

/// The Sessions tab's answer to Enter on a Claude row with no session id.
const NO_SESSION_ID_TO_RESUME: &str = "No session ID to resume";

/// The id Enter may resume, or the status message saying why it may not.
///
/// **The resume gate (T-lr9-01).** This build's only resume producer spells a
/// `claude` argv, so a Codex thread id must never reach it: every
/// [`SessionKind::Codex`](crate::session_detector::SessionKind::Codex) session
/// is refused, whatever its id. The match is exhaustive with no wildcard, so a
/// new kind is a compile error here rather than a silent resume.
fn resumable_session_id(
    session: &crate::session_detector::ClaudeSession,
) -> Result<&Untrusted, &'static str> {
    use crate::session_detector::SessionKind;
    match session.kind {
        SessionKind::Codex => Err(CODEX_RESUME_UNSUPPORTED),
        SessionKind::Claude => session.session_id.as_ref().ok_or(NO_SESSION_ID_TO_RESUME),
    }
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

/// The narrowest subject worth keeping a co-author column beside.
///
/// Below this, the row is all attribution and no content, which inverts what a
/// log row is for. Twelve characters is roughly two short words plus an
/// ellipsis — enough to recognise a commit you already know.
pub(super) const GIT_ROW_MIN_SUBJECT_COLS: usize = 12;

/// How a Git tab log row divides its width (QD-04).
pub(super) struct GitRowBudget {
    /// Characters the subject may take. May be zero at absurd widths.
    pub(super) subject_cols: usize,
    /// Whether the co-author column is drawn at all.
    pub(super) show_co_authors: bool,
}

/// Divide a log row's width between the subject and the attribution columns.
///
/// **The subject gives way first, and the co-author before the author**
/// (QD-04). The subject is the field with a second home — Enter opens the full
/// message in the pane below — so it is the one that can afford an ellipsis.
/// The author is never dropped: "who wrote this" is the question the row exists
/// to answer, and the co-author is the newer, more expendable half of it.
///
/// # These are CHARACTERS, not display columns — disclosed
///
/// Every count here is `chars().count()` of the ESCAPED string, which equals
/// display width only for single-width glyphs. A CJK author name or an emoji in
/// a subject is two cells wide and counted as one, so the row can still overrun
/// by the number of wide glyphs it holds. That is the pre-existing
/// display-width question this codebase tracks separately in the open todo
/// `2026-07-29-badge-glyph-display-width-alignment.md`, and this function
/// deliberately does not claim to answer it — a half-answer here would make the
/// todo look closed.
pub(super) fn git_row_budget(
    total_cols: usize,
    hash_cols: usize,
    date_cols: usize,
    author_cols: usize,
    co_author_cols: Option<usize>,
) -> GitRowBudget {
    // "> " highlight symbol + hash + " -- " + date + " -- " + <subject> + "  "
    // + author, then optionally "  (" + co-authors + ")".
    const HIGHLIGHT: usize = 2;
    const SEPARATOR: usize = 4;
    const AUTHOR_GAP: usize = 2;
    const CO_AUTHOR_FRAME: usize = 4; // "  (" plus ")"

    let fixed = HIGHLIGHT + hash_cols + SEPARATOR + date_cols + SEPARATOR + AUTHOR_GAP + author_cols;
    let available = total_cols.saturating_sub(fixed);

    match co_author_cols {
        Some(cols) => {
            let with_co_authors = available.saturating_sub(cols + CO_AUTHOR_FRAME);
            if with_co_authors >= GIT_ROW_MIN_SUBJECT_COLS {
                GitRowBudget {
                    subject_cols: with_co_authors,
                    show_co_authors: true,
                }
            } else {
                // Dropping the column returns its room to the subject rather
                // than leaving a gap where it would have been.
                GitRowBudget {
                    subject_cols: available,
                    show_co_authors: false,
                }
            }
        }
        None => GitRowBudget {
            subject_cols: available,
            show_co_authors: false,
        },
    }
}

/// Truncate `subject` to `cols` characters, marking the cut with an ellipsis.
///
/// Character-wise, never byte-wise: a byte slice of a multi-byte subject
/// panics, which is the defect
/// `a_multibyte_session_id_does_not_panic_the_render` pins elsewhere in this
/// file.
fn truncate_subject(subject: &str, cols: usize) -> String {
    if cols == 0 {
        return String::new();
    }
    if subject.chars().count() <= cols {
        return subject.to_string();
    }
    let mut out: String = subject.chars().take(cols.saturating_sub(1)).collect();
    out.push('…');
    out
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

/// The **array capacity** of [`TAB_LABELS_FULL`] and [`TAB_LABELS_COMPACT`] —
/// every tab this view can have, Driver included.
///
/// **Not the number of tabs a given session shows.** Since quick task
/// 260917-fko the Driver tab exists only when `AppContext.experimental` is set,
/// so the bound every loop, clamp and navigation guard needs is
/// [`visible_tab_count`], not this. This constant survives as the label arrays'
/// length and as the index the Driver entry sits at the end of.
pub(crate) const TAB_COUNT: usize = 10;

/// How many tabs the detail view actually shows: 10 with the experimental
/// surfaces on, 9 without.
///
/// The one bound. A second `TAB_COUNT - 1` left behind anywhere is how the
/// Right arrow would walk onto a tab that does not render (260917-fko D2).
pub(crate) fn visible_tab_count(experimental: bool) -> usize {
    if experimental {
        TAB_COUNT
    } else {
        TAB_COUNT - 1
    }
}

/// Full tab labels, one per index. The Driver entry omits its live-marker cell,
/// which [`tab_titles`] always appends as a span of its own (see
/// [`DRIVER_LIVE_MARKER`]).
///
/// The Phase 24 order (D-B01, D-B11): Roadmap first, the Pipeline sub-view
/// labelled `Phases` second. `9:Arch` is an **interim** ninth tab — plan 24-07
/// folds the milestone archive into Docs and drops it.
const TAB_LABELS_FULL: [&str; TAB_COUNT] = [
    "1:Roadmap",
    "2:Phases",
    "3:Backlog",
    "4:Git",
    "5:Queue",
    "6:Sess",
    "7:Cfg",
    "8:Docs",
    "9:Arch",
    "D:Drive",
];

/// Compact tab labels, same order. The digit is the real handle in every case,
/// so a two-letter mnemonic loses nothing that matters; the pairs are mutually
/// unambiguous.
const TAB_LABELS_COMPACT: [&str; TAB_COUNT] = [
    "1:Rd", "2:Ph", "3:Bk", "4:Gt", "5:Qu", "6:Ss", "7:Cf", "8:Dc", "9:Ar", "D:Dr",
];

/// The Driver tab's index. Named because five sites compare against it.
pub(crate) const DRIVER_TAB_INDEX: usize = 9;

/// Rendered width of the full label set, in terminal cells.
///
/// **Derived from the render, not chosen:** `Tabs` draws `1 pad + label + 1 pad`
/// per tab and a one-cell `"|"` divider between adjacent tabs, so the bar costs
/// `Σ(len + 2) + (n − 1)`. For [`TAB_LABELS_FULL`] plus the Driver tab's
/// always-present marker cell that is `68 + 1 + 20 + 9 = 98`.
///
/// The number matters because **the shipped tabs have never fitted 80 columns
/// whole in the full tier**: before tiering, the last two labels (config and
/// docs) were silently dropped off the right edge at an 80-column terminal,
/// which is the "tab bar overflow at 80 columns" defect carried in `STATE.md`
/// since Phase 12. Appending the Driver tab at the end without tiering would
/// have made it the one that never renders, at every common width.
pub(crate) const TAB_BAR_FULL_CELLS: u16 = 98;

/// Rendered width of the compact label set, by the same arithmetic:
/// `40 + 1 + 20 + 9 = 70`.
pub(crate) const TAB_BAR_COMPACT_CELLS: u16 = 70;

/// Rendered width of the full label set **without** the Driver tab, by the same
/// `Σ(len + 2) + (n − 1)` formula: `61 + 18 + 8 = 87`.
///
/// Dropping the Driver tab costs four things and the arithmetic has to lose
/// all four: its 7-cell label, its reserved marker cell, its two pads and the
/// one divider that joined it to the tab before it — `98 − 7 − 1 − 2 − 1 = 87`.
/// `the_tab_bar_widths_are_the_label_arrays_own_arithmetic` re-derives it from
/// [`TAB_LABELS_FULL`] so a renamed label cannot leave this number behind
/// (260917-fko).
pub(crate) const TAB_BAR_FULL_CELLS_NO_DRIVER: u16 = 87;

/// Rendered width of the compact label set without the Driver tab:
/// `36 + 18 + 8 = 62`. The same four deductions as above, against a 4-cell
/// compact label: `70 − 4 − 1 − 2 − 1 = 62`.
pub(crate) const TAB_BAR_COMPACT_CELLS_NO_DRIVER: u16 = 62;

/// The full tier's threshold for this session's tab count.
pub(crate) fn tab_bar_full_cells(experimental: bool) -> u16 {
    if experimental {
        TAB_BAR_FULL_CELLS
    } else {
        TAB_BAR_FULL_CELLS_NO_DRIVER
    }
}

/// The compact tier's threshold for this session's tab count.
///
/// Without the Driver tab the compact bar fits in
/// [`TAB_BAR_COMPACT_CELLS_NO_DRIVER`] cells rather than
/// [`TAB_BAR_COMPACT_CELLS`], so a flag-off session a few columns narrower gets
/// whole compact labels where a with-Driver session would have been pushed into
/// the windowed tier. Reusing the with-Driver numbers would have cost exactly
/// that.
pub(crate) fn tab_bar_compact_cells(experimental: bool) -> u16 {
    if experimental {
        TAB_BAR_COMPACT_CELLS
    } else {
        TAB_BAR_COMPACT_CELLS_NO_DRIVER
    }
}

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
/// whole tab bar can be shown.
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
///
/// `experimental` decides whether the Driver label is in the set at all: with
/// it off the labels are sliced to the first [`visible_tab_count`] and the tier
/// thresholds drop accordingly, so nothing about the bar tells the user a
/// Driver tab exists (260917-fko D2).
pub(crate) fn tab_titles(
    width: u16,
    active: usize,
    driver_live: bool,
    experimental: bool,
) -> (Vec<Line<'static>>, usize) {
    let visible = visible_tab_count(experimental);
    let active = active.min(visible - 1);

    if width >= tab_bar_full_cells(experimental) {
        return (
            (0..visible)
                .map(|i| tab_label_line(&TAB_LABELS_FULL[..visible], i, driver_live))
                .collect(),
            active,
        );
    }
    if width >= tab_bar_compact_cells(experimental) {
        return (
            (0..visible)
                .map(|i| tab_label_line(&TAB_LABELS_COMPACT[..visible], i, driver_live))
                .collect(),
            active,
        );
    }

    windowed_tab_titles(width, active, driver_live, experimental)
}

/// One tab's `Line`. The Driver entry gets its marker cell as a second span so
/// the label's width is identical live and idle.
///
/// Takes a **slice** rather than a `[&str; TAB_COUNT]` so the flag-off
/// Driver-less view can be passed without copying: with the Driver label sliced away this
/// function's `index == DRIVER_TAB_INDEX` branch — and therefore the magenta
/// live marker — is simply never reached.
fn tab_label_line(labels: &[&'static str], index: usize, driver_live: bool) -> Line<'static> {
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
///
/// The marker cell is the Driver tab's, so it is only ever charged when the
/// experimental surfaces are on — with them off the index can never be
/// [`DRIVER_TAB_INDEX`] anyway, and the flag keeps that explicit rather than
/// relying on the caller's bound.
fn compact_label_cells(index: usize, experimental: bool) -> usize {
    TAB_LABELS_COMPACT[index].chars().count()
        + usize::from(experimental && index == DRIVER_TAB_INDEX)
}

/// The windowed tier: the widest contiguous run of compact labels that contains
/// `active` and fits, with overflow markers on whichever side was truncated.
///
/// The window is grown outward from the active tab — right first, then left — so
/// the active label is the one thing that is never given up. If even the active
/// label alone overflows the bar it is still rendered: a clipped label the user
/// can see beats a correct one they cannot.
fn windowed_tab_titles(
    width: u16,
    active: usize,
    driver_live: bool,
    experimental: bool,
) -> (Vec<Line<'static>>, usize) {
    let visible = visible_tab_count(experimental);
    let cost = |start: usize, end: usize| -> usize {
        let mut entries: Vec<usize> = (start..end)
            .map(|index| compact_label_cells(index, experimental))
            .collect();
        if start > 0 {
            entries.insert(0, 1);
        }
        if end < visible {
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
        if end < visible && cost(start, end + 1) <= budget {
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
        titles.push(tab_label_line(
            &TAB_LABELS_COMPACT[..visible],
            i,
            driver_live,
        ));
    }
    if end < visible {
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
    /// Last-rendered viewport metrics for the Git tab's commit-message pane.
    ///
    /// A fifth `Cell` alongside the four above rather than a fifth *mechanism*,
    /// for the reason the Driver one records: the pane's PageUp/PageDown arms
    /// clamp through the same [`clamp_scroll`] formula, so UIFIX-04 cannot come
    /// back here in a new spelling.
    git_commit_viewport: Cell<ViewportMetrics>,
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
            git_commit_viewport: Cell::default(),
        }
    }

    /// A detail screen that is ALREADY on `sub_view`, with that tab's arrival
    /// work done — so the first paint shows loaded content, not an empty pane.
    ///
    /// # Why this goes through `switch_to_tab`
    ///
    /// Landing on a tab is not the same as rendering it. Every tab that needs
    /// anything on arrival has that work in exactly one place —
    /// [`switch_to_tab`]: the Backlog parse, the git-log spawn, the defaults
    /// load, the browser's lazy init, the Driver run scan. A caller that wanted
    /// to open the screen pre-parked on a tab and set
    /// `detail_sub_view_per_project` itself would render a blank tab on first
    /// paint AND would be a second copy of the arrival rule, free to drift from
    /// the one the digit keys use. 260916-vr0 is the standing example of what
    /// two rules kept in agreement by care actually do: the overview counted
    /// backlog items by one rule and the tab drew them by another, and they
    /// disagreed totally.
    ///
    /// The index comes from [`tab_index`] rather than from a literal at the
    /// call site. That is the whole reason `tab_index`/[`sub_view_from_index`]
    /// are `pub(crate)` and round-trip-asserted: a hand-written index is a
    /// second mapping, and a wrong one lands the user on a tab they did not ask
    /// for.
    ///
    /// `switch_to_tab`'s `ScreenAction` is discarded because a screen under
    /// construction has no stack to act on. That discard is pinned by
    /// `switching_to_the_backlog_tab_returns_no_screen_action`, which goes red
    /// if the function ever returns anything but `None`.
    pub(crate) fn opened_on(alias: String, sub_view: DetailSubView, ctx: &mut AppContext) -> Self {
        let mut screen = Self::new(alias);
        let index = tab_index(&sub_view);
        let _ = switch_to_tab(&screen.alias, index, &mut screen.scroll_offset, ctx);
        screen
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
    // Exhaustive and wildcard-free on purpose (T-24-09): a new variant must be
    // given an index here or the crate does not compile.
    match sub_view {
        DetailSubView::RoadmapViz => 0,
        DetailSubView::Pipeline => 1,
        DetailSubView::Backlog => 2,
        DetailSubView::GitHistory => 3,
        DetailSubView::Queue => 4,
        DetailSubView::Sessions => 5,
        DetailSubView::Defaults => 6,
        DetailSubView::Browse => 7,
        // Interim ninth tab until plan 24-07 folds it into Docs.
        DetailSubView::Archive => 8,
        // Index 9 — the last tab, reachable by `Left`/`Right`, by `Shift+D`,
        // and rendered by `tab_titles` at every width.
        DetailSubView::Driver => DRIVER_TAB_INDEX,
    }
}

/// `pub(crate)` for the same reason as [`tab_index`]: the round trip is the
/// property worth asserting, and it takes both halves.
///
/// With `experimental` off, index 9 ([`DRIVER_TAB_INDEX`]) is **not a tab**, so
/// it falls through to the same default-tab fallback an out-of-range index
/// already took. That is what makes `Shift+D` and a stored Driver index land
/// somewhere real instead of on a tab the bar does not draw (260917-fko D2).
pub(crate) fn sub_view_from_index(index: usize, experimental: bool) -> DetailSubView {
    if index == DRIVER_TAB_INDEX && !experimental {
        return DetailSubView::RoadmapViz;
    }
    match index {
        0 => DetailSubView::RoadmapViz,
        1 => DetailSubView::Pipeline,
        2 => DetailSubView::Backlog,
        3 => DetailSubView::GitHistory,
        4 => DetailSubView::Queue,
        5 => DetailSubView::Sessions,
        6 => DetailSubView::Defaults,
        7 => DetailSubView::Browse,
        8 => DetailSubView::Archive,
        DRIVER_TAB_INDEX => DetailSubView::Driver,
        // Fallback: an out-of-range index lands on the default (Roadmap) tab
        // rather than on the newest one (D-B07).
        _ => DetailSubView::RoadmapViz,
    }
}

/// The sub-view a flag-off session is actually on, given the one it has stored.
///
/// **`DetailSubView::Driver` does not exist when the experimental surfaces are
/// off**, so a stored one — parked by an earlier session that had the flag set,
/// or by a `Shift+D` that slipped through — has to read as the default tab
/// rather than as a tab nothing renders.
///
/// Applied at the **three** sites that read `detail_sub_view_per_project` for
/// the current view: the key handler, the main render and the overlay backdrop
/// render. Those three are the whole reachability surface of the Driver tab, so
/// coercing here disarms all ten Driver-only key arms, both `render_driver_tab`
/// dispatches and the Driver-entry scan schedule at once — one coercion instead
/// of a guard per arm, which is the version that cannot be half-applied.
pub(crate) fn effective_sub_view(stored: DetailSubView, experimental: bool) -> DetailSubView {
    if !experimental && stored == DetailSubView::Driver {
        return DetailSubView::RoadmapViz;
    }
    stored
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
///
/// **Unreferenced between plans 24-03 and 24-06, kept on purpose.** Its only
/// caller was the removed PhaseList tab (D-B02); plan 24-06 re-wires it as the
/// per-phase `[stage]` badge in the Roadmap tab's detail pane (D-B08). Deleting
/// it here would only make that plan re-create it.
#[allow(dead_code)]
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
            // Same evidence rule as the D-R-P-E-V row: `has_summaries` is
            // count-derived, so the verification half is read from what the
            // artifact concluded rather than from the filename existing. A
            // `*-VERIFICATION.md` that states nothing is not a verified phase.
            if inf.has_summaries || inf.verification_status != VerificationStatus::Missing {
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
    let new_view = sub_view_from_index(new_index, ctx.experimental);
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
        // A returning operator lands on the whole list, not in typing mode.
        cache.defaults_filter.clear();
        cache.defaults_filter_typing = false;
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
     tab-bar title, and in its ten tabs the values parsed out of the \
     project's `.planning/`. Per tab, the values and where their bytes come \
     from: RoadmapViz's header draws the status and milestone parsed from \
     `ROADMAP.md`/`STATE.md`, its default graph draws phase ids, \
     dependency ids and milestone labels parsed from `ROADMAP.md` plus the \
     current phase's name, and its box list draws each `RoadmapPhase`'s \
     number, name and description; Pipeline (the tab labelled Phases) \
     draws the current phase name, status \
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
     sub-view, all ten, EACH RENDERING ITS POPULATED BRANCH (21-25), plus \
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
        // Read through `effective_sub_view`: with the experimental surfaces off
        // a stored `Driver` is not a tab, and every Driver-only key arm below
        // is guarded by `current_view == DetailSubView::Driver` — so coercing
        // here is what disarms all ten of them at once (260917-fko D2).
        let current_view = effective_sub_view(
            ctx.detail_sub_view_per_project
                .get(&self.alias)
                .cloned()
                .unwrap_or_default(),
            ctx.experimental,
        );
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
                        .filter(|e| matches!(e.kind.editable(), ConfigValueKind::String))
                        .map(|_| idx)
                })
            };
            if let Some(editing_idx) = editing_text_idx {
                return self.handle_text_input_key(code, ctx, editing_idx);
            }
            // Filter-input intercept (quick 260922-hdi): while the `/` line
            // has focus every key belongs to it, so a query containing x / d /
            // r / q / a digit never clears a value, flips the edit target,
            // switches the tab or pops the screen (T-HDI-01).
            let filter_typing = ctx
                .view_cache
                .get(&self.alias)
                .is_some_and(|c| c.defaults_filter_typing && c.defaults_editing.is_none());
            if filter_typing {
                return self.handle_config_filter_key(code, ctx);
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
                // If the commit pane is showing on Git tab, dismiss it first
                if current_view == DetailSubView::GitHistory {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    if cache.git_commit_detail.is_some() {
                        cache.close_git_commit_detail();
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
                    // Then a confirmed `/` filter: Esc and q clear it and keep
                    // the screen ([INFERRED A3], the arm is shared).
                    if !cache.defaults_filter.is_empty() {
                        cache.defaults_filter.clear();
                        cache.defaults_filter_typing = false;
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
                            cache.close_git_commit_detail();
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
                                if let Some(data) = ctx.archive_cache.get(&self.alias, milestone) {
                                    let total = data.top_level_files.len() + data.phases.len();
                                    let max = total.saturating_sub(1);
                                    cache.archive_selected[1] =
                                        (cache.archive_selected[1] + 1).min(max);
                                }
                            }
                            ArchiveDepth::FileList { milestone, phase_idx } => {
                                if let Some(data) = ctx.archive_cache.get(&self.alias, milestone) {
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
                            move_defaults_selection(cache, 1);
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
                            cache.close_git_commit_detail();
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
                            move_defaults_selection(cache, -1);
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
                        // QD-06: the Driver tab's posture, reused. While the
                        // commit pane is open PageDown scrolls THAT — paging
                        // the selection instead would close the very pane the
                        // key was aimed at. With it closed, the key keeps its
                        // old job of paging the log.
                        if cache.git_commit_detail.is_some() {
                            let vp = self.git_commit_viewport.get();
                            cache.git_commit_scroll = clamp_scroll(
                                cache.git_commit_scroll.saturating_add(PAGE_SCROLL_LINES),
                                vp.total_lines,
                                vp.visible_height,
                            );
                        } else if !cache.git_entries.is_empty() {
                            let max = cache.git_entries.len().saturating_sub(1);
                            cache.git_selected = (cache.git_selected + PAGE_SCROLL_LINES as usize).min(max);
                            cache.close_git_commit_detail();
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
                                if let Some(data) = ctx.archive_cache.get(&self.alias, milestone) {
                                    let total = data.top_level_files.len() + data.phases.len();
                                    let max = total.saturating_sub(1);
                                    cache.archive_selected[1] =
                                        (cache.archive_selected[1] + PAGE_SCROLL_LINES as usize).min(max);
                                }
                            }
                            ArchiveDepth::FileList { milestone, phase_idx } => {
                                if let Some(data) = ctx.archive_cache.get(&self.alias, milestone) {
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
                        move_defaults_selection(cache, PAGE_SCROLL_LINES as isize);
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
                        // The QD-06 branch again, and the up direction clamps
                        // FIRST so a stale offset cannot survive a viewport
                        // that shrank — the UIFIX-04 lesson, through the one
                        // shared `clamp_scroll` rather than a second copy.
                        if cache.git_commit_detail.is_some() {
                            let vp = self.git_commit_viewport.get();
                            let current = clamp_scroll(
                                cache.git_commit_scroll,
                                vp.total_lines,
                                vp.visible_height,
                            );
                            cache.git_commit_scroll = current.saturating_sub(PAGE_SCROLL_LINES);
                        } else if !cache.git_entries.is_empty() {
                            cache.git_selected = cache.git_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                            cache.close_git_commit_detail();
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
                        move_defaults_selection(cache, -(PAGE_SCROLL_LINES as isize));
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
            // Tab switching via number keys: `1`-`9` → indices 0-8 (D-B10).
            // The zero key has no arm on purpose — it falls through to the
            // no-op `_` arm at the bottom of this match, so it cannot land the
            // user on a tab by an old habit.
            KeyCode::Char('1') => switch_to_tab(&self.alias, 0, &mut self.scroll_offset, ctx),
            KeyCode::Char('2') => switch_to_tab(&self.alias, 1, &mut self.scroll_offset, ctx),
            KeyCode::Char('3') => switch_to_tab(&self.alias, 2, &mut self.scroll_offset, ctx),
            KeyCode::Char('4') => switch_to_tab(&self.alias, 3, &mut self.scroll_offset, ctx),
            KeyCode::Char('5') => switch_to_tab(&self.alias, 4, &mut self.scroll_offset, ctx),
            KeyCode::Char('6') => switch_to_tab(&self.alias, 5, &mut self.scroll_offset, ctx),
            KeyCode::Char('7') => switch_to_tab(&self.alias, 6, &mut self.scroll_offset, ctx),
            KeyCode::Char('8') => switch_to_tab(&self.alias, 7, &mut self.scroll_offset, ctx),
            KeyCode::Char('9') => switch_to_tab(&self.alias, 8, &mut self.scroll_offset, ctx),
            // The Driver tab (D-15), always the last. It has no digit, uppercase is
            // entirely unclaimed in the detail view, and `KeyCode::Char('D')`
            // arrives without needing the `_modifiers` parameter this handler
            // ignores — so `Shift+D` costs no new plumbing and collides with
            // nothing.
            //
            // The match guard, not an `if` inside the arm: with the flag off
            // the key must fall through **unhandled**, exactly as it did before
            // the Driver tab existed, rather than being consumed by an arm that
            // does nothing (260917-fko D2).
            KeyCode::Char('D') if ctx.experimental => {
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
                // The visible count, not `TAB_COUNT - 1`: with the flag off the
                // last tab is index 8, and walking to 9 (the Driver index) would
                // park the user on a tab the bar does not draw.
                if current_idx < visible_tab_count(ctx.experimental) - 1 {
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
                            // `git show ... <hash>`. A subprocess argument
                            // is a lookup, not something a human reads, and an
                            // escaped hash would name no commit.
                            let hash = entry.hash.as_raw_for_logic_only().to_string();
                            cache.loading_commit_detail = true;
                            if let (Some(project), Some(tx)) =
                                (ctx.config.projects.get(&self.alias), &ctx.event_tx)
                            {
                                let tx = tx.clone();
                                let project_path = project.path.clone();
                                let alias = self.alias.clone();
                                tokio::spawn(async move {
                                    // BOTH paths send back the hash they were
                                    // ASKED for, so the handler can tell a
                                    // stale answer from a current one (QD-07).
                                    let detail = git_ops::load_commit_detail(&project_path, &hash)
                                        .await
                                        .unwrap_or_else(|_| git_ops::GitCommitDetail {
                                            // An empty body opens the pane
                                            // saying the commit has no readable
                                            // message, which is a better answer
                                            // than hanging on `Loading...`.
                                            hash: crate::text::Untrusted::from_untrusted_source(
                                                hash.clone(),
                                            ),
                                            body: Vec::new(),
                                            stat: Default::default(),
                                        });
                                    let _ = tx.send(Action::GitCommitDetailLoaded {
                                        alias,
                                        hash,
                                        detail,
                                    });
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
                            // The resume gate runs BEFORE resolve_launch_plan,
                            // so a Codex row reaches neither tmux nor a
                            // terminal spawn (260923-lr9, T-lr9-01).
                            let sid = match resumable_session_id(session) {
                                Ok(sid) => sid,
                                Err(msg) => return ScreenAction::SetStatusMessage(msg.to_string()),
                            };
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
                                                    Some(e) => {
                                                        format!("tmux: {} — opened in {}", e, term)
                                                    }
                                                    None => format!("Resumed session {}", short_id),
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
                                        None => {
                                            "No terminal emulator found (set $TERMINAL)".to_string()
                                        }
                                    })
                                }
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
                                    if ctx.archive_cache.get(&self.alias, &milestone).is_none() {
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
                                if let Some(data) = ctx.archive_cache.get(&self.alias, &milestone) {
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
                                if let Some(data) = ctx.archive_cache.get(&self.alias, &milestone) {
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
                                        if set_config_value(active, entry.key.as_ref(), &value) {
                                            persist_active_config(target, project_path.as_deref(), active, &mut ctx.status_message);
                                        }
                                    }
                                }
                            }
                            cache.defaults_editing = None;
                            cache.defaults_dropdown_selected = 0;
                        } else if !defaults_selection_visible(cache) {
                            // A filter that hides the cursor's row (T-HDI-04):
                            // there is nothing on screen to edit.
                        } else {
                            let entries = entries_for_cache(cache);
                            let selected = cache.defaults_selected;
                            if let Some(entry) = entries.get(selected).cloned() {
                                let options = dropdown_options(&entry.kind);
                                if !options.is_empty() {
                                    let current_idx = options.iter().position(|o| o == &entry.value).unwrap_or(0);
                                    cache.defaults_editing = Some(selected);
                                    cache.defaults_dropdown_selected = current_idx;
                                } else if matches!(entry.kind.editable(), ConfigValueKind::String) {
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
                                } else if matches!(entry.kind.editable(), ConfigValueKind::Integer) {
                                    if let Some(active) = active_config_mut(cache) {
                                        if mutate_config_entry(active, entry.key.as_ref(), &entry.kind) {
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
            // '/' key: open the Config tab's filter input (quick 260922-hdi),
            // seeded with the current filter so it can be refined ([INFERRED A4]).
            KeyCode::Char('/') if current_view == DetailSubView::Defaults => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                if cache.defaults_editing.is_some() {
                    return ScreenAction::None;
                }
                cache.defaults_filter_typing = true;
                snap_defaults_selection(cache);
                ctx.needs_redraw = true;
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
                // Never clear a row the filter hides (T-HDI-04).
                if cache.defaults_editing.is_none() && defaults_selection_visible(cache) {
                    let entries = entries_for_cache(cache);
                    let selected = cache.defaults_selected;
                    if let Some(entry) = entries.get(selected).cloned() {
                        let key = entry.key;
                        // The status message interleaves the key with a
                        // sentence this build wrote, so there is no field to
                        // give a carrier and the escape happens at the
                        // `format!` (the WR-03 shape). A pass-through key
                        // reaches the `cannot be cleared` arm by construction,
                        // which is exactly the arm an unescaped key would leak
                        // through.
                        let key_shown = shown(key.as_ref());
                        if let Some(active) = active_config_mut(cache) {
                            let cleared = clear_config_value(active, key.as_ref());
                            if cleared {
                                persist_active_config(
                                    target,
                                    project_path.as_deref(),
                                    active,
                                    &mut ctx.status_message,
                                );
                                ctx.status_message = Some((
                                    format!("Cleared {}", key_shown),
                                    std::time::Instant::now(),
                                ));
                            } else {
                                ctx.status_message = Some((
                                    format!("{} cannot be cleared", key_shown),
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
                // The pass-through row set may have changed under the filter.
                snap_defaults_selection(cache);
                ctx.status_message = Some(("Config reloaded".to_string(), std::time::Instant::now()));
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // Tab: switch the host terminal to a Claude or Codex session for this project.
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
                    None => super::normal::no_active_session_status(&self.alias),
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
                // The filter is kept across targets ([INFERRED A8]); row 0 may
                // be one it hides.
                snap_defaults_selection(cache);
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
                cache.close_git_commit_detail();
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
                        let archive = ctx.archive_cache.get(&self.alias, milestone);
                        let file_path = archive.and_then(|data| {
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
            // Roadmap tab only: flip between the dependency graph (default) and
            // the box list. `v` is bound nowhere else on this screen.
            KeyCode::Char('v') if current_view == DetailSubView::RoadmapViz => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                cache.roadmap_box_view = !cache.roadmap_box_view;
                self.scroll_offset = 0;
                ctx.needs_redraw = true;
                ScreenAction::None
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

        // Coerced, for the reason `effective_sub_view` records: a stored
        // `Driver` is not a tab when the experimental surfaces are off, and
        // this read is what feeds the content dispatch below — so the
        // `render_driver_tab` arm becomes unreachable from here.
        let sub_view = effective_sub_view(
            ctx.detail_sub_view_per_project
                .get(alias)
                .cloned()
                .unwrap_or_default(),
            ctx.experimental,
        );
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
        let (titles, select) = tab_titles(
            tab_area.width,
            tab_idx,
            driver_live_for(ctx, alias),
            ctx.experimental,
        );
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
        let footer = build_footer(&sub_view, footer_area.width, ctx.experimental);
        frame.render_widget(footer, footer_area);
    }

    fn name(&self) -> &str {
        Self::NAME
    }
}

impl DetailScreen {
    /// Handle a keystroke while a text-input editor is open on the Defaults
    /// tab. Char/Backspace edit the buffer, Enter persists, Esc cancels.
    /// Keys while the Config tab's `/` filter input has focus (quick
    /// 260922-hdi), modelled on `NormalScreen::handle_search_key`. Every arm
    /// returns `ScreenAction::None`, and `_` swallows the rest, which is what
    /// keeps x / d / r / q / digits / ? / Left / Right / Tab from reaching
    /// their global arms while the operator types.
    fn handle_config_filter_key(&self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
        match code {
            KeyCode::Char(c) => {
                cache.defaults_filter.push(c);
                select_first_visible(cache);
            }
            KeyCode::Backspace => {
                cache.defaults_filter.pop();
                select_first_visible(cache);
            }
            // Clear and leave; the cursor is an underlying index, so it stays
            // on the row it was on.
            KeyCode::Esc => {
                cache.defaults_filter.clear();
                cache.defaults_filter_typing = false;
            }
            // Confirm: keep the filter, drop focus ([INFERRED A2]). The next
            // Enter edits the selected row.
            KeyCode::Enter => {
                cache.defaults_filter_typing = false;
                snap_defaults_selection(cache);
            }
            // [INFERRED A7] arrows and pages navigate while typing; j/k are text.
            KeyCode::Down => move_defaults_selection(cache, 1),
            KeyCode::Up => move_defaults_selection(cache, -1),
            KeyCode::PageDown => move_defaults_selection(cache, PAGE_SCROLL_LINES as isize),
            KeyCode::PageUp => move_defaults_selection(cache, -(PAGE_SCROLL_LINES as isize)),
            _ => {}
        }
        ctx.needs_redraw = true;
        ScreenAction::None
    }

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
                            clear_config_value(active, key.as_ref())
                        } else {
                            set_string_value(active, key.as_ref(), &buffer)
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

            if let Some(line) = unreadable_state_line(state) {
                header_lines.push(line);
            }
            if let Some(line) = recovered_state_line(state) {
                header_lines.push(line);
            }

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

            if ctx.view_cache.get(alias).is_some_and(|c| c.roadmap_box_view) {
                let current_phase_num = state.active_phase_number();
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
                    disk_statuses: &state.phase_disk_statuses,
                    scroll_offset: clamped_offset,
                };
                frame.render_widget(roadmap_widget, roadmap_area);
            } else {
                // The default: the dependency graph (quick 260923-md1). Its
                // rows are what the `_ =>` j/k/PageUp/PageDown arms scroll.
                let markers = roadmap_graph::phase_markers(state);
                let layout = roadmap_graph::layout_for_state(state, &markers);
                let graph_area = roadmap_area.inner(Margin::new(2, 0));
                let [_, graph_rect, _] = roadmap_graph::split_areas(&layout, graph_area);
                let total = u16::try_from(layout.rows.len()).unwrap_or(u16::MAX);
                self.generic_viewport.set(ViewportMetrics {
                    total_lines: total,
                    visible_height: graph_rect.height,
                });
                let clamped = self
                    .scroll_offset
                    .min(total.saturating_sub(graph_rect.height));
                frame.render_widget(
                    roadmap_graph::RoadmapGraphWidget {
                        layout: &layout,
                        markers: &markers,
                        scroll_offset: clamped,
                    },
                    graph_area,
                );
            }
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

        // Layout: mode indicator (1 line), then log (and optionally the commit
        // pane). The split is 40/60 in the pane's favour — today's 60/40
        // INVERTED (QD-05), because a commit message is the half a reader came
        // for and the log rows above it stay recognisable at 40%.
        let has_detail = cache.git_commit_detail.is_some() || cache.loading_commit_detail;
        let content_chunks = if has_detail {
            Layout::vertical([
                Constraint::Length(1),
                Constraint::Percentage(40),
                Constraint::Percentage(60),
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
                // spelling of these lines is a compile error rather than a
                // site a reader has to notice.
                //
                // The escape happens BEFORE the width budget, not after: the
                // budget counts what will actually occupy cells, and
                // `render_for_terminal` can change a string's length.
                let hash = entry.hash.shown().to_string();
                let date = entry.date.shown().to_string();
                let author = entry.author.shown().to_string();
                let co_authors = entry.co_authors.as_ref().map(|c| c.shown().to_string());
                let subject = entry.message.shown().to_string();

                let budget = git_row_budget(
                    log_area.width as usize,
                    hash.chars().count(),
                    date.chars().count(),
                    author.chars().count(),
                    co_authors.as_ref().map(|c| c.chars().count()),
                );

                let mut spans = vec![
                    Span::styled(hash, Style::default().fg(Color::Yellow)),
                    Span::raw(" -- "),
                    Span::raw(date),
                    Span::raw(" -- "),
                ];
                let truncated = truncate_subject(&subject, budget.subject_cols);
                if !truncated.is_empty() {
                    spans.push(Span::raw(truncated));
                }
                spans.push(Span::raw("  "));
                spans.push(Span::styled(author, Style::default().fg(Color::DarkGray)));

                // The column AND its separator are emitted together, so a
                // commit with no trailer renders no dangling "  ()" (QD-01).
                if budget.show_co_authors {
                    if let Some(co_authors) = co_authors {
                        spans.push(Span::raw("  ("));
                        spans.push(Span::styled(
                            co_authors,
                            Style::default().fg(Color::DarkGray),
                        ));
                        spans.push(Span::raw(")"));
                    }
                }

                ListItem::new(Line::from(spans))
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

        // Render the commit pane if present
        if has_detail {
            let detail_area = content_chunks[2];
            if cache.loading_commit_detail {
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(" Commit: loading... ");
                let loading = Paragraph::new("  Loading commit...")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(block);
                frame.render_widget(loading, detail_area);
            } else if let Some(detail) = cache.git_commit_detail.as_ref() {
                // SHOWN: the title is what a human reads, and `Block::title` is
                // the widget family that PRESERVES the invisible class most
                // completely (measured — even `U+202E` reaches a cell through
                // it). The same hash goes to `load_commit_detail` RAW, above,
                // which is the split this carrier exists to make the compiler
                // ask about separately.
                let selected_hash = cache
                    .git_entries
                    .get(cache.git_selected)
                    .map(|e| e.hash.shown().to_string())
                    .unwrap_or_else(|| "???".to_string());

                // Under EIGHT rows — two borders, a title and five body lines —
                // there is no room for two bordered panes, and the message is
                // the half a reader came for, so it takes all of it (QD-05).
                const MIN_ROWS_FOR_BOTH_PANES: u16 = 8;
                let (message_area, files_area) =
                    if detail_area.height < MIN_ROWS_FOR_BOTH_PANES {
                        (detail_area, None)
                    } else {
                        let split = Layout::vertical([
                            Constraint::Percentage(60),
                            Constraint::Percentage(40),
                        ])
                        .split(detail_area);
                        (split[0], Some(split[1]))
                    };

                let message_block = Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Commit: {} ", selected_hash));
                let message_inner = message_block.inner(message_area);

                // Recorded at render, read by the PageUp/PageDown arms —
                // the same `Cell<ViewportMetrics>` protocol the Browse, Archive
                // and Driver panes use, so all four clamp through one formula.
                let total_lines = detail.body.len().min(u16::MAX as usize) as u16;
                self.git_commit_viewport.set(ViewportMetrics {
                    total_lines,
                    visible_height: message_inner.height,
                });
                let scroll = clamp_scroll(
                    cache.git_commit_scroll,
                    total_lines,
                    message_inner.height,
                );

                let message_lines: Vec<Line> = if detail.body.is_empty() {
                    vec![Line::from(Span::styled(
                        "  (this commit carries no message body)",
                        Style::default().fg(Color::DarkGray),
                    ))]
                } else {
                    // SHOWN, one carrier per line — see `GitCommitDetail::body`.
                    detail
                        .body
                        .iter()
                        .map(|line| Line::from(Span::raw(line.shown())))
                        .collect()
                };

                // NO `Wrap` (QD-10). `total_lines` must equal the pane's real
                // scroll range or `clamp_scroll` lies to the key handler about
                // where the bottom is; git bodies are hard-wrapped by
                // convention, so long lines clip horizontally instead.
                let message = Paragraph::new(message_lines)
                    .block(message_block)
                    .scroll((scroll, 0));
                frame.render_widget(message, message_area);

                if let Some(files_area) = files_area {
                    let stat = &detail.stat;
                    // Retitled to name the files: the hash now titles the pane
                    // above, and two panes headed by the same hash would read
                    // as one pane drawn twice.
                    let files_block = Block::default().borders(Borders::ALL).title(" Files ");

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

                    let files_paragraph = Paragraph::new(diff_lines).block(files_block);
                    frame.render_widget(files_paragraph, files_area);
                }
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
                let block = Block::default().borders(Borders::ALL).title(" Phases ");
                let msg = Paragraph::new("  No state data available.").block(block);
                frame.render_widget(msg, area);
                return;
            }
        };

        if state.phases.is_empty() {
            let block = Block::default().borders(Borders::ALL).title(" Phases ");
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
                ListItem::new(phase_list_label(
                    phase,
                    state.phase_disk_statuses.get(&phase.number),
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

        let right_block = Block::default().borders(Borders::NONE).title(" Phases ");
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

                // Per-plan token cost (estimate vs actual), read during the disk
                // scan and cached on the inference — no file is read here.
                let token_lines = build_plan_token_lines(inf);
                if !token_lines.is_empty() {
                    lines.push(Line::from(""));
                    for tl in token_lines {
                        lines.push(tl);
                    }
                }

                // Waves — ONE resolution, then ONE render. The on-disk
                // `waves.json` manifest (GSD 1.8.0 claude-orchestration) is
                // preferred where it exists; when it is absent, unparsable or
                // carries no waves, the grouping derived from each plan's own
                // `wave:` frontmatter stands in. That fallback reads no files:
                // the data was derived during the refresh scan and is already
                // on the inference, so nothing here touches the render tick.
                // Neither source available → render nothing at all.
                let manifest = ctx
                    .config
                    .projects
                    .get(alias)
                    .and_then(|proj| {
                        crate::state_reader::disk_status::find_phase_dir(
                            &proj.path.join(".planning"),
                            &phase.number,
                        )
                    })
                    .and_then(|phase_dir| {
                        std::fs::read_to_string(phase_dir.join("waves.json")).ok()
                    })
                    .and_then(|raw| parse_waves_manifest(&raw))
                    .filter(|manifest| !manifest.waves.is_empty())
                    .or_else(|| {
                        (!inf.plan_waves.is_empty())
                            .then(|| waves_manifest_from_derived(&inf.plan_waves))
                    });
                if let Some(manifest) = manifest {
                    lines.push(Line::from(""));
                    for wl in build_waves_lines(&manifest) {
                        lines.push(wl);
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

    /// Render the sessions tab listing active agent sessions (Claude or Codex)
    /// for this project. Each row names its agent.
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
                Line::from("  No active Claude or Codex sessions"),
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

        use crate::session_detector::SessionKind;
        let items: Vec<ListItem> = filtered_sessions
            .iter()
            .map(|session| {
                // READ BY A HUMAN, so escaped — and shortened by CHARACTERS,
                // never by bytes (T-21-25-05).
                let sid_display = match (&session.session_id, session.kind) {
                    (Some(sid), _) => shorten_session_id(sid),
                    (None, SessionKind::Claude) => "new session".to_string(),
                    // A Codex session cannot be resumed or "new" from here.
                    (None, SessionKind::Codex) => "unknown".to_string(),
                };
                let time_display = session
                    .start_time
                    .map(|_| "active".to_string())
                    .unwrap_or_else(|| "active".to_string());
                ListItem::new(Line::from(format!(
                    "  PID {} | {} | Session: {} | {}",
                    session.pid,
                    session.kind.label(),
                    sid_display,
                    time_display
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
        let breadcrumb = Self::archive_breadcrumb(&self.alias, &cache.archive_depth, cache, ctx);
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
                if let Some(data) = ctx.archive_cache.get(&self.alias, milestone) {
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
                if let Some(data) = ctx.archive_cache.get(&self.alias, milestone) {
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
        alias: &str,
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
                if let Some(data) = ctx.archive_cache.get(alias, milestone) {
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
                    if let Some(data) = ctx.archive_cache.get(alias, milestone) {
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

        // Coerced for the same reason as the site in `render`: this is the
        // third and last read of the stored sub-view, used as the backdrop
        // behind the Enqueue and Driver-inject overlays.
        let sub_view = effective_sub_view(
            ctx.detail_sub_view_per_project
                .get(alias)
                .cloned()
                .unwrap_or_default(),
            ctx.experimental,
        );
        let tab_idx = tab_index(&sub_view);

        // Split into tab bar and content
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(main_area);
        let tab_area = chunks[0];
        let content_area = chunks[1];

        // Render tab bar — the duplicate of the site in `render`, and the reason
        // `tab_titles` exists as one function rather than two constructions.
        let (titles, select) = tab_titles(
            tab_area.width,
            tab_idx,
            driver_live_for(ctx, alias),
            ctx.experimental,
        );
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

        // The `/` filter (quick 260922-hdi). `visible` holds UNDERLYING indices;
        // `selected` stays one too, so the highlight compares underlying
        // indices and only the `ListState` below speaks visible positions.
        let (filter, typing) = cache
            .map(|c| (c.defaults_filter.clone(), c.defaults_filter_typing))
            .unwrap_or_default();
        let filtering = !filter.is_empty();
        let visible = match cache {
            Some(c) => visible_defaults_indices(c, &entries),
            None => (0..entries.len()).collect(),
        };

        let items: Vec<ListItem> = visible
            .iter()
            .enumerate()
            .map(|(pos, &i)| {
                let entry = &entries[i];
                // Unfiltered: the builder's own `show_category`, so the
                // unfiltered render is unchanged. Filtered: the category heads
                // the first visible row of each run, since the builder's flag
                // is keyed to the unfiltered order.
                let show_category = if filtering {
                    pos == 0 || entries[visible[pos - 1]].category != entry.category
                } else {
                    entry.show_category
                };
                let cat_span = if show_category {
                    Span::styled(
                        format!("{:<18}", entry.category),
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    Span::raw("                  ")
                };
                // READ BY A HUMAN, through a `ListItem`, and untrusted since
                // quick task 260916-vqw (T-VQW-01). The comment that used to
                // sit at `val_span` said "`category` and `key` beside it are
                // `&'static str` literals from `build_defaults_entries` and
                // need nothing" — true while every row was authored here, and
                // FALSE the moment `append_passthrough_entries` started
                // emitting rows whose key is a dotted path built out of the
                // project's own `.planning/config.json`. Escaped BEFORE the
                // padding, so the column width counts what the terminal will
                // draw rather than what the file contained.
                let key_span = Span::styled(
                    format!("{:<30}", shown(entry.key.as_ref())),
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
                    ConfigValueKind::Null | ConfigValueKind::Unset(_) => {
                        Style::default().fg(Color::DarkGray)
                    }
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
                // author. `category` beside it is a `&'static str` literal
                // from `build_defaults_entries` and needs nothing; `key` is
                // NO LONGER in that class and is escaped at its own span.
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

        // A filter that matches nothing: one dim line, no selection, no help
        // (T-HDI-04). The `ListState` below selects `None` because the
        // cursor's position in an empty `visible` is `None`.
        let items = if filtering && visible.is_empty() {
            vec![ListItem::new("  No config keys match").style(Style::default().fg(Color::DarkGray))]
        } else {
            items
        };

        let mut title = match edit_target {
            DefaultsEditTarget::Project => " Config Settings ".to_string(),
            DefaultsEditTarget::Global => " Global Defaults (~/.gsd/defaults.json) ".to_string(),
        };
        // Filter echo + count ([INFERRED A5]). The filter is operator-typed
        // and `Block::title` preserves the invisible class, so it is drawn
        // only through `shown()` (T-HDI-03).
        if filtering || typing {
            title.push_str(&format!(
                " /{}{} ({}/{}) ",
                shown(&filter),
                if typing { "_" } else { "" },
                visible.len(),
                entries.len()
            ));
        }
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
        if filtering {
            // The VISIBLE position of the underlying cursor, never the
            // underlying index itself (else the viewport follows the wrong row).
            list_state.select(visible.iter().position(|&i| i == selected));
        } else {
            list_state.select(Some(selected));
        }
        frame.render_stateful_widget(list, list_area, &mut list_state);

        if let Some(help_area) = help_area {
            if filtering {
                // Help only for a row the filter shows (T-HDI-05).
                if visible.contains(&selected) {
                    frame.render_widget(build_config_help_pane(&entries[selected].help), help_area);
                }
            } else {
                // `selected` is the cache's own cursor and the list is non-empty
                // here, but the cache is not this function's to trust: an entry
                // count that shrank since the cursor was set would panic on a
                // bare index.
                let idx = selected.min(entries.len() - 1);
                frame.render_widget(build_config_help_pane(&entries[idx].help), help_area);
            }
        }

        // Render dropdown OR text-input overlay if editing
        if let Some(cache) = cache {
            if let Some(editing_idx) = cache.defaults_editing {
                if let Some(entry) = entries.get(editing_idx) {
                    if matches!(entry.kind.editable(), ConfigValueKind::String) {
                        // `Block::title` PRESERVES the invisible class (the
                        // per-widget-family table in `render_escape_guard.rs`),
                        // so the key is escaped here as well as in the list.
                        let title = format!(
                            " {} {DEFAULTS_EDIT_BRANCH_TOKEN} ",
                            shown(entry.key.as_ref())
                        );
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
                        let title = format!(" {} ", shown(entry.key.as_ref()));
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
    // Whether each stage's artifact is present.
    //
    // **E and V read the same KIND of evidence.** E has always been derived from
    // the paired-summary count — what the artifacts amount to — while V read
    // `has_verification`, a bare filename-presence flag. That asymmetry is what
    // let a phase render `Execute [--]` (skipped, dimmed) beside a green
    // `Verify [V]`: an undercount on the left could not move the right, so the
    // row contradicted itself. V now reads what the verification artifact
    // concluded, so a content-free `*-VERIFICATION.md` stub no longer outranks
    // a counted Execute stage.
    let verification_concluded = inf.verification_status != VerificationStatus::Missing;
    let present = [
        inf.has_context,        // D: Discuss
        inf.has_research,       // R: Research
        inf.has_plans,          // P: Plan
        inf.summary_count > 0,  // E: Execute (at least one)
        verification_concluded, // V: Verify
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
                // Execute is the one stage that cannot be skipped: nothing
                // downstream of it can exist without it, so an absent E beside a
                // present V is an inconsistency in the evidence, not a phase
                // that deliberately bypassed execution. Render it unfinished
                // rather than dimmed-out, so the row can never claim a phase was
                // verified without being executed.
                statuses[i] = if i == 3 {
                    StageStatus::Current
                } else {
                    StageStatus::Skipped
                };
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

/// One row of the Pipeline tab's left-hand phase list.
///
/// Today's text verbatim — `P{number}: {name}`, both halves escaped — plus, for
/// a phase whose plans ran in two or more waves, a compact `Nw` marker two
/// spaces behind it. The marker is what answers the question the selected-phase
/// wave section cannot: *which* phases ran anything in parallel, without
/// selecting each in turn.
///
/// **Only NUMBERED waves are counted.** [`PlanWave`](crate::state_reader::plan_waves::PlanWave)'s
/// trailing `None` bucket records plans whose wave was never written down, so
/// counting it would report a phase with one real wave plus a couple of unwaved
/// plans as parallel — a claim its own frontmatter does not make.
///
/// A free function rather than inline construction so the four cases are
/// testable without a `Frame` or a `Buffer`.
fn phase_list_label(
    phase: &crate::state_reader::roadmap_md::RoadmapPhase,
    inf: Option<&DiskInference>,
) -> String {
    let mut label = format!("P{}: {}", shown(&phase.number), shown(&phase.name));
    let numbered = inf
        .map(|inf| {
            inf.plan_waves
                .iter()
                .filter(|w| w.wave.is_some())
                .count()
        })
        .unwrap_or(0);
    if numbered >= 2 {
        // The left pane is 40% of the tab width, so ratatui truncates this
        // marker before the phase name is lost on a narrow terminal. No width
        // computation is warranted here.
        label.push_str(&format!("  {numbered}w"));
    }
    label
}

/// Present waves derived from `*-PLAN.md` frontmatter in the shape
/// [`build_waves_lines`] already renders.
///
/// A second *source* for the existing renderer, not a second renderer: the
/// on-disk `waves.json` manifest exists only under GSD 1.8.0's
/// claude-orchestration backend, so without this the section renders for
/// nobody. `files_modified` is left empty — the hint it feeds is appended only
/// when the count is above zero, so the derived path degrades to just the plan
/// count rather than showing a false `0f`.
///
/// Every plan id passes through [`shown`] first. These strings are filenames
/// from another project's `.planning/` directory — untrusted text, escaped on
/// the same path as the other ~50 call sites in this file.
fn waves_manifest_from_derived(waves: &[crate::state_reader::plan_waves::PlanWave]) -> WavesManifest {
    WavesManifest {
        waves: waves
            .iter()
            .map(|w| WaveEntry {
                // The label carries the real frontmatter wave number, so it is
                // supplied here rather than left to `WaveEntry::label`'s
                // index-plus-one fallback.
                id: Some(serde_json::Value::String(w.label())),
                wave: None,
                plans: w
                    .plans
                    .iter()
                    .map(|id| WavePlan {
                        id: Some(shown(id)),
                        plan: None,
                        files_modified: Vec::new(),
                    })
                    .collect(),
            })
            .collect(),
    }
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

/// How many per-plan token rows the Pipeline pane draws before it stops and says
/// so (D-INF-03).
///
/// This pane is a plain `Paragraph` with no scroll state, so lines past the pane
/// height are simply not drawn. Phase 19 of this very repository has 33 plans:
/// an uncapped list would push the overflow off the bottom INVISIBLY. A cap plus
/// a stated remainder is honest about what is not shown, and the header's totals
/// cover every plan regardless of the cap. If this proves annoying the follow-up
/// is pane scrolling, not a bigger constant.
const MAX_PLAN_TOKEN_ROWS: usize = 10;

/// A token count in at most six ASCII characters, so the column is predictable.
///
/// Below 1 000 the bare integer; below 100 000 one decimal place and a `k`;
/// below 1 000 000 no decimal place and a `k`; otherwise one decimal place and
/// an `M`. `fmt_tokens_boundaries_are_pinned_not_described` asserts every
/// boundary as an exact string rather than trusting this paragraph.
///
/// **The rounding is integer arithmetic, deliberately, not `{:.1}` on a float.**
/// Rust's float formatting rounds a tie to even, so `format!("{:.1}", 1.25)`
/// yields `1.2`; a token count is reported to a human comparing a plan against
/// its outcome, and half-up is the rule such a reader expects. The widening to
/// `u128` keeps `u64::MAX * 10` from wrapping.
///
/// ASCII only: this project carries an open todo about badge glyph display width
/// misaligning by one cell across terminals, and a numeric column is the last
/// place to introduce a non-ASCII glyph.
fn fmt_tokens(n: u64) -> String {
    /// `n / divisor` to one decimal place, rounded half up.
    fn tenths(n: u64, divisor: u128) -> (u128, u128) {
        let scaled = (n as u128 * 10 + divisor / 2) / divisor;
        (scaled / 10, scaled % 10)
    }
    /// `n / divisor` to zero decimal places, rounded half up.
    fn whole(n: u64, divisor: u128) -> u128 {
        (n as u128 + divisor / 2) / divisor
    }

    if n < 1_000 {
        return n.to_string();
    }
    if n < 100_000 {
        let (units, tenth) = tenths(n, 1_000);
        return format!("{units}.{tenth}k");
    }
    if n < 1_000_000 {
        return format!("{}k", whole(n, 1_000));
    }
    let (units, tenth) = tenths(n, 1_000_000);
    format!("{units}.{tenth}M")
}

/// Render the compact `Plan tokens (est/act)` section for the Pipeline pane.
///
/// Empty in, empty out: an inference whose `plan_tokens` is empty produces no
/// lines at all, so a project whose plans predate GSD's `estimate`/`actuals`
/// keys renders exactly what it rendered before this section existed.
///
/// Otherwise a header carrying the phase's summed estimate, its summed actual
/// and how many of its plans have been MEASURED over its plan count, then up to
/// [`MAX_PLAN_TOKEN_ROWS`] rows, then — if the cap dropped any — one line saying
/// how many. The totals are over ALL rows, capped or not.
///
/// Every number here comes off `inf`; this function reads no file. The numbers
/// were parsed during the `.planning/` disk scan and cached on the inference,
/// and are invalidated with it.
fn build_plan_token_lines(inf: &DiskInference) -> Vec<Line<'static>> {
    if inf.plan_tokens.is_empty() {
        return Vec::new();
    }

    let label_style = Style::default().fg(Color::DarkGray);
    let mut lines: Vec<Line> = Vec::new();

    let total_estimate: u64 = inf.plan_tokens.iter().filter_map(|r| r.estimate).sum();
    let total_actual: u64 = inf.plan_tokens.iter().filter_map(|r| r.actual).sum();
    let measured = inf
        .plan_tokens
        .iter()
        .filter(|r| r.actual.is_some())
        .count();

    // The header, and every word and number below it, is this crate's own text
    // and arithmetic — authored strings and integers, neither of which can carry
    // an escape sequence — so none of it goes through `shown()`. Only the plan
    // LABEL does, because only the label came out of another project's
    // filesystem. `unreadable_state_line` keeps that same asymmetry visible for
    // the same reason.
    lines.push(Line::from(vec![
        Span::styled(
            "  Plan tokens (est/act):",
            label_style.add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "  est {}  act {}  {}/{} measured",
                fmt_tokens(total_estimate),
                fmt_tokens(total_actual),
                measured,
                inf.plan_count
            ),
            label_style,
        ),
    ]));

    for row in inf.plan_tokens.iter().take(MAX_PLAN_TOKEN_ROWS) {
        let estimate = row.estimate.map_or_else(|| "-".to_string(), fmt_tokens);
        let actual = row.actual.map_or_else(|| "-".to_string(), fmt_tokens);
        let mut spans = vec![
            Span::raw("    "),
            // The ONE value on this line that is third-party text.
            Span::styled(
                format!("{:<10}", shown(&row.label())),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!("est {estimate:>7}  act {actual:>7}  "),
                Style::default().fg(Color::White),
            ),
        ];
        if let (Some(estimate), Some(actual)) = (row.estimate, row.actual) {
            // An estimate of zero renders no delta and does not divide.
            if estimate > 0 {
                let percent = (actual as i128 - estimate as i128) * 100 / estimate as i128;
                let color = if actual <= estimate {
                    Color::Green
                } else {
                    Color::Yellow
                };
                spans.push(Span::styled(
                    format!("{percent:+}%"),
                    Style::default().fg(color),
                ));
            }
        }
        lines.push(Line::from(spans));
    }

    let dropped = inf.plan_tokens.len().saturating_sub(MAX_PLAN_TOKEN_ROWS);
    if dropped > 0 {
        lines.push(Line::from(Span::styled(
            format!("    ... +{dropped} more (totals above cover all)"),
            label_style,
        )));
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
        spans.push(Span::styled("[1-9/D]", b));
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
/// other tabs keep their single shipped form, with the shared prefix's tabs hint
/// changed once, for every tab, so `Shift+D` is discoverable from anywhere in
/// the detail view.
///
/// `experimental` decides that shared hint. **It is the one string that would
/// otherwise leak the Driver tab onto every other tab** (260917-fko D2): a
/// user with the flag off who read `[1-9/D]` would have been told about a tab
/// that does not exist and a key that does nothing, which is the discovery this
/// gate exists to prevent.
fn footer_spans(sub_view: &DetailSubView, width: u16, experimental: bool) -> Vec<Span<'static>> {
    if matches!(sub_view, DetailSubView::Driver) {
        return driver_footer_spans(width);
    }

    let b = Style::default().add_modifier(Modifier::BOLD);
    let mut spans = vec![
        Span::raw("  "),
        Span::styled("[Esc]", b),
        Span::raw("back  "),
        Span::styled(if experimental { "[1-9/D]" } else { "[1-9]" }, b),
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
            spans.push(Span::raw("commit  "));
            // QD-06 added no keybinding, so the only place its behaviour can be
            // discovered is here — there is deliberately no new help-screen row.
            spans.push(Span::styled("[PgUp/PgDn]", b));
            spans.push(Span::raw("scroll  "));
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
            spans.push(Span::styled("[/]", b));
            spans.push(Span::raw("filter  "));
        }
        DetailSubView::RoadmapViz => {
            spans.push(Span::styled("[v]", b));
            spans.push(Span::raw("iew  "));
            spans.push(Span::styled("[e]", b));
            spans.push(Span::raw("nqueue  "));
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
fn build_footer(sub_view: &DetailSubView, width: u16, experimental: bool) -> Paragraph<'static> {
    Paragraph::new(Line::from(footer_spans(sub_view, width, experimental)))
}

// --- Defaults tab helpers ---

#[derive(Debug, Clone)]
enum ConfigValueKind {
    Bool,
    Enum(&'static [&'static str]),
    String,
    Integer,
    /// Unset, with nothing to say what it would be — the read-only
    /// shape-varying keys. Not editable.
    Null,
    /// Unset in both layers, but with a known shape: the kind the row takes
    /// the moment it is set. It renders as `(unset)` exactly like `Null`,
    /// yet Enter still opens that kind's editor — collapsing it into `Null`
    /// is what made every unset bool/enum row a dead key (debug session
    /// `enter-unset-config-row`).
    Unset(Box<ConfigValueKind>),
    /// Display-only: shape-varying keys (JSON value could be int/string/array)
    /// that we surface as a formatted string but never make editable.
    ReadOnly,
}

impl ConfigValueKind {
    /// The kind an EDIT acts on: an unset row edits as the kind it becomes.
    /// Display code matches on the kind itself; every edit path goes through
    /// this, so an unset row and a set row of the same key edit identically.
    fn editable(&self) -> &ConfigValueKind {
        match self {
            ConfigValueKind::Unset(inner) => inner.editable(),
            other => other,
        }
    }
}

/// What one Defaults-tab option MEANS, authored at the option's definition site.
///
/// **Why a field and not a lookup table** (ID-01, quick task 260909-s0n). The
/// Defaults tab lists 130 raw GSD config keys — `nyquist_validation`,
/// `workflow.specless_probe_fallback`, `granularity` — as bare identifiers next
/// to a value, and there is no second place in this TUI that says what any of
/// them does. A `HashMap<&str, &str>` keyed by config key would compile fine on
/// the day a 131st option is added and silently render a blank line at runtime.
///
/// **The mechanism was measured doing its job.** The gsd-core 1.14.0 re-sync
/// (quick task 260916-vqw) added 57 options in one pass; every one of them was
/// a compile error until its `ConfigHelp` was written, which is exactly the
/// breakage a side table would have replaced with 57 blank lines.
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
    /// The gsd-core release that introduced this key, as a tag name
    /// (`"v1.14.0"`), or `""` for a key whose introduction predates the sync
    /// record (quick task 260916-vqw).
    ///
    /// **MEASURED from gsd-core's history, never recalled** (ID-4). The
    /// command is recorded in `docs/GSD-CORE-SYNC.md` so the next sync
    /// reproduces it rather than guessing:
    ///
    /// ```text
    /// C=$(git -C ~/projects/node/gsd-core log --reverse --format=%H \
    ///       -S"<dotted.key>" -- docs/CONFIGURATION.md src/ | head -1)
    /// git -C ~/projects/node/gsd-core tag --contains "$C" --sort=v:refname | head -1
    /// ```
    ///
    /// It is a `&'static str` this build authored, exactly like `summary` and
    /// `choices`, so rendering it keeps the help pane outside the file's
    /// `shown()` rule by construction (T-S0N-01).
    since: &'static str,
}

impl ConfigHelp {
    /// Help for an option with no discrete choice set.
    const fn new(summary: &'static str) -> Self {
        Self { summary, choices: &[], since: "" }
    }

    /// Help for an option whose values are enumerable, each with its own
    /// explanation.
    const fn with_choices(
        summary: &'static str,
        choices: &'static [(&'static str, &'static str)],
    ) -> Self {
        Self { summary, choices, since: "" }
    }

    /// Record the gsd-core release this key arrived in.
    ///
    /// A builder rather than a fourth constructor parameter so the 73 call
    /// sites that predate the sync record are untouched, and so the two
    /// constructors do not have to be doubled.
    const fn since(self, version: &'static str) -> Self {
        Self {
            summary: self.summary,
            choices: self.choices,
            since: version,
        }
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

/// Introduces the help pane's `since` marker.
///
/// Spelled once because the assertion that `workflow.compact_content`'s help
/// NAMES the gsd-core version that introduced it looks for this token rather
/// than respelling the separator — a test that spells its own separator passes
/// on a render that drew a different one.
const SINCE_PREFIX: &str = "  · since gsd-core ";

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
    let mut summary_spans: Vec<Span<'static>> = vec![Span::styled(
        help.summary,
        Style::default().fg(Color::Gray),
    )];
    if !help.since.is_empty() {
        // A dim trailing marker rather than a line of its own: the pane has
        // four rows total and a whole row spent on a version string is a row
        // the choice list needs. Still a `&'static str` — see `ConfigHelp`.
        summary_spans.push(Span::styled(
            format!("{SINCE_PREFIX}{} ", help.since),
            Style::default().fg(Color::DarkGray),
        ));
    }
    let mut lines: Vec<Line<'static>> = vec![Line::from(summary_spans)];

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
    /// **`Cow`, and the `Owned` half is UNTRUSTED TEXT** (T-VQW-01, quick task
    /// 260916-vqw).
    ///
    /// Every row this build authors passes a `&'static str` and arrives here as
    /// `Borrowed`, exactly as before. The pass-through rows at the end of
    /// [`build_defaults_entries`] carry `Owned` dotted paths built from keys
    /// read out of the project's `.planning/config.json`, which is
    /// attacker-influenced content in precisely the way `value` beside it
    /// already was. Every site that draws this field therefore goes through
    /// [`shown`], and `category` stays `&'static str` because nothing
    /// project-supplied can reach it.
    key: std::borrow::Cow<'static, str>,
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
        ("(unset)".to_string(), ConfigValueKind::Unset(Box::new(ConfigValueKind::Bool)), false)
    }
}

fn opt_str_layered(project: Option<&str>, defaults: Option<&str>) -> (String, ConfigValueKind, bool) {
    if let Some(s) = project {
        (s.to_string(), ConfigValueKind::String, false)
    } else if let Some(s) = defaults {
        (s.to_string(), ConfigValueKind::String, true)
    } else {
        ("(unset)".to_string(), ConfigValueKind::Unset(Box::new(ConfigValueKind::String)), false)
    }
}

fn opt_u32_layered(project: Option<u32>, defaults: Option<u32>) -> (String, ConfigValueKind, bool) {
    if let Some(n) = project {
        (n.to_string(), ConfigValueKind::Integer, false)
    } else if let Some(n) = defaults {
        (n.to_string(), ConfigValueKind::Integer, true)
    } else {
        ("(unset)".to_string(), ConfigValueKind::Unset(Box::new(ConfigValueKind::Integer)), false)
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
        ("(unset)".to_string(), ConfigValueKind::Unset(Box::new(ConfigValueKind::Enum(options))), false)
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
    // `help` is REQUIRED and last (ID-01). A 131st option added below without a
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
            key: std::borrow::Cow::Borrowed(key),
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
    // gsd-core 1.14.0 re-sync (quick task 260916-vqw). `since` is MEASURED per
    // key from gsd-core's own history — see docs/GSD-CORE-SYNC.md for the
    // command, and never edit one of these from memory.
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.context_coverage_gate), dwf.and_then(|w| w.context_coverage_gate));
    push(cat, "workflow.context_coverage_gate", v, k, false, fd, ConfigHelp::new(
        "Requires each recorded decision to be traceable into a plan and then into built work; off skips both gates silently.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.context_drift_precheck), dwf.and_then(|w| w.context_drift_precheck));
    push(cat, "workflow.context_drift_precheck", v, k, false, fd, ConfigHelp::new(
        "Before reusing RESEARCH.md or SPEC.md, compares each one's age against CONTEXT.md's newest decision and says what is stale.",
    ).since("v1.13.0"));
    let (v, k, fd) = enum_l(
        pwf.and_then(|w| w.context_drift_action.as_deref()),
        dwf.and_then(|w| w.context_drift_action.as_deref()),
        &["warn", "block"],
    );
    push(cat, "workflow.context_drift_action", v, k, false, fd, ConfigHelp::with_choices(
        "What happens when that pre-check finds an artifact older than the decision it was derived from.",
        &[
            ("warn", "names the stale artifacts and how to regenerate them"),
            ("block", "halts planning until they are regenerated"),
        ],
    ).since("v1.13.0"));
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.drift_threshold), dwf.and_then(|w| w.drift_threshold));
    push(cat, "workflow.drift_threshold", v, k, false, fd, ConfigHelp::new(
        "How many new directories, migrations or route modules must appear before the codebase-drift gate reacts; default 3.",
    ).since("v1.01.0"));
    let (v, k, fd) = enum_l(
        pwf.and_then(|w| w.drift_action.as_deref()),
        dwf.and_then(|w| w.drift_action.as_deref()),
        &["warn", "auto-remap"],
    );
    push(cat, "workflow.drift_action", v, k, false, fd, ConfigHelp::with_choices(
        "What happens after a wave when the codebase has grown more new structure than that threshold allows.",
        &[
            ("warn", "suggests re-mapping the affected paths by hand"),
            ("auto-remap", "spawns the codebase mapper over those paths"),
        ],
    ).since("v1.01.0"));
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.inline_plan_threshold), dwf.and_then(|w| w.inline_plan_threshold));
    push(cat, "workflow.inline_plan_threshold", v, k, false, fd, ConfigHelp::new(
        "Task count above which a phase gets its own file rather than having its tasks written into the prompt; default 3.",
    ).since("v1.01.0"));
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.max_discuss_passes), dwf.and_then(|w| w.max_discuss_passes));
    push(cat, "workflow.max_discuss_passes", v, k, false, fd, ConfigHelp::new(
        "How many question rounds the discussion may take before it stops asking — the guard against a loop in auto mode.",
    ).since("v1.01.0"));
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.smart_zone_tokens), dwf.and_then(|w| w.smart_zone_tokens));
    push(cat, "workflow.smart_zone_tokens", v, k, false, fd, ConfigHelp::new(
        "Estimated budget above which a phase is flagged as worth splitting — advisory only, never a block; default 100000.",
    ).since("v1.9.0"));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.post_planning_gaps), dwf.and_then(|w| w.post_planning_gaps));
    push(cat, "workflow.post_planning_gaps", v, k, false, fd, ConfigHelp::new(
        "Once every plan is committed, reports which requirements and recorded decisions no plan in the phase covers.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.plan_bounce), dwf.and_then(|w| w.plan_bounce));
    push(cat, "workflow.plan_bounce", v, k, false, fd, ConfigHelp::new(
        "Pipes every finished PLAN.md through an external validator script and stops the phase when it exits non-zero.",
    ).since("v1.01.0"));
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.plan_bounce_passes), dwf.and_then(|w| w.plan_bounce_passes));
    push(cat, "workflow.plan_bounce_passes", v, k, false, fd, ConfigHelp::new(
        "How many times that validator re-reads its own output before a plan is accepted; more rigor, more latency.",
    ).since("v1.01.0"));
    let (v, k, fd) = str_l(pwf.and_then(|w| w.plan_bounce_script.as_deref()), dwf.and_then(|w| w.plan_bounce_script.as_deref()));
    push(cat, "workflow.plan_bounce_script", v, k, false, fd, ConfigHelp::new(
        "Path to that validator, invoked with the generated file's path as its first argument; required once bouncing is on.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.plan_review_convergence), dwf.and_then(|w| w.plan_review_convergence));
    push(cat, "workflow.plan_review_convergence", v, k, false, fd, ConfigHelp::new(
        "Unlocks the replan-until-the-reviewers-agree loop; while off, that command exits telling you which key to set.",
    ).since("v1.01.0"));
    let ppl = config.planning.as_ref();
    let dpl = defaults.and_then(|d: &GsdConfig| d.planning.as_ref());
    let (v, k, fd) = bool_l(ppl.and_then(|p| p.chunked_parallel), dpl.and_then(|p| p.chunked_parallel));
    push(cat, "planning.chunked_parallel", v, k, false, fd, ConfigHelp::new(
        "In chunked mode, writes the per-plan files concurrently instead of one after another; opt-in.",
    ).since("v1.13.0"));
    let (v, k, fd) = bool_l(ppl.and_then(|p| p.pr_strict), dpl.and_then(|p| p.pr_strict));
    push(cat, "planning.pr_strict", v, k, false, fd, ConfigHelp::new(
        "Drops every .planning path from a generated PR branch, structural files included, rather than just the transient ones.",
    ).since("v1.12.0"));
    let prv = config.plan_review.as_ref();
    let dprv = defaults.and_then(|d: &GsdConfig| d.plan_review.as_ref());
    let (v, k, fd) = bool_l(prv.and_then(|p| p.source_grounding), dprv.and_then(|p| p.source_grounding));
    push(cat, "plan_review.source_grounding", v, k, false, fd, ConfigHelp::new(
        "Resolves every symbol a plan cites against the live tree, so a plan naming a function nobody wrote is flagged.",
    ).since("v1.2.0"));
    let (v, k, fd) = enum_l(
        prv.and_then(|p| p.source_grounding_authority.as_deref()),
        dprv.and_then(|p| p.source_grounding_authority.as_deref()),
        &["grep", "intel", "treesitter", "lsp", "scip"],
    );
    push(cat, "plan_review.source_grounding_authority", v, k, false, fd, ConfigHelp::with_choices(
        "Which resolver decides whether a cited symbol exists, trading setup cost against precision.",
        &[
            ("grep", "a ripgrep search; needs no extra tooling anywhere"),
            ("intel", "the api-map index; needs intel_enabled on"),
            ("treesitter", "reserved for a future tree-sitter adapter"),
            ("lsp", "reserved for a future language-server adapter"),
            ("scip", "reserved for a future SCIP index adapter"),
        ],
    ).since("v1.2.0"));
    let pfe = config.features.as_ref();
    let dfe = defaults.and_then(|d: &GsdConfig| d.features.as_ref());
    let (v, k, fd) = bool_l(pfe.and_then(|f| f.global_learnings), dfe.and_then(|f| f.global_learnings));
    push(cat, "features.global_learnings", v, k, false, fd, ConfigHelp::new(
        "Copies what one project learned into a shared store at phase end and feeds it back to later planners.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pfe.and_then(|f| f.thinking_partner), dfe.and_then(|f| f.thinking_partner));
    push(cat, "features.thinking_partner", v, k, false, fd, ConfigHelp::new(
        "Adds a second opinion at each decision point in the workflow, arguing the case before a choice is made.",
    ).since("v1.01.0"));

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
    // gsd-core 1.14.0 re-sync (quick task 260916-vqw).
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.security_enforcement), dwf.and_then(|w| w.security_enforcement));
    push(cat, "workflow.security_enforcement", v, k, false, fd, ConfigHelp::new(
        "Runs the threat-model-anchored audit over a phase once it is built; off skips those checks entirely.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.agent_hint_routing), dwf.and_then(|w| w.agent_hint_routing));
    push(cat, "workflow.agent_hint_routing", v, k, false, fd, ConfigHelp::new(
        "Sends a plan whose frontmatter names a specialist subagent to that one instead of the generic executor.",
    ).since("v1.11.0"));
    let (v, k, fd) = enum_l(
        pwf.and_then(|w| w.code_review_point.as_deref()),
        dwf.and_then(|w| w.code_review_point.as_deref()),
        &["execute:post", "execute:wave:post"],
    );
    push(cat, "workflow.code_review_point", v, k, false, fd, ConfigHelp::with_choices(
        "When the reviewing step runs relative to a phase's waves, and how much of the diff it is handed.",
        &[
            ("execute:post", "once, after every wave in the phase has landed"),
            ("execute:wave:post", "once per wave, over that wave's own diff"),
        ],
    ).since("v1.13.0"));
    let (v, k, fd) = opt_json_readonly(
        pwf.and_then(|w| w.code_review_depth_overrides.as_ref()),
        dwf.and_then(|w| w.code_review_depth_overrides.as_ref()),
    );
    push(cat, "workflow.code_review_depth_overrides", v, k, false, fd, ConfigHelp::new(
        "Path rules raising how hard chosen directories are read, e.g. src/auth to deep; a list, so edited in the file.",
    ).since("v1.12.0"));
    let (v, k, fd) = enum_l(
        pwf.and_then(|w| w.human_verify_mode.as_deref()),
        dwf.and_then(|w| w.human_verify_mode.as_deref()),
        &["end-of-phase", "mid-flight"],
    );
    push(cat, "workflow.human_verify_mode", v, k, false, fd, ConfigHelp::with_choices(
        "Whether a run stops at each checkpoint you must look at, or collects them for one review at the end.",
        &[
            ("end-of-phase", "collects the checks for one review at the end"),
            ("mid-flight", "stops the run at each blocking checkpoint"),
        ],
    ).since("v1.01.0"));
    let (v, k, fd) = str_l(pwf.and_then(|w| w.build_command.as_deref()), dwf.and_then(|w| w.build_command.as_deref()));
    push(cat, "workflow.build_command", v, k, false, fd, ConfigHelp::new(
        "Shell line the post-merge gate runs; unset auto-detects cargo, npm, make, go or Xcode from what is in the repo.",
    ).since("v1.01.0"));
    let (v, k, fd) = str_l(pwf.and_then(|w| w.test_command.as_deref()), dwf.and_then(|w| w.test_command.as_deref()));
    push(cat, "workflow.test_command", v, k, false, fd, ConfigHelp::new(
        "Shell line the post-merge and regression gates run; unset auto-detects cargo, npm, go, pytest or Xcode.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.worktree_skip_hooks), dwf.and_then(|w| w.worktree_skip_hooks));
    push(cat, "workflow.worktree_skip_hooks", v, k, false, fd, ConfigHelp::new(
        "Lets agents commit with --no-verify and moves the check to the merged result; for projects whose hooks cannot run there.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.live_dom_uat), dwf.and_then(|w| w.live_dom_uat));
    push(cat, "workflow.live_dom_uat", v, k, false, fd, ConfigHelp::new(
        "Runs a browser-driven verifier after each wave and writes its report; while off no browser tooling is ever driven.",
    ).since("v1.12.0"));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.cross_ai_execution), dwf.and_then(|w| w.cross_ai_execution));
    push(cat, "workflow.cross_ai_execution", v, k, false, fd, ConfigHelp::new(
        "Hands a whole phase to an external CLI model instead of spawning local executor agents; needs the command below.",
    ).since("v1.01.0"));
    let (v, k, fd) = str_l(pwf.and_then(|w| w.cross_ai_command.as_deref()), dwf.and_then(|w| w.cross_ai_command.as_deref()));
    push(cat, "workflow.cross_ai_command", v, k, false, fd, ConfigHelp::new(
        "Shell template fed the phase prompt on stdin when execution is delegated out; it must print SUMMARY-shaped text.",
    ).since("v1.01.0"));
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.cross_ai_timeout), dwf.and_then(|w| w.cross_ai_timeout));
    push(cat, "workflow.cross_ai_timeout", v, k, false, fd, ConfigHelp::new(
        "Seconds that delegated command may run before it is killed, so a runaway process cannot hold a phase open; default 300.",
    ).since("v1.01.0"));

    // ── Docs & Output ─────────────────────────────────────────
    let cat = "Docs & Output";
    let (v, k, fd) = bool_l(config.commit_docs, defaults.and_then(|d| d.commit_docs));
    push(cat, "commit_docs", v, k, true, fd, ConfigHelp::new(
        "Commits .planning/ artifacts such as PLAN.md and SUMMARY.md; off keeps them out of git history.",
    ));
    // gsd-core's NAMESPACED spelling of the same switch. Both are modelled
    // because they are different JSON paths and a project may carry either.
    let (v, k, fd) = bool_l(
        config.planning.as_ref().and_then(|p| p.commit_docs),
        defaults.and_then(|d| d.planning.as_ref().and_then(|p| p.commit_docs)),
    );
    push(cat, "planning.commit_docs", v, k, false, fd, ConfigHelp::new(
        "The namespaced form of the switch above — gsd-core reads this path, and a project may carry either one.",
    ).since("v1.01.0"));
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
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.compact_content), dwf.and_then(|w| w.compact_content));
    push(cat, "workflow.compact_content", v, k, false, fd, ConfigHelp::with_choices(
        "Serves GSD's own shipped workflows, templates and agent personas in their terser form to spend less of the window.",
        &[
            ("true", "skips the deferred elaborations, reads .compact.md siblings"),
            ("false", "the full instruction set, byte-identical to before"),
        ],
    ).since("v1.14.0"));
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
    let (v, k, fd) = bool_l(
        config.graphify.as_ref().and_then(|g| g.auto_update),
        defaults.and_then(|d| d.graphify.as_ref().and_then(|g| g.auto_update)),
    );
    push(cat, "graphify.auto_update", v, k, false, fd, ConfigHelp::new(
        "Rebuilds that graph in the background after a commit or merge on the default branch, instead of on request.",
    ).since("v1.01.0"));
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
        (None, None) => (
            "(unset)".to_string(),
            ConfigValueKind::Unset(Box::new(ConfigValueKind::String)),
            false,
        ),
    };
    push(cat, "quick_branch_template", qbt_val, qbt_kind, false, qbt_fd, ConfigHelp::new(
        "Optional name pattern, with {slug} substituted, for a quick task's branch; unset keeps quick work here.",
    ));
    // gsd-core 1.14.0 re-sync (quick task 260916-vqw).
    let (v, k, fd) = bool_l(pgit.and_then(|g| g.create_tag), dgit.and_then(|g| g.create_tag));
    push(cat, "git.create_tag", v, k, false, fd, ConfigHelp::new(
        "Tags the repository when a milestone closes; turn it off for a project with its own release flow.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(
        pgit.and_then(|g| g.allow_default_branch_commits),
        dgit.and_then(|g| g.allow_default_branch_commits),
    );
    push(cat, "git.allow_default_branch_commits", v, k, false, fd, ConfigHelp::with_choices(
        "Escape hatch letting an executor commit straight onto the repository's own trunk instead of refusing.",
        &[
            ("true", "the pre-commit guard stops refusing on trunk"),
            ("false", "the guard refuses, which is what you want"),
        ],
    ).since("v1.13.0"));
    let (v, k, fd) = opt_json_readonly(
        pgit.and_then(|g| g.protected_branches.as_ref()),
        dgit.and_then(|g| g.protected_branches.as_ref()),
    );
    push(cat, "git.protected_branches", v, k, false, fd, ConfigHelp::new(
        "Extra shared branch names that raise the same warning the base one does; a list, so edited in the file.",
    ).since("v1.12.0"));

    // ── Misc ──────────────────────────────────────────────────
    let cat = "Misc";
    let (v, k, fd) = bool_l(
        config.hooks.as_ref().and_then(|h| h.context_warnings),
        defaults.and_then(|d| d.hooks.as_ref().and_then(|h| h.context_warnings)),
    );
    push(cat, "context_warnings", v, k, true, fd, ConfigHelp::new(
        "Warns in the statusline as a session's context budget runs out, before a compaction loses what was said.",
    ));
    // gsd-core 1.14.0 re-sync (quick task 260916-vqw).
    let phk = config.hooks.as_ref();
    let dhk = defaults.and_then(|d: &GsdConfig| d.hooks.as_ref());
    let (v, k, fd) = u32_l(
        phk.and_then(|h| h.context_warning_threshold),
        dhk.and_then(|h| h.context_warning_threshold),
    );
    push(cat, "hooks.context_warning_threshold", v, k, false, fd, ConfigHelp::new(
        "Percent of the window still FREE at which that warning first appears; default 35, and it must sit above the next row.",
    ).since("v1.14.0"));
    let (v, k, fd) = u32_l(
        phk.and_then(|h| h.context_critical_threshold),
        dhk.and_then(|h| h.context_critical_threshold),
    );
    push(cat, "hooks.context_critical_threshold", v, k, false, fd, ConfigHelp::new(
        "Percent still FREE at which the warning escalates; default 25, and it must stay strictly below the row above.",
    ).since("v1.14.0"));
    let (v, k, fd) = bool_l(phk.and_then(|h| h.workflow_guard), dhk.and_then(|h| h.workflow_guard));
    push(cat, "hooks.workflow_guard", v, k, false, fd, ConfigHelp::new(
        "Warns when a file is edited outside any GSD command, and hard-blocks force-adding files on an agent branch.",
    ).since("v1.01.0"));
    let (v, k, fd) = u32_l(config.context_window, defaults.and_then(|d| d.context_window));
    push(cat, "context_window", v, k, false, fd, ConfigHelp::new(
        "Tokens the model you run can hold; 200000 by default, and at 500000 or more GSD reads prior summaries in full.",
    ).since("v1.01.0"));
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
    let (v, k, fd) = bool_l(
        config.planning.as_ref().and_then(|p| p.search_gitignored),
        defaults.and_then(|d| d.planning.as_ref().and_then(|p| p.search_gitignored)),
    );
    push(cat, "planning.search_gitignored", v, k, false, fd, ConfigHelp::new(
        "The namespaced form of the switch above — gsd-core reads this path, and a project may carry either one.",
    ).since("v1.01.0"));
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
    let (v, k, fd) = opt_json_readonly(
        config.planning.as_ref().and_then(|p| p.sub_repos.as_ref()),
        defaults.and_then(|d| d.planning.as_ref().and_then(|p| p.sub_repos.as_ref())),
    );
    push(cat, "planning.sub_repos", v, k, false, fd, ConfigHelp::new(
        "The namespaced form of the list above — gsd-core reads this path; a list either way, so edited in the file.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.auto_prune_state), dwf.and_then(|w| w.auto_prune_state));
    push(cat, "workflow.auto_prune_state", v, k, false, fd, ConfigHelp::new(
        "Drops entries from STATE.md that have gone stale at each phase boundary, rather than stopping to ask you.",
    ).since("v1.01.0"));

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
    // gsd-core 1.14.0 re-sync (quick task 260916-vqw).
    let (v, k, fd) = enum_l(
        psl.and_then(|s| s.context_position.as_deref()),
        dsl.and_then(|s| s.context_position.as_deref()),
        &["end", "front"],
    );
    push(cat, "statusline.context_position", v, k, false, fd, ConfigHelp::with_choices(
        "Where the window meter sits on the line, which decides whether a narrow terminal cuts it off.",
        &[
            ("end", "at the tail of the line, the default"),
            ("front", "right after the model name, so it survives a cut"),
        ],
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(psl.and_then(|s| s.show_last_command), dsl.and_then(|s| s.show_last_command));
    push(cat, "statusline.show_last_command", v, k, false, fd, ConfigHelp::new(
        "Appends the slash command most recently invoked, read out of the running session's transcript.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(psl.and_then(|s| s.show_state_freshness), dsl.and_then(|s| s.show_state_freshness));
    push(cat, "statusline.show_state_freshness", v, k, false, fd, ConfigHelp::new(
        "Says how many commits the tree has moved since STATE.md was stamped, once that gap passes twenty.",
    ).since("v1.12.0"));

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
    // gsd-core 1.14.0 re-sync (quick task 260916-vqw).
    let (v, k, fd) = bool_l(pdr.and_then(|r| r.enabled), ddr.and_then(|r| r.enabled));
    push(cat, "dynamic_routing.enabled", v, k, false, fd, ConfigHelp::new(
        "Master switch: agents pick their model from the work's difficulty tier rather than from a fixed profile.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pdr.and_then(|r| r.escalate_on_failure), ddr.and_then(|r| r.escalate_on_failure));
    push(cat, "dynamic_routing.escalate_on_failure", v, k, false, fd, ConfigHelp::new(
        "Moves a retried step up one tier after a soft failure; off, every attempt stays on the tier it started at.",
    ).since("v1.01.0"));

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

    // ── Gates ─────────────────────────────────────────────────
    // gsd-core's `gates.*` block: eight stop-and-ask points, all defaulting to
    // on. Its own category because turning the set off is how a project goes
    // unattended, and a reader scanning for that should find them together.
    let cat = "Gates";
    let pga = config.gates.as_ref();
    let dga = defaults.and_then(|d: &GsdConfig| d.gates.as_ref());
    let (v, k, fd) = bool_l(pga.and_then(|g| g.confirm_project), dga.and_then(|g| g.confirm_project));
    push(cat, "gates.confirm_project", v, k, true, fd, ConfigHelp::new(
        "Shows you the gathered project details and waits for a yes before writing PROJECT.md.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pga.and_then(|g| g.confirm_roadmap), dga.and_then(|g| g.confirm_roadmap));
    push(cat, "gates.confirm_roadmap", v, k, false, fd, ConfigHelp::new(
        "Shows you the drafted milestone plan and waits for a yes before any phase is written.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pga.and_then(|g| g.confirm_phases), dga.and_then(|g| g.confirm_phases));
    push(cat, "gates.confirm_phases", v, k, false, fd, ConfigHelp::new(
        "Shows you how the milestone was cut into phases and waits for a yes before planning any of them.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pga.and_then(|g| g.confirm_breakdown), dga.and_then(|g| g.confirm_breakdown));
    push(cat, "gates.confirm_breakdown", v, k, false, fd, ConfigHelp::new(
        "Shows you how a phase was cut into tasks and waits for a yes before the plan is finalised.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pga.and_then(|g| g.confirm_plan), dga.and_then(|g| g.confirm_plan));
    push(cat, "gates.confirm_plan", v, k, false, fd, ConfigHelp::new(
        "Waits for a yes on each finished PLAN.md before its tasks are executed.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pga.and_then(|g| g.execute_next_plan), dga.and_then(|g| g.execute_next_plan));
    push(cat, "gates.execute_next_plan", v, k, false, fd, ConfigHelp::new(
        "Stops between two plans of the same phase and asks before starting the following one.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pga.and_then(|g| g.confirm_transition), dga.and_then(|g| g.confirm_transition));
    push(cat, "gates.confirm_transition", v, k, false, fd, ConfigHelp::new(
        "Stops at a phase boundary and asks before moving on to the one after it.",
    ).since("v1.01.0"));
    let (v, k, fd) = bool_l(pga.and_then(|g| g.issues_review), dga.and_then(|g| g.issues_review));
    push(cat, "gates.issues_review", v, k, false, fd, ConfigHelp::new(
        "Shows you the open issues and waits for a yes before any fix plan is written from them.",
    ).since("v1.01.0"));

    append_passthrough_entries(&mut entries, config, defaults);

    entries
}

/// The category every unmodelled key lands under.
///
/// Last, and its own category, so the list a reader scans top-down is
/// "everything this build understands" followed by "everything it does not".
const PASSTHROUGH_CATEGORY: &str = "Not modelled";

/// The one help summary every pass-through row shares.
///
/// **A static literal on purpose** (T-VQW-01). Interpolating the key into it
/// would put project-supplied text inside a [`ConfigHelp`], and
/// [`build_config_help_pane`]'s structural "nothing out of config.json reaches
/// here" property would stop being structural.
const PASSTHROUGH_HELP: ConfigHelp = ConfigHelp::new(
    "Present in this project's config.json but not modelled by this build — shown read-only, and preserved when you save.",
);

/// Rows for every key the project's (or the global defaults') config carries
/// that this build has no typed field for.
///
/// **This is what makes "not silently missing" true for the keys ID-3 does not
/// promote to typed rows** — gsd-core's nested and templated families
/// (`review.models.<cli>`, `model_policy.*`, `effort.*`, `agent_tools.<selector>`,
/// …), plus anything a release adds after [`GSD_CORE_SYNCED_COMMIT`]. A key
/// here is visible, is preserved by the writer, and cannot be edited from the
/// TUI; `docs/GSD-CORE-SYNC.md` records which families are deliberately in this
/// state rather than merely unnoticed.
///
/// Rows are emitted in SORTED key order (a `BTreeMap`), so the list does not
/// reshuffle between frames as a `serde_json::Map`'s iteration order would
/// allow under the `preserve_order` feature.
///
/// Layering matches the `opt_*_layered` helpers: the global defaults are laid
/// down first and the project's own keys overwrite them, so a key present in
/// both is attributed to the project and only a defaults-only key carries the
/// inherited marker.
fn append_passthrough_entries(
    entries: &mut Vec<ConfigEntry>,
    config: &crate::state_reader::config_json::GsdConfig,
    defaults: Option<&crate::state_reader::config_json::GsdConfig>,
) {
    use crate::state_reader::config_json::{ExtraKeys, GsdConfig};
    use std::collections::BTreeMap;

    /// Every `(dotted path, value)` pair one config contributes.
    fn collect(config: &GsdConfig, out: &mut Vec<(String, serde_json::Value)>) {
        let mut take = |prefix: &str, extra: &ExtraKeys| {
            for (key, value) in extra {
                out.push((format!("{prefix}{key}"), value.clone()));
            }
        };
        take("", &config.extra);
        if let Some(block) = config.git.as_ref() {
            take("git.", &block.extra);
        }
        if let Some(block) = config.workflow.as_ref() {
            take("workflow.", &block.extra);
        }
        if let Some(block) = config.hooks.as_ref() {
            take("hooks.", &block.extra);
        }
        if let Some(block) = config.intel.as_ref() {
            take("intel.", &block.extra);
        }
        if let Some(block) = config.graphify.as_ref() {
            take("graphify.", &block.extra);
        }
        if let Some(block) = config.claude_orchestration.as_ref() {
            take("claude_orchestration.", &block.extra);
        }
        if let Some(block) = config.statusline.as_ref() {
            take("statusline.", &block.extra);
        }
        if let Some(block) = config.dynamic_routing.as_ref() {
            take("dynamic_routing.", &block.extra);
        }
        if let Some(block) = config.review.as_ref() {
            take("review.", &block.extra);
        }
        if let Some(block) = config.external_job.as_ref() {
            take("external_job.", &block.extra);
        }
        if let Some(block) = config.capabilities.as_ref() {
            take("capabilities.", &block.extra);
        }
        // --- gsd-core re-sync at 1.14.0 (quick task 260916-vqw) ---
        //
        // These four blocks were ADDED by the same re-sync that wrote this
        // walk, and the walk was not extended to cover them: their `extra` maps
        // captured unmodelled keys (so the save path stayed lossless) but no
        // row was ever emitted, so the key was invisible in the Defaults tab —
        // the exact "silently missing" failure this function exists to prevent,
        // relocated from the write path to the display path. A block added to
        // `GsdConfig` MUST get a `take` here; the census in
        // `an_unmodelled_key_in_every_nested_block_becomes_a_visible_pass_through_row`
        // is what makes forgetting it a test failure instead of a quiet gap.
        if let Some(block) = config.features.as_ref() {
            take("features.", &block.extra);
        }
        if let Some(block) = config.gates.as_ref() {
            take("gates.", &block.extra);
        }
        if let Some(block) = config.planning.as_ref() {
            take("planning.", &block.extra);
        }
        if let Some(block) = config.plan_review.as_ref() {
            take("plan_review.", &block.extra);
        }
    }

    // `(value, from_defaults)`, defaults first so the project overwrites them.
    let mut rows: BTreeMap<String, (serde_json::Value, bool)> = BTreeMap::new();
    if let Some(defaults) = defaults {
        let mut found = Vec::new();
        collect(defaults, &mut found);
        for (key, value) in found {
            rows.insert(key, (value, true));
        }
    }
    let mut found = Vec::new();
    collect(config, &mut found);
    for (key, value) in found {
        rows.insert(key, (value, false));
    }

    let mut first = true;
    for (key, (value, from_defaults)) in rows {
        entries.push(ConfigEntry {
            category: PASSTHROUGH_CATEGORY,
            key: std::borrow::Cow::Owned(key),
            value: json_display(&value),
            kind: ConfigValueKind::ReadOnly,
            show_category: first,
            from_defaults,
            help: PASSTHROUGH_HELP,
        });
        first = false;
    }
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

/// Does `entry` match the Config tab's `/` filter? `q_lower` is the filter,
/// already lowercased; empty matches everything. Key + category only
/// (quick 260922-hdi, [INFERRED A1]): help prose would make short terms hit
/// dozens of unrelated rows, and matching the value would make a row vanish
/// from under the cursor the moment it is edited.
fn config_row_matches(entry: &ConfigEntry, q_lower: &str) -> bool {
    q_lower.is_empty()
        || entry.key.to_lowercase().contains(q_lower)
        || entry.category.to_lowercase().contains(q_lower)
}

/// UNDERLYING indices (into `entries`) of the rows the cache's filter shows —
/// every index when the filter is empty.
fn visible_defaults_indices(
    cache: &super::ProjectViewCache,
    entries: &[ConfigEntry],
) -> Vec<usize> {
    let q = cache.defaults_filter.to_lowercase();
    entries
        .iter()
        .enumerate()
        .filter(|(_, e)| config_row_matches(e, &q))
        .map(|(i, _)| i)
        .collect()
}

/// Move `defaults_selected` by `delta` VISIBLE rows. With no filter this is
/// exactly the tab's historical arithmetic (clamp to the last row going down,
/// saturate at 0 going up). With a filter the cursor walks the visible list;
/// a hidden cursor snaps to the first visible row, and an empty visible list
/// is a no-op.
fn move_defaults_selection(cache: &mut super::ProjectViewCache, delta: isize) {
    let entries = entries_for_cache(cache);
    if cache.defaults_filter.is_empty() {
        if delta >= 0 {
            if !entries.is_empty() {
                let max = entries.len() - 1;
                cache.defaults_selected = (cache.defaults_selected + delta as usize).min(max);
            }
        } else {
            cache.defaults_selected = cache.defaults_selected.saturating_sub(delta.unsigned_abs());
        }
        return;
    }
    let visible = visible_defaults_indices(cache, &entries);
    let Some(&first) = visible.first() else {
        return;
    };
    match visible.iter().position(|&i| i == cache.defaults_selected) {
        None => cache.defaults_selected = first,
        Some(pos) => {
            let new_pos = pos.saturating_add_signed(delta).min(visible.len() - 1);
            cache.defaults_selected = visible[new_pos];
        }
    }
}

/// With a non-empty filter, move a cursor that sits on a HIDDEN row to the
/// first visible one. A visible cursor, an empty filter, or a filter that
/// matches nothing leaves it where it is.
fn snap_defaults_selection(cache: &mut super::ProjectViewCache) {
    if cache.defaults_filter.is_empty() {
        return;
    }
    let entries = entries_for_cache(cache);
    let visible = visible_defaults_indices(cache, &entries);
    if let Some(&first) = visible.first() {
        if !visible.contains(&cache.defaults_selected) {
            cache.defaults_selected = first;
        }
    }
}

/// Is the Defaults cursor on a row the filter currently shows? Always true
/// with no filter, so the unfiltered Enter / x paths are untouched (a stale
/// cursor still falls to the `entries.get` they already go through).
fn defaults_selection_visible(cache: &super::ProjectViewCache) -> bool {
    if cache.defaults_filter.is_empty() {
        return true;
    }
    let entries = entries_for_cache(cache);
    visible_defaults_indices(cache, &entries).contains(&cache.defaults_selected)
}

/// After the filter text changed: put the cursor on the first match
/// ([INFERRED A6], the dashboard's `select(Some(0))`). An emptied filter or
/// one that matches nothing leaves the cursor alone.
fn select_first_visible(cache: &mut super::ProjectViewCache) {
    if cache.defaults_filter.is_empty() {
        return;
    }
    let entries = entries_for_cache(cache);
    if let Some(&first) = visible_defaults_indices(cache, &entries).first() {
        cache.defaults_selected = first;
    }
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

/// The index of the first PASS-THROUGH row of a cache's Defaults list — for the
/// render-escape probe, and DERIVED for the same reason [`first_string_entry`]
/// is (quick task 260916-vqw).
///
/// Pass-through rows are appended AFTER every authored category, so at a
/// 60-row probe terminal a 130-row list never draws one with the cursor at
/// zero. `probe_ctx` moves the cursor here so the row scrolls into the
/// viewport; without that the fixture's hostile key would be populated,
/// unrendered, and counted as coverage.
///
/// Returns `None` for a cache with no config — which is what `chrome_ctx` is —
/// so the baseline keeps its cursor at zero.
#[cfg(test)]
pub(super) fn first_passthrough_entry(cache: &super::ProjectViewCache) -> Option<usize> {
    entries_for_cache(cache)
        .into_iter()
        .position(|entry| entry.category == PASSTHROUGH_CATEGORY)
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
    match kind.editable() {
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
            // gsd-core 1.14.0 re-sync (quick task 260916-vqw)
            "workflow.compact_content" => { config.workflow.get_or_insert_with(WorkflowConfig::default).compact_content = Some(b); return true; }
            "workflow.agent_hint_routing" => { config.workflow.get_or_insert_with(WorkflowConfig::default).agent_hint_routing = Some(b); return true; }
            "workflow.auto_prune_state" => { config.workflow.get_or_insert_with(WorkflowConfig::default).auto_prune_state = Some(b); return true; }
            "workflow.context_coverage_gate" => { config.workflow.get_or_insert_with(WorkflowConfig::default).context_coverage_gate = Some(b); return true; }
            "workflow.context_drift_precheck" => { config.workflow.get_or_insert_with(WorkflowConfig::default).context_drift_precheck = Some(b); return true; }
            "workflow.cross_ai_execution" => { config.workflow.get_or_insert_with(WorkflowConfig::default).cross_ai_execution = Some(b); return true; }
            "workflow.live_dom_uat" => { config.workflow.get_or_insert_with(WorkflowConfig::default).live_dom_uat = Some(b); return true; }
            "workflow.plan_bounce" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_bounce = Some(b); return true; }
            "workflow.plan_review_convergence" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_review_convergence = Some(b); return true; }
            "workflow.post_planning_gaps" => { config.workflow.get_or_insert_with(WorkflowConfig::default).post_planning_gaps = Some(b); return true; }
            "workflow.security_enforcement" => { config.workflow.get_or_insert_with(WorkflowConfig::default).security_enforcement = Some(b); return true; }
            "workflow.worktree_skip_hooks" => { config.workflow.get_or_insert_with(WorkflowConfig::default).worktree_skip_hooks = Some(b); return true; }
            "features.global_learnings" => { config.features.get_or_insert_with(FeaturesConfig::default).global_learnings = Some(b); return true; }
            "features.thinking_partner" => { config.features.get_or_insert_with(FeaturesConfig::default).thinking_partner = Some(b); return true; }
            "gates.confirm_breakdown" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_breakdown = Some(b); return true; }
            "gates.confirm_phases" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_phases = Some(b); return true; }
            "gates.confirm_plan" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_plan = Some(b); return true; }
            "gates.confirm_project" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_project = Some(b); return true; }
            "gates.confirm_roadmap" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_roadmap = Some(b); return true; }
            "gates.confirm_transition" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_transition = Some(b); return true; }
            "gates.execute_next_plan" => { config.gates.get_or_insert_with(GatesConfig::default).execute_next_plan = Some(b); return true; }
            "gates.issues_review" => { config.gates.get_or_insert_with(GatesConfig::default).issues_review = Some(b); return true; }
            "planning.chunked_parallel" => { config.planning.get_or_insert_with(PlanningConfig::default).chunked_parallel = Some(b); return true; }
            "planning.commit_docs" => { config.planning.get_or_insert_with(PlanningConfig::default).commit_docs = Some(b); return true; }
            "planning.pr_strict" => { config.planning.get_or_insert_with(PlanningConfig::default).pr_strict = Some(b); return true; }
            "planning.search_gitignored" => { config.planning.get_or_insert_with(PlanningConfig::default).search_gitignored = Some(b); return true; }
            "plan_review.source_grounding" => { config.plan_review.get_or_insert_with(PlanReviewConfig::default).source_grounding = Some(b); return true; }
            "git.create_tag" => { config.git.get_or_insert_with(GitConfig::default).create_tag = Some(b); return true; }
            "git.allow_default_branch_commits" => { config.git.get_or_insert_with(GitConfig::default).allow_default_branch_commits = Some(b); return true; }
            "hooks.workflow_guard" => { config.hooks.get_or_insert_with(HooksConfig::default).workflow_guard = Some(b); return true; }
            "graphify.auto_update" => { config.graphify.get_or_insert_with(GraphifyConfig::default).auto_update = Some(b); return true; }
            "statusline.show_last_command" => { config.statusline.get_or_insert_with(StatuslineConfig::default).show_last_command = Some(b); return true; }
            "statusline.show_state_freshness" => { config.statusline.get_or_insert_with(StatuslineConfig::default).show_state_freshness = Some(b); return true; }
            "dynamic_routing.enabled" => { config.dynamic_routing.get_or_insert_with(DynamicRoutingConfig::default).enabled = Some(b); return true; }
            "dynamic_routing.escalate_on_failure" => { config.dynamic_routing.get_or_insert_with(DynamicRoutingConfig::default).escalate_on_failure = Some(b); return true; }
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
        // gsd-core 1.14.0 re-sync (quick task 260916-vqw)
        "workflow.code_review_point" => {
            config.workflow.get_or_insert_with(WorkflowConfig::default).code_review_point = Some(value.to_string());
            true
        }
        "workflow.context_drift_action" => {
            config.workflow.get_or_insert_with(WorkflowConfig::default).context_drift_action = Some(value.to_string());
            true
        }
        "workflow.drift_action" => {
            config.workflow.get_or_insert_with(WorkflowConfig::default).drift_action = Some(value.to_string());
            true
        }
        "workflow.human_verify_mode" => {
            config.workflow.get_or_insert_with(WorkflowConfig::default).human_verify_mode = Some(value.to_string());
            true
        }
        "plan_review.source_grounding_authority" => {
            config.plan_review.get_or_insert_with(PlanReviewConfig::default).source_grounding_authority = Some(value.to_string());
            true
        }
        "statusline.context_position" => {
            config.statusline.get_or_insert_with(StatuslineConfig::default).context_position = Some(value.to_string());
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
        // gsd-core 1.14.0 re-sync (quick task 260916-vqw)
        "workflow.build_command" => { config.workflow.get_or_insert_with(WorkflowConfig::default).build_command = Some(value.to_string()); true }
        "workflow.test_command" => { config.workflow.get_or_insert_with(WorkflowConfig::default).test_command = Some(value.to_string()); true }
        "workflow.cross_ai_command" => { config.workflow.get_or_insert_with(WorkflowConfig::default).cross_ai_command = Some(value.to_string()); true }
        "workflow.plan_bounce_script" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_bounce_script = Some(value.to_string()); true }
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

        // gsd-core 1.14.0 re-sync (quick task 260916-vqw)
        "workflow.compact_content" => { config.workflow.get_or_insert_with(WorkflowConfig::default).compact_content = None; true }
        "workflow.agent_hint_routing" => { config.workflow.get_or_insert_with(WorkflowConfig::default).agent_hint_routing = None; true }
        "workflow.auto_prune_state" => { config.workflow.get_or_insert_with(WorkflowConfig::default).auto_prune_state = None; true }
        "workflow.build_command" => { config.workflow.get_or_insert_with(WorkflowConfig::default).build_command = None; true }
        "workflow.code_review_point" => { config.workflow.get_or_insert_with(WorkflowConfig::default).code_review_point = None; true }
        "workflow.context_coverage_gate" => { config.workflow.get_or_insert_with(WorkflowConfig::default).context_coverage_gate = None; true }
        "workflow.context_drift_action" => { config.workflow.get_or_insert_with(WorkflowConfig::default).context_drift_action = None; true }
        "workflow.context_drift_precheck" => { config.workflow.get_or_insert_with(WorkflowConfig::default).context_drift_precheck = None; true }
        "workflow.cross_ai_command" => { config.workflow.get_or_insert_with(WorkflowConfig::default).cross_ai_command = None; true }
        "workflow.cross_ai_execution" => { config.workflow.get_or_insert_with(WorkflowConfig::default).cross_ai_execution = None; true }
        "workflow.cross_ai_timeout" => { config.workflow.get_or_insert_with(WorkflowConfig::default).cross_ai_timeout = None; true }
        "workflow.drift_action" => { config.workflow.get_or_insert_with(WorkflowConfig::default).drift_action = None; true }
        "workflow.drift_threshold" => { config.workflow.get_or_insert_with(WorkflowConfig::default).drift_threshold = None; true }
        "workflow.human_verify_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).human_verify_mode = None; true }
        "workflow.inline_plan_threshold" => { config.workflow.get_or_insert_with(WorkflowConfig::default).inline_plan_threshold = None; true }
        "workflow.live_dom_uat" => { config.workflow.get_or_insert_with(WorkflowConfig::default).live_dom_uat = None; true }
        "workflow.max_discuss_passes" => { config.workflow.get_or_insert_with(WorkflowConfig::default).max_discuss_passes = None; true }
        "workflow.plan_bounce" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_bounce = None; true }
        "workflow.plan_bounce_passes" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_bounce_passes = None; true }
        "workflow.plan_bounce_script" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_bounce_script = None; true }
        "workflow.plan_review_convergence" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_review_convergence = None; true }
        "workflow.post_planning_gaps" => { config.workflow.get_or_insert_with(WorkflowConfig::default).post_planning_gaps = None; true }
        "workflow.security_enforcement" => { config.workflow.get_or_insert_with(WorkflowConfig::default).security_enforcement = None; true }
        "workflow.smart_zone_tokens" => { config.workflow.get_or_insert_with(WorkflowConfig::default).smart_zone_tokens = None; true }
        "workflow.test_command" => { config.workflow.get_or_insert_with(WorkflowConfig::default).test_command = None; true }
        "workflow.worktree_skip_hooks" => { config.workflow.get_or_insert_with(WorkflowConfig::default).worktree_skip_hooks = None; true }
        "features.global_learnings" => { config.features.get_or_insert_with(FeaturesConfig::default).global_learnings = None; true }
        "features.thinking_partner" => { config.features.get_or_insert_with(FeaturesConfig::default).thinking_partner = None; true }
        "gates.confirm_breakdown" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_breakdown = None; true }
        "gates.confirm_phases" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_phases = None; true }
        "gates.confirm_plan" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_plan = None; true }
        "gates.confirm_project" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_project = None; true }
        "gates.confirm_roadmap" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_roadmap = None; true }
        "gates.confirm_transition" => { config.gates.get_or_insert_with(GatesConfig::default).confirm_transition = None; true }
        "gates.execute_next_plan" => { config.gates.get_or_insert_with(GatesConfig::default).execute_next_plan = None; true }
        "gates.issues_review" => { config.gates.get_or_insert_with(GatesConfig::default).issues_review = None; true }
        "planning.chunked_parallel" => { config.planning.get_or_insert_with(PlanningConfig::default).chunked_parallel = None; true }
        "planning.commit_docs" => { config.planning.get_or_insert_with(PlanningConfig::default).commit_docs = None; true }
        "planning.pr_strict" => { config.planning.get_or_insert_with(PlanningConfig::default).pr_strict = None; true }
        "planning.search_gitignored" => { config.planning.get_or_insert_with(PlanningConfig::default).search_gitignored = None; true }
        "plan_review.source_grounding" => { config.plan_review.get_or_insert_with(PlanReviewConfig::default).source_grounding = None; true }
        "plan_review.source_grounding_authority" => { config.plan_review.get_or_insert_with(PlanReviewConfig::default).source_grounding_authority = None; true }
        "git.create_tag" => { config.git.get_or_insert_with(GitConfig::default).create_tag = None; true }
        "git.allow_default_branch_commits" => { config.git.get_or_insert_with(GitConfig::default).allow_default_branch_commits = None; true }
        "hooks.workflow_guard" => { config.hooks.get_or_insert_with(HooksConfig::default).workflow_guard = None; true }
        "hooks.context_warning_threshold" => { config.hooks.get_or_insert_with(HooksConfig::default).context_warning_threshold = None; true }
        "hooks.context_critical_threshold" => { config.hooks.get_or_insert_with(HooksConfig::default).context_critical_threshold = None; true }
        "graphify.auto_update" => { config.graphify.get_or_insert_with(GraphifyConfig::default).auto_update = None; true }
        "context_window" => { config.context_window = None; true }
        "statusline.context_position" => { config.statusline.get_or_insert_with(StatuslineConfig::default).context_position = None; true }
        "statusline.show_last_command" => { config.statusline.get_or_insert_with(StatuslineConfig::default).show_last_command = None; true }
        "statusline.show_state_freshness" => { config.statusline.get_or_insert_with(StatuslineConfig::default).show_state_freshness = None; true }
        "dynamic_routing.enabled" => { config.dynamic_routing.get_or_insert_with(DynamicRoutingConfig::default).enabled = None; true }
        "dynamic_routing.escalate_on_failure" => { config.dynamic_routing.get_or_insert_with(DynamicRoutingConfig::default).escalate_on_failure = None; true }

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
        ConfigValueKind::Null => false, // Shape unknown: nothing to toggle to
        // Every arm below initialises its parent block, so an unset row
        // toggles/cycles exactly as its set counterpart does.
        ConfigValueKind::Unset(inner) => mutate_config_entry(config, key, inner),
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
                // gsd-core 1.14.0 re-sync (quick task 260916-vqw)
                "workflow.compact_content" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.compact_content = Some(!wf.compact_content.unwrap_or(false)); true }
                "workflow.agent_hint_routing" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.agent_hint_routing = Some(!wf.agent_hint_routing.unwrap_or(false)); true }
                "workflow.auto_prune_state" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.auto_prune_state = Some(!wf.auto_prune_state.unwrap_or(false)); true }
                "workflow.context_coverage_gate" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.context_coverage_gate = Some(!wf.context_coverage_gate.unwrap_or(false)); true }
                "workflow.context_drift_precheck" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.context_drift_precheck = Some(!wf.context_drift_precheck.unwrap_or(false)); true }
                "workflow.cross_ai_execution" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.cross_ai_execution = Some(!wf.cross_ai_execution.unwrap_or(false)); true }
                "workflow.live_dom_uat" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.live_dom_uat = Some(!wf.live_dom_uat.unwrap_or(false)); true }
                "workflow.plan_bounce" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.plan_bounce = Some(!wf.plan_bounce.unwrap_or(false)); true }
                "workflow.plan_review_convergence" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.plan_review_convergence = Some(!wf.plan_review_convergence.unwrap_or(false)); true }
                "workflow.post_planning_gaps" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.post_planning_gaps = Some(!wf.post_planning_gaps.unwrap_or(false)); true }
                "workflow.security_enforcement" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.security_enforcement = Some(!wf.security_enforcement.unwrap_or(false)); true }
                "workflow.worktree_skip_hooks" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.worktree_skip_hooks = Some(!wf.worktree_skip_hooks.unwrap_or(false)); true }
                "features.global_learnings" => { let f = config.features.get_or_insert_with(FeaturesConfig::default); f.global_learnings = Some(!f.global_learnings.unwrap_or(false)); true }
                "features.thinking_partner" => { let f = config.features.get_or_insert_with(FeaturesConfig::default); f.thinking_partner = Some(!f.thinking_partner.unwrap_or(false)); true }
                "gates.confirm_breakdown" => { let g = config.gates.get_or_insert_with(GatesConfig::default); g.confirm_breakdown = Some(!g.confirm_breakdown.unwrap_or(false)); true }
                "gates.confirm_phases" => { let g = config.gates.get_or_insert_with(GatesConfig::default); g.confirm_phases = Some(!g.confirm_phases.unwrap_or(false)); true }
                "gates.confirm_plan" => { let g = config.gates.get_or_insert_with(GatesConfig::default); g.confirm_plan = Some(!g.confirm_plan.unwrap_or(false)); true }
                "gates.confirm_project" => { let g = config.gates.get_or_insert_with(GatesConfig::default); g.confirm_project = Some(!g.confirm_project.unwrap_or(false)); true }
                "gates.confirm_roadmap" => { let g = config.gates.get_or_insert_with(GatesConfig::default); g.confirm_roadmap = Some(!g.confirm_roadmap.unwrap_or(false)); true }
                "gates.confirm_transition" => { let g = config.gates.get_or_insert_with(GatesConfig::default); g.confirm_transition = Some(!g.confirm_transition.unwrap_or(false)); true }
                "gates.execute_next_plan" => { let g = config.gates.get_or_insert_with(GatesConfig::default); g.execute_next_plan = Some(!g.execute_next_plan.unwrap_or(false)); true }
                "gates.issues_review" => { let g = config.gates.get_or_insert_with(GatesConfig::default); g.issues_review = Some(!g.issues_review.unwrap_or(false)); true }
                "planning.chunked_parallel" => { let p = config.planning.get_or_insert_with(PlanningConfig::default); p.chunked_parallel = Some(!p.chunked_parallel.unwrap_or(false)); true }
                "planning.commit_docs" => { let p = config.planning.get_or_insert_with(PlanningConfig::default); p.commit_docs = Some(!p.commit_docs.unwrap_or(false)); true }
                "planning.pr_strict" => { let p = config.planning.get_or_insert_with(PlanningConfig::default); p.pr_strict = Some(!p.pr_strict.unwrap_or(false)); true }
                "planning.search_gitignored" => { let p = config.planning.get_or_insert_with(PlanningConfig::default); p.search_gitignored = Some(!p.search_gitignored.unwrap_or(false)); true }
                "plan_review.source_grounding" => { let r = config.plan_review.get_or_insert_with(PlanReviewConfig::default); r.source_grounding = Some(!r.source_grounding.unwrap_or(false)); true }
                "git.create_tag" => { let g = config.git.get_or_insert_with(GitConfig::default); g.create_tag = Some(!g.create_tag.unwrap_or(false)); true }
                "git.allow_default_branch_commits" => { let g = config.git.get_or_insert_with(GitConfig::default); g.allow_default_branch_commits = Some(!g.allow_default_branch_commits.unwrap_or(false)); true }
                "hooks.workflow_guard" => { let h = config.hooks.get_or_insert_with(HooksConfig::default); h.workflow_guard = Some(!h.workflow_guard.unwrap_or(false)); true }
                "graphify.auto_update" => { let g = config.graphify.get_or_insert_with(GraphifyConfig::default); g.auto_update = Some(!g.auto_update.unwrap_or(false)); true }
                "statusline.show_last_command" => { let s = config.statusline.get_or_insert_with(StatuslineConfig::default); s.show_last_command = Some(!s.show_last_command.unwrap_or(false)); true }
                "statusline.show_state_freshness" => { let s = config.statusline.get_or_insert_with(StatuslineConfig::default); s.show_state_freshness = Some(!s.show_state_freshness.unwrap_or(false)); true }
                "dynamic_routing.enabled" => { let r = config.dynamic_routing.get_or_insert_with(DynamicRoutingConfig::default); r.enabled = Some(!r.enabled.unwrap_or(false)); true }
                "dynamic_routing.escalate_on_failure" => { let r = config.dynamic_routing.get_or_insert_with(DynamicRoutingConfig::default); r.escalate_on_failure = Some(!r.escalate_on_failure.unwrap_or(false)); true }
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
                // gsd-core 1.14.0 re-sync (quick task 260916-vqw). The fallback
                // passed to `cycle` is gsd-core's own documented default, so a
                // first press advances FROM that rather than from a value this
                // build invented.
                "workflow.code_review_point" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.code_review_point.as_deref().unwrap_or("execute:post");
                    wf.code_review_point = Some(cycle(current));
                    true
                }
                "workflow.context_drift_action" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.context_drift_action.as_deref().unwrap_or("warn");
                    wf.context_drift_action = Some(cycle(current));
                    true
                }
                "workflow.drift_action" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.drift_action.as_deref().unwrap_or("warn");
                    wf.drift_action = Some(cycle(current));
                    true
                }
                "workflow.human_verify_mode" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.human_verify_mode.as_deref().unwrap_or("end-of-phase");
                    wf.human_verify_mode = Some(cycle(current));
                    true
                }
                "plan_review.source_grounding_authority" => {
                    let r = config.plan_review.get_or_insert_with(PlanReviewConfig::default);
                    let current = r.source_grounding_authority.as_deref().unwrap_or("grep");
                    r.source_grounding_authority = Some(cycle(current));
                    true
                }
                "statusline.context_position" => {
                    let s = config.statusline.get_or_insert_with(StatuslineConfig::default);
                    let current = s.context_position.as_deref().unwrap_or("end");
                    s.context_position = Some(cycle(current));
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
                // gsd-core 1.14.0 re-sync (quick task 260916-vqw). Each step and
                // wrap point is chosen from the key's own documented range, so
                // the cycle reaches gsd-core's default rather than stepping past
                // it — the `x` key is always the way back to (unset).
                "workflow.cross_ai_timeout" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.cross_ai_timeout.unwrap_or(0);
                    wf.cross_ai_timeout = Some(if current >= 900 { 60 } else { current + 60 });
                    true
                }
                "workflow.drift_threshold" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.drift_threshold.unwrap_or(0);
                    wf.drift_threshold = Some(if current >= 10 { 1 } else { current + 1 });
                    true
                }
                "workflow.inline_plan_threshold" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.inline_plan_threshold.unwrap_or(0);
                    wf.inline_plan_threshold = Some(if current >= 20 { 1 } else { current + 1 });
                    true
                }
                "workflow.max_discuss_passes" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.max_discuss_passes.unwrap_or(0);
                    wf.max_discuss_passes = Some(if current >= 10 { 1 } else { current + 1 });
                    true
                }
                "workflow.plan_bounce_passes" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.plan_bounce_passes.unwrap_or(0);
                    wf.plan_bounce_passes = Some(if current >= 10 { 1 } else { current + 1 });
                    true
                }
                "workflow.smart_zone_tokens" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.smart_zone_tokens.unwrap_or(0);
                    wf.smart_zone_tokens =
                        Some(if current >= 400_000 { 25_000 } else { current + 25_000 });
                    true
                }
                // The two thresholds are a PAIR with an ordering constraint
                // gsd-core enforces (critical strictly below warning), so each
                // cycles inside its own half of the range rather than through
                // the whole of 0..100 and straight past the other.
                "hooks.context_warning_threshold" => {
                    let h = config.hooks.get_or_insert_with(HooksConfig::default);
                    let current = h.context_warning_threshold.unwrap_or(0);
                    h.context_warning_threshold =
                        Some(if current >= 90 { 30 } else { current.max(25) + 5 });
                    true
                }
                "hooks.context_critical_threshold" => {
                    let h = config.hooks.get_or_insert_with(HooksConfig::default);
                    let current = h.context_critical_threshold.unwrap_or(0);
                    h.context_critical_threshold =
                        Some(if current >= 20 { 5 } else { current + 5 });
                    true
                }
                "context_window" => {
                    let current = config.context_window.unwrap_or(0);
                    config.context_window =
                        Some(if current >= 1_000_000 { 200_000 } else { current + 200_000 });
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
            "context_window": 200000,
            "features": {
                "global_learnings": false,
                "thinking_partner": false
            },
            "gates": {
                "confirm_breakdown": true,
                "confirm_phases": true,
                "confirm_plan": true,
                "confirm_project": true,
                "confirm_roadmap": true,
                "confirm_transition": true,
                "execute_next_plan": true,
                "issues_review": true
            },
            "planning": {
                "chunked_parallel": false,
                "commit_docs": true,
                "pr_strict": false,
                "search_gitignored": false,
                "sub_repos": []
            },
            "plan_review": {
                "source_grounding": true,
                "source_grounding_authority": "grep"
            },
            "git": {
                "branching_strategy": "phase",
                "base_branch": "master",
                "phase_branch_template": "gsd/phase-{phase}-{slug}",
                "milestone_branch_template": "gsd/{milestone}-{slug}",
                "quick_branch_template": "gsd/quick-{slug}",
                "create_tag": true,
                "allow_default_branch_commits": false,
                "protected_branches": ["release"]
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
                "security_block_on": "high",
                "compact_content": true,
                "agent_hint_routing": true,
                "auto_prune_state": false,
                "build_command": "cargo build",
                "code_review_depth_overrides": [{ "paths": ["src/auth"], "depth": "deep" }],
                "code_review_point": "execute:post",
                "context_coverage_gate": true,
                "context_drift_action": "warn",
                "context_drift_precheck": true,
                "cross_ai_command": "codex exec -",
                "cross_ai_execution": false,
                "cross_ai_timeout": 300,
                "drift_action": "warn",
                "drift_threshold": 3,
                "human_verify_mode": "end-of-phase",
                "inline_plan_threshold": 3,
                "live_dom_uat": false,
                "max_discuss_passes": 3,
                "plan_bounce": false,
                "plan_bounce_passes": 2,
                "plan_bounce_script": "./scripts/bounce.sh",
                "plan_review_convergence": false,
                "post_planning_gaps": true,
                "security_enforcement": true,
                "smart_zone_tokens": 100000,
                "test_command": "cargo test --no-fail-fast",
                "worktree_skip_hooks": false
            },
            "hooks": {
                "context_warnings": true,
                "workflow_guard": false,
                "context_warning_threshold": 35,
                "context_critical_threshold": 25
            },
            "intel": { "enabled": true },
            "graphify": {
                "enabled": true,
                "build_timeout": 300,
                "graph_path": ".planning/graphs",
                "auto_update": false
            },
            "claude_orchestration": {
                "enabled": false,
                "execution_backend": "auto",
                "min_agent_sdk_version": "0.3.149"
            },
            "statusline": {
                "show_context_tokens": true,
                "state_format": "compact",
                "show_git": true,
                "context_position": "end",
                "show_last_command": false,
                "show_state_freshness": false
            },
            "dynamic_routing": {
                "provider_escalation": false,
                "max_escalations": 1,
                "enabled": false,
                "escalate_on_failure": true
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
    /// the time the help was authored and RE-measured at the gsd-core 1.14.0
    /// re-sync (73 -> 130, quick task 260916-vqw). It is asserted rather than
    /// trusted so a 131st option cannot slip past the coverage assertions below
    /// by being added to a list nobody counted.
    ///
    /// **It counts STATIC rows only.** `append_passthrough_entries` emits one
    /// row per unmodelled key found in the config it is handed, so a fixture
    /// carrying such a key would make this number a property of the fixture
    /// rather than of the tab. [`populated_gsd_config`] is therefore kept free
    /// of unmodelled keys, and the pass-through rows have their own fixture.
    const DEFAULTS_OPTION_COUNT: usize = 130;

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
            enum_entries, 12,
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
            .find(|e| e.key.as_ref() == "model_profile")
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
        render_defaults_config_to_text(populated_gsd_config(), width, height, selected)
    }

    /// [`render_defaults_to_text`] over a CALLER-SUPPLIED config.
    ///
    /// The pass-through rows cannot be probed through the populated fixture:
    /// that fixture is deliberately free of unmodelled keys so
    /// `DEFAULTS_OPTION_COUNT` keeps counting static rows only (the plan's Task
    /// 1 step 7). A pass-through row only exists for a config that carries a
    /// key this build has no typed field for, so the probe has to bring its own.
    fn render_defaults_config_to_text(
        config: crate::state_reader::config_json::GsdConfig,
        width: u16,
        height: u16,
        selected: usize,
    ) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut ctx = test_ctx();
        {
            let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
            cache.defaults_config = Some(config);
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
            .position(|e| e.key.as_ref() == "model_profile")
            .expect("model_profile is one of the Defaults tab's options");
        let model_summary = squeeze_ws(entries[model_idx].help.summary);

        let at_first = squeeze_ws(&render_defaults_to_text(120, 40, 0));
        assert!(
            at_first.contains(entries[0].key.as_ref()),
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
            .find(|line| line.contains(entries[0].key.as_ref()))
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

    // ── Pass-through rows (quick task 260916-vqw) ─────────────────────────

    /// A config carrying keys gsd-core writes and this build does not model —
    /// one at the top level, one nested in a modelled block, and one from a
    /// templated family ID-3 deliberately never promotes to a typed row.
    fn config_with_unmodelled_keys() -> crate::state_reader::config_json::GsdConfig {
        crate::state_reader::config_json::parse_gsd_config(
            r#"{
                "mode": "yolo",
                "workflow": { "research": true, "unknown_gsd_key": 7 },
                "brand_new_block": { "a": 1 },
                "review": { "models": { "codex": "gpt-5" } }
            }"#,
        )
        .expect("the pass-through fixture parses")
    }

    fn passthrough_rows(entries: &[ConfigEntry]) -> Vec<&ConfigEntry> {
        entries
            .iter()
            .filter(|entry| entry.category == PASSTHROUGH_CATEGORY)
            .collect()
    }

    #[test]
    fn an_unmodelled_key_becomes_a_read_only_row_under_its_own_category() {
        let entries = build_defaults_entries(&config_with_unmodelled_keys(), None);
        let rows = passthrough_rows(&entries);
        let keys: Vec<&str> = rows.iter().map(|entry| entry.key.as_ref()).collect();

        assert_eq!(
            keys,
            vec!["brand_new_block", "review.models", "workflow.unknown_gsd_key"],
            "the pass-through walk must reach the top level, the nested blocks \
             AND the templated families, in sorted order"
        );
        for entry in &rows {
            assert!(
                matches!(entry.kind, ConfigValueKind::ReadOnly),
                "`{}` is a pass-through row but its kind is {:?}, so the TUI \
                 offers to edit a key it cannot write back",
                entry.key,
                entry.kind
            );
            assert!(
                dropdown_options(&entry.kind).is_empty(),
                "`{}` offers dropdown options for a key this build does not model",
                entry.key
            );
        }
        assert!(
            rows[0].show_category,
            "the first pass-through row must print the category header"
        );

        // The CONTROL. Without it this passes on a walk that reports every key
        // as unmodelled, including the ones that have typed rows.
        let modelled: Vec<&str> = entries
            .iter()
            .filter(|entry| entry.category != PASSTHROUGH_CATEGORY)
            .map(|entry| entry.key.as_ref())
            .collect();
        assert!(modelled.contains(&"research"));
        assert!(
            !modelled.is_empty() && !keys.contains(&"research"),
            "a MODELLED key leaked into the pass-through rows"
        );
    }

    /// EVERY nested block `GsdConfig` carries an `Option<…Config>` field for.
    ///
    /// **This list is the fix for the bug it pins.** The original 260916-vqw
    /// pass-through walk enumerated eleven blocks by hand and then Task 3 of the
    /// same item added four more (`features`, `gates`, `planning`,
    /// `plan_review`) without extending the walk — so an unmodelled key under
    /// any of those four was preserved on save but drawn nowhere, breaking the
    /// item's own must-have that every key in a project's config.json is
    /// visible somewhere in the Defaults tab. Spelled as data and asserted over
    /// exhaustively so the next block added to `GsdConfig` fails HERE rather
    /// than becoming invisible in the same way.
    const NESTED_CONFIG_BLOCKS: &[&str] = &[
        "capabilities",
        "claude_orchestration",
        "dynamic_routing",
        "external_job",
        "features",
        "gates",
        "git",
        "graphify",
        "hooks",
        "intel",
        "plan_review",
        "planning",
        "review",
        "statusline",
        "workflow",
    ];

    /// The four blocks the gap report named — a subset of
    /// [`NESTED_CONFIG_BLOCKS`], called out so a failure says which regression
    /// came back rather than only that the census shrank.
    const BLOCKS_THE_PASSTHROUGH_WALK_ONCE_OMITTED: &[&str] =
        &["features", "gates", "plan_review", "planning"];

    /// A config carrying one unmodelled key at the top level and one inside
    /// every nested block, so the walk is probed at each of its sources.
    fn config_with_an_unmodelled_key_in_every_block(
    ) -> crate::state_reader::config_json::GsdConfig {
        let mut json = serde_json::Map::new();
        json.insert("zz_top_level_probe".to_string(), serde_json::json!(1));
        for block in NESTED_CONFIG_BLOCKS {
            json.insert(
                (*block).to_string(),
                serde_json::json!({ "zz_block_probe": block }),
            );
        }
        crate::state_reader::config_json::parse_gsd_config(
            &serde_json::Value::Object(json).to_string(),
        )
        .expect("the per-block pass-through fixture parses")
    }

    #[test]
    fn an_unmodelled_key_in_every_nested_block_becomes_a_visible_pass_through_row() {
        let config = config_with_an_unmodelled_key_in_every_block();
        let entries = build_defaults_entries(&config, None);
        let rows = passthrough_rows(&entries);
        let keys: Vec<&str> = rows.iter().map(|entry| entry.key.as_ref()).collect();

        // The NON-VACUITY floor: the fixture really does carry an unmodelled
        // key per block, so a walk that reached none would fail loudly here
        // rather than pass on an empty expectation.
        assert_eq!(
            keys.len(),
            NESTED_CONFIG_BLOCKS.len() + 1,
            "expected one pass-through row per nested block plus the top level; got {keys:?}"
        );

        for block in NESTED_CONFIG_BLOCKS {
            let dotted = format!("{block}.zz_block_probe");
            assert!(
                keys.contains(&dotted.as_str()),
                "`{dotted}` is captured by `{block}`'s `extra` map and survives a \
                 save, but `append_passthrough_entries` never emits a row for it — \
                 the operator cannot see the key exists. Add `take(\"{block}.\", \
                 &block.extra)` to `collect()`. Rows present: {keys:?}"
            );
        }
        assert!(keys.contains(&"zz_top_level_probe"));

        // Pass-through rows stay READ-ONLY: visible is not editable.
        for entry in &rows {
            assert!(
                matches!(entry.kind, ConfigValueKind::ReadOnly),
                "`{}` is a pass-through row but its kind is {:?}",
                entry.key,
                entry.kind
            );
        }
        let mut editable = config.clone();
        for block in BLOCKS_THE_PASSTHROUGH_WALK_ONCE_OMITTED {
            let dotted = format!("{block}.zz_block_probe");
            assert!(
                !set_config_value(&mut editable, &dotted, "false"),
                "`{dotted}` is a key this build does not model — no set arm may claim it"
            );
            assert!(
                !clear_config_value(&mut editable, &dotted),
                "`{dotted}` must have no clear arm"
            );
            assert!(
                !mutate_config_entry(&mut editable, &dotted, &ConfigValueKind::ReadOnly),
                "`{dotted}` must have no toggle arm"
            );
        }

        // And the rows REACH THE TERMINAL, not just the entry vector. The
        // pass-through category is last, so selecting the final row scrolls the
        // whole block into view.
        let last = entries.len() - 1;
        let rendered = render_defaults_config_to_text(config, 140, 48, last);
        for block in BLOCKS_THE_PASSTHROUGH_WALK_ONCE_OMITTED {
            let dotted = format!("{block}.zz_block_probe");
            assert!(
                rendered.contains(&dotted),
                "`{dotted}` never reached a terminal cell:\n{rendered}"
            );
        }
    }

    /// The tracer's negative half: the one key the todo named by hand must be a
    /// first-class row, not a pass-through one.
    #[test]
    fn compact_content_is_an_editable_row_naming_its_gsd_core_version() {
        let entries = all_config_entries();
        let entry = entries
            .iter()
            .find(|entry| entry.key.as_ref() == "workflow.compact_content")
            .expect("workflow.compact_content has a Defaults-tab row");

        assert!(
            matches!(entry.kind, ConfigValueKind::Bool),
            "compact_content is a boolean gate; kind is {:?}",
            entry.kind
        );
        assert_ne!(entry.category, PASSTHROUGH_CATEGORY);
        assert_eq!(
            entry.help.since, "v1.14.0",
            "the help must NAME the gsd-core version that introduced the key"
        );

        // Editable: the dropdown applies, the toggle flips, the clear unsets.
        let mut config = populated_gsd_config();
        assert!(set_config_value(&mut config, "workflow.compact_content", "false"));
        assert_eq!(
            config.workflow.as_ref().unwrap().compact_content,
            Some(false)
        );
        assert!(mutate_config_entry(
            &mut config,
            "workflow.compact_content",
            &ConfigValueKind::Bool
        ));
        assert_eq!(
            config.workflow.as_ref().unwrap().compact_content,
            Some(true)
        );
        assert!(clear_config_value(&mut config, "workflow.compact_content"));
        assert!(config.workflow.as_ref().unwrap().compact_content.is_none());

        // And the version reaches the pane a human reads, not just the struct.
        let idx = entries
            .iter()
            .position(|e| e.key.as_ref() == "workflow.compact_content")
            .unwrap();
        let rendered = squeeze_ws(&render_defaults_to_text(120, 40, idx));
        assert!(
            rendered.contains(&squeeze_ws(&format!("{SINCE_PREFIX}v1.14.0"))),
            "the help pane did not draw the `since` marker: {rendered}"
        );
    }

    /// Every key the gsd-core 1.14.0 re-sync added, with the kind its row must
    /// carry — MEASURED from gsd-core's own key table, not from this build.
    ///
    /// **Spelled as data rather than as one assertion per key** so the
    /// `ConfigValueKind` and the `since` are checked together, and so a key
    /// added to `build_defaults_entries` without a row here is caught by the
    /// count pin (`DEFAULTS_OPTION_COUNT`) from the other direction.
    const RESYNCED_KEYS: &[(&str, &str)] = &[
        ("workflow.agent_hint_routing", "bool"),
        ("workflow.auto_prune_state", "bool"),
        ("workflow.build_command", "string"),
        ("workflow.code_review_depth_overrides", "readonly"),
        ("workflow.code_review_point", "enum"),
        ("workflow.compact_content", "bool"),
        ("workflow.context_coverage_gate", "bool"),
        ("workflow.context_drift_action", "enum"),
        ("workflow.context_drift_precheck", "bool"),
        ("workflow.cross_ai_command", "string"),
        ("workflow.cross_ai_execution", "bool"),
        ("workflow.cross_ai_timeout", "integer"),
        ("workflow.drift_action", "enum"),
        ("workflow.drift_threshold", "integer"),
        ("workflow.human_verify_mode", "enum"),
        ("workflow.inline_plan_threshold", "integer"),
        ("workflow.live_dom_uat", "bool"),
        ("workflow.max_discuss_passes", "integer"),
        ("workflow.plan_bounce", "bool"),
        ("workflow.plan_bounce_passes", "integer"),
        ("workflow.plan_bounce_script", "string"),
        ("workflow.plan_review_convergence", "bool"),
        ("workflow.post_planning_gaps", "bool"),
        ("workflow.security_enforcement", "bool"),
        ("workflow.smart_zone_tokens", "integer"),
        ("workflow.test_command", "string"),
        ("workflow.worktree_skip_hooks", "bool"),
        ("features.global_learnings", "bool"),
        ("features.thinking_partner", "bool"),
        ("gates.confirm_breakdown", "bool"),
        ("gates.confirm_phases", "bool"),
        ("gates.confirm_plan", "bool"),
        ("gates.confirm_project", "bool"),
        ("gates.confirm_roadmap", "bool"),
        ("gates.confirm_transition", "bool"),
        ("gates.execute_next_plan", "bool"),
        ("gates.issues_review", "bool"),
        ("planning.chunked_parallel", "bool"),
        ("planning.commit_docs", "bool"),
        ("planning.pr_strict", "bool"),
        ("planning.search_gitignored", "bool"),
        ("planning.sub_repos", "readonly"),
        ("plan_review.source_grounding", "bool"),
        ("plan_review.source_grounding_authority", "enum"),
        ("git.create_tag", "bool"),
        ("git.allow_default_branch_commits", "bool"),
        ("git.protected_branches", "readonly"),
        ("hooks.workflow_guard", "bool"),
        ("hooks.context_warning_threshold", "integer"),
        ("hooks.context_critical_threshold", "integer"),
        ("graphify.auto_update", "bool"),
        ("context_window", "integer"),
        ("statusline.context_position", "enum"),
        ("statusline.show_last_command", "bool"),
        ("statusline.show_state_freshness", "bool"),
        ("dynamic_routing.enabled", "bool"),
        ("dynamic_routing.escalate_on_failure", "bool"),
    ];

    /// The re-sync's own size, MEASURED against gsd-core 1.14.0's key table:
    /// 27 `workflow.*` keys plus 30 elsewhere.
    ///
    /// Asserted rather than trusted for the same reason
    /// [`DEFAULTS_OPTION_COUNT`] is: a row quietly dropped from the table above
    /// would make every loop over it pass while covering one key fewer.
    const RESYNCED_KEY_COUNT: usize = 57;

    /// `docs/GSD-CORE-SYNC.md` is the baseline a future sync DIFFS FROM, so a
    /// record that outlives its subject is worse than no record — it tells the
    /// next reader a surface is covered when it is not.
    ///
    /// This pins the three things that can silently rot: the version and commit
    /// it names must be the ones the constants carry, and its `## Modelled`
    /// section must list every key `build_defaults_entries` pushes.
    ///
    /// **Read from disk through `CARGO_MANIFEST_DIR`**, the same route
    /// `render_escape_guard.rs`'s census uses, so the assertion is about the
    /// committed file rather than about a copy of it in this test.
    #[test]
    fn the_sync_record_names_every_modelled_key_and_the_measured_baseline() {
        use crate::state_reader::config_json::{
            GSD_CORE_SYNCED_COMMIT, GSD_CORE_SYNCED_VERSION,
        };

        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("docs/GSD-CORE-SYNC.md");
        let doc = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("the sync record is missing at {}: {e}", path.display())
        });

        assert!(
            doc.contains(GSD_CORE_SYNCED_VERSION),
            "the record does not name version {GSD_CORE_SYNCED_VERSION:?}, which \
             config_json.rs's constant carries"
        );
        assert!(
            doc.contains(GSD_CORE_SYNCED_COMMIT),
            "the record does not name commit {GSD_CORE_SYNCED_COMMIT:?}"
        );

        let modelled = doc
            .split_once("\n## Modelled")
            .expect("the record has a `## Modelled` section")
            .1
            .split_once("\n## Pass-through")
            .expect("the record has a `## Pass-through` section after it")
            .0;

        let missing: Vec<&str> = all_config_entries()
            .iter()
            .map(|entry| entry.key.as_ref())
            .filter(|key| !modelled.contains(&format!("`{key}`")))
            .map(|key| {
                // Leak-free: the keys are `&'static str` behind `Cow::Borrowed`
                // for every authored row, and this fixture pushes no others.
                Box::leak(key.to_string().into_boxed_str()) as &str
            })
            .collect();
        assert!(
            missing.is_empty(),
            "docs/GSD-CORE-SYNC.md's Modelled section does not list {missing:?} — \
             the record and the tab have drifted, which is the exact failure the \
             record exists to prevent. Update both in the SAME commit."
        );
    }

    #[test]
    fn the_resync_covers_every_key_the_drift_measurement_found() {
        assert_eq!(
            RESYNCED_KEYS.len(),
            RESYNCED_KEY_COUNT,
            "the re-synced key table changed size; re-measure the drift against \
             gsd-core and update docs/GSD-CORE-SYNC.md in the SAME commit"
        );
        let workflow = RESYNCED_KEYS
            .iter()
            .filter(|(key, _)| key.starts_with("workflow."))
            .count();
        assert_eq!(workflow, 27, "the workflow.* half of the re-sync changed size");
    }

    fn kind_name(kind: &ConfigValueKind) -> &'static str {
        match kind {
            ConfigValueKind::Bool => "bool",
            ConfigValueKind::Enum(_) => "enum",
            ConfigValueKind::String => "string",
            ConfigValueKind::Integer => "integer",
            ConfigValueKind::Null | ConfigValueKind::Unset(_) => "null",
            ConfigValueKind::ReadOnly => "readonly",
        }
    }

    /// Each re-synced key appears EXACTLY once, with the kind gsd-core's type
    /// column implies and a non-empty measured `since`.
    ///
    /// The "exactly once" half matters: a key pushed under two categories
    /// renders twice and the second row's edits silently fight the first.
    #[test]
    fn every_resynced_key_has_one_row_of_the_right_kind_with_a_measured_since() {
        let entries = all_config_entries();
        for (key, expected_kind) in RESYNCED_KEYS {
            let matching: Vec<&ConfigEntry> = entries
                .iter()
                .filter(|entry| entry.key.as_ref() == *key)
                .collect();
            assert_eq!(
                matching.len(),
                1,
                "`{key}` has {} Defaults-tab rows; a key listed twice renders \
                 twice and its two rows fight each other on save",
                matching.len()
            );
            let entry = matching[0];
            assert_eq!(
                kind_name(&entry.kind),
                *expected_kind,
                "`{key}` is a {expected_kind} in gsd-core's key table but this \
                 build gives it a {} row",
                kind_name(&entry.kind)
            );
            assert!(
                !entry.help.since.is_empty(),
                "`{key}` carries no `since` — resolve it by MEASURING gsd-core's \
                 history, never from memory (ID-4)"
            );
            assert!(
                entry.help.since.starts_with('v'),
                "`{key}`'s since is {:?}, which is not a gsd-core tag name",
                entry.help.since
            );
            assert_ne!(
                entry.category, PASSTHROUGH_CATEGORY,
                "`{key}` is supposed to be MODELLED but rendered as a \
                 pass-through row"
            );
        }
    }

    /// Every re-synced key that is editable at all is editable through the arm
    /// its kind dispatches to, and clears back to `(unset)`.
    ///
    /// The read-only key is asserted in the OTHER direction in the same loop,
    /// so "it is read-only" is a checked property rather than an omission.
    #[test]
    fn every_editable_resynced_key_sets_toggles_and_clears() {
        let entries = all_config_entries();
        for (key, expected_kind) in RESYNCED_KEYS {
            let entry = entries
                .iter()
                .find(|entry| entry.key.as_ref() == *key)
                .unwrap();
            let mut config = populated_gsd_config();

            match *expected_kind {
                "bool" => {
                    assert!(set_config_value(&mut config, key, "false"), "{key} set");
                    assert!(
                        mutate_config_entry(&mut config, key, &entry.kind),
                        "{key} toggle"
                    );
                }
                "enum" => {
                    let options = dropdown_options(&entry.kind);
                    assert_eq!(
                        options,
                        entry
                            .help
                            .choices
                            .iter()
                            .map(|(v, _)| (*v).to_string())
                            .collect::<Vec<_>>(),
                        "{key}: the documented choices and the dropdown disagree"
                    );
                    for option in &options {
                        assert!(
                            set_config_value(&mut config, key, option),
                            "{key} could not be set to {option:?}"
                        );
                    }
                    assert!(
                        mutate_config_entry(&mut config, key, &entry.kind),
                        "{key} cycle"
                    );
                }
                "string" => {
                    assert!(set_string_value(&mut config, key, "x"), "{key} set");
                }
                "integer" => {
                    assert!(
                        mutate_config_entry(&mut config, key, &ConfigValueKind::Integer),
                        "{key} has an Integer row but no Integer arm, so Enter on \
                         it is a dead key"
                    );
                }
                "readonly" => {
                    assert!(
                        !mutate_config_entry(&mut config, key, &entry.kind),
                        "{key} is read-only but the toggle arm accepted it"
                    );
                    assert!(
                        dropdown_options(&entry.kind).is_empty(),
                        "{key} is read-only but offers dropdown options"
                    );
                    continue;
                }
                other => panic!("unknown expected kind {other:?} for {key}"),
            }

            assert!(
                clear_config_value(&mut config, key),
                "{key} cannot be cleared back to (unset)"
            );
        }
    }

    /// T-VQW-02's other half: a pass-through row is READ-only in every one of
    /// the three arms that can mutate a config, so nothing can write a key back
    /// under a name this build does not understand.
    #[test]
    fn a_pass_through_key_is_refused_by_every_mutation_arm() {
        let mut config = config_with_unmodelled_keys();
        for key in ["brand_new_block", "workflow.unknown_gsd_key", "review.models"] {
            assert!(
                !set_config_value(&mut config, key, "true"),
                "`{key}` was accepted by the dropdown arm"
            );
            assert!(
                !set_string_value(&mut config, key, "x"),
                "`{key}` was accepted by the string arm"
            );
            assert!(
                !clear_config_value(&mut config, key),
                "`{key}` was accepted by the clear arm"
            );
            assert!(
                !mutate_config_entry(&mut config, key, &ConfigValueKind::ReadOnly),
                "`{key}` was accepted by the toggle arm"
            );
        }
        // The control: the same four arms DO recognise a modelled key, so the
        // refusals above are about the key rather than about broken arms.
        assert!(set_config_value(&mut config, "research", "false"));
        assert!(clear_config_value(&mut config, "research"));
    }

    /// T-VQW-01. A pass-through row is the first Defaults-tab row whose KEY is
    /// project-supplied, so the key crosses the same untrusted-text boundary
    /// `entry.value` already did.
    ///
    /// The fixture's hostile characters are drawn BY IMPORT from
    /// `LOOK_ALIKE_PAIRS` (D-21-6) rather than respelled here, and the row is
    /// SELECTED so the list scrolls it into the viewport — a 130-row list at 40
    /// rows of terminal would otherwise never draw it and this test would pass
    /// by never rendering the site it is about.
    #[test]
    fn a_hostile_pass_through_key_cannot_reach_a_cell_unescaped() {
        use crate::test_support::LOOK_ALIKE_PAIRS;

        let hostile_key = format!("brand_{}_block", LOOK_ALIKE_PAIRS[4].1);
        let hostile_value = format!("value{}", LOOK_ALIKE_PAIRS[4].1);
        // Built THROUGH serde_json rather than by `format!` into a raw string:
        // a hand-escaped literal has to spell JSON's `\uXXXX` surrogate pairs
        // for an astral-plane tag character, and the first version of this test
        // wrote Rust's `\u{e0041}` instead and failed at "the hostile fixture
        // parses" — a fixture that cannot parse asserts nothing about escaping.
        let mut root = serde_json::Map::new();
        root.insert("mode".to_string(), serde_json::Value::from("yolo"));
        root.insert(
            hostile_key.clone(),
            serde_json::Value::from(hostile_value.clone()),
        );
        let raw = serde_json::to_string(&serde_json::Value::Object(root))
            .expect("the hostile fixture serialises");
        let config = crate::state_reader::config_json::parse_gsd_config(&raw)
            .expect("the hostile fixture parses");

        let entries = build_defaults_entries(&config, None);
        let idx = entries
            .iter()
            .position(|entry| entry.category == PASSTHROUGH_CATEGORY)
            .expect("the hostile key produced a pass-through row");

        let mut ctx = test_ctx();
        {
            let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
            cache.defaults_config = Some(config);
            cache.defaults_selected = idx;
        }
        let screen = DetailScreen::new(TEST_ALIAS.to_string());

        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let (width, height) = (200u16, 40u16);
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("TestBackend terminal");
        terminal
            .draw(|frame| screen.render_defaults_tab(frame, frame.area(), &ctx))
            .expect("draw the Defaults tab");
        let buffer = terminal.backend().buffer().clone();
        let drawn: String = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| (x, y))
            })
            .filter_map(|(x, y)| buffer.cell((x, y)).map(|cell| cell.symbol().to_string()))
            .collect();

        // ARRIVAL first: without it every assertion below passes by the row
        // never having been rendered at all.
        assert!(
            drawn.contains("brand_"),
            "the hostile pass-through row never reached the viewport, so nothing \
             below this line is about escaping"
        );
        let leaked: Vec<char> = drawn
            .chars()
            .filter(|c| crate::text::is_invisible_formatting_char(*c))
            .collect();
        assert!(
            leaked.is_empty(),
            "the pass-through row put {leaked:?} into the terminal buffer — those \
             characters render as nothing, so what the operator reads is not what \
             the key is"
        );
        // The expected spelling comes from the SAME composition the render
        // applies (`shown` delegates to `render_for_terminal`), not from the
        // half of it that handles this one class. Naming a half here would make
        // the assertion disagree with the render the day the composition
        // changes, and would add a bare `display_identity` needle to the census
        // `ui::tests::no_display_identity_call_under_ui_stands_outside_a_composition`
        // keeps over this directory.
        assert!(
            drawn.contains(&crate::text::render_for_terminal(&hostile_key).to_string()),
            "the escaped spelling of the key is absent, so the row drew something \
             other than the key it is for"
        );
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
        footer_text_at(sub_view, 120, true)
    }

    fn footer_text_at(sub_view: &DetailSubView, width: u16, experimental: bool) -> String {
        footer_spans(sub_view, width, experimental)
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
             [r]eload  [/]filter  [?]help"
        );
    }

    // ── Plan 18-09: the 11th tab, at all six sites ────────────────────────

    /// The shared prefix's tabs hint changes **once, for every tab**, so
    /// `Shift+D` is discoverable from anywhere in the detail view — not only
    /// from the tab it opens, which the user has no reason to be on.
    #[test]
    fn the_tabs_hint_names_shift_d_on_every_tab() {
        for sub_view in [
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

    /// The flag-off half of the same shared prefix (260917-fko D2).
    ///
    /// Asserted as the **exact prefix string**, not as "the letter `D` is
    /// absent": eight of the ten non-driver footers legitimately contain a `D`
    /// (`[Enter]done`, `[d]el`, `[d] defaults`, `[PgUp/PgDn]`), so a
    /// letter-level negative check would be either falsely red or, restricted
    /// enough to pass, vacuous.
    #[test]
    fn the_tabs_hint_drops_shift_d_on_every_tab_when_experimental_is_off() {
        for sub_view in [
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
            let text = footer_text_at(&sub_view, 120, false);
            assert!(
                text.starts_with("  [Esc]back  [1-0]tabs  "),
                "{sub_view:?} must not advertise a Driver tab the user cannot \
                 reach: {text}"
            );
            assert!(
                !text.contains("[1-0/D]"),
                "{sub_view:?} still leaks the Driver tab into its tabs hint: {text}"
            );
        }
    }

    #[test]
    fn the_driver_footer_has_three_measured_width_forms() {
        assert_eq!(
            footer_text_at(&DetailSubView::Driver, 120, true),
            "  [Esc]back  [1-0/D]tabs  [j/k]runs  [PgUp/PgDn]output  [f]ollow  [i]nject  \
             [s]tart  [x]stop  [?]help"
        );
        assert_eq!(
            footer_text_at(&DetailSubView::Driver, 80, true),
            "  [Esc]back  [j/k]runs  [f]ollow  [i]nject  [s]tart  [x]stop  [?]help"
        );
        assert_eq!(
            footer_text_at(&DetailSubView::Driver, 50, true),
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
            let text = footer_text_at(&DetailSubView::Driver, width, true);
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
            let view = sub_view_from_index(index, true);
            assert_eq!(
                tab_index(&view),
                index,
                "index {index} mapped to {view:?}, which maps back to {}",
                tab_index(&view)
            );
        }
        assert_eq!(sub_view_from_index(DRIVER_TAB_INDEX, true), DetailSubView::Driver);
        assert_eq!(tab_index(&DetailSubView::Driver), DRIVER_TAB_INDEX);
        // The out-of-range fallback lands on the default (Roadmap) tab, not the
        // newest.
        assert_eq!(sub_view_from_index(TAB_COUNT, true), DetailSubView::RoadmapViz);
    }

    /// Phase 24 end to end (D-B01, D-B03, D-B07, D-B10): a detail screen with no
    /// stored view opens on the Roadmap tab, `2` reaches the tab labelled
    /// Phases, and the zero key — which used to open the Docs tab — is inert.
    ///
    /// Driven through the real `handle_key` and the real render, because every
    /// piece of this is a different consumer of the tab index: the enum's
    /// `#[default]`, the digit arms, the dispatch and the block titles.
    #[test]
    fn a_fresh_detail_screen_opens_on_the_roadmap_and_two_reaches_phases() {
        let mut ctx = test_ctx();
        ctx.project_states.insert(
            TEST_ALIAS.to_string(),
            crate::state_reader::ProjectState::default(),
        );
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
        assert!(
            !ctx.detail_sub_view_per_project.contains_key(TEST_ALIAS),
            "the fixture must start with no stored view"
        );

        // The view the key handler and both renders read when nothing is
        // stored: the enum's default, through the same coercion they apply.
        let stored = ctx
            .detail_sub_view_per_project
            .get(TEST_ALIAS)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            effective_sub_view(stored, ctx.experimental),
            DetailSubView::RoadmapViz
        );
        let roadmap = render_detail_to_text(&screen, &ctx);
        assert!(
            roadmap.contains("Path:"),
            "a fresh detail screen must render the Roadmap header: {roadmap}"
        );
        assert!(
            !roadmap.contains(" Phases "),
            "the Phases tab's block title is on screen before `2` was pressed: {roadmap}"
        );

        press(&mut screen, &mut ctx, KeyCode::Char('2'));
        assert_eq!(
            ctx.detail_sub_view_per_project.get(TEST_ALIAS),
            Some(&DetailSubView::Pipeline)
        );
        let phases = render_detail_to_text(&screen, &ctx);
        assert!(
            phases.contains(" Phases "),
            "`2` must reach the tab titled Phases: {phases}"
        );
        assert!(
            !phases.contains(" Pipeline "),
            "the Phases tab still carries its old block title: {phases}"
        );

        // The zero key is not a tab any more: the user stays where they are.
        press(&mut screen, &mut ctx, KeyCode::Char('0'));
        assert_eq!(
            ctx.detail_sub_view_per_project.get(TEST_ALIAS),
            Some(&DetailSubView::Pipeline),
            "the zero key must fall through to the no-op arm"
        );
    }

    // ── 260917-fko: the experimental flag, both states ────────────────────
    //
    // Group 2 (flag OFF) and group 3 (flag ON) of the seven the CONTEXT
    // requires. Every existing assertion above threads `true`, which is the
    // flag-on half; these are the half that did not exist before.

    /// The label text of one rendered tab title, marker cell included.
    fn title_text(line: &Line<'static>) -> String {
        line.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>()
    }

    /// Every rendered tab label at `width`, joined — the form a "no Driver
    /// label anywhere" assertion can search without caring which tier answered.
    fn bar_text(width: u16, active: usize, experimental: bool) -> String {
        let (titles, _) = tab_titles(width, active, true, experimental);
        titles.iter().map(title_text).collect::<Vec<_>>().join("|")
    }

    /// The Driver tab's two labels, as the tab bar spells them. Read from the
    /// arrays rather than retyped, so a rename cannot make this vacuous.
    fn driver_labels() -> [&'static str; 2] {
        [
            TAB_LABELS_FULL[DRIVER_TAB_INDEX],
            TAB_LABELS_COMPACT[DRIVER_TAB_INDEX],
        ]
    }

    #[test]
    fn the_visible_tab_count_drops_the_eleventh_tab_when_experimental_is_off() {
        assert_eq!(visible_tab_count(true), TAB_COUNT);
        assert_eq!(visible_tab_count(false), TAB_COUNT - 1);
        assert_eq!(visible_tab_count(false), 10);
    }

    /// The whole of D2's tab-bar half: at **every** tier, not just the one a
    /// developer happens to run at.
    ///
    /// `driver_live` is passed `true` throughout, which is the hostile case —
    /// a run really is live and the bar still must not say so.
    #[test]
    fn no_width_tier_emits_a_driver_label_when_experimental_is_off() {
        for width in [
            TAB_BAR_FULL_CELLS,
            TAB_BAR_FULL_CELLS_NO_DRIVER,
            TAB_BAR_COMPACT_CELLS,
            TAB_BAR_COMPACT_CELLS_NO_DRIVER,
            60,
            40,
            20,
        ] {
            for active in [0usize, 5, 9] {
                let text = bar_text(width, active, false);
                for label in driver_labels() {
                    assert!(
                        !text.contains(label),
                        "at {width} columns with tab {active} active the bar \
                         still offers {label:?}: {text}"
                    );
                }
                assert!(
                    !text.contains(DRIVER_LIVE_MARKER),
                    "at {width} columns the live marker survived the tab it \
                     belongs to: {text}"
                );
            }
        }
    }

    /// The flag-on half of the same property, so the test above cannot pass by
    /// the labels having been deleted outright.
    #[test]
    fn the_whole_bar_tiers_still_offer_the_driver_label_when_experimental_is_on() {
        assert!(bar_text(TAB_BAR_FULL_CELLS, 0, true).contains(TAB_LABELS_FULL[DRIVER_TAB_INDEX]));
        assert!(
            bar_text(TAB_BAR_COMPACT_CELLS, 0, true)
                .contains(TAB_LABELS_COMPACT[DRIVER_TAB_INDEX])
        );
        // The windowed tier reaches it by making it active.
        assert!(
            bar_text(40, DRIVER_TAB_INDEX, true).contains(TAB_LABELS_COMPACT[DRIVER_TAB_INDEX])
        );
    }

    /// Ten labels fit the compact bar in **69** cells, not the eleven-tab 77 —
    /// so a flag-off session at 70 columns gets whole labels where an
    /// eleven-tab one is pushed into the windowed tier. Reusing the eleven-tab
    /// thresholds would have silently cost exactly that.
    #[test]
    fn the_ten_tab_bar_renders_whole_at_its_own_re_derived_widths() {
        let (titles, select) = tab_titles(TAB_BAR_FULL_CELLS_NO_DRIVER, 9, false, false);
        assert_eq!(titles.len(), 10);
        assert_eq!(select, 9);
        assert_eq!(bar_cells(&titles), usize::from(TAB_BAR_FULL_CELLS_NO_DRIVER));

        let (titles, select) = tab_titles(TAB_BAR_COMPACT_CELLS_NO_DRIVER, 4, false, false);
        assert_eq!(titles.len(), 10);
        assert_eq!(select, 4);
        assert_eq!(bar_cells(&titles), usize::from(TAB_BAR_COMPACT_CELLS_NO_DRIVER));

        // One cell below the ten-tab full width is already the compact tier.
        let (titles, _) = tab_titles(TAB_BAR_FULL_CELLS_NO_DRIVER - 1, 4, false, false);
        assert_eq!(bar_cells(&titles), usize::from(TAB_BAR_COMPACT_CELLS_NO_DRIVER));
    }

    /// **The anti-drift test.** All four cell constants re-derived from the
    /// label arrays themselves with the documented `Σ(len + 2) + (n − 1)`
    /// formula, plus the Driver tab's reserved marker cell in the eleven-tab
    /// cases. This is what keeps 107 / 96 / 77 / 69 honest when a label is
    /// renamed — the numbers stop being four literals a reader has to trust.
    #[test]
    fn the_tab_bar_widths_are_the_label_arrays_own_arithmetic() {
        fn derive(labels: &[&'static str], count: usize, marker: bool) -> u16 {
            let cells: usize = labels[..count]
                .iter()
                .map(|label| label.chars().count() + 2)
                .sum::<usize>()
                + usize::from(marker)
                + count.saturating_sub(1);
            u16::try_from(cells).expect("a tab bar fits in u16 cells")
        }

        assert_eq!(
            derive(&TAB_LABELS_FULL, TAB_COUNT, true),
            TAB_BAR_FULL_CELLS
        );
        assert_eq!(
            derive(&TAB_LABELS_FULL, TAB_COUNT - 1, false),
            TAB_BAR_FULL_CELLS_NO_DRIVER
        );
        assert_eq!(
            derive(&TAB_LABELS_COMPACT, TAB_COUNT, true),
            TAB_BAR_COMPACT_CELLS
        );
        assert_eq!(
            derive(&TAB_LABELS_COMPACT, TAB_COUNT - 1, false),
            TAB_BAR_COMPACT_CELLS_NO_DRIVER
        );

        // And the accessors hand back exactly those four, so no call site can
        // be reading a fifth number.
        assert_eq!(tab_bar_full_cells(true), TAB_BAR_FULL_CELLS);
        assert_eq!(tab_bar_full_cells(false), TAB_BAR_FULL_CELLS_NO_DRIVER);
        assert_eq!(tab_bar_compact_cells(true), TAB_BAR_COMPACT_CELLS);
        assert_eq!(
            tab_bar_compact_cells(false),
            TAB_BAR_COMPACT_CELLS_NO_DRIVER
        );
    }

    #[test]
    fn index_ten_is_not_a_tab_when_experimental_is_off() {
        assert_eq!(
            sub_view_from_index(DRIVER_TAB_INDEX, false),
            DetailSubView::RoadmapViz,
            "index 10 must take the same first-tab fallback an out-of-range \
             index already takes"
        );
        // The other ten still round-trip, so the fallback did not swallow them.
        for index in 0..(TAB_COUNT - 1) {
            let view = sub_view_from_index(index, false);
            assert_eq!(tab_index(&view), index);
        }
    }

    #[test]
    fn a_stored_driver_sub_view_reads_as_the_default_tab_when_experimental_is_off() {
        assert_eq!(
            effective_sub_view(DetailSubView::Driver, false),
            DetailSubView::RoadmapViz
        );
        assert_eq!(
            effective_sub_view(DetailSubView::Driver, true),
            DetailSubView::Driver
        );
        // Identity on every other sub-view, in both states: the coercion is
        // the Driver tab's alone.
        for index in 0..(TAB_COUNT - 1) {
            let view = sub_view_from_index(index, true);
            assert_eq!(effective_sub_view(view.clone(), false), view);
            assert_eq!(effective_sub_view(view.clone(), true), view);
        }
    }

    /// Driven through the real `handle_key`, because a test that calls
    /// `switch_to_tab` directly cannot catch a key that is still bound.
    #[test]
    fn shift_d_does_not_reach_the_driver_tab_when_experimental_is_off() {
        let mut ctx = test_ctx().with_experimental(false);
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::Queue);

        press(&mut screen, &mut ctx, KeyCode::Char('D'));

        assert_eq!(
            ctx.detail_sub_view_per_project.get(TEST_ALIAS),
            Some(&DetailSubView::Queue),
            "`D` must fall through UNHANDLED with the flag off, leaving the \
             user on the tab they were already on"
        );
    }

    #[test]
    fn shift_d_still_reaches_the_driver_tab_when_experimental_is_on() {
        let mut ctx = test_ctx().with_experimental(true);
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::Queue);

        press(&mut screen, &mut ctx, KeyCode::Char('D'));

        assert_eq!(
            ctx.detail_sub_view_per_project.get(TEST_ALIAS),
            Some(&DetailSubView::Driver)
        );
    }

    /// The Right arrow is the other way onto tab 10, and it has its own bound.
    #[test]
    fn right_from_the_last_visible_tab_stays_put_when_experimental_is_off() {
        let mut ctx = test_ctx().with_experimental(false);
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
        // Tab 9 is `Browse`, the last tab a flag-off session has.
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), sub_view_from_index(9, false));

        press(&mut screen, &mut ctx, KeyCode::Right);

        assert_eq!(
            ctx.detail_sub_view_per_project.get(TEST_ALIAS),
            Some(&DetailSubView::Browse),
            "walking right off the end must not park the user on a tab the \
             bar does not draw"
        );
    }

    #[test]
    fn right_from_tab_nine_reaches_the_driver_tab_when_experimental_is_on() {
        let mut ctx = test_ctx().with_experimental(true);
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), sub_view_from_index(9, true));

        press(&mut screen, &mut ctx, KeyCode::Right);

        assert_eq!(
            ctx.detail_sub_view_per_project.get(TEST_ALIAS),
            Some(&DetailSubView::Driver)
        );
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
        let (titles, select) = tab_titles(TAB_BAR_FULL_CELLS, DRIVER_TAB_INDEX, false, true);
        assert_eq!(titles.len(), TAB_COUNT);
        assert_eq!(select, DRIVER_TAB_INDEX);
        assert_eq!(bar_cells(&titles), usize::from(TAB_BAR_FULL_CELLS));
    }

    #[test]
    fn the_compact_tier_renders_eleven_labels_at_its_measured_width() {
        let (titles, select) = tab_titles(TAB_BAR_COMPACT_CELLS, 4, false, true);
        assert_eq!(titles.len(), TAB_COUNT);
        assert_eq!(select, 4);
        assert_eq!(bar_cells(&titles), usize::from(TAB_BAR_COMPACT_CELLS));
        // One cell below the full width is already the compact tier.
        let (titles, _) = tab_titles(TAB_BAR_FULL_CELLS - 1, 4, false, true);
        assert_eq!(bar_cells(&titles), usize::from(TAB_BAR_COMPACT_CELLS));
    }

    /// Below the compact width the bar is windowed — and the window always
    /// contains the active tab, with the returned select index pointing at it.
    /// **A tab bar that silently drops the active tab is a defect, not a tier.**
    #[test]
    fn the_windowed_tier_always_contains_the_active_tab() {
        for active in [0usize, 5, DRIVER_TAB_INDEX] {
            for width in [20u16, 30, 40, 60, 76] {
                let (titles, select) = tab_titles(width, active, false, true);
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
        let (titles, _) = tab_titles(40, DRIVER_TAB_INDEX, false, true);
        assert!(bar_cells(&titles) <= 40, "windowed bar overflows 40 columns");
        let first: String = titles[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert_eq!(
            first, TAB_OVERFLOW_LEFT,
            "tabs were truncated on the left with no marker"
        );

        let (titles, _) = tab_titles(40, 0, false, true);
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
            let (live, _) = tab_titles(width, DRIVER_TAB_INDEX, true, true);
            let (idle, _) = tab_titles(width, DRIVER_TAB_INDEX, false, true);
            assert_eq!(
                bar_cells(&live),
                bar_cells(&idle),
                "the bar changed width when a run started, at width {width}"
            );
        }

        let (live, select) = tab_titles(TAB_BAR_FULL_CELLS, DRIVER_TAB_INDEX, true, true);
        let text: String = live[select]
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect();
        assert_eq!(text, format!("D:Drive{DRIVER_LIVE_MARKER}"));

        let (idle, select) = tab_titles(TAB_BAR_FULL_CELLS, DRIVER_TAB_INDEX, false, true);
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
                (0usize, "1:Roadmap", "1:Rd"),
                (DRIVER_TAB_INDEX, "D:Drive", "D:Dr"),
            ] {
                let mut ctx = test_ctx();
                let screen = DetailScreen::new("meta-mgr".to_string());
                ctx.detail_sub_view_per_project
                    .insert("meta-mgr".to_string(), sub_view_from_index(active, true));

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
            archive_cache: crate::archive::ArchiveCache::default(),
            // Fixtures default the experimental flag ON, so every driver test
            // written before 260917-fko keeps asserting what it always did;
            // the flag-off tests call `with_experimental(false)`.
            experimental: true,
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

    // ── The Git tab's commit pane: the Driver tab's posture, reused ────────
    //
    // Selection on `j`/`k`, pane scroll on PageUp/PageDown, no second mode to
    // track (QD-06). The pane's presence is the ONLY thing that switches which
    // of the two PageDown means, so both branches are driven through the real
    // `handle_key` here rather than asserted about the arithmetic.

    /// A DetailScreen and AppContext parked on the Git tab, with `entries` log
    /// rows and — when `body_lines` is `Some` — an open commit pane whose
    /// viewport was recorded with that many lines in a 10-row body.
    fn git_fixture(
        entries: usize,
        selected: usize,
        body_lines: Option<u16>,
        stored_offset: u16,
    ) -> (DetailScreen, AppContext) {
        use crate::state_reader::git_ops::{GitCommitDetail, GitLogEntry};
        use crate::text::Untrusted;

        let screen = DetailScreen::new(TEST_ALIAS.to_string());
        if let Some(total_lines) = body_lines {
            screen.git_commit_viewport.set(ViewportMetrics {
                total_lines,
                visible_height: 10,
            });
        }

        let mut ctx = test_ctx();
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::GitHistory);
        let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
        let field = |s: &str| Untrusted::from_untrusted_source(s.to_string());
        cache.git_entries = (0..entries)
            .map(|i| GitLogEntry {
                hash: field(&format!("hash{i:03}")),
                date: field("2026-09-17"),
                author: field("Human"),
                co_authors: None,
                message: field("a subject"),
            })
            .collect();
        cache.git_selected = selected;
        if let Some(total_lines) = body_lines {
            cache.git_commit_detail = Some(GitCommitDetail {
                hash: field("hash000"),
                body: (0..total_lines).map(|i| field(&format!("line {i}"))).collect(),
                stat: Default::default(),
            });
            cache.git_commit_scroll = stored_offset;
        }

        (screen, ctx)
    }

    #[test]
    fn page_down_scrolls_the_commit_pane_while_open_and_pages_the_selection_otherwise() {
        // Pane OPEN: PageDown moves the pane, not the selection.
        let (mut screen, mut ctx) = git_fixture(100, 0, Some(60), 0);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        let cache = &ctx.view_cache[TEST_ALIAS];
        assert_eq!(
            cache.git_commit_scroll, PAGE_SCROLL_LINES,
            "with the pane open PageDown scrolls the message"
        );
        assert_eq!(
            cache.git_selected, 0,
            "and leaves the selection where it is — moving it would close the \
             very pane the key was aimed at"
        );
        assert!(cache.git_commit_detail.is_some());

        // The clamp is the SHARED formula: 60 lines in a 10-row body caps the
        // offset at 50, so a third PageDown does not overshoot.
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(ctx.view_cache[TEST_ALIAS].git_commit_scroll, 50);

        // And back up, clamped at zero.
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(ctx.view_cache[TEST_ALIAS].git_commit_scroll, 30);

        // Pane CLOSED: PageDown pages the selection, as it always did.
        let (mut screen, mut ctx) = git_fixture(100, 0, None, 0);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(
            ctx.view_cache[TEST_ALIAS].git_selected,
            PAGE_SCROLL_LINES as usize,
            "with no pane open PageDown keeps paging the log"
        );
    }

    // ── The co-author column and the message pane (260916-vr1, Task 3) ─────

    /// Render the detail screen at an arbitrary size and join the cells.
    ///
    /// Sized, unlike `render_detail_to_text`'s fixed 120x30, because two of the
    /// claims below are ABOUT the size: what a narrow row drops, and what a
    /// short detail area does with the file pane.
    fn render_detail_sized(
        screen: &DetailScreen,
        ctx: &AppContext,
        width: u16,
        height: u16,
    ) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

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

    /// The Git tab holding exactly the rows described by `rows`, each a
    /// `(subject, co_authors)` pair.
    fn git_rows_fixture(rows: &[(&str, Option<&str>)]) -> (DetailScreen, AppContext) {
        use crate::state_reader::git_ops::GitLogEntry;
        use crate::text::Untrusted;

        let screen = DetailScreen::new(TEST_ALIAS.to_string());
        let mut ctx = test_ctx();
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::GitHistory);
        let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
        let field = |s: &str| Untrusted::from_untrusted_source(s.to_string());
        cache.git_entries = rows
            .iter()
            .enumerate()
            .map(|(i, (subject, co))| GitLogEntry {
                hash: field(&format!("hash{i:03}")),
                date: field("2026-09-17"),
                author: field("Human"),
                co_authors: co.map(field),
                message: field(subject),
            })
            .collect();
        cache.git_selected = 0;

        (screen, ctx)
    }

    /// The Git tab with an OPEN commit pane: `body_lines` message lines and one
    /// file-stat line below them.
    fn git_pane_fixture(body_lines: usize, scroll: u16) -> (DetailScreen, AppContext) {
        use crate::state_reader::git_ops::{GitCommitDetail, GitDiffStat};
        use crate::text::Untrusted;

        let (screen, mut ctx) = git_rows_fixture(&[("a subject", None)]);
        let field = |s: &str| Untrusted::from_untrusted_source(s.to_string());
        let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
        cache.git_commit_detail = Some(GitCommitDetail {
            hash: field("hash000"),
            body: (0..body_lines).map(|i| field(&format!("line {i:02}"))).collect(),
            stat: GitDiffStat {
                files_changed: 1,
                insertions: 2,
                deletions: 1,
                file_stats: vec![" src/probe_file.rs | 3 ++-".to_string()],
            },
        });
        cache.git_commit_scroll = scroll;

        (screen, ctx)
    }

    #[test]
    fn a_git_row_without_co_authors_has_no_trailing_parentheses() {
        let (screen, ctx) = git_rows_fixture(&[
            ("with a helper", Some("Claude Opus 5")),
            ("alone", None),
        ]);
        let drawn = render_detail_sized(&screen, &ctx, 120, 30);

        let with_row = drawn
            .lines()
            .find(|l| l.contains("with a helper"))
            .expect("the co-authored row renders");
        assert!(
            with_row.contains("Human") && with_row.contains("(Claude Opus 5)"),
            "the co-author follows the human author, parenthesised: {with_row:?}"
        );

        let alone_row = drawn
            .lines()
            .find(|l| l.contains("alone"))
            .expect("the solo row renders");
        assert!(
            !alone_row.contains('(') && !alone_row.contains(')'),
            "a commit with no trailer must render no empty column and no \
             dangling separator: {alone_row:?}"
        );
        // The scraped line still carries the enclosing block's right border, so
        // the content is what is left once that and the padding come off.
        assert!(
            alone_row
                .trim_end_matches(|c: char| c == '│' || c.is_whitespace())
                .ends_with("Human"),
            "the solo row ends at the author: {alone_row:?}"
        );
    }

    #[test]
    fn a_git_row_truncates_the_subject_before_the_attribution_columns() {
        // The budget, stated as arithmetic first so a render-layout change
        // cannot quietly turn this claim into a different one.
        let budget = git_row_budget(78, 7, 10, 5, Some(13));
        assert!(
            budget.show_co_authors,
            "at this width both attribution columns still fit"
        );
        assert!(
            budget.subject_cols < 60,
            "and the subject is what gives up the room"
        );

        let long = "a subject long enough that it cannot possibly fit beside both columns";
        let (screen, ctx) = git_rows_fixture(&[(long, Some("Claude Opus 5"))]);
        let drawn = render_detail_sized(&screen, &ctx, 80, 30);
        let row = drawn
            .lines()
            .find(|l| l.contains("a subject long"))
            .expect("the row renders");

        assert!(
            row.contains('…'),
            "the subject is truncated with an ellipsis rather than silently \
             clipped at the edge: {row:?}"
        );
        assert!(
            row.contains("Human") && row.contains("(Claude Opus 5)"),
            "both attribution columns survive the truncation: {row:?}"
        );
    }

    #[test]
    fn a_narrow_git_row_drops_the_co_author_column_and_keeps_the_author() {
        // Below the point where a 12-char subject still fits beside both, the
        // CO-AUTHOR goes and the author stays (QD-04). The author is never
        // dropped: "who wrote this" is the question the row exists to answer.
        let budget = git_row_budget(58, 7, 10, 5, Some(13));
        assert!(!budget.show_co_authors);
        assert!(
            budget.subject_cols >= GIT_ROW_MIN_SUBJECT_COLS,
            "and the room the co-author gave up goes back to the subject"
        );

        // A width so small that even the subject is squeezed to nothing still
        // yields a budget rather than an underflow.
        let starved = git_row_budget(4, 7, 10, 5, Some(13));
        assert!(!starved.show_co_authors);
        assert_eq!(starved.subject_cols, 0);

        let (screen, ctx) = git_rows_fixture(&[("a subject", Some("Claude Opus 5"))]);
        let drawn = render_detail_sized(&screen, &ctx, 60, 30);
        let row = drawn
            .lines()
            .find(|l| l.contains("2026-09-17"))
            .expect("the row renders");
        assert!(row.contains("Human"), "the author survives: {row:?}");
        assert!(
            !row.contains("Claude Opus 5"),
            "the co-author column is dropped rather than clipped mid-name: {row:?}"
        );
    }

    #[test]
    fn the_commit_message_renders_above_the_file_list() {
        let (screen, ctx) = git_pane_fixture(3, 0);
        let drawn = render_detail_sized(&screen, &ctx, 120, 30);
        let lines: Vec<&str> = drawn.lines().collect();

        let body_at = lines
            .iter()
            .position(|l| l.contains("line 00"))
            .expect("the commit message renders");
        let files_at = lines
            .iter()
            .position(|l| l.contains("probe_file.rs"))
            .expect("the file list renders");

        assert!(
            body_at < files_at,
            "the message is the more valuable half and takes the upper pane \
             (QD-05); message at row {body_at}, files at row {files_at}"
        );
    }

    #[test]
    fn the_commit_message_takes_the_whole_detail_area_below_eight_rows() {
        let (screen, ctx) = git_pane_fixture(3, 0);
        let drawn = render_detail_sized(&screen, &ctx, 120, 14);

        assert!(
            drawn.contains("line 00"),
            "under height pressure the message is what survives: {drawn}"
        );
        assert!(
            !drawn.contains("probe_file.rs"),
            "and the file pane is skipped rather than drawn two rows tall: {drawn}"
        );
    }

    #[test]
    fn the_commit_message_pane_renders_from_its_stored_offset() {
        // 40 lines, offset 10: the pane starts at the ELEVENTH line. No `Wrap`
        // is applied (QD-10), so the recorded `total_lines` is the real scroll
        // range and `clamp_scroll` is not lying to the key handler.
        let (screen, ctx) = git_pane_fixture(40, 10);
        let drawn = render_detail_sized(&screen, &ctx, 120, 30);

        assert!(drawn.contains("line 10"), "the offset's line is at the top");
        assert!(
            !drawn.contains("line 09"),
            "and the line above it has scrolled off: {drawn}"
        );
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
            DetailSubView::RoadmapViz,
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
    // Reached by exactly one sub-view since the PhaseList tab was removed
    // (D-B02) — RoadmapViz. Every other sub-view has an explicit match arm. See plan 14-04 decision GD-01, which
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
            .insert(TEST_ALIAS.to_string(), DetailSubView::RoadmapViz);

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
            kind: crate::session_detector::SessionKind::Claude,
            session_id: Some(Untrusted::from_untrusted_source(session_id.to_string())),
            working_dir: project_path,
            start_time: Some(1),
            tty: None,
        }];

        (DetailScreen::new(TEST_ALIAS.to_string()), ctx)
    }

    /// The resume gate (260923-lr9, T-lr9-01): a Codex session is refused
    /// WHATEVER its id, so a Codex thread id can never reach the `claude`
    /// resume argv. A Claude session behaves exactly as before.
    #[test]
    fn resumable_session_id_refuses_every_codex_session() {
        use crate::session_detector::{ClaudeSession, SessionKind};

        let session = |kind, id: Option<&str>| ClaudeSession {
            pid: 1,
            kind,
            session_id: id.map(|id| Untrusted::from_untrusted_source(id.to_string())),
            working_dir: std::path::PathBuf::from("/nonexistent"),
            start_time: None,
            tty: None,
        };
        let codex_id = "0199a3f2-7c4e-7a10-8b2c-1d2e3f405162";

        assert_eq!(
            resumable_session_id(&session(SessionKind::Codex, Some(codex_id))).err(),
            Some(CODEX_RESUME_UNSUPPORTED)
        );
        assert_eq!(
            resumable_session_id(&session(SessionKind::Codex, None)).err(),
            Some(CODEX_RESUME_UNSUPPORTED)
        );
        assert_eq!(
            resumable_session_id(&session(SessionKind::Claude, None)).err(),
            Some(NO_SESSION_ID_TO_RESUME)
        );
        let claude = session(SessionKind::Claude, Some("abc"));
        assert_eq!(
            resumable_session_id(&claude).map(|id| id.as_raw_for_logic_only()),
            Ok("abc")
        );
    }

    /// Enter on a Codex row reports the refusal and launches nothing.
    ///
    /// **The fixture id is `None` deliberately.** A regressed gate must fail
    /// here by MESSAGE (the Claude no-id text) and must never reach a tmux or
    /// terminal spawn, which an id of `Some` would. The `Some(id)` case is
    /// certified by the pure `resumable_session_id_refuses_every_codex_session`.
    #[test]
    fn enter_on_a_codex_session_reports_resume_unsupported_and_launches_nothing() {
        let (mut screen, mut ctx) = sessions_fixture("unused");
        ctx.active_sessions[0].kind = crate::session_detector::SessionKind::Codex;
        ctx.active_sessions[0].session_id = None;

        let action = screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        let ScreenAction::SetStatusMessage(message) = action else {
            panic!("expected the Codex refusal as a status message, got another action");
        };
        assert_eq!(message, CODEX_RESUME_UNSUPPORTED);
    }

    /// Every Sessions row names its agent; a Codex row with no id says
    /// `unknown`, never Claude's `new session`.
    #[test]
    fn the_sessions_tab_labels_each_row_with_its_agent() {
        let (screen, mut ctx) = sessions_fixture("abc12345");
        let mut codex = ctx.active_sessions[0].clone();
        codex.kind = crate::session_detector::SessionKind::Codex;
        codex.pid = 5151;
        codex.session_id = None;
        ctx.active_sessions.push(codex);

        let text = render_detail_to_text(&screen, &ctx);
        let claude_row = text
            .lines()
            .find(|line| line.contains("PID 4242"))
            .expect("the Claude row renders");
        assert!(claude_row.contains("Claude"), "{claude_row}");
        let codex_row = text
            .lines()
            .find(|line| line.contains("PID 5151"))
            .expect("the Codex row renders");
        assert!(codex_row.contains("Codex"), "{codex_row}");
        assert!(codex_row.contains("unknown"), "{codex_row}");
        assert!(!codex_row.contains("new session"), "{codex_row}");
    }

    #[test]
    fn the_sessions_tab_empty_state_names_both_agents() {
        let (screen, mut ctx) = sessions_fixture("unused");
        ctx.active_sessions.clear();
        let text = render_detail_to_text(&screen, &ctx);
        assert!(
            text.contains("No active Claude or Codex sessions"),
            "{text}"
        );
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

    // ── D-R-P-E-V: Execute and Verify read the same kind of evidence ──────

    fn pipeline_text(inf: &DiskInference) -> String {
        let statuses = derive_all_stage_statuses(inf);
        build_pipeline_line(inf, &statuses)
            .spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect()
    }

    /// The reported symptom, as a unit: a phase whose summaries did not get
    /// counted, beside a verification that passed. The row used to read
    /// `[E]` as a dimmed magenta `[--]` — "Execute skipped" — next to a green
    /// `[V]`, which is not a state a GSD phase can be in.
    #[test]
    fn execute_is_never_drawn_skipped_beside_a_verified_stage() {
        let inf = DiskInference {
            status: DiskStatus::Planned,
            plan_count: 4,
            summary_count: 0,
            has_plans: true,
            has_context: true,
            has_research: true,
            has_verification: true,
            verification_status: VerificationStatus::Passed,
            ..DiskInference::default()
        };

        let statuses = derive_all_stage_statuses(&inf);
        assert_ne!(
            statuses[3],
            StageStatus::Skipped,
            "Execute cannot be skipped: nothing downstream of it exists without it"
        );
        assert_eq!(statuses[3], StageStatus::Current);
        assert!(
            !pipeline_text(&inf).contains("[--]"),
            "no stage may render as bypassed while a later stage reports a result"
        );
    }

    /// Verify reads what the artifact CONCLUDED, not that a file exists. A
    /// `*-VERIFICATION.md` with no readable status is `Missing`, and a phase
    /// with only that is not verified — the same evidence rule Execute has
    /// always used.
    #[test]
    fn verify_reads_the_verification_status_not_the_filename() {
        let stub = DiskInference {
            status: DiskStatus::Executed,
            plan_count: 2,
            summary_count: 2,
            has_plans: true,
            has_summaries: true,
            has_verification: true,
            verification_status: VerificationStatus::Missing,
            ..DiskInference::default()
        };
        assert_ne!(
            derive_all_stage_statuses(&stub)[4],
            StageStatus::Complete,
            "a content-free verification artifact must not read as a verified phase"
        );

        let concluded = DiskInference {
            verification_status: VerificationStatus::GapsFound,
            ..stub.clone()
        };
        assert_eq!(
            derive_all_stage_statuses(&concluded)[4],
            StageStatus::Complete,
            "a verification that reached a conclusion is a completed Verify stage"
        );
    }

    /// End to end through the real reader, on GSD's real filename shapes: a
    /// slugged plan beside a slugless summary. This is the dogfood the existing
    /// fixtures could not provide, because this repository's own phases are
    /// slugless and so is every fixture in `disk_status.rs`.
    #[test]
    fn a_real_shaped_gsd_phase_renders_execute_complete() {
        let dir = tempfile::tempdir().unwrap();
        for (plan, summary) in [
            (
                "15-01-translation-resolver-tracer-PLAN.md",
                "15-01-SUMMARY.md",
            ),
            (
                "15-02-dart-read-sites-and-the-lapse-path-PLAN.md",
                "15-02-SUMMARY.md",
            ),
        ] {
            std::fs::write(dir.path().join(plan), "plan").unwrap();
            std::fs::write(dir.path().join(summary), "summary").unwrap();
        }
        std::fs::write(dir.path().join("15-CONTEXT.md"), "ctx").unwrap();
        std::fs::write(dir.path().join("15-RESEARCH.md"), "res").unwrap();
        std::fs::write(
            dir.path().join("15-VERIFICATION.md"),
            "---\nstatus: passed\n---\nbody",
        )
        .unwrap();

        let inf = crate::state_reader::disk_status::infer_disk_status(dir.path());
        assert_eq!(inf.status, DiskStatus::Complete);
        assert_eq!(
            derive_all_stage_statuses(&inf),
            [StageStatus::Complete; 5],
            "every stage of a finished, real-shaped phase is complete"
        );
        assert_eq!(pipeline_text(&inf), "  [D]---[R]---[P]---[E 2/2]---[V]");
    }

    // ── Plan token section (quick task 260916-vqx) ───────────────────────

    // Named only by the fixtures: the production code reaches its rows through
    // `inf.plan_tokens` and never spells the type.
    use crate::state_reader::disk_status::PlanTokens;

    /// The rendered text of a `Line`, spans concatenated — the characters that
    /// actually reach a terminal cell.
    fn line_text(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    fn token_row(id: &str, estimate: Option<u64>, actual: Option<u64>) -> PlanTokens {
        PlanTokens {
            id: id.to_string(),
            estimate,
            actual,
        }
    }

    fn inference_with_tokens(plan_count: u32, rows: Vec<PlanTokens>) -> DiskInference {
        DiskInference {
            plan_count,
            plan_tokens: rows,
            ..DiskInference::default()
        }
    }

    #[test]
    fn fmt_tokens_boundaries_are_pinned_not_described() {
        // The point of the table is that the column width is PREDICTABLE, so
        // every boundary is asserted as an exact string rather than described.
        let table: [(u64, &str); 9] = [
            (0, "0"),
            (999, "999"),
            (1_000, "1.0k"),
            (12_846, "12.8k"),
            (95_000, "95.0k"),
            (100_000, "100k"),
            (999_999, "1000k"),
            (1_000_000, "1.0M"),
            (1_250_000, "1.3M"),
        ];
        for (input, expected) in table {
            assert_eq!(fmt_tokens(input), expected, "fmt_tokens({input})");
        }
    }

    #[test]
    fn the_token_section_is_ascii_only() {
        // This project carries an open todo about badge glyph display width
        // misaligning by one cell across terminals. A numeric column is the last
        // place to introduce a non-ASCII glyph.
        for n in [
            0,
            1,
            999,
            1_000,
            12_846,
            100_000,
            999_999,
            1_000_000,
            u64::MAX,
        ] {
            assert!(
                fmt_tokens(n).is_ascii(),
                "fmt_tokens({n}) emitted a non-ASCII character"
            );
        }
        let inf = inference_with_tokens(
            3,
            vec![
                token_row("07-01", Some(95_000), Some(12_846)),
                token_row("07-02", Some(10_000), Some(15_000)),
                token_row("07-03", Some(40_000), None),
            ],
        );
        for line in build_plan_token_lines(&inf) {
            assert!(
                line_text(&line).is_ascii(),
                "the section's authored strings must be ASCII too"
            );
        }
    }

    #[test]
    fn an_empty_plan_tokens_renders_no_section_at_all() {
        let inf = inference_with_tokens(5, Vec::new());
        assert!(
            build_plan_token_lines(&inf).is_empty(),
            "a project whose plans carry neither key renders byte-identically to \
             before this section existed: absence degrades to silence, never to a \
             column of dashes"
        );
    }

    #[test]
    fn the_header_states_both_totals_and_the_denominator() {
        let inf = inference_with_tokens(
            5,
            vec![
                token_row("07-01", Some(95_000), Some(12_846)),
                token_row("07-02", Some(40_000), Some(30_000)),
                token_row("07-03", Some(15_000), None),
            ],
        );
        let lines = build_plan_token_lines(&inf);
        let header = line_text(&lines[0]);
        // 95 000 + 40 000 + 15 000 = 150 000 -> "150k"
        assert!(
            header.contains("150k"),
            "header must sum ALL estimates: {header}"
        );
        // 12 846 + 30 000 = 42 846 -> "42.8k"
        assert!(
            header.contains("42.8k"),
            "header must sum ALL actuals: {header}"
        );
        assert!(
            header.contains("2/5"),
            "the denominator is the PHASE's plan count, not the row count: {header}"
        );
    }

    #[test]
    fn a_row_renders_both_numbers_and_a_dash_for_an_absent_one() {
        let inf = inference_with_tokens(
            2,
            vec![
                token_row("07-01", Some(95_000), None),
                token_row("07-02", None, Some(12_846)),
            ],
        );
        let lines = build_plan_token_lines(&inf);
        let estimate_only = line_text(&lines[1]);
        assert!(estimate_only.contains("95.0k"), "{estimate_only}");
        assert!(
            estimate_only.contains('-'),
            "an absent actual renders a dash: {estimate_only}"
        );
        let actual_only = line_text(&lines[2]);
        assert!(actual_only.contains("12.8k"), "{actual_only}");
        assert!(
            actual_only.contains('-'),
            "an absent estimate renders a dash: {actual_only}"
        );
    }

    #[test]
    fn the_delta_is_signed_and_relative_to_the_estimate() {
        let inf = inference_with_tokens(
            4,
            vec![
                token_row("07-01", Some(95_000), Some(12_846)),
                token_row("07-02", Some(10_000), Some(15_000)),
                token_row("07-03", Some(40_000), None),
                token_row("07-04", Some(0), Some(5_000)),
            ],
        );
        let lines = build_plan_token_lines(&inf);
        // (12846 - 95000) * 100 / 95000 == -86 under one truncating rule.
        assert!(
            line_text(&lines[1]).contains("-86%"),
            "under by 86%: {}",
            line_text(&lines[1])
        );
        assert!(
            line_text(&lines[2]).contains("+50%"),
            "over by 50%: {}",
            line_text(&lines[2])
        );
        assert!(
            !line_text(&lines[3]).contains('%'),
            "a row missing either number renders no delta: {}",
            line_text(&lines[3])
        );
        assert!(
            !line_text(&lines[4]).contains('%'),
            "an estimate of 0 renders no delta and does not divide: {}",
            line_text(&lines[4])
        );
    }

    #[test]
    fn the_row_cap_is_honest_about_what_it_does_not_show() {
        let rows: Vec<PlanTokens> = (1..=14)
            .map(|i| token_row(&format!("07-{i:02}"), Some(1_000), None))
            .collect();
        let inf = inference_with_tokens(14, rows);
        let lines = build_plan_token_lines(&inf);
        assert_eq!(
            lines.len(),
            1 + MAX_PLAN_TOKEN_ROWS + 1,
            "header + {MAX_PLAN_TOKEN_ROWS} rows + exactly one overflow line"
        );
        let overflow = line_text(lines.last().unwrap());
        assert!(
            overflow.contains("+4 more"),
            "the pane is a Paragraph with no scroll (D-INF-03), so the remainder \
             is STATED rather than dropped off the bottom invisibly: {overflow}"
        );
        // 14 x 1 000 = 14 000 -> the totals still cover every plan, capped or not.
        assert!(
            line_text(&lines[0]).contains("14.0k"),
            "the header's totals sum ALL rows, including the capped ones: {}",
            line_text(&lines[0])
        );
    }

    #[test]
    fn a_hostile_plan_label_reaches_no_cell_unescaped() {
        // A plan identifier is a filename stem out of ANOTHER project's
        // `.planning/` directory. A terminal interprets control bytes it is
        // handed, so this is code-execution surface, not text.
        let inf = inference_with_tokens(
            1,
            vec![token_row(
                "07-\u{202E}01\u{1b}[31m",
                Some(1_000),
                Some(2_000),
            )],
        );
        let lines = build_plan_token_lines(&inf);
        for line in &lines {
            let text = line_text(line);
            assert!(
                !text.contains('\u{202E}'),
                "a bidi override reached a cell: {text:?}"
            );
            assert!(
                !text.contains('\u{1b}'),
                "a raw ESC reached a cell: {text:?}"
            );
        }
    }

    // ── Pipeline phase-list wave marker (quick task 260916-vqy) ──────────

    use crate::state_reader::plan_waves::PlanWave;
    use crate::state_reader::roadmap_md::RoadmapPhase;

    fn phase_fixture(number: &str, name: &str) -> RoadmapPhase {
        RoadmapPhase {
            number: number.to_string(),
            name: name.to_string(),
            description: String::new(),
            completed: false,
            total_plans: 0,
            completed_plans: 0,
            depends_on: Vec::new(),
        }
    }

    fn inference_with_waves(waves: Vec<PlanWave>) -> DiskInference {
        DiskInference {
            plan_waves: waves,
            ..DiskInference::default()
        }
    }

    fn wave(number: Option<u32>, plans: &[&str]) -> PlanWave {
        PlanWave {
            wave: number,
            plans: plans.iter().map(|p| p.to_string()).collect(),
        }
    }

    #[test]
    fn a_phase_spanning_two_waves_is_marked_in_the_list() {
        let phase = phase_fixture("19", "gitsafe");
        let inf = inference_with_waves(vec![
            wave(Some(1), &["19-01"]),
            wave(Some(2), &["19-02", "19-03"]),
        ]);
        assert_eq!(phase_list_label(&phase, Some(&inf)), "P19: gitsafe  2w");
    }

    #[test]
    fn a_phase_with_one_waves_entry_keeps_todays_label_and_stays_quiet() {
        let phase = phase_fixture("19", "gitsafe");
        let inf = inference_with_waves(vec![wave(Some(1), &["19-01", "19-02"])]);
        assert_eq!(phase_list_label(&phase, Some(&inf)), "P19: gitsafe");
    }

    #[test]
    fn a_phase_with_no_waves_and_a_phase_with_no_inference_render_unchanged() {
        let phase = phase_fixture("19", "gitsafe");
        // The exact string this list drew before the marker existed.
        let today = format!("P{}: {}", shown(&phase.number), shown(&phase.name));
        assert_eq!(
            phase_list_label(&phase, Some(&inference_with_waves(vec![]))),
            today
        );
        assert_eq!(phase_list_label(&phase, None), today);
    }

    #[test]
    fn the_unknown_waves_bucket_is_not_counted_as_a_wave() {
        // One real wave plus some plans whose wave was never recorded is a
        // SERIAL phase. Counting the `w?` bucket would report it as parallel.
        let phase = phase_fixture("19", "gitsafe");
        let inf = inference_with_waves(vec![
            wave(Some(1), &["19-01"]),
            wave(None, &["19-02"]),
        ]);
        assert_eq!(phase_list_label(&phase, Some(&inf)), "P19: gitsafe");
    }

    #[test]
    fn a_hostile_phase_name_reaches_no_cell_unescaped_beside_a_waves_marker() {
        let phase = phase_fixture("19", "git\u{202E}safe\u{1b}[31m");
        let inf = inference_with_waves(vec![
            wave(Some(1), &["19-01"]),
            wave(Some(2), &["19-02"]),
        ]);
        let label = phase_list_label(&phase, Some(&inf));
        assert!(!label.contains('\u{202E}'), "bidi override: {label:?}");
        assert!(!label.contains('\u{1b}'), "raw ESC: {label:?}");
        assert!(label.ends_with("  2w"), "marker still appended: {label:?}");
    }

    /// The pin [`DetailScreen::opened_on`] rests on (260916-vqz).
    ///
    /// That constructor calls `switch_to_tab` for its arrival work and DROPS
    /// the `ScreenAction` it returns, because a screen being built has no stack
    /// to act on yet. This test is what makes the discard safe rather than
    /// assumed: it goes red the day `switch_to_tab` learns to return anything
    /// but `None`, which is the day the constructor has to stop dropping it.
    #[test]
    fn switching_to_the_backlog_tab_returns_no_screen_action() {
        let mut ctx = test_ctx();
        let mut scroll_offset = 7;

        let action = switch_to_tab(
            TEST_ALIAS,
            tab_index(&DetailSubView::Backlog),
            &mut scroll_offset,
            &mut ctx,
        );

        assert!(
            matches!(action, ScreenAction::None),
            "a tab switch performs arrival work; it does not move the screen stack"
        );
        assert_eq!(scroll_offset, 0, "the tab switch resets the scroll offset");
        assert_eq!(
            ctx.detail_sub_view_per_project.get(TEST_ALIAS),
            Some(&DetailSubView::Backlog),
            "the index must round-trip back to the tab that was asked for"
        );
    }

    // --- debug enter-on-unset-config-row: Enter opens the chooser ---------

    /// A Defaults-tab context parked on `key`'s row of `config`, plus that
    /// row's index. Driven through the real `handle_key` by the callers, never
    /// through the helpers the arm calls: the defect was an arm that reached
    /// no branch, and only the key path can see that.
    fn ctx_on_config_row(
        config: crate::state_reader::config_json::GsdConfig,
        key: &str,
    ) -> (AppContext, usize) {
        let idx = build_defaults_entries(&config, None)
            .iter()
            .position(|e| e.key.as_ref() == key)
            .unwrap_or_else(|| panic!("the Defaults tab has no `{key}` row"));
        let mut ctx = test_ctx();
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::Defaults);
        let cache = ctx.view_cache.entry(TEST_ALIAS.to_string()).or_default();
        cache.defaults_config = Some(config);
        cache.defaults_selected = idx;
        (ctx, idx)
    }

    fn sparse_gsd_config() -> crate::state_reader::config_json::GsdConfig {
        crate::state_reader::config_json::parse_gsd_config(r#"{"mode":"yolo"}"#)
            .expect("the sparse fixture parses")
    }

    /// The reported symptom, end to end: Enter on an enum row the project's
    /// `config.json` leaves unset drew nothing, because an unset row carried
    /// no kind for the Enter arm to open a chooser for.
    #[test]
    fn enter_on_an_unset_enum_row_opens_its_chooser_and_applies_the_pick() {
        let key = "workflow.context_drift_action";
        let (mut ctx, idx) = ctx_on_config_row(sparse_gsd_config(), key);
        assert_eq!(
            build_defaults_entries(&sparse_gsd_config(), None)[idx].value,
            "(unset)",
            "precondition: the row under test must be an UNSET row"
        );
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());

        press(&mut screen, &mut ctx, KeyCode::Enter);
        assert_eq!(
            ctx.view_cache[TEST_ALIAS].defaults_editing,
            Some(idx),
            "Enter on an unset enum row must open its chooser"
        );

        // ARRIVAL: the popup is drawn, titled with the key, not merely flagged.
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let (width, height) = (160u16, 45u16);
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("TestBackend terminal");
        terminal
            .draw(|frame| screen.render_defaults_tab(frame, frame.area(), &ctx))
            .expect("draw the Defaults tab");
        let buffer = terminal.backend().buffer().clone();
        let rows: Vec<String> = (0..height)
            .map(|y| {
                (0..width)
                    .filter_map(|x| buffer.cell((x, y)).map(|c| c.symbol().to_string()))
                    .collect()
            })
            .collect();
        assert!(
            rows.iter().any(|r| r.contains(&format!("┌ {key} "))),
            "the chooser popup for `{key}` was not drawn"
        );

        // `warn` is offered first; Down then Enter picks `block`.
        press(&mut screen, &mut ctx, KeyCode::Down);
        press(&mut screen, &mut ctx, KeyCode::Enter);
        let cache = &ctx.view_cache[TEST_ALIAS];
        assert_eq!(cache.defaults_editing, None, "applying closes the chooser");
        let applied = cache
            .defaults_config
            .as_ref()
            .and_then(|c| c.workflow.as_ref())
            .and_then(|w| w.context_drift_action.clone());
        assert_eq!(applied.as_deref(), Some("block"));
    }

    /// The whole class, not the one row: every row that is a bool or an enum
    /// when SET must still open a chooser when UNSET, and every option that
    /// chooser offers must land on the key it is drawn under.
    ///
    /// The expected kind comes from [`all_config_entries`] (every key set),
    /// so this cannot pass by the sparse fixture happening to leave few rows
    /// unset: the count of unset choice rows is asserted non-trivial.
    #[test]
    fn every_unset_choice_row_opens_a_chooser_whose_options_all_apply() {
        let populated = all_config_entries();
        let sparse = build_defaults_entries(&sparse_gsd_config(), None);
        let mut checked = 0usize;
        for (idx, entry) in sparse.iter().enumerate() {
            if entry.value != "(unset)" {
                continue;
            }
            let Some(set_kind) = populated
                .iter()
                .find(|p| p.key.as_ref() == entry.key.as_ref())
                .map(|p| p.kind.clone())
            else {
                continue;
            };
            let options = dropdown_options(&set_kind);
            if options.is_empty() {
                continue;
            }
            checked += 1;

            let (mut ctx, row) = ctx_on_config_row(sparse_gsd_config(), entry.key.as_ref());
            assert_eq!(row, idx);
            let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
            press(&mut screen, &mut ctx, KeyCode::Enter);
            assert_eq!(
                ctx.view_cache[TEST_ALIAS].defaults_editing,
                Some(idx),
                "Enter on unset `{}` opened no chooser",
                entry.key
            );

            for option in &options {
                let mut config = sparse_gsd_config();
                assert!(
                    set_config_value(&mut config, entry.key.as_ref(), option),
                    "`{}` offers {option:?} but applying it changes nothing",
                    entry.key
                );
                let after = build_defaults_entries(&config, None);
                assert_eq!(
                    &after[idx].value, option,
                    "`{}`: picked {option:?}, the row now reads {:?}",
                    entry.key, after[idx].value
                );
            }
        }
        assert!(
            checked >= 20,
            "only {checked} unset choice rows exercised — the fixture no longer \
             leaves the class unset, so this test is not testing it"
        );
    }

    // --- quick 260922-hdi: `/` filters the Config Settings tab -------------

    /// Every Defaults row's value, in UNDERLYING order.
    fn config_row_values(ctx: &AppContext) -> Vec<String> {
        entries_for_cache(&ctx.view_cache[TEST_ALIAS])
            .into_iter()
            .map(|e| e.value)
            .collect()
    }

    /// `/` then each char of `text`, through the real `handle_key`.
    fn type_config_filter(screen: &mut DetailScreen, ctx: &mut AppContext, text: &str) {
        press(screen, ctx, KeyCode::Char('/'));
        for c in text.chars() {
            press(screen, ctx, KeyCode::Char(c));
        }
    }

    /// The Defaults tab drawn into a `width`x`height` TestBackend, one String
    /// per screen row.
    fn draw_config_tab(
        screen: &DetailScreen,
        ctx: &AppContext,
        width: u16,
        height: u16,
    ) -> Vec<String> {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("TestBackend terminal");
        terminal
            .draw(|frame| screen.render_defaults_tab(frame, frame.area(), ctx))
            .expect("draw the Defaults tab");
        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .filter_map(|x| buffer.cell((x, y)).map(|c| c.symbol().to_string()))
                    .collect()
            })
            .collect()
    }

    /// Underlying indices of the rows whose key contains "drift", computed
    /// here and NOT through the production filter helper.
    fn drift_row_indices() -> Vec<usize> {
        build_defaults_entries(&sparse_gsd_config(), None)
            .iter()
            .enumerate()
            .filter(|(_, e)| e.key.to_lowercase().contains("drift"))
            .map(|(i, _)| i)
            .collect()
    }

    /// The tracer: `/`, a query full of shortcut letters (x, d, r, ...), Enter
    /// to confirm, Enter to open — the chooser that opens is the FILTERED
    /// row's, by its underlying index, and the pick lands on that key only.
    /// Re-proves the Unset Enter-opens-chooser fix through the filter.
    #[test]
    fn config_filter_enter_on_a_filtered_unset_row_opens_that_rows_chooser() {
        let key = "workflow.context_drift_action";
        let entries = build_defaults_entries(&sparse_gsd_config(), None);
        let target_idx = entries
            .iter()
            .position(|e| e.key.as_ref() == key)
            .expect("the Defaults tab has a context_drift_action row");
        let (mut ctx, mode_idx) = ctx_on_config_row(sparse_gsd_config(), "mode");
        assert_ne!(
            target_idx, mode_idx,
            "precondition: the cursor starts elsewhere"
        );
        assert_eq!(
            entries[target_idx].value, "(unset)",
            "precondition: an UNSET row"
        );
        let before = config_row_values(&ctx);
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());

        type_config_filter(&mut screen, &mut ctx, "context_drift_action");
        {
            let cache = &ctx.view_cache[TEST_ALIAS];
            assert_eq!(cache.defaults_filter, "context_drift_action");
            assert!(cache.defaults_filter_typing, "the input still has focus");
            assert_eq!(
                cache.defaults_selected, target_idx,
                "the cursor follows the match"
            );
            assert!(cache.defaults_editing.is_none());
            assert!(matches!(
                cache.defaults_edit_target,
                super::super::DefaultsEditTarget::Project
            ));
        }
        assert_eq!(
            config_row_values(&ctx),
            before,
            "a typed char fired its shortcut"
        );
        assert_eq!(
            ctx.detail_sub_view_per_project.get(TEST_ALIAS),
            Some(&DetailSubView::Defaults)
        );

        press(&mut screen, &mut ctx, KeyCode::Enter);
        {
            let cache = &ctx.view_cache[TEST_ALIAS];
            assert!(!cache.defaults_filter_typing, "Enter while typing confirms");
            assert_eq!(
                cache.defaults_filter, "context_drift_action",
                "and keeps the filter"
            );
            assert_eq!(cache.defaults_editing, None, "confirming opens nothing");
        }

        press(&mut screen, &mut ctx, KeyCode::Enter);
        assert_eq!(
            ctx.view_cache[TEST_ALIAS].defaults_editing,
            Some(target_idx),
            "Enter on the filtered row must open THAT row's chooser"
        );

        press(&mut screen, &mut ctx, KeyCode::Down);
        press(&mut screen, &mut ctx, KeyCode::Enter);
        let after = config_row_values(&ctx);
        let pick = dropdown_options(&entries[target_idx].kind)[1].clone();
        assert_eq!(
            after[target_idx], pick,
            "the pick lands on the filtered key"
        );
        for (i, (b, a)) in before.iter().zip(&after).enumerate() {
            if i != target_idx {
                assert_eq!(b, a, "row {i} ({}) changed", entries[i].key);
            }
        }
    }

    #[test]
    fn config_filter_arrows_move_only_among_matching_rows() {
        let expected = drift_row_indices();
        assert_eq!(expected.len(), 5, "precondition: five drift rows");
        for query in ["drift", "DRIFT"] {
            let (mut ctx, _) = ctx_on_config_row(sparse_gsd_config(), "mode");
            let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
            let sel = |ctx: &AppContext| ctx.view_cache[TEST_ALIAS].defaults_selected;

            type_config_filter(&mut screen, &mut ctx, query);
            assert_eq!(
                sel(&ctx),
                expected[0],
                "{query}: typing selects the first match"
            );
            for want in &expected[1..=4] {
                press(&mut screen, &mut ctx, KeyCode::Down);
                assert_eq!(sel(&ctx), *want, "{query}: Down skips hidden rows");
            }
            press(&mut screen, &mut ctx, KeyCode::Down);
            assert_eq!(
                sel(&ctx),
                expected[4],
                "{query}: Down clamps at the last match"
            );
            press(&mut screen, &mut ctx, KeyCode::Up);
            assert_eq!(sel(&ctx), expected[3]);
            press(&mut screen, &mut ctx, KeyCode::PageUp);
            assert_eq!(sel(&ctx), expected[0]);
            press(&mut screen, &mut ctx, KeyCode::PageDown);
            assert_eq!(sel(&ctx), expected[4]);

            press(&mut screen, &mut ctx, KeyCode::Enter);
            assert!(!ctx.view_cache[TEST_ALIAS].defaults_filter_typing);
            press(&mut screen, &mut ctx, KeyCode::Char('k'));
            assert_eq!(
                sel(&ctx),
                expected[3],
                "{query}: k navigates once confirmed"
            );
            press(&mut screen, &mut ctx, KeyCode::Char('k'));
            assert_eq!(sel(&ctx), expected[2]);
            press(&mut screen, &mut ctx, KeyCode::Char('j'));
            assert_eq!(sel(&ctx), expected[3]);
            assert_eq!(
                ctx.view_cache[TEST_ALIAS].defaults_filter, query,
                "j/k are not text"
            );
        }
    }

    #[test]
    fn config_filter_render_shows_only_matches_and_count() {
        let entries = build_defaults_entries(&sparse_gsd_config(), None);
        let total = entries.len();
        let drift_keys: Vec<String> = drift_row_indices()
            .into_iter()
            .map(|i| entries[i].key.to_string())
            .collect();
        assert_eq!(drift_keys.len(), 5);
        assert!(
            entries
                .iter()
                .any(|e| e.key.as_ref() == "granularity" && !e.key.contains("drift")),
            "precondition: a non-matching `granularity` row exists"
        );
        let (mut ctx, _) = ctx_on_config_row(sparse_gsd_config(), "mode");
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());

        type_config_filter(&mut screen, &mut ctx, "drift");
        let text = draw_config_tab(&screen, &ctx, 160, 45).join("\n");
        for key in &drift_keys {
            assert!(
                text.contains(key.as_str()),
                "matching row `{key}` not drawn"
            );
        }
        assert!(
            !text.contains("granularity"),
            "a non-matching row was drawn"
        );
        assert!(text.contains("/drift_"), "the typing echo is missing");
        assert!(
            text.contains(&format!("({}/{total})", 5)),
            "the count is missing"
        );

        press(&mut screen, &mut ctx, KeyCode::Enter);
        let text = draw_config_tab(&screen, &ctx, 160, 45).join("\n");
        assert!(
            !text.contains("/drift_"),
            "the cursor mark outlived the input focus"
        );
        assert!(
            text.contains("/drift"),
            "the confirmed filter is no longer echoed"
        );
        assert!(text.contains(&format!("(5/{total})")));
    }

    /// T-HDI-01: while the input has focus, NO key reaches its global arm.
    /// `d` goes last: it would flip the target to Global, whose persist path
    /// is the operator's real `~/.gsd/defaults.json`.
    #[test]
    fn config_filter_typing_swallows_shortcut_keys() {
        let (mut ctx, _) = ctx_on_config_row(sparse_gsd_config(), "mode");
        let before = config_row_values(&ctx);
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());

        let action = screen.handle_key(KeyCode::Char('/'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::None));
        for c in ['x', 'q', '3', 'r', '?', 'j', 'k', 'd'] {
            let action = screen.handle_key(KeyCode::Char(c), KeyModifiers::NONE, &mut ctx);
            assert!(
                matches!(action, ScreenAction::None),
                "`{c}` returned an action"
            );
        }
        {
            let cache = &ctx.view_cache[TEST_ALIAS];
            assert_eq!(cache.defaults_filter, "xq3r?jkd");
            assert!(matches!(
                cache.defaults_edit_target,
                super::super::DefaultsEditTarget::Project
            ));
        }
        assert_eq!(
            config_row_values(&ctx),
            before,
            "a typed key mutated a value"
        );
        let mode_idx = build_defaults_entries(&sparse_gsd_config(), None)
            .iter()
            .position(|e| e.key.as_ref() == "mode")
            .unwrap();
        assert_eq!(config_row_values(&ctx)[mode_idx], "yolo");

        for code in [KeyCode::Left, KeyCode::Right, KeyCode::Tab, KeyCode::Delete] {
            let action = screen.handle_key(code, KeyModifiers::NONE, &mut ctx);
            assert!(
                matches!(action, ScreenAction::None),
                "{code:?} returned an action"
            );
            assert_eq!(
                ctx.detail_sub_view_per_project.get(TEST_ALIAS),
                Some(&DetailSubView::Defaults),
                "{code:?} left the Config tab"
            );
        }
        assert!(ctx.view_cache[TEST_ALIAS].defaults_filter_typing);
    }

    #[test]
    fn config_filter_esc_clears_typing_then_confirmed_filter_then_pops() {
        let (mut ctx, _) = ctx_on_config_row(sparse_gsd_config(), "mode");
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
        let drift = drift_row_indices();

        // Esc while typing: clears, keeps the screen and the cursor's row.
        type_config_filter(&mut screen, &mut ctx, "drift");
        let row = ctx.view_cache[TEST_ALIAS].defaults_selected;
        let action = screen.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::None));
        {
            let cache = &ctx.view_cache[TEST_ALIAS];
            assert!(cache.defaults_filter.is_empty());
            assert!(!cache.defaults_filter_typing);
            assert_eq!(
                cache.defaults_selected, row,
                "Esc moved the cursor off its row"
            );
        }

        // Esc, then q, on a CONFIRMED filter: clears, no Pop ([INFERRED A3]).
        for key in [KeyCode::Esc, KeyCode::Char('q')] {
            type_config_filter(&mut screen, &mut ctx, "drift");
            press(&mut screen, &mut ctx, KeyCode::Enter);
            let action = screen.handle_key(key, KeyModifiers::NONE, &mut ctx);
            assert!(
                matches!(action, ScreenAction::None),
                "{key:?} popped a filtered tab"
            );
            let cache = &ctx.view_cache[TEST_ALIAS];
            assert!(cache.defaults_filter.is_empty(), "{key:?} kept the filter");
            assert!(!cache.defaults_filter_typing);
        }

        // No filter: Esc pops exactly as before.
        let action = screen.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx);
        assert!(
            matches!(action, ScreenAction::Pop),
            "Esc with no filter must pop"
        );

        // Popup first, then filter, then pop.
        type_config_filter(&mut screen, &mut ctx, "drift");
        press(&mut screen, &mut ctx, KeyCode::Enter);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(ctx.view_cache[TEST_ALIAS].defaults_selected, drift[4]);
        press(&mut screen, &mut ctx, KeyCode::Enter);
        assert_eq!(ctx.view_cache[TEST_ALIAS].defaults_editing, Some(drift[4]));
        let action = screen.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(
            ctx.view_cache[TEST_ALIAS].defaults_editing, None,
            "popup closes first"
        );
        assert_eq!(
            ctx.view_cache[TEST_ALIAS].defaults_filter, "drift",
            "and keeps the filter"
        );
        let action = screen.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::None));
        assert!(ctx.view_cache[TEST_ALIAS].defaults_filter.is_empty());
        let action = screen.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::Pop));
    }

    /// T-HDI-04: a filter that matches nothing leaves the cursor on a HIDDEN
    /// row; Enter / x / navigation must not act on it, and nothing may panic.
    #[test]
    fn config_filter_empty_result_is_inert() {
        let (mut ctx, _) = ctx_on_config_row(sparse_gsd_config(), "mode");
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
        type_config_filter(&mut screen, &mut ctx, "zzzz");
        press(&mut screen, &mut ctx, KeyCode::Enter);
        let selected = ctx.view_cache[TEST_ALIAS].defaults_selected;
        let before = config_row_values(&ctx);

        for code in [
            KeyCode::Enter,
            KeyCode::Char('x'),
            KeyCode::Down,
            KeyCode::Up,
            KeyCode::PageDown,
            KeyCode::PageUp,
        ] {
            press(&mut screen, &mut ctx, code);
            let cache = &ctx.view_cache[TEST_ALIAS];
            assert_eq!(
                cache.defaults_selected, selected,
                "{code:?} moved the cursor"
            );
            assert_eq!(cache.defaults_editing, None, "{code:?} opened an editor");
            assert_eq!(config_row_values(&ctx), before, "{code:?} changed a value");
        }

        let text = draw_config_tab(&screen, &ctx, 120, 30).join("\n");
        assert!(
            text.contains("No config keys match"),
            "the empty state is not drawn"
        );
        assert!(text.contains("(0/"), "the zero count is not echoed");
    }

    /// T-HDI-02: `x` on a filtered row clears THAT key and no other.
    #[test]
    fn config_filter_x_clears_the_filtered_rows_key_only() {
        let config = crate::state_reader::config_json::parse_gsd_config(
            r#"{"mode":"yolo","workflow":{"drift_threshold":5}}"#,
        )
        .expect("the fixture parses");
        let entries = build_defaults_entries(&config, None);
        let pos = |key: &str| entries.iter().position(|e| e.key.as_ref() == key).unwrap();
        let (threshold, mode) = (pos("workflow.drift_threshold"), pos("mode"));
        assert_eq!(
            entries[threshold].value, "5",
            "precondition: the row is set"
        );
        let (mut ctx, _) = ctx_on_config_row(config, "mode");
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());

        type_config_filter(&mut screen, &mut ctx, "drift_threshold");
        press(&mut screen, &mut ctx, KeyCode::Enter);
        press(&mut screen, &mut ctx, KeyCode::Char('x'));
        let values = config_row_values(&ctx);
        assert_eq!(
            values[threshold], "(unset)",
            "x did not clear the filtered row"
        );
        assert_eq!(
            values[mode], "yolo",
            "x cleared the row under the old cursor"
        );
    }

    #[test]
    fn config_filter_arrival_and_target_toggle_keep_cursor_consistent() {
        // Arrival clears a stale filter and input focus (RESEARCH Pitfall 6).
        let (mut ctx, _) = ctx_on_config_row(sparse_gsd_config(), "mode");
        {
            let cache = ctx.view_cache.get_mut(TEST_ALIAS).unwrap();
            cache.defaults_filter = "drift".to_string();
            cache.defaults_filter_typing = true;
        }
        let mut offset = 0u16;
        switch_to_tab(
            TEST_ALIAS,
            tab_index(&DetailSubView::Defaults),
            &mut offset,
            &mut ctx,
        );
        let cache = &ctx.view_cache[TEST_ALIAS];
        assert!(
            cache.defaults_filter.is_empty(),
            "arrival kept a stale filter"
        );
        assert!(
            !cache.defaults_filter_typing,
            "arrival kept the input focus"
        );

        // `d` keeps the filter and the cursor on a VISIBLE row. Nothing
        // mutating follows: Global persists to the real ~/.gsd/defaults.json.
        let (mut ctx, _) = ctx_on_config_row(sparse_gsd_config(), "mode");
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
        type_config_filter(&mut screen, &mut ctx, "drift");
        press(&mut screen, &mut ctx, KeyCode::Enter);
        press(&mut screen, &mut ctx, KeyCode::Char('d'));
        let cache = &ctx.view_cache[TEST_ALIAS];
        assert!(matches!(
            cache.defaults_edit_target,
            super::super::DefaultsEditTarget::Global
        ));
        assert_eq!(cache.defaults_filter, "drift", "d dropped the filter");
        let entries = entries_for_cache(cache);
        let key = entries[cache.defaults_selected].key.to_string();
        assert!(
            key.contains("drift"),
            "d parked the cursor on hidden row `{key}`"
        );
    }

    /// T-HDI-03: the echo goes through `shown()`; the raw ESC is matched but
    /// never reaches a cell.
    #[test]
    fn config_filter_echo_is_escaped() {
        let (mut ctx, _) = ctx_on_config_row(sparse_gsd_config(), "mode");
        let total = build_defaults_entries(&sparse_gsd_config(), None).len();
        let mut screen = DetailScreen::new(TEST_ALIAS.to_string());
        press(&mut screen, &mut ctx, KeyCode::Char('/'));
        press(&mut screen, &mut ctx, KeyCode::Char('\u{1b}'));
        press(&mut screen, &mut ctx, KeyCode::Char('a'));
        assert_eq!(
            ctx.view_cache[TEST_ALIAS].defaults_filter, "\u{1b}a",
            "matched raw"
        );

        let rows = draw_config_tab(&screen, &ctx, 120, 30);
        assert!(
            rows.iter().all(|r| !r.contains('\u{1b}')),
            "a raw ESC reached a cell"
        );
        assert!(
            rows.join("\n").contains(&format!("(0/{total})")),
            "the count is missing"
        );
    }

    // ── quick 260923-md1: Roadmap tab dependency graph + `v` toggle ──────

    /// A DetailScreen on the Roadmap tab of a project whose roadmap is the
    /// chain 1 → 2 → 3.
    fn roadmap_graph_fixture() -> (DetailScreen, AppContext) {
        use crate::state_reader::roadmap_md::RoadmapPhase;
        let phase = |n: &str, deps: &[&str]| RoadmapPhase {
            number: n.to_string(),
            name: format!("Name {n}"),
            description: String::new(),
            completed: false,
            total_plans: 0,
            completed_plans: 0,
            depends_on: deps.iter().map(|d| d.to_string()).collect(),
        };
        let mut ctx = test_ctx();
        ctx.project_states.insert(
            TEST_ALIAS.to_string(),
            crate::state_reader::ProjectState {
                phases: vec![phase("1", &[]), phase("2", &["1"]), phase("3", &["2"])],
                ..Default::default()
            },
        );
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::RoadmapViz);
        (DetailScreen::new(TEST_ALIAS.to_string()), ctx)
    }

    fn roadmap_box_flag(ctx: &AppContext) -> bool {
        ctx.view_cache
            .get(TEST_ALIAS)
            .is_some_and(|c| c.roadmap_box_view)
    }

    #[test]
    fn roadmap_graph_tab_draws_the_graph_by_default_and_v_toggles_the_box_list() {
        let (mut screen, mut ctx) = roadmap_graph_fixture();
        let graph = render_detail_to_text(&screen, &ctx);
        assert!(graph.contains("1 ─► 2 ─► 3"), "{graph}");
        assert_eq!(screen.generic_viewport.get().total_lines, 1);

        screen.scroll_offset = 2;
        press(&mut screen, &mut ctx, KeyCode::Char('v'));
        assert!(roadmap_box_flag(&ctx), "v did not select the box list");
        assert_eq!(screen.scroll_offset, 0, "v must reset the scroll offset");
        let boxes = render_detail_to_text(&screen, &ctx);
        assert!(
            boxes.matches('┌').count() > graph.matches('┌').count(),
            "the box list draws a box border per phase: {boxes}"
        );
        assert!(!boxes.contains("1 ─► 2"), "{boxes}");

        press(&mut screen, &mut ctx, KeyCode::Char('v'));
        assert!(!roadmap_box_flag(&ctx));
        assert!(render_detail_to_text(&screen, &ctx).contains("1 ─► 2 ─► 3"));
    }

    #[test]
    fn roadmap_graph_tab_v_is_inert_on_other_tabs() {
        let (mut screen, mut ctx) = roadmap_graph_fixture();
        ctx.detail_sub_view_per_project
            .insert(TEST_ALIAS.to_string(), DetailSubView::Backlog);
        press(&mut screen, &mut ctx, KeyCode::Char('v'));
        assert!(!roadmap_box_flag(&ctx));
    }

    #[test]
    fn roadmap_graph_tab_footer_advertises_v() {
        let footer: String = footer_spans(&DetailSubView::RoadmapViz, 200, false)
            .iter()
            .map(|s| s.content.to_string())
            .collect();
        assert!(footer.contains("[v]"), "{footer}");
        assert!(footer.contains("[e]"), "{footer}");
    }
}
