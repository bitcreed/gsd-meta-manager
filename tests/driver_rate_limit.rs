// ============================================================================
// The subscription quota park: a rate-limited run STOPS, names the window that
// blocked it and when it resets, and does not retry (CTRL-07, DRIVE-06).
//
// **THE RULE THIS FILE IS WRITTEN UNDER, borrowed verbatim from
// `tests/driver_iteration_loop.rs`: no assertion may read a model's summary.**
// Every fact below is a record read back out of `journal.jsonl` or a field of
// `run.json`, both by the shipped reader and both after the driven process has
// exited.
//
// **Why the transcripts here are built rather than captured.** A `rejected`
// rate-limit event cannot be captured without burning the very quota it
// describes, so the payloads come from the research document's binary-verified
// field inventory. The one that is committed is
// `tests/fixtures/transcripts/09-rate-limit-rejected.ndjson`, and the
// transcript directory's README labels it as synthesised for exactly that
// reason. The rest are assembled at test time from the committed clean baseline
// so that every envelope around the event — the `system/init` the capability
// gate reads, the `result` the outcome derivation reads — is a real capture.
//
// Unix-only by construction: driving is a Unix capability (D-05), and the run
// body is `#[cfg(unix)]`. Debug-only in practice too — `--claude-program` has
// no parser entry in a release build (D-30, WR-16).
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::path::Path;

use gsd_meta_manager::config::{Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::{drive, rate_limit, DriveArgs};
use serde_json::Value;
use tempfile::TempDir;

const FAKE_CLAUDE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fake-claude.sh");

/// The committed clean-success capture, used as the scaffolding every built
/// transcript is assembled from.
///
/// Every line except the rate-limit one is a real capture, so the capability
/// gate, the framing and the outcome derivation are all exercised against the
/// wire rather than against something written to satisfy them.
const CLEAN_BASELINE: &str =
    include_str!("fixtures/transcripts/01-success-textonly.ndjson");

/// The committed **synthesised** rejection transcript.
///
/// The one status no capture exists for. `tests/fixtures/transcripts/README.md`
/// labels it as synthesised and records the provenance of every field name and
/// enum value in it; the tests below enforce that label rather than trusting it.
const REJECTED_FIXTURE: &str = include_str!("fixtures/transcripts/09-rate-limit-rejected.ndjson");

const REJECTED_FIXTURE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/transcripts/09-rate-limit-rejected.ndjson"
);

/// The directory the fixture inventory is checked against.
const TRANSCRIPT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/transcripts");

const TRANSCRIPT_README: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/transcripts/README.md"
);

const ALIAS: &str = "quota";

/// The phase the fixture project declares and the router is pointed at.
const TARGET_PHASE: &str = "20";

/// A step cap deliberately greater than one, so "no further iteration was
/// spawned" is a fact about the quota park rather than about the cap.
const ROOMY_STEP_CAP: u32 = 5;

/// The window the synthesised rejections name.
const REJECTED_WINDOW: &str = rate_limit::WINDOW_SEVEN_DAY;

// ---------------------------------------------------------------------------
// Transcript construction
// ---------------------------------------------------------------------------

