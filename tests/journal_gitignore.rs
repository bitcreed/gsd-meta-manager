// ============================================================================
// The ignore posture (D-07, D-08), proved against a real repository
//
// Everything here needs a real `git` binary and a real repository, which is why
// it is not in-source with the writer's unit tests. The library can assert that
// it wrote six particular lines; only git can answer whether those six lines
// make `run.json` stageable and `journal.jsonl` invisible.
//
// The property under test is one git answers BACKWARDS when asked the obvious
// way. `git check-ignore` reports *"did a pattern match?"*, not *"is this file
// ignored?"*, and a negation counts as a match — so the quiet form exits 0 for
// the very file our `!*/run.json` rule deliberately re-includes, reporting it
// as ignored, the precise inverse of the truth. A first pass of RESEARCH §1.2
// made exactly that mistake before the ground-truth check corrected it.
//
// So these tests verify by STAGING: `git add -A` followed by `git ls-files`
// asks the index what it actually holds, which is the only answer that cannot
// be inverted. Do not "simplify" any assertion below into a question about
// whether a pattern matched — that simplification is how this file becomes
// vacuous while still passing.
// ============================================================================

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use gsd_meta_manager::journal::inbox::{self, InboxMessage};
use gsd_meta_manager::journal::writer::{
    create_run_dir, parent_excludes_run_record, write_active_pointer, write_run_record,
};
use gsd_meta_manager::journal::{runs_root, RunPaths, RunRecord};

const R1: &str = "2026-07-28T14-03-11Z-a3f9";
const R2: &str = "2026-07-29T01-00-00Z-b111";

/// Whether `git` is usable here.
///
/// A CI image without git is not a failure of this code, but a silent pass is
/// indistinguishable from a real one — so the skip prints its reason.
fn git_available() -> bool {
    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => true,
        _ => {
            println!(
                "SKIPPED: `git` is not available on PATH, so the ignore posture \
                 cannot be proved against a real repository here."
            );
            false
        }
    }
}

/// Run git inside `repo`, isolated from whatever config the environment has.
///
/// Both config paths point at a file that does not exist, so a developer's
/// global `core.excludesFile` or `user.name` cannot change the answer this test
/// measures.
fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    let nowhere = repo.join("no-such-gitconfig");
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_CONFIG_GLOBAL", &nowhere)
        .env("GIT_CONFIG_SYSTEM", &nowhere)
        .output()
        .expect("git runs, because git_available() said so")
}

/// A fresh repository with a local identity. Nothing is committed: staging is
/// sufficient to answer the question and is faster.
fn init_repo(root: &Path) {
    std::fs::create_dir_all(root).expect("create the repository directory");
    let output = git(root, &["init", "--quiet"]);
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    git(root, &["config", "user.email", "journal@example.invalid"]);
    git(root, &["config", "user.name", "Journal Test"]);
}

/// The set of paths the index holds.
///
/// Stage everything with `git add -A`, then ask the index what it actually
/// took. This is the ground truth of RESEARCH §1.2 — the one form of the
/// question that cannot be answered backwards.
fn tracked(root: &Path) -> HashSet<String> {
    let staged = git(root, &["add", "-A"]);
    assert!(
        staged.status.success(),
        "staging failed: {}",
        String::from_utf8_lossy(&staged.stderr)
    );
    let listed = git(root, &["ls-files"]);
    assert!(
        listed.status.success(),
        "listing the index failed: {}",
        String::from_utf8_lossy(&listed.stderr)
    );
    String::from_utf8_lossy(&listed.stdout)
        .lines()
        .map(|line| line.to_string())
        .collect()
}

/// The path git would print for `absolute`, relative to the repository root.
fn rel(root: &Path, absolute: &Path) -> String {
    absolute
        .strip_prefix(root)
        .expect("every fixture path is inside the repository")
        .to_string_lossy()
        .replace('\\', "/")
}

fn record(run_id: &str) -> RunRecord {
    RunRecord {
        run_id: run_id.to_string(),
        goal: "prove the ignore posture".to_string(),
        gsd_command: "/gsd:execute-phase".to_string(),
        target: "claude".to_string(),
        opt_in: None,
        started_at: "2026-07-28T14:03:11Z".to_string(),
        session_id: "9f1c0e2a-0000-4000-8000-000000000000".to_string(),
        pid: 4242,
        pgid: 4242,
        claude_code_version: "2.1.0".to_string(),
        argv_digest: "fnv1a64:0000000000000000".to_string(),
        ended_at: None,
        outcome: None,
    }
}

/// Build one run's on-disk footprint **through the library**, so a change to
/// the library's ignore body is caught here rather than drifting away from an
/// inline copy of it.
///
/// The inbox is written through `inbox::append` at `RunPaths::inbox` rather than
/// with a raw write at a hand-typed `dir.join("inbox.jsonl")`. That was the
/// vacuity: a test that invents the path it then asserts is ignored proves that
/// *a path shaped like that* would be ignored, not that the file the TUI
/// actually writes is. Since plan 18-06 the inbox is a real file the injection
/// flow appends to, so both halves now come from the library.
fn populate_run(planning: &Path, run_id: &str) -> RunPaths {
    let paths = create_run_dir(planning, run_id).expect("create the run directory");
    write_run_record(&paths, &record(run_id)).expect("write the run record");
    std::fs::write(&paths.journal, "{\"kind\":\"exec_event\",\"seq\":1}\n")
        .expect("write the journal");
    inbox::append(&paths.inbox, &InboxMessage::new("steer the run"))
        .expect("append a message to the inbox, through the production writer");
    assert!(
        paths.inbox.is_file(),
        "the inbox must actually exist on disk, or every assertion about it \
         below is about a file that was never written: {}",
        paths.inbox.display()
    );
    paths
}

