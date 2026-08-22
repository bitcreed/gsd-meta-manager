//! Fixtures shared by the crate's in-module tests.
//!
//! `#[cfg(test)]`, so nothing here reaches a release binary. It exists because
//! the alternative — each seam's pin spelling its own blank-shape list — is
//! exactly what pass 5 caught: the `--run-id` pin was given
//! `["   ", "\t", "\n  \n"]`, three of six, **in the same commit that defined
//! six**, so the two zero-width shapes were never asserted at the one seam where
//! they were reachable end to end.

/// The payloads that carry no instruction at all.
///
/// Enumerated once, tree-wide, so a seventh blank shape lands in **every** seam's
/// pin at once rather than in whichever pins somebody remembered.
///
/// **The payload set and the production predicate are DELIBERATELY different
/// expressions of "blank".** An enumeration that shared
/// [`crate::text::carries_visible_content`]'s structure could not contain a
/// payload that predicate mishandles, so it could never falsify the thing it
/// exists to check (round-3 WR-03). These are therefore **literals, asserted by
/// name**. The last two are the demonstration: `U+200B` and `U+FEFF` both
/// survive `str::trim` untouched, so under the old trim-based coupling neither
/// could ever have appeared here.
pub const DEGENERATE: [&str; 6] = ["", "   ", "\t", "\n  \n", "\u{200b}", "\u{feff}"];

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
/// Like `DEGENERATE` these are **literals, asserted by name**: an enumeration
/// derived from [`crate::text::carries_invisible_formatting`]'s own structure
/// could not contain a pair that predicate mishandles (round-3 WR-03's
/// tautology).
pub const LOOK_ALIKE_PAIRS: [(&str, &str); 3] = [
    ("demo", "demo\u{200b}"),
    ("abc", "a\u{200b}bc"),
    ("x", "x\u{feff}"),
];
