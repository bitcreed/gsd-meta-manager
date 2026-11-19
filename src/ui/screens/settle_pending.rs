//! What a row shows after Yes in the settle popup (quick 261118-v6l).
//!
//! The settle launches `/gsd-verify-work` in a session the app does not own, so
//! until the checks leave the list there is no sign that anything happens. This
//! module keeps the in-memory half of that sign: one [`PendingSettle`] per Yes, the
//! `settling…` badge it draws on the row, and the stage it steps through as the
//! source file changes on disk (`launched` -> `recorded` -> a `✓` line for about
//! two seconds -> gone).
//!
//! **The state is memory only.** It survives refreshes, screen switches and the
//! inbox's `i` reset, is pruned with removed projects and is gone after a restart.
//! **Nothing here writes a file or starts a process, and the render path reads no
//! file**: the only filesystem call is [`fingerprint`], a `stat` of a source file
//! taken when a reparse lands (and once before the launch). Like the other settle
//! modules it is covered by `tests/owner_settle_guard.rs`.
//!
//! There is no timeout and no stall state: a source file that never changes leaves
//! the badge counting.

use super::detail::{fit_cells, fit_spans, shown};
use super::owner::OWNER_SEP;
use super::owner_block::{blanks, cells, GroupKey, RowSrc};
use crate::ui::glyphs;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Where a settle stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Yes was pressed; no source file has changed since.
    Launched,
    /// A source file changed: GSD wrote.
    Recorded,
    /// The settled checks are gone from the digest; the `✓` line shows from `since`.
    Done { since: Instant },
}

impl Stage {
    fn word(self) -> &'static str {
        match self {
            Stage::Launched => "launched",
            Stage::Recorded | Stage::Done { .. } => "recorded",
        }
    }
}

/// A source file's modified time and length.
pub(crate) type Fingerprint = Option<(SystemTime, u64)>;

/// One settle after Yes.
#[derive(Clone)]
pub(crate) struct PendingSettle {
    pub(crate) alias: String,
    /// The project path the settle ran in.
    pub(crate) root: PathBuf,
    /// The row's lane word (`test`, `blocked`), its scope label, workstream and phase.
    pub(crate) lane: &'static str,
    pub(crate) scope_label: Option<String>,
    pub(crate) workstream: Option<String>,
    pub(crate) phase: String,
    /// The project-relative source files of the settled checks.
    pub(crate) files: Vec<String>,
    /// The settled checks as `(item id, item name)`: unnumbered items carry
    /// positional ids a refresh can reuse, so the name pairs with the id.
    pub(crate) checks: Vec<(String, String)>,
    /// What the badge says the settle is: `won't do`, `test 2 move to phase 02`.
    pub(crate) words: String,
    /// What the `✓` line says: `phase 01 skipped`, `moved to phase 02`.
    pub(crate) done_words: String,
    pub(crate) started: Instant,
    pub(crate) stage: Stage,
    /// The files' fingerprints taken before the launch, parallel to `files`.
    pub(crate) fingerprints: Vec<Fingerprint>,
    /// The inbox and Owner block rows the settle covered at Yes, kept to draw a
    /// lingering `✓` row after the digest has dropped them.
    pub(crate) inbox_rows: Vec<RowSrc>,
    pub(crate) block_rows: Vec<RowSrc>,
}

impl PendingSettle {
    /// Whether this settle speaks for `row` of `alias`: the same facts (alias, lane,
    /// scope, workstream, phase) and at least one source file in common, never an
    /// index or a whole [`GroupKey`].
    fn covers(&self, alias: &str, row: &RowSrc) -> bool {
        self.alias == alias
            && row.key.lane == self.lane
            && row.key.scope_label == self.scope_label
            && row.key.workstream == self.workstream
            && row.key.phase == self.phase
            && row.files.iter().any(|file| self.files.contains(file))
    }

    /// Keep the rows of `inbox` and `block` this settle covers: the snapshots a
    /// lingering done row is drawn from once the digest has dropped them.
    pub(crate) fn snapshot_rows(&mut self, inbox: Vec<RowSrc>, block: Vec<RowSrc>) {
        let keep = |rows: Vec<RowSrc>| -> Vec<RowSrc> {
            rows.into_iter()
                .filter(|row| self.covers(&self.alias, row))
                .collect()
        };
        let (inbox, block) = (keep(inbox), keep(block));
        self.inbox_rows = inbox;
        self.block_rows = block;
    }

