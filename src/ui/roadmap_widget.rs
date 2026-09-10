use crate::state_reader::disk_status::DiskInference;
use crate::state_reader::roadmap_md::RoadmapPhase;
use crate::state_reader::PhaseMarker;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;
use std::collections::HashMap;

/// Height of each phase box (top border + content + bottom border).
const BOX_HEIGHT: u16 = 3;
/// Height of the connector between boxes (pipe + arrow).
const CONNECTOR_HEIGHT: u16 = 2;

pub struct RoadmapWidget<'a> {
    pub phases: &'a [RoadmapPhase],
    /// Which phase to mark `*`. Callers pass
    /// [`crate::state_reader::ProjectState::active_phase_number`] — the
    /// disk-inferred frontier — rather than `completed_phases + 1`, which
    /// tracks the roadmap's completion count and lags behind the disk.
    pub current_phase_num: u32,
    /// Per-phase disk inference, keyed exactly as `RoadmapPhase::number` is
    /// written — [`crate::state_reader::ProjectState::phase_disk_statuses`].
    ///
    /// This is what decides `+` (see [`PhaseMarker`]). Pass an empty map only
    /// when there genuinely is no scan: every phase then falls back to its
    /// ROADMAP checkbox, which is the pre-`PhaseMarker` behaviour and the
    /// degraded-but-sensible answer, not a wrong one.
    pub disk_statuses: &'a HashMap<String, DiskInference>,
    pub scroll_offset: u16,
}

impl<'a> RoadmapWidget<'a> {
    /// Total height of one phase block (box + connector), except the last has no connector.
    fn phase_block_height() -> u16 {
        BOX_HEIGHT + CONNECTOR_HEIGHT
    }

    /// The one marker decision, shared with the Detail screen's Phases list.
    fn marker(&self, phase: &RoadmapPhase) -> PhaseMarker {
        PhaseMarker::decide(
            &phase.number,
            phase.completed,
            self.disk_statuses,
            self.current_phase_num,
        )
    }

    fn phase_style(marker: PhaseMarker) -> Style {
        match marker {
            PhaseMarker::Current => Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            PhaseMarker::Done => Style::default().fg(Color::DarkGray),
            PhaseMarker::Future => Style::default(),
        }
    }
}

