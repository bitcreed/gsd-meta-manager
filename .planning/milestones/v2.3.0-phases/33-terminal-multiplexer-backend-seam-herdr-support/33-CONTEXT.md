# Phase 33: Terminal multiplexer backend seam + herdr support - Context

**Gathered:** 2026-11-10
**Status:** Ready for planning

> **Unattended run (`--auto`).** The user was unavailable. Every decision below was made by the
> orchestrator from REQUIREMENTS.md, ROADMAP.md, PROJECT.md, `src/terminal_switch.rs`, the five call
> sites and the local `herdr` 0.9.5 `--help` output. All are **[inferred]** and open to audit.
> No decision is one-way; none needed escalation.

<domain>
## Phase Boundary

Replace the tmux-hardcoded `src/terminal_switch.rs` with a reusable multiplexer backend trait
(detect, focus a running Claude session, open a new session in a cwd with an argv). Move tmux onto
it with unchanged behaviour, add a herdr backend, pick the innermost multiplexer when they are
nested, and keep the GUI-terminal fallback chain. The trait is designed against zellij, WezTerm mux,
kitty remote control, GNU screen, Rex and herdr-core/`hc`, but only tmux and herdr are implemented
(the others are MUX-F01..F06). Requirements: MUX-01..MUX-07.

Not in this phase: a config key to force a backend, or a TUI indicator showing the active backend
(MUX-F07). Controlling a multiplexer the TUI is not running inside. Windows backends.

</domain>

<decisions>
## Implementation Decisions

### Trait shape and module layout
- **D-01 [inferred]:** New module `src/multiplexer/` (`mod.rs`: trait, detection, shared argv sink,
  launch plan; `tmux.rs`; `herdr.rs`; `gui.rs` for `probe_terminals` and the GUI rank order).
  `src/terminal_switch.rs` is removed, not left as a re-export shim, so MUX-07's "no tmux call
  outside the tmux backend" holds by grep. — **Reversibility:** reversible — internal module; the
  `pub mod` in `src/lib.rs` is used only by in-tree tests.
- **D-02 [inferred]:** It is a real trait (`MultiplexerBackend`, or a better name the planner picks),
  object-safe, with backends in a static registry (`&[&dyn MultiplexerBackend]`), not an enum.
  The user asked for a trait other multiplexers can implement later. Methods:
  - `name()`.
  - `detect(&EnvSnapshot) -> Option<Detection>`. This is a pure function over an env snapshot,
    so it can be fixture-tested.
  - `focus(&ClaudeSession) -> Result<FocusOutcome, MuxError>`. `FocusOutcome` reports which
    identity matched (session id, tty, pid or cwd), because join keys differ per tool.
  - `open(cwd, &ExecArgv) -> Result<(), MuxError>`.

  Capabilities differ between tools: GNU screen has weak enumeration, kitty needs
  `allow_remote_control`, Rex and `hc` exist only behind a socket. So a backend can report
  `Unsupported` for any method, with a default implementation. "Inside" and "reachable" are
  separate states in `Detection`.
- **D-03 [inferred]:** `MuxError` separates three cases:
  - "not mine / not found": try the next backend.
  - "refused for safety": never retried by another path that would add an interpreter.
  - "failed": the reason is surfaced.

  The planner may shape this as an enum.
- **D-04 [inferred]:** Keep the existing split between a pure core and a thin impure edge. Output
  parsing (tmux `list-panes`, herdr `pane list` JSON) and all ordering decisions are pure functions,
  testable with no multiplexer installed. Commands run only at the edge. There is no generic
  "command runner" abstraction shared across backends; the reasoning of the old D-06 still applies
  to invocations. D-06's "only `in_tmux()` is shared" is superseded: detection is now the shared
  layer, by design.

