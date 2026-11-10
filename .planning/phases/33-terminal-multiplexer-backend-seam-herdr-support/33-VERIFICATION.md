---
phase: 33-terminal-multiplexer-backend-seam-herdr-support
verified: 2026-11-10T23:10:00Z
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
covered_digest: "v4:sha256:6ce152447d498cdaf84585b292c90a645b5643176fd0a04898edcc82512235cc"
behavior_unverified: 0
overrides_applied: 0
coincidental_reliance_items:
  - truth: "Focus inside herdr lands in the session's pane (MUX-04)"
    reason: incidental-ordering
    harden: "Tier 3 (unique cwd) is reported as an exact switch. Mark MatchedBy::Cwd as partial, or let focus_with keep trying outer backends before settling for a cwd-only match (review WR-01)."
advisory:
  - finding: "herdr cwd-only focus tier can focus a non-session pane and pre-empts tmux's exact tty match when herdr is innermost (REVIEW WR-01)"
    category: other
    reason: "Plan 33-02 and MUX-04 ('cwd/tty as fallback') prescribe a unique-cwd tier, so this is not a must-have contradiction. It is a weak-key risk to weigh in a follow-up."
    evidence_status: "code-read at herdr.rs:217-226, 653-664"
  - finding: "herdr pid tier is gated behind a byte-exact cwd comparison (REVIEW WR-02)"
    category: other
    reason: "A symlinked or non-canonical project path would skip the pid join. Live pid-tier focus succeeded for a canonical path; the symlink case was not exercised."
    evidence_status: "code-read at herdr.rs:194-214"
  - finding: "MuxError::is_try_next is dead in production, and the doc says RefusedForSafety is never retried while focus_with/launch_with fall through on it (REVIEW WR-03)"
    category: other
    reason: "The must-have is that the type separates the classes, and it does. Falling through to the GUI chain on a refusal is exactly what MUX-05/MUX-06 require, and the GUI path never uses the refused vector. This is a doc/contract mismatch only."
    evidence_status: "code-read at mod.rs:99-141, 575-693"
---

# Phase 33: Terminal multiplexer backend seam + herdr support — Verification Report

**Phase Goal:** Replace the tmux-hardcoded `src/terminal_switch.rs` with a reusable multiplexer backend trait (detect, focus a Claude session, open a new session in a cwd with an argv), migrate tmux onto it, and add a herdr backend, so focus and launch work inside herdr (also nested in or around tmux). zellij, WezTerm mux, kitty, GNU screen, Rex and herdr-core/`hc` are deferred (MUX-F01..F06).
**Verified:** 2026-11-10
**Status:** passed
**Re-verification:** No, initial verification.

## Goal Achievement

SUMMARY.md claims were not trusted. Everything below was read from the code, run, or observed against the live herdr 0.9.5 server this verifier is running inside (`HERDR_ENV=1`, `HERDR_PANE_ID=wA:p1`).

