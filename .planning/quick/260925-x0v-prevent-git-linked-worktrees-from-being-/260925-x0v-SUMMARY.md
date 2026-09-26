---
quick_id: 260925-x0v
phase: quick-260925-x0v
plan: 01
status: complete
subsystem: registry
tags: [registry, git-worktree, auto-register, config-prune]
requires: []
provides:
  - registry::linked_worktree_main
  - registry::LinkedWorktreeRefusal
  - registry::prune_worktree_entries
  - registry::load_config_pruning_worktrees
  - state_reader::git_ops::git_dir_pair
  - agents::worktrees::{path_under_claude_worktrees, has_claude_agent_worktree_segment}
affects: [auto-registration, CLI add/list, TUI Add, create-project, TUI launch]
tech-stack:
  added: []
  patterns: [structural gitfile classification with git fallback, prune-on-load at selected call sites only]
key-files:
  created:
    - tests/registry_worktree_guard.rs
  modified:
    - src/registry.rs
    - src/main.rs
    - src/app.rs
    - src/state_reader/git_ops.rs
    - src/agents/worktrees.rs
    - src/ui/screens/add_project.rs
    - README.md
    - docs/GETTING-STARTED.md
decisions:
  - "[inferred] Bare repository: main_of returns the repository itself (no main worktree exists)"
  - "[inferred] Auto-register skip logs at debug, not warn (5s poll would flood the log)"
  - "[inferred] Linked worktree never auto-registered even when its main worktree is unregistered"
  - "[inferred] Manual add does not apply the path-shaped secondary signal; only structural+git verdict refuses"
  - "[inferred] Pruning-loader save failure is logged and non-fatal; retries next launch"
  - "[inferred] Pruned list is log-only (no TUI banner)"
  - "[inferred] CLI remove keeps plain load_config so `remove agent-...` still finds its entry"
metrics:
  started: 2026-09-26T04:55:19Z
  completed: 2026-09-26T05:05:59Z
  duration: ~11m
  tasks: 3
  files: 9
actuals:
  tokens: 13300
  tasks: 3
  commits: 3
plan_head_before: 1114da3e10f5a848b4baf4a42160d4bbeb6f39aa
---

# Quick 260925-x0v: Prevent git linked worktrees from being registered as projects Summary

Now no registration route can add a git linked worktree. A filesystem-only check spots them: it reads the `.git` gitfile and then `commondir` or the `worktrees/` path layout, and only calls `git rev-parse --git-dir --git-common-dir` when that read is inconclusive. Both registration functions return a typed `LinkedWorktreeRefusal` that names the main worktree. Stale worktree entries are pruned when the config loads at TUI launch and in CLI `add`/`list`.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 (tracer) | cc60108 | Filesystem worktree check, `LinkedWorktreeRefusal` in `add_project` + `add_project_unchecked`, auto-register skip, CLI refusal, real-git integration tests |
| 2 | bc88ce8 | `git_ops::git_dir_pair` fallback, shared `.claude/worktrees` path helpers + secondary discovery signal, fake-gitfile tests (absolute/relative/pruned admin/submodule/plain/FIFO), TUI Add refusal test |
| 3 | cdd7d94 | `prune_worktree_entries` + `load_config_pruning_worktrees`, wired into `App::new` and CLI add/list, README/GETTING-STARTED note |

## Entry points guarded

- **Auto-discovery** (`registry::auto_register_from_sessions`): quietly skips a linked worktree (debug log). It also skips a cwd containing `.claude/worktrees/agent-*`, or a cwd under a registered project's `.claude/worktrees/`. `add_project` still refuses as the final check.
- **CLI `add`** (`src/main.rs` Add arm): uses `downcast_ref::<LinkedWorktreeRefusal>`, prints both lines through `text::render_for_terminal`, suggests `gsd-meta-manager add <main>`, and exits 1.
- **TUI Add** (`ui/screens/add_project.rs::do_add_project` → `registry::add_project`): the refusal shows in `ctx.error_message` (test asserts it names the main path).
- **Create-project** (`app.rs` → `registry::add_project_unchecked`): same refusal, shown as "Failed to register: ...".
- **Prune on load**: `App::new` and the CLI `add`/`list` arms call `registry::load_config_pruning_worktrees`. `remove`, `drive`, `envelope/hooks.rs` and `envelope/cred.rs` stay on plain `load_config`, which never writes.

## Verification

- Full suite (`rtk proxy cargo test --no-fail-fast`): **2506 passed / 1 failed / 15 ignored**. The one failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, which is expected on this machine (local git 2.53.0, but the test is pinned to 2.55.0).
- New tests: 4 integration tests (`tests/registry_worktree_guard.rs`, real git, none skipped), 10 in-source registry tests, 1 `git_ops` real-git test, 2 `agents::worktrees` pure tests, 1 TUI state test.
- `spawn_seam_guard`, `async_blocking_guard`, `agents_scan`, `registry_test` and the `config::` lib tests are green. `registry.rs`, `worktrees.rs` and `add_project.rs` gained no spawn site.
- Manual (no human needed): I copied `~/.config/gsd-meta-manager/config.json` to the scratchpad and ran the freshly built binary with `--config <copy> list`. The copy went from 15 entries to 14, `agent-ad8ab6d33e06a2f84` was removed from both the listing and the file, and the real config's sha256 is unchanged (2532d877...).
- Manual CLI refusal against a real `git worktree add` repo: exits 1, and stderr names the main path and suggests `add <main>`.
- `cargo clippy --all-targets -- -D warnings`: **no lints from this change.** It still fails overall because of 11 existing lints in files this change doesn't touch, which clippy 1.98.1 now reports: `src/browser.rs:156-158`, `src/project_creator.rs:146`, `tests/envelope_config_resolution.rs:2206`, `tests/envelope_control_carrier.rs:978`, `tests/envelope_carrier_reach.rs:1711`, and `tests/envelope_wrapper_class.rs:6127,6213,10795,11232`. With exactly those lint classes allowed, clippy finishes clean.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Moved new in-source test modules above `mod tests {`**
- **Found during:** Task 2
- **Issue:** `spawn_seam_guard::no_production_item_follows_a_test_module_marker` rejects column-zero items after a file's `mod tests {` marker.
- **Fix:** `worktree_fixture` and `linked_worktree_tests` in `src/registry.rs` now sit above the existing `#[cfg(test)] mod tests {`.
- **Commit:** bc88ce8

**2. [Rule 3] Dropped the now-unused `load_config` import in `src/app.rs`** (Task 3, cdd7d94).

### Deferred Issues

- The 11 existing clippy 1.98 lints listed above. They are outside this change's scope and were not fixed.

## Notes

- The FIFO case was covered with `rustix::fs::mknodat` (rustix is already a dependency, `fs` feature). The test is linux-only (`#[cfg(target_os = "linux")]`).
- Prune warnings go through `tracing::warn!`, but the binary's default `EnvFilter` (with no `RUST_LOG` set) records only errors. The warning therefore shows up in the log only when `RUST_LOG` includes `warn`. This is existing logging behaviour.
- The `.claude/worktrees` helpers are `pub(crate)`, so the integration test uses only the public registry API.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: tests/registry_worktree_guard.rs
- FOUND commits: cc60108, bc88ce8, cdd7d94
