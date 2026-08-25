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
use gsd_meta_manager::driver::{bounds, drive, escalate, goal, router, DriveArgs, RawDriveArgs};
use gsd_meta_manager::error::DriveError;
use gsd_meta_manager::journal::{ApprovalRefusal, ApprovalTokenError};
use serde_json::Value;
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

/// The RAW goal-only invocation, before the parse boundary has judged it.
///
/// It exists so a test can hand `DriveArgs::from_argv` a payload the judged type
/// cannot hold — a blank `--goal`, say — and assert the refusal. `goal_args`
/// below is this record put through the boundary.
fn raw_goal_args(run_id: &str, workdir: &Path, stated_goal: &str) -> RawDriveArgs {
    RawDriveArgs {
        alias: ALIAS.to_string(),
        command: None,
        target_phase: None,
        max_steps: None,
        wall_clock_cap_secs: None,
        max_escalations: None,
        approved_plan: None,
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

/// A goal-only `DriveArgs` pointed at the seam-aware stand-in.
///
/// Built through the **production** parse boundary rather than by a struct
/// literal, so the fixtures exercise the same conversion `src/main.rs` performs.
fn goal_args(run_id: &str, workdir: &Path, stated_goal: &str) -> DriveArgs {
    DriveArgs::from_argv(raw_goal_args(run_id, workdir, stated_goal))
        .expect("the fixture invocation is well-formed")
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

/// The same step with a caller-chosen rationale, so a test can vary the one
/// field the digest deliberately excludes and nothing else.
fn step_with_rationale(command: &str, phase: &str, rationale: &str) -> Value {
    serde_json::json!({
        goal::FIELD_COMMAND: command,
        goal::FIELD_PHASE: phase,
        goal::FIELD_TERMINAL_STATE: goal::TERMINAL_VERIFICATION_PASSED,
        goal::FIELD_RATIONALE: rationale,
    })
}

fn payload(steps: Vec<Value>) -> Value {
    serde_json::json!({ goal::FIELD_STEPS: steps })
}

/// The roadmap phases the fixture project declares.
const PHASES: &[&str] = &["20", "21"];

/// The run's resolved step cap for a fixture that supplies no `--max-steps`.
fn resolved_cap() -> u32 {
    bounds::resolve(None, None)
        .expect("the default bounds resolve")
        .max_steps
}

/// A legal plan built the way the driver builds one: through the shipped
/// `goal::legality`, never by constructing `PlanStep` values a test invented.
fn plan_from(wire: &Value) -> goal::GoalPlan {
    goal::legality(wire, PHASES, resolved_cap()).expect("the fixture plan is legal")
}

/// The approval TOKEN a reviewer would be shown for `wire`, against `root`'s
/// disclosed files as they stand right now: both halves in one value.
///
/// **Composed the way the driver composes it**, through the shipped
/// `goal::legality`, `goal::plan_digest`, `journal::approval_digest` and
/// `journal::render_approval_token`, so this helper cannot agree with a test
/// while disagreeing with the run — and so no test in this file assembles a
/// token by string concatenation, which would be a second spelling of the
/// renderer and therefore a second thing that can be wrong about the order.
fn approval_for(root: &Path, wire: &Value, max_steps: Option<u32>) -> String {
    let cap = bounds::resolve(max_steps, None)
        .expect("the fixture's bounds resolve")
        .max_steps;
    let plan = goal::legality(wire, PHASES, cap).expect("the fixture plan is legal");
    let plan_digest = goal::plan_digest(&plan);
    let approval_digest = gsd_meta_manager::journal::approval_digest(
        &plan_digest,
        &gsd_meta_manager::registry::current_prompt_inputs(root),
    );
    gsd_meta_manager::journal::render_approval_token(&plan_digest, &approval_digest)
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
    with_command.command = Some(nonblank("/gsd-progress"));
    assert!(
        GoalDecomposition::from_argv_goal(&with_command).is_none(),
        "a goal supplied beside --command must not open a model seam: the \
         command source is already decided, and consulting a model to re-decide \
         it would be a seam firing where no ambiguity exists"
    );

    // `--target-phase` is Phase 20's goal primitive and is likewise already
    // machine-checkable.
    let mut with_target = goal_args("r", workdir.path(), "get phase 21 verified");
    with_target.target_phase = Some(nonblank("21"));
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

    // **A blank goal is nothing to do, and it is now UNREPRESENTABLE here
    // rather than filtered here** (21-15). This row used to build
    // `goal_args(.., "   \t ")` and assert the capability declined it, which
    // meant a blank goal travelled all the way into a `DriveArgs` and was caught
    // by an `is_empty` branch inside `from_argv_goal`. `DriveArgs::goal` is now
    // an `Option<payload::NonBlank>`: there is no such value to build, and the
    // refusal happens at the parse boundary before a `DriveArgs` exists. The
    // assertion moves with the code rather than being deleted — this is the same
    // fact, checked one layer earlier and for every degenerate shape rather than
    // for the one this test happened to spell.
    // The list is `test_support::DEGENERATE` since 21-17 dropped its cfg gate:
    // four hand-picked shapes became all six, and every one of the six is a
    // valid fixture for this assertion — each carries nothing visible, so each
    // must reach `NoCommandSource` at the parse boundary. No exception, so no
    // filter.
    for blank in gsd_meta_manager::test_support::DEGENERATE {
        assert!(
            matches!(
                DriveArgs::from_argv(raw_goal_args("r", workdir.path(), blank)),
                Err(DriveError::NoCommandSource)
            ),
            "a goal carrying nothing visible is not a goal; decomposing it would \
             ask the model to invent one. blank={blank:?}"
        );
    }

    // And an invocation with no goal at all is every Phase 20 run.
    let mut none = goal_args("r", workdir.path(), "a goal that is then removed");
    none.goal = None;
    none.target_phase = Some(nonblank("21"));
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
    let wire = payload(vec![
        step(router::COMMAND_PLAN_PHASE, "20"),
        step(router::COMMAND_PLAN_PHASE, "21"),
    ]);
    plant_payload(workdir.path(), &wire);

    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    // Two steps, which is the smallest cap that leaves an escalation budget to
    // spend: `escalate::resolve` reduces the unsupplied default to
    // `min(3, max_steps - 1)`, so a ONE-step run resolves to a budget of zero and
    // could not decompose at all. The run then halts on this cap rather than
    // driving an agent for the length of the test.
    args.max_steps = Some(2);
    args.approved_plan = Some(nonblank(&approval_for(root.path(), &wire, args.max_steps)));

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
    let wire = payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]);
    plant_payload(workdir.path(), &wire);

    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    args.max_steps = Some(2);
    args.approved_plan = Some(nonblank(&approval_for(root.path(), &wire, args.max_steps)));
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

// ---------------------------------------------------------------------------
// The approval, bound to the plan AND the bytes, re-checked at spawn
// ---------------------------------------------------------------------------

/// The token a user would copy, lifted out of the refusal that printed it.
///
/// **Extracted rather than recomputed, and that is the point of the tests that
/// use it.** Recomputing the token in a test proves the test agrees with the
/// helper; lifting it out of the rendered refusal proves the value the user is
/// *shown* is the value the run then *checks*. Those are different claims, and
/// only the second one is about the review flow.
fn token_from_refusal(rendered: &str) -> String {
    const FLAG: &str = "--approved-plan ";
    let after = rendered
        .split_once(FLAG)
        .unwrap_or_else(|| {
            panic!("the refusal must name `{FLAG}` and the token to pass; got: {rendered}")
        })
        .1;
    let token: String = after.chars().take_while(|c| *c != '`').collect();
    assert!(
        !token.is_empty(),
        "the refusal named the flag but printed no token after it; got: {rendered}"
    );
    token
}

#[tokio::test]
async fn a_re_decomposition_that_changed_the_plan_is_refused_as_a_changed_plan_not_as_changed_files()
{
    const REVIEW_RUN_ID: &str = "2026-08-19T12-00-00Z-planchanged-review";
    const RUN_ID: &str = "2026-08-19T12-00-00Z-planchanged";

    let root = project_root();
    let config = config_for(root.path());
    let workdir = seam_workdir();

    // **The review flow, exactly as a user performs it.** Run once with no
    // token to obtain one, then re-run with it. The second run re-decomposes
    // the goal through a non-deterministic model, so a different answer is the
    // EXPECTED case rather than an exotic one — which is precisely why the
    // refusal it produces has to name the right cause.
    let reviewed = payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]);
    plant_payload(workdir.path(), &reviewed);

    let mut review = goal_args(REVIEW_RUN_ID, workdir.path(), "get the goal layer verified");
    review.max_steps = Some(2);
    review.approved_plan = None;
    let required = drive(review, &config)
        .await
        .expect_err("a goal run with no approval prints the plan and its token");
    let token = token_from_refusal(&required.to_string());

    // The model answers differently the second time: a two-step plan over the
    // same roadmap, legal under the same cap.
    let answered = payload(vec![
        step(router::COMMAND_PLAN_PHASE, "20"),
        step(router::COMMAND_PLAN_PHASE, "21"),
    ]);
    plant_payload(workdir.path(), &answered);

    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    args.max_steps = Some(2);
    args.approved_plan = Some(nonblank(&token));

    let err = drive(args, &config)
        .await
        .expect_err("a plan the approval never covered must not run");

    assert!(
        matches!(
            err,
            DriveError::PlanApprovalStale(ApprovalRefusal::PlanChanged { .. })
        ),
        "**this is the defect.** `approve_plan` used to compare the freshly \
         observed plan digest against ITSELF, so `PlanChanged` was unreachable \
         from production and every real mismatch fell through to \
         `DisclosedFilesChanged`. Against that build this reads \
         `DisclosedFilesChanged`, and the user is sent looking for a `git pull` \
         that never happened; got: {err:?}"
    );

    // The message defect is half of WR-01, so the rendered bytes are asserted
    // on as well as the variant: a user reads the sentence, not the enum.
    let rendered = err.to_string();
    assert!(
        !rendered.contains("the plan is unchanged"),
        "the refusal must not assert a fact the code just disproved — the plan \
         is exactly what changed; got: {rendered}"
    );
    assert!(
        rendered.contains("is not the plan that was approved"),
        "and it must say so in the words the plan-changed arm owns; got: \
         {rendered}"
    );

    // Both plan digests are named, so the refusal is diagnosable rather than
    // merely correct. Recomputed here only to check the MESSAGE — the token
    // above came from the refusal, which is the claim under test.
    let approved_digest = goal::plan_digest(&plan_from(&reviewed));
    let observed_digest = goal::plan_digest(&plan_from(&answered));
    assert_ne!(
        approved_digest, observed_digest,
        "the fixture's premise: the two plans really do have different digests, \
         or the assertion below cannot tell the arms apart"
    );
    assert!(
        rendered.contains(&approved_digest) && rendered.contains(&observed_digest),
        "the refusal must name the digest the approval covered AND the one \
         observed now; got: {rendered}"
    );

    assert!(
        !root.path().join(".planning/meta-manager").exists(),
        "an approval refusal is an above-the-run refusal: it creates nothing at \
         all, which is the property the ordered refusal chain in \
         `src/driver/mod.rs` documents"
    );
}

