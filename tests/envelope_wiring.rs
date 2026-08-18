// ============================================================================
// The envelope, attached to the spawn seam — and every refusal proved from
// disk, an exit code or a git ref (D-24, D-25, SAFE-01).
//
// **THE RULE THIS FILE IS WRITTEN UNDER: no assertion may read a model's summary.**
// Every fact below is an exit code, a file on disk, a git ref, or a journal event
// read back out of `journal.jsonl`. Not one is a sentence somebody wrote about
// the run.
//
// That is not stylistic fussiness. The incident this whole phase is built on had
// **two** failures, and in the second the agent reported success over a deletion
// it had performed. A verification suite that asked the same agent whether the
// envelope held would have reproduced the second failure while testing for the
// first. So the only prose in this file is in comments — and the negative grep in
// the plan's acceptance criteria is what keeps it there: every match of
// `(agent|model).{0,40}(summary|said|reported)` in this file is inside these two
// paragraphs, and none is inside an `assert`.
//
// `#![cfg(unix)]` because the hook stubs are `#!/bin/sh` with mode 0755 and the
// driver's detachment is a Unix facility. The policy under test is portable and
// is unit-tested in `src/envelope/`; only the delivery proved here is
// Unix-shaped.
// ============================================================================
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use gsd_meta_manager::config::{Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::{drive, DriveArgs};
use gsd_meta_manager::envelope::{cred, hooks, policy, ENVELOPE_ROOT_ENV};
use gsd_meta_manager::error::DriveError;
use gsd_meta_manager::journal::{self, reader, JournalEvent, JournalRun, RunPaths, RunRecord};
use tempfile::TempDir;

/// The binary under test, resolved by cargo for this integration target.
const BIN: &str = env!("CARGO_BIN_EXE_gsd-meta-manager");

/// The run id every fixture starts under.
const RUN_ID: &str = "wiringrun";

/// A credential shape planted by these fixtures.
///
/// A PEM block rather than a one-line token, and the shape is borrowed from
/// `tests/envelope_hook_refusals.rs` rather than forked: a PEM block is the case
/// a line-by-line scanner silently misses, because `BEGIN` and `END` are never
/// on the same line.
const PLANTED_PEM: &str = "-----BEGIN RSA PRIVATE KEY-----\n\
                           MIIBOgIBAAJBAKj34GkxFhD9abcdefgh\n\
                           ijklmnopqrstuvwxyz0123456789ABCD\n\
                           -----END RSA PRIVATE KEY-----\n";

// ---------------------------------------------------------------------------
// The two shared helpers. Both are used by this plan's drive-time assertions
// and by its four re-entry assertions, rather than being re-derived per test.
// ---------------------------------------------------------------------------

/// Every `Parked` event in the journal **at `journal_path` on disk**, as
/// `(reason, needs)` pairs.
///
/// **Reading the file rather than an in-memory handle is the entire point.** The
/// hook and guard re-entries this file exercises are *other processes* — git
/// spawns one, the agent's own CLI spawns another — and the only thing this
/// process shares with them is the filesystem. A helper that inspected a
/// `JournalRun` this process was holding would prove that this process can write
/// what it just wrote.
///
/// `reader::read_all` is the shipped reader, tolerant parse and all, so a record
/// of a kind this build did not model still arrives rather than being dropped.
fn parked_events(journal_path: &Path) -> Vec<(String, String)> {
    let (records, _diagnostics) = reader::read_all(journal_path).expect("the journal is readable");
    records
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

/// A real run — run directory, `run.json`, `active` pointer and journal — plus
/// the envelope environment a child of that run would carry.
///
/// Built through the production `JournalRun::start` path rather than by writing
/// the files here, because the thing under test is that a *separate process*
/// can find this run through the same `active` pointer the driver writes.
struct RunFixture {
    /// Held for its `Drop`; every path below lives inside it.
    _tmp: TempDir,
    /// The driven project root, which is also a real git repository.
    project: PathBuf,
    /// The `file://` bare remote, so a push has somewhere to go and a ref that
    /// can be compared before and after.
    bare: PathBuf,
    /// The envelope root, redirected away from the developer's real data dir.
    envelope_root: PathBuf,
    /// The installed hook stubs.
    hooks_dir: PathBuf,
    /// The run's six paths.
    paths: RunPaths,
    /// The alias this run was started under.
    alias: String,
    /// The run this fixture started. Held open so the journal handle exists;
    /// dropping it does not clear the `active` pointer, which is what makes a
    /// re-entry able to resolve the run.
    _run: JournalRun,
}

impl RunFixture {
    /// A ref inside the namespace this alias may push to.
    fn inside_ref(&self) -> String {
        format!("refs/heads/gsd-auto/{}/wiring", self.alias)
    }

    /// The environment a child of this run carries.
    ///
    /// The three variables that matter to a re-entry: the hooks path (so git
    /// runs the envelope's stubs), the envelope root (so the re-entered binary
    /// agrees which directory is sanctioned), and **the run-journal locator**,
    /// which is the whole subject of these tests. `RUN_ID_ENV` rides along
    /// because the guard's per-run cap reads it.
    fn child_env(&self) -> Vec<(std::ffi::OsString, std::ffi::OsString)> {
        let mut env: Vec<(std::ffi::OsString, std::ffi::OsString)> =
            cred::hooks_path_env(&self.hooks_dir);
        env.push((
            std::ffi::OsString::from(ENVELOPE_ROOT_ENV),
            self.envelope_root.clone().into_os_string(),
        ));
        env.push((
            std::ffi::OsString::from(cred::PROJECT_ROOT_ENV),
            self.project.clone().into_os_string(),
        ));
        env.push((
            std::ffi::OsString::from(cred::RUN_ID_ENV),
            std::ffi::OsString::from(RUN_ID),
        ));
        env
    }

    /// The refs the bare remote actually holds, read from the remote itself.
    ///
    /// The remote's own answer, not the local repository's idea of it: a push
    /// that was refused locally and a push that succeeded look identical from
    /// the pushing side if you ask the wrong repository.
    fn remote_refs(&self) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(&self.bare)
            .args(["for-each-ref", "--format=%(refname)"])
            .output()
            .expect("git for-each-ref is runnable");
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// Ask the guard about one shell command, **as a separate process** —
    /// which is the point: the journal it appends to is one it did not open.
    fn ask_guard(&self, command: &str, with_locator: bool) -> Output {
        let request = serde_json::json!({
            "session_id": "wiring",
            "hook_event_name": "PreToolUse",
            "tool_name": "Bash",
            "tool_input": { "command": command },
        })
        .to_string();

        let mut cmd = Command::new(BIN);
        cmd.args(["envelope", "guard", &self.alias]);
        for (key, value) in self.child_env() {
            if !with_locator && key == std::ffi::OsStr::new(cred::PROJECT_ROOT_ENV) {
                continue;
            }
            cmd.env(key, value);
        }
        if !with_locator {
            cmd.env_remove(cred::PROJECT_ROOT_ENV);
        }

        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("the binary under test is spawnable");
        {
            use std::io::Write as _;
            child
                .stdin
                .as_mut()
                .expect("the guard's stdin")
                .write_all(request.as_bytes())
                .expect("the request is writable");
        }
        child.wait_with_output().expect("the guard answers")
    }

    /// `git push` under the envelope's environment, and nothing else changed.
    fn push(&self, args: &[&str]) -> Output {
        let mut cmd = Command::new("git");
        cmd.arg("-C").arg(&self.project).args(args);
        for (key, value) in self.child_env() {
            cmd.env(key, value);
        }
        cmd.output().expect("git push is runnable")
    }
}

fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the parent directory");
    }
    std::fs::write(path, contents).expect("the planted file");
}

