//! The tool-observed counting rules: gap rounds (this file's first half) and,
//! below it, outstanding-dispatch accounting and exit classification.
//!
//! Ported line-for-line from the spike-003 Python tools so the numbers the 26.1
//! re-run measured stay the numbers Rust enforces (D-08, D-09, D-17).

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Two `gaps_found` writes of one phase less than this many seconds apart are
/// one gap round. Strict `<`, float seconds, no rounding or truncation.
///
/// `[inferred - audit]` measured verifier re-write gap 6.5 s (transcript) / 8 s
/// (LOG); shortest legitimate gap between two gap rounds 534 s (26.1-RESEARCH
/// A1). 45 s sits well inside that interval on both sides.
pub const COALESCE_S: f64 = 45.0;

/// One digest the counter has already observed (the `caps.gap_rounds.seen` value).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeenWrite {
    pub phase: String,
    #[serde(default)]
    pub file: Option<String>,
}

/// The exact `caps.gap_rounds` shape of a run's `state.json`.
///
/// Every field defaults so the spike-002 shape (only `limit`, `per_phase` and
/// `seen`) still loads; a relaunch that reloads the state keeps the window.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GapRoundsState {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub seen: BTreeMap<String, SeenWrite>,
    #[serde(default)]
    pub per_phase: BTreeMap<String, u32>,
    #[serde(default)]
    pub last_write: BTreeMap<String, f64>,
    #[serde(default)]
    pub coalesced: BTreeMap<String, u32>,
}

/// What one observation did to the counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapKind {
    /// A digest already seen: moves neither the count nor the anchor.
    Duplicate,
    /// Within [`COALESCE_S`] of the previous write: the anchor slides, the count stays.
    Coalesced,
    /// A new round: the phase count goes up by one.
    New,
}

impl GapKind {
    pub fn as_str(self) -> &'static str {
        match self {
            GapKind::Duplicate => "duplicate",
            GapKind::Coalesced => "coalesced",
            GapKind::New => "new",
        }
    }
}

/// Observe one `gaps_found` VERIFICATION.md write; returns the kind and the
/// phase's round count after it (port of `GapCounter.observe`).
pub fn observe_gap(
    state: &mut GapRoundsState,
    phase: &str,
    ts: f64,
    digest: &str,
    file: Option<&str>,
) -> (GapKind, u32) {
    let count = state.per_phase.get(phase).copied().unwrap_or(0);
    if state.seen.contains_key(digest) {
        return (GapKind::Duplicate, count);
    }
    state.seen.insert(
        digest.to_owned(),
        SeenWrite {
            phase: phase.to_owned(),
            file: file.map(str::to_owned),
        },
    );
    let prev = state.last_write.get(phase).copied();
    if let Some(prev) = prev {
        if count >= 1 && (ts - prev).abs() < COALESCE_S {
            // Sliding anchor, never moves backwards.
            state.last_write.insert(phase.to_owned(), prev.max(ts));
            *state.coalesced.entry(phase.to_owned()).or_insert(0) += 1;
            return (GapKind::Coalesced, count);
        }
    }
    state
        .last_write
        .insert(phase.to_owned(), prev.map_or(ts, |p| p.max(ts)));
    state.per_phase.insert(phase.to_owned(), count + 1);
    (GapKind::New, count + 1)
}

/// The phase with the most gap rounds, ties to the most recent `last_write`
/// `[inferred - audit]` (the Python relied on dict insertion order); remaining
/// ties go to the first phase key in sorted order. `(None, 0)` when empty.
pub fn worst_phase(state: &GapRoundsState) -> (Option<String>, u32) {
    let mut best: Option<(&String, u32, f64)> = None;
    for (phase, &count) in &state.per_phase {
        let at = state
            .last_write
            .get(phase)
            .copied()
            .unwrap_or(f64::NEG_INFINITY);
        let better = match best {
            None => true,
            Some((_, bc, bt)) => count > bc || (count == bc && at > bt),
        };
        if better {
            best = Some((phase, count, at));
        }
    }
    match best {
        Some((phase, count, _)) => (Some(phase.clone()), count),
        None => (None, 0),
    }
}

