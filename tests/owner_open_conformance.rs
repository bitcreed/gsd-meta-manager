// ============================================================================
// Layer A of the owner-items reader's open signals against gsd-core's own
// `audit-open` (OWN-10, D-17; the debug half, plan 35-01).
//
// **What this file proves.** `owner_items::scan_open` ports gsd-core 1.21.0's
// `scanDebugSessions`. For every committed fixture under `tests/fixtures/owner/` that
// carries an `audit-open.expect` reading `same`, this test runs the real oracle from
// the root of a copy of the tree and requires the sorted `(slug, status)` of its open
// debug sessions, and the number of acknowledged ones, to equal ours. Unreadable
// sessions are excluded on our side (upstream skips them; the owner rules turn them
// into Fix work, as Phase 34 treats unreadable UAT files). Layer B (lanes, groups,
// commands, the recovered status of a frontmatter YAML rejects) is never compared.
//
// **The context-question half.** The oracle runs once from the root and once per
// workstream directory (`GSD_WORKSTREAM`). For each run the sorted
// `(archived_milestone or "-", phase, file, question_count)` of its
// `items.context_questions`, and its `acknowledged.context_questions`, must equal ours
// over what that run can see: the active phases of its own tree plus the archived ones
// (every archive in the root run; only the workstream's own, under its bare label, in a
// workstream run). Debug sessions are root-scoped and compared from the root run only.
//
// **A missing oracle FAILS; skipping is opt-in.** As in `owner_audit_conformance.rs`:
// the default is to fail with the install hint, an environment that cannot run Node
// says so through `GSD_META_MANAGER_ALLOW_MISSING_ORACLE`, and a run in which no
// fixture compared fails.
//
// **The oracle runs at test time only.** `no_source_file_names_the_open_audit_verb`
// scans every non-comment line under `src/` for the verb.
//
// Shape copied from `tests/owner_audit_conformance.rs`, deliberately not abstracted.
// ============================================================================

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use gsd_meta_manager::state_reader::gsd_install::{install_candidates, InstallRoots};
use gsd_meta_manager::state_reader::owner_items::{scan_open, OpenScan, Scope};
use gsd_meta_manager::state_reader::parse_project_state;
use serde_json::Value;
use tempfile::TempDir;

/// The oracle verb: the one place it is spelled outside comments in this crate's
/// tests, and spelled nowhere under `src/`.
const ORACLE_VERB: &str = "audit-open";

/// The variable an environment sets to say that it genuinely cannot run the oracle
/// and accepts a conformance run that checked nothing. Empty and whitespace-only
/// values are not an opt-out.
const ALLOW_MISSING_ORACLE: &str = "GSD_META_MANAGER_ALLOW_MISSING_ORACLE";

/// The marker file naming a fixture the open-signal oracle compares.
const MARKER: &str = "audit-open.expect";

/// The `audit-open.expect` values a fixture may carry.
const EXPECTATIONS: [&str; 1] = ["same"];

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

    /// One oracle run from the root of `root`: the parsed `--json` document, or the
    /// combined output of a failed run.
    fn audit_open(&self, root: &Path, workstream: Option<&str>) -> Result<Value, String> {
        let mut command = self.query_command(ORACLE_VERB);
        command
            .arg("--json")
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
        serde_json::from_str(&text)
            .map_err(|err| format!("the oracle's output did not parse as JSON: {err}"))
    }
}

/// Sorted `(slug, status)` of the oracle's open debug sessions; a `scan_error`
/// sentinel is an error.
fn oracle_debug(doc: &Value) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for item in doc["items"]["debug_sessions"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if item["scan_error"].as_bool() == Some(true) {
            return Err("the oracle reported a scan_error sentinel".to_owned());
        }
        out.push((
            item["slug"].as_str().unwrap_or("").to_owned(),
            item["status"].as_str().unwrap_or("").to_owned(),
        ));
    }
    out.sort();
    Ok(out)
}

/// Sorted `(slug, status)` of our open, readable, unacknowledged sessions.
fn our_debug(open: &OpenScan) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = open
        .debug_sessions
        .iter()
        .filter(|s| !s.acknowledged && s.unreadable.is_none())
        .map(|s| (s.slug.clone(), s.status.clone()))
        .collect();
    out.sort();
    out
}

type ContextTuple = (String, String, String, u64);

