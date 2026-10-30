---
phase: 30-driver-tab-on-direction-md
reviewed: 2026-10-29T00:00:00Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - docs/DRIVER-SANDBOX.md
  - src/launcher/config_dir.rs
  - tests/launcher_config_dir.rs
  - docs/CONFIGURATION.md
  - docs/GSD-CORE-SYNC.md
  - src/agents/adapters/mod.rs
  - src/config.rs
  - src/executor/mod.rs
  - src/launcher/process.rs
  - src/launcher/run.rs
  - src/registry.rs
  - src/state_reader/config_json.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/mod.rs
findings:
  critical: 0
  warning: 3
  info: 4
  total: 7
new_findings:
  critical: 0
  warning: 1
  info: 2
status: issues_found
---

# Phase 30: Code Review Report (incremental re-review at HEAD)

**Reviewed:** 2026-10-29
**Depth:** standard
**Files Reviewed:** 14
**Status:** issues_found

## Summary

This review is the union of two passes. The first (the WR-13 fix, `ec8f8fb..HEAD` at the time) is
preserved verbatim below under "Prior review (a5ac2aa)". The second is an incremental pass over
`git diff a5ac2aa HEAD` for quick task 261029-1dp over the eleven delta files: `f4cfa01` (the SAFE-04
redacting capture pump, `src/launcher/process.rs` and `src/launcher/run.rs`) and `ce86206` (removal of
the dead `AgentRuntime` config key, `src/executor/runtime.rs` deleted, doc-comment corrections).

**Delta a5ac2aa..HEAD, what was traced.**

* The pump (`process.rs:96-209`, caller `run.rs:1629-1703`). stdout and stderr share one `O_CLOEXEC`
  pipe (`process.rs:105-114`), a single pump thread and one `File` that has no other writer
  (`launch-{n}.jsonl` is named only at `run.rs:1629` and read at `run.rs:1698`). `drop(cmd)`
  (`:119`) releases the parent's write ends so EOF is reachable. I checked the read loop at the cap
  boundary: a line of exactly `MAX_STREAM_LINE` bytes with its newline is kept, one without a newline
  within the cap goes through `discard_through_newline`, and EOF on an unterminated line is
  handled. A write error flips to drain-and-discard (`:168-170`). `redact_line` redacts a JSON line by
  tree walk and anything else as text, and `redact_value` rewrites keys as well as leaves
  (`journal/redact.rs:348-388`). The outcome reader's pre-filter (`outcome.rs:90`, a substring test
  for `"result"` / `rate_limit_event`) still matches after the compact re-serialisation, and the
  `AUTH_FAILURE_MARKERS` survive redaction. `cargo test --lib launcher::process` ran 11 passed, 0
  failed, including the end-to-end test that spawns `sh` with a token split across two writes on each
  stream. `serde_json` is built without `preserve_order` (`Cargo.toml:42`, `cargo tree -e features`
  shows only `default` and `std`), so re-serialised captured lines have sorted keys, which no reader
  here depends on.
* The `AgentRuntime` removal: `grep` finds no remaining reference to `AgentRuntime`,
  `UnrecognizedRuntime`, `executor::runtime` or `from_config_value` outside comments (`registry.rs:2363,2369`,
  `executor/mod.rs:13`, `config.rs:66`). The flattened `extra` maps (`config.rs:63-67`, `:513-516`) keep
  both keys through a load and save. The `registry.rs:958-961` claim that nothing launches on the
  opt-in record agrees with `docs/CONFIGURATION.md:143` and with the tests at `driver_confirm.rs:731,833`.
  `executor/mod.rs` says `stream_json` is "test-only today"; the only other users are inside
  `#[cfg(test)]` (`driver/rate_limit.rs:388`), and the cited todo
  `.planning/todos/pending/2026-10-28-retire-test-only-executor-remnants.md` exists.
* Already tracked under phase 29 and not re-raised here: multi-line PEM left unredacted by the line
  pump (29 WR-14), a pump-thread spawn failure leaving the child unsupervised (29 WR-15), an over-cap
  output line skipped by the reader (29 IN-07), the ignored capture-finish timeout (29 IN-08), the
  stale redaction-corpus assertion message (29 IN-09), the dangling `runtime()` doc reference (29 IN-10)
  and the unpinned per-project `runtime` round trip (29 IN-11). Interleaving and non-JSON-line
  handling were also left out, as instructed.

