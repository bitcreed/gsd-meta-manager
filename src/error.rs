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
/// Feature detection is by `capabilities[]` on `system/init`, never by
/// comparing version strings (D-06). The error always names what is missing so
/// the user sees a capability, not a version number.
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
        }
    }
}

impl std::error::Error for CapabilityError {}
