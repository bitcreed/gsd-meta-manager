//! Schema v1 of the tool-written frontmatter block of a driver run's `DIRECTION.md`
//! (DCTL-03, D-15, D-24).
//!
//! # What this is
//!
//! [`DriverFrontmatter`] is the one typed description of the block between the two
//! `---` lines at the top of `<run-dir>/DIRECTION.md`. The same type is **built and
//! emitted** by the launcher-side store ([`DriverFrontmatter::emit`]) and **parsed**
//! by every reader ([`parse_driver_frontmatter`], Phase 30's Driver tab included).
//! One type for both halves means the writer cannot emit what the reader cannot
//! read: "format-correct by construction".
//!
//! The block is a **derived view** of the run's `state.json`, regenerated on every
//! run event. It is tool-written and never LLM-written; the body below the closing
//! `---` is human-owned and is not this module's concern.
//!
//! # Key map (D-15, D-24)
//!
//! Keys are emitted in this order, which is the order spike 002's generator wrote:
//! `generated_by`, `driver_frontmatter_version`, `run_id`, `status`,
//! `status_reason`, `control`, `current_item`, `current_phase`, `current_step`,
//! `items`, `item_states`, `caps`, `dispatch_audit`, `launches`, `connected`,
//! `heartbeat_stale_after_s`, `last_event`, `updated_at`. `item_states` (D-19) is
//! the one addition over the spike's generator; it sits right after `items`.
//!
//! # Reading is tolerant, with one exception
//!
//! The reader ignores unknown keys (no `deny_unknown_fields`), maps an unknown
//! `status` word to [`RunStatus::Unknown`], and defaults every field but
//! `generated_by`, `driver_frontmatter_version`, `run_id` and `status`, so the 17
//! spike-generated files (which have no `item_states`) parse. The exception is an
//! unknown `driver_frontmatter_version` major, which is rejected with
//! [`FrontmatterError::UnsupportedVersion`] rather than half-understood.
//!
//! # Schema evolution rule (D-19)
//!
//! **Adding a value is additive: a new key, a new `status` word or a new field in a
//! nested map is not a version change, because readers tolerate it. Renaming or
//! removing one is a major-version change: `driver_frontmatter_version` is bumped.**
//! [`ItemState`] is the deliberately closed vocabulary: it rejects an unknown word,
//! so a new item state is a coordinated change, not a silent one.
//!
//! # Parse only; emission is hand-written
//!
//! `serde_yml` is used to **parse** only. RUSTSEC-2025-0068 sits in its emitter
//! path, and a derived serializer cannot produce the spike generator's single-line
//! flow maps anyway. [`DriverFrontmatter::emit`] writes the text by hand: every
//! string double-quoted with `\` and `"` escaped and every control character
//! escaped as `\uXXXX`, so no raw newline can reach any value (T-27-09), nested
//! maps are single-line flow maps, and the closing `---` is on its own line.
//! The types still derive `Serialize` because the store persists several of them
//! (`RunStatus`, `ItemState`, `Connection`, the caps, `LastEvent`) in `state.json`
//! through `serde_json`.

use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Write as _};

use super::state_md::extract_frontmatter;

/// The schema major this build reads and writes (`driver_frontmatter_version`).
pub const DRIVER_FRONTMATTER_VERSION: u32 = 1;

/// Seconds after which a heartbeat is considered stale; recorded in the block so a
/// reader applies the same threshold the writer used.
pub const HEARTBEAT_STALE_AFTER_S: u64 = 300;

/// The trailing comment on the `generated_by` line.
pub const GENERATED_BY_COMMENT: &str =
    "# TOOL-WRITTEN. Do not edit: regenerated on every run event.";

fn default_stale_after() -> u64 {
    HEARTBEAT_STALE_AFTER_S
}

// ---------------------------------------------------------------- RunStatus

/// Where a run is in its life (D-05): spike 002's enum plus `stalled`.
///
/// Tolerant on read: a word this build does not know becomes
/// [`Unknown`](Self::Unknown), so a newer writer's status never fails the whole
/// block. It serializes as its bare word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStatus {
    /// The launcher has begun but the driver has not started.
    Starting,
    /// The driver is working.
    Running,
    /// Held by an operator.
    Paused,
    /// Stopped itself because it needs a human (terminal until resumed).
    Parked,
    /// Ended by `driver:stop` or a tripped cap.
    Stopped,
    /// Finished all its items.
    Done,
    /// The driver exited without ending the run.
    Crashed,
    /// The driver ended its turn cleanly while a worker was still unreported and no
    /// cap had tripped. Distinct from [`Crashed`](Self::Crashed).
    Stalled,
    /// A word this build does not know, kept verbatim.
    Unknown(String),
}

