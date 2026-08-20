// ============================================================================
// The two model seams, end to end: a stated goal becomes a validated plan, and
// the one state the deterministic rules do not cover gets a bounded
// consultation (DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08).
//
// **THE RULE THIS FILE IS WRITTEN UNDER, borrowed verbatim from
// `tests/driver_iteration_loop.rs`: no assertion may read a model's summary.**
// Every fact below is a record read back out of `journal.jsonl`, a field of
// `run.json`, or a file a stand-in program wrote as evidence that it ran. Not
// one is a sentence somebody wrote about the run.
//
// It is an integration test rather than an in-source one for two reasons. The
// first is the one `tests/driver_iteration_loop.rs` records: it spawns a genuine
// child process through a checked-in shell stand-in. The second is specific to
// this phase — `tests/spawn_seam_guard.rs` asserts that the decomposition
// capability's production constructor has exactly ONE executable call site under
// `src/`, so an in-source test calling it would be a second one, and widening
// that guard to forgive `#[cfg(test)]` modules would also forgive a production
// call site hidden behind a `cfg` a later refactor removed.
//
// Unix-only by construction: driving is a Unix capability (D-05) and the run
// body is `#[cfg(unix)]`. Debug-only in practice too — `--claude-program` has no
// parser entry in a release build (D-30, WR-16).
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::path::Path;

use gsd_meta_manager::config::{Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::run::GoalDecomposition;
use gsd_meta_manager::driver::{bounds, drive, escalate, goal, router, DriveArgs};
use gsd_meta_manager::error::DriveError;
use serde_json::Value;
use tempfile::TempDir;

/// The seam-aware stand-in. It answers a seam spawn from a caller-supplied
/// payload, replays a transcript for everything else, and leaves evidence of
/// every seam spawn on disk.
const SEAM_CLAUDE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-seam.sh"
);

/// The clean-success capture the iteration-loop tests already use for a GSD
/// command that changes nothing.
const CLEAN_BASELINE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/transcripts/01-success-textonly.ndjson"
);

const ALIAS: &str = "goalseam";

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

/// A project whose roadmap declares two phases, one of them already discussed
/// so the router's forward rule can fire on it.
fn project_root() -> TempDir {
    let root = TempDir::new().expect("temp dir");
    let planning = root.path().join(".planning");
    std::fs::create_dir_all(&planning).expect("scratch .planning");

    std::fs::write(
        planning.join("ROADMAP.md"),
        "# Roadmap\n\n\
         - [ ] **Phase 20: Deterministic Router** - the predecessor\n\
         - [ ] **Phase 21: Goal Layer** - the phase under test\n",
    )
    .expect("write ROADMAP.md");

    for (number, slug) in [(20, "deterministic-router"), (21, "goal-layer")] {
        let phase_dir = planning.join("phases").join(format!("{number}-{slug}"));
        std::fs::create_dir_all(&phase_dir).expect("scratch phase dir");
        std::fs::write(phase_dir.join(format!("{number}-CONTEXT.md")), "# Context\n")
            .expect("write phase context");
    }

    root
}

/// Point the envelope at a temp root for this test binary, exactly as
/// `tests/driver_iteration_loop.rs` does and for the same reason.
fn isolate_envelope_root() {
    static ROOT: std::sync::OnceLock<TempDir> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = TempDir::new().expect("an envelope temp root");
        std::env::set_var(gsd_meta_manager::envelope::ENVELOPE_ROOT_ENV, dir.path());
        dir
    });
}

