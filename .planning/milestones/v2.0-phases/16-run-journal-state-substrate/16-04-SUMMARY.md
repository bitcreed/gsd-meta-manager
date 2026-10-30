---
phase: 16-run-journal-state-substrate
plan: 04
subsystem: infra
tags: [journal, tail, ndjson, serde, forward-compatibility, diagnostics, tolerance, obs-01]

# Dependency graph
requires:
  - phase: 16-run-journal-state-substrate
    plan: "01"
    provides: "tail_lines / TailCursor / TailRead / JournalRecord / ParsedLine / parse_line and MAX_TAIL_BYTES — this plan extends them, it does not create them"
provides:
  - "seq_gaps(records) -> Vec<(u64, u64)>: one (last seen, next seen) pair per seq discontinuity"
  - "read_all(path) -> io::Result<(Vec<JournalRecord>, ReadDiagnostics)>: whole-journal read that no gap and no unparseable line can abort"
  - "ReadDiagnostics { unparseable, gaps, last_seq }: content-free diagnostics a caller may log wholesale under D-28"
  - "The tolerance contract as a tested property: six tail edge cases pinned against real files, forward compatibility asserted on field VALUES"
affects: [16-05, 17-detached-driver, 18-driver-tab, 20-drpev-router]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A whole-file read implemented as a loop over the incremental tail, so both paths share one torn-line and one oversize-line rule rather than two subtly different ones"
    - "Forward-compatibility tests assert the VALUES of carried payload fields, because the shape that destroys the payload does not error either"
    - "Fixture lines built by hand rather than through the build's own writer, so a forward-compat test cannot be limited to shapes this build already understands"

key-files:
  created: []
  modified:
    - src/journal/reader.rs

key-decisions:
  - "ReadDiagnostics keeps exactly the three fields the plan declared (unparseable, gaps, last_seq) and does NOT carry restarted/skipped_oversize: those are per-read tail facts that TailRead already owns, and read_all starts from offset zero where restarted is unreachable by construction"
  - "read_all loops over tail_lines rather than reading the file whole — MAX_TAIL_BYTES (4 MiB) is below MAX_RUN_JOURNAL_BYTES (64 MiB), so a single tail call cannot see a large journal, and reusing the tail keeps one torn-line rule instead of two"
  - "seq_gaps reports ANY step other than exactly +1, so a repeated or backwards seq is surfaced too — both are equally impossible under the writer's monotonic counter and equally worth seeing if they appear"
  - "The three overlapping Wave 1 tail tests were RENAMED to the plan's mandated names and one was SPLIT in two, rather than adding near-duplicates beside them"

# Metrics
duration: 23 min
completed: 2026-07-29
status: complete
---

# Phase 16 Plan 04: Reader Tolerance & Forward Compatibility Summary

**The reader's three D-30 tolerances stop being an intention and become tested properties: a torn trailing line is asserted absent from one read and complete in the next, an unknown `kind` is asserted to carry the *values* of every payload field, and a `seq` gap is a reported diagnostic that returns every record it read.**

## Performance

- **Duration:** 23 min
- **Started:** 2026-07-29T11:04:00Z
- **Completed:** 2026-07-29T11:27:00Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- **The load-bearing forward-compatibility test asserts survival, not silence.** RESEARCH Pitfall 5's warning sign is a test that checks an unknown kind "does not error" — because the shape that *destroys* the payload does not error either. `an_unknown_kind_keeps_every_one_of_its_fields` asserts `rest["reason"]`, `rest["needs"]` **and** `rest.len() == 2` on a hand-written Phase 20 `parked` record, so an internally-tagged enum with a catch-all unit variant could not pass it. Wave 1 had already chosen the surviving shape (`kind: String` + `#[serde(flatten)]`, RESEARCH §9 Option B); this plan proves it rather than trusting it.
- **The torn-line test is written to fail against the naive implementation.** It asserts the cursor after the torn read is *byte-identical* to the cursor before it, that the completed line arrives exactly once from the second read, and that a third read does not re-deliver it. `BufReader::lines()` — which treats EOF as a line terminator and therefore yields the torn line as complete — fails the first two of those.
- **All five tail edge cases from RESEARCH §2.3 now run against real files.** Every one operates on a path inside a `tempfile::tempdir()`; none constructs a reader over an in-memory buffer, because the properties under test are file offsets, file length and end-of-file semantics, and a fake would test the fake. The expected values transcribe RESEARCH's executed table rather than guessing.
- **The oversize branch is pinned by a strict-increase assertion.** `a_line_that_cannot_complete_inside_the_bound_is_stepped_over` writes a full `MAX_TAIL_BYTES` region with no newline anywhere and asserts `cursor > start` — so a regression that leaves the cursor parked (which would make every later event in the run invisible *forever*, not just the offending region) fails the test rather than passing it quietly.
- **A gap and an unparseable line are both diagnostics that return every surviving record.** Over RESEARCH §9.2's verified fixture (1, 2, 4, a non-JSON line, 5) `read_all` returns `gaps=[(2,4)] unparseable=1 last_seq=5` and all four well-formed records — the exact triple RESEARCH measured end to end.
- **Both tail diagnostics are documented at the field they belong to.** `restarted` records that under D-31/D-32 it can only mean an invariant broke and deserves a `tracing::warn!` at the call site; `skipped_oversize` records that with the per-event cap in place the branch is pure defence against a corrupted file, and that the cursor advancing is the entire point.
- **Zero new dependencies, zero new clippy lints, and one file touched.** `git diff --stat` against the base shows `src/journal/reader.rs` only.

