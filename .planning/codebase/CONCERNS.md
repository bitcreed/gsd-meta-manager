# Codebase Concerns

**Analysis Date:** 2026-08-18

This is a Rust ratatui TUI that, as of milestone v2.0, spawns and supervises `claude -p`
child processes to drive other GSD projects (`src/driver/`, `src/executor/`). The relevant
risk surface is process supervision, filesystem trust boundaries between the TUI and the
projects it manages, and secret handling in captured agent output — not typical web-app
concerns.

The codebase is unusually disciplined about the risks it does recognize: extensive
doc-comments cite the design decision (`D-NN`) or code-review finding (`CR-NN`/`WR-NN`)
that shaped each defensive line, mechanical tests guard against permission-bypass flags and
development-only flags leaking into production argv, and a dedicated redaction module
(`src/journal/redact.rs`) sits on the capture path before anything reaches disk. Most gaps
below are already tracked with a scheduled owner rather than freshly discovered.

## Tracked with a Scheduled Owner (verified still present)

These are pre-existing, deliberately deferred items recorded in `.planning/STATE.md` /
`17-08-PLAN.md`'s deferral table. Each was re-checked against current source for this audit;
all are confirmed still open exactly as logged, not fixed since.

### WR-02 — `run_id` path-join has no component validation

- Files: `src/journal/writer.rs` (`create_run_dir` line 329, `run_paths`, `write_active_pointer`
  line 440, and the raw `root.join(run_id)` / `root.join(&run_id)` uses at lines 502 and 672)