### Observable Truths (roadmap contract = REQUIREMENTS MUX-01..07, plus plan must_haves)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | MUX-01: object-safe `MultiplexerBackend` (name, detect, focus, open; focus/open default `Unsupported`), static `REGISTRY`, `Detection{inside,reachable}`, and a recorded design against zellij, WezTerm, kitty, screen, Rex, herdr-core/hc | VERIFIED | `mod.rs:329-364` trait, `:367` `REGISTRY = [TMUX, HERDR]`, `:282` Detection. Design-note table at `mod.rs:39-48` has one row each for tmux, herdr TUI, herdr-core/hc, Rex, zellij, WezTerm mux, kitty and GNU screen, with the join key, open form, exec-direct status and deferral id (MUX-F01..F06). |
| 2 | MUX-02: innermost wins when nested, deterministic, unit-tested from env fixtures with no live multiplexer | VERIFIED | `order_innermost` (`mod.rs:402`) is pure over `EnvSnapshot` plus a `(pid, comm)` ancestry. The six measured shapes and `every_registered_backend_is_selected_by_its_own_fixtures` are in the 76-test multiplexer run, all passing. |
| 3 | MUX-03: tmux on the trait, unchanged behaviour; fail-closed direct-exec boundary enforced on the shared sink | VERIFIED | `tmux.rs` `open` runs `new-window -c <cwd> <argv...>`, and `list-panes`/exact-tty/`select-window`/`select-pane` focus. Reason strings are byte-identical to `git show 7a5c3af:src/terminal_switch.rs`. The only intentional text change is the none-detected message, which the plan prescribes. `ExecArgv::new` (`mod.rs:202`) refuses <2 elements and a `-`-leading head, and its private field with no `Default`/`From<Vec>` means no backend can bypass it. Old tests are moved (8 old, tmux.rs 5 + mod.rs 39 + herdr.rs 35 now). |
| 4 | MUX-04: in herdr, the focus key lands in the pane running the session; join on `agent_session.value` from `herdr pane list`, cwd/pid as fallback | VERIFIED | `herdr.rs:182-260`: tiers session-id, pid, unique cwd, then `herdr agent focus <pane_id>` with `tab focus` fallback. Ambiguity is refused (`:626`). **Live run:** a temporary test (removed afterwards) fed the real session from `detect_sessions()` to `HERDR.focus` against the live server and got `Ok(FocusOutcome{by: Pid, partial: None})`. The live `pane list` shows `agent_session{kind:"id", value:<uuid>}`, which is the field tier 1 joins on. The ignored live test `the_live_herdr_pane_list_and_process_info_parse` passes. |
| 5 | MUX-05: in herdr, launch/resume opens a new tab in the project cwd with no interpreter parsing the program vector; otherwise refuse and fall back with the reason | VERIFIED | `open_over_socket` (`herdr.rs:279`) sends one `layout.apply` NDJSON request. The command is a `serde_json` array of the `ExecArgv` elements (`:95-107`) and the cwd is a JSON field. `CLI_VERBS` (`:404`) is limited to pane list, pane process-info, agent focus and tab focus. The shell-typing verbs appear only inside the refusal test (`:1482-1485`). Test `launch_inside_herdr_opens_a_tab_through_layout_apply` drives a mock socket, and `layout_apply_request_carries_a_hostile_element_as_one_array_element` passes. A non-UTF-8 cwd, an unreachable socket and an error reply each return `Failed` and fall through to the GUI. |
| 6 | MUX-06: no multiplexer, or a backend failure, falls to the unchanged GUI chain and the reason reaches the user | VERIFIED | `launch_with` (`mod.rs:575-613`) tries only `backends.first()`, then `plan.gui`, and formats `"<backend>: <reason> — opened in <term>"` or `"...; no terminal emulator found (set $TERMINAL)"`. `gui.rs` `rank_gui` keeps the D-02 order. Tests: `launch_with_falls_through_to_the_gui_and_names_the_backend`, `..._a_refused_argv_never_calls_open`, `..._an_unreachable_detection_never_calls_open`, and the only-innermost test. |
| 7 | MUX-07: all five call sites go through the seam; no tmux-specific call outside the tmux backend | VERIFIED | `src/terminal_switch.rs` is deleted and the only `terminal_switch` strings left are in two doc comments and one unrelated test name. Seam calls: `normal.rs:840` focus; `detail.rs:4952` resume launch, `:5415` new-session launch, `:5378` Sessions focus; `resolve_launch_plan` (`detail.rs:2409`) builds the plan via `detect_current` and `plan_launch`. `Command::new("tmux")` and `$TMUX` reads occur only in `src/multiplexer/tmux.rs`. `launcher/sandbox.rs` only lists `TMUX` as a scrubbed env name. `resumable_session_id` runs before `resolve_launch_plan` (`detail.rs:4932-4937`). |
| 8 | Prohibition: never type the vector into a shell via herdr (pane run, send-text, send-keys, agent start) | VERIFIED | No production use. The allowlist refusal test enumerates all four verbs. |
| 9 | Prohibition: never open in an outer multiplexer, only the innermost then GUI | VERIFIED | `launch_with` takes `backends.first()` only, and a test with a second backend asserts it is never called. |
| 10 | Prohibition: no silent fall-through | VERIFIED | The status string always carries `<backend>: <reason>`. |
| 11 | Prohibition: an ambiguous herdr match is refused, never guessed | VERIFIED | `Tier::Ambiguous` becomes `RefusedForSafety`. Tests: `two_panes_with_the_same_session_id_are_refused`, `two_panes_matching_by_pid_are_refused_through_focus`, `an_ambiguous_cwd_is_refused_and_nothing_is_focused`, `the_order_of_panes_never_decides_the_match`. |
| 12 | Ids from herdr output that are empty or `-`-leading are refused before argv (CWE-88) | VERIFIED | `validate_id` (`herdr.rs:594`); test `a_dash_leading_pane_id_is_refused_before_it_reaches_argv` passes. |
| 13 | Pure core: parsing and ordering are pure over `EnvSnapshot`; no shared command runner | VERIFIED | `EnvSnapshot::from_process` is the only `std::env` read, and `herdr_cli` is private to herdr. |
| 14 | Spawn-seam allowlist names exactly the spawning files; the argv census is unchanged | VERIFIED | `spawn_seam_guard` passes 10/10, listing `multiplexer/{gui,herdr,tmux}.rs`. `every_claude_argv_option_site_under_src_is_adjudicated` passes. |
| 15 | Env markers are exact strings: `TMUX` present with any value, `HERDR_ENV == "1"`, reachable only with a non-empty socket path | VERIFIED | `tmux.rs` `detect` uses `env.get("TMUX").map(..)`; `herdr.rs:159-165`. |
| 16 | `ExecArgv` boundary, ordering and hostile elements: 2 elements OK, 1/0 refused, head `-`/`-d` refused, `env` accepted, order preserved, one array element per argv element | VERIFIED | `ExecArgv::new` code plus the passing boundary and hostile-element tests. |
| 17 | Backstop truth (MUX-03 precision): no numeric conversion in the argv sink can truncate | VERIFIED (directly observed) | The sink holds a `Vec<String>`, with `len()` as `usize` and no `as`/`try_from` cast anywhere in `mod.rs` non-test code, `tmux.rs` or `herdr.rs` (grep: the only `as u32` is in a test fixture). Byte pass-through is observed by the hostile-element unit test, and the ignored live `layout.apply` test records a file named from a payload with `;`, `$HOME` and `*` arriving as one argv element. |
| 18 | Focus: none detected gives the `Tab-switching requires running inside tmux or herdr (none detected)` text; a single backend's reason is verbatim and several are name-prefixed innermost first | VERIFIED | `focus_with` (`mod.rs:652-693`); test `focus_with_reports_the_none_detected_text`. |
| 19 | Docs and README describe tmux and herdr, list the `HERDR_*` variables and cite only `src/multiplexer` | VERIFIED | `docs/CONFIGURATION.md:116-120, 642, 663`, `README.md:35-47`, `docs/ARCHITECTURE.md:249-252`, and `docs/GETTING-STARTED.md:17`. |
| 20 | Pid join, `/proc` ancestry, and herdr CLI called with `HERDR_SOCKET_PATH` from the snapshot | VERIFIED | `herdr.rs:196-199, 440-448`; `focus_without_a_socket_path_fails_before_spawning`. |

