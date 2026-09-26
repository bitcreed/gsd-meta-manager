//! The per-project wave model (AGENT-02): which wave a phase's run is in, and
//! which of its plans are running, queued, finished or done.
//!
//! **Pure.** Nothing here reads a file, runs git or looks at a clock. Every fact
//! it needs arrives already gathered: the agent rows from the scan
//! ([`super::scan_project_with`] attributes each row to a plan and stats its
//! worktree SUMMARY while it is scanning), and the plan files' own waves and
//! summaries from the disk inference the refresh already computed
//! ([`DiskInference::plan_waves`], [`DiskInference::summarized_plans`]).
//! [`derive`] therefore runs in the handler on the scan cadence and never at
//! render time — the doctrine `crate::state_reader::plan_waves` states for the
//! wave grouping it reuses.
//!
//! **Frontmatter `wave:` is the only authority on waves** (D-A10). GSD's
//! `waves.json` and the orchestrator's `WAVE_WORKTREE_MANIFEST` are never
//! consulted (D-A09), and a plan's NUMBER never orders or groups waves (D-A02):
//! plan 13-02 may run in wave 3 while 13-09 runs in wave 1.
//!
//! **Agent text is never a path.** A plan id captured from a description,
//! branch, commit subject or ledger name becomes a join key only after
//! [`PlanRef::from_id`] accepts it, which admits digits, one optional decimal
//! tail and one dash — nothing that can walk a directory.
//!
//! **Transparency over flattery.** A plan whose SUMMARY sits only in its
//! agent's worktree is [`PlanState::Finished`], never [`PlanState::Done`]: the
//! orchestrator has not merged it, and the model always keeps the two apart.

use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::SystemTime;

use regex::Regex;

use super::adapters::ChildAgent;
use super::fixers::FixerEstimate;
use super::{AgentLiveness, AgentRow, ProjectAgents};
use crate::state_reader::disk_status::{plan_index, DiskInference};
use crate::state_reader::phase_num::{same_phase, PhaseNum};
use crate::state_reader::ProjectState;

/// A validated plan identity: the pad-insensitive join key every attribution
/// tier resolves to. `13-1` and `13-01` are one `PlanRef`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanRef {
    pub phase: PhaseNum,
    pub plan: u32,
}

/// `^[0-9]+(\.[0-9]+)?(-[0-9]+)?$` — the only shape a captured id may have
/// before it becomes a join key (D-C10). [`PlanRef::from_id`] additionally
/// requires the dash.
fn plan_id_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[0-9]+(\.[0-9]+)?(-[0-9]+)?$").expect("static regex"))
}

/// Tier 1: GSD's own dispatch template, `Execute plan {plan} of phase {phase}`.
fn plan_of_phase_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\bplan (\S+) of phase (\S+)").expect("static regex"))
}

/// Tier 2: `plan 04-06`, `plans 05-02 and 05-03` (the first one wins).
fn plan_dashed_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)\bplans? (\d+(?:\.\d+)?-\d+)\b").expect("static regex"))
}

/// Tier 4: a conventional-commit scope naming a plan, `feat(13-17): …`.
fn commit_scope_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-z]+\((\d+(?:\.\d+)?-\d+)\):").expect("static regex"))
}

impl PlanRef {
    /// GSD's spelling: `13-02`, `07.1-03` — the phase padded, the plan two digits.
    pub fn label(&self) -> String {
        format!("{}-{:02}", self.phase.padded(), self.plan)
    }

    /// Parse a captured id. Only `^[0-9]+(\.[0-9]+)?-[0-9]+$` is accepted; the
    /// numbers then go through `disk_status::plan_index`, the same key the
    /// summary pairing uses, so `13-1` equals `13-01`. Anything else — a slug,
    /// a path, a bare plan number, an overflowing number — is `None`.
    pub fn from_id(id: &str) -> Option<PlanRef> {
        if !id.contains('-') || !plan_id_re().is_match(id) {
            return None;
        }
        let (phase, plan) = plan_index(id)?;
        Some(PlanRef { phase, plan })
    }

    /// The plan a `*-PLAN.md` / `*-SUMMARY.md` stem names (slug allowed).
    fn from_stem(stem: &str) -> Option<PlanRef> {
        let (phase, plan) = plan_index(stem)?;
        Some(PlanRef { phase, plan })
    }
}

/// The one state each plan of the active phase is in (D-C10, amended by
/// RESEARCH Pattern 5 with `Finished`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlanState {
    /// Its SUMMARY is paired in the main worktree.
    Done,
    /// Not done in main, but its agent's worktree holds its SUMMARY or its
    /// agent reads `Finished`: complete, not yet merged.
    Finished,
    /// A `Live` or `Idle` agent is attributed to it.
    Running,
    /// Every agent attributed to it is `Stalled`.
    Stalled,
    /// Anything else.
    Queued,
}

/// One wave's plan-state counts.
///
/// `wave` is `None` for the trailing bucket of plans with no readable `wave:`,
/// which is shown but is never the current wave and never the denominator.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WaveRow {
    pub wave: Option<u32>,
    pub done: u32,
    pub finished: u32,
    pub running: u32,
    pub stalled: u32,
    pub queued: u32,
    pub current: bool,
}

impl WaveRow {
    /// `w2` for a numbered wave, `w?` for the unknown bucket — the stored
    /// number, never a position (`PlanWave::label`'s rule).
    pub fn label(&self) -> String {
        match self.wave {
            Some(n) => format!("w{n}"),
            None => "w?".to_string(),
        }
    }
}

