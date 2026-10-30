# Phase 25: Running Agents & Live Wave View - Pattern Map

**Mapped:** 2026-09-25
**Files analyzed:** 19 (7 new, 12 modified)
**Analogs found:** 18 / 19. The only file without a close analog is `src/agents/adapter.rs`; it has a partial analog.
**Scope note:** The Codex adapter (D-A08) is deferred and is not mapped here.

All analog paths below are git-tracked source; each was checked with `git ls-files`. Line numbers were read on 2026-09-25 and will drift, so re-grep the named symbol before editing.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/agents/mod.rs` (new) | service: scan entry, public types, thresholds | batch read-only scan → `Action` | `src/driver/reconcile.rs` | exact (read-only per-project scan over registered projects, verdict enum, zero-write guard) |
| `src/agents/worktrees.rs` (new) | core scanner and parser | transform (pure parse) + git read | `src/driver/reconcile.rs` `read_run_facts`/`run_facts_from_value` split; `src/state_reader/plan_waves.rs` pure-function test style | role-match |
| `src/agents/adapter.rs` (new) | trait and registry | request-response (sync, plain data) | `src/executor/mod.rs:94` `trait Executor`; `src/executor/runtime.rs:174-212` per-runtime dispatch | partial |
| `src/agents/claude.rs` (new) | runtime adapter | file I/O (tolerant JSON plus stat) | `src/driver/reconcile.rs:167-244` tolerant record read; `src/state_reader/queue_md.rs:38-108` serde `default` and `dirs::home_dir` | role-match |
| `src/agents/waves.rs` (new) | pure model | transform | `src/state_reader/plan_waves.rs` | exact |
| `src/agents/fixers.rs` (new, AGENT-07) | utility | transform (frontmatter plus log subjects) | `src/state_reader/disk_status.rs:443` `leading_frontmatter_nested_value`; `src/state_reader/roadmap_md.rs:331-347` regex statics | role-match |
| `tests/agents_scan.rs` (new) | integration test | real git worktrees plus fake Claude root | `tests/driver_dry_run.rs:239-297` (zero-write fingerprint); `tests/common/mod.rs:61-118` (`git()` and fixture) | exact |
| `src/state_reader/git_ops.rs` (mod) | git read wrapper | request-response subprocess | same file: `git_read_raw` 198-212, `working_tree_stat` 291-317 | exact |
| `src/state_reader/disk_status.rs` (mod) | reader | file I/O (existing scan) | same file: `plan_waves` field 298-312, Pass 2 pairing 888-935 | exact |
| `src/lib.rs` (mod) | module registry | — | same file, line 19 `pub mod state_reader;` | exact |
| `src/action.rs` (mod) | message enum | event | `Action::RunsReconciled` 138-170 | exact |
| `src/app.rs` (mod) | controller: tick and update | event-driven | Tick block 1129-1171; `RunsReconciled` handler 1615-1647; `AppContext` literal 570-603; `DetailSubView::Archive` doc 28-34 | exact |
| `src/ui/screens/mod.rs` (mod) | state store (`AppContext`, `ProjectViewCache`) | — | `observed_runs` sibling map 1333-1355; `sessions_selected` 965 | exact |
| `src/ui/screens/normal.rs` (mod) | component (dashboard) | render | `compact_pipeline` 285-322; `dashboard_columns` 338-370; status-cell build 823-853; test harness 1911-2010 | exact |
| `src/ui/screens/detail.rs` (mod) | component (detail tabs) | render plus key handling | `docs_sub_tab_strip`/`docs_sub_tab_row` 6457-6491; `m` arm 3242-3257; `render_sessions_tab` 4911-4999; `footer_spans` 6568-6644 | exact |
| `src/ui/screens/render_escape_guard.rs` (mod) | guard test | — | `ALL_SUB_VIEWS` 1396; `DETAIL_TAB_ARRIVAL` 1423-1490; `sub_view_label` 1618; `probe_ctx` 1068-1115; NormalScreen states 1891-1932 | exact |
| `src/ui/screens/help.rs` (mod) | component (help) | render | `row("m", …)` 244 and its pin test 936-953 | exact |
| `.planning/REQUIREMENTS.md` (mod) | docs | — | `### UI Fixes (UIFIX)` 99-104 and the Traceability table 126+ | exact |
| `.planning/ROADMAP.md` (mod) | docs | — | line 787 `**Requirements**: TBD (to be derived in discuss)` | exact |

## Pattern Assignments

### `src/agents/mod.rs` (service, batch read-only scan)

**Analog:** `src/driver/reconcile.rs`

**Module-doc pattern** (lines 1-34). Open the module by stating its governing rule first: "This module performs zero disk writes". State how the rule is enforced, then list the tempting repairs it must never make. For this module those are: no `worktree prune`, no `fetch`, no index refresh, never reading transcript contents, and never scanning `$TMPDIR`.

