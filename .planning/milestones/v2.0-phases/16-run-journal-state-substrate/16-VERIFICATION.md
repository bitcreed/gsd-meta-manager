---
phase: 16-run-journal-state-substrate
verified: 2026-07-29T00:00:00Z
status: passed
score: 3/3 roadmap success criteria verified
behavior_unverified: 0
overrides_applied: 0
deferred: []
---

# Phase 16: Run Journal & State Substrate Verification Report

**Phase Goal:** Every run leaves a durable, redacted, cheap-to-read record on disk that outlives
the processes that wrote it
**Verified:** 2026-07-29
**Status:** passed
**Re-verification:** No — initial verification of this phase

## Verification method

Not a SUMMARY-trust pass. All three success criteria were re-derived directly from source and by
independently re-running the specific tests that evidence them (not merely reading the numbers the
SUMMARYs report). The three named vacuous-check hazards were checked explicitly — see the dedicated
section below — before any criterion was marked verified.

## Goal Achievement

### Observable Truths (Roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | After a run is killed mid-flight or its host process dies, the on-disk journal still shows every step it completed and the last event before death | ✓ VERIFIED | `tests/journal_crash.rs` re-execs the test binary as a child running the **real** `JournalWriter` via the `pub` (not `#[cfg(test)]`) `crash_test_writer_loop` in `src/journal/writer.rs:645-664`, so the child cannot be a shell stand-in. I independently re-ran `cargo test --test journal_crash`: 3 passed in 0.08s. `a_sigkilled_writer_leaves_every_flushed_line_readable` asserts, against the real SIGKILL aftermath: (a) `status.code() == None` — died by signal, not by exit; (b) >100 complete lines survived (measured ~2150 in RESEARCH's own session); (c) **zero** `seq_gaps()` among surviving records, plus `seq[0]==1` and `seq[last]==count`, proving no interior event is missing, not merely that many lines exist; (d) at most one torn line, and if present it is the last one; (e) every complete line reparses through `parse_line` into a `Record`. The writer itself (`src/journal/writer.rs:111-130`) writes one `write_all` per record with no per-event `fsync` — confirmed by `grep -c 'sync_all\|sync_data' src/journal/writer.rs` = 0 — which is the documented, measured basis (RESEARCH §3: 8/8 SIGKILL runs, zero torn lines, zero seq gaps) for why this durability model works without a per-event syscall. |
| 2 | A run appending events every few seconds for an hour leaves TUI navigation as responsive as when idle — journal writes do not trigger a full project re-parse | ✓ VERIFIED | `src/app.rs`'s `Action::FileChanged` arm (lines 457-509) classifies **before** the 500ms dedup check (D-14) via `journal::classify_change`, routing a `DriverJournal` change to `schedule_journal_tail` (a byte-offset tail, `src/app.rs:280-351`) and a `Planning` change to `schedule_reparse`, the **sole** call site of `parse_project_state` (`src/app.rs:226-260`, doc comment states this claim explicitly and the OBS-06 test is what makes it true rather than aspirational). I independently re-ran `cargo test --lib app::` and `cargo test --lib watcher::`: all 19 tests pass, including `journal_appends_never_trigger_a_full_reparse` (500 journal-path `FileChanged` events, `reparse_dispatches` counter unchanged) **and its required control arm** `a_planning_write_still_triggers_a_reparse` (one planning-path event, counter advances by exactly 1 — not "at least 1"). I read both test bodies directly: the control arm is a real, independent assertion, not a decorative no-op — it uses the same `obs_app` harness and would fail if the handler dropped `FileChanged` outright, which is exactly the failure a lone zero-arm test cannot catch (this file's own comment names that exact risk at `src/app.rs:959-962`). `the_driver_route_leaves_the_refresh_dedup_map_untouched` additionally confirms D-14: 10 journal-path events leave `last_refresh` empty, while a subsequent planning-path event populates it — proving the driver route genuinely bypasses the shared dedup map rather than merely happening not to hit it in this run. |
| 3 | A credential or token that appears in a run's output is already redacted in the journal file on disk, not merely in the rendered view | ✓ VERIFIED | Three independent layers, all re-run rather than trusted: (a) `src/journal/redact.rs`'s 29-row corpus (`every_corpus_case_redacts_exactly_as_specified`, exact-output assertions, includes both the slash-form and the **dash-encoded** `-home-<user>-<repo>` WR-15 shape as separate named rows) plus `redaction_is_idempotent_over_the_whole_corpus`; (b) `src/journal/writer.rs`'s `a_planted_credential_is_already_redacted_in_the_file_bytes`, which asserts via `std::fs::read_to_string` on the file — not an in-memory `Value`; (c) `tests/journal_crash.rs`'s `a_sigkilled_writers_surviving_bytes_are_already_redacted`, which reads the bytes that survive a real SIGKILL and asserts **both** `CRASH_TEST_PLANTED_KEY` (an `sk-ant-…` shape) and `CRASH_TEST_PLANTED_HOME` (`-home-fakeuser-projects-secretrepo`, the dash-encoded form) are absent while their redacted literals are present — the dash form gets its own explicit assertion, not folded into a general path check, exactly matching the requirement that a slash-form-only check is insufficient. The seam is enforced by a type, not review: `RedactedLine` (`src/journal/redact.rs:319-340`) has a private field, a `pub(crate)` accessor, and implements no traits — confirmed `grep -c 'for RedactedLine' src/journal/redact.rs` = 0, `grep -c 'for_testing' src/journal/redact.rs` = 0 — so there is no compilable path from a bare `String` to a written line. |

