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
//! `sandbox` (`bwrap-scope` | `degraded`) and `sandbox_missing` (D-30, Phase 28)
//! are the second: emitted right after `launches`, **only when set**, so a block
//! that predates the launcher keeps its bytes. `prompt_version` (Phase 29, D-03:
//! `driver-prompt/1.0.0` or `custom:<sha8>`) is the third, right after the sandbox
//! keys, under the same only-when-set rule.
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
    /// How the launcher contained the driver: `bwrap-scope` or `degraded` (28-04,
    /// D-30). Additive: absent from every frontmatter written before the launcher
    /// existed, and emitted only when set so those stay byte-identical.
    #[serde(default)]
    pub sandbox: Option<String>,
    /// What the host probes found missing when `sandbox` is `degraded`.
    #[serde(default)]
    pub sandbox_missing: Option<Vec<String>>,
    /// The master-prompt version this launch ran: `driver-prompt/1.0.0` for the
    /// embedded prompt, `custom:<sha8>` for an operator-supplied one (Phase 29,
    /// D-03). Additive like `sandbox`: absent from every frontmatter written
    /// before Phase 29 and emitted only when set, so those stay byte-identical.
    #[serde(default)]
    pub prompt_version: Option<String>,
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
    /// The run directory's `DIRECTION.md` could not be read ([`read_driver_run`]).
    Io { detail: String },
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
            FrontmatterError::Io { detail } => write!(f, "cannot read DIRECTION.md: {detail}"),
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
        // D-30: additive keys, written only when set so every frontmatter that
        // predates the launcher stays byte-identical.
        if let Some(sandbox) = &self.sandbox {
            lines.push(format!("sandbox: {}", quote(sandbox)));
        }
        if let Some(missing) = &self.sandbox_missing {
            let items: Vec<String> = missing.iter().map(|m| quote(m)).collect();
            lines.push(format!("sandbox_missing: [{}]", items.join(", ")));
        }
        // Phase 29 D-03: the same additive rule, no schema version bump.
        if let Some(version) = &self.prompt_version {
            lines.push(format!("prompt_version: {}", quote(version)));
        }
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

// ---------------------------------------------------------------- liveness

/// A run's status as the TUI should show it: the written status, corrected by what
/// the run lock says about the process (D-07, corrected by D-20/D-22).
///
/// **Never persisted.** `Crashed` here is a reading, not a record: nothing in this
/// module writes it back to `DIRECTION.md` or `state.json`, so a run that was merely
/// between launches, or whose lock was momentarily unreadable, is never marked
/// crashed on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedStatus {
    /// The written status is the answer: it is a status a lock cannot contradict
    /// (`paused`, any terminal one, an unknown word), or the lock agrees with it
    /// (`running` or `starting` with the run lock held).
    AsWritten(RunStatus),
    /// `starting` and the driver has never been launched (`launches == 0`): a fresh
    /// run directory, not a crash.
    NotLaunched,
    /// `running`, or `starting` after at least one launch, but the run lock is free
    /// or absent: the launcher is gone without having written a terminal status.
    Crashed { written: RunStatus },
    /// The lock probe could not tell (an errno other than `EWOULDBLOCK`). Neither
    /// crashed nor alive: `Unknown` is never a synonym for `Free` (the
    /// `driver::liveness` posture). The operator deciding whether to relaunch must
    /// be told that liveness is unestablished, not given a guess.
    LivenessUnknown { written: RunStatus },
}

/// A run directory read into its frontmatter plus the honestly derived status.
#[derive(Debug, Clone, PartialEq)]
pub struct DriverRunView {
    /// The parsed block, exactly as written.
    pub frontmatter: DriverFrontmatter,
    /// What to show, given the lock ([`derive_status`]).
    pub derived: DerivedStatus,
}

/// What the lock probe established, reduced to what the rule needs.
#[cfg_attr(not(unix), allow(dead_code))]
enum Seen {
    Held,
    Free,
    Unknown,
}

