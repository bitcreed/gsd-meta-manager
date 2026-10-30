# Phase 25 — UI Review (post-execution re-verification, verify:post)

**Audited:** 2026-10-28
**Baseline:** abstract 6-pillar standards (no UI-SPEC.md), read for a ratatui TUI (colour = style semantics, typography = bold/dim hierarchy, spacing = layout and column structure)
**Screenshots:** captured as terminal frames (tmux `capture-pane`), not images. Binary built with `rtk proxy cargo build` (HEAD, v1.15.0), run with a throwaway HOME/XDG and `--config` under the scratchpad, at 160x45 and 100x30. Fixture: a git repo registered as `demo`, with phase 5 (4 plans, waves 1-2, plan 01 has a SUMMARY), two `.claude/worktrees/agent-*` worktrees on `worktree-agent-p05-NN-<ts>` branches, and Claude meta/jsonl files (one fresh, one 5 minutes old). Captures: `/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/30911796-0f3c-4659-8e08-ce10f00ad0d8/scratchpad/` files `dash2.txt`, `dash2.ansi`, `detail2.txt`, `detail-160x45.txt`, `detail-100x30.txt`, `agents-160x45.txt`, `agents-100x30.txt`.
**Interaction captures:** off (`interaction_capture: false`). Navigation was driven through `tmux send-keys`; only the keys listed above were exercised. Hover and mouse states were not.
**Registry audit:** skipped (no `components.json`).
**Method:** read the 07 summaries and 25-CONTEXT.md (D-C14 placement), grepped `src/ui/screens/{normal,detail}.rs`, and compared the rendered frames to what the plans promised. Judgments marked `[inferred — cleanup 2026-10-28]` have no human behind them.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 2/4 | The Agents rows and the Waves pane disagree about the same plan: `unmerged` in one, `done` in the other. Glyph-and-abbreviation vocabulary (`+0 ~1`, `w2∙`, `1 run`) has no legend on the screen. |
| 2. Visuals | 3/4 | The Status-cell summary is compact and the Waves pane has a clear w1/w2 hierarchy. Plan rows carry no agent type, age or liveness, and the Agents view crowds its header lines. |
| 3. Color | 3/4 | Cyan means running and yellow means all-stalled. The summary is the only colour change in the row. The meaning is carried by words as well, so it survives monochrome. The 58 DarkGray/Gray uses in `detail.rs` are an existing todo. |
| 4. Typography | 3/4 | The hierarchy holds, but the Agents header (`▸w1 1 running  w2∙`) has no separator after the glyph and a mixed-case run-on. |
| 5. Spacing | 3/4 | The summary degrades by width. At 100x30 the panes reflow and nothing clips. The Waves and Agents panes leave 25+ empty rows and wide columns unused. |
| 6. Experience Design | 3/4 | Scan is polled, read-only and cached. The first dashboard paint shows the old D R P E V ladder until the first scan lands, then flips. Navigation (Enter, 6, Right, `waves → 2:Phases`) works. |

**Overall: 17/24**

---

## Top 3 Priority Fixes

