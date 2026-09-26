<!-- generated-by: gsd-doc-writer -->
# GSD Meta Manager

A TUI command center for managing multiple GSD-run projects from a single interface.

## What is it?

GSD (Get S\[oftware\] Done) Meta Manager gives you a unified dashboard across all
your GSD workflow projects. It reads `.planning/` state directly from disk -- no
need to launch Claude or run `/gsd-progress` in each project directory. Register
your projects once and see phase status, roadmap progress, queued work, and
pending actions at a glance.

![Dashboard overview across registered projects](assets/screenshots/gsd-mm-overview.png)

![Roadmap tab with dependency lanes and phase detail pane](assets/screenshots/gsd-mm-roadmap.png)

## Why GSD Meta Manager?

GSD Meta Manager is the **cross-project** command center: it observes and acts on
*every* registered project at once, from a single terminal. What you get spanning
all of them:

- **Zero-token state reading** -- parses each project's `.planning/` directory on
  disk. No Claude run, no `/gsd-progress`, no API cost required to see status.
- **Claude session awareness** -- detects which projects have a live `claude`
  instance, launches or resumes a session on any of them, and auto-registers new
  projects from active sessions (Linux only -- see
  [Platform support](#platform-support)).
- **Initial Codex support** -- interactive `codex` sessions are detected and
  shown alongside Claude sessions (reachable with `Tab` under tmux), and the
  experimental driver can run a project whose manager-config entry sets
  `"runtime": "codex"` (or `preferences.default_runtime`) through `codex exec`. Resume stays Claude-only -- see
  [Platform support](#platform-support).
- **tmux focus** -- `Tab`-to-switch straight into a project's running Claude
  session without hunting through terminal tabs.
- **Milestone archive browsing** -- read shipped-milestone artifacts with inline
  markdown rendering in a project's Docs › Milestones sub-tab, without leaving
  the dashboard.

### Not the same as GSD's claude-orchestration backend

GSD 1.8.0 added an experimental, opt-in `claude-orchestration` execution backend.
That backend parallelizes plan execution *inside a single Claude session on a
single project* -- running a phase's waves concurrently via Claude Code's Workflow
tool -- and produces the same commits and `SUMMARY.md` artifacts as normal
execution, writing no new on-disk state format. GSD Meta Manager works at the
opposite scope: it observes and acts *across many projects* from one terminal. The
two are complementary -- orchestration speeds up work *within* one project's phase;
the Meta Manager gives you the view and the controls *across* your whole portfolio.

## Features

- Unified dashboard with color-coded project status (active, paused, idle).
  The dashboard's `k/n phases` and the Roadmap header's `k of n phases done`
  count the current milestone's phases; a phase is done once its
  implementation is finished (executed on disk, or ticked in ROADMAP.md when
  it has no phase directory). Verification state is shown separately
- Live filesystem watching -- auto-refreshes when `.planning/` files change
- Vim-style navigation (`j`/`k`, `/` search, `Enter` to drill in)
- 8-tab detail view (plus the experimental Driver tab): Roadmap (phase list
  with git-log-style dependency lanes and a detail pane showing each phase's
  goal, needs, unblocks and parallel phases), Phases, Backlog, Git History,
  Queue, Sessions (with an Agents sub-view), Config, Docs (Files: rendered
  `.planning/` browser rooted at
  the active phase, with quick jumps to `.planning/` and back; Milestones: the
  shipped-milestone archive)
- Queue management: create, edit, delete, and reorder items
- New project creation from within the TUI
- Session detection -- shows which projects have a live Claude instance;
  interactive Codex sessions are detected best-effort, without resume. Linux
  only -- see [Platform support](#platform-support)
- Running agents -- for each project, which GSD agents are running right now,
  read from git worktrees and Claude Code's subagent metadata without invoking
  Claude: the dashboard Status cell summarises the live wave (e.g. `w2/11 13run`,
  or an estimated `~5/12 fixed` for code-review fix runs), and the Sessions tab's
  Agents sub-view (`→` inside the Sessions tab) lists every agent with its plan, commits, dirty files and
  last activity, wave by wave
- Auto-registration of GSD projects from active Claude sessions -- any running
  `claude` whose working directory contains `.planning/` is added to the
  registry automatically; a git linked worktree (e.g. an agent worktree under
  `.claude/worktrees/`) is never registered itself -- a session inside one, or
  in any subdirectory of it, resolves to its main worktree, and the
  main worktree is auto-registered instead when it has `.planning/` and is not
  already registered, so many concurrent agent sessions of one project yield a
  single entry; `add` still refuses a linked worktree and names the main
  worktree to add instead, and stale worktree entries are pruned from the
  config on launch
- Paused project detection (parses HANDOFF files). A handoff is ignored as
  stale when its phase is behind STATE.md's `current_phase`, or when STATE.md's
  `last_updated` is more than an hour newer than the handoff's `timestamp`
  (the file's mtime when it has none); the detail view then shows a dimmed
  "Stale HANDOFF ignored" hint instead of "Paused", and the dashboard raises
  no pause badge or needs-human flag for it
- Milestone archive browser with inline markdown rendering, in the Docs tab's
  Milestones sub-tab (`←` `→` inside the Docs tab switch Files / Milestones)
- Search and filter across projects

## Installation

Requires **Rust 1.88+**.

### From crates.io

```bash
cargo install gsd-meta-manager
```

### From source

```bash
cargo install --path .
```

### Build manually

```bash
cargo build --release
```

The binary is at `target/release/gsd-meta-manager`.

## Quick start

1. Install the binary:

   ```bash
   cargo install gsd-meta-manager
   ```

2. Launch the TUI:

   ```bash
   gsd-meta-manager
   ```

3. Register a project by pressing `a` and entering the path to any directory
   that contains a `.planning/` folder, or via the CLI:

   ```bash
   gsd-meta-manager add /path/to/your/gsd-project
   ```

4. Active `claude` sessions whose working directory contains `.planning/` are
   auto-registered on launch and during the ~5s session poll. A git linked
   worktree (such as an agent worktree under `.claude/worktrees/`) is never
   registered itself. A session running inside one, or in any subdirectory of
   it, resolves to its main worktree, and that main worktree is auto-registered
   instead when it has `.planning/` and is not already registered, so many
   concurrent agent sessions of one project yield a single entry. `add` still
   refuses a linked worktree and names the main worktree to register instead,
   and stale worktree entries are pruned from the config on launch.

## Usage

```text
gsd-meta-manager --help
TUI command center for GSD projects

Usage: gsd-meta-manager [OPTIONS] [COMMAND]

Commands:
  add     Add a GSD project to the registry
  remove  Remove a project from the registry
  list    List all registered projects
  help    Print this message or the help of the given subcommand(s)

Options:
      --config <CONFIG>  Path to config file (overrides default location)
  -h, --help             Print help
  -V, --version          Print version
```

### Key bindings

| Key              | Action                  |
|------------------|-------------------------|
| `j` / `k`        | Navigate up/down        |
| `Enter`          | Open project detail     |
| `/`              | Search / filter         |
| `a`              | Add existing project    |
| `n`              | Create new project      |
| `d`              | Delete project          |
| `?`              | Show help               |
| `q`              | Quit (dashboard); back to the dashboard (detail view) |

The detail view has two focus levels: the **tab bar** at the top and the
**content** of the active tab below it. It opens on the content of the tab you
last used; the active tab is shown bracketed, e.g. `[6:Sess]`, and while the tab
bar has focus it is also reversed and the content is dimmed.

| Key (detail view) | Action |
|-------------------|--------|
| `1`-`8`           | Jump straight into a tab's content |
| `←` `→`           | Tab bar: previous / next tab. Inside Sessions or Docs: previous / next sub-tab (Sessions / Agents, Files / Milestones). Inside any other tab: switch tab and return to the tab bar |
| `↓` / `Enter`     | Tab bar: enter the tab's content (no action is taken) |
| `↑`               | From the first row of the content: back to the tab bar |
| `[` `]`           | Sessions / Docs: previous / next sub-tab (on the Roadmap: previous / next phase in the same wave) |
| `Esc`             | Close an open pane or level first; otherwise go to the tab bar; at the tab bar, back to the dashboard |
| `q`               | Back to the dashboard from any level |
| `m`               | Sessions / Docs: switch sub-tab (alias of `←` `→`) |
| `Tab`             | Switch the terminal to this project's Claude / Codex session |

`6` and `8` re-open the sub-tab you last used on the Sessions and Docs tabs.

### Examples

Register a project with a custom alias:

```bash
gsd-meta-manager add /home/me/projects/my-app my-app
```

List all registered projects:

```bash
gsd-meta-manager list
```

Remove a project from the registry:

```bash
gsd-meta-manager remove my-app
```

### Configuration

User configuration is stored at:

```text
~/.config/gsd-meta-manager/config.json
```

Override the config location with `--config <PATH>`.

## How it works

GSD Meta Manager reads each registered project's `.planning/` directory to infer
phase status, roadmap progress, queue state, and workflow position. A filesystem
watcher ([notify](https://crates.io/crates/notify)) triggers live updates
whenever planning files change on disk. A single async event loop
([tokio](https://crates.io/crates/tokio)) races terminal input, filesystem
events, and a render tick interval -- the UI never blocks on I/O.

The TUI is built with [ratatui](https://crates.io/crates/ratatui) and the
[crossterm](https://crates.io/crates/crossterm) backend for cross-platform
terminal support.

## Requirements

- **Rust 1.88+** (for building from source)
- Produces a single static binary with no runtime dependencies

### Platform support

Live session detection, for both Claude and Codex, reads the Linux `/proc`
filesystem, so it works on Linux only. On macOS no running sessions are
detected: the Sessions tab stays empty, and auto-registration from running
sessions, tmux `Tab`-to-switch, and resuming a detected session have nothing to
act on. The rest of the TUI -- everything read from `.planning/` -- works
normally.

Codex session detection is best-effort: it relies on Codex CLI process details
that may change between releases. Interactive `codex` sessions are shown
alongside Claude sessions and can be reached with `Tab` under tmux;
non-interactive runs such as `codex exec` are not shown. Codex sessions cannot
be resumed from the TUI -- resume is Claude-only.

### Compatibility

Reads GSD 1.8.0 `.planning/` state formats. Because it observes on-disk state
rather than driving GSD, it is non-intrusive and works alongside any GSD workflow
backend, including the experimental `claude-orchestration` execution path.

## Contributing

Contributions welcome. Please open an issue to discuss changes before submitting
a PR.

## License

MIT License -- see [LICENSE](LICENSE) for details.
