---
phase: 37-inbox-screen-and-actions
plan: 06
subsystem: Owner inbox acknowledge (A, confirm popup, gsd-core writer)
tags: [inbox-screen, acknowledge, confirm-popup, owner-actions, d-14, d-15, d-16, d-21]
requires: ["37-05"]
provides:
  - owner_actions::{ACK_VERB, ACK_ENV_REMOVE, AckTarget, ack_target_for, ack_args, AckRequest, ack_request, ack_open, ack_recheck, ack_outcome, run_ack}
  - ack_confirm::{AckConfirmScreen, AckHost, ack_body}
  - launcher::process::run_bounded_with_stderr_env
  - Action::OwnerAcknowledged and AppContext.{owner_ack_busy, start_owner_acknowledge, start_owner_acknowledge_with}
  - queue_md::{GsdToolsCmd, resolve_gsd_tools} at pub(crate)
  - tests/owner_acknowledge_roundtrip.rs (oracle-gated)
affects: [37-07]
key-files:
  created:
    - src/ui/screens/ack_confirm.rs
    - tests/owner_acknowledge_roundtrip.rs
  modified:
    - src/ui/screens/owner_actions.rs
    - src/ui/screens/inbox.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/render_escape_guard.rs
    - src/action.rs
    - src/app.rs
    - src/launcher/process.rs
    - src/state_reader/queue_md.rs
    - tests/owner_open_conformance.rs
    - tests/spawn_seam_guard.rs
    - src/ui/screens/{delete_confirm,driver_confirm,normal}.rs (AppContext initializer only)
key-decisions:
  - "D-14: the verb pair is spelled once, in ACK_VERB; every argv and expectation is built from it"
  - "D-15: the child is a bounded direct child (10 s, cwd = project) with ACK_ENV_REMOVE stripped; a root group is refused while .planning/active-workstream is non-empty (inferred, RESEARCH Pitfall 2)"
  - "D-16: failure copy uses the LAST non-empty stderr line (inferred, RESEARCH Pitfall 7), diverging from the UI-SPEC's 'first line'"
  - "Yes re-resolves the group by identity and re-derives the target; any difference but the item count refuses (T-37-20)"
requirements-completed: [OWN-22]
duration: 150 min
completed: 2026-11-15
status: complete
commits: 1
plan_head_before: 4804aec61ada699f2f33fee90e7d03478d8e770a
plan_head_after: 3982eaa6
actuals:
  tokens: 90000
  tasks: 2
  commits: 1
---

# Phase 37 Plan 06: Acknowledge through gsd-core Summary

`A` on an inbox row (or an Owner block row) validates the group, shows a Yellow, default-No popup with the exact gsd-core argv and its consequences, and only on Yes runs gsd-core's own acknowledge writer as a bounded, environment-scrubbed child; the app itself writes nothing.

## Accomplishments

- Task 1 (tracer): `ack_target_for` / `ack_args` build the argv for `uat_gaps`, `verification_gaps`, `context_questions` and `debug_sessions` (with `--archived-milestone` for Old debt and `--ws` for workstream groups, `--phase` as the directory token). `run_bounded_with_stderr_env` removes `ACK_ENV_REMOVE` before `extra_env`. `AckConfirmScreen` over the inbox, `Action::OwnerAcknowledged` and its handler (escaped status, busy cleared), `[A]cknowledge` in the footer ladder, the narrowed verb guard, the spawn-allowlist reason, and the oracle round trip (real `gsd-tools` 1.21.0 on temp copies: group hidden, `hidden.acknowledged_files` +1, `--ws` for a workstream group).
- Task 2: every refusal as an exact status line (handoff, checkpoint, deferred, BLOCKED, already acknowledged, unknown milestone, unsafe phase/workstream/file/label, root group under an active workstream, no launcher, no `node`), vanished and busy rules, the popup at 80x24 (every line and the argv) and 60x20 (clipped, `y`/Enter-on-Yes inert, `n`/Esc cancel), the Owner block's `A` (D-21), hostile census states and a 80x24/60x20 raw-control-cell test.

## Deviations from Plan

- [Process] Both tasks landed in one commit (3982eaa6): they share hunks in owner_actions.rs, inbox.rs and the census file, and the AppContext initializer edits in detail.rs are needed for the crate to compile at all, so a path-scoped split could not give a green first commit (same call as 37-05).
- [Signature] `ack_target_for` and `ack_recheck` take `alias` (the refusal copy names it); `ack_body(req)` drops the unused `ctx` parameter; `AckRequest.command_line()` returns the program plus argv for the popup.
- [Added, Rule 2] `owner_actions::ack_open` is the one shared `A` entry for the inbox and the Owner block (request, busy check, push); `ack_recheck` makes Yes re-derive the target (not only the group identity) so the argv shown is the argv run.
- [Added, Rule 2] `is_flag_value`: milestone, label, file name and slug values must be identity-alphabet and not start with `-`, so no value can be parsed as a flag (CWE-88).
- [Added, Rule 3] Windows: `launcher::process` is Linux/macOS only, so `spawn_ack` has a non-unix twin that fails visibly (`Acknowledge failed: running gsd-core is not supported on this platform`); clippy `-D warnings` is clean for `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`.
- [Added] Test seam `force_launcher` (thread-local, `#[cfg(test)]`) so no unit test depends on this machine's gsd-tools install; the oracle round trip stays the only test that runs the real writer.
- [Inferred] Copy: `Cannot acknowledge deferred items: gsd-core has no category for it`, `Cannot acknowledge: {alias} is not a registered project`, `...has no owner data yet`, `The file changed since the popup opened; nothing acknowledged`, `Select a group row to acknowledge it`, `Refused: {v} is not a safe {file name|milestone version|milestone label}; nothing started`.
- [Inferred] Footer: at 100 columns the long rung is 107 cells, so the short rung shows; ladder test widths moved (107, 106, 84, 65) and the 100x30 assertion now expects `[v]run`/`[A]cknowledge`.
- [Inferred] An unreadable `.planning/active-workstream` counts as set (refusal is the safe answer).
- The third round-trip test (`a_root_group_is_refused_while_an_active_workstream_is_set`) needs no oracle (the refusal precedes any run), so it also runs where the oracle is absent.

## Divergences from CONTEXT

- Failure copy shows the **last** non-empty stderr line, not the first (gsd-tools prints a `.gsd/defaults.json` warning first on this machine; RESEARCH Pitfall 7). The full capped output goes to the tracing log.
- A root-scoped group is refused while `.planning/active-workstream` is non-empty: `Cannot acknowledge {file}: {alias} has an active workstream set; run it in the project` (RESEARCH Pitfall 2; gsd-core has no spelling for "the root"). The child also runs with `GSD_WORKSTREAM`, `GSD_PROJECT` and the session-key variables removed.

## Follow-ups

- 37-07: help overlay entries and README for `A`, `[y]copy`.
- Human UAT (D-23 item 3, in /gsd-verify-work): on a throwaway project, `A` shows the popup with the exact argv, Yes writes `audit_acknowledged`, leaves the file uncommitted, and the row disappears without a restart. `./scripts/pre-tag-check.sh --container` was not run (milestone tagging).

## Verification

- `cargo test --locked --no-fail-fast --all-targets`: 4373 passed, 0 failed, 19 ignored; no SKIP (oracle present).
- `cargo clippy --all-targets -- -D warnings`: clean on the host, `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`.
- Owner block pin goldens untouched and green.

## Known Stubs

None.

## Self-Check: PASSED
