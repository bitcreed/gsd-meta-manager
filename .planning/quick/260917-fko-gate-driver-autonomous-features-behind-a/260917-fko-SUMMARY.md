---
quick_id: 260917-fko
phase: quick-260917-fko
plan: 01
subsystem: ui
status: complete
tags: [experimental-flag, driver, feature-gate, tui, env-var]
requires: []
provides:
  - "GSDMM_EXPERIMENTAL_FEATURES startup flag"
  - "AppContext.experimental as the single gate field"
  - "src/experimental.rs truthy parse"
affects:
  - src/experimental.rs
  - src/lib.rs
  - src/app.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/help.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/driver_confirm.rs
  - docs/CONFIGURATION.md
tech-stack:
  added: []
  patterns:
    - "One env read at startup, one bool on AppContext, N gate points read the field"
    - "Coerce-at-read (effective_sub_view) instead of a guard per consumer"
key-files:
  created:
    - src/experimental.rs
  modified:
    - src/lib.rs
    - src/app.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/help.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/driver_confirm.rs
    - docs/CONFIGURATION.md
decisions:
  - "Truthy set is the closed set 1/true/yes/on, not a presence test, so =0 means off"
  - "attention_rank_for needed its own gate: the sort order is a disclosure surface"
  - "The drive CLI subcommand stays ungated (D5), because the TUI respawns this binary"
metrics:
  duration: "~1h"
  completed: 2026-09-17
actuals:
  tokens: 31000
  tasks: 3
  commits: 5
plan_head_before: 1ba4516
---

# Quick Task 260917-fko: Gate Driver Features Behind `GSDMM_EXPERIMENTAL_FEATURES` Summary

The driver / autonomous-orchestration surface is now hidden behind a
`GSDMM_EXPERIMENTAL_FEATURES` startup flag that defaults OFF, threaded from one
env read through one `AppContext` field to every surface that could reveal a
driver exists; what survives with the flag on is labelled EXPERIMENTAL.

## What shipped

One env read (`src/app.rs:522`), one parse helper (`src/experimental.rs:35`),
one `AppContext` field (`src/ui/screens/mod.rs:1402`), and the gate points below.

### The parse (D1)

`src/experimental.rs`, three public items:

| Item | Line |
|------|------|
| `EXPERIMENTAL_FEATURES_ENV` | `src/experimental.rs:19` |
| `experimental_features_enabled_from(Option<&str>) -> bool` | `src/experimental.rs:35` |
| `experimental_features_enabled() -> bool` | `src/experimental.rs:51` |

**The exact truthy-value parsing implemented:**

```rust
let Some(value) = raw else { return false };
let value = value.trim();
["1", "true", "yes", "on"].iter().any(|t| value.eq_ignore_ascii_case(t))
```

That is: surrounding whitespace trimmed (`str::trim`, so spaces, tabs and
newlines), then ASCII-case-insensitive equality against the **closed** set
`1`, `true`, `yes`, `on`. `None` (unset), `""`, `"   "`, `0`, `false`, `no`,
`off`, and every unrecognised value (`maybe`, `ture`, `1 1`, `2`, `enabled`) are
OFF. Interior whitespace is *not* collapsed, so `"1 1"` is off. Deliberately not
a presence test, because `GSDMM_EXPERIMENTAL_FEATURES=0` has to mean off.

### Every driver surface gated, with final `file:line`

