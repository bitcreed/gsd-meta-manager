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

pub mod counters;
pub mod log;
pub mod protocol;
pub mod scan;
#[cfg(target_os = "linux")]
pub mod serve;
#[cfg(target_os = "linux")]
pub mod store;