No critical defect in the delta. One new warning (the pump's launcher-death rationale is not true for
the degraded posture, and the new design drops the transcript when the launcher dies) and two new info
items (a stale test name, a stale `driver_max_concurrent` enforcement claim). The carried findings from
the prior pass (WR-14, WR-15, IN-18, IN-19) are unchanged: the sources they cite
(`src/launcher/config_dir.rs`, `tests/launcher_config_dir.rs`, `docs/DRIVER-SANDBOX.md`) are untouched
by a5ac2aa..HEAD.

### Prior review (a5ac2aa)

Incremental re-review of `git diff ec8f8fb HEAD` over the three scoped files (commits 601d20f, 1158139, 92dac7a, bfbd231), which fix WR-13. IN-10..IN-17 are still open and not re-raised.

**WR-13 verdict: VERIFIED FIXED on both paths.**

- **Managed seed path (`managed_settings`, config_dir.rs:682-712).**
  - Host `hooks` objects go through `well_formed_hooks` before `append_unique`. Every surviving event value is a non-empty array of well-formed matcher entries, so the old `or_insert_with(|| other.clone())` branch is gone.
  - Host `permissions` objects go through `well_formed_permissions`. The four `PERMISSION_LISTS` keys survive only as string-item arrays. Any other key survives only as a string or bool.
  - A host `deny` can therefore never arrive as a non-array, and a non-array `allow`, `ask` or `additionalDirectories` never reaches the file.
  - The existing IN-03 and WR-08 guards stay in place.
- **Degraded path (`settings_transform`, config_dir.rs:617-622).**
  - The same two filters run on `settings["hooks"]` and `settings["permissions"]` before `filter_user_hooks` and before the gate and deny list are added. This is one code path for both postures.
  - I traced the `wrong_shape_nested_host_docs()` corpus by hand against `settings_transform(json!({}), Some(gate))`. Every doc yields `hooks: {}`, then gate-only hooks, and `permissions: {}`, then deny-only. That equals the baseline, so the loop test `a_wrong_shape_nested_host_value_never_reaches_the_degraded_settings` is sound.
- **Ordering.** `well_formed_hooks` runs before `filter_user_hooks`, so the "entries that were empty before keep their place" rule still holds. `hooks: []` entries pass the shape check and are kept. The `return true` branch for entries without a `hooks` array in `filter_user_hooks` is now unreachable (see IN-19).
- **Tests.**
  - The unit tests are loop-driven over one shared corpus for both postures.
  - The new test `well_formed_host_values_still_merge_beside_wrong_shape_ones` pins that good entries still merge, in both postures.
  - The integration test `a_managed_seed_drops_wrong_shape_nested_host_values_and_keeps_the_gate` exercises the file plus a drop-in through `seed()`.
  - The doc edit in DRIVER-SANDBOX.md matches the code.

I found no critical issues and no regressions in the diff. Two warnings remain in the fix itself. One is a pre-existing inconsistency that the diff now codifies with a comment and a test. The other is a residual one nesting level deeper, of the same class WR-13 targets.

## Critical Issues

None.

## Warnings

### WR-16: the pump's launcher-death rationale does not hold for the degraded posture, and the new design stops capturing exactly when the unsupervised child matters most (new, 2026-10-29)

**File:** `src/launcher/process.rs:93-95`; context `src/launcher/run.rs:1599,1655,1679-1684`
**Issue:**
- The `[inferred]` note says that if the launcher dies, the child's next write fails and "the child is
  stopped by its group or scope". Only the sandboxed posture has a mechanism tying the child to the
  launcher: bwrap runs with `--die-with-parent` (`sandbox.rs:630`).
- In the degraded posture `claude` is spawned directly (`run.rs:1599`, `Containment::Group` at `:1655`).
  It is only a process-group leader, with no `PDEATHSIG` or other link to the launcher
  (`grep -rn "die-with-parent\|PDEATHSIG" src` finds bwrap only). Nothing signals that group when the
  launcher is killed, because the group kill at `run.rs:1682-1684` runs inside the launcher.
