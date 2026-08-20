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
        alias: String,
    },
    /// The project is registered but carries no `driver_opt_in` record. A
    /// registered project is one the dashboard may *read*; driving it is a
    /// separate, deliberate act (D-14).
    NotOptedIn {
        /// The alias that is registered but not opted in.
        alias: String,
    },
    /// The registered path is not an existing directory. Checked before the
    /// process is launched, mirroring [`SpawnError::ProjectRootUnusable`]'s
    /// reasoning: a stale registry entry must not spawn an agent against a path
    /// that no longer exists.
    RootUnusable {
        /// The alias whose path failed the check.
        alias: String,
        /// The root that failed the check.
        root: PathBuf,
    },
}

impl fmt::Display for OptInError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAlias { alias } => {
                write!(f, "no project is registered under the alias `{alias}`")
            }
            Self::NotOptedIn { alias } => write!(
                f,
                "the project `{alias}` has not opted in to being driven; \
                 registering a project lets the dashboard read it, driving it is a separate \
                 deliberate opt-in"
            ),
            Self::RootUnusable { alias, root } => write!(
                f,
                "the registered path for `{alias}` is not a usable directory: {}",
                root.display()
            ),
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
        /// The id that was refused, verbatim.
        run_id: String,
    },
    /// Neither `--command` nor `--target-phase` was supplied (CTRL-06).
    ///
    /// A run with no command source has nothing to do. Until Phase 20 clap made
    /// this unrepresentable by requiring `--command`; the moment a second source
    /// existed, "exactly one of these two" stopped being something a parser can
    /// express and became a seam refusal — the same move, and for the same
    /// reason, as the `run_id` pair above.
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
        /// The value that was refused, verbatim.
        target_phase: String,
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

impl fmt::Display for DriveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            Self::RunIdInvalid { run_id } => write!(
                f,
                "the run id {run_id:?} is not a single directory name, so it is refused \
                 before anything is created. A run id names one directory under \
                 .planning/meta-manager/runs/; it may not contain a path separator, \
                 `..`, or a leading `/`"
            ),
            Self::NoCommandSource => write!(
                f,
                "a run needs something to do: pass `--command <c>` to run one GSD \
                 command, or `--target-phase <N>` to let the decision router choose \
                 each command from observed project state"
            ),
            Self::AmbiguousCommandSource => write!(
                f,
                "`--command` and `--target-phase` are mutually exclusive — the first \
                 runs one supplied command, the second runs a routed sequence under \
                 the run bounds. Pass exactly one, so the run's terminal record names \
                 the mode you chose"
            ),
            Self::TargetPhaseInvalid { target_phase } => write!(
                f,
                "the target phase {target_phase:?} is not a single plain path \
                 component, so it is refused before anything is created. A target \
                 phase is a phase number such as `20`; it may not contain a path \
                 separator, `..`, or a leading `/`"
            ),
            Self::BoundsRefused(refusal) => write!(f, "{refusal}"),
            Self::EscalationRefused(refusal) => write!(f, "{refusal}"),
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
            | Self::NoCommandSource
            | Self::AmbiguousCommandSource
            | Self::TargetPhaseInvalid { .. }
            // `BoundsRefusal` is a plain data enum rather than an `Error`: it is
            // a classification of an argv value, not a failure that wrapped one.
            // `EscalationRefusal` is the same shape for the same reason.
            | Self::BoundsRefused(_)
            | Self::EscalationRefused(_)
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
