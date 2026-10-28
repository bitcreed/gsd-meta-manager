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
// (Until Phase 31 one deliberate exception existed inside the crate: the legacy
// drive error enum's `Display` reached for
// `crate::ui::screens::sanitize_render_line` to paint model-selected tokens
// safely. It went with that type; every message below now escapes an
// `Untrusted` field instead.)

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

/// Why a run could not be started.
///
/// Every variant is returned before [`crate::executor::Executor::start`] hands
/// back a handle, so a failure here is always a start-time error rather than a
/// mid-run surprise.
///
/// **That is not the same as "before any turn begins", and the difference is
/// [`Self::Capability`]'s alone (D-06, 260908-uqq).** Every other variant below
/// is reached before anything could have been written to the child at all, so
/// it genuinely costs zero tokens and zero quota. `Capability` costs zero only
/// on the arm where the CLI announced itself inside
/// [`crate::executor::ExecutionOptions::prompt_release_grace`]; see that
/// variant's own documentation.
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
    /// The first `system/init` failed the capability gate.
    ///
    /// **Whether a user message had already been written to stdin depends on
    /// the CLI's announce timing, and this variant does not distinguish the two
    /// (D-06, TRANS-04, 260908-uqq).**
    ///
    /// * If the CLI announced within
    ///   [`crate::executor::ExecutionOptions::prompt_release_grace`], nothing
    ///   was ever written and the refusal cost zero tokens and zero quota. This
    ///   is the arm CLI 2.1.220 takes.
    /// * If the CLI announced only after reading a user message — CLI
    ///   **2.1.266**, measured — the grace released the prompt first, because
    ///   otherwise there is no init to gate on and the run deadlocks until the
    ///   idle cap. The init that failed the gate is then the one the prompt
    ///   provoked, so this refusal aborted a turn that had already begun.
    ///
    /// The refusal is unconditional either way: the run loop stops and the
    /// agent's process group is torn down on both arms. Only the cost differs,
    /// and a caller that needs to know which arm ran must look at what the
    /// child was actually sent, not at this variant.
    Capability(CapabilityError),
    /// stdout closed before any `system/init` arrived. Either the CLI died
    /// during startup or it is not speaking the `stream-json` protocol at all.
    ///
    /// **This is the "the stream ENDED" case only.** A supervisor bound
    /// expiring while the stream was still open is one of the two variants
    /// below, and collapsing all three into this one is what let a
    /// fifteen-minute stall be reported as a launch failure (260908-uqq).
    InitNeverObserved,
    /// A supervisor **idle** bound expired while `start` was still parked on
    /// the capability gate: the child was launched, it is (or was) alive, and
    /// it simply never said anything.
    ///
    /// This is a *stall*, not a launch failure, and the distinction is the
    /// whole reason the variant exists. The reproduced 2.1.266 deadlock spent
    /// 900.068 seconds here — `ExecutionOptions::default().idle_cap` to the
    /// millisecond — and was reported with the one word that means "the binary
    /// would not launch".
    ///
    /// The supervisor computes the equivalent `RunOutcome::Stalled` and sends
    /// it on its outcome channel, but that channel lives in an
    /// `ExecutionHandle` this path never returns, so the gate channel is the
    /// only surface a caller has and it has to carry the same fact.
    StalledBeforeInit {
        /// The idle cap that was breached.
        idle_for: Duration,
    },
    /// A supervisor **wall-clock** bound expired while `start` was still parked
    /// on the capability gate.
    ///
    /// Kept apart from [`Self::StalledBeforeInit`] on exactly the grounds the
    /// supervisor already separates its two breaches: "went silent" and "took
    /// too long" have different remedies, and a caller that cannot tell them
    /// apart cannot suggest either one.
    TimedOutBeforeInit {
        /// The wall-clock cap that was breached.
        after: Duration,
    },
    /// The command could not be encoded as an outbound NDJSON user message.
    EncodeCommand {
        /// The underlying serialisation error.
        source: serde_json::Error,
    },
    /// The project's agent runtime cannot drive what this spawn asked for, so
    /// it was refused **before anything was launched** (260922-hdj).
    ///
    /// Raised under Codex for the model seam, `resume_session`, `budget_usd`,
    /// and a project whose `.git` is not a directory. A refusal rather than a
    /// silent fallback: quietly running a Codex project's seam under Claude
    /// would spend a second account's quota the user never pointed at it.
    UnsupportedByRuntime {
        /// The runtime, as its config word.
        runtime: &'static str,
        /// What was asked for, named by the driver.
        feature: &'static str,
    },
}