/// Whether `line` is the baseline's rate-limit event.
fn is_rate_limit_line(line: &str) -> bool {
    line.contains(r#""type":"rate_limit_event""#)
}

/// The clean baseline with its rate-limit line replaced by `event`, or dropped
/// entirely when `event` is `None`.
///
/// The line count and the envelope ORDER are preserved in the replacement case,
/// which is what keeps the built transcript the same shape as a capture.
fn baseline_with_quota_event(event: Option<&str>) -> String {
    let mut out = String::new();
    for line in CLEAN_BASELINE.lines().filter(|line| !line.trim().is_empty()) {
        if is_rate_limit_line(line) {
            match event {
                Some(replacement) => {
                    out.push_str(replacement);
                    out.push('\n');
                }
                None => continue,
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// A `rate_limit_event` line carrying `status`, `rateLimitType` and, when
/// supplied, `resetsAt` — field names verbatim from the binary-verified
/// inventory in the research document.
fn quota_event_line(status: &str, window: &str, resets_at: Option<i64>) -> String {
    let mut info = serde_json::Map::new();
    info.insert(
        rate_limit::STATUS_FIELD.to_string(),
        Value::from(status.to_string()),
    );
    if let Some(secs) = resets_at {
        info.insert(rate_limit::RESETS_AT_FIELD.to_string(), Value::from(secs));
    }
    info.insert(
        rate_limit::RATE_LIMIT_TYPE_FIELD.to_string(),
        Value::from(window.to_string()),
    );
    info.insert("isUsingOverage".to_string(), Value::from(false));

    let event = serde_json::json!({
        "type": "rate_limit_event",
        rate_limit::RATE_LIMIT_INFO_FIELD: Value::Object(info),
        "uuid": "11111111-1111-4111-8111-000000000002",
        "session_id": "00000000-0000-4000-8000-000000000001",
    });
    serde_json::to_string(&event).expect("the built event serialises")
}

/// The clean baseline with **no** rate-limit event and a terminal `result`
/// rewritten to fail with `terminal_reason`.
///
/// This is research assumption A2's case: the rejection arriving only on the
/// failure envelope, with no `rate_limit_event` on the stream at all.
fn baseline_failing_with(terminal_reason: &str) -> String {
    let mut out = String::new();
    let lines: Vec<&str> = CLEAN_BASELINE
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !is_rate_limit_line(line))
        .collect();

    for (index, line) in lines.iter().enumerate() {
        if index + 1 == lines.len() {
            let mut envelope: Value =
                serde_json::from_str(line).expect("the baseline's result envelope parses");
            let object = envelope
                .as_object_mut()
                .expect("a result envelope is an object");
            object.insert("is_error".to_string(), Value::from(true));
            object.insert(
                "subtype".to_string(),
                Value::from("error_during_execution"),
            );
            object.insert(
                "terminal_reason".to_string(),
                Value::from(terminal_reason.to_string()),
            );
            out.push_str(
                &serde_json::to_string(&envelope).expect("the rewritten envelope serialises"),
            );
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

/// Write `transcript` into `dir` and return its path.
fn transcript_file(dir: &TempDir, name: &str, transcript: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, transcript).expect("write the built transcript");
    path
}

// ---------------------------------------------------------------------------
// Fixture — the same shape `tests/driver_iteration_loop.rs` uses
// ---------------------------------------------------------------------------

/// A project whose roadmap declares phase 20 and whose phase directory holds a
/// context artifact, so the router's single forward rule fires.
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

/// Point the envelope at a temp root for this test binary, so no fixture writes
/// hook stubs or a settings file into the developer's real `~/.local/share`.
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

/// A routed `DriveArgs` replaying `transcript` and exiting `exit_code`.
fn routed_args(run_id: &str, transcript: &Path, exit_code: &str) -> DriveArgs {
    DriveArgs {
        alias: ALIAS.to_string(),
        command: None,
        target_phase: Some(TARGET_PHASE.to_string()),
        max_steps: Some(ROOMY_STEP_CAP),
        wall_clock_cap_secs: None,
        max_escalations: None,
        run_id: Some(run_id.to_string()),
        dry_run: false,
        goal: Some("drive phase 20 forward".to_string()),
        claude_program: Some(FAKE_CLAUDE.into()),
        claude_args: vec![
            OsString::from(transcript.as_os_str()),
            OsString::from(exit_code),
        ],
    }
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

/// The `ts` of `record`, parsed.
fn stamp(record: &Value) -> chrono::DateTime<chrono::Utc> {
    let raw = record["ts"]
        .as_str()
        .unwrap_or_else(|| panic!("every journal record carries a ts: {record}"));
    chrono::DateTime::parse_from_rfc3339(raw)
        .unwrap_or_else(|err| panic!("the ts parses ({raw}): {err}"))
        .with_timezone(&chrono::Utc)
}

/// The single `parked` record, with the `diagnostic` that carries its detail.
fn park_and_detail(records: &[Value]) -> (Value, String) {
    let parked = of_kind(records, "parked");
    assert_eq!(
        parked.len(),
        1,
        "a run stops once and says why once. Two park records would make 'why \
         did this run end' ambiguous, which is the one question the terminal \
         record exists to answer. Got: {records:#?}"
    );

    let detail = of_kind(records, "diagnostic")
        .into_iter()
        .filter(|record| record["code"] == rate_limit::REASON_QUOTA_REJECTED)
        .map(|record| record["detail"].as_str().unwrap_or_default().to_string())
        .next_back()
        .unwrap_or_default();

    (parked[0].clone(), detail)
}

// ---------------------------------------------------------------------------
// The one outcome entry point is NOT widened to reach this signal
// ---------------------------------------------------------------------------

const OUTCOME_SOURCE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/executor/outcome.rs");

const DECLARATION: &str = "pub fn derive_run_outcome_from_envelopes(";

/// The signature that must not change, whitespace-normalised.
const EXPECTED_SIGNATURE: &str = "pub fn derive_run_outcome_from_envelopes( \
     envelopes: &[ResultMessage], exit: Option<ExitStatus>, before: &RunSnapshot, \
     after: &RunSnapshot, ) -> RunOutcome";

/// The declaration of [`DECLARATION`] in `source`, whitespace-normalised, or
/// `None` when the declaration is not there at all.
fn signature_of(source: &str) -> Option<String> {
    let start = source.find(DECLARATION)?;
    let rest = &source[start..];
    let end = rest.find('{')?;
    Some(rest[..end].split_whitespace().collect::<Vec<_>>().join(" "))
}

#[test]
fn the_one_outcome_entry_point_still_takes_exactly_the_full_result_envelopes() {
    let source =
        std::fs::read_to_string(OUTCOME_SOURCE).expect("the outcome module is readable");
    let observed = signature_of(&source).unwrap_or_else(|| {
        panic!("`{DECLARATION}` must exist in {OUTCOME_SOURCE}; the scan found nothing to check")
    });

    assert_eq!(
        observed, EXPECTED_SIGNATURE,
        "the rate-limit signal must be observed in the driver's drain loop, \
         where it already arrives, and NOT by widening this function to reach \
         it. `derive_run_outcome_from_envelopes` deliberately takes the full \
         `result` envelopes precisely so no envelope-discarding sibling path can \
         exist: one used to sit beside it, the production coordinator called that \
         one, and a `--permission-mode dontAsk` run that was blocked from doing \
         anything reported as a plain success (CR-04). A `rate_limit_event` is \
         not a `result` envelope and never enters that vector, so reaching it \
         from here requires exactly the widening that made CR-04 possible"
    );
}

#[test]
fn the_signature_scanner_fires_on_a_synthetic_widened_declaration() {
    // The control arm, and without it the assertion above could pass vacuously
    // forever: a `find` that had quietly stopped matching — a reformatted
    // declaration, a renamed function — would make `signature_of` return `None`
    // or a truncated string, and a matcher that matches nothing agrees with
    // every tree.
    const WIDENED: &str = "pub fn derive_run_outcome_from_envelopes(\n    \
         envelopes: &[ResultMessage],\n    rate_limit: Option<&serde_json::Value>,\n    \
         exit: Option<ExitStatus>,\n    before: &RunSnapshot,\n    \
         after: &RunSnapshot,\n) -> RunOutcome {\n    todo!()\n}";

    let widened = signature_of(WIDENED)
        .expect("the matcher must still FIND a declaration, or it proves nothing");
    assert!(
        widened.contains("rate_limit: Option<&serde_json::Value>"),
        "the matcher must capture the whole parameter list, or a widening could \
         hide past its end: {widened}"
    );
    assert_ne!(
        widened, EXPECTED_SIGNATURE,
        "and it must REJECT the widened form. A scanner that accepted this would \
         report the tree clean while the exact change it exists to catch had \
         landed"
    );

    assert_eq!(
        signature_of("fn something_else() -> u8 { 0 }"),
        None,
        "and it must find nothing where the declaration is absent, so a renamed \
         or deleted entry point fails loudly rather than silently passing"
    );
}

// ---------------------------------------------------------------------------
// A rejected quota event parks the run
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_rejected_quota_event_parks_the_run_and_names_the_window() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-quota";

    let root = project_root();
    let config = config_for(root.path());
    let scratch = TempDir::new().expect("temp dir");
    // A reset time inside the sanity bound relative to a real `now`, so the
    // detail carries a rendered instant rather than `unknown`.
    let resets_at = chrono::Utc::now().timestamp() + 2 * 24 * 60 * 60;
    let transcript = transcript_file(
        &scratch,
        "rejected.ndjson",
        &baseline_with_quota_event(Some(&quota_event_line(
            rate_limit::STATUS_REJECTED,
            REJECTED_WINDOW,
            Some(resets_at),
        ))),
    );

    drive(routed_args(RUN_ID, &transcript, "0"), &config)
        .await
        .expect("a run that parks on a quota is an ordinary end, not an error");

    let records = journal_records(root.path(), RUN_ID);
    let (parked, detail) = park_and_detail(&records);

    assert_eq!(
        parked["reason"], rate_limit::REASON_QUOTA_REJECTED,
        "the park must carry the quota taxonomy's own reason, not a bounds or \
         router one: `which detector fired` and `which quota window blocked me` \
         are different questions and a shared identifier makes both unanswerable"
    );
    assert_eq!(
        parked["needs"], "human",
        "only a person can decide whether to wait out the window or run \
         somewhere else; the driver may not decide that, and specifically may \
         not decide it by sleeping"
    );
    assert!(
        detail.contains(REJECTED_WINDOW),
        "CTRL-07 asks the run to report WHICH quota window blocked it, and the \
         payload's own rateLimitType field is where that comes from. Got: \
         {detail}"
    );
    assert!(
        !detail.contains(rate_limit::UNKNOWN),
        "the reset time was inside the sanity bound, so it must be reported \
         rather than suppressed: {detail}"
    );

    for forbidden in ["$", "usd", "USD"] {
        assert!(
            !detail.contains(forbidden),
            "no dollar figure may be presented as what this run cost (D-16). \
             Under subscription auth `total_cost_usd` is a notional \
             API-equivalent price and not a charge, and the quota — not a \
             budget — is the constraint the user is deciding about. Found \
             `{forbidden}` in: {detail}"
        );
    }

    assert_eq!(
        run_record(root.path(), RUN_ID)["outcome"],
        format!("parked:{}", rate_limit::REASON_QUOTA_REJECTED),
        "the reason reaches a separate process through the same `parked:` prefix \
         every other park uses, so one grep answers why a run ended across the \
         safety envelope, the router, the bounds and the quota at once"
    );
}

#[tokio::test]
async fn a_rejection_with_no_usable_reset_time_still_parks_and_says_unknown() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-noreset";

    let root = project_root();
    let config = config_for(root.path());
    let scratch = TempDir::new().expect("temp dir");
    let transcript = transcript_file(
        &scratch,
        "no-reset.ndjson",
        &baseline_with_quota_event(Some(&quota_event_line(
            rate_limit::STATUS_REJECTED,
            rate_limit::WINDOW_FIVE_HOUR,
            None,
        ))),
    );

    drive(routed_args(RUN_ID, &transcript, "0"), &config)
        .await
        .expect("a run that parks on a quota is an ordinary end");

    let records = journal_records(root.path(), RUN_ID);
    let (parked, detail) = park_and_detail(&records);

    assert_eq!(parked["reason"], rate_limit::REASON_QUOTA_REJECTED);
    assert!(
        detail.contains(rate_limit::WINDOW_FIVE_HOUR),
        "the window is still known and must still be named: {detail}"
    );
    assert!(
        detail.contains(rate_limit::UNKNOWN),
        "an unreadable reset time is SAID rather than omitted or estimated. An \
         omission is indistinguishable from a build that never recorded the \
         field, and an estimate is a nonsense timestamp presented as fact to a \
         human deciding when to come back. Got: {detail}"
    );
}

#[tokio::test]
async fn a_failure_whose_terminal_reason_names_a_rate_limit_parks_under_the_same_reason() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-second";

    let root = project_root();
    let config = config_for(root.path());
    let scratch = TempDir::new().expect("temp dir");
    // No rate_limit_event on this stream AT ALL: research assumption A2's case,
    // where the rejection arrives only on the failure envelope.
    let transcript = transcript_file(
        &scratch,
        "api-error.ndjson",
        &baseline_failing_with("api_error_rate_limit"),
    );

    drive(routed_args(RUN_ID, &transcript, "1"), &config)
        .await
        .expect("a run that parks on a quota is an ordinary end");

    let records = journal_records(root.path(), RUN_ID);
    assert!(
        of_kind(&records, "exec_event")
            .iter()
            .all(|record| record["stream"] != "rate_limit_event"),
        "this fixture's whole point is that no quota event was ever observed, so \
         the first detector had nothing to read: {records:#?}"
    );

    let (parked, detail) = park_and_detail(&records);
    assert_eq!(
        parked["reason"],
        rate_limit::REASON_QUOTA_REJECTED,
        "no capture of a rejected rate_limit_event can be produced without \
         burning quota, so the failure envelope's own terminal reason is the \
         second detector. It reads a field the driver already holds, so nothing \
         about the outcome derivation changes"
    );
    assert!(
        detail.contains(rate_limit::UNKNOWN),
        "the window is genuinely unknown here: the only payload this detector \
         could borrow one from is an `allowed` event, which by definition did \
         not describe this refusal. Reporting a plausible window would be a \
         guess wearing a fact's clothes. Got: {detail}"
    );
}

#[tokio::test]
async fn a_rejection_followed_by_an_allowed_event_still_parks_the_run() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-latched";

    let root = project_root();
    let config = config_for(root.path());
    let scratch = TempDir::new().expect("temp dir");

    // **Two events on one stream, in the order that used to lose the first**
    // (WR-02). The CLI emits one `rate_limit_event` per window, and a steered
    // run emits one per turn, so `rejected` on the five-hour window followed by
    // `allowed` on the seven-day one is an ordinary shape rather than a
    // contrived one. Retaining the LATEST event left the driver classifying the
    // `allowed` payload: `classify` answered `Allowed`, the second detector saw
    // a successful run rather than a failed one, and the loop went round to
    // spend more of a quota that had already refused it.
    let transcript = transcript_file(
        &scratch,
        "rejected-then-allowed.ndjson",
        &baseline_with_quota_event(Some(&format!(
            "{}\n{}",
            quota_event_line(
                rate_limit::STATUS_REJECTED,
                rate_limit::WINDOW_FIVE_HOUR,
                None,
            ),
            quota_event_line(
                "allowed",
                rate_limit::WINDOW_SEVEN_DAY,
                Some(chrono::Utc::now().timestamp() + 2 * 24 * 60 * 60),
            ),
        ))),
    );

    drive(routed_args(RUN_ID, &transcript, "0"), &config)
        .await
        .expect("a run that parks on a quota is an ordinary end");

    let records = journal_records(root.path(), RUN_ID);
    let (parked, detail) = park_and_detail(&records);

    assert_eq!(
        parked["reason"], rate_limit::REASON_QUOTA_REJECTED,
        "a rejection is a fact about the whole iteration and a later event about \
         a DIFFERENT window does not undo it. Got: {records:#?}"
    );
    assert!(
        detail.contains(rate_limit::WINDOW_FIVE_HOUR),
        "and the window reported must be the one that REFUSED the run, not the \
         one that happened to be described last: {detail}"
    );
    assert_eq!(
        of_kind(&records, "exec_started").len(),
        1,
        "no second command may run after a rejection, whatever arrived on the \
         stream afterwards — CTRL-07's prohibition is on retrying at all"
    );
}

// ---------------------------------------------------------------------------
// The park STOPS the run: no next iteration, and no sleep
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_further_iteration_is_spawned_after_a_quota_park() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-stop";

    let root = project_root();
    let config = config_for(root.path());
    let scratch = TempDir::new().expect("temp dir");
    let transcript = transcript_file(
        &scratch,
        "rejected.ndjson",
        &baseline_with_quota_event(Some(&quota_event_line(
            rate_limit::STATUS_REJECTED,
            REJECTED_WINDOW,
            None,
        ))),
    );

    // A step cap of five, so a second iteration is permitted by every bound. If
    // one is spawned anyway it is the quota park that failed to stop the run.
    drive(routed_args(RUN_ID, &transcript, "0"), &config)
        .await
        .expect("a run that parks on a quota is an ordinary end");

    let records = journal_records(root.path(), RUN_ID);

    assert_eq!(
        of_kind(&records, "decided").len(),
        1,
        "the router must be asked exactly once. This run's transcript succeeds, \
         so without the quota park the loop would come round and route again — \
         and the quota is shared with every other Claude surface the user has, \
         so the next command spends budget the user did not allocate to this run"
    );
    assert_eq!(
        of_kind(&records, "exec_started").len(),
        1,
        "and exactly one command runs. A second exec_started is a RETRY, which \
         CTRL-07 forbids outright"
    );
    assert_eq!(
        of_kind(&records, "run_ended").len(),
        1,
        "one run, one terminal record"
    );
    assert_eq!(
        records.last().expect("the journal is non-empty")["kind"],
        "run_ended",
        "and it is the last record"
    );
}

#[tokio::test]
async fn no_sleep_until_reset_is_inserted_between_the_park_and_the_terminal_record() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-nosleep";
    /// Teardown alone. A sleep-until-reset would be hours, so any bound in
    /// seconds is decisive; this one is loose enough to survive a loaded CI box.
    const TEARDOWN_BUDGET_SECS: i64 = 30;

    let root = project_root();
    let config = config_for(root.path());
    let scratch = TempDir::new().expect("temp dir");
    // A reset time two days out — the value a backoff would sleep until.
    let resets_at = chrono::Utc::now().timestamp() + 2 * 24 * 60 * 60;
    let transcript = transcript_file(
        &scratch,
        "rejected.ndjson",
        &baseline_with_quota_event(Some(&quota_event_line(
            rate_limit::STATUS_REJECTED,
            REJECTED_WINDOW,
            Some(resets_at),
        ))),
    );

    drive(routed_args(RUN_ID, &transcript, "0"), &config)
        .await
        .expect("a run that parks on a quota is an ordinary end");

    let records = journal_records(root.path(), RUN_ID);
    let parked = of_kind(&records, "parked");
    assert_eq!(parked.len(), 1);
    // The non-vacuity half. Without it this test measures the gap after
    // WHICHEVER park happened to end the run — and this transcript succeeds, so
    // a build with no quota detector at all parks on `bounds_command_repeat`
    // just as promptly and the timing assertion below passes while proving
    // nothing about the quota path.
    assert_eq!(
        parked[0]["reason"], rate_limit::REASON_QUOTA_REJECTED,
        "the run must have ended on the QUOTA park for the interval below to be \
         the interval this test is about"
    );
    let ended = of_kind(&records, "run_ended");
    assert_eq!(ended.len(), 1);

    let elapsed = stamp(ended[0])
        .signed_duration_since(stamp(parked[0]))
        .num_seconds();
    assert!(
        (0..=TEARDOWN_BUDGET_SECS).contains(&elapsed),
        "the run must end immediately after the park. A backoff IS a retry with \
         a delay, and one that waits out a shared 5-hour or 7-day window holds \
         the project lock for the whole wait while spending nothing usefully. \
         The reset this fixture named was two days out; the gap between park and \
         terminal record was {elapsed}s, budget {TEARDOWN_BUDGET_SECS}s"
    );
}

// ---------------------------------------------------------------------------
// The committed synthesised fixture, and the label that keeps it honest
// ---------------------------------------------------------------------------

#[test]
fn every_line_of_the_synthesised_fixture_parses_through_the_shipped_parser() {
    let lines: Vec<&str> = REJECTED_FIXTURE.lines().collect();
    assert!(
        lines.len() >= 4,
        "the fixture must carry the whole envelope sequence a real capture does \
         — init, the quota event, the replay and the terminal result — or it \
         proves nothing about how the driver reads one. Got {} lines",
        lines.len()
    );

    for (index, line) in lines.iter().enumerate() {
        match gsd_meta_manager::executor::stream_json::parse_line(line) {
            gsd_meta_manager::executor::stream_json::Envelope::Parsed { .. } => {}
            other => panic!(
                "line {} of the synthesised fixture did not parse, so the fixture \
                 does not match the wire it claims to: {other:?}",
                index + 1
            ),
        }
    }
}

#[test]
fn the_synthesised_fixture_matches_the_stated_ndjson_format() {
    let raw = std::fs::read_to_string(REJECTED_FIXTURE_PATH).expect("the fixture is readable");

    assert!(
        raw.ends_with('\n') && !raw.ends_with("\n\n"),
        "the directory's README states the format: one JSON object per line, ONE \
         trailing newline, no blank lines. A missing newline makes the last line \
         invisible to a line-oriented reader and a second one makes an empty \
         line the parser is then asked to explain"
    );
    for (index, line) in raw.lines().enumerate() {
        assert!(
            !line.trim().is_empty(),
            "line {} is blank, and the stated format has none",
            index + 1
        );
    }
}

#[test]
fn the_synthesised_fixtures_quota_event_classifies_as_a_rejection() {
    let payload = REJECTED_FIXTURE
        .lines()
        .filter(|line| is_rate_limit_line(line))
        .map(|line| serde_json::from_str::<Value>(line).expect("the quota line parses"))
        .next()
        .expect("the fixture carries a rate_limit_event");

    // Judged against a `now` one hour before the fixture's own reset value, so
    // this assertion is as true in ten years as it is today. A `now` read from
    // the wall clock would make it pass this month and fail the next.
    let resets_at = payload[rate_limit::RATE_LIMIT_INFO_FIELD][rate_limit::RESETS_AT_FIELD]
        .as_i64()
        .expect("the fixture's resetsAt is an integer");
    let now = chrono::DateTime::from_timestamp(resets_at - 3_600, 0).expect("representable");

    match rate_limit::classify(Some(&payload), now) {
        rate_limit::QuotaVerdict::Rejected { window, resets_at: at } => {
            assert_eq!(
                window,
                rate_limit::QuotaWindow::SevenDay,
                "the fixture names a seven_day window, and the whole value of a \
                 synthesised file is that its enum values are the wire's rather \
                 than plausible ones"
            );
            assert_eq!(
                at.map(|at| at.timestamp()),
                Some(resets_at),
                "and its reset time is inside the sanity bound and round-trips to \
                 the epoch second on the wire"
            );
        }
        other => panic!(
            "the one status no capture exists for must classify as a rejection, \
             or nothing in this tree exercises the park at all: {other:?}"
        ),
    }
}

#[test]
fn the_transcript_readme_labels_the_synthesised_file_and_its_inventory_agrees_with_disk() {
    let readme = std::fs::read_to_string(TRANSCRIPT_README).expect("the README is readable");

    assert!(
        !readme.contains("Every `.ndjson` file in this directory is a **real capture**"),
        "the opening contract used to assert that every file here is a capture. \
         That became FALSE the moment a synthesised one landed, and a contract \
         left to be discovered by a reader is the quiet lie this directory's own \
         redaction record exists to prevent"
    );
    assert!(
        readme.contains("09-rate-limit-rejected.ndjson"),
        "the README must name the synthesised file"
    );
    assert!(
        readme.contains("synthesised") || readme.contains("SYNTHESISED"),
        "and must say plainly that it is synthesised, so no later reader mistakes \
         it for a capture"
    );

    // The inventory, enforced in BOTH directions: a file the README does not
    // describe is an undocumented fixture, and a file the README describes that
    // is not on disk is a description of something that no longer exists.
    let mut on_disk: Vec<String> = std::fs::read_dir(TRANSCRIPT_DIR)
        .expect("the transcript directory is readable")
        .map(|entry| entry.expect("a readable dir entry").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".ndjson"))
        .collect();
    on_disk.sort();

    let mut described: Vec<String> = readme
        .lines()
        .filter(|line| line.starts_with("| `"))
        .filter_map(|line| line.split('`').nth(1).map(str::to_string))
        .filter(|name| name.ends_with(".ndjson"))
        .collect();
    described.sort();

    assert_eq!(
        described, on_disk,
        "the README's per-file table and the directory's contents must agree. A \
         fixture nobody described is one a later reader cannot tell a capture \
         from a construction, and a described file that is gone is a claim about \
         evidence that is not there"
    );
}