## Task Commits

Each task was committed atomically:

1. **Task 1: The tail's five edge cases, pinned against real files** — `75fb6b0` (test)
2. **Task 2: Forward compatibility — an unknown kind arrives with its fields intact** — `141f0aa` (feat)

## Files Created/Modified

- `src/journal/reader.rs` (modified, +351/−29 across both commits) — added `ReadDiagnostics`, `seq_gaps`, `read_all`; expanded the `restarted` and `skipped_oversize` field docs; added the module-doc paragraph on why rotation is detected by length and never by inode identity; rewrote the test module's tail section against real files and added the forward-compatibility and diagnostics tests.

### Symbols added

| Symbol | Shape |
|---|---|
| `ReadDiagnostics` | `{ unparseable: usize, gaps: Vec<(u64, u64)>, last_seq: Option<u64> }` |
| `seq_gaps` | `pub fn seq_gaps(records: &[JournalRecord]) -> Vec<(u64, u64)>` |
| `read_all` | `pub fn read_all(path: &Path) -> std::io::Result<(Vec<JournalRecord>, ReadDiagnostics)>` |

### Tests

All ten plan-mandated names are present. Three more were added where a boundary was cheap to pin, and one Wave 1 test was kept.

| Test | Origin |
|---|---|
| `a_missing_journal_reads_as_empty_without_an_error` | renamed from Wave 1 |
| `two_whole_lines_are_returned_and_the_cursor_lands_past_the_last_newline` | split out of Wave 1's combined test |
| `a_partial_trailing_line_is_not_consumed_and_completes_on_the_next_read` | split out of Wave 1's combined test, assertions strengthened |
| `a_file_that_shrank_resets_the_cursor_and_reports_it` | renamed from Wave 1, assertions strengthened |
| `a_line_that_cannot_complete_inside_the_bound_is_stepped_over` | new |
| `a_non_utf8_fragment_is_skipped_rather_than_mangled` | new |
| `an_unknown_kind_keeps_every_one_of_its_fields` | renamed from Wave 1, now asserts `rest.len()` too |
| `a_record_with_an_unexpected_extra_field_still_parses` | new |
| `a_sequence_gap_is_reported_and_the_read_still_returns_every_record` | new |
| `an_unparseable_line_is_counted_and_the_records_around_it_survive` | new |
| `a_journal_that_does_not_exist_reads_as_no_records_and_no_diagnostics` | new (beyond plan) |
| `an_unbroken_run_of_sequence_numbers_reports_no_gaps` | new (beyond plan) |
| `a_malformed_line_is_a_diagnostic_and_not_a_failure` | kept from Wave 1 |

## Decisions Made

1. **`ReadDiagnostics` carries exactly the three declared fields and not the tail's two flags.** Adding `restarted` / `skipped_oversize` was tempting, and wrong for `read_all` specifically: `read_all` starts at offset zero, so `restarted` is unreachable there by construction, and both flags are per-read facts `TailRead` already owns for the incremental path that actually needs them. The plan declared a three-field struct; widening the artifact contract to carry a field that can never be `true` would be noise in every caller's log line.

2. **`read_all` loops over `tail_lines` rather than reading the file whole.** `MAX_TAIL_BYTES` is 4 MiB and `MAX_RUN_JOURNAL_BYTES` is 64 MiB, so one tail call demonstrably cannot see a full-sized journal — a single-call implementation would silently truncate a large read. Looping also means the whole-file path inherits the torn-line and oversize-line rules instead of growing a second, subtly different copy of them. Termination is bounded by the cursor strictly advancing: a read that does not move the cursor ends the loop, which is simultaneously the end-of-file case and the torn-trailing-line case.

3. **`seq_gaps` reports any step other than exactly `+1`.** A repeated or backwards `seq` is as impossible as a missing one under the writer's monotonic counter, so both deserve the same surfacing. Documented on the function.

