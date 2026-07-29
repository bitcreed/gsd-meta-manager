//! Drive a `claude` CLI process over the duplex `stream-json` protocol.
//!
//! This module root carries the whole domain surface every later plan builds
//! against, so nothing downstream has to reopen it (D-21). The four submodules
//! divide cleanly: `claude` owns the process transport, `gate` owns the
//! `system/init` capability check, `outcome` owns run-outcome derivation, and
//! `stream_json` owns the wire model.
//!
//! Two facts govern every type below, both measured directly against CLI
//! 2.1.220 rather than inferred:
//!
//! 1. **`type:"result"` is a TURN boundary, not a run terminator (D-29).** One
//!    process emits one `result` per turn. Collecting turns into a vector and
//!    ending the run only on stdin EOF → process exit is not defensive coding;
//!    an executor that returns on the first `result` truncates every steered
//!    run while reporting success.
//! 2. **The first user message is withheld until the gate passes (D-02, D-06).**
//!    Because the prompt travels over stdin rather than as a positional
//!    argument, no turn ever begins before validation, which is what makes a
//!    capability refusal cost zero tokens and zero quota.

// `claude` is Unix-only by construction: `process-wrap`'s `ProcessGroup` and
// its `signal()` method are both `#[cfg(unix)]`, and process-group teardown is
// the entire reason the dependency is here (D-03, D-14). Driving is a Unix
// capability in v2.0; the wire model, the gate and outcome derivation stay
// portable and testable everywhere.
#[cfg(unix)]
pub mod claude;
pub mod gate;
pub mod outcome;
pub mod stream_json;

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::ExitStatus;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, Mutex};
use uuid::Uuid;

use crate::error::{SendError, SpawnError};
use crate::executor::stream_json::{
    ControlRequest, ControlResponse, ResultMessage, StreamMessage, UserMessage,
};

/// A boxed, `Send` future — how the [`Executor`] trait stays object-safe.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Correlation map from a `control_request` id to the waiter for its response.
pub type PendingControl = Arc<Mutex<HashMap<String, oneshot::Sender<ControlResponse>>>>;

/// Drives an agent process and reports what it did.
///
/// **On object safety and `async fn`.** The async methods return boxed futures
/// by hand rather than depending on `async-trait`. There is exactly one
/// implementor in this phase, and Phase 22 adds an [`ExecutionTarget`] *variant*
/// — not a second implementor — so the dependency would buy nothing but a
/// Cargo line. If a genuinely second backend ever lands, revisit.
///
/// **On backend honesty.** `send` and `interrupt` are a *Claude capability
/// tier*, not backend parity. A hypothetical Codex or aider backend satisfies
/// `start`, `cancel` and `is_running` and nothing else — `interrupt` here rides
/// on `interrupt_cancel_queued_v1`, a Claude-specific protocol capability with
/// a Claude-specific `still_queued` response shape. Reporting that honestly is
/// exactly what [`Executor::capabilities`] exists for (D-22).
pub trait Executor {
    /// Spawn a run and withhold `command` until the first `system/init` passes
    /// the capability gate. Returns only once the gate has been decided, so a
    /// refusal is a start-time error rather than a mid-run surprise.
    fn start<'a>(
        &'a self,
        project: &'a DrivableProject,
        command: String,
        options: ExecutionOptions,
    ) -> BoxFuture<'a, Result<ExecutionHandle, SpawnError>>;

    /// Write one user message to the running process's stdin.
    ///
    /// The CLI queues a mid-turn message and executes it as its own turn, so
    /// there is deliberately **no** driver-side turn-boundary flush buffer —
    /// re-implementing the buffering the CLI already does would only make the
    /// `still_queued` accounting harder to reason about (D-31).
    fn send<'a>(
        &'a self,
        handle: &'a mut ExecutionHandle,
        message: UserMessage,
    ) -> BoxFuture<'a, Result<(), SendError>>;

    /// Write a `control_request` interrupt and await its correlated response.
    fn interrupt<'a>(
        &'a self,
        handle: &'a mut ExecutionHandle,
    ) -> BoxFuture<'a, Result<InterruptAck, SendError>>;

    /// Tear the process group down and return the run's outcome.
    fn cancel<'a>(&'a self, handle: &'a mut ExecutionHandle) -> BoxFuture<'a, RunOutcome>;

    /// Whether the run's process is still alive.
    fn is_running(&self, handle: &ExecutionHandle) -> bool;

    /// The capabilities the driven CLI advertised at its first `system/init`.
    fn capabilities<'a>(&self, handle: &'a ExecutionHandle) -> &'a [String];
}