/// One project's agents joined with its active phase's waves, as of one scan.
///
/// Everything a renderer needs, and nothing it has to compute: the dashboard
/// (25-04) and the Agents sub-view (25-05) read these fields and
/// [`AgentView::summary_forms`] only.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentView {
    pub active_phase: Option<PhaseNum>,
    /// The lowest numbered wave holding a plan not done in main.
    pub current_wave: Option<u32>,
    /// The highest numbered wave; the `w?` bucket never counts.
    pub max_wave: Option<u32>,
    /// `DiskInference.plan_count` — surviving plans only.
    pub plan_total: u32,
    pub done: u32,
    pub finished: u32,
    pub running: u32,
    pub stalled: u32,
    pub queued: u32,
    pub waves: Vec<WaveRow>,
    /// Agent rows in display order (liveness, then plan, then path).
    pub agents: Vec<AgentRow>,
    pub worktreeless: Vec<ChildAgent>,
    pub scanned_at: Option<SystemTime>,
    /// The code-review fix-run estimate, copied from
    /// [`ProjectAgents::fixer_estimate`]; drives the fixer summary forms.
    pub fixers: Option<FixerEstimate>,
}

/// Strip the punctuation a sentence wraps around a captured token.
fn strip_trailing_punct(s: &str) -> &str {
    s.trim_end_matches(['.', ',', ';', ':', ')'])
}

/// Attribute one agent to a plan (D-C10), trying each source in priority order:
///
/// 1. the description, `plan X of phase Y` — a digits-only `X` means `Y-X`;
/// 2. the description, `plan(s) N-M`;
/// 3. the `p{plan}` component of a Codex-style branch (D-A06);
/// 4. the plan scope of the worktree's own commit subjects;
/// 5. GSD's `gsd-plan-head-before-*` ledger (confirmation only; often absent).
///
/// The first tier that yields a valid [`PlanRef`] wins; a tier whose capture
/// fails validation falls through to the next rather than ending the search.
pub fn attribute(
    description: Option<&str>,
    branch_plan: Option<&str>,
    commit_scope: Option<&str>,
    ledger_plan: Option<&str>,
) -> Option<PlanRef> {
    if let Some(desc) = description {
        if let Some(caps) = plan_of_phase_re().captures(desc) {
            let plan = strip_trailing_punct(&caps[1]);
            let phase = strip_trailing_punct(&caps[2]);
            let id = if !plan.is_empty() && plan.bytes().all(|b| b.is_ascii_digit()) {
                format!("{phase}-{plan}")
            } else {
                plan.to_string()
            };
            if let Some(found) = PlanRef::from_id(&id) {
                return Some(found);
            }
        }
        if let Some(caps) = plan_dashed_re().captures(desc) {
            if let Some(found) = PlanRef::from_id(&caps[1]) {
                return Some(found);
            }
        }
    }
    [branch_plan, commit_scope, ledger_plan]
        .into_iter()
        .flatten()
        .find_map(PlanRef::from_id)
}

/// The plan named by the first subject carrying a `type(N-M):` scope, newest
/// first as `git log` lists them. `None` when no subject names a plan
/// (`fix: typo`, `chore(release): v1.8.0`).
pub fn commit_scope_plan(subjects: &[String]) -> Option<String> {
    subjects
        .iter()
        .find_map(|s| commit_scope_re().captures(s).map(|c| c[1].to_string()))
}

/// Display rank: what is happening now first, what ended last.
fn liveness_rank(liveness: AgentLiveness) -> u8 {
    match liveness {
        AgentLiveness::Live => 0,
        AgentLiveness::Idle => 1,
        AgentLiveness::Finished => 2,
        AgentLiveness::Stalled => 3,
        AgentLiveness::Unknown => 4,
        AgentLiveness::Ended => 5,
    }
}

/// The total display order of agent rows: liveness rank, then plan (an
/// attributed row before an unattributed one), then path — which is unique per
/// worktree, so two derivations of one scan always order identically.
fn display_key(row: &AgentRow) -> (u8, bool, Option<&PlanRef>, &std::path::Path) {
    (
        liveness_rank(row.liveness),
        row.plan.is_none(),
        row.plan.as_ref(),
        row.path.as_path(),
    )
}

/// Whether a row's agent is doing, or has just done, the work.
fn is_active_liveness(liveness: AgentLiveness) -> bool {
    matches!(
        liveness,
        AgentLiveness::Live | AgentLiveness::Idle | AgentLiveness::Finished
    )
}

/// The disk inference for `phase`, pad-insensitively. `disk_status_for`
/// resolves through the roadmap; a map populated with another spelling of the
/// same phase (`5` for `05`) is found by the sorted key scan, so the choice is
/// deterministic whatever the `HashMap`'s order.
fn disk_for<'a>(state: &'a ProjectState, phase: &PhaseNum) -> Option<&'a DiskInference> {
    let padded = phase.padded();
    state.disk_status_for(&padded).or_else(|| {
        let mut keys: Vec<&String> = state
            .phase_disk_statuses
            .keys()
            .filter(|k| same_phase(k, &padded))
            .collect();
        keys.sort();
        keys.first().and_then(|k| state.phase_disk_statuses.get(*k))
    })
}

