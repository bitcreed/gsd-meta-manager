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
//! **Milestone-prefixed ids.** gsd-core's `milestone-prefixed` convention
//! names a phase `M-NN` (`3-01`, directory `03-01-alpha`): milestone 3, phase
//! 1 of it. One optional dash segment after the leading integer parses, so
//! `3-01`, `03-01` and `3-1` are one value that sorts after its milestone's
//! bare id and decimals (`3 < 3.1 < 3-01 < 3-01.1 < 3-02 < 4`). Identity is
//! convention-free, as upstream `comparePhaseNum`, `normalizePhaseName` and
//! `phaseKeyFromToken` are; only the *scanners* (which text counts as an id)
//! are gated on [`PhaseIdConvention`].
//!
//! **Otherwise pure numeric.** A letter-prefixed id (`M-2`, `AB-29`) or one
//! with a trailing letter (`4a`) does not parse — it has no defined place in a
//! numeric order, and every caller already treats "no number" as "abstain",
//! never as 0.

use std::fmt;

/// Which text a ROADMAP / directory scanner reads as a phase id. It decides
/// what counts as an id, never identity: [`PhaseNum`] is convention-free.
///
/// Read from `.planning/config.json` `phase_id_convention`; only the exact
/// string `milestone-prefixed` opts in (upstream compares
/// `convention === 'milestone-prefixed'`, case-sensitive and unvalidated).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhaseIdConvention {
    /// Integer ids with decimals (`3`, `07.1`), letter-prefixed project codes.
    #[default]
    Legacy,
    /// gsd-core's `M-NN` ids (`3-01`, directory `03-01-alpha`).
    MilestonePrefixed,
}

impl PhaseIdConvention {
    /// Absent, empty, `sequential`, `bracket` (not modelled by this reader),
    /// differently-cased and unknown values are all [`Self::Legacy`].
    pub fn from_config_value(value: Option<&str>) -> Self {
        match value {
            Some("milestone-prefixed") => Self::MilestonePrefixed,
            _ => Self::Legacy,
        }
    }
}

/// A parsed, pad-insensitive, orderable phase number. See the module docs.
///
/// Field order is the sort order: legacy values have `sub == None`, so every
/// legacy comparison is unchanged, and an `M-NN` id sorts after its
/// milestone's bare id and decimals.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PhaseNum {
    major: u32,
    sub: Option<u32>,
    decimals: Vec<u32>,
}

fn digits(seg: &str) -> Option<u32> {
    if seg.is_empty() || !seg.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    seg.parse::<u32>().ok()
}

impl PhaseNum {
    /// Parse `7`, `07`, `7.1`, `07.1`, `0.3`, and the milestone-prefixed
    /// `3-01`, `03-01`, `3-01.1`. Surrounding whitespace is ignored. Returns
    /// `None` for anything else — empty segments (`7.`, `.1`, `7..1`, `3-`),
    /// signs, prefixes, suffixes, a second dash (`3-01-02`) or a dash after a
    /// decimal (`3.1-01`).
    pub fn parse(text: &str) -> Option<PhaseNum> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        let mut segs = text.split('.');
        let first = segs.next()?;
        let (major, sub) = match first.split_once('-') {
            Some((m, s)) => (digits(m)?, Some(digits(s)?)),
            None => (digits(first)?, None),
        };
        let mut decimals = Vec::new();
        for seg in segs {
            decimals.push(digits(seg)?);
        }
        Some(PhaseNum {
            major,
            sub,
            decimals,
        })
    }

    /// The leading integer: `7` for `7.1`, `999` for `999.2`, and the
    /// milestone (`3`) for `3-01`.
    pub fn major(&self) -> u32 {
        self.major
    }

    /// Whether this is a milestone-prefixed `M-NN` id.
    pub fn has_milestone_segment(&self) -> bool {
        self.sub.is_some()
    }

    /// GSD's directory/file-prefix spelling: the integer part (and the `M-NN`
    /// segment) zero-padded to two digits, any decimal tail as written
    /// canonically — `7.1` → `07.1`, `5` → `05`, `3-1` → `03-01`.
    pub fn padded(&self) -> String {
        let mut out = format!("{:02}", self.major);
        if let Some(sub) = self.sub {
            out.push_str(&format!("-{sub:02}"));
        }
        for seg in &self.decimals {
            out.push('.');
            out.push_str(&seg.to_string());
        }
        out
    }
}

