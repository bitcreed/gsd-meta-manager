# Phase 31 — UI Review

**Audited:** 2026-10-25
**Baseline:** abstract 6-pillar standards (no UI-SPEC.md), interpreted for a ratatui TUI (colour = style semantics, typography = bold/dim/heading hierarchy and glyph+label legends, spacing = layout, popup sizing and blank-line structure)
**Screenshots:** not captured (terminal UI, no dev server; code-only audit). Plan 31-07 is HELD and not scored as missing.
**Registry audit:** skipped (no `components.json`; not a web UI).
**Method note:** the standing code review (`31-REVIEW.md`) was read first. Every UI-relevant finding in it (WR-01, WR-02, WR-03, IN-03, IN-04) was re-verified against the source here, not copied. Findings marked `[inferred — audit]` are judgments from the artifacts, with no human in the loop.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 2/4 | The opt-in disclosure and both opt-in prompts still describe controls that no longer exist (model seams, re-confirmation, branch namespace); an internal phase number is shown to users |
| 2. Visuals | 3/4 | The Driver tab and dashboard closed cleanly around the removed sections; the Preferences screen is now a placeholder panel, and the opt-in popup dims the wrong background |
| 3. Color | 3/4 | Red = destructive, Yellow = grant, consistent across the surviving confirm popups; the consent disclosure is DarkGray, the lowest-contrast style, on the text that matters most |
| 4. Typography | 3/4 | Heading, bold-key and dim-gloss hierarchy intact; help rows were removed with their blank-line invariants preserved; leftover `DRY_RUN_LOADING` naming is cosmetic |
| 5. Spacing | 3/4 | No layout gaps from the deleted legacy section; the grant popup's body is far taller than a normal terminal, and the Preferences placeholder is hand-wrapped at fixed columns |
| 6. Experience Design | 2/4 | The live-run guard on project removal was deleted and not re-aimed at launcher runs; the opt-in grant popup is probably unconfirmable on ordinary terminal heights; `o` is not in the Driver footer |

**Overall: 16/24**

---

## Top 3 Priority Fixes

1. **The opt-in grant popup shows retired consent text and, by my estimate, cannot be confirmed on ordinary terminals** — a user is asked to consent on the strength of controls that do not exist, and then may be unable to press Yes — in `src/ui/screens/driver_confirm.rs`, delete `SECTION_MODEL_SEAM`, `SECTION_RESIDUAL_EXPOSURE` and the per-file list from `render_disclosure` (or replace them with one sentence stating what the record feeds: PR caps only). Remove "branch namespace" from `prompt_text` (lines 237, 252), the module doc (:14) and `docs/CONFIGURATION.md`. Add a render test that asserts the grant popup is not `clipped` at 80x24 (the current test at :1250 renders at 110x40 and only checks that the button labels are drawn, which happens even when clipped). Covers WR-01, WR-02 and the clipping risk in one change.
2. **Removing a project with a live launcher run is unguarded** — the `d` + `y` path drops the project from the launcher-marker scan while the detached launcher and its sandbox scope keep running, and the TUI then shows nothing about it — re-aim the deleted guard in `src/ui/screens/delete_confirm.rs:121` (`do_remove_project`) at `launcher_marker.any_live`, as drafted in `31-REVIEW.md` WR-03, and restore a refusal test. Alternatively record an explicit decision that removal with a live run is allowed. (WR-03; this is the only destructive-action confirmation that regressed.)
3. **Sweep user-visible copy that names retired or internal things** — a first-time user sees "legacy driver, removed in Phase 31" on Preferences, "Injection is retired" on `i`, and "Cannot reach the driver seam" on a closed channel; docs still describe the legacy `drive`. Concretely: `preferences.rs:86-89` becomes "(none: this build has no manager-wide settings)" or drops the section (IN-04); `driver_run.rs:2640` `INJECTION_RETIRED` is reworded or the `i` key is simply unbound; `driver_confirm.rs:409` loses "seam"; the `docs/CONFIGURATION.md:149` paragraph is trimmed to a one-line changelog note.

