// Error types for gsd-meta-manager.
//
// The rest of the codebase uses `anyhow::Result` + `.with_context()` at every
// seam existing callers touch, and that stays true (see `config.rs`,
// `registry.rs`, `git_ops.rs`). Only the executor's own surface uses the typed
// errors below: a driver UI needs a *state* to render, not a message, and
// PITFALLS Pitfall 10 assigns "widen the error type before the driver" to this
// phase explicitly (D-12).
//
// `Display` and `std::error::Error` are hand-written rather than derived. Three
// small enums do not justify an error-derive dependency, and keeping the Cargo
// surface to exactly the two crates plan 15-01 added (`process-wrap`, `uuid`)
// is a deliberate line, not an oversight.
//
// That narrow dependency surface has exactly one deliberate exception inside
// the crate: `DriveError::PlanApprovalRequired`'s `Display` reaches for
// `crate::ui::screens::sanitize_render_line` (21-08, WR-05). It renders
// model-selected tokens to the operator's terminal, so it needs the same
// "make these bytes safe to paint" rule the TUI applies — and it SHARES that
// rule rather than restating it, because two implementations of that rule are
// two things that can disagree about what a C1 introducer is. `ui` is a
// `pub mod` in `lib.rs`, so this is an in-crate reference and adds no
// dependency.

use std::fmt;
use std::path::PathBuf;

/// Why a run could not be started.
///
/// Every variant is reachable before any turn begins, which is the point: a
/// refusal at this stage costs zero tokens and zero quota (D-06).
#[derive(Debug)]
pub enum SpawnError {
    /// The drivable project's root is not an existing directory. Checked before
    /// the process is launched so a stale registry entry cannot spawn an agent
    /// against a path that no longer exists (T-15-06).
    ProjectRootUnusable {
        /// The root that failed the check.
        root: PathBuf,
    },
    /// The child process could not be launched at all — missing binary, not
    /// executable, permission denied.
    Launch {
        /// The program that was attempted.
        program: String,
        /// The underlying OS error.
        source: std::io::Error,
    },
    /// A stdio pipe was not present after spawn. All three are requested as
    /// pipes, so this means the child was reaped between spawn and take.
    PipeUnavailable {
        /// Which pipe was missing: `stdin`, `stdout` or `stderr`.
        pipe: &'static str,
    },
    /// The child's pid was not readable after spawn, so the process group id
    /// could not be recorded. Without it there is no teardown handle (D-14).
    PidUnavailable,
    /// The first `system/init` failed the capability gate. No user message was
    /// ever written to stdin (D-06, TRANS-04).
    Capability(CapabilityError),
    /// stdout closed before any `system/init` arrived. Either the CLI died
    /// during startup or it is not speaking the `stream-json` protocol at all.
    InitNeverObserved,
    /// The command could not be encoded as an outbound NDJSON user message.
    EncodeCommand {
        /// The underlying serialisation error.
        source: serde_json::Error,
    },
}

impl fmt::Display for SpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectRootUnusable { root } => {
                write!(f, "project root is not a usable directory: {}", root.display())
            }
            Self::Launch { program, source } => {
                write!(f, "failed to launch `{program}`: {source}")
            }
            Self::PipeUnavailable { pipe } => {
                write!(f, "child {pipe} pipe was not available after spawn")
            }
            Self::PidUnavailable => {
                write!(f, "child pid was not readable after spawn, so no process group could be recorded")
            }
            Self::Capability(err) => write!(f, "{err}"),
            Self::InitNeverObserved => write!(
                f,
                "the process stream ended before any system/init event was observed"
            ),
            Self::EncodeCommand { source } => {
                write!(f, "failed to encode the command as a user message: {source}")
            }
        }
    }
}

impl std::error::Error for SpawnError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Launch { source, .. } => Some(source),
            Self::Capability(err) => Some(err),
            Self::EncodeCommand { source } => Some(source),
            _ => None,
        }
    }
}

impl From<CapabilityError> for SpawnError {
    fn from(err: CapabilityError) -> Self {
        Self::Capability(err)
    }
}

/// Why a message could not be delivered to a running run.
#[derive(Debug)]
pub enum SendError {
    /// The run has already ended; there is nothing listening on stdin.
    NotRunning,
    /// The writer task is gone (its receiver was dropped), so stdin is closed.
    WriterGone,
    /// The outbound message could not be encoded as NDJSON.
    Encode(serde_json::Error),
    /// A `control_request` was written but no matching `control_response` ever
    /// arrived — the correlation oneshot was dropped when the run ended.
    ControlResponseLost {
        /// The request id that was never answered.
        request_id: String,
    },
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotRunning => write!(f, "the run is no longer running"),
            Self::WriterGone => write!(f, "the stdin writer task has exited"),
            Self::Encode(err) => write!(f, "failed to encode outbound message: {err}"),
            Self::ControlResponseLost { request_id } => write!(
                f,
                "no control_response ever arrived for request id `{request_id}`"
            ),
        }
    }
}

impl std::error::Error for SendError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Encode(err) => Some(err),
            _ => None,
        }
    }
}

/// Why the observed CLI cannot be driven.
///
/// **Every variant here is a refusal, never a warning.** A refusal that becomes
/// advice is worse than no gate at all, because the user believes they were
/// protected (TRANS-04). Each `Display` names the concrete observed value
/// rather than saying a check "did not match", so the diagnostic is actionable
/// without re-running anything.
///
/// Feature detection is by `capabilities[]` on `system/init`, never by
/// comparing version strings (D-06). The version floor is a separate,
/// narrower claim — see [`CapabilityError::VersionBelowFloor`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityError {
    /// The first `system/init` did not advertise every required capability.
    /// An absent or empty `capabilities` array lands here too — the parser
    /// carries it, the gate is what refuses it (TRANS-04).
    MissingCapabilities {
        /// Required capabilities the CLI did not advertise, in required order.
        missing: Vec<String>,
        /// Everything the CLI did advertise, carried for the diagnostic.
        observed: Vec<String>,
    },
    /// The CLI is older than the supported floor (D-07). This is not feature
    /// detection: it is the one property `capabilities[]` cannot express,
    /// because a CLI that truncates its own terminal envelope cannot advertise
    /// that it does.
    VersionBelowFloor {
        /// The version string the CLI reported, verbatim.
        observed: String,
        /// The floor it failed, rendered.
        floor: String,
    },
    /// The CLI reported no version, or one that is not three numeric
    /// components. Refused rather than assumed new enough: a blanket-renamed
    /// struct silently nulling this field is exactly how a version gate comes
    /// to pass everything (D-07, Pitfall F).
    VersionUnreadable {
        /// What was reported, if anything at all.
        observed: Option<String>,
    },
    /// The auth-source regression guard fired (D-08). Anthropic states the flag
    /// that skips the keychain read will become the default for print mode in a
    /// future release; when that happens this fails loudly at run start instead
    /// of producing a mysteriously context-free agent mid-run. An **absent**
    /// field is a guard failure, not a pass — a future CLI that drops the field
    /// is precisely the silent regression being defended against.
    AuthPathChanged {
        /// The reported source, or `None` if the field was absent entirely.
        observed: Option<String>,
        /// The value that means the subscription/OAuth path is alive.
        expected: &'static str,
    },
}

