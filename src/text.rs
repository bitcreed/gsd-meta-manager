//! The one production spelling of the invisible-character CLASS, the two
//! judgments made over it, and the finite ALPHABET that judges identities.
//!
//! **Two directions, because the two problems are not the same problem**
//! (D-19-1, D-19-2). Free text — a `--goal`, a `--command` — legitimately
//! carries arbitrary script, so it is judged by a DERIVED DENY-LIST: the
//! invisible class, read from Unicode's own data. Identities — aliases, run
//! ids, phase tokens, envelope roots — need no script at all, so they are
//! judged by a finite ALLOW-LIST, [`is_identity_char`], which structurally
//! cannot be one code point short. This phase spent six rounds discovering that
//! a deny-list over a growing standard always can be.
//!
//! **One class, two questions.** [`is_invisible_formatting_char`] is the single
//! production spelling of the invisible class — `General_Category=Cf` union
//! `Default_Ignorable_Code_Point`, derived from `icu_properties` rather than
//! enumerated. Two predicates consume it and they ask different things:
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
//! **The deliberate exception, restated rather than repointed** (D-19-3). The
//! test-side detector `visibly_empty_numbered_entry` in `src/driver/mod.rs` is
//! an oracle for the guard, and it MUST NOT read
//! [`is_invisible_formatting_char`]. Its old doc here said it "re-spells these
//! classes independently"; after this round that sentence is false in one
//! direction and stronger in another, so both halves are said plainly:
//!
//! * **It is no longer independent OF THE TESTS.** It now shares the
//!   `unicode-properties` dev-dependency — and the named non-`Cf`
//!   default-ignorable member list — with THIS module's own test module. The two
//!   are the second and third consumers of one oracle, and a wrong oracle
//!   deceives both.
//! * **It is still independent OF PRODUCTION, which is the only property that
//!   was ever load-bearing.** It shares no code with
//!   [`is_invisible_formatting_char`], which reads `icu_properties` and nothing
//!   else. So the tautology round-3 WR-03 named — an oracle that consumes the
//!   predicate it checks and can therefore only agree with it — remains closed,
//!   and if this predicate were ever narrowed that detector is what goes red.
//!
//! The residual, named because it is real: a defect in `unicode-properties`
//! itself is invisible to every test in this tree. Only a defect in the
//! IMPLEMENTATION's derivation is caught, which is the defect this phase kept
//! shipping.

use icu_properties::props::{DefaultIgnorableCodePoint, GeneralCategory};
use icu_properties::{
    CodePointMapData, CodePointMapDataBorrowed, CodePointSetData, CodePointSetDataBorrowed,
};

/// Whether `value` carries at least one character a reader could see.
///
/// **"Visible" is deliberately wider than `!str::trim().is_empty()`** (D-13-2).
/// Trimming answers only for whitespace; a value of `U+200B` (zero-width space),
/// `U+FEFF` (byte-order mark) or `U+2060` (word joiner) survives it untouched
/// while rendering as nothing and recording as *field absent* on the tolerant
/// read path (D-30). Refused here: the empty string, and any value whose every
/// character is whitespace, a control character, or in the derived invisible
/// class ([`is_invisible_formatting_char`]).
///
/// **The class widened here as of D-19-1, and this is a released-surface
/// change.** Emptiness used to be judged against three literal ranges, so a
/// value of one `U+202E`, `U+00AD`, `U+034F`, `U+E0041`, `U+FE0F`, `U+13430`,
/// `U+180E` or `U+FFF9` carried "visible content" and was accepted as an
/// instruction — pass 7 measured every one of them. They are refused now.
/// Nothing that was accepted and legitimate stopped being accepted: embedded
/// ZWJ/ZWNJ beside visible content still pass, which is what
/// `free_text_keeps_its_joiners` pins.
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

