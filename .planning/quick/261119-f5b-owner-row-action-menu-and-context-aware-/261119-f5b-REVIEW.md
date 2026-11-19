---
phase: quick-261119-f5b
reviewed: 2026-11-19T00:00:00Z
depth: quick
files_reviewed: 13
files_reviewed_list:
  - README.md
  - docs/GETTING-STARTED.md
  - src/ui/screens/detail.rs
  - src/ui/screens/help.rs
  - src/ui/screens/inbox.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/owner_actions.rs
  - src/ui/screens/owner_block.rs
  - src/ui/screens/owner_row_actions.rs
  - src/ui/screens/owner_settle.rs
  - src/ui/screens/render_escape_guard.rs
  - src/ui/screens/settle_pending.rs
  - src/ui/screens/settle_popup.rs
findings:
  critical: 0
  warning: 4
  info: 4
  total: 8
  necessity: 1
status: fixed
---

# Phase quick-261119-f5b: Code Review Report

**Reviewed:** 2026-11-19
**Depth:** quick (diff 237ca4d9..HEAD, changed code read in context; `cargo clippy --lib -D warnings` is clean)
**Files Reviewed:** 13
**Status:** issues_found

## Summary

The shared action model, the footer and the menu are coherent. The menu re-dispatches through the host's `handle_key` with the menu already taken, so there is no recursion hazard. Test hooks (`launch_real` recording, `RUN_LAUNCHES`, `take_run_launches`, `fixture`, `with_owner_menu_open`) are all `cfg(test)`-gated and do not reach the release build. Refusal texts for `o`, `A` (lane/kind), `x` (lane/scope) and `v` are shared, so those hints cannot drift from the keys.

The remaining problems are: the footer/menu still list `x` and `A` as ready where the key will refuse (the model is pure only for some refusals), a menu whose selected entry disappears turns Enter into a silent no-op, the menu has no viewport when the frame is short, and a dropped menu is never cleared. No security issue was found; every project-authored string in the menu goes through `shown()`.

## Warnings

### WR-01: `x` and `A` are listed Ready although their pure refusals are not modeled (footer truthfulness)

**Status:** fixed (ee78c69f)

**File:** `src/ui/screens/owner_row_actions.rs:222-262` (settle), `:289-312` (acknowledge)
**Issue:** `v` mirrors every pure `run_plan` refusal, including the unsafe phase token and workstream name. `x` and `A` do not: they stop after the lane/scope/kind rule.
- `settle_target_for` (`owner_settle.rs:322-343`) also refuses an unsafe workstream name, an unsafe phase token, and a group with no checks ("Row is no longer listed"). All of these are pure over the digest and `ProjectState`, which `ctx` carries.
- `ack_target_for` (`owner_actions.rs:519-610`) also refuses an unsafe workstream, milestone, file name or phase token, and "no known milestone version". These are pure over `ctx.project_states` too.

For such a row the footer shows `[x]settle` / `[A]ck`, and the menu shows the entry as ready (`whole phase, asks first`, `FILE via gsd-core, asks first`). The key then prints "Refused: ...". That is the "listed key does nothing" case the plan set out to remove. The module doc admits it only for I/O-bound refusals (missing launcher, pointer file), and these are not I/O-bound. The one remaining I/O refusal (`active-workstream` pointer) is correctly left out.
**Fix:** Run the pure part of both resolvers inside `row_actions` and map `Err(message)` to `Availability::Blocked(message)`. Call `settle_target_for` with `active_workstream_set = false` and `ack_target_for` with `false` (drop their only I/O input), then add a test for an unsafe token and a missing milestone, as exists for `v`.

### WR-02: Enter in the menu is a silent no-op when the selected entry vanished

**Status:** fixed (ee78c69f, dd15c1a2)

**File:** `src/ui/screens/owner_row_actions.rs:567-570` (`KeyCode::Enter` arm), `:633-650` (`menu_lines` marker)
**Issue:** Entries are recomputed per key, but `selected: ActionId` is not reconciled. If a refresh removes the selected action (for example `Run` disappears when the group turns BLOCKED, or `Acknowledge` when the file becomes acknowledged), `actions.iter().find(|a| a.id == self.selected)` returns `None`. Enter then returns `MenuKey::Stay`, and `menu_lines` draws no `>` marker. The menu looks frozen. `step` has a recovery branch for this state, but Enter and the render do not use it. The `Enter` arm and the `step` doc comment ("a selection that is no longer among them ... lands on the nearest end") describe different behavior.
**Fix:** Reconcile the selection once at the top of `key()` and in the render path, for example:
```rust
fn reconcile(&mut self, actions: &[RowAction]) {
    if !actions.iter().any(|a| a.id == self.selected) {
        if let Some(a) = actions.iter().find(|a| a.is_ready()).or(actions.first()) {
            self.selected = a.id;
        }
    }
}
```
Call it before matching `code`. Do the same in the hosts' `render_menu`, or pass `selected` through the helper so the marker always shows.