/// The state of one plan, from its main-worktree SUMMARY and the rows
/// attributed to it.
fn plan_state(done_in_main: bool, rows: &[&AgentRow]) -> PlanState {
    if done_in_main {
        return PlanState::Done;
    }
    if rows
        .iter()
        .any(|r| r.summary_in_worktree || r.liveness == AgentLiveness::Finished)
    {
        return PlanState::Finished;
    }
    if rows
        .iter()
        .any(|r| matches!(r.liveness, AgentLiveness::Live | AgentLiveness::Idle))
    {
        return PlanState::Running;
    }
    if !rows.is_empty() && rows.iter().all(|r| r.liveness == AgentLiveness::Stalled) {
        return PlanState::Stalled;
    }
    PlanState::Queued
}

/// Add one plan's state to a set of counts.
fn tally(row: &mut WaveRow, state: PlanState) {
    match state {
        PlanState::Done => row.done += 1,
        PlanState::Finished => row.finished += 1,
        PlanState::Running => row.running += 1,
        PlanState::Stalled => row.stalled += 1,
        PlanState::Queued => row.queued += 1,
    }
}

/// Join one project's scan with its parsed state into an [`AgentView`].
///
/// * **Active phase:** the phase most attributed `Live`, `Idle`, `Finished` or
///   `Stalled` rows name, ties to the higher phase; with none,
///   `ProjectState::active_phase_number()`.
/// * **Plan universe:** every plan of that phase's `plan_waves` (the `w?`
///   bucket included). A phase whose plans carry no `wave:` has no waves to
///   draw; its universe is then the attributed plans plus the summarized ones.
/// * **Plan state:** see [`PlanState`]; each plan has exactly one.
/// * **Current wave:** the lowest numbered wave with a plan not `Done`.
///
/// Deterministic: two derivations of the same inputs compare equal.
pub fn derive(agents: &ProjectAgents, state: &ProjectState) -> AgentView {
    let mut by_phase: BTreeMap<&PhaseNum, u32> = BTreeMap::new();
    for row in &agents.rows {
        let counted = matches!(
            row.liveness,
            AgentLiveness::Live
                | AgentLiveness::Idle
                | AgentLiveness::Finished
                | AgentLiveness::Stalled
        );
        if let (true, Some(plan)) = (counted, &row.plan) {
            *by_phase.entry(&plan.phase).or_default() += 1;
        }
    }
    // BTreeMap iterates ascending, and `max_by_key` keeps the LAST maximum, so
    // a tie goes to the higher phase.
    let active_phase = by_phase
        .iter()
        .max_by_key(|(_, count)| **count)
        .map(|(phase, _)| (*phase).clone())
        .unwrap_or_else(|| state.active_phase_number());

    let empty = DiskInference::default();
    let di = disk_for(state, &active_phase).unwrap_or(&empty);

    let rows_for = |plan: &PlanRef| -> Vec<&AgentRow> {
        agents
            .rows
            .iter()
            .filter(|r| r.plan.as_ref() == Some(plan))
            .collect()
    };
    let is_done = |stem: &str| di.summarized_plans.iter().any(|s| s == stem);
    let state_of = |stem: &str| {
        let rows = PlanRef::from_stem(stem)
            .map(|p| rows_for(&p))
            .unwrap_or_default();
        plan_state(is_done(stem), &rows)
    };

    let mut totals = WaveRow::default();
    let mut waves: Vec<WaveRow> = Vec::new();
    if di.plan_waves.is_empty() {
        // No wave metadata: count the plans something is known about, keyed by
        // PlanRef so a summarized `13-01-slug` and an agent on `13-01` are one.
        let mut universe: BTreeMap<Result<PlanRef, String>, bool> = BTreeMap::new();
        for stem in &di.summarized_plans {
            let key = PlanRef::from_stem(stem).ok_or_else(|| stem.clone());
            universe.insert(key, true);
        }
        for plan in agents.rows.iter().filter_map(|r| r.plan.as_ref()) {
            if plan.phase == active_phase {
                universe.entry(Ok(plan.clone())).or_insert(false);
            }
        }
        for (key, done) in &universe {
            let rows = key.as_ref().map(rows_for).unwrap_or_default();
            tally(&mut totals, plan_state(*done, &rows));
        }
    } else {
        for wave in &di.plan_waves {
            let mut row = WaveRow {
                wave: wave.wave,
                ..WaveRow::default()
            };
            for stem in &wave.plans {
                let s = state_of(stem);
                tally(&mut row, s);
                tally(&mut totals, s);
            }
            waves.push(row);
        }
    }

    let current_wave = waves
        .iter()
        .filter(|w| w.wave.is_some() && w.finished + w.running + w.stalled + w.queued > 0)
        .filter_map(|w| w.wave)
        .min();
    let max_wave = waves.iter().filter_map(|w| w.wave).max();
    for w in &mut waves {
        w.current = w.wave.is_some() && w.wave == current_wave;
    }

    let mut rows = agents.rows.clone();
    rows.sort_by(|a, b| display_key(a).cmp(&display_key(b)));

    AgentView {
        active_phase: Some(active_phase),
        current_wave,
        max_wave,
        plan_total: di.plan_count,
        done: totals.done,
        finished: totals.finished,
        running: totals.running,
        stalled: totals.stalled,
        queued: totals.queued,
        waves,
        agents: rows,
        worktreeless: agents.worktreeless.clone(),
        scanned_at: agents.scanned_at,
        fixers: agents.fixer_estimate.clone(),
    }
}