| # | Surface | Gate | Final location |
|---|---------|------|----------------|
| 1 | Tab count / bar bound | `visible_tab_count(experimental)` → 11 or 10 | `src/ui/screens/detail.rs:337` |
| 2 | Full-tier width threshold | `tab_bar_full_cells()` → 107 or **96** | `src/ui/screens/detail.rs:408` (const at `:400`) |
| 3 | Compact-tier width threshold | `tab_bar_compact_cells()` → 77 or **69** | `src/ui/screens/detail.rs:422` (const at `:405`) |
| 4 | `tab_titles` label set | slices `TAB_LABELS_*[..visible]`, all three tiers | `src/ui/screens/detail.rs:483` |
| 5 | `tab_label_line` | takes `&[&'static str]`; Driver branch unreachable | `src/ui/screens/detail.rs:519` |
| 6 | `compact_label_cells` | marker cell charged only when experimental | `src/ui/screens/detail.rs:536` |
| 7 | `windowed_tab_titles` | every `TAB_COUNT` bound → `visible` | `src/ui/screens/detail.rs:550` |
| 8 | Tab-strip live marker (`◆`) | gone with the sliced Driver label (no edit needed) | `src/ui/screens/detail.rs:519` |
| 9 | `sub_view_from_index` | index 10 falls to `PhaseList` when off | `src/ui/screens/detail.rs:783` |
| 10 | `effective_sub_view` (new) | stored `Driver` reads as `PhaseList` | `src/ui/screens/detail.rs:817` |
| 11 | `handle_key` sub-view read | coerced | `src/ui/screens/detail.rs:1470` |
| 12 | Main render sub-view read | coerced | `src/ui/screens/detail.rs:3349` |
| 13 | Overlay-backdrop render read | coerced | `src/ui/screens/detail.rs:4895` |
| 14 | `Shift+D` | `KeyCode::Char('D') if ctx.experimental` — falls through unhandled | `src/ui/screens/detail.rs:2198` |
| 15 | `Right` arrow guard | `current_idx < visible_tab_count(ctx.experimental) - 1` | `src/ui/screens/detail.rs:2213` |
| 16 | Shared footer tabs hint | `[1-0/D]` vs `[1-0]` on all ten non-driver tabs | `src/ui/screens/detail.rs:5953` |
| 17 | Dashboard driven badge | `driven_and_live` gated at its one computation site | `src/ui/screens/normal.rs:252` |
| 18 | Dashboard `r` (start) | match guard | `src/ui/screens/normal.rs:472` |
| 19 | Dashboard `x` (stop) | match guard | `src/ui/screens/normal.rs:486` |
| 20 | Dashboard `o` (toggle opt-in) | match guard | `src/ui/screens/normal.rs:500` |
| 21 | Attention-first sort rank 1 | **deviation, see below** | `src/ui/screens/mod.rs:1772` |
| 22 | Help: 3 dashboard driver rows | omitted | `src/ui/screens/help.rs:221` |
| 23 | Help: whole Driver Tab section | omitted (heading, blank, 9 rows, blank) | `src/ui/screens/help.rs:243` |
| 24 | Help: driven badge legend entry | skipped in the `BADGE_LEGEND` loop | `src/ui/screens/help.rs:290` |
| 25 | Help: whole Injected Message States section | omitted | `src/ui/screens/help.rs:301` |
| 26 | Help render call site | `help_lines(ctx.experimental)`, `_ctx` renamed | `src/ui/screens/help.rs:393` |
| 27 | `start_driver_run` | defensive `if !self.ctx.experimental { return; }` | `src/app.rs:1916` |
| 28 | `stop_driver_run` | same | `src/app.rs:2044` |

Ten Driver-only key arms (`j`/`k`/`PageUp`/`PageDown`/`f`/`G`/`i`/`s`/`x`/`o`),
both `render_driver_tab` dispatches and the `switch_to_tab` Driver-entry scan
schedule are disarmed by items 10–13 alone — one coercion rather than a guard
per arm.

### The EXPERIMENTAL markers (D3)

| Marker | Location |
|--------|----------|
| Driver pane title, both sites | `src/ui/screens/driver.rs:995` (`DRIVER_PANE_TITLE`), used at `:1012` and `:1076` |
| Help Driver section heading | `src/ui/screens/help.rs:248` |

Both use the em dash as `\u{2014}`, matching the repo's no-raw-glyph convention.
The narrow-tier `run_list_title` is deliberately untouched — that tier exists to
give the detail pane the selected run's identity.

### The tab-bar width correction

The CONTEXT surface map guessed 66 for the ten-tab compact bar and flagged it
unverified. Re-derived from `Σ(len + 2) + (n − 1)` it is **69**, not 66:
`77 − 4 (label) − 1 (marker) − 2 (pads) − 1 (divider) = 69`. Full drops
`107 → 96` by the same four deductions against a 7-cell label. All four numbers
are now re-derived from the label arrays by
`the_tab_bar_widths_are_the_label_arrays_own_arithmetic`, so a renamed label
fails a test rather than silently leaving a stale constant.

### Documentation

