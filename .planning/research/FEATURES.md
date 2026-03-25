# Feature Research

**Domain:** TUI multi-project management dashboard (GSD workflow orchestration)
**Researched:** 2026-03-24
**Confidence:** HIGH (cross-referenced against lazygit, k9s, lazydocker, taskwarrior-tui, btop, and GSD file structure docs)

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features users assume exist. Missing these = product feels incomplete. These are drawn from universal patterns across lazygit, k9s, lazydocker, and btop — tools that define what "a good TUI" means to the target audience.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Project list panel | Every multi-resource TUI leads with an index pane (k9s, lazydocker, lazygit branch list) | LOW | Scrollable list of registered projects with name + status summary per row |
| Current phase/status per project | Users open the tool specifically to know "what's happening" — must be on the main view | LOW | Read from `.planning/STATE.md`; show phase name, milestone, last-updated timestamp |
| Keyboard-driven navigation | Target users are developers in terminal environments — mouse-dependence is a regression | LOW | j/k (or arrows) to move, Enter to select, q/Esc to go back; vim-modal standard |
| Help overlay | Every mature TUI has `?` → keybinding cheatsheet (k9s, lazygit, lazydocker all do this) | LOW | Full-screen or popup modal listing all active keybindings for current view |
| Search / filter across projects | Projects accumulate; users need to find one fast — `/` search is universal in the domain | LOW | Inline filter on project list; matches on name and status |
| Detail view / drill-down | Selecting a project must open a detail pane — otherwise the list is read-only | MEDIUM | Show project path, current phase, roadmap progress, last activity |
| Status-aware visual indicators | btop, k9s, htop all use color + symbol to signal state at a glance — users scan not read | LOW | Color-coded rows (idle/active/blocked/complete); status icons |
| Responsive terminal layout | Must work in any terminal size without crashing or corrupting output | MEDIUM | Constraint-based layout (ratatui style); min-width graceful degradation |
| Exit/quit without side effects | Any TUI that quits weirdly (corrupted terminal, missing newlines) is immediately untrusted | LOW | Proper terminal restore on exit; handle Ctrl+C and q equally |
| Project registration (add/remove) | Users need to onboard their projects — there's no value without at least one registered project | MEDIUM | Add by path; validate that `.planning/` exists; persist to config |

### Differentiators (Competitive Advantage)

These features are specific to the GSD domain and represent the real reason users would choose this tool over a generic TUI or just running `/gsd:progress` in each project directory.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| GSD roadmap ASCII visualization | See the full phase/milestone graph for a project without opening any file — unique to this domain | HIGH | Parse `ROADMAP.md` and render phase boxes with progress markers; similar to git graph in lazygit |
| Enqueue next action | Queue a phase, task, or verification from the TUI — closes the "I need to switch terminals to issue a command" gap | HIGH | Writes to a queue file that GSD agents pick up; requires agreed format with GSD; v1 can be clipboard-paste of command |
| Phase progress indicators | Show how many tasks are done in the current phase, not just which phase is active | MEDIUM | Parse phase `PLAN.md` and count checked vs unchecked tasks |
| Multi-project at-a-glance status bar | One-line health summary across all projects shown permanently (e.g., "5 projects: 2 active, 1 blocked, 2 idle") | LOW | Trivially derived from project list state; high signal-to-noise value |
| Create new GSD project from TUI | Spin up a new project without leaving the tool — directory, git init, GSD initialization | HIGH | Shell out to `git init` and GSD setup; complex but completes the management loop |
| File-based state polling (no agent required) | Read status without triggering Claude — passive, non-intrusive, fast | MEDIUM | inotify/kqueue watch on `.planning/` dirs, or periodic polling with configurable interval |
| Roadmap diff / last-change summary | Show what changed in `.planning/` since last visit — "Phase 3 completed 2h ago" | MEDIUM | Stat mtime on key files; diff STATE.md or ROADMAP.md for change detection |
| Backlog item count per project | Surface that a project has queued work even if no phase is currently active | LOW | Count `999.*` dirs in `.planning/`; single integer in project row |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| Embedded terminal / Claude session hooking | "I want to run commands directly from the TUI" | High complexity, requires screen/tmux integration, platform-specific, fragile — explicitly deferred to v2 in PROJECT.md | Open project dir in a new terminal pane; copy command to clipboard |
| Real-time streaming log view | Users want to see Claude's output as it runs | GSD sessions run in separate Claude Code processes; there is no stdout to tail. Polling `.planning/` files is the right model | File-watch STATE.md and render its last-modified content |
| Plugin / extension system | Power users want to customize everything | Premature architecture. Cannot design a good plugin boundary before the core API stabilizes. Adds maintenance burden from day one | Hardcode v1 behavior; extract plugin boundaries in v2 after usage patterns are clear |
| Remote project management | "I have projects on another machine" | SSH + file parsing adds a full networking layer. v1 is local-only per PROJECT.md | Out of scope; point users to SSH port-forwarding if desperate |
| GUI / web companion | "A web dashboard would be nicer" | Defeats the entire purpose of a TUI; target audience lives in the terminal | TUI is the product; no companion app |
| Global keybinding daemon / system tray | Background monitoring with notifications | Platform-specific, requires daemon management, out of TUI scope | Auto-refresh polling in the TUI while it's open is sufficient |
| Animated transitions / loading spinners | "Makes it feel polished" | Animations are a TUI anti-pattern — they increase perceived latency, break screen readers, and add rendering complexity for zero productivity gain | Instant view transitions; status indicators via color/symbol only |