impl AgentView {
    /// Whether anything is under way: a row `Live`, `Idle` or `Finished`, or
    /// any worktree-less agent (the scan keeps only live ones).
    pub fn is_active(&self) -> bool {
        self.agents.iter().any(|r| is_active_liveness(r.liveness)) || !self.worktreeless.is_empty()
    }

    /// The dashboard Status-cell summary, widest form first (RESEARCH
    /// Pattern 6). The renderer takes the first form that fits its cell.
    ///
    /// Reads only this view's fields, so a UI test can build the view as a
    /// literal. `·` is U+00B7, one cell wide.
    ///
    /// `P13` is the FIRST segment dropped [inferred — RESEARCH A3, deviating
    /// from D-C14's "drop from the right"]: the Phase column beside the cell
    /// already shows the phase, and at the common 13-cell width a right-drop
    /// would keep `P13 · w2/11` and hide the running count, which is the point.
    ///
    /// Executor mode (the active phase has plans, and some row is attributed to
    /// one of them) yields the wave ladder; otherwise fixer mode (an estimate
    /// with at least one active fixer, [`Self::fixer_forms`]) yields the fixer
    /// ladder; otherwise an active view yields `N agents`; an inactive view
    /// whose rows include stalled ones yields `N stalled`; anything else yields
    /// no forms and the cell is left alone.
    pub fn summary_forms(&self) -> Vec<String> {
        if !self.is_active() {
            let stalled = self
                .agents
                .iter()
                .filter(|r| r.liveness == AgentLiveness::Stalled)
                .count();
            return if stalled > 0 {
                vec![format!("{stalled} stalled")]
            } else {
                Vec::new()
            };
        }

        let executor_mode = match &self.active_phase {
            Some(phase) => {
                self.plan_total > 0
                    && self
                        .agents
                        .iter()
                        .any(|r| r.plan.as_ref().is_some_and(|p| &p.phase == phase))
            }
            None => false,
        };
        if !executor_mode {
            if let Some(forms) = self.fixer_forms() {
                return forms;
            }
            let n = self
                .agents
                .iter()
                .filter(|r| is_active_liveness(r.liveness))
                .count()
                + self.worktreeless.len();
            return vec![if n == 1 {
                "1 agent".to_string()
            } else {
                format!("{n} agents")
            }];
        }

        let p = self
            .active_phase
            .as_ref()
            .map(PhaseNum::padded)
            .unwrap_or_default();
        let r = self.running;
        let d = self.done + self.finished;
        let t = self.plan_total;
        match (self.current_wave, self.max_wave) {
            (Some(c), Some(m)) => vec![
                format!("P{p} \u{b7} w{c}/{m} \u{b7} {r} run \u{b7} {d}/{t} done"),
                format!("w{c}/{m} \u{b7} {r} run \u{b7} {d}/{t} done"),
                format!("w{c}/{m} \u{b7} {r} run \u{b7} {d}/{t}"),
                format!("w{c}/{m} \u{b7} {r} run"),
                format!("w{c}/{m} {r}run"),
                format!("{r}run"),
            ],
            _ => vec![
                format!("P{p} \u{b7} {r} run \u{b7} {d}/{t} done"),
                format!("{r} run \u{b7} {d}/{t} done"),
                format!("{r} run \u{b7} {d}/{t}"),
                format!("{r}run"),
            ],
        }
    }

    /// The fixer ladder (AGENT-07, D-C12, D-C14), widest first, or `None`
    /// when there is no estimate or it counts no fixer.
    ///
    /// With both counts: `3 fixers · ~5/48 fixed`, `3 fixers · ~5/48`,
    /// `3fix ~5/48`, `3fix`. Without: `3 fixers`, `3fix`. `fixer` is singular
    /// for one. The count is an estimate and every form carrying it says so
    /// with `~`; the widest keeps the word `fixed` (T-25-26).
    fn fixer_forms(&self) -> Option<Vec<String>> {
        let est = self.fixers.as_ref().filter(|e| e.fixers >= 1)?;
        let n = est.fixers;
        let noun = if n == 1 { "fixer" } else { "fixers" };
        Some(match (est.fixed, est.total) {
            (Some(f), Some(t)) => vec![
                format!("{n} {noun} \u{b7} ~{f}/{t} fixed"),
                format!("{n} {noun} \u{b7} ~{f}/{t}"),
                format!("{n}fix ~{f}/{t}"),
                format!("{n}fix"),
            ],
            _ => vec![format!("{n} {noun}"), format!("{n}fix")],
        })
    }
}

