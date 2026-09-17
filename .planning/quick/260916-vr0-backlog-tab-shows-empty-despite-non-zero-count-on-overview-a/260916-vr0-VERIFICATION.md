---
phase: quick-260916-vr0
verified: 2026-09-17T00:00:00Z
status: passed
score: 6/6 must-haves verified
covered_files:
  - ".planning/quick/260916-vr0-backlog-tab-shows-empty-despite-non-zero-count-on-overview-a/260916-vr0-PLAN.md"
  - ".planning/quick/260916-vr0-backlog-tab-shows-empty-despite-non-zero-count-on-overview-a/260916-vr0-SUMMARY.md"
  - ".planning/todos/pending/2026-09-15-backlog-tab-shows-empty-despite-non-zero-count-on-overview.md"
  - "src/state_reader/backlog.rs"
  - "src/state_reader/mod.rs"
  - "tests/state_reader_test.rs"
covered_digest: "v1:sha256:9b34226f87c23f138e10cddfdbf991600ab83d83e03021a83f9581f1abff7675"
behavior_unverified: 0
overrides_applied: 0
---

# Quick 260916-vr0: Backlog tab shows empty despite non-zero count on overview — Verification Report

**Item goal:** Fix the Backlog tab rendering empty while the overview advertises a non-zero
backlog count for the same project.
**Verified:** 2026-09-17 (working tree at `dev`, commits 02c7dee, b566e50, f1cd6e9 already
merged at 2251792)
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Original User-Visible Symptom — Settled Directly

Rather than trusting SUMMARY.md's claim, I wrote a throwaway integration test
(`tests/zz_verify_scratch_test.rs`, deleted immediately after running — not part of
the shipped change) that calls `count_backlog_items` and `parse_backlog_items`
against the **real** `.planning/phases/` directories on this machine, and ran it:

```
running 1 test
/home/blk/projects/flutter/hm-relverify: count=1 parsed=1 agree=true
  - path=None content_present=false
/home/blk/projects/flutter/hitchmatch: count=1 parsed=1 agree=true
  - path=None content_present=false
/home/blk/projects/python/picsync: count=2 parsed=2 agree=true
  - path=None content_present=false
  - path=None content_present=false
/home/blk/projects/web/shopify-orderly-rescue: count=3 parsed=3 agree=true
  - path=None content_present=false
  - path=None content_present=false
  - path=None content_present=false
/home/blk/projects/rust/gsd-meta-manager: count=4 parsed=4 agree=true
  - path=None content_present=false
  - path=None content_present=false
  - path=None content_present=false
  - path=None content_present=false
test verify_real_projects_agree ... ok
```

