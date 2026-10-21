//! Starting, launching and relaunching launcher runs from the Driver tab (Phase 30,
//! plan 30-05, D-18 to D-21): the launch form and its pure helpers.
//!
//! **A key handler here never touches the filesystem, never spawns a process and never
//! calls the launcher** (D-25). The form collects a [`LaunchArgs`], dispatches
//! [`Action::DriverLaunchRequested`] with it as plain data, and the App builds the
//! [`LaunchOptions`] and calls the launcher library on `spawn_blocking`. The launch is
//! the library's *detached* path: the daemon it starts owns the run lock and the
//! driver socket, so the run outlives this process (D-19).
//!
//! **The defaults and every edited value come from clap's own parser**
//! ([`crate::cli::launch_defaults`], [`crate::cli::parse_launch_field`]): the form
//! accepts exactly what `driver launch` accepts and carries no copy of its bounds.
//!
//! **Nothing here claims a hard per-run spend ceiling.** `--max-budget-usd` is passed
//! to each `claude` session the launcher starts, so the form says "each driver
//! session" and, in `fresh-per-item` mode, that a run can spend a multiple of it.
//!
//! Every value drawn or interpolated that came from a file or a registry goes through
//! [`crate::text::render_for_terminal`].

use super::confirm_popup::{
    body_room, click_as_key, dim_background, fit_list, render_confirm_popup, wrapped_rows,
    ConfirmOutcome, ConfirmPopup,
};
use super::driver_confirm::dispatch;
use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::cli::{launch_defaults, parse_launch_field, DriverLoopMode, LaunchArgs};
use crate::launcher::LauncherError;
use crate::state_reader::driver_frontmatter::{DerivedStatus, DriverFrontmatter, RunStatus};
use crate::ui::mouse::MouseInput;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;
#[cfg(target_os = "linux")]
use std::path::Path;
use std::sync::LazyLock;

fn bold() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

fn muted() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn shown(text: &str) -> String {
    crate::text::render_for_terminal(text).to_string()
}

// ── Which launch a run offers (D-21) ────────────────────────────────────────

/// What `R` does for a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchKind {
    /// A run directory that was never launched: the CLI defaults, then launch.
    Launch,
    /// A run that ended or crashed: prefilled from what it recorded, then relaunch
    /// the SAME run id.
    Relaunch,
}

/// Why `R` opens no form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchRefusal {
    /// The run lock is held: the run is running.
    LockHeld,
    /// The lock probe could not tell.
    LivenessUnknown,
    /// The run is in a state a relaunch does not apply to (`done`, an unknown word);
    /// carries the word.
    NothingToRelaunch(String),
}

/// The pure decision `R` makes from a run's derived status. Liveness is the lock's
/// (`DerivedStatus`), never the frontmatter alone (D-09).
pub fn launch_kind_for(derived: &DerivedStatus) -> Result<LaunchKind, LaunchRefusal> {
    match derived {
        DerivedStatus::NotLaunched => Ok(LaunchKind::Launch),
        DerivedStatus::Crashed { .. } => Ok(LaunchKind::Relaunch),
        DerivedStatus::LivenessUnknown { .. } => Err(LaunchRefusal::LivenessUnknown),
        DerivedStatus::AsWritten(status) => match status {
            // The reader returns these two only while the run lock is held.
            RunStatus::Running | RunStatus::Starting => Err(LaunchRefusal::LockHeld),
            RunStatus::Parked
            | RunStatus::Stopped
            | RunStatus::Stalled
            | RunStatus::Paused
            | RunStatus::Crashed => Ok(LaunchKind::Relaunch),
            RunStatus::Done => Err(LaunchRefusal::NothingToRelaunch("done".to_string())),
            RunStatus::Unknown(word) => Err(LaunchRefusal::NothingToRelaunch(word.clone())),
        },
    }
}

/// The status line for a refused `R`.
pub fn refusal_copy(run_id: &str, refusal: &LaunchRefusal) -> String {
    let id = super::driver::shown_capped(run_id);
    match refusal {
        LaunchRefusal::LockHeld => {
            format!("{id} is still running (its run lock is held). Stop it first.")
        }
        LaunchRefusal::LivenessUnknown => format!(
            "Cannot tell whether {id} is running (the lock probe failed), so it was not relaunched."
        ),
        LaunchRefusal::NothingToRelaunch(word) => format!(
            "{id} is {}: there is nothing to relaunch. Start a new run with [s].",
            super::driver::shown_capped(word)
        ),
    }
}

// ── The form's values (D-19, D-21) ──────────────────────────────────────────

/// The six editable values, in the order the form shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Model,
    LoopMode,
    Budget,
    Dispatches,
    Minutes,
    GapRounds,
}

impl Field {
    pub const ALL: [Field; 6] = [
        Field::Model,
        Field::LoopMode,
        Field::Budget,
        Field::Dispatches,
        Field::Minutes,
        Field::GapRounds,
    ];

    fn label(self) -> &'static str {
        match self {
            Field::Model => "model",
            Field::LoopMode => "loop mode",
            Field::Budget => "spend ceiling",
            Field::Dispatches => "dispatches",
            Field::Minutes => "minutes",
            Field::GapRounds => "gap rounds",
        }
    }

    /// The `driver launch` flag the field is validated through.
    fn flag(self) -> &'static str {
        match self {
            Field::Model => "--model",
            Field::LoopMode => "--loop-mode",
            Field::Budget => "--max-budget-usd",
            Field::Dispatches => "--cap-dispatches",
            Field::Minutes => "--cap-minutes",
            Field::GapRounds => "--cap-gap-rounds",
        }
    }
}

/// Which values are the CLI defaults because the run recorded nothing for them
/// (shown ` (default)`, D-21).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LaunchDefaulted {
    pub model: bool,
    pub loop_mode: bool,
    pub budget: bool,
    pub dispatches: bool,
    pub minutes: bool,
    pub gap_rounds: bool,
}

impl LaunchDefaulted {
    fn get(&self, field: Field) -> bool {
        match field {
            Field::Model => self.model,
            Field::LoopMode => self.loop_mode,
            Field::Budget => self.budget,
            Field::Dispatches => self.dispatches,
            Field::Minutes => self.minutes,
            Field::GapRounds => self.gap_rounds,
        }
    }

    fn clear(&mut self, field: Field) {
        match field {
            Field::Model => self.model = false,
            Field::LoopMode => self.loop_mode = false,
            Field::Budget => self.budget = false,
            Field::Dispatches => self.dispatches = false,
            Field::Minutes => self.minutes = false,
            Field::GapRounds => self.gap_rounds = false,
        }
    }
}

/// What the launch form holds: the `driver launch` arguments it will confirm, which of
/// them are defaults, and whether the recorded wall-clock limit is already used up.
#[derive(Debug, Clone)]
pub struct LaunchForm {
    /// The arguments the confirmation launches with. `unsandboxed` is always `false`
    /// here: the unsandboxed launch is a separate confirmation (D-20).
    pub args: LaunchArgs,
    pub defaulted: LaunchDefaulted,
    /// The run recorded a wall-clock limit that its used minutes already reach, so the
    /// launcher would refuse it as `CapExhausted` until `minutes` is raised.
    pub minutes_used_up: bool,
    /// Active wall-clock minutes the run has used (0 when nothing is recorded).
    pub minutes_used: f64,
}

impl LaunchForm {
    /// The CLI defaults, for a run that was never launched. Nothing is "recorded", so
    /// nothing is flagged ` (default)`.
    pub fn fresh() -> Self {
        Self {
            args: launch_defaults(),
            defaulted: LaunchDefaulted::default(),
            minutes_used_up: false,
            minutes_used: 0.0,
        }
    }
}

/// A model name the form accepts, typed or recorded: the conservative rule
/// `^[A-Za-z0-9][A-Za-z0-9._:\[\]-]{0,63}$` (T-30-17, T-30-30). A recorded connection is not
/// trusted as an argument the user did not type, and a typed value that looks like a
/// flag (`-x`, `--foreground`) never reaches the detached launch's argv. The brackets
/// admit Claude Code's `[1m]` long-context suffix (WR-07).
fn model_name_ok(model: &str) -> bool {
    let mut chars = model.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && model.chars().count() <= 64
        && chars
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-' | '[' | ']'))
}

/// The form for relaunching the run whose frontmatter is `fm` (D-21).
///
/// The caps come from `caps.*.limit` and the model from the driver connection. Each
/// recorded number is re-judged by clap's own parser, so a forged `limit: 1e308` or a
/// negative one falls back to the default instead of reaching the form. The spend
/// ceiling and the loop mode are not recorded on the run (the connection's `mode` is
/// `headless`, not a loop mode), so they show the defaults.
pub fn relaunch_prefill(fm: &DriverFrontmatter) -> LaunchForm {
    let mut form = LaunchForm {
        args: launch_defaults(),
        defaulted: LaunchDefaulted {
            model: true,
            loop_mode: true,
            budget: true,
            dispatches: true,
            minutes: true,
            gap_rounds: true,
        },
        minutes_used_up: false,
        minutes_used: 0.0,
    };
    if let Some(model) = fm
        .connected
        .iter()
        .find(|c| c.role == "driver")
        .and_then(|c| c.model.as_deref())
        .filter(|m| model_name_ok(m))
    {
        form.args.model = model.to_string();
        form.defaulted.model = false;
    }
    if let Some(limit) = fm.caps.dispatches.limit {
        if let Ok(parsed) = parse_launch_field("--cap-dispatches", &limit.to_string()) {
            form.args.cap_dispatches = parsed.cap_dispatches;
            form.defaulted.dispatches = false;
        }
    }
    if let Some(limit) = fm.caps.wall_clock_min.limit {
        if let Ok(parsed) = parse_launch_field("--cap-minutes", &limit.to_string()) {
            form.args.cap_minutes = parsed.cap_minutes;
            form.defaulted.minutes = false;
        }
    }
    if let Some(limit) = fm.caps.gap_rounds.limit {
        if let Ok(parsed) = parse_launch_field("--cap-gap-rounds", &limit.to_string()) {
            form.args.cap_gap_rounds = parsed.cap_gap_rounds;
            form.defaulted.gap_rounds = false;
        }
    }
    let used = fm.caps.wall_clock_min.used;
    form.minutes_used = if used.is_finite() && used > 0.0 {
        used
    } else {
        0.0
    };
    form.minutes_used_up = !form.defaulted.minutes && form.minutes_used >= form.args.cap_minutes;
    form
}

// ── What the launcher is given (D-19) ───────────────────────────────────────

/// The [`LaunchOptions`] `driver launch` builds from `args` (`src/driver_cli.rs`), so
/// the tab launches exactly what the CLI would: the registry alias and project path,
/// all three caps, `open_pr` from the preferences, the PR policy from the project's
/// opt-in, and no prompt override.
///
/// [`LaunchOptions`]: crate::launcher::run::LaunchOptions
#[cfg(target_os = "linux")]
pub fn launch_options_for(
    alias: &str,
    project: &Path,
    args: &LaunchArgs,
    open_pr: bool,
    opt_in: Option<&crate::config::DriverOptIn>,
) -> crate::launcher::run::LaunchOptions {
    use crate::driverctl::store::CapLimits;
    use crate::launcher::push::policy_for;
    use crate::launcher::run::{LaunchOptions, LoopMode};

    LaunchOptions {
        project: project.to_path_buf(),
        alias: alias.to_string(),
        run_id: args.run_id.clone(),
        prompt_file: None,
        model: args.model.clone(),
        loop_mode: match args.loop_mode {
            DriverLoopMode::Loop => LoopMode::Loop,
            DriverLoopMode::FreshPerItem => LoopMode::FreshPerItem,
        },
        max_budget_usd: args.max_budget_usd,
        limits: CapLimits {
            dispatches: Some(args.cap_dispatches),
            wall_clock_min: Some(args.cap_minutes),
            gap_rounds: Some(args.cap_gap_rounds),
        },
        unsandboxed: args.unsandboxed,
        open_pr,
        pr_policy: policy_for(alias, opt_in),
    }
}

// ── The preflight (D-20) ────────────────────────────────────────────────────

/// What the launch form needs to know before `y` can do anything, computed off the
/// render thread. Plain data, so it travels as an [`Action`] payload.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchPreflight {
    /// The sandbox pieces the host lacks (`bwrap`, `systemd-run`, `user-systemd`).
    pub missing: Vec<String>,
    /// Why the host's `claude` is refused in both postures, when it is.
    pub claude_refusal: Option<String>,
    /// The `control` word the launcher would read: the first `- control:` under
    /// `## Direction` (`driver_templates::lint::control_line`), as written.
    pub control: Option<String>,
    /// `driver check` problems, rendered `DIRECTION.md:<line>: <message>`, the line
    /// counted from the first body line (the numbering the editor shows).
    pub lint_problems: Vec<String>,
    /// `driver check` warnings, numbered the same way; they do not block a launch.
    pub lint_warnings: Vec<String>,
}

/// Run the host check's decision and the `driver check` lint for `run_dir`.
///
/// **Blocking: `spawn_blocking` only.** It reads `DIRECTION.md` (under the edit flow's
/// 1 MiB cap); the host probes themselves ran in `caps`, which the caller got from the
/// injectable `detect` seam.
#[cfg(target_os = "linux")]
pub fn launch_preflight(run_dir: &Path, caps: &crate::launcher::host::HostCaps) -> LaunchPreflight {
    use crate::driver::direction::split_frontmatter;
    use crate::driver_templates::lint::{control_line, lint_direction};

    let mut pre = LaunchPreflight {
        missing: caps.missing.clone(),
        ..LaunchPreflight::default()
    };
    match crate::launcher::host::decide(caps, false) {
        Ok(_) => pre.missing.clear(),
        Err(LauncherError::SandboxUnavailable { missing }) => pre.missing = missing,
        Err(LauncherError::ClaudeVersion { detail }) => pre.claude_refusal = Some(detail),
        Err(other) => pre.claude_refusal = Some(other.to_string()),
    }
    match crate::driver::steer::read_direction(run_dir) {
        Ok(text) => {
            // The edit flow shows the user the body alone, so findings are numbered from
            // its first line: the line the editor shows (WR-04). The control word is the
            // launcher's own reading of the file (IN-03).
            let report = lint_direction(split_frontmatter(&text).1);
            pre.lint_problems = report.problems.iter().map(ToString::to_string).collect();
            pre.lint_warnings = report.warnings.iter().map(|f| f.as_warning()).collect();
            pre.control = control_line(&text).map(|(_, word)| word);
        }
        Err(error) => {
            pre.lint_problems = vec![format!("DIRECTION.md could not be read: {error}")];
        }
    }
    pre
}

