---
phase: 39-check-actions-through-gsd
reviewed: 2026-11-17T00:00:00Z
depth: standard
files_reviewed: 18
files_reviewed_list:
  - README.md
  - src/app.rs
  - src/state_reader/owner_items/mod.rs
  - src/state_reader/owner_items/rules.rs
  - src/ui/glyphs.rs
  - src/ui/screens/ack_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/help.rs
  - src/ui/screens/inbox.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/owner_actions.rs
  - src/ui/screens/owner_block.rs
  - src/ui/screens/owner_settle.rs
  - src/ui/screens/render_escape_guard.rs
  - src/ui/screens/settle_popup.rs
  - tests/focus_entry_guard.rs
  - tests/owner_settle_guard.rs
  - tests/owner_settle_oracle.rs
findings:
  critical: 0
  warning: 2
  info: 3
  total: 5
status: issues_found
---

# Phase 39: Code Review Report

**Reviewed:** 2026-11-17
**Depth:** standard
**Files Reviewed:** 18
**Status:** issues_found

## Summary

Reviewed the phase 39 diff (`136a3e22..HEAD`) of the 18 files. The security-relevant core holds up:

- The command is a typed `RunCommand`; phase and workstream are re-validated with `is_command_token` / `is_workstream_name`.
- The prompt is ONE argv element, and every free-text field goes through `clean`.
- `clean` neutralizes `'`, `"`, `\`, control and invisible characters, and `--` runs, which is what verify-work's `sed | xargs` and `--ws` greps need.
- Verify-work reads `$ARGUMENTS` through a quoted heredoc, so `$` and backticks are inert there.
- Yes re-resolves the row and compares the prompt and cwd (`settle_recheck`).
- The settle modules contain no write or spawn API, and `tests/owner_settle_guard.rs` scans for that.

No critical issues. Two behavior defects in the live-session and cursor paths, and three smaller items. Nothing was built or run (read-only review; another agent works in the checkout).

## Warnings

### WR-01: Live-session settle copies the command and says "paste there" even when the focus failed

**File:** `src/ui/screens/settle_popup.rs:548-557`
**Issue:** `confirm()` takes the string from `launch_settle` (`focus_message`) and always builds `lead = "{focus} · paste the command there"`, then copies the prompt to the clipboard. `focus_message` returns a raw failure reason when focusing fails, for example "Tab-switching requires running inside tmux or herdr (none detected)", or a backend error. The status then reads "<failure> · paste the command there; Copied 1 line via wl-copy". The owner is told to paste into a session that was never focused, and the clipboard is overwritten anyway. The sibling `v` path avoids this with `owner_actions::focus_status`, which only appends the command to type when the message starts with `multiplexer::FOCUS_OK_PREFIX`. D-11 describes the success case only.
**Fix:**
```rust
let focused = focus.starts_with(crate::multiplexer::FOCUS_OK_PREFIX);
let lead = if focused {
    format!("{focus} \u{b7} paste the command there")
} else {
    format!("{focus} \u{b7} command copied; open the Claude session in {} and paste it", fresh.draft.target.alias)
};
```
Optionally skip the copy on failure and `Pop` with `status_message = focus`, the way `focus_status` does. Add a test with a `launch_settle` stand-in that returns a failure message.

### WR-02: Inbox `j` past the last item of the last row silently drops the item selection

**File:** `src/ui/screens/inbox.rs:1746-1778` (via `set_cursor` at ~416)
**Issue:** When the cursor is on the last item of an expanded row and that row is the last stop, `item_step` returns `Leave`. `step_items` then calls `self.step(ctx, 1)`, which calls `move_to` and then `set_cursor(to)` with `to` clamped to the same stop. `set_cursor` begins with `ctx.inbox_view.item_cursor = None`. The selection jumps from the last check back up to the row without moving to another stop. A following `x` then settles the whole phase instead of the one check the owner had selected. The popup does show the target, so nothing is launched blindly, but a key press that should be a no-op changes the selection. The Detail host does not do this on a bottom-edge press: `step_with_items` clears the item and then `move_cursor` returns `PastLast`, which leaves the block. The two hosts are inconsistent.
**Fix:** In `step_items`, when `Leave` is forward and the cursor is already on the last stop, return without touching the item cursor:
```rust
owner_block::ItemStep::Leave => {
    if forward && at + 1 >= rows.len() + usize::from(!fleet_folds(ctx).is_empty()) {
        return; // nothing below: keep the item selected
    }
}
```
Alternatively, save `item_cursor` before `self.step` and restore it when `cursor_index` did not change. A test would press `j` at the last item of the last expanded row and assert `item_cursor` is unchanged.

## Info

### IN-01: `move_instruction` panics on an inconsistent `SettleChoice`

**File:** `src/ui/screens/owner_settle.rs:424-427`
**Issue:** `.expect("a Move choice carries its target")` sits on a production render path (`settle_body` calls `settle_mux_argv`, which calls `prompt`). The invariant "`move_to` is `Some` exactly for `Outcome::Move`" is held only by convention: `SettleChoice` has public fields and `SettleReasonScreen::new` accepts any `Outcome`. A future caller that passes `Outcome::Move` with `move_to: None` would panic mid-frame and corrupt the terminal. The opposite mismatch (`WontDo` with `move_to: Some`) is silently ignored.
**Fix:** Make the type carry the invariant, for example `enum Outcome { WontDo, DeferPermanently, Move(MoveTarget) }`. Failing that, return a plain "Move needs a target phase" error from `instruction` instead of `expect`.

### IN-02: The `[x]settle` footer hint exists only on the widest inbox rung

**File:** `src/ui/screens/inbox.rs:1306-1315`
**Issue:** D-05 says the footer gains the `x` hint. The hint is kept on rung 1 only (137 cells and wider), a deliberate choice recorded in the code comment to keep the 100x30 golden. At 80 and 100 columns, the widths the 38 UAT targets, `x` is discoverable only through the help overlay and the README. The widest rung also moved from 126 to 137 cells, so terminals of 126-136 columns lost the long labels they had before.
**Fix:** Accept and document this in the phase VERIFICATION, or give `[x]` a one-character label on rung 2 (`[x]` only costs about 4 cells) and re-pin the golden.

### IN-03: The reason sentence is built twice

**File:** `src/ui/screens/owner_settle.rs:390-399` and `440-449`
**Issue:** The block that appends the owner's reason with a terminating period is copied verbatim in `instruction` and `move_instruction`. A later change to the punctuation rule must be made in both places, or the Move prompt will drift from the other two.
**Fix:** Extract `fn reason_sentence(reason: &str) -> String` and call it from both functions.

---

_Reviewed: 2026-11-17_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