- Before f4cfa01 the child wrote straight into `launch-N.jsonl`. A SIGKILLed or crashed launcher left
  an unsupervised child, but the file kept recording what it did.
- Now the read end of the pipe closes with the launcher. The next write raises `EPIPE`. Whether the
  agent then exits depends on how it handles that error: Node ignores `SIGPIPE`, so an `EPIPE` on
  stdout is a swallowed or logged error, not a guaranteed exit. I did not verify what the current
  `claude` binary does.
- If the agent keeps running with write access to the project, its stream is dropped from that point on.
  The doc comment calls this acceptable on the strength of a containment that does not exist in this
  posture.

**Fix:** Correct the comment so it names the posture it is true for. For the degraded posture either
(a) keep an unredacted-write fallback out of scope and accept the loss explicitly, or (b) give the
group a link to the launcher, for example `PR_SET_PDEATHSIG(SIGKILL)` set from a `pre_exec` on the
launch child, so that "the launcher died" is also "the child died":
```rust
// in spawn_launch_child, Linux only, before spawn()
unsafe { cmd.pre_exec(|| { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL); Ok(()) }); }
```
Add a doc line saying that the capture ends with the launcher for both postures.

### WR-14: Scalar `permissions` settings use first-doc-wins while every other scalar uses last-doc-wins, contradicting drop-in precedence

**File:** `src/launcher/config_dir.rs:704-709`
**Issue:**
- For the string and bool `permissions` settings (`defaultMode`, `disableBypassPermissionsMode`, `blockReadsOutsideWorkingDirectories`), the merge does `ours.entry(key).or_insert(other)`. The first doc that sets the key wins.
- The catch-all arm (line 720) does `settings.insert(key, value)`, so for every other scalar the last doc wins.
- `host` is documented as "in precedence order: the file, then the drop-ins". As I recall, Claude Code lets later `managed-settings.d/*.json` files override earlier values; I did not re-verify this against current Claude Code docs.
- So an administrator who sets `defaultMode: "default"` in `managed-settings.json` and tightens it to `"plan"` in `managed-settings.d/10-x.json` gets the base file's value in the sandbox. This is the opposite of what runs on the host.
- The old code had the same `or_insert_with`, but only for non-array values. The diff now makes it the deliberate, commented behavior ("the first doc that sets it wins (I-6)"), and no test covers more than one doc setting the same scalar.

**Fix:** Make last-doc-wins the rule for scalar permission settings, as at top level, and add a two-doc test:
```rust
other => {
    ours.insert(key, other); // later doc (drop-in) overrides, like the catch-all
}
```
Plus a test with the file setting `defaultMode: "default"` and a drop-in setting `"plan"`, asserting `"plan"`. If first-wins is a deliberate safety choice, document why it differs from top-level scalars.

### WR-15: Hook-item validation stops at `type`, so `{"type":"command"}` or a non-string `command` still reaches the managed file beside the gate

**File:** `src/launcher/config_dir.rs:365-378`
**Issue:**
- `is_well_formed_hook_entry` accepts any hook item that is an object with a string `type`. `command`, `url`, `prompt` and `timeout` are explicitly not checked.
- A host entry such as `{"matcher":"Bash","hooks":[{"type":"command"}]}`, `{"type":"command","command":7}` or `{"type":"command","command":"x","timeout":"soon"}` therefore passes. It is merged into `hooks.PreToolUse` and so on in the managed file, next to the gate hooks.
- WR-13's own rationale is that "the safety tier must not depend on Claude Code refusing such a file". The residual is the same failure mode one level deeper. The risk is that Claude Code rejects or drops the block, and the gate and deny list go with it. I did not verify what Claude Code does.
- `command` is the one type that is stable across releases and is the only type `filter_user_hooks` ever keeps. The "varies by release" justification does not cover it.
- In the degraded posture `filter_user_hooks` drops such an item. Only the managed seed is exposed, so the two postures diverge on this input.

