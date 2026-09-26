pub mod backlog;
pub mod config_json;
pub mod disk_status;
pub mod git_ops;
pub mod phase_num;
pub mod plan_waves;
pub mod queue_md;
pub mod roadmap_md;
pub mod state_md;
pub mod workstreams;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Whether a registered project is still on disk as a GSD project.
///
/// Registration checks the path exactly once (`registry::add_project`); after
/// that a folder can be moved or deleted under the dashboard. Without this, such
/// an entry parses to the same placeholder as a project whose `STATE.md` is
/// unreadable, and the two cannot be told apart.
///
/// **An enum, not a `String` or a `bool`.** It is a classification this crate
/// authors from two `is_dir()` stats, not text read from the repository, so it
/// must not widen `tests/spawn_seam_guard.rs`'s census of free `String` fields
/// on [`ProjectState`]; and it has three states, not two, because a folder
/// that exists without `.planning/` is a different fix for the user than a
/// folder that is gone.
///
/// Decided in [`parse_project_state`] only, never at render time: a state built
/// from [`ProjectState::default()`] is [`ProjectPresence::Present`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ProjectPresence {
    /// The project folder and its `.planning/` directory both exist.
    #[default]
    Present,
    /// The project folder exists but has no `.planning/` directory.
    NoPlanning,
    /// The project folder itself is gone (or is not a directory).
    FolderMissing,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProjectState {
    pub status: String,
    pub current_phase: String,
    /// ADR-2207: human-readable current phase name from STATE.md frontmatter
    /// (`current_phase_name`); empty when absent.
    pub current_phase_name: String,
    /// ADR-2207: current plan identifier from STATE.md frontmatter
    /// (`current_plan`); empty when absent.
    pub current_plan: String,
    pub total_phases: u32,
    pub completed_phases: u32,
    pub total_plans: u32,
    pub completed_plans: u32,
    pub milestone: String,
    pub backlog_count: u32,
    pub phases: Vec<roadmap_md::RoadmapPhase>,
    /// The milestones ROADMAP.md names, with their phase membership
    /// ([`roadmap_md::roadmap_milestones`]); drawn by the Roadmap graph.
    ///
    /// Their labels are `Untrusted`, which is why this is not a free-string
    /// field: third-party text reaches a cell only through `shown()`.
    pub milestones: Vec<roadmap_md::RoadmapMilestone>,
    /// The placeholder phases ROADMAP.md declares as `#### Build phase N
    /// (Milestone M): Title` headings
    /// ([`roadmap_md::parse_planned_build_phases`]); listed by the Roadmap
    /// after [`Self::phases`]. Display-only.
    ///
    /// **Deliberately NOT merged into `phases`.** GSD's own heading grammar
    /// does not count a `Build phase` heading, and the driver router and the
    /// frontier read `phases`: merging would let them target a phase GSD does
    /// not know exists.
    pub planned_phases: Vec<roadmap_md::RoadmapPhase>,
    /// The phases ROADMAP.md lists inside closed-milestone `<details>`
    /// collapses — the shipped history GSD's `complete-milestone` folds away
    /// ([`roadmap_md::parse_shipped_phases`]); drawn in the Roadmap's shipped
    /// bands. Display-only.
    ///
    /// **Deliberately NOT merged into `phases`.** GSD strips closed collapses
    /// before it enumerates phases, and an archived phase usually has no
    /// directory: in `phases` it would infer `NoDirectory`, take the
    /// current-phase cell, and become a legal driver goal target.
    pub shipped_phases: Vec<roadmap_md::RoadmapPhase>,
    /// Each phase's and build phase's `**Goal**:` line
    /// ([`roadmap_md::parse_phase_goals`]), keyed by `phase_key`; shown in the
    /// Roadmap's detail pane. Display-only.
    ///
    /// **`Untrusted` values in a map keyed by a reader-generated phase key**,
    /// so no free-string field joins `THIRD_PARTY_STRINGS`: a goal reaches a
    /// cell only through `shown()`, and it is deliberately kept out of the
    /// driver's model seam (widening the prose set would be a decision of its
    /// own).
    pub phase_goals: HashMap<String, crate::text::Untrusted>,
    /// STATE.md's `milestone_name`, e.g. `Actuation Routines`; `None` when the
    /// key is absent or empty. Names a milestone the roadmap itself does not
    /// list (sentriq's `v0.12`), so the Roadmap can title a synthetic band.
    ///
    /// `Untrusted`, not `String`, for the same reason as
    /// [`Self::milestones`]' labels: third-party text reaches a cell only
    /// through `shown()`, and the free-string census stays unchanged.
    pub milestone_name: Option<crate::text::Untrusted>,
    pub queued_actions: Vec<queue_md::QueuedAction>,
    /// Per-phase disk inference keyed by phase number (e.g., "01", "05")
    pub phase_disk_statuses: HashMap<String, disk_status::DiskInference>,
    /// Disk inference for the current/active phase: the first phase whose
    /// **implementation** is unfinished (status below
    /// [`disk_status::DiskStatus::Executed`]), or the last phase when every one
    /// of them is implemented.
    ///
    /// The threshold is implementation and not verification, deliberately — see
    /// the comment at the assignment site in [`parse_project_state`] (WR-04).
    pub current_phase_status: Option<disk_status::DiskInference>,
    /// The phase NUMBER that [`Self::current_phase_status`] describes.
    ///
    /// `DiskInference` carries what a phase's directory looks like but not
    /// which phase it is, so the frontier the loop already finds was
    /// unusable as an answer to "which phase is active?" — every caller
    /// re-derived that from `completed_phases + 1` instead, off a count taken
    /// from ROADMAP's `## Progress` table. That count lags: picsync's table
    /// said 2 complete while its phase 4 was already planned on disk, so the
    /// dashboard said P3 and the disk scanner, correctly, said 4.
    ///
    /// `None` only when the roadmap has no phases, or the frontier phase's id
    /// is not numeric (`M-2`). Decimal (inserted) phases such as `7.1` parse —
    /// see [`phase_num::PhaseNum`].
    pub current_phase_number: Option<phase_num::PhaseNum>,
    /// The numeric `current_phase` STATE.md declares, when it declares one.
    ///
    /// Kept separately from [`Self::current_phase`], which resolves to a
    /// human-readable *label* and so loses the number as soon as GSD also
    /// writes a `current_phase_name`. GSD advances it deliberately, so it is
    /// the one source that can say a phase is active before any artifact of it
    /// exists — [`Self::active_phase_number`] lets it lead the disk frontier,
    /// but clamps it up to that frontier rather than below it.
    pub state_md_phase_number: Option<phase_num::PhaseNum>,
    /// Whether the project has a non-empty HANDOFF.md or HANDOFF.json in
    /// .planning/ **that is not stale** (see [`Self::stale_handoff`]).
    pub paused: bool,
    /// Extracted context from HANDOFF file (next_action from JSON, or first
    /// content line from MD). `None` whenever the handoff is stale.
    pub pause_context: Option<String>,
    /// A non-empty HANDOFF that the parser judged **stale** and therefore
    /// ignored: [`Self::paused`] is `false` and [`Self::pause_context`] is
    /// `None` whenever this is `Some`.
    ///
    /// A handoff is stale when its phase is numerically behind STATE.md's
    /// declared `current_phase` ([`StaleHandoffReason::PhaseBehind`]), or when
    /// STATE.md's `last_updated` is more than an hour newer than the handoff's
    /// written time ([`StaleHandoffReason::StateNewer`]) — see
    /// [`handoff_staleness`]. GSD moved on after the handoff was written, so
    /// its `next_action` is an instruction about a past state; showing it as
    /// the project's current pause (ttbook: "Dispatch /gsd-discuss-phase 12"
    /// while executing phase 13) is worse than not showing it at all, so it is
    /// dropped rather than kept.
    ///
    /// Holds no `String` — phase numbers, a timestamp and an enum — so the
    /// free-string census in `tests/spawn_seam_guard.rs` stays unchanged.
    pub stale_handoff: Option<StaleHandoff>,
    /// True when `.planning/async-jobs/` holds at least one `*.json` manifest —
    /// the phase is legitimately waiting on an external job (not stuck).
    pub external_job_waiting: bool,
    /// Most recent activity timestamp (last commit, mtime fallback) for the project.
    pub last_activity: Option<chrono::DateTime<chrono::Utc>>,
    /// Filesystem root of the project (the directory containing `.planning/`).
    pub project_root: PathBuf,
    /// G10: a **non-empty** `.planning/.continue-here.md` at the project root.
    ///
    /// A hard stop whose only bypass is `--force` (`next.md:46-58`). The content
    /// check, rather than a bare existence check, follows the precedent
    /// [`detect_handoff`] already sets in this file: a file trimmed to nothing
    /// is a leftover, not a signal.
    pub continue_here_present: bool,
    /// G15: the phases carrying a `## Deferred Verification` row in STATE.md.
    ///
    /// Named rather than counted, because what a router or a human needs is
    /// *which* phase still owes a verification, not how many do.
    pub deferred_verification_phases: Vec<String>,
    /// GSD 1.8.0 parallel workstreams under `.planning/workstreams/<ws>/`.
    /// Empty for flat projects. Bounded to one level: a workstream's own
    /// sub-state carries an empty `workstreams` vec (recursion guard below).
    pub workstreams: Vec<workstreams::WorkstreamState>,
    /// `STATE.md` exists on disk but its frontmatter could not be read.
    ///
    /// **Distinct from "no STATE.md" on purpose.** Every field this reader
    /// takes from STATE.md is defaulted when the read fails, so an unreadable
    /// file and an absent one produce byte-identical state — and the dashboard
    /// then prints a confident, count-derived, wrong phase for a project whose
    /// real phase it simply could not read. A `warn!` to a log file nobody
    /// opens is what let that ship for every registered project at once; this
    /// flag is what lets the render path say so instead.
    pub state_md_unreadable: bool,
    /// Why the frontmatter could not be read, when [`Self::state_md_unreadable`].
    ///
    /// A classification this crate authored, never the parser's message — see
    /// [`state_md::FrontmatterFault`]. Every `String` on this struct is
    /// third-party text by construction (`src/driver/untrusted.rs` is the
    /// census); a parser error quoting a foreign STATE.md would have been one
    /// more, held for the sake of a line number.
    pub state_md_fault: Option<state_md::FrontmatterFault>,
    /// WHERE the frontmatter broke, when the parser could say — as FILE-relative
    /// 1-based numbers.
    ///
    /// This is the other half of the argument the comment above makes. That one
    /// says a parser message would have been third-party prose held "for the
    /// sake of a line number"; the line number is now held **without** the
    /// message, which is precisely the distinction being drawn. A `u32` pair
    /// cannot carry an instruction and cannot be text a foreign repository
    /// wrote.
    ///
    /// Deliberately **not** a `String`, for two independent reasons that
    /// converge: `src/driver/untrusted.rs` forbids unclassified third-party
    /// text crossing into the UI, and `tests/spawn_seam_guard.rs` censuses every
    /// `String`/`Option<String>`/`Vec<String>` field on this struct against
    /// `THIRD_PARTY_STRINGS` in both directions — so a stringly position would
    /// force the census wider, which is the property this field exists to keep
    /// intact.
    pub state_md_fault_position: Option<state_md::FrontmatterFaultPosition>,
    /// `STATE.md`'s frontmatter was read only after an **in-memory** repair
    /// pass ([`state_md::FrontmatterOutcome::Recovered`]).
    ///
    /// **Distinct from a clean read on purpose, and the file on disk is
    /// unchanged.** A value shown from a repaired document is a value this tool
    /// rewrote before believing; a strict reader would still refuse the file. A
    /// reader who cannot tell the two apart has been told something slightly
    /// false about the repository — the same argument
    /// [`crate::driver::untrusted`]'s truncation marker makes about a string,
    /// applied to a document.
    ///
    /// A `bool`, not a message: nothing the parser wrote may cross this
    /// boundary, and `tests/spawn_seam_guard.rs`'s census of every free
    /// `String` on this struct stays satisfied without widening it.
    pub state_md_recovered: bool,
    /// Whether the project folder and its `.planning/` still exist, as of the
    /// last parse. See [`ProjectPresence`].
    pub presence: ProjectPresence,
}

