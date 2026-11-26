// ============================================================================
// Layer A of the owner-items reader against gsd-core's own `audit-uat`
// (OWN-01, OWN-02, D-07, D-08, D-09).
//
// **What this file proves.** `owner_items::scan` is a port of gsd-core 1.21.0's
// `cmdAuditUat`: which files it reads, how it parses their rows, how it categorizes
// them. For every committed fixture under `tests/fixtures/owner/` this test runs the
// real oracle and our reader over a copy of the same tree and requires the sorted
// multiset of `(file, test, result, category)` to be equal, and the parse-gap and
// acknowledged-file counts to match. Layer B (the owner's own lanes, groups, ages and
// commands) is never compared here: those rules are ours and are unit-tested in the
// crate.
//
// **A missing oracle FAILS; skipping is opt-in.** The oracle is a Node script from
// the GSD install. On a machine or runner without it, a test that returned early
// would pass while nothing checked the port, so the default is to fail with the
// install hint, and an environment that genuinely cannot run Node says so through
// `GSD_META_MANAGER_ALLOW_MISSING_ORACLE`. When the oracle IS available every
// discovered fixture must produce a comparison or assert its declared divergence,
// and a run in which no fixture compared fails: a test that passes because it
// checked nothing is worse than no test.
//
// **The oracle runs at test time only.** `no_source_file_names_the_audit_oracle_verb`
// scans every non-comment line under `src/` for the verb, so the separation is a
// property of the build. Markdown files are outside that scan (D-09).
//
// Shape copied from `tests/driver_router_conformance.rs`, deliberately not
// abstracted into a shared helper.
// ============================================================================

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use gsd_meta_manager::state_reader::gsd_install::{install_candidates, InstallRoots};
use gsd_meta_manager::state_reader::owner_items::{scan, AuditFile, AuditKind, Unreadable};
use gsd_meta_manager::state_reader::parse_project_state;
use serde_json::Value;
use tempfile::TempDir;

/// The oracle verb: the one place it is spelled outside comments in this crate's
/// tests, and spelled nowhere under `src/`.
const ORACLE_VERB: &str = "audit-uat";

/// The variable an environment sets to say that it genuinely cannot run the oracle
/// and accepts a conformance run that checked nothing. Empty and whitespace-only
/// values are not an opt-out.
const ALLOW_MISSING_ORACLE: &str = "GSD_META_MANAGER_ALLOW_MISSING_ORACLE";

/// The sentinel `test` of an item with no test number, on both sides.
const NO_TEST: &str = "-";

/// The `oracle.expect` values a fixture may carry.
const EXPECTATIONS: [&str; 3] = ["same", "no-phases-dir", "out-of-set-verification"];

/// The VERIFICATION statuses upstream accepts.
const VERIFICATION_STATUSES: [&str; 3] = ["passed", "gaps_found", "human_needed"];

fn skip_permitted(opt_out: Option<&str>) -> bool {
    matches!(opt_out, Some(value) if !value.trim().is_empty())
}

/// Where the oracle looks for `gsd-tools.cjs`: the global arm of the production
/// install list (`${CLAUDE_CONFIG_DIR:-~/.claude}`, then `${CODEX_HOME:-~/.codex}`).
fn oracle_candidates(roots: &InstallRoots) -> Vec<PathBuf> {
    install_candidates(None, roots)
        .into_iter()
        .map(|candidate| candidate.gsd_core_dir.join("bin/gsd-tools.cjs"))
        .collect()
}

/// A resolved way to run gsd-core's own `gsd-tools`.
struct Oracle {
    program: PathBuf,
    script: Option<PathBuf>,
}

impl Oracle {
    fn resolve() -> Option<Self> {
        for script in oracle_candidates(&InstallRoots::from_env()) {
            if script.is_file() {
                if Command::new("node").arg("--version").output().is_ok() {
                    return Some(Oracle {
                        program: PathBuf::from("node"),
                        script: Some(script),
                    });
                }
                eprintln!(
                    "SKIP: found {} but `node` is not runnable, so the oracle cannot be executed",
                    script.display()
                );
                return None;
            }
        }
        if Command::new("gsd-tools").arg("--version").output().is_ok() {
            return Some(Oracle {
                program: PathBuf::from("gsd-tools"),
                script: None,
            });
        }
        eprintln!(
            "SKIP: neither ${{CLAUDE_CONFIG_DIR:-~/.claude}}/gsd-core/bin/gsd-tools.cjs, \
             ${{CODEX_HOME:-~/.codex}}/gsd-core/bin/gsd-tools.cjs nor a `gsd-tools` on PATH is \
             available, so the conformance oracle cannot run."
        );
        None
    }

