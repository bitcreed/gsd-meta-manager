// ============================================================================
// CTRL-02: the dry-run preview, and the mechanical proof it wrote nothing and
// spawned nothing.
//
// This is an integration test rather than an in-source one because every proof
// here needs the real world: a genuine git repository with a remote, an upstream
// and a dirty working tree; the whole `.git` directory on disk to fingerprint;
// and a real child process for the tripwire's positive control.
//
// The file is deliberately NOT `#![cfg(unix)]`. Unlike the rest of the driver,
// the preview is portable — it is git reads and string building, and only the
// *running* of an agent is a Unix capability (D-05). The single test that spawns
// a shell script carries its own `#[cfg(unix)]` instead.
// ============================================================================

use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use gsd_meta_manager::config::{Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::{drive, dry_run, DriveArgs};
use gsd_meta_manager::error::DriveError;
use gsd_meta_manager::executor::DrivableProject;
use tempfile::TempDir;

const TRIPWIRE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-tripwire.sh"
);

const ALIAS: &str = "preview";
const COMMAND: &str = "/gsd:progress";
/// The tracked file the repository is built with and then modifies.
const TRACKED: &str = "tracked.txt";
/// The untracked file `git diff --stat` will never mention.
const UNTRACKED: &str = "brand-new.txt";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Run one git command in `dir`, reporting only whether it succeeded.
fn git(dir: &Path, args: &[&str]) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// A repository with one commit, a modified tracked file, an untracked file, a
/// remote and an upstream — every input all three sections need.
///
/// `None` when the sandbox forbids `git init`/`git commit`, so the tests skip
/// gracefully rather than failing for a reason that is not about the code.
fn repo() -> Option<TempDir> {
    let dir = TempDir::new().ok()?;
    let root = dir.path();

    if !git(root, &["init"]) {
        return None;
    }
    // Repo-scoped identity, so the tests neither depend on nor disturb a
    // developer's global git configuration.
    git(root, &["config", "user.email", "test@example.com"]);
    git(root, &["config", "user.name", "Test User"]);
    git(root, &["config", "commit.gpgsign", "false"]);

    std::fs::write(root.join(TRACKED), "one\ntwo\nthree\n").ok()?;
    if !git(root, &["add", TRACKED]) {
        return None;
    }
    if !git(root, &["commit", "-m", "initial commit"]) {
        return None;
    }

    let branch = String::from_utf8(
        std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["symbolic-ref", "--short", "HEAD"])
            .output()
            .ok()?
            .stdout,
    )
    .ok()?
    .trim()
    .to_string();

    git(
        root,
        &["remote", "add", "origin", "https://example.invalid/demo.git"],
    );
    git(root, &["config", &format!("branch.{branch}.remote"), "origin"]);
    git(
        root,
        &[
            "config",
            &format!("branch.{branch}.merge"),
            &format!("refs/heads/{branch}"),
        ],
    );
    git(root, &["config", "push.default", "simple"]);

    // The working-tree state a run would inherit: one tracked modification and
    // one untracked file.
    std::fs::write(root.join(TRACKED), "one\ntwo\nthree\nfour\n").ok()?;
    std::fs::write(root.join(UNTRACKED), "not committed\n").ok()?;

    // A `.planning/` directory, so "no run directory was created" is an
    // assertion about the driver rather than about a missing parent.
    std::fs::create_dir_all(root.join(".planning")).ok()?;

    Some(dir)
}

/// A one-entry, opted-in registry pointing at `root`.
fn config_for(root: &Path) -> Config {
    let mut config = Config::new();
    config.projects.insert(
        ALIAS.to_string(),
        RegisteredProject {
            path: root.to_path_buf(),
            added: "2026-07-29T12:00:00Z".to_string(),
            driver_opt_in: Some(DriverOptIn {
                opted_in_at: "2026-07-29T11:59:00Z".to_string(),
                claude_md_digest: None,
                branch_namespace: None,
                credential: None,
                pr_cap_per_24h: None,
                pr_cap_per_run: None,
            }),
            extra: Default::default(),
        },
    );
    config
}

