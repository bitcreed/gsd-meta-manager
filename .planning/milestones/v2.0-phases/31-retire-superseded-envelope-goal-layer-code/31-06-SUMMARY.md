---
phase: 31-retire-superseded-envelope-goal-layer-code
plan: 06
subsystem: planning-record
tags: [docs, security, threat-closure, archive, traceability]

requires:
  - phase: 31-retire-superseded-envelope-goal-layer-code
    provides: 31-05 deleted the Phase 19 guard (13ba5ac) and 31-04 the goal layer (2a23d9e, 9caf162), so the closure rows can cite real deletion shas
provides:
  - a dated Phase 31 closure table in the Phase 19 deferred-items.md, one row per open threat id (27), sorted, classed G/N/K
  - 19-SECURITY.md superseded (threats_open 0, register_totals_after_phase_31, appended closure section) with the audit trail untouched
  - Phase 21 criterion 4 closed as superseded (D-11 conditional), CR-01 and CR-02 kept open with backlog owners
  - SAFE-05/07/08 traceability rows say the guard is retired
  - the Phase 22 plans archived in place as superseded with 22-SUPERSEDED.md
affects: [31-07, verify-work]

actuals:
  tokens: 21000
  tasks: 3
  commits: 3

plan_head_before: 358e18bc48cef59d425f0cf8b6178eb9d43dc9d6
plan_head_after: 4c37336ad28471d1417311ca8611fd6c107d6037
commits: 3

tech-stack:
  added: []
  patterns:
    - "closure tables are appended beside the audit trail, never over it; a diff gate against the research commit shows only three replaced frontmatter keys"
    - "a threat against surviving code is closed only if the push step removes the attack, else it stays open with a backlog todo as its owner"

key-files:
  created:
    - .planning/phases/22-container-execution-target/22-SUPERSEDED.md
    - .planning/todos/pending/2026-10-24-session-detector-resume-optional-arity.md
    - .planning/todos/pending/2026-10-25-session-detector-wire-forms-omit-session-id.md
    - .planning/todos/pending/2026-10-25-phase-19-surviving-pin-gaps-scan-and-credential-source.md
  modified:
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
    - .planning/REQUIREMENTS.md
    - .planning/phases/22-container-execution-target/22-01-PLAN.md
    - .planning/phases/22-container-execution-target/22-02-PLAN.md
    - .planning/phases/22-container-execution-target/22-03-PLAN.md
    - .planning/phases/22-container-execution-target/22-04-PLAN.md

key-decisions:
  - "T-19-72 and T-19-73 are class K and stay OPEN (low), owned by one backlog todo: both are missing-pin gaps in surviving scan.rs / config.rs that the push step running outside the sandbox does not remove [inferred — audit]"
  - "CR-02 of 21-VERIFICATION is the wire_forms() certificate gap, not the shell injection research A5 described (that was a pass-10 review row, fixed by 4b9f18b); it is still live and gets its own todo [inferred — audit]"
  - "threats_open is 0: no row left open is at or above high. The six blocking ids close as superseded, four of them (T-19-86, T-19-112, T-19-116, T-19-121) conditional on the pending 28.1 D-11 live probe [inferred — audit]"
  - "T-19-17r is recorded as a note under the table, not a row: it is not a register id, so the row count stays equal to the register's 27 open [inferred — audit]"
  - "requirements mark-complete was NOT run: D-17 and the task acceptance criteria require RETIRE-01..03 to stay Pending until verification"

patterns-established:
  - "the open set of a long-lived register is enumerated from the register's own bookkeeping table, not from a terminal statement's prose"

requirements-completed: [RETIRE-02, RETIRE-03]  # copied verbatim from the plan. RETIRE-03's SC4 (container dry run) is OPEN, see "Open items"

