# Phase 2: Dashboard and Navigation - Research

**Researched:** 2026-03-25
**Domain:** TUI dashboard rendering, keyboard navigation, search/filter, overlay widgets (ratatui 0.30 / crossterm 0.29)
**Confidence:** HIGH

## Summary

Phase 2 transforms the Phase 1 stub table into a production dashboard with color-coded rows, aggregate status bar, vim-style navigation, inline search/filter with column selectors, a help overlay, and terminal resize handling. The existing codebase provides a strong foundation: `App` struct with `InputMode` enum, `TableState`, `project_states: HashMap<String, ProjectState>`, `sorted_aliases()`, and a footer that already switches on `InputMode`. All `ProjectState` fields needed for the new columns (status, current_phase, total_phases, completed_phases, backlog_count) are already parsed and available.

The key technical challenges are: (1) per-row color coding using `Row::style()` with a highlight style that uses only modifiers (bold+underline) to preserve the underlying status color -- ratatui's `Style::patch()` is additive, so a highlight with no foreground set will not overwrite row colors; (2) a centered popup overlay for the help screen using ratatui's `Clear` widget and `Rect::centered()` helper; (3) an inline filter mode in the footer that live-filters the table as the user types, with column selector syntax (`/term/key`).

**Primary recommendation:** Extend the existing `InputMode` enum with `Search` and `HelpOverlay` variants, add per-row `Row::style()` color coding in `render_main`, replace the footer with a richer aggregate status bar, and implement the help overlay as a `Clear` + `Block` + `Paragraph` drawn after the main layout. All changes fit cleanly into the existing TEA architecture with no structural rewrites needed.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Columns: `Alias | Current Phase Name | Status | Progress (e.g., 2/4) | Backlog` -- rich info per row, path moved to detail view
- **D-02:** Progress shown as fraction text only (`2/4 phases`) -- no inline progress bars
- **D-03:** Current phase displayed as number + truncated name: `"P2: Dashboard and Nav"` -- compact but informative
- **D-04:** Workflow state color mapping: Active/In Progress = Green, Idle/Ready to plan = Yellow, Blocked = Red, Complete = Dim/Gray, Unknown/Error = Magenta
- **D-05:** Selected row uses bold + underline (not reverse video) -- preserves status color so users can see state while navigating
- **D-06:** Aggregate counts use icon shorthand: `5 projects: 2 > 1 ! 1 * 1 +` -- dense, scannable
- **D-07:** Single-line footer: aggregate counts on left, keybind hints on right -- same layout as Phase 1 but richer content
- **D-08:** Filter triggered by `/`, inline in footer -- replaces keybind hints with filter input, live-filters as you type, Esc to clear and restore hints
- **D-09:** Filter matches across all visible columns (name, status, phase name) with fuzzy matching
- **D-10:** Column selector syntax: `/term/column_key` where column keys are: p = phase, n = name/alias, s = status, no suffix = match across all columns

### Claude's Discretion
- Help overlay (`?`) layout and content -- standard keybind reference panel
- Exact fuzzy matching algorithm (simple substring vs scored fuzzy)
- Terminal resize handling strategy (reflow vs redraw)
- Status icon choice (Unicode symbols are suggestions, can adjust for terminal compatibility)
- How to truncate phase names when terminal is narrow

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DASH-01 | Scrollable project list with name, current phase, and status per row | Existing `TableState` + `sorted_aliases()` + `ProjectState` fields; add new columns per D-01 |
| DASH-02 | Project rows color-coded by workflow state (idle, active, blocked, complete) | `Row::style()` with D-04 color map; `Style::patch()` additive behavior preserves colors under highlight |
| DASH-03 | Persistent status bar with aggregate counts across all projects | Footer left-side aggregate counting by status category; D-06 icon shorthand |
| NAV-01 | Vim-style keys (j/k, Enter, Esc, q) | Already implemented for j/k/q; add Enter (detail view stub) and ensure Esc returns from all modes |
| NAV-02 | Help overlay on `?` | `Clear` widget + `Rect::centered()` + `Block` + `Paragraph` overlay; new `InputMode::HelpOverlay` |
| NAV-03 | `/` to filter project list by name or status | New `InputMode::Search` variant; parse `/term/key` syntax; live-filter `sorted_aliases()` |
| NAV-04 | TUI adapts to terminal size changes | crossterm `Event::Resize` + ratatui auto-resize on `draw()`; add minimum size guard |
| NAV-05 | Clean exit with full terminal state restored | Already handled by `ratatui::init()`/`ratatui::restore()` panic hooks from Phase 1 |
</phase_requirements>

