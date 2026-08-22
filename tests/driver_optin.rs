// ============================================================================
// ROADMAP success criterion #4, second half: a live run against one project
// performs no write, no spawn and no git operation against any other
// registered project.
//
// This is an integration test rather than an in-source one because it needs the
// real world twice over: it initialises two genuine git repositories and it
// spawns the checked-in shell stand-in as a real child process. Neither belongs
// in a unit test under `src/`.
//
// WHAT THIS FILE IS NOT, and the distinction is load-bearing: it is a **test**
// that a run stays inside its project, never a sandbox **mechanism**. Nothing
// here confines anything. A driven agent has ordinary filesystem access to every
// other registered project and this file does not take it away — it asserts that
// the driver, as written, does not use it. The containment story for v2.0 is
// Phase 19's blast-radius envelope (push allowlists, tool denylists, branch
// policy) and Phase 22's container target. Criterion #4's own Watch note says so
// in as many words: do not let this grow into path confinement. The absence of a
// mechanism here is a scope decision with a named owner, not an oversight.
//
// Unix-only by construction: driving is a Unix capability (D-05) and the run
// body is `#[cfg(unix)]`.
// ============================================================================

#![cfg(unix)]

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use gsd_meta_manager::config::Config;
use gsd_meta_manager::driver::{drive, DriveArgs};
use gsd_meta_manager::registry;
use gsd_meta_manager::state_reader::git_ops;
use tempfile::TempDir;

/// A visible argv payload for the fixtures below.
///
/// `DriveArgs`'s argv-derived fields are `payload::NonBlank`, whose field is
/// private: there is exactly one route in and it refuses a value carrying
/// nothing a reader could see. It `expect`s rather than returning the
/// constructor's `Option` directly, so a fixture whose own literal turned out to
/// be invisible fails loudly here instead of silently becoming an ABSENT flag —
/// which would quietly convert a test of "blank is refused" into a test of
/// "nothing was supplied".
fn nonblank(raw: &str) -> gsd_meta_manager::driver::payload::NonBlank {
    gsd_meta_manager::driver::payload::NonBlank::new(raw)
        .expect("a visible test literal is a payload")
}


/// The stand-in that records its own `$PWD` before emitting a single stream
/// byte. Checked in by plan 17-01 for exactly this test.
const FAKE_CWD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-cwd.sh"
);

/// The clean-success capture, named `01-success-textonly.ndjson` in
/// `tests/fixtures/transcripts/README.md`.
const CLEAN_BASELINE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/transcripts/01-success-textonly.ndjson"
);

/// A fixed run id, so an assertion reads a known path rather than globbing.
const RUN_ID: &str = "2026-07-29T12-00-00Z-b7c1";

/// One file's identity: its path relative to the tree root, its byte length, and
/// a content digest.
type FileFingerprint = (String, u64, u64);

/// A non-cryptographic content digest, reusing `journal::argv_digest`.
///
/// The rendering is `fnv1a64:` plus sixteen lowercase hex digits; the hex half is
/// parsed back out so a fingerprint entry stays a cheap tuple of scalars. Like
/// the function it wraps, this authenticates nothing — it answers "did these
/// bytes change", which is the whole question here.
fn content_digest(bytes: &[u8]) -> u64 {
    let text = String::from_utf8_lossy(bytes).into_owned();
    let digest = gsd_meta_manager::journal::argv_digest(&[text]);
    let hex = digest
        .strip_prefix("fnv1a64:")
        .expect("argv_digest renders its prefix unconditionally");
    u64::from_str_radix(hex, 16).expect("argv_digest renders sixteen hex digits")
}

/// Every file under `root`, sorted, with its length and content digest.
///
/// **Nothing is skipped — `.git/` least of all.** A git operation against the
/// wrong project is precisely what criterion #4 forbids, and `.git` is where it
/// would show; a walk that filtered dot-directories would pass while the exact
/// failure it exists to catch was happening.
///
/// **mtime is deliberately not part of the fingerprint.** Reading a tree can move
/// atime on some mounts and a nervous filesystem could move mtime with it, and a
/// false failure in a safety test is worse than a slightly weaker one — it trains
/// the reader to ignore the alarm. Length plus content digest already catches
/// every write, which is the property under test.
///
/// Symlinks are recorded by their own entry rather than followed into, so a link
/// loop cannot hang the walk.
fn fingerprint_tree(root: &Path) -> Vec<FileFingerprint> {
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<FileFingerprint>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.is_dir() {
            walk(root, &path, out);
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        let bytes = std::fs::read(&path).unwrap_or_default();
        out.push((rel, bytes.len() as u64, content_digest(&bytes)));
    }
}