/// Sorted `(archived_milestone or "-", phase, file, question_count)` of one oracle
/// run's context questions; a `scan_error` sentinel is an error.
fn oracle_context(doc: &Value) -> Result<Vec<ContextTuple>, String> {
    let mut out = Vec::new();
    for item in doc["items"]["context_questions"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if item["scan_error"].as_bool() == Some(true) {
            return Err("the oracle reported a scan_error sentinel".to_owned());
        }
        out.push((
            item["archived_milestone"]
                .as_str()
                .unwrap_or("-")
                .to_owned(),
            item["phase"].as_str().unwrap_or("").to_owned(),
            item["file"].as_str().unwrap_or("").to_owned(),
            item["question_count"].as_u64().unwrap_or(0),
        ));
    }
    out.sort();
    Ok(out)
}

/// Our readable files of one oracle run: `(milestone label, file)` for the files that
/// run can see, `workstream` being `None` for the root run.
fn visible_context<'a>(
    open: &'a OpenScan,
    workstream: Option<&str>,
) -> Vec<(
    String,
    &'a gsd_meta_manager::state_reader::owner_items::ContextFile,
)> {
    open.context_files
        .iter()
        .filter(|file| file.unreadable.is_none())
        .filter_map(|file| match &file.scope {
            Scope::Current if file.workstream.as_deref() == workstream => {
                Some(("-".to_owned(), file))
            }
            Scope::Current => None,
            Scope::Archived { label, .. } => match workstream {
                None => Some((label.clone(), file)),
                Some(stream) if file.workstream.as_deref() == Some(stream) => {
                    let bare = label.strip_prefix(&format!("{stream}/")).unwrap_or(label);
                    Some((bare.to_owned(), file))
                }
                Some(_) => None,
            },
        })
        .collect()
}

fn basename(rel_path: &str) -> String {
    rel_path.rsplit('/').next().unwrap_or(rel_path).to_owned()
}

/// One oracle run's context questions against ours.
fn compare_context(
    name: &str,
    run: &str,
    open: &OpenScan,
    workstream: Option<&str>,
    doc: &Value,
) -> Result<(), String> {
    let theirs = oracle_context(doc)?;
    let visible = visible_context(open, workstream);
    let mut ours: Vec<ContextTuple> = visible
        .iter()
        .filter(|(_, file)| !file.acknowledged)
        .map(|(label, file)| {
            (
                label.clone(),
                file.phase.clone(),
                basename(&file.rel_path),
                file.questions.len() as u64,
            )
        })
        .collect();
    ours.sort();
    if ours != theirs {
        return Err(format!(
            "fixture {name} ({run}): context questions differ\n  ours:   {ours:?}\n  oracle: {theirs:?}"
        ));
    }
    let our_ack = visible.iter().filter(|(_, file)| file.acknowledged).count() as u64;
    let their_ack = doc["acknowledged"]["context_questions"]
        .as_u64()
        .unwrap_or(0);
    if our_ack != their_ack {
        return Err(format!(
            "fixture {name} ({run}): acknowledged context files: ours {our_ack}, oracle {their_ack}"
        ));
    }
    Ok(())
}

/// The per-category comparison of one fixture's root run: debug sessions and context
/// questions.
fn compare_open(name: &str, open: &OpenScan, doc: &Value) -> Result<(), String> {
    compare_context(name, "root", open, None, doc)?;
    let theirs = oracle_debug(doc)?;
    let ours = our_debug(open);
    if ours != theirs {
        return Err(format!(
            "fixture {name}: debug sessions differ\n{}",
            diff(&ours, &theirs)
        ));
    }
    let our_ack = open
        .debug_sessions
        .iter()
        .filter(|s| s.acknowledged && s.unreadable.is_none())
        .count() as u64;
    let their_ack = doc["acknowledged"]["debug_sessions"].as_u64().unwrap_or(0);
    if our_ack != their_ack {
        return Err(format!(
            "fixture {name}: acknowledged debug sessions: ours {our_ack}, oracle {their_ack}"
        ));
    }
    Ok(())
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

/// Every directory under `tests/fixtures/owner/` that holds `.planning/` and the
/// marker file, sorted.
fn discover() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(fixtures_root())
        .expect("tests/fixtures/owner exists")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.join(".planning").is_dir() && path.join(MARKER).is_file())
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
    let text = std::fs::read_to_string(fixture.join(MARKER))
        .unwrap_or_else(|_| panic!("fixture {name} has no {MARKER}"));
    let value = text.trim();
    assert!(
        EXPECTATIONS.contains(&value),
        "fixture {name}: {MARKER} must be one of {EXPECTATIONS:?}, found {value:?}"
    );
    value.to_owned()
}