/// The ONE production spelling of the invisible class — **derived, not
/// enumerated** (D-19-1).
///
/// A character is in the class when Unicode's own data says its
/// `General_Category` is `Format` (`Cf`) OR that it is a
/// `Default_Ignorable_Code_Point`. Those two properties are read from
/// `icu_properties`' compiled data. **No literal character range appears in this
/// function, and none may be added back.**
///
/// **Why derived.** This used to be three hand-written ranges — `U+200B..U+200F`,
/// `U+2060..U+2064`, `U+FEFF` — under a doc that called them "the zero-width and
/// format ranges". They were a 22-code-point subset of that class, and pass 7
/// measured twenty values accepted because of the gap: a `--goal` of one
/// `U+202E` (RIGHT-TO-LEFT OVERRIDE), of one `U+00AD` (SOFT HYPHEN), of one
/// `U+E0041` (a tag character, the LLM ASCII-smuggling carrier). Six consecutive
/// rounds answered a miss like that by enumerating the next level down, and each
/// time the next level down was still a subset. A deny-list over a growing
/// standard can always be one code point short; the only way out of that
/// regress is to stop writing the list. For *identities*, where the accepted set
/// can be finite, this phase went further still — see [`is_identity_char`].
///
/// **Maintenance obligation, so it is not inherited as timeless.** The class is
/// exactly as current as `icu_properties`' pinned Unicode version. It refreshes
/// under the release process's existing `cargo update` (CLAUDE.md, step 2), and
/// the exhaustive sweep in this module's test module goes red on version skew
/// between the implementation's data and the independent oracle's.
///
/// **Residual, disclosed (T-21-19-05).** A code point *unassigned* at the pinned
/// version, which a later Unicode release makes `Cf`, is accepted in FREE TEXT
/// until the next refresh. It cannot reach an identity: identities are judged by
/// the finite alphabet, which is version-independent.
///
/// Private on purpose: the class is a class, not an API. Every judgment in this
/// module reads it from here so a widening lands in all of them at once.
fn is_invisible_formatting_char(c: char) -> bool {
    // Both constructors are `const fn` over compiled data, so these are
    // borrowed static references — no allocation, no lazy-init cell.
    const GENERAL_CATEGORY: CodePointMapDataBorrowed<'static, GeneralCategory> =
        CodePointMapData::<GeneralCategory>::new();
    const DEFAULT_IGNORABLE: CodePointSetDataBorrowed<'static> =
        CodePointSetData::new::<DefaultIgnorableCodePoint>();

    GENERAL_CATEGORY.get(c) == GeneralCategory::Format || DEFAULT_IGNORABLE.contains(c)
}

