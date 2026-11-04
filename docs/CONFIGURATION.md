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

> Note: the config file is **JSON** (`config.json`), not TOML.

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
    "gsd_integration": false,
    "mouse": true
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
| `driver.egress.presets` | array of strings | Optional. Named package and git host sets the sandboxed driver's CONNECT proxy may reach for this project. Each name expands as: `rust` = `index.crates.io`, `static.crates.io`, `crates.io`; `github-read` = `github.com`, `codeload.github.com`, `objects.githubusercontent.com`; `npm` = `registry.npmjs.org`; `python` = `pypi.org`, `files.pythonhosted.org`. An unknown name is refused when the launch reads the config. |
| `driver.egress.hosts` | array of strings | Optional. Extra hosts to allow, written as plain DNS names (`example.org`): no scheme, port, path, wildcard or IP literal, at least two labels. Lowercased; the first occurrence wins when a host repeats a preset's. `api.anthropic.com` is refused: the inference host is reached only through the inference proxy. |

The default for `driver.egress` is **empty, which means no CONNECT egress at all**: no preset is implied. The active list is recorded on the run as `egress_allow` in `state.json` and the `DIRECTION.md` frontmatter. A malformed entry is refused with a message naming it, never silently dropped. Example:

```json
"driver": { "egress": { "presets": ["rust", "github-read"], "hosts": ["example.org"] } }
```

### `preferences`

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `hooks.pre_create` | string \| null | `null` | Shell command run before `create_project` creates a new project directory. Receives `GSD_PROJECT_NAME`, `GSD_PROJECT_PATH`, and `GSD_PROJECT_ALIAS` in its environment. Executed via `sh -c <cmd>`. |
| `hooks.post_create` | string \| null | `null` | Shell command run after `git init` succeeds in the new project directory. Same env vars as `pre_create`. |
| `gsd_integration` | boolean | `false` | When true, the detail view renders extra verified/inferred badges for GSD-integrated projects (see `src/ui/screens/detail.rs`). |
| `mouse` | boolean | `true` | Whether the TUI starts with terminal mouse capture on (click tabs, sub-tabs and rows, double-click to open, wheel to scroll). An absent key means `true`. `M` toggles capture at runtime on the dashboard and in the detail view; the toggle is not saved back here. While capture is on, most terminals select text with Shift+drag. |

Older configs may also carry `preferences.default_runtime`, `preferences.driver_max_concurrent` and a per-project `runtime` key. They are kept and written back unchanged, so an old `config.json` loads, but they now select no agent and cap nothing: the Preferences screen (`G`) no longer offers them.

Hook commands are non-fatal in the sense that the surrounding logic only proceeds when they exit `0` — see `execute_hook` in `src/project_creator.rs`.

## CLI flags

CLI flag definitions live in `src/cli.rs`.

| Flag / Argument | Scope | Default | Description |
|-----------------|-------|---------|-------------|
| `--config <PATH>` | Global (works with all subcommands and TUI mode) | Result of `Config::default_path()` | Override the config file location. Useful for testing or sandboxing multiple registries. |
| `--help` / `-h` | Global | — | Print clap-generated help. |
| `--version` / `-V` | Global | — | Print the crate version from `Cargo.toml` (`version = "2.1.0"` at the time of writing). |

### Subcommands

| Subcommand | Arguments | Behavior |
|------------|-----------|----------|
| `add <path> [alias]` | `path` required, `alias` optional | Canonicalize the path, derive an alias from the directory basename if not provided, and write the entry into `projects`. Fails (exit 1) if the alias already exists. |
| `remove <alias>` | `alias` required | Remove a registered project by alias. |
| `list` | (none) | Print the registered projects table to stdout. |
| `task ...` | See "One-off tasks" below | Run, inspect, stop, resume and land one-off agent sessions. |
| _(no subcommand)_ `[target]` | optional alias or path of a registered project | Launch the TUI, opening the given project directly. A target cannot be combined with a subcommand; to open a project whose alias is a subcommand name (for example `list`), pass its path (`./list`). |

## Environment variables

The binary itself does not require any environment variables to start. It does, however, consult several at runtime for integrations:

| Variable | Required | Consumer | Purpose |
|----------|----------|----------|---------|
| `VISUAL` | Optional | `src/main.rs` (TUI editor launch) | Preferred editor command when opening a file from the TUI. Checked first. |
| `EDITOR` | Optional | `src/main.rs` (TUI editor launch) | Fallback editor if `$VISUAL` is unset. |
| `TERMINAL` | Optional | `src/terminal_switch.rs` (`plan_launch`) | Preferred GUI terminal emulator for launching Claude sessions. Inside tmux (`$TMUX` set and a server reachable) a new tmux window is tried first; otherwise `$TERMINAL` is used, then `xdg-terminal-exec`, `x-terminal-emulator`, and finally the first installed of `kitty`, `alacritty`, `ptyxis`, `gnome-terminal`, `xterm` (`FALLBACK_TERMINAL_CANDIDATES` in `src/ui/screens/detail.rs`). |
| `TMUX` | Optional | `src/terminal_switch.rs` | Presence of `$TMUX` indicates the binary is running inside tmux; required for the "switch to existing Claude session" feature. Absent → tab-switching returns a clear error. |
| `GSD_HOME` | Optional | `src/state_reader/config_json.rs` (`user_defaults_path`) | Root of the GSD-owned store. GSD user defaults are read from, and the Config tab's global scope writes to, `$GSD_HOME/.gsd/defaults.json`. Unset or empty falls back to `$HOME`, matching gsd-core's own `GSD_HOME \|\| homedir()` rule. Any other value is used as given. |
| `HOME` | Optional | `src/state_reader/config_json.rs` (`user_defaults_path`) | Fallback root for `$GSD_HOME/.gsd/defaults.json` (the GSD user defaults overlay) when `GSD_HOME` is unset or empty. If neither is set, the defaults file is simply not loaded. |
| `GSDMM_EXPERIMENTAL_FEATURES` | Optional | `src/experimental.rs` (read once at startup by `App::from_config`) | Turns on the experimental surfaces, which today means the **Driver** tab and everything that leads to it, one-off tasks (`t` / `T`) included. **Default is OFF.** Accepted truthy values are `1`, `true`, `yes` and `on`, ASCII-case-insensitive with surrounding whitespace trimmed; unset, empty, `0`, `false`, `no`, `off` and any unrecognised value are OFF. |
| `GSD_MM_ENVELOPE_ROOT` | Optional | `src/envelope/mod.rs` (`envelope_root`) | Overrides the root directory of the per-alias envelope ledger, which otherwise lives under `dirs::data_local_dir()/gsd-meta-manager/`. Intended for tests; it only moves where the ledger is read and written. Unset or empty uses the default. |
| `GSDMM_ONEOFF_REPORT_DIR` | Never set by users | `src/taskctl_cli.rs` (`taskctl report`) | Set by the one-off launcher inside the sandbox (and pinned by the run's `gsd-meta-manager` shim): the directory `taskctl report` writes `report.json` to. Unset, relative or missing makes `taskctl report` exit 1. |
| `GSDMM_ONEOFF_SOFT_CAP_GRACE_SECS` | Tests only | `src/oneoff/session.rs` (`grace_from_env`) | Shortens the one-off soft cap's grace (integer 1-600 seconds; default 300). Read once by the launcher daemon at session start. |
| `GSDMM_ONEOFF_IDLE_SETTLE_SECS` | Tests only | `src/oneoff/session.rs` (`idle_settle_from_env`) | Shortens how long a one-off session may stay quiet, with nothing outstanding and no agent report, before the launcher closes the agent's input (integer 1-600 seconds; default 120). Read once by the launcher daemon at session start. |
| `GSDMM_ONEOFF_STOP_GRACE_SECS` | Tests only | `src/oneoff/cancel.rs` (`stop_grace_from_env`) | Shortens the one-off hard cut's grace between SIGTERM and SIGKILL (integer 1-600 seconds; default 10). Read by the launcher daemon (and a landing) at session start. |
| `GSDMM_ONEOFF_PAUSE_GRACE_SECS` | Tests only | `src/oneoff/cancel.rs` (`pause_grace_from_env`) | Shortens how long a one-off pause may take before the launcher cuts the run hard (integer 1-600 seconds; default 300). Read at session start. |

Editor fallback chain: `$VISUAL` → `$EDITOR` → `vi`.

### `GSDMM_EXPERIMENTAL_FEATURES` in detail

The driver launcher runs a real agent against a real repository (inside a
sandbox), so its surface is hidden rather than merely disabled: with the variable
unset the TUI has **eight** tabs, no `Shift+D`, no `D` in the detail-view tab
hint, no Driver section in the help screen, and none of the launcher's dashboard
markers (the driven badge, the attention ranking and the `h` filter that read
them). A user who never asked for a driver cannot discover one by pressing a
key.

With the variable set, the full surface returns and is labelled: the Driver
pane's title and the help screen's Driver section heading both read
`— EXPERIMENTAL`.

Two things are deliberately **not** affected by it:

- **The flag is a discovery gate, not an authorisation boundary.** Anyone able
  to set an environment variable for this process can also run
  `gsd-meta-manager driver …` directly: that command line is not gated, and
  neither is the sandbox the launcher runs a run in.
- **`driver_opt_in` is not a launch gate.** The record, written from the Driver
  tab's `o` key, carries the per-project PR caps the push step reads
  (`launcher::push::policy_for`). A `branch_namespace` key in it is accepted and
  validated but not used: every run pushes to `refs/heads/gsd-auto/<alias>/<run-id>`.
  A project without a record pushes under the defaults; nothing refuses a launch
  for lacking it. There is no manager-wide concurrency cap either.

The legacy `drive` subcommand and the dashboard's `r` / `x` / `o` keys that
started and stopped it were removed, together with the startup
reconciliation of its run records. Run directories it left under
`.planning/meta-manager/runs/` are still on disk but are never read.

It is read exactly once, at startup. Changing it requires restarting the TUI.

#### One-off tasks

A one-off runs ONE headless agent session (`claude -p`) on a project: `/gsd-quick
<task>` or `/gsd-fast <task>`, in the driver launcher's sandbox, with no driver
run, `DIRECTION.md` or terminal. It reports through a strict CLI and you land its
work when you choose.

**Keys (Driver tab).** `t` opens a picker of the project's pending todos
(`.planning/todos/pending/*.md`) plus `type a task...`; `q` picks /gsd-quick, `f`
picks /gsd-fast, Esc cancels. A confirm popup then shows the model, the spend
ceiling and the wall clock; `m` switches between opus and sonnet and Enter
launches (detached, off the render thread). `t` starts another run even while others
are live. The tab shows a one-line count of the project's listed runs, for example
`one-offs: 2 running, 1 to land (T)` (parts that are zero are left out; `landing` counts
runs whose merge landing is in progress, and `N finished` stands in when nothing else
is left to say).

`T` opens the **run list**: the newest 20 runs, newest first, live and finished, one
row each with its id, state, mode, outcome (the report status, else the host's
outcome word), landing (`ff`, `merge`, `resolved`, `pushed`, `landing` while one runs,
`land failed`, or `-`) and headline. `j`/`k` or the arrow keys select (the selection
follows the run, not the row, when the list refreshes) and every action key acts on
the SELECTED run: Enter shows its summary (status, headline, changed items,
verifications, follow-ups, the SUMMARY path, spend and tracked branch, labelled
agent-reported versus host facts; Esc goes back to the list, and Esc in the list
closes it), `e` opens its SUMMARY.md from the clone in `$EDITOR`, `m` merges it, `p`
pushes it, `D` discards it (a capital D: `d` is the merge diff now), `x` stops it
(SIGTERM, then SIGKILL after 10 s), `X` kills it at once and `z` pauses it (a live run);
each of those asks first, and a merge is refused while a landing of the same run is
already in progress. The same three keys work in the run's summary and in both live
views (the run's and its resolver's); on a run that is not live but whose landing is
running they act on the landing's conflict resolver instead, and the confirmation says
so. `u` resumes a paused, stopped or killed run (it asks `b` to start from its branch
or `M` to merge the checkout's branch in first; Esc cancels), or, on a run whose landing
was paused, asks the merge question again to retry it. While the live view's message or
soft-cap input is open these keys are typed text. A run an agent resolved a merge for is marked
`resolved by agent` with its conflicted files and attempt dirs; a landing that
failed shows `land failed: <reason>` with the files and the attempt dirs kept.
`d` opens the landed merge's diff (`git diff <merge>^1 <merge>`, hardened git, no
external diff or textconv) in a scrollable full-screen view headed by the conflicted
files: `j`/`k`, PgUp/PgDn, `g`/`G`, `o` for the whole diff in `$EDITOR`, Esc back. It
is read off the render thread, bounded as "The merge diff" below says, and exists only
for a run landed by a `merge` or `resolved` landing (a fast-forward has no merge
commit; `task status` prints the range it landed). `r`
opens the live view on the run's newest conflict-resolver session (read-only: no
message or soft cap; its header says `resolver land/<n>`), and says so when the run
has none.

