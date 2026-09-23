---
phase: quick-260923-lr8
plan: 01
subsystem: terminal-switch
tags: [tmux, tty, bugfix, security, tdd]
status: complete
requires: []
provides:
  - exact tmux pane TTY match in switch_to_session (strip one leading /dev/ on both sides, compare equal)
affects:
  - src/ui/screens/normal.rs and src/ui/screens/detail.rs Tab-switch status line (behaviour unchanged on a real match or a miss)
tech-stack:
  added: []
  patterns:
    - pure helper extracted from an impure tmux call so the predicate is testable without a tmux server
key-files:
  created: []
  modified:
    - src/terminal_switch.rs
    - src/session_detector.rs
decisions:
  - "Normalize both sides with the same /dev/ strip (INFERRED 1)"
  - "Empty post-strip session TTY never matches (INFERRED 2)"
  - "Doc-only edit of ClaudeSession.tty in session_detector.rs (INFERRED 3)"
  - "Test via private pure helpers pane_tty_matches/find_pane_target, not through switch_to_session (INFERRED 4)"
metrics:
  duration: ~4m
  started: 2026-09-23T20:53:49Z
  completed: 2026-09-23T20:57:12Z
actuals:
  tokens: 2400
  tasks: 2
  commits: 2
plan_head_before: de81754be0386a314228cccd108f70613b09d979
---

# Quick 260923-lr8: Exact tmux pane TTY match Summary

`switch_to_session` now focuses a tmux pane only when that pane's `#{pane_tty}`, minus one leading `/dev/`, is exactly equal to the session TTY. Before, any pane whose TTY contained the session TTY as a substring could match, so a session on `pts/1` could focus the pane on `/dev/pts/12`. The fix is pinned by RED-proven regression tests and an edge-case table.

## Tasks

| Task | Name | Commit | Files |
| ---- | ---- | ------ | ----- |
| 1 | Tracer: exact TTY match through the pane-selection path | 67d2ac6 | src/terminal_switch.rs |
| 2 | Edge-case table, doc contracts, full-suite gates | 7cf6910 | src/terminal_switch.rs, src/session_detector.rs |

## What changed

- New private helpers in `src/terminal_switch.rs`:
  - `strip_dev_prefix` removes one leading `/dev/`.
  - `pane_tty_matches` strips both sides, returns false for an empty session TTY, and otherwise compares for equality.
  - `find_pane_target` parses the `list-panes` output. It keeps the whole rest of the line as the target and skips lines that have no target.
- `switch_to_session` calls `find_pane_target(&stdout, tty)`. The tmux argv, the format string, the select-window/select-pane pair and every error string are byte-identical to before.
- Doc fixes:
  - The module doc in `terminal_switch.rs` now describes the exact-equality rule.
  - The `ClaudeSession.tty` field doc in `session_detector.rs` no longer describes a `contains()` match. This file got a doc-only change.
- Tests added to `mod tests`:
  - `pane_tty_match_is_exact_not_substring`
  - `pane_tty_find_target_skips_a_longer_tty_listed_first`
  - `pane_tty_match_edge_cases` (12 rows)
  - `pane_tty_find_target_parsing_edge_cases` (4 rows)

## TDD Gate Compliance / RED evidence

At the RED step `pane_tty_matches` still held the substring predicate copied unchanged from the old line 268. Running `rtk proxy cargo test --no-fail-fast --lib terminal_switch` gave:

```
test terminal_switch::tests::pane_tty_match_is_exact_not_substring ... FAILED
test terminal_switch::tests::pane_tty_find_target_skips_a_longer_tty_listed_first ... FAILED

thread 'terminal_switch::tests::pane_tty_match_is_exact_not_substring' panicked at src/terminal_switch.rs:462:9:
session TTY pts/1 matched the pane on /dev/pts/12: Tab-switch would focus an unrelated pane and send the operator's keystrokes there

thread 'terminal_switch::tests::pane_tty_find_target_skips_a_longer_tty_listed_first' panicked at src/terminal_switch.rs:478:9:
assertion `left == right` failed: with /dev/pts/12 listed before /dev/pts/1, the session on pts/1 must focus main:0.0; any other target misfocuses the operator's input
  left: Some("work:1.0")
 right: Some("main:0.0")

test result: FAILED. 3 passed; 2 failed; 1 ignored
```

After the GREEN change the result was `5 passed; 0 failed; 1 ignored`. After Task 2 it was `7 passed; 0 failed; 1 ignored`.

RED and GREEN went into one commit (67d2ac6), as the plan's "Commit once" instruction says, so no RED-only commit exists. The output above is the evidence.

## Verification

- `cargo test --no-fail-fast --lib terminal_switch`: 7 passed, 0 failed, 1 ignored. The ignored test is the live-tmux probe.
- `cargo test --no-fail-fast` (full suite, 49 test binaries): 2194 passed, 1 failed, 15 ignored. The one failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the known local git-version witness.
- The comment-stripped `terminal_switch.rs` has 0 occurrences of `pane_tty.contains`. `find_pane_target(&stdout, tty)` appears once. `session_detector.rs` has 0 occurrences of `contains()` and 1 of `pts/12`.
- `cargo clippy -- -D warnings`: exit 0.
- `cargo clippy --all-targets -- -D warnings`: exit 101, with 0 diagnostics in `src/terminal_switch.rs` or `src/session_detector.rs`. This baseline failure is outside this item's scope. The failing files are:
  - `src/browser.rs` (3)
  - `src/project_creator.rs` (1)
  - `tests/envelope_carrier_reach.rs` (1)
  - `tests/envelope_config_resolution.rs` (1)
  - `tests/envelope_control_carrier.rs` (1)
  - `tests/envelope_wrapper_class.rs` (4)
- rustfmt (edition 2021) on `terminal_switch.rs` reports one diff: the `Err(...)` wrapping in the `in_tmux` guard of `switch_to_session`. That diff was already there at base de81754, and the plan says to leave the guard alone. The code added here is rustfmt-clean.

## Deviations from Plan

None. The plan was executed as written.

## Inferred decisions (audit)

- **[INFERRED 1]** Both sides get the same one-time `/dev/` strip, not only `pane_tty`. For today's `read_tty` output this changes nothing. It also covers a future caller that records the unstripped form, for example the lr9 Codex path. Pinned by the row `("/dev/pts/1","/dev/pts/1") -> true`.
- **[INFERRED 2]** An empty session TTY, after the strip, matches nothing. The rows `("/dev/pts/1","")`, `("","")` and `("/dev/","/dev/")` are all false, and `find_pane_target(..., "")` returns None.
- **[INFERRED 3]** `src/session_detector.rs` got a doc-only edit to the `ClaudeSession.tty` comment. The struct-level doc (lines 14-16) is unchanged. lr9 also edits `ClaudeSession`, and these edits are on adjacent lines.
- **[INFERRED 4]** The tests call private pure helpers instead of `switch_to_session`, which needs `$TMUX` and a live server. Because of this, the error strings and tmux argv shown by `normal.rs` and `detail.rs` did not change.
- **[Executor]** RED and GREEN share one commit because of the plan's "Commit once" instruction. The RED run is recorded above.
- **[Executor]** The pre-existing rustfmt diff in `switch_to_session`'s `in_tmux` guard was left alone, both to keep scope tight and because the plan says not to touch the guard.

## Threat Flags

None. No new surface. T-LR8-01 and T-LR8-02 are mitigated and pinned by the tests listed above. T-LR8-03 and T-LR8-04 did not change.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: src/terminal_switch.rs, src/session_detector.rs
- FOUND: 67d2ac6, 7cf6910 (in git log)