impl CapabilityError {
    /// The unmet requirements, one per line, for a coarse run-outcome display.
    ///
    /// The typed error is the fidelity-preserving surface and is what
    /// `Executor::start` returns; this is the lossy projection onto the
    /// outcome the TUI renders after the fact.
    pub fn unmet_requirements(&self) -> Vec<String> {
        match self {
            Self::MissingCapabilities { missing, .. } => missing.clone(),
            Self::VersionBelowFloor { observed, floor } => {
                vec![format!("claude_code_version >= {floor} (observed {observed})")]
            }
            Self::VersionUnreadable { observed } => vec![format!(
                "a readable claude_code_version (observed {})",
                render_observed(observed)
            )],
            Self::AuthPathChanged { observed, expected } => vec![format!(
                "apiKeySource == {expected} (observed {})",
                render_observed(observed)
            )],
        }
    }
}

/// Render an optional observation without ever printing a bare `None`.
fn render_observed(observed: &Option<String>) -> String {
    match observed {
        Some(value) => format!("`{value}`"),
        None => "no such field".to_string(),
    }
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCapabilities { missing, observed } => write!(
                f,
                "the claude CLI is missing required capabilities [{}]; it advertises [{}]",
                missing.join(", "),
                observed.join(", ")
            ),
            Self::VersionBelowFloor { observed, floor } => write!(
                f,
                "the claude CLI reports version {observed}, which is below the minimum supported version {floor}"
            ),
            Self::VersionUnreadable { observed } => write!(
                f,
                "the claude CLI did not report a readable version ({}); refusing rather than assuming it is new enough",
                render_observed(observed)
            ),
            Self::AuthPathChanged { observed, expected } => write!(
                f,
                "the claude CLI reports apiKeySource {} rather than `{expected}`, so the subscription auth path is no longer in use",
                render_observed(observed)
            ),
        }
    }
}

impl std::error::Error for CapabilityError {}

/// Why a project may not be driven (D-14, D-16).
///
/// **Every variant here is reachable before any process is launched.** That is
/// what makes CTRL-03's "a non-opted-in project is never spawned against" a
/// structural property of the type system rather than a check that some future
/// call site might skip: the only production constructor of
/// [`DrivableProject`](crate::executor::DrivableProject) returns this error, and
/// [`Executor::start`](crate::executor::Executor::start) accepts nothing else.
///
/// The refusal lives in the **driver process**, not in the TUI, so a user typing
/// `gsd-meta-manager drive foo` by hand is refused by exactly the same code as a
/// TUI-initiated run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptInError {
    /// No registry entry carries this alias. Raised by the caller that performs
    /// the registry lookup, never by the constructor, which is handed an entry.
    UnknownAlias {
        /// The alias that was asked for.
        ///
        /// **Carried verbatim and displayed escaped, and those are two
        /// different questions this type asks separately** (`21-24`, CR-02).
        /// [`crate::text::Untrusted`] holds the bytes the caller supplied with
        /// no judgment applied — a refusal must name what was refused, exactly
        /// — while implementing no [`Display`](std::fmt::Display) at all, so the
        /// `Display` below cannot compile until the value goes through
        /// [`shown`](crate::text::Untrusted::shown).
        alias: crate::text::Untrusted,
    },
    /// The project is registered but carries no `driver_opt_in` record. A
    /// registered project is one the dashboard may *read*; driving it is a
    /// separate, deliberate act (D-14).
    ///
    /// **Its message carries the `o`-key affordance because it is now the ONLY
    /// place this judgment is spelled** (`21-24`, D-21-18).
    /// `ui::screens::driver_confirm::do_start_run` used to hand-write a second
    /// sentence for the same refusal — and put the raw alias into a rendered
    /// error line while doing it. Two sentences for one judgment is the defect
    /// D-19-2 removed from `Alias::new` and WR-03 removed from `advisory.rs`;
    /// the screen delegates here now, so the affordance had to move here with
    /// it or delegation would have cost the user the actionable half. Pinned by
    /// `driver_confirm`'s
    /// `the_delegated_opt_in_refusal_still_names_the_key_to_press`.
    NotOptedIn {
        /// The alias that is registered but not opted in.
        ///
        /// Carried verbatim, displayed escaped — see
        /// [`OptInError::UnknownAlias::alias`](OptInError::UnknownAlias).
        alias: crate::text::Untrusted,
    },
    /// The registered path is not an existing directory. Checked before the
    /// process is launched, mirroring [`SpawnError::ProjectRootUnusable`]'s
    /// reasoning: a stale registry entry must not spawn an agent against a path
    /// that no longer exists.
    RootUnusable {
        /// The alias whose path failed the check.
        ///
        /// Carried verbatim, displayed escaped — see
        /// [`OptInError::UnknownAlias::alias`](OptInError::UnknownAlias).
        alias: crate::text::Untrusted,
        /// The root that failed the check.
        ///
        /// Stays a [`PathBuf`]: it is this build's own canonicalised path, not
        /// a string the caller supplied, and `Path::display` is the sink. The
        /// carrier is for values this build did not author.
        root: PathBuf,
    },
    /// The opt-in record no longer covers the bytes that would reach a prompt.
    ///
    /// **Checked at the gate, not at render time**, which is the whole point:
    /// a confirmation screen left open while a `git pull` rewrites `CLAUDE.md`
    /// must not be able to approve bytes that changed underneath it. The user
    /// approved a specific set of files with specific contents; if either has
    /// moved, the approval no longer describes what would run.
    ///
    /// Not a failure — a re-confirmation. The remedy is to opt in again, having
    /// seen the new disclosure.
    PromptInputsDrifted {
        /// The alias whose disclosed inputs no longer match.
        ///
        /// Carried verbatim, displayed escaped — see
        /// [`OptInError::UnknownAlias::alias`](OptInError::UnknownAlias).
        alias: crate::text::Untrusted,
        /// Which file moved, and how.
        drift: crate::registry::OptInDrift,
    },
}