/// Dry-run arguments, optionally pointed at the tripwire with an evidence path.
fn args(evidence: Option<&Path>) -> DriveArgs {
    DriveArgs {
        alias: ALIAS.to_string(),
        command: Some(COMMAND.to_string()),
        target_phase: None,
        max_steps: None,
        wall_clock_cap_secs: None,
        max_escalations: None,
        run_id: None,
        dry_run: true,
        goal: None,
        claude_program: evidence.map(|_| PathBuf::from(TRIPWIRE)),
        claude_args: evidence
            .map(|path| vec![OsString::from(path)])
            .unwrap_or_default(),
    }
}

// ---------------------------------------------------------------------------
// The zero-write fingerprint
// ---------------------------------------------------------------------------

/// Three independent captures of a repository's git state.
///
/// PITFALLS:520 names the reflog check specifically. The other two close the
/// ways a write could hide from it: a ref updated without a reflog entry, and a
/// new object or an index rewrite that touches neither. The directory listing
/// carries a content digest as well as a length, because the single most likely
/// unwanted write — `git diff` opportunistically refreshing `.git/index` — leaves
/// the file exactly the same size.
fn git_fingerprint(repo: &Path) -> (String, String, Vec<(String, u64, u64)>) {
    let capture = |args: &[&str]| -> String {
        std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).into_owned())
            .unwrap_or_default()
    };

    let reflog = capture(&["reflog", "--all", "--format=%H%x09%gd%x09%gs"]);
    let refs = capture(&["for-each-ref", "--format=%(refname) %(objectname)"]);

    let git_dir = repo.join(".git");
    let mut listing = Vec::new();
    walk(&git_dir, &git_dir, &mut listing);
    listing.sort();

    (reflog, refs, listing)
}

/// Every file under `dir` as `(path relative to `base`, byte length, digest)`.
fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, u64, u64)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            walk(&path, base, out);
        } else {
            let relative = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .display()
                .to_string();
            let bytes = std::fs::read(&path).unwrap_or_default();
            let mut hasher = DefaultHasher::new();
            bytes.hash(&mut hasher);
            out.push((relative, bytes.len() as u64, hasher.finish()));
        }
    }
}

// ---------------------------------------------------------------------------
// The proofs
/// The same invocation, routed instead of command-mode.
///
/// A routed preview reads `.planning/` and calls the decision router, neither of
/// which the command-mode path does — so the zero-write and zero-spawn proofs
/// have to be re-run against it rather than inherited.
fn routed_args(evidence: Option<&Path>) -> DriveArgs {
    DriveArgs {
        command: None,
        target_phase: Some("20".to_string()),
        ..args(evidence)
    }
}

// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_routed_dry_run_also_leaves_the_git_directory_byte_identical() {
    let Some(repo_dir) = repo() else {
        return;
    };
    let root = repo_dir.path();

    let (reflog_before, refs_before, listing_before) = git_fingerprint(root);
    assert!(
        !refs_before.trim().is_empty(),
        "the fingerprint must actually see this repository's refs"
    );
    assert!(
        listing_before.len() > 3,
        "the walk must descend into .git; it saw {} files",
        listing_before.len()
    );

    drive(routed_args(None), &config_for(root))
        .await
        .expect("a routed dry-run against an opted-in project succeeds");

    let (reflog_after, refs_after, listing_after) = git_fingerprint(root);

    assert_eq!(
        reflog_before, reflog_after,
        "a routed preview reads project state and calls the router — neither may \
         move a ref (D-23, PITFALLS:520)"
    );
    assert_eq!(refs_before, refs_after, "nor update one without a reflog entry");
    assert_eq!(
        listing_before, listing_after,
        "nor write an object or the index"
    );
}