impl<'a> Widget for RoadmapWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 10 || area.height < 3 || self.phases.is_empty() {
            return;
        }

        // Box width: min(area.width - 4, 40) to leave room for marker and margins
        let box_width = (area.width.saturating_sub(4)).min(40) as usize;
        // Horizontal centering: left margin to center the box (account for 2-char marker area)
        let marker_width: u16 = 2;
        let total_box_area = marker_width + box_width as u16;
        let left_margin = area.x + (area.width.saturating_sub(total_box_area)) / 2;
        let box_x = left_margin + marker_width;

        // Compute total content height
        let total_height: u16 = if self.phases.len() <= 1 {
            BOX_HEIGHT
        } else {
            BOX_HEIGHT + (self.phases.len() as u16 - 1) * Self::phase_block_height()
        };

        // Scroll: skip scroll_offset lines from top
        let scroll_skip = self.scroll_offset;
        let mut y_logical: u16 = 0; // logical y (before scroll)

        for (i, phase) in self.phases.iter().enumerate() {
            let marker = self.marker(phase);
            let is_current = marker == PhaseMarker::Current;
            let style = Self::phase_style(marker);

            // Top border chars
            let (tl, horiz, tr, bl, br, vert) = if is_current {
                (
                    '\u{250F}', '\u{2501}', '\u{2513}', '\u{2517}', '\u{251B}', '\u{2503}',
                ) // bold: heavy box
            } else {
                (
                    '\u{250C}', '\u{2500}', '\u{2510}', '\u{2514}', '\u{2518}', '\u{2502}',
                ) // light box
            };

            // --- Top border ---
            if let Some(screen_y) = self.screen_y(y_logical, scroll_skip, area) {
                // Draw top border
                let border_str = format!(
                    "{}{}{}",
                    tl,
                    std::iter::repeat_n(horiz, box_width.saturating_sub(2)).collect::<String>(),
                    tr
                );
                buf.set_string(box_x, screen_y, &border_str, style);

                // Draw marker for current phase
                if is_current {
                    // Place marker one line down (on content line), but let's place on top border line
                }
            }
            y_logical += 1;

            // --- Content line ---
            if let Some(screen_y) = self.screen_y(y_logical, scroll_skip, area) {
                let icon = marker.glyph();
                let plan_display = if phase.total_plans == 0 {
                    "0/?".to_string()
                } else {
                    format!("{}/{}", phase.completed_plans, phase.total_plans)
                };

                // The identity split at this site (CR-01): `phase.number` is
                // COMPARED raw by `Self::marker` above and only READ here,
                // and `phase.name` is only ever read. Both come out of the
                // project's `.planning/ROADMAP.md`, which is third-party text
                // under SAFE-07, so both are escaped on their way to a cell.
                //
                // Escape BEFORE measuring and truncating — the escaped form is
                // what occupies cells — and truncate by `char`, because
                // `&s[..n]` panics when byte `n` is not a char boundary and a
                // phase name is read off disk. That was a reachable panic for as
                // long as the slice was written that way.
                //
                // Both halves, through the ONE composition (WR-05): a
                // `.planning/ROADMAP.md` this build did not author can carry a
                // raw `ESC` exactly as easily as a `U+202E`, and this file used
                // to apply only the invisible-formatting half. See
                // `crate::ui::tests` for the census that keeps this true.
                //
                // **The width arithmetic below is DELIBERATELY UNTOUCHED.**
                // `chars().count()` is not display width and the truncation can
                // split a grapheme cluster; both are recorded as IN-02 and IN-03
                // in this phase's `deferred-items.md` with their reasons, and
                // both are outside round 10's scope. The conversion changes
                // WHICH function escapes, not how the result is measured — read
                // the untouched arithmetic as a deferral, not an oversight.
                //
                // Neither site is an `Into<Cow>` sink: both measure and truncate
                // the escaped form by `char`. `name_shown` therefore takes the
                // `Rendered` into a `String` through the `From<Rendered> for
                // String` that `text.rs` provides for exactly this — the
                // carrier's documented trait surface, not a `.to_string()`
                // workaround.
                //
                // Available space for name: box_width - 2 (vert chars) - icon(1) - space(1) - "P##:"(~4) - space(1) - plan_display - space(1)
                let prefix = format!(
                    "{} P{}: ",
                    icon,
                    crate::text::render_for_terminal(&phase.number)
                );
                let suffix = format!("  {}", plan_display);
                let inner_width = box_width.saturating_sub(2); // content between vertical bars
                let name_max = inner_width.saturating_sub(prefix.chars().count() + suffix.chars().count());

                let name_shown: String = crate::text::render_for_terminal(&phase.name).into();
                let name_truncated = if name_shown.chars().count() > name_max {
                    format!(
                        "{}...",
                        name_shown
                            .chars()
                            .take(name_max.saturating_sub(3))
                            .collect::<String>()
                    )
                } else {
                    name_shown
                };

                let content = format!("{}{}{}", prefix, name_truncated, suffix);
                // Pad to inner_width
                let padded = format!("{:<width$}", content, width = inner_width);

                let line = format!("{}{}{}", vert, padded, vert);
                buf.set_string(box_x, screen_y, &line, style);

                // Draw current phase marker
                if is_current && left_margin > area.x {
                    buf.set_string(
                        left_margin,
                        screen_y,
                        "\u{25B6}",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    );
                }
            }
            y_logical += 1;

            // --- Bottom border ---
            if let Some(screen_y) = self.screen_y(y_logical, scroll_skip, area) {
                let border_str = format!(
                    "{}{}{}",
                    bl,
                    std::iter::repeat_n(horiz, box_width.saturating_sub(2)).collect::<String>(),
                    br
                );
                buf.set_string(box_x, screen_y, &border_str, style);
            }
            y_logical += 1;

            // --- Connector (except after last phase) ---
            if i < self.phases.len() - 1 {
                let connector_x = box_x + (box_width as u16) / 2;

                // Pipe
                if let Some(screen_y) = self.screen_y(y_logical, scroll_skip, area) {
                    buf.set_string(connector_x, screen_y, "\u{2502}", style);
                }
                y_logical += 1;

                // Arrow
                if let Some(screen_y) = self.screen_y(y_logical, scroll_skip, area) {
                    buf.set_string(connector_x, screen_y, "\u{25BC}", style);
                }
                y_logical += 1;
            }
        }

        // Suppress unused variable warning
        let _ = total_height;
    }
}

