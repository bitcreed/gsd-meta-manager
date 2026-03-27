# Phase 08: Queue Execution - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Make QUEUE.md a fully managed queue from the TUI: add, edit, reorder, delete, and mark-done queue items. No process execution — queue items are planning artifacts written to QUEUE.md and picked up by GSD sessions. The existing EnqueueScreen handles adding; this phase adds a queue view for managing existing items.

</domain>

<decisions>
## Implementation Decisions

### Queue View & Editing
- **D-01:** Queue management lives as a view accessible from the detail view — shows all items from QUEUE.md
- **D-02:** Adding items reuses the existing EnqueueScreen (already works with suggestions)
- **D-03:** Press `e` on selected item to open inline text edit (replace the command string in-place)
- **D-04:** Press `d` or `x` to delete with confirmation — persists to QUEUE.md immediately

### Queue Persistence & Status
- **D-05:** Shift+K/J moves selected item up/down, immediately writes updated QUEUE.md via `save_queue()`
- **D-06:** No status field — items in QUEUE.md are implicitly "pending". Done items get removed.
- **D-07:** Press `Enter` or `space` to mark complete — removes item from QUEUE.md
- **D-08:** Empty state shows dim "Queue empty — press 'a' to add"

### Scope Clarification
- **D-09:** No process execution, no output capture, no real-time status — queue is a planning/reminder tool
- **D-10:** Queue items are GSD slash commands or freeform text, not shell commands

### Claude's Discretion
- Queue view layout (list width, key hint placement)
- Inline edit UX details (cursor position, escape to cancel)
- Whether to show queue item count in the tab bar

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/state_reader/queue_md.rs` — QueuedAction struct, parse/write/load/save_queue, suggest_next_commands
- `src/ui/screens/enqueue.rs` — EnqueueScreen for adding new items
- `src/ui/screens/detail.rs` — Tab system for hosting queue view
- `src/ui/screens/delete_confirm.rs` — Confirmation dialog pattern

### Established Patterns
- Tab system: DetailSubView enum + number-key switching
- Inline editing: EnqueueScreen has text input with cursor
- File persistence: save_queue() writes atomically via temp file + rename

### Integration Points
- DetailSubView needs Queue variant (new tab or section in detail view)
- QueuedAction may need index/position for reorder operations
- save_queue() already handles atomic writes — reorder just calls it after mutation

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond decisions above.

</specifics>

<deferred>
## Deferred Ideas

- Process execution of queue items (launching Claude Code sessions) — needs research on GSD hooks/injection
- Batch execution of multiple items — QUEUE-05
- Inline text editing of queue items — QUEUE-04 (simpler: open EnqueueScreen pre-filled)

</deferred>

---

*Phase: 08-queue-execution*
*Context gathered: 2026-03-27 via Smart Discuss (autonomous mode)*
