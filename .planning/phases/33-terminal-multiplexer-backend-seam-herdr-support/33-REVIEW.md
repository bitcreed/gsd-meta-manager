---
phase: 33-terminal-multiplexer-backend-seam-herdr-support
reviewed: 2026-11-11T01:50:01Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - Cargo.toml
  - README.md
  - src/agent_skill.md
  - src/cli.rs
  - src/main.rs
  - src/multiplexer/herdr.rs
  - src/multiplexer/mod.rs
  - src/next_cli.rs
  - src/status_cli.rs
  - tests/status_cli.rs
findings:
  critical: 0
  warning: 0
  info: 1
  total: 1
  necessity: 0
status: issues_found
---

# Phase 33: Code Review Report (re-review after code-review fixes)

**Reviewed:** 2026-11-11T01:50:01Z
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

This is an incremental re-review. It covers what changed since the previous review commit `c61782f`, which is mainly the fix commits in `33-REVIEW-FIX.md` (WR-01..03, IN-01..05, UI-01). The evaluation-scope resolver came back `degraded` (`no-task-commits-since`) and widened to the phase-directory range. That range also contains quick task 261110-qap (`status --json`, `--skill`: `src/status_cli.rs`, `src/cli.rs`, `src/main.rs`, `src/next_cli.rs`, `src/agent_skill.md`, `README.md`, `tests/status_cli.rs`) and the dependency trim `1f5f404` (`Cargo.toml`). Those files were reviewed too. `Cargo.lock` is a generated lockfile and was skipped.

**The fixed code (herdr.rs, mod.rs).** None of the fixes introduces a new Critical or Warning.
- WR-01: the cwd tier now chooses only among panes whose process-info could not be read *and* that herdr reports as running the session's agent kind (`cwd_fallback`). A plain shell, or a pane whose process-info names other processes, can no longer be focused. A directory-only success carries `CWD_ONLY_NOTE`, and that note is merged correctly with the tab-fallback note.
- WR-02: `same_dir` compares the paths as written first, then by `canonicalize`. A pane path that cannot be resolved only matches by exact equality.
- WR-03: `is_try_next` is deleted. The docs now describe the real fall-through rule.
- IN-03: `"error": null` is filtered with `.filter(|e| !e.is_null())`. `{"error":null}` with no `result` still fails closed, and a test pins that.
- IN-02 / UI-01: `launch_phrase` keeps socket paths and herdr's JSON out of the status line, and the full reason is logged at `warn`. The `ExecArgv` refusal text is backend-neutral, and the pinned substrings `directly` and `option` survive.
- IN-05: `accept_within` is `#[cfg(all(test, unix))]`, bounded, and sets a read timeout on the accepted stream.

A candidate whose pane id fails `validate_id` is put in `unprobed`, so it can win the cwd tier. The later `validate_id(&pane.pane_id)?` then refuses it, which fails closed. That is acceptable and is not reported.

**Quick task 261110-qap.** `status_cli` is read-only as documented. It uses plain `load_config` and not the pruning loader, and nothing in the module writes. Project-authored strings go through `render_for_terminal` and are capped. `owning_root` compares whole components and picks the deepest root. The error-entry shape matches the eleven-key schema in README and `agent_skill.md`, and the exit codes match the docs. `--skill` conflicts with both a target and a subcommand. Its output goes through `process::exit`, and std's runtime cleanup flushes stdout before the process exits. No defects found.

Necessity was assessed against `necessity-review.md`. No necessity findings.

## Info

### IN-06: `same_dir` adds unbounded filesystem I/O per herdr pane on the UI thread

**File:** `src/multiplexer/herdr.rs:702-716`
**Issue:** `cwd_candidates` calls `same_dir(dir, working_dir)` for every `cwd` and `foreground_cwd` of every pane herdr lists. Any pair that is not byte-equal runs two `std::fs::canonicalize` calls, and that includes canonicalizing the session's own `working_dir` again for every pane. Focus runs on the UI thread (the T-33-05 concern). A pane sitting in a directory on a hung network mount can block `canonicalize` with no bound, so the WR-02 fix adds a new way for the TUI to freeze during focus. This is close to the herdr CLI calls' accepted no-deadline exposure (T-33-13), but it is not covered by that acceptance. It is also wasted work: `working_dir` is already the resolved `/proc/<pid>/cwd`.
**Fix:** Canonicalize `working_dir` once outside the loop, and only canonicalize a pane path whose final component matches (cheap pre-filter):
```rust
let target = std::fs::canonicalize(working_dir).unwrap_or_else(|_| working_dir.to_path_buf());
// ...
.any(|dir| dir == working_dir
    || (dir.file_name() == target.file_name()
        && std::fs::canonicalize(dir).is_ok_and(|d| d == target)))
```
(The basename pre-filter misses a symlink whose name differs from its target's name. If that case matters, keep the full canonicalize but hoist the `working_dir` one.) An alternative is to note this in the T-33-13 acceptance.

---

_Reviewed: 2026-11-11T01:50:01Z_
_Reviewer: Claude (gsd-code-reviewer, run inline by the orchestrator: no Agent tool was available in this session) [inferred, for audit]_
_Depth: standard_
_Finding numbered IN-06, not IN-01, so it does not reuse an ID the disposition ledger has already recorded as fixed [inferred, for audit]_