/// Proof that the user designated a project as drivable.
///
/// This is the D-23 capability token, and it is a *type* on purpose: the
/// compiler, not a code review, is what enforces the single spawn seam.
/// [`Executor::start`] takes this and never a bare path and never a `bool`,
/// so a call site cannot spawn an agent against a directory the user never
/// opted in.
///
/// The fields are private, so only this module can construct one. Phase 17
/// supplies the validated `driver_opt_in` config record as the **only**
/// production constructor; [`DrivableProject::for_testing`] is the explicitly
/// named test/dev escape hatch until then. Adding the parameter now costs one
/// signature; retrofitting it later means auditing every call site added
/// between now and then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrivableProject {
    alias: String,
    root: PathBuf,
}

impl DrivableProject {
    /// Construct a token without a validated opt-in record.
    ///
    /// **Test and development only.** Phase 17 adds the production constructor
    /// that reads the user's `driver_opt_in` record; until it exists this is
    /// the only way to build one, and the name is deliberately loud.
    pub fn for_testing(alias: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        Self {
            alias: alias.into(),
            root: root.into(),
        }
    }

    /// The project's registered alias.
    pub fn alias(&self) -> &str {
        &self.alias
    }

    /// The filesystem root the agent runs in.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Where a run executes.
///
/// One variant today (D-21). Phase 22 adds `Container`, which is a variant
/// addition rather than a signature change — a one-line cost now against a
/// touch-every-call-site cost later. Derives match `DetailSubView` in
/// `src/app.rs`, the repository's precedent for a plain grow-over-time enum.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ExecutionTarget {
    /// Run directly on this machine.
    #[default]
    Host,
}

/// Which `settings.json` tiers the driven CLI is allowed to load.
///
/// `--setting-sources project` (omitting `user`) is the mitigation for the
/// reproduced hook hang: the user's global `PreToolUse` hooks are registered
/// with no timeout, run synchronously on the agent's critical path, and have
/// no TTY under `-p` (D-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingSources {
    /// Project tier only. The default, and the hang mitigation.
    #[default]
    ProjectOnly,
    /// Re-enable the user tier. The config-facing toggle D-01 asks for; it
    /// re-exposes the hook hang and defaults off for that reason.
    UserAndProject,
}

impl SettingSources {
    /// The value passed to `--setting-sources`.
    pub fn as_flag_value(self) -> &'static str {
        match self {
            Self::ProjectOnly => "project",
            Self::UserAndProject => "user,project",
        }
    }
}

/// The permission posture of a driver-launched session.
///
/// There is deliberately **no** bypass variant. `dontAsk` denies anything
/// outside `permissions.allow` and denies `AskUserQuestion` even when an allow
/// rule matches, which converts "unattended run hangs on an interactive gate"
/// from a hang into a fast, classifiable failure. `bypassPermissions` and
/// `--dangerously-skip-permissions` are never emitted on the host, and the
/// absence of a variant is what makes that a compile-time property rather than
/// a review comment (D-15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
    /// Deny rather than prompt. The only mode a driver-launched run uses.
    #[default]
    DontAsk,
}

impl PermissionMode {
    /// The value passed to `--permission-mode`.
    pub fn as_flag_value(self) -> &'static str {
        match self {
            Self::DontAsk => "dontAsk",
        }
    }
}