**Imports** (lines 36-41):
```rust
use std::collections::HashMap;
use std::path::Path;

use crate::config::RegisteredProject;
use crate::driver::liveness::{self, Liveness};
```
Use `crate::config::RegisteredProject` with the same shape. Research suggests passing `Vec<(String, PathBuf)>`; alternatively pass `&HashMap<String, RegisteredProject>` the way `reconcile_all` does. The app already clones `self.ctx.config.projects` for the reconcile task (app.rs:1154), so reuse that clone.

**Verdict enum with an explicit "unknown" state** (lines 43-68, and `driver/liveness.rs:73-83`). Every state gets a doc paragraph saying why it is not a synonym for its neighbour. `Unknown` is "nothing was established" and is **never** `Dead`. Copy that doctrine for `unknown` (no adapter data) versus `stalled`.
**Naming collision:** `crate::driver::liveness::Liveness` already exists. Name the new enum `AgentLiveness`, or keep it as `agents::Liveness` and never glob-import it next to the driver one.

**Plain-data payload type** (lines 70-111). The type is `#[derive(Debug, Clone, PartialEq, Eq)]`, holds "ids, counts and strings — never a file handle and never a join handle", and its doc says why: `Action` derives `Clone`.

**Pure classifier split from I/O** (lines 137-154):
```rust
fn classify(ended_at: Option<&str>, liveness: Liveness) -> RunVerdict {
    match (ended_at, liveness) {
        (Some(_), _) => RunVerdict::Ended,
        (None, Liveness::Alive) => RunVerdict::Live,
        ...
    }
}
```
Apply this to the D-C08 liveness classification. Make it a pure function of `(Enrichment facts, now, worktree_summary_present)` and keep the threshold constants (`LIVE_SECS`, `IDLE_SECS`, `MAX_AGENT_AGE`) in one `const` block beside it.

**Fleet scan with deterministic ordering** (lines 314-324):
```rust
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
```
Apply this to `scan_projects_guarded`. Also sort each project's rows (by worktree path) so the handler's equality-guarded redraw works. **New:** wrap each project in `std::panic::catch_unwind(AssertUnwindSafe(..))`, as RESEARCH Pattern 3 and Pitfall 6 require. The codebase has no existing `catch_unwind`, so this has no analog. `Cargo.toml` has no `[profile]` section, so unwinding is the default.

**Zero-write guard test** (lines 700-749). Copy this test into `src/agents/mod.rs` `#[cfg(test)]`, pointing it at each `src/agents/*.rs` file:
```rust
const SOURCE: &str = include_str!("reconcile.rs");
// verbs split into halves so this file's own source doesn't trip the guard
let offenders: Vec<(usize, &str, String)> = SOURCE
    .lines()
    .enumerate()
    .filter(|(_, line)| !line.trim_start().starts_with("//"))
    .filter_map(|(index, line)| verbs.iter().find(|verb| line.contains(verb.as_str()))
        .map(|verb| (index + 1, line.trim(), verb.clone())))
    .collect();
assert!(offenders.is_empty(), "...");
```
Suggested verbs: `fs::write`, `create_dir`, `remove_`, `rename(`, `OpenOptions`, `set_modified`. Exempt `#[cfg(test)]` blocks, or keep fixture writes in `tests/agents_scan.rs`.

---

### `src/agents/worktrees.rs` (core, pure parse plus git read)

**Analogs:** `src/driver/reconcile.rs:194-244` for the read/parse split, and `src/state_reader/plan_waves.rs` for the pure-function-plus-tests layout.

**Split I/O from parsing so the parser is testable without a disk or a spawn** (reconcile.rs:194-198, 200-213):
```rust
fn read_run_facts(run_dir: &Path) -> Option<RunFacts> {
    let raw = std::fs::read_to_string(run_dir.join("run.json")).ok()?;
    let value = serde_json::from_str::<serde_json::Value>(&raw).ok()?;
    run_facts_from_value(&value)
}
```
Here that becomes `scan_worktrees(root)`, which calls `git_ops::worktree_list_porcelain(root)` and then feeds `parse_porcelain_z(&str) -> Vec<RawWorktree>`. The parser is pure, so its in-source tests are string fixtures. **`Command::new(` must not appear here, including in tests** (see Shared Patterns).

**Pure function doc and test style** (plan_waves.rs:46-60, 228-237). Give each rejection its own assertion line, in the style of `test_plan_waves_reject_values_that_are_not_a_wave_number`. Apply this to `parse_agent_branch` (D-A06): it must reject `agent-a<16hex>` and accept both `p13-13` and bare `p22`. Apply it also to `is_agent_worktree` (the `worktree-wf_` widening) and to agent-id validation (`^[A-Za-z0-9_-]{1,64}$`).

**Validate before a string becomes a path or a key** (reconcile.rs:264-271). Here `run_paths` is fallible, "so a traversing id" cannot escape. Apply the same refusal to the agent id before building `.git/worktrees/<name>/gsd-plan-head-before-*` or `agent-<id>.meta.json`.

**Branch and path text:** wrap in `Untrusted::from_untrusted_source(..)` at parse time (see Shared Patterns).

---

### `src/agents/adapter.rs` (trait and registry)

