<!-- generated-by: gsd-doc-writer -->
# GSD Meta Manager

A TUI command center for managing multiple GSD-run projects from a single interface.

## What is it?

GSD (Get S\[oftware\] Done) Meta Manager gives you a unified dashboard across all
your GSD workflow projects. It reads `.planning/` state directly from disk -- no
need to launch Claude or run `/gsd:progress` in each project directory. Register
your projects once and see phase status, roadmap progress, queued work, and
pending actions at a glance.

## Features

- Unified dashboard with color-coded project status (active, paused, idle)
- Live filesystem watching -- auto-refreshes when `.planning/` files change
- Vim-style navigation (`j`/`k`, `/` search, `Enter` to drill in)
- 10-tab detail view: Phases, Roadmap (ASCII DAG), Backlog, Git History,
  Pipeline, Queue, Sessions, Archive, Config, Docs (rendered `.planning/`
  browser rooted at the active phase, with quick jumps to `.planning/` and back)
- Queue management: create, edit, delete, and reorder items
- New project creation from within the TUI
- Claude session detection (shows which projects have active Claude instances)
- Auto-registration of GSD projects from active Claude sessions -- any running
  `claude` whose working directory contains `.planning/` is added to the
  registry automatically
- Paused project detection (parses HANDOFF files)
- Milestone archive browser with inline markdown rendering
- Search and filter across projects

## Installation

Requires **Rust 1.85+**.

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

1. Build and install the binary:

   ```bash
   cargo install --path .
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
   auto-registered on launch and during the ~5s session poll.

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
| `Tab` / `S-Tab`  | Switch detail tabs      |
| `/`              | Search / filter         |
| `a`              | Add existing project    |
| `n`              | Create new project      |
| `d`              | Delete project          |
| `?`              | Show help               |
| `q`              | Quit                    |

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

- **Rust 1.85+** (for building from source)
- Produces a single static binary with no runtime dependencies

## Contributing

Contributions welcome. Please open an issue to discuss changes before submitting
a PR.

## License

MIT License -- see [LICENSE](LICENSE) for details.
