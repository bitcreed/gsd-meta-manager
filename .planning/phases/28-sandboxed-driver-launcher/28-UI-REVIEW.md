---
phase: 28
phase_name: Sandboxed Driver Launcher
audited: 2026-10-23
baseline: abstract 6-pillar standards (TUI-adapted; no UI-SPEC.md for Phase 28)
screenshots: not captured (ratatui TUI, no dev server; code + test-source audit only)
interaction_capture: false
overall_score: 18/24
blockers: 0
---

# Phase 28 — UI Review

**Overall: 18/24** — no BLOCKER findings; all findings are WARNING.

## Scope

Phase 28's own Driver-tab section from 28-07 (`Launcher runs (n)  [{/}]select  [X]stop`) was retired by Phase 30 (retired keys asserted gone at `src/ui/screens/detail.rs:18181`). The surviving Phase-28 UI surface audited here:

- `UNSANDBOXED` posture badge and `!` row marker — `src/ui/screens/driver_run.rs`
- `Launch UNSANDBOXED` confirmation — `src/ui/screens/driver_launch.rs`
- `Kill driver run` prompt — `src/ui/screens/driver_confirm.rs`
- CLI refusal text — `src/launcher/mod.rs:154`

`[INFERRED — audit]` Audited the current re-hosted surface rather than the 28-07 excerpt, since Phase 30 replaced it.

## Pillar Summary

| Pillar | Score | Key finding |
|--------|-------|-------------|
| Copywriting | 3/4 | Refusals are specific and actionable; launch-success line is silent about posture; `!` and "bwrap + systemd scope" are jargon |
| Visuals | 3/4 | Posture shown at three levels (row, token, header); `!` is not a real column and has no help legend |
| Color | 3/4 | Yellow Bold matches D-30; Yellow carries ~8 meanings; severity flips Red (asking) to Yellow (running) |
| Typography | 3/4 | BOLD/DarkGray roles consistent; load-bearing `missing:` list is de-emphasised; label casing inconsistent |
| Spacing | 3/4 | Width tiers protect the badge; `!` hugs the id; unsandboxed popup is one long paragraph |
| Experience Design | 3/4 | Strong guard rails (double confirm, default No, nothing remembered, clipped popup keeps `y` inert); state-feedback gaps |

## Findings by Pillar

### Copywriting (3/4)
- Good: CLI refusal "sandbox unavailable: missing X; rerun with --unsandboxed" (`src/launcher/mod.rs:154`); form "Sandbox unavailable: missing …" with "[U] launch UNSANDBOXED instead (asks again)" (`driver_launch.rs:832`); kill prompt honestly states in-flight work is not reported and points to `[c]` then stop.
- WARNING: `UnsandboxedConfirmScreen` prompt (`driver_launch.rs:1085`) is a single ~230-char sentence in one Red paragraph.
- WARNING: `finish_launch` (`src/app.rs:1762`) reports "launched <id>. It runs detached…" even for unsandboxed launches — the posture is not restated.
- WARNING: "Stop launcher run" (28-07), "Stop driver run" (legacy `Stop`), "Kill driver run" (`StopLauncher`, `driver_confirm.rs:510`) — wording drift across two distinct actions.

### Visuals (3/4)
- Good: escalating posture hierarchy — `!` row cell (`driver_run.rs:327`), `UNSANDBOXED` token (`:2140`), header row 4 with `missing:` list (`:1048`).
- WARNING: `!` is not a fixed column — id is cut by `fit_cells` but not padded, so a short id renders `20261020-x!` and reads as part of the id (comment at `:276` calls it a "posture column").
- WARNING: `!`, `UNSANDBOXED`, `cap!`, `stale` absent from the help legend (`help.rs:387` documents only `RUN_STATES_LEGEND` and `ITEM_STATES_LEGEND`).

