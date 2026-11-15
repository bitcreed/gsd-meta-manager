---
phase: 37-inbox-screen-and-actions
fixed_at: 2026-11-15T00:00:00Z
review_path: .planning/phases/37-inbox-screen-and-actions/37-REVIEW.md
iteration: 1
findings_in_scope: 12
fixed: 11
skipped: 1
status: partial
---

# Phase 37: Code Review Fix Report

**Fixed at:** 2026-11-15
**Source review:** .planning/phases/37-inbox-screen-and-actions/37-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 12 (fix_scope `all`: 6 warnings, 6 info, no necessity findings)
- Fixed: 11 (WR-01 to WR-06, IN-01, IN-03, IN-04, IN-05 in part, IN-06; IN-04 is inside the WR-02 commit)
- Skipped: 1 (IN-02)
- Partly applied: WR-01 (bracketed paste and cooldown not done), WR-05 (no `env_clear`), IN-05 (OSC 52 cap not done)

**Verification environment:** the main-checkout path of the linked worktree
`/home/blk/projects/rust/gsd-meta-manager-owner-inbox` on branch `feat/v2.5.0-owner-inbox`
(the orchestrator named this worktree as the only place to work, so no nested review-fix
worktree was created). `cargo test --locked --no-fail-fast --all-targets`: 4403 passed,
0 failed, 19 ignored, 65 suites, exit 0. `cargo clippy --locked --all-targets -- -D warnings`:
clean. The acknowledge round trip ran against temp fixtures only; no registered project was
touched and no real terminal, multiplexer or clipboard tool was driven.

## Fixed Issues

### WR-01: Action keys ignore key modifiers

**Files modified:** `src/ui/screens/inbox.rs`, `src/ui/screens/detail.rs`, `src/ui/screens/owner_actions.rs`
**Commit:** df9fedf1
**Applied fix:** `v`, `o`, `A`, `y` and `g` on the inbox, and `v`, `o`, `A`, `y` on the Owner block, act only
when Control and Alt are absent (`owner_actions::is_plain_action_key`; Shift stays allowed for `A`).
Tests: `a_control_or_alt_chord_is_not_an_action_key` (red before the fix) and a chord loop in
`owner_block_o_opens_the_viewer`. Status: fixed, requires human verification (input-handling logic).
Not applied from the suggestion: bracketed paste (see Skipped) and the per-group `v` cooldown.

### WR-02: A failed spawn is reported as a timeout

**Files modified:** `src/bounded_output.rs`, `src/launcher/process.rs`, `src/ui/screens/owner_actions.rs`
**Commit:** 80925de5
**Applied fix:** new `BoundedFailure` (`Spawn`, `TimedOut`, `Io`, `Sink`) and `try_` variants of the
bounded runner; the `Option` shapes are thin wrappers, so other callers are unchanged. `run_ack` now says
`could not start gsd-tools (kind); nothing was run` for a spawn or sink failure, and keeps the timeout text
only for a real overrun. A stderr file that cannot be read back no longer hides a finished child's exit.
Test: `run_ack_tells_a_spawn_failure_from_a_timeout` (the old test pinned the bug). Also covers IN-04:
`ACK_BUDGET` is defined on every target and the message spells it from the budget actually used.

### WR-03: Cursor re-anchor lands on the fold line

**Files modified:** `src/ui/screens/inbox.rs`, `src/app.rs`
**Commit:** 29703f88 (formatting: 797b31fc)
**Applied fix:** `cursor_pos` clamps a vanished row to `rows.len() - 1` and reaches the fold line only when no
row is left. The tests now read the cursor through `cursor_pos` over `stop_rows`; the test-only `resolve_cursor`
is deleted. Reproduced first: the strengthened test failed with `<fold line>` instead of `elm-shop`.
Status: fixed, requires human verification (cursor logic).

### WR-04: `--ws` value skips the flag-shape guard

**Files modified:** `src/state_reader/owner_items/rules.rs`, `src/ui/screens/owner_actions.rs`
**Commit:** eace9662
**Applied fix:** `is_workstream_name` itself now rejects a leading `-` and dot-only names, so the acknowledge
`--ws` value and the `--ws` inside the `v` slash command share the guard. Tests cover `-x`, `--raw`, `.`, `..`
on both paths (rules unit test went red without the guard).

### WR-05: Acknowledge launcher resolved project-local first

