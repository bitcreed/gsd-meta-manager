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
///
/// **THE FREE-TEXT RESIDUAL, disclosed because this phase's own standard is to
/// name a residual with its direction and this doc did not** (round 8;
/// verification pass 8's warning). Everything above names the class this covers.
/// This names what falls OUTSIDE it.
///
/// Free text is judged by a **deny-list** over the derived invisible class, and a
/// deny-list has an outside. A value of ONE blank-rendering character that is not
/// in the class is therefore accepted in `--goal` and `--command` and renders as
/// nothing. Round 8 measured four such witnesses against this built library, with
/// oracles independent of this tree — one from the symbol category
/// (`U+2800` BRAILLE PATTERN BLANK), one private-use (`U+E000`), one unassigned
/// (`U+0378`) and one lone combining mark (`U+0301`). All four returned `true`
/// here. **Direction: over-permissive, in FREE TEXT ONLY.**
///
/// **What bounds it.** None of the four can reach an *identity*: identities are
/// judged by the finite alphabet ([`is_identity_char`]), which refuses all four —
/// measured in the same run, `is_plain_path_component` answered `false` for every
/// one. So the residual is display honesty in a string the user typed themselves,
/// not the two-values-one-name harm this phase's earlier rounds reproduced end to
/// end. `tests::the_free_text_emptiness_residual_is_accepted_and_cannot_reach_an_identity`
/// pins both halves, so this paragraph cannot quietly go stale in either
/// direction.
///
/// **What bounds the deny-list's own completeness — two things, both already
/// disclosed nearby rather than newly claimed.** (1) The pinned Unicode version:
/// the class is exactly as current as `icu_properties`' data, and a code point
/// unassigned today that a later release makes `Cf` is accepted in free text
/// until the next refresh — see [`is_invisible_formatting_char`]. (2) The
/// thirteen HAND-NAMED `Default_Ignorable` members in this module's test module:
/// the `unicode-properties` oracle does not expose that property, so that half of
/// the class has NO second machine oracle, and a subset bug in an unlisted
/// default-ignorable code point is invisible to every test in this tree.
///
/// **The predicate is deliberately NOT changed.** An ASCII allow-list in free
/// text would refuse legitimate script, which is the trade
/// [`is_identity_char`]'s doc already argues and records — the allow-list belongs
/// at identity seams and the deny-list belongs here.
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
/// Crate-private on purpose: the class is a class, not a public API. Every
/// judgment in this module reads it from here so a widening lands in all of them
/// at once.
///
/// It is `pub(crate)` rather than module-private for exactly one consumer:
/// `ui::screens::render_escape_guard`'s behavioural probe asserts that ZERO
/// characters in a rendered terminal buffer satisfy this predicate. That
/// assertion has to consult the ONE production spelling of the class — a probe
/// with its own copy would be a second spelling that can silently disagree,
/// which is the defect this function exists to remove.
pub(crate) fn is_invisible_formatting_char(c: char) -> bool {
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

/// Render `value` for a terminal with every invisible-class character replaced
/// by its visible `U+XXXX` form.
///
/// **A RENDER-side defence for entries an older build accepted** (D-19-5). New
/// identities cannot carry these bytes at all — [`is_identity_char`] refuses
/// them at registration — so this is not the primary control and must not be
/// mistaken for one. It exists because refusing future registrations does
/// nothing about the rows already sitting in a user's `config.json`, and those
/// rows still render.
///
/// Pass 7 reproduced what that costs at the binary level: a legacy key
/// `"gsd-\u{202e}nur"` printed by `list` as `gsd-run`, because `U+202E`
/// (RIGHT-TO-LEFT OVERRIDE) reverses everything after it. That is Trojan Source
/// (CVE-2021-42574) inside the tool's own project list — the operator reads one
/// project's name and acts on another. Escaping makes the spoof VISIBLE instead
/// of invisible: the row reads `gsd-U+202Enur`, which is ugly and honest.
///
/// **Only the invisible class is escaped.** Every other character passes
/// through unchanged, so a legacy non-ASCII alias still renders as itself rather
/// than as a wall of code points — this is a legibility defence, not a
/// transliteration. Nothing persisted changes; this is display only.
pub fn display_identity(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        if is_invisible_formatting_char(c) {
            out.push_str(&format!("U+{:04X}", c as u32));
        } else {
            out.push(c);
        }
    }
    out
}

// ---------------------------------------------------------------------------
// The CONTROL class, the composition of the two classes, and the two carrier
// types — the round-9 inversion (D-21-7 … D-21-10)
// ---------------------------------------------------------------------------

/// The escape character, stripped unconditionally by
/// [`strip_terminal_controls`].
pub(crate) const ESC: char = '\u{1b}';

/// What every other C0 control character, `DEL` and the C1 block is replaced
/// with: `·`.
///
/// A `\u{…}` escape rather than the literal glyph, following the house rule that
/// no raw glyph appears in source (`ui::screens::normal.rs:69-74`).
pub(crate) const CONTROL_REPLACEMENT: char = '\u{00b7}';

/// Spaces one tab expands to.
pub(crate) const TAB_WIDTH: usize = 4;

/// The ONE production spelling of the terminal-CONTROL class.
///
/// **This is the class, and it is deliberately NOT the display cap** (D-21-9).
/// These rules used to live inside `ui::screens::sanitize_render_line`, welded
/// to a 512-character truncation. The round-8 review proposed composing
/// `display_identity(&sanitize_render_line(v))` to answer both classes at once;
/// that one-liner is wrong as written, because it would silently cap every CLI
/// echo, every error message and every rendered name at
/// `DRIVER_OUTPUT_LINE_CELLS`. Separating the class from the cap is what lets
/// ONE composition ([`render_for_terminal`]) serve both, and
/// `ui::screens::sanitize_render_line` is now this function plus that cap.
///
/// The rules, in this order, carried across unchanged in behaviour:
///
/// 1. **`ESC` (`0x1B`) is stripped unconditionally.** This is the single
///    highest-value rule in the boundary: without it, text read off disk can
///    emit ANSI/OSC sequences that repaint the screen, forge a status line, move
///    the cursor, or set the window title. Stripping the introducer is what makes
///    the rest of a sequence inert text.
/// 2. `\t` expands to [`TAB_WIDTH`] spaces.
/// 3. Every other C0 control character (`0x00`–`0x1F`), `DEL` (`0x7F`) **and the
///    whole C1 block (`0x80`–`0x9F`)** is replaced with
///    [`CONTROL_REPLACEMENT`] — present, visible, and harmless.
///
///    **C1 is not an afterthought and rule 1 does not cover it** (WR-06).
///    `U+009B` is the single-character CSI, `U+009D` is OSC and `U+0090` is DCS:
///    each is a one-codepoint equivalent of an `ESC`-led introducer, so stripping
///    `ESC` alone leaves the same capability reachable by another spelling.
///    ratatui writes each grapheme's bytes straight to the terminal, so these
///    arrive as `0xC2 0x9B` and terminals that honour 8-bit controls decoded from
///    UTF-8 (xterm without `allowC1Printable`, among others) treat what follows
///    as a control sequence — reinstating exactly the repaint-the-screen and
///    forge-a-status-line capability rule 1 exists to remove.
///
/// **What it does NOT do.** It does not touch the INVISIBLE-FORMATTING class:
/// `Cf` ∪ `Default_Ignorable` is not `Cc`, so `U+202E`, `U+00AD` and `U+E0041`
/// pass through here untouched. That is [`display_identity`]'s question, and
/// [`render_for_terminal`] is the one place the two are composed.
pub(crate) fn strip_terminal_controls(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch == ESC {
            continue;
        }
        if ch == '\t' {
            for _ in 0..TAB_WIDTH {
                out.push(' ');
            }
        } else if (ch as u32) < 0x20 || ('\u{7f}'..='\u{9f}').contains(&ch) {
            out.push(CONTROL_REPLACEMENT);
        } else {
            out.push(ch);
        }
    }
    out
}

/// A string that has been through [`render_for_terminal`], and the ONLY string
/// type in this tree that is both escaped and `Into<Cow<'static, str>>`.
///
/// **The ergonomic asymmetry is the mechanism** (D-21-8). [`Untrusted`] holds
/// the raw bytes and implements none of the string conversions, so it cannot be
/// interpolated or handed to a ratatui sink at all. This type implements them,
/// so at a render site `Span::raw(v.shown())` and `format!("{}", v.shown())`
/// compile with no ceremony while the raw path costs a deliberate,
/// greppable `as_raw_for_logic_only()`. The short path is the escaped one.
///
/// **Which sinks take it directly, measured rather than asserted.** `Span::raw`
/// and `Span::styled` take `Into<Cow<'_, str>>` and therefore take this
/// directly. `Block::title` takes `Into<Line>`, and this type deliberately does
/// NOT implement that: `text.rs` has no ratatui dependency and gaining one to
/// shorten a call would put a UI crate under the module every identity seam in
/// the tree consults. At those sinks the call is `format!("{}", v.shown())` or
/// `Span::raw(v.shown())`, both of which go through [`Display`](std::fmt::Display)
/// and [`Into<Cow>`](std::borrow::Cow) respectively.
///
/// **What being `Rendered` does and does not claim.** It is a statement about
/// which transformations were APPLIED — control stripping and invisible-class
/// escaping — and not a proof that the underlying value is harmless. Homoglyphs
/// pass through untouched (Unicode TR39 is a separate question,
/// [`carries_invisible_formatting`]'s doc records why), and a value that is a
/// lie in plain ASCII is still a lie after escaping.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered(String);

impl Rendered {
    /// The only constructor, and it is **module-private on purpose**: the only
    /// way to obtain a `Rendered` outside this module is to escape something,
    /// through [`render_for_terminal`] or [`Untrusted::shown`].
    fn new(escaped: String) -> Self {
        Self(escaped)
    }
}

impl std::fmt::Display for Rendered {
    /// Uses [`Formatter::pad`], **not** `write_str`, so that format specs are
    /// honoured. `write_str` silently ignores width/fill/alignment, which made
    /// `{:<20}` on a `Rendered` emit no padding at all — `list` printed
    /// `clean /tmp` where it had printed `clean                /tmp` before the
    /// carrier landed. Found by binary measurement during 21-24; no test
    /// asserted `list`'s column alignment, so nothing went red for it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&self.0)
    }
}

impl AsRef<str> for Rendered {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<Rendered> for std::borrow::Cow<'static, str> {
    fn from(value: Rendered) -> Self {
        std::borrow::Cow::Owned(value.0)
    }
}

impl From<Rendered> for String {
    fn from(value: Rendered) -> Self {
        value.0
    }
}

/// The ONE composition of the two classes, resolved once so that no consumer
/// re-decides which halves apply (WR-01, WR-02).
///
/// **Two classes, two predicates, neither subsuming the other.**
/// [`strip_terminal_controls`] answers the `ESC` / C0 / `DEL` / C1 *control*
/// question; [`display_identity`] answers the *invisible-formatting* question
/// over `General_Category=Cf` ∪ `Default_Ignorable_Code_Point`. `Cf` ∪
/// `Default_Ignorable` is not `Cc`, and `ESC` is `Cc` and in neither of the
/// first two — so a site that applies only one of them is open in the other
/// direction. `src/ui/screens/driver.rs:874-878` already states exactly this;
/// what was missing was a single place that composed them, which is why round 8
/// found four classes of site where one half had been applied and the other had
/// not.
///
/// The order is control-stripping FIRST. [`display_identity`] emits `U+XXXX`
/// spellings made of ASCII, which the control pass would leave alone anyway, but
/// stripping first means the invisible-class pass never sees a `\t` that has not
/// yet become spaces.
///
/// **The deliberate second composition, named so it is not mistaken for drift.**
/// `ui::screens::sanitize_render_line` is this same control class plus the
/// `DRIVER_OUTPUT_LINE_CELLS` display cap, and `driver.rs` / `driver_confirm.rs`
/// compose `display_identity(&sanitize_render_line(..))` because they draw agent
/// prose that must be capped. The ONLY difference between that composition and
/// this one is the cap, which is pinned by
/// `ui::screens::tests::the_capped_and_uncapped_compositions_agree_below_the_cap`
/// rather than argued here.
pub fn render_for_terminal(value: &str) -> Rendered {
    Rendered::new(display_identity(&strip_terminal_controls(value)))
}

