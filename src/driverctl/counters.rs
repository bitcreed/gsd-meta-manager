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

use super::log::LogEvent;

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
    // WR-07 `[inferred - audit]`: a digest is "seen" per phase. Two phases whose
    // VERIFICATION.md bytes are identical (a stub, a template) hash identically, and
    // keying by digest alone counted only the first phase's round. Keys are
    // `<phase>:<digest>`; a spike-layout key (the bare digest) still counts as seen
    // for the phase it records, so an older `state.json` is not double counted.
    let key = format!("{phase}:{digest}");
    if state.seen.contains_key(&key) || state.seen.get(digest).is_some_and(|w| w.phase == phase) {
        return (GapKind::Duplicate, count);
    }
    state.seen.insert(
        key,
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

/// Item states that close a dispatch (exitclass.py `DONE_STATES`).
pub const DONE_STATES: [&str; 3] = ["done", "parked", "failed"];

/// The latest open dispatch as `(item, event index)`, or `None` (D-06, WR-01).
///
/// Every item-level `dispatched` opens an entry, and a `report`, `recovered` or
/// terminal `status` (see [`DONE_STATES`]) closes all open entries of that
/// item. A `RUN | dispatch-denied` cancels exactly one entry: the most recent
/// open one, and only when no item-level event (any event whose item is not
/// `RUN`) lies between that dispatch and the denial. RUN-level launcher lines
/// (`gap-observed`, `connect`, `launch`, ...) never count as "between", so they
/// cannot shield a denied attempt; a denial cannot hide a different, genuinely
/// unreported dispatch.
pub fn outstanding_dispatch(events: &[LogEvent]) -> Option<(String, usize)> {
    // (event index, item) in dispatch order.
    let mut open: Vec<(usize, &str)> = Vec::new();
    // Index of the latest item-level event seen so far.
    let mut last_item_event: Option<usize> = None;
    for (n, e) in events.iter().enumerate() {
        if e.item != "RUN" {
            match e.event.as_str() {
                "dispatched" => open.push((n, e.item.as_str())),
                "report" | "recovered" => open.retain(|(_, item)| *item != e.item),
                "status" if e.get("state").is_some_and(|s| DONE_STATES.contains(&s)) => {
                    open.retain(|(_, item)| *item != e.item);
                }
                _ => {}
            }
            last_item_event = Some(n);
        } else if e.event == "dispatch-denied" && open.last().map(|(i, _)| *i) == last_item_event {
            open.pop();
        }
    }
    open.last().map(|&(n, item)| (item.to_owned(), n))
}

/// How a driver exit that is neither a cap trip nor a `RUN end` is classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitClass {
    /// The process exited cleanly with a worker still outstanding.
    Stalled,
    /// Anything else.
    Crashed,
}

impl ExitClass {
    pub fn as_str(self) -> &'static str {
        match self {
            ExitClass::Stalled => "stalled",
            ExitClass::Crashed => "crashed",
        }
    }
}

