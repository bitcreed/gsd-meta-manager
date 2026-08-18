// ============================================================================
// The remote-protection probe and the pinned honesty statement (D-26, D-27).
//
// **Every fixture here is offline.** A `file://` remote and a repository with no
// remote at all are the two shapes that exercise the probe's early exits, and
// neither reaches a network. That is D-35's rule rather than a convenience: a
// criterion that needed a real GitHub repository would be a criterion this phase
// could not honestly verify, and a probe test that hit the network would fail
// for reasons that have nothing to do with the code.
//
// The two properties under test are asymmetric on purpose:
//
//   * the probe, where the load-bearing assertion is that a FAILURE to ask
//     yields `unknown` **carrying its own reason** — never `protected`, and
//     never a silence that reads as an absence of risk;
//   * the honesty statement, where the load-bearing assertion is that each of
//     its three required parts is present, so a later refactor that softens the
//     paragraph naming what is NOT guaranteed fails the build instead of
//     shipping.
// ============================================================================

mod common;

use std::path::{Path, PathBuf};

use gsd_meta_manager::envelope::advisory::{
    self, protection_line, ProtectionState, PROTECTION_WARNING,
};
use gsd_meta_manager::envelope::cred;

/// The module under audit, resolved at compile time so the test is
/// cwd-independent — the idiom `tests/spawn_seam_guard.rs:23` already uses.
const ADVISORY_SRC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/envelope/advisory.rs");

/// A git repository with no remote configured, inside `dir`.
fn repo_without_remote(dir: &Path) -> bool {
    common::git(dir, &["init", "--quiet"])
}

/// The environment the probe runs the client under.
fn env_for(root: &Path, alias: &str, project_root: &Path) -> cred::EnvelopeEnv {
    cred::build_env_in(root, alias, project_root, Path::new(common::BIN))
        .expect("building the child environment for a real directory succeeds")
}

#[test]
fn a_file_url_remote_is_unknown_with_that_as_the_reason_rather_than_unprotected() {
    let Some(fx) = common::fixture("advisory-file") else {
        eprintln!(
            "SKIPPED a_file_url_remote_is_unknown_with_that_as_the_reason_rather_than_unprotected: \
             the sandbox forbids `git init`, so nothing below was asserted"
        );
        return;
    };

    let env = env_for(&fx.envelope_root, &fx.alias, &fx.work);
    let state = advisory::probe_protection(&fx.work, &env);

    assert_eq!(
        state.as_str(),
        "unknown",
        "a local remote is one the probe cannot ask, not one that is unprotected: {state:?}"
    );
    let reason = state.reason().expect("an unknown carries a reason");
    assert!(
        reason.contains("names no network host"),
        "the reason has to say WHY it is not known, so a reader can tell a local \
         remote from an unreachable one; got: {reason}"
    );
}

#[test]
fn a_repository_with_no_remote_at_all_is_unknown_and_says_so() {
    let tmp = tempfile::TempDir::new().expect("a temp dir");
    let work = tmp.path().join("work");
    std::fs::create_dir_all(&work).expect("the work directory");
    if !repo_without_remote(&work) {
        eprintln!(
            "SKIPPED a_repository_with_no_remote_at_all_is_unknown_and_says_so: the \
             sandbox forbids `git init`, so nothing below was asserted"
        );
        return;
    }

    let env = env_for(&tmp.path().join("envelope"), "advisory-bare", &work);
    let state = advisory::probe_protection(&work, &env);

    assert_eq!(state.as_str(), "unknown", "got {state:?}");
    let reason = state.reason().expect("an unknown carries a reason");
    assert!(
        reason.contains("no `origin` remote"),
        "a project with nothing to push to must say that, not report a verdict \
         about a remote it does not have; got: {reason}"
    );
}

#[test]
fn the_two_offline_reasons_are_distinct_so_a_reader_can_tell_them_apart() {
    let Some(fx) = common::fixture("advisory-distinct") else {
        eprintln!(
            "SKIPPED the_two_offline_reasons_are_distinct_so_a_reader_can_tell_them_apart: \
             the sandbox forbids `git init`, so nothing below was asserted"
        );
        return;
    };
    let bare_repo = fx.tmp().join("no-remote");
    std::fs::create_dir_all(&bare_repo).expect("the second work directory");
    if !repo_without_remote(&bare_repo) {
        eprintln!("SKIPPED the_two_offline_reasons_are_distinct_...: `git init` failed");
        return;
    }

    let with_remote =
        advisory::probe_protection(&fx.work, &env_for(&fx.envelope_root, &fx.alias, &fx.work));
    let without_remote = advisory::probe_protection(
        &bare_repo,
        &env_for(&fx.envelope_root, "advisory-distinct-2", &bare_repo),
    );

    assert_ne!(
        with_remote.reason(),
        without_remote.reason(),
        "two different failures to probe must not collapse into one reason — the \
         reason is the entire value of an `unknown`"
    );
}

#[test]
fn an_unknown_renders_its_reason_and_never_borrows_the_protected_wording() {
    let unknown = advisory::not_probed();
    assert_eq!(unknown.as_str(), "unknown");

    let rendered = protection_line(&unknown);
    let protected = protection_line(&ProtectionState::Protected);
    let reason = unknown.reason().expect("an unknown carries a reason");

    assert!(
        rendered.contains(reason),
        "the reason has to be visible where the state is shown, not only carried \
         in the type; got: {rendered}"
    );
    assert!(
        rendered.contains(PROTECTION_WARNING),
        "an unknown state is a warning, so it carries the marker; got: {rendered}"
    );
    // The protected line's distinctive clause, taken from the rendering itself
    // so a rewording of one cannot silently pass this test.
    let claim = protected
        .split(" — ")
        .nth(1)
        .expect("the protected line carries a claim after its dash")
        .trim_end_matches('.');
    assert!(
        !rendered.contains(claim),
        "an unknown must never render as protected — the invariant this whole \
         module exists for. Unknown line: {rendered}\nProtected claim: {claim}"
    );
}

#[test]
fn an_unprotected_state_renders_with_the_warning_marker() {
    let rendered = protection_line(&ProtectionState::Unprotected);

    assert!(
        rendered.contains(PROTECTION_WARNING),
        "`unprotected` is one of the two states D-26 requires a warning for; got: {rendered}"
    );
    assert!(
        rendered.contains("unprotected"),
        "the state's own identifier belongs in the line a human reads; got: {rendered}"
    );
}

#[test]
fn the_module_has_no_write_path_at_all() {
    let source = std::fs::read_to_string(PathBuf::from(ADVISORY_SRC))
        .expect("the advisory module is readable");

    // Extraction integrity first: a test that scanned the wrong file would pass
    // vacuously, and a vacuous pass in a security suite is the failure mode this
    // phase exists to prevent.
    assert!(
        source.contains("pub fn probe_protection"),
        "this test must be scanning the advisory module itself"
    );

    for verb in ["POST", "PUT", "PATCH", "DELETE"] {
        assert!(
            !source.contains(verb),
            "src/envelope/advisory.rs mentions {verb}: applying or removing \
             protection needs an Administration scope the run credential \
             withholds by design (D-18/D-26), so this module has no write path \
             even in its prose"
        );
    }
}