**CLI.** `gsd-meta-manager task run <alias|path> (--todo <file> | --text <task>)
[--mode quick|fast] [--model opus] [--max-budget-usd 10] [--cap-minutes 60]
[--unsandboxed] [--foreground]`; `task status <project> [--run-id <id>] [--json]`;
`task list <project> [--json]`; `task stop <project> [--run-id <id>] [--wait-secs
60] [--kill | --pause] [--resolver]`; `task resume <project> [--run-id <id>] [--from
branch|master] [--budget <usd>] [--detach] [--unsandboxed]`; `task send <project> <text> [--run-id <id>] [--wait-secs 10]`;
`task budget <project> (--soft <usd> | --clear) [--run-id <id>] [--wait-secs 10]`;
`task attach <project> [--run-id <id>] [--no-input]` (prints the same rendered lines as the live
view, follows until the run is finished and drained, and turns each line you type
into a message; `--no-input` only follows); `task land merge|push|discard <project>
<run-id>`.
The defaults are the ones shown above (the same as `driver launch`). `task` is not
gated by this variable: the flag is a discovery gate, as it is for `driver`.

**Stop, kill and pause.** Three ways to end a run (or its conflict resolver) early, from
`task stop` and from `x`, `X` and `z`. **Stop** (the default of `task stop`) sends
SIGTERM to the run's scope (its process group with `--unsandboxed`) and SIGKILL after 10
seconds if anything is left: the run ends `stopped`, or `killed` when it needed the
SIGKILL. **Kill** (`--kill`, `X`) sends SIGKILL at once: `killed`. **Pause** (`--pause`,
`z`) injects ONE message through the agent's stdin (recorded in `injections.jsonl` as
`pause`) asking it to commit its work in progress, call `taskctl report --status
partial` and end. The host, not the agent, decides the outcome `paused`: a DELIVERED
pause, a clean exit and a `partial` report. A `done` report wins (the run ends
`finished`), and a missing or `blocked`/`failed` report falls back to the normal
outcome. The branch is tracked exactly as for any finished run, so a paused run
is `gsd-oneoff/<run-id>` at its head. A pause the agent ignores is cut hard after 5
minutes and ends `killed` with `escalated: true`. The launcher daemon, which owns the
scope, does all of the signalling: the CLI and the TUI only write a request file
(`<run-dir>/stop.request`, host-only, never mounted, at most 4 KiB, the stronger of two
requests wins) and, for stop and kill, SIGTERM the daemon so an older daemon and a
terminal's Ctrl-C still work. `run.json` keeps `stop {kind, requested_at, ended_at,
escalated, by, refused}`; `by` is `tui`, `cli` or `signal`, and `ended_at` stays empty while the
run is still stopping (the summary says `Stopping...` or `Pausing...`). Without a live
daemon (an orphaned scope) `task stop` stops, or with `--kill` kills, the leftover scope
itself, and a pause is refused. `task stop --pause` waits up to `--wait-secs` for the
launcher's own record of the pause in `injections.jsonl` and exits 0 only when the pause
was delivered (a run still winding down is then cut by its own launcher after the
grace). It exits 1 when the launcher refused the pause, for example because the session
is already ending after the agent reported (`the pause could not be delivered to <run>
(<reason>)`, with the hint to use `task stop` without `--pause`), and 1 with `not
confirmed` wording when no answer appeared in time. A refused pause is withdrawn: no
grace cut follows it, `run.json` keeps the reason as `stop.refused`, and `task status`
and the TUI summary say `pause refused` instead of `pausing`. `z` in the TUI waits up to
10 seconds and shows a refusal as an error (`pause refused: ...`).

The same three apply to a **landing** (the resolver agent of a `resolved` merge):
`task stop --resolver`, or no flag when the run is not live but its `land.lock` is held
(the command says which it chose), and `x`/`X`/`z` on such a run. The landing's own
request file is `<run-dir>/land/stop.request`; the process doing the landing reads it
(`task land merge`, or the TUI), and the lock holder's pid is never signalled, because
the holder of a TUI landing is the TUI itself. Stop and kill end the resolver and
`X` also kills its scope by name. Pause lets the current attempt finish (up to the
pause grace) and then lands NOTHING and starts no further attempt. All three leave the
run unlanded with its branch, keep the attempt dirs, and record `land_failure` with the
reason `stopped by user`, `killed by user` or `paused` and `stop` set to `hard`, `kill`
or `pause`. `task land merge` (or `u` in the TUI) tries again; a stale request is
cleared when a landing takes its lock. The two graces are read by the daemon at start
and can be shortened for tests with `GSDMM_ONEOFF_STOP_GRACE_SECS` and
`GSDMM_ONEOFF_PAUSE_GRACE_SECS` (integers 1-600).

**Resume.** `task resume <project> --from branch|master` (TUI `u`) continues a run whose
outcome is `paused`, `stopped` or `killed` and which has a head, as a NEW run: a new id,
run dir, lock, stream and budget. The old record gets `resumed_by` and the new one
`resumed_from`; the old run stops counting as "to land" and its tracked branch and clone
stay until a run later in its resume chain lands, or until its own `discard`. When the new
run lands (`task land merge`, or the next scan for a landing made earlier), every
predecessor whose head is in the branch it landed in is recorded `already-landed`, its
`gsd-oneoff/<id>` is deleted and its clone is removed; each such predecessor is named in the
landing's output. If the new run is discarded, fails its resume merge or is only pushed,
the old run keeps its clone and branch and can be resumed again. Nothing is reopened: the new run
is a fresh sandboxed session, primed with the old task, the old report's headline and
follow-ups, the `git log` of the commits already on the branch and a note that it is a
continuation (the agent's own session is not resumed, because its transcript lives inside
the old sandbox and cannot be relied on). `--from branch` makes the new clone's work
branch start at the old head, which becomes the new run's `base`. `--from master` first
merges the checkout's current branch (it refuses a detached HEAD) into the old head in
the new run's own clone, with parents `[old head, checkout tip]`: a clean merge becomes a
host-made commit `Merge <branch> into one-off <old-id> (resume <new-id>)`; conflicts go
to ONE sandboxed resolver attempt under the new run's `land/1/`, whose result the host
re-checks exactly as for a landing. The merge commit is the new `base`, and
`resume_merge {how, merge_sha, conflicted, attempts, failure}` records it. The resolver's
spend counts against the new run's budget, the agent session gets the remainder, and
below $0.25 the run ends `failed`. A failed merge ends the new run `failed` before the
agent starts and never strands the old one: a run whose successor failed its resume merge
(or whose dir is gone) can be resumed again. The new budget is `--budget`, else what the
old run had left (refused below $0.25 with "no budget left to resume; pass --budget");
mode, model, task and wall clock are the old run's, and `--unsandboxed` is never
inherited. `task resume` stays in the terminal unless `--detach`; the TUI always detaches.
It refuses a live run, a landed run, a run that finished on its own (land it instead), a
run with commits whose branch is not tracked in the project (its work is not there), and
a second resume of the same run (checked and recorded under the old run's `land.lock`, so
two resumes cannot interleave). For a run whose landing was paused, `task resume` takes no
`--from`: it retries `task land merge`. `task status` shows `resumed:` and `resume:`
lines, and `task list` ends the row with `resumed as <id>`.

**Live view and messages.** `v` in the `T` run list opens a full-screen view of the
selected run (live or finished): it replays `stream.jsonl` from the start and then follows
it, off the render thread, keeping at most the last 10,000 rendered lines. Each
event is one or a few lines: assistant text, `* Tool: <short argument>`, a
truncated tool result (`= <first line> (N lines)`), `[sub]` for subagent events,
`prompt:` for the first message and `you:` for yours; a refused message shows as
`! refused ...`. PgUp/PgDn and `j`/`k` scroll, `f` toggles following, `G` jumps to
the end and follows, `i` sends a message to the running agent (Enter sends, Esc
cancels) and Esc or `q` closes the view and stops its tail. A finished run is
read-only. Every string from the stream is drawn escaped.

The agent reads stream-json on its stdin: the prompt is its first message and each
message you send is the next one. A turn ending is not the end of the session: the
agent may have background subagents running, and each one's notification starts a
new turn. The launcher closes the agent's stdin (which ends the session) only when
a turn has ended since the last turn start, every message you sent was read back,
no background task is outstanding, no turn is queued, and either the agent's report
(`report/report.json`) exists (closed at once) or the stream has been quiet for 120
seconds with no report (closed anyway, recorded as the `run.json` warning `closed
the agent's input after 120s idle with no report`). Messages, a pause and the soft
cap's wrap-up are delivered during background work; a message you send mid-turn is
read when that turn ends and starts another. The wall-clock limit, `--max-budget-usd`,
the soft-cap grace stop and a stop or kill still end a session at any time.
`gsd-meta-manager task send <project> <text> [--wait-secs 10]` does the same from
the CLI and exits non-zero when the run is not live. A message is 1 to 4000
characters, at most 200 per run, and is put in `<run-dir>/inbox/` (0700, host-only,
never mounted into the sandbox: only you can inject, never the agent).
`<run-dir>/injections.jsonl` (0600) records every message, delivered or refused,
BEFORE it is written to the agent.

