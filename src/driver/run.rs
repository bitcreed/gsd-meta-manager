//! The run body: gate, journal start, executor spawn, event drain, journal
//! finish.
//!
//! This module is declared `#[cfg(unix)]` by its parent, so it carries no inner
//! attribute of its own — the portable surface lives in
//! [`crate::driver`](super) and only the implementation is gated (D-05).
//!
//! It is the **first production caller** of Phase 15's `ClaudeExecutor` and
//! Phase 16's `JournalRun`. Both were shipped complete and both were dead code
//! until this file existed; `src/journal/mod.rs` says so in as many words.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rustix::process::Signal;
use tokio::sync::{mpsc, oneshot};

use crate::config::RegisteredProject;
use crate::driver::{
    bounds, kill, liveness, lock, rate_limit, router, DriveArgs, ROUTED_RECORD_MARKER,
};
use crate::envelope::advisory::{self, ProtectionState};
use crate::envelope::cred::{self, EnvelopeEnv};
use crate::envelope::hooks;
use crate::envelope::policy::{self, ParkReason};
use crate::error::{DriveError, LockError};
use crate::executor::claude::ClaudeExecutor;
use crate::executor::stream_json::{StreamMessage, UserMessage};
use crate::executor::{
    DrivableProject, ExecutionEvent, ExecutionHandle, ExecutionOptions, Executor, RunOutcome,
};
use crate::journal::inbox::{self, InboxMessage};
use crate::journal::reader::TailCursor;
use crate::journal::{self, JournalEvent, JournalRun, RunRecord};

/// The diagnostic code the terminate-signal shutdown journals before its
/// terminal record.
///
/// A fixed identifier rather than a sentence, because it is what a later reader
/// greps for: a run that ends with the killed outcome could have been stopped
/// from the TUI, stopped from a shell, or stopped by a service manager, and this
/// record is the only thing that says a terminate signal is what arrived.
const TERMINATE_DIAGNOSTIC_CODE: &str = "terminate_signal_shutdown";

/// The prefix of the diagnostic code the run-start envelope notice carries.
///
/// Suffixed with [`ProtectionState::as_str`], so the code a reader greps for is
/// `envelope_protection_protected`, `…_unprotected` or `…_unknown` — the stable
/// identifiers `envelope::advisory` already owns, in the snake_case shape every
/// other `Diagnostic` code in this tree uses.
const PROTECTION_DIAGNOSTIC_PREFIX: &str = "envelope_protection_";

/// Everything one established envelope hands to the run that it protects.
///
/// A single value rather than four out-parameters because establishment is
/// all-or-nothing: a run with three of the four layers is the partial envelope
/// [`DriveError::EnvelopeAssertionFailed`] exists to refuse.
struct EstablishedEnvelope {
    /// The generated settings file, for `--settings` (D-07).
    settings: PathBuf,
    /// The child's whole environment as a value (D-09, D-16).
    env: EnvelopeEnv,
    /// What the read-only probe found on the remote (D-26). Never a refusal:
    /// an unknown protection state is a fact about the probe, not about the run.
    protection: ProtectionState,
}

/// Establish the envelope for `alias` against `project_root`, or refuse.
///
/// **Synchronous by design and called only from inside `spawn_blocking`.** Every
/// line below is file or process work: four generated files, two `git config`
/// reads, and a bounded external-client probe. `<D-28, WR-10>` forbids that on an
/// `async fn` path, and the deadlock the discipline prevents was **observed**
/// rather than theorised — `tests/driver_lock.rs:201-215` records a blocking
/// `flock` inside an `async fn` defeating `tokio::time::timeout` outright on a
/// current-thread runtime.
///
/// The order is the decision:
///
/// 1. `envelope::hooks::install` ([`hooks::install`]) first, because the
///    hooks directory is what
///    `core.hooksPath` in the generated environment will point at, and an
///    environment naming a directory that does not exist is an environment that
///    delivers nothing.
/// 2. [`hooks::write_settings`] second, and **its `?` is the D-07 gate**. That
///    function writes the file, reads it back and compares; a mismatch is an
///    error and must never be downgraded to a warning, because a settings file
///    that failed validation is silently ignored by the CLI with nothing shown.
/// 3. [`hooks::write_exclude_block`] third — the one persistent mutation the
///    envelope makes to the driven repository, so it happens before anything
///    the agent could observe. It is the one layer with a **skip** condition,
///    and the distinction the condition draws is *absence versus failure*: a
///    project with no `.git` entry has no repository for an ignore rule to
///    protect and no history for a swept file to reach, so the block is
///    unnecessary rather than unwritable. A `.git` that exists and cannot be
///    written to is a failure and refuses, because that is a repository whose
///    protection was attempted and did not land. Refusing every non-git project
///    outright would be a control failing into unusability, which is the shape
///    of control that gets switched off.
/// 4. [`cred::build_env`] last, because it is the layer that depends on the
///    other three: it writes the generated git config (which is why
///    `cred::write_gitconfig` is **not** called separately here — a second call
///    would resolve the user's identity twice and could write two different
///    files), creates the `gh` directory, generates the askpass responder, and
///    folds in the `core.hooksPath` entry naming the directory step 1 created.
///
/// The probe runs **once**, here, after the environment exists — it needs that
/// environment to run the external client under. One producer means the claim
/// the dry-run preview makes and the claim the run journal records cannot
/// disagree (T-19-42).
fn establish_envelope(
    alias: &str,
    project_root: &Path,
    run_id: &str,
) -> anyhow::Result<EstablishedEnvelope> {
    hooks::install(alias)?;
    let settings = hooks::write_settings(alias)?;
    // `.git` covers both forms — the directory, and the pointer file a linked
    // worktree carries — which is exactly the pair `hooks::git_dir` resolves, so
    // this predicate and that function cannot disagree about what a repository
    // is. Existence is asked here rather than inside `write_exclude_block`
    // because that function's failure IS the refusal for every caller that has a
    // repository, and weakening it there would weaken it for all of them.
    if project_root.join(".git").exists() {
        hooks::write_exclude_block(project_root)?;
    }
    let env = cred::build_env(alias, project_root)?.with_run_id(run_id);
    let protection = advisory::probe_protection(project_root, &env);

    Ok(EstablishedEnvelope {
        settings,
        env,
        protection,
    })
}

/// The program this driver execs unless a debug build was told otherwise.
///
/// A named constant because a release build has exactly one answer here and the
/// override that produced the other one is gone from the parser (D-30).
const DEFAULT_AGENT_PROGRAM: &str = "claude";

/// The diagnostic code that marks a run driven by a stand-in rather than by the
/// agent (D-30, WR-16).
///
/// **The second half of D-30, and it exists because the first half cannot cover
/// a debug build.** `--claude-program` has no parser entry in release, so there
/// is nothing to mark there; in a debug build the flag must keep working, since
/// the nine `tests/fixtures/fake-claude*.sh` stand-ins are how every driver
/// integration test runs without a subscription. Without this record such a run
/// writes `run_started`, `exec_started`, `run_ended` and an outcome — a
/// transcript indistinguishable from a real agent's. A short stable identifier
/// rather than a sentence, for the same reason as
/// [`TERMINATE_DIAGNOSTIC_CODE`]: it is what a later reader greps for.
#[cfg(debug_assertions)]
const AGENT_PROGRAM_OVERRIDDEN: &str = "agent_program_overridden";

/// How long the agent's group is given between the terminate signal and the
/// uncatchable one **when the stop arrives during startup**.
///
/// **Five seconds, not the drain path's ten, and the difference is the state the
/// agent is in.** The drain path's ten seconds exist for a working agent: a turn
/// to abort, a Bash tree mid-command with its own signal handler, and a
/// `SessionEnd` hook chain that is allowed to finish. During startup the agent is
/// parked at — or before — its first `system/init`. There is no turn, no
/// grandchild mid-build, and no hook chain worth ten seconds of a user's stop.
///
/// The upper bound is not taste. The whole startup teardown has to fit inside
/// [`crate::driver::kill::DRIVER_TEARDOWN_GRACE`], the twelve seconds the TUI
/// gives the driver before it escalates to SIGKILL — and a driver SIGKILLed
/// mid-teardown orphans the agent group, which is the exact failure this path
/// exists to prevent. Five plus [`STARTUP_REAP_BOUND`] plus the journal's two is
/// nine, which fits with three to spare, and
/// `the_startup_stop_budget_fits_inside_the_driver_teardown_grace` asserts that
/// rather than this comment claiming it. The drain path does not fit and does not
/// need to: those twelve seconds were sized *for* it, by
/// `the_driver_grace_exceeds_the_claude_group_grace_plus_slack`.
const STARTUP_AGENT_GRACE: Duration = Duration::from_secs(5);

/// How long the uncatchable signal is given to take effect on the startup path.
///
/// SIGKILL cannot be caught, so this is a reap window rather than a grace, and it
/// mirrors `kill::KILL_REAP_BOUND` for the same reason: two seconds is enough to
/// observe a `/proc` entry disappear and short enough to leave room in the
/// budget above.
const STARTUP_REAP_BOUND: Duration = Duration::from_secs(2);

/// How often the agent group is re-probed while waiting out either bound above.
///
/// A signal is delivered asynchronously and a just-signalled process is briefly
/// still a pid, so the wait is "gone soon" rather than "gone now" — the same
/// reasoning, and the same tenth of a second, as `kill::DEATH_POLL_INTERVAL`.
const STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// How often the drain loop looks in this run's `inbox.jsonl` for a message the
/// user queued from the TUI (D-04, STEER-01).
///
/// **Polling, deliberately, rather than a `notify` watcher.** The driver is a
/// detached process that has no watcher today, and adding one to save half a
/// second is not a trade this phase needs to make: the unit of work here is
/// *minutes* — a GSD command running an agent turn — so a sub-second injection
/// latency buys nothing a user can perceive. A watcher would also put a second
/// event source and a second failure mode inside a process whose entire value is
/// being simple enough to survive its parent.
///
/// 750 ms is a defensible starting value with **no tuning data behind it** — it
/// is a named constant so tuning is a one-line change, following the
/// `main_loop.rs:41-49` idiom.
const INBOX_POLL_INTERVAL: Duration = Duration::from_millis(750);

/// The two reasons recorded on a message the run could not deliver (D-10).
///
/// Fixed sentences rather than ones composed at the call site, so the four-state
/// display renders one string per reason and a later reader greps for one thing.
/// They are defined in [`crate::journal`] beside the event that carries them,
/// because this module is `#[cfg(unix)]` and the render layer that has to
/// recognise them is not.
use crate::journal::{MISSED_AFTER_CLOSE, MISSED_SEND_FAILED};

/// The wire name of the marker that makes a `user` envelope a **replay echo**.
///
/// camelCase on the wire, and **absent rather than `false`** on an ordinary
/// message — so the test is "is this exactly `true`", never "is this not
/// `false`". The parsed model spells the same field `is_replay`
/// (`src/executor/stream_json.rs:113-122`) and
/// [`ClaudeExecutor::observing_replay_echoes`](crate::executor::claude::ClaudeExecutor::observing_replay_echoes)
/// filters on it before a line ever reaches this module; re-reading it here is
/// the second of two independent checks, and it is cheap.
const REPLAY_MARKER: &str = "isReplay";

/// Messages written to the agent's stdin that have not yet been echoed back.
///
/// A FIFO of `(id, text as sent)`, and both halves are load-bearing. The **id**
/// is what the journal record names, because text alone is not a correlation
/// key — a user may legitimately send the same sentence twice, and STEER-02's
/// states are per message rather than per string. The **text** is the only thing
/// the echo carries that can be matched against, because the CLI's replay echo
/// reproduces the body and mints its own `uuid`.
///
/// Nothing prunes this except a matched echo and the end of the run, and that is
/// correct: it holds at most one entry per message the user injected, which is a
/// number bounded by how fast a human types.
#[derive(Debug, Default)]
struct PendingAcks {
    entries: VecDeque<(String, String)>,
}

impl PendingAcks {
    /// Record that `text` was written to stdin under `id`.
    ///
    /// Called at exactly one moment: immediately after `Executor::send` returned
    /// `Ok`. A message whose write failed is never pushed, because it will never
    /// be echoed and would sit here shadowing a later identical message.
    fn push_delivered(&mut self, id: String, text: String) {
        self.entries.push_back((id, text));
    }

    /// The id of the message `echoed_text` acks, if any.
    fn match_echo(&mut self, echoed_text: &str) -> Option<String> {
        match_replay_echo(&mut self.entries, echoed_text)
    }

    /// Take every id still waiting for an echo when the run ends.
    ///
    /// **These messages were delivered and are not `missed`.** Each keeps its
    /// `interjected` record and simply never gains an `interjection_acted_on`,
    /// which is the honest answer: the run ended before the agent dequeued them,
    /// and fabricating the transition would assert an observation the driver
    /// never made. The ids are returned so the count can be logged; they are
    /// deliberately not journaled as anything.
    fn drain_undelivered(&mut self) -> Vec<String> {
        self.entries.drain(..).map(|(id, _)| id).collect()
    }
}

/// The id of the first pending message whose text is **exactly** `echoed_text`.
///
/// **`is_replay: true` is emitted at DEQUEUE, not at receipt.** Measured against
/// CLI 2.1.220: a message written at t=12s was echoed at t=68s, 45 ms after the
/// *previous* turn's `result` (D-07, Phase 15 D-31). The echo therefore means
/// *"the agent has started processing this"*, and the only correct word for the
/// state it establishes is **acted-on**. **"received", "read" and "acknowledged"
/// are forbidden renderings**: each promises an observation 55 seconds earlier
/// than the one the protocol actually supports, and the whole point of the
/// three-state display is that each state names evidence that exists.
///
/// Matching is **exact `String` equality on the UTF-8 text as sent** — no
/// trimming, no Unicode normalisation, no case folding. Anything looser would
/// let a message the user did not send ack one they did. The scan runs **front
/// to back and removes the first match**, so two legitimately identical messages
/// are acked in delivery order; matching the newest first would let the second
/// echo re-ack the first message and leave the second showing `delivered`
/// forever.
///
/// **Pure, and that is what makes it testable at all** — the register of
/// `driver/mod.rs:129-135`. Exercising this against a real agent would need a
/// process, a dequeue delay and two messages with the same body; fed a deque
/// directly it is four assertions.
///
/// **The declined alternative was correlating in the TUI** from
/// `ExecEvent.text`. That was rejected because `ExecEvent.text` is a *projection*
/// written for a human to read, and reconstructing protocol semantics from a
/// rendered string is exactly the screen-scraping D-01 forbids, wearing a
/// different hat. Only the driver has parsed envelopes, so only the driver can
/// answer this honestly (D-08).
fn match_replay_echo(pending: &mut VecDeque<(String, String)>, echoed_text: &str) -> Option<String> {
    let index = pending.iter().position(|(_, text)| text == echoed_text)?;
    pending.remove(index).map(|(id, _)| id)
}

/// The text an echoed `user` line carries, or `None` if it is not an echo.
///
/// The driver re-reads the `is_replay` marker off the raw line rather than
/// trusting the executor's filter alone, and then walks the body itself. Both
/// halves are deliberate: this is the party D-08 makes responsible for the
/// protocol reading, and a `user` line **without** the marker is a tool result —
/// not an echo of anything the driver sent, and never allowed to ack an injected
/// message.
///
/// Tolerant by construction, following `src/executor/stream_json.rs`'s whole
/// premise: a body shape no version we know emits yields `None` and a logged
/// non-event, never a parse failure and never a run-ending error. `content` is
/// accepted both as an array of blocks and as a bare string, because the CLI has
/// shipped both shapes; only `text` blocks contribute, and they are concatenated
/// in order so a multi-block echo of a single-block send still compares equal.
fn replay_echo_text(raw: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;

    if value.get(REPLAY_MARKER).and_then(serde_json::Value::as_bool) != Some(true) {
        return None;
    }

    match value.get("message")?.get("content")? {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Array(blocks) => {
            let mut text = String::new();
            for block in blocks {
                if block.get("type").and_then(serde_json::Value::as_str) == Some("text") {
                    if let Some(chunk) = block.get("text").and_then(serde_json::Value::as_str) {
                        text.push_str(chunk);
                    }
                }
            }
            Some(text)
        }
        _ => None,
    }
}

/// Turn one observed replay echo into an `interjection_acted_on` record.
///
/// An **unmatched** echo is logged by kind and otherwise ignored. That is the
/// right answer rather than a lenient one: the agent echoes every `user` message
/// it dequeues, including the run's own command prompt, and a hostile or merely
/// unexpected echo carrying text the driver never sent must not be able to
/// invent a transition. It is informational in exactly the way a later
/// `system/init` is (Phase 15 D-30), and it must never abort a run.
///
/// **No log line ever carries the echoed text.** It is either the user's own
/// words or the agent's, and both are message bodies (T-18-12, `PATTERNS` §S3).
fn correlate_replay_echo(pending: &mut PendingAcks, journal: &mut JournalRun, raw: &str) {
    let Some(text) = replay_echo_text(raw) else {
        tracing::debug!("a replay echo carried no readable text body");
        return;
    };

    let Some(id) = pending.match_echo(&text) else {
        tracing::debug!("a replay echo matched no message this driver delivered");
        return;
    };

    if let Err(err) = journal.record(&JournalEvent::InterjectionActedOn { id }) {
        tracing::warn!(kind = ?err.kind(), "journal write failed");
    }
}