    pub(crate) fn is_done(&self) -> bool {
        matches!(self.stage, Stage::Done { .. })
    }

    #[cfg(test)]
    pub(crate) fn for_alias_test(alias: &str) -> Self {
        Self {
            alias: alias.to_owned(),
            root: PathBuf::new(),
            lane: "test",
            scope_label: None,
            workstream: None,
            phase: "01".to_owned(),
            files: vec![".planning/phases/01-a/01-UAT.md".to_owned()],
            checks: vec![("2".to_owned(), "a check".to_owned())],
            words: "won't do".to_owned(),
            done_words: "phase 01 skipped".to_owned(),
            started: Instant::now(),
            stage: Stage::Launched,
            fingerprints: vec![None],
            inbox_rows: Vec::new(),
            block_rows: Vec::new(),
        }
    }
}

/// Every settle of this run, oldest first.
#[derive(Default)]
pub struct PendingSettles {
    entries: Vec<PendingSettle>,
}

impl PendingSettles {
    /// Register `entry`; an older entry of the same alias that settles any of the
    /// same `(id, name)` checks is replaced (a new Yes on the same target).
    pub(crate) fn insert(&mut self, entry: PendingSettle) {
        self.entries.retain(|old| {
            old.alias != entry.alias || !old.checks.iter().any(|c| entry.checks.contains(c))
        });
        self.entries.push(entry);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// The latest settle covering `row` of `alias`.
    pub(crate) fn for_row(&self, alias: &str, row: &RowSrc) -> Option<&PendingSettle> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.covers(alias, row))
    }

    /// Drop the settles of aliases that are no longer registered.
    pub(crate) fn retain_registered<V>(&mut self, registered: &HashMap<String, V>) {
        self.entries
            .retain(|entry| registered.contains_key(&entry.alias));
    }

    #[cfg(test)]
    pub(crate) fn entries(&self) -> &[PendingSettle] {
        &self.entries
    }
}

/// The modified time and length of the source file `rel` of the project at
/// `project_root`, or `None` when it cannot be seen. `rel` comes from a project
/// file, so it is untrusted: both `.planning/` and the target are canonicalized and
/// the target must lie inside the first and be a regular file (the containment of
/// `owner_actions::read_owner_file`). A `stat` only; the file is never opened.
pub(crate) fn fingerprint(project_root: &Path, rel: &str) -> Fingerprint {
    let planning = project_root.join(".planning").canonicalize().ok()?;
    let target = project_root.join(rel).canonicalize().ok()?;
    if !target.starts_with(&planning) {
        return None;
    }
    let meta = std::fs::metadata(&target).ok()?;
    if !meta.is_file() {
        return None;
    }
    Some((meta.modified().ok()?, meta.len()))
}

// ---------------------------------------------------------------------------
// Badge
// ---------------------------------------------------------------------------

/// The spinner frames: ASCII, so no glyph fallback is ever needed.
const FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
/// How long a frame shows.
const FRAME_MS: u128 = 250;

/// Whole minutes, a colon and two-digit seconds: `0:12`, `75:03`.
fn elapsed_text(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// The frame for `elapsed`: one step every [`FRAME_MS`].
fn spinner(elapsed: Duration) -> &'static str {
    let step = usize::try_from(elapsed.as_millis() / FRAME_MS % 4).unwrap_or(0);
    FRAMES[step]
}

fn dim() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// The settling rungs, longest first: with the outcome words, with the stage, and
/// the bare timer. Every string goes through [`shown`].
fn settling_rungs(entry: &PendingSettle, now: Instant) -> [String; 3] {
    let elapsed = now.saturating_duration_since(entry.started);
    let (spin, time, stage) = (spinner(elapsed), elapsed_text(elapsed), entry.stage.word());
    [
        shown(&format!(
            "{spin} settling\u{2026} {}{OWNER_SEP}{stage} {time}",
            entry.words
        )),
        shown(&format!("{spin} settling\u{2026} {stage} {time}")),
        shown(&format!("{spin} settling\u{2026} {time}")),
    ]
}

