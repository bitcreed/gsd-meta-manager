---
phase: quick-260916-vqx
verified: 2026-09-17T00:00:00Z
status: passed
score: 8/8 must-haves verified
covered_files:
  - ".planning/quick/260916-vqx-show-plan-token-estimate-and-actual-counts-area-ui-severity/260916-vqx-PLAN.md"
  - ".planning/quick/260916-vqx-show-plan-token-estimate-and-actual-counts-area-ui-severity/260916-vqx-SUMMARY.md"
  - ".planning/todos/pending/2026-09-11-show-plan-token-estimate-and-actual-counts.md"
  - "src/state_reader/disk_status.rs"
  - "src/ui/screens/detail.rs"
  - "src/ui/screens/render_escape_guard.rs"
covered_digest: "v1:sha256:60604243acfe3940a02996f0f433f0f4c0757f59fcf089af664146cf187380dc"
behavior_unverified: 0
overrides_applied: 0
---

# Quick Task 260916-vqx: Show plan token estimate and actual counts — Verification Report

**Item Goal:** Show plan token estimate and actual counts (area: ui, severity: major)
**Verified:** 2026-09-17
**Status:** passed
**Commits (already merged to dev):** `ac0b203`, `853d639`, `f330b9a`

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `estimate.tokens` from `*-PLAN.md` frontmatter is parsed and surfaced per plan | ✓ VERIFIED | `frontmatter_token_count(content, "estimate")` called at `src/state_reader/disk_status.rs:816` during the existing pass-1 plan read; independently confirmed against real files (see Real-Data Probe below) |
| 2 | `actuals.tokens` from the matching `*-SUMMARY.md` is surfaced once a plan has executed | ✓ VERIFIED | `frontmatter_token_count(&content, "actuals")` at `disk_status.rs:900`, attributed only through the existing two-tier pairing (`matched_plans`); confirmed against phase 19's 33 plan/summary pairs |
| 3 | A nested key is read as a direct child only — never a column-zero twin, wrong-parent key, or two-levels-in key | ✓ VERIFIED | `leading_frontmatter_nested_value` (`disk_status.rs:423-468`) implements exactly this; named unit tests present (`disk_status.rs:2384-2464` range) and pass |
| 4 | The block reader survives real GSD shapes (blank lines, `#` comments, inline comments, closing `---`) | ✓ VERIFIED | Rule set implemented at `disk_status.rs:436-465`; `frontmatter_token_count` strips inline `#` comments (`:483-499`); `cargo test --lib state_reader::disk_status` 98 passed / 0 failed (run independently, see below) |
| 5 | Degrades gracefully when either field is absent — no wrong number, no broken row, absence is silent (not a dash column) | ✓ VERIFIED | `plan_tokens` filter at `disk_status.rs:911-925` omits a plan row unless at least one of `estimate`/`actual` is `Some`; `build_plan_token_lines` returns empty `Vec` for empty `plan_tokens` (`detail.rs:5152-5155`); confirmed by test `test_plans_without_the_keys_leave_plan_tokens_empty` (disk_status.rs:2683) and `an_empty_plan_tokens_renders_no_section_at_all` (detail.rs:11819), both passing |
| 6 | Real repo data: phase 22 yields 4 rows with estimates and no actuals; phase 19 yields 33 rows with both | ✓ VERIFIED | Independently reproduced via a throwaway integration test calling `infer_disk_status` directly on this repo's `.planning/phases/22-container-execution-target` and `.../19-gitsafe-git-blast-radius-envelope` — output matched exactly: phase 22 → 4 rows (`22-01..22-04`, `act=None` on all), phase 19 → 33 rows (`19-01..19-33`, both `Some`). Probe file deleted after use; `git status --porcelain` confirmed clean afterward. |
| 7 | Row list caps at 10 with `+N more`, but totals account for ALL plans, not just the visible 10 | ✓ VERIFIED | `total_estimate`/`total_actual`/`measured` in `build_plan_token_lines` (`detail.rs:5160-5166`) sum over `inf.plan_tokens` in full, BEFORE the `.take(MAX_PLAN_TOKEN_ROWS)` used only for row rendering (`:5191`); pinned by test asserting a 14-row fixture produces a `14.0k` header total alongside a `+4 more` line (`detail.rs` ~11930-11939) |
| 8 | No new Cargo dependency; token parsing uses the existing hand-rolled scan | ✓ VERIFIED | `git diff 2251792f..f330b9a -- Cargo.toml Cargo.lock` produces no output — zero changes to either file |