**Analog (partial):** `src/executor/mod.rs:80-94`. Note its doc style: "On object safety" and "On backend honesty (capability tier, not parity)".

- Keep the trait **sync** and object-safe (`Send + Sync`), and return plain data. The `BoxFuture` machinery in `executor/mod.rs:75` is not needed here, so do not copy it.
- Add a doc paragraph like executor/mod.rs:88-93 that states honestly what adapters may report (facts only: last activity, lock released, ended flag, text) and that classification belongs to the core.
- The registry is `fn registered_adapters() -> Vec<Box<dyn AgentAdapter>>`, with a comment line reserving the Codex slot (D-A07/D-A08), as in RESEARCH Pattern 3.
- `runtime.rs:174-178` explains why the executor chose an enum over `Box<dyn>`. The reason is concrete builders, which does not apply here. State in a doc line why `Box<dyn>` is right for adapters: D-A07 requires one module plus one registration line.
- The D-A07 proof test is a `#[cfg(test)] struct FakeAdapter` fed through a public `scan_project_with(root, &[Box<dyn AgentAdapter>], now)`. Because it needs real git worktrees, it lives in `tests/agents_scan.rs`.

---

### `src/agents/claude.rs` (adapter, tolerant file I/O)

**Analogs:** `src/driver/reconcile.rs:167-198` for the tolerant read, and `src/state_reader/queue_md.rs:38-49, 85-92` for serde defaults and the home dir.

**Tolerant-read doctrine** (reconcile.rs:167-182). The record is read as raw or partial data because "a record written by a **later** schema must still answer this question". A missing, unreadable or unparseable record yields `None`, and the caller treats that as "no observable run", never as "crashed". Map this to D-A03: a bad meta means "no data this tick" and the row degrades to "(no agent metadata)".

**serde tolerant struct** (queue_md.rs:38-49):
```rust
#[derive(Debug, Deserialize)]
struct SmartEntryOutput {
    #[serde(default)]
    actions: Vec<SmartEntryAction>,
}
```
Apply it with `#[serde(rename_all = "camelCase", default)]` at container level and `Option<T>` fields, as in RESEARCH "Tolerant meta model". **Never `deny_unknown_fields`**; the ban is enforced tree-wide by `tests/spawn_seam_guard.rs:675`. `Untrusted` has no `Deserialize` impl, so deserialize `String` and wrap it immediately.

**Home-dir resolution** (queue_md.rs:90):
```rust
if let Some(home) = dirs::home_dir() {
    candidates.push(home.join(".claude/gsd-core/bin/gsd-tools.cjs"));
}
```
Apply it in `ClaudeCodeAdapter::from_env()`. Prefer `$CLAUDE_CONFIG_DIR` only when it is set, non-empty and absolute; otherwise use `dirs::home_dir()?.join(".claude")`. Tests use `ClaudeCodeAdapter::new(root)` only (D-C17, Pitfall 10).

**Size-capped read:** cap reads at 64 KiB with `File::open(..).take(64 * 1024).read_to_end(..)` and then `serde_json::from_slice::<Meta>(..).ok()`. The codebase has no existing capped read, so this has no analog.

**Tests:** these are in-source and tempdir-only (`tempfile::tempdir`, as in plan_waves.rs:117-127). Set mtimes with `std::fs::File::set_modified` and inject the clock as a `now: SystemTime` parameter. Cover the encoding cases (ASCII, `.`, non-BMP gives 2 dashes, over 200 characters gives the hash suffix) as pure string tests.

---

### `src/agents/waves.rs` (pure model, transform)

**Analog:** `src/state_reader/plan_waves.rs` (whole file, 283 lines)

**Imports** (line 16): reuse, don't re-read.
```rust
use super::disk_status::leading_frontmatter_value;
```
Here the imports become `crate::state_reader::plan_waves::PlanWave`, `crate::state_reader::disk_status::{DiskInference, plan_index}` (after `plan_index` is made `pub(crate)`) and `crate::state_reader::phase_num::{PhaseNum, same_phase}`.

**Label formats the stored number, not the position** (plan_waves.rs:31-43):
```rust
pub fn label(&self) -> String {
    match self.wave {
        Some(n) => format!("w{n}"),
        None => "w?".to_string(),
    }
}
```
Use `PlanWave::label()` for wave rows, and exclude `wave: None` from both the `w?/N` denominator and the "current wave".

**Deterministic ordering doctrine** (plan_waves.rs:75-79): "the sorting is what makes the result stable across refreshes". The `AgentView` compares equal across unchanged scans only if every `Vec` in it is sorted.

**Test style** (plan_waves.rs:113-282). Pure `#[test]`s build `PlanWave`s and `DiskInference`s by hand. Write one test for each of the three observed shapes: 1 executor with a 1-plan wave; 3 fixers; 13 executors at w2/11.

**Lookup by phase:** use `ProjectState::disk_status_for(id)` (`src/state_reader/mod.rs:232-237`), which is pad-insensitive. Never index `phase_disk_statuses` by raw string. When no agents are attributed, fall back to `ProjectState::active_phase_number()` (mod.rs:~299) or `current_phase`.