/// **The ONE producer, so no consumer has to be named** (`21-24`, CR-02).
///
/// The four alias-carrying variants above hold [`crate::text::Untrusted`], which
/// implements no [`Display`](std::fmt::Display) — so this impl does not compile
/// while any of them is interpolated raw, and the four `eprintln!("Error: {}",
/// err)` sites in `src/main.rs` became correct **without appearing in this
/// plan's diff**. That absence is the evidence the fix is at the producer; a
/// list of consumers would have been four this round and five the next.
///
/// The escape is bound ONCE, above the `match`, exactly as
/// `impl Display for crate::registry::AliasRefusal` binds it, so a variant added
/// tomorrow cannot embed its alias without going through it.
impl fmt::Display for OptInError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Bound once, above the match, so a variant added tomorrow cannot embed
        // the alias without going through it. `shown()` is
        // `crate::text::render_for_terminal`: BOTH the invisible-formatting
        // class and the ESC/C0/DEL/C1 control class, composed in one place.
        let escaped = |alias: &crate::text::Untrusted| alias.shown();
        match self {
            Self::UnknownAlias { alias } => {
                let alias = escaped(alias);
                write!(f, "no project is registered under the alias `{alias}`")
            }
            Self::NotOptedIn { alias } => {
                let alias = escaped(alias);
                write!(
                    f,
                    "the project `{alias}` has not opted in to being driven; \
                     registering a project lets the dashboard read it, driving it is a separate \
                     deliberate opt-in. Press `o` on the dashboard to opt it in"
                )
            }
            Self::RootUnusable { alias, root } => {
                let alias = escaped(alias);
                write!(
                    f,
                    "the registered path for `{alias}` is not a usable directory: {}",
                    root.display()
                )
            }
            Self::PromptInputsDrifted { alias, drift } => {
                let alias = escaped(alias);
                write!(
                    f,
                    "the driver opt-in for `{alias}` needs re-confirming: {}. \
                     Opt in again to review what reaches a prompt and approve it",
                    drift.describe()
                )
            }
        }
    }
}

impl std::error::Error for OptInError {}

/// Why the single-execution lock could not be taken (D-19, D-20).
///
/// **The lock itself is `flock(2)` on a held descriptor; these variants only
/// describe the refusal.** The kernel is the enforcement — a variant here is
/// what the *user* is told, and criterion #5 of this phase is entirely about
/// that message: "a second attempt reports **which run** holds the lock instead
/// of starting a second one". A refusal that says only "locked" has not
/// delivered it, which is why [`LockError::HeldBy`] carries all three fields of
/// the holder's record and renders every one of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    /// Another live run holds the lock, and its own record was readable.
    ///
    /// The three fields are read out of the lock file, which the holder wrote
    /// **after** winning. They are advisory metadata: a hand-edited record can
    /// change this message but cannot grant a second lock, because the kernel's
    /// `flock` and not the file's contents is what refuses (T-17-08).
    HeldBy {
        /// The holding run's id, as the holder recorded it.
        run_id: String,
        /// The holding driver's process group id — the teardown handle.
        pgid: u32,
        /// When the holder acquired the lock, RFC3339.
        started_at: String,
    },
    /// The lock is **held**, but the holder's record was absent, empty, short or
    /// unparseable.
    ///
    /// **D-20.3's rule in full: a partial or empty read is reported as held by
    /// an unknown run, never as not held.** The holder writes its record in a
    /// second step after winning the lock, so there is a real window in which
    /// the lock is taken and the file is still empty. Reading that window as
    /// "not held" would convert a lost race into a second concurrent run, which
    /// is the exact outcome CTRL-05 exists to prevent. Losing the identity of
    /// the holder costs a worse message; losing the refusal costs the invariant.
    HeldByUnknownRun,
    /// The lock file could not be opened, or the runs root could not be created.
    ///
    /// A real I/O fault — a read-only filesystem, a permissions problem, a
    /// vanished path — and deliberately distinct from contention, because the
    /// two call for opposite responses from the user.
    Unavailable {
        /// The underlying failure, rendered.
        detail: String,
    },
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeldBy {
                run_id,
                pgid,
                started_at,
            } => write!(
                f,
                "another run is already driving this project: run `{run_id}` \
                 (process group {pgid}, started {started_at}). \
                 Only one driver may run against a project at a time (CTRL-05)"
            ),
            Self::HeldByUnknownRun => write!(
                f,
                "another run is already driving this project, but its lock record \
                 could not be read, so the holding run id is unknown. \
                 The lock is held; this is not a reason to start a second run (CTRL-05)"
            ),
            Self::Unavailable { detail } => {
                write!(f, "the run lock could not be taken: {detail}")
            }
        }
    }
}

impl std::error::Error for LockError {}