`docs/CONFIGURATION.md` gains a `GSDMM_EXPERIMENTAL_FEATURES` row in the
environment-variable table naming `src/experimental.rs` as the consumer and
spelling out the truthy set and the OFF default, plus a
`### GSDMM_EXPERIMENTAL_FEATURES in detail` section covering what is hidden,
what the markers say, and the three things the flag deliberately does **not**
affect (opt-in, the `drive` subcommand, startup reconciliation).

## Tests added

| Test | File |
|------|------|
| `the_four_documented_truthy_spellings_turn_the_flag_on` | `src/experimental.rs` |
| `the_truthy_set_is_case_insensitive_and_whitespace_trimmed` | `src/experimental.rs` |
| `an_unset_variable_leaves_the_experimental_surfaces_off` | `src/experimental.rs` |
| `the_explicit_falsey_spellings_turn_the_flag_off` | `src/experimental.rs` |
| `an_empty_or_whitespace_only_value_is_off` | `src/experimental.rs` |
| `an_unrecognised_value_is_off_rather_than_on` | `src/experimental.rs` |
| `the_env_var_is_named_once_and_carries_the_gsdmm_prefix` | `src/experimental.rs` |
| `the_flag_neither_weakens_opt_in_nor_the_concurrency_cap` | `src/experimental.rs` |
| `the_tabs_hint_drops_shift_d_on_every_tab_when_experimental_is_off` | `src/ui/screens/detail.rs` |
| `the_visible_tab_count_drops_the_eleventh_tab_when_experimental_is_off` | `src/ui/screens/detail.rs` |
| `no_width_tier_emits_a_driver_label_when_experimental_is_off` | `src/ui/screens/detail.rs` |
| `the_whole_bar_tiers_still_offer_the_driver_label_when_experimental_is_on` | `src/ui/screens/detail.rs` |
| `the_ten_tab_bar_renders_whole_at_its_own_re_derived_widths` | `src/ui/screens/detail.rs` |
| `the_tab_bar_widths_are_the_label_arrays_own_arithmetic` | `src/ui/screens/detail.rs` |
| `index_ten_is_not_a_tab_when_experimental_is_off` | `src/ui/screens/detail.rs` |
| `a_stored_driver_sub_view_reads_as_the_default_tab_when_experimental_is_off` | `src/ui/screens/detail.rs` |
| `shift_d_does_not_reach_the_driver_tab_when_experimental_is_off` | `src/ui/screens/detail.rs` |
| `shift_d_still_reaches_the_driver_tab_when_experimental_is_on` | `src/ui/screens/detail.rs` |
| `right_from_the_last_visible_tab_stays_put_when_experimental_is_off` | `src/ui/screens/detail.rs` |
| `right_from_tab_nine_reaches_the_driver_tab_when_experimental_is_on` | `src/ui/screens/detail.rs` |
| `the_driver_pane_title_carries_the_experimental_marker_with_and_without_runs` | `src/ui/screens/driver.rs` |
| `the_help_screen_documents_no_driver_row_when_experimental_is_off` | `src/ui/screens/help.rs` |
| `the_help_screen_has_no_driver_heading_and_no_injection_legend_when_off` | `src/ui/screens/help.rs` |
| `the_driver_heading_is_marked_experimental_when_the_flag_is_on` | `src/ui/screens/help.rs` |
| `neither_flag_state_leaves_a_double_blank_line_where_a_section_was_cut` | `src/ui/screens/help.rs` |
| `the_flag_off_help_body_is_shorter_but_still_documents_the_other_keys` | `src/ui/screens/help.rs` |
| `a_live_run_lights_no_driven_badge_when_experimental_is_off` | `src/ui/screens/normal.rs` |
| `the_same_live_run_lights_the_driven_badge_when_experimental_is_on` | `src/ui/screens/normal.rs` |
| `attention_first_does_not_float_a_driven_project_when_experimental_is_off` | `src/ui/screens/normal.rs` |
| `the_dashboard_driver_keys_push_nothing_when_experimental_is_off` | `src/ui/screens/normal.rs` |
| `the_dashboard_driver_keys_still_push_the_confirmation_when_experimental_is_on` | `src/ui/screens/normal.rs` |

31 tests added. No existing test was deleted; the ~30 pre-existing driver tests
were threaded with `true` and assert exactly what they always asserted.

**Anti-vacuity checked, not assumed.** Three gates were temporarily neutered and
the suite re-run to confirm the new tests are load-bearing:

| Gate neutered | Tests that went red |
|---|---|
| `visible_tab_count` → always 11 | `no_width_tier_emits_a_driver_label_when_experimental_is_off`, `the_ten_tab_bar_renders_whole_at_its_own_re_derived_widths`, `the_visible_tab_count_drops_the_eleventh_tab_when_experimental_is_off`, `right_from_the_last_visible_tab_stays_put_when_experimental_is_off` |
| `Shift+D` match guard removed | `shift_d_does_not_reach_the_driver_tab_when_experimental_is_off` |
| `effective_sub_view` coercion disabled | `a_stored_driver_sub_view_reads_as_the_default_tab_when_experimental_is_off` |

All three were restored before committing.

## Build gates — raw results

Run raw via `rtk proxy`, never piped through grep (rtk strips `warning:` and
`test result:` lines, so a piped gate would pass vacuously).

| Gate | Result | Classification |
|------|--------|----------------|
| `rtk proxy cargo build` | **exit 0**, no warnings | — |
| `rtk proxy cargo clippy -- -D warnings` | **exit 0**, no warnings | — |
| `rtk proxy cargo test --no-fail-fast` | **2118 passed, 1 failed, 15 ignored** | see below |

The single failure:

| Failure | Classification |
|---------|----------------|
| `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` | **Known-environmental.** Installed git is 2.53.0; the constants were derived against 2.43.0. Pre-existing, unrelated to this change, and explicitly listed as the single expected `--lib` failure. |

No other failure, real or environmental. `driver::run::tests::the_current_group_agrees_with_the_proc_parse`
passed on every run and never needed isolated re-running.

Baseline before this task was **2087 passed / 1 failed**; the delta is +31
passing tests and the same one environmental failure.

## Deviations from Plan

### 1. [Rule 2 — Missing critical functionality] The attention-first sort was a second, unguarded disclosure surface

- **Found during:** Task 3, by `attention_first_does_not_float_a_driven_project_when_experimental_is_off`
  failing on its first run.
- **Issue:** The plan (and the CONTEXT surface map, section E) asserted that
  gating `driven_and_live` inside `row_badge` alone would remove "the driven
  badge, its rank-1 position, and therefore the attention-first sort's
  driver-first ordering". That is false. `AppContext::attention_rank_for`
  derives rank 1 from `observed_runs` on its **own** path and never consults
  `row_badge`, so with the flag off a driven project still floated to the top of
  the dashboard under `SortMode::AttentionFirst`. That ordering discloses
  "something is driving this repo" as plainly as the badge does — and it does so
  while painting no glyph, so no glyph-shaped flag-off test would ever have
  caught it.
- **Fix:** Gated `driven_and_live` at `attention_rank_for` too, with a doc
  comment recording why the duplication with `row_badge` is deliberate rather
  than an oversight to be "simplified" away later.
- **Files modified:** `src/ui/screens/mod.rs:1772`
- **Commit:** `83aa7a1`

### 2. [Rule 3 — Blocking issue] `App::new_for_test` read the real process environment

- **Found during:** Task 2, six `app::tests` failures.
- **Issue:** `App::new_for_test` funnels through `App::from_config`, which is
  now the production env read. Six existing driver tests therefore started
  failing, and — worse than failing — every driver test's verdict had silently
  become dependent on whether the developer or CI runner happened to export
  `GSDMM_EXPERIMENTAL_FEATURES`. That is exactly the "depends on the machine"
  defect `new_for_test` exists to remove for `config.json`.
- **Fix:** `new_for_test` pins `ctx.experimental = true` after construction,
  matching the convention every other fixture uses, with the reasoning recorded
  on the line.
- **Files modified:** `src/app.rs:479`
- **Commit:** `be2473c`

### 3. [Plan/allowlist conflict] Group-7 test placed in `experimental.rs`, not `driver/spawn.rs`

- **Issue:** Task 3's action text says to put the D4 regression guard "in
  whichever module already tests admission" — that is `src/driver/spawn.rs`,
  which is **not** in the plan's `files_modified` allowlist, and the executor's
  commit discipline requires aborting a commit that stages anything outside it.
