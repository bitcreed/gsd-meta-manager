---
phase: 27-driverctl-driver-frontmatter
verified: 2026-10-29T00:00:00Z
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
covered_digest: "v4:sha256:96b6397b52746922b28a88c0f83ef9dbad681aec53019f166ba780c3c74d9343"
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
**Verified:** 2026-10-29 at HEAD ef9d319
**Status:** passed
**Re-verification:** Yes. The 2026-10-28 report (HEAD 1c6a229) went stale because quick batch 261028-t3d (65233b8, 4977318) and the follow-ons 261028-t3e (7f4fc49, b42e57c), t3f and v2z changed covered files. The covered source files that moved since 1c6a229 are `src/driver/direction.rs`, `src/driver/lock.rs`, `src/driverctl/serve.rs`, `src/driverctl/store.rs` and `src/state_reader/driver_frontmatter.rs`; `src/launcher/{config_dir,run,stop}.rs` also moved but are outside the covered set. Every truth was re-checked against the code and the test suites at HEAD, not carried forward.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `driverctl serve\|log\|hook\|show` replaces `tools/driverctl.py`, same socket protocol; the spike-002 MASTER-PROMPT runs unchanged | VERIFIED | `tests/driverctl_client.rs` 20 passed (parses every MASTER-PROMPT line); `tests/driverctl_replay.rs` 4 passed (three recorded spike LOGs replayed through the Rust client and server); `tests/driverctl_serve.rs` 16 passed (was 13 before t3d added 3). `Commands::Driverctl` is still dispatched before tracing (`src/main.rs:56`). |
| 2 | Editor-race port: zero lost rename-saves, in-place loss at or below the spike rate; gsd-mm's own DIRECTION writes take the shared flock | VERIFIED | `tests/driverctl_race.rs` 4 passed (+1 ignored); the orchestrator ran `editor_race_full --ignored` at this HEAD: ok, in-place lost=0 (allowed 4 of 13783 saves), 30.0 s. `tests/driverctl_lock_race.rs` 5/5. `direction_lock` (`src/driver/direction.rs:321`) is the lock `splice_frontmatter` (`:411`) and `write_body` (`:440`) share; the exchange is `RenameFlags::EXCHANGE` (`:685`). The t3e change (`exchange_or_restore`, restore of a raced save by no-replace rename) was reviewed in 27-REVIEW.md and holds up (bounded `EEXIST` loop, stays inside `SPLICE_TRIES`). |
| 3 | Every generated frontmatter parses into a typed `DriverFrontmatter`; a body without `## Direction`/`## Items` is never written back | VERIFIED | `tests/driverctl_frontmatter.rs` 12 passed (+1 ignored golden regenerator); `body_sane` at `direction.rs:134`, with `write_body_refuses_an_insane_body` and `write_body_keeps_the_tool_block_verbatim` among the in-module tests. `tests/driverctl_liveness.rs` 11 passed. |
| 4 | Malformed events and unknown items rc=2; dispatches from hook payloads; gap rounds from `gaps_found` VERIFICATION.md on disk coalesced per 26.1; heartbeat rewrites coalesced to 120 s | VERIFIED | `COALESCE_S = 45.0` with strict `<` (`counters.rs:22,99`); `HEARTBEAT_REGEN_MIN_S = 120` (`store.rs:72`, applied at `:505`). `tests/driverctl_store.rs` 61 passed (was 60; t3d added one), `tests/driverctl_reconcile.rs` 7, `tests/driverctl_stop_guard.rs` 9, and the serve e2e suite cover rejection rc=2, hook dispatches, scan-observed gap rounds and SIGKILL crash detection. |

**Score:** 4/4 truths verified, 0 behavior-unverified. The two `retired-by-31` plan-level items below are excluded from the denominator; they were never roadmap success criteria.

### Retired by Phase 31 (excluded from the denominator, not gaps)

| Plan must-have | Evidence it was retired |
|----------------|-------------------------|
| 27-01 truth D-20, final clause: "the per-project `run.lock` / `acquire` for `drive` keeps its semantics"; and the 27-01 key link "`acquire(planning_dir)` becomes a wrapper over `acquire_at`" | `lock::{lock_path, acquire, read_holder}` and the hidden `drive` command were deleted in Phase 31 (RETIRE-01, `[x]`). The `acquire_at` family Phase 27 delivered is intact and now also hardened by t3d (`open_lock_file` shared by `acquire_at`, `probe_at`, `read_holder_at`). |
| 27-04 acceptance count "`hide = true` at least 3 (Drive, Envelope, Driverctl)" | `Commands::Drive` is gone. `grep -c "hide = true" src/cli.rs` is 8, so the `>= 3` bound holds; `Driverctl` remains hidden. |

[inferred — cleanup 2026-10-28] These two items are retired rather than failed because Phase 31 intentionally removed the legacy consumer; the delivered `acquire_at` family and the hidden `driverctl` command are the surviving contract.

### Delta since the prior report (1c6a229 to ef9d319)

| Change | Effect on the must-haves |
|--------|--------------------------|
| t3d (65233b8, 4977318): `ensure_lock_file` (`O_NOFOLLOW`/`O_NONBLOCK`, regular-file and link-count checks) for `DIRECTION.lock`; capped guarded read of `state.json`; unguarded `launcher.lock`/`DIRECTION.lock` opens in the probe and holder read closed (WR-03, WR-04 fixed) | Hardens the single-writer lock (truths 1, 2). New tests pass (serve 16, store 61, lock-module tests). No regression. |
| t3e (7f4fc49, b42e57c): `exchange_or_restore` put-back helper in `direction.rs` | Truth 2 intact; race suites and the full `editor_race_full` run pass at HEAD. |
| t3f, v2z: `src/launcher/config_dir.rs` host hooks/permissions shape guards | Outside the covered set; `tests/launcher_config_dir.rs` is part of the 3211-pass full suite. |