/// The `✓` line of a finished settle.
fn done_text(entry: &PendingSettle) -> String {
    shown(&format!("{} {}", glyphs::CHECK, entry.done_words))
}

/// The badge in `room` cells: the longest rung that fits, else the bare timer cut
/// with `fit_cells`; DIM while settling, the `✓` Green and the rest plain once done.
pub(crate) fn badge_spans(entry: &PendingSettle, now: Instant, room: usize) -> Vec<Span<'static>> {
    if entry.is_done() {
        let text = fit_cells(&done_text(entry), room);
        return match text.strip_prefix(glyphs::CHECK) {
            Some(rest) => vec![
                Span::styled(glyphs::CHECK, Style::default().fg(Color::Green)),
                Span::raw(rest.to_owned()),
            ],
            None => vec![Span::raw(text)],
        };
    }
    let rungs = settling_rungs(entry, now);
    let text = rungs
        .iter()
        .find(|rung| cells(rung) <= room)
        .cloned()
        .unwrap_or_else(|| fit_cells(&rungs[2], room));
    vec![Span::styled(text, dim())]
}

/// The cells the badge needs at the least: the bare timer, or the whole `✓` line.
fn badge_floor(entry: &PendingSettle, now: Instant) -> usize {
    if entry.is_done() {
        cells(&done_text(entry))
    } else {
        cells(&settling_rungs(entry, now)[2])
    }
}

