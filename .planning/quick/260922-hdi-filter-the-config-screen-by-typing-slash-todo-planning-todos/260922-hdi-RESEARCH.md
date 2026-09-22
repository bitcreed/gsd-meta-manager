# Quick 260922-hdi: Filter the Config screen by typing `/` - Research

**Researched:** 2026-09-22
**Domain:** ratatui key dispatch / list filtering in `src/ui/screens/detail.rs` (Defaults sub-view, titled "Config Settings")
**Confidence:** HIGH (all claims read from source this session; no external packages)

<user_constraints>
## User Constraints (no CONTEXT.md; from the task + todo `.planning/todos/pending/2026-09-22-filter-the-config-screen-by-typing-slash.md`)

- `/` on the Config Settings screen opens a filter input; typing narrows rows by key (help text optional).
- Esc clears the filter. Enter/arrows act on the filtered selection, which must map back to the correct underlying row.
- Reuse an existing filter/search input pattern if one exists.
- MUST NOT regress the `ConfigValueKind::Unset` Enter-opens-chooser fix (72b5e03..9ef0f7f).
- Human unavailable: inferred decisions are marked **[INFERRED]** for audit.
- Project: work goes through GSD; tests with `--no-fail-fast` (memory: fail-fast hides suites); no new crates.
</user_constraints>

## Summary

The "Config Settings" screen is `DetailSubView::Defaults`, rendered by `render_defaults_tab` (`detail.rs:4957-5231`). Its rows are a **flat** `Vec<ConfigEntry>` rebuilt on every key/render by `entries_for_cache(cache)` (`detail.rs:7358-7369`) — there are no header rows; the category is a left column printed only when `entry.show_category` is true (`detail.rs:4989-4996`, set by `first` in the `push` closure `detail.rs:6405-6421`). State is `ProjectViewCache.defaults_selected: usize` and `defaults_editing: Option<usize>` (`mod.rs:971-974`), both **indices into the full `entries_for_cache` list**. Every edit path (Enter chooser/text/int step, `x` clear, dropdown apply, text-input intercept, popup render) resolves the row via `entries.get(idx)` and then mutates **by key string** (`set_config_value(active, entry.key.as_ref(), ..)` `detail.rs:2599`).

**Primary recommendation:** Keep `defaults_selected`/`defaults_editing` as UNDERLYING indices, and add a derived `visible` index list (underlying indices matching the filter). Only navigation (j/k/Up/Down/PgUp/PgDn) and rendering become filter-aware; Enter/x/dropdown/text-input code and the two Unset regression tests stay byte-for-byte unchanged because they already speak underlying indices. Add a typing-mode intercept modelled on `NormalScreen::handle_search_key` (`normal.rs:651-695`).

## Code Map (verified, file:line)

| Concern | Location | Notes |
|---|---|---|
| Cache state | `src/ui/screens/mod.rs:965-984` | `defaults_selected: usize`, `defaults_editing: Option<usize>`, `defaults_dropdown_selected`, `defaults_text_buffer: EditBuffer`; struct is `#[derive(Default)]` (`mod.rs:906-907`); all struct literals use `..Default::default()` (`mod.rs:3018-3022`, `detail.rs:9329-9333`) → new fields are additive |
| Row model | `detail.rs:6303-6327` `ConfigEntry { category: &'static str, key: Cow<'static,str>, value, kind, show_category, from_defaults, help }` | `key` Owned half is UNTRUSTED (pass-through rows, category `PASSTHROUGH_CATEGORY = "Not modelled"` `detail.rs:7219`) |
| Kinds | `detail.rs:6046-6075` | `Bool, Enum, String, Integer, Null, Unset(Box<ConfigValueKind>), ReadOnly`; `editable()` unwraps `Unset` |
| Categories | `detail.rs:6433..7174` | "Planning", "Execution", "Docs & Output", "Features", "Model & Pipeline", "Misc", "Orchestration", "Statusline", "Routing", "External Job", "Capabilities", "Review", "Gates" (+ "Not modelled"); 130 `push(cat, ..)` rows |
| Tab arrival reset | `detail.rs:1315-1332` | sets `defaults_selected = 0`, `defaults_editing = None`, clears buffer |
| String-edit intercept | `detail.rs:1479-1497` → `handle_text_input_key` `detail.rs:3429-3490` | routes ALL keys when a String row's popup is open |
| Esc/q arm | `detail.rs:1500-1603`; Defaults part `1589-1599` (closes open popup) then `ScreenAction::Pop` `1600-1602` | `Esc` and `q` share the arm |
| Down/j | `detail.rs:1700-1721` | dropdown cursor if editing, else `defaults_selected+1` clamped to `entries_count_for_cache` |
| Up/k | `detail.rs:1829-1838` | |
| PageDown / PageUp | `detail.rs:1984-1992` / `2120-2124` | `PAGE_SCROLL_LINES` |
| Enter/Space | `detail.rs:2583-2643` | editing → apply dropdown pick (`2591-2606`); else open chooser (`2611-2615`), text popup (`2616-2631`), or step Integer (`2632-2637`) on `entries.get(defaults_selected)` |
| `x` clear | `detail.rs:2708-2753` | acts on `entries.get(defaults_selected)` — **destructive** |
| `r` reload / `d` target toggle | `detail.rs:2755-2771` / `2814-2841` (`d` resets `defaults_selected = 0` at `2833`) | |
| Global digit tabs, Left/Right, Tab, `?` | `detail.rs:2178-2219`, `2775`, `3334` | would fire while typing unless intercepted |
| `/` binding | only `normal.rs:588` (dashboard). **Unbound in DetailScreen** (grep `Char('/')` over `src/`) | free to bind |
| Render list | `detail.rs:4985-5092` | `enumerate()` → `i == selected` highlight (`5058`); fresh `ListState` each frame, `select(Some(selected))` (`5090-5092`) — no stored scroll offset for this tab |
| Help pane | `detail.rs:5094-5101` | `entries[selected.min(len-1)].help` |
| Popup render | `detail.rs:5103-5230` | `entries.get(defaults_editing)` — underlying index |
| Footer | `footer_spans` `detail.rs:5945-6036`, Defaults arm `6016-6025`; exact text pinned by `test_other_footers_unchanged_by_browse_edit_hint` `detail.rs:9468-9479` | not cache-aware (signature `(sub_view, width, experimental)`) |
| Escape helper | `shown()` `detail.rs:77-79` → `crate::text::render_for_terminal` | `Block::title` preserves invisible chars (comment `detail.rs:5108-5110`) |