**Spend meter.** While the run may be live the view's header shows `spend ≥ $X.XX
(lower bound) / soft ... / hard $Z (notional)`, and `spend $X.XX / ...` once the figure
is exact, that is right after a turn's `result` reported the session's cumulative cost.
The figure is that reported cost plus the token usage seen since, priced from a built-in
table of first-party list prices (dated 2026-10-06), with 1-hour cache writes at 2 x the
input price (5-minute and unsplit writes at 1.25 x). Subagent usage IS included: their
messages are forwarded in the stream and folded, and the reported cumulative cost covers
them. The live part undercounts, because the streamed output-token counts are the
message-start snapshot and the agent's thinking is not visible: each message's output is
estimated as the larger of the reported count and the visible text plus tool input
divided by four, and a message already folded into a reported total is not counted
again. It is a lower bound that each result snaps to the exact figure. It is an estimate,
not an invoice: subscription tokens are not billed per token. `--max-budget-usd` stays
the real hard ceiling, fixed at launch. The latest rate-limit event of the stream is
shown next to it (status, window, the window's utilization and the reset time in local
time).

`task status` of a live run folds the same stream: `spend:     ≥ $X so far of $Y (live
estimate, lower bound; exact at the end)`, a `soft cap:` line when one is set and a
`rate:` line when a rate-limit event was seen. `task status --json` adds
`live_spend_usd`, `live_spend_is_lower_bound: true` and `rate_limit` (`status`, `kind`,
`utilization`, `resets_at`) for a live run only. A finished run keeps its recorded,
exact `spend:` line.

**Soft cap.** Off by default. `b` in the live view (while the run is live) or `task
budget <project> --soft <usd>` sets or changes it, `--clear` (or an empty `b` input)
removes it; it must be above 0 and below `--max-budget-usd`. When the estimate
reaches it the launcher sends ONE wrap-up message ("Budget reached: commit your work
now, call `gsd-meta-manager taskctl report` with your status, then stop."), recorded
in `injections.jsonl` as `wrap_up`, and starts a 5-minute grace. If the session is
still alive when the grace ends it is stopped and recorded as `over_budget`. Raising
or clearing the cap during the grace cancels the stop (the wrap-up stays sent), and a
second crossing restarts the grace without a second message. `run.json` records
`soft_cap_usd` and `soft_cap_hit_at`. The cap is judged against the lower-bound estimate above, so it can
fire late, by roughly the unseen output and thinking of the current turn; each result
snaps the estimate to the exact cumulative figure. `--max-budget-usd` is enforced by the
agent itself and is exact: it stays the real limit.

**Where runs live.** `<data dir>/gsd-meta-manager/oneoff/<hash8 of the
project path>/<run-id>/` (`~/.local/share/...` on Linux): `run.lock` (the run's own
lock, see "Several at once"), `run.json` (the host's record), `stream.jsonl` (the agent's stream-json output, credential-redacted),
`launch.log`, `injections.jsonl` and `inbox/` (host-only, see above), `report/report.json`, `land/<n>/` (the attempts of a merge landing, see "Landing") with `land.lock`, the per-run config dir and `work/`, a
`git clone --no-hardlinks` of the project on branch `gsd-oneoff/<run-id>`. The
clone copies the whole object store, so each run costs about the repository's
size on disk until it lands (a merge landing or a push) or is discarded: a successful
landing then removes the clone, the resolver clones, the per-run config dir and every
other bulky or temporary file, and keeps a small record (under 1 MB) so `task list` and
the TUI still show the run. The record keeps `run.json` (with `pruned {at, note}` and
the spend folded in), `report/report.json`, `injections.jsonl`, a copy of the run's
SUMMARY.md as `summary.md` (the TUI's SUMMARY view opens it), the last 256 KiB of
`stream.jsonl` cut at a line boundary behind a `{"type":"gsdmm_truncated",...}` marker
line, the last 64 KiB of `launch.log`, and the small files of the resolver attempts the
landing names. A cleanup that fails is a warning line, never a failed landing.
**Uncommitted changes in the project are not included**: the clone starts at the
project's last commit, and a todo must be committed to be picked.

**Sandbox.** The driver launcher's posture is reused: bwrap under a
`systemd-run --user` scope, the egress proxy holding the token, the managed deny
list, `--setting-sources user`. Inside, only the clone, `report/` and the per-run
config dir are writable; there is no driverctl and no gate hook. A host that
cannot sandbox is refused; `--unsandboxed` (CLI only, never the TUI) runs it
degraded and records that. The wall clock is enforced from outside the sandbox,
plus systemd's `RuntimeMaxSec`; the spend ceiling is the agent's `--max-budget-usd`.
The sandbox mounts the host root read-only and masks only `/tmp` and `$HOME`, so
when `XDG_DATA_HOME` or `XDG_CONFIG_HOME` puts `gsd-meta-manager`'s data or config
directory outside both (the run dirs with their host-only inbox, the stored run
token), an empty tmpfs hides that directory inside the sandbox, and only the run's
own clone, `report/` and config dir are bound back on top. This applies to the
driver's sandbox as well. With the default layout (everything under `$HOME`) the
sandbox is unchanged.

**The report contract.** The agent ends with `gsd-meta-manager taskctl report
--status done|partial|blocked|failed --headline <one line, max 120>
[--changed <item>]... [--verified "<command>: <result>"]... [--followup <item>]...`
(each list at most 5 items of at most 200 characters, one line each, no control
or invisible formatting characters). A bad call exits 2 and writes nothing; the
last good call wins. The host re-validates the file and never trusts it; the
commit range, the commit count and the SUMMARY path are read from git on the
host. A run without a valid report gets one synthesized from its outcome.

**Landing.** `merge` lands the run's head in the branch `HEAD` pointed at when the run
was dispatched (`dispatch_branch` in `run.json`, with `dispatch_tip` and
`dispatch_detached`; a resume inherits them), not in whatever is checked out now.
`task land merge ... --into <branch>` lands into another existing local branch
instead. When that branch is the one your checkout has checked out, the checkout must
be clean (a detached HEAD, a dirty tree, a live run, no head, no commits or a
git-relevant planted file in the clone each refuse and change nothing; `discard` is the
way out of a planted file). When it is NOT checked out, your working tree, your index
and your checked-out branch are never touched and a dirty tree is no obstacle: a
fast-forward is `git update-ref refs/heads/<branch> <new> <old>` with the expected old
value, and a merge commit is made in the private land clone and then moved the same
way. It refuses, changing nothing, when the run was dispatched on a detached HEAD and
no `--into` is given, when the recorded branch was deleted since (the message says to
pass `--into`), when the branch is checked out in another linked worktree of the
project (`git worktree list` names it), and when the branch's tip changed during the
landing. A `run.json` written before the target was recorded (a legacy record) lands
into the checked-out branch exactly as before. `task list` shows the target as `→
<branch>`, `→ detached @<sha7>` or `→ checked-out branch (legacy)`, `task status` as a
`target:` line, and the TUI as a `Target:` line and in the merge question. There are
three ways it lands, recorded as `how` in `run.json` and shown by `task status`:

- `ff`: your branch is behind the head, so your own `git merge --ff-only` runs, hooks
  included. The message counts the commits your branch advanced by, which exceeds the
  agent's count for a `--from master` resume (the host's merge commit is the only commit
  the agent did not make). The agent's count spans the whole resume chain, because the runs
  the landed run continued were the agent's too, and the message names it when the two
  differ: `landed 3 commit(s) of <run> on <branch> (2 by the agent; now at <sha>)`.
- `merge`: your branch moved on. The launcher makes a fresh `--no-hardlinks` clone of
  your branch's tip in `<run-dir>/land/<n>/work`, fetches the run's branch into it and
  runs `git merge --no-ff` there with hooks off and your global git config off, using
  the project's own `user.name`/`user.email` (no identity is a refusal). A clean merge
  is committed as `Merge one-off <run-id> (<headline>)`; no agent is involved.
- `resolved`: the merge conflicts. ONE sandboxed agent session, the same sandbox profile
  and agent seam a run uses, resolves them in that clone. It is told the conflicted
  files, both sides, to keep both sides' intent (`.planning/STATE.md` tables: the union
  of rows), to never rebase, to run the project's tests (`workflow.test_command` of
  `.planning/config.json` when it is a short clean string, otherwise it finds the
  command and names it), to commit the merge and to report through the same
  `taskctl report` call with `--status done` and a `--verified` item. Its budget is the
  run's remaining budget (`max_budget_usd` minus the recorded spend), and a landing
  with less than $0.25 left refuses with "no budget left to resolve conflicts"; its
  wall clock is 15 minutes; its stream, `launch.log` and report live in
  `<run-dir>/land/<n>/`, and its spend is added to the run's `spend_usd`. A conflicted
  landing needs the sandbox: the resolver never runs unsandboxed.

The launcher never trusts the resolver's report alone. It lands the merge only when the
planted-file gate on the land clone is clean, the report is a valid `done` with a
`--verified` item, the merge commit's parents are exactly your branch's tip and the
run's head, the tree is clean and no conflict marker (`<<<<<<<` or `>>>>>>>` at the
start of a line) is left in the conflicted files. Then it fetches the merge commit into
your project and, while your checkout is still clean, on the same branch and at the
same tip, runs your own `git merge --ff-only` on it: your tree changes only at that
last step. If the branch moved in the meantime the whole merge is redone on the new tip
in a new attempt dir, at most 3 attempts in total, then it refuses with "the target kept
moving". A dirty checkout at the last step refuses and keeps the resolved merge in its
attempt dir. On any failure nothing lands, the tracked branch stays, every attempt dir
is kept, and `run.json` `land_failure` (reason, conflicted files, attempts) says why; a
later success clears it. A landing is never a rebase. One landing of a run runs at a
time (`<run-dir>/land.lock`): a second `merge`, and `discard`, refuse while it runs.
`land.lock` exists only while a landing, a slim or a scan holds it: the holder unlinks it
when it releases it, and a probe never creates it. A `merge` of a run whose head is
already in the target branch (you merged or fast-forwarded it yourself) changes nothing in
your checkout: it records a landing of kind `already-landed` (the target branch, the
commit that brought the head in, the target's tip when it was found), clears
`land_failure`, deletes the tracked branch and slims the run like any landing. A second
`merge` of that run refuses, saying where the work is. Binaries older than this change
cannot read an `already-landed` record in `run.json`.
`task land merge` blocks until it ends and Ctrl-C stops a resolver; the TUI runs it off
the render thread. Changed paths under `.claude/`, `.github/`, `.husky/` or named
`.gitattributes`, `.gitmodules`, `.envrc` are listed as a warning. `push` sends the head
only to `refs/heads/gsd-auto/<alias>/<run-id>` from the launcher's pinned mirror after
the credential scan; no pull request is opened, and a planted file refuses it too.
After a push the run's clone is removed when `gsd-oneoff/<run-id>` (or any branch of the
project) holds its head, since the commits then live in that branch and on the remote;
a push-only run with no such branch keeps its clone and the output says so. A slimmed
run still lands and pushes: with its clone gone, `merge` (the fast-forward and the
resolver path alike) and a repeated `push` take the work from `gsd-oneoff/<run-id>` in
the project once it points at the recorded head, with no planted-file gate (it ran
before that branch was made). A `push` of a run that a merge landing already slimmed is
refused with "the run's clone was removed after it landed in <branch>; its work is in
<branch>". `discard` deletes the run dir once the run is not live, and the run's tracked branch
(see below) while it still points at the run's recorded head. A landing that succeeds
deletes the tracked branch with a safe `git branch -d`, because the work is then
reachable from the branch you merged into.

Every landing records the git range it advanced your branch over, as `range` in
`run.json`: `<your HEAD before>..<head>` for `ff` (with `merge_sha` equal to the head)
and `<your branch's tip>..<merge commit>` for `merge` and `resolved`. Once the landing
has deleted `gsd-oneoff/<run-id>`, `tracked_branch` is cleared and `tracking_note` reads
`deleted after landing`; a `discard` that deletes the branch does the same before it
removes the run dir (`deleted by discard`), so a removal that fails half-way never
leaves `run.json` naming a branch that is gone. A `run.json` written before this (a
merged landing with `tracked_branch` still set, no `range`) is still read: every
display shows it as landed with no live branch. `task status` prints the landed range
and, for a merge or resolved landing, the exact command to see what the merge brought,
`git -C <project> diff <merge>^1 <merge>`.

