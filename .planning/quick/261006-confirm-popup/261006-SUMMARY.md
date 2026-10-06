---
status: complete
commit: 3140a5b
---
# 261006-confirm: Modal Yes/No popup for destructive confirmations

Shipped in 3140a5b (new `src/ui/screens/confirm_popup.rs`; `delete_confirm.rs`, `queue_delete_confirm.rs`, `driver_confirm.rs` routed through it; todo moved to completed). `delete_confirm` no longer blanks the dashboard. Render and key tests added.

Inferred decisions [audit]: No focused by default (safe default for destructive actions); legacy `y`/`n`/Esc kept; Left/Right/Tab move focus, Enter activates.
