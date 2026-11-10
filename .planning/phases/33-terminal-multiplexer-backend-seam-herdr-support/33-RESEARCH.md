# Phase 33: Terminal multiplexer backend seam + herdr support - Research

**Researched:** 2026-11-10
**Domain:** Rust process orchestration / terminal multiplexers (tmux 3.6, herdr 0.9.5 CLI + socket API) / Linux process ancestry
**Confidence:** HIGH for herdr and tmux behaviour (measured live on this machine). MEDIUM for the design-note rows of the unimplemented tools (docs and secondary sources only).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
(Verbatim from `33-CONTEXT.md` § Implementation Decisions. All are marked **[inferred]** by the orchestrator.)

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

### Deferred Ideas (OUT OF SCOPE)
- MUX-F01..F06 backends (zellij, WezTerm, kitty, screen, Rex, herdr-core/`hc`): designed for, not built.
- MUX-F07: a config override to force a backend, and a TUI indicator showing the active backend.
- Launch fallback into an *outer* multiplexer when the innermost one refuses. Rejected for now (D-10);
  revisit if herdr launch ends up always refusing.
- A native herdr socket client, if the CLI turns out to be insufficient (D-13).
- musk: no cuts this phase (the milestone and phase passes both kept everything).

Not in this phase (from § Phase Boundary): a config key to force a backend, or a TUI indicator showing
the active backend (MUX-F07). Controlling a multiplexer the TUI is not running inside. Windows backends.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| MUX-01 | Trait covering detect / focus / open, designed against every tool in the ecosystem scan, with the design recorded | § Design-note table (refined scan); § Trait shape |
| MUX-02 | Innermost multiplexer wins when nested; deterministic, unit-tested from env fixtures | § Empirical finding E4/E5: env alone is insufficient for tmux-in-herdr; ancestry walk gives the order; herdr scrubs `$TMUX` |
| MUX-03 | tmux on the trait with unchanged behaviour; direct-exec refusals on the shared sink | § ExecArgv sink; verbatim refusal text in § Code being replaced |
| MUX-04 | Focus in herdr lands in the pane; join on `agent_session.value`, cwd/tty fallback | § E2 (pane list JSON), E6 (focus verbs). **Correction:** herdr exposes no tty; use pid. `session_id` is `None` for sessions not started with `--resume` |
| MUX-05 | Launch/resume in a new herdr tab without an interpreter; else refuse + fall back | § E3: `pane run` is **VIA-SHELL** (injection reproduced); `layout.apply` over the socket is **EXEC-DIRECT** |
| MUX-06 | GUI chain runs on no-backend or failure; reason reaches the user | § Launch flow; status strings quoted verbatim |
| MUX-07 | All five call sites through the seam; no tmux call outside the tmux backend | § Call-site inventory; `tests/spawn_seam_guard.rs` allowlist must change |
</phase_requirements>

## Summary

**`herdr pane run` goes through a shell.** It joins the argv with spaces, without quoting, and types the result into the pane's interactive shell. The live probe below reproduced command injection: an element `'x;touch INJECTED'` created `INJECTED`, and `$HOME` expanded. So the CLI path D-12 named as the candidate is unsafe, and MUX-05 forbids it. `herdr agent start` uses the same typed-into-the-shell model (`--pane` must be "at an interactive shell prompt").

**`layout.apply` over the socket execs directly.** It is a raw socket method with no CLI wrapper in 0.9.5. Each pane node takes `command: [string]`, `cwd` and `env`. One argv element containing spaces, `;`, `$HOME` and `*` arrived as one literal file name. The spawned process's parent is `herdr server` itself, with no shell in between. The request also creates the tab and runs the program atomically, so D-12's "close the empty tab on failure" step is unnecessary. An unresolvable head returns a synchronous `layout_apply_failed` error and creates no tab.

**Scope fork, flagged for audit.** D-12 names the socket API as the alternative if the CLI types into a shell. D-13 and the deferred list say "no socket client in this phase". Recommendation [inferred]: write one narrow `std::os::unix::net::UnixStream` request for `layout.apply` only. That is one NDJSON line out, one line back, with a timeout, and no new crate. Keep list and focus on the CLI per D-13. This satisfies SC1's launch half; the alternative of refusing and falling back to the GUI satisfies MUX-05 but fails SC1. It is reversible.

**Nesting.** Env markers cannot order tmux inside herdr: both `$TMUX` and `HERDR_ENV=1` are present (measured). herdr inside tmux is easy, because herdr strips `$TMUX` from its panes (measured). Walking `/proc/<pid>/stat` ppid from the TUI upward and taking the first ancestor whose `comm` is `tmux: server` or `herdr` gives the innermost backend in every case measured. The walk must be nearest-first: a freshly started herdr server is a child of its client, so both `herdr` and `tmux: server` can appear in one chain. Each server usually daemonizes to `systemd --user`, so typically only the innermost server appears. `HERDR_SOCKET_PATH` is always injected into herdr panes, so it never needs to be passed explicitly.

**Primary recommendation:**
- Build `src/multiplexer/` per D-01..D-15.
- tmux: move the code verbatim behind the trait.
- herdr focus: `herdr pane list` → join → `herdr agent focus <pane_id>`. Verified to switch workspace, tab and pane in one call. `herdr tab focus <tab_id>` is the fallback.
- herdr open: one socket `layout.apply` request with `command = ExecArgv`.
- Detection: env gate, then a pure nearest-first ancestry rule over a `(pid, comm)` fixture, then the static order with tmux first.

## Empirical Settlement (orchestrator directive)

All runs were on this machine, inside herdr pane `wA:p1`. Every throwaway tab, named session and private tmux server created below was closed, and the closure was verified. No existing tab or pane was touched, and the user's focused tab (`wB:t1`) was never changed. All probe tabs were created with `--no-focus` / `"focus": false`.

### E1. Environment and versions [VERIFIED: live]
```
$ env | grep -E '^(HERDR|TMUX|TERM_PROGRAM)'
HERDR_PANE_ID=wA:p1
TERM_PROGRAM_VERSION=0.9.5
HERDR_BIN_PATH=/home/blk/.local/bin/herdr
HERDR_ENV=1
HERDR_TAB_ID=wA:t1
HERDR_SOCKET_PATH=/home/blk/.config/herdr/herdr.sock
HERDR_WORKSPACE_ID=wA
TERM_PROGRAM=herdr
$ herdr --version            -> herdr 0.9.5
$ herdr status               -> server: running, version 0.9.5, private_protocol: 22, socket: /home/blk/.config/herdr/herdr.sock
$ tmux -V                    -> tmux 3.6
```
`$TMUX` is unset.

