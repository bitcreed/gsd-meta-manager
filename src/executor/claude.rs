// The `claude` CLI transport.
//
// Design rationale, stated here because every one of these is a mistake a
// future reader makes without it:
//
// * **Async `tokio::process` via `process-wrap`; never called from
//   `spawn_blocking`.** The child is long-lived and duplex. The only blocking
//   work is the before/after project snapshot, which is explicitly moved onto
//   `spawn_blocking` because it does full-tree file I/O.
// * **Parsing happens in the reader task, never on the render thread.**
//   Internally-tagged serde enums buffer their content through an intermediate
//   representation, and a real run's stream is 41% high-frequency events. The
//   render loop never sees a raw line.
// * **Three pipe tasks, never two (D-04).** stdout reader, stderr reader and
//   stdin writer are independent tasks; the task awaiting process exit is a
//   fourth and owns no pipe. A task that both writes stdin and awaits exit
//   deadlocks: the child blocks writing stdout while the parent blocks writing
//   stdin. stdout and stderr are never merged — a non-empty stderr is itself a
//   diagnostic (the workspace-trust warning that silently voids project
//   `permissions.allow` arrives there and nowhere else).
// * **Teardown is `signal(15)` first (D-14).** `process-wrap`'s `start_kill()`
//   sends **SIGKILL**, and its `kill()` is `start_kill()` + `wait()` — so
//   reaching for `kill()` skips Claude's documented clean path entirely: no
//   turn abort, no Bash-tree teardown, no `SessionEnd` hooks, no exit 143.
//   SIGTERM to the group, grace, then SIGKILL, then an unconditional `wait()`
//   so no zombie survives.
// * **Process exit is never awaited unbounded (D-13), and here is the
//   mechanism.** Every bound this file owns is evaluated at the TOP of every
//   supervisor pass, not only when the `select!` happens to reach an arm: a
//   `biased;` macro with a hot stream arm never polls its later arms at all,
//   and the winning arm's body awaits outside the macro, where every timer
//   future has already been dropped. The hand-off to the caller is itself
//   bounded by the earliest armed deadline, so a consumer that stops draining
//   cannot park the supervisor with every cap and the cancel signal switched
//   off. The drain that follows an observed exit is bounded too, and that bound
//   is deliberately not guarded by the exited flag — a bound that switches off
//   when the child exits is not a bound. The cost of a consumer stalled past
//   the forward ceiling is one **counted, logged dropped event**; the benefit
//   is that a run always ends and always reports how.
//   The two caps are independent and measure different things: the wall-clock
//   cap measures time since spawn, and the idle cap measures time since the
//   **reader** last observed a line. Only the second can tell a legitimately
//   long run from a hung one — the research pass reproduced a hang that sat
//   silent for minutes, while a real multi-step run emits events continuously,
//   and time-since-spawn cannot separate those.
// * **The child's environment is scrubbed of every inherited `CLAUDE*`
//   variable.** This TUI is plausibly launched from inside a Claude Code
//   session, so those variables would otherwise leak into the driven child and
//   change `-p` behaviour in ways that look like "works on my machine". The one
//   variable this executor sets deliberately —
//   `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` — is set *after* the scrub, never
//   inherited from a silent default (D-14).
// * **Line framing is `BufReader` + an explicit byte bound, not a codec crate
//   (D-05).** `tokio-util` would be added for exactly one type; the
//   cancellation half of its justification dissolves once teardown already
//   closes the pipe. The byte bound is the mitigation, and it is applied while
//   reading rather than after, because a bound checked after the line is
//   already in memory bounds nothing.
// * **No raw stream content is logged at any level.** The log file under the
//   user's local share directory must not become an unredacted transcript of
//   everything the driven agent read. Raw lines travel to the caller in memory,
//   as events; the log gets counts and shapes only.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop, ProcessGroup};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tokio::time::Instant;
use uuid::Uuid;

use crate::error::{CapabilityError, SendError, SpawnError};
use crate::executor::gate::{self, GateOutcome};
use crate::executor::outcome::{derive_run_outcome_from_envelopes, RunSnapshot};
use crate::executor::stream_json::{
    parse_line, Envelope, ResultMessage, StreamMessage, SystemMessage, UserMessage,
};
use crate::executor::{
    encode_interrupt, BoxFuture, DrivableProject, ExecutionEvent, ExecutionHandle, ExecutionId,
    ExecutionOptions, ExecutionTarget, Executor, InterruptAck, PendingControl, PermissionMode,
    RunOutcome, SpawnProfile, TurnOutcome, WriterCommand,
};

/// Upper bound on one stream line, **in bytes**.
///
/// This is a byte count and deliberately not a character count, a code-point
/// count, a grapheme-cluster count, or any normalized form: the threat is
/// memory exhaustion from a subprocess line, and only bytes measure that.
/// `BufReader::lines()` is unbounded per line, so the bound is applied while
/// the line is being consumed — a line over the bound yields a truncation event
/// and the run continues rather than failing (T-15-07, D-05).
pub const MAX_LINE_BYTES: usize = 4 * 1024 * 1024;

/// How much of an oversize line is carried in the truncation event.
const TRUNCATION_PREFIX_BYTES: usize = 256;

/// SIGTERM, as a raw signal number. `ChildWrapper::signal` takes an `i32`, so
/// no `nix` or `libc` dependency is needed to reach the graceful path.
const SIGTERM: i32 = 15;

/// Grace between SIGTERM and SIGKILL on the process group (D-14).
const TEARDOWN_GRACE: Duration = Duration::from_secs(10);

/// How long a cleanly-ending run's process may take to exit after its stream
/// has already closed.
///
/// stdout EOF means every writer on that pipe is gone, so the child is already
/// on its way out, and the CLI caps its own exit drain at 30 seconds. A child
/// that has closed its stream and *still* has not exited is not ending cleanly.
/// This bound exists so that **no** path in this file awaits process exit
/// unbounded (D-13); when it expires the run escalates into the same four-step
/// teardown a cap breach uses.
///
/// **Not to be confused with [`POST_EXIT_DRAIN_CAP`], which is its mirror.**
/// The two bound opposite directions and the names are close enough that the
/// distinction is worth stating rather than leaving to be rediscovered:
///
/// * this one bounds *"the stream closed — is the process gone?"*;
/// * that one bounds *"the process is gone — is the stream closed?"*.
const EXIT_DRAIN_CAP: Duration = Duration::from_secs(30);

/// Capacity of the executor event channel.
///
/// Bounded, so a blocked consumer applies backpressure to the reader rather
/// than growing memory without limit. Generous on purpose: Claude caps its exit
/// drain at 30 seconds, so a slow consumer can truncate a run's tail.
const EVENT_CHANNEL_CAPACITY: usize = 8192;

/// Capacity of the stdin writer's command channel.
const WRITER_CHANNEL_CAPACITY: usize = 64;

/// An absolute ceiling on how long the supervisor may be parked handing **one**
/// event to its consumer.
///
/// The consumer is a TUI that can legitimately stop draining for a while — the
/// blocking `$EDITOR` shell-out is the known case — so this ceiling is generous
/// rather than tight. But it is finite, and that is the whole point: a
/// supervisor parked on a send is a supervisor whose wall-clock cap, idle cap,
/// grace and cancel signal are **all** disabled at once, because every one of
/// them lives in a `select!` the loop is no longer inside (CR-01). The per-pass
/// forward deadline is the earliest of this ceiling and every armed bound, so a
/// cap that expires mid-send unparks the supervisor at exactly the right
/// instant.
///
/// The cost of a stall longer than this is one **counted, logged** dropped
/// event. A dropped diagnostic is strictly preferable to an unbounded park: the
/// former loses a line of run history and says so, the latter loses the run.
pub const EVENT_FORWARD_TIMEOUT: Duration = Duration::from_secs(5);

/// How long the loop keeps draining already-framed stream lines after the child
/// **leader** has been observed exited.
///
/// The mirror of [`EXIT_DRAIN_CAP`], and routinely confused with it: that one
/// bounds *"the stream closed, is the process gone?"*, this one bounds *"the
/// process is gone, is the stream closed?"*.
///
/// Five seconds because once the leader has exited the remaining lines are
/// already framed and in flight, and the CLI terminates its background Bash
/// tasks about five seconds after the final result. A stream still open after
/// that is being held by something that outlived the leader — a backgrounded
/// grandchild that inherited stdout — which is precisely the condition this
/// bound exists to end.
///
/// It is deliberately **not** guarded by the exited flag. A bound that switches
/// off when the child exits is not a bound (CR-02).
pub const POST_EXIT_DRAIN_CAP: Duration = Duration::from_secs(5);

/// The `terminal_reason` a turn carries when an interrupt actually stopped it.
pub const ABORTED_STREAMING: &str = "aborted_streaming";

/// Whether an interrupt actually stopped a turn.
///
/// **This is the only honest answer to "did my interrupt work?", and it is
/// deliberately not derived from the acknowledgement.** The acknowledgement
/// cannot answer it, for two measured reasons:
///
/// * [`InterruptAck::subtype`] of `success` acknowledges that the *request was
///   accepted*, never that the thing the caller meant was cancelled. In golden
///   transcript 07 that acceptance arrived on the wire **before the target turn
///   had even been dequeued** — the replay echo follows it. Reporting a
///   cancellation there would have been a claim about a turn that had not
///   started.
/// * [`InterruptAck::still_queued`] is the authoritative statement of what
///   **remains queued** — the `interrupt_cancel_queued_v1` accounting surfacing
///   — and it is empty in *both* golden interrupt transcripts. An empty array
///   means nothing was waiting behind the interrupt; it is not evidence that
///   anything stopped.
///
/// What actually confirms a turn stopped arrives **later on the stream**: the
/// CLI flushes the partial assistant message, injects a synthetic
/// `[Request interrupted by user]` user message, and closes the turn with
/// `terminal_reason: "aborted_streaming"` (D-31, Pitfall D). A caller must
/// therefore keep watching the stream after the ack rather than reporting on
/// the ack.
///
/// Reporting an accepted-but-nothing-stopped-yet interrupt as a cancellation is
/// the repudiation threat T-15-18; this function is its mitigation.
pub fn interrupt_stopped_a_turn(turns: &[TurnOutcome]) -> bool {
    turns
        .iter()
        .any(|turn| turn.terminal_reason.as_deref() == Some(ABORTED_STREAMING))
}