**Score:** 3/3 roadmap success criteria verified as met by the shipped code.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/journal/mod.rs` | `JournalEvent` schema, `RunPaths`/`run_paths`, `classify_change`/`ChangeKind`, `JournalRun` lifecycle, `from_exec_event` mapping, growth constants | ✓ VERIFIED | Read in full (1339 lines). All symbols present and match the plan's artifact list; `JournalRun::start`/`finish` enforce the exactly-twice `run.json` write with a `debug_assert_eq!`. |
| `src/journal/redact.rs` | `PARTS` table, `redact`/`redact_value`, `RedactedLine` seam | ✓ VERIFIED | Read in full (720 lines). 29-row corpus, idempotence property, key-collision disambiguation, char-boundary-safe truncation, host-layer test that doesn't depend on the actual dev machine's home. |
| `src/journal/writer.rs` | `JournalWriter`, `create_run_dir`, `write_run_record`, `prune_runs`, `crash_test_writer_loop` | ✓ VERIFIED | Read in full (1213 lines). One `write_all` per record confirmed (`grep -c BufWriter` = 0). Atomic `run.json` write via `NamedTempFile` + `persist`, mode fixed to 0644 post-persist. Retention never prunes an active or unended run. |
| `src/journal/reader.rs` | `tail_lines`, `TailCursor`/`TailRead`, `parse_line`, `seq_gaps`, `read_all` | ✓ VERIFIED | Read in full (620 lines). Byte-splitting tail (not `BufReader::lines()`, confirmed 0 hits outside comments) leaves a torn trailing line unconsumed; non-UTF-8 fragments skipped rather than lossily mangled; unknown `kind` carries its full payload via `#[serde(flatten)]`. |
| `src/action.rs` | `Action::FileChanged { changed_path }`, `Action::DriverJournalAppended` | ✓ VERIFIED | Both fields present with rationale docs (D-09, D-12); `DriverJournalAppended` is unboxed (~80 bytes), correctly not triggering `clippy::large_enum_variant`. |
| `src/watcher.rs` | Per-`(root, ChangeKind)` dedup fold | ✓ VERIFIED | `batch_actions` extracted as a free function, dedup key widened from `PathBuf` to `(PathBuf, ChangeKind)` (D-10); three regression tests including the journal-first-then-planning ordering case. |
| `src/app.rs` | `FileChanged` classification fork, `schedule_journal_tail`, `schedule_reparse`, `DriverJournalAppended` handler | ✓ VERIFIED | Classification precedes the 500ms dedup check; `schedule_journal_tail` never touches `last_refresh` or `reparse_dispatches`; `DriverJournalAppended` handler only advances its own `(alias, run_id)` cursor entry. |
| `tests/journal_crash.rs` | Real-SIGKILL survival + on-disk redaction | ✓ VERIFIED | Read in full. Re-execs the test binary via `std::env::current_exe()`, selects the child role with an env sentinel, kills with `Child::kill()` (SIGKILL on unix), asserts on bytes read back from the killed process's file. |
| `tests/journal_gitignore.rs` | Ignore posture proved against a real repo | ✓ VERIFIED | Read in full. Uses `git add -A` + `git ls-files` (ground truth), **not** `git check-ignore -q` — the exact method the verification brief calls out as the correct one. Includes the deeper-nesting-stays-ignored case and the parent-directory-exclusion diagnostic. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `src/journal/writer.rs` (`JournalWriter::append`) | `src/journal/redact.rs` (`RedactedLine`) | `write_event` constructs a `RedactedLine` and writes only its `as_line()` output | ✓ WIRED | `writer.rs:183-190`, confirmed by direct reading. No other write path to the file exists. |
| `src/watcher.rs` (`batch_actions`) | `src/journal/mod.rs` (`classify_change`) | pure classification call before the dedup insert | ✓ WIRED | `watcher.rs:67`, confirmed the function is called with no filesystem access on the debouncer's callback thread (`grep` for `metadata\|exists\|is_dir\|is_file\|read_dir\|canonicalize` inside `classify_change`'s body returns 0 hits). |
| `src/app.rs` (`FileChanged` arm) | `src/journal/mod.rs` (`ChangeKind`) | classification dispatch before the 500ms dedup check | ✓ WIRED | `app.rs:477-508`, confirmed the `DriverJournal` arm never reaches the `last_refresh` code path (D-14). |
| `src/journal/mod.rs` (`from_exec_event`) | `src/executor/mod.rs` (`ExecutionEvent`) | reads existing fields only, widens no Phase 15 type | ✓ WIRED (and non-invasive) | `git diff --stat f1e5cda HEAD -- src/executor/` is empty for the whole phase; confirmed directly. Every `ExecutionEvent` variant maps to a journal kind in `EMITTED_KINDS`, verified by `every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase`. |
| `tests/journal_crash.rs` | `src/journal/writer.rs` (`crash_test_writer_loop`) | re-exec'd child calls the real writer loop | ✓ WIRED | Confirmed by reading both files: the child sentinel branch in `journal_child_writer` calls `crash_test_writer_loop`, which is `pub` (not `#[cfg(test)]`) specifically so a re-exec'd child process can reach it. |

