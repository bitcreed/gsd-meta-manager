---
status: complete
commit: a23b6ca
---
# 261006-badge: Width-stable badge glyphs

Shipped in a23b6ca (`src/ui/screens/normal.rs`, `detail.rs`; todo moved to completed). Test `badge_glyphs_are_width_stable` added.

Inferred decisions [audit]: U+2016 and U+25F7 picked as narrow single-cell, non-emoji-presentation substitutes; test asserts display width rather than any specific terminal rendering.
