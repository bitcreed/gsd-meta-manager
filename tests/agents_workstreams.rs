// ============================================================================
// Quick 261023-05i: the agents scan, scoped by workstream, against real
// repositories with real linked worktrees.
//
// A project's parallel workstreams live in `.planning/workstreams/<dir>/` and
// carry their own `phases/`. The scan used to key everything by bare phase
// number, so the project's Phase 16 and a workstream's Phase 16 were one
// thing. These tests pin the scoped scan: unmerged work, worktree-held plans
// and worktree-only phases land in the workstream whose directory holds them
// and never in the project (or another workstream), and agent rows resolve to
// the scope their plan belongs to.
//
// An integration test because building these worlds spawns git, and
// `src/agents/` is not on the spawn allowlist (tests/spawn_seam_guard.rs). The
// fixtures are MODELLED on sentriq's names (v0-13-public-beta,
// v0-14-voice-assistant) without reading sentriq.
// ============================================================================

mod common;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use common::git;
use gsd_meta_manager::agents::unmerged::UnmergedKey;
use gsd_meta_manager::agents::waves::{self, PlanRef};
use gsd_meta_manager::agents::{scan_project_with, ProjectAgents};
use gsd_meta_manager::state_reader::parse_project_state;
use tempfile::TempDir;

const WS13: &str = "v0-13-public-beta";
const WS14: &str = "v0-14-voice-assistant";

/// Write the nested `rel` under `dir` (creating parents). Untracked is fine:
/// the scan reads the filesystem, never git objects.
fn put(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("fixture mkdir");
    std::fs::write(&path, body).expect("fixture write");
}

fn plan(n: &str) -> PlanRef {
    PlanRef::from_id(n).unwrap_or_else(|| panic!("{n} is a valid plan id"))
}

/// A sentriq-shaped repository: the project has Phase 16; v0.13 has Phase 16;
/// v0.14 lists 26, 27 and 28 with directories for 26 and 28 (28 holds three
/// plans and one summary). `None` when the sandbox forbids `git init`.
fn workstream_repo() -> Option<(TempDir, PathBuf)> {
    let tmp = TempDir::new().ok()?;
    let root = std::fs::canonicalize(tmp.path()).ok()?.join("project");
    std::fs::create_dir_all(&root).ok()?;
    if !git(&root, &["init", "--quiet"]) {
        return None;
    }
    assert!(git(&root, &["config", "user.email", "test@example.com"]));
    assert!(git(&root, &["config", "user.name", "Test User"]));
    assert!(git(&root, &["config", "commit.gpgsign", "false"]));

    let state = |milestone: &str, name: &str, phase: u32| {
        format!(
            "---\nmilestone: {milestone}\nmilestone_name: {name}\ncurrent_phase: {phase}\n\
             status: executing\n---\n\n# State\n"
        )
    };
    let planning = root.join(".planning");
    put(&planning, "STATE.md", &state("v0.12", "Actuation", 16));
    put(
        &planning,
        "ROADMAP.md",
        "# Roadmap\n\n## v0.12 — Actuation\n\n## Phases\n\n\
         - [ ] **Phase 16: Raw-CAN Button Replay PoC** - replay\n",
    );
    put(
        &planning,
        "phases/16-raw-can-button-replay-poc/16-01-PLAN.md",
        "# plan\n",
    );

    let ws13 = format!(".planning/workstreams/{WS13}");
    put(
        &root,
        &format!("{ws13}/STATE.md"),
        &state("v0.13", "Beta", 16),
    );
    put(
        &root,
        &format!("{ws13}/ROADMAP.md"),
        "# Roadmap\n\n## v0.13 — Public Beta\n\n## Phases\n\n\
         - [ ] **Phase 16: Beta Build Channel** - build\n",
    );
    put(
        &root,
        &format!("{ws13}/phases/16-beta-build-channel/16-01-PLAN.md"),
        "# plan\n",
    );

    let ws14 = format!(".planning/workstreams/{WS14}");
    put(
        &root,
        &format!("{ws14}/STATE.md"),
        &state("v0.14", "Voice", 28),
    );
    put(
        &root,
        &format!("{ws14}/ROADMAP.md"),
        "# Roadmap\n\n## v0.14 — Voice\n\n## Phases\n\n\
         - [ ] **Phase 26: Push-to-Talk Voice** - ptt\n\
         - [ ] **Phase 27: Parity** - parity\n\
         - [ ] **Phase 28: Mechanic Toolbox** - toolbox\n",
    );
    put(
        &root,
        &format!("{ws14}/phases/26-push-to-talk-voice/26-01-PLAN.md"),
        "# plan\n",
    );
    for n in 1..=3 {
        put(
            &root,
            &format!("{ws14}/phases/28-mechanic-toolbox/28-0{n}-PLAN.md"),
            "# plan\n",
        );
    }
    put(
        &root,
        &format!("{ws14}/phases/28-mechanic-toolbox/28-01-SUMMARY.md"),
        "# summary\n",
    );

    assert!(git(&root, &["add", "."]), "git add");
    assert!(
        git(&root, &["commit", "-m", "planning", "--quiet"]),
        "git commit"
    );
    Some((tmp, root))
}

