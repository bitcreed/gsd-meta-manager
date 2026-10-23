//! Workstreams data layer (GSD 1.8.0).
//!
//! GSD 1.8.0 supports parallel workstreams. Each lives under
//! `.planning/workstreams/<ws>/` and holds its own `ROADMAP.md`, `STATE.md`,
//! `phases/`, and `milestones/` — exactly the shape [`super::parse_project_state`]
//! already parses. The active workstream is named in `.planning/active-workstream`
//! (the file contains the workstream name, e.g. `alpha\n`).
//!
//! Recursion note: a workstream dir is itself a planning-shaped directory, so
//! parsing one via [`super::parse_project_state`] could in principle recurse into
//! another `workstreams/` layer. [`super::parse_project_state`] carries a
//! recursion guard (added in plan 8 task 2) that skips workstream loading when the
//! dir is already inside a `workstreams/` parent, bounding depth to one level.

use super::phase_num::PhaseNum;
use chrono::{DateTime, Duration, Utc};
use std::io::Read;
use std::path::Path;

/// How long a [`MilestoneClaim`] stays live without a heartbeat: GSD's own
/// `MILESTONE_LOCK_TTL_MS` (`gsd-core/bin/lib/milestone-lock.cjs`), four hours.
///
/// Mirrored rather than invented: the claim is GSD's, so the window in which GSD
/// itself calls it live is the only defensible one. An abandoned claim then
/// expires here exactly when it expires there.
pub const MILESTONE_CLAIM_TTL: Duration = Duration::hours(4);

/// The most of a `milestone.lock` this reader will take. GSD writes ~150 bytes;
/// the lock is third-party input and its size is unbounded by construction.
const MILESTONE_LOCK_READ_CAP: u64 = 64 * 1024;

/// A `<planning>/milestone.lock` claim: which phase a GSD session has claimed.
///
/// GSD keeps ONE `## Current Position` slot per planning directory
/// (`STATE.md`'s `current_phase`), and two sessions running different phases of
/// the same milestone overwrite each other's value. The lock is GSD's advisory
/// answer to that (gsd-core issue #3311): a `{phase, session, pid, updated_at}`
/// claim, heartbeated by the session that holds it and workstream-scoped because
/// each workstream is its own planning directory. In a milestone that runs
/// phases in parallel it is the one STRUCTURED statement of which phase is being
/// worked, where `STATE.md`'s `current_phase` is whichever session wrote last.
///
/// Only the two fields this crate needs are kept, and both are typed: the
/// `session` string and `pid` are neither read nor held, so no free `String`
/// from the repository enters the model.
#[derive(Debug, Clone, PartialEq)]
pub struct MilestoneClaim {
    /// The claimed phase, when the lock names a parseable one.
    pub phase: Option<PhaseNum>,
    /// The last heartbeat, when the lock carries a parseable `updated_at`
    /// (epoch milliseconds, as GSD writes it).
    pub updated_at: Option<DateTime<Utc>>,
}

/// Read `<planning_dir>/milestone.lock`. `None` when it is absent, unreadable,
/// not JSON, or not an object — never an error: a claim is a hint, and a
/// project without one is the common case.
fn read_milestone_claim(planning_dir: &Path) -> Option<MilestoneClaim> {
    let mut text = String::new();
    std::fs::File::open(planning_dir.join("milestone.lock"))
        .ok()?
        .take(MILESTONE_LOCK_READ_CAP)
        .read_to_string(&mut text)
        .ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    let object = value.as_object()?;
    let phase = object.get("phase").and_then(super::handoff_phase);
    let updated_at = object
        .get("updated_at")
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .and_then(DateTime::<Utc>::from_timestamp_millis);
    Some(MilestoneClaim { phase, updated_at })
}

/// One workstream discovered under `.planning/workstreams/`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorkstreamState {
    /// The workstream directory name (e.g. `alpha`).
    pub name: String,
    /// True when this workstream is named in `.planning/active-workstream`.
    pub active: bool,
    /// The parsed project state of the workstream's own planning directory.
    pub state: super::ProjectState,
    /// The workstream's own `milestone.lock`, when it has one.
    pub claim: Option<MilestoneClaim>,
}

impl WorkstreamState {
    /// The phase a LIVE claim names, as of `now`.
    ///
    /// Live is GSD's own rule: the heartbeat is younger than
    /// [`MILESTONE_CLAIM_TTL`]. A heartbeat further in the future than that is
    /// not credible either (a clock a day ahead would otherwise pin a claim
    /// live forever), so the age is bounded on both sides. The phase must also
    /// be one this workstream's roadmap lists — a claim naming a phase nobody
    /// can see would only ever draw `P<n>: Unknown`.
    pub fn live_claim_phase(&self, now: DateTime<Utc>) -> Option<PhaseNum> {
        let claim = self.claim.as_ref()?;
        let phase = claim.phase.as_ref()?;
        let age = now.signed_duration_since(claim.updated_at?);
        if age >= MILESTONE_CLAIM_TTL || age <= -MILESTONE_CLAIM_TTL {
            return None;
        }
        self.state
            .phases
            .iter()
            .any(|p| PhaseNum::parse(&p.number).as_ref() == Some(phase))
            .then(|| phase.clone())
    }

