//! The `stream-json` wire parser.
//!
//! Until Phase 31 this module also drove a `claude` or `codex` child in
//! process: a transport, a capability gate, run-outcome derivation and the
//! TUI's executor event channel. All of that was reachable only through the
//! legacy `drive` path, which the Phase 28 sandboxed launcher replaced, and it
//! went with it. One submodule remains:
//!
//! * `stream_json` holds the wire model for Claude Code's `stream-json` output.
//!   It is test-only today (see the pending todo
//!   `2026-10-28-retire-test-only-executor-remnants`).
//!
//! The `AgentRuntime` value that used to live here as `runtime` was removed by
//! quick 261029-1dp: the legacy `runtime` and `default_runtime` config keys are
//! kept in the flattened `extra` maps and written back unchanged, and select
//! nothing.

pub mod stream_json;
