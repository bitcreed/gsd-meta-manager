<!-- generated-by: gsd-doc-writer -->
# Getting Started

This guide walks a new user from a fresh checkout to a running `gsd-meta-manager`
TUI with at least one registered GSD project. For the full configuration surface
see [CONFIGURATION.md](CONFIGURATION.md); for system internals see
[ARCHITECTURE.md](ARCHITECTURE.md).

## Prerequisites

| Requirement | Version | Notes |
|-------------|---------|-------|
| Rust toolchain | `>=1.88` (stable) | Required to build the binary. Includes `cargo`. Install via [rustup](https://rustup.rs). The crate declares `rust-version = "1.88"`, which is the highest `rust-version` across the resolved dependency graph (1.88.0 — the `ratatui` 0.30.x family, `time`, `darling`, the `icu_*` 2.3.x crates); an ordinary `cargo update` can raise it with no manifest edit, so CI's `msrv` job re-checks it before publishing. |
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
  you register a project manually. A git linked worktree (such as an agent
  worktree under `.claude/worktrees/`) is never registered itself. A session
  running inside one, or in any subdirectory of it, resolves to its main
  worktree, and that main worktree is auto-registered instead when it has
  `.planning/` and is not already registered, so many concurrent agent
  sessions of one project yield a single entry. `add` still refuses a linked
  worktree and names the main worktree to register instead, and stale worktree
  entries are pruned from the config on launch.

### 2. Register a GSD project

You can register a project from inside the TUI or via the CLI.

**From the TUI:** press `a`, type an alias (no spaces) and press `Enter`,
then enter the absolute path to a directory that contains a `.planning/`
subfolder and press `Enter`; `Esc` cancels.

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

The config file is JSON; see [CONFIGURATION.md](CONFIGURATION.md) for the schema.

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

- `j` / `k` (or `↓` / `↑`) — move the selection down / up
- `Enter` — drill into the project detail view (8 tabs: Roadmap, Phases,
  Backlog, Git, Queue, Sessions, Config, Docs)
- In the detail view, `1`-`8` or `←` / `→` switch tabs; `↓` / `Enter` enter a
  tab from the tab bar, and `↑` on the first row goes back up to it. Inside the
  Sessions and Docs tabs, `←` / `→` switch sub-tabs (Sessions / Agents,
  Files / Milestones) instead of tabs
- `Esc` — go back one level (close a pane, then the tab bar, then the
  dashboard); `q` — go straight back to the dashboard
- `Tab` — switch the terminal to the project's running Claude / Codex session
  (it does not cycle tabs)
- On the Phases tab, `→` or `Enter` focuses the **Waves pane** (see below);
  `←` / `Esc` go back to the phase list
- `/` — filter projects
- `c` — create a new project (name, path, confirm)
- The mouse works too: click a tab, a sub-tab or a row, double-click a row to
  open it (the same as `Enter`), and use the wheel to move the selection in the
  pane under the pointer. `M` turns the mouse off and on; while it is on, use
  Shift+drag to select text (terminal-dependent)
- `?` — show the help overlay
- `q` — quit (on the dashboard)

### Reading the Waves pane

The Phases tab shows the selected phase's name, its stage ladder and a
two-line stage summary. The summary's `Checks` line shows
`(N open, M deferred)` beside `✓Code Review` when gsd-core 1.15's
`NN-REVIEW-DISPOSITION.md` findings ledger is present. When the phase's
verification report is present but not passed, one more `Verify:` line names
its state and what to run next: `/gsd:execute-phase N` for a stale report
(it re-runs the verifier), `/gsd:verify-work N` for `human_needed`,
`/gsd:plan-phase N --gaps` for `gaps_found`, and, for an `unparseable`
VERIFICATION.md, fixing the YAML frontmatter in the report itself. Below the
summary is the **Waves pane**: one row per plan, grouped by
the `wave:` each PLAN.md declares. Every row shows a glyph and a state word —
`✓ done`, `▶ running`, `◐ leftover` (finished in a worktree, not merged yet),
`! stalled`, `· queued`, `○ planned` (a phase that is not the active one) —
then the plan id, its PLAN.md title and `act/est` tokens. The state comes
from the running-agents scan for the active phase, and from the phase's own
SUMMARY files otherwise; nothing is read from disk while the screen draws.

Finished waves fold into one row, and consecutive ones merge
(`w1–w10 ✓ 28/28 done`). The current wave is marked `▸`, drawn bold and
expanded along with the next one; later waves show as header rows.
`Enter` or `Space` on a header or merged row folds or unfolds it, and
`↑ +N more` / `↓ +N more` say what is scrolled out of view. Four phase shapes have their own display:
a phase not started yet (`not started`, every wave open), a completed phase
(one merged row), plans with no `wave:` metadata (a flat
`Plans (no wave metadata)` list) and no plans at all
(`No plans yet — /gsd:plan-phase N`). Below 100 columns, focusing the pane
widens it to the full width under a one-line breadcrumb.

## Common setup issues

### "command not found: gsd-meta-manager"

`cargo install --path .` puts the binary in `~/.cargo/bin/`. If that directory
is not on your `$PATH`, either add it (e.g., `export PATH="$HOME/.cargo/bin:$PATH"`
in your shell profile) or invoke the binary by its full path
(`target/release/gsd-meta-manager` after `cargo build --release`).

### "error: package requires rustc 1.88 or newer"

Your Rust toolchain is too old. Update with `rustup update stable` and confirm
with `rustc --version`. Nothing in this project picks that number by hand: the
MSRV is simply the highest `rust-version` declared across the resolved
dependency graph, currently 1.88.0 from the `ratatui` 0.30.x family, `time`,
`darling` and the `icu_*` 2.3.x crates. Because an ordinary `cargo update` can
raise it without any manifest edit, CI's `msrv` job compiles every target on the
declared floor before a release is published. To see the current value yourself:

```bash
cargo metadata --format-version 1 --locked \
  | jq -r '.packages[].rust_version | select(. != null)' | sort -V | tail -1
```

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