coverage:
  - id: D1
    description: "Every open Phase 19 threat id has one closure row naming its class, the superseding mechanism and its status"
    requirement: RETIRE-03
    verification:
      - kind: other
        ref: "awk row gate over deferred-items.md: rows=27, sorted, no dup, six blocking ids present, set equals the register's open set"
        status: pass
    human_judgment: false
  - id: D2
    description: "19-SECURITY.md reads status superseded with threats_open equal to the open-at-high rows, earlier text unchanged"
    requirement: RETIRE-03
    verification:
      - kind: other
        ref: "git diff 036426d on 19-SECURITY.md and deferred-items.md: removed lines are exactly status, threats_open, updated"
        status: pass
    human_judgment: false
  - id: D3
    description: "Phase 21 criterion 4 closed as superseded; CR-01 and CR-02 owned by backlog todos; the older render-probe gap cited as fixed"
    requirement: RETIRE-03
    verification:
      - kind: other
        ref: "grep of the three headings in 21 deferred-items.md; rtk proxy cargo test --lib -- the_screen_renders_identity_escaped (1 passed)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Phase 22 plans archived in place as superseded; bollard absent from Cargo.toml and Cargo.lock"
    requirement: RETIRE-02
    verification:
      - kind: other
        ref: "sed -n 1,8p 22-0N-PLAN.md | grep -cE superseded keys = 2 for all four; grep -ci bollard Cargo.toml Cargo.lock = 0 and 0"
        status: pass
    human_judgment: false
  - id: D5
    description: "The N-class closures lean on Phase 28.1, whose setup-token reach probe (D-11) is still pending"
    requirement: RETIRE-03
    verification: []
    human_judgment: true
    rationale: "28.1-UAT.md test 1 needs a browser OAuth login to mint a setup-token; until it passes, every N row and the criterion-4 closure are conditional by design"

duration: 35min
completed: 2026-10-25
status: complete
---

# Phase 31 Plan 06: Close the superseded books Summary

**All 27 open Phase 19 threat ids get one sorted closure row (22 G, 3 N, 2 K), 19-SECURITY.md reads `superseded` with `threats_open: 0`, Phase 21 criterion 4 closes as superseded while CR-01 and CR-02 keep backlog owners, and the Phase 22 plans are archived in place.**

## Performance

- **Duration:** ~35 min
- **Completed:** 2026-10-25
- **Tasks:** 3
- **Files modified:** 12 (8 existing, 4 new)

## Accomplishments

- **Phase 19 closure table.** `deferred-items.md` ends with `## Phase 31 closure (2026-10-25)`: 27 rows sorted by numeric id, columns `Threat ID | Severity | Class | Superseded by | Pointer | Status`. G rows cite `13ba5ac` (and `f114b66` where `ParkReason` moved). N rows point at LNCH-09/10/12 and carry the literal `reopens against Phase 28.1 if the D-11 live probe fails`.
- **19-SECURITY.md superseded.** Frontmatter `status: superseded`, `superseded_by: phase-31`, `updated: 2026-10-25`, `threats_open: 0`; the original `register_totals` line and comments stay, followed by `register_totals_after_phase_31: 139 total / 137 closed / 2 open / 0 at or above high`. An appended `## Phase 31 closure` section lists the six blocking ids and says what the closure does not claim.
- **Phase 21.** `# Phase 31 closure (2026-10-25)` closes criterion 4 (quoted verbatim) as superseded, cites 28.1-VERIFICATION SC6 and `docs/DRIVER-SANDBOX.md` for the accepted planning-doc residual, and carries the D-11 conditional. CR-01 and CR-02 are `NOT superseded`.
- **REQUIREMENTS.md.** SAFE-05, SAFE-07 and SAFE-08 end with `; guard retired in Phase 31`, matching SAFE-02. RETIRE-01..03 (3 Pending) and LNCH-13/14 (2 Gaps Found) are unchanged.
- **Phase 22.** Four plans gain two frontmatter lines each (8 insertions, 0 deletions); `22-SUPERSEDED.md` records bwrap + `systemd-run --user --scope`, 0/4 executed, `bollard` never added (0 and 0 in Cargo.toml and Cargo.lock; also absent from their git history), and why the directory stays.