11 real `999.*` directories across 5 registered projects (the 6th project the
executor measured, `cdr-configurator`, was not found by `find` at verification
time — likely moved/deregistered since planning; not a defect of this fix) — in
every case `count == parsed.len()` and every item survived with `path: None` (no
`.md` present, exactly the measured on-disk shape) rather than being silently
dropped. This is the exact symptom from the todo, settled against live data, not
inferred from the SUMMARY's narrative.

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A `999.N-slug` directory with no `.md` file is RETURNED by `parse_backlog_items`, described by its humanized slug, with `path: None` / `content: None` | VERIFIED | `src/state_reader/backlog.rs:116-133` — `find_first_md_file` result is stored in `path` as an `Option` rather than gating survival via `?`; `description` falls back to `humanize_slug(&slug)` at line 120. Confirmed live against 11 real directories above — all `path: None`, all present. |
| 2 | `count_backlog_items(d)` equals `parse_backlog_items(d).len()` for every directory | VERIFIED | Both delegate to the single `backlog_dirs()` function (`backlog.rs:70-98`); `count_backlog_items` at `mod.rs:606-608` is `backlog::count_backlog_dirs(planning_dir) as u32`. Confirmed live (above) and via `the_overview_count_equals_the_number_of_rows_the_backlog_tab_can_draw` and the 6-shape sweep `the_backlog_count_and_the_backlog_list_come_from_one_rule_across_every_fixture_shape`, both green. |
| 3 | The two rules are ONE rule in ONE place | VERIFIED | `backlog_dirs()` (`backlog.rs:70-92`) is the sole matcher; `count_backlog_dirs` (line 96-98) and `parse_backlog_items` (line 103-148) both call it. `mod.rs`'s old independent `read_dir`+`starts_with("999")` implementation is gone — `git diff` confirms it was replaced with a one-line delegation. |
| 4 | Every text field of a returned `BacklogItem` is still constructed through `Untrusted::from_untrusted_source` at the single existing construction site | VERIFIED | `backlog.rs:127-133` — `dir_name`, `number`, `description` all wrapped at construction; no `Display`/`AsRef<str>` added to `BacklogItem` (struct doc at line 5-21 explicitly notes it implements none of those). |
| 5 | The `No backlog items found.` empty state is reachable only when the project genuinely has no `999.*` directory | VERIFIED | `backlog_dirs()` returns `Vec::new()` only when `read_dir(phases_dir)` errors (line 72-75) or no entry passes the `999`-prefix + `is_dir` + `parse_backlog_dir_name` filters (line 78-91) — i.e. only when there is truly nothing to draw. Test `test_count_backlog_missing_phases_dir` and `test_count_backlog_zero` cover both empty paths, both green. |
| 6 | Well-formed backlog items that already displayed keep their existing order and description | VERIFIED | `backlog_number_ordering` (unchanged, `backlog.rs:230-234`) is still the sort; `well_formed_backlog_numbers_keep_the_order_they_had_before_the_total_order_fix` and the 9 pre-existing `state_reader::backlog` unit tests (WR-05/WR-07 total-order suite) all pass unchanged — confirmed by running `cargo test --lib state_reader::backlog` (9 passed, 0 failed). |

**Score:** 6/6 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/state_reader/backlog.rs` | shared directory matcher + `.md` requirement removed | VERIFIED | `backlog_dirs`/`count_backlog_dirs` added (lines 70-98), `parse_backlog_items` rewritten to build from it (103-148), `?`-gated `find_first_md_file` removed from the survival path |
| `src/state_reader/mod.rs` | `count_backlog_items` delegating (body-only) | VERIFIED | `git diff` confirms only the function body changed; signature, doc-comment location, and surrounding code untouched otherwise; no new item added after the `#[cfg(test)]` marker at line 610 |
| `tests/state_reader_test.rs` | count/parse agreement invariant | VERIFIED | 6 new tests present and green (see below) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `count_backlog_items` | shared matcher | `mod.rs:607` calls `backlog::count_backlog_dirs` | WIRED | grep + read confirmed |
| `parse_backlog_items` | shared matcher | `backlog.rs:106` calls `backlog_dirs(planning_dir)` | WIRED | read confirmed |
| `parse_backlog_items` | `ProjectViewCache.backlog_items` -> `render_backlog_tab` | unchanged downstream call site | WIRED (unchanged) | Per plan's confirmed root cause, this path already existed; only the upstream data source changed. Not re-verified line-by-line since no UI file was touched (see below), and PLAN's cited consumer behavior (`detail.rs:2985`, `:3002`, `:3564`) already handled `path: None`/`content: None` before this fix. |
| `BacklogItem.path: None` | `detail.rs:2985` SuspendAndEdit guard | `if let Some(ref path)` | Present pre-existing, now reachable | Not re-verified by re-reading `detail.rs` since the plan's D-INF-04 explicitly kept this file untouched; confirmed via `git diff` that no UI file appears in the three merged commits. |

### Behavioral Spot-Checks / Direct Evidence

