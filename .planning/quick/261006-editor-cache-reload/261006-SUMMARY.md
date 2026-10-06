---
status: complete
commit: e90be67
---
# 261006-editor: Reload browser file cache after $EDITOR exit

Shipped in e90be67 (`ProjectViewCache` tracks the viewed file path; `App::refresh_after_editor` re-reads it on the editor-return path in `main.rs`; unit test covers the seam without a PTY). ffbe603 moved the todo to completed.

Inferred decisions [audit]: seam verified by unit test only; real PTY suspend/resume not UAT'd.
