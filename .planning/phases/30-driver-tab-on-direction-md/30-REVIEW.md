---
phase: 30-driver-tab-on-direction-md
reviewed: 2026-10-29T00:00:00Z
depth: standard
files_reviewed: 3
files_reviewed_list:
  - docs/DRIVER-SANDBOX.md
  - src/launcher/config_dir.rs
  - tests/launcher_config_dir.rs
findings:
  critical: 0
  warning: 2
  info: 2
  total: 4
status: issues_found
---

# Phase 30: Code Review Report (incremental, quick task 261028-v2z)

**Reviewed:** 2026-10-29
**Depth:** standard
**Files Reviewed:** 3
**Status:** issues_found

## Summary

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

## Warnings

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