/// A run record with the fixed fields these tests do not care about.
fn run_record(run_id: &str) -> RunRecord {
    RunRecord {
        run_id: run_id.to_string(),
        goal: "prove the envelope parks the run".to_string(),
        gsd_command: "/gsd-progress".to_string(),
        target: "host".to_string(),
        opt_in: Some("2026-08-18T00:00:00Z".to_string()),
        started_at: "2026-08-18T00:00:00Z".to_string(),
        session_id: "00000000-0000-0000-0000-000000000000".to_string(),
        pid: std::process::id(),
        pgid: std::process::id(),
        claude_code_version: "2.1.214".to_string(),
        argv_digest: "sha256:0".to_string(),
        ended_at: None,
        outcome: None,
    }
}

/// Start a real run under a real repository with a real envelope, or `None`
/// when the sandbox forbids `git init` — so these tests skip gracefully rather
/// than failing for a reason that is not about the code.
fn start_run_with_envelope(alias: &str) -> Option<RunFixture> {
    let tmp = TempDir::new().ok()?;
    let project = tmp.path().join("work");
    let bare = tmp.path().join("remote.git");
    let envelope_root = tmp.path().join("envelope");
    std::fs::create_dir_all(&project).ok()?;
    std::fs::create_dir_all(&bare).ok()?;

    if !git(&bare, &["init", "--bare", "--quiet"]) {
        return None;
    }
    if !git(&project, &["init", "--quiet"]) {
        return None;
    }
    git(&project, &["config", "user.email", "test@example.com"]);
    git(&project, &["config", "user.name", "Test User"]);
    git(&project, &["config", "commit.gpgsign", "false"]);
    write(&project, "tracked.txt", "one\n");
    if !git(&project, &["add", "tracked.txt"]) {
        return None;
    }
    if !git(&project, &["commit", "-m", "initial", "--quiet"]) {
        return None;
    }
    let url = format!("file://{}", bare.display());
    if !git(&project, &["remote", "add", "origin", &url]) {
        return None;
    }

    let hooks_dir = hooks::install_in(&envelope_root, alias, Path::new(BIN))
        .expect("installing hook stubs for a plain alias succeeds");

    // Through `JournalRun::start`, never by writing the files here: the `active`
    // pointer this writes is the ONLY thing a re-entering process has to find
    // the run, so a fixture that wrote it by hand would be testing its own
    // spelling of a path rather than the production one.
    let planning = project.join(".planning");
    std::fs::create_dir_all(&planning).ok()?;
    let run = JournalRun::start(&planning, run_record(RUN_ID)).expect("the run starts");
    let paths = run.paths().clone();

    Some(RunFixture {
        _tmp: tmp,
        project,
        bare,
        envelope_root,
        hooks_dir,
        paths,
        alias: alias.to_string(),
        _run: run,
    })
}

