---
phase: 33-terminal-multiplexer-backend-seam-herdr-support
reviewed: 2026-11-10T00:00:00Z
depth: standard
files_reviewed: 14
files_reviewed_list:
  - README.md
  - docs/ARCHITECTURE.md
  - docs/CONFIGURATION.md
  - docs/DEVELOPMENT.md
  - docs/GETTING-STARTED.md
  - src/lib.rs
  - src/multiplexer/gui.rs
  - src/multiplexer/herdr.rs
  - src/multiplexer/mod.rs
  - src/multiplexer/tmux.rs
  - src/session_detector.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/normal.rs
  - tests/spawn_seam_guard.rs
findings:
  critical: 0
  warning: 3
  info: 5
  total: 8
  necessity: 1
status: issues_found
---

# Phase 33: Code Review Report

**Reviewed:** 2026-11-10
**Depth:** standard
**Files Reviewed:** 14
**Status:** issues_found

## Summary

The phase replaces the tmux-hardcoded `terminal_switch` with a `MultiplexerBackend` trait, a static registry, a herdr backend and a GUI fallback module. I diffed against `7a5c3af` and compared the moved tmux code with the deleted `terminal_switch.rs`. The tmux behaviour, reason strings and the fail-closed `ExecArgv` boundary carry over unchanged. Stale references to `terminal_switch` are gone from `src/`, `tests/` and the docs. The herdr launch path is sound: argv travels as JSON array elements built with `serde_json`, the cwd is checked for UTF-8, no shell-typing CLI verb is reachable, and the allowlisted spawn files match the real spawn sites. The docs match the code.

No security vulnerability or data-loss bug found. The real defects are in herdr's focus join, which can report a false success against a pane that does not host the session, and in the error-class contract that `MuxError` documents but the seam never applies.

## Warnings

### WR-01: herdr cwd-only tier can focus a pane that does not run the session, and it pre-empts tmux's exact tty match

**File:** `src/multiplexer/herdr.rs:217-226` (tier 3), `src/multiplexer/herdr.rs:653-664`, `src/multiplexer/mod.rs:666-683`
**Issue:** When tiers 1 (session id) and 2 (pid) find nothing, a single herdr pane whose `cwd` or `foreground_cwd` equals the session's working dir is accepted as the match. Nothing checks that the pane hosts a Claude process. `cwd_candidates` is deliberately not restricted by agent kind, and the pid probe result is not used to rule candidates out. Two realistic cases:
1. The user has a plain shell pane open in the project directory while the session runs elsewhere (a GUI terminal, a detached tmux pane). `Tab` reports `Switched to <alias>` and focuses the shell pane.
2. Nesting with herdr as the innermost multiplexer (herdr inside tmux). `focus_with` tries herdr first and "the first success wins". A same-cwd herdr pane therefore wins over the exact `pane_tty` match that tmux would have made. The module's own stated threat (`tmux.rs` `pane_tty_matches`, `herdr.rs` module docs) is a focus key that lands the operator's keystrokes in an unrelated pane. This tier reintroduces that outcome through a weaker key, and does so as a reported success rather than a refusal.

**Fix:** Make the cwd tier a last resort that cannot pre-empt another backend's exact match. Either accept a cwd match only when `process_info` succeeded for that pane and shows a foreground process whose name is the session's kind, or return the cwd match as a `partial` outcome and let `focus_with` keep trying the remaining backends before settling for it. At minimum, set `partial: Some("matched by working directory only")` for `MatchedBy::Cwd` so the status line does not claim an exact switch:
```rust
Tier::One(pane) => (pane, MatchedBy::Cwd),
...
let partial = if by == MatchedBy::Cwd && partial.is_none() {
    Some("matched by working directory only".to_string())
} else { partial };
```

### WR-02: herdr pid tier is gated behind a byte-exact cwd comparison, so it never runs for symlinked or non-canonical project paths

**File:** `src/multiplexer/herdr.rs:194-214`, `src/multiplexer/herdr.rs:656-664`
**Issue:** `process_info` is probed only for `cwd_candidates`, which compares paths with `==` and no canonicalization (`cwd_candidates` doc says so). The session's `working_dir` comes from `/proc/<pid>/cwd`, which is the resolved path. herdr's `cwd` / `foreground_cwd` can be the unresolved path (a project registered or entered through a symlink). The pid join is the most exact tier after the session id, but a cwd string mismatch removes the pane from the candidate set before the pid is ever compared. The result is a false `no herdr pane runs this session (session may have exited)` for a live session with no `agent_session` record. This is the common case for Codex sessions, which the README advertises as reachable by `Tab` under herdr.
**Fix:** Probe the pid on every pane, or at least on every pane when the cwd filter yields nothing, and only use the cwd filter for the tier-3 fallback:
```rust
let all: Vec<&HerdrPane> = panes.iter().collect();
let probe_set = if candidates.is_empty() { &all } else { &candidates };
```
Alternatively, compare `std::fs::canonicalize` of both sides in `cwd_candidates`.

### WR-03: `MuxError::is_try_next` and the documented error classes are not applied by either seam function