---

### `src/agents/fixers.rs` (AGENT-07, transform)

**Analogs:** `src/state_reader/disk_status.rs:414-470` and `src/state_reader/roadmap_md.rs:331-347`.

- For the total, call `leading_frontmatter_nested_value(content, "findings", "total")`. It is private today at disk_status.rs:443; make it `pub(crate)`. Do not write a second YAML reader (D-C11, "Don't Hand-Roll").
- **Regex statics: use the codebase's `OnceLock` idiom, not `LazyLock`** (roadmap_md.rs:344-347):
```rust
fn any_heading_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*#{1,6}[ \t]").unwrap())
}
```
Apply it to `finding_id_re()` (`\b(?:CR|WR|IN)-\d+\b`) and `fix_subject_re(phase)`. The same idiom applies to the description-attribution regexes in `waves.rs` and the branch grammar in `worktrees.rs`.
- Get subjects through `git_ops::log_subjects` only.

---

### `tests/agents_scan.rs` (integration test)

**Analogs:** `tests/common/mod.rs:61-118` and `tests/driver_dry_run.rs:239-297`.

**Header comment style** (driver_dry_run.rs:1-14). A `// ===` block says why this is an integration test: it needs real repositories and real worktrees, and `src/` must stay free of spawn sites.

**Fixture that skips gracefully** (common/mod.rs:61-101):
```rust
pub fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git").arg("-C").arg(dir).args(args).output().ok()
        .map(|out| out.status.success()).unwrap_or(false)
}
// ...
if !git(&work, &["init", "--quiet"]) { return None; }
git(&work, &["config", "user.email", "test@example.com"]);
git(&work, &["config", "user.name", "Test User"]);
git(&work, &["config", "commit.gpgsign", "false"]);
```
Declare `mod common;` and reuse `common::git`. Do not build a new `Fixture`, because `common::fixture()` installs envelope hooks this test does not want. Write a local `agents_fixture() -> Option<..>` on `TempDir` that runs `git worktree add -b worktree-agent-<id> .claude/worktrees/agent-<id>` and `git worktree lock --reason "claude agent agent-<id> (pid 1 start 2)"`. `common/mod.rs` carries `#![allow(dead_code)]`, which covers the helpers this target does not call.

**Non-intrusion proof** (driver_dry_run.rs:243-297): the `git_fingerprint` and `walk` digest idiom.
```rust
fn walk(dir: &Path, base: &Path, out: &mut Vec<(String, u64, u64)>) {
    ...
    let bytes = std::fs::read(&path).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    out.push((relative, bytes.len() as u64, hasher.finish()));
}
```
Apply it to Pitfall 8:
1. Make a tracked file stat-dirty in an agent worktree.
2. Digest `$(git rev-parse --git-path index)` for that worktree, which lives under `.git/worktrees/<name>/index`.
3. Run the scan.
4. Assert the digest is byte-identical and that no `index.lock` exists.

The doc at driver_dry_run.rs:246-250 explains why a length-only check misses an index refresh; cite it.

---

### `src/state_reader/git_ops.rs` (modified: env var plus four helpers)

**Analog:** same file.

**The wrapper to extend** (lines 198-212):
```rust
pub(crate) fn git_read_raw(project_root: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}
```
Add `.env("GIT_OPTIONAL_LOCKS", "0")` here, and extend the doc paragraph at 175-182 with one sentence explaining why both are set.

**Helper shape to copy** (lines 291-317, `working_tree_stat`). It routes through `git_read_raw`, returns a plain value, maps failure to empty or `None`, and never panics:
```rust
pub fn working_tree_stat(project_root: &Path) -> WorkingTreeStat {
    let stat_lines = git_read_raw(project_root, &["diff", "--stat", "HEAD"])
        .map(|raw| raw.lines().map(|l| l.trim_end().to_string())
            .filter(|l| !l.trim().is_empty()).collect())
        .unwrap_or_default();
```
The new helpers:
- `worktree_list_porcelain`, which returns the raw output (no trim, because NUL records);
- `commits_ahead`, which hex-validates the base sha before `format!`;
- `dirty_count`;
- `log_subjects`.

Each is `pub(crate)` and gets a section banner comment in the style of lines 156-171.

**Do NOT copy:** `git_last_commit_time` (line 12), `head_sha` (109-128) or `is_dirty` (140-154). All three build `Command::new("git")` directly **without** `--no-optional-locks`. `is_dirty` runs a plain `status --porcelain`, which is exactly the index-refreshing call D-B02 forbids. `dirty_count` must go through `git_read_raw`, not reuse `is_dirty`.

**Tests:** the in-source `#[cfg(test)]` at line 822 may use `Command::new` (this file is on the spawn allowlist). Use `try_init_repo_with_commit` (830-856) for the helper-level tests, and add a `git worktree add` step there if needed.

---

### `src/state_reader/disk_status.rs` (modified)

**Analog:** same file.

