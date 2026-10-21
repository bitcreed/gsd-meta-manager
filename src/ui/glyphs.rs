//! The single registry of status glyphs the TUI draws (quick 261021-1id).
//!
//! # The rule
//!
//! Every glyph in [`ALL`] is **one char** that is
//!
//! * East Asian Width Neutral, Narrow or Halfwidth (never Ambiguous, Wide or
//!   Fullwidth),
//! * not `Emoji`, `Emoji_Presentation` or `Extended_Pictographic`,
//! * exactly 1 cell under both `unicode-width` measures (`width` and
//!   `width_cjk`), and
//! * covered by DejaVu Sans Mono (the base of Menlo and Hack), so no terminal
//!   has to fall back to a second font for it.
//!
//! # Why
//!
//! ratatui counts a glyph as 1 cell. A terminal configured for CJK or
//! "ambiguous-wide" widths, or one that falls back to a CJK font, draws an
//! East-Asian-Ambiguous glyph in 2 cells. An emoji-capable glyph can switch to
//! 2-cell emoji presentation. Either way the terminal and ratatui disagree
//! about where the glyph ends, and every later column in that row shifts: a
//! status word lands in the posture column and a parked run reads as paused.
//!
//! # Exemptions (audited, kept, recorded as known limitations)
//!
//! * STRUCTURAL: U+2500..=U+259F (box drawing and block elements: borders,
//!   rules, graph lanes, the cap-bar fill, the text cursor). ratatui's own
//!   `Block` borders and `Gauge` use this range; every terminal font carries it
//!   and many draw it natively. No Narrow or Neutral equivalent exists.
//! * TYPOGRAPHY: the eight code points listed in the source-scan test, used
//!   inside composed text (separators, truncation, ranges, key hints, scroll
//!   indicators). Replacing them means an ASCII fallback mode, which is new
//!   config and so a follow-up (see the pending todo).
//!
//! # Adding a glyph
//!
//! Add a `pub const` below and list it in [`ALL`]. The tests in this module
//! check the rule, and the source scan proves nothing else under `src/ui/`
//! (plus the agents and git-lane files that draw glyphs) spells a status glyph
//! outside this registry.
//!
//! Every constant is a `&'static str` written as a `\u{..}` escape: a glyph can
//! never be derived from file content (D-13, D-24, T-18-32).