#[tokio::test]
async fn the_committed_rejection_fixture_parks_a_real_run() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-fixture";

    let root = project_root();
    let config = config_for(root.path());

    // Driven straight off the committed file, with no rewriting at all: this is
    // the end-to-end proof that the fixture is the shape the driver reads.
    let args = routed_args(RUN_ID, Path::new(REJECTED_FIXTURE_PATH), "1");
    drive(args, &config)
        .await
        .expect("a run that parks on a quota is an ordinary end");

    let records = journal_records(root.path(), RUN_ID);
    let (parked, detail) = park_and_detail(&records);

    assert_eq!(parked["reason"], rate_limit::REASON_QUOTA_REJECTED);
    assert!(
        detail.contains(rate_limit::WINDOW_SEVEN_DAY),
        "the park names the seven-day window the fixture's own rateLimitType \
         field carries: {detail}"
    );
    // **The reset half, which nothing asserted on and which is WR-03.** This run
    // is driven with a real `now`, and the committed fixture's `resetsAt` is a
    // fixed instant in 2026 that recedes further into the past with every day
    // that passes. Under the old symmetric bound it was accepted for thirty days
    // and rendered into the detail as when the quota resets — telling a human to
    // come back at a time that had already been and gone. A reset behind `now` by
    // more than clock skew is not a reset time, and the honest report is that it
    // is unknown.
    assert!(
        detail.contains(&format!("resets_at={}", rate_limit::UNKNOWN)),
        "the fixture's reset instant is in the past relative to a real clock, so \
         the bound must refuse it and the detail must SAY unknown rather than \
         render a past instant as the moment the window reopens. Got: {detail}"
    );
    assert_eq!(
        of_kind(&records, "exec_started").len(),
        1,
        "one command ran and no second was spawned, under a step cap of five"
    );
    assert_eq!(
        run_record(root.path(), RUN_ID)["outcome"],
        format!("parked:{}", rate_limit::REASON_QUOTA_REJECTED),
        "and a separate process reads the reason off run.json"
    );
}