/// A human-readable symmetric difference, so a regression names the file that
/// moved instead of printing two thousand-line vectors.
fn describe_difference(before: &[FileFingerprint], after: &[FileFingerprint]) -> String {
    let index = |list: &[FileFingerprint]| -> BTreeMap<String, (u64, u64)> {
        list.iter()
            .map(|(path, len, digest)| (path.clone(), (*len, *digest)))
            .collect()
    };
    let before = index(before);
    let after = index(after);

    let mut lines = Vec::new();
    for (path, entry) in &before {
        match after.get(path) {
            None => lines.push(format!("  removed: {path}")),
            Some(other) if other != entry => lines.push(format!(
                "  changed: {path} ({} bytes -> {} bytes)",
                entry.0, other.0
            )),
            Some(_) => {}
        }
    }
    for path in after.keys() {
        if !before.contains_key(path) {
            lines.push(format!("  added:   {path}"));
        }
    }
    if lines.is_empty() {
        "  (no difference)".to_string()
    } else {
        lines.join("\n")
    }
}

/// Run one git command in `root`, failing loudly.
///
/// Identity and signing are forced on the command line so the test does not
/// depend on — or disturb — the developer's global git configuration.
fn git(root: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args([
            "-c",
            "user.name=gsd-test",
            "-c",
            "user.email=gsd-test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .output()
        .expect("git is on PATH");
    assert!(
        output.status.success(),
        "git {:?} failed in {}: {}",
        args,
        root.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A committed GSD project at `<tmp>/<name>`, with a `.planning/` tree.
fn make_project(tmp: &Path, name: &str) -> PathBuf {
    let root = tmp.join(name);
    let planning = root.join(".planning");
    std::fs::create_dir_all(&planning).expect("the project scaffold is writable");
    std::fs::write(
        planning.join("STATE.md"),
        format!("# {name}\n\nstatus: idle\n"),
    )
    .expect("STATE.md is writable");

    git(&root, &["init", "-q"]);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "initial"]);

    root
}

/// Point the envelope at a temp root for this test binary.
///
/// **Since plan 19-07 a driven run establishes its envelope before the executor
/// is constructed**, and the envelope lives under the application data
/// directory. Without this redirect these fixtures would write hook stubs, a
/// generated git config and a settings file into the developer's real
/// `~/.local/share` under a fixture's alias — the same "a test may not write
/// into the developer's real data directory" rule `envelope::hooks::guard_in`
/// records for its own explicit-root sibling.
///
/// The `set_var` happens exactly **once** per test binary, inside the
/// `OnceLock` initialiser, and the `TempDir` is held by the `static` for the
/// process lifetime so the root outlives every test that drives a run.
fn isolate_envelope_root() {
    static ROOT: std::sync::OnceLock<TempDir> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = TempDir::new().expect("an envelope temp root");
        std::env::set_var(gsd_meta_manager::envelope::ENVELOPE_ROOT_ENV, dir.path());
        dir
    });
}

/// A registry holding both projects, with `opted_in` the only one opted in.
fn config_for(a: &Path, b: &Path, opted_in: &str) -> Config {
    isolate_envelope_root();
    let mut config = Config::new();
    let alias_a = registry::Alias::new("a").expect("a visible test alias");
    let alias_b = registry::Alias::new("b").expect("a visible test alias");
    registry::add_project(&mut config, &alias_a, a).expect("project a registers");
    registry::add_project(&mut config, &alias_b, b).expect("project b registers");
    // Through the single construction site, never by building the record here:
    // a test that hand-rolls the opt-in is not testing the gate the user meets.
    registry::record_opt_in(&mut config, opted_in).expect("the opt-in records");
    config
}

/// Drive arguments pointed at the cwd-recording stand-in.
fn drive_args(alias: &str, sentinel: &Path) -> DriveArgs {
    DriveArgs {
        alias: nonblank(alias),
        command: Some(nonblank("/gsd-progress")),
        target_phase: None,
        max_steps: None,
        wall_clock_cap_secs: None,
        max_escalations: None,
        approved_plan: None,
        run_id: Some(nonblank(RUN_ID)),
        dry_run: false,
        goal: None,
        claude_program: Some(FAKE_CWD.into()),
        claude_args: vec![
            OsString::from(sentinel),
            OsString::from(CLEAN_BASELINE),
            OsString::from("0"),
        ],
    }
}

