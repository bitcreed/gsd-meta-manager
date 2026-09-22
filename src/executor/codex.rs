// The `codex exec --json` transport (260922-hdj).
//
// A strict subset of the Claude transport, and deliberately built ON it rather
// than beside it: the process-group supervisor, the caps, the teardown, the
// stderr reader and the outcome derivation are `claude.rs`'s, reused through
// `pub(super)` visibility, so there is one supervisor to reason about and not
// two. What differs is only what differs on the wire, measured against
// codex-cli 0.155.1:
//
// * **The prompt rides argv, after `--`, as ONE element.** `codex exec` takes
//   one positional prompt and runs one turn; there is no stdin message
//   protocol, so there is nothing to withhold and nothing to steer with.
// * **stdin is `/dev/null`.** With stdin inherited, `codex exec` printed
//   "Reading additional input from stdin..." and blocked until an external
//   180 s timeout with zero stdout lines. A closed stdin is not an optimisation
//   here; it is the difference between a run and a hang.
// * **`-s workspace-write` plus `--add-dir <root>/.git`.** The default exec
//   sandbox is read-only, which cannot write GSD artifacts; `workspace-write`
//   makes `.git` read-only, which makes every GSD commit fail while the agent
//   narrates success. Adding the git dir as a writable root made `git commit`
//   succeed in the probe. Network stays off inside that sandbox. The
//   full-access sandbox value and the permission-bypass flag family are never
//   emitted (D-15 house rule), and tests assert that.
// * **The first `thread.started` is the start gate.** Codex announces no
//   capabilities, so there is nothing to validate; the gate is "the child is
//   speaking the protocol and has a session".
// * **No raw stream content is logged**, exactly as for Claude: counts only.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use process_wrap::tokio::{CommandWrap, KillOnDrop, ProcessGroup};
use tokio::io::BufReader;
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tokio::time::Instant;
use uuid::Uuid;

use crate::error::{SendError, SpawnError};
use crate::executor::claude::{
    capture_snapshot, read_bounded_line, read_stderr, BoundedLine, Coordinator, ReaderItem,
    StreamProtocol, EVENT_CHANNEL_CAPACITY, MAX_LINE_BYTES, WRITER_CHANNEL_CAPACITY,
};
use crate::executor::codex_json::{parse_codex_line, CodexStreamState};
use crate::executor::runtime::{self, AgentRuntime};
use crate::executor::stream_json::{Envelope, UserMessage};
use crate::executor::{
    BoxFuture, DrivableProject, ExecutionHandle, ExecutionId, ExecutionOptions, ExecutionTarget,
    Executor, InterruptAck, PendingControl, RunOutcome, SpawnProfile,
};

/// The runtime word every refusal from this module names.
const RUNTIME: &str = "codex";

