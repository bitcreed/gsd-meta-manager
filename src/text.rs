//! The one production spelling of the invisible-character CLASS, and the two
//! judgments made over it.
//!
//! **One class, two questions.** [`is_invisible_formatting_char`] is the single
//! production spelling of the zero-width and format ranges. Two predicates
//! consume it and they ask different things:
//!
//! * [`carries_visible_content`] — *emptiness*: is anything visible at all?
//!   Consumed by [`crate::driver::payload::NonBlank::new`],
//!   [`crate::journal::is_plain_path_component`]'s blank half,
//!   [`crate::app::goal_or_none`] and `crate::ui::screens::driver::goal_lines`.
//! * [`carries_invisible_formatting`] — *identity*: does it carry bytes that
//!   render as nothing? Consumed by
//!   [`crate::journal::is_plain_path_component`]'s identity half (and so,
//!   transitively, by every run directory, envelope root, credential scope and
//!   phase token) and by `crate::registry::Alias::new`.
//!
//! The second judgment exists because the first **structurally cannot** close
//! the look-alike harm: `carries_visible_content` refuses `"\u{200b}"` and
//! accepts `"demo\u{200b}"` by construction, so it can never tell two values
//! apart that render identically. Pass 5 named that harm, round 5 claimed to
//! close it with the emptiness judgment alone, and pass 6 reproduced it
//! unchanged. Both judgments over ONE class spelling is what stops the two from
//! drifting the way the two blankness spellings drifted below.
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
    value
        .chars()
        .any(|c| !(c.is_whitespace() || c.is_control() || is_invisible_formatting_char(c)))
}

/// The ONE production spelling of the zero-width and format ranges.
///
/// Private on purpose: the ranges are a class, not an API, and both judgments in
/// this module read them from here so a widening lands in both at once. The only
/// other spelling of these ranges in `src/` is the deliberately independent
/// test-side oracle named in the module doc.
fn is_invisible_formatting_char(c: char) -> bool {
    matches!(c, '\u{200b}'..='\u{200f}' | '\u{2060}'..='\u{2064}' | '\u{feff}')
}

