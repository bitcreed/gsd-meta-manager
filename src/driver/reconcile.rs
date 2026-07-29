//! **This module performs zero disk writes** (D-12).
//!
//! That is the module's governing rule, it is stated first because everything
//! else here is subordinate to it, and it is enforced three ways: by
//! [`the_reconcile_module_contains_no_write_call`](tests), by a shell-checkable
//! negative grep over this file, and by a before/after fingerprint of an entire
//! `.planning` tree in [`the_scan_leaves_the_planning_tree_byte_identical`](tests).
//!
//! Three specific repairs a well-meaning implementation would add, and why each
//! is destructive:
//!
//! * **It must not repair `run.json`.** That document is written exactly twice
//!   across a run's lifetime and nothing else ever rewrites it; a third write
//!   breaks the `debug_assert_eq!(self.record_writes, 2)` contract at
//!   `src/journal/mod.rs:636-639`. Worse, stamping an `ended_at` onto a crashed
//!   run destroys the only evidence of *how* it died — an absent `ended_at`
//!   beside a dead pid is not a defect in the record, it **is** the crash
//!   record, and it is already correct on disk.
//! * **It must not clear a stale `active` pointer.** The pointer plus a missing
//!   `ended_at` plus a dead pid is the whole of the crash signal, and
//!   `writer::read_active_run` already treats a pointer inconsistent with the
//!   directory listing as ignorable at read time. Nothing needs tidying; a tidy
//!   here is a deletion of evidence.
//! * **It must not prune.** Retention runs at run *start*, by design, precisely
//!   because an exit-time prune is skipped by exactly the crash this module is
//!   reading. A prune here would delete the run directory the verdict is about.
//!
//! **The adopted state is "observed", not "reattached" and not "streaming"**
//! (D-11). Once the TUI exits, the driver's stdout pipe is gone; live output
//! re-streaming after a restart is physically impossible, and REQUIREMENTS lists
//! it as out of scope for exactly that reason. What this module recovers is the
//! journal tail — history and current step — plus the handles a kill-by-pgid
//! needs. The verdict lives in memory and is surfaced; the disk stays exactly as
//! the driver left it. Code, docs and status text must not promise otherwise.

use std::collections::HashMap;
use std::path::Path;

use crate::config::RegisteredProject;
use crate::driver::liveness;
use crate::journal::{run_paths, writer};

/// What the scan concluded about one project's most recent run.
///
/// `Ended` is a real verdict rather than an absence: it is the difference
/// between "this project has no run to observe because the last one finished
/// cleanly" and "this project has no run to observe because its record is
/// unreadable", and only the first is good news.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunVerdict {
    /// The record has no `ended_at` and the pid is still this run's driver.
    Live,
    /// The record has no `ended_at` and the pid is gone — the driver died
    /// without reaching its terminal write. **Nothing is repaired.**
    CrashedWithoutEnding,
    /// The record carries an `ended_at`. The run is over.
    Ended,
}

/// One run the scan observed, live or crashed.
///
/// The type is `Clone` because [`crate::action::Action`] derives `Clone` and
/// this value travels as an `Action` payload.
///
/// It holds **ids, counts and strings — never a file handle and never a join
/// handle**, which is the same rule the sibling maps in
/// `src/ui/screens/mod.rs:126-137` carry. A handle is not `Clone`, so putting
/// one here would not merely be untidy: it would make `Action` uncloneable and
/// the failure would surface a long way from its cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedRun {
    /// The registry alias whose project this run belongs to.
    ///
    /// `run.json` has no alias or project-path field — the project is implied by
    /// which `.planning/` tree the run directory sits in — so the alias is
    /// supplied by the scan rather than read from disk.
    pub alias: String,
    /// The run id, matching its directory name.
    pub run_id: String,
    /// The driver's process id, as the driver recorded it.
    pub pid: u32,
    /// The driver's process group id. Equal to `pid`, because the driver makes
    /// itself its own group leader (D-04). This is the kill switch's handle.
    pub pgid: u32,
    /// RFC3339 UTC timestamp of the run's first write.
    pub started_at: String,
    /// The originating goal prompt, verbatim.
    pub goal: String,
    /// The GSD command the run was started with.
    pub gsd_command: String,
    /// Whether the pid **and** cmdline double-check says this run is still
    /// running. `false` here means [`RunVerdict::CrashedWithoutEnding`].
    pub live: bool,
}

impl ObservedRun {
    /// The verdict this observation carries.
    ///
    /// [`RunVerdict::Ended`] is never returned: an ended run yields no
    /// `ObservedRun` at all, because there is nothing left to observe.
    pub fn verdict(&self) -> RunVerdict {
        if self.live {
            RunVerdict::Live
        } else {
            RunVerdict::CrashedWithoutEnding
        }
    }
}