Further fixes, lower priority (detail below): add `[o]` to the Driver footer or document its absence; render `DetailScreen` behind the opt-in popup (IN-03); rename `DRY_RUN_LOADING`; replace the "legacy needs-a-human predicate" wording in `help.rs:105`.

---

## Detailed Findings

### Pillar 1: Copywriting (2/4)

**What is clean (verified):**
- The help screen has no `Injected Message States` legend, no `i` row, no `r`/`x`/`o` dashboard rows and no legacy-runs toggle. The comment at `help.rs:236-237` says why, and `help.rs:763-799` and `:1089-1097` pin it with the flag on and off.
- The Driver footer names no retired token. `detail.rs:11171-11250` builds only tokens whose key is live for the current selection, and `detail.rs:18050-18073` asserts the absence of `[i]nject`, `[X]`, `[{/}]`, `[x]stop` and `[L]egacy`.
- The kill prompt (`driver_confirm.rs:240-249`) is accurate, names the run and alias, and points at the gentler `[c]` then stop path.
- The Driver empty state (`driver_run.rs:3012-3032`) is actionable ("Press [s] to start one: name it, edit its DIRECTION, check it, then launch.") and has a non-Linux variant.

**Defects:**
- **BLOCKER-class copy (WR-01, verified).** `driver_confirm.rs:43-89` `SECTION_PROMPT_INPUTS`, `SECTION_MODEL_SEAM` and `SECTION_RESIDUAL_EXPOSURE` are still rendered by the grant popup (`:333`, `:352-366`). They state that "the goal-decomposition and ambiguity seams" exist and that "the digest and the re-confirmation above exist". Phase 31 deleted the goal layer (D-07) and `drive`; the re-confirmation has no caller. The file's own doc (`:15-16`, T-17-48) says "no safety claim the code does not back". This is the last text read before recording consent. It also contradicts the one-line prompt directly above it.
- **Unbacked claim (WR-02, verified).** `driver_confirm.rs:237` ("Its PR caps and branch namespace stop applying"), `:252` ("reads its PR caps and branch namespace from it") and `:14`. `31-REVIEW.md` shows `push.rs` reads only the PR caps. The failure direction is safe (stricter default namespace), but a user who sets a namespace is told it applies. `docs/CONFIGURATION.md:144-145` repeats it.
- **Internal phase number shown to users (IN-04, verified).** `preferences.rs:89`: "legacy driver, removed in Phase 31)". Users have no frame of reference for it, and it will go stale. The two-line placeholder is also split by hand with an unbalanced closing parenthesis across the lines.
- **Retired-feature vocabulary persists.** `driver_run.rs:2640` `INJECTION_RETIRED` = "Injection is retired: use [c]ontrol, [a]nswer, [e]dit." Deliberate (Phase 30 D-02) and tested, but the injection screen is now gone from the codebase entirely, so the message refers to something a new user never saw. `[inferred — audit]`: acceptable for one release as a pointer for existing users; drop it at the next opportunity.
- **Jargon in an error.** `driver_confirm.rs:409` "Cannot reach the driver seam for ... the event channel is closed". "Seam" is internal vocabulary.
- **Docs.** `docs/CONFIGURATION.md:149-152` is a changelog entry, not configuration documentation, and names Phase 31.

### Pillar 2: Visuals (3/4)