**Field pattern** (lines 298-312, `plan_waves` doc). Document the new `pub summarized_plans: Vec<String>` the same way:
- where it is filled ("out of the SAME … scan");
- that it costs no extra I/O;
- "**Ordered, and the order is load-bearing**" (`PartialEq` drives Updated suppression).

**Where to fill it** (lines 912-935). `matched_plans: HashSet<String>` already holds exactly the paired plan ids. After line 935, collect it into a `Vec`, sort it, and store it:
```rust
let summary_count: u32 = matched_plans.len() as u32;
// + let mut summarized_plans: Vec<String> = matched_plans.into_iter().collect();
// + summarized_plans.sort_by(|a, b| (plan_index(a).is_none(), plan_index(a), a).cmp(...)); // same key as plan_tokens 955-960
```
Add the field to the struct literal at 992-1022. `DiskInference` derives `Default`, so test literals using `..Default::default()` keep compiling.

**Visibility changes:** `fn plan_index` (399) and `fn leading_frontmatter_nested_value` (443) become `pub(crate) fn`. Leave their docs as they are; they already explain the `13-1 ≡ 13-01` and nested-key doctrine.

**Test:** extend next to `test_plan_tokens_collected_end_to_end_over_a_phase_directory` (~2596). Use the tempdir, write the plan and summary files, then call `infer_disk_status`.

---

### `src/lib.rs` (modified)

The file is alphabetical; insert `pub mod agents;` between `pub mod action;` (line 1) and `pub mod archive;` (line 2). Integration tests reach the module as `gsd_meta_manager::agents::…`.

---

### `src/action.rs` (modified)

**Analog:** `Action::RunsReconciled` (lines 138-170).

Copy its doc structure:
- "The payload is the **whole** result, not a delta, because the scan is authoritative … The handler replaces rather than merges for exactly that reason."
- "Every field … is plain data … so `Action` stays `Clone`".
- The sizing note (lines 110-126). `Vec`/`HashMap` payloads are 24 to 48 bytes, so no `Box` is needed. `clippy::large_enum_variant` fires only on a 200-byte difference.

```rust
AgentsScanned {
    per_project: std::collections::HashMap<String, crate::agents::ProjectAgents>,
    // or Vec<(String, ProjectAgents)> sorted by alias, like `runs`
},
```

---

### `src/app.rs` (modified: tick wiring, handler, context init, `DetailSubView`)

**Tick wiring analog** (lines 1141-1171). Place the new block **inside** the `if self.session_poll_counter >= 20` block, after the reconcile task. Copy its comment voice: "rides THIS counter and must never get one of its own".
```rust
if let Some(ref tx) = self.ctx.event_tx {
    let tx: UnboundedSender<Action> = tx.clone();
    let projects = self.ctx.config.projects.clone();
    tokio::task::spawn_blocking(move || {
        let runs = crate::driver::reconcile::reconcile_all(&projects);
        ...
        let _ = tx.send(Action::RunsReconciled { runs, last_outcomes });
    });
}
```
Add the `agents_scan_in_flight` gate (RESEARCH "Tick wiring"). The flag is set before the spawn, and the closure body always sends, even an empty result after a panic.

**Handler analog** (lines 1615-1647):
```rust
// Replaced wholesale for the reason `observed_runs` is: the scan is
// authoritative ...
// Equality-guarded redraw, following this file's existing discipline: a scan
// lands every ~5s for the whole life of the process ...
if self.ctx.observed_runs != observed {
    tracing::debug!(observed = observed.len(), live = ..., "driver reconciliation scan applied");
    self.ctx.observed_runs = observed;
    self.needs_redraw = true;
}
```
For `AgentsScanned`:
1. Clear `agents_scan_in_flight` unconditionally.
2. Derive `AgentView` per alias with `waves::derive(&agents, state)` from `self.ctx.project_states.get(alias)`. Derive in the handler, never at render time (plan_waves.rs:11-14).
3. Compare with `ctx.agent_views` and replace it wholesale plus redraw only when they differ.

Log counts only. `tracing::debug!` must never carry descriptions, because those are untrusted text.

**`AppContext` literal** (lines 570-603). Add `agent_views: HashMap::new(),` and `agents_scan_in_flight: false,` next to `driver_output`. The **same two fields must be added at the other five `AppContext { .. }` literals**, otherwise the crate does not compile:
- `src/ui/screens/mod.rs:2387` (`ctx_with_aliases`)
- `src/ui/screens/delete_confirm.rs:269`
- `src/ui/screens/normal.rs:1237`
- `src/ui/screens/driver_confirm.rs:716`
- `src/ui/screens/detail.rs:11148` (`test_ctx`)

**`DetailSubView::Agents`** (enum at lines 15-51). Add it after `Sessions`, with a doc copied from the `Archive` doc (lines 28-34): "The Sessions tab's `Agents` sub-view (D-C15), not a tab of its own. It shares Sessions' index … reached with `m` on the Sessions tab. Per-project view state is in memory only, so … needs no migration."

**Tests:** `#[tokio::test]` is required for anything reaching `spawn_blocking` (app.rs:2613-2614). Use the `obs_app(root)` helper shape (2635-2651) to get a live `event_tx`, and feed a synthetic `Action::AgentsScanned` the way `reconciled(runs)` does (2658-2663).

