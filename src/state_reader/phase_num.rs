//! A GSD phase number that can be compared and ordered, decimals included.
//!
//! GSD numbers inserted phases with a decimal (`7.1` sits between `7` and `8`)
//! and writes the same phase with and without zero padding depending on where
//! it appears: ROADMAP's checklist says `Phase 7.1`, its `### Phase 07.1:`
//! heading and the directory `07.1-slug` pad it. Every comparison in this crate
//! used to be `parse::<u32>()`, which made a decimal phase unrepresentable —
//! STATE.md's `current_phase: "7.1"` and a disk frontier on `7.1` both
//! abstained, and the active phase fell through to `completed_phases + 1`, a
//! count taken from ROADMAP's `## Progress` table that says nothing about which
//! phase is actually under way (ttbook drew `*` on its `[Complete]` phase 5).
//!
//! A [`PhaseNum`] is the dot-separated numeric segments, each parsed as an
//! integer, so `07.1`, `7.1` and `7.01` are one value, and ordering is
//! segment-wise: `7 < 7.1 < 7.2 < 8`.
//!
//! **Pure numeric only.** A prefixed id (`M-2`, `AB-29`) or one with a trailing
//! letter (`4a`) does not parse — it has no defined place in a numeric order,
//! and every caller already treats "no number" as "abstain", never as 0.

use std::fmt;

/// A parsed, pad-insensitive, orderable phase number. See the module docs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhaseNum(Vec<u32>);

impl PhaseNum {
    /// Parse `7`, `07`, `7.1`, `07.1`, `0.3`. Surrounding whitespace is
    /// ignored. Returns `None` for anything else — empty segments (`7.`,
    /// `.1`, `7..1`), signs, prefixes, suffixes.
    pub fn parse(text: &str) -> Option<PhaseNum> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let mut segments = Vec::new();
        for seg in text.split('.') {
            if seg.is_empty() || !seg.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            segments.push(seg.parse::<u32>().ok()?);
        }
        Some(PhaseNum(segments))
    }
}

impl From<u32> for PhaseNum {
    fn from(n: u32) -> Self {
        PhaseNum(vec![n])
    }
}

/// `PhaseNum::from(4) == 4`, and `7.1 != 7` — an integer only equals a
/// single-segment phase number.
impl PartialEq<u32> for PhaseNum {
    fn eq(&self, other: &u32) -> bool {
        self.0.len() == 1 && self.0[0] == *other
    }
}

/// The canonical, unpadded spelling: `07.1` displays as `7.1`.
impl fmt::Display for PhaseNum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, seg) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            write!(f, "{seg}")?;
        }
        Ok(())
    }
}

/// The identity key two spellings of one phase share: the canonical
/// [`PhaseNum`] spelling when the id is numeric (`07.1` → `7.1`, `05` → `5`),
/// the raw trimmed text otherwise (`M-2` stays `M-2`).
pub fn phase_key(id: &str) -> String {
    match PhaseNum::parse(id) {
        Some(n) => n.to_string(),
        None => id.trim().to_string(),
    }
}

/// Whether two written phase ids name the same phase (`7.1` and `07.1` do).
pub fn same_phase(a: &str, b: &str) -> bool {
    phase_key(a) == phase_key(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn padding_is_not_identity() {
        assert_eq!(PhaseNum::parse("07.1"), PhaseNum::parse("7.1"));
        assert_eq!(PhaseNum::parse("05"), PhaseNum::parse("5"));
        assert_eq!(PhaseNum::parse("05").unwrap(), 5);
        assert!(same_phase("07.1", "7.1"));
        assert!(!same_phase("7.1", "7"));
        assert_eq!(phase_key("07.1"), "7.1");
    }

    #[test]
    fn decimal_phases_order_between_their_neighbours() {
        let p = |s| PhaseNum::parse(s).unwrap();
        assert!(p("7") < p("7.1"));
        assert!(p("7.1") < p("7.2"));
        assert!(p("7.2") < p("8"));
        assert!(p("9") < p("10"), "numeric, not lexical");
        assert!(p("7.9") < p("7.10"), "segment-wise, not decimal-fraction");
    }

    #[test]
    fn non_numeric_ids_abstain() {
        for bad in [
            "", "M-2", "AB-29", "4a", "7.", ".1", "7..1", "-1", "phase 3",
        ] {
            assert_eq!(PhaseNum::parse(bad), None, "{bad:?}");
        }
        assert_eq!(phase_key("M-2"), "M-2");
    }

    #[test]
    fn integer_equality_is_single_segment_only() {
        assert!(PhaseNum::parse("7.1").unwrap() != 7);
        assert_eq!(PhaseNum::from(7).to_string(), "7");
        assert_eq!(PhaseNum::parse("07.01").unwrap().to_string(), "7.1");
    }
}
