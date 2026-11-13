---
phase: quick-261113-dcj
plan: 01
subsystem: agents/adapters
tags: [claude-code, sessions, config-roots]
status: complete
commits: 4
plan_head_before: 7c857d5ba76e548469575a218a4971c1fd0d031e
plan_head_after: 9bbb2fbf3da85c791f7f676cb679ad90fa6f2227
key-files:
  modified:
    - src/agents/adapters/claude.rs
    - tests/agents_claude.rs
actuals:
  tasks: 2
  commits: 4
---

# Quick 261113-dcj: Sessions panel sees both Claude config roots

The Claude Code adapter now scans both `$CLAUDE_CONFIG_DIR` and `<home>/.claude` in one pass, each project directory once.

## What changed

- `ClaudeCodeAdapter` holds `config_roots: Vec<PathBuf>` (CLAUDE_CONFIG_DIR root first, then `<home>/.claude`, at most two).
- `with_env(Option<OsString>, Option<PathBuf>)` is the injected-environment seam; `from_env` delegates to it and stays the module's only process-environment read.
- `resolve_config_roots` is built from the unchanged `resolve_config_root`, so a blank, whitespace-only or relative value counts as unset (T-25-12).
- `project_dirs_across` merges per-root project directories and dedupes with `same_path` (raw or canonical), keeping the first spelling.
- Module, struct and constructor docs updated.

## Tests added (5 net new tests + 1 tracer; suite 3827 -> 3832)

- `resolve_config_roots_lists_the_env_root_then_the_home_root`
- `a_config_dir_aliasing_the_home_root_scans_once` (unix; went red against raw dedupe, green with `same_path`)
- `a_blank_or_relative_config_dir_scans_only_the_home_root`
- `the_home_root_keeps_every_safety_property` (unix; symlinked meta, symlinked session entry, oversize meta, transcript-only valid-looking bytes, with a listed control)
- `both_claude_config_roots_reach_the_scan` (tests/agents_claude.rs, real git worktree, with both single-root controls)

## Verification

- `cargo test --no-fail-fast`: exit 0, 3832 passed, 0 failed (log: target/dcj-full-tests.log).
- `cargo fmt --all -- --check`: clean. `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo check --locked` and `cargo clippy --locked -- -D warnings` with `--all-targets` for aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-gnu: all exit 0.
- Invariant guards: no `set_var`/`remove_var`; no `home_dir` or `from_env()` in the test region or in tests/agents_claude.rs; `from_env()` appears once in production code (its definition).

## Deviations from Plan

None - plan executed as written. `cargo fmt` also reflowed three lines of tests/agents_claude.rs from Task 1; that formatting is included in the Task 2 test commit.

## Inferred decisions [inferred — audit]

- Dedupe point is the project directory via `same_path` (covers aliased root and aliased `projects/`), first spelling kept.
- `InstallRoots::claude_home` not used (expands `~`, accepts relative values, contradicting T-25-12).
- A symlinked config root or `projects/` is still followed, as it was for the single root (user's own configuration).

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

Commits f9ca9e9, 3816094, 1391e25, 9bbb2fb exist on the branch; both files touched exist.