### The Three Vacuous-Check Hazards — Explicitly Checked

1. **Filtered `cargo` output.** Every gate command in this pass used `rtk proxy cargo ...` for build/test/clippy. Confirmed the summarising wrapper strips raw lines by comparing: `rtk proxy cargo clippy --all-targets 2>&1 | grep -c '^warning: '` returned **6** (includes the trailing `generated N warnings` crate-summary line, which also matches the pattern), while `| grep -v generated | wc -l` returned the true count, **5**. Used the corrected form throughout. `rtk proxy cargo test` was used for the full suite and its raw `test result:` lines were visible and counted directly (427 passed, 0 failed).

2. **`git check-ignore` inversion.** `tests/journal_gitignore.rs` does **not** use `git check-ignore -q`. It uses `git add -A` followed by `git ls-files` (`tracked()` at line 87-104) — asking the index what it actually holds, which cannot be inverted. I read this file in full and confirmed both the presence assertions (ignore file + both `run.json`s at the documented depth are tracked) and the absence assertions (`journal.jsonl`, `inbox.jsonl`, the `active` pointer, and a deeper-nested `run.json` are never tracked) are both present — a one-directional check would have passed against an ignore file that ignores nothing. `parent_excludes_run_record` (`writer.rs:322-362`) itself documents the inversion hazard in its own doc comment and inspects the *reported pattern* rather than the exit status for exactly this reason.