/// The state one driver process holds for the duration of one run.
pub struct DriverRun {
    /// The run's journal, open from `start` to `finish`.
    journal: JournalRun,
    /// The single-execution lock, held for the whole run (D-20.2).
    ///
    /// **This field is deliberately never read, and it is not bookkeeping.** An
    /// advisory `flock` is held per *open file description*, so dropping this
    /// value closes the descriptor and releases the lock — the field's only job
    /// is to keep it alive, and its placement on the run state rather than in a
    /// local is what makes "held for the run's duration" true. It is not renamed
    /// to `_lock`: an underscore reads as "leftover" and would invite the next
    /// reader to delete the lock along with it.
    #[allow(dead_code)]
    lock: lock::RunLock,
}

/// Map a derived outcome onto the short label the journal records.
///
/// Kept as its own function rather than inlined so plan 17-06 reuses it for the
/// killed path instead of inventing a second vocabulary for the same states.
///
/// `pub(crate)` since plan 18-10 so the render layer's terminal-state table can
/// be proved to read **exactly** the vocabulary this function writes. The two
/// sides are a string protocol across a process boundary, and a test that spells
/// the labels out a second time would agree with itself while disagreeing with
/// disk.
pub(crate) fn outcome_label(outcome: &RunOutcome) -> &'static str {
    match outcome {
        RunOutcome::SucceededWithChanges { .. } => "succeeded_with_changes",
        RunOutcome::SucceededNoChanges { .. } => "succeeded_no_changes",
        RunOutcome::Failed { .. } => "failed",
        RunOutcome::PermissionDenied { .. } => "permission_denied",
        RunOutcome::Killed { .. } => "killed",
        RunOutcome::TimedOut { .. } => "timed_out",
        RunOutcome::Stalled { .. } => "stalled",
        RunOutcome::CapabilityRefused { .. } => "capability_refused",
        RunOutcome::SpawnFailed { .. } => "spawn_failed",
    }
}

/// The prefix a terminal label carries when the run's journal names a park.
///
/// `parked:force_push_blocked` rather than a bare `parked`, so the terminal
/// record answers *why* without a second read of the journal — and the suffix is
/// [`ParkReason::as_str`], never a fresh string.
pub(crate) const PARKED_LABEL_PREFIX: &str = "parked:";

/// How a run ended, with no unclassified arm (DRIVE-06, D-25).
///
/// **Five arms, and the absence of a sixth is the requirement.** Criterion 5's
/// *"never as an unclassified 'loop ended'"* is a **type-level** property rather
/// than a logging convention: if the type cannot express "ended for no stated
/// reason", that failure mode is unrepresentable. Every match on this type is
/// exhaustive with no wildcard, so an arm added later is a compile error at each
/// site that has to classify it.
///
/// Every arm carries a reason, and every reason reaches disk through machinery
/// that already existed rather than through a third string source:
/// [`Terminal::Parked`] and [`Terminal::Halted`] write `JournalEvent::Parked`
/// with their sibling enum's `as_str()`, which [`terminal_label`] picks up
/// through the [`PARKED_LABEL_PREFIX`]; [`Terminal::Completed`] writes no park
/// record and so falls through to [`outcome_label`], which is the same label
/// every Phase 17 run already carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Terminal {
    /// The run performed every command it was asked to perform.
    ///
    /// Its reason is the derived [`RunOutcome`]'s own label —
    /// `succeeded_with_changes`, `failed`, `timed_out` and the six others
    /// [`outcome_label`] spells out. It is reached in single-command mode when
    /// the one supplied command finishes, and in routed mode when the agent's
    /// own outcome ends the run. It is named for what it asserts: the work ran
    /// to its end.
    ///
    /// **It is no longer DRIVE-06's goal-met arm** — [`Terminal::GoalMet`] is,
    /// and the split is WR-08. Carrying both meanings here made the label depend
    /// on whether any iteration had spawned: a target already met at launch
    /// wrote `goal_met`, and a target the run actually *achieved* wrote
    /// `succeeded_with_changes`, so the more interesting of the two goal-met
    /// cases was the one that did not say so.
    Completed,
    /// The declared target was reached: the `--target-phase`'s verification
    /// frontmatter reads `passed` (CONTEXT.md OQ4, [`router::is_goal_met`]).
    ///
    /// **A fact about project state, never a claim by the agent**, and never an
    /// inference from artifact presence — which is exactly what would step past
    /// the `human_needed` gate DRIVE-05 exists to park at.
    ///
    /// Its label is [`GOAL_MET_LABEL`] whether or not this run issued a single
    /// command. A run that found the target already met and a run that drove it
    /// there ended in the same state, and a reader of `run.json` should not have
    /// to infer goal-met from the absence of a `parked:` prefix.
    GoalMet,
    /// The router refused to choose a next command.
    Parked {
        /// The taxonomy member, from the router's closed reason set.
        reason: router::RouterReason,
        /// One token naming what was observed. Empty when there is nothing to
        /// add beyond the reason itself.
        detail: String,
    },
    /// A run bound fired, so the run halted **before** the next spawn.
    Halted {
        /// Which one detector fired. Exactly one, never a list: CTRL-06 asks
        /// which detector fired, and [`bounds::BoundVerdict`] cannot express
        /// more than one.
        reason: bounds::BoundsReason,
    },
    /// The transport reported a Claude subscription quota rejection, so the run
    /// stopped (CTRL-07).
    ///
    /// **Its own arm rather than a fifth [`bounds::BoundsReason`]**, because the
    /// four bounds are facts about *this* run's budget while a quota rejection is
    /// a fact about a budget shared with every other Claude surface the user has.
    /// That difference is the whole reason this arm never retries: a backoff
    /// would spend somebody else's remaining quota as well as this run's.
    QuotaParked {
        /// Which window blocked the run and when it resets, from
        /// [`rate_limit::park_detail`]. Never a dollar figure (D-16).
        detail: String,
    },
}

impl Terminal {
    /// The stable identifier this terminal writes into `JournalEvent::Parked`,
    /// or `None` when the run ends on its outcome label instead.
    ///
    /// Exhaustive, no wildcard. The reason is always the sibling enum's
    /// `as_str()` — never a fresh string minted here — so `grep bounds_no_progress`
    /// finds the detector and the record it produced together.
    fn park_reason(&self) -> Option<&'static str> {
        match self {
            Terminal::Completed | Terminal::GoalMet => None,
            Terminal::Parked { reason, .. } => Some(reason.as_str()),
            Terminal::Halted { reason } => Some(reason.as_str()),
            Terminal::QuotaParked { .. } => Some(rate_limit::QuotaReason::Rejected.as_str()),
        }
    }

    /// The one token this terminal records beside its reason, or empty when the
    /// reason says everything there is to say.
    ///
    /// Exhaustive, no wildcard, for the same reason [`Terminal::park_reason`] is:
    /// an arm added later must be a decision here rather than a silent fall
    /// through to "no detail", which is how a park loses the only field that
    /// says *which* window or *which* state it was about.
    fn detail(&self) -> &str {
        match self {
            Terminal::Completed | Terminal::GoalMet | Terminal::Halted { .. } => "",
            Terminal::Parked { detail, .. } => detail,
            Terminal::QuotaParked { detail } => detail,
        }
    }

    /// What would unpark this run, in the register `JournalEvent::Parked.needs`
    /// documents: a short phrase naming the actor, never a sentence of advice.
    ///
    /// Every reason in both sibling taxonomies needs a person. A bound that
    /// fired wants an operator to decide whether to raise it or to fix the
    /// stall; a router that found no rule wants one to widen the table. Neither
    /// is something the driver may decide for itself, which is the whole of
    /// CONTEXT.md's always-park resolution.
    fn needs(&self) -> &'static str {
        match self {
            Terminal::Completed | Terminal::GoalMet => "",
            Terminal::Parked { .. } | Terminal::Halted { .. } => "human",
            // A quota park needs a person more plainly than either sibling: the
            // only thing that unparks it is a human deciding to wait out the
            // window or to run on a different surface. The driver may not decide
            // that for itself, and specifically may not decide it by sleeping.
            Terminal::QuotaParked { .. } => "human",
        }
    }
}

/// Where each iteration's command comes from.
///
/// Two arms, mutually exclusive, refused at the seam in
/// [`crate::driver::drive`] if a caller supplies both or neither. The split is
/// what keeps single-command mode byte-for-byte unchanged: [`Fixed`] runs
/// exactly one iteration through exactly the Phase 17 path, with no snapshot
/// capture, no bounds evaluation, no router call and no `observed`/`decided`
/// record.
///
/// [`Fixed`]: CommandSource::Fixed
enum CommandSource {
    /// One supplied command, one iteration.
    Fixed(String),
    /// A routed sequence driving toward a phase.
    Routed {
        /// The phase number, already validated as a plain path component. It is
        /// a map key and never a path component in practice (T-20-03).
        target_phase: String,
    },
}

/// The label this run's **own** terminal decides, or `None` when the answer has
/// to come from the outcome and the journal.
///
/// **It takes no path, and that is the property rather than a convenience.** A
/// run that halted on a bound, parked at a gate or hit a quota already holds the
/// authoritative reason in memory; deriving it by re-reading the journal makes a
/// classification depend on an I/O operation that can fail — and
/// [`record_terminal`] deliberately swallows a failed journal write with a
/// `warn!`, so the failure is silent by design. WR-01 is what that cost: a run
/// whose `Parked` record failed to land reported `succeeded_with_changes`.
/// Because this signature cannot express a read, no read can lose the reason.
///
/// `None` is the genuinely journal-shaped case — this run stopped for no reason
/// of its own, so a park recorded by another process (the hook or guard
/// re-entries) may still decide the label.
fn own_terminal_label(terminal: &Terminal) -> Option<String> {
    match terminal {
        // **Regardless of whether any iteration spawned** (WR-08). The same
        // terminal condition — the declared target's verification passed — must
        // not write `goal_met` when the target was already met at launch and
        // `succeeded_with_changes` when this run is what achieved it.
        Terminal::GoalMet => Some(GOAL_MET_LABEL.to_string()),
        Terminal::Completed => None,
        Terminal::Parked { .. } | Terminal::Halted { .. } | Terminal::QuotaParked { .. } => terminal
            .park_reason()
            .map(|reason| format!("{PARKED_LABEL_PREFIX}{reason}")),
    }
}

/// The label the terminal `run.json` carries, which is
/// [`outcome_label`] **unless the run's journal names a park** (D-25).
///
/// D-25 requires the run's terminal record to carry the park reason, and
/// `outcome_label` cannot know one: it maps a derived [`RunOutcome`], and a park
/// is produced by *other processes* — the hook and guard re-entries — that this
/// driver never observes. The journal is the only thing the two sides share, so
/// the label is decided by reading it once at the end of the run.
///
/// **The LAST park wins.** A run may be refused more than once — a force push,
/// then a secret, then a pull-request cap — and the terminal record names the
/// state the run ended in, which is the one that was still true when it stopped.
///
/// A journal that cannot be read yields [`outcome_label`], not a panic and not a
/// guess: a park that cannot be read is a park that was not observed, and
/// claiming one would be inventing evidence in the file this phase exists to
/// make trustworthy.
///
/// **Synchronous, and every caller hands it to a blocking task.** It is one
/// end-of-run read rather than a hot path, but it is still `read_all` on an
/// `async fn`'s path (D-28, WR-10), so both call sites move it into
/// `spawn_blocking` alongside the value it needs.
pub(crate) fn terminal_label(outcome: &RunOutcome, journal: &Path) -> String {
    let Ok((records, _diagnostics)) = journal::reader::read_all(journal) else {
        // A journal that cannot be read is a park that was not observed.
        // Claiming one would be inventing evidence in the file this phase
        // exists to make trustworthy.
        return outcome_label(outcome).to_string();
    };

    match records
        .iter()
        .rev()
        .find(|record| record.kind == "parked")
        .and_then(|record| record.rest["reason"].as_str())
    {
        Some(reason) => format!("{PARKED_LABEL_PREFIX}{reason}"),
        None => outcome_label(outcome).to_string(),
    }
}

/// Build the immutable half of `run.json`.
///
/// **`run.json` is written by the driver, not by the TUI before spawning.** The
/// record carries the driver's own pid and pgid, which only the driver knows. A
/// spawn failure is synchronous and reportable in the TUI, and correctly leaves
/// nothing at all on disk rather than a half-record (D-03).
///
/// **`pgid` is a parameter and not a second `std::process::id()` call, and that
/// is WR-01.** The two are equal for every run that reaches this point, but they
/// were equal here because the same expression was written twice rather than
/// because anything had been observed. On a failed `setpgid` — `EPERM` for a
/// session leader is the realistic one — the process does **not** lead its own
/// group, and the record asserted a leadership it did not hold. What that costs
/// downstream is concrete: `kill::resolve_signal_target` compares the recorded
/// group against the kernel's, so such a record makes every stop against that run
/// refuse, and the user's kill switch stops working for a reason nothing on
/// screen can explain. The caller passes what [`current_group`] reports.
fn make_run_record(
    run_id: String,
    args: &DriveArgs,
    entry: &RegisteredProject,
    options: &ExecutionOptions,
    argv_digest: String,
    pgid: u32,
    run_bounds: bounds::RunBounds,
) -> RunRecord {
    RunRecord {
        run_id,
        goal: args.goal.clone().unwrap_or_default(),
        // **A single `String`, and it stays one under a multi-command loop.**
        // `run.json` is written exactly twice, so this field cannot accumulate;
        // the per-iteration sequence belongs on the journal's `decided` records,
        // which is where a reader finds every command a routed run issued and in
        // what order. A routed run records the marker rather than a command,
        // because it chose none at this point and will choose several later.
        gsd_command: recorded_command(args),
        // The routed run's identity, in a field whose type says what it is
        // rather than smuggled into one whose name says command.
        target_phase: args.target_phase.clone(),
        // **The resolved caps, never the constants.** This is the whole of what
        // the field is for: a reader answering "what was this run allowed to
        // do?" must not have to work out which binary produced the record and
        // what its compiled-in defaults were at the time.
        bounds: Some(journal::RecordedBounds {
            max_steps: run_bounds.max_steps,
            wall_clock_cap_secs: run_bounds.wall_clock_cap.as_secs(),
        }),
        target: format!("{:?}", options.target),
        // The field Phase 16 reserved at `src/journal/mod.rs:475` specifically
        // so this phase adds no migration.
        opt_in: entry
            .driver_opt_in
            .as_ref()
            .map(|record| record.opted_in_at.clone()),
        started_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        session_id: options.session_id.to_string(),
        // D-04's arithmetic still holds — `pid == pgid` for every run that gets
        // here — but it now holds because the group was **observed** rather than
        // because the same expression was written on both lines. A record that
        // asserts a leadership the process does not hold is worse than one that
        // reports an inherited group: the first makes the kill switch refuse,
        // the second at least tells the truth about what to signal.
        pid: std::process::id(),
        pgid,
        // Empty until the first `system/init`. Record what is known; nothing
        // overwrites it, because `run.json` is written exactly twice.
        claude_code_version: String::new(),
        argv_digest,
        ended_at: None,
        outcome: None,
        // Empty at write one and at write two: this build models every field it
        // writes. The map exists so a record written by a *newer* build survives
        // a round trip through this struct rather than being silently pruned
        // (T-20-08).
        extra: serde_json::Map::new(),
    }
}

/// What `run.json`'s single `gsd_command` field carries.
///
/// Single-command mode records the command, exactly as it always did. A routed
/// run has no single command to record — that is the point of it — so it records
/// [`ROUTED_RECORD_MARKER`], and its identity moves to the typed sibling field
/// [`RunRecord::target_phase`].
///
/// **This deliberately no longer records `--target-phase N`.** That earlier
/// value put an *argv fragment* in a field named `gsd_command` and rendered it
/// to the user as `cmd: --target-phase 3` — something that reads as a pasteable
/// command line and is not one. `run.json` carries no version discriminator, so
/// a field whose meaning drifts silently reinterprets every record already on
/// disk; the marker is the one value that can be neither pasted as a command nor
/// mistaken for an absent field.
///
/// The argv **digest** does not go through here, and that separation is the
/// point — see [`digested_command_fragment`].
fn recorded_command(args: &DriveArgs) -> String {
    match (&args.command, &args.target_phase) {
        (Some(command), _) => command.clone(),
        (None, Some(_)) => ROUTED_RECORD_MARKER.to_string(),
        // Unreachable: `driver::drive` refuses a run with neither before
        // anything is created. An empty string rather than a panic, because a
        // detached driver that panicked here would leave a run directory with no
        // terminal record, which is the crash signal D-12 reserves for a genuine
        // crash.
        (None, None) => String::new(),
    }
}

/// The command-selecting fragment of the **driver's own argv**, for the digest.
///
/// Split from [`recorded_command`] when the record started carrying a marker,
/// because the two answer different questions and one string cannot answer both:
///
/// * `recorded_command` answers *"what command did this run issue?"* — for a
///   routed run, none in particular.
/// * This answers *"what command line was this driver invoked with?"* — for a
///   routed run, `--target-phase 3`, which is literally what the user typed.
///
/// Digesting the marker instead would collapse **every routed run against every
/// target** to one digest, destroying the only thing
/// [`journal::argv_digest`](crate::journal::argv_digest) promises: telling two
/// runs with different command lines apart. The digest authenticates nothing;
/// it discriminates, and a constant discriminates nothing.
fn digested_command_fragment(args: &DriveArgs) -> String {
    match (&args.command, &args.target_phase) {
        (Some(command), _) => command.clone(),
        (None, Some(target_phase)) => format!("--target-phase {target_phase}"),
        (None, None) => String::new(),
    }
}

