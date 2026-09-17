---
phase: quick-260916-vr0
plan: 01
subsystem: state_reader
tags: [backlog, bugfix, tdd, invariant, untrusted]
status: complete
requires: []
provides:
  - "state_reader::backlog::backlog_dirs — the single shared backlog matcher"
  - "state_reader::backlog::count_backlog_dirs — its length, for the counter"
affects:
  - "src/state_reader/mod.rs::count_backlog_items (body-only delegation)"
  - "Backlog tab rendering in src/ui/screens/detail.rs (behaviour only, file untouched)"
tech-stack:
  added: []
  patterns:
    - "One matching rule in one place, with a test that fails if it is re-split"
key-files:
  created: []
  modified:
    - src/state_reader/backlog.rs
    - src/state_reader/mod.rs
    - tests/state_reader_test.rs
decisions:
  - "D-INF-01: fix the .md requirement, not the name grammar — branch 1 measured absent (0 of 12 names malformed)"
  - "D-INF-02: reconcile toward the STRICT rule — the count follows the parser, because the count's only job is to say how many rows the tab will draw"
  - "D-INF-03: the shared matcher lives in backlog.rs so mod.rs gets a body-only edit, keeping the spawn-seam guard satisfied"
  - "D-INF-04: touch no UI file — every consumer already handles path: None and content: None"
metrics:
  duration: ~35m
  completed: 2026-09-17
actuals:
  tokens: 4536
  tasks: 3
  commits: 3
plan_head_before: 3fa669e880dfd26df5254a9e829a557f45d2dfd6
---

# Quick 260916-vr0: Backlog tab shows empty despite non-zero count on overview — Summary

The overview's backlog count and the Backlog tab's row list were two different
matching rules; they are now one function, and the `.md`-file requirement that
discarded 100% of real backlog directories is gone.

## What Was Wrong

The overview counted any entry under `.planning/phases/` whose name merely
`starts_with("999")`. `parse_backlog_items` applied that same prefix rule, ran
the name through the strict `parse_backlog_dir_name`, **and additionally
required a `.md` file inside the directory** — via a `?` on `find_first_md_file`
sitting inside a `filter_map`, so a directory with no markdown was silently
dropped.

The plan established at planning time, and this execution re-confirmed against
the fixture, that the extra requirement is not a partial filter but a total one:
across all six registered projects, 12 of 12 `999.*` directories contain exactly
one entry — `.gitkeep` — and **zero** contain a `.md` file. So the parser
returned an empty `Vec` for every project that had a backlog at all, while the
overview cheerfully advertised 4. The comment at `backlog.rs:69` calling
md-less directories "empty backlog placeholders" was itself the defect: a
backlog item's entire payload is its DIRECTORY NAME, and a markdown body is an
optional enrichment that in practice never exists.

## What Changed

`src/state_reader/backlog.rs`
- New `pub fn backlog_dirs(planning_dir) -> Vec<(String, String, String)>` — the
  **sole** definition of which directories are backlog items. One `read_dir`,
  the prefix filter, the `is_dir` filter, then `parse_backlog_dir_name`. It
  performs no file reads beyond that single `read_dir`, because the counter runs
  on the dashboard refresh path for every registered project.
- New `pub fn count_backlog_dirs(planning_dir) -> usize` — that collection's
  length.
- `parse_backlog_items` now builds from `backlog_dirs`. The fix itself is the
  removal of the `?`: the first `.md` is resolved into an `Option` and stored in
  `path` as-is, instead of deciding whether the item survives. `description`
  still prefers the first `# heading` and falls back to `humanize_slug`.
- The stale line-69 comment is replaced by one recording the measured fact.
- Both functions sit immediately after `parse_backlog_dir_name` and strictly
  above the `#[cfg(test)]` marker (D-INF-03).
- `backlog_number_ordering` and the WR-05/WR-07 total-order work are untouched.

`src/state_reader/mod.rs`
- `count_backlog_items` keeps its signature and loses its body to
  `backlog::count_backlog_dirs(planning_dir) as u32`. No new item added to this
  file, and the function was not moved.

`tests/state_reader_test.rs`
- Six new tests (detail below).

**No UI file was modified**, as planned. `BacklogItem.path` was already
`Option<PathBuf>`, `content` already `Option<Untrusted>`, `detail.rs:3564`
already renders a fallback for absent content, and `detail.rs:2985` already
guards `SuspendAndEdit` behind `if let Some(ref path)`.

## Tests Added

| Test | Pins |
|---|---|
| `md_less_backlog_directories_reach_the_tab_instead_of_being_discarded` | the reported symptom |
| `the_overview_count_equals_the_number_of_rows_the_backlog_tab_can_draw` | the invariant |
| `a_backlog_item_with_no_md_file_is_described_by_its_humanized_slug` | the description fallback, `path: None`, `content: None` |
| `the_backlog_count_and_the_backlog_list_come_from_one_rule_across_every_fixture_shape` | six fixture shapes in one sweep, both functions driven from the same path in the same assertion |
| `a_backlog_name_the_strict_parser_rejects_is_counted_as_zero_and_parsed_as_zero` | the reconciliation DIRECTION (D-INF-02) |
| `a_backlog_directory_holding_a_md_heading_is_described_by_that_heading` | the enrichment direction — optional must not mean ignored |