#[tokio::test]
async fn a_preview_refuses_exactly_what_the_real_run_would_refuse() {
    let Some(repo_dir) = repo() else {
        return;
    };
    let root = repo_dir.path();
    let config = config_for(root);

    // **WR-09.** Both refusals used to sit BELOW the dry-run branch.
    //
    // The first cost a false claim: `RouterAction::command_for`'s doc asserts
    // the phase "arrived on argv and was validated at the seam", and the preview
    // rendered `../../../escaped` verbatim into something that reads as a
    // pasteable command line.
    let mut escaped = routed_args(None);
    escaped.target_phase = Some("../../../escaped".to_string());
    let refusal = drive(escaped, &config)
        .await
        .expect_err("a preview must refuse a target phase that is not a plain path component");
    assert!(
        matches!(refusal, DriveError::TargetPhaseInvalid { .. }),
        "and it must be the SAME typed refusal a real run gives, not a preview \
         of an invocation nobody can run. Got: {refusal:?}"
    );

    // The second cost a wrong answer to the preview's only question. A preview
    // of `--max-steps 0` reported what would happen; what would happen is a
    // refusal.
    let mut zero_cap = routed_args(None);
    zero_cap.max_steps = Some(0);
    let refusal = drive(zero_cap, &config)
        .await
        .expect_err("a preview of a cap that can never take a step must refuse it");
    assert!(
        matches!(refusal, DriveError::BoundsRefused(_)),
        "a preview whose whole purpose is `what would happen` must not answer \
         cleanly for an invocation that would be refused. Got: {refusal:?}"
    );
}

#[tokio::test]
async fn a_routed_dry_run_spawns_no_agent() {
    let Some(repo_dir) = repo() else {
        return;
    };
    let root = repo_dir.path();
    let evidence = root.join("tripwire-fired-routed");

    // The tripwire program leaves a file behind if it is ever executed, so this
    // proves the absence of a spawn rather than asserting it.
    drive(routed_args(Some(&evidence)), &config_for(root))
        .await
        .expect("a routed dry-run succeeds");

    assert!(
        !evidence.exists(),
        "the tripwire fired — a routed PREVIEW executed the agent program, which \
         is the one thing a preview may never become (D-23, T-20-11)"
    );
}

#[test]
fn a_routed_preview_lists_the_routers_own_first_selection_and_nothing_after_it() {
    let Some(repo_dir) = repo() else {
        return;
    };
    let root = repo_dir.path();

    let project = DrivableProject::for_testing_bypassing_opt_in(ALIAS, root);
    let preview = dry_run::build_routed_report(&project, "20");
    let rendered = dry_run::render_routed(&preview);

    // This fixture has no `.planning/` to corroborate phase 20, so the router
    // parks — which is itself the honest answer, and the preview says so rather
    // than inventing a command.
    assert!(
        preview.report.commands.len() <= 1,
        "a routed preview lists at most the FIRST selection; it must never pad \
         the list with guesses, got {:?}",
        preview.report.commands
    );
    assert!(
        rendered.contains(dry_run::SECTION_COMMANDS),
        "the pinned commands section still renders:\n{rendered}"
    );
    assert!(
        rendered.contains("Routed run:"),
        "and a routed preview says which model it is previewing:\n{rendered}"
    );
}

