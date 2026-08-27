// ============================================================================
// Path traversal through a run id, both directions (D-27, WR-02).
//
// This file exists because Phase 17's review **reproduced** both directions
// against the shipped tree rather than theorising them:
//
//   * the **write** side — `drive --run-id '../../../../escaped'` created
//     `run.json` and `journal.jsonl` in a directory outside the project, with no
//     `.gitignore`, and exited 0;
//   * the **read** side — `writer::read_active_run` returned whatever the
//     `active` file held, guarded only by an `is_dir()` check that a traversing
//     path satisfies. That file lives inside the driven project, so **the agent
//     controls it**, after which `reconcile_one` read `run.json` from anywhere
//     and `App::schedule_journal_tail` tailed anything.
//
// This subsystem runs unattended with git and push rights, which is why the fix
// is a fallible `journal::run_paths` — an `Option` conscripts the compiler into
// enumerating every caller, where a validation helper that merely *existed*
// would have been called at three of the five sites.
//
// **Both tests below fail against the pre-fix tree**, and each says how in its
// own doc. They are integration tests because they drive real entry points —
// `driver::drive`, `writer::read_active_run`, `reconcile::reconcile_one` —
// against real directories, and the property under test is what does or does not
// appear on a filesystem.
// ============================================================================

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gsd_meta_manager::config::{Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::{drive, reconcile, DriveArgs};
use gsd_meta_manager::error::DriveError;
use gsd_meta_manager::journal::{writer, RunRecord};
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


const ALIAS: &str = "victim";
const COMMAND: &str = "/gsd-progress";
const PLAIN_RUN_ID: &str = "2026-07-29T21-40-02Z-3f2a";

/// Run ids that must never reach a `join`.
///
/// The first is the review's verbatim reproduction; the rest are its near
/// neighbours, because a fix that closed only the exact string reproduced would
/// be a fix for a test rather than for the hole.
const HOSTILE: &[&str] = &[
    "../../../../escaped",
    "../escaped",
    "..",
    "nested/escaped",
    "/tmp/absolute-escaped",
    "./escaped",
];

/// A sandbox holding the driven project as a **subdirectory**, so a traversal
/// out of the runs root lands somewhere this test can still observe.
///
/// `../../../../escaped` from `<project>/.planning/meta-manager/runs/` resolves
/// to a **sibling of the project**. A fixture rooted at the project itself would
/// therefore watch the wrong side of the boundary and pass regardless.
struct Sandbox {
    dir: TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let dir = TempDir::new().expect("temp dir");
        std::fs::create_dir_all(dir.path().join("project/.planning")).expect("scratch .planning");
        Sandbox { dir }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn project(&self) -> PathBuf {
        self.dir.path().join("project")
    }

    fn planning(&self) -> PathBuf {
        self.project().join(".planning")
    }

    fn config(&self) -> Config {
        let mut config = Config::new();
        config.projects.insert(
            ALIAS.to_string(),
            RegisteredProject {
                path: self.project(),
                added: "2026-07-29T12:00:00Z".to_string(),
                driver_opt_in: Some(DriverOptIn {
                    opted_in_at: "2026-07-29T11:59:00Z".to_string(),
                    claude_md_digest: None,
                    prompt_inputs: gsd_meta_manager::registry::current_prompt_inputs(&self.project()),
                    extra: Default::default(),
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
}

/// Every path under `root`, relative and sorted — the whole filesystem footprint
/// of the sandbox as one comparable value.
///
/// A walk rather than a handful of `exists()` checks, deliberately: the hole was
/// that a *joined* path escaped, and nobody can enumerate in advance every place
/// a `join` might land. Comparing the entire tree before and after is the only
/// assertion that does not require guessing where the damage would appear.
fn footprint(root: &Path) -> BTreeSet<PathBuf> {
    let mut found = BTreeSet::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            found.insert(
                path.strip_prefix(root)
                    .expect("every walked path is under the walk root")
                    .to_path_buf(),
            );
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                stack.push(path);
            }
        }
    }
    found
}

fn drive_args(run_id: &str) -> DriveArgs {
    DriveArgs {
        alias: nonblank(ALIAS),
        command: Some(nonblank(COMMAND)),
        target_phase: None,
        max_steps: None,
        wall_clock_cap_secs: None,
        max_escalations: None,
        approved_plan: None,
        run_id: Some(nonblank(run_id)),
        dry_run: false,
        goal: None,
        // Deliberately absent. A refused run must never reach a spawn, so a
        // stand-in program here would only be able to prove that it *was* not
        // reached — and if the refusal regressed, the test would spawn a real
        // `claude` instead of failing.
        claude_program: None,
        claude_args: Vec::new(),
    }
}

/// A `run.json` a reconciliation scan would happily believe.
///
/// No `ended_at`, which is the crash signal — so `reconcile_one` surfaces the
/// run rather than short-circuiting on an ending. That makes the negative test
/// below meaningful: the fixture is one the pre-fix code path *would* have
/// returned.
fn plausible_record(run_id: &str) -> String {
    let record = RunRecord {
        // Phase 21's three Run-scoped fields, absent on a fixture that
        // predates them.
        approved_plan: None,
        escalation_cap: None,
        escalations_used: None,
        run_id: run_id.to_string(),
        goal: "a record the scan must never reach".to_string(),
        gsd_command: COMMAND.to_string(),
        target: "claude".to_string(),
        opt_in: None,
        started_at: "2026-07-29T21:40:02Z".to_string(),
        session_id: "9f1c0e2a-0000-4000-8000-000000000000".to_string(),
        pid: std::process::id(),
        pgid: std::process::id(),
        claude_code_version: "2.1.220".to_string(),
        argv_digest: "fnv1a64:0000000000000000".to_string(),
        ended_at: None,
        outcome: None,
        target_phase: None,
        bounds: None,
        extra: serde_json::Map::new(),
    };
    serde_json::to_string_pretty(&record).expect("the record serialises")
}

/// WR-02, write side.
///
/// **Against the pre-fix tree this fails on the very first assertion**, because
/// `drive` returned `Ok(())` for a traversing id and `run_paths` joined it
/// unvalidated: the review observed `run.json` and `journal.jsonl` landing in a
/// sibling of the project, in a directory with no `.gitignore`, with exit 0.
/// Both halves are asserted here — the non-zero refusal *and* the untouched
/// filesystem — because either one alone can be satisfied by a fix that is only
/// half made.
#[tokio::test]
async fn a_traversing_run_id_creates_nothing_outside_the_runs_root_and_exits_non_zero() {
    let sandbox = Sandbox::new();
    let config = sandbox.config();
    let before = footprint(sandbox.root());

    for hostile in HOSTILE {
        let error = drive(drive_args(hostile), &config)
            .await
            .expect_err("a traversing run id must refuse rather than run");

        assert!(
            // COMPARISON only, not expectation: `21-24` retyped `run_id` to
            // `crate::text::Untrusted`, which deliberately implements no
            // `PartialEq<&str>`. The demand is unchanged — the refusal must
            // carry the offending id BYTE-FOR-BYTE, which is what
            // `as_raw_for_logic_only` reads. Escaping here would have weakened
            // the assertion to "carries something like the id".
            matches!(&error, DriveError::RunIdInvalid { run_id } if run_id.as_raw_for_logic_only() == *hostile),
            "the refusal must be the typed one that names the offending id, so the \
             caller can act on it; got {error:?} for {hostile:?}"
        );
        assert!(
            error.to_string().contains("not a single directory name"),
            "a refusal a caller cannot act on is a bug report rather than an error \
             message; got {error}"
        );

        assert_eq!(
            footprint(sandbox.root()),
            before,
            "{hostile:?} changed the filesystem. A refused run must create nothing \
             at all: no lock file, no run directory, no run.json, no journal — and \
             above all nothing outside .planning/meta-manager/runs/"
        );
    }

    assert!(
        !sandbox.root().join("escaped").exists(),
        "the sibling directory the review's reproduction created must not exist"
    );
    assert!(
        !sandbox.planning().join("meta-manager").exists(),
        "a refused run must not even bring the runs root into being"
    );
}

/// WR-02, read side — the half that matters more, because the `active` file
/// lives inside the driven project and the agent writes it.
///
/// **Against the pre-fix tree both refusals fail**: `read_active_run` was
/// guarded only by `is_dir()`, which a traversing path satisfies, so it returned
/// the planted id and `reconcile_one` went on to read the planted `run.json`
/// from outside the runs root and surface it as an observed run.
///
/// The positive control at the end is what stops this test passing vacuously. It
/// drives the *same* code path with the *same* fixture shape and only the id's
/// **shape** changed, so a build that had simply broken reconciliation would
/// fail there rather than sailing through the two negatives.
#[test]
fn an_active_pointer_naming_a_traversing_id_causes_no_read_outside_the_runs_root() {
    let sandbox = Sandbox::new();
    let planning = sandbox.planning();
    let runs_root = planning.join("meta-manager").join("runs");
    std::fs::create_dir_all(&runs_root).expect("create the runs root");

    // The bait: a well-formed record a scan would believe, one level above the
    // project — exactly where `../../../../planted` from the runs root lands.
    let planted = sandbox.root().join("planted");
    std::fs::create_dir_all(&planted).expect("create the planted directory");
    std::fs::write(planted.join("run.json"), plausible_record("planted"))
        .expect("write the planted record");

    let traversing = "../../../../planted";
    std::fs::write(runs_root.join("active"), format!("{traversing}\n"))
        .expect("write the hostile active pointer");

    // The fixture's own premise, asserted rather than assumed: the bait really
    // is reachable by a naive join, and really does parse. Without this, a typo
    // in the traversal depth would make both refusals below pass for the wrong
    // reason.
    assert!(
        runs_root.join(traversing).is_dir(),
        "the planted directory must satisfy the is_dir() check that used to be \
         the only guard, or this test proves nothing"
    );
    assert!(
        serde_json::from_str::<RunRecord>(
            &std::fs::read_to_string(planted.join("run.json")).expect("read the bait")
        )
        .is_ok(),
        "the planted record must be one a scan would believe"
    );

    assert_eq!(
        writer::read_active_run(&planning),
        None,
        "a pointer naming a path that is not a single directory component must be \
         refused, not followed — the agent writes this file"
    );
    assert_eq!(
        reconcile::reconcile_one(ALIAS, &sandbox.project()),
        None,
        "no ObservedRun may be built from a record outside the runs root; surfacing \
         one would put an arbitrary file's contents on the dashboard and hand the \
         kill switch a pid the agent chose"
    );

    // --- Positive control: the same fixture with a plain id must still work ---
    let real = runs_root.join(PLAIN_RUN_ID);
    std::fs::create_dir_all(&real).expect("create a real run directory");
    std::fs::write(real.join("run.json"), plausible_record(PLAIN_RUN_ID))
        .expect("write the real record");
    std::fs::write(runs_root.join("active"), format!("{PLAIN_RUN_ID}\n"))
        .expect("point at the real run");

    assert_eq!(
        writer::read_active_run(&planning).as_deref(),
        Some(PLAIN_RUN_ID),
        "a plain run id must still be followed, or the refusal above is just a \
         broken reconciliation scan wearing a security fix's clothes"
    );
    let observed = reconcile::reconcile_one(ALIAS, &sandbox.project())
        .expect("a run inside the runs root is still observed");
    assert_eq!(observed.run_id, PLAIN_RUN_ID);
    assert_eq!(observed.goal, "a record the scan must never reach");
}
