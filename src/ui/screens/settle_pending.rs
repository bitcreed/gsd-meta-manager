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
use super::owner_block::{blanks, cells, scope_label, GroupKey, RowSrc};
use super::AppContext;
use crate::state_reader::owner_items::OwnerDigest;
use crate::ui::glyphs;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::collections::{HashMap, HashSet};
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

    /// Whether `digest` still lists one of the settled `(id, name)` checks in a group
    /// of this settle's lane, scope, workstream and phase.
    fn still_listed(&self, digest: &OwnerDigest) -> bool {
        digest.groups.iter().any(|group| {
            group.lane.as_str() == self.lane
                && scope_label(&group.scope) == self.scope_label
                && group.workstream == self.workstream
                && group.phase == self.phase
                && group.items.iter().any(|item| {
                    self.checks
                        .iter()
                        .any(|(id, name)| *id == item.id && *name == item.name)
                })
        })
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

    /// A reparse of `alias` landed with `digest` (I-1). For every settle of the
    /// alias that is not done: `Launched` becomes `Recorded` when a source file's
    /// fingerprint differs from the one taken before the launch (a file that was
    /// readable and no longer is counts as changed); when the settled `(id, name)`
    /// checks are no longer listed, a `Recorded` settle becomes `Done` and one that
    /// never saw a file change is dropped without claiming anything. Returns whether
    /// anything changed.
    pub(crate) fn refresh(&mut self, alias: &str, digest: &OwnerDigest, now: Instant) -> bool {
        let mut changed = false;
        self.entries.retain_mut(|entry| {
            if entry.alias != alias || entry.is_done() {
                return true;
            }
            if entry.stage == Stage::Launched {
                let seen: Vec<Fingerprint> = entry
                    .files
                    .iter()
                    .map(|file| fingerprint(&entry.root, file))
                    .collect();
                if seen != entry.fingerprints {
                    entry.stage = Stage::Recorded;
                    changed = true;
                }
            }
            if entry.still_listed(digest) {
                return true;
            }
            changed = true;
            if entry.stage == Stage::Recorded {
                entry.stage = Stage::Done { since: now };
                true
            } else {
                false
            }
        });
        changed
    }

    /// Drop the done settles whose [`LINGER`] is over. Returns whether any went.
    pub(crate) fn expire(&mut self, now: Instant) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| match entry.stage {
            Stage::Done { since } => now.saturating_duration_since(since) < LINGER,
            _ => true,
        });
        self.entries.len() != before
    }

    /// The inbox rows of `alias` that are done and no longer in `live`, each with
    /// the settle that finished it: the lingering done rows, newest first, one
    /// per row identity (I-8).
    pub(crate) fn lingering_inbox(
        &self,
        alias: &str,
        live: &HashSet<GroupKey>,
    ) -> Vec<(&RowSrc, &PendingSettle)> {
        self.lingering(alias, live, |entry| &entry.inbox_rows)
    }

    /// [`lingering_inbox`](Self::lingering_inbox) for the Owner block's rows.
    pub(crate) fn lingering_block(
        &self,
        alias: &str,
        live: &HashSet<GroupKey>,
    ) -> Vec<(&RowSrc, &PendingSettle)> {
        self.lingering(alias, live, |entry| &entry.block_rows)
    }

    fn lingering<'a>(
        &'a self,
        alias: &str,
        live: &HashSet<GroupKey>,
        rows: impl Fn(&'a PendingSettle) -> &'a [RowSrc],
    ) -> Vec<(&'a RowSrc, &'a PendingSettle)> {
        let mut seen: HashSet<&GroupKey> = HashSet::new();
        let mut out = Vec::new();
        for entry in self.entries.iter().rev() {
            if entry.alias != alias || !entry.is_done() {
                continue;
            }
            for src in rows(entry) {
                if !live.contains(&src.key) && seen.insert(&src.key) {
                    out.push((src, entry));
                }
            }
        }
        out
    }

    /// Whether any settle of `alias` is done: the cheap gate before a lingering
    /// row's live keys are computed.
    pub(crate) fn has_done(&self, alias: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.alias == alias && entry.is_done())
    }

    #[cfg(test)]
    pub(crate) fn entries(&self) -> &[PendingSettle] {
        &self.entries
    }

    #[cfg(test)]
    pub(crate) fn entries_mut(&mut self) -> &mut [PendingSettle] {
        &mut self.entries
    }
}

