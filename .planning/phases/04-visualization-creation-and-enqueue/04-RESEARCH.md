# Phase 4: Visualization, Creation, and Enqueue - Research

**Researched:** 2026-03-25
**Domain:** TUI roadmap visualization, project scaffolding, work queue management
**Confidence:** HIGH

## Summary

Phase 4 adds three distinct feature clusters to the existing TUI: (1) an ASCII roadmap visualization toggled within the detail view, (2) a project creation flow that creates a directory + `git init` + auto-registers, and (3) a work enqueue system that writes commands to `.planning/QUEUE.md` with state-aware suggestions. All three build on existing patterns -- TEA action dispatch, modal input modes, and the detail view scaffold from Phase 3.

The technical risk is low. The roadmap visualization is a custom ratatui Widget rendering boxes connected by vertical arrows -- straightforward box-drawing with the existing `RoadmapPhase` data. Project creation uses `std::process::Command` for `git init` (no git2 FFI dependency needed). The enqueue feature introduces a new file format (QUEUE.md) that must be simple enough for manual editing while being parseable.

**Primary recommendation:** Split into 3 plans: (1) ASCII roadmap widget + detail view toggle, (2) project creation modal flow with `git init` + hooks, (3) enqueue system with QUEUE.md read/write and command suggestions.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Roadmap visualization appears inside the detail view -- press `r` to toggle between phase list and roadmap view
- **D-02:** Vertical pipeline style: phases as boxes connected by `|` arrows, with progress fill and status icon per box
- **D-03:** Each phase box shows: phase number, name (truncated), status icon, plan count -- 3-line box format
- **D-04:** Current phase highlighted with bold/bright border + marker
- **D-05:** Press `c` from dashboard -- modal input: name -> path -> confirm. Creates directory, `git init`, auto-registers in manager. Does NOT initialize GSD -- user runs `/gsd:new-project` themselves
- **D-06:** Path input supports `~` expansion (to `$HOME`), relative-to-pwd resolution, and tab autocompletion for directory paths
- **D-07:** Hook script support -- configurable pre/post-create hooks in `~/.config/gsd-manager/config.json` with params: `name`, `path`, `alias`. Hooks are shell commands executed with environment variables `GSD_PROJECT_NAME`, `GSD_PROJECT_PATH`, `GSD_PROJECT_ALIAS`
- **D-08:** Newly created project appears in dashboard immediately after creation (auto-registered + file watcher starts)
- **D-09:** Press `e` in detail view -- free-form text input for commands. Suggestions/autocomplete for GSD commands based on the project's current state
- **D-10:** Enqueued actions stored in each project's `.planning/QUEUE.md` -- one action per line. GSD can check this file when it thinks it's done
- **D-11:** Enqueued actions visible in the detail view as a "Queued" section below the phase list
- **D-12:** Clipboard copy feature dropped -- not needed

### Claude's Discretion
- Exact box dimensions and ASCII art style for roadmap visualization
- Tab completion implementation (filesystem walk vs shell integration)
- QUEUE.md format (simple markdown list vs structured format)
- How to determine "next logical step" for command suggestions
- Whether to show confirmation dialog before `git init`

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DASH-04 | ASCII roadmap visualization showing phase structure and progress | Custom ratatui Widget with box-drawing chars, uses existing `ProjectState.phases` data |
| CREATE-01 | Create new GSD project from TUI (name, directory path) | Multi-step modal input (CreateName -> CreatePath -> CreateConfirm), reuses existing AddAlias/AddPath patterns |
| CREATE-02 | Initialize directory, git repo, and import global GSD settings | `std::fs::create_dir_all` + `std::process::Command::new("git").arg("init")`, no GSD init per D-05 |
| CREATE-03 | Newly created project auto-registered in manager | Reuse `registry::add_project()` but skip `.planning/` validation since project is fresh |
| ENQ-01 | Enqueue next action for a project | Free-form text input in detail view, write to `.planning/QUEUE.md` |
| ENQ-02 | Enqueued actions visible in detail view | Parse QUEUE.md on project state load, render below phase list |
| ENQ-03 | V1 copies GSD command to clipboard | D-12 dropped clipboard -- instead, store in QUEUE.md for GSD to read |
</phase_requirements>