### E2. CLI surface (help output, verbatim excerpts) [VERIFIED: live `herdr <group>` / `--help`]
```
$ herdr pane run --help
Run a command in a pane
Usage: herdr pane run <PANE_ID> <COMMAND>...

$ herdr tab create --help
Usage: herdr tab create [OPTIONS]
      --workspace <WORKSPACE_ID>   --cwd <PATH>   --label <TEXT>
      --env <KEY=VALUE>  Set an environment variable for the launched process
      --focus   --no-focus

$ herdr pane      (relevant lines)
  herdr pane list [--workspace <workspace_id>]
  herdr pane process-info [--pane ID|--current]
  herdr pane focus --direction left|right|up|down [--pane ID|--current]     <- direction only, no pane-id focus
  herdr pane run <pane_id> <command>
$ herdr tab       ->  herdr tab focus <tab_id>   herdr tab close <tab_id>
$ herdr workspace ->  herdr workspace focus <workspace_id>
$ herdr agent     ->  herdr agent focus <target>
                      targets accept unique agent names and pane ids that currently host agents
$ herdr layout    ->  unknown command: layout
```
`herdr --skill` (214 lines) says verbatim: *"`pane run` queues command text and Enter together; success acknowledges admission, not command execution."* The 0.9.5 socket-API docs say verbatim: *"`pane.send_input` (used by `pane run`)"* [CITED: raw.githubusercontent.com/herdrdev/herdr/v0.9.5/docs/next/website/src/content/docs/socket-api.mdx].

`herdr pane list` JSON per pane, with the keys observed live (abridged from a real Claude pane):
```json
{"agent":"claude",
 "agent_session":{"agent":"claude","kind":"id","source":"herdr:claude","value":"0e4f6238-39fc-4d98-9a18-b7156e96bd4b"},
 "agent_status":"idle","cwd":"/home/blk/projects/flutter/bitcreed-apps","focused":false,
 "foreground_cwd":"/home/blk/projects/flutter/bitcreed-apps","pane_id":"w1:p1","revision":3,
 "scroll":{...},"tab_id":"w1:t1","terminal_id":"term_65fdd7c960fcd1",
 "terminal_title":"...","terminal_title_stripped":"...","workspace_id":"w1"}
```
The envelope is `{"id":"cli:pane:list","result":{"panes":[...]}}`. `agent` and `agent_session` are **absent** on non-agent panes. **There is no `tty` field anywhere.**

`herdr pane process-info --current`, verbatim:
```json
{"id":"cli:pane:process_info","result":{"process_info":{"foreground_process_group_id":3620262,"foreground_processes":[{"argv":["claude"],"cmdline":"claude","cwd":"/home/blk/projects/rust/gsd-meta-manager","name":"claude","pid":3620262}],"pane_id":"wA:p1","shell_pid":3620011},"type":"pane_process_info"}}
```
This output has no tty either, so D-11's tty fallback cannot be built from herdr data. What is available is the **pid** (`foreground_processes[].pid`, `shell_pid`).

Latency: `herdr pane list` and `pane process-info` each took under 10 ms (`/usr/bin/time` printed `0.00s`).

### E3. EXEC-DIRECT or VIA-SHELL

**Probe A, `herdr pane run`: VIA-SHELL, unquoted (command injection).**
```
$ herdr tab create --workspace "$HERDR_WORKSPACE_ID" --cwd "$P" --label gsdmm-probe1 --no-focus
  -> {"result":{"root_pane":{"pane_id":"wA:p2",...},"tab":{"tab_id":"wA:t2",...},"type":"tab_created"}}
$ herdr pane process-info --pane wA:p2
  -> foreground_processes:[{"argv":["/usr/bin/fish"],...,"pid":3863612}], "shell_pid":3863612
$ herdr pane run wA:p2 env printf '%s@@' 'a b' '$HOME' 'x;touch INJECTED' '*'
  [pane run exit 0]
$ herdr pane read wA:p2 --source recent-unwrapped --lines 20
blk@mediabox /t/c/-/0/s/probe1> env printf %s@@ a b $HOME x;touch INJECTED *
a@@b@@/home/blk@@x@@⏎
$ ls $P
INJECTED  zz-glob-witness
```
An exec-direct path would have printed `a b@@$HOME@@x;touch INJECTED@@*@@` and created no `INJECTED`. Instead:
- The elements were space-joined with their quoting stripped.
- The line was typed into fish.
- `$HOME` expanded.
- `;` ran a second command.
- `*` globbed.

Process ancestry of a `pane run` child confirms a shell sits in between:
```
$ herdr pane run wA:p2 env sleep 41   # then walk /proc/<pid>/stat
pid=3864898 comm=sleep ppid=3863612 cmdline=sleep 41
pid=3863612 comm=fish ppid=671927 cmdline=/usr/bin/fish
pid=671927 comm=herdr ppid=2384 cmdline=/home/blk/.local/bin/herdr server
pid=2384 comm=systemd ppid=1 cmdline=/usr/lib/systemd/systemd --user
```
**Verdict: `herdr pane run` is VIA-SHELL.** For MUX-05 it is forbidden, along with `pane send-text` / `send-keys` (D-12) and `agent start`, which also needs "an existing available shell pane" at its prompt.