// Pure tests only: every input is built by hand — no git, no file, no process.
// This directory is not on the spawn allowlist, and that holds for test code too.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::plan_waves::PlanWave;
    use crate::text::Untrusted;
    use ratatui::text::Line;
    use std::collections::HashMap;
    use std::path::PathBuf;

    const DOT: &str = "\u{b7}";

    fn pr(id: &str) -> PlanRef {
        PlanRef::from_id(id).unwrap_or_else(|| panic!("{id} is a valid plan id"))
    }

    fn row(path: &str, liveness: AgentLiveness, plan: Option<&str>) -> AgentRow {
        AgentRow {
            path: PathBuf::from(path),
            liveness,
            plan: plan.map(pr),
            ..AgentRow::default()
        }
    }

    fn agents(rows: Vec<AgentRow>) -> ProjectAgents {
        ProjectAgents {
            rows,
            ..ProjectAgents::default()
        }
    }

    fn wave(n: Option<u32>, plans: &[String]) -> PlanWave {
        PlanWave {
            wave: n,
            plans: plans.to_vec(),
        }
    }

    /// `13-{first:02}` ..= `13-{last:02}`.
    fn ids(first: u32, last: u32) -> Vec<String> {
        (first..=last).map(|n| format!("13-{n:02}")).collect()
    }

    fn strs(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    /// A `ProjectState` whose only disk inference is `phase`'s.
    fn state(
        phase: &str,
        plan_waves: Vec<PlanWave>,
        summarized: &[String],
        plan_count: u32,
    ) -> ProjectState {
        let di = DiskInference {
            plan_waves,
            summarized_plans: summarized.to_vec(),
            plan_count,
            summary_count: summarized.len() as u32,
            ..DiskInference::default()
        };
        ProjectState {
            phase_disk_statuses: HashMap::from([(phase.to_string(), di)]),
            ..ProjectState::default()
        }
    }

    /// Phase 13 as observed: 35 plans; wave 1's 8 all done in main; wave 2's
    /// 14 plans (13-09..13-22); waves 3..11 holding the other 13 (13-23..13-27
    /// in wave 3, one plan in each of waves 4..11).
    fn phase_13_state() -> ProjectState {
        let mut waves = vec![wave(Some(1), &ids(1, 8)), wave(Some(2), &ids(9, 22))];
        waves.push(wave(Some(3), &ids(23, 27)));
        for (w, plan) in (4..=11).zip(28..=35) {
            waves.push(wave(Some(w), &ids(plan, plan)));
        }
        state("13", waves, &ids(1, 8), 35)
    }

    /// One Live executor on each of 13-09..13-21: thirteen of wave 2's fourteen.
    fn thirteen_live_executors() -> Vec<AgentRow> {
        (9..=21)
            .map(|n| {
                row(
                    &format!("/wt/agent-{n:02}"),
                    AgentLiveness::Live,
                    Some(&format!("13-{n:02}")),
                )
            })
            .collect()
    }

    fn width(form: &str) -> usize {
        Line::from(form).width()
    }

    // --- attribution ------------------------------------------------------

    #[test]
    fn attribution_tiers_in_priority_order() {
        let a = |d, b, c, l| attribute(d, b, c, l).map(|p| p.label());
        let some = |s: &str| Some(s.to_string());
        assert_eq!(
            a(Some("Execute plan 13-13 of phase 13"), None, None, None),
            some("13-13")
        );
        assert_eq!(
            a(Some("Close out plan 04-03"), None, None, None),
            some("04-03")
        );
        assert_eq!(
            a(Some("Execute plans 05-02 and 05-03"), None, None, None),
            some("05-02")
        );
        assert_eq!(
            a(Some("Execute 260923-e9d: quick"), Some("13-02"), None, None),
            some("13-02"),
            "a quick-task description falls through to the branch"
        );
        assert_eq!(a(None, None, Some("13-03"), None), some("13-03"));
        assert_eq!(a(None, None, None, Some("13-04")), some("13-04"));
        assert_eq!(
            a(
                Some("Execute plan 13-03 of phase 13"),
                Some("13-02"),
                Some("13-05"),
                Some("13-06")
            ),
            some("13-03"),
            "the description outranks the branch"
        );
        assert_eq!(
            a(None, Some("13-02"), Some("13-05"), Some("13-06")),
            some("13-02"),
            "the branch outranks the commit scope"
        );
        assert_eq!(
            a(None, None, Some("13-05"), Some("13-06")),
            some("13-05"),
            "the commit scope outranks the ledger"
        );
        assert_eq!(a(None, None, None, None), None);
    }

    #[test]
    fn a_bare_plan_number_joins_its_phase() {
        let a = |d: &str| attribute(Some(d), None, None, None).map(|p| p.label());
        assert_eq!(a("Execute plan 22 of phase 21"), Some("21-22".to_string()));
        assert_eq!(
            a("Execute plan 7 of phase 07.1."),
            Some("07.1-07".to_string()),
            "trailing punctuation stripped, decimal phase kept"
        );
        assert_eq!(
            a("execute PLAN 3 of Phase 5, then stop"),
            Some("05-03".to_string())
        );
    }

    #[test]
    fn hostile_or_malformed_ids_never_attribute() {
        assert_eq!(
            attribute(Some("plan ../../x of phase 13"), None, None, None),
            None
        );
        assert_eq!(
            attribute(Some("plan ../../x of phase 13"), Some("13-07"), None, None)
                .map(|p| p.label()),
            Some("13-07".to_string()),
            "a hostile capture falls through to the next tier"
        );
        assert_eq!(
            attribute(Some("plan 13-02x of phase 13"), None, None, None),
            None
        );
        for bad in [
            "../13-02",
            "13-02/../../etc",
            "13-02; rm -rf /",
            "13-02-slug",
            "13",
            "7",
            "-13-02",
            "13--02",
            "13.-02",
            "99999999999-1",
            "13-99999999999",
            "",
        ] {
            assert_eq!(PlanRef::from_id(bad), None, "{bad:?} must not parse");
            assert_eq!(
                attribute(None, Some(bad), Some(bad), Some(bad)),
                None,
                "{bad:?} must not attribute"
            );
        }
    }

    #[test]
    fn commit_scopes_pick_the_first_plan_scope() {
        assert_eq!(
            commit_scope_plan(&strs(&["docs(13-17): summary", "feat(13-17): x"])),
            Some("13-17".to_string())
        );
        assert_eq!(
            commit_scope_plan(&strs(&["fix: typo", "chore(release): v1.8.0"])),
            None
        );
        assert_eq!(
            commit_scope_plan(&strs(&["fix: typo", "test(07.1-03): red"])),
            Some("07.1-03".to_string()),
            "the first subject that names a plan, not the first subject"
        );
        assert_eq!(commit_scope_plan(&[]), None);
    }

    #[test]
    fn plan_ids_join_pad_insensitively() {
        assert_eq!(PlanRef::from_id("13-1"), PlanRef::from_id("13-01"));
        assert_eq!(PlanRef::from_id("7.1-3"), PlanRef::from_id("07.1-03"));
        assert_eq!(pr("13-1").label(), "13-01");

        let st = state(
            "13",
            vec![wave(Some(1), &strs(&["13-01-slug", "13-02-slug"]))],
            &[],
            2,
        );
        let view = derive(
            &agents(vec![
                row("/wt/a", AgentLiveness::Live, Some("13-02")),
                row("/wt/b", AgentLiveness::Live, Some("13-2")),
            ]),
            &st,
        );
        assert_eq!(
            view.running, 1,
            "two agents on one plan make one running plan"
        );
        assert_eq!(view.queued, 1);
    }

    // --- the three observed run shapes -------------------------------------

    #[test]
    fn a_single_executor_in_a_single_plan_wave() {
        let st = state(
            "05",
            vec![
                wave(Some(1), &strs(&["05-01"])),
                wave(Some(2), &strs(&["05-02"])),
            ],
            &strs(&["05-01"]),
            2,
        );
        let view = derive(
            &agents(vec![row("/wt/a", AgentLiveness::Live, Some("05-02"))]),
            &st,
        );
        assert_eq!(view.active_phase, PhaseNum::parse("5"));
        assert_eq!(view.current_wave, Some(2));
        assert_eq!(view.max_wave, Some(2));
        assert_eq!(
            view.summary_forms()[0],
            format!("P05 {DOT} w2/2 {DOT} 1 run {DOT} 1/2 done")
        );
        assert_eq!(view.waves.len(), 2);
        assert!(!view.waves[0].current && view.waves[1].current);
    }

    #[test]
    fn three_code_fixers_without_plans_use_the_generic_form() {
        let fixer = |path: &str| AgentRow {
            agent_type: Some(Untrusted::from_untrusted_source("gsd-code-fixer".into())),
            ..row(path, AgentLiveness::Live, None)
        };
        let view = derive(
            &agents(vec![fixer("/wt/a"), fixer("/wt/b"), fixer("/wt/c")]),
            &ProjectState::default(),
        );
        assert_eq!(view.summary_forms(), vec!["3 agents".to_string()]);

        let one = derive(&agents(vec![fixer("/wt/a")]), &ProjectState::default());
        assert_eq!(one.summary_forms(), vec!["1 agent".to_string()]);
    }

    #[test]
    fn thirteen_executors_in_wave_two_of_eleven() {
        let view = derive(&agents(thirteen_live_executors()), &phase_13_state());
        assert_eq!(view.active_phase, PhaseNum::parse("13"));
        assert_eq!(view.current_wave, Some(2));
        assert_eq!(view.max_wave, Some(11));
        assert_eq!(view.plan_total, 35);
        assert_eq!(
            (
                view.done,
                view.finished,
                view.running,
                view.stalled,
                view.queued
            ),
            (8, 0, 13, 0, 14)
        );
        assert_eq!(view.waves.len(), 11);
        assert_eq!(
            view.waves[1],
            WaveRow {
                wave: Some(2),
                running: 13,
                queued: 1,
                current: true,
                ..WaveRow::default()
            }
        );
        assert_eq!(
            view.summary_forms(),
            vec![
                format!("P13 {DOT} w2/11 {DOT} 13 run {DOT} 8/35 done"),
                format!("w2/11 {DOT} 13 run {DOT} 8/35 done"),
                format!("w2/11 {DOT} 13 run {DOT} 8/35"),
                format!("w2/11 {DOT} 13 run"),
                "w2/11 13run".to_string(),
                "13run".to_string(),
            ]
        );
    }

    #[test]
    fn finished_agents_count_as_done_plus_unmerged() {
        let mut rows = thirteen_live_executors();
        rows[0].liveness = AgentLiveness::Finished;
        rows[1].liveness = AgentLiveness::Finished;
        let view = derive(&agents(rows), &phase_13_state());
        assert_eq!(view.finished, 2);
        assert_eq!(view.running, 11);
        assert_eq!(view.done, 8, "finished is never folded into done");
        assert_eq!(
            view.summary_forms()[0],
            format!("P13 {DOT} w2/11 {DOT} 11 run {DOT} 10/35 done")
        );
        assert_eq!(view.waves[1].finished, 2);
        assert_eq!(view.current_wave, Some(2));

        // A SUMMARY in the worktree finishes a plan whose agent still reads Live.
        let mut rows = thirteen_live_executors();
        rows[2].summary_in_worktree = true;
        let view = derive(&agents(rows), &phase_13_state());
        assert_eq!((view.finished, view.running), (1, 12));
    }

    /// A live worktree-less agent (the scan keeps only live ones).
    fn live_child() -> ChildAgent {
        ChildAgent {
            liveness: AgentLiveness::Live,
            ..ChildAgent::default()
        }
    }

    /// CR-01: `Finished` is done work. It counts inside a ladder something
    /// running switched on, but never switches the summary on by itself, and
    /// a leftover never selects the executor ladder.
    #[test]
    fn a_finished_row_alone_never_switches_the_summary_on() {
        let finished: Vec<AgentRow> = (9..=10)
            .map(|n| {
                row(
                    &format!("/wt/agent-{n:02}"),
                    AgentLiveness::Finished,
                    Some(&format!("13-{n:02}")),
                )
            })
            .collect();
        let view = derive(&agents(finished.clone()), &phase_13_state());
        assert!(!view.is_active(), "finished rows alone are not running");
        assert_eq!(view.summary_forms(), Vec::<String>::new());
        assert_eq!(view.finished, 2, "the counts are still derived");

        let with_live_child = ProjectAgents {
            rows: finished,
            worktreeless: vec![live_child()],
            ..ProjectAgents::default()
        };
        let view = derive(&with_live_child, &phase_13_state());
        assert!(view.is_active());
        assert_eq!(
            view.summary_forms(),
            vec!["1 agent".to_string()],
            "the live agent is counted, the leftovers pick no ladder"
        );

        let stalled_plus_live_child = ProjectAgents {
            rows: vec![row("/wt/agent-09", AgentLiveness::Stalled, Some("13-09"))],
            worktreeless: vec![live_child()],
            ..ProjectAgents::default()
        };
        assert_eq!(
            derive(&stalled_plus_live_child, &phase_13_state()).summary_forms(),
            vec!["1 agent".to_string()]
        );
    }

    /// Phase 12 with plans 12-01..12-03 in wave 1, beside [`phase_13_state`].
    fn phases_12_and_13_state() -> ProjectState {
        let mut st = phase_13_state();
        st.phase_disk_statuses.insert(
            "12".to_string(),
            DiskInference {
                plan_waves: vec![wave(Some(1), &strs(&["12-01", "12-02", "12-03"]))],
                plan_count: 3,
                ..DiskInference::default()
            },
        );
        st
    }

    /// WR-01: running rows choose the active phase; `Finished` and `Stalled`
    /// rows vote only when nothing attributed is running.
    #[test]
    fn running_agents_outvote_orphans_for_the_active_phase() {
        let orphans = vec![
            row("/wt/o1", AgentLiveness::Stalled, Some("12-01")),
            row("/wt/o2", AgentLiveness::Stalled, Some("12-02")),
            row("/wt/o3", AgentLiveness::Finished, Some("12-03")),
        ];
        let mut rows = orphans.clone();
        rows.push(row("/wt/l1", AgentLiveness::Live, Some("13-01")));
        rows.push(row("/wt/l2", AgentLiveness::Live, Some("13-02")));
        let view = derive(&agents(rows), &phases_12_and_13_state());
        assert_eq!(view.active_phase, PhaseNum::parse("13"));
        assert_eq!(view.running, 2);
        let forms = view.summary_forms();
        assert!(forms[0].starts_with("P13"), "{forms:?}");

        let view = derive(&agents(orphans.clone()), &phases_12_and_13_state());
        assert_eq!(
            view.active_phase,
            PhaseNum::parse("12"),
            "the fallback tier: orphans vote when nothing runs"
        );
        assert_eq!(view.summary_forms(), vec!["2 stalled".to_string()]);

        // A first-tier tie goes to the higher phase, whatever the orphans say.
        let mut rows = orphans;
        rows.push(row("/wt/l0", AgentLiveness::Live, Some("12-01")));
        rows.push(row("/wt/l1", AgentLiveness::Idle, Some("13-01")));
        let view = derive(&agents(rows), &phases_12_and_13_state());
        assert_eq!(view.active_phase, PhaseNum::parse("13"));
    }

    #[test]
    fn stalled_and_unknown_only_states() {
        let st = state(
            "13",
            vec![wave(Some(1), &strs(&["13-01", "13-02", "13-03"]))],
            &[],
            3,
        );
        let view = derive(
            &agents(vec![
                row("/wt/a", AgentLiveness::Live, Some("13-01")),
                row("/wt/b", AgentLiveness::Stalled, Some("13-02")),
                row("/wt/c", AgentLiveness::Stalled, Some("13-03")),
                row("/wt/d", AgentLiveness::Unknown, Some("13-03")),
            ]),
            &st,
        );
        assert_eq!(view.running, 1);
        assert_eq!(view.stalled, 1, "13-02's only agent is stalled");
        assert_eq!(
            view.queued, 1,
            "13-03 has an Unknown agent beside the stalled one: not every agent is stalled"
        );

        let all_stalled = derive(
            &agents(vec![
                row("/wt/b", AgentLiveness::Stalled, Some("13-02")),
                row("/wt/c", AgentLiveness::Stalled, Some("13-03")),
            ]),
            &st,
        );
        assert_eq!(all_stalled.summary_forms(), vec!["2 stalled".to_string()]);

        // An Unknown row names no active phase, so STATE.md's phase is used.
        let mut st13 = st.clone();
        st13.state_md_phase_number = PhaseNum::parse("13");
        let unknown_only = derive(
            &agents(vec![row("/wt/d", AgentLiveness::Unknown, Some("13-03"))]),
            &st13,
        );
        assert_eq!(unknown_only.active_phase, PhaseNum::parse("13"));
        assert!(!unknown_only.is_active());
        assert_eq!(unknown_only.summary_forms(), Vec::<String>::new());
        assert_eq!(unknown_only.queued, 3, "an Unknown agent runs nothing");

        let nothing = derive(&agents(vec![]), &st);
        assert_eq!(nothing.summary_forms(), Vec::<String>::new());
    }

    #[test]
    fn a_phase_without_wave_metadata_has_no_wave_segment() {
        let st = state("13", vec![], &strs(&["13-01-slug"]), 3);
        let view = derive(
            &agents(vec![row("/wt/a", AgentLiveness::Live, Some("13-02"))]),
            &st,
        );
        assert!(view.waves.is_empty());
        assert_eq!((view.current_wave, view.max_wave), (None, None));
        assert_eq!((view.done, view.running), (1, 1));
        let forms = view.summary_forms();
        assert_eq!(forms[0], format!("P13 {DOT} 1 run {DOT} 1/3 done"));
        assert!(
            forms.iter().all(|f| !f.contains('w')),
            "no w segment: {forms:?}"
        );
    }

    #[test]
    fn the_unknown_wave_bucket_is_never_current_nor_the_denominator() {
        let st = state(
            "13",
            vec![
                wave(Some(1), &strs(&["13-01"])),
                wave(Some(2), &strs(&["13-02"])),
                wave(None, &strs(&["13-03", "13-04"])),
            ],
            &strs(&["13-01"]),
            4,
        );
        let view = derive(
            &agents(vec![row("/wt/a", AgentLiveness::Live, Some("13-02"))]),
            &st,
        );
        let last = view.waves.last().expect("the w? bucket is shown");
        assert_eq!(last.wave, None);
        assert_eq!(last.label(), "w?");
        assert!(!last.current);
        assert_eq!(last.queued, 2);
        assert_eq!((view.current_wave, view.max_wave), (Some(2), Some(2)));

        // Only the w? bucket holds unfinished plans: there is no current wave.
        let st = state(
            "13",
            vec![
                wave(Some(1), &strs(&["13-01"])),
                wave(None, &strs(&["13-02"])),
            ],
            &strs(&["13-01"]),
            2,
        );
        let view = derive(
            &agents(vec![row("/wt/a", AgentLiveness::Live, Some("13-02"))]),
            &st,
        );
        assert_eq!((view.current_wave, view.max_wave), (None, Some(1)));
        assert!(view.waves.iter().all(|w| !w.current));
        assert_eq!(
            view.summary_forms()[0],
            format!("P13 {DOT} 1 run {DOT} 1/2 done")
        );
    }

    #[test]
    fn every_summary_form_is_narrower_than_the_one_before() {
        let views = [
            derive(&agents(thirteen_live_executors()), &phase_13_state()),
            derive(
                &agents(vec![row("/wt/a", AgentLiveness::Live, Some("13-02"))]),
                &state("13", vec![], &strs(&["13-01"]), 3),
            ),
            AgentView {
                active_phase: PhaseNum::parse("999.12"),
                current_wave: Some(99),
                max_wave: Some(99),
                plan_total: 999,
                done: 998,
                running: 99,
                agents: vec![row("/wt/a", AgentLiveness::Live, Some("999.12-01"))],
                ..AgentView::default()
            },
        ];
        for view in &views {
            let forms = view.summary_forms();
            assert!(forms.len() >= 4, "{forms:?}");
            for pair in forms.windows(2) {
                assert!(
                    width(&pair[1]) < width(&pair[0]),
                    "{:?} is not narrower than {:?}",
                    pair[1],
                    pair[0]
                );
            }
        }

        // The common-case form fits the 13-cell Status column at its extremes.
        let widest = AgentView {
            active_phase: PhaseNum::parse("13"),
            current_wave: Some(99),
            max_wave: Some(99),
            plan_total: 99,
            running: 99,
            agents: vec![row("/wt/a", AgentLiveness::Live, Some("13-01"))],
            ..AgentView::default()
        };
        let forms = widest.summary_forms();
        assert_eq!(forms[4], "w99/99 99run");
        assert!(width(&forms[4]) <= 13);
        assert_eq!(width(DOT), 1, "U+00B7 is one cell");
    }

    #[test]
    fn display_order_is_total_and_stable() {
        let rows = vec![
            row("/wt/z", AgentLiveness::Ended, Some("13-01")),
            row("/wt/y", AgentLiveness::Unknown, None),
            row("/wt/x", AgentLiveness::Stalled, Some("13-02")),
            row("/wt/w", AgentLiveness::Finished, Some("13-03")),
            row("/wt/v", AgentLiveness::Idle, Some("13-04")),
            row("/wt/b", AgentLiveness::Live, None),
            row("/wt/c", AgentLiveness::Live, Some("13-06")),
            row("/wt/a", AgentLiveness::Live, Some("13-06")),
            row("/wt/d", AgentLiveness::Live, Some("13-05")),
        ];
        let st = state("13", vec![wave(Some(1), &ids(1, 6))], &[], 6);
        let view = derive(&agents(rows.clone()), &st);
        let order: Vec<&str> = view
            .agents
            .iter()
            .map(|r| r.path.to_str().unwrap())
            .collect();
        assert_eq!(
            order,
            vec!["/wt/d", "/wt/a", "/wt/c", "/wt/b", "/wt/v", "/wt/w", "/wt/x", "/wt/y", "/wt/z"]
        );

        let mut reversed = rows;
        reversed.reverse();
        assert_eq!(
            derive(&agents(reversed), &st),
            view,
            "input order never changes the view"
        );
        assert_eq!(derive(&agents(view.agents.clone()), &st), view);
    }
}
