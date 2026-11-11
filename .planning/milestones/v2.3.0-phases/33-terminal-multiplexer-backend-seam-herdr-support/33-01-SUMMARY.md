---
phase: 33-terminal-multiplexer-backend-seam-herdr-support
plan: 01
subsystem: ui
tags: [multiplexer, tmux, herdr, unix-socket, argv-safety, rust]

requires:
  - phase: 21
    provides: "CWE-88 fused resume element, spawn-seam allowlist, exec-direct tmux boundary"
provides:
  - "src/multiplexer/ backend seam: MultiplexerBackend trait, static REGISTRY, ExecArgv fail-closed sink, EnvSnapshot, pure order_innermost / plan_launch / launch_with / focus_with"
  - "tmux backend moved onto the trait with unchanged behaviour and byte-identical status strings"
  - "herdr backend that opens a tab with one std-only layout.apply socket request (no shell)"
  - "GUI terminal fallback chain with the backend reason surfaced on the status line"
  - "All five call sites routed through crate::multiplexer; src/terminal_switch.rs deleted"
affects: [33-02 nesting precedence, herdr focus, design note, user docs]

actuals:
  tokens: 35000
  tasks: 2
  commits: 2
plan_head_before: 5925b295122053c3d7d9f71c989dd21a9f044bf6
plan_head_after: 7438f20a184994708af7618f1431e6c88f6f5287

tech-stack:
  added: []
  patterns:
    - "Object-safe trait with Unsupported defaults + static registry slice (mirrors session_detector::ProcessProbe)"
    - "Validated newtype sink (ExecArgv): private field, no Default, no From, checked before any process or socket"
    - "Pure core over an injected EnvSnapshot; commands and the socket only at each backend's edge"
    - "Tests inject backends and the GUI spawner (launch_with / focus_with); real-environment entries are called only by production call sites"

key-files:
  created:
    - src/multiplexer/mod.rs
    - src/multiplexer/tmux.rs
    - src/multiplexer/gui.rs
    - src/multiplexer/herdr.rs
  modified:
    - src/lib.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/normal.rs
    - src/session_detector.rs
    - tests/spawn_seam_guard.rs
  deleted:
    - src/terminal_switch.rs

key-decisions:
  - "[inferred - audit] herdr launch uses one narrow std-only socket layout.apply request, resolving D-12 (socket alternative) against D-13 (no socket client): herdr pane run / send-text / send-keys / agent start type into a shell and are never used, not even as a fallback"
  - "[inferred - audit] focus and open also receive the EnvSnapshot, so a stateless static registry entry acts on the env it was detected from (refinement of D-02's method list)"
  - "[inferred - audit] tmux detection keeps var_os(TMUX).is_some() semantics: a set-but-empty value still counts, for SC2"
  - "[inferred - audit] ExecArgv refusal text keeps naming 'a tmux window' byte-identical; the substrings 'directly' and 'option' are unchanged"
  - "Matched identity of a successful focus goes to tracing::debug only; success text stays 'Switched to <label>'"
  - "A non-UTF-8 cwd is refused by the herdr backend (Failed) rather than sent lossily, since a lossy path would open the tab elsewhere"

patterns-established:
  - "Seam entry points split into launch/focus_message (real env, production call sites only) and launch_with/focus_with (injected, used by every test)"
  - "A spawn-allowlist edit lands in the same commit as the file move it describes"

requirements-completed: [MUX-01, MUX-03, MUX-05, MUX-06, MUX-07]

duration: 1 session
completed: 2026-11-10
status: complete
---

# Phase 33 Plan 01: Multiplexer backend seam and herdr launch Summary

**A `src/multiplexer/` backend seam replaces the tmux-hardcoded `terminal_switch`; inside herdr, `n` and resume open a tab through one exec-direct `layout.apply` socket request, tmux behaves and reports exactly as before, and every other failure falls through to the GUI chain with `<backend>: <reason>` on the status line.**

## Performance

- **Tasks:** 2 of 2 (Task 1 tracer, Task 2 auto/tdd)
- **Commits:** 2 task commits (plus this SUMMARY commit)
- **Files:** 4 created, 5 modified, 1 deleted

## Accomplishments