// ---------------------------------------------------------------------------
// The negative half: a healthy run is untouched
// ---------------------------------------------------------------------------

#[tokio::test]
async fn an_allowed_quota_event_parks_nothing_and_the_run_completes_normally() {
    const RUN_ID: &str = "2026-08-19T12-00-00Z-allowed";

    let root = project_root();
    let config = config_for(root.path());
    let scratch = TempDir::new().expect("temp dir");
    // The committed baseline UNCHANGED — it already carries `status: "allowed"`
    // on a completely successful run.
    let transcript = transcript_file(&scratch, "allowed.ndjson", CLEAN_BASELINE);

    let mut args = routed_args(RUN_ID, &transcript, "0");
    args.command = Some("/gsd-progress".to_string());
    args.target_phase = None;

    drive(args, &config)
        .await
        .expect("a healthy run completes");

    let records = journal_records(root.path(), RUN_ID);
    assert!(
        of_kind(&records, "parked").is_empty(),
        "a rule keyed on the PRESENCE of a rate_limit_event would park this run \
         — and every other healthy one, since seven of the eight committed \
         captures carry the event. Got: {records:#?}"
    );
    assert_eq!(
        run_record(root.path(), RUN_ID)["outcome"],
        "succeeded_no_changes",
        "and the terminal label is still `outcome_label`'s, with no `parked:` \
         prefix — the exact string this transcript produced before the quota \
         detector existed"
    );
}
