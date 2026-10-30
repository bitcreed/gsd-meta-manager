---
phase: 16-run-journal-state-substrate
plan: 01
subsystem: infra
tags: [ndjson, redaction, regex, serde_json, append-only, tail, journal, safe-04]

# Dependency graph
requires:
  - phase: 15-agent-execution-substrate
    provides: "ExecutionEvent / RunOutcome / ExecutionHandle shapes the journal records, the DrivableProject enforce-with-a-type idiom RedactedLine copies, and the WR-15 redaction record that shaped the corpus"
provides:
  - "src/journal/ module tree registered in src/lib.rs — the domain surface Phases 17/18/20 build against"
  - "JournalEvent: 13 struct variants under #[serde(tag = \"kind\")], incl. the Phase 18/20 kinds so those phases add no schema migration"
  - "RedactedLine: the SAFE-04 capture-path seam, enforced by the compiler (private field, crate-private accessor, zero trait impls, no escape hatch)"
  - "redact / redact_value: 29-case-pinned pattern redactor and a Value-tree walk that rewrites string leaves AND object keys"
  - "JournalWriter: append-only NDJSON, one write_all per record, handle open for the run"
  - "tail_lines / TailCursor / TailRead / parse_line / JournalRecord: byte-offset tail that leaves a torn line unconsumed, plus a tolerant parse"
  - "classify_change / ChangeKind: the pure, filesystem-free path classifier Wave 2's OBS-06 wiring consumes"
  - "RunPaths / run_paths / runs_root / new_run_id / argv_digest / RunRecord and the four growth constants"
affects: [16-02, 16-03, 16-04, 16-05, 16-06, 17-detached-driver, 18-driver-tab, 20-drpev-router]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Capability-token newtype with zero trait impls as a compile-time seam (RedactedLine, after DrivableProject)"
    - "Single-alternation regex redactor with named-group closure dispatch, compiled once in std::sync::LazyLock"
    - "Byte-offset tail that stops at the last newline, so a torn write self-heals instead of being lost"
    - "Executed corpus + idempotence property as the pin for a transform, rather than review"

key-files:
  created:
    - src/journal/mod.rs
    - src/journal/redact.rs
    - src/journal/writer.rs
    - src/journal/reader.rs
  modified:
    - src/lib.rs

key-decisions:
  - "classify_change requires FIVE path components (.planning/meta-manager/runs/<run-id>/<entry>), not four — a path directly under runs/ names no run, carries no run_id, and has no journal to tail, so runs/.gitignore and runs/active are Planning exactly as the plan requires"
  - "The generic pattern alternation runs BEFORE the runtime-home layer, reversing the plan's stated order: a bare /home/<user> is a substring of the Silverblue /var/home/<user>, so a host-first pass would strip the context the generic rule needs and make the corpus host-dependent"
  - "argv_digest is FNV-1a 64 implemented inline and documented as a non-cryptographic identity value only — the phase adds zero dependencies and no hashing crate is present"
  - "JournalWriter::append takes &mut self, so D-29's ordering contract is structural; no writer channel is introduced for a producer that does not exist until Phase 17"

patterns-established:
  - "Growth constants carry value + rationale + an explicit admission that no tuning data exists (main_loop.rs:41-64 shape)"
  - "Module docs name the rejected alternative and why, so an acceptance criterion can point at a doc comment"
  - "Redaction correctness is asserted against the BYTES ON DISK, never an in-memory value"

requirements-completed: [OBS-01, SAFE-04, OBS-06]