static STATUS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?m)^status:\s*["']?([A-Za-z_]+)"#).expect("static regex"));

static PHASE_KEY_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^0*(\d+(?:\.\d+)?)").expect("static regex"));

/// The one status definition (D-27): the `status:` value in the leading
/// frontmatter of a VERIFICATION.md, or `None`.
///
/// The frontmatter is the text before the first `\n---`, and only when the text
/// starts with `---`; a `status:` line in the body does not count, the value
/// may be quoted, the decode is lossy, and a BOM-prefixed file yields none.
pub fn fm_status(raw: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(raw);
    if !text.starts_with("---") {
        return None;
    }
    let fm = text.split("\n---").next().unwrap_or("");
    STATUS_RE.captures(fm).map(|c| c[1].to_owned())
}

/// The phase key of a phase directory name: leading zeros dropped, a decimal
/// part kept (`01-slugify` -> `1`, `26.1-x` -> `26.1`, `notes` -> `None`).
pub fn phase_key(dir_name: &str) -> Option<String> {
    PHASE_KEY_RE.captures(dir_name).map(|c| c[1].to_owned())
}

/// `sha256(raw)` as hex, first 16 characters.
pub fn digest16(raw: &[u8]) -> String {
    use std::fmt::Write as _;

    let mut hasher = Sha256::new();
    hasher.update(raw);
    let mut out = String::with_capacity(16);
    for byte in hasher.finalize().iter().take(8) {
        // `write!` to a String is infallible.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driverctl::log::{epoch_seconds, parse_log};

    const T: f64 = 1_000_000.0;

    fn obs(st: &mut GapRoundsState, phase: &str, ts: f64, digest: &str) -> (GapKind, u32) {
        observe_gap(st, phase, ts, digest, None)
    }

    // -- the gapcount selftest value table, one test per case family --------------

    #[test]
    fn rounds_534_seconds_apart_stay_separate_and_drive_the_s2_predicates() {
        let mut st = GapRoundsState::default();
        let got: Vec<_> = [T, T + 534.0, T + 1068.0]
            .iter()
            .enumerate()
            .map(|(i, &t)| obs(&mut st, "1", t, &format!("d{i}")))
            .collect();
        assert_eq!(
            got,
            [(GapKind::New, 1), (GapKind::New, 2), (GapKind::New, 3)]
        );
        // S2 predicates: deny when n >= gl + 1, trip when n > gl + 1.
        let gl = 2;
        let preds: Vec<_> = got.iter().map(|&(_, n)| (n > gl, n > gl + 1)).collect();
        assert_eq!(preds, [(false, false), (false, false), (true, false)]);
        let four = 4;
        assert_eq!((four > gl, four > gl + 1), (true, true)); // kill only at L+2
    }

    #[test]
    fn window_boundary_is_strict_and_unrounded() {
        let mut st = GapRoundsState::default();
        assert_eq!(obs(&mut st, "1", T, "a"), (GapKind::New, 1));
        assert_eq!(obs(&mut st, "1", T + 44.9, "b"), (GapKind::Coalesced, 1));

        let mut st = GapRoundsState::default();
        assert_eq!(obs(&mut st, "1", T, "a"), (GapKind::New, 1));
        assert_eq!(obs(&mut st, "1", T + 45.0, "b"), (GapKind::New, 2));

        let mut st = GapRoundsState::default();
        assert_eq!(obs(&mut st, "1", T, "a"), (GapKind::New, 1));
        assert_eq!(obs(&mut st, "1", T + 44.999999, "b").0, GapKind::Coalesced);
    }

    #[test]
    fn sliding_anchor_makes_five_writes_20s_apart_one_round() {
        let mut st = GapRoundsState::default();
        let kinds: Vec<_> = [0.0, 20.0, 40.0, 60.0, 80.0]
            .iter()
            .enumerate()
            .map(|(i, off)| obs(&mut st, "1", T + off, &format!("d{i}")).0)
            .collect();
        assert_eq!(
            kinds,
            [
                GapKind::New,
                GapKind::Coalesced,
                GapKind::Coalesced,
                GapKind::Coalesced,
                GapKind::Coalesced
            ]
        );
        assert_eq!(st.per_phase.get("1"), Some(&1));
        assert_eq!(st.coalesced.get("1"), Some(&4));
        // 45 s after the slid anchor opens a new round, far from the first write.
        assert_eq!(obs(&mut st, "1", T + 80.0 + 45.0, "z"), (GapKind::New, 2));
    }

    #[test]
    fn duplicate_digest_leaves_count_and_anchor() {
        let mut st = GapRoundsState::default();
        obs(&mut st, "1", T, "same");
        let anchor = st.last_write["1"];
        assert_eq!(
            obs(&mut st, "1", T + 100.0, "same"),
            (GapKind::Duplicate, 1)
        );
        assert_eq!(st.last_write["1"], anchor);
        assert_eq!(st.per_phase.get("1"), Some(&1));
        assert_eq!(obs(&mut st, "1", T + 100.0, "other"), (GapKind::New, 2));
    }

    #[test]
    fn a_write_older_than_the_anchor_never_moves_it_backwards() {
        let mut st = GapRoundsState::default();
        obs(&mut st, "1", T, "a");
        // Within the window but earlier than the anchor: coalesced, anchor stays.
        assert_eq!(obs(&mut st, "1", T - 10.0, "b"), (GapKind::Coalesced, 1));
        assert_eq!(st.last_write["1"], T);
        // Far earlier than the anchor: a new round, anchor still max(prev, ts).
        assert_eq!(obs(&mut st, "1", T - 500.0, "c"), (GapKind::New, 2));
        assert_eq!(st.last_write["1"], T);
    }

    #[test]
    fn phases_count_independently() {
        let mut st = GapRoundsState::default();
        assert_eq!(obs(&mut st, "1", T, "a"), (GapKind::New, 1));
        assert_eq!(obs(&mut st, "2", T + 1.0, "b"), (GapKind::New, 1));
    }

    #[test]
    fn json_round_trip_keeps_the_window_across_a_relaunch() {
        let mut st = GapRoundsState::default();
        observe_gap(&mut st, "1", T, "a", Some("f.md"));
        let json = serde_json::to_string(&st).unwrap();
        let mut st2: GapRoundsState = serde_json::from_str(&json).unwrap();
        assert_eq!(st2, st);
        assert_eq!(obs(&mut st2, "1", T + 10.0, "b"), (GapKind::Coalesced, 1));
        assert_eq!(obs(&mut st2, "1", T + 10.0 + 600.0, "c"), (GapKind::New, 2));
        assert_eq!(obs(&mut st2, "1", T + 10.0, "b"), (GapKind::Duplicate, 2));
    }

    #[test]
    fn s2_shaped_state_without_window_fields_loads_and_counts() {
        let mut st: GapRoundsState =
            serde_json::from_str(r#"{"limit": 2, "per_phase": {}, "seen": {}}"#).unwrap();
        assert_eq!(st.limit, Some(2));
        assert_eq!(obs(&mut st, "1", T, "a"), (GapKind::New, 1));
        assert!(st.last_write.contains_key("1"));
        let bare: GapRoundsState = serde_json::from_str("{}").unwrap();
        assert_eq!(bare, GapRoundsState::default());
    }

    #[test]
    fn worst_phase_picks_the_highest_count() {
        let mut st = GapRoundsState::default();
        assert_eq!(worst_phase(&st), (None, 0));
        obs(&mut st, "1", T, "a");
        obs(&mut st, "2", T + 1.0, "b");
        obs(&mut st, "2", T + 1000.0, "c");
        assert_eq!(worst_phase(&st), (Some("2".to_owned()), 2));
    }

    #[test]
    fn worst_phase_ties_go_to_the_latest_write() {
        let mut st = GapRoundsState::default();
        obs(&mut st, "1", T + 500.0, "a");
        obs(&mut st, "2", T, "b");
        assert_eq!(worst_phase(&st), (Some("1".to_owned()), 1));
        obs(&mut st, "3", T + 900.0, "c");
        assert_eq!(worst_phase(&st), (Some("3".to_owned()), 1));
    }

    // -- the one status / phase-key / digest definitions ----------------------------

    #[test]
    fn fm_status_is_quote_tolerant_and_frontmatter_only() {
        let some = |s: &str| Some(s.to_owned());
        assert_eq!(
            fm_status(b"---\nstatus: \"gaps_found\"\n---\n"),
            some("gaps_found")
        );
        assert_eq!(
            fm_status(b"---\nstatus: gaps_found\n---\nx"),
            some("gaps_found")
        );
        assert_eq!(
            fm_status(b"---\nphase: x\nstatus: 'passed'\n---\n"),
            some("passed")
        );
        assert_eq!(fm_status(b"status: gaps_found\n"), None);
        assert_eq!(fm_status(b"---\nphase: x\n---\nstatus: gaps_found\n"), None);
        assert_eq!(fm_status(b"\xff\xfe---\nstatus: gaps_found\n"), None);
        assert_eq!(fm_status(b""), None);
    }

    #[test]
    fn phase_key_drops_leading_zeros_and_keeps_a_decimal_part() {
        assert_eq!(phase_key("01-slugify").as_deref(), Some("1"));
        assert_eq!(phase_key("26.1-driver").as_deref(), Some("26.1"));
        assert_eq!(phase_key("27-driverctl").as_deref(), Some("27"));
        assert_eq!(phase_key("00-x").as_deref(), Some("0"));
        assert_eq!(phase_key("010-x").as_deref(), Some("10"));
        assert_eq!(phase_key("notes"), None);
        assert_eq!(phase_key(""), None);
    }

    #[test]
    fn digest16_is_the_first_16_hex_of_sha256() {
        assert_eq!(digest16(b""), "e3b0c44298fc1c14");
        assert_eq!(digest16(b"abc"), "ba7816bf8f01cfea");
    }

    // -- replays over committed copies of the spike-003 evidence ---------------------

    const RUN1_LOG: &str = include_str!("../../tests/fixtures/driverctl/spike003/run1/LOG.md");
    const R261A_STATE: &str =
        include_str!("../../tests/fixtures/driverctl/spike003/r261a/state.json");

    #[test]
    fn run1_replay_coalesces_the_sed_rewrite() {
        let events = parse_log(RUN1_LOG);
        let gap_events: Vec<_> = events
            .iter()
            .filter(|e| e.item == "RUN" && e.event == "gap-observed" && e.get("phase") == Some("1"))
            .collect();
        let old: Vec<u32> = gap_events
            .iter()
            .map(|e| e.get("count").unwrap().parse().unwrap())
            .collect();

        let mut st = GapRoundsState::default();
        let mut new = Vec::new();
        let mut coalesced = 0;
        for (n, e) in gap_events.iter().enumerate() {
            let ts = epoch_seconds(&e.at).expect("fixture timestamp");
            match observe_gap(&mut st, "1", ts, &format!("obs{n}"), e.get("file")) {
                (GapKind::New, k) => new.push(k),
                (GapKind::Coalesced, _) => coalesced += 1,
                (GapKind::Duplicate, _) => {}
            }
        }
        assert_eq!(old, [1, 2, 3]);
        assert_eq!(new, [1, 2]);
        assert_eq!(coalesced, 1);
    }

    #[test]
    fn r261a_state_shape_loads() {
        let doc: serde_json::Value = serde_json::from_str(R261A_STATE).unwrap();
        let gap = doc["caps"]["gap_rounds"].clone();
        let st: GapRoundsState = serde_json::from_value(gap).unwrap();
        assert_eq!(st.limit, Some(2));
        assert_eq!(st.per_phase, BTreeMap::from([("1".to_owned(), 3)]));
        assert_eq!(st.coalesced, BTreeMap::from([("1".to_owned(), 2)]));
        assert_eq!(st.seen.len(), 5);
        assert_eq!(st.last_write["1"], 1792290232.7904544);
        assert_eq!(
            st.seen["06110c788ecfe659"].file.as_deref(),
            Some(".planning/phases/01-slugify/01-VERIFICATION.md")
        );

        let back = serde_json::to_value(&st).unwrap();
        let keys: std::collections::BTreeSet<String> =
            back.as_object().unwrap().keys().cloned().collect();
        let want: std::collections::BTreeSet<String> =
            ["limit", "seen", "per_phase", "last_write", "coalesced"]
                .iter()
                .map(|s| (*s).to_owned())
                .collect();
        assert_eq!(keys, want);
        let again: GapRoundsState = serde_json::from_value(back).unwrap();
        assert_eq!(again, st);
    }
}