## The Verbatim RED

### Task 1 (three tests, against the unmodified tree)

```text
running 7 tests
test test_count_backlog_missing_phases_dir ... ok
test test_count_backlog_ignores_files_starting_with_999 ... ok
test test_count_backlog_two ... ok
test test_count_backlog_zero ... ok
test md_less_backlog_directories_reach_the_tab_instead_of_being_discarded ... FAILED
test the_overview_count_equals_the_number_of_rows_the_backlog_tab_can_draw ... FAILED
test a_backlog_item_with_no_md_file_is_described_by_its_humanized_slug ... FAILED

failures:

---- md_less_backlog_directories_reach_the_tab_instead_of_being_discarded stdout ----

thread 'md_less_backlog_directories_reach_the_tab_instead_of_being_discarded' (4107333) panicked at tests/state_reader_test.rs:240:5:
assertion `left == right` failed: overview counted 2, the tab could draw 0. A backlog directory's payload is its NAME; requiring a `.md` file inside it discards 100% of the directories that exist on real disks.
  left: 0
 right: 2
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- the_overview_count_equals_the_number_of_rows_the_backlog_tab_can_draw stdout ----

thread 'the_overview_count_equals_the_number_of_rows_the_backlog_tab_can_draw' (4107338) panicked at tests/state_reader_test.rs:256:5:
assertion `left == right` failed: overview counted 2, the tab could draw 0. These must be one rule in one place; two rules kept in agreement by care is what produced a non-zero count above an empty list.
  left: 2
 right: 0

---- a_backlog_item_with_no_md_file_is_described_by_its_humanized_slug stdout ----

thread 'a_backlog_item_with_no_md_file_is_described_by_its_humanized_slug' (4107332) panicked at tests/state_reader_test.rs:271:5:
assertion `left == right` failed: overview counted 2, the tab could draw 0
  left: 0
 right: 2


failures:
    a_backlog_item_with_no_md_file_is_described_by_its_humanized_slug
    md_less_backlog_directories_reach_the_tab_instead_of_being_discarded
    the_overview_count_equals_the_number_of_rows_the_backlog_tab_can_draw

test result: FAILED. 4 passed; 3 failed; 0 ignored; 0 measured; 55 filtered out; finished in 0.00s
```

The red is the parser returning zero items — not a compile error, not a fixture
mistake — and the four pre-existing count tests stayed green throughout.

### Task 3 (the two new enforcement tests, against `src/state_reader/` checked back out at 3fa669e)

```text
running 10 tests
test a_backlog_name_the_strict_parser_rejects_is_counted_as_zero_and_parsed_as_zero ... FAILED
test a_backlog_directory_holding_a_md_heading_is_described_by_that_heading ... ok
test the_backlog_count_and_the_backlog_list_come_from_one_rule_across_every_fixture_shape ... FAILED

---- a_backlog_name_the_strict_parser_rejects_is_counted_as_zero_and_parsed_as_zero stdout ----
thread '...' (4182023) panicked at tests/state_reader_test.rs:414:5:
assertion `left == right` failed: overview counted 1, the tab could draw 0. A name from which no number and no slug can be extracted could never have been DISPLAYED, so it must not be COUNTED — the count follows the parser.
  left: (1, 0)
 right: (0, 0)

---- the_backlog_count_and_the_backlog_list_come_from_one_rule_across_every_fixture_shape stdout ----
thread '...' (4182029) panicked at tests/state_reader_test.rs:322:5:
assertion `left == right` failed: [md-less 999.N-slug directories] overview counted 2, the tab could draw 0. These two numbers must come from ONE rule, not from two rules that happen to agree.
  left: 2
 right: 0

test result: FAILED. 5 passed; 5 failed; 0 ignored; 0 measured; 55 filtered out; finished in 0.00s
```

The enrichment control passing against pre-fix source is correct and expected —
that direction already worked, and it is present as the no-regression half.

## Test Counts

| Run | passed | failed |
|---|---|---|
| Baseline, this tree, before any change | 2005 | 1 |
| Final, `--no-fail-fast` | 2010 | 2 |

Six tests were added, so 2011/1 is the arithmetically expected final. The
shortfall is one flaky failure, not a regression — see Deviations.

Per-gate results after Task 3:

| Gate | Result |
|---|---|
| `cargo test --test state_reader_test backlog` | 10 passed; 0 failed |
| `cargo test --lib state_reader::backlog` | 9 passed; 0 failed |
| `cargo test --test spawn_seam_guard no_production_item_follows_a_test_module_marker` | 1 passed |
| `cargo clippy -- -D warnings` | exit 0 |

