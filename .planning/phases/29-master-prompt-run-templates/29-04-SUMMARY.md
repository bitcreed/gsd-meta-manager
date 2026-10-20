---
phase: 29-master-prompt-run-templates
plan: 04
subsystem: driver-templates
tags: [driver-new, driver-check, run-templates, direction-lint, gitignore, symlink-safety]

requires:
  - phase: 27-driverctl-driver-frontmatter
    provides: "split_frontmatter/parse_direction/body_sane, driverctl::log::{parse_log, utc_string}, read_driver_run"
  - phase: 29-master-prompt-run-templates (plan 29-01)
    provides: "launcher::prompt::{MASTER_PROMPT, stop_rules_block}: the stop-rules block copied byte-for-byte into the DIRECTION template"
  - phase: 29-master-prompt-run-templates (plan 29-02)
    provides: "driverctl::store::{frontmatter_for, RunState}: the emitter behind the never-launched frontmatter"
provides:
  - "driver_templates::{mint_run_id, create_run_dir, NewRun, NewRunError, DIRECTION_TEMPLATE, LOG_TEMPLATE}"
  - "driver_templates::lint::{lint_direction, LintReport, Finding, template_placeholders}: the pure, strict DIRECTION lint plan 29-06 reuses as the launch preflight"
  - "gsd-meta-manager driver new <project> <slug> and driver check <project> <run-id>"
affects: [29-05, 29-06 (launch preflight calls lint_direction), 29-07 (harness parses the first driver new line), 29-08]

actuals:
  tokens: 20700
  tasks: 3
  commits: 5

plan_head_before: 953b3b8ab46fc850534ae35a1e4a8263bea3cb2c
plan_head_after: 7c682b72749ce629eecb6c1243ba2befa30691df
commits: 5

tech-stack:
  added: []
  patterns:
    - "run dir made with create_dir (EEXIST = collision) and every file opened create_new; self-ignoring '*' .gitignore written first"
    - "symlink_metadata refusal per path component plus canonical containment before and after creating the parents"
    - "lint blanks HTML comments byte-for-byte (same offsets, newlines kept) so placeholder spans slice the 'static template and line numbers stay true"
    - "source-scanning golden: LOG event names are read from src/driverctl and src/launcher at test time, so a new event fails until the template names it"

key-files:
  created:
    - src/driver_templates/mod.rs
    - src/driver_templates/lint.rs
    - src/driver_templates/assets/DIRECTION.md
    - src/driver_templates/assets/LOG.md
    - tests/driver_new.rs
  modified:
    - src/lib.rs
    - src/cli.rs
    - src/driver_cli.rs

key-decisions:
  - "Slug shape ^[A-Za-z0-9][A-Za-z0-9._-]*$, checked before valid_run_id on the composed id (80-byte bound) [inferred - audit]."
  - "driver check is a separate verb from driver new [inferred - audit, D-11]; a gap-rounds above the launcher default 2 is a warning that never changes the exit code [inferred - audit, RESEARCH OQ1]."
  - "template_placeholders() includes {run-id} (the exact brace spans of the template body outside comments); driver new fills it, so a minted run never reports it, but a hand-written title left as {run-id} is flagged."
  - "A failed write after create_dir removes the three known files and the empty dir (the dir was created by this call), so a failed new leaves no half run."
  - "driver new / driver check are Linux-gated with UnsupportedPlatform stubs elsewhere (see Deviations); the templates and the lint compile everywhere."

patterns-established:
  - "Refusals before the first create_dir: slug, id, .planning, symlink chain, containment."

requirements-completed: [PRMT-02, DRIVE-03, PRMT-03]

duration: 17min
completed: 2026-10-20
status: complete
---

# Phase 29 Plan 04: Run Templates, driver new and driver check Summary