fn config_for(root: &Path) -> Config {
    isolate_envelope_root();
    let mut config = Config::new();
    config.projects.insert(
        ALIAS.to_string(),
        RegisteredProject {
            path: root.to_path_buf(),
            added: "2026-08-19T12:00:00Z".to_string(),
            driver_opt_in: Some(DriverOptIn {
                opted_in_at: "2026-08-19T11:59:00Z".to_string(),
                claude_md_digest: None,
                prompt_inputs: gsd_meta_manager::registry::current_prompt_inputs(root),
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

/// The stand-in's working directory: where it records seam spawns, captures
/// what each seam was sent, and reads the payload it must answer with.
fn seam_workdir() -> TempDir {
    TempDir::new().expect("a seam workdir")
}

/// Write the structured payload the stand-in answers the **next** seam spawn
/// with. `seam-payload.N` wins over `seam-payload.json`, so a test can give two
/// consecutive consultations different answers.
fn plant_payload(workdir: &Path, payload: &Value) {
    std::fs::write(
        workdir.join("seam-payload.json"),
        serde_json::to_string(payload).expect("the payload serialises"),
    )
    .expect("write the seam payload");
}

/// How many seam spawns the stand-in recorded, read off disk.
///
/// **Observed through a program that left evidence when it ran**, never through
/// an in-process counter — the tripwire pattern `tests/driver_dry_run.rs`
/// established. A counter inside the driver would be a count of what the driver
/// believes it did.
fn seam_spawns(workdir: &Path) -> usize {
    std::fs::read_to_string(workdir.join("seam-spawns"))
        .map(|text| text.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or(0)
}

/// Everything the seam was sent on stdin, across every spawn, concatenated.
fn seam_stdin(workdir: &Path) -> String {
    let mut out = String::new();
    for entry in std::fs::read_dir(workdir).expect("the workdir is readable") {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with("seam-stdin.") {
            out.push_str(&std::fs::read_to_string(entry.path()).unwrap_or_default());
            out.push('\n');
        }
    }
    out
}

/// A goal-only `DriveArgs` pointed at the seam-aware stand-in.
fn goal_args(run_id: &str, workdir: &Path, stated_goal: &str) -> DriveArgs {
    DriveArgs {
        alias: ALIAS.to_string(),
        command: None,
        target_phase: None,
        max_steps: None,
        wall_clock_cap_secs: None,
        max_escalations: None,
        run_id: Some(run_id.to_string()),
        dry_run: false,
        goal: Some(stated_goal.to_string()),
        claude_program: Some(SEAM_CLAUDE.into()),
        claude_args: vec![
            OsString::from(workdir),
            OsString::from(CLEAN_BASELINE),
        ],
    }
}

/// One well-formed wire step.
fn step(command: &str, phase: &str) -> Value {
    serde_json::json!({
        goal::FIELD_COMMAND: command,
        goal::FIELD_PHASE: phase,
        goal::FIELD_TERMINAL_STATE: goal::TERMINAL_VERIFICATION_PASSED,
        goal::FIELD_RATIONALE: "because the phase has context and no plans",
    })
}

fn payload(steps: Vec<Value>) -> Value {
    serde_json::json!({ goal::FIELD_STEPS: steps })
}

fn journal_records(root: &Path, run_id: &str) -> Vec<Value> {
    let paths = gsd_meta_manager::journal::run_paths(&root.join(".planning"), run_id)
        .expect("the fixture run id is a plain path component");
    std::fs::read_to_string(&paths.journal)
        .expect("journal.jsonl exists and is readable")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("every journal line parses"))
        .collect()
}

// ---------------------------------------------------------------------------
// The capability's shape, exercised rather than only scanned
// ---------------------------------------------------------------------------

#[test]
fn the_decomposition_capability_exists_only_for_an_invocation_that_supplies_a_goal_alone() {
    let workdir = seam_workdir();

    // `--command` is already machine-checkable, so a goal beside it is recorded
    // prose and there is nothing to decompose.
    let mut with_command = goal_args("r", workdir.path(), "get phase 21 verified");
    with_command.command = Some("/gsd-progress".to_string());
    assert!(
        GoalDecomposition::from_argv_goal(&with_command).is_none(),
        "a goal supplied beside --command must not open a model seam: the \
         command source is already decided, and consulting a model to re-decide \
         it would be a seam firing where no ambiguity exists"
    );

    // `--target-phase` is Phase 20's goal primitive and is likewise already
    // machine-checkable.
    let mut with_target = goal_args("r", workdir.path(), "get phase 21 verified");
    with_target.target_phase = Some("21".to_string());
    assert!(
        GoalDecomposition::from_argv_goal(&with_target).is_none(),
        "a goal supplied beside --target-phase must not open a model seam"
    );

    // The DRIVE-01 invocation: a goal, and nothing else.
    let goal_only = goal_args("r", workdir.path(), "  get phase 21 verified  ");
    let capability = GoalDecomposition::from_argv_goal(&goal_only)
        .expect("a goal supplied alone is the invocation this capability exists for");
    assert_eq!(
        capability.goal(),
        "get phase 21 verified",
        "the goal is carried verbatim after trimming, because it is the one \
         trusted input and mangling it would change what was asked for"
    );

    // A blank goal is nothing to do, and must not become an empty prompt.
    let blank = goal_args("r", workdir.path(), "   \t ");
    assert!(
        GoalDecomposition::from_argv_goal(&blank).is_none(),
        "a whitespace-only goal is not a goal; decomposing it would ask the \
         model to invent one"
    );

    // And an invocation with no goal at all is every Phase 20 run.
    let mut none = goal_args("r", workdir.path(), "");
    none.goal = None;
    none.target_phase = Some("21".to_string());
    assert!(GoalDecomposition::from_argv_goal(&none).is_none());
}

// ---------------------------------------------------------------------------
// Seam one: the decomposition, once, above the run
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_goal_that_cannot_be_reduced_refuses_the_run_and_creates_nothing() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-unreducible";

    let root = project_root();
    let workdir = seam_workdir();
    // A terminal state that reads like a stopping condition and is not one. The
    // command and the phase are both legal, so the refusal below is about the
    // part that could not be reduced rather than about the plan's shape.
    plant_payload(
        workdir.path(),
        &serde_json::json!({ goal::FIELD_STEPS: [{
            goal::FIELD_COMMAND: router::COMMAND_PLAN_PHASE,
            goal::FIELD_PHASE: "21",
            goal::FIELD_TERMINAL_STATE: "when the maintainer is happy with it",
            goal::FIELD_RATIONALE: "prose",
        }]}),
    );

    let err = drive(
        goal_args(RUN_ID, workdir.path(), "make the maintainer happy"),
        &config_for(root.path()),
    )
    .await
    .expect_err("a goal with no machine-checkable stopping condition is refused");

    assert!(
        matches!(err, DriveError::GoalRefused(_)),
        "the refusal must carry the goal taxonomy rather than a fresh string, \
         got: {err:?}"
    );
    let rendered = err.to_string();
    assert!(
        rendered.contains("when the maintainer is happy with it"),
        "the refusal must NAME the part that could not be reduced, or the user \
         cannot act on it; got: {rendered}"
    );
    assert!(
        rendered.contains(goal::REASON_TERMINAL_STATE_NOT_REDUCIBLE),
        "and it must carry the taxonomy's own constant rather than a sentence \
         minted at the call site; got: {rendered}"
    );

    // A refused run has created NOTHING: no run directory, no journal, no
    // run.json, no lock file.
    assert!(
        !root.path().join(".planning/meta-manager").exists(),
        "a goal refused above the run must leave nothing at all behind — the \
         same property every other above-the-run refusal in `driver::drive` holds"
    );

    // And the seam really did fire, so the refusal is about the answer rather
    // than about a consultation that never happened.
    assert_eq!(
        seam_spawns(workdir.path()),
        1,
        "exactly one decomposition consultation, and the stand-in recorded it \
         on disk"
    );
}

#[tokio::test]
async fn a_stated_goal_becomes_a_recorded_plan_and_the_run_drives_its_terminal_phase() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-decomposed";

    let root = project_root();
    let workdir = seam_workdir();
    // Two steps, opening on the predecessor exactly as plan 21-01's live arms
    // did. The run must drive toward the LAST step's phase.
    plant_payload(
        workdir.path(),
        &payload(vec![
            step(router::COMMAND_PLAN_PHASE, "20"),
            step(router::COMMAND_PLAN_PHASE, "21"),
        ]),
    );

    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    // Two steps, which is the smallest cap that leaves an escalation budget to
    // spend: `escalate::resolve` reduces the unsupplied default to
    // `min(3, max_steps - 1)`, so a ONE-step run resolves to a budget of zero and
    // could not decompose at all. The run then halts on this cap rather than
    // driving an agent for the length of the test.
    args.max_steps = Some(2);

    drive(args, &config_for(root.path()))
        .await
        .expect("a legal plan starts the run");

    let records = journal_records(root.path(), RUN_ID);
    let plan_record = records
        .iter()
        .find(|record| {
            record["kind"] == "diagnostic"
                && record["code"] == "goal_plan_decomposed"
        })
        .expect("the decomposed plan reaches the journal before the first spawn");

    let detail = plan_record["detail"].as_str().expect("a rendered detail");
    assert!(
        detail.contains("steps=2"),
        "the record must name the plan it recorded; got: {detail}"
    );
    assert!(
        detail.contains("escalations_used=1"),
        "the decomposition counts against the run's model-consultation budget, \
         and it counts BEFORE it happens; got: {detail}"
    );
    assert!(
        detail.contains(&format!("command={} phase=21", router::COMMAND_PLAN_PHASE)),
        "every step reaches the record as typed key=value tokens; got: {detail}"
    );
    assert!(
        !detail.contains(&format!("{} 21", router::COMMAND_PLAN_PHASE)),
        "and NONE of it may read as a pasteable command line assembled from \
         model output (WR-09); got: {detail}"
    );

    // The run drove toward the plan's LAST step, not its first.
    let run_record = {
        let paths = gsd_meta_manager::journal::run_paths(&root.path().join(".planning"), RUN_ID)
            .expect("a plain run id");
        let raw = std::fs::read_to_string(&paths.run_json).expect("run.json is readable");
        serde_json::from_str::<Value>(&raw).expect("run.json parses")
    };
    assert_eq!(
        run_record["target_phase"], "21",
        "the phase a decomposed plan drives toward is its TERMINAL step's. \
         Against the unfixed behaviour — reading step zero — this is \"20\", the \
         prerequisite the plan merely passes through"
    );

    assert_eq!(
        seam_spawns(workdir.path()),
        1,
        "exactly ONE decomposition per run. A second would be a run taking a \
         goal from something other than the human who stated it"
    );
}

#[tokio::test]
async fn the_decomposition_seam_is_shown_no_bytes_read_from_a_project_file() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-noleak";

    let root = project_root();
    // A distinctive token planted in a `.planning/` file the seam has no
    // business reading. STATE.md is one of the five disclosed prompt inputs, so
    // if any file body were going to reach a seam this is the one.
    let planted = "CANARY-6f21a0b4-NEVER-IN-A-PROMPT";
    std::fs::write(
        root.path().join(".planning/STATE.md"),
        format!("# State\n\n{planted}\n"),
    )
    .expect("write STATE.md");

    let workdir = seam_workdir();
    plant_payload(
        workdir.path(),
        &payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]),
    );

    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    args.max_steps = Some(2);
    drive(args, &config_for(root.path()))
        .await
        .expect("a legal plan starts the run");

    let sent = seam_stdin(workdir.path());
    assert!(
        !sent.is_empty(),
        "the stand-in captured nothing, so the absence below proves nothing"
    );
    assert!(
        sent.contains("get the goal layer verified"),
        "the arrival assertion: the goal really did reach the seam, so the \
         absence of the canary is a fact about what was NOT sent rather than \
         about a prompt that never arrived"
    );
    assert!(
        !sent.contains(planted),
        "a byte read out of a project file reached the seam. The seam's input is \
         the goal text plus typed state tokens plus the enumerated third-party \
         strings inside one boundary — never a file body (SAFE-07). Sent: {sent}"
    );
}

