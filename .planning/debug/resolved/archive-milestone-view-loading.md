---
status: resolved
trigger: "Archive tab (8:Arch) milestone drill-in view shows Loading... and flips back to Loading after a few seconds on refresh"
created: 2026-09-23
updated: 2026-09-23
resolved: 2026-09-23
---

# Debug: archive-milestone-view-loading

## Symptoms

DATA_START
- expected: In a project's detail screen, tab `8:Arch` (Archive) drilled into a milestone (e.g. `Archive > v1.2` for /home/blk/projects/flutter/daily-vow) shows the milestone's content (v1.2, v1.1), and keeps it across refreshes; when underlying files change it reloads in place without blanking.
- actual: Shows `Loading...` and stays there. ESC and re-enter shows content, but after a few seconds it flips back to `Loading...`.
- errors: none reported.
- timeline: Used to work. Recent candidates (unverified): 5-second session poll that gained Codex detection (quick batch 260923-lr7, commits 2054f81/5e49ba3), roadmap dependency graph quick task 260923-md1 (merged 8d24318, commits 8137449..10a0bf3), or a file-watcher/notify event.
- reproduction: Open TUI, open project detail for daily-vow, tab 8, drill into v1.2, wait a few seconds.
DATA_END

## Operator constraints (unattended run)

- Human unavailable: decide from evidence, record inferred decisions below marked `[INFERRED]`.
- Verify causal claim with a failing regression test (measurement over narrative; past planning notes recorded wrong diagnoses).
- Fix requirement: archive milestone view keeps content across refreshes and reloads in place (no blank to Loading) on file change. Regression test must fail before the fix.
- Tests: `rtk proxy cargo test --no-fail-fast`; only acceptable failure is the git-version constants test in src/envelope/policy.rs. Clippy: 6 known pre-existing errors (src/browser.rs, src/project_creator.rs, tests/envelope_*.rs); add none.
- Commit on master, never rebase, don't push, no stray worktrees.

## Current Focus

bug_class: Bohrbug (deterministic: fires on every 20-tick / ~5s prune)
hypothesis: "`App::prune_driver_maps` (runs every 20 ticks, ~5s) retains `ctx.archive_cache` entries whose KEY is a registered alias, but `archive_cache` is keyed by MILESTONE version (`ArchiveLoaded` inserts `milestone`, detail.rs reads `archive_cache.get(milestone)`). So every ~5s the whole cache is emptied, PhaseList's `archive_cache.get(\"v1.2\")` returns None, and the render's else-branch draws `Loading...`. Nothing re-requests the load (the load is only triggered on Enter from MilestoneList), so it stays Loading until ESC + re-enter."
status_note: "RESOLVED — fix committed (3c0e38f, 581aa7d); all guardrail signals passed; see Resolution."
next_action: "none — session closed"

reasoning_checkpoint:
  hypothesis: "prune_driver_maps (every 20 ticks ~5s) drops every archive_cache entry because it retains by registered-alias key while the map is keyed by milestone version; PhaseList render then finds no data and draws Loading..., and nothing re-requests the load."
  confirming_evidence:
    - "RED: App-level test renders 'Phase 01: Core Flow' before, 'Loading...' after exactly 20 Action::Tick (/tmp/amvl-red.log)."
    - "Removing ONLY the archive_cache retain line makes that test pass with the rest of the tick path intact (/tmp/amvl-exp.log)."
    - "git show 392902d adds the retain; its parent has no archive_cache prune."
  falsification_test: "If the view still flipped with the retain removed, or stayed intact with it present, the hypothesis would be wrong. Neither happened."
  fix_rationale: "Key the cache by alias -> milestone. The prune's alias retain then becomes CORRECT (it drops only unregistered projects' archives, which is what D-27/18-11 intended), and the cross-project collision (same key 'v1.2' for two projects) disappears with it. Removing the prune instead would reintroduce the Phase-16-style leak AND make the collision permanent."
  blind_spots: "In-place reload is a new behaviour (not a regression): selection indices can point past a shrunken list, handled by clamping on arrival. FileView content (archive_file_content) is read once on Enter and is NOT reloaded on change — out of scope, recorded as open."
  candidate_causes:
    - "code: prune key-model mismatch (archive_cache milestone-keyed, pruned as alias-keyed) — CONFIRMED"
    - "code: cache keyed by milestone only, shared across projects — CONFIRMED (second test), not the flip itself"
    - "environment/timing: 5s session poll + Codex detection /proc scan — ELIMINATED (shares timer only)"
    - "data/event: watcher FileChanged on .planning writes — ELIMINATED (no archive code on that path)"
  and_gate: "no for the flip (single sufficient cause, proven by the one-line experiment). The collision is an independent defect sharing the same key-model root; a fix that only removes the prune would expose it permanently, so both are addressed by the same re-keying."

