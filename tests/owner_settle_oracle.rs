//! The settle flow against gsd-core's own readers (39 D-13, OWN-31, SC5).
//!
//! The product never settles a check itself: `x` launches `/gsd-verify-work N
//! <instruction>` and gsd-core records the outcome. What gsd-core decides
//! deterministically is checked here, with the **product's own** `settle_target_for`,
//! `prompt` and `DEFERRED_PREFIX`, through the real `gsd-tools` and `bash`:
//!
//! 1. `query init.verify-work` finds the phase for the argument every outcome builds;
//! 2. verify-work's own argument parse (the two pipelines pinned below, asserted
//!    verbatim against the installed `verify-work.md`) gives no `xargs` warning and
//!    resolves the workstream to the group's own, even for hostile check text;
//! 3. a UAT row recorded in the encoding the instruction asks for (`result: skipped`,
//!    `reason: Deferred follow-up: ...`) is `deferred` for `audit-uat`, passing for
//!    `phase uat-passed --uat-only`, and gone from the product's own digest.
//!
//! What the model does with the instruction (does it follow it, does it file the
//! todo) is human UAT, not tested here.
//!
//! **Temp copies only.** Test 3 rewrites a UAT file the way GSD would; every run
//! operates on a copy under a `TempDir`, never on the repository tree or on a
//! registered project.
//!
//! **The oracle is required** (the `owner_acknowledge_roundtrip` and
//! `owner_open_conformance` policy): with no gsd-core install the tests fail, unless
//! `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` holds a non-blank value.
//! `scripts/pre-tag-check.sh --container` provisions the oracle.

use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Utc};
use gsd_meta_manager::state_reader::gsd_install::{install_candidates, InstallRoots};
use gsd_meta_manager::state_reader::owner_items::{
    read_owner_digest, Kind, Lane, OwnerDigest, OwnerGroup, Scope,
};
use gsd_meta_manager::state_reader::{parse_project_state, ProjectState};
use gsd_meta_manager::ui::screens::owner_actions::ACK_ENV_REMOVE;
use gsd_meta_manager::ui::screens::owner_block::GroupKey;
use gsd_meta_manager::ui::screens::owner_settle::{
    clean, prompt, settle_target_for, MoveTarget, Outcome, SettleChoice, DEFERRED_PREFIX,
    REASON_MAX_CHARS,
};
use tempfile::TempDir;

const ALLOW_MISSING_ORACLE: &str = "GSD_META_MANAGER_ALLOW_MISSING_ORACLE";

/// verify-work's workstream parse, verbatim from `workflows/verify-work.md`
/// (gsd-core 1.23.0, step `initialize`). A sync that rewords it fails
/// [`the_installed_verify_work_still_parses_arguments_this_way`] loudly.
const WS_PIPELINE: &str = r#"GSD_WS=$(echo " $GSD_ARGS" | sed -nE 's/.* --ws +["'\'']?([A-Za-z0-9][A-Za-z0-9._-]*).*/--ws \1/p' | head -1)"#;

/// verify-work's phase-argument parse, verbatim from the same step.
const PHASE_ARG_PIPELINE: &str = r#"PHASE_ARG=$(echo "$GSD_ARGS" | sed -E 's/--ws[[:space:]]+("[^"]*"|'\''[^'\'']*'\''|[A-Za-z0-9._-]+)//g' | xargs)"#;

fn now() -> DateTime<Utc> {
    "2026-11-14T00:00:00Z".parse().unwrap()
}

/// A resolved way to run gsd-core's own `gsd-tools`.
struct Oracle {
    program: PathBuf,
    script: Option<PathBuf>,
    /// The installed `workflows/verify-work.md` of a `node` install (a bare
    /// `gsd-tools` on `PATH` has no tree to read it from).
    workflow: Option<PathBuf>,
}

impl Oracle {
    /// The installed `gsd-tools`, or `None` when this environment has none and says
    /// so through [`ALLOW_MISSING_ORACLE`]; a missing oracle otherwise fails the
    /// test (the default is fail-loud).
    fn resolve() -> Option<Self> {
        let found = Self::find();
        if found.is_none() {
            let opt_out = std::env::var(ALLOW_MISSING_ORACLE).unwrap_or_default();
            assert!(
                !opt_out.trim().is_empty(),
                "the gsd-core oracle is not installed, so the settle round trip cannot run; \
                 install it, or set {ALLOW_MISSING_ORACLE} to accept a run that checked nothing"
            );
            eprintln!("SKIP: no gsd-core oracle, and {ALLOW_MISSING_ORACLE} accepts that");
        }
        found
    }