/// Everything that shapes one run's argv and supervision.
#[derive(Debug, Clone)]
pub struct ExecutionOptions {
    /// Generated **before** spawn and passed as `--session-id`, so a crash
    /// before the first stdout byte is still resumable. A v4 UUID from a
    /// CSPRNG-backed generator, never a counter or a timestamp.
    pub session_id: Uuid,
    /// Where the run executes.
    pub target: ExecutionTarget,
    /// Plumbed to `--max-budget-usd` and **nothing more** (D-16). It is a
    /// post-turn circuit breaker — the turn it fires on runs to completion
    /// first — so it bounds the *next* turn and can never cap the current one.
    /// This phase builds no cost enforcement on it, and neither does it treat
    /// the turn-count flag as a load-bearing bound: that counter resets when
    /// `stream-json` queues messages.
    pub budget_usd: Option<f64>,
    /// `--model`.
    pub model: Option<String>,
    /// `--resume <id>`, always with an explicit value — a bare `-r` opens an
    /// interactive picker, which under `-p` with no TTY is a hang.
    pub resume_session: Option<String>,
    /// `--permission-mode`.
    pub permission_mode: PermissionMode,
    /// `--setting-sources`.
    pub setting_sources: SettingSources,
    /// Total wall-clock cap for the run. Enforced by the supervisor (D-13).
    pub wall_clock_cap: Duration,
    /// Cap on time since the **last observed stream line** (D-13). This, not
    /// elapsed time, is the correct stuck-detector: the spike measured a
    /// legitimate 774-second run whose stream advanced continuously, and a
    /// 90-second hang that emitted nothing — indistinguishable by elapsed time,
    /// trivially distinguishable by stream liveness.
    pub idle_cap: Duration,
    /// `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS`, set explicitly rather than
    /// inherited. It bounds how long the CLI waits for *background* subagents
    /// after the final result, not foreground agentic work — a 774-second run
    /// exceeded this value by 29% and still completed cleanly. A silent
    /// default is exactly the kind of invisible bound that yields an
    /// unexplained truncated tail (D-14).
    pub bg_wait_ceiling_ms: u64,
    /// `-n/--name`.
    pub name: Option<String>,
    /// How long a caller waits for the `control_response` correlated to its
    /// `control_request` before the wait is abandoned.
    ///
    /// This is a **protocol-layer** reply and does not depend on the model
    /// finishing a turn — the observed acknowledgement is near-immediate — so
    /// the bound is generous by orders of magnitude against what a healthy CLI
    /// takes. It exists so that no caller can be parked for the process
    /// lifetime waiting on an answer that is never coming (D-13, D-31).
    ///
    /// It is the outer of two release paths. The inner one is the run-end
    /// drain, which releases every blocked caller the instant the run ends;
    /// this cap is what covers a child that is still alive and simply not
    /// answering.
    pub control_response_cap: Duration,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            session_id: Uuid::new_v4(),
            target: ExecutionTarget::Host,
            budget_usd: None,
            model: None,
            resume_session: None,
            permission_mode: PermissionMode::DontAsk,
            setting_sources: SettingSources::ProjectOnly,
            // Defensible starting numbers, made configurable rather than tuned:
            // real tuning data does not exist yet and is a v2.1 concern (D-13).
            wall_clock_cap: Duration::from_secs(4 * 60 * 60),
            idle_cap: Duration::from_secs(15 * 60),
            bg_wait_ceiling_ms: 600_000,
            name: None,
            control_response_cap: Duration::from_secs(30),
        }
    }
}

/// Identifies one run within this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExecutionId(pub Uuid);

impl std::fmt::Display for ExecutionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A command for the dedicated stdin writer task.
///
/// The writer is a task of its own, never the task awaiting process exit: the
/// two-pipe deadlock is live here, because the child blocks writing stdout
/// while a parent that owns both blocks writing stdin (D-04).
#[derive(Debug)]
pub(crate) enum WriterCommand {
    /// Write one NDJSON line plus its newline.
    Line(String),
    /// Drop the stdin handle. EOF means "no more input", **not** "stop": the
    /// CLI drains its queued turns, finishes them, and exits 0.
    Close,
}

