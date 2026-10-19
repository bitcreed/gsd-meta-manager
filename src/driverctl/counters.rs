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
