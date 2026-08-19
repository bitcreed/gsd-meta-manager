// ============================================================================
// The iteration loop: one process, one lock, one journal — and MORE THAN ONE
// GSD command, chosen by a rule and stopped by a bound (CTRL-06, DRIVE-02,
// DRIVE-06).
//
// **THE RULE THIS FILE IS WRITTEN UNDER, borrowed verbatim from
// `tests/envelope_wiring.rs`: no assertion may read a model's summary.** Every
// fact below is a record read back out of `journal.jsonl` or a field of
// `run.json`, both by the shipped reader and both after the driven process has
// exited. Not one is a sentence somebody wrote about the run. The whole value of
// a deterministic router is that its choice is checkable without asking anything
// what it did.
//
// It is an integration test rather than an in-source one for the reasons
// `tests/driver_tracer.rs` records: it spawns a genuine child process through a
// checked-in shell stand-in and builds a real `Config` whose registry entry
// points at a real directory on disk.
//
// Unix-only by construction: driving is a Unix capability (D-05), and the run
// body is `#[cfg(unix)]`. Debug-only in practice too — `--claude-program` has no
// parser entry in a release build (D-30, WR-16) — which is why
// `cargo test --release` does not build this target.
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::path::Path;

use gsd_meta_manager::config::{Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::{bounds, drive, router, DriveArgs};
use serde_json::Value;
use tempfile::TempDir;

const FAKE_CLAUDE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fake-claude.sh");

/// The clean-success capture named in `tests/fixtures/transcripts/README.md`.
///
/// **It is the right fixture precisely because it changes nothing.** The agent
/// replays a successful transcript and writes not a byte into `.planning/`, so
/// two consecutive iterations observe the same project — which is the condition
/// the no-progress and command-repeat detectors exist to notice, reproduced
/// without having to make an agent misbehave.
const CLEAN_BASELINE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/transcripts/01-success-textonly.ndjson"
);

const ALIAS: &str = "iterloop";

/// The phase the fixture project declares and the router is pointed at.
const TARGET_PHASE: &str = "20";

/// The one command this plan's rule table can produce, spelled out here rather
/// than imported, so a change to the rule fails this test instead of agreeing
/// with itself.
const ROUTED_COMMAND: &str = "/gsd-plan-phase 20";

// ---------------------------------------------------------------------------
// Fixture
// ---------------------------------------------------------------------------

/// A project whose roadmap declares phase 20 and whose phase directory holds a
/// context artifact, so the router's single forward rule fires.
///
/// Both halves are load-bearing and the router refuses without either: the
/// roadmap entry is what makes the phase *corroborated* rather than
/// inferred-only state, and the `*-CONTEXT.md` is what makes its disk status
/// `Discussed`.
fn project_root() -> TempDir {
    let root = TempDir::new().expect("temp dir");
    let planning = root.path().join(".planning");
    std::fs::create_dir_all(&planning).expect("scratch .planning");

    std::fs::write(
        planning.join("ROADMAP.md"),
        "# Roadmap\n\n- [ ] **Phase 20: Deterministic Router** - the phase under test\n",
    )
    .expect("write ROADMAP.md");

    let phase_dir = planning.join("phases").join("20-deterministic-router");
    std::fs::create_dir_all(&phase_dir).expect("scratch phase dir");
    std::fs::write(phase_dir.join("20-CONTEXT.md"), "# Context\n").expect("write phase context");

    root
}

/// Point the envelope at a temp root for this test binary.
///
/// Verbatim in intent from `tests/driver_tracer.rs`: without the redirect these
/// fixtures would write hook stubs, a generated git config and a settings file
/// into the developer's real `~/.local/share` under a fixture's alias.
fn isolate_envelope_root() {
    static ROOT: std::sync::OnceLock<TempDir> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = TempDir::new().expect("an envelope temp root");
        std::env::set_var(gsd_meta_manager::envelope::ENVELOPE_ROOT_ENV, dir.path());
        dir
    });
}

/// A one-entry registry pointing at `root`, opted in to being driven.
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

/// A routed `DriveArgs` pointed at the transcript-replaying stand-in.
fn routed_args(run_id: &str, max_steps: Option<u32>) -> DriveArgs {
    DriveArgs {
        alias: ALIAS.to_string(),
        command: None,
        target_phase: Some(TARGET_PHASE.to_string()),
        max_steps,
        wall_clock_cap_secs: None,
        run_id: Some(run_id.to_string()),
        dry_run: false,
        goal: Some("drive phase 20 forward".to_string()),
        claude_program: Some(FAKE_CLAUDE.into()),
        claude_args: vec![OsString::from(CLEAN_BASELINE), OsString::from("0")],
    }
}

