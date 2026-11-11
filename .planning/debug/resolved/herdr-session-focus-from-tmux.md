---
status: resolved
trigger: "Focus fails for herdr-hosted sessions when TUI runs in tmux: status \"TTY pts/3 not found in any tmux pane\". Focus backend chosen from TUI ancestry rather than session's multiplexer."
created: 2026-11-10
updated: 2026-11-10
---

## Current Focus

hypothesis: CONFIRMED and FIXED (commit a8c71b8). Focus backends were chosen from the TUI's OWN environment, never from the session's; focus now follows the session's own multiplexer.
test: DONE. Gates green (raw): `cargo test --no-fail-fast` 3808 passed / 0 failed / 19 ignored over 61 targets; `cargo clippy --all-targets -- -D warnings` exit 0, no warnings; `cargo fmt --check` exit 0. Live check focused herdr pane w2:p1 and selected tmux 0:0.0 (see Evidence).
expecting: n/a (resolved).
next_action: none. Session archived to .planning/debug/resolved/ with a knowledge-base entry.

bug_class: Bohrbug (deterministic; reproduces every time for this topology)

tdd_checkpoint:
  test_file: "src/multiplexer/mod.rs"
  test_name: "multiplexer::tests::a_herdr_session_is_focused_in_herdr_instead_of_failing_in_tmux (end to end, user's exact symptom); multiplexer::tests::plan_focus_routes_a_herdr_session_to_herdr_when_the_tui_runs_in_tmux; multiplexer::tests::plan_focus_routes_a_tmux_session_to_tmux_when_the_tui_runs_in_herdr"
  status: "green (commit a8c71b8: all three pass; the two guards still pass)"
  failure_output: |
    a session that runs under herdr was reported missing from tmux: "TTY pts/3 not found in any tmux pane (session may have exited)"
    assertion `left == right` failed: the session lives under herdr (HERDR_ENV=1, a `herdr` server in its own chain), so herdr must be tried FIRST even though the TUI itself runs in tmux; got ["tmux"], the TUI-derived list, which can only ever end in the tmux tty error
      left: Some("tmux")
     right: Some("herdr")
    assertion `left == right` failed: the session lives in tmux ($TMUX set, `tmux: server` in its own chain), so tmux must be tried first even though the TUI runs in herdr; got ["herdr"]
      left: Some("herdr")
     right: Some("tmux")
  guards_passing_before_and_after: "plan_focus_keeps_tmux_only_routing_for_a_tmux_session_in_a_tmux_tui; plan_focus_without_session_facts_falls_back_to_the_tuis_own_backends (pin SC2 and the unreadable-/proc fallback so the fix cannot regress them)"

green_phase_plan:
  # A [INFERRED] decision list is in "Decisions" below; this is the build order.
  1_plan_focus: "mod.rs `plan_focus`: session = order_innermost(registry, &ctx.env, &ctx.ancestry); tui = order_innermost(registry, tui_env, tui_ancestry); backends = session ++ (tui minus names already present); env = tui_env overlaid with ctx.env's ENV_KEYS (session wins); empty ctx => identical to today. This alone turns the 3 RED tests green."
  2_probe: "mod.rs: `EnvSnapshot::from_environ_block(&[u8])` (NUL-separated KEY=VALUE, ENV_KEYS only, pure, tested) and `SessionContext::probe(pid)` (impure edge: read /proc/<pid>/environ + proc_chain(pid); any read error => empty facts). `focus_message` calls it. Accept HERDR_SOCKET_PATH only if absolute."
  3_reveal: "trait MultiplexerBackend gains two defaulted methods: `client_ttys(&self, env) -> Option<Vec<String>>` (None = no outer/reveal concept; herdr = Some(sorted ttys of herdr CLIENT processes)) and `focus_tty(&self, env, ttys: &[String]) -> Result<FocusOutcome, MuxError>` (default Unsupported; tmux = refactor of `focus` around `find_pane_target`, first list-panes match of any tty, select-window + select-pane). `focus_with` runs it after the winner against the plan's other backends. Notes (partial, never an error): no client attached; client not in a reachable tmux pane."
  4_client_scan: "herdr.rs: pure `is_herdr_client(comm, argv)` + `client_ttys_from(procs)` over an injected process listing (same seam style as proc_chain/parse_stat_*), impure edge scans /proc/<pid>/{stat,cmdline,fd/0}; no new Command::new."
  5_tests_to_add: "env-block parsing; probe-less fallback; nested session (both markers, nearer herdr wins) ordering through plan_focus; herdr client classification table (bare, `session attach default`, `--session x`, `server`, `pane list`, `agent focus`, `--remote x`); several clients => sorted deterministic pick; client in non-tmux terminal => 'Switched to X (focused in herdr; its herdr client is not in a tmux pane)'; no client => '(focused in herdr; no herdr client is attached)'; tmux focus_tty exact-match rows (reuse pane_tty_matches); symmetric case result has no spurious note; MUX trait default for focus_tty is Unsupported."
  6_docs: "Update the mod.rs module docs (MUX design table 'detect' column: 'of the SESSION for focus, of the TUI for launch'), the `focus_with` doc (D-09 now: session-derived first), herdr.rs/tmux.rs headers, and the Phase 33 note that 'controlling a multiplexer the TUI is not running inside' is now in scope for FOCUS."

