# API Coverage — Claude Code CLI stream-json control protocol (via the `Executor` trait)

> Full coverage by default. Opt-outs are explicit, reasoned decisions.

The deterministic detector returned `detected: false` over the ROADMAP Phase 18 section. This matrix
is produced anyway, because the phase is the **first consumer** of the duplex half of the transport
Phase 15 built, and "we integrated the API" would otherwise silently mean "we integrated whatever the
first use case exercised". The surface below is the `Executor` trait plus the stream-json envelope
fields this phase can reach.

## `Executor` trait surface

| capability | decision | reason |
|---|---|---|
| `start` | INTEGRATE | |
| `send` | INTEGRATE | The whole point of STEER-01; reached from the driver's inbox drain arm. |
| `close_input` | INTEGRATE | Relocated by D-11 from spawn time to a turn boundary with an empty inbox. |
| `is_running` | INTEGRATE | |
| `capabilities` | INTEGRATE | Unchanged from Phase 15; the gate still fires on the first `system/init` only. |
| `wait_outcome` | INTEGRATE | |
| `cancel` | INTEGRATE | Already reached by the existing stop path. |
| `interrupt` | OPT-OUT | Explicitly out of scope — deferred to Phase 20 (18-CONTEXT deferred list). STEER-01/02/03 are about *messages*; an interrupt affordance whose true state can only be known one turn later would be a second three-state UI. `interrupt_stopped_a_turn` remains its only honest reporter. |

## stream-json envelope fields this phase reads or writes

| capability | decision | reason |
|---|---|---|
| `user` message write shape (`UserMessage::text`) | INTEGRATE | Pinned byte-for-byte by `tests/executor_transport.rs:321`. |
| `isReplay` on echoed `user` messages | INTEGRATE | The dequeue ack; the only delivery evidence the protocol offers (D-07, D-08). |
| `assistant` / `user` message **body** content blocks | INTEGRATE | Newly modelled in 18-03, tolerantly: every field `Option` or defaulted. |
| `result` envelope (turn boundary, cumulative cost, terminal reason) | INTEGRATE | Projected readably in 18-03; a run has N turns and one outcome (Phase 15 D-29). |
| `system/init` (subsequent, post-first) | INTEGRATE | Rendered as informational; never as "the run restarted" (Phase 15 D-30). |
| `system/thinking_tokens` | INTEGRATE | Tolerated, never treated as an unknown-type error (Phase 15 D-32). |
| `control_request` / `control_response` (`still_queued`) | OPT-OUT | Not needed yet — reached only by `interrupt`, which is opted out above and owned by Phase 20. |
| `--max-budget-usd` post-turn circuit breaker | OPT-OUT | Explicitly out of scope — Phase 20's quota floor is the real cost control (Phase 15 D-16 / OQ3). |
| `ExecutionTarget::Container` transport | OPT-OUT | Explicitly out of scope — Phase 22. |
| `control.sock` fast-path transport | OPT-OUT | Not needed yet — deferred to a later milestone; the unit of work is minutes, so the durable file path is built first (ARCHITECTURE §5.5). |
| `tmux send-keys` as a control channel | OPT-OUT | Explicitly out of scope — a **named anti-feature** (D-01): driver and human keystrokes interleave on a shared pane and corrupt each other, there is no delivery confirmation, and it breaks mid-prompt. tmux stays attach/watch-only. |

## Package legitimacy

Not applicable. This phase adds **zero** package-manager installs — `VecDeque` is std and no Cargo
dependency is added — so no `[ASSUMED]` / `[SUS]` / `[SLOP]` package exists to verify.