#[tokio::test]
async fn a_seam_that_answers_with_nothing_parks_the_run_before_it_exists() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-unusable";

    let root = project_root();
    let workdir = seam_workdir();
    // No payload planted at all: the stand-in answers with a result envelope
    // carrying no `structured_output`, which is the transport's own shape for
    // "the model produced nothing usable".

    let err = drive(
        goal_args(RUN_ID, workdir.path(), "get the goal layer verified"),
        &config_for(root.path()),
    )
    .await
    .expect_err("a seam that answers with nothing must not start a run");

    assert!(
        matches!(
            err,
            DriveError::GoalSeamUnusable {
                reason: escalate::EscalationReason::OutputUnusable,
                ..
            }
        ),
        "the refusal must carry the escalation taxonomy's own reason rather than \
         a fresh string, got: {err:?}"
    );
    assert!(
        err.to_string()
            .contains(escalate::REASON_ESCALATION_OUTPUT_UNUSABLE),
        "the rendered message must carry the greppable constant; got: {err}"
    );
    assert!(
        !root.path().join(".planning/meta-manager").exists(),
        "nothing at all is created"
    );
    assert_eq!(
        seam_spawns(workdir.path()),
        1,
        "ONE consultation and no retry. Retrying a model that has just produced \
         an unusable answer is how a bounded seam becomes an unbounded one"
    );
}

