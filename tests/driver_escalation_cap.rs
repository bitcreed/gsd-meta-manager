// ============================================================================
// The escalation cap PARKS, and the journal says so (DRIVE-04, ROADMAP
// criterion 3).
//
// **THE EVIDENTIARY STANDARD, borrowed verbatim from `tests/envelope_wiring.rs`:
// no assertion may read a model's summary.** Every fact below is a record read
// back out of `journal.jsonl` through the SHIPPED reader, a field of `run.json`,
// or a line a stand-in program appended because it actually ran. Not one is a
// sentence somebody wrote about the run.
//
// # Three things, in order, and the order is the point
//
// 1. The run **parks** — not halts, not completes — under the model seam's own
//    cap reason, read through `EscalationReason::as_str` and never a literal
//    typed into this file. `grep -c` for that identifier's stable string returns
//    **0** here, which is the mechanical form of "the reason came from the
//    taxonomy".
// 2. The run did **not silently degrade to rules-only**. That is the assertion
//    which distinguishes a park from a shrug: after the parked event there is no
//    further command record, and the run consumed fewer steps than its cap. A
//    run that kept going without the model would have consumed more.
// 3. The count is **honest**: the number on disk equals the number of child
//    processes that actually ran, observed through the stand-in's own spawn
//    ledger rather than through anything the driver reports about itself.
//
// # Never the compiled-in step-cap constant
//
// Every cap in this file is built by CALLING `bounds::resolve` and reading
// `max_steps` off what it returned, then handing that to `escalate::resolve`.
// The compiled-in default is named nowhere — `grep -c` for it returns **0** —
// because Phase 20's review found a Critical hiding behind a guard that compared
// two constants with each other instead of the resolved value, and this file is
// exactly where that shape would recur.
//
// # The state that makes the driven arms possible
//
// The ambiguity seam fires only where `router::decide` returns `NoRule`, and
// three earlier plans recorded that as unreachable from disk. It is reachable:
// a phase the roadmap still declares but that has been archived into
// `.planning/milestones/` is read as `complete` with its verification status
// unread, which is a `complete` observation that is not goal-met and that the
// rule table has no row for. `tests/driver_refusal_record.rs` carries the full
// account and the premise test; this file states the premise again in its own
// terms so it cannot go stale here unnoticed.
//
// Unix-only by construction (D-05, D-30, WR-16).
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::path::Path;

use gsd_meta_manager::config::{Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::{bounds, drive, escalate, goal, router, DriveArgs};
use gsd_meta_manager::error::DriveError;
use gsd_meta_manager::journal::{self, reader};
use gsd_meta_manager::state_reader::parse_project_state;
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


/// This file's own text, for the completeness guard and the two negative greps.
const OWN_SOURCE: &str = include_str!("driver_escalation_cap.rs");

const SEAM_CLAUDE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-seam.sh"
);

const CLEAN_BASELINE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/transcripts/01-success-textonly.ndjson"
);

const ALIAS: &str = "escalationcap";

/// The phase every driven run below targets.
const TARGET: &str = "31";

/// The step cap the driven arms run under.
///
/// Four, so the cap of two below is one the run can actually reach: two
/// escalated iterations run, and the third no-rule state is the one the budget
/// refuses. A step cap the escalation budget could not fit inside would halt on
/// the wrong detector and this file would prove nothing about the cap.
const DRIVEN_MAX_STEPS: u32 = 4;

/// The escalation budget the driven arms run under.
const DRIVEN_BUDGET: u32 = 2;

// ---------------------------------------------------------------------------
// The two resolvers, called rather than assumed
// ---------------------------------------------------------------------------

/// The step cap a run bounded at `max_steps` is REALLY running under, built the
/// only sanctioned way: by asking `bounds::resolve` and reading `max_steps` off
/// what it returned.
///
/// The compiled-in default is never named. A test that reached for the constant
/// would be comparing two constants with each other, which is precisely the
/// defect the boundary arms below exist to catch.
fn resolved_step_cap(max_steps: Option<u32>) -> u32 {
    bounds::resolve(max_steps, None)
        .expect("a legal step cap resolves")
        .max_steps
}

// ---------------------------------------------------------------------------
// The fixture: a roadmap phase the project has archived
// ---------------------------------------------------------------------------

