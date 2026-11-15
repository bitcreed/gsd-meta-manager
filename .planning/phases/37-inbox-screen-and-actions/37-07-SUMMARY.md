---
phase: 37-inbox-screen-and-actions
plan: 07
subsystem: Owner inbox copy (y), clipboard, help overlay and README
tags: [inbox-screen, clipboard, osc52, help, readme, d-09, d-20, d-21]
requires: ["37-06"]
provides:
  - clipboard::{ClipboardProbes, Tool, pick_tool, probe, copy_with_tool, base64, osc52, MAX_COPY_BYTES}
  - inbox::{group_checklist, old_debt_checklist}
  - Action::ClipboardCopied, ScreenAction::WriteOsc52, App.pending_osc52, AppContext.{start_copy, start_copy_with}
  - owner_actions::{copy_text, old_debt_text, fold_copy, copy, line_count}, owner_block::groups_for_copy
  - help overlay Owner Inbox section and the Owner block v o A y row; README inbox-screen text
affects: []
key-files:
  created:
    - src/clipboard.rs
  modified:
    - src/inbox.rs
    - src/lib.rs
    - src/action.rs
    - src/app.rs
    - src/main.rs
    - src/ui/screens/{inbox,owner_actions,owner_block,mod,detail,help}.rs
    - tests/spawn_seam_guard.rs
    - README.md
key-decisions:
  - "D-09: y copies exactly the lines the CLI prints (one shared group_checklist); native tool through a bounded stdin child (2 s, stdout/stderr null, writer thread), else OSC 52 handed to the main loop and written after the frame"
  - "The target OS is data (ClipboardProbes.os), so no cfg(target_os) and no cross-target dead-code lint"
  - "Test seam in owner_actions (thread-local probes plus a recorder): no unit test reads the real environment or starts a clipboard tool; the default under cfg(test) is 'no tool' (OSC 52)"
requirements-completed: [OWN-20, OWN-23]
duration: 110 min
completed: 2026-11-15
status: complete
commits: 2
plan_head_before: b1f85e1c5d3e021fd6dfa5faa51d079d09b58aa2
plan_head_after: 4dfdadc152e3cabdd2b8c7dc665487e791d59428
actuals:
  tokens: 60000
  tasks: 2
  commits: 2
---

# Phase 37 Plan 07: Copy, help and README Summary

`y` puts the selected group's plain checklist (byte-identical to what `gsd-meta-manager inbox` prints) on the clipboard through wl-copy, xclip, xsel, pbcopy or clip.exe, else an OSC 52 sequence the main loop writes after the frame; the footer carries every UI-SPEC key, and the help overlay and README document the inbox, every key, the acknowledge argv and the clipboard paths.

## Accomplishments

- Task 1 (tracer, 3fcbcff7): `inbox::group_checklist` extracted from `push_section` (both owner text goldens untouched, `git rev-list` count still 2); `src/clipboard.rs` (pure `pick_tool`, `probe` as the only environment reader via a `split_paths` scan, `copy_with_argv` with a writer thread so a tool that never reads cannot block past its budget, RFC 4648 `base64`, `osc52`); `Action::ClipboardCopied`, `ScreenAction::WriteOsc52`, `App.pending_osc52` written by `main.rs` after `terminal.draw`; `y` on group rows, on the fold line (Old debt copies the `OLD DEBT` checklist; Acknowledged and Triage refuse with the UI-SPEC copy) and on the Owner block (D-21; BLOCKED aggregate copies every group of the phase, one heading each); footer rungs 1-3 and the fold-line footer exactly as the UI-SPEC; spawn allowlist entry.
- Task 2 (docs, 4dfdadc1): `Owner Inbox` help section with every key (ASCII key vocabulary), the Owner block `v o A y` row, whole-row and exactly-once assertions; README reading-is-free paragraph, the `Owner inbox` tab-specific row, the inbox-screen block (sections, ordering, keys, `v` commands, exact acknowledge argv, clipboard tools and `set -g set-clipboard on`), and the keyboard-only note in the Mouse paragraph. The dashboard `i` help and README rows from 36 already matched the UI-SPEC, and `hidden.planned_questions` is already in the schema table (37-01).

