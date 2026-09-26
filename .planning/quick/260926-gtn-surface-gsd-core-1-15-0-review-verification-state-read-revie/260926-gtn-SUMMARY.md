---
phase: quick-260926-gtn
plan: 01
subsystem: state_reader / driver router / Phases tab
status: complete
tags: [gsd-core-1.15.0, verification, unparseable, review-disposition, conformance]
requires: ["260926-gtm"]
provides:
  - VerificationStatus::Unparseable + closed/unterminated-aware verification reader
  - RouterReason::GateVerificationUnparseable (gate_verification_unparseable)
  - DiskInference.review_disposition (REVIEW-DISPOSITION ledger counts)
  - Phases-tab Verify line and Checks-line ledger counts
affects: [src/state_reader/disk_status.rs, src/state_reader/state_md.rs, src/driver/router.rs, src/ui/screens/detail.rs, src/agents/fixers.rs, tests/driver_router_conformance.rs, README.md, docs/GETTING-STARTED.md]
tech-stack:
  added: []
  patterns: [single serde_yml parse site with one-shot repair, leading-block classifier, bounded ledger read]
key-files:
  created: []
  modified:
    - src/state_reader/state_md.rs
    - src/state_reader/disk_status.rs
    - src/driver/router.rs
    - tests/driver_router_conformance.rs
    - src/ui/screens/detail.rs
    - src/agents/fixers.rs
    - README.md
    - docs/GETTING-STARTED.md
decisions:
  - "Unparseable gets its own router reason (gate_verification_unparseable), not a fold into G6"
  - "V ladder unchanged: Unparseable counts as a concluded verification; the Verify line carries the failure"
  - "Fixer estimate is not fed from the disposition ledger"
metrics:
  duration: 12m
  completed: 2026-09-26
plan_head_before: a6f4952297b848b201828cd5f5f95b98649dbb25
actuals:
  tokens: 12900
  tasks: 3
  commits: 4
---

# Quick 260926-gtn: Surface gsd-core 1.15.0 review/verification state — Summary

A VERIFICATION.md whose closed frontmatter is broken YAML now reads `Unparseable`, and one whose block is never closed now reads `Missing`. Before this, either shape still leaked a `status: passed` line, which gave a false Complete/GoalMet. That fail-open is closed, and conformance proves it under both the 1.14.0 and 1.15.0 oracles. The Phases tab also gained a `Verify:` line that names the 1.15.0 remedy, and the Checks line now shows the counts from the REVIEW-DISPOSITION ledger as `(N open, M deferred)`.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | b886ed8 | feat(quick-260926-gtn): read unparseable/unterminated VERIFICATION.md like gsd-core 1.15.0 |
| 2 (RED) | cecd7e5 | test(quick-260926-gtn): add failing REVIEW-DISPOSITION ledger tests |
| 2 (GREEN) | 2aed3a7 | feat(quick-260926-gtn): read the REVIEW-DISPOSITION ledger and show open/deferred counts |
| 3 | 1d18eac | docs(quick-260926-gtn): describe the Verify line and review-ledger counts |

## Routing answer (verified)