3. **Zero-assertion without a control arm.** `journal_appends_never_trigger_a_full_reparse` (the zero arm) is paired with `a_planning_write_still_triggers_a_reparse` (the control arm), and I read both bodies directly rather than trusting the SUMMARY's claim that the pairing exists. The control arm asserts `reparse_dispatches == before + 1`, not merely "changed" — a genuine, non-decorative positive assertion using the identical `obs_app` test harness as the zero arm. The source itself documents why the control arm is required (`app.rs:959-962`: *"A test that only asserts zero would pass against a handler that dropped `FileChanged` entirely"*). Both tests independently re-run and passed.

Additionally, for criterion 3: both `src/journal/writer.rs#a_planted_credential_is_already_redacted_in_the_file_bytes` and `tests/journal_crash.rs#a_sigkilled_writers_surviving_bytes_are_already_redacted` assert against bytes read back from disk via `std::fs::read_to_string`/`std::fs::read`, not an in-memory `Value`, and both include the dash-encoded home-path form (`-home-fakeuser-projects-secretrepo` in the crash test; six dedicated WR-15 rows in the redaction corpus) as its own explicit, separately-named assertion rather than folded into a slash-form-only check.

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|---|---|---|---|---|
| OBS-01 | 16-01, 16-02, 16-03, 16-04, 16-06 | Append-only journal survives process death and is the source of truth for run state | ✓ SATISFIED | Real-SIGKILL survival test (criterion 1), tolerant reader (torn line / seq gap / unknown kind all non-fatal), `run.json` written exactly twice with atomic persist, dropped-event count (D-33) reaches the journal (`a_dropped_event_count_reaches_the_journal`, independently re-run). |
| OBS-06 | 16-05, 16-06 | Driver journal writes do not trigger full project re-parses | ✓ SATISFIED | Zero-arm + control-arm pair independently re-run and read in full (criterion 2 above); `schedule_reparse` is documented and enforced as the sole `parse_project_state` call site. |
| SAFE-04 | 16-01, 16-02, 16-06 | Credentials/tokens redacted at capture, not at render | ✓ SATISFIED | Type-enforced `RedactedLine` seam, 29-row pinned corpus with idempotence, byte-level assertions in both the writer's own unit test and the real-SIGKILL crash test, both encodings of a home path covered (criterion 3 above). |

No orphaned requirements: REQUIREMENTS.md maps exactly these three IDs to Phase 16, and all three are declared across the phase's six plans.

**Administrative note, not a code gap:** `.planning/ROADMAP.md` line 90 and `.planning/REQUIREMENTS.md` lines 58/66/71 still show `[ ]` (unchecked) for Phase 16 / OBS-01 / OBS-06 / SAFE-04. The shipped code satisfies all three requirements as detailed above; this is a tracking-document staleness issue of the same kind Phase 15's verification flagged, not a gap in the implementation.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| — | — | No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/placeholder markers found in any file this phase touched (`src/journal/*.rs`, `src/watcher.rs`, `src/action.rs`, `src/app.rs`'s journal-related sections, `tests/journal_crash.rs`, `tests/journal_gitignore.rs`) | — | None — swept directly, not from the SUMMARYs' claims. |

Two things noted in the SUMMARYs as "look like stubs and are not" were independently verified as correctly scoped, not silently unwired: `RunStarted.dry_run` is hardcoded `false` because dry-run is Phase 17's field to populate (the field exists now purely so that phase adds no schema migration); and `RESERVED_KINDS` (`observed`, `decided`, `parked`, `interjected`) are schema-only kinds that Phase 18/20 will emit — `every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase` mechanically proves this phase never emits them, independently re-run and passing.

