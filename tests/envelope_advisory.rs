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
fn the_honesty_statement_carries_each_of_its_three_required_parts() {
    // One distinctive phrase per part, rather than the whole paragraph: a
    // rewording is allowed, a DROPPED part is a build failure. Softening the
    // second part is the specific failure this test exists to catch, because an
    // overstated safety claim is worse than a stated limitation — it gets
    // trusted.
    let guaranteed = [
        "cannot reach your ambient git credentials",
        "passes the pre-push hook",
        "append-only ledger this repository does not contain",
    ];
    // Each phrase is deliberately short enough to sit on one wrapped line of the
    // constant, so re-wrapping the paragraph does not fail this test while
    // deleting a clause does.
    let not_guaranteed = [
        "defeatable by an agent that can spawn an unsupervised",
        "do not depend on the agent's cooperation",
        "own ruleset and the scope of the credential",
    ];
    let therefore = ["enable server-side branch protection"];

    for phrase in guaranteed.iter().chain(&not_guaranteed).chain(&therefore) {
        assert!(
            advisory::SECTION_ENVELOPE.contains(phrase),
            "the honesty statement no longer says {phrase:?}. D-27 requires all \
             three parts — what IS guaranteed, what is NOT, and the server-side \
             recommendation — and a dropped clause is exactly what this pin \
             exists to catch:\n{}",
            advisory::SECTION_ENVELOPE
        );
    }

    // And in that order: the recommendation is the conclusion of the two
    // paragraphs above it, not a footnote floating anywhere in the text.
    let first = advisory::SECTION_ENVELOPE
        .find(guaranteed[0])
        .expect("part 1 appears");
    let second = advisory::SECTION_ENVELOPE
        .find(not_guaranteed[0])
        .expect("part 2 appears");
    let third = advisory::SECTION_ENVELOPE
        .find(therefore[0])
        .expect("part 3 appears");
    assert!(
        first < second && second < third,
        "the three parts have a fixed order; got offsets {first}, {second}, {third}"
    );
}

/// A word cap on a safety text, and why one belongs there (G-19-1, D-27).
///
/// The test above proves the three required parts are PRESENT. That is
/// necessary and it is not sufficient. The failure this test catches is
/// different in kind: a statement long enough that nobody finishes it. The human
/// UAT pass on 2026-08-28 found no claim overstated and still recorded the
/// statement as failing its own purpose — *"it reads like it's too long.. it's
/// hard to follow"*. D-27's closing line is that an overstated safety claim is
/// worse than a stated limitation because it gets trusted, and PITFALLS' concern
/// is precisely that safety claims get trusted **without being understood**. An
/// honesty statement that is not read is doing none of the work D-27
/// commissioned it for, so legibility is held here by a control rather than by
/// an intention — otherwise the text re-grows one well-meaning clarification at
/// a time and G-19-1 quietly re-opens.
#[test]
fn the_honesty_statement_stays_short_enough_that_a_reader_finishes_it() {
    // 250 tokens measured before the G-19-1 rewrite. This cap was first set at
    // 190, from a projected ~172 (a 161-token body plus the 11-token section
    // header, which is the header `dry_run` renders and is not the density
    // problem). That projection did not survive contact with the text: the
    // rewrite carries every residual disclosure the phase enumerated, plus the
    // newly required opening ceiling sentence and the closing recommendation,
    // and its floor is 200. Reaching 190 needed telegraphic fragments that
    // damaged the legibility this whole control exists to protect, and buying
    // tokens with content is the one thing the rewrite was forbidden to do.
    //
    // So the cap is derived from the achieved count rather than from the
    // projection: 200 achieved, +15 (7.5%) of ordinary-rewording headroom, and
    // the pre-fix density of 250 still fails by 35. A cap set flush against a
    // floor that mandated content already dictates is not a control — it is a
    // standing invitation to shave a word off a disclosure to make the build
    // pass, which is precisely the failure the pin test above guards. The
    // number is arithmetic, not taste, so a future editor can audit it.
    const MAX_TOKENS: usize = 215;

    let tokens = advisory::SECTION_ENVELOPE.split_whitespace().count();
    assert!(
        tokens <= MAX_TOKENS,
        "the honesty statement is {tokens} whitespace-separated tokens, over the \
         {MAX_TOKENS} cap. Shortening it must never mean softening it — the pin \
         test above is what makes that a build failure — so cut redundancy and \
         layout, never a residual disclosure (G-19-1, D-27):\n{}",
        advisory::SECTION_ENVELOPE
    );

    // This clause is a GUARD, not a repair: the widest line measured 76 before
    // the rewrite, so it is green on arrival. It protects the property the pin
    // test above silently depends on — a pinned phrase straddling a `\n` cannot
    // match, and the failure message would then name the phrase rather than the
    // wrapping, sending the next reader at entirely the wrong thing.
    const MAX_LINE: usize = 80;

    if let Some((number, line)) = advisory::SECTION_ENVELOPE
        .lines()
        .enumerate()
        .find(|(_, line)| line.chars().count() > MAX_LINE)
    {
        panic!(
            "line {} of the honesty statement is {} characters, over the \
             {MAX_LINE} cap. Wrap around the pinned phrases, never through one \
             — the longest is `defeatable by an agent that can spawn an \
             unsupervised` at 53 characters. Offending line:\n{line}",
            number + 1,
            line.chars().count()
        );
    }
}

