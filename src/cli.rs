use clap::{Args, Parser, Subcommand};
use std::ffi::OsString;
use std::path::PathBuf;

/// The largest `--max-budget-usd` accepted: a typo guard, far above any real run.
pub const MAX_BUDGET_USD: f64 = 100_000.0;
/// The largest `--cap-minutes` accepted (one year). systemd stores `RuntimeMaxSec`
/// as `u64` microseconds, so an unbounded value is rejected by `systemd-run` after
/// the launch was already recorded.
pub const MAX_CAP_MINUTES: f64 = 525_600.0;

/// A finite number in `(0, max]`; `inf`, `NaN`, zero and negatives are refused.
pub fn positive_finite(text: &str, what: &str, max: f64) -> Result<f64, String> {
    let value: f64 = text
        .parse()
        .map_err(|_| format!("{what} must be a number, got {text:?}"))?;
    if value.is_finite() && value > 0.0 && value <= max {
        Ok(value)
    } else {
        Err(format!(
            "{what} must be a finite number greater than 0 and at most {max}, got {text:?}"
        ))
    }
}

fn positive_finite_usd(text: &str) -> Result<f64, String> {
    positive_finite(text, "the budget", MAX_BUDGET_USD)
}

fn positive_finite_minutes(text: &str) -> Result<f64, String> {
    positive_finite(text, "the cap", MAX_CAP_MINUTES)
}

#[derive(Parser)]
#[command(
    name = "gsd-meta-manager",
    version,
    about = "TUI command center for managing multiple GSD-run projects from a single interface."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to config file (overrides default location)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    // The positional launch target (quick 260930-vvk, D-01). Not an
    // argv-to-identity declaration: it is a raw membership lookup against the
    // registry that creates nothing (the same class as `Remove`'s D-17-3), and
    // it is echoed back only through `text::render_for_terminal` inside
    // `launch_target::LaunchTargetError`'s `Display`.
    //
    // **Why `#[command(args_conflicts_with_subcommands = true)]` is NOT on
    // `Cli`.** It was the operator's named mechanism, and it breaks the very
    // "subcommands keep priority" rule it was meant to enforce. clap_builder
    // 4.6.7 sets `valid_arg_found` for ANY long flag, the global `--config`
    // included (parser.rs:821), and once that is set `possible_subcommand`
    // stops matching subcommand names (parser.rs:592). With the attribute,
    // `--config X list` parses as target "list" and `--config X add p` is an
    // error — and a leading `--config X` before a subcommand is exactly how
    // the binary re-enters itself, so that would break invisibly. clap's
    // default already gives subcommands priority; the one gap (`<target> list`) is closed by `Cli::try_parse_checked_from`'s
    // post-parse conflict check instead.
    /// Alias or path of a registered project to open directly
    ///
    /// An alias that shares its name with a subcommand (e.g. `list`) must be
    /// opened by its path instead, such as `gsd-meta-manager ./list` or the
    /// absolute path.
    pub target: Option<String>,
}

// The post-parse conflict check below is what replaces
// `args_conflicts_with_subcommands = true` (see the comment on `Cli::target`).
// clap's default already gives subcommands priority over the positional; the
// only combination it accepts that the attribute would have refused is
// `<target> <subcommand>`, and that is refused here as a usage error instead.
impl Cli {
    /// Parse `argv`, refusing a launch target combined with a subcommand.
    pub fn try_parse_checked_from<I, T>(argv: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        let cli = <Self as Parser>::try_parse_from(argv)?;
        if let (Some(target), Some(_)) = (&cli.target, &cli.command) {
            let msg = format!(
                "the project target '{}' cannot be combined with a subcommand; \
                 to open a project whose alias is a subcommand name, pass its path \
                 instead (e.g. `./list`)",
                crate::text::render_for_terminal(target)
            );
            return Err(<Self as clap::CommandFactory>::command()
                .error(clap::error::ErrorKind::ArgumentConflict, msg));
        }
        Ok(cli)
    }

