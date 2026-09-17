---
quick_id: 260917-hc3
phase: quick-260917-hc3
plan: 01
subsystem: cli
status: complete
tags: [cli, clap, help, driver, documentation]
requires: []
provides:
  - "`drive` absent from rendered `--help` (short and long form) while fully parseable"
affects:
  - src/cli.rs
  - docs/CONFIGURATION.md
tech-stack:
  added: []
  patterns:
    - "clap `#[command(hide = true)]` for a subcommand another process re-enters — never `#[cfg(debug_assertions)]`"
    - "every absence assertion carries a positive control in the same rendered string, consuming the same predicate"
key-files:
  created: []
  modified:
    - src/cli.rs
    - docs/CONFIGURATION.md
decisions:
  - "Hidden, not gated: `hide = true` is help visibility only; the TUI's `current_exe() drive` respawn is the caller that must keep working"
  - "The positive control and the absence assertion call ONE `is_subcommand_entry` helper, so the control certifies the same code path the assertion runs"
  - "The born-green regression pin (test 2) was written anyway — it guards the half of the change that can be broken silently"
metrics:
  duration: ~15 min
  completed: 2026-09-17
  tasks: 2
  files: 2
actuals:
  tokens: 1828
  tasks: 2
  commits: 2
plan_head_before: 1abfdd4ede722ff518351d7ed976a6bddbb1aba0
---

# Quick Task 260917-hc3: Hide the `drive` Subcommand from `--help` Summary

`Commands::Drive` now carries `#[command(hide = true)]` — it is gone from both rendered help
forms and unchanged everywhere else: it still parses, still dispatches, is not `#[cfg]`-gated,
and is not conditioned on `GSDMM_EXPERIMENTAL_FEATURES`.

## What Was Built

**Task 1 — `src/cli.rs` (commit `7ca2e94`)**

Two tests in a new `#[cfg(test)] mod tests` at the bottom of the file, written and run BEFORE
the variant was touched:

| Test | Starting colour | After the attribute |
|------|-----------------|---------------------|
| `the_drive_subcommand_is_absent_from_rendered_help` | **RED** | GREEN |
| `the_drive_subcommand_still_parses_and_resolves` | **GREEN** | GREEN |

Test 1's RED was the intended one, not an accident of spelling — it reported the actual
offending line, `  drive   Run one GSD command against a project that has opted in to being
driven`, from `render_help`. It asserts over BOTH `render_help()` and `render_long_help()`,
because `hide` covers both and asserting one would leave a short/long regression undetected.
A **positive control** — a subcommand entry for the visible `list` command — is asserted in the
same rendered string and BEFORE the absence, so the absence cannot pass against an empty or
unrendered help. Control and absence call one `is_subcommand_entry(line, name)` predicate
(entry shape: trimmed line starts with the token followed by ASCII whitespace or end-of-line),
so the control certifies the path the assertion runs rather than a second matcher that could
disagree with it.

**Why test 2 was written even though it was born green.** It is a regression pin over the half
of this task that fails silently: the TUI respawns itself as `current_exe() drive …` into a
child whose stdio is `/dev/null`. A change that severed the parser would show the user
"Driving {alias}" and no run — nothing to see, nothing to read. A green-before/green-after pin
is what converts "we were careful" into an enforced property. It asserts the parse resolves to
`Commands::Drive` with `alias == "demo"` and `command == Some("/gsd-progress")`, so it proves
the subcommand RESOLVED with its operands, not merely that some parse succeeded.

The attribute itself sits between the doc comment and `Drive {`, spelled exactly as
`Commands::Envelope` spells it thirty lines down. The rationale went into `//` comments only
(per the convention at `src/cli.rs:39-41` — every `///` is rendered verbatim by clap), covering
the three things a later reader would otherwise reconstruct: what `hide` does (help only,
pointing at the `Envelope` block for the general rule), why not more (260917-fko's deliberate
exemption and the invisible-breakage failure mode), and why this is not the `--claude-program`
case (that capability is shaped like arbitrary code execution; this one's authorisation
boundary is the per-project `driver_opt_in` record, never help visibility).

**Task 2 — `docs/CONFIGURATION.md` (commit `3ff42d4`)**