/// A project whose roadmap declares two phases, the second of which has been
/// archived into a completed milestone.
///
/// That is the ordinary shape a tree takes after `/gsd-complete-milestone`
/// archives a phase the roadmap still lists, and it is what puts `router::decide`
/// on its no-rule arm every iteration — which is what lets a bounded budget be
/// spent to exhaustion inside the step cap.
fn no_rule_project() -> TempDir {
    let root = TempDir::new().expect("temp dir");
    let planning = root.path().join(".planning");
    std::fs::create_dir_all(&planning).expect("scratch .planning");

    std::fs::write(
        planning.join("ROADMAP.md"),
        "# Roadmap\n\n\
         - [x] **Phase 30: Baseline** - the finished predecessor\n\
         - [ ] **Phase 31: Target** - the phase every run below drives toward\n",
    )
    .expect("write ROADMAP.md");

    let baseline = planning.join("phases").join("30-baseline");
    std::fs::create_dir_all(&baseline).expect("the baseline phase dir");
    std::fs::write(baseline.join("30-CONTEXT.md"), "# Context\n").expect("write context");

    // The target, archived. `infer_phase_status` returns for it early, without
    // reading a verification artifact, so the inference is `complete` with a
    // verification status of "nothing was read" — honest, and not goal-met.
    let archived = planning
        .join("milestones")
        .join("v1.0")
        .join("31-target");
    std::fs::create_dir_all(&archived).expect("the archived phase dir");
    std::fs::write(archived.join("31-01-SUMMARY.md"), "# Summary\n").expect("write summary");

    root
}

