<!-- generated-by: gsd-doc-writer -->
# GSD Meta Manager

A TUI command center for managing multiple GSD-run projects from a single interface.
GSD stands for **Git. Ship. Done.**

## What is it?

GSD Meta Manager gives you a unified dashboard across all your GSD workflow
projects. It reads `.planning/` state directly from disk -- no
need to launch Claude or run `/gsd-progress` in each project directory. Register
your projects once and see phase status, roadmap progress, queued work, and
pending actions at a glance.

**GSD compatibility:** regularly synced against gsd-core releases. At startup
the app reads which gsd-core you have installed and warns when it is newer than
what this build is synced to -- see [Compatibility](#compatibility) and
[docs/GSD-CORE-SYNC.md](docs/GSD-CORE-SYNC.md) for the exact baseline. This
build is synced to gsd-core 1.21.0 (`v1.21.0`), with oracle pin `1.21.0`.

![Dashboard overview across registered projects](assets/screenshots/gsd-mm-overview.png)

![Roadmap tab with dependency lanes and phase detail pane](assets/screenshots/gsd-mm-roadmap.png)

## Why GSD Meta Manager?

GSD Meta Manager is the **cross-project** command center: it observes and acts on
*every* registered project at once, from a single terminal. What you get spanning
all of them:

- **Zero-token state reading** -- parses each project's `.planning/` directory on
  disk. No Claude run, no `/gsd-progress`, no API cost required to see status.
- **Claude session awareness** -- detects which projects have a live `claude`
  instance, launches or resumes a session on any of them (in a new tmux window
  or herdr tab when the TUI runs inside one, else in a GUI terminal), and
  auto-registers new projects from active sessions (Linux only -- see
  [Platform support](#platform-support)).
- **Initial Codex support** -- interactive `codex` sessions are detected and
  shown alongside Claude sessions (reachable with `Tab` under tmux or herdr). Resume stays
  Claude-only -- see [Platform support](#platform-support). Next-command
  suggestions also use a Codex-only GSD install (`~/.codex/gsd-core`, or
  `$CODEX_HOME/gsd-core`; a Claude install under `$CLAUDE_CONFIG_DIR/gsd-core`
  is found too). The app does not drive Codex: the experimental driver launcher
  runs Claude only.
- **tmux and herdr focus** -- `Tab`-to-switch straight into a project's running
  Claude session, in its tmux pane or its herdr pane, without hunting through
  terminal tabs. When tmux and herdr are nested, the innermost one is used.
- **Milestone archive browsing** -- read shipped-milestone artifacts with inline
  markdown rendering in a project's Docs › Milestones sub-tab (`←` `→` inside
  the Docs tab switch Files / Milestones), without leaving the dashboard.

### Where it fits in the GSD ecosystem

GSD Meta Manager sits beside GSD rather than inside it: it reads what GSD
leaves on disk and writes only the few files listed below.

| GSD piece | Relationship |
|-----------|--------------|
| gsd-core (`.planning/`) | What the app manages: each project's `.planning/` state, read straight from disk. Regularly synced to gsd-core releases -- see [Compatibility](#compatibility) |
| `/gsd-progress`, `/gsd-manager` | One project, from inside a Claude session. This app covers every registered project from outside any session |
| `claude-orchestration` backend | Runs one project's phase waves in parallel inside a single Claude session, leaving the same commits and `SUMMARY.md` files as normal execution -- complementary, and read like any other run |
| Runtimes | Claude Code and Codex: live sessions are detected (Linux). The experimental driver launcher runs Claude in a sandbox ([docs/DRIVER-SANDBOX.md](docs/DRIVER-SANDBOX.md)); Codex is detected, never driven. Resume is Claude-only |
| gsd-tools | Your own gsd-core install, never one a project ships: `smart-entry --json` for next-command suggestions, `audit-open … acknowledge` for the owner inbox (`A`), and `config-set` / `config-set-model-profile` for Config-tab project saves |
| Capabilities | Their `capabilities.*` config keys are on the Config tab; keys this build does not model round-trip losslessly as read-only rows. Installed packs are not read |
| gsd-pi | Not supported: it keeps project state in its own `.gsd/` directory and database, not `.planning/` |

What it writes:

- `.planning/config.json`, on a Config-tab project save -- never directly: the
  app runs your gsd-tools `config-set` (GSD's own writer and lock) once per
  changed key, then re-reads the file.
- `$GSD_HOME/.gsd/defaults.json`, on a Config-tab Global save -- written
  directly, under GSD's installer lock and replaced atomically, because
  gsd-core has no command that writes it.
- `.planning/meta-manager/QUEUE.md` (Queue tab), its own config and log, and a
  new project's directory with `git init` (`c`).
- The experimental driver launcher: run directories under
  `.planning/meta-manager/drivers/`, files in its data directory -- plus whatever
  the GSD commands it launches write.
- Experimental one-off tasks: one run directory per task under its data
  directory (`gsd-meta-manager/oneoff/`), holding a private clone the agent
  commits in ([docs/CONFIGURATION.md](docs/CONFIGURATION.md#one-off-tasks)). A
  run, or its conflict resolver, can be stopped, killed or paused (`task stop`,
  or `x` / `X` / `z` in the TUI), and a paused or stopped run resumed from its
  branch or from master (`task resume`, or `u`); a run's `run.json` records
  the stop and the resume links, and its merge diff is bounded and exportable. A run
  lands into the branch `HEAD` pointed at when it was dispatched (`task land merge
  --into <branch>` overrides), and once it has landed or been pushed its clone and
  bulky files are removed, leaving a small record. A run whose launcher died (a
  crash, a reboot) is recovered as `crashed` on the next TUI start, `task list` or
  `task run`, its work kept as `gsd-oneoff/<run-id>`; orphan run directories are
  recycled and finished records age out after 30 days or beyond the newest 50 (only
  when nothing unlanded is left in them). Nothing is deleted unless its commits are
  already in a branch of the project.

What it never touches:

- Session transcripts (a subagent's is checked only for its timestamp, never
  opened), the GSD install, or installed capabilities.
- `ROADMAP.md`, `STATE.md` or phase artifacts, except through your `$EDITOR`
  (`e`). Git is read with `--no-optional-locks`. The app never commits in your
  checkout; it pushes, or fast-forwards your checkout, only when you land an
  experimental driver or one-off run. A one-off run that cannot fast-forward is
  merged in a private clone first (a sandboxed agent session resolves conflicts),
  and your checkout only fast-forwards to that merge. When the branch a one-off
  run lands into is not the one you have checked out, the app moves that branch
  with `git update-ref` (expected old value) and never touches your working tree.

## Features

- Unified dashboard with color-coded project status (active, paused, idle).
  The dashboard's `k/n phases` and the Roadmap header's `k of n phases done`
  count the current milestone's phases; a phase is done once its
  implementation is finished (executed on disk, or ticked in ROADMAP.md when
  it has no phase directory). Verification state is shown separately.
- Live filesystem watching -- auto-refreshes when `.planning/` files change
- Vim-style navigation (`j`/`k`, `/` search, `Enter` to drill in)
- Mouse support: click tabs, sub-tabs and rows on every tab, double-click to
  open, click a Roadmap band's `▸` / `▾` to fold it, and scroll lists and panes
  with the wheel; `M` turns mouse capture off and on
- 8-tab detail view (plus the experimental Driver tab): Roadmap (phase list
  with git-log-style dependency lanes and a detail pane showing each phase's
  goal, needs, unblocks and parallel phases), Phases (the selected phase's
  ladder and stage summary above a focusable Waves pane: one row per plan
  with its state, PLAN.md title and act/est tokens, finished waves folded;
  the stage summary flags a non-passed or unparseable verification report
  with the command to run, and shows the code-review ledger's open/deferred
  counts),
  Backlog, Git History,
  Queue, Sessions (with an Agents sub-view), Config, Docs (Files: rendered
  `.planning/` browser rooted at
  the active phase, with quick jumps to `.planning/` and back; Milestones: the
  shipped-milestone archive)
- Queue management: create, edit, delete, and reorder items
- New project creation from within the TUI
- Session detection -- shows which projects have a live Claude instance
  (Codex best-effort, no resume); see [Platform support](#platform-support).
  Each Sessions row shows the session's tmux or herdr location, see
  [Terminal multiplexers](#terminal-multiplexers)
- Running agents -- for each project, which GSD agents are running right now,
  read from git worktrees and Claude Code's subagent metadata without invoking
  Claude: the dashboard Status cell summarises the live wave (e.g. `w2/11 13run`,
  or an estimated `~5/12 fixed` for code-review fix runs), and the Sessions tab's
  Agents sub-view (`→` inside the Sessions tab) shows a one-line wave strip
  above every agent with its plan, commits, dirty files and last activity;
  `Enter` on an agent jumps to its plan in the Phases tab's Waves pane, and
  `Enter` on a plan there jumps back to its agent
- Worktree-only plans -- plans that live only in a linked git worktree (main's
  phase directory is empty or absent) are browsable in the Phases tab's Waves
  pane, titled `plans in worktree <name> (<branch>)`, and counted on the
  Roadmap with an `@<ref>` badge. Main's plans always win; with several
  worktrees holding the phase, the one with the newest PLAN wins (` +N more`
  names the rest). Read-only, and refreshed on the ~5 s agents poll
- Auto-registration of GSD projects from active Claude sessions, with git
  linked worktrees resolved to their main worktree so many concurrent agent
  sessions yield a single entry (see [Quick start](#quick-start))
- Paused project detection (parses HANDOFF files). A handoff is ignored as
  stale when its phase is behind STATE.md's `current_phase`, or when STATE.md's
  `last_updated` is more than an hour newer than the handoff's `timestamp`
  (the file's mtime when it has none); the detail view then shows a dimmed
  "Stale HANDOFF ignored" hint, with no pause badge or needs-human flag
- Search and filter across projects
- GSD version awareness -- each project's effective gsd-core install and its
  version are compared with the version this build is synced to; see
  [Compatibility](#compatibility)

## Installation

Requires **Rust 1.89+**.

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

3. Register a project in the TUI (press `a`, type an alias for the project and
   press `Enter`, then enter the path to any directory that contains a
   `.planning/` folder), or via the CLI:

   ```bash
   gsd-meta-manager add /path/to/your/gsd-project
   ```

4. Active `claude` sessions whose working directory is a GSD project are
   auto-registered on launch and during the ~5s session poll. A GSD project
   here means its `.planning/` holds `STATE.md`, `ROADMAP.md`, `PROJECT.md` or `config.json`,
   so an empty `.planning/`, or one holding only gsd-meta-manager's own
   `.planning/meta-manager/`, does not count. Folders under the system temp dir
   (`$TMPDIR`, else `/tmp`) are never auto-registered, so test fixtures and
   scratch checkouts stay out of the registry; `add` still registers them. A
   git linked worktree (such as an agent worktree under `.claude/worktrees/`)
   is never registered itself: a session inside one resolves to its main
   worktree, which is registered instead when it is a GSD project, so many concurrent agent
   sessions of one project yield a single entry. `add` refuses a linked
   worktree and names the main worktree to register instead, and stale
   worktree entries are pruned from the config on launch.

## Usage

```text
$ gsd-meta-manager --help
TUI command center for managing multiple GSD-run projects from a single interface.

Usage: gsd-meta-manager [OPTIONS] [TARGET] [COMMAND]

Commands:
  add     Add a GSD project to the registry
  remove  Remove a project from the registry
  list    List all registered projects
  help    Print this message or the help of the given subcommand(s)

Arguments:
  [TARGET]
          Alias or path of a registered project to open directly

          An alias that shares its name with a subcommand (e.g. `list`) must be opened by its path instead, such as `gsd-meta-manager ./list` or the absolute path.

Options:
      --config <CONFIG>
          Path to config file (overrides default location)

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

### Key bindings

| Key              | Action                  |
|------------------|-------------------------|
| `j` / `↓`        | Move the selection down |
| `k` / `↑`        | Move the selection up   |
| `Enter`          | Open project detail     |
| `b`              | Open project detail on the Backlog tab |
| `/`              | Filter projects. Type to filter; `Enter` keeps the filter, `Esc` clears it. The help overlay (`?`) lists the filter syntax; `//y [lane]` lists projects with owner work |
| `s`              | Toggle sort: alphabetical / attention first (waiting on a human, then owner work, then driven) |
| `i`              | Owner inbox: every project's owner work in one list |
| `a`              | Add an existing project: type an alias, `Enter`, then its path, `Enter`; `Esc` cancels |
| `c`              | Create a new project: enter a name, then a path (`Tab` completes it), then confirm with `y` / `Enter` |
| `d`              | Remove the selected project from the registry after a `y` / `n` confirm; project files are not deleted |
| `Tab`            | Switch the terminal to the selected project's Claude / Codex session |
| `?`              | Show / hide the help overlay (`↑` / `↓` and `PgUp` / `PgDn` scroll it; `Esc` also closes it). Any other key highlights the rows that list it; pressing it again moves to the next one, and the title shows e.g. `x: 2/4`, or `no help entry for <key>` |
| `M`              | Toggle mouse capture    |
| `q`              | Quit (dashboard); back to the dashboard (detail view) |
| `Ctrl+C`         | Force quit from any screen |

With `GSDMM_EXPERIMENTAL_FEATURES` set, `Shift+D` in the detail view opens the
experimental Driver tab; see [Configuration](docs/CONFIGURATION.md).

The detail view has two focus levels: the **tab bar** at the top and the
**content** of the active tab below it. It opens on the content of the tab you
last used; the active tab is shown bracketed, e.g. `[6:Sess]`, and while the tab
bar has focus it is also reversed and the content is dimmed.

| Key (detail view) | Action |
|-------------------|--------|
| `1`-`8`           | Jump straight into a tab's content |
| `←` `→`           | Tab bar: previous / next tab; stops at the first / last tab. Inside Sessions or Docs: previous / next sub-tab (Sessions / Agents, Files / Milestones), stopping at the ends. On the Phases tab, `→` focuses the Waves pane; with a Backlog item open, `←` closes it. Inside any other tab: switch tab and return to the tab bar |
| `↓` / `j` / `Enter` / `Space` | Tab bar: enter the tab's content (no action is taken; on the Phases tab with an Owner block: its first row, and down past its last row to the phase list) |
| `↑` / `k`         | From the first row of the content: back to the tab bar (on the Phases tab with an Owner block: into the block, and up from its first row to the tab bar) |
| `[` `]`           | Sessions / Docs: previous / next sub-tab (on the Roadmap list: previous / next phase in the same wave) |
| `Esc`             | Close an open pane or level first; otherwise go to the tab bar; at the tab bar, back to the dashboard |
| `q`               | Back to the dashboard from any level |
| `m`               | Sessions / Docs: switch sub-tab (alias of `←` `→`) |
| `Tab`             | Switch the terminal to this project's Claude / Codex session (on the Sessions tab, the highlighted session) |

`6` and `8` re-open the sub-tab you last used on the Sessions and Docs tabs.

Tab-specific keys:

| Tab | Key | Action |
|-----|-----|--------|
| Roadmap | `j` `k` `g` `G` `PgUp` `PgDn` | Move the phase cursor, jump to top / bottom, or page |
| Roadmap | `h` / `l` | Jump to a dependency / to a phase it unblocks; repeat to cycle |
| Roadmap | `[` / `]` | Previous / next phase in the same wave |
| Roadmap | `Space` | Fold / unfold the milestone |
| Roadmap | `Enter` | Open the phase in Phases. On a milestone row, fold / unfold it; on the shipped-milestones row, open Docs › Milestones |
| Phases | `→` / `Enter` / `Space` | Focus the Waves pane (on top of the right side, under the ladder) |
| Waves pane | `j` `k` `g` `G` `PgUp` `PgDn` | Move the row cursor, top / bottom, page |
| Waves pane | `Enter` / `Space` | Fold / unfold a wave (or a merged `w1–w10 ✓` row); on a plan, jump to its agent in Sessions › Agents |
| Waves pane | `e` | Edit the plan's PLAN.md in `$EDITOR` at its objective |
| Waves pane | `←` / `Esc` | Back to the phase list (`q` still leaves the detail view; a digit, `[` or `]` leaves the pane and acts as usual) |
| Phases | `↑` / `k` from the first phase | Enter the Owner block (with no phases the block holds the keyboard from the start) |
| Owner block | `j` `k` `g` `G` `PgUp` `PgDn` `Enter` `Space` `←` `→` `h` `l` `Esc` | Move; `Enter` opens the action menu on a group (on a check or file line: that file read-only at that place; on the fold line: opens or closes the fold); `Space` expands or collapses; `→` `l` expand, `←` `h` collapse (on the fold line: switch fold); back to the phase list (to the tab bar when there are no phases) |
| Backlog | `Enter` | Open / close the item's content pane |
| Backlog | `j` `k` `PgUp` `PgDn` | Scroll the open content pane |
| Backlog | `e` | With the pane open, edit the item where it lives (e.g. its ROADMAP.md section) in `$EDITOR`; otherwise enqueue it |
| Backlog | `←` / `Esc` | Close the content pane |
| Git | `Enter` | Show the selected commit's message and diff stat |
| Git | `p` | Toggle planning-only commits |
| Git | `PgUp` / `PgDn` | Scroll by a page |
| Git | `Esc` | Close the commit pane |
| Queue | `a` | Add an item |
| Queue | `e` | Edit the selected item. The item is removed while you edit, so `Esc` leaves it removed |
| Queue | `d` / `x` | Delete the selected item (asks first) |
| Queue | `Enter` / `Space` | Mark the selected item done, which removes it from the queue |
| Queue | `J` / `K` | Move the selected item down / up |
| Sessions | `Enter` | Resume the selected Claude session (Codex sessions cannot be resumed) |
| Sessions | `n` | Launch a new Claude session in this project |
| Sessions › Agents | `Enter` | Jump to the agent's plan in the Phases Waves pane (navigation only; nothing is sent to the agent) |
| Config | `Enter` | Edit the selected value; in an open dropdown, apply the choice. On a secret row (an API key, shown as `•••••••• (set)`) the prompt opens empty with hidden input, and an empty `Enter` keeps the current key — see [Secret values](docs/CONFIGURATION.md#secret-values-in-gsd-project-config-config-tab) |
| Config | `x` | Clear (unset) the selected value |
| Config | `g` (or `d`) | Switch between the project's config and `$GSD_HOME/.gsd/defaults.json` (`GSD_HOME` defaults to your home directory) |
| Config | `r` | Reload the config from disk |
| Config | `/` | Filter the rows; `Esc` clears the filter |
| Docs › Files | `Enter` | Open the selected directory or file |
| Docs › Files | `Esc` | Close the file, then go up one directory |
| Docs › Files | `g` / `p` | Jump to the `.planning/` root / back to the active phase's directory |
| Docs › Files | `e` | Open the selected file in `$EDITOR` |
| Docs › Milestones | `Enter` | Open the milestone, phase or file |
| Docs › Milestones | `Esc` | Back up one level |
| Docs › Milestones | `e` | Archived files are read-only, so this only shows a status message; a workstream's active-milestone docs (`ws …` rows) open in `$EDITOR` |
| Any other tab | `e` | Enqueue the suggested next GSD command |
| Owner inbox | `1`-`4` `a` `j` `k` `Enter` `Space` `←` `→` `h` `l` `g` `o` `v` `A` `y` `x` `Esc` | See Owner inbox; `Enter` opens the action menu |

#### Mouse

Mouse capture is on by default, on the dashboard and in the detail view. Dialogs,
the help overlay and text entry (the `/` search, Config value edits and the
Config filter) ignore the mouse.

| Mouse | Action |
|-------|--------|
| Click | Select a dashboard row; in the detail view, switch tab or sub-tab (exactly like its digit key, including Docs › Files / Milestones), or select a row on any tab — Roadmap, Phases, Backlog, Git, Queue, Sessions, Agents, Config (key or value column), Docs Files and Milestones, the Waves pane, the Driver runs — and focus that pane |
| Click `▸` / `▾` | Roadmap: fold or unfold that milestone band, or the shipped-milestones row |
| Click a Config value | On the row that is already selected, the same as `Enter`: opens the chooser or the editor (a secret's prompt opens empty) or steps an integer |
| Config chooser | Click an option to highlight it, click it again to apply; a click outside the chooser closes it without applying |
| Double-click | Open the row, the same as `Enter`: a project's detail view, a phase from the Roadmap, a Docs folder or file, a milestone level, a Backlog item's content, a Git commit, a Config chooser or editor, session resume, the Agents <-> Waves-pane jumps, wave fold / unfold. On a Roadmap band or the shipped row it folds / unfolds instead |
| Wheel | Over a list, move its selection one row per step; over the Backlog content, the Git commit or the Driver output, scroll it one line per step; over an open Docs file, scroll it. It never switches tabs or climbs to the tab bar |
| `M` | Toggle mouse capture on / off (not saved) |
| Shift+drag | Select and copy text while mouse capture is on (terminal-dependent) |

Not mouse-driven, on purpose: a Queue double-click does nothing, because
`Enter` there marks the action done and saves the queue (select with a click,
then press `Enter`); the Config `d` scope switch, breadcrumbs, the Git mode
line, footers and titles are labels, not controls; the phase names in the
Roadmap detail pane's Needs / Unblocks / Parallel lines are not links (`h` /
`l` follow them); the wheel over the Backlog list while its content pane is
open does nothing; dialogs, the help overlay and text entry ignore the mouse;
the Owner block on the Phases tab is keyboard-only (a click there selects nothing and
the wheel over it scrolls nothing, and a click does nothing while its action menu is open), and so is the inbox screen (`i`);
drag, right / middle click, hover and horizontal scroll are ignored.

To start with mouse capture off, set `"mouse": false` under `preferences` in
`config.json` (see [Configuration](docs/CONFIGURATION.md)).

### Owner inbox

The dashboard shows what is waiting on you, read from each project's `.planning/`
files: checks only you can run, rulings you have to make, and code that needs an agent.
Reading is free: the dashboard, the project detail and the inbox screen only read each
project's `.planning/` files. Four actions go further, each on your key press: `v`
launches the command in the project, `A` runs gsd-core's acknowledge writer after a
confirmation, `x` settles a check or a phase through `/gsd-verify-work N` (a prepared
instruction, launched after a confirmation), `y` copies a checklist. The app itself
writes nothing into any project.
The same data is available to scripts and agents through
[`gsd-meta-manager inbox`](#owner-work-inbox).

**Dashboard (100 columns and wider).** A `You` column sits between Status and Progress
and shows at most two lanes for the current milestone, always in the order `decide`,
`test`, `fix`:

| Cell | Meaning |
|------|---------|
| `test 10 → closes 4` | 10 checks to run; testing alone closes 4 phases (only shown when `test` is the only lane) |
| `decide 1 · fix 2 ph` | 1 ruling waits on you; 2 phases need code |
| `fix 2` | open debug investigations count as fixes but not as phases |
| `·` | nothing waits on you (blocked work is not yours to act on) |
| `(+3 old)` | 3 more test, decide or fix items in archived milestones (the detail Old debt fold also counts blocked ones, so it can be higher) |
| (blank) | no owner data: it has not loaded yet, or the folder or its `.planning/` is gone |

A summary line under the table adds the totals over the visible rows
(`You: test 43 in 20 phases (17 close) · decide 9 · fix 9 phases · 11 of 20 projects`)
and `[i] inbox`. Below 100 columns the table is unchanged. `s` (attention first) ranks
projects with owner work right after those waiting on a human, and `/y` filters by it:
`//y` lists every project with owner work, `//y test`, `//y decide` and `//y fix` (or
`//yt`, `//yd`, `//yf`) narrow it to one lane. The `❢` badge and `/h` are unchanged.

**Project detail, Phases tab.** An Owner block above the phase list names what closes
the milestone (`closes after: test 12 (ph 21 22 24) · unblock 5 (ph 20 23) · …`) and
lists one row per group: `TEST`, `DECIDE`, `FIX`, `BLOCKED`, each with its phase, checks,
age and the command that handles it. `↓` from the tab bar and `↑` from the first phase
enter the block, and `↓` past its last row reaches the phase list; with no phases the block
holds the keyboard from the start. `Enter` on a group opens its action menu (see
the footer and action menu paragraph below); `Space` (or `→` / `l`) expands
a group to its checks and source file, and `←` / `h` collapses it. `Esc` returns to the
phase list (to the tab bar when there are no phases). Old debt,
acknowledged files and pending todos are collapsed folds under the rows; `Enter` or `Space`
on the fold line opens or closes the fold, and `←` `→` (or `h` `l`) move between folds.
On a group row `v`, `o`, `A`, `y` and `x` work as in the inbox screen below, and `j` walks
into an expanded group's checks (the first five) and its file line, so `x` can settle one
check. `Enter` on a check or on the file line opens that file read-only at that check (the
file line: the group's first check, else the phase heading, else the top); the opened line
is marked in the viewer. `Space` collapses the group from any of those lines.

**Inbox screen (`i` on the dashboard).** One list of everything that waits on you, across
every registered project, grouped into `TEST — closes a phase`, `TEST — phase stays open`,
`DECIDE` and `FIX — code needed` (`BLOCKED` with `a` or `4`), with Old debt, Acknowledged,
Triage and Stale (phases older than `preferences.owner.fold_days`, default 90) folded below.
A UAT and VERIFICATION pair of one phase is one row. Inside Test, phases that close first, then projects closest to a
milestone close, then the oldest. Decide and Fix list the oldest first. The numbers in the
header cover every project and match `gsd-meta-manager inbox --json`. The screen is
keyboard-only.

| Key | Action |
|-----|--------|
| `1` `2` `3` `4` / `a` | Show one lane / all lanes including Blocked; press again for the default |
| `j` `k` `PgUp` `PgDn` `Home` `End` | Move |
| `Enter` | Open the group's action menu; on a check or file line, open that file read-only at that place; on the fold line, open or close a fold |
| `Space` | Expand or collapse a group to its checks and source file, or a fold |
| `←` `→` / `h` `l` | Collapse / expand the selected group; on the fold line, move between folds |
| `g` | Open the project's detail view on the same group |
| `o` | Open the group's source file in a read-only viewer |
| `v` | Run the group's command in its project through your multiplexer (tmux, herdr, ...), or switch to the project's live session and show the command to type |
| `A` | Acknowledge = hide the group's file until it changes; nothing is recorded and the phase stays open (see below) |
| `x` | Settle = record the selected check or phase as skipped, so the phase can close: won't do (with a reason), deferred permanently, or move to phase M; asks first, then launches `/gsd-verify-work N` |
| `y` | Copy the checklist `gsd-meta-manager inbox ALIAS` prints for that row (on the Old debt fold: that fold's checklist) |
| `Esc` / `q` | Back to the dashboard |

**`A` or `x`?** `A` hides a file for now: nothing is recorded, its phase stays open, and its items come back when the file changes. `x` records checks you cannot or will not test as skipped (won't do, deferred permanently or moved to phase M), so the phase can close.

**The footer and the action menu.** In both the inbox and the Owner block the footer lists
the keys that work on the selected row, with the command `v` runs when there is room
(`[v]run /gsd-verify-work 16`). Narrow terminals drop the lower-priority hints (the movement
and `Space` hints first, then the command text) but always keep `[?]help`. `Enter` on a group
or debug row opens an action menu: one line per action for that row with its key, a label and
the command, target or effect (`A`: hide this file until it
changes; `x`: record as skipped). A key letter (`v` `x` `o` `A` `y`, and `g` in the inbox) or `Space`
runs that action at once; `j` / `k` and `Enter` pick one; `Esc` or `q` closes the menu. An
action that exists for the row but cannot run now stays in the list, dimmed, with the reason
in text (for example `Settle already pending for this row`). A menu entry runs exactly what
the key does, so `x` still opens the settle chooser and its confirmation and `A` still asks
first. The menu is keyboard-only: other keys and mouse clicks do nothing while it is open.

`v` builds the command from the group's kind, phase and workstream
(`/gsd-verify-work N`, `/gsd-plan-phase N --gaps`, `/gsd-debug`, `/gsd-resume-work`,
`--ws NAME` for workstream groups) and refuses a phase token or workstream name that is not
plain. `A` asks first, then runs

```
node <gsd-tools.cjs> audit-open acknowledge --category <uat_gaps|verification_gaps|context_questions|debug_sessions> --milestone <version> <--phase DIR --file NAME | --slug STEM> [--archived-milestone V] [--ws NAME]
```

in the project directory. `<gsd-tools.cjs>` comes from your own install only (`${CLAUDE_CONFIG_DIR:-~/.claude}/gsd-core`, then `${CODEX_HOME:-~/.codex}/gsd-core`, then an absolute `PATH` entry named `gsd-tools`); a `gsd-tools.cjs` the project itself ships is never run by `A`. gsd-core writes `audit_acknowledged` into that one file's
frontmatter and does not commit; the marker is file-level (it hides every item of the file
in every lane) and lapses by itself when the file changes. This app never writes the file.
`x` settles what you could not or will not test. On a TEST or BLOCKED row it settles the
whole phase; after `Space` and `j` onto one check it settles that check. A two-row chooser
says the target is recorded as skipped so the phase can close, and offers three outcomes: `w` won't do, `p` deferred permanently, `m` move to another phase
(you pick phase M; an optional reason follows). A default-No popup then sums the settle
up: project, target, outcome, your reason, the command it runs in bold with how many
session variables `env -u` clears, and the start of the instruction with its key facts in
bold. `d` switches to the full command, every argv element verbatim, and back. Only `y`
launches `/gsd-verify-work N <instruction>` in the project (`--ws NAME` for a workstream),
in a new session through your multiplexer; a view the terminal cuts off cannot be
confirmed, so enlarge it or press `d` for the one that fits. The instruction asks GSD
to resume the UAT session, record each check as `result: skipped` with a
`Deferred follow-up:` reason (which gsd-core counts as non-blocking), and not to promote
it to the 999.x backlog. Move also asks for one todo naming phase M, so the check comes up again
when phase M runs. If the project already has a live Claude session, `y` switches to it
and copies the command (instruction included) for you to paste, by the same clipboard
route as `y` on a row; nothing is typed into that session and no second one starts.
After `y` the row stays where it was and shows a dim `settling… <outcome>` badge with a
spinner and the time since `y`; it reads `recorded` once the source file changes on disk,
and when the check leaves the list it shows `✓ test 2 skipped` (or `✓ moved to phase M`)
for about two seconds before the row goes. The badge shows in the inbox and in the Owner
block, and the state lives in memory only: it survives refreshes, is gone after a restart,
and with no change on disk the badge just keeps counting. The
app writes nothing: every change comes from the command GSD runs.

`y` uses `wl-copy`, `xclip`/`xsel`, `pbcopy` or `clip.exe` when present, otherwise an OSC 52
escape sequence your terminal must allow (inside tmux: `set -g set-clipboard on`).

### Examples

Open a project directly, by alias, from inside it (any subdirectory works too),
or by path:

```bash
gsd-meta-manager my-app
gsd-meta-manager .
gsd-meta-manager ~/projects/my-app
```

The TUI starts on that project's detail view; Esc returns to the overview with
that project selected. An unregistered folder that is a GSD project (its
`.planning/` holds `STATE.md`, `ROADMAP.md`, `PROJECT.md` or `config.json`) is
registered under its folder name and opened, wherever it lives, temp dir
included. Any other target that is neither a registered alias nor inside a
registered project exits with a hint to run `gsd-meta-manager add <path>`. An
alias that collides with a subcommand name (e.g. a project registered as `list`)
must be opened by its path, such as `gsd-meta-manager ./list`.

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

### Scripting and agents

`gsd-meta-manager status --json [project...]` prints the state of every registered
project, or only the named aliases or paths, as ONE JSON document on one line.
It is read-only: it takes no lock, writes no log file or journal and does not
touch the registry, so it is safe to run while GSD sessions are active. `--json`
is required. A named project is a registered alias, a path inside a registered
project, or a directory containing `.planning/`.

```bash
gsd-meta-manager status --json                 # every registered project, as `list` orders them
gsd-meta-manager status --json my-app ~/p/api  # just these two, in this order
```

Exit codes: `0` the document was printed, including entries that carry an
`error` (an unknown alias, a registered folder that is gone, a folder without
`.planning/`); `1` the registry could not be loaded or parsed (nothing on stdout,
one `status: ...` line on stderr); `2` a usage error, such as a missing `--json`.

Schema version 1:

```json
{
  "schema": "gsd-meta-manager/status",
  "schema_version": 1,
  "sessions_supported": true,
  "projects": [
    {
      "target": null,
      "alias": "my-app",
      "path": "/home/me/projects/my-app",
      "error": null,
      "milestone": {"version": "v1.2", "name": "Search", "status": "in_progress"},
      "phase": {"id": "4", "name": "Index builder", "status": "executed", "verification": "human_needed", "uat": "missing"},
      "progress": {"phases_done": 3, "phases_total": 6},
      "attention": [{"kind": "verification_human_needed", "phase": "4", "workstream": null, "status": null}],
      "sessions": [{"agent": "claude", "pid": 4242, "session_id": "0f3c9a", "cwd": "/home/me/projects/my-app", "tty": "pts/3"}],
      "next": {"advisory": true, "source": "phase-20-router", "project": "/home/me/projects/my-app", "target_phase": "4", "decision": "park", "command": null, "reason": "gate_verification_human_needed", "detail": "human_needed"},
      "warnings": [],
      "owner": {"test": 2, "decide": 0, "fix_phases": 0, "fix_debug": 0, "blocked": 0, "closeable_phases": 1, "closes_after": {"test": 2, "decide": 0, "fix": 0, "blocked": 0, "phases_open": ["4", "5"], "not_built": ["6"]}}
    },
    {
      "target": "gone-app",
      "alias": null,
      "path": null,
      "error": {"kind": "unresolved", "message": "gone-app is neither a directory containing .planning/ nor a registered project"},
      "milestone": null,
      "phase": null,
      "progress": null,
      "attention": [],
      "sessions": [],
      "next": null,
      "warnings": [],
      "owner": null
    }
  ]
}
```

| Path | Type | Meaning |
|---|---|---|
| `schema` | string | always `gsd-meta-manager/status` |
| `schema_version` | integer | `1`. Bumped on any rename, removal or type change; new keys may be added without a bump |
| `sessions_supported` | bool | `true` on Linux. When false, `sessions` is always empty |
| `projects` | array | registry order (as `list`) with no names, else the order named |
| `projects[].target` | string or null | the name you passed (escaped); null when listing the registry |
| `projects[].alias` | string or null | registry alias (escaped); null for an unregistered path or an unresolved name |
| `projects[].path` | string or null | canonical project root, equal to `next.project`; null when unresolved |
| `projects[].error` | object or null | null on success |
| `projects[].error.kind` | string | `unresolved` / `folder_missing` / `no_planning` |
| `projects[].error.message` | string | one sentence, with names escaped |
| `projects[].milestone` | object or null | null only on error |
| `projects[].milestone.version` | string or null | STATE.md `milestone` |
| `projects[].milestone.name` | string or null | STATE.md `milestone_name` |
| `projects[].milestone.status` | string or null | STATE.md `status` |
| `projects[].phase` | object or null | the active phase; null on error or when no source names one |
| `projects[].phase.id` | string | the active phase number |
| `projects[].phase.name` | string or null | roadmap phase name, else STATE.md `current_phase_name` |
| `projects[].phase.status` | string or null | disk status of the phase (`discussed`, `planned`, `executed`, `complete`, ...) |
| `projects[].phase.verification` | string or null | VERIFICATION.md status (`passed`, `gaps_found`, `human_needed`, `missing`, ...) |
| `projects[].phase.uat` | string or null | UAT.md status (`pending`, `partial`, `missing`, ...) |
| `projects[].progress` | object or null | null only on error |
| `projects[].progress.phases_done` | integer | phases done in the current milestone |
| `projects[].progress.phases_total` | integer | phases in the current milestone |
| `projects[].attention` | array | pending human/UAT items, root first, then each workstream; empty on error |
| `projects[].attention[].kind` | string | `paused`, `external_job_waiting`, `continue_here`, `verification_human_needed`, `uat_outstanding`, `continue_here_blocking`, `deferred_verification` |
| `projects[].attention[].phase` | string or null | phase id for per-phase kinds |
| `projects[].attention[].workstream` | string or null | workstream name, null for the project root |
| `projects[].attention[].status` | string or null | the UAT status for `uat_outstanding`, else null |
| `projects[].sessions` | array | live agent processes in this project; empty on error |
| `projects[].sessions[].agent` | string | `claude` / `codex` |
| `projects[].sessions[].pid` | integer | process id |
| `projects[].sessions[].session_id` | string or null | escaped |
| `projects[].sessions[].cwd` | string | the process's working directory |
| `projects[].sessions[].tty` | string or null | e.g. `pts/3` |
| `projects[].next` | object or null | exactly what `gsd-meta-manager next <path> --json` prints; null only on error |
| `projects[].next.advisory` | bool | always true |
| `projects[].next.source` | string | `phase-20-router` |
| `projects[].next.project` | string | the project root |
| `projects[].next.target_phase` | string or null | the phase routed |
| `projects[].next.decision` | string | `run` / `park` / `goal_met` / `no_rule` / `none` |
| `projects[].next.command` | string or null | the advisory `/gsd-*` command when `decision` is `run` |
| `projects[].next.reason` | string | router token |
| `projects[].next.detail` | string or null | router token |
| `projects[].warnings` | array of strings | `state_md_unreadable`, `state_md_recovered` |
| `projects[].owner` | object or null | how much owner work the project holds: the checks a human has to run, over the current milestone of the project and every workstream, after the owner rules (duplicates folded, archived and acknowledged work left out); null on error |
| `projects[].owner.test` | integer | checks waiting to be run |
| `projects[].owner.decide` | integer | open decisions: debug sessions awaiting your confirmation, handoffs, checkpoints, open CONTEXT questions and unreadable CONTEXT files |
| `projects[].owner.fix_phases` | integer | phases that need a fix plan (failed checks or unreadable files) |
| `projects[].owner.fix_debug` | integer | open debug investigations (`.planning/debug/`, root-scoped, not counted as phases) |
| `projects[].owner.blocked` | integer | checks waiting on a device, server, build or third party |
| `projects[].owner.closeable_phases` | integer | phases whose only open work is checks to run, so running them can close the phase |
| `projects[].owner.closes_after` | object | the milestone gate: which open phases of the current milestone close after which owner work (root project only; debug sessions, handoffs, todos, archived, deferred and acknowledged work never count) |
| `projects[].owner.closes_after.test` | integer | checks to run in the open phases |
| `projects[].owner.closes_after.decide` | integer | open decisions (checkpoints, open CONTEXT questions, unreadable CONTEXT files) in the open phases |
| `projects[].owner.closes_after.fix` | integer | failed checks and unreadable files in the open phases |
| `projects[].owner.closes_after.blocked` | integer | blocked checks in the open phases |
| `projects[].owner.closes_after.phases_open` | array of strings | open current-milestone phases that have plans, in phase order, spelled as the ROADMAP spells them |
| `projects[].owner.closes_after.not_built` | array of strings | open current-milestone phases without any plan on disk; they add nothing to the totals |

A session belongs to the deepest project root that contains its working
directory, so `cd src && claude` still counts for the project. Text a project
authored (names, ids) is escaped for terminals and cut at 200 characters;
artifact prose (goals, descriptions, handoff notes) never appears.

Stability: a rename, removal or type change of any path above bumps
`schema_version`; new keys may appear without a bump, so consumers should ignore
keys they do not know.

Recipes:

```bash
# Projects that need a human right now.
gsd-meta-manager status --json | jq -r '.projects[] | select(.attention | length > 0) | "\(.alias): \([.attention[].kind] | join(", "))"'

# The advisory next command per project.
gsd-meta-manager status --json | jq -r '.projects[] | select(.next.decision == "run") | "\(.alias): \(.next.command)"'

# Projects with a live Claude or Codex session.
gsd-meta-manager status --json | jq -r '.projects[] | select(.sessions | length > 0) | .alias'
```

#### Owner work: `inbox`

`gsd-meta-manager inbox` answers one question across every registered project in
one call: which checks does the owner (a human) still have to run? It reads the
projects' `.planning/` files only (UAT and VERIFICATION files, `deferred-items.md`,
pending todos); it never runs gsd-core and, like `status`, takes no lock and writes
nothing. Duplicate checks are folded, a UAT's gaps are merged into the failed check
they belong to, and archived milestones, acknowledged files and deferred items are
counted but left out of the lists unless you ask for them.

```bash
gsd-meta-manager inbox                      # a short digest: totals, one `closes after` line per project, the 5 next actions, folds
gsd-meta-manager inbox my-app               # that project's full checklist: merged entries, labels, a `STALE (> 90 d)` section
gsd-meta-manager inbox --lane test,fix      # one line per phase across projects, stale phases folded
gsd-meta-manager inbox --all                # the full plain checklist of every project, as before the digest
gsd-meta-manager inbox --json               # the same data as ONE JSON line (schema below)
gsd-meta-manager inbox --json my-app        # only this project (alias, path or directory)
gsd-meta-manager inbox --include archived,todos  # also list: archived, deferred, acknowledged, todos
```

Plain `inbox` prints the digest: the totals, one `closes after` line per project, the five
next actions ranked by payoff (phases that close first, then the fewest checks) and the
fold lines, within 22 lines (one 80x24 screen). `inbox ALIAS` prints that project's whole checklist.
`--lane` over several projects prints one collapsed line per phase (a 51-step UAT is one
line with `51 checks`). `--all` is the stable plain format: the full checklist exactly as
it was before the digest, so scripts use it (or `--json`) rather than the digest, whose
layout may change.

A UAT and a VERIFICATION file of one phase are one entry (`2 files`): nothing is dropped, the
checklist names each file. Work whose file is older than `preferences.owner.fold_days`
(default 90, clamped 1-3650) folds into one `Stale: N checks in M phases older than 90 d`
line; it is still counted in every total, and `inbox ALIAS` lists it under `STALE`.
Checks whose text reads like a decision or a fix that already landed carry the hints
`decision?` and `looks resolved`; they are hints only and never move, hide or re-rank a row.

Exit codes are those of `status`: `0` the output was printed, including entries that
carry an `error`; `1` the registry could not be loaded or parsed (nothing on stdout,
one `inbox: ...` line on stderr); `2` a usage error such as an unknown `--lane`.
`--lane` and `--include` only change which groups and todos are listed: `summary`
and `hidden` always report the full counts.

Schema version 1:

```json
{
  "schema": "gsd-meta-manager/inbox",
  "schema_version": 1,
  "summary": {
    "projects": 2, "test": 3, "decide": 0, "fix_phases": 1, "fix_debug": 0, "blocked": 1, "closeable_phases": 1,
    "archived": {"test": 0, "blocked": 0, "fix": 0, "decide": 0},
    "deferred": 0, "acknowledged_files": 0, "planned_questions": 0, "superseded": 0, "noise": 0,
    "next_actions": [{"alias": "my-app", "workstream": null, "phase": "04", "lane": "test", "checks": 1, "closes_phase": true, "command": "/gsd-verify-work 04", "age_days": 3}],
    "stale_folded": {"entries": 0, "items": 0}
  },
  "projects": [
    {
      "target": null,
      "alias": "my-app",
      "path": "/home/me/projects/my-app",
      "error": null,
      "milestone": {"version": "v1.2", "closes_after": {"test": 2, "decide": 0, "fix": 0, "blocked": 0, "phases_open": ["4", "5"], "not_built": ["6"]}},
      "groups": [
        {
          "lane": "test",
          "kind": "uat",
          "scope": "current",
          "archived_milestone": null,
          "workstream": null,
          "phase": "04",
          "phase_name": "Index builder",
          "file": ".planning/phases/04-index-builder/04-UAT.md",
          "file_status": "testing",
          "closes_phase": true,
          "bookkeeping_only": false,
          "acknowledged": false,
          "age_days": 3,
          "stale": false,
          "folded_stale": false,
          "entry_files": ["04-UAT.md", "04-VERIFICATION.md"],
          "command": "/gsd-verify-work 04",
          "supersedes": [],
          "counts": {"checks": 1, "failed": 0, "gaps": 0, "unreadable": 0},
          "items": [
            {
              "id": "04-UAT.md#2",
              "test": "2",
              "name": "Search box",
              "expected": "Typing narrows the list",
              "why_human": null,
              "result": "issue",
              "category": "issue",
              "reason": "Nothing happens",
              "blocked_by": null,
              "gaps": [{"name": "Narrowing works", "result": "failed", "reason": "No filter applied"}],
              "labels": ["decision?"]
            }
          ]
        },
        {
          "lane": "test", "kind": "verification", "scope": "current", "archived_milestone": null, "workstream": null,
          "phase": "04", "phase_name": "Index builder", "file": ".planning/phases/04-index-builder/04-VERIFICATION.md",
          "file_status": "human_needed", "closes_phase": true, "bookkeeping_only": false, "acknowledged": false,
          "age_days": 3, "stale": false, "folded_stale": false, "entry_files": ["04-UAT.md", "04-VERIFICATION.md"],
          "command": "/gsd-verify-work 04", "supersedes": [],
          "counts": {"checks": 0, "failed": 0, "gaps": 0, "unreadable": 0}, "items": []
        }
      ],
      "hidden": {"deferred": 0, "acknowledged_files": 0, "planned_questions": 0, "todos": 1, "noise": 0, "todos_oldest_days": 0, "todos_unreadable": false},
      "todos": [{"file": "2026-01-01-first.md", "title": "First todo"}]
    },
    {
      "target": "gone-app",
      "alias": null,
      "path": null,
      "error": {"kind": "unresolved", "message": "gone-app is neither a directory containing .planning/ nor a registered project"},
      "milestone": null,
      "groups": [],
      "hidden": {"deferred": 0, "acknowledged_files": 0, "planned_questions": 0, "todos": 0, "noise": 0, "todos_oldest_days": null, "todos_unreadable": false},
      "todos": []
    }
  ]
}
```

| Path | Type | Meaning |
|---|---|---|
| `schema` | string | always `gsd-meta-manager/inbox` |
| `schema_version` | integer | `1`. Bumped on any rename, removal or type change; new keys may be added without a bump |
| `summary` | object | totals over every readable project, never narrowed by `--lane` or `--include` |
| `summary.projects` | integer | entries requested, error entries included |
| `summary.test` | integer | checks waiting to be run |
| `summary.decide` | integer | open decisions: debug sessions awaiting your confirmation, handoffs, checkpoints, open CONTEXT questions and unreadable CONTEXT files |
| `summary.fix_phases` | integer | phases that need a fix plan |
| `summary.fix_debug` | integer | open debug investigations (root-scoped, not counted as phases) |
| `summary.blocked` | integer | checks waiting on a device, server, build or third party |
| `summary.closeable_phases` | integer | phases whose only open work is checks to run |
| `summary.archived` | object | open checks of archived milestones, by lane |
| `summary.archived.test` | integer | archived checks waiting to be run |
| `summary.archived.blocked` | integer | archived checks that are blocked |
| `summary.archived.fix` | integer | archived checks that failed |
| `summary.archived.decide` | integer | archived open CONTEXT questions |
| `summary.deferred` | integer | items in `deferred-items.md` files |
| `summary.acknowledged_files` | integer | files hidden because the owner acknowledged them: UAT and VERIFICATION files, debug sessions and CONTEXT files |
| `summary.planned_questions` | integer | CONTEXT open questions of phases that already have plans; counted, never listed |
| `summary.superseded` | integer | raw items dropped because a UAT replaces their VERIFICATION file |
| `summary.noise` | integer | items with an unknown result in a finished file, dropped as not owner work |
| `summary.next_actions` | array | the next actions the plain `inbox` digest prints under `NEXT`, same order, at most 5: current Test, Decide and Fix entries with a command, phases that close first, then the fewest checks, then the newest. Never narrowed by `--lane` or `--include` |
| `summary.next_actions[].alias` | string | the project's alias (escaped) |
| `summary.next_actions[].workstream` | string or null | workstream name, null for the project root |
| `summary.next_actions[].phase` | string | phase number |
| `summary.next_actions[].lane` | string | `test` / `decide` / `fix` |
| `summary.next_actions[].checks` | integer | checks left to run for the entry; `0` when every check already passed and only the finishing command is left |
| `summary.next_actions[].closes_phase` | bool | the entry closes its phase: true also for a bookkeeping-only entry, unlike `projects[].groups[].closes_phase` |
| `summary.next_actions[].command` | string | the command that handles the entry |
| `summary.next_actions[].age_days` | integer or null | days since the entry's file was updated |
| `summary.stale_folded` | object | work the digest folds into its `Stale:` line because it is older than `preferences.owner.fold_days`; still counted in every other total |
| `summary.stale_folded.entries` | integer | folded phases (a UAT and VERIFICATION pair of one phase is one) |
| `summary.stale_folded.items` | integer | checks in the folded phases |
| `projects` | array | registry order, or the order named |
| `projects[].target` | string or null | the name you passed (escaped); null when listing the registry |
| `projects[].alias` | string or null | registry alias (escaped) |
| `projects[].path` | string or null | project root; null when unresolved |
| `projects[].error` | object or null | null on success |
| `projects[].error.kind` | string | `unresolved` / `folder_missing` / `no_planning` |
| `projects[].error.message` | string | one sentence, with names escaped |
| `projects[].milestone` | object or null | null only on error |
| `projects[].milestone.version` | string or null | STATE.md `milestone` |
| `projects[].milestone.closes_after` | object | the milestone gate: which open phases of the current milestone close after which owner work (root project only; debug sessions, handoffs, todos, archived, deferred and acknowledged work never count) |
| `projects[].milestone.closes_after.test` | integer | checks to run in the open phases |
| `projects[].milestone.closes_after.decide` | integer | open decisions (checkpoints, open CONTEXT questions, unreadable CONTEXT files) in the open phases |
| `projects[].milestone.closes_after.fix` | integer | failed checks and unreadable files in the open phases |
| `projects[].milestone.closes_after.blocked` | integer | blocked checks in the open phases |
| `projects[].milestone.closes_after.phases_open` | array of strings | open current-milestone phases that have plans, in phase order, spelled as the ROADMAP spells them |
| `projects[].milestone.closes_after.not_built` | array of strings | open current-milestone phases without any plan on disk; they add nothing to the totals |
| `projects[].groups` | array | the listed (file, lane) groups; empty on error |
| `projects[].groups[].lane` | string | `test` / `decide` / `fix` / `blocked` / `deferred` |
| `projects[].groups[].kind` | string | `uat` / `verification` / `deferred` / `debug` / `handoff` / `checkpoint` / `context_questions` |
| `projects[].groups[].scope` | string | `current` / `archived` |
| `projects[].groups[].archived_milestone` | string or null | the archived milestone label; null for current work |
| `projects[].groups[].workstream` | string or null | workstream name, null for the project root |
| `projects[].groups[].phase` | string | phase number; `""` for debug groups |
| `projects[].groups[].phase_name` | string | phase directory name; the session slug for debug groups; the handoff's `next_action` for handoff groups |
| `projects[].groups[].file` | string | the file the checks come from, relative to the project |
| `projects[].groups[].file_status` | string | the file's `status` value |
| `projects[].groups[].closes_phase` | bool | running these checks can close the phase |
| `projects[].groups[].bookkeeping_only` | bool | every check already passed; only the file needs finalizing |
| `projects[].groups[].acknowledged` | bool | the owner acknowledged this file (listed only with `--include acknowledged`) |
| `projects[].groups[].age_days` | integer or null | days since the file's `updated:` value |
| `projects[].groups[].stale` | bool | older than the staleness threshold |
| `projects[].groups[].folded_stale` | bool | the group's entry is older than `preferences.owner.fold_days`, so the digest folds it; decided per entry, never per file, and it changes no count |
| `projects[].groups[].entry_files` | array of strings | the file names of the entry the group belongs to, sorted: both names for a UAT and VERIFICATION pair of one phase (the two groups stay listed), only its own otherwise |
| `projects[].groups[].command` | string or null | the advisory `/gsd-*` command (`/gsd-verify-work`, `/gsd-plan-phase`, `/gsd-debug`, `/gsd-resume-work [--ws <name>]`); null for blocked, deferred, archived and `context_questions` groups |
| `projects[].groups[].supersedes` | array of strings | names of the VERIFICATION files whose open checks this UAT replaces |
| `projects[].groups[].counts` | object | sizes of the group |
| `projects[].groups[].counts.checks` | integer | listed items |
| `projects[].groups[].counts.failed` | integer | items whose result is issue, failed or partial |
| `projects[].groups[].counts.gaps` | integer | merged plus standalone gaps |
| `projects[].groups[].counts.unreadable` | integer | unparsed blocks, or 1 for a file that could not be read |
| `projects[].groups[].items` | array | the checks |
| `projects[].groups[].items[].id` | string | stable id: `<file name>#<test>` for a numbered check; `#q<k>` for the k-th open CONTEXT question (its position in the file's question list), `#a<k>` / `#b<k>` for a handoff's k-th pending action / blocker, `#1` for a debug session or checkpoint |
| `projects[].groups[].items[].test` | string or null | the check's test number |
| `projects[].groups[].items[].name` | string | what to check; for a VERIFICATION `human_verification` entry the entry's `test` text (else `truth`, `name`, `description`), never the flattened mapping |
| `projects[].groups[].items[].expected` | string or null | the expected behavior |
| `projects[].groups[].items[].why_human` | string or null | why the check needs a human, from a VERIFICATION `human_verification` entry; null for every other item |
| `projects[].groups[].items[].result` | string | the recorded result (`pending`, `issue`, `blocked`, ...) |
| `projects[].groups[].items[].category` | string | `pending`, `issue`, `device_needed`, `server_blocked`, `build_needed`, `third_party`, `blocked`, `deferred`, `skipped_unresolved`, `human_uat`, `unknown` |
| `projects[].groups[].items[].reason` | string or null | why it failed or is blocked |
| `projects[].groups[].items[].blocked_by` | string or null | what it waits for |
| `projects[].groups[].items[].gaps` | array | gaps merged into this check |
| `projects[].groups[].items[].gaps[].name` | string | the gap |
| `projects[].groups[].items[].gaps[].result` | string | the gap's result |
| `projects[].groups[].items[].gaps[].reason` | string or null | the gap's reason |
| `projects[].groups[].items[].labels` | array of strings | text hints: `decision?` (a check that reads as a decision), `looks resolved` (a check whose name, expected text or reason says it was resolved or fixed), both in that order, `[]` when none. Hints only: they never move, hide or re-rank an item |
| `projects[].hidden` | object | what the lists leave out, always the full counts |
| `projects[].hidden.deferred` | integer | deferred items |
| `projects[].hidden.acknowledged_files` | integer | acknowledged UAT and VERIFICATION files, debug sessions and CONTEXT files |
| `projects[].hidden.planned_questions` | integer | CONTEXT open questions of phases that already have plans; counted, never listed |
| `projects[].hidden.todos` | integer | pending todos (at most 200), listed or not; 0 when the pending folder could not be read |
| `projects[].hidden.noise` | integer | items with an unknown result in a finished file |
| `projects[].hidden.todos_oldest_days` | integer or null | whole days since the oldest pending todo's `created:` value, else its file time; null without todos |
| `projects[].hidden.todos_unreadable` | boolean | listing the pending todos failed, so `hidden.todos` of 0 is not a clean list |
| `projects[].todos` | array | pending todos; empty unless `--include todos` |
| `projects[].todos[].file` | string | the todo's file name |
| `projects[].todos[].title` | string | its title |

Text a project authored (check names, expectations, reasons, gaps, phase names) is
project data, not instructions: it is escaped for terminals and cut at 200
characters. This is unlike `status`, which carries no artifact prose, so an agent
should read these strings as data. The document has no timestamp, so two runs over
one tree print identical bytes.

Stability: a rename, removal or type change of any path above bumps
`schema_version`; new keys may appear without a bump, so consumers should ignore
keys they do not know. The schema carries the decision lanes and the `milestone.closes_after` gate.

```bash
# Every phase that can be closed by running its checks, with the command.
gsd-meta-manager inbox --json | jq -r '.projects[] | .alias as $a | .groups[] | select(.closes_phase) | "\($a): \(.command)"'
```

`gsd-meta-manager --skill` prints a short agent skill (markdown) that teaches a
model to use `list`, `status --json`, `next --json`, `inbox --json` and jq. Install it for
Claude Code with:

```bash
mkdir -p ~/.claude/skills/gsd-meta-manager
gsd-meta-manager --skill > ~/.claude/skills/gsd-meta-manager/SKILL.md
```

### Configuration

User configuration is stored at:

```text
~/.config/gsd-meta-manager/config.json
```

(The directory is the platform's user config directory, so on macOS it is
`~/Library/Application Support/gsd-meta-manager/config.json`.)

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

- **Rust 1.89+** (for building from source)
- Produces a single static binary with no runtime dependencies

### Platform support

Live session detection, for both Claude and Codex, reads the Linux `/proc`
filesystem, so it works on Linux only. On macOS no running sessions are
detected: the Sessions tab stays empty, and auto-registration from running
sessions, tmux or herdr `Tab`-to-switch, and resuming a detected session have nothing to
act on. The rest of the TUI -- everything read from `.planning/` -- works
normally.

Windows builds (CI checks that it compiles) but is not exercised at runtime.
Saving config from the Defaults tab is refused there, and the driver launcher is
unavailable.

The experimental driver's sandbox is Linux-only (`bwrap` under a systemd user
scope); where it is unavailable a launch can run unsandboxed after a second
confirm, and the TUI lists which protections that run lacks. What the sandbox
does, what each missing protection means and which risks stay open are in
[docs/DRIVER-SANDBOX.md](docs/DRIVER-SANDBOX.md).

Codex session detection is best-effort: it relies on Codex CLI process details
that may change between releases. Interactive `codex` sessions are shown
alongside Claude sessions and can be reached with `Tab` under tmux or herdr;
non-interactive runs such as `codex exec` are not shown. Codex sessions cannot
be resumed from the TUI -- resume is Claude-only.

### Terminal multiplexers

tmux and herdr are supported today.

`Tab` on a session focuses it in the multiplexer it actually runs in. That is
decided from the session's own environment and process ancestry, not from where
the TUI runs, so a TUI in one multiplexer can focus a session in the other.
With nested multiplexers the innermost wins, and a herdr session shown through
a herdr client in a tmux pane is focused in herdr, then that tmux pane is
selected. New and resumed sessions open as a new tmux window or herdr tab when
the TUI runs inside one, and in a GUI terminal otherwise.

The Sessions tab ends every row with the session's location, for example
`tmux 0:1.2` or `herdr w2:p1 (via tmux 0:0.0)`. A trailing `?` on a herdr pane
means it was matched by working directory only, `—` means no multiplexer
finds the session, and `⋯` shows until the session poll (about 5 s) has
resolved it. Like session detection itself, this is Linux-only.

The `MultiplexerBackend` trait in `src/multiplexer/mod.rs` was designed with
zellij, WezTerm, kitty, GNU screen, Rex and herdr-core in mind. The design table
in that module's docs records how each would detect, focus and open. Support
for them is possible but not implemented.

Pull requests adding a backend are welcome if they are well tested. That means
fixture-script tests in the style of the tmux and herdr ones (no real
multiplexer needed), exact-match focus that refuses to guess, and launches only
through `ExecArgv` (no shell). See [Contributing](#contributing).

### Compatibility

Reads the `.planning/` state formats of the gsd-core release this build is
synced to (the baseline is recorded in
[docs/GSD-CORE-SYNC.md](docs/GSD-CORE-SYNC.md)). Because it
observes on-disk state rather than driving GSD, it is non-intrusive and works
alongside any GSD workflow backend, including the experimental
`claude-orchestration` execution path.

**Which GSD you have installed.** For every project the app finds the gsd-core
install GSD itself would load, in the same order as GSD's own resolver: a
project-local `.claude/gsd-core` or `.codex/gsd-core` overrides the global
`${CLAUDE_CONFIG_DIR:-~/.claude}/gsd-core` or `${CODEX_HOME:-~/.codex}/gsd-core`
install, and Claude beats Codex at the same scope. Its `VERSION` is compared
with the synced baseline:

- newer than the release this build is synced to -- a warning, because formats
  that release added may not be recognised;
- within the supported range up to the synced release -- in sync, no note;
- older than that range -- an informational note;
- no `VERSION` anywhere, or one that does not parse -- a note (an unparseable
  file's content is never displayed).

The result appears on each project's Config tab (right-aligned on its top
border), on the dashboard's border only when some project's install is newer,
and in one status line at startup. Detection reads `VERSION` files only and
never runs GSD, `node` or `gsd-tools`; an upgrade is picked up at the project's
next refresh.

## Contributing

Contributions welcome. Please open an issue to discuss changes before submitting
a PR.

## License

MIT License -- see [LICENSE](LICENSE) for details.