// ── Launch failures (T-30-20) ───────────────────────────────────────────────

/// At most this many lines, and this many characters, of a launcher detail reach the
/// status line.
const DETAIL_MAX_LINES: usize = 3;
const DETAIL_MAX_CHARS: usize = 300;

/// `text` cut to [`DETAIL_MAX_LINES`] lines of [`DETAIL_MAX_CHARS`] characters in all,
/// each line escaped, joined with ` / `.
fn capped_detail(text: &str) -> String {
    let joined = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(DETAIL_MAX_LINES)
        .map(shown)
        .collect::<Vec<_>>()
        .join(" / ");
    if joined.chars().count() <= DETAIL_MAX_CHARS {
        return joined;
    }
    let mut cut: String = joined.chars().take(DETAIL_MAX_CHARS - 1).collect();
    cut.push('\u{2026}');
    cut
}

/// The refusal for a host that cannot sandbox the driver (D-20).
pub fn sandbox_refused_copy(missing: &[String]) -> String {
    format!(
        "Launch refused: this host cannot sandbox the driver (missing: {}). Press [U] to launch it unsandboxed instead.",
        missing.iter().map(|m| shown(m)).collect::<Vec<_>>().join(", ")
    )
}

/// The status or error line for a launch that did not start a run. Every variant says
/// that nothing was started (or that the launch was refused), and every value in it is
/// escaped and capped.
pub fn launch_error_copy(run_id: &str, error: &LauncherError) -> String {
    let id = super::driver::shown_capped(run_id);
    let general = |error: &LauncherError| {
        let text = capped_detail(&error.to_string());
        format!(
            "Could not launch {id}: {}. Nothing was started.",
            text.trim_end_matches('.')
        )
    };
    match error {
        LauncherError::Locked { .. } | LauncherError::DaemonExited { code: 3, .. } => {
            format!("Could not launch {id}: it is already launched. Nothing was started.")
        }
        LauncherError::DirectionInvalid { problems } => {
            let mut copy = format!(
                "Could not launch {id}: DIRECTION.md failed driver check ({} problem(s)). Nothing was started.",
                problems.len()
            );
            if let Some(first) = problems.first() {
                copy.push_str(&format!(" First problem: {}", capped_detail(first)));
            }
            copy
        }
        LauncherError::CapExhausted { detail } => format!(
            "Could not launch {id}: {}. Raise minutes in the launch form. Nothing was started.",
            capped_detail(detail).trim_end_matches('.')
        ),
        LauncherError::SandboxUnavailable { missing } => sandbox_refused_copy(missing),
        LauncherError::ClaudeVersion { detail } => {
            format!("Launch refused: {}", capped_detail(detail))
        }
        other => general(other),
    }
}

/// The control word that stops a launch, when the live body holds one: the launcher
/// refuses to start a run whose DIRECTION.md says anything but `control: run`, comparing
/// the word exactly (`launcher::run::preflight_direction`), so the form does too.
fn control_blocking(preflight: &LaunchPreflight) -> Option<&str> {
    preflight.control.as_deref().filter(|word| *word != "run")
}

/// The Yellow control row, and the refusal `y` gives with it.
fn control_copy(word: &str) -> String {
    let word = shown(word);
    format!(
        "control is {word}: the launcher refuses to start a run whose DIRECTION.md says control: {word}. Set control: run with [c] first."
    )
}

/// The Red lint headline, and the refusal `y` gives with it. `[e]` is the Driver tab's
/// edit key: inside the form `e` edits a field, hence "close this form" (the spec's
/// string, which lacked it, was `[inferred — audit]`).
fn lint_copy(count: usize) -> String {
    format!(
        "driver check found {count} problem(s). Close this form, then fix them with [e] before launching."
    )
}

/// The most rows the form gives its lint problems and warnings, the `… k more` count
/// row included (the old 3 + 3 + one count row). `[inferred — audit]`
const FORM_FINDINGS_MAX_ROWS: u16 = 7;

// ── The launch form (D-19, D-20, D-21) ──────────────────────────────────────

/// What the sandbox row of the form says before the preflight arrives.
const SANDBOX_LOADING: &str = "Sandbox: Loading...";
/// And once it passed.
const SANDBOX_OK: &str = "Sandbox: bwrap + systemd scope (host check passed)";

/// The per-field edit hint and the key row.
const KEY_ROW: &str = "[j/k] field  [e] edit field";

/// `$10.00`: the spend ceiling as the form prints it.
fn dollars(amount: f64) -> String {
    format!("${amount:.2}")
}

/// A number as the user would type it: `60`, not `60.0`; `2.5` stays `2.5`.
fn plain_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{value:.0}")
    } else {
        format!("{value}")
    }
}

/// The launch form: `Launch driver run`, a Yellow `ConfirmPopup` whose default focus
/// is No. `y` launches nothing until the preflight for this very run has arrived.
pub struct LaunchFormScreen {
    alias: String,
    run_id: String,
    kind: LaunchKind,
    form: LaunchForm,
    /// Which of the six fields the cursor is on.
    cursor: usize,
    /// The text being typed into the cursor's field, when editing.
    editing: Option<String>,
    /// The edit's refusal (`must be a positive number`), shown Red while editing.
    error: Option<String>,
    /// Why `y` launched nothing (the refusal copy), shown Red until the next key.
    refusal: Option<String>,
    popup: ConfirmPopup,
}

impl LaunchFormScreen {
    pub fn new(alias: String, run_id: String, kind: LaunchKind, mut form: LaunchForm) -> Self {
        // The real project and run travel with the arguments (the launcher takes them
        // from the options, built from these); the placeholders clap parsed are gone.
        form.args.project = alias.clone();
        form.args.run_id = run_id.clone();
        form.args.unsandboxed = false;
        Self {
            alias,
            run_id,
            kind,
            form,
            cursor: 0,
            editing: None,
            error: None,
            refusal: None,
            popup: ConfirmPopup::default(),
        }
    }

    /// The form's current arguments (for tests and the confirmation).
    pub fn args(&self) -> &LaunchArgs {
        &self.form.args
    }

    pub fn kind(&self) -> LaunchKind {
        self.kind
    }

    fn field(&self) -> Field {
        Field::ALL[self.cursor.min(Field::ALL.len() - 1)]
    }

    /// This run's preflight, once it has arrived.
    fn preflight<'a>(&self, ctx: &'a AppContext) -> Option<&'a LaunchPreflight> {
        ctx.view_cache
            .get(&self.alias)
            .and_then(|cache| cache.launch_preflight.as_ref())
            .filter(|(run_id, _)| *run_id == self.run_id)
            .map(|(_, preflight)| preflight)
    }

    /// The value a field shows, without the cursor or the `(default)` note.
    fn value_text(&self, field: Field) -> String {
        let args = &self.form.args;
        match field {
            Field::Model => shown(&args.model),
            Field::LoopMode => match args.loop_mode {
                DriverLoopMode::Loop => "loop".to_string(),
                DriverLoopMode::FreshPerItem => "fresh-per-item".to_string(),
            },
            Field::Budget => dollars(args.max_budget_usd),
            Field::Dispatches => args.cap_dispatches.to_string(),
            Field::Minutes => plain_number(args.cap_minutes),
            Field::GapRounds => format!("{} per phase", args.cap_gap_rounds),
        }
    }

    /// The text an edit of `field` starts from: what the field holds, as typed.
    fn edit_seed(&self, field: Field) -> String {
        let args = &self.form.args;
        match field {
            Field::Model => args.model.clone(),
            Field::LoopMode => String::new(),
            Field::Budget => plain_number(args.max_budget_usd),
            Field::Dispatches => args.cap_dispatches.to_string(),
            Field::Minutes => plain_number(args.cap_minutes),
            Field::GapRounds => args.cap_gap_rounds.to_string(),
        }
    }

    /// Validate `text` for `field` through clap's parser and store it. The refusal is
    /// the copy to show.
    fn commit_edit(&mut self, field: Field, text: &str) -> Result<(), &'static str> {
        let text = text.trim();
        if field == Field::Model {
            // The 64-character bound replaces the old 100 `[inferred — audit]`; clap's own
            // parser still has the last word, as for every other field.
            if !model_name_ok(text) || parse_launch_field(field.flag(), text).is_err() {
                return Err("must be a model name");
            }
            self.form.args.model = text.to_string();
        } else {
            let parsed =
                parse_launch_field(field.flag(), text).map_err(|_| "must be a positive number")?;
            match field {
                Field::Budget => self.form.args.max_budget_usd = parsed.max_budget_usd,
                Field::Dispatches => self.form.args.cap_dispatches = parsed.cap_dispatches,
                Field::Minutes => {
                    self.form.args.cap_minutes = parsed.cap_minutes;
                    self.form.minutes_used_up = self.form.minutes_used > 0.0
                        && self.form.minutes_used >= parsed.cap_minutes;
                }
                Field::GapRounds => self.form.args.cap_gap_rounds = parsed.cap_gap_rounds,
                Field::Model | Field::LoopMode => {}
            }
        }
        self.form.defaulted.clear(field);
        Ok(())
    }

    /// Why a launch of ANY posture must go no further: the host's `claude` is refused
    /// (both postures refuse it), the live body says `control: pause|stop`, or
    /// `driver check` found problems. These block the unsandboxed launch too.
    fn body_blocker(preflight: &LaunchPreflight) -> Option<String> {
        if let Some(detail) = &preflight.claude_refusal {
            return Some(format!("Launch refused: {}", shown(detail)));
        }
        if let Some(word) = control_blocking(preflight) {
            return Some(control_copy(word));
        }
        if !preflight.lint_problems.is_empty() {
            return Some(lint_copy(preflight.lint_problems.len()));
        }
        None
    }

    /// Why `y` must launch nothing right now, as the copy to show; `None` when the
    /// launch may go ahead. A host that cannot sandbox refuses a sandboxed launch with
    /// the sandbox copy; the unsandboxed launch is a separate confirmation (`U`).
    fn blocker(preflight: &LaunchPreflight) -> Option<String> {
        if let Some(detail) = &preflight.claude_refusal {
            return Some(format!("Launch refused: {}", shown(detail)));
        }
        if !preflight.missing.is_empty() {
            return Some(sandbox_refused_copy(&preflight.missing));
        }
        Self::body_blocker(preflight)
    }

    /// `U`: offer the unsandboxed launch, through its own confirmation (D-20). Only a
    /// host that is missing sandbox pieces offers it, and only when nothing else would
    /// refuse the launch anyway; every other case is inert or says why.
    fn offer_unsandboxed(&mut self, ctx: &mut AppContext) -> ScreenAction {
        let Some(preflight) = self.preflight(ctx).cloned() else {
            return ScreenAction::None;
        };
        if preflight.claude_refusal.is_some() || preflight.missing.is_empty() {
            return ScreenAction::None;
        }
        if let Some(copy) = Self::body_blocker(&preflight) {
            self.refusal = Some(copy);
            ctx.needs_redraw = true;
            return ScreenAction::None;
        }
        let mut args = self.form.args.clone();
        args.project = self.alias.clone();
        args.run_id = self.run_id.clone();
        ctx.needs_redraw = true;
        // Replace, not Push: the form that handed off is gone, so neither answer can
        // return the user to a screen they can submit again.
        ScreenAction::Replace(Box::new(UnsandboxedConfirmScreen::new(
            self.alias.clone(),
            self.run_id.clone(),
            args,
            preflight.missing,
        )))
    }

    /// `c`: close the form and open the control chooser for this run, when the live
    /// body says `pause` or `stop`. Nothing is written automatically.
    fn open_control_chooser(&mut self, ctx: &mut AppContext) -> ScreenAction {
        let Some(preflight) = self.preflight(ctx) else {
            return ScreenAction::None;
        };
        let Some(word) = control_blocking(preflight).map(str::to_string) else {
            return ScreenAction::None;
        };
        ctx.needs_redraw = true;
        ScreenAction::Replace(Box::new(super::driver_steer::ControlChooserScreen::new(
            self.alias.clone(),
            self.run_id.clone(),
            Some(word),
        )))
    }

    /// `y` (or Enter on Yes, or a click on Yes).
    fn confirm(&mut self, ctx: &mut AppContext) -> ScreenAction {
        let Some(preflight) = self.preflight(ctx).cloned() else {
            // The host check has not answered: `y` is inert (UI loading).
            return ScreenAction::None;
        };
        if let Some(copy) = Self::blocker(&preflight) {
            self.refusal = Some(copy);
            ctx.needs_redraw = true;
            return ScreenAction::None;
        }
        let mut args = self.form.args.clone();
        args.unsandboxed = false;
        args.project = self.alias.clone();
        args.run_id = self.run_id.clone();
        dispatch(
            ctx,
            Action::DriverLaunchRequested {
                alias: self.alias.clone(),
                run_id: self.run_id.clone(),
                args,
            },
            &self.alias,
        );
        ScreenAction::Pop
    }

    fn handle_edit_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        ctx.needs_redraw = true;
        let field = self.field();
        match code {
            KeyCode::Esc => {
                self.editing = None;
                self.error = None;
            }
            KeyCode::Enter => {
                let text = self.editing.clone().unwrap_or_default();
                match self.commit_edit(field, &text) {
                    Ok(()) => {
                        self.editing = None;
                        self.error = None;
                    }
                    Err(copy) => self.error = Some(copy.to_string()),
                }
            }
            KeyCode::Backspace => {
                if let Some(buffer) = &mut self.editing {
                    buffer.pop();
                }
                self.error = None;
            }
            KeyCode::Char(c) => {
                if let Some(buffer) = &mut self.editing {
                    if buffer.chars().count() < 100 {
                        buffer.push(c);
                    }
                }
                self.error = None;
            }
            _ => {}
        }
        ScreenAction::None
    }

    /// The rows the confirmation cannot be given without: the title, what a launch may
    /// spend and that it keeps running, how it is sandboxed (or why it is refused), the
    /// blockers, the six fields and the minutes hint. WR-05: these are what the user must
    /// read before `y`, so they come first and are never the rows that give way.
    fn essential_lines(&self, ctx: &AppContext) -> Vec<Line<'static>> {
        let yellow = Style::default().fg(Color::Yellow);
        let red = Style::default().fg(Color::Red);
        let mut lines: Vec<Line<'static>> = vec![Line::from(format!(
            "Launch {} on \"{}\"?",
            shown(&self.run_id),
            shown(&self.alias)
        ))];
        lines.push(Line::from(Span::styled(
            format!(
                "Each driver session may spend up to {} of Claude usage (claude --max-budget-usd). It runs detached and keeps running after you close this TUI.",
                dollars(self.form.args.max_budget_usd)
            ),
            yellow,
        )));
        if self.form.args.loop_mode == DriverLoopMode::FreshPerItem {
            lines.push(Line::from(Span::styled(
                "fresh-per-item starts one session per item, so the run can spend a multiple of this.",
                yellow,
            )));
        }
        match self.preflight(ctx) {
            None => lines.push(Line::from(Span::styled(SANDBOX_LOADING, muted()))),
            Some(preflight) => {
                if let Some(detail) = &preflight.claude_refusal {
                    lines.push(Line::from(Span::styled(
                        format!("Launch refused: {}", shown(detail)),
                        red,
                    )));
                } else if !preflight.missing.is_empty() {
                    let items = preflight
                        .missing
                        .iter()
                        .map(|m| shown(m))
                        .collect::<Vec<_>>()
                        .join(", ");
                    lines.push(Line::from(Span::styled(
                        format!("Sandbox unavailable: missing {items}."),
                        red,
                    )));
                    lines.push(Line::from(Span::styled(
                        "A sandboxed launch is refused on this host.",
                        red,
                    )));
                    lines.push(Line::from(Span::styled(
                        "[U] launch UNSANDBOXED instead (asks again)",
                        yellow,
                    )));
                } else {
                    lines.push(Line::from(SANDBOX_OK));
                }
                if let Some(word) = control_blocking(preflight) {
                    lines.push(Line::from(Span::styled(control_copy(word), yellow)));
                }
                if !preflight.lint_problems.is_empty() {
                    lines.push(Line::from(Span::styled(
                        lint_copy(preflight.lint_problems.len()),
                        red,
                    )));
                }
            }
        }
        if let Some(refusal) = &self.refusal {
            // A refusal whose text is already a row above (the claude refusal, the
            // control row, the lint headline) is not said twice.
            let said = lines.iter().any(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
                    == *refusal
            });
            if !said {
                lines.push(Line::from(Span::styled(refusal.clone(), red)));
            }
        }
        let selection = Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD);
        for (index, field) in Field::ALL.into_iter().enumerate() {
            let on_cursor = index == self.cursor;
            let label = format!("{:<16}", field.label());
            let marker = if on_cursor { "> " } else { "  " };
            let marker_style = if on_cursor {
                selection
            } else {
                Style::default()
            };
            let mut spans = vec![
                Span::styled(marker, marker_style),
                Span::styled(
                    label,
                    if on_cursor {
                        selection
                    } else {
                        Style::default()
                    },
                ),
            ];
            match (&self.editing, on_cursor) {
                (Some(buffer), true) => {
                    spans.push(Span::styled(format!("{}_", shown(buffer)), bold()));
                    if let Some(error) = &self.error {
                        spans.push(Span::styled(format!("  {error}"), red));
                    }
                }
                _ => {
                    spans.push(Span::raw(self.value_text(field)));
                    if self.form.defaulted.get(field) {
                        spans.push(Span::styled(" (default)", muted()));
                    }
                }
            }
            lines.push(Line::from(spans));
        }
        if self.form.minutes_used_up {
            lines.push(Line::from(Span::styled(
                "  minutes: the old limit is used up. Raise it or the launcher refuses this launch.",
                yellow,
            )));
        }
        lines
    }

    /// The `driver check` findings, one line each: the problems (Red), then the warnings
    /// (Yellow). They are the rows that give way when the terminal is short.
    fn finding_lines(&self, ctx: &AppContext) -> Vec<Line<'static>> {
        let Some(preflight) = self.preflight(ctx) else {
            return Vec::new();
        };
        let red = Style::default().fg(Color::Red);
        let yellow = Style::default().fg(Color::Yellow);
        let problems = preflight
            .lint_problems
            .iter()
            .map(|problem| Line::from(Span::styled(format!("  {}", shown(problem)), red)));
        let warnings = preflight
            .lint_warnings
            .iter()
            .map(|warning| Line::from(Span::styled(format!("  {}", shown(warning)), yellow)));
        problems.chain(warnings).collect()
    }

    /// Every body line, unfitted: the essentials, all the findings, the key row. Tests
    /// read this; [`Self::fitted_body_lines`] is what is drawn.
    #[cfg(test)]
    fn body_lines(&self, ctx: &AppContext) -> Vec<Line<'static>> {
        let mut lines = self.essential_lines(ctx);
        lines.extend(self.finding_lines(ctx));
        lines.push(Line::from(Span::styled(KEY_ROW, muted())));
        lines
    }

    /// The body as drawn in `area` (WR-05, UI D5): the essentials in full, then as many
    /// findings as the rows left hold (at most [`FORM_FINDINGS_MAX_ROWS`]) with a
    /// `… {k} more` row counting the rest, then the key row when a row is still free. The
    /// findings always give way first. When the essentials alone exceed the room the
    /// popup is clipped, and the popup guard keeps `y` inert.
    fn fitted_body_lines(&self, ctx: &AppContext, area: Rect) -> Vec<Line<'static>> {
        let (inner, room) = body_room(area);
        let mut lines = self.essential_lines(ctx);
        let left = room.saturating_sub(wrapped_rows(&lines, inner));
        // No row left: no findings and no count row either (it would not fit, and a row
        // too many would clip a confirmation whose essentials are all on screen).
        if left > 0 {
            lines.extend(fit_list(
                self.finding_lines(ctx),
                left.min(FORM_FINDINGS_MAX_ROWS),
                inner,
            ));
        }
        let key_row = Line::from(Span::styled(KEY_ROW, muted()));
        if room > wrapped_rows(&lines, inner) {
            lines.push(key_row);
        }
        lines
    }
}