- **Fix:** The guard lives in `src/experimental.rs` (allowlisted) as
  `the_flag_neither_weakens_opt_in_nor_the_concurrency_cap`, importing
  `driver::spawn::admit`. It asserts the same property — `admit()` and
  `driver_opt_in` are identical under every parse of the variable — and arguably
  reads better there, since the claim being pinned is about the flag.
  `src/driver/spawn.rs` was reverted and is untouched by this task.

### 4. [Minor] Module registration order in `lib.rs`

The plan said to register `experimental` "between `error` and `executor`". That
is not alphabetical (`exe` < `exp`). Registered after `executor`, which is,
matching the rest of the list.

## Coordinator inferred decisions (for audit)

1. **D1's truthy set was an inferred decision, and remains one.** This codebase
   had no boolean env-var convention to copy: every existing variable is either
   presence-tested (`TMUX`) or read as a string value (`VISUAL`, `EDITOR`,
   `TERMINAL`, `HOME`, `GSD_MM_ENVELOPE_ROOT`). The closed set `1`/`true`/`yes`/`on`
   with unrecognised-is-off was chosen and documented rather than derived from
   precedent. The consequential half is that a typo such as
   `GSDMM_EXPERIMENTAL_FEATURES=ture` reads as OFF, not ON — safe for a surface
   that spawns an agent against a real repository, but it will silently not do
   what a user who typo'd expects.

2. **D5 leaves the `drive` CLI subcommand ungated, deliberately.** The TUI
   launches a run by re-executing *this same binary* as `current_exe() drive …`
   (`src/driver/spawn.rs:131`), and the envelope/exec layer scrubs environment,
   so propagation of `GSDMM_EXPERIMENTAL_FEATURES` into that child is not
   guaranteed. Gating the CLI arm would risk breaking the spawn path for exactly
   the users who *did* set the flag. Verified by grep that `src/cli.rs`,
   `src/main.rs` and `src/driver/` contain no reference to `experimental` in
   production code. The consequence to accept: the flag is a **discovery** gate,
   not an authorisation boundary — anyone who can set env can invoke `drive`
   directly. `driver_opt_in` remains the real consent gate and is unweakened
   (threat T-fko-01, disposition `accept`).

3. **The CONTEXT surface map's ten-tab compact width was corrected from 66 to
   69** by re-derivation from the documented `Σ(len + 2) + (n − 1)` formula. The
   map flagged its own figure as unverified and asked for exactly this. The full
   tier is 96. All four constants are now pinned against the label arrays by
   test, so this cannot drift again.

4. **`start_driver_run` / `stop_driver_run` return silently** rather than setting
   an error message. An error message on a flag-off session would itself
   disclose that a driver exists, which is the thing D2 forbids. These are
   defensive backstops; every key that reaches them is already gated.

5. **Both `App::new_for_test` and every `AppContext` fixture default the flag
   ON.** This preserves the meaning of ~30 pre-existing driver tests, but it also
   means the *default* production behaviour (OFF) is the one exercised only by
   the explicitly flag-off tests listed above. That is the intended trade, and it
   is why those tests were anti-vacuity checked by neutering the gates.

## Known Stubs

None. No stub, placeholder, TODO or skipped test was introduced.

## Threat Flags

None. This change adds no network endpoint, no auth path, no file access pattern
and no schema change. It introduces no dependency and touches no manifest, so
the package-legitimacy gate does not apply (T-fko-SC).

The one threat-register item needing a note at close: **T-fko-02 (`mitigate`) is
satisfied and checkable** — `rtk proxy grep -rn "experimental_features_enabled()" src/`
returns exactly two hits, the definition (`src/experimental.rs:51`) and the one
production caller (`src/app.rs:522`). No second read site exists, so no two
surfaces can disagree mid-session.

## Verification

- `Cargo.toml` `version` unchanged (`1.6.0`).
- No tag created; branch is still `dev`.
- Every commit staged by explicit path. The four pre-existing dirty paths
  (`.gsd/`, `.planning/state.json`, and the two unrelated
  `.planning/quick/*-SUMMARY.md`) remain unstaged and untouched.

## Self-Check: PASSED

- `src/experimental.rs` — FOUND
- `docs/CONFIGURATION.md` — FOUND
- `260917-fko-SUMMARY.md` — FOUND
- Commits `b867a6d`, `509063a`, `1a63521`, `be2473c`, `83aa7a1` — all resolve
- `git rev-list --count 1ba4516..HEAD` = **5**, matching the `commits:` field