/// Why a `drive` invocation ended without a run.
///
/// Later plans in this phase widen this enum, and each addition is a variant
/// rather than a signature change: plan 17-02 added [`DriveError::Lock`], plan
/// 17-04 *removed* a placeholder rather than replacing it (see below), and plan
/// 17-05 adds a concurrency-cap variant. It is deliberately **not** marked
/// `#[non_exhaustive]` — this crate is the only consumer, and an attribute would
/// buy nothing but a `_` arm at every match.
///
/// **Plan 17-06 added no variant, and the absence is deliberate rather than an
/// omission.** A *stopped* run is a normal terminal outcome, not an error: the
/// driver's terminate handler tears the agent's group down, journals the reason,
/// writes its terminal record with the killed outcome and returns `Ok(())`. A
/// stop that surfaced as a `DriveError` would report the user's own deliberate
/// action back to them as a failure, and would make a stop indistinguishable
/// from a crash in every place this enum is rendered (D-06.3).
///
/// **Plan 17-08 added exactly one variant, [`DriveError::RunIdRequired`], and
/// deliberately did not add a second.** The other refusal it introduces — a real
/// run on a platform whose liveness cannot be determined — reuses
/// [`DriveError::UnsupportedPlatform`] with a second, narrower detail. That
/// variant already means "a facility this needs is missing here, and it is a
/// recorded limitation rather than a defect", which is exactly what an absent
/// `/proc` is; a variant of its own would widen an enum four call sites match on
/// in order to say the same sentence twice.
#[derive(Debug)]
pub enum DriveError {
    /// Driving is not supported on this platform (D-05).
    UnsupportedPlatform {
        /// Which facility is missing, named concretely.
        detail: String,
    },
    /// A **real** run was asked for with no run id (CR-04).
    ///
    /// The run id is the only thing that makes a driver findable. It reaches
    /// `/proc/<pid>/cmdline` through the driver's own argv, and
    /// `driver::liveness::probe` matches on it — so a run whose id is not on the
    /// argv is invisible to every consumer of that module at once, and the three
    /// consequences are three separate CTRL failures:
    ///
    /// * `reconcile_one` finds a live pid whose cmdline does not name the run and
    ///   reports a perfectly healthy run as **crashed** (CTRL-04);
    /// * `kill::stop_run` takes the already-gone branch and returns *"already
    ///   finished; nothing was signalled"* **before sending any signal**, while
    ///   the agent runs on with git and push rights (CTRL-01);
    /// * `admit` under-counts the live runs, so the concurrency cap can be
    ///   exceeded and one user's quota burned N times over (CTRL-05).
    ///
    /// The driver used to generate an id when none arrived, which produced
    /// exactly that state and looked like success. It is refused **before**
    /// anything is created, so a refused run leaves nothing on disk to clean up.
    ///
    /// `--dry-run` is unaffected: a preview creates no run to identify (D-22,
    /// D-24).
    RunIdRequired,
    /// The run id is present but is not a single plain path component (D-27,
    /// WR-02).
    ///
    /// **The sibling of [`DriveError::RunIdRequired`], and it exists so the
    /// common case fails loudly instead of quietly yielding `None`.**
    /// `journal::run_paths` already refuses a traversing id, which is what makes
    /// the hole unreachable — but a refusal that only manifests as an absent
    /// `RunPaths` deep inside the journal would surface to the operator as a run
    /// that simply did nothing. `--run-id '../../../../escaped'` was
    /// *reproduced* against the shipped tree: it created `run.json` and
    /// `journal.jsonl` outside the project, in a directory with no `.gitignore`,
    /// and exited 0.
    ///
    /// It carries the offending id because the caller passed it and can only act
    /// on the message if it names what was wrong. That is the one place the
    /// untrusted value is echoed, and it is echoed to the operator's own
    /// terminal rather than into a log, a journal record or a render surface.
    RunIdInvalid {
        /// The id that was refused.
        ///
        /// **Carried verbatim and displayed escaped, and those are two
        /// different questions this type asks separately** (`21-24`).
        /// [`crate::text::Untrusted`] holds the argv bytes with no judgment
        /// applied — a refusal must name what was refused, exactly — while
        /// implementing no [`Display`](std::fmt::Display), so the `Display`
        /// below cannot compile until the value goes through
        /// [`shown`](crate::text::Untrusted::shown).
        run_id: crate::text::Untrusted,
    },
    /// `--alias` is present but carries no visible name (D-17-4).
    ///
    /// **This variant exists because the borrowed one asserted a falsehood**
    /// (WR-06). `from_argv` used to map a blank `--alias` onto
    /// [`OptInError::UnknownAlias`], whose message reads "no project is
    /// registered under the alias `…`" — and pass 6 measured that an invisible
    /// alias *can* be registered by an older build, so the sentence was a claim
    /// about durable state that the refusal had not checked and could not know.
    /// A refusal about a value's shape must not narrate the registry's contents.
    ///
    /// The boundary, the purity and the ordering are unchanged: this fires at
    /// the same seam, before any file is created. Only the claim is corrected.
    AliasNotVisible {
        /// The value that was refused.
        ///
        /// Carried verbatim, displayed escaped — see
        /// [`DriveError::RunIdInvalid::run_id`](DriveError::RunIdInvalid).
        alias: crate::text::Untrusted,
    },
    /// None of `--command`, `--target-phase` or `--goal` names anything to do
    /// (CTRL-06).
    ///
    /// A run with no command source has nothing to do. Until Phase 20 clap made
    /// this unrepresentable by requiring `--command`; the moment a second source
    /// existed, "exactly one of these two" stopped being something a parser can
    /// express and became a seam refusal — the same move, and for the same
    /// reason, as the `run_id` pair above.
    ///
    /// **This doc named only two sources and meant only absence; both halves
    /// were false and the correction rides the commit that fixed them** (IN-01).
    /// `--goal` became a third source in 21-07, and since 21-13 a source that is
    /// *present but carries no visible instruction* resolves here too: a value
    /// made entirely of whitespace, control characters or zero-width/format
    /// characters is nothing to execute, renders as visually empty, and reads as
    /// **field absent** in `run.json` on the tolerant read path (D-30) — so a
    /// run started on one leaves a record that cannot be evidence of what ran.
    /// The three degenerate arms were closed one per cycle (`Goal` 21-07,
    /// `Command` 21-11, `Routed` 21-13) before the payload type made a blank
    /// value unrepresentable.
    NoCommandSource,
    /// Both `--command` and `--target-phase` were supplied (CTRL-06).
    ///
    /// **Refused rather than resolved by precedence, and the direction is the
    /// decision.** One of the two would have to win silently, and whichever it
    /// was, the run's terminal record would name a mode the caller did not
    /// choose while the other mode's bounds went unenforced. An unattended run
    /// whose stopping conditions are not the ones the operator asked for is the
    /// failure CTRL-06 exists to prevent.
    AmbiguousCommandSource,
    /// `--target-phase` is present but is not a single plain path component
    /// (D-27, T-20-03).
    ///
    /// The sibling of [`DriveError::RunIdInvalid`], refused at the same seam and
    /// for the same reason. The value is used only as a map key into the project
    /// state today and the iteration loop composes no path from it — but that is
    /// a fact about today rather than a property of the type, and
    /// `--run-id '../../../../escaped'` is what that distinction cost the last
    /// time it was left to the callers.
    TargetPhaseInvalid {
        /// The value that was refused.
        ///
        /// Carried verbatim, displayed escaped — see
        /// [`DriveError::RunIdInvalid::run_id`](DriveError::RunIdInvalid).
        target_phase: crate::text::Untrusted,
    },
    /// A run bound was asked for that a run cannot be bounded by (CTRL-06).
    ///
    /// Raised **before anything is created**, so a run asked for with a cap that
    /// switches off a detector leaves nothing on disk at all. The taxonomy is
    /// [`crate::driver::bounds::BoundsRefusal`] and this variant carries it
    /// rather than restating it, so one list answers "which caps are refusable".
    BoundsRefused(crate::driver::bounds::BoundsRefusal),
    /// An escalation cap was asked for that could never bind (DRIVE-04).
    ///
    /// The sibling of [`DriveError::BoundsRefused`], raised at the same seam, on
    /// the adjacent line, and **before anything is created** for the same reason:
    /// a run asked for with a model-consultation budget that can never fire
    /// leaves nothing on disk at all.
    ///
    /// The taxonomy is [`crate::driver::escalate::EscalationRefusal`] and this
    /// variant carries it rather than restating it, so one list answers "which
    /// escalation caps are refusable" — exactly as `BoundsRefusal` does for the
    /// run bounds.
    EscalationRefused(crate::driver::escalate::EscalationRefusal),
    /// The stated goal could not be reduced to a machine-checkable plan
    /// (DRIVE-03, CONTEXT.md OQ6).
    ///
    /// **A refusal, never a repair.** The decomposition happens once, above the
    /// run, and a plan with one unreducible step is not a plan with that step
    /// dropped: repairing model output would make this driver a second producer
    /// of plans, and the user would then be running something no model proposed
    /// and no human wrote.
    ///
    /// The taxonomy is [`crate::driver::goal::GoalRefusal`] and this variant
    /// carries it rather than restating it, exactly as [`Self::BoundsRefused`]
    /// carries its own — so one list answers "why can a goal be refused", and
    /// the refusal names the part that could not be reduced.
    GoalRefused(crate::driver::goal::GoalRefusal),
    /// The goal-decomposition seam produced nothing this driver can act on, or
    /// the run's model-consultation budget was already spent (DRIVE-04).
    ///
    /// **It parks the run before it exists rather than retrying with a stricter
    /// prompt.** Retrying a model that has just produced an unusable answer is
    /// how a bounded seam becomes an unbounded one, and the CLI already retries
    /// its own structured-output validation internally — a count plan 21-01
    /// pinned by measurement rather than by assumption.
    ///
    /// The reason is [`crate::driver::escalate::EscalationReason`], the fifth
    /// sibling taxonomy, so the string a reader greps for here is the same one
    /// a mid-run escalation park writes. No second string source.
    GoalSeamUnusable {
        /// The taxonomy member, from the escalation reason set.
        reason: crate::driver::escalate::EscalationReason,
        /// What was observed, already bounded and control-character-stripped.
        detail: String,
    },
    /// A goal-driven run supplied no plan approval (DRIVE-01, research Q4).
    ///
    /// **Absence of a recorded approval is a refusal, never a default yes.** It
    /// is its own variant rather than a mismatch so the message can say *absent*
    /// — reporting an unapproved run as a stale approval would tell the user to
    /// look for a change that never happened.
    ///
    /// **This refusal IS the review surface**, and that is why it carries the
    /// plan as well as the digest. DRIVE-03 requires the user to review the plan
    /// before it runs, and a refusal that named only a digest would be asking
    /// them to approve an opaque string — which is consent in form and not in
    /// substance. `--dry-run` deliberately decomposes nothing (a preview spawns
    /// no process, D-23), so it is not and cannot be the place the plan is
    /// shown.
    ///
    /// A refusal a caller cannot act on is a bug report rather than an error
    /// message; this one shows what would run and the exact flag that authorises
    /// it.
    PlanApprovalRequired {
        /// The approval token for the plan just decomposed and the files as
        /// they stand: **both halves**, joined by
        /// [`crate::journal::APPROVAL_TOKEN_SEPARATOR`] and rendered by
        /// [`crate::journal::render_approval_token`].
        ///
        /// It carries the plan half as well as the file half because the plan
        /// half is what the re-check has to compare *against*. A token naming
        /// only the file half leaves the re-check re-deriving the approved plan
        /// digest from the plan under test, which is a value against itself.
        token: String,
        /// The plan's steps, as the typed `key=value` tokens the run record
        /// carries. Never a command line and never the model's prose.
        steps: Vec<String>,
    },
    /// The `--approved-plan` value could not be read as a token at all.
    ///
    /// **Separate from [`Self::PlanApprovalStale`] and from
    /// `ApprovalRefusal::Absent` on purpose.** "Nobody approved this", "what was
    /// approved has changed" and "what you passed is not a token" are three
    /// different statements to the person reading the refusal, and only the last
    /// one is answered by re-transcribing a value. An approval that cannot be
    /// parsed is an absent approval — never a partial one.
    ///
    /// **Raised in [`crate::driver::drive`]'s invocation-shape group, above the
    /// dry-run branch and above the decomposition seam**, and the position is
    /// what the variant is worth. The parse is pure — it opens no file and starts
    /// no process — so a typo costs **zero** process spawns and zero model
    /// consultations, and a preview refuses exactly what the real run would.
    /// Both halves of that sentence were false until 21-11: the parse lived
    /// inside `approve_plan`, which `drive` reached only *after*
    /// `GoalDecomposition::decompose` had spawned into the driven repository and
    /// spent a consultation out of the run's budget, and `--dry-run` returned
    /// above that whole region so a preview never performed the parse at all
    /// (review-CR-01). A preview that refuses less than the run it previews is
    /// previewing something the user cannot run (WR-09).
    PlanApprovalMalformed(crate::journal::ApprovalTokenError),
    /// A recorded approval no longer covers what would run (research Q4).
    ///
    /// The taxonomy is [`crate::journal::ApprovalRefusal`] and this variant
    /// carries it rather than restating it, exactly as [`Self::BoundsRefused`]
    /// carries its own — so *which half* moved, the plan or the disclosed files,
    /// stays readable without a second error type.
    PlanApprovalStale(crate::journal::ApprovalRefusal),
    /// The opt-in gate refused before anything was spawned.
    OptIn(OptInError),
    // `DryRunUnavailable` lived here between plans 17-01 and 17-04. It said
    // "--dry-run is not implemented yet", which stopped being true the moment
    // `driver::dry_run` landed; a variant that can never be constructed and
    // whose text is false is worse than no variant. A preview is now a success
    // path — it renders three sections to stdout and returns `Ok(())` (D-22).
    /// Another run already holds this project's single-execution lock, or the
    /// lock could not be taken at all (CTRL-05, D-19, D-20).
    ///
    /// Raised **before** the journal starts, so a losing attempt prunes nothing,
    /// creates no run directory, writes no `run.json` and clears nobody's
    /// `active` pointer — D-12's evidence-preservation argument applies to a
    /// loser with exactly as much force as to a crash.
    Lock(LockError),
    /// The agent process could not be started.
    Spawn(SpawnError),
    /// The journal could not be started, written or closed. Carries an `anyhow`
    /// chain rendered at the boundary, because the journal's own surface is
    /// `anyhow` by design and a driver UI needs a state, not a chain.
    Journal {
        /// The rendered failure.
        detail: String,
    },
    /// The safety envelope could not be established, so the run is refused
    /// (D-24, SAFE-02).
    ///
    /// **A partial envelope is the failure this variant exists to prevent.**
    /// The envelope is four independent writes — the hook stubs, the generated
    /// git config, the settings file and the ignore block — and a run that
    /// started with three of them would be a run with one enforcement layer
    /// quietly absent and nothing at all saying so. That is precisely the
    /// silently-disarmed control D-06's layering is written against, so any
    /// establishment failure refuses instead of degrading.
    ///
    /// It carries the [`ParkReason`](crate::envelope::policy::ParkReason)
    /// rather than a fresh string, so `grep envelope_assertion_failed` finds
    /// this refusal alongside every other producer of that reason.
    EnvelopeAssertionFailed {
        /// D-24's taxonomy member this refusal parks under.
        reason: crate::envelope::policy::ParkReason,
        /// The rendered failure, already passed through
        /// [`crate::journal::redact`].
        detail: String,
    },
}

