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
use crate::driver::liveness::{self, Liveness};
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
    /// The record has no `ended_at` and this platform cannot say whether the
    /// driver lives — **which is emphatically not a crash report** (CR-05).
    ///
    /// The distinction is the whole reason this variant exists. The dashboard
    /// renders [`CrashedWithoutEnding`](Self::CrashedWithoutEnding) as *"your run
    /// died"*; on a platform where the `/proc` probe simply does not apply, that
    /// sentence would be printed about every healthy run, on every scan, for as
    /// long as the run lasted. What is true there is that nothing is known, and
    /// this is the value that says so.
    LivenessUnknown,
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
    /// What the pid **and** cmdline double-check established about this run.
    ///
    /// **This was a `bool` until plan 17-08, and the `bool` was the defect**
    /// (CR-05). Two states cannot carry three answers, so "the probe does not
    /// apply on this platform" had to be spelled as one of the two — and it was
    /// spelled `false`, which [`verdict`](ObservedRun::verdict) turned into
    /// [`RunVerdict::CrashedWithoutEnding`] and the dashboard turned into *"your
    /// run died"*. A field that cannot represent "I do not know" forces every
    /// consumer to invent an answer.
    pub liveness: Liveness,
}

impl ObservedRun {
    /// The verdict this observation carries.
    ///
    /// [`RunVerdict::Ended`] is never returned: an ended run yields no
    /// `ObservedRun` at all, because there is nothing left to observe.
    pub fn verdict(&self) -> RunVerdict {
        match self.liveness {
            Liveness::Alive => RunVerdict::Live,
            Liveness::Dead => RunVerdict::CrashedWithoutEnding,
            Liveness::Unknown => RunVerdict::LivenessUnknown,
        }
    }

    /// Whether this run is **positively** known to be running.
    ///
    /// The narrow reading is deliberate and is what every caller wants: the
    /// concurrency cap counts runs that are consuming quota, and the dashboard
    /// marks runs that are working. An undeterminable answer is neither, and
    /// treating it as live would leave a project permanently unstartable.
    pub fn is_live(&self) -> bool {
        self.liveness == Liveness::Alive
    }
}