---

### `src/ui/screens/mod.rs` (modified: `AppContext`, `ProjectViewCache`)

**Sibling-map field doc analog** (lines 1333-1355, `observed_runs`). Copy the bullet structure:
- "A **sibling map**, shaped exactly like `run_states` and `observed_runs`";
- "It must not live on `ProjectState`: that type derives `PartialEq` …";
- "holds ids, counts and strings — **never a file handle**";
- "**replaced wholesale** by every scan", which is why it needs no prune.

Lines 1406-1410 warn that a per-alias map that is not pruned "reintroduces the Phase 16 leak". Wholesale replacement from a scan over registered projects is the answer; say so in the doc.

```rust
pub agent_views: HashMap<String, crate::agents::AgentView>,
pub agents_scan_in_flight: bool,
```

**View-cache scroll field** (line 965): add `pub agents_selected: usize,` beside `pub sessions_selected: usize,`. `ProjectViewCache` is `#[derive(Default)]` (line 908), so there is no other edit.

---

### `src/ui/screens/normal.rs` (modified: Status cell ladder)

**Analog:** same file.

**Width-exact cell builder with a named constant** (lines 285-322). `compact_pipeline` builds a `Line` from spans, and `STATUS_COLUMN_MIN_CELLS` documents its exact cell count and the bug a mismatch caused (UIFIX-02 / CR-01). The new `agent_summary_line(view, cells) -> Option<Line<'static>>` picks the first ladder form whose `Line::width()` is at most `cells`. Never use `str::len`.

**Single source of truth for widths** (lines 324-370): `dashboard_columns(terminal_width)` is "Single source of truth for the three width tiers". The new `fn status_column_cells(terminal_width: u16) -> u16` must re-run `Layout::horizontal(dashboard_columns(w).1).flex(Flex::Start).spacing(1)`:
- over the inner width (`w - 2`, the border gap described at lines 329-333);
- minus the 2-cell `"> "` highlight symbol (line 387).

It must never hard-code 13.

**Insertion point** (lines 829-853). Today the code is:
```rust
let status_cell: Line = if is_milestone_complete {
    ...
} else {
    match state.and_then(|s| s.current_phase_status.as_ref()) {
        Some(inference) => compact_pipeline(&inference.status),
        None => Line::from(Span::styled(crate::text::render_for_terminal(&status_str), ...)),
    }
};
```
Wrap it: `let status_cell = match ctx.agent_views.get(alias).and_then(|v| agent_summary_line(v, status_column_cells(terminal_width))) { Some(line) => line, None => <existing expression unchanged> };`. With no agents, the row must be byte-identical to today.

**Do not touch** `alias_badge` (179-228) or `row_badge` (241-277). D-C14 keeps the single-badge rule.

**Render tests** (lines 1911-2010). Reuse `render_dashboard_interior(width, rows)` / `render_dashboard_rows`, and follow the `pipeline_row()` fixture style. Assert at widths 60, 80, 120, 160 and 200 that the rendered Status cell equals a whole ladder form, and that `status_column_cells(w)` equals the rendered column width. Model the assertions on `test_dashboard_columns_status_floor_at_upper_tiers` (1988+).

---

### `src/ui/screens/detail.rs` (modified: Agents sub-view)

**Analog:** same file (Docs `Files | Milestones`).

**`tab_index`** (lines 994-1013) is exhaustive and wildcard-free on purpose. Copy the `Browse | Archive` arm:
```rust
// Sessions has two sub-views sharing one index (D-C15) ...
DetailSubView::Sessions | DetailSubView::Agents => 5,
```
`sub_view_from_index(5)` (line 1037) stays `Sessions`.

**The `m` toggle** (lines 3242-3257). Add a second guarded arm, in the same shape:
```rust
KeyCode::Char('m')
    if matches!(current_view, DetailSubView::Sessions | DetailSubView::Agents) =>
{
    let other = if current_view == DetailSubView::Sessions { DetailSubView::Agents } else { DetailSubView::Sessions };
    switch_to_sub_view(&self.alias, other, &mut self.scroll_offset, ctx)
}
```
`switch_to_sub_view` (1697-1706) is "the ONE arrival rule". The `n` arm (3412) stays guarded on `== DetailSubView::Sessions`.

**Scroll arms.** Add a `DetailSubView::Agents` branch next to each `DetailSubView::Sessions` branch in:
- `j`/`Down` (2096; Sessions branch at 2143-2161): clamp `agents_selected` to `rows.len() - 1` via `ctx.agent_views.get(&self.alias)`;
- `k`/`Up` (2266; 2300-2304): `saturating_sub(1)`;
- `PageDown` (2387; 2445);
- `PageUp` (2564; 2610).

For `Enter` (2749; the Sessions branch at 2881 resumes a session), the `Agents` arm is a no-op returning `ScreenAction::None`.

**Render dispatch.** Two sites must gain an `Agents` arm: line 4010 and line 5467 (`render_main_only`).