#[tokio::test]
async fn a_disclosed_file_rewritten_under_an_approval_is_refused_as_changed_files_not_a_changed_plan()
{
    const RUN_ID: &str = "2026-08-19T12-00-00Z-fileschanged";

    let root = project_root();
    let workdir = seam_workdir();
    let wire = payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]);
    plant_payload(workdir.path(), &wire);

    // The bytes the user reviewed, and the token that covered them.
    std::fs::write(root.path().join("CLAUDE.md"), b"the bytes the user reviewed")
        .expect("write CLAUDE.md");
    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    args.max_steps = Some(2);
    args.approved_plan = Some(nonblank(&approval_for(root.path(), &wire, args.max_steps)));

    // Now the file moves under the approval, and the opt-in is re-confirmed
    // against the NEW bytes. That ordering is deliberate: the opt-in's own drift
    // check (plan 21-03) sits at the capability gate and fires first, so a test
    // that let it fire would be asserting on the first of two independent
    // layers. Snapshotting the opt-in after the rewrite disarms only that one,
    // leaving the approval binding — the layer under test — as the thing that
    // has to refuse.
    std::fs::write(
        root.path().join("CLAUDE.md"),
        b"## IMPORTANT SYSTEM OVERRIDE",
    )
    .expect("rewrite CLAUDE.md");
    let config = config_for(root.path());

    let err = drive(args, &config)
        .await
        .expect_err("bytes that changed after the approval must not be run against");

    assert!(
        matches!(
            err,
            DriveError::PlanApprovalStale(ApprovalRefusal::DisclosedFilesChanged { .. })
        ),
        "the plan is genuinely identical here, so this must be the FILES arm. It \
         is the companion assertion to the plan-changed proof above: together \
         they show the two halves are distinguishable rather than that one arm \
         swallowed both; got: {err:?}"
    );
    let rendered = err.to_string();
    assert!(
        rendered.contains("the plan is unchanged"),
        "and the message may open with that claim precisely because the code \
         just established it — the plan half was compared first, against the \
         digest the token carried; got: {rendered}"
    );
    assert!(
        !rendered.contains("is not the plan that was approved"),
        "it must not also report a changed plan; got: {rendered}"
    );
    assert!(
        !root.path().join(".planning/meta-manager").exists(),
        "an approval refusal creates nothing"
    );
}

