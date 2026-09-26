//! Mouse support (quick 260926-dyf).
//!
//! **Everything here is a pure function over rects recorded at render time
//! (D-04).** Screens record their clickable regions while they draw; the event
//! handler maps a click or a wheel event onto those rects with the functions
//! below. Nothing in this module reads a file, touches the terminal or looks at
//! the clock on its own — the caller hands in `Instant`s — which is what makes
//! the whole click -> target mapping unit-testable.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};
use std::time::{Duration, Instant};

/// One mouse input as a screen sees it.
///
/// `double` is decided by [`ClickTracker`] in `App`, never by a screen, so every
/// screen agrees on what a double-click is (I-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseInput {
    Click { column: u16, row: u16, double: bool },
    Wheel { column: u16, row: u16, down: bool },
}

/// A mouse event that survived the reader filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Routed {
    Click { column: u16, row: u16 },
    Wheel { column: u16, row: u16, down: bool },
}

/// The reader filter (I-5, threat T-dyf-01).
///
/// `EnableMouseCapture` also turns on any-motion tracking, so an idle pointer
/// crossing the window is a continuous event producer. Only a left press and
/// vertical wheel steps may enter the biased-first Action FIFO; motion, drag,
/// release, the other buttons and horizontal scroll give `None` and are dropped
/// in the reader. Key modifiers are ignored.
pub fn routed(event: &MouseEvent) -> Option<Routed> {
    let (column, row) = (event.column, event.row);
    match event.kind {
        MouseEventKind::Down(MouseButton::Left) => Some(Routed::Click { column, row }),
        MouseEventKind::ScrollDown => Some(Routed::Wheel { column, row, down: true }),
        MouseEventKind::ScrollUp => Some(Routed::Wheel { column, row, down: false }),
        _ => None,
    }
}

/// Two presses closer together than this are a double-click (I-6).
pub const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(500);

/// Double-click detection (I-6).
///
/// A second left press is a double when it arrives within
/// [`DOUBLE_CLICK_WINDOW`], on the same row and within one column of the first.
/// After a double the tracker forgets the click, so a third press starts over.
/// `App` resets it on every key press.
#[derive(Debug, Default, Clone)]
pub struct ClickTracker {
    last: Option<(Instant, u16, u16)>,
}

impl ClickTracker {
    /// Record a left press; `true` when it completes a double-click.
    pub fn register(&mut self, now: Instant, column: u16, row: u16) -> bool {
        let double = matches!(
            self.last,
            Some((at, c, r)) if r == row
                && c.abs_diff(column) <= 1
                && now.saturating_duration_since(at) <= DOUBLE_CLICK_WINDOW
        );
        self.last = if double { None } else { Some((now, column, row)) };
        double
    }

    /// Forget the previous press, so the next one is a single click.
    pub fn reset(&mut self) {
        self.last = None;
    }
}

/// A scrolled list's visible rows, recorded at render time.
///
/// `rect` covers the list's item rows (no border, no header), `offset` is the
/// index drawn in the first row and `len` the item count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListRegion {
    pub rect: Rect,
    pub offset: usize,
    pub len: usize,
}

impl ListRegion {
    /// The item index under (`column`, `row`), or `None` outside the rect or
    /// past the last item.
    pub fn row_at(&self, column: u16, row: u16) -> Option<usize> {
        if !self.rect.contains(Position::new(column, row)) {
            return None;
        }
        let index = self.offset + usize::from(row - self.rect.y);
        (index < self.len).then_some(index)
    }
}

/// The rects ratatui 0.30's `Tabs` gives each title in `area` (its first row).
///
/// It mirrors `Tabs::render_tabs`: x starts at `area.x`; entry i occupies
/// `entry_widths[i]` cells clipped at `area.right()`; x then advances past the
/// entry and past `divider_cells` between entries (not after the last). An entry
/// that starts at or past the right edge gets width 0. The detail view's
/// `mouse_tab_rects_match_the_rendered_tab_bar` test pins this against the real
/// widget's buffer.
pub fn tab_entry_rects(area: Rect, entry_widths: &[u16], divider_cells: u16) -> Vec<Rect> {
    let right = area.right();
    let mut x = area.x;
    let mut rects = Vec::with_capacity(entry_widths.len());
    for (i, &w) in entry_widths.iter().enumerate() {
        let width = if x >= right { 0 } else { w.min(right - x) };
        rects.push(Rect::new(x.min(right), area.y, width, 1));
        x = x.saturating_add(w);
        if i + 1 < entry_widths.len() {
            x = x.saturating_add(divider_cells);
        }
    }
    rects
}