reasoning_checkpoint:
  hypothesis: "Focus fails because `focus_message` -> `detect_current()` derives the backend list only from the TUI's own env markers (TMUX set, HERDR_ENV unset), so a session hosted by a sibling herdr server is only ever offered to the tmux backend, whose tty join cannot see herdr-owned ptys (pts/3, pts/4)."
  confirming_evidence:
    - "mod.rs `detect_current()` = EnvSnapshot::from_process() + proc_chain(TUI pid) + order_innermost(REGISTRY, ..); `HerdrBackend::detect` returns None unless the TUI env has HERDR_ENV == \"1\"; order_innermost doc: 'ancestry alone never makes a backend a candidate' (and test order_innermost_ignores_an_ancestor_whose_env_gate_fails)."
    - "tmux.rs:128-131 produces the exact observed text 'TTY {tty} not found in any tmux pane (session may have exited)'; live `tmux list-panes -a` lists only /dev/pts/13 (the herdr CLIENT's pty), never pts/3 or pts/4."
    - "/proc: session 672012 chain claude -> fish(671995) -> herdr(671927, `herdr server`) -> systemd; environ HERDR_ENV=1, HERDR_SOCKET_PATH=/home/blk/.config/herdr/herdr.sock, HERDR_PANE_ID=w2:p1, no TMUX; fd/0 -> /dev/pts/3. herdr client 3630647 `herdr session attach default` on /dev/pts/13 = tmux pane 0:0.0."
    - "herdr's own join WOULD succeed: `herdr pane process-info --pane w2:p1` lists foreground pid 672012 and shell_pid 671995; cmdline `claude --resume 6e02a046-...` equals pane list agent_session.value."
  falsification_test: "If, with herdr offered as a candidate and the session's env supplying HERDR_SOCKET_PATH, `HerdrBackend::focus` still failed to find the pane, the routing hypothesis would be insufficient. It does not: the join was measured read-only against the live server (Evidence 6)."
  fix_rationale: "Route by where the SESSION lives: read the session's own /proc/<pid>/environ markers and /proc ancestry (same pure `order_innermost` rule), try those backends first with the session's env (so herdr uses the session's own socket), keep the TUI-derived list as the fallback, then reveal the herdr client in the tmux pane that displays it. This fixes the cause (wrong source of truth for routing), not the message."
  blind_spots: "Named herdr sessions with several clients in different tmux panes cannot be matched client->server without extra CLI verbs (see decisions). tmux server B vs TUI on tmux server A is a related but separate defect (tmux::focus ignores the session's $TMUX). HERDR_PANE_ID can go stale after `pane move` (herdr skill docs), so it is not used as a join key. Untested on macOS (no /proc: session context is empty, legacy TUI-derived routing applies)."
  candidate_causes:
    - "code: backend list derived from TUI env/ancestry only (CONFIRMED, root cause)"
    - "config/environment: herdr CLI needs HERDR_SOCKET_PATH from the snapshot and the TUI env has none (CONFIRMED contributing: second blocker once herdr is routed; fixed by using the session's env)"
    - "data: session.tty is a herdr-owned pty, not a tmux pane tty (consequence, not a cause: tmux has no pane for it by design)"
  and_gate: "yes. Two conditions must both hold for the failure to be fixed: (1) herdr must become a candidate, and (2) its CLI must receive the session's socket path. Fixing (1) alone yields 'HERDR_SOCKET_PATH is unset' (herdr.rs:470-473, pinned by test focus_without_a_socket_path_fails_before_spawning). root_cause is therefore a set."

## Symptoms

