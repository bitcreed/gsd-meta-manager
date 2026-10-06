---
status: complete
commit: aa34fde
---
# 261006-cpp: Create-project confirm popup + mouse on confirm popups

Shipped in aa34fde. `create_project` Confirm phase is now the shared modal popup. Mouse was tractable: `render_confirm_popup` returns the button `Rect`s, `ConfirmPopup` stores them in a `Cell` (render takes `&self`) and `click_as_key` turns a click on `[ Yes ]`/`[ No ]` into `y`/`n`, so click and key share one code path in all four screens (delete_confirm, queue_delete_confirm, driver_confirm, create_project). Tests: rect-vs-drawn-label pin at three sizes, click mapping, per-screen click tests, create_project render/escape/key tests.

Inferred decisions [audit]:
- Create popup defaults focus to Yes (non-destructive; Enter used to create). Destructive popups still default to No.
- Clicking a button now confirms. This reverses the dyf I-13 default that "a click never confirms a dialog"; it was requested, and only a press landing on a labelled button acts (stray clicks, wheel and the double flag are inert).
- Click is mapped to the `y`/`n` key path rather than a separate outcome handler, to avoid duplicating each screen's confirm logic.
- Yes-click is not tested on driver_confirm (would start a real run); No-click and stray-click are.
- Test-only helper `confirm_popup::locate` sits before `mod tests` to satisfy `spawn_seam_guard`'s post-marker rule.
- `envelope_carrier_reach` / `envelope_interior_path` failed once with BrokenPipe under full-suite load and passed on rerun (flaky, unrelated).
