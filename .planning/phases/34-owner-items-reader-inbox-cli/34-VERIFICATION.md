---
phase: 34-owner-items-reader-inbox-cli
verified: 2026-11-14T20:10:00Z
status: human_needed
score: 5/7 must-haves verified
covered_files:
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-01-PLAN.md"
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-01-SUMMARY.md"
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-02-PLAN.md"
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-02-SUMMARY.md"
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-03-PLAN.md"
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-03-SUMMARY.md"
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-04-PLAN.md"
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-04-SUMMARY.md"
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-05-PLAN.md"
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-05-SUMMARY.md"
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-06-PLAN.md"
  - ".planning/phases/34-owner-items-reader-inbox-cli/34-06-SUMMARY.md"
  - "README.md"
  - "src/cli_targets.rs"
  - "src/inbox.rs"
  - "src/inbox_cli.rs"
  - "src/state_reader/disk_status.rs"
  - "src/state_reader/owner_items/deferred.rs"
  - "src/state_reader/owner_items/frontmatter.rs"
  - "src/state_reader/owner_items/markdown.rs"
  - "src/state_reader/owner_items/mod.rs"
  - "src/state_reader/owner_items/rules.rs"
  - "src/state_reader/owner_items/scan.rs"
  - "src/state_reader/owner_items/uat.rs"
  - "src/state_reader/owner_items/verification.rs"
  - "src/status_cli.rs"
  - "tests/inbox_cli.rs"
  - "tests/owner_audit_conformance.rs"
  - "tests/owner_digest.rs"
  - "tests/status_cli.rs"
covered_digest: "v4:sha256:8db85b705d3e978b8f92d7a4104a5605182c04a74e7607e27b457fae66ce74ef"
behavior_unverified: 2
overrides_applied: 0
behavior_unverified_items:
  - truth: "The watcher invalidates the parse cache: a FileChanged action for a project drops that project's cached parses (OWN-12, plan 34-03)"
    test: "In the TUI (or a unit test around App::update), send Action::FileChanged for a project whose owner files were parsed and cached, then read the owner digest again after rewriting a file"
    expected: "The next read re-parses the rewritten file; no stale parse survives the event"
    why_human: "evict_project() is unit-tested (scan.rs evicting_a_project_drops_only_its_entries) and the call sits in the FileChanged arm at src/app.rs:2736, but no test dispatches FileChanged through app.rs and checks the cache. Presence and wiring are visible to grep, the transition is not. No TUI code reads the cache in this phase (only the one-shot CLI does), so the practical risk is low until Phase 36 consumes it."
  - truth: "A file rewritten while a scan runs is read before or after the write, never as a panicking mix; a marker written by gsd-core between two inbox runs is honored by the next run (plan 34-03 and 34-06 concurrency truths, verification: backstop)"
    test: "Rewrite a UAT file in a loop while running inbox --json repeatedly; write an audit_acknowledged marker between two runs"
    expected: "No panic or torn parse in any run; the second run honors the marker"
    why_human: "Both truths are tagged verification: backstop in the plans (non-inferable by presence and wiring). The code reads each file through one capped open and keys the cache by (mtime, len), which supports the claim, but no held-out or property test exercises the race."
human_verification:
  - test: "Re-run the live check on a cold OS page cache with the release binary: gsd-meta-manager inbox --json over the owner's registered projects"
    expected: "Wall time under 1 s. This verifier measured 0.29-0.33 s warm (release), 0.95-0.99 s warm (debug), and 177 ms in the strict synthetic 20-project test (release). The first debug run after a fresh build took 10.7 s wall with only 1.3 s of CPU, which points at cold-disk I/O rather than parsing, but it could not be reproduced warm."
    why_human: "Cold page-cache timing needs root (drop caches) or a reboot; the default timing test is a 10 s hang detector by design (WR-05) and only enforces 1 s with GSD_MM_STRICT_TIMING=1."
  - test: "Read the plain `gsd-meta-manager inbox` checklist for your own projects and judge whether it answers 'what checks do I have to run?' (wording, ordering, noise)"
    expected: "The lane sections, ordering (closes-a-phase first) and one-line group format read well enough to paste into a note"
    why_human: "Subjective readability; the golden output only pins the current format."