crate::ui::screens::adjudicate_screen!(
    LaunchFormScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints the Driver tab behind a dimmed popup (the registry alias and the run list) \
     and draws the run id (a directory name a cloned repository or a sibling sandbox \
     chooses), the registry alias, the model (recorded in the run's frontmatter, or \
     typed), the host check's missing items and claude refusal (probe output) and the \
     `driver check` findings of DIRECTION.md (quoting its text). Every one is drawn \
     through `render_for_terminal`. Fixture states: the hostile identity as the run id, \
     the alias, the model, a missing item and a lint finding.",
);

impl Screen for LaunchFormScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        // While a field is being typed, every key is text or an edit key: `y` and `n`
        // are letters here, never a confirmation (the Defaults-tab input rule).
        if self.editing.is_some() {
            return self.handle_edit_key(code, ctx);
        }
        self.refusal = None;
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.cursor = (self.cursor + 1).min(Field::ALL.len() - 1);
                ctx.needs_redraw = true;
                return ScreenAction::None;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.cursor = self.cursor.saturating_sub(1);
                ctx.needs_redraw = true;
                return ScreenAction::None;
            }
            KeyCode::Char('U') => return self.offer_unsandboxed(ctx),
            KeyCode::Char('c') => return self.open_control_chooser(ctx),
            KeyCode::Char('e') => {
                let field = self.field();
                if field == Field::LoopMode {
                    self.form.args.loop_mode = match self.form.args.loop_mode {
                        DriverLoopMode::Loop => DriverLoopMode::FreshPerItem,
                        DriverLoopMode::FreshPerItem => DriverLoopMode::Loop,
                    };
                    self.form.defaulted.clear(field);
                } else {
                    self.editing = Some(self.edit_seed(field));
                    self.error = None;
                }
                ctx.needs_redraw = true;
                return ScreenAction::None;
            }
            _ => {}
        }
        match self.popup.on_key(code) {
            ConfirmOutcome::Confirm => self.confirm(ctx),
            ConfirmOutcome::Cancel => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            ConfirmOutcome::Pending => {
                ctx.needs_redraw = true;
                ScreenAction::None
            }
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, area, ctx);
        dim_background(frame, area);
        self.popup.record(render_confirm_popup(
            frame,
            area,
            "Launch driver run",
            self.fitted_body_lines(ctx, area),
            Color::Yellow,
            self.popup.focus,
        ));
    }

    fn handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction {
        click_as_key(&self.popup, input).map_or(ScreenAction::None, |code| {
            self.handle_key(code, KeyModifiers::NONE, ctx)
        })
    }

    fn name(&self) -> &str {
        "driver_launch_form"
    }
}

// ── The unsandboxed launch (D-20) ───────────────────────────────────────────

/// `Launch UNSANDBOXED`: the one way to launch a driver without bwrap and a systemd
/// scope, and only from a host that cannot sandbox. A Red `ConfirmPopup`, default focus
/// No, naming the missing pieces. Only its `y` launches, with `unsandboxed = true`; the
/// choice lives in this screen's arguments and nowhere else, so nothing is remembered
/// for the next launch (28 D-29).
pub struct UnsandboxedConfirmScreen {
    alias: String,
    run_id: String,
    args: LaunchArgs,
    missing: Vec<String>,
    popup: ConfirmPopup,
}

impl UnsandboxedConfirmScreen {
    pub fn new(alias: String, run_id: String, mut args: LaunchArgs, missing: Vec<String>) -> Self {
        args.project = alias.clone();
        args.run_id = run_id.clone();
        Self {
            alias,
            run_id,
            args,
            missing,
            popup: ConfirmPopup::default(),
        }
    }

    /// The prompt the popup shows, every value escaped.
    fn prompt(&self) -> String {
        format!(
            "Launch {} on \"{}\" WITHOUT a sandbox? The driver runs as an ordinary process group with your full user permissions: no bwrap, no systemd scope. Missing: {}. [y/n]",
            shown(&self.run_id),
            shown(&self.alias),
            self.missing
                .iter()
                .map(|m| shown(m))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

crate::ui::screens::adjudicate_screen!(
    UnsandboxedConfirmScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints the Driver tab behind a dimmed popup (the registry alias and the run list) \
     and names the run id (a directory name a cloned repository or a sibling sandbox \
     chooses), the registry alias and the sandbox pieces the host probes found missing \
     (probe output). Every one is drawn through `render_for_terminal`. Fixture states: \
     the hostile identity as the run id, the alias and a missing item.",
);

impl Screen for UnsandboxedConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match self.popup.on_key(code) {
            ConfirmOutcome::Confirm => {
                let mut args = self.args.clone();
                args.unsandboxed = true;
                dispatch(
                    ctx,
                    Action::DriverLaunchRequested {
                        alias: self.alias.clone(),
                        run_id: self.run_id.clone(),
                        args,
                    },
                    &self.alias,
                );
                ScreenAction::Pop
            }
            ConfirmOutcome::Cancel => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            ConfirmOutcome::Pending => {
                ctx.needs_redraw = true;
                ScreenAction::None
            }
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, area, ctx);
        dim_background(frame, area);
        let prompt = self.prompt();
        let body = vec![Line::from(Span::styled(
            super::confirm_popup::strip_key_hint(&prompt).to_string(),
            Style::default().fg(Color::Red),
        ))];
        self.popup.record(render_confirm_popup(
            frame,
            area,
            "Launch UNSANDBOXED",
            body,
            Color::Red,
            self.popup.focus,
        ));
    }

    fn handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction {
        click_as_key(&self.popup, input).map_or(ScreenAction::None, |code| {
            self.handle_key(code, KeyModifiers::NONE, ctx)
        })
    }

    fn name(&self) -> &str {
        "driver_launch_unsandboxed"
    }
}

// ── A new run (D-18) ────────────────────────────────────────────────────────

/// What the slug input says besides its preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewRunStatus {
    /// The run id the slug would make exists already; nothing was written.
    Exists(String),
}

static SLUG: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(crate::driver_templates::SLUG_RE).expect("static regex"));

/// Why a slug cannot make a run id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlugProblem {
    /// Nothing typed yet.
    Empty,
    /// It fails the slug pattern: ASCII, one path component, no leading `.` or `-`.
    NotUsable,
    /// The slug is fine but `<yyyymmdd>-<slug>` would pass the 80-byte run-id bound.
    TooLong,
}

/// `<yyyymmdd>-<slug>` when `slug` makes a usable run id: the same two checks
/// `driver new` makes (`SLUG_RE`, then the run-id shape with its 80-byte bound), so the
/// input never offers an id the creation would refuse.
pub fn slug_run_id(slug: &str, date: &str) -> Result<String, SlugProblem> {
    if slug.is_empty() {
        return Err(SlugProblem::Empty);
    }
    if !SLUG.is_match(slug) {
        return Err(SlugProblem::NotUsable);
    }
    let run_id = format!("{date}-{slug}");
    if !crate::state_reader::driver_frontmatter::is_plain_run_id(&run_id) {
        return Err(SlugProblem::TooLong);
    }
    Ok(run_id)
}

/// The Red row for a slug that cannot be used.
const SLUG_NOT_USABLE: &str =
    "not usable: use letters, digits, \".\", \"_\" or \"-\", starting with a letter or digit";
const SLUG_TOO_LONG: &str = "not usable: the run id <yyyymmdd>-<slug> may be at most 80 bytes";

/// `A run named {run id} already exists. Pick another slug.`
pub fn exists_copy(run_id: &str) -> String {
    format!(
        "A run named {} already exists. Pick another slug.",
        shown(run_id)
    )
}

/// `Created run {run id}. It is checked before launch.`
pub fn created_copy(run_id: &str) -> String {
    format!(
        "Created run {}. It is checked before launch.",
        shown(run_id)
    )
}

/// `{run id} created, not launched: [R] launches it.` (58 cells for an 18-character id,
/// inside the 62-cell status room at 80 columns; UI D4 `[inferred — audit]`).
pub fn left_unlaunched_copy(run_id: &str) -> String {
    format!("{} created, not launched: [R] launches it.", shown(run_id))
}

/// `Could not create the run: {err}`
pub fn create_failed_copy(error: &str) -> String {
    format!("Could not create the run: {}", shown(error))
}

/// The longest slug the input takes: well past anything that makes a usable id, so the
/// not-usable row can explain a too-long one instead of the keys going dead.
const SLUG_INPUT_MAX_CHARS: usize = 90;

/// `s`: the slug input, `New run slug> {buffer}_   [Enter] create  [Esc] cancel`, and
/// under it the run id the slug would make (UTC date), or why it cannot.
///
/// Enter on an empty, unusable or already-submitted slug does nothing; a usable one
/// dispatches [`Action::DriverNewRunRequested`] and waits for the App's answer. The App
/// pops this screen when the run is created; when the id already exists it leaves the
/// screen open and sets [`NewRunStatus::Exists`], which this screen shows under the
/// buffer it kept.
pub struct SlugInputScreen {
    alias: String,
    buffer: String,
    /// A request is in flight: Enter does nothing until the App answers.
    pending: bool,
}

