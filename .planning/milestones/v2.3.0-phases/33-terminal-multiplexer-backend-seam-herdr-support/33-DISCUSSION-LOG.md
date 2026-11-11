# Phase 33: Terminal multiplexer backend seam + herdr support - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md. This log preserves the alternatives considered.

**Date:** 2026-11-10
**Phase:** 33-terminal-multiplexer-backend-seam-herdr-support
**Mode:** `--auto` (user unavailable; every selection was made by the orchestrator and is [inferred])
**Areas discussed:** Trait shape & layout, Shared argv sink, Nesting precedence, Focus/launch fallback, herdr join & launch, Design note & tests

---

## Trait shape & layout

| Option | Description | Selected |
|--------|-------------|----------|
| Object-safe trait + static registry in `src/multiplexer/` | Reusable by future backends; the user asked for a trait | ✓ |
| Enum dispatch | Simpler, but every new backend edits a central match | |
| Keep `terminal_switch.rs` as a re-export shim | Eases churn, but weakens the MUX-07 grep check | |

**Choice:** trait + registry; `terminal_switch.rs` removed (D-01..D-04).

## Shared argv sink

| Option | Description | Selected |
|--------|-------------|----------|
| Validated `ExecArgv` newtype taken by every backend's `open` | Type-enforced fail-closed boundary | ✓ |
| Shared check function each backend calls | A new backend can forget to call it | |

**Choice:** newtype (D-05, D-06).

## Nesting precedence

| Option | Description | Selected |
|--------|-------------|----------|
| Env gate + process-ancestry innermost + static tmux-first tie-break | Env alone cannot order a nesting; tmux-first keeps current behaviour | ✓ |
| Env-only static order | Deterministic, but wrong for herdr-in-tmux or tmux-in-herdr | |
| `$TMUX_PANE` / `HERDR_PANE_ID` heuristics | Inherited values go stale across a nesting | |

**Choice:** ancestry with a static tie-break (D-07, D-08). Research verifies macOS and the systemd herdr server.

## Focus/launch fallback

| Option | Description | Selected |
|--------|-------------|----------|
| Focus: all detected backends innermost-first; launch: innermost only, then GUI | Focus is safe to retry; launch should appear where the user is | ✓ |
| Both innermost-only | Misses a session in an outer multiplexer | |
| Launch also tries outer backends | Opens where the user may not be looking | |

**Choice:** D-09, D-10. The duplicated launch blocks collapse into one seam function, and the status text format is unchanged for tmux.

## herdr join & launch

| Option | Description | Selected |
|--------|-------------|----------|
| Join on `agent_session.value`, then exact tty, then cwd only when unique; launch via `tab create --cwd` + proven direct-exec run, otherwise refuse + GUI | Never focuses the wrong session; never adds an interpreter | ✓ |
| `send-text` a command line | It is a shell parser; violates the no-interpreter invariant | |
| cwd match picks the first pane | Can focus an unrelated session | |

**Choice:** D-11..D-13. Risk: if `herdr pane run` types into a shell and the socket API has no exec path, herdr launch always falls back to GUI. MUX-05 permits that, so it was not escalated.

## Design note & tests

| Option | Description | Selected |
|--------|-------------|----------|
| Module rustdoc table in `src/multiplexer/mod.rs` | Lives with the trait | ✓ |
| Separate `.planning` doc | Drifts from the code | |

**Choice:** D-14, D-15.

## Claude's Discretion

Names, the `MuxError` shape, `EnvSnapshot` form, the plan split, and whether the matched identity appears in the status message.

## Deferred Ideas

- MUX-F01..F07.
- Outer-multiplexer launch fallback.
- A native herdr socket client.
- 17 todo matches reviewed and none folded. All were keyword false positives; the scope guardrail overrode the auto-fold threshold.
