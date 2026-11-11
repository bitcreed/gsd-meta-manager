---
phase: 33
fixed_at: 2026-11-10T23:30:00Z
review_path: .planning/phases/33-terminal-multiplexer-backend-seam-herdr-support/33-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 9
skipped: 0
status: all_fixed
---

# Phase 33: Code Review Fix Report

**Fixed at:** 2026-11-10
**Source review:** `33-REVIEW.md` (WR-01..03, IN-01..05) plus `33-UI-REVIEW.md` (UI-01, UI-03; UI-02 not applied)
**Iteration:** 1

**Summary:**
- Findings in scope: 9 (3 warning, 5 info, 1 UI-review item pulled in by the orchestrator)
- Fixed: 9 (IN-01 in the reviewer's second form, see below)
- Skipped: 0
- Verification: run in the main checkout on `master` (no worktree). After the last commit: `cargo test --no-fail-fast` 3755 passed, 0 failed, 19 ignored; `cargo clippy --all-targets -- -D warnings` clean. Baseline before any fix was 2999 + integration suites, all green.

All commits are local; nothing was pushed. `src/cli.rs` and `src/main.rs` were not touched.

## Fixed Issues

### WR-01: herdr cwd-only tier can focus a pane that does not run the session

**Files modified:** `src/multiplexer/herdr.rs`
**Commit:** 8425be8
**Status:** fixed: requires human verification (logic change in a focus join)
**Applied fix:** The directory-only tier no longer guesses.
- `HerdrPane` now parses herdr's `agent` field (the research doc shows `"agent":"claude"` on agent panes and the field absent on non-agent panes).
- The cwd tier (`cwd_fallback`) considers only candidates whose `agent` equals the session kind's name AND whose `process-info` could not be read. A plain shell pane is never focused. A pane whose `process-info` was read and named other processes (for example another Claude in the same directory) is positively not the session and is excluded, so it cannot pre-empt tmux's exact `pane_tty` match.
- A directory match carries the note `matched by working directory only`, combined with the tab note when the tab fallback also applies.
- The pid tier is deliberately NOT agent-restricted: herdr can fail to track an agent in a pane that does run the session (the existing `agent_not_found` -> `tab focus` path covers exactly that), and a pid match is exact.
- Tests added: plain shell pane in the same cwd is not focused (and nothing is focused); a Claude pane in the same cwd is focused with the note (when process-info is unreadable); a Claude pane whose process-info names another pid is not focused; unit test for `cwd_fallback` per kind. Existing tests that relied on an agent-less pane matching by cwd were rewritten to use an agent pane or a pid match.
- [inferred, for audit] The reviewer offered two designs (require process-info evidence, or return a partial and let `focus_with` keep trying). The chosen design is the first: it needs no two-phase focus, since `focus` has already acted by the time it returns.
- [inferred, for audit] `SessionKind::Codex` maps to herdr agent name `"codex"`. Only `"claude"` is measured in research. A wrong guess costs a missed directory match for Codex (the pid tier still works), never a wrong focus.
- Residual: a Claude-agent pane in the same directory with unreadable process-info can still be focused by directory, with the approximate-match note, and then pre-empts an outer backend. This needs process-info to fail for that pane.

### WR-02: herdr pid tier gated behind a byte-exact cwd comparison

**Files modified:** `src/multiplexer/herdr.rs`
**Commit:** d947b32
**Status:** fixed
**Applied fix:** `cwd_candidates` compares with `same_dir`: equal as written, or both paths resolve (`fs::canonicalize`) to the same place. A path that cannot be resolved still matches only by exact equality. This is the reviewer's second option; it adds no extra process spawns. Test uses a real symlink in a tempdir.
- [inferred, for audit] Chose canonicalization over probing every pane because probing costs one CLI spawn per pane on the UI thread.

### UI-01: launch status line shows raw backend error text

**Files modified:** `src/multiplexer/mod.rs`, `src/multiplexer/herdr.rs`
**Commit:** 781a36b
**Status:** fixed
**Applied fix:** `launch_with` now reports `<backend>: <phrase> — opened in <term>` (and `...; no terminal emulator found (set $TERMINAL)`), where the phrase comes from the new `MuxError::launch_phrase()` (by class: "no target found", "cannot open sessions", "refused for safety", "could not open a session") or is "not reachable" for an unreachable detection. The full reason is logged with `tracing::warn!`. Tests pin the new strings and that the raw herdr error stays out. The tmux fall-through text is therefore no longer byte-identical to the pre-phase `tmux: <reason>`; this was requested by the UI review and the orchestrator.
- [inferred, for audit] Phrase wording, and mapping by class rather than truncating, were chosen because a cap would still show socket paths and JSON.

### UI-03: approximate matches should say so

**Files modified:** `src/multiplexer/herdr.rs`
**Commit:** 8425be8 (cwd note), 781a36b (tab note wording)
**Status:** fixed
**Applied fix:** The directory-only path says `(matched by working directory only)` (see WR-01). The jargon note `tab only: ...` became `tab focused, not the pane: herdr tracks no agent in that pane and its tab has N panes`.

### WR-03: `MuxError::is_try_next` and the documented classes are not applied

**Files modified:** `src/multiplexer/mod.rs`
**Commit:** 0a3a0ea
**Status:** fixed: requires human verification (contract decision)
**Applied fix:** Deleted `is_try_next` and its test; rewrote the `MuxError` docs to say the classes describe why a backend did not act, and that both seam functions treat every error alike (report, try the next step, first success wins). The doc records why falling through on `RefusedForSafety` is safe (no later step reuses the refused vector).
- [inferred, for audit] Chose "delete and document" over "implement": `33-VERIFICATION.md` states that falling through to the GUI chain on a refusal is exactly what MUX-05/MUX-06 require, and a herdr ambiguity refusal falling through to an outer backend's own exact join is the designed D-09 behavior.

### IN-02: neutral wording for `ExecArgv` refusals and the layout.apply error

**Files modified:** `src/multiplexer/mod.rs`, `src/multiplexer/herdr.rs`
**Commit:** 80e56ad
**Status:** fixed
**Applied fix:** Refusals now say "a new window" and "the multiplexer's own option parser"; the pinned substrings `directly` and `option` are kept. The reply error reads `layout.apply failed (code): message` (no doubled "herdr"). The old "still name tmux" rationale doc was rewritten.

### IN-03: `"error": null` treated as a failure

**Files modified:** `src/multiplexer/herdr.rs`
**Commit:** 0bafda2
**Status:** fixed
**Applied fix:** `object.get("error").filter(|e| !e.is_null())`; tests cover a success with `"error":null` and a null error with no result (still a failure). The echoed-id check was NOT added [inferred, for audit]: nothing shows herdr always echoes the id, and a strict check would turn a working open into a failure plus a second GUI window.

### IN-05: test listeners can hang the suite

**Files modified:** `src/multiplexer/mod.rs`, `src/multiplexer/herdr.rs`
**Commit:** c7a5478
**Status:** fixed
**Applied fix:** A test-only `accept_within` (non-blocking accept polled for up to 10 s, plus a read timeout on the accepted stream) replaces the blocking `accept()` in `serve_once` and in the closing-listener test, so a client that never connects fails the test instead of hanging it.

### IN-01: `Detection.inside` can never be false (necessity)

**Files modified:** `src/multiplexer/mod.rs`
**Commit:** 67d2854
**Status:** fixed: requires human verification (reviewer's second option; no code removed)
**Applied fix:** Kept the field and added `order_innermost_skips_a_backend_that_is_not_inside`, a fixture backend returning `inside: false`. Removal was not done [inferred, for audit]: `inside` vs `reachable` is a locked design decision (D-02) recorded in the trait's design note, and the deferred kitty backend (MUX-F03) is the stated case that needs it. The reviewer offered the fixture as the alternative.

### IN-04: uneven blocking bounds, leaked connect thread

**Files modified:** `src/multiplexer/herdr.rs`
**Commit:** 401f652
**Status:** fixed (documentation only)
**Applied fix:** The timeout constants' doc now states the real worst case (about 9 s for a launch), that the CLI calls used by focus have no deadline (same exposure as tmux, T-33-13, accepted), and that a timed-out connect leaves its helper thread until the kernel gives up. A CLI deadline (spawn, poll `try_wait`, kill) was not added [inferred, for audit]: it is a behavior change beyond parity with tmux and the docs record the exposure as accepted.

## Not applied

### UI-02: "Tab-switching requires running inside tmux or herdr (none detected)" gives no remedy

Not in the orchestrator's requested scope, marked optional/inferred in the UI review, and the string is pinned byte-identical for tmux users (SC2). Left as is.

## Notes for the orchestrator

- `33-REVIEW-DISPOSITION.md` rows for WR-01, WR-02, WR-03, IN-01..IN-05 can be set to `fixed`; IN-01 is "fixed (fixture, field kept)" and IN-04 "fixed (docs only)".
- The design table in `src/multiplexer/mod.rs` still says herdr focus ends "else unique cwd"; it is now "else a unique Claude-agent pane by cwd when process-info is unreadable". Not edited here to keep this change to behavior and its tests.
- Pre-existing `cargo doc` warnings exist elsewhere in the crate (private intra-doc links); none were introduced for the touched docs, which use escaped brackets.

---

_Fixed: 2026-11-10_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