**File:** `src/multiplexer/mod.rs:99-141`, `src/multiplexer/mod.rs:652-693`, `src/multiplexer/mod.rs:575-613`
**Issue:** The `MuxError` docs define three classes and say `NotFound`/`Unsupported` mean "the caller tries the next one ([`MuxError::is_try_next`])" while `RefusedForSafety` "is never retried through a path that adds one". Neither seam function consults `is_try_next`; it is referenced only by its own unit test (`mod.rs:1402-1405`), so it is dead in production. The behaviour differs from the contract:
- `focus_with` pushes every error, including `RefusedForSafety` (herdr's "refusing to guess" ambiguity) and `Failed`, onto `failures` and continues to the next backend. Only `NotFound` was meant to be non-final.
- `launch_with` falls through to the GUI on `RefusedForSafety` too (pinned by `launch_with_a_refused_argv_never_calls_open`). This is safe today because `gui_argv` does not use the refused vector, but the doc says otherwise.

A future backend author who trusts the doc will get different behaviour from the one implemented.
**Fix:** Either implement the contract (in `focus_with`, stop and report on a `RefusedForSafety`, and use `is_try_next()` for the continue decision), or delete `is_try_next` and rewrite the class docs to describe the actual "every error falls through, first success wins" behaviour. Do not keep a public method that exists only for its test.

## Info

### IN-01: `Detection.inside` can never be false, so `order_innermost`'s `inside` check guards nothing

**File:** `src/multiplexer/mod.rs:281-287`, `src/multiplexer/mod.rs:413`
**Category:** necessity
**Issue:** Generality without instantiation. `detect` returns `Option<Detection>`, and both registered backends return `Some` only with `inside: true` (`tmux.rs:72-75`, `herdr.rs:166-169`; the fake in the tests does too). The `if detection.inside` branch in `order_innermost` and the `inside` field are therefore always true in the repository. `None` already means "not inside". The rationale (kitty can be inside but not reachable) is carried by `reachable`, not `inside`. Evidence rests on the current backend set, so this is a judgment call rather than a proof; it is Info.
**Fix:** Drop `inside` and treat `Some(_)` as inside, or document the case in which a backend returns `Some { inside: false }` and add a fixture that exercises it.

### IN-02: `ExecArgv` refusals name "a tmux window" when raised on the herdr path

**File:** `src/multiplexer/mod.rs:203-219`, `src/multiplexer/mod.rs:604-611`
**Issue:** The refusal texts say `refusing to open a tmux window ...`, and `launch_with` prefixes the backend name, so a herdr launch would show `herdr: refusing to open a tmux window from a 1-element program vector`. The doc records this as intentional for tmux byte-compatibility, and the branch is unreachable from the in-tree builders. A related cosmetic: a herdr socket failure reads `herdr: herdr layout.apply failed (...)` (pinned in `mod.rs:1652`), with the name twice.
**Fix:** Make the `ExecArgv` messages backend-neutral ("refusing to open a session from a ... program vector") and update the two tests that pin `tmux window`, or drop the `herdr ` from `parse_layout_apply_reply`'s text.

### IN-03: `parse_layout_apply_reply` treats `"error": null` as a failure and never checks the echoed id

**File:** `src/multiplexer/herdr.rs:129-148`
**Issue:** `object.get("error")` is `Some(Value::Null)` for a reply such as `{"id":"gsdmm-open","result":{...},"error":null}`. That is reported as `herdr layout.apply failed (unknown): unknown` even though a `result` is present, so a successful tab open would be shown as a failure and the GUI fallback would open a second window. The id is also never compared to `REQUEST_ID`, although the request sets it and the docs say herdr echoes it.
**Fix:** Use `object.get("error").filter(|e| !e.is_null())`, and optionally require `object.get("id") == Some(&json!(REQUEST_ID))`.

### IN-04: Blocking bounds are uneven, and an abandoned connect thread is leaked per timeout

**File:** `src/multiplexer/herdr.rs:300-338`, `src/multiplexer/herdr.rs:342-395`, `src/multiplexer/herdr.rs:445-449`
**Issue:** The socket path is bounded (2 s connect, 2 s write, 5 s read), which is a worst case of about 9 s on the UI thread, not the 2 s and 5 s the constant comments suggest. `herdr_cli` is unbounded: `Command::output()` has no timeout, and focus runs it once for `pane list` and once per cwd candidate for `process-info`. A wedged herdr server freezes the TUI there. The docs record this as accepted (T-33-13, parity with tmux), so it is Info. Each connect that times out leaves a `herdr-connect` thread blocked in `connect(2)`, and repeated `n` presses against a full backlog accumulate them.
**Fix:** If parity with tmux is the intent, say in the constant comments that the bound is the sum of the three timeouts. Otherwise run the CLI calls with a deadline (spawn, then poll `try_wait` and kill on expiry).

### IN-05: Test listeners in `mod.rs` and `herdr.rs` can hang the suite if the client never connects

**File:** `src/multiplexer/mod.rs:1562-1589`, `src/multiplexer/herdr.rs:786-810`
**Issue:** `serve_once` and the closing-listener test call blocking `accept()` with no timeout and the test then calls `handle.join().unwrap()`. If the client fails before `connect` (for example a regression that errors out on the socket path), the test hangs instead of failing. That makes a regression look like a CI timeout.
**Fix:** `listener.set_nonblocking(true)` and poll `accept` for a bounded time, or join through `recv_timeout` on the channel.

---

_Reviewed: 2026-11-10_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