/// `row` with the badge from column `at` to the row end (`width` cells): the
/// row's own phrase, age and command cells were cleared by the caller, so the row
/// is cut two cells before the badge and padded to it. If even the shortest badge
/// does not fit there, `at` moves left until it does. One line, never wraps.
pub(crate) fn badged_row(
    row: Line<'static>,
    at: usize,
    entry: &PendingSettle,
    now: Instant,
    width: usize,
) -> Line<'static> {
    let at = at.min(width.saturating_sub(badge_floor(entry, now)));
    let mut spans = if at >= 2 {
        fit_spans(row.spans, at - 2).spans
    } else {
        Vec::new()
    };
    let used: usize = spans.iter().map(Span::width).sum();
    spans.push(Span::raw(blanks(at.saturating_sub(used))));
    spans.extend(badge_spans(entry, now, width.saturating_sub(at)));
    fit_spans(spans, width)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::inbox::InboxScreen;
    use crate::ui::screens::owner_settle::take_settle_launches;
    use crate::ui::screens::owner_settle::test_support::{fixture_ctx, temp_fixture};
    use crate::ui::screens::{AppContext, Screen, ScreenAction};
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    const ALIAS: &str = "alder-api";

    fn text(spans: &[Span<'static>]) -> String {
        spans.iter().map(|s| s.content.as_ref()).collect()
    }

    /// The ladder at a synthetic `now` twelve seconds after Yes.
    #[test]
    fn the_badge_ladder_shortens_with_the_room() {
        let entry = PendingSettle::for_alias_test(ALIAS);
        let now = entry.started + Duration::from_secs(12);
        let rung = |room| text(&badge_spans(&entry, now, room));
        assert_eq!(rung(60), "| settling\u{2026} won't do \u{b7} launched 0:12");
        assert_eq!(rung(25), "| settling\u{2026} launched 0:12");
        assert_eq!(rung(16), "| settling\u{2026} 0:12");
        let cut = rung(10);
        assert_eq!(cells(&cut), 10, "{cut}");
        assert!(cut.starts_with("| settli"), "{cut}");
        // The stage word follows the stage.
        let mut recorded = entry.clone();
        recorded.stage = Stage::Recorded;
        assert_eq!(
            text(&badge_spans(&recorded, now, 60)),
            "| settling\u{2026} won't do \u{b7} recorded 0:12"
        );
        // Settling is DIM and never color-only.
        assert!(badge_spans(&entry, now, 60)[0]
            .style
            .add_modifier
            .contains(Modifier::DIM));
    }

    #[test]
    fn the_done_badge_is_a_green_check_and_words() {
        let mut entry = PendingSettle::for_alias_test(ALIAS);
        entry.stage = Stage::Done {
            since: entry.started,
        };
        let spans = badge_spans(&entry, entry.started, 40);
        assert_eq!(text(&spans), "\u{2713} phase 01 skipped");
        assert_eq!(spans[0].style.fg, Some(Color::Green));
        assert!(!spans[1].style.add_modifier.contains(Modifier::DIM));
    }

    #[test]
    fn the_spinner_cycles_every_250_ms_and_the_timer_reads_minutes() {
        let frames: Vec<&str> = (0..5)
            .map(|step| spinner(Duration::from_millis(step * 250)))
            .collect();
        assert_eq!(frames, ["|", "/", "-", "\\", "|"]);
        assert_eq!(spinner(Duration::from_millis(249)), "|");
        assert_eq!(elapsed_text(Duration::ZERO), "0:00");
        assert_eq!(elapsed_text(Duration::from_secs(75)), "1:15");
        assert_eq!(elapsed_text(Duration::from_secs(3600)), "60:00");
        assert_eq!(elapsed_text(Duration::from_secs(75 * 60 + 3)), "75:03");
    }

    #[test]
    fn a_badged_row_stays_one_line_and_moves_left_to_fit() {
        let entry = PendingSettle::for_alias_test(ALIAS);
        let now = entry.started + Duration::from_secs(5);
        let row = Line::from(vec![Span::raw("> r alder-api  ph 01  Alpha  2 checks")]);
        let wide = badged_row(row.clone(), 40, &entry, now, 100);
        let wide_text = text(&wide.spans);
        assert!(wide_text.starts_with("> r alder-api"), "{wide_text}");
        assert_eq!(wide_text.find('|'), Some(40), "{wide_text}");
        // At 44 cells with the badge asked for column 40, the bare timer still fits.
        let tight = badged_row(row, 40, &entry, now, 44);
        assert!(tight.width() <= 44, "{}", text(&tight.spans));
        assert!(text(&tight.spans).contains("settling\u{2026} 0:05"));
    }

    /// A small screen stack: `Push`, `Replace` and `Pop` as the App applies them.
    struct Stack(Vec<Box<dyn Screen>>);

    impl Stack {
        fn render(&self, ctx: &AppContext, width: u16, height: u16) -> Vec<String> {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|f| self.0.last().unwrap().render(f, f.area(), ctx))
                .unwrap();
            let buffer = terminal.backend().buffer();
            (0..height)
                .map(|y| {
                    (0..width)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                })
                .collect()
        }

        fn key(&mut self, ctx: &mut AppContext, code: KeyCode) {
            // A draw first: a popup refuses Yes until it has drawn unclipped.
            self.render(ctx, 100, 30);
            match self
                .0
                .last_mut()
                .unwrap()
                .handle_key(code, KeyModifiers::NONE, ctx)
            {
                ScreenAction::Push(screen) => self.0.push(screen),
                ScreenAction::Replace(screen) => {
                    self.0.pop();
                    self.0.push(screen);
                }
                ScreenAction::Pop | ScreenAction::PopWriteOsc52(_) => {
                    self.0.pop();
                }
                _ => {}
            }
        }

        fn keys(&mut self, ctx: &mut AppContext, text: &str) {
            for ch in text.chars() {
                self.key(ctx, KeyCode::Char(ch));
            }
        }
    }

    #[test]
    fn yes_in_the_inbox_leaves_a_settling_badge_on_the_row() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut stack = Stack(vec![Box::new(InboxScreen::new())]);
        stack.key(&mut ctx, KeyCode::Char('x'));
        stack.key(&mut ctx, KeyCode::Char('w'));
        stack.keys(&mut ctx, "no device here");
        stack.key(&mut ctx, KeyCode::Enter);
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(take_settle_launches().len(), 1, "one launch");
        assert_eq!(ctx.settle_pending.len(), 1, "one pending entry");
        let lines = stack.render(&ctx, 100, 30);
        let row = lines
            .iter()
            .find(|line| line.contains(ALIAS))
            .unwrap_or_else(|| panic!("no row line:\n{}", lines.join("\n")));
        assert!(row.contains("ph 01"), "{row}");
        assert!(
            row.contains("settling\u{2026} won't do \u{b7} launched 0:0"),
            "{row}"
        );
    }
}