## Standard Stack

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30.0 | TUI rendering + custom Widget trait | Already in use; Widget trait is the mechanism for roadmap viz |
| crossterm | 0.29.0 | Terminal backend | Already in use |
| tokio | 1.x | Async runtime | Already in use; needed for hook execution (spawn_blocking) |
| serde / serde_json | 1.x | Config serialization | Already in use; extend Config struct for hooks |
| dirs | 6.x | Home directory resolution | Already in use; needed for `~` expansion in path input |
| anyhow | 1.x | Error handling | Already in use |

### No New Dependencies Required

All Phase 4 features can be built with the existing dependency set:
- Roadmap viz: custom `Widget` implementation using ratatui primitives
- Project creation: `std::process::Command` (stdlib) for `git init`
- QUEUE.md: `std::fs` read/write (stdlib) + simple line-based parsing
- Hook execution: `std::process::Command` with env vars
- Path tilde expansion: `dirs::home_dir()` (already a dependency)

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `std::process::Command` for git | `git2` (0.20.4) | Adds libgit2 C FFI dependency, ~30s compile time increase, only need `git init` -- overkill |
| Manual QUEUE.md parsing | serde YAML/TOML | Overengineered for a simple line-based file; users want to edit it manually |
| Manual tilde expansion | `shellexpand` crate | 1-line implementation with `dirs::home_dir()`, not worth a new dependency |

## Architecture Patterns

### Recommended Changes to Project Structure
```
src/
├── app.rs               # Extend InputMode with CreateName, CreatePath, CreateConfirm, Enqueue
├── action.rs            # Extend Action with CreateProject, EnqueueAction, ToggleRoadmap
├── config.rs            # Extend Config with hooks field
├── registry.rs          # Add add_project_unchecked() for fresh projects (no .planning/ validation)
├── state_reader/
│   └── queue_md.rs      # NEW: Parse .planning/QUEUE.md into Vec<QueuedAction>
├── ui/
│   ├── detail_view.rs   # Extend with roadmap viz toggle, queued section, `r`/`e` keys
│   ├── roadmap_widget.rs # NEW: Custom Widget for vertical pipeline roadmap
│   ├── help_overlay.rs  # Update with new keybindings
│   └── project_list.rs  # Add `c` key hint in footer
└── project_creator.rs   # NEW: Directory creation, git init, hook execution
```

### Pattern 1: Custom Widget for Roadmap Visualization
**What:** Implement ratatui's `Widget` trait for a `RoadmapWidget` that renders a vertical pipeline of phase boxes.
**When to use:** When rendering the ASCII roadmap in the detail view.

```rust
// Source: ratatui.rs/recipes/widgets/custom/
pub struct RoadmapWidget<'a> {
    phases: &'a [RoadmapPhase],
    current_phase_num: u32,
    completed_phases: u32,
}

impl<'a> Widget for RoadmapWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Each phase box: 3 lines tall + 1 line for connector
        // Box format (D-03):
        // ┌──────────────────────┐
        // │ + P1: Core Infra 3/3 │
        // └──────────────────────┘
        //            │
        //            ▼
        // ┌──────────────────────┐
        // │ * P2: Dashboard  2/2 │
        // └──────────────────────┘

        let box_height = 3u16; // top border + content + bottom border
        let connector_height = 2u16; // "│" + "▼"
        let phase_block = box_height + connector_height; // 5 lines per phase

        for (i, phase) in self.phases.iter().enumerate() {
            let y_offset = i as u16 * phase_block;
            if y_offset + box_height > area.height {
                break; // Stop rendering if we run out of space
            }
            // Render box with status-colored border for current phase
            // ... (detailed implementation in plan)
        }
    }
}
```

### Pattern 2: Multi-Step Modal Input for Project Creation
**What:** Chain of InputMode states: `CreateName` -> `CreatePath { name }` -> `CreateConfirm { name, path }` following the established AddAlias -> AddPath pattern.
**When to use:** Project creation flow (D-05).

```rust
// Follows existing pattern from AddAlias/AddPath:
pub enum InputMode {
    // ... existing variants ...
    CreateName,
    CreatePath { name: String },
    CreateConfirm { name: String, path: PathBuf },
    EnqueueInput { alias: String },
}
```

### Pattern 3: Detail View Sub-Mode for Roadmap Toggle
**What:** Track whether the detail view shows the phase list (default) or the roadmap visualization. Per D-01, `r` toggles between them. Store as a field on App, not in InputMode (to avoid breaking the existing DetailView variant).

