---
phase: 37-inbox-screen-and-actions
verified: 2026-11-15T09:00:00Z
status: human_needed
score: 4/4 must-haves verified
covered_files:
  - ".planning/phases/37-inbox-screen-and-actions/37-01-PLAN.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-01-SUMMARY.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-02-PLAN.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-02-SUMMARY.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-03-PLAN.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-03-SUMMARY.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-04-PLAN.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-04-SUMMARY.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-05-PLAN.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-05-SUMMARY.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-06-PLAN.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-06-SUMMARY.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-07-PLAN.md"
  - ".planning/phases/37-inbox-screen-and-actions/37-07-SUMMARY.md"
  - "README.md"
  - "src/action.rs"
  - "src/agent_skill.md"
  - "src/app.rs"
  - "src/bounded_output.rs"
  - "src/clipboard.rs"
  - "src/inbox.rs"
  - "src/launcher/process.rs"
  - "src/lib.rs"
  - "src/main.rs"
  - "src/multiplexer/mod.rs"
  - "src/state_reader/owner_items/context.rs"
  - "src/state_reader/owner_items/gate.rs"
  - "src/state_reader/owner_items/mod.rs"
  - "src/state_reader/owner_items/rules.rs"
  - "src/state_reader/queue_md.rs"
  - "src/ui/screens/ack_confirm.rs"
  - "src/ui/screens/delete_confirm.rs"
  - "src/ui/screens/detail.rs"
  - "src/ui/screens/driver_confirm.rs"
  - "src/ui/screens/help.rs"
  - "src/ui/screens/inbox.rs"
  - "src/ui/screens/mod.rs"
  - "src/ui/screens/normal.rs"
  - "src/ui/screens/owner.rs"
  - "src/ui/screens/owner_actions.rs"
  - "src/ui/screens/owner_block.rs"
  - "src/ui/screens/owner_file_view.rs"
  - "src/ui/screens/render_escape_guard.rs"
  - "tests/focus_entry_guard.rs"
  - "tests/inbox_cli.rs"
  - "tests/owner_acknowledge_roundtrip.rs"
  - "tests/owner_digest.rs"
  - "tests/owner_open_conformance.rs"
  - "tests/spawn_seam_guard.rs"
covered_digest: "v4:sha256:ec66bbec165886cc3f067b84cae72165a34962005b83e317daa58a556d91b692"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Ordering and legibility on the real fleet. Open the inbox (`i`) at 120 columns, then at 100, in your real terminal font and theme."
    expected: "Sections run TEST (closes a phase), TEST (phase stays open), DECIDE, FIX. Inside TEST the projects closest to a milestone close come first, then the oldest. DECIDE and FIX list the oldest first. No column is clipped, `stale` is readable against your theme, and the `·` and `—` separators leave the checks column aligned (they are East-Asian-ambiguous). The order should feel right for triage."
    why_human: "Ordering feel, color in a real theme and glyph width on the owner's font are subjective or environment-bound. The 37-UI-REVIEW listed lane ordering (D-02) as not changed and left it to UAT."
  - test: "`v` into a real multiplexer. From the inbox press `j`/`v` on a TEST row with no live session, under tmux and again under herdr. Then repeat with a live session at that project."
    expected: "A new window or pane opens in the project directory and runs `claude /gsd-verify-work N`. With a live session at exactly that path, that session is focused and the status line reads `Switched to ... · type /gsd-verify-work N`; nothing is typed into it. Check that the new session behaves normally despite `env -u` removing `TMUX_PANE` and the other session variables from its environment."
    why_human: "The launch is only tested through recording closures and argv pins. No test starts a real tmux or herdr, and the `env -u TMUX_PANE ...` wrapper (added after the review as IN-02) has not been seen in a real multiplexer."
  - test: "`A` on a throwaway copy of a project (never a real one). Press `A` on a UAT row, read the popup, press Esc/n once, then `A` and Yes."
    expected: "The popup is Yellow, defaults to No, and names the project, file, category, milestone, 'file-level', 'lapses when the file changes', 'commits nothing' and the exact argv. Esc/n starts nothing. Yes writes `audit_acknowledged` into that one file's frontmatter via gsd-core, leaves it uncommitted (`git status` shows the modification), and the row leaves the inbox on the next watcher refresh without a restart. The status line `Acknowledged ... via gsd-core (uncommitted)` stays about 12 s."
    why_human: "The oracle round-trip test uses temp copies of fixtures. It does not exercise a live watcher over a real project tree or the owner's real gsd-core install and PATH. Real projects must not be written by the verifier."
  - test: "`y` clipboard on the real desktop and over OSC 52. Press `y` on a group row, then paste. Repeat with no native tool on PATH (or over ssh/tmux with `set -g set-clipboard on`)."
    expected: "The pasted text equals what `gsd-meta-manager inbox` prints for that group. The status line says `Copied N lines via {tool}`, or says `Sent N lines to the terminal clipboard (OSC 52; the terminal must allow it)`. The terminal shows no stray escape output."
    why_human: "No test starts a real clipboard tool or lets a real terminal accept OSC 52. Both paths are tested with recorded seams only."
  - test: "Held-key and paste safety for `v`. Hold `v` on a row, or paste text containing `v` into the terminal, with the inbox open."
    expected: "Decide whether launching one session per key repeat is acceptable. 37-REVIEW-FIX applied only the Ctrl/Alt guard (WR-01). It did not add bracketed-paste handling or a per-group cooldown, and `v` has no confirm by design (SC1)."
    why_human: "This is a product-risk judgment on a residual the review explicitly left open. It is not a goal failure."