## Pre-closure open set and the 27 / 26 reconciliation

Enumerated from `19-SECURITY.md`'s own bookkeeping table ("Audit 13's bookkeeping", ~line 15985), which splits the 27 into the six that count and "the twenty-one that do not". Each id's latest status was checked in the register and later audit tables.

- **Six at or above `high`:** T-19-86, T-19-91, T-19-111, T-19-112, T-19-116, T-19-121.
- **Twenty-one below `high`:** T-19-61 to T-19-73 (13), T-19-84, T-19-85, T-19-96, T-19-105, T-19-110, T-19-113, T-19-115, T-19-127.
- **Reconciliation.** Audit 13's terminal statement names 26 (it omits T-19-105 from its "beside them" list). The 27th is **T-19-105**, which the frontmatter count, the group table and audit 8 to 12's lists all include; the register row reads "ADDRESSED for the config-resolution axis, and OPEN for the tenth consecutive round", and no later audit closes it. Research A1 guessed this id correctly.
- **Not rows.** `T-19-17r` is an unregistered acceptance (no register row, no `AR-19-13`) and is noted as G beneath the table. Several register rows that read `open` where first written (T-19-94, 95, 99, 101, 102, 106, 107, 109, 114, 117, 120, 124, 125, 126) are closed by later audits, so research's K-candidates T-19-117 and T-19-120 were already closed and have no row.

## Class counts and dispositions

| Class | Count | Ids | Disposition |
|---|---|---|---|
| G | 22 | 61-71, 84, 85, 86 (+N), 91, 96, 105, 110, 111, 113, 115, 127 | closed (superseded), cite `13ba5ac` |
| N | 3 | 112, 116 (+G), 121 | closed (superseded), D-11 conditional |
| K | 2 | 72, 73 | **open**, owner: backlog todo |

Post-closure `threats_open` is **0** (both open rows are `low`), down from 6. **K rows:** T-19-72 (`scan.rs:188`, "Absent never permits" unpinned) and T-19-73 (`config.rs:440`, no literal-token drift pin) both remain true at HEAD; the push step running outside the sandbox does not remove a missing test pin, so they are not called superseded. Owner: `.planning/todos/pending/2026-10-25-phase-19-surviving-pin-gaps-scan-and-credential-source.md`.

## 21-VERIFICATION `status: failed` truths

| Truth | Disposition | Evidence |
|---|---|---|
| ROADMAP criterion 4 (behaviour unverified) | closed as superseded | 2a23d9e, 9caf162 deleted the corpus and goal layer; LNCH-04/09/10/12; SC6 residual; D-11 conditional |
| CR-01 (`-r`/`--resume` arity) | **NOT superseded**, live | `src/session_detector.rs:728-734`; todo `2026-10-24-session-detector-resume-optional-arity.md` |
| CR-02 (`wire_forms()` omits `--session-id`) | **NOT superseded**, live | `src/ui/screens/detail.rs:22286` still four rows; todo `2026-10-25-session-detector-wire-forms-omit-session-id.md` |
| render-probe `the_screen_renders_identity_escaped` (older addendum, head f442881) | fixed | `46cfdf7` (RED) and `f0d5d6a` (fix); passes at HEAD (1 passed, 0 failed) |

Research A5 described CR-02 as a `sh -c` shell injection at `detail.rs:1692-1707`. That was a pass-10 review row, fixed by `4b9f18b` (21-27); `detail.rs` has no `"sh"`, `"bash"`, `"-c"` or `/bin/sh` literal at HEAD. The CR-02 that 21-VERIFICATION's frontmatter records is the certificate gap above.

Extra todo paths (new files of this plan beyond the declared set): `.planning/todos/pending/2026-10-25-session-detector-wire-forms-omit-session-id.md` and `.planning/todos/pending/2026-10-25-phase-19-surviving-pin-gaps-scan-and-credential-source.md`.

## Task Commits

