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

    // -----------------------------------------------------------------------
    // The production source scan
    // -----------------------------------------------------------------------

    use icu_properties::props::GeneralCategory;
    use std::path::{Path, PathBuf};

    /// Box drawing and block elements: borders, rules, graph lanes, the cap-bar
    /// fill, the text cursor. Every terminal font carries them.
    const STRUCTURAL: std::ops::RangeInclusive<u32> = 0x2500..=0x259F;
    /// Typography inside composed text: separators, truncation, ranges, key
    /// hints, scroll indicators. Replacing them needs an ASCII fallback mode (a
    /// follow-up todo).
    const TYPOGRAPHY: [u32; 8] = [
        0x00B7, 0x2013, 0x2014, 0x2026, 0x2190, 0x2191, 0x2192, 0x2193,
    ];

    fn is_cfg_test_mod_decl(attr: &str, next: &str) -> bool {
        if attr.trim() != "#[cfg(test)]" {
            return false;
        }
        let mut n = next.trim();
        if let Some(rest) = n.strip_prefix("pub(") {
            n = rest.split_once(')').map_or("", |(_, r)| r).trim();
        } else if let Some(rest) = n.strip_prefix("pub ") {
            n = rest.trim();
        }
        match n.strip_prefix("mod ") {
            Some(rest) => rest.trim_end().ends_with('{') || rest.trim_end().ends_with(';'),
            None => false,
        }
    }

    /// The `(line, char)` pairs production code draws: `\u{HEX}` escapes and
    /// literal non-ASCII chars, outside comments and outside the first
    /// `#[cfg(test)] mod`, minus the Cc and Cf sanitizer data.
    fn drawn_codepoints(src: &str) -> Vec<(usize, char)> {
        let lines: Vec<&str> = src.lines().collect();
        let cut = (0..lines.len())
            .find(|&i| i + 1 < lines.len() && is_cfg_test_mod_decl(lines[i], lines[i + 1]))
            .unwrap_or(lines.len());
        let category = CodePointMapData::<GeneralCategory>::new();
        let mut out = Vec::new();
        for (idx, raw) in lines[..cut].iter().enumerate() {
            let trimmed = raw.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            let mut line = *raw;
            if let Some(pos) = line.rfind("//") {
                if !line[pos + 2..].contains('"') {
                    line = &line[..pos];
                }
            }
            let mut found: Vec<char> = Vec::new();
            let bytes = line.as_bytes();
            let mut i = 0;
            let mut literal_start = 0;
            while i < bytes.len() {
                if bytes[i] == b'\\' && line[i..].starts_with("\\u{") {
                    if let Some(end) = line[i..].find('}') {
                        let hex = &line[i + 3..i + end];
                        if (1..=6).contains(&hex.len()) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                            found.extend(line[literal_start..i].chars().filter(|c| !c.is_ascii()));
                            if let Some(c) = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32) {
                                found.push(c);
                            }
                            i += end + 1;
                            literal_start = i;
                            continue;
                        }
                    }
                }
                i += 1;
            }
            found.extend(line[literal_start..].chars().filter(|c| !c.is_ascii()));
            for c in found {
                let gc = category.get(c);
                if gc == GeneralCategory::Control || gc == GeneralCategory::Format {
                    continue;
                }
                out.push((idx + 1, c));
            }
        }
        out
    }

    fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
            .map(|e| e.expect("dir entry").path())
            .collect();
        entries.sort();
        for p in entries {
            if p.is_dir() {
                collect_rs(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }

    #[test]
    fn production_ui_code_draws_only_registry_or_exempt_glyphs() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        collect_rs(&root.join("src/ui"), &mut files);
        for extra in [
            "src/agents/unmerged.rs",
            "src/agents/waves.rs",
            "src/state_reader/git_ops.rs",
        ] {
            files.push(root.join(extra));
        }
        // The exclusion may only hide a test module: prove it is one.
        let screens_mod = std::fs::read_to_string(root.join("src/ui/screens/mod.rs")).expect("mod.rs");
        assert!(
            screens_mod.contains("#[cfg(test)]\nmod render_escape_guard;"),
            "render_escape_guard must stay declared under #[cfg(test)]"
        );
        files.retain(|p| !p.ends_with("src/ui/screens/render_escape_guard.rs"));

        let registry: Vec<u32> = ALL.iter().flat_map(|(_, g)| g.chars().map(|c| c as u32)).collect();
        for t in TYPOGRAPHY {
            assert!(!registry.contains(&t), "U+{t:04X} is both registry and TYPOGRAPHY");
        }
        for r in &registry {
            assert!(!STRUCTURAL.contains(r), "U+{r:04X} is both registry and STRUCTURAL");
        }

        let mut stray = Vec::new();
        let (mut escape_seen, mut literal_seen) = (false, false);
        let (mut structural_seen, mut typography_seen) = (false, false);
        for f in &files {
            let src = std::fs::read_to_string(f).unwrap_or_else(|e| panic!("{}: {e}", f.display()));
            for (line, c) in drawn_codepoints(&src) {
                let v = c as u32;
                if src.lines().nth(line - 1).is_some_and(|l| l.contains(&format!("{c}"))) {
                    literal_seen = true;
                } else {
                    escape_seen = true;
                }
                if STRUCTURAL.contains(&v) {
                    structural_seen = true;
                } else if TYPOGRAPHY.contains(&v) {
                    typography_seen = true;
                } else if !registry.contains(&v) {
                    stray.push(format!(
                        "{}:{line}: U+{v:04X} {c:?}",
                        f.strip_prefix(root).unwrap_or(f).display()
                    ));
                }
            }
        }
        assert!(files.len() >= 25, "only {} files read", files.len());
        assert!(escape_seen, "no escape-form codepoint seen: the scan is vacuous");
        assert!(literal_seen, "no literal codepoint seen: the scan is vacuous");
        assert!(structural_seen, "no STRUCTURAL codepoint seen");
        assert!(typography_seen, "no TYPOGRAPHY codepoint seen");
        assert!(
            stray.is_empty(),
            "glyphs drawn outside the registry and the exemptions:\n{}",
            stray.join("\n")
        );
    }

    #[test]
    fn the_scanner_flags_a_stray_glyph_and_ignores_tests_and_comments() {
        let src = r##"pub const A: &str = "\u{25C6}";
pub const B: &str = "◆ x";
// a comment line with ◆ and \u{25C6}
let _c = 1; // a trailing comment with ◆
#[cfg(test)]
mod tests {
    const T: &str = "\u{2691}";
}
"##;
        let got = drawn_codepoints(src);
        assert_eq!(got, vec![(1, '\u{25C6}'), (2, '\u{25C6}')], "{got:?}");
    }

    #[test]
    fn glyphs_defined_outside_ui_are_registry_glyphs() {
        assert_eq!(crate::agents::unmerged::GLYPH_UNMERGED, CIRCLE_HALF);
        assert_eq!(crate::state_reader::git_ops::GRAPH_COMMIT.to_string(), CIRCLE_FILLED);
        assert_eq!(crate::state_reader::git_ops::GRAPH_MERGE.to_string(), CIRCLE_OPEN);
    }
}
