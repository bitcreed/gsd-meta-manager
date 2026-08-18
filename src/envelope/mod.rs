//! The git blast-radius envelope (SAFE-01, D-01..D-10).
//!
//! A **top-level** module rather than a submodule of `src/driver/` (D-01),
//! because it is invoked from three distinct processes and that is what settles
//! the placement: the TUI renders its honesty text in a preview, the driver
//! builds the child's environment before spawn, and **this same binary is
//! re-entered as a subprocess** by a git hook. A module under `driver/` would
//! imply the driver owns every caller, and it does not.
//!
//! ## Where the artifacts live, and the three failures that placement prevents
//!
//! Every artifact for an alias lives under
//! `dirs::data_local_dir()/gsd-meta-manager/envelope/<alias>/` — **outside the
//! driven repository** (D-02), on the same base directory `src/main.rs:19` uses
//! for the log and `src/config.rs:170` for the registry. Each of the three
//! reasons is a failure it prevents, not a preference:
//!
//! 1. **`git add -A` in the driven repository cannot sweep them into a commit.**
//!    The agent's own sweep is the concrete hazard here, not a hypothetical one.
//! 2. **They are not governed by `.gitignore`**, so an agent that edits
//!    `.gitignore` cannot expose them.
//! 3. **A `Write`/`Edit` deny rule on the driven repository does not have to
//!    enumerate them** in order to protect them.
//!
//! `src/main.rs`'s log-directory fallback to `/tmp` is deliberately **not**
//! copied here. A fallback that could land inside a repository would defeat
//! reason 1 outright, so an unresolvable data-local directory yields `None` and
//! every caller refuses rather than guessing.
//!
//! ## The three layers, and what each one is blind to (D-06)
//!
//! Enforcement is three **independent** layers. The "does NOT see" sentence
//! under each is the load-bearing half: a future change that removes a layer has
//! to delete a sentence that says what it was for, which is harder to do by
//! accident than deleting a call.
//!
//! 1. **The `--disallowedTools` argv denylist.** Cheapest, fires before the tool
//!    runs, and cannot be silently dropped, because it is argv rather than a
//!    file that could fail validation.
//!    This layer does NOT see any command that never reaches the tool boundary —
//!    anything issued from a nested shell, a Makefile, or a script the agent
//!    wrote and then ran through a single permitted invocation.
//! 2. **The `PreToolUse` deny hook.** It splits the Bash command into argv and
//!    applies the policy functions, rather than prefix-matching a string, so
//!    `git  push   --force` and `git push -f` are the same verdict.
//!    This layer does NOT see a command whose argv the shell-quoting split
//!    cannot recover: a reassembled variable (`g=push; git $g --force`), `eval`,
//!    a base64-decoded string, or a script the agent writes and then runs. It
//!    raises the cost of an accident to near-certain detection; it does not stop
//!    deliberate evasion.
//! 3. **The `pre-push` git hook** ([`hooks::pre_push`]). It reads the actual
//!    `<local-ref> <local-sha> <remote-ref> <remote-sha>` lines git supplies on
//!    stdin, regardless of how git was invoked. This is the only layer that
//!    observes ground truth.
//!    This layer does NOT see anything at all if the child's process tree has
//!    had the injected git configuration removed — an agent that unsets
//!    `GIT_CONFIG_COUNT` in a subshell is past it.
//!
//! Layers 1 and 2 exist to make layer 3's known bypasses (`--no-verify`,
//! `core.hooksPath`) unreachable through the tool boundary. Layer 3 exists
//! because layers 1 and 2 only see commands that go through the `Bash` tool.
//! Neither direction is redundant.
//!
//! ## What this module will not grow
//!
//! Three things are out of scope permanently, not merely unimplemented:
//!
//! - **No server-side protection is ever applied.** Applying a ruleset needs
//!   `Administration` scope, which the run credential withholds by design. The
//!   remote's protection state is *probed and reported*, never mutated.
//! - **No secret is ever rotated or purged.** The envelope blocks; recovery
//!   after a leak is a documented human procedure. A module that could purge
//!   history is a module that could destroy it.
//! - **No `.claude/settings.json` inside the driven repository is ever trusted
//!   as a carrier of policy.** That file is agent-writable, and a control whose
//!   carrier the agent can edit is not a control.
//!
//! ## The honest limit
//!
//! Nothing in this module is a guarantee. It is a client-side control: an agent
//! that can spawn an unsupervised shell and choose to can strip the environment
//! this module relies on (see [`cred::hooks_path_env`]). The only boundaries
//! that do not depend on the agent's cooperation are the remote's own ruleset
//! and the scope of the credential — which is why enabling **server-side branch
//! protection** is the phase's conclusion rather than its footnote.