/// `None` keeps the existing status; otherwise the class and its reason (D-17).
///
/// A tripped cap or a driver-logged `RUN end` takes precedence. Otherwise a
/// clean exit (`rc == 0`) whose stream ended in `result success` with a
/// dispatch still outstanding is [`ExitClass::Stalled`] (never collapsed into
/// crashed: the process did not die, the driver ended its turn early); anything
/// else is [`ExitClass::Crashed`].
pub fn classify_exit(
    rc: i32,
    tripped: bool,
    ended_by_driver: bool,
    result_ok: bool,
    outstanding: Option<&str>,
) -> Option<(ExitClass, String)> {
    if tripped || ended_by_driver {
        return None;
    }
    if let (0, true, Some(item)) = (rc, result_ok, outstanding) {
        return Some((
            ExitClass::Stalled,
            format!(
                "driver ended its turn with a worker outstanding (dispatch {item} has no \
                 report in LOG.md); the process exited cleanly, so this is not a crash"
            ),
        ));
    }
    Some((
        ExitClass::Crashed,
        format!("driver exited rc={rc} without RUN end"),
    ))
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
    fn wr07_identical_bytes_in_two_phases_are_two_rounds() {
        let mut st = GapRoundsState::default();
        // Phase 1 and phase 2 each carry the same stub, far apart in time.
        assert_eq!(
            observe_gap(&mut st, "1", 1000.0, "stubdigest", Some("a")),
            (GapKind::New, 1)
        );
        assert_eq!(
            observe_gap(&mut st, "2", 2000.0, "stubdigest", Some("b")),
            (GapKind::New, 1),
            "phase 2's round must not collapse into phase 1's"
        );
        // Re-scanning both on the next tick moves nothing, in either order.
        assert_eq!(
            observe_gap(&mut st, "1", 1000.0, "stubdigest", Some("a")),
            (GapKind::Duplicate, 1)
        );
        assert_eq!(
            observe_gap(&mut st, "2", 2000.0, "stubdigest", Some("b")),
            (GapKind::Duplicate, 1)
        );
        assert_eq!(st.per_phase.get("1"), Some(&1));
        assert_eq!(st.per_phase.get("2"), Some(&1));
    }

    #[test]
    fn wr07_a_spike_layout_seen_key_is_still_a_duplicate_for_its_phase() {
        let mut st: GapRoundsState = serde_json::from_str(
            r#"{"per_phase":{"1":1},"seen":{"abc":{"phase":"1","file":"f"}}}"#,
        )
        .unwrap();
        assert_eq!(
            observe_gap(&mut st, "1", 5000.0, "abc", Some("f")),
            (GapKind::Duplicate, 1)
        );
        // The same digest under another phase is a fresh observation.
        assert_eq!(
            observe_gap(&mut st, "2", 5000.0, "abc", Some("g")),
            (GapKind::New, 1)
        );
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

    // -- outstanding_dispatch: the exitclass.py WR-01 cases ---------------------------

    fn ev(item: &str, event: &str, kv: &[(&str, &str)]) -> LogEvent {
        LogEvent {
            at: "2026-10-17T03:00:00Z".to_owned(),
            item: item.to_owned(),
            event: event.to_owned(),
            kv: kv
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
            detail: Vec::new(),
        }
    }

    fn dispatched(item: &str) -> LogEvent {
        ev(item, "dispatched", &[("step", "s")])
    }

    fn denied() -> LogEvent {
        ev("RUN", "dispatch-denied", &[("reason", "x")])
    }

    fn item_of(events: &[LogEvent]) -> Option<String> {
        outstanding_dispatch(events).map(|(item, _)| item)
    }

    fn open(item: &str, n: usize) -> Option<(String, usize)> {
        Some((item.to_owned(), n))
    }

    #[test]
    fn outstanding_item_open_and_close_rules() {
        let d = dispatched("I-01");
        assert_eq!(item_of(std::slice::from_ref(&d)).as_deref(), Some("I-01"));
        let close = |e: LogEvent| item_of(&[d.clone(), e]);
        assert_eq!(close(ev("I-01", "report", &[("status", "done")])), None);
        assert_eq!(close(ev("I-01", "status", &[("state", "done")])), None);
        assert_eq!(close(ev("I-01", "status", &[("state", "parked")])), None);
        assert_eq!(close(ev("I-01", "status", &[("state", "failed")])), None);
        assert_eq!(
            close(ev("I-01", "status", &[("state", "running")])).as_deref(),
            Some("I-01")
        );
        assert_eq!(close(ev("I-01", "recovered", &[("action", "park")])), None);
        assert_eq!(close(denied()), None);
    }

    #[test]
    fn denial_before_the_dispatch_does_not_cancel_it() {
        assert_eq!(
            item_of(&[denied(), dispatched("I-01")]).as_deref(),
            Some("I-01")
        );
    }

    #[test]
    fn a_redispatch_reopens_and_the_latest_dispatch_wins() {
        let d = dispatched("I-01");
        let rep = ev("I-01", "report", &[("status", "done")]);
        assert_eq!(
            item_of(&[d.clone(), rep, dispatched("I-01")]).as_deref(),
            Some("I-01")
        );
        assert_eq!(
            outstanding_dispatch(&[d, dispatched("I-02")]),
            open("I-02", 1)
        );
    }

    #[test]
    fn empty_event_list_has_nothing_outstanding() {
        assert_eq!(outstanding_dispatch(&[]), None);
    }

    #[test]
    fn wr01_a_denial_cancels_only_its_own_immediately_preceding_dispatch() {
        let d = dispatched("I-01");
        let d2 = ev("I-01", "dispatched", &[("step", "s2")]);
        // (a) two dispatches of one item, one denial: the first stays open.
        assert_eq!(
            outstanding_dispatch(&[d.clone(), d2.clone(), denied()]),
            open("I-01", 0)
        );
        // (b) one dispatch, one denial: nothing outstanding.
        assert_eq!(outstanding_dispatch(&[d.clone(), denied()]), None);
        // (c) a denial cannot hide a different item's unreported dispatch.
        assert_eq!(
            outstanding_dispatch(&[d.clone(), dispatched("I-02"), denied()]),
            open("I-01", 0)
        );
        // (d) a RUN-level line between dispatch and denial does not shield the denial.
        assert_eq!(
            outstanding_dispatch(&[
                d.clone(),
                ev("RUN", "gap-observed", &[("n", "1")]),
                denied()
            ]),
            None
        );
        // (e) a report closes the dispatch.
        assert_eq!(
            outstanding_dispatch(&[d.clone(), ev("I-01", "report", &[("status", "done")])]),
            None
        );
        // (f) report, re-dispatch, denial, re-dispatch: the last dispatch is the open one.
        assert_eq!(
            outstanding_dispatch(&[
                d.clone(),
                ev("I-01", "report", &[("status", "done")]),
                d2,
                denied(),
                ev("I-01", "dispatched", &[("step", "s3")]),
            ]),
            open("I-01", 4)
        );
        // An item-level event between the dispatch and the denial protects the dispatch.
        assert_eq!(
            outstanding_dispatch(&[d, ev("I-01", "status", &[("state", "running")]), denied()]),
            open("I-01", 0)
        );
    }

    // -- classify_exit ----------------------------------------------------------------

    #[test]
    fn classify_exit_matrix() {
        let class = |r: Option<(ExitClass, String)>| r.map(|(c, _)| c);
        // A tripped cap or a driver-logged `RUN end` keeps the existing status.
        assert_eq!(classify_exit(0, true, false, true, Some("I-01")), None);
        assert_eq!(classify_exit(0, false, true, true, None), None);
        assert_eq!(classify_exit(1, true, true, false, None), None);
        // Clean exit, result success, dispatch outstanding: stalled, naming the item.
        let (c, reason) = classify_exit(0, false, false, true, Some("I-01")).unwrap();
        assert_eq!(c, ExitClass::Stalled);
        assert_eq!(
            reason,
            "driver ended its turn with a worker outstanding (dispatch I-01 has no report in \
             LOG.md); the process exited cleanly, so this is not a crash"
        );
        // Everything else is a crash.
        let (c, reason) = classify_exit(0, false, false, false, Some("I-01")).unwrap();
        assert_eq!(c, ExitClass::Crashed);
        assert_eq!(reason, "driver exited rc=0 without RUN end");
        assert_eq!(
            class(classify_exit(1, false, false, true, Some("I-01"))),
            Some(ExitClass::Crashed)
        );
        assert_eq!(
            class(classify_exit(0, false, false, true, None)),
            Some(ExitClass::Crashed)
        );
        assert_eq!(
            class(classify_exit(-9, false, false, true, Some("I-01"))),
            Some(ExitClass::Crashed)
        );
        assert_eq!(
            classify_exit(137, false, false, true, None).unwrap().1,
            "driver exited rc=137 without RUN end"
        );
        assert_eq!(ExitClass::Stalled.as_str(), "stalled");
        assert_eq!(ExitClass::Crashed.as_str(), "crashed");
    }

    // -- the five exitclass replays over the committed spike-003 results ---------------

    const RUN1B_LOG: &str = include_str!("../../tests/fixtures/driverctl/spike003/run1b/LOG.md");
    const RUN1B_RES: &str =
        include_str!("../../tests/fixtures/driverctl/spike003/run1b/launcher-result-1.json");
    const RUN1_RES: &str =
        include_str!("../../tests/fixtures/driverctl/spike003/run1/launcher-result-1.json");
    const R261A_LOG: &str = include_str!("../../tests/fixtures/driverctl/spike003/r261a/LOG.md");
    const R261A_RES: &str =
        include_str!("../../tests/fixtures/driverctl/spike003/r261a/launcher-result-1.json");
    const R261_RUN2_LOG: &str =
        include_str!("../../tests/fixtures/driverctl/spike003/r261-run2/LOG.md");
    const R261_RUN2_RES: &str =
        include_str!("../../tests/fixtures/driverctl/spike003/r261-run2/launcher-result-1.json");
    const R261_L8_LOG: &str =
        include_str!("../../tests/fixtures/driverctl/spike003/r261-l8/LOG.md");
    const R261_L8_RES: &str =
        include_str!("../../tests/fixtures/driverctl/spike003/r261-l8/launcher-result-1.json");

    /// `exitclass.py replay` without a stream skeleton: (EXIT_CLASS, OUTSTANDING, RESULT_OK).
    fn exit_class_replay(
        log_text: &str,
        launcher_result: &serde_json::Value,
    ) -> (String, Option<String>, bool) {
        let events = parse_log(log_text);
        let recorded = launcher_result["status"].as_str().unwrap_or("").to_owned();
        let tripped = launcher_result.get("tripped").is_some_and(|t| {
            !(t.is_null() || t.as_bool() == Some(false) || t.as_str() == Some(""))
        });
        let result_ok = launcher_result["cost"]["subtype"].as_str() == Some("success");
        let ended = events.iter().any(|e| e.item == "RUN" && e.event == "end");
        if ended || tripped {
            return (recorded, None, result_ok);
        }
        let item = outstanding_dispatch(&events).map(|(i, _)| i);
        let rc = launcher_result["rc"].as_i64().unwrap() as i32;
        let verdict = classify_exit(rc, false, false, result_ok, item.as_deref());
        let status = verdict.map_or(recorded, |(c, _)| c.as_str().to_owned());
        (status, item, result_ok)
    }

    fn replay(log: &str, res: &str) -> (String, Option<String>, bool) {
        exit_class_replay(log, &serde_json::from_str(res).unwrap())
    }

    #[test]
    fn run1b_replay_classifies_stalled_with_i01_outstanding() {
        let (class, item, ok) = replay(RUN1B_LOG, RUN1B_RES);
        assert_eq!(class, "stalled");
        assert_eq!(item.as_deref(), Some("I-01"));
        assert!(ok);
    }

    #[test]
    fn run1_replay_keeps_the_recorded_parked_status() {
        let run1_log = include_str!("../../tests/fixtures/driverctl/spike003/run1/LOG.md");
        assert_eq!(replay(run1_log, RUN1_RES).0, "parked");
    }

    #[test]
    fn r261a_r261_run2_and_r261_l8_replays_keep_their_recorded_status() {
        assert_eq!(replay(R261A_LOG, R261A_RES).0, "parked");
        assert_eq!(replay(R261_RUN2_LOG, R261_RUN2_RES).0, "done");
        assert_eq!(replay(R261_L8_LOG, R261_L8_RES).0, "stopped");
    }

    #[test]
    fn run1b_shape_controls_are_never_stalled() {
        let base: serde_json::Value = serde_json::from_str(RUN1B_RES).unwrap();
        let with = |f: &dyn Fn(&mut serde_json::Value)| {
            let mut v = base.clone();
            f(&mut v);
            v
        };
        // rc != 0
        let v = with(&|v| v["rc"] = serde_json::json!(1));
        assert_eq!(exit_class_replay(RUN1B_LOG, &v).0, "crashed");
        // a tripped cap keeps the recorded status
        let v = with(&|v| v["tripped"] = serde_json::json!("gap_rounds"));
        assert_eq!(
            exit_class_replay(RUN1B_LOG, &v).0,
            base["status"].as_str().unwrap()
        );
        // the worker reported after all
        let reported = format!("{RUN1B_LOG}- 2026-10-17T03:07:00Z | I-01 | report | status=done\n");
        let (class, item, _) = exit_class_replay(&reported, &base);
        assert_eq!((class.as_str(), item), ("crashed", None));
        // the last dispatch was denied
        let denied_log =
            format!("{RUN1B_LOG}- 2026-10-17T03:07:00Z | RUN | dispatch-denied | reason=x\n");
        assert_eq!(exit_class_replay(&denied_log, &base).0, "crashed");
        // the stream did not end in success
        let v = with(&|v| v["cost"]["subtype"] = serde_json::json!("error_max_turns"));
        let (class, _, ok) = exit_class_replay(RUN1B_LOG, &v);
        assert_eq!((class.as_str(), ok), ("crashed", false));
    }
}