#[tokio::test]
async fn a_run_whose_step_cap_leaves_no_room_to_escalate_refuses_the_decomposition() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-nobudget";

    let root = project_root();
    let workdir = seam_workdir();
    plant_payload(
        workdir.path(),
        &payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]),
    );

    // `escalate::resolve` reduces the unsupplied default to what the step cap
    // leaves room for, so a one-step run resolves to a budget of ZERO. The
    // decomposition asks first and is refused, and the model is never spawned.
    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    args.max_steps = Some(1);

    let err = drive(args, &config_for(root.path()))
        .await
        .expect_err("a run with no consultations left cannot decompose a goal");

    assert!(
        matches!(
            err,
            DriveError::GoalSeamUnusable {
                reason: escalate::EscalationReason::CapReached,
                ..
            }
        ),
        "got: {err:?}"
    );
    assert_eq!(
        seam_spawns(workdir.path()),
        0,
        "the budget is asked BEFORE the consultation, so a refused permission \
         means the model was never spawned at all. Against a check performed \
         afterwards this is 1: the tokens are spent and the cap is a report \
         rather than a control"
    );
    assert_eq!(
        bounds::resolve(Some(1), None)
            .expect("a one-step run resolves")
            .max_steps,
        1,
        "the fixture's premise: the run really is bounded at one step"
    );
}
