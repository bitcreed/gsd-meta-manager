---
phase: quick-260509-k9m
plan: 01
status: complete
date: 2026-05-09
commits:
  - c73cbb9
  - 3d77fa5
files_modified:
  - src/state_reader/config_json.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/detail.rs
---

# Quick Task: Extend Defaults Tab — intel/graphify, text input, clear shortcut

## What Changed

1. **New config keys surfaced.** `intel.enabled`, `graphify.enabled`,
   and `graphify.build_timeout` are now parsed (`IntelConfig`,
   `GraphifyConfig`) and rendered as their own categories in the
   Defaults tab. The bool dropdown wiring and the integer cycle
   (`graphify_build_timeout`) match the conventions of existing rows.

2. **Text input on String rows.** Pressing Enter on a row whose kind
   is `String` (e.g. `base_branch`, `phase_branch_template`,
   `project_code`, `response_language`) opens an inline text-input
   popup pre-filled with the current value. Char keys append,
   Backspace removes, Enter saves, Esc cancels. An empty submission
   clears the field to `(unset)`.

3. **`x`-to-clear shortcut.** Pressing `x` on the Defaults tab while
   no popup is open sets the selected row to `(unset)` (i.e. None /
   absent in JSON). Required scalars (`mode`, `granularity`,
   `model_profile`) ignore the keystroke and show a "cannot be
   cleared" status message — clearing them would break GSD.

4. **Top-level intercept for text-input mode.** The `handle_key`
   entry point detects when a String entry is being edited and
   re-routes every keystroke through `handle_text_input_key` so
   global shortcuts like `q`, `r`, `e`, `?`, `x` don't interfere
   while the user types into the buffer.

## Verification

- `cargo build`: clean
- `cargo clippy`: no issues
- `cargo test`: 102 passed (5 suites)

## Notes

- `set_string_value` and `clear_config_value` are split for clarity:
  Enter with a non-empty buffer goes through `set_string_value`,
  Enter with an empty buffer (and the `x` shortcut) go through
  `clear_config_value`. This keeps the unset-vs-empty-string
  distinction explicit, which matters because GSD treats absent
  keys as "fall back to global defaults" while empty strings would
  override that with a literal empty value.
- The text-input intercept only activates when the entry's kind is
  `String`. Bool/Enum entries continue to use the dropdown picker
  introduced in commit 16a8e92.