fn planning_dir(root: &Path) -> PathBuf {
    root.join(".planning")
}

#[test]
fn staging_a_real_repo_tracks_run_json_and_ignores_the_journal() {
    if !git_available() {
        return;
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    init_repo(root);
    let planning = planning_dir(root);

    let p1 = populate_run(&planning, R1);
    let p2 = populate_run(&planning, R2);
    write_active_pointer(&runs_root(&planning), R1).expect("write the active pointer");

    // One record a level deeper than the documented depth. It must stay
    // ignored: the re-inclusion matches exactly one level, and only the
    // per-run record at the documented depth is meant to be committed.
    let deeper = p1.dir.join("nested");
    std::fs::create_dir_all(&deeper).expect("create the deeper directory");
    std::fs::write(deeper.join("run.json"), "{}\n").expect("write the deeper record");

    let files = tracked(root);

    // Present: the ignore file and both records at the documented depth.
    for expected in [
        rel(root, &p1.gitignore),
        rel(root, &p1.run_json),
        rel(root, &p2.run_json),
    ] {
        assert!(
            files.contains(&expected),
            "{expected} must be stageable, but the index holds {files:?}"
        );
    }

    // The inbox files are real files on disk, written through the production
    // appender by `populate_run`. **That is what makes the two assertions below
    // non-vacuous**: "this path is not in the index" is trivially true of a path
    // nothing ever created, so the existence check is the half that turns the
    // absence into evidence.
    for inbox_path in [&p1.inbox, &p2.inbox] {
        assert!(
            inbox_path.is_file(),
            "the inbox must exist before its absence from the index means \
             anything: {}",
            inbox_path.display()
        );
        assert!(
            std::fs::metadata(inbox_path)
                .expect("the inbox is readable")
                .len()
                > 0,
            "and must hold the message that was appended to it: {}",
            inbox_path.display()
        );
    }

    // Absent: every transcript, the inbox files, the pointer, and the deeper
    // record. Asserting only the first direction would pass against an ignore
    // file that ignores nothing at all.
    for forbidden in [
        rel(root, &p1.journal),
        rel(root, &p2.journal),
        rel(root, &p1.inbox),
        rel(root, &p2.inbox),
        rel(root, &p1.active),
        rel(root, &deeper.join("run.json")),
    ] {
        assert!(
            !files.contains(&forbidden),
            "{forbidden} must never reach the index, but it did: {files:?}"
        );
    }
}

#[test]
fn a_parent_that_excludes_the_directory_is_detected() {
    if !git_available() {
        return;
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    init_repo(root);

    // The driven repository has opted our whole namespace out, as a DIRECTORY.
    // Git never descends into an excluded directory, so the nested ignore file
    // we are about to write never runs and its re-inclusion cannot take effect.
    // Written before the run directory exists, which is the real-world order.
    std::fs::write(
        root.join(".gitignore"),
        "# the driven repository's own\n.planning/meta-manager/\n",
    )
    .expect("write the parent ignore file");

    let planning = planning_dir(root);
    let paths = populate_run(&planning, R1);

    let files = tracked(root);
    assert!(
        files.contains(".gitignore"),
        "the repository's own ignore file is tracked: {files:?}"
    );
    assert!(
        !files.contains(&rel(root, &paths.run_json)),
        "RESEARCH §1.3's measured failure: the record is not committable, {files:?}"
    );
    assert!(
        !files.contains(&rel(root, &paths.gitignore)),
        "our ignore file is inside the excluded directory too"
    );

    // The user-facing surface is a tracing::warn!, which an integration test
    // cannot capture; the predicate behind it is what is asserted here.
    assert!(
        parent_excludes_run_record(root, &paths.run_json),
        "the detection predicate must agree with what staging measured"
    );
}

#[test]
fn a_parent_that_excludes_by_file_glob_is_harmless() {
    if !git_available() {
        return;
    }
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path();
    init_repo(root);

    // A file glob, not a directory. RESEARCH §1.3 measured this case as benign,
    // and pinning it keeps a future over-broad diagnostic from crying wolf —
    // a warning users learn to ignore is worse than no warning.
    std::fs::write(root.join(".gitignore"), "*.jsonl\n").expect("write the parent ignore file");

    let planning = planning_dir(root);
    let paths = populate_run(&planning, R1);

    let files = tracked(root);
    assert!(
        files.contains(&rel(root, &paths.run_json)),
        "a file glob must leave the record committable: {files:?}"
    );
    assert!(
        files.contains(&rel(root, &paths.gitignore)),
        "and must leave our ignore file tracked: {files:?}"
    );
    assert!(
        !parent_excludes_run_record(root, &paths.run_json),
        "a benign parent must not raise the diagnostic"
    );
}
