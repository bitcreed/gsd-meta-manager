---
phase: 34-owner-items-reader-inbox-cli
verified: 2026-11-27T07:10:00Z
status: passed
score: 7/7 must-haves verified
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
  - "docs/OWNER-INBOX.md"
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
covered_digest: "v4:sha256:b8bb1cfbd52e55ba5e73a408140e7895f045eeb5a2ec6f8d251191dac6615e6a"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 5/7
  gaps_closed:
    - "Truth 6 (watcher invalidates the parse cache): now behaviorally evidenced"
    - "Truth 7 (backstop concurrency invariants): directly observed in this run"
    - "Human items 1 and 2: settled by the user in 34-UAT.md (skipped, deferred permanently)"
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "WR-01 (34-REVIEW.md): a VERIFICATION with an unterminated or non-mapping frontmatter block reads as clean (src/state_reader/owner_items/frontmatter.rs:112-134)"
    category: other
    reason: "No must-have breaks: OWN-11 requires 'no panic on unterminated frontmatter' and none occurs. It mirrors upstream's `{}` and is inconsistent with disk_status (Missing). Disposition open; worth a declared-divergence decision or a fix."
    evidence_status: "reasoned from code and review; not reproduced as a failing test"
  - finding: "WR-02 (34-REVIEW.md): HANDOFF.json read via plain read_to_string in parse_project_state (src/state_reader/mod.rs:1008)"
    category: security
    reason: "A FIFO named HANDOFF.json could hang the one-shot CLI. The read sits in the pre-existing state reader (same flaw as the STATE.md, ROADMAP.md and .continue-here.md reads), outside the owner-items reader OWN-11 hardens; 34-SECURITY.md has threats_open 0. Disposition open."
    evidence_status: "reasoned from code and review; not reproduced"
---

# Phase 34: Owner items reader + inbox CLI Verification Report

**Phase Goal:** On any machine, `gsd-meta-manager inbox` answers "what checks do I have to run?" for every registered project in one call, read natively from the files (no gsd-core process spawned), with per-item results that match gsd-core's `audit-uat` and the owner's rules (lanes, dedup, acknowledged, archived and deferred handling, closeable-by-testing) applied on top.
**Verified:** 2026-11-27T07:10:00Z
**Status:** passed
**Re-verification:** Yes. The earlier report (human_needed, 5/7) went stale because README.md, src/status_cli.rs, tests/inbox_cli.rs and tests/status_cli.rs changed. Phases 35 to 38 and quick tasks have since extended the same code. Everything below was re-checked against HEAD, not carried forward.

## Goal Achievement

The goal holds at HEAD. The phase-34 behavior (native reader, oracle parity, owner rules, schema v1, `owner` counts) still passes its pinned tests. Later phases extended it additively.

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: `inbox` lists missing UATs for all registered projects in one call; wordoclock reports 18 open checks (not 39 or 41); projects without a phases dir return empty groups, not an error | VERIFIED | The release binary built from HEAD. Live `inbox --json` over the registry (now 3 projects) exits 0 with 3 project entries in one call, in 0.097 s. The wordoclock figure and the no-phases case are pinned on fixtures: `tests/owner_digest.rs` 45 passed (includes `gaps_merge_reports_ten_failed_checks_and_fourteen_gaps` and the supersede-six-phases case) and `no-phases` is a declared oracle divergence. Wordoclock is no longer registered here, so the 18 figure is fixture-proven, not live. |
| 2 | SC2: per-item (file, test, result, category) equals `audit-uat --raw` on every fixture; oracle test green; a source scan proves nothing under `src/` spawns `audit-uat` | VERIFIED | `cargo test --release --test owner_audit_conformance`: 3 passed against the installed oracle. The source-scan test is among them. A grep of `src/state_reader/owner_items` and `src/inbox*.rs` for `Command::new` / `process::Command` finds nothing. Container pre-tag run is the owner's release step and was not re-run. |
| 3 | SC3: acknowledged files hidden while the snapshot matches and resurface on change or malformed marker; archived and deferred never enter current counts; a `status: testing` UAT with open items counts as outstanding | VERIFIED | Observed live on a copy of `uat-ack-match`: `groups 0, acknowledged_files 1`; after editing the marker snapshot, `groups 1, acknowledged_files 0`; after restoring it, `groups 0, acknowledged_files 1`. The four ack fixtures, archived-complete-skipped and deferred-200 are in `owner_digest` (45 passed). `status_cli` (29 passed) covers `testing` and the other non-terminal UAT statuses firing `uat_outstanding`. |
| 4 | SC4: `inbox --json` is deterministic schema v1 with `status`-style exit codes; plain `inbox` prints a lane checklist with working `--lane`/`--include`; `status --json` carries additive `owner` counts documented in `status --skill` | VERIFIED | Two consecutive `inbox --json` runs are byte-identical. `schema: gsd-meta-manager/inbox`, `schema_version: 1`. `--lane bogus` exits 2. `--lane test` prints the lane checklist (TEST sections, per-phase `/gsd-verify-work N` commands). `status --json` keeps `schema_version: 1`; each healthy project carries `owner` with `test`, `decide`, `fix_phases`, `blocked`, `closeable_phases` unchanged, plus keys Phases 35 and later added (`fix_debug`, `closes_after`). Error entries have `owner: null`. `--skill` documents `inbox --json` and the `owner` counts. `tests/inbox_cli.rs` 36 passed, pinning the key paths. |
| 5 | SC5: a cold `inbox --json` over 20 projects under 1 s, parses cached by (path, mtime, len), hostile inputs neither panic nor escape `render_for_terminal` | VERIFIED | The synthetic 20-project timing test is in the green `inbox_cli` run. Prior strict-mode measurement was 177 ms (not repeated). Live run over 3 projects took 0.097 s. Cache key `(mtime, len)` is proven by `a_changed_length_or_mtime_reparses` (passes). Hostile-input tests are in the green `inbox_cli`, `owner_digest` and lib runs. Cold-OS-page-cache timing was a human item, settled by the user (see below). |
| 6 | Watcher invalidates the parse cache: FileChanged drops that project's cached parses (OWN-12) | VERIFIED | Closes the earlier behavior-unverified item. `app.rs:2973` calls `evict_project` in the `Action::FileChanged` arm. `app::tests::a_watcher_refresh_delivers_a_fresh_owner_digest` drives FileChanged, rewrites a UAT row from pending to pass, and asserts the next digest has `test == first.test - 1`, so no stale parse survives. `evicting_a_project_drops_only_its_entries` and `a_changed_length_or_mtime_reparses` also pass (3/3 run here). |
| 7 | Concurrency (`backstop`): a file rewritten mid-scan is read before or after the write, never a panicking mix; a marker written between two runs is honored by the next run | VERIFIED | Directly observed. (a) 300 alternating atomic rewrites of a UAT file while `inbox --json` ran 60 times: 0 non-zero exits, 0 panics in stderr. (b) The marker edit between runs in truth 3 was honored by the very next run, both ways. The one-shot CLI persists nothing between runs. |

