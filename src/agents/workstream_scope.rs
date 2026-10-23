//! Workstream scopes for the agents scan (quick 261023-05i).
//!
//! **Read-only, names only.** Like [`super::unmerged`], this module lists
//! directory entries with `std::fs::read_dir` and nothing else: no file is
//! written, created or opened for its contents, no process is spawned and no
//! git is run.
//!
//! A *scope* is the project itself or one workstream directory under
//! `<project>/.planning/workstreams/`. The agents scan used to key everything
//! by bare phase number, so the project's Phase 16 and a workstream's Phase 16
//! were indistinguishable. This module supplies the scope vocabulary the scan
//! joins on: [`names`] (which workstreams exist), [`scope_planning`] (a
//! scope's planning directory under any base) and, for agent rows,
//! [`resolve`] (which scope a plan belongs to).

use std::path::{Path, PathBuf};

use super::unmerged;
use super::waves::PlanRef;
use crate::state_reader::disk_status;

/// The UTF-8 sub-directory names of `planning/workstreams`, sorted. Files are
/// skipped; an unreadable or missing directory gives an empty list.
///
/// Matches `load_workstreams`, which takes every sub-directory, so these names
/// equal `WorkstreamState.name`. A name is a single path component read from
/// `read_dir`, never text an agent wrote.
pub fn names(planning: &Path) -> Vec<String> {
    let mut out: Vec<String> = unmerged::subdirs(&planning.join("workstreams"))
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    out.sort();
    out
}

/// A scope's planning directory under `base` (a `.planning` directory, main's
/// or a worktree's): `base` for the project, `base/workstreams/<name>` for a
/// workstream.
pub fn scope_planning(base: &Path, workstream: Option<&str>) -> PathBuf {
    match workstream {
        None => base.to_path_buf(),
        Some(name) => base.join("workstreams").join(name),
    }
}

/// Whether `dir` (a phase directory in some scope) holds an entry name that is
/// a PLAN or SUMMARY of `plan`. FIX and GAPCLOSURE entries are never plan
/// partners, the same exclusion the SUMMARY pairing makes.
fn plan_entries(dir: &Path, plan: &PlanRef) -> Vec<String> {
    unmerged::names(dir)
        .into_iter()
        .filter(|name| {
            if name.contains("-FIX-") || name.ends_with("-GAPCLOSURE-SUMMARY.md") {
                return false;
            }
            let stem = name
                .strip_suffix("-PLAN.md")
                .or_else(|| name.strip_suffix("-SUMMARY.md"));
            stem.and_then(disk_status::plan_index)
                .is_some_and(|(phase, number)| phase == plan.phase && number == plan.plan)
        })
        .collect()
}

/// Whether `text` has a token (a run of `[A-Za-z0-9._-]`) equal to `name`.
fn names_token(text: &str, name: &str) -> bool {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')))
        .any(|token| token == name)
}