4. **The overlapping Wave 1 tests were renamed and split, not duplicated.** Wave 1's `a_torn_final_line_is_left_for_the_next_read` covered two of the plan's mandated cases in one body. Keeping it beside two new tests asserting the same facts would have left three tests that fail together and say the same thing. It was split into the two mandated names and both halves strengthened.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The TDD RED gate cannot fail first for a characterization task, and the plan's own action text says so**

- **Found during:** Task 1
- **Issue:** Both tasks are marked `tdd="true"`, whose gate expects a failing test before implementation. Task 1's subject is `tail_lines`, which plan 16-01 already shipped complete; its tests therefore passed on their first run. Under a literal RED-first reading that is a stop condition.
- **Fix:** Followed the plan's own `<action>`, which resolves it explicitly: *"Write the tests first, then adjust `tail_lines` only if a test exposes a defect — RESEARCH §2.3 already recorded this exact behaviour from an executed run, so the expected outcome of each test is known before it is written and the tests are transcribing a measurement rather than guessing."* No test passed *unexpectedly*: every expected value was read off RESEARCH §2.3's executed table before the test was written, and each was checked against the shipped code path by inspection first. Task 1 is therefore committed as `test(...)` and no defect was found. Task 2 adds genuinely new code and is committed as `feat(...)`.
- **Files modified:** none beyond the planned ones
- **Verification:** `rtk proxy cargo test --lib journal::reader` — 13 passed, 0 failed
- **Committed in:** `75fb6b0`

**2. [Rule 2 - Missing critical functionality] Removed the parenthetical naming of the strict-unknown-field attribute from this file's module doc**

- **Found during:** Task 2
- **Issue:** Wave 1's module doc read *"Serde's strict unknown-field rejection attribute (`deny_unknown_fields`) is never opted into…"*. The plan's Task 2 action is explicit: *"Do not name the attribute in prose; the guard is a grep and naming it in a comment would make that grep self-invalidating."* The name in a comment is what turns a mechanical guard into a guard that matches itself.
- **Fix:** Rewrote the sentence in the `stream_json.rs:6-8` shape without the parenthetical, and added the reason inline so a future editor does not helpfully put it back. `src/journal/mod.rs` still carries the name in its own doc, but that file belongs to another plan in this wave and editing it would create a merge conflict for no correctness gain — the plan's acceptance grep filters comment lines, so both forms pass it today, and this file (the one the guard is about) is now robust even to an unfiltered grep.
- **Files modified:** `src/journal/reader.rs`
- **Verification:** `grep -v '^[[:space:]]*//' src/journal/reader.rs | grep -c 'deny_unknown_fields'` → `0`; unfiltered `grep -c 'deny_unknown_fields' src/journal/reader.rs` → `0`
- **Committed in:** `75fb6b0`

**3. [Rule 3 - Blocking] Three tests beyond the plan's ten**

- **Found during:** Tasks 1 and 2
- **Issue:** Not a defect — a coverage judgement made under the autonomy contract. `seq_gaps`'s empty and single-element inputs are exactly where a `windows(2)` implementation goes wrong, and `read_all` on a missing path is the one input every caller will hit before a run starts.
- **Fix:** Added `an_unbroken_run_of_sequence_numbers_reports_no_gaps` (which also covers the one-record and zero-record cases) and `a_journal_that_does_not_exist_reads_as_no_records_and_no_diagnostics`. Kept Wave 1's `a_malformed_line_is_a_diagnostic_and_not_a_failure`, which pins `parse_line` directly rather than through `read_all`.
- **Files modified:** `src/journal/reader.rs`
- **Verification:** included in the 13 passing `journal::reader` tests
- **Committed in:** `141f0aa`

---

**Total deviations:** 3 auto-fixed (0 bugs, 1 doc-hardening, 2 process/coverage)
**Impact on plan:** No behavioural change to `tail_lines` was needed — RESEARCH §2.3's table matched the shipped implementation on every one of the five edge cases, which is the outcome the plan predicted. The declared symbol surface (`seq_gaps`, `read_all`, `ReadDiagnostics`) is exactly as planned. No scope creep: `git diff --stat` against the base commit touches one file.

## Issues Encountered