**Score:** 8/8 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/state_reader/disk_status.rs` | `leading_frontmatter_nested_value`, `frontmatter_token_count`, `PlanTokens`, `DiskInference::plan_tokens`, population inside the single scan | ✓ VERIFIED | All present and reviewed in full (lines 190-500, 780-990); wired into the one exhaustive `DiskInference` literal at `:986` |
| `src/ui/screens/detail.rs` | `fmt_tokens`, `build_plan_token_lines`, `MAX_PLAN_TOKEN_ROWS`, call site in `render_pipeline_tab` | ✓ VERIFIED | All present (lines 5095-5233); called from `render_pipeline_tab`'s `Some(inf)` arm at `:3875`, inserted between the external-job block and the waves block exactly as specified, no reordering of the waves block observed |
| `src/ui/screens/render_escape_guard.rs` | populated `phase_disk_statuses` in `hostile_project_state`, closing the Pipeline-tab fixture hole | ✓ VERIFIED | `hostile_project_state` (lines 859-938) now builds a `phase_disk_statuses` map keyed by `"1"` with a populated `PlanTokens` row; fail-first claim independently reproduced (see Behavioral Spot-Checks) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `infer_disk_status` pass 1 | `plan_estimates` → `PlanTokens.estimate` | plan file read once, reused for superseded check | ✓ WIRED | `disk_status.rs:801-819` — single `read_to_string`, both the superseded check and the estimate parse reuse it |
| `infer_disk_status` pairing loop | summary read → `PlanTokens.actual` | `matched_plans` pairing (`disk_status.rs:883-905`) | ✓ WIRED | Actual is only recorded for a summary that resolves to a surviving plan via the SAME two-tier pairing that drives `summary_count`; confirmed no second matching rule exists |
| `plan_index()` | sort key of `plan_tokens` | `.sort_by(...)` at `disk_status.rs:926-930` | ✓ WIRED | Numeric `(u32, u32)` tuple key, `u32::MAX` fallback for unparseable ids — deterministic |
| `PlanTokens.label()` | `shown()` → ratatui `Span` | `detail.rs:5198` | ✓ WIRED | `format!("{:<10}", shown(&row.label()))` — confirmed by direct fail-first reproduction below |
| `hostile_project_state.phase_disk_statuses` | `render_pipeline_tab`'s `Some(inf)` arm | key `"1"` matching `RoadmapPhase.number` | ✓ WIRED | Confirmed via independent test run of `the_screen_renders_identity_escaped` |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| `cargo test --lib state_reader::disk_status` | `rtk proxy sh -c '...'` | 98 passed / 0 failed | ✓ PASS |
| `cargo test --lib ui::screens::detail` | `rtk proxy sh -c '...'` | 90 passed / 0 failed | ✓ PASS |
| `cargo test --lib ui::screens::render_escape_guard` | `rtk proxy sh -c '...'` | 14 passed / 0 failed | ✓ PASS |
| `cargo clippy -- -D warnings` | `rtk proxy cargo clippy -- -D warnings` | exit 0 | ✓ PASS |
| Fail-first proof of `shown()` escaping on the Pipeline tab label | Independently reverted `shown(&row.label())` → `row.label()` at `detail.rs:5198`, ran `cargo test --lib ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped -- --exact`, then restored the file from a backup copy | Test FAILED naming `DetailScreen (src/ui/screens/detail.rs) [Pipeline tab]` rendering `['\u{e0041}']` unescaped — exact match to SUMMARY's claimed verbatim output. Restored file, re-ran: `1 passed / 0 failed`. `git status --porcelain` confirmed clean after restore. | ✓ PASS |
| Real-data smoke against this repo's own `.planning/phases/22-container-execution-target` and `19-gitsafe-git-blast-radius-envelope` | Wrote a throwaway `tests/verifier_probe_vqx.rs` calling `infer_disk_status` directly on both phase directories, ran `cargo test --test verifier_probe_vqx -- --nocapture`, then deleted the file | Phase 22: `plan_count=4 summary_count=0 rows=4`, all four rows `est=Some(N) act=None` (95000/80000/78000/82000). Phase 19: `plan_count=33 summary_count=33 rows=33`, every row has both `Some(estimate)` and `Some(actual)`. Matches SUMMARY's claim exactly. `git status --porcelain` confirmed clean after deletion. | ✓ PASS |
| Full `--no-fail-fast` suite (run once) | `rtk proxy sh -c 'cargo test --no-fail-fast 2>&1 | tee <log> | grep -E "^test result:"'` | 2051 passed total (SUMMARY claimed 2052 — within noise of the flake below); one binary showed 2 additional failures beyond the documented single environmental failure | ⚠️ See Anti-Patterns / Info note below |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `tests/driver_reattach.rs` | `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`, `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired` | Test-parallelism flake — both failed in the full `--no-fail-fast` run but pass cleanly (`3 passed / 0 failed`) under `cargo test --test driver_reattach -- --test-threads=1` | ℹ️ Info | Not a regression from this phase. `tests/driver_reattach.rs` is untouched by any of the three vqx commits and exercises process-liveness scanning unrelated to `disk_status.rs`/`detail.rs`/`render_escape_guard.rs`. Independently confirmed pre-existing and environment-sensitive (shared-resource contention under parallel test execution), not caused by this phase's code. |
| — | — | Debt markers (`TBD`/`FIXME`/`XXX`) | — | None found in the three modified files. The only `XXXX` occurrences are documentation of Unicode escape notation (`U+XXXX`), not debt markers. |

No stub patterns, empty implementations, or hardcoded-empty-data patterns were found in the reviewed code — every function traced to a real, exercised code path.

### Requirements Coverage

This is a quick-batch item (`requirements: [QUICK-260916-vqx]`), not tied to a milestone `REQUIREMENTS.md` entry. No orphaned requirements apply.

### Human Verification Required

None. Every must-have was settled with direct evidence: code reading, independent test execution, an independently-reproduced fail-first proof, and an independently-run real-data probe against this repository's own `.planning/phases/`.

### Gaps Summary

No gaps. All 8 observable truths verified with first-hand evidence (not just SUMMARY claims). The single deviation worth flagging is an unrelated, pre-existing test-parallelism flake in `tests/driver_reattach.rs` (untouched by this phase, passes in isolation) — noted as informational, not a blocker.

---

_Verified: 2026-09-17_
_Verifier: Claude (gsd-verifier)_