**Probe B, socket `layout.apply` with an argv `command`: EXEC-DIRECT.** Request sent as one NDJSON line to `$HERDR_SOCKET_PATH`:
```json
{"id":"gsdmm_probe2","method":"layout.apply","params":{"workspace_id":"wA","tab_label":"gsdmm-probe2","focus":false,
 "root":{"type":"split","direction":"right","ratio":0.5,
  "first":{"type":"pane","cwd":"<probe2>","command":["env","touch","--","lit a b;touch INJECTED2 $HOME *"]},
  "second":{"type":"pane","cwd":"<probe2>","command":["env","sleep","43"]}}}}
```
Response, verbatim (paths shortened):
```json
{"id":"gsdmm_probe2","result":{"type":"layout_apply","layout":{"workspace_id":"wA","tab_id":"wA:t3","zoomed":false,"focused_pane_id":"wA:p3","root":{"type":"split",...,"first":{"type":"pane","pane_id":"wA:p3",...},"second":{"type":"pane","pane_id":"wA:p4",...}}}}}
```
Result:
```
$ ls -1b <probe2>
lit\ a\ b;touch\ INJECTED2\ $HOME\ *        <- ONE file, literal name; no INJECTED2
zz-glob-witness
$ (ancestry of the sleep pane's process; process-info reports shell_pid == the sleep pid)
pid=3865722 comm=sleep ppid=671927 cmdline=sleep 43
pid=671927 comm=herdr ppid=2384 cmdline=/home/blk/.local/bin/herdr server
```
**Verdict: `layout.apply` with `command: [argv]` is EXEC-DIRECT.** The program is a direct child of `herdr server`, its argv arrives element-for-element, and there is no shell. Further observations:
- **The pane closes when the program exits** (`herdr pane get wA:p3` → `pane_not_found` once `touch` exited), and the tab closes with its last pane. That matches tmux `new-window` behaviour without `remain-on-exit`.
- **Spawn failure of the head is synchronous and leaves nothing behind.** `command:["gsdmm-nonexistent-xyz","a"]` returns `{"id":"nx","error":{"code":"layout_apply_failed","message":"Unable to spawn gsdmm-nonexistent-xyz because:\nNo viable candidates found in PATH \"...\""}}`, and no tab is created.
- **With a leading `env`, a missing program is not reported.** `command:["env","gsdmm-nonexistent-xyz"]` returns success and the tab then vanishes. This is the same blind spot today's tmux path has.
- **Exec context.** `PATH` is the **herdr server's** `PATH`, not the TUI's. Measured: the server lacks `~/.claude/gsd-core/bin`, which the TUI has. `HERDR_*` are injected into the new process (`HERDR_PANE_ID=wA:p4`, `HERDR_SOCKET_PATH`, …).
- The docs agree: *"`layout.apply` creates a fresh tab from a declarative tree … This restores structure, labels, cwd, env, and optional argv commands"* [CITED: socket-api.mdx v0.9.5].

Cleanup: `herdr tab close wA:t2` → `{"result":{"type":"ok"}}`. `wA:t3` had already auto-closed (`tab_not_found`). `herdr tab list --workspace wA` then listed only `wA:t1`, and the re-check after the later probe also showed only `wA:t1`.

### E4. Process ancestry from this process (herdr, systemd) [VERIFIED: live /proc walk]
```
pid=3871370 comm=[bash]   ppid=3620262
pid=3620262 comm=[claude] ppid=3620011 cmdline=claude
pid=3620011 comm=[fish]   ppid=671927  cmdline=/usr/bin/fish
pid=671927  comm=[herdr]  ppid=2384    cmdline=/home/blk/.local/bin/herdr server
pid=2384    comm=[systemd] ppid=1      cmdline=/usr/lib/systemd/systemd --user
```
- The "herdr under systemd" in the planning notes is actually a daemonized server reparented to `systemd --user`, not a systemd unit. Its cgroup is `.../app.slice/ptyxis-spawn-....scope`.
- The **herdr client is never in a pane's ancestry**; only the server is. The client and the server both have `comm` = `herdr`, and only the cmdline (`herdr server`) tells them apart.

### E5. Nesting: what env and ancestry look like [VERIFIED: live, private tmux sockets + isolated named herdr session]

**tmux inside herdr.** `tmux -L gsdmm-nest-… new-session -d … "sleep 60"`, started from this herdr pane:
```
pid=3868470 comm=[sleep] ppid=3868469
pid=3868469 comm=[tmux: server] ppid=2384 exe=/usr/bin/tmux
pid=2384 comm=[systemd] ...
pane env: HERDR_PANE_ID=wA:p1 TMUX=/tmp/tmux-1000/gsdmm-nest-3868466,3868469,0 TMUX_PANE=%0
          HERDR_ENV=1 HERDR_SOCKET_PATH=/home/blk/.config/herdr/herdr.sock TERM_PROGRAM=tmux ...
```
→ **Both** markers are present, so env alone cannot pick. The ancestry has one multiplexer, `tmux: server`, which is the correct innermost.

**herdr inside tmux.** A private tmux server with `HERDR_*` unset, running `herdr --session gsdmm-nestprobe`:
```
pid=3869884 comm=[bash]  ppid=3869860                 (herdr pane shell)
pid=3869860 comm=[herdr] ppid=3869852 cmdline=/home/blk/.local/bin/herdr server
pid=3869852 comm=[herdr] ppid=3869851 cmdline=herdr --session gsdmm-nestprobe   (client)
pid=3869851 comm=[tmux: server] ppid=2384
herdr pane env: HERDR_ENV=1 HERDR_PANE_ID=w1:p1 HERDR_SESSION=gsdmm-nestprobe
                HERDR_SOCKET_PATH=/home/blk/.config/herdr/sessions/gsdmm-nestprobe/herdr.sock
                TERM_PROGRAM=herdr     (no TMUX, no TMUX_PANE)
```
→ **herdr strips `$TMUX`/`$TMUX_PANE` from its panes**, so env alone already says herdr. A freshly spawned herdr server is a **child of its client**, so here the chain holds both `herdr` and `tmux: server`; the nearest one is the innermost. The walk must therefore be **nearest-first, first hit wins**, not "is X anywhere in the chain".

Cleanup: `herdr session stop gsdmm-nestprobe` / `delete`, `tmux -L … kill-server`, stale socket files removed. `herdr session list` shows only `default`, and `/tmp/tmux-1000/` holds only `default`.