/// A string this build did NOT author, in a type that cannot reach a terminal
/// cell unescaped.
///
/// **The round-9 inversion, and what it generalises** (D-21-8). Round 8 built
/// exactly the right shape in [`crate::registry::LegacyRegistryKey`] — no
/// `Display`, two accessors named after the questions they answer — and then
/// applied it to ONE call site while every data carrier in the tree kept bare
/// `String` fields. The shape was right; the application was one site. This type
/// is that correction: it is the general carrier, and `LegacyRegistryKey` is the
/// argv-lookup variant of it.
///
/// **Why a carrier and not a ban on a ratatui API.** `Span::raw` takes
/// `Into<Cow<'_, str>>` and `String: Into<Cow<'_, str>>` is a standard-library
/// impl this project cannot un-implement, so there is no way to forbid the raw
/// sink globally and any claim to have done so would be false. Measured at
/// round-9 HEAD: 129 `Span::raw` and 198 `Span::styled` calls under `src/`. The
/// lever gates NONE of them. What it gates is the values: a field of this type
/// cannot be interpolated, coerced, or handed to a sink at all, so the compiler
/// — not a reader working through a list of sites — names every consumer when a
/// carrier is retyped.
///
/// It implements:
///
/// * **no `Display`** — so it cannot be interpolated,
/// * **no `AsRef<str>`, no `Deref`, no `Borrow<str>`, no `Into<Cow<'_, str>>`**
///   — so it cannot be coerced into one either,
/// * **no `serde` traits** — persistence goes through
///   [`as_raw_for_logic_only`](Self::as_raw_for_logic_only), which is a choice
///   visible in a diff,
/// * **no derived `Debug`.** The derive is what re-opened `{:?}` on
///   `LegacyRegistryKey` (WR-04); the hand-written impl below prints the ESCAPED
///   form, so a raw invisible character cannot reach a log line, a panic
///   message, an `anyhow` chain, or the `#[derive(Debug)]` of any struct that
///   contains one of these.
///
/// # CORRECTED 2026-08-27 (21-27, WR-01): the count named neither the list nor the certificate
///
/// **The sentence this corrects, verbatim:** *"Those five absences are
/// certified by `tests::an_untrusted_carrier_implements_none_of_the_string_conversions`,
/// a runtime control observed RED by planting the impls."*
///
/// It said **five**; the bullets above enumerate **six** absent conversions
/// (`Display`, `AsRef<str>`, `Deref<Target = str>`, `Borrow<str>`,
/// `Into<Cow<'_, str>>`, `serde`); and the control certified **three**. Three
/// different numbers for one claim, and the gap was not cosmetic:
/// `Deref<Target = str>` was among the uncertified three, and it is the one
/// impl that silently rewrites code that is already written — auto-deref
/// restores the raw path at every call site in the tree at once, invisible in a
/// diff, with every test still green.
///
/// **What is true now:** all **SIX** absences above are certified by
/// `tests::an_untrusted_carrier_implements_none_of_the_string_conversions`, a
/// runtime control in which each absence has its own `String` presence arm (so
/// a broken probe fails loudly instead of certifying nothing) and each was
/// observed RED by planting the impl — not by a comment quoting a compile error
/// somebody once saw, which is precisely what `LegacyRegistryKey` had. The
/// hand-written `Debug` is pinned separately by
/// `tests::a_carrier_debug_never_carries_an_invisible_character` and is not one
/// of the six.
///
/// Six is what the doc claims and what the control certifies; it is **not** the
/// closed set of ways a raw `&str` could escape. `PartialEq<str>`,
/// `Into<String>` and any trait a dependency adds by blanket impl are not
/// probed — **under-detection, silent**, and named in that test's own doc.
///
/// Two accessors, each named after the question it answers:
/// [`as_raw_for_logic_only`](Self::as_raw_for_logic_only) for lookups,
/// comparisons, map keys, path segments, **argv elements** and persistence;
/// [`shown`](Self::shown) for what a human reads.
///
/// **Two accessors, THREE questions** (21-27, CR-01). This line used to say
/// *"subprocess arguments"*, which collapsed an argv element — inert, because
/// nothing parses it — with a fragment spliced into a program an interpreter
/// will parse, which is not inert at all. The third question gets no accessor
/// of its own on purpose; it gets a rule (`shell_command_fragment`) and a
/// census. See [`as_raw_for_logic_only`](Self::as_raw_for_logic_only)'s doc.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Untrusted(String);

impl Untrusted {
    /// Wrap a string that came from outside this build. **No judgment is applied
    /// and none may be added.**
    ///
    /// A carrier that refused values could not carry the third-party and legacy
    /// data it exists for — a hostile commit subject must still be displayable,
    /// and a registry key an older build accepted must still be removable
    /// (D-17-3). Judging happens at the seams that create identities
    /// ([`is_identity_char`]), never here.
    pub fn from_untrusted_source(raw: String) -> Self {
        Self(raw)
    }

    /// The raw bytes. **Three questions, not two** — and the third one has no
    /// site left in this codebase.
    ///
    /// Deliberately unattractive to type. Reaching for it is a choice a reviewer
    /// sees in a diff, which is what a bare `String` field never was.
    ///
    /// # CORRECTED 2026-08-27 (21-27, CR-01): this doc used to collapse two different questions into one phrase
    ///
    /// **The sentence this corrects, verbatim:** *"The raw bytes, for lookups,
    /// comparisons, map keys, path segments, subprocess arguments and
    /// persistence ONLY."*
    ///
    /// It listed *subprocess arguments* as one accepted use. Two genuinely
    /// different things hide under that phrase, and only one of them is inert.
    /// **That collapse is why round 9's compiler-named-sites methodology — the
    /// strongest mechanism this phase has built — could not see CR-01**: the
    /// Sessions-tab resume in `ui::screens::detail` interpolated a
    /// `/proc`-scraped session id into a program string handed to a command
    /// interpreter, and because the value was, *in this doc's own words*, a
    /// subprocess argument, the site was never retyped away from being answered.
    /// The vocabulary was short by one question, so the site looked answered.
    ///
    /// # The three questions
    ///
    /// 1. **A lookup** — a map key, a comparison, a path segment, a persisted
    ///    field. Inert: nothing parses it.
    /// 2. **An argv element** — one element of a vector handed to
    ///    [`std::process::Command`], which the kernel passes to `execve`
    ///    unparsed. Also inert, and for the same reason: nothing parses it.
    /// 3. **A fragment of a program an interpreter will parse** — a string
    ///    interpolated into something handed to `sh -c` or an equivalent.
    ///    **NOT inert.** Every metacharacter in the value is a candidate token,
    ///    so a quote, a semicolon, a backtick, a dollar-parenthesis or a
    ///    newline is code.
    ///
    /// # The rule: `shell_command_fragment`
    ///
    /// **A value reaching a subprocess goes in as its own argv element, and
    /// never into a program string an interpreter will parse.** Quoting for the
    /// interpreter is a correct technique and it is the WEAKER answer: it keeps
    /// a parser in the path, so the property depends on the escaper being right
    /// about every metacharacter of every interpreter. Deleting the interpreter
    /// makes the question not arise. If an external program genuinely cannot be
    /// driven without a program string, that program is reported as an
    /// unsupported case by name rather than silently handled by quoting.
    ///
    /// There is deliberately **no third accessor** for question 3. Minting an
    /// `as_raw_for_a_shell` would mint a supported way to do the unsafe thing.
    /// The rule is checked instead of offered: see
    /// [`no_executable_line_under_src_hands_an_interpreter_an_interpolated_program`](tests::no_executable_line_under_src_hands_an_interpreter_an_interpolated_program),
    /// the census that reports any executable line under `src/` where question 3
    /// would have to be asked. It returns zero, and it was observed red by
    /// planting.
    pub fn as_raw_for_logic_only(&self) -> &str {
        &self.0
    }

    /// What a human READS: both classes escaped, via [`render_for_terminal`].
    pub fn shown(&self) -> Rendered {
        render_for_terminal(&self.0)
    }
}