**Score:** 7/7 truths verified (0 present, behavior-unverified)

### Settled by the user, not open items

`34-UAT.md` records both earlier human items as `result: skipped`, reason "Deferred follow-up: deferred permanently" (deferred_at 2026-11-27):

1. Cold-OS-page-cache live re-run of `inbox --json` with the release binary.
2. Subjective review of the plain `inbox` checklist wording and ordering.

Both are recorded as settled by user decision. They are not human_verification items and do not affect status.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/state_reader/owner_items/{mod,scan,uat,verification,deferred,rules,markdown,frontmatter}.rs` | Native Layer A reader and Layer B rules | VERIFIED | Present, substantive, wired (`inbox_cli.rs`, `status_cli.rs`, `app.rs`); no `TBD`/`FIXME`/`XXX` markers in any covered source or test file |
| `src/inbox.rs`, `src/inbox_cli.rs`, `src/cli_targets.rs` | Schema v1 document, runner, shared targets | VERIFIED | Live `inbox` output confirmed |
| `src/status_cli.rs`, `src/state_reader/disk_status.rs` | `owner` counts; `testing` outstanding | VERIFIED | Live `status --json` and 29 green tests |
| `tests/owner_audit_conformance.rs` plus fixtures under `tests/fixtures/owner/` | Oracle comparison | VERIFIED | 3 passed with the oracle present |
| `tests/owner_digest.rs`, `tests/inbox_cli.rs`, `docs/OWNER-INBOX.md` | Pins and contract docs | VERIFIED | 45 and 36 green. The inbox contract moved from the README (cut to an overview in `d40e5e64`) to `docs/OWNER-INBOX.md`; README links to it and the tests still pass |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `main.rs`/`cli.rs` | `inbox_cli::run` | `inbox` subcommand | WIRED | Binary runs `inbox` |
| `inbox_cli.rs`, `status_cli.rs` | `read_owner_digest` | shared digest | WIRED | Same digest feeds both outputs |
| `app.rs` FileChanged | `owner_items::evict_project` | watcher event | WIRED | `app.rs:2973`; exercised by the watcher-refresh test |
| `owner_items::scan` | `parse_project_state` | `ProjectState` for R7 | WIRED | `closes_phase` follows the disk statuses |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `inbox --json` groups | `OwnerDigest.groups` | file reads of UAT/VERIFICATION/deferred-items.md | Yes, live groups for 2 real projects | FLOWING |
| `status --json` `owner` | `OwnerDigest` counts | same digest | Yes, e.g. `test: 25, closeable_phases: 4` | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Inbox/digest/status suites | `cargo test --release --test inbox_cli --test owner_digest --test owner_audit_conformance --test status_cli` | 36, 45, 3 and 29 passed, 0 failed | PASS |
| Watcher, cache keying and eviction | `cargo test --release --lib -- a_watcher_refresh_delivers_a_fresh_owner_digest a_changed_length_or_mtime_reparses evicting_a_project_drops` | 3 passed | PASS |
| Determinism | two `inbox --json` runs | byte-identical | PASS |
| Exit code | `inbox --lane bogus` | 2 | PASS |
| Ack resurfacing | edit marker snapshot between runs (scratch copy of `uat-ack-match`) | hidden, resurfaced, hidden again | PASS |
| Rewrite race | 300 rewrites against 60 `inbox` runs | 0 failures, 0 panics | PASS |
| Full suite (orchestrator) | `cargo test --no-fail-fast` | 4741 passed, 1 failed (not re-run here) | see below |

The one failure is `tests/owner_settle_oracle.rs::the_installed_verify_work_still_parses_arguments_this_way`. It is a phase-39 test that pins a line of the installed gsd-core's `verify-work.md`, and the installed gsd-core has changed that line. That is environmental drift in a later phase's oracle, not phase-34 code. It is not a regression of phase 34, but the owner should refresh that pin separately.

### Probe Execution

SKIPPED: the phase declares no probe scripts.

### Requirements Coverage

All 13 IDs in the roadmap line appear in the PLAN `requirements:` fields (01: OWN-01/02/11/13; 02: OWN-01/04/11; 03: OWN-01/02/03/06/11/12; 04: OWN-03/04/06/07/09/15; 05: OWN-03/04/05/06/11/12/13/14; 06: OWN-01/05/11). REQUIREMENTS.md marks each as Phase 34 / Complete. OWN-08 and OWN-10 belong to Phase 35 and OWN-16..23 to Phases 36 and later, so no orphans.

| Requirement | Source Plan(s) | Status | Evidence |
|-------------|----------------|--------|----------|
| OWN-01 | 01, 02, 03, 06 | SATISFIED | Oracle conformance green |
| OWN-02 | 01, 03 | SATISFIED | Oracle harness and source scan green |
| OWN-03 | 03, 04, 05 | SATISFIED | supersede-six-phases fixture, 18 not 39 |
| OWN-04 | 02, 04, 05 | SATISFIED | `gaps_merge_reports_ten_failed_checks_and_fourteen_gaps` |
| OWN-05 | 05, 06 | SATISFIED | Four ack fixtures plus live marker round trip |
| OWN-06 | 03, 04, 05 | SATISFIED | archived and deferred fixtures |
| OWN-07 | 04 | SATISFIED | R7 table test |
| OWN-09 | 04 | SATISFIED | `status_cli` `testing` tests |
| OWN-11 | 01, 02, 03, 05, 06 | SATISFIED | Hostile-input tests green; the race run showed no panic. See advisory WR-01/WR-02. |
| OWN-12 | 03, 05 | SATISFIED | Cache key, watcher eviction and the watcher-refresh test |
| OWN-13 | 01, 05 | SATISFIED | Schema v1, deterministic, key paths pinned |
| OWN-14 | 05 | SATISFIED | `--lane`/`--include` work. The no-flag plain output is now a short digest (Phase 38, deliberate). The full lane checklist is `--all` or any filter flag, and the golden tests pass. |
| OWN-15 | 04 | SATISFIED | `owner` counts live; `--skill` text |

### Anti-Patterns Found

None blocking. No `TBD`, `FIXME` or `XXX` in any covered code file. Review (`34-REVIEW.md`, re-run at `fe14903e`): 0 critical, 2 warning, 2 info; the fixed items are recorded in `34-REVIEW-DISPOSITION.md`.

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/state_reader/owner_items/frontmatter.rs` | 112-134 | Unterminated or non-mapping VERIFICATION frontmatter returns an empty mapping (WR-01) | Warning (advisory) | A damaged VERIFICATION can hide a pending check from the inbox while the dashboard flags it. No panic, so OWN-11 holds. Decide: declare it an oracle divergence or return `Unparseable` for VERIFICATION. |
| `src/state_reader/mod.rs` | 1008 | `HANDOFF.json` read with plain `read_to_string` (WR-02) | Warning (advisory) | A planted FIFO could hang the one-shot CLI. The same flaw already exists in the STATE.md, ROADMAP.md and `.continue-here.md` reads. Not in the owner reader, `34-SECURITY.md` has `threats_open: 0`, and nothing in the phase's must-haves requires it. Route the read through `read_text_file`. |

### Human Verification Required

None open. The two earlier items were settled by the user (see above).

Inferred decisions for audit (the user was unavailable):

- Truth 6 is VERIFIED rather than left behavior-unverified. The evidence is the end-to-end watcher-refresh test, not a test of eviction alone; no test reads the cache after FileChanged in isolation.
- Truth 7 is VERIFIED on direct observation in this run, not on a held-out or property test. If the owner wants a durable test for it, add one.
- WR-01 and WR-02 are advisory and do not block, because neither contradicts a must-have as written.

### Gaps Summary

No gaps. All five roadmap success criteria and all 13 requirement IDs hold at HEAD. Follow-ups outside this phase: the failing Phase 39 settle-oracle pin, and the WR-01 and WR-02 hardening.

---

_Verified: 2026-11-27T07:10:00Z_
_Verifier: Claude (gsd-verifier)_