#[tokio::test]
async fn a_dry_run_leaves_the_git_directory_byte_identical() {
    let Some(repo_dir) = repo() else {
        return;
    };
    let root = repo_dir.path();

    let (reflog_before, refs_before, listing_before) = git_fingerprint(root);
    // A fingerprint that saw nothing would compare equal to itself, so the
    // comparison below is pinned against vacuity first.
    assert!(
        !refs_before.trim().is_empty(),
        "the fingerprint must actually see this repository's refs"
    );
    assert!(
        listing_before.len() > 3,
        "the walk must descend into .git; it saw {} files",
        listing_before.len()
    );

    drive(args(None), &config_for(root))
        .await
        .expect("a dry-run against an opted-in project succeeds");

    let (reflog_after, refs_after, listing_after) = git_fingerprint(root);

    assert_eq!(
        reflog_before, reflog_after,
        "the REFLOG diverged — a dry-run moved a ref (D-23, PITFALLS:520)"
    );
    assert_eq!(
        refs_before, refs_after,
        "the REF LISTING diverged — a ref was updated without a reflog entry (D-23)"
    );
    assert_eq!(
        listing_before, listing_after,
        "the .git DIRECTORY LISTING diverged — an object or the index was written \
         without touching a ref (D-23)"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn a_dry_run_never_executes_the_agent_program() {
    let Some(repo_dir) = repo() else {
        return;
    };
    let root = repo_dir.path();

    let scratch = TempDir::new().expect("temp dir");
    let evidence = scratch.path().join("tripwire-fired");

    drive(args(Some(&evidence)), &config_for(root))
        .await
        .expect("a dry-run against an opted-in project succeeds");

    assert!(
        !evidence.exists(),
        "the tripwire left evidence at {}, so the dry-run path constructed an \
         executor and spawned an agent (D-23)",
        evidence.display()
    );

    // A tripwire that has never been seen to fire proves nothing. Run the same
    // fixture directly and require the evidence to appear.
    let status = std::process::Command::new(TRIPWIRE)
        .arg(&evidence)
        .status()
        .expect("the tripwire fixture is executable");
    assert!(
        !status.success(),
        "the tripwire exits non-zero by design, so a caller ignoring its output still fails"
    );
    assert!(
        evidence.exists(),
        "positive control: the tripwire must create its evidence file when it IS \
         executed, or its absence above would prove nothing"
    );
}

#[tokio::test]
async fn a_dry_run_writes_no_run_directory_and_no_journal() {
    let Some(repo_dir) = repo() else {
        return;
    };
    let root = repo_dir.path();
    let planning = root.join(".planning");

    drive(args(None), &config_for(root))
        .await
        .expect("a dry-run against an opted-in project succeeds");

    let meta_manager = planning.join("meta-manager");
    assert!(
        !meta_manager.exists(),
        "a preview writes no run directory and no journal, but {} exists (D-23)",
        meta_manager.display()
    );

    let lock = gsd_meta_manager::driver::lock::lock_path(&planning);
    assert!(
        !lock.exists(),
        "a preview never contends for the single-execution lock, but {} exists",
        lock.display()
    );
}

#[test]
fn the_dry_run_output_names_the_command_the_diffstat_and_the_refspecs() {
    let Some(repo_dir) = repo() else {
        return;
    };
    let root = repo_dir.path();

    // Built and rendered directly, so the assertion is on the string itself
    // rather than on captured stdout.
    let project = DrivableProject::for_testing_bypassing_opt_in(ALIAS, root);
    let rendered = dry_run::render(&dry_run::build_report(&project, COMMAND));

    let commands = rendered
        .find(dry_run::SECTION_COMMANDS)
        .unwrap_or_else(|| panic!("the commands section is missing:\n{rendered}"));
    let diffstat = rendered
        .find(dry_run::SECTION_DIFFSTAT)
        .unwrap_or_else(|| panic!("the diffstat section is missing:\n{rendered}"));
    // PITFALLS:69 names "the dry-run output does not include a refspec list" as
    // THE warning sign for a hollow preview, and its disappearance is invisible
    // to a smoke test. This assertion is what makes it a build failure.
    let refspecs = rendered
        .find(dry_run::SECTION_REFSPECS)
        .unwrap_or_else(|| panic!("the refspec section is missing:\n{rendered}"));

    assert!(
        commands < diffstat && diffstat < refspecs,
        "the three sections have a fixed order; got offsets {commands}, {diffstat}, {refspecs}"
    );

    assert!(
        rendered.contains(COMMAND),
        "the command the run would issue has to appear:\n{rendered}"
    );

    let diffstat_section = &rendered[diffstat..refspecs];
    assert!(
        diffstat_section.contains(TRACKED),
        "the modified tracked file belongs in the diffstat section:\n{diffstat_section}"
    );
    assert!(
        diffstat_section.contains("Untracked:") && diffstat_section.contains(UNTRACKED),
        "the untracked file belongs under its own sub-list — `git diff --stat` \
         never mentions it, and omitting it would under-report exactly the files \
         a fresh agent creates:\n{diffstat_section}"
    );

    let refspec_section = &rendered[refspecs..];
    assert!(
        refspec_section
            .lines()
            .any(|line| line.trim().starts_with("refs/heads/") && line.contains(':')),
        "a full refs/heads/<src>:refs/<dst> line must appear — a bare branch name \
         hides the destination, which is the whole point (D-22.3):\n{refspec_section}"
    );
    assert!(
        refspec_section.contains("https://example.invalid/demo.git"),
        "the resolved remote URL is carried verbatim so the user recognises \
         it:\n{refspec_section}"
    );
}