**The merge diff.** `d` on a run with a merge landing shows its diff against the
merge's first parent without ever holding all of git's output: the TUI reads the totals
(`git diff --shortstat`), a per-file summary (`--numstat`, at most 2000 files, then
`... N more files`) and the hunks (at most 5000 lines and 512 KiB, after which git is
killed; a drawn line is cut at 1000 characters). The view is the `conflicted:` header, the summary, the hunks and, when the
hunks were cut, a last line `truncated — N more file(s), ~M more line(s)` counted from
the totals. `o` streams the complete diff straight from git into
`<run-dir>/land/merge-<sha12>.diff` (mode 0600, a symlink at that path is refused) and
opens it in `$EDITOR`.

**Several at once.** Any number of one-offs may run on a project at the same time,
each with its own wall clock and spend ceiling; `task run` and the TUI's `t` no longer
refuse (and `task run` no longer exits 3) while another run is live. Liveness comes from a lock file inside
each run dir, `<run-dir>/run.lock`, which the launcher takes before `run.json`
exists; when the run ends and the record is final, the launcher removes the file while
still holding the lock and then releases it. It is host-only and never mounted into the
sandbox. A run
dir made before this change has no `run.lock` and is judged by the old per-project
`oneoff.lock`, exactly as before. Two runs started in the same second get distinct
ids (`oneoff-YYYYMMDD-HHMMSS`, then `-2`, `-3`...). Every verb that acts on one run
takes `--run-id <id>`: `task status`, `stop`, `send`, `budget` and `attach` act on
the newest run when it is omitted, except that when more than one run may be live
they refuse (exit 1) and list the live ids instead of guessing; exactly one live run
is picked even when a newer run has finished. `task land` always names its run.
`task list <project> [--json]` prints every run, newest first: its id, live state,
mode, report status, commit count, the land target (`→ <branch>`), whether a tracked
branch exists, its last landing and `clone removed` once it was slimmed. Parallel `/gsd-quick` runs can mint the same quick id; nothing prevents
that, and a later merge of their STATE.md rows is yours to resolve.

