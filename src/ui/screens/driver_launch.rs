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
    click_as_key, dim_background, render_confirm_popup, ConfirmOutcome, ConfirmPopup,
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

/// A model name worth prefilling from a recorded connection: the conservative charset
/// `^[A-Za-z0-9][A-Za-z0-9._:-]{0,63}$` (T-30-17). Anything else is not trusted as an
/// argument the user did not type.
fn recorded_model_ok(model: &str) -> bool {
    let mut chars = model.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && model.chars().count() <= 64
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'))
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
        .filter(|m| recorded_model_ok(m))
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
    /// The live body's `control` word.
    pub control: Option<String>,
    /// `driver check` problems, rendered `DIRECTION.md:<line>: <message>`.
    pub lint_problems: Vec<String>,
    /// `driver check` warnings; they do not block a launch.
    pub lint_warnings: Vec<String>,
}

/// Run the host check's decision and the `driver check` lint for `run_dir`.
///
/// **Blocking: `spawn_blocking` only.** It reads `DIRECTION.md` (under the edit flow's
/// 1 MiB cap); the host probes themselves ran in `caps`, which the caller got from the
/// injectable `detect` seam.
#[cfg(target_os = "linux")]
pub fn launch_preflight(run_dir: &Path, caps: &crate::launcher::host::HostCaps) -> LaunchPreflight {
    use crate::driver::direction::{parse_direction, split_frontmatter};
    use crate::driver_templates::lint::lint_direction;

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
            let report = lint_direction(&text);
            pre.lint_problems = report.problems.iter().map(ToString::to_string).collect();
            pre.lint_warnings = report.warnings.iter().map(|f| f.as_warning()).collect();
            pre.control = parse_direction(split_frontmatter(&text).1).control;
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
            let ok = !text.is_empty()
                && text.chars().count() <= 100
                && !text.chars().any(|c| c.is_control())
                && parse_launch_field(field.flag(), text).is_ok();
            if !ok {
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

    /// Why `y` must launch nothing right now, as the copy to show; `None` when the
    /// launch may go ahead.
    fn blocker(preflight: &LaunchPreflight) -> Option<String> {
        if let Some(detail) = &preflight.claude_refusal {
            return Some(format!("Launch refused: {}", shown(detail)));
        }
        if !preflight.missing.is_empty() {
            return Some(sandbox_refused_copy(&preflight.missing));
        }
        None
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

    /// The body lines of the popup.
    fn body_lines(&self, ctx: &AppContext) -> Vec<Line<'static>> {
        let yellow = Style::default().fg(Color::Yellow);
        let red = Style::default().fg(Color::Red);
        let mut lines: Vec<Line<'static>> = vec![Line::from(format!(
            "Launch {} on \"{}\"?",
            shown(&self.run_id),
            shown(&self.alias)
        ))];
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
                } else {
                    lines.push(Line::from(SANDBOX_OK));
                }
            }
        }
        if let Some(refusal) = &self.refusal {
            lines.push(Line::from(Span::styled(refusal.clone(), red)));
        }
        lines.push(Line::from(Span::styled(
            format!(
                "Each driver session may spend up to {} of Claude usage (claude --max-budget-usd). It runs detached and keeps running after you close this TUI.",
                dollars(self.form.args.max_budget_usd)
            ),
            yellow,
        )));
        lines.push(Line::from(Span::styled(KEY_ROW, muted())));
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
            self.body_lines(ctx),
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
        ] {
            for conn in &mut fm.connected {
                conn.model = Some(model.to_string());
            }
            let form = relaunch_prefill(&fm);
            assert_eq!(form.args.model, "opus", "{model:?} is not trusted");
            assert!(form.defaulted.model);
        }
        for conn in &mut fm.connected {
            conn.model = Some("claude-opus-4-7:1m".to_string());
        }
        assert_eq!(relaunch_prefill(&fm).args.model, "claude-opus-4-7:1m");
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
        assert_eq!(rows[0], format!("Launch {RUN} on \"{ALIAS}\"?"));
        assert_eq!(rows[1], "> model           opus");
        assert_eq!(rows[2], "  loop mode       loop (default)");
        assert_eq!(rows[3], "  spend ceiling   $10.00 (default)");
        assert_eq!(rows[4], "  dispatches      10");
        assert_eq!(rows[5], "  minutes         90");
        assert_eq!(rows[6], "  gap rounds      2 per phase");
        assert_eq!(rows[7], "Sandbox: Loading...");
        assert!(rows.iter().any(|r| r == "[j/k] field  [e] edit field"));
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
        assert!(
            rows[3].contains("must be a positive number"),
            "an invalid value shows the copy and editing continues: {rows:?}"
        );
        assert!(rows[3].contains("0_"), "{rows:?}");
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
        assert_eq!(rows[3], "> spend ceiling   $25.50");
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
        assert_eq!(rows[2], "> loop mode       fresh-per-item");
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
}