    /// Parse the process argv with [`Cli::try_parse_checked_from`], letting
    /// clap print and exit on error (2 for usage errors, 0 for help/version).
    pub fn parse_checked() -> Self {
        Self::try_parse_checked_from(std::env::args_os()).unwrap_or_else(|err| err.exit())
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a GSD project to the registry
    Add {
        /// Path to the project root (must contain .planning/)
        path: PathBuf,
        /// Optional alias (defaults to last folder component of path)
        alias: Option<String>,
    },
    /// Remove a project from the registry
    Remove {
        /// Alias of the project to remove
        alias: String,
    },
    /// List all registered projects
    List,
    // The envelope's evidence-gate scan (Phase 31 kept only `envelope scan`).
    //
    // **`hide = true` alone is the right treatment here, and
    // `#[cfg(debug_assertions)]` would be wrong.** A subcommand that exists only
    // in a debug build is an entry point that exits 2 with "unrecognized
    // subcommand" in every shipped configuration; hiding it only keeps `--help`
    // honest about what a user is meant to type.
    /// Internal: scan a tree for credential shapes (the acceptance scripts' evidence gate)
    #[command(hide = true)]
    Envelope {
        #[command(subcommand)]
        action: EnvelopeAction,
    },
    // The sandbox-side half of the driver's control channel (Phase 27, D-01,
    // D-02). The driver's unchanged prompt calls `driverctl log|show` and the
    // Claude hooks call `driverctl hook`, so the binary re-enters itself.
    //
    // **Hidden, because users never type it** (D-02): `hide = true` removes the
    // entry from `--help` and from nothing else, the general rule the
    // `Commands::Envelope` block above states at length.
    //
    // **Deliberately NOT `#[cfg]`-gated.** The sandboxed driver and the hook
    // settings reach this through the released binary on every platform, and a
    // refusal must be a typed error from the handler, not "unknown subcommand"
    // from the parser. One build, one version (Phase 17 D-01).
    //
    // `serve` (27-06) is the launcher-side server; the sandbox never runs it.
    /// Internal: the driver's control channel (log events, show status, hook gate)
    #[command(hide = true)]
    Driverctl {
        #[command(subcommand)]
        action: DriverctlAction,
    },
    // The sandbox-side half of a one-off run (quick 261103-f5y, C-12): the agent
    // calls `taskctl report` once it is finished, and the shim in the run's config
    // dir pins the report directory.
    //
    // **Hidden, because users never type it**, for the reason `Driverctl` is, and
    // **deliberately NOT `#[cfg]`-gated** for the same reason: a refusal is a typed
    // error from the handler, never "unknown subcommand". `main` dispatches it
    // before `init_tracing`: it runs inside the sandbox and creates no log file.
    /// Internal: the one-off agent's strict report CLI
    #[command(hide = true)]
    Taskctl {
        #[command(subcommand)]
        action: TaskctlAction,
    },
    // The sandbox's one way out (Phase 28.1, D-08). bwrap runs this as its
    // command inside the network-isolated profile: it binds `127.0.0.1:<port>`,
    // relays every connection to the bound-in egress socket, runs the driver
    // after `--` as its child and hands the child's exit code back to bwrap.
    //
    // **Hidden, because users never type it**: the launcher builds the bwrap
    // argv that names it, the general rule the `Commands::Envelope` block above
    // states at length.
    //
    // **Deliberately NOT `#[cfg]`-gated**, for the reason `Driverctl` is not: the
    // sandbox re-enters the released binary, so a platform without the launcher
    // gets a typed refusal from the handler, not "unknown subcommand".
    //
    // `main` dispatches it before `init_tracing`: it creates no file and writes
    // nothing to stdout (claude's stream-json shares it).
    /// Internal: the in-sandbox TCP-to-unix-socket bridge that runs the driver
    #[command(hide = true)]
    Bridge {
        /// Loopback TCP port to listen on (127.0.0.1)
        #[arg(long)]
        port: u16,
        /// Unix socket of the launcher-side egress server
        #[arg(long)]
        sock: PathBuf,
        /// The driver command line, after `--`
        #[arg(last = true, required = true, num_args = 1..)]
        command: Vec<OsString>,
    },
    // The launcher half of the autonomous driver (Phase 28, D-01): the host-side
    // owner of the run lock, the embedded store, the per-run Claude config, auth
    // and the sandbox. Unlike `driverctl`, users type this one, so it is NOT
    // hidden. It is not `#[cfg]`-gated either: on a platform without the launcher
    // the handler returns a typed `UnsupportedPlatform`, not "unknown subcommand".
    /// Launch and control sandboxed driver runs
    Driver {
        #[command(subcommand)]
        action: DriverAction,
    },
    // One-off sandboxed task dispatch (quick 261103-f5y): the host verbs. Users type
    // these, so they are NOT hidden (I-12); not `#[cfg]`-gated either: off Linux and
    // macOS the handler returns a typed refusal. Not flag-gated: the experimental flag
    // is a discovery gate for the TUI, as it is for `driver`.
    /// Run one sandboxed /gsd-quick or /gsd-fast session on a project, and land its work
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    // The read-only half of the master prompt's "what next" citation (Phase 29,
    // D-20, D-21). The Phase 20 router answers from the state reader and nothing
    // else, so the hint needs no registry write, no lock, no journal and no log
    // file; `main` therefore dispatches it before `init_tracing`, like
    // `driverctl`, and the sandbox shim exposes exactly this verb.
    //
    // `--json` is required (not defaulted) so the output contract is chosen
    // explicitly and a future human-readable form has room to exist.
    /// Advisory next GSD command from the Phase 20 router (read-only)
    Next {
        /// Alias of a registered project, or a path to a directory containing .planning/
        project: String,
        /// The phase to route (defaults to the first unfinished current-milestone phase)
        #[arg(long)]
        target_phase: Option<String>,
        /// Print the hint as one JSON object on stdout
        #[arg(long, required = true)]
        json: bool,
    },
}

/// The verbs of `driver`.
#[derive(Subcommand)]
pub enum DriverAction {
    // Boxed for the reason `DriverctlAction::Log` is: the argument struct would
    // otherwise size the whole enum (`clippy::large_enum_variant`).
    /// Run the headless driver for an existing run inside the sandbox
    Launch(Box<LaunchArgs>),
    /// Stop a run: signal its launcher, which stops the whole sandbox scope
    Stop {
        /// Alias or path of a registered project
        project: String,
        /// The run id: `.planning/meta-manager/drivers/<run-id>`
        run_id: String,
        /// How long to wait for the launcher to finish stopping, in seconds
        #[arg(long, default_value_t = 60)]
        wait_secs: u64,
    },
    /// Push a finished run's work: its recorded branch tip, scanned, from the
    /// launcher's own mirror into `refs/heads/gsd-auto/<alias>/<run-id>`
    Push {
        /// Alias or path of a registered project
        project: String,
        /// The run id: `.planning/meta-manager/drivers/<run-id>`
        run_id: String,
        /// Push even though planted git files were found in the checkout, after
        /// you inspected them
        #[arg(long)]
        ack_planted: bool,
    },
    /// Create a new run: a self-ignoring run directory with a DIRECTION.md and LOG.md
    /// template (id `<yyyymmdd>-<slug>`, UTC)
    New {
        /// Alias or path of a registered project
        project: String,
        /// The run's name: ASCII letters, digits, `.`, `_`, `-`; not starting with `.` or `-`
        slug: String,
    },
    /// Lint a run's DIRECTION.md: print every problem with its line and exit 1,
    /// or nothing and exit 0 (read-only)
    Check {
        /// Alias or path of a registered project
        project: String,
        /// The run id: `.planning/meta-manager/drivers/<run-id>`
        run_id: String,
    },
    /// The driver's long-lived OAuth token: store one, or see what is stored
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
}

/// The verbs of `driver auth` (D-21).
#[derive(Subcommand, Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthAction {
    /// Run `claude setup-token`, then store the long-lived token you paste (0600)
    Setup,
    /// Report whether a token is stored, where, and when the host token expires
    /// (never a token value)
    Status,
}

