---
phase: 27-driverctl-driver-frontmatter
verified: 2026-10-28T00:00:00Z
status: passed
score: 4/4 must-haves verified
covered_files:
  - ".planning/phases/27-driverctl-driver-frontmatter/27-01-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-01-SUMMARY.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-02-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-02-SUMMARY.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-03-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-03-SUMMARY.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-04-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-04-SUMMARY.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-05-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-05-SUMMARY.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-06-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-06-SUMMARY.md"
  - "src/cli.rs"
  - "src/driver/direction.rs"
  - "src/driver/lock.rs"
  - "src/driverctl/counters.rs"
  - "src/driverctl/log.rs"
  - "src/driverctl/mod.rs"
  - "src/driverctl/protocol.rs"
  - "src/driverctl/scan.rs"
  - "src/driverctl/serve.rs"
  - "src/driverctl/store.rs"
  - "src/driverctl_cli.rs"
  - "src/main.rs"
  - "src/state_reader/driver_frontmatter.rs"
covered_digest: "v4:sha256:3805625d1efd85e3587c5b6c458c83c7cb668d700ed9eee0ccbf31436e39775b"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 4/4
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Phase 27: driverctl & Driver Frontmatter Verification Report

**Phase Goal:** The single writer of a run's state is a Rust subcommand of gsd-mm. Its frontmatter on DIRECTION.md is format-correct by construction and never costs a human edit.
**Verified:** 2026-10-28 at HEAD 1c6a229
**Status:** passed
**Re-verification:** Yes. The 2026-10-19 report went stale because Phase 28.1, Phase 30, Phase 31 (deletion of the `drive` subcommand and the goal layer), and the quick tasks 261021-*, 261023-v1s, 261028-n0z and 261028-n12 touched covered files. Every truth was re-checked against the code and a live binary at HEAD, not carried forward.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `driverctl serve\|log\|hook\|show` replaces `tools/driverctl.py`, same socket protocol; the spike-002 MASTER-PROMPT runs unchanged | VERIFIED | Live at HEAD: `serve` started on a scratch run dir, `log`/`hook` over the Unix socket answered. `tests/driverctl_client.rs` (20) parses every MASTER-PROMPT line; `tests/driverctl_replay.rs` (4) replays three recorded spike LOGs through the Rust client and server. All pass. `Commands::Driverctl` is still dispatched before tracing (`src/main.rs`). |
| 2 | Editor-race port: zero lost rename-saves, in-place loss at or below the spike rate; gsd-mm's own DIRECTION writes take the shared flock | VERIFIED | `tests/driverctl_race.rs` 4 passed (smoke + naive positive control + two writers under DIRECTION.lock); `editor_race_full --ignored` (run by the orchestrator at this HEAD) 1 passed. `tests/driverctl_lock_race.rs` tests 1-5 pass (5/5). `direction_lock` (`src/driver/direction.rs:318`) is the lock `splice_frontmatter` (`:408`) and `write_body` (`:437`) share; the exchange is `RenameFlags::EXCHANGE` (`:681`). |
| 3 | Every generated frontmatter parses into a typed `DriverFrontmatter`; a body without `## Direction`/`## Items` is never written back | VERIFIED | `tests/driverctl_frontmatter.rs` 12 passed (7 goldens round-trip, 18 spike files parse; 1 ignored is the golden regenerator `write_goldens`). `body_sane` at `direction.rs:131`. Live: after a full serve session (start, rejected events, hook, gap observation, SIGTERM) the body below the frontmatter was byte-identical to the input (`diff` clean). |
| 4 | Malformed events and unknown items rc=2; dispatches from hook payloads; gap rounds from `gaps_found` VERIFICATION.md on disk coalesced per 26.1; heartbeat rewrites coalesced to 120 s | VERIFIED | Live: `log bogus-event` gives `REJECTED: unknown event`, rc=2; `log status --item I-99` gives `REJECTED: --item 'I-99' is not an item in DIRECTION.md`, rc=2; an invalid `start` gives rc=2. An agent-less Agent hook yielded `dispatches.used: 1`, `dispatch_audit.observed: 1`, and the nested (`agent_id`) hook was not counted. A `gaps_found` file dropped on disk gave `gap_rounds {used: 1, phase: "3"}` and a `RUN \| gap-observed ... source=driverctl-scan` LOG line within 3 s. Code: `COALESCE_S = 45.0` with strict `<` (`counters.rs:22,99`), `HEARTBEAT_REGEN_MIN_S = 120` (`store.rs:72`), `set_gate` (`store.rs:680`), launcher-only `set_terminal` (`store.rs:1366`). |

**Score:** 4/4 truths verified, 0 behavior-unverified. Plan-level truths were also checked (see below); the only plan-level clauses not counted are the two `retired-by-31` items, which are excluded from the denominator and were never roadmap success criteria.

### Retired by Phase 31 (excluded from the denominator, not gaps)

| Plan must-have | Evidence it was retired |
|----------------|-------------------------|
| 27-01 truth D-20, final clause: "the per-project `run.lock` / `acquire` for `drive` keeps its semantics"; and the 27-01 key link "`acquire(planning_dir)` becomes a wrapper over `acquire_at`" | Commit `9caf162` (`refactor(31-04)`) deleted `lock::{lock_path, acquire, read_holder}` and the hidden `drive` command (`2a23d9e`). `31-04-SUMMARY.md` records "`fn (stop_run\|lock_path\|acquire\|read_holder)` kill.rs 0 / lock.rs 0; `acquire_at`, `acquire_at_with_retry`, `probe_at`, `read_holder_at` stay". `src/driver/lock.rs:66-70` documents that the per-project `run.lock` is no longer written or read. RETIRE-01 is `[x]`. The `acquire_at` family that Phase 27 delivered is intact (`lock.rs:163,304,320,370`). |
| 27-04 acceptance count "`hide = true` at least 3 (Drive, Envelope, Driverctl)" | `Commands::Drive` is gone (`2a23d9e`). The hidden-command count is now 8 (`grep -c "hide = true" src/cli.rs`), so the `>= 3` bound still holds; `Driverctl` remains hidden. |