- Issue: `run_id` (attacker/caller-controlled string, ultimately sourced from `--run-id` on a
  spawned driver's argv) is joined directly into a filesystem path with no check that it is a
  single, traversal-free path component. Reproduced during Phase 17: a `run_id` containing
  `../` segments can escape the project's run directory.
- Status: deliberately deferred (WR-02 in `17-08-PLAN.md`'s table) as "a separate hardening
  axis (path-component validation across four call sites in `src/journal/`), untouched by any
  fix here." Owner: a future hardening phase, not yet scheduled by name.
- Fix approach: validate `run_id` is a single path component (no `/`, no `..`, no leading `.`)
  at the one seam where it enters `src/journal/`, before any of the four call sites join it.

### WR-10 — blocking `flock`/filesystem/git calls inside `async fn`

- Files: `src/driver/lock.rs` (`acquire`, uses `rustix::fs::flock` synchronously), `src/driver/mod.rs`
  (`drive`, `dry_run`)
- Issue: `flock(2)` and other blocking fs/git calls run directly inside `async fn` bodies,
  which can starve the async runtime's worker thread. This is not hypothetical — it caused an
  **observed deadlock** in `tests/driver_lock.rs` (see the block of comments starting around
  line 209: "the run body's blocking work runs on the ... suite deadlocks instead of" and the
  WR-10/D-28 regression note at line 244–288).
- Status: deliberately deferred — "a runtime-shape change across `drive`, `lock::acquire` and
  `dry_run`; no blocker depends on it." `tests/driver_lock.rs` carries a regression test
  documenting the deadlock so a future fix has a red test to turn green.
- Fix approach: move blocking calls to `tokio::task::spawn_blocking`, or make the lock
  acquisition synchronous end-to-end and hand it off from a dedicated blocking thread.

### WR-15 — `DriverStopped` drops the observed run unconditionally

- Files: `src/action.rs` (`Action::DriverStopped` definition, line 224), `src/app.rs` (dispatch
  around line 1501, 1974)
- Issue: the `DriverStopped` action always drops the observed run from UI state, even when the
  underlying stop request did nothing (e.g., the run was already gone). The action carries no
  `StopOutcome` distinguishing "I stopped it" from "there was nothing to stop."
- Status: deliberately deferred — "requires carrying `StopOutcome` as a value through `Action`;
  a UI-state change, not a teardown one." Related: WR-11 ("silent return with no event channel"
  inside `stop_driver_run`) shares the same code path and was folded into the same CR-05 note.
- Fix approach: extend `Action::DriverStopped` with a `StopOutcome` enum (stopped / already-gone
  / error) and branch the UI update on it instead of unconditionally clearing the row.

### WR-16 — hidden `--claude-program` / `--claude-args` flags ship in release builds

- Files: `src/cli.rs` (flag definitions, lines 138–143), `src/driver/spawn.rs` (doc comment,
  line 25: "carries two hidden development flags")
- Issue: `--claude-program` and `--claude-args` let a caller point the driver's executor at an
  arbitrary program instead of the real `claude` binary. They are undocumented/hidden but not
  compiled out of release builds, so a hand-typed `drive` invocation on a shipped binary can
  substitute a different executable for the agent.
- Status: deliberately deferred — "every integration test in this phase depends on those flags
  being present in the test build." `src/driver/spawn.rs::drive_argv` is separately proven
  (`the_drive_argv_carries_no_development_flag` test) to never let the **TUI-initiated** spawn
  path emit either flag, so the risk is scoped to a hand-typed CLI invocation, not the TUI's own
  spawn seam.
- Fix approach: `#[cfg(any(test, feature = "dev-fixtures"))]`-gate the two flags so they cannot
  exist in a release binary, and update the integration tests to build with the feature enabled.

### Unbounded git authority in a driven run — Phase 19 scope, not an unrecognized gap

- Files: `src/state_reader/git_ops.rs` (all git subprocess calls), `src/journal/writer.rs:365`
  (a `git` invocation in the journal writer itself), `src/executor/outcome.rs` (head-sha/dirty
  checks after a run)
- Issue: a driven agent process (`claude -p`, spawned via `src/driver/spawn.rs`) has no
  enforced boundary on which git operations it may perform inside the project it is driving —
  commit, push, branch changes are all reachable with `--permission-mode dontAsk`.
- Status: **this is Phase 19's stated goal** ("GITSAFE — Git & Blast-Radius Envelope"), currently
  planned (8 plans, 8 waves per `19-PLAN.md`) but not yet executed as of this audit
  (`.planning/STATE.md`: `current_phase: 19`, `status: executing`, `Plan: Not started`). Report
  this as a scheduled gap, not a new discovery — the phase exists specifically because the team
  already identified it.

### Broken-windows ledger — 1 open deviation

- File: `src/ui/screens/driver.rs` (no specific line recorded)
- Issue (from `.planning/WINDOWS.md`, id 1): "18-10: injection state refuses to promote an
  interjected record with `delivered:false`" — a deliberate deviation toward the safer
  prohibition rather than the originally planned behavior.
- Status: open, unwaived, blocks `/gsd-ship` while `open_count > 0`. Owner: revisit "if the
  driver ever stops writing the failed-write record."

### 3 open todos in `.planning/todos/pending/`

1. **Badge glyph display width** (`2026-07-29-badge-glyph-display-width-alignment.md`) — UI
   alignment issue with wide glyphs in status badges.
2. **Driver tab blank pipeline row at terminal heights 8–13** (`src/ui/screens/driver.rs`) —
   logged as Phase 18 code-review finding WR-05; the medium-height layout tier renders section
   headers ("── steps ──") with no content underneath at that height band. Needs an additional
   height tier or a minimum-height single-line fallback; requires observation at a real
   terminal (UAT), not a unit test.
3. **`$EDITOR` exit does not invalidate `cache.browser_file_content`** (`src/main.rs`) — after
   shelling out to `$EDITOR`, the markdown viewer's file-content cache is not invalidated on
   resume, so a user's own edit does not appear until some unrelated event refreshes the cache.
   Carried forward from Phase 14 (WR-03); no PTY/suspend-resume test harness exists in this
   repo, so closing it needs either a testable seam or an explicit manual UAT step.

### 5 pre-existing `cargo clippy --all-targets` lints

Reconfirmed by direct clippy run on 2026-08-18; all five below are the only lints reported.

- `src/browser.rs:131`, `132`, `133` — three `assert_eq!(x, true)` / `assert_eq!(x, false)`
  calls in test code; clippy prefers `assert!(x)` / `assert!(!x)`. (Note: these live in
  `src/browser.rs`, not `src/ui/browser.rs` — there is no `src/ui/browser.rs` in this tree.)
- `src/project_creator.rs:146` — `assert!(result != PathBuf::from("~") || ...)` constructs an
  owned `PathBuf` purely for a comparison; clippy suggests comparing against a borrowed `Path`.
- `src/state_reader/mod.rs:258` — `items after a test module`: the `mod tests { ... }` block is
  not the last item in the file, which clippy flags as an organizational lint (not a bug).

All five are cosmetic/style lints in test code or test-adjacent code, none affecting runtime
behavior. No owner scheduled; low priority.

## Newly Identified Concerns (not accounted for above)

### Two very large "god files" in the UI layer

- `src/ui/screens/detail.rs` — **6,615 lines**, the largest file in the tree.
- `src/app.rs` — **4,366 lines**.
- `src/ui/screens/driver.rs` — **3,436 lines**.
- Impact: these files mix rendering, state transitions, and business logic for their respective
  screens/app-loop in single modules. Navigating and safely modifying any one of them requires
  holding a large amount of context; a future contributor unfamiliar with the file's internal
  organization is more likely to introduce a regression than in a codebase with more granular
  modules. `detail.rs` in particular is more than 1.5x the size of the next-largest file.
- No test-coverage or correctness issue was found tied specifically to file size, but this is a
  maintainability risk worth flagging before the next major feature lands in either file — it
  is not mentioned in `.planning/STATE.md`'s deferral list or the WINDOWS ledger.
- Fix approach: if a future phase touches `detail.rs` or `app.rs` substantially, consider
  splitting render logic from state-update logic into separate submodules (e.g.,
  `detail/render.rs` + `detail/update.rs`) as a low-risk refactor riding along with that work,
  rather than as a standalone project.

### `run.json` and the lock file are written world-readable (`0o644`)

- Files: `src/journal/writer.rs:562-566`, `src/driver/lock.rs:234-236`
- Observation: both explicitly `set_permissions(..., 0o644)`. Given the redaction module's own
  stated threat model ("a false negative is a persistent secret sink under `.planning/`"), any
  future field added to `run.json` or the lock record should be held to the same no-secrets
  bar as journal content, since the file is not access-restricted to the owning user beyond
  standard directory permissions.
- Impact: currently benign — `run.json`/lock records hold pid/pgid/timestamps, not secrets —
  but the permission choice is a standing invariant that a future change could violate silently
  (there is no test asserting `run.json`'s *contents* stay secret-free, only that redaction
  runs on the separate journal capture path).
- Fix approach: no code change needed today; note as a review checkpoint for any future PR that
  adds a field to `RunRecord`.

### No dependency-audit or supply-chain check found

- `cargo audit`, `cargo deny`, or an equivalent supply-chain scan was not found configured in
  CI or as a local tool in this repo (no `deny.toml`, no `audit.toml`, no workflow file
  invoking either). Given that this project executes another program's git/filesystem
  operations autonomously, an unpatched CVE in a transitive dependency (e.g., in `notify`,
  `tokio`, or the git-shelling path) has an elevated blast radius compared to a purely
  interactive tool.
- Fix approach: add `cargo audit` (or `cargo-deny`) as a CI step; low effort, meaningful
  coverage gain, not currently tracked anywhere in `.planning/`.

---

*Concerns audit: 2026-08-18*