/// **The ONE producer for this type's three argv-carrying variants** (`21-24`).
///
/// Same mechanism as [`impl Display for OptInError`](OptInError), and the escape
/// is bound once above the `match` for the same reason.
///
/// **What the retype closed here that was NOT an invisible character reaching a
/// terminal.** These three sites interpolated `{run_id:?}` / `{alias:?}` /
/// `{target_phase:?}`, and `str`'s own `Debug` already escaped the class it was
/// handed. The defect was the PROVENANCE: the guarantee rested on
/// `core::char::is_printable`, a standard-library table that no test in this tree
/// pins, no doc in this tree names, that moves with a toolchain upgrade, and that
/// is a SECOND spelling of a class this project derives for itself in
/// [`crate::text`]. `src/registry.rs:65-101` recorded that finding in round 8 and
/// closed it for `AliasRefusal` only. It is closed here now: the escape is this
/// project's own predicate, in this project's own notation (`U+202E`, not
/// `\u{202e}`), and `{:?}` then adds nothing because the escaped form is ASCII.
impl fmt::Display for DriveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Bound once, above the match, so a variant added tomorrow cannot embed
        // an argv value without going through it. It yields a `String` rather
        // than a `Rendered` because the three sites below quote with `{:?}` and
        // want `str`'s quoting around the ALREADY-escaped text — the same shape
        // `impl Display for crate::registry::AliasRefusal` uses.
        let escaped = |value: &crate::text::Untrusted| value.shown().to_string();
        match self {
            Self::UnsupportedPlatform { detail } => write!(
                f,
                "driving is not supported on this platform: {detail}. \
                 This is a recorded accepted limitation in REQUIREMENTS, not a defect"
            ),
            // The flag is named explicitly: a refusal a caller cannot act on is
            // a bug report rather than an error message.
            Self::RunIdRequired => write!(
                f,
                "a real run needs a `--run-id`, which is what makes it findable, \
                 stoppable and countable afterwards. Pass one, or use `--dry-run`, \
                 which creates no run to identify"
            ),
            Self::RunIdInvalid { run_id } => {
                let run_id = escaped(run_id);
                write!(
                    f,
                    "the run id {run_id:?} is not a single directory name, so it is refused \
                     before anything is created. A run id names one directory under \
                     .planning/meta-manager/runs/; it may not contain a path separator, \
                     `..`, or a leading `/`"
                )
            }
            // True whether or not an invisible alias is sitting in someone's
            // config.json — which is exactly what the borrowed `UnknownAlias`
            // message was not (WR-06).
            Self::AliasNotVisible { alias } => {
                let alias = escaped(alias);
                write!(
                    f,
                    "the supplied alias {alias:?} carries no visible name, so it \
                     cannot identify any project. Pass the alias as it appears in \
                     `gsd-meta-manager list`"
                )
            }
            Self::NoCommandSource => write!(
                f,
                "a run needs something to do: pass `--command <c>` to run one GSD \
                 command, `--target-phase <N>` to let the decision router choose \
                 each command from observed project state, or `--goal <text>` to \
                 state the objective in plain language. A value made only of \
                 whitespace or invisible characters counts as absent — it names \
                 nothing to run, and recorded it would read as a missing field"
            ),
            Self::AmbiguousCommandSource => write!(
                f,
                "`--command` and `--target-phase` are mutually exclusive — the first \
                 runs one supplied command, the second runs a routed sequence under \
                 the run bounds. Pass exactly one, so the run's terminal record names \
                 the mode you chose"
            ),
            Self::TargetPhaseInvalid { target_phase } => {
                let target_phase = escaped(target_phase);
                write!(
                    f,
                    "the target phase {target_phase:?} is not a single plain path \
                     component, so it is refused before anything is created. A target \
                     phase is a phase number such as `20`; it may not contain a path \
                     separator, `..`, or a leading `/`"
                )
            }
            Self::BoundsRefused(refusal) => write!(f, "{refusal}"),
            Self::EscalationRefused(refusal) => write!(f, "{refusal}"),
            // The refusal's own `Display` already names the reason and quotes
            // the offending value; the sentence around it is what tells the
            // caller the run never started and what they can do about it.
            Self::GoalRefused(refusal) => write!(
                f,
                "the stated goal could not be reduced to a machine-checkable plan, \
                 so the run was refused before anything was created ({refusal}). \
                 Restate the goal in terms of a phase reaching verified, or pass \
                 `--target-phase <N>` directly"
            ),
            Self::GoalSeamUnusable { reason, detail } => write!(
                f,
                "the goal-decomposition seam produced nothing usable, so the run \
                 was refused before anything was created (reason: {}): {detail}. \
                 It is not retried with a stricter prompt — a retry is how a \
                 bounded seam becomes an unbounded one",
                reason.as_str()
            ),
            Self::PlanApprovalRequired { token, steps } => write!(
                f,
                "this run states a goal but records no approval for the plan it \
                 was decomposed into, and approval is an explicit act rather than \
                 something inferred from silence. The plan is:\n{}\n\nIf that is \
                 what you want run, re-run with `--approved-plan {token}`, which \
                 binds the approval to this plan AND to the disclosed files whose \
                 bytes reach a prompt. Both are re-checked at spawn",
                steps
                    .iter()
                    .enumerate()
                    // Each step is built from a model-selected `target_phase`
                    // that consumed third-party repository content, and this
                    // string goes straight to the operator's stdout at the
                    // moment they are deciding whether to approve — the single
                    // best moment to hand someone a terminal-repaint capability.
                    //
                    // This is the SECOND of two layers, not the only one:
                    // `driver::goal::legality` bounds the token at construction
                    // and refuses one that bounding would alter. Neither is
                    // redundant. The bound is what keeps the value that reaches
                    // `run.json` and the router clean; this is what keeps the
                    // *rendering* safe, including for step strings a future
                    // caller composes from somewhere else.
                    .map(|(index, step)| {
                        format!(
                            "  {}. {}",
                            index + 1,
                            crate::ui::screens::sanitize_render_line(step)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            ),
            // Delegated rather than wrapped in a sentence, exactly as
            // `BoundsRefused`, `EscalationRefused` and `PlanApprovalStale` are:
            // the taxonomy's own message already names the flag and the action.
            Self::PlanApprovalMalformed(err) => write!(f, "{err}"),
            Self::PlanApprovalStale(refusal) => write!(f, "{refusal}"),
            Self::OptIn(err) => write!(f, "{err}"),
            Self::Lock(err) => write!(f, "{err}"),
            Self::Spawn(err) => write!(f, "{err}"),
            Self::Journal { detail } => write!(f, "the run journal failed: {detail}"),
            Self::EnvelopeAssertionFailed { reason, detail } => write!(
                f,
                "the safety envelope could not be established, so this run is refused \
                 before anything is created (reason: {}): {detail}",
                reason.as_str()
            ),
        }
    }
}

impl std::error::Error for DriveError {
    // Exhaustive rather than `_ => None`, since plan 17-08. The wildcard meant a
    // variant added later that DID wrap an error would silently lose its source
    // — the chain would simply stop, and nothing would say so. Spelling every
    // arm out makes the next author decide, which is the only moment the
    // decision is cheap.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::OptIn(err) => Some(err),
            Self::Lock(err) => Some(err),
            Self::Spawn(err) => Some(err),
            // No source: these carry their whole story in their own text.
            Self::UnsupportedPlatform { .. }
            | Self::RunIdRequired
            | Self::RunIdInvalid { .. }
            | Self::AliasNotVisible { .. }
            | Self::NoCommandSource
            | Self::AmbiguousCommandSource
            | Self::TargetPhaseInvalid { .. }
            // `BoundsRefusal` is a plain data enum rather than an `Error`: it is
            // a classification of an argv value, not a failure that wrapped one.
            // `EscalationRefusal` is the same shape for the same reason.
            | Self::BoundsRefused(_)
            | Self::EscalationRefused(_)
            // `GoalRefusal` is the same shape again: a classification of a
            // payload, not a failure that wrapped an error.
            | Self::GoalRefused(_)
            | Self::GoalSeamUnusable { .. }
            // `ApprovalRefusal` is the same shape once more: a classification of
            // two digests, not a failure that wrapped an error.
            // `ApprovalTokenError` likewise classifies an argv value's shape.
            | Self::PlanApprovalRequired { .. }
            | Self::PlanApprovalMalformed(_)
            | Self::PlanApprovalStale(_)
            | Self::Journal { .. }
            | Self::EnvelopeAssertionFailed { .. } => None,
        }
    }
}