impl From<u32> for PhaseNum {
    fn from(n: u32) -> Self {
        PhaseNum {
            major: n,
            sub: None,
            decimals: Vec::new(),
        }
    }
}

/// `PhaseNum::from(4) == 4`, and `7.1 != 7`, `3-01 != 3` — an integer only
/// equals a bare single-segment phase number.
impl PartialEq<u32> for PhaseNum {
    fn eq(&self, other: &u32) -> bool {
        self.sub.is_none() && self.decimals.is_empty() && self.major == *other
    }
}

/// The canonical, unpadded spelling: `07.1` displays as `7.1`, and `03-1` as
/// `3-01` (the ROADMAP spelling).
impl fmt::Display for PhaseNum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.major)?;
        if let Some(sub) = self.sub {
            write!(f, "-{sub:02}")?;
        }
        for seg in &self.decimals {
            write!(f, ".{seg}")?;
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
            "",
            "M-2",
            "AB-29",
            "4a",
            "7.",
            ".1",
            "7..1",
            "-1",
            "phase 3",
            "3-",
            "-01",
            "3--01",
            "3-01-02",
            "3.1-01",
            "3-01a",
            "3-99999999999",
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

    #[test]
    fn padded_is_the_directory_prefix_spelling() {
        let p = |s| PhaseNum::parse(s).unwrap();
        assert_eq!(p("7.1").padded(), "07.1");
        assert_eq!(p("5").padded(), "05");
        assert_eq!(p("13").padded(), "13");
        assert_eq!(p("999.2").major(), 999);
    }

    #[test]
    fn milestone_prefixed_ids_parse_as_one_phase_apart_from_decimals() {
        let p = |s| PhaseNum::parse(s).unwrap();
        assert_eq!(p("3-01"), p("03-01"));
        assert_eq!(p("3-01"), p("3-1"));
        assert_ne!(p("3-01"), p("3"));
        assert_ne!(p("3-01"), p("3.1"));
        assert_eq!(p("3-1").to_string(), "3-01");
        assert_eq!(p("3-01").padded(), "03-01");
        assert_eq!(p("3-01").major(), 3);
        assert!(p("3-01") != 3);
        assert_eq!(p("3-01.1").padded(), "03-01.1");
        assert_eq!(phase_key("03-01"), "3-01");
        assert!(same_phase("3-1", "03-01"));
    }

    #[test]
    fn milestone_prefixed_ids_order_after_their_milestone_and_its_decimals() {
        let p = |s| PhaseNum::parse(s).unwrap();
        assert!(p("3") < p("3.1"));
        assert!(p("3.1") < p("3-01"));
        assert!(p("3-01") < p("3-01.1"));
        assert!(p("3-01.1") < p("3-02"));
        assert!(p("3-02") < p("4"));
        assert!(p("3-09") < p("3-10"));
    }

    #[test]
    fn milestone_prefixed_convention_is_read_from_the_exact_config_value() {
        assert_eq!(
            PhaseIdConvention::from_config_value(Some("milestone-prefixed")),
            PhaseIdConvention::MilestonePrefixed
        );
        for other in [
            None,
            Some(""),
            Some("sequential"),
            Some("bracket"),
            Some("Milestone-Prefixed"),
        ] {
            assert_eq!(
                PhaseIdConvention::from_config_value(other),
                PhaseIdConvention::Legacy,
                "{other:?}"
            );
        }
        assert_eq!(PhaseIdConvention::default(), PhaseIdConvention::Legacy);
    }
}