/// The verdict for one run, as a pure function of the two inputs that decide it.
///
/// Separated out so the decision is unit-testable without a process or a disk.
fn classify(ended_at: Option<&str>, alive: bool) -> RunVerdict {
    match (ended_at, alive) {
        (Some(_), _) => RunVerdict::Ended,
        (None, true) => RunVerdict::Live,
        (None, false) => RunVerdict::CrashedWithoutEnding,
    }
}

/// The six fields of a `run.json` this scan actually needs.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RunFacts {
    ended_at: Option<String>,
    pid: u32,
    pgid: u32,
    started_at: String,
    goal: String,
    gsd_command: String,
}

/// Read the run record as raw JSON, **not** through
/// [`crate::journal::RunRecord`].
///
/// The reasoning is `writer::has_end_timestamp`'s, and it applies here for the
/// same reason: a record written by a **later** schema must still answer this
/// question. Deserialising through today's struct would make a run written by a
/// newer build invisible to an older TUI — which is precisely the situation a
/// user hits mid-upgrade, when a driver from the new binary is running and the
/// old TUI is the one asking. The only fields this question needs are the six
/// below.
///
/// A missing, unreadable or unparseable record yields `None`, and the caller
/// treats that as **"no observable run" and never as "crashed"**. An unreadable
/// file is not evidence of a death; reporting it as one would manufacture crash
/// reports out of permission errors.
fn read_run_facts(run_dir: &Path) -> Option<RunFacts> {
    let raw = std::fs::read_to_string(run_dir.join("run.json")).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&raw).ok()?;

    let string_field = |key: &str| -> String {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };

    Some(RunFacts {
        ended_at: value
            .get("ended_at")
            .filter(|v| !v.is_null())
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        pid: value.get("pid").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        pgid: value.get("pgid").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        started_at: string_field("started_at"),
        goal: string_field("goal"),
        gsd_command: string_field("gsd_command"),
    })
}

/// Observe one project's most recent run, or report that there is none.
///
/// Reads, in order: the `active` pointer for the run id, then that run's
/// `run.json` for the six facts, then `/proc` for liveness. Every one of those
/// is a read.
///
/// Returns `None` when the project has no active pointer, when its record is
/// unreadable, or when the run ended cleanly — in the last case the verdict is
/// [`RunVerdict::Ended`] and there is simply no live or crashed run to surface.
///
/// A **crashed** run is returned exactly like a live one, with `live == false`.
/// It is as worth surfacing as a live one: a run that died without an ending is
/// the thing a user most needs to be told about on restart, and suppressing it
/// would leave the evidence on disk and nothing on screen.
pub fn reconcile_one(alias: &str, project_root: &Path) -> Option<ObservedRun> {
    let planning_dir = project_root.join(".planning");
    let run_id = writer::read_active_run(&planning_dir)?;
    let paths = run_paths(&planning_dir, &run_id);
    let facts = read_run_facts(&paths.dir)?;

    let alive = facts.ended_at.is_none() && liveness::is_run_alive(facts.pid, &run_id);
    match classify(facts.ended_at.as_deref(), alive) {
        RunVerdict::Ended => None,
        RunVerdict::Live | RunVerdict::CrashedWithoutEnding => Some(ObservedRun {
            alias: alias.to_string(),
            run_id,
            pid: facts.pid,
            pgid: facts.pgid,
            started_at: facts.started_at,
            goal: facts.goal,
            gsd_command: facts.gsd_command,
            live: alive,
        }),
    }
}