**Strip helper** (lines 6457-6491). Copy `docs_sub_tab_strip` and `docs_sub_tab_row` to `sessions_sub_tab_strip`/`sessions_sub_tab_row`:
```rust
pub(crate) fn docs_sub_tab_strip(active: &DetailSubView) -> Line<'static> {
    let on_milestones = *active == DetailSubView::Archive;
    let active_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD | Modifier::REVERSED);
    let dim = Style::default().fg(Color::DarkGray);
    let label = |name: &'static str, is_active: bool| -> Span<'static> {
        if is_active { Span::styled(format!("[{name}]"), active_style) } else { Span::raw(name) }
    };
    Line::from(vec![label("Files", !on_milestones), Span::styled(" \u{2502} ", dim),
                    label("Milestones", on_milestones), Span::styled("   m switch", dim)])
}
```
Call `sessions_sub_tab_row` first in **both** `render_sessions_tab` and the new `render_agents_tab`, the way `render_archive_tab` calls `docs_sub_tab_row` (line 5006).

**Agents body analog** (lines 4911-4999, `render_sessions_tab`):
- bordered `Block`;
- early return when `inner.height < 3 || inner.width < 10`;
- empty state as a `Paragraph` of `Line`s (4935-4947), which becomes "No running agents";
- selection clamped from `view_cache` (4950-4957);
- `List::new(items).highlight_style(Cyan+BOLD).highlight_symbol("> ")` with `ListState` (4985-4998).

Put the summary line and the wave rows above the list with `Layout::vertical([Length(1), Length(n_waves), Min(0)])`, as `render_archive_tab` does at 5017-5020. Put the current-wave marker `▸` plus BOLD in the text, so the highlight does not rely on colour alone.

**Footer** (`footer_spans`, lines 6613-6644). Add `[m] agents` to the Sessions arm, and add a new `Agents` arm that follows the Archive/Browse arms' `[m]` convention:
```rust
DetailSubView::Agents => {
    spans.push(Span::styled("[j/k]", b)); spans.push(Span::raw("scroll  "));
    spans.push(Span::styled("[m]", b));   spans.push(Span::raw(" sessions  "));
}
```

**Tests that must change:**
- `m_is_inert_outside_docs` (10964-10979) iterates every tab index except Docs and asserts `m` is inert. **Tab 5 (Sessions) now toggles**, so exclude `tab_index(&DetailSubView::Sessions)` as well and rename or re-doc the test.
- Add `m_switches_sessions_between_sessions_and_agents`, modelled on `m_switches_docs_between_files_and_milestones` (10933-10962). It uses `test_ctx()` (11122), `press()` (11227), `render_detail_to_text()` (12343) and `active_tab_text()` (10892), and asserts `"6:Ss"`-style tab text on both sub-views.

---

### `src/ui/screens/render_escape_guard.rs` (modified)

**Analog:** same file.

- `ALL_SUB_VIEWS` (line 1396): change `[DetailSubView; 10]` to `11` and add `Agents` after `Sessions`. Update the doc (1392-1395) in the same way it explains `Archive`.
- `sub_view_label` (1618-1632): add `Agents => "Agents sub-view",`.
- `DETAIL_TAB_ARRIVAL` (1423+): add a row next to "Sessions tab" (1462-1468), in the same voice:
```rust
(
    "Agents sub-view",
    true,
    "Sessions › Agents (D-C15). Draws each agent row's description, agentType and branch \
     from `ctx.agent_views` — populated by 25-xx. The Sessions sub-tab strip above it is \
     authored text and draws no identity.",
),
```
- `probe_ctx` (1068-1115): populate `ctx.agent_views.insert(identity.to_string(), <AgentView with the hostile identity as description, agent_type, branch and child description>)`, next to the `ctx.active_sessions` fixture (1089-1099).
- NormalScreen states (1891-1932): because `probe_ctx` now carries an `AgentView`, the `"dashboard"` state probes the Status-cell summary automatically. If the summary never draws untrusted text, record that explicitly. Otherwise add a `one_state("dashboard with running agents", …)`.

---

### `src/ui/screens/help.rs` (modified)

Line 244 is `row("m", "Docs tab: switch Files / Milestones (detail view)")`. Change it to a shared description, for example `row("m", "Docs / Sessions tab: switch sub-view (detail view)")`, and update the pinned test `the_docs_milestones_switch_is_documented` (936-953) **in the same commit**. It asserts the exact row appears exactly once, with the flag on and off.

---

### `.planning/REQUIREMENTS.md` and `.planning/ROADMAP.md` (docs)

- Add `### Agent Observation (AGENT)` after `### UI Fixes (UIFIX)` (lines 99-104). It needs `- [ ] **AGENT-0N**: …` bullets for AGENT-01 to AGENT-07 (CONTEXT D-C18), and one `| AGENT-0N | Phase 25: Running Agents & Live Wave View | Pending |` row per requirement in the Traceability table (126+).
- In ROADMAP.md line 787, replace `**Requirements**: TBD (to be derived in discuss)` with the AGENT IDs.

## Shared Patterns

