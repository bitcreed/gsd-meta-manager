<!-- generated-by: gsd-doc-writer -->
# Getting Started

This guide walks a new user from a fresh checkout to a running `gsd-meta-manager`
TUI with at least one registered GSD project. For the full configuration surface
see [CONFIGURATION.md](CONFIGURATION.md); for system internals see
[ARCHITECTURE.md](ARCHITECTURE.md).

## Prerequisites

| Requirement | Version | Notes |
|-------------|---------|-------|
| Rust toolchain | `>=1.85` (stable) | Required to build the binary. Includes `cargo`. Install via [rustup](https://rustup.rs). The crate uses `edition = "2021"` and depends on `notify 8.x` (MSRV 1.85). |
| Git | any recent | Needed to clone the repository. |
| A terminal | any | crossterm targets Linux, macOS, and Windows. A truecolor terminal is recommended for the dashboard colors. |
| (Optional) `tmux` | any | Required only for the "switch to existing Claude session" feature in the TUI. |
| (Optional) An editor on `$PATH` | — | Required only for opening files from the TUI. Fallback chain is `$VISUAL` → `$EDITOR` → `vi`. |
| (Optional) A terminal emulator on `$PATH` | — | Required only for launching new Claude sessions from the TUI. Probed in order: `$TERMINAL` → `kitty` → `alacritty` → `gnome-terminal` → `xterm`. |

The produced binary is statically linked against the Rust standard library and
has no runtime dependencies beyond a working terminal.

## Installation

### 1. Clone the repository

```bash
git clone git@github.com:bitcreed/gsd-meta-manager.git
cd gsd-meta-manager
```

(Use the HTTPS URL `https://github.com/bitcreed/gsd-meta-manager.git` if you do
not have SSH configured.)

### 2. Install the binary

The most ergonomic install is `cargo install` from the working tree, which
places `gsd-meta-manager` on your `$PATH` (typically `~/.cargo/bin/`):

```bash
cargo install --path .
```

Alternatively, build a release binary without installing it:

```bash
cargo build --release
# binary at target/release/gsd-meta-manager
```

Both routes produce the same artifact. Debug builds (`cargo build`) work too
but the release profile is noticeably snappier for the render tick.

## First run

### 1. Launch the TUI

With nothing registered yet, launch the binary directly:

```bash
gsd-meta-manager
```

On first launch:

- No config file exists yet — the binary loads an in-memory default config
  rather than erroring.
- The dashboard appears with an empty project list.
- A log file is created under `dirs::data_local_dir() + "gsd-meta-manager/"`
  (typically `~/.local/share/gsd-meta-manager/gsd-meta-manager.log.<date>`).
- Active `claude` sessions whose working directory contains `.planning/` are
  auto-registered. If you happen to have a Claude session running in a GSD
  project, it will appear immediately; otherwise the list stays empty until
  you register a project manually.

### 2. Register a GSD project

You can register a project from inside the TUI or via the CLI.

**From the TUI:** press `a`, then enter the absolute path to any directory
that contains a `.planning/` subfolder.

**From the CLI** (in a separate shell, or before relaunching the TUI):

```bash
gsd-meta-manager add /path/to/your/gsd-project
```

The alias defaults to the last path component (`gsd-project` in the example
above). Pass an explicit alias as the second positional argument if you want a
shorter or unambiguous name:

```bash
gsd-meta-manager add /home/me/projects/my-app my-app
```

The first `add` invocation creates the config file at:

```text
$XDG_CONFIG_HOME/gsd-meta-manager/config.json      # Linux (typically ~/.config/...)
~/Library/Application Support/gsd-meta-manager/config.json   # macOS
%APPDATA%\gsd-meta-manager\config.json             # Windows
```

> Despite what README.md says, the on-disk format is **JSON**, not TOML. See
> [CONFIGURATION.md](CONFIGURATION.md) for the schema.

### 3. Verify your registration

```bash
gsd-meta-manager list
```

Expected output:

```text
ALIAS                PATH                                               ADDED
------------------------------------------------------------------------------------------
my-app               /home/me/projects/my-app                           2026-05-12T...
```

### 4. Open the dashboard

Relaunch the TUI:

```bash
gsd-meta-manager
```

You should see your registered project in the dashboard. Useful first
keystrokes:

- `j` / `k` — move the selection up/down
- `Enter` — drill into the project detail view (10 tabs: Phases, Roadmap,
  Backlog, Git History, Pipeline, Queue, Sessions, Archive, Config, Docs)
- `Tab` / `Shift+Tab` — cycle detail tabs
- `/` — filter projects
- `?` — show the help overlay
- `q` — quit

## Common setup issues

### "command not found: gsd-meta-manager"

`cargo install --path .` puts the binary in `~/.cargo/bin/`. If that directory
is not on your `$PATH`, either add it (e.g., `export PATH="$HOME/.cargo/bin:$PATH"`
in your shell profile) or invoke the binary by its full path
(`target/release/gsd-meta-manager` after `cargo build --release`).

### "error: package requires rustc 1.85 or newer"

Your Rust toolchain is too old. Update with `rustup update stable` and confirm
with `rustc --version`. The MSRV is driven by the `notify` 8.x dependency.

### Empty dashboard after `add`

The TUI reads the config file on launch. If you registered a project from a
different shell *while* the TUI was running, quit (`q`) and relaunch. Live
filesystem watching covers `.planning/` changes inside registered projects, not
new registrations.

The `add` command also requires the target path to exist; it is canonicalized
on registration but does not currently enforce the presence of `.planning/`.
A project without a `.planning/` directory will show empty state until that
directory is created.

### "Error: alias '...' already exists"

Provide an explicit alias as the second argument to `add`:

```bash
gsd-meta-manager add /home/me/projects/another-app my-app-2
```

Or remove the existing entry first:

```bash
gsd-meta-manager remove my-app
```

### Logs are silent

`tracing` writes to a file, not the terminal — ratatui owns the terminal. Tail
the daily log under `~/.local/share/gsd-meta-manager/` (Linux):

```bash
tail -f ~/.local/share/gsd-meta-manager/gsd-meta-manager.log.$(date +%F)
```

If `dirs::data_local_dir()` returns nothing on your platform, the binary falls
back to `/tmp/gsd-meta-manager.log.<date>`.

### Custom config location for testing

Use `--config` to point at an alternate registry without touching the default
file:

```bash
gsd-meta-manager --config /tmp/test-registry.json add /home/me/projects/scratch
gsd-meta-manager --config /tmp/test-registry.json
```

## Next steps

- **Configuration reference** — every field, env var, and CLI flag is
  documented in [CONFIGURATION.md](CONFIGURATION.md).
- **Architecture overview** — see [ARCHITECTURE.md](ARCHITECTURE.md) for the
  event loop, module layout, and how `.planning/` files are parsed.
- **Project README** — the top-level [README.md](../README.md) lists features,
  the full key-binding table, and example invocations.