[inferred — cleanup 2026-10-28] These two items are marked retired rather than failed because Phase 31 (RETIRE-01) intentionally removed the legacy consumer, and the delivered `acquire_at` family and hidden `driverctl` command are the surviving contract.

### Required Artifacts

`gsd-tools verify.artifacts` on all six plans: 01 4/4, 02 4/4, 03 3/3, 04 6/6, 05 3/3, 06 4/4, no issues. Files are substantive (direction.rs 1523 lines, lock.rs 571, counters.rs 818, store.rs 2037, serve.rs 1729, protocol.rs 1047, driver_frontmatter.rs 1564, driverctl_cli.rs 694) and the ten `tests/driverctl_*.rs` binaries exist.

### Key Link Verification

`gsd-tools verify.key-links`: 01 2/2, 02 2/2, 03 2/2, 04 2/2, 05 4/4, 06 3/3, none unverified. (The 27-01 `acquire` wrapper link is the retired one; the tool matches its `acquire_at\(` pattern but it is counted as retired above.) Wiring also exercised live: `serve` took the run lock before binding (a second `serve` on the same project was refused with rc=3, "another run is already driving this project").

### Data-Flow Trace (Level 4)

Frontmatter values observed live came from real inputs: hook payloads (dispatch counts), the VERIFICATION.md on disk (gap rounds, `phase: "3"`), DIRECTION.md items, and the lock file. SIGTERM wrote `status: stopped`, `status_reason: "serve:signal"`. No static or hollow values.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Driverctl must-have suites | `rtk proxy cargo test --no-fail-fast --test driverctl_{client,frontmatter,liveness,lock_race,race,reconcile,replay,serve,stop_guard,store}` | 20 / 12 (+1 ignored) / 11 / 5 / 4 (+1 ignored) / 7 / 4 / 13 / 9 / 60 passed, 0 failed | PASS |
| Full workspace (orchestrator, same HEAD) | `rtk proxy cargo test --no-fail-fast` | 55 suites, 3183 passed, 0 failed, 17 ignored | PASS |
| Full editor race (orchestrator) | `--ignored editor_race_full` | 1 passed (30.01 s) | PASS |
| Live serve + log/hook through the binary | see truths 1-4 | rc 0 / 2 / 3 as specified; body byte-identical | PASS |
| Hook gate with unreachable socket | `driverctl hook` with an over-long socket path | Deny JSON `LAUNCHER CAP: driverctl unreachable ...`, rc 0 (fails closed) | PASS |

### Probe Execution

No probe scripts are declared by the phase (SKIPPED).

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| DCTL-01 | 27-04, 27-05, 27-06 | serve/log/hook/show is the single writer of state.json, LOG.md, frontmatter | SATISFIED | Truth 1; live run |
| DCTL-02 | 27-01 | Exchange-verify splice, body preserved, insane body never written, shared flock | SATISFIED | Truths 2, 3; race and lock-race suites |
| DCTL-03 | 27-01, 27-03, 27-04, 27-06 | Typed `DriverFrontmatter`; liveness from a non-blocking run-lock probe | SATISFIED | Truth 3; `driverctl_liveness` 11, SIGKILL e2e in `driverctl_serve` |
| DCTL-04 | 27-02, 27-04, 27-05, 27-06 | Tool-observed counters, rc=2 rejection, 120 s heartbeat coalescing | SATISFIED | Truth 4; live run; `driverctl_store` 60 |

All four IDs are declared in plan frontmatter, marked `[x]` in REQUIREMENTS.md and mapped to Phase 27 as Complete. No orphaned requirements.

### Anti-Patterns Found

`grep -E "TBD|FIXME|XXX|todo!|unimplemented!"` over the covered source files found nothing. No stubs.

### Code review findings (advisory, from 27-REVIEW.md; all `open`, none falsify a must-have)

0 critical, 6 warning, 10 info. Two are worth recording because they touch must-have wording:

- **WR-01 (reproduced live):** `driverctl log ... --outcome "- note"` is rejected by clap with rc 2 (`unexpected argument '- '`), where the argparse client accepted it. SC1 and plan D-04 are about the MASTER-PROMPT lines and "clap usage errors exit 2", both of which hold; no committed prompt template or fixture passes a hyphen-leading value (grep over `src/driver_templates` and `tests/fixtures/driverctl` found none). It is a latent parity gap for free text and should be fixed with `allow_hyphen_values`, but it is not a goal gap.
- **WR-05:** SIGTERM to `serve` after a driver `end --reason pause` rewrites `paused` to `stopped`. The ROADMAP status list names stopped, crashed, running and starting, and the frontmatter stays well-formed; the semantics are a refinement.
- WR-02, WR-03, WR-04, WR-06 are hardening of opens and the log directory; IN-06 notes stale comments that still name the deleted `drive`/goal layer (`src/cli.rs:530`, `src/state_reader/mod.rs:87,977`).

Security (27-SECURITY.md refreshed in 83cc518): SECURED, 27/27 threats closed, 0 open.

### Human Verification Required

None. Everything checkable was checked by execution.

### Gaps Summary

No gaps. All four ROADMAP success criteria hold at HEAD 1c6a229, all four requirement IDs are satisfied, and Phase 31's deletions removed only the legacy `drive` lock consumer, which is recorded above as `retired-by-31`.

---

_Verified: 2026-10-28_
_Verifier: Claude (gsd-verifier)_