/// Observe every registered project, returning one entry per project that has a
/// live or crashed run.
///
/// This deliberately runs over **every** registered project, including ones the
/// user never opted into driving, and that is safe precisely because the scan is
/// a read: reading a project's `.planning/` is what this tool already does on
/// every watcher event and every session poll, and D-12's zero-write rule is
/// what keeps it true here. A project that never opted in has no runs directory,
/// so `read_active_run` yields `None` and it contributes nothing.
///
/// The result is a `Vec` rather than a map because it travels as an `Action`
/// payload and the receiver keys it by alias on arrival; building the map twice
/// would be the only difference.
pub fn reconcile_all(projects: &HashMap<String, RegisteredProject>) -> Vec<ObservedRun> {
    let mut observed: Vec<ObservedRun> = projects
        .iter()
        .filter_map(|(alias, project)| reconcile_one(alias, &project.path))
        .collect();
    // A `HashMap` iterates in an unspecified order; sorting makes the result a
    // function of the inputs alone, which is what lets the `RunsReconciled`
    // handler compare two scans for equality and skip a redraw.
    observed.sort_by(|a, b| a.alias.cmp(&b.alias));
    observed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::RunRecord;

    /// A pid no process can plausibly own.
    const DEAD_PID: u32 = 999_999_999;

    const RUN_ID: &str = "2026-07-29T12-00-00Z-aaaa";

    /// A run record with the shape the driver writes, parameterised on the two
    /// fields this module reads for its verdict.
    fn record(run_id: &str, pid: u32, ended_at: Option<&str>) -> RunRecord {
        RunRecord {
            run_id: run_id.to_string(),
            goal: "ship the thing".to_string(),
            gsd_command: "/gsd-progress".to_string(),
            target: "local".to_string(),
            opt_in: Some("2026-07-29T11:59:00Z".to_string()),
            started_at: "2026-07-29T12:00:00Z".to_string(),
            session_id: "s".to_string(),
            pid,
            pgid: pid,
            claude_code_version: String::new(),
            argv_digest: "fnv1a64:0000000000000000".to_string(),
            ended_at: ended_at.map(|s| s.to_string()),
            outcome: ended_at.map(|_| "completed".to_string()),
        }
    }

    /// Fabricate a run on disk, exactly the way the driver does.
    ///
    /// **Deliberately routed through the production writers** rather than
    /// hand-rolled file writes, for two independent reasons. It is more faithful
    /// — the fixture is the real layout, not this test's idea of it. And it is
    /// what keeps this file free of the write verbs the module-level guard and
    /// the shell-side negative grep both search for, neither of which knows a
    /// test module from production code (and neither of which should have to).
    fn fabricate_run(project_root: &Path, run_id: &str, pid: u32, ended_at: Option<&str>) {
        let planning = project_root.join(".planning");
        let paths = writer::create_run_dir(&planning, run_id).expect("run directory");
        writer::write_run_record(&paths, &record(run_id, pid, ended_at)).expect("run record");
        writer::write_active_pointer(&crate::journal::runs_root(&planning), run_id)
            .expect("active pointer");
    }

    /// A one-entry registry pointing at `root`.
    fn registry(alias: &str, root: &Path) -> HashMap<String, RegisteredProject> {
        let mut projects = HashMap::new();
        projects.insert(
            alias.to_string(),
            RegisteredProject {
                path: root.to_path_buf(),
                added: "2026-07-29T12:00:00Z".to_string(),
                driver_opt_in: None,
                extra: Default::default(),
            },
        );
        projects
    }

    /// Every file under `dir`, as `(relative path, byte length, content digest)`.
    ///
    /// Content is digested and not merely measured: the most plausible unwanted
    /// write here is a `run.json` rewrite that stamps an `ended_at` of the same
    /// width, or an `active` pointer rewritten with the same run id, and a
    /// length-only comparison cannot see either.
    fn fingerprint(dir: &Path) -> Vec<(String, u64, String)> {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, u64, String)>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let Ok(kind) = entry.file_type() else { continue };
                if kind.is_dir() {
                    walk(root, &path, out);
                } else {
                    let bytes = std::fs::read(&path).unwrap_or_default();
                    let relative = path
                        .strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .into_owned();
                    let digest =
                        crate::journal::argv_digest(&[String::from_utf8_lossy(&bytes).into_owned()]);
                    out.push((relative, bytes.len() as u64, digest));
                }
            }
        }

        let mut out = Vec::new();
        walk(dir, dir, &mut out);
        out.sort();
        out
    }

    #[test]
    fn a_run_with_an_end_timestamp_is_not_reported_live() {
        let root = tempfile::TempDir::new().expect("temp dir");
        fabricate_run(root.path(), RUN_ID, DEAD_PID, Some("2026-07-29T12:05:00Z"));

        assert_eq!(
            reconcile_one("demo", root.path()),
            None,
            "a run that reached its terminal write has nothing left to observe"
        );
        assert_eq!(
            classify(Some("2026-07-29T12:05:00Z"), false),
            RunVerdict::Ended
        );
    }

    #[test]
    fn a_run_without_an_end_timestamp_and_a_dead_pid_is_reported_crashed() {
        let root = tempfile::TempDir::new().expect("temp dir");
        fabricate_run(root.path(), RUN_ID, DEAD_PID, None);

        let observed = reconcile_one("demo", root.path()).expect("a crashed run is still observed");
        assert_eq!(observed.run_id, RUN_ID);
        assert_eq!(observed.alias, "demo");
        assert!(!observed.live, "a dead pid is not live");
        assert_eq!(observed.verdict(), RunVerdict::CrashedWithoutEnding);
        // The facts came off `run.json`, not from a default.
        assert_eq!(observed.goal, "ship the thing");
        assert_eq!(observed.gsd_command, "/gsd-progress");
        assert_eq!(observed.pid, DEAD_PID);
        assert_eq!(observed.pgid, DEAD_PID);
        assert!(!observed.started_at.is_empty());
    }

    #[test]
    fn the_scan_leaves_the_planning_tree_byte_identical() {
        let root = tempfile::TempDir::new().expect("temp dir");
        fabricate_run(root.path(), RUN_ID, DEAD_PID, None);
        let planning = root.path().join(".planning");

        let before = fingerprint(&planning);
        // Pinned against vacuity: two empty walks compare equal to each other,
        // so a fingerprint that silently found nothing would "prove" this.
        assert!(
            before
                .iter()
                .any(|(path, _, _)| path.ends_with("run.json")),
            "the walk must have seen the run record it is protecting, got: {before:?}"
        );

        let observed = reconcile_all(&registry("demo", root.path()));
        assert_eq!(observed.len(), 1, "the fabricated crashed run is surfaced");

        let after = fingerprint(&planning);
        assert_eq!(
            before, after,
            "the scan must leave the .planning tree BYTE-IDENTICAL. A repaired \
             run.json breaks the exactly-twice write contract and destroys the \
             evidence of how the run died (D-12)"
        );
    }

    #[test]
    fn a_stale_active_pointer_is_not_cleared_by_the_scan() {
        let root = tempfile::TempDir::new().expect("temp dir");
        fabricate_run(root.path(), RUN_ID, DEAD_PID, None);
        let planning = root.path().join(".planning");
        let pointer = crate::journal::runs_root(&planning).join("active");

        let before = std::fs::read(&pointer).expect("the pointer is on disk");

        let _ = reconcile_all(&registry("demo", root.path()));

        assert!(
            pointer.is_file(),
            "the stale pointer must still exist — it is half the crash record"
        );
        assert_eq!(
            before,
            std::fs::read(&pointer).expect("the pointer survives the scan"),
            "the stale pointer must still name the dead run. Clearing it is the \
             specific 'repair' a well-meaning implementation adds, and it deletes \
             the evidence (D-12)"
        );
    }

    #[test]
    fn an_unreadable_record_is_not_reported_as_a_crash() {
        // A project with no runs at all: `read_active_run` yields `None` and the
        // scan contributes nothing. The distinction matters because "no record"
        // must never be rendered as "your run died".
        let root = tempfile::TempDir::new().expect("temp dir");
        assert_eq!(reconcile_one("demo", root.path()), None);
        assert!(reconcile_all(&registry("demo", root.path())).is_empty());
    }

    /// The halves of every write verb this module forbids itself, joined at
    /// **runtime**.
    ///
    /// Split deliberately: spelled out as whole literals, this table would be a
    /// non-comment line containing the very strings the guard searches for, and
    /// the guard would report itself. The halves are meaningless apart. The same
    /// technique fences `tests/spawn_seam_guard.rs` against its own source.
    const WRITE_VERB_HALVES: &[(&str, &str)] = &[
        ("fs::", "write"),
        ("File::", "create"),
        ("Open", "Options"),
        ("remove_", "file"),
        ("remove_", "dir"),
        ("create_", "dir"),
        ("ren", "ame"),
        ("per", "sist"),
        ("clear_active_", "pointer"),
        ("prune_", "runs"),
    ];

    #[test]
    fn the_reconcile_module_contains_no_write_call() {
        // `include_str!` rather than a path read at runtime: a path read can
        // fail, or resolve somewhere else under a different working directory,
        // and a guard that silently reads nothing passes vacuously. The macro is
        // resolved by the compiler against this file's own location.
        const SOURCE: &str = include_str!("reconcile.rs");

        let verbs: Vec<String> = WRITE_VERB_HALVES
            .iter()
            .map(|(head, tail)| format!("{head}{tail}"))
            .collect();

        // Comment lines are filtered so this module's own documentation may name
        // the calls it forbids — a doc that states "never clear the active
        // pointer" necessarily contains the string, and a guard that could not
        // tolerate that would forbid explaining itself.
        let offenders: Vec<(usize, &str, String)> = SOURCE
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim_start().starts_with("//"))
            .filter_map(|(index, line)| {
                verbs
                    .iter()
                    .find(|verb| line.contains(verb.as_str()))
                    .map(|verb| (index + 1, line.trim(), verb.clone()))
            })
            .collect();

        assert!(
            offenders.is_empty(),
            "this module performs ZERO disk writes (D-12). A write here would \
             break the exactly-twice run.json contract and destroy the crash \
             evidence the scan exists to report. Offending lines: {offenders:?}"
        );
    }
}