One trivial one: `rustfmt` wanted the `record_line` fixture helper's `format!` broken across lines. Fixed before the Task 2 commit. Note for anyone repeating the check — `cargo fmt --check -- <path>` ignores the path argument and reports pre-existing drift in `src/app.rs`; `rustfmt --check --edition 2021 src/journal/reader.rs` is the per-file form and it is clean.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | clean |
| `rtk proxy cargo test --lib journal::reader` | **13 passed**, 0 failed |
| `rtk proxy cargo test` (full suite) | **399 passed**, 0 failed (344 lib + 11 + 7 + 12 + 25) — baseline 391; +8 net, none removed |
| `cargo clippy -- -D warnings` | exits 0 |
| `rtk proxy cargo clippy --all-targets` lint count | exactly **5**, unchanged from base, none in `src/journal/` |
| `rustfmt --check --edition 2021 src/journal/reader.rs` | clean |
| `git diff --stat` vs base | `src/journal/reader.rs` only |
| `grep -c 'pub fn seq_gaps' src/journal/reader.rs` | `1` |
| `grep -v '^[[:space:]]*//' src/journal/reader.rs \| grep -c 'from_utf8_lossy'` | `0` |
| `grep -v '^[[:space:]]*//' src/journal/reader.rs \| grep -c 'BufReader'` | `0` (RESEARCH §2.1) |
| `grep -v '^[[:space:]]*//' src/journal/reader.rs \| grep -c 'serde(other)'` | `0` (RESEARCH §9) |
| `cat src/journal/*.rs \| grep -v '^[[:space:]]*//' \| grep -c 'deny_unknown_fields'` | `0` (D-30) |
| Every tail test on a `tempfile::tempdir()` path | yes — no in-memory reader anywhere in the file |
| `Cargo.toml` / `Cargo.lock` | unchanged — zero new dependencies |

### Must-have truths

| Truth | Pinned by |
|---|---|
| A torn final line yields every complete record and leaves the torn bytes unconsumed (D-13, D-30) | `a_partial_trailing_line_is_not_consumed_and_completes_on_the_next_read` — asserts cursor equality across the torn read, arrival exactly once, and no re-delivery |
| An unknown kind is carried with all its fields intact (D-30, RESEARCH §9) | `an_unknown_kind_keeps_every_one_of_its_fields` — asserts two field values and the payload length |
| A seq gap is a diagnostic and never fails a read (D-03, D-30) | `a_sequence_gap_is_reported_and_the_read_still_returns_every_record` — `gaps == [(2,4)]` with all four records returned |
| A line that cannot complete inside the bound is stepped over (D-13) | `a_line_that_cannot_complete_inside_the_bound_is_stepped_over` — asserts the cursor strictly increased |
| A file that shrank resets and says so (RESEARCH §2.4) | `a_file_that_shrank_resets_the_cursor_and_reports_it`, plus the `restarted` field doc naming it as broken-invariant evidence |

## Known Stubs

None. Every symbol this plan declared is implemented and exercised by a test.

`read_all` and `seq_gaps` have no in-tree caller yet — plan 16-05 owns the watcher/app wiring and 17/18 own the driver surfaces — but they are `pub` items of a library crate with complete implementations and direct test coverage, not placeholders. The `tracing::warn!` on `restarted` that the field doc prescribes is likewise a call-site instruction for 16-05, deliberately not issued from inside `tail_lines`: a pure read function that logs on behalf of unknown callers is the wrong place for a policy decision, and D-28's content rule is easier to hold at one wiring site than at every read.

## Threat Flags

None. The five threats this plan registered are all addressed as planned: T-16-18 by the strict-increase assertion on the oversize branch, T-16-19 by `read_all`'s count-and-skip contract, T-16-20 by the field-value assertions on an unknown kind, T-16-21 by the torn-line test against a real file with real offsets, and T-16-22 by `ReadDiagnostics` carrying only counts and sequence numbers. No new network endpoint, auth path, file-access pattern or trust-boundary schema was introduced; the file's only inputs are a path and the bytes at it, both of which were already in scope.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `read_all` is the whole-file entry point plan 16-05 and Phase 18's driver tab need for "show me this run's journal"; `tail_lines` remains the incremental path for a live run.
- **One carried-forward instruction for whichever plan owns the tail call site (16-05):** `TailRead::restarted` and `TailRead::skipped_oversize` must be surfaced, not dropped on the floor. The field docs prescribe a `tracing::warn!` plus a `JournalEvent::Diagnostic`; `tail_lines` deliberately does not log them itself.
- Phase 20 needs no reader change to emit new event kinds. That is now asserted rather than assumed.
- No blockers.

## Self-Check: PASSED

- `src/journal/reader.rs` — present on disk, contains `pub fn seq_gaps`, `pub fn read_all` and `pub struct ReadDiagnostics`.
- `.planning/phases/16-run-journal-state-substrate/16-04-SUMMARY.md` — present on disk.
- Commits `75fb6b0` and `141f0aa` — both found in `git log`.
- All task `<acceptance_criteria>` re-run after the final code commit; every one passing, none substituted.
- Plan-level `<verification>` re-run: build clean, all ten mandated test names present and green, `clippy -- -D warnings` exits 0, `--all-targets` lint count still 5, `git diff --stat` touches only `src/journal/reader.rs`.
- `STATE.md` and `ROADMAP.md` deliberately untouched — the orchestrator owns those writes.

---
*Phase: 16-run-journal-state-substrate*
*Completed: 2026-07-29*
