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
// * **Process exit is never awaited unbounded (D-13).** Every `wait()` in this
//   file is either raced by the supervisor's two caps or wrapped in an explicit
//   bound. The two caps are independent and measure different things: the
//   wall-clock cap measures time since spawn, and the idle cap measures time
//   since the **reader** last observed a line. Only the second can tell a
//   legitimately long run from a hung one — the research pass reproduced a hang
//   that sat silent for minutes, while a real multi-step run emits events
//   continuously, and time-since-spawn cannot separate those.
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
    RunOutcome, TurnOutcome, WriterCommand,
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
const EXIT_DRAIN_CAP: Duration = Duration::from_secs(30);

/// Capacity of the executor event channel.
///
/// Bounded, so a blocked consumer applies backpressure to the reader rather
/// than growing memory without limit. Generous on purpose: Claude caps its exit
/// drain at 30 seconds, so a slow consumer can truncate a run's tail.
const EVENT_CHANNEL_CAPACITY: usize = 8192;

/// Capacity of the stdin writer's command channel.
const WRITER_CHANNEL_CAPACITY: usize = 64;

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
        }
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

        let mut wrap = CommandWrap::with_new(&program, |cmd| {
            cmd.args(&argv)
                .current_dir(&cwd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            // The TUI is plausibly launched from inside a Claude Code session,
            // so inherited CLAUDE* variables would leak into the driven child
            // and change `-p` behaviour in ways that look like "works on my
            // machine". Scrub them all, then set the one we mean to set.
            for (key, _) in std::env::vars_os() {
                if key.to_string_lossy().starts_with("CLAUDE") {
                    cmd.env_remove(&key);
                }
            }
            cmd.env("CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS", &bg_ceiling);
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
                last_line_rx,
                before,
                project_root: root,
            }
            .run(),
        );

        // Block on the gate verdict: a refusal must be a start-time error, not
        // a mid-run surprise. Because the prompt travels over stdin and has not
        // been released yet, a refusal here costs zero tokens and zero quota.
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

            match rx.await {
                Ok(response) => Ok(InterruptAck::from_response(&response)),
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
    /// The instant the reader last observed a line. The idle arm re-arms from
    /// this and from nothing else (D-13, Pitfall E).
    last_line_rx: watch::Receiver<Instant>,
    before: RunSnapshot,
    project_root: PathBuf,
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
    /// One `tokio::select!` races four things against each other: the child's
    /// exit future, the total wall-clock cap, the idle cap, and the cancel
    /// signal — plus the stream itself, which is what the loop is otherwise
    /// pumping. Racing them is also what resolves the borrow problem the
    /// wrapper's API creates: `wait()` holds a `&mut` borrow of the child for
    /// the whole life of its future, so a supervisor holding that future across
    /// an await could never call `signal()` on the same object. When another arm
    /// wins, `select!` drops every loser — including the exit future — and the
    /// borrow ends before the statement after the macro runs, which is exactly
    /// where the signal call lives.
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
            last_line_rx,
            before,
            project_root,
        } = self;

        let mut gate_tx = Some(gate_tx);
        let mut gated = false;
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

        let wall_deadline = Instant::now() + wall_clock_cap;

        loop {
            let mut stop = false;
            let mut terminate = false;
            // Re-armed on every pass from the instant the READER recorded.
            let idle_deadline = *last_line_rx.borrow() + idle_cap;

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
                                &mut refused,
                                &mut envelopes,
                                &first_message,
                                permission_mode,
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
                }

                _ = tokio::time::sleep_until(wall_deadline), if !exited => {
                    breach = Some(Breach::WallClock);
                    stop = true;
                }

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

        // A refused run never had its prompt released, and a breached run is by
        // definition not going to end on its own; both leave a live group.
        let tear_down = refused.is_some() || breach.is_some();
        if tear_down || cancelled {
            let _ = writer_tx.send(WriterCommand::Close).await;
        }
        drop(writer_tx);

        // stdout closed without ever announcing itself.
        if let Some(tx) = gate_tx.take() {
            let _ = tx.send(Err(SpawnError::InitNeverObserved));
        }

        let status = if exited {
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

        if let Some(status) = status {
            let _ = events_tx.send(ExecutionEvent::Exited(status)).await;
        }
        drop(events_tx);
        let _ = outcome_tx.send(outcome);
    }
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
    refused: &mut Option<CapabilityError>,
    envelopes: &mut Vec<ResultMessage>,
    first_message: &str,
    permission_mode: PermissionMode,
) -> bool {
    match item {
        ReaderItem::Truncated { bytes, prefix } => {
            events_tx
                .send(ExecutionEvent::LineTruncated { bytes, prefix })
                .await
                .is_ok()
        }
        ReaderItem::Envelope(Envelope::Unparseable { raw, error }) => events_tx
            .send(ExecutionEvent::Unparseable { raw, error })
            .await
            .is_ok(),
        ReaderItem::Envelope(Envelope::Parsed { raw, msg }) => match msg {
            StreamMessage::Unknown => events_tx
                .send(ExecutionEvent::Unknown { raw })
                .await
                .is_ok(),

            StreamMessage::System(SystemMessage::Init(init)) if !*gated => {
                match gate::validate_first_init(&init, permission_mode) {
                    Ok(facts) => {
                        *gated = true;
                        if let Some(tx) = gate_tx.take() {
                            let _ = tx.send(Ok(facts.clone()));
                        }
                        if events_tx
                            .send(ExecutionEvent::SessionStarted {
                                session_id: facts.session_id.unwrap_or_default(),
                                capabilities: facts.capabilities,
                                claude_code_version: facts
                                    .claude_code_version
                                    .unwrap_or_default(),
                                api_key_source: facts.api_key_source,
                                permission_mode: facts.permission_mode,
                            })
                            .await
                            .is_err()
                        {
                            return false;
                        }
                        // Only now is the prompt released (D-02, D-06).
                        writer_tx
                            .send(WriterCommand::Line(first_message.to_string()))
                            .await
                            .is_ok()
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
            | StreamMessage::RateLimitEvent(_)) => events_tx
                .send(ExecutionEvent::Message(Box::new(msg)))
                .await
                .is_ok(),

            StreamMessage::Result(result) => {
                // The whole envelope, verbatim off the wire, before the box is
                // moved into the event. `permission_denials[]` lives here and
                // nowhere else, so anything that projects first loses it.
                envelopes.push((*result).clone());
                if let Some(cost) = result.total_cost_usd {
                    if events_tx
                        .send(ExecutionEvent::Cost {
                            cumulative_usd: cost,
                        })
                        .await
                        .is_err()
                    {
                        return false;
                    }
                }
                // Deliberately no `break` here: `result` closes a TURN, not the
                // run. Stopping on the first one truncates every steered run
                // while reporting success (D-29).
                events_tx
                    .send(ExecutionEvent::TurnCompleted(result))
                    .await
                    .is_ok()
            }

            StreamMessage::ControlResponse(response) => {
                let request_id = response.response.request_id.clone();
                if let Some(waiter) = pending_control.lock().await.remove(&request_id) {
                    let _ = waiter.send(response.clone());
                }
                events_tx
                    .send(ExecutionEvent::Message(Box::new(
                        StreamMessage::ControlResponse(response),
                    )))
                    .await
                    .is_ok()
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
    async fn feed(lines: &[&str]) -> (bool, Option<CapabilityError>, Vec<ExecutionEvent>, Vec<String>) {
        let (events_tx, mut events_rx) = mpsc::channel(64);
        let (writer_tx, mut writer_rx) = mpsc::channel(64);
        let pending_control: PendingControl = Arc::new(Mutex::new(Default::default()));
        let (gate_tx, _gate_rx) = oneshot::channel();

        let mut gate_tx = Some(gate_tx);
        let mut gated = false;
        let mut refused: Option<CapabilityError> = None;
        let mut envelopes: Vec<ResultMessage> = Vec::new();
        let mut keep_going = true;

        for line in lines {
            keep_going = handle_item(
                ReaderItem::Envelope(parse_line(line)),
                &events_tx,
                &writer_tx,
                &pending_control,
                &mut gate_tx,
                &mut gated,
                &mut refused,
                &mut envelopes,
                "FIRST-MESSAGE",
                PermissionMode::DontAsk,
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
            "the prompt is released exactly once, on the first init: {written:?}"
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

    #[tokio::test]
    async fn a_refused_first_init_never_releases_the_prompt() {
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
            "a refused run must never release the prompt to stdin (D-06, TRANS-04): {written:?}"
        );
    }

    // ========================================================================
    // A refused run writes zero bytes to the child's stdin (D-06, TRANS-04)
    // ========================================================================

    const FAKE_CLAUDE_ECHO: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/fake-claude-echo.sh"
    );

    #[tokio::test]
    async fn a_refused_run_writes_zero_bytes_to_the_child_stdin() {
        let scratch = tempfile::TempDir::new().expect("temp dir");
        let stdin_log = scratch.path().join("stdin.log");

        // One capability short of the required set, everything else healthy.
        let executor = ClaudeExecutor::with_program(
            FAKE_CLAUDE_ECHO,
            vec![
                OsString::from("interrupt_receipt_v1,msg_lifecycle_v1"),
                OsString::from("2.1.220"),
                OsString::from("none"),
                stdin_log.clone().into_os_string(),
            ],
        );
        let project = DrivableProject::for_testing("refused", scratch.path());

        let err = executor
            .start(
                &project,
                "/gsd-progress".to_string(),
                ExecutionOptions::default(),
            )
            .await
            .expect_err("a CLI missing a required capability must be refused up front");

        assert!(
            matches!(
                err,
                SpawnError::Capability(CapabilityError::MissingCapabilities { .. })
            ),
            "expected a capability refusal, got: {err:?}"
        );

        // The stand-in truncates its stdin log before writing its init, so the
        // file existing proves the child ran; its length proves what we wrote.
        let recorded = std::fs::metadata(&stdin_log)
            .expect("the stand-in truncates the stdin log at startup, so it must exist");
        assert_eq!(
            recorded.len(),
            0,
            "a refused run must write zero bytes to the child's stdin — the refusal costs zero tokens and zero quota (D-06)"
        );
    }
}