1. **Make the Agents row and the Waves pane describe a plan with the same word** — a user cross-checking 05-03 sees `unmerged` in the Waves pane (`detail2.txt`, row `◒ unmerged 05-03`) and `done` in the Agents tab (`agents-160x45.txt`, row `done 05-03 gsd-executor +0 ~0 6m`) for the same plan, and "done" contradicts a plan that has no SUMMARY. Both come from `AgentLiveness`/wave state in `src/agents/waves.rs` rendered through `detail.rs` (`render_agents_tab` at `src/ui/screens/detail.rs:7968`, `render_waves_pane` at `:7682`). Render the liveness column with the plan state's vocabulary, or call the liveness `ended`.
2. **Label the Agents-row columns** — `+0  ~1  10s` is `commits / dirty files / age`, but nothing on screen says so (`agents-160x45.txt` rows 8-9). Add a one-line dim header (`state plan type +commits ~dirty age`) or a footer legend entry, in `render_agents_tab` (`src/ui/screens/detail.rs:7968`); also move `waves → 2:Phases` and the `w2∙` strip to a separate row or add a space after `▸w1`.
3. **Remove the first-paint flicker in the Status cell** — the dashboard renders the plain D R P E V ladder for a project with running agents until the first agent scan returns, then swaps to `w1/2 · 1 run · 1/4` (`dash-160x45.txt` at 8s versus `dash2.txt` at 14s). Seed `ctx.agent_views` on startup (the Tick-poll counter in `src/app.rs`, D-C01), or show a dim placeholder in the Status cell at `src/ui/screens/normal.rs:1133-1137`. A related gap: the summary replaces the ladder entirely, so the project's D/R/P/E/V stage is invisible while agents run. `[inferred — cleanup 2026-10-28]`: keep the ladder's current stage in the 20-column cell when width allows.

---

## Detailed Findings

### Pillar 1: Copywriting (2/4)
- **WARNING: contradictory vocabulary.** The Waves pane calls plan 05-03 `unmerged` (`detail2.txt`), the Agents tab calls its agent `done` (`agents-160x45.txt`). The first says the work is not integrated, the second says it is finished. See fix 1.
- **WARNING: unlabeled columns.** `+0  ~1  10s` (`agents-160x45.txt`). A user learns `+`=commits and `~`=dirty only from help or source. `agent_summary_line` (`src/ui/screens/normal.rs:519`) has the same problem: `w1/2 · 1 run · 1/4` abbreviates to `run` without saying `1/4` is plans done. The `summary_forms` ladder (`src/agents/waves.rs:686`) does progressively shorten, which is the right idea, but the widest form is already terse.
- The Waves pane headers read well: `w1  2 parallel · 1 done · 1 running`, `✓ done 05-01`, `∙ queued 05-04`. The `waves → 2:Phases` hint is actionable.
- The dashboard also prints `◒1 unmerged` for another project (`dash2.txt`, `knit` row), so unmerged is a deliberate state with its own word. That makes the Agents-tab `done` the odd one out.
- No empty-state wording was exercised: no fixture without agents on the Agents tab. `[inferred — cleanup 2026-10-28]`.

### Pillar 2: Visuals (3/4)
- The Status cell gets a clear focal change: cyan `w1/2 · 1 run · 1/4` replaces the grey ladder for the one row with live agents (`dash2.ansi` shows `38;5;6` on the summary only). The row stays scannable.
- The Waves pane: `▸ w1` header, then indented `✓/►/◒/∙` plan rows with the cursor `>` (`detail2.txt`). The glyph set is distinct per state. Plan rows say nothing about *which agent* is running a plan, so the Waves pane and Agents tab have to be read side by side.
- **WARNING:** the Agents pane header packs three facts into two lines with no spacing (`▸w1 1 running  w2∙  waves → 2:Phases`). The `▸` has no separator before `w1`.
- 25 of 30 rows in the Waves and Agents panes are empty at 100x30 and 160x45. That is expected for a single phase, not a defect.

### Pillar 3: Color (3/4)
- Cyan for running (`38;5;6` in `dash2.ansi`), yellow for the all-stalled form (`normal.rs:515`). The selected row's green `> demo` text continues unchanged after the cyan span, so the summary does not bleed.
- The meaning is carried by text too (`run`, `unmerged`, `queued`), not only colour.
- Magenta stays reserved for "driving"/non-ordinary (`normal.rs:230-233`). The new cyan does not collide with it.
- Not verified: contrast of Cyan against the user's theme, and the unmerged/stalled colours in the Agents tab (`-e` capture was taken only of the dashboard).

