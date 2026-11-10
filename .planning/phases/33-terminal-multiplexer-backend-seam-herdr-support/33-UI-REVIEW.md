# Phase 33 - UI Review

**Audited:** 2026-11-10
**Baseline:** abstract 6-pillar standards (no UI-SPEC); TUI source audit, no browser surface
**Screenshots:** not captured (Rust ratatui TUI, no dev server)
**Interaction captures:** off

## Scope

Phase 33 is a backend seam (`src/multiplexer/`), not a UI change. No widget, layout, style, color or keybinding was added or changed. The only user-visible surface is the status-line text from `launch` and `focus_message` (`src/multiplexer/mod.rs` ~lines 596-690), plus README/docs prose. Visuals, Color, Typography and Spacing are N/A.

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Status strings are specific and actionable; some are terse or jargon-heavy |
| 2. Visuals | N/A | No rendering change |
| 3. Color | N/A | No style change |
| 4. Typography | N/A | No text-style change |
| 5. Spacing | N/A | No layout change |
| 6. Experience Design | 3/4 | Failure and fallback paths are surfaced; no UI-level verification |

**Overall: 6/8 scored pillars (4 N/A)**

## Top 3 Priority Fixes

1. **Fallback status exposes raw backend reasons** (`mod.rs:601`, `608`: `<name>: <reason> - opened in <term>`) - the user may see internals such as socket errors, and a long reason may be truncated on a narrow status line - cap the reason length or map it to a short human phrase, keeping the full text in `tracing`.
2. **"Tab-switching requires running inside tmux or herdr (none detected)"** (`mod.rs:660`) - states the cause but gives no remedy for a user on a plain terminal - consider a hint such as "start gsd-meta-manager inside tmux or herdr" (inferred, optional).
3. **Ambiguous herdr focus and the cwd last-resort tier** (33-02 summary residual) - focus may land on a plain shell tab in the same directory, and the success text "Switched to <label> (tab only)" does not say the match was approximate - word the note to say the match was by directory.

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- Success and no-backend strings keep the tmux wording byte-identical (`Switched to <label>`, `Tab-switching requires running inside ... (none detected)`), so there is no regression for tmux users.
- Failure strings follow a consistent `<backend>: <reason>` pattern, and multi-backend focus failures are name-prefixed. This is good.
- Only the generic `Failed to launch: {e}` (`mod.rs:604`) is vague.
- The note `(tab only)` (`mod.rs:677`) is jargon for the tab-focus fallback.

### Pillar 2 / 3 / 4 / 5: N/A
No render code was touched (`detail.rs` and `normal.rs` changes only reroute calls). Nothing to score, and no findings were invented.

### Pillar 6: Experience Design (3/4)
- The launch path falls through to the GUI terminal chain and tells the user why. Focus tries each detected backend innermost-first. A refusal on ambiguity is safer than a wrong jump.
- A non-UTF-8 cwd is refused rather than opening the tab elsewhere.
- There is no loading or in-progress feedback during the herdr socket call (bounded at 2 s connect and 5 s reply). The bounded wait can block the UI thread for up to about 7 s. This is unverified and inferred from the summary; confirm whether `launch` runs off the render loop.
- Live herdr tests are `#[ignore]`, so SC1 (Tab inside herdr) is pending human UAT. This is not a defect, but it is unverified here.

## Files Audited
- `.planning/phases/33-*/33-01-SUMMARY.md`, `33-02-SUMMARY.md`
- `src/multiplexer/mod.rs` (status strings), `src/multiplexer/gui.rs`