/// The ONE spelling of the identity alphabet: `[A-Za-z0-9._-]`.
///
/// **This is an ALLOW-LIST, and the direction is the whole point** (D-19-2,
/// Premise 8 — "level 5").
///
/// [`is_invisible_formatting_char`] answers a deny-list question over 1.1M
/// growing code points, and a deny-list like that can always be one item short.
/// This phase proved that empirically at three successive levels: round 3 missed
/// arms, round 5 missed fields, rounds 4-7 missed character ranges — each time
/// the fix enumerated the next level down and each time the next level down was
/// still a subset. An allow-list cannot be one item short, because the accepted
/// set is *finite*. That is a structural property, not a promise to be more
/// careful next round, and it is the only reason to prefer this direction.
///
/// So every identity seam judges with this: run directories, envelope roots,
/// credential scopes, phase tokens (via
/// [`crate::journal::is_plain_path_component`]) and registry aliases (via
/// `crate::registry::Alias::new`). In one clause, with no table and no
/// dependency, it closes bidi controls (Trojan Source, CVE-2021-42574), the
/// `U+E0000..U+E007F` tag block, variation selectors — and homoglyphs, which
/// the deny-list explicitly could not close (a Cyrillic `а` is simply not in the
/// alphabet).
///
/// **Where this must NOT go: free text.** A `--goal` or `--command`
/// legitimately carries arbitrary script, and ZWJ/ZWNJ are load-bearing in real
/// writing. An ASCII allow-list there would refuse legitimate input. Free text
/// keeps [`carries_visible_content`] over the derived class. Two questions, two
/// directions; the boundary is "does this value become a filesystem path,
/// registry key, or comparison token?"
///
/// **THE RECORDED PRODUCT TRADE (D-19-2), so it is chosen rather than
/// discovered.** An alias, run id, or phase token cannot carry a non-ASCII
/// script. A project living at ANY path can still be registered — the alias is
/// the tool's identifier for the project, not the project's name, and the folder
/// itself may be called anything in any script. A non-Latin alias that an older
/// build accepted stops working: registration refuses it, and legacy entries
/// fail closed at the envelope seams. The recovery route is the one already
/// shipped and named in the refusal messages — `remove <alias>`, then re-add
/// under an ASCII alias (D-17-3). Reversibility: costly, NOT one-way. Reverting
/// this clause restores acceptance, no data is destroyed, and legacy entries
/// stay in `config.json` and removable throughout — which is certified by
/// `a_legacy_alias_the_alphabet_refuses_is_still_removable`, not by this
/// sentence.
///
/// **The honest bottom, recorded so it is not re-derived.** No predicate over
/// code points is complete for "renders identically" — rendering belongs to
/// fonts and shaping engines. The move with zero enumeration left is to stop
/// letting user bytes BE an identity at all (generated keys, user string as
/// display label). That is a larger change and is deliberately not made here.
pub fn is_identity_char(c: char) -> bool {
    matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | '-')
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
/// is.** This covers exactly the derived invisible class —
/// `General_Category=Cf` union `Default_Ignorable_Code_Point`. It is **not** a
/// general Unicode-confusables defence: a Cyrillic `а` renders like a Latin `a`
/// and is accepted *here*, because homoglyph confusability is canon security
/// territory (Unicode TR39) and a partial implementation of it under this name
/// would be inherited as a boundary rather than as the narrow class it is. That
/// carve-out is unchanged by D-19-1 and D-19-2. Note where the homoglyph harm
/// IS closed and where it is not: at identity seams it is closed, not by this
/// predicate but by the finite alphabet ([`is_identity_char`]), which admits no
/// Cyrillic at all; in free text it remains open by design.
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

    /// **The sampling fix, and the only fixture shape in this tree that can go
    /// red on a SUBSET** (D-19-3, the round's primary target).
    ///
    /// Every other fixture here is a list somebody wrote. For six rounds every
    /// such list was drawn from inside the class the implementation already
    /// covered, so it could only ever agree with the implementation — round-3
    /// WR-03's tautology, recurring at a third level. This test moves the
    /// SAMPLING instead of adding literals: it sweeps every code point Rust
    /// admits, and its oracle is `unicode-properties` (unicode-rs), a SECOND
    /// independently maintained derivation of the same standard. The production
    /// class reads `icu_properties` (ICU4X) and nothing else. Neither reads the
    /// other, so a subset in either goes red against the other.
    ///
    /// **Direction, stated so the claim is not inherited as wider than it is.**
    /// This asserts one implication — every `General_Category=Format` code point
    /// the oracle names is inside the production class. The reverse containment
    /// is deliberately NOT asserted: the class is `Cf` UNION
    /// `Default_Ignorable_Code_Point`, and default-ignorable is legitimately
    /// wider than `Cf`. Over-detection is bounded by
    /// [`visible_characters_are_outside_the_class`] instead.
    ///
    /// **The non-vacuity floor is the point of `format_seen`, and it ships.** An
    /// implication over a filtered set is vacuously TRUE when the filter matches
    /// nothing. With the `general-category` feature off, the trait import wrong,
    /// or the `u32 -> char` scope mistaken, this loop yields zero Format
    /// characters and passes green forever while asserting nothing — which is
    /// precisely the failure shape six rounds of fixtures had. So the corpus
    /// must prove it ARRIVED. Derivation of the bound, measured rather than
    /// recalled: Unicode 15.0 assigns 170 `Cf` code points (`python3 -c "import
    /// unicodedata; ..."` over `range(0x110000)`, `unidata_version 15.0.0`); the
    /// tag block `U+E0020..U+E007F` alone is 96 of them. 150 sits ~12% under
    /// that. The margin only ever needs to absorb a crate pinned to an OLDER
    /// Unicode than the measurement, because the standard never un-assigns a
    /// code point — the count is monotonically non-decreasing, so a future
    /// `cargo update` can only push it further above the floor. Do NOT tighten
    /// this to an exact count or add an upper bound: that WOULD flake on the
    /// next Unicode release, and a green-to-red on a routine dependency refresh
    /// is how a floor gets deleted instead of investigated.
    #[test]
    fn every_format_character_the_standard_names_is_inside_the_class() {
        use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

        let mut format_seen = 0_usize;
        for cp in 0_u32..=0x10_FFFF {
            let Some(c) = char::from_u32(cp) else {
                continue; // surrogate range — not a `char`
            };
            if c.general_category() != GeneralCategory::Format {
                continue;
            }
            format_seen += 1;
            assert!(
                is_invisible_formatting_char(c),
                "U+{cp:04X} is General_Category=Format per `unicode-properties` \
                 and must be inside the production class; the production class \
                 is a SUBSET of the standard's, which is the defect that \
                 survived six rounds"
            );
        }

        assert!(
            format_seen >= 150,
            "the oracle produced only {format_seen} Format characters, so this \
             sweep proved nothing — an implication over an empty set is \
             vacuously true. Unicode 15.0 assigns 170. Look at the \
             `unicode-properties` dev-dependency's `general-category` feature \
             and at this loop's iteration scope."
        );
    }

    /// The half of the class with **no second machine oracle**, pinned by named
    /// members with the gap disclosed.
    ///
    /// `Default_Ignorable_Code_Point` is not `General_Category`, and the
    /// `unicode-properties` dev-dependency does not expose it, so the sweep
    /// above cannot cover this half. These are named members instead, which
    /// carries the weakness this phase keeps paying for: a subset bug in a
    /// default-ignorable code point NOT listed here is invisible to every test
    /// in this tree. Disclosed rather than papered over (Premise 7a) — the `Cf`
    /// sweep is the load-bearing control, and this is a supplement to it.
    #[test]
    fn named_default_ignorable_members_beyond_cf_are_inside_the_class() {
        for c in [
            '\u{034f}',  // COMBINING GRAPHEME JOINER
            '\u{fe00}',  // VARIATION SELECTOR-1
            '\u{fe0f}',  // VARIATION SELECTOR-16
            '\u{e0100}', // VARIATION SELECTOR-17
            '\u{e01ef}', // VARIATION SELECTOR-256
            '\u{115f}',  // HANGUL CHOSEONG FILLER
            '\u{1160}',  // HANGUL JUNGSEONG FILLER
            '\u{3164}',  // HANGUL FILLER
            '\u{ffa0}',  // HALFWIDTH HANGUL FILLER
            '\u{17b4}',  // KHMER VOWEL INHERENT AQ
            '\u{17b5}',  // KHMER VOWEL INHERENT AA
            '\u{180b}',  // MONGOLIAN FREE VARIATION SELECTOR ONE
            '\u{180d}',  // MONGOLIAN FREE VARIATION SELECTOR THREE
        ] {
            assert!(
                is_invisible_formatting_char(c),
                "U+{:04X} is a Default_Ignorable_Code_Point and renders as \
                 nothing, so it is in the class even though it is not Cf",
                c as u32
            );
        }
    }

    /// **The over-detection bound, and it deliberately does not stop at ASCII.**
    ///
    /// A class that swallowed everything would satisfy every refusal pin in this
    /// tree forever while making the tool unusable. Bounding that below `0x7F`
    /// is not enough, and the reason is worth stating because the gap is
    /// otherwise invisible: a derivation bug — a wrong `icu_properties`
    /// accessor, an inverted set query, a `Default_Ignorable` set confused for
    /// something wider — that made accented Latin, CJK, Arabic or Devanagari
    /// "invisible" would silently refuse every legitimate non-Latin `--goal`,
    /// and no committed test in the tree would see it. That is a
    /// released-surface NARROWING past the one D-19-1 discloses (which is about
    /// the emptiness class widening, not about visible script becoming
    /// unusable), and this plan does not get to ship an undisclosed one. So the
    /// bound runs one named member per major script family.
    #[test]
    fn visible_characters_are_outside_the_class() {
        for cp in 0x20_u32..=0x7E {
            let c = char::from_u32(cp).expect("ASCII printable is a char");
            assert!(
                !is_invisible_formatting_char(c),
                "U+{cp:04X} ({c:?}) is an ASCII printable and must never be \
                 judged invisible"
            );
        }

        for (c, script) in [
            ('\u{e9}', "Latin e-acute"),
            ('\u{f6}', "Latin o-umlaut"),
            ('\u{65e5}', "CJK unified ideograph"),
            ('\u{d55c}', "Hangul syllable"),
            ('\u{646}', "Arabic noon"),
            ('\u{915}', "Devanagari ka"),
            ('\u{5d0}', "Hebrew alef"),
            ('\u{3b1}', "Greek alpha"),
            ('\u{439}', "Cyrillic short i"),
            ('\u{e01}', "Thai ko kai"),
        ] {
            assert!(
                !is_invisible_formatting_char(c),
                "U+{:04X} is {script} — a character a reader can see. Judging it \
                 invisible would refuse every `--goal` written in that script, \
                 which is a narrowing this plan never disclosed",
                c as u32
            );
        }
    }

    /// **The acceptance direction for free text, which the widened class must
    /// not cost.**
    ///
    /// ZWJ and ZWNJ are load-bearing in real scripts, and a `--goal` is free
    /// text rather than an identity — so a value carrying them BESIDE visible
    /// content is a legitimate instruction and stays accepted (D-19-6). Free
    /// text keeps the deny-list direction precisely so this holds; identities
    /// get the alphabet instead.
    #[test]
    fn free_text_keeps_its_joiners() {
        for joined in ["ka\u{200d}ta", "x\u{200c}y", "\u{200d}x\u{200c}"] {
            assert!(
                carries_visible_content(joined),
                "{joined:?} carries visible content beside a joiner and is a \
                 legitimate goal — refusing it would break scripts in which \
                 ZWJ/ZWNJ are load-bearing"
            );
        }

        for accepted in ["x", " x ", "\u{200b}x", "x\u{feff}", "20", "2.1", "/gsd:progress"] {
            assert!(
                carries_visible_content(accepted),
                "{accepted:?} was accepted before D-19-1 widened the class and \
                 must still be accepted; the widening is about values with \
                 NOTHING visible in them"
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