### Spawn-site allowlist (blocking constraint)
**Source:** `tests/spawn_seam_guard.rs:45-102` (`SPAWN_ALLOWLIST`, which includes `src/state_reader/git_ops.rs` and `src/executor/outcome.rs`).
**Apply to:** every `src/agents/*.rs` file.
No `Command::new(` may appear anywhere under `src/agents/`, **including `#[cfg(test)]` modules**. All git goes through new `git_ops` helpers. Tests that need git live in `tests/agents_scan.rs`; `tests/` is not walked. Also run `tests/async_blocking_guard.rs`: the scan is sync and must only ever be called from `spawn_blocking`.

### Non-intrusive git
**Source:** `src/state_reader/git_ops.rs:173-212`.
**Apply to:** the core scan and the fixers.
Always go through `git_read_raw`: `--no-optional-locks`, plus the `GIT_OPTIONAL_LOCKS=0` added in this phase, and `-C <worktree>`. Never put a branch name into argv; use `HEAD` inside the worktree. Hex-validate the base sha. Never reuse `is_dirty`, `head_sha` or `git_last_commit_time`.

### Untrusted text: escape at render
**Source:** `src/text.rs:567-644` (`Untrusted`, `from_untrusted_source`, `shown()`, `as_raw_for_logic_only()`); `src/ui/screens/normal.rs:756-773` (the render-site rule) and 858-865 (raw for lookup, escaped for display).
**Apply to:** meta `description` and `agentType`, branch names, worktree paths, and plan ids taken from descriptions.
```rust
Untrusted::from_untrusted_source(raw)      // at parse
value.shown()                              // at render (Rendered → Span::raw)
value.as_raw_for_logic_only()              // for joins / map keys only
```
`Untrusted` has a hand-written `Debug` (text.rs:655), so `{:?}` in logs is safe, but still log counts only.

### Failure as data
**Source:** `src/driver/reconcile.rs:178-182`; `git_ops.rs:188-189` ("`None` on a non-zero exit … Never panics").
**Apply to:** all of `src/agents/`.
Readers return `Option` or `Vec` defaults. A git failure becomes `?` counts on a row that is still shown (D-C16). An adapter failure becomes the core-only row (D-A03). Nothing may `unwrap()` on external data.

### Sibling map, not `ProjectState`
**Source:** `src/ui/screens/mod.rs:1288-1297, 1333-1355`.
**Apply to:** `agent_views`. Agent state never goes on `ProjectState` or `DiskInference`. The only `DiskInference` addition is `summarized_plans`, which is `.planning/` data from the existing scan.

### Deterministic ordering for `PartialEq`-guarded redraws
**Source:** `plan_waves.rs:75-79`, `reconcile.rs:319-322`, `disk_status.rs:291-296`.
**Apply to:** every `Vec` in `ProjectAgents`, `AgentView` and `summarized_plans`.

### Glyph width, and meaning that does not rely on colour
**Source:** `normal.rs:1075` (`s.width()`), `normal.rs:1842` (`line.width()` asserted in tests), `detail.rs:6457-6460` (bracketed plus reversed so it reads in monochrome).
**Apply to:** the dashboard ladder, the wave rows and the agent state words. Use ASCII state words (`live`, `idle`, `done`, `stall`, `ended`, `?`) and never `str::len`.

### Regex statics
**Source:** `src/state_reader/roadmap_md.rs:331-347` (the `OnceLock<Regex>` accessor function).
**Apply to:** `worktrees.rs` (branch grammar), `waves.rs` (description tiers 1 and 2) and `fixers.rs`. `LazyLock` would compile at MSRV 1.88, but `OnceLock` is the established house idiom.

### Edition 2021
Do not use `if let … && let …` chains (Pitfall 9). Use nested `if let` or `let … else`, as in `disk_status.rs:924`: `let Some(plan_id) = matched else { continue };`.

## No Analog Found

| File / concern | Role | Data Flow | Reason | Use instead |
|---|---|---|---|---|
| `src/agents/adapter.rs` (trait object registry) | trait/registry | sync request-response | The only existing trait (`executor::Executor`) is async, and the driver deliberately avoided `Box<dyn>` there. | RESEARCH.md Pattern 3 shape; doc voice from `executor/mod.rs:80-93` |
| `catch_unwind` per adapter and per project | core | — | There is no `catch_unwind` anywhere in `src/`. | RESEARCH.md Pattern 3 and Pitfall 6 |
| 64 KiB capped JSON read | adapter | file I/O | There is no existing capped read. | `std::io::Read::take` + `serde_json::from_slice(..).ok()` |
| Claude project-dir encoding (`encode_project_dir`, base-36 hash) | adapter utility | transform | The format is specific to Claude Code. | RESEARCH.md "Claude project-dir encoding (verified algorithm)" |

## Metadata

**Analog search scope:** `src/` (state_reader, driver, executor, ui/screens, app, action, text, lib), `tests/` (common, driver_dry_run, spawn_seam_guard, async_blocking_guard), `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`
**Files scanned:** 24
**Pattern extraction date:** 2026-09-25
