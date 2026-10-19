//! `RunStore`: the one writer of a driver run's `state.json`, `LOG.md` and
//! `DIRECTION.md` frontmatter (DCTL-01, D-03).
//!
//! Ported from spike 002's `RunStore` (`driverctl.py`) and the launch bookkeeping
//! of spike 003's launcher. The decisions that shape it:
//!
//! * **The run lock is in the type (D-03, D-20).** [`RunStore::open`] takes the
//!   [`RunLock`] guard by value, so a store without the lock cannot be built, and
//!   the lock is released only when the store is dropped, after its last save.
//!   `flock` lasts as long as the descriptor, which is why the guard is a field
//!   and not a local.
//! * **Validate fully, then apply (D-04).** A request is validated by
//!   [`super::protocol`] before anything is touched. The event is then applied to
//!   a *clone* of the state; the clone is committed only after the LOG line and
//!   `state.json` are written. A rejection, or an I/O failure on the way, leaves
//!   the in-memory state as it was and writes nothing further.
//! * **Counters are not the driver's to set (D-10, D-26).** `caps.dispatches.used`
//!   counts hook-observed driver-level dispatches the Gate allowed, never the
//!   driver's own `dispatched` lines; gap rounds come only from
//!   [`RunStore::observe_verification`]; `crashed` and `stalled` are written only
//!   by [`RunStore::set_terminal`]. None of these has a request shape.
//! * **The frontmatter is a derived view (D-19).** [`RunStore::frontmatter`] builds
//!   a [`DriverFrontmatter`] from `state.json` and `DIRECTION.md`, and writes it
//!   with [`DriverFrontmatter::emit`] through [`splice_frontmatter`]. There is no
//!   second sidecar file and the model never writes the block.
//! * **Forward compatibility (Pitfall 7).** Every field of [`RunState`] defaults and
//!   [`RunState`] / [`ItemRecord`] carry a flattened `extra` map, so the keys the
//!   Phase 28 launcher writes (`auth`, `cost`, `active_seconds`, ...) survive a
//!   rewrite unchanged.
//! * **Heartbeats are coalesced (D-11).** A heartbeat-only hook rewrites
//!   `state.json` every time but regenerates `DIRECTION.md` at most once per
//!   [`HEARTBEAT_REGEN_MIN_S`].
//! * **Crash detection (D-17).** [`RunStore::begin_launch`] notices a previous
//!   launch that never logged a driver `RUN end`.
//!
//! Synchronous throughout: it takes `flock` and does file I/O, so never call it
//! from an `async fn` (see the module root).

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tempfile::NamedTempFile;

use super::counters::{worst_phase, GapRoundsState};
use super::log;
use super::protocol::{
    self, HookPayload, Invalid, LogArgs, Request, Response, ValidatedEvent,
};
use crate::driver::direction::{
    parse_direction, split_frontmatter, splice_frontmatter, DirectionInfo, DIRECTION_FILE,
};
use crate::driver::lock::{LockHolder, RunLock};
use crate::state_reader::driver_frontmatter::{
    Cap, Caps, Connection, DispatchAudit, DriverFrontmatter, GapCap, ItemCounts, ItemState,
    ItemStateEntry, ItemStates, LastEvent, RunStatus, WallClockCap, DRIVER_FRONTMATTER_VERSION,
    HEARTBEAT_STALE_AFTER_S,
};

/// The state file inside a run directory.
pub const STATE_FILE: &str = "state.json";

/// The event log inside a run directory.
pub const LOG_FILE: &str = "LOG.md";

/// A heartbeat-only event regenerates `DIRECTION.md` at most this often (D-11).
pub const HEARTBEAT_REGEN_MIN_S: i64 = 120;

/// How long the `git rev-parse` behind a `dispatched` event may take (T-27-18).
pub const DISPATCH_GIT_BUDGET: Duration = Duration::from_secs(2);

/// The evidence recorded when a launch finds the previous one never ended.
const CRASH_EVIDENCE: &str = "status=running and no RUN end; lock was free";

// ---------------------------------------------------------------- state.json

fn schema_default() -> u32 {
    1
}

/// The item the driver is working on, as last dispatched.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Current {
    pub item: Option<String>,
    pub phase: Option<String>,
    pub step: Option<String>,
}