pub mod advisory;
pub mod cred;
pub mod hooks;
pub mod ledger;
pub mod policy;
pub mod scan;

use std::path::{Path, PathBuf};

/// The application data subdirectory, shared with the log and the registry.
const APP_SUBDIR: &str = "gsd-meta-manager";

/// The envelope's own subdirectory beneath [`APP_SUBDIR`].
const ENVELOPE_SUBDIR: &str = "envelope";

/// Environment override for the envelope root, honoured before `dirs`.
///
/// **Why this exists at all, stated rather than left to be discovered.** The
/// end-to-end fixture in `tests/envelope_tracer.rs` has to install a hook stub,
/// hand its directory to a real `git push`, and have the re-entered binary agree
/// about which directory is the sanctioned one. Without an override the fixture
/// would have to write into the developer's real `~/.local/share`, which is not
/// a thing a test may do.
///
/// **Why it is not the `--claude-program` case.** `cli.rs:107-129` argues that a
/// hidden *flag* is right where an env var would be wrong, because an env var is
/// inherited by children. That argument is about a knob shaped like arbitrary
/// code execution. This one is shaped differently, and the difference is
/// mechanical: a value an attacker sets can only move the *sanctioned* location
/// away from the hook git actually runs, and a mismatch is a **refusal**
/// ([`hooks`] compares the invoked path against this root). Policy is compiled
/// in and is not read from here, so there is no value of this variable that
/// turns a refusal into an allow.
///
/// The residual limit, named honestly: a party who already controls the
/// environment the TUI starts in can relocate the envelope. That party can
/// equally unset `GIT_CONFIG_COUNT`, which is D-09's stated ceiling, so this
/// override does not lower a boundary that was standing.
pub const ENVELOPE_ROOT_ENV: &str = "GSD_MM_ENVELOPE_ROOT";

/// The root every alias's envelope directory hangs from, or `None`.
///
/// `None` rather than a fallback, for the reason in the module doc: there is no
/// second-choice directory that is guaranteed to be outside a repository.
pub fn envelope_root() -> Option<PathBuf> {
    if let Some(override_root) = std::env::var_os(ENVELOPE_ROOT_ENV) {
        if !override_root.is_empty() {
            return Some(PathBuf::from(override_root));
        }
    }
    dirs::data_local_dir().map(|d| d.join(APP_SUBDIR).join(ENVELOPE_SUBDIR))
}

/// One alias's envelope directory, or `None` for an alias that is not a plain
/// path component (D-03).
///
/// The `Option` is doing the same job here that it does in
/// [`crate::journal::run_paths`]: it conscripts the compiler into making every
/// caller decide what to do about a hostile alias, instead of leaving a
/// validation helper that merely *exists* to be called at some of the sites.
pub fn envelope_dir(alias: &str) -> Option<PathBuf> {
    let root = envelope_root()?;
    envelope_dir_in(&root, alias)
}

/// [`envelope_dir`] against an explicit root, for callers that already resolved
/// one — and for the fixture, which supplies a temporary directory.
///
/// **The validation happens before the `join`, not after it.** A check applied
/// to the joined path would have to ask the filesystem what the path means,
/// which is exactly what a traversal check must not depend on (the argument
/// [`crate::journal::is_plain_path_component`] records in full).
pub fn envelope_dir_in(root: &Path, alias: &str) -> Option<PathBuf> {
    if !crate::journal::is_plain_path_component(alias) {
        return None;
    }
    Some(root.join(alias))
}

// ---------------------------------------------------------------------------
// D-24: every refusal parks the run
// ---------------------------------------------------------------------------