/// The D-22 table, platform-independent: only the probe is unix-specific.
fn derive(fm: &DriverFrontmatter, seen: Seen) -> DerivedStatus {
    let from_probe = |written: RunStatus| match seen {
        Seen::Held => DerivedStatus::AsWritten(written),
        Seen::Free => DerivedStatus::Crashed { written },
        Seen::Unknown => DerivedStatus::LivenessUnknown { written },
    };
    match &fm.status {
        RunStatus::Running => from_probe(RunStatus::Running),
        // R1: a fresh run directory is `starting` with zero launches. A free lock
        // there is the normal state of something not yet launched.
        RunStatus::Starting if fm.launches == 0 => match seen {
            Seen::Held => DerivedStatus::AsWritten(RunStatus::Starting),
            Seen::Free | Seen::Unknown => DerivedStatus::NotLaunched,
        },
        RunStatus::Starting => from_probe(RunStatus::Starting),
        // R2: `paused` is a clean end (the launcher has exited and the lock is
        // free by design), and the terminal statuses and an unknown word are
        // shown as written. D-07 named only `running` for the crash rule.
        other => DerivedStatus::AsWritten(other.clone()),
    }
}

/// Derive the shown status from the written one and a probe of the run lock.
///
/// The rule (D-22, 27-RESEARCH Answer A, table R1-R6):
///
/// | written | lock held | lock free / absent | probe errno |
/// |---|---|---|---|
/// | `running` | as written | **crashed** | liveness unknown |
/// | `starting`, `launches > 0` | as written | **crashed** | liveness unknown |
/// | `starting`, `launches == 0` | as written | not launched | not launched |
/// | `paused`, terminal, unknown word | as written | as written | as written |
///
/// The probe MUST be `LOCK_SH | LOCK_NB` ([`crate::driver::lock::probe_at`] is):
/// two readers probing with `LOCK_EX` would each see the other's probe as a holder
/// and read a dead run as live (R6). The result is derived at read time and never
/// written anywhere (D-07).
///
/// # Phase 28 obligations this rule relies on
///
/// * **R3:** `end --reason fresh-exit` records `running`, so the launcher must hold
///   the run lock ACROSS `fresh-exit` relaunches. Releasing it between launches
///   makes every fresh-per-item run read as crashed in the gap.
/// * **R4:** acquire the lock BEFORE writing `running`, and write the terminal
///   status BEFORE releasing it. The reverse orders leave a window in which a
///   healthy run reads as crashed.
#[cfg(unix)]
pub fn derive_status(
    fm: &DriverFrontmatter,
    probe: crate::driver::lock::LockProbe,
) -> DerivedStatus {
    use crate::driver::lock::LockProbe;
    derive(
        fm,
        match probe {
            LockProbe::Held => Seen::Held,
            LockProbe::Free | LockProbe::Absent => Seen::Free,
            LockProbe::Unknown(_) => Seen::Unknown,
        },
    )
}

/// The largest `DIRECTION.md` [`read_driver_run`] will read (1 MiB, T-30-04). A run
/// directory is writable from inside an `--unsandboxed` run, so the file is display
/// data of unbounded size; a bigger one is an unreadable row, not a stalled blocking
/// worker.
pub const DIRECTION_READ_MAX_BYTES: u64 = 1 << 20;

/// Open a file in a run directory for reading (WR-02, D-09, D-25).
///
/// SKELETON (RED): opens by path with no guard yet.
pub fn open_run_file(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    std::fs::File::open(path)
}

/// Read a run directory: its `DIRECTION.md` frontmatter and the derived status.
///
/// Reads `<run_dir>/DIRECTION.md` and, on unix, probes `<run_dir>/launcher.lock`
/// with [`crate::driver::lock::probe_at`], which opens read-only and so never
/// creates the file. Nothing is written: the run directory is byte-identical
/// afterwards (D-07). On a platform without `flock` liveness cannot be probed, so
/// `running` and a launched `starting` derive [`DerivedStatus::LivenessUnknown`]
/// rather than a guess.
///
/// A delegate of [`read_driver_run_text`], which also hands back the file text.
pub fn read_driver_run(run_dir: &std::path::Path) -> Result<DriverRunView, FrontmatterError> {
    read_driver_run_text(run_dir).map(|(view, _text)| view)
}