/// The process group this process is **actually** in, straight from the kernel.
///
/// `getpgrp()`, which cannot fail: every process is in exactly one group.
///
/// **Split out of [`establish_own_group`] rather than inlined into it, and the
/// reason is testability without collateral damage.** A test that wanted to
/// cross-check the syscall against `liveness::process_group`'s `/proc` parse
/// could not call `establish_own_group`, because `setpgid` in a shared test
/// binary would move **the harness's own process group** — every other test in
/// the same process, and the runner's job control with them. This function
/// observes and changes nothing, so the cross-check costs nothing.
fn current_group() -> u32 {
    rustix::process::getpgrp().as_raw_nonzero().get() as u32
}

/// Become this process's own group leader, and report the group it ends up in.
///
/// `setpgid(0, 0)`. Under the TUI's spawn — which already applies
/// `process_group(0)` — this is a harmless no-op. It exists for the other launch
/// mode: a **hand-typed** `gsd-meta-manager drive`, which would otherwise
/// inherit the shell's job group and write a `pgid` naming a group it does not
/// lead. A later teardown signalling that pgid would signal the user's shell job
/// instead of the run (T-17-03).
///
/// An `EPERM` — the process is already a session leader — is downgraded to a
/// warning carrying the error **kind** only. It is not a reason to refuse a run.
///
/// **It returns [`current_group`] rather than nothing, and that return value is
/// WR-01.** The previous doc said this call *"is what makes the equality honest
/// rather than assumed"*, and that was true of the call and false of the record:
/// `make_run_record` wrote `std::process::id()` into `pgid` unconditionally, so
/// on the very failure path the `warn!` above describes the record still claimed
/// group leadership. Returning the group the kernel reports is what closes the
/// gap between the warning and the document — the record now says what happened,
/// including when what happened was not what was asked for.
fn establish_own_group() -> u32 {
    if let Err(err) = rustix::process::setpgid(None, None) {
        tracing::warn!(
            kind = ?err.kind(),
            "could not become process group leader; the recorded pgid may name an inherited group",
        );
    }
    current_group()
}

/// **Layers 2 and 3 of D-06's four-step stop**, in the order that makes them
/// work.
///
/// The stop the user presses is two-layer because there are **two process
/// groups, not one**: Phase 15 spawns `claude` with `ProcessGroup::leader()`, so
/// its pgid is distinct from this driver's and the TUI's
/// `kill(-driver_pgid, SIGTERM)` does not reach it. Layer 1 (the signal to this
/// process's group) and layer 4 (the grace, the escalation and the reap) belong
/// to [`crate::driver::kill`]; the two below belong here:
///
/// 1. **Layer 2 — `Executor::cancel`, and that single call is the whole of it.**
///    It is Phase 15's already-built sequence at `src/executor/claude.rs:1482-1549`:
///    SIGTERM to the `claude` process **group**, a ten-second grace, SIGKILL,
///    then an `wait()` that is deliberately unbounded and deliberately raced
///    against nothing, because that final wait is what stops the grandchildren
///    becoming zombies.
///
///    **A second teardown must not be written here, and the reason is specific
///    rather than stylistic** (D-06.2). `process-wrap`'s `start_kill()` and its
///    `kill()` convenience both send the **uncatchable** signal; `signal(15)` is
///    the only graceful path, and reading `kill()` as "terminate politely" is
///    natural and wrong. Re-implementing that distinction here is exactly how
///    the CLI's documented clean shutdown — the turn abort, the Bash-tree
///    teardown through its own handler, the `SessionEnd` hooks — gets silently
///    skipped, which is the failure mode the whole two-layer design exists to
///    prevent.
///
/// 2. **The reason, journaled before the ending.** A `Diagnostic` carrying
///    [`TERMINATE_DIAGNOSTIC_CODE`], written first so the *why* survives even if
///    the terminal write is the one that fails.
///
/// 3. **Layer 3 — the terminal record.** This is the write that makes a stopped
///    run distinguishable from a crashed one on disk: a crashed run has no
///    `ended_at` (that absence *is* Phase 16's crash contract), a stopped one has
///    `ended_at` plus the killed outcome. `src/driver/reconcile.rs` reads exactly
///    that difference, so skipping it would make every stop look like a crash.
///
/// The outcome label comes from the outcome `cancel` actually returned rather
/// than from a hard-coded `Killed`: the coordinator maps a cancelled run onto
/// [`RunOutcome::Killed`] at `src/executor/claude.rs:1223`, so this reports
/// `killed` by construction, and in the rare case a wall-clock or idle breach
/// was classified in the same pass it reports the breach truthfully instead of
/// overwriting it with a label that is merely expected.
///
/// The lock is **not** released here. Releasing it is dropping [`DriverRun`],
/// which happens after this returns, so the release lands after the last write
/// rather than in the middle of it (D-20.2). An explicit unlock would be a second
/// release path for one resource.
async fn shutdown_on_terminate(
    executor: &ClaudeExecutor,
    handle: &mut ExecutionHandle,
    journal: &mut JournalRun,
) {
    let outcome = executor.cancel(handle).await;

    if let Err(err) = journal.record(&JournalEvent::Diagnostic {
        code: TERMINATE_DIAGNOSTIC_CODE.to_string(),
        detail: "the driver received the terminate signal and tore down its agent process group"
            .to_string(),
    }) {
        // The error KIND only, never a message body (T-17-05). A journal that
        // cannot take the diagnostic must still be given the chance to take the
        // terminal record, which is the more important of the two.
        tracing::warn!(
            kind = ?err.kind(),
            "could not journal the terminate-signal diagnostic",
        );
    }

    // **No blocking read inside an `async fn`** (D-28, WR-10). `terminal_label`
    // reads the whole journal, so the read goes into a blocking task and only
    // the resulting `String` comes back.
    //
    // The inline re-run on a join failure is `driver::drive`'s dry-run arm's own
    // answer to the same question, and this repository's established one: it
    // keeps the terminal record honest on a path no healthy run reaches, at the
    // cost of a blocking call in a process that is already ending anyway. The
    // record must be written either way — a run that ends with no record at all
    // is the one failure OBS-01 cannot tolerate.
    let journal_path = journal.paths().journal.clone();
    let task_outcome = outcome.clone();
    let task_path = journal_path.clone();
    let label = match tokio::task::spawn_blocking(move || {
        terminal_label(&task_outcome, &task_path)
    })
    .await
    {
        Ok(label) => label,
        Err(err) => {
            tracing::warn!(
                panicked = err.is_panic(),
                "the terminal-label task did not run to completion",
            );
            terminal_label(&outcome, &journal_path)
        }
    };

    if let Err(err) = journal.finish(&label) {
        tracing::warn!(
            detail = %format!("{err:#}"),
            "could not close the journal after a terminate-signal shutdown",
        );
    }
}

/// Whether the agent process has left, counting a **zombie as gone**.
///
/// The `'Z'` arm is the load-bearing half. This driver is the agent's parent and
/// is about to exit without reaping it — the whole point of
/// [`shutdown_during_startup`] is that the process leaves immediately afterwards
/// — so a zombie here is an *exited* process waiting for init to adopt and reap
/// it. Treating it as still-running would burn the remaining grace waiting for
/// something that has already happened, and on the escalation path that wait is
/// subtracted directly from the budget the TUI is counting down.
fn agent_has_exited(pid: u32) -> bool {
    matches!(liveness::process_state(pid), None | Some('Z'))
}

/// Poll until `pid` has left, giving up after `limit`. Reports whether it did.
async fn agent_gone_within(pid: u32, limit: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if agent_has_exited(pid) {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(STARTUP_POLL_INTERVAL).await;
    }
}

/// The stop that arrives while the agent is still starting (CR-01).
///
/// **The window this closes, precisely.** `tokio::signal::unix::signal` replaces
/// SIGTERM's default disposition the moment it is constructed, at the very top of
/// [`execute_run`] — so from that instant the driver survives a terminate signal.
/// Until plan 17-08 nothing polled `term.recv()` until the drain loop two hundred
/// lines later, which meant a stop issued while `Executor::start` was in flight
/// was **swallowed**: the driver ignored the TUI's SIGTERM, the TUI escalated to
/// SIGKILL after its twelve-second grace, and the `claude` group — a *different*
/// process group that had never been signalled — was orphaned with its
/// grandchildren. The documented `SessionStart` hook hang
/// (`src/executor/mod.rs:225-239`) makes that window minutes rather than
/// microseconds.
///
/// **Why this tears the group down itself rather than trusting the Coordinator.**
/// Dropping the `start` future does drop `cancel_tx`, and the Coordinator's
/// cancellation arm does tear the agent group down. But that teardown is
/// unobservable from here — there is no handle, no outcome and no channel back —
/// and it dies with the tokio runtime the instant this process exits, which is
/// the next thing that happens. The driver must not return until it has *seen*
/// the group go, so it signals the group itself and waits.
///
/// It reaches the group through [`kill::signal_group`] and never through a direct
/// `rustix` call, so the "process group 0 is the caller's own group" refusal
/// exists in exactly one place in the tree (D-08).
async fn shutdown_during_startup(pgid_rx: &mut oneshot::Receiver<u32>, journal: &mut JournalRun) {
    // A non-blocking read, and it has to be: by the time this body runs
    // `tokio::select!` has already dropped the `start` future, so nothing further
    // will ever be sent on this channel and an `.await` here would hang until the
    // sender dropped. `Ok` means the agent child exists and its group is known;
    // any `Err` means the spawn had not reached `child.id()` yet, so there is no
    // agent and nothing to tear down.
    let agent_pgid = pgid_rx.try_recv().ok();

    if let Some(agent_pgid) = agent_pgid {
        if let Err(err) = kill::signal_group(agent_pgid, Signal::TERM) {
            // The error KIND only, never a message body (T-17-05).
            tracing::warn!(
                kind = ?err.kind(),
                "could not send the terminate signal to the agent process group during startup",
            );
        }

        if !agent_gone_within(agent_pgid, STARTUP_AGENT_GRACE).await {
            tracing::warn!(
                agent_pgid,
                "the agent group outlasted its startup grace; escalating to the \
                 uncatchable signal",
            );
            if let Err(err) = kill::signal_group(agent_pgid, Signal::KILL) {
                tracing::warn!(
                    kind = ?err.kind(),
                    "could not send the uncatchable signal to the agent process group",
                );
            }
            let _ = agent_gone_within(agent_pgid, STARTUP_REAP_BOUND).await;
        }
    }

    // The same diagnostic CODE the drain-loop path writes, deliberately: it is
    // what a later reader greps for, and a second code for "stopped, but earlier"
    // would split one question across two searches. The *detail* carries the
    // difference, including whether there was an agent group at all.
    if let Err(err) = journal.record(&JournalEvent::Diagnostic {
        code: TERMINATE_DIAGNOSTIC_CODE.to_string(),
        detail: match agent_pgid {
            Some(_) => "the driver received the terminate signal while the agent was still \
                        starting, and tore down the agent process group"
                .to_string(),
            None => "the driver received the terminate signal before the agent process \
                     existed, so there was no agent process group to tear down"
                .to_string(),
        },
    }) {
        tracing::warn!(
            kind = ?err.kind(),
            "could not journal the terminate-signal diagnostic",
        );
    }

    // Hard-coded, unlike `shutdown_on_terminate`, and the difference is not an
    // inconsistency. That path has an `ExecutionHandle` and therefore a
    // `RunOutcome` to derive a label from — including the rare case where a
    // wall-clock or idle breach was classified in the same pass. Here there is no
    // handle and no outcome: the run was stopped before one could exist, and
    // "killed" is the only truthful thing to write.
    if let Err(err) = journal.finish("killed") {
        tracing::warn!(
            detail = %format!("{err:#}"),
            "could not close the journal after a startup terminate-signal shutdown",
        );
    }
}

/// Read every complete inbox line after `cursor`, off the async worker.
///
/// **The read runs on `tokio::task::spawn_blocking` and never inline** (D-28,
/// WR-10). It is synchronous filesystem work, and this repository has *observed*
/// what a blocking syscall inside an `async fn` costs: `tests/driver_lock.rs`
/// records a blocking `flock` defeating `tokio::time::timeout` outright on a
/// current-thread runtime. A blocking read on the driver's one poll thread would
/// stall the terminate arm — the arm whose whole job is to be reachable.
///
/// An I/O failure is a warning and an empty batch, never a run-ending error: an
/// unreadable inbox costs the user their steering, and killing the run over it
/// would cost them the run as well. Every log line carries the error **kind**
/// only — never a path and never a message body, because a body is text the user
/// typed (T-18-03, PATTERNS §S3).
async fn read_inbox(inbox_path: &Path, cursor: &mut TailCursor) -> Vec<InboxMessage> {
    let path = inbox_path.to_path_buf();
    let start = *cursor;

    let read = match tokio::task::spawn_blocking(move || inbox::tail(&path, start)).await {
        Ok(Ok(read)) => read,
        Ok(Err(err)) => {
            tracing::warn!(kind = ?err.kind(), "inbox tail failed");
            return Vec::new();
        }
        Err(_) => {
            tracing::warn!("the inbox tail task did not run to completion");
            return Vec::new();
        }
    };

    *cursor = read.cursor;

    // Both diagnostic flags are surfaced rather than swallowed, following
    // `App::schedule_journal_tail`. Under this design's own invariants an inbox
    // is never truncated in place, so `restarted` can only mean an invariant
    // broke — and its consequence is re-delivery of messages already sent.
    if read.restarted {
        tracing::warn!(
            restarted = true,
            "inbox tail: the file shrank and the cursor was reset",
        );
    }
    if read.skipped_oversize {
        tracing::warn!(
            skipped_oversize = true,
            "inbox tail: stepped over a line that exceeded the read bound",
        );
    }
    if read.unparseable > 0 {
        // A count, never the line (D-28).
        tracing::warn!(
            count = read.unparseable,
            "inbox tail: complete lines did not parse as messages",
        );
    }

    read.messages
}

/// Deliver every queued message to the agent's stdin, journalling each attempt.
///
/// Returns how many were **delivered**, not how many were drained, and the
/// difference is load-bearing: the caller uses the answer to decide whether to
/// expect another turn. A message that was drained but whose write failed
/// produces no new turn, so counting it would park the run waiting for a
/// `result` that is never coming.
///
/// `delivered` on the journal record is exactly *"`Executor::send` returned
/// `Ok`"* and is never rendered as an acknowledgement from the agent (D-07);
/// that transition is `interjection_acted_on`, measured 55 seconds later.
///
/// **There is no turn-boundary flush buffer here, and there must never be one**
/// (D-02). ARCHITECTURE's AP3 — buffer a mid-turn message and flush it at the
/// `result` boundary — was *refuted* empirically against CLI 2.1.220 by Phase
/// 15's spike: a message written mid-turn is queued by the CLI and executed as
/// its own turn, so a driver-side buffer would duplicate the CLI's own queue and
/// make queued-message accounting incoherent.
/// `tests/executor_transport.rs::a_message_sent_mid_turn_is_not_buffered_by_the_driver`
/// is the regression guard that fails anyone who adds one.
async fn deliver_pending_inbox(
    executor: &ClaudeExecutor,
    handle: &mut ExecutionHandle,
    journal: &mut JournalRun,
    inbox_path: &Path,
    cursor: &mut TailCursor,
    pending: &mut PendingAcks,
) -> usize {
    let mut delivered_count = 0usize;

    for message in read_inbox(inbox_path, cursor).await {
        let delivered = match executor
            .send(handle, UserMessage::text(message.text.clone()))
            .await
        {
            Ok(()) => {
                delivered_count += 1;
                // The `{id → text}` D-08 requires, recorded at the only moment
                // it is true: the write returned `Ok`, so an echo of this text
                // can now legitimately arrive. Pushing before the write would
                // let a failed send shadow a later identical message.
                pending.push_delivered(message.id.clone(), message.text.clone());
                true
            }
            Err(err) => {
                // The error KIND only, never the message body (T-18-03).
                tracing::warn!(
                    kind = ?err,
                    "could not write an injected message to the agent's stdin",
                );
                false
            }
        };

        if let Err(err) = journal.record(&JournalEvent::Interjected {
            id: Some(message.id.clone()),
            text: message.text,
            delivered,
        }) {
            tracing::warn!(kind = ?err.kind(), "journal write failed");
        }

        // **A failed write is terminal HERE, where the fact is known** (CR-03,
        // D-10). The message was consumed from the tail, so `cursor` has already
        // moved past it: it can never be read again, never retried, and never
        // reaches `sweep_inbox_as_missed`, which only sees messages the cursor
        // has not passed. Without this record `interjected { delivered: false }`
        // is the last word ever written about it, and the four-state display
        // left it in `queued` — *"durably on disk; nothing has read it yet"* —
        // forever, which is false twice over. That is exactly the undelivered
        // injection D-10 exists to abolish, in the code written to abolish it.
        //
        // It is **not** a retry and never becomes one: stdin cannot be
        // reopened, and a `WriterGone` or `NotRunning` send error means there is
        // nothing left to write to.
        if !delivered {
            journal_one_as_missed(journal, &message.id, MISSED_SEND_FAILED);
        }
    }

    delivered_count
}

/// Journal every remaining inbox message as undeliverable (D-10).
///
/// **The honest fourth state.** Once the agent's stdin is closed it can never be
/// reopened, so a message that arrives afterwards has nowhere to go. Leaving it
/// in `queued` forever is PITFALLS' undelivered-injection failure dressed up as
/// a spinner, and it is the one thing the user's own steering intent must never
/// suffer: every message reaches a named terminal state the user can see.
///
/// It is reached from **two** places and classifies in **one**: the drain
/// loop's inbox poll once stdin has closed, and a final pass after the event
/// stream has ended. The two are the same question asked at two moments — "can
/// this still be delivered?" — and the answer is `no` from `close_input()`
/// onwards. Keeping the classification in [`journal_as_missed`] rather than at
/// each branch is what stops the two answers drifting apart.
///
/// It is deliberately **not** a retry: stdin cannot be reopened.
async fn sweep_inbox_as_missed(
    journal: &mut JournalRun,
    inbox_path: &Path,
    cursor: &mut TailCursor,
) -> usize {
    journal_as_missed(journal, read_inbox(inbox_path, cursor).await)
}