/// How long the `done` line shows before the row goes (I-2). The tick that drops it
/// runs every 250 ms, so the line is visible for 2.0 to 2.25 s.
pub(crate) const LINGER: Duration = Duration::from_secs(2);

/// A reparse of `alias` landed: step its settles (see [`PendingSettles::refresh`]).
/// Only evaluated while the alias holds a digest, so an unreadable one keeps every
/// entry. Returns whether anything changed (the caller redraws).
pub(crate) fn on_refresh(ctx: &mut AppContext, alias: &str, now: Instant) -> bool {
    if ctx.owner_state_unavailable(alias) {
        return false;
    }
    match ctx.owner_digests.get(alias) {
        Some(digest) => ctx.settle_pending.refresh(alias, digest, now),
        None => false,
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
    use crate::state_reader::gsd_install::InstallRoots;
    use crate::state_reader::owner_items::read_owner_digest;
    use crate::state_reader::parse_project_state_with;
    use crate::ui::screens::inbox::InboxScreen;
    use crate::ui::screens::owner_settle;
    use crate::ui::screens::owner_settle::take_settle_launches;
    use crate::ui::screens::owner_settle::test_support::{fixture_ctx, temp_fixture};
    use crate::ui::screens::{AppContext, Screen, ScreenAction};
    use crossterm::event::{KeyCode, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    const ALIAS: &str = "alder-api";
    const UAT: &str = ".planning/phases/01-alpha/01-UAT.md";

    /// Re-read the project's state and digest exactly as `fixture_ctx` does.
    fn reread(ctx: &mut AppContext, root: &Path) {
        let planning = root.join(".planning");
        let state = parse_project_state_with(&planning, &InstallRoots::default());
        let now = chrono::DateTime::parse_from_rfc3339("2026-01-12T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        let digest = read_owner_digest(&planning, &state, now, 30);
        ctx.project_states.insert(ALIAS.to_owned(), state);
        ctx.owner_digests.insert(ALIAS.to_owned(), digest);
    }

    /// Record `tests` (by number) of the fixture's UAT file as skipped, the way
    /// GSD writes a Deferred follow-up.
    fn record(root: &Path, tests: &[&str]) {
        let path = root.join(UAT);
        let mut text = std::fs::read_to_string(&path).unwrap();
        for test in tests {
            let (head, old) = match *test {
                "2" => ("### 2. Saving a note\n", "result: pending\n"),
                _ => ("### 3. Deleting a note\n", "result: [pending]\n"),
            };
            let at = text.find(head).unwrap();
            let rel = text[at..].find(old).unwrap();
            text.replace_range(
                at + rel..at + rel + old.len(),
                "result: skipped\nreason: Deferred follow-up: will not do.\n",
            );
        }
        std::fs::write(&path, text).unwrap();
    }

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

        /// The style of the first cell of `needle` on the first line that has it.
        fn style_of(&self, ctx: &AppContext, needle: &str) -> Style {
            let (width, height) = (100, 30);
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|f| self.0.last().unwrap().render(f, f.area(), ctx))
                .unwrap();
            let buffer = terminal.backend().buffer();
            for y in 0..height {
                let line: String = (0..width).map(|x| buffer[(x, y)].symbol()).collect();
                if let Some(at) = line.find(needle) {
                    let x = line[..at].chars().count();
                    return buffer[(u16::try_from(x).unwrap(), y)].style();
                }
            }
            panic!("no line has {needle:?}");
        }

        fn text(&self, ctx: &AppContext, width: u16) -> String {
            self.render(ctx, width, 30).join("\n")
        }

        /// The popup flow of the tracer: `x w <reason> Enter`.
        fn open_popup(&mut self, ctx: &mut AppContext) {
            self.key(ctx, KeyCode::Char('x'));
            self.key(ctx, KeyCode::Char('w'));
            self.keys(ctx, "no device here");
            self.key(ctx, KeyCode::Enter);
        }
    }

    /// The phase row of `uat-pending` settled as Won't do from the inbox.
    fn settled_in_inbox(root: &Path) -> (AppContext, Stack) {
        take_settle_launches();
        let mut ctx = fixture_ctx(root, ALIAS);
        let mut stack = Stack(vec![Box::new(InboxScreen::new())]);
        stack.open_popup(&mut ctx);
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(ctx.settle_pending.len(), 1);
        (ctx, stack)
    }

    fn at(ctx: &AppContext, secs: u64) -> Instant {
        ctx.settle_pending.entries()[0].started + Duration::from_secs(secs)
    }

    /// [`on_refresh`] `secs` after Yes.
    fn refresh_at(ctx: &mut AppContext, alias: &str, secs: u64) -> bool {
        let now = at(ctx, secs);
        on_refresh(ctx, alias, now)
    }

    fn stage(ctx: &AppContext) -> Stage {
        ctx.settle_pending.entries()[0].stage
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

    #[test]
    fn a_refresh_without_a_file_change_keeps_counting() {
        let (_dir, root) = temp_fixture("uat-pending");
        let (mut ctx, stack) = settled_in_inbox(&root);
        reread(&mut ctx, &root);
        assert!(!refresh_at(&mut ctx, ALIAS, 5));
        assert_eq!(stage(&ctx), Stage::Launched);
        // An hour later the badge still counts: no timeout, no stall state.
        assert!(!refresh_at(&mut ctx, ALIAS, 3600));
        assert_eq!(stage(&ctx), Stage::Launched);
        assert!(!ctx.settle_pending.expire(at(&ctx, 3600)));
        let text = stack.text(&ctx, 100);
        assert!(text.contains("won't do \u{b7} launched"), "{text}");
    }

    #[test]
    fn a_changed_file_steps_the_badge_to_recorded() {
        let (_dir, root) = temp_fixture("uat-pending");
        let (mut ctx, stack) = settled_in_inbox(&root);
        record(&root, &["2"]);
        reread(&mut ctx, &root);
        assert!(refresh_at(&mut ctx, ALIAS, 8));
        assert_eq!(stage(&ctx), Stage::Recorded);
        let text = stack.text(&ctx, 100);
        assert!(
            text.contains("settling\u{2026} won't do \u{b7} recorded"),
            "{text}"
        );
    }

    /// Both checks recorded: the row leaves the digest; the done line lingers, is
    /// drawn only, and the cursor never rests on it.
    #[test]
    fn a_finished_settle_lingers_as_a_draw_only_row() {
        let (_dir, root) = temp_fixture("uat-pending");
        let (mut ctx, mut stack) = settled_in_inbox(&root);
        record(&root, &["2", "3"]);
        reread(&mut ctx, &root);
        assert!(refresh_at(&mut ctx, ALIAS, 9));
        assert!(
            matches!(stage(&ctx), Stage::Done { .. }),
            "one write: Launched -> Done"
        );
        assert!(
            crate::ui::screens::inbox::stop_rows(&ctx)
                .iter()
                .all(|row| row.src.group.phase != "01"),
            "the settled row is gone from the stops"
        );
        let lines = stack.render(&ctx, 100, 30);
        let row = lines
            .iter()
            .find(|line| line.contains("\u{2713}"))
            .unwrap_or_else(|| panic!("no done row:\n{}", lines.join("\n")));
        assert!(row.contains(ALIAS) && row.contains("ph 01"), "{row}");
        assert!(row.contains("\u{2713} phase 01 skipped"), "{row}");
        assert!(
            lines.iter().any(|line| line.contains("TEST")),
            "the section title comes with the lone lingering row"
        );
        // The row is DIM; the check mark is Green and its words are not DIM.
        assert!(stack
            .style_of(&ctx, "ph 01")
            .add_modifier
            .contains(Modifier::DIM));
        assert_eq!(stack.style_of(&ctx, "\u{2713}").fg, Some(Color::Green));
        assert!(!stack
            .style_of(&ctx, "phase 01 skipped")
            .add_modifier
            .contains(Modifier::DIM));
        // No key reaches it: the cursor has no row to rest on, and x refuses.
        for code in [
            KeyCode::Char('j'),
            KeyCode::Char('k'),
            KeyCode::Down,
            KeyCode::Up,
        ] {
            stack.key(&mut ctx, code);
        }
        stack.key(&mut ctx, KeyCode::Char('x'));
        assert_eq!(stack.0.len(), 1, "x on a lingering row opens nothing");
        // It lingers 2 s and then it is gone.
        let since = match stage(&ctx) {
            Stage::Done { since } => since,
            other => panic!("{other:?}"),
        };
        assert!(!ctx
            .settle_pending
            .expire(since + Duration::from_millis(1900)));
        assert!(stack.text(&ctx, 100).contains("\u{2713} phase 01 skipped"));
        assert!(ctx.settle_pending.expire(since + LINGER));
        assert!(ctx.settle_pending.is_empty());
        assert!(!stack.text(&ctx, 100).contains("\u{2713}"));
    }

    #[test]
    fn settled_checks_gone_without_a_file_change_claim_nothing() {
        let (_dir, root) = temp_fixture("uat-pending");
        let (mut ctx, _stack) = settled_in_inbox(&root);
        ctx.owner_digests.get_mut(ALIAS).unwrap().groups.clear();
        assert!(refresh_at(&mut ctx, ALIAS, 3));
        assert!(
            ctx.settle_pending.is_empty(),
            "dropped silently, no check mark"
        );
    }

    #[test]
    fn a_refresh_of_another_alias_or_without_a_digest_changes_nothing() {
        let (_dir, root) = temp_fixture("uat-pending");
        let (mut ctx, _stack) = settled_in_inbox(&root);
        record(&root, &["2", "3"]);
        reread(&mut ctx, &root);
        assert!(!refresh_at(&mut ctx, "other", 3));
        assert_eq!(stage(&ctx), Stage::Launched);
        ctx.owner_digests.remove(ALIAS);
        assert!(!refresh_at(&mut ctx, ALIAS, 3));
        assert_eq!(
            stage(&ctx),
            Stage::Launched,
            "an unreadable digest keeps the entry"
        );
    }

    #[test]
    fn a_vanished_source_file_counts_as_changed() {
        let (_dir, root) = temp_fixture("uat-pending");
        let (mut ctx, _stack) = settled_in_inbox(&root);
        std::fs::remove_file(root.join(UAT)).unwrap();
        reread(&mut ctx, &root);
        assert!(refresh_at(&mut ctx, ALIAS, 3));
        assert!(
            matches!(stage(&ctx), Stage::Done { .. }),
            "{:?}",
            stage(&ctx)
        );
    }

    #[test]
    fn a_fingerprint_never_leaves_planning() {
        let (dir, root) = temp_fixture("uat-pending");
        assert!(fingerprint(&root, UAT).is_some());
        assert_eq!(fingerprint(&root, ".planning/phases"), None, "a directory");
        assert_eq!(fingerprint(&root, ".planning/none.md"), None);
        std::fs::write(dir.path().join("outside.md"), "x").unwrap();
        assert_eq!(fingerprint(&root, "outside.md"), None);
        assert_eq!(fingerprint(&root, ".planning/../outside.md"), None);
    }

    fn session_in(root: &Path) -> crate::session_detector::ClaudeSession {
        crate::session_detector::ClaudeSession {
            pid: 4242,
            kind: crate::session_detector::SessionKind::Claude,
            session_id: None,
            working_dir: root.to_path_buf(),
            start_time: None,
            tty: None,
        }
    }

    /// One check from the inbox: `Space`, `j` selects test 2, then the popup flow.
    #[test]
    fn a_single_check_badges_its_row_and_its_done_line_stays_on_the_live_row() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let mut stack = Stack(vec![Box::new(InboxScreen::new())]);
        stack.key(&mut ctx, KeyCode::Char(' '));
        stack.key(&mut ctx, KeyCode::Char('j'));
        assert_eq!(ctx.inbox_view.item_cursor.as_deref(), Some("01-UAT.md#2"));
        stack.open_popup(&mut ctx);
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(ctx.settle_pending.len(), 1);
        assert_eq!(ctx.settle_pending.entries()[0].words, "test 2 won't do");
        let text = stack.text(&ctx, 100);
        assert!(
            text.contains("settling\u{2026} test 2 won't do \u{b7} launched"),
            "{text}"
        );
        // Test 2 is recorded, test 3 stays open: the row stays and says so.
        record(&root, &["2"]);
        reread(&mut ctx, &root);
        assert!(refresh_at(&mut ctx, ALIAS, 4));
        assert!(matches!(stage(&ctx), Stage::Done { .. }));
        let text = stack.text(&ctx, 100);
        assert!(text.contains("\u{2713} test 2 skipped"), "{text}");
        assert!(
            crate::ui::screens::inbox::stop_rows(&ctx)
                .iter()
                .any(|row| row.src.group.phase == "01"),
            "the row is still a stop"
        );
        assert!(ctx.settle_pending.expire(at(&ctx, 4) + LINGER));
        assert!(!stack.text(&ctx, 100).contains("\u{2713}"));
    }

    /// A Move: the words and the done line name the target phase.
    #[test]
    fn a_move_names_its_target_phase() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let row = crate::ui::screens::inbox::stop_rows(&ctx).remove(0);
        let draft = owner_settle::resolve_draft(&ctx, ALIAS, &row.src.key, None).unwrap();
        let plan = owner_settle::SettlePlan {
            draft,
            choice: owner_settle::SettleChoice {
                outcome: owner_settle::Outcome::Move,
                reason: String::new(),
                move_to: Some(owner_settle::MoveTarget {
                    number: "02".to_owned(),
                    name: None,
                    known: false,
                }),
            },
        };
        owner_settle::launch_and_track(&mut ctx, &plan);
        assert_eq!(ctx.settle_pending.entries()[0].words, "move to phase 02");
        assert_eq!(
            ctx.settle_pending.entries()[0].done_words,
            "moved to phase 02"
        );
        let stack = Stack(vec![Box::new(InboxScreen::new())]);
        assert!(stack
            .text(&ctx, 100)
            .contains("settling\u{2026} move to phase 02 \u{b7} launched"));
        record(&root, &["2", "3"]);
        reread(&mut ctx, &root);
        assert!(refresh_at(&mut ctx, ALIAS, 6));
        assert!(stack.text(&ctx, 100).contains("\u{2713} moved to phase 02"));
    }

    /// I-10: a live session always registers, a new session only when it started.
    #[test]
    fn a_live_session_registers_and_a_failed_launch_does_not() {
        crate::ui::screens::owner_actions::force_clipboard(None);
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        ctx.active_sessions.push(session_in(&root));
        let mut stack = Stack(vec![Box::new(InboxScreen::new())]);
        stack.open_popup(&mut ctx);
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(stack.0.len(), 1, "the popup closed");
        assert_eq!(ctx.settle_pending.len(), 1, "the paste happens outside");
        let text = stack.text(&ctx, 100);
        assert!(
            text.contains("settling\u{2026} won't do \u{b7} launched"),
            "{text}"
        );

        let plan = {
            let row = crate::ui::screens::inbox::stop_rows(&ctx).remove(0);
            owner_settle::SettlePlan {
                draft: owner_settle::resolve_draft(&ctx, ALIAS, &row.src.key, None).unwrap(),
                choice: owner_settle::SettleChoice {
                    outcome: owner_settle::Outcome::WontDo,
                    reason: String::new(),
                    move_to: None,
                },
            }
        };
        let mut new_session = plan.clone();
        new_session.draft.live_session = None;
        for (status, started) in [
            (owner_settle::launched_status(&new_session), true),
            ("Launched via opened in gnome-terminal".to_owned(), true),
            ("Failed to launch: no such file".to_owned(), false),
            (
                "no terminal emulator found (set $TERMINAL)".to_owned(),
                false,
            ),
        ] {
            assert_eq!(
                owner_settle::launch_started(&new_session, &status),
                started,
                "{status}"
            );
        }
        assert!(owner_settle::launch_started(&plan, "Failed to focus"));
    }

    /// A new Yes on the same checks replaces the old entry.
    #[test]
    fn a_second_yes_on_the_same_checks_replaces_the_entry() {
        let (_dir, root) = temp_fixture("uat-pending");
        let (mut ctx, mut stack) = settled_in_inbox(&root);
        stack.open_popup(&mut ctx);
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(ctx.settle_pending.len(), 1);
    }

    /// The detail view on the Owner block, rendered at `width` x `height`.
    fn detail_stack(ctx: &mut AppContext, key: &GroupKey) -> Stack {
        let screen =
            crate::ui::screens::detail::DetailScreen::opened_on_owner(ALIAS.to_owned(), key, ctx);
        assert!(screen.owner_focused(), "the Owner block holds the keyboard");
        let stack = Stack(vec![Box::new(screen)]);
        stack.render(ctx, 100, 30);
        stack
    }

    fn uat_key(ctx: &AppContext) -> GroupKey {
        crate::ui::screens::inbox::stop_rows(ctx).remove(0).src.key
    }

    /// The line of `stack` that carries `needle`, panicking with the screen.
    fn line_with(stack: &Stack, ctx: &AppContext, w: u16, h: u16, needle: &str) -> String {
        let lines = stack.render(ctx, w, h);
        lines
            .iter()
            .find(|line| line.contains(needle))
            .cloned()
            .unwrap_or_else(|| panic!("no line with {needle:?}:\n{}", lines.join("\n")))
    }

    /// Yes from the Owner block: the row keeps its place, shows the badge in place
    /// of its phrase, age and command, and the cursor marking still covers it.
    #[test]
    fn yes_in_the_owner_block_badges_the_row_and_both_hosts_share_the_entry() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let mut ctx = fixture_ctx(&root, ALIAS);
        let key = uat_key(&ctx);
        let mut stack = detail_stack(&mut ctx, &key);
        stack.key(&mut ctx, KeyCode::Enter);
        stack.open_popup(&mut ctx);
        stack.key(&mut ctx, KeyCode::Char('y'));
        assert_eq!(take_settle_launches().len(), 1);
        assert_eq!(ctx.settle_pending.len(), 1);
        let row = line_with(&stack, &ctx, 100, 30, "settling");
        assert!(
            row.contains("settling\u{2026} won't do \u{b7} launched 0:0"),
            "{row}"
        );
        assert!(row.contains("TEST") && row.contains("ph 01"), "{row}");
        assert!(row.trim_start().starts_with('>'), "the cursor mark: {row}");
        // The same entry shows on the inbox row of the same phase.
        let inbox = Stack(vec![Box::new(InboxScreen::new())]);
        let row = line_with(&inbox, &ctx, 100, 30, "settling");
        assert!(row.contains(ALIAS) && row.contains("won't do"), "{row}");
        // And an entry registered from the inbox shows in the Owner block.
        let (_dir2, root2) = temp_fixture("uat-pending");
        let (mut ctx2, _inbox) = settled_in_inbox(&root2);
        let key2 = uat_key(&ctx2);
        let block = detail_stack(&mut ctx2, &key2);
        let row = line_with(&block, &ctx2, 100, 30, "settling");
        assert!(row.contains("won't do \u{b7} launched"), "{row}");
    }

    /// The done row of the Owner block: drawn only, outside the stops and the
    /// grid, the block keeps its height while it lingers and is as before after.
    #[test]
    fn a_finished_settle_lingers_in_the_owner_block_without_a_stop() {
        use crate::ui::screens::owner_block::{block_height, block_model, block_model_with};
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let (mut ctx, _inbox) = settled_in_inbox(&root);
        let view = crate::ui::screens::owner_block::OwnerBlockState::default();
        let live = |ctx: &AppContext, pending: bool| {
            let digest = ctx.owner_digest(ALIAS).unwrap();
            match pending {
                true => block_model_with(
                    ALIAS,
                    "v1",
                    digest,
                    &view,
                    100,
                    Some((&ctx.settle_pending, Instant::now())),
                ),
                false => block_model(ALIAS, "v1", digest, &view, 100),
            }
        };
        record(&root, &["2", "3"]);
        reread(&mut ctx, &root);
        assert!(refresh_at(&mut ctx, ALIAS, 5));
        let digest = ctx.owner_digest(ALIAS).unwrap();
        let skeleton = crate::ui::screens::owner_block::skeleton(digest, &view);
        let bare = live(&ctx, false);
        let with = live(&ctx, true);
        assert!(
            !bare.has_content || bare.body.len() < with.body.len(),
            "the bare block has no row"
        );
        assert_eq!(with.stops, skeleton.stops, "a lingering row is no stop");
        assert_eq!(with.stop_lines.len(), skeleton.stops.len());
        for line in with.stop_lines.iter().flatten() {
            assert!(
                !with.body[*line].to_string().contains('\u{2713}'),
                "no stop points at the lingering line"
            );
        }
        assert!(
            with.has_content,
            "the block keeps its height while it lingers"
        );
        assert!(block_height(&with, 30) > 0);
        let text: String = with.body.iter().map(|l| l.to_string()).collect();
        assert!(text.contains("\u{2713} phase 01 skipped"), "{text}");
        assert!(
            with.body
                .iter()
                .all(|line| !line.spans.iter().any(|s| s.content.contains(">"))),
            "no cursor mark on it"
        );
        // After the linger the block is exactly what it is without pending state.
        let since = match stage(&ctx) {
            Stage::Done { since } => since,
            other => panic!("{other:?}"),
        };
        assert!(ctx.settle_pending.expire(since + LINGER));
        let after = live(&ctx, true);
        assert_eq!(after.body.len(), bare.body.len());
        assert_eq!(block_height(&after, 30), block_height(&bare, 30));
    }

    /// 80x24: the pending row is one line with `settling...`, the stage and the
    /// timer, in the inbox and in the detail view; the done line fits as well.
    #[test]
    fn the_badge_fits_80_columns_in_both_hosts() {
        take_settle_launches();
        let (_dir, root) = temp_fixture("uat-pending");
        let (mut ctx, inbox) = settled_in_inbox(&root);
        let key = uat_key(&ctx);
        let detail = detail_stack(&mut ctx, &key);
        for (host, stack) in [("inbox", &inbox), ("detail", &detail)] {
            let row = line_with(stack, &ctx, 80, 24, "settling");
            eprintln!("{host} 80x24 pending: {row}");
            assert!(cells(&row) <= 80, "{row}");
            assert!(
                row.contains("launched") && row.contains(" 0:0"),
                "{host}: {row}"
            );
            assert_eq!(
                stack
                    .render(&ctx, 80, 24)
                    .iter()
                    .filter(|line| line.contains("settling"))
                    .count(),
                1,
                "{host}: one line, never wrapped"
            );
        }
        record(&root, &["2", "3"]);
        reread(&mut ctx, &root);
        assert!(refresh_at(&mut ctx, ALIAS, 5));
        for (host, stack) in [("inbox", &inbox), ("detail", &detail)] {
            let row = line_with(stack, &ctx, 80, 24, "\u{2713}");
            eprintln!("{host} 80x24 done: {row}");
            assert!(row.contains("\u{2713} phase 01 skipped"), "{host}: {row}");
        }
    }
}
