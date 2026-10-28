//! The agent-runtime config value and the `stream-json` wire parser.
//!
//! Until Phase 31 this module also drove a `claude` or `codex` child in
//! process: a transport, a capability gate, run-outcome derivation and the
//! TUI's executor event channel. All of that was reachable only through the
//! legacy `drive` path, which the Phase 28 sandboxed launcher replaced, and it
//! went with it. Two submodules remain, and each has a live user:
//!
//! * `runtime` holds [`runtime::AgentRuntime`], the parsed value of the
//!   per-project `runtime` key and the `default_runtime` preference. It selects
//!   nothing to drive; it stays because existing configs name it.
//! * `stream_json` holds the wire model for Claude Code's `stream-json` output,
//!   which the rate-limit fixtures parse transcripts through.

pub mod runtime;
pub mod stream_json;
