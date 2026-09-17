# 260917-fko — CONTEXT (locked decisions + surface map)

Task: Gate driver/autonomous features behind a `GSDMM_EXPERIMENTAL_FEATURES` startup
flag and mark them EXPERIMENTAL in the TUI.

These decisions are **locked**. Do not revisit them; implement them.

---

## D1 — Where the flag is read, and how it is parsed

Read `GSDMM_EXPERIMENTAL_FEATURES` **once at startup**, in the single production
funnel `App::from_config` (`src/app.rs:469`), and store the resolved `bool` on
`AppContext` as a new field `experimental: bool` (`src/ui/screens/mod.rs:1225`).

**Truthy set (inferred decision — flag for audit).** This codebase has no existing
boolean env-var convention: every current env var is either presence-tested
(`TMUX`, `src/terminal_switch.rs:24`) or read as a string value (`VISUAL`/`EDITOR`
`src/main.rs:637-638`, `TERMINAL`, `HOME`, `GSD_MM_ENVELOPE_ROOT`
`src/envelope/mod.rs:210`). There is therefore nothing to copy. Adopt an explicit,
documented truthy set and put it behind ONE pure helper so there is one parse site:

```rust
/// Accepted truthy values, ASCII-case-insensitive, surrounding whitespace trimmed:
/// `1`, `true`, `yes`, `on`. Everything else — including unset, empty, `0`,
/// `false`, `no`, `off` and any unrecognised value — is OFF.
pub fn experimental_features_enabled_from(raw: Option<&str>) -> bool
pub fn experimental_features_enabled() -> bool   // reads std::env::var, calls the above
```

Not presence-testing, because `GSDMM_EXPERIMENTAL_FEATURES=0` must mean off. The
pure `_from` variant is what the tests drive — do **not** write tests that mutate
process env (`std::env::set_var` is unsound under the parallel test harness).

Name the env var as a `pub const EXPERIMENTAL_FEATURES_ENV: &str =
"GSDMM_EXPERIMENTAL_FEATURES";` so tests and docs reference one literal.

## D2 — Default (flag unset): the driver does not exist in the TUI

Hidden entirely, not visible-but-disabled. A user who never sets the flag must not
learn a driver exists.

## D3 — Flag set: the Driver tab is marked EXPERIMENTAL

