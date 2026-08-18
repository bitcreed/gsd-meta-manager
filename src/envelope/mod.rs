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

pub mod cred;
pub mod hooks;
pub mod policy;

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
/// [`crate::journal::is_plain_run_id`] records in full).
pub fn envelope_dir_in(root: &Path, alias: &str) -> Option<PathBuf> {
    if !crate::journal::is_plain_run_id(alias) {
        return None;
    }
    Some(root.join(alias))
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
}