/// The master prompt's loop mode (`{MODE}` in the prompt).
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverLoopMode {
    /// One driver context for the whole run
    Loop,
    /// A fresh driver context per item
    FreshPerItem,
}

/// The launcher's default gap-round cap per phase (`driver launch --cap-gap-rounds`, D-14).
/// The DIRECTION lint warns about an item budget above it, so the lint reads this one
/// constant rather than a hand-copied `2` (IN-04).
pub const DEFAULT_CAP_GAP_ROUNDS: u32 = 2;

/// Arguments of `driver launch <project> <run-id>`.
///
/// The run directory must already exist (D-02): minting one is `driver new`
/// (Phase 29).
#[derive(Args, Debug, Clone)]
pub struct LaunchArgs {
    /// Alias or path of a registered project
    pub project: String,
    /// The run id: `.planning/meta-manager/drivers/<run-id>` must exist
    pub run_id: String,
    /// Test and acceptance override: run this prompt file instead of the embedded
    /// `driver-prompt/X.Y.Z`. The run records it as `custom:<sha8>` (the first 8 hex
    /// of the SHA-256 of the file's bytes), never as the shipped version (D-04).
    /// `{RUN_DIR}`, `{PROJECT_ROOT}` and `{MODE}` are filled in.
    #[arg(long, hide = true)]
    pub prompt_file: Option<PathBuf>,
    /// The driver's model
    #[arg(long, default_value = "opus")]
    pub model: String,
    /// One context for the run, or a fresh one per item
    #[arg(long, value_enum, default_value_t = DriverLoopMode::Loop)]
    pub loop_mode: DriverLoopMode,
    /// The driver's spend ceiling, passed to `claude --max-budget-usd`
    #[arg(long, default_value_t = 10.0, value_parser = positive_finite_usd)]
    pub max_budget_usd: f64,
    /// Dispatch cap recorded on the run
    #[arg(long, default_value_t = 10)]
    pub cap_dispatches: u32,
    /// Active wall-clock cap in minutes, recorded on the run
    #[arg(long, default_value_t = 60.0, value_parser = positive_finite_minutes)]
    pub cap_minutes: f64,
    /// Gap-round cap per phase, recorded on the run
    #[arg(long, default_value_t = DEFAULT_CAP_GAP_ROUNDS)]
    pub cap_gap_rounds: u32,
    /// Stay in this terminal for the run's life; without it the launcher detaches
    /// into its own session and logs to `launcher.log` under the run's state dir
    #[arg(long)]
    pub foreground: bool,
    /// Internal: this process is the re-executed detached launcher (D-04)
    #[arg(long, hide = true)]
    pub daemon_child: bool,
    /// Run the driver WITHOUT bwrap and a systemd scope, as a supervised process
    /// group (degraded; D-29). Per launch, never a default: without it a host that
    /// cannot sandbox is refused. The run records `sandbox: degraded`.
    #[arg(long)]
    pub unsandboxed: bool,
}

/// `driver launch`'s own parser, reachable without a command line (30-05, D-19).
///
/// The Driver tab's launch form needs the launch defaults and a per-field validity
/// verdict, and both belong to clap: a TUI copy of `10.0`, `60.0` or the bounds of
/// `positive_finite` would be a second judge of the same values. Flattening
/// [`LaunchArgs`] into a private parser lets the form ask the real one.
#[derive(Parser, Debug)]
#[command(name = "launch", disable_help_flag = true, disable_version_flag = true)]
struct LaunchFieldParser {
    #[command(flatten)]
    args: LaunchArgs,
}

/// The two positionals every launch parse needs. The caller never uses them: the form
/// carries the run's real id and project itself.
const LAUNCH_PLACEHOLDERS: [&str; 3] = ["launch", "_", "_"];

/// What `driver launch <project> <run>` means with no flag given: the clap defaults of
/// [`LaunchArgs`]. `project` and `run_id` hold placeholders.
pub fn launch_defaults() -> LaunchArgs {
    LaunchFieldParser::try_parse_from(LAUNCH_PLACEHOLDERS)
        .expect("the launch defaults always parse")
        .args
}

/// Parse `driver launch` with one flag set, e.g. `("--cap-minutes", "45")`, and report
/// clap's own verdict on it. The form uses `.is_ok()`; the error text is clap's.
///
/// The flag and value travel as one `--flag=value` word, so a value that starts with
/// `-` is a value, not another flag.
pub fn parse_launch_field(flag: &str, value: &str) -> Result<LaunchArgs, String> {
    let word = format!("{flag}={value}");
    LaunchFieldParser::try_parse_from(LAUNCH_PLACEHOLDERS.into_iter().chain([word.as_str()]))
        .map(|parsed| parsed.args)
        .map_err(|error| error.to_string())
}

/// The sandbox-side verbs of `driverctl`.
///
/// Each returns a process exit code, because the Claude hook protocol and the
/// driver's prompt both read it as control (`driverctl_cli::run`).
#[derive(Subcommand)]
pub enum DriverctlAction {
    // Boxed: `DriverctlLog` is 17 optional strings, and unboxed it makes this
    // enum (and through it `Commands`) the size of that struct for the sake of two
    // unit variants (`clippy::large_enum_variant`).
    /// Record one event on the run's log
    Log(Box<DriverctlLog>),
    /// Print the run's current status block
    Show,
    /// Claude hook entry point: read one hook payload on stdin
    Hook,
    /// Stop hook entry point: keep the driver alive while a dispatch is unreported
    // Hidden: only the seeded settings.json calls it (D-17), with the run dir it
    // was built for. It always exits 0.
    #[command(hide = true)]
    StopGuard {
        /// The run directory whose LOG.md is read
        #[arg(long)]
        run_dir: Option<PathBuf>,
    },
    /// Launcher side: serve one run's store on a unix socket until SIGTERM/SIGINT
    Serve {
        /// The run directory: `<project>/.planning/meta-manager/drivers/<run-id>`
        #[arg(long)]
        run_dir: PathBuf,
        /// The project root the run belongs to
        #[arg(long)]
        project: PathBuf,
        /// Socket path (default: `$XDG_RUNTIME_DIR/gsdmm-driverctl/<run-id>-<hash>.sock`;
        /// on macOS without XDG_RUNTIME_DIR, under `$TMPDIR`; a fixed-length hashed
        /// name when that would exceed the unix socket path limit)
        #[arg(long)]
        sock: Option<PathBuf>,
    },
}