## Standard Stack

No new dependencies needed. Phase 2 uses only libraries already in Cargo.toml from Phase 1.

### Core (already installed)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30 | TUI rendering (Table, Block, Paragraph, Clear, Layout) | All widgets needed for dashboard, overlay, and footer are built-in |
| crossterm | 0.29 | Terminal backend, event stream (including Resize events) | Already wired in EventBus |
| tokio | 1 | Async runtime for event loop | Already running the main loop |

### Supporting (already installed)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| ratatui (style module) | 0.30 | `Style`, `Color`, `Modifier` for per-row color coding | Every row render |
| ratatui (layout module) | 0.30 | `Rect::centered()`, `Layout`, `Constraint` for help overlay and responsive layout | Help overlay, adaptive columns |

### Not Needed
| Library | Why Not |
|---------|---------|
| tui-popup | Built-in `Clear` + `Rect::centered()` is sufficient for a simple help overlay |
| tui-textarea | Single-line filter input is trivially rendered with `Paragraph` + `Span` (already done for add-alias input) |
| fuzzy-matcher / nucleo | Simple case-insensitive substring matching is sufficient for V1; no need for scored fuzzy ranking |

## Architecture Patterns

### Recommended Changes to Existing Structure

No new files needed except potentially a `src/ui/help_overlay.rs` for separation. All changes fit within existing modules.

```
src/
├── app.rs              # Extend InputMode with Search, HelpOverlay; add filter_text, filter_column fields
├── action.rs           # No changes needed (RawKey handles all new keys)
├── event.rs            # Add Event::Resize mapping to trigger redraw
├── ui/
│   ├── mod.rs          # Add help_overlay render call after main render
│   ├── project_list.rs # Rewrite table columns, per-row coloring, aggregate footer, filter logic
│   └── help_overlay.rs # NEW: Help overlay rendering (Clear + centered Block + Paragraph)
└── (all other files unchanged)
```

### Pattern 1: Per-Row Color Coding via Row::style()

**What:** Apply `Row::style(Style::default().fg(color))` based on the project's workflow state. The `Table::row_highlight_style()` uses only `Modifier::BOLD | Modifier::UNDERLINED` without setting foreground color, so `Style::patch()` preserves the row's status color on the selected row.

**When to use:** Every table render.

**Example:**
```rust
fn status_color(status: &str) -> Color {
    match status.to_lowercase().as_str() {
        s if s.contains("active") || s.contains("in progress") || s.contains("executing") => Color::Green,
        s if s.contains("idle") || s.contains("ready") => Color::Yellow,
        s if s.contains("blocked") => Color::Red,
        s if s.contains("complete") || s.contains("done") => Color::DarkGray,
        _ => Color::Magenta, // Unknown/Error
    }
}

// In render_main:
let row = Row::new(cells).style(Style::default().fg(status_color(&status)));

let table = Table::new(rows, &widths)
    .header(header)
    .row_highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED))
    .highlight_symbol("> ");
```

### Pattern 2: Inline Filter with Column Selector Parsing

**What:** When user presses `/`, switch to `InputMode::Search`. Footer shows filter input. Parse input: if it ends with `/p`, `/n`, or `/s`, extract the column key and filter term. Otherwise, match across all columns with case-insensitive substring.

**When to use:** Search mode active.

**Example:**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum FilterColumn {
    All,
    Name,
    Phase,
    Status,
}

fn parse_filter(input: &str) -> (String, FilterColumn) {
    if let Some(term) = input.strip_suffix("/p") {
        (term.to_string(), FilterColumn::Phase)
    } else if let Some(term) = input.strip_suffix("/n") {
        (term.to_string(), FilterColumn::Name)
    } else if let Some(term) = input.strip_suffix("/s") {
        (term.to_string(), FilterColumn::Status)
    } else {
        (input.to_string(), FilterColumn::All)
    }
}

