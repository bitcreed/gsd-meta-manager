---
status: complete
commit: 4c5c71b
---
# 261006-cfg: Config view defaults and value search

Shipped in 4c5c71b (`src/ui/screens/detail.rs`, todo moved to completed). `ConfigEntry` gains `builtin_default`, filled by `builtin_default_for_key`; Project view no longer sets `from_defaults`. `config_value_matches` extends the `/` filter to values (and built-in defaults); `(unset)`, true/false and empty never match, numbers do. Tests added for builtin-default display, value/key matching, boolean/unset exclusion and numeric match.

Inferred decisions [audit]: built-in defaults come from a manifest-based table inside the UI layer (`builtin_default_for_key`) rather than reading gsd-core at runtime; value matching can hide a row currently being edited when the query no longer matches its value; the `from_defaults` machinery was left in place but is now unreachable from Project view (cleanup deferred).