/// A live run.
///
/// Follows `FileWatcher`'s ownership shape — handed its channels at
/// construction and exposing `&mut self` verbs.
#[derive(Debug)]
pub struct ExecutionHandle {
    /// This run's id.
    pub id: ExecutionId,
    /// The session UUID. Non-optional: it is generated before spawn, so it is
    /// always known, even if the process dies before its first stdout byte.
    pub session_id: String,
    /// Parsed events, in stream order. Bounded, so a blocked consumer applies
    /// backpressure to the reader rather than growing memory without limit.
    pub events: mpsc::Receiver<ExecutionEvent>,
    /// Capabilities observed at the first `system/init`.
    pub capabilities: Vec<String>,
    /// The process **group** id. With `ProcessGroup::leader()` the child is the
    /// group leader, so this equals the child pid. Recorded at spawn because
    /// the concrete group type is not reachable through the returned trait
    /// object, and because Phase 17 needs it for crash reconciliation.
    pub pgid: u32,
    /// `claude_code_version` from the first `system/init`, recorded so Phase 16
    /// can journal it without a signature change (D-07).
    pub claude_code_version: String,
    /// Correlates a `control_request` id to whoever is awaiting its response.
    pub pending_control: PendingControl,

    /// How long [`Executor::interrupt`] waits for its correlated response
    /// before abandoning the wait. Carried from [`ExecutionOptions`] at spawn.
    pub(crate) control_response_cap: Duration,

    pub(crate) stdin_tx: mpsc::Sender<WriterCommand>,
    pub(crate) running: Arc<AtomicBool>,
    pub(crate) cancel_tx: Option<oneshot::Sender<()>>,
    pub(crate) outcome_rx: Option<oneshot::Receiver<RunOutcome>>,
    pub(crate) outcome: Option<RunOutcome>,
    pub(crate) next_control_seq: u64,
}

impl ExecutionHandle {
    /// Whether the run's process is still alive.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Write one raw NDJSON line to the child's stdin.
    pub(crate) async fn write_line(&self, line: String) -> Result<(), SendError> {
        if !self.is_running() {
            return Err(SendError::NotRunning);
        }
        self.stdin_tx
            .send(WriterCommand::Line(line))
            .await
            .map_err(|_| SendError::WriterGone)
    }

    /// Allocate the next `control_request` id for this run.
    pub(crate) fn next_control_id(&mut self) -> String {
        self.next_control_seq += 1;
        format!("req_{}", self.next_control_seq)
    }

    /// Signal end-of-input.
    ///
    /// This is how a single-command run ends: the CLI drains whatever is
    /// queued, finishes, and exits on its own. Closing stdin does **not** kill
    /// the run (D-04).
    pub async fn close_input(&self) -> Result<(), SendError> {
        self.stdin_tx
            .send(WriterCommand::Close)
            .await
            .map_err(|_| SendError::WriterGone)
    }

    /// Await the run's derived outcome. Idempotent — the outcome is cached.
    pub async fn wait_outcome(&mut self) -> RunOutcome {
        if let Some(outcome) = &self.outcome {
            return outcome.clone();
        }
        let outcome = match self.outcome_rx.take() {
            Some(rx) => rx.await.unwrap_or(RunOutcome::Failed {
                reason: "the run ended without reporting an outcome".to_string(),
                subtype: None,
                terminal_reason: None,
                exit_code: None,
            }),
            None => RunOutcome::Failed {
                reason: "the run's outcome channel was already consumed".to_string(),
                subtype: None,
                terminal_reason: None,
                exit_code: None,
            },
        };
        self.outcome = Some(outcome.clone());
        outcome
    }
}

/// One observed thing on the stream.
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// The first `system/init` passed the gate. Emitted exactly once per run.
    SessionStarted {
        /// The session UUID the CLI reported.
        session_id: String,
        /// Everything the CLI advertised.
        capabilities: Vec<String>,
        /// The observed CLI version.
        claude_code_version: String,
        /// `"none"` means subscription/OAuth auth (D-08).
        api_key_source: Option<String>,
        /// Confirms `--permission-mode` actually took effect (D-15).
        permission_mode: Option<String>,
    },
    /// A parsed message of a known type.
    Message(Box<StreamMessage>),
    /// A well-formed line of a type no version we know emits. Forward-compat:
    /// carried, never fatal. Deliberately distinct from `Unparseable` (D-09).
    Unknown {
        /// The raw line.
        raw: String,
    },
    /// A line that did not parse — a torn write or invalid JSON. A real
    /// diagnostic, but still never fails the run.
    Unparseable {
        /// The raw line.
        raw: String,
        /// The parse error, rendered.
        error: String,
    },
    /// A line exceeded `MAX_LINE_BYTES` and was not parsed. The run continues:
    /// an oversize line from the subprocess must not be able to exhaust memory
    /// or abort a run (T-15-07, D-05).
    LineTruncated {
        /// The line's full length **in bytes**.
        bytes: usize,
        /// A short leading excerpt, for the diagnostic.
        prefix: String,
    },
    /// A line the child wrote to stderr. Kept separable from stdout because a
    /// non-empty stderr is itself a signal — the spike's only non-empty stderr
    /// was the workspace-trust warning that silently voids project
    /// `permissions.allow` (D-04).
    Stderr(String),
    /// A `result` envelope closed a **turn** (D-29). Boxed for the same
    /// large-variant reason as the wire model.
    TurnCompleted(Box<ResultMessage>),
    /// Cumulative notional cost as of the last `result`.
    Cost {
        /// `total_cost_usd`, which accumulates across turns.
        cumulative_usd: f64,
    },
    /// The process exited.
    Exited(ExitStatus),
}

