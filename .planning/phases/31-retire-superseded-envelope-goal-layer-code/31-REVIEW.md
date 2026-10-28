---
phase: 31-retire-superseded-envelope-goal-layer-code
reviewed: 2026-10-28T00:00:00Z
depth: standard
files_reviewed: 23
files_reviewed_list:
  - Cargo.lock
  - Cargo.toml
  - docs/TESTING.md
  - src/app.rs
  - src/config.rs
  - src/error.rs
  - src/executor/mod.rs
  - src/executor/runtime.rs
  - src/journal/redact.rs
  - src/main.rs
  - src/main_loop.rs
  - src/registry.rs
  - src/session_detector.rs
  - src/state_reader/disk_status.rs
  - src/state_reader/queue_md.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - tests/async_blocking_guard.rs
  - tests/driver_router_conformance.rs
  - tests/spawn_seam_guard.rs
findings:
  critical: 0
  warning: 2
  info: 6
  total: 8
status: issues_found
---

# Phase 31: Code Review Report (incremental, plan 31-07)

_Note: this report overwrites the earlier one; the prior report (plans 31-01..31-06/08/09) is at git f0eb8b8._

**Reviewed:** 2026-10-28
**Depth:** standard
**Files Reviewed:** 23
**Status:** issues_found

## Summary

Scope: `git diff f0eb8b8..HEAD` (commits a23d1d4, 2e2e335) plus surrounding context. The removal itself is clean. I found no broken build edge, no behaviour regression and no security regression introduced by the deletion:

- No Rust identifier, module path or fixture name from the deleted files (`ClaudeExecutor`, `CodexExecutor`, `AgentExecutor`, `ExecutionHandle`, `DrivableProject`, `ExecEvent`, `exec_tx/exec_rx`, `run_states`, `SpawnError/SendError/CapabilityError/OptInError`, `fake-claude*`, `fake-codex`, `executor_lifecycle`, `executor_transport`) is still used as code. The remaining hits are comments, doc strings and assertion messages (below).
- `process-wrap` is gone from `Cargo.toml` and `Cargo.lock`. `uuid` has no `uuid::` user left in `src/` or `tests/`; the remaining `uuid` hits are fields on the stream-json wire model (`Option<String>`) and prose. It stays in the lock transitively through `wezterm-blob-leases`, as the manifest comment says. The lock diff is deletions only (process-wrap, the `windows` 0.62 family, uuid's direct edge); `nix`, `indexmap`, `futures`, `tracing` stay because other crates or direct deps still use them. Nothing relied on a feature that only `process-wrap` or `uuid` enabled: `tokio` is `full`, and `rustix` features are declared directly.
- `pump` is correct with two arms. A closed action channel disables arm 1 and arm 2 answers, so the `else` branch and `Idle` stay unreachable in practice. The three replacement tests assert real outcomes (`Action`, `RedrawTick`) and are not vacuous.
- The guard tests that remain still earn their keep. The bidirectional vanished checks, `BLOCKING_HELPERS` (the `RunSnapshot::capture(` removal is needed, since `every_named_blocking_helper_still_exists_in_the_tree` would reject it) and `MIN_ASYNC_BODY_LINES` (800 against a measured 1229) all hold. The non-vacuity control in `async_blocking_guard.rs` was retargeted from `RunSnapshot::capture(` to `scan_worktree(`, which is still a marker.
- Deleting the Codex bypass-flag scan and the MCP-set guard was authorised by the plan (31-07-PLAN.md lines 193 and 227), so it is not raised as a defect. See IN-06 for the one residual note.
- The summary's "no ignored test lived in the deleted binaries" is correct. The old TESTING.md claim about an ignored escalation test was already stale before this plan.

What remains is stale or vacuous residue the deletion left behind: one test that now guards nothing, and a set of comments and assertion messages that state facts about the removed executor as if it were still part of the build.

## Warnings

### WR-01: `DRIVER_TEARDOWN_GRACE` lower-bound guard now compares against a mirror of a deleted constant

**File:** `src/driver/kill.rs:31`, `src/driver/kill.rs:92-128` (not in the diff; this plan's deletion orphaned it)
**Issue:** `the_driver_grace_exceeds_the_claude_group_grace_plus_slack` asserts `DRIVER_TEARDOWN_GRACE > CLAUDE_GROUP_GRACE` and `>= CLAUDE_GROUP_GRACE + 2s`. `CLAUDE_GROUP_GRACE` is a literal `Duration::from_secs(10)` in the test module, documented as an "observable copy of a constant that is private to the executor (`src/executor/claude.rs:107`)". That file and constant no longer exist, so the mirror has no referent and can never move. The test is now two assertions of `12 > 10` and `12 >= 12`. The doc comment on `DRIVER_TEARDOWN_GRACE` (line 31-36) still gives as its reason that "the driver spends that entire grace inside `Executor::cancel`", and that test message repeats it. Nothing in the surviving launcher spends a 10 s grace, so the invariant that justified the 12 s value is gone and a future edit to the figure is no longer constrained by anything real. The sibling upper-bound test (strictly below the 15 s verification point) still has a real basis. The 12 s is consumed by `launcher::run::stop_group` (`src/launcher/run.rs:580`).
**Fix:** Either retire the lower-bound test and rewrite the `DRIVER_TEARDOWN_GRACE` doc to give only the surviving bound and the real reason for the floor (the launcher's own SIGTERM wait), or repoint it at a value that exists, such as `TimeoutStopSec`/the launcher unit's stop timeout in `src/launcher/run.rs:116`:
```rust
// delete CLAUDE_GROUP_GRACE and the first test, then reword the doc:
//   "It must be strictly less than fifteen seconds ... (the only remaining bound)."
```
`[inferred — audit]` This is a warning rather than info because a green test is asserting a rationale that is no longer true.

### WR-02: Comments and assertion messages in `session_detector.rs` still say this build's own executor emits `--session-id` and `codex exec`

**File:** `src/session_detector.rs:96`, `:503-506`, `:565-568`, `:1547-1550`, `:1567-1570`
**Issue:** After 31-07 no code in this build emits `claude -p --session-id <uuid>` or runs `codex exec --json`. The commit updated three nearby doc comments (lines 390, 470, 1718, 1894) but left these statements:
- line 96: "`e` is `exec`'s visible alias. This build's own executor runs `codex exec --json …`."
- lines 503-506 and 1570: "the exact argv this build's executor emits when resuming, `--session-id <fresh-uuid>` followed by `--resume <old>`".
- lines 565-568 and 1547-1550: "`--session-id <uuid>` is what THIS BUILD's own executor emits on every driver-launched run ... made every session this build itself started invisible".

Lines 1547 and 1570 are assertion failure messages, so a future red would send the reader chasing a producer that does not exist. The rank rule (`--resume` outranks `--session-id`) is still sound as a parser rule for externally launched sessions, but the justification as "the argv this build emits" is false, and the round-trip control that backed it (`the_executors_own_argv_is_an_argv_this_build_can_read_back`) was deleted in this plan. Note too that the claim at 1550 ("the population DRIVE-01 is about") no longer has a measured producer in this tree; the sandboxed launcher's argv does not carry a `--session-id` literal.
**Fix:** Reword each to the parser's actual basis: "`claude` documents `--session-id <uuid>` (2.1.250); the retired in-process executor used to emit it and externally launched sessions still do". Drop the "this build emits" framing from the two assertion messages, for example:
```rust
"`--session-id <uuid>` is a documented `claude` option; an externally launched \
 session carrying it must still read back its id."
```
and at line 96 delete the second sentence.

## Info

### IN-01: Comments still cite deleted files and line numbers

**File:** `src/driver/rate_limit.rs:18` (`src/executor/outcome.rs:9-12`), `src/state_reader/git_ops.rs:98` and `:166` (`src/executor/outcome.rs`, `:60`), `src/launcher/process.rs:42` (`src/executor/claude.rs`), `src/driver/kill.rs:31`/`:92` (`src/executor/claude.rs:107`), `src/driver/liveness.rs:245` and `src/driver/kill.rs:94` (`tests/executor_lifecycle.rs`), `src/executor/stream_json.rs:151`, `:354`, `:978` (`ExecutionEvent::Unparseable`, `derive_run_outcome_from_envelopes`), `src/journal/redact.rs:396` (`ExecutionEvent::LineTruncated`), `src/driver/mod.rs:6`, `src/agents/adapters/mod.rs:15`, `src/config.rs:413`
**Issue:** Each is a path, line number or type name from a file this plan deleted. Several sit in files the plan did touch for the same kind of drift (`queue_md.rs`, `disk_status.rs`, `redact.rs` were fixed), so the sweep was partial.
**Fix:** Reword each to past tense or drop the citation. In `git_ops.rs:98` the claimed consumer ("the run snapshot") no longer exists; see IN-04.

### IN-02: Broken intra-doc link to a deleted constant

**File:** `src/ui/screens/mod.rs:386`
**Issue:** ``[`crate::main_loop::EXEC_BATCH`]`` is an intra-doc link, and `EXEC_BATCH` was deleted from `main_loop.rs` in this plan. `cargo doc` emits a `rustdoc::broken_intra_doc_links` warning. Neither the build nor the clippy gate runs rustdoc, so it passed.
**Fix:** Drop the link and keep the sentence: "2000 is a defensible starting value with no tuning data behind it."

### IN-03: Spawn-seam guard still names the process-wrap marker and a capability type that no longer exist

**File:** `tests/spawn_seam_guard.rs:226` (assertion message), `:67-69` (`SPAWN_MARKERS`)
**Issue:** `SPAWN_MARKERS` still lists `"CommandWrap::with_new("`, a `process-wrap` API. The crate is gone, so the marker can never match and only widens the list. The failure message tells the next author to "Confirm it takes a capability type", but `DrivableProject`, the capability type, was deleted in this plan; the instruction cannot be followed.
**Fix:** Remove the `CommandWrap::with_new(` marker (keeping `Command::new(` and `process_group(`) and reword the message to the real rule: "Confirm the file is a deliberate spawn site and add it with a one-line reason."

### IN-04: Production-dead code is retained, and the module doc overstates its use

**File:** `src/executor/mod.rs:8-14`, `src/executor/stream_json.rs`, `src/executor/runtime.rs` (`resolve`), `src/state_reader/git_ops.rs:107` (`head_sha`)
**Issue:** `executor/mod.rs` says each remaining submodule "has a live user", and 31-07-SUMMARY repeats it. For `stream_json` the only user outside its own file is `src/driver/rate_limit.rs:388`, which is inside `#[cfg(test)] mod tests`. The roughly 1000-line wire model (`UserMessage`, `ControlRequest` builders, `parse_line`) therefore has no production caller; the mod doc's "the rate-limit fixtures parse transcripts through" describes test code. In the same way `AgentRuntime::resolve` has no caller (config only validates the value through `from_config_value`), and `head_sha`'s only remaining callers are its own tests now that the run snapshot is gone. All are `pub` items in a library crate, so no dead-code lint fires. The SUMMARY records some of this under "Deferred Issues" but not these.
**Fix:** Either reword the module doc honestly ("test-only parser kept for the rate-limit transcript fixtures") and move `stream_json` behind `#[cfg(test)]` or relocate it next to `rate_limit`, or log it as a follow-up retirement candidate alongside `check_prompt_input_drift`. `[inferred — audit]` Not a defect in 31-07's scope, since the plan chose to keep these.

### IN-05: Retargeted app tests keep "driver maps" names but now assert a non-driver map

**File:** `src/app.rs:4733-4753` (`removing_a_project_interactively_clears_its_driver_maps`), `src/app.rs:4769` (`every_per_alias_driver_map_is_pruned`)
**Issue:** The `run_states` assertions were replaced with `last_refresh`. `last_refresh` is cleared by `do_remove_project` (`delete_confirm.rs:173`) independently of any driver feature and was already asserted in the second test, so the first test now adds no driver-specific coverage and `every_per_alias_driver_map_is_pruned` no longer lists a map that is "per-alias driver" state. The wildcard-free destructure still forces a decision on any new `AppContext` field, which is the part of the guard that matters.
**Fix:** Rename both tests (for example `..._clears_its_per_alias_maps`) so the name matches what they assert, or drop the first test's `last_refresh` seeding if the removal is already covered by `delete_confirm.rs` tests.

### IN-06: Src-wide negative guards for Codex bypass flags and MCP config went with their subjects

**File:** `tests/spawn_seam_guard.rs` (deleted `no_executable_line_in_src_carries_a_codex_bypass_flag_or_the_full_access_sandbox`, `the_permitted_mcp_set_stays_empty_on_both_profiles`)
**Issue:** The plan authorised both deletions (31-07-PLAN.md lines 193 and 227), and the executor argv builders they exercised are gone, so this is not a regression. For the record: two of the deleted checks were scans over all of `src/` (no executable line carries a Codex permission-bypass flag or `danger-full-access`; no line supplies `--mcp-config`). Those would still have caught a resurrected Codex path or an MCP widening in the sandboxed launcher. Repo-wide grep now shows the launcher's `claude` argv builder (`src/launcher/sandbox.rs:720`) uses `--setting-sources user` and `--permission-mode` but no `--strict-mcp-config`, so after this plan no test pins the MCP posture of any spawn path. That predates 31-07 for the launcher, but 31-07 removed the last test that stated the posture at all.
**Fix:** If the launcher's MCP posture matters (it runs `--permission-mode bypassPermissions` inside bwrap, per `tests/launcher_e2e.rs:458`), add one launcher-level assertion in `tests/launcher_sandbox.rs` rather than reviving the deleted src scan. `[inferred — audit]` Raising this as a coverage note only; no action is required for 31-07 to close.

---

_Reviewed: 2026-10-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
