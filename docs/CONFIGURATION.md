<!-- generated-by: gsd-doc-writer -->
# Configuration

GSD Meta Manager is configured through three mechanisms:

1. A user-level JSON config file (project registry + preferences)
2. CLI flags passed at launch
3. Environment variables consulted at runtime for editor and terminal integration

There are no `.env` files and no `dotenv`-style loading — configuration is either a JSON file managed by the binary itself or environment variables already exported in the user's shell.

## Config file location

By default the config file is resolved through the platform `dirs::config_dir()` lookup and the `gsd-meta-manager/config.json` suffix:

```
$XDG_CONFIG_HOME/gsd-meta-manager/config.json   # Linux (typically ~/.config/gsd-meta-manager/config.json)
~/Library/Application Support/gsd-meta-manager/config.json   # macOS
%APPDATA%\gsd-meta-manager\config.json   # Windows
```

The resolution logic lives in `src/config.rs` (`Config::default_path`). If `dirs::config_dir()` returns `None`, the binary falls back to a relative `.config/gsd-meta-manager/config.json` path.

The file is created on first write (e.g., the first `gsd-meta-manager add <path>` invocation). If the file does not exist, the binary loads an empty default config instead of erroring (see `load_config` in `src/config.rs`).

> Note: the project `README.md` mentions `config.toml`, but the implementation uses **JSON** (`config.json`). This document reflects the actual on-disk format.

## Config file format

The file is JSON. The top-level shape mirrors the `Config` struct in `src/config.rs`:

```json
{
  "version": 1,
  "projects": {
    "my-app": {
      "path": "/home/me/projects/my-app",
      "added": "2026-05-12T10:00:00Z"
    }
  },
  "preferences": {
    "hooks": {
      "pre_create": null,
      "post_create": null
    },
    "gsd_integration": false
  }
}
```

### Top-level keys

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `version` | integer | Yes (written by binary) | Schema version. Currently `1`. Set automatically when the config is created. |
| `projects` | object | Yes (may be empty) | Map of `alias` → `RegisteredProject`. Managed via the `add` / `remove` CLI subcommands or the `a` / `d` keys in the TUI. |
| `preferences` | object | Optional | User preferences. Defaults to an empty `Preferences` if absent (uses `#[serde(default)]`). |

### `projects.<alias>`

| Field | Type | Description |
|-------|------|-------------|
| `path` | string | Absolute path to the project root. The binary canonicalizes paths when adding via CLI. |
| `added` | string | Timestamp recorded when the project was added (free-form string field). |

### `preferences`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `hooks.pre_create` | string \| null | `null` | Shell command run before `create_project` creates a new project directory. Receives `GSD_PROJECT_NAME`, `GSD_PROJECT_PATH`, and `GSD_PROJECT_ALIAS` in its environment. Executed via `sh -c <cmd>`. |
| `hooks.post_create` | string \| null | `null` | Shell command run after `git init` succeeds in the new project directory. Same env vars as `pre_create`. |
| `gsd_integration` | boolean | `false` | When true, the detail view renders extra verified/inferred badges for GSD-integrated projects (see `src/ui/screens/detail.rs`). |

Hook commands are non-fatal in the sense that the surrounding logic only proceeds when they exit `0` — see `execute_hook` in `src/project_creator.rs`.

## CLI flags

CLI flag definitions live in `src/cli.rs`.

| Flag / Argument | Scope | Default | Description |
|-----------------|-------|---------|-------------|
| `--config <PATH>` | Global (works with all subcommands and TUI mode) | Result of `Config::default_path()` | Override the config file location. Useful for testing or sandboxing multiple registries. |
| `--help` / `-h` | Global | — | Print clap-generated help. |
| `--version` / `-V` | Global | — | Print the crate version from `Cargo.toml` (`version = "1.4.0"` at the time of writing). |

### Subcommands

| Subcommand | Arguments | Behavior |
|------------|-----------|----------|
| `add <path> [alias]` | `path` required, `alias` optional | Canonicalize the path, derive an alias from the directory basename if not provided, and write the entry into `projects`. Fails (exit 1) if the alias already exists. |
| `remove <alias>` | `alias` required | Remove a registered project by alias. |
| `list` | (none) | Print the registered projects table to stdout. |
| _(no subcommand)_ | — | Launch the TUI. |