**Score:** 20/20 truths verified, 0 behavior-unverified.

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/multiplexer/mod.rs` (1656 lines) | VERIFIED | Substantive, wired from `lib.rs`, `detail.rs` and `normal.rs`. |
| `src/multiplexer/tmux.rs` (386) | VERIFIED | `pub static TMUX`, registered. |
| `src/multiplexer/herdr.rs` (1525) | VERIFIED | `layout.apply`, `CLI_VERBS`, `proc_chain(` use. |
| `src/multiplexer/gui.rs` (118) | VERIFIED | `xdg-terminal-exec` rank, `spawn_terminal`. |
| `src/terminal_switch.rs` | VERIFIED DELETED | Absent from the tree. |
| `docs/CONFIGURATION.md` | VERIFIED | `HERDR_SOCKET_PATH` rows. |

### Key Link Verification

| From | To | Status |
|------|----|--------|
| `detail.rs` | `multiplexer::launch(` (x2) | WIRED (`:4952`, `:5415`) |
| `normal.rs` / `detail.rs` | `multiplexer::focus_message(` | WIRED (`:840`, `:5378`) |
| `mod.rs` REGISTRY | `herdr::HERDR` | WIRED (`:367`) |
| `herdr.rs` | `$HERDR_SOCKET_PATH` via `UnixStream` | WIRED |
| `herdr.rs` pid tier | `proc_chain(` | WIRED (`:199`) |
| `spawn_seam_guard.rs` | tmux.rs / herdr.rs / gui.rs | WIRED, test green both directions |

### Data-Flow Trace (Level 4)

The herdr pane list is real data. A live `herdr pane list` returns `agent_session.value`, `cwd`, `pane_id` and `tab_id`, which parse into `HerdrPane` (live parse test passes), and the pid tier resolved a real `ClaudeSession` to `Ok(Pid)`. FLOWING.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Multiplexer suite | `cargo test --lib multiplexer` | 76 passed, 0 failed, 3 ignored | PASS |
| Spawn seam guard | `cargo test --test spawn_seam_guard` | 10 passed | PASS |
| Argv census | `cargo test --lib every_claude_argv_option_site` | 1 passed | PASS |
| Live herdr pane list and process-info | `cargo test --lib the_live_herdr_pane_list -- --ignored` | 1 passed | PASS |
| Live end-to-end herdr focus (temporary test, removed, tree clean) | `HERDR.focus(env, real session)` | `Ok(FocusOutcome{by: Pid, partial: None})` | PASS |
| `herdr agent focus <pane>` against the live server | CLI | `agent_info` result | PASS |
| Clippy | `cargo clippy --all-targets -- -D warnings` | clean | PASS |
| Orchestrator gate | `cargo test --no-fail-fast` | 3714 passed, 0 failed, 19 ignored | accepted from orchestrator, not re-run in full |

I did not re-run the live `layout.apply` test, because it creates a tab in the user's herdr workspace. The 33-02 SUMMARY records it passing, with no leftover tabs. Its in-process equivalent, the mock-socket launch test, passes here.

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

Every ID in the PLAN frontmatter (33-01: MUX-01, 03, 05, 06, 07; 33-02: MUX-01, 02, 04) maps to REQUIREMENTS.md, and the union covers MUX-01..07 with no orphans. REQUIREMENTS.md still shows these as `Pending`/unchecked. That is a tracking update for the orchestrator, not a code gap.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX` in `src/multiplexer/`. The review findings were weighed against the must-haves:

| Finding | Severity | Contradicts a must-have? |
|---------|----------|--------------------------|
| WR-01 cwd-only tier can focus a non-session pane and pre-empts tmux's exact tty match | Warning (advisory) | No. The tier is explicit in MUX-04 ("cwd/tty as fallback") and in 33-02 ("then a unique cwd match"). The prohibition covers only *ambiguous* matches, which are refused. A weak-key risk remains. |
| WR-02 pid tier gated behind exact cwd equality | Warning (advisory) | No. The live pid focus worked for a canonical path. A symlinked path would degrade to NotFound, never to a wrong focus. |
| WR-03 `is_try_next` unused, docs disagree with the fall-through behaviour | Warning (advisory) | No. The classes exist as required. Fall-through on a refusal is what MUX-05/06 specify. |
| IN-01..IN-05 | Info | No. |

33-SECURITY.md reports SECURED. 33-UI-REVIEW.md scores 6/8; this phase adds no new UI surface, only status strings.

### Human Verification Required

None. The one candidate, real herdr Tab-focus behaviour, was settled by running the real `HERDR.focus` against the live server (above). The remaining unexercised piece is the visual confirmation that the keypress moves the screen, which `agent focus` returning `agent_info` and the unit-tested key-to-`focus_message` wiring cover.

### Gaps Summary

No gaps. All seven requirements are satisfied in the code, and tmux behaviour is preserved. The three review warnings are recorded as advisories for a follow-up phase (WR-01 and WR-02 are the most worth fixing, since both concern the herdr focus join's reliability). None contradicts a must-have.

---

_Verified: 2026-11-10_
_Verifier: Claude (gsd-verifier)_