/// Write the terminal `missed` record for each of `messages`, all of them for
/// the same reason: they arrived after the close.
///
/// One message, one record, no retry. Each message arrives here exactly once
/// because the cursor has already advanced past it.
///
/// The `reason` is [`MISSED_AFTER_CLOSE`], a fixed machine-readable string; the
/// human-readable gloss belongs to the render layer, which must not have to
/// parse prose written here.
fn journal_as_missed(journal: &mut JournalRun, messages: Vec<InboxMessage>) -> usize {
    let count = messages.len();

    for message in messages {
        journal_one_as_missed(journal, &message.id, MISSED_AFTER_CLOSE);
    }

    count
}

/// Write the terminal `missed` record for one id. **The only emission site.**
///
/// Two callers with two reasons — [`journal_as_missed`] for a message that
/// arrived after the close, and [`deliver_pending_inbox`] for one whose stdin
/// write failed — and the record's shape is decided here so the two cannot
/// drift. Both reasons are terminal and neither is ever retried.
///
/// **`interjected` and `interjection_missed` are no longer mutually exclusive
/// for one id, and that is deliberate** (CR-03). A failed write produces both:
/// the `interjected { delivered: false }` record is the honest account of the
/// attempt — the driver read the message and tried — and the `missed` record is
/// the honest account of its fate. Suppressing the first would hide that the
/// driver ever saw the message; omitting the second is what left it rendering
/// `queued` forever. They remain mutually exclusive for the *after-close* path,
/// where the message was never read from the tail at all.
///
/// A journal write failure here is a warning and nothing more, following every
/// other `journal.record` call in this file: an unwritable journal costs the
/// user their record of the message, and ending the run over it would cost them
/// the run as well. The render layer's own refusal to leave a
/// `delivered: false` record in `queued` is the second, independent guard for
/// exactly this case.
fn journal_one_as_missed(journal: &mut JournalRun, id: &str, reason: &str) {
    if let Err(err) = journal.record(&JournalEvent::InterjectionMissed {
        id: id.to_string(),
        reason: reason.to_string(),
    }) {
        tracing::warn!(kind = ?err.kind(), "journal write failed");
    }
}

/// The program this run will exec.
///
/// Two bodies behind one name (D-30, WR-16). In a debug build the override is
/// honoured; in release the field does not exist, so the answer is a constant
/// and there is no branch a caller could reach. Written as a pair of `#[cfg]`
/// functions rather than as an `if cfg!(…)` because `cfg!` compiles **both**
/// arms — the released binary would still contain the code path that execs an
/// arbitrary program, merely with nothing able to select it, and "unreachable
/// today" is a fact about today.
#[cfg(debug_assertions)]
fn agent_program(args: &DriveArgs) -> PathBuf {
    args.claude_program
        .clone()
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_PROGRAM))
}

#[cfg(not(debug_assertions))]
fn agent_program(_args: &DriveArgs) -> PathBuf {
    PathBuf::from(DEFAULT_AGENT_PROGRAM)
}

/// The leading arguments placed before the executor's own generated argv.
///
/// Debug builds only, for the same reason as [`agent_program`]: they are the
/// payload half of one override, and a release build has none.
#[cfg(debug_assertions)]
fn agent_leading_args(args: &DriveArgs) -> Vec<String> {
    args.claude_args
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect()
}

#[cfg(not(debug_assertions))]
fn agent_leading_args(_args: &DriveArgs) -> Vec<String> {
    Vec::new()
}

/// The one line of `detail` that accompanies [`AGENT_PROGRAM_OVERRIDDEN`].
///
/// **The file name only — never the full path, and never the arguments.** The
/// path would disclose where a developer's tree lives, and the arguments are
/// caller-supplied free text that can carry a token or a transcript path; the
/// journal is written into the *driven* project's `.planning/`, which is a
/// directory a user may well commit. Naming the stand-in is the whole job: a
/// reader needs to know this run was not the agent, not to be able to reproduce
/// it. This is the same error-kind-only discipline every driver log line in this
/// module already follows (T-18-45).
///
/// A path ending in `..` or `/` has no file name; it degrades to a fixed word
/// rather than falling back to the full path, because the fallback is the thing
/// being avoided.
#[cfg(debug_assertions)]
fn agent_override_detail(program: &Path) -> String {
    let name = program
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "an unnamed program".to_string());
    format!(
        "this run execs the stand-in `{name}` instead of the agent, so it is not a real agent run"
    )
}

/// Mark an overridden run on disk, **before** the run's first exec record.
///
/// Position is the point (D-30). A reader scanning `journal.jsonl` top to bottom
/// meets the marker before `exec_started` — before anything it could otherwise
/// be mistaken for. A record written at the end would be a footnote on a
/// transcript that already read as a real run.
///
/// A failed write is warned about and swallowed, exactly as the terminate
/// diagnostic's is: a journal that cannot take this record must still be given
/// the chance to take the run's terminal one, which is the more important of the
/// two.
#[cfg(debug_assertions)]
fn journal_agent_program_override(journal: &mut JournalRun, args: &DriveArgs) {
    let Some(program) = args.claude_program.as_ref() else {
        return;
    };

    if let Err(err) = journal.record(&JournalEvent::Diagnostic {
        code: AGENT_PROGRAM_OVERRIDDEN.to_string(),
        detail: agent_override_detail(program),
    }) {
        // The error KIND only, never a message body (T-17-05).
        tracing::warn!(
            kind = ?err.kind(),
            "could not journal the agent-program override diagnostic",
        );
    }
}

/// The executor for **one** iteration.
///
/// **Constructed per iteration rather than once per run, and the reason is the
/// spawn observer.** `observing_spawn` takes a `oneshot::Sender` that is
/// *consumed* by the first spawn, so an executor reused across iterations would
/// publish the agent's process group for the first command and for no other —
/// and that channel is precisely what a stop landing during startup uses to
/// reach a group the driver otherwise has no handle on (CR-01). A second
/// iteration whose startup stop found an exhausted channel would orphan the
/// agent group it never signalled, which is the failure `driver::kill`'s whole
/// two-layer design exists to prevent.
///
/// The replay-echo sender is **cloned** rather than moved, so the run-level
/// channel outlives every iteration and `echo_open` keeps meaning what its own
/// comment says it means.
///
/// A pair of `#[cfg]` functions rather than an `if cfg!(…)`, for exactly the
/// reason [`agent_program`] gives: `cfg!` compiles both arms, so the released
/// binary would still contain the path that execs an arbitrary program.
#[cfg(debug_assertions)]
fn build_executor(args: &DriveArgs) -> ClaudeExecutor {
    match &args.claude_program {
        Some(program) => ClaudeExecutor::with_program(program, args.claude_args.clone()),
        None => ClaudeExecutor::new(),
    }
}

#[cfg(not(debug_assertions))]
fn build_executor(_args: &DriveArgs) -> ClaudeExecutor {
    ClaudeExecutor::new()
}

/// The terminal label for a routed run whose declared target was met.
///
/// **The one label in this module sourced from neither [`outcome_label`] nor the
/// [`PARKED_LABEL_PREFIX`] carrier, and it exists because both are wrong here.**
/// `outcome_label` maps a [`RunOutcome`] — and the outcome of the *previous*
/// iteration describes that command, not the run's ending; `parked:` would claim
/// the run stopped needing a human when in fact it stopped because there was
/// nothing left to do.
///
/// **It is written whenever [`Terminal::GoalMet`] is reached, whether or not any
/// iteration spawned** (WR-08). Keyed on the presence of a `RunOutcome` instead,
/// it named only the case where the target was already met at launch, and a run
/// that actually drove its target to `passed` ended `succeeded_with_changes` —
/// leaving a reader of `run.json` to infer goal-met from the absence of a
/// `parked:` prefix.
///
/// **Reachable since 20-04.** It was unreachable when 20-01 introduced it —
/// `router::Decision::GoalMet` had no producer until the one reader recorded the
/// VERIFICATION frontmatter `status` (research Pitfall 2). 20-03 added that
/// status and 20-04 added the producer: [`router::is_goal_met`], which is
/// `verification_status.is_passed()` on the run's `--target-phase` (CONTEXT.md
/// OQ4). The constant existed so the plan adding that producer would find a
/// decision here instead of inheriting a guess; it did.
pub(crate) const GOAL_MET_LABEL: &str = "goal_met";

/// The options for **one** iteration of a run.
///
/// Rebuilt per iteration, and three of its properties are decisions:
///
/// * **A fresh `session_id` every time, and never `resume_session`** (CONTEXT.md
///   OQ5). A resumed session accumulates the previous command's whole transcript
///   into the next command's window, which is how a multi-hour run hits a
///   context limit for reasons unrelated to the work. It also keeps `run.json`'s
///   single `session_id` honest: per-iteration ids belong on the journal's
///   `exec_started` records, which already carry one. `ExecutionOptions::default`
///   generates a fresh v4 UUID, so this is what *not* overriding it buys.
/// * **[`bounds::iteration_wall_clock_cap`] rather than the default or the bare
///   [`bounds::ITERATION_WALL_CLOCK_CAP`] ceiling.** The caller passes what is
///   left of the *run's* budget and the helper takes the smaller of the two, so
///   an iteration can never outlive the run's own cap. Passing the ceiling
///   unconditionally is what made that cap not a bound on the run (CR-01):
///   `bounds::evaluate` runs only between iterations, so nothing at all bounds a
///   *running* iteration except the value handed to the executor here, and a run
///   asked for sixty seconds spawned an agent allowed three hours. The ceiling
///   still applies on top, and is still strictly less than the default run-level
///   cap so that CTRL-06's run-level reason stays reportable (research
///   Pitfall 3).
/// * The envelope's settings path and environment are **cloned**, because
///   establishment happens once per run and every iteration is protected by that
///   same envelope.
fn iteration_options(
    envelope_settings: &Path,
    envelope_env: &EnvelopeEnv,
    run_bounds: &bounds::RunBounds,
    elapsed: Duration,
) -> ExecutionOptions {
    ExecutionOptions {
        envelope_disallowed_tools: policy::disallowed_tools(),
        envelope_settings: Some(envelope_settings.to_path_buf()),
        envelope_env: Some(envelope_env.clone()),
        wall_clock_cap: bounds::iteration_wall_clock_cap(run_bounds, elapsed),
        ..Default::default()
    }
}

/// The five D-R-P-E-V stage statuses for `target_phase`, in order.
///
/// `JournalEvent::Observed.drpev` is documented as *"the five D-R-P-E-V stage
/// statuses, in order"* — a `Vec<String>` of length five, not a free-form list —
/// so this function is the one producer and the length is a property of it
/// rather than of each call site.
///
/// The five stages map onto GSD's own artifact presence: Discuss is
/// `has_context`, Research is `has_research`, Plan is the plan count, Execute is
/// the summary count, Verify is `has_verification`.
///
/// **Verify reports presence, and since 20-03 that is a choice rather than a
/// limit.** `DiskInference::verification_status` now carries the frontmatter
/// `status` — the reader gap research Pitfall 2 named was closed, and the two
/// doc comments here that still described it as open were stale (IN-01). This
/// element stays a presence flag because the five are one positional vocabulary:
/// four artifact-presence facts and a count pair, read by index. A status word in
/// slot five would make that slot mean something different from its neighbours,
/// and the status is not lost — it is what decides the DRIVE-05 gate, so it
/// reaches the journal through the `parked` record's reason and its `Diagnostic`
/// detail, which is where a reader greps for *why a run stopped* rather than
/// *what its stages looked like*.
///
/// Values are enum names and counts — never artifact content — so this record
/// cannot carry agent-authored text (T-20-05).
fn drpev_stages(state: &crate::state_reader::ProjectState, target_phase: &str) -> Vec<String> {
    let inference = state.phase_disk_statuses.get(target_phase);
    let flag = |present: bool| if present { "yes" } else { "no" }.to_string();
    vec![
        flag(inference.is_some_and(|found| found.has_context)),
        flag(inference.is_some_and(|found| found.has_research)),
        inference.map_or(0, |found| found.plan_count).to_string(),
        inference.map_or(0, |found| found.summary_count).to_string(),
        flag(inference.is_some_and(|found| found.has_verification)),
    ]
}

/// Capture a project snapshot off the async path, with the inline fallback this
/// module already uses for a join failure.
///
/// **`RunSnapshot::capture` does full-tree file I/O and shells out to git
/// twice**, and it says so in its own doc — so it goes on a blocking thread
/// (D-28, WR-10). The thread it must not park is the one polling the terminate
/// arm: a driver that stops responding to the kill switch during a long observe
/// is a driver the user experiences as ignoring the stop.
///
/// A join failure re-runs it inline rather than yielding a default snapshot,
/// which is the same answer `ClaudeExecutor::capture_snapshot` and the
/// terminal-label task below already give. A *default* snapshot would compare
/// unequal to everything and quietly reset the no-progress evidence, which is a
/// stall detector silently switched off on the one path no healthy run reaches.
async fn capture_snapshot(project_root: &Path) -> crate::executor::outcome::RunSnapshot {
    let owned = project_root.to_path_buf();
    let for_task = owned.clone();
    match tokio::task::spawn_blocking(move || {
        crate::executor::outcome::RunSnapshot::capture(&for_task)
    })
    .await
    {
        Ok(snapshot) => snapshot,
        Err(err) => {
            tracing::warn!(
                panicked = err.is_panic(),
                "the project snapshot task did not run to completion",
            );
            crate::executor::outcome::RunSnapshot::capture(&owned)
        }
    }
}