    fn find() -> Option<Self> {
        let node_runs = Command::new("node").arg("--version").output().is_ok();
        for candidate in install_candidates(None, &InstallRoots::from_env()) {
            let script = candidate.gsd_core_dir.join("bin/gsd-tools.cjs");
            if script.is_file() && node_runs {
                let workflow = candidate.gsd_core_dir.join("workflows/verify-work.md");
                return Some(Oracle {
                    program: PathBuf::from("node"),
                    script: Some(script),
                    workflow: Some(workflow),
                });
            }
        }
        if Command::new("gsd-tools").arg("--version").output().is_ok() {
            return Some(Oracle {
                program: PathBuf::from("gsd-tools"),
                script: None,
                workflow: None,
            });
        }
        None
    }

    /// [`Self::run`], but the answer whatever the exit status: `phase uat-passed`
    /// exits non-zero while the phase is not passed, and prints its JSON anyway.
    fn run_any(&self, root: &Path, args: &[&str]) -> String {
        self.run(root, args).unwrap_or_else(|text| text)
    }

    /// `gsd-tools` in the project `root`, the way the product runs it: every
    /// selector variable removed.
    fn run(&self, root: &Path, args: &[&str]) -> Result<String, String> {
        let mut command = Command::new(&self.program);
        if let Some(script) = &self.script {
            command.arg(script);
        }
        command.args(args).current_dir(root);
        for name in ACK_ENV_REMOVE {
            command.env_remove(name);
        }
        let output = command
            .output()
            .map_err(|err| format!("the oracle did not run: {}", err.kind()))?;
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            Ok(text)
        } else {
            Err(format!("{text}{}", String::from_utf8_lossy(&output.stderr)))
        }
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A fixture copied into a TempDir.
fn temp_project(name: &str) -> (TempDir, PathBuf) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/owner")
        .join(name)
        .join(".planning");
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    copy_dir(&src, &root.join(".planning"));
    (dir, root)
}

fn read(root: &Path) -> (ProjectState, OwnerDigest) {
    let planning = root.join(".planning");
    let state = parse_project_state(&planning);
    let digest = read_owner_digest(&planning, &state, now(), 30);
    (state, digest)
}

/// The first current-milestone TEST group of `phase` in `workstream`.
fn test_group<'a>(
    digest: &'a OwnerDigest,
    workstream: Option<&str>,
    phase: &str,
) -> &'a OwnerGroup {
    digest
        .groups
        .iter()
        .find(|group| {
            group.kind == Kind::Uat
                && group.lane == Lane::Test
                && !group.acknowledged
                && group.scope == Scope::Current
                && group.workstream.as_deref() == workstream
                && group.phase == phase
        })
        .unwrap_or_else(|| panic!("the fixture has a TEST group for {workstream:?} phase {phase}"))
}

fn choices() -> Vec<SettleChoice> {
    let choice = |outcome, move_to| SettleChoice {
        outcome,
        reason: "no device here".to_owned(),
        move_to,
    };
    vec![
        choice(Outcome::WontDo, None),
        choice(Outcome::DeferPermanently, None),
        choice(
            Outcome::Move,
            Some(MoveTarget {
                number: "02".to_owned(),
                name: Some("sync".to_owned()),
                known: true,
            }),
        ),
    ]
}

/// One argument verify-work would receive: the target, the outcome's prompt
/// without the leading `/gsd-verify-work `, the phase and the workstream.
struct Case {
    label: String,
    args: String,
    phase: String,
    workstream: Option<String>,
}

/// Every outcome for the phase target and for the group's first check, built with
/// the product's own resolver and prompt.
fn cases(root: &Path, workstream: Option<&str>, phase: &str) -> Vec<Case> {
    let (state, digest) = read(root);
    let group = test_group(&digest, workstream, phase);
    let key = GroupKey::of(group);
    let first_item = group.items[0].id.clone();
    let mut out = Vec::new();
    for item in [None, Some(first_item.as_str())] {
        let target = settle_target_for("demo", &state, &digest, &key, item, 30, false)
            .unwrap_or_else(|why| panic!("{workstream:?} {phase} {item:?}: {why}"));
        for choice in choices() {
            let full = prompt(&target, &choice);
            let args = full
                .strip_prefix("/gsd-verify-work ")
                .unwrap_or_else(|| panic!("the prompt starts with the slash command: {full}"))
                .to_owned();
            out.push(Case {
                label: format!(
                    "{} / {} / {}",
                    workstream.unwrap_or("root"),
                    target.short(),
                    choice.outcome.label()
                ),
                phase: target.phase().to_owned(),
                workstream: target.command.workstream.clone(),
                args,
            });
        }
    }
    out
}