// The 17 flags below are spike 002's argparse surface verbatim, and every value
// stays a string. The client validates nothing a server would not: the one
// parser that judges a value is the store (D-01), so adding a `value_parser`
// here would be a second judge of the same property. The wire keys are the
// underscored field names (`done_when`, `loop_mode`); clap derives the dashed
// long flags (`--done-when`, `--loop-mode`) from the same names.
/// Arguments of `driverctl log <event>`
#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct DriverctlLog {
    /// The event name, e.g. `start`, `dispatched`, `report`
    pub event: String,
    #[arg(long)]
    pub item: Option<String>,
    #[arg(long)]
    pub step: Option<String>,
    #[arg(long)]
    pub round: Option<String>,
    #[arg(long)]
    pub phase: Option<String>,
    #[arg(long)]
    pub status: Option<String>,
    #[arg(long)]
    pub done_when: Option<String>,
    #[arg(long)]
    pub evidence: Option<String>,
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long)]
    pub rule: Option<String>,
    #[arg(long)]
    pub outcome: Option<String>,
    #[arg(long)]
    pub question: Option<String>,
    #[arg(long)]
    pub action: Option<String>,
    #[arg(long)]
    pub reason: Option<String>,
    #[arg(long)]
    pub found: Option<String>,
    #[arg(long)]
    pub mode: Option<String>,
    #[arg(long)]
    pub loop_mode: Option<String>,
    #[arg(long)]
    pub session: Option<String>,
}

/// The verbs of `task` (quick 261103-f5y).
#[derive(Subcommand)]
pub enum TaskAction {
    /// Run one headless agent session on a todo or a typed task, in a private clone,
    /// inside the launcher's sandbox (detaches unless --foreground)
    Run(Box<TaskRunArgs>),
    /// Show the project's latest one-off run
    Status {
        /// Alias or path of a registered project
        project: String,
        /// Print the record as one JSON object
        #[arg(long)]
        json: bool,
    },
    /// Stop the project's live one-off run
    Stop {
        /// Alias or path of a registered project
        project: String,
        /// How long to wait for the run to finish stopping, in seconds
        #[arg(long, default_value_t = 60)]
        wait_secs: u64,
    },
    /// Send a message to the project's running one-off agent (experimental)
    Send {
        /// Alias or path of a registered project
        project: String,
        /// The message (at most 4000 characters)
        #[arg(allow_hyphen_values = true)]
        text: String,
        /// How long to wait for the launcher to pick the message up, in seconds
        #[arg(long, default_value_t = 10)]
        wait_secs: u64,
    },
    /// Land a finished run's work: merge, push or discard
    Land {
        #[command(subcommand)]
        action: LandAction,
    },
}

/// The verbs of `task land`.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum LandAction {
    /// Fast-forward your checkout to the run's head (clean checkout on a branch only)
    Merge {
        /// Alias or path of a registered project
        project: String,
        /// The run id (`task status` shows it)
        run_id: String,
    },
    /// Push the run's head to refs/heads/gsd-auto/<alias>/<run-id> (scanned first)
    Push {
        /// Alias or path of a registered project
        project: String,
        /// The run id (`task status` shows it)
        run_id: String,
    },
    /// Delete the run's directory, its clone included
    Discard {
        /// Alias or path of a registered project
        project: String,
        /// The run id (`task status` shows it)
        run_id: String,
    },
}

/// Arguments of `task run <project> (--todo <file> | --text <task>)`.
#[derive(Args, Debug, Clone)]
#[command(group(clap::ArgGroup::new("task").required(true).args(["todo", "text"])))]
pub struct TaskRunArgs {
    /// Alias or path of a registered project
    pub project: String,
    /// A pending todo's file name under .planning/todos/pending/ (must be committed)
    #[arg(long)]
    pub todo: Option<String>,
    /// A typed task: one line, at most 2000 characters
    #[arg(long, allow_hyphen_values = true)]
    pub text: Option<String>,
    /// quick runs /gsd-quick, fast runs /gsd-fast
    #[arg(long, value_enum, default_value_t = crate::oneoff::Mode::Quick)]
    pub mode: crate::oneoff::Mode,
    /// The agent's model
    #[arg(long, default_value = "opus")]
    pub model: String,
    /// The agent's spend ceiling, in US dollars
    #[arg(long, default_value_t = 10.0, value_parser = positive_finite_usd)]
    pub max_budget_usd: f64,
    /// The wall clock in minutes, enforced from outside the sandbox
    #[arg(long, default_value_t = 60.0, value_parser = positive_finite_minutes)]
    pub cap_minutes: f64,
    /// Run WITHOUT bwrap and a systemd scope (degraded); recorded on the run
    #[arg(long)]
    pub unsandboxed: bool,
    /// Stay in this terminal for the run's life
    #[arg(long)]
    pub foreground: bool,
    /// Internal: this process is the re-executed detached run
    #[arg(long, hide = true)]
    pub daemon_child: bool,
    /// Internal: the run id the detached parent minted
    #[arg(long, hide = true)]
    pub run_id: Option<String>,
}

/// `task run`'s own parser, reachable without a command line (the
/// [`launch_defaults`] pattern): the TUI reads the defaults from clap, never a copy.
#[derive(Parser, Debug)]
#[command(name = "run", disable_help_flag = true, disable_version_flag = true)]
struct TaskRunFieldParser {
    #[command(flatten)]
    args: TaskRunArgs,
}

/// What `task run <project> --text <t>` means with no other flag: the clap defaults
/// of [`TaskRunArgs`]. `project` and `text` hold placeholders.
pub fn task_run_defaults() -> TaskRunArgs {
    TaskRunFieldParser::try_parse_from(["run", "_", "--text", "_"])
        .expect("the task run defaults always parse")
        .args
}