    /// The `gsd-tools query <verb>` command, pinned to the Claude runtime like the
    /// router harness so a Codex marker cannot change the spelling.
    fn query_command(&self, verb: &str) -> Command {
        let mut command = Command::new(&self.program);
        if let Some(script) = &self.script {
            command.arg(script);
        }
        command.args(["query", verb]).env("GSD_RUNTIME", "claude");
        command
    }

    /// One oracle run over `root`: its `results` and `acknowledged_files`, or the
    /// combined output of a failed run.
    fn audit(&self, root: &Path, workstream: Option<&str>) -> Result<(Vec<Value>, u64), String> {
        let mut command = self.query_command(ORACLE_VERB);
        command
            .arg("--raw")
            .current_dir(root)
            .env_remove("GSD_WORKSTREAM")
            .env_remove("GSD_PROJECT");
        if let Some(workstream) = workstream {
            command.env("GSD_WORKSTREAM", workstream);
        }
        let output = command
            .output()
            .map_err(|err| format!("the oracle did not run: {}", err.kind()))?;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        if !output.status.success() {
            text.push_str(&String::from_utf8_lossy(&output.stderr));
            return Err(text);
        }
        // Output over 50,000 characters goes to a temp file named by `@file:<path>`.
        if let Some(path) = text.trim().strip_prefix("@file:") {
            let path = PathBuf::from(path);
            text = std::fs::read_to_string(&path)
                .map_err(|err| format!("the @file: output was unreadable: {err}"))?;
            let _ = std::fs::remove_file(&path);
        }
        let parsed: Value = serde_json::from_str(&text)
            .map_err(|err| format!("the oracle's output did not parse as JSON: {err}"))?;
        let results = parsed["results"].as_array().cloned().unwrap_or_default();
        let acknowledged = parsed["acknowledged_files"].as_u64().unwrap_or(0);
        Ok((results, acknowledged))
    }