impl<'a> RoadmapWidget<'a> {
    /// Convert logical y to screen y, accounting for scroll and area bounds.
    /// Returns None if the line is scrolled off or below the area.
    fn screen_y(&self, y_logical: u16, scroll_skip: u16, area: Rect) -> Option<u16> {
        if y_logical < scroll_skip {
            return None;
        }
        let screen_y = area.y + (y_logical - scroll_skip);
        if screen_y >= area.y + area.height {
            return None;
        }
        Some(screen_y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::disk_status::DiskStatus;

    fn phase(number: &str, name: &str, completed: bool) -> RoadmapPhase {
        RoadmapPhase {
            number: number.to_string(),
            name: name.to_string(),
            description: String::new(),
            completed,
            total_plans: 7,
            completed_plans: 7,
            depends_on: Vec::new(),
        }
    }

    fn inference(status: DiskStatus) -> DiskInference {
        DiskInference {
            status,
            ..Default::default()
        }
    }

    /// The glyph column, read off the rendered cells rather than off the
    /// decision that produced them. Returns one entry per content line, in
    /// order.
    fn glyph_column(buf: &Buffer, area: Rect) -> Vec<char> {
        let mut out = Vec::new();
        for y in area.y..area.y + area.height {
            let row: String = (area.x..area.x + area.width)
                .map(|x| buf[(x, y)].symbol())
                .collect::<Vec<_>>()
                .concat();
            // A content line is the one carrying `P<number>:`.
            if let Some(i) = row.find(" P") {
                if row[i + 2..].starts_with(|c: char| c.is_ascii_digit()) {
                    // The glyph sits immediately before the leading space.
                    if let Some(g) = row[..i].chars().next_back() {
                        out.push(g);
                    }
                }
            }
        }
        out
    }

    /// picsync's shape, rendered: phase 3 is `Complete` on disk while its
    /// ROADMAP checkbox is a stale `- [ ]`, and the active phase is 4. Before
    /// the marker moved onto disk inference this drew `o` for phase 3 — the
    /// *future* glyph, for a phase with a passing verification behind it.
    #[test]
    fn a_phase_complete_on_disk_draws_the_done_glyph_not_the_future_one() {
        let phases = [
            phase("1", "Bootstrap", true),
            phase("2", "Ingest", true),
            phase("3", "Vertical Slice", false),
            phase("4", "Pixel over ADB", false),
            phase("5", "iPhone over AFC", false),
        ];
        let mut disk = HashMap::new();
        disk.insert("1".to_string(), inference(DiskStatus::Complete));
        disk.insert("2".to_string(), inference(DiskStatus::Complete));
        disk.insert("3".to_string(), inference(DiskStatus::Complete));
        disk.insert("4".to_string(), inference(DiskStatus::Planned));
        disk.insert("5".to_string(), inference(DiskStatus::NoDirectory));

        let area = Rect::new(0, 0, 60, 30);
        let mut buf = Buffer::empty(area);
        RoadmapWidget {
            phases: &phases,
            current_phase_num: 4,
            disk_statuses: &disk,
            scroll_offset: 0,
        }
        .render(area, &mut buf);

        assert_eq!(
            glyph_column(&buf, area),
            vec!['+', '+', '+', '*', 'o'],
            "phase 3 is done on disk; phase 4 is current; phase 5 has no directory"
        );
    }

    /// With no scan at all, the ROADMAP checkboxes still drive the glyphs.
    #[test]
    fn without_disk_inference_the_checkboxes_still_draw_the_glyphs() {
        let phases = [
            phase("1", "Bootstrap", true),
            phase("2", "Ingest", false),
            phase("3", "Vertical Slice", false),
        ];
        let disk = HashMap::new();

        let area = Rect::new(0, 0, 60, 20);
        let mut buf = Buffer::empty(area);
        RoadmapWidget {
            phases: &phases,
            current_phase_num: 2,
            disk_statuses: &disk,
            scroll_offset: 0,
        }
        .render(area, &mut buf);

        assert_eq!(glyph_column(&buf, area), vec!['+', '*', 'o']);
    }
}
