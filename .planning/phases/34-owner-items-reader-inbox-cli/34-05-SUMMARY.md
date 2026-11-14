---
phase: 34-owner-items-reader-inbox-cli
plan: 05
subsystem: cli
tags: [owner-inbox, checklist, filters, readme-contract, golden, timing, rust]
status: complete

requires:
  - phase: 34-01..34-04, 34-06
    provides: owner_items digest, inbox JSON document, rules R1-R10, fixtures
provides:
  - "inbox::render_text: the D-21 plain checklist (lane sections, items, gaps, folds)"
  - "Filters::lists_group shared by build_document and render_text (--lane, --include archived|deferred|acknowledged|todos)"
  - "tests/owner_digest.rs: fixture-level owner numbers and two goldens"
  - "README `#### Owner work: inbox` with schema v1 fence and table, pinned by tests"
  - "hostile-output and cold-run (20 projects) tests"
affects: []

actuals:
  tokens: 90000
  tasks: 3
  commits: 2
plan_head_before: bd1aa0fb
plan_head_after: dbb54300
commits: 2

key-files:
  created:
    - tests/owner_digest.rs
    - tests/fixtures/owner_golden/inbox.txt
    - tests/fixtures/owner_golden/inbox-include-all.txt
  modified:
    - src/inbox.rs
    - src/inbox_cli.rs
    - src/text.rs
    - tests/inbox_cli.rs
    - README.md
    - Cargo.toml

key-decisions:
  - "Group listing is one predicate, Filters::lists_group: lane filter applies to the four owner lanes (Deferred is exempt and needs --include deferred); archived-scope groups need --include archived; acknowledged groups need --include acknowledged [inferred]"
  - "Text sections: TEST closes / stays open, DECIDE, FIX, BLOCKED, then OLD DEBT, DEFERRED, ACKNOWLEDGED, TODOS on request; a group is placed by scope first (archived), then lane, so an archived acknowledged group is OLD DEBT [inferred]"
  - "TEST order: closes_phase first, remaining milestone phases ascending, age oldest first, then alias, phase, file; other sections by alias, phase, file [inferred, as the plan]"
  - "Group line parts are joined by two spaces (alias, ph <ws/>phase, name, detail, (archived label), age, command); a bookkeeping-only group prints `all checks passed, run <cmd> to finalize` and no trailing command"
  - "text::terminal_capped gets an ASCII-printable fast path, pinned equal to the full pipeline for every printable byte"
  - "Dev profile builds regex, regex-automata, regex-syntax, memchr and aho-corasick at opt-level 3 so the debug-build cold run meets the 1 s budget; release is unaffected"

requirements-completed: [OWN-03, OWN-04, OWN-05, OWN-06, OWN-11, OWN-12, OWN-13, OWN-14]

completed: 2026-11-14
---

# Phase 34 Plan 05: Checklist, filters, README contract, cold-run proof

**`inbox` now prints the checklist the owner pastes into a note, `--lane` and `--include` narrow both modes identically, the owner numbers (41 raw items to 18 checks, 10 failed checks with 14 gaps) are pinned on fixtures, the schema v1 contract is in the README under test, and the container rehearsal is green with the conformance oracle in the suite.**

## Accomplishments

- Task 1 and 2 (ab99795): `tests/owner_digest.rs` runs the lib over TempDir copies of the fixtures at `now` 2026-11-14T00:00:00Z. The first run of every behavior-block expectation matched exactly, so no owner_items defect surfaced: supersede-six-phases is test 12, blocked 5, fix 1 (phase 25), noise 2, superseded 21, closeable 3 (phases 21, 22, 24); uat-gaps-merge is one Fix group {10, 10, 14, 0}; deferred-200 is 200 hidden and one deferred group of 200 under `--include deferred`; archived-complete-skipped is archived test 10, blocked 4, listed only with archived. `render_text` plus two goldens (default and include-all) over six fixtures, the lane and archived filter tests, and four binary tests (plain output, lane filter, `--include todos`, bogus lane exit 2).
- Task 3 (dbb5430): README subsection with fence, a 69-row table, usage, exit codes, stability rule, project-authored-text note and a jq recipe. `tests/inbox_cli.rs` gains `INBOX_PATHS` (69 paths), `the_inbox_key_paths_are_exactly_schema_v1` (two fixtures with gaps, supersession and a todo, plus an unresolved name, all includes), the README fence and the table read from the inbox section slice only, `hostile_strings_are_escaped_in_json_and_text` (ESC, RLO, BEL, 500-character name and expectation, both modes) and `a_cold_inbox_over_twenty_projects_is_under_a_second`.

## Verification (raw)

- `cargo test --no-fail-fast` (whole suite): 4062 passed, 0 failed. `--test inbox_cli`: 22 passed. `--test owner_digest`: 16 passed.
- `cargo clippy --all-targets -- -D warnings`: clean on the host, `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`.
- Cold timing (debug build, 20 projects x 15 phases, 1.1 MB of JSON): `inbox --json` 0.64 s, lib loop 0.40 s; before the changes below the binary took 1.2 s and failed the bound.
- `./scripts/pre-tag-check.sh --container` (no tag), exit 0:

```
=== PRE-TAG SUMMARY ===
  gate 1  tag / Cargo.toml version   SKIPPED     (no tag argument)
  gate 2  MSRV check                 PASS
  gate 3  cargo build --release      PASS
  gate 4  cargo test                 PASS
  gate 5  cargo clippy -D warnings   PASS
  gate 6  cargo package --locked     PASS
  gate 7  cross-target check+clippy  PASS        (3 targets)
```

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The cold 20-project run missed the 1 s budget (1.2 s)**
- **Found during:** Task 3
- **Issue:** debug builds of the regex stack and `terminal_capped` over about 50k strings dominated; `parse_project_state` also spawns one `git log` per project (pre-existing, Phase 33).
- **Fix:** `terminal_capped` returns printable-ASCII input of at most 200 bytes unchanged (equivalence pinned by `terminal_capped_fast_path_agrees_with_the_full_pipeline`), and the regex crates are built at opt-level 3 in the dev profile. The bound was not raised (Pitfall 10).
- **Files modified:** src/text.rs, Cargo.toml
- **Commit:** dbb5430

### Plan wording not literally followed

- Tasks 1 and 2 share one commit (ab99795): `tests/owner_digest.rs` holds both the fixture-level numbers and the golden tests, and it imports `render_text`.
- TDD: no separate RED commits; tests were written with the code.
- The plan's final read-only check against the owner's live registry (wordoclock 18 checks) was not run: real projects are read-only and out of scope for an unattended run. The same numbers are pinned by `supersede_six_phases_reports_eighteen_checks` on the synthetic analog. The verifier should run it.
- `Cargo.toml` is outside `files_modified`; see the auto-fix above.
- The cold-run margin is about 35 percent in a debug build on this machine; a much slower CI runner could flake it.
- The known `multiplexer::tmux::...focus_without_an_absolute_socket_addresses_no_server_at_all` flake did not occur.

## Known Stubs

None. `summary.decide` is 0 by plan until Phase 35 lands the decision signals.

## Threat Flags

None. T-34-18 (checklist escaping) and T-34-19/20 are covered by the hostile and timing tests.

## Self-Check: PASSED

- Files exist: tests/owner_digest.rs, both goldens, this SUMMARY.
- Commits ab99795 and dbb5430 are ancestors of HEAD.