/// A registry file naming one opted-in project.
fn registry_at(dir: &Path, alias: &str, project: &Path) -> PathBuf {
    let mut config = Config::new();
    config.projects.insert(
        alias.to_string(),
        RegisteredProject {
            path: project.to_path_buf(),
            added: "2026-08-18T00:00:00Z".to_string(),
            driver_opt_in: Some(DriverOptIn {
                opted_in_at: "2026-08-18T00:00:00Z".to_string(),
                claude_md_digest: None,
                branch_namespace: None,
                credential: None,
                pr_cap_per_24h: None,
                pr_cap_per_run: None,
            }),
            extra: Default::default(),
        },
    );
    let path = dir.join("config.json");
    gsd_meta_manager::config::save_config(&config, &path).expect("the fixture registry is writable");
    path
}

/// Drive `alias` out of process, with `envelope_root` as the envelope's root.
fn drive_out_of_process(config: &Path, alias: &str, envelope_root: &Path, run_id: &str) -> Output {
    Command::new(BIN)
        .arg("--config")
        .arg(config)
        .args(["drive", alias, "--command", "/gsd-progress", "--run-id"])
        .arg(run_id)
        .env(ENVELOPE_ROOT_ENV, envelope_root)
        .output()
        .expect("the binary under test is runnable")
}

// ---------------------------------------------------------------------------
// The drive-time refusal: an envelope that cannot be established refuses the
// run BEFORE anything is created (D-24).
// ---------------------------------------------------------------------------