/// The same project with the target phase where GSD normally keeps it — the
/// control shape, in which the rule table covers the state and no seam fires.
fn covered_project() -> TempDir {
    let root = TempDir::new().expect("temp dir");
    let planning = root.path().join(".planning");
    std::fs::create_dir_all(&planning).expect("scratch .planning");

    std::fs::write(
        planning.join("ROADMAP.md"),
        "# Roadmap\n\n\
         - [x] **Phase 30: Baseline** - the finished predecessor\n\
         - [ ] **Phase 31: Target** - the phase every run below drives toward\n",
    )
    .expect("write ROADMAP.md");

    for (number, slug) in [(30, "baseline"), (31, "target")] {
        let dir = planning.join("phases").join(format!("{number}-{slug}"));
        std::fs::create_dir_all(&dir).expect("a phase dir");
        std::fs::write(dir.join(format!("{number}-CONTEXT.md")), "# Context\n")
            .expect("write context");
    }

    root
}

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
            added: "2026-08-20T12:00:00Z".to_string(),
            driver_opt_in: Some(DriverOptIn {
                opted_in_at: "2026-08-20T11:59:00Z".to_string(),
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

fn seam_workdir() -> TempDir {
    TempDir::new().expect("a seam workdir")
}

/// Answer seam spawn `n` with a one-step plan naming `command`.
///
/// Consecutive spawns are given DIFFERENT legal commands on purpose: the
/// command-repeat detector halts a run that selects the same command twice
/// running, and a run halted on that detector would never reach the cap the
/// arms below are about.
fn plant_payload(workdir: &Path, n: usize, command: &str) {
    let payload = serde_json::json!({ goal::FIELD_STEPS: [{
        goal::FIELD_COMMAND: command,
        goal::FIELD_PHASE: TARGET,
        goal::FIELD_TERMINAL_STATE: goal::TERMINAL_VERIFICATION_PASSED,
        goal::FIELD_RATIONALE: "the rule table covers no rule for this state",
    }]});
    std::fs::create_dir_all(workdir).expect("the seam workdir");
    std::fs::write(
        workdir.join(format!("seam-payload.{n}")),
        serde_json::to_string(&payload).expect("the payload serialises"),
    )
    .expect("write the seam payload");
}

/// How many SEAM spawns the stand-in recorded, read off disk.
///
/// **A program that ran, never an in-process counter.** The whole point of the
/// count-honesty assertion is that the number on the record matches the number
/// of child processes that really happened, which is the property the pinned
/// structured-output retry exists to make true.
fn seam_spawns(workdir: &Path) -> usize {
    std::fs::read_to_string(workdir.join("seam-spawns"))
        .map(|text| text.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or(0)
}

fn agent_spawns(workdir: &Path) -> usize {
    std::fs::read_to_string(workdir.join("agent-spawns"))
        .map(|text| text.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or(0)
}

fn routed_args(run_id: &str, workdir: &Path, budget: Option<u32>) -> DriveArgs {
    routed_args_with_steps(run_id, workdir, budget, DRIVEN_MAX_STEPS)
}

fn routed_args_with_steps(
    run_id: &str,
    workdir: &Path,
    budget: Option<u32>,
    max_steps: u32,
) -> DriveArgs {
    DriveArgs {
        alias: nonblank(ALIAS),
        command: None,
        target_phase: Some(nonblank(TARGET)),
        max_steps: Some(max_steps),
        wall_clock_cap_secs: None,
        max_escalations: budget,
        approved_plan: None,
        run_id: Some(nonblank(run_id)),
        dry_run: false,
        goal: None,
        claude_program: Some(SEAM_CLAUDE.into()),
        claude_args: vec![OsString::from(workdir), OsString::from(CLEAN_BASELINE)],
    }
}

// ---------------------------------------------------------------------------
// Reading the run back off disk
// ---------------------------------------------------------------------------

fn run_paths(root: &Path, run_id: &str) -> journal::RunPaths {
    journal::run_paths(&root.join(".planning"), run_id).expect("a plain run id")
}

fn records(root: &Path, run_id: &str) -> Vec<reader::JournalRecord> {
    let paths = run_paths(root, run_id);
    let (records, _diagnostics) =
        reader::read_all(&paths.journal).expect("the journal is readable");
    records
}

fn parked_events(root: &Path, run_id: &str) -> Vec<(String, String)> {
    records(root, run_id)
        .iter()
        .filter(|record| record.kind == "parked")
        .map(|record| {
            (
                record.rest["reason"].as_str().unwrap_or_default().to_string(),
                record.rest["needs"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

fn run_record(root: &Path, run_id: &str) -> Value {
    let paths = run_paths(root, run_id);
    serde_json::from_str(&std::fs::read_to_string(&paths.run_json).expect("run.json is readable"))
        .expect("run.json parses")
}

/// The journal record kinds that say a command was chosen or run.
///
/// `decided` is the router's or the seam's selection for one iteration;
/// `exec_started` is a command actually beginning. Either one appearing after
/// the park would be a run that kept working past the moment it said it had
/// stopped.
const COMMAND_KINDS: &[&str] = &["decided", "exec_started"];

// ---------------------------------------------------------------------------
// The premise, restated in this file's own terms
// ---------------------------------------------------------------------------

#[test]
fn the_fixture_really_reaches_the_state_the_rule_table_does_not_cover() {
    let root = no_rule_project();
    let state = parse_project_state(&root.path().join(".planning"));

    let inference = state
        .phase_disk_statuses
        .get(TARGET)
        .expect("the reader records one inference per declared roadmap phase");
    assert!(
        !router::is_goal_met(inference),
        "an archived phase must not read as goal-met: nothing read its \
         verification frontmatter, and a claim that it passed would be an \
         inference rather than an observation"
    );

    match router::decide(&state, TARGET) {
        router::Decision::NoRule { observed } => assert_eq!(
            observed,
            router::status_token(inference.status),
            "the no-rule park names what it observed through the router's own \
             token function"
        ),
        other => panic!(
            "the fixture no longer reaches the seam, so every driven arm in this \
             file would pass while testing nothing. `router::decide` answered \
             {other:?}"
        ),
    }

    // The control shape: with the phase where GSD normally keeps it, the rule
    // table covers it and no model is consulted at all.
    let covered = covered_project();
    let live = parse_project_state(&covered.path().join(".planning"));
    assert!(
        matches!(router::decide(&live, TARGET), router::Decision::Run { .. }),
        "the covered control must route deterministically, or the comparison is \
         between two questions rather than two answers"
    );
}

// ---------------------------------------------------------------------------
// The pure boundary arms — no spawn, no run directory, no clock
// ---------------------------------------------------------------------------

/// **The most load-bearing guard here, and the shape of Phase 20's Critical.**
///
/// Every number is the RESOLVED one. Against a resolution that compared the
/// supplied cap with the compiled-in constant, a run bounded at two steps would
/// accept a cap of two — a budget that can never bind while still looking
/// configured — and this arm FAILS.
#[test]
fn the_three_boundaries_are_measured_against_the_resolved_step_cap() {
    let cap = resolved_step_cap(Some(DRIVEN_MAX_STEPS));
    assert_eq!(
        cap, DRIVEN_MAX_STEPS,
        "the fixture's premise: the resolver really returns the step cap the \
         driven arms below run under"
    );

    // One below the resolved cap can bind, so it is accepted.
    let budget = escalate::resolve(Some(cap - 1), cap)
        .expect("a cap one below the resolved step cap can bind");
    assert_eq!(budget.cap(), cap - 1);
    assert!(budget.can_escalate());

    // Equal to it cannot: the run halts on its step cap first, so such a budget
    // is a disablement wearing a cap's clothing.
    let refusal = escalate::resolve(Some(cap), cap)
        .expect_err("a cap equal to the resolved step cap can never bind");
    assert_eq!(
        refusal,
        escalate::EscalationRefusal::CapCannotBind {
            cap,
            max_steps: cap
        },
        "the refusal must carry BOTH numbers: a caller told only one of them \
         cannot tell which to change"
    );

    // Zero is a seam that can never fire while still looking configured.
    assert_eq!(
        escalate::resolve(Some(0), cap).expect_err("a supplied zero is refused"),
        escalate::EscalationRefusal::ZeroCap
    );

    // The control arm: the same numbers are legal under a roomier resolved step
    // cap, so the refusals above are about the RESOLVED value and not about the
    // number four.
    let roomy = resolved_step_cap(None);
    assert!(
        roomy > cap,
        "the control needs a genuinely roomier cap to compare against"
    );
    assert!(escalate::resolve(Some(cap), roomy).is_ok());
}

// ---------------------------------------------------------------------------
// The driven arms
// ---------------------------------------------------------------------------

/// The cap parks, the journal says so, the run did not degrade, and the count is
/// honest.
///
/// One test rather than three because all three assertions are about the SAME
/// run, in order, and splitting them would mean driving the same run three times
/// and asserting on three different histories.
#[tokio::test]
async fn a_run_that_spends_its_budget_parks_and_says_so_rather_than_continuing() {
    const RUN_ID: &str = "2026-08-20T13-00-00Z-capparked";

    let root = no_rule_project();
    let workdir = seam_workdir();
    // Two consecutive escalations, given two DIFFERENT legal commands so the
    // command-repeat detector does not halt the run before the budget binds.
    plant_payload(workdir.path(), 1, router::COMMAND_DISCUSS_PHASE);
    plant_payload(workdir.path(), 2, router::COMMAND_PLAN_PHASE);

    let step_cap = resolved_step_cap(Some(DRIVEN_MAX_STEPS));
    let budget = escalate::resolve(Some(DRIVEN_BUDGET), step_cap)
        .expect("a budget below the resolved step cap can bind");

    drive(
        routed_args(RUN_ID, workdir.path(), Some(budget.cap())),
        &config_for(root.path()),
    )
    .await
    .expect("a run that parks is not an error");

    // The run really happened.
    assert!(
        run_paths(root.path(), RUN_ID).journal.exists(),
        "no journal exists, so nothing below is about a real run"
    );

    // ---- 1. It PARKED, under the seam's own cap reason -------------------
    let cap_reason = escalate::EscalationReason::CapReached.as_str();
    assert_eq!(
        parked_events(root.path(), RUN_ID),
        vec![(cap_reason.to_string(), "human".to_string())],
        "the run must park exactly once, under the cap reason read through \
         `EscalationReason::as_str`. A literal typed here would be a second \
         source for a string the taxonomy already owns"
    );

    let outcome = run_record(root.path(), RUN_ID)["outcome"]
        .as_str()
        .expect("a terminal label")
        .to_string();
    assert!(
        outcome.ends_with(cap_reason) && outcome != cap_reason,
        "the terminal label must carry the reason behind the EXISTING parked \
         label prefix, so it reached disk through machinery that already existed \
         rather than through a third string source; got: {outcome}"
    );
    assert_eq!(
        outcome,
        format!("{}{cap_reason}", parked_label_prefix(root.path(), RUN_ID)),
        "and the prefix is the one this run's own park record implies"
    );

    // ---- 2. It did NOT silently degrade to rules-only --------------------
    let all = records(root.path(), RUN_ID);
    let park_at = all
        .iter()
        .position(|record| record.kind == "parked")
        .expect("the parked record");
    let after: Vec<&str> = all[park_at + 1..]
        .iter()
        .map(|record| record.kind.as_str())
        .filter(|kind| COMMAND_KINDS.contains(kind))
        .collect();
    assert!(
        after.is_empty(),
        "the journal carries {after:?} AFTER the park. A run that stopped \
         consulting the model and carried on under the rules alone has \
         materially changed what it is, and a change that large has to be \
         readable off disk rather than hidden behind a park record that turned \
         out not to end anything"
    );

    let steps_taken = all
        .iter()
        .filter(|record| record.kind == "decided")
        .count() as u32;
    assert_eq!(
        steps_taken, DRIVEN_BUDGET,
        "the run must take exactly one step per granted consultation"
    );
    assert!(
        steps_taken < step_cap,
        "the run consumed {steps_taken} of its {step_cap} steps. A run that kept \
         going without the model would have consumed more, so a step count at \
         the cap would mean the park did not stop anything"
    );
    assert_eq!(
        run_record(root.path(), RUN_ID)["bounds"]["max_steps"],
        Value::from(step_cap),
        "and the record carries the RESOLVED step cap the comparison above used"
    );

    // ---- 3. The count is HONEST ------------------------------------------
    let record = run_record(root.path(), RUN_ID);
    assert_eq!(
        record["escalation_cap"],
        Value::from(budget.cap()),
        "the record carries the resolved budget, never a compiled-in default"
    );
    assert_eq!(
        record["escalations_used"],
        Value::from(budget.cap()),
        "a run that parks on its cap has spent its budget exactly: every \
         consultation it was permitted, and not the one that would have been \
         number N+1"
    );
    assert_eq!(
        seam_spawns(workdir.path()),
        budget.cap() as usize,
        "the number on disk must equal the number of CHILD PROCESSES that ran, \
         observed through a program that left evidence rather than through an \
         in-process counter. A number that reads as a safety property while \
         disagreeing with the processes it describes is worse than no number"
    );
    assert_eq!(
        agent_spawns(workdir.path()),
        budget.cap() as usize,
        "and each permitted consultation produced exactly one executed command, \
         so no step ran unaccounted for"
    );
}

/// The parked-label prefix implied by a run's own park record.
///
/// `run::PARKED_LABEL_PREFIX` is `pub(crate)`, so an integration test cannot name
/// it, and typing it here would be the third string source the assertion exists
/// to forbid. It is derived instead: the terminal label minus the reason the
/// parked record independently carries.
fn parked_label_prefix(root: &Path, run_id: &str) -> String {
    let reason = parked_events(root, run_id)
        .first()
        .map(|(reason, _)| reason.clone())
        .expect("a parked record");
    let label = run_record(root, run_id)["outcome"]
        .as_str()
        .expect("a terminal label")
        .to_string();
    label
        .strip_suffix(&reason)
        .unwrap_or_else(|| {
            panic!("the label {label:?} does not end in its own reason {reason:?}")
        })
        .to_string()
}

/// The step cap the upper-boundary arm runs under.
///
/// Three, so the largest legal budget is two — and two is also the largest
/// budget any run can spend to exhaustion against a tree its agent does not
/// change, because Phase 20's no-progress detector fires on the third
/// unchanged observation. **That is correct behaviour rather than an obstacle**:
/// a run consulting the model again and again while nothing moves IS stalled,
/// and the stall detector is the right thing to report. It is recorded here
/// because an earlier draft of this arm drove a budget of three under a step cap
/// of four and parked on `bounds_no_progress` instead — a real ordering fact
/// that the arm was corrected to respect rather than to work around.
const UPPER_BOUNDARY_MAX_STEPS: u32 = 3;

/// A budget one below the resolved step cap RUNS, and parks on the cap.
///
/// The upper boundary driven for real rather than only resolved: the resolver
/// accepting a value proves the parser, and this proves the run.
#[tokio::test]
async fn a_budget_one_below_the_resolved_step_cap_runs_and_parks_on_the_cap() {
    const RUN_ID: &str = "2026-08-20T13-00-00Z-oneBelow";

    let step_cap = resolved_step_cap(Some(UPPER_BOUNDARY_MAX_STEPS));
    let budget = step_cap - 1;
    assert!(
        escalate::resolve(Some(budget), step_cap).is_ok(),
        "the arm's premise: this really is the largest budget the resolver \
         accepts under this step cap"
    );
    assert!(
        escalate::resolve(Some(budget + 1), step_cap).is_err(),
        "and one more is refused, so `budget` is the boundary rather than a \
         number near it"
    );

    let root = no_rule_project();
    let workdir = seam_workdir();
    // Consecutive escalations given alternating legal commands so the
    // command-repeat detector never fires before the budget binds.
    for (n, command) in [router::COMMAND_DISCUSS_PHASE, router::COMMAND_PLAN_PHASE]
        .iter()
        .enumerate()
    {
        plant_payload(workdir.path(), n + 1, command);
    }

    drive(
        routed_args_with_steps(RUN_ID, workdir.path(), Some(budget), UPPER_BOUNDARY_MAX_STEPS),
        &config_for(root.path()),
    )
    .await
    .expect("a budget that can bind starts a run");

    assert_eq!(
        parked_events(root.path(), RUN_ID),
        vec![(
            escalate::EscalationReason::CapReached.as_str().to_string(),
            "human".to_string()
        )],
        "the largest budget that can bind must still bind: a cap that is legal \
         and never fires is a cap in name only"
    );
    assert_eq!(
        run_record(root.path(), RUN_ID)["escalations_used"],
        Value::from(budget)
    );
    assert_eq!(seam_spawns(workdir.path()), budget as usize);
}

/// A budget EQUAL to the resolved step cap refuses before the run exists.
#[tokio::test]
async fn a_budget_equal_to_the_resolved_step_cap_refuses_above_the_run() {
    const RUN_ID: &str = "2026-08-20T13-00-00Z-equal";

    let step_cap = resolved_step_cap(Some(DRIVEN_MAX_STEPS));
    let root = no_rule_project();
    let workdir = seam_workdir();
    plant_payload(workdir.path(), 1, router::COMMAND_PLAN_PHASE);

    let err = drive(
        routed_args(RUN_ID, workdir.path(), Some(step_cap)),
        &config_for(root.path()),
    )
    .await
    .expect_err("a budget that can never bind must not start a run");

    assert!(
        matches!(
            err,
            DriveError::EscalationRefused(escalate::EscalationRefusal::CapCannotBind { .. })
        ),
        "the refusal must carry the escalation taxonomy's own refusal rather \
         than a fresh string; got: {err:?}"
    );
    assert_nothing_was_created(root.path(), workdir.path(), &err, step_cap);
}

/// A budget of ZERO refuses before the run exists.
#[tokio::test]
async fn a_budget_of_zero_refuses_above_the_run() {
    const RUN_ID: &str = "2026-08-20T13-00-00Z-zero";

    let step_cap = resolved_step_cap(Some(DRIVEN_MAX_STEPS));
    let root = no_rule_project();
    let workdir = seam_workdir();
    plant_payload(workdir.path(), 1, router::COMMAND_PLAN_PHASE);

    let err = drive(
        routed_args(RUN_ID, workdir.path(), Some(0)),
        &config_for(root.path()),
    )
    .await
    .expect_err("a seam that can never fire must not look configured");

    assert!(
        matches!(
            err,
            DriveError::EscalationRefused(escalate::EscalationRefusal::ZeroCap)
        ),
        "got: {err:?}"
    );
    assert_nothing_was_created(root.path(), workdir.path(), &err, step_cap);
}

/// A refusal ABOVE the run leaves nothing at all behind.
///
/// This is what distinguishes the two refusal arms from the park: a park is
/// something a run does, and a run that never existed cannot do it. If a run
/// directory or a journal appeared, the refusal happened inside the run and the
/// two boundaries would be the same boundary wearing two names.
fn assert_nothing_was_created(root: &Path, workdir: &Path, err: &DriveError, step_cap: u32) {
    assert!(
        !root.join(".planning/meta-manager").exists(),
        "a refusal above the run created a run directory, so the refusal is a \
         park inside a run rather than a refusal of one"
    );
    assert!(
        !root.join(".planning/meta-manager/runs").exists(),
        "a runs directory exists"
    );
    assert_eq!(
        seam_spawns(workdir),
        0,
        "the model was consulted by a run that was refused before it existed"
    );
    assert_eq!(agent_spawns(workdir), 0, "and no command ran");

    let rendered = err.to_string();
    assert!(
        rendered.contains("--max-escalations"),
        "the refusal must name the flag, or a caller cannot act on it; got: \
         {rendered}"
    );
    assert!(
        rendered.contains(&step_cap.to_string()) || rendered.contains('0'),
        "and it must name a number the caller can compare against; got: \
         {rendered}"
    );
}

// ---------------------------------------------------------------------------
// Guards
// ---------------------------------------------------------------------------

/// Both sets are present: the pure boundary arms and the driven arms.
///
/// Deleting either set makes this FAIL naming what went, which is the only thing
/// standing between a suite that quietly shrank and a `cargo test` that stayed
/// green.
#[test]
fn both_the_pure_boundary_arms_and_the_driven_arms_are_present() {
    for (what, function) in [
        (
            "the pure three-boundary arm",
            "fn the_three_boundaries_are_measured_against_the_resolved_step_cap",
        ),
        (
            "the premise that the fixture reaches the seam",
            "fn the_fixture_really_reaches_the_state_the_rule_table_does_not_cover",
        ),
        (
            "the driven cap park, with its no-degradation and count-honesty halves",
            "async fn a_run_that_spends_its_budget_parks_and_says_so_rather_than_continuing",
        ),
        (
            "the driven upper boundary",
            "async fn a_budget_one_below_the_resolved_step_cap_runs_and_parks_on_the_cap",
        ),
        (
            "the driven equal-to-cap refusal",
            "async fn a_budget_equal_to_the_resolved_step_cap_refuses_above_the_run",
        ),
        (
            "the driven zero refusal",
            "async fn a_budget_of_zero_refuses_above_the_run",
        ),
    ] {
        assert!(
            OWN_SOURCE.contains(function),
            "{what} is gone: `{function}` is not in this file"
        );
    }
}

/// Neither string this file must never spell appears in it.
///
/// The plan states both as `grep -c` criteria; they are asserted here instead of
/// being left to a reviewer's memory, because a criterion nobody re-runs is a
/// criterion that decays. The cap reason must arrive through
/// `EscalationReason::as_str` and the step cap through `bounds::resolve`, and the
/// mechanical form of both sentences is that the literals are absent.
#[test]
fn this_file_spells_neither_the_cap_reason_nor_the_compiled_in_step_cap() {
    // Built from the taxonomy rather than typed, so this guard cannot itself be
    // the occurrence it forbids.
    let reason = escalate::EscalationReason::CapReached.as_str();
    let occurrences = OWN_SOURCE.matches(reason).count();
    assert_eq!(
        occurrences, 0,
        "the cap reason {reason:?} is spelled {occurrences} time(s) in this \
         file. It must arrive through `EscalationReason::as_str` only, so a \
         renamed arm is a compile error rather than a test that keeps passing \
         against a string nothing produces any more"
    );

    // Assembled from fragments for the same reason: naming the constant would
    // put the very identifier this guard forbids into the file.
    let constant = format!("{}{}", "DEFAULT_MAX", "_STEPS");
    let occurrences = OWN_SOURCE.matches(constant.as_str()).count();
    assert_eq!(
        occurrences, 0,
        "the compiled-in step-cap constant is named {occurrences} time(s). Every \
         cap here must be built by calling `bounds::resolve` and reading the \
         value back — comparing two constants with each other is the exact shape \
         of the Critical Phase 20's review found"
    );

    // And the guard is not vacuous: the file really was read.
    assert!(
        OWN_SOURCE.len() > 4_000,
        "the source this guard scans is empty or truncated, so both counts above \
         are facts about nothing"
    );
}