### Existing pattern to reuse: dashboard filter
- Enter mode: `normal.rs:588-595` (`self.searching = true; clear`).
- Intercept: `normal.rs:415-417` (`if self.searching { return self.handle_search_key(..) }`).
- Keys: `normal.rs:651-695` — `Char` push + reselect first, `Backspace` pop, `Esc` clear+exit, `Enter` exit keeping filter, `_ => None` (swallows everything else).
- Match: raw, case-insensitive `to_lowercase().contains` (`mod.rs:1818-1833`).
- Echo: `render_search_footer` `normal.rs:1074-1101` — `"/ "` + `render_for_terminal(filter)` underlined + `"_"`, right hint `"[Esc]clear [Enter]keep"`.

### Regression tests that pin the Unset fix (must stay green unchanged)
`detail.rs:13412-13556`: helper `ctx_on_config_row(config, key)` (sets `defaults_selected = underlying idx`), `sparse_gsd_config()` (`{"mode":"yolo"}`), `enter_on_an_unset_enum_row_opens_its_chooser_and_applies_the_pick` and `every_unset_choice_row_opens_a_chooser_whose_options_all_apply` — both assert `defaults_editing == Some(underlying idx)`. Also `render_escape_guard.rs:1119` / `1717` set `defaults_selected`/`defaults_editing` to underlying indices from `first_passthrough_entry` / `first_string_entry` (`detail.rs:7397-7423`). Harness: `press()` `detail.rs:10223-10225`, `test_ctx()` `detail.rs:10118`, `TestBackend` buffer scrape pattern `detail.rs:13462-13481`.

## Recommended Design

**State** (add to `ProjectViewCache` after `defaults_text_buffer`, `mod.rs:984`):
```rust
/// Raw `/` filter for the Config tab (operator-typed; matched raw, echoed via `shown`).
pub defaults_filter: String,
/// True while the filter input line has focus.
pub defaults_filter_typing: bool,
```

**Helpers** (beside `entries_for_cache`, `detail.rs:7358`):
```rust
fn config_row_matches(entry: &ConfigEntry, q: &str) -> bool {   // q already lowercased
    q.is_empty()
        || entry.key.to_lowercase().contains(q)
        || entry.category.to_lowercase().contains(q)
}
/// UNDERLYING indices of rows the current filter shows (all rows when empty).
fn visible_defaults_indices(cache: &ProjectViewCache, entries: &[ConfigEntry]) -> Vec<usize> {
    let q = cache.defaults_filter.to_lowercase();
    entries.iter().enumerate().filter(|(_, e)| config_row_matches(e, &q)).map(|(i, _)| i).collect()
}
/// Move `defaults_selected` by `delta` visible rows; snaps to first visible if hidden.
fn move_defaults_selection(cache: &mut ProjectViewCache, delta: isize) { .. }
/// If `defaults_selected` is not visible, set it to `visible[0]` (leave as-is when none).
fn snap_defaults_selection(cache: &mut ProjectViewCache) { .. }
```
Match on **key + category** only. [INFERRED] Help summary excluded: prose makes short terms ("plan", "phase") hit dozens of unrelated rows; `value` excluded because editing a value would make the row vanish from under the cursor.