#[test]
fn the_journal_notice_and_the_rendered_preview_carry_the_same_claim_text() {
    use gsd_meta_manager::driver::dry_run;
    use gsd_meta_manager::state_reader::git_ops::{PushPreview, WorkingTreeStat};

    for state in [
        ProtectionState::Protected,
        ProtectionState::Unprotected,
        advisory::not_probed(),
    ] {
        let notice = advisory::envelope_notice(&state);
        let rendered = dry_run::render(&dry_run::DryRunReport {
            commands: vec!["/gsd:progress".to_string()],
            diffstat: WorkingTreeStat::default(),
            push: PushPreview::default(),
            protection: state.clone(),
        });

        assert!(
            rendered.contains(&notice),
            "the preview and the run journal must carry the SAME claim, produced \
             once — two assemblies are two things that can drift, and a preview \
             and a journal disagreeing about what was claimed is the worst drift \
             available here (T-19-42).\nState: {state:?}\nNotice:\n{notice}\n\
             Rendered:\n{rendered}"
        );
    }
}

#[test]
fn the_preview_warns_for_an_unprotected_remote_and_shows_the_reason_for_an_unknown_one() {
    use gsd_meta_manager::driver::dry_run;
    use gsd_meta_manager::state_reader::git_ops::{PushPreview, WorkingTreeStat};

    let render_with = |state: ProtectionState| {
        dry_run::render(&dry_run::DryRunReport {
            commands: vec!["/gsd:progress".to_string()],
            diffstat: WorkingTreeStat::default(),
            push: PushPreview::default(),
            protection: state,
        })
    };

    let unprotected = render_with(ProtectionState::Unprotected);
    assert!(
        unprotected.contains(PROTECTION_WARNING),
        "an unprotected remote is warned about in the preview; got:\n{unprotected}"
    );

    let unknown_state = advisory::not_probed();
    let reason = unknown_state
        .reason()
        .expect("an unknown carries a reason")
        .to_string();
    let unknown = render_with(unknown_state);
    assert!(
        unknown.contains(PROTECTION_WARNING) && unknown.contains(&reason),
        "an unknown state is warned about AND carries its reason into the \
         preview; got:\n{unknown}"
    );

    // The invariant, asserted where a user actually reads it.
    assert!(
        !unknown.contains(&protection_line(&ProtectionState::Protected)),
        "an unknown must never render the protected line; got:\n{unknown}"
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