impl ProjectState {
    /// The roadmap entry a written phase id names, pad-insensitively: `07.1`,
    /// `7.1` and `Phase 7.1`'s id all find the one merged `7.1` row
    /// ([`phase_num::same_phase`]). Walks `phases` in roadmap order, so the
    /// answer never depends on map iteration order.
    ///
    /// Every lookup keyed by an id that did NOT come from `phases` itself — an
    /// argv target, a `**Depends on**:` reference, a STATE.md table cell —
    /// goes through this rather than `==` on `RoadmapPhase::number`, which is
    /// the roadmap's first-seen spelling and nothing more.
    pub fn roadmap_phase(&self, id: &str) -> Option<&roadmap_md::RoadmapPhase> {
        self.phases
            .iter()
            .find(|phase| phase_num::same_phase(&phase.number, id))
    }

    /// The disk inference for a written phase id, pad-insensitively.
    ///
    /// `phase_disk_statuses` is keyed by the roadmap's own spelling of each
    /// phase, so the id is first resolved to that spelling through
    /// [`Self::roadmap_phase`]; an exact key hit is taken as-is (a caller that
    /// already holds `phase.number`, or a map populated directly).
    pub fn disk_status_for(&self, id: &str) -> Option<&disk_status::DiskInference> {
        self.phase_disk_statuses.get(id).or_else(|| {
            self.roadmap_phase(id)
                .and_then(|phase| self.phase_disk_statuses.get(&phase.number))
        })
    }

    /// The phase number to treat as active — the phase a label names, a
    /// roadmap highlights, a suggested command targets, and a browser opens
    /// into.
    ///
    /// **A clamp, not a precedence ladder.** Two sources answer the question,
    /// and the active phase is the *larger* of them:
    ///
    /// ```text
    /// active = max(STATE.md's `current_phase`, the disk frontier)
    /// ```
    ///
    /// - **STATE.md's `current_phase`** ([`Self::state_md_phase_number`]) is a
    ///   statement rather than an inference: GSD writes it and advances it.
    /// - **The disk frontier** ([`Self::current_phase_number`]) is the first
    ///   phase in `.planning/phases/` whose implementation is unfinished.
    ///
    /// Each is the *only* witness for one direction, which is why neither can
    /// simply outrank the other:
    ///
    /// **STATE.md may legitimately LEAD the disk.** A phase that has been
    /// discussed or planned but not yet executed leaves the frontier behind it
    /// — GSD has moved on and the directories have not caught up. This
    /// repository has been exactly that case, and an inversion that let the
    /// frontier win unconditionally would have reported a phase the project had
    /// already left.
    ///
    /// **STATE.md must never drag the active phase BACKWARDS.** The frontier is
    /// the harder evidence — PLAN/SUMMARY pairs and verification files that
    /// exist — and it is a floor. picsync's STATE.md said `current_phase: 3`
    /// while its phase 3 was `Complete` on disk with verification `Passed` and
    /// its phase 4 was already planned; that number was written by an agent with
    /// a partial view and, under the `Option::or` ladder this replaces, it
    /// outranked the disk. Phase 3 then drew `*` (current) on the same screen
    /// whose badge called it `[Complete]` — a self-contradicting render. A
    /// number cannot un-write work that is on disk, so it does not get to.
    ///
    /// A `max` also restores an invariant the ladder broke: the frontier is the
    /// phase [`Self::current_phase_status`] describes, so under `.or()` the
    /// active phase could name phase 3 while the inference beside it described
    /// phase 4. The clamp can only ever agree with the frontier or lead it.
    ///
    /// **`completed_phases + 1` only when BOTH sources are absent** (no STATE.md
    /// phase and no parsable roadmap phases). The arithmetic is last for a
    /// reason: the count comes from ROADMAP's `## Progress` table, which records
    /// what has been *marked* complete. picsync's said 2 complete of 8 while
    /// phase 4 was already planned, so the arithmetic produced P3 for a project
    /// whose STATE.md and whose phase directories both said more.
    ///
    /// The comparison is numeric because both sources are
    /// [`phase_num::PhaseNum`]s — a zero-padded `04` and a bare `4` are the
    /// same number, an inserted `7.1` orders between `7` and `8`, and a
    /// prefixed id (`M-2`) arrives as `None` and simply does not participate
    /// rather than collapsing to 0.
    ///
    /// **Decimals are sources, not noise.** Both sources used to be
    /// `parse::<u32>()`, so an inserted phase abstained from both: ttbook's
    /// STATE.md said `current_phase: "7.1"` and its frontier was phase 7.1,
    /// yet the active phase fell through to the arithmetic — 4 phases marked
    /// `Complete` in the Progress table, so phase 5, a `[Complete]` phase,
    /// drew `*` while the phase actually being executed drew `o`.
    pub fn active_phase_number(&self) -> phase_num::PhaseNum {
        match (&self.state_md_phase_number, &self.current_phase_number) {
            (Some(declared), Some(frontier)) => declared.max(frontier).clone(),
            (Some(declared), None) => declared.clone(),
            (None, Some(frontier)) => frontier.clone(),
            (None, None) => phase_num::PhaseNum::from(self.completed_phases + 1),
        }
    }

    /// Every roadmap entry the Roadmap lists, in its order: [`Self::phases`],
    /// then [`Self::planned_phases`] no GSD phase already holds (deduped by
    /// `phase_key`, so a GSD phase wins a duplicate). `true` marks a planned
    /// (placeholder) entry.
    pub fn roadmap_entries(&self) -> Vec<(&roadmap_md::RoadmapPhase, bool)> {
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        self.phases
            .iter()
            .map(|p| (p, false))
            .chain(self.planned_phases.iter().map(|p| (p, true)))
            .filter(|(p, _)| seen.insert(phase_num::phase_key(&p.number)))
            .collect()
    }

    /// Whether a phase belongs to the current milestone, by the Roadmap's own
    /// band rule: with an active roadmap milestone, a phase in it or in no
    /// milestone at all; with none, a phase in no milestone (the synthetic /
    /// unbanded phases — every phase, in a roadmap without milestones).
    pub fn in_current_milestone(&self, phase_id: &str) -> bool {
        let own = roadmap_md::milestone_index_of(&self.milestones, phase_id);
        match roadmap_md::active_milestone_index(&self.milestones, &self.milestone) {
            Some(active) => own.is_none() || own == Some(active),
            None => own.is_none(),
        }
    }