---

# Phase 37: Inbox screen and actions Verification Report

**Phase Goal:** The owner triages all owner work across projects on one screen and acts on it in a few keys: run the group's GSD command in the right project, acknowledge through gsd-core, or copy the checklist, with the app itself writing nothing into any project.
**Verified:** 2026-11-15
**Status:** human_needed
**Re-verification:** No, initial verification
**HEAD:** 72bd2da14297c71aee2567097bdaf414691e74bb (branch feat/v2.5.0-owner-inbox, clean tree)

All automatable checks pass and the code matches the goal. Five items need a human: launch into a real multiplexer, a real acknowledge round trip, the real clipboard, ordering feel and legibility, and the held-key residual. These are the three ROADMAP human UAT checks plus clipboard and paste.

## Goal Achievement

### Observable Truths (ROADMAP success criteria are the contract)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| SC1 | From the dashboard `i`, `j`, `v` starts `/gsd-verify-work N` in the right project in 3 keys or fewer, through `multiplexer::launch` (focusing a live session), with argv built only from the validated phase token and workstream name via `ExecArgv::new` | VERIFIED (behavioral) | `normal.rs:810` pushes `InboxScreen::new()` on `i`. `owner_actions.rs:60-131` (`run_plan`, `launch_real`) builds from the typed `group.run` only and re-checks `is_command_token` and `is_workstream_name`. A live session is matched by exact `working_dir == project.path` and only `focus_message` is called, so nothing is injected. `detail.rs:3085` `owner_mux_argv` ends in `ExecArgv::new`. Test `sc1_i_j_v_launches_verify_work_in_the_project` passed in my own run, with the focus-or-launch, sibling-prefix, encoding (`２４`, `1;rm`) and refusal tests in the 22 `owner_actions` and 45 `inbox` tests. |
| SC2 | The inbox screen is legible at 100x30, filters lanes with `1-4`/`a`, follows the ordering rule, expands groups, and supports `g`, `o` and `y` | VERIFIED (automated); legibility feel and ordering feel go to a human | `inbox.rs:99-103` lane keys, key arms at `:1731-1735`. `sort_groups` (`:265`) orders sections TestCloses, TestOpen, Decide, Fix. Inside TEST it sorts by progress ratio by cross-multiplication, then fewer open phases, then age oldest first, then alias, phase key, kind, file, workstream, so the order is total. Tests: `sc2_legible_at_100x30`, `the_inbox_renders_sections_in_order_at_100x30`, `sort_groups_follows_d05`, `sort_groups_breaks_every_tie_the_same_way`, `degrade_never_panics`, `quiet_color_at_100x30`, `footer_ladder_matches_the_ui_spec`; 45 inbox tests pass. `o` reads only inside the canonical `.planning/` through one handle with a 1 MiB cap and starts no program. |
| SC3 | `A` launches `gsd-tools audit-open acknowledge ...` only after a confirm popup; the row disappears on the next watcher refresh without a restart; after a UAT passes, its row likewise disappears | VERIFIED (behavioral) | `ACK_VERB` is spelled once (`owner_actions.rs:228`). `ack_confirm.rs` shows the popup and has `enter_defaults_to_no`. `run_ack` is a bounded 10 s child with `ACK_ENV_REMOVE` stripped. `tests/owner_acknowledge_roundtrip.rs` (3 passed, `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` unset, so the real gsd-core 1.21.0 ran on temp copies) shows the group hidden and `hidden.acknowledged_files` +1. `app.rs` tests `sc3_a_passed_uat_leaves_the_inbox_and_the_cursor_moves_on`, `sc3_the_cursor_falls_back_to_the_previous_row_when_the_last_one_passes` and `sc3_an_acknowledged_file_leaves_the_inbox` drive real watcher refreshes of temp fixture copies. |
| SC4 | The help overlay and README ("Owner inbox" section plus the `inbox` schema) describe every key and the CLI contract | VERIFIED | `help.rs:532` "Owner Inbox" section (`?` from the inbox opens at it); 35 help tests pass. README `### Owner inbox` (`:369`) has the key table, `v` commands, the exact acknowledge argv, the launcher source, the clipboard paths and `set -g set-clipboard on`. `inbox_cli` (25 tests) pins `summary.planned_questions` and `hidden.planned_questions` against the README fence and table. Every documented key exists in the key match. |
| G | The app itself writes nothing into any project | VERIFIED | A scan of the non-test portions of `inbox.rs`, `owner_actions.rs`, `ack_confirm.rs` and `owner_file_view.rs` shows no `fs::write`, `File::create`, `OpenOptions`, `remove_file`, `create_dir`, `rename`, `Command::new` or `git commit`. The one spawn is `clipboard.rs:148`, which has a spawn-allowlist entry and only gets text on stdin. The acknowledge writer is gsd-core's, and `owner_open_conformance` and `spawn_seam_guard` pass. |