The literal word `EXPERIMENTAL` must be visible on the Driver tab itself (the tab
pane's block title rendered by `render_driver_tab`, `src/ui/screens/driver.rs:979`)
and on the help screen's Driver section heading (`src/ui/screens/help.rs:221`).
The tab-strip label is width-constrained — leave `"D:Drive"` / `"D:Dr"` as-is there;
the marker requirement is satisfied by the pane title and the help heading.
No raw glyph in source — house rule, `normal.rs:55-70`.

## D4 — `driver_opt_in` / `driver_max_concurrent` are untouched

`RegisteredProject::driver_opt_in: Option<DriverOptIn>` (`src/config.rs:47`) and
`driver_max_concurrent` (`src/config.rs:303`, default fn `:268`, consumed at
`src/app.rs:1911` `admit(live, ..)`) keep their exact current semantics and
defaults. The env flag is a NEW OUTER layer above them. Do not weaken, bypass,
short-circuit or "simplify" either. With the flag set, opt-in is still required.

## D5 — The `drive` CLI subcommand is NOT gated (inferred decision — flag for audit)

`src/cli.rs:47` `Commands::Drive` and its dispatch arm `src/main.rs:264` stay
functional regardless of the flag. Reason: `src/driver/spawn.rs:68,131` re-executes
**this same binary** as `current_exe() drive …` to launch a run, and environment
propagation into that child is not guaranteed (the envelope/exec layer scrubs env
— see `src/executor/claude.rs:502`, `src/envelope/policy.rs:4172`). Gating the CLI
arm would risk breaking the TUI's own spawn path, which is a functional regression
for the very users who DID set the flag. The task's requirement is the TUI surface;
the CLI is the machine-facing spawn target, not a discovery surface. Record this in
SUMMARY.md under inferred decisions.

## D6 — Startup reconcile stays; only its user-visible reporting is gated

`src/main.rs:562` `reconcile_all` and `:571` `last_ended_outcomes` still run (cheap,
and keeps state coherent for a user who toggles the flag on). What is gated is
every surface that *shows* the result.

---

## Surface map — everything to gate (verified file:line)

### A. Tab plumbing — `src/ui/screens/detail.rs`
| line | what |
|---|---|
| `:323` | `TAB_COUNT: usize = 11` — becomes the **array capacity**; introduce `fn visible_tab_count(experimental) -> usize` returning 11 or 10 |
| `:328-341` | `TAB_LABELS_FULL` (`"D:Drive"` at `:339`) — slice to the visible count |
| `:345-347` | `TAB_LABELS_COMPACT` (`"D:Dr"` last) — same |
| `:350` | `DRIVER_TAB_INDEX: usize = 10` |
| `:365` | `TAB_BAR_FULL_CELLS: u16 = 107` — must be **96** without the Driver tab (107 − 1 divider − `"D:Drive"`(7)+2 pads − 1 marker cell) |
| `:369` | `TAB_BAR_COMPACT_CELLS: u16 = 77` — must be **66** without it (77 − 1 − 4+2 − 1... verify the arithmetic against the doc comment formula `Σ(len+2) + (n−1)` and re-derive rather than guessing) |
| `:419` | `tab_titles(width, active, driver_live)` — add the flag; clamp `active` to the visible count; all three tiers |
| `:448` | `tab_label_line` |
| `:463` | `compact_label_cells` |
| `:473` | `windowed_tab_titles` — `TAB_COUNT` bounds become visible-count bounds |
| `:682` | `tab_index()` → `DRIVER_TAB_INDEX` |
| `:700` | `sub_view_from_index()` arm `10 => DetailSubView::Driver` — must fall back to `PhaseList` when off |
| `:1256` | `switch_to_tab` schedules `ctx.schedule_run_list_scan` on Driver entry |
| `:2060-2070` | digit arms `'1'..'0'` → tabs 0..9 (no driver digit — unaffected) |
| `:2071` | **`Shift+D`** jump to Driver tab |
| `:2074-2087` | `Left`/`Right` nav; `Right` guard `current_idx < TAB_COUNT - 1` must use the visible count |

### B. Driver-tab key arms — `src/ui/screens/detail.rs`
`:1627` `j`/Down, `:1738` `k`/Up, `:1899` `PageDown`, `:2033` `PageUp`,
`:2901` `f`, `:2914` `G`, `:2947` `i` (pushes `DriverInjectScreen` `:2964`),
`:2991` `s` (pushes `DriverStartScreen` `:2993`), `:3004` `x` (pushes
`DriverConfirmScreen(Stop)` `:3006`), `:3031` `o` (pushes
`DriverConfirmScreen(ToggleOptIn)` `:3033`).
These are all reached only via `DetailSubView::Driver`, so making that sub-view
unreachable covers them — but add a belt-and-braces guard where cheap.

### C. Render dispatches — `src/ui/screens/detail.rs`
`:3264` (main) and `:4794` (the copy used as the `EnqueueScreen`/`DriverInjectScreen`
backdrop). Both must be unreachable / no-op when off.

### D. Footer — `src/ui/screens/detail.rs`
- `:5739` `driver_footer_spans(width)`; consts `:5727` `DRIVER_FOOTER_FULL_CELLS`,
  `:5730` `DRIVER_FOOTER_MEDIUM_CELLS`
- `:5786` `footer_spans()` early-return into `driver_footer_spans`
- **`:5795` `"[1-0/D]"`** — the shared prefix rendered on **all ten non-driver tabs**.
  This is the one string that leaks `D` everywhere; it must read `"[1-0]"` when off.
- `:5877` `build_footer(sub_view, width)` — signature will need the flag

### E. Dashboard — `src/ui/screens/normal.rs`
- `:461` `r` → `DriverConfirmScreen(Start)` at `:463`
- `:471` `x` → `DriverConfirmScreen(Stop)` at `:474`
- `:482` `o` → `DriverConfirmScreen(ToggleOptIn)` at `:485`
- `:81-86` `BADGE_DRIVEN` (rank 1, magenta+bold); `:128` `driven_and_live` field;
  `:180`, `:190-193` rank logic; `:244-261` `row_badge(ctx, alias)`;
  `:822-838` render site
- attention-first sort puts driven-and-live first (test at `:2166-2176`); with the
  flag off `driven_and_live` must always resolve `false` so the ordering is
  driver-free. Gate it at the ONE source — `row_badge` / the badge-condition
  construction — not at each consumer.

### F. Detail tab-strip live marker
`detail.rs:448-460` renders magenta `◆` via `driver_live_for(ctx, alias)` — gone
with the tab.

### G. Help screen — `src/ui/screens/help.rs`
- `:80` `(BADGE_DRIVEN, "an agent is driving this repo right now")`
- `:92` `INJECTION_LEGEND` (4 entries), `:126` `INJECTION_LEGEND_CONT`
- `:210` `row("r", "Start a driver run (dashboard)")`
- `:211` `row("x", "Stop the live driver run (dashboard)")`
- `:212` `row("o", "Toggle driver opt-in (dashboard)")`
- `:221` `heading("Driver Tab (detail view)")` → append ` — EXPERIMENTAL` when on
- `:223-230` `Shift+D`, `j / k`, `PgUp / PgDn`, `f`, `G`, `i`, `s`, `x`
- `:237` `row("o", "Toggle driver opt-in for this project")`
- `:260-266` `heading("Injected Message States")` + its loop
- existing help tests asserting these rows: `:442, :447, :450, :466, :491, :511` —
  they will need the flag threaded through, NOT deleted.

### H. Status messages — `src/app.rs`
`:1929` `"Driving {alias} — run {run_id}"`, `:2066` `"Stopping {alias} — run {run_id}"`.
Reached only through `start_driver_run` (`:1889`) / `stop_driver_run` (`:2009`),
which are only reachable from gated keys. Add a defensive early-return in both so a
run cannot be started or stopped from the TUI with the flag off.

### I. NOT driver-related — do not touch
`DefaultsEditTarget` (`src/ui/screens/mod.rs:344`) is `{ Project, Global }` — the
config-file target, nothing to do with the driver. The Defaults tab option table
(`detail.rs:4800-7213`) lists **zero** `driver*` keys; verified by grep. There is no
driver option in the config editor to gate.

### J. `AppContext` construction sites (adding `experimental: bool` breaks all 9)
| file:line | kind |
|---|---|
| `src/app.rs:474` | the only production construction (`App::from_config`) |
| `src/app.rs:4422` | exhaustive destructure guard (test) — will flag deliberately |
| `src/ui/screens/mod.rs:2226` | test helper `ctx_with_aliases` (`:2209`) |
| `src/ui/screens/normal.rs:1183` | test helper `ctx_with_aliases` (`:1165`) |
| `src/ui/screens/delete_confirm.rs:249` | test |
| `src/ui/screens/driver_confirm.rs:696` | test |
| `src/ui/screens/detail.rs:9649` | test helper `test_ctx` (`:9643`) |
| `src/ui/screens/render_escape_guard.rs:1022` | test helper `probe_ctx` |
| `src/ui/screens/render_escape_guard.rs:1245` | test helper `chrome_ctx` |

Existing test helpers should default `experimental: true` so the ~30 existing
driver-tab tests keep passing unchanged, and the new tests set it explicitly both
ways. Prefer adding a `ctx.with_experimental(bool)` or a second helper over
rewriting every call site.

---

## Required tests (requirement 6)

At minimum, and named descriptively in this repo's sentence style:

1. Truthy parsing: `1`/`true`/`TRUE`/`yes`/`on` → on; unset/empty/`0`/`false`/`no`/
   `off`/`maybe` → off. Pure, via `experimental_features_enabled_from`.
2. Flag OFF: `visible_tab_count` is 10; `tab_titles` never emits a Driver label at
   any of the three width tiers; `sub_view_from_index(10)` is not `Driver`;
   `Shift+D` does not reach the Driver tab; `Right` from tab 9 stays at 9.
3. Flag ON: `visible_tab_count` is 11; the Driver label is present; `Shift+D`
   reaches it; `Right` from 9 reaches 10.
4. Flag ON: the Driver pane title contains the literal `EXPERIMENTAL`.
5. Flag OFF: the non-driver footer prefix contains no `D`; the help screen contains
   no driver rows and no `Driver` heading. Flag ON: it does, with `EXPERIMENTAL`.
6. Flag OFF: the dashboard `r`/`x`/`o` keys push no driver screen; `row_badge`
   never yields `BADGE_DRIVEN`.
7. `driver_opt_in` default is still `None` and `driver_max_concurrent` admission is
   unchanged in both flag states (regression guard for D4).

---

## Documentation

Add `GSDMM_EXPERIMENTAL_FEATURES` to the env-var table in
`docs/CONFIGURATION.md:95-106`, spelling out the accepted truthy values and that
the default is OFF and hides the Driver tab entirely.

---

## Build gates (run raw, never piped through grep — rtk strips `warning:`/`test result:`)

```
rtk proxy cargo build
rtk proxy cargo test --no-fail-fast
rtk proxy cargo clippy -- -D warnings
```

Known-environmental, NOT regressions:
- `envelope::policy` git-version-constants test (installed git 2.53 vs expected 2.43)
  — this is the single pre-existing `--lib` failure; baseline is 2087 passed / 1 failed.
- `driver::run::tests::the_current_group_agrees_with_the_proc_parse` — a real flake
  under parallel load; re-run isolated before calling it a failure.
- `cargo clippy --all-targets` has 4 PRE-EXISTING lint errors
  (`tests/envelope_carrier_reach.rs`, `tests/envelope_config_resolution.rs`,
  `src/browser.rs`, `src/project_creator.rs`). Do not fix them. The plain
  `cargo clippy -- -D warnings` gate is the one that must stay green.