    /// The project's phase count — see [`PhaseProgress`].
    ///
    /// Scope: [`Self::roadmap_entries`] in [`Self::in_current_milestone`], so
    /// a future milestone's build-phase placeholders are not counted. Done:
    /// [`phase_is_done`], which counts the current phase too, so a finished
    /// milestone reads n/n rather than n-1/n. When the scope is empty (no
    /// parsable roadmap phases), the STATE.md / `## Progress` bookkeeping
    /// answers instead, clamped so done never exceeds total.
    pub fn phase_progress(&self) -> PhaseProgress {
        let scope: Vec<&roadmap_md::RoadmapPhase> = self
            .roadmap_entries()
            .into_iter()
            .map(|(p, _)| p)
            .filter(|p| self.in_current_milestone(&p.number))
            .collect();
        if scope.is_empty() {
            return PhaseProgress {
                done: self.completed_phases.min(self.total_phases),
                total: self.total_phases,
            };
        }
        let done = scope
            .iter()
            .filter(|p| phase_is_done(&p.number, p.completed, &self.phase_disk_statuses))
            .count();
        PhaseProgress {
            done: done as u32,
            total: scope.len() as u32,
        }
    }

    /// The marker one roadmap entry earns — see [`PhaseMarker`] for why the
    /// two questions behind it must be answered from these two sources and no
    /// others.
    pub fn phase_marker(&self, phase: &roadmap_md::RoadmapPhase) -> PhaseMarker {
        PhaseMarker::decide(
            &phase.number,
            phase.completed,
            &self.phase_disk_statuses,
            &self.active_phase_number(),
        )
    }
}

/// What a phase's row says about itself: behind you, under way, or ahead.
///
/// **One decision, two call sites** ([`crate::ui::roadmap_widget`] and the
/// Detail screen's Phases list). Both used to inline the same three arms over
/// the same two inputs, and the inputs disagreed:
///
/// - "is this phase current?" moved onto [`ProjectState::active_phase_number`]
///   (`max(STATE.md, disk frontier)`, arithmetic only when neither exists), while
/// - "is this phase done?" stayed on `RoadmapPhase::completed`, the literal
///   `- [x]` checkbox in ROADMAP's `## Phases` list.
///
/// picsync's phase 3 is `Complete` on disk with verification `Passed`, and its
/// checkbox is a stale `- [ ]`. Once "current" advanced to phase 4, phase 3
/// matched neither arm and rendered as `o` — *future* — for a phase with seven
/// PLAN/SUMMARY pairs and a passing `03-VERIFICATION.md` behind it. The `*` it
/// showed before was not correctness; it was the arithmetic being wrong in a
/// direction that happened to look right.
///
/// So "done" now comes from the same place "current" does — the disk — and the
/// roadmap's own bookkeeping is the fallback for a phase the disk cannot speak
/// to at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseMarker {
    /// Implementation is behind this phase. Rendered `+`.
    Done,
    /// The phase [`ProjectState::active_phase_number`] names. Rendered `*`.
    Current,
    /// Not started, or at least not finished and not current. Rendered `o`.
    Future,
}

impl PhaseMarker {
    /// The ASCII glyph, matching the Detail screen's legend
    /// (`+ done  * current  o future`).
    pub fn glyph(self) -> &'static str {
        match self {
            PhaseMarker::Done => "+",
            PhaseMarker::Current => "*",
            PhaseMarker::Future => "o",
        }
    }

    /// Decide one phase's marker from that phase's own evidence.
    ///
    /// **Current outranks done**, which inverts what both call sites used to
    /// do — and it has to, now that "done" reads the disk. A phase can be both
    /// at once: `Executed`/`HumanNeeded` is implementation finished with
    /// verification waiting on a human, and
    /// [`ProjectState::active_phase_number`] can still name such a phase —
    /// STATE.md may name one the disk has already executed, and when every
    /// phase is implemented the frontier itself falls back to the last one.
    /// Under the old "done first" order those phases would draw `+` and the
    /// roadmap would carry no `*` at all — the one glyph a human scans for,
    /// missing from the one screen that exists to show where work is. A phase
    /// that is both finished and named as current is where the work is; it
    /// says so.
    ///
    /// The cost is the shipped-milestone case: when every phase is
    /// implemented, [`ProjectState::active_phase_number`] resolves to the last
    /// one, which then draws `*` rather than `+`. That is the convention the
    /// rest of the app already follows — [`parse_project_state`] deliberately
    /// reports the last phase's inference as the current-phase status in
    /// exactly that situation — and the dashboard reads milestone completion
    /// from `completed_phases >= total_phases`, not from this glyph.
    ///
    /// The done half lives in [`phase_is_done`], the single threshold the
    /// phase counts use too.
    ///
    /// **`>= Executed` is the done threshold, not `== Complete`.** It is the
    /// same threshold [`parse_project_state`] uses to place the frontier — the
    /// frontier is the first phase *below* `Executed`, so every phase at or
    /// above it is by construction behind the frontier. Two thresholds over one
    /// ordering is how this bug happened; there is now one. The verification
    /// nuance that separates `Executed` from `Complete` is not lost: the Detail
    /// screen's `[Executed]` badge says it in words, right beside this glyph,
    /// and `+` never meant "verified" — the `- [x]` it replaces is written on
    /// execution.
    ///
    /// **A phase with no directory falls back to the checkbox**, rather than
    /// being declared unfinished. `DiskStatus::NoDirectory` is the absence of
    /// evidence, not evidence of absence; a roadmap that says a phase shipped
    /// is the only witness left for a phase whose artifacts were archived
    /// somewhere this scanner does not look.
    pub fn decide(
        phase_number: &str,
        roadmap_completed: bool,
        disk_statuses: &HashMap<String, disk_status::DiskInference>,
        active_phase_number: &phase_num::PhaseNum,
    ) -> PhaseMarker {
        // Numeric comparison, so a zero-padded roadmap entry (`04`, `07.1`)
        // matches phase 4 / 7.1 — the same parse `active_phase_number`'s own
        // sources use.
        if phase_num::PhaseNum::parse(phase_number).as_ref() == Some(active_phase_number) {
            return PhaseMarker::Current;
        }
        if phase_is_done(phase_number, roadmap_completed, disk_statuses) {
            return PhaseMarker::Done;
        }
        PhaseMarker::Future
    }
}

/// Whether one phase's **implementation** is finished: disk status
/// `>= Executed`, or — when the phase has no directory or no disk entry at
/// all — the ROADMAP checkbox.
///
/// The one done threshold, shared by [`PhaseMarker::decide`] (whose doc
/// comment argues the threshold and the checkbox fallback) and
/// [`ProjectState::phase_progress`], so the `+` glyph, the Roadmap band
/// counts, the Roadmap header and the dashboard `k/n phases` cell cannot use
/// different ones. Unlike the marker, it does not care whether the phase is
/// also the current one.
pub fn phase_is_done(
    phase_number: &str,
    roadmap_completed: bool,
    disk_statuses: &HashMap<String, disk_status::DiskInference>,
) -> bool {
    match disk_statuses.get(phase_number) {
        Some(inf) if inf.status != disk_status::DiskStatus::NoDirectory => {
            inf.status >= disk_status::DiskStatus::Executed
        }
        _ => roadmap_completed,
    }
}

/// A project's phase count: `done` of `total` phases in the CURRENT
/// milestone, done meaning implementation finished ([`phase_is_done`]).
///
/// The one definition behind the dashboard's `k/n phases` cell and the
/// Roadmap header's `k of n phases done` ([`ProjectState::phase_progress`]).
/// Those used to disagree for ttbook: the dashboard read ROADMAP's `##
/// Progress` table (4/6, lagging phase 12's executed-and-verified work) and
/// the header counted every listed node, future milestones' build-phase
/// placeholders included (5 of 11). Both now read 5/6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseProgress {
    pub done: u32,
    pub total: u32,
}

/// One phase's `(completed, total)` plan counts, for display.
///
/// ROADMAP's plan checklist under the phase comes first — it is what both
/// render sites always showed. When the roadmap lists no plans (GSD leaves
/// `**Plans**: TBD` in place for phases planned without the list being
/// back-filled), the phase directory's own PLAN/SUMMARY counts answer instead,
/// so a phase with six executed plans on disk does not read `0/?` beside an
/// `[Executed]` badge. `None` only when neither source knows of any plan.
pub fn phase_plan_counts(
    phase: &roadmap_md::RoadmapPhase,
    disk_statuses: &HashMap<String, disk_status::DiskInference>,
) -> Option<(u32, u32)> {
    if phase.total_plans > 0 {
        return Some((phase.completed_plans, phase.total_plans));
    }
    disk_statuses
        .get(&phase.number)
        .filter(|inf| inf.plan_count > 0)
        .map(|inf| (inf.summary_count.min(inf.plan_count), inf.plan_count))
}

/// Why a HANDOFF was judged stale. See [`handoff_staleness`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaleHandoffReason {
    /// The handoff names a phase numerically lower than STATE.md's declared
    /// `current_phase`.
    PhaseBehind,
    /// STATE.md's `last_updated` is later than the handoff's written time
    /// plus [`HANDOFF_STALE_GRACE_SECS`].
    StateNewer,
}

/// A HANDOFF the parser ignored as stale ([`ProjectState::stale_handoff`]).
///
/// Every field is reader-generated (a numeric parse, a timestamp, an enum),
/// never the handoff's prose, so it can reach a terminal cell without
/// carrying third-party text.
#[derive(Debug, Clone, PartialEq)]
pub struct StaleHandoff {
    /// The phase the handoff names, when it names a parsable one.
    pub phase: Option<phase_num::PhaseNum>,
    /// STATE.md's declared `current_phase`, when it declares one.
    pub state_phase: Option<phase_num::PhaseNum>,
    /// When the handoff was written: its JSON `timestamp`, else its mtime.
    pub written: Option<chrono::DateTime<chrono::Utc>>,
    pub reason: StaleHandoffReason,
}

