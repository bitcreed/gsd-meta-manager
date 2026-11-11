---
phase: 33-terminal-multiplexer-backend-seam-herdr-support
verified: 2026-11-11T02:30:00Z
status: passed
score: 7/7 requirements verified; 20/20 plan truths verified (0 behavior-unverified)
covered_files:
  - ".planning/phases/33-terminal-multiplexer-backend-seam-herdr-support/33-01-PLAN.md"
  - ".planning/phases/33-terminal-multiplexer-backend-seam-herdr-support/33-01-SUMMARY.md"
  - ".planning/phases/33-terminal-multiplexer-backend-seam-herdr-support/33-02-PLAN.md"
  - ".planning/phases/33-terminal-multiplexer-backend-seam-herdr-support/33-02-SUMMARY.md"
  - "src/multiplexer/gui.rs"
  - "src/multiplexer/herdr.rs"
  - "src/multiplexer/mod.rs"
  - "src/multiplexer/tmux.rs"
  - "src/ui/screens/detail.rs"
  - "src/ui/screens/normal.rs"
  - "tests/spawn_seam_guard.rs"
covered_digest: "v4:sha256:44530c7572c56699b4a86d1bf027ca6e0ee49b5a733e2c8d6ae645309613546e"
behavior_unverified: 0
overrides_applied: 1
overrides:
  - must_have: "Inside tmux every status string is byte-identical, including 'tmux: <reason> — opened in <term>' and 'tmux: <reason>; no terminal emulator found (set $TERMINAL)' (33-01 plan truth 2)"
    reason: "Deliberate change UI-01: the fall-through line now shows 'tmux: <failure-class phrase> — opened in <term>'. The raw reason (socket paths, herdr error JSON) is logged at warn. The success text, the none-detected text, 'Failed to launch: <e>' and 'No terminal emulator found (set $TERMINAL)' are unchanged. What launches, and where, is unchanged, so roadmap SC2 and MUX-03 hold."
    accepted_by: "orchestrator, requested via 33-UI-REVIEW.md UI-01 and recorded in 33-REVIEW-FIX.md (confirm at commit)"
    accepted_at: "2026-11-10T23:30:00Z"
coincidental_reliance_items: []
advisory:
  - finding: "IN-06: same_dir runs fs::canonicalize on both paths per herdr pane on the UI thread (REVIEW IN-06, open)"
    category: other
    reason: "A hung network mount under a pane cwd could block focus. It is a robustness concern next to the accepted no-deadline CLI exposure (T-33-13), not a must-have. No requirement or success criterion mentions a bound. Fix is to hoist the working_dir canonicalize out of the loop."
    evidence_status: "code-read at herdr.rs:702-716, reproduced from REVIEW"
  - finding: "Residual of WR-01: a Claude-agent pane in the same directory whose process-info cannot be read can still win the directory-only tier"
    category: other
    reason: "MUX-04 prescribes cwd as a fallback. The match carries the 'matched by working directory only' note, a plain shell pane is never chosen, and an ambiguous match is refused."
    evidence_status: "code-read at herdr.rs:760-771"
---

# Phase 33: Terminal multiplexer backend seam + herdr support — Verification Report

**Phase Goal:** Replace the tmux-hardcoded `src/terminal_switch.rs` with a reusable multiplexer backend trait (detect, focus a Claude session, open a new session in a cwd with an argv), migrate tmux onto it, and add a herdr backend, so focus and launch work inside herdr (also nested in or around tmux). zellij, WezTerm mux, kitty, GNU screen, Rex and herdr-core/`hc` are deferred (MUX-F01..F06).
**Verified:** 2026-11-11
**Status:** passed
**Re-verification:** Yes. The previous report (2026-11-10, fingerprint `v4:sha256:6ce15244...`) went stale when the code-review fixes WR-01..03, IN-01..05 and UI-01 changed `src/multiplexer/herdr.rs` and `src/multiplexer/mod.rs`. Every truth was re-checked against the current code, and the changed behaviour got a full check.

