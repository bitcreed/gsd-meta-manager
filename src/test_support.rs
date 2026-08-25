//! Fixtures shared by the crate's tests — in-module and integration alike.
//!
//! It exists because the alternative — each seam's pin spelling its own
//! blank-shape list — is exactly what pass 5 caught: the `--run-id` pin was
//! given `["   ", "\t", "\n  \n"]`, three of six, **in the same commit that
//! defined six**, so the two zero-width shapes were never asserted at the one
//! seam where they were reachable end to end.
//!
//! **The module ships unconditionally (D-17-5).** It used to be `#[cfg(test)]`,
//! which put it out of reach of the integration crates under `tests/` and left
//! three hand-copied `DEGENERATE` subsets there — the prohibited pattern, and
//! precisely the drift the module exists to prevent. It exports two consts and
//! no behaviour, so there is no runtime cost and no API surface a consumer could
//! misuse; the alternative was keeping the hand copies.

/// The payloads that carry no instruction at all.
///
/// Enumerated once, tree-wide, so a NEW blank shape lands in **every** seam's
/// pin at once rather than in whichever pins somebody remembered. (The count is
/// deliberately not stated in prose anywhere: a number that has to be updated by
/// hand is the same failure mode this const exists to prevent.)
///
/// **These are named seam fixtures. They are necessary and they are NOT
/// sufficient, and the doc that used to claim otherwise was measured asserting
/// a falsehood** (pass-7 gap 2, D-19-4).
///
/// The old text here said this array "could never falsify the thing it exists
/// to check" *unless* it was written independently of the predicate, and
/// concluded — because these are literals rather than a derivation — that it
/// therefore had that falsifying property. Literal spelling is not
/// independence. Every payload in this array was chosen from INSIDE the class
/// the implementation already covered, for six consecutive rounds, which is
/// exactly why six consecutive rounds of fixtures went green over a class that
/// was a 22-code-point subset of the class its own doc named.
///
/// The property this array used to claim now belongs to a test that actually
/// has it: `text::tests::every_format_character_the_standard_names_is_inside_the_class`
/// sweeps ALL code points and takes its oracle from the `unicode-properties`
/// dev-dependency, which shares no code with the production derivation. THAT is
/// what can falsify the class. This array's job is narrower and still real:
/// naming concrete payloads that every seam's pin must refuse, so a seventh
/// blank shape lands in **every** pin at once rather than in whichever pins
/// somebody remembered.
///
/// **The last four members are from OUTSIDE the pre-round-7 ranges** — `U+202E`
/// (RIGHT-TO-LEFT OVERRIDE, Trojan Source / CVE-2021-42574), `U+00AD` (SOFT
/// HYPHEN), `U+E0041` (a tag character, the LLM ASCII-smuggling carrier) and
/// `U+FE0F` (VARIATION SELECTOR-16, default-ignorable but not `Cf`). Pass 7
/// measured all four ACCEPTED against the shipped tree. They are here so the
/// seam pins that consume this array exercise the derived class rather than
/// re-confirming the old ranges.
pub const DEGENERATE: [&str; 10] = [
    "",
    "   ",
    "\t",
    "\n  \n",
    "\u{200b}",
    "\u{feff}",
    "\u{202e}",
    "\u{00ad}",
    "\u{e0041}",
    "\u{fe0f}",
];

/// Pairs that differ in bytes and agree on screen.
///
/// **The fixture shape this tree has never had, and whose absence let a 42-cell
/// matrix go green over a reproduced harm.** [`DEGENERATE`] enumerates "carries
/// nothing"; this enumerates "carries something different from what it renders".
/// Every payload in `DEGENERATE` is *wholly* invisible, so no fixture in the tree
/// could pair two values that a reader cannot tell apart — which is why pass 5
/// could write the sentence "two visually identical aliases resolve to two
/// different envelope and credential paths", pass 6 could reproduce it, and the
/// round-5 fix could still not close it:
/// [`crate::text::carries_visible_content`] refuses `"\u{200b}"` and accepts
/// `"demo\u{200b}"` **by construction**, because it asks whether anything is
/// visible, not whether anything invisible is carried.
///
/// `.0` is the visible member and MUST be accepted wherever a value becomes an
/// identity; `.1` is the look-alike and MUST be refused there. The two members
/// render identically in every terminal the tool is used from.
///
/// **Like `DEGENERATE`, these are named seam fixtures — necessary, not
/// sufficient — and the same doc correction applies** (pass-7 gap 2, D-19-4).
/// The old text here claimed that being literals rather than a derivation gave
/// this array the power to contain a pair the predicate mishandles. It does
/// not: for three rounds every pair was drawn from inside the class the
/// implementation already covered, so the array could only ever agree with it.
/// The falsifying property belongs to
/// `text::tests::every_format_character_the_standard_names_is_inside_the_class`,
/// whose oracle is an independent derivation of the standard.
///
/// What this array is for is the seams: a pin that consumes it exercises both
/// directions at a real call site, and adding a pair here lands in every such
/// pin at once.
///
/// **The last three pairs are from OUTSIDE the pre-round-7 ranges** — `U+202E`,
/// `U+E0041` and `U+00AD` — so a seam pin cannot pass by re-confirming the old
/// literal ranges.
pub const LOOK_ALIKE_PAIRS: [(&str, &str); 6] = [
    ("demo", "demo\u{200b}"),
    ("abc", "a\u{200b}bc"),
    ("x", "x\u{feff}"),
    ("demo", "demo\u{202e}"),
    ("demo", "demo\u{e0041}"),
    ("run", "r\u{00ad}un"),
];