/// The verdict for one run, as a pure function of the two inputs that decide it.
///
/// Separated out so the decision is unit-testable without a process or a disk —
/// and, since plan 17-08, so the [`Liveness::Unknown`] arm is testable **at
/// all**: it is unreachable at runtime on Linux, so a pure function fed the
/// non-Linux answer is the only way to exercise it (D-05).
///
/// `Some(ended_at)` wins over every liveness answer, including `Unknown`: a
/// record that reached its terminal write is over regardless of what `/proc`
/// can or cannot say about the pid that wrote it.
fn classify(ended_at: Option<&str>, liveness: Liveness) -> RunVerdict {
    match (ended_at, liveness) {
        (Some(_), _) => RunVerdict::Ended,
        (None, Liveness::Alive) => RunVerdict::Live,
        (None, Liveness::Dead) => RunVerdict::CrashedWithoutEnding,
        (None, Liveness::Unknown) => RunVerdict::LivenessUnknown,
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
///
/// **A `pid` or `pgid` that is absent, zero, or too large to be a pid yields
/// `None` for exactly the same reason** (plan 17-08, T-17-08-08). The previous
/// `unwrap_or(0) as u32` did two dangerous things silently: it turned a missing
/// or non-numeric field into `0`, and it *truncated* a `u64` — so `4294967297`
/// became `1`, the init process. **Zero is the single most dangerous value in
/// this codebase**: `kill(0, sig)` signals the caller's own process group, which
/// under a TUI is the user's whole terminal session. `signal_group`'s refusal is
/// the backstop; this is the value never reaching it in the first place. Refusing
/// the record entirely is the posture this function already takes for an
/// unreadable one, and a record whose pid cannot be believed is unreadable in
/// every sense that matters here.
fn read_run_facts(run_dir: &Path) -> Option<RunFacts> {
    let raw = std::fs::read_to_string(run_dir.join("run.json")).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    run_facts_from_value(&value)
}

/// The field extraction, split out of [`read_run_facts`] so the clamp above is
/// testable **without a disk write**.
///
/// The split is not cosmetic. The three records the clamp has to reject — `pid`
/// absent, `pid` zero, `pid` past `u32::MAX` — cannot all be produced through
/// `writer::write_run_record`, because [`crate::journal::RunRecord::pid`] is a
/// `u32` and two of the three are not representable in one. Producing them as raw
/// JSON would mean a file write inside this module's own test code, which
/// `the_reconcile_module_contains_no_write_call` forbids **and should keep
/// forbidding**: that guard cannot tell a test module from production code, and
/// it is worth more intact than the convenience of a fixture is worth. A pure
/// function over a `serde_json::Value` is fed exactly the bytes a tampered record
/// would parse to, with nothing else changed.
fn run_facts_from_value(value: &serde_json::Value) -> Option<RunFacts> {
    let string_field = |key: &str| -> String {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string()
    };

    // Non-zero and in range, or no observable run at all. `as u32` is
    // deliberately absent: the range check is the conversion.
    let pid_field = |key: &str| -> Option<u32> {
        let raw = value.get(key)?.as_u64()?;
        if raw == 0 || raw > u32::MAX as u64 {
            return None;
        }
        u32::try_from(raw).ok()
    };

    Some(RunFacts {
        ended_at: value
            .get("ended_at")
            .filter(|v| !v.is_null())
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        pid: pid_field("pid")?,
        pgid: pid_field("pgid")?,
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
/// A **crashed** run is returned exactly like a live one. It is as worth
/// surfacing as a live one: a run that died without an ending is the thing a user
/// most needs to be told about on restart, and suppressing it would leave the
/// evidence on disk and nothing on screen. So is a run whose liveness could not
/// be determined — for the opposite reason, that nothing is known about it.
pub fn reconcile_one(alias: &str, project_root: &Path) -> Option<ObservedRun> {
    let planning_dir = project_root.join(".planning");
    let run_id = writer::read_active_run(&planning_dir)?;
    // One `?`, and it is the WR-02 read side (D-27). The `active` file lives
    // inside the driven project, so the agent controls what this run id is; a
    // fallible `run_paths` is what stops a traversing id turning this scan into
    // a read of `run.json` from anywhere on the filesystem. Returning `None` is
    // the right refusal here rather than a log-and-continue: this function
    // already answers "there is no observable run" that way, and a project whose
    // pointer is hostile has none.
    let paths = run_paths(&planning_dir, &run_id)?;
    let facts = read_run_facts(&paths.dir)?;

    // The short-circuit is today's and is kept: a run already known to be over
    // needs no `/proc` read, and `classify` ignores the liveness on that arm
    // anyway. `Dead` rather than `Unknown` for the skipped probe, so the skip
    // cannot leak an undeterminable answer into a record that is not.
    let liveness = if facts.ended_at.is_some() {
        Liveness::Dead
    } else {
        liveness::probe(facts.pid, &run_id)
    };

    match classify(facts.ended_at.as_deref(), liveness) {
        RunVerdict::Ended => None,
        RunVerdict::Live | RunVerdict::CrashedWithoutEnding | RunVerdict::LivenessUnknown => {
            Some(ObservedRun {
                alias: alias.to_string(),
                run_id,
                pid: facts.pid,
                pgid: facts.pgid,
                started_at: facts.started_at,
                goal: facts.goal,
                gsd_command: facts.gsd_command,
                liveness,
            })
        }
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

/// The outcome label of each project's most recent **ended** run (WR-02, D-14).
///
/// The companion to [`reconcile_all`], and it exists because that function
/// cannot answer this: [`reconcile_one`] returns `None` for an ended run, so the
/// one fact D-14's third evidence source needs is precisely the one the scan
/// drops. Without a reader for it the needs-a-human badge could never light for
/// a failed, permission-denied, stalled or timed-out run — and a badge that can
/// never light is worse than no badge, because it teaches the user to ignore it.
///
/// A project with no ended run, or none at all, is **absent** rather than
/// present with a placeholder: the same authoritative-by-absence rule
/// `reconcile_all` follows.
///
/// **Still zero disk writes** (D-12): [`crate::journal::last_ended_outcome`]
/// delegates to `list_runs`, which is a `read_dir` and one small read per run.
/// Blocking work, so callers invoke it on `tokio::task::spawn_blocking` beside
/// the probe (D-28).
pub fn last_ended_outcomes(
    projects: &HashMap<String, RegisteredProject>,
) -> HashMap<String, String> {
    projects
        .iter()
        .filter_map(|(alias, project)| {
            crate::journal::last_ended_outcome(&project.path.join(".planning"))
                .map(|outcome| (alias.clone(), outcome))
        })
        .collect()
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
            classify(Some("2026-07-29T12:05:00Z"), Liveness::Dead),
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
        assert!(!observed.is_live(), "a dead pid is not live");
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

    #[test]
    fn an_undeterminable_liveness_is_never_classified_as_a_crash() {
        // All six combinations, so the table is pinned rather than sampled.
        assert_eq!(classify(None, Liveness::Alive), RunVerdict::Live);
        assert_eq!(
            classify(None, Liveness::Dead),
            RunVerdict::CrashedWithoutEnding
        );
        assert_eq!(
            classify(None, Liveness::Unknown),
            RunVerdict::LivenessUnknown,
            "an undeterminable liveness must NOT be classified as a crash. \
             CrashedWithoutEnding is what the dashboard renders as 'your run \
             died', and on a platform where the /proc probe does not apply that \
             sentence would be printed about every healthy run, on every scan, \
             for as long as the run lasted (CR-05, D-05)"
        );

        // An ended record wins over every liveness answer, including Unknown.
        for liveness in [Liveness::Alive, Liveness::Dead, Liveness::Unknown] {
            assert_eq!(
                classify(Some("2026-07-29T12:05:00Z"), liveness),
                RunVerdict::Ended,
                "a record that reached its terminal write is over regardless of \
                 what /proc can say about the pid that wrote it, got {liveness:?}"
            );
        }
    }

    #[test]
    fn a_record_whose_pid_is_zero_or_truncated_yields_no_observable_run() {
        // The end-to-end half: a zero pid IS representable in a `RunRecord`, so
        // it goes in through the production writer exactly like every other
        // fixture in this module.
        let root = tempfile::TempDir::new().expect("temp dir");
        fabricate_run(root.path(), RUN_ID, 0, None);
        assert_eq!(
            reconcile_one("demo", root.path()),
            None,
            "a record carrying pid 0 must yield NO observable run. Zero reaching \
             `signal_group` means kill(0, sig), which signals the caller's own \
             process group — under a TUI that is the user's entire terminal \
             session (T-17-08-08)"
        );

        // The other two shapes are not representable in a `RunRecord` — `pid` is
        // a `u32` — so they are fed to the value parser directly rather than
        // written to disk, which would put a write verb in this module. See
        // `run_facts_from_value`.
        let base = serde_json::json!({
            "run_id": RUN_ID,
            "started_at": "2026-07-29T12:00:00Z",
            "goal": "ship the thing",
            "gsd_command": "/gsd-progress",
            "pid": 4242,
            "pgid": 4242,
        });
        assert!(
            run_facts_from_value(&base).is_some(),
            "the control arm: a well-formed record must still parse, or the three \
             refusals below would pass by rejecting everything"
        );

        let mut absent = base.clone();
        absent.as_object_mut().expect("object").remove("pid");
        assert_eq!(
            run_facts_from_value(&absent),
            None,
            "an absent pid must yield no facts. `unwrap_or(0)` turned exactly this \
             into the most dangerous value in the codebase"
        );

        let mut truncated = base.clone();
        truncated["pid"] = serde_json::json!(u32::MAX as u64 + 1);
        assert_eq!(
            run_facts_from_value(&truncated),
            None,
            "a pid past u32::MAX must yield no facts. `as u32` TRUNCATED it — \
             4294967297 became 1, which is init"
        );

        let mut zero_group = base.clone();
        zero_group["pgid"] = serde_json::json!(0);
        assert_eq!(
            run_facts_from_value(&zero_group),
            None,
            "the pgid is clamped exactly like the pid: it is the value a stop \
             signals, so a zero there is the more dangerous of the two"
        );
    }

    /// WR-02: the outcome of a run that ENDED is read off disk, which
    /// `reconcile_all` cannot do by construction.
    ///
    /// The two functions are complementary rather than redundant, and this test
    /// asserts the complement in both directions: `reconcile_all` sees nothing
    /// (the run is over, so there is nothing to observe) while
    /// `last_ended_outcomes` sees the outcome. Before it had a reader, D-14's
    /// finished-run arm could never fire.
    #[test]
    fn a_run_that_ended_reports_its_outcome_even_though_it_is_no_longer_observed() {
        let root = tempfile::TempDir::new().expect("temp dir");
        fabricate_run(root.path(), RUN_ID, DEAD_PID, Some("2026-07-29T12:05:00Z"));
        let projects = registry("demo", root.path());

        assert!(
            reconcile_all(&projects).is_empty(),
            "the run is over, so there is nothing left to observe — which is \
             exactly why the outcome needs a second reader"
        );

        let outcomes = last_ended_outcomes(&projects);
        assert_eq!(
            outcomes.get("demo").map(String::as_str),
            Some("completed"),
            "the label `run.json` recorded, read verbatim rather than derived"
        );
    }

    /// A project with a live or crashed run — no `ended_at` — reports nothing.
    ///
    /// **Absent rather than present with a placeholder**: an outcome is a fact
    /// about a run that finished, and manufacturing one for a run that has not
    /// is the CR-05 defect in a new place.
    #[test]
    fn a_project_with_no_ended_run_reports_no_outcome() {
        let root = tempfile::TempDir::new().expect("temp dir");
        fabricate_run(root.path(), RUN_ID, DEAD_PID, None);
        assert!(last_ended_outcomes(&registry("demo", root.path())).is_empty());

        // And one that was never driven at all.
        let empty = tempfile::TempDir::new().expect("temp dir");
        assert!(last_ended_outcomes(&registry("demo", empty.path())).is_empty());
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