#[test]
fn an_envelope_that_cannot_be_established_refuses_before_anything_is_created() {
    let tmp = TempDir::new().expect("temp dir");
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join(".planning")).expect("scratch .planning");
    let config = registry_at(tmp.path(), "wired", &project);

    // A **file** where the envelope root should be a directory. Every generated
    // artifact hangs off this path, so nothing under it can be created — which
    // is an establishment failure rather than a policy decision, and is exactly
    // the case that must refuse instead of proceeding with a partial envelope.
    let envelope_root = tmp.path().join("envelope-is-a-file");
    std::fs::write(&envelope_root, b"not a directory").expect("the blocking file");

    let out = drive_out_of_process(&config, "wired", &envelope_root, "refused1");
    let stderr = String::from_utf8_lossy(&out.stderr);

    // Evidence 1: the exit code. Non-zero, and it is the carrier that does not
    // depend on anybody parsing a message.
    assert!(
        !out.status.success(),
        "a run whose envelope could not be established exited zero; stderr was:\n{stderr}"
    );

    // Evidence 2: the park reason, on stderr, spelled as the D-24 identifier
    // rather than as a fresh sentence.
    assert!(
        stderr.contains(policy::REASON_ENVELOPE_ASSERTION_FAILED),
        "the refusal must name its park reason; stderr was:\n{stderr}"
    );

    // Evidence 3: nothing on disk. No run directory, no run record, no lock
    // file, no journal — the position of the refusal is what makes this true,
    // and this assertion is what keeps the position.
    let runs = journal::runs_root(&project.join(".planning"));
    assert!(
        !runs.exists(),
        "a refused run created {} — the envelope refusal must sit before the \
         lock, the run directory and the journal",
        runs.display()
    );
}

#[test]
fn an_alias_that_can_have_no_envelope_is_refused_by_the_ordered_chain() {
    // The chain's own position, exercised in process so the TYPED error is
    // visible rather than only its rendering. A hostile alias can have no
    // envelope directory at all (D-03), so `drive` refuses it after the
    // capability gate and before dispatch.
    let tmp = TempDir::new().expect("temp dir");
    let project = tmp.path().join("project");
    std::fs::create_dir_all(project.join(".planning")).expect("scratch .planning");

    let mut config = Config::new();
    config.projects.insert(
        "../escaped".to_string(),
        RegisteredProject {
            path: project.clone(),
            added: "2026-08-18T00:00:00Z".to_string(),
            driver_opt_in: Some(DriverOptIn {
                opted_in_at: "2026-08-18T00:00:00Z".to_string(),
                claude_md_digest: None,
                branch_namespace: None,
                credential: None,
                pr_cap_per_24h: None,
                pr_cap_per_run: None,
            }),
            extra: Default::default(),
        },
    );

    let args = DriveArgs {
        alias: "../escaped".to_string(),
        command: "/gsd-progress".to_string(),
        run_id: Some("hostile1".to_string()),
        dry_run: false,
        goal: None,
        #[cfg(debug_assertions)]
        claude_program: None,
        #[cfg(debug_assertions)]
        claude_args: Vec::new(),
    };

    let err = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("a runtime")
        .block_on(drive(args, &config))
        .expect_err("an alias that can have no envelope must be refused");

    match &err {
        DriveError::EnvelopeAssertionFailed { reason, .. } => {
            assert_eq!(
                reason.as_str(),
                policy::REASON_ENVELOPE_ASSERTION_FAILED,
                "the refusal must carry D-24's identifier, not a fresh string"
            );
        }
        other => panic!("expected an envelope-assertion refusal, got {other:?}"),
    }

    let runs = journal::runs_root(&project.join(".planning"));
    assert!(
        !runs.exists(),
        "a refused run created {}; the chain's refusal must precede dispatch",
        runs.display()
    );
}

#[test]
fn the_two_helpers_round_trip_a_park_reason_through_the_file() {
    // The helpers themselves, proved before four other tests lean on them. A
    // `parked_events` that silently returned nothing would make every refusal
    // assertion in this file pass vacuously, which is the failure mode a
    // shared helper has that an inline assertion does not.
    let Some(mut fx) = start_run_with_envelope("helpers") else {
        return;
    };

    assert!(
        parked_events(&fx.paths.journal).is_empty(),
        "a fresh run carries no park; a helper that reported one would be \
         matching on the wrong field"
    );

    fx._run
        .record(&JournalEvent::Parked {
            reason: policy::ParkReason::ForcePushBlocked.as_str().to_string(),
            needs: "human".to_string(),
        })
        .expect("the journal takes the park");

    assert_eq!(
        parked_events(&fx.paths.journal),
        vec![(
            policy::REASON_FORCE_PUSH_BLOCKED.to_string(),
            "human".to_string()
        )],
        "the reason read back off disk must equal the one written"
    );

    // And the fixture really did produce a resolvable run, or every re-entry
    // assertion below would be proving that a hook cannot find a run that does
    // not exist.
    assert!(fx.paths.run_json.is_file(), "the run record must be on disk");
    assert_eq!(
        gsd_meta_manager::journal::writer::read_active_run(&fx.project.join(".planning")).as_deref(),
        Some("wiringrun"),
        "the active pointer is the only thing a re-entering process has"
    );
}