impl SlugInputScreen {
    pub fn new(alias: String) -> Self {
        Self {
            alias,
            buffer: String::new(),
            pending: false,
        }
    }

    fn date() -> String {
        chrono::Utc::now().format("%Y%m%d").to_string()
    }
}

crate::ui::screens::adjudicate_screen!(
    SlugInputScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints the Driver tab behind its footer rows (the registry alias and the run \
     list), echoes the slug the operator typed or pasted, and names the run id an \
     existing run already has (a directory name a cloned repository or a sibling \
     sandbox chooses). Fixture states: the hostile identity as the buffer and as the \
     existing run id.",
);

impl Screen for SlugInputScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        // The App answered a request with "exists": the input is open again.
        if ctx
            .view_cache
            .get(&self.alias)
            .is_some_and(|cache| cache.new_run_status.is_some())
        {
            self.pending = false;
        }
        let edited = |this: &mut Self, ctx: &mut AppContext| {
            this.pending = false;
            if let Some(cache) = ctx.view_cache.get_mut(&this.alias) {
                cache.new_run_status = None;
            }
            ctx.needs_redraw = true;
        };
        match code {
            KeyCode::Esc => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            KeyCode::Backspace => {
                self.buffer.pop();
                edited(self, ctx);
                ScreenAction::None
            }
            KeyCode::Char(c) => {
                if self.buffer.chars().count() < SLUG_INPUT_MAX_CHARS {
                    self.buffer.push(c);
                    edited(self, ctx);
                }
                ScreenAction::None
            }
            KeyCode::Enter => {
                // An id that exists is final until the slug changes: the row under the
                // buffer says to pick another one.
                let exists = ctx
                    .view_cache
                    .get(&self.alias)
                    .is_some_and(|cache| cache.new_run_status.is_some());
                if self.pending || exists || slug_run_id(&self.buffer, &Self::date()).is_err() {
                    return ScreenAction::None;
                }
                self.pending = true;
                dispatch(
                    ctx,
                    Action::DriverNewRunRequested {
                        alias: self.alias.clone(),
                        slug: self.buffer.clone(),
                    },
                    &self.alias,
                );
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let hint = "   [Enter] create  [Esc] cancel";
        let prompt = "  New run slug> ";
        let room = usize::from(area.width)
            .saturating_sub(Span::raw(prompt).width())
            .saturating_sub(1)
            .saturating_sub(Span::raw(hint).width());
        let echoed = super::driver_steer::tail_cells(
            crate::text::render_for_terminal(&self.buffer).as_ref(),
            room,
        );
        let first = Line::from(vec![
            Span::styled(prompt, bold()),
            Span::raw(echoed),
            Span::raw("_"),
            Span::styled(hint, muted()),
        ]);
        let exists = ctx
            .view_cache
            .get(&self.alias)
            .and_then(|cache| cache.new_run_status.as_ref());
        let date = Self::date();
        let second = match (exists, slug_run_id(&self.buffer, &date)) {
            (Some(NewRunStatus::Exists(run_id)), _) => Line::from(Span::styled(
                format!("      {}", exists_copy(run_id)),
                Style::default().fg(Color::Red),
            )),
            (None, Ok(run_id)) => Line::from(Span::styled(
                format!("      run id: {}", shown(&run_id)),
                muted(),
            )),
            (None, Err(SlugProblem::Empty)) => {
                Line::from(Span::styled(format!("      run id: {date}-"), muted()))
            }
            (None, Err(SlugProblem::NotUsable)) => Line::from(Span::styled(
                format!("      {SLUG_NOT_USABLE}"),
                Style::default().fg(Color::Red),
            )),
            (None, Err(SlugProblem::TooLong)) => Line::from(Span::styled(
                format!("      {SLUG_TOO_LONG}"),
                Style::default().fg(Color::Red),
            )),
        };
        // The second row wraps (UI D8): the not-usable copy is longer than 80 columns
        // with its indent. The footer is one input row plus the rows that wrap needs,
        // never more than half the screen. It is not a scrolling pane.
        let second =
            ratatui::widgets::Paragraph::new(second).wrap(ratatui::widgets::Wrap { trim: false });
        let second_rows = u16::try_from(second.line_count(area.width.max(1))).unwrap_or(u16::MAX);
        let footer_rows = 1u16
            .saturating_add(second_rows)
            .min((area.height / 2).max(2));
        let chunks = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(footer_rows),
        ])
        .split(area);
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, chunks[0], ctx);
        let rows = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Length(1),
            ratatui::layout::Constraint::Min(0),
        ])
        .split(chunks[1]);
        frame.render_widget(ratatui::widgets::Paragraph::new(first), rows[0]);
        frame.render_widget(second, rows[1]);
    }

    fn name(&self) -> &str {
        "driver_slug_input"
    }
}

/// How many problems the `Check DIRECTION.md` popup lists before `… k more`.
const POPUP_PROBLEMS_SHOWN: u16 = 8;

/// `Check DIRECTION.md`: `driver check` found problems in the new run's edited body.
/// Launch is blocked until they are fixed. `y` opens the editor on the body again (the
/// new-run flow carries on from the edit), `n` and Esc leave the run created and
/// unlaunched. A Red `ConfirmPopup`, default focus No.
pub struct LintPopupScreen {
    alias: String,
    run_id: String,
    problems: Vec<String>,
    popup: ConfirmPopup,
}

impl LintPopupScreen {
    pub fn new(alias: String, run_id: String, problems: Vec<String>) -> Self {
        Self {
            alias,
            run_id,
            problems,
            popup: ConfirmPopup::default(),
        }
    }
}

crate::ui::screens::adjudicate_screen!(
    LintPopupScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints the Driver tab behind a dimmed popup (the registry alias and the run list) \
     and lists the `driver check` findings of the new run's DIRECTION.md, each quoting \
     text from a file the operator edited and drawn through `render_for_terminal`. The \
     run id is carried for the dispatch and never drawn. Fixture states: the hostile \
     identity as a finding.",
);

