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
        command: COMMAND.to_string(),
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
// ---------------------------------------------------------------------------

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