```rust
// In app.rs:
pub enum DetailSubView {
    PhaseList,  // default, existing behavior
    RoadmapViz, // new, shows ASCII pipeline
}

// In App struct:
pub detail_sub_view: DetailSubView,
// Per D-09 specifics: remember per project within session
pub detail_sub_view_per_project: HashMap<String, DetailSubView>,
```

### Pattern 4: Hook Execution with Environment Variables
**What:** Run shell commands with `GSD_PROJECT_NAME`, `GSD_PROJECT_PATH`, `GSD_PROJECT_ALIAS` env vars. Use `tokio::task::spawn_blocking` to avoid blocking the event loop.

```rust
fn execute_hook(hook_cmd: &str, name: &str, path: &Path, alias: &str) -> anyhow::Result<()> {
    let status = std::process::Command::new("sh")
        .args(["-c", hook_cmd])
        .env("GSD_PROJECT_NAME", name)
        .env("GSD_PROJECT_PATH", path.display().to_string())
        .env("GSD_PROJECT_ALIAS", alias)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;
    if !status.success() {
        anyhow::bail!("Hook failed with exit code: {:?}", status.code());
    }
    Ok(())
}
```

### Anti-Patterns to Avoid
- **Blocking git init on the main thread:** `git init` and hook execution must use `spawn_blocking` or be dispatched as async Actions to avoid freezing the TUI render loop
- **Validating .planning/ on fresh project create:** The existing `registry::add_project()` validates `.planning/` exists, but for CREATE flow the project is brand new with no `.planning/` -- need a separate code path
- **Storing QUEUE.md path in ProjectState:** QUEUE.md is a separate concern from GSD state reading; parse it separately and store in a distinct structure or add an optional field

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Home dir resolution | Manual `$HOME` env var lookup | `dirs::home_dir()` | Cross-platform, handles edge cases (Windows, macOS) |
| Git repository init | Raw `.git/` directory creation | `Command::new("git").arg("init")` | Git handles internal format, config, hooks dir correctly |
| Box-drawing characters | Custom char arrays | Unicode box-drawing block (`\u250C`, `\u2502`, `\u2514`, etc.) | Standard set, renders consistently in modern terminals |
| Path canonicalization | Manual `..` resolution | `std::fs::canonicalize()` | Handles symlinks, relative paths, OS differences |

## Common Pitfalls

### Pitfall 1: Roadmap Widget Overflows Terminal Height
**What goes wrong:** With 10+ phases, the vertical pipeline exceeds the available terminal height, rendering garbage or panicking on buffer out-of-bounds.
**Why it happens:** Each phase box is 5 lines (3 box + 2 connector). 10 phases = 50 lines. Small terminals may only have 24 rows.
**How to avoid:** The roadmap widget must: (a) clamp rendering to available area height, (b) support scroll offset (reuse `detail_scroll_offset`), (c) auto-center on the current phase.
**Warning signs:** Widget renders correctly with 4 phases but fails with 8+.

### Pitfall 2: Blocking the Event Loop During Project Creation
**What goes wrong:** `git init` or hook scripts take > 100ms, causing the TUI to freeze and miss input events.
**Why it happens:** `std::process::Command::status()` blocks the calling thread.
**How to avoid:** Use `tokio::task::spawn_blocking` for `git init` and hook execution. Send completion/failure back as an Action through the EventBus.
**Warning signs:** TUI becomes unresponsive during project creation.

### Pitfall 3: Registry Validation Blocks Fresh Project Registration
**What goes wrong:** `registry::add_project()` requires `.planning/` to exist, but freshly created projects don't have it yet.
**Why it happens:** The validation was correct for the "register existing project" use case but wrong for "create new project."
**How to avoid:** Add `add_project_unchecked()` or a `skip_planning_check` parameter for the creation flow. The project won't have state data until the user runs GSD in it.
**Warning signs:** Project creation appears to succeed but registration fails.

### Pitfall 4: Tilde Expansion Not Applied Before Path Operations
**What goes wrong:** User types `~/projects/new-app`, but `PathBuf::from("~/projects/new-app")` treats `~` as a literal directory name.
**Why it happens:** The shell expands `~` but TUI text input bypasses the shell.
**How to avoid:** Implement explicit tilde expansion: if path starts with `~`, replace with `dirs::home_dir()`. Do this before any `canonicalize()`, `exists()`, or `create_dir_all()` call.
**Warning signs:** "No such file or directory" errors when user enters paths with `~`.