deferred: []
---

# Phase 34: Owner items reader + inbox CLI Verification Report

**Phase Goal:** On any machine, `gsd-meta-manager inbox` answers "what checks do I have to run?" for every registered project in one call, read natively from the files (no gsd-core process spawned), with per-item results that match gsd-core's `audit-uat` and the owner's rules (lanes, dedup, acknowledged, archived and deferred handling, closeable-by-testing) applied on top.
**Verified:** 2026-11-14T20:10:00Z
**Status:** human_needed
**Re-verification:** No, initial verification

## Goal Achievement

The goal is achieved in the code. Two truths are present and wired but not behaviorally exercised (watcher-driven cache eviction through `app.rs`, and the two `backstop` concurrency truths). They route to human verification and are not failures. No gaps.

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: `inbox` lists missing UATs for all registered projects in one call; wordoclock reports 18 open checks (not 39 or 41); projects without a phases dir return empty groups, not an error | VERIFIED | Live `inbox --json`: 17 registered projects (the registry has 17 now, not 20), 17 entries, every `error` null. wordoclock: test 12 + blocked 5 + fix 1 = 18 open checks (also pinned on the supersede-six-phases fixture in `tests/owner_digest.rs`, 16 tests green). bitcreed.us and usbeehive (no phases dir) return `groups: []`, `error: null`. starlink-rv-tracker is no longer registered, so it could not be checked live. |
| 2 | SC2: per-item (file, test, result, category) equals `audit-uat --raw` on every fixture; oracle test green in container pre-tag; a source scan proves nothing under `src/` spawns `audit-uat` | VERIFIED | `tests/owner_audit_conformance.rs` run here with the real oracle: "16 compared, 2 declared divergences, over 18 fixtures" (the 2 are no-phases-dir and out-of-set-verification, both asserted against the oracle's specific error text). Missing oracle fails unless `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` is set; zero comparisons fails. `no_source_file_names_the_audit_oracle_verb` green, and a grep of `src/` for process spawning in `inbox*.rs` and `owner_items/` found none. Container rehearsal green per 34-05-SUMMARY (not re-run, as instructed). |
| 3 | SC3: acknowledged files hidden while the snapshot matches and resurface on change or malformed marker; archived and deferred never enter current counts; a `status: testing` UAT with open items counts as outstanding | VERIFIED | Four acknowledgment fixtures (match, count-changed, status-changed, missing-at) compared with the oracle's `acknowledged_files`; archived-complete-skipped (archived test 10, blocked 4, current 0) and deferred-200 (hidden.deferred 200, zero groups) in `tests/owner_digest.rs`; `tests/status_cli.rs` `a_testing_uat_fires_uat_outstanding`, `a_non_terminal_uat_with_open_rows_fires_uat_outstanding`, `a_complete_or_deferred_uat_with_open_rows_does_not_fire` all green (27/27). Live: summary shows archived {blocked 15, fix 14, test 33} and deferred 326 separate from current test 97. R7 eight-combination table test `r7_closeable_table` green. |
| 4 | SC4: `inbox --json` is deterministic schema v1 with `status`-style exit codes; plain `inbox` prints a lane checklist with working `--lane`/`--include`; `status --json` carries additive `owner` counts documented in the skill | VERIFIED | Three consecutive release `inbox --json` runs byte-identical. `schema: gsd-meta-manager/inbox`, `schema_version: 1`. Unresolved name gives an error entry and exit 0 (same as `status`); `--lane bogus` exits 2. `--lane test` and `--include archived,deferred,acknowledged,todos` change the listed groups (140 vs 145 total groups live). Plain output prints `TEST`, `FIX`, `BLOCKED` sections with a summary line. `status --json` live: `owner {test, decide, fix_phases, blocked, closeable_phases}` per project (mailbot 6/0/3/0/2, wordoclock 12/0/1/5/3, null on error entries), schema_version still 1. `gsd-meta-manager --skill` documents `inbox --json` and the `owner` counts. `tests/inbox_cli.rs` (22) pins 69 key paths against the README fence and table; `inbox_writes_nothing_anywhere` green. |
| 5 | SC5: a cold `inbox --json` over 20 projects takes under 1 s, parses are cached by (path, mtime, len), and hostile inputs neither panic nor escape `render_for_terminal` | VERIFIED | Strict run: `GSD_MM_STRICT_TIMING=1 cargo test --release --test inbox_cli a_cold_inbox` gives 177 ms CLI cold and 125 ms lib loop on the synthetic 20-project tree. Parse cache keyed by (path, kind) with a (mtime, len) `CacheKey`, lock never held over I/O. Reads go through a capped, contained open (`READ_TEXT_CAP`, canonicalized containment bounded to the planning root, FIFO and symlink-escape handled), covered by `inbox_cli` hostile-string test and owner_items unit tests (135 green). Every project string passes `terminal_capped`. `read_owner_digest(` is listed in the `async_blocking_guard` helpers and no TUI render path calls it. Caveat: the default timing test only enforces a 10 s hang bound (WR-05); the 1 s bound is opt-in. See human_verification for the cold-disk observation. |
| 6 | Watcher invalidates the parse cache (OWN-12 "invalidated by the watcher") | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | `evict_project` exists, `evict_under` is unit-tested, and `src/app.rs:2736` calls it in the `Action::FileChanged` arm. No test drives that arm. No TUI consumer reads the cache yet. |
| 7 | Concurrency invariants tagged `backstop` (file rewritten mid-scan; acknowledgment marker written by gsd-core between runs) | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED (insufficient_spec) | Design supports both (single capped open per file, (mtime, len) cache key, nothing persisted by the app), but no test exercises either race. Abstains per the non-inferable rule. |

**Score:** 5/7 truths verified (2 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/state_reader/owner_items/{mod,scan,uat,verification,deferred,rules,markdown,frontmatter}.rs` | Native Layer A reader and Layer B rules | VERIFIED | 7,583 lines total with `inbox*.rs`; no stubs, no TODO/HACK/todo!/unimplemented! in changed `src/` files |
| `src/inbox.rs`, `src/inbox_cli.rs`, `src/cli_targets.rs` | Schema v1 document, text renderer, CLI runner, shared targets | VERIFIED | Wired in `main.rs`/`cli.rs`; live output confirmed |
| `src/status_cli.rs` `owner` counts, `src/state_reader/disk_status.rs` `Testing` | Additive status counts; `testing` outstanding | VERIFIED | Live and test confirmed |
| `tests/owner_audit_conformance.rs` + 18 fixtures under `tests/fixtures/owner/` | Oracle comparison | VERIFIED | Ran against the real oracle |
| `tests/owner_digest.rs`, `tests/fixtures/owner_golden/*.txt`, README `inbox` section | Owner-number and golden pins, contract docs | VERIFIED | 16 and 22 tests green |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `main.rs`/`cli.rs` | `inbox_cli::run` | `inbox` subcommand | WIRED | Binary lists and runs `inbox` |
| `inbox_cli.rs` and `status_cli.rs` | `read_owner_digest` | one shared digest per project | WIRED | `src/status_cli.rs:232`, `src/inbox_cli.rs:67` |
| `app.rs` FileChanged | `owner_items::evict_project` | watcher event | WIRED (untested) | `src/app.rs:2736`; see truth 6 |
| `owner_items::scan` | disk_status/`parse_project_state` | `ProjectState` for R7 | WIRED | `closes_phase` logic driven by disk statuses |
| driver router G7 | `UatStatus::is_upstream_blocking` | `testing` stays non-blocking | WIRED | `driver_router_conformance` green |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `inbox --json` groups | `OwnerDigest.groups` | file reads of UAT/VERIFICATION/deferred-items.md | Yes, live output shows real project items | FLOWING |
| `status --json` `owner` | `OwnerDigest` counts | same digest | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full suite | `cargo test --locked --no-fail-fast` | 3264 lib tests plus all integration binaries passed; one failure, `multiplexer::tmux::tests::cli::the_reveal_of_a_terminal_in_no_pane_...`, with `Text file busy (os error 26)` | PASS (one unrelated flake) |
| That flake alone | `cargo test --lib the_reveal_of_a_terminal_in_no_pane` | passed on rerun; the test is in `src/multiplexer`, untouched by this phase | PASS |
| Oracle conformance | `cargo test --test owner_audit_conformance -- --nocapture` | 16 compared, 2 declared divergences, 18 fixtures | PASS |
| Strict cold timing | `GSD_MM_STRICT_TIMING=1 cargo test --release --test inbox_cli a_cold_inbox` | 177 ms / 125 ms | PASS |
| Determinism | three `inbox --json` runs | byte-identical | PASS |
| Exit codes | `--lane bogus`; unresolved name | 2; 0 with error entry | PASS |

### Probe Execution

SKIPPED: the phase declares no probe scripts (`scripts/*/tests/probe-*.sh` not referenced by any plan).

### Requirements Coverage

Every ID in the phase's roadmap line appears in at least one PLAN `requirements:` field, and REQUIREMENTS.md marks all 13 as Phase 34 / Complete. OWN-08 and OWN-10 map to Phase 35 and OWN-16..23 to Phases 36 and 37, so none are orphaned here.

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|----------------|-------------|--------|----------|
| OWN-01 | 01, 02, 03, 06 | Native per-item reader, oracle-equal | SATISFIED | 16 fixtures equal the oracle |
| OWN-02 | 01, 03 | Oracle pattern test and source scan | SATISFIED | Conformance test and scan green |
| OWN-03 | 03, 04, 05 | R2 supersede; 18 not 39 | SATISFIED | supersede-six-phases fixture, live wordoclock 18 |
| OWN-04 | 02, 04, 05 | R3 gaps merge | SATISFIED | uat-gaps-merge: one Fix group, 10 failed and 14 gaps |
| OWN-05 | 05, 06 | R4 acknowledged | SATISFIED | Four fixtures plus oracle `acknowledged_files` |
| OWN-06 | 03, 04, 05 | R5/R6 archived and deferred | SATISFIED | archived-complete-skipped, deferred-200 |
| OWN-07 | 04 | R7 closes_phase table | SATISFIED | `r7_closeable_table`, eight cases |
| OWN-09 | 04 | `testing` is outstanding | SATISFIED | status_cli and disk_status tests |
| OWN-11 | 01, 02, 03, 05, 06 | Safety | SATISFIED | Hostile tests green; WR-01..04 fixes in place |
| OWN-12 | 03, 05 | Cache, watcher invalidation, < 1 s | SATISFIED with caveat | Cache and timing verified; watcher call-site untested (truth 6) |
| OWN-13 | 01, 05 | `inbox --json` schema v1 | SATISFIED | 69 key paths pinned; writes nothing |
| OWN-14 | 05 | Plain checklist and filters | SATISFIED | Golden output and live runs |
| OWN-15 | 04 | `owner` counts and skill text | SATISFIED | Live `status --json`, `--skill` text |

### Anti-Patterns Found

None blocking. Debt-marker scan over the files changed in this phase found `TBD` only inside pre-existing roadmap-shape strings (`**Plans**: TBD`, `0/TBD`), which are data, not markers. No `TODO`/`HACK`/`unimplemented!`. Review findings CR-01 and WR-01..05 are fixed (34-REVIEW-DISPOSITION.md); IN-02 (item id collision after truncation), IN-04 (cache bounded by entry count, not bytes) and IN-05 (zero todos reported when listing failed) remain open as info only, non-blocking.

### Human Verification Required

1. **Cold-disk wall time**: see `human_verification` in the frontmatter. The first debug run took 10.7 s wall for 1.3 s CPU. Warm runs are all under 1 s.
2. **Checklist readability**: subjective judgment of the plain output.
3. **Behavior-unverified truths 6 and 7**: see `behavior_unverified_items` in the frontmatter.

### Gaps Summary

No gaps. All five roadmap success criteria hold in the code and in live output. Status is `human_needed` only because two truths (watcher eviction through `app.rs`, and the two `backstop` concurrency invariants) are present and wired without a test that exercises the transition, plus two subjective or hardware-dependent checks. Suggested follow-up, not blocking: add one `App::update(FileChanged)` test that proves eviction, ahead of Phase 36 consuming the cache.

---

_Verified: 2026-11-14T20:10:00Z_
_Verifier: Claude (gsd-verifier)_