expected: In the TUI (Project Sentriq, Sessions view), focusing a session (PID 672012 / 672048) switches to that session's herdr pane (and, if the herdr client is shown inside a tmux pane, selects that tmux pane too).
actual: Status line shows `TTY pts/3 not found in any tmux pane (session may have exited)` (same for PID 672048 / pts/4). Nothing is focused.
errors: "TTY pts/3 not found in any tmux pane (session may have exited)"
timeline: Since Phase 33 multiplexer backends (v2.3.0); herdr-from-tmux topology never worked.
reproduction: Run `cargo run` in a plain tmux pane; open Sentriq > Sessions; focus a session whose claude process runs under herdr (systemd─herdr(server)─fish─claude). Env of session: HERDR_ENV=1, HERDR_PANE_ID=w2:p1 (672048: w2:p2), no $TMUX. TUI env: TMUX set, no HERDR_ENV; herdr server is a sibling daemon, not an ancestor of the TUI.
started: Since Phase 33 multiplexer backends (v2.3.0); herdr-from-tmux topology never worked.

## Eliminated

- hypothesis: tmux's tty join matches the wrong pane or mis-parses the pane list (substring match, stripped prefix)
  evidence: `pane_tty_matches` is exact equality after one /dev/ strip (tmux.rs:30-36, pinned by tests). Live `tmux list-panes -a` contains only /dev/pts/13; there is no pts/3 or pts/4 pane to find. The NotFound is correct for tmux.
  timestamp: 2026-11-10

- hypothesis: the session has no tty recorded / the detector misread it
  evidence: the status text names `pts/3`, so `session.tty` was Some("pts/3"); /proc/672012/fd/0 -> /dev/pts/3 matches.
  timestamp: 2026-11-10

- hypothesis: herdr's pane join cannot identify these sessions, so routing alone would not help
  evidence: read-only live run of `herdr pane list` and `herdr pane process-info --pane w2:p1|w2:p2`: foreground pids 672012 / 672048, shell pids 671995 / 672020, `agent_session.value` equals the `--resume` uuid in each cmdline. Both the session-id tier and the pid tier would match exactly one pane each.
  timestamp: 2026-11-10

- hypothesis: (orchestrator pre-measurement) "backends are picked from the TUI's ancestry"
  evidence: partly wrong. Ancestry only REORDERS candidates; the gate that excludes herdr is the TUI's env marker (HERDR_ENV != "1"). `order_innermost` doc rule 3 and test `order_innermost_ignores_an_ancestor_whose_env_gate_fails` ("ancestry alone never makes a backend a candidate"). Consequence for the fix: reading the session's ancestry alone would not be enough; the candidate gate must be evaluated against the SESSION's env too.
  timestamp: 2026-11-10