/// Build the spawn argv.
///
/// Produced as a vector of owned OS strings with incremental pushes — never a
/// shell string and never a shell interpreter. `process-wrap` wraps
/// `tokio::process::Command`, which does not invoke a shell, so an alias or a
/// path containing a flag-shaped token cannot become a flag (T-15-06).
///
/// The baseline is exactly D-01 plus D-15: print mode, stream-json in and out,
/// verbose (**without it, stream-json emits nothing at all**), replay of user
/// messages (the only delivery ack the design has), an explicit session id,
/// project-only setting sources, `dontAsk` permissions, and
/// `--strict-mcp-config` so a project-local MCP config in a registered
/// third-party repository cannot introduce tools. The permission-bypass flag
/// and the bypass permission mode are never emitted on the host.
///
/// **Why the envelope's tool denylist rides here rather than only in the
/// settings file (D-06 layer 1, D-07).** Of the three enforcement layers this
/// is the cheapest — it costs one flag and fires *before* the tool runs, with
/// no process to spawn and no file to read. The decisive property is not the
/// cost though: it is that **argv cannot be silently dropped, because it is
/// argv rather than a file**. A `--settings` file that fails validation is
/// silently ignored in print mode with no error shown, so a run whose only
/// carrier was that file would be a run with the deny list quietly absent and
/// nothing at all saying so. The same controls therefore ride on both, and
/// `tests/envelope_pr_cap.rs`'s
/// `every_pattern_the_settings_file_denies_is_also_carried_on_argv` is what
/// keeps the two lists from drifting apart.
///
/// The settings path rides here too, because the file has to be *named* to be
/// loaded at all; naming it is not the same as depending on it.
pub fn build_argv(options: &ExecutionOptions) -> Vec<OsString> {
    let mut argv: Vec<OsString> = Vec::new();

    // Exhaustive on purpose: Phase 22's `Container` variant must land here as a
    // compile error rather than as a silently host-shaped argv.
    match options.target {
        ExecutionTarget::Host => {}
    }

    push(&mut argv, "-p");
    push(&mut argv, "--input-format");
    push(&mut argv, "stream-json");
    push(&mut argv, "--output-format");
    push(&mut argv, "stream-json");
    push(&mut argv, "--verbose");
    push(&mut argv, "--replay-user-messages");
    push(&mut argv, "--session-id");
    push(&mut argv, options.session_id.to_string());
    push(&mut argv, "--setting-sources");
    push(&mut argv, options.setting_sources.as_flag_value());
    push(&mut argv, "--permission-mode");
    push(&mut argv, options.permission_mode.as_flag_value());
    push(&mut argv, "--strict-mcp-config");

    if let Some(model) = &options.model {
        push(&mut argv, "--model");
        push(&mut argv, model);
    }
    if let Some(session) = &options.resume_session {
        // Always with an explicit value: a bare `-r` opens an interactive
        // picker, which under `-p` with no TTY is at best an error.
        push(&mut argv, "--resume");
        push(&mut argv, session);
    }
    if let Some(budget) = options.budget_usd {
        push(&mut argv, "--max-budget-usd");
        push(&mut argv, budget.to_string());
    }
    if let Some(name) = &options.name {
        push(&mut argv, "--name");
        push(&mut argv, name);
    }

    // Comma-joined into ONE value rather than pushed as a variadic run of
    // words. The installed CLI documents this flag as taking a "comma or space
    // separated list", and a space-separated variadic would keep consuming
    // words — including the next flag — if one were ever appended below.
    if !options.envelope_disallowed_tools.is_empty() {
        push(&mut argv, "--disallowedTools");
        push(&mut argv, options.envelope_disallowed_tools.join(","));
    }
    if let Some(settings) = &options.envelope_settings {
        push(&mut argv, "--settings");
        push(&mut argv, settings);
    }

    // Exhaustive on purpose, exactly as `options.target` is above: a third
    // profile must land here as a compile error rather than as a silently
    // executor-shaped spawn wearing a seam's name.
    //
    // Note what is deliberately NOT here. `--strict-mcp-config` stays
    // unconditional at its line above, for both profiles, and `--mcp-config` is
    // pushed under neither: with no config supplied the permitted MCP set is
    // empty, and supplying one would move it from empty to whatever that file
    // names. This is a regression guard rather than a feature, and
    // `tests/spawn_seam_guard.rs` fails in both directions on it.
    match &options.profile {
        SpawnProfile::Executor => {}
        SpawnProfile::ModelSeam { json_schema } => {
            // An EMPTY VALUE, not an absent flag. The CLI reduces the advertised
            // tool set to exactly `["StructuredOutput"]` when this is present
            // and empty; omitting it leaves the full set. The value is pushed as
            // its own argv word rather than joined, so an empty string reaches
            // the child as an empty argument rather than disappearing the way it
            // would through a shell.
            push(&mut argv, "--tools");
            push(&mut argv, "");
            // Inline JSON only. A path here is rejected at startup with exit 1
            // and nothing on stdout, so the schema is never written to a temp
            // file — see `SpawnProfile::ModelSeam::json_schema`.
            push(&mut argv, "--json-schema");
            push(&mut argv, json_schema);
        }
    }

    argv
}

fn push(argv: &mut Vec<OsString>, arg: impl AsRef<OsStr>) {
    argv.push(arg.as_ref().to_os_string());
}

/// Drives the `claude` CLI.
#[derive(Debug, Clone)]
pub struct ClaudeExecutor {
    program: PathBuf,
    leading_args: Vec<OsString>,
    /// Where the agent's process group id is published the instant the child
    /// exists, for a caller that may never receive an [`ExecutionHandle`].
    ///
    /// **The `Arc` is here so the struct keeps its derived `Clone` and `Debug`,
    /// and for no other reason** — a bare `oneshot::Sender` is neither. The slot
    /// is take-once by construction: [`start_run`](ClaudeExecutor::start_run)
    /// takes the sender out, and a `oneshot` can be sent on exactly once anyway,
    /// so a cloned executor started twice publishes for the first start and
    /// silently does not for the second. That is the honest behaviour for a
    /// channel whose receiver is a single driver's single run.
    spawn_observer: Arc<Mutex<Option<oneshot::Sender<u32>>>>,
    /// Where the **raw wire line** of every `user` replay echo is published, for
    /// a caller that has to correlate the echo against something it sent.
    ///
    /// A plain `Option<UnboundedSender>` rather than the `Arc<Mutex<Option<..>>>`
    /// its neighbour needs, because `mpsc::UnboundedSender` is already `Clone`
    /// and `Debug` and may be sent on many times — the take-once dance above
    /// exists only because a `oneshot::Sender` is neither.
    ///
    /// See [`ClaudeExecutor::observing_replay_echoes`] for why this carries the
    /// raw line rather than a projection of it.
    replay_observer: Option<mpsc::UnboundedSender<String>>,
}

impl Default for ClaudeExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ClaudeExecutor {
    /// Drive the `claude` binary found on `PATH`.
    pub fn new() -> Self {
        Self {
            program: PathBuf::from("claude"),
            leading_args: Vec::new(),
            spawn_observer: Arc::new(Mutex::new(None)),
            replay_observer: None,
        }
    }

    /// Drive a different program, with fixed leading arguments placed before
    /// the generated argv.
    ///
    /// This is how the transcript-replaying stand-in is driven in tests without
    /// spawning a real `claude`, and it keeps the fixture out of the production
    /// path entirely — no Cargo feature, no fixture branch in `main`.
    pub fn with_program(program: impl Into<PathBuf>, leading_args: Vec<OsString>) -> Self {
        Self {
            program: program.into(),
            leading_args,
            spawn_observer: Arc::new(Mutex::new(None)),
            replay_observer: None,
        }
    }

    /// Publish the agent's process group id on `tx` the instant the child exists.
    ///
    /// **A builder on the concrete type, deliberately not a method on the
    /// [`Executor`] trait.** That trait is what every Phase 15 caller programs
    /// against and what every future backend — a container, a remote runner —
    /// will implement; a Unix process-group id is a fact about *this* backend's
    /// spawn, and putting it in the portable contract would oblige a backend that
    /// has no process groups to describe one.
    ///
    /// **What makes the channel trustworthy is an invariant of `start_run`:** the
    /// child is spawned synchronously at `wrap.spawn()` and the pgid is read on
    /// the very next statement, with no `.await` between them. So a caller that
    /// receives nothing here can conclude the agent child does not exist — not
    /// that it exists and the message was late.
    ///
    /// The driver is the caller, and the failure this closes is CR-01: a stop
    /// that lands while `start` is still awaiting the capability gate has no
    /// [`ExecutionHandle`] and therefore no `pgid`, so it has no handle on the
    /// **agent's** group — which is a different process group from the driver's,
    /// because `claude` is spawned with `ProcessGroup::leader()`. Tearing down the
    /// group the signal reached is not tearing down the group that matters
    /// (D-06, D-09).
    pub fn observing_spawn(mut self, tx: oneshot::Sender<u32>) -> Self {
        self.spawn_observer = Arc::new(Mutex::new(Some(tx)));
        self
    }

    /// Publish the **raw wire line** of every `user` replay echo on `tx`.
    ///
    /// A builder on the concrete type for the same reason as
    /// [`observing_spawn`](ClaudeExecutor::observing_spawn): `--replay-user-messages`
    /// is a fact about *this* backend's protocol, and a future container or
    /// remote backend should not be obliged to describe one.
    ///
    /// **It carries the RAW LINE, not a projection of it, and that is the whole
    /// decision** (D-08). The only consumer is the driver, whose job is to
    /// correlate the echo against the text it sent; correlating on a rendered
    /// string is exactly the screen-scraping D-01 forbids in another guise, so
    /// this layer hands over the bytes it received and interprets none of them.
    /// The one thing it *does* interpret is the envelope's own
    /// `is_replay` marker, because that is what decides whether a line belongs
    /// on this channel at all — and this layer already has the parsed envelope,
    /// so asking it that question costs nothing and keeps every non-echo line
    /// off the channel entirely.
    ///
    /// **Unbounded, deliberately.** A bounded channel here would put the
    /// coordinator — the task that owns the caps, the cancel and the teardown —
    /// at the mercy of a consumer that is slow to drain, which is the failure
    /// [`forward`]'s deadline exists to prevent on the event channel. The queue
    /// is bounded in practice by the number of user messages one run sends, and
    /// the driver drains it on every pass of its own loop.
    ///
    /// A closed receiver is ignored: the driver may have finished normally and
    /// dropped it, which is not an error and is not this function's business.
    pub fn observing_replay_echoes(mut self, tx: mpsc::UnboundedSender<String>) -> Self {
        self.replay_observer = Some(tx);
        self
    }