/// `git worktree add -b <branch> <path>` in `root`, asserting it worked.
fn add_worktree(root: &Path, branch: &str, path: &str) -> PathBuf {
    assert!(
        git(root, &["worktree", "add", "--quiet", "-b", branch, path]),
        "git worktree add -b {branch} {path}"
    );
    root.join(path)
}

fn scan(root: &Path) -> ProjectAgents {
    scan_project_with(root, &[], SystemTime::now())
}

fn entry<'a>(
    scan: &'a ProjectAgents,
    name: &str,
) -> Option<&'a gsd_meta_manager::agents::WorkstreamAgents> {
    scan.workstreams.iter().find(|w| w.name == name)
}

fn unmerged_plans(items: &[gsd_meta_manager::agents::unmerged::UnmergedItem]) -> Vec<PlanRef> {
    items
        .iter()
        .filter_map(|i| match &i.key {
            UnmergedKey::Plan(p) => Some(p.clone()),
            UnmergedKey::Quick(_) => None,
        })
        .collect()
}

/// The tracer: a workstream's finished plans in a linked worktree travel
/// through the scan and `derive` into that workstream's own view, and the
/// project's same-numbered phase never shares them.
#[test]
fn a_workstream_worktree_finishing_plans_marks_its_own_phase_unmerged_end_to_end() {
    let Some((_tmp, root)) = workstream_repo() else {
        return;
    };
    let lanes = add_worktree(&root, "gsd/v0.14-voice", ".claude/worktrees/lanes-v014");
    for n in [2, 3] {
        put(
            &lanes,
            &format!(".planning/workstreams/{WS14}/phases/28-mechanic-toolbox/28-0{n}-SUMMARY.md"),
            "# summary\n",
        );
    }

    let scan = scan(&root);
    assert!(scan.unmerged.is_empty(), "{:?}", scan.unmerged);
    let ws = entry(&scan, WS14).expect("v0.14 has unmerged work");
    assert_eq!(
        unmerged_plans(&ws.unmerged),
        vec![plan("28-02"), plan("28-03")]
    );
    assert!(entry(&scan, WS13).is_none());

    let state = parse_project_state(&root.join(".planning"));
    let view = waves::derive(&scan, &state);
    assert_eq!(view.unmerged_in_phase("28"), 0);
    let ws_view = view.workstreams.get(WS14).expect("a v0.14 view");
    assert_eq!(ws_view.unmerged_in_phase("28"), 2);
    assert!(!view.workstreams.contains_key(WS13));

    // A second worktree finishing the PROJECT's 16-01 lands in the project's
    // list and in neither workstream's.
    let other = add_worktree(&root, "gsd/phase-16", ".claude/worktrees/lanes-16");
    put(
        &other,
        ".planning/phases/16-raw-can-button-replay-poc/16-01-SUMMARY.md",
        "# summary\n",
    );
    let scan = scan_project_with(&root, &[], SystemTime::now());
    assert_eq!(unmerged_plans(&scan.unmerged), vec![plan("16-01")]);
    for ws in &scan.workstreams {
        assert!(
            !unmerged_plans(&ws.unmerged).contains(&plan("16-01")),
            "{}: {:?}",
            ws.name,
            ws.unmerged
        );
    }
    let view = waves::derive(&scan, &state);
    assert_eq!(view.unmerged_in_phase("16"), 1);
    assert!(view
        .workstreams
        .values()
        .all(|w| w.unmerged_in_phase("16") == 0));
}