impl From<crate::driver::bounds::BoundsRefusal> for DriveError {
    fn from(refusal: crate::driver::bounds::BoundsRefusal) -> Self {
        Self::BoundsRefused(refusal)
    }
}

impl From<crate::driver::escalate::EscalationRefusal> for DriveError {
    fn from(refusal: crate::driver::escalate::EscalationRefusal) -> Self {
        Self::EscalationRefused(refusal)
    }
}

impl From<crate::driver::goal::GoalRefusal> for DriveError {
    fn from(refusal: crate::driver::goal::GoalRefusal) -> Self {
        Self::GoalRefused(refusal)
    }
}

impl From<OptInError> for DriveError {
    fn from(err: OptInError) -> Self {
        Self::OptIn(err)
    }
}

impl From<LockError> for DriveError {
    fn from(err: LockError) -> Self {
        Self::Lock(err)
    }
}

impl From<SpawnError> for DriveError {
    fn from(err: SpawnError) -> Self {
        Self::Spawn(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::is_invisible_formatting_char;

    /// The ONE place this module builds an alias-carrying field, so the retype
    /// costs one line here rather than one line per subject.
    ///
    /// Before `21-24` this returned `String`; it now returns
    /// [`crate::text::Untrusted`], and **not one assertion below changed**. That
    /// is the point of routing every construction through a single helper: the
    /// diff of this module is the helper's return type, so a reader can see that
    /// no expectation was weakened to make the retype pass.
    fn carrier(raw: &str) -> crate::text::Untrusted {
        crate::text::Untrusted::from_untrusted_source(raw.to_string())
    }

    /// Every alias-carrying variant of BOTH error types, over one value, as
    /// `(name, Display, Debug)`.
    ///
    /// Both routes are returned together because they are two different escapes
    /// of the same field and this phase has already shipped a fix to one of them
    /// while the other stayed open (`WR-04`, `LegacyRegistryKey`'s derived
    /// `Debug`). A subject list that returned only `to_string()` would be the
    /// same defect in a new place.
    fn alias_carrying_messages(value: &str) -> Vec<(&'static str, String, String)> {
        let opt_in: Vec<(&'static str, OptInError)> = vec![
            (
                "OptInError::UnknownAlias",
                OptInError::UnknownAlias {
                    alias: carrier(value),
                },
            ),
            (
                "OptInError::NotOptedIn",
                OptInError::NotOptedIn {
                    alias: carrier(value),
                },
            ),
            (
                "OptInError::RootUnusable",
                OptInError::RootUnusable {
                    alias: carrier(value),
                    root: PathBuf::from("/nonexistent"),
                },
            ),
            (
                "OptInError::PromptInputsDrifted",
                OptInError::PromptInputsDrifted {
                    alias: carrier(value),
                    drift: crate::registry::OptInDrift::NothingDisclosed,
                },
            ),
        ];
        let drive: Vec<(&'static str, DriveError)> = vec![
            (
                "DriveError::AliasNotVisible",
                DriveError::AliasNotVisible {
                    alias: carrier(value),
                },
            ),
            (
                "DriveError::RunIdInvalid",
                DriveError::RunIdInvalid {
                    run_id: carrier(value),
                },
            ),
            (
                "DriveError::TargetPhaseInvalid",
                DriveError::TargetPhaseInvalid {
                    target_phase: carrier(value),
                },
            ),
        ];

        opt_in
            .into_iter()
            .map(|(name, err)| (name, err.to_string(), format!("{err:?}")))
            .chain(
                drive
                    .into_iter()
                    .map(|(name, err)| (name, err.to_string(), format!("{err:?}"))),
            )
            .collect()
    }

    /// **CR-02, closed at the producer.** Every alias-carrying variant of
    /// [`OptInError`] and [`DriveError`] escapes the value it names, and still
    /// names it.
    ///
    /// Measured at the built binary before the retype, `drive` on a hostile
    /// argv alias printed to a terminal:
    ///
    /// ```text
    /// Error: no project is registered under the alias `no^[[31msuchM-bM-^@M-.x`
    /// ```
    ///
    /// — a live ANSI introducer and a raw `U+202E` from a value the caller
    /// typed, while `add` on the same value printed it escaped one arm away.
    ///
    /// **Three assertions, in this order, and the order is the design.**
    ///
    /// 1. *Non-vacuity.* The hostile message must differ from the clean one.
    ///    Without it, a variant whose message stopped naming the alias at all
    ///    would satisfy assertion 3 by silence — which is exactly how six rounds
    ///    of this phase's fixtures passed while covering nothing.
    /// 2. *Arrival.* The clean member appears verbatim in the message. A refusal
    ///    that does not name what was refused is not actionable, and this is the
    ///    assertion that goes red if a future edit "fixes" the escape by
    ///    dropping the value.
    /// 3. *The property.* Zero characters satisfying
    ///    [`crate::text::is_invisible_formatting_char`].
    ///
    /// The `ESC`/C0/C1 half of the class is deliberately NOT asserted here: it
    /// belongs to `21-24`'s one shared control,
    /// `registry::tests::a_refusal_never_carries_a_raw_control_or_invisible_character`,
    /// which drives both enums AND both halves of `remove` over the same corpus.
    /// Two spellings of one property is the defect this plan exists to remove.
    #[test]
    fn every_alias_carrying_variant_escapes_the_value_it_names() {
        for (clean, hostile) in crate::test_support::LOOK_ALIKE_PAIRS {
            let clean_messages = alias_carrying_messages(clean);
            let hostile_messages = alias_carrying_messages(hostile);
            assert_eq!(
                clean_messages.len(),
                hostile_messages.len(),
                "the subject list must be the same for both members of a pair"
            );

            for ((name, clean_display, _), (_, hostile_display, _)) in
                clean_messages.iter().zip(hostile_messages.iter())
            {
                // 1. Non-vacuity.
                assert_ne!(
                    clean_display, hostile_display,
                    "{name}'s message is identical for {clean:?} and {hostile:?}, \
                     so it either does not name the alias or does not escape it. \
                     Either way every assertion below this one is about a message \
                     that could not have failed"
                );

                // 2. Arrival.
                assert!(
                    clean_display.contains(clean),
                    "{name}'s message does not name the clean alias {clean:?} at \
                     all: {clean_display:?}. A refusal that drops the value it \
                     refused is not actionable, and it would pass assertion 3 by \
                     silence"
                );

                // 3. The property.
                let survivors: Vec<char> = hostile_display
                    .chars()
                    .filter(|c| is_invisible_formatting_char(*c))
                    .collect();
                assert!(
                    survivors.is_empty(),
                    "{name}'s message carries {survivors:?} — characters that \
                     render as nothing — from the alias {hostile:?}. What the \
                     operator reads is not what the value is, which is Trojan \
                     Source (CVE-2021-42574) in a refusal. The escape belongs at \
                     the producer: the field is `crate::text::Untrusted` so this \
                     `Display` cannot compile until every interpolation is \
                     `shown()`"
                );
            }
        }
    }

    /// The `{:?}` route, pinned separately because it is a DIFFERENT route with
    /// a different provenance — and this phase has already been bitten by
    /// exactly that gap.
    ///
    /// **What changed here is the PROVENANCE of the guarantee, and the first
    /// assertion below does not go red before the retype.** Said plainly rather
    /// than claimed as a fix: `str`'s own `Debug` already escaped every member
    /// of the invisible class it was handed — `src/registry.rs:65-101` recorded
    /// that measurement in round 8. The guarantee rested on
    /// `core::char::is_printable`, **a standard-library table that no test in
    /// this tree pins, no doc in this tree names, and that moves with a
    /// toolchain upgrade** — a SECOND spelling of a class this project derives
    /// for itself in [`crate::text`]. `WR-04` is what that costs when it is not
    /// the same table: `LegacyRegistryKey`'s *derived* `Debug` printed raw
    /// bytes, because a derive prints the field, not `str::fmt::Debug` of an
    /// escaped copy.
    ///
    /// So the discriminating assertion is the SECOND one: the escape must be in
    /// **this project's own notation** (`U+202E`), produced by this project's
    /// own predicate, not in Rust's (`\u{202e}`). That assertion IS red before
    /// the retype, and it is what proves the route now goes through
    /// [`crate::text::Untrusted`]'s hand-written `Debug` rather than through the
    /// standard library's table.
    #[test]
    fn the_debug_route_of_every_alias_carrying_variant_uses_this_projects_own_notation() {
        for (_, hostile) in crate::test_support::LOOK_ALIKE_PAIRS {
            for (name, _, debug) in alias_carrying_messages(hostile) {
                let survivors: Vec<char> =
                    debug.chars().filter(|c| is_invisible_formatting_char(*c)).collect();
                assert!(
                    survivors.is_empty(),
                    "{name}'s `{{:?}}` carries {survivors:?} from {hostile:?}. \
                     `{{:?}}` reaches log lines, panic messages and `anyhow` \
                     chains, so it is a route in its own right"
                );

                // **Per CHARACTER, not over a concatenation** (WR-08,
                // D-21-46). This used to build one expected string by
                // concatenating the marker of every invisible character in the
                // fixture with no separator, and then assert `contains` on it.
                // That passed only because every `LOOK_ALIKE_PAIRS` member
                // carried exactly ONE such character, which makes the
                // concatenation and the single marker the same string. A
                // fixture carrying TWO makes the expected value `U+200BU+00AD`
                // — a form a CORRECT rendering never produces, because the
                // markers are separated by the visible characters between them
                // — so the test went red for a right implementation. Observed:
                // index 6 of `LOOK_ALIKE_PAIRS` produced
                //   does not carry "U+200BU+00AD" ... It reads
                //   "UnknownAlias { alias: Untrusted(\"dU+200BemoU+00AD\") }"
                // against an implementation that is doing exactly the right
                // thing.
                //
                // The non-vacuity assertion above is UNCHANGED and is still
                // load-bearing: without it a fixture carrying no invisible
                // character would make this loop body execute zero times and the
                // test would pass by silence — which is the failure mode this
                // whole phase is named after.
                let invisible: Vec<char> = hostile
                    .chars()
                    .filter(|c| is_invisible_formatting_char(*c))
                    .collect();
                assert!(
                    !invisible.is_empty(),
                    "the fixture {hostile:?} carries no invisible-class character, \
                     so this assertion would be vacuous — LOOK_ALIKE_PAIRS' \
                     second member is supposed to be the hostile one"
                );
                for c in invisible {
                    let expected = format!("U+{:04X}", c as u32);
                    assert!(
                        debug.contains(&expected),
                        "{name}'s `{{:?}}` does not carry {expected:?} — this \
                         project's own notation for U+{:04X} in {hostile:?}. It \
                         reads {debug:?} instead, which means the escape came \
                         from `core::char::is_printable` (an unpinned std table \
                         that is a second spelling of a class this project \
                         derives) rather than from `crate::text::Untrusted`'s \
                         hand-written `Debug`",
                        c as u32
                    );
                }
            }
        }
    }
}