**`driver new` mints a self-ignoring run dir (`.gitignore` `*`, LOG.md with its grammar, DIRECTION.md = Phase 27 emitter frontmatter + a template carrying the master prompt's stop-rules block byte-for-byte) and `driver check` lints the DIRECTION body with file line numbers, exit 0/1, read-only.**

## Performance

- **Duration:** 17 min (2026-10-20T03:30:21Z to 03:47:38Z)
- **Tasks:** 3 (1 tracer, 2 TDD)
- **Files:** 5 created, 3 modified; 5 task commits (+2260 lines)

## Exact `driver new` stdout (harness-parsed in plan 29-07)

```
driver new: created run <yyyymmdd>-<slug>
  direction: <absolute path of DIRECTION.md>
  check:  gsd-meta-manager driver check <project> <run-id>
  launch: gsd-meta-manager driver launch <project> <run-id>
```

`<project>` echoes the argument as given; no `--prompt-file`; no editor is opened. Refusals print one `driver new: ...` stderr line and exit 1 with nothing created. A collision names the existing run dir.

## LOG event names found by the coverage golden (29 names)

From `src/driverctl/*.rs` and `src/launcher/*.rs`, test modules excluded: `auth-relaunch`, `cap-tripped`, `connect`, `crash-detected`, `direction-lock-timeout`, `disconnect`, `dispatch-denied`, `dispatched`, `end`, `gap-coalesced`, `gap-observed`, `launch`, `launch-end`, `needs-human`, `pr`, `pr-failed`, `pr-refused`, `pr-skipped`, `push`, `push-failed`, `push-refused`, `reconcile`, `recovered`, `report`, `skipped-insane-body`, `splice-gave-up`, `start`, `status`, `verified`. Each is named in `assets/LOG.md` at its event column; `launch` documents `prompt=` and `launch-end` documents `stalled`.

## Accomplishments

- **Tracer:** one command creates a run dir git never stages (staging proof through `git add -A` + `git ls-files`), the project's own `.gitignore` byte-identical, `read_driver_run` reading `starting` / 0 launches / not crashed.
- **Refusals and goldens:** collision leaves bytes and mtimes unchanged with no suffixed sibling; slug edges (`""`, `-x`, `.x`, `a/b`, `a b`, non-ASCII, newline, bidi override, 72-byte) refused before any directory; planted symlinks at `.planning`, `meta-manager` and `drivers` refused with the outside dir left empty; stop-rules block equality; `parse_log(LOG_TEMPLATE)` is empty.
- **Lint:** every D-11 problem kind on its file line (frontmatter lines counted), self-edge and two-item cycles reported once, forward `after` valid, `${HOME}` / `{"a":1}` / `{a,b}` not placeholders, comments blanked, empty body reports both sections, `no items`, findings sorted by (line, message), gap-rounds > 2 warns only. `parse_direction` is untouched (no `(29-04)` commit touches `src/driver/direction.rs`).

## Task Commits

1. **Task 1 (tracer): driver new** - `3202234` (feat)
2. **Task 2: refusal and contract goldens** - `8c70266` (test)
3. **Task 3: lint + driver check** - RED `f19365b` (test), GREEN `6846b65` (feat)
4. **Fix (Task 1 regression found by the full suite)** - `7c682b7` (fix)

**Plan metadata:** the docs commit that carries this file.

## TDD Gate Compliance

- **Task 3:** RED `f19365b` (stub `lint_direction` / `template_placeholders`; 12 lint unit tests and 4 `driver check` e2e tests failed on their assertions, the rest were guard tests that a stub satisfies) then GREEN `6846b65` (all 24 lint/template unit tests and 13 e2e green). No REFACTOR commit (nothing to clean). `gsd_run check tdd-red-evidence` has no adapter for `cargo test` console output, so the RED was assessed semantically: each target test executed and failed on the planned assertion (empty report vs the expected findings; exit 2 from clap vs the expected 1/0), not on a compile or fixture fault.
- **Task 2 violation, disclosed:** no RED commit exists. The plan's Task 1 action already specified the whole `create_run_dir` refusal set and the full LOG grammar, so every Task 2 behaviour was green on introduction (unexpected GREEN, fail-fast rule 1). Investigated rather than ignored: the tests were mutation-checked (deleting the `auth-relaunch` grammar line fails the event-coverage golden; dropping the drivers symlink check fails the symlink e2e), so they bite. Committed as `test(29-04)` characterization of the contract; there is no GREEN `feat` commit for Task 2 because no production code changed.

## Decisions Made

See `key-decisions` above. The inferred ones (slug shape, separate `check` verb, gap-rounds warning) are marked `[inferred - audit]` for later review; none is a scope or irreversible fork.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `driver new` / `driver check` are Linux-gated, not platform-neutral**
- **Found during:** Task 1
- **Issue:** the plan says `new_main`/`check_main` are "not linux-gated" and the module "not platform-gated", but `create_run_dir` needs `driverctl::store::{frontmatter_for, RunState}` and `driverctl::serve::{valid_run_id, DRIVERS_REL, validate_run_dir}`, which are `#[cfg(target_os = "linux")]`. An ungated use would break the macOS build.
- **Fix:** `pub mod driver_templates;` and the templates, `lint` and `template_placeholders` are ungated; `mint_run_id`, `create_run_dir`, `NewRun`, `NewRunError`, `new_main` and `check_main` are Linux-only, with `UnsupportedPlatform` stubs in `driver_cli.rs` elsewhere (the launcher itself is Linux-only, so no capability is lost).
- **Files modified:** src/driver_templates/mod.rs, src/driver_cli.rs
- **Committed in:** 3202234, 6846b65

**2. [Rule 1 - Bug] Slug refusal message tripped the identity-alphabet census**
- **Found during:** final full-suite run
- **Issue:** my `BadSlug` message quoted the allowed punctuation as char literals; `text::tests::exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src` counted it as a second spelling of the alphabet.
- **Fix:** reworded to "dot, underscore and dash".
- **Verification:** that test and the 24 + 13 plan tests green; full suite back to the single expected failure.
- **Committed in:** 7c682b7

**3. [Rule 3 - Blocking] Hidden test-time guard on `RUN_ID_RE`/`DRIVERS_REL` visibility**
- `DRIVERS_REL` is `pub(crate)`, so the bin-side `check_main` uses the public `state_reader::driver_frontmatter::DRIVER_RUNS_REL` (documented as the same path) with `valid_run_id` + `validate_run_dir` doing the real validation. No behaviour change.

---

**Total deviations:** 3 auto-fixed (1 bug, 2 blocking)
**Impact on plan:** none on scope; the platform gating is the only contract difference and follows the repo's existing Linux-only launcher boundary.

## Issues Encountered

- A `git checkout -- <file>` I used to undo a deliberate mutation also reverted that file's then-uncommitted unit-test module; I re-added it verbatim and re-ran the suite. Nothing was lost and no commit was affected.
- The spike-003 filled body (`DIRECTION.run1.md`) lints clean except for its title line `# Driver direction: {run-id}` (an unfilled `{run-id}` in a hand-copied body). That is by design of the exact placeholder set; `driver new` always fills it.

## Verification

- `cargo test --no-fail-fast --test driver_new`: 13 passed. `--lib driver_templates`: 24 passed. `--lib cli::`: green incl. the `new` and `check` parse tests. `--test driver_prompt --test driverctl_store`: 14 and 47 passed. `--lib driver::direction`: 15 passed.
- Full suite: 3635 passed, 1 failed, 22 ignored across 84 targets; the single failure is the expected local `envelope::policy` git-version witness.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean.
- Acceptance greps: `^pub mod driver_templates;` 1, `create_new(true)` 1, `^- control: run$` 1, `^- human-verification: defer$` 1, `prompt=` and `stalled` in LOG.md, `pub fn lint_direction` 1, `DriverAction::Check` 1, zero `(29-04)` commits on `src/driver/direction.rs`.

## Known Stubs

None. The `{...}` spans in `assets/DIRECTION.md` are intentional placeholders the lint reports until the human fills them.

## Threat Flags

None beyond the plan's register. T-29-14 (symlink chain) and T-29-16 (clobber) are covered by e2e tests, T-29-15 (slug tricks) by unit and e2e tests, T-29-17 (staging) by the `git add -A` + `git ls-files` proof, T-29-18 (`check` mutating) by the bytes-and-mtime assertions.

## Next Phase Readiness

Plan 29-06 can call `driver_templates::lint::lint_direction` for the launch preflight (problems = refusal, warnings printed); plan 29-07 can parse the first `driver new` line and the two echoed commands.

## Self-Check: PASSED

All five created files exist and all five task commits (`3202234`, `8c70266`, `f19365b`, `6846b65`, `7c682b7`) are ancestors of HEAD.