**Conclusions for MUX-02 / D-07:**
1. The env gate is necessary but not sufficient: tmux-in-herdr carries both markers.
2. A nearest-first ancestry walk is sufficient on Linux in all three measured shapes: herdr alone, tmux-in-herdr, herdr-in-tmux.
3. `TERM_PROGRAM` names the innermost in both nestings (`tmux` vs `herdr`, measured). It is a further env-only signal that also works on macOS, but D-07 says not to invent another heuristic. Recommendation [inferred]: **do not** use it in this phase, and record it in the design note as a candidate. It is also fragile: tmux older than 3.2 does not set it, and an inner tmux would then inherit `TERM_PROGRAM=herdr` [ASSUMED for the version boundary].
4. **macOS:** `/proc` does not exist. `ps -o ppid=,comm= -p <pid>` exists on BSD `ps` [ASSUMED], but would add a spawn site. Session detection in this tree is Linux-only (`session_detector.rs` reads `/proc` for everything, and `NoProcessProbe` is used off-Linux), so on macOS focus never has a session to act on. Recommendation [inferred], per D-07's own fallback clause: Linux uses the `/proc` walk; non-Linux uses the static order with tmux first, recorded as the only rule there.

### E6. herdr focus composition [VERIFIED: live, isolated headless named session `gsdmm-focusprobe`, never the user's session]
Setup: workspaces w1 (A) and w2 (B); w2 has tabs t1 and t2; w2:t2 is split into p2 and p3. Focus was read from `herdr api snapshot`.
```
start:                                   focused w1 / w1:t1 / w1:p1
herdr tab focus w2:t2                 -> focused w2 / w2:t2 / w2:p2   (switches WORKSPACE too; lands on tab's last-focused pane)
herdr workspace focus w1              -> w1 / w1:t1 / w1:p1
raw socket {"method":"pane.focus","params":{"pane_id":"w2:p2"}}
                                      -> w2 / w2:t2 / w2:p2   (any pane, all three levels)
(pane report-agent w2:p3 ... --agent probe --state idle)
herdr agent focus w2:p3 (from w1)     -> w2 / w2:t2 / w2:p3   (exact pane, all three levels, one call)
herdr agent focus w2:p2 (no agent)    -> {"error":{"code":"agent_not_found","message":"agent target w2:p2 not found"},...}
```
- **There is no `workspace → tab → pane` sequence to build.** `herdr agent focus <pane_id>` does all three levels in one call when the pane hosts a detected agent. Every Claude pane in the live `pane list` carried `"agent":"claude"`.
- Fallback when the pane is not agent-hosted: `herdr tab focus <tab_id>`. It is exact only when the tab has one pane, and that partial result should be reported.
- The raw socket `pane.focus` focuses any pane exactly. Use it only if the socket helper (§ E3) exists anyway; that is the planner's call. The CLI-first default honours D-13.

### E7. Does `HERDR_SOCKET_PATH` need to be passed explicitly? No. [VERIFIED: live + CITED]
- Herdr injects `HERDR_SOCKET_PATH` into every pane process: in panes created by the CLI, by `layout.apply` (E3) and by a named session (E5).
- CLI resolution order, from the docs: *"1. explicit CLI `--session <name>` 2. `HERDR_SOCKET_PATH` 3. `HERDR_SESSION=<name>` 4. default session socket"* [CITED: socket-api.mdx v0.9.5].
- So the TUI, which inherits its pane's env, reaches the right server. Passing it explicitly from the `EnvSnapshot` (`Command::env("HERDR_SOCKET_PATH", …)`) is harmless and makes the edge testable. Recommended for the socket helper, which must read it anyway.
- Under tmux-in-herdr the tmux server's frozen `HERDR_SOCKET_PATH` is still the live socket while the herdr server lives. A dead socket surfaces as a CLI exit 1 with JSON on stderr, which maps to "failed".

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Env snapshot + ancestry collection | Impure edge (`multiplexer::detect` collector) | — | Only place reading `std::env` / `/proc`; everything after is pure |
| Innermost ordering (MUX-02) | Pure core (`multiplexer::mod`) | — | Function of `(EnvSnapshot, &[(pid, comm)])`, fixture-tested |
| Argv safety (MUX-03/05) | Shared sink type `ExecArgv` | Backend-specific extra refusals | Type makes the boundary unbypassable for every backend |
| tmux list/select/new-window | `multiplexer::tmux` (impure edge + pure parser) | — | Moved verbatim from `terminal_switch.rs` |
| herdr pane list parse + join | `multiplexer::herdr` pure core | — | serde_json over captured stdout; exact-match rules |
| herdr CLI calls (list, process-info, agent/tab focus) | `multiplexer::herdr` impure edge | — | D-13 |
| herdr `layout.apply` socket request | `multiplexer::herdr` impure edge (cfg(unix)) | — | Only exec-direct launch path (E3) |
| GUI terminal probing + rank | `multiplexer::gui` | `detail.rs` keeps per-emulator argv builders + separator table | D-02 order unchanged |
| Status-message composition | Seam function in `multiplexer::mod` | Call sites in `normal.rs` / `detail.rs` | D-10: one entry point; tmux text byte-identical |

## Standard Stack

No new crates. Everything uses crates already in `Cargo.toml` [VERIFIED: Cargo.toml read this session] or `std`.

| Need | Use | Why |
|------|-----|-----|
| herdr JSON parse | `serde_json = "1"` + `serde` derive (present) | Tolerant parse (`#[serde(default)]`, no strict unknown-field rejection, which `tests/spawn_seam_guard.rs` forbids in `src/`) |
| Socket request | `std::os::unix::net::UnixStream` + `set_read_timeout` / `set_write_timeout` | One NDJSON line each way; no tokio needed. Call sites are synchronous key handlers |
| Ancestry | `std::fs::read_to_string("/proc/<pid>/stat")`, `/proc/<pid>/comm` | Same parsing rule as `session_detector::parse_stat_starttime` (comm closed by the LAST `)`) |
| Spawning | `std::process::Command` (`.output()`) | As today |

**Package Legitimacy Audit:** not applicable. This phase installs no external packages. Packages removed: none. Flagged: none.

## Architecture Patterns