// ---------------------------------------------------------------------------
// Every refusal parks the run — four reasons, four processes other than this
// one, four journals read off disk (D-24, D-25, SAFE-02, SAFE-03, SAFE-06).
//
// Every test below asserts all four of D-25's evidence facts in ONE test rather
// than in four: a non-zero exit, the `Parked` event with the expected reason in
// the on-disk journal, the park reason in the run's terminal record, and — for
// the two push rows — the remote ref byte-identical to what it was before.
// Splitting them would let a refusal that exited non-zero and pushed anyway
// pass three tests out of four.
//
// The refusing process is never this one. git spawns the pre-push stub; the
// guard is spawned as `gsd-meta-manager envelope guard`. The only thing the
// refusing process shares with this one is the filesystem, which is exactly the
// property under test.
// ---------------------------------------------------------------------------

/// Seed the pull-request ledger to the default rolling-24h cap.
fn seed_ledger_at_cap(fx: &RunFixture) {
    let path = gsd_meta_manager::envelope::ledger::ledger_path_in(&fx.envelope_root, &fx.alias)
        .expect("a plain alias has a ledger");
    std::fs::create_dir_all(path.parent().unwrap()).expect("the ledger directory");

    // Distinct run ids, so the PER-RUN bound is not what fires: this row is
    // about the window, and a per-run refusal would satisfy the assertion for
    // the wrong reason.
    let mut body = String::new();
    for index in 0..policy::DEFAULT_PR_CAP_PER_24H {
        let at = (chrono::Utc::now() - chrono::Duration::seconds(60))
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        body.push_str(
            &serde_json::to_string(&gsd_meta_manager::envelope::ledger::LedgerEntry {
                at,
                run_id: format!("earlier-{index}"),
                command: "gh pr create".to_string(),
                platform: "github".to_string(),
            })
            .unwrap(),
        );
        body.push('\n');
    }
    std::fs::write(&path, body).expect("the seeded ledger");
}

/// The run's terminal record, closed the way the driver closes it, so the
/// `outcome` field can be read.
fn finish_and_read_outcome(fx: RunFixture) -> String {
    let run_json = fx.paths.run_json.clone();
    let mut run = fx._run;
    // The label the driver computes at the end of a run. `terminal_label` is
    // `pub(crate)`, so this reproduces its ONE decision — last park wins — from
    // the same on-disk journal the driver reads, and the in-source tests in
    // `src/driver/run.rs` hold the function itself.
    let parks = parked_events(&fx.paths.journal);
    let label = match parks.last() {
        Some((reason, _)) => format!("parked:{reason}"),
        None => "succeeded_no_changes".to_string(),
    };
    run.finish(&label).expect("the run finishes");

    let body = std::fs::read_to_string(&run_json).expect("the run record is on disk");
    let record: serde_json::Value = serde_json::from_str(&body).expect("it is JSON");
    record["outcome"].as_str().unwrap_or_default().to_string()
}

#[test]
fn a_force_push_is_refused_and_lands_a_force_push_blocked_park() {
    let Some(fx) = start_run_with_envelope("forcepush") else {
        return;
    };
    let before = fx.remote_refs();

    // Through the guard, because `--force` is visible in ARGV and not in the
    // ref lines git hands a pre-push hook: a force push inside the reserved
    // namespace is a perfectly ordinary push as far as the hook can see. This
    // is the layer that has the fact.
    let out = fx.ask_guard(
        &format!("git push --force origin HEAD:{}", fx.inside_ref()),
        true,
    );
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(
        out.status.code(),
        Some(2),
        "a force push must be denied with the hook protocol's blocking status; \
         stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains(policy::REASON_FORCE_PUSH_BLOCKED),
        "the refusal must name its park reason; stderr was:\n{stderr}"
    );
    assert_eq!(
        parked_events(&fx.paths.journal),
        vec![(
            policy::REASON_FORCE_PUSH_BLOCKED.to_string(),
            "human".to_string()
        )],
        "a separate process must have found this run's journal and parked it"
    );
    assert_eq!(
        fx.remote_refs(),
        before,
        "the remote changed; a refusal that still pushed is not a refusal"
    );
    assert_eq!(
        finish_and_read_outcome(fx),
        format!("parked:{}", policy::REASON_FORCE_PUSH_BLOCKED),
        "the terminal record must name the park reason"
    );
}