/// [`read_driver_run`] plus the whole `DIRECTION.md` text it parsed (D-06, D-25).
///
/// The selected-run view needs the frontmatter, the derived status **and** the body
/// (live `control`, item titles, `hold`, `answer`), and they must come from one read
/// of one file: two reads could straddle a splice and pair a new frontmatter with an
/// old body. The 1 MiB cap (T-30-04) applies exactly as for [`read_driver_run`].
///
/// Blocking file I/O: call it only inside `spawn_blocking`.
pub fn read_driver_run_text(
    run_dir: &std::path::Path,
) -> Result<(DriverRunView, String), FrontmatterError> {
    let path = run_dir.join("DIRECTION.md");
    let io_err = |e: std::io::Error| FrontmatterError::Io {
        detail: format!("{}: {e}", path.display()),
    };
    let file = std::fs::File::open(&path).map_err(io_err)?;
    // T-30-04: the size is read from the open handle, so a file swapped in after
    // the check cannot dodge it, and the read itself is bounded by `take`.
    let len = file.metadata().map_err(io_err)?.len();
    if len > DIRECTION_READ_MAX_BYTES {
        return Err(FrontmatterError::Io {
            detail: format!(
                "{}: {len} bytes is larger than the {DIRECTION_READ_MAX_BYTES}-byte limit",
                path.display()
            ),
        });
    }
    let mut text = String::new();
    std::io::Read::read_to_string(
        &mut std::io::Read::take(file, DIRECTION_READ_MAX_BYTES + 1),
        &mut text,
    )
    .map_err(io_err)?;
    let frontmatter = parse_driver_frontmatter(&text)?;

    #[cfg(unix)]
    let derived = {
        use crate::driver::lock::{probe_at, run_lock_path};
        derive_status(&frontmatter, probe_at(&run_lock_path(run_dir)))
    };
    #[cfg(not(unix))]
    let derived = derive(&frontmatter, Seen::Unknown);

    Ok((
        DriverRunView {
            frontmatter,
            derived,
        },
        text,
    ))
}

/// Where a project's launcher run directories live, relative to the project root
/// (the same place `driverctl::serve::DRIVERS_REL` names; that constant is
/// Linux-only, and a listing must compile everywhere).
pub const DRIVER_RUNS_REL: &str = ".planning/meta-manager/drivers";

/// The shape of a run id the launcher will touch: one plain path component. A
/// second spelling of `driverctl::serve::RUN_ID_RE` (that module is Linux-only),
/// pinned equal to it by a test.
const PLAIN_RUN_ID_RE: &str = r"^[A-Za-z0-9][A-Za-z0-9._-]{0,79}$";

static PLAIN_RUN_ID: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(PLAIN_RUN_ID_RE).expect("static regex"));

/// Whether `name` is the shape of a run id the launcher will touch: one plain path
/// component. The check a caller makes before joining an id it did not list itself
/// (a watcher event's path part) onto the drivers directory.
pub fn is_plain_run_id(name: &str) -> bool {
    PLAIN_RUN_ID.is_match(name)
}

/// One row of [`list_driver_runs`]: the run id (its directory name) and what
/// [`read_driver_run`] made of it. An unreadable `DIRECTION.md` is that row's
/// `Err`, never a reason to drop the other rows.
pub type DriverRunEntry = (String, Result<DriverRunView, FrontmatterError>);

/// List a project's launcher runs, **newest first**, at most `limit` of them (D-30).
///
/// Reads `<project_root>/.planning/meta-manager/drivers/`, keeps the directory
/// entries whose name is a plain run id, sorts the names descending (a run id starts
/// with `yyyymmdd`, so byte order is time order) and runs [`read_driver_run`] on each
/// of the first `limit`. A missing drivers directory is an empty list.
///
/// Read-only and blocking: it must run on `spawn_blocking`, never inside a render
/// function. A symlink named like a run id is skipped (`DirEntry::file_type` does not
/// follow it), so a link cannot point the listing at a directory outside the project.
/// Cross-platform: on a platform without `flock` every `running` run derives
/// [`DerivedStatus::LivenessUnknown`] rather than a guess.
pub fn list_driver_runs(project_root: &std::path::Path, limit: usize) -> Vec<DriverRunEntry> {
    let dir = project_root.join(DRIVER_RUNS_REL);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| PLAIN_RUN_ID.is_match(name))
        .collect();
    names.sort_unstable_by(|a, b| b.cmp(a));
    names.truncate(limit);
    names
        .into_iter()
        .map(|name| {
            let view = read_driver_run(&dir.join(&name));
            (name, view)
        })
        .collect()
}