    async fn start_run(
        &self,
        project: &DrivableProject,
        command: String,
        options: ExecutionOptions,
    ) -> Result<ExecutionHandle, SpawnError> {
        let root = project.root().to_path_buf();
        if !root.is_dir() {
            return Err(SpawnError::ProjectRootUnusable { root });
        }

        let first_message = serde_json::to_string(&UserMessage::text(command))
            .map_err(|source| SpawnError::EncodeCommand { source })?;

        // Captured before spawn so the delta has a floor even if the agent's
        // very first act is a write.
        let before = capture_snapshot(root.clone()).await;

        let mut argv = self.leading_args.clone();
        argv.extend(build_argv(&options));

        let program = self.program.clone();
        let cwd = root.clone();
        let bg_ceiling = options.bg_wait_ceiling_ms.to_string();
        // Cloned out before the closure for the same reason `bg_ceiling` is:
        // the closure is `move` and `options` is still needed below.
        let envelope_env = options.envelope_env.clone();
        let profile = options.profile.clone();

        let mut wrap = CommandWrap::with_new(&program, |cmd| {
            cmd.args(&argv)
                .current_dir(&cwd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            // The TUI is plausibly launched from inside a Claude Code session,
            // so inherited CLAUDE* variables would leak into the driven child
            // and change `-p` behaviour in ways that look like "works on my
            // machine". Scrub them all, then set the ones we mean to set —
            // one unconditionally, and two more under the model-seam profile.
            //
            // **The envelope extends that identical argument from the CLAUDE*
            // family to git and ssh** (D-09, D-16). An inherited `SSH_AUTH_SOCK`
            // is the shortest path from a driven run to the user's own keys, and
            // an inherited `GIT_CONFIG_GLOBAL` is the shortest path to their
            // credential helper — the same "works on my machine" failure with a
            // blast radius instead of a support ticket. **This closure is the
            // ONE place in the tree that builds the child's environment**, so
            // the envelope is applied here and nowhere else; a second applier is
            // a second thing that can disagree about what the child inherits.
            for (key, _) in std::env::vars_os() {
                if key.to_string_lossy().starts_with("CLAUDE") {
                    cmd.env_remove(&key);
                }
            }
            cmd.env("CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS", &bg_ceiling);

            // Exhaustive, no wildcard — the same discipline `build_argv` uses on
            // this discriminant.
            match &profile {
                SpawnProfile::Executor => {}
                SpawnProfile::ModelSeam { .. } => {
                    // Two variables, and they are DISTINCT INSTRUCTIONS for
                    // opposite reasons. Spelling that out because the pair reads
                    // like one idea and is two:
                    //
                    // 1. `CLAUDE_CODE_DISABLE_CLAUDE_MDS` carries the `CLAUDE`
                    //    prefix, so the loop directly above ALREADY REMOVED it.
                    //    Setting it here is a re-set, exactly as
                    //    `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` one line up is.
                    //    It is the control for the finding that the CLI loads the
                    //    target repository's `CLAUDE.md` into the model's context
                    //    ITSELF, outside any boundary this driver builds — so no
                    //    amount of careful prompt construction closes it, because
                    //    the prompt is not where it enters.
                    //
                    // 2. `MAX_STRUCTURED_OUTPUT_RETRIES` does NOT carry that
                    //    prefix. It SURVIVES the scrub, so assignment rather than
                    //    removal is the control: an ambient
                    //    `MAX_STRUCTURED_OUTPUT_RETRIES=50` in the operator's
                    //    shell would otherwise be inherited, and one driver-side
                    //    escalation would silently become up to that many model
                    //    turns — which makes the per-run escalation count a lie.
                    //
                    // **There is NO on-the-wire signal that either took effect.**
                    // The init envelope reports the tool set and the MCP server
                    // list, and it reports nothing at all about `CLAUDE.md`
                    // suppression or the retry ceiling. The guard for these two is
                    // the argv/env shape asserted in `tests/spawn_seam_guard.rs`
                    // plus the end-to-end corpus fixture in plan 21-05 — never a
                    // field read back off the stream, because there is none, and
                    // claiming otherwise would be the unearned assurance this
                    // codebase's honesty conventions exist to prevent.
                    cmd.env("CLAUDE_CODE_DISABLE_CLAUDE_MDS", "1");
                    cmd.env("MAX_STRUCTURED_OUTPUT_RETRIES", "1");
                }
            }

            // The `EnvelopeEnv` value `envelope::cred::build_env` produced, applied
            // one entry at a time. Removal and assignment are distinct
            // instructions and the `Option` in `EnvelopeVar` is what keeps them
            // distinct: `SSH_AUTH_SOCK=""`
            // is a variable an agent can notice and work around, while an absent
            // one is absent. Matching here rather than collapsing to `env` is
            // the whole reason that type carries an `Option` (D-16).
            if let Some(envelope) = &envelope_env {
                for (key, value) in envelope.entries() {
                    match value {
                        Some(value) => {
                            cmd.env(key, value);
                        }
                        None => {
                            cmd.env_remove(key);
                        }
                    }
                }
            }
        });
        wrap.wrap(ProcessGroup::leader());
        // Backstop only, never the teardown story: `KillOnDrop` is SIGKILL.
        wrap.wrap(KillOnDrop);

        let mut child = wrap.spawn().map_err(|source| SpawnError::Launch {
            program: program.display().to_string(),
            source,
        })?;

        // With a process-group leader wrapper the child *is* the group leader,
        // so the pgid equals the child pid. The concrete group type is not
        // reachable through the returned trait object, so this is how the pgid
        // is obtained — and it is recorded immediately, because a teardown with
        // no group handle is not a teardown.
        let pgid = child.id().ok_or(SpawnError::PidUnavailable)?;

        // Published HERE, before the gate is awaited, and that position is the
        // whole point (CR-01). The `ExecutionHandle` this function eventually
        // returns also carries the pgid — but a caller parked at `gate_rx.await`
        // below has no handle yet, and the documented hook hang
        // (`src/executor/mod.rs:225-239`) makes that window minutes rather than
        // microseconds. A driver stopped inside that window without this channel
        // has nothing to tear down but its OWN group, and `claude` leads a group
        // of its own — so the group that was signalled is not the group that had
        // to go (D-06, D-09).
        //
        // A closed receiver is ignored: the driver may have finished normally and
        // dropped it, which is not an error and is not this function's business.
        if let Some(observer) = self.spawn_observer.lock().await.take() {
            let _ = observer.send(pgid);
        }

        let stdin = child
            .stdin()
            .take()
            .ok_or(SpawnError::PipeUnavailable { pipe: "stdin" })?;
        let stdout = child
            .stdout()
            .take()
            .ok_or(SpawnError::PipeUnavailable { pipe: "stdout" })?;
        let stderr = child
            .stderr()
            .take()
            .ok_or(SpawnError::PipeUnavailable { pipe: "stderr" })?;

        let (events_tx, events_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let (reader_tx, reader_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let (writer_tx, writer_rx) = mpsc::channel(WRITER_CHANNEL_CAPACITY);
        let (gate_tx, gate_rx) = oneshot::channel();
        let (outcome_tx, outcome_rx) = oneshot::channel();
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // The idle cap's clock. It is published by the READER tasks and read by
        // the supervisor, never derived from any duration field in a terminal
        // envelope: `duration_api_ms` aggregates across parallel API calls and
        // was measured EXCEEDING the wall-clock `duration_ms` in a clean run,
        // and in any case both fields arrive only *with* the envelope, which is
        // far too late to serve as a stuck detector (D-13, Pitfall E).
        //
        // `Arc` rather than a cloned sender because `watch::Sender` is not
        // `Clone`, and both readers publish into the same slot.
        let (last_line_at, last_line_rx) = watch::channel(Instant::now());
        let last_line_at = Arc::new(last_line_at);

        let pending_control: PendingControl = Arc::new(Mutex::new(Default::default()));
        let running = Arc::new(AtomicBool::new(true));

        tokio::spawn(read_stdout(stdout, reader_tx, Arc::clone(&last_line_at)));
        tokio::spawn(read_stderr(stderr, events_tx.clone(), last_line_at));
        tokio::spawn(write_stdin(stdin, writer_rx));

        tokio::spawn(
            Coordinator {
                child,
                reader_rx,
                events_tx,
                writer_tx: writer_tx.clone(),
                pending_control: Arc::clone(&pending_control),
                gate_tx,
                outcome_tx,
                cancel_rx,
                running: Arc::clone(&running),
                first_message,
                permission_mode: options.permission_mode,
                wall_clock_cap: options.wall_clock_cap,
                idle_cap: options.idle_cap,
                prompt_release_grace: options.prompt_release_grace,
                last_line_rx,
                before,
                project_root: root,
                replay_observer: self.replay_observer.clone(),
            }
            .run(),
        );

        // Block on the gate verdict: a refusal must be a start-time error, not
        // a mid-run surprise. That much is unconditional.
        //
        // **What a refusal COSTS is conditional, and the condition is the
        // child's own announce timing (260908-uqq).** The prompt travels over
        // stdin, and the supervisor releases it either when the gate passes or
        // when `prompt_release_grace` expires with nothing announced —
        // whichever comes first:
        //
        //   * **eager arm** — the CLI announced within the grace (2.1.220's
        //     shape). The gate rules before a single byte reaches stdin, so a
        //     refusal here still costs zero tokens and zero quota (D-06).
        //   * **late arm** — the CLI announced only after reading a user
        //     message (2.1.266's measured shape). The grace released the prompt
        //     first, so the init being judged is the one the prompt provoked and
        //     a refusal here aborts a turn that has already begun. It still
        //     refuses, it still returns `SpawnError::Capability`, and the
        //     Coordinator still tears the process group down — but the spend
        //     has already started.
        //
        // Withholding the prompt unconditionally is not an available
        // alternative: 2.1.266 emits nothing at all until a user message
        // arrives, so waiting for its init is a deadlock, and eliciting one
        // with a probe message would itself be a turn.
        let facts = match gate_rx.await {
            Ok(Ok(facts)) => facts,
            Ok(Err(err)) => return Err(err),
            Err(_) => return Err(SpawnError::InitNeverObserved),
        };

        Ok(ExecutionHandle {
            id: ExecutionId(Uuid::new_v4()),
            session_id: options.session_id.to_string(),
            events: events_rx,
            capabilities: facts.capabilities,
            pgid,
            claude_code_version: facts.claude_code_version.unwrap_or_default(),
            pending_control,
            control_response_cap: options.control_response_cap,
            stdin_tx: writer_tx,
            running,
            cancel_tx: Some(cancel_tx),
            outcome_rx: Some(outcome_rx),
            outcome: None,
            next_control_seq: 0,
        })
    }
}

impl Executor for ClaudeExecutor {
    fn start<'a>(
        &'a self,
        project: &'a DrivableProject,
        command: String,
        options: ExecutionOptions,
    ) -> BoxFuture<'a, Result<ExecutionHandle, SpawnError>> {
        Box::pin(self.start_run(project, command, options))
    }

    /// Write one NDJSON user message to the held child stdin and flush.
    ///
    /// **Written directly, with no driver-side turn-boundary flush buffer
    /// (D-31).** The committed architecture pass prescribed one, on the belief
    /// that a mid-turn message is ignored *and* lost from history. The phase
    /// spike refutes the second half on 2.1.220: the message is **queued and
    /// executed as its own turn**. The CLI already performs exactly the
    /// buffering that workaround prescribed, so a driver-side duplicate would
    /// buy nothing and would make the `still_queued` accounting harder to
    /// reason about.
    ///
    /// **The replay echo is a "started processing" acknowledgement, not a
    /// "received" acknowledgement.** The `--replay-user-messages` echo carries
    /// `isReplay: true` and is emitted at **dequeue**, not at receipt: the
    /// spike wrote a message ~12 seconds into a run and saw it echoed 45
    /// milliseconds *after the previous turn's terminal envelope*, roughly 55
    /// seconds later. Reading it as a delivery receipt is the easy mistake, and
    /// it is exactly the distinction Phase 18's queued → delivered → acted-on
    /// display is built on.
    ///
    /// The stdin handle is never closed or dropped here. It stays alive for the
    /// whole run; dropping it is the EOF that ends the run cleanly, and EOF
    /// means "no more input", not "stop" — the spike closed stdin 14 seconds
    /// into a 71-second run and Claude drained its queue, finished, and exited
    /// 0. Use [`ExecutionHandle::close_input`] for that, deliberately.
    fn send<'a>(
        &'a self,
        handle: &'a mut ExecutionHandle,
        message: UserMessage,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async move {
            let line = serde_json::to_string(&message).map_err(SendError::Encode)?;
            handle.write_line(line).await
        })
    }

    /// Write an interrupt as a `control_request` and await its correlated
    /// response.
    ///
    /// Only the request-and-response form is ever written. The single-field
    /// interrupt form that community sources describe was empirically refuted
    /// on 2.1.220 — it produces no response and has no effect — so it is never
    /// emitted; see [`crate::executor::encode_interrupt`].
    ///
    /// Correlation is explicit: a oneshot is registered in the handle's
    /// `pending_control` map under a caller-generated request id, and the
    /// reader resolves it only on a response carrying that same id. Nothing
    /// here assumes the next response on the wire is this request's.
    ///
    /// **Read the result honestly.** The returned [`InterruptAck`] carries the
    /// acceptance subtype and the `still_queued` list read from the doubly
    /// nested response field, and *neither* is a statement that anything was
    /// cancelled — see [`interrupt_stopped_a_turn`], which is the check a
    /// caller must use before telling a user their interrupt worked.
    fn interrupt<'a>(
        &'a self,
        handle: &'a mut ExecutionHandle,
    ) -> BoxFuture<'a, Result<InterruptAck, SendError>> {
        Box::pin(async move {
            if !handle.is_running() {
                return Err(SendError::NotRunning);
            }
            let request_id = handle.next_control_id();
            let (tx, rx) = oneshot::channel();
            handle
                .pending_control
                .lock()
                .await
                .insert(request_id.clone(), tx);

            let line = encode_interrupt(&request_id)?;
            handle.write_line(line).await?;

            // The cap is `Copy`, so reading it here creates no borrow that
            // outlives the expression and the map lock below stays legal.
            match tokio::time::timeout(handle.control_response_cap, rx).await {
                // Correlated on the request id, never on arrival order.
                Ok(Ok(response)) => Ok(InterruptAck::from_response(&response)),
                // The sender was dropped: the run ended and its drain released
                // every waiter. A finished run can never answer a control
                // request, so there is nothing left to wait for.
                Ok(Err(_)) => {
                    handle.pending_control.lock().await.remove(&request_id);
                    Err(SendError::ControlResponseLost { request_id })
                }
                // The cap elapsed against a child that is still alive and
                // simply did not answer. A bounded, honest "lost" beats a
                // confident wrong answer: an acknowledgement is acceptance and
                // never cancellation, so inventing one here would be the
                // repudiation threat itself (D-13, D-31).
                Err(_) => {
                    handle.pending_control.lock().await.remove(&request_id);
                    Err(SendError::ControlResponseLost { request_id })
                }
            }
        })
    }

    fn cancel<'a>(&'a self, handle: &'a mut ExecutionHandle) -> BoxFuture<'a, RunOutcome> {
        Box::pin(async move {
            if let Some(tx) = handle.cancel_tx.take() {
                let _ = tx.send(());
            }
            handle.wait_outcome().await
        })
    }

    fn is_running(&self, handle: &ExecutionHandle) -> bool {
        handle.is_running()
    }

    fn capabilities<'a>(&self, handle: &'a ExecutionHandle) -> &'a [String] {
        &handle.capabilities
    }
}

/// Capture a project snapshot without touching the async reactor thread.
async fn capture_snapshot(root: PathBuf) -> RunSnapshot {
    let fallback = root.clone();
    tokio::task::spawn_blocking(move || RunSnapshot::capture(&root))
        .await
        .unwrap_or_else(|_| RunSnapshot::capture(&fallback))
}