/// What one `result` envelope reported.
#[derive(Debug, Clone, PartialEq)]
pub struct TurnOutcome {
    /// Observed `subtype`, carried verbatim.
    pub subtype: String,
    /// Whether the CLI classified this turn as an error.
    pub is_error: bool,
    /// Observed `terminal_reason`, carried verbatim.
    pub terminal_reason: Option<String>,
    /// Per-turn count; it resets, so a run-level count must be summed (D-29).
    pub num_turns: Option<u64>,
    /// Cumulative across turns.
    pub total_cost_usd: Option<f64>,
    /// Stable across the process's turns.
    pub session_id: Option<String>,
}

impl TurnOutcome {
    /// Project a `result` envelope onto a turn outcome.
    pub fn from_result(msg: &ResultMessage) -> Self {
        Self {
            subtype: msg.subtype.clone(),
            is_error: msg.is_error,
            terminal_reason: msg.terminal_reason.clone(),
            num_turns: msg.num_turns,
            total_cost_usd: msg.total_cost_usd,
            session_id: msg.session_id.clone(),
        }
    }
}

/// How a run ended.
///
/// A typed value rather than `Result<(), anyhow::Error>` because the TUI needs
/// a *state* to render, not a message, and retrofitting that after the driver
/// ships means auditing every call site (D-12). The variant set is exactly what
/// the four corroboration sources can actually distinguish.
#[derive(Debug, Clone, PartialEq)]
pub enum RunOutcome {
    /// The last turn reported success **and** the project changed on disk.
    SucceededWithChanges {
        /// Turns observed, in stream order.
        turns: Vec<TurnOutcome>,
        /// Cumulative notional cost from the last `result`.
        total_cost_usd: Option<f64>,
    },
    /// The last turn reported success and **nothing changed**. Reported as its
    /// own outcome rather than as success: an envelope saying `success` while
    /// no artifact moved is precisely the disagreement that makes TRANS-02 a
    /// testable claim rather than an assertion (D-11).
    SucceededNoChanges {
        /// Turns observed, in stream order.
        turns: Vec<TurnOutcome>,
        /// Cumulative notional cost from the last `result`.
        total_cost_usd: Option<f64>,
    },
    /// The run failed, with the CLI's own classification carried verbatim.
    Failed {
        /// Human-readable classification.
        reason: String,
        /// The last `subtype`, if any envelope was observed.
        subtype: Option<String>,
        /// The last `terminal_reason`, if any envelope was observed.
        terminal_reason: Option<String>,
        /// The process exit code. A **liveness signal only**: the hung arm's
        /// exit code came from an external `timeout`, never from Claude (D-10).
        exit_code: Option<i32>,
    },
    /// The run was blocked by permissions. Carries the CLI's own denial
    /// records so the driver can say which tool was denied.
    PermissionDenied {
        /// `permission_denials[]` from the last `result`, unmodelled.
        denials: Vec<serde_json::Value>,
    },
    /// The process group was torn down on request.
    Killed {
        /// Turns observed before teardown.
        turns: Vec<TurnOutcome>,
    },
    /// The total wall-clock cap was breached (D-13).
    TimedOut {
        /// The cap that was breached.
        after: Duration,
    },
    /// The idle cap was breached — no stream line for longer than the cap. The
    /// stuck-detector proper (D-13).
    Stalled {
        /// The cap that was breached.
        idle_for: Duration,
    },
    /// The first `system/init` failed the capability gate. **No turn ever
    /// started**, so this cost zero tokens and zero quota (D-06).
    CapabilityRefused {
        /// The capabilities the CLI did not advertise.
        missing: Vec<String>,
    },
    /// The process could not be launched.
    SpawnFailed {
        /// The spawn failure, rendered.
        reason: String,
    },
}