**Tracked branches.** When a run finishes with commits and the planted-file gate
(file reads, run before any git touches the clone) finds nothing git-relevant, the
launcher fetches the clone's branch into your project as
`refs/heads/gsd-oneoff/<run-id>` (hardened git, no tags, no submodule recursion),
checks that it equals the recorded head, and records it as `tracked_branch` in
`run.json`; `git branch --no-merged` then lists unmerged one-off work. A branch of
that name that points elsewhere is never overwritten: nothing is fetched and
`tracking_note` says so. A run with no commits, a planted finding or no head gets no
branch and a `tracking_note` saying why. A tracking failure never fails the run.
`task status` prints `branch: gsd-oneoff/<run-id> @ <sha12>` while the branch is
tracked, and `branch: none (<why>)` once a landing or a discard removed it.

**Cleanup and recovery.** A scan puts a project's run directories in order: on TUI start
(experimental flag on, off the render thread, one status line when it did something),
on `task list` and on `task run` (a summary on stderr; stdout and `--json` are
unchanged). It is cheap by default (a directory listing and lock probes; git only for a
run that needs it) and does the following.

- *Dead runs.* A record that says `preparing` or `running` whose `run.lock` is free, whose
  `land.lock` is free and whose systemd scope unit (when one is recorded, and equals the
  name rebuilt from the run id and the project) is not active is finished as `crashed`
  (outcome `crashed`, a synthesized `failed` report unless the agent left a valid one, a
  warning). Its commits beyond the base are tracked as `gsd-oneoff/<run-id>` through the
  same planted-file gate and tracking as a normal finish, and only then is its clone
  removed. A planted finding or a refused tracking keeps the clone and records the reason
  in `tracking_note`. A lock that cannot be probed, a held land lock, an active or
  unprobeable scope each mean the run is not touched at all. A crashed run can be
  resumed (`task resume`, `u`).