impl fmt::Display for SpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectRootUnusable { root } => {
                write!(
                    f,
                    "project root is not a usable directory: {}",
                    root.display()
                )
            }
            Self::Launch { program, source } => {
                write!(f, "failed to launch `{program}`: {source}")
            }
            Self::PipeUnavailable { pipe } => {
                write!(f, "child {pipe} pipe was not available after spawn")
            }
            Self::PidUnavailable => {
                write!(
                    f,
                    "child pid was not readable after spawn, so no process group could be recorded"
                )
            }
            Self::Capability(err) => write!(f, "{err}"),
            Self::InitNeverObserved => write!(
                f,
                "the process stream ended before any system/init event was observed"
            ),
            Self::StalledBeforeInit { idle_for } => write!(
                f,
                "the child emitted nothing for {} seconds while startup waited for its \
                 system/init, so the run was stopped as stalled rather than launched",
                idle_for.as_secs()
            ),
            Self::TimedOutBeforeInit { after } => write!(
                f,
                "the run's wall-clock cap of {} seconds expired while startup was still \
                 waiting for the child's system/init",
                after.as_secs()
            ),
            Self::EncodeCommand { source } => {
                write!(
                    f,
                    "failed to encode the command as a user message: {source}"
                )
            }
            Self::UnsupportedByRuntime { runtime, feature } => write!(
                f,
                "the {runtime} runtime cannot drive {feature}, so nothing was launched"
            ),
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
    /// The run's agent runtime has no channel for this operation (260922-hdj).
    ///
    /// `codex exec` takes one prompt on argv and reads no stdin, so there is
    /// nothing to deliver a mid-run message or an interrupt over. The driver
    /// journals an injected message as undelivered rather than losing it.
    UnsupportedByRuntime {
        /// The runtime, as its config word.
        runtime: &'static str,
        /// The operation that was attempted.
        operation: &'static str,
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
            Self::UnsupportedByRuntime { runtime, operation } => write!(
                f,
                "the {runtime} runtime has no channel for {operation} on a running agent"
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
                vec![format!(
                    "claude_code_version >= {floor} (observed {observed})"
                )]
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
    /// Not a failure. The remedy is to withdraw and record the opt-in again,
    /// which refreshes the recorded digests. Produced only by the legacy
    /// executor gate (`DrivableProject::from_registry`), which has no production
    /// caller; nothing on the launcher path consults the recorded digests and no
    /// popup shows them any more.
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
                     deliberate opt-in. Press `o` on the project's Driver tab to opt it in"
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
                    "the driver opt-in for `{alias}` was recorded against different file \
                     contents: {}. Withdraw and record the opt-in again to refresh it",
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

    /// Every alias-carrying variant of [`OptInError`], over one value, as
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

        opt_in
            .into_iter()
            .map(|(name, err)| (name, err.to_string(), format!("{err:?}")))
            .collect()
    }

    /// **CR-02, closed at the producer.** Every alias-carrying variant of
    /// [`OptInError`] escapes the value it names, and still names it.
    ///
    /// Measured at the built binary before the retype, the (since retired)
    /// `drive` subcommand on a hostile argv alias printed to a terminal:
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
                let survivors: Vec<char> = debug
                    .chars()
                    .filter(|c| is_invisible_formatting_char(*c))
                    .collect();
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