## Evidence

- timestamp: 2026-09-23
  checked: "Knowledge base (.planning/debug/knowledge-base.md)"
  found: "Does not exist; no MemPalace match attempted. No known-pattern candidate."
  implication: "Proceed with open investigation."

- timestamp: 2026-09-23
  checked: "src/app.rs:1360-1369 (Action::ArchiveLoaded) and src/ui/screens/detail.rs:2537,2567,4682,4729"
  found: "`self.ctx.archive_cache.insert(milestone, data)` — keyed by the milestone version string only (e.g. \"v1.2\"); every reader does `ctx.archive_cache.get(milestone)`. The drill-in load is triggered ONLY on Enter at MilestoneList when `!archive_cache.contains_key(&milestone)`."
  implication: "archive_cache is milestone-keyed, not alias-keyed (and is shared across projects: two projects with v1.0 collide — latent second defect)."

- timestamp: 2026-09-23
  checked: "src/app.rs:1873-1889 (prune_driver_maps) and :1082-1144 (Tick handler)"
  found: "`self.ctx.archive_cache.retain(|alias, _| registered.contains_key(alias))` — treats the milestone key as an alias. prune_driver_maps runs every 20 ticks (~5s at 250ms), inline on the Tick path. The comment claims archive_cache 'holds a whole parsed milestone archive per alias'. The unit test `every_per_alias_driver_map_is_pruned` (app.rs:4513) inserts `archive_cache` with ALIAS keys, so it encodes the wrong key model and passes."
  implication: "Every ~5s every archive_cache entry (key \"v1.2\" is never a registered alias) is dropped -> PhaseList render else-branch 'Loading...' (detail.rs:4719-4723). Exactly matches: content after ESC+re-enter, flips to Loading a few seconds later."

- timestamp: 2026-09-23
  checked: "git blame src/app.rs:1884-1889; git tag --contains"
  found: "archive_cache prune introduced by 392902d (feat(18-11), 2026-07-29), first shipped in v1.7.0. The 20-tick call site is 15c477b (same day). Recent candidates 2054f81/5e49ba3 (Codex detection) and 8d24318 (roadmap graph) do not touch archive_cache or the prune."
  implication: "Offending commit candidate: 392902d, not the lr7/md1 commits named in the report. Needs measurement to confirm."

- timestamp: 2026-09-23
  checked: "git diff 7cf6910..8d24318 -- src (lr8/lr9/lra + md1 range) for archive/prune/view_cache"
  found: "No hunk touches archive_cache, ArchiveLoaded, ArchiveDepth handling or prune_driver_maps; only render_escape_guard fixture context mentions the Archive tab."
  implication: "Reported candidates 2054f81/5e49ba3/8d24318 are not causal (pending the measurement)."

- timestamp: 2026-09-23
  checked: "src/watcher.rs batch_actions + src/journal/mod.rs classify_change"
  found: "Watcher emits at most one Planning FileChanged per (root, kind) per 200ms batch, carrying the FIRST changed path; nothing in the FileChanged/Planning path touches archive state. Milestone list is discovered once (only when empty) and a milestone archive is loaded once (only when not cached)."
  implication: "A file-watcher event does NOT blank the view (no archive code on that path) — but it also never reloads it. An in-place reload must not be gated on changed_path being under milestones/ (the batch may carry STATE.md as its representative path)."

- timestamp: 2026-09-23
  checked: "Baseline clippy on unmodified tree: rtk proxy cargo clippy --all-targets > /tmp/amvl-clippy-baseline.log"
  found: "0 errors, 11 warning sites, all in src/browser.rs (3), src/project_creator.rs (1), tests/envelope_*.rs (7). None in app.rs/detail.rs/archive.rs/mod.rs."
  implication: "Baseline for the no-new-lints check."

- timestamp: 2026-09-23
  checked: "RED run on unfixed tree: 3 new App-level tests in src/app.rs (drive opened_on(Archive) -> Down -> Enter -> pump ArchiveLoaded -> TestBackend render). Logs /tmp/amvl-red.log, /tmp/amvl-red2.log"
  found: |
    archive_milestone_view_keeps_its_content_across_the_periodic_prune ... FAILED
      'Archive > v1.2 flipped back to Loading... after one 20-tick pass:' buffer shows 'Archive > v1.2' / 'Loading...'
      (content 'Phase 01: Core Flow' present BEFORE the 20 ticks — assertion passed)
    the_same_milestone_version_in_two_projects_does_not_share_a_cache_entry ... FAILED
      project `other`'s Archive > v1.2 shows 'Phase 01: Alpha Work' (project proj's phase); its load never arrived (no load scheduled)
    archive_milestone_view_reloads_in_place_when_planning_files_change ... FAILED
      'a planning change scheduled no reload of the open archive'
  implication: "Symptom reproduced exactly (content, then Loading... after the 20-tick block). Second defect (cross-project collision) and missing in-place reload also measured."