**Score:** 4/4 ROADMAP success criteria verified (plus the goal's no-write clause); 0 behavior-unverified truths. Each behavior-dependent truth (focus-not-inject, cursor re-anchor on vanish, acknowledge round trip) has a named passing test. Real-world execution against a multiplexer, a clipboard and the owner's fleet is routed to human verification as a separate class.

### Plan-level truths checked beyond the SCs

| Plan | Truth | Status | Evidence |
|------|-------|--------|----------|
| 37-01 | D-17 planned-question gate, `hidden.planned_questions`, D-18 entry counting, schema v1 unchanged | VERIFIED | `owner_digest` (30), `inbox_cli` (25), `owner_audit_conformance` (3) and `owner_open_conformance` (4) pass on all fixtures including `context-planned`, `context-summary-only` and `context-entries`. |
| 37-02 | Typed `RunCommand`; `command == run.to_string()` on every fixture; Owner block byte-identity pin | VERIFIED | `run_command_matches_the_command_string_in_every_fixture` and `owner_block_output_is_pinned` pass (28 owner_block tests). The three goldens were written once. |
| 37-03 | `v` D-12 focus-only; D-21 `v` on the Owner block | VERIFIED | `focus_status` and `launch_real` call only `focus_message`. `focus_entry_guard` (3) passes. |
| 37-04 | Loading versus unreadable, `OwnerDigestFailed`, lane filters, ladders | VERIFIED | `a_failed_owner_digest_reads_unreadable_until_one_arrives` and `unregistering_prunes_the_failed_set` pass. |
| 37-05 | `o` viewer, expansion, folds, scroll, `g` | VERIFIED | 5 `owner_file_view` tests, the contained `read_owner_file` and `read_capped_stops_one_byte_past_the_cap`. |
| 37-06 | `A` argv, env scrub, root-under-active-workstream refusal, popup defaults to No | VERIFIED | The round trip passes. `run_ack_tells_a_spawn_failure_from_a_timeout` (WR-02) and `enter_defaults_to_no` pass. |
| 37-07 | `y` shares `group_checklist` with the CLI; native tool or OSC 52; 64 KiB cap | VERIFIED | 11 `clipboard` tests pass. `spawn_seam_guard` (10) passes. |
| Review | 11 of 12 review findings fixed; IN-02 later fixed in the UI review (`aaaad1f0`) | VERIFIED | `37-REVIEW-DISPOSITION.md` shows open: 0. `owner_scrubbed_program` in `detail.rs:3098` applies `env -u` for every `ACK_ENV_REMOVE` variable. |

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/ui/screens/inbox.rs` (4565 lines) | VERIFIED | Substantive, pushed from `normal.rs:810`, rendered from the live digests. |
| `src/ui/screens/owner_actions.rs` (1782) | VERIFIED | Used by the inbox and the detail Owner block. |
| `src/ui/screens/ack_confirm.rs` (642) | VERIFIED | Pushed from `ack_key`. |
| `src/ui/screens/owner_file_view.rs` (484) | VERIFIED | Pushed by `open_target`. |
| `src/clipboard.rs` (387) | VERIFIED | Called from `owner_actions::copy`. `main.rs` writes `pending_osc52` after the frame. |
| `tests/owner_acknowledge_roundtrip.rs` | VERIFIED | Passes against the real oracle. |

### Key Link Verification

| From | To | Status |
|------|----|--------|
| `normal.rs` `i` | `InboxScreen::new` | WIRED (`normal.rs:810`) |
| `inbox.rs` `v` | `owner_actions::launch_real` | WIRED (`:1731`) |
| `owner_actions::launch_real` | `multiplexer::launch` with `owner_mux_argv` and `ExecArgv::new` | WIRED |
| `inbox.rs` `A` | `ack_confirm` then `run_ack` then `Action::OwnerAcknowledged` | WIRED (round trip) |
| `inbox.rs` `y` | `inbox::group_checklist` then `clipboard::pick_tool` | WIRED |
| `App::update` | `inbox::reanchor` | WIRED (SC3 tests) |

### Data-Flow Trace (Level 4)

Inbox rows are built each frame from `ctx.owner_digest(alias)` (`inbox_rows`, `fold_rows`). These digests are fed by the watcher reparse, and `ProjectStateLoaded` and `OwnerDigestLoaded` update them. The SC3 tests prove the reparse drives the rows. Status: FLOWING.

### Behavioral Spot-Checks

| Behavior | Command | Result |
|----------|---------|--------|
| SC1 tracer | `cargo test --locked --lib sc1` | 2 passed |
| Screen units | `--lib ui::screens::{inbox, owner_actions, ack_confirm, owner_file_view, owner_block, help}`, `clipboard` | 45, 22, 10, 5, 28, 35 and 11 passed, 0 failed |
| App and SC3 | `--lib app::tests` | 162 passed |
| Integration guards | `--test` for `owner_acknowledge_roundtrip`, `owner_open_conformance`, `owner_audit_conformance`, `spawn_seam_guard`, `focus_entry_guard`, `inbox_cli`, `owner_digest` | all green (3, 4, 3, 10, 3, 25, 30) |

I did not re-run the full suite. The orchestrator reports 4409 passed, 0 failed twice at HEAD.

### Probe Execution

No probes declared for this phase. Skipped.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| OWN-20 | 37-01, 03, 04, 05, 07 | Inbox screen: lanes, ordering, expand, `g`, `o`, `y`, legible at 100x30 | SATISFIED | SC2 |
| OWN-21 | 37-02, 37-03 | `v` through `multiplexer::launch`, focus-or-launch, argv via `ExecArgv::new` | SATISFIED | SC1 |
| OWN-22 | 37-06 | `A` via gsd-core after a confirm popup; the app writes nothing; the row disappears on the next refresh | SATISFIED | SC3 |
| OWN-23 | 37-01, 03, 07 | Help overlay and README updated | SATISFIED | SC4 |

All four IDs appear in the PLAN frontmatter and in REQUIREMENTS.md (marked complete, traceability rows `:188-191`). REQUIREMENTS.md maps no other ID to Phase 37, so there are no orphans.

### Anti-Patterns Found

None blocking. A scan of the added lines in `src`, `tests/*.rs` and `README.md` found no `TBD`, `FIXME`, `XXX`, `TODO`, `HACK`, `todo!` or `unimplemented!`.

| File | Severity | Note |
|------|----------|------|
| `src/ui/screens/detail.rs:3109-3113`, test name `..._is_pinned_at_three` | Info (resolved by the orchestrator after verification: doc comments updated; the test name is kept because the 37-03 plan's acceptance check names it) | Docs and a test name still say the GUI argv has 3 elements. Since the IN-02 fix it is separator, `env`, `-u VAR` pairs, `claude`, then the command, and the command is still the single last element. The code and tests are right; only the wording is stale. |
| WR-01 residual | Warning (human item 5) | No bracketed-paste handling or per-group cooldown for `v`, which has no confirm by design. Non-blocking, already recorded in 37-SECURITY.md. |
| WR-05 residual | Info | The acknowledge child has no `env_clear`, only `env -u` for the selector variables. The review left it as "consider". |

### Deviations from the plans (recorded, accepted)

- The narrowing order is phrase, alias, name, then phase cell, not the UI-SPEC's phrase, phase, alias, name (UI-REVIEW fix 1). `grid_for_inbox_matches_the_ui_spec_table` was changed to match (60 columns: phase cell 7). This was a legibility fix and it is in the recorded deviations.
- The 37-03 truth "multiplexer argv exactly `[env, claude, cmd]`" and "GUI argv exactly 3 elements" were widened by `env -u` (IN-02, `aaaad1f0`). The command is still one element, and the pin tests were updated.
- Failure copy for `A` uses the last stderr line rather than the first (inferred, RESEARCH Pitfall 7).

### Human Verification Required

See the `human_verification` list in the frontmatter (5 items): ordering and legibility, real multiplexer launch, real acknowledge round trip on a throwaway project, real clipboard and OSC 52, and the held-key residual.

### Gaps Summary

No gaps. Every ROADMAP success criterion and all four requirement IDs are backed by code that is wired and by passing tests. The app writes nothing into a project outside gsd-core's own acknowledge writer, which runs after an explicit Yes in a popup that defaults to No. What remains is verification that no unit test can do (a real multiplexer, a real clipboard, a real fleet), which is why the status is human_needed and not passed.

---

_Verified: 2026-11-15_
_Verifier: Claude (gsd-verifier)_