### System flow
```
key press (Tab / Enter / n)
   │
   ▼
call site (normal.rs:840, detail.rs:5448 focus | detail.rs:4938, :5485 launch)
   │   resumable_session_id gate stays BEFORE the seam (T-lr9-01)
   ▼
multiplexer::collect_env() ──► EnvSnapshot {TMUX, HERDR_ENV, HERDR_SOCKET_PATH, HERDR_PANE_ID, HERDR_WORKSPACE_ID, ...}
multiplexer::collect_ancestry() ──► Vec<(pid, comm)> nearest-first (Linux /proc; empty elsewhere)
   │
   ▼
detect_ordered(env, ancestry)  [PURE]  ──► Vec<&dyn MultiplexerBackend> innermost-first
   │                                      (env gate → nearest-first ancestry → static order tmux, herdr)
   ├── FOCUS: for b in ordered { b.focus(session) } first Ok wins; else join all reasons
   │          (none detected → "requires running inside tmux or herdr")
   └── LAUNCH: ExecArgv::new(argv)? ── ordered.first().open(cwd, &argv)
                 ├─ Ok → "Launched…"/"Resumed…"
                 └─ Err(reason) → plan.gui → spawn GUI → "<backend>: <reason> — opened in <term>"
                                     └─ none → "<backend>: <reason>; no terminal emulator found (set $TERMINAL)"
herdr.focus: CLI `pane list` → pure join (agent_session → pid via process-info → unique cwd) → CLI `agent focus <pane_id>` (fallback `tab focus <tab_id>`)
herdr.open : socket NDJSON layout.apply {workspace_id, tab_label?, focus:true, root:{type:pane, cwd, command: argv}}
tmux.focus : `tmux list-panes -a -F …` → find_pane_target (exact tty) → select-window + select-pane
tmux.open  : `tmux new-window -c <cwd> <argv…>`
```

### Recommended structure
```
src/multiplexer/
├── mod.rs     # design-note rustdoc (D-14 table), trait, EnvSnapshot, Detection, MuxError, ExecArgv,
│              # detect_ordered (pure), ancestry collector, LaunchPlan/plan_launch, focus/launch seam fns
├── tmux.rs    # in_tmux-equivalent detect, switch (list-panes/select-*), open (new-window), pure parsers + tests
├── herdr.rs   # detect, pane-list parse + join (pure), CLI edge, socket layout.apply edge (cfg(unix))
└── gui.rs     # TerminalProbes, probe_terminals, GUI rank order (D-02) — moved verbatim
```
`src/lib.rs:34` `pub mod terminal_switch;` becomes `pub mod multiplexer;`.

### Pattern: ExecArgv, a validated newtype (D-05)
```rust
// Refusal text and checks carried VERBATIM from src/terminal_switch.rs:189-205 (see quote below).
pub struct ExecArgv(Vec<String>);          // no pub field, no Default, no From<Vec<String>>
impl ExecArgv {
    pub fn new(argv: Vec<String>) -> Result<Self, MuxError /* RefusedForSafety */> {
        if argv.len() < 2 { /* "refusing to open a tmux window from a {}-element program vector: …" */ }
        if argv[0].starts_with('-') { /* "…whose program vector begins with {:?}: tmux's own option parser…" */ }
        Ok(Self(argv))
    }
    pub fn as_slice(&self) -> &[String] { &self.0 }
}
```
The existing tests assert `err.contains("directly")` and `err.contains("option")` [VERIFIED: src/terminal_switch.rs:446-465]. Keep both words in the shared messages. Generalizing the wording from "a tmux window" to "a multiplexer window" is allowed, provided those substrings survive. Byte-identical is safer for SC2 [inferred].

### Pattern: pure innermost ordering (D-07/D-08)
```rust
pub enum BackendId { Tmux, Herdr }
/// ancestry: nearest-first (parent of self first). comm as in /proc/<pid>/comm.
pub fn order_innermost(env: &EnvSnapshot, ancestry: &[(u32, String)]) -> Vec<BackendId> {
    let mut candidates: Vec<BackendId> = STATIC_ORDER.iter().copied()      // [Tmux, Herdr]
        .filter(|b| b.env_gate(env)).collect();                           // TMUX non-empty; HERDR_ENV=="1" && HERDR_SOCKET_PATH non-empty
    if let Some(first) = ancestry.iter().find_map(|(_, comm)| BackendId::from_comm(comm)) // "tmux: server"/starts_with("tmux") -> Tmux; "herdr" -> Herdr
        .filter(|b| candidates.contains(b)) {
        candidates.retain(|b| *b != first);
        candidates.insert(0, first);
    }
    candidates   // inconclusive ancestry ⇒ static order (tmux first)
}
```
Fixtures to pin, all from E4/E5 measurements:
- herdr only, chain `[fish, herdr, systemd]` → `[Herdr]`.
- tmux-in-herdr, both markers, chain `[sleep|shell, tmux: server, systemd]` → `[Tmux, Herdr]`.
- herdr-in-tmux, herdr markers only, chain `[bash, herdr, herdr, tmux: server]` → `[Herdr]`.
- Both markers, chain has both with herdr nearer → `[Herdr, Tmux]`.
- Both markers, empty ancestry (macOS or unreadable) → `[Tmux, Herdr]`.
- An ancestor named `herdr` but no HERDR env → excluded, because the gate wins.