## Environment variables

The binary itself does not require any environment variables to start. It does, however, consult several at runtime for integrations:

| Variable | Required | Consumer | Purpose |
|----------|----------|----------|---------|
| `VISUAL` | Optional | `src/main.rs` (TUI editor launch) | Preferred editor command when opening a file from the TUI. Checked first. |
| `EDITOR` | Optional | `src/main.rs` (TUI editor launch) | Fallback editor if `$VISUAL` is unset. |
| `TERMINAL` | Optional | `src/ui/screens/detail.rs` (`find_terminal`) | Preferred terminal emulator for launching new Claude sessions. Probed before falling back to `kitty`, `alacritty`, `gnome-terminal`, then `xterm`. |
| `TMUX` | Optional | `src/terminal_switch.rs` | Presence of `$TMUX` indicates the binary is running inside tmux; required for the "switch to existing Claude session" feature. Absent → tab-switching returns a clear error. |
| `HOME` | Optional | `src/state_reader/config_json.rs` (`user_defaults_path`) | Used to resolve `~/.gsd/defaults.json` for GSD user defaults overlay. If unset, the defaults file is simply not loaded. |
| `GSDMM_EXPERIMENTAL_FEATURES` | Optional | `src/experimental.rs` (read once at startup by `App::from_config`) | Turns on the experimental surfaces, which today means the **Driver** tab and everything that leads to it. **Default is OFF.** Accepted truthy values are `1`, `true`, `yes` and `on`, ASCII-case-insensitive with surrounding whitespace trimmed; unset, empty, `0`, `false`, `no`, `off` and any unrecognised value are OFF. |

Editor fallback chain: `$VISUAL` → `$EDITOR` → `vi`.

### `GSDMM_EXPERIMENTAL_FEATURES` in detail

The driver runs a real agent against a real repository, so its surface is hidden
rather than merely disabled: with the variable unset the TUI has **ten** tabs,
no `Shift+D`, no `D` in the detail-view tab hint, no driver rows or Driver
section in the help screen, no driven badge on the dashboard, and no `r`/`x`/`o`
driver keys. A user who never asked for a driver cannot discover one by pressing
a key.

With the variable set, the full surface returns and is labelled: the Driver
pane's title and the help screen's Driver section heading both read
`— EXPERIMENTAL`.

Three things are deliberately **not** affected by it:

- **`driver_opt_in` is still required.** The flag is a *discovery* gate, not an
  authorisation boundary — anyone able to set an environment variable for this
  process can also invoke the binary directly. Per-project opt-in remains the
  consent gate, and `preferences.driver_max_concurrent` still caps concurrent
  runs, in both flag states.
- **The `drive` subcommand is not gated.** The TUI launches a run by
  re-executing *this same binary* as `<current_exe> drive …`, and environment
  propagation into that child is not guaranteed; gating the subcommand would
  break the spawn path for the users who did set the flag. It *is* marked
  `hide = true`, so it no longer appears in `--help` — hidden rather than gated
  for exactly that reason: the TUI's own `<current_exe> drive` respawn is the
  caller that has to keep working. That is help visibility only. The subcommand
  parses, dispatches and runs identically in both flag states, `hide` removes
  nothing from the parser, and both flag states render the same help — this
  variable is not what hides it.
- **Startup reconciliation still runs**, so a session that toggles the flag on
  sees coherent state immediately. Only the surfaces that *display* its result
  are gated.

It is read exactly once, at startup. Changing it requires restarting the TUI.

The binary additionally reads `dirs::data_local_dir()` (typically `~/.local/share` on Linux) to choose the log directory `gsd-meta-manager/`. If `dirs::data_local_dir()` returns `None`, logs are written to `/tmp` (see `src/main.rs`). Logs themselves are configured by `tracing-subscriber`, but no environment variable currently feeds the subscriber (the `env-filter` feature is compiled in, so `RUST_LOG`-style filtering may be added later but is not currently wired into `tracing_subscriber::fmt().init()`).

