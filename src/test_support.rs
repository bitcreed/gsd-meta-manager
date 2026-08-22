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