/// The scope an agent row's plan belongs to: `Some(workstream)` or `None` for
/// the project (quick 261023-05i, I-2).
///
/// Candidates are the project plus every workstream in `names` that holds a
/// directory for the plan's phase number, in main (`main_planning`) or in the
/// row's worktree copy (`wt_planning`). In order:
///
/// 1. **Naming.** `texts` (the agent's description and branch) are split into
///    `[A-Za-z0-9._-]` tokens; when exactly one candidate workstream's name
///    equals a token, it wins. A token naming a workstream that is not a
///    candidate is ignored (T-05i-01).
/// 2. **Path evidence.** The worktree's phase directory in a candidate scope
///    holds a PLAN or SUMMARY entry for the plan that main's copy of that
///    scope lacks, in exactly one scope.
/// 3. **Uniqueness.** Exactly one candidate.
/// 4. Otherwise the project, today's attribution: a workstream gains a row
///    only on positive evidence or uniqueness.
///
/// Entry names only; no file is opened, and the names come from `read_dir`,
/// never from agent text. With no workstreams nothing is read at all.
pub fn resolve(
    plan: &PlanRef,
    texts: &[&str],
    wt_planning: &Path,
    main_planning: &Path,
    names: &[String],
) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let padded = plan.phase.padded();
    let find = |base: &Path, scope: Option<&str>| {
        disk_status::find_phase_dir(&scope_planning(base, scope), &padded)
    };
    let scopes = std::iter::once(None).chain(names.iter().map(|n| Some(n.as_str())));
    let candidates: Vec<Option<&str>> = scopes
        .filter(|s| find(main_planning, *s).is_some() || find(wt_planning, *s).is_some())
        .collect();
    let owned = |scope: Option<&str>| scope.map(str::to_string);

    let named: Vec<&str> = candidates
        .iter()
        .flatten()
        .copied()
        .filter(|name| texts.iter().any(|text| names_token(text, name)))
        .collect();
    if let [only] = named.as_slice() {
        return Some((*only).to_string());
    }

    let with_evidence: Vec<Option<&str>> = candidates
        .iter()
        .copied()
        .filter(|scope| {
            let Some(wt_dir) = find(wt_planning, *scope) else {
                return false;
            };
            let main_has: Vec<String> = find(main_planning, *scope)
                .map(|dir| plan_entries(&dir, plan))
                .unwrap_or_default();
            plan_entries(&wt_dir, plan)
                .iter()
                .any(|entry| !main_has.contains(entry))
        })
        .collect();
    if let [only] = with_evidence.as_slice() {
        return owned(*only);
    }

    if let [only] = candidates.as_slice() {
        return owned(*only);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_lists_workstream_directories_sorted_and_skips_files() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let ws = tmp.path().join("workstreams");
        std::fs::create_dir_all(ws.join("v0-14-voice")).unwrap();
        std::fs::create_dir_all(ws.join("v0-13-beta")).unwrap();
        std::fs::write(ws.join("README.md"), "").unwrap();
        assert_eq!(names(tmp.path()), vec!["v0-13-beta", "v0-14-voice"]);
        assert!(names(&tmp.path().join("missing")).is_empty());
    }

    const WS13: &str = "v0-13-public-beta";
    const WS14: &str = "v0-14-voice-assistant";

    /// A main and a worktree `.planning` that both hold the committed
    /// sentriq-shaped plans: project 16-01, v0.13 16-01, v0.14 26-01.
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().expect("temp dir");
        let main = tmp.path().join("main");
        let wt = tmp.path().join("wt");
        for base in [&main, &wt] {
            for rel in [
                "phases/16-raw-can/16-01-PLAN.md",
                &format!("workstreams/{WS13}/phases/16-beta/16-01-PLAN.md"),
                &format!("workstreams/{WS14}/phases/26-ptt/26-01-PLAN.md"),
            ] {
                touch(base, rel);
            }
        }
        (tmp, main, wt)
    }

    fn touch(base: &Path, rel: &str) {
        let path = base.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }

    fn ws_names() -> Vec<String> {
        vec![WS13.to_string(), WS14.to_string()]
    }

    fn pr(id: &str) -> PlanRef {
        PlanRef::from_id(id).expect("valid id")
    }

    #[test]
    fn resolve_picks_the_scope_by_naming_then_evidence_then_uniqueness() {
        let (_tmp, main, wt) = fixture();
        let names = ws_names();
        let resolve = |id: &str, texts: &[&str]| resolve(&pr(id), texts, &wt, &main, &names);

        // Evidence: the worktree's v0.13 copy holds a SUMMARY main's lacks.
        touch(
            &wt,
            &format!("workstreams/{WS13}/phases/16-beta/16-01-SUMMARY.md"),
        );
        assert_eq!(resolve("16-01", &[]), Some(WS13.to_string()));

        // Naming outranks evidence, among candidates only.
        assert_eq!(
            resolve(
                "16-01",
                &["Execute plan 1 of phase 16 --ws v0-13-public-beta"]
            ),
            Some(WS13.to_string())
        );
        // A token naming a workstream that holds no phase-16 directory is
        // ignored: v0.14 has none, so evidence still decides.
        assert_eq!(
            resolve("16-01", &["gsd run v0-14-voice-assistant"]),
            Some(WS13.to_string())
        );

        // Uniqueness: only v0.14 holds a phase 26.
        assert_eq!(resolve("26-01", &[]), Some(WS14.to_string()));
        // No scope holds phase 99.
        assert_eq!(resolve("99-01", &[]), None);
        // No workstreams: never a workstream, and nothing is read.
        assert_eq!(super::resolve(&pr("26-01"), &[], &wt, &main, &[]), None);
    }

    #[test]
    fn resolve_leaves_an_ambiguous_plan_with_the_project() {
        let (_tmp, main, wt) = fixture();
        let names = ws_names();
        let resolve = |id: &str, texts: &[&str]| resolve(&pr(id), texts, &wt, &main, &names);

        // No evidence and two candidate scopes with a phase 16: the project.
        assert_eq!(resolve("16-01", &[]), None);
        // Evidence in the PROJECT's copy only: the project.
        touch(&wt, "phases/16-raw-can/16-01-SUMMARY.md");
        assert_eq!(resolve("16-01", &[]), None);
        // Evidence in both copies is ambiguous again: the project.
        touch(
            &wt,
            &format!("workstreams/{WS13}/phases/16-beta/16-01-SUMMARY.md"),
        );
        assert_eq!(resolve("16-01", &[]), None);
        // FIX and GAPCLOSURE entries are never plan evidence.
        let (_tmp2, main2, wt2) = fixture();
        touch(
            &wt2,
            &format!("workstreams/{WS13}/phases/16-beta/16-01-FIX-SUMMARY.md"),
        );
        touch(
            &wt2,
            &format!("workstreams/{WS13}/phases/16-beta/16-01-GAPCLOSURE-SUMMARY.md"),
        );
        assert_eq!(resolve_in(&main2, &wt2, "16-01"), None);
        // Two candidate workstreams named at once is not a naming win.
        touch(
            &main,
            &format!("workstreams/{WS14}/phases/16-x/16-01-PLAN.md"),
        );
        touch(
            &wt,
            &format!("workstreams/{WS14}/phases/16-x/16-01-PLAN.md"),
        );
        assert_eq!(
            resolve("16-01", &["v0-13-public-beta v0-14-voice-assistant"]),
            None
        );
    }

    fn resolve_in(main: &Path, wt: &Path, id: &str) -> Option<String> {
        resolve(&pr(id), &[], wt, main, &ws_names())
    }

    #[test]
    fn scope_planning_joins_the_workstream_only_for_a_workstream() {
        let base = Path::new("/p/.planning");
        assert_eq!(scope_planning(base, None), base);
        assert_eq!(
            scope_planning(base, Some("v1")),
            Path::new("/p/.planning/workstreams/v1")
        );
    }
}