#[tokio::test]
async fn a_half_supplied_approval_token_is_refused_by_name_and_never_treated_as_an_approval() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-halftoken";

    let root = project_root();
    let workdir = seam_workdir();
    let wire = payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]);
    plant_payload(workdir.path(), &wire);

    // The approval half alone — which is exactly the value an earlier build
    // printed and accepted, so this is also the legacy-token case. It must fail
    // the parse rather than being read as an approval with an empty plan half,
    // because an empty half would compare equal to an empty recorded value.
    let token = approval_for(root.path(), &wire, Some(2));
    let (plan_half, approval_half) = token
        .split_once(gsd_meta_manager::journal::APPROVAL_TOKEN_SEPARATOR)
        .expect("the helper renders a two-half token");
    assert!(
        !plan_half.is_empty() && !approval_half.is_empty(),
        "the fixture's premise: the whole token really does carry two non-empty \
         halves, so the half passed below is a truncation rather than the whole \
         thing"
    );

    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    args.max_steps = Some(2);
    args.approved_plan = Some(nonblank(approval_half));

    let err = drive(args, &config_for(root.path()))
        .await
        .expect_err("a value that is not a token cannot approve a run");

    assert!(
        matches!(
            err,
            DriveError::PlanApprovalMalformed(ApprovalTokenError::SeparatorAbsent)
        ),
        "a half token is refused BY NAME. It is neither an absent approval — the \
         caller did supply something, and telling them nobody approved it sends \
         them to the wrong fix — nor a stale one, and above all it is not a \
         partial approval that starts a run; got: {err:?}"
    );
    let rendered = err.to_string();
    assert!(
        rendered.contains("--approved-plan"),
        "and the refusal names the flag, or it is a bug report rather than an \
         error message; got: {rendered}"
    );
    assert!(
        !root.path().join(".planning/meta-manager").exists(),
        "a malformed token creates nothing at all"
    );

    // **The one line that gives this test the ordering-detection `21-REVIEW.md`
    // says it lacks.** It proved the refusal and could not see that a live model
    // consultation had already been spent reaching it: the harness plants a
    // payload, so the decomposition succeeded and only then was the parse
    // reached. A refusal that costs a consultation is a refusal an attacker who
    // can influence the launching command line — or a user with a fat finger —
    // can bill to the run's budget for free (review-CR-01, T-21-11-03).
    assert_eq!(
        seam_spawns(workdir.path()),
        0,
        "a value that is not a token must be refused by a PURE string check, \
         above the seam: zero spawns, read off the stand-in's own on-disk ledger"
    );
}

/// **review-CR-01, as the reproduction `21-VERIFICATION.md` performed by hand.**
///
/// The verifier ran a real goal invocation with a garbage `--approved-plan` and
/// watched it return `PlanApprovalMalformed(SeparatorAbsent)` **after exactly one
/// seam spawn was recorded on disk**, then ran `--dry-run` with the same token
/// and watched it exit `Ok(())` with a clean preview that never mentioned the
/// token at all. Against that build the first arm below FAILS on the spawn count
/// and the second FAILS on `expect_err`.
///
/// **A payload IS planted in every arm**, so the seam would answer if it were
/// reached. That is what makes the zeroes a fact about ordering rather than about
/// a fixture that could not have spawned — and the well-formed control arm, which
/// must record exactly one spawn against its own workdir, is what makes the same
/// point from the other direction.
#[tokio::test]
async fn a_malformed_approval_token_is_refused_before_the_seam_is_spawned_and_identically_in_preview(
) {
    const GARBAGE: &str = "total-garbage-no-separator";
    const STATED_GOAL: &str = "get the goal layer verified";

    let wire = payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]);

    // ---- Arm one: a real run. Zero spawns, not one. ----
    {
        const RUN_ID: &str = "2026-08-19T12-00-00Z-garbagereal";
        let root = project_root();
        let workdir = seam_workdir();
        plant_payload(workdir.path(), &wire);

        let mut args = goal_args(RUN_ID, workdir.path(), STATED_GOAL);
        args.max_steps = Some(2);
        args.approved_plan = Some(nonblank(GARBAGE));

        let err = drive(args, &config_for(root.path()))
            .await
            .expect_err("a value that is not a token cannot approve a run");

        assert!(
            matches!(
                err,
                DriveError::PlanApprovalMalformed(ApprovalTokenError::SeparatorAbsent)
            ),
            "refused by name; got: {err:?}"
        );
        assert_eq!(
            seam_spawns(workdir.path()),
            0,
            "the refusal is a pure string check and must cost NO process spawn \
             and NO model consultation out of the run's budget — this count was \
             1 against the build that shipped (T-21-11-03)"
        );
        assert!(
            !root.path().join(".planning/meta-manager").exists(),
            "and it creates nothing at all"
        );
    }

    // ---- Arm two: the preview, answering identically. ----
    {
        const RUN_ID: &str = "2026-08-19T12-00-00Z-garbagepreview";
        let root = project_root();
        let workdir = seam_workdir();
        plant_payload(workdir.path(), &wire);

        let mut args = goal_args(RUN_ID, workdir.path(), STATED_GOAL);
        args.max_steps = Some(2);
        args.approved_plan = Some(nonblank(GARBAGE));
        args.dry_run = true;

        let err = drive(args, &config_for(root.path()))
            .await
            .expect_err("a preview must refuse exactly what the real run refuses");

        assert!(
            matches!(
                err,
                DriveError::PlanApprovalMalformed(ApprovalTokenError::SeparatorAbsent)
            ),
            "the SAME typed refusal, in the same words. A preview that refuses \
             less than the run it previews is previewing something the user \
             cannot run, and the person rehearsing a run on an unfamiliar \
             repository is exactly the person a preview exists for (WR-09); \
             got: {err:?}"
        );
        assert_eq!(
            seam_spawns(workdir.path()),
            0,
            "a preview spawns nothing regardless (D-23), but the refusal is what \
             this arm is about"
        );
        assert!(
            !root.path().join(".planning/meta-manager").exists(),
            "a preview creates no run directory"
        );
    }

    // ---- Arm three: the control. Without it the zeroes prove nothing. ----
    {
        const RUN_ID: &str = "2026-08-19T12-00-00Z-garbagecontrol";
        let root = project_root();
        let workdir = seam_workdir();
        plant_payload(workdir.path(), &wire);

        let mut args = goal_args(RUN_ID, workdir.path(), STATED_GOAL);
        args.max_steps = Some(2);
        args.approved_plan = Some(nonblank(&approval_for(root.path(), &wire, Some(2))));

        // The run's own outcome is not this test's subject — a well-formed token
        // reaches the decomposition, which is the point. What matters is that
        // the stand-in DID spawn for this workdir, so the two zeroes above are
        // facts about ordering rather than about a stand-in that never runs.
        let _ = drive(args, &config_for(root.path())).await;

        assert_eq!(
            seam_spawns(workdir.path()),
            1,
            "a well-formed token must still reach the seam and spend exactly the \
             one consultation the decomposition legitimately needs — if this were \
             0 the fixture would be incapable of spawning and the arms above \
             would be vacuous"
        );
    }
}