---

## Feature Dependencies

```
[Project Registration]
    └──required by──> [Project List Panel]
                          └──required by──> [Detail View / Drill-down]
                          └──required by──> [Roadmap ASCII Visualization]
                          └──required by──> [Phase Progress Indicators]
                          └──required by──> [Enqueue Next Action]
                          └──required by──> [Roadmap Diff / Last-change]

[File-Based State Polling]
    └──enables──> [Status-Aware Visual Indicators]
    └──enables──> [Phase Progress Indicators]
    └──enables──> [Backlog Item Count]
    └──enables──> [Multi-Project Status Bar]

[Help Overlay] ──independent──> (all views)

[Search / Filter] ──enhances──> [Project List Panel]

[Create New GSD Project] ──requires──> [Project Registration]
    (new project must auto-register after creation)

[Enqueue Next Action] ──requires──> [Detail View / Drill-down]
    (must know which project and phase to enqueue against)
```

### Dependency Notes

- **Project Registration requires validation**: Must confirm `.planning/` exists before registering — otherwise state polling will silently fail.
- **File-Based State Polling enables everything dynamic**: Without it, the TUI is a static snapshot. This is the core engine that makes the dashboard live.
- **Enqueue Next Action is loosely coupled**: v1 can implement this as "copy GSD command to clipboard" — actual GSD-queue integration is v2 after queue format is defined.
- **Roadmap ASCII Visualization is independent from Enqueue**: Users get read value before write value is implemented. Build in that order.

---

## MVP Definition

### Launch With (v1)

Minimum viable product — validates "unified dashboard without opening separate terminals."

- [ ] Project registration (add by path, remove, persist to config) — no projects = no value
- [ ] Project list panel with name, current phase, status indicator, last-updated — the core view
- [ ] File-based state polling (read `.planning/STATE.md` and `ROADMAP.md`) — makes data live
- [ ] Keyboard navigation (j/k, Enter, q, Esc, ?) — non-negotiable for developer TUI
- [ ] Help overlay (? key) — trust signal; users immediately check if keybindings are discoverable
- [ ] Detail view showing project path, roadmap phases, current phase, task completion count — drill-down validates the tool beyond a list
- [ ] Multi-project status bar (count by status) — headline metric users quote when recommending the tool
- [ ] Search/filter on project list — needed once user has more than ~5 projects

### Add After Validation (v1.x)

Add once core is working and users confirm the read-model is valuable.

- [ ] Roadmap ASCII visualization — when users say "I wish I could see the full roadmap without opening files"
- [ ] Phase progress indicators (task count from PLAN.md) — when users ask "how far into this phase are we?"
- [ ] Backlog item count per project — when users start using the backlog feature actively
- [ ] Roadmap diff / last-change summary — when users complain "I don't know what changed since yesterday"
- [ ] Create new GSD project from TUI — when users report friction of switching away to initialize projects

### Future Consideration (v2+)

Defer until product-market fit is established and usage patterns are clear.