/// The sandbox-side verbs of `taskctl` (quick 261103-f5y).
#[derive(Subcommand)]
pub enum TaskctlAction {
    /// Record the one-off run's report (validated; exit 2 on a problem)
    Report(Box<TaskctlReport>),
}

/// Arguments of `taskctl report`. Every value is judged by
/// `oneoff::report::Report::validate`, the one validator the host uses too.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct TaskctlReport {
    /// done, partial, blocked or failed
    #[arg(long, value_enum)]
    pub status: crate::oneoff::report::ReportStatus,
    /// One line of at most 120 characters
    #[arg(long, allow_hyphen_values = true)]
    pub headline: String,
    /// What changed (repeatable, at most 5)
    #[arg(long, allow_hyphen_values = true)]
    pub changed: Vec<String>,
    /// `<command>: <result>` (repeatable, at most 5)
    #[arg(long, allow_hyphen_values = true)]
    pub verified: Vec<String>,
    /// A follow-up for a human (repeatable, at most 5)
    #[arg(long, allow_hyphen_values = true)]
    pub followup: Vec<String>,
}

/// The one `envelope` subcommand that survives Phase 31.
///
/// The hook, askpass and `PreToolUse` re-entry arms went with the argv-classifying
/// guard they served; what remains is the credential-shape scan, which the
/// acceptance scripts use as their evidence gate. Its exit code **is** the
/// verdict: non-zero on any finding.
#[derive(Subcommand)]
pub enum EnvelopeAction {
    /// Scan a worktree for credential shapes and exit non-zero on any finding
    ///
    /// The same scan the launcher's push step runs before it publishes a branch,
    /// reachable on its own so a human (or an acceptance script's evidence gate)
    /// can see what the push step would say **before** anything is pushed, and so
    /// the report - including the list of files the scan declined to read (D-14) -
    /// can be inspected without a remote in the picture.
    Scan {
        /// Registry alias the scan is reported against
        alias: String,
        /// Repository root to walk
        root: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    // Is this rendered help line the SUBCOMMAND ENTRY for `name`?
    //
    // The entry shape is matched rather than a bare `contains`, which would also
    // fire on the word appearing inside an unrelated description. Both the
    // positive control and the absence assertion call this one predicate, so the
    // control certifies the same code path the assertion runs — two matchers
    // could disagree and the control would then prove nothing about the
    // assertion.
    fn is_subcommand_entry(line: &str, name: &str) -> bool {
        let trimmed = line.trim();
        match trimmed.strip_prefix(name) {
            Some(rest) => rest.is_empty() || rest.starts_with(|c: char| c.is_ascii_whitespace()),
            None => false,
        }
    }

    // ── The positional project target (quick 260930-vvk) ───────────────

    fn checked(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_checked_from(std::iter::once("gsd-meta-manager").chain(args.iter().copied()))
    }

    #[test]
    fn the_project_target_loses_to_a_same_named_subcommand() {
        let cli = checked(&["list"]).expect("`list` parses");
        assert!(
            matches!(cli.command, Some(Commands::List)),
            "`list` must stay the subcommand, not become a launch target (D-01)"
        );
        assert_eq!(cli.target, None);
    }

    #[test]
    fn the_project_target_does_not_swallow_a_subcommand_after_config() {
        let cli = checked(&["--config", "/c.json", "list"]).expect("`--config X list` parses");
        assert!(
            matches!(cli.command, Some(Commands::List)),
            "`--config X list` must parse as the list subcommand. This pins the deviation from \
             `args_conflicts_with_subcommands`, which turns `list` into a target here because \
             clap marks any long flag as a valid arg first — the same leading `--config` the \
             TUI's `drive` respawn emits with /dev/null stdio, so it would break invisibly"
        );
        assert!(cli.config.is_some());
        assert_eq!(cli.target, None);
    }

    #[test]
    fn the_project_target_combines_with_config_in_either_order() {
        for args in [
            &["--config", "/c.json", "ttbook"][..],
            &["ttbook", "--config", "/c.json"][..],
        ] {
            let cli = checked(args).unwrap_or_else(|e| panic!("{args:?} did not parse: {e}"));
            assert_eq!(cli.target.as_deref(), Some("ttbook"), "{args:?}");
            assert_eq!(
                cli.config.as_deref(),
                Some(std::path::Path::new("/c.json")),
                "{args:?}"
            );
            assert!(cli.command.is_none(), "{args:?}");
        }
    }

    #[test]
    fn the_project_target_with_a_subcommand_is_a_usage_conflict() {
        let err = match checked(&["ttbook", "list"]) {
            Err(err) => err,
            Ok(_) => panic!("a target riding along with a subcommand must be refused"),
        };
        assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);
        let msg = err.to_string();
        assert!(
            msg.contains("ttbook"),
            "the refusal must name the target: {msg}"
        );
        assert!(
            msg.contains("path"),
            "the refusal must point at opening by path: {msg}"
        );
    }

    #[test]
    fn the_project_target_accepts_dot() {
        let cli = checked(&["."]).expect("`.` parses");
        assert_eq!(cli.target.as_deref(), Some("."));
        assert!(cli.command.is_none());
    }

    #[test]
    fn the_project_target_is_documented_in_help() {
        let mut cmd = Cli::command();
        let long = cmd.render_long_help().to_string();
        assert!(
            long.contains("Alias or path of a registered project to open directly"),
            "{long}"
        );
        assert!(
            long.contains("must be") && long.contains("`list`") && long.contains("path"),
            "the long help must carry the open-by-path collision note (D-04): {long}"
        );
    }

    // ── The hidden in-sandbox `bridge` (Phase 28.1, D-08) ────────────────

    #[test]
    fn bridge_takes_a_port_a_socket_and_the_driver_command_after_the_separator() {
        let cli = checked(&[
            "bridge",
            "--port",
            "3128",
            "--sock",
            "/run/egress.sock",
            "--",
            "claude",
            "-p",
            "--output-format",
            "stream-json",
        ])
        .expect("parses");
        match cli.command {
            Some(Commands::Bridge {
                port,
                sock,
                command,
            }) => {
                assert_eq!(port, 3128);
                assert_eq!(sock, PathBuf::from("/run/egress.sock"));
                assert_eq!(
                    command,
                    ["claude", "-p", "--output-format", "stream-json"].map(OsString::from)
                );
            }
            _ => panic!("did not resolve to Commands::Bridge"),
        }
    }