// Filter sorted_aliases() based on parsed filter
fn filtered_aliases(&self) -> Vec<String> {
    let all = self.sorted_aliases();
    if self.filter_text.is_empty() {
        return all;
    }
    let (term, column) = parse_filter(&self.filter_text);
    let term_lower = term.to_lowercase();
    all.into_iter().filter(|alias| {
        let state = self.project_states.get(alias);
        match column {
            FilterColumn::Name => alias.to_lowercase().contains(&term_lower),
            FilterColumn::Phase => state.map_or(false, |s| s.current_phase.to_lowercase().contains(&term_lower)),
            FilterColumn::Status => state.map_or(false, |s| s.status.to_lowercase().contains(&term_lower)),
            FilterColumn::All => {
                alias.to_lowercase().contains(&term_lower)
                    || state.map_or(false, |s|
                        s.status.to_lowercase().contains(&term_lower)
                        || s.current_phase.to_lowercase().contains(&term_lower))
            }
        }
    }).collect()
}
```

### Pattern 3: Help Overlay with Clear Widget

**What:** When user presses `?`, toggle `InputMode::HelpOverlay`. Render the normal dashboard first, then draw a centered popup on top using `Clear` to erase the background area, followed by a bordered `Block` with keybinding reference content.

**When to use:** Help mode active.

**Example:**
```rust
// In ui/help_overlay.rs
pub fn render(frame: &mut Frame) {
    let area = frame.area().centered(
        Constraint::Percentage(60),
        Constraint::Percentage(70),
    );
    frame.render_widget(Clear, area);

    let help_text = vec![
        Line::from(Span::styled("Keybindings", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  j / Down    Move down"),
        Line::from("  k / Up      Move up"),
        Line::from("  Enter       Open project detail (Phase 3)"),
        Line::from("  /           Filter projects"),
        Line::from("  a           Add project"),
        Line::from("  d           Delete project"),
        Line::from("  ?           Toggle this help"),
        Line::from("  q / Esc     Quit / Back"),
        Line::from("  Ctrl+C      Force quit"),
        Line::from(""),
        Line::from(Span::styled("Filter Syntax", Style::default().add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from("  /term       Search all columns"),
        Line::from("  /term/n     Search name only"),
        Line::from("  /term/p     Search phase only"),
        Line::from("  /term/s     Search status only"),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help (press ? or Esc to close) ");
    let paragraph = Paragraph::new(help_text).block(block);
    frame.render_widget(paragraph, area);
}
```

### Pattern 4: Aggregate Status Bar

**What:** Count projects by workflow state category, display as icon shorthand on footer left. Keybind hints on footer right.

**Example:**
```rust
fn render_aggregate_footer(frame: &mut Frame, app: &App, area: Rect) {
    let total = app.config.projects.len();
    let mut active = 0;
    let mut blocked = 0;
    let mut idle = 0;
    let mut complete = 0;

    for state in app.project_states.values() {
        match status_category(&state.status) {
            StatusCategory::Active => active += 1,
            StatusCategory::Blocked => blocked += 1,
            StatusCategory::Idle => idle += 1,
            StatusCategory::Complete => complete += 1,
            StatusCategory::Unknown => idle += 1, // count unknown as idle
        }
    }

    let left = format!("{} projects: {} > {} ! {} * {} +", total, active, blocked, idle, complete);
    let right = "[/]search [?]help [a]dd [d]el [q]uit";
    // Render left-aligned and right-aligned in footer
}
```

### Pattern 5: Terminal Resize Handling

**What:** Map `crossterm::event::Event::Resize` in the EventBus to trigger a redraw. Ratatui's `terminal.draw()` automatically queries the new terminal size, so no explicit size tracking is needed. Add a minimum-size guard in the render function.

**Example:**
```rust
// In event.rs, extend map_event_to_action:
Event::Resize(_w, _h) => Some(Action::Resize),

// In app.rs update():
Action::Resize => {
    self.needs_redraw = true;
}

// In ui/project_list.rs, at the top of render():
if area.width < 40 || area.height < 8 {
    let msg = Paragraph::new("Terminal too small. Resize to at least 40x8.")
        .alignment(Alignment::Center);
    frame.render_widget(msg, area);
    return;
}
```

### Anti-Patterns to Avoid
- **Row::style overriding highlight:** Do NOT set foreground color in `row_highlight_style()` -- it would overwrite the per-row status color. Use only modifiers (bold, underline).
- **Filtering in render:** Do NOT compute filtered aliases inside the render function. Compute once in `update()` when filter text changes; store the result in `App`.
- **Parsing filter on every frame:** Parse the `/term/key` syntax once when input changes, not on every render call.
- **Blocking on resize:** Do NOT do any I/O or state recalculation on resize events. Just set `needs_redraw = true` and let the next draw call handle the new size.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Popup overlay | Custom z-layer management | ratatui `Clear` widget + `Rect::centered()` | Built-in, handles background clearing correctly |
| Per-row coloring | Custom cell-by-cell color management | `Row::style()` with `Style::patch()` additive behavior | Row-level style cascades to all cells automatically |
| Fuzzy matching | Scored fuzzy matcher with ranking | Case-insensitive `str::contains()` | V1 scope; 5-50 projects makes ranking irrelevant |
| Text input widget | Full text editor with cursor movement | Existing `Paragraph` + `input_buffer` + `Span` pattern from Phase 1 | Single-line input, already proven in add-alias flow |
| Terminal size detection | Manual SIGWINCH handling | crossterm `Event::Resize` + ratatui auto-resize in `draw()` | Already handled by the stack |

## Common Pitfalls

### Pitfall 1: Highlight Style Overwriting Row Colors
**What goes wrong:** Setting `row_highlight_style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD))` overwrites the per-row status color on the selected row, making all selected rows white regardless of status.
**Why it happens:** `Style::patch()` replaces `fg` if the patch style has a non-None `fg`.
**How to avoid:** Use `row_highlight_style(Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED))` with NO foreground color set. The additive patch will add modifiers while preserving the existing row fg color.
**Warning signs:** Selected row loses its status color.

### Pitfall 2: Filter State Desync with Table Selection
**What goes wrong:** User filters the list (reducing visible rows), but `TableState::selected()` still points to the old index. The selection jumps to the wrong row or panics on out-of-bounds.
**Why it happens:** `TableState` indices are absolute into the rows vec passed to `Table::new()`. When the filtered list is shorter, the old selection index may exceed the new length.
**How to avoid:** When filter text changes, clamp `table_state.select(Some(0))` (reset to first visible row) or clamp to `min(current, filtered_count - 1)`. Store filtered aliases separately from full aliases.
**Warning signs:** Selection disappears or jumps erratically when typing in filter.

### Pitfall 3: Unicode Status Icons Breaking Column Alignment
**What goes wrong:** Status icons like `>` `!` `*` `+` display correctly, but if you use emoji like `▶` `⚠` the terminal may render them as 2 columns wide, breaking alignment.
**Why it happens:** Unicode width varies by terminal emulator. Multi-codepoint sequences are inconsistent.
**How to avoid:** Use ASCII-safe icons (`>` `!` `*` `+`) or single-codepoint Unicode that is consistently 1 column wide. Test in tmux (strictest renderer). The context decision D-06 suggests Unicode icons as suggestions but allows ASCII fallbacks.
**Warning signs:** Footer aggregate counts misalign in tmux or VS Code terminal.

### Pitfall 4: Phase Name Truncation Without Indicator
**What goes wrong:** Long phase names like "Visualization, Creation, and Enqueue" overflow the column, pushing other columns off screen.
**Why it happens:** No truncation logic; ratatui renders the full string.
**How to avoid:** Truncate phase names to column width minus 1, append ellipsis character `...` when truncated. Use the D-03 format: `"P2: Dashboard and Nav..."`.
**Warning signs:** Table columns misalign when a project has a long phase name.

### Pitfall 5: Help Overlay Not Dismissing on Expected Keys
**What goes wrong:** User presses `?` to open help, then presses `q` expecting to close it, but `q` quits the entire app.
**Why it happens:** Key dispatch doesn't check InputMode before processing quit.
**How to avoid:** In `handle_key`, when `InputMode::HelpOverlay`, only `?` and `Esc` dismiss the overlay. All other keys are consumed (no-op). Do NOT fall through to normal key handling.
**Warning signs:** App quits when user presses `q` in help overlay.

## Code Examples

### Current Phase Name Formatting (D-03)
```rust
fn format_phase_display(state: &ProjectState) -> String {
    // Find the current phase from roadmap phases
    let phase_num = state.completed_phases + 1;
    let phase_name = state.phases.iter()
        .find(|p| p.number == phase_num.to_string())
        .map(|p| p.name.as_str())
        .unwrap_or("Unknown");

    let full = format!("P{}: {}", phase_num, phase_name);
    // Truncation handled at render time based on column width
    full
}
```

### Status Category Classification
```rust
enum StatusCategory {
    Active,
    Idle,
    Blocked,
    Complete,
    Unknown,
}

fn classify_status(status: &str) -> StatusCategory {
    let s = status.to_lowercase();
    if s.contains("executing") || s.contains("active") || s.contains("in progress") {
        StatusCategory::Active
    } else if s.contains("blocked") {
        StatusCategory::Blocked
    } else if s.contains("complete") || s.contains("done") {
        StatusCategory::Complete
    } else if s.contains("idle") || s.contains("ready") || s.contains("plan") {
        StatusCategory::Idle
    } else {
        StatusCategory::Unknown
    }
}
```

### InputMode Extension
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    AddAlias,
    AddPath { alias: String },
    DeleteConfirm { alias: String },
    Search,       // NEW: filter input active
    HelpOverlay,  // NEW: help popup visible
}
```

### App Fields Extension
```rust
pub struct App {
    // ... existing fields ...
    pub filter_text: String,           // NEW: current filter input
    pub filtered_aliases: Vec<String>, // NEW: cached filtered result
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `Table::highlight_style()` | `Table::row_highlight_style()` | ratatui 0.28+ | Old method deprecated; use `row_highlight_style()` |
| Manual centered rect calculation | `Rect::centered()` helper | ratatui 0.28+ | Simpler popup positioning |
| Manual terminal size tracking | `terminal.draw()` auto-queries size | Always (ratatui design) | No need for explicit resize handling beyond triggering redraw |

**Deprecated/outdated:**
- `Table::highlight_style()` -- use `Table::row_highlight_style()` instead (ratatui 0.28+)

## Open Questions

1. **Status string variations across GSD projects**
   - What we know: STATE.md frontmatter has a `status` field with values like "Ready to plan", "Executing"
   - What's unclear: Full set of possible status strings across different GSD projects
   - Recommendation: Use `contains()` matching (as shown in `classify_status`) rather than exact string matching. This handles variations like "Ready to plan Phase 2" or "Executing plan 3". Log unclassified statuses for future refinement.

2. **Enter key behavior in Phase 2**
   - What we know: NAV-01 requires Enter to select. Detail view is Phase 3.
   - What's unclear: What Enter should do in Phase 2 before detail view exists.
   - Recommendation: Bind Enter now but show a status message "Detail view coming in Phase 3" or no-op. This keeps the keybinding discoverable in help without requiring Phase 3 implementation.

## Sources

### Primary (HIGH confidence)
- [ratatui Table widget docs](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Table.html) - row_highlight_style, Row::style, TableState interaction
- [ratatui Row widget docs](https://docs.rs/ratatui/latest/ratatui/widgets/struct.Row.html) - Row::style additive behavior with cells
- [ratatui Style docs](https://docs.rs/ratatui/latest/ratatui/style/struct.Style.html) - Style::patch() additive behavior confirmed; modifiers-only highlight preserves fg color
- [ratatui Popup example](https://ratatui.rs/examples/apps/popup/) - Clear widget + centered rect pattern
- [ratatui centering helpers](https://ratatui.rs/recipes/layout/center-a-widget/) - Rect::centered(), centered_horizontally(), centered_vertically()
- [ratatui FAQ](https://ratatui.rs/faq/) - Resize handling, event handling delegation to backend
- Existing codebase: `src/app.rs`, `src/ui/project_list.rs`, `src/event.rs`, `src/state_reader/mod.rs` - Phase 1 implementation patterns

### Secondary (MEDIUM confidence)
- [crossterm event docs](https://docs.rs/crossterm/latest/crossterm/event/index.html) - Event::Resize variant

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new deps, all ratatui 0.30 APIs verified against docs.rs
- Architecture: HIGH - extends proven Phase 1 patterns; all widgets are built-in ratatui
- Pitfalls: HIGH - Style::patch() additive behavior verified in official docs; filter/selection desync is a known ratatui pattern
- Color coding: HIGH - Row::style + modifiers-only highlight confirmed to preserve fg color

**Research date:** 2026-03-25
**Valid until:** 2026-04-25 (stable stack, no version changes expected)
