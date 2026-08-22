//! The one production spelling of "does this text carry anything a reader could
//! see?".
//!
//! **Why this module exists at all: two spellings shipped in one commit and
//! disagreed.** Round 4 built [`crate::driver::payload::NonBlank`] around an
//! explicit character-class test — whitespace, control, and the zero-width and
//! format ranges — while [`crate::journal::is_plain_path_component`] kept
//! judging blankness with `str::trim`. `str::trim` cannot see `U+200B` or
//! `U+FEFF`, so the two predicates answered differently for the same value, and
//! pass 5 reproduced what that cost end to end: `--run-id '\u{200b}'` drove a
//! **complete run** whose directory name was one invisible character, and two
//! visually identical registry aliases resolved to two different envelope paths.
//!
//! One judgment, one place. Every production consumer delegates here:
//! [`crate::driver::payload::NonBlank::new`],
//! [`crate::journal::is_plain_path_component`], [`crate::app::goal_or_none`] and
//! `crate::ui::screens::driver::goal_lines`.
//!
//! **The deliberate exception, named rather than left to be discovered.** The
//! test-side detector `visibly_empty_numbered_entry` in `src/driver/mod.rs`
//! re-spells these classes independently and MUST keep doing so. It is an oracle
//! for the guard, and an oracle that consumes the predicate it checks can only
//! ever agree with it — which is the tautology round-3 WR-03 named and round 4
//! removed. If this predicate were ever loosened, that detector is what goes red.

/// Whether `value` carries at least one character a reader could see.
///
/// **"Visible" is deliberately wider than `!str::trim().is_empty()`** (D-13-2).
/// Trimming answers only for whitespace; a value of `U+200B` (zero-width space),
/// `U+FEFF` (byte-order mark) or `U+2060` (word joiner) survives it untouched
/// while rendering as nothing and recording as *field absent* on the tolerant
/// read path (D-30). Refused here: the empty string, and any value whose every
/// character is whitespace, a control character, or a zero-width/format
/// character.
///
/// It says nothing about whether the visible content is *safe* — embedded
/// control characters, path separators and traversal tokens are a different
/// question, answered where it belongs by
/// [`crate::journal::is_plain_path_component`]'s own structural half. This
/// predicate composes with those checks; it does not replace them.
pub fn carries_visible_content(value: &str) -> bool {
    value.chars().any(|c| {
        !(c.is_whitespace()
            || c.is_control()
            || matches!(c, '\u{200b}'..='\u{200f}' | '\u{2060}'..='\u{2064}' | '\u{feff}'))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Both directions, and the acceptances matter as much as the refusals.**
    /// A predicate that refused everything would satisfy every blank-payload pin
    /// in the tree forever while making the tool unusable, so one visible
    /// character — padded with the exact invisible characters the refusals are
    /// made of — must pass.
    #[test]
    fn only_a_value_with_no_visible_character_is_blank() {
        for blank in crate::test_support::DEGENERATE {
            assert!(
                !carries_visible_content(blank),
                "{blank:?} carries no visible instruction and must be judged blank"
            );
        }
        for blank in ["\u{2060}", "\u{200b}\u{feff}", "\u{200f}", "\r\n"] {
            assert!(
                !carries_visible_content(blank),
                "{blank:?} carries no visible instruction and must be judged blank"
            );
        }

        for visible in ["x", " x ", "\u{200b}x", "x\u{feff}", "20", "2.1", "/gsd:progress"] {
            assert!(
                carries_visible_content(visible),
                "{visible:?} carries a visible instruction and must be judged \
                 non-blank — a guard that refused this would be refusing on \
                 length rather than on emptiness"
            );
        }
    }

    /// The disagreement this module exists to remove, pinned as a *difference*
    /// rather than only as a behaviour: `str::trim` accepts every one of these
    /// and this predicate refuses them. If someone re-implemented
    /// `carries_visible_content` as a trim, this fails.
    #[test]
    fn the_zero_width_classes_are_exactly_what_trim_cannot_see() {
        for invisible in ["\u{200b}", "\u{feff}", "\u{2060}", "\u{200e}"] {
            assert!(
                !invisible.trim().is_empty(),
                "{invisible:?} must survive `str::trim` — otherwise this test is \
                 not exercising the gap between the two judgments"
            );
            assert!(
                !carries_visible_content(invisible),
                "{invisible:?} survives `trim` and must still be refused here; \
                 pass 5 reproduced a complete run whose directory name was \
                 exactly this value"
            );
        }
    }
}
