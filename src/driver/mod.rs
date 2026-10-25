//! The surviving driver modules: the pure readers and classifiers the launcher,
//! `driverctl`, the Driver tab and `next --json` build on, plus the two Unix
//! process primitives the launcher still uses.
//!
//! Phase 31 retired the legacy Phase 17 `drive` subcommand (a detached child of
//! this binary that ran one GSD command through the in-process executor) and
//! everything only it used. What is left is what a Phase 27-30 path calls:
//!
//! - [`direction`] and [`steer`] own `DIRECTION.md`: the frontmatter split, the
//!   atomic exchange and the `control` steering edits (Phase 27/30);
//! - [`router`] chooses the next GSD command from observed project state with no
//!   model call, and is the contract behind `next --json` (Phase 20/29);
//! - [`rate_limit`] classifies a quota event off the stream (Phase 17/28);
//! - [`liveness`] is the `/proc` probe behind a run's "is it alive" answer;
//! - [`kill`] and [`lock`] are the process-group signal and the launcher's run
//!   lock. Only the helpers the launcher calls remain (`signal_group`,
//!   `acquire_at`, `probe_at` and their types); the legacy
//!   `.planning/meta-manager/runs/` entry points went with `drive`.
//!
//! Nothing here runs an agent. The launcher (`crate::launcher`) is the only
//! path that starts one.

// Outside the `#[cfg(unix)]` block: the portable helpers of [`direction`]
// (frontmatter split, body sanity, parse) compile everywhere, and the
// `renameat2(RENAME_EXCHANGE)` functions are gated inside the module itself
// (D-12, D-13). Shared by the driverctl writer and Phase 30's steering edits.
pub mod direction;
// Pure body readers (and later transforms) beside [`direction`], for the same reason:
// no file, no clock, no platform facility, so it compiles everywhere (D-06, D-15).
pub mod steer;
// Pure, and outside the block for that reason: [`router`] opens no file, spawns
// no process and reads no clock — the caller captures the state and hands it in
// (D-11) — so there is nothing platform-specific in it to gate, and gating it
// would put the module whose determinism is the router's central claim out of
// reach of a non-Unix build's type checker.
pub mod bounds;
pub mod router;
// Also pure, for the identical reason: [`rate_limit`] reads no clock and opens no
// file — the caller retains the payload off the stream and hands in the instant
// to judge a reset time against — so gating it would put the classification that
// decides whether a run parks on a shared quota out of reach of a non-Unix
// build's type checker.
pub mod escalate;
pub mod goal;
pub mod rate_limit;
pub mod untrusted;
// [`liveness`] is a `/proc` **read**, and D-10 is explicit that it should carry
// `src/session_detector.rs`'s honest-failure posture — that module has no
// platform attributes at all and its reads simply yield nothing off Linux —
// rather than diverging from the module it copies. [`kill`] and [`lock`] are
// `#[cfg(unix)]` because a process-group signal and a `flock` have no portable
// form.
#[cfg(unix)]
pub mod kill;
pub mod liveness;
#[cfg(unix)]
pub mod lock;