### Pillar 4: Typography (3/4)
- A bold/dim hierarchy exists in the Waves pane and the strip (`Sessions │ [Agents]`). The agent rows are one flat weight.
- The Agents pane header is a run-on (`▸w1 1 running  w2∙  waves → 2:Phases`), and the `w2∙` strip glyph is unexplained.
- Note that a terminal has no size scale; the checks are the label/value split and consistent abbreviations. `run` and `running` both appear.

### Pillar 5: Spacing (3/4)
- 160x45 and 100x30 both render without clipping (`detail-160x45.txt`, `detail-100x30.txt`). The Waves pane narrows from about 94 to 58 columns and the plan rows do not wrap.
- At 100x30 the Status column is 13 wide in the dashboard and the ladder or summary is truncated to fit (`dash-100x30.txt`). The summary's shorter forms are meant to cover this. I did not capture the summary at 100x30 because the fixture's scan had not landed by the 8-second capture. `[inferred — cleanup 2026-10-28]`.
- Agent rows use two spaces between the columns and leave about 130 columns of the 160x45 pane unused. A fixed-width type column would align longer agent types (`gsd-code-fixer`) better.

### Pillar 6: Experience Design (3/4)
- Read-only and non-blocking: the scan rides the existing Tick counter and returns through an action (CONTEXT D-C01), and the render path only reads the cache (`normal.rs:1133`).
- **WARNING: first-paint state.** The ladder shows first and the summary replaces it ~5-14 s later (fix 3). Nothing indicates a scan is pending.
- Navigation works: `Enter` opens the project, `2` shows the Phases tab with the Waves pane focus hint (`[→/Enter]waves` in the footer), `6` then `Right` reaches Agents, and the pane offers `waves → 2:Phases`. The cursor survives (`> ► running 05-02`).
- Agent text goes through the `Untrusted` path with an escape-guard test (CONTEXT D-C13, `render_escape_guard.rs`). I did not inject hostile text in this run.
- No actions (kill, remove) are offered from the Agents tab, matching the phase's read-only scope.
- Disabled/error states were not exercised (no broken meta file, no non-git project).

---

## Files Audited
- `/home/blk/projects/rust/gsd-meta-manager/src/ui/screens/normal.rs` (`agent_summary_line` at 519, status-cell block at 1129-1137)
- `/home/blk/projects/rust/gsd-meta-manager/src/ui/screens/detail.rs` (`render_waves_pane` 7682, `render_agents_tab` 7968, sub-tab strip 10696)
- `/home/blk/projects/rust/gsd-meta-manager/src/agents/waves.rs` (`summary_forms` 686)
- Phase 25 SUMMARY/CONTEXT files; format precedent: the Phase 31 and Phase 28 UI reviews.
- Rendered frames listed in the header.

---

## Delta vs prior audit (eafb2d5) — no re-audit

`[inferred — cleanup 2026-10-28]` Existing review kept (the workflow's unattended path reuses it; the auditor was not re-spawned) because quick 261029-1dp (ce86206) has no user-visible TUI surface:
- `git diff --stat eafb2d5 HEAD -- src/ui src/app.rs src/agents`: only `src/ui/screens/detail.rs` (test doc comment + assert message in `mod tests`, ~21257-21277), `src/ui/screens/mod.rs:1389-1393` (doc comment on `AppContext::experimental`) and `src/agents/adapters/mod.rs:15-17` (module doc comment). No rendered string, layout, style or key binding changed; `normal.rs`, `src/agents/waves.rs` and the render paths audited above did not change.
- ce86206 drops the dead `AgentRuntime` key; `grep -rn AgentRuntime src/ui` is empty.
- Audited anchors still hold: `render_waves_pane` at `src/ui/screens/detail.rs:7682`, `render_agents_tab` at `:7968`, `agent_summary_line` at `src/ui/screens/normal.rs:519`. All three top fixes remain open as recorded.
- `rtk proxy cargo test --no-fail-fast --lib -- ui::`: 961 passed, 0 failed.

Scores unchanged: 17/24, 0 blockers; top fixes as listed above. Method: git diff + grep + render tests.