### Pattern: herdr focus join (MUX-04, D-11 corrected)
1. **`agent_session.value == session.session_id`** (exact, via `Untrusted::as_raw_for_logic_only()`). Only rows with `agent_session.kind == "id"` count. `session.session_id` is populated **only** when the Claude process was started with `--resume` [VERIFIED: src/session_detector.rs `read_session_id`, "The ONE place a session id enters this build: the token after another process's `--resume`"]. A plain `claude` (this very pane: cmdline `claude`, herdr `agent_session.value = 0cd7998f-…`) has `session_id == None`, so **step 1 misses most fresh sessions** and step 2 is the common path.
2. **pid** (replaces D-11's tty step; herdr exposes no tty, E2). For candidate panes, i.e. `agent == "claude"` and `foreground_cwd` or `cwd` equal to `session.working_dir`, run `herdr pane process-info --pane <id>`. Match when `session.pid` ∈ `foreground_processes[].pid` **or** `shell_pid` is `session.pid` or one of its `/proc` ancestors, which reuses the ancestry walker. Exact integer equality only. Measured: wA:p1 `foreground_processes[0].pid == 3620262 ==` the claude pid, `shell_pid 3620011 ==` its parent.
3. **Unique cwd** last resort. If more than one pane matches, refuse with "ambiguous"; never guess.
4. Focus with `herdr agent focus <pane_id>`. On `agent_not_found`, use `herdr tab focus <tab_id>`, and report when the tab has more than one pane.

`FocusOutcome::Matched { by: SessionId | Pid | Cwd }`.

### Pattern: herdr open (MUX-05) via one socket request
```rust
#[cfg(unix)]
fn layout_apply(socket: &Path, workspace: Option<&str>, cwd: &Path, argv: &ExecArgv) -> Result<(), MuxError> {
    let req = serde_json::json!({
        "id": "gsdmm-open",
        "method": "layout.apply",
        "params": { "workspace_id": workspace, "focus": true,
                    "root": { "type": "pane", "cwd": cwd, "command": argv.as_slice() } }
    });
    let mut s = std::os::unix::net::UnixStream::connect(socket)?;      // -> Failed
    s.set_write_timeout(Some(Duration::from_secs(2)))?; s.set_read_timeout(Some(Duration::from_secs(5)))?;
    s.write_all(format!("{req}\n").as_bytes())?;
    // read ONE line (bounded, e.g. 1 MiB), parse {"id","result"} | {"id","error":{"code","message"}}
}
```
Schema facts [VERIFIED: `herdr api schema --json`, protocol 22]:
- Request envelope: `id` (string, required), `method`, `params`.
- `LayoutApplyParams`: `root` (required), `focus` (default false), `tab_id`, `tab_label`, `workspace_id` (string|null).
- Pane `LayoutNode`: `type:"pane"` (required), `command` (array of string|null), `cwd`, `env` (object), `label`, `pane_id`.
- Error envelope: `{"id","error":{"code","message"}}`.
- Transport: *"Herdr uses newline-delimited JSON over a local socket. On Unix, that socket is a Unix domain socket"* [CITED: socket-api.mdx v0.9.5].

`workspace_id`: pass `HERDR_WORKSPACE_ID` from the snapshot so the tab lands in the TUI's own workspace [inferred]. When it is omitted, herdr uses the *focused* workspace.

### Anti-patterns to avoid
- **`herdr pane run` / `send-text` / `send-keys` / `agent start` for launch.** All of them type into a shell (E3).
- **Shell-quoting the argv for the target shell** (fish ≠ bash ≠ zsh). That still puts an interpreter in the path, which MUX-05 forbids.
- **"Is herdr anywhere in the ancestry?"** Use nearest-first; E5 shows chains holding both.
- **A generic command-runner trait across backends.** D-04 forbids it.
- **Reading `~/.claude/sessions/<pid>.json` to recover session ids.** That would add a second session-id entry point against D-21-48 ("the ONE place a session id enters this build"). Out of scope.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON encode/parse for herdr | String formatting / manual parsing | `serde_json` (present) | Untrusted titles and paths contain quotes and unicode |
| Shell-safe command line for `pane run` | A quoting function | `layout.apply` argv over the socket | Quoting is per-shell; any parser violates MUX-05 |
| `/proc/<pid>/stat` ppid parse | `split(' ')` | The "last `)`" rule already in `session_detector::parse_stat_starttime` (ppid is the 2nd field after comm) | comm may contain spaces and `)` |
| A herdr socket *client* (subscriptions, reconnect) | Full client | One request/one line, with timeouts | Only `layout.apply` needs the socket (E3) |

## Code being replaced: verbatim facts the planner must preserve

- `LaunchPlan` [VERIFIED: src/terminal_switch.rs:54-60]: `pub struct LaunchPlan { pub try_tmux: bool, pub gui: Option<String>, }`.
- `TerminalProbes` fields [VERIFIED: src/terminal_switch.rs:31-43]: `in_tmux: bool`, `terminal_env: Option<String>`, `xdg_terminal_exec_ok: bool`, `x_terminal_emulator_present: bool`, `fallback_present: Vec<String>`.
- Refusal messages [VERIFIED: src/terminal_switch.rs:189-205]: `"refusing to open a tmux window from a {}-element program vector: tmux execs a shell-command given as MULTIPLE arguments directly, but hands a SINGLE argument to a command interpreter — which would put a parser back in a path that deliberately has none"` and `"refusing to open a tmux window whose program vector begins with {:?}: tmux's own option parser would consume it as one of its options instead of treating it as the program"`.
- Focus-outside-tmux text [VERIFIED: src/terminal_switch.rs:273]: `"Tab-switching requires running inside tmux (no $TMUX set)"`. Generalize per D-09.
- Status strings [VERIFIED: src/ui/screens/detail.rs:5003, 5019-5021, 5005, 5496, 5517, 5528]: `format!("tmux: {} — opened in {}", e, term)`, `"tmux: {e}; no terminal emulator found (set $TERMINAL)"`, `format!("Resumed session {}", short_id)`, `"Launched new Claude session"`, `format!("Failed to launch: {}", e)`, `"No terminal emulator found (set $TERMINAL)"`.
- Focus call shape [VERIFIED: src/ui/screens/normal.rs:840-843]: `match crate::terminal_switch::switch_to_session(&s) { Ok(()) => format!("Switched to {}", alias), Err(e) => e, }`.
- `ClaudeSession` [VERIFIED: src/session_detector.rs:45-58]: `pid: u32`, `kind: SessionKind`, `session_id: Option<Untrusted>`, `working_dir: PathBuf`, `start_time: Option<u64>`, `tty: Option<String>`. Its doc names `crate::terminal_switch` (lines 53-54), so update the doc comment.
- `tmux_resume_argv` / `tmux_launch_argv` [VERIFIED: src/ui/screens/detail.rs:2658-2672] both begin `vec!["env".to_string()]`.

### Call-site inventory (MUX-07) [VERIFIED: grep `terminal_switch` this session]
`src/lib.rs:34` (mod decl). `src/ui/screens/normal.rs:840` (focus). `src/ui/screens/detail.rs:2388, 2404` (doc links), `:2407-2410` (`resolve_launch_plan`), `:4945` (resume open), `:5448` (focus), `:5490` (new-session open), `:23850, :23972` (doc links in tests), `:24093` (`use crate::terminal_switch::{plan_launch, TerminalProbes}` in a test). `src/session_detector.rs:53` (doc). `tests/spawn_seam_guard.rs:61` (allowlist).

## Common Pitfalls

### P1. `tests/spawn_seam_guard.rs` fails on the move
**What:** `SPAWN_ALLOWLIST` lists `"src/terminal_switch.rs"` (line 61). The test fails in **both** directions [VERIFIED: tests/spawn_seam_guard.rs:220-233]: an unexpected new spawn file, or an allowlisted file that no longer spawns ("vanished").
**Avoid:** in the same commit, remove the `terminal_switch.rs` entry and add each `src/multiplexer/*.rs` that contains `Command::new(` (`tmux.rs`, `herdr.rs`, `gui.rs` because of `which`/`xdg-terminal-exec`, and `mod.rs` only if it spawns), each with a one-line reason. The socket `connect` is not a spawn marker.

### P2. `session_id` is usually `None` (agent_session join silently never fires)
See § herdr focus join. The UAT for SC1 must cover **a freshly started `claude`**, not only a resumed one.

### P3. Tests reaching the live edge from inside herdr
`cargo test` run in a herdr pane inherits `HERDR_ENV=1`. Any test that drives the Enter/`n` key path past the seam with a Claude session would **create a real herdr tab running `env claude …`**. Existing tests avoid this by using Codex rows (e.g. `enter_on_a_session_row_after_descending_still_resumes` sets `SessionKind::Codex`). **Avoid:** detection and ordering take an injected `EnvSnapshot` + ancestry. Composed-flow tests use a fake backend list. No test constructs the real registry with the process env.

### P4. herdr server PATH ≠ TUI PATH
`layout.apply` resolves `env` (and `env` resolves `claude`) on the **herdr server's** `PATH` (E3). tmux has the same property today. Keep parity and inject no `PATH` [inferred]. Note the residual in rustdoc.

### P5. Leading `env` hides "claude not found"
With `["env","claude",…]`, a missing `claude` returns success and the tab vanishes (E3). This is the same as tmux today, and the parity is accepted. Do not drop `env`: D-06 needs it for len ≥ 2.

### P6. UI-thread blocking
The call sites are synchronous key handlers. herdr CLI calls take under 10 ms (E2). The socket read needs a timeout (≤5 s) so a wedged server cannot freeze the TUI. process-info runs once per candidate pane, so bound the candidate set by cwd.

### P7. `agent focus` needs a detected agent
It returns `agent_not_found` when herdr does not classify the pane as an agent (E6). Fall back to `tab focus`.

### P8. comm-based ancestry pitfalls
- `comm` is truncated to 15 bytes. `tmux: server` (12 bytes) and `herdr` fit.
- A tmux *client* (`tmux: client`) is never a pane ancestor.
- Unreadable `/proc` entries mean stopping the walk and using the static order. Never guess.
- Cap the walk depth, for example at 64.

## Design-note table (D-14, refined ecosystem scan)

| Tool | detect (env gate) | focus / join key | open in cwd with argv | Exec-direct? | Notes |
|---|---|---|---|---|---|
| tmux | `$TMUX` non-empty; ancestor comm `tmux: server` [VERIFIED] | `list-panes -a -F '#{pane_tty} …'` → exact tty → `select-window`+`select-pane` | `new-window -c <cwd> <argv…>` | Yes when ≥2 args [CITED: man tmux, quoted in terminal_switch.rs:159-162] | Implemented |
| herdr TUI 0.9.5 | `HERDR_ENV=1` + `HERDR_SOCKET_PATH`; ancestor comm `herdr` [VERIFIED] | `pane list` → `agent_session.value` / pid via `pane process-info` / unique cwd → `agent focus <pane_id>` (fallback `tab focus`) [VERIFIED] | socket `layout.apply` `{root:{type:pane,cwd,command:[…]}}` [VERIFIED] | `layout.apply`: **yes**; `pane run`: **no** (VIA-SHELL) [VERIFIED] | Implemented. Strips `$TMUX` from panes [VERIFIED] |
| herdr-core / `hc` | TBD; separate server (systemd user service on remotes), not the TUI's [CITED: secondary sources] | `hc ls --json` [CITED] | `hc run --json --cwd <dir> -- <cmd>` [CITED] | Unknown [ASSUMED argv after `--`] | MUX-F06; OpenRPC schema via `hc schema` [CITED] |
| Rex (Superlogical) | Unknown | `rex` CLI with `--json`, terminal API returns process JSON [CITED: secondary] | CLI can create sessions/splits, run commands [CITED: secondary] | Unknown | MUX-F05; closed-source public beta. Socket transport undocumented |
| zellij | `$ZELLIJ` / `$ZELLIJ_SESSION_NAME` [ASSUMED]; `$ZELLIJ_PANE_ID` [CITED: zellij.dev cli-actions] | `zellij action list-panes -j` / `list-clients` → `focus-pane-id <id>` [CITED] | `zellij action new-pane --cwd <cwd> -- <argv>` (`zellij run` is shorthand) [CITED] | Likely (argv after `--` replaces shell) [ASSUMED] | MUX-F01 |
| WezTerm mux | `$WEZTERM_PANE` [CITED: wezterm.org cli/spawn] | `wezterm cli list --format json` → `activate-pane --pane-id` [ASSUMED] | `wezterm cli spawn --cwd <cwd> -- <argv>` ("Instead of executing your shell, run PROG") [CITED] | Likely [ASSUMED] | MUX-F02 |
| kitty remote control | `$KITTY_WINDOW_ID` [CITED]; reachable only with `allow_remote_control` + `KITTY_LISTEN_ON` [CITED: sw.kovidgoyal.net/kitty/remote-control] | `kitten @ ls` JSON (id, cwd, pid, cmdline per window) [CITED] → `focus-window --match id:<n>` | `kitten @ launch --type=tab --cwd <cwd> <argv>` [ASSUMED for `--cwd`] | Likely [ASSUMED] | MUX-F03. "Inside" ≠ "reachable" |
| GNU screen 4.09 | `$STY` [CITED: man screen] | `screen -Q windows` (weak, no tty/pid) → `-X select <n>` | `-X chdir <dir>` then `-X screen <cmd> <args>` [CITED: man screen "this command (with the given arguments) is started"] | `-X` text is re-tokenized by screen's own command parser [ASSUMED] → treat as a parser; likely `open` = `Unsupported`/refuse | MUX-F04; focus-only |

Candidate signal recorded but unused: `TERM_PROGRAM` names the innermost (`tmux`/`herdr`, E5).

## Runtime State Inventory (refactor: `terminal_switch` → `multiplexer`)

| Category | Items Found | Action |
|----------|-------------|--------|
| Stored data | None. No config key, cache or file stores the module name or backend choice (MUX-F07 deferred). Verified by grep of `src/` for `terminal_switch`: only code/doc references | none |
| Live service config | None. Nothing is registered with tmux or herdr | none |
| OS-registered state | None | none |
| Secrets/env vars | Reads `TMUX`, `TERMINAL`, `HERDR_ENV`, `HERDR_SOCKET_PATH`, `HERDR_PANE_ID`, `HERDR_WORKSPACE_ID`; nothing renamed | none |
| Build artifacts | `tests/spawn_seam_guard.rs` allowlist path (P1); `src/lib.rs` `pub mod` | code edit |

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| herdr | herdr backend, SC1 UAT | ✓ | 0.9.5 (server protocol 22) | — |
| tmux | tmux backend, SC2 UAT, ignored live test | ✓ | 3.6 | — |
| GNU screen | (design only) | ✓ | 4.09.01 | — |
| zellij / wezterm / kitty / rex / hc | (design only) | ✗ | — | Docs only |
| Rust toolchain | build | ✓ | rustc/cargo 1.99.1 (MSRV 1.89) | — |
| docker | pre-tag container check (release only) | ✓ | 29.1.3 | podman |

## Security Domain

| ASVS Category | Applies | Control |
|---|---|---|
| V5 Input Validation | yes | `ExecArgv` constructor (len ≥ 2, head not `-`); session id stays `Untrusted`, compared exactly; herdr JSON parsed tolerantly; no substring matching |
| V12/V14 Command execution | yes | No interpreter in any launch path (tmux ≥2-arg direct exec; herdr `layout.apply` argv); cwd as argv/JSON field, never a `cd` string |
| V2/V3/V4/V6 | no | Local same-user socket and CLI; no auth or crypto |

| Threat | STRIDE | Mitigation |
|---|---|---|
| Command injection through `herdr pane run` (reproduced, E3) | Tampering/EoP | Never use `pane run`/`send-*`/`agent start` for launch; `layout.apply` only |
| Wrong-pane focus sends keystrokes to an unrelated session | Tampering | Exact id/pid equality; ambiguous cwd refused (T-LR8-01 rationale) |
| Session-id option injection (`--resume=<id>` fusion, CWE-88) | Tampering | Unchanged: fused element built in `detail.rs`; travels as one JSON array element |
| Hung herdr server freezes the TUI | DoS | Socket read/write timeouts; bounded response read |
| Spoofed `comm` in ancestry | Spoofing | Same-user only; the env gate must also hold; worst case is the wrong-but-real backend, which still execs directly |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | tmux < 3.2 does not set `TERM_PROGRAM` | E5 | Only matters if `TERM_PROGRAM` is later adopted |
| A2 | BSD `ps -o ppid=,comm=` works on macOS | E5 | None: recommendation uses the static order off-Linux |
| A3 | zellij/WezTerm/kitty argv after `--` is exec-direct; screen `-X` is re-parsed | Design table | Design note only; re-verify in MUX-F01..F04 |
| A4 | Claude keeps the same session id under `--resume=<id>`, so herdr's `agent_session.value` equals the TUI's id | herdr focus join | Step 1 misses; step 2 (pid) still matches |
| A5 | Using the socket for `layout.apply` is acceptable despite D-13 / deferred "native socket client" | Summary | If rejected: herdr `open` returns RefusedForSafety and the GUI chain runs, so SC1's launch half is unmet |
| A6 | `foreground_processes` lists the claude pid while Claude runs tool subprocesses | herdr focus join | The `shell_pid`-is-ancestor clause still matches |

## Open Questions

1. **Socket for launch (A5).** This is the one decision that needs audit. Recommendation: adopt it, narrowly. It is D-12's own named alternative and reversible.
2. **Status-text generalization.** Keep tmux byte-identical (`"tmux: …"`) and use `"herdr: …"` for herdr per D-10. Whether the focus success text should name the matched identity is at the planner's discretion (D-15).
3. **`workspace_id` for the new tab.** `HERDR_WORKSPACE_ID` (the TUI's workspace) vs the focused workspace. Recommendation: the TUI's, retrying without it on `workspace_not_found` [inferred].

## Sources

### Primary (HIGH)
- Live `herdr` 0.9.5: `--help`, `--skill`, group help, `pane list`, `pane process-info`, `tab create/close`, `pane run`, `pane read`, `agent focus`, `tab focus`, `api schema --json`, `api snapshot`, and raw socket `layout.apply` / `pane.focus` / `ping` (outputs above).
- Live tmux 3.6 on private `-L` sockets; `/proc/<pid>/{stat,comm,cmdline,environ,cgroup}` walks.
- Repo files read this session: `src/terminal_switch.rs`, `src/session_detector.rs:25-58`, `src/ui/screens/detail.rs` (2610-2709, 4998-5023, 5483-5530), `src/ui/screens/normal.rs:836-845`, `tests/spawn_seam_guard.rs`, `Cargo.toml`, `.planning/REQUIREMENTS.md`, `33-CONTEXT.md`.

### Secondary (MEDIUM)
- herdr v0.9.5 docs: https://raw.githubusercontent.com/herdrdev/herdr/v0.9.5/docs/next/website/src/content/docs/socket-api.mdx and cli-reference.mdx.
- https://zellij.dev/documentation/cli-actions.html, https://wezterm.org/cli/cli/spawn.html, https://sw.kovidgoyal.net/kitty/remote-control/, `man screen` (4.09.01).

### Tertiary (LOW)
- Rex: https://www.bitdoze.com/superlogical-rex/, https://dev.classmethod.jp/en/articles/reona-superlogical-rex-beta-try/.
- herdr-core/hc: https://redock.dev/blog/herdr-tui-vs-herdr-core/, https://dev.classmethod.jp/en/articles/reona-herdr-app-herdr-core/, https://github.com/omacom/omarchy-pkgs/pull/1883.

## Metadata
**Confidence:** stack HIGH (no new deps); herdr/tmux behaviour HIGH (measured); detection HIGH on Linux, static fallback elsewhere; design table rows for unimplemented tools MEDIUM/LOW.
**Research date:** 2026-11-10. **Valid until:** about 2026-12-10 (herdr ships often; re-check `layout.apply` if the server protocol moves past 22).