    /// The phase this workstream is working on, for display.
    ///
    /// **A live claim first, then [`ProjectState::active_phase_number`].** A
    /// workstream is the unit GSD uses to run a milestone's phases in PARALLEL,
    /// which is exactly where the fallback is weakest: `STATE.md`'s
    /// `current_phase` is one slot shared by every session (sentriq's v0.14 says
    /// 24 while its owner works 26), and the disk frontier is the first phase
    /// that is not yet executed — here 27, which has no directory at all. The
    /// claim is the one input that says which phase a session is working on, so
    /// it leads while it is live and the established rule answers when it is not.
    ///
    /// `None` only when the workstream's roadmap lists no phase, so there is
    /// nothing to name.
    ///
    /// [`ProjectState::active_phase_number`]: super::ProjectState::active_phase_number
    pub fn focus_phase_number(&self, now: DateTime<Utc>) -> Option<PhaseNum> {
        if self.state.phases.is_empty() {
            return None;
        }
        Some(
            self.live_claim_phase(now)
                .unwrap_or_else(|| self.state.active_phase_number()),
        )
    }

    /// The roadmap entry [`Self::focus_phase_number`] names, when the roadmap
    /// lists it.
    pub fn focus_phase(&self, now: DateTime<Utc>) -> Option<&super::roadmap_md::RoadmapPhase> {
        let focus = self.focus_phase_number(now)?;
        self.state
            .phases
            .iter()
            .find(|p| PhaseNum::parse(&p.number).as_ref() == Some(&focus))
    }

    /// The disk inference of the focus phase, which is what the dashboard's
    /// D-R-P-E-V pipeline cell draws for it.
    pub fn focus_inference(
        &self,
        now: DateTime<Utc>,
    ) -> Option<&super::disk_status::DiskInference> {
        let phase = self.focus_phase(now)?;
        self.state.phase_disk_statuses.get(&phase.number)
    }
}

/// The identity key of one phase across the project's own roadmap AND its
/// parallel workstreams' roadmaps.
///
/// A phase number is only unique inside ONE roadmap: every workstream numbers
/// its phases itself, and sentriq's root and its `v0-13-public-beta` workstream
/// both have a Phase 16. So a phase is identified by where it lives as well as
/// by its number: the project's own phases keep the bare [`phase_key`] (every
/// key that existed before workstreams were listed is unchanged), and a
/// workstream's phase is `<workstream dir>/<phase key>`.
///
/// The `/` cannot occur in a directory name and is never part of a numeric
/// phase id, so a scoped key is never mistaken for a bare one — nor changed by
/// a second trip through [`phase_key`], which leaves a non-numeric id as it is.
pub fn scoped_phase_key(workstream: Option<&str>, number: &str) -> String {
    let key = super::phase_num::phase_key(number);
    match workstream {
        None => key,
        Some(name) => format!("{name}/{key}"),
    }
}

/// One phase the Detail screen's Roadmap and Phases tabs can list: a phase of
/// the project's own `.planning/` or of one of its parallel workstreams.
///
/// `owner` is the state the phase must be read against — the project's or the
/// workstream's — because the per-phase evidence (`phase_disk_statuses`, goals,
/// the active phase, the external-job flag) lives on the state whose directory
/// holds the phase, and a workstream's numbers mean nothing to the project's.
#[derive(Debug, Clone, Copy)]
pub struct ScopedPhase<'a> {
    /// The workstream the phase belongs to; `None` for the project's own.
    pub workstream: Option<&'a WorkstreamState>,
    /// The state that owns the phase.
    pub owner: &'a super::ProjectState,
    /// The roadmap entry.
    pub phase: &'a super::roadmap_md::RoadmapPhase,
}

impl ScopedPhase<'_> {
    /// The phase's identity key — see [`scoped_phase_key`].
    pub fn key(&self) -> String {
        scoped_phase_key(self.workstream_name(), &self.phase.number)
    }

    /// The owning workstream's directory name, `None` for the project's own.
    pub fn workstream_name(&self) -> Option<&str> {
        self.workstream.map(|ws| ws.name.as_str())
    }
}

