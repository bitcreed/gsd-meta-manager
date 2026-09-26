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
    /// one of them) yields the wave ladder; otherwise an active view yields
    /// `N agents`; an inactive view whose rows include stalled ones yields
    /// `N stalled`; anything else yields no forms and the cell is left alone.
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
}