- *Landed runs from before.* A finished run with a recorded merge or push that still has
  its clone (a leftover from before the slim existed, or a slim that was cut short) is
  slimmed. A finished run that nothing landed keeps its clone: that is the unmerged work,
  and a resumed run is not slimmed before a run in its chain lands. A free `run.lock` of
  every finished run is removed first, whatever else happens to the run. Stale `.tmp*`
  files, stop requests and `inbox/` of finished runs are removed, and the resolver clone of an unlanded run's failed attempt is removed only
  when everything in it is already in a branch of the project.
- *Runs already in their target.* A finished, not-paused run that made commits, whose
  recorded dispatch branch still exists and has the run's head among its ancestors (a
  manual merge or fast-forward), is recorded as `already-landed`, its tracked branch is
  deleted and it is slimmed, exactly as `task land merge` does. The same goes for a run
  that a successor continued, paused or not and with or without commits of its own, once a
  run later in its resume chain has landed (merged or already-landed, not pushed) in a
  branch that holds its head; every link of the chain must be confirmed from both sides
  (`resumed_by` and `resumed_from`) and a walk follows at most 32 links. The scan takes the
  land lock without waiting and skips a run whose lock is held. A plain run with a
  detached or legacy target, a target branch that is gone or no commits is left alone, and
  so are a paused run that no landed successor continues and a run whose successors have
  not landed (discarded, failed or only pushed).
- *Orphans.* A directory named like a run id with no valid `run.json`, with no lock held
  and untouched for more than an hour (the daemon creates its directory before its lock)
  has its clone's work preserved as `gsd-oneoff/<dirname>` first (same gate; a finding
  keeps the directory and warns) and is then removed. Other entries are never touched.