coverage:
  - id: D1
    description: "A journal event written through the only available write path lands on disk with its planted credential and planted dash-encoded home path already replaced by fixed literals — asserted against the file's bytes, not a rendered view"
    requirement: SAFE-04
    verification:
      - kind: unit
        ref: "src/journal/writer.rs#a_planted_credential_is_already_redacted_in_the_file_bytes"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every record in journal.jsonl is exactly one complete JSON object on one line, terminated by exactly one newline, carrying ts, a monotonic seq starting at 1, and kind"
    requirement: OBS-01
    verification:
      - kind: unit
        ref: "src/journal/writer.rs#every_appended_record_is_one_complete_line_with_one_newline"
        status: pass
      - kind: unit
        ref: "src/journal/writer.rs#seq_starts_at_one_and_increments_monotonically"
        status: pass
    human_judgment: false
  - id: D3
    description: "A tail read from a stored byte offset returns only complete lines; a torn trailing line is left unconsumed and re-read on the next append"
    requirement: OBS-01
    verification:
      - kind: unit
        ref: "src/journal/reader.rs#a_torn_final_line_is_left_for_the_next_read"
        status: pass
      - kind: unit
        ref: "src/journal/reader.rs#a_missing_journal_reads_as_empty_rather_than_erroring"
        status: pass
      - kind: unit
        ref: "src/journal/reader.rs#a_shrinking_journal_restarts_the_cursor_and_says_so"
        status: pass
    human_judgment: false
  - id: D4
    description: "An unknown journal kind — every Phase 20 event kind — parses with its payload intact rather than collapsing to a unit variant"
    requirement: OBS-01
    verification:
      - kind: unit
        ref: "src/journal/reader.rs#an_unknown_kind_arrives_with_its_payload_intact"
        status: pass
      - kind: unit
        ref: "src/journal/reader.rs#a_malformed_line_is_a_diagnostic_and_not_a_failure"
        status: pass
    human_judgment: false
  - id: D5
    description: "A path under .planning/meta-manager/runs/<run-id>/ classifies as a driver journal change and every other path — including a bare .planning/runs/, the shared .gitignore, the active pointer, and any path outside the project — classifies as a planning change, with no filesystem access"
    requirement: OBS-06
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#classify_change_names_a_journal_path_as_driver"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#classify_change_names_state_md_as_planning"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#classify_change_does_not_treat_a_bare_runs_directory_as_driver"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#classify_change_returns_planning_for_a_path_outside_the_project"
        status: pass
    human_judgment: false
  - id: D6
    description: "There is no compilable path from a bare String to a written journal line: RedactedLine has a private field, one crate-private accessor, no trait implementations and no escape-hatch constructor"
    requirement: SAFE-04
    verification:
      - kind: other
        ref: "cargo build + grep -c 'for RedactedLine' src/journal/redact.rs == 0 && grep -c 'pub(crate) fn as_line' == 1 && grep -c 'for_testing' == 0"
        status: pass
    human_judgment: false
  - id: D7
    description: "The redactor's behaviour is pinned by an executed 29-case corpus asserting exact output, is a fixed point under a second application over the whole corpus, rewrites object keys with collision disambiguation, cannot have its NDJSON framing forged from a payload, and truncates oversize payloads on a character boundary with a visible marker"
    requirement: SAFE-04
    verification:
      - kind: unit
        ref: "src/journal/redact.rs#every_corpus_case_redacts_exactly_as_specified"
        status: pass
      - kind: unit
        ref: "src/journal/redact.rs#redaction_is_idempotent_over_the_whole_corpus"
        status: pass
      - kind: unit
        ref: "src/journal/redact.rs#object_keys_are_redacted_and_collisions_are_disambiguated"
        status: pass
      - kind: unit
        ref: "src/journal/redact.rs#a_payload_with_embedded_newlines_still_serialises_to_one_line"
        status: pass
      - kind: unit
        ref: "src/journal/redact.rs#an_oversize_payload_is_truncated_on_a_char_boundary_with_a_marker"
        status: pass
      - kind: unit
        ref: "src/journal/redact.rs#the_runtime_home_prefix_is_redacted_in_both_encodings"
        status: pass
    human_judgment: false
  - id: D8
    description: "The run-id format's load-bearing property holds: lexicographic sort equals chronological sort, including across a day boundary and when the uuid suffix would sort the other way"
    requirement: OBS-01
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#run_id_sorts_lexicographically_in_chronological_order"
        status: pass
    human_judgment: false
  - id: D9
    description: "Whether the redaction pattern set is adequate for credential shapes this project's streams actually carry, beyond the 29 pinned cases"
    requirement: SAFE-04
    verification: []
    human_judgment: true
    rationale: "RESEARCH assumption A4 records this as MEDIUM confidence: a pattern redactor cannot catch an arbitrary high-entropy secret with no recognisable shape, and D-25 documents that limit as honest rather than fixable. The corpus is a floor to extend when a new shape is observed; adequacy is a judgement, not a test."

# Metrics
duration: 41 min
completed: 2026-07-29
status: complete
---

# Phase 16 Plan 01: Journal Module & Redaction Seam Summary