<!-- VERIFY: Whether RUST_LOG is honored — the env-filter feature is enabled on tracing-subscriber but the current `tracing_subscriber::fmt()` builder in src/main.rs does not call `.with_env_filter(...)`, so RUST_LOG should have no effect today. -->

## Required vs optional settings

Nothing is strictly required for the binary to start. Specifically:

- The config file is **optional** — a missing file produces an in-memory default `Config { version: 1, projects: {}, preferences: Default::default() }` (`load_config` in `src/config.rs`).
- All `preferences` fields default through `#[serde(default)]`.
- All env vars listed above are optional — each has a fallback.

Settings that are required only for specific features:

| Setting | Required for |
|---------|--------------|
| `preferences.hooks.pre_create` / `post_create` | Running custom commands around `gsd-meta-manager`'s "create new project" flow (TUI `n` key). |
| `$TMUX` | Switching to an existing Claude session pane (tab-switching). |
| `$TERMINAL` or one of `kitty`/`alacritty`/`gnome-terminal`/`xterm` on `$PATH` | Launching a new terminal window for a Claude session from the TUI. |
| `$VISUAL` or `$EDITOR` or `vi` on `$PATH` | Opening files in an external editor from the TUI. |

## Defaults

The defaults declared in source (`src/config.rs`):

| Setting | Default | Source |
|---------|---------|--------|
| `version` | `1` | `Config::new` |
| `projects` | `{}` (empty map) | `Config::new` |
| `preferences.hooks.pre_create` | `None` | `HooksConfig::default` |
| `preferences.hooks.post_create` | `None` | `HooksConfig::default` |
| `preferences.gsd_integration` | `false` | `Preferences` derives `Default` (bool default is `false`) |
| Config path | `dirs::config_dir() + "gsd-meta-manager/config.json"` | `Config::default_path` |
| Log directory | `dirs::data_local_dir() + "gsd-meta-manager/"` (rotated daily) | `src/main.rs` |
| Log filename pattern | `gsd-meta-manager.log.<YYYY-MM-DD>` (via `tracing_appender::rolling::daily`) | `src/main.rs` |
| Session poll interval (TUI) | ~5s (auto-registration of Claude sessions) | `src/main.rs` / README |
| Render tick interval | 250 ms | `event_bus.spawn_tick(250)` in `src/main.rs` |
| Editor fallback chain | `$VISUAL` → `$EDITOR` → `vi` | `src/main.rs` |
| Terminal fallback chain | `$TERMINAL` → `kitty` → `alacritty` → `gnome-terminal` → `xterm` | `src/ui/screens/detail.rs` (`find_terminal`) |

## Per-environment overrides

There is no first-class notion of development/staging/production environments in this binary — it is a single-user TUI. The supported overrides are:

- **Per-invocation config override**: pass `--config /alternate/path/config.json` to use a different registry without modifying the default file. Useful for keeping work and personal projects separate, or for integration tests.
- **Per-platform paths**: the defaults adapt automatically because they are derived from the `dirs` crate (`config_dir` and `data_local_dir`), so the same binary picks `XDG`-style paths on Linux, `Application Support` on macOS, and `%APPDATA%` / `%LOCALAPPDATA%` on Windows without configuration.
- **Atomic writes**: `save_config` uses `tempfile::NamedTempFile` plus `persist` (rename) to update the config file atomically, so a partial write cannot corrupt the registry. Concurrent invocations are not coordinated; the last writer wins.

## Related files in the repository

- `src/config.rs` — `Config`, `RegisteredProject`, `Preferences`, `HooksConfig` types; `default_path`, `load_config`, `save_config`.
- `src/cli.rs` — `clap` definitions for `--config`, `add`, `remove`, `list`.
- `src/main.rs` — log directory setup, editor launch, TUI bootstrap.
- `src/project_creator.rs` — `pre_create` / `post_create` hook execution and `GSD_PROJECT_*` env vars.
- `src/state_reader/config_json.rs` — separate concern: parses each registered project's own `.planning/config.json` (GSD project config), not the meta-manager's config. `~/.gsd/defaults.json` is also read here.
- `Cargo.toml` — declares the `version` printed by `--version`.