- **Driver tab (good).** `render_launcher_tab` (`driver_run.rs:3035+`) now has exactly three states: Loading, empty ("No driver runs yet ..."), and list + detail. Removing the legacy section left no orphan header, divider or blank band; the list and detail panes own the full area. The `Loading...` state is deliberately distinct from the empty state, so it does not claim there are no runs before the first scan lands.
- **Focal point / hierarchy.** Run list, then selected run header, then item rows. Preserved by 31-03; the shared helpers Phase 30 imports (`driver_panes`, `pane_header_line`, `follow_indicator`) were kept.
- **Defect, background mismatch (IN-03, verified).** `driver_confirm.rs:309-313` renders `NormalScreen` (the dashboard) dimmed behind the opt-in popup. After 31-02 the only entry to that popup is `detail.rs:5987-5991` (Driver tab `o`), so the user's Driver tab is replaced by the dashboard for the duration of the prompt. One-line fix: use `DetailScreen::render_main_only` for both actions.
- **Defect, placeholder panel.** `preferences.rs:77-107`: the "MANAGER PREFERENCES" section is now a heading plus a "(none ...)" note, kept at the top of the screen, with nothing to interact with. It is honest (the tests pin that no key edits anything), but it is a dead block. The G key description in help ("Global settings: the GSD global template") already describes the only live part. `[inferred — audit]`: collapse the manager section.
- **Defect, discoverability.** `o` (opt-in toggle) is documented in help (`help.rs:370`) but is not a Driver footer token (`detail.rs:11171-11250`). It is the only live Driver-tab key missing from the footer.
- **Cosmetic.** The generic loading string is still named `DRY_RUN_LOADING` (`driver.rs:98`, used at `driver_run.rs:990, 1724, 2101, 3075, 3191`) after the dry-run preview was deleted (IN-05).

### Pillar 3: Color (3/4)

Terminal styling, not a web palette. Counts are for the surviving confirm popups and help legends.

- **Semantic use is consistent.** `Color::Red` for removal (kill launcher run, withdraw opt-in, remove project: `delete_confirm.rs` and `driver_confirm.rs:261-266`), `Color::Yellow` accent for the one additive action (grant: `:358`), `Color::Reset` for the prompt body when granting. Magenta = driven badge, Red = needs-a-human or crashed, Yellow = stale or unknown (`normal.rs:202-209`). No hardcoded hex or RGB values in the touched screens.
- **Defect, contrast on the consent text.** `driver_confirm.rs:349-353` renders the whole disclosure in `Color::DarkGray`. DarkGray is the lowest-contrast style in the palette, used here for text the module calls "the last thing an operator reads before an irreversible act". An open todo covers DarkGray ambiguity project-wide (`2026-10-21-ambiguous-wide-typography-borders-and-darkgray.md`, out of phase scope), so this is flagged, not scored down further. If fix 1 removes the disclosure, this finding disappears with it.
- **Accent not overused.** The destructive red is confined to the three confirm directions and the badge set; no new colours were introduced by the phase.

### Pillar 4: Typography (3/4)

- Hierarchy is intact and stable: bold headings (`help.rs:189-194`), a fixed `{key:<13}` key column (`:196-198`), dim footer lines, and a glyph+label legend where no meaning rides on colour alone (`help.rs:33-37`).
- **Removed rows did not break structure.** The help body keeps "exactly one blank row" between sections (comments at `:266-270, :292-295, :306-309, :330-333`), and the Driver section block is the only thing the flag gates. Verified by reading `help_lines(experimental)` end to end.
- The Driver section's Pitfall-7 continuation of the `e` row uses three key-less rows (`help.rs:356-358`), which render as an indented block. It is correct but is the only wrapped description in the section; the others are unwrapped single rows. Cosmetic.
- Stale vocabulary only: `help.rs:105` "the legacy needs-a-human predicate" (the predicate is the non-driver one, not a removed legacy feature), and the `DRY_RUN_LOADING` constant name.

### Pillar 5: Spacing (3/4)

- **Removed sections left no gaps.** Driver tab: see Pillar 2. Help: blank-line invariants preserved. Dashboard: the `r`/`x`/`o` row-less footer is simply shorter. `31-03-SUMMARY` re-measured the UI census pins (`MEASURED_REACH`, `UI_SOURCE_FLOOR`) and they hold.
- **Defect, popup height.** `confirm_popup.rs:169-184` caps the popup at 72 columns wide and `body_rows + 5` tall, and refuses confirmation when the body is cut off (`:283-285`, `CLIPPED_HINT`: "Cut off: enlarge the terminal to confirm"). The grant popup body is: prompt (2 rows) + blank + file-set header (about 5) + 5 files at 2 rows each (10) + blank + model-seam block (about 11) + blank + residual block (about 12), which I estimate at about 43 body rows and about 48 popup rows at 72 columns. That is taller than 24, 30 and 40-row terminals. `[inferred — audit]`: the figure comes from a word-wrap estimate of the shipped constants, not a rendered frame; confirm with a TestBackend render at 80x24 and 110x40 asserting `clipped == false`. Note the existing test `every_driver_confirmation_renders_as_a_popup_with_buttons` (`driver_confirm.rs:1250-1275`) renders the grant popup at 110x40 and asserts the buttons exist, which does not detect clipping: the buttons are drawn whenever two inner rows exist (`confirm_popup.rs:270-279`).
- **Preferences placeholder** is hand-wrapped at roughly 62 columns (`preferences.rs:85-89, 97-104`) rather than relying on `Paragraph::wrap`, so it will not reflow on narrow terminals. Minor.