#[test]
fn a_hooks_path_rewrite_is_refused_and_lands_a_hook_bypass_blocked_park() {
    let Some(fx) = start_run_with_envelope("hookspath") else {
        return;
    };
    let before = fx.remote_refs();

    // The bypass the whole three-layer design exists for: rewriting
    // `core.hooksPath` makes layer 3 unreachable, so layer 2 has to be the one
    // that sees it — and it sees it in argv, before git ever runs.
    let out = fx.ask_guard(
        &format!(
            "git -c core.hooksPath=/tmp/nowhere push origin HEAD:{}",
            fx.inside_ref()
        ),
        true,
    );
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(
        out.status.code(),
        Some(2),
        "a hooks-path rewrite must be denied; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains(policy::REASON_HOOK_BYPASS_BLOCKED),
        "the refusal must name its park reason; stderr was:\n{stderr}"
    );
    assert_eq!(
        parked_events(&fx.paths.journal),
        vec![(
            policy::REASON_HOOK_BYPASS_BLOCKED.to_string(),
            "human".to_string()
        )],
    );
    assert_eq!(fx.remote_refs(), before, "the remote must be unchanged");
    assert_eq!(
        finish_and_read_outcome(fx),
        format!("parked:{}", policy::REASON_HOOK_BYPASS_BLOCKED)
    );
}

#[test]
fn a_worktree_carrying_a_credential_is_refused_and_lands_a_secret_detected_park() {
    let Some(fx) = start_run_with_envelope("secretpush") else {
        return;
    };
    let before = fx.remote_refs();

    // This one goes through a REAL `git push` and the generated `pre-push`
    // stub, because the full-worktree credential scan is that hook's, not the
    // guard's. git spawns the stub; the stub re-enters this binary; the binary
    // finds the run through the locator on the environment git handed down.
    write(&fx.project, ".gitignore", "secrets/\n");
    write(&fx.project, "secrets/prod.pem", PLANTED_PEM);

    let inside = fx.inside_ref();
    let out = fx.push(&["push", "origin", &format!("HEAD:{inside}")]);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "a push carrying a credential left the machine; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains(policy::REASON_SECRET_DETECTED),
        "the refusal must name its park reason; stderr was:\n{stderr}"
    );
    assert_eq!(
        parked_events(&fx.paths.journal),
        vec![(
            policy::REASON_SECRET_DETECTED.to_string(),
            "human".to_string()
        )],
    );
    assert!(
        !stderr.contains("MIIBOgIBAAJBAKj34GkxFhD9abcdefgh"),
        "the refusal reproduced the secret it blocked; stderr was:\n{stderr}"
    );
    assert_eq!(
        fx.remote_refs(),
        before,
        "the remote grew {inside} — the exit code refused but the write happened"
    );
    assert_eq!(
        finish_and_read_outcome(fx),
        format!("parked:{}", policy::REASON_SECRET_DETECTED)
    );
}

#[test]
fn a_pull_request_beyond_the_cap_is_refused_and_lands_a_pr_cap_exceeded_park() {
    let Some(fx) = start_run_with_envelope("prcap") else {
        return;
    };
    seed_ledger_at_cap(&fx);

    let out = fx.ask_guard("gh pr create --title t --body b", true);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(
        out.status.code(),
        Some(2),
        "the attempt past the cap must be denied; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains(policy::REASON_PR_CAP_EXCEEDED),
        "the refusal must name its park reason; stderr was:\n{stderr}"
    );
    assert_eq!(
        parked_events(&fx.paths.journal),
        vec![(
            policy::REASON_PR_CAP_EXCEEDED.to_string(),
            "human".to_string()
        )],
    );
    assert_eq!(
        finish_and_read_outcome(fx),
        format!("parked:{}", policy::REASON_PR_CAP_EXCEEDED)
    );
}