/// How much later than a handoff STATE.md may be written before the handoff
/// counts as stale ([`StaleHandoffReason::StateNewer`]).
///
/// An hour absorbs same-session STATE writes and hand-written timestamps that
/// drift by tens of minutes (ttbook's JSON `timestamp` is 41 minutes after its
/// own file mtime); the real stale gaps observed are 20 hours to 4 months.
const HANDOFF_STALE_GRACE_SECS: i64 = 3600;

/// What [`detect_handoff`] found in a non-empty HANDOFF file.
#[derive(Debug, Clone, PartialEq)]
struct HandoffFile {
    /// `next_action` (JSON) or the first non-heading line (MD).
    context: Option<String>,
    /// The JSON `phase` (string or number), normalised; never set for MD.
    phase: Option<phase_num::PhaseNum>,
    /// The JSON `timestamp` (RFC 3339) when parseable, else the file mtime.
    written: Option<chrono::DateTime<chrono::Utc>>,
}

/// A file's mtime as a UTC timestamp, `None` when the platform cannot say.
fn file_mtime(path: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .map(chrono::DateTime::<chrono::Utc>::from)
}

/// Normalise a HANDOFF.json `phase` value: `"12"`, `12`, `"04"`, `"18.1"`,
/// `18.1` and `"Phase 12"` all parse; anything else is `None`.
fn handoff_phase(value: &serde_json::Value) -> Option<phase_num::PhaseNum> {
    let text = match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => return None,
    };
    roadmap_md::extract_phase_id(&text).and_then(|id| phase_num::PhaseNum::parse(&id))
}

/// Detect HANDOFF.md or HANDOFF.json in a planning directory.
///
/// `None` when neither exists non-empty (after trim) -- not considered
/// paused. HANDOFF.json wins over HANDOFF.md.
///
/// - HANDOFF.json: context is the `next_action` field; `phase` and
///   `timestamp` feed [`handoff_staleness`]. Non-empty invalid JSON is still
///   a handoff, with no context and no phase.
/// - HANDOFF.md: context is the first non-empty line after any `#` heading,
///   or the first non-empty line.
///
/// The written time falls back to the file's mtime whenever no parseable
/// `timestamp` is declared (always, for HANDOFF.md). The declared timestamp is
/// preferred because a handoff can be touched after the fact: cdr-configurator's
/// HANDOFF.json has a later mtime than its STATE.md, but declares a timestamp
/// older than it — and it asks to plan a phase that is complete on disk.
fn detect_handoff(planning_dir: &Path) -> Option<HandoffFile> {
    // Try HANDOFF.json first
    let json_path = planning_dir.join("HANDOFF.json");
    if let Ok(content) = std::fs::read_to_string(&json_path) {
        if !content.trim().is_empty() {
            let mut handoff = HandoffFile {
                context: None,
                phase: None,
                written: None,
            };
            // Non-empty but invalid JSON -- still paused, no context
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                handoff.context = val
                    .get("next_action")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                handoff.phase = val.get("phase").and_then(handoff_phase);
                handoff.written = val
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s.trim()).ok())
                    .map(|t| t.with_timezone(&chrono::Utc));
            }
            if handoff.written.is_none() {
                handoff.written = file_mtime(&json_path);
            }
            return Some(handoff);
        }
    }

    // Try HANDOFF.md
    let md_path = planning_dir.join("HANDOFF.md");
    if let Ok(content) = std::fs::read_to_string(&md_path) {
        if !content.trim().is_empty() {
            // Extract first non-empty line after any # heading line, or first
            // non-empty line. A file of only headings has no context.
            let context = content
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_string);
            return Some(HandoffFile {
                context,
                phase: None,
                written: file_mtime(&md_path),
            });
        }
    }

    None
}

/// Decide whether a handoff is stale; `None` means it is current.
///
/// - **R1, [`StaleHandoffReason::PhaseBehind`]:** the handoff's phase is
///   numerically lower than STATE.md's declared `current_phase`. Compared
///   against the DECLARED number only, not
///   [`ProjectState::active_phase_number`]: the disk frontier can
///   legitimately lead a phase whose plans are executed but still being
///   verified, and a handoff for that phase is still current. GSD advances
///   `current_phase` deliberately.
/// - **R2, [`StaleHandoffReason::StateNewer`]:** STATE.md's `last_updated` is
///   later than the handoff's written time plus [`HANDOFF_STALE_GRACE_SECS`].
///
/// R1 is reported when both hold. A missing input disables only the rule
/// that needs it, so a state with no STATE signals never judges a handoff
/// stale. The handoff's own `status` field is deliberately not consulted:
/// GSD's pause-work template always writes `paused`.
fn handoff_staleness(
    handoff_phase: Option<&phase_num::PhaseNum>,
    handoff_written: Option<chrono::DateTime<chrono::Utc>>,
    state_phase: Option<&phase_num::PhaseNum>,
    state_last_updated: Option<chrono::DateTime<chrono::Utc>>,
) -> Option<StaleHandoffReason> {
    if let (Some(handoff), Some(state)) = (handoff_phase, state_phase) {
        if handoff < state {
            return Some(StaleHandoffReason::PhaseBehind);
        }
    }
    if let (Some(written), Some(updated)) = (handoff_written, state_last_updated) {
        if updated > written + chrono::Duration::seconds(HANDOFF_STALE_GRACE_SECS) {
            return Some(StaleHandoffReason::StateNewer);
        }
    }
    None
}