**Key handling** (`handle_key`):
1. After the String-edit intercept (`detail.rs:1497`), add: `if current_view == Defaults && cache.defaults_filter_typing && cache.defaults_editing.is_none() { return self.handle_config_filter_key(code, ctx); }`
   - `Char(c)` → push, `snap` to first visible (mirror `normal.rs:653-662`; reselect first match on every change).
   - `Backspace` → pop char, snap.
   - `Esc` → clear filter, `typing = false`; **selection stays on its underlying row** (free, since it is an underlying index).
   - `Enter` → `typing = false`, keep filter. Does NOT open the chooser; the next Enter does. [INFERRED — mirrors dashboard `normal.rs:687-692`]
   - `Down`/`Up`/`PageDown`/`PageUp` → `move_defaults_selection` (arrows work while typing; `j`/`k` are text). [INFERRED]
   - `_` → `ScreenAction::None` (swallows digits, `q`, `x`, `d`, `r`, Left/Right, Tab, `?`).
2. New arm `KeyCode::Char('/') if current_view == DetailSubView::Defaults` (only when `defaults_editing.is_none()`): `typing = true`, keep existing filter text so `/` re-edits it. [INFERRED; dashboard clears instead — either is fine, Esc clears]
3. Esc/q arm Defaults block (`detail.rs:1590-1599`): after the popup-close check, `if !cache.defaults_filter.is_empty() { clear filter; typing=false; redraw; return None }`; otherwise fall through to `Pop` (existing behaviour). Applies to `q` too because the arm is shared — same as popup-close. [INFERRED]
4. Down/Up/PgDn/PgUp non-editing branches (`1714-1718`, `1835`, `1986-1990`, `2122`) → `move_defaults_selection(cache, ±1 / ±PAGE_SCROLL_LINES)`. With empty filter this is identical to today.
5. Enter (`2608-2610`) and `x` (`2717-2719`): add a guard `if !visible.contains(&selected) { no-op }` — only reachable when the filter matches nothing. Everything after the guard is unchanged.
6. `d` toggle (`2833`): after `defaults_selected = 0`, `snap`. `r` reload (`2755-2771`): `snap` (pass-through row set may change). Tab arrival (`1328-1331`): also clear `defaults_filter` and `defaults_filter_typing`.