#[test]
fn a_permitted_command_is_permitted_and_parks_nothing() {
    // **The pairing.** Every assertion above is a refusal, and an envelope that
    // refused everything would satisfy all four of them while being useless. A
    // push inside the reserved namespace is permitted, writes no decision, and
    // leaves the journal without a park.
    let Some(fx) = start_run_with_envelope("permitted") else {
        return;
    };

    let out = fx.ask_guard(
        &format!("git push origin HEAD:{}", fx.inside_ref()),
        true,
    );

    assert_eq!(
        out.status.code(),
        Some(0),
        "a push inside the namespace must be permitted; stderr was:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stdout.is_empty(),
        "a permit writes nothing at all; answering `allow` would turn a deny-only \
         control into an approval authority"
    );
    assert!(
        parked_events(&fx.paths.journal).is_empty(),
        "a permitted command must park nothing"
    );
    assert_eq!(
        finish_and_read_outcome(fx),
        "succeeded_no_changes",
        "a run nothing refused keeps its ordinary terminal label"
    );
}

#[test]
fn a_re_entry_with_the_locator_absent_still_refuses_and_says_it_could_not_park() {
    // T-19-57, accepted and stated rather than papered over. An agent that
    // unsets the locator loses its own park record and **gains nothing**: the
    // verdict is already decided by the time the park is attempted, so the
    // refusal and the exit code are identical with and without it.
    let Some(fx) = start_run_with_envelope("nolocator") else {
        return;
    };

    let out = fx.ask_guard(
        &format!("git push --force origin HEAD:{}", fx.inside_ref()),
        false,
    );
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert_eq!(
        out.status.code(),
        Some(2),
        "the refusal must not depend on the park being recordable; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains(policy::REASON_FORCE_PUSH_BLOCKED),
        "the refusal itself is unchanged; stderr was:\n{stderr}"
    );
    assert!(
        stderr.contains("NOT recorded"),
        "silence is the one behaviour forbidden here: a refusal that could not be \
         recorded must SAY so, or it is indistinguishable from a refusal nobody \
         attempted to record; stderr was:\n{stderr}"
    );
    assert!(
        parked_events(&fx.paths.journal).is_empty(),
        "with no locator there is nothing to append to, and nothing was appended"
    );
}

// ---------------------------------------------------------------------------
// D-28's carried-in constraints, asserted rather than asserted-about.
//
// Each of these is a property an earlier phase paid for. A phase that touches
// four of the five files they name is exactly the phase that regresses one, so
// they are re-asserted here — except where an existing test already holds one,
// in which case it is referenced by comment. A second copy is a second thing
// that can drift.
// ---------------------------------------------------------------------------

#[test]
fn run_paths_still_returns_an_option_and_still_refuses_a_traversing_id() {
    // D-28: `journal::run_paths` returns `Option<RunPaths>`. The signature is
    // the whole of the WR-02 path-traversal fix, and it exists to conscript the
    // compiler into enumerating callers — this plan added one (`envelope::park`)
    // and it handles the `None`.
    let planning = Path::new("/tmp/does-not-need-to-exist/.planning");
    for hostile in ["../escaped", "a/b", "", ".", "..", "/etc"] {
        assert!(
            journal::run_paths(planning, hostile).is_none(),
            "{hostile:?} must not be joined into a run path"
        );
    }
    assert!(
        journal::run_paths(planning, "plainrun").is_some(),
        "a plain component must still resolve, or the guard refuses everything"
    );
}

#[test]
fn liveness_is_still_consumed_as_a_tri_state_rather_than_a_boolean() {
    // D-28: `ObservedRun.live` no longer exists; `Liveness` is a tri-state and
    // `Unknown` is never a synonym for `Dead`. The match is what holds it —
    // collapsing the enum to a boolean would fail to compile here.
    use gsd_meta_manager::driver::liveness::Liveness;

    let every = [Liveness::Alive, Liveness::Dead, Liveness::Unknown];
    let mut live = 0usize;
    for state in every {
        match state {
            Liveness::Alive => live += 1,
            Liveness::Dead => {}
            Liveness::Unknown => {}
        }
    }
    assert_eq!(
        live, 1,
        "exactly one of the three states means live; an `Unknown` counted as \
         live would signal a process this probe never identified, and an \
         `Unknown` counted as dead would report a healthy run as crashed"
    );
    assert_ne!(
        Liveness::Unknown,
        Liveness::Dead,
        "`Unknown` must never be a synonym for `Dead`"
    );
}