#[tokio::test]
async fn a_run_against_one_project_leaves_every_other_project_byte_identical() {
    let tmp = TempDir::new().expect("temp dir");
    let sentinel_dir = TempDir::new().expect("a third temp dir, under neither project");
    let sentinel = sentinel_dir.path().join("cwd");

    let a = make_project(tmp.path(), "alpha");
    let b = make_project(tmp.path(), "beta");
    let config = config_for(&a, &b, "a");

    // Captured before the run, and nothing touches B between here and the
    // comparison — not even a git read, which would refresh `.git/index`.
    let before = fingerprint_tree(&b);

    // An empty fingerprint compares equal to itself, so the comparison below
    // would pass vacuously if the walk silently found nothing. Pin what it must
    // have seen: the project's own file, and the `.git` a wrong-project git
    // operation would disturb.
    assert!(
        before
            .iter()
            .any(|(path, _, _)| path == ".planning/STATE.md"),
        "the walk must see the project's own files, got {} entries",
        before.len()
    );
    assert!(
        before.iter().any(|(path, _, _)| path.starts_with(".git/")),
        "the walk must descend into .git — that is where a git operation \
         against the wrong project would show"
    );

    drive(drive_args("a", &sentinel), &config)
        .await
        .expect("driving the opted-in project against the clean baseline completes");

    let after = fingerprint_tree(&b);
    assert_eq!(
        before,
        after,
        "a run against project A must leave project B byte-identical, \
         including its .git (criterion #4). Difference:\n{}",
        describe_difference(&before, &after)
    );

    // An independent second check through a different mechanism. A fingerprint
    // helper with a bug — a swallowed `read_dir` error, a wrong root — would
    // otherwise pass silently, and this is a safety test.
    assert_eq!(
        git_ops::is_dirty(&b),
        Some(false),
        "project B's working tree must still be clean after a run against A"
    );

    // And the run really did happen, so the assertions above are not vacuous.
    assert!(
        a.join(".planning").join("meta-manager").is_dir(),
        "the drive must actually have written its run directory under A"
    );
}

#[tokio::test]
async fn the_driven_agents_working_directory_is_the_driven_projects_root() {
    let tmp = TempDir::new().expect("temp dir");
    let sentinel_dir = TempDir::new().expect("a third temp dir, under neither project");
    let sentinel = sentinel_dir.path().join("cwd");

    let a = make_project(tmp.path(), "alpha");
    let b = make_project(tmp.path(), "beta");
    let config = config_for(&a, &b, "a");

    drive(drive_args("a", &sentinel), &config)
        .await
        .expect("driving the opted-in project completes");

    let raw = std::fs::read_to_string(&sentinel).expect("the stand-in records its own cwd");
    let recorded = PathBuf::from(raw.trim())
        .canonicalize()
        .expect("the recorded cwd exists");

    assert_eq!(
        recorded,
        a.canonicalize().expect("A exists"),
        "the driven agent's working directory is the driven project's root"
    );
    assert!(
        !recorded.starts_with(b.canonicalize().expect("B exists")),
        "the driven agent must never be started under another registered project"
    );

    // D-21 made mechanical. OQ4 resolved that `claude --worktree` exists and
    // works, and this phase deliberately declines it: the worktree lands at
    // `<repo>/.claude/worktrees/<name>`, inside the repository working
    // directory, where `watcher.rs::extract_project_root` would resolve its
    // copied `.planning/` as a phantom project and mis-route every file event.
    // Phase 19 owns the decision and the safe fallback (a worktree created
    // OUTSIDE the repo). This assertion is what fails loudly if someone adopts
    // the flag early.
    //
    // Anchored at the temp root rather than run over the absolute path, so the
    // assertion is about the driver's choice and not about whatever the host
    // happens to have set TMPDIR to.
    let under_tmp = recorded
        .strip_prefix(tmp.path().canonicalize().expect("the temp root exists"))
        .expect("the driven root is under the test's temp root")
        .display()
        .to_string();
    assert!(
        !under_tmp.contains("worktrees"),
        "the driven working directory must not be inside any worktree (D-21), got: {under_tmp}"
    );
}

#[tokio::test]
async fn a_refused_drive_writes_nothing_under_either_project() {
    let tmp = TempDir::new().expect("temp dir");
    let sentinel_dir = TempDir::new().expect("a third temp dir, under neither project");
    let sentinel = sentinel_dir.path().join("cwd");

    let a = make_project(tmp.path(), "alpha");
    let b = make_project(tmp.path(), "beta");
    // A is opted in; B is registered and deliberately is not.
    let config = config_for(&a, &b, "a");

    let before_a = fingerprint_tree(&a);
    let before_b = fingerprint_tree(&b);

    let err = drive(drive_args("b", &sentinel), &config)
        .await
        .expect_err("a project with no opt-in record must never be driven");
    assert!(
        err.to_string().contains('b'),
        "the refusal must name the alias so it is actionable, got: {err}"
    );

    let after_a = fingerprint_tree(&a);
    let after_b = fingerprint_tree(&b);

    // A refusal must be inert on EVERY project, not only on the one it refused.
    // That is the other end of criterion #4's sentence.
    assert_eq!(
        before_b,
        after_b,
        "the refused project must be untouched. Difference:\n{}",
        describe_difference(&before_b, &after_b)
    );
    assert_eq!(
        before_a,
        after_a,
        "the OTHER project must be untouched too. Difference:\n{}",
        describe_difference(&before_a, &after_a)
    );

    for root in [&a, &b] {
        let meta_manager = root.join(".planning").join("meta-manager");
        assert!(
            !meta_manager.exists(),
            "a refused drive writes nothing at all, but {} exists",
            meta_manager.display()
        );
    }
    assert!(
        !sentinel.exists(),
        "a refused drive spawns nothing, so the stand-in never ran"
    );
}