/// The status message `M` shows (I-3). One function owns both strings.
pub fn mouse_status_text(on: bool) -> &'static str {
    if on {
        "Mouse on (M toggles) — Shift+drag selects text"
    } else {
        "Mouse off (M toggles)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn ev(kind: MouseEventKind) -> MouseEvent {
        MouseEvent { kind, column: 7, row: 9, modifiers: KeyModifiers::NONE }
    }

    #[test]
    fn mouse_reader_forwards_only_left_press_and_vertical_wheel() {
        assert_eq!(
            routed(&ev(MouseEventKind::Down(MouseButton::Left))),
            Some(Routed::Click { column: 7, row: 9 })
        );
        assert_eq!(
            routed(&ev(MouseEventKind::ScrollDown)),
            Some(Routed::Wheel { column: 7, row: 9, down: true })
        );
        assert_eq!(
            routed(&ev(MouseEventKind::ScrollUp)),
            Some(Routed::Wheel { column: 7, row: 9, down: false })
        );
        // Modifiers are ignored.
        let mut shifted = ev(MouseEventKind::Down(MouseButton::Left));
        shifted.modifiers = KeyModifiers::SHIFT;
        assert!(routed(&shifted).is_some());
        for kind in [
            MouseEventKind::Moved,
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
            MouseEventKind::Down(MouseButton::Right),
            MouseEventKind::Down(MouseButton::Middle),
            MouseEventKind::ScrollLeft,
            MouseEventKind::ScrollRight,
        ] {
            assert_eq!(routed(&ev(kind)), None, "{kind:?} must never enter the FIFO");
        }
    }

    #[test]
    fn mouse_double_click_needs_the_same_cell_within_the_window() {
        let t0 = Instant::now();
        let ms = |n| t0 + Duration::from_millis(n);
        let mut t = ClickTracker::default();
        assert!(!t.register(t0, 5, 3));
        assert!(t.register(ms(200), 5, 3));
        assert!(!t.register(ms(300), 5, 3), "a third click starts over");

        let mut t = ClickTracker::default();
        assert!(!t.register(t0, 5, 3));
        assert!(t.register(ms(100), 6, 3), "one column of slack");

        let mut t = ClickTracker::default();
        assert!(!t.register(t0, 5, 3));
        assert!(!t.register(ms(100), 5, 4), "another row is a new click");

        let mut t = ClickTracker::default();
        assert!(!t.register(t0, 5, 3));
        assert!(!t.register(ms(600), 5, 3), "outside the window");

        let mut t = ClickTracker::default();
        assert!(!t.register(t0, 5, 3));
        t.reset();
        assert!(!t.register(ms(100), 5, 3), "reset forgets the first press");
    }

    #[test]
    fn mouse_list_region_maps_rows_through_the_offset() {
        let r = ListRegion { rect: Rect::new(2, 4, 30, 5), offset: 10, len: 12 };
        assert_eq!(r.row_at(3, 4), Some(10));
        assert_eq!(r.row_at(3, 5), Some(11));
        assert_eq!(r.row_at(3, 6), None, "beyond len");
        assert_eq!(r.row_at(40, 4), None, "outside the columns");
        assert_eq!(r.row_at(3, 3), None, "above the rect");
        assert_eq!(r.row_at(3, 9), None, "below the rect");
    }

    #[test]
    fn mouse_tab_entry_rects_mirror_the_tabs_layout() {
        let area = Rect::new(1, 2, 20, 1);
        assert_eq!(
            tab_entry_rects(area, &[4, 5, 3], 1),
            vec![Rect::new(1, 2, 4, 1), Rect::new(6, 2, 5, 1), Rect::new(12, 2, 3, 1)]
        );
        // Clipping: the second entry is cut at the right edge, the third starts
        // past it and gets width 0.
        let narrow = Rect::new(0, 0, 8, 1);
        assert_eq!(
            tab_entry_rects(narrow, &[4, 5, 3], 1),
            vec![Rect::new(0, 0, 4, 1), Rect::new(5, 0, 3, 1), Rect::new(8, 0, 0, 1)]
        );
    }

    #[test]
    fn mouse_status_text_names_the_toggle_and_the_selection_modifier() {
        assert!(mouse_status_text(false).starts_with("Mouse off"));
        assert!(mouse_status_text(true).contains("Shift+drag"));
        assert!(mouse_status_text(true).contains("M toggles"));
    }
}
