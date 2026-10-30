---
phase: 28
phase_name: Sandboxed Driver Launcher
audited: 2026-10-29
baseline: abstract 6-pillar standards (TUI-adapted; no UI-SPEC.md for Phase 28)
screenshots: not captured (ratatui TUI, no dev server; code audit only)
interaction_capture: false
overall_score: 18/24
blockers: 0
re_audit_of: 03107c1
---

# Phase 28 — UI Review (re-audit at HEAD)

**Overall: 18/24** — no BLOCKER findings; all findings are WARNING. Score unchanged from ee65618; most prior findings persist, the Phase 28.1 work added useful posture detail but also more Yellow load and a longer confirmation paragraph.

## Scope

Surviving Phase-28 UI surface (Phase 30 retired the 28-07 section; Phase 31-03 made the Driver tab list launcher runs only):

- `UNSANDBOXED` token, `!` row marker, header row 4 (`PLANTED`, `absent:`, `missing:`) — `src/ui/screens/driver_run.rs`
- Launch form sandbox block and `Launch UNSANDBOXED` confirmation — `src/ui/screens/driver_launch.rs`
- Kill prompt — `src/ui/screens/driver_confirm.rs`; launch success line — `src/app.rs:1311`

`[inferred — cleanup 2026-10-28]` MacOS enablement (76a9ecb) only widened `cfg(target_os)` gates in `driver_launch.rs`; it changed no rendered copy or style, so it has no visual score impact.

## Pillar Scores

| Pillar | Score | Key finding |
|--------|-------|-------------|
| Copywriting | 3/4 | Form now names what an unsandboxed run would lack; success line still silent about posture; confirm prompt still one long paragraph |
| Visuals | 3/4 | Posture hierarchy intact and richer; `!` still unlegended and not a padded column |
| Color | 3/4 | Yellow now 38x in `driver_run.rs`, 4x in `driver_launch.rs`; duplicate style helpers remain |
| Typography | 3/4 | `missing:` list still drawn in the quietest style |
| Spacing | 3/4 | One cell reserved for the posture column, but a short id still abuts `!` |
| Experience Design | 3/4 | Guard rails intact; `[U]` still missing from key row; no hung-preflight state |

## Findings by Pillar

### Copywriting (3/4)
- Good: form copy "Sandbox unavailable: missing …" plus "A sandboxed launch is refused on this host." plus "Unsandboxed, the run would lack: …" plus "[U] launch UNSANDBOXED instead (asks again)" (`driver_launch.rs:~846-865`). This consequence line is new since ee65618 and a clear improvement.
- WARNING: `UnsandboxedConfirmScreen::prompt` (`driver_launch.rs:1148-1155`) is still one sentence in a single Red `Line` (`:1186-1190`), now also interpolating the absent-protection list, so it is longer than before.
- WARNING: `finish_launch` message `"{}: launched {}. It runs detached…"` (`src/app.rs:1311`) is unchanged and never says UNSANDBOXED.
- WARNING: "Stop" / "Kill" naming drift (`driver_confirm.rs`) not addressed.

### Visuals (3/4)
- Good: three-level posture hierarchy retained (`!` cell `driver_run.rs:~327`, `UNSANDBOXED` token `:2245`, header row 4 `:1041-1084` now with `PLANTED`, `absent:`, `missing:`).
- WARNING: `!` (`POSTURE_DEGRADED`, `:90`) has no help-legend entry; grep of the help screen finds no `UNSANDBOXED`/`PLANTED`/`!` documentation.
- WARNING: the id is `fit_cells`-clamped to `room` but not padded (`:~318`), so a short id still renders `id!`. Only the non-degraded case pads with a space.

### Color (3/4)
- Yellow usage: 38 in `driver_run.rs`, 4 in `driver_launch.rs`; it now also carries `PLANTED`, `absent:` and the unsandboxed-would-lack line.
- WARNING: `posture_style()` (`driver_run.rs:254`) and `flag_style()` (`:2151`) remain byte-identical (Yellow + BOLD).
- WARNING: Red (asking, `driver_launch.rs:1189`) vs Yellow (persistent) severity flip unchanged; accepted as the D-30 warning-style contract `[inferred — cleanup 2026-10-28]`.

### Typography (3/4)
- Good: BOLD/DarkGray roles consistent.
- WARNING: `missing:` still uses `label_style()` (`driver_run.rs:1077`, `:1082`) while the less critical `PLANTED` token gets Yellow.
- WARNING: "Sandbox:" (`driver_launch.rs:526-528`) vs lowercase "sandbox" in the run header.

### Spacing (3/4)
- Good: width tiers still protect the badge; 28.1-13 (8cea0d7) keeps absent protections visible on narrow Driver tabs, and the row reserves one posture cell (`driver_run.rs:~314`).
- WARNING: unpadded `!` (see Visuals).
- WARNING: form stack grew by up to three lines in the degraded case (missing, refusal, would-lack, `[U]`), raising clip risk on short terminals where the popup goes inert by design.

### Experience Design (3/4)
- Good: double confirmation, focus defaults to No, nothing remembered (`args.unsandboxed = true` only on confirm, `driver_launch.rs:1177-1180`), escape guards, run-lock-derived status.
- WARNING: `KEY_ROW` is still `"[j/k] field  [e] edit field"` (`driver_launch.rs:531`); `U` is discoverable only from the Yellow line.
- WARNING (unverified): a preflight that never arrives still shows `Sandbox: Loading...` (`:526`) with no timeout state; whether `y` is inert there was not confirmed.
- No regression from macOS enablement: the same screens render on both platforms.

## Top 3 Fixes