#[tokio::test]
async fn a_goal_run_with_no_recorded_approval_refuses_and_says_the_approval_is_absent() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-unapproved";

    let root = project_root();
    let workdir = seam_workdir();
    let wire = payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]);
    plant_payload(workdir.path(), &wire);

    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    args.max_steps = Some(2);
    // No `--approved-plan`. **Absence of a recorded approval is a refusal,
    // never a default yes**, and a timeout into one is not expressible: there is
    // no clock on this path at all.
    args.approved_plan = None;

    let err = drive(args, &config_for(root.path()))
        .await
        .expect_err("a goal-driven run with no approval must never spawn");

    assert!(
        matches!(err, DriveError::PlanApprovalRequired { .. }),
        "the refusal must say the approval is ABSENT rather than reporting a \
         mismatch — 'nobody approved this' and 'what was approved has changed' \
         are different statements to the person reading it; got: {err:?}"
    );
    let rendered = err.to_string();
    assert!(
        rendered.contains("--approved-plan sha256:"),
        "and it must name the digest to approve, so the refusal is actionable \
         in one step; got: {rendered}"
    );
    // **The refusal IS the review surface.** DRIVE-03 requires the user to
    // review the plan before it runs, and a refusal naming only an opaque
    // digest would be asking them to approve a string — consent in form and not
    // in substance.
    assert!(
        rendered.contains(&format!(
            "command={} phase=21 terminal={}",
            router::COMMAND_PLAN_PHASE,
            goal::TERMINAL_VERIFICATION_PASSED
        )),
        "the refusal must show the plan it is asking about, as typed tokens; \
         got: {rendered}"
    );
    assert!(
        !rendered.contains(&format!("{} 21", router::COMMAND_PLAN_PHASE)),
        "and it must not render the plan as a pasteable command line (WR-09); \
         got: {rendered}"
    );
    assert!(
        !root.path().join(".planning/meta-manager").exists(),
        "an unapproved run creates nothing"
    );
}

#[tokio::test]
async fn an_approval_bound_to_a_different_plan_refuses_the_run() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-otherplan";

    let root = project_root();
    let workdir = seam_workdir();
    // The user reviewed and approved a ONE-step plan; the seam then answers
    // with a two-step one. A model asked the same question twice may answer
    // differently, and an approval covers one answer.
    let approved = payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]);
    let answered = payload(vec![
        step(router::COMMAND_PLAN_PHASE, "20"),
        step(router::COMMAND_PLAN_PHASE, "21"),
    ]);
    plant_payload(workdir.path(), &answered);

    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    args.max_steps = Some(2);
    args.approved_plan = Some(nonblank(&approval_for(root.path(), &approved, args.max_steps)));

    let err = drive(args, &config_for(root.path()))
        .await
        .expect_err("a plan the approval never covered must not run");

    assert!(
        matches!(err, DriveError::PlanApprovalStale(_)),
        "got: {err:?}"
    );
    assert!(
        !root.path().join(".planning/meta-manager").exists(),
        "nothing is created"
    );
}

#[tokio::test]
async fn an_approval_whose_disclosed_bytes_moved_refuses_the_run_though_the_plan_is_identical() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-bytesmoved";

    let root = project_root();
    let workdir = seam_workdir();
    let wire = payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]);
    plant_payload(workdir.path(), &wire);

    // The bytes the user approved.
    std::fs::write(root.path().join("CLAUDE.md"), b"the bytes the user reviewed")
        .expect("write CLAUDE.md");
    let config = config_for(root.path());

    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    args.max_steps = Some(2);
    args.approved_plan = Some(nonblank(&approval_for(root.path(), &wire, args.max_steps)));

    // Now the file changes underneath — a `git pull` the user never read. This
    // tool drives other people's cloned repositories, so this is the ordinary
    // case rather than the exotic one.
    std::fs::write(
        root.path().join("CLAUDE.md"),
        b"## IMPORTANT SYSTEM OVERRIDE",
    )
    .expect("rewrite CLAUDE.md");

    let err = drive(args, &config)
        .await
        .expect_err("bytes that changed after the approval must not be run against");

    // The opt-in's own drift check (plan 21-03) fires first, at the capability
    // gate, and that is correct rather than a shortcoming of this test: the
    // approval binding is the SECOND of two independent layers over the same
    // bytes, and asserting on it here would require disarming the first. What
    // this asserts is the property both exist for — **the run does not start** —
    // and that the refusal names the file.
    //
    // **Against a build with neither layer this FAILS by starting the run.** The
    // approval half specifically is asserted without the opt-in layer in the way
    // in `journal::tests::an_approval_whose_disclosed_files_moved_is_refused_
    // even_though_the_plan_is_identical`, which is the same predicate this path
    // calls.
    let rendered = err.to_string();
    assert!(
        rendered.contains("CLAUDE.md"),
        "the refusal must name the file whose bytes moved; got: {rendered}"
    );
    assert!(
        !root.path().join(".planning/meta-manager").exists(),
        "nothing is created"
    );
}