/// Every record in `run_id`'s journal, in order, read with the shipped reader.
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

/// The parsed `run.json` for `run_id`.
fn run_record(root: &Path, run_id: &str) -> Value {
    let paths = gsd_meta_manager::journal::run_paths(&root.join(".planning"), run_id)
        .expect("the fixture run id is a plain path component");
    let raw = std::fs::read_to_string(&paths.run_json).expect("run.json exists and is readable");
    serde_json::from_str(&raw).expect("run.json parses")
}

fn of_kind<'a>(records: &'a [Value], kind: &str) -> Vec<&'a Value> {
    records
        .iter()
        .filter(|record| record["kind"] == kind)
        .collect()
}

// ---------------------------------------------------------------------------
// The end-to-end proof
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_routed_run_issues_two_commands_and_halts_naming_the_detector_that_fired() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-loop";

    let root = project_root();
    let config = config_for(root.path());

    drive(routed_args(RUN_ID, None), &config)
        .await
        .expect("a routed run that halts on a bound is an ordinary end, not an error");

    let records = journal_records(root.path(), RUN_ID);

    // ---- The loop ran twice through the router --------------------------
    let decided = of_kind(&records, "decided");
    assert_eq!(
        decided.len(),
        2,
        "a routed run must journal one `decided` record per iteration. One record \
         would mean the outer loop never came round — which is the entire claim \
         this phase makes, since `DriveArgs::command` was a single String and \
         three source sites said in as many words that no command loop existed. \
         Got: {records:#?}"
    );
    for record in &decided {
        assert_eq!(
            record["command"], ROUTED_COMMAND,
            "both iterations observed the same unchanged project, so the \
             deterministic router must choose the same command both times. A \
             different second choice would mean the decision depended on \
             something other than the state handed in"
        );
        assert_eq!(
            record["by"], "policy",
            "`by` is a closed three-value vocabulary and this phase emits only \
             the policy value; `llm` is Phase 21's and a fourth value would be a \
             schema change nobody declared"
        );
        assert_eq!(
            record["rationale"], router::RATIONALE_READY_TO_PLAN,
            "the rationale is a &'static str from the router's closed set, never \
             anything the agent wrote (SAFE-04, T-20-05)"
        );
    }

    // ---- What it observed is on disk, in the documented shape -------------
    let observed = of_kind(&records, "observed");
    assert_eq!(
        observed.len(),
        2,
        "one observation per iteration: a decision with no recorded observation \
         asks a later reader to take the router's input on trust"
    );
    for record in &observed {
        assert_eq!(
            record["phase"], TARGET_PHASE,
            "the observation must name the phase it is about"
        );
        let drpev = record["drpev"]
            .as_array()
            .expect("drpev is an array of stage statuses");
        assert_eq!(
            drpev.len(),
            5,
            "`Observed.drpev` is documented as the FIVE D-R-P-E-V stage statuses \
             in order — a fixed-length vector, not a free-form list. A shorter one \
             cannot be read positionally, which is the only way it is useful"
        );
    }

    // ---- Exactly one detector is reported, and it is named ----------------
    let parked = of_kind(&records, "parked");
    assert_eq!(
        parked.len(),
        1,
        "a run halts once and says why once. Two park records would make 'which \
         detector fired' ambiguous, which is exactly what CTRL-06's criterion \
         asks to be answerable"
    );
    assert_eq!(
        parked[0]["reason"], bounds::REASON_COMMAND_REPEAT,
        "with the wall-clock and step caps at their defaults and only one \
         unchanged pair observed, command-repeat is the first — and the only — \
         detector in the documented order whose condition holds. The order is \
         wall-clock, step cap, no progress, command repeat; \
         `the_documented_evaluation_order_decides_when_every_condition_holds_at_once` \
         in src/driver/bounds.rs pins the order itself"
    );
    assert_eq!(
        parked[0]["needs"], "human",
        "a halted run needs a person to decide whether to raise the bound or fix \
         the stall; the driver may not decide that for itself"
    );

    // ---- The halt happened BEFORE the second spawn -----------------------
    assert_eq!(
        of_kind(&records, "exec_started").len(),
        1,
        "the bound must stop the run before it spawns, not after. A second \
         exec_started would mean the detector reported a halt while the command \
         it refused was already running"
    );

    // ---- One run, one journal, one terminal record ------------------------
    assert_eq!(
        of_kind(&records, "run_started").len(),
        1,
        "two commands under ONE run_started is the D-20.2 property: one process, \
         one lock, one journal, one terminal record"
    );
    assert_eq!(
        of_kind(&records, "run_ended").len(),
        1,
        "exactly one terminal record, and it is the last"
    );
    assert_eq!(
        records.last().expect("the journal is non-empty")["kind"],
        "run_ended"
    );

    // ---- And a separate process can read the reason off disk --------------
    let record = run_record(root.path(), RUN_ID);
    assert_eq!(
        record["outcome"],
        format!("parked:{}", bounds::REASON_COMMAND_REPEAT),
        "the terminal record carries the reason through the `parked:` prefix that \
         Phase 19 established and proved readable by a separate process — not \
         through a second carrier invented for the bounds"
    );
    assert!(
        record["ended_at"].is_string(),
        "a run that halted itself still ends cleanly; an absent ended_at is the \
         crash signal (D-12) and a self-halt is not a crash"
    );
    assert_eq!(
        record["gsd_command"],
        format!("--target-phase {TARGET_PHASE}"),
        "`run.json` carries a single command field and is written exactly twice, \
         so a routed run records the TARGET that bounded the sequence rather than \
         a guess at which command the router picked. The sequence itself is on the \
         `decided` records"
    );
}