1. **Unlegended, unpadded `!`** — pad so `!` sits in a fixed last column (`driver_run.rs:~318`); add a run-row marks legend (`!`, `UNSANDBOXED`, `PLANTED`, `cap!`, `stale`) to the help screen.
2. **Wall-of-text unsandboxed confirm and silent success** — split `prompt()` (`driver_launch.rs:1148`) into headline, consequence and Missing/absent lines; make `app.rs:1311` say "launched <id> UNSANDBOXED" by threading `args.unsandboxed` through `DriverLaunched`.
3. **De-emphasised `missing:`** — render in default or Yellow non-bold (`driver_run.rs:1077/1082`); also collapse `posture_style`/`flag_style` and add `[U]` to `KEY_ROW` when the host cannot sandbox.

Minor: Stop vs Kill naming; hung-preflight failure state; "Sandbox:" casing.

## Delta vs prior audit (ee65618)

| Pillar | Prior | Now | Change |
|--------|-------|-----|--------|
| Copywriting | 3 | 3 | none (form copy better, prompt longer) |
| Visuals | 3 | 3 | none |
| Color | 3 | 3 | none (Yellow load up) |
| Typography | 3 | 3 | none |
| Spacing | 3 | 3 | none |
| Experience Design | 3 | 3 | none |
| **Overall** | 18 | 18 | 0 |

**Regressions (minor, no score change):**
- `driver_launch.rs:1148-1155` confirm prompt lengthened by the absent-protection list, worsening the wall-of-text finding.
- `driver_run.rs` Yellow count 33 to 38 (`PLANTED`/`absent:`), deepening overload of one color.
- Degraded form block is up to three lines taller (`driver_launch.rs:~846-865`), raising popup-clip risk.

**Prior findings resolved / improved:**
- Reader was told only what was missing; the form now also states the consequence ("Unsandboxed, the run would lack: …") and that a sandboxed launch is refused (28.1-06, 8a35d8d).
- Header row 4 now surfaces `PLANTED` and `absent:` protections ahead of `missing:`, and narrow tabs keep them (28.1-13).
- Posture column is now explicitly reserved one cell in the row layout (partial fix for the `!` crowding).
- Phase 31-03 removed the legacy scan paths; no leftover legacy Driver-tab copy found in the two audited files.

**Still open from prior:** unlegended `!`, `finish_launch` silence, single-paragraph confirm, duplicate Yellow helpers, `missing:` de-emphasis, `[U]` absent from key row, "Sandbox:" casing, Stop/Kill drift, hung-preflight state.

## Delta vs prior audit (03107c1)

Verified `git diff --stat 03107c1..HEAD -- src/ui src/app.rs` is empty: no rendered UI changed. Delta is 8 non-UI files under `src/` (launcher, driverctl, driver, state_reader) plus tests. All scores unchanged: 3/3/3/3/3/3 = 18/24, 0 blockers.

Anchors re-checked at HEAD, all still valid: `app.rs:1311` (launch success line, still no UNSANDBOXED), `driver_launch.rs:531` (`KEY_ROW`), `:1148` (`prompt`), `driver_run.rs:254` / `:2151` (`posture_style` / `flag_style`, still duplicated).

User-facing text candidates:
- `driver_frontmatter.rs:834/851/861` "not a regular file (a symbolic link)": returned as an io error from the guarded read of run files; the TUI Driver tab consumes it as a missing/unreadable run (no new rendered string in `src/ui`). CLI `driver stop`/`launch` surfaces it through `stop.rs` messages; wording is specific and actionable. No score change.
- `store.rs` "could not open {path}: {err}": CorruptState detail, reaches only `driverctl serve` / CLI output. Names the path and OS error; adequate.
- `direction.rs:806/849` "{N} restore attempts did not settle": CLI/log text; states a count but no next step (WARNING, minor, Copywriting; does not move the 3/4).
- `config_dir.rs`: wrong-shape host hooks/permissions are dropped silently; no warn/eprintln emitted. WARNING (minor, Experience Design): a user whose host hooks are discarded gets no notice that sandbox config omitted them. `[inferred — cleanup 2026-10-28]` This is low severity, since dropping is the safe direction.

No security-relevant UI finding: the symlink/FIFO refusal is hardening and its messages leak only the path the user supplied.

Method: code-only; no cargo run.

## Method Notes
- Code-only audit (no TUI run, no `cargo test`); diff inspected for ee65618..HEAD on both files (12 commits).
- Line numbers marked `~` were read from snippets and may be off by a few lines.

## Delta vs prior audit (90516c0) — no re-audit

`[inferred — cleanup 2026-10-28]` Existing review kept (workflow's unattended path reuses it; the auditor was not re-spawned) because quick 261029-1dp has no user-visible TUI surface:
- `git diff --stat 90516c0..HEAD -- src/ui src/app.rs`: only `src/ui/screens/detail.rs` (doc comment + test assertion message in `mod tests`, ~21257-21277) and `src/ui/screens/mod.rs:1389-1393` (doc comment on `AppContext::experimental`). No rendered string, layout, style or key binding changed.
- f4cfa01 (SAFE-04): line-redacting stdout/stderr pump in `src/launcher/process.rs` plus wiring in `src/launcher/run.rs`. It changes what is written to `launch-N.jsonl`, which no `src/ui` code renders (the only `jsonl` reference in `driver_run.rs:5941` is a test fixture).
- ce86206: removes the dead `AgentRuntime` key (`config.rs`, `registry.rs`, `executor/runtime.rs`); `grep -rn AgentRuntime src/ui` is empty, so no screen read it.

Scores unchanged: 3/3/3/3/3/3 = 18/24, 0 blockers. Method: code-only, git diff + grep.
