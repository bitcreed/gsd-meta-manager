---
status: complete
commit: c3c9d95
---
# 261006-cfr summary

Both items shipped in c3c9d95 (`src/ui/screens/detail.rs`). Tests added: `the_row_under_edit_stays_visible_when_the_filter_stops_matching` and `saving_a_value_the_filter_no_longer_matches_keeps_the_cursor_on_a_visible_row`. Removed `ConfigEntry.from_defaults`, the `opt_*_layered` fallback (helpers are now `opt_bool` etc.), the `defaults` params and the ` *` render; no dead_code allow added.

Inferred decisions [audit]: pin the edited row rather than make edit and filter mutually exclusive; the cursor snaps only when an edit commits (Esc leaves the value unchanged); no layering tests existed (every caller passed None), so none were deleted; both items landed in one commit because they share one file and the signature change touches both.