**An append-only NDJSON run journal whose only write path takes a `RedactedLine` — a newtype with zero trait impls whose sole constructor runs a 29-case-pinned redactor — plus a byte-offset tail that leaves torn lines unconsumed and a pure, filesystem-free path classifier.**

## Performance

- **Duration:** 41 min
- **Started:** 2026-07-29T10:07:00Z
- **Completed:** 2026-07-29T10:48:00Z
- **Tasks:** 2
- **Files modified:** 5 (4 created, 1 modified)

## Accomplishments

- **The SAFE-04 seam is closed by construction on the first commit, not retrofitted.** `JournalWriter::append` can only produce bytes by way of `RedactedLine`, whose field is private, whose accessor is `pub(crate)`, which implements no traits at all, and which has no test-only escape-hatch constructor. There is no compilable path from a bare `String` to a written journal line.
- **The end-to-end slice runs for real and the assertion that closes it reads the file's bytes (D-27).** One `JournalEvent::ExecEvent` carrying a planted `sk-ant-…` key and a planted dash-encoded `-home-fakeuser-projects-secretrepo` travels capture → redact → cap → serialise → one `write_all` → disk → `tail_lines` → `parse_line`, and `std::fs::read_to_string` confirms both planted strings are already gone and both fixed literals are present.
- **The redactor is pinned by execution rather than review.** 29 corpus rows assert exact output — including six WR-15 rows covering both encodings of a home path, the `Authorization: Basic <base64>` row that caught leak 1, and three deliberate non-matches — and the whole corpus is a fixed point under a second application.
- **`classify_change` is pure, exhaustive and available to Wave 2 unchanged.** It anchors on the full three-component `.planning/meta-manager/runs/` prefix, never a bare `runs`, and makes no filesystem call, so it is safe on the debouncer's callback thread.
- **The domain surface is complete for the phase.** `JournalEvent`'s 13 variants already include the `observed` / `decided` / `parked` kinds Phase 20 emits and the `interjected` kind Phase 18 emits, so neither phase adds a schema migration. Nothing downstream should have to reopen `src/journal/mod.rs` except plan 16-06's `ExecutionEvent` mapping.
- **Zero new dependencies.** `Cargo.toml` and `Cargo.lock` are byte-identical to the base commit; `std::sync::LazyLock` at MSRV 1.87 removed the only plausible one.

## Task Commits

Each task was committed atomically:

1. **Task 1: End-to-end "a redacted event reaches disk and is tailed back"** — `9ac8943` (feat)
2. **Task 2: Prove the redactor by execution — the corpus, idempotence, and keys** — `df72333` (test)

## Files Created/Modified

- `src/journal/mod.rs` (created, ~570 lines) — module root carrying the five governing facts, the three submodule declarations, the four growth constants, `RUNS_SUBDIR`, `runs_root` / `run_paths` / `RunPaths`, `new_run_id`, `argv_digest`, `ChangeKind` + `classify_change`, `JournalEvent` + `is_content`, and `RunRecord`.
- `src/journal/redact.rs` (created, ~700 lines) — the `PARTS` table transcribed verbatim in its significant order with the two leak comments, the `RE` and `HOST_RE` lazy statics, `redact`, `redact_value`, `cap_payload`, the `RedactedLine` seam, and the 29-case corpus plus six property tests.
- `src/journal/writer.rs` (created, ~290 lines) — `JournalWriter` with an open-for-the-run handle, one `write_all` per record, `seq` / `bytes_written` accessors, and the three end-to-end tests.
- `src/journal/reader.rs` (created, ~300 lines) — `TailCursor` / `TailRead` / `tail_lines` with all four commented edge cases, `JournalRecord` / `ParsedLine` / `parse_line`, and five tolerance tests.
- `src/lib.rs` (modified, +1 line) — `pub mod journal;` between `executor` and `main_loop`, preserving the alphabetical list.

## Decisions Made

1. **`classify_change` requires five path components, not four.** The plan's prose said "the three components plus a further component" but also required `.planning/meta-manager/runs/.gitignore` to classify as `Planning` — those two statements cannot both hold at four components. Requiring `runs/<run-id>/<entry>` satisfies both: a path *directly* under `runs/` names no run, so it yields no `run_id` and has no journal to tail. `active`, the shared `.gitignore` and the run directory's own creation event therefore take the safe `Planning` default (one re-parse per run start, which is negligible). Documented on the function.