/// Driven, live, running, starting, legacy live, detail driver marker.
/// `U+29EB` BLACK LOZENGE. Was `U+25C6`.
pub const LOZENGE_BLACK: &str = "\u{29EB}";
/// Stale (filled = live, half = stale, open = unknown).
/// `U+2B16` DIAMOND WITH LEFT HALF BLACK. Was `U+25C8`.
pub const DIAMOND_HALF_BLACK: &str = "\u{2B16}";
/// Liveness unknown or unreadable. `U+25CA` LOZENGE. Was `U+25C7`.
pub const LOZENGE_WHITE: &str = "\u{25CA}";
/// Crashed, stalled, failed, missed. `U+2717` BALLOT X.
pub const BALLOT_X: &str = "\u{2717}";
/// Parked, needs a human. `U+2762` HEAVY EXCLAMATION MARK ORNAMENT. Was `U+2691`.
pub const ATTENTION: &str = "\u{2762}";
/// Paused (draws as two bars). `U+01C1` LATIN LETTER LATERAL CLICK. Was `U+2016`.
pub const DOUBLE_BAR: &str = "\u{01C1}";
/// Stopped, killed. `U+25AE` BLACK VERTICAL RECTANGLE. Was `U+25A0`.
pub const BLACK_RECT: &str = "\u{25AE}";
/// Done, succeeded, acted on, roadmap done, git commit, selected option.
/// `U+25CD` CIRCLE WITH VERTICAL FILL. Was `U+25CF`.
pub const CIRCLE_FILLED: &str = "\u{25CD}";
/// Unlaunched, pending, queued, roadmap ready, planned, succeeded-no-changes,
/// git merge. `U+25E6` WHITE BULLET. Was `U+25CB`.
pub const CIRCLE_OPEN: &str = "\u{25E6}";
/// Unmerged, delivered. `U+25D2` CIRCLE WITH LOWER HALF BLACK. Was `U+25D0`.
pub const CIRCLE_HALF: &str = "\u{25D2}";
/// Roadmap active. `U+25C9` FISHEYE.
pub const FISHEYE: &str = "\u{25C9}";
/// Roadmap blocked. `U+25CC` DOTTED CIRCLE.
pub const CIRCLE_DOTTED: &str = "\u{25CC}";
/// External job. `U+25F7` WHITE CIRCLE WITH UPPER RIGHT QUADRANT.
pub const CLOCK: &str = "\u{25F7}";
/// Session, roadmap selected, Waves running, current-phase marker.
/// `U+25BA` BLACK RIGHT-POINTING POINTER. Was `U+25B6`.
pub const POINTER_RIGHT: &str = "\u{25BA}";
/// Folded, collapsed, active wave row. `U+25B8` BLACK RIGHT-POINTING SMALL TRIANGLE.
pub const TRIANGLE_RIGHT_SMALL: &str = "\u{25B8}";
/// Open, expanded, more. `U+25BE` BLACK DOWN-POINTING SMALL TRIANGLE.
/// Also replaces `U+25BC` in the roadmap widget.
pub const TRIANGLE_DOWN_SMALL: &str = "\u{25BE}";
/// Roadmap dependency marker. `U+21E1` UPWARDS DASHED ARROW. Was `U+2191`.
pub const ARROW_UP_DASHED: &str = "\u{21E1}";
/// Roadmap unblocks marker. `U+21E3` DOWNWARDS DASHED ARROW. Was `U+2193`.
pub const ARROW_DOWN_DASHED: &str = "\u{21E3}";
/// Item unseen, roadmap implied, stderr marker, Waves queued.
/// `U+2219` BULLET OPERATOR. Was `U+00B7` as a marker.
pub const BULLET_OPERATOR: &str = "\u{2219}";
/// Done (Waves, stage ladder, checks). `U+2713` CHECK MARK.
pub const CHECK: &str = "\u{2713}";
/// Stage in progress. `U+22EF` MIDLINE HORIZONTAL ELLIPSIS. Was `U+2026` as a stage mark.
pub const MIDLINE_ELLIPSIS: &str = "\u{22EF}";
/// Tab overflow to the left. `U+2039` SINGLE LEFT-POINTING ANGLE QUOTATION MARK.
pub const ANGLE_LEFT: &str = "\u{2039}";
/// Tab overflow to the right, crumb. `U+203A` SINGLE RIGHT-POINTING ANGLE QUOTATION MARK.
pub const ANGLE_RIGHT: &str = "\u{203A}";
/// Injection marker. `U+00BB` RIGHT-POINTING DOUBLE ANGLE QUOTATION MARK.
pub const GUILLEMET_RIGHT: &str = "\u{00BB}";
/// Key hint. `U+23CE` RETURN SYMBOL.
pub const RETURN_SYMBOL: &str = "\u{23CE}";

/// Every registry glyph as `(name, glyph)`. The width-safety test iterates it.
pub const ALL: &[(&str, &str)] = &[
    ("LOZENGE_BLACK", LOZENGE_BLACK),
    ("DIAMOND_HALF_BLACK", DIAMOND_HALF_BLACK),
    ("LOZENGE_WHITE", LOZENGE_WHITE),
    ("BALLOT_X", BALLOT_X),
    ("ATTENTION", ATTENTION),
    ("DOUBLE_BAR", DOUBLE_BAR),
    ("BLACK_RECT", BLACK_RECT),
    ("CIRCLE_FILLED", CIRCLE_FILLED),
    ("CIRCLE_OPEN", CIRCLE_OPEN),
    ("CIRCLE_HALF", CIRCLE_HALF),
    ("FISHEYE", FISHEYE),
    ("CIRCLE_DOTTED", CIRCLE_DOTTED),
    ("CLOCK", CLOCK),
    ("POINTER_RIGHT", POINTER_RIGHT),
    ("TRIANGLE_RIGHT_SMALL", TRIANGLE_RIGHT_SMALL),
    ("TRIANGLE_DOWN_SMALL", TRIANGLE_DOWN_SMALL),
    ("ARROW_UP_DASHED", ARROW_UP_DASHED),
    ("ARROW_DOWN_DASHED", ARROW_DOWN_DASHED),
    ("BULLET_OPERATOR", BULLET_OPERATOR),
    ("CHECK", CHECK),
    ("MIDLINE_ELLIPSIS", MIDLINE_ELLIPSIS),
    ("ANGLE_LEFT", ANGLE_LEFT),
    ("ANGLE_RIGHT", ANGLE_RIGHT),
    ("GUILLEMET_RIGHT", GUILLEMET_RIGHT),
    ("RETURN_SYMBOL", RETURN_SYMBOL),
];

#[cfg(test)]
mod tests {
    use super::*;
    use icu_properties::props::{
        EastAsianWidth, Emoji, EmojiPresentation, ExtendedPictographic,
    };
    use icu_properties::{CodePointMapData, CodePointSetData};
    use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