### Shared fail-closed argv sink (MUX-03)
- **D-05 [inferred]:** The direct-exec refusals move off `open_new_window` and onto a validated
  newtype (`ExecArgv`, name the planner's call). It can only be built through the fail-closed check:
  refuse fewer than 2 elements, refuse a head that starts with `-`. Every backend's `open` takes
  `&ExecArgv`, so the type system enforces the boundary for every current and future backend.
  The cwd stays a separate argument and travels as an argv element (`-c` for tmux, `--cwd` for
  herdr), never as a `cd` in a program string (T-W0D-03). The refusal messages and the existing
  tests (D-03, T-W0D-01/03/04) carry over without being weakened. Backend-specific extra refusals,
  such as tmux's option-parser reason for a `-` head, can stay inside the backend.
- **D-06 [inferred]:** The in-tree argv builders `tmux_resume_argv` / `tmux_launch_argv` in
  `detail.rs` become backend-neutral, building one `ExecArgv` for whichever backend launches. They
  keep the leading literal `env` that makes `len() >= 2` true by construction.

### Detection and nesting precedence (MUX-02)
- **D-07 [inferred]:** Gate: a backend is a candidate only when its env marker is present
  (`$TMUX`; `HERDR_ENV=1`, with `HERDR_SOCKET_PATH` / `HERDR_PANE_ID`).
  - **Innermost rule:** walk the TUI's process ancestry. The nearest ancestor that is a candidate
    backend's server or pane host is the innermost one. Env vars alone cannot order a nesting,
    because both sets are inherited through it. A tmux pane's shell is a child of the tmux server,
    and a herdr pane's shell is a child of the herdr server, even under systemd.
  - **Tie-break:** when the ancestry is unavailable or inconclusive, a fixed static order applies,
    with **tmux first**. That keeps today's behaviour for every existing tmux user (SC2).
  - **Tests:** the ancestry walk takes a fixture list (pid, comm) next to the `EnvSnapshot`, so
    both the precedence rule and the tie-break are unit-tested with no live multiplexer.
  - **For research:** check whether the ancestry signal holds on macOS (no `/proc`; `ps -o ppid,comm`)
    and with herdr's server under systemd. If it does not, record the static order as the only rule
    instead of inventing another heuristic. — **Reversibility:** reversible — a pure function behind
    the detection API.
- **D-08 [inferred]:** Detection returns an ordered list, innermost first, and not just one backend.
  D-09 and D-10 use the order differently.

### Focus and launch flow, fallback, error surfacing (MUX-04/05/06)
- **D-09 [inferred]:** **Focus** tries each detected backend from innermost outward. The first match
  wins, and a backend's "not found" is not final. Example: herdr inside tmux, with the session in a
  tmux pane elsewhere. If nothing matches, the status line shows every backend's reason, innermost
  first. The GUI chain does not apply to focus, since there is nothing to fall back to; MUX-06's
  fallback chain is read as launch-only. With no backend detected, the message generalizes today's
  "requires running inside tmux" text to name the supported multiplexers.
- **D-10 [inferred]:** **Launch/resume** tries the innermost backend only, then the GUI chain. It
  never opens in an outer multiplexer the user is not looking at.
  - **One entry point:** the two duplicated "try tmux, then GUI" blocks in `detail.rs` (resume about
    `:4938`, new session about `:5485`) collapse into one seam function. It takes the `ExecArgv` for
    the multiplexer and the GUI argv builder, and returns the status message.
  - **Status text:** the message format becomes `"<backend>: <reason> — opened in <term>"`. Today's
    text is `"tmux: <reason> — opened in <term>"`, so tmux output stays byte-identical.
  - **Launch plan:** `LaunchPlan { try_tmux: bool, gui }` becomes an ordered backend list plus
    `gui`, still computed purely and unconditionally (D-01/D-02 rationale kept verbatim). `$TERMINAL`
    still ranks below any detected multiplexer, and the GUI rank order (D-02) is unchanged.
- **D-11 [inferred]:** **herdr focus join.**
  1. `agent_session.value` from `herdr pane list` equals the session's id (MUX-04).
  2. Fallback: exact tty equality, if herdr exposes a tty (`pane process-info`), with the same
     exact-match, no-substring rule as tmux.
  3. Last resort: cwd, but only when exactly one pane matches. An ambiguous cwd match is refused,
     never guessed. A wrong focus sends keystrokes to an unrelated session, the same rationale as
     `pane_tty_matches`.

  Focus then composes workspace → tab → pane with `herdr workspace focus` / `tab focus` / pane
  focus; research confirms the exact verbs.
- **D-12 [inferred]:** **herdr launch.** `herdr tab create --cwd <cwd> --focus` (cwd as an argv
  element), then run the `ExecArgv` in the new pane only through a path proven not to pass the
  vector through a shell. `herdr pane run` is the candidate; the herdr socket API is the alternative
  if the CLI types into a shell. If no such path exists, `open` returns "refused for safety" with
  that reason and the GUI chain runs (MUX-05 permits this outcome).
  - **Order of checks:** the safety decision is made before any tab is created. If a tab was created
    and the run step then fails, the backend closes that tab (`herdr tab close`), so no empty tab is
    left behind.
  - **Never:** `send-text` or `send-keys` of a command line. That is a shell parser by another name.
- **D-13 [inferred]:** herdr is reached through the CLI, using the inherited `HERDR_SOCKET_PATH`,
  with no socket client written in this phase. If research shows the CLI needs the socket path
  passed explicitly (systemd server), pass it as an argv or env element from the `EnvSnapshot`.

### Design note and verification (MUX-01, SC4)
- **D-14 [inferred]:** The design note is the module-level rustdoc of `src/multiplexer/mod.rs`, so it
  lives with the trait. It has a table mapping each surveyed tool (tmux, herdr, herdr-core/`hc`,
  Rex, zellij, WezTerm, kitty, screen) to `detect` / `focus` (and its join key) / `open`, and marks
  `Unsupported` where a tool cannot satisfy a method. Research may refine the REQUIREMENTS.md scan;
  the rustdoc is the canonical copy.
- **D-15 [inferred]:** Tests.
  - Existing `terminal_switch` tests and the `detail.rs` tests on `plan_launch` / `TerminalProbes` /
    argv refusal (about `:23850`, `:23972`, `:24093`) move with the code and are not weakened.
  - New tests: env + ancestry fixtures for detection, pure herdr JSON parse and join tests (including
    the ambiguous-cwd refusal), and launch-plan ordering.
  - No live-multiplexer test runs by default. An `#[ignore]` live herdr smoke test is optional.
  - herdr is installed on this machine (`~/.local/bin/herdr`), so UAT of SC1 can be run for real
    rather than escalated.

### Claude's Discretion
- Exact type and method names, the `MuxError` shape, and whether `EnvSnapshot` is a struct or a
  closure-backed lookup.
- Plan split. A natural cut is (1) seam + sink + tmux migration + call sites, (2) herdr backend,
  (3) design note + fixtures. The planner decides waves; (2) depends on (1).
- Whether `FocusOutcome`'s matched identity appears in the status message or only in the logs.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements and scope
- `.planning/REQUIREMENTS.md`: MUX-01..07, MUX-F01..F07, the ecosystem scan table, seam implications and out-of-scope.
- `.planning/ROADMAP.md` § Phase 33: goal, success criteria 1-4, open research questions.
- `.planning/PROJECT.md` § Current Milestone: v2.3.0 Multiplexer Backends.
- `.planning/musk/MILESTONE-v2.3.0.md`: subtraction pass (nothing cut).

### Code being replaced and its safety invariants
- `src/terminal_switch.rs`: `plan_launch` doc (D-01, D-02 rationale, kept verbatim), `open_new_window` doc (D-03, T-W0D-01/03/04, the direct-exec boundary), `pane_tty_matches` (exact-match rationale), tests.
- `src/ui/screens/detail.rs`: `resolve_launch_plan` (about `:2407`), the resume site (about `:4925-4990`), the Sessions focus site (about `:5448`), the new-session site (about `:5480-5530`), `tmux_resume_argv` / `tmux_launch_argv` / `resume_terminal_argv` / `launch_terminal_argv`, the tests at about `:23850`, `:23972`, `:24093`, `:28364`.
- `src/ui/screens/normal.rs`: the focus site (about `:840`).
- `src/session_detector.rs`: `ClaudeSession` (`session_id: Option<Untrusted>`, `tty`, `pid`, `working_dir`).

### External tool surface
- Local `herdr` 0.9.5 CLI: `herdr pane --help`, `herdr pane run --help`, `herdr tab create --help`, `herdr --skill`. Primary source for D-11/D-12 research. `tab create` has `--cwd`, `--env`, `--focus`, but no command argument.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `plan_launch` / `probe_terminals` / `TerminalProbes`: already a pure plan over gathered probes. Extend the shape, don't rewrite it.
- `pane_tty_matches` / `strip_dev_prefix` / `find_pane_target`: the tmux join, moved as-is into `tmux.rs`. The tty-equality helper is reused by herdr's tty fallback.
- `resumable_session_id` (detail.rs): already refuses non-Claude (Codex) rows before any spawn (T-lr9-01). Keep it ahead of the seam.

### Established Patterns
- Fail-closed refusals happen before any `Command` is constructed, so they are testable without the tool installed.
- Long "why" rustdoc on safety boundaries, with decision and threat IDs. Keep and extend it, don't trim.
- Status messages go back to the caller as `Result<_, String>` and are rendered in the status line.

### Integration Points
- Focus: `normal.rs:840` and `detail.rs:5448` call `switch_to_session`, becoming one seam `focus` call.
- Launch: `detail.rs:2407` (`resolve_launch_plan`), `:4938` and `:5485` (`open_new_window` + GUI fallthrough), collapsing into one seam launch function (D-10).

</code_context>

<specifics>
## Specific Ideas

- User intent: "a properly designed trait that can be reused for other frameworks too … at least
  consider it for the seam, implementation can be done later". So the trait shape is the main
  deliverable, alongside working herdr support. Don't overfit it to tmux + herdr.
- Real environment: the TUI runs inside a herdr pane (`HERDR_ENV=1`, `$TMUX` unset), with the herdr
  server under systemd. SC1 must work there.

</specifics>

<deferred>
## Deferred Ideas

- MUX-F01..F06 backends (zellij, WezTerm, kitty, screen, Rex, herdr-core/`hc`): designed for, not built.
- MUX-F07: a config override to force a backend, and a TUI indicator showing the active backend.
- Launch fallback into an *outer* multiplexer when the innermost one refuses. Rejected for now (D-10);
  revisit if herdr launch ends up always refusing.
- A native herdr socket client, if the CLI turns out to be insufficient (D-13).
- musk: no cuts this phase (the milestone and phase passes both kept everything).

### Reviewed Todos (not folded)
`todo.match-phase` returned 17 matches with score >= 0.4. All are keyword false positives ("src",
"tab", "run", "inferred") from unrelated subsystems: UI typography and help popup, queue input,
release-map badge, driver/driverctl hardening, launcher capture. None touches terminal switching or
multiplexers. **None was folded [inferred]:** the scope guardrail overrides the auto-fold threshold.
- Make queue input truly multi-line: ui, out of scope.
- Ambiguous-wide typography / DarkGray: ui, out of scope.
- TUI badge for per-repo release maps: ui, out of scope.
- Help popup scannable at 80x24: ui, out of scope.
- Launcher-marker scan in-flight flag: ui, out of scope.
- Unregister guard cached live check: ui, out of scope.
- D-17 recovery block with no DIRECTION items; driverctl fm_status port; retire test-only executor remnants; sandbox tmpfs for symlinked planning; hard link blocks driver stop; run-file open/size hardening; driverctl LOG.md redaction: driver/driverctl, out of scope.
- Capture-pump redaction edge cases; managed-policy shape validation; unsupervised child on capture failure: launcher, out of scope.

</deferred>

---

*Phase: 33-terminal-multiplexer-backend-seam-herdr-support*
*Context gathered: 2026-11-10*