### WR-03: Menu is clipped without a viewport on short frames

**Status:** fixed (ee78c69f)

**File:** `src/ui/screens/owner_row_actions.rs:700-705`
**Issue:** `height = (lines.len() + 2).min(area.height)`. Blocked reasons wrap onto several lines (`menu_lines`), so a row with three blocked entries needs 12 or more lines. On a short terminal (or the 80x24 inbox once the anchor leaves little room) the `Paragraph` is clipped at the bottom with no scroll. The selected entry, which `j`/`k` can move below the clip, becomes invisible, and Enter still runs it. The same applies to the `[Enter]run  [Esc]close` hint, which is drawn on the box's bottom border.
**Fix:** When `lines.len() + 2 > area.height`, scroll the line window so the selected entry's first line stays visible (an offset derived from the selected index), or truncate blocked reasons to one line once the menu does not fit.

### WR-04: A dropped menu is left in `self.menu` and can reappear

**Status:** fixed (dd15c1a2)

**File:** `src/ui/screens/inbox.rs:1689-1702` (render), `:2254-2266` (`handle_key`); `src/ui/screens/detail.rs` (`owner_menu_actions`, render tail)
**Issue:** When the row vanishes, the cursor moves to another row, or the Owner block loses focus, `menu_actions` returns `None`. Render then draws nothing, but `self.menu` stays `Some` until the next key. If a later refresh brings the same row back under the cursor (or focus returns to the Owner block), the old menu draws again with a stale selection. In addition, the first key after the drop acts as a normal hotkey on the new cursor row. For example, `v` run through a menu the user last saw before the refresh runs a different row's command. The documented behavior ("a menu whose row vanished is dropped and the key acts normally") is the safe choice, but the drop only happens on a key, not when the data changes.
**Fix:** Clear the menu in `render` (take `&mut` through a `Cell<bool>`/`RefCell` flag) or on the host's refresh path (`reanchor`, the detail's refresh hook) as soon as `menu_actions` is `None`. Alternatively, store the menu in a `RefCell` and drop it in render.

## Info

### IN-01: `fold_line_actions` ignores its `host` parameter

**Status:** fixed (ee78c69f)

**File:** `src/ui/screens/owner_row_actions.rs:340`
**Issue:** `_host: Host` is unused, yet the doc comment says "the inbox's the fleet's, the Owner block's this project's". That difference lives in `owner_copy`/`copy_key`, not here. The parameter adds a false impression of host-specific behavior.
**Fix:** Drop the parameter, or reword the doc to say the host split is applied by the copy path.

### IN-02: `OWNER_BLOCK_ONLY` is an empty list with its own mechanism

**Status:** kept as documented exception contract (no change)

**File:** `src/ui/screens/owner_row_actions.rs:61-63`, `:324-329`
**Issue:** **Category:** necessity. `OWNER_BLOCK_ONLY` is empty and `offered` filters against it, so `!OWNER_BLOCK_ONLY.contains(&id)` is always true. The plan states "OWNER_BLOCK_ONLY is empty". This is generality without an instance (the repo has exactly one screen-only action, `GoToProject`). The test that asserts the screen difference equals the two lists is still useful. The empty list and its filter are not needed to enforce anything today.
**Fix:** Keep the constant only when a second screen-only action exists, or leave it and note it as intentional scaffolding; it is cheap, so this is a judgment call.

### IN-03: Footer doc comment disagrees with the code on where Space shows

**Status:** fixed (ee78c69f)

**File:** `src/ui/screens/owner_row_actions.rs:386-388` (doc on `footer_spans`) vs `:413-418`
**Issue:** The doc says "Space on a check or file line (2)". The code lists the Toggle action whenever `enter` is `Actions` or `View`, so Space also shows on a plain group row. The README text ("the movement and `Space` hints first") matches the code.
**Fix:** Reword the doc to "Space on a group row, check or file line".

### IN-04: Expand/collapse logic is duplicated between the hosts

**Status:** left: helpers act on different state types

**File:** `src/ui/screens/inbox.rs:1765-1784` (`expand_key`) and `src/ui/screens/owner_block.rs:2056-2070` (`set_expanded`)
**Issue:** Both implement "set expanded to X, clear the sub-stop, change the set". They differ only in the `follow` scroll call. A future change to one (for example, what happens to the sub-stop) can silently diverge from the other.
**Fix:** Have the inbox call a shared helper over `(expanded, sub_stop-setter)`, or keep the duplication and add a test that both stay in step.

---

_Reviewed: 2026-11-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: quick_