/// Parse a GSD project's .planning/ directory into a ProjectState.
/// Gracefully handles missing or malformed files -- never panics.
pub fn parse_project_state(planning_dir: &Path) -> ProjectState {
    let mut state = ProjectState {
        status: "unknown".to_string(),
        ..Default::default()
    };

    // Resolve the project root (parent of .planning/) and its last-activity
    // timestamp (last commit time, mtime fallback).
    state.project_root = planning_dir
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| planning_dir.to_path_buf());

    // Presence, decided here and nowhere else. No early return: every read
    // below already fails silently on a missing path, and a single code path
    // means a field added later cannot be skipped for missing projects.
    state.presence = if !state.project_root.is_dir() {
        ProjectPresence::FolderMissing
    } else if !planning_dir.is_dir() {
        ProjectPresence::NoPlanning
    } else {
        ProjectPresence::Present
    };

    state.last_activity = git_ops::project_last_activity(&state.project_root);

    // STATE.md's `last_updated`, kept only as a local: it feeds the stale
    // HANDOFF decision below and is not stored on `ProjectState` (no free
    // `String` joins the census). Unparseable values are ignored.
    let mut state_last_updated: Option<chrono::DateTime<chrono::Utc>> = None;

    // Parse STATE.md
    let state_md_path = planning_dir.join("STATE.md");
    if let Ok(content) = std::fs::read_to_string(&state_md_path) {
        // ONE exhaustive match with no wildcard arm, deliberately. The two
        // sequential `if let`s this replaced classified `Parsed` and
        // `Unreadable` and would have skipped a third outcome in silence —
        // which is the whole failure mode `FrontmatterOutcome` exists to
        // prevent. A fourth variant is now a compile error here.
        let frontmatter = match state_md::read_frontmatter(&content) {
            state_md::FrontmatterOutcome::Parsed(fm) => Some(*fm),
            // The values arrived, but only after an in-memory repair. Both
            // facts are recorded: the fields populate exactly as a clean read
            // populates them (one site below, so the two cannot drift), and the
            // render path says the document was repaired.
            state_md::FrontmatterOutcome::Recovered { frontmatter, .. } => {
                state.state_md_recovered = true;
                Some(*frontmatter)
            }
            state_md::FrontmatterOutcome::Absent => None,
            state_md::FrontmatterOutcome::Unreadable { fault, position } => {
                // Recorded rather than swallowed: the render path shows it.
                state.state_md_unreadable = true;
                state.state_md_fault = Some(fault);
                state.state_md_fault_position = position;
                None
            }
        };
        if let Some(fm) = frontmatter {
            state.status = if fm.status.is_empty() {
                "unknown".to_string()
            } else {
                fm.status
            };
            state.total_phases = fm.progress.total_phases;
            state.completed_phases = fm.progress.completed_phases;
            state.total_plans = fm.progress.total_plans;
            state.completed_plans = fm.progress.completed_plans;

            // ADR-2207 frontmatter keys (empty when absent).
            // The NUMBER is kept before `current_phase` is resolved to a label,
            // because that resolution discards it whenever a name is present.
            state.state_md_phase_number = fm
                .current_phase
                .as_deref()
                .and_then(phase_num::PhaseNum::parse);
            state_last_updated = chrono::DateTime::parse_from_rfc3339(fm.last_updated.trim())
                .ok()
                .map(|t| t.with_timezone(&chrono::Utc));
            state.current_phase_name = fm.current_phase_name.clone().unwrap_or_default();
            state.current_plan = fm.current_plan.clone().unwrap_or_default();

            // Derive current_phase. Preference order:
            //   1. explicit `current_phase_name` frontmatter,
            //   2. explicit `current_phase` frontmatter (as `Phase {n}`),
            //   3. ADR-2207 status: milestone-terminal renders `<milestone> Complete`,
            //      but the intermediate `All phases complete` must NOT (the milestone
            //      is not done — it awaits `/gsd:complete-milestone`),
            //   4. the legacy stopped_at / count-based completion heuristic.
            if let Some(name) = fm
                .current_phase_name
                .as_deref()
                .filter(|s| !s.is_empty())
            {
                state.current_phase = name.to_string();
            } else if let Some(ph) = fm.current_phase.as_deref().filter(|s| !s.is_empty()) {
                state.current_phase = format!("Phase {}", ph);
            } else if state_md::is_milestone_terminal(&state.status) {
                state.current_phase = if !fm.milestone.is_empty() {
                    format!("{} Complete", fm.milestone)
                } else {
                    "Complete".to_string()
                };
            } else if state_md::is_all_phases_complete(&state.status) {
                state.current_phase = "All phases complete".to_string();
            } else if !fm.stopped_at.is_empty() {
                state.current_phase = fm.stopped_at.clone();
            } else if fm.progress.completed_phases >= fm.progress.total_phases
                && fm.progress.total_phases > 0
            {
                state.current_phase = if !fm.milestone.is_empty() {
                    format!("{} Complete", fm.milestone)
                } else {
                    "Complete".to_string()
                };
            } else {
                state.current_phase = format!("Phase {}", fm.progress.completed_phases + 1);
            }

            state.milestone = fm.milestone;
            let milestone_name = fm.milestone_name.trim();
            state.milestone_name = (!milestone_name.is_empty())
                .then(|| crate::text::Untrusted::from_untrusted_source(milestone_name.to_string()));
        }
        // G15 reads the document body, not the frontmatter, so it is parsed
        // from the same content regardless of whether the frontmatter parsed.
        state.deferred_verification_phases = state_md::deferred_verification_phases(&content);
    }

    // G10: a non-empty project-root continue-here marker. Content check, not
    // existence check, on `detect_handoff`'s precedent.
    state.continue_here_present = std::fs::read_to_string(planning_dir.join(".continue-here.md"))
        .map(|content| !content.trim().is_empty())
        .unwrap_or(false);

    // Parse ROADMAP.md. The `## Progress` table (GSD 1.8.0) is authoritative
    // for progress counts when present; otherwise STATE.md frontmatter stands.
    let roadmap_path = planning_dir.join("ROADMAP.md");
    if let Ok(content) = std::fs::read_to_string(&roadmap_path) {
        state.phases = roadmap_md::parse_roadmap_phases(&content);
        state.planned_phases = roadmap_md::parse_planned_build_phases(&content);
        state.shipped_phases = roadmap_md::parse_shipped_phases(&content);
        state.phase_goals = roadmap_md::parse_phase_goals(&content);
        state.milestones = roadmap_md::roadmap_milestones(&content);
        // STATE.md is the milestone's primary source; a project whose STATE.md
        // carries no `milestone:` key still names it in ROADMAP's
        // `## Milestones` list, and an empty `Milestone:` field is worse than
        // the roadmap's own words.
        if state.milestone.is_empty() {
            if let Some(name) = roadmap_md::active_milestone(&content) {
                state.milestone = name;
            }
        }
        if let Some(prog) = roadmap_md::roadmap_progress(&content) {
            state.total_phases = prog.total_phases;
            state.completed_phases = prog.completed_phases;
            state.total_plans = prog.total_plans;
            state.completed_plans = prog.completed_plans;
        }
    }

    // Run disk inference for each phase
    let mut current_phase_inference: Option<disk_status::DiskInference> = None;
    let mut current_phase_number: Option<phase_num::PhaseNum> = None;
    for phase in &state.phases {
        let inference = disk_status::infer_phase_status(planning_dir, &phase.number);
        // Track the first phase whose IMPLEMENTATION is not finished as the
        // current phase status.
        //
        // **`< Executed`, and the threshold is a decision rather than an
        // inheritance (WR-04).** This condition read `!= Complete` until Phase
        // 20 made `Complete` a conjunction (implementation AND verification
        // passed) and inserted `Executed` beneath it. That silently moved the
        // threshold: every phase awaiting verification now reads `Executed`, so
        // the loop stopped at the first *unverified* phase instead of the first
        // *unexecuted* one, and this repository's own dashboard cell moved from
        // phase 20 to phase 19.
        //
        // The frontier is implementation, and the reason is what this value
        // feeds: the dashboard row's compact D-R-P-E-V cell
        // (`ui/screens/normal.rs`) sits beside a phase LABEL taken from
        // STATE.md's `current_phase`, which GSD advances on execution. A cell
        // describing a different phase from the one its own row names is worse
        // than a cell that omits something. The verification gate is not lost by
        // this choice — it surfaces through the needs-human badge (D-24) and,
        // for the driver, through the DRIVE-05 gate set, both of which read the
        // per-phase inference directly rather than this summary.
        //
        // Written against the `Ord` Phase 20 added, so the next variant inserted
        // below `Executed` is included automatically and one inserted above it
        // is not — which is the behaviour a threshold wants and the reason this
        // is a comparison rather than a list of variants.
        if current_phase_inference.is_none() && inference.status < disk_status::DiskStatus::Executed
        {
            current_phase_inference = Some(inference.clone());
            current_phase_number = phase_num::PhaseNum::parse(&phase.number);
        }
        state
            .phase_disk_statuses
            .insert(phase.number.clone(), inference);
    }
    // If every phase is implemented, use the last phase's status — including
    // when some of them are still awaiting verification, which is the same
    // threshold the loop above uses and for the same reason.
    if current_phase_inference.is_none() && !state.phases.is_empty() {
        if let Some(last) = state.phases.last() {
            current_phase_inference = state.phase_disk_statuses.get(&last.number).cloned();
            current_phase_number = phase_num::PhaseNum::parse(&last.number);
        }
    }
    state.current_phase_status = current_phase_inference;
    state.current_phase_number = current_phase_number;

    // Count backlog items
    state.backlog_count = count_backlog_items(planning_dir);

    // Load queued actions from QUEUE.md
    state.queued_actions = queue_md::load_queue(planning_dir);

    // Detect HANDOFF files for pause state. Staleness is decided here, once,
    // so every consumer of `paused` / `pause_context` (the dashboard badge,
    // needs-human and its /h filter, the Roadmap header banner) agrees.
    if let Some(handoff) = detect_handoff(planning_dir) {
        match handoff_staleness(
            handoff.phase.as_ref(),
            handoff.written,
            state.state_md_phase_number.as_ref(),
            state_last_updated,
        ) {
            Some(reason) => {
                state.paused = false;
                state.pause_context = None;
                state.stale_handoff = Some(StaleHandoff {
                    phase: handoff.phase,
                    state_phase: state.state_md_phase_number.clone(),
                    written: handoff.written,
                    reason,
                });
            }
            None => {
                state.paused = true;
                state.pause_context = handoff.context;
            }
        }
    }

    // Detect async external jobs (a phase waiting on an external job is
    // legitimately blocked, not stuck).
    state.external_job_waiting = detect_async_jobs(planning_dir);

    // Load parallel workstreams (GSD 1.8.0). Recursion guard: skip when this
    // planning dir is itself a workstream (immediate parent named `workstreams`),
    // so a workstream sub-parse never re-enters workstream loading. This bounds
    // recursion depth to one level.
    let inside_workstream = planning_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        == Some("workstreams");
    if !inside_workstream {
        state.workstreams = workstreams::load_workstreams(planning_dir);
    }

    state
}

/// Returns true when `.planning/async-jobs/` contains at least one `*.json`
/// manifest. GSD 1.8.0 writes `.planning/async-jobs/<job>.json` for a phase
/// waiting on an external (long-running) job.
fn detect_async_jobs(planning_dir: &Path) -> bool {
    let dir = planning_dir.join("async-jobs");
    std::fs::read_dir(&dir)
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|e| {
                e.file_type().map(|t| t.is_file()).unwrap_or(false)
                    && e.path().extension().and_then(|x| x.to_str()) == Some("json")
            })
        })
        .unwrap_or(false)
}

/// Count the backlog items in `.planning/phases/`.
///
/// **Delegates to [`backlog::count_backlog_dirs`] rather than re-deciding what a
/// backlog item is** (260916-vr0, D-INF-02). This used to carry its own looser
/// rule — any entry whose name `starts_with("999")` — while the Backlog tab
/// applied a stricter one, so the number advertised here was not the number of
/// rows the tab could draw. One rule in one place is what makes that
/// impossible; a future edit to the matching rule now cannot move the count
/// without moving the list.
pub fn count_backlog_items(planning_dir: &Path) -> u32 {
    backlog::count_backlog_dirs(planning_dir) as u32
}