**Render** (`render_defaults_tab`):
- Keep the `entries.is_empty()` no-config early return (`4970-4983`).
- Compute `visible`; build `items` from `visible` only. Recompute category display: show `entry.category` when it is the first visible row or differs from the previous visible row's category (replaces `entry.show_category`, which is keyed to the unfiltered order). This gives "headers kept only if a child matches" for free.
- Highlight where underlying `idx == selected`; `list_state.select(visible.iter().position(|&i| i == selected))` — the **visible position**, never the underlying index.
- Title (`5066-5069`): when filter non-empty or typing, append `format!(" /{}{} ({}/{}) ", shown(&filter), if typing {"_"} else {""}, visible.len(), entries.len())` (e.g. ` Config Settings  /drift_ (5/130) `). Title keeps the change inside this function; `footer_spans` has no cache access. [INFERRED]
- Empty result: render a single dim `ListItem` "  No config keys match" and skip the help pane (don't show help for a hidden row).
- Help pane (`5099-5100`): use `entries[selected]` only when `visible.contains(&selected)`.
- Popup block (`5103+`) unchanged.

**Footer:** add `[/]filter  ` to the Defaults arm (`6016-6025`, e.g. before `[?]help` or after `[r]eload  `) and update the pinned string at `detail.rs:9475-9477`. Help screen (`help.rs:200-232`) has no Config section; leave it. [INFERRED]

## Common Pitfalls

1. **Storing a visible index in `defaults_selected`/`defaults_editing`.** Breaks Enter/x/dropdown/text-intercept/popup (all do `entries.get(idx)` on the full list), both Unset regression tests (`13455-13459`, `13528-13534`), and the render-escape probes (`render_escape_guard.rs:1119`, `1717`). Keep underlying indices; convert only at render (`ListState`) and in navigation.
2. **Typing mode leaking keys to global arms.** Without the intercept, typing `x` clears the selected config value on disk (`2708-2753`), `d` flips the edit target, digits switch tabs (`2178-2187`), `q` pops the screen. Intercept must run before `match code` (`1499`).
3. **Highlight/scroll mismatch.** `render_defaults_tab` compares `i == selected` over `enumerate()` and passes `selected` to `ListState` (`5058`, `5091`). Over a filtered iterator both must use the underlying idx for the highlight and the visible position for `ListState`, else the wrong row highlights or the viewport doesn't follow. No stored scroll offset exists for this tab, so `ListState` auto-scroll is the only mechanism.
4. **Empty result set.** `visible` empty → Enter/x must no-op (guard), `move_defaults_selection` must not index `visible[0]`, help pane must not render a hidden row's help.
5. **Selection hidden after `d`/`r`/typing.** `d` resets to 0 (`2833`), which may be filtered out; call `snap`.
6. **Stale filter on re-entry.** View cache is per-alias and persists; clear filter + typing in the arrival reset (`1328-1331`), or a returning user lands in a narrowed list (or worse, typing mode swallowing keys).
7. **Untrusted text.** Pass-through keys are project-supplied; match raw, but echo the filter through `shown()` (mirrors `normal.rs:1075-1083`). Never interpolate the filter into a `ConfigHelp` (must stay `&'static`, `detail.rs:6094-6101`).
8. **`rtk` output filtering** can fake pass counts — run tests via `rtk proxy` and `--no-fail-fast` (project memory).

## Test Plan (add in `detail.rs` tests beside `13412+`, reusing `ctx_on_config_row`, `sparse_gsd_config`, `press`, TestBackend scrape)

| Test | Asserts |
|---|---|
| `slash_filter_then_enter_opens_the_unset_rows_chooser` | sparse config, cursor at 0; `/`, type `context_drift_action`, Enter (confirm), Enter → `defaults_editing == Some(underlying idx)` of `workflow.context_drift_action`; Down+Enter applies `"block"` (re-proves the Unset fix through the filter; that query matches exactly one key — siblings are `plan_drift_precheck`, `context_drift_precheck`, `drift_threshold`, `drift_action` at `6482-6525`) |
| `typing_drift_narrows_navigation_to_matching_rows` | `/drift`, Down/Up visit only the 5 drift rows' underlying indices |
| `filter_render_shows_only_matches_and_count` | TestBackend: matching keys drawn, a non-matching key (`mode`) absent, title contains `/drift` and `(5/` |
| `typing_swallows_mutating_and_navigation_keys` | while typing, `x`, `d`, `q`, `3` only extend the filter; config/target/sub-view unchanged, no Pop |
| `esc_clears_typing_then_confirmed_filter_then_pops` | Esc while typing → filter empty, not Pop; confirmed filter + Esc → cleared, `ScreenAction::None`; Esc with no filter → `ScreenAction::Pop` |
| `empty_filter_result_is_inert` | `/zzzz`, Enter; Enter and `x` change nothing; render does not panic |
| footer pin update | `detail.rs:9475-9477` includes `[/]filter` |

Commands: `rtk proxy cargo test --no-fail-fast --lib ui::screens::detail` (quick), `rtk proxy cargo test --no-fail-fast` (full; the lone `envelope/policy.rs` git-version failure is expected locally per project memory). Nyquist validation is disabled in `.planning/config.json` (`"nyquist_validation": false`), so no Validation Architecture section.

## Environment Availability

| Dependency | Available | Version |
|---|---|---|
| cargo/rustc | yes | cargo 1.98.1 |

No new crates; no Package Legitimacy Audit needed.

## Security Domain

V5 input handling only: filter text is operator-typed, matched raw, rendered escaped via `shown()`; pass-through keys it matches against are untrusted (T-VQW-01) and already escaped at their own render site (`detail.rs:5007-5010`). No persistence of the filter. Mutations remain gated on a visible selected row.

## Assumptions Log

| # | Claim | Risk if wrong |
|---|---|---|
| A1 | [INFERRED] Match key + category, not help summary/value | Users can't find a key by describing it; easy to extend `config_row_matches` later |
| A2 | [INFERRED] Enter while typing only confirms; second Enter edits | One extra keystroke; alternative is confirm+open in one step |
| A3 | [INFERRED] `q` clears an active filter like Esc (shared arm) | User pressing `q` to leave needs two presses when filtered |
| A4 | [INFERRED] `/` re-opens input seeded with current filter (dashboard clears) | Minor UX inconsistency with dashboard |
| A5 | [INFERRED] Filter echo lives in the list block title, not the footer | Title may truncate on very narrow terminals |
| A6 | [ASSUMED] ratatui 0.30 `List` auto-scrolls to `ListState::selected` with a default (0) offset each frame, as the current code relies on | None new — the existing tab already depends on it |

## Sources
- Source read this session: `src/ui/screens/detail.rs`, `src/ui/screens/mod.rs`, `src/ui/screens/normal.rs`, `src/ui/screens/help.rs`, `src/app.rs:72-84`, commits `72b5e03`, `0581c70`, `9ef0f7f` (stat + messages), todo file, `.planning/config.json`, `.planning/STATE.md`.

**Valid until:** until `detail.rs` Defaults key/render code changes (line numbers drift with any edit above ~1500).