impl RunStatus {
    /// The bare word as written in the frontmatter.
    pub fn as_str(&self) -> &str {
        match self {
            RunStatus::Starting => "starting",
            RunStatus::Running => "running",
            RunStatus::Paused => "paused",
            RunStatus::Parked => "parked",
            RunStatus::Stopped => "stopped",
            RunStatus::Done => "done",
            RunStatus::Crashed => "crashed",
            RunStatus::Stalled => "stalled",
            RunStatus::Unknown(word) => word,
        }
    }

    /// Parse a status word; an unknown word becomes [`RunStatus::Unknown`].
    pub fn from_word(word: &str) -> Self {
        match word {
            "starting" => RunStatus::Starting,
            "running" => RunStatus::Running,
            "paused" => RunStatus::Paused,
            "parked" => RunStatus::Parked,
            "stopped" => RunStatus::Stopped,
            "done" => RunStatus::Done,
            "crashed" => RunStatus::Crashed,
            "stalled" => RunStatus::Stalled,
            other => RunStatus::Unknown(other.to_string()),
        }
    }

    /// Whether the run has ended and a free lock is expected: `stopped | done |
    /// crashed | stalled | parked` (27-CONTEXT: terminal statuses are shown as
    /// written). `Unknown` is not terminal, because nothing was established.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RunStatus::Stopped
                | RunStatus::Done
                | RunStatus::Crashed
                | RunStatus::Stalled
                | RunStatus::Parked
        )
    }
}

impl Serialize for RunStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RunStatus {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct WordVisitor;
        impl Visitor<'_> for WordVisitor {
            type Value = RunStatus;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a run status word")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<RunStatus, E> {
                Ok(RunStatus::from_word(v))
            }
        }
        deserializer.deserialize_str(WordVisitor)
    }
}

// ---------------------------------------------------------------- ItemState

/// One DIRECTION item's state (D-19). A **closed** vocabulary: an unknown word is a
/// parse error, never an `Unknown` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemState {
    Pending,
    Running,
    Done,
    Parked,
    Failed,
}

impl ItemState {
    /// Every state, in the order `items:` counts them.
    pub const ALL: [ItemState; 5] = [
        ItemState::Pending,
        ItemState::Running,
        ItemState::Done,
        ItemState::Parked,
        ItemState::Failed,
    ];

    /// The lowercase word as written in the frontmatter.
    pub fn as_str(self) -> &'static str {
        match self {
            ItemState::Pending => "pending",
            ItemState::Running => "running",
            ItemState::Done => "done",
            ItemState::Parked => "parked",
            ItemState::Failed => "failed",
        }
    }
}

// ---------------------------------------------------------------- plain records

/// The `items:` roll-up.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ItemCounts {
    pub total: u32,
    pub pending: u32,
    pub running: u32,
    pub done: u32,
    pub parked: u32,
    pub failed: u32,
}

/// One item's entry in `item_states`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStateEntry {
    pub state: ItemState,
    #[serde(default)]
    pub dispatches: u32,
    #[serde(default)]
    pub outcome: Option<String>,
}

/// `item_states`: item id to entry, in DIRECTION order.
///
/// A `Vec` of pairs rather than a map so document order survives parse and emit
/// (a round trip must never reorder it).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemStates(pub Vec<(String, ItemStateEntry)>);

impl Serialize for ItemStates {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (id, entry) in &self.0 {
            map.serialize_entry(id, entry)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for ItemStates {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct OrderedVisitor;
        impl<'de> Visitor<'de> for OrderedVisitor {
            type Value = ItemStates;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a map of item id to item state")
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<ItemStates, A::Error> {
                let mut out = Vec::new();
                while let Some((id, entry)) = map.next_entry::<String, ItemStateEntry>()? {
                    out.push((id, entry));
                }
                Ok(ItemStates(out))
            }
        }
        deserializer.deserialize_map(OrderedVisitor)
    }
}

/// A counted cap: `used` against an optional `limit`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Cap {
    pub used: u32,
    pub limit: Option<u32>,
}

/// The wall-clock cap, in minutes (fractional).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WallClockCap {
    pub used: f64,
    pub limit: Option<f64>,
}

/// The gap-closure rounds cap: the worst phase's rounds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GapCap {
    pub used: u32,
    pub limit: Option<u32>,
    pub phase: Option<String>,
}

/// The `caps:` block.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Caps {
    pub dispatches: Cap,
    pub wall_clock_min: WallClockCap,
    pub gap_rounds: GapCap,
}