/// Load every workstream under `<planning_dir>/workstreams/`.
///
/// Returns one [`WorkstreamState`] per subdirectory, each parsed with
/// [`super::parse_project_state`]. The workstream named in
/// `<planning_dir>/active-workstream` (trimmed) is marked `active`; when that file
/// is absent no workstream is active. Results are sorted by name for stability.
///
/// Never panics: a missing or unreadable `workstreams/` directory yields an empty
/// `Vec`.
pub fn load_workstreams(planning_dir: &Path) -> Vec<WorkstreamState> {
    let ws_root = planning_dir.join("workstreams");

    let active_name = std::fs::read_to_string(planning_dir.join("active-workstream"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let entries = match std::fs::read_dir(&ws_root) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut workstreams: Vec<WorkstreamState> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            let active = active_name.as_deref() == Some(name.as_str());
            let state = super::parse_project_state(&e.path());
            let claim = read_milestone_claim(&e.path());
            Some(WorkstreamState {
                name,
                active,
                state,
                claim,
            })
        })
        .collect();

    workstreams.sort_by(|a, b| a.name.cmp(&b.name));
    workstreams
}

/// Write a `.planning/` tree shaped like the real `sentriq` project's, which is
/// the layout the workstream surfacing was found broken on (debug session
/// `workstreams-not-surfaced`).
///
/// **What makes it sentriq-shaped, and why each piece is here:**
///
/// - The ROOT is an older milestone (`v0.12`, `phase_complete`) whose phases top
///   out at 16, with a *long* current-phase label (54 cells). The dashboard's
///   Phase column is a percentage of the terminal, so that label alone is wider
///   than the column at ordinary widths — anything appended after it is clipped.
/// - `.planning/workstreams/` holds two DASHED directories, one per parallel
///   milestone, and NO `.planning/active-workstream` (GSD 1.18 keeps the active
///   pointer per session in a temp dir, so on disk there usually is none).
/// - `v0-14-voice-assistant` carries GSD's single-slot hazards: STATE.md
///   declares `current_phase: 24` (stale — parallel sessions clobber it), the
///   disk frontier lands on phase 27 (which has NO directory), and a live
///   `milestone.lock` claims phase 26. Its `stopped_at` is a long quoted string
///   holding `;` `(` `)` and `,`. Phase 26 verified `gaps_found`; phase 28 is
///   partly built; `phases/` also holds an `archived-…-phases` directory, and
///   `quick/` and `milestones/archived-…-phases/` exist beside it. Phase 26 holds
///   fourteen plans (`26-01`..`26-14`).
/// - `v0-13-public-beta` has a stale HANDOFF.json and every phase executed
///   with `human_needed` verification.
///
/// `now` stamps the lock claims, so the fixture is live whenever the test runs:
/// the v0.14 claim is 53 minutes old (inside GSD's 4-hour window), the root's is
/// two days old (long expired).
#[cfg(test)]
pub(crate) fn write_sentriq_workstreams_fixture(
    planning: &Path,
    now: chrono::DateTime<chrono::Utc>,
) {
    use std::fs;

    fn put(path: std::path::PathBuf, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    /// One phase directory: `plans` PLAN files, `summaries` SUMMARY files and an
    /// optional VERIFICATION status.
    fn phase_dir(base: &Path, dir: &str, plans: u32, summaries: u32, verification: Option<&str>) {
        let num = dir.split('-').next().unwrap();
        for n in 1..=plans {
            put(
                base.join(dir).join(format!("{num}-{n:02}-PLAN.md")),
                "# plan\n",
            );
        }
        for n in 1..=summaries {
            put(
                base.join(dir).join(format!("{num}-{n:02}-SUMMARY.md")),
                "# summary\n",
            );
        }
        if let Some(status) = verification {
            put(
                base.join(dir).join(format!("{num}-VERIFICATION.md")),
                &format!("---\nstatus: {status}\n---\n"),
            );
        }
    }

    let lock = |phase: &str, age: chrono::Duration| {
        format!(
            "{{\n  \"phase\": \"{phase}\",\n  \"session\": \"claude-code-session-id-0000\",\n  \
             \"pid\": 4242,\n  \"updated_at\": {}\n}}\n",
            (now - age).timestamp_millis()
        )
    };

    // ── root: the older v0.12 milestone ─────────────────────────────────────
    put(planning.join("config.json"), "{\"commit_docs\": true}\n");
    put(
        planning.join("STATE.md"),
        "---\ngsd_state_version: \"1.0\"\nmilestone: v0.12\nmilestone_name: Actuation Routines\n\
         current_phase: 15\n\
         current_phase_name: Ram Catalog Procedures (experimental, from AlfaOBD RE)\n\
         status: phase_complete\nlast_updated: \"2026-10-21T03:03:11.000Z\"\n\
         progress:\n  total_phases: 9\n  completed_phases: 2\n  total_plans: 44\n  \
         completed_plans: 44\n---\n\n# Project State\n",
    );
    put(
        planning.join("ROADMAP.md"),
        "# Roadmap: Sentriq\n\n## v0.12 — Actuation Routines\n\n## Phases\n\n\
         - [x] **Phase 15: Ram Catalog Procedures (experimental, from AlfaOBD RE)** - catalog\n\
         - [ ] **Phase 16: Raw-CAN Button Replay PoC** - replay\n\n\
         ## Phase Details\n\n### Phase 15: Ram Catalog Procedures (experimental, from AlfaOBD RE)\n\
         **Goal**: catalog\n\n### Phase 16: Raw-CAN Button Replay PoC\n**Goal**: replay\n",
    );
    put(
        planning.join("milestone.lock"),
        &lock("15", chrono::Duration::hours(48)),
    );
    phase_dir(
        &planning.join("phases"),
        "15-ram-catalog-procedures",
        2,
        2,
        Some("passed"),
    );
    phase_dir(
        &planning.join("phases"),
        "16-raw-can-button-replay-poc",
        1,
        0,
        None,
    );

    // ── workstream v0-13-public-beta: every phase executed, awaiting a human ─
    let ws13 = planning.join("workstreams").join("v0-13-public-beta");
    put(
        ws13.join("STATE.md"),
        "---\ngsd_state_version: \"1.0\"\nmilestone: v0.13\nmilestone_name: Public Beta\n\
         current_phase: 20\ncurrent_phase_name: Compatibility Check\ncurrent_plan: N/A\n\
         status: human_needed\nlast_updated: \"2026-10-13T19:32:00.000Z\"\n\
         progress:\n  total_phases: 7\n  completed_phases: 0\n  total_plans: 25\n  \
         completed_plans: 25\n---\n\n# Project State — v0.13 Public Beta (workstream)\n",
    );
    put(
        ws13.join("ROADMAP.md"),
        "# Roadmap: Sentriq\n\n## v0.13 — Public Beta\n\n## Phases\n\n\
         - [ ] **Phase 16: Beta Build Channel** - build\n\
         - [ ] **Phase 17: Mandatory Upload Pipeline** - upload\n\
         - [ ] **Phase 18: BLE Adapters** - ble\n\
         - [ ] **Phase 19: First-Run Onboarding and Consent** - onboarding\n\
         - [ ] **Phase 20: Compatibility Check** - compat\n\
         - [ ] **Phase 21: Mandatory Data Flows and Feedback** - flows\n\
         - [ ] **Phase 22: Play Release Readiness** - release\n",
    );
    put(ws13.join("REQUIREMENTS.md"), "# Requirements\n");
    put(ws13.join("MILESTONE-CONTEXT.md"), "# Milestone context\n");
    put(
        ws13.join("HANDOFF.json"),
        "{\n  \"paused_at\": \"2026-10-13\",\n  \"workstream\": \"v0-13-public-beta\",\n  \
         \"milestone\": \"v0.13\",\n  \"phases\": {\"16\": {\"status\": \"executed_verify_pending\"}}\n}\n",
    );
    for (dir, plans) in [
        ("16-beta-build-channel", 3),
        ("17-mandatory-upload-pipeline", 3),
        ("18-ble-adapters", 2),
        ("19-first-run-onboarding-and-consent", 2),
        ("20-compatibility-check", 2),
        ("21-mandatory-data-flows-and-feedback", 2),
        ("22-play-release-readiness", 2),
    ] {
        phase_dir(
            &ws13.join("phases"),
            dir,
            plans,
            plans,
            Some("human_needed"),
        );
    }

    // ── workstream v0-14-voice-assistant: parallel phases, one live claim ───
    let ws14 = planning.join("workstreams").join("v0-14-voice-assistant");
    put(
        ws14.join("STATE.md"),
        "---\n\
         gsd_state_version: \"1.0\"\n\
         milestone: v0.14\n\
         milestone_name: Voice Mechanic\n\
         current_phase: 24\n\
         current_phase_name: Live Read-Only Queries and Online Lookup\n\
         current_plan: 1\n\
         status: executing\n\
         stopped_at: \"Phase 26 gap plans 26-11..26-14 planned (CR-01, WR-01..06, UI fixes 1-3; \
         2 waves), gap execution next; Phase 28 planned (9 plans, 6 waves; integration \
         28-06..28-09; executes after 24-07, 25-04 and 26 integration); Phase 24 gap plans \
         24-09..24-11 merged, re-verified human_needed (3 owner-only UAT items in \
         24-UAT.md); Phase 26 executed, verified gaps_found (CR-01 + WR-01..06, gap round \
         pending); Phase 26 planned (10 plans); Phase 28 added and context gathered \
         (28-CONTEXT.md); Phase 24 planned (8 plans); Phase 25 executed + verified \
         human_needed (2 owner risk sign-offs in 25-UAT.md: WR-02, T-25-23; no gaps); \
         quick 261022-isv (WR-07) done; phase verification next\"\n\
         last_updated: \"2026-10-23T01:54:13.298Z\"\n\
         last_activity: 2026-10-22\n\
         last_activity_desc: Phase 24 execution started\n\
         progress:\n  total_phases: 6\n  completed_phases: 0\n  total_plans: 49\n  \
         completed_plans: 12\n  percent: 0\n\
         workstream: v0-14-voice-assistant\n\
         created: 2026-10-22\n\
         ---\n\n# Project State — v0.14 Voice Mechanic (workstream)\n",
    );
    put(
        ws14.join("ROADMAP.md"),
        "# Roadmap: Sentriq\n\n## v0.14 — Voice Mechanic\n\n\
         In progress: Phases 23-27, workstream `v0-14-voice-assistant`, parallel with v0.12 \
         and v0.13.\n\n## Phases\n\n\
         - [ ] **Phase 23: Mechanic Mode Foundation and Cost Guardrails** - the ONE switch\n\
         - [ ] **Phase 24: Live Read-Only Queries and Online Lookup** - live reads\n\
         - [ ] **Phase 25: Conversation Memory** - memory\n\
         - [ ] **Phase 26: Push-to-Talk Voice** - PRIMARY: on-device STT and TTS\n\
         - [ ] **Phase 27: Parity, Designs and Wake-Word Verdict** - parity\n\
         - [ ] **Phase 28: Mechanic Toolbox, Full Data Access and Voice-Started Actions** - toolbox\n\n\
         ## Phase Details\n\n\
         ### Phase 26: Push-to-Talk Voice\n**Goal**: talk to Sentriq like a mechanic\n",
    );
    put(ws14.join("REQUIREMENTS.md"), "# Requirements\n");
    put(ws14.join("MILESTONE-CONTEXT.md"), "# Milestone context\n");
    put(
        ws14.join("config.json"),
        "{\"workflow\": {\"use_worktrees\": true}}\n",
    );
    put(
        ws14.join("milestone.lock"),
        &lock("26", chrono::Duration::minutes(53)),
    );
    put(ws14.join("state.json"), "{\"contract\": \"1.0.0\"}\n");
    let phases = ws14.join("phases");
    phase_dir(
        &phases,
        "23-mechanic-mode-foundation-and-cost-guardrails",
        3,
        3,
        Some("human_needed"),
    );
    phase_dir(
        &phases,
        "24-live-read-only-queries-and-online-lookup",
        3,
        3,
        Some("human_needed"),
    );
    phase_dir(
        &phases,
        "25-conversation-memory",
        2,
        2,
        Some("human_needed"),
    );
    // Phase 26 holds FOURTEEN plans (26-01..26-14: ten planned, then four
    // gap-closure plans), all summarised, verified `gaps_found` — the phase the
    // owner is working, and the one the Detail screen's Phases tab must list
    // plan by plan.
    phase_dir(&phases, "26-push-to-talk-voice", 14, 14, Some("gaps_found"));
    // Phase 27 has NO directory: it is the disk frontier.
    phase_dir(
        &phases,
        "28-mechanic-toolbox-full-data-access-and-voice-started-actions",
        3,
        1,
        None,
    );
    // The two `archived-…-phases` directories GSD leaves behind: one inside
    // `phases/` (where it must not be mistaken for a phase) and the empty one
    // under `milestones/`.
    fs::create_dir_all(phases.join("archived-20261022-phases")).unwrap();
    fs::create_dir_all(ws14.join("milestones").join("archived-20261022-phases")).unwrap();
    for quick in [
        "261022-isv-fix-wr-07-stream-idle-timeout-and-re-ena",
        "261022-srz-voice-screen-try-again-re-runs-failed-me",
    ] {
        put(ws14.join("quick").join(quick).join("PLAN.md"), "# plan\n");
        put(
            ws14.join("quick").join(quick).join("SUMMARY.md"),
            "# summary\n",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Build a `.planning/` dir with the given files (relative paths under
    /// `.planning/`). Returns the TempDir (keep it alive).
    fn make_planning(files: &[(&str, &str)]) -> TempDir {
        let td = TempDir::new().unwrap();
        let planning = td.path().join(".planning");
        fs::create_dir_all(&planning).unwrap();
        for (rel, content) in files {
            let path = planning.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(path, content).unwrap();
        }
        td
    }

    #[test]
    fn test_two_workstreams_both_returned() {
        let td = make_planning(&[
            (
                "workstreams/alpha/STATE.md",
                "---\nstatus: executing\ncurrent_phase: 1\n---\n",
            ),
            ("workstreams/alpha/ROADMAP.md", "# Roadmap\n"),
            (
                "workstreams/beta/STATE.md",
                "---\nstatus: planning\ncurrent_phase: 2\n---\n",
            ),
            ("workstreams/beta/ROADMAP.md", "# Roadmap\n"),
        ]);
        let wss = load_workstreams(&td.path().join(".planning"));
        assert_eq!(wss.len(), 2);
        // Sorted by name.
        assert_eq!(wss[0].name, "alpha");
        assert_eq!(wss[1].name, "beta");
        // Each carries its own parsed state.
        assert_eq!(wss[0].state.status, "executing");
        assert_eq!(wss[1].state.status, "planning");
    }

    #[test]
    fn test_active_workstream_flag_set_from_pointer() {
        let td = make_planning(&[
            (
                "workstreams/alpha/STATE.md",
                "---\nstatus: executing\n---\n",
            ),
            ("workstreams/beta/STATE.md", "---\nstatus: planning\n---\n"),
            ("active-workstream", "alpha\n"),
        ]);
        let wss = load_workstreams(&td.path().join(".planning"));
        assert_eq!(wss.len(), 2);
        assert!(wss[0].active, "alpha should be active");
        assert!(!wss[1].active, "beta should be inactive");
    }

    #[test]
    fn test_no_active_pointer_means_none_active() {
        let td = make_planning(&[
            (
                "workstreams/alpha/STATE.md",
                "---\nstatus: executing\n---\n",
            ),
            ("workstreams/beta/STATE.md", "---\nstatus: planning\n---\n"),
        ]);
        let wss = load_workstreams(&td.path().join(".planning"));
        assert!(wss.iter().all(|w| !w.active));
    }

    #[test]
    fn test_no_workstreams_dir_returns_empty() {
        let td = make_planning(&[("STATE.md", "---\nstatus: executing\n---\n")]);
        let wss = load_workstreams(&td.path().join(".planning"));
        assert!(wss.is_empty());
    }

    // ── debug session `workstreams-not-surfaced`: sentriq's layout ───────────

    use crate::state_reader::parse_project_state;
    use chrono::{DateTime, Duration, Utc};

    /// The sentriq-shaped tree, parsed. Returns the dir (keep it alive), the
    /// root state and the `now` the fixture's claims were stamped against.
    fn sentriq() -> (TempDir, super::super::ProjectState, DateTime<Utc>) {
        let td = TempDir::new().unwrap();
        let planning = td.path().join(".planning");
        let now = Utc::now();
        write_sentriq_workstreams_fixture(&planning, now);
        let state = parse_project_state(&planning);
        (td, state, now)
    }

    fn ws<'a>(state: &'a super::super::ProjectState, name: &str) -> &'a WorkstreamState {
        state
            .workstreams
            .iter()
            .find(|w| w.name == name)
            .unwrap_or_else(|| panic!("workstream {name} was not loaded"))
    }

    /// The layers BELOW the surfacing were never broken, and this pins that so a
    /// future "fix" does not go looking for the defect there: both dashed
    /// directories are detected, each STATE.md parses — the ~1 KB quoted
    /// `stopped_at` full of `;` `(` `)` `,` included — and the archived-phases
    /// directories are not mistaken for phases.
    #[test]
    fn sentriq_layout_detection_and_parsing_are_sound() {
        let (_td, root, _) = sentriq();

        assert_eq!(root.milestone, "v0.12");
        assert_eq!(root.workstreams.len(), 2);
        assert_eq!(
            root.workstreams
                .iter()
                .map(|w| w.name.as_str())
                .collect::<Vec<_>>(),
            ["v0-13-public-beta", "v0-14-voice-assistant"]
        );
        // No `.planning/active-workstream`: neither is marked active.
        assert!(root.workstreams.iter().all(|w| !w.active));

        let v14 = ws(&root, "v0-14-voice-assistant");
        assert!(
            !v14.state.state_md_unreadable,
            "{:?}",
            v14.state.state_md_fault
        );
        assert!(!v14.state.state_md_recovered);
        assert_eq!(v14.state.status, "executing");
        assert_eq!(v14.state.milestone, "v0.14");
        assert_eq!(
            v14.state
                .phases
                .iter()
                .map(|p| p.number.as_str())
                .collect::<Vec<_>>(),
            ["23", "24", "25", "26", "27", "28"]
        );
        // Phase 27 has no directory; the archived dir is not a phase.
        assert!(!v14
            .state
            .phase_disk_statuses
            .contains_key("archived-20261022-phases"));

        let v13 = ws(&root, "v0-13-public-beta");
        assert_eq!(v13.state.status, "human_needed");
        assert_eq!(v13.state.milestone, "v0.13");
        assert_eq!(v13.state.phases.len(), 7);
    }

    /// The model must say 26 for v0.14 — the phase the owner is working — and
    /// the two inputs it used to rely on must be shown to disagree with it, so
    /// this test fails if the fixture ever stops modelling the hazard.
    #[test]
    fn the_v014_focus_is_the_live_claim_26_not_declared_24_nor_frontier_27() {
        let (_td, root, now) = sentriq();
        let v14 = ws(&root, "v0-14-voice-assistant");

        // STATE.md's single slot is stale, and the frontier lands on a phase
        // with no directory — neither can be the phase being worked.
        assert_eq!(v14.state.state_md_phase_number, Some(PhaseNum::from(24)));
        assert_eq!(v14.state.active_phase_number(), PhaseNum::from(27));

        assert_eq!(v14.live_claim_phase(now), Some(PhaseNum::from(26)));
        assert_eq!(v14.focus_phase_number(now), Some(PhaseNum::from(26)));
        let phase = v14.focus_phase(now).expect("26 is on the roadmap");
        assert_eq!(phase.name, "Push-to-Talk Voice");
        // The pipeline the dashboard draws for it is phase 26's own.
        let inference = v14.focus_inference(now).expect("26 has a directory");
        assert_eq!(
            inference.status,
            crate::state_reader::disk_status::DiskStatus::Executed
        );
    }

    /// A workstream with no lock keeps the established rule, so v0.13 — every
    /// phase executed, STATE.md declaring 20 — still resolves to its last phase.
    #[test]
    fn a_workstream_without_a_claim_keeps_the_established_active_phase() {
        let (_td, root, now) = sentriq();
        let v13 = ws(&root, "v0-13-public-beta");

        assert!(v13.claim.is_none());
        assert_eq!(v13.live_claim_phase(now), None);
        assert_eq!(
            v13.focus_phase_number(now),
            Some(v13.state.active_phase_number())
        );
        assert_eq!(v13.focus_phase_number(now), Some(PhaseNum::from(22)));
        assert_eq!(v13.focus_phase(now).unwrap().name, "Play Release Readiness");
    }

    /// GSD's own window: a claim not heartbeated for four hours is abandoned, so
    /// the display falls back instead of pinning a dead session's phase forever.
    #[test]
    fn an_expired_claim_falls_back_to_the_established_active_phase() {
        let (_td, root, now) = sentriq();
        let v14 = ws(&root, "v0-14-voice-assistant");

        // 53 minutes old at `now`: live. Four hours on from the heartbeat: not.
        let just_inside = now + MILESTONE_CLAIM_TTL - Duration::minutes(54);
        assert_eq!(
            v14.focus_phase_number(just_inside),
            Some(PhaseNum::from(26))
        );
        let expired = now + MILESTONE_CLAIM_TTL;
        assert_eq!(v14.live_claim_phase(expired), None);
        assert_eq!(v14.focus_phase_number(expired), Some(PhaseNum::from(27)));
    }

    /// A heartbeat dated far in the future (a skewed clock, a hand-edited lock)
    /// must not read as eternally live.
    #[test]
    fn a_claim_dated_beyond_the_window_in_the_future_is_not_live() {
        let (_td, root, now) = sentriq();
        let v14 = ws(&root, "v0-14-voice-assistant");
        let long_before = now - MILESTONE_CLAIM_TTL - Duration::hours(1);
        assert_eq!(v14.live_claim_phase(long_before), None);
    }

    /// A claim for a phase the roadmap does not list would only ever draw
    /// `P<n>: Unknown`, so it is ignored rather than believed.
    #[test]
    fn a_claim_for_a_phase_the_roadmap_does_not_list_is_ignored() {
        let (td, _, now) = sentriq();
        let planning = td.path().join(".planning");
        fs::write(
            planning.join("workstreams/v0-14-voice-assistant/milestone.lock"),
            format!(
                "{{\"phase\": \"99\", \"session\": \"x\", \"pid\": 1, \"updated_at\": {}}}",
                now.timestamp_millis()
            ),
        )
        .unwrap();
        let root = parse_project_state(&planning);
        let v14 = ws(&root, "v0-14-voice-assistant");

        assert_eq!(v14.claim.as_ref().unwrap().phase, Some(PhaseNum::from(99)));
        assert_eq!(v14.live_claim_phase(now), None);
        assert_eq!(v14.focus_phase_number(now), Some(PhaseNum::from(27)));
    }

    /// The lock is third-party JSON: every shape GSD writes parses, and no
    /// other shape does anything but yield less.
    #[test]
    fn milestone_lock_parsing_is_tolerant() {
        let read = |text: &str| {
            let td = TempDir::new().unwrap();
            fs::write(td.path().join("milestone.lock"), text).unwrap();
            read_milestone_claim(td.path())
        };

        // A string phase, a numeric phase, a decimal phase.
        for (json, expected) in [
            (
                r#"{"phase":"26","updated_at":1792718191629}"#,
                Some(PhaseNum::from(26)),
            ),
            (
                r#"{"phase":26,"updated_at":1792718191629}"#,
                Some(PhaseNum::from(26)),
            ),
            (
                r#"{"phase":"2.5","updated_at":1792718191629}"#,
                PhaseNum::parse("2.5"),
            ),
            (
                r#"{"phase":"Phase 12","updated_at":1}"#,
                Some(PhaseNum::from(12)),
            ),
        ] {
            assert_eq!(read(json).expect(json).phase, expected, "{json}");
        }
        // The heartbeat is epoch milliseconds, integer or float.
        let at = DateTime::<Utc>::from_timestamp_millis(1_792_718_191_629).unwrap();
        assert_eq!(
            read(r#"{"phase":"26","updated_at":1792718191629}"#)
                .unwrap()
                .updated_at,
            Some(at)
        );
        assert_eq!(
            read(r#"{"phase":"26","updated_at":1792718191629.0}"#)
                .unwrap()
                .updated_at,
            Some(at)
        );
        // Missing or wrong-typed fields abstain individually.
        let partial = read(r#"{"phase":{"x":1},"updated_at":"soon"}"#).unwrap();
        assert_eq!((partial.phase, partial.updated_at), (None, None));
        // Not an object, not JSON, empty, absent: no claim at all.
        for junk in ["[1,2]", "26", "{not json", ""] {
            assert!(read(junk).is_none(), "{junk:?}");
        }
        assert!(read_milestone_claim(&TempDir::new().unwrap().path().join("nope")).is_none());
    }

    // ── debug session `workstreams-not-surfaced`: the Detail tabs' phase list ──

    /// The project's own phases keep the bare `phase_key`, so every key that
    /// existed before workstreams were listed is unchanged; a workstream's is
    /// behind its directory, and `phase_key` leaves the result alone.
    #[test]
    fn scoped_phase_keys_keep_the_projects_and_prefix_a_workstreams() {
        use crate::state_reader::phase_num::phase_key;
        assert_eq!(scoped_phase_key(None, "07"), "7");
        assert_eq!(scoped_phase_key(None, "07.1"), "7.1");
        assert_eq!(scoped_phase_key(Some("v0-13"), "16"), "v0-13/16");
        assert_eq!(scoped_phase_key(Some("v0-13"), "07.1"), "v0-13/7.1");
        // Numeric-looking workstream names do not turn the key numeric.
        for name in ["1", "0.1", "v0.14"] {
            let key = scoped_phase_key(Some(name), "2");
            assert_eq!(phase_key(&key), key, "{key}");
        }
    }

    /// `scoped_phases` is the project's phases, then each workstream's in name
    /// order, every one with a distinct key — v0.13's Phase 16 is not the
    /// project's. For a project without workstreams it is exactly `phases`.
    #[test]
    fn scoped_phases_list_the_project_then_each_workstream() {
        let (_td, root, _) = sentriq();
        let scoped = root.scoped_phases();
        assert_eq!(scoped.len(), 2 + 7 + 6);
        assert_eq!(scoped.len(), root.scoped_phase_count());

        let keys: Vec<String> = scoped.iter().map(ScopedPhase::key).collect();
        assert_eq!(
            keys.iter().collect::<std::collections::HashSet<_>>().len(),
            keys.len(),
            "{keys:?}"
        );
        assert_eq!(&keys[..2], ["15", "16"]);
        assert_eq!(keys[2], "v0-13-public-beta/16");
        assert_eq!(keys[8], "v0-13-public-beta/22");
        assert_eq!(keys[9], "v0-14-voice-assistant/23");
        assert_eq!(keys[14], "v0-14-voice-assistant/28");

        // Each is read against the state that owns it.
        assert!(scoped[0].workstream.is_none());
        assert_eq!(scoped[2].workstream_name(), Some("v0-13-public-beta"));
        assert_eq!(scoped[2].owner.milestone, "v0.13");
        assert_eq!(scoped[0].owner.milestone, "v0.12");
        assert!(scoped[2].owner.phase_disk_statuses.contains_key("16"));

        // A flat project: nothing changes.
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            (
                "ROADMAP.md",
                "# Roadmap\n\n## Phases\n\n- [ ] **Phase 1: A** - a\n- [ ] **Phase 2: B** - b\n",
            ),
        ]);
        let flat = parse_project_state(&td.path().join(".planning"));
        let numbers: Vec<&str> = flat
            .scoped_phases()
            .iter()
            .map(|p| p.phase.number.as_str())
            .collect();
        assert_eq!(numbers, ["1", "2"]);
        assert!(flat.scoped_phases().iter().all(|p| p.workstream.is_none()));
    }

    /// An oversized lock is read only up to the cap, so a hostile repository
    /// cannot make the reader slurp an arbitrary file.
    #[test]
    fn an_oversized_milestone_lock_is_not_read_past_its_cap() {
        let td = TempDir::new().unwrap();
        let padding = " ".repeat(MILESTONE_LOCK_READ_CAP as usize);
        fs::write(
            td.path().join("milestone.lock"),
            format!("{padding}{{\"phase\":\"26\",\"updated_at\":1}}"),
        )
        .unwrap();
        // The JSON begins past the cap, so what was read is only whitespace.
        assert!(read_milestone_claim(td.path()).is_none());
    }
}