- timestamp: 2026-09-23
  checked: "Falsification experiment: removed ONLY the `archive_cache.retain(...)` line in prune_driver_maps, re-ran the prune test (/tmp/amvl-exp.log), then restored the file"
  found: "test ... keeps_its_content_across_the_periodic_prune ... ok (1 passed). Everything else on the tick path (session detection, reconcile, driver rescan) left in place."
  implication: "CONFIRMED: the archive_cache prune is the sole cause of the flip to Loading. `git show 392902d -- src/app.rs` adds that retain line (parent 392902d^ has no archive_cache prune) -> offending commit 392902d (feat(18-11), first in v1.7.0)."

- timestamp: 2026-09-23
  checked: "GREEN after fix: 6 archive tests + prune test (/tmp/amvl-green3.log); full suite (/tmp/amvl-test.log); clippy (/tmp/amvl-clippy.log); 25x stability loop"
  found: "6/6 archive tests + every_per_alias_driver_map_is_pruned pass. Full suite: 49 suites, 2251 passed, 1 failed (envelope::policy git-version constants test, the accepted env failure), 15 ignored. Clippy: 0 errors, same 11 pre-existing warning sites as baseline (browser.rs x3, project_creator.rs x1, tests/envelope_*.rs x7). 25/25 stability runs green."
  implication: "Fix verified; no regressions, no new lints."

- timestamp: 2026-09-23
  checked: "Seeded mutants (cargo-mutants not installed; applied by script, each restored): M1 retain_aliases drops all; M2 ArchiveLoaded keyed by milestone (old model); M3 no refresh call on planning change; M4 no PhaseList cursor clamp; M5 reload that sets archive_loading (blank-to-Loading); M6 milestone list never re-discovered"
  found: "All 6 KILLED. M1 by prune test + periodic-prune test; M2 by 4 tests incl. periodic-prune and two-project; M3 by 3 reload tests; M4 by shrink/clamp test; M5 by reload-in-place 'during' assertion; M6 by milestone-list test."
  implication: "Tests assert the root cause (key model + prune) and the no-blank reload contract, not just the symptom."

## Eliminated

- hypothesis: "5-second session poll gaining Codex detection (260923-lr7: 2054f81/5e49ba3) blanks the archive"
  evidence: "Diff 7cf6910..8d24318 touches no archive code; falsification run with ONLY the archive_cache retain removed passes while detect_sessions still runs every 20 ticks. The poll shares the 20-tick timing with the prune, which is why it looked plausible."
  timestamp: 2026-09-23

- hypothesis: "Roadmap dependency graph (260923-md1, merged 8d24318) broke the Archive render"
  evidence: "No archive/prune/view_cache hunk in the md1 range; the render path for PhaseList is unchanged since before v1.7.0; bug reproduced independent of the roadmap tab."
  timestamp: 2026-09-23

- hypothesis: "A file-watcher/notify FileChanged event clears the archive view"
  evidence: "FileChanged -> Planning -> schedule_reparse touches only project_states/last_refresh; no archive state. The reload test shows a planning change leaves the view untouched (and never reloads it)."
  timestamp: 2026-09-23

## Inferred Decisions

- [INFERRED] Fix by RE-KEYING the cache (alias -> milestone) rather than deleting the archive_cache prune. Deleting the prune would reintroduce the per-alias leak the 18-11 carry-forward (D-27) exists to prevent AND make the cross-project `v1.2` collision permanent (the prune had been masking it by emptying the map every 5s).
- [INFERRED] Introduced a small newtype `archive::ArchiveCache` (get/insert/loaded_milestones/has_alias/retain_aliases) instead of a nested `HashMap` so the key model cannot be misread again; the only prune primitive takes an alias. This touched 6 `AppContext` literal fixtures mechanically (`ArchiveCache::default()`).
- [INFERRED] In-place reload is triggered from the `FileChanged`/Planning arm AHEAD of the 500 ms dedup and is NOT gated on the changed path being under `milestones/`: the watcher emits one Planning event per (root, kind) per 200 ms batch carrying the batch's FIRST path, so a complete-milestone write can arrive named STATE.md. Cost is bounded to aliases whose Archive tab was visited (re-discovery only if `archive_milestones` is non-empty; re-load only milestones already cached).
- [INFERRED] The reload never sets `archive_loading`; the existing `ArchiveMilestonesDiscovered`/`ArchiveLoaded` handlers replace data whole, and clamp `archive_selected[0..=2]` when a reload shrinks the listing under the cursor.
- [INFERRED] FileView content (`archive_file_content`) is NOT reloaded on change — it is read synchronously once on Enter, it was never affected by the bug, and archived milestone files are read-only in the TUI. Left as-is (open item).
- [INFERRED] Split into two atomic commits: (1) the re-key fix + its two regression tests (verified independently: 2248 passed / 1 expected failure on that intermediate tree), (2) the in-place reload + its three tests.

