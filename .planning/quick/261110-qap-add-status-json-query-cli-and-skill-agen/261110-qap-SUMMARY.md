---
phase: quick-261110-qap
plan: 01
subsystem: cli
tags: [cli, json, agent-skill, read-only, status]
requires:
  - phase: 29
    provides: "next --json router hint (driver::router::decide)"
provides:
  - "gsd-meta-manager status --json [project...] (schema v1)"
  - "gsd-meta-manager --skill (agent skill markdown)"
  - "next_cli::hint_for, the shared default-target router hint"
affects: [cli, readme, agent-integrations]
tech-stack:
  added: []
  patterns: ["read-only query verbs dispatched before init_tracing", "SCHEMA_PATHS key-path collector applied to output, skill and README"]
key-files:
  created:
    - src/status_cli.rs
    - src/agent_skill.md
    - tests/status_cli.rs
  modified:
    - src/cli.rs
    - src/main.rs
    - src/next_cli.rs
    - README.md
decisions:
  - "D1-D10 from the plan, see Decisions (audit)"
metrics:
  completed: 2026-11-10
  tasks: 3
status: complete
commits: 4
plan_head_before: a67846f4168ebdc4543f039a9d576e1b1beec378
plan_head_after: c81159d5dae56fd07468c3dd030f9994d8bd726e
actuals:
  tokens: 16000
  tasks: 3
  commits: 4
---

# Phase quick-261110-qap Plan 01: status --json query CLI and --skill Summary

`gsd-meta-manager status --json [project...]` prints one schema-v1 JSON line (milestone, active phase, progress, attention items, live Claude/Codex sessions, and the same router hint `next --json` prints) for every registered project or the named ones. `gsd-meta-manager --skill` prints a SKILL.md-style document teaching a model to use `list`, `status --json`, `next --json` and jq.

## What was built

- **Task 1 (tracer)** `d90f6e6`: `Commands::Status` (`--json` required), dispatched in `main` before `init_tracing`; `next_cli::hint_for` extracted so `status` and `next` share one hint builder (`next` output unchanged, tests/next_cli.rs untouched and green); `status_cli` envelope plus the 11-key entry for a path target.
- **Task 2** `ef21571`: registry listing in `list` order (plain `load_config`, never the pruning loader), alias / path-in-root / registered-path resolution, `unresolved` / `folder_missing` / `no_planning` error entries, attention items from the root and every workstream, sessions attributed to the deepest root, exit 1 with one `status: ` line for an unparseable registry. The behaviour tests were written first and seen failing (9 of 16 red) before the implementation; they were committed together with it as a single feat commit.
- **Task 3** `e689e9b`, `c81159d`: `Cli.skill` flag, `src/agent_skill.md`, README "Scripting and agents" section (schema table, skeleton, exit codes, stability rule, jq recipes, install example), and drift guards pinning the output, the skill skeleton and the README skeleton to the same 44 `SCHEMA_PATHS`.

## Verification

- `rtk bypass cargo test --no-fail-fast`: 3750 passed, 0 failed, 19 ignored across 60 test binaries. The router-conformance oracle suite (`tests/driver_router_conformance.rs`) ran and passed (6 of 6) against the locally installed GSD oracle.
- `rtk bypass cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- `cargo package --list --allow-dirty` includes `src/agent_skill.md`; `git diff --stat 82cc1a8..HEAD -- src/multiplexer src/ui` is empty.
- Smoke against the real registry: `status --json | jq '.schema_version, (.projects|length)'` gives `1` and `17`; `--skill | head -2` gives the frontmatter.

## Decisions (audit)

All ten plan decisions were implemented as written:

- D1 [INFERRED — audit]: one compact JSON object on one stdout line, no timestamp.
- D2 [INFERRED — audit]: no targets reads the registry with plain `load_config` in `list_projects` order; stale entries surface as `folder_missing` instead of being pruned.
- D3 [INFERRED — audit]: only an unparseable registry (or unreadable cwd) fails the run (exit 1).
- D4 [INFERRED — audit]: a session belongs to the deepest registered-or-listed root that is its cwd or an ancestor of it.
- D5 [INFERRED — audit]: project-authored strings escaped with `render_for_terminal` and capped at 200 characters; `path` and `sessions[].cwd` emitted raw; prose never read.
- D6 [INFERRED — audit]: the seven-kind attention vocabulary, read from every scope.
- D7 [INFERRED — audit]: `phase` is `active_phase_number()`, null when neither STATE.md nor the disk frontier names a phase.
- D8 [INFERRED — audit]: error entries have null data keys and empty arrays; no sessions attributed.
- D9 [INFERRED — audit]: `--skill` is a top-level flag; text in `src/agent_skill.md` via `include_str!`.
- D10 [INFERRED — audit]: `--json` is required.

## Deviations from Plan

### Auto-fixed / inferred

**1. [INFERRED — audit] Sessions sorted by pid within an entry.** The plan did not fix an order for `projects[].sessions`; `detect_sessions` returns Claude then Codex in process-table order. Sorting by pid keeps the output deterministic (D1).

**2. [INFERRED — audit] Hermetic test runner.** The plan's `status(args)` helper would have read the developer's real registry once Task 2 made the path form consult it for the alias. The helper passes `--config` to a nonexistent scratch file (empty registry, nothing written) instead.

**3. [INFERRED — audit] Skill wording on `list`.** The plan's rules said everything in the skill is read-only. `list` uses the pruning loader and may save the registry (drops entries for vanished linked worktrees), so the skill and its description claim read-only only for `status` and `next` and note the `list` caveat. `list` stays in the skill as the plan required.

**4. RED and GREEN for Task 2 committed together** (the plan allows a single feat commit).

None of Rules 1-4 triggered: no bugs, missing critical functionality or blockers were found, and no dependencies were added (Cargo.toml and Cargo.lock unchanged).

## Known Stubs

None.

## Threat Flags

None. The new surface (a read-only CLI verb, `/proc` session metadata to stdout) is the one the plan's threat model covers (T-qap-01 to T-qap-06), and each mitigation has a test: injection, writes-nothing, SCHEMA_PATHS drift.

## Self-Check: PASSED

- FOUND: src/status_cli.rs, src/agent_skill.md, tests/status_cli.rs
- FOUND commits: d90f6e6, ef21571, e689e9b, c81159d (all ancestors of HEAD)
