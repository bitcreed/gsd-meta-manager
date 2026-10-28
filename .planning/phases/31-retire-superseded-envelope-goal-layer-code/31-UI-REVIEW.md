# Phase 31 — UI Review (post-31-07 re-audit, final tree at HEAD b9990c0)

**Audited:** 2026-10-28 (post-31-07 re-audit; supersedes the 2026-10-27 post-31-08 review, 19/24, and the 2026-10-25 review, 16/24)
**Baseline:** abstract 6-pillar standards (no UI-SPEC.md), read for a ratatui TUI (colour = style semantics, typography = bold/dim hierarchy, spacing = layout, popup sizing and blank-line structure)
**Screenshots:** not captured (terminal UI, no dev server; code-only audit). Plan 31-07 (D-04, commits a23d1d4, 2e2e335, b9990c0) has now executed; the former hold note is obsolete.
**Interaction captures:** off (`interaction_capture: false`); interaction findings are code-derived.
**Registry audit:** skipped (no `components.json`).
**Post-31-07 re-audit:** 31-07 retired the Codex executor and the in-process Claude transport and removed the TUI's dead exec-event channel (`AppContext::run_states`/`exec_tx`, `App::apply_exec_event`, the `main_loop` ExecEvent arm), and retargeted some screen tests to `registry::is_opted_in`. Every finding and file:line citation below was re-checked against HEAD. A grep of `src` for `run_states`, `exec_tx`, `apply_exec_event` and `ExecEvent` finds only two unrelated test names in `help.rs` (`the_driver_run_states_legend...`). No finding cited removed code. `DriverOutputLine` stays and is live through the launcher tail in `driver_run.rs`, so nothing was dropped. The 31-07 change is rendering-neutral: it removed no visible surface, so no score moved. Citations that shifted: the render block in `driver_confirm.rs` now starts at line 186, not 187.
**Method:** read the prior review, then `git diff 36e0805^..5078bf5 -- src/ui` and the current source. Judgments marked `[inferred — audit]` have no human behind them.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | WR-01 and WR-02 are fixed: the grant popup is now the single consent sentence and no longer claims a branch namespace. Internal vocabulary remains: "Phase 31" on Preferences, "driver seam" in an error, `INJECTION_RETIRED`. |
| 2. Visuals | 3/4 | The popup is a clean one-sentence body. The opt-in popup still dims the dashboard instead of the Driver tab. The Preferences panel is still a dead block. `o` is still missing from the Driver footer. |
| 3. Color | 4/4 | The DarkGray consent text was deleted with the disclosure. Red = destructive and Yellow = grant, with no hardcoded RGB in the touched screens. |
| 4. Typography | 3/4 | The hierarchy is intact. Stale names remain (`DRY_RUN_LOADING`, "legacy needs-a-human predicate" in `help.rs:105`). |
| 5. Spacing | 3/4 | The 43-row clipping risk is gone with the disclosure. The Preferences placeholder is still hand-wrapped at fixed columns. |
| 6. Experience Design | 3/4 | The live-run guard on unregister is restored (WR-03) with tests. It is not airtight (see below), and the footer omission persists. |

**Overall: 19/24** (unchanged by 31-07; 16/24 in the first review)

---

## Prior findings: resolved vs open

| Prior finding | Status |
|---|---|
| WR-01: grant popup showed retired seam, re-confirmation and file-list text | RESOLVED. `render_disclosure` and the three `SECTION_*` constants are deleted. The body is only `strip_key_hint(prompt)`. A new test (`the_grant_popup_shows_no_disclosure_of_retired_seams`, `driver_confirm.rs:937`) pins the absence of "seam", "digest", "CLAUDE.md" and "==". |
| Popup unconfirmable at ordinary heights (about 48 rows) | RESOLVED as a side effect. The body is now 1 to 2 wrapped rows, so the `CLIPPED_HINT` guard cannot trip. `[inferred — audit]`: I did not render a frame. By construction the popup is small, but no 80x24 `clipped == false` assertion was added. |
| WR-02: "branch namespace" claimed by the prompts | RESOLVED in the code. `prompt_text` now says "PR caps" only, and the module doc states the namespace is parsed but not consulted. `docs/CONFIGURATION.md` was not re-verified. |
| DarkGray contrast on the consent text | RESOLVED (text deleted). |
| WR-03: unregister had no live-run guard | RESOLVED with caveats. `delete_confirm.rs:121-142` refuses when `ctx.experimental` and the cached `launcher_marker.any_live` hold. Two tests cover refusal, an unchanged config, and the not-live case. |
| IN-03: opt-in popup dims the dashboard | STILL OPEN. `driver_confirm.rs:186-195` fixed it for the kill popup only (DetailScreen behind `StopLauncher`). `ToggleOptIn` is reachable only from the Driver tab `o`, yet it still renders `NormalScreen`. The comment calls this deliberate, which is wrong since 31-02 removed the dashboard entry. |
| IN-04: "legacy driver, removed in Phase 31" on Preferences | STILL OPEN (`preferences.rs:89`). |
| `driver seam` jargon in an error | STILL OPEN (`driver_confirm.rs:267`). |
| `INJECTION_RETIRED`, `DRY_RUN_LOADING`, `help.rs:105` wording | STILL OPEN (cosmetic). |
| `o` missing from the Driver footer | STILL OPEN. |

---

## Top 3 Priority Fixes