/// One item's record in `state.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ItemRecord {
    pub state: ItemState,
    pub outcome: Option<String>,
    pub dispatches: u32,
    pub first_dispatch_at: Option<String>,
    /// The `FINDINGS: N` counts the driver's reports carried, in order.
    pub findings: Vec<i64>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for ItemRecord {
    fn default() -> Self {
        ItemRecord {
            state: ItemState::Pending,
            outcome: None,
            dispatches: 0,
            first_dispatch_at: None,
            findings: Vec::new(),
            extra: Map::new(),
        }
    }
}

/// The `caps` block of `state.json`: the counters plus the gap-round window state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StateCaps {
    pub dispatches: Cap,
    pub wall_clock_min: WallClockCap,
    pub gap_rounds: GapRoundsState,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// `state.json`: the run's durable state. See the module header for the
/// forward-compatibility contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RunState {
    #[serde(default = "schema_default")]
    pub schema: u32,
    pub run_id: String,
    pub project_root: String,
    pub status: RunStatus,
    pub status_reason: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub launches: u32,
    pub current: Current,
    pub items: BTreeMap<String, ItemRecord>,
    pub connections: Vec<Connection>,
    pub caps: StateCaps,
    /// Hook-observed driver-level dispatches the Gate allowed (D-10).
    pub hook_dispatches: u32,
    pub denied_dispatches: u32,
    pub last_event: Option<LastEvent>,
    /// Whether the driver itself logged `RUN end` in the current launch.
    pub ended_by_driver: bool,
    /// Keys this build does not know (the launcher's `auth`, `cost`, ...).
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for RunState {
    fn default() -> Self {
        RunState {
            schema: 1,
            run_id: String::new(),
            project_root: String::new(),
            status: RunStatus::Starting,
            status_reason: None,
            created_at: String::new(),
            updated_at: String::new(),
            launches: 0,
            current: Current::default(),
            items: BTreeMap::new(),
            connections: Vec::new(),
            caps: StateCaps::default(),
            hook_dispatches: 0,
            denied_dispatches: 0,
            last_event: None,
            ended_by_driver: false,
            extra: Map::new(),
        }
    }
}

impl RunState {
    /// A fresh run: status `starting`, no launches, empty maps (the spike's init).
    pub fn new(run_id: &str, project_root: &str, now: &str) -> Self {
        RunState {
            run_id: run_id.to_owned(),
            project_root: project_root.to_owned(),
            created_at: now.to_owned(),
            updated_at: now.to_owned(),
            ..RunState::default()
        }
    }
}

// ---------------------------------------------------------------- launcher inputs

/// The caps a launcher applies at a launch (`None` = no limit).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CapLimits {
    pub dispatches: Option<u32>,
    pub wall_clock_min: Option<f64>,
    pub gap_rounds: Option<u32>,
}

/// The driver process a launch starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriverSpec {
    pub model: Option<String>,
    pub mode: String,
}

/// What [`RunStore::begin_launch`] records about this launch.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LaunchSpec {
    /// How the launcher runs the driver (`standalone`, `bwrap`, ...); the launcher
    /// connection's `mode` and the `mode=` of the `RUN | launch` line.
    pub launcher_mode: String,
    pub limits: CapLimits,
    pub driver: Option<DriverSpec>,
    pub scope_unit: Option<String>,
}

// ---------------------------------------------------------------- hooks

/// What the Gate decided about a driver-level dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDecision {
    Allow,
    /// Refuse the dispatch; the reason goes back to the hook and into the LOG.
    Deny(String),
}

/// The Phase 28 hook point (D-26): decides a driver-level dispatch from the hook
/// payload and the run state, including `caps.gap_rounds.per_phase`.
pub type Gate = Box<dyn FnMut(&HookPayload, &RunState) -> GateDecision + Send>;

/// The store's time source, replaceable for tests.
pub type Clock = Box<dyn Fn() -> DateTime<Utc> + Send>;

/// A terminal status only the launcher may record (D-06, D-17).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalStatus {
    Crashed,
    Stalled,
    Stopped,
}

/// What [`RunStore::set_terminal`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalOutcome {
    Applied,
    /// A driver `RUN end` is on record; the status stays as the driver set it.
    KeptDriverEnd,
}

/// Why the store could not open or write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreError {
    Io { detail: String },
    /// `state.json` exists but does not parse. The store refuses to open rather
    /// than overwrite what it cannot read.
    CorruptState { detail: String },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Io { detail } => write!(f, "run store I/O failed: {detail}"),
            StoreError::CorruptState { detail } => {
                write!(f, "state.json is corrupt, refusing to overwrite it: {detail}")
            }
        }
    }
}