/// Shared helpers for the tests that plant a FIFO in a run directory (WR-02).
#[cfg(all(test, target_os = "linux"))]
pub(crate) mod fifo_test_support {
    use std::path::Path;

    /// Create a FIFO (no reader, no writer) at `path`.
    pub(crate) fn mkfifo(path: &Path) {
        rustix::fs::mknodat(
            rustix::fs::CWD,
            path,
            rustix::fs::FileType::Fifo,
            rustix::fs::Mode::from_raw_mode(0o600),
            0,
        )
        .expect("mkfifo");
    }

    /// Run `f` on its own thread and give it five seconds: a regression that blocks
    /// on the FIFO fails this assertion instead of hanging the suite.
    pub(crate) fn within_5s<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(f());
        });
        rx.recv_timeout(std::time::Duration::from_secs(5))
            .expect("the call blocked on a FIFO for five seconds")
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

    /// Build `<root>/.planning/meta-manager/drivers/<id>/DIRECTION.md` from the spike
    /// fixture with `run_id` and `launches` replaced.
    fn write_run(root: &std::path::Path, id: &str) {
        let dir = root.join(DRIVER_RUNS_REL).join(id);
        std::fs::create_dir_all(&dir).unwrap();
        let text = RUN1B.replace("s003-run1b", id);
        std::fs::write(dir.join("DIRECTION.md"), text).unwrap();
    }

    #[test]
    fn list_driver_runs_orders_newest_first_and_honours_the_limit() {
        let tmp = tempfile::tempdir().unwrap();
        for id in ["20261017-aaaa", "20261019-bbbb", "20261018-cccc"] {
            write_run(tmp.path(), id);
        }
        let all: Vec<String> = list_driver_runs(tmp.path(), 20)
            .into_iter()
            .map(|(id, view)| {
                assert!(view.is_ok(), "{id} should parse");
                id
            })
            .collect();
        assert_eq!(all, ["20261019-bbbb", "20261018-cccc", "20261017-aaaa"]);

        let two: Vec<String> = list_driver_runs(tmp.path(), 2)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(two, ["20261019-bbbb", "20261018-cccc"]);
        assert!(list_driver_runs(tmp.path(), 0).is_empty());
    }

    #[test]
    fn list_driver_runs_skips_names_that_are_not_plain_run_ids() {
        let tmp = tempfile::tempdir().unwrap();
        write_run(tmp.path(), "20261019-ok");
        let drivers = tmp.path().join(DRIVER_RUNS_REL);
        // A stray file, a dot-name, a name with a space and an over-long name.
        std::fs::write(drivers.join("20261020-file"), "x").unwrap();
        std::fs::create_dir_all(drivers.join(".hidden")).unwrap();
        std::fs::create_dir_all(drivers.join("has space")).unwrap();
        std::fs::create_dir_all(drivers.join("x".repeat(81))).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(tmp.path(), drivers.join("20261021-link")).unwrap();

        let ids: Vec<String> = list_driver_runs(tmp.path(), 20)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids, ["20261019-ok"]);
    }

    #[test]
    fn list_driver_runs_of_a_missing_drivers_dir_is_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(list_driver_runs(tmp.path(), 20).is_empty());
        std::fs::create_dir_all(tmp.path().join(".planning")).unwrap();
        assert!(list_driver_runs(tmp.path(), 20).is_empty());
    }

    #[test]
    fn list_driver_runs_keeps_an_unreadable_run_beside_the_others() {
        let tmp = tempfile::tempdir().unwrap();
        write_run(tmp.path(), "20261019-good");
        let bad = tmp.path().join(DRIVER_RUNS_REL).join("20261020-bad");
        std::fs::create_dir_all(&bad).unwrap(); // no DIRECTION.md at all
        let rows = list_driver_runs(tmp.path(), 20);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "20261020-bad");
        assert!(rows[0].1.is_err());
        assert_eq!(rows[1].0, "20261019-good");
        assert!(rows[1].1.is_ok());
    }

    #[test]
    fn list_driver_runs_derives_the_status_from_the_lock_not_the_written_word() {
        // The fixture says `crashed`; a `running` file with no lock must read as
        // crashed, never as the written `running`.
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(DRIVER_RUNS_REL).join("20261019-run");
        std::fs::create_dir_all(&dir).unwrap();
        let text = RUN1B
            .replace("s003-run1b", "20261019-run")
            .replace("status: crashed", "status: running");
        assert!(text.contains("status: running"));
        std::fs::write(dir.join("DIRECTION.md"), &text).unwrap();
        let rows = list_driver_runs(tmp.path(), 20);
        let view = rows[0].1.as_ref().unwrap();
        assert_eq!(view.frontmatter.status, RunStatus::Running);
        #[cfg(unix)]
        assert_eq!(
            view.derived,
            DerivedStatus::Crashed {
                written: RunStatus::Running
            }
        );
    }

    #[test]
    fn read_driver_run_refuses_an_oversized_direction() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(DRIVER_RUNS_REL).join("20261019-big");
        std::fs::create_dir_all(&dir).unwrap();
        let mut text = RUN1B.replace("s003-run1b", "20261019-big");
        // Pad the body past the cap; the frontmatter itself stays valid.
        text.push_str(&"x".repeat(DIRECTION_READ_MAX_BYTES as usize));
        std::fs::write(dir.join("DIRECTION.md"), &text).unwrap();
        match read_driver_run(&dir) {
            Err(FrontmatterError::Io { detail }) => {
                assert!(detail.contains("larger than"), "{detail}");
            }
            other => panic!("expected an Io refusal, got {other:?}"),
        }
        // Exactly at the cap is still read.
        let ok = RUN1B.replace("s003-run1b", "20261019-big");
        let pad = DIRECTION_READ_MAX_BYTES as usize - ok.len();
        std::fs::write(dir.join("DIRECTION.md"), format!("{ok}{}", "x".repeat(pad))).unwrap();
        assert!(read_driver_run(&dir).is_ok());
    }

    #[test]
    fn read_driver_run_text_returns_the_same_view_and_the_whole_file_text() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(DRIVER_RUNS_REL).join("20261019-text");
        std::fs::create_dir_all(&dir).unwrap();
        let text = format!(
            "{}\n## Direction\n\n- control: pause\n\n## Items\n",
            RUN1B.replace("s003-run1b", "20261019-text")
        );
        std::fs::write(dir.join("DIRECTION.md"), &text).unwrap();
        let plain = read_driver_run(&dir).expect("plain read");
        let (with_text, read_text) = read_driver_run_text(&dir).expect("text read");
        assert_eq!(plain, with_text);
        assert_eq!(read_text, text);
        // The cap holds for the text reader too.
        let mut big = text.clone();
        big.push_str(&"x".repeat(DIRECTION_READ_MAX_BYTES as usize));
        std::fs::write(dir.join("DIRECTION.md"), &big).unwrap();
        assert!(matches!(
            read_driver_run_text(&dir),
            Err(FrontmatterError::Io { .. })
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn open_run_file_refuses_a_fifo_without_blocking() {
        use super::fifo_test_support::{mkfifo, within_5s};
        let tmp = tempfile::tempdir().unwrap();
        let fifo = tmp.path().join("DIRECTION.md");
        mkfifo(&fifo);
        let result = within_5s(move || open_run_file(&fifo).map(|_| ()));
        let error = result.expect_err("a FIFO is not a regular file");
        assert!(error.to_string().contains("not a regular file"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn open_run_file_refuses_a_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("real.md");
        std::fs::write(&target, "x").unwrap();
        let link = tmp.path().join("DIRECTION.md");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let error = open_run_file(&link).expect_err("a link is not followed");
        assert!(error.to_string().contains("not a regular file"), "{error}");
        // A regular file opens, and a missing one stays NotFound.
        assert!(open_run_file(&target).is_ok());
        let missing = open_run_file(&tmp.path().join("nope")).unwrap_err();
        assert_eq!(missing.kind(), std::io::ErrorKind::NotFound);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn read_driver_run_text_refuses_a_fifo_direction_without_blocking() {
        use super::fifo_test_support::{mkfifo, within_5s};
        let tmp = tempfile::tempdir().unwrap();
        write_run(tmp.path(), "20261019-good");
        let bad = tmp.path().join(DRIVER_RUNS_REL).join("20261020-fifo");
        std::fs::create_dir_all(&bad).unwrap();
        mkfifo(&bad.join("DIRECTION.md"));

        let dir = bad.clone();
        let result = within_5s(move || read_driver_run_text(&dir).map(|_| ()));
        assert!(matches!(result, Err(FrontmatterError::Io { .. })), "{result:?}");

        // The run list shows it as an unreadable row beside the good one.
        let root = tmp.path().to_path_buf();
        let rows = within_5s(move || list_driver_runs(&root, 20));
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].0, "20261020-fifo");
        assert!(rows[0].1.is_err());
        assert!(rows[1].1.is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn read_driver_run_text_refuses_a_symlinked_direction() {
        let tmp = tempfile::tempdir().unwrap();
        write_run(tmp.path(), "20261019-real");
        let real = tmp
            .path()
            .join(DRIVER_RUNS_REL)
            .join("20261019-real")
            .join("DIRECTION.md");
        let linked = tmp.path().join(DRIVER_RUNS_REL).join("20261020-link");
        std::fs::create_dir_all(&linked).unwrap();
        std::os::unix::fs::symlink(&real, linked.join("DIRECTION.md")).unwrap();
        assert!(matches!(
            read_driver_run_text(&linked),
            Err(FrontmatterError::Io { .. })
        ));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_plain_run_id_pattern_is_the_one_serve_uses() {
        assert_eq!(PLAIN_RUN_ID_RE, crate::driverctl::serve::RUN_ID_RE);
    }

    #[test]
    fn the_sandbox_keys_round_trip_and_sit_right_after_launches() {
        let mut fm = parse_driver_frontmatter(RUN1B).unwrap();
        assert_eq!((&fm.sandbox, &fm.sandbox_missing), (&None, &None));
        let plain = fm.emit();
        assert!(
            !plain.contains("sandbox"),
            "emitted only when set:\n{plain}"
        );

        fm.sandbox = Some("degraded".to_owned());
        fm.sandbox_missing = Some(vec!["bwrap".to_owned(), "user-systemd".to_owned()]);
        let text = fm.emit();
        assert!(
            text.contains(
                "launches: 1\nsandbox: \"degraded\"\n\
                 sandbox_missing: [\"bwrap\", \"user-systemd\"]\nconnected:"
            ),
            "{text}"
        );
        assert_eq!(parse_driver_frontmatter(&text).unwrap(), fm);

        fm.sandbox = Some("bwrap-scope".to_owned());
        fm.sandbox_missing = None;
        let text = fm.emit();
        assert!(
            text.contains("launches: 1\nsandbox: \"bwrap-scope\"\nconnected:"),
            "{text}"
        );
        assert!(!text.contains("sandbox_missing"), "{text}");
        assert_eq!(parse_driver_frontmatter(&text).unwrap(), fm);
    }

    #[test]
    fn the_prompt_version_key_round_trips_right_after_the_sandbox_keys() {
        let mut fm = parse_driver_frontmatter(RUN1B).unwrap();
        assert_eq!(fm.prompt_version, None);
        let plain = fm.emit();
        assert!(
            !plain.contains("prompt_version"),
            "emitted only when set:\n{plain}"
        );

        fm.prompt_version = Some("driver-prompt/1.0.0".to_owned());
        let text = fm.emit();
        assert!(
            text.contains("launches: 1\nprompt_version: \"driver-prompt/1.0.0\"\nconnected:"),
            "{text}"
        );
        assert_eq!(parse_driver_frontmatter(&text).unwrap(), fm);

        fm.sandbox = Some("bwrap-scope".to_owned());
        fm.sandbox_missing = Some(vec!["bwrap".to_owned()]);
        fm.prompt_version = Some("custom:0a1b2c3d".to_owned());
        let text = fm.emit();
        assert!(
            text.contains(
                "sandbox_missing: [\"bwrap\"]\nprompt_version: \"custom:0a1b2c3d\"\nconnected:"
            ),
            "{text}"
        );
        assert_eq!(parse_driver_frontmatter(&text).unwrap(), fm);
    }

    #[test]
    fn a_hostile_sandbox_value_cannot_span_lines() {
        let mut fm = parse_driver_frontmatter(RUN1B).unwrap();
        fm.sandbox = Some("a\nstatus: done".to_owned());
        fm.sandbox_missing = Some(vec!["x\"]\nlaunches: 9".to_owned()]);
        let text = fm.emit();
        assert_eq!(parse_driver_frontmatter(&text).unwrap(), fm);
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
