# GSD Meta Manager

A TUI command center for managing multiple GSD-run projects from a single interface.

<!-- TODO: Add screenshot or terminal recording here -->

## What is it?

GSD (Get S\[oftware\] Done) Meta Manager gives you a unified dashboard across all your
[GSD workflow](https://github.com/anthropics/claude-code/tree/main/.claude/get-shit-done)
projects. It reads `.planning/` state directly from disk -- no need to launch
Claude or run `/gsd:progress` in each project directory. Register your projects
once and see phase status, roadmap progress, queued work, and pending actions at
a glance.

## Features

- Unified dashboard with color-coded project status (active, paused, idle)
- Live filesystem watching -- auto-refreshes when `.planning/` files change
- Vim-style navigation (`j`/`k`, `/` search, `Enter` to drill in)
- 10-tab detail view: Phases, Roadmap (ASCII DAG), Backlog, Git History, Pipeline, Queue, Sessions, Archive, Config, Docs (rendered `.planning/` browser rooted at the active phase, with quick jumps to `.planning/` and back)
- Queue management: create, edit, delete, and reorder items
- New project creation from within the TUI
- Claude session detection (shows which projects have active Claude instances)
- Auto-registration of GSD projects from active Claude sessions -- any running `claude` whose working directory contains `.planning/` is added to the registry automatically
- Paused project detection (parses HANDOFF files)
- Milestone archive browser with inline markdown rendering
- Search and filter across projects

## Installation

Requires **Rust 1.85+**.

### From source

```sh
cargo install --path .
```

### Build manually

```sh
cargo build --release
```

The binary is at `target/release/gsd-meta-manager`.

## Usage

Launch the manager with `gsd-meta-manager`

```sh
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
```

Register a project by pressing `a` and entering the path to a GSD project
directory (any directory containing a `.planning/` folder) or by using the
command line option `gsd-meta-manager add <path> [alias]`.

Projects are also discovered automatically: at launch and on each session
poll (~5s), any active `claude` process whose working directory is a GSD
project (contains `.planning/`) is registered without prompting. The alias
is derived from the directory's basename (with a `-2`, `-3`, ... suffix on
collision), and an `Auto-registered: <alias>` status message confirms each
addition.

### Key Bindings

| Key              | Action                  |
|------------------|-------------------------|
| `j` / `k`        | Navigate up/down        |
| `Enter`          | Open project detail     |
| `Tab` / `S-Tab`  | Switch detail tabs      |
| `/`              | Search / filter         |
| `a`              | Add existing project    |
| `n`              | Create new project      |
| `d`              | Delete project          |
| `?`              | Show help               |
| `q`              | Quit                    |

### Configuration

User configuration is stored at:

```
~/.config/gsd-meta-manager/config.toml
```

## How it Works

GSD Meta Manager reads each registered project's `.planning/` directory to
infer phase status, roadmap progress, queue state, and workflow position. A
filesystem watcher ([notify](https://crates.io/crates/notify)) triggers live
updates whenever planning files change on disk. A single async event loop
([tokio](https://crates.io/crates/tokio)) races terminal input, filesystem
events, and a render tick interval -- the UI never blocks on I/O.

The TUI is built with [ratatui](https://crates.io/crates/ratatui) and the
[crossterm](https://crates.io/crates/crossterm) backend for cross-platform
terminal support.

## Requirements

- **Rust 1.85+** (for building from source)
- Produces a single static binary with no runtime dependencies

## Contributing

Contributions welcome. Please open an issue to discuss changes before
submitting a PR.

## License

MIT License -- see [LICENSE](LICENSE) for details.