    #[test]
    fn bridge_without_a_driver_command_is_a_usage_error() {
        assert!(checked(&["bridge", "--port", "1", "--sock", "/s"]).is_err());
    }

    // ── The hidden `driverctl` client (Phase 27, D-01, D-02) ─────────────

    fn driverctl(args: &[&str]) -> Result<DriverctlAction, clap::Error> {
        let argv: Vec<&str> = ["driverctl"].iter().chain(args.iter()).copied().collect();
        match checked(&argv)?.command {
            Some(Commands::Driverctl { action }) => Ok(action),
            _ => panic!("{argv:?} did not resolve to Commands::Driverctl"),
        }
    }

    fn driverctl_log(args: &[&str]) -> DriverctlLog {
        let mut full = vec!["log"];
        full.extend_from_slice(args);
        match driverctl(&full).unwrap_or_else(|e| panic!("{full:?} did not parse: {e}")) {
            DriverctlAction::Log(log) => *log,
            _ => panic!("{full:?} did not resolve to DriverctlAction::Log"),
        }
    }

    #[test]
    fn driverctl_log_parses_the_master_prompt_shapes() {
        let start = driverctl_log(&["start", "--mode", "headless", "--loop-mode", "loop"]);
        assert_eq!(start.event, "start");
        assert_eq!(start.mode.as_deref(), Some("headless"));
        assert_eq!(start.loop_mode.as_deref(), Some("loop"));
        assert_eq!(start.item, None);

        let dispatched = driverctl_log(&[
            "dispatched",
            "--item",
            "I-01",
            "--step",
            "/gsd-plan-phase 3",
            "--round",
            "0",
            "--phase",
            "3",
        ]);
        assert_eq!(dispatched.event, "dispatched");
        assert_eq!(dispatched.step.as_deref(), Some("/gsd-plan-phase 3"));
        assert_eq!(dispatched.round.as_deref(), Some("0"));
        assert_eq!(dispatched.phase.as_deref(), Some("3"));

        let verified = driverctl_log(&[
            "verified",
            "--item",
            "I-01",
            "--done-when",
            "met",
            "--evidence",
            "tests green",
        ]);
        assert_eq!(verified.done_when.as_deref(), Some("met"));
        assert_eq!(verified.evidence.as_deref(), Some("tests green"));

        let status = driverctl_log(&[
            "status",
            "--item",
            "I-01",
            "--state",
            "parked",
            "--outcome",
            "gaps",
            "--rule",
            "budget",
        ]);
        assert_eq!(status.state.as_deref(), Some("parked"));
        assert_eq!(status.rule.as_deref(), Some("budget"));
    }

    #[test]
    fn driverctl_show_and_hook_parse() {
        assert!(matches!(driverctl(&["show"]), Ok(DriverctlAction::Show)));
        assert!(matches!(driverctl(&["hook"]), Ok(DriverctlAction::Hook)));
    }

    #[test]
    fn driverctl_stop_guard_takes_a_run_dir_and_is_hidden() {
        match driverctl(&["stop-guard", "--run-dir", "/r/d"]).expect("parses") {
            DriverctlAction::StopGuard { run_dir } => {
                assert_eq!(run_dir, Some(PathBuf::from("/r/d")));
            }
            _ => panic!("not a stop-guard"),
        }
        assert!(matches!(
            driverctl(&["stop-guard"]),
            Ok(DriverctlAction::StopGuard { run_dir: None })
        ));
        let mut cmd = Cli::command();
        let help = cmd
            .find_subcommand_mut("driverctl")
            .expect("driverctl exists")
            .render_long_help()
            .to_string();
        assert!(!help.contains("stop-guard"), "hidden: {help}");
    }

    #[test]
    fn driverctl_unknown_flag_is_a_usage_error() {
        let err = match driverctl(&["log", "start", "--bogus", "x"]) {
            Err(err) => err,
            Ok(_) => panic!("an unknown flag must be refused"),
        };
        assert_eq!(err.kind(), clap::error::ErrorKind::UnknownArgument);
        assert_eq!(err.exit_code(), 2, "argparse parity: usage errors exit 2");
    }

    // ── The read-only `next` hint (Phase 29, D-20) ───────────────────────

    #[test]
    fn next_parses_a_project_target_phase_and_json() {
        let cli = checked(&["next", "p", "--target-phase", "3", "--json"]).expect("parses");
        match cli.command {
            Some(Commands::Next {
                project,
                target_phase,
                json,
            }) => {
                assert_eq!(project, "p");
                assert_eq!(target_phase.as_deref(), Some("3"));
                assert!(json);
            }
            _ => panic!("did not resolve to Commands::Next"),
        }
    }

    #[test]
    fn next_without_a_target_phase_leaves_it_unset() {
        let cli = checked(&["next", "p", "--json"]).expect("parses");
        assert!(matches!(
            cli.command,
            Some(Commands::Next {
                target_phase: None,
                ..
            })
        ));
    }

    #[test]
    fn next_without_json_is_a_usage_error() {
        let err = match checked(&["next", "p"]) {
            Err(err) => err,
            Ok(_) => panic!("`next` without --json must be refused"),
        };
        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
        assert_eq!(err.exit_code(), 2, "argparse parity: usage errors exit 2");
    }

    fn driver_action(args: &[&str]) -> Result<DriverAction, clap::Error> {
        let argv: Vec<&str> = ["gsd-meta-manager", "driver"]
            .into_iter()
            .chain(args.iter().copied())
            .collect();
        match Cli::try_parse_from(argv)?.command {
            Some(Commands::Driver { action }) => Ok(action),
            _ => panic!("did not resolve to Commands::Driver"),
        }
    }