fn diff(ours: &[(String, String)], theirs: &[(String, String)]) -> String {
    let mut count: BTreeMap<&(String, String), (usize, usize)> = BTreeMap::new();
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
fn ours_equals_the_open_audit_oracle_on_every_marked_fixture() {
    let fixtures = discover();
    assert!(
        !fixtures.is_empty(),
        "no fixture carries {MARKER}, so nothing would be compared"
    );
    for fixture in &fixtures {
        assert_eq!(expectation(fixture), "same");
    }

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
    for fixture in &fixtures {
        let name = name_of(fixture);
        let tmp = TempDir::new().expect("a temp dir");
        copy_tree(&fixture.join(".planning"), &tmp.path().join(".planning"));
        let planning = tmp.path().join(".planning");
        let state = parse_project_state(&planning);
        let open = scan_open(&planning, &state);
        let doc = oracle
            .audit_open(tmp.path(), None)
            .unwrap_or_else(|err| panic!("fixture {name}: the oracle failed: {err}"));
        if let Err(message) = compare_open(&name, &open, &doc) {
            panic!("{message}");
        }
        let mut streams: Vec<String> = std::fs::read_dir(planning.join("workstreams"))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .filter_map(|entry| entry.file_name().into_string().ok())
            .collect();
        streams.sort();
        for stream in streams {
            let doc = oracle
                .audit_open(tmp.path(), Some(&stream))
                .unwrap_or_else(|err| {
                    panic!("fixture {name} ({stream}): the oracle failed: {err}")
                });
            if let Err(message) = compare_context(&name, &stream, &open, Some(&stream), &doc) {
                panic!("{message}");
            }
        }
        compared += 1;
    }
    assert!(
        compared >= 1,
        "not one fixture produced a real comparison with the oracle"
    );
    assert_eq!(
        compared,
        fixtures.len(),
        "every marked fixture must compare"
    );
    eprintln!("open conformance: {compared} fixtures compared with the oracle");
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

/// The one file under `src/` that may spell the oracle verb (37 D-14, RESEARCH
/// Pitfall 1): `ACK_VERB`, the owner inbox's acknowledge argv, lives there.
const ACK_SITE: &str = "src/ui/screens/owner_actions.rs";

/// What every allowed line must also contain: only the writer's own subcommand is
/// allowed, so the read verb (`audit-open`, `audit-open run`) stays unreachable
/// from the product.
const ACK_SUBCOMMAND: &str = "acknowledge";

/// Every non-comment line that names the oracle verb and is not the acknowledge
/// writer's own spelling in [`ACK_SITE`].
fn forbidden_verb_hits(files: &[(String, Vec<String>)]) -> Vec<String> {
    let mut hits = Vec::new();
    for (path, lines) in files {
        for (number, line) in lines.iter().enumerate() {
            if line.trim_start().starts_with("//") || !line.contains(ORACLE_VERB) {
                continue;
            }
            let allowed = Path::new(path).ends_with(ACK_SITE) && line.contains(ACK_SUBCOMMAND);
            if !allowed {
                hits.push(format!("{path}:{}: {}", number + 1, line.trim()));
            }
        }
    }
    hits
}

#[test]
fn no_source_file_names_the_open_audit_verb() {
    let files = source_files();
    let hits = forbidden_verb_hits(&files);
    assert!(
        hits.is_empty(),
        "the audit oracle's read verb is a TEST-time tool and must never be reachable from the \
         product; only the acknowledge writer's spelling in {ACK_SITE} is allowed: {hits:?}"
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
    // The allowance is not vacuous: the writer's spelling really is there.
    let spelled = files
        .iter()
        .filter(|(path, _)| Path::new(path).ends_with(ACK_SITE))
        .flat_map(|(_, lines)| lines.iter())
        .filter(|line| !line.trim_start().starts_with("//") && line.contains(ORACLE_VERB))
        .count();
    assert_eq!(spelled, 1, "{ACK_SITE} must spell the verb exactly once");
}

/// The narrowed guard still fails on a planted read verb, in the allowed file and
/// anywhere else, and on the writer's spelling outside the allowed file.
#[test]
fn the_narrowed_verb_guard_still_catches_the_read_verb() {
    let planted = |path: &str, line: &str| vec![(path.to_owned(), vec![line.to_owned()])];
    let read = format!("let verb = \"{ORACLE_VERB}\";");
    let write = format!("[\"{ORACLE_VERB}\", \"{ACK_SUBCOMMAND}\"]");
    assert_eq!(
        forbidden_verb_hits(&planted(ACK_SITE, &read)).len(),
        1,
        "the read verb in the allowed file"
    );
    assert_eq!(
        forbidden_verb_hits(&planted("src/ui/screens/inbox.rs", &write)).len(),
        1,
        "the writer's spelling in any other file"
    );
    assert!(
        forbidden_verb_hits(&planted(ACK_SITE, &write)).is_empty(),
        "the writer's spelling in the allowed file"
    );
    assert!(
        forbidden_verb_hits(&planted("src/x.rs", &format!("// {read}"))).is_empty(),
        "a comment is not a use"
    );
}
