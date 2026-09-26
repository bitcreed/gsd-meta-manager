//! The runtime-adapter seam (D-A01b, D-A07): how runtime-specific knowledge
//! about an agent — its type, its description, when it last did anything —
//! reaches the runtime-agnostic core in [`crate::agents`].
//!
//! **Facts, not verdicts.** An adapter reports what it observed: a transcript's
//! mtime, whether the runtime released its worktree lock, whether the runtime
//! recorded the agent as ended. It never decides whether an agent is live,
//! idle, stalled or finished. The core classifies every row in one place,
//! [`crate::agents::classify_liveness`], with the thresholds declared in
//! `src/agents/mod.rs`, so two runtimes can never disagree about what "stalled"
//! means.
//!
//! **Why `Box<dyn>`.** D-A07 requires that a runtime be added as one module
//! plus one registration line. A registry of trait objects gives exactly that;
//! the executor's enum dispatch (`src/executor/runtime.rs`) does not, because
//! every new variant there is an edit to every `match` over it.
//!
//! **Adding a runtime** (D-A07, D-A08). One new file,
//! `src/agents/adapters/<runtime>.rs`, implementing [`AgentAdapter`]; plus a
//! `pub mod <runtime>;` line and one line in [`registered_adapters`], both in
//! THIS file. Nothing in `src/agents/mod.rs`, `worktrees.rs`, the wave model
//! or the UI changes. The proof that the seam needs no core edit is the
//! test-only adapter in `tests/agents_scan.rs`, fed through
//! [`crate::agents::scan_project_with`]. The Codex adapter is deferred (D-A08);
//! its design notes live in the Phase 25 CONTEXT.md, § Deferred Ideas.
//!
//! **Failure is data.** An adapter that cannot read its runtime's files
//! reports nothing for that worktree, and the row degrades to what git knows.
//! The core also wraps every `enrich` call in `catch_unwind`, but that is
//! defence in depth, not a licence: adapters must be panic-free by
//! construction (no `unwrap` on external input).

use std::path::Path;
use std::time::SystemTime;

use super::worktrees::CoreWorktree;
use super::AgentLiveness;
use crate::text::Untrusted;

/// What the core found, handed read-only to every adapter.
#[derive(Debug, Clone)]
pub struct CoreSnapshot<'a> {
    /// The registered project root.
    pub project_root: &'a Path,
    /// The main worktree's path as git reported it, when git answered.
    pub main_worktree: Option<&'a Path>,
    /// Every non-main worktree; [`AdapterReport::per_worktree`] indexes this.
    pub worktrees: &'a [CoreWorktree],
    /// The scan's clock, so an adapter never reads the time itself.
    pub now: SystemTime,
}

/// A sub-agent an adapter observed — nested under a worktree agent, or with no
/// worktree of its own.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChildAgent {
    /// The runtime's agent type, e.g. `gsd-executor`.
    pub agent_type: Option<Untrusted>,
    /// The runtime's one-line description of the agent's task.
    pub description: Option<Untrusted>,
    /// The most recent activity the adapter could observe (a transcript mtime).
    pub last_activity: Option<SystemTime>,
    /// Whether the runtime recorded the agent as ended.
    pub ended: bool,
    /// Adapters leave this at its default; the core overwrites it.
    pub liveness: AgentLiveness,
}

/// What one adapter observed about one worktree. FACTS only — no liveness.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Enrichment {
    /// The runtime's agent type, e.g. `gsd-executor`.
    pub agent_type: Option<Untrusted>,
    /// The runtime's one-line description of the agent's task.
    pub description: Option<Untrusted>,
    /// The most recent activity the adapter could observe (a transcript mtime).
    pub last_activity: Option<SystemTime>,
    /// Whether the runtime recorded the agent as ended.
    pub ended: bool,
    /// Whether the runtime released the worktree's lock, in its own lock
    /// semantics; `None` when the adapter cannot say.
    pub lock_released: Option<bool>,
    /// Sub-agents running inside this worktree.
    pub children: Vec<ChildAgent>,
}

/// Everything one adapter reports for one scan.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AdapterReport {
    /// `(index into CoreSnapshot::worktrees, facts)`. An out-of-range index is
    /// ignored; the first adapter to claim an index wins.
    pub per_worktree: Vec<(usize, Enrichment)>,
    /// Agents with no worktree of their own (D-C07); the core keeps only the
    /// live ones.
    pub worktreeless: Vec<ChildAgent>,
}

/// One agent runtime's view of a project.
pub trait AgentAdapter: Send + Sync {
    /// A short, stable name shown on the rows this adapter enriched.
    fn name(&self) -> &'static str;
    /// Report facts about `snap`'s worktrees. Must not panic, write, or spawn.
    fn enrich(&self, snap: &CoreSnapshot<'_>) -> AdapterReport;
}

/// Every adapter this build ships, in registration order (first claim wins).
pub fn registered_adapters() -> Vec<Box<dyn AgentAdapter>> {
    // Claude Code (25-02): `pub mod claude;` above and one line here.
    // Codex (deferred, D-A08): `pub mod codex;` above and one line here.
    Vec::new()
}