### Required Artifacts

`gsd-tools verify.artifacts` on all six plans at HEAD: 01 4/4, 02 4/4, 03 3/3, 04 6/6, 05 3/3, 06 4/4, no issues. The ten `tests/driverctl_*.rs` binaries exist and ran.

### Key Link Verification

`gsd-tools verify.key-links` at HEAD: 01 2/2, 02 2/2, 03 2/2, 04 2/2, 05 4/4, 06 3/3, none unverified (the 27-01 `acquire` wrapper link is the retired one, see above).

### Data-Flow Trace (Level 4)

Unchanged in kind from the prior live run: dispatch counts come from hook payloads, gap rounds from the VERIFICATION.md on disk, items from DIRECTION.md, liveness from the run-lock probe. The serve e2e tests (SIGKILL crash, SIGTERM stop, scan-observed gap) exercise those flows against the real binary. No static or hollow values were found in the changed code (`write_state`, `open_with_clock`, `ensure_lock_file` all operate on real files).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Driverctl must-have suites | `rtk proxy cargo test --no-fail-fast --test driverctl_{client,frontmatter,liveness,lock_race,race,reconcile,replay,serve,stop_guard,store}` | 20 / 12 (+1 ignored) / 11 / 5 / 4 (+1 ignored) / 7 / 4 / 16 / 9 / 61 passed, 0 failed | PASS |
| Full workspace (orchestrator, same HEAD) | `rtk proxy cargo test --no-fail-fast` | 3211 passed, 0 failed, 17 ignored, exit 0 | PASS |
| Full editor race (orchestrator) | `--ignored editor_race_full` | ok, in-place lost=0 (4 allowed of 13783), 30.0 s | PASS |

### Probe Execution

No probe scripts are declared by the phase (SKIPPED).

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| DCTL-01 | 27-04, 27-05, 27-06 | serve/log/hook/show is the single writer of state.json, LOG.md, frontmatter | SATISFIED | Truth 1 |
| DCTL-02 | 27-01 | Exchange-verify splice, body preserved, insane body never written, shared flock | SATISFIED | Truths 2, 3 |
| DCTL-03 | 27-01, 27-03, 27-04, 27-06 | Typed `DriverFrontmatter`; liveness from a non-blocking run-lock probe | SATISFIED | Truth 3; `driverctl_liveness` 11 |
| DCTL-04 | 27-02, 27-04, 27-05, 27-06 | Tool-observed counters, rc=2 rejection, 120 s heartbeat coalescing | SATISFIED | Truth 4 |

All four IDs are declared in plan frontmatter and marked `[x]` / "Complete" in REQUIREMENTS.md (lines 146-158, 381-384). No orphaned requirements.

### Anti-Patterns Found

`grep -E "TBD|FIXME|XXX|todo!|unimplemented!"` over the covered source files matches only two doc-comment mentions of `\uXXXX` escapes in `driver_frontmatter.rs` (lines 60, 504). Not debt markers. No stubs.

### Code review findings (advisory, from 27-REVIEW.md at HEAD; dispositions in 27-REVIEW-DISPOSITION.md)

0 critical, 6 warning (WR-01, 02, 05, 06, 10, 11), 10 info. WR-03 and WR-04 are confirmed fixed. None falsifies a must-have; the two new ones were read in the code:

- **WR-10:** `log::append` (`src/driverctl/log.rs:91-101`) opens `LOG.md` with append+create and no `O_NOFOLLOW`/`O_NONBLOCK`. That is a hardening gap against a hostile run directory (same class as the fixed WR-03). SC1-SC4 concern correctness for the driver the tool serves, and the threat model is closed 27/27 in 27-SECURITY.md; this does not break a success criterion.
- **WR-11:** `read_run_file_capped` (1 MiB, `store.rs:650`) has no matching bound in `write_state` (`store.rs:742-763`). A run whose `state.json` exceeds 1 MiB (tens of thousands of `FINDINGS` reports or thousands of items) would be written fine and then fail to reopen as `CorruptState`. This is a real latent regression risk introduced by the t3d read cap, but it needs a state size far outside any phase-bounded run, and it does not affect the four success criteria. Left as an advisory; the right fix is a write-side cap or a findings cap. [inferred — cleanup 2026-10-28]
- **WR-01** (clap rejects hyphen-leading free text, reproduced earlier) and **WR-05** (SIGTERM rewrites `paused` to `stopped`) remain open and unchanged; both are refinements of already-met criteria.

Security (27-SECURITY.md refreshed in 5859eb0 at HEAD 4c7a46a): verified, threats_open 0, 27/27 closed.

### Human Verification Required

None. Everything checkable was checked by execution.

### Gaps Summary

No gaps. All four ROADMAP success criteria hold at HEAD ef9d319, all four requirement IDs are satisfied, and the post-1c6a229 changes are hardening and a race-restore refinement that the existing and new tests pin. WR-10 and WR-11 are recorded as open advisories, not gaps.

---

_Verified: 2026-10-29_
_Verifier: Claude (gsd-verifier)_