1. **Make the unregister guard airtight and accurate** — a live run can still be orphaned, and the refusal text points at the wrong key. Handle the three cases below in `src/ui/screens/delete_confirm.rs:121-142`:
   - The guard fails open: the marker is `None` before the first scan lands. It is also gated on `ctx.experimental`, so a run left over from an earlier session with the flag off passes silently.
   - It fires after the user has already pressed `y`. Disable the `y` path, or show the live-run state in the popup text, so the refusal arrives as the prompt rather than after it.
   - The text says "Stop it from its Driver tab (x)", but `x` opens a kill confirmation. The gentler path is `[c]` then stop.
2. **Render the Driver tab behind the opt-in popup** — grant and withdraw flip the user from the Driver tab to the dashboard for the duration of the prompt. In `src/ui/screens/driver_confirm.rs:186-195`, use `DetailScreen::render_main_only` for both actions and drop the stale "as it did when the dashboard also opened it" comment.
3. **Sweep leftover internal vocabulary and add `[o]` to the footer** — new users see phase numbers and retired-feature names. Replace `preferences.rs:86-89` with "(none: this build has no manager-wide settings)", reword `driver_confirm.rs:267` to drop "seam", unbind or reword `INJECTION_RETIRED`, add `[o]pt-in` to the Driver footer (`detail.rs` footer builder), and rename `DRY_RUN_LOADING`.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- The grant prompt, "Record a driver opt-in for "{alias}"? The push step reads its PR caps from it; without one it uses the defaults. [y/n]", is short and accurate. The withdraw prompt matches ("Its PR caps stop applying"). Both are free of unbacked safety claims (T-17-48).
- The refusal message ("'{alias}' has a live driver run. Stop it from its Driver tab (x) first.") names the alias and renders it through `render_for_terminal`. It is actionable, but see fix 1: `x` is the kill key, not a plain stop.
- Open: `preferences.rs:89` internal phase number, `driver_confirm.rs:267` "driver seam", `driver_run.rs` `INJECTION_RETIRED`, and the docs changelog paragraph. The earlier "Copywriting" blocker (retired consent text) is gone, hence 2 to 3.

### Pillar 2: Visuals (3/4)
- The popup is a compact prompt plus buttons, with an accent colour per direction. The earlier wall of text is gone.
- The kill popup keeps the Driver tab behind it (good). The opt-in popup does not (IN-03, open).
- The "MANAGER PREFERENCES" block is still a heading plus "(none...)" with nothing interactive (`[inferred — audit]`: collapse it).
- `o` is not in the Driver footer, though help documents it.

### Pillar 3: Color (4/4)
- Red covers kill, withdraw and remove-project. Yellow marks the one additive action (grant). No new colours were introduced by 31-08, and the lowest-contrast DarkGray text on the consent step is deleted.
- The open project-wide DarkGray todo is out of phase scope. No contract defect remains in the touched screens.

### Pillar 4: Typography (3/4)
- The bold heading, key-column and dim-gloss hierarchy is intact. The confirm popup title and body use the same pattern as the other confirms.
- Stale names only: `DRY_RUN_LOADING`, and "legacy needs-a-human predicate" at `help.rs:105`.

### Pillar 5: Spacing (3/4)
- The popup no longer depends on the 72-column and `body_rows + 5` sizing edge cases. Removed sections left no gaps (31-03 behaviour retained).
- Open: the Preferences placeholder is hand-wrapped with an unbalanced parenthesis across two lines (`preferences.rs:85-89`), so it will not reflow on narrow terminals.

### Pillar 6: Experience Design (3/4)
- Restored: the refusal guard, tested for both live (refused, config untouched, nothing written) and not-live (removed, status reported). It also respects the `Crashed` marker, which holds no lock.
- Gaps, all `[inferred — audit]` from reading the diff:
  - The guard reads a cached marker that is at most one slow tick stale, and is `None` before the first scan.
  - It is skipped when the experimental flag is off.
  - The refusal fires after the confirm popup rather than before it.
  - Help's `d` row ("Delete project", `help.rs:231`) carries no warning.
- Post-31-07: removing the dead exec-event channel leaves the pump with two arms (actions, redraw timer), so no UI state can be driven by an orphaned event source. This is a structural plus, but it changes no user-visible behaviour, so the score stays 3. `[inferred — audit]`
- Unchanged and good: Loading, empty and list states on the Driver tab, and inert retired keys with regression tests.
- Still open: the `o` footer omission, and the note that a legacy `drive` process running at upgrade is invisible to the TUI. The note lives only in a planning document and belongs in release notes.

---

## Files Audited
- `src/ui/screens/driver_confirm.rs` (prompts, render, background choice, 31-08 diff, new test)
- `src/ui/screens/delete_confirm.rs` (31-08 refusal and its tests)
- `src/ui/screens/preferences.rs`, `help.rs`, `detail.rs`, `driver_run.rs`, `driver.rs`, `confirm_popup.rs`, `normal.rs` (carried over from the prior review; spot-checked for changes)
- `src/ui/mod.rs`, `render_escape_guard.rs` (census pins only, bookkeeping)
- Re-verified at HEAD for the post-31-07 pass: `driver_confirm.rs:186-195, 267`, `delete_confirm.rs:121-142`, `preferences.rs:85-89`, `help.rs:105, 370`, `detail.rs` footer tokens (`[c]`, `[x]` present; no `[o]`), `driver_run.rs:2640`
- Phase artifacts: the prior `31-UI-REVIEW.md`, plus the SUMMARY, PLAN and CONTEXT files listed in the request