- [ ] Enqueue next action (write to GSD queue) — requires GSD queue format to be defined first; do not design in isolation
- [ ] Claude session hooking / embedded terminal — high complexity, platform-specific; PROJECT.md explicitly defers this
- [ ] Plugin system — design plugin boundaries only after 3+ real usage patterns emerge
- [ ] Remote project management — adds full networking layer; local-only is sufficient for v1 target users

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Project list panel | HIGH | LOW | P1 |
| Project registration | HIGH | LOW | P1 |
| Keyboard navigation (vim-style) | HIGH | LOW | P1 |
| File-based state polling | HIGH | MEDIUM | P1 |
| Status-aware visual indicators | HIGH | LOW | P1 |
| Help overlay | HIGH | LOW | P1 |
| Detail view / drill-down | HIGH | MEDIUM | P1 |
| Multi-project status bar | HIGH | LOW | P1 |
| Search / filter | MEDIUM | LOW | P1 |
| Responsive terminal layout | HIGH | MEDIUM | P1 |
| Roadmap ASCII visualization | HIGH | HIGH | P2 |
| Phase progress indicators | MEDIUM | MEDIUM | P2 |
| Backlog item count | MEDIUM | LOW | P2 |
| Roadmap diff / last-change | MEDIUM | MEDIUM | P2 |
| Create new GSD project | MEDIUM | HIGH | P2 |
| Enqueue next action | HIGH | HIGH | P3 |
| Claude session hooking | HIGH | VERY HIGH | P3 |
| Plugin system | LOW | HIGH | P3 |

**Priority key:**
- P1: Must have for launch
- P2: Should have, add when possible
- P3: Nice to have, future consideration

---

## Competitor Feature Analysis

The closest analogues are lazydocker (multi-container overview), k9s (multi-resource monitoring), and taskwarrior-tui (task/project state). None manage GSD-style AI projects, so this tool has no direct competitors — but these tools define user expectations.

| Feature | lazydocker | k9s | taskwarrior-tui | Our Approach |
|---------|------------|-----|-----------------|--------------|
| Multi-resource list | Containers + services side-by-side | All Kubernetes resource types | Tasks per project | Projects list with phase context |
| Status indicators | Color per container state | Color per pod state | Priority color + urgency | Phase name + color per workflow state |
| Detail/log view | Right-pane log tail | Describe + live logs | Task details panel | Right-pane shows roadmap + phase detail |
| Keybindings | Configurable, lazygit-style | Vim + command mode | Vim-modal | Vim-modal; configurable via config file |
| Help overlay | Shift+? cheatsheet | ? per context | ? overlay | ? overlay; context-sensitive |
| Search/filter | / inline filter | / filter + command mode | / filter | / inline filter on project list |
| Real-time updates | Live Docker API | Live Kubernetes API | Periodic task sync | File-watch or configurable polling interval |
| Create resources | No | No | Add task | Create new GSD project (v1.x) |
| Write actions | Restart/stop/remove | Scale/delete/exec | Mark done/modify | Enqueue GSD commands (v2) |
| Config file | YAML at ~/.config | YAML + skins | .taskrc | TOML or YAML at ~/.config/gsd-manager/ |

---

## Sources

- [lazygit GitHub](https://github.com/jesseduffield/lazygit) — keybinding patterns, panel layout, vim-modal design
- [k9s official site](https://k9scli.io/) — multi-resource dashboard patterns, command mode, help overlay
- [lazydocker GitHub](https://github.com/jesseduffield/lazydocker) — multi-container overview, log view, config
- [taskwarrior-tui GitHub](https://github.com/kdheepak/taskwarrior-tui) — project/task TUI patterns
- [btop++ (itsfoss.com)](https://itsfoss.com/btop-plus-plus/) — multi-panel layout, keyboard-first design
- [ratatui.rs](https://ratatui.rs/) — constraint-based layout, async rendering
- [GSD get-shit-done user guide](https://github.com/gsd-build/get-shit-done/blob/main/docs/USER-GUIDE.md) — `.planning/` file structure, STATE.md, ROADMAP.md, phase directories
- [HN: Things I've learned building a modern TUI Framework](https://news.ycombinator.com/item?id=41215679) — rendering pitfalls, unicode width, animation anti-patterns
- [Learning from Terminals (brandur.org)](https://brandur.org/interfaces) — terminal design philosophy, responsiveness over animation
- [LazyArchon v2.0 discussion](https://github.com/coleam00/Archon/discussions/790) — AI project TUI patterns, status management, HTTP polling architecture

---

*Feature research for: TUI multi-project GSD management dashboard*
*Researched: 2026-03-24*
