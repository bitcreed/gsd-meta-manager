# 260908-uqq — CONTEXT (diagnosis brief)

> This diagnosis is **already proven empirically**. Do not re-derive it. Treat every
> claim in "Root cause" and "Secondary defects" as locked input and go fix it.

## The bug

A driver run against a real project (`picsync`) ended after exactly **900.068 s** with
`outcome: "spawn_failed"`, having never delivered its command.

Evidence on disk:
`/home/blk/projects/python/picsync/.planning/meta-manager/runs/2026-09-09T03-30-04Z-aa49/`
— `journal.jsonl` holds exactly three records (`run_started`, the envelope `diagnostic`,
`run_ended`), with nothing from the child at all.

## Root cause (reproduced)

`src/executor/claude.rs::start_run` parks on `gate_rx.await` waiting for the child's first
`system/init`, and only releases the prompt to the child's stdin *after* the capability gate
passes (the D-02/D-06 "gate before prompt" ordering, `handle_item` at ~claude.rs:1618-1645).

**On the installed CLI (claude 2.1.266) `system/init` is not emitted at startup. It is emitted
when the first user message arrives on stdin.** Verified directly:

- `claude -p --input-format stream-json --output-format stream-json --verbose
  --session-id <uuid> --setting-sources project --permission-mode dontAsk --strict-mcp-config`
  with stdin held open and empty: **0 bytes on stdout, 0 bytes on stderr**, process stays alive
  as long as stdin is open, exits 0 on EOF.
- Same invocation with a user message written at +6 s: `system/init` arrives at **+6 s**,
  assistant message +7 s, result +7 s.

So it is a **deadlock**: the driver waits for `init`, the CLI waits for the prompt. Nothing
breaks the tie except the Coordinator's idle cap — `ExecutionOptions::default().idle_cap` =
15 min, which `iteration_options` (src/driver/run.rs:1656) never overrides. 900.068 s is that
cap to the millisecond. (The run's wall-clock cap was 14400 s, so it was NOT the wall cap.)

The repo's spike notes were taken on CLI 2.1.220; this is 2.1.266.

## Secondary defects that made it opaque (also in scope)

1. **Misclassification.** The idle breach fired pre-gate. The Coordinator *did* compute
   `RunOutcome::Stalled { idle_for }`, but it goes out on `outcome_tx`, which lives in an
   `ExecutionHandle` that is never returned on this path — so the caller only ever sees the
   gate's `Err(SpawnError::InitNeverObserved)` (claude.rs:1397) and `src/driver/run.rs:3003`
   records `spawn_failed`. A 15-minute stall is reported with the one word that means "the
   binary would not launch".

2. **No diagnostic record names the reason.** The error path at run.rs:~2998-3005 calls
   `finish_run(…, "spawn_failed", …)` then `return Err(DriveError::Spawn(err))` without
   journalling *which* `SpawnError` it was. The string "the process stream ended before any
   system/init event was observed" exists nowhere on disk.

3. **The reason has no reachable sink at all.** `src/main.rs:322` does
   `eprintln!("Error: {}", err)`, but `src/driver/spawn.rs` detaches the driver with all three
   stdio handles on `/dev/null` by design. And
   `~/.local/share/gsd-meta-manager/gsd-meta-manager.log.*` are all 0 bytes because
   `main.rs:75` uses `EnvFilter::from_default_env()` with `RUST_LOG` unset → ERROR-only, and
   nothing on this path logs at error level (the `tracing::warn!`s are filtered out).
   **The journal is the only sink that survives.**

## What to fix

**Primary (the acceptance test):** a driver run must actually get past startup and deliver its
command against CLI 2.1.266.

**Secondary (do these too if they stay small; drop and report if the primary balloons):** make
the failure legible — a pre-gate stall must not be reported as `spawn_failed`, and whatever the
terminal reason is must reach `journal.jsonl`.

## Constraints — locked, read before designing

- **This codebase is fanatical about not claiming more than it can prove.** The gate's whole
  documented justification is that a capability refusal "costs zero tokens and zero quota
  (D-06)" *because the prompt has not been released yet* — see the doc comments on
  `SpawnError::Capability`, `RunOutcome::CapabilityRefused`, and the comment at
  claude.rs:~663. If the fix releases the prompt before the gate verdict, that property is
  **no longer true** and every comment asserting it MUST be corrected in the same commit.
  Do not leave a stale guarantee in a doc comment. **An honest weaker claim beats a false
  strong one.**
- Do not silently widen or delete the capability gate. If it has to become a post-hoc
  validation that aborts, **say so in the code** and keep the refusal path working.
- `iteration_options` deliberately inherits `idle_cap` from `Default`; if a startup-specific
  bound is the right shape, add it **deliberately** rather than shrinking the run-wide idle cap.
- Preserve the pre-gate `pgid` publication (CR-01) and the `biased` terminate-arm ordering in
  run.rs — a stop landing during startup must still tear down the agent's process group.
- **Latent hazard — note it; fix only if cheap.** Pre-gate nothing drains `events_rx` (it is
  local to `start_run` until the handle is returned), so `read_stderr`'s `tx.send().await` on
  the 8192-slot channel can park — and because `last_line_at` is stamped *before* the send, a
  child that floods stderr before init would freeze the idle clock. Buffered stderr is also
  dropped when `events_rx` is dropped on the error path. If not cheap, just report it.
- **Test discipline: this repo proves things with tests, not comments.** Integration tests live
  under `tests/` (`driver_kill_startup.rs`, `spawn_seam_guard.rs`, …) and there is a checked-in
  shell stand-in reachable via the hidden `--claude-program` / `--claude-args` dev flags — use
  that stand-in to test the new startup handshake shape **without spending quota**.
- Verify clean before committing: `cargo build && cargo test && cargo clippy -- -D warnings`.
  This project's CLAUDE.md warns `rtk` filters cargo output — use `rtk proxy cargo …` when a
  check depends on reading raw output.

## Process constraints

- Work **directly in the primary working tree on `master`**. No git worktree.
- Commit atomically as `/gsd-quick` normally does.