## Resolution

root_cause: "`App::prune_driver_maps` (runs every 20 ticks, ~5 s) retained `AppContext::archive_cache` by registered ALIAS, but the map was keyed by MILESTONE version (\"v1.2\"). Every pass dropped every loaded archive; the Archive tab's PhaseList render then found no data and drew its `Loading...` fallback, and nothing re-requested the load (only Enter at MilestoneList does). Introduced by 392902d (feat(18-11), 2026-07-29, first shipped in v1.7.0), whose prune test fixture inserted alias keys and so encoded the wrong key model. Same key also let two projects that archived the same version share one entry (second project drew the first's phases). NOT the 260923-lr7 Codex session poll (2054f81/5e49ba3) — it merely shares the 20-tick timer — and NOT 260923-md1 (8d24318) or the file watcher."
fix: "3c0e38f: new `archive::ArchiveCache` newtype keyed alias -> milestone with `retain_aliases` as its only prune; ArchiveLoaded inserts under (alias, milestone); every detail.rs reader and the breadcrumb pass the screen's alias; prune test fixture and render_escape_guard fixture use both halves of the key. 581aa7d: `AppContext::schedule_archive_refresh` re-discovers the milestone list and re-loads every cached milestone for the alias on each Planning FileChanged (ahead of the 500 ms dedup, never setting archive_loading), and the Archive handlers clamp cursors when a reload shrinks a listing."
verification:
  target_test: { result: pass, test: "src/app.rs app::tests::archive_milestone_view_keeps_its_content_across_the_periodic_prune (RED before: 'flipped back to Loading... after one 20-tick pass'; GREEN after, across two passes)" }
  mutation_check: { result: pass, reason_if_skipped: "cargo-mutants not installed; 6 seeded mutants applied manually", mutant_killed: "6/6 (M1-M6, see Evidence)" }
  no_op_deletion: { result: pass, deletion_justified_by_rca: n/a, note: "fix adds a type and a refresh path; the prune is kept and made correct, nothing deleted" }
  adjacent_tests: { result: pass, suites_run: ["cargo test --no-fail-fast: 49 suites, 2251 passed, 1 failed (accepted env failure envelope::policy git-version constants), 15 ignored", "render_escape_guard (Archive tab probes) green", "clippy --all-targets: 0 errors, 11 pre-existing warning sites before and after"] }
  revert_and_reconfirm: { result: pass, bug_returned_on_revert: true, fixed_on_reapply: true, note: "same tests RED on the unfixed tree (/tmp/amvl-red.log, /tmp/amvl-red2.log) and GREEN on the fix; one-line experiment (remove only the retain) proved the prune causal; mutant M2 (restore the old milestone key under the new code) returns the flip" }
  stability: "25/25 green runs of the 6 archive tests + prune test"
  oracle_type: specified
  guardrail_verdict: accepted
files_changed:
  - src/archive.rs
  - src/app.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/render_escape_guard.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/driver_confirm.rs
regression_tests:
  - "src/app.rs app::tests::archive_milestone_view_keeps_its_content_across_the_periodic_prune"
  - "src/app.rs app::tests::the_same_milestone_version_in_two_projects_does_not_share_a_cache_entry"
  - "src/app.rs app::tests::archive_milestone_view_reloads_in_place_when_planning_files_change"
  - "src/app.rs app::tests::the_archive_milestone_list_picks_up_a_newly_archived_milestone_in_place"
  - "src/app.rs app::tests::an_in_place_reload_that_shrinks_the_listing_keeps_the_cursor_on_a_row"
prevention: "A prune written against a bare map misread its key and its test fixture copied the misreading; the cache is now a type whose only prune takes an alias, and App-level tests drive drill-in -> 20 ticks -> render so a future map-prune regression fails at the seam where it shows."
why_not_caught: "The 18-11 prune test (every_per_alias_driver_map_is_pruned) populated archive_cache with alias keys, i.e. the test shared the bug's assumption; no test crossed the drill-in -> tick -> render seam."
open_items:
  - "Phase 24 D-B04 (53b2159, committed concurrently) folds the Archive tab into Docs > Milestones and removes DetailSubView::Archive; the five regression tests above open the tab via `DetailScreen::opened_on(.., DetailSubView::Archive, ..)` and must be ported to the new entry point, not deleted."
  - "Archive FileView content is not reloaded on file change (read once on Enter). Deliberately out of scope."
  - "Index-based depths (FileList{phase_idx}, FileView{file_idx}) can point at a different row if an in-place reload reorders phases; render and key paths are bounds-checked, cursors are clamped."