1.15.0 `init.manager` still emits action `verify` for every `executed` phase, with `command = verification_next_command`. Only the command changed:
- `stale` now points to execute-phase (#4682).
- `unparseable` now has an empty command (#4806). This was measured: the 1.15.0 oracle returned `command: ""` for the executed_unparseable fixture.

This router never auto-selects a verify command (the declared Upstream::VerifyGate divergence), so neither change moves a routing decision. No 1.15.0 router, init or progress code reads the disposition ledger, so the ledger is display-only.

The one change the router can see is the closed fail-open. Both oracles read broken or unterminated frontmatter as `executed`, while this reader read `complete`.

## Task 1 evidence

**RED (before the reader change).** Each fixture was run in isolation by swapping the fixture order, because the first failing fixture panics:
- `executed_unparseable`, 1.14.0: `every_fixture_reaches_the_state_it_is_named_for` gave "reader sees `complete`". The oracle test gave "upstream routes to `/gsd-execute-phase 01` and this router must park at the gate ... Got GoalMet".
- `executed_unparseable`, 1.15.0: same, except "upstream routes to `` ... Got GoalMet".
- `executed_unterminated`, 1.14.0 and 1.15.0: "reader sees `complete`" and "upstream routes to `/gsd-execute-phase 01` ... Got GoalMet".

**GREEN.** Both fixtures now Park:
- `executed_unparseable` gives gate_verification_unparseable.
- `executed_unterminated` gives gate_stale_check_indeterminate.

Conformance line, identical under both oracles: `conformance: 5 command comparisons, 4 declared divergences, 2 upstream-silent states, over 11 fixtures`. The run was 6/6 passing under the local 1.14.0 oracle (~/.claude, VERSION 1.14.0) and 6/6 under the 1.15.0 oracle (HOME=scratchpad/fakehome).

**Measured 1.15.0 `verification_next_command` table.** The source was `query init.manager` with HOME=fakehome and GSD_RUNTIME=claude, one probe tree per status: ROADMAP Phase 01 + 01-01-PLAN + 01-01-SUMMARY + 01-VERIFICATION.md.

| VERIFICATION.md shape | verification_status | next_command | Verify line (this app) |
|---|---|---|---|
| `status: passed` + `covered_digest: "v1:sha256:bogus"` (fingerprint stale) | stale | `/gsd-execute-phase 01` | `stale — /gsd:execute-phase N re-runs the verifier (...)` |
| literal `status: stale` | unknown | `/gsd-execute-phase 01` | `stale — /gsd:execute-phase N ...` (same command) |
| literal `status: unparseable` | unparseable | `` (empty) | `unparseable — fix the YAML frontmatter ...` (no command) |
| `status: passed` + `  bad: indent` | unparseable | `` (empty) | same as above |
| `status: human_needed` | human_needed | `/gsd-verify-work 01` | `/gsd:verify-work N` |
| `status: gaps_found` | gaps_found | `/gsd-plan-phase 01 --gaps` | `/gsd:plan-phase N --gaps` |
| `status: reticulating` | unknown | `/gsd-execute-phase 01` | `<value via shown()> — /gsd:execute-phase N` |
| closed block, no status | missing | `/gsd-execute-phase 01` | `no status — /gsd:execute-phase N` |

**Agreement sweep.** The throwaway tests/zz_gtn_sweep.rs was run once and then deleted; it was never committed. The `find` covered 458 `*VERIFICATION.md` files in 449 dirs. The count has grown from the 337 recorded at planning time.
- App (infer_disk_status) Unparseable set: `{/home/blk/projects/rust/gsd-meta-manager/.planning/milestones/v1.1-phases/06-read-only-views}`.
- 1.15.0 per-file oracle (`frontmatter get --field status`, "not parseable") set: `{.../v1.1-phases/06-read-only-views/06-VERIFICATION.md}`.
- The two sets agree exactly. No anchor/alias residual showed up in the corpus.

## Task 2

The ledger never sets `has_review`, and the name arm runs ahead of every `*REVIEW` arm. `fixers::is_code_review_name("12-REVIEW-DISPOSITION.md", 12)` returns false; this needed no production change, and a test now pins it.

Counts come from the table rows:
- Upstream's id grammar and case-sensitive enum are used, with `open` as the fallback.
- Duplicate ids are counted once, first occurrence wins.
- Rows inside fences are skipped.
- Reads are capped at 256 KiB.
- If several ledgers exist, the sorted-first one is read.

RED showed 6 failing tests at cecd7e5, where the scaffolding always returned None. GREEN is at 2aed3a7.

## Gates

- `rtk proxy cargo test --no-fail-fast`: **2722 passed, 1 failed, 15 ignored.** The lib crate alone was 1862 passed, 1 failed, 1 ignored.
  - The only failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the permitted git-version witness. Nothing else failed.
  - The "Text file busy" flake did not occur.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean (exit 0).
- Conformance: passes under both 1.14.0 and 1.15.0, with the line above.

## INFERRED decisions (for audit)

- **INFERRED:** Unparseable gets a named router reason, `gate_verification_unparseable`, instead of being folded into G6.
  - After this change G6 means "artifact present, block valid or unterminated, no status", which is upstream's `missing`.
  - `unparseable` is upstream's own distinct key, so the journal says which one fired.
- **INFERRED:** The Verify line uses this tab's `/gsd:` spelling, matching the neighbouring `No plans yet — /gsd:plan-phase N` hint, rather than upstream's `/gsd-`.
- **INFERRED:** `derive_all_stage_statuses` and `build_pipeline_line` are unchanged. The V ladder keeps meaning "a verification artifact concluded something", and Unparseable counts as concluded because the verify step ran (upstream #4806's reading). The Verify line carries the failure, and the Driver tab reuses the ladder unchanged.
- **INFERRED:** The Stale remedy wording is "re-runs the verifier (a stale report is not refreshed by re-verifying)". The plan asked for a note that verify-work cannot refresh it, and it also required that the Stale text never contain `verify-work`. The wording satisfies both.
- **INFERRED:** The fixer estimate (`N fixers · ~x/y fixed`) is NOT fed from the ledger.
  - That estimate's numerator is live, taken from `fix(NN)` commit subjects over the current REVIEW.md.
  - The ledger is written AFTER a fix pass (`record_disposition`), so mixing the two would put a previous pass's record into a live estimate.
  - The ladder widths are also pinned by tests.
- **INFERRED:** Ledger counts are drawn only when `has_review` is true. Upstream writes the ledger only after reading REVIEW.md (code-review-disposition.md:145-170, :276-282), so an orphan ledger is not a state upstream produces.
- **INFERRED:** The 256 KiB ledger cap is spelled locally as `REVIEW_DISPOSITION_READ_CAP` instead of importing `agents::fixers::REVIEW_READ_CAP`, because `state_reader` does not depend on `agents`. The value is the same.
- **INFERRED:** A ledger row must start with `|` at column 0, mirroring upstream's `^\|` anchor. An indented table row is not counted.
- **INFERRED:** The Task 2 RED commit carries minimal scaffolding (the type, the always-None field and a stub parser) so that the tree compiles at every commit and bisect keeps working.

## Named residuals

- **Anchors/aliases and U+E000.** Upstream refuses these and reads them as `unparseable`. serde_yml accepts them, so an anchor-bearing `status: passed` reads Passed here. There is no security delta, and none appeared in the 458-file corpus.
- **BOM-prefixed files.** A VERIFICATION.md starting with U+FEFF reads Missing here, because the first line is not a bare `---`. Upstream strips the BOM. The result fails safe (it parks) but diverges.
- **YAML bomb.** serde_yml bounds it with "alias expansion limit exceeded" after about 2.4 s in release. It runs on the refresh `spawn_blocking` thread, which is the same exposure STATE.md parsing already has.

## Follow-up (out of scope, not built)

Upstream's fingerprint/mtime staleness scan is not modelled by this reader: `covered_files`/`covered_digest`, with a SUMMARY-mtime fallback. It predates 1.15.0.
- The probe proves the gap. `status: passed` + `covered_digest: "v1:sha256:bogus"` reads `stale`/`executed` in 1.15.0, while this reader reads Passed/Complete.
- Both oracles read this repo's phases 19/24/25 as `stale`.
- The Stale hint therefore fires only for a literal `status: stale` until that scan is ported.
- A conformance fixture that declares a mismatched `covered_digest` would expose the divergence and is the natural RED for the port.

## Deviations from Plan

None beyond the INFERRED items above. The sweep corpus had grown from 337 to 458 files; the sets still agree.

## Threat Flags

None. No new network, auth or path surface was added. The ledger path comes only from `read_dir` names with a fixed suffix (T-gtn-06). The Verify line escapes the phase number and any unknown status value through `shown()`, and the Checks-line counts are integers only (T-gtn-04).

## Self-Check: PASSED

- Commits b886ed8, cecd7e5, 2aed3a7 and 1d18eac exist on master (`git rev-list --count a6f4952..HEAD` = 4).
- All 8 modified files exist, and tests/zz_gtn_sweep.rs is absent.
