---
quick_id: 261016-dx4
phase: quick-261016-dx4
plan: 01
subsystem: executor, state_reader, ui
tags: [gsd-core-1.17.0, slash-form, compat-rewrite]
status: complete
requirements: [QUICK-261016-dx4]
key-files:
  modified:
    - src/executor/runtime.rs
    - src/executor/claude.rs
    - tests/executor_transport.rs
    - src/state_reader/queue_md.rs
    - src/ui/screens/detail.rs
    - docs/GETTING-STARTED.md
actuals:
  tokens: 30000
  tasks: 3
  commits: 3
plan_head_before: a254c32
plan_head_after: 21085da
commits_list:
  - 65b7e3b  # Task 1 (tracer)
  - 657d032  # Task 2
  - 21085da  # Task 3
tests:
  total_passed: 2988
  total_failed: 1   # envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against (expected on this machine)
  ignored: 16
  clippy: clean (--all-targets -D warnings)
---

# Quick 261016-dx4: emit /gsd-name instead of /gsd:name

Every command the app emits now names the registered `/gsd-<name>` form, and colon-form commands still in QUEUE.md reach Claude as `/gsd-<name>` through a send-time rewrite.

## Commits

| Task | Commit | Summary |
|------|--------|---------|
| 1 (tracer) | 65b7e3b | `runtime::claude_command` plus a shared prefix splitter. `codex_command` is rebuilt on it with identical output. `ClaudeExecutor::start_run` translates the first message. The stdin-wire integration test passes. |
| 2 | 657d032 | Keyword suggestions are hyphen-form. `smart_entry_command` pins `GSD_RUNTIME=claude`. Docs and fixtures updated. |
| 3 | 21085da | Backlog `e` prefill, No-plans hint, Verify remedies and the `GETTING-STARTED.md` prose use the hyphen form. Backlog `e` test added. |

## Verification

- `cargo build` is clean. `cargo clippy --all-targets -- -D warnings` is clean.
- `rtk proxy cargo test --no-fail-fast`: 2988 passed, 1 failed, 16 ignored across all suites. The lib suite was 2104 passed, 1 failed, 1 ignored.
- The one failure is the expected `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- Task-level gates ran and passed: 4 runtime tests, 10 `executor_transport` tests, 28 `queue_md` tests and the 3 named `detail` tests.
- No `/gsd-core:` plugin-form string was added (I-6).

## Residual `/gsd:` grep in src (classification)

- **`GSD_COMMAND_PREFIXES` and its tests, plus colon input rows in the `claude_command` / `codex_command` tests**: `executor/runtime.rs`.
- **Colon-absence assert**: `detail.rs:26167` in the Verify test.
- **User-data, journal and queue fixtures (I-4)**: all remaining hits in `queue_md.rs` tests, `journal/*`, `driver/mod.rs`, `ui/mod.rs`, `ui/screens/driver*.rs`, `driver_start.rs`, `driver_confirm.rs`, `queue_delete_confirm.rs`, `text.rs`, `config_json.rs`, `app.rs` tests, and `detail.rs` fixtures (`gsd_command: "/gsd:progress"`, `"/gsd:execute-phase 18"`, `/gsd:task-NN`).
- **I-5 prose comments**: `state_md.rs`, `state_reader/mod.rs`, `app.rs:362`, `driver/dry_run.rs:6` (a quoted design remark in a module doc). No production emission site remains.

## Deviations from Plan

None. The plan was executed as written.

## Inferred decisions (audit)

- **I-1 [inferred]: send-time compat rewrite for Claude.** `/gsd:<name>`, `gsd:<name>` and `$gsd-<name>` are rewritten to `/gsd-<name>` in `runtime::claude_command`. It is applied only in `ClaudeExecutor::start_run`, mirroring Codex's single translation point. `send()` is not rewritten, because steering text is prose and replay-echo `PendingAcks` correlation matches it verbatim. The journal keeps the command as given.
- **I-2 [inferred]: `claude_command` mirrors upstream `formatGsdSlash`'s slash branch.** It accepts all four prefixes ASCII case-insensitively, keeps the token's case and keeps the argument tail byte for byte. Prose, the empty string and a prefix with no token pass through verbatim.
- **I-3 [inferred]: pin `GSD_RUNTIME=claude` on the `smart-entry` child.** Since 1.17.0, smart-entry spells actions through `resolveRuntime(cwd)`. A Codex-resolved install would otherwise answer `$gsd-…`, which breaks the canonical comparisons. The pin uses `Command::env` on that child only. Older installs answer in the colon form, which the dispatch rewrites repair.
- **I-4 [inferred]: keep colon-form test fixtures that model user-authored or recorded data.** They guard acceptance of that data.
- **I-5 [inferred]: out of scope and left unchanged.** Prose doc comments (`state_md.rs`, `state_reader/mod.rs`, `app.rs`), `docs/DEVELOPMENT.md:205,217-218` and `README.md` (no colon-form mention).
- **I-6: no `/gsd-core:gsd-<name>` plugin-form support.**
- **I-7 [inferred]: update the smart-entry JSON fixtures in `queue_md` tests to the 1.17.0 `/gsd-…` shape.** Parsing is pass-through, so behaviour is unchanged.

## Follow-up candidates

- `docs/DEVELOPMENT.md:205,217-218` still tells contributors to type the colon form (`/gsd:quick` etc.), which no install registers (I-5).

## Known Stubs

None.

## Self-Check: PASSED

Commits 65b7e3b, 657d032 and 21085da are in `git log`. Every modified file exists.