### Behavioral Spot-Checks (independently run, not trusted from SUMMARY)

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full workspace build | `rtk proxy cargo build` | `Finished` profile, clean | ✓ PASS |
| Full test suite | `rtk proxy cargo test` | 427 passed, 0 failed (366 lib + 11 executor_lifecycle + 7 executor_transport + 3 journal_crash + 3 journal_gitignore + 12 registry + 25 state_reader) — matches the expected total exactly (baseline 363 + 64) | ✓ PASS |
| Lib clippy gate | `rtk proxy cargo clippy -- -D warnings` | exit 0 | ✓ PASS |
| `--all-targets` clippy delta (corrected for the trailing summary line) | `rtk proxy cargo clippy --all-targets 2>&1 \| grep '^warning: ' \| grep -v generated \| wc -l` | `5` — 3x `browser.rs` literal-bool `assert_eq!`, 1x `project_creator.rs` owned-instance, 1x `state_reader/mod.rs` items-after-test-module — all pre-existing, none in `src/journal/`, `src/watcher.rs`, `src/action.rs`, or the two new test files | ✓ PASS |
| Zero new dependencies across the whole phase | `git diff --stat f1e5cda HEAD -- Cargo.toml Cargo.lock` | empty | ✓ PASS |
| SIGKILL crash suite (named) | `rtk proxy cargo test --test journal_crash` | 3 passed, 0.08s | ✓ PASS |
| Gitignore posture suite (named) | `rtk proxy cargo test --test journal_gitignore` | 3 passed, 0.02s | ✓ PASS |
| Journal module unit tests | `rtk proxy cargo test --lib journal::` | 46 passed | ✓ PASS |
| App-context OBS-06 tests (zero-arm + control-arm + D-14 + cursor isolation) | `rtk proxy cargo test --lib app::` | 12 passed | ✓ PASS |
| Watcher dedup tests | `rtk proxy cargo test --lib watcher::` | 7 passed | ✓ PASS |
| No `sync_all`/`sync_data` in the writer (D-04) | `grep -c 'sync_all\|sync_data' src/journal/writer.rs` | 0 | ✓ PASS |
| No `BufWriter` in the writer (Pitfall 8) | `grep -c 'BufWriter' src/journal/writer.rs` | 0 | ✓ PASS |
| No `RedactedLine` trait impls (D-22 seam) | `grep -c 'for RedactedLine' src/journal/redact.rs` | 0 | ✓ PASS |
| No debt markers in phase-touched files | manual sweep, all files read in full | 0 | ✓ PASS |

### Probe Execution

Not applicable — this phase has no `scripts/*/tests/probe-*.sh` convention and none is referenced in any of the six PLAN/SUMMARY files. Verification instead independently re-ran the specific named tests that evidence each success criterion, plus the full project gate, rather than trusting either the SUMMARYs' reported numbers or a probe script.

### Human Verification Required

None. All three roadmap success criteria resolved to VERIFIED via a combination of direct source reading and independently re-running the specific tests that exercise the claimed behaviors (real signal delivery, real git staging, a genuine paired control-arm assertion) — nothing here requires a human to observe runtime/visual behavior that a real test run cannot demonstrate. Phase 16 ships no UI (confirmed: the only `src/ui/` changes across the whole phase are two `AppContext` fields and their test-helper initialisation, with no render call added for journal content).

### Gaps Summary

None. All three ROADMAP success criteria are met by the shipped code, independently verified rather than taken from the SUMMARYs:

1. **Survives death** — a real `SIGKILL` against a process running the production `JournalWriter` (via the `pub` re-exec entry point `crash_test_writer_loop`, not a shell stand-in) leaves an unbroken run of sequence numbers, at most one torn trailing line, and every complete line valid JSON.
2. **Costs nothing to watch** — a byte-offset tail is the only route a driver-journal `FileChanged` event takes; `parse_project_state` is proved to have exactly one call site and a genuine, independently-verified control-arm test proves the fork is real rather than a dropped handler.
3. **Redacted on disk** — a type-enforced seam (`RedactedLine`, zero trait impls, private field, crate-private accessor) makes an unredacted write path uncompilable; a 29-row pinned corpus plus an idempotence property cover both the slash and dash-encoded home-path forms; and a real post-SIGKILL byte read confirms both a planted credential and a planted dash-encoded home path are already replaced in the surviving file, not merely in a rendered view.

The only finding surfaced is administrative, not a code gap: ROADMAP.md and REQUIREMENTS.md checkboxes for this phase's requirements remain unchecked despite the shipped code satisfying them — the same class of staleness Phase 15's verification report flagged and explicitly did not treat as a gap.

---

_Verified: 2026-07-29_
_Verifier: Claude (gsd-verifier)_
