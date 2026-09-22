---
status: resolved
trigger: "Pressing Enter on a Config Settings entry no longer opens the chooser listing its options (todo 2026-09-22, severity major)."
created: 2026-09-22
updated: 2026-09-22
---

# Debug: Enter on an unset Cfg row opens nothing

## Symptoms

- **Expected:** Enter on a Config Settings (Cfg tab) row opens its chooser (bool/enum) or editor.
- **Actual:** nothing is drawn, no status message.
- **Reported as:** regression, examples `mode` / `granularity` / `model_profile`.

## Evidence

- Live TUI under tmux (160x45, this repo registered in a scratch config):
  Enter on `research` (set bool), `code_review_depth` (set enum) and `mode` (set enum)
  open their chooser, with and without `GSDMM_EXPERIMENTAL_FEATURES=1`.
  Enter on `workflow.context_drift_action` (unset enum) draws nothing.
- Probe through the real `DetailScreen::handle_key` on a `{"mode":"yolo"}` config:
  `mode` -> `defaults_editing = Some(73)`; unset rows -> `None`.
- All 13 registered projects' configs: every `mode`/`granularity`/`model_profile` row is
  `ConfigValueKind::Enum` (even when absent, `mode` parses to `""`), so the named examples
  are not the failing rows.

## Hypotheses eliminated

- 260916-vqz (`b` binding) — dashboard-scoped arm in normal.rs; never sees Detail keys.
- 260917-fko (experimental gating) — `effective_sub_view` resolves Defaults unchanged;
  live test with the flag on still opens the chooser on set rows.
- Enter intercepted upstream (event.rs / app.rs / normal.rs) — `RawKey` goes straight to
  the top screen; probe shows the Defaults arm IS reached.
- Popup opens but is not rendered — the render path draws it whenever `defaults_editing`
  is set (confirmed live for set rows).

## Root cause

`src/ui/screens/detail.rs` layered row builders (`opt_bool_layered`, `opt_str_layered`,
`opt_u32_layered`, `opt_enum_layered`, quick_branch_template's inline match) returned
`("(unset)", ConfigValueKind::Null, false)` for a key unset in both layers, discarding the
row's real kind. The Enter arm dispatches on kind: `dropdown_options(Null)` is empty and
`Null` is neither String nor Integer, so no branch runs. Long-standing (since 55baa9d), but
the gsd-core 1.14.0 re-sync (260916-vqw, +57 keys) made most rows of a typical project
`(unset)` — e.g. 25 of the first 32 rows here — which is what surfaced as "Enter stopped
working".

## Fix

`ConfigValueKind::Unset(Box<ConfigValueKind>)` carries the would-be kind; renders like
`Null`; every edit path uses `kind.editable()` (dropdown options, String intercept/popup,
Integer step, `mutate_config_entry`). `Null` stays for read-only shape-varying keys.

- RED: 72b5e03 — GREEN: 0581c70
- Tests: `enter_on_an_unset_enum_row_opens_its_chooser_and_applies_the_pick`,
  `every_unset_choice_row_opens_a_chooser_whose_options_all_apply` (>= 20 unset rows,
  every offered option applied and read back).
- Verified live: unset `workflow.context_drift_action` now opens `warn`/`block`.

## Inferred decisions (for audit)

- The "regression" is attributed to vqw's exposure of many unset rows rather than a code
  change in key dispatch; no commit ever made unset rows editable. Inferred from evidence,
  the reporter was unavailable.
- Unset String and Integer rows were made editable too (same defect class, same fix path);
  choosing an option on an unset row writes the key into the project's config.json.