/// Write the ttbook-shaped "phase 13 executing, stale phase-12 HANDOFF"
/// fixture into `planning` (quick 260926-16t): the three sanitised files under
/// `tests/fixtures/roadmaps/ttbook-phase13-*`, phases 8-12 executed with a
/// passing verification, and phase 13 part-way (2 plans, 1 summary).
///
/// Shared by the state reader's, the Detail screen's and the dashboard's
/// tests, so all three pin the same project.
#[cfg(test)]
pub(crate) fn write_ttbook_phase13_fixture(planning: &Path) {
    use std::fs;
    fs::create_dir_all(planning).unwrap();
    fs::write(
        planning.join("ROADMAP.md"),
        include_str!("../../tests/fixtures/roadmaps/ttbook-phase13-ROADMAP.md"),
    )
    .unwrap();
    fs::write(
        planning.join("STATE.md"),
        include_str!("../../tests/fixtures/roadmaps/ttbook-phase13-STATE.md"),
    )
    .unwrap();
    fs::write(
        planning.join("HANDOFF.json"),
        include_str!("../../tests/fixtures/roadmaps/ttbook-phase13-HANDOFF.json"),
    )
    .unwrap();
    for (num, slug) in [("08", "a"), ("09", "b"), ("10", "c"), ("11", "d"), ("12", "e")] {
        let dir = planning.join("phases").join(format!("{num}-{slug}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{num}-01-PLAN.md")), "# plan\n").unwrap();
        fs::write(dir.join(format!("{num}-01-SUMMARY.md")), "# summary\n").unwrap();
        // Written last, so no SUMMARY is newer than it (a stale verification).
        fs::write(
            dir.join(format!("{num}-VERIFICATION.md")),
            "---\nstatus: passed\n---\n",
        )
        .unwrap();
    }
    let dir = planning.join("phases").join("13-f");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("13-01-PLAN.md"), "# plan\n").unwrap();
    fs::write(dir.join("13-02-PLAN.md"), "# plan\n").unwrap();
    fs::write(dir.join("13-01-SUMMARY.md"), "# summary\n").unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // --- quick-260926-16t: stale HANDOFF --------------------------------------

    fn pn(s: &str) -> phase_num::PhaseNum {
        phase_num::PhaseNum::parse(s).unwrap()
    }

    fn ts(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    /// Set a file's mtime (std `File::set_modified`, stable since 1.75).
    fn set_mtime(path: &Path, when: chrono::DateTime<chrono::Utc>) {
        let file = fs::File::options().write(true).open(path).unwrap();
        file.set_modified(std::time::SystemTime::from(when)).unwrap();
    }

    #[test]
    fn stale_handoff_phase_behind_regardless_of_times() {
        let t = ts("2026-09-25T00:00:00Z");
        assert_eq!(
            handoff_staleness(Some(&pn("12")), Some(t), Some(&pn("13")), Some(t)),
            Some(StaleHandoffReason::PhaseBehind)
        );
        // Handoff written long after STATE: still behind.
        assert_eq!(
            handoff_staleness(
                Some(&pn("12")),
                Some(ts("2026-12-01T00:00:00Z")),
                Some(&pn("13")),
                Some(t)
            ),
            Some(StaleHandoffReason::PhaseBehind)
        );
    }

    #[test]
    fn stale_handoff_state_newer_beyond_grace() {
        assert_eq!(
            handoff_staleness(
                Some(&pn("13")),
                Some(ts("2026-09-25T00:00:00Z")),
                Some(&pn("13")),
                Some(ts("2026-09-25T02:00:00Z"))
            ),
            Some(StaleHandoffReason::StateNewer)
        );
    }

    #[test]
    fn stale_handoff_state_newer_within_grace_is_current() {
        assert_eq!(
            handoff_staleness(
                Some(&pn("13")),
                Some(ts("2026-09-25T00:00:00Z")),
                Some(&pn("13")),
                Some(ts("2026-09-25T00:10:00Z"))
            ),
            None
        );
    }

    #[test]
    fn stale_handoff_phase_ahead_is_not_behind() {
        assert_eq!(
            handoff_staleness(
                Some(&pn("21")),
                Some(ts("2026-09-25T00:00:00Z")),
                Some(&pn("19")),
                Some(ts("2026-09-01T00:00:00Z"))
            ),
            None
        );
    }

    #[test]
    fn stale_handoff_no_inputs_is_current() {
        assert_eq!(handoff_staleness(None, None, None, None), None);
    }

    #[test]
    fn stale_handoff_both_rules_report_phase_behind() {
        assert_eq!(
            handoff_staleness(
                Some(&pn("7.1")),
                Some(ts("2026-01-01T00:00:00Z")),
                Some(&pn("08")),
                Some(ts("2026-09-01T00:00:00Z"))
            ),
            Some(StaleHandoffReason::PhaseBehind)
        );
    }

    #[test]
    fn handoff_phase_accepts_strings_numbers_and_prefixes() {
        use serde_json::json;
        assert_eq!(handoff_phase(&json!("12")), Some(pn("12")));
        assert_eq!(handoff_phase(&json!("04")), Some(pn("4")));
        assert_eq!(handoff_phase(&json!("18.1")), Some(pn("18.1")));
        assert_eq!(handoff_phase(&json!("Phase 12")), Some(pn("12")));
        assert_eq!(handoff_phase(&json!(12)), Some(pn("12")));
        assert_eq!(handoff_phase(&json!("TBD")), None);
        assert_eq!(handoff_phase(&json!(null)), None);
    }

    #[test]
    fn stale_handoff_json_phase_behind_state_is_not_paused() {
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\ncurrent_phase: 13\n---\n"),
            (
                "HANDOFF.json",
                "{\"phase\":\"12\",\"next_action\":\"Dispatch /gsd-discuss-phase 12\"}",
            ),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(!state.paused);
        assert!(state.pause_context.is_none());
        let stale = state.stale_handoff.expect("stale handoff");
        assert_eq!(stale.reason, StaleHandoffReason::PhaseBehind);
        assert_eq!(stale.phase, Some(pn("12")));
        assert_eq!(stale.state_phase, Some(pn("13")));
    }

    #[test]
    fn stale_handoff_json_timestamp_older_than_state_is_not_paused() {
        let td = make_planning(&[
            (
                "STATE.md",
                "---\nstatus: executing\ncurrent_phase: 12\nlast_updated: \"2026-09-25T22:34:52.070Z\"\n---\n",
            ),
            (
                "HANDOFF.json",
                "{\"phase\":\"12\",\"timestamp\":\"2026-09-24T21:30:00-05:00\",\"next_action\":\"x\"}",
            ),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(!state.paused);
        assert!(state.pause_context.is_none());
        let stale = state.stale_handoff.expect("stale handoff");
        assert_eq!(stale.reason, StaleHandoffReason::StateNewer);
        assert_eq!(stale.written, Some(ts("2026-09-25T02:30:00Z")));
    }

    #[test]
    fn current_handoff_json_timestamp_after_state_still_pauses() {
        let td = make_planning(&[
            (
                "STATE.md",
                "---\nstatus: executing\ncurrent_phase: 12\nlast_updated: \"2026-09-25T22:34:52.070Z\"\n---\n",
            ),
            (
                "HANDOFF.json",
                "{\"phase\":\"12\",\"timestamp\":\"2026-09-26T08:00:00Z\",\"next_action\":\"Resume 12-04\"}",
            ),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.paused);
        assert_eq!(state.pause_context.as_deref(), Some("Resume 12-04"));
        assert!(state.stale_handoff.is_none());
    }

    #[test]
    fn stale_handoff_json_without_timestamp_falls_back_to_mtime() {
        let state_md = "---\nstatus: executing\nlast_updated: \"2026-01-01T00:00:00Z\"\n---\n";
        let td = make_planning(&[
            ("STATE.md", state_md),
            ("HANDOFF.json", "{\"next_action\":\"Resume\"}"),
        ]);
        let planning = td.path().join(".planning");
        // Natural (now) mtime: after STATE, so still a current pause.
        let state = parse_project_state(&planning);
        assert!(state.paused);
        assert!(state.stale_handoff.is_none());

        set_mtime(&planning.join("HANDOFF.json"), ts("2020-01-01T00:00:00Z"));
        let state = parse_project_state(&planning);
        assert!(!state.paused);
        assert!(state.pause_context.is_none());
        let stale = state.stale_handoff.expect("stale handoff");
        assert_eq!(stale.reason, StaleHandoffReason::StateNewer);
        assert_eq!(stale.written, Some(ts("2020-01-01T00:00:00Z")));
    }

    #[test]
    fn stale_handoff_md_uses_mtime() {
        let td = make_planning(&[
            (
                "STATE.md",
                "---\nstatus: executing\nlast_updated: \"2026-01-01T00:00:00Z\"\n---\n",
            ),
            ("HANDOFF.md", "# Handoff\n\nPick up at plan 14-02\n"),
        ]);
        let planning = td.path().join(".planning");
        set_mtime(&planning.join("HANDOFF.md"), ts("2020-01-01T00:00:00Z"));
        let state = parse_project_state(&planning);
        assert!(!state.paused);
        assert!(state.pause_context.is_none());
        assert_eq!(
            state.stale_handoff.map(|s| s.reason),
            Some(StaleHandoffReason::StateNewer)
        );
    }

    // --- quick-260926-16t: one phase-count definition ------------------------

    fn disk(status: disk_status::DiskStatus) -> disk_status::DiskInference {
        disk_status::DiskInference {
            status,
            ..Default::default()
        }
    }

    #[test]
    fn phase_is_done_reads_disk_then_checkbox() {
        use disk_status::DiskStatus as S;
        let mut map = HashMap::new();
        for (id, st) in [
            ("1", S::Executed),
            ("2", S::Complete),
            ("3", S::Partial),
            ("4", S::Planned),
            ("5", S::NoDirectory),
        ] {
            map.insert(id.to_string(), disk(st));
        }
        assert!(phase_is_done("1", false, &map));
        assert!(phase_is_done("2", false, &map));
        assert!(!phase_is_done("3", true, &map));
        assert!(!phase_is_done("4", true, &map));
        // NoDirectory and a missing entry fall back to the checkbox.
        assert!(phase_is_done("5", true, &map));
        assert!(!phase_is_done("5", false, &map));
        assert!(phase_is_done("6", true, &map));
        assert!(!phase_is_done("6", false, &map));
        // The marker still ranks Current above Done.
        assert_eq!(
            PhaseMarker::decide("1", false, &map, &pn("1")),
            PhaseMarker::Current
        );
        assert_eq!(
            PhaseMarker::decide("1", false, &map, &pn("9")),
            PhaseMarker::Done
        );
        assert_eq!(
            PhaseMarker::decide("3", true, &map, &pn("9")),
            PhaseMarker::Future
        );
    }

    #[test]
    fn human_needed_verification_still_counts_as_done() {
        // HumanNeeded is a verification outcome on an Executed phase.
        let mut map = HashMap::new();
        map.insert(
            "1".to_string(),
            disk_status::DiskInference {
                status: disk_status::DiskStatus::Executed,
                verification_status: disk_status::VerificationStatus::HumanNeeded,
                ..Default::default()
            },
        );
        assert!(phase_is_done("1", false, &map));
    }

    #[test]
    fn ttbook_phase13_phase_progress_is_5_of_6() {
        let td = TempDir::new().unwrap();
        let planning = td.path().join(".planning");
        write_ttbook_phase13_fixture(&planning);
        let state = parse_project_state(&planning);
        // The bookkeeping lags; the fix is not the bookkeeping.
        assert_eq!((state.completed_phases, state.total_phases), (4, 6));
        // Future milestones' placeholders exist but are out of scope.
        assert_eq!(state.planned_phases.len(), 5);
        assert_eq!(state.roadmap_entries().len(), 11);
        assert!(state
            .planned_phases
            .iter()
            .all(|p| !state.in_current_milestone(&p.number)));
        assert_eq!(state.phase_progress(), PhaseProgress { done: 5, total: 6 });
    }

    #[test]
    fn a_finished_milestone_whose_last_phase_is_current_reads_n_of_n() {
        let roadmap = "# Roadmap\n\n## Phases\n\n\
                       - [x] **Phase 1: One** - a\n\
                       - [x] **Phase 2: Two** - b\n";
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: verifying\ncurrent_phase: 2\n---\n"),
            ("ROADMAP.md", roadmap),
            ("phases/01-one/01-01-PLAN.md", "# p\n"),
            ("phases/01-one/01-01-SUMMARY.md", "# s\n"),
            ("phases/02-two/02-01-PLAN.md", "# p\n"),
            ("phases/02-two/02-01-SUMMARY.md", "# s\n"),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.phases.len(), 2);
        assert_eq!(
            state.phase_marker(&state.phases[1]),
            PhaseMarker::Current
        );
        assert_eq!(state.phase_progress(), PhaseProgress { done: 2, total: 2 });
    }

    #[test]
    fn phase_progress_falls_back_to_bookkeeping_without_roadmap_phases() {
        let state = ProjectState {
            completed_phases: 7,
            total_phases: 5,
            ..Default::default()
        };
        assert_eq!(state.phase_progress(), PhaseProgress { done: 5, total: 5 });
        let state = ProjectState {
            completed_phases: 2,
            total_phases: 5,
            ..Default::default()
        };
        assert_eq!(state.phase_progress(), PhaseProgress { done: 2, total: 5 });
    }

    #[test]
    fn a_roadmap_without_milestones_counts_every_phase() {
        let roadmap = "# Roadmap\n\n## Phases\n\n\
                       - [x] **Phase 1: One** - a\n\
                       - [ ] **Phase 2: Two** - b\n\
                       - [ ] **Phase 3: Three** - c\n";
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            ("ROADMAP.md", roadmap),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.milestones.is_empty());
        assert_eq!(state.phase_progress(), PhaseProgress { done: 1, total: 3 });
    }

    #[test]
    fn ttbook_phase13_fixture_handoff_is_stale_phase_behind() {
        let td = TempDir::new().unwrap();
        let planning = td.path().join(".planning");
        write_ttbook_phase13_fixture(&planning);
        let state = parse_project_state(&planning);
        assert!(!state.paused);
        assert!(state.pause_context.is_none());
        let stale = state.stale_handoff.expect("stale handoff");
        assert_eq!(stale.reason, StaleHandoffReason::PhaseBehind);
        assert_eq!(stale.phase, Some(pn("12")));
        assert_eq!(stale.state_phase, Some(pn("13")));
    }

    /// Build a temp project with a `.planning/` dir and the given files
    /// (relative paths under `.planning/`). Returns the TempDir (keep it alive).
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

    // --- quick-260922-hdh: presence is decided at parse time ---------------

    #[test]
    fn folder_presence_is_folder_missing_when_root_is_gone() {
        let td = TempDir::new().unwrap();
        let gone = td.path().join("never-created");
        let state = parse_project_state(&gone.join(".planning"));
        assert_eq!(state.presence, ProjectPresence::FolderMissing);
        assert_eq!(state.status, "unknown");
        assert_eq!(state.project_root, gone);
    }

    #[test]
    fn folder_presence_is_no_planning_when_root_exists_without_planning() {
        let td = TempDir::new().unwrap();
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.presence, ProjectPresence::NoPlanning);
    }

    #[test]
    fn folder_presence_is_present_for_a_real_planning_dir() {
        let td = make_planning(&[("STATE.md", "---\nstatus: executing\n---\n")]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.presence, ProjectPresence::Present);
    }

    #[test]
    fn test_frontmatter_current_phase_name_flows_into_state() {
        let td = make_planning(&[(
            "STATE.md",
            "---\nstatus: executing\ncurrent_phase: 14\ncurrent_phase_name: Live State\ncurrent_plan: \"0.3\"\n---\n",
        )]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.current_phase_name, "Live State");
        assert_eq!(state.current_plan, "0.3");
        // current_phase prefers the explicit frontmatter phase name.
        assert_eq!(state.current_phase, "Live State");
        // project_root is the directory containing .planning/
        assert_eq!(state.project_root, td.path());
    }

    #[test]
    fn test_frontmatter_current_phase_number_fallback() {
        // Only current_phase present (no name) → `Phase {n}`.
        let td = make_planning(&[(
            "STATE.md",
            "---\nstatus: executing\ncurrent_phase: 14\n---\n",
        )]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.current_phase, "Phase 14");
    }

    #[test]
    fn test_progress_table_overrides_frontmatter_counts() {
        let state_md = "---\nstatus: executing\nprogress:\n  total_phases: 9\n  completed_phases: 9\n  total_plans: 20\n  completed_plans: 20\n---\n";
        let roadmap = "## Progress\n\n| Phase | Plans Complete | Status | Completed |\n| --- | --- | --- | --- |\n| 1. Alpha | 2/2 | Complete | ✅ |\n| 2. Beta | 1/3 | In Progress | |\n";
        let td = make_planning(&[("STATE.md", state_md), ("ROADMAP.md", roadmap)]);
        let state = parse_project_state(&td.path().join(".planning"));
        // The ## Progress table is authoritative and overrides the frontmatter.
        assert_eq!(state.total_phases, 2);
        assert_eq!(state.completed_phases, 1);
        assert_eq!(state.total_plans, 5);
        assert_eq!(state.completed_plans, 3);
    }

    /// Quick 260923-md1: ROADMAP.md milestones reach `ProjectState`.
    #[test]
    fn parse_project_state_reads_roadmap_milestones() {
        let roadmap = "# Roadmap\n\n## Milestones\n\n\
            - 🚧 **v2.0 Next** - Phases 1-3 (in progress)\n\n\
            ## Phases\n\n- [ ] **Phase 1: Alpha** - a\n";
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            ("ROADMAP.md", roadmap),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.milestones.len(), 1);
        assert_eq!(state.milestones[0].label.as_raw_for_logic_only(), "v2.0 Next");
        assert!(state.milestones[0].contains("1"));
    }

    /// Phase 24-01: ttbook's `#### Build phase N` headings reach
    /// `planned_phases`, and the GSD-facing `phases` list is untouched.
    #[test]
    fn parse_project_state_reads_planned_build_phases() {
        let td = make_planning(&[
            ("STATE.md", include_str!("../../tests/fixtures/roadmaps/ttbook-STATE.md")),
            ("ROADMAP.md", include_str!("../../tests/fixtures/roadmaps/ttbook-ROADMAP.md")),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        let gsd: Vec<&str> = state.phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(gsd, ["8", "9", "10", "11", "12", "13"]);
        let planned: Vec<&str> = state.planned_phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(planned, ["14", "15", "16", "17", "18"]);
    }

    /// quick 260926-fi9: closed-milestone collapse lines reach
    /// `shipped_phases` only — never `phases`, disk inference, the current
    /// phase or the phase count.
    #[test]
    fn parse_project_state_keeps_shipped_phases_out_of_the_gsd_facing_state() {
        let td = make_planning(&[
            (
                "STATE.md",
                "---\nmilestone: v2.0\nstatus: executing\ncurrent_phase: 13\n---\n# Project State\n",
            ),
            (
                "ROADMAP.md",
                include_str!("../../tests/fixtures/roadmap-shapes/v1-era-ROADMAP.md"),
            ),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        let gsd: Vec<&str> = state.phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(gsd, ["12", "13", "13.1", "14", "16"]);
        let shipped: Vec<&str> = state.shipped_phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(
            shipped,
            ["1", "2", "2.1", "3", "4", "5", "6", "07", "08", "9", "10", "11"]
        );
        assert_eq!(state.current_phase_number, phase_num::PhaseNum::parse("12"));
        for p in &state.shipped_phases {
            assert!(
                !state.phase_disk_statuses.contains_key(&p.number),
                "shipped {} was disk-inferred",
                p.number
            );
        }
        assert_eq!(state.phase_progress().total, 5);
    }

    /// quick 260926-fi9 (T-fi9-03): after GSD's `--reset-phase-numbers`, an
    /// archived `[x] Phase 1` never completes the current `Phase 1`.
    #[test]
    fn a_renumbered_current_phase_is_not_completed_by_its_shipped_namesake() {
        let td = make_planning(&[
            ("STATE.md", "---\nmilestone: v2.0\nstatus: executing\n---\n# Project State\n"),
            ("ROADMAP.md", RENUMBERED_ROADMAP),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.phases.len(), 1);
        assert_eq!(state.phases[0].name, "New One");
        assert!(!state.phases[0].completed);
        assert_eq!(state.shipped_phases.len(), 1);
        assert_eq!(state.shipped_phases[0].name, "Old One");
        assert!(state.shipped_phases[0].completed);
    }

    /// A roadmap renumbered by GSD's `--reset-phase-numbers`: the shipped
    /// v1.0 and the current v2.0 both have a `Phase 1`.
    const RENUMBERED_ROADMAP: &str = "# Roadmap\n\n## Milestones\n\n\
        - \u{2705} **v1.0 Old** - Phases 1-1 (shipped 2026-01-01)\n\
        - \u{1F6A7} **v2.0 New** - Phases 1-1 (in progress)\n\n\
        ## Phases\n\n<details>\n\
        <summary>\u{2705} v1.0 Old (Phases 1-1) - SHIPPED 2026-01-01</summary>\n\n\
        - [x] Phase 1: Old One (2/2 plans) \u{2014} completed 2026-01-01\n\n</details>\n\n\
        - [ ] **Phase 1: New One** - fresh\n\n## Phase Details\n\n### Phase 1: New One\n\n\
        **Depends on**: Nothing\n";

    /// Phase 24-01: sentriq writes `milestone_name` AFTER the `progress:`
    /// block; it still reaches `ProjectState` for the synthetic band.
    #[test]
    fn parse_project_state_reads_milestone_name_after_progress() {
        let td = make_planning(&[
            ("STATE.md", include_str!("../../tests/fixtures/roadmaps/sentriq-STATE.md")),
            ("ROADMAP.md", include_str!("../../tests/fixtures/roadmaps/sentriq-ROADMAP.md")),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.milestone, "v0.12");
        assert_eq!(
            state.milestone_name.as_ref().map(|m| m.as_raw_for_logic_only()),
            Some("Actuation Routines")
        );

        let td = make_planning(&[("STATE.md", "---\nstatus: executing\nmilestone_name: \n---\n")]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.milestone_name, None, "an empty value is no name");
    }

    /// Phase 24-01: goals reach `ProjectState`, keyed by phase.
    #[test]
    fn parse_project_state_reads_phase_goals() {
        let td = make_planning(&[
            ("STATE.md", include_str!("../../tests/fixtures/roadmaps/daily-vow-STATE.md")),
            ("ROADMAP.md", include_str!("../../tests/fixtures/roadmaps/daily-vow-ROADMAP.md")),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        let goal = state.phase_goals.get("23").expect("phase 23 has a goal");
        assert!(goal.as_raw_for_logic_only().starts_with("(sanitised) The user can see"));
        assert_eq!(state.phase_goals.len(), 6);
    }

    /// quick 260926-gtl: a gsd-core 1.15.0 letter-suffixed phase id (#2128 /
    /// #4830) reads end-to-end — phase row, plan counts, dependency, goal key.
    #[test]
    fn letter_suffixed_phase_reads_end_to_end() {
        let roadmap = "# Roadmap\n\n## Phases\n\n\
            - [ ] **Phase 12A: Twelve** - twelve\n\
            - [ ] **Phase 23A.1.2: Letter** - letter\n\n\
            ## Phase Details\n\n\
            ### Phase 12A: Twelve\n\n\
            **Goal**: twelve goal\n\n\
            ### Phase 23A.1.2: Letter\n\n\
            **Goal**: letter-suffixed goal\n\
            **Depends on**: Phase 12A\n\n\
            Plans:\n\
            - [x] 23A.1.2-01-PLAN.md — one\n";
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            ("ROADMAP.md", roadmap),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        let phase = state
            .roadmap_phase("23A.1.2")
            .expect("the letter-suffixed phase is a roadmap row");
        assert_eq!(phase.name, "Letter");
        assert_eq!((phase.total_plans, phase.completed_plans), (1, 1));
        assert_eq!(phase.depends_on, ["12A"]);
        assert_eq!(
            state
                .phase_goals
                .get("23A.1.2")
                .map(|g| g.as_raw_for_logic_only()),
            Some("letter-suffixed goal")
        );
    }

    #[test]
    fn test_no_progress_table_uses_frontmatter_counts() {
        let state_md = "---\nstatus: executing\nprogress:\n  total_phases: 4\n  completed_phases: 2\n  total_plans: 8\n  completed_plans: 5\n---\n";
        let roadmap = "# Roadmap\n\n- [ ] **Phase 1: Alpha** - no progress table\n";
        let td = make_planning(&[("STATE.md", state_md), ("ROADMAP.md", roadmap)]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.total_phases, 4);
        assert_eq!(state.completed_phases, 2);
        assert_eq!(state.total_plans, 8);
        assert_eq!(state.completed_plans, 5);
    }

    #[test]
    fn test_async_jobs_sets_external_job_waiting() {
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            ("async-jobs/job1.json", "{\"id\":\"job1\"}"),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.external_job_waiting);
    }

    #[test]
    fn test_no_async_jobs_means_not_waiting() {
        let td = make_planning(&[("STATE.md", "---\nstatus: executing\n---\n")]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(!state.external_job_waiting);
    }

    #[test]
    fn test_handoff_md_non_empty_sets_paused() {
        // UI-SPEC UIFIX-01 row 1: non-empty HANDOFF.md after trim → paused,
        // context is the first non-empty non-heading line.
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            (
                "HANDOFF.md",
                "# Handoff\n\nPick up at plan 14-02\n\nmore detail\n",
            ),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.paused);
        assert_eq!(state.pause_context.as_deref(), Some("Pick up at plan 14-02"));
    }

    #[test]
    fn test_handoff_json_non_empty_sets_paused_with_context() {
        // UI-SPEC UIFIX-01 row 2: HANDOFF.json `next_action` becomes the context.
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            (
                "HANDOFF.json",
                "{\"next_action\":\"Run /gsd-execute-phase 14\"}",
            ),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.paused);
        assert_eq!(
            state.pause_context.as_deref(),
            Some("Run /gsd-execute-phase 14")
        );
    }

    #[test]
    fn test_handoff_md_whitespace_only_is_not_paused() {
        // UI-SPEC UIFIX-01 row 3: a whitespace-only file is not a pause signal.
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            ("HANDOFF.md", "   \n\t\n  \n"),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(!state.paused);
        assert!(state.pause_context.is_none());
    }

    #[test]
    fn test_no_handoff_file_is_not_paused() {
        // UI-SPEC UIFIX-01 row 4: no HANDOFF file at all.
        let td = make_planning(&[("STATE.md", "---\nstatus: executing\n---\n")]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(!state.paused);
        assert!(state.pause_context.is_none());
    }

    #[test]
    fn test_handoff_json_invalid_is_paused_without_context() {
        // UI-SPEC UIFIX-01 row 8: the badge never depends on JSON parsing.
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            ("HANDOFF.json", "{not valid json at all"),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.paused);
        assert!(state.pause_context.is_none());
    }

    #[test]
    fn test_all_phases_complete_is_intermediate() {
        // `All phases complete` (ADR-2207 intermediate) must NOT render a
        // premature `<milestone> Complete`.
        let state_md = "---\nstatus: All phases complete\nmilestone: v1.5.0\nprogress:\n  total_phases: 5\n  completed_phases: 5\n---\n";
        let td = make_planning(&[("STATE.md", state_md)]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.current_phase, "All phases complete");
    }

    #[test]
    fn test_milestone_terminal_renders_complete() {
        let state_md = "---\nstatus: v1.5.0 milestone complete\nmilestone: v1.5.0\nprogress:\n  total_phases: 5\n  completed_phases: 5\n---\n";
        let td = make_planning(&[("STATE.md", state_md)]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.current_phase, "v1.5.0 Complete");
    }

    #[test]
    fn test_legacy_project_preserves_prior_behavior() {
        // No new artifacts: frontmatter counts, stopped_at heuristic, empty new
        // fields, and no external job — all as before.
        let state_md = "---\nstatus: planning\nmilestone: v1.0\nstopped_at: Phase 1 context gathered\nprogress:\n  total_phases: 4\n  completed_phases: 0\n  total_plans: 0\n  completed_plans: 0\n---\n";
        let td = make_planning(&[("STATE.md", state_md)]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.status, "planning");
        assert_eq!(state.total_phases, 4);
        assert_eq!(state.completed_phases, 0);
        assert_eq!(state.current_phase, "Phase 1 context gathered");
        assert_eq!(state.current_phase_name, "");
        assert_eq!(state.current_plan, "");
        assert!(!state.external_job_waiting);
        // project_root and last_activity are populated for any project.
        assert_eq!(state.project_root, td.path());
        assert!(state.last_activity.is_some());
    }

    #[test]
    fn test_workstreams_populated_on_project_state() {
        let td = make_planning(&[
            ("STATE.md", "---\nstatus: executing\n---\n"),
            ("workstreams/alpha/STATE.md", "---\nstatus: executing\n---\n"),
            ("workstreams/beta/STATE.md", "---\nstatus: planning\n---\n"),
            ("active-workstream", "alpha\n"),
        ]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert_eq!(state.workstreams.len(), 2);
        // Bounded recursion: each workstream sub-state has no nested workstreams.
        assert!(state.workstreams.iter().all(|w| w.state.workstreams.is_empty()));
        // Active flag flows through from the active-workstream pointer.
        assert!(state.workstreams.iter().find(|w| w.name == "alpha").unwrap().active);
    }

    #[test]
    fn test_flat_project_has_empty_workstreams() {
        let td = make_planning(&[("STATE.md", "---\nstatus: executing\n---\n")]);
        let state = parse_project_state(&td.path().join(".planning"));
        assert!(state.workstreams.is_empty());
    }
}
