//! What survives of the git blast-radius envelope after Phase 31 (SAFE-01).
//!
//! The Phase 19 argv-classifying guard - the `--disallowedTools` denylist, the
//! `PreToolUse` deny hook, the `pre-push` / `pre-commit` hook stubs and the
//! askpass credential layer - is deleted. Safety now comes from capability
//! removal in the sandbox (Phases 28 and 28.1), not from classifying what an
//! agent's shell command *looks like*. What this module still holds is exactly
//! what the launcher's push step ([`crate::launcher`]) calls (D-06: the module
//! keeps its name):
//!
//! - [`policy`]: the branch-namespace refusal ([`policy::classify_push_ref`],
//!   [`policy::validate_namespace`]), the park-reason taxonomy and
//!   [`policy::EnvelopePolicy`] with its pull-request caps;
//! - [`ledger`]: the pull-request ledger that enforces those caps;
//! - [`scan`]: the credential-shape scan, also reachable as
//!   `gsd-meta-manager envelope scan <alias> <root>`, which is the evidence gate
//!   of the launcher and prompt acceptance scripts;
//! - this file: where an alias's envelope directory lives.
//!
//! ## Where the artifacts live
//!
//! Every artifact for an alias lives under
//! `dirs::data_local_dir()/gsd-meta-manager/envelope/<alias>/` - **outside the
//! driven repository** (D-02), on the same base directory `src/main.rs` uses
//! for the log and `src/config.rs` for the registry. That placement is a
//! failure prevented, not a preference: `git add -A` in the driven repository
//! cannot sweep the ledger into a commit, `.gitignore` does not govern it, and a
//! `Write`/`Edit` deny rule on the driven repository does not have to enumerate
//! it.
//!
//! `src/main.rs`'s log-directory fallback to `/tmp` is deliberately **not**
//! copied here. A fallback that could land inside a repository would defeat that
//! placement outright, so an unresolvable data-local directory yields `None` and
//! every caller refuses rather than guessing.
//!
//! ## The honest limit
//!
//! Nothing in this module is a guarantee. It is a client-side control on the
//! launcher's own push step. The boundaries that do not depend on the agent's
//! cooperation are the sandbox's removed capabilities, the remote's own ruleset
//! and the scope of the credential - which is why enabling **server-side branch
//! protection** remains the recommendation rather than a footnote. No
//! server-side protection is ever applied here, no secret is ever rotated or
//! purged, and no `.claude/settings.json` inside the driven repository is ever
//! trusted as a carrier of policy.
//!
//! ## Residual exposure: an agent can write the status that unparks it
//!
//! **This one is disclosed, not closed, and the difference is the point of
//! writing it down.**
//!
//! The deterministic router
//! ([`crate::driver::router`]) parks an unattended run at every
//! human-judgement gate it can observe, and its goal-met predicate is the
//! target phase's verification status read from the leading frontmatter of that
//! phase's `*-VERIFICATION.md`. Both facts come off disk, which is what makes
//! them deterministic — and it is also the exposure: **nothing in this codebase
//! mechanically prevents the driven agent writing `status: passed` into that
//! artifact and thereby unparking itself, or reporting its own goal met.**
//!
//! Three sentences, each of which is a limit rather than a reassurance:
//!
//! 1. **This envelope does not cover it.** What survives here governs the
//!    launcher's *push step* — the branch namespace, the secret scan, the
//!    pull-request ledger. A `Write` into the driven repository's `.planning/`
//!    directory is not a push, and nothing here inspects one. The envelope was
//!    never the boundary for this and does not become one by being nearby.
//! 2. **GSD's staleness detector is a partial mitigation, and it is named here
//!    as the partial thing it is.** GSD's own verification library derives a
//!    `stale` status when a `*-SUMMARY.md` is newer than the `*-VERIFICATION.md`
//!    it is supposed to have been checked against — which is exactly the
//!    fingerprint of a passing status written after the fact. It is partial in
//!    two directions at once: this repository's reader recognises `stale` only
//!    when the value is literally on disk and does not derive it from mtimes,
//!    and an agent that writes the summary and the verification in the same
//!    breath leaves no ordering to detect. Calling it a mitigation without
//!    calling it partial would be the quiet lie this paragraph exists to
//!    refuse.
//! 3. **The hardening belongs to the next phase, and the case is open until it
//!    lands.** The router's contribution is narrower and worth stating exactly:
//!    no *prose* the agent emits reaches a routing arm — every rule and gate
//!    reads a typed enum, a count or a boolean, and the result envelope's own
//!    free-text field is read by no branch of the outcome derivation. That
//!    closes the agent *saying* it is done. It does not close the agent
//!    *writing* the byte that says so.
//!
//! Recorded in this module rather than in the router because this is where the
//! project keeps its list of things it cannot promise, and a residual exposure
//! filed anywhere else is a residual exposure nobody re-reads.

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
/// Exists so a test can point the ledger at a temporary directory instead of the
/// developer's real `~/.local/share`, which is not a thing a test may do.
///
/// A value an attacker sets can only move where the ledger is read and written;
/// policy is compiled in and is not read from here, so there is no value of this
/// variable that turns a refusal into an allow. The residual limit, named
/// honestly: a party who already controls the environment the TUI starts in can
/// relocate the envelope, and can equally edit the ledger files under it.
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
/// The `Option` conscripts the compiler into making every caller decide what
/// to do about a hostile alias, instead of leaving a validation helper that
/// merely *exists* to be called at some of the sites.
pub fn envelope_dir(alias: &str) -> Option<PathBuf> {
    let root = envelope_root()?;
    envelope_dir_in(&root, alias)
}