    /// The root run plus one run per workstream, unioned and deduplicated by
    /// `file_path` (first wins). A workstream run keeps only the results under its
    /// own `phases/`.
    fn audit_all(&self, root: &Path) -> Result<(Vec<Value>, u64), String> {
        let (mut results, acknowledged) = self.audit(root, None)?;
        let workstreams = root.join(".planning/workstreams");
        let mut names: Vec<String> = std::fs::read_dir(&workstreams)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        names.sort();
        for name in names {
            let prefix = format!(".planning/workstreams/{name}/phases/");
            let (more, _) = self.audit(root, Some(&name))?;
            for result in more {
                let path = result["file_path"].as_str().unwrap_or_default();
                if path.starts_with(&prefix) {
                    results.push(result);
                }
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        results.retain(|result| seen.insert(result["file_path"].as_str().unwrap_or("").to_owned()));
        Ok((results, acknowledged))
    }
}

type Tuple = (String, String, String, String);

/// Sorted `(file, test, result, category)` of the oracle's items.
fn oracle_tuples(results: &[Value]) -> Vec<Tuple> {
    let mut out = Vec::new();
    for result in results {
        let file = result["file_path"].as_str().unwrap_or("").to_owned();
        for item in result["items"].as_array().into_iter().flatten() {
            let test = match &item["test"] {
                Value::Number(number) => number.to_string(),
                Value::String(text) => text.clone(),
                _ => NO_TEST.to_owned(),
            };
            out.push((
                file.clone(),
                test,
                item["result"].as_str().unwrap_or("").to_owned(),
                item["category"].as_str().unwrap_or("").to_owned(),
            ));
        }
    }
    out.sort();
    out
}

/// Sorted tuples of our files, acknowledged and unreadable ones excluded.
fn our_tuples(files: &[AuditFile]) -> Vec<Tuple> {
    let mut out = Vec::new();
    for file in files {
        if file.acknowledged || file.unreadable.is_some() {
            continue;
        }
        for item in &file.items {
            out.push((
                file.rel_path.clone(),
                item.test.clone().unwrap_or_else(|| NO_TEST.to_owned()),
                item.result.clone(),
                item.category.as_str().to_owned(),
            ));
        }
    }
    out.sort();
    out
}

/// The fixture whose Layer A item names are compared with the oracle's as well (D-16a):
/// owner-facing naming lives only in Layer B, so Layer A must still print the
/// flattened `test: ..., expected: ..., why_human: ...` strings upstream prints.
const NAME_CHECKED: &str = "verification-human-mapping";

/// Sorted `(file, test, name)` of the oracle's items and ours.
fn assert_names_equal(fixture: &str, files: &[AuditFile], results: &[Value]) {
    let mut theirs: Vec<Tuple> = Vec::new();
    for result in results {
        let file = result["file_path"].as_str().unwrap_or("").to_owned();
        for item in result["items"].as_array().into_iter().flatten() {
            let test = match &item["test"] {
                Value::Number(number) => number.to_string(),
                Value::String(text) => text.clone(),
                _ => NO_TEST.to_owned(),
            };
            let name = item["name"].as_str().unwrap_or("").to_owned();
            theirs.push((file.clone(), test, name, String::new()));
        }
    }
    let mut ours: Vec<Tuple> = Vec::new();
    for file in files
        .iter()
        .filter(|f| !f.acknowledged && f.unreadable.is_none())
    {
        for item in &file.items {
            ours.push((
                file.rel_path.clone(),
                item.test.clone().unwrap_or_else(|| NO_TEST.to_owned()),
                item.name.clone(),
                String::new(),
            ));
        }
    }
    ours.sort();
    theirs.sort();
    assert_eq!(
        ours,
        theirs,
        "fixture {fixture}: Layer A names differ from the oracle's\n{}",
        diff(&ours, &theirs)
    );
}

/// Recursively copy `from` into `to`. Committed fixtures hold no symlinks.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("the copy target is creatable");
    for entry in std::fs::read_dir(from).expect("the fixture is readable") {
        let entry = entry.expect("a readable entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("a file type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("the file copies");
        }
    }
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/owner")
}

/// Every directory under `tests/fixtures/owner/` that holds `.planning/`, sorted.
fn discover() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(fixtures_root())
        .expect("tests/fixtures/owner exists")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(".planning").is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn name_of(fixture: &Path) -> String {
    fixture
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("?")
        .to_owned()
}

fn expectation(fixture: &Path) -> String {
    let name = name_of(fixture);
    let text = std::fs::read_to_string(fixture.join("oracle.expect"))
        .unwrap_or_else(|_| panic!("fixture {name} has no oracle.expect"));
    let value = text.trim();
    assert!(
        EXPECTATIONS.contains(&value),
        "fixture {name}: oracle.expect must be one of {EXPECTATIONS:?}, found {value:?}"
    );
    value.to_owned()
}

fn diff(ours: &[Tuple], theirs: &[Tuple]) -> String {
    let mut count: BTreeMap<&Tuple, (usize, usize)> = BTreeMap::new();
    for tuple in ours {
        count.entry(tuple).or_default().0 += 1;
    }
    for tuple in theirs {
        count.entry(tuple).or_default().1 += 1;
    }
    count
        .into_iter()
        .filter(|(_, (a, b))| a != b)
        .map(|(tuple, (a, b))| format!("  ours x{a}, oracle x{b}: {tuple:?}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn ours_equals_the_oracle_on_every_owner_fixture() {
    let fixtures = discover();
    assert!(!fixtures.is_empty(), "no owner fixture was discovered");
    let expectations: Vec<String> = fixtures.iter().map(|f| expectation(f)).collect();

    let Some(oracle) = Oracle::resolve() else {
        let opt_out = std::env::var(ALLOW_MISSING_ORACLE).ok();
        assert!(
            skip_permitted(opt_out.as_deref()),
            "gsd-core's gsd-tools is not available, so Layer A was not compared with `{ORACLE_VERB}`. \
             Install it with scripts/install-conformance-oracle.sh, or set {ALLOW_MISSING_ORACLE}=1 \
             on an environment that genuinely cannot run Node"
        );
        eprintln!("SKIPPED by {ALLOW_MISSING_ORACLE}: no fixture was compared");
        return;
    };

    let mut compared = 0usize;
    let mut diverged = 0usize;
    for (fixture, expect) in fixtures.iter().zip(&expectations) {
        let name = name_of(fixture);
        let tmp = TempDir::new().expect("a temp dir");
        copy_tree(&fixture.join(".planning"), &tmp.path().join(".planning"));
        let planning = tmp.path().join(".planning");
        let state = parse_project_state(&planning);
        let files = scan(&planning, &state);
        let theirs = oracle.audit_all(tmp.path());
        match expect.as_str() {
            "same" => {
                let (results, acknowledged) =
                    theirs.unwrap_or_else(|err| panic!("fixture {name}: the oracle failed: {err}"));
                let ours_tuples = our_tuples(&files);
                let their_tuples = oracle_tuples(&results);
                assert_eq!(
                    ours_tuples,
                    their_tuples,
                    "fixture {name}: our items differ from the oracle's\n{}",
                    diff(&ours_tuples, &their_tuples)
                );
                if name == NAME_CHECKED {
                    assert_names_equal(&name, &files, &results);
                }
                let our_gaps = files
                    .iter()
                    .filter(|f| {
                        !f.acknowledged && f.kind == AuditKind::Uat && f.unparsed_blocks > 0
                    })
                    .count();
                let their_gaps = results
                    .iter()
                    .filter(|r| r["parse_gap"].as_bool() == Some(true))
                    .count();
                assert_eq!(
                    our_gaps, their_gaps,
                    "fixture {name}: parse-gap file counts"
                );
                if !tmp.path().join(".planning/workstreams").is_dir() {
                    let ours = files.iter().filter(|f| f.acknowledged).count() as u64;
                    assert_eq!(ours, acknowledged, "fixture {name}: acknowledged files");
                }
                compared += 1;
            }
            "no-phases-dir" => {
                let err = theirs.expect_err(&format!(
                    "fixture {name}: the oracle no longer fails on a missing phases directory; \
                     the declared divergence has stopped diverging"
                ));
                assert!(
                    err.contains("No phases directory found"),
                    "fixture {name}: unexpected oracle failure: {err}"
                );
                assert!(files.is_empty(), "fixture {name}: we must yield no files");
                diverged += 1;
            }
            "out-of-set-verification" => {
                let err = theirs.expect_err(&format!(
                    "fixture {name}: the oracle no longer rejects an out-of-set status; \
                     the declared divergence has stopped diverging"
                ));
                assert!(
                    err.contains("outside the closed set"),
                    "fixture {name}: unexpected oracle failure: {err}"
                );
                let outside: Vec<&AuditFile> = files
                    .iter()
                    .filter(|f| {
                        f.kind == AuditKind::Verification
                            && !VERIFICATION_STATUSES.contains(&f.status.as_str())
                    })
                    .collect();
                assert!(
                    !outside.is_empty(),
                    "fixture {name}: no out-of-set VERIFICATION seen"
                );
                for file in outside {
                    assert!(
                        matches!(file.unreadable, Some(Unreadable::OutOfSetStatus(_))),
                        "fixture {name}: {} must be unreadable, not clean",
                        file.rel_path
                    );
                }
                diverged += 1;
            }
            other => unreachable!("expectation {other} was validated above"),
        }
    }

    assert!(
        compared >= 1,
        "not one fixture produced a real comparison with the oracle; a run in which every \
         fixture diverged or was skipped passes for the wrong reason"
    );
    assert_eq!(
        compared + diverged,
        fixtures.len(),
        "every fixture must land in exactly one arm; {compared} compared and {diverged} diverged \
         out of {} fixtures",
        fixtures.len()
    );
    eprintln!(
        "owner conformance: {compared} compared, {diverged} declared divergences, over {} fixtures",
        fixtures.len()
    );
}

#[test]
fn a_missing_oracle_is_a_failure_unless_the_environment_says_otherwise() {
    assert!(
        !skip_permitted(None),
        "an unset variable must NOT permit a skip: the default has to be fail-loud"
    );
    assert!(
        !skip_permitted(Some("")),
        "an empty value is how a variable is unset by accident, not consent"
    );
    assert!(!skip_permitted(Some("   ")), "nor a whitespace-only one");
    assert!(
        skip_permitted(Some("1")),
        "an environment that cannot run Node must have a way to say so"
    );
}

/// Every `*.rs` file under `src/`, as (relative path, lines), sorted.
fn source_files() -> Vec<(String, Vec<String>)> {
    fn collect(dir: &Path, out: &mut Vec<(String, Vec<String>)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                collect(&path, out);
            } else if path.extension().and_then(OsStr::to_str) == Some("rs") {
                if let Ok(text) = std::fs::read_to_string(&path) {
                    out.push((
                        path.display().to_string(),
                        text.lines().map(str::to_owned).collect(),
                    ));
                }
            }
        }
    }
    let mut out = Vec::new();
    collect(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"), &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!out.is_empty(), "the walk found no Rust source under src/");
    out
}

#[test]
fn no_source_file_names_the_audit_oracle_verb() {
    let files = source_files();
    let mut hits = Vec::new();
    for (path, lines) in &files {
        for (number, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if line.contains(ORACLE_VERB) {
                hits.push(format!("{path}:{}: {}", number + 1, line.trim()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "the audit oracle is a TEST-time tool and must never be reachable from the product: \
         {hits:?}"
    );
    let executable: usize = files
        .iter()
        .flat_map(|(_, lines)| lines.iter())
        .filter(|line| !line.trim_start().starts_with("//"))
        .count();
    assert!(
        executable > 1000,
        "the scan examined only {executable} executable lines, too few for its silence to mean anything"
    );
}