    #[test]
    fn driver_launch_detaches_unless_foreground_and_hides_the_daemon_flag() {
        let launch = |extra: &[&str]| {
            let mut args = vec!["launch", "p", "r1", "--prompt-file", "/x/p.md"];
            args.extend_from_slice(extra);
            match driver_action(&args).expect("parses") {
                DriverAction::Launch(args) => *args,
                _ => panic!("not a launch"),
            }
        };
        let detached = launch(&[]);
        assert!(!detached.foreground && !detached.daemon_child);
        assert!(launch(&["--foreground"]).foreground);
        assert!(launch(&["--foreground", "--daemon-child"]).daemon_child);

        let mut driver = Cli::command();
        let help = driver
            .find_subcommand_mut("driver")
            .and_then(|d| d.find_subcommand_mut("launch"))
            .expect("driver launch exists")
            .render_long_help()
            .to_string();
        assert!(help.contains("--foreground"), "{help}");
        assert!(!help.contains("daemon-child"), "{help}");
    }

    #[test]
    fn a_hidden_prompt_override_is_not_in_launch_help() {
        let mut root = Cli::command();
        let launch = root
            .find_subcommand_mut("driver")
            .and_then(|d| d.find_subcommand_mut("launch"))
            .expect("driver launch exists");
        for (form, rendered) in [
            ("render_help", launch.render_help().to_string()),
            ("render_long_help", launch.render_long_help().to_string()),
        ] {
            // POSITIVE CONTROL: a visible launch flag is rendered, so an absent
            // `--prompt-file` is a hidden one and not an empty rendering.
            assert!(rendered.contains("--cap-gap-rounds"), "{form}: {rendered}");
            assert!(
                !rendered.contains("prompt-file"),
                "{form} advertises the test override (D-04):\n{rendered}"
            );
        }
        // Hidden is not removed: the override still parses (the tests and the
        // detached launcher's own command line depend on it).
        assert!(driver_action(&["launch", "p", "r1", "--prompt-file", "/x/p.md"]).is_ok());
        assert!(driver_action(&["launch", "p", "r1"]).is_ok());
    }

    #[test]
    fn the_default_gap_cap_is_two() {
        // D-14: the shipped launcher cap L. The DIRECTION lint warns against it, so
        // a change here must change that rule too.
        let DriverAction::Launch(args) = driver_action(&["launch", "p", "r1"]).expect("parses")
        else {
            panic!("not a launch");
        };
        assert_eq!(args.cap_gap_rounds, DEFAULT_CAP_GAP_ROUNDS);
        assert_eq!(DEFAULT_CAP_GAP_ROUNDS, 2, "D-14: the shipped launcher cap");
        assert_eq!(args.prompt_file, None);
    }

    #[test]
    fn the_launch_form_defaults_are_the_documented_clap_defaults() {
        let defaults = launch_defaults();
        assert_eq!(defaults.model, "opus");
        assert_eq!(defaults.loop_mode, DriverLoopMode::Loop);
        assert_eq!(defaults.max_budget_usd, 10.0);
        assert_eq!(defaults.cap_dispatches, 10);
        assert_eq!(defaults.cap_minutes, 60.0);
        assert_eq!(defaults.cap_gap_rounds, DEFAULT_CAP_GAP_ROUNDS);
        assert!(!defaults.unsandboxed, "a sandboxed launch is the default");
        assert!(!defaults.foreground && !defaults.daemon_child);
        assert_eq!(defaults.prompt_file, None);
    }

    #[test]
    fn launch_field_values_are_judged_by_the_launch_parser() {
        assert_eq!(
            parse_launch_field("--cap-dispatches", "12")
                .expect("12 is a dispatch cap")
                .cap_dispatches,
            12
        );
        assert_eq!(
            parse_launch_field("--cap-minutes", "45.5")
                .expect("45.5 is a minute cap")
                .cap_minutes,
            45.5
        );
        assert_eq!(
            parse_launch_field("--loop-mode", "fresh-per-item")
                .expect("a loop mode")
                .loop_mode,
            DriverLoopMode::FreshPerItem
        );
        for (flag, value) in [
            ("--max-budget-usd", "0"),
            ("--max-budget-usd", "-3"),
            ("--max-budget-usd", "nan"),
            ("--max-budget-usd", "100001"),
            ("--cap-minutes", "nan"),
            ("--cap-minutes", "inf"),
            ("--cap-minutes", "525601"),
            ("--cap-dispatches", "-1"),
            ("--cap-dispatches", "ten"),
            ("--cap-gap-rounds", "1.5"),
            ("--loop-mode", "never"),
        ] {
            let refused = parse_launch_field(flag, value).expect_err(&format!("{flag}={value}"));
            assert!(
                refused.contains(flag.trim_start_matches("--")) || refused.contains("invalid"),
                "clap's own error text for {flag}={value}: {refused}"
            );
        }
        // `--max-budget-usd 0` is clap's error text, not a TUI paraphrase.
        let text = parse_launch_field("--max-budget-usd", "0").unwrap_err();
        assert!(text.contains("the budget must be a finite number greater than 0"));
    }

    #[test]
    fn a_field_value_that_looks_like_a_flag_is_a_value() {
        // Joined as `--flag=value`, so `--model --foreground` sets the model to that
        // text and cannot turn on a different flag.
        let args = parse_launch_field("--model", "--foreground").expect("a model name");
        assert_eq!(args.model, "--foreground");
        assert!(!args.foreground);
    }

    #[test]
    fn driver_auth_takes_setup_or_status_and_nothing_else() {
        assert!(matches!(
            driver_action(&["auth", "setup"]),
            Ok(DriverAction::Auth {
                action: AuthAction::Setup
            })
        ));
        assert!(matches!(
            driver_action(&["auth", "status"]),
            Ok(DriverAction::Auth {
                action: AuthAction::Status
            })
        ));
        assert!(driver_action(&["auth"]).is_err(), "a verb is required");
        assert!(
            driver_action(&["auth", "setup", "--token", "x"]).is_err(),
            "a token is never an argument (argv is world-readable)"
        );
    }