impl Screen for LintPopupScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match self.popup.on_key(code) {
            ConfirmOutcome::Confirm => {
                dispatch(
                    ctx,
                    Action::DriverEditRequested {
                        alias: self.alias.clone(),
                        run_id: self.run_id.clone(),
                    },
                    &self.alias,
                );
                ScreenAction::Pop
            }
            ConfirmOutcome::Cancel => {
                dispatch(
                    ctx,
                    Action::DriverNewRunLeft {
                        alias: self.alias.clone(),
                        run_id: self.run_id.clone(),
                    },
                    &self.alias,
                );
                ScreenAction::Pop
            }
            ConfirmOutcome::Pending => {
                ctx.needs_redraw = true;
                ScreenAction::None
            }
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, area, ctx);
        dim_background(frame, area);
        let n = self.problems.len();
        let red = Style::default().fg(Color::Red);
        let header = Line::from(Span::styled(
            format!("driver check found {n} problem(s). Launch is blocked until they are fixed:"),
            red,
        ));
        let question = Line::from("Edit it again now?");
        // The problems give way, the question does not (WR-05, UI D5): they are fitted
        // to the rows left after the header and the question, and at most
        // `POPUP_PROBLEMS_SHOWN` plus a count row are listed on a tall terminal. At a
        // zero budget the list is left out, so the header and the question still fit
        // and `y` stays live.
        let (inner, room) = body_room(area);
        let taken = wrapped_rows(&[header.clone(), question.clone()], inner);
        let rows = room.saturating_sub(taken).min(POPUP_PROBLEMS_SHOWN + 1);
        let problems = self
            .problems
            .iter()
            .map(|problem| Line::from(Span::styled(format!("  {}", shown(problem)), red)))
            .collect();
        let mut body: Vec<Line<'static>> = vec![header];
        if rows > 0 {
            body.extend(fit_list(problems, rows, inner));
        }
        body.push(question);
        self.popup.record(render_confirm_popup(
            frame,
            area,
            "Check DIRECTION.md",
            body,
            Color::Red,
            self.popup.focus,
        ));
    }

    fn handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction {
        click_as_key(&self.popup, input).map_or(ScreenAction::None, |code| {
            self.handle_key(code, KeyModifiers::NONE, ctx)
        })
    }

    fn name(&self) -> &str {
        "driver_lint_popup"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::driver_frontmatter::parse_driver_frontmatter;
    use crate::ui::screens::driver_confirm::tests::{ctx_with_project, ALIAS};
    use tokio::sync::mpsc::UnboundedReceiver;

    const RUN: &str = "20261019-run0";
    const RUN1B: &str = include_str!("../../../tests/fixtures/driverctl/direction/s003-run1b.md");

    fn fm() -> DriverFrontmatter {
        parse_driver_frontmatter(RUN1B).expect("the spike fixture parses")
    }

    fn form_screen(
        kind: LaunchKind,
        form: LaunchForm,
    ) -> (LaunchFormScreen, AppContext, UnboundedReceiver<Action>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let (ctx, rx) = ctx_with_project(dir.path());
        (
            LaunchFormScreen::new(ALIAS.to_string(), RUN.to_string(), kind, form),
            ctx,
            rx,
        )
    }

    fn arrive(ctx: &mut AppContext, preflight: LaunchPreflight) {
        ctx.view_cache
            .entry(ALIAS.to_string())
            .or_default()
            .launch_preflight = Some((RUN.to_string(), preflight));
    }

    fn press(screen: &mut LaunchFormScreen, ctx: &mut AppContext, key: char) -> ScreenAction {
        screen.handle_key(KeyCode::Char(key), KeyModifiers::NONE, ctx)
    }

    fn text_of(lines: &[Line<'_>]) -> Vec<String> {
        lines
            .iter()
            .map(|line| line.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    /// The body row of `label`'s field (`> model           opus`), wherever the order puts it.
    fn field_row<'a>(rows: &'a [String], label: &str) -> &'a str {
        let padded = format!("{label:<16}");
        rows.iter()
            .find(|row| row.get(2..).is_some_and(|rest| rest.starts_with(&padded)))
            .unwrap_or_else(|| panic!("no `{label}` row in {rows:?}"))
    }

    #[test]
    fn launcher_launch_kind_over_every_derived_status() {
        use RunStatus::*;
        let kind = |derived: DerivedStatus| launch_kind_for(&derived);
        assert_eq!(kind(DerivedStatus::NotLaunched), Ok(LaunchKind::Launch));
        for crashed in [
            DerivedStatus::Crashed { written: Running },
            DerivedStatus::AsWritten(Crashed),
            DerivedStatus::AsWritten(Stopped),
            DerivedStatus::AsWritten(Parked),
            DerivedStatus::AsWritten(Stalled),
            DerivedStatus::AsWritten(Paused),
        ] {
            assert_eq!(
                kind(crashed.clone()),
                Ok(LaunchKind::Relaunch),
                "{crashed:?}"
            );
        }
        for held in [Running, Starting] {
            assert_eq!(
                kind(DerivedStatus::AsWritten(held)),
                Err(LaunchRefusal::LockHeld)
            );
        }
        assert_eq!(
            kind(DerivedStatus::LivenessUnknown { written: Running }),
            Err(LaunchRefusal::LivenessUnknown)
        );
        assert_eq!(
            kind(DerivedStatus::AsWritten(Done)),
            Err(LaunchRefusal::NothingToRelaunch("done".to_string()))
        );
        assert_eq!(
            kind(DerivedStatus::AsWritten(Unknown("weird".to_string()))),
            Err(LaunchRefusal::NothingToRelaunch("weird".to_string()))
        );
    }

    #[test]
    fn launcher_launch_the_refusal_copy_is_the_spec_text() {
        assert_eq!(
            refusal_copy(RUN, &LaunchRefusal::LockHeld),
            format!("{RUN} is still running (its run lock is held). Stop it first.")
        );
        assert_eq!(
            refusal_copy(RUN, &LaunchRefusal::LivenessUnknown),
            format!(
                "Cannot tell whether {RUN} is running (the lock probe failed), so it was not relaunched."
            )
        );
        assert_eq!(
            refusal_copy(RUN, &LaunchRefusal::NothingToRelaunch("done".to_string())),
            format!("{RUN} is done: there is nothing to relaunch. Start a new run with [s].")
        );
        // A hostile word and id are escaped.
        let hostile = refusal_copy(
            "a\u{1b}[2Jb",
            &LaunchRefusal::NothingToRelaunch("x\u{1b}[31my".to_string()),
        );
        assert!(!hostile.contains('\u{1b}'), "{hostile:?}");
    }

    #[test]
    fn launcher_launch_prefill_uses_the_recorded_caps_and_model() {
        let form = relaunch_prefill(&fm());
        assert_eq!(form.args.model, "opus");
        assert_eq!(form.args.cap_dispatches, 10);
        assert_eq!(form.args.cap_minutes, 90.0);
        assert_eq!(form.args.cap_gap_rounds, 2);
        assert!(!form.defaulted.model);
        assert!(!form.defaulted.dispatches);
        assert!(!form.defaulted.minutes);
        assert!(!form.defaulted.gap_rounds);
        // Not recorded on the run: the defaults, flagged.
        assert!(form.defaulted.budget);
        assert!(form.defaulted.loop_mode);
        assert_eq!(form.args.max_budget_usd, 10.0);
        assert_eq!(form.args.loop_mode, DriverLoopMode::Loop);
        assert!(!form.minutes_used_up, "9.1 of 90 is not used up");
    }

    #[test]
    fn launcher_launch_prefill_defaults_what_the_run_did_not_record() {
        let mut fm = fm();
        fm.caps.dispatches.limit = None;
        fm.caps.wall_clock_min.limit = None;
        fm.caps.gap_rounds.limit = None;
        fm.connected.clear();
        let form = relaunch_prefill(&fm);
        assert_eq!(form.args.cap_dispatches, 10);
        assert_eq!(form.args.cap_minutes, 60.0);
        assert_eq!(form.args.cap_gap_rounds, crate::cli::DEFAULT_CAP_GAP_ROUNDS);
        assert_eq!(form.args.model, "opus");
        assert_eq!(
            form.defaulted,
            LaunchDefaulted {
                model: true,
                loop_mode: true,
                budget: true,
                dispatches: true,
                minutes: true,
                gap_rounds: true,
            }
        );
        assert!(!form.minutes_used_up);
    }

    #[test]
    fn launcher_launch_prefill_distrusts_a_hostile_model_and_a_forged_cap() {
        let mut fm = fm();
        for model in [
            "--dangerously-skip-permissions",
            "opus\u{1b}[2J",
            "",
            "a b",
            "-x",
            "-opus[1m]",
        ] {
            for conn in &mut fm.connected {
                conn.model = Some(model.to_string());
            }
            let form = relaunch_prefill(&fm);
            assert_eq!(form.args.model, "opus", "{model:?} is not trusted");
            assert!(form.defaulted.model);
        }
        for conn in &mut fm.connected {
            conn.model = Some("opus[1m]".to_string());
        }
        let form = relaunch_prefill(&fm);
        assert_eq!(form.args.model, "opus[1m]");
        assert!(!form.defaulted.model);
        // A cap clap refuses falls back to the default instead of reaching the form.
        fm.caps.wall_clock_min.limit = Some(f64::INFINITY);
        let form = relaunch_prefill(&fm);
        assert_eq!(form.args.cap_minutes, 60.0);
        assert!(form.defaulted.minutes);
        fm.caps.wall_clock_min.limit = Some(-5.0);
        assert!(relaunch_prefill(&fm).defaulted.minutes);
    }

    #[test]
    fn launcher_launch_prefill_flags_a_used_up_wall_clock() {
        let mut fm = fm();
        fm.caps.wall_clock_min.used = 90.0;
        let form = relaunch_prefill(&fm);
        assert!(form.minutes_used_up);
        // Raising the minutes in the form clears the hint.
        let (mut screen, mut ctx, _rx) = form_screen(LaunchKind::Relaunch, form);
        for _ in 0..4 {
            press(&mut screen, &mut ctx, 'j');
        }
        press(&mut screen, &mut ctx, 'e');
        for _ in 0..2 {
            screen.handle_key(KeyCode::Backspace, KeyModifiers::NONE, &mut ctx);
        }
        for c in "120".chars() {
            press(&mut screen, &mut ctx, c);
        }
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert_eq!(screen.args().cap_minutes, 120.0);
        assert!(!screen.form.minutes_used_up);
        // The limit alone does not flag a run that recorded none.
        fm.caps.wall_clock_min.limit = None;
        assert!(!relaunch_prefill(&fm).minutes_used_up);
    }

    #[test]
    fn launcher_launch_a_fresh_form_is_the_clap_defaults() {
        let form = LaunchForm::fresh();
        assert_eq!(form.args.model, "opus");
        assert_eq!(form.args.max_budget_usd, 10.0);
        assert_eq!(form.args.cap_dispatches, 10);
        assert_eq!(form.args.cap_minutes, 60.0);
        assert_eq!(form.args.cap_gap_rounds, crate::cli::DEFAULT_CAP_GAP_ROUNDS);
        assert_eq!(form.defaulted, LaunchDefaulted::default());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_launch_options_mirror_what_driver_launch_builds() {
        let mut args = launch_defaults();
        args.run_id = RUN.to_string();
        let opts = launch_options_for("alias", Path::new("/p"), &args, true, None);
        assert_eq!(opts.alias, "alias");
        assert_eq!(opts.project, Path::new("/p"));
        assert_eq!(opts.run_id, RUN);
        assert_eq!(opts.prompt_file, None);
        assert_eq!(opts.model, "opus");
        assert_eq!(opts.loop_mode, crate::launcher::run::LoopMode::Loop);
        assert_eq!(opts.max_budget_usd, 10.0);
        assert_eq!(opts.limits.dispatches, Some(10));
        assert_eq!(opts.limits.wall_clock_min, Some(60.0));
        assert_eq!(
            opts.limits.gap_rounds,
            Some(crate::cli::DEFAULT_CAP_GAP_ROUNDS)
        );
        assert!(!opts.unsandboxed);
        assert!(opts.open_pr);
    }

    /// The golden argv: the daemon is started with exactly this vector.
    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_launch_the_default_options_produce_the_golden_argv() {
        let mut args = launch_defaults();
        args.run_id = "20261019-run0".to_string();
        let opts = launch_options_for("alias", Path::new("/proj"), &args, false, None);
        let argv: Vec<String> =
            crate::launcher::run::launch_argv(&opts, Some(Path::new("/cfg/registry.json")))
                .into_iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
        assert_eq!(
            argv,
            [
                "--config",
                "/cfg/registry.json",
                "driver",
                "launch",
                "/proj",
                "20261019-run0",
                "--model",
                "opus",
                "--loop-mode",
                "loop",
                "--max-budget-usd",
                "10",
                "--cap-dispatches",
                "10",
                "--cap-minutes",
                "60",
                "--cap-gap-rounds",
                "2",
                "--foreground",
                "--daemon-child",
            ]
        );
    }

    #[test]
    fn launcher_launch_the_form_shows_the_six_fields_and_the_loading_sandbox_row() {
        let (screen, ctx, _rx) = form_screen(LaunchKind::Relaunch, relaunch_prefill(&fm()));
        let rows = text_of(&screen.body_lines(&ctx));
        // WR-05: the title, the spend paragraph and the sandbox row come before the
        // fields; the key row is last.
        assert_eq!(rows[0], format!("Launch {RUN} on \"{ALIAS}\"?"));
        assert!(rows[1].starts_with("Each driver session may spend up to $10.00"));
        assert_eq!(rows[2], "Sandbox: Loading...");
        assert_eq!(rows[3], "> model           opus");
        assert_eq!(rows[4], "  loop mode       loop (default)");
        assert_eq!(rows[5], "  spend ceiling   $10.00 (default)");
        assert_eq!(rows[6], "  dispatches      10");
        assert_eq!(rows[7], "  minutes         90");
        assert_eq!(rows[8], "  gap rounds      2 per phase");
        assert_eq!(
            rows.last().map(String::as_str),
            Some("[j/k] field  [e] edit field")
        );
    }

    #[test]
    fn launcher_launch_y_is_inert_until_the_preflight_arrives() {
        let (mut screen, mut ctx, mut rx) =
            form_screen(LaunchKind::Relaunch, relaunch_prefill(&fm()));
        assert!(matches!(
            press(&mut screen, &mut ctx, 'y'),
            ScreenAction::None
        ));
        assert!(
            rx.try_recv().is_err(),
            "nothing is dispatched while loading"
        );
        // A preflight for ANOTHER run does not count.
        ctx.view_cache
            .entry(ALIAS.to_string())
            .or_default()
            .launch_preflight = Some(("someone-else".to_string(), LaunchPreflight::default()));
        assert!(matches!(
            press(&mut screen, &mut ctx, 'y'),
            ScreenAction::None
        ));
        assert!(rx.try_recv().is_err());
        arrive(&mut ctx, LaunchPreflight::default());
        assert!(matches!(
            press(&mut screen, &mut ctx, 'y'),
            ScreenAction::Pop
        ));
        let Some(Action::DriverLaunchRequested {
            alias,
            run_id,
            args,
        }) = rx.try_recv().ok()
        else {
            panic!("y must dispatch a launch request");
        };
        assert_eq!((alias.as_str(), run_id.as_str()), (ALIAS, RUN));
        assert_eq!(args.run_id, RUN);
        assert_eq!(args.project, ALIAS);
        assert!(!args.unsandboxed);
        assert_eq!(args.cap_minutes, 90.0);
        assert!(rx.try_recv().is_err(), "exactly one request");
    }

    #[test]
    fn launcher_launch_n_and_escape_close_the_form_and_launch_nothing() {
        for key in [KeyCode::Char('n'), KeyCode::Esc, KeyCode::Enter] {
            let (mut screen, mut ctx, mut rx) =
                form_screen(LaunchKind::Relaunch, relaunch_prefill(&fm()));
            arrive(&mut ctx, LaunchPreflight::default());
            assert!(matches!(
                screen.handle_key(key, KeyModifiers::NONE, &mut ctx),
                ScreenAction::Pop
            ));
            assert!(rx.try_recv().is_err(), "{key:?} launches nothing");
        }
    }

    #[test]
    fn launcher_launch_editing_validates_through_clap_and_y_n_are_text() {
        let (mut screen, mut ctx, mut rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        arrive(&mut ctx, LaunchPreflight::default());
        // spend ceiling
        press(&mut screen, &mut ctx, 'j');
        press(&mut screen, &mut ctx, 'j');
        press(&mut screen, &mut ctx, 'e');
        for _ in 0..4 {
            screen.handle_key(KeyCode::Backspace, KeyModifiers::NONE, &mut ctx);
        }
        press(&mut screen, &mut ctx, '0');
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        let rows = text_of(&screen.body_lines(&ctx));
        let budget = field_row(&rows, "spend ceiling");
        assert!(
            budget.contains("must be a positive number"),
            "an invalid value shows the copy and editing continues: {rows:?}"
        );
        assert!(budget.contains("0_"), "{rows:?}");
        assert_eq!(screen.args().max_budget_usd, 10.0, "nothing was stored");
        // While editing, y and n are text: nothing is dispatched and the form stays.
        for c in ['y', 'n'] {
            assert!(matches!(
                press(&mut screen, &mut ctx, c),
                ScreenAction::None
            ));
        }
        assert!(rx.try_recv().is_err());
        for _ in 0..3 {
            screen.handle_key(KeyCode::Backspace, KeyModifiers::NONE, &mut ctx);
        }
        for c in "25.5".chars() {
            press(&mut screen, &mut ctx, c);
        }
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert_eq!(screen.args().max_budget_usd, 25.5);
        let rows = text_of(&screen.body_lines(&ctx));
        assert_eq!(
            field_row(&rows, "spend ceiling"),
            "> spend ceiling   $25.50"
        );
        // Esc cancels an edit without storing it.
        press(&mut screen, &mut ctx, 'e');
        press(&mut screen, &mut ctx, '9');
        screen.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx);
        assert_eq!(screen.args().max_budget_usd, 25.5);
        // And Esc outside an edit closes the form.
        assert!(matches!(
            screen.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx),
            ScreenAction::Pop
        ));
    }

    #[test]
    fn launcher_launch_every_numeric_field_is_judged_by_the_launch_parser() {
        // field index, bad text, good text, check
        let (mut screen, mut ctx, _rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        let cases: [(usize, &str, &str); 4] = [
            (2, "-1", "7"),
            (3, "ten", "12"),
            (4, "nan", "45"),
            (5, "1.5", "3"),
        ];
        for (index, bad, good) in cases {
            screen.cursor = index;
            press(&mut screen, &mut ctx, 'e');
            screen.editing = Some(bad.to_string());
            screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
            assert_eq!(
                screen.error.as_deref(),
                Some("must be a positive number"),
                "{bad}"
            );
            screen.editing = Some(good.to_string());
            screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
            assert!(screen.editing.is_none(), "{good} is accepted");
        }
        assert_eq!(screen.args().max_budget_usd, 7.0);
        assert_eq!(screen.args().cap_dispatches, 12);
        assert_eq!(screen.args().cap_minutes, 45.0);
        assert_eq!(screen.args().cap_gap_rounds, 3);
    }

    #[test]
    fn launcher_launch_the_model_field_refuses_an_empty_or_control_value() {
        let (mut screen, mut ctx, _rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        press(&mut screen, &mut ctx, 'e');
        screen.editing = Some("   ".to_string());
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert_eq!(screen.error.as_deref(), Some("must be a model name"));
        screen.editing = Some("op\u{1b}us".to_string());
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert_eq!(screen.error.as_deref(), Some("must be a model name"));
        assert_eq!(screen.args().model, "opus");
        screen.editing = Some("sonnet".to_string());
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert_eq!(screen.args().model, "sonnet");
    }

    #[test]
    fn launcher_launch_e_on_the_loop_mode_toggles_it() {
        let (mut screen, mut ctx, _rx) = form_screen(LaunchKind::Relaunch, relaunch_prefill(&fm()));
        press(&mut screen, &mut ctx, 'j');
        press(&mut screen, &mut ctx, 'e');
        assert!(screen.editing.is_none(), "no text box for a two-way choice");
        assert_eq!(screen.args().loop_mode, DriverLoopMode::FreshPerItem);
        let rows = text_of(&screen.body_lines(&ctx));
        assert_eq!(
            field_row(&rows, "loop mode"),
            "> loop mode       fresh-per-item"
        );
        press(&mut screen, &mut ctx, 'e');
        assert_eq!(screen.args().loop_mode, DriverLoopMode::Loop);
    }

    #[test]
    fn launcher_launch_a_host_that_cannot_sandbox_refuses_y() {
        let (mut screen, mut ctx, mut rx) =
            form_screen(LaunchKind::Relaunch, relaunch_prefill(&fm()));
        arrive(
            &mut ctx,
            LaunchPreflight {
                missing: vec!["bwrap".to_string()],
                ..LaunchPreflight::default()
            },
        );
        assert!(matches!(
            press(&mut screen, &mut ctx, 'y'),
            ScreenAction::None
        ));
        assert!(rx.try_recv().is_err(), "nothing is launched");
        let rows = text_of(&screen.body_lines(&ctx));
        assert!(rows.contains(&"Sandbox unavailable: missing bwrap.".to_string()));
        assert!(rows.contains(&"A sandboxed launch is refused on this host.".to_string()));
        assert!(rows.contains(
            &"Launch refused: this host cannot sandbox the driver (missing: bwrap). Press [U] to launch it unsandboxed instead."
                .to_string()
        ));
    }

    #[test]
    fn launcher_launch_error_copy_names_that_nothing_was_started() {
        let copy = |e: LauncherError| launch_error_copy(RUN, &e);
        assert_eq!(
            copy(LauncherError::Locked {
                detail: "held".to_string()
            }),
            format!("Could not launch {RUN}: it is already launched. Nothing was started.")
        );
        assert_eq!(
            copy(LauncherError::DaemonExited {
                code: 3,
                detail: String::new()
            }),
            format!("Could not launch {RUN}: it is already launched. Nothing was started.")
        );
        let invalid = copy(LauncherError::DirectionInvalid {
            problems: vec![
                "DIRECTION.md:7: item I-01 has no done-when".to_string(),
                "DIRECTION.md:9: other".to_string(),
            ],
        });
        assert!(
            invalid.starts_with(&format!(
                "Could not launch {RUN}: DIRECTION.md failed driver check (2 problem(s)). Nothing was started."
            )),
            "{invalid}"
        );
        assert!(invalid.contains("DIRECTION.md:7: item I-01 has no done-when"));
        assert!(!invalid.contains("DIRECTION.md:9"));
        let exhausted = copy(LauncherError::CapExhausted {
            detail: "the run has used 90 of its 90 minutes".to_string(),
        });
        assert_eq!(
            exhausted,
            format!(
                "Could not launch {RUN}: the run has used 90 of its 90 minutes. Raise minutes in the launch form. Nothing was started."
            )
        );
        assert_eq!(
            copy(LauncherError::SandboxUnavailable {
                missing: vec!["bwrap".to_string(), "user-systemd".to_string()]
            }),
            "Launch refused: this host cannot sandbox the driver (missing: bwrap, user-systemd). Press [U] to launch it unsandboxed instead."
        );
        assert_eq!(
            copy(LauncherError::ClaudeVersion {
                detail: "too old".to_string()
            }),
            "Launch refused: too old"
        );
        for other in [
            LauncherError::Auth {
                detail: "no token".to_string(),
            },
            LauncherError::Io {
                detail: "disk".to_string(),
            },
            LauncherError::UnsupportedPlatform,
            LauncherError::DaemonExited {
                code: 1,
                detail: "boom".to_string(),
            },
        ] {
            let text = copy(other.clone());
            assert!(
                text.starts_with(&format!("Could not launch {RUN}: ")),
                "{other:?}: {text}"
            );
            assert!(text.ends_with("Nothing was started."), "{other:?}: {text}");
        }
    }

    #[test]
    fn launcher_launch_error_copy_is_escaped_and_capped() {
        let hostile = launch_error_copy(
            "r\u{1b}[2Jun",
            &LauncherError::DaemonExited {
                code: 1,
                detail: format!("l1\u{1b}[31m\nl2\nl3\nl4-never-shown\n{}", "x".repeat(500)),
            },
        );
        assert!(!hostile.contains('\u{1b}'), "{hostile:?}");
        assert!(!hostile.contains("l4-never-shown"), "three lines at most");
        assert!(hostile.chars().count() < 450, "{}", hostile.chars().count());
        let long = launch_error_copy(
            RUN,
            &LauncherError::Auth {
                detail: "y".repeat(2_000),
            },
        );
        assert!(long.chars().count() < 450, "{}", long.chars().count());
        assert!(long.ends_with("Nothing was started."));
    }

    // ── Task 2: refusals, the unsandboxed confirmation, the spend copy ──────

    /// Draw `screen` into a `TestBackend` and join the cells row by row.
    fn draw(screen: &dyn Screen, ctx: &AppContext, width: u16, height: u16) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
        terminal
            .draw(|frame| screen.render(frame, frame.area(), ctx))
            .expect("draw");
        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .map(|cell| cell.symbol().to_string())
                            .unwrap_or_default()
                    })
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn degraded() -> LaunchPreflight {
        LaunchPreflight {
            missing: vec!["bwrap".to_string()],
            ..LaunchPreflight::default()
        }
    }

    fn replaced_name(action: ScreenAction) -> (String, Box<dyn Screen>) {
        match action {
            ScreenAction::Replace(screen) => (screen.name().to_string(), screen),
            ScreenAction::Push(_) => panic!("U must replace the form, not stack on it"),
            _ => panic!("expected a screen replacement"),
        }
    }

    #[test]
    fn launcher_launch_a_degraded_host_offers_u_and_y_launches_nothing() {
        let (mut screen, mut ctx, mut rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        arrive(&mut ctx, degraded());
        let rows = text_of(&screen.body_lines(&ctx));
        assert!(rows.contains(&"[U] launch UNSANDBOXED instead (asks again)".to_string()));
        assert!(matches!(
            press(&mut screen, &mut ctx, 'y'),
            ScreenAction::None
        ));
        assert!(rx.try_recv().is_err(), "y launches nothing");
        let rows = text_of(&screen.body_lines(&ctx));
        assert!(rows.iter().any(|r| r
            .starts_with("Launch refused: this host cannot sandbox the driver (missing: bwrap)")));
        // The args the form would launch with stay sandboxed whatever happened.
        assert!(!screen.args().unsandboxed);
    }

    #[test]
    fn launcher_launch_u_then_n_dispatches_nothing() {
        // `n`, Esc and Enter (default focus is No) all decline, and a decline neither
        // launches anything nor leaves a screen the user can submit again.
        for key in [KeyCode::Char('n'), KeyCode::Esc, KeyCode::Enter] {
            let (mut screen, mut ctx, mut rx) =
                form_screen(LaunchKind::Launch, LaunchForm::fresh());
            arrive(&mut ctx, degraded());
            let (name, mut confirm) = replaced_name(press(&mut screen, &mut ctx, 'U'));
            assert_eq!(name, "driver_launch_unsandboxed");
            let action = confirm.handle_key(key, KeyModifiers::NONE, &mut ctx);
            assert!(matches!(action, ScreenAction::Pop), "{key:?}");
            assert!(rx.try_recv().is_err(), "{key:?} launches nothing");
        }
    }

    #[test]
    fn launcher_launch_u_then_y_launches_unsandboxed_once_and_remembers_nothing() {
        let (mut screen, mut ctx, mut rx) =
            form_screen(LaunchKind::Relaunch, relaunch_prefill(&fm()));
        arrive(&mut ctx, degraded());
        let (_, mut confirm) = replaced_name(press(&mut screen, &mut ctx, 'U'));
        // Nothing is dispatched by merely opening the confirmation.
        assert!(rx.try_recv().is_err());
        let action = confirm.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::Pop));
        let Some(Action::DriverLaunchRequested {
            alias,
            run_id,
            args,
        }) = rx.try_recv().ok()
        else {
            panic!("y must dispatch exactly one launch");
        };
        assert_eq!((alias.as_str(), run_id.as_str()), (ALIAS, RUN));
        assert!(args.unsandboxed, "the second confirmation sets the flag");
        assert_eq!(args.cap_minutes, 90.0, "the form's values travel with it");
        assert!(rx.try_recv().is_err(), "exactly one request");

        // The choice is not remembered: the form's own arguments never carried it, and
        // a form opened again (what `R` builds) starts sandboxed.
        assert!(!screen.args().unsandboxed);
        let reopened = LaunchFormScreen::new(
            ALIAS.to_string(),
            RUN.to_string(),
            LaunchKind::Relaunch,
            relaunch_prefill(&fm()),
        );
        assert!(!reopened.args().unsandboxed);
    }

    #[test]
    fn launcher_launch_the_unsandboxed_confirmation_names_what_is_missing() {
        let (mut screen, mut ctx, _rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        arrive(
            &mut ctx,
            LaunchPreflight {
                missing: vec!["bwrap".to_string(), "user-systemd".to_string()],
                ..LaunchPreflight::default()
            },
        );
        let (_, confirm) = replaced_name(press(&mut screen, &mut ctx, 'U'));
        let text = draw(confirm.as_ref(), &ctx, 100, 30);
        assert!(text.contains("Launch UNSANDBOXED"), "{text}");
        assert!(text.contains("WITHOUT a sandbox?"), "{text}");
        // The popup wraps its prose, so each phrase is checked on its own.
        assert!(text.contains("no systemd scope."), "{text}");
        assert!(text.contains("Missing: bwrap, user-systemd."), "{text}");
        assert!(text.contains("[ No ]"), "{text}");
    }

    #[test]
    fn launcher_launch_u_is_inert_unless_the_host_is_missing_sandbox_pieces() {
        for preflight in [
            None,
            Some(LaunchPreflight::default()),
            Some(LaunchPreflight {
                claude_refusal: Some("too old".to_string()),
                missing: vec!["bwrap".to_string()],
                ..LaunchPreflight::default()
            }),
        ] {
            let (mut screen, mut ctx, mut rx) =
                form_screen(LaunchKind::Launch, LaunchForm::fresh());
            if let Some(preflight) = preflight.clone() {
                arrive(&mut ctx, preflight);
            }
            assert!(
                matches!(press(&mut screen, &mut ctx, 'U'), ScreenAction::None),
                "{preflight:?}"
            );
            assert!(rx.try_recv().is_err());
        }
    }

    #[test]
    fn launcher_launch_a_claude_refusal_has_no_u_and_y_launches_nothing() {
        let (mut screen, mut ctx, mut rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        arrive(
            &mut ctx,
            LaunchPreflight {
                claude_refusal: Some("Claude Code 2.0.0 is older than 2.1.295".to_string()),
                missing: vec!["bwrap".to_string()],
                ..LaunchPreflight::default()
            },
        );
        let rows = text_of(&screen.body_lines(&ctx));
        assert!(
            rows.contains(&"Launch refused: Claude Code 2.0.0 is older than 2.1.295".to_string())
        );
        assert!(
            !rows.iter().any(|r| r.contains("[U]")),
            "both postures refuse it, so no unsandboxed offer: {rows:?}"
        );
        assert!(matches!(
            press(&mut screen, &mut ctx, 'y'),
            ScreenAction::None
        ));
        assert!(rx.try_recv().is_err());
        // Said once, not twice.
        let rows = text_of(&screen.body_lines(&ctx));
        assert_eq!(
            rows.iter()
                .filter(|r| r.starts_with("Launch refused: Claude Code"))
                .count(),
            1
        );
    }

    #[test]
    fn launcher_launch_a_paused_body_blocks_y_and_c_opens_the_chooser() {
        for word in ["pause", "stop"] {
            let (mut screen, mut ctx, mut rx) =
                form_screen(LaunchKind::Relaunch, relaunch_prefill(&fm()));
            arrive(
                &mut ctx,
                LaunchPreflight {
                    control: Some(word.to_string()),
                    ..LaunchPreflight::default()
                },
            );
            let rows = text_of(&screen.body_lines(&ctx));
            assert!(
                rows.contains(&format!(
                    "control is {word}: the launcher refuses to start a run whose DIRECTION.md says control: {word}. Set control: run with [c] first."
                )),
                "{rows:?}"
            );
            assert!(matches!(
                press(&mut screen, &mut ctx, 'y'),
                ScreenAction::None
            ));
            assert!(rx.try_recv().is_err(), "y launches nothing on {word}");
            // `c` closes the form and opens the chooser; nothing is written for the user.
            let (name, _chooser) = replaced_name(press(&mut screen, &mut ctx, 'c'));
            assert_eq!(name, "driver_control");
            assert!(rx.try_recv().is_err(), "no control write is dispatched");
        }
    }

    #[test]
    fn launcher_launch_c_is_inert_when_the_body_says_run() {
        let (mut screen, mut ctx, mut rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        arrive(
            &mut ctx,
            LaunchPreflight {
                control: Some("run".to_string()),
                ..LaunchPreflight::default()
            },
        );
        assert!(!text_of(&screen.body_lines(&ctx))
            .iter()
            .any(|r| r.starts_with("control is")));
        assert!(matches!(
            press(&mut screen, &mut ctx, 'c'),
            ScreenAction::None
        ));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn launcher_launch_driver_check_problems_block_y_and_warnings_do_not() {
        let (mut screen, mut ctx, mut rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        arrive(
            &mut ctx,
            LaunchPreflight {
                lint_problems: (1..=5)
                    .map(|n| format!("DIRECTION.md:{n}: placeholder {{short title}}"))
                    .collect(),
                lint_warnings: vec!["DIRECTION.md:9: warning: budget above the cap".to_string()],
                ..LaunchPreflight::default()
            },
        );
        let rows = text_of(&screen.body_lines(&ctx));
        assert!(rows.contains(&"driver check found 5 problem(s). Close this form, then fix them with [e] before launching.".to_string()), "{rows:?}");
        for n in 1..=5 {
            assert!(
                rows.contains(&format!("  DIRECTION.md:{n}: placeholder {{short title}}")),
                "the unfitted body lists every problem: {rows:?}"
            );
        }
        assert!(rows.contains(&"  DIRECTION.md:9: warning: budget above the cap".to_string()));
        assert!(matches!(
            press(&mut screen, &mut ctx, 'y'),
            ScreenAction::None
        ));
        assert!(rx.try_recv().is_err(), "y launches nothing with problems");

        // Warnings alone never block.
        let (mut screen, mut ctx, mut rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        arrive(
            &mut ctx,
            LaunchPreflight {
                lint_warnings: vec!["DIRECTION.md:9: warning: x".to_string()],
                ..LaunchPreflight::default()
            },
        );
        assert!(matches!(
            press(&mut screen, &mut ctx, 'y'),
            ScreenAction::Pop
        ));
        assert!(matches!(
            rx.try_recv().ok(),
            Some(Action::DriverLaunchRequested { .. })
        ));
    }

    #[test]
    fn launcher_launch_the_spend_line_is_per_session_and_gains_the_multiple() {
        let (mut screen, mut ctx, _rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        let rows = text_of(&screen.body_lines(&ctx));
        let spend = "Each driver session may spend up to $10.00 of Claude usage (claude --max-budget-usd). It runs detached and keeps running after you close this TUI.";
        assert!(rows.contains(&spend.to_string()), "{rows:?}");
        let multiple =
            "fresh-per-item starts one session per item, so the run can spend a multiple of this.";
        assert!(!rows.contains(&multiple.to_string()));
        // Toggle the loop mode to fresh-per-item.
        press(&mut screen, &mut ctx, 'j');
        press(&mut screen, &mut ctx, 'e');
        let rows = text_of(&screen.body_lines(&ctx));
        assert!(rows.contains(&spend.to_string()));
        assert!(rows.contains(&multiple.to_string()), "{rows:?}");
        // And the ceiling follows an edit.
        press(&mut screen, &mut ctx, 'j');
        press(&mut screen, &mut ctx, 'e');
        screen.editing = Some("2.5".to_string());
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        let rows = text_of(&screen.body_lines(&ctx));
        assert!(rows
            .iter()
            .any(|r| r.starts_with("Each driver session may spend up to $2.50 of")));
        // It never claims a hard per-run ceiling.
        for row in &rows {
            assert!(!row.starts_with("This run"), "{row}");
        }
    }

    #[test]
    fn launcher_launch_a_used_up_wall_clock_shows_the_raise_hint() {
        let mut fm = fm();
        fm.caps.wall_clock_min.used = 95.0;
        let (screen, ctx, _rx) = form_screen(LaunchKind::Relaunch, relaunch_prefill(&fm));
        let rows = text_of(&screen.body_lines(&ctx));
        assert!(
            rows.contains(
                &"  minutes: the old limit is used up. Raise it or the launcher refuses this launch."
                    .to_string()
            ),
            "{rows:?}"
        );
    }

    #[test]
    fn launcher_launch_the_form_renders_at_a_small_terminal_without_clipping_the_buttons() {
        let (mut screen, mut ctx, _rx) = form_screen(LaunchKind::Relaunch, relaunch_prefill(&fm()));
        arrive(&mut ctx, LaunchPreflight::default());
        let _ = &mut screen;
        let text = draw(&screen, &ctx, 80, 24);
        assert!(text.contains("Launch driver run"), "{text}");
        assert!(
            text.contains("[ Yes ]") && text.contains("[ No ]"),
            "{text}"
        );
        assert!(text.contains("Sandbox: bwrap + systemd scope"), "{text}");
    }

    // ── Plan 30-09 Task 1: the essentials outlive the findings (WR-05, UI D5) ──

    const SPEND_SENTENCE: &str = "Each driver session may spend up to $10.00 of Claude usage (claude --max-budget-usd). It runs detached and keeps running after you close this TUI.";

    /// The popup's inner text of an 80-column draw (the popup is 72 wide, inner 70 from
    /// column 5), rows joined by one space with runs of blanks collapsed, so a wrapped
    /// sentence reads as the sentence.
    fn popup_prose(drawn: &str) -> String {
        drawn
            .lines()
            .map(|row| row.chars().skip(5).take(70).collect::<String>())
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn warnings(count: usize) -> LaunchPreflight {
        LaunchPreflight {
            lint_warnings: (1..=count)
                .map(|n| format!("DIRECTION.md:{n}: warning: item I-0{n} has no budget"))
                .collect(),
            ..LaunchPreflight::default()
        }
    }

    /// How many `DIRECTION.md:{n}: warning` rows the drawing lists, and the `k` of the
    /// `… k more` row (0 when there is none).
    fn warnings_shown(prose: &str) -> (usize, usize) {
        let shown = prose.matches(": warning: item I-0").count();
        let more = prose
            .split("\u{2026} ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|k| k.parse::<usize>().ok())
            .unwrap_or(0);
        (shown, more)
    }

    #[test]
    fn launcher_launch_the_spend_and_sandbox_rows_outlive_the_findings() {
        use crate::ui::screens::confirm_popup::{body_room, wrapped_rows};
        // The essential rows at the default form: the title, the spend paragraph, the
        // sandbox row and the six fields, counted the way the popup wraps them.
        let (mut screen, mut ctx, mut rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        arrive(&mut ctx, LaunchPreflight::default());
        let mut essential = screen.body_lines(&ctx);
        essential.pop(); // the key row is not essential
        let (inner, room) = body_room(Rect::new(0, 0, 80, 18));
        assert_eq!((inner, room), (70, 13));
        assert_eq!(
            wrapped_rows(&essential, inner),
            11,
            "title 1 + spend 3 + sandbox 1 + six fields 6, leaving 2 finding rows at 80x18"
        );

        // 80x24, three warnings: everything is on screen and y launches.
        arrive(&mut ctx, warnings(3));
        let tall = popup_prose(&draw(&screen, &ctx, 80, 24));
        assert!(tall.contains(SPEND_SENTENCE), "{tall}");
        assert!(tall.contains(SANDBOX_OK), "{tall}");
        for label in Field::ALL.map(Field::label) {
            assert!(tall.contains(label), "{label}: {tall}");
        }
        assert_eq!(warnings_shown(&tall), (3, 0), "{tall}");
        assert!(!screen.popup.clipped());

        // 80x18, six warnings: the warnings give way, the spend and sandbox rows do not,
        // and the confirmation is still live.
        arrive(&mut ctx, warnings(6));
        let mid = popup_prose(&draw(&screen, &ctx, 80, 18));
        assert!(mid.contains(SPEND_SENTENCE), "{mid}");
        assert!(mid.contains(SANDBOX_OK), "{mid}");
        for label in Field::ALL.map(Field::label) {
            assert!(mid.contains(label), "{label}: {mid}");
        }
        let (shown, more) = warnings_shown(&mid);
        assert!((1..6).contains(&shown), "some warnings are cut: {mid}");
        assert_eq!(shown + more, 6, "the count row counts what was cut: {mid}");
        assert!(!screen.popup.clipped(), "the essentials fit: {mid}");
        assert!(matches!(
            press(&mut screen, &mut ctx, 'y'),
            ScreenAction::Pop
        ));
        assert!(matches!(
            rx.try_recv().ok(),
            Some(Action::DriverLaunchRequested { .. })
        ));
        assert!(rx.try_recv().is_err(), "exactly one request");
    }

    #[test]
    fn launcher_launch_a_degraded_host_at_80x24_still_shows_the_two_red_rows_and_u() {
        let (mut screen, mut ctx, mut rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        arrive(&mut ctx, degraded());
        let prose = popup_prose(&draw(&screen, &ctx, 80, 24));
        assert!(prose.contains(SPEND_SENTENCE), "{prose}");
        assert!(
            prose.contains("Sandbox unavailable: missing bwrap."),
            "{prose}"
        );
        assert!(
            prose.contains("A sandboxed launch is refused on this host."),
            "{prose}"
        );
        assert!(
            prose.contains("[U] launch UNSANDBOXED instead (asks again)"),
            "{prose}"
        );
        assert!(matches!(
            press(&mut screen, &mut ctx, 'y'),
            ScreenAction::None
        ));
        assert!(rx.try_recv().is_err(), "y still refuses");
    }

    #[test]
    fn launcher_launch_a_form_cut_off_at_80x12_launches_nothing() {
        let (mut screen, mut ctx, mut rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        arrive(&mut ctx, LaunchPreflight::default());
        let drawn = draw(&screen, &ctx, 80, 12);
        let prose = popup_prose(&drawn);
        assert!(prose.contains(SPEND_SENTENCE), "{prose}");
        assert!(prose.contains(SANDBOX_OK), "{prose}");
        assert!(
            prose.contains("Cut off: enlarge the terminal to confirm"),
            "{prose}"
        );
        assert!(screen.popup.clipped());
        // y, and Enter on Yes, dispatch nothing while the form is cut off.
        assert!(matches!(
            press(&mut screen, &mut ctx, 'y'),
            ScreenAction::None
        ));
        screen.handle_key(KeyCode::Tab, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(
            screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx),
            ScreenAction::None
        ));
        assert!(rx.try_recv().is_err(), "a cut-off form launches nothing");
        // n still leaves.
        assert!(matches!(
            press(&mut screen, &mut ctx, 'n'),
            ScreenAction::Pop
        ));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn launcher_launch_the_lint_popup_keeps_its_question_at_80x12() {
        let (mut screen, mut ctx, mut rx) = lint_popup(6);
        let prose = popup_prose(&draw(&screen, &ctx, 80, 12));
        assert!(prose.contains("Edit it again now?"), "{prose}");
        assert!(
            prose.contains("DIRECTION.md:1: unfilled placeholder"),
            "{prose}"
        );
        let more: usize = prose
            .split("\u{2026} ")
            .nth(1)
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|k| k.parse().ok())
            .expect("a `… k more` row");
        let listed = prose.matches("unfilled placeholder").count();
        assert_eq!(listed + more, 6, "{prose}");
        assert!(listed < 6, "{prose}");
        assert!(!screen.popup.clipped(), "the question fits: {prose}");
        assert!(matches!(
            screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx),
            ScreenAction::Pop
        ));
        assert!(matches!(
            rx.try_recv().ok(),
            Some(Action::DriverEditRequested { .. })
        ));
    }

    /// IN-04: at the height where the header and the question exactly fill the room, the
    /// problems get no rows. They are left out, so the popup is not clipped and `y` lives.
    #[test]
    fn launcher_launch_the_lint_popup_confirms_when_its_problems_get_no_rows() {
        use crate::ui::screens::confirm_popup::{body_room, wrapped_rows, CLIPPED_HINT};
        let header = "driver check found 6 problem(s). Launch is blocked until they are fixed:";
        let (inner, room) = body_room(Rect::new(0, 0, 80, 8));
        assert_eq!((inner, room), (70, 3));
        let taken = wrapped_rows(
            &[Line::from(header), Line::from("Edit it again now?")],
            inner,
        );
        assert_eq!(taken, room, "the problems' row budget is exactly 0");

        let (mut screen, mut ctx, mut rx) = lint_popup(6);
        let drawn = draw(&screen, &ctx, 80, 8);
        let prose = popup_prose(&drawn);
        assert!(!screen.popup.clipped(), "{drawn}");
        assert!(prose.contains("Edit it again now?"), "{prose}");
        assert!(!prose.contains(CLIPPED_HINT), "{prose}");
        assert!(matches!(
            screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx),
            ScreenAction::Pop
        ));
        assert!(matches!(
            rx.try_recv().ok(),
            Some(Action::DriverEditRequested { .. })
        ));
    }

    // ── Plan 30-09 Task 3: the form agrees with the launcher and the editor ──

    /// WR-01 and WR-07: a typed model is judged by the rule a recorded model meets, so a
    /// value that looks like a flag never reaches the detached launch's argv, while the
    /// `[1m]` long-context suffix and a provider-qualified id are accepted.
    #[test]
    fn launcher_launch_a_typed_model_that_looks_like_a_flag_is_refused() {
        let (mut screen, mut ctx, _rx) = form_screen(LaunchKind::Launch, LaunchForm::fresh());
        press(&mut screen, &mut ctx, 'e');
        for bad in [
            "-x",
            "--foreground",
            &"a".repeat(65),
            "a b",
            ".opus",
            "-opus[1m]",
        ] {
            screen.editing = Some(bad.to_string());
            screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
            assert_eq!(
                screen.error.as_deref(),
                Some("must be a model name"),
                "{bad:?} is refused in the form"
            );
            assert!(screen.editing.is_some(), "{bad:?}: editing continues");
            assert_eq!(screen.args().model, "opus", "{bad:?} is not stored");
        }
        for good in [
            "opus",
            "sonnet",
            "claude-opus-4-5",
            "opus[1m]",
            "sonnet[1m]",
            "claude-sonnet-4-5[1m]",
            "anthropic.claude-3-5-sonnet-20240620-v1:0",
        ] {
            screen.editing = Some(good.to_string());
            screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
            assert!(screen.editing.is_none(), "{good} is accepted");
            assert_eq!(screen.args().model, good);
            press(&mut screen, &mut ctx, 'e');
        }
        // A 64-character name is the longest.
        screen.editing = Some("a".repeat(64));
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert!(screen.editing.is_none());
    }

    /// IN-03: the form reads `control` with the launcher's reader and blocks exactly
    /// when the launcher would.
    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_launch_preflight_reads_control_like_the_launcher() {
        use crate::launcher::host::HostCaps;
        let ready = HostCaps {
            missing: Vec::new(),
            claude_version: Some((2, 1, 295)),
        };
        let dir = tempfile::tempdir().expect("temp dir");

        // `control: run` under `## Direction`, a stray `control: pause` elsewhere: the
        // launcher accepts this run, so the form must not block it.
        let stray = format!(
            "{}\n## Notes\n\n- control: pause\n",
            crate::driver::steer::tests::clean_body()
        );
        crate::driver::steer::tests::write_run(dir.path(), &stray);
        let pre = launch_preflight(dir.path(), &ready);
        assert_eq!(pre.control.as_deref(), Some("run"), "{pre:?}");
        assert_eq!(control_blocking(&pre), None);

        // `control: pause` under `## Direction` is what the launcher refuses.
        let paused =
            crate::driver::steer::set_control(&crate::driver::steer::tests::clean_body(), "pause")
                .expect("a control write");
        crate::driver::steer::tests::write_run(dir.path(), &paused);
        let pre = launch_preflight(dir.path(), &ready);
        assert_eq!(pre.control.as_deref(), Some("pause"));
        assert_eq!(control_blocking(&pre), Some("pause"));

        // The launcher compares the word exactly: anything but `run` blocks.
        for (word, blocks) in [("run", false), ("Run", true), ("stop", true)] {
            let pre = LaunchPreflight {
                control: Some(word.to_string()),
                ..LaunchPreflight::default()
            };
            assert_eq!(control_blocking(&pre).is_some(), blocks, "{word}");
        }
    }

    /// WR-04: a finding's `DIRECTION.md:<line>:` counts from the first body line, the
    /// numbering the body-only edit temp shows in the editor.
    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_launch_preflight_numbers_lint_lines_like_the_editor() {
        use crate::driver::direction::split_frontmatter;
        use crate::driver_templates::lint::lint_direction;
        use crate::launcher::host::HostCaps;
        let ready = HostCaps {
            missing: Vec::new(),
            claude_version: Some((2, 1, 295)),
        };
        let dir = tempfile::tempdir().expect("temp dir");
        crate::driver::steer::tests::write_run(
            dir.path(),
            crate::driver_templates::DIRECTION_TEMPLATE,
        );
        let text = std::fs::read_to_string(dir.path().join("DIRECTION.md")).unwrap();
        let (frontmatter, body) = split_frontmatter(&text);
        assert!(frontmatter.is_some(), "the run has a tool block");
        let editor: Vec<String> = lint_direction(body)
            .problems
            .iter()
            .map(ToString::to_string)
            .collect();
        assert!(!editor.is_empty());
        assert_ne!(
            lint_direction(&text).problems[0].to_string(),
            editor[0],
            "the whole-file numbering differs by the tool block"
        );
        let pre = launch_preflight(dir.path(), &ready);
        assert_eq!(pre.lint_problems, editor);
        let warnings: Vec<String> = lint_direction(body)
            .warnings
            .iter()
            .map(|f| f.as_warning())
            .collect();
        assert_eq!(pre.lint_warnings, warnings);
    }

    /// The preflight over real files: the same lint and control the launcher applies.
    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_launch_preflight_reads_the_lint_and_the_control_word() {
        use crate::launcher::host::HostCaps;
        let ready = HostCaps {
            missing: Vec::new(),
            claude_version: Some((2, 1, 295)),
        };
        let dir = tempfile::tempdir().expect("temp dir");

        // A clean body: nothing blocks.
        crate::driver::steer::tests::write_run(
            dir.path(),
            &crate::driver::steer::tests::clean_body(),
        );
        let clean = launch_preflight(dir.path(), &ready);
        assert_eq!(clean.missing, Vec::<String>::new());
        assert_eq!(clean.claude_refusal, None);
        assert!(clean.lint_problems.is_empty(), "{clean:?}");
        assert_eq!(clean.control.as_deref(), Some("run"));

        // The template, unedited, has placeholders: problems.
        crate::driver::steer::tests::write_run(
            dir.path(),
            crate::driver_templates::DIRECTION_TEMPLATE,
        );
        let template = launch_preflight(dir.path(), &ready);
        assert!(!template.lint_problems.is_empty(), "{template:?}");
        assert!(template.lint_problems[0].starts_with("DIRECTION.md:"));

        // A paused body.
        let paused =
            crate::driver::steer::set_control(&crate::driver::steer::tests::clean_body(), "pause")
                .expect("a control write");
        crate::driver::steer::tests::write_run(dir.path(), &paused);
        let pre = launch_preflight(dir.path(), &ready);
        assert_eq!(pre.control.as_deref(), Some("pause"));

        // A host missing pieces, and one with an old claude.
        let degraded = launch_preflight(
            dir.path(),
            &HostCaps {
                missing: vec!["bwrap".to_string(), "systemd-run".to_string()],
                claude_version: Some((2, 1, 295)),
            },
        );
        assert_eq!(degraded.missing, ["bwrap", "systemd-run"]);
        assert_eq!(degraded.claude_refusal, None);
        let old = launch_preflight(
            dir.path(),
            &HostCaps {
                missing: Vec::new(),
                claude_version: Some((2, 0, 1)),
            },
        );
        assert!(old.claude_refusal.is_some_and(|d| d.contains("2.0.1")));
        let none = launch_preflight(
            dir.path(),
            &HostCaps {
                missing: Vec::new(),
                claude_version: None,
            },
        );
        assert!(none.claude_refusal.is_some());

        // A run directory with no DIRECTION.md is a problem, not a panic.
        let empty = tempfile::tempdir().expect("temp dir");
        let unreadable = launch_preflight(empty.path(), &ready);
        assert!(unreadable.lint_problems[0].starts_with("DIRECTION.md could not be read"));
    }

    #[test]
    fn launcher_launch_error_copy_escapes_every_variant() {
        let esc = "\u{1b}[31mred\u{1b}]0;title\u{7}";
        let variants = [
            LauncherError::InvalidRun {
                detail: esc.to_string(),
            },
            LauncherError::Locked {
                detail: esc.to_string(),
            },
            LauncherError::Auth {
                detail: esc.to_string(),
            },
            LauncherError::Spawn {
                detail: esc.to_string(),
            },
            LauncherError::Io {
                detail: esc.to_string(),
            },
            LauncherError::Store {
                detail: esc.to_string(),
            },
            LauncherError::UnsupportedPlatform,
            LauncherError::DaemonExited {
                code: 1,
                detail: esc.to_string(),
            },
            LauncherError::DaemonExited {
                code: 3,
                detail: esc.to_string(),
            },
            LauncherError::SandboxUnavailable {
                missing: vec![esc.to_string()],
            },
            LauncherError::ClaudeVersion {
                detail: esc.to_string(),
            },
            LauncherError::CapExhausted {
                detail: esc.to_string(),
            },
            LauncherError::DirectionInvalid {
                problems: vec![esc.to_string()],
            },
        ];
        for variant in variants {
            let copy = launch_error_copy(&format!("r{esc}"), &variant);
            assert!(
                !copy.contains('\u{1b}') && !copy.contains('\u{7}'),
                "{variant:?}: {copy:?}"
            );
            let refusal = copy.starts_with("Launch refused: ");
            assert!(
                refusal
                    || copy.ends_with("Nothing was started.")
                    || copy.contains("Nothing was started."),
                "every failure says nothing was started, or that it was refused: {copy}"
            );
        }
    }

    // ── Task 3: a new run (D-18) ─────────────────────────────────────────────

    fn slug_screen() -> (SlugInputScreen, AppContext, UnboundedReceiver<Action>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let (ctx, rx) = ctx_with_project(dir.path());
        (SlugInputScreen::new(ALIAS.to_string()), ctx, rx)
    }

    fn type_slug(screen: &mut SlugInputScreen, ctx: &mut AppContext, text: &str) {
        for c in text.chars() {
            screen.handle_key(KeyCode::Char(c), KeyModifiers::NONE, ctx);
        }
    }

    #[test]
    fn launcher_new_run_the_slug_makes_a_run_id_or_says_why_not() {
        assert_eq!(
            slug_run_id("tab", "20261020"),
            Ok("20261020-tab".to_string())
        );
        assert_eq!(
            slug_run_id("a.b_c-d", "20261020").unwrap(),
            "20261020-a.b_c-d"
        );
        assert_eq!(slug_run_id("", "20261020"), Err(SlugProblem::Empty));
        for bad in ["-x", ".x", "_x", "a/b", "a b", "a\u{e4}", "x\u{1b}", "a\nb"] {
            assert_eq!(
                slug_run_id(bad, "20261020"),
                Err(SlugProblem::NotUsable),
                "{bad:?}"
            );
        }
        // 9 bytes of `yyyymmdd-` plus the slug: 71 is the longest, 72 is one too many.
        assert!(slug_run_id(&"a".repeat(71), "20261020").is_ok());
        assert_eq!(
            slug_run_id(&"a".repeat(72), "20261020"),
            Err(SlugProblem::TooLong)
        );
    }

    /// The input never offers an id `driver new` would refuse.
    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_new_run_the_preview_agrees_with_the_id_driver_new_mints() {
        use chrono::TimeZone;
        let now = chrono::Utc
            .with_ymd_and_hms(2026, 10, 20, 12, 0, 0)
            .single()
            .expect("an instant");
        let long = "a".repeat(72);
        for slug in [
            "tab",
            "a.b",
            "-x",
            ".x",
            "a/b",
            "",
            "x y",
            &"b".repeat(71),
            long.as_str(),
            "Z9",
        ] {
            assert_eq!(
                slug_run_id(slug, "20261020").ok(),
                crate::driver_templates::mint_run_id(slug, now).ok(),
                "{slug:?}"
            );
        }
    }

    #[test]
    fn launcher_new_run_the_input_previews_the_id_and_shows_the_not_usable_copy() {
        let (mut screen, mut ctx, _rx) = slug_screen();
        let date = chrono::Utc::now().format("%Y%m%d").to_string();
        let text = draw(&screen, &ctx, 100, 20);
        assert!(
            text.contains("New run slug> _   [Enter] create  [Esc] cancel"),
            "{text}"
        );
        assert!(text.contains(&format!("run id: {date}-")), "{text}");
        type_slug(&mut screen, &mut ctx, "tab");
        let text = draw(&screen, &ctx, 100, 20);
        assert!(text.contains("New run slug> tab_"), "{text}");
        assert!(text.contains(&format!("run id: {date}-tab")), "{text}");
        for bad in ["-x", ".x", "a/b"] {
            let (mut screen, mut ctx, _rx) = slug_screen();
            type_slug(&mut screen, &mut ctx, bad);
            let text = draw(&screen, &ctx, 120, 20);
            assert!(
                text.contains(
                    "not usable: use letters, digits, \".\", \"_\" or \"-\", starting with a letter or digit"
                ),
                "{bad}:\n{text}"
            );
        }
        let (mut screen, mut ctx, _rx) = slug_screen();
        type_slug(&mut screen, &mut ctx, &"a".repeat(72));
        let text = draw(&screen, &ctx, 120, 20);
        assert!(text.contains("not usable: the run id"), "{text}");
        assert!(text.contains("may be at most 80 bytes"), "{text}");
    }

    /// UI D8: the not-usable row wraps instead of being cut at the window's edge.
    #[test]
    fn launcher_new_run_the_not_usable_copy_wraps_to_its_last_word() {
        let (mut screen, mut ctx, _rx) = slug_screen();
        type_slug(&mut screen, &mut ctx, "-x");
        let narrow = draw(&screen, &ctx, 80, 24);
        let footer = narrow
            .lines()
            .skip(20)
            .collect::<Vec<_>>()
            .join(" ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            footer.contains(SLUG_NOT_USABLE),
            "the whole copy, to its last word, at 80 columns:\n{narrow}"
        );
        assert!(
            footer.ends_with("starting with a letter or digit"),
            "{footer}"
        );
        // The input row is unchanged: it starts the same and is one row.
        assert!(
            narrow
                .lines()
                .any(|row| row.starts_with("  New run slug> -x_")),
            "{narrow}"
        );

        // A wide terminal keeps the copy on one row.
        let wide = draw(&screen, &ctx, 120, 40);
        assert!(
            wide.lines().any(|row| row.contains(SLUG_NOT_USABLE)),
            "one row at 120 columns:\n{wide}"
        );
    }

    /// UI D4: the left-unlaunched copy fits the 62-cell status room at 80 columns.
    #[test]
    fn launcher_new_run_the_unlaunched_copy_fits_the_status_bar() {
        let copy = left_unlaunched_copy("20261019-0042-abcd");
        assert_eq!(
            copy,
            "20261019-0042-abcd created, not launched: [R] launches it."
        );
        assert!(Span::raw(copy.as_str()).width() <= 62, "{copy}");
    }

    #[test]
    fn launcher_new_run_enter_on_an_empty_or_unusable_slug_does_nothing() {
        for slug in ["", "-x", "a/b"] {
            let (mut screen, mut ctx, mut rx) = slug_screen();
            type_slug(&mut screen, &mut ctx, slug);
            let action = screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
            assert!(matches!(action, ScreenAction::None), "{slug:?}");
            assert!(rx.try_recv().is_err(), "{slug:?} dispatches nothing");
        }
    }

    #[test]
    fn launcher_new_run_enter_dispatches_once_and_a_second_enter_while_pending_is_inert() {
        let (mut screen, mut ctx, mut rx) = slug_screen();
        type_slug(&mut screen, &mut ctx, "tab");
        let action = screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert!(
            matches!(action, ScreenAction::None),
            "the screen stays until the App answers"
        );
        let Some(Action::DriverNewRunRequested { alias, slug }) = rx.try_recv().ok() else {
            panic!("Enter must dispatch the creation");
        };
        assert_eq!((alias.as_str(), slug.as_str()), (ALIAS, "tab"));
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert!(rx.try_recv().is_err(), "pending: a second Enter is inert");
    }

    #[test]
    fn launcher_new_run_an_existing_run_keeps_the_buffer_and_asks_for_another_slug() {
        let (mut screen, mut ctx, mut rx) = slug_screen();
        type_slug(&mut screen, &mut ctx, "tab");
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        let _ = rx.try_recv();
        // The App's answer.
        ctx.view_cache
            .entry(ALIAS.to_string())
            .or_default()
            .new_run_status = Some(NewRunStatus::Exists("20261020-tab".to_string()));
        let text = draw(&screen, &ctx, 100, 20);
        assert!(
            text.contains("New run slug> tab_"),
            "the buffer is kept:\n{text}"
        );
        assert!(
            text.contains("A run named 20261020-tab already exists. Pick another slug."),
            "{text}"
        );
        // Enter on the same slug does nothing; editing clears the row and re-arms Enter.
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert!(rx.try_recv().is_err());
        type_slug(&mut screen, &mut ctx, "2");
        assert!(ctx.view_cache[ALIAS].new_run_status.is_none());
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        let Some(Action::DriverNewRunRequested { slug, .. }) = rx.try_recv().ok() else {
            panic!("another slug can be submitted");
        };
        assert_eq!(slug, "tab2");
    }

    #[test]
    fn launcher_new_run_escape_cancels_and_backspace_edits() {
        let (mut screen, mut ctx, mut rx) = slug_screen();
        type_slug(&mut screen, &mut ctx, "abc");
        screen.handle_key(KeyCode::Backspace, KeyModifiers::NONE, &mut ctx);
        let text = draw(&screen, &ctx, 100, 20);
        assert!(text.contains("New run slug> ab_"), "{text}");
        assert!(matches!(
            screen.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx),
            ScreenAction::Pop
        ));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn launcher_new_run_the_copy_is_the_spec_text() {
        assert_eq!(
            created_copy("20261020-tab"),
            "Created run 20261020-tab. It is checked before launch."
        );
        assert_eq!(
            exists_copy("20261020-tab"),
            "A run named 20261020-tab already exists. Pick another slug."
        );
        assert_eq!(
            left_unlaunched_copy("20261020-tab"),
            "20261020-tab created, not launched: [R] launches it."
        );
        assert_eq!(
            create_failed_copy("disk full"),
            "Could not create the run: disk full"
        );
        assert!(!created_copy("a\u{1b}b").contains('\u{1b}'));
    }

    fn lint_popup(problems: usize) -> (LintPopupScreen, AppContext, UnboundedReceiver<Action>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let (ctx, rx) = ctx_with_project(dir.path());
        (
            LintPopupScreen::new(
                ALIAS.to_string(),
                RUN.to_string(),
                (1..=problems)
                    .map(|n| format!("DIRECTION.md:{n}: unfilled placeholder {{short title}}"))
                    .collect(),
            ),
            ctx,
            rx,
        )
    }

    #[test]
    fn launcher_new_run_the_lint_popup_lists_the_problems_and_blocks_launch() {
        let (screen, ctx, _rx) = lint_popup(10);
        let text = draw(&screen, &ctx, 110, 36);
        assert!(text.contains("Check DIRECTION.md"), "{text}");
        assert!(
            text.contains("driver check found 10 problem(s). Launch is blocked until they are"),
            "{text}"
        );
        assert!(text.contains("fixed:"), "{text}");
        assert!(
            text.contains("DIRECTION.md:1: unfilled placeholder"),
            "{text}"
        );
        assert!(
            text.contains("DIRECTION.md:8: unfilled placeholder"),
            "{text}"
        );
        assert!(!text.contains("DIRECTION.md:9:"), "cut at eight:\n{text}");
        assert!(text.contains("\u{2026} 2 more"), "{text}");
        assert!(text.contains("Edit it again now?"), "{text}");
    }

    #[test]
    fn launcher_new_run_y_edits_again_and_n_leaves_the_run_unlaunched() {
        let (mut screen, mut ctx, mut rx) = lint_popup(2);
        let action = screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);
        assert!(matches!(action, ScreenAction::Pop));
        let Some(Action::DriverEditRequested { alias, run_id }) = rx.try_recv().ok() else {
            panic!("y must re-open the editor");
        };
        assert_eq!((alias.as_str(), run_id.as_str()), (ALIAS, RUN));
        assert!(rx.try_recv().is_err());

        for key in [KeyCode::Char('n'), KeyCode::Esc, KeyCode::Enter] {
            let (mut screen, mut ctx, mut rx) = lint_popup(2);
            let action = screen.handle_key(key, KeyModifiers::NONE, &mut ctx);
            assert!(matches!(action, ScreenAction::Pop), "{key:?}");
            let Some(Action::DriverNewRunLeft { run_id, .. }) = rx.try_recv().ok() else {
                panic!("{key:?} must end the flow");
            };
            assert_eq!(run_id, RUN);
            assert!(
                rx.try_recv().is_err(),
                "{key:?} never opens the editor or the form"
            );
        }
    }
}