## Deviations from Plan

- [Signature] `copy_text(alias, groups: &[OwnerGroup])` takes the groups instead of `(ctx, alias, group)`: a BLOCKED aggregate needs several groups, and the lookup by identity stays in the callers (`owner_block::groups_for_copy`, new). `fold_copy` and `line_count` added as the shared fold-line and status helpers.
- [Added, Rule 2] `inbox::old_debt_checklist` (sorted and filtered like the CLI's `OLD DEBT` section, via `Filters::lists_group`) so Old debt copy cannot drift from the CLI.
- [Added, Rule 1] Two existing help tests encoded the 36 state and were updated, not weakened: `help_documents_the_owner_block_keys` no longer forbids the words acknowledge/copy for the block (it now forbids mouse, click, inject, pass, fail, native, i.e. things this phase does not build), and `the_help_screen_never_says_sent_received_or_acknowledged` (a driver-steering D-28 guard) strips the fold list `Old debt, Acknowledged and Triage` before its word check, since Acknowledged is a fold name.
- [Process] `cfg_attr(test, allow(dead_code))` on `AppContext::start_copy`: tests go through the `owner_actions` seam, so the wrapper is unused under test.
- [Inferred] The 2 s tool budget (RESEARCH A7); `Select a group row to copy its checklist` on the Owner block fold line; `Nothing to copy: no old debt is listed` and `...the group is no longer listed` for the empty cases; `Copy failed: this session has no event channel`; the key column text `j/k Up/Down` and `Left / Right` for the UI-SPEC arrows (glyph scan).
- [Inferred] README adds one sentence under the Project detail paragraph saying `v o A y` work on a group row of the Owner block as in the inbox, and the argv sentence notes `gsd-tools` runs directly when it is on `PATH` (matches `resolve_gsd_tools`).
- A live clipboard run is not a test (UI-SPEC records the paths as automated with fake tools): `sh` scripts stand in for tools (forking, exit code, never exits, never reads), run as `sh script` so no ETXTBSY race.

## Divergences from CONTEXT

(Repeated for the verifier, as this is the phase's last plan.)

- D-17 gate source: as 37-01 recorded it (see 37-01-SUMMARY).
- Never-loading resolution: as 37-04 resolved it (see 37-04-SUMMARY).
- In-app `o` viewer: `o` opens the read-only in-app viewer (37-05), not an editor.
- `g` pushes the detail view itself instead of going through `App::open_project_view`, and re-selects the project's dashboard row when it is listed there (37-05, RESEARCH Pitfall 6). D-01's "selection unchanged" therefore holds only for `i`/`Esc` without `g`.
- Last-stderr-line and active-workstream refusal (37-06): acknowledge failure copy uses the last non-empty stderr line (UI-SPEC says first), and a root group is refused while `.planning/active-workstream` is non-empty.
- Follow-ups carried: the dashboard counts a failed project as loading; the screen and the CLI order TEST rows differently in a tie case.
- This plan: the OSC 52 status says the sequence was sent and the terminal must allow it (never `Copied`); no silent OSC 52 retry after a native tool failed.

## Follow-ups

- Human UAT (D-23, in /gsd-verify-work): ordering, a real multiplexer launch, the acknowledge round trip. A live clipboard check is not one of them. `./scripts/pre-tag-check.sh --container` was not run (milestone tagging).

## Verification

- `cargo test --locked --no-fail-fast --all-targets`: 4394 passed, 0 failed, 19 ignored.
- `cargo clippy --all-targets -- -D warnings`: clean on the host, `x86_64-pc-windows-gnu`, `aarch64-apple-darwin` and `x86_64-apple-darwin` (checked after Task 1; Task 2 touches only help.rs and README).
- Owner text goldens and the Owner block pin goldens untouched.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: src/clipboard.rs
- FOUND: 3fcbcff7, 4dfdadc1 (ancestors of HEAD)