/// Build the `codex` argv for one run. **Pure**: no filesystem, no environment.
///
/// In order: `exec --json -s workspace-write --add-dir <root>/.git -C <root>`,
/// then `-m <model>` only when a model is set, then `--`, then the prompt as a
/// single element. Pushed element by element into a vector handed to
/// `execve` with no shell, so a prompt carrying a flag-shaped or
/// metacharacter-laden token cannot become an option or code (CWE-88,
/// T-hdj-01); the `--` is what keeps a prompt that *starts* with `-` a prompt.
///
/// Refuses, before anything exists to launch, what Codex cannot honour: the
/// model seam, a resume, a budget cap (260922-hdj ID-5). The other
/// Claude-only options — `name`, `setting_sources`, `permission_mode`,
/// `bg_wait_ceiling_ms` and the envelope's deny list and settings path — have
/// no Codex carrier and are ignored; each Codex run says so in its journal.
pub fn build_codex_argv(
    options: &ExecutionOptions,
    root: &Path,
    prompt: &str,
) -> Result<Vec<OsString>, SpawnError> {
    // Exhaustive, no wildcard, for the reason `claude::build_argv` gives: a new
    // target or profile must land here as a compile error.
    match options.target {
        ExecutionTarget::Host => {}
    }
    match &options.profile {
        SpawnProfile::Executor => {}
        SpawnProfile::ModelSeam { .. } => {
            return Err(SpawnError::UnsupportedByRuntime {
                runtime: RUNTIME,
                feature: "a model seam (structured-output consultation)",
            })
        }
    }
    if options.resume_session.is_some() {
        return Err(SpawnError::UnsupportedByRuntime {
            runtime: RUNTIME,
            feature: "resuming a session",
        });
    }
    if options.budget_usd.is_some() {
        return Err(SpawnError::UnsupportedByRuntime {
            runtime: RUNTIME,
            feature: "a USD budget cap",
        });
    }

    let mut argv: Vec<OsString> = Vec::new();
    push(&mut argv, "exec");
    push(&mut argv, "--json");
    push(&mut argv, "-s");
    push(&mut argv, "workspace-write");
    push(&mut argv, "--add-dir");
    push(&mut argv, root.join(".git"));
    push(&mut argv, "-C");
    push(&mut argv, root);
    if let Some(model) = &options.model {
        push(&mut argv, "-m");
        push(&mut argv, model);
    }
    push(&mut argv, "--");
    push(&mut argv, prompt);
    Ok(argv)
}

fn push(argv: &mut Vec<OsString>, arg: impl AsRef<OsStr>) {
    argv.push(arg.as_ref().to_os_string());
}

/// Drives the `codex` CLI through `codex exec --json`.
#[derive(Debug, Clone)]
pub struct CodexExecutor {
    program: PathBuf,
    leading_args: Vec<OsString>,
    /// Where the agent's process group id is published the instant the child
    /// exists. Same take-once `Arc<Mutex<Option<..>>>` shape as
    /// `ClaudeExecutor`'s, for the same reason: it keeps `Clone` and `Debug`
    /// derivable over a `oneshot::Sender`.
    spawn_observer: Arc<Mutex<Option<oneshot::Sender<u32>>>>,
}

impl Default for CodexExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl CodexExecutor {
    /// Drive the `codex` binary found on `PATH`.
    pub fn new() -> Self {
        Self::with_program(AgentRuntime::Codex.program(), Vec::new())
    }

    /// Drive a different program, with fixed leading arguments placed before
    /// the generated argv — the fixture path, exactly as
    /// `ClaudeExecutor::with_program` is.
    pub fn with_program(program: impl Into<PathBuf>, leading_args: Vec<OsString>) -> Self {
        Self {
            program: program.into(),
            leading_args,
            spawn_observer: Arc::new(Mutex::new(None)),
        }
    }