1. **Task 1: Phase 19 threat closure (tracer)** - `f2b2886` (docs)
2. **Task 2: Phase 21 closure, todos, SAFE rows** - `06f74a5` (docs)
3. **Task 3: Phase 22 archived as superseded** - `4c37336` (docs)

**Plan metadata:** the docs(31-06) commit that follows this file.

## Verification

- Task 1 row gate: `rows=27`, `sorted`, no `dup`, no `missing`; the set equals the 27 ids above (diff of the enumerated list against the table: empty). N-class and G-class awk gates print nothing.
- Append-only gate (`git diff 036426d`, Phase 19 files): removed lines are exactly `-status: blocked`, `-threats_open: 6`, `-updated: 2026-09-04`.
- Task 2: three headings present, conditional count 1, `grep -cE '^\| SAFE-0(5|7|8) .*guard retired'` = 3, RETIRE Pending = 3, LNCH-13/14 Gaps Found = 2, todo `## Problem` count 1.
- Task 3: each Phase 22 plan prints 2 for the key grep; `git diff --stat 036426d` shows 2 insertions each, no deletions; `22-SUPERSEDED.md` has `systemd-run --user --scope` (2) and `bollard` (2); Cargo.toml and Cargo.lock 0 and 0; directory present.
- `rtk proxy cargo test --lib -- the_screen_renders_identity_escaped`: 1 passed. No other tests run; this plan changes only `.planning/` files.

## Decisions Made

See `key-decisions`. In short: K rows stay open, `threats_open` is 0, T-19-17r is a note and not a row, and `requirements mark-complete` was deliberately not run.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Two extra backlog todos**
- **Found during:** Tasks 1 and 2
- **Issue:** The plan required a named owner for any live K row and any live CR gap but declared only the CR-01 todo as a file.
- **Fix:** Created one todo for CR-02 and one for the two live K rows (T-19-72, T-19-73), so no open item is ownerless.
- **Files modified:** the two todo files listed above
- **Committed in:** `06f74a5`

**2. [Rule 1 - Bug] Capitalised conditional**
- **Found during:** Task 1 acceptance gate
- **Issue:** Three N rows wrote `Reopens against Phase 28.1...`, failing the literal-match awk gate.
- **Fix:** lower-cased before the commit.

**Total deviations:** 2 auto-fixed (1 missing critical, 1 bug). **Impact:** no scope change.

## Issues Encountered

None blocking. The research's list of K candidates (T-19-117, T-19-120) was stale, and its CR-02 description belonged to a different pass; both were resolved by reading the register and verification file.

## Open items (stated plainly)

- **Plan 31-07 (D-04: Codex executor and in-process Claude transport removal) is HELD pending a user decision and did not run.** The executor code remains in the tree, callerless, with a `dead_code` allow on `DrivableProject::with_runtime`. Nothing owned by 31-07 is recorded as done here; the `13ba5ac` rows cite only the guard deletion.
- **SC4 (container dry run) is not green.** 31-05 reports `./scripts/pre-tag-check.sh --container v1.15.0` red at gate 4 on two pre-existing 28.1 launcher tests (`planted::a_truncation_the_baseline_had_too_...`, order-dependent on `read_dir` under overlayfs; `launcher_sandbox::no_flag_names_a_credential_directory...`, which needs `/run/user/1000`). No record written by this plan claims otherwise, so RETIRE-03's gate half stays open and RETIRE-03 should not flip at verification until it is settled.
- **D-11 live probe pending** (28.1-UAT.md test 1): the N rows and the criterion-4 closure are conditional on it by design.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

The planning record is consistent with the tree. Remaining Phase 31 work: the held 31-07 decision and the SC4 gate.

## Threat Flags

None. No source, endpoint or schema changed.

---
*Phase: 31-retire-superseded-envelope-goal-layer-code*
*Completed: 2026-10-25*

## Self-Check: PASSED

- Created files exist: 22-SUPERSEDED.md and the three todos, checked with `[ -f ]`.
- Commits `f2b2886`, `06f74a5` and `4c37336` are ancestors of HEAD.