/// What one attempt to record a park actually did.
///
/// **Five outcomes rather than a `bool` or a `Result<(), _>`, because the four
/// non-success cases are four different facts about the world** and a caller
/// that collapsed them would print one sentence for all of them. "The child
/// unset the locator", "this alias has no live run", "the `active` pointer names
/// something that is not a run directory" and "the journal would not take the
/// append" are things a human reading a failed push needs told apart.
///
/// **None of them softens the refusal.** By the time any of these is produced
/// the verdict is already decided and the non-zero exit is already committed
/// to; this type describes the *evidence*, never the decision (D-25, SAFE-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParkOutcome {
    /// The event is on disk, at this sequence number.
    Appended {
        /// The `seq` the journal writer stamped.
        seq: u64,
    },
    /// [`cred::PROJECT_ROOT_ENV`] is absent from this process's environment, so
    /// there is no run journal to reach. See [`park`] for why that costs the
    /// agent its own evidence and buys it nothing.
    NoLocator,
    /// The locator resolved, and the project has no active run — a hook that
    /// outlived the run it belonged to, which is the ordinary case rather than
    /// a fault.
    NoActiveRun,
    /// [`crate::journal::run_paths`] refused to build a path from the run id.
    /// Carries the refusal's *shape*, never a resolved path.
    ///
    /// **Unreachable today, and kept because D-28 says the `None` is handled
    /// rather than unwrapped.** `journal::writer::read_active_run` validates
    /// the pointer first and answers `None` for a traversing one, so a hostile
    /// `active` file surfaces as [`Self::NoActiveRun`] before `run_paths` is
    /// ever reached. This variant is the second of the two layers: if that
    /// validation were ever weakened, this is what still stands between an
    /// agent-controlled string and a `join`. A branch that `unwrap`ped here
    /// instead would be one edit away from the WR-02 traversal.
    Unresolvable(String),
    /// The journal exists and would not take the append.
    Failed(String),
}

impl ParkOutcome {
    /// One line naming what happened, for the second stderr line a non-appended
    /// outcome writes.
    fn describe(&self) -> String {
        match self {
            Self::Appended { seq } => format!("recorded as journal event {seq}"),
            Self::NoLocator => format!(
                "NOT recorded: {} is absent from this process's environment, so no run \
                 journal could be located. The refusal above still stands",
                cred::PROJECT_ROOT_ENV
            ),
            Self::NoActiveRun => {
                "NOT recorded: this project has no active run, so there is no journal to \
                 park. The refusal above still stands"
                    .to_string()
            }
            Self::Unresolvable(detail) => format!(
                "NOT recorded: the active-run pointer could not be resolved ({detail}). \
                 The refusal above still stands"
            ),
            Self::Failed(detail) => format!(
                "NOT recorded: the run journal would not take the event ({detail}). \
                 The refusal above still stands"
            ),
        }
    }
}

/// Append a [`crate::journal::JournalEvent::Parked`] to the active run of the
/// project rooted at `project_root`.
///
/// **The whole of the work, with the project root as an argument**, so this is a
/// unit test with no process in it and no environment mutation — the same
/// factoring [`cred::build_env`] and `executor::claude::build_argv` already use,
/// and for the same reason: the environment is process-global, so a function
/// that reads it cannot be exercised by a parallel test suite without a race.
/// [`park`] is the one-line wrapper that reads it.
///
/// **Why the `active` pointer is unambiguous here rather than racy.**
/// `src/driver/lock.rs` holds a per-project advisory lock for the duration of a
/// run, so an alias has **at most one** live run and the pointer names it. A
/// pointer that [`crate::journal::JournalRun::finish`] has already cleared
/// yields [`ParkOutcome::NoActiveRun`] — the correct answer for a hook that
/// outlived its run, and not a guess at which run it might have meant.
///
/// [`crate::journal::run_paths`]' `Option` is **handled, never unwrapped**
/// (D-28). That signature is the whole of the WR-02 path-traversal fix and it
/// exists to conscript the compiler into enumerating callers; this is one of the
/// callers it conscripted, and the `None` becomes
/// [`ParkOutcome::Unresolvable`] rather than a panic — the `active` file lives
/// inside the driven project, so **the agent controls it**.
pub fn park_at(project_root: &Path, reason: policy::ParkReason, needs: &str) -> ParkOutcome {
    let planning = project_root.join(".planning");

    // The ONE run resolver. `read_active_run` validates the pointer before it
    // answers, so a traversing `active` file — which the agent can write —
    // becomes `None` here rather than a path this function would join.
    let Some(run_id) = crate::journal::writer::read_active_run(&planning) else {
        return ParkOutcome::NoActiveRun;
    };

    let Some(paths) = crate::journal::run_paths(&planning, &run_id) else {
        // The refusal's shape, never the refused value: it is agent-controlled
        // and this string reaches a terminal (D-28).
        return ParkOutcome::Unresolvable(format!(
            "the active pointer names {} bytes that are not a single plain path \
             component",
            run_id.len()
        ));
    };

    let mut writer = match crate::journal::writer::JournalWriter::open(&paths.journal) {
        Ok(writer) => writer,
        Err(err) => return ParkOutcome::Failed(crate::journal::redact::redact(&err.to_string())),
    };

    match writer.append(&crate::journal::JournalEvent::Parked {
        reason: reason.as_str().to_string(),
        needs: needs.to_string(),
    }) {
        Ok(seq) => ParkOutcome::Appended { seq },
        Err(err) => ParkOutcome::Failed(crate::journal::redact::redact(&err.to_string())),
    }
}