impl std::error::Error for StoreError {}

fn io_err(context: &str) -> impl Fn(std::io::Error) -> StoreError + '_ {
    move |e| StoreError::Io {
        detail: format!("{context}: {e}"),
    }
}

// ---------------------------------------------------------------- the store

/// One LOG event, before it is formatted.
struct LogEntry {
    item: String,
    event: &'static str,
    kv: Vec<(&'static str, Option<String>)>,
    detail: Vec<String>,
}

impl LogEntry {
    fn run(event: &'static str, kv: Vec<(&'static str, Option<String>)>) -> Self {
        LogEntry {
            item: "RUN".to_owned(),
            event,
            kv,
            detail: Vec::new(),
        }
    }
}

/// A launcher-supplied header value made safe for one LOG header: one line, no
/// `|`, at most 200 characters. Launcher text is trusted, so it is cleaned rather
/// than refused.
fn header_safe(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| if matches!(c, '\n' | '\r' | '|') || c.is_control() { ' ' } else { c })
        .take(200)
        .collect();
    cleaned.trim().to_owned()
}

fn host_name() -> Option<String> {
    fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

/// The single writer of one run's state. See the module header.
pub struct RunStore {
    run_dir: PathBuf,
    project: PathBuf,
    state: RunState,
    /// Held for the store's whole life: dropping it releases the run lock.
    lock: RunLock,
    clock: Clock,
    last_regen: Option<DateTime<Utc>>,
}

impl RunStore {
    /// Open the store for `run_dir`, holding `lock` until it is dropped.
    pub fn open(run_dir: &Path, project: &Path, lock: RunLock) -> Result<Self, StoreError> {
        Self::open_with_clock(run_dir, project, lock, Box::new(Utc::now))
    }

    /// [`open`](Self::open) with an injected clock.
    ///
    /// Loads `state.json` when it exists; a file that does not parse is
    /// [`StoreError::CorruptState`] and is never overwritten.
    pub fn open_with_clock(
        run_dir: &Path,
        project: &Path,
        lock: RunLock,
        clock: Clock,
    ) -> Result<Self, StoreError> {
        let state_path = run_dir.join(STATE_FILE);
        let state = match fs::read(&state_path) {
            Ok(bytes) => serde_json::from_slice::<RunState>(&bytes).map_err(|e| {
                StoreError::CorruptState {
                    detail: format!("{}: {e}", state_path.display()),
                }
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let run_id = run_dir
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                RunState::new(
                    &run_id,
                    &project.to_string_lossy(),
                    &log::utc_string(clock()),
                )
            }
            Err(e) => return Err(io_err("read state.json")(e)),
        };
        Ok(RunStore {
            run_dir: run_dir.to_owned(),
            project: project.to_owned(),
            state,
            lock,
            clock,
            last_regen: None,
        })
    }

    /// The run state as the store holds it.
    pub fn state(&self) -> &RunState {
        &self.state
    }

    /// The record of the run lock this store holds.
    pub fn holder(&self) -> &LockHolder {
        self.lock.holder()
    }

    fn now_string(&self) -> String {
        log::utc_string((self.clock)())
    }

    fn log_path(&self) -> PathBuf {
        self.run_dir.join(LOG_FILE)
    }

    /// `DIRECTION.md`'s control word and items; an absent file has neither.
    fn direction_info(&self) -> DirectionInfo {
        let Ok(bytes) = fs::read(self.run_dir.join(DIRECTION_FILE)) else {
            return DirectionInfo::default();
        };
        let text = String::from_utf8_lossy(&bytes);
        let (_, body) = split_frontmatter(&text);
        parse_direction(body)
    }

    // -- persistence

    /// `state.json` via a temp file in the run dir: write, mode 0644, fsync, rename.
    fn write_state(&self, state: &RunState) -> Result<(), StoreError> {
        let mut json = serde_json::to_string_pretty(state).map_err(|e| StoreError::Io {
            detail: format!("serialize state.json: {e}"),
        })?;
        json.push('\n');
        let mut tmp = NamedTempFile::new_in(&self.run_dir).map_err(io_err("create state temp"))?;
        tmp.write_all(json.as_bytes())
            .map_err(io_err("write state temp"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tmp.as_file()
                .set_permissions(fs::Permissions::from_mode(0o644))
                .map_err(io_err("chmod state temp"))?;
        }
        tmp.as_file().sync_all().map_err(io_err("fsync state temp"))?;
        tmp.persist(self.run_dir.join(STATE_FILE))
            .map_err(|e| io_err("persist state.json")(e.error))?;
        Ok(())
    }

    /// Append `entries` to LOG.md, commit `next` as the new state and regenerate
    /// the frontmatter. On an I/O failure before the commit the in-memory state is
    /// unchanged.
    fn commit(&mut self, mut next: RunState, entries: &[LogEntry]) -> Result<(), StoreError> {
        let now = self.now_string();
        if let Some(last) = entries.last() {
            let mut data = String::new();
            for e in entries {
                data.push_str(&log::format_line(&now, &e.item, e.event, &e.kv, &e.detail));
            }
            log::append(&self.log_path(), &data).map_err(io_err("append LOG.md"))?;
            next.last_event = Some(LastEvent {
                at: Some(now.clone()),
                item: Some(last.item.clone()),
                event: Some(last.event.to_owned()),
                summary: Some(log::summary(&last.kv)),
            });
        }
        next.updated_at = now;
        self.write_state(&next)?;
        self.state = next;
        self.regen();
        Ok(())
    }

    // -- frontmatter

    /// The derived frontmatter view (D-19): item counts and states in DIRECTION
    /// order, caps with `gap_rounds` from the worst phase, the dispatch audit.
    pub fn frontmatter(&self) -> DriverFrontmatter {
        let direction = self.direction_info();
        let mut ids: Vec<String> = Vec::new();
        let source: Vec<&String> = if direction.items.is_empty() {
            self.state.items.keys().collect()
        } else {
            direction.items.iter().map(|i| &i.id).collect()
        };
        for id in source {
            // A duplicated `### I-01` header must not become a duplicate YAML key.
            if !ids.contains(id) {
                ids.push(id.clone());
            }
        }

        let mut counts = ItemCounts {
            total: u32::try_from(ids.len()).unwrap_or(u32::MAX),
            ..ItemCounts::default()
        };
        let mut entries = Vec::with_capacity(ids.len());
        for id in &ids {
            let record = self.state.items.get(id);
            let state = record.map_or(ItemState::Pending, |r| r.state);
            match state {
                ItemState::Pending => counts.pending += 1,
                ItemState::Running => counts.running += 1,
                ItemState::Done => counts.done += 1,
                ItemState::Parked => counts.parked += 1,
                ItemState::Failed => counts.failed += 1,
            }
            entries.push((
                id.clone(),
                ItemStateEntry {
                    state,
                    dispatches: record.map_or(0, |r| r.dispatches),
                    outcome: record.and_then(|r| r.outcome.clone()),
                },
            ));
        }

        let (phase, used) = worst_phase(&self.state.caps.gap_rounds);
        let logged = self
            .state
            .items
            .values()
            .fold(0u32, |acc, r| acc.saturating_add(r.dispatches));

        DriverFrontmatter {
            generated_by: format!("driverctl {}", env!("CARGO_PKG_VERSION")),
            driver_frontmatter_version: DRIVER_FRONTMATTER_VERSION,
            run_id: self.state.run_id.clone(),
            status: self.state.status.clone(),
            status_reason: self.state.status_reason.clone(),
            control: direction.control,
            current_item: self.state.current.item.clone(),
            current_phase: self.state.current.phase.clone(),
            current_step: self.state.current.step.clone(),
            items: counts,
            item_states: ItemStates(entries),
            caps: Caps {
                dispatches: self.state.caps.dispatches.clone(),
                wall_clock_min: self.state.caps.wall_clock_min.clone(),
                gap_rounds: GapCap {
                    used,
                    limit: self.state.caps.gap_rounds.limit,
                    phase,
                },
            },
            dispatch_audit: DispatchAudit {
                logged,
                observed: self.state.hook_dispatches,
                denied: self.state.denied_dispatches,
            },
            launches: self.state.launches,
            connected: self.state.connections.clone(),
            heartbeat_stale_after_s: HEARTBEAT_STALE_AFTER_S,
            last_event: self.state.last_event.clone().unwrap_or_default(),
            updated_at: self.state.updated_at.clone(),
        }
    }

    /// The text of the frontmatter block, as `show` replies with it.
    pub fn frontmatter_text(&self) -> String {
        self.frontmatter().emit()
    }

    /// Splice the current frontmatter into `DIRECTION.md` (a run dir without one
    /// has nothing to regenerate).
    fn regen(&mut self) {
        if !self.run_dir.join(DIRECTION_FILE).exists() {
            return;
        }
        let text = self.frontmatter_text();
        // A lost race or an insane body is not this request's failure: the event
        // is already durable in LOG.md and state.json.
        let _report = splice_frontmatter(&self.run_dir, &text);
        self.last_regen = Some((self.clock)());
    }

    // -- launch

    /// Record the start of one launch (spike 003 `launcher.run`, D-17).
    ///
    /// Order: a `crash-detected` line when the previous launch left the run
    /// `running` or `starting` without a driver `RUN end`; connected connections
    /// become `lost`; the last four non-launcher connections are kept; `launches`
    /// goes up, the status is `running`, the caps limits are applied; the launcher
    /// (and, when given, the driver) connection is recorded; `RUN | launch`.
    pub fn begin_launch(&mut self, spec: LaunchSpec) -> Result<(), StoreError> {
        let now = self.now_string();
        let mut next = self.state.clone();
        let mut entries = Vec::new();

        if matches!(next.status, RunStatus::Running | RunStatus::Starting)
            && next.launches > 0
            && !next.ended_by_driver
        {
            entries.push(LogEntry::run(
                "crash-detected",
                vec![
                    ("previous_launch", Some(next.launches.to_string())),
                    ("evidence", Some(CRASH_EVIDENCE.to_owned())),
                ],
            ));
        }

        for c in &mut next.connections {
            if c.state == "connected" {
                c.state = "lost".to_owned();
            }
        }
        next.connections.retain(|c| c.role != "launcher");
        if next.connections.len() > 4 {
            let drop = next.connections.len() - 4;
            next.connections.drain(..drop);
        }

        next.launches = next.launches.saturating_add(1);
        next.ended_by_driver = false;
        next.status = RunStatus::Running;
        next.status_reason = None;
        next.caps.dispatches.limit = spec.limits.dispatches;
        next.caps.wall_clock_min.limit = spec.limits.wall_clock_min;
        next.caps.gap_rounds.limit = spec.limits.gap_rounds;

        let host = host_name();
        let launcher_mode = header_safe(&spec.launcher_mode);
        next.connections.push(Connection {
            role: "launcher".to_owned(),
            state: "connected".to_owned(),
            session_id: None,
            model: None,
            mode: Some(launcher_mode.clone()),
            pid: Some(std::process::id()),
            scope_unit: spec.scope_unit.clone(),
            host: host.clone(),
            started_at: Some(now.clone()),
            last_heartbeat: Some(now.clone()),
        });
        let mut model = None;
        if let Some(driver) = &spec.driver {
            model = driver.model.as_deref().map(header_safe);
            next.connections.push(Connection {
                role: "driver".to_owned(),
                state: "connected".to_owned(),
                session_id: None,
                model: driver.model.clone(),
                mode: Some(driver.mode.clone()),
                pid: None,
                scope_unit: spec.scope_unit.clone(),
                host,
                started_at: Some(now.clone()),
                last_heartbeat: None,
            });
        }
        entries.push(LogEntry::run(
            "launch",
            vec![
                ("n", Some(next.launches.to_string())),
                ("mode", Some(launcher_mode)),
                ("model", model),
            ],
        ));
        self.commit(next, &entries)
    }

    // -- requests

    /// Handle one parsed request. Never panics and never returns an error value:
    /// every failure is a [`Response`].
    pub fn handle(&mut self, request: Request) -> Response {
        match request {
            Request::Show => Response::frontmatter(self.frontmatter_text()),
            Request::Log { event, args } => self.handle_log(&event, &args),
            Request::Hook { .. } => Response::rejected("hook is not supported yet"),
        }
    }

    /// Parse and handle one request line.
    pub fn handle_json(&mut self, bytes: &[u8]) -> Response {
        match protocol::parse_request(bytes) {
            Ok(request) => self.handle(request),
            Err(Invalid(message)) => Response::rejected(message),
        }
    }

    fn handle_log(&mut self, event: &str, args: &LogArgs) -> Response {
        let direction = self.direction_info();
        let ids: Vec<String> = direction.items.iter().map(|i| i.id.clone()).collect();
        let validated = match protocol::validate_log(event, args, &ids) {
            Ok(v) => v,
            Err(Invalid(message)) => return Response::rejected(message),
        };

        let (budget, base) = match &validated {
            ValidatedEvent::Dispatched { item, .. } => (
                direction
                    .items
                    .iter()
                    .find(|i| &i.id == item)
                    .and_then(|i| i.budget.get("dispatches").copied()),
                self.git_base(),
            ),
            _ => (None, None),
        };

        let now = self.now_string();
        let mut next = self.state.clone();
        let entry = apply_event(&mut next, validated, &now, budget, base);
        match self.commit(next, &[entry]) {
            Ok(()) => Response::ok(),
            Err(e) => Response::rejected(format!("internal: {e}")),
        }
    }

    /// `git rev-parse --short HEAD` in the project, or `None` (not a repo, a
    /// failure, or output that is not a hex hash). The value reaches a LOG header,
    /// so it is held to hex digits (T-27-15, T-27-18).
    fn git_base(&self) -> Option<String> {
        let raw = crate::state_reader::git_ops::git_read_raw_within(
            &self.project,
            &["rev-parse", "--short", "HEAD"],
            DISPATCH_GIT_BUDGET,
        )?;
        let hash = raw.trim();
        (!hash.is_empty() && hash.len() <= 40 && hash.chars().all(|c| c.is_ascii_hexdigit()))
            .then(|| hash.to_owned())
    }
}

/// Apply one validated event to `next` and describe its LOG line.
fn apply_event(
    next: &mut RunState,
    event: ValidatedEvent,
    now: &str,
    budget: Option<i64>,
    base: Option<String>,
) -> LogEntry {
    match event {
        ValidatedEvent::Start {
            mode,
            loop_mode,
            session,
        } => LogEntry::run(
            "start",
            vec![
                ("mode", Some(mode)),
                ("loop_mode", loop_mode),
                ("session", session),
            ],
        ),
        ValidatedEvent::Reconcile { found } => {
            LogEntry::run("reconcile", vec![("found", Some(found.to_string()))])
        }
        ValidatedEvent::Dispatched {
            item,
            step,
            round,
            phase,
        } => {
            let record = next.items.entry(item.clone()).or_default();
            record.dispatches = record.dispatches.saturating_add(1);
            if record.first_dispatch_at.is_none() {
                record.first_dispatch_at = Some(now.to_owned());
            }
            if record.state == ItemState::Pending {
                record.state = ItemState::Running;
            }
            let dispatch = format!(
                "{}/{}",
                record.dispatches,
                budget.map_or_else(|| "None".to_owned(), |b| b.to_string())
            );
            next.current = Current {
                item: Some(item.clone()),
                phase: phase.clone(),
                step: Some(step.clone()),
            };
            LogEntry {
                item,
                event: "dispatched",
                kv: vec![
                    ("step", Some(step)),
                    ("base", base),
                    ("round", Some(round.to_string())),
                    ("dispatch", Some(dispatch)),
                    ("phase", phase),
                ],
                detail: Vec::new(),
            }
        }
        ValidatedEvent::Report {
            item,
            status,
            body,
            truncated,
            findings,
        } => {
            if let Some(n) = findings {
                next.items.entry(item.clone()).or_default().findings.push(n);
            }
            LogEntry {
                item,
                event: "report",
                kv: vec![
                    ("status", Some(status)),
                    ("truncated", truncated.then(|| "yes".to_owned())),
                ],
                detail: body,
            }
        }
        ValidatedEvent::Verified {
            item,
            done_when,
            evidence,
        } => LogEntry {
            item,
            event: "verified",
            kv: vec![
                ("done-when", Some(done_when)),
                ("evidence", Some(evidence)),
            ],
            detail: Vec::new(),
        },
        ValidatedEvent::Status {
            item,
            state,
            rule,
            outcome,
        } => {
            let record = next.items.entry(item.clone()).or_default();
            record.state = state;
            record.outcome = outcome.clone();
            LogEntry {
                item,
                event: "status",
                kv: vec![
                    ("state", Some(state.as_str().to_owned())),
                    ("rule", rule),
                    ("outcome", outcome),
                ],
                detail: Vec::new(),
            }
        }
        ValidatedEvent::NeedsHuman { item, question } => LogEntry {
            item,
            event: "needs-human",
            kv: vec![("question", Some(question))],
            detail: Vec::new(),
        },
        ValidatedEvent::Recovered {
            item,
            action,
            reason,
        } => LogEntry {
            item,
            event: "recovered",
            kv: vec![("action", Some(action)), ("reason", Some(reason))],
            detail: Vec::new(),
        },
        ValidatedEvent::End { reason, status } => {
            next.status = status;
            next.status_reason = Some(format!("driver:{reason}"));
            next.ended_by_driver = true;
            LogEntry::run("end", vec![("reason", Some(reason))])
        }
    }
}