**Files modified:** `src/state_reader/queue_md.rs`, `src/ui/screens/owner_actions.rs`, `README.md`
**Commits:** 5f4fa4c7 (code), 0892638b (README)
**Applied fix:** new `resolve_user_gsd_tools`, which takes no project: `${CLAUDE_CONFIG_DIR:-~/.claude}`, then
`${CODEX_HOME:-~/.codex}` (via `install_candidates(None, ..)`, the same list `tests/owner_acknowledge_roundtrip.rs`
uses), then an absolute `PATH` entry. The PATH arm skips relative entries and returns the full path, because the
child runs with the project as cwd and a bare `gsd-tools` would be looked up there. `smart-entry` (read-only) keeps
the project-aware resolver. README now states where the launcher comes from. Tests: user install chosen,
project-only install yields none, `CLAUDE_CONFIG_DIR` before `CODEX_HOME`, relative PATH skipped. Not applied:
`env_clear` plus an allowlist for the child (a design change; the review marked it "consider"). Status: fixed,
requires human verification (trust-boundary logic).

### WR-06: File viewer drops lines past 65,535 and reads past its cap

**Files modified:** `src/ui/screens/owner_actions.rs`, `src/ui/screens/owner_file_view.rs`
**Commit:** 10908488
**Applied fix:** `read_owner_file` opens once, checks the handle is a regular file, and reads at most cap + 1 bytes
(`read_capped`; refused when over the cap). The viewer caps its lines at `u16::MAX` and ends with a DIM
`... N more lines not shown`. Tests: `read_capped_stops_one_byte_past_the_cap` (an endless reader is refused),
`a_file_past_the_scroll_range_ends_with_a_notice`.

### IN-01: Busy set keyed by basename

**Files modified:** `src/ui/screens/mod.rs`, `src/ui/screens/owner_actions.rs`, `src/action.rs`, `src/ui/screens/ack_confirm.rs`, `src/ui/screens/inbox.rs`
**Commit:** 26e8b37d
**Applied fix:** the busy set and the `OwnerAcknowledged` answer carry the project-relative file. Status lines still
name the basename. Test: `a_same_named_file_elsewhere_is_not_busy`. Not applied: naming the workstream in the
status line (a copy change).

### IN-03: `focus_status` keys off a literal prefix

**Files modified:** `src/multiplexer/mod.rs`, `src/ui/screens/owner_actions.rs`
**Commit:** 462cd011
**Applied fix:** `multiplexer::FOCUS_OK_PREFIX` is used by `focus_with_procs` and by `focus_status`. Existing
focus tests cover both ends.

### IN-04: "10 s" hard-coded in the timeout text

Fixed inside the WR-02 commit (80925de5); see WR-02.

### IN-05: Copy-size message rounding (partial)

**Files modified:** `src/ui/screens/owner_actions.rs`
**Commit:** 569dc86e
**Applied fix:** the size rounds up (`div_ceil`), so 64 KiB + 1 byte reads `65 KiB`. Test added to
`too_large_to_copy`. The OSC 52 half (lower cap or a "may be dropped" note) is not applied: it changes the copy
limit or the status copy, which is a product decision.

### IN-06: No guard against overlapping copies

**Files modified:** `src/app.rs`, `src/ui/screens/mod.rs`, `src/ui/screens/detail.rs`, `src/ui/screens/driver_confirm.rs`, `src/ui/screens/delete_confirm.rs`, `src/ui/screens/normal.rs`, `src/ui/screens/owner_actions.rs`
**Commit:** 8fbc3ea1
**Applied fix:** `AppContext::owner_copy_busy`, set when a copy starts and cleared by the `ClipboardCopied`
handler; a second `y` says `Already copying; wait for it to finish`. Test extended in
`start_copy_reports_the_tools_answer`.

## Skipped Issues

### IN-02: `v` does not scrub `GSD_WORKSTREAM` and session-pointer variables

**File:** `src/ui/screens/owner_actions.rs:118-131`
**Reason:** the review offers "document it or scrub it". Scrubbing means changing the environment handed to
three launch backends (multiplexer, GUI terminal, direct) for root-scoped commands, which is a behavior design
change with its own tests; documenting alone leaves the finding open. Left for a decision.
**Original issue:** the acknowledge child removes `ACK_ENV_REMOVE`, but `v` launches inherit the TUI's environment,
so an exported `GSD_WORKSTREAM` could send a root-scoped command to another workstream.

### WR-01 (parts): bracketed paste and the `v` cooldown

**File:** `src/main.rs`, `src/tui.rs`, `src/input.rs`
**Reason:** enabling bracketed paste and swallowing `Event::Paste` would break pasting into every text input in the
app (new-project name, composers, prompts), which currently works because a paste replays as key events. The `v`
cooldown adds timing state and new status copy. Both are design changes; the modifier guard closes the reported
Ctrl-V, Ctrl-O, Ctrl-Y and Ctrl-G cases. A pasted plain `v`, `A`, `y` is still replayed as keys.

---

_Fixed: 2026-11-15_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