/// [`park_at`] against the project the envelope environment names, for the
/// re-entry points that are separate processes.
///
/// **Why the locator travels on the environment rather than on the subcommand's
/// arguments.** The alternative is resolving the project root from `alias`,
/// which goes through `DrivableProject::from_registry` — whose single production
/// call site `tests/spawn_seam_guard.rs` holds (D-28). A second call site added
/// to make a log line writable would trade a real containment property for an
/// evidence convenience. It is also why no `cli::EnvelopeAction` variant changes
/// shape: the three re-entry points keep the arguments plans 19-01, 19-03 and
/// 19-05 gave them.
///
/// **The honest limit.** An agent that unsets [`cred::PROJECT_ROOT_ENV`] in a
/// subshell loses its own park record and **gains nothing** — the refusal is
/// already decided by the time this is called, and the non-zero exit does not
/// depend on what this returns. The answer to the residue is D-27's server-side
/// branch protection recommendation, not a stronger client-side guess.
///
/// `detail` goes into the stderr line this writes, **not** into the event.
/// `Parked`'s two fields are the schema, and widening them is a journal
/// migration this phase has no reason to spend.
///
/// **Silence is the one behaviour forbidden here.** An outcome other than
/// [`ParkOutcome::Appended`] writes a second stderr line naming which outcome it
/// was, so a refusal that could not be recorded says so rather than looking like
/// a refusal that was never attempted. That line is written *here* rather than
/// left to each caller, because "every caller remembers to report it" is not a
/// property anything can check.
pub fn park(reason: policy::ParkReason, needs: &str, detail: &str) -> ParkOutcome {
    let outcome = match std::env::var_os(cred::PROJECT_ROOT_ENV) {
        Some(root) if !root.is_empty() => park_at(Path::new(&root), reason, needs),
        _ => ParkOutcome::NoLocator,
    };

    if !matches!(outcome, ParkOutcome::Appended { .. }) {
        eprintln!(
            "gsd-meta-manager envelope: park ({}) {} — {detail}",
            reason.as_str(),
            outcome.describe(),
        );
    }

    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hostile_alias_yields_no_envelope_directory_at_all() {
        for hostile in ["", ".", "..", "../escaped", "a/b", "/etc/passwd", "./x", "x/"] {
            assert!(
                envelope_dir_in(Path::new("/data/envelope"), hostile).is_none(),
                "{hostile:?} must not be joined into an envelope path"
            );
        }
    }

    #[test]
    fn a_plain_alias_hangs_directly_off_the_supplied_root() {
        assert_eq!(
            envelope_dir_in(Path::new("/data/envelope"), "alpha"),
            Some(PathBuf::from("/data/envelope/alpha"))
        );
    }

    // -----------------------------------------------------------------------
    // `park_at`: the whole of the work, with no process and no environment in
    // it. `park`'s one environment read is exercised end to end through spawned
    // processes in `tests/envelope_wiring.rs`, because a function that reads a
    // process-global cannot be driven from a parallel suite without a race.
    // -----------------------------------------------------------------------

    /// A started run in a temp project, and the project root it lives under.
    fn project_with_a_live_run(run_id: &str) -> (tempfile::TempDir, crate::journal::JournalRun) {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let planning = dir.path().join(".planning");
        let record = crate::journal::RunRecord {
            run_id: run_id.to_string(),
            goal: "prove the park lands".to_string(),
            gsd_command: "/gsd-progress".to_string(),
            target: "host".to_string(),
            opt_in: None,
            started_at: "2026-08-18T00:00:00Z".to_string(),
            session_id: "00000000-0000-0000-0000-000000000000".to_string(),
            pid: std::process::id(),
            pgid: std::process::id(),
            claude_code_version: "2.1.214".to_string(),
            argv_digest: "fnv1a64:0000000000000000".to_string(),
            ended_at: None,
            outcome: None,
        };
        let run = crate::journal::JournalRun::start(&planning, record).expect("the run starts");
        (dir, run)
    }

    /// Every `Parked` event on disk, as `(reason, needs)`.
    fn parked_on_disk(journal: &Path) -> Vec<(String, String)> {
        let (records, _) = crate::journal::reader::read_all(journal).expect("readable");
        records
            .iter()
            .filter(|record| record.kind == "parked")
            .map(|record| {
                (
                    record.rest["reason"].as_str().unwrap_or_default().to_string(),
                    record.rest["needs"].as_str().unwrap_or_default().to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn a_park_against_a_live_run_lands_exactly_one_event_carrying_that_reason() {
        let (dir, run) = project_with_a_live_run("2026-08-18T00-00-00Z-live");
        let journal = run.paths().journal.clone();

        let outcome = park_at(
            dir.path(),
            policy::ParkReason::ForcePushBlocked,
            "a human to review the push",
        );

        assert!(
            matches!(outcome, ParkOutcome::Appended { .. }),
            "a live run must take the park, got {outcome:?}"
        );
        assert_eq!(
            parked_on_disk(&journal),
            vec![(
                policy::REASON_FORCE_PUSH_BLOCKED.to_string(),
                "a human to review the push".to_string()
            )],
            "exactly one event, carrying the reason that was passed in"
        );
    }

    #[test]
    fn a_project_with_no_active_pointer_yields_no_active_run_and_appends_nothing() {
        // The ordinary case, not a fault: a hook that outlived its run. It must
        // pass against the RED stub too, or it would be proving nothing.
        let dir = tempfile::TempDir::new().expect("temp dir");
        std::fs::create_dir_all(dir.path().join(".planning")).expect("scratch .planning");

        assert_eq!(
            park_at(dir.path(), policy::ParkReason::SecretDetected, "human"),
            ParkOutcome::NoActiveRun
        );
        assert!(
            !crate::journal::runs_root(&dir.path().join(".planning")).exists()
                || crate::journal::run_paths(&dir.path().join(".planning"), "any")
                    .map(|p| !p.journal.exists())
                    .unwrap_or(true),
            "a park with nowhere to go must create nothing"
        );
    }

    #[test]
    fn an_active_pointer_that_is_not_a_plain_component_is_refused_and_appends_nothing() {
        // The `active` file lives inside the driven project, so the AGENT
        // controls it. Both layers hold here: `read_active_run` refuses the
        // traversing pointer, and `run_paths`' `Option` is handled rather than
        // unwrapped behind it (D-28, WR-02).
        let (dir, run) = project_with_a_live_run("2026-08-18T00-00-00Z-traverse");
        let journal = run.paths().journal.clone();
        let runs = crate::journal::runs_root(&dir.path().join(".planning"));

        for hostile in ["../../../../escaped\n", "a/b\n", "/etc/passwd\n", "..\n"] {
            std::fs::write(runs.join("active"), hostile).expect("the hostile pointer");
            let outcome = park_at(dir.path(), policy::ParkReason::SecretDetected, "human");
            assert_eq!(
                outcome,
                ParkOutcome::NoActiveRun,
                "{hostile:?} must be refused by the pointer reader before a path is \
                 ever joined; if this ever reports Unresolvable instead, the second \
                 layer caught what the first stopped catching"
            );
            assert!(
                parked_on_disk(&journal).is_empty(),
                "{hostile:?} appended into the run it was pointed away from"
            );
            assert!(
                !Path::new("/tmp/escaped").exists(),
                "a traversing pointer was followed out of the project"
            );
        }
    }

    #[test]
    fn a_park_never_reports_appended_unless_something_was_appended() {
        // The pairing. Every row above asserts a refusal to record; this one
        // asserts the recording still happens when it should, so an
        // implementation that answered `NoActiveRun` unconditionally — which is
        // exactly the RED stub — cannot satisfy the file.
        let (dir, run) = project_with_a_live_run("2026-08-18T00-00-00Z-pair");
        let journal = run.paths().journal.clone();

        for reason in [
            policy::ParkReason::SecretDetected,
            policy::ParkReason::PrCapExceeded,
            policy::ParkReason::HookBypassBlocked,
        ] {
            assert!(
                matches!(
                    park_at(dir.path(), reason, "human"),
                    ParkOutcome::Appended { .. }
                ),
                "a live run must take {reason:?}"
            );
        }

        let reasons: Vec<String> = parked_on_disk(&journal)
            .into_iter()
            .map(|(reason, _)| reason)
            .collect();
        assert_eq!(
            reasons,
            vec![
                policy::REASON_SECRET_DETECTED,
                policy::REASON_PR_CAP_EXCEEDED,
                policy::REASON_HOOK_BYPASS_BLOCKED,
            ],
            "three parks, in order, each carrying its own reason"
        );
    }
}