### Pillar 6: Experience Design (2/4)

State coverage that survived and is good: Loading vs empty vs list on the Driver tab; refusal copy for unreadable runs and non-Linux steering (`driver_run.rs:2747-2758`); retired keys (`L`, `i`, dashboard `r`/`x`/`o`) are inert with regression tests that select a project and run with the flag on and off (`normal.rs:2312`, `detail.rs:20804`); the kill confirm names the run and the whole scope and offers the gentler alternative; the clipped-popup guard exists in principle.

**Defects:**
- **Destructive-action guard lost (WR-03, verified by diff).** `delete_confirm.rs` previously refused removal when the project had an observed run whose liveness was not `Dead` (CR-06/D-11/D-27, tests `unregistering_a_project_with_a_live_run_is_refused_and_leaves_every_map_intact`). 31-03 deleted it because the TUI "can no longer observe a legacy run", but the replacement signal exists: `launcher_marker.any_live` / `MarkerClass::Driven`, already used by the dashboard badge. Today `d` then `y` unregisters a project, which drops it out of the launcher-marker scan (`launcher_markers_for` iterates `config.projects`), while the detached launcher and its sandbox scope keep running with push rights and no badge anywhere. Only `driver stop <path>` from a shell recovers it. The help text for `d` ("Delete project") carries no warning either.
- **Likely-unconfirmable consent step.** See Pillar 5. The grant popup's success path (`y`) is blocked below roughly 48 rows, and the only on-screen signal is a yellow hint line at the bottom of a popup whose body is cut off. Because the disclosure text is also factually stale (Pillar 1), the best fix is removal rather than scrolling.
- **Operator note not surfaced.** 31-03's summary records that a legacy `drive` process still running at upgrade is invisible to the TUI and cannot be stopped from it. That is recorded only in a planning document. `[inferred — audit]`: acceptable for a hidden, experimental, flag-gated surface, but it belongs in the release notes.
- **Footer omits a live key.** `o` (see Pillar 2).
- **Loading and error states** on the touched screens: not regressed; no new async path was added by this phase.

---

## Files Audited

- `src/ui/screens/driver_confirm.rs` (prompts, disclosure, render, background choice, tests)
- `src/ui/screens/confirm_popup.rs` (popup sizing and clipped-guard)
- `src/ui/screens/help.rs` (help body, legends, badge legend, retired-row tests)
- `src/ui/screens/detail.rs` (Driver footer builder, `i`/`o`/`c` key handlers, retired-key tests)
- `src/ui/screens/driver_run.rs` (launcher tab, empty state, steer refusal, `INJECTION_RETIRED`)
- `src/ui/screens/driver.rs` (remaining shared helpers, `DRY_RUN_LOADING`)
- `src/ui/screens/normal.rs` (`row_badge`, badge ranking, retired-key regression)
- `src/ui/screens/delete_confirm.rs` (`do_remove_project`, diff against `81c2ad7`)
- `src/ui/screens/preferences.rs` (placeholder panel)
- `docs/CONFIGURATION.md`, `README.md` (user-facing claims about the opt-in and the retired keys)
- Phase artifacts: `31-CONTEXT.md`, `31-01` through `31-06` SUMMARY files, `31-REVIEW.md`, `31-REVIEW-DISPOSITION.md`