#[tokio::test]
async fn the_run_record_carries_the_approval_the_cap_and_the_count() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-recorded";

    let root = project_root();
    let workdir = seam_workdir();
    let wire = payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]);
    plant_payload(workdir.path(), &wire);

    let mut args = goal_args(RUN_ID, workdir.path(), "get the goal layer verified");
    args.max_steps = Some(2);
    let token = approval_for(root.path(), &wire, args.max_steps);
    args.approved_plan = Some(nonblank(&token));

    drive(args, &config_for(root.path()))
        .await
        .expect("an approved plan starts the run");

    let paths = gsd_meta_manager::journal::run_paths(&root.path().join(".planning"), RUN_ID)
        .expect("a plain run id");
    let record: Value =
        serde_json::from_str(&std::fs::read_to_string(&paths.run_json).expect("run.json is readable"))
            .expect("run.json parses");

    // The record keeps the two halves as separate fields — that is what lets a
    // failed re-check name WHICH half moved — so the round trip back through the
    // renderer is what says "this record is the token the user gave". A bare
    // comparison against one field would pass while the other half was empty,
    // which is precisely the shape WR-01's throwaway record had.
    assert_eq!(
        gsd_meta_manager::journal::render_approval_token(
            record["approved_plan"]["plan_digest"]
                .as_str()
                .expect("the record carries a plan digest"),
            record["approved_plan"]["approval_digest"]
                .as_str()
                .expect("the record carries an approval digest"),
        ),
        token,
        "the approval recorded on the run record is the one the user gave, both \
         halves of it"
    );
    assert_eq!(record["approved_plan"]["target_phase"], "21");
    assert_eq!(
        record["approved_plan"]["steps"][0],
        format!(
            "command={} phase=21 terminal={}",
            router::COMMAND_PLAN_PHASE,
            goal::TERMINAL_VERIFICATION_PASSED
        ),
        "the plan reaches disk as typed key=value tokens, never as a command line"
    );

    // `escalate::resolve` reduces the unsupplied default to `min(3, 2 - 1)`, so
    // the RESOLVED cap is 1 — never `DEFAULT_MAX_ESCALATIONS`. Against a record
    // that wrote the constant this reads 3, and a reader answering "how often
    // could this run consult a model?" would be told something false.
    assert_eq!(
        record["escalation_cap"], 1,
        "the record carries the RESOLVED cap, never the compiled-in default"
    );
    assert_eq!(
        record["escalations_used"], 1,
        "the decomposition spent exactly one consultation, and the total lands \
         on write two"
    );
    assert_eq!(
        escalate::DEFAULT_MAX_ESCALATIONS, 3,
        "the fixture's premise: the resolved cap really does differ from the \
         default, or the assertion above cannot tell the two apart"
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

// ---------------------------------------------------------------------------
// The approval's PLAN half: collision-resistant, and legacy records fail closed
// (CR-02)
// ---------------------------------------------------------------------------

#[test]
fn the_plan_half_of_an_approval_is_collision_resistant_rather_than_a_non_cryptographic_hash() {
    let wire = payload(vec![
        step(router::COMMAND_PLAN_PHASE, "20"),
        step(router::COMMAND_EXECUTE_PHASE, "21"),
    ]);
    let plan = plan_from(&wire);
    let digest = goal::plan_digest(&plan);

    // --- shape -------------------------------------------------------------
    let hex = digest.strip_prefix("sha256:").unwrap_or_else(|| {
        panic!(
            "the plan half of an approval must carry the `sha256:` prefix. The \
             prefix is not decoration: it is what stops a legacy `fnv1a64:` \
             record ever comparing equal to a freshly computed value. Got: \
             {digest}"
        )
    });
    assert_eq!(hex.len(), 64, "SHA-256 renders as 64 hex digits; got: {digest}");
    assert!(
        hex.chars().all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "lowercase hex only, so two records of the same plan compare as strings \
         without normalising; got: {digest}"
    );
    assert_eq!(
        digest.chars().count(),
        71,
        "seven prefix characters plus sixty-four hex digits; got: {digest}"
    );

    // --- provenance --------------------------------------------------------
    // The token vector the function builds, rebuilt here from the plan's own
    // typed fields so this compares the two hashers over identical input.
    let tokens: Vec<String> = plan
        .steps
        .iter()
        .flat_map(|step| {
            [
                step.command.verb().to_string(),
                step.target_phase.clone(),
                step.terminal_state.as_str().to_string(),
            ]
        })
        .collect();
    assert_ne!(
        digest,
        gsd_meta_manager::journal::argv_digest(&tokens),
        "equality here means the plan half of an approval has silently reverted \
         to FNV-1a-64, whose own doc says it is not a security control. FNV \
         second preimages are CONSTRUCTED rather than searched — multiplication \
         by the FNV prime is invertible mod 2^64 — and the tokens hashed include \
         a `target_phase` authored by whoever wrote the cloned repository's \
         ROADMAP.md. Hashing that under `approval_digest`'s outer SHA-256 does \
         not help: two colliding inner values produce byte-identical input to \
         the outer hash, so the weak collision class survives intact"
    );

    // --- order sensitivity, unchanged by the hasher swap -------------------
    let reversed = payload(vec![
        step(router::COMMAND_EXECUTE_PHASE, "21"),
        step(router::COMMAND_PLAN_PHASE, "20"),
    ]);
    assert_ne!(
        digest,
        goal::plan_digest(&plan_from(&reversed)),
        "a plan is an ordered traversal, so the same steps in a different order \
         are a different plan and must not share an approval"
    );

    // --- rationale exclusion, unchanged by the hasher swap -----------------
    let reworded = payload(vec![
        step_with_rationale(router::COMMAND_PLAN_PHASE, "20", "one wording"),
        step_with_rationale(router::COMMAND_EXECUTE_PHASE, "21", "a different wording"),
    ]);
    assert_eq!(
        digest,
        goal::plan_digest(&plan_from(&reworded)),
        "the rationale is prose no predicate reads; including it would expire a \
         user's approval on a reworded explanation of an identical plan"
    );
}

/// The `recheck_approval` **predicate** refuses a legacy `fnv1a64:` plan digest.
///
/// **What this pins, said plainly, because the name it used to carry claimed
/// more.** It builds an `ApprovedPlan` by hand and calls the predicate directly,
/// so what it proves is the *comparison*: a legacy `fnv1a64:` value never covers
/// a freshly computed `sha256:` one. It pins **no production route into that
/// comparison**, and there is none to pin — production never feeds a
/// deserialised record to `recheck_approval`. The predicate's two production
/// call sites both hold values from the run in progress: `driver::approve_plan`
/// compares the halves of a token parsed off argv in the same invocation, and
/// the spawn gate in `src/driver/run.rs` uses the in-memory `ApprovedPlan` from
/// that run.
///
/// The legacy value a user could actually still be holding is a token on argv,
/// and
/// `a_half_supplied_approval_token_is_refused_by_name_and_never_treated_as_an_approval`
/// is the test that exercises *that* route. Both fail closed; they are different
/// routes and this one is the predicate.
#[test]
fn the_recheck_approval_predicate_refuses_a_legacy_fnv1a64_plan_digest() {
    let wire = payload(vec![step(router::COMMAND_PLAN_PHASE, "21")]);
    let fresh = goal::plan_digest(&plan_from(&wire));

    // The plan half in the format a build that hashed it with FNV-1a-64 would
    // have produced. Hand-built, because no production path deserialises one:
    // what is under test below is the comparison, and it fails closed because
    // the whole prefixed string is compared.
    let legacy = "fnv1a64:0123456789abcdef";
    assert!(
        fresh.starts_with("sha256:") && legacy.starts_with("fnv1a64:"),
        "the fixture's premise: the two values really are in different formats. \
         Against a build whose `plan_digest` is still FNV-1a-64 both carry the \
         same prefix and the refusal below would be about a differing hash \
         rather than about a format that fails closed. Got fresh: {fresh}"
    );

    let recorded = gsd_meta_manager::journal::ApprovedPlan {
        steps: vec![format!(
            "command={} phase=21 terminal={}",
            router::COMMAND_PLAN_PHASE,
            goal::TERMINAL_VERIFICATION_PASSED
        )],
        target_phase: "21".to_string(),
        plan_digest: legacy.to_string(),
        // The approval the user gave covered the LEGACY value, so the outer
        // digest is self-consistent and the refusal below cannot be an artefact
        // of a record this test built wrong.
        approval_digest: gsd_meta_manager::journal::approval_digest(legacy, &[]),
        approved_at: "2026-08-19T12:00:00Z".to_string(),
        extra: Default::default(),
    };

    let refusal = gsd_meta_manager::journal::recheck_approval(
        Some((&recorded.plan_digest, &recorded.approval_digest)),
        &fresh,
        &[],
    )
        .expect_err(
            "a legacy `fnv1a64:` record must never cover a freshly computed \
             `sha256:` plan. Failing closed is the whole reason the digests \
             carry prefixes rather than bare hex",
        );

    match refusal {
        gsd_meta_manager::journal::ApprovalRefusal::PlanChanged { approved, observed } => {
            assert_eq!(approved, legacy, "the refusal names the digest the approval covered");
            assert_eq!(observed, fresh, "and the one observed now");
        }
        other => panic!(
            "a legacy plan digest must re-check as PlanChanged — 'what was \
             approved has changed' — rather than as an absent approval or a \
             file-drift report; got: {other:?}"
        ),
    }
}

// ---------------------------------------------------------------------------
// The model-selected phase token: bounded at construction, refused rather than
// repaired, and sanitized before the reviewer's terminal (WR-05)
// ---------------------------------------------------------------------------

/// The hostile phase tokens, each a real terminal capability rather than a
/// generic "bad character".
///
/// `\u{9b}` is the single-codepoint CSI and `\u{9d}` the single-codepoint OSC:
/// each is a one-character spelling of an `ESC`-led introducer, so a check that
/// only knows about `ESC` leaves the same capability reachable (WR-06).
const HOSTILE_PHASE_TOKENS: &[(&str, &str)] = &[
    ("ESC", "21\u{1b}[2K\u{1b}[1;32m ALL CHECKS PASSED"),
    ("newline", "21\nphase 99: harmless"),
    ("carriage return", "21\rVERIFIED"),
    ("C1 CSI", "21\u{9b}2K"),
];

#[test]
fn a_phase_token_carrying_a_control_character_is_refused_by_name_rather_than_stored() {
    for (label, hostile) in HOSTILE_PHASE_TOKENS {
        // **This premise was INVERTED in 21-13, and the correction rides the
        // commit that falsified it.** It used to assert that
        // `journal::is_plain_path_component` ACCEPTS every one of these tokens
        // — true at the time, and the whole reason the goal layer needed a
        // control-character bound of its own: the predicate rejected path
        // separators and `.`/`..` and nothing else.
        //
        // 21-13 tightened the predicate to refuse any control-carrying value,
        // because the same acceptance let `--run-id '   '` name a run directory
        // made of spaces and let an embedded newline reach the dry-run render
        // verbatim. So the control-character property is now defended at BOTH
        // layers, and this fixture is refused by the first one it meets.
        //
        // **What that costs this test, stated rather than glossed:** for these
        // four fixtures the goal layer's own control-character bound is now
        // defence in depth rather than the sole control, so this test can no
        // longer distinguish which layer refused. What it still pins uniquely
        // is the two things below — that the refusal REUSES the existing reason
        // instead of silently repairing the token into a different phase, and
        // that the refusal reporting the control bytes does not itself render
        // them (`GoalRefusal::new` bounds the offending value at construction).
        // Neither is answered by the predicate.
        assert!(
            !gsd_meta_manager::journal::is_plain_path_component(hostile),
            "since 21-13 the {label} fixture must be a token \
             `is_plain_path_component` REFUSES — if it is accepted again the \
             predicate has been loosened back to the shape that let an embedded \
             newline reach the dry-run render: {hostile:?}"
        );

        let refusal = goal::legality(
            &payload(vec![step(router::COMMAND_PLAN_PHASE, hostile)]),
            PHASES,
            resolved_cap(),
        )
        .expect_err(&format!(
            "a phase token carrying a {label} must be refused. Stored, it \
             reaches the operator's terminal through \
             `DriveError::PlanApprovalRequired` and the committed `run.json`; \
             what actually stopped it before was `PHASE_ID`, a regex in \
             `state_reader::roadmap_md` that the goal layer never mentions — \
             the same reasoning `driver::run`'s escalation prompt already \
             rejects"
        ));

        assert_eq!(
            refusal.reason().as_str(),
            goal::REASON_PHASE_NOT_PLAIN_COMPONENT,
            "the {label} token reuses the existing reason rather than minting a \
             new arm: a token carrying a control character is not a plain path \
             component in any useful sense. Against a build that merely bounds \
             the value without refusing it, this reads \
             `{}` instead — the roadmap-membership arm, reached because the \
             token was silently repaired into a different phase",
            goal::REASON_PHASE_ABSENT_FROM_ROADMAP
        );
        assert!(
            !refusal.offending().contains('\u{1b}')
                && !refusal.offending().chars().any(char::is_control),
            "and the refusal REPORTING the control bytes must not itself render \
             them: `GoalRefusal::new` bounds the offending value at \
             construction. Got: {:?}",
            refusal.offending()
        );
    }
}

/// The phase tokens that carry a character rendering as nothing.
///
/// **A separate const from [`HOSTILE_PHASE_TOKENS`] on purpose.** Those four are
/// each a real terminal capability, and the test over them is named for the
/// control-character property it pins; a look-alike token carries no control
/// character at all — it is refused by the identity clause D-17-1 added, not by
/// the control clause — so folding it into that array would make that test's
/// name assert a falsehood about two of its fixtures. Same register, different
/// harm, own name (deviation recorded in 21-17-SUMMARY).
/// **The last three are OUTSIDE the pre-round-7 ranges, and 21-19 put them here
/// rather than in [`HOSTILE_PHASE_TOKENS`] where its plan text said to.**
/// 21-19's Task 2(d) asked for them in the hostile array; that array's test is
/// named for the control-character property and asserts
/// `!refusal.offending().chars().any(char::is_control)`, and none of these three
/// carries a control character — so following the plan literally would have made
/// that test's name assert a falsehood about three of its fixtures, which is the
/// exact mistake the deviation recorded in 21-17-SUMMARY created this const to
/// avoid. Same intent, correct home. Deviation recorded in 21-19-SUMMARY.
///
/// Pass 7 measured all three reaching roadmap membership at HEAD: the identity
/// clause D-17-1 added was a 22-code-point subset, so `"2\u{202e}0"` was a plain
/// path component and only the roadmap's contents kept it harmless. Under
/// D-19-2's alphabet the refusal is a property of the VALUE (21-17 truth 6).
const LOOK_ALIKE_PHASE_TOKENS: &[(&str, &str)] = &[
    ("interior U+200B", "2\u{200b}0"),
    ("trailing U+FEFF", "20\u{feff}"),
    ("interior U+202E (bidi override)", "2\u{202e}0"),
    ("interior U+E0041 (tag character)", "2\u{e0041}0"),
    ("interior U+00AD (soft hyphen)", "2\u{ad}0"),
];

/// A model-supplied phase token that renders like a declared phase is refused.
///
/// **Pass-6 coincidental-reliance item 2, at the integration seam.** What used
/// to keep these harmless was that no roadmap declares a token containing a
/// `U+200B` — a fact about roadmap contents, not about the value. `PHASES`
/// declares `"20"`, so a build refusing only by membership would report
/// `PHASE_ABSENT_FROM_ROADMAP` here and this test would fail.
#[test]
fn a_phase_token_that_renders_like_a_declared_phase_is_refused_by_the_predicate() {
    assert!(
        PHASES.contains(&"20"),
        "the fixture roadmap must declare the visible member, or this test \
         proves nothing about where the refusal comes from"
    );

    for (label, look_alike) in LOOK_ALIKE_PHASE_TOKENS {
        assert!(
            !gsd_meta_manager::journal::is_plain_path_component(look_alike),
            "the {label} fixture must be a token `is_plain_path_component` \
             REFUSES — if it is accepted again, two phase tokens that render \
             identically can both name a phase: {look_alike:?}"
        );

        let refusal = goal::legality(
            &payload(vec![step(router::COMMAND_PLAN_PHASE, look_alike)]),
            PHASES,
            resolved_cap(),
        )
        .expect_err("a phase token carrying invisible formatting is refused");

        assert_eq!(
            refusal.reason().as_str(),
            goal::REASON_PHASE_NOT_PLAIN_COMPONENT,
            "the {label} token must be refused by the predicate. Reading `{}` \
             here instead would mean the refusal came from roadmap membership — \
             a coincidence of today's roadmap contents rather than a property \
             of the value",
            goal::REASON_PHASE_ABSENT_FROM_ROADMAP
        );
    }
}

/// **The layer that had no coverage at all, and why that mattered** (pass-5
/// adjudication note).
///
/// `goal::legality` refuses a model-supplied phase token at TWO layers: first
/// `journal::is_plain_path_component` (separators, `..`, blank, control
/// characters), then `untrusted::bounded(named_phase) != named_phase`, which
/// answers a different question — renderability — and refuses a value the
/// bound would have to shorten.
///
/// 21-13 tightened the predicate to refuse control characters, and every one of
/// [`HOSTILE_PHASE_TOKENS`] carries one. From that commit on, all four fixtures
/// were refused at the FIRST layer and the second had **zero live coverage**: it
/// could have been deleted, inverted, or turned into a silent truncation and the
/// suite would have stayed green. A defence-in-depth layer with zero coverage is
/// a layer nobody notices breaking.
///
/// The only invocation that reaches it is a token that is control-free, a legal
/// path component, and longer than `untrusted::MAX_UNTRUSTED_FIELD_CHARS`. That
/// is what this builds.
///
/// **What it pins is REFUSAL, not repair.** `bounded` truncates; the goal layer
/// must not. A 201-character token silently shortened to 200 is a *different
/// phase token*, and a run driving toward a phase the model did not name — with
/// the roadmap-membership check downstream then deciding on the shortened value
/// — is exactly the "repair a model's answer" failure SAFE-08 forbids.
#[test]
fn an_over_length_phase_token_is_refused_by_the_bound_not_truncated_into_a_phase() {
    // Control-free, path-component-legal, and one character past the bound. The
    // constant is read rather than the number spelled, so a future cap change
    // moves this fixture with it rather than leaving it silently under the bound.
    let token = "2".repeat(gsd_meta_manager::driver::untrusted::MAX_UNTRUSTED_FIELD_CHARS + 1);

    // **Arrival before property (C-3), the discipline this suite already applies
    // at the injection corpus.** If the token were refused by the FIRST layer,
    // the refusal below would prove nothing about the second — which is exactly
    // the state every HOSTILE_PHASE_TOKENS fixture is in.
    assert!(
        gsd_meta_manager::journal::is_plain_path_component(&token),
        "the fixture must PASS the first layer, or this test is re-testing \
         the predicate instead of the bound it exists for"
    );
    // And it must be a token the bound genuinely acts on: if `bounded` left it
    // alone, the refusal below could only come from somewhere else.
    assert_ne!(
        gsd_meta_manager::driver::untrusted::bounded(&token),
        token,
        "the fixture must be a token `untrusted::bounded` would shorten, or \
         the second layer is not the one under test"
    );

    let refusal = goal::legality(
        &payload(vec![step(router::COMMAND_PLAN_PHASE, &token)]),
        PHASES,
        resolved_cap(),
    )
    .expect_err(
        "a phase token the bound would have to shorten must be REFUSED. \
         Truncated instead, it becomes a different phase token — and the \
         roadmap-membership check downstream would then decide on a value the \
         model never named, which is repairing a model's answer rather than \
         refusing it (SAFE-08)",
    );

    assert_eq!(
        refusal.reason().as_str(),
        goal::REASON_PHASE_NOT_PLAIN_COMPONENT,
        "the over-length token REUSES the existing reason — the goal.rs \
         comment's documented choice, no new `GoalReason` arm. Against a \
         build that truncates instead of refusing, this reads `{}`: the \
         roadmap-membership arm, reached because the token was silently \
         repaired into a phase the model did not choose",
        goal::REASON_PHASE_ABSENT_FROM_ROADMAP
    );

    // The refusal reporting an over-length value must not itself render 201
    // characters: `GoalRefusal::new` bounds the offending value at construction,
    // the same property the control-character fixtures pin for control bytes.
    assert!(
        refusal.offending().chars().count()
            <= gsd_meta_manager::driver::untrusted::MAX_UNTRUSTED_FIELD_CHARS
                + gsd_meta_manager::driver::untrusted::TRUNCATION_MARKER
                    .chars()
                    .count(),
        "the refusal's rendered value must be bounded, not the full token; got          {} characters",
        refusal.offending().chars().count()
    );
    assert_ne!(
        refusal.offending(),
        token,
        "and it must not be the raw token: a refusal that echoes 201 unbounded \
         characters into the operator's terminal is the shape `GoalRefusal::new`'s \
         bounding exists to prevent"
    );
    assert!(
        refusal
            .offending()
            .ends_with(gsd_meta_manager::driver::untrusted::TRUNCATION_MARKER),
        "a shortened value must be MARKED as shortened — a silently shortened \
         string is indistinguishable from a short one, and the char bound above \
         alone would be satisfied by rendering the token in full. Got: {:?}",
        refusal.offending()
    );
}

#[test]
fn a_legal_phase_token_reaches_the_step_byte_identical_to_what_the_roadmap_declared() {
    let plan = plan_from(&payload(vec![
        step(router::COMMAND_PLAN_PHASE, "20"),
        step(router::COMMAND_EXECUTE_PHASE, "21"),
    ]));

    let stored: Vec<&str> = plan
        .steps
        .iter()
        .map(|step| step.target_phase.as_str())
        .collect();
    assert_eq!(
        stored,
        vec!["20", "21"],
        "bounding must not silently CHANGE which phase the run drives toward. \
         This value becomes `args.target_phase`, and therefore the map key into \
         `ProjectState::phase_disk_statuses` and the input to \
         `RouterAction::command_for` — a truncated token would drive the run \
         toward a different phase than the plan the user approved named, which \
         is worse than the defect being fixed. That is why `legality` refuses a \
         token bounding would alter rather than storing the bounded form of it"
    );
}

#[tokio::test]
async fn a_hostile_roadmap_phase_token_is_refused_at_the_seam_the_run_actually_reaches() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-hostilephase";

    let root = project_root();
    let workdir = seam_workdir();
    let (_, hostile) = HOSTILE_PHASE_TOKENS[0];
    plant_payload(
        workdir.path(),
        &payload(vec![step(router::COMMAND_PLAN_PHASE, hostile)]),
    );

    let err = drive(
        goal_args(RUN_ID, workdir.path(), "get the goal layer verified"),
        &config_for(root.path()),
    )
    .await
    .expect_err("a plan naming an escape-bearing phase must not start a run");

    assert!(matches!(err, DriveError::GoalRefused(_)), "got: {err:?}");
    let rendered = err.to_string();
    assert!(
        rendered.contains(goal::REASON_PHASE_NOT_PLAIN_COMPONENT),
        "the end-to-end seam must refuse it as a phase that is not a plain \
         component. Against the unfixed build the token survives the \
         path-component check and is refused one arm later as absent from the \
         roadmap — a refusal that would stop being reached the moment a hostile \
         ROADMAP.md declared the token it also planted; got: {rendered}"
    );
    assert!(
        !rendered.contains('\u{1b}'),
        "and no ESC may reach the terminal of the person reading the refusal; \
         got: {rendered:?}"
    );
    assert!(
        !root.path().join(".planning/meta-manager").exists(),
        "a run refused above the run creates nothing"
    );
}