### Pitfall 5: QUEUE.md Written While GSD is Reading It
**What goes wrong:** Race condition between the manager writing QUEUE.md and a GSD session reading it.
**Why it happens:** Both processes access the same file.
**How to avoid:** Use atomic write (tempfile + rename, same pattern as `config.rs::save_config`). GSD reads are safe because rename is atomic on the same filesystem.
**Warning signs:** Truncated or empty QUEUE.md seen by GSD.

### Pitfall 6: FileWatcher Not Started for Newly Created Projects
**What goes wrong:** New project appears in dashboard but never auto-refreshes.
**Why it happens:** The file watcher is initialized at startup for existing projects. Newly created projects don't have `.planning/` yet, so there's nothing to watch. Even if the user later runs GSD and creates `.planning/`, no watcher is set up.
**How to avoid:** After project creation, start a watcher on the project root. When `.planning/` appears (detected by watcher), switch to watching `.planning/` specifically. Or: only start watching when the user first drills into the project and `.planning/` exists.
**Warning signs:** New project shows "unknown" state forever even after GSD runs in it.

## Code Examples

### Tilde Expansion Utility
```rust
// Source: dirs crate + stdlib
fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    } else if path == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path)
}
```

### QUEUE.md Format (Recommendation)
```markdown
# Queue

- /gsd:plan-phase 4
- /gsd:execute-phase 4
# This is a comment -- ignored by parser
```

**Parser:** Read lines, skip empty lines and lines starting with `#`, trim whitespace. Each remaining line is one queued command.

```rust
pub struct QueuedAction {
    pub command: String,
}

pub fn parse_queue_md(content: &str) -> Vec<QueuedAction> {
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#') && !l.starts_with("# "))
        .filter_map(|l| l.strip_prefix("- ").or(Some(l)))
        .map(|cmd| QueuedAction { command: cmd.to_string() })
        .collect()
}

pub fn write_queue_md(actions: &[QueuedAction]) -> String {
    let mut content = String::from("# Queue\n\n");
    for action in actions {
        content.push_str(&format!("- {}\n", action.command));
    }
    content
}
```

### GSD Command Suggestions Based on State
```rust
fn suggest_next_commands(state: &ProjectState) -> Vec<String> {
    let status = state.status.to_lowercase();
    let next_phase = state.completed_phases + 1;

    if status.contains("ready to plan") || status.contains("idle") {
        vec![
            format!("/gsd:discuss-phase {}", next_phase),
            format!("/gsd:plan-phase {}", next_phase),
        ]
    } else if status.contains("executing") || status.contains("active") {
        vec![
            format!("/gsd:execute-phase {}", next_phase),
            format!("/gsd:verify-work {}", next_phase),
        ]
    } else if status.contains("complete") || status.contains("done") {
        vec![
            "/gsd:progress".to_string(),
        ]
    } else {
        vec![
            format!("/gsd:discuss-phase {}", next_phase),
            format!("/gsd:plan-phase {}", next_phase),
            format!("/gsd:execute-phase {}", next_phase),
        ]
    }
}
```

### Box-Drawing Characters Reference
```rust
// Unicode box-drawing characters for roadmap visualization
const BOX_TOP_LEFT: &str = "\u{250C}";     // ┌
const BOX_TOP_RIGHT: &str = "\u{2510}";    // ┐
const BOX_BOTTOM_LEFT: &str = "\u{2514}";  // └
const BOX_BOTTOM_RIGHT: &str = "\u{2518}"; // ┘
const BOX_HORIZONTAL: &str = "\u{2500}";   // ─
const BOX_VERTICAL: &str = "\u{2502}";     // │
const CONNECTOR_VERTICAL: &str = "\u{2502}"; // │
const CONNECTOR_ARROW: &str = "\u{25BC}";  // ▼
const CURRENT_MARKER: &str = "\u{25B6}";   // ▶

// Bold variants for current phase (D-04)
const BOLD_TOP_LEFT: &str = "\u{250F}";    // ┏
const BOLD_TOP_RIGHT: &str = "\u{2513}";   // ┓
const BOLD_BOTTOM_LEFT: &str = "\u{2517}"; // ┗
const BOLD_BOTTOM_RIGHT: &str = "\u{251B}"; // ┛
const BOLD_HORIZONTAL: &str = "\u{2501}";  // ━
const BOLD_VERTICAL: &str = "\u{2503}";    // ┃
```