/// The driver's per-alias live state.
///
/// This is the small value plan 15-06 stores in a **sibling map** on
/// `AppContext`. It must never be added to `ProjectState`: that type derives
/// `PartialEq` and `app.rs` relies on the equality to suppress
/// "Updated: {alias}" status spam, while driver state changes every few seconds
/// — one 68-second turn emitted 22 `thinking_tokens` events (D-19).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum RunState {
    /// No run for this alias.
    #[default]
    Idle,
    /// Spawned; awaiting the first `system/init`.
    Starting,
    /// Gated and streaming.
    Running,
    /// Teardown in progress.
    Stopping,
    /// Finished, with its derived outcome.
    Finished(Box<RunOutcome>),
}

/// The result of an interrupt request.
///
/// `subtype == "success"` means **the request was accepted**, not that the
/// thing you meant was cancelled: the spike observed a `success` response with
/// an empty `still_queued` that cancelled nothing meaningful, because the
/// interrupt landed before the target turn was streaming. Read `still_queued`
/// as authoritative, and treat the subsequent `[Request interrupted by user]`
/// message plus `terminal_reason: "aborted_streaming"` as the real
/// confirmation (D-31, Pitfall D).
#[derive(Debug, Clone, PartialEq)]
pub struct InterruptAck {
    /// The id this response correlated to.
    pub request_id: String,
    /// The response subtype, carried verbatim.
    pub subtype: String,
    /// What the CLI says is still queued. Authoritative.
    pub still_queued: Vec<serde_json::Value>,
}

impl InterruptAck {
    /// Project a correlated `control_response` onto an ack.
    pub fn from_response(response: &ControlResponse) -> Self {
        Self {
            request_id: response.response.request_id.clone(),
            subtype: response.response.subtype.clone(),
            still_queued: response.response.still_queued(),
        }
    }
}

/// Build the NDJSON line for an interrupt request.
pub(crate) fn encode_interrupt(request_id: &str) -> Result<String, SendError> {
    serde_json::to_string(&ControlRequest::interrupt(request_id)).map_err(SendError::Encode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_sources_defaults_to_project_only() {
        assert_eq!(
            SettingSources::default().as_flag_value(),
            "project",
            "the user tier re-exposes the hook hang and must stay opt-in (D-01)"
        );
    }

    #[test]
    fn permission_mode_offers_no_bypass_value() {
        assert_eq!(
            PermissionMode::default().as_flag_value(),
            "dontAsk",
            "a bypass mode must not be reachable on the host (D-15)"
        );
    }

    #[test]
    fn a_drivable_project_carries_its_alias_and_root() {
        let project = DrivableProject::for_testing("demo", "/tmp/demo");
        assert_eq!(project.alias(), "demo");
        assert_eq!(project.root(), Path::new("/tmp/demo"));
    }

    #[test]
    fn the_default_control_response_cap_is_bounded_and_non_zero() {
        let cap = ExecutionOptions::default().control_response_cap;
        assert!(
            cap > Duration::ZERO,
            "a zero cap would abandon every request before the child could answer it"
        );
        assert!(
            cap <= Duration::from_secs(60),
            "the acknowledgement is a protocol-layer reply and does not wait on a turn, so \
             the cap must stay bounded and generous rather than effectively absent — an \
             unbounded wait is what let an unanswered interrupt park its caller for the \
             whole process lifetime (D-13, CR-03). Observed: {cap:?}"
        );
    }

    #[test]
    fn interrupt_request_encodes_to_the_observed_wire_shape() {
        let line = encode_interrupt("req_1").expect("encode");
        assert_eq!(
            line,
            r#"{"type":"control_request","request_id":"req_1","request":{"subtype":"interrupt"}}"#,
            "the bare interrupt form does nothing and must never be emitted (D-31)"
        );
    }
}