/// Run one GSD command to completion and leave a complete run directory.
///
/// In order, and the order is the decision:
///
/// 1. Become our own process group leader, so D-04's `pid == pgid` invariant is
///    established rather than assumed.
/// 2. Take the single-execution lock, **after** the opt-in gate (which ran in
///    [`crate::driver::drive`]) and **before** the journal. Both boundaries are
///    the decision: a project the user never opted in must not get a lock file
///    in its tree (T-17-14), and a *losing* run must prune nothing, create no
///    run directory, write no `run.json` and clear nobody's `active` pointer —
///    D-12's evidence-preservation argument applies to a loser with exactly as
///    much force as to a crash.
/// 3. Start the journal — `run.json` write one of two, plus `run_started`.
/// 4. Spawn the agent. A spawn failure still calls `finish`, so a run that
///    started always has a terminal record (T-17-06).
/// 5. Journal the `claude` process group id **before draining a single event**:
///    a teardown handle recorded late is a teardown handle that can be missed.
/// 6. Drain the event stream into the journal, racing the terminate signal
///    **first** — see the `biased` `select!` below.
/// 7. Finish the journal with the derived outcome — `run.json` write two.
///
/// **The terminate signal is raced against every await between the handler's
/// installation and step 7 — not only the drain loop.** This doc used to say the
/// signal was raced "first", which was true of the drain loop and false of
/// everything before it, and CR-01 is what that cost. The awaits in the window
/// were `lock::acquire`, `JournalRun::start`, `executor.start` and
/// `close_input`; the last two are long — `start` blocks on the capability gate,
/// which the documented hook hang can park for minutes — and each is now the
/// second arm of a `biased` `select!` whose first arm is `term.recv()`. The
/// synchronous steps in that window need no arm of their own, because tokio
/// **buffers** a signal delivered before the first `recv()`: whichever poll comes
/// first acts on it. `establish_own_group` and the two shorter awaits are covered
/// by that buffering, which is why the driver responds to a stop issued before it
/// has finished starting rather than after.
///
/// **The two shorter ones are now genuine await points rather than synchronous
/// calls, and that is D-28 / WR-10.** `lock::acquire` and `JournalRun::start`
/// each run on `tokio::task::spawn_blocking`, so the syscalls inside them can no
/// longer park the thread the terminate handler is waiting on. They still need no
/// `select!` arm — the buffering above covers them exactly as it covered their
/// synchronous predecessors — but the buffering only helps a thread that is still
/// able to poll, which is the property the wrap restores.
///
/// The lock guard lives on [`DriverRun`], which outlives the terminal
/// `finish` call, so the lock is released **after** the last write rather than
/// somewhere in the middle of it. It is also moved back **out** of the blocking
/// task rather than dropped inside it; the call site below says why at length,
/// and the short version is that dropping a `RunLock` releases the lock.
///
/// The `entry` parameter carries the opt-in record whose timestamp lands in
/// `RunRecord.opt_in`; the [`DrivableProject`] token proves the gate ran, but by
/// construction it carries only the alias and the root.
pub async fn execute_run(
    project: DrivableProject,
    args: &DriveArgs,
    entry: &RegisteredProject,
) -> Result<(), DriveError> {
    // Installed FIRST — before the group is established, before the lock, and
    // before a single byte lands on disk.
    //
    // **A driver that cannot observe the terminate signal is a driver that
    // cannot be stopped**, because layer 2 of D-06 is precisely this handler
    // calling `Executor::cancel`. Without it, the TUI's SIGTERM would reach the
    // driver's default disposition, the process would die *immediately*, and the
    // `claude` group — which is a different process group and never received
    // anything — would be orphaned along with its Bash grandchildren. That is
    // the exact failure CTRL-01 exists to prevent, so this refuses rather than
    // warning and continuing, and refusing here costs nothing: nothing has been
    // created yet, so there is no half-run to clean up.
    //
    // The refusal is `UnsupportedPlatform` rather than a new `DriveError`
    // variant. `tokio::signal::unix::signal` fails only for signals the kernel
    // does not let a process catch, so a build where this errors on SIGTERM is a
    // platform that cannot support driving at all — which is what that variant
    // already says (D-05). A variant of its own would widen an enum three later
    // plans in this phase also match on, to describe a branch no supported
    // platform reaches.
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|err| DriveError::UnsupportedPlatform {
            detail: format!(
                "the terminate-signal handler could not be installed ({}), so a stop \
                 request could not tear the agent's process group down",
                err.kind()
            ),
        })?;

    let pgid = establish_own_group();

    // The TUI owns the id so it knows what to look for; the driver owns the
    // record (D-03).
    //
    // A **second** line of defence, not the tested one: `driver::drive` refuses
    // an absent run id before anything is created, and that refusal is what
    // `drive_refuses_a_real_run_that_carries_no_run_id_without_touching_disk`
    // exercises. This one exists so a future direct caller of `execute_run` —
    // there is none today — cannot reintroduce a run that `liveness::probe`
    // cannot see. The driver used to GENERATE an id here when none was supplied,
    // which produced a run whose id appeared on no argv: invisible to the probe,
    // reported crashed by every scan, and un-stoppable because a stop answered
    // already-gone without signalling (CR-04).
    //
    // It is read **before** the envelope is established rather than after, so a
    // run that has no id costs no generated file: the cheaper refusal goes first.
    let run_id = args.run_id.clone().ok_or(DriveError::RunIdRequired)?;

    // **The envelope, established once, before the executor is constructed.**
    //
    // Everything the previous plans in this phase built is inert until it
    // reaches the child, and argv plus the environment are the only two carriers
    // that cross the spawn. This is where they are filled, and it is the single
    // production `ExecutionOptions` construction site precisely so there is one
    // place to look.
    //
    // **No blocking syscall inside an `async fn`** (D-28, WR-10), the same
    // discipline and the same provenance as the lock and journal wraps below.
    // See `establish_envelope`'s own doc for what is blocking in there.
    //
    // The alias and root are **cloned** into the closure rather than moved for
    // the reason `driver::drive`'s dry-run arm records at length: `project` is
    // the capability token and is still needed below, and rebuilding one would
    // mean a second `DrivableProject::from_registry` call site — the exact
    // uniqueness `tests/spawn_seam_guard.rs` exists to check.
    let envelope_alias = project.alias().to_string();
    let envelope_root = project.root().to_path_buf();
    let envelope_run_id = run_id.clone();
    let envelope = tokio::task::spawn_blocking(move || {
        establish_envelope(&envelope_alias, &envelope_root, &envelope_run_id)
    })
    .await
    .map_err(|_| DriveError::EnvelopeAssertionFailed {
        reason: ParkReason::EnvelopeAssertionFailed,
        detail: "the envelope establishment task did not run to completion".to_string(),
    })?
    .map_err(|err| DriveError::EnvelopeAssertionFailed {
        reason: ParkReason::EnvelopeAssertionFailed,
        // Redacted at the boundary, because the chain can carry a remote URL or
        // a home-directory path and this text reaches the operator's terminal.
        detail: crate::journal::redact::redact(&format!("{err:#}")),
    })?;

    let protection = envelope.protection.clone();
    // **The envelope's two carriers are cloned per iteration rather than moved
    // once**, because a routed run constructs a fresh `ExecutionOptions` for
    // every command it issues. Establishment itself stays exactly where it was —
    // once, above, before the lock — so the four generated files are written
    // once and the external-client probe runs once (research Pitfall 4). Only
    // the *description* of them is rebuilt.
    let envelope_settings = envelope.settings;
    let envelope_env = envelope.env;

    // The caps in force, resolved **before** the record is built rather than at
    // the loop below, so the record can name them. `bounds::resolve` is pure, so
    // the value threaded down to the loop is provably the same one on disk —
    // which is the property the field is claiming.
    //
    // **Resolved before `iteration_options` too, and that ordering is CR-01.**
    // The per-iteration executor cap is now derived from these caps rather than
    // from a constant, so the resolution has to precede the first construction
    // of the options rather than merely precede the record.
    let run_bounds =
        bounds::resolve(args.max_steps, args.wall_clock_cap_secs).map_err(DriveError::from)?;

    // Built here only for the two fields `run.json` reads off it — the target
    // and the session id. Nothing spawns with these options; every iteration
    // builds its own below, against the budget remaining at that moment.
    let options = iteration_options(
        &envelope_settings,
        &envelope_env,
        &run_bounds,
        Duration::ZERO,
    );

    // The executor's own generated argv is not reachable from here — the
    // builder is private to `src/executor/claude.rs` — so the digest covers the
    // driver's effective command line. That is enough for what the digest
    // promises: comparing two runs for "same command line". It authenticates
    // nothing (see `journal::argv_digest`).
    //
    // It is computed once, because `run.json` carries exactly one digest and is
    // written exactly twice. A routed run's per-iteration argv differs by the
    // command; that variation is visible on the journal's `decided` records,
    // which name each command in full.
    //
    // **`digested_command_fragment`, not `recorded_command`**: the record's
    // routed marker is a constant, and digesting a constant would give every
    // routed run against every target the same digest. The driver's real argv
    // carries `--target-phase N`, which is what tells two routed runs apart.
    let mut argv = vec![agent_program(args).display().to_string()];
    argv.extend(agent_leading_args(args));
    argv.push(digested_command_fragment(args));
    let argv_digest = journal::argv_digest(&argv);

    let record = make_run_record(run_id, args, entry, &options, argv_digest, pgid, run_bounds);

    let planning_dir = project.root().join(".planning");

    // The same observed group the record carries, and for the same reason
    // (WR-01): `setpgid(0, 0)` ran at entry, but its success is not something to
    // assume — the lock's holder record and `run.json` must name the same group,
    // or a stop resolved through one would refuse against the other. A second
    // `drive` against this project now refuses and names this run rather than
    // starting alongside it (CTRL-05).
    // **No blocking syscall inside an `async fn`** (D-28, WR-10), and **the
    // deadlock that discipline prevents was OBSERVED rather than theorised**:
    // `tests/driver_lock.rs:201-215` records a blocking `flock` inside an
    // `async fn` defeating `tokio::time::timeout` outright on a current-thread
    // runtime, because `Timeout::poll` polls its inner future inline and a
    // parked thread polls nothing at all. `lock::acquire` opens a file, calls
    // `flock`, may read the holder's record and writes its own — every one of
    // those is a synchronous syscall, and the arm this driver most needs to keep
    // reachable is the terminate arm.
    //
    // **The `RunLock` is moved back OUT of the task, and that is the load-bearing
    // half of this wrap.** `RunLock` holds the `File` whose descriptor *is* the
    // advisory lock and it deliberately has **no `Drop` impl**, so a `RunLock`
    // dropped inside the blocking closure would close the descriptor and release
    // the lock silently — while the run carried on believing it held it. A second
    // `drive` against the same project would then start alongside this one, which
    // is precisely the concurrency the lock exists to close (CTRL-05, D-20.2,
    // T-18-11). Returning the guard through the join handle is what keeps "held
    // for the run's duration" true.
    //
    // **The declined alternative was leaving the lock inside the task and
    // re-acquiring it afterwards.** That reintroduces a window in which the
    // project is unlocked — between the closure's return and the re-acquire —
    // which is the same race with a smaller name, and it would additionally make
    // a *losing* re-acquire a mid-run failure rather than a start-time refusal.
    let lock_planning = planning_dir.clone();
    let lock_run_id = record.run_id.clone();
    let lock = tokio::task::spawn_blocking(move || lock::acquire(&lock_planning, &lock_run_id, pgid))
        .await
        .map_err(|_| {
            DriveError::Lock(LockError::Unavailable {
                detail: "the lock acquisition task did not run to completion".to_string(),
            })
        })??;

    // **No blocking syscall inside an `async fn`** (D-28, WR-10), for the same
    // reason and with the same provenance. `JournalRun::start` prunes the runs
    // directory and writes `run.json` plus the first record synchronously, and a
    // prune walks however many retained runs are on disk.
    //
    // The `JournalRun` is moved back out for the same reason the `RunLock` is:
    // it owns the run's open files and its clock, and a journal dropped inside
    // the task would close the run directory the caller is about to write to.
    let journal_planning = planning_dir.clone();
    let journal = tokio::task::spawn_blocking(move || JournalRun::start(&journal_planning, record))
        .await
        .map_err(|_| DriveError::Journal {
            detail: "the journal start task did not run to completion".to_string(),
        })?
        .map_err(|err| DriveError::Journal {
            detail: format!("{err:#}"),
        })?;
    let mut run = DriverRun { journal, lock };

    // The envelope's honest account of itself, recorded at run start — the
    // earliest moment there is a journal to record it into (D-26, D-27).
    //
    // **One producer, two consumers.** `advisory::envelope_notice` is the same
    // function the dry-run preview renders, so the claim a user reads before a
    // run and the claim a later reader finds in the journal cannot disagree
    // about what was promised (T-19-42). The notice carries newlines; the
    // journal line does not, because the writer escapes them — the record stays
    // one NDJSON line, which is the property the reader depends on.
    //
    // A failure to record it is a warning rather than a refusal, and that
    // direction is deliberate: the notice is *evidence*, and evidence that
    // cannot be written must not take the run down with it. Every containment
    // layer is already established by this point.
    if let Err(err) = run.journal.record(&JournalEvent::Diagnostic {
        code: format!("{PROTECTION_DIAGNOSTIC_PREFIX}{}", protection.as_str()),
        detail: advisory::envelope_notice(&protection),
    }) {
        tracing::warn!(
            detail = %format!("{err:#}"),
            "could not journal the envelope notice",
        );
    }

    // D-30's second half, and its position is the whole of it: the journal is
    // open and nothing has been exec'd yet, so a run driven by a stand-in is
    // labelled on disk *before* the first record that could be mistaken for a
    // real agent's. Release builds have no override to mark.
    #[cfg(debug_assertions)]
    journal_agent_program_override(&mut run.journal, args);

    // The raw wire line of every `user` replay echo, which is the **only**
    // evidence the protocol offers that the agent has started on an injected
    // message (D-07, D-08). Unbounded on purpose: this channel must never be
    // able to park the executor's coordinator, which owns the caps, the cancel
    // and the teardown. Its depth is bounded by the number of user messages one
    // run sends, and the loop below drains it on every pass.
    //
    // **Per-RUN, and the sender is cloned into each iteration's executor.** The
    // receiver has to survive between commands, and holding the original sender
    // here for the whole run is what keeps `echo_open` below meaning what its
    // comment says: the channel cannot close while the run is live.
    let (echo_tx, mut echo_rx) = mpsc::unbounded_channel::<String>();

    // The inbox this run is steered through, and the driver's own cursor into it
    // (D-03, D-04).
    //
    // **Per-RUN, not per-iteration, and the cursor is why.** A cursor rebuilt
    // between commands would rewind to the head of `inbox.jsonl` and re-deliver
    // every message the previous iteration already handed the agent — the user's
    // words repeated to a fresh agent that has no idea it is a replay. One
    // inbox, one cursor, one run.
    let inbox_path = run.journal.paths().inbox.clone();
    let mut inbox_cursor = TailCursor::default();

    // The first tick fires immediately, which is what makes a message queued
    // *before* the driver existed arrive without waiting out a full interval.
    // `Delay` rather than the default burst behaviour: a poll the loop was too
    // busy to service is worth doing once, not N times in a row.
    let mut inbox_poll = tokio::time::interval(INBOX_POLL_INTERVAL);
    inbox_poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    // Whether the echo channel can still produce. It cannot close while the run
    // is live — `echo_tx` above outlives the loop — so this flag exists purely
    // so a future refactor that *does* drop it early cannot turn a closed
    // channel into a permanently ready arm spinning the poll thread.
    let mut echo_open = true;

    // Which of the two execution models this run is. `driver::drive` has already
    // refused both-or-neither, so the last arm is unreachable; it is spelled out
    // rather than `unwrap`ped because a detached driver that panicked here would
    // leave a run directory with no terminal record, which is the signal D-12
    // reserves for a genuine crash.
    let source = match (&args.command, &args.target_phase) {
        (Some(command), _) => CommandSource::Fixed(command.clone()),
        (None, Some(target_phase)) => CommandSource::Routed {
            target_phase: target_phase.clone(),
        },
        (None, None) => CommandSource::Fixed(String::new()),
    };

    // `run_bounds` is the value resolved above, before the record was built —
    // **not a second call**. It used to be resolved again here on the reasoning
    // that a pure function cannot disagree with itself, which was true but is no
    // longer sufficient: the same value now reaches disk in
    // `RunRecord::bounds`, is what every iteration's executor cap is derived
    // from, and "the caps on disk are the caps the loop enforced" is a property
    // to guarantee by construction rather than by re-deriving and trusting
    // purity.
    let mut bounds_state = bounds::BoundsState::default();

    // The run-level clock. `std::time::Instant` rather than `tokio`'s, so the
    // elapsed value handed to `bounds::evaluate` is the same monotonic quantity
    // the cap was resolved against and cannot be moved by a test runtime's
    // time-pausing.
    let run_started_at = std::time::Instant::now();

    // How the run ended, seeded with the arm that means "everything asked for
    // ran". Every `break` below either leaves it alone or replaces it, and the
    // type has no arm for "ended with nothing said" (DRIVE-06).
    let mut terminal = Terminal::Completed;

    // The last iteration's derived outcome. `None` only when no iteration ever
    // spawned, which a routed run reaches by parking or halting on its first
    // pass — the one case where there is no `RunOutcome` for `outcome_label` to
    // map, handled explicitly at the terminal write below.
    let mut last_outcome: Option<RunOutcome> = None;

    // **THE ITERATION LOOP.** Everything above is per-run and established
    // exactly once — the terminate handler, the process group, the envelope, the
    // lock and the journal — and everything below the loop is the run's single
    // terminal record. Only spawn, drain and outcome repeat (D-20.2, research
    // Pitfall 4). The lock is never re-acquired: `RunLock` has no `Drop` impl
    // and its descriptor *is* the lock, so a re-acquire would open the
    // concurrency window CTRL-05 exists to close.
    //
    // **The iteration tick is the previous iteration's `wait_outcome()`
    // returning — never a file watcher.** The driver writes into
    // `.planning/meta-manager/runs/…`, which sits under the tree a watcher would
    // watch, so a watcher-fed loop would observe its own journal writes as
    // project change, re-route, and never converge. The detached driver has no
    // watcher today and this loop does not give it one.
    'iterations: loop {
        // ---- 1. OBSERVE and ROUTE (routed mode only) -------------------
        //
        // Single-command mode takes none of this: no snapshot capture, no
        // bounds evaluation, no router call and no `observed`/`decided` record,
        // so its journal and its `run.json` are byte-for-byte what Phase 17
        // wrote.
        let command = match &source {
            CommandSource::Fixed(command) => command.clone(),
            CommandSource::Routed { target_phase } => {
                let snapshot = capture_snapshot(project.root()).await;
                let observed = snapshot.project_state.clone();
                bounds_state.observe(snapshot);

                // Pure: no I/O, no model call. The state was read above and is
                // handed in (D-11).
                //
                // **The router runs before the bounds even though the bounds are
                // evaluated before the spawn**, because `evaluate` is asked
                // *which* command is about to run — the command-repeat detector
                // has no question to answer without one. Nothing happens between
                // the two: `decide` opens no file, starts no process and writes
                // no record, so a halt still halts before anything is spawned
                // and before anything is journalled.
                let (command, rationale) = match router::decide(&observed, target_phase) {
                    router::Decision::Run { command, rationale } => (command, rationale),
                    router::Decision::Park { reason, detail } => {
                        terminal = Terminal::Parked { reason, detail };
                        break 'iterations;
                    }
                    router::Decision::NoRule { observed } => {
                        terminal = Terminal::Parked {
                            reason: router::RouterReason::NoRule,
                            detail: observed,
                        };
                        break 'iterations;
                    }
                    // **Its own terminal arm, not `Completed`** (WR-08). The
                    // two are different endings and were labelled by whether an
                    // iteration happened to have spawned, so the run that drove
                    // its target to `passed` reported `succeeded_with_changes`
                    // while the run that found it already there reported
                    // `goal_met`.
                    router::Decision::GoalMet => {
                        terminal = Terminal::GoalMet;
                        break 'iterations;
                    }
                };

                // **Journalled before the bounds are evaluated, and the order is
                // the decision.** What the driver observed and what the router
                // chose are facts about this iteration whether or not a bound
                // then stops it — and the halt is only legible with them: a
                // `parked` record reading `bounds_command_repeat` beside two
                // `decided` records naming the same command says exactly what
                // happened, while the same halt with the second decision missing
                // asks the reader to take it on trust. Recording a decision the
                // bounds refused is not a claim that it ran; `exec_started` is
                // what says a command ran, and none follows a halt.
                record_iteration_decision(
                    &mut run.journal,
                    target_phase,
                    &observed,
                    &command,
                    rationale,
                );

                // The detectors, in their documented order, with the first hit
                // deciding. Exactly one reason is ever reported, and the halt
                // happens **before the spawn**: nothing between the router call
                // and here starts a process.
                if let bounds::BoundVerdict::Halt(reason) = bounds::evaluate(
                    &run_bounds,
                    &bounds_state,
                    run_started_at.elapsed(),
                    &command,
                ) {
                    terminal = Terminal::Halted { reason };
                    break 'iterations;
                }

                command
            }
        };

        // ---- 2. SPAWN --------------------------------------------------

        // The agent's process group, published the instant the child exists
        // rather than only on the `ExecutionHandle` (CR-01). A stop that lands
        // while `start` is still awaiting the capability gate never receives a
        // handle, so without this channel it would have no way to reach the
        // agent's group — which is a *different* group from this driver's, and
        // therefore the one that survives a signal aimed here (D-06, D-09).
        //
        // Per iteration, because the sender is consumed by the spawn it
        // observes; see `build_executor`.
        let (pgid_tx, mut pgid_rx) = oneshot::channel::<u32>();

        let executor = build_executor(args)
            .observing_spawn(pgid_tx)
            .observing_replay_echoes(echo_tx.clone());

        // `biased`, terminate arm FIRST — the same discipline as the drain loop
        // below, for a sharper reason. There the cost of losing the race is a
        // *delayed* stop; here it is a **swallowed** one. `Executor::start`
        // blocks on the capability gate, and the documented `SessionStart` hook
        // hang parks it for minutes, during which the driver would ignore the
        // TUI's SIGTERM, get SIGKILLed at the end of the twelve-second grace,
        // and orphan the agent group it never signalled. That is CR-01, and
        // `tests/driver_kill_startup.rs` is the three-process proof.
        //
        // **Every iteration re-enters this window**, which is the whole reason
        // the arm order is repeated rather than hoisted: an outer loop multiplies
        // the number of startups a stop can land inside, so a swallow that used
        // to be possible once per run is now possible once per command.
        //
        // Losing the race also drops the `start` future, which drops `cancel_tx`
        // and fires the Coordinator's own cancellation — but that teardown is
        // unobservable from here and dies with the runtime, which is why
        // `shutdown_during_startup` tears the group down explicitly instead of
        // relying on it.
        let started = tokio::select! {
            biased;

            _ = term.recv() => {
                shutdown_during_startup(&mut pgid_rx, &mut run.journal).await;
                // Returning drops `run` and with it the `RunLock` — the descriptor
                // close IS the release (D-20.2). The terminal record was written by
                // the call above, so nothing below runs and no second one follows.
                return Ok(());
            }

            // **The remaining run budget, read at the instant of the spawn**
            // (CR-01). `bounds::evaluate` above cannot see inside the iteration
            // this line is starting, so this value is the only thing that keeps
            // that iteration inside the run's own wall-clock cap.
            result = executor.start(
                &project,
                command.clone(),
                iteration_options(
                    &envelope_settings,
                    &envelope_env,
                    &run_bounds,
                    run_started_at.elapsed(),
                ),
            ) => result,
        };

        let mut handle = match started {
            Ok(handle) => handle,
            Err(err) => {
                // A run that started always has a terminal record, even when the
                // thing it was started for never launched (T-17-06).
                if let Err(journal_err) = run.journal.finish("spawn_failed") {
                    tracing::warn!(
                        detail = %format!("{journal_err:#}"),
                        "could not close the journal after a failed spawn",
                    );
                }
                return Err(DriveError::Spawn(err));
            }
        };

        run.journal.set_claude_pgid(handle.pgid);

        // ---- 3. DRAIN --------------------------------------------------
        //
        // The three locals below are **per-iteration**, because each names a
        // fact about one agent process. `stdin_open` in particular has to reset:
        // the previous iteration closed its agent's stdin to let it exit, and a
        // flag carried forward would leave the next agent unsteerable from the
        // moment it started.

        // Whether the agent can still be written to. It starts `true`, and **that
        // is the change that makes steering physically possible** (D-11).
        let mut stdin_open = true;

        // The `rate_limit_event` this iteration will be classified on, retained
        // verbatim (CTRL-07).
        //
        // **Two slots, because "latest" is the wrong retention rule when one of
        // the values is terminal** (WR-02). A rejection is a fact about the whole
        // iteration: a stream that emits `rejected` on one window and then
        // `allowed` on another — one event per window, or one per turn on a
        // steered run — left the single slot holding the `allowed` payload, so
        // `classify` answered `Allowed` and a successful iteration went on to
        // spend more of a quota that had already refused it. Once a rejection is
        // seen it is latched and nothing later clears it.
        //
        // **Per-iteration rather than per-run**, because the classification below
        // runs at the end of this iteration and a value carried forward could only
        // ever be a stale one: a quota rejection stops the run where it is
        // observed, so there is no later iteration for it to be read by.
        let mut latest_quota_event: Option<serde_json::Value> = None;
        let mut rejected_quota_event: Option<serde_json::Value> = None;

        // Every message written to stdin that has not yet been echoed back, and the
        // state the acted-on transition is derived from (D-08).
        let mut pending_acks = PendingAcks::default();

        // How many messages have been written to stdin since the last turn boundary,
        // **counting the ones the poll arm wrote** (CR-01).
        //
        // Two arms deliver, and only one of them decides whether stdin survives. The
        // boundary arm's question is *"is another turn coming?"*, and the answer is
        // yes if **either** arm wrote something since the last boundary — a message
        // the poll arm delivered mid-turn is queued inside the CLI and will run as
        // its own turn, exactly like one delivered at the boundary itself. Without
        // this counter the poll arm's delivery was invisible to the boundary arm
        // (`deliver_pending_inbox` has already advanced `inbox_cursor` past it), so
        // the boundary drain read an empty inbox, concluded `delivered == 0` and
        // closed stdin while a human-steered turn was queued and about to run. Since
        // `INBOX_POLL_INTERVAL` is 750 ms and a turn is minutes, the poll arm wins
        // that race for essentially every message a user types — so the run could be
        // steered exactly **once** and every later message was journaled `missed`.
        // Step 3 of the four-step rule below says the opposite, and this counter is
        // what makes it true.
        let mut delivered_since_boundary = 0usize;

        // `biased`, with the terminate arm FIRST, following `src/main_loop.rs:130`.
        //
        // Arm order is the decision, not a formality. Without `biased` the macro
        // picks a ready arm at random, and with a fast agent the event arm is
        // essentially always ready — so a stop request would lose the race for as
        // long as the stream kept producing, which is the entire duration of the run
        // the user is trying to stop. A stop that loses to a busy event queue is a
        // stop the user experiences as ignored (D-06.1). The inbox poll goes **last**
        // for the mirror-image reason: it is the only arm whose work can wait, and
        // the only one that touches the filesystem. The replay-echo arm sits between
        // them — its work is a string compare against a deque, and the state it
        // records happened 55 seconds ago (D-07), so it is neither urgent nor
        // expensive.
        //
        // `tokio::select!` drops the other arms' futures before it runs the chosen
        // arm's body, which is what lets the terminate arm take `&mut handle` while
        // the event arm's future borrowed it.
        //
        // **When stdin closes, and why it is here rather than after the spawn**
        // (D-11). Until this plan, `close_input()` ran immediately after the spawn
        // with the comment *"one command means one message"* — correct for Phase 17
        // and fatal for Phase 18, because the writer task breaks its loop on
        // `Close` and every later `Executor::send` returns `WriterGone`. **While that
        // line stood, STEER-01/02/03 were not merely unimplemented but physically
        // impossible.** The rule that replaces it is four steps:
        //
        // 1. Do **not** close stdin after spawn.
        // 2. On each `ExecutionEvent::TurnCompleted` — a `result`, which closes a
        //    TURN and not the run (Phase 15 D-29) — drain the inbox one final time.
        // 3. If a message was delivered **by either arm since the last boundary**
        //    (`delivered_since_boundary`), the agent runs it as a new turn and the
        //    loop repeats from step 2. This supports N human-steered turns for free.
        // 4. If nothing was delivered, `close_input()`. EOF is "no more input", not
        //    "stop": the CLI drains what is queued, finishes, and **exits 0**.
        //
        // The final drain at step 2 is what resolves the common race in the user's
        // favour; anything arriving after the close is `missed`, named, and not
        // retried (D-10) — see the sweep below the loop. No new bound is needed:
        // `ExecutionOptions`' idle cap and wall-clock cap remain the backstop for an
        // agent that goes quiet with stdin open.
        loop {
            tokio::select! {
                biased;

                _ = term.recv() => {
                    shutdown_on_terminate(&executor, &mut handle, &mut run.journal).await;
                    // Returning here drops `run`, and with it the `RunLock` — the
                    // descriptor close IS the release (D-20.2). Nothing below this
                    // point runs, so the terminal record written by the call above
                    // is not followed by a second one.
                    return Ok(());
                }

                event = handle.events.recv() => {
                    match event {
                        Some(event) => {
                            let turn_boundary = matches!(event, ExecutionEvent::TurnCompleted(_));

                            if let Err(err) = run.journal.record_exec(&event) {
                                // The error KIND only. Never a message body, which
                                // could carry agent output (T-17-05).
                                tracing::warn!(kind = ?err.kind(), "journal write failed");
                            }

                            // **The quota signal is observed HERE, in the arm that
                            // already receives it** (CTRL-07). It is not reachable
                            // from `derive_run_outcome_from_envelopes`: that
                            // function takes the full `result` envelopes, and this
                            // event is not one — it never enters the envelope
                            // vector at all. Widening that signature to reach it
                            // would recreate the exact shape of CR-04, the
                            // envelope-discarding sibling that let a blocked run
                            // report as a plain success. The driver is the party
                            // responsible for parsed protocol it consumes (D-08),
                            // which is how the acknowledgement correlation above
                            // already works.
                            //
                            // Extending this arm rather than adding a second
                            // consumer of the stream, for the plainest reason:
                            // there is one stream and it already arrives here.
                            // Nothing else about the arm changes — the turn
                            // boundary, the journal write and the stream-close
                            // break are all exactly as they were.
                            //
                            // The payload is retained UNINSPECTED. Every question
                            // about it is `driver::rate_limit`'s, which is pure and
                            // reads no clock, so the one place a wire field is
                            // interpreted is the one place its malformed shapes are
                            // tested.
                            if let ExecutionEvent::Message(message) = &event {
                                if let StreamMessage::RateLimitEvent(payload) = message.as_ref() {
                                    // The FIRST rejection wins its slot and is
                                    // never overwritten; the latest of anything
                                    // else fills the other. The predicate is
                                    // `rate_limit`'s, so this arm still asks no
                                    // question of the payload itself and still
                                    // reads no clock (WR-02).
                                    if rejected_quota_event.is_none()
                                        && rate_limit::is_rejection(Some(payload))
                                    {
                                        rejected_quota_event = Some(payload.clone());
                                    }
                                    latest_quota_event = Some(payload.clone());
                                }
                            }

                            if turn_boundary && stdin_open {
                                let delivered = delivered_since_boundary
                                    + deliver_pending_inbox(
                                        &executor,
                                        &mut handle,
                                        &mut run.journal,
                                        &inbox_path,
                                        &mut inbox_cursor,
                                        &mut pending_acks,
                                    )
                                    .await;
                                // Reset unconditionally: whatever this boundary
                                // decided, the next one asks about the turn that is
                                // starting now and not about the one that just ended.
                                delivered_since_boundary = 0;

                                if delivered == 0 {
                                    // Raced the same way and for the same reason as
                                    // every other await in this file: by this point
                                    // there **is** a handle, so a stop here takes
                                    // the ordinary layer-2 path through
                                    // `Executor::cancel` and no second teardown is
                                    // written (D-06.2).
                                    tokio::select! {
                                        biased;

                                        _ = term.recv() => {
                                            shutdown_on_terminate(
                                                &executor,
                                                &mut handle,
                                                &mut run.journal,
                                            )
                                            .await;
                                            return Ok(());
                                        }

                                        result = handle.close_input() => {
                                            if let Err(err) = result {
                                                tracing::warn!(
                                                    kind = ?err,
                                                    "could not signal end-of-input to the agent",
                                                );
                                            }
                                        }
                                    }
                                    stdin_open = false;
                                }
                            }
                        }
                        None => break,
                    }
                }

                // The acted-on transition, and the only arm that produces it. It
                // sits after the event arm because an echo is never urgent — the
                // state it establishes happened 55 seconds ago (D-07) — and before
                // the inbox poll because it is the cheaper of the two: a string
                // compare against a deque, with no filesystem call at all.
                echo = echo_rx.recv(), if echo_open => {
                    match echo {
                        Some(raw) => correlate_replay_echo(
                            &mut pending_acks,
                            &mut run.journal,
                            &raw,
                        ),
                        // Unreachable while `executor` is alive; see `echo_open`.
                        None => echo_open = false,
                    }
                }

                _ = inbox_poll.tick() => {
                    if stdin_open {
                        // The count is CARRIED, never dropped (CR-01). A message
                        // written here is a turn the CLI has queued, and the
                        // boundary arm has no other way to learn that.
                        delivered_since_boundary += deliver_pending_inbox(
                            &executor,
                            &mut handle,
                            &mut run.journal,
                            &inbox_path,
                            &mut inbox_cursor,
                            &mut pending_acks,
                        )
                        .await;
                    } else {
                        // **The arm keeps polling after the close, and that is the
                        // point** (D-10). Nothing read here can ever be delivered —
                        // stdin cannot be reopened — so each message is journaled
                        // `missed` the moment it is seen rather than at the end of
                        // the run. The difference is what the user watches: a
                        // message that reports its fate within a poll interval,
                        // versus one that sits in `queued` for however long the
                        // agent takes to finish, indistinguishable from a slow
                        // agent. That indistinguishability is PITFALLS' Pitfall 11
                        // wearing a spinner.
                        sweep_inbox_as_missed(
                            &mut run.journal,
                            &inbox_path,
                            &mut inbox_cursor,
                        )
                        .await;
                    }
                }
            }
        }

        // The stream has ended, so every echo that will ever arrive has arrived.
        // Drained without awaiting: a `recv()` here would park until the executor's
        // sender dropped, which happens after this function returns.
        while let Ok(raw) = echo_rx.try_recv() {
            correlate_replay_echo(&mut pending_acks, &mut run.journal, &raw);
        }

        // Whatever is left was **delivered and never dequeued**. It keeps its
        // `interjected` record and gains no `interjection_acted_on`, because the
        // driver never observed one and inventing it would be the lie the whole
        // three-state display exists to prevent. A count only — never an id and
        // never a body — because this is the ordinary end of a run and not a fault.
        let unacked = pending_acks.drain_undelivered().len();
        if unacked > 0 {
            tracing::debug!(
                count = unacked,
                "the run ended before the agent dequeued every delivered message",
            );
        }

        // ---- 4. CLASSIFY -----------------------------------------------
        //
        // **The iteration boundary is the event stream closing, never a `result`
        // envelope** (D-29). A single `claude` process emits one `result` per
        // *turn*, and a run steered mid-turn emits several; breaking on the first
        // would truncate every steered iteration while reporting success. The
        // drain loop above therefore ends only on `handle.events.recv()`
        // returning `None`, and this is the line after it.
        let iteration_outcome = handle.wait_outcome().await;

        // **The quota check, and it applies to BOTH execution models** (CTRL-07).
        //
        // Two detectors, because no capture of a `rejected` `rate_limit_event`
        // exists and one cannot be produced without burning the very quota it
        // describes (research assumption A2). The first reads the payload the
        // drain loop retained; the second reads the failure envelope's own
        // terminal reason, off the outcome value this function already holds —
        // so **nothing about the outcome derivation changes** and no second
        // envelope-discarding path is created.
        //
        // The second detector reports the window as `unknown` rather than
        // borrowing one from a retained payload, and that is deliberate: the only
        // payload it could borrow from is one that said `allowed`, which by
        // definition did not describe this refusal. CONTEXT.md's commitment is to
        // report unknown rather than to guess, and a plausible window presented as
        // the one that blocked the run is a guess wearing a fact's clothes.
        //
        // **The latched rejection is classified first, and the latest event only
        // if there was none** (WR-02). `or` rather than a second `classify` call,
        // so there is still exactly one classification per iteration and still
        // exactly one place that decides what a payload means.
        let quota_park = match rate_limit::classify(
            rejected_quota_event
                .as_ref()
                .or(latest_quota_event.as_ref()),
            chrono::Utc::now(),
        ) {
            rate_limit::QuotaVerdict::Rejected { window, resets_at } => {
                Some(rate_limit::park_detail(&window, resets_at))
            }
            rate_limit::QuotaVerdict::Allowed => {
                let terminal_reason = match &iteration_outcome {
                    RunOutcome::Failed {
                        terminal_reason, ..
                    } => terminal_reason.as_deref(),
                    _ => None,
                };
                rate_limit::terminal_reason_names_a_rate_limit(terminal_reason).then(|| {
                    rate_limit::park_detail(&rate_limit::QuotaWindow::Unknown(None), None)
                })
            }
        };

        let succeeded = matches!(
            iteration_outcome,
            RunOutcome::SucceededWithChanges { .. } | RunOutcome::SucceededNoChanges { .. }
        );
        last_outcome = Some(iteration_outcome);

        if let Some(detail) = quota_park {
            // **The run STOPS. No retry, no backoff, no sleep-until-reset, and
            // nothing scheduled against the reset time.** CTRL-07's requirement is
            // that a rate-limited run parks rather than retrying, and a backoff is
            // a retry with a delay — one that additionally spends the wait from a
            // quota shared with every other Claude surface the user has, so the
            // budget it burns on the retry is partly somebody else's.
            //
            // The reset time is reported so a human can decide when to come back.
            // It is displayed and never acted on, which is also what makes an
            // attacker-influenced value harmless beyond the sanity bound already
            // applied to it (T-20-24).
            //
            // It breaks out of BOTH execution models rather than only the routed
            // one. Single-command mode is otherwise byte-for-byte the run Phase 17
            // shipped, and it stays so for every run that is not rate-limited; but
            // a Phase 17 run that hit a quota reported a bare `failed` with no
            // reason at all, which is exactly the unclassified ending DRIVE-06
            // exists to remove.
            terminal = Terminal::QuotaParked { detail };
            break 'iterations;
        }

        match &source {
            // One supplied command, one iteration. The break is what keeps
            // single-command mode exactly the run Phase 17 shipped.
            CommandSource::Fixed(_) => break 'iterations,
            CommandSource::Routed { .. } => {
                // Recorded **after** the iteration ran, so the step count is
                // completed steps rather than attempted ones and the
                // command-repeat detector compares against a command that
                // actually executed.
                bounds_state.record_step(command);

                if !succeeded {
                    // An agent that failed, was denied, timed out or stalled
                    // ends the run. Continuing would issue the next command
                    // against a project whose previous step did not land, which
                    // is how a routed run converts one failure into a sequence
                    // of them. The outcome's own label names it, so this
                    // terminal needs no reason of its own.
                    break 'iterations;
                }
            }
        }
    }

    // ---- The run's single terminal record --------------------------------

    // A park or a halt says why, once, through the carrier Phase 19 established
    // and proved readable by a separate process — never through a second one
    // invented here (D-25, DRIVE-06).
    record_terminal(&mut run.journal, &terminal);

    // The stream has ended, so nothing further can be delivered. Anything still
    // in the inbox reaches its own named terminal state instead of sitting in
    // `queued` forever (D-10).
    //
    // **Once per run rather than once per iteration**, and the difference is
    // real: swept between commands, a message queued while iteration one was
    // finishing would be journaled `missed` even though iteration two was about
    // to open a fresh stdin and could have delivered it.
    sweep_inbox_as_missed(&mut run.journal, &inbox_path, &mut inbox_cursor).await;

    // The label. **This run's own terminal is asked FIRST, and the order is
    // WR-01.** It used to be asked only on the branch where no iteration had
    // spawned; a run that halted on a bound *after* a successful iteration
    // derived its label by re-reading the journal from disk instead, and
    // `terminal_label` answers `outcome_label` whenever that read fails. Since
    // `record_terminal` swallows every journal error with a `warn!`, a failed
    // `Parked` write — ENOSPC, a truncated line, an unreadable journal at run
    // end — turned `parked:bounds_step_cap` into `succeeded_with_changes`, and a
    // supervising process reading `run.json` concluded the run had finished its
    // work. The authoritative value was in hand the whole time; an I/O failure
    // must not be able to change a run's classification (DRIVE-06, criterion 5).
    //
    // The journal read is still reached whenever this run has no reason of its
    // own, which is the case it was written for: a park recorded by *another*
    // process — the hook or guard re-entries — that this driver never observed.
    let label = match own_terminal_label(&terminal) {
        Some(label) => label,
        None => match last_outcome {
            Some(outcome) => {
                // Same blocking hand-off and the same inline fallback as
                // `shutdown_on_terminate`'s: one end-of-run journal read, moved off the
                // async path (D-28, WR-10).
                let journal_path = run.journal.paths().journal.clone();
                let task_outcome = outcome.clone();
                let task_path = journal_path.clone();
                match tokio::task::spawn_blocking(move || terminal_label(&task_outcome, &task_path))
                    .await
                {
                    Ok(label) => label,
                    Err(err) => {
                        tracing::warn!(
                            panicked = err.is_panic(),
                            "the terminal-label task did not run to completion",
                        );
                        terminal_label(&outcome, &journal_path)
                    }
                }
            }
            // Nothing ever spawned and this run named no reason of its own.
            // Unreachable today: every `break` that precedes a spawn sets a
            // terminal that `own_terminal_label` answers. Spelled out rather
            // than `unwrap`ped because a detached driver that panicked here
            // would leave a run directory with no terminal record, which is the
            // signal D-12 reserves for a genuine crash.
            None => GOAL_MET_LABEL.to_string(),
        },
    };

    run.journal.finish(&label).map_err(|err| DriveError::Journal {
        detail: format!("{err:#}"),
    })?;

    Ok(())
}