#[test]
fn the_configuration_path_is_still_first_on_the_drive_argv() {
    // D-28 / CR-03: `spawn::drive_argv` takes `config_path` first, and any new
    // argument this phase adds goes after the existing ones. This phase adds
    // none — the envelope's locator travels on the environment — and this
    // assertion is what says so mechanically.
    let argv = gsd_meta_manager::driver::spawn::drive_argv(
        Path::new("/cfg/config.json"),
        "alpha",
        "/gsd-progress",
        "run1",
        None,
    );
    assert_eq!(argv[0], std::ffi::OsString::from("--config"));
    assert_eq!(argv[1], std::ffi::OsString::from("/cfg/config.json"));
    assert_eq!(argv[2], std::ffi::OsString::from("drive"));
}

#[test]
fn the_permission_mode_still_has_no_bypass_and_the_target_match_is_still_exhaustive() {
    // D-28, both halves.
    //
    // The exhaustive `match options.target` in `build_argv` cannot be asserted
    // from outside the crate — a missing arm is a COMPILE error inside
    // `src/executor/claude.rs`, which is stronger than anything assertable here,
    // and the plan's acceptance criteria grep for the arm. What is assertable
    // from here is the observable consequence: the host argv carries no
    // permission bypass, with either envelope field set or unset.
    //
    // The variant-level proof lives in
    // `src/executor/claude.rs#the_permission_mode_enum_has_exactly_one_variant_and_it_is_not_a_bypass`,
    // which matches on the enum so a bypass variant fails to compile. It is
    // referenced rather than copied: a second copy is a second thing that can
    // drift, and the one that gets updated is not necessarily the one somebody
    // reads.
    use gsd_meta_manager::executor::{claude::build_argv, ExecutionOptions};

    let forbidden = [
        concat!("--dangerously", "-skip-permissions"),
        concat!("bypass", "Permissions"),
    ];
    for options in [
        ExecutionOptions::default(),
        ExecutionOptions {
            envelope_disallowed_tools: policy::disallowed_tools(),
            envelope_settings: Some(PathBuf::from("/envelope/alpha/settings.json")),
            ..Default::default()
        },
    ] {
        let argv: Vec<String> = build_argv(&options)
            .into_iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        for token in forbidden {
            assert!(
                !argv.iter().any(|arg| arg.contains(token)),
                "argv must never carry {token}: {argv:?}"
            );
        }
        assert!(
            argv.iter().any(|arg| arg == "dontAsk"),
            "the only permission mode a driven run uses must still be on argv: {argv:?}"
        );
    }
}

#[test]
fn the_envelope_deny_list_and_the_settings_file_carry_the_same_controls() {
    // D-07's rule, re-asserted at the wiring seam rather than only at the
    // generator: the settings file is never the SOLE carrier of a control, so a
    // file the CLI silently ignores leaves the argv layer standing.
    let envelope = TempDir::new().expect("temp dir");
    let settings =
        hooks::write_settings_in(envelope.path(), "wired", Path::new(BIN)).expect("settings");
    let value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings).expect("readable")).expect("json");

    let denied: Vec<String> = value["permissions"]["deny"]
        .as_array()
        .expect("the deny list is an array")
        .iter()
        .map(|entry| entry.as_str().unwrap().to_string())
        .collect();
    assert!(!denied.is_empty(), "an empty deny list carries nothing");

    // The argv rendering, exactly as `build_argv` will emit it.
    let options = gsd_meta_manager::executor::ExecutionOptions {
        envelope_disallowed_tools: policy::disallowed_tools(),
        envelope_settings: Some(settings.clone()),
        ..Default::default()
    };
    let argv: Vec<String> = gsd_meta_manager::executor::claude::build_argv(&options)
        .into_iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let position = argv
        .iter()
        .position(|arg| arg == "--disallowedTools")
        .expect("the deny list rides on argv");
    let rendered = &argv[position + 1];

    for pattern in &denied {
        assert!(
            rendered.contains(pattern),
            "`{pattern}` is registered in the settings file but is NOT on the \
             `--disallowedTools` value `{rendered}`"
        );
    }
    assert_eq!(
        argv.iter().position(|arg| arg == "--settings").map(|i| &argv[i + 1]),
        Some(&settings.display().to_string()),
        "the settings file must be named on argv, or it is never loaded at all"
    );
}