#[test]
fn the_approval_refusal_cannot_repaint_the_terminal_of_the_person_about_to_approve() {
    // Two steps, so the assertion below is also about the refusal still naming
    // EVERY step rather than about it surviving with one.
    let benign = format!(
        "command={} phase=20 terminal={}",
        router::COMMAND_PLAN_PHASE,
        goal::TERMINAL_VERIFICATION_PASSED
    );
    let hostile = format!(
        "command={} phase=21\u{1b}[2K\u{9b}1;32m terminal={}",
        router::COMMAND_EXECUTE_PHASE,
        goal::TERMINAL_VERIFICATION_PASSED
    );
    assert!(
        hostile.contains('\u{1b}') && hostile.contains('\u{9b}'),
        "the fixture's premise: the step really does carry both an ESC-led and \
         a single-codepoint C1 introducer"
    );

    // The token composed through the shipped renderer, never by concatenation,
    // so this test cannot agree with itself about a shape the run would not
    // print.
    let token = gsd_meta_manager::journal::render_approval_token(
        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
        "sha256:2222222222222222222222222222222222222222222222222222222222222222",
    );
    let rendered = DriveError::PlanApprovalRequired {
        token: token.clone(),
        steps: vec![benign.clone(), hostile],
    }
    .to_string();

    assert!(
        !rendered.contains('\u{1b}'),
        "an ESC reaching this string is a terminal-repaint capability handed to \
         a hostile roadmap at the exact moment the operator is deciding whether \
         to approve — it can erase the line, forge a green VERIFIED, or move the \
         cursor over the plan it is asking about. Got: {rendered:?}"
    );
    assert!(
        !rendered.contains('\u{9b}'),
        "and C1 is not an afterthought: `U+009B` is the one-codepoint CSI, so \
         stripping ESC alone leaves the same capability reachable by another \
         spelling (WR-06). Got: {rendered:?}"
    );

    // Sanitizing must not eat the actionable half of the message.
    assert!(
        rendered.contains(&benign),
        "the refusal must still name every step — the refusal IS the review \
         surface DRIVE-03 requires; got: {rendered}"
    );
    assert!(
        rendered.contains("phase=21") && rendered.contains(router::COMMAND_EXECUTE_PHASE),
        "including the sanitized one, which is still readable as typed tokens; \
         got: {rendered}"
    );
    assert!(
        rendered.contains(&format!("--approved-plan {token}")),
        "and it must still name the flag and the WHOLE token the caller must \
         pass — both halves, not just the approval digest — or the refusal is a \
         bug report rather than an error message; got: {rendered}"
    );
    assert!(
        rendered.contains("  1. ") && rendered.contains("  2. "),
        "the steps stay numbered, so a reviewer reads a plan rather than a \
         run-on line; got: {rendered}"
    );
}