    /// Publish the agent's process group id on `tx` the instant the child
    /// exists, before anything is awaited — the invariant the driver's
    /// stop-during-startup path relies on (CR-01).
    pub fn observing_spawn(mut self, tx: oneshot::Sender<u32>) -> Self {
        self.spawn_observer = Arc::new(Mutex::new(Some(tx)));
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
        // ID-4: the sandbox's writable git dir is `<root>/.git`, so it must BE
        // that directory. A worktree or submodule (`.git` is a file pointing
        // elsewhere) or a missing `.git` would leave every commit failing
        // read-only while the agent reports success.
        if !root.join(".git").is_dir() {
            return Err(SpawnError::UnsupportedByRuntime {
                runtime: RUNTIME,
                feature: "a project whose .git is not a directory (a worktree, a submodule, \
                          or no git repository)",
            });
        }

        // Translated HERE and nowhere else: everything upstream compares the
        // canonical `/gsd-…` spelling.
        let prompt = runtime::codex_command(&command);

        let mut argv = self.leading_args.clone();
        argv.extend(build_codex_argv(&options, &root, &prompt)?);

        // Captured before spawn so the delta has a floor even if the agent's
        // very first act is a write.
        let before = capture_snapshot(root.clone()).await;

        let program = self.program.clone();
        let cwd = root.clone();
        let envelope_env = options.envelope_env.clone();

        let mut wrap = CommandWrap::with_new(&program, |cmd| {
            cmd.args(&argv)
                .current_dir(&cwd)
                // A closed stdin, never an inherited one: see the module doc.
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            // This closure is the ONE place the Codex child's environment is
            // built, as `claude.rs`'s is for Claude. Inherited agent variables
            // are scrubbed first, then the envelope is applied.
            for (key, _) in std::env::vars_os() {
                if scrubbed_from_codex_child(&key) {
                    cmd.env_remove(&key);
                }
            }
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

        // The child leads its own group, so the pgid is its pid. Read on the
        // statement after the spawn and published before anything is awaited.
        let pgid = child.id().ok_or(SpawnError::PidUnavailable)?;
        if let Some(observer) = self.spawn_observer.lock().await.take() {
            let _ = observer.send(pgid);
        }

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
        let (writer_tx, mut writer_rx) = mpsc::channel(WRITER_CHANNEL_CAPACITY);
        let (gate_tx, gate_rx) = oneshot::channel();
        let (outcome_tx, outcome_rx) = oneshot::channel();
        let (cancel_tx, cancel_rx) = oneshot::channel();

        let (last_line_at, last_line_rx) = watch::channel(Instant::now());
        let last_line_at = Arc::new(last_line_at);

        let pending_control: PendingControl = Arc::new(Mutex::new(Default::default()));
        let running = Arc::new(AtomicBool::new(true));

        tokio::spawn(read_codex_stdout(
            stdout,
            reader_tx,
            Arc::clone(&last_line_at),
        ));
        tokio::spawn(read_stderr(stderr, events_tx.clone(), last_line_at));
        // There is no stdin to write to. The writer channel still exists so the
        // coordinator's `Close` and the handle's `close_input` succeed; this
        // task drains and discards until every sender is gone.
        tokio::spawn(async move { while writer_rx.recv().await.is_some() {} });

        tokio::spawn(
            Coordinator {
                child,
                protocol: StreamProtocol::CodexExecJson,
                reader_rx,
                events_tx,
                writer_tx: writer_tx.clone(),
                pending_control: Arc::clone(&pending_control),
                gate_tx,
                outcome_tx,
                cancel_rx,
                running: Arc::clone(&running),
                first_message: String::new(),
                permission_mode: options.permission_mode,
                wall_clock_cap: options.wall_clock_cap,
                idle_cap: options.idle_cap,
                prompt_release_grace: options.prompt_release_grace,
                last_line_rx,
                before,
                project_root: root,
                replay_observer: None,
            }
            .run(),
        );

        // No `thread.started` before the stream ends (a bad `-C`, an untrusted
        // directory: exit 1 with the reason on stderr only) drops the gate
        // sender, which is `InitNeverObserved` — and the stderr lines are
        // already on the event stream for the journal.
        let facts = match gate_rx.await {
            Ok(Ok(facts)) => facts,
            Ok(Err(err)) => return Err(err),
            Err(_) => return Err(SpawnError::InitNeverObserved),
        };

        Ok(ExecutionHandle {
            id: ExecutionId(Uuid::new_v4()),
            session_id: facts.session_id.unwrap_or_default(),
            events: events_rx,
            capabilities: Vec::new(),
            pgid,
            claude_code_version: String::new(),
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

/// Whether an inherited variable is removed from the Codex child's
/// environment (260922-hdj ID-8).
///
/// Every `CLAUDE*` variable, for the reason `claude.rs` scrubs them. Every
/// `CODEX_*` variable too — a TUI launched from inside a Codex shell inherits
/// `CODEX_THREAD_ID`, `CODEX_SESSION_ID`, `CODEX_CI` and
/// `CODEX_SANDBOX_NETWORK_DISABLED`, which describe THAT session, not the
/// driven one — **except** `CODEX_HOME` (where the user's config and auth
/// live) and `CODEX_CA_CERTIFICATE` (a TLS trust anchor the user chose).
pub fn scrubbed_from_codex_child(key: &OsStr) -> bool {
    let key = key.to_string_lossy();
    if key.starts_with("CLAUDE") {
        return true;
    }
    key.starts_with("CODEX_") && key != "CODEX_HOME" && key != "CODEX_CA_CERTIFICATE"
}

/// The Codex stdout reader. Parsing happens here, never in the coordinator.
///
/// Same framing, bound and liveness stamp as the Claude reader; only the
/// parser differs. Logs counts and shapes only, never content.
async fn read_codex_stdout(
    stdout: tokio::process::ChildStdout,
    tx: mpsc::Sender<ReaderItem>,
    last_line_at: Arc<watch::Sender<Instant>>,
) {
    let mut reader = BufReader::new(stdout);
    let mut state = CodexStreamState::default();
    loop {
        let observed = read_bounded_line(&mut reader).await;
        if !matches!(observed, Ok(BoundedLine::Eof)) {
            last_line_at.send_replace(Instant::now());
        }
        let item = match observed {
            Ok(BoundedLine::Eof) => break,
            Ok(BoundedLine::Line(line)) => {
                if line.trim().is_empty() {
                    continue;
                }
                match parse_codex_line(&line) {
                    Ok(event) => ReaderItem::Codex {
                        step: state.step(event),
                        raw: line,
                    },
                    Err(error) => ReaderItem::Envelope(Envelope::Unparseable {
                        raw: line,
                        error: error.to_string(),
                    }),
                }
            }
            Ok(BoundedLine::Truncated { bytes, prefix }) => {
                tracing::warn!(
                    "codex stdout line of {} bytes exceeded the {} byte bound and was discarded",
                    bytes,
                    MAX_LINE_BYTES
                );
                ReaderItem::Truncated { bytes, prefix }
            }
            Err(err) => {
                tracing::warn!(kind = ?err.kind(), "codex stdout read failed");
                break;
            }
        };
        if tx.send(item).await.is_err() {
            break;
        }
    }
}

impl Executor for CodexExecutor {
    fn start<'a>(
        &'a self,
        project: &'a DrivableProject,
        command: String,
        options: ExecutionOptions,
    ) -> BoxFuture<'a, Result<ExecutionHandle, SpawnError>> {
        Box::pin(self.start_run(project, command, options))
    }

    /// Refused: `codex exec` reads no stdin, so a mid-run message has no
    /// channel. The driver journals it as undelivered rather than losing it.
    fn send<'a>(
        &'a self,
        _handle: &'a mut ExecutionHandle,
        _message: UserMessage,
    ) -> BoxFuture<'a, Result<(), SendError>> {
        Box::pin(async {
            Err(SendError::UnsupportedByRuntime {
                runtime: RUNTIME,
                operation: "delivering a message",
            })
        })
    }

    /// Refused: there is no control channel. Stopping a Codex run is
    /// [`Executor::cancel`], which tears the process group down.
    fn interrupt<'a>(
        &'a self,
        _handle: &'a mut ExecutionHandle,
    ) -> BoxFuture<'a, Result<InterruptAck, SendError>> {
        Box::pin(async {
            Err(SendError::UnsupportedByRuntime {
                runtime: RUNTIME,
                operation: "an interrupt",
            })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn argv_strings(options: &ExecutionOptions, prompt: &str) -> Vec<String> {
        build_codex_argv(options, Path::new("/work/proj"), prompt)
            .expect("an executor-profile run builds")
            .into_iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_argv_is_exactly_the_documented_shape() {
        let argv = argv_strings(&ExecutionOptions::default(), "$gsd-progress");
        assert_eq!(
            argv,
            [
                "exec",
                "--json",
                "-s",
                "workspace-write",
                "--add-dir",
                "/work/proj/.git",
                "-C",
                "/work/proj",
                "--",
                "$gsd-progress",
            ]
        );
    }
}