- **Trait seam (MUX-01, partial):** `MultiplexerBackend` is object-safe (`name`, `detect`, `focus`, `open`; `focus` and `open` default to `MuxError::Unsupported`). Backends live in a static `REGISTRY` (tmux, then herdr). `Detection` keeps `inside` and `reachable` as separate states. The written design note is plan 33-02's deliverable (D-14), so MUX-01 is only fully closed after that plan.
- **One fail-closed argv sink (MUX-03):** every `open` takes an `&ExecArgv`, which `ExecArgv::new` refuses below 2 elements or with a `-`-leading head, with the original refusal text. The tmux direct-exec rustdoc moved onto the type. The cwd travels as tmux's `-c` element, herdr's JSON `cwd`, or GUI `current_dir`, never as a `cd` string.
- **herdr launch (MUX-05):** `layout_apply_request` is built with `serde_json` and carries the `ExecArgv` as the pane `command` array element for element. The transport copies the `driverctl_cli.rs` one-line pattern (2 s bounded connect on a helper thread, write timeout, 5 s reply deadline, 1 MiB cap). `herdr.rs` has no process spawn and is not on the spawn allowlist.
- **GUI fallback (MUX-06):** `launch_with` tries only the innermost detected backend, then `$TERMINAL`, `xdg-terminal-exec`, `x-terminal-emulator`, fallback list (D-02 order unchanged). Every fallthrough puts `<backend>: <reason>` on the status line; the tmux strings are byte-identical.
- **Focus (D-09):** `focus_with` tries each detected backend innermost-first, first success wins; none detected reads `Tab-switching requires running inside tmux or herdr (none detected)`; a single backend's reason is shown verbatim, several are name-prefixed. tmux `focus` carries the exact-pane-tty logic and its four `pane_tty_*` tests verbatim.
- **Call sites (MUX-07):** both launch sites in `detail.rs` call `multiplexer::launch`, both Tab sites call `multiplexer::focus_message`. `detail.rs` spawns nothing. `resumable_session_id` still runs before the seam.

## Verification

| Check | Result |
|---|---|
| Baseline lib tests (before Task 1) | 2930 passed, 1 ignored |
| Final `cargo test --no-fail-fast`, lib | 2959 passed, 0 failed, 1 ignored (all other suites green, incl. `driver_router_conformance`) |
| `cargo test --test spawn_seam_guard` | 10 passed |
| `every_claude_argv_option_site_under_src_is_adjudicated` | passed, `CLAUDE_ARGV_SITES` unedited |
| `launch_inside_herdr_opens_a_tab_through_layout_apply` (tracer) | passed |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo clippy --target aarch64-apple-darwin --all-targets -- -D warnings` | clean |
| `cargo clippy --target x86_64-pc-windows-gnu -- -D warnings` | clean |

Tracer feedback gate: the end-to-end `<verify>` set was re-run after Task 1 and passed before Task 2 expanded.

## Task Commits

1. **Task 1 (tracer): launch through the seam** - `149602b` (feat)
2. **Task 2: focus through the seam, delete the old module** - `7438f20` (feat)

## Deviations from Plan

None - plan executed as written. Two small points that stayed inside the plan's intent:

- Clippy `type_complexity` fired on the GUI spawner parameter and a test table; resolved with a `GuiSpawner<'a>` type alias and a `LaunchRow` test alias (no behaviour change).
- `order_innermost` also de-duplicates by backend name, so a registry slice that lists the same backend twice still yields it once (covers the MUX-01 adjacency truth).

## Known Stubs

None.

## Threat Flags

None. The new network-adjacent surface (the herdr Unix socket request) is the planned T-33-01/04/05 surface, mitigated as specified.

## Notes for the orchestrator

- STATE.md and ROADMAP.md were not touched, per the dispatch instruction.
- REQUIREMENTS.md was not touched: MUX-01's design-note half lands in plan 33-02, so checking it off is left to the orchestrator after the wave.
- The live-tmux direct-exec test (`multiplexer::tmux::tests::the_tmux_direct_exec_boundary_holds_against_a_live_server`) moved verbatim and stays `#[ignore]`d.

## Self-Check: PASSED

- Created files present: `src/multiplexer/mod.rs`, `tmux.rs`, `gui.rs`, `herdr.rs`. `src/terminal_switch.rs` absent.
- Commits `149602b` and `7438f20` are ancestors of HEAD.
- `grep -rn "crate::terminal_switch" src tests` prints nothing.