## Goal Achievement

The tree is `master` at `fa06bec`, product code untouched. This verifier runs inside herdr (`HERDR_ENV=1`), so the live herdr server was available for read-only checks.

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | MUX-01: object-safe `MultiplexerBackend` (name, detect, focus, open; focus/open default `Unsupported`), static `REGISTRY`, `Detection{inside,reachable}`, and a recorded design against zellij, WezTerm, kitty, screen, Rex, herdr-core/hc | VERIFIED | The design-note table in `mod.rs:39-48` has one row each for tmux, herdr TUI, herdr-core/hc, Rex, zellij, WezTerm mux, kitty and GNU screen, with deferral ids MUX-F01..F06. The herdr row was updated to the restricted cwd tier (commit bce8ebf). `Detection.inside` was kept and now has a fixture where `inside=false` (IN-01). |
| 2 | MUX-02: innermost wins when nested, deterministic, unit-tested from env fixtures | VERIFIED | `order_innermost` is pure over `EnvSnapshot` plus a `(pid, comm)` ancestry. The six measured shapes, the "own fixtures" invariant and the new `order_innermost_skips_a_backend_that_is_not_inside` all pass in the 81-test multiplexer run. |
| 3 | MUX-03: tmux on the trait with unchanged behaviour; fail-closed direct-exec boundary on the shared sink | VERIFIED | `ExecArgv::new` refuses <2 elements and a `-`-leading head, and the field is private with no `Default` or `From<Vec>`. `tmux.rs` is the only file with `Command::new("tmux")`. The refusal text now says "a new window" instead of "a tmux window" (IN-02), and the pinned substrings `directly` and `option` still hold. |
| 4 | MUX-04: in herdr, the focus key lands in the pane running the session; join on `agent_session.value`, cwd/pid as fallback | VERIFIED | `herdr.rs:182-260`: session-id tier, then pid via `process-info`, then a directory tier, then `agent focus <pane_id>` with `tab focus` as the fallback. The pid tier now matches paths through `same_dir` (WR-02), so a symlinked project path no longer skips it. The directory tier (WR-01) considers only panes that herdr reports as running the session's agent kind and whose process-info was unreadable, and it flags the result with a note. The live `herdr pane list` I ran returns `"agent":"claude"` and `agent_session.value` on agent panes, and omits both on plain shell panes (`w2:p3`, `w3:p2`). So the new filter field exists in the real data. The ignored live test `the_live_herdr_pane_list_and_process_info_parse` passes. |
| 5 | MUX-05: in herdr, launch opens a new tab in the project cwd with no interpreter parsing the program vector | VERIFIED | `open_over_socket` sends one `layout.apply` request. The command is a JSON array and the cwd a JSON field. `CLI_VERBS` is limited to pane list, pane process-info, agent focus and tab focus. The mock-socket launch test and the hostile-element test pass. `parse_layout_apply_reply` now accepts `"error": null` (IN-03) while `{"error":null}` with no result still fails closed. |
| 6 | MUX-06: no multiplexer, or a backend failure, falls to the GUI chain and the reason reaches the user | VERIFIED | `launch_with` (`mod.rs:597-640`) tries only `backends.first()`, then the GUI. The status line carries `<backend>: <phrase> — opened in <term>`, where `<phrase>` is the failure class from `launch_phrase()` ("no target found", "cannot open sessions", "refused for safety", "could not open a session", "not reachable"). The full reason goes to `tracing::warn!`. The user is told which backend failed and how it failed. Tests `launch_with_falls_through_to_the_gui_and_names_the_backend`, `..._without_a_gui_reports_the_reason_and_the_hint` and `every_error_class_has_a_short_launch_phrase_free_of_the_raw_reason` pass. This differs from the plan's "byte-identical" text, see the override. |
| 7 | MUX-07: all five call sites go through the seam; no tmux-specific call outside the tmux backend | VERIFIED | `src/terminal_switch.rs` does not exist. The seam call sites are `normal.rs:840`, `detail.rs:4952`, `:5378` and `:5415`, plus `resolve_launch_plan`. The only `terminal_switch` strings left are two doc comments and an unrelated test name. `Command::new("tmux")` is only in `multiplexer/tmux.rs`. |
| 8 | Prohibition: never type the vector into a shell via herdr | VERIFIED | No production use of the typing verbs; the allowlist refusal test enumerates them. |
| 9 | Prohibition: never open in an outer multiplexer | VERIFIED | `launch_with` takes `backends.first()` only. The `..._only_tries_the_first` test passes. |
| 10 | Prohibition: no silent fall-through | VERIFIED | The status line always carries `<backend>: <phrase>`. |
| 11 | Prohibition: an ambiguous herdr match is refused, never guessed | VERIFIED | `Tier::Ambiguous` becomes `RefusedForSafety` on all three tiers. The ambiguity tests pass. The WR-01 fix tightens the weakest tier. |
| 12 | Ids from herdr output that are empty or `-`-leading are refused (CWE-88) | VERIFIED | `validate_id` runs before every CLI id use. A candidate that fails it lands in `unprobed` and is then refused by the later `validate_id`, which fails closed (REVIEW confirmed). |
| 13 | Pure core, no shared command runner | VERIFIED | Unchanged. `herdr_cli` is private. |
| 14 | Spawn-seam allowlist and argv census | VERIFIED | `spawn_seam_guard` 10/10 passes. The census test passes in the full run. |
| 15 | Env markers are exact strings; herdr reachable only with a non-empty socket path | VERIFIED | `herdr.rs:159-165`. |
| 16 | `ExecArgv` boundary, ordering, hostile elements | VERIFIED | Boundary and hostile-element tests pass. |
| 17 | Backstop: no numeric conversion in the argv sink can truncate | VERIFIED (directly observed) | The sink holds a `Vec<String>`. The hostile-element test shows bytes arriving as one array element. The ignored live `layout.apply` test was not re-run, because it creates a tab in the user's workspace. The previous report and the 33-02 SUMMARY record it passing. |
| 18 | Focus: none-detected text; a single backend's reason verbatim; several are name-prefixed | VERIFIED | `focus_with` and `focus_with_reports_the_none_detected_text` pass. |
| 19 | Docs and README describe tmux and herdr and list `HERDR_*` | VERIFIED | `docs/CONFIGURATION.md:119, 642`. |
| 20 | Pid join, `/proc` ancestry, and the CLI called with `HERDR_SOCKET_PATH` from the snapshot | VERIFIED | `focus_without_a_socket_path_fails_before_spawning` passes. |
| 21 | Plan 33-01 truth 2: tmux status strings byte-identical | PASSED (override) | Override: deliberate UI-01 change to the fall-through phrase. See the frontmatter. |