/// [`envelope_dir`] against an explicit root, for callers that already resolved
/// one - and for tests, which supply a temporary directory.
///
/// **The validation happens before the `join`, not after it.** A check applied
/// to the joined path would have to ask the filesystem what the path means,
/// which is exactly what a traversal check must not depend on (the argument
/// [`crate::journal::is_plain_path_component`] records in full).
///
/// Since D-17-1 that predicate also refuses an alias carrying **embedded
/// invisible formatting**, so two aliases that render identically can no longer
/// name two envelope roots (and, through them, two credential scopes) — the harm
/// pass 5 named and pass 6 reproduced.
///
/// **This function keeps `&str` + `Option` deliberately, and the deviation is
/// recorded rather than left to be read as an oversight (D-17-6).** The pass-6
/// verifier's remediation list proposed replacing it with a signature taking the
/// validated `registry::Alias`. It is not taken, for two reasons that point the
/// same way. First, the `Option` return **is** the WR-02 mechanism: it conscripts
/// the compiler into making every internal caller (the ledger and the launcher's
/// push step) decide what to do about an alias it cannot use — an infallible newtype-taking variant would delete that enumeration.
/// Second, the harm is already closed at this seam by the D-17-1 clause above,
/// and the argv-side bound lives at `Alias::new` in the entry arms; swapping a
/// deliberately fallible seam for an infallible one would remove a defence layer
/// in order to add a redundant one.
pub fn envelope_dir_in(root: &Path, alias: &str) -> Option<PathBuf> {
    if !crate::journal::is_plain_path_component(alias) {
        return None;
    }
    Some(root.join(alias))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This module's own source, resolved at compile time.
    ///
    /// Reading the bytes is what makes the disclosure below a *pinned* one. A
    /// test that asserted a constant would only prove a constant exists; the
    /// paragraph this project owes a reader lives in the module doc, and the
    /// module doc is the thing that can be quietly softened.
    const OWN_SOURCE: &str = include_str!("mod.rs");

    #[test]
    fn the_residual_exposure_is_disclosed_as_open_and_its_mitigation_named_partial() {
        assert!(
            OWN_SOURCE.contains("Residual exposure: an agent can write the status that unparks it"),
            "the residual-exposure section must exist. CONTEXT.md's OQ3 resolution puts the \
             self-unpark case out of Phase 20's scope ON CONDITION that it is recorded here, \
             and a scope exclusion with no disclosure is just an omission"
        );
        assert!(
            OWN_SOURCE.contains("disclosed, not closed"),
            "the disclosure must say plainly that the case is open. A paragraph that describes \
             a hazard without saying it is unclosed reads, to the next person, as a hazard \
             somebody handled"
        );
        assert!(
            OWN_SOURCE.contains("staleness detector is a partial mitigation")
                && OWN_SOURCE.contains("partial in\n//!    two directions"),
            "GSD's staleness detector must be named as the PARTIAL mitigation it is, with the \
             directions it is partial in stated. Naming a partial control without the \
             qualifier is how a residual risk turns into a claimed boundary"
        );
        assert!(
            OWN_SOURCE.contains("hardening belongs to the next phase"),
            "the disclosure must say where the fix lives, or it is a complaint rather than a \
             handoff"
        );
    }

    #[test]
    fn a_hostile_alias_yields_no_envelope_directory_at_all() {
        for hostile in [
            "",
            ".",
            "..",
            "../escaped",
            "a/b",
            "/etc/passwd",
            "./x",
            "x/",
        ] {
            assert!(
                envelope_dir_in(Path::new("/data/envelope"), hostile).is_none(),
                "{hostile:?} must not be joined into an envelope path"
            );
        }

        // **The look-alike half, which is a different harm from the traversal
        // shapes above.** Those are values that escape the root; these are two
        // values that BOTH stay inside it and render identically, so the
        // developer sees one envelope and the tool keeps two — the credential
        // scope included, since it hangs off this directory. Pass 6 reproduced
        // exactly this; refused since D-17-1's clause.
        for (visible, look_alike) in crate::test_support::LOOK_ALIKE_PAIRS {
            assert!(
                envelope_dir_in(Path::new("/data/envelope"), look_alike).is_none(),
                "{look_alike:?} renders exactly as {visible:?} and must not be \
                 joined into an envelope path beside it"
            );
            assert!(
                envelope_dir_in(Path::new("/data/envelope"), visible).is_some(),
                "{visible:?} must still name its envelope root — the refusal \
                 above must be about the invisible bytes, not about the pair"
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