### Project Creation with git init
```rust
// Source: std::process::Command docs
fn create_project(name: &str, path: &Path) -> anyhow::Result<()> {
    // Create directory
    std::fs::create_dir_all(path)
        .with_context(|| format!("Failed to create directory: {}", path.display()))?;

    // git init
    let status = std::process::Command::new("git")
        .arg("init")
        .current_dir(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .with_context(|| "Failed to run git init")?;

    if !status.success() {
        anyhow::bail!("git init failed with exit code: {:?}", status.code());
    }

    Ok(())
}
```

## Config Extension for Hooks

The existing `Config` struct needs a `hooks` field. Per D-07, hooks are shell commands with env vars.

```rust
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Preferences {
    #[serde(default)]
    pub hooks: HooksConfig,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct HooksConfig {
    pub pre_create: Option<String>,  // Shell command to run before project creation
    pub post_create: Option<String>, // Shell command to run after project creation
}
```

**Note:** The CONTEXT.md says hooks go in `config.json` but the existing config file IS `config.json` (using serde_json), so this maps directly. Just extend the existing Config/Preferences structs.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| tui-rs Widget trait | ratatui Widget trait (same signature) | 2023 (ratatui fork) | No change needed; same `fn render(self, area: Rect, buf: &mut Buffer)` |
| git2 for simple init | std::process::Command | Ongoing | Avoids C FFI; git2 still valid for complex operations |

## Open Questions

1. **Tab Autocompletion for Path Input (D-06)**
   - What we know: User wants tab completion for directory paths during project creation
   - What's unclear: How deep should the completion go? Full filesystem walk is expensive; directory-only is simpler
   - Recommendation: Start with `~` expansion + relative path resolution. Tab key shows immediate children of the currently typed directory prefix (one level only). Pressing Tab cycles through matches. This is sufficient for v1 and avoids the complexity of full shell-style completion.

2. **QUEUE.md File Location for Projects Without .planning/**
   - What we know: Newly created projects won't have `.planning/` until GSD initializes them
   - What's unclear: Where to store queued actions for fresh projects
   - Recommendation: Don't write QUEUE.md until `.planning/` exists. Show a message in the enqueue UI: "Run GSD in this project first to enable queue." This keeps the contract simple.

3. **Confirmation Before git init (Claude's Discretion)**
   - Recommendation: Show the CreateConfirm step with name + resolved path. This serves as implicit confirmation. No separate "are you sure?" dialog for git init specifically -- the confirm step covers both directory creation and git init.

## Sources

### Primary (HIGH confidence)
- [ratatui custom widget recipe](https://ratatui.rs/recipes/widgets/custom/) - Widget trait signature, implementation pattern
- [ratatui Widget trait docs](https://docs.rs/ratatui/latest/ratatui/widgets/trait.Widget.html) - Official API reference
- [std::process::Command docs](https://doc.rust-lang.org/std/process/struct.Command.html) - git init invocation pattern
- Existing codebase: `src/app.rs`, `src/config.rs`, `src/registry.rs`, `src/ui/detail_view.rs` - Established patterns for modal input, config persistence, rendering

### Secondary (MEDIUM confidence)
- [git2-rs init example](https://github.com/rust-lang/git2-rs/blob/master/examples/init.rs) - Alternative approach (not recommended)
- cargo search results - git2 0.20.4 confirmed available

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies, all patterns verified in existing codebase
- Architecture: HIGH - extends established TEA pattern, InputMode, and Widget trait
- Pitfalls: HIGH - based on direct codebase analysis (registry validation, event loop blocking)
- Roadmap viz: HIGH - ratatui Widget trait is well-documented, box-drawing is straightforward
- Project creation: HIGH - std::process::Command is stdlib, git is available on system
- Enqueue: MEDIUM - QUEUE.md format is new (no prior art in GSD ecosystem)

**Research date:** 2026-03-25
**Valid until:** 2026-04-25 (stable domain, no fast-moving dependencies)