The "not affected by `GSDMM_EXPERIMENTAL_FEATURES`" bullet was extended, not replaced — its
existing sentences stay, because they stay true. Added: it is now `hide = true` and absent from
`--help`; hidden rather than gated because the TUI's own respawn is the caller that must keep
working; that this is visibility only, identical in both flag states, and that **the flag is not
what hides it** (both states render the same help). The other two bullets and the environment
variable table were not touched.

## Spelling Constraints Honoured

The three live scanners in `tests/spawn_seam_guard.rs` walk `src/` without stripping test
modules, so the new module was spelled around them:

- No line trims to `alias: String,` / `alias: Option<String>,` — the match uses struct shorthand
  `Commands::Drive { alias, command, .. }`. The census still measures exactly 8.
- No line begins `claude_program:` or `claude_args:`.
- `    Drive {` stays on its own line; the attribute was added above it.

All `tests/spawn_seam_guard.rs` guards are green, as is
`src/ui/screens/driver_start::tests::a_hyphen_led_goal_reaches_the_child_parser_as_a_goal` — the
pre-existing producer/consumer round-trip over the spawn argv.

## Verification

| Gate | Result |
|------|--------|
| `cargo build` | exit 0 |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo test --no-fail-fast` | **2120 passed / 1 failed / 15 ignored** |

Baseline (260917-fko tree, same session convention): **2118 passed / 1 failed**. Measured delta
is exactly **+2 tests, no new failure**. The failing SET is identical to the baseline's — the
single known-environmental `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
(installed git 2.53 vs constants derived against 2.43). No `driver::run` flake occurred this run.

Other done criteria:
- `grep -c '^    #\[command(hide = true)\]' src/cli.rs` → `2` (the `Envelope` variant's and the new one).
- `git diff --name-only 1abfdd4..HEAD` → exactly `src/cli.rs` and `docs/CONFIGURATION.md`.
  `src/driver/spawn.rs`, `src/main.rs`, the envelope env scrub and every
  `GSDMM_EXPERIMENTAL_FEATURES` site are untouched.
- Post-commit deletion check clean on both commits.

## Deviations from Plan

None — the plan executed exactly as written. No deviation rule fired.

## Coordinator inferred decisions (for audit)

1. **Ran unisolated on the primary checkout (`dev`), no worktree** — per the dispatch
   instruction. The #1941/#48 worktree base-check auto-degrades on this repo (origin/HEAD
   `89fdea87` is stale against HEAD and `worktree.baseRef:"head"` is not honoured by the
   harness), so forking a worktree would have based the work on a stale tree. Same treatment
   260916-vqz and 260917-fko recorded.
2. **The pre-existing dirty file
   `.planning/quick/260915-f4n-…/260915-f4n-SUMMARY.md` was left untouched and unstaged.** It
   was already modified in the working tree before this task started and is unrelated to it;
   both commits staged named files individually.
3. **`docs/CONFIGURATION.md` was committed with the code rather than left for the orchestrator's
   docs commit.** It is repository documentation under `docs/`, not a `.planning/` artifact, and
   the plan requires it to change in the same change that makes it incomplete (D-04). The
   `.planning/` artifacts (this SUMMARY, STATE.md, PLAN.md) are left uncommitted for the
   orchestrator.
4. **`cargo clippy --all-targets` was not run.** The dispatch records 4 pre-existing lint errors
   there and scopes the gate to the plain lib-target `cargo clippy -- -D warnings`, which is
   green.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path, file access pattern or schema change; `Cargo.toml` is
unmodified and no dependency was added. T-hc3-02 (the respawn path) is mitigated structurally —
`hide` touches help only — and pinned by test 2 plus the pre-existing round-trip. T-hc3-03 (the
source-text guards) is discharged by the three spelling constraints above and by the full
`--no-fail-fast` run.

## Self-Check: PASSED

- `src/cli.rs` — FOUND (modified, `#[command(hide = true)]` present on `Commands::Drive`)
- `docs/CONFIGURATION.md` — FOUND (modified)
- Commit `7ca2e94` — FOUND
- Commit `3ff42d4` — FOUND
