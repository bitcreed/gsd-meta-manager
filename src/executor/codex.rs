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
///
/// And exactly `GSD_RUNTIME` (quick 260926-gtm; `GSD_RUNTIME_*` siblings and
/// other `GSD_*` variables are kept). **Scrubbed, never set to `codex`:**
///   - an inherited value describes the launching shell or session, not the
///     driven one — the same class as `CODEX_THREAD_ID` above;
///   - gsd-core 1.15.0 (#4717) ranks it above the project's `config.runtime`
///     and the loaded install's `.gsd-runtime` marker, so a leaked
///     `GSD_RUNTIME=claude` would give the Codex child Claude spelling, agents
///     dir and model tiers;
///   - once removed, the child's own GSD resolves `config.runtime`, then the
///     marker of the install it loads (post-#4667 the Codex one), then host
///     detection — no assertion from the manager needed;
///   - setting it to `codex` would override an explicit project `runtime`,
///     which #4717 deliberately never does, and couple the manager into GSD's
///     runtime ladder (the inverse of ID-2 in `runtime.rs`);
///   - envelope entries are applied after this scrub in `start_run`, so an
///     explicit envelope value would still win.
pub fn scrubbed_from_codex_child(key: &OsStr) -> bool {
    let key = key.to_string_lossy();
    if key == "GSD_RUNTIME" {
        return true;
    }
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

    #[test]
    fn a_dash_leading_prompt_is_the_one_element_after_the_terminator() {
        let prompt = "-m gpt-evil --add-dir / ; rm -rf ~";
        let argv = argv_strings(&ExecutionOptions::default(), prompt);
        let terminator = argv
            .iter()
            .position(|arg| arg == "--")
            .expect("the argv carries a terminator");
        assert_eq!(terminator, argv.len() - 2, "`--` is second to last");
        assert_eq!(
            argv.last().map(String::as_str),
            Some(prompt),
            "the whole prompt is ONE element, after `--`, and last (CWE-88)"
        );
    }

    #[test]
    fn the_model_flag_appears_only_when_a_model_is_set() {
        let unset = argv_strings(&ExecutionOptions::default(), "p");
        assert!(!unset.iter().any(|arg| arg == "-m"), "{unset:?}");

        let options = ExecutionOptions {
            model: Some("gpt-5-codex".to_string()),
            ..Default::default()
        };
        let set = argv_strings(&options, "p");
        let at = set
            .iter()
            .position(|arg| arg == "-m")
            .expect("-m is emitted for a set model");
        assert_eq!(set[at + 1], "gpt-5-codex");
        assert!(
            at < set.iter().position(|arg| arg == "--").expect("terminator"),
            "the model flag sits before the terminator"
        );
    }

    #[test]
    fn the_add_dir_is_always_the_projects_git_dir() {
        let argv = argv_strings(&ExecutionOptions::default(), "p");
        let at = argv
            .iter()
            .position(|arg| arg == "--add-dir")
            .expect("--add-dir is always emitted");
        assert_eq!(argv[at + 1], "/work/proj/.git");
    }

    #[test]
    fn what_codex_cannot_honour_is_refused_before_anything_is_built() {
        let seam = ExecutionOptions {
            profile: SpawnProfile::ModelSeam {
                json_schema: "{}".to_string(),
            },
            ..Default::default()
        };
        let resume = ExecutionOptions {
            resume_session: Some("t-1".to_string()),
            ..Default::default()
        };
        let budget = ExecutionOptions {
            budget_usd: Some(1.0),
            ..Default::default()
        };
        for (what, options) in [("model seam", seam), ("resume", resume), ("budget", budget)] {
            let err = build_codex_argv(&options, Path::new("/work/proj"), "p")
                .expect_err("refused under codex");
            assert!(
                matches!(
                    err,
                    SpawnError::UnsupportedByRuntime {
                        runtime: "codex",
                        ..
                    }
                ),
                "{what}: got {err:?}"
            );
        }
    }

    /// The permission-bypass flag family and the full-access sandbox value,
    /// assembled at RUNTIME from halves so no line of this file carries them
    /// (the `tests/spawn_seam_guard.rs` src scan would otherwise report this
    /// test itself).
    fn forbidden_needles() -> Vec<String> {
        vec![
            format!("{}{}", "--danger", "ously"),
            format!("{}{}", "danger-full", "-access"),
            // `--yolo` is the documented alias of the bypass flag.
            format!("{}{}", "--yo", "lo"),
        ]
    }

    #[test]
    fn no_combination_of_options_widens_the_sandbox_or_bypasses_approval() {
        let needles = forbidden_needles();
        for model in [None, Some("gpt-5-codex".to_string())] {
            for tools in [Vec::new(), vec!["Bash(git push:*)".to_string()]] {
                for settings in [None, Some(std::path::PathBuf::from("/tmp/settings.json"))] {
                    let options = ExecutionOptions {
                        model: model.clone(),
                        envelope_disallowed_tools: tools.clone(),
                        envelope_settings: settings.clone(),
                        ..Default::default()
                    };
                    let argv = argv_strings(&options, "p");
                    let sandbox = argv
                        .iter()
                        .position(|arg| arg == "-s")
                        .expect("the sandbox is always named");
                    assert_eq!(argv[sandbox + 1], "workspace-write", "{argv:?}");
                    for arg in &argv {
                        for needle in &needles {
                            assert!(!arg.contains(needle.as_str()), "{arg:?} in {argv:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn the_scrub_keeps_codex_home_and_the_ca_and_drops_every_other_agent_variable() {
        for kept in ["CODEX_HOME", "CODEX_CA_CERTIFICATE", "PATH", "HOME", "CODEXHOME"] {
            assert!(!scrubbed_from_codex_child(OsStr::new(kept)), "{kept} is kept");
        }
        for dropped in [
            "CODEX_THREAD_ID",
            "CODEX_SESSION_ID",
            "CODEX_CI",
            "CODEX_SANDBOX_NETWORK_DISABLED",
            "CLAUDE_CODE_ENTRYPOINT",
            "CLAUDECODE",
        ] {
            assert!(
                scrubbed_from_codex_child(OsStr::new(dropped)),
                "{dropped} is scrubbed"
            );
        }
    }

    #[test]
    fn the_scrub_drops_exactly_gsd_runtime_and_no_gsd_sibling() {
        assert!(
            scrubbed_from_codex_child(OsStr::new("GSD_RUNTIME")),
            "an inherited GSD_RUNTIME describes the launching session and would \
             outrank the child's own config.runtime and install marker (#4717)"
        );
        for kept in [
            "GSD_RUNTIME_ROOT",
            "GSD_RUNTIMES",
            "GSD_TOOLS",
            "GSD_MM_ENVELOPE_ROOT",
            "PATH",
            "HOME",
            "CODEX_HOME",
        ] {
            assert!(
                !scrubbed_from_codex_child(OsStr::new(kept)),
                "{kept} is kept: the GSD_RUNTIME scrub is an exact-name match"
            );
        }
    }

    /// A handle that is not attached to any process: only the fields `send`
    /// and `interrupt` could possibly touch need to be real.
    fn detached_handle() -> ExecutionHandle {
        let (_events_tx, events) = mpsc::channel(1);
        let (stdin_tx, _stdin_rx) = mpsc::channel(1);
        ExecutionHandle {
            id: ExecutionId(Uuid::new_v4()),
            session_id: "t-1".to_string(),
            events,
            capabilities: Vec::new(),
            pgid: 0,
            claude_code_version: String::new(),
            pending_control: Arc::new(Mutex::new(Default::default())),
            control_response_cap: std::time::Duration::from_secs(1),
            stdin_tx,
            running: Arc::new(AtomicBool::new(true)),
            cancel_tx: None,
            outcome_rx: None,
            outcome: None,
            next_control_seq: 0,
        }
    }

    #[tokio::test]
    async fn send_and_interrupt_are_typed_refusals_under_codex() {
        let executor = runtime::AgentExecutor::new(AgentRuntime::Codex);
        let mut handle = detached_handle();
        let sent = executor
            .send(&mut handle, UserMessage::text("steer"))
            .await
            .expect_err("codex has no stdin channel");
        assert!(
            matches!(
                sent,
                SendError::UnsupportedByRuntime {
                    runtime: "codex",
                    ..
                }
            ),
            "got {sent:?}"
        );
        let interrupted = executor
            .interrupt(&mut handle)
            .await
            .expect_err("codex has no control channel");
        assert!(
            matches!(
                interrupted,
                SendError::UnsupportedByRuntime {
                    runtime: "codex",
                    ..
                }
            ),
            "got {interrupted:?}"
        );
    }
}