- *Retention.* Finished records age out after 30 days or beyond the newest 50 per
  project, and only when they are terminal and clean: no clone and no
  `gsd-oneoff/<run-id>` branch. The limits are constants; the config file has no one-off
  section.

The invariant behind all of it: no code path except an explicit `task land discard`
deletes a clone, an orphan or a resolver clone whose commits beyond the run's base are
not contained in a branch of the project (`git for-each-ref --contains`, after a clean
planted-file gate). A clone that fails the check is kept and the output says why.

The binary additionally reads `dirs::data_local_dir()` (typically `~/.local/share` on Linux) to choose the log directory `gsd-meta-manager/`. If `dirs::data_local_dir()` returns `None`, logs are written to `/tmp` (see `src/main.rs`). Logs themselves are configured by `tracing-subscriber` with `.with_env_filter(EnvFilter::from_default_env())`, so `RUST_LOG` controls the log filter (see `src/main.rs`).

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
| Terminal fallback chain | tmux window (if inside tmux) → `$TERMINAL` → `xdg-terminal-exec` → `x-terminal-emulator` → `kitty` → `alacritty` → `ptyxis` → `gnome-terminal` → `xterm` | `src/terminal_switch.rs` (`plan_launch`), `src/ui/screens/detail.rs` (`FALLBACK_TERMINAL_CANDIDATES`) |

## Per-environment overrides

There is no first-class notion of development/staging/production environments in this binary — it is a single-user TUI. The supported overrides are:

- **Per-invocation config override**: pass `--config /alternate/path/config.json` to use a different registry without modifying the default file. Useful for keeping work and personal projects separate, or for integration tests.
- **Per-platform paths**: the defaults adapt automatically because they are derived from the `dirs` crate (`config_dir` and `data_local_dir`), so the same binary picks `XDG`-style paths on Linux, `Application Support` on macOS, and `%APPDATA%` / `%LOCALAPPDATA%` on Windows without configuration.
- **Atomic writes**: `save_config` uses `tempfile::NamedTempFile` plus `persist` (rename) to update the config file atomically, so a partial write cannot corrupt the registry. Concurrent invocations are not coordinated; the last writer wins.

## Secret values in GSD project config (Config tab)

A GSD project's `.planning/config.json` (and `$GSD_HOME/.gsd/defaults.json`) can hold API keys. The Config tab (`7:Cfg`) never draws one — not in a row, not in the edit prompt, not in a status message, and not in the log file. Assume anything the TUI draws ends up in a screenshot or a screen share.

**Which keys are secret.**

- Seven keys by name, all gsd-core search providers: `brave_search`, `firecrawl`, `exa_search`, `tavily_search`, `ref_search`, `perplexity`, `jina`. Each holds the provider's API key, or `true` / `false` to override gsd-core's auto-detection of `<X>_API_KEY` / `~/.gsd/<x>_api_key`.
- Any key this build does not model (a `Not modelled` row, or a member nested inside a read-only JSON value such as `review.reviewer_instances`) whose full dotted path contains, case-insensitively, `api_key`, `apikey`, `api-key`, `token`, `secret`, `password` or `passwd`.
- Four gsd-core keys match those markers but hold budgets, not secrets, and are shown normally: `review.max_prompt_tokens`, `review.max_prompt_tokens_per_reviewer` (and its `.<slug>` entries), `statusline.show_context_tokens` and `workflow.smart_zone_tokens`.

Detection is by key name only. A secret stored under an innocent name, or embedded in another value (for example inside `workflow.test_command`), is displayed like any other value.

**What is displayed.**

| Stored value | Shown as |
|---|---|
| a key (any non-empty string, or any number, list or object) | `•••••••• (set)` |
| `true` / `false` | `true` / `false` (the auto-detection override carries no secret) |
| `null`, missing, or `""` | `(unset)` |

The mask is always exactly eight bullets. Unlike gsd-core's own `****<last-4>`, it shows no trailing characters and no length: the last four characters leak part of the key, and the length reveals the provider's key format. Nothing in the TUI needs to tell two keys apart.

**Editing a secret row.**

- `Enter` opens an EMPTY prompt — the stored key is never pre-filled. Each typed character is shown as one `•`.
- `Enter` again stores what you typed (surrounding whitespace is trimmed). Typing exactly `true` or `false` stores that override as a boolean; anything else is stored as the key.
- An empty `Enter` keeps the current value and says `<key> unchanged`. It never clears a key.
- `x` clears the value back to `(unset)`, as on any other row.

**Storage.** The key is still written in plaintext to `.planning/config.json` (or `$GSD_HOME/.gsd/defaults.json` in the `d` view) — masking is a display rule. As gsd-core itself states, that file is the security boundary, so its permissions are yours to set.

**Logs.** When a `config.json` fails to parse, the log line names only the error category and its line and column, never the offending value.

## Related files in the repository

- `src/config.rs` — `Config`, `RegisteredProject`, `Preferences`, `HooksConfig` types; `default_path`, `load_config`, `save_config`.
- `src/cli.rs` — `clap` definitions for `--config`, `add`, `remove`, `list`.
- `src/main.rs` — log directory setup, editor launch, TUI bootstrap.
- `src/project_creator.rs` — `pre_create` / `post_create` hook execution and `GSD_PROJECT_*` env vars.
- `src/state_reader/config_json.rs` — separate concern: parses each registered project's own `.planning/config.json` (GSD project config), not the meta-manager's config. `$GSD_HOME/.gsd/defaults.json` is also read here.
- `src/state_reader/config_secrets.rs` — which GSD config keys are secret (the explicit list, the name heuristic and its exemptions) and the masking the Config tab applies to them.
- `Cargo.toml` — declares the `version` printed by `--version`.