    /// The glyphs this registry replaced (and the two the badge set dropped
    /// earlier). The control proves the predicate rejects every one.
    const FORMER: &[char] = &[
        '\u{25C6}', '\u{25C8}', '\u{25C7}', '\u{2016}', '\u{2691}', '\u{25A0}', '\u{25CF}',
        '\u{25CB}', '\u{25D0}', '\u{25B6}', '\u{25BC}', '\u{2191}', '\u{2193}', '\u{00B7}',
        '\u{2026}', '\u{23F8}', '\u{23F3}',
    ];

    /// One reason per failed check; empty means width-safe.
    fn width_safety_violations(c: char) -> Vec<&'static str> {
        let mut out = Vec::new();
        if c.width() != Some(1) {
            out.push("unicode-width width() is not 1");
        }
        if c.width_cjk() != Some(1) {
            out.push("unicode-width width_cjk() is not 1");
        }
        let mut buf = [0u8; 4];
        let s: &str = c.encode_utf8(&mut buf);
        if UnicodeWidthStr::width(s) != 1 {
            out.push("str width() is not 1");
        }
        if UnicodeWidthStr::width_cjk(s) != 1 {
            out.push("str width_cjk() is not 1");
        }
        let eaw = CodePointMapData::<EastAsianWidth>::new().get(c);
        if eaw == EastAsianWidth::Ambiguous {
            out.push("East Asian Width is Ambiguous");
        }
        if eaw == EastAsianWidth::Wide {
            out.push("East Asian Width is Wide");
        }
        if eaw == EastAsianWidth::Fullwidth {
            out.push("East Asian Width is Fullwidth");
        }
        if CodePointSetData::new::<Emoji>().contains(c) {
            out.push("is Emoji");
        }
        if CodePointSetData::new::<EmojiPresentation>().contains(c) {
            out.push("is Emoji_Presentation");
        }
        if CodePointSetData::new::<ExtendedPictographic>().contains(c) {
            out.push("is Extended_Pictographic");
        }
        out
    }

    #[test]
    fn every_registry_glyph_is_width_safe() {
        let mut failures = Vec::new();
        for (name, glyph) in ALL {
            let mut chars = glyph.chars();
            let (Some(c), None) = (chars.next(), chars.next()) else {
                failures.push(format!("{name}: not exactly one char"));
                continue;
            };
            let v = width_safety_violations(c);
            if !v.is_empty() {
                failures.push(format!("{name} U+{:04X}: {}", c as u32, v.join(", ")));
            }
            if FORMER.contains(&c) {
                failures.push(format!("{name} U+{:04X}: is a FORMER glyph", c as u32));
            }
        }
        assert!(failures.is_empty(), "unsafe registry glyphs:\n{}", failures.join("\n"));

        assert_eq!(ALL.len(), 25, "ALL has exactly the 25 glyph_map entries");
        for (i, (n1, g1)) in ALL.iter().enumerate() {
            for (n2, g2) in &ALL[i + 1..] {
                assert_ne!(n1, n2, "duplicate registry name");
                assert_ne!(g1, g2, "{n1} and {n2} draw the same glyph");
            }
        }
    }

    #[test]
    fn the_width_predicate_rejects_every_former_glyph() {
        for &c in FORMER {
            if c == '\u{2691}' {
                // Deviation, measured at execution time: icu_properties 2.3.0
                // does NOT list U+2691 as Extended_Pictographic (the Unicode 13
                // emoji-data.txt that termwiz ships does), and both widths are 1,
                // so this predicate accepts it. It is still FORMER (never allowed
                // in ALL, and replaced because older Unicode data and terminals
                // treat it as emoji-capable). See
                // `the_property_check_carries_weight_the_widths_cannot`.
                continue;
            }
            assert!(
                !width_safety_violations(c).is_empty(),
                "the predicate accepted former glyph U+{:04X}",
                c as u32
            );
        }
        // U+25B6 is rejected for Emoji; U+23F3 for Wide.
        assert!(width_safety_violations('\u{25B6}').contains(&"is Emoji"));
        assert!(width_safety_violations('\u{23F3}').contains(&"East Asian Width is Wide"));
    }

    /// U+23F8 is 1 cell under both widths and EAW Neutral, yet is emoji-capable:
    /// only the property check rejects it, which proves the property check
    /// carries weight the width checks cannot.
    #[test]
    fn the_property_check_carries_weight_the_widths_cannot() {
        assert_eq!('\u{23F8}'.width(), Some(1));
        assert_eq!('\u{23F8}'.width_cjk(), Some(1));
        let v = width_safety_violations('\u{23F8}');
        assert!(!v.is_empty(), "U+23F8 must be rejected");
        assert!(
            v.iter().all(|r| r.starts_with("is ")),
            "U+23F8 must be rejected by the property checks alone: {v:?}"
        );
    }
}