All counts were read through `rtk proxy` with any pipe inside the proxied
command, per project memory.

## Deviations from Plan

### [Rule 1 - Bug] Two new enforcement tests were invisible to the project's own gate

- **Found during:** Task 3
- **Issue:** The sweep and the strict-parser guard were first named
  `the_count_and_the_list_come_from_one_rule_...` and
  `a_name_the_strict_parser_rejects_...`. Neither contains the substring
  `backlog`, so the plan's own gate —
  `cargo test --test state_reader_test backlog` — filtered them out and ran 8
  tests where 10 were expected. An enforcement test the gate does not run is not
  an enforcement test; it would have sat green-by-absence forever.
- **How it was caught:** the RED run reported `running 8 tests` when 10 were
  expected. The arithmetic, not the output's colour, is what surfaced it.
- **Fix:** renamed to
  `the_backlog_count_and_the_backlog_list_come_from_one_rule_across_every_fixture_shape`
  and `a_backlog_name_the_strict_parser_rejects_is_counted_as_zero_and_parsed_as_zero`.
- **Files modified:** `tests/state_reader_test.rs`
- **Commit:** f1cd6e9

### [Not a deviation — measured pre-existing] Two failures in the final full run

1. `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
   — the documented environmental failure: constants derived against
   `git version 2.43.0`, installed git is `2.53.0`. Present in the baseline.
2. `envelope_carrier_reach::the_t_19_116_replacement_takes_layer_2_as_well_and_that_is_measured_separately`
   — failed with `Os { code: 26, kind: ExecutableFileBusy, message: "Text file
   busy" }`, a race on replacing a test binary while cargo runs binaries in
   parallel. **Re-run alone: 39 passed; 0 failed.** It touches no `state_reader`
   code. Not a regression from this change.

### Out of scope, not fixed

`cargo clippy --all-targets -- -D warnings` reports 4 pre-existing errors in
test code unrelated to this item (`src/project_creator.rs:146` `cmp_owned`,
`bool_assert_comparison` in disk-status tests). The project's declared gate is
`cargo clippy -- -D warnings`, which exits 0. Left untouched per the scope
boundary.

## Inferred decisions (for audit)

The human operator was unavailable for this entire execution. All four decisions
below were carried forward from the plan, taken from measured evidence, and are
flagged here for later audit.

- **D-INF-01 — Fix the `.md` requirement, not the name grammar.** *Inferred
  decision, operator unavailable.* Branch 2 confirmed; branch 1 measured absent
  (0 of 12 names malformed). No `999-nodot` directory exists in any measured
  project, so no test was added that would loosen `parse_backlog_dir_name`.
  Honoured: the parser's grammar is byte-identical to before.
- **D-INF-02 — Reconcile toward the STRICT rule; the count follows the parser.**
  *Inferred decision, operator unavailable.* Accepted consequence: a
  hypothetical `999-nodot` directory stops being counted. That is correct — it
  could never have been displayed. Now pinned by
  `a_backlog_name_the_strict_parser_rejects_is_counted_as_zero_and_parsed_as_zero`.
- **D-INF-03 — Shared matcher in `backlog.rs`; `mod.rs` gets a body-only edit.**
  *Inferred decision, operator unavailable.* Honoured: no new item was added to
  `mod.rs` and `count_backlog_items` was not moved.
  `no_production_item_follows_a_test_module_marker` passes.
- **D-INF-04 — Touch no UI file.** *Inferred decision, operator unavailable.*
  Honoured: `files_modified` is exactly the three declared files. Sibling batch
  items 260916-vqz, 260916-vqx and 260916-vqy own `src/ui/screens/detail.rs` and
  `src/ui/screens/normal.rs`; this item merges beside them without overlap.

## Threat Surface

No new surface beyond the plan's `<threat_model>`. T-vr0-01's `mitigate`
disposition was applied: all three text fields (`dir_name`, `number`,
`description`) are still constructed through `Untrusted::from_untrusted_source`
at the single existing construction site, no `Display`/`AsRef<str>` shortcut was
added, and no raw `String` is handed to the struct. The change raises the number
of untrusted directory names reaching the renderer from zero to all of them,
which is why that wrapping is now more load-bearing, not less. No dependency was
added, removed or version-changed; `Cargo.toml` and `Cargo.lock` are untouched.

## Known Stubs

None. No placeholder values, no skipped tests, no unrun `<verify>` steps.

## Self-Check: PASSED

- `src/state_reader/backlog.rs` — FOUND (modified, `backlog_dirs` and
  `count_backlog_dirs` present above the `#[cfg(test)]` marker)
- `src/state_reader/mod.rs` — FOUND (modified, `count_backlog_items` delegates)
- `tests/state_reader_test.rs` — FOUND (modified, 6 new tests)
- Commit `02c7dee` — FOUND
- Commit `b566e50` — FOUND
- Commit `f1cd6e9` — FOUND
- `git rev-list --count 3fa669e..HEAD` — 3, matching `commits: 3`