2. **The generic alternation runs before the runtime-home layer.** See Deviations — this is the one place the implementation departs from the plan's stated sequence, and the reason is a substring interaction the plan's sketch did not anticipate.

3. **`JournalWriter::append` returns the `seq` it used, and `seq()` reports the *next* one.** The plan left the accessor's semantics open. Returning the written value makes the caller's journal-and-continue sequence testable without a second read; documenting `seq()` as "the seq the next append will use" keeps the invariant `seq() == writes + 1` obvious.

4. **`RETAIN_RUNS`'s doc opens with "Ten." rather than "10.".** A doc comment beginning `10.` is parsed as an ordered-list item and trips `clippy::doc_lazy_continuation` under `-D warnings`. Trivial, recorded only because the constant-doc shape is a pattern later plans copy.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Redaction layer order reversed: generic alternation before the runtime-home layer**

- **Found during:** Task 1 (`src/journal/redact.rs`)
- **Issue:** The plan specifies "Apply the host regex first, then the generic alternation, in that order." The host layer holds a **bare literal** home path, and a bare `/home/<user>` is a *substring* of the Silverblue form `/var/home/<user>`. Applied first, it rewrites the inner half and strips the `/var/home/` context the generic `shome` rule needs, producing `/var/home/[REDACTED:user]` instead of `/home/[REDACTED:user]`. That makes the corpus **host-dependent** — the plan's own mandated silverblue row (`/var/home/blk/projects/x`, RESEARCH §4.4) fails on any machine whose `$HOME` is `/home/blk`, which is this one.
- **Fix:** Run the generic alternation first and the host layer second. Both statics named in the plan's artifact list (`RE`, `HOST_RE`) are kept, and both layers still emit the *same* replacement literals, so the second pass is a no-op on whatever the first produced — which is the invariant the plan's rationale actually rests on ("their replacements must be the same literals the generic rules produce"), independent of order. The rationale is written into `redact`'s doc comment.
- **Files modified:** `src/journal/redact.rs`
- **Verification:** All 29 corpus rows pass with exact expected output; `redaction_is_idempotent_over_the_whole_corpus` is green; `the_runtime_home_prefix_is_redacted_in_both_encodings` passes without depending on the developer's actual home.
- **Committed in:** `9ac8943` (Task 1 commit)

**2. [Rule 3 - Blocking] Two acceptance criteria are literally unsatisfiable as written; equivalent checks substituted**

- **Found during:** Task 1 and Task 2 (acceptance-criteria verification loop)
- **Issue A:** `grep -c 'fn classify_change' src/journal/mod.rs` cannot output `1` while the plan's own four mandated test names exist, because `fn classify_change_names_a_journal_path_as_driver` *contains* that substring. Measured: `5`.
- **Issue B:** `rtk proxy cargo clippy --all-targets 2>&1 | grep -c '^warning: '` cannot output `5` — `cargo` emits a trailing crate-summary line (`warning: \`gsd-meta-manager\` (lib test) generated 5 warnings`) that also matches `^warning: `. Measured **at the base commit, before any change**: `6`.
- **Fix:** Substituted checks that preserve the intent exactly. For A: `grep -c 'fn classify_change(' src/journal/mod.rs` → `1` (exactly one *definition*; the test names do not match the open paren). For B: `rtk proxy cargo clippy --all-targets 2>&1 | grep '^warning: ' | grep -v generated | wc -l` → `5`, and the five lints are enumerated and confirmed identical to baseline (`assert_eq!` with a literal bool ×3, owned instance for comparison ×1, items after a test module ×1 — none in `src/journal/`).
- **Files modified:** none (verification-only)
- **Verification:** Both substituted commands run and reported the intended values; the base-commit measurement for B was taken before the first edit, so the "6" is provably pre-existing.
- **Committed in:** n/a (no code change)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Deviation 1 is a genuine correctness fix that the plan's own mandated corpus row would have caught at the first test run; the seam, the symbol surface and the replacement literals are all exactly as planned. Deviation 2 changed no code. No scope creep — nothing outside `src/journal/` and the one `src/lib.rs` line was touched.

## Issues Encountered