/// What a reader task hands to the coordinator.
#[derive(Debug)]
enum ReaderItem {
    /// A framed line, already parsed in the reader task.
    Envelope(Envelope),
    /// A line over `MAX_LINE_BYTES`, discarded rather than parsed.
    Truncated { bytes: usize, prefix: String },
}

/// One framed line, or the reason there is not one.
#[derive(Debug)]
enum BoundedLine {
    Line(String),
    Truncated { bytes: usize, prefix: String },
    Eof,
}

/// Read one newline-terminated line, bounded in **bytes**.
///
/// The bound is enforced while consuming, so an unbounded single line from the
/// subprocess is drained to its newline without ever being fully retained.
async fn read_bounded_line<R>(reader: &mut R) -> std::io::Result<BoundedLine>
where
    R: AsyncBufRead + Unpin,
{
    let mut buf: Vec<u8> = Vec::new();
    let mut total_bytes: usize = 0;
    let mut overflowed = false;
    let mut saw_any = false;

    loop {
        let (consumed, found_newline, chunk) = {
            let available = reader.fill_buf().await?;
            if available.is_empty() {
                break;
            }
            match available.iter().position(|b| *b == b'\n') {
                Some(index) => (index + 1, true, available[..index].to_vec()),
                None => (available.len(), false, available.to_vec()),
            }
        };
        reader.consume(consumed);
        saw_any = true;
        total_bytes += chunk.len();

        let room = MAX_LINE_BYTES.saturating_sub(buf.len());
        if room >= chunk.len() {
            buf.extend_from_slice(&chunk);
        } else {
            buf.extend_from_slice(&chunk[..room]);
            overflowed = true;
        }

        if found_newline {
            break;
        }
    }

    if !saw_any {
        return Ok(BoundedLine::Eof);
    }
    if overflowed {
        let cut = buf.len().min(TRUNCATION_PREFIX_BYTES);
        return Ok(BoundedLine::Truncated {
            bytes: total_bytes,
            prefix: String::from_utf8_lossy(&buf[..cut]).into_owned(),
        });
    }
    // Tolerate CRLF the way `BufReader::lines()` does.
    if buf.last() == Some(&b'\r') {
        buf.pop();
    }
    Ok(BoundedLine::Line(
        String::from_utf8_lossy(&buf).into_owned(),
    ))
}