/// The `dispatch_audit:` counters: dispatches logged by the driver, observed by the
/// hook, and denied.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DispatchAudit {
    pub logged: u32,
    pub observed: u32,
    pub denied: u32,
}

/// The `last_event:` flow map.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LastEvent {
    pub at: Option<String>,
    pub item: Option<String>,
    pub event: Option<String>,
    pub summary: Option<String>,
}

/// One entry of `connected:` (the launcher, the driver, a previous driver).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Connection {
    pub role: String,
    pub state: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub mode: Option<String>,
    pub pid: Option<u32>,
    pub scope_unit: Option<String>,
    pub host: Option<String>,
    pub started_at: Option<String>,
    pub last_heartbeat: Option<String>,
}

// ---------------------------------------------------------------- the block

/// The whole v1 block. See the module documentation for the key map and the
/// schema-evolution rule (adding a value is additive; renaming or removing one is a
/// `driver_frontmatter_version` major bump).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DriverFrontmatter {
    /// `driverctl <version>`; callers pass that form so the `generated_by: driverctl`
    /// marker survives.
    pub generated_by: String,
    pub driver_frontmatter_version: u32,
    pub run_id: String,
    pub status: RunStatus,
    #[serde(default)]
    pub status_reason: Option<String>,
    #[serde(default)]
    pub control: Option<String>,
    #[serde(default)]
    pub current_item: Option<String>,
    #[serde(default)]
    pub current_phase: Option<String>,
    #[serde(default)]
    pub current_step: Option<String>,
    #[serde(default)]
    pub items: ItemCounts,
    /// D-19: absent in the 17 spike-generated files, hence `default`.
    #[serde(default)]
    pub item_states: ItemStates,
    #[serde(default)]
    pub caps: Caps,
    #[serde(default)]
    pub dispatch_audit: DispatchAudit,
    #[serde(default)]
    pub launches: u32,
    #[serde(default)]
    pub connected: Vec<Connection>,
    #[serde(default = "default_stale_after")]
    pub heartbeat_stale_after_s: u64,
    #[serde(default)]
    pub last_event: LastEvent,
    #[serde(default)]
    pub updated_at: String,
}

// ---------------------------------------------------------------- errors + parse

/// Why a `DIRECTION.md` frontmatter did not parse. Module-local and typed: a
/// hostile or hand-edited block yields one of these, never a panic (T-27-10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontmatterError {
    /// The text has no `---` ... `---` block.
    NoFrontmatter,
    /// A block exists but no line starts `generated_by: driverctl`.
    NotDriverctl,
    /// The block is not valid for the schema.
    Yaml { detail: String },
    /// `driver_frontmatter_version` names a major this build does not read.
    UnsupportedVersion { found: u32 },
}

impl fmt::Display for FrontmatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrontmatterError::NoFrontmatter => f.write_str("no frontmatter block"),
            FrontmatterError::NotDriverctl => {
                f.write_str("frontmatter block is not written by driverctl")
            }
            FrontmatterError::Yaml { detail } => write!(f, "invalid driver frontmatter: {detail}"),
            FrontmatterError::UnsupportedVersion { found } => write!(
                f,
                "unsupported driver_frontmatter_version {found} (this build reads {DRIVER_FRONTMATTER_VERSION})"
            ),
        }
    }
}

impl std::error::Error for FrontmatterError {}

/// The two fields read before committing to the full schema, so an unknown major is
/// reported as such instead of as whatever field changed shape.
#[derive(Deserialize)]
struct VersionProbe {
    #[allow(dead_code)]
    generated_by: String,
    driver_frontmatter_version: u32,
}

/// Parse the tool-written block of a `DIRECTION.md` (or any text starting with it).
pub fn parse_driver_frontmatter(text: &str) -> Result<DriverFrontmatter, FrontmatterError> {
    let block = extract_frontmatter(text).ok_or(FrontmatterError::NoFrontmatter)?;
    if !block
        .lines()
        .any(|line| line.starts_with("generated_by: driverctl"))
    {
        return Err(FrontmatterError::NotDriverctl);
    }
    let probe: VersionProbe = serde_yml::from_str(block).map_err(|e| FrontmatterError::Yaml {
        detail: e.to_string(),
    })?;
    if probe.driver_frontmatter_version != DRIVER_FRONTMATTER_VERSION {
        return Err(FrontmatterError::UnsupportedVersion {
            found: probe.driver_frontmatter_version,
        });
    }
    serde_yml::from_str(block).map_err(|e| FrontmatterError::Yaml {
        detail: e.to_string(),
    })
}