- hypothesis: (orchestrator, live case B1) master still emits "Tab-switching requires running inside tmux (no $TMUX set)" for a herdr TUI tab-switching to a herdr session in another workspace
  evidence: that text exists nowhere in src/ on master (the only current wording is "Tab-switching requires running inside tmux or herdr (none detected)", reached only when neither the TUI nor, now, the session shows any multiplexer). `~/.cargo/bin/gsd-meta-manager --version` prints 2.2.0: the user ran the stale installed v2.2.0 binary. On master a herdr TUI gets herdr from its own env; test `a_herdr_tui_focuses_a_herdr_session_in_another_workspace` pins the result "Switched to knit" with a single `agent focus w6:p1`, and asserts neither message appears. Live run 2 (herdr TUI to w2:p1, a different workspace than the TUI's wA) also gave "Switched to sentriq".
  timestamp: 2026-11-10

## Evidence

- timestamp: 2026-11-10
  checked: src/multiplexer/mod.rs focus_message / detect_current / order_innermost; herdr.rs detect + herdr_cli; tmux.rs focus
  found: focus_message -> detect_current() reads ONLY the TUI process (env + ancestry). HerdrBackend::detect needs HERDR_ENV == "1" in that snapshot. herdr_cli needs HERDR_SOCKET_PATH in that same snapshot or fails with "HERDR_SOCKET_PATH is unset" before spawning.
  implication: root cause (code). Herdr is not even a candidate for a TUI in tmux, and would fail anyway for lack of a socket path.

- timestamp: 2026-11-10
  checked: /proc/672012 and /proc/672048 (stat, environ, fd/0, cwd, cmdline)
  found: chain claude(672012) -> fish(671995) -> herdr(671927 `herdr server`) -> systemd(2384) -> systemd(1). environ: HERDR_ENV=1, HERDR_SOCKET_PATH=/home/blk/.config/herdr/herdr.sock, HERDR_WORKSPACE_ID=w2, HERDR_TAB_ID=w2:t1, HERDR_PANE_ID=w2:p1, TERM_PROGRAM=herdr; no TMUX/TMUX_PANE. fd/0 -> /dev/pts/3. 672048: HERDR_PANE_ID=w2:p2, tab w2:t2, /dev/pts/4. cmdline `claude --resume <uuid>`.
  implication: the session's OWN facts (environ + ancestry) identify herdr unambiguously; nothing in them mentions tmux.

- timestamp: 2026-11-10
  checked: herdr client/tmux topology: /proc/3630647, /proc/3628117, /proc/3628116; `tmux list-panes -a`
  found: client 3630647 `herdr session attach default`, fd/0 /dev/pts/13, parent fish 3628117, whose parent is `tmux: server` 3628116. tmux lists a single pane `0:0.0` on /dev/pts/13 running `herdr`. (The TUI pane 0:0.1 from the report is no longer open at measurement time.)
  implication: the tmux pane to reveal is found by tty equality with a herdr CLIENT process; pts/3 and pts/4 are herdr-server-owned ptys that tmux cannot list.

- timestamp: 2026-11-10
  checked: `herdr pane list`, `herdr pane process-info`, `herdr status`, `herdr session list` with and without HERDR_* in the environment (read-only; nothing focused)
  found: the CLI answers from a clean env via the default socket (/home/blk/.config/herdr/herdr.sock), so a TUI in tmux CAN drive herdr; `herdr session list` maps session name -> socket. Pane list/process-info join the sessions exactly (see Eliminated).
  implication: no new transport is needed; only the env the existing herdr backend is handed must come from the session.

- timestamp: 2026-11-10
  checked: `herdr --skill` (id stability), `herdr --help` (CLI shape)
  found: "A pane moved into another workspace receives a new workspace-qualified pane ID ... only the moved process's inherited caller context keeps resolving that old ID". IDs and agent names are scoped to one server.
  implication: HERDR_PANE_ID in /proc/<pid>/environ is a hint that can be stale after `pane move`; the existing verified joins (session id, pid, cwd) stay the join. The environ is used for the SERVER (socket), not the pane.

- timestamp: 2026-11-10
  checked: .planning/milestones/v2.3.0-phases/33-.../33-CONTEXT.md
  found: "Not in this phase: ... Controlling a multiplexer the TUI is not running inside." D-07 gate is the TUI's env marker; D-09 focus tries each DETECTED backend (TUI-detected). The SC1 environment recorded was "the TUI runs inside a herdr pane".
  implication: the topology (herdr server as sibling daemon, client in a tmux pane next to the TUI) was explicitly out of the Phase 33 scope, not a regression. This fix is a scope extension for FOCUS only.

- timestamp: 2026-11-10
  checked: tests/spawn_seam_guard.rs SPAWN_ALLOWLIST
  found: `Command::new(` is allowed only in src/multiplexer/{gui,herdr,tmux}.rs (and a few unrelated files). Reading /proc needs no spawn.
  implication: the fix must keep every new spawn inside tmux.rs/herdr.rs; mod.rs only reads /proc.

- timestamp: 2026-11-10
  checked: knowledge base .planning/debug/knowledge-base.md
  found: no entry about multiplexers/focus/herdr/tmux (grep).
  implication: no known-pattern candidate; investigated from scratch.

- timestamp: 2026-11-10
  checked: RED run, `rtk bypass cargo test --lib --no-fail-fast multiplexer::` after the behaviour-preserving `plan_focus` extraction (focus_message now = proc_chain(TUI) + plan_focus(REGISTRY, .., SessionContext::default()) + focus_with; same routing as before)
  found: 83 passed, 3 failed, 3 ignored. The 3 failures are the new RED tests and nothing else; the full lib suite is 3012 passed / 3 failed, `tests/spawn_seam_guard` ok, `cargo clippy --all-targets` clean, `cargo fmt` clean. End-to-end failure text is the user's symptom verbatim: `a session that runs under herdr was reported missing from tmux: "TTY pts/3 not found in any tmux pane (session may have exited)"`. Routing failures: got ["tmux"] where ["herdr", ...] is required, and got ["herdr"] where ["tmux", ...] is required.
  implication: the bug is reproduced hermetically (fixtures only, nothing spawned) for the right reason. The two guard tests pass today and must keep passing.

- timestamp: 2026-11-10
  checked: GREEN phase. `plan_focus` now = order_innermost over the SESSION's env+ancestry, then the TUI's backends de-duplicated, acting on tui_env overlaid with the session's markers. `ProcFs` (explicit /proc root) reads the session's environ (<=1 MiB, first entry wins, absolute-socket check) and chain; `focus_message` = `focus_message_in(ProcFs::system(), REGISTRY, ...)`.
  found: the 3 RED tests pass; `multiplexer::` 86 passed before the new tests were added, 132 passed after (3 ignored live tests), 0 failed.
  implication: the routing hypothesis is sufficient for the reported topology (falsification test from the reasoning checkpoint held: herdr, handed the session's socket, finds the pane by session id).

- timestamp: 2026-11-10
  checked: orchestrator requirements A-D, implemented and tested through fake /proc + fixture `herdr`/`tmux` scripts that log to one shared file (`multiplexer::tests::entry::*`).
  found: (A) session-routed herdr focus uses the existing session-id tier: calls are exactly `pane list`, `agent focus w2:p1`, no `pane process-info`. (B1) herdr TUI -> herdr w6:p1 in another workspace: `Switched to knit`, herdr calls only. (B2) herdr TUI (no $TMUX) -> tmux session: tmux first, `tmux -S /tmp/tmux-1000/default list-panes ...`, `select-window -t 0:1.2`, `select-pane -t 0:1.2`, herdr never asked; a session on another tmux server is addressed by ITS socket (`-S /tmp/tmux-1000/work`); a relative `TMUX=relative/sock,1,0` is dropped and never reaches tmux. (D) measured topology (herdr server 671927, client 3630647 on pts/13 = tmux 0:0.0, TUI in 0:0.1): calls are herdr pane list, herdr agent focus w2:p1, tmux list-panes, select-window -t 0:0.0, select-pane -t 0:0.0. Edge cases: several clients -> first match in tmux's listing order, same result on repeat; client outside tmux -> "Switched to sentriq (focused in herdr; its herdr client is not in a tmux pane)", no select call; no client -> "(focused in herdr; no herdr client is attached)", tmux never asked.
  implication: both focus entry points share `focus_message`; `tests/focus_entry_guard.rs` pins that (exactly normal.rs and detail.rs call it; nothing outside src/multiplexer/ touches plan_focus/focus_with/REGISTRY/focus_tty/client_ttys).

- timestamp: 2026-11-10
  checked: first full `cargo test --no-fail-fast` after the change
  found: 1 failure in an UNRELATED-looking gate: `session_detector::tests::every_claude_argv_option_site_under_src_is_adjudicated` ("src/multiplexer/mod.rs spells a claude argv option token"). My fake cmdlines contained `"--resume"`. The session id reaches the focus path through `ClaudeSession::session_id`, never through the fake cmdline, so the literal was removed (not a new census row); rerun: 3808 passed / 0 failed.
  implication: the census gate worked as designed; the fix did not need a new adjudication.

- timestamp: 2026-11-10
  checked: guardrail signal 5 / mutation, done by hand (no cargo-mutants or Stryker configured). Mutation at the fix site: `plan_focus` made to ignore the session again (empty session list, TUI env only).
  found: 14 tests FAIL (the 3 RED tests, 4 new plan_focus tests, 7 `entry::*` tests); the guard tests and B1 (already worked on master) stay green. Restored: all pass. A second mutation (a UI file referencing `plan_focus`) makes `no_ui_code_reaches_a_backend_without_the_per_session_routing` fail with file:line.
  implication: the tests assert the root cause (routing source), not the symptom string; the bug returns on revert and is gone on reapply.

- timestamp: 2026-11-10
  checked: LIVE check against the running herdr/tmux, using a throwaway example (not committed) that calls the real `focus_message` for pid 672012 only. Run 1: TUI emulated in tmux (`TMUX=/tmp/tmux-1000/default,3628116,0`, all HERDR_* unset), with a logging `tmux` shim in PATH. Run 2: the process's real env (a herdr pane, wA:p1).
  found: before: `herdr pane list` had w2:p1 focused=False. Run 1: `Switched to sentriq` (was: "TTY pts/3 not found in any tmux pane (session may have exited)"); tmux calls made: `-S /tmp/tmux-1000/default list-panes -a -F ...`, `select-window -t 0:0.0`, `select-pane -t 0:0.0`; afterwards herdr focused pane = w2:p1 (session 6e02a046-...); tmux 0:0.0 pane_active=1 window_active=1 (it was already the only pane, so the selection was a no-op visually but the calls ran and exited 0). Run 2 (herdr TUI, different workspace): `Switched to sentriq`, no tmux calls, focused pane still w2:p1. No other pane was focused, nothing was sent to, closed or killed in any pane.
  implication: the reported symptom is gone in the real environment; the live check substitutes for the human-verify checkpoint because the user is unavailable.

- timestamp: 2026-11-10
  checked: `~/.cargo/bin/gsd-meta-manager --version`
  found: 2.2.0, the stale installed binary behind the "(no $TMUX set)" text the user saw.
  implication: not a defect of master; `cargo install --path .` (or the next release) refreshes it. Not done here (non-development work, user's machine).

## Decisions (every inferred decision is marked [INFERRED])

- [INFERRED] RED-phase scope: the only production change is the behaviour-preserving extraction of `focus_message`'s routing into the pure `plan_focus` (+ `SessionContext`, `FocusPlan`, stub body ignoring the session). Rationale: the test needs a pure seam that the real `focus_message` also goes through; otherwise it would pin a copy of the logic. Not committed on purpose: a red test must not land on its own, the green phase commits test + fix together.

- [INFERRED] Scope: focus only. Launch keeps targeting the TUI's innermost multiplexer (D-10: "never opens in an outer multiplexer the user is not looking at"). Rationale: launch has no session whose location could differ from the user's; the orchestrator asked to leave it unless clearly wrong, and it is not.
- [INFERRED] Routing source of truth = the session's own facts: `/proc/<pid>/environ` (the existing ENV_KEYS) + `proc_chain(session.pid)`, fed to the SAME pure `order_innermost`. The TUI-derived list stays as the fallback (appended, de-duplicated by backend name). Rationale: keeps D-09 "not found is not final", keeps the tmux-only user byte-identical (SC2), and degrades to today's behaviour wherever /proc cannot be read (non-Linux, other uid).
- [INFERRED] The env handed to `focus` = TUI env overlaid with the session's ENV_KEYS (session wins). Rationale: herdr must reach the session's own server socket (named herdr sessions have different sockets); the tmux backend ignores env.
- [INFERRED] HERDR_PANE_ID is NOT a join key. Rationale: stale after `pane move`; the existing session-id/pid/cwd joins are verified and exact, and were measured to match here.
- [INFERRED] Reveal step: two new trait methods with defaults, `client_ttys(env) -> Vec<String>` (herdr: ttys of herdr CLIENT processes) and `focus_tty(env, ttys) -> Result<FocusOutcome, MuxError>` (tmux: first `list-panes -a` pane whose tty equals one of them, exact match, select-window + select-pane). `focus_with` runs it after a winning backend, against the other backends of the plan. Rationale: tmux's tty join is already the right primitive; generic enough for zellij/WezTerm later.
- [INFERRED] herdr CLIENT = process with comm `herdr`, a terminal on fd 0 (pts/N or ttyN), and argv[1] absent, or `session attach <name>`, or `--session <name>`. `herdr server` and short-lived CLI calls (`pane list`, `agent focus`, ... including the TUI's own child calls) never count. `--remote`/`--machine` clients never count (different machine).
- [INFERRED] Several clients: collect all client ttys, sorted; tmux picks the first matching pane in `list-panes -a` order (session, window, pane index = deterministic). Rationale: the herdr pane focus is exact and server-wide, so this only chooses which already-correct view to show; it can never route keystrokes to another session. Mapping a client to its server via `herdr session list` is deferred (documented limitation).
- [INFERRED] Client in a non-tmux terminal, or no client at all: success with a partial note, never an error: "Switched to X (focused in herdr; its herdr client is not in a tmux pane)" and "... (focused in herdr; no herdr client is attached)". Rationale: the herdr pane IS focused; reporting an error would be false.
- [INFERRED] Symmetric case (TUI in herdr, session in tmux) is routed by the same rule and is covered by a RED test; the reverse reveal (tmux client shown inside a herdr pane) is NOT built. Rationale: same root cause, one rule; the reveal needs herdr to expose a "which pane shows this client" verb that was not measured.
- [INFERRED] Untrusted input: the socket path comes from another process's environ. Accept only a non-empty absolute path; it reaches the herdr CLI as an env var and only the allowlisted read/focus verbs ever run (same-uid process; no privilege boundary crossed).

- [INFERRED] (green phase, orchestrator requirements) Both focus entry points already call `focus_message`; no UI code was edited. They are pinned by `tests/focus_entry_guard.rs` (the callers must be exactly normal.rs and detail.rs; nothing outside src/multiplexer/ may use the seam internals). The detail.rs test helper near line ~20714 was neither edited nor reused.
- [INFERRED] The env block is untrusted: first entry wins (getenv), reads capped at 1 MiB, `HERDR_SOCKET_PATH` accepted only when absolute, `TMUX` only when the part before the first comma (how tmux itself reads `<socket>,<pid>,<idx>`) is absolute; a rejected value is DROPPED, so it neither routes nor reaches a command line. Chosen over "keep the marker, drop only the socket" because a tmux that names no server is not evidence the session is in tmux.
- [INFERRED] `TMUX_PANE` / `HERDR_PANE_ID` are not join keys. tty (tmux) and session id (herdr) are exact and were measured to match; a pane id can go stale after `pane move` (herdr) and adds a second source of truth.
- [INFERRED] tmux focus always adds `-S <socket>` when the focus env's `$TMUX` carries an absolute socket (the session's, else the TUI's, which is the same server as before for a tmux-only user). No `-S` otherwise. `open` (launch) is untouched.
- [INFERRED] herdr CLIENT = comm `herdr`, terminal on stdin, argv after argv[0] exactly empty, `session attach <name>` or `--session <name>`. Unrecognised shapes are not clients (a miss costs a note, a false hit would select the wrong tmux pane). `herdr session list` client->server mapping is DEFERRED.
- [INFERRED] The reveal runs only when another reachable backend is in the plan and the winner has a client concept; otherwise no /proc walk and no note. Notes: "focused in herdr; no herdr client is attached", "...; its herdr client is not in a tmux pane", "...; could not reveal its herdr client in tmux" (the reason is logged at warn, not shown). They follow the winner's own partial note after "; ".
- [INFERRED] Test seams: `ProcFs` (explicit root), `focus_message_in`, cfg(test) `with_program` constructors for the two backends, one shared `fixtures::EXEC_LOCK` (the herdr cli tests now take it instead of a private one, because ETXTBSY comes from a fork on ANY test thread). Fake cmdlines omit `--resume` to keep the argv census (`session_detector`) unchanged.
- [INFERRED] Optional item E (the session's own multiplexer location on the Sessions row) was NOT built: showing `herdr w2:p1 (via tmux 0:0.0)` needs a `herdr pane list` and a `tmux list-panes` per refresh (a spawn in the render path, which the async-blocking guard and the render budget exclude), or a new ClaudeSession field touching every struct literal in the tree, and detail.rs is being edited in parallel. Deferred.

## Resolution

root_cause: A set (the AND-gate fired). (1) CODE: `focus_message` -> `detect_current` -> `order_innermost` gated and ordered the focus backends on the TUI's OWN env markers and ancestry, never on where the SESSION lives; with a TUI in tmux and a session under a sibling herdr server, herdr was not a candidate (no HERDR_ENV in the TUI env) so only tmux ran and reported "TTY pts/3 not found in any tmux pane" (tmux cannot list ptys a herdr server owns). (2) CODE/ENVIRONMENT: once herdr is routed, it still had no control socket, because the snapshot every backend acts on was the TUI's. (3) CODE, found with the orchestrator's B2 case: `tmux::focus` ran `tmux list-panes` against whatever server the TUI's inherited env named, ignoring the session's own `$TMUX`, so a herdr TUI could not focus a tmux session at all.
fix: Commit a8c71b8. `plan_focus` orders the backends by the SESSION's own `/proc/<pid>/environ` markers and ancestry (the same pure `order_innermost`), then appends the TUI-derived backends as the fallback, and the backends act on the TUI env overlaid with the session's markers (session wins). herdr therefore reaches the session's socket and joins by session id; tmux addresses the session's server with `tmux -S <socket>` from the session's absolute `$TMUX` socket. After a herdr focus, a reveal step finds the herdr CLIENT's terminal in /proc and has tmux select the pane that shows it (new defaulted trait methods `client_ttys` / `focus_tty`); no client, a client outside tmux, or a failed reveal is a partial-outcome note, never a tmux "not found". `ProcFs` is the single /proc reader; `focus_message_in` is the testable entry; both UI paths call the one `focus_message`, pinned by `tests/focus_entry_guard.rs`. Launch is unchanged.
verification:
  target_test: { result: pass, tests: "multiplexer::tests::a_herdr_session_is_focused_in_herdr_instead_of_failing_in_tmux, plan_focus_routes_a_herdr_session_to_herdr_when_the_tui_runs_in_tmux, plan_focus_routes_a_tmux_session_to_tmux_when_the_tui_runs_in_herdr (RED on the legacy stub, GREEN now); 2 guards still pass" }
  mutation_check: { result: skipped, reason_if_skipped: "no cargo-mutants/Stryker configured; replaced by a manual mutation at the fix site (plan_focus ignores the session again): 14 tests failed, mutant killed", mutant_killed: true }
  no_op_deletion: { result: pass, deletion_justified_by_rca: n/a, note: "net change is additive; the only removals are the cfg-gated proc_chain (moved into ProcFs::chain, same logic) and the inline body of tmux focus (moved into list_panes/select)" }
  adjacent_tests: { result: pass, suites_run: "cargo test --no-fail-fast, all 61 targets incl. tests/spawn_seam_guard and tests/focus_entry_guard: 3808 passed, 0 failed, 19 ignored (raw)", also: "cargo clippy --all-targets -- -D warnings exit 0; cargo fmt --check exit 0" }
  revert_and_reconfirm: { result: pass, bug_returned_on_revert: true, fixed_on_reapply: true }
  guardrail_verdict: accepted
  live_check: "real focus path for pid 672012 only: TUI-in-tmux emulation gave 'Switched to sentriq' (was 'TTY pts/3 not found in any tmux pane'), herdr focused pane w2:p1, tmux calls list-panes + select-window -t 0:0.0 + select-pane -t 0:0.0 all ran (0:0.0 was already the active pane). Herdr-TUI run: 'Switched to sentriq', no tmux step. Substitutes for the human-verify checkpoint (user unavailable)."
oracle_type: specified (expected status texts and exact fixture-script call sequences come from the user's stated rule and from facts measured on the live /proc, herdr and tmux). Boundary neighbours added around the defect class: first vs duplicate environ entry, absolute vs relative vs empty vs comma-only socket, zero / one / several herdr clients, pts/13 vs pts/1 style exact tty matching, server vs client vs CLI-call argv shapes.
files_changed:
  - src/multiplexer/mod.rs (plan_focus, SessionContext, ProcFs, EnvSnapshot::from_environ_block/overlaid_with, ProcEntry/ProcScan, client_ttys/focus_tty trait methods, reveal_client, focus_with_procs, focus_message_in, docs, tests)
  - src/multiplexer/tmux.rs (program seam, -S from $TMUX, socket_of, focus_tty, find_pane_target_any, tests)
  - src/multiplexer/herdr.rs (client classification, client_ttys override, shared exec lock, docs, tests)
  - src/multiplexer/fixtures.rs (new, cfg(test): FakeProc, fixture-script World)
  - tests/focus_entry_guard.rs (new)

postmortem (blameless):
  five_whys:
    - why: "focus failed for a herdr session from a tmux TUI?"
      because: "only tmux was tried, and tmux has no pane for a herdr-owned pty."
    - why: "only tmux was tried?"
      because: "the candidate list is built from the TUI's env markers and ancestry; herdr's gate (HERDR_ENV) is absent in a tmux TUI."
    - why: "was the list built from the TUI?"
      because: "Phase 33 modelled one location, 'where the user is', for launch AND focus (D-09/D-10 shared the detection layer), and explicitly excluded 'controlling a multiplexer the TUI is not running inside'. For launch that is right; for focus the target already exists elsewhere."
    - why_branch_config: "even with herdr routed it would have failed: the CLI edge reads the socket only from the snapshot it acts on (the TUI's), so 'HERDR_SOCKET_PATH is unset' (AND-gate)."
    - why_branch_environment: "the same assumption sat in tmux::focus, which trusted the TUI's inherited $TMUX; it only held while the TUI and the session shared one tmux server."
  why_not_caught: "no gate existed for this class. Every Phase 33 focus test described the SC1 topology (TUI inside herdr) or a tmux-only user, so each fed the same environment to detection and to the session; no test or review step asked 'what if the TUI and the session are in different multiplexers?'. The SC1 environment recorded in Phase 33 context was one topology, and the exclusion in 33-CONTEXT.md was a scope note, not a failing check."
  recurrence_guard: "regression tests multiplexer::tests::entry::* (fake /proc + fixture herdr/tmux scripts, six topologies incl. the exact measured one), multiplexer::tests::plan_focus_* (pure routing), multiplexer::tmux::tests::cli::* (-S addressing), multiplexer::herdr::tests::only_a_long_lived_local_client_on_a_terminal_is_a_herdr_client; structural guard tests/focus_entry_guard.rs:the_sessions_panel_and_the_tab_switch_call_the_same_focus_entry and no_ui_code_reaches_a_backend_without_the_per_session_routing; knowledge-base entry herdr-session-focus-from-tmux."
  deferred:
    - "client -> server mapping via `herdr session list` (several clients of different named sessions: the reveal may show the wrong, already-correct view; nothing is ever sent to a wrong pane)."
    - "reverse reveal (a tmux client displayed inside a herdr pane)."
    - "optional item E: the session's multiplexer location on the Sessions row."
    - "the installed ~/.cargo/bin gsd-meta-manager is v2.2.0; refresh it to see this fix in the user's own TUI."