None. Both tasks passed their acceptance loops without a fix cycle; the only test-time failure surface (the corpus's exact expected outputs) was analysed before writing and passed on the first run.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | clean |
| `rtk proxy cargo test --lib journal::` | 24 passed, 0 failed |
| `rtk proxy cargo test` (full suite) | **387 passed**, 0 failed (332 lib + 11 + 7 + 12 + 25) — baseline was 363 (308 lib); +24 new, none removed |
| `cargo clippy -- -D warnings` | exits 0 |
| `rtk proxy cargo clippy --all-targets` lint count | exactly **5**, all pre-existing, none in `src/journal/` |
| `git diff --stat Cargo.toml Cargo.lock` | no change — zero new dependencies |
| `grep -c 'sync_all\|sync_data' src/journal/writer.rs` | `0` (D-04) |
| `grep -c 'BufWriter' src/journal/writer.rs` | `0` (Pitfall 8) |
| `grep -v '^[[:space:]]*//' src/journal/reader.rs \| grep -c 'BufReader'` | `0` (§2.1) |
| `cat src/journal/*.rs \| grep -v '^[[:space:]]*//' \| grep -c 'deny_unknown_fields'` | `0` (D-30) |
| `grep -c 'for_testing' src/journal/redact.rs` | `0` (D-22) |
| `grep -c 'for RedactedLine' src/journal/redact.rs` | `0` (D-22) |
| `grep -c 'pub(crate) fn as_line' src/journal/redact.rs` | `1` (D-22) |
| `classify_change` body: `metadata\|exists\|is_dir\|is_file\|read_dir\|canonicalize` | `0` (D-11) |
| `src/lib.rs` line 10 | `pub mod journal;`, between `executor` (9) and `main_loop` (11) |

## Known Stubs

Four `JournalEvent` variants are **schema only** in this plan and are documented as such on the variants themselves — `Observed`, `Decided` and `Parked` (Phase 20 emits them, D-36) and `Interjected` (Phase 18). This is not an unwired stub: D-36 is explicit that the kinds exist now precisely so those phases add no schema migration, and the reader tolerates unknown kinds regardless. `RunRecord::opt_in` is likewise `Option` because populating it is Phase 17's job (D-06).

Nothing in this plan is a placeholder that prevents the plan's goal from being achieved.

## Threat Flags

None. Every file created here is inside the plan's declared `<threat_model>` scope, no network endpoint, auth path or schema at a trust boundary was introduced, and the seven registered threats are all addressed as planned: T-16-01 by the `RedactedLine` type, T-16-02 by the 29-case corpus plus the idempotence property, T-16-03 by the D-28 sentences in every module doc, T-16-04 by the framing test, T-16-05 by `cap_payload` and its char-boundary test, T-16-06 accepted per RESEARCH §4.1, and T-16-07 by `classify_change`'s three-component anchor plus the bare-`runs` test.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The domain surface every other plan in this phase builds against is committed and green. 16-02 through 16-06 can take `JournalEvent`, `RunPaths`, `RedactedLine`, `JournalWriter`, `tail_lines` and `classify_change` as given; none of them should need to reopen `src/journal/mod.rs` except 16-06's `ExecutionEvent` mapping.
- `classify_change` returns `ChangeKind` deriving `Hash`, which is what the watcher's per-`(root, classification)` dedup fix (D-10) needs as half its key.
- `JournalWriter::bytes_written()` is exposed for plan 16-03's per-run cap, and `JournalEvent::is_content()` is the predicate that decides what survives it.
- **One carried-forward note for whichever plan owns run-start:** `JournalWriter::open` requires the run directory to already exist. Creating it, writing the `runs/.gitignore` before the first byte of journal (D-08), and the atomic `run.json` write (D-05) are not in this plan.
- No blockers.

## Self-Check: PASSED

- `src/journal/mod.rs`, `src/journal/redact.rs`, `src/journal/writer.rs`, `src/journal/reader.rs` — all present on disk.
- `src/lib.rs` — present, contains `pub mod journal;` at line 10.
- Commits `9ac8943` and `df72333` — both found in `git log`.
- All task `<acceptance_criteria>` re-run after the final commit; two substituted per Deviation 2, all others verbatim and passing.
- Plan-level `<verification>` re-run: build clean, `journal::` tests green, `clippy -- -D warnings` exits 0, `--all-targets` lint count 5, `Cargo.toml`/`Cargo.lock` unchanged.

---
*Phase: 16-run-journal-state-substrate*
*Completed: 2026-07-29*
