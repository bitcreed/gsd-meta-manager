---
phase: quick-260509-t8m
plan: 01
status: complete
date: 2026-05-09
commits:
  - ed1f3b2
files_modified:
  - src/session_detector.rs
  - src/lib.rs
  - src/terminal_switch.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/detail.rs
---

# Quick Task: Tab-to-switch into a Claude session (tmux)

## What Changed

1. `ClaudeSession.tty` — new field. Populated from
   `/proc/<pid>/fd/0` and stored without the `/dev/` prefix
   (e.g. `pts/3`). The leading slash is stripped on purpose: tmux's
   `#{pane_tty}` formatter prints `/dev/pts/3`, so a `contains()` match
   on the bare `pts/3` value lands cleanly without us having to
   reconstruct the full path.

2. `terminal_switch.rs` — new module exposing
   `switch_to_session(&ClaudeSession) -> Result<(), String>`.
   - Bails with a friendly error when `$TMUX` is unset.
   - Lists all panes on the running tmux server.
   - Finds the pane whose TTY contains our session's TTY, then runs
     `tmux select-window -t <target>` and `tmux select-pane -t <target>`.
   - Designed to be extended later for kitty/wezterm/ghostty by
     branching on a `detect_terminal()` enum (claudectl-style); for
     now we only do the tmux path the user asked for.

3. `KeyCode::Tab` on the project-list overview
   (`ui/screens/normal.rs`) — finds the first active session whose
   `working_dir` matches the selected project's path and switches.
   Surfaces success/failure via the status line.

4. `KeyCode::Tab` on the project-detail screen
   (`ui/screens/detail.rs`) — when on the Sessions tab, switches to
   the highlighted session. On any other tab, falls back to the same
   project-level lookup as the overview.

5. Footer hints updated:
   - Overview: `[Tab] session`
   - Detail Sessions tab: `[Tab] switch`

## Verification

- `cargo build`: clean
- `cargo clippy`: no issues
- `cargo test`: 102 passed (5 suites)
- Live sanity check: `pgrep -x claude` → 3 PIDs, each
  `readlink /proc/<pid>/fd/0` returns `/dev/pts/N`, and
  `tmux list-panes -a -F ...` returns lines starting with `/dev/pts/N`.
  The `contains(session.tty)` lookup matches in all three cases.

## Notes

- We deliberately did NOT mirror the full claudectl terminal-detection
  matrix (kitty, ghostty, wezterm, iterm2, apple terminal,
  windows-terminal). The user asked for "works e.g. with tmux" — we
  give a clear error in non-tmux environments rather than silently
  failing or implementing six terminals' worth of switching code.
  The module is structured so adding more is a matter of branching
  on `detect_terminal()` at the top of `switch_to_session`.
- The Sessions-tab Tab handler intentionally takes priority over the
  general project-tab fallback, because users on that tab will have
  picked a specific session out of multiple possibilities.
