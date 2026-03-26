use crate::state_reader::roadmap_md::RoadmapPhase;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;

/// Height of each phase box (top border + content + bottom border).
const BOX_HEIGHT: u16 = 3;
/// Height of the connector between boxes (pipe + arrow).
const CONNECTOR_HEIGHT: u16 = 2;

pub struct RoadmapWidget<'a> {
    pub phases: &'a [RoadmapPhase],
    pub current_phase_num: u32,
    pub scroll_offset: u16,
}

impl<'a> RoadmapWidget<'a> {
    /// Total height of one phase block (box + connector), except the last has no connector.
    fn phase_block_height() -> u16 {
        BOX_HEIGHT + CONNECTOR_HEIGHT
    }

    fn is_current(&self, phase: &RoadmapPhase) -> bool {
        phase.number.parse::<u32>().unwrap_or(0) == self.current_phase_num
    }

    fn phase_icon(phase: &RoadmapPhase, is_current: bool) -> &'static str {
        if phase.completed {
            "+"
        } else if is_current {
            "*"
        } else {
            "o"
        }
    }

    fn phase_style(phase: &RoadmapPhase, is_current: bool) -> Style {
        if is_current {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if phase.completed {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
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
            let is_current = self.is_current(phase);
            let style = Self::phase_style(phase, is_current);

            // Top border chars
            let (tl, horiz, tr, bl, br, vert) = if is_current {
                ('\u{250F}', '\u{2501}', '\u{2513}', '\u{2517}', '\u{251B}', '\u{2503}') // bold: heavy box
            } else {
                ('\u{250C}', '\u{2500}', '\u{2510}', '\u{2514}', '\u{2518}', '\u{2502}') // light box
            };

            // --- Top border ---
            if let Some(screen_y) = self.screen_y(y_logical, scroll_skip, area) {
                // Draw top border
                let border_str = format!(
                    "{}{}{}",
                    tl,
                    std::iter::repeat_n(horiz, box_width.saturating_sub(2))
                        .collect::<String>(),
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
                let icon = Self::phase_icon(phase, is_current);
                let plan_display = if phase.total_plans == 0 {
                    "0/?".to_string()
                } else {
                    format!("{}/{}", phase.completed_plans, phase.total_plans)
                };

                // Available space for name: box_width - 2 (vert chars) - icon(1) - space(1) - "P##:"(~4) - space(1) - plan_display - space(1)
                let prefix = format!("{} P{}: ", icon, phase.number);
                let suffix = format!("  {}", plan_display);
                let inner_width = box_width.saturating_sub(2); // content between vertical bars
                let name_max = inner_width.saturating_sub(prefix.len() + suffix.len());

                let name_truncated = if phase.name.len() > name_max {
                    format!("{}...", &phase.name[..name_max.saturating_sub(3)])
                } else {
                    phase.name.clone()
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
                    std::iter::repeat_n(horiz, box_width.saturating_sub(2))
                        .collect::<String>(),
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
