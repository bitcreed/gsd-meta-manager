# Phase 33 — UI Review

**Audited:** 2026-11-10 (re-audit after the code-review fix commits 781a36b, 8425be8, 80e56ad .. 401f652; supersedes the b3648c8 review)
**Baseline:** abstract 6-pillar standards (no UI-SPEC); TUI source audit, no browser surface
**Screenshots:** not captured (Rust ratatui TUI, no dev server on localhost:3000, 5173 or 8080)
**Interaction captures:** off (workflow.ui_interaction_capture is false)

---

## Scope

Phase 33 is a backend seam (`src/multiplexer/`), not a rendering change. No widget, layout, style, color or keybinding was added or changed. `detail.rs` and `normal.rs` only reroute calls to `multiplexer::launch` and `multiplexer::focus_message`. The user-visible surface is the status-line text those two functions return (`src/multiplexer/mod.rs` `launch_with` ~L611-641 and `focus_with` ~L683-723, plus the herdr notes and errors in `src/multiplexer/herdr.rs`). Visuals, Color, Typography and Spacing are N/A, as in the first review. [inferred, for audit] N/A is kept instead of a forced 1-4 score, because scoring render pillars on code this phase did not touch would invent findings.

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Launch fall-through is now a short human phrase (UI-01 fixed); focus failures still show raw backend text, and the partial-focus note is long |
| 2. Visuals | N/A | No rendering change |
| 3. Color | N/A | No style change |
| 4. Typography | N/A | No text-style change |
| 5. Spacing | N/A | No layout change |
| 6. Experience Design | 3/4 | Fallbacks and approximate matches are reported; the synchronous herdr calls can still freeze the UI without feedback |

**Overall: 6/8 scored pillars (4 N/A)**. This is unchanged from the first review: two of its three fixes landed, and the remaining findings are of the same weight.

---

## Status of the First Review's Fixes

| Item | Status | Evidence |
|------|--------|----------|
| UI-01 raw reasons on the launch status line | fixed | `MuxError::launch_phrase()` (`mod.rs:149-156`); `launch_with` formats `"{name}: {phrase} — opened in {term}"` (`mod.rs:630`) and logs the full reason at `warn` (`mod.rs:618-622`) |
| UI-02 no remedy in "Tab-switching requires running inside tmux or herdr (none detected)" | not applied (by decision in 33-REVIEW-FIX.md) | `mod.rs:689-691`; still open, see fix 3 |
| UI-03 approximate matches should say so | fixed | `CWD_ONLY_NOTE = "matched by working directory only"` (`herdr.rs:741`), attached on any `MatchedBy::Cwd` focus (`herdr.rs:247`); jargon `tab only` replaced (`herdr.rs:271-273`) |

---

## Top 3 Priority Fixes

1. **WARNING — Focus failures still show raw backend text** (`mod.rs:716-722`: one backend gives `reason.clone()`, several give `"{name}: {reason}"`). UI-01 was applied to `launch_with` only. A herdr focus failure can still put strings such as `herdr pane list failed: <stderr>` (`herdr.rs:479-481`), `herdr pane list is not the expected JSON: <serde error>` (`herdr.rs:578`) or `HERDR_SOCKET_PATH is unset` (`herdr.rs:306`) on the status line. The launch and focus paths now follow two different copy policies. Fix: use the same treatment as launch. Keep the `NotFound` and the "refusing to guess" texts, which are written for people, map `Failed` to a short phrase, and log the full reason with `tracing::warn!`.
2. **WARNING — The combined partial-focus note is too long for a 3-second status line.** With both notes the line reads `Switched to <label> (tab focused, not the pane: herdr tracks no agent in that pane and its tab has 3 panes; matched by working directory only)`, about 135 characters plus the label (`herdr.rs:271-277`). The ambiguity refusal is about 120 characters (`herdr.rs:660-664`). Status messages expire after 3 s (`app.rs:2563-2567`), and a narrow terminal clips the line. Fix: shorten the notes, for example `tab focused, not the pane (3 panes)` and `matched by directory`, and move the explanation to the log.
3. **WARNING — The no-backend message names the cause but gives no remedy** (`mod.rs:689-691`, carried over from UI-02). Fix: add a hint such as `start gsd-meta-manager inside tmux or herdr`. [inferred] This stays optional; 33-REVIEW-FIX.md records UI-02 as not applied by decision.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- Improved: the launch fall-through copy is now a short, class-based phrase (`no target found`, `cannot open sessions`, `refused for safety`, `could not open a session`, `not reachable`; `mod.rs:149-156`, `mod.rs:612`). Socket paths and herdr's error JSON stay out of the status line, and tests pin this.
- Improved: the `ExecArgv` refusals are backend-neutral (`a new window`, `the multiplexer's own option parser`; `mod.rs:221-235`), and the layout.apply error no longer says "herdr" twice (IN-02).
- Improved: the tab-fallback note now explains itself (`tab focused, not the pane: ...`), and a directory-only match says so.
- WARNING: focus failure copy is still raw (fix 1). This is the main inconsistency left in this pillar.
- WARNING: the notes are verbose (fix 2).
- Minor: `Failed to launch: {e}` (`mod.rs:633`) is still a bare `io::Error` text for a GUI spawn failure. Prefer `Could not start <term>: <short reason>`.
- Minor: the `focus_with` test fixture still uses the old `"tab only"` note (`mod.rs:1605-1611`). This is not user-visible, but the fixture no longer matches the real wording.

### Pillar 2 / 3 / 4 / 5: N/A
No render code was touched. Nothing to score, and no findings were invented.

### Pillar 6: Experience Design (3/4)
- Good: launch tries only the innermost backend and reports any fall-through to a GUI terminal. Focus tries each backend innermost-first, and the first success wins. Ambiguity is refused instead of guessed. An approximate match is now reported to the user (UI-03).
- WARNING: `launch` and `focus_message` run synchronously in the key handlers (`detail.rs:4952`, `detail.rs:5378`, `detail.rs:5415`, `normal.rs:840`). IN-04 records a launch worst case of about 9 s over the socket. The herdr CLI calls used by focus have no deadline (T-33-13, accepted). The UI shows no in-progress feedback while it waits. Fix, if this becomes visible: set a "Switching..." / "Opening..." status before the call, or move the call off the render loop.
- WARNING: a 3 s expiry combined with long messages (fix 2) means the most informative text is the least likely to be read.
- Live herdr tests are still `#[ignore]`, so "Tab inside herdr" stays pending human UAT. This is not a defect.

---

## Files Audited
- `.planning/phases/33-terminal-multiplexer-backend-seam-herdr-support/33-01-SUMMARY.md`, `33-02-SUMMARY.md`, `33-CONTEXT.md`, `33-REVIEW-FIX.md`
- `src/multiplexer/mod.rs` (`MuxError::launch_phrase`, `ExecArgv::new`, `launch_with`, `focus_with`)
- `src/multiplexer/herdr.rs` (focus tiers, notes, refusals, socket and CLI error text)
- `src/app.rs` (status-message expiry), `src/ui/screens/detail.rs`, `src/ui/screens/normal.rs` (call sites)