| Check | Command | Result | Status |
|-------|---------|--------|--------|
| Count/parse agreement on real on-disk data | scratch `tests/zz_verify_scratch_test.rs`, run then deleted | 11/11 directories agree, all previously-dropped items now present | PASS |
| Backlog-focused test suite | `rtk proxy cargo test --test state_reader_test backlog` | 10 passed, 0 failed | PASS |
| Pre-existing unit tests (WR-05/WR-07 total-order) | `rtk proxy cargo test --lib state_reader::backlog` | 9 passed, 0 failed | PASS |
| Spawn-seam guard (D-INF-03) | `rtk proxy cargo test --test spawn_seam_guard no_production_item_follows_a_test_module_marker` | 1 passed | PASS |
| Clippy gate | `rtk proxy cargo clippy -- -D warnings` | exit 0 | PASS |
| No UI file modified | `git diff --stat 3fa669e..f1cd6e9` | only `src/state_reader/backlog.rs`, `src/state_reader/mod.rs`, `tests/state_reader_test.rs` changed | PASS |
| No debt markers in modified files | `grep -n -E "TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER"` | no matches | PASS |

### Requirements Coverage

| Requirement | Source | Description | Status | Evidence |
|-------------|--------|--------------|--------|----------|
| QUICK-260916-vr0 | PLAN frontmatter | Backlog tab renders items matching overview count | SATISFIED | All truths above verified; live data check confirms the fix on this machine's actual registered projects |

Todo file `2026-09-15-backlog-tab-shows-empty-despite-non-zero-count-on-overview.md` cited four
files/ranges as the diagnostic evidence: `mod.rs:598-615` (count logic — now delegates),
`backlog.rs:34-44` (name grammar — unchanged, correctly, per D-INF-01), `backlog.rs:49-98`
(the `.md`-gated parser — rewritten), `detail.rs:1019-1029` (cache population — untouched,
correctly, since the bug was upstream of it). All four are accounted for by the fix or by an
explicit, evidenced decision not to touch them.

### Anti-Patterns Found

None. No debt markers, no stub returns, no hardcoded empty data flowing to the renderer.

### Inferred Decisions Audit (D-INF-01 through D-INF-04)

All four are honored in the actual diff, not merely claimed in the SUMMARY:

- **D-INF-01** (fix `.md` requirement, not name grammar) — `parse_backlog_dir_name` is
  byte-identical to before (confirmed via `git diff`, no changes to lines 34-44).
- **D-INF-02** (count follows the strict parser) — confirmed by
  `a_backlog_name_the_strict_parser_rejects_is_counted_as_zero_and_parsed_as_zero`, green.
- **D-INF-03** (shared matcher in `backlog.rs`, `mod.rs` body-only) — confirmed by `git diff`
  showing only a function-body replacement in `mod.rs`, and the spawn-seam guard test passing.
- **D-INF-04** (no UI file touched) — confirmed: `git diff --stat` across all three commits
  touches exactly `src/state_reader/backlog.rs`, `src/state_reader/mod.rs`,
  `tests/state_reader_test.rs`. No `src/ui/` file appears.

All four decisions were reasonable given the measured evidence (0 of 12+ real backlog
directories contain a `.md`, 0 of 12+ names are malformed) and carry no scope risk — no
override needed.

### Human Verification Required

None. Every must-have was settled with direct evidence: reading the diff, running the
project's own test gates, and — for the core symptom — running a throwaway check against
this machine's actual `.planning/phases/999.*` directories rather than trusting the
SUMMARY's narrative or its captured (but not independently reproduced) test output.

### Gaps Summary

None. All 6 must-have truths verified, all 3 artifacts verified, all key links wired
(with two downstream links reasonably not re-derived since D-INF-04 kept that file
untouched and the plan's root-cause analysis already established those call sites handle
`None` correctly — pre-existing, unmodified code). Full test suite delta consistent with
plan's claim (baseline 2005/1 documented pre-existing environmental failure; this
verification independently re-ran the specific relevant gates rather than the full
`--no-fail-fast` sweep, since the phase touches only `state_reader` and the targeted gates
plus a live-data check are stronger evidence for this specific bug than a full-suite rerun).

---

_Verified: 2026-09-17_
_Verifier: Claude (gsd-verifier)_
