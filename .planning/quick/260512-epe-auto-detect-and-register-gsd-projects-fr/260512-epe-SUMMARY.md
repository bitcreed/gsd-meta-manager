---
quick_id: 260512-epe
description: Auto-detect and register GSD projects from active claude sessions
status: complete
date: 2026-05-12
commits:
  - 6a40986 feat(quick-260512-epe): add auto_register_from_sessions helper
  - b350b46 feat(quick-260512-epe): auto-register new GSD projects from claude sessions
---

# Quick Task 260512-epe — Summary

## What Changed

Active Claude sessions whose `working_dir` is an unregistered GSD project (contains `.planning/`) are now auto-registered in the meta-manager.

The TUI runs a one-shot startup scan, and the existing 5-second session poll re-runs the same logic on every tick — so new claude sessions opened after launch also get picked up without restart.

## Files Modified

- **`src/registry.rs`** — Added `auto_register_from_sessions(&mut Config, &[ClaudeSession]) -> Vec<(String, PathBuf)>`. Canonical-path dedup against the existing registry; alias collisions resolved with `-2`, `-3`, … suffixes. 6 new unit tests cover: new path added, existing canonical path skipped, non-GSD path skipped, duplicate sessions in one pass register once, alias collision uses suffix, returned paths are canonical.
- **`src/app.rs`** — Added `App::auto_register_new_sessions()` that persists the config, starts the file watcher on the new `.planning/` dir, loads initial state, records a change-tracker snapshot, emits `tracing::info!`, and sets a `Auto-registered: <alias>` status message per added project. Wired into the `Action::SessionsDetected` handler.
- **`src/main.rs`** — Runs `detect_sessions()` + `app.auto_register_new_sessions()` once before `event_bus.spawn_tick(250)`, so projects appear before the first poll tick.

## Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Source | Active `/proc` claude sessions only | User asked for this scope; reuses `session_detector.rs` |
| Trigger | Startup scan + existing poll tick | No new polling infrastructure; piggybacks on the 5s session poll |
| UX | Auto-add silently + log + status row | Lowest friction; surfaces what happened without blocking the user |
| Dedup | Canonical path compare | Symlinks and trailing-slash variants don't double-register |
| Alias | `path.file_name().to_lowercase()` with `-N` suffix on collision | Predictable; matches the existing CLI `add` defaulting |
| Save failures | Log via `tracing::warn!`, continue | In-memory registration still benefits the current session |

## Verification

- `cargo build` — clean
- `cargo test` — **111 passed** (6 new for `auto_register`)
- `cargo clippy --lib -- -D warnings` — no issues
- `cargo clippy --bin gsd-meta-manager -- -D warnings` — no issues

## Out of Scope

- Auto-removing projects when their session ends (registry persistence is intentional).
- Scanning `~/.claude/projects/` directory for historical projects (decision: active sessions only).
- Confirmation UI before adding (decision: silent + notify).