/// The stdout reader task. Parsing happens here and nowhere else.
///
/// It also owns the idle cap's clock: every observed line stamps `last_line_at`
/// with the instant the reader saw it, and the supervisor re-arms its idle timer
/// from that stamp (D-13). The stamp is written *before* the item is forwarded,
/// so a line that is merely slow to be consumed still counts as liveness — the
/// question the idle cap answers is "is the child still producing?", not "is the
/// driver still keeping up?".
async fn read_stdout(
    stdout: tokio::process::ChildStdout,
    tx: mpsc::Sender<ReaderItem>,
    last_line_at: Arc<watch::Sender<Instant>>,
) {
    let mut reader = BufReader::new(stdout);
    loop {
        let observed = read_bounded_line(&mut reader).await;
        if !matches!(observed, Ok(BoundedLine::Eof)) {
            last_line_at.send_replace(Instant::now());
        }
        match observed {
            Ok(BoundedLine::Eof) => break,
            Ok(BoundedLine::Line(line)) => {
                if line.trim().is_empty() {
                    continue;
                }
                if tx.send(ReaderItem::Envelope(parse_line(&line))).await.is_err() {
                    break;
                }
            }
            Ok(BoundedLine::Truncated { bytes, prefix }) => {
                // Counts and shapes only; never the content.
                tracing::warn!(
                    "claude stdout line of {} bytes exceeded the {} byte bound and was discarded",
                    bytes,
                    MAX_LINE_BYTES
                );
                if tx
                    .send(ReaderItem::Truncated { bytes, prefix })
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(err) => {
                tracing::warn!("claude stdout read failed: {}", err);
                break;
            }
        }
    }
}

/// The stderr reader task. Never merged with stdout.
///
/// It stamps `last_line_at` too, deliberately: a child writing diagnostics is a
/// child that is alive, and the idle cap's job is to kill runs that have gone
/// *silent*. Counting only stdout would let a run that is loudly retrying on
/// stderr be torn down as stalled.
async fn read_stderr(
    stderr: tokio::process::ChildStderr,
    tx: mpsc::Sender<ExecutionEvent>,
    last_line_at: Arc<watch::Sender<Instant>>,
) {
    let mut reader = BufReader::new(stderr);
    loop {
        let observed = read_bounded_line(&mut reader).await;
        if !matches!(observed, Ok(BoundedLine::Eof)) {
            last_line_at.send_replace(Instant::now());
        }
        match observed {
            Ok(BoundedLine::Eof) => break,
            Ok(BoundedLine::Line(line)) => {
                if tx.send(ExecutionEvent::Stderr(line)).await.is_err() {
                    break;
                }
            }
            Ok(BoundedLine::Truncated { bytes, prefix }) => {
                if tx
                    .send(ExecutionEvent::LineTruncated { bytes, prefix })
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(err) => {
                tracing::warn!("claude stderr read failed: {}", err);
                break;
            }
        }
    }
}

/// The stdin writer task. Never the task awaiting exit (D-04).
async fn write_stdin(
    mut stdin: tokio::process::ChildStdin,
    mut rx: mpsc::Receiver<WriterCommand>,
) {
    while let Some(command) = rx.recv().await {
        match command {
            WriterCommand::Line(line) => {
                if stdin.write_all(line.as_bytes()).await.is_err()
                    || stdin.write_all(b"\n").await.is_err()
                    || stdin.flush().await.is_err()
                {
                    tracing::warn!("claude stdin write failed; the child is no longer reading");
                    break;
                }
            }
            WriterCommand::Close => break,
        }
    }
    // Dropping the handle is the EOF signal. EOF means "no more input", not
    // "stop": the CLI drains its queued turns, finishes them, and exits.
}

/// Owns the child and the run's state machine.
struct Coordinator {
    child: Box<dyn ChildWrapper>,
    reader_rx: mpsc::Receiver<ReaderItem>,
    events_tx: mpsc::Sender<ExecutionEvent>,
    writer_tx: mpsc::Sender<WriterCommand>,
    pending_control: PendingControl,
    gate_tx: oneshot::Sender<Result<GateOutcome, SpawnError>>,
    outcome_tx: oneshot::Sender<RunOutcome>,
    cancel_rx: oneshot::Receiver<()>,
    running: Arc<AtomicBool>,
    first_message: String,
    /// What the argv asked for, so the gate can confirm the flag took effect
    /// against what `system/init` reports back (D-15).
    permission_mode: PermissionMode,
    /// Total time since spawn this run may take (D-13).
    wall_clock_cap: Duration,
    /// Time since the reader last observed a line this run may go silent for
    /// (D-13). Strictly greater than the background-subagent wait ceiling; see
    /// [`Coordinator::run`].
    idle_cap: Duration,
    /// How long to wait for the child's `system/init` before releasing the
    /// first user message anyway (260908-uqq). Carried from
    /// [`ExecutionOptions::prompt_release_grace`], where the reasoning lives.
    ///
    /// It is what decides which of the two refusal arms a run takes, and
    /// therefore what a capability refusal costs on this run: a CLI that
    /// announces inside the grace is refused before anything reaches its stdin,
    /// a CLI that announces later is refused after the prompt it was sent.
    prompt_release_grace: Duration,
    /// The instant the reader last observed a line. The idle arm re-arms from
    /// this and from nothing else (D-13, Pitfall E).
    last_line_rx: watch::Receiver<Instant>,
    before: RunSnapshot,
    project_root: PathBuf,
    /// Where the raw line of every `user` replay echo is republished, when a
    /// caller asked for it (see [`ClaudeExecutor::observing_replay_echoes`]).
    replay_observer: Option<mpsc::UnboundedSender<String>>,
}

/// Which cap the supervisor breached.
///
/// The two are kept distinct all the way to the outcome because they mean
/// different things to a user and to Phase 20's router: "this run was too long"
/// is a budgeting problem, "this run went silent" is a hang.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Breach {
    /// Time since spawn exceeded the total wall-clock cap.
    WallClock,
    /// Time since the reader's last observed line exceeded the idle cap.
    Idle,
}

impl Coordinator {
    /// The supervisor.
    ///
    /// **Two layers, and keeping them apart is the whole design.** The
    /// `tokio::select!` is the WAKE-UP mechanism: it parks the loop until
    /// something worth looking at happens — a stream line, the child's exit,
    /// one of the timers, the cancel signal. The unconditional block at the top
    /// of every pass is the ENFORCEMENT mechanism: it reads the wall-clock cap,
    /// the idle cap, the grace, the post-exit drain bound and the cancel signal
    /// on *every* pass, whatever the `select!` did or did not reach.
    ///
    /// Arm ordering is therefore a stream-fidelity preference and never a
    /// correctness dependency. That distinction is not decorative. Under the
    /// single-layer shape this loop used to have, `biased;` with the reader
    /// first meant a child emitting faster than the loop retired kept arm 1
    /// permanently ready and the later arms were never polled at all; and once
    /// the reader arm won, the loop sat awaiting a send *outside* the macro,
    /// with every timer future already dropped. Either way both caps and the
    /// cancel were unenforceable exactly when a runaway run made them matter
    /// (CR-01). The forward to the caller is bounded by the earliest armed
    /// deadline for the same reason.
    ///
    /// Racing the exit future is also what resolves the borrow problem the
    /// wrapper's API creates: `wait()` holds a `&mut` borrow of the child for
    /// the whole life of its future, so a supervisor holding that future across
    /// an await could never call `signal()` on the same object. When another arm
    /// wins, `select!` drops every loser — including the exit future — and the
    /// borrow ends before the statement after the macro runs, which is exactly
    /// where the signal call lives.
    ///
    /// **An observed exit is not a reaped group.** `ProcessGroupChild::wait`
    /// awaits the leader and caches its status before reaping the rest of the
    /// group, so a partially-polled wait future dropped by another arm winning
    /// — which the tail of lines after the leader exits makes likely — leaves
    /// that status cached and the next `wait()` returning immediately, with
    /// group members still alive. `exited` therefore means "the leader is gone",
    /// never "nothing of this run is still running", and the post-exit path
    /// bounds its drain and tears the group down whenever EOF has not proven
    /// otherwise (CR-02).
    ///
    /// **The idle cap is driven by the reader, not by the envelope (D-13,
    /// Pitfall E).** Every observed line stamps `last_line_at`; this loop
    /// re-computes its idle deadline from that stamp on every pass, so a run
    /// that keeps emitting keeps pushing its own deadline out and only a run
    /// that goes *silent* trips it. Time-since-spawn cannot make that
    /// distinction, and the duration fields on the terminal envelope arrive far
    /// too late to try.
    ///
    /// **Why the two default numbers are what they are.** The wall-clock cap is
    /// four hours and is a frank guess: no tuning data exists yet. The idle cap
    /// is fifteen minutes and is *derived*, not guessed — the CLI waits for
    /// background subagents up to `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS`, which
    /// this executor sets explicitly to ten minutes, and a healthy run can
    /// therefore legitimately emit nothing for that long. **The idle cap must
    /// stay strictly greater than that ceiling** so the ceiling always fires
    /// first; an idle cap at or below it would kill healthy runs. When retuning,
    /// preserve that ordering, not the numbers.
    ///
    /// **What these caps do NOT bound.** They bound a single `claude`
    /// invocation. Run-level step, wall-clock and no-progress caps across a
    /// multi-step run belong to Phase 20, and mistaking one for the other is the
    /// easy error. Likewise `budget_usd` stays plumbed to the flag and nothing
    /// more (D-16): it is a post-turn circuit breaker that cannot prevent the
    /// turn it fires on from spending, so no cost enforcement is built on it.
    async fn run(self) {
        let Coordinator {
            mut child,
            mut reader_rx,
            events_tx,
            writer_tx,
            pending_control,
            gate_tx,
            outcome_tx,
            mut cancel_rx,
            running,
            first_message,
            permission_mode,
            wall_clock_cap,
            idle_cap,
            prompt_release_grace,
            last_line_rx,
            before,
            project_root,
            replay_observer,
        } = self;

        let mut gate_tx = Some(gate_tx);
        let mut gated = false;
        // Whether the first user message has gone to the writer. Two paths set
        // it — the gate passing in `handle_item`, and the grace expiring in the
        // enforcement block below — and it is what keeps the prompt written
        // exactly once whichever of them fires first (260908-uqq).
        let mut prompt_released = false;
        let mut cancelled = false;
        let mut refused: Option<CapabilityError> = None;
        let mut breach: Option<Breach> = None;
        // The **full** terminal envelopes, not a projection of them. Only the
        // envelope carries `permission_denials[]`, and a run that was blocked
        // by `--permission-mode dontAsk` is otherwise indistinguishable from a
        // clean one: every verdict field on it says success. Collecting the
        // projection here is what reported a refused run as a success (CR-04).
        let mut envelopes: Vec<ResultMessage> = Vec::new();

        // Observed by the exit arm rather than by the teardown, so a child that
        // ended on its own is never signalled and never waited on twice.
        let mut exited = false;
        let mut exit_status: Option<ExitStatus> = None;

        // Step 2 of the teardown, running while the loop keeps draining. Set
        // when a cancel takes step 1; when it expires the drain stops and the
        // escalation follows. Without it a child that ignores the terminate
        // signal is drained forever and the teardown never reaches step 3.
        let mut grace_deadline: Option<Instant> = None;

        // Armed by the exit arm and by nothing else, and read by the loop head
        // and by an arm that is NOT guarded on `exited`: an observed exit is
        // what starts this clock, but it must never be what stops it (CR-02).
        let mut drain_deadline: Option<Instant> = None;
        let mut drain_expired = false;

        // Events lost because a stalled consumer did not take them inside the
        // forward bound. Counted so the loss is reportable: the count reaches
        // the log as it accumulates, and now also the stream — once, at the
        // end, via `report_dropped`, because a journal reads the stream and not
        // the log (D-33). The count is the only thing that ever leaves this
        // loop; the lost events themselves never do (T-15-53).
        let mut dropped_events: u64 = 0;

        let wall_deadline = Instant::now() + wall_clock_cap;
        // Computed once, from spawn, alongside the wall-clock deadline: it
        // bounds how long the child gets to introduce itself, and that clock
        // starts at spawn and is never re-armed by anything.
        let prompt_deadline = Instant::now() + prompt_release_grace;

        loop {
            let mut stop = false;
            let mut terminate = false;
            // Re-armed on every pass from the instant the READER recorded.
            let idle_deadline = *last_line_rx.borrow() + idle_cap;

            // ================================================================
            // Enforcement. Every bound this supervisor owns is evaluated HERE,
            // unconditionally, on every pass — never only when the `select!`
            // below happens to reach an arm.
            //
            // The `select!` is `biased;` with the reader first, so a child that
            // emits faster than the loop retires keeps arm 1 permanently ready
            // and the later arms are never polled at all; and the reader arm's
            // body awaits OUTSIDE the macro, so while that await is pending
            // every timer future has already been dropped. Either way the caps
            // and the cancel were unenforceable exactly when they mattered
            // most (CR-01). Hoisting them here makes arm ordering a
            // stream-fidelity preference rather than a correctness dependency.
            // ================================================================
            let now = Instant::now();

            // The startup handshake's tie-breaker (260908-uqq). CLI 2.1.266
            // emits `system/init` only *after* a user message arrives on stdin
            // — measured with stdout and stderr both at zero bytes against an
            // open, empty stdin — so an executor that withholds the prompt
            // until the gate has judged an init deadlocks against it and only
            // the idle cap ever breaks the tie, fifteen minutes later, reported
            // as a spawn failure. When the grace expires with nothing
            // announced, the prompt goes out anyway and the gate becomes a
            // POST-HOC check on this run: it still refuses, still stops the
            // loop and still tears the group down, but the turn it aborts has
            // already begun. On a CLI that announces inside the grace nothing
            // changes and the refusal still costs zero tokens (D-06).
            //
            // **`try_send`, never an awaited send, and the flag is set BEFORE
            // the result is inspected.** An awaited send here would park the
            // enforcement block, which is the one place every cap and the
            // cancel are evaluated — precisely the CR-01 class of defect this
            // block's own doc warns about. And a flag set only on success would
            // turn a `Full` channel into a busy spin against a deadline already
            // in the past, because this block runs on every pass. `Full` is
            // unreachable in practice: nothing writes to the child's stdin
            // before the gate, so the writer channel is empty here. Either
            // error is logged and the run is left to fail on its own terms —
            // a prompt that never reached the writer ends as the same startup
            // failure it would have been anyway.
            if !prompt_released && now >= prompt_deadline {
                prompt_released = true;
                if let Err(err) = writer_tx.try_send(WriterCommand::Line(first_message.clone())) {
                    tracing::warn!(
                        "the first user message could not be released to the stdin writer \
                         after the {}ms startup grace: {}",
                        prompt_release_grace.as_millis(),
                        match err {
                            mpsc::error::TrySendError::Full(_) => "the writer channel was full",
                            mpsc::error::TrySendError::Closed(_) => "the writer task is gone",
                        }
                    );
                }
            }

            if !cancelled && cancel_rx.try_recv().is_ok() {
                cancelled = true;
                // A cancel that arrives after the child has already been
                // observed exited must still end the loop. Under the old shape
                // it set `terminate`, whose every consequence was guarded on
                // `!exited` — so it was swallowed and the run hung on (CR-02).
                if exited {
                    stop = true;
                } else {
                    terminate = true;
                }
            }

            // Guarding the two breach checks on `!cancelled` is required, not
            // incidental. Evaluating them every pass would otherwise make the
            // pre-existing breach-outranks-cancel race in the outcome match
            // fire far more often than it does today; the guard keeps that race
            // exactly as frequent as it already is. Fixing the race itself is
            // WR-02's and is out of scope here.
            if !exited && !cancelled {
                if now >= wall_deadline {
                    breach = Some(Breach::WallClock);
                    stop = true;
                } else if now >= idle_deadline {
                    breach = Some(Breach::Idle);
                    stop = true;
                }
            }

            if grace_deadline.is_some_and(|deadline| now >= deadline) {
                stop = true;
            }

            if drain_deadline.is_some_and(|deadline| now >= deadline) {
                drain_expired = true;
                stop = true;
            }

            // A terminate observed HERE is acted on HERE. Deferring it to the
            // post-`select!` block would park it behind the very send the
            // cancel exists to interrupt.
            if terminate || stop {
                if terminate {
                    // Step 1 of the teardown, taken immediately so the CLI's
                    // own clean shutdown overlaps with the drain of its
                    // remaining stream. SIGTERM to the whole GROUP; never
                    // `kill()`, which is SIGKILL and skips the clean path
                    // entirely (D-14).
                    if !exited {
                        terminate_group(&*child);
                    }
                    grace_deadline = Some(now + TEARDOWN_GRACE);
                }
                if stop {
                    break;
                }
                // The bounds were just re-armed; re-evaluate them before
                // parking again. At most one extra pass, and it cannot spin:
                // `cancelled` is now set, so this branch is not reachable twice.
                continue;
            }

            // The hand-off to the caller can never outlive the earliest armed
            // bound, so a cap expiring while the supervisor is parked unparks
            // it at exactly the right instant and the loop head above then
            // classifies the breach on the next pass.
            let mut forward_deadline = now + EVENT_FORWARD_TIMEOUT;
            if !exited && !cancelled {
                forward_deadline = forward_deadline.min(wall_deadline).min(idle_deadline);
            }
            if let Some(deadline) = grace_deadline {
                forward_deadline = forward_deadline.min(deadline);
            }
            if let Some(deadline) = drain_deadline {
                forward_deadline = forward_deadline.min(deadline);
            }
            // In the minimum for exactly the reason the caps are: while the
            // prompt is still withheld, a hand-off to the caller must not be
            // able to outlive the grace, or the release the enforcement block
            // owes would be deferred behind it (260908-uqq, CR-01).
            if !prompt_released {
                forward_deadline = forward_deadline.min(prompt_deadline);
            }

            tokio::select! {
                biased;

                item = reader_rx.recv() => {
                    match item {
                        None => stop = true,
                        Some(item) => {
                            stop = !handle_item(
                                item,
                                &events_tx,
                                &writer_tx,
                                &pending_control,
                                &mut gate_tx,
                                &mut gated,
                                &mut prompt_released,
                                &mut refused,
                                &mut envelopes,
                                &first_message,
                                permission_mode,
                                forward_deadline,
                                &mut dropped_events,
                                replay_observer.as_ref(),
                            )
                            .await;
                        }
                    }
                }

                // Deliberately not a `stop`: the process exiting closes stdout,
                // so the remaining lines are already framed and in flight. The
                // loop keeps draining them and ends on the reader's EOF, which
                // is what stops a run's tail — including its last `result` —
                // from being lost to a race with process exit.
                status = child.wait(), if !exited => {
                    exited = true;
                    exit_status = status.ok();
                    // The tail of already-framed lines gets this long to
                    // arrive, and no longer. stdout EOF is the clean escape;
                    // this is the bound for when there is not one, because a
                    // descendant that outlived the leader is holding the write
                    // end open (CR-02).
                    drain_deadline = Some(Instant::now() + POST_EXIT_DRAIN_CAP);
                }

                _ = tokio::time::sleep_until(wall_deadline), if !exited => {
                    breach = Some(Breach::WallClock);
                    stop = true;
                }

                // The startup grace. Deliberately an EMPTY body: like the cap
                // arms, this arm exists only to unpark the loop at the instant
                // the bound expires so the enforcement block at the top of the
                // next pass can act on it. Doing the release here instead would
                // put it back inside the `select!`, where a hot reader arm can
                // starve it — the exact shape CR-01 removed.
                _ = tokio::time::sleep_until(prompt_deadline), if !prompt_released => {}

                _ = tokio::time::sleep_until(idle_deadline), if !exited => {
                    breach = Some(Breach::Idle);
                    stop = true;
                }

                _ = &mut cancel_rx, if !cancelled => {
                    cancelled = true;
                    terminate = true;
                }

                // The grace a cancel started has run out and the child is still
                // here. Stop draining; the escalation is below. The deadline is
                // only read when the precondition holds, so the fallback value
                // is never observed.
                _ = tokio::time::sleep_until(grace_deadline.unwrap_or(wall_deadline)),
                    if grace_deadline.is_some() && !exited =>
                {
                    stop = true;
                }

                // The post-exit drain has run out and stdout still has not
                // reached EOF. Gated ONLY on the deadline being armed and
                // emphatically NOT on `exited`: the exited flag is what arms
                // this bound, so guarding the bound on it would switch off the
                // one escape from the state it exists to escape (CR-02). Same
                // fallback idiom as the grace arm — the precondition means the
                // fallback value is never observed.
                _ = tokio::time::sleep_until(drain_deadline.unwrap_or(wall_deadline)),
                    if drain_deadline.is_some() =>
                {
                    drain_expired = true;
                    stop = true;
                }
            }

            // Every future the `select!` built — including the one holding the
            // `&mut` borrow of the child — is dropped by the time control gets
            // here, which is the only reason the signal below is legal at all.
            if terminate {
                // Step 1 of the teardown, taken immediately so the CLI's own
                // clean shutdown overlaps with the drain of its remaining
                // stream. SIGTERM to the whole GROUP; never `kill()`, which is
                // SIGKILL and skips the clean path entirely (D-14).
                if !exited {
                    terminate_group(&*child);
                }
                grace_deadline = Some(Instant::now() + TEARDOWN_GRACE);
            }

            if stop {
                break;
            }
        }

        // A refused run is not going to be answered — either its prompt was
        // never released, or `prompt_release_grace` released it into a child
        // whose init then failed the gate and which must not be left running a
        // turn nobody will read (260908-uqq) — and a breached run is by
        // definition not going to end on its own; both leave a live group.
        let tear_down = refused.is_some() || breach.is_some();
        if tear_down || cancelled {
            let _ = writer_tx.send(WriterCommand::Close).await;
        }
        drop(writer_tx);

        // The loop ended with the gate still unanswered, so `start` is parked on
        // it and this is the only thing that will ever release it.
        //
        // **Which error it gets is chosen from the breach this supervisor
        // already classified, not flattened into one word (260908-uqq).** The
        // supervisor does compute the right verdict — `RunOutcome::Stalled` or
        // `RunOutcome::TimedOut` — and it sends it on `outcome_tx` a few lines
        // below. But `outcome_tx`'s receiver lives in an `ExecutionHandle` that
        // this path never returns: `start` failed, so the caller got an `Err`
        // and no handle at all. The gate channel is therefore the caller's ONLY
        // surface, and it has to carry the same fact the outcome channel does,
        // or a fifteen-minute stall reaches the driver indistinguishable from a
        // binary that would not launch — which is exactly how one was recorded
        // as `spawn_failed` for 900.068 seconds.
        //
        // With no breach the original meaning is unchanged: stdout closed
        // without the child ever announcing itself.
        if let Some(tx) = gate_tx.take() {
            let _ = tx.send(Err(match breach {
                Some(Breach::Idle) => SpawnError::StalledBeforeInit {
                    idle_for: idle_cap,
                },
                Some(Breach::WallClock) => SpawnError::TimedOutBeforeInit {
                    after: wall_clock_cap,
                },
                None => SpawnError::InitNeverObserved,
            }));
        }

        // **An observed exit is not a reaped group.** `ProcessGroupChild::wait`
        // awaits the leader and CACHES its status, so a partially-polled wait
        // future dropped by another `select!` arm winning leaves that status
        // cached with the group never reaped — and the next `wait()` then
        // returns `Ready` immediately. "Exited" therefore means "the leader is
        // gone", never "nothing of this run is left running" (CR-02, T-15-52).
        //
        // The two exited branches are split on exactly that distinction: stdout
        // EOF inside the drain bound proves every writer on the pipe is gone,
        // and nothing more is owed. The bound expiring proves the opposite.
        let status = if exited && !drain_expired {
            // The group reached stdout EOF and is proven done. This is the path
            // every healthy run takes, and it is deliberately unchanged.
            exit_status
        } else if exited {
            // Something outlived the leader and is holding the pipe open. Run
            // the full documented four-step teardown and DISCARD its returned
            // status: the teardown here is for the survivors, not for the
            // verdict — the leader's own already-observed status is the
            // authoritative liveness signal and is what gets reported.
            tracing::warn!(
                "the claude leader exited but its stream stayed open for {} seconds; \
                 tearing the process group down",
                POST_EXIT_DRAIN_CAP.as_secs()
            );
            let _ = tear_down_group(&mut child).await;
            exit_status
        } else if tear_down {
            tear_down_group(&mut child).await
        } else if cancelled {
            // Steps 1 and 2 already ran: the terminate signal went out the
            // moment the cancel arrived, and its grace has been elapsing under
            // the drain ever since. Only whatever is left of that grace is
            // waited out here, so a cancel never costs two grace periods.
            let remaining = grace_deadline
                .map(|deadline| deadline.saturating_duration_since(Instant::now()))
                .unwrap_or(TEARDOWN_GRACE);
            finish_teardown(&mut child, remaining).await
        } else {
            await_clean_exit(&mut child).await
        };
        running.store(false, Ordering::SeqCst);

        let after = capture_snapshot(project_root).await;

        // The per-turn projection, rebuilt for the outcomes that carry it. The
        // derivation itself is handed the envelopes, so nothing downstream of
        // here loses a field the verdict depends on.
        let turns: Vec<TurnOutcome> = envelopes.iter().map(TurnOutcome::from_result).collect();

        let outcome = match refused {
            // Every refusal path projects onto the same coarse outcome the TUI
            // renders. The typed `CapabilityError` returned by `start()` is the
            // fidelity-preserving surface; this is deliberately the lossy one.
            Some(err) => RunOutcome::CapabilityRefused {
                missing: err.unmet_requirements(),
            },
            // The two breaches are classified apart on purpose: "too long" and
            // "went silent" are different failures with different remedies.
            None => match breach {
                Some(Breach::WallClock) => RunOutcome::TimedOut {
                    after: wall_clock_cap,
                },
                Some(Breach::Idle) => RunOutcome::Stalled { idle_for: idle_cap },
                None if cancelled => RunOutcome::Killed { turns },
                None => derive_run_outcome_from_envelopes(&envelopes, status, &before, &after),
            },
        };

        // Ahead of the terminal event on purpose: a journal reading the stream
        // in order then sees the loss before the ending, so a truncated run
        // history is distinguishable from a complete one (D-33). Best-effort
        // and bounded — see `report_dropped`.
        report_dropped(&events_tx, dropped_events).await;

        if let Some(status) = status {
            // Bounded for the same reason every in-loop hand-off is. This send
            // sits between "the outcome is decided" and "the outcome is sent",
            // and it goes to the same bounded channel a stalled consumer has
            // already filled — so left unbounded it is a third way for a run to
            // end without `outcome_tx` ever being sent, which is the one thing
            // that must never happen (CR-01).
            let _ = tokio::time::timeout(
                EVENT_FORWARD_TIMEOUT,
                events_tx.send(ExecutionEvent::Exited(status)),
            )
            .await;
        }
        drop(events_tx);

        // Dropping every registered sender is what releases each blocked caller
        // with `ControlResponseLost`. A run that has ended can never answer a
        // control request, and leaving this map populated for the process
        // lifetime is exactly what made that error variant unreachable — the
        // waiter simply sat on its oneshot forever (D-13, CR-03).
        pending_control.lock().await.clear();

        let _ = outcome_tx.send(outcome);
    }
}

/// Hand one event to the caller, under a deadline.
///
/// The single funnel every supervisor-side event goes through, and the reason
/// the supervisor's own bounds cannot be disabled by its consumer. Returns:
///
/// * `true` on a delivered event;
/// * `false` when the receiver is gone — the established "stop producing"
///   convention this file already uses;
/// * `true` on an elapsed deadline, after counting the loss, so the loop
///   continues and re-evaluates its bounds rather than parking further.
///
/// The warning carries the running **count** and the channel capacity and
/// nothing else — no event, no raw line, no message body. Redact-at-capture
/// does not land until Phase 16, so anything logged here stays unredacted
/// forever (T-15-53).
async fn forward(
    sender: &mpsc::Sender<ExecutionEvent>,
    event: ExecutionEvent,
    forward_deadline: Instant,
    dropped: &mut u64,
) -> bool {
    match tokio::time::timeout_at(forward_deadline, sender.send(event)).await {
        Ok(Ok(())) => true,
        Ok(Err(_)) => false,
        Err(_) => {
            *dropped += 1;
            tracing::warn!(
                "the executor event consumer did not drain within the forward bound; \
                 {} event(s) dropped so far from a channel of {} slots",
                *dropped,
                EVENT_CHANNEL_CAPACITY
            );
            true
        }
    }
}

/// Report the run's total dropped-event count on the stream, once, at the end.
///
/// The companion to [`forward`]: that function *counts* the losses, this one is
/// the only thing that ever tells a consumer they happened. Until now the count
/// reached a `tracing::warn!` and nothing else, and a journal reads the
/// [`ExecutionEvent`] stream rather than the log, so the count had to arrive
/// here for D-33 to be satisfiable at all (see plan 15-08's handover).
///
/// Sends nothing when `dropped` is zero. A lossless run therefore emits no
/// report and the absence of the event is itself the signal.
///
/// Bounded for precisely the reason the `Exited` send beside it is: this report
/// goes to the very channel a stalled consumer has already filled, so left
/// unbounded it would be a new way for a run to end without `outcome_tx` ever
/// being sent, which is the one thing that must never happen (CR-01).
///
/// The report is therefore **best-effort**. If the consumer is still stalled
/// when the run ends the report is lost and only the warning in [`forward`]
/// remains — an acceptable degradation, because a lost diagnostic is strictly
/// better than a parked run.
async fn report_dropped(sender: &mpsc::Sender<ExecutionEvent>, dropped: u64) {
    if dropped == 0 {
        return;
    }

    let _ = tokio::time::timeout(
        EVENT_FORWARD_TIMEOUT,
        sender.send(ExecutionEvent::EventsDropped { count: dropped }),
    )
    .await;
}

/// Handle one reader item. Returns `false` when the run loop should stop.
#[allow(clippy::too_many_arguments)]
async fn handle_item(
    item: ReaderItem,
    events_tx: &mpsc::Sender<ExecutionEvent>,
    writer_tx: &mpsc::Sender<WriterCommand>,
    pending_control: &PendingControl,
    gate_tx: &mut Option<oneshot::Sender<Result<GateOutcome, SpawnError>>>,
    gated: &mut bool,
    // Whether the supervisor's startup grace already released the first user
    // message. Read AND written here: the gate-passed branch releases the
    // prompt only when this is still false, and sets it when it does, so the
    // prompt is written exactly once whichever arm fired (260908-uqq).
    prompt_released: &mut bool,
    refused: &mut Option<CapabilityError>,
    envelopes: &mut Vec<ResultMessage>,
    first_message: &str,
    permission_mode: PermissionMode,
    forward_deadline: Instant,
    dropped: &mut u64,
    replay_observer: Option<&mpsc::UnboundedSender<String>>,
) -> bool {
    match item {
        ReaderItem::Truncated { bytes, prefix } => {
            forward(
                events_tx,
                ExecutionEvent::LineTruncated { bytes, prefix },
                forward_deadline,
                dropped,
            )
            .await
        }
        ReaderItem::Envelope(Envelope::Unparseable { raw, error }) => {
            forward(
                events_tx,
                ExecutionEvent::Unparseable { raw, error },
                forward_deadline,
                dropped,
            )
            .await
        }
        ReaderItem::Envelope(Envelope::Parsed { raw, msg }) => match msg {
            StreamMessage::Unknown => {
                forward(
                    events_tx,
                    ExecutionEvent::Unknown { raw },
                    forward_deadline,
                    dropped,
                )
                .await
            }

            StreamMessage::System(SystemMessage::Init(init)) if !*gated => {
                match gate::validate_first_init(&init, permission_mode) {
                    Ok(facts) => {
                        *gated = true;
                        if let Some(tx) = gate_tx.take() {
                            let _ = tx.send(Ok(facts.clone()));
                        }
                        if !forward(
                            events_tx,
                            ExecutionEvent::SessionStarted {
                                session_id: facts.session_id.unwrap_or_default(),
                                capabilities: facts.capabilities,
                                claude_code_version: facts
                                    .claude_code_version
                                    .unwrap_or_default(),
                                api_key_source: facts.api_key_source,
                                permission_mode: facts.permission_mode,
                            },
                            forward_deadline,
                            dropped,
                        )
                        .await
                        {
                            return false;
                        }
                        // The **eager arm**: the CLI announced itself inside
                        // `prompt_release_grace`, so the gate ruled before a
                        // single byte reached the child's stdin and only now is
                        // the prompt released (D-02, D-06). On this arm — and
                        // only on this arm — a refusal would have cost zero
                        // tokens and zero quota.
                        //
                        // When the grace already fired, `prompt_released` is
                        // set and this send is skipped: the init being handled
                        // is then the one the prompt provoked, and re-releasing
                        // it would queue the command a second time as its own
                        // turn (260908-uqq, D-31).
                        if *prompt_released {
                            true
                        } else {
                            *prompt_released = true;
                            writer_tx
                                .send(WriterCommand::Line(first_message.to_string()))
                                .await
                                .is_ok()
                        }
                    }
                    Err(err) => {
                        if let Some(tx) = gate_tx.take() {
                            let _ = tx.send(Err(SpawnError::Capability(err.clone())));
                        }
                        *refused = Some(err);
                        false
                    }
                }
            }

            // A later `system/init` is informational: every queued turn emits
            // its own, and re-running the gate there would convert a start-time
            // refusal into a mid-run abort (D-30). Turn messages and rate-limit
            // events forward verbatim for the same reason — this layer routes
            // envelopes, it does not interpret them.
            msg @ (StreamMessage::System(_)
            | StreamMessage::Assistant(_)
            | StreamMessage::User(_)
            | StreamMessage::RateLimitEvent(_)) => {
                // The replay echo, republished RAW to whoever asked for it —
                // **before** the event is forwarded, so a consumer that sees
                // both never sees the event first (D-07, D-08).
                //
                // The filter is the envelope's own `is_replay` marker, read off
                // the parsed message rather than sniffed out of the string: a
                // `user` message without it is a tool result, which is not an
                // echo of anything the driver sent and must never be able to
                // ack an injected message. The send itself is infallible-ish by
                // construction — an unbounded channel cannot apply backpressure
                // to this task — so a coordinator that is tearing a run down can
                // never be parked here.
                if let (Some(observer), StreamMessage::User(turn)) = (replay_observer, &msg) {
                    if turn.is_replay {
                        let _ = observer.send(raw.clone());
                    }
                }

                forward(
                    events_tx,
                    ExecutionEvent::Message(Box::new(msg)),
                    forward_deadline,
                    dropped,
                )
                .await
            }

            StreamMessage::Result(result) => {
                // The whole envelope, verbatim off the wire, before the box is
                // moved into the event. `permission_denials[]` lives here and
                // nowhere else, so anything that projects first loses it.
                envelopes.push((*result).clone());
                if let Some(cost) = result.total_cost_usd {
                    if !forward(
                        events_tx,
                        ExecutionEvent::Cost {
                            cumulative_usd: cost,
                        },
                        forward_deadline,
                        dropped,
                    )
                    .await
                    {
                        return false;
                    }
                }
                // Deliberately no `break` here: `result` closes a TURN, not the
                // run. Stopping on the first one truncates every steered run
                // while reporting success (D-29).
                forward(
                    events_tx,
                    ExecutionEvent::TurnCompleted(result),
                    forward_deadline,
                    dropped,
                )
                .await
            }

            StreamMessage::ControlResponse(response) => {
                let request_id = response.response.request_id.clone();
                if let Some(waiter) = pending_control.lock().await.remove(&request_id) {
                    let _ = waiter.send(response.clone());
                }
                forward(
                    events_tx,
                    ExecutionEvent::Message(Box::new(StreamMessage::ControlResponse(response))),
                    forward_deadline,
                    dropped,
                )
                .await
            }
        },
    }
}

/// Send SIGTERM to the process **group** — step 1 of the teardown.
///
/// `signal(15)` and not `start_kill()`, and emphatically not `kill()`: on a
/// process group the wrapper's start-of-kill method sends the **uncatchable**
/// signal, and its combined convenience method is that plus a wait. Reading
/// `kill()` as "terminate politely" is natural and wrong, and taking it would
/// skip the CLI's entire documented clean shutdown — the turn abort, the
/// Bash-tree teardown through its own handler, the `SessionEnd` hooks, and the
/// conventional signal-terminated exit status (D-14, Pitfall B).
fn terminate_group(child: &dyn ChildWrapper) {
    if let Err(err) = child.signal(SIGTERM) {
        tracing::warn!("failed to SIGTERM the claude process group: {}", err);
    }
}

/// The four-step teardown (D-14).
///
/// In order, and every step earns its place:
///
/// 1. **SIGTERM to the group.** The documented clean path — see
///    [`terminate_group`]. Signalling the *group* rather than the child is the
///    whole reason this executor carries a process-group dependency: `claude`
///    spawns Bash grandchildren, and a signal to the direct child alone orphans
///    them (T-15-30).
/// 2. **A ten-second grace.** Enough for the CLI to abort its turn, tear its
///    tree down and run its session-end hooks.
/// 3. **SIGKILL to the group**, via `start_kill()`, as the backstop for a child
///    that ignored the terminate signal.
/// 4. **The wait, AWAITED TO COMPLETION.** This is what prevents zombies, and
///    "to completion" is load-bearing: `ProcessGroupChild::wait()` awaits the
///    leader, then loops a non-blocking group reap ten times, then falls back to
///    a blocking reap on a blocking task. Dropping that future mid-reap leaves
///    the grandchildren unreaped, so it is never raced against anything here
///    (T-15-34).
///
/// A cancel takes step 1 the instant it arrives, so that the CLI's shutdown
/// overlaps with the drain of its remaining stream rather than starting after
/// it, and its grace elapses under that drain. That path therefore calls
/// [`finish_teardown`] directly with whatever grace is left, and this function
/// is for the paths that have not signalled anything yet.
async fn tear_down_group(child: &mut Box<dyn ChildWrapper>) -> Option<ExitStatus> {
    terminate_group(&**child);
    finish_teardown(child, TEARDOWN_GRACE).await
}

/// Steps 2 to 4 of the teardown: wait out `grace`, escalate, then reap.
///
/// A zero `grace` means it has already elapsed elsewhere and the escalation is
/// due now. The final `wait()` is deliberately unbounded and deliberately not
/// raced against anything — see [`tear_down_group`] step 4.
async fn finish_teardown(child: &mut Box<dyn ChildWrapper>, grace: Duration) -> Option<ExitStatus> {
    if !grace.is_zero() {
        if let Ok(result) = tokio::time::timeout(grace, child.wait()).await {
            return result.ok();
        }
    }
    if let Err(err) = child.start_kill() {
        tracing::warn!("failed to SIGKILL the claude process group: {}", err);
    }
    child.wait().await.ok()
}

/// Await the exit of a run whose stream ended on its own, under a bound.
///
/// The bound is what keeps the D-13 promise absolute: with it, **no** path in
/// this file awaits process exit unbounded. A child whose stdout has already
/// reached EOF is by definition on its way out, so this expiring means the run
/// is not ending cleanly after all — and the escalation is the same four-step
/// teardown a cap breach takes.
async fn await_clean_exit(child: &mut Box<dyn ChildWrapper>) -> Option<ExitStatus> {
    match tokio::time::timeout(EXIT_DRAIN_CAP, child.wait()).await {
        Ok(result) => result.ok(),
        Err(_) => {
            tracing::warn!(
                "the claude stream closed but the process group did not exit within {} seconds; \
                 tearing it down",
                EXIT_DRAIN_CAP.as_secs()
            );
            tear_down_group(child).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::{PermissionMode, SettingSources};

    fn argv_strings(options: &ExecutionOptions) -> Vec<String> {
        build_argv(options)
            .into_iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    fn flag_value<'a>(argv: &'a [String], flag: &str) -> Option<&'a str> {
        argv.iter()
            .position(|arg| arg == flag)
            .and_then(|index| argv.get(index + 1))
            .map(String::as_str)
    }

    #[test]
    fn the_argv_baseline_carries_every_required_flag() {
        let argv = argv_strings(&ExecutionOptions::default());
        for expected in [
            "-p",
            "--verbose",
            "--replay-user-messages",
            "--session-id",
            "--setting-sources",
            "--permission-mode",
            "--strict-mcp-config",
        ] {
            assert!(
                argv.iter().any(|arg| arg == expected),
                "argv is missing {expected}: {argv:?}"
            );
        }
        assert_eq!(flag_value(&argv, "--input-format"), Some("stream-json"));
        assert_eq!(flag_value(&argv, "--output-format"), Some("stream-json"));
        assert_eq!(flag_value(&argv, "--setting-sources"), Some("project"));
        assert_eq!(flag_value(&argv, "--permission-mode"), Some("dontAsk"));
    }

    /// The pinned baseline vector, with a fixed session id so the whole thing
    /// can be compared rather than sampled.
    fn pinned_baseline() -> Vec<String> {
        vec![
            "-p".to_string(),
            "--input-format".to_string(),
            "stream-json".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
            "--replay-user-messages".to_string(),
            "--session-id".to_string(),
            Uuid::nil().to_string(),
            "--setting-sources".to_string(),
            "project".to_string(),
            "--permission-mode".to_string(),
            "dontAsk".to_string(),
            "--strict-mcp-config".to_string(),
        ]
    }

    #[test]
    fn the_pinned_vector_is_unchanged_when_no_envelope_was_established() {
        // The half of the pair that keeps the two new flags from becoming
        // unconditional: a caller that establishes no envelope gets exactly the
        // vector this driver shipped before Phase 19, byte for byte.
        let options = ExecutionOptions {
            session_id: Uuid::nil(),
            ..Default::default()
        };
        assert_eq!(argv_strings(&options), pinned_baseline());
    }

    #[test]
    fn the_envelope_deny_list_and_settings_path_ride_on_argv() {
        // D-06 layer 1 and D-07: both controls are carried on argv, where a
        // validation failure cannot silently drop them the way it drops a
        // settings file. The assertion is the EXACT vector rather than a
        // `contains`, because the thing that goes wrong here is a flag arriving
        // without its value or in the wrong place.
        let options = ExecutionOptions {
            session_id: Uuid::nil(),
            envelope_disallowed_tools: vec![
                "Bash(git push:*)".to_string(),
                "Write(.claude/**)".to_string(),
            ],
            envelope_settings: Some(PathBuf::from("/envelope/alpha/settings.json")),
            ..Default::default()
        };

        let mut expected = pinned_baseline();
        expected.extend([
            "--disallowedTools".to_string(),
            "Bash(git push:*),Write(.claude/**)".to_string(),
            "--settings".to_string(),
            "/envelope/alpha/settings.json".to_string(),
        ]);

        assert_eq!(argv_strings(&options), expected);
    }

    #[test]
    fn the_envelope_flags_are_each_absent_on_their_own_when_unset() {
        // Independently optional, because they are established by different
        // calls and a partial envelope must not render half a flag.
        let only_tools = ExecutionOptions {
            envelope_disallowed_tools: vec!["Bash(git push:*)".to_string()],
            ..Default::default()
        };
        let argv = argv_strings(&only_tools);
        assert_eq!(
            flag_value(&argv, "--disallowedTools"),
            Some("Bash(git push:*)")
        );
        assert!(
            !argv.iter().any(|arg| arg == "--settings"),
            "an unset settings path must emit no flag at all: {argv:?}"
        );

        let only_settings = ExecutionOptions {
            envelope_settings: Some(PathBuf::from("/envelope/alpha/settings.json")),
            ..Default::default()
        };
        let argv = argv_strings(&only_settings);
        assert_eq!(
            flag_value(&argv, "--settings"),
            Some("/envelope/alpha/settings.json")
        );
        assert!(
            !argv.iter().any(|arg| arg == "--disallowedTools"),
            "an empty deny list must emit no flag at all: {argv:?}"
        );
    }

    #[test]
    fn the_permission_mode_enum_has_exactly_one_variant_and_it_is_not_a_bypass() {
        // D-28: `PermissionMode` gains no bypass variant. The MATCH is what
        // holds that — adding a variant makes this fail to compile, which is a
        // louder failure than any grep, and the grep cannot see a variant that
        // is spelled differently from the token it looks for.
        let every: &[PermissionMode] = &[PermissionMode::DontAsk];
        for mode in every {
            match mode {
                PermissionMode::DontAsk => {
                    assert_eq!(mode.as_flag_value(), "dontAsk");
                }
            }
        }
        assert_eq!(
            every.len(),
            1,
            "a second permission mode exists; if it is a bypass it must be \
             removed, and if it is not, this count is what forces somebody to \
             say so out loud"
        );
    }

    #[test]
    fn the_envelope_flags_never_smuggle_a_permission_bypass() {
        // The envelope's own fields are attacker-adjacent in the sense that
        // matters here: they are the newest thing on argv, so they are where a
        // bypass would most plausibly arrive next. Same fragment-assembly
        // discipline as the baseline test below.
        let forbidden = [
            concat!("--dangerously", "-skip-permissions"),
            concat!("bypass", "Permissions"),
        ];
        let options = ExecutionOptions {
            envelope_disallowed_tools: vec!["Bash(git push:*)".to_string()],
            envelope_settings: Some(PathBuf::from("/envelope/alpha/settings.json")),
            ..Default::default()
        };
        let argv = argv_strings(&options);
        for token in forbidden {
            assert!(
                !argv.iter().any(|arg| arg.contains(token)),
                "argv must never carry {token}: {argv:?}"
            );
        }
    }

    #[test]
    fn the_argv_never_carries_a_permission_bypass() {
        // The forbidden tokens are assembled from fragments rather than
        // written out: this file is itself grepped for those literals as the
        // mechanical D-15 and D-08 guards, and a test asserting their absence
        // must not be what makes a guard report their presence.
        let forbidden = [
            concat!("--dangerously", "-skip-permissions"),
            concat!("bypass", "Permissions"),
            concat!("--ba", "re"),
        ];
        let argv = argv_strings(&ExecutionOptions::default());
        for token in forbidden {
            assert!(
                !argv.iter().any(|arg| arg.contains(token)),
                "argv must never carry {token}: {argv:?}"
            );
        }
    }

    #[test]
    fn the_user_setting_source_toggle_is_opt_in() {
        let options = ExecutionOptions {
            setting_sources: SettingSources::UserAndProject,
            ..Default::default()
        };
        assert_eq!(
            flag_value(&argv_strings(&options), "--setting-sources"),
            Some("user,project")
        );
    }

    #[test]
    fn optional_flags_are_absent_unless_requested() {
        let argv = argv_strings(&ExecutionOptions::default());
        for absent in ["--model", "--resume", "--max-budget-usd", "--name"] {
            assert!(
                !argv.iter().any(|arg| arg == absent),
                "{absent} should be absent by default: {argv:?}"
            );
        }
    }

    // ========================================================================
    // The two deadlines, and the ordering constraint between them (D-13, D-14)
    // ========================================================================

    #[test]
    fn the_default_idle_cap_is_strictly_greater_than_the_background_wait_ceiling() {
        let options = ExecutionOptions::default();
        let ceiling = Duration::from_millis(options.bg_wait_ceiling_ms);
        assert!(
            options.idle_cap > ceiling,
            "a healthy run may legitimately emit nothing while it waits for background \
             subagents, so the idle cap must let that ceiling fire FIRST — otherwise the \
             stuck detector kills healthy runs. idle cap {:?} vs ceiling {:?}",
            options.idle_cap,
            ceiling
        );
    }

    #[test]
    fn the_default_wall_clock_cap_is_the_looser_of_the_two_caps() {
        let options = ExecutionOptions::default();
        assert!(
            options.wall_clock_cap > options.idle_cap,
            "the idle cap is the stuck detector and the wall-clock cap is the outer \
             backstop; inverting them would make the idle cap unreachable. wall {:?} vs \
             idle {:?}",
            options.wall_clock_cap,
            options.idle_cap
        );
    }

    #[tokio::test]
    async fn a_line_over_the_byte_bound_is_truncated_not_parsed() {
        let oversize = "x".repeat(MAX_LINE_BYTES + 10);
        let input = format!("{oversize}\n{{\"type\":\"system\",\"subtype\":\"init\"}}\n");
        let mut reader = BufReader::new(std::io::Cursor::new(input.into_bytes()));

        match read_bounded_line(&mut reader).await.expect("read") {
            BoundedLine::Truncated { bytes, .. } => assert_eq!(
                bytes,
                MAX_LINE_BYTES + 10,
                "the bound is measured in BYTES, not characters"
            ),
            other => panic!("expected a truncated line, got: {other:?}"),
        }

        // The reader recovers: the next line still frames normally.
        match read_bounded_line(&mut reader).await.expect("read") {
            BoundedLine::Line(line) => assert!(line.starts_with('{'), "got: {line}"),
            other => panic!("expected the following line, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn the_byte_bound_counts_bytes_not_characters() {
        // Each 'é' is two bytes, so this is under the character bound and over
        // the byte bound — the whole point of measuring in bytes.
        let line = "é".repeat(MAX_LINE_BYTES / 2 + 1);
        let mut reader = BufReader::new(std::io::Cursor::new(format!("{line}\n").into_bytes()));
        assert!(
            matches!(
                read_bounded_line(&mut reader).await.expect("read"),
                BoundedLine::Truncated { .. }
            ),
            "a line under MAX_LINE_BYTES chars but over MAX_LINE_BYTES bytes must truncate"
        );
    }

    #[tokio::test]
    async fn an_empty_stream_reads_as_eof() {
        let mut reader = BufReader::new(std::io::Cursor::new(Vec::new()));
        assert!(matches!(
            read_bounded_line(&mut reader).await.expect("read"),
            BoundedLine::Eof
        ));
    }

    // ========================================================================
    // The dropped-event report (D-33)
    // ========================================================================

    // The channel locals below are deliberately NOT named after the run loop's
    // own sender. That name plus the helper's is the grep that asserts there is
    // exactly ONE call site in the terminal path, and a test reusing the name
    // would leave that check unable to tell one call site from four.

    #[tokio::test]
    async fn a_run_that_dropped_nothing_reports_nothing() {
        let (report_tx, mut report_rx) = mpsc::channel(8);

        report_dropped(&report_tx, 0).await;

        assert!(
            matches!(report_rx.try_recv(), Err(mpsc::error::TryRecvError::Empty)),
            "a run that lost nothing must emit no report at all — the absence \
             of the event is the signal (D-33)"
        );
    }

    #[tokio::test]
    async fn a_run_that_dropped_events_reports_the_count_once() {
        let (report_tx, mut report_rx) = mpsc::channel(8);

        report_dropped(&report_tx, 40).await;

        match report_rx
            .try_recv()
            .expect("a lossy run must report its count on the stream, not only to the log")
        {
            ExecutionEvent::EventsDropped { count } => assert_eq!(
                count, 40,
                "the reported count must be the run's running total"
            ),
            other => panic!("expected EventsDropped, got: {other:?}"),
        }

        assert!(
            matches!(report_rx.try_recv(), Err(mpsc::error::TryRecvError::Empty)),
            "one report per run, not one per dropped event"
        );
    }

    #[tokio::test]
    async fn reporting_a_drop_into_a_full_channel_returns_within_the_forward_bound() {
        // Capacity one, already full, and a receiver that is held but never
        // read: exactly the stalled consumer whose channel the report has to
        // squeeze into. An unbounded send here would park the run forever.
        let (report_tx, _report_rx) = mpsc::channel(1);
        report_tx
            .send(ExecutionEvent::Stderr("fills the only slot".to_string()))
            .await
            .expect("the receiver is still held");

        let started = Instant::now();
        report_dropped(&report_tx, 7).await;
        let elapsed = started.elapsed();

        // A wall-clock assertion is warranted here and nowhere else in this
        // module, because the property under test *is* a deadline. The bound is
        // a generous multiple of the forward timeout so it cannot flake on a
        // loaded machine while still failing outright on an unbounded send.
        assert!(
            elapsed < EVENT_FORWARD_TIMEOUT * 3,
            "the report must not park the run; it returned only after {elapsed:?}"
        );
    }

    // ========================================================================
    // First-init-only (D-30)
    // ========================================================================

    /// A healthy 2.1.220 `system/init`.
    const GOOD_INIT: &str = r#"{"type":"system","subtype":"init","session_id":"s","capabilities":["interrupt_receipt_v1","interrupt_cancel_queued_v1","msg_lifecycle_v1"],"apiKeySource":"none","claude_code_version":"2.1.220","permissionMode":"dontAsk"}"#;

    /// A deliberately hostile later `system/init`: no capabilities, no version
    /// and an auth source that would fire every guard. Re-running the gate on
    /// it would convert a start-time refusal into a mid-run abort (D-30).
    const HOSTILE_LATER_INIT: &str = r#"{"type":"system","subtype":"init","session_id":"s","capabilities":[],"apiKeySource":"ANTHROPIC_API_KEY"}"#;

    /// Feed raw lines through `handle_item` exactly as the coordinator does.
    ///
    /// Returns whether the loop would keep going, the refusal (if any), the
    /// events emitted, and every line released to the stdin writer.
    ///
    /// **This drives the EAGER arm and only the eager arm.** There is no
    /// supervisor loop here and therefore no startup grace timer, so
    /// `prompt_released` starts false and can only be set by the gate passing —
    /// which is exactly the ordering a CLI that announces inside
    /// `prompt_release_grace` produces. The late arm, where the grace releases
    /// the prompt first, needs a real child and lives in
    /// `tests/executor_transport.rs` (260908-uqq).
    async fn feed(lines: &[&str]) -> (bool, Option<CapabilityError>, Vec<ExecutionEvent>, Vec<String>) {
        let (events_tx, mut events_rx) = mpsc::channel(64);
        let (writer_tx, mut writer_rx) = mpsc::channel(64);
        let pending_control: PendingControl = Arc::new(Mutex::new(Default::default()));
        let (gate_tx, _gate_rx) = oneshot::channel();

        let mut gate_tx = Some(gate_tx);
        let mut gated = false;
        // False and never set from outside: no grace timer runs here, so this
        // helper reproduces the eager arm's ordering by construction.
        let mut prompt_released = false;
        let mut refused: Option<CapabilityError> = None;
        let mut envelopes: Vec<ResultMessage> = Vec::new();
        let mut keep_going = true;
        // Far out of reach, and a throwaway counter: these tests are about the
        // first-init routing, and a forward that could expire under them would
        // change what they mean.
        let forward_deadline = Instant::now() + Duration::from_secs(3600);
        let mut dropped = 0u64;

        for line in lines {
            keep_going = handle_item(
                ReaderItem::Envelope(parse_line(line)),
                &events_tx,
                &writer_tx,
                &pending_control,
                &mut gate_tx,
                &mut gated,
                &mut prompt_released,
                &mut refused,
                &mut envelopes,
                "FIRST-MESSAGE",
                PermissionMode::DontAsk,
                forward_deadline,
                &mut dropped,
                None,
            )
            .await;
            if !keep_going {
                break;
            }
        }

        drop(events_tx);
        drop(writer_tx);

        let mut events = Vec::new();
        while let Ok(event) = events_rx.try_recv() {
            events.push(event);
        }
        let mut written = Vec::new();
        while let Ok(command) = writer_rx.try_recv() {
            if let WriterCommand::Line(line) = command {
                written.push(line);
            }
        }
        (keep_going, refused, events, written)
    }

    #[tokio::test]
    async fn a_second_system_init_does_not_re_run_the_gate_or_abort_the_run() {
        let (keep_going, refused, events, written) =
            feed(&[GOOD_INIT, HOSTILE_LATER_INIT]).await;

        assert!(keep_going, "a later system/init must not stop the run (D-30)");
        assert!(
            refused.is_none(),
            "a later system/init must not re-arm the guard as a mid-run abort, got: {refused:?}"
        );
        assert_eq!(
            written.len(),
            1,
            "the prompt is released exactly once. On this EAGER arm — no startup grace \
             ran, so the gate is what released it — that means the first init and not the \
             second. The other arm, where `prompt_release_grace` released it before any \
             init arrived, must also produce exactly one line, and that is pinned in \
             `tests/executor_lifecycle.rs`: {written:?}"
        );
        assert!(
            matches!(events.first(), Some(ExecutionEvent::SessionStarted { .. })),
            "the first init announces the session, got: {:?}",
            events.first()
        );
        assert!(
            matches!(events.get(1), Some(ExecutionEvent::Message(_))),
            "the second init is forwarded as informational and is not a protocol error, got: {:?}",
            events.get(1)
        );
        assert_eq!(events.len(), 2, "no other event is emitted: {events:?}");
    }

    /// The EAGER arm's zero-cost property, and nothing wider.
    ///
    /// `feed` runs no startup grace, so this is the ordering a CLI that
    /// announces inside `prompt_release_grace` produces: the gate rules first
    /// and the refusal is genuinely free. A CLI that announces only after
    /// reading a user message takes the other arm, where the prompt is already
    /// out — see the sibling test in `tests/executor_transport.rs`
    /// (260908-uqq).
    #[tokio::test]
    async fn a_refused_first_init_never_releases_the_prompt_on_the_eager_arm() {
        let no_capabilities = r#"{"type":"system","subtype":"init","session_id":"s","capabilities":[],"apiKeySource":"none","claude_code_version":"2.1.220"}"#;
        let (keep_going, refused, _events, written) = feed(&[no_capabilities]).await;

        assert!(!keep_going, "a refusal stops the run loop");
        assert!(
            matches!(
                refused,
                Some(CapabilityError::MissingCapabilities { .. })
            ),
            "expected a capability refusal, got: {refused:?}"
        );
        assert!(
            written.is_empty(),
            "on the EAGER arm — the CLI announced before the startup grace expired — a \
             refused run must never release the prompt to stdin, which is what makes the \
             refusal cost zero tokens and zero quota there (D-06, TRANS-04). This pins \
             that arm and does not claim it of the late arm, where \
             `prompt_release_grace` has already written the prompt: {written:?}"
        );
    }

    // ========================================================================
    // `a_refused_run_writes_zero_bytes_to_the_child_stdin` used to live here.
    //
    // It moved to `tests/executor_transport.rs` for two reasons that agree. It
    // spawns a real child process, and this repository's convention is that such
    // a test belongs in `tests/` rather than in-source. And it needs a
    // `DrivableProject`, which under `src/` it could only obtain through the
    // opt-in escape hatch — `tests/spawn_seam_guard.rs` fences that identifier
    // out of `src/` entirely, and keeping the fence absolute is worth more than
    // the test's location (D-17).
    // ========================================================================
}