    #[test]
    fn driver_stop_takes_a_project_a_run_id_and_a_wait() {
        match driver_action(&["stop", "p", "r1"]).expect("parses") {
            DriverAction::Stop {
                project,
                run_id,
                wait_secs,
            } => {
                assert_eq!((project.as_str(), run_id.as_str()), ("p", "r1"));
                assert_eq!(wait_secs, 60, "the default wait");
            }
            _ => panic!("not a stop"),
        }
        match driver_action(&["stop", "p", "r1", "--wait-secs", "5"]).expect("parses") {
            DriverAction::Stop { wait_secs, .. } => assert_eq!(wait_secs, 5),
            _ => panic!("not a stop"),
        }
        assert!(
            driver_action(&["stop", "p"]).is_err(),
            "the run id is required"
        );
    }

    #[test]
    fn driver_push_takes_a_project_and_a_run_id() {
        match driver_action(&["push", "p", "r1"]).expect("parses") {
            DriverAction::Push {
                project,
                run_id,
                ack_planted,
            } => {
                assert_eq!((project.as_str(), run_id.as_str()), ("p", "r1"));
                assert!(!ack_planted, "the ack is never implied");
            }
            _ => panic!("not a push"),
        }
        assert!(
            driver_action(&["push", "p"]).is_err(),
            "the run id is required"
        );
    }

    #[test]
    fn driver_push_takes_an_ack_for_planted_files() {
        match driver_action(&["push", "p28", "r28", "--ack-planted"]).expect("parses") {
            DriverAction::Push { ack_planted, .. } => assert!(ack_planted),
            _ => panic!("not a push"),
        }
        match driver_action(&["push", "p28", "r28"]).expect("parses") {
            DriverAction::Push { ack_planted, .. } => assert!(!ack_planted),
            _ => panic!("not a push"),
        }
    }

    #[test]
    fn driver_new_takes_a_project_and_a_slug() {
        match driver_action(&["new", "p", "smoke"]).expect("parses") {
            DriverAction::New { project, slug } => {
                assert_eq!((project.as_str(), slug.as_str()), ("p", "smoke"));
            }
            _ => panic!("not a new"),
        }
        assert!(
            driver_action(&["new", "p"]).is_err(),
            "the slug is required"
        );
        assert!(
            driver_action(&["new", "p", "smoke", "--prompt-file", "x"]).is_err(),
            "a run is minted from the embedded templates only"
        );
    }

    #[test]
    fn driver_check_takes_a_project_and_a_run_id() {
        match driver_action(&["check", "p", "r1"]).expect("parses") {
            DriverAction::Check { project, run_id } => {
                assert_eq!((project.as_str(), run_id.as_str()), ("p", "r1"));
            }
            _ => panic!("not a check"),
        }
        assert!(
            driver_action(&["check", "p"]).is_err(),
            "the run id is required"
        );
    }

    #[test]
    fn task_run_defaults_are_the_launch_defaults() {
        let defaults = task_run_defaults();
        assert_eq!(defaults.max_budget_usd, 10.0);
        assert_eq!(defaults.cap_minutes, 60.0);
        assert_eq!(defaults.model, "opus");
        assert_eq!(defaults.mode, crate::oneoff::Mode::Quick);
        assert!(!defaults.unsandboxed);
    }

    #[test]
    fn task_run_needs_exactly_one_of_todo_and_text() {
        let parse = |args: &[&str]| {
            Cli::try_parse_checked_from(
                ["gsd-meta-manager", "task", "run", "p"]
                    .iter()
                    .chain(args.iter()),
            )
        };
        assert!(parse(&[]).is_err());
        assert!(parse(&["--todo", "a.md", "--text", "x"]).is_err());
        assert!(parse(&["--todo", "a.md"]).is_ok());
        assert!(parse(&["--text", "-x", "--mode", "fast"]).is_ok());
        assert!(parse(&["--text", "x", "--cap-minutes", "0"]).is_err());
    }

    #[test]
    fn task_is_listed_in_help() {
        let rendered = Cli::command().render_help().to_string();
        assert!(rendered
            .lines()
            .any(|line| is_subcommand_entry(line, "task")));
    }

    #[test]
    fn taskctl_is_absent_from_rendered_help() {
        let mut cmd = Cli::command();
        for (form, rendered) in [
            ("render_help", cmd.render_help().to_string()),
            ("render_long_help", cmd.render_long_help().to_string()),
        ] {
            assert!(
                rendered
                    .lines()
                    .any(|line| is_subcommand_entry(line, "list")),
                "{form}: positive control missing, the absence check certifies nothing"
            );
            assert!(
                !rendered
                    .lines()
                    .any(|line| is_subcommand_entry(line, "taskctl")),
                "{form} advertises `taskctl`, which users never type"
            );
        }
    }

    #[test]
    fn taskctl_report_parses_repeatable_flags() {
        let cli = Cli::try_parse_checked_from([
            "gsd-meta-manager",
            "taskctl",
            "report",
            "--status",
            "done",
            "--headline",
            "-starts with a dash",
            "--changed",
            "a",
            "--changed",
            "b",
            "--verified",
            "cargo test: ok",
        ])
        .unwrap();
        let Some(Commands::Taskctl {
            action: TaskctlAction::Report(report),
        }) = cli.command
        else {
            panic!("not taskctl report");
        };
        assert_eq!(report.status, crate::oneoff::report::ReportStatus::Done);
        assert_eq!(report.headline, "-starts with a dash");
        assert_eq!(report.changed, ["a", "b"]);
        assert!(report.followup.is_empty());
    }

    #[test]
    fn driverctl_is_absent_from_rendered_help() {
        let mut cmd = Cli::command();
        for (form, rendered) in [
            ("render_help", cmd.render_help().to_string()),
            ("render_long_help", cmd.render_long_help().to_string()),
        ] {
            assert!(
                rendered
                    .lines()
                    .any(|line| is_subcommand_entry(line, "list")),
                "{form}: positive control missing, the absence check certifies nothing"
            );
            assert!(
                !rendered
                    .lines()
                    .any(|line| is_subcommand_entry(line, "driverctl")),
                "{form} advertises `driverctl`, which users never type (D-02)"
            );
        }
    }
}