/// Journal what this iteration observed and what it decided, in that order.
///
/// Two records rather than one, because they answer different questions and a
/// later reader wants them separately: *what did the driver see* and *what did it
/// do about it*. Both were schema'd by Phase 16 specifically so this phase adds
/// no migration (D-36).
///
/// **`by` is `"policy"` and nothing else.** The field's vocabulary is exactly
/// three values — `policy`, `llm`, `human` — and `llm` is Phase 21's. This phase
/// mints no fourth.
///
/// **Neither record can carry agent output.** `drpev` is enum names and counts,
/// `command` is composed from a phase number, and `rationale` is a `&'static str`
/// from the router's closed set (SAFE-04, T-20-05).
///
/// A failed write is warned about and swallowed, exactly as the envelope notice
/// and the override diagnostic are: a journal that cannot take a decision record
/// must still be given the chance to take the run's terminal one, which is the
/// more important of the two.
fn record_iteration_decision(
    journal: &mut JournalRun,
    target_phase: &str,
    observed: &crate::state_reader::ProjectState,
    command: &str,
    rationale: &'static str,
) {
    for event in [
        JournalEvent::Observed {
            phase: target_phase.to_string(),
            drpev: drpev_stages(observed, target_phase),
        },
        JournalEvent::Decided {
            by: "policy".to_string(),
            command: command.to_string(),
            rationale: rationale.to_string(),
        },
    ] {
        if let Err(err) = journal.record(&event) {
            // The error KIND only, never a message body (T-17-05).
            tracing::warn!(kind = ?err.kind(), "could not journal an iteration decision");
        }
    }
}