### Color (3/4)
- `Color::Yellow` used 33x in `driver_run.rs` and 5x in `driver_launch.rs`/`driver_confirm.rs` — posture, stale, cap, paused, unreadable, unknown word, custom prompt, clipped hint, lint warnings, launch cost warning.
- WARNING: `posture_style()` (`driver_run.rs:261`) and `flag_style()` (`:2046`) are byte-identical — drift risk; no palette/theme module in `src/ui`.
- WARNING: unsandboxed launch confirm is Red, persistent posture afterwards is Yellow. `[INFERRED — audit]` Accepted as the D-30 "warning style" contract; flagged only as a severity-consistency risk.
- Meaning never carried by color alone (word `UNSANDBOXED` always present), except the lone `!`.

### Typography (3/4)
- Good: BOLD reserved for flags/selection/titles; DarkGray for labels, consistently.
- WARNING: `missing: bwrap, systemd-run` drawn in `label_style()` (`driver_run.rs:1063`) — the one actionable posture fact is the quietest text on the row.
- WARNING: "Sandbox:" (`driver_launch.rs:506`) vs lowercase "sandbox" (`driver_run.rs:1049`).

### Spacing (3/4)
- Good: `fit_cells` + `SUMMARY_ID_MIN_CELLS` (12) and `RUN_ROW_WORD_MIN_CELLS` (36) keep the badge unclipped; tested at 30, 40 and wider.
- WARNING: unpadded `!` (see Visuals).
- WARNING: launch-form essentials stack without separators; on short terminals the popup clips and `y` goes inert, dead-ending at "Cut off: enlarge the terminal" (by design, `confirm_popup.rs:67`; safe but user-visible).

### Experience Design (3/4)
- Good: second confirmation for unsandboxed launch; focus defaults to No; `Replace` leaves no submittable form; choice not remembered (`args.unsandboxed` set only on confirm); hostile identity escaped and guarded by `render_escape_guard`; row status from the run lock, not the written word; preflight "Sandbox: Loading..." state exists.
- WARNING: `U` missing from `KEY_ROW` ("[j/k] field  [e] edit field", `driver_launch.rs:509`) — discoverable only from the Red line.
- WARNING (unverified): no timeout/error state found for a preflight that never arrives; row could read "Loading..." indefinitely. Whether `y` is inert in that state was not confirmed.

## Top Fixes

1. **Cryptic, unlegended `!`** — pad the id so `!` sits in a fixed last column; add a "Run row marks" legend line to `help.rs` covering `!`, `UNSANDBOXED`, `cap!`, `stale`; or replace `!` with `U`/`⚠` via width-safe `glyphs.rs`.
2. **De-emphasised `missing:` list** — render in default style or Yellow non-bold in `header_row_4` (`driver_run.rs:1063`).
3. **Silent unsandboxed launch success** — `finish_launch` (`app.rs:1762`) should say "launched <id> UNSANDBOXED", threading `args.unsandboxed` through `DriverLaunched`.
4. **Wall-of-text confirmation** — split `UnsandboxedConfirmScreen::render` into three Red lines: headline, consequence ("full user permissions, no bwrap, no scope"), "Missing: …".
5. **Overloaded Yellow / duplicate helpers** — collapse `posture_style` and `flag_style` into one or a small named palette (warn, flag, label); reconsider sharing Yellow between persistent security posture and transient `stale`.

### Minor
- Add `[U]` to the form key row when the host cannot sandbox.
- Reconcile "Stop" vs "Kill" naming across confirmations.
- Add/verify a failure state for a hung preflight.
- Align "Sandbox:" / "sandbox" casing.

## Method Notes
- No `cargo test` run (concurrent timing-sensitive tests); rendered output read from test source and the 28-07 SUMMARY.
- Gitignore/screenshot gate skipped — no screenshots captured.
- Report authored by gsd-ui-auditor; file written by the orchestrator from the auditor's returned text because the auditor's harness disallowed writing report files.
