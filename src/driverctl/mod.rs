//! `driverctl`: the single writer of a run's state, plus the tool-observed counting rules.
//!
//! This module root carries the facts every later plan in the phase builds
//! against, each with the decision that settled it:
//!
//! 1. **driverctl is the single writer of a run's `state.json`, `LOG.md` and
//!    `DIRECTION.md` frontmatter (DCTL-01).** The store that enforces this
//!    arrives in a later plan; the LOG grammar it writes with ([`log`]) and the
//!    counters it consults ([`counters`]) land first so they can be proven alone.
//! 2. **Every counting rule is a pure function over recorded evidence, replay-
//!    tested offline (D-18).** A rule is ported from the spike-003 Python tools
//!    (`gapcount.py`, `exitclass.py`) and pinned by committed copies of the
//!    spike's own results under `tests/fixtures/driverctl/spike003/`. No test
//!    spawns `python3`, and nothing here reads a clock.
//! 3. **Gap rounds come only from `gaps_found` VERIFICATION.md writes on disk,
//!    coalesced per the 26.1 rule (D-08, D-09, D-26).** This module owns the
//!    counter and its scan; Phase 28 owns only the gate, trip and kill that
//!    read `caps.gap_rounds.per_phase`. A count is never derived from the
//!    driver's own account of itself (its LOG lines, its `show` output or any
//!    value it sends): the cap the launcher enforces must not be self-reported.
//! 4. **There is one status definition, the `fm_status` port, deliberately not
//!    the dashboard's `VerificationStatus::from_raw` (D-27, todo filed).** The
//!    two disagree on quoting and on a BOM-prefixed file, and a counter that
//!    read the dashboard's definition would miss a `status: "gaps_found"` write
//!    the spike counted.
//! 5. **All code here is synchronous: never call it from an `async fn`.** It
//!    does blocking file I/O (the scan) and, in later plans, `flock`; the
//!    mechanical guard in `tests/async_blocking_guard.rs` walks `src/` for the
//!    shape this would take.
//! 6. **The store is gated by the run lock by type.** `RunStore::open` takes the
//!    `RunLock` by value (D-20, Pattern 2), so a store without the lock cannot be
//!    built and the lock is released only when the store is dropped.
//! 7. **Every request is validated in full before anything is written**, so a
//!    rejection writes nothing (D-04): the event is applied to a clone of the
//!    state and committed only after LOG.md and `state.json` are written.
//! 8. **The frontmatter is a derived view regenerated from `state.json`**, never
//!    LLM-written (D-19); the store emits it through the typed
//!    `DriverFrontmatter::emit` and splices it into `DIRECTION.md`.
//! 9. **`driverctl serve` is the standalone entry point over the same store
//!    ([`serve`], D-03).** It takes the run lock before any socket operation
//!    (D-20), never steals a live run's socket (D-21), and is proven end to end by
//!    `tests/driverctl_serve.rs` and, for SC1, by the replay of three recorded
//!    spike runs in `tests/driverctl_replay.rs` (D-01).
//!
//! # Phase 28 hand-off
//!
//! What the launcher (Phase 28) must do to embed this module, each with the
//! decision that makes it a requirement:
//!
//! 1. **Seed the PreToolUse `Agent|Task` hook with `"onFailure": "block"`** (D-25).
//!    The client closes every failure it can observe; a missing or killed binary
//!    exits non-zero, which Claude Code treats as a non-blocking error unless the
//!    hook says otherwise.
//! 2. **Bind `/dev/null` over `<run-dir>/launcher.lock` and
//!    `<run-dir>/DIRECTION.lock` inside bwrap**, written as the flag
//!    `--ro-bind /dev/null <path>` for each (D-25). `serve` and the store create
//!    both files before the bwrap argv is built, so the bind has a target. Without
//!    it a sandboxed process could take either `flock` and starve the single
//!    writer.
//! 3. **Hold the run lock across `fresh-exit` relaunches** (R3, D-25). `end
//!    --reason fresh-exit` records `running`; releasing `launcher.lock` between
//!    fresh-per-item launches makes the reader derive `crashed` in the gap.
//!    Acquire the lock before writing `running` and write the terminal status
//!    before releasing it (R4).
//! 4. **Embed [`store::RunStore`] on a dedicated `std::thread` rather than
//!    spawning `serve`** (D-03, Pattern 2). A spawned `serve` would be refused by
//!    the launcher's own run lock. The launcher acquires the lock, builds the
//!    store with `RunStore::open`, and runs the accept loop it needs on that
//!    thread.
//! 5. **Pass [`serve::socket_path`]`(run_id, run_dir)` as `DRIVERCTL_SOCK`**
//!    (D-21). It is the one helper that applies the private-directory rules and
//!    the 107-byte limit; do not rebuild the path by hand.
//! 6. **Install the cap gate with [`store::RunStore::set_gate`]**, reading
//!    `caps.gap_rounds.per_phase` (D-26). This module counts gap rounds from disk
//!    and calls the gate; denying at limit + 1 and killing the scope at limit + 2
//!    is Phase 28's.
//! 7. **At launch end call [`counters::classify_exit`] and
//!    [`store::RunStore::set_terminal`]** (D-06, D-17). `crashed` and `stalled`
//!    are launcher verdicts and reach the store through no socket request.
//! 8. **Put a `driverctl` shim on the sandbox `PATH` that execs
//!    `gsd-meta-manager driverctl "$@"`** so the MASTER-PROMPT runs unchanged
//!    (SC1).
//! 9. **Drive [`store::RunStore::scan_tick`] and
//!    [`store::RunStore::flush_heartbeat`] about every 2 s** (D-26, D-11), as
//!    `serve`'s ticker does.
//!
//! # Phase 30 hand-off
//!
//! What the driver tab (Phase 30) must respect:
//!
//! 1. **Steering edits use `driver::direction::write_body` under
//!    `DIRECTION.lock`** (D-13), never a plain write: the splice and a steering
//!    edit share that one lock so neither loses the other's bytes.
//! 2. **Live `control` is read from the DIRECTION body, not the frontmatter
//!    echo.** The frontmatter is a derived view and lags the body until the next
//!    event regenerates it.
//! 3. **The watcher must debounce or ignore `state.json` and `.DIRECTION.md.fm.*`**
//!    (Pitfall 9). Every event rewrites `state.json`, and every splice creates and
//!    renames a temp; an unfiltered watcher redraws on its own writer's churn.
//! 4. **The derived status comes from
//!    [`state_reader::driver_frontmatter::read_driver_run`](crate::state_reader::driver_frontmatter::read_driver_run)**
//!    (D-22), which corrects the written status with the lock probe. Do not
//!    display `status:` straight from the file.

pub mod counters;
pub mod log;
pub mod protocol;
pub mod scan;
#[cfg(target_os = "linux")]
pub mod serve;
#[cfg(target_os = "linux")]
pub mod store;
