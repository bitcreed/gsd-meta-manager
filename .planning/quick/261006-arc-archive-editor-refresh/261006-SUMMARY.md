---
status: complete
---
# 261006-arc: Archive view refresh after editor exit

`ProjectViewCache` gains `archive_file_path` (set at both archive file-open sites in `detail.rs`) and `reload_archive_file_if`; `App::refresh_after_editor` now calls it next to the browser reload. Test `editor_exit_reloads_archive_file_cache` mirrors the browser one.

Inferred decisions [audit]: the path is not cleared on view close (same as `browser_file_path`; guarded by `archive_file_content.is_some()`); the `e` key refuses files under `/milestones/` as read-only, so the reload only fires for archive files outside that tree -- the read-only rule was left untouched.