/// Record why the run stopped, if it stopped for a reason of its own.
///
/// [`Terminal::Completed`] and [`Terminal::GoalMet`] write nothing, and that is
/// correct rather than a gap: the first's reason is the derived outcome's own
/// label and the second's is [`GOAL_MET_LABEL`], both of which the terminal
/// record already carries. A `Parked` record for either would claim a run that
/// finished its work needed a human.
///
/// A halt and a park share one carrier deliberately. Phase 19 established
/// `JournalEvent::Parked` as the durable "this run stopped, here is why", proved
/// readable by a separate process, and CONTEXT.md is explicit that the
/// router/bounds reasons reuse it rather than inventing a second — so one grep
/// answers *why did this run end* across the safety envelope, the router and the
/// bounds at once.
///
/// The detail, when there is one, goes on a preceding `Diagnostic` rather than
/// into `needs`: `needs` is documented as naming the actor that would unpark the
/// run, and overloading it with an observed status would make the field mean two
/// things depending on which producer wrote it.
fn record_terminal(journal: &mut JournalRun, terminal: &Terminal) {
    let Some(reason) = terminal.park_reason() else {
        return;
    };

    let detail = terminal.detail();
    if !detail.is_empty() {
        if let Err(err) = journal.record(&JournalEvent::Diagnostic {
            code: reason.to_string(),
            detail: detail.to_string(),
        }) {
            tracing::warn!(kind = ?err.kind(), "could not journal the park detail");
        }
    }

    if let Err(err) = journal.record(&JournalEvent::Parked {
        reason: reason.to_string(),
        needs: terminal.needs().to_string(),
    }) {
        tracing::warn!(kind = ?err.kind(), "could not journal the terminal park record");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Aliased on import rather than called through its module path, and the
    // rename is load-bearing rather than stylistic. `tests/spawn_seam_guard.rs`
    // treats `process_group(` as a process-SPAWN marker, and its left word
    // boundary accepts a `::` — so an inline `liveness::process_group(…)` here
    // would report this module, which spawns nothing, as a spawn site. The
    // guard's own doc names the right fix for that class (`kill_process_group(`
    // is the same case) and names the wrong one: putting a file that spawns
    // nothing onto a spawn allowlist, which "quietly turns an audit into a list
    // of files somebody once had to add". An identifier character before the
    // marker is exactly what the boundary was built to accept.
    use crate::driver::liveness::process_group as kernel_process_group;

    /// A `DriveArgs` carrying nothing this module's own assertions vary.
    fn args() -> DriveArgs {
        DriveArgs {
            alias: "demo".to_string(),
            command: Some("/gsd-progress".to_string()),
            target_phase: None,
            max_steps: None,
            wall_clock_cap_secs: None,
            max_escalations: None,
            run_id: Some("2026-07-29T12-00-00Z-aaaa".to_string()),
            dry_run: false,
            goal: None,
            #[cfg(debug_assertions)]
            claude_program: None,
            #[cfg(debug_assertions)]
            claude_args: Vec::new(),
        }
    }

    /// A registry entry with no opt-in record; `make_run_record` only reads the
    /// opt-in timestamp, which is legitimately absent for a hand-typed run.
    fn entry() -> RegisteredProject {
        RegisteredProject {
            path: std::path::PathBuf::from("/nonexistent"),
            added: "2026-07-29T12:00:00Z".to_string(),
            driver_opt_in: None,
            extra: Default::default(),
        }
    }

    /// A started run in a temp project, for the terminal-label assertions.
    fn started_run(run_id: &str) -> (tempfile::TempDir, JournalRun) {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let planning = dir.path().join(".planning");
        let record = make_run_record(
            run_id.to_string(),
            &args(),
            &entry(),
            &ExecutionOptions::default(),
            "fnv1a64:0000000000000000".to_string(),
            std::process::id(),
            bounds::RunBounds::default(),
        );
        let run = JournalRun::start(&planning, record).expect("the run starts");
        (dir, run)
    }

    /// A succeeded-with-no-changes outcome, which is the label a park has to
    /// displace — the pairing matters: a `terminal_label` that always said
    /// `parked:` would satisfy every park assertion and be useless.
    fn clean_outcome() -> RunOutcome {
        RunOutcome::SucceededNoChanges {
            turns: Vec::new(),
            total_cost_usd: None,
        }
    }

    #[test]
    fn a_run_whose_journal_carries_no_park_keeps_its_ordinary_terminal_label() {
        let (_dir, run) = started_run("2026-08-18T00-00-00Z-nopark");
        assert_eq!(
            terminal_label(&clean_outcome(), &run.paths().journal),
            "succeeded_no_changes",
            "a run nothing refused must not be labelled parked"
        );
    }

    #[test]
    fn the_terminal_label_names_the_park_reason_from_the_last_park_on_disk() {
        let (_dir, mut run) = started_run("2026-08-18T00-00-00Z-parked");

        // TWO parks, because a run may be refused more than once and the
        // terminal record names the state it ENDED in.
        run.record(&JournalEvent::Parked {
            reason: ParkReason::SecretDetected.as_str().to_string(),
            needs: "human".to_string(),
        })
        .expect("the first park");
        run.record(&JournalEvent::Parked {
            reason: ParkReason::ForcePushBlocked.as_str().to_string(),
            needs: "human".to_string(),
        })
        .expect("the second park");

        assert_eq!(
            terminal_label(&clean_outcome(), &run.paths().journal),
            format!(
                "{PARKED_LABEL_PREFIX}{}",
                ParkReason::ForcePushBlocked.as_str()
            ),
            "the LAST park is the state the run ended in"
        );
    }

    #[test]
    fn a_halt_labels_itself_from_memory_and_a_journal_it_never_reads() {
        // **WR-01.** The failure this pins: a routed run halts on
        // `bounds_step_cap` after a successful iteration; the `Parked` record
        // fails to land (ENOSPC, a truncated line, an unreadable journal at run
        // end) and `record_terminal` swallows that with a `warn!`. Deriving the
        // label from the journal then answers `outcome_label`, and `run.json`
        // records `succeeded_with_changes` for a run that stopped short. A
        // supervising process reads that as work completed.
        //
        // The proof is structural as well as behavioural: `own_terminal_label`
        // takes no `&Path`, so no journal read — failed, stale or absent — can
        // reach its answer. This test constructs the exact pair the bug needed
        // (a terminal that halted AND a plausible successful outcome) and shows
        // the outcome loses.
        let halted = Terminal::Halted {
            reason: bounds::BoundsReason::StepCap,
        };
        assert_eq!(
            own_terminal_label(&halted).as_deref(),
            Some(format!("{PARKED_LABEL_PREFIX}{}", bounds::REASON_STEP_CAP).as_str()),
            "the run's own halt reason is authoritative and needs no round trip \
             through disk. An I/O failure may cost a journal record; it may not \
             change how the run is classified (DRIVE-06, criterion 5)"
        );

        // The same for every other arm that names a reason of its own.
        assert_eq!(
            own_terminal_label(&Terminal::Parked {
                reason: router::RouterReason::NoRule,
                detail: "planned".to_string(),
            })
            .as_deref(),
            Some(format!("{PARKED_LABEL_PREFIX}{}", router::RouterReason::NoRule.as_str()).as_str())
        );
        assert_eq!(
            own_terminal_label(&Terminal::QuotaParked {
                detail: "window=five_hour resets_at=unknown".to_string(),
            })
            .as_deref(),
            Some(
                format!(
                    "{PARKED_LABEL_PREFIX}{}",
                    rate_limit::QuotaReason::Rejected.as_str()
                )
                .as_str()
            )
        );

        // And the negative half, which is what keeps the journal read reachable
        // for the case it was written for: a run that stopped for no reason of
        // its OWN must still let a park recorded by another process — the hook
        // or guard re-entries this driver never observes — decide the label.
        assert_eq!(
            own_terminal_label(&Terminal::Completed),
            None,
            "a completed run has no reason of its own, so the outcome and the \
             journal still decide. Answering here would suppress an earlier \
             envelope park that only the journal knows about"
        );
    }

    #[test]
    fn the_terminal_record_on_disk_carries_the_park_reason_as_its_outcome() {
        // The whole of D-25's fourth evidence fact, through the production
        // `finish` path rather than through the label function alone.
        let (dir, mut run) = started_run("2026-08-18T00-00-00Z-record");
        run.record(&JournalEvent::Parked {
            reason: ParkReason::PrCapExceeded.as_str().to_string(),
            needs: "human".to_string(),
        })
        .expect("the park");

        let journal = run.paths().journal.clone();
        let run_json = run.paths().run_json.clone();
        let label = terminal_label(&clean_outcome(), &journal);
        run.finish(&label).expect("the run finishes");

        let body = std::fs::read_to_string(&run_json).expect("the run record is on disk");
        let record: serde_json::Value = serde_json::from_str(&body).expect("it is JSON");
        assert_eq!(
            record["outcome"].as_str(),
            Some(
                format!(
                    "{PARKED_LABEL_PREFIX}{}",
                    ParkReason::PrCapExceeded.as_str()
                )
                .as_str()
            ),
            "the terminal record must name the park reason, not a generic label; \
             record was {body}"
        );
        drop(dir);
    }

    #[test]
    fn the_run_record_carries_the_group_it_was_given_and_not_a_second_copy_of_the_pid() {
        // A group this process cannot possibly be in, so an implementation that
        // ignored the argument and wrote `std::process::id()` twice — which is
        // what WR-01 describes — could not pass by coincidence.
        const SENTINEL_PGID: u32 = 4_242_424;

        let record = make_run_record(
            "2026-07-29T12-00-00Z-aaaa".to_string(),
            &args(),
            &entry(),
            &ExecutionOptions::default(),
            "fnv1a64:0000000000000000".to_string(),
            SENTINEL_PGID,
            bounds::RunBounds::default(),
        );

        assert_eq!(
            record.pgid, SENTINEL_PGID,
            "the record must carry the group it was HANDED. Writing \
             std::process::id() here made the record assert a leadership a failed \
             setpgid means the process does not hold — and \
             kill::resolve_signal_target then refuses every stop against that run \
             (WR-01, D-04)"
        );
        assert_eq!(
            record.pid,
            std::process::id(),
            "the pid is still this process's own; the two fields are simply no \
             longer the same expression"
        );
    }

    #[test]
    fn the_current_group_agrees_with_the_proc_parse() {
        // Two independent sources of one fact: the `getpgrp` syscall and the
        // `/proc/<pid>/stat` field parse `kill::resolve_signal_target` compares
        // the record against. If either the syscall wrapper or the stat field
        // index were wrong, every stop in the product would refuse — and the two
        // would disagree here first.
        //
        // Deliberately NOT `establish_own_group()`: `setpgid` in a shared test
        // binary would move the harness's own process group, and with it every
        // other test in this process.
        let from_syscall = current_group();
        let from_proc = kernel_process_group(std::process::id())
            .expect("this process's own /proc/<pid>/stat is readable");

        assert_eq!(
            from_syscall, from_proc,
            "getpgrp() and the /proc pgrp field must name the same group. A \
             disagreement means one of the two is reading the wrong thing, and \
             the consequence is a kill switch that refuses every stop (D-04)"
        );
    }

    /// What the startup teardown still has to do after the agent group is gone:
    /// the diagnostic append, the terminal `run.json` write, and this process's
    /// own exit.
    ///
    /// A named budget rather than a fudge factor, following
    /// `tests/executor_lifecycle.rs:43-46`'s convention of mirroring a value
    /// where the reasoning about it happens.
    const JOURNAL_BUDGET: Duration = Duration::from_secs(2);

    #[test]
    fn the_startup_stop_budget_fits_inside_the_driver_teardown_grace() {
        let budget = STARTUP_AGENT_GRACE + STARTUP_REAP_BOUND + JOURNAL_BUDGET;
        assert!(
            budget <= crate::driver::kill::DRIVER_TEARDOWN_GRACE,
            "the whole startup teardown ({budget:?}) must fit inside the grace the \
             TUI gives the driver ({:?}). A breach is not a slow stop: the TUI \
             SIGKILLs the driver mid-teardown, which orphans the agent process \
             group and its grandchildren — the exact failure this path exists to \
             prevent, reintroduced by a number",
            crate::driver::kill::DRIVER_TEARDOWN_GRACE
        );

        // And the startup grace is deliberately SHORTER than the drain path's,
        // which is the claim the constant's doc makes. Pinned so a later edit
        // that "harmonises" the two has to face the budget above.
        assert!(
            STARTUP_AGENT_GRACE < Duration::from_secs(10),
            "the startup grace must stay below the drain path's ten seconds: \
             during startup there is no turn to abort, no Bash tree mid-command \
             and no SessionEnd chain, and the ten-second version does not fit the \
             budget asserted above"
        );
    }

    #[test]
    fn the_outcome_label_for_a_stopped_run_is_killed() {
        // One line, and it is what stops a later refactor silently relabelling a
        // stop as a failure. `src/driver/reconcile.rs` and the TUI both read the
        // `outcome` string off `run.json`, and a stop that arrives there as
        // `failed` tells the user their run broke when in fact they stopped it.
        assert_eq!(
            outcome_label(&RunOutcome::Killed { turns: Vec::new() }),
            "killed"
        );

        // The neighbour that would be reached if the terminate path ever stopped
        // going through `Executor::cancel` and started letting the run fall out
        // of its drain instead. Pinned so the two stay distinguishable.
        assert_ne!(
            outcome_label(&RunOutcome::Failed {
                reason: "x".to_string(),
                subtype: None,
                terminal_reason: None,
                exit_code: None,
            }),
            outcome_label(&RunOutcome::Killed { turns: Vec::new() })
        );
    }

    /// The other side of D-11's boundary, and the reason it is testable here
    /// rather than end to end.
    ///
    /// The *delivered* side is proved by `tests/driver_inbox.rs` against a real
    /// child process. The *missed* side cannot be reached that way without a
    /// sleep: it needs a message to land after the driver has closed stdin, and
    /// "after" in a live run is a race no assertion can pin. So the sweep is
    /// exercised directly against a real `JournalRun` and a real inbox file,
    /// which is the whole of what runs once the event stream has ended.
    ///
    /// The load-bearing assertion is that the ids come back. A sweep that
    /// journalled a *count* would satisfy "nothing is silently abandoned" in
    /// prose and leave the four-state display unable to say **which** message
    /// was lost, which is the only version of that answer a user can act on.
    #[tokio::test]
    async fn a_message_left_in_the_inbox_when_the_stream_ends_is_journaled_as_missed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");

        let record = make_run_record(
            "2026-07-29T12-00-00Z-aaaa".to_string(),
            &args(),
            &entry(),
            &ExecutionOptions::default(),
            journal::argv_digest(&["claude".to_string()]),
            4242,
            bounds::RunBounds::default(),
        );
        let mut journal = JournalRun::start(&planning, record).expect("start the run");
        let inbox_path = journal.paths().inbox.clone();

        let first = InboxMessage::new("this one was delivered");
        let second = InboxMessage::new("this one arrived too late");
        inbox::append(&inbox_path, &first).expect("append");

        // Consume the first message the way the drain loop would, so the cursor
        // sits exactly where the close left it.
        let mut cursor = TailCursor::default();
        assert_eq!(read_inbox(&inbox_path, &mut cursor).await.len(), 1);

        inbox::append(&inbox_path, &second).expect("append after the close");
        assert_eq!(
            sweep_inbox_as_missed(&mut journal, &inbox_path, &mut cursor).await,
            1,
            "only the message that arrived after the cursor may be swept"
        );
        journal.finish("succeeded_no_changes").expect("finish");

        let (records, _) =
            crate::journal::reader::read_all(&journal.paths().journal).expect("read");
        let missed: Vec<_> = records
            .iter()
            .filter(|record| record.kind == "interjection_missed")
            .collect();
        assert_eq!(missed.len(), 1, "one message, one terminal state");
        assert_eq!(
            missed[0].rest["id"], second.id,
            "the record must name WHICH message was lost, not merely that one was"
        );
        assert_eq!(missed[0].rest["reason"], MISSED_AFTER_CLOSE);
        assert!(
            !records
                .iter()
                .any(|record| record.kind == "interjection_missed"
                    && record.rest["id"] == first.id),
            "a message already past the cursor must never be re-reported as missed"
        );
    }

    /// A handle whose agent is **not running**, so every `Executor::send`
    /// returns `SendError::NotRunning`.
    ///
    /// Built field by field rather than by spawning something and killing it:
    /// the failure under test is the *write returning `Err`*, and reproducing it
    /// through a real child would be a race between the kill and the write. This
    /// reaches the same branch of `ExecutionHandle::write_line` deterministically
    /// and spawns nothing at all.
    fn dead_handle() -> ExecutionHandle {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;

        let (stdin_tx, _stdin_rx) = tokio::sync::mpsc::channel(1);
        let (_events_tx, events_rx) = tokio::sync::mpsc::channel(1);

        ExecutionHandle {
            id: crate::executor::ExecutionId(uuid::Uuid::nil()),
            session_id: "s".to_string(),
            events: events_rx,
            capabilities: Vec::new(),
            pgid: 0,
            claude_code_version: String::new(),
            pending_control: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            control_response_cap: Duration::from_secs(1),
            stdin_tx,
            // The one field the test is about.
            running: Arc::new(AtomicBool::new(false)),
            cancel_tx: None,
            outcome_rx: None,
            outcome: None,
            next_control_seq: 0,
        }
    }

    /// CR-03: a message whose stdin write FAILED must reach a terminal state on
    /// disk, at the driver, where the fact is known.
    ///
    /// This is the pitfall D-10 exists to abolish, in the code written to
    /// abolish it. The message is consumed from the tail, so `inbox_cursor` has
    /// already moved past it: it can never be read again, never retried, and
    /// never reaches `sweep_inbox_as_missed`, which only sees messages the
    /// cursor has not passed. Before this fix `interjected { delivered: false }`
    /// was the last word ever written about it and the four-state display left
    /// it in `queued` — *"durably on disk; nothing has read it yet"* — forever,
    /// which is false twice over.
    ///
    /// Both records are asserted, and the pairing is the point: the
    /// `interjected` record is the honest account of the **attempt**, the
    /// `missed` record the honest account of its **fate**, and the delivery
    /// count must not include it — counting a failed write would park the run
    /// waiting for a turn that is never coming.
    #[tokio::test]
    async fn a_message_whose_stdin_write_failed_is_journaled_missed_rather_than_left_queued() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");

        let record = make_run_record(
            "2026-07-29T12-00-00Z-aaaa".to_string(),
            &args(),
            &entry(),
            &ExecutionOptions::default(),
            journal::argv_digest(&["claude".to_string()]),
            4242,
            bounds::RunBounds::default(),
        );
        let mut journal = JournalRun::start(&planning, record).expect("start the run");
        let inbox_path = journal.paths().inbox.clone();

        let message = InboxMessage::new("steer this run");
        inbox::append(&inbox_path, &message).expect("append");

        let executor = ClaudeExecutor::new();
        let mut handle = dead_handle();
        let mut cursor = TailCursor::default();
        let mut pending = PendingAcks::default();

        let delivered = deliver_pending_inbox(
            &executor,
            &mut handle,
            &mut journal,
            &inbox_path,
            &mut cursor,
            &mut pending,
        )
        .await;
        assert_eq!(
            delivered, 0,
            "a write that returned Err delivered nothing, and counting it would \
             park the run waiting for a turn that is never coming"
        );
        journal.finish("succeeded_no_changes").expect("finish");

        let (records, _) =
            crate::journal::reader::read_all(&journal.paths().journal).expect("read");
        let for_id = |kind: &str| -> Vec<&crate::journal::reader::JournalRecord> {
            records
                .iter()
                .filter(|record| record.kind == kind && record.rest["id"] == message.id)
                .collect()
        };

        let interjected = for_id("interjected");
        assert_eq!(interjected.len(), 1, "the attempt is recorded");
        assert_eq!(
            interjected[0].rest["delivered"], false,
            "and recorded as what it was: a write that did not return Ok"
        );

        let missed = for_id("interjection_missed");
        assert_eq!(
            missed.len(),
            1,
            "the message must reach a TERMINAL state on disk. Without this \
             record the cursor has moved past it, nothing will ever read it \
             again, and the four-state widget renders `queued` forever"
        );
        assert_eq!(
            missed[0].rest["reason"], MISSED_SEND_FAILED,
            "and the reason must name the WRITE. Reusing the after-close reason \
             would tell a user the run had stopped listening when it may still \
             be live"
        );
        assert!(
            interjected[0].seq < missed[0].seq,
            "the fate follows the attempt"
        );

        // The cursor really has moved past it, which is what makes the record
        // above the only chance this message ever had.
        assert!(
            read_inbox(&inbox_path, &mut cursor).await.is_empty(),
            "a consumed message is gone from the tail — there is no second look"
        );
    }

    #[test]
    fn the_inbox_poll_interval_is_sized_for_a_unit_of_work_measured_in_minutes() {
        // The bound in both directions is the decision, not the number. Too
        // short and a detached process burns syscalls forever for a latency no
        // human perceives; too long and the injection the user just typed feels
        // dropped. CONTEXT fixes the order at 500ms-1s and prefers polling to a
        // `notify` watcher outright.
        assert!(INBOX_POLL_INTERVAL >= Duration::from_millis(500));
        assert!(INBOX_POLL_INTERVAL <= Duration::from_secs(1));
    }

    // ========================================================================
    // The replay-echo correlator (D-07, D-08, STEER-02)
    //
    // Exhaustive here rather than end to end, and that is what the free
    // function bought: reaching the duplicate-text case against a real agent
    // would need a process, a dequeue delay and two messages with the same
    // body, and the assertion would still be about the order of two records.
    // ========================================================================

    /// A pending FIFO built from `(id, text)` literals.
    fn pending(entries: &[(&str, &str)]) -> VecDeque<(String, String)> {
        entries
            .iter()
            .map(|(id, text)| (id.to_string(), text.to_string()))
            .collect()
    }

    /// The override marker names the stand-in and discloses nothing else
    /// (D-30, T-18-45).
    ///
    /// The journal is written into the **driven** project's `.planning/`, which
    /// is a directory a user may commit, so a full path would publish where a
    /// developer's tree lives. The arguments are worse — they are caller-supplied
    /// free text that in this repo's own fixtures already carries transcript
    /// paths — and they are not in the detail at all, which is why this test
    /// asserts on their absence rather than on their shape.
    #[cfg(debug_assertions)]
    #[test]
    fn the_override_marker_names_the_stand_in_and_leaks_neither_its_path_nor_its_arguments() {
        let detail = agent_override_detail(Path::new("/home/someone/secret-tree/fake-claude.sh"));

        assert!(
            detail.contains("fake-claude.sh"),
            "the marker must name the stand-in, or it says only that something was \
             overridden and not what: {detail}"
        );
        assert!(
            !detail.contains("secret-tree") && !detail.contains('/'),
            "no path component may reach the journal — the file name is the whole \
             of what a reader needs: {detail}"
        );

        // A path with no file name degrades to a fixed word rather than to the
        // full path, because the full path is the thing being avoided.
        let unnamed = agent_override_detail(Path::new("/tmp/.."));
        assert!(
            !unnamed.contains("/tmp"),
            "a path with no file name must not fall back to the path itself: {unnamed}"
        );
    }

    #[test]
    fn an_echo_against_an_empty_deque_matches_nothing() {
        let mut deque = pending(&[]);
        assert_eq!(match_replay_echo(&mut deque, "anything"), None);
        assert!(deque.is_empty());
    }

    #[test]
    fn an_echo_matches_the_pending_message_with_the_same_text() {
        let mut deque = pending(&[("id-1", "skip the UI review")]);
        assert_eq!(
            match_replay_echo(&mut deque, "skip the UI review"),
            Some("id-1".to_string())
        );
        assert!(
            deque.is_empty(),
            "a matched message must leave the queue, or its echo could ack it \
             twice"
        );
    }

    #[test]
    fn two_identical_texts_are_matched_in_delivery_order() {
        // The case the whole FIFO exists for. Two identical messages are
        // legitimate — a user may say "continue" twice — and matching the
        // NEWEST first would let the second echo re-ack the first message,
        // leaving the second showing `delivered` for the rest of the run.
        let mut deque = pending(&[("first", "continue"), ("second", "continue")]);

        assert_eq!(
            match_replay_echo(&mut deque, "continue"),
            Some("first".to_string()),
            "the FIRST delivered message is acked first"
        );
        assert_eq!(
            match_replay_echo(&mut deque, "continue"),
            Some("second".to_string()),
            "and the second echo acks the second message, not the first again"
        );
        assert!(deque.is_empty());
    }

    #[test]
    fn a_non_matching_echo_leaves_the_deque_untouched() {
        // The run's own command prompt is echoed exactly like an injected
        // message is, so this is the common case and not an edge one. An
        // implementation that popped the front on any echo would ack a message
        // the agent has not started, which is the failure D-07 names.
        let mut deque = pending(&[("id-1", "skip the UI review")]);

        assert_eq!(match_replay_echo(&mut deque, "/gsd-progress"), None);
        assert_eq!(
            deque.len(),
            1,
            "an unmatched echo must consume nothing: the message it did not \
             name is still waiting for its own"
        );

        // And the comparison is exact. Each of these differs from the stored
        // text only by something a lenient matcher would forgive, and each must
        // still miss (no trimming, no case folding, no normalisation).
        for near_miss in [
            " skip the UI review",
            "skip the UI review ",
            "Skip the UI review",
            "skip  the UI review",
        ] {
            assert_eq!(
                match_replay_echo(&mut deque, near_miss),
                None,
                "{near_miss:?} is not the text that was sent"
            );
        }
        assert_eq!(deque.len(), 1);
    }

    #[test]
    fn the_echo_text_comes_out_of_the_wire_body_only_when_the_replay_marker_is_true() {
        const TEXT: &str = "skip the UI review \u{1F680}";

        let echo = format!(
            r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"text","text":{}}}]}},"isReplay":true,"uuid":"echo-1"}}"#,
            serde_json::to_string(TEXT).expect("the text encodes")
        );
        assert_eq!(
            replay_echo_text(&echo).as_deref(),
            Some(TEXT),
            "a multi-byte body must come back byte for byte, or exact equality \
             can never match what was sent"
        );

        // A `user` message WITHOUT the marker is a tool result. Acking an
        // injected message from one would report the agent as having started on
        // the user's steering when it was in fact reporting a Bash exit code.
        let tool_result = format!(
            r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"text","text":{}}}]}}}}"#,
            serde_json::to_string(TEXT).expect("the text encodes")
        );
        assert_eq!(replay_echo_text(&tool_result), None);

        // Tolerant by construction: a shape no version we know emits yields
        // `None`, never a panic and never a parse failure that could end a run.
        assert_eq!(replay_echo_text("{not json"), None);
        assert_eq!(replay_echo_text(r#"{"isReplay":true}"#), None);
        assert_eq!(
            replay_echo_text(r#"{"isReplay":true,"message":{"content":"bare string"}}"#)
                .as_deref(),
            Some("bare string"),
            "`content` has shipped as a bare string as well as an array"
        );
    }

    #[test]
    fn a_delivered_message_awaiting_its_echo_is_never_reported_as_missed() {
        // The distinction Task 2's doc turns on, pinned as a test because the
        // two states are one word apart and mean opposite things: `missed` says
        // the message NEVER reached the agent, while an un-acked pending entry
        // says it reached the agent and the run ended before the agent got to
        // it. `drain_undelivered` therefore journals nothing at all.
        let mut acks = PendingAcks::default();
        acks.push_delivered("id-1".to_string(), "continue".to_string());
        acks.push_delivered("id-2".to_string(), "and again".to_string());

        assert_eq!(acks.match_echo("continue"), Some("id-1".to_string()));
        assert_eq!(
            acks.drain_undelivered(),
            vec!["id-2".to_string()],
            "only the message that never came back is left, and it is returned \
             for a COUNT — nothing here writes a journal record"
        );
    }

    /// D-11's idle interaction, asserted where the driver actually decides it.
    ///
    /// **The failure mode this guards is a run that hangs for fifteen minutes
    /// looking healthy.** Once stdin stays open for the life of a steerable run,
    /// an agent that goes quiet is held only by `ExecutionOptions`' idle cap —
    /// and the driver's terminal record is derived from the outcome that cap
    /// produces, through [`outcome_label`] and nothing else. So the property to
    /// pin is that the derivation reports the breach rather than laundering it
    /// into a success.
    ///
    /// The breach itself — an idle cap firing and producing
    /// [`RunOutcome::Stalled`] — is proved against a real silent child at
    /// `tests/executor_lifecycle.rs:435`. Reproducing that here would need a
    /// per-run idle-cap knob on `DriveArgs`, which is the new code path this
    /// test was asked *not* to add.
    #[test]
    fn a_run_idle_at_the_empty_inbox_step_is_reported_stalled_not_succeeded() {
        let stalled = outcome_label(&RunOutcome::Stalled {
            idle_for: Duration::from_secs(900),
        });

        assert_eq!(stalled, "stalled");
        for success in [
            outcome_label(&RunOutcome::SucceededWithChanges {
                turns: Vec::new(),
                total_cost_usd: None,
            }),
            outcome_label(&RunOutcome::SucceededNoChanges {
                turns: Vec::new(),
                total_cost_usd: None,
            }),
        ] {
            assert_ne!(
                stalled, success,
                "a run parked at the empty-inbox step for the idle cap must not \
                 reach the journal wearing a success label: `reconcile.rs` and \
                 the TUI both read this string, and a hang reported as a success \
                 is a hang nobody investigates"
            );
        }
    }

    #[test]
    fn the_terminate_diagnostic_code_is_a_stable_grep_target() {
        // The code travels into `journal.jsonl` verbatim and is what a later
        // reader searches for, so it is snake_case and carries no timestamp, pid
        // or run id — those live in the record around it.
        assert_eq!(TERMINATE_DIAGNOSTIC_CODE, "terminate_signal_shutdown");
        assert!(TERMINATE_DIAGNOSTIC_CODE
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '_'));
    }
}