// ---------------------------------------------------------------- emission

/// A YAML double-quoted scalar: `\` and `"` escaped, every control character (C0,
/// DEL, C1) escaped as `\uXXXX`. U+2028 and U+2029 are escaped too: they are not
/// `char::is_control` but libyaml treats them as line breaks.
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if c.is_control() || c == '\u{2028}' || c == '\u{2029}' => {
                let _ = write!(out, "\\u{:04X}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn quote_or_null(v: &Option<String>) -> String {
    v.as_deref().map_or_else(|| "null".to_string(), quote)
}

fn num_or_null<T: fmt::Display>(v: Option<T>) -> String {
    v.map_or_else(|| "null".to_string(), |n| n.to_string())
}

/// An `f64` via Rust `Display`; non-finite has no YAML spelling a reader accepts as
/// a number here, so it is written `null`.
fn float(v: f64) -> String {
    if v.is_finite() {
        v.to_string()
    } else {
        "null".to_string()
    }
}

fn float_or_null(v: Option<f64>) -> String {
    v.map_or_else(|| "null".to_string(), float)
}

impl DriverFrontmatter {
    /// The complete block, `---\n` through `---\n`, in D-24 key order.
    ///
    /// Hand-written (see the module documentation): no raw newline can appear in
    /// any value and serde_yml is never used to emit.
    pub fn emit(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push("---".to_string());
        // The value is written plain so the `generated_by: driverctl` marker
        // survives; control characters are dropped so it cannot span lines.
        let generated_by: String = self
            .generated_by
            .chars()
            .filter(|c| !c.is_control())
            .collect();
        lines.push(format!(
            "generated_by: {generated_by}   {GENERATED_BY_COMMENT}"
        ));
        lines.push(format!(
            "driver_frontmatter_version: {}",
            self.driver_frontmatter_version
        ));
        lines.push(format!("run_id: {}", quote(&self.run_id)));
        let status = match &self.status {
            RunStatus::Unknown(word) => quote(word),
            known => known.as_str().to_string(),
        };
        lines.push(format!("status: {status}"));
        lines.push(format!(
            "status_reason: {}",
            quote_or_null(&self.status_reason)
        ));
        lines.push(format!("control: {}", quote_or_null(&self.control)));
        lines.push(format!(
            "current_item: {}",
            quote_or_null(&self.current_item)
        ));
        lines.push(format!(
            "current_phase: {}",
            quote_or_null(&self.current_phase)
        ));
        lines.push(format!(
            "current_step: {}",
            quote_or_null(&self.current_step)
        ));
        let i = &self.items;
        lines.push(format!(
            "items: {{total: {}, pending: {}, running: {}, done: {}, parked: {}, failed: {}}}",
            i.total, i.pending, i.running, i.done, i.parked, i.failed
        ));
        if self.item_states.0.is_empty() {
            lines.push("item_states: {}".to_string());
        } else {
            lines.push("item_states:".to_string());
            for (id, e) in &self.item_states.0 {
                lines.push(format!(
                    "  {}: {{state: {}, dispatches: {}, outcome: {}}}",
                    quote(id),
                    quote(e.state.as_str()),
                    e.dispatches,
                    quote_or_null(&e.outcome)
                ));
            }
        }
        let c = &self.caps;
        lines.push("caps:".to_string());
        lines.push(format!(
            "  dispatches: {{used: {}, limit: {}}}",
            c.dispatches.used,
            num_or_null(c.dispatches.limit)
        ));
        lines.push(format!(
            "  wall_clock_min: {{used: {}, limit: {}}}",
            float(c.wall_clock_min.used),
            float_or_null(c.wall_clock_min.limit)
        ));
        lines.push(format!(
            "  gap_rounds: {{used: {}, limit: {}, phase: {}}}",
            c.gap_rounds.used,
            num_or_null(c.gap_rounds.limit),
            quote_or_null(&c.gap_rounds.phase)
        ));
        let a = &self.dispatch_audit;
        lines.push(format!(
            "dispatch_audit: {{logged: {}, observed: {}, denied: {}}}",
            a.logged, a.observed, a.denied
        ));
        lines.push(format!("launches: {}", self.launches));
        if self.connected.is_empty() {
            lines.push("connected: []".to_string());
        } else {
            lines.push("connected:".to_string());
            for k in &self.connected {
                lines.push(format!(
                    "  - {{role: {}, state: {}, session_id: {}, model: {}, mode: {}, pid: {}, \
                     scope_unit: {}, host: {}, started_at: {}, last_heartbeat: {}}}",
                    quote(&k.role),
                    quote(&k.state),
                    quote_or_null(&k.session_id),
                    quote_or_null(&k.model),
                    quote_or_null(&k.mode),
                    num_or_null(k.pid),
                    quote_or_null(&k.scope_unit),
                    quote_or_null(&k.host),
                    quote_or_null(&k.started_at),
                    quote_or_null(&k.last_heartbeat)
                ));
            }
        }
        lines.push(format!(
            "heartbeat_stale_after_s: {}",
            self.heartbeat_stale_after_s
        ));
        let e = &self.last_event;
        lines.push(format!(
            "last_event: {{at: {}, item: {}, event: {}, summary: {}}}",
            quote_or_null(&e.at),
            quote_or_null(&e.item),
            quote_or_null(&e.event),
            quote_or_null(&e.summary)
        ));
        lines.push(format!("updated_at: {}", quote(&self.updated_at)));
        lines.push("---".to_string());
        lines.push(String::new());
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RUN1B: &str = include_str!("../../tests/fixtures/driverctl/direction/s003-run1b.md");

    #[test]
    fn spike_run1b_parses_and_reemits_equal() {
        let parsed = parse_driver_frontmatter(RUN1B).expect("spike frontmatter parses");
        assert_eq!(parsed.status, RunStatus::Crashed);
        assert_eq!(parsed.run_id, "s003-run1b");
        assert_eq!(parsed.connected.len(), 2);
        assert_eq!(parsed.connected[0].role, "launcher");
        assert_eq!(parsed.connected[1].role, "driver");
        assert!(parsed.item_states.0.is_empty());
        assert_eq!(parsed.launches, 1);

        let emitted = parsed.emit();
        assert!(emitted.starts_with("---\ngenerated_by: driverctl 0.2-spike   # TOOL-WRITTEN"));
        assert!(emitted.ends_with("\n---\n"));
        let again = parse_driver_frontmatter(&emitted).expect("emitted block parses");
        assert_eq!(again, parsed);
    }

    #[test]
    fn quote_escapes_control_and_quotes() {
        let nasty = "line1\nline2\u{7f}say \"hi\" \\ caf\u{e9} \u{2603} \u{85}\u{2028}end";
        let q = quote(nasty);
        assert!(!q.contains('\n'));
        assert!(!q.chars().any(|c| c.is_control()));
        assert!(q.contains("\\u000A"));
        assert!(q.contains("\\u007F"));

        let mut fm = parse_driver_frontmatter(RUN1B).unwrap();
        fm.status_reason = Some(nasty.to_string());
        fm.item_states.0.push((
            "I-\"1\"".to_string(),
            ItemStateEntry {
                state: ItemState::Parked,
                dispatches: 2,
                outcome: Some("a\nb: c\n---\nd".to_string()),
            },
        ));
        let emitted = fm.emit();
        assert_eq!(emitted.lines().filter(|l| *l == "---").count(), 2);
        let back = parse_driver_frontmatter(&emitted).expect("nasty values round-trip");
        assert_eq!(back, fm);
        assert_eq!(back.status_reason.as_deref(), Some(nasty));
    }

    #[test]
    fn unknown_status_is_tolerated() {
        let text = RUN1B.replace("status: crashed", "status: levitating");
        let parsed = parse_driver_frontmatter(&text).expect("unknown status tolerated");
        assert_eq!(parsed.status, RunStatus::Unknown("levitating".to_string()));
        assert!(!parsed.status.is_terminal());
        let back = parse_driver_frontmatter(&parsed.emit()).unwrap();
        assert_eq!(back.status, RunStatus::Unknown("levitating".to_string()));
    }

    #[test]
    fn unknown_major_is_rejected() {
        let text = RUN1B.replace(
            "driver_frontmatter_version: 1",
            "driver_frontmatter_version: 2",
        );
        assert_eq!(
            parse_driver_frontmatter(&text),
            Err(FrontmatterError::UnsupportedVersion { found: 2 })
        );
    }

    #[test]
    fn unknown_key_is_tolerated() {
        let text = RUN1B.replace("launches: 1\n", "launches: 1\nfuture_key: {a: 1}\n");
        assert!(parse_driver_frontmatter(&text).is_ok());
    }

    #[test]
    fn closed_item_state_rejects_unknown_word() {
        let text = RUN1B.replace(
            "caps:\n",
            "item_states:\n  \"I-01\": {state: \"levitating\", dispatches: 0, outcome: null}\ncaps:\n",
        );
        assert!(matches!(
            parse_driver_frontmatter(&text),
            Err(FrontmatterError::Yaml { .. })
        ));
    }
}