/// Whether `value` carries a character that renders as nothing.
///
/// **The SECOND judgment over the SAME class, and the one the emptiness judgment
/// structurally cannot make.** [`carries_visible_content`] asks "is anything
/// visible?" — it refuses `"\u{200b}"` and, by construction, *accepts*
/// `"demo\u{200b}"`, because something visible is there. This asks "does it
/// carry bytes that render as nothing?" and refuses `"demo\u{200b}"`, which is
/// the difference between an emptiness check and an identity check. Two values
/// that a reader cannot tell apart must not both be able to name a project, a
/// run directory, an envelope root or a phase.
///
/// **The boundary of the claim, stated so it is not inherited as wider than it
/// is.** This covers exactly the invisible-formatting class this phase
/// reproduced — zero-width and format characters. It is **not** a general
/// Unicode-confusables defence: a Cyrillic `а` renders like a Latin `a` and is
/// accepted here, because homoglyph confusability is canon security territory
/// (Unicode TR39) and a partial implementation of it under this name would be
/// inherited as a boundary rather than as the narrow class it is.
///
/// Only *identity* seams consult this. Free text — a `--goal`, a `--command` —
/// legitimately carries ZWJ/ZWNJ (they are load-bearing in real scripts), so it
/// is judged by [`carries_visible_content`] alone.
pub fn carries_invisible_formatting(value: &str) -> bool {
    value.chars().any(is_invisible_formatting_char)
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

    /// **Both directions for the identity judgment.** The refusing direction is
    /// the point of the function; the accepting direction is what stops it from
    /// being a predicate that refuses everything — every shape the tree actually
    /// passes through an identity seam must survive it.
    #[test]
    fn only_a_value_carrying_a_character_that_renders_as_nothing_is_look_alike() {
        for carrying in [
            "demo\u{200b}",
            "de\u{200b}mo",
            "demo\u{feff}",
            "x\u{200c}y",
            "\u{200b}",
        ] {
            assert!(
                carries_invisible_formatting(carrying),
                "{carrying:?} carries a character that renders as nothing and \
                 must not be able to name an identity beside a twin that does not"
            );
        }

        for clean in [
            "demo",
            "20",
            "2.1",
            "2026-08-19T12-00-00Z-aaaa",
            " x ",
            "/gsd:progress",
        ] {
            assert!(
                !carries_invisible_formatting(clean),
                "{clean:?} carries nothing invisible; refusing it would narrow \
                 the tool rather than close the harm"
            );
        }
    }

    /// **The split that made the round-5 fix structurally unable to close pass
    /// 5's named harm**, pinned as a difference rather than as two behaviours.
    ///
    /// For every look-alike pair the two members AGREE under the emptiness
    /// judgment — both carry something visible, so `carries_visible_content`
    /// cannot separate them no matter how it is tuned — and DISAGREE under the
    /// identity judgment. If someone ever re-implemented
    /// `carries_invisible_formatting` in terms of visibility, this fails.
    #[test]
    fn the_two_judgments_agree_on_visibility_and_disagree_on_identity() {
        for (visible, look_alike) in crate::test_support::LOOK_ALIKE_PAIRS {
            assert!(
                carries_visible_content(visible) && carries_visible_content(look_alike),
                "both members of ({visible:?}, {look_alike:?}) carry visible \
                 content — the emptiness judgment cannot tell them apart, which \
                 is exactly why the identity judgment exists"
            );
            assert!(
                !carries_invisible_formatting(visible),
                "{visible:?} is the visible member and must pass the identity \
                 judgment"
            );
            assert!(
                carries_invisible_formatting(look_alike),
                "{look_alike:?} renders exactly as {visible:?} and must be \
                 refused by the identity judgment"
            );
        }
    }

    /// **The tracer for pass-7 gaps 1 and 2: the class outside the list.**
    ///
    /// Every character here is one pass 7 MEASURED as accepted against the
    /// shipped tree, and every one of them is `General_Category=Cf` or
    /// `Default_Ignorable_Code_Point` — inside the class this module's own doc
    /// names and outside the three literal ranges the implementation spelled.
    /// The emptiness half is the half no round had named: `--goal '\u{202e}'`
    /// carried no visible instruction and was accepted as one.
    ///
    /// **Committed RED, verbatim, before the fix** (the house tracer contract:
    /// red evidence lands in history first and the tree stays green meanwhile).
    /// Against HEAD, `cargo test --lib a_character_the_standard_calls_invisible`:
    ///
    /// ```text
    /// running 1 test
    /// test text::tests::a_character_the_standard_calls_invisible_is_refused_even_outside_the_old_ranges ... FAILED
    ///
    /// ---- text::tests::a_character_the_standard_calls_invisible_is_refused_even_outside_the_old_ranges stdout ----
    ///
    /// thread 'text::tests::a_character_the_standard_calls_invisible_is_refused_even_outside_the_old_ranges' (386400) panicked at src/text.rs:227:13:
    /// "\u{202e}" renders as nothing, so it carries no visible instruction — pass 7 measured this value ACCEPTED because the class was three hand-written ranges instead of the standard's own answer
    ///
    /// test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1037 filtered out; finished in 0.00s
    /// ```
    ///
    /// It fails on the FIRST assertion because `is_invisible_formatting_char`
    /// is three literal ranges; every later assertion fails for the same cause.
    #[test]
    #[ignore = "red: closes pass-7 gap 1/2; un-ignored in the fix commit"]
    fn a_character_the_standard_calls_invisible_is_refused_even_outside_the_old_ranges() {
        // The emptiness half (pass-7 gap 1) — each of these is a value whose
        // every character renders as nothing, so it carries no instruction.
        for solely_invisible in [
            "\u{202e}", // RIGHT-TO-LEFT OVERRIDE (Cf) — Trojan Source, CVE-2021-42574
            "\u{00ad}", // SOFT HYPHEN (Cf)
            "\u{034f}", // COMBINING GRAPHEME JOINER (Default_Ignorable, not Cf)
            "\u{e0041}", // TAG LATIN CAPITAL LETTER A (Cf) — the LLM smuggling carrier
            "\u{fe0f}", // VARIATION SELECTOR-16 (Default_Ignorable, not Cf)
            "\u{13430}", // EGYPTIAN HIEROGLYPH VERTICAL JOINER (Cf)
            "\u{180e}", // MONGOLIAN VOWEL SEPARATOR (Cf)
            "\u{fff9}", // INTERLINEAR ANNOTATION ANCHOR (Cf)
        ] {
            assert!(
                !carries_visible_content(solely_invisible),
                "{solely_invisible:?} renders as nothing, so it carries no \
                 visible instruction — pass 7 measured this value ACCEPTED \
                 because the class was three hand-written ranges instead of \
                 the standard's own answer"
            );
        }

        // The identity half (pass-7 gap 2) — visible content beside bytes that
        // render as nothing, so the emptiness judgment cannot see them.
        for embedded in ["demo\u{202e}", "demo\u{e0041}", "demo\u{ad}", "demo\u{fe0f}"] {
            assert!(
                carries_invisible_formatting(embedded),
                "{embedded:?} renders exactly as \"demo\" and must not be able \
                 to name an identity beside it"
            );
        }

        // And the seam that consumes both, so the tracer is end to end rather
        // than a predicate unit test.
        for outside in ["demo\u{202e}", "2\u{e0041}0"] {
            assert!(
                !crate::journal::is_plain_path_component(outside),
                "{outside:?} must not be able to name a run directory, envelope \
                 root, credential scope or phase token"
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