/// verify-work's own argument parse, run through `bash` exactly as the workflow
/// spells it: `(workstream word, phase argument, stderr)`.
fn parse_like_verify_work(args: &str) -> (String, String, String) {
    let script = format!(
        "GSD_WS=\"\"\n{WS_PIPELINE}\n{PHASE_ARG_PIPELINE}\n\
         printf '%s\\n' \"${{GSD_WS##* }}\"\nprintf '%s' \"$PHASE_ARG\""
    );
    let output = Command::new("bash")
        .arg("-c")
        .arg(script)
        .env("GSD_ARGS", args)
        .output()
        .expect("bash runs");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let (ws, phase_arg) = stdout.split_once('\n').unwrap_or((&stdout, ""));
    (
        ws.to_owned(),
        phase_arg.to_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn init_verify_work_finds_the_phase_for_every_outcome() {
    let Some(oracle) = Oracle::resolve() else {
        return;
    };
    for (fixture, workstream, phase) in [
        ("heuristic-labels", None, "01"),
        ("workstreams", Some("alpha"), "03"),
        ("workstreams", None, "01"),
    ] {
        let (_guard, root) = temp_project(fixture);
        let cases = cases(&root, workstream, phase);
        assert_eq!(cases.len(), 6, "three outcomes for a phase and a check");
        for case in cases {
            // The argument verify-work passes on, resolved the way it resolves it.
            let (ws, phase_arg, _) = parse_like_verify_work(&case.args);
            assert_eq!(
                Some(ws.as_str()).filter(|word| !word.is_empty()),
                case.workstream.as_deref(),
                "{}: verify-work resolves the command's own workstream",
                case.label
            );
            let ws_flag = format!("--ws={ws}");
            let mut args = vec!["query", "init.verify-work", phase_arg.as_str()];
            if !ws.is_empty() {
                args.push(&ws_flag);
            }
            let out = oracle
                .run(&root, &args)
                .unwrap_or_else(|why| panic!("{}: init.verify-work refused: {why}", case.label));
            assert!(
                out.contains("\"phase_found\": true"),
                "{}: the phase is not found for {:?}: {out}",
                case.label,
                case.args
            );
            let found: serde_json::Value =
                serde_json::from_str(&out).expect("init.verify-work prints JSON");
            let number = found["phase_number"].as_str().unwrap_or_default();
            // `1` and `01` name the same phase for gsd-core.
            assert_eq!(
                number.trim_start_matches('0'),
                case.phase.trim_start_matches('0'),
                "{}: another phase answered: {out}",
                case.label
            );
        }
    }
}

#[test]
fn the_installed_verify_work_still_parses_arguments_this_way() {
    let Some(oracle) = Oracle::resolve() else {
        return;
    };
    let Some(workflow) = &oracle.workflow else {
        // A gsd-tools on PATH without its install tree: the pin cannot be compared.
        eprintln!("SKIP: no verify-work.md next to the oracle");
        return;
    };
    // A gsd-core install carries its workflows; a missing one is a broken oracle.
    let text = std::fs::read_to_string(workflow)
        .unwrap_or_else(|err| panic!("cannot read {}: {err}", workflow.display()));
    for pinned in [WS_PIPELINE, PHASE_ARG_PIPELINE] {
        assert!(
            text.contains(pinned),
            "the installed {} no longer holds the pinned argument parse; re-check the \
             prompt's encoding (D-08, RESEARCH Pitfalls 2 and 3), then update the pin:\n{pinned}",
            workflow.display()
        );
    }
}

#[test]
fn the_argument_parse_takes_the_prompt_without_warnings_or_a_foreign_workstream() {
    let Some(_oracle) = Oracle::resolve() else {
        return;
    };
    for (fixture, workstream, phase) in [
        ("heuristic-labels", None, "01"),
        ("workstreams", Some("alpha"), "03"),
    ] {
        let (_guard, root) = temp_project(fixture);
        for case in cases(&root, workstream, phase) {
            let (ws, phase_arg, stderr) = parse_like_verify_work(&case.args);
            assert!(
                stderr.is_empty(),
                "{}: the parse warned: {stderr}",
                case.label
            );
            assert_eq!(
                ws,
                workstream.unwrap_or_default(),
                "{}: the workstream is the group's own",
                case.label
            );
            assert!(
                phase_arg.starts_with(&case.phase),
                "{}: the phase leads the argument: {phase_arg}",
                case.label
            );
        }
    }

    // Hostile text: a name and a reason that spell a flag and every quote shape.
    let (_guard, root) = temp_project("heuristic-labels");
    let uat = std::fs::read_dir(root.join(".planning/phases/01-pricing"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.to_string_lossy().ends_with("01-UAT.md"))
        .expect("the fixture has a UAT file");
    let text = std::fs::read_to_string(&uat).unwrap();
    let hostile = "x --ws evil won't a \"b\" \\ end";
    std::fs::write(
        &uat,
        text.replace("Approve the new icon", hostile)
            .replace("Review the export screen", hostile),
    )
    .unwrap();
    let (state, digest) = read(&root);
    let group = test_group(&digest, None, "01");
    let key = GroupKey::of(group);
    for item in group
        .items
        .iter()
        .map(|item| item.id.as_str())
        .map(Some)
        .chain([None])
    {
        let target = settle_target_for("demo", &state, &digest, &key, item, 30, false).unwrap();
        for mut choice in choices() {
            // The product cleans the reason before it reaches a plan.
            choice.reason = clean(hostile, REASON_MAX_CHARS);
            let full = prompt(&target, &choice);
            let args = full.strip_prefix("/gsd-verify-work ").unwrap();
            let (ws, phase_arg, stderr) = parse_like_verify_work(args);
            assert!(stderr.is_empty(), "{full}: the parse warned: {stderr}");
            assert_eq!(ws, "", "{full}: free text must not name a workstream");
            assert!(phase_arg.starts_with("01 "), "{full}: {phase_arg}");
        }
    }
}

#[test]
fn the_requested_encoding_is_a_non_blocking_deferral() {
    let Some(oracle) = Oracle::resolve() else {
        return;
    };
    let (_guard, root) = temp_project("uat-pending");
    let uat = root.join(".planning/phases/01-alpha/01-UAT.md");

    // The test stands in for GSD: it writes the encoding the instruction asks for.
    let before = std::fs::read_to_string(&uat).unwrap();
    let pending =
        "### 2. Saving a note\nexpected: a saved note appears in the list\nresult: pending";
    assert!(before.contains(pending), "the fixture row changed");
    let reason = format!("{DEFERRED_PREFIX} will not do. no device here");
    std::fs::write(
        &uat,
        before.replace(
            pending,
            &format!(
                "### 2. Saving a note\nexpected: a saved note appears in the list\nresult: skipped\nreason: {reason}"
            ),
        ),
    )
    .unwrap();

    let audit = oracle
        .run(&root, &["query", "audit-uat", "--raw"])
        .expect("audit-uat runs");
    let audit: serde_json::Value = serde_json::from_str(&audit).expect("audit-uat prints JSON");
    let item = audit["results"][0]["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["test"] == 2)
        .expect("test 2 is still listed by audit-uat");
    assert_eq!(item["category"], "deferred", "{item}");
    assert_eq!(item["reason"], reason.as_str());

    let passed = oracle.run_any(&root, &["phase", "uat-passed", "1", "--uat-only"]);
    let passed: serde_json::Value = serde_json::from_str(&passed).expect("uat-passed prints JSON");
    let check = passed["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["test"] == 2)
        .expect("test 2 is listed by uat-passed");
    assert_eq!(check["passing"], true, "{check}");
    assert_eq!(check["deferred"], true, "{check}");

    // The product's own reader agrees: only test 3 is left in the TEST lane.
    let (_, digest) = read(&root);
    let test_ids: Vec<&str> = digest
        .groups
        .iter()
        .filter(|group| group.lane == Lane::Test && group.phase == "01")
        .flat_map(|group| group.items.iter().map(|item| item.id.as_str()))
        .collect();
    assert_eq!(test_ids, ["01-UAT.md#3"], "test 2 left the TEST lane");
}