**Fix:** Require a string `command` when `type == "command"`:
```rust
.all(|hook| {
    let Some(Value::String(ty)) = hook.get("type") else { return false };
    ty != "command" || hook.get("command").is_some_and(Value::is_string)
})
```
Add `{"hooks":[{"type":"command"}]}` and `{"hooks":[{"type":"command","command":7}]}` to `wrong_shape_nested_host_docs()`. Note that for the degraded path the baseline equality already holds for these, so the test would pin the managed path.

## Info

### IN-20: test `opting_in_from_the_driver_tab_flips_the_spawn_seam` still names a seam that no longer exists (new, 2026-10-29)

**File:** `src/ui/screens/detail.rs:21271` (doc comment at `:21260-21269`)
**Issue:** ce86206 rewrote the doc comment and the first assertion message to say the opt-in record
"gates no launch" and "only shapes the push policy". The function name was left as
`opting_in_from_the_driver_tab_flips_the_spawn_seam`. Nothing in the test touches a spawn seam. It
asserts `registry::is_opted_in` before the keypress, after the keypress, and on the reloaded config. A
reader or `cargo test` output sees a claim the same commit just retracted in the comment above.
**Fix:** Rename to something that matches the body, for example
`opting_in_from_the_driver_tab_writes_the_opt_in_record`, and update any `.planning` reference that
quotes the old name.

### IN-21: `Preferences::driver_max_concurrent` still documents an enforcement site that is gone, next to docs that say it caps nothing (new, 2026-10-29)

**File:** `src/config.rs:497-499` (field at `:500-501`); contrast `docs/CONFIGURATION.md:84`
**Issue:** ce86206 reworded `docs/CONFIGURATION.md:84` to say `preferences.driver_max_concurrent` selects
nothing and caps nothing "since Phase 31", and rewrote the neighbouring `extra` doc in `config.rs`. The
field's own doc still says "The count is compared against live runs at the spawn seam in plan 17-05". The
only references to the field outside `config.rs` are comments (`envelope/policy.rs:183,284`). No code
compares it against anything, so the in-code claim contradicts both the user doc and the code. The
delta touched this file and the sentence next to it, so the cleanup is incomplete.
**Fix:** Replace the "Enforcement is not here" paragraph with: "Not enforced since Phase 31 (the retired
`drive` path was the only reader); kept so an old config round-trips." Keep the default-of-1 rationale only
if a future cap is planned, and say so.

### IN-18: Dropped host policy is silent

**File:** `src/launcher/config_dir.rs:384-424`
**Issue:**
- `well_formed_hooks` and `well_formed_permissions` discard host administrator policy (whole events, entries, rule items, unknown-shape permission keys) with no `tracing` event.
- The doc promises that host policy is "merged in, never hidden". An operator whose valid-but-unrecognised policy is dropped has no way to learn why the sandbox is more permissive than the host.
- `PERMISSION_LISTS` also treats every other `permissions` key as a string or bool setting. A future object-valued or array-valued key would be dropped for the same silent reason.

**Fix:** Emit one `tracing::warn!` per dropped item, naming the file, event or key, and reason. The merge loop could pass a label in, or return the dropped keys. Reword the "never hidden" claim to "never hidden unless malformed".

### IN-19: Dead branch in `filter_user_hooks`, and shadowed `key` in `managed_settings`

**File:** `src/launcher/config_dir.rs:556-558`, `src/launcher/config_dir.rs:701`
**Issue:**
- `filter_user_hooks` is private and only called after `well_formed_hooks`. Its `let Some(Value::Array(inner)) = entry.get_mut("hooks") else { return true; }` branch (and the `Value::Array` guard at line 552) can no longer fire, so it is now misleading dead code.
- In the `permissions` arm of `managed_settings`, `for (key, value) in well_formed_permissions(lists)` shadows the outer `(key, value)` of the `for (key, value) in doc` loop. This is harmless today but easy to misread next to the sibling arms.

**Fix:** Rename the inner binding to `(name, setting)`. Either keep the `filter_user_hooks` guards with a comment saying they are defensive, or tighten them with a `debug_assert!`.


---

_Reviewed: 2026-10-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
