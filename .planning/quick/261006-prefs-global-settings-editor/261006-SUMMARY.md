---
status: complete
commit: 0e55bf2
---
# 261006-prefs: Global settings editor and Preferences screen

Shipped in 0e55bf2 (`src/ui/screens/preferences.rs` new; `detail.rs`, `normal.rs`, `help.rs`, `mod.rs`, `render_escape_guard.rs` touched; todo moved to completed). Preferences edits save at once to the manager config. Stale '* = inherited from global' legend removed.

Inferred decisions [audit]: only `default_runtime` and `driver_max_concurrent` exposed (the manager-config keys that exist today); save-on-edit rather than an explicit save step; scope wording as chosen by the commit.
Open: item 8 'runtime picker' of the Codex todo remains open; the 2026-09-23 settings-editor todo itself is fully complete.