#[tokio::test]
async fn the_step_cap_halts_a_routed_run_and_reports_itself_rather_than_the_agents_timeout() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-step";

    let root = project_root();
    let config = config_for(root.path());

    // A cap of one: iteration one runs, iteration two is refused. The step cap
    // sits ABOVE command-repeat in the documented order, so this also proves the
    // order is real end to end rather than only in the unit tests — the same run
    // shape halts under `bounds_command_repeat` when the cap is left at its
    // default.
    drive(routed_args(RUN_ID, Some(1)), &config)
        .await
        .expect("a routed run that halts on its step cap is an ordinary end");

    let records = journal_records(root.path(), RUN_ID);

    let parked = of_kind(&records, "parked");
    assert_eq!(parked.len(), 1, "one halt, one reason");
    assert_eq!(
        parked[0]["reason"], bounds::REASON_STEP_CAP,
        "with max_steps of 1 the first iteration runs and the second is refused, \
         and the reason must be the RUN's step cap rather than the executor's \
         `timed_out`. CTRL-06 asks a run that exceeds its own bound to say so \
         distinguishably from an agent that hung"
    );

    assert_eq!(
        of_kind(&records, "exec_started").len(),
        1,
        "max_steps of 1 means exactly one command runs"
    );
    assert_eq!(
        run_record(root.path(), RUN_ID)["outcome"],
        format!("parked:{}", bounds::REASON_STEP_CAP)
    );
}

#[tokio::test]
async fn single_command_mode_is_untouched_by_the_iteration_loop() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-single";

    let root = project_root();
    let config = config_for(root.path());

    let mut args = routed_args(RUN_ID, None);
    args.command = Some("/gsd-progress".to_string());
    args.target_phase = None;

    drive(args, &config)
        .await
        .expect("single-command mode must still complete exactly as it did");

    let records = journal_records(root.path(), RUN_ID);

    // The negative half is the whole test: single-command mode takes none of
    // the routed path, so it observes nothing, decides nothing and parks for no
    // reason of its own.
    assert!(
        of_kind(&records, "observed").is_empty(),
        "single-command mode captures no snapshot and journals no observation; a \
         record here would mean the routed path leaked into the mode this phase \
         promised to leave byte-for-byte alone"
    );
    assert!(of_kind(&records, "decided").is_empty());
    assert!(
        of_kind(&records, "parked").is_empty(),
        "no bound applies to a run that was asked for exactly one command"
    );
    assert_eq!(
        of_kind(&records, "exec_started").len(),
        1,
        "exactly one command, exactly one spawn"
    );

    let record = run_record(root.path(), RUN_ID);
    assert_eq!(
        record["gsd_command"], "/gsd-progress",
        "the record still carries the command the run was started with"
    );
    assert_eq!(
        record["outcome"], "succeeded_no_changes",
        "and the terminal label is still `outcome_label`'s, with no `parked:` \
         prefix — the exact string a Phase 17 run wrote for this transcript"
    );
}