/// **Hand-written, not derived** (WR-04, T-21-23-05).
///
/// `#[derive(Debug)]` on a carrier re-opens the raw path through `{:?}` — a log
/// line, a panic message, an `anyhow` chain, or the derived `Debug` of any
/// struct holding one. This prints [`shown`](Untrusted::shown), so the escape
/// travels with the value into every one of those. Pinned over
/// `LOOK_ALIKE_PAIRS` by
/// `tests::a_carrier_debug_never_carries_an_invisible_character`, in both
/// directions.
impl std::fmt::Debug for Untrusted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Untrusted({:?})", self.shown().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **A control for `Display for Rendered`, in both directions.**
    ///
    /// `Rendered`'s `Display` must route through `Formatter::pad`, not
    /// `write_str`. `write_str` ignores width/fill/alignment entirely, so
    /// `{:<20}` emitted the bare string and `list`'s columns collapsed. That
    /// regression shipped through wave 1 and was found by binary measurement
    /// in 21-24, not by a test — nothing in the tree asserted alignment.
    ///
    /// Planted red: reverting the impl body to `f.write_str(&self.0)` fails the
    /// padded assertion below. The unpadded assertion is the other direction —
    /// it fails if `pad` were ever given a spurious default width, so this
    /// cannot be satisfied by a formatter that pads unconditionally.
    #[test]
    fn rendered_display_honours_the_format_spec_in_both_directions() {
        let r = render_for_terminal("clean");
        assert_eq!(
            format!("{r:<20}"),
            "clean               ",
            "width/alignment must be honoured — `write_str` silently drops the spec"
        );
        assert_eq!(
            format!("{r:>8}"),
            "   clean",
            "right alignment must be honoured too, not just left"
        );
        assert_eq!(
            format!("{r}"),
            "clean",
            "with no spec the output must be exactly the escaped string, unpadded"
        );
    }

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

    /// **The Trojan Source reproduction, as a unit** (D-19-5).
    ///
    /// `"gsd-\u{202e}nur"` is the legacy key pass 7 measured `list` printing as
    /// `gsd-run`. The escaped form is what a reader can act on.
    #[test]
    fn an_invisible_character_renders_as_its_visible_code_point() {
        assert_eq!(
            display_identity("gsd-\u{202e}nur"),
            "gsd-U+202Enur",
            "a bidi override must render as a marker a reader can see — printed \
             raw, this row reads `gsd-run` and the operator acts on the wrong \
             project"
        );
        assert_eq!(
            display_identity("demo"),
            "demo",
            "and a clean alias must pass through untouched — this is a \
             legibility defence, not a transliteration"
        );
        assert_eq!(
            display_identity("d\u{e9}mo"),
            "d\u{e9}mo",
            "a legacy non-ASCII alias is VISIBLE, so it renders as itself; only \
             the invisible class is escaped"
        );
        assert_eq!(
            display_identity("a\u{200b}b\u{e0041}c"),
            "aU+200BbU+E0041c",
            "every invisible-class character is escaped, including one from \
             outside the pre-round-7 ranges"
        );
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

    /// **The free-text residual, RECORDED as a measurement rather than left to a
    /// sentence** (round 8; verification pass 8's warning on
    /// [`carries_visible_content`]).
    ///
    /// Free text is judged by a DENY-LIST over the derived invisible class, and a
    /// deny-list has an outside. Every value below renders as nothing and is
    /// nonetheless ACCEPTED as a `--goal` or a `--command`, because none of its
    /// characters is `General_Category=Cf` or a `Default_Ignorable_Code_Point`.
    /// Direction: **over-permissive, in free text only.**
    ///
    /// The second assertion in each row is the BOUND: the same value cannot reach
    /// an identity, because identity seams judge with the finite alphabet
    /// ([`is_identity_char`]) and it refuses all four. So this residual is display
    /// honesty in a string the user typed themselves — not the two-values-one-name
    /// harm this phase's earlier rounds reproduced end to end.
    ///
    /// **This test RECORDS the residual; it does not bless it.** A round that
    /// closes it must change this test and [`carries_visible_content`]'s
    /// disclosure in the SAME commit. That coupling is the point of pinning it
    /// here rather than only describing it in prose.
    #[test]
    fn the_free_text_emptiness_residual_is_accepted_and_cannot_reach_an_identity() {
        for (value, what) in [
            ("\u{2800}", "BRAILLE PATTERN BLANK (So, a symbol - not a format char)"),
            ("\u{e000}", "the first private-use code point (Co)"),
            ("\u{0378}", "an unassigned code point (Cn) at the pinned version"),
            ("\u{0301}", "COMBINING ACUTE ACCENT (Mn), with no base character"),
        ] {
            let accepted_in_free_text = carries_visible_content(value);
            let can_name_an_identity = crate::journal::is_plain_path_component(value);
            println!(
                "{what}\n  carries_visible_content   -> {accepted_in_free_text}\n  \
                 is_plain_path_component   -> {can_name_an_identity}"
            );
            assert!(
                accepted_in_free_text,
                "{value:?} ({what}) is no longer accepted in free text. That is a \
                 released-surface NARROWING and it may well be right — but \
                 `carries_visible_content`'s doc discloses this value as an \
                 accepted residual, so the disclosure has to be corrected in the \
                 same commit or the doc starts overclaiming in the other direction."
            );
            assert!(
                !can_name_an_identity,
                "{value:?} ({what}) reached an identity seam. The whole reason \
                 this residual is disclosed as ACCEPTABLE is that the finite \
                 alphabet refuses it, so a run directory, envelope root, \
                 credential scope or phase token can never carry it."
            );
        }
    }

    // -----------------------------------------------------------------------
    // The spelling census: `is_identity_char`'s ONE-spelling claim, made
    // checkable instead of believed
    // -----------------------------------------------------------------------

    /// The needle, assembled at RUNTIME from two halves that are meaningless
    /// apart — the anti-self-match idiom this tree already uses at
    /// `tests/spawn_seam_guard.rs`'s `DEGENERATE_WITNESS_HEADS` and at
    /// `ui::screens::render_escape_guard`'s `IMPL_HEAD`/`IMPL_TAIL`.
    ///
    /// The census walks `src/`, and `src/text.rs` is under `src/`. Spelled as one
    /// literal here, this const's own line would be a hit and the census would
    /// count itself.
    const ALPHABET_CLAUSE_HEAD: &str = "'.' |";
    /// The tail of [`ALPHABET_CLAUSE_HEAD`].
    const ALPHABET_CLAUSE_TAIL: &str = " '_' | '-')";

    /// How many physical lines a wrapped alternation may span before the join
    /// gives up.
    const ALPHABET_CLAUSE_JOIN_LINES: usize = 3;

    /// The **bare single-character literal atoms** on `line`, sorted and
    /// deduplicated — the normal form the census matches on (WR-05).
    ///
    /// # Why a normal form and not one exact byte string
    ///
    /// The census used to search each executable line for the single byte string
    /// [`ALPHABET_CLAUSE_HEAD`] + [`ALPHABET_CLAUSE_TAIL`]. That made the scan's
    /// power depend on the ORDER the author happened to type the arms in and on
    /// the spaces they happened to leave: `matches!(c, '-' | '_' | '.')` is the
    /// same character set and was invisible. The defect the census exists to
    /// catch — `envelope::advisory::is_plain_component`, which respelled the
    /// identity alphabet and used it to gate GitHub `owner`/`repo` segments
    /// interpolated into a request path — would have been invisible too, had its
    /// author typed the arms in any other order. A guard whose power depends on
    /// a coin flip is not a guard.
    ///
    /// # What the normal form is, exactly
    ///
    /// Whitespace is stripped, then every `'X'` literal that is not an endpoint
    /// of a `..=` range is collected, sorted and deduplicated. So the identity
    /// alphabet's own clause — which spells `'A'..='Z' | 'a'..='z' | '0'..='9'`
    /// as ranges and `'.' | '_' | '-'` as bare atoms — normalises to the same
    /// three atoms as a copy that writes `c.is_ascii_alphanumeric()` and then the
    /// same three punctuation arms in any order. That equivalence is what makes
    /// the census see the WR-03 respelling and its reorderings alike.
    ///
    /// # The over-matching risk this creates, and how it is bounded
    ///
    /// Normalising loosens the needle, so over-matching becomes the new failure
    /// direction. The bound is that the comparison is an **equality on the whole
    /// line's atom set**, not a containment: `advisory::default_branch_of`
    /// additionally admits `'/'`, so its atoms are four rather than three and it
    /// is excluded. That exclusion is a genuinely different question — a branch
    /// name legitimately carries a separator and is not an identity — and it is
    /// ASSERTED by
    /// [`the_normalized_needle_still_excludes_the_branch_name_set`](the_normalized_needle_still_excludes_the_branch_name_set)
    /// rather than assumed.
    ///
    /// **Its own residual, with the direction.** Because the comparison is an
    /// equality over the WHOLE line, an executable line that spells the clause
    /// AND some unrelated character literal beside it normalises to a larger set
    /// and is NOT counted. **Under-detection, silent.** What bounds that is the
    /// delegation — one function every seam calls — exactly as it bounds the
    /// non-textual constructions named below.
    /// Whether `logical` is syntactically UNFINISHED, so the next physical line
    /// is part of the same clause.
    ///
    /// **This predicate is what keeps the anti-self-match property alive under
    /// joining, and it was added because joining broke it.** Measured, not
    /// reasoned about: with an unconditional join, the census reported TWO sites
    /// — `src/text.rs:251` and `src/text.rs:1086` — because
    /// [`ALPHABET_CLAUSE_HEAD`]'s own line (atoms `['.']`) joined onto
    /// [`ALPHABET_CLAUSE_TAIL`]'s (atoms `['-', '_']`) and the union is exactly
    /// the clause. The two halves are meaningless apart *as strings*, but they
    /// were not meaningless apart as ADJACENT SOURCE LINES.
    ///
    /// A line is unfinished when an alternation is left open (`|` at the end) or
    /// a delimiter is still unclosed. `const ALPHABET_CLAUSE_HEAD: &str = "'.' |";`
    /// is finished on both counts — it ends with `;` and its parens balance — so
    /// it never starts a join.
    fn continues_onto_the_next_line(logical: &str) -> bool {
        let trimmed = logical.trim_end();
        if trimmed.ends_with('|') {
            return true;
        }
        let opened = trimmed.matches('(').count();
        let closed = trimmed.matches(')').count();
        opened > closed
    }

    fn bare_char_literal_atoms(line: &str) -> Vec<char> {
        let compact: Vec<char> = line.chars().filter(|c| !c.is_whitespace()).collect();
        let mut atoms = Vec::new();
        let mut index = 0;
        while index < compact.len() {
            if compact[index] != '\'' {
                index += 1;
                continue;
            }
            // A bare atom is exactly `'X'`. Anything longer (an escape, a
            // lifetime, an unterminated quote) is not one and is stepped over.
            if index + 2 < compact.len() && compact[index + 2] == '\'' {
                let preceded_by_range = index >= 3
                    && compact[index - 3] == '.'
                    && compact[index - 2] == '.'
                    && compact[index - 1] == '=';
                let followed_by_range = index + 6 <= compact.len()
                    && compact[index + 3] == '.'
                    && compact[index + 4] == '.'
                    && compact[index + 5] == '=';
                if !preceded_by_range && !followed_by_range {
                    atoms.push(compact[index + 1]);
                }
                index += 3;
                continue;
            }
            index += 1;
        }
        atoms.sort_unstable();
        atoms.dedup();
        atoms
    }

    /// Every `.rs` file under `dir`, recursively, as `(relative path, lines)`.
    ///
    /// The recursive `read_dir` shape follows
    /// `ui::screens::render_escape_guard::collect`: an unreadable entry is
    /// skipped rather than panicked on, and paths are relative to
    /// `CARGO_MANIFEST_DIR`.
    fn collect_rs(
        dir: &std::path::Path,
        base: &std::path::Path,
        out: &mut Vec<(String, Vec<(usize, String)>)>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                collect_rs(&path, base, out);
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let relative = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let lines = text
                .lines()
                .enumerate()
                .map(|(index, line)| (index + 1, line.to_string()))
                .collect();
            out.push((relative, lines));
        }
    }

    /// [`is_identity_char`]'s doc claims to be **the** spelling of the identity
    /// alphabet. This makes that claim checkable rather than believed.
    ///
    /// **Why an EQUALITY on a count and not a containment.** WR-03 measured the
    /// claim false: `envelope::advisory::is_plain_component` respelled the same
    /// character set byte-for-byte and used it to gate the GitHub `owner`/`repo`
    /// segments that are then interpolated into a request path. A containment
    /// check ("at least one spelling exists") could never have seen that; only an
    /// equality on the number of executable occurrences can.
    ///
    /// **Committed RED, verbatim, before the delegation.** Against the tree
    /// before `advisory.rs` delegated,
    /// `cargo test --lib -- --ignored exactly_one_executable_spelling`:
    ///
    /// ```text
    /// running 1 test
    /// test text::tests::exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src ... FAILED
    ///
    /// ---- text::tests::exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src stdout ----
    ///
    /// thread 'text::tests::exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src' (1779910) panicked at src/text.rs:810:9:
    /// assertion `left == right` failed: the identity alphabet's character clause is spelled 2 times in executable lines under src/, and `is_identity_char`'s doc claims to be THE one spelling. Sites: ["src/envelope/advisory.rs:569", "src/text.rs:213"]. A second spelling is a boundary that can stop agreeing with the boundary: WR-03 measured exactly that, in `envelope::advisory::is_plain_component`, gating the GitHub owner/repo segments that are interpolated into a request path. The repair is DELEGATION to `crate::text::is_identity_char`, not a softening of the claim.
    ///   left: 2
    ///  right: 1
    ///
    /// test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1055 filtered out; finished in 0.08s
    /// ```
    ///
    /// # CORRECTED 2026-08-27 (21-26, WR-05): the residual named only non-textual copies, and that was incomplete
    ///
    /// **The paragraph this doc used to carry, verbatim:** *"**Its
    /// under-detection direction, named because a textual census has one.** A
    /// THIRD spelling written with a different but equivalent construction — a
    /// `match` with the same arms, a byte-range comparison, an `is_ascii_*`
    /// composition — is invisible to this scan and always will be. What bounds
    /// that residual is the DELEGATION itself (one function every seam calls),
    /// not this census; the census only stops the *textual* copy from being
    /// re-introduced silently. It is deliberately not sold as more than that."*
    ///
    /// That named only NON-textual constructions, and the census's real blind
    /// spot was narrower and worse: it searched each executable line for one
    /// exact byte string, so a **textual** copy typed with the arms in a
    /// different order — `matches!(c, '-' | '_' | '.')` — was invisible too. The
    /// `advisory.rs` defect this census exists to have caught would have been
    /// invisible had its author typed the same three arms in any other order.
    /// The claim "the census stops the textual copy from being re-introduced"
    /// was therefore false for five of the six orderings.
    ///
    /// **What is true now.** The census matches on a NORMAL FORM
    /// ([`bare_char_literal_atoms`]): whitespace is stripped, the bare character
    /// literals are collected, sorted and deduplicated, and the result compared
    /// for EQUALITY against the same normalisation of the identity alphabet's
    /// own clause. Arm order and spacing no longer matter, and a clause wrapped
    /// so one arm ends a line is joined into one logical unit first. Observed
    /// red by planting a reordered copy — and a reordered, wrapped copy — in
    /// `src/driver/liveness.rs`: invisible to the old needle
    /// (`grep -rn` for the exact byte string does not name the file), counted at
    /// two by the new one.
    ///
    /// **What REMAINS invisible, with its direction.** Two things, and both are
    /// under-detection, silent:
    ///
    /// 1. A copy written with a **genuinely different construction** — a `match`
    ///    with the same arms, a byte-range comparison, an `is_ascii_*`
    ///    composition. No character literals, nothing to normalise.
    /// 2. A line that spells the clause **and some unrelated character literal
    ///    beside it**, which normalises to a larger set and fails the equality.
    ///
    /// What bounds both is the DELEGATION itself — one function every seam calls
    /// — not this census. The census's job is only to stop a textual copy from
    /// being re-introduced silently, and it now does that for the whole family of
    /// textual respellings rather than for one ordering of it.
    ///
    /// The nearby set in `advisory::default_branch_of`, which additionally admits
    /// `'/'`, is genuinely a different question — a branch name legitimately
    /// carries a separator and is not an identity — and is excluded because the
    /// comparison is an equality on the whole line's atom set, so four atoms are
    /// not three. **Normalising made over-matching the new failure direction, so
    /// that exclusion is ASSERTED and not assumed:**
    /// [`the_normalized_needle_still_excludes_the_branch_name_set`](the_normalized_needle_still_excludes_the_branch_name_set)
    /// pins it in both directions and additionally runs the live census and
    /// requires that no line of `envelope/advisory.rs` is counted.
    #[test]
    fn exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        collect_rs(&base.join("src"), &base, &mut files);
        assert!(
            !files.is_empty(),
            "the census walked src/ and found no Rust source at all, so a clean \
             result here would be a walk that never looked"
        );
        files.sort_by(|a, b| a.0.cmp(&b.0));

        // The comparison target, still ASSEMBLED AT RUNTIME from two halves that
        // are meaningless apart (WR-05 keeps the anti-self-match property). The
        // two consts above normalise to `['.']` and `['-', '_']` on their own
        // lines — neither equals the joined atom set — so this module's own
        // source cannot become a hit.
        let needle = format!("{ALPHABET_CLAUSE_HEAD}{ALPHABET_CLAUSE_TAIL}");
        let wanted = bare_char_literal_atoms(&needle);
        assert_eq!(
            wanted.len(),
            3,
            "the identity alphabet's punctuation arms are three atoms; the \
             assembled needle {needle:?} normalised to {wanted:?}, so the halves \
             have drifted and this census is now looking for the wrong thing"
        );

        let mut sites: Vec<String> = Vec::new();
        for (path, lines) in &files {
            for (index, (number, line)) in lines.iter().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                let atoms = bare_char_literal_atoms(line);
                if atoms == wanted {
                    sites.push(format!("{path}:{number}"));
                    continue;
                }
                // A PROPER, NON-EMPTY SUBSET is a line that looks like part of
                // the clause, so try joining what follows it — that is how a
                // clause wrapped so one arm ends a line is counted. The subset
                // test is what stops the join double-counting: a line that
                // already matches is counted above and never joined, and a line
                // with no atoms at all (`foo(`) never starts a join, so a
                // matching line cannot be counted once on its own and again
                // inside an enclosing unit.
                if atoms.is_empty() || !atoms.iter().all(|atom| wanted.contains(atom)) {
                    continue;
                }
                let mut logical = line.to_string();
                let mut taken = 1;
                let mut ahead = index;
                while taken < ALPHABET_CLAUSE_JOIN_LINES
                    && ahead + 1 < lines.len()
                    && continues_onto_the_next_line(&logical)
                {
                    ahead += 1;
                    let next = lines[ahead].1.trim();
                    if next.is_empty() {
                        break;
                    }
                    taken += 1;
                    if next.starts_with("//") {
                        continue;
                    }
                    logical.push(' ');
                    logical.push_str(next);
                    if bare_char_literal_atoms(&logical) == wanted {
                        sites.push(format!("{path}:{number}"));
                        break;
                    }
                }
            }
        }

        assert_eq!(
            sites.len(),
            1,
            "the identity alphabet's character clause is spelled {} times in \
             executable lines under src/, and `is_identity_char`'s doc claims to \
             be THE one spelling. Sites: {sites:?}. A second spelling is a \
             boundary that can stop agreeing with the boundary: WR-03 measured \
             exactly that, in `envelope::advisory::is_plain_component`, gating \
             the GitHub owner/repo segments that are interpolated into a request \
             path. The repair is DELEGATION to `crate::text::is_identity_char`, \
             not a softening of the claim.",
            sites.len()
        );
        assert_eq!(
            sites[0].split(':').next(),
            Some("src/text.rs"),
            "the one surviving spelling must be `is_identity_char`'s own, in this \
             module; found it at {}. A single spelling that lives somewhere else \
             is still one spelling, but it is no longer the one the doc claims.",
            sites[0]
        );
    }

    /// The normal form the census matches on, driven directly (WR-05).
    ///
    /// Every arm here is a spelling the OLD single-byte-string needle missed or
    /// would have missed. It drives the same [`bare_char_literal_atoms`] the
    /// live census consumes, so a normalisation that stopped normalising cannot
    /// leave this green.
    /// One arm of the identity alphabet's punctuation set, alone on its line.
    ///
    /// **Assembled at runtime for the same reason the needle is** (WR-05, and it
    /// was measured): the first draft of the test below spelled its equivalent
    /// clauses as whole literals, and the census — which walks `src/text.rs`
    /// like every other file — counted SEVEN of them, reporting eight sites
    /// where one exists. A control whose fixtures are themselves hits is a
    /// control that breaks the thing it is certifying. Each of these three lines
    /// carries ONE atom, is a proper subset of the clause, and ends with `;` so
    /// [`continues_onto_the_next_line`] refuses to join it to its neighbour.
    const ATOM_DOT: &str = "'.'";
    /// See [`ATOM_DOT`].
    const ATOM_UNDERSCORE: &str = "'_'";
    /// See [`ATOM_DOT`].
    const ATOM_DASH: &str = "'-'";

    #[test]
    fn the_alphabet_needle_survives_a_reorder_and_a_respacing() {
        let identity_atoms = bare_char_literal_atoms(&format!(
            "{ALPHABET_CLAUSE_HEAD}{ALPHABET_CLAUSE_TAIL}"
        ));
        let expected = {
            let mut atoms: Vec<char> = [ATOM_DOT, ATOM_UNDERSCORE, ATOM_DASH]
                .iter()
                .flat_map(|fragment| bare_char_literal_atoms(fragment))
                .collect();
            atoms.sort_unstable();
            atoms.dedup();
            atoms
        };
        assert_eq!(
            identity_atoms, expected,
            "the assembled needle must normalise to the three punctuation atoms"
        );

        let (dot, underscore, dash) = (ATOM_DOT, ATOM_UNDERSCORE, ATOM_DASH);
        for equivalent in [
            // The one surviving production spelling, ranges and all.
            format!("matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | {dot} | {underscore} | {dash})"),
            // The WR-03 respelling `advisory::is_plain_component` used to carry.
            format!("c.is_ascii_alphanumeric() || matches!(c, {dot} | {underscore} | {dash})"),
            // REORDERED — invisible to the old needle, which is WR-05 itself.
            format!("matches!(c, {dash} | {underscore} | {dot})"),
            // RE-SPACED.
            format!("matches!(c,{dot}|{underscore}|{dash})"),
            format!("matches!(  c ,  {dot}  |  {underscore}  |  {dash}  )"),
            // A different range spelling around the same three atoms.
            format!("matches!(c, '0'..='9' | {dash} | {dot} | {underscore})"),
        ] {
            assert_eq!(
                bare_char_literal_atoms(&equivalent),
                identity_atoms,
                "{equivalent:?} spells the identity alphabet's punctuation set \
                 and must normalise to the same atoms. A census that depends on \
                 the order the author typed the arms in is a census whose power \
                 is a coin flip."
            );
        }

        // Range ENDPOINTS are not bare atoms. Without this the ranges above
        // would contribute 'A', 'Z', 'a', 'z', '0', '9' and nothing would ever
        // compare equal.
        assert_eq!(
            bare_char_literal_atoms("matches!(c, 'A'..='Z')"),
            Vec::<char>::new(),
            "a `..=` range contributes no bare atoms"
        );
    }

    /// **The exclusion, ASSERTED rather than assumed.**
    ///
    /// Normalising the needle loosens it, so over-matching is the new failure
    /// direction and the nearby set this census must NOT count is the one to
    /// pin. `envelope::advisory::default_branch_of` admits `'/'` in addition to
    /// the identity alphabet's three punctuation arms, because a branch name
    /// legitimately carries a path separator and is not an identity —
    /// `is_plain_component`'s doc says so and collapsing the two would be the
    /// opposite error to WR-03.
    ///
    /// The comparison is an EQUALITY on the whole line's atom set, so four atoms
    /// are not three and the line is excluded. This test drives the same helper
    /// the live census consumes, in both directions.
    #[test]
    fn the_normalized_needle_still_excludes_the_branch_name_set() {
        let identity_atoms = bare_char_literal_atoms(&format!(
            "{ALPHABET_CLAUSE_HEAD}{ALPHABET_CLAUSE_TAIL}"
        ));

        // Spelled exactly as `advisory::default_branch_of` carries it, and
        // assembled from the same one-atom fragments so this line is not itself
        // a census hit.
        let (dot, underscore, dash) = (ATOM_DOT, ATOM_UNDERSCORE, ATOM_DASH);
        let branch_line = format!(
            "            .all(|c| c.is_ascii_alphanumeric() || matches!(c, {dot} | {underscore} | {dash} | '/'))"
        );
        let branch_atoms = bare_char_literal_atoms(&branch_line);
        assert_eq!(
            branch_atoms.len(),
            identity_atoms.len() + 1,
            "the branch-name set is the identity alphabet's punctuation PLUS the \
             path separator; normalised to {branch_atoms:?}"
        );
        assert!(
            branch_atoms.contains(&'/'),
            "the extra atom must be the path separator, got {branch_atoms:?}"
        );
        assert_ne!(
            branch_atoms, identity_atoms,
            "`default_branch_of`'s set admits `'/'` and is a different question \
             from the identity alphabet. If the census started counting it, the \
             normalisation has begun over-matching and the equality on a count \
             would go red for a line that is correct — which sends the reader to \
             delegate a boundary that must not be collapsed."
        );

        // And the census, run for real, does not name that file.
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        collect_rs(&base.join("src"), &base, &mut files);
        let mut counted: Vec<String> = Vec::new();
        for (path, lines) in &files {
            for (number, line) in lines {
                if !line.trim_start().starts_with("//")
                    && bare_char_literal_atoms(line) == identity_atoms
                {
                    counted.push(format!("{path}:{number}"));
                }
            }
        }
        assert!(
            !counted.iter().any(|site| site.contains("advisory.rs")),
            "the census counted a line in `envelope/advisory.rs`, and the only \
             character-set clause left in that file is `default_branch_of`'s — \
             which must stay excluded. Counted: {counted:?}"
        );
    }

    // -----------------------------------------------------------------------
    // The interpreter census: `shell_command_fragment`, made checkable instead
    // of written (21-27, CR-01, T-21-27-04)
    // -----------------------------------------------------------------------

    /// The stems of the command-interpreter binaries, each **missing its last
    /// letter** so no line of this module spells one.
    ///
    /// The anti-self-match idiom this tree already uses at
    /// [`ALPHABET_CLAUSE_HEAD`] / [`ALPHABET_CLAUSE_TAIL`] and at
    /// `ui::screens::render_escape_guard`'s `IMPL_HEAD`/`IMPL_TAIL`. The census
    /// walks `src/`, and `src/text.rs` is under `src/`; spelled whole, these
    /// lines would be hits and the census would report itself.
    ///
    /// Splitting on the LAST letter rather than in the middle is deliberate:
    /// a middle split would leave `"sh"` sitting in this array as the tail of
    /// `bash`, `zsh`, `dash`, `ksh` and `fish`, which is exactly the token the
    /// census looks for.
    const INTERPRETER_STEMS: [&str; 8] = ["s", "bas", "zs", "das", "ks", "fis", "cs", "tcs"];
    /// The letter every entry of [`INTERPRETER_STEMS`] is missing. Meaningless
    /// alone, which is the whole point.
    const INTERPRETER_STEM_TAIL: &str = "h";

    /// How many physical lines a wrapped call may span before the join gives
    /// up. Larger than [`ALPHABET_CLAUSE_JOIN_LINES`] because the construction
    /// this census looks for is a `.args([...])` block, not a `matches!` arm:
    /// the pre-fix `ui::screens::detail` site spanned fifteen physical lines
    /// with the interpreter on one and the interpolation on another.
    const INTERPRETER_JOIN_LINES: usize = 16;

    /// The interpreter binary names, assembled at runtime.
    fn interpreter_binary_names() -> Vec<String> {
        INTERPRETER_STEMS
            .iter()
            .map(|stem| format!("{stem}{INTERPRETER_STEM_TAIL}"))
            .collect()
    }

    /// Does `logical` NAME a command-interpreter binary?
    ///
    /// The name must appear as a **quoted token** (`"sh"`) or as the last
    /// segment of a quoted path (`"/bin/sh"`) — which is how a binary is
    /// actually named to [`std::process::Command`]. Matching a bare substring
    /// would catch `shown`, `shorten_session_id` and `dash` used as a local
    /// variable, none of which spawn anything.
    fn names_an_interpreter(logical: &str) -> bool {
        interpreter_binary_names().iter().any(|name| {
            logical.contains(&format!("\"{name}\"")) || logical.contains(&format!("/{name}\""))
        })
    }

    /// Does `logical` contain one of **five** interpolation markers?
    ///
    /// Not "does it interpolate". The markers are exactly
    /// [`INTERPOLATION_MARKERS`] — `format!`, `write!`, `writeln!`, `+ &` and
    /// `push_str(&` — and nothing else is looked for.
    ///
    /// Both halves must hold for a line to be reported: naming an interpreter
    /// with a FIXED command is not the defect, and a census that reported it
    /// would be a ban on a word rather than a check on a construction — at
    /// which point the next author works around it by renaming.
    ///
    /// # What the five markers miss, with the direction
    ///
    /// A string assembled by any OTHER means is invisible here: a `push_str`
    /// of an already-owned value (no `&`), a `concat!`, a `join`, an owned `+`
    /// without the reference marker, a `replace`, a `String::from` fed a
    /// previously-built variable. **Under-detection, silent.**
    ///
    /// **Why the set is not widened** (IN-05, 21-32 T2). Every additional
    /// substring marker adds false positives to a control whose entire value is
    /// that its zero can be trusted, and a census that reports correct code is a
    /// census the next author disables. The honest move is to say what the five
    /// are and what they miss — not to enumerate the next level down and call
    /// the claim repaired.
    fn interpolates_into_a_string(logical: &str) -> bool {
        INTERPOLATION_MARKERS
            .iter()
            .any(|marker| logical.contains(marker))
    }

    /// The five substrings [`interpolates_into_a_string`] recognises, hoisted
    /// out of its body so the boundary control can build its fixture from the
    /// SAME list the live census consumes rather than respelling one.
    const INTERPOLATION_MARKERS: [&str; 5] = ["format!", "write!", "writeln!", "+ &", "push_str(&"];

    /// Whether `logical` has an unclosed `(` or `[`, so the next physical line
    /// belongs to the same call.
    ///
    /// Deliberately NOT a method-chain follower. A rule that chased `.method()`
    /// onto the next line would join
    /// `Command::new("/bin/sh")` in `driver::liveness`'s test fixture to the
    /// `.args([.., &format!(..)])` three lines below it and report a fixture
    /// that spawns a FIXED command — a false positive on correct code.
    ///
    /// The price is paid, and it is stated as the **method-chain residual** in
    /// [`no_executable_line_under_src_hands_an_interpreter_an_interpolated_program`]'s
    /// "What it does NOT see" block. Pass 11 recorded that this sentence used to
    /// point at prose that did not exist — the block named three residuals and
    /// this was not one of them. It is now written there, with its direction.
    fn has_an_unclosed_delimiter(logical: &str) -> bool {
        logical.matches('(').count() > logical.matches(')').count()
            || logical.matches('[').count() > logical.matches(']').count()
    }

    /// Every executable logical line under `src/` that hands a command
    /// interpreter an interpolated program, as `path:line`.
    ///
    /// Extracted so the live assertion and its planted-defect control consume
    /// the SAME function — a control that exercises a re-implementation
    /// certifies the re-implementation.
    fn interpreter_program_sites(root: &std::path::Path) -> Vec<String> {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        collect_rs(root, &base, &mut files);
        files.sort_by(|a, b| a.0.cmp(&b.0));

        let mut sites = Vec::new();
        for (path, lines) in &files {
            sites.extend(interpreter_sites_in(path, lines));
        }
        sites
    }

    /// The census's per-file walk, over an already-numbered list of lines.
    ///
    /// Extracted from [`interpreter_program_sites`] for the reason this module
    /// gives everywhere else: **the live assertion and its boundary control
    /// must consume the SAME function**, because a control that exercises a
    /// re-implementation certifies the re-implementation. Pass 11 read this
    /// census's published algorithm, re-implemented it in Python and found the
    /// mechanism disagreed with the doc — the split below is what makes the
    /// in-tree control able to find that class of defect without leaving Rust.
    ///
    /// Taking `&[(usize, String)]` rather than a path also means the boundary
    /// control drives it from an in-memory fixture: no temporary directory, no
    /// filesystem read, nothing to leave behind if the test panics.
    fn interpreter_sites_in(path: &str, lines: &[(usize, String)]) -> Vec<String> {
        let mut sites = Vec::new();
        for (index, (number, line)) in lines.iter().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            let mut logical = line.to_string();
            let mut ahead = index;
            let mut taken = 1;
            loop {
                if names_an_interpreter(&logical) && interpolates_into_a_string(&logical) {
                    sites.push(format!("{path}:{number}"));
                    break;
                }
                if taken >= INTERPRETER_JOIN_LINES
                    || ahead + 1 >= lines.len()
                    || !has_an_unclosed_delimiter(&logical)
                {
                    break;
                }
                ahead += 1;
                let next = lines[ahead].1.trim();
                if next.starts_with("//") {
                    continue;
                }
                // `taken` counts EXECUTABLE lines joined, and it is incremented
                // HERE — after the comment `continue`, not before it (21-32 T1,
                // T-21-32-01). With the increment above, a comment the join
                // then discarded had already been paid for, so twelve comment
                // lines exhausted INTERPRETER_JOIN_LINES and silenced the
                // census on the exact CR-01 construction it exists to catch.
                // A comment cannot carry a call; it must not cost budget.
                // Pinned in both directions by
                // `the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments`.
                taken += 1;
                logical.push(' ');
                logical.push_str(next);
            }
        }
        sites
    }

    /// **`shell_command_fragment`, checked rather than written** (CR-01,
    /// T-21-27-04).
    ///
    /// [`Untrusted::as_raw_for_logic_only`]'s doc states the rule — a value
    /// reaching a subprocess goes in as its own argv element, never into a
    /// program string an interpreter will parse. A doc cannot go red. This can.
    ///
    /// # Exactly what it reports — three bounds, not a completeness claim
    ///
    /// It reports every executable line under `src/` at which **all three** of
    /// these hold, and the answer must be none:
    ///
    /// 1. a command-interpreter binary is named by a **QUOTED LITERAL** —
    ///    `"sh"` or the last segment of a quoted path, per
    ///    [`names_an_interpreter`]. An interpreter named through a variable, a
    ///    const or a config value is not seen.
    /// 2. one of the **five** markers in [`INTERPOLATION_MARKERS`] appears, per
    ///    [`interpolates_into_a_string`]. That doc enumerates them and names
    ///    what they miss.
    /// 3. both fall inside **ONE joined logical unit** of at most
    ///    [`INTERPRETER_JOIN_LINES`] **executable** lines, joined only while
    ///    delimiters stay open, per [`has_an_unclosed_delimiter`].
    ///
    /// That is the claim. It is deliberately narrower than "any such line":
    /// round 10's version of this doc said **any**, pass 11 measured that false
    /// (twelve comment lines defeated it), and 21-32 repaired the mechanism and
    /// cut the word in the same commit rather than leaving a claim wider than
    /// its certificate.
    ///
    /// **Why an EQUALITY on a count and not an `is_empty()`.** The same reason
    /// [`exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src`]
    /// uses one: the failure message can then name the offending sites and the
    /// count, which is what sends the reader to the line rather than to the
    /// census.
    ///
    /// # Observed RED by planting
    ///
    /// A function added to `src/driver/liveness.rs`, a file 21-27 does not
    /// otherwise touch, spawning an interpreter with an interpolated program.
    /// **The plant was the multi-line `.args([..])` shape CR-01 itself had** —
    /// interpreter on one physical line, `format!` on another — so this red is
    /// also the evidence that the join reaches across them and that the census
    /// would have caught CR-01:
    ///
    /// ```text
    /// thread 'text::tests::no_executable_line_under_src_hands_an_interpreter_an_interpolated_program' (2006122) panicked at src/text.rs:1799:9:
    /// assertion `left == right` failed: 1 executable line(s) under src/ hand a command interpreter a program string with a value interpolated into it. Sites: ["src/driver/liveness.rs:281"]. Every metacharacter of that value is a candidate token there — a quote, a semicolon, a backtick, a dollar-parenthesis or a newline is CODE. The repair is to delete the interpreter: build an argv vector and let `execve` receive the bytes unparsed. Quoting for the interpreter is the weaker answer and `Untrusted::as_raw_for_logic_only`'s doc says why.
    ///   left: 1
    ///  right: 0
    /// ```
    ///
    /// The plant was removed and `git status --porcelain` confirmed clean
    /// afterwards (`M src/text.rs` alone). A control never observed red is not
    /// a certificate — this module's own doc says so and this census is held to
    /// it.
    ///
    /// # The reach, MEASURED
    ///
    /// [`the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments`]
    /// drives [`interpreter_sites_in`] over the CR-01 shape and measures both
    /// sides of the window:
    ///
    /// - **Comment lines cost nothing, at any count.** The shape is reported
    ///   with **100** comment lines between the interpreter name and the
    ///   interpolation — the largest count actually tested — because `taken`
    ///   counts executable lines only.
    /// - **The executable window is 11 / 12.** The same shape with executable
    ///   filler is reported at **11** filler lines and MISSED at **12**, both
    ///   asserted, and both measured identical before and after the 21-32 fix.
    ///
    /// Those two numbers ARE this census's reach. A change to either means the
    /// window moved, and it must be re-measured and re-disclosed here rather
    /// than absorbed.
    ///
    /// # What it does NOT see, with the direction
    ///
    /// Five residuals, each with its failure direction. None of them is bounded
    /// by this census; what bounds them is
    /// [`Untrusted::as_raw_for_logic_only`]'s rule and code review.
    ///
    /// 1. **Assembled across statements.** It is a SOURCE SCAN over one logical
    ///    call at a time, so a program string built into a local on one line and
    ///    handed to an interpreter three lines later is invisible.
    ///    **Under-detection, silent.**
    /// 2. **Interpreter named by a variable.** [`names_an_interpreter`] wants a
    ///    quoted literal; a binary chosen through a variable, a const or a
    ///    config value is invisible. **Under-detection, silent.**
    /// 3. **Interpolation only into a neighbouring non-program string.**
    ///    `project_creator::execute_hook` spawns an interpreter on one line and
    ///    interpolates only into its ERROR message on another, and is not
    ///    reported — correctly: the hook command is a shell command by design,
    ///    supplied by the operator's own config. **Deliberate exclusion**, not a
    ///    gap.
    /// 4. **Method chains are not followed** — the residual
    ///    [`has_an_unclosed_delimiter`]'s doc promises is stated here. The join
    ///    advances only while `(` or `[` stay unclosed; it does NOT chase
    ///    `.method()` onto the next line. So a construction whose interpreter
    ///    name sits on a line with balanced delimiters and whose interpolation
    ///    sits on a later chained call is never joined to it and is invisible.
    ///    **Under-detection, silent.** The price is paid on purpose: a
    ///    chain-follower would join `Command::new("/bin/sh")` in
    ///    `driver::liveness`'s fixture to an unrelated `.args([.., &format!(..)])`
    ///    three lines below and report correct code — and a census that cries
    ///    wolf is a census the next author deletes.
    /// 5. **Four of five string-building forms are unseen.**
    ///    [`interpolates_into_a_string`] looks for five substrings; a string
    ///    built by `concat!`, `join`, an owned `+`, a `replace` or an owned
    ///    `push_str` carries none of them. **Under-detection, silent.** Widening
    ///    the set is rejected in that doc, with the reason.
    ///
    /// The census's job is to stop the single-call form from being
    /// re-introduced silently, and it is deliberately not sold as more than
    /// that.
    #[test]
    fn no_executable_line_under_src_hands_an_interpreter_an_interpolated_program() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut probe = Vec::new();
        collect_rs(&base.join("src"), &base, &mut probe);
        assert!(
            !probe.is_empty(),
            "the census walked src/ and found no Rust source at all, so a clean \
             result here would be a walk that never looked"
        );

        let sites = interpreter_program_sites(&base.join("src"));
        assert_eq!(
            sites.len(),
            0,
            "{} executable line(s) under src/ hand a command interpreter a \
             program string with a value interpolated into it. Sites: \
             {sites:?}. Every metacharacter of that value is a candidate token \
             there — a quote, a semicolon, a backtick, a dollar-parenthesis or \
             a newline is CODE. The repair is to delete the interpreter: build \
             an argv vector and let `execve` receive the bytes unparsed. \
             Quoting for the interpreter is the weaker answer and \
             `Untrusted::as_raw_for_logic_only`'s doc says why.",
            sites.len()
        );
    }

    /// The two `.args` flag letters the CR-01 fixture needs, **split from their
    /// leading dash and their quotes** so no line of this module spells a
    /// quoted command flag. Same idiom as [`INTERPRETER_STEMS`], same reason.
    const FIXTURE_FLAG_STEMS: [&str; 2] = ["e", "c"];

    /// The CR-01 opener, **split across the spawn seam's own needle** so no
    /// executable line of this module spells `Command::new(`.
    ///
    /// `tests/spawn_seam_guard.rs` walks `src/` for exactly that substring and
    /// requires every file carrying it to be on its allowlist. `src/text.rs`
    /// spawns nothing and must not join that allowlist to accommodate a test
    /// fixture — the allowlist is the record of which files really do spawn.
    /// Same anti-self-match idiom as [`INTERPRETER_STEMS`], applied to a
    /// SIBLING census rather than to this one.
    const FIXTURE_SPAWN_HEAD: &str = "std::process::Comman";
    const FIXTURE_SPAWN_TAIL: &str = "d::new(&term)";

    /// The CR-01 construction, built IN MEMORY: a `.args([..])` block whose
    /// interpreter name is separated from its interpolation by `comments`
    /// comment lines and `filler` executable filler lines.
    ///
    /// Every interpreter name, command flag and interpolation marker is
    /// ASSEMBLED AT RUNTIME from [`interpreter_binary_names`],
    /// [`FIXTURE_FLAG_STEMS`] and [`INTERPOLATION_MARKERS`] — the same halves
    /// the live census consumes. Spelled whole, these fixture lines would be
    /// live census hits in `src/text.rs`, which is under `src/`: the census
    /// would report itself.
    ///
    /// The census joins from the `.args([` line, not from the `match` opener —
    /// `Command::new(&term)` has balanced delimiters, so the opener is not
    /// joined to anything. That is why the reported line is the second one.
    fn cr01_fixture(comments: usize, filler: usize) -> Vec<(usize, String)> {
        let name = interpreter_binary_names()[0].clone();
        let quote = '"';
        let mut lines: Vec<String> = vec![
            format!("match {FIXTURE_SPAWN_HEAD}{FIXTURE_SPAWN_TAIL}"),
            ".args([".to_string(),
            format!("{quote}-{}{quote},", FIXTURE_FLAG_STEMS[0]),
            format!("{quote}{name}{quote},"),
            format!("{quote}-{}{quote},", FIXTURE_FLAG_STEMS[1]),
        ];
        for index in 0..filler {
            lines.push(format!("{quote}filler-{index}{quote},"));
        }
        for index in 0..comments {
            lines.push(format!("// a spacer comment, number {index}"));
        }
        lines.push(format!(
            "&{}({quote}{{prefix}} {{value}}{quote}),",
            INTERPOLATION_MARKERS[0]
        ));
        lines.push("])".to_string());
        lines
            .into_iter()
            .enumerate()
            .map(|(index, line)| (index + 1, line))
            .collect()
    }

    /// **A comment line must not spend the census's join budget** (21-32 T1,
    /// T-21-32-01, pass 11 gaps[1]).
    ///
    /// Pass 11 re-implemented this census's published algorithm and ran it
    /// against the exact CR-01 shape with N comment lines between the
    /// interpreter name and the `format!`. It reported at N = 0, 4, 10, 11 and
    /// **MISSED at N = 12, 13, 20** — on the one construction the census exists
    /// to catch. The cause was one statement's placement: `taken += 1` executed
    /// immediately after `ahead += 1`, i.e. BEFORE the comment `continue`, so a
    /// line the join then threw away had already been paid for. Twelve comments
    /// exhausted [`INTERPRETER_JOIN_LINES`] and the walk gave up.
    ///
    /// # Observed RED against the pre-fix placement
    ///
    /// ```text
    /// thread 'text::tests::the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments' (2948088) panicked at src/text.rs:2014:13:
    /// the census MISSED the CR-01 shape with 12 comment line(s) between the interpreter name and the interpolation. A comment cannot carry a call, so it must not spend the join budget: `taken += 1` has to run AFTER the comment `continue`, not before it.
    /// note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    /// test text::tests::the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments ... FAILED
    /// test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 0.00s
    /// ```
    ///
    /// Recorded exactly: the first failure fired at **twelve** comment lines,
    /// the arm having passed at zero, four, ten and eleven — reproducing pass
    /// 11's measurement independently, in Rust, against the committed function
    /// rather than against a re-implementation of it.
    ///
    /// # The window this pins, in both directions
    ///
    /// The comment arm is asserted well past the join constant, so no comment
    /// count silences the census. The EXECUTABLE arm is asserted at N and at
    /// N + 1 in both directions — reported at [`LAST_REPORTED_FILLER`], missed
    /// at [`FIRST_MISSED_FILLER`] — and those two numbers were measured
    /// identical before and after the move, which is how the fix is known to
    /// have changed the comment behaviour WITHOUT moving the executable window.
    /// Those two numbers are the census's reach, and a change to either means
    /// the window moved and must be re-measured and re-disclosed on the census
    /// itself.
    #[test]
    fn the_interpreter_join_budget_is_spent_on_executable_lines_not_on_comments() {
        // ---- The executable arm: the window, asserted at N and N + 1. ----
        //
        // Deliberately FIRST. Run against the pre-fix increment placement this
        // arm passes and the comment arm below is what goes red, so the same
        // single run measures the executable window BEFORE the fix; run again
        // after the fix it measures the same window AFTER. That is how the pair
        // below is known to be identical either side of the move rather than
        // merely argued to be.
        //
        // The fixture spends four lines of budget before the filler starts —
        // the `.args([` line the join begins from (`taken` = 1), then the exec
        // flag, the interpreter name and the command flag (`taken` = 4) — and
        // one further line of budget is needed for the interpolation itself.
        // So with INTERPRETER_JOIN_LINES = 16 the last reportable filler count
        // is 16 - 5 = 11 and the first missed one is 16 - 4 = 12. Both numbers
        // are MEASURED, not derived: they were run before and after the
        // increment moved and came out identical, which is the evidence that
        // the comment fix left the executable window where it was.
        const LAST_REPORTED_FILLER: usize = 11;
        const FIRST_MISSED_FILLER: usize = 12;
        assert_eq!(
            FIRST_MISSED_FILLER,
            LAST_REPORTED_FILLER + 1,
            "the two sides of the boundary must be adjacent, or this is not a \
             boundary test"
        );
        assert_eq!(
            FIRST_MISSED_FILLER,
            INTERPRETER_JOIN_LINES - 4,
            "the executable window must follow from INTERPRETER_JOIN_LINES = \
             {INTERPRETER_JOIN_LINES} and the fixture's four lines of prefix \
             budget. If this fails the constant moved, the census's reach moved \
             with it, and the claim on the census must be re-measured and \
             re-disclosed rather than this number edited."
        );

        for filler in [0usize, 5, 9, 10, LAST_REPORTED_FILLER] {
            let sites = interpreter_sites_in("fixture.rs", &cr01_fixture(0, filler));
            assert!(
                !sites.is_empty(),
                "the census MISSED the CR-01 shape at {filler} executable \
                 filler line(s), inside the documented window of \
                 {LAST_REPORTED_FILLER}"
            );
        }
        for filler in [FIRST_MISSED_FILLER, 13, 16, 20] {
            let sites = interpreter_sites_in("fixture.rs", &cr01_fixture(0, filler));
            assert!(
                sites.is_empty(),
                "the census REPORTED the CR-01 shape at {filler} executable \
                 filler line(s), beyond the documented window of \
                 {LAST_REPORTED_FILLER}. The window widened; the claim on the \
                 census now understates its reach and must be re-disclosed. \
                 Sites: {sites:?}"
            );
        }

        // ---- The comment arm: budget is never spent on a comment. ----
        for comments in [0usize, 4, 10, 11, 12, 13, 20, 40, 100] {
            let sites = interpreter_sites_in("fixture.rs", &cr01_fixture(comments, 0));
            assert!(
                !sites.is_empty(),
                "the census MISSED the CR-01 shape with {comments} comment \
                 line(s) between the interpreter name and the interpolation. A \
                 comment cannot carry a call, so it must not spend the join \
                 budget: `taken += 1` has to run AFTER the comment `continue`, \
                 not before it."
            );
        }

        // The fixture is in memory: no temporary directory, no filesystem read,
        // nothing left behind if any assertion above panics.
        let reported = interpreter_sites_in("fixture.rs", &cr01_fixture(100, 0));
        assert_eq!(
            reported,
            vec!["fixture.rs:2".to_string()],
            "the join begins at the `.args([` line, whose delimiters are the \
             ones left open; the `match` opener is balanced and joins nothing"
        );
    }

    /// **The census cannot report itself, ASSERTED rather than assumed.**
    ///
    /// `src/text.rs` is the file that builds the needle, and it is under `src/`.
    /// This is not vacuous only because [`INTERPRETER_STEMS`] carries stems with
    /// their last letter removed and the names are assembled at runtime — so no
    /// line of this module spells a quoted interpreter binary, and the lines
    /// that DO interpolate (this census's own `format!` calls) name none.
    ///
    /// Both directions: the module is clean, AND the needle it assembles is the
    /// real one — a stem list that had drifted into meaninglessness would leave
    /// the first assertion green while certifying nothing.
    #[test]
    fn the_interpreter_census_does_not_report_the_module_that_builds_its_needle() {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sites = interpreter_program_sites(&base.join("src").join("text.rs"));
        assert!(
            sites.is_empty(),
            "the census reported its own module at {sites:?}. The needle must \
             be assembled at runtime from halves that are meaningless apart, or \
             the census counts itself and every real hit is buried in noise."
        );

        let names = interpreter_binary_names();
        assert_eq!(
            names.len(),
            INTERPRETER_STEMS.len(),
            "every stem must assemble to a name"
        );
        assert!(
            names.iter().all(|name| name.len() >= 2
                && name.ends_with(INTERPRETER_STEM_TAIL)
                && !INTERPRETER_STEMS.contains(&name.as_str())),
            "the assembled names {names:?} must differ from the stems that \
             build them; if a stem already equalled its name, that stem's own \
             line would spell an interpreter and the anti-self-match property \
             above would be an accident rather than a construction"
        );
    }

    /// **The NON-BAN direction** (21-27 T2(d)).
    ///
    /// A line that names an interpreter binary with a **fixed, non-interpolated**
    /// command is not the defect and must not be reported. Without this arm the
    /// census would be a ban on a word rather than a check on a construction,
    /// and the next author would work around it by renaming the binary.
    ///
    /// The in-tree fixture is `src/driver/liveness.rs:414`,
    /// `std::process::Command::new("/bin/sh")` with `.args(["-c", "sleep 300;
    /// :", ..])` — a real spawn of a real interpreter with a fixed program,
    /// which the census leaves alone. Both directions are driven through the
    /// same predicates the live census consumes: the fixed form is silent, and
    /// the same line with an interpolation added is reported.
    #[test]
    fn the_interpreter_census_does_not_report_a_fixed_non_interpolated_invocation() {
        // Assembled at runtime for the same reason the needle is — spelled
        // whole, these fixture lines would become live census hits in this very
        // file. The SPAWN CONSTRUCTOR is split for the same reason against a
        // DIFFERENT census: `tests/spawn_seam_guard.rs` scans `src/` for the
        // literal `Command::new(`, and a fixture that spelled it whole put
        // `src/text.rs` on that guard's unexpected-spawn-site list. Measured,
        // not anticipated — it went red on the first full run.
        let interpreter = interpreter_binary_names()[0].clone();
        let quoted = format!("\"/bin/{interpreter}\"");
        let spawn = format!("Comm{}::new(", "and");

        let fixed = format!("let c = {spawn}{quoted}).args([\"-c\", \"sleep 300; :\"]);");
        assert!(
            names_an_interpreter(&fixed),
            "the fixture {fixed:?} must NAME an interpreter, or the arm below \
             passes because the fixture is inert rather than because the census \
             is precise"
        );
        assert!(
            !interpolates_into_a_string(&fixed),
            "the fixture {fixed:?} interpolates, so it is the wrong fixture for \
             the non-ban arm"
        );

        // The same line with a value interpolated in IS reported. Without this
        // arm, a predicate that answered `false` for everything would leave the
        // assertion above green while certifying nothing.
        let hostile =
            format!("let c = {spawn}{quoted}).args([\"-c\", &format!(\"echo {{v}}\")]);");
        assert!(
            names_an_interpreter(&hostile) && interpolates_into_a_string(&hostile),
            "the hostile twin {hostile:?} must satisfy BOTH halves, or the \
             census cannot see the defect it exists for"
        );
    }

    // -----------------------------------------------------------------------
    // The carrier's ABSENT traits, certified by a control that goes red
    // (WR-04, T-21-23-06)
    // -----------------------------------------------------------------------

    /// Answer, at runtime, whether a concrete type implements a given trait —
    /// **autoref specialization**, the only way to ask that question on stable
    /// Rust.
    ///
    /// # Why a control at all, and why not a comment
    ///
    /// [`crate::registry::LegacyRegistryKey`]'s doc claims "no `Display` — so it
    /// cannot be interpolated at all", and its only evidence is a compile error
    /// somebody once saw, quoted in a comment at `src/main.rs`. A comment cannot
    /// go red. Add `impl Display` tomorrow and the doc keeps claiming the
    /// absence while the tree no longer has it — which is this phase's defining
    /// failure shape, one level up: a guard whose green result is not evidence.
    ///
    /// # How it works
    ///
    /// For each trait there is a `Yes` half implemented for `&Wrap<T> where T:
    /// ThatTrait` and a `No` half implemented blanket for `Wrap<T>`. The call is
    /// written `(&&Wrap(value)).implements_x()` and **the second `&` is
    /// load-bearing**: every method here takes `&self`, so at receiver type
    /// `&&Wrap<T>` the first candidate step solves `&Self == &&Wrap<T>`, giving
    /// `Self = &Wrap<T>` and selecting the `Yes` half when its bound holds. Drop
    /// one `&` and the first step instead solves `Self = Wrap<T>`, which matches
    /// the `No` half for EVERY type — the probe then answers `false`
    /// unconditionally and the control is worthless. That was observed here
    /// before it was fixed: with a single `&`, the `String` arm went red with
    /// *"`String` implements `Display` and the probe said otherwise, so the
    /// probe is broken and every absence asserted above proved nothing"*, which
    /// is the control arm doing exactly the job it exists for.
    ///
    /// When the bound does not hold, resolution autoderefs to `&Wrap<T>` and
    /// finds the `No` half.
    ///
    /// **It must be invoked at a MONOMORPHIC call site**, which is why the three
    /// entry points below are macros rather than generic functions: inside a
    /// generic `fn f<T>(v: T)` the parameter carries no `Display` bound, so the
    /// `Yes` half could never be selected and the probe would answer `false` for
    /// everything — a control that is always `false` certifies nothing.
    ///
    /// # What it certifies, and its direction — disclosed
    ///
    /// It answers the question for the type **as the test binary sees it**, so
    /// it certifies the absence of an inherent or in-crate impl. An impl added
    /// behind a Cargo feature this test build does not enable would be invisible
    /// to it. **Direction: under-detection, disclosed.** What bounds it is
    /// coherence: [`Untrusted`] is defined in this crate and all three traits
    /// are foreign, so any impl of them for it MUST live in this crate — the
    /// orphan rule leaves nowhere else to put one.
    mod trait_probe {
        pub struct Wrap<T>(pub T);

        pub trait DisplayYes {
            fn implements_display(&self) -> bool;
        }
        impl<T: std::fmt::Display> DisplayYes for &Wrap<T> {
            fn implements_display(&self) -> bool {
                true
            }
        }
        pub trait DisplayNo {
            fn implements_display(&self) -> bool;
        }
        impl<T> DisplayNo for Wrap<T> {
            fn implements_display(&self) -> bool {
                false
            }
        }

        pub trait AsRefStrYes {
            fn implements_as_ref_str(&self) -> bool;
        }
        impl<T: AsRef<str>> AsRefStrYes for &Wrap<T> {
            fn implements_as_ref_str(&self) -> bool {
                true
            }
        }
        pub trait AsRefStrNo {
            fn implements_as_ref_str(&self) -> bool;
        }
        impl<T> AsRefStrNo for Wrap<T> {
            fn implements_as_ref_str(&self) -> bool {
                false
            }
        }

        pub trait IntoCowYes {
            fn implements_into_cow_str(&self) -> bool;
        }
        impl<T: Into<std::borrow::Cow<'static, str>>> IntoCowYes for &Wrap<T> {
            fn implements_into_cow_str(&self) -> bool {
                true
            }
        }
        pub trait IntoCowNo {
            fn implements_into_cow_str(&self) -> bool;
        }
        impl<T> IntoCowNo for Wrap<T> {
            fn implements_into_cow_str(&self) -> bool {
                false
            }
        }

        // ── WR-01: the three absences the doc claimed and nothing certified ──

        pub trait DerefStrYes {
            fn implements_deref_str(&self) -> bool;
        }
        impl<T: std::ops::Deref<Target = str>> DerefStrYes for &Wrap<T> {
            fn implements_deref_str(&self) -> bool {
                true
            }
        }
        pub trait DerefStrNo {
            fn implements_deref_str(&self) -> bool;
        }
        impl<T> DerefStrNo for Wrap<T> {
            fn implements_deref_str(&self) -> bool {
                false
            }
        }

        pub trait BorrowStrYes {
            fn implements_borrow_str(&self) -> bool;
        }
        impl<T: std::borrow::Borrow<str>> BorrowStrYes for &Wrap<T> {
            fn implements_borrow_str(&self) -> bool {
                true
            }
        }
        pub trait BorrowStrNo {
            fn implements_borrow_str(&self) -> bool;
        }
        impl<T> BorrowStrNo for Wrap<T> {
            fn implements_borrow_str(&self) -> bool {
                false
            }
        }

        pub trait SerializeYes {
            fn implements_serialize(&self) -> bool;
        }
        impl<T: serde::Serialize> SerializeYes for &Wrap<T> {
            fn implements_serialize(&self) -> bool {
                true
            }
        }
        pub trait SerializeNo {
            fn implements_serialize(&self) -> bool;
        }
        impl<T> SerializeNo for Wrap<T> {
            fn implements_serialize(&self) -> bool {
                false
            }
        }
    }

    /// Does the value's type implement [`std::fmt::Display`]?
    macro_rules! implements_display {
        ($value:expr) => {{
            #[allow(unused_imports)]
            use $crate::text::tests::trait_probe::{DisplayNo, DisplayYes, Wrap};
            (&&Wrap($value)).implements_display()
        }};
    }

    /// Does the value's type implement `AsRef<str>`?
    macro_rules! implements_as_ref_str {
        ($value:expr) => {{
            #[allow(unused_imports)]
            use $crate::text::tests::trait_probe::{AsRefStrNo, AsRefStrYes, Wrap};
            (&&Wrap($value)).implements_as_ref_str()
        }};
    }

    /// Does the value's type implement `Into<Cow<'static, str>>`?
    macro_rules! implements_into_cow_str {
        ($value:expr) => {{
            #[allow(unused_imports)]
            use $crate::text::tests::trait_probe::{IntoCowNo, IntoCowYes, Wrap};
            (&&Wrap($value)).implements_into_cow_str()
        }};
    }

    /// Does the value's type implement `Deref<Target = str>`?
    macro_rules! implements_deref_str {
        ($value:expr) => {{
            #[allow(unused_imports)]
            use $crate::text::tests::trait_probe::{DerefStrNo, DerefStrYes, Wrap};
            (&&Wrap($value)).implements_deref_str()
        }};
    }

    /// Does the value's type implement `Borrow<str>`?
    macro_rules! implements_borrow_str {
        ($value:expr) => {{
            #[allow(unused_imports)]
            use $crate::text::tests::trait_probe::{BorrowStrNo, BorrowStrYes, Wrap};
            (&&Wrap($value)).implements_borrow_str()
        }};
    }

    /// Does the value's type implement `serde::Serialize`?
    macro_rules! implements_serialize {
        ($value:expr) => {{
            #[allow(unused_imports)]
            use $crate::text::tests::trait_probe::{SerializeNo, SerializeYes, Wrap};
            (&&Wrap($value)).implements_serialize()
        }};
    }

    /// **Fourteen facts in one test, and the both-directions shape is what stops
    /// it going vacuous** (WR-04, WR-01, D-21-8, D-21-36).
    ///
    /// SIX ABSENCES for [`Untrusted`] — the carrier cannot be interpolated,
    /// coerced, borrowed or persisted — and SIX PRESENCES for `String`, which is
    /// the control arm: if a probe were broken (wrong receiver, a bound that
    /// never selects, a trait not in scope) it would answer `false` for
    /// everything, and asserting only the absences would pass forever while
    /// certifying nothing. That is exactly the failure this phase has shipped at
    /// three levels, which is why **each of the three new presence arms was
    /// confirmed answering `true` for `String` BEFORE its absence arm was
    /// written**, not after.
    ///
    /// # WR-01: the certificate was three-sixths of the claim
    ///
    /// [`Untrusted`]'s doc listed six absent conversions, said "five", and this
    /// control certified three. `Deref<Target = str>` is the one that mattered:
    /// add it tomorrow and auto-deref restores the raw path at every existing
    /// call site in the tree at once — no diff at any of them, every test in
    /// this repository still green, and the doc still claiming the absence.
    /// The review offered softening the doc's count as the alternative; that is
    /// honest and weaker, and this phase's own history is that a
    /// disclosed-but-unclosed residual returns as a Critical two rounds later.
    ///
    /// Then two PRESENCES for [`Rendered`], which is the other half of the
    /// design (D-21-8): the ESCAPED type is the convenient one, so at a render
    /// site the short path is the safe path and the raw path costs a deliberate
    /// `as_raw_for_logic_only()`. That asymmetry is a checked property here
    /// rather than a design intention stated in a doc.
    ///
    /// # Observed RED by planting, twice
    ///
    /// **`impl std::fmt::Display for Untrusted`** added to this module's parent,
    /// writing `self.0`:
    ///
    /// ```text
    /// thread 'text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions' (459964) panicked at src/text.rs:1361:9:
    /// `Untrusted` implements `Display`. The whole mechanism is that a carrier cannot be interpolated: with `Display` present, `format!("{}", untrusted)` and `Span::raw(untrusted.to_string())` compile again at every site in the tree, and the compiler stops naming the consumers. Remove the impl; if a human needs to read the value, that is `shown()`.
    /// ```
    ///
    /// **`impl AsRef<str> for Untrusted`** returning `&self.0`:
    ///
    /// ```text
    /// thread 'text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions' (460706) panicked at src/text.rs:1370:9:
    /// `Untrusted` implements `AsRef<str>`. A carrier that can be coerced to `&str` can be handed to any sink that takes one, which is the raw path restored everywhere at once and invisible in a diff. Remove the impl; the raw path is `as_raw_for_logic_only()` and it is meant to be conspicuous.
    /// ```
    ///
    /// **`impl std::ops::Deref<Target = str> for Untrusted`** returning
    /// `&self.0` (21-27, WR-01):
    ///
    /// ```text
    /// thread 'text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions' (2050107) panicked at src/text.rs:2194:9:
    /// `Untrusted` implements `Deref<Target = str>`. This is the one that silently rewrites code that is ALREADY WRITTEN: auto-deref restores the raw path at every existing call site in the tree at once, with no diff at any of them and every test in this repository still green. Remove the impl; the raw path is `as_raw_for_logic_only()` and it is meant to be conspicuous.
    /// ```
    ///
    /// **`impl std::borrow::Borrow<str> for Untrusted`** returning `&self.0`:
    ///
    /// ```text
    /// thread 'text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions' (2053239) panicked at src/text.rs:2202:9:
    /// `Untrusted` implements `Borrow<str>`. A carrier that borrows as `&str` is usable as a `&str` map key and can be handed to any sink that takes one — the same hole as `AsRef<str>`, through a second door.
    /// ```
    ///
    /// **`impl serde::Serialize for Untrusted`** writing `serialize_str(&self.0)`:
    ///
    /// ```text
    /// thread 'text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions' (2055074) panicked at src/text.rs:2209:9:
    /// `Untrusted` implements `serde::Serialize`. Persistence must go through `as_raw_for_logic_only()`, which is a choice a reviewer sees in a diff; a derived `Serialize` writes the raw bytes to disk from any struct that happens to contain one, invisibly.
    /// ```
    ///
    /// Every impl was planted ONE AT A TIME, its red captured, then removed with
    /// `git status --porcelain` confirming a clean tree before the next.
    /// A control never observed red is not a certificate.
    ///
    /// # What is STILL not certified, with the direction
    ///
    /// Six traits are probed. `PartialEq<str>`, `Into<String>`,
    /// `ToString` via some other route, a future INHERENT method returning
    /// `&str` under a different name, and any trait a dependency adds to
    /// [`Untrusted`] by blanket impl are **not** probed. **Under-detection,
    /// silent.** These six are the ones whose absence the doc claims — they are
    /// not the closed set of ways a raw `&str` could escape, and this test does
    /// not claim to be one.
    ///
    /// The probe also answers for the type as THIS test binary sees it, so an
    /// impl behind a Cargo feature this build does not enable is invisible to
    /// it. What bounds that is coherence: [`Untrusted`] is defined in this crate
    /// and all six traits are foreign, so any impl of them for it must live in
    /// this crate — the orphan rule leaves nowhere else to put one.
    #[test]
    fn an_untrusted_carrier_implements_none_of_the_string_conversions() {
        // ── The carrier: three absences ───────────────────────────────────
        let carrier = || Untrusted::from_untrusted_source("demo".to_string());

        assert!(
            !implements_display!(carrier()),
            "`Untrusted` implements `Display`. The whole mechanism is that a \
             carrier cannot be interpolated: with `Display` present, \
             `format!(\"{{}}\", untrusted)` and `Span::raw(untrusted.to_string())` \
             compile again at every site in the tree, and the compiler stops \
             naming the consumers. Remove the impl; if a human needs to read the \
             value, that is `shown()`."
        );
        assert!(
            !implements_as_ref_str!(carrier()),
            "`Untrusted` implements `AsRef<str>`. A carrier that can be coerced \
             to `&str` can be handed to any sink that takes one, which is the \
             raw path restored everywhere at once and invisible in a diff. \
             Remove the impl; the raw path is `as_raw_for_logic_only()` and it \
             is meant to be conspicuous."
        );
        assert!(
            !implements_into_cow_str!(carrier()),
            "`Untrusted` implements `Into<Cow<'static, str>>`. That is precisely \
             the bound `Span::raw` and `Span::styled` take, so the carrier could \
             be handed straight to a ratatui sink and the retype would gate \
             nothing at all."
        );
        assert!(
            !implements_deref_str!(carrier()),
            "`Untrusted` implements `Deref<Target = str>`. This is the one that \
             silently rewrites code that is ALREADY WRITTEN: auto-deref \
             restores the raw path at every existing call site in the tree at \
             once, with no diff at any of them and every test in this \
             repository still green. Remove the impl; the raw path is \
             `as_raw_for_logic_only()` and it is meant to be conspicuous."
        );
        assert!(
            !implements_borrow_str!(carrier()),
            "`Untrusted` implements `Borrow<str>`. A carrier that borrows as \
             `&str` is usable as a `&str` map key and can be handed to any sink \
             that takes one — the same hole as `AsRef<str>`, through a second \
             door."
        );
        assert!(
            !implements_serialize!(carrier()),
            "`Untrusted` implements `serde::Serialize`. Persistence must go \
             through `as_raw_for_logic_only()`, which is a choice a reviewer \
             sees in a diff; a derived `Serialize` writes the raw bytes to disk \
             from any struct that happens to contain one, invisibly."
        );

        // ── The control arm: three presences, so a broken probe cannot pass ──
        assert!(
            implements_display!(String::from("demo")),
            "`String` implements `Display` and the probe said otherwise, so the \
             probe is broken and every absence asserted above proved nothing"
        );
        assert!(
            implements_as_ref_str!(String::from("demo")),
            "`String` implements `AsRef<str>` and the probe said otherwise, so \
             the probe is broken"
        );
        assert!(
            implements_into_cow_str!(String::from("demo")),
            "`String` implements `Into<Cow<'static, str>>` and the probe said \
             otherwise, so the probe is broken"
        );
        assert!(
            implements_deref_str!(String::from("demo")),
            "`String` implements `Deref<Target = str>` and the probe said \
             otherwise, so the probe is broken and the absence asserted above \
             proved nothing"
        );
        assert!(
            implements_borrow_str!(String::from("demo")),
            "`String` implements `Borrow<str>` and the probe said otherwise, so \
             the probe is broken"
        );
        assert!(
            implements_serialize!(String::from("demo")),
            "`String` implements `serde::Serialize` and the probe said \
             otherwise, so the probe is broken"
        );

        // ── The other half of the design: the ESCAPED type is the easy one ──
        assert!(
            implements_display!(render_for_terminal("demo")),
            "`Rendered` must implement `Display`, or `format!(\"{{}}\", \
             v.shown())` stops compiling and the escaped path stops being the \
             short one — which is the entire reason the raw path is a \
             deliberate choice rather than the convenient default"
        );
        assert!(
            implements_into_cow_str!(render_for_terminal("demo")),
            "`Rendered` must implement `Into<Cow<'static, str>>`, or \
             `Span::raw(v.shown())` stops compiling and every render site grows \
             a `.to_string()` — friction on the SAFE path is how a rule stops \
             being followed"
        );
    }

    /// **`{:?}` is a render surface, and the derive is what re-opened it**
    /// (WR-04, T-21-23-05).
    ///
    /// [`Untrusted`] has a hand-written [`std::fmt::Debug`] that prints
    /// [`shown`](Untrusted::shown), so a raw invisible character cannot reach a
    /// log line, a panic message, an `anyhow` chain, or the derived `Debug` of
    /// any struct that contains one — `GitLogEntry` derives `Debug`, and so does
    /// `Action`, which carries a `Vec<GitLogEntry>`.
    ///
    /// **Both directions over the whole corpus.** The hostile member must come
    /// out escaped and carrying nothing invisible; the clean member must come
    /// out VERBATIM, which is the arrival arm — without it this test would pass
    /// against a `Debug` that printed the empty string.
    ///
    /// The non-vacuity guard runs FIRST and follows the shape
    /// `registry.rs`'s idempotence pin already uses: a corpus of all-ASCII
    /// fixtures would satisfy "carries nothing invisible" trivially, so at least
    /// one pair must actually be changed by [`display_identity`].
    ///
    /// Fixtures are drawn BY IMPORT from
    /// [`LOOK_ALIKE_PAIRS`](crate::test_support::LOOK_ALIKE_PAIRS) (D-21-6) and
    /// never respelled here.
    #[test]
    fn a_carrier_debug_never_carries_an_invisible_character() {
        let corpus_is_actually_hostile = crate::test_support::LOOK_ALIKE_PAIRS
            .iter()
            .any(|(_, hostile)| display_identity(hostile) != *hostile);
        assert!(
            corpus_is_actually_hostile,
            "no member of LOOK_ALIKE_PAIRS is changed by `display_identity`, so \
             every assertion below would hold for a `Debug` that did nothing at \
             all. The corpus, not this test, is what needs fixing."
        );

        for (clean, hostile) in crate::test_support::LOOK_ALIKE_PAIRS {
            let carrier = Untrusted::from_untrusted_source(hostile.to_string());
            let debugged = format!("{carrier:?}");

            assert!(
                !debugged.chars().any(is_invisible_formatting_char),
                "`{{:?}}` on a carrier holding {hostile:?} produced \
                 {debugged:?}, which still carries an invisible-class \
                 character. `#[derive(Debug)]` is how this hole gets re-opened; \
                 the impl must print `shown()`."
            );
            assert!(
                debugged.contains(&display_identity(hostile)),
                "`{{:?}}` on a carrier holding {hostile:?} must contain the \
                 `U+`-escaped spelling {:?}, got {debugged:?}. Carrying nothing \
                 invisible is satisfied by printing nothing at all, so the \
                 escaped form has to be there too.",
                display_identity(hostile)
            );

            let clean_carrier = Untrusted::from_untrusted_source(clean.to_string());
            let clean_debugged = format!("{clean_carrier:?}");
            assert!(
                clean_debugged.contains(clean),
                "`{{:?}}` on a carrier holding the CLEAN member {clean:?} must \
                 show it verbatim, got {clean_debugged:?}. This is the arrival \
                 arm: without it the assertions above would hold for a `Debug` \
                 that printed the empty string."
            );
        }
    }
}