**Score:** 20/20 truths verified, plus 1 PASSED (override). 0 behavior-unverified.

### Deferred Items

None.

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/multiplexer/mod.rs` (1751 lines) | VERIFIED | Substantive, wired from `lib.rs`, `detail.rs` and `normal.rs`. |
| `src/multiplexer/tmux.rs` (386) | VERIFIED | Registered in `REGISTRY`. |
| `src/multiplexer/herdr.rs` (1715) | VERIFIED | `layout.apply`, `CLI_VERBS`, `proc_chain(`, `cwd_fallback`, `same_dir`. |
| `src/multiplexer/gui.rs` (118) | VERIFIED | GUI chain. |
| `src/terminal_switch.rs` | VERIFIED DELETED | Absent. |
| `docs/CONFIGURATION.md` | VERIFIED | `HERDR_SOCKET_PATH` rows. |

### Key Link Verification

| From | To | Status |
|------|----|--------|
| `detail.rs` | `multiplexer::launch(` (x2) | WIRED (`:4952`, `:5415`) |
| `normal.rs`, `detail.rs` | `multiplexer::focus_message(` | WIRED (`:840`, `:5378`) |
| `mod.rs` REGISTRY | `herdr::HERDR` | WIRED |
| `herdr.rs` | `$HERDR_SOCKET_PATH` via `UnixStream` | WIRED |
| `herdr.rs` pid tier | `proc_chain(` | WIRED |
| `spawn_seam_guard.rs` | tmux.rs, herdr.rs, gui.rs | WIRED, green |

### Data-Flow Trace (Level 4)

A live `herdr pane list` returns `agent`, `agent_session.value`, `cwd`, `foreground_cwd`, `pane_id` and `tab_id`, which parse into `HerdrPane` (the live parse test passes). FLOWING.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full suite | `cargo test --no-fail-fast` (raw output, via `rtk bypass`) | 3755 passed, 0 failed, 19 ignored, no FAILED lines | PASS |
| Multiplexer suite | `cargo test --lib multiplexer` | 81 passed, 0 failed, 3 ignored | PASS |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | PASS |
| Live pane list and process-info parse | `cargo test --lib the_live_herdr_pane_list -- --ignored` | 1 passed | PASS |
| Live pane data carries `agent` and `agent_session` on agent panes only | `herdr pane list` | confirmed on 11 panes | PASS |
| Old module gone | `ls src/terminal_switch.rs` | not found | PASS |

### Probe Execution

No probes declared. Step 7c skipped.

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| MUX-01 | 33-01, 33-02 | SATISFIED | Truth 1 |
| MUX-02 | 33-02 | SATISFIED | Truth 2 |
| MUX-03 | 33-01 | SATISFIED | Truth 3 |
| MUX-04 | 33-02 | SATISFIED | Truth 4 |
| MUX-05 | 33-01 | SATISFIED | Truth 5 |
| MUX-06 | 33-01 | SATISFIED | Truth 6 |
| MUX-07 | 33-01 | SATISFIED | Truth 7 |

The PLAN frontmatter IDs (33-01: MUX-01, 03, 05, 06, 07; 33-02: MUX-01, 02, 04) cover MUX-01..07 with no orphans. REQUIREMENTS.md marks all seven `[x]` and "Complete".

### Anti-Patterns Found

No `TBD`, `FIXME` or `XXX` in `src/multiplexer/`.

| Finding | Severity | Contradicts a must-have? |
|---------|----------|--------------------------|
| IN-06 `same_dir` does unbounded `canonicalize` per pane on the UI thread (open) | Info (advisory) | No |
| WR-01 residual: a Claude-agent pane with unreadable process-info can win the directory tier | Info (advisory) | No; the match is noted and ambiguity is refused |
| WR-01..03, IN-01..05, UI-01 | Fixed | Each fix was read in the code and has a passing test |

33-SECURITY.md reports SECURED. 33-UI-REVIEW.md was refreshed after the UI-01 fix. UI-02 (the none-detected text gives no remedy) was left unapplied on purpose, because that string is pinned for tmux users.

### Human Verification Required

None. The remaining unexercised piece is the visual confirmation that a keypress moves the screen in a real herdr session. The prior report settled the focus path by running `HERDR.focus` against the live server (`Ok(FocusOutcome{by: Pid})`). The restricted directory tier and `same_dir` since then are covered by unit tests, including a real-symlink fixture.

### Gaps Summary

No gaps. All seven requirements and all success criteria hold in the current code. Two intentional deviations from the plan text exist: the launch fall-through line shows a failure-class phrase instead of the raw reason (UI-01, recorded as an override), and the directory tier is stricter than the plan's "unique cwd match" (WR-01), which only narrows it. The open info finding IN-06 is advisory.

---

_Verified: 2026-11-11_
_Verifier: Claude (gsd-verifier)_
