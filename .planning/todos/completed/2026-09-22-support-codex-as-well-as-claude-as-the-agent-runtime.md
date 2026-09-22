---
created: 2026-09-22T16:52:25.454Z
title: Support Codex as well as Claude as the agent runtime
area: driver
severity: minor
completed: 2026-09-22
resolved_by: quick 260922-hdj MVP slice (5eb7d2a..862c038), batch 260922-hdg; remainder in todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md
files:
  - src/executor/claude.rs
  - src/executor/mod.rs
  - src/executor/stream_json.rs
  - src/session_detector.rs
  - src/driver/run.rs
---

## Problem

The manager only knows how to drive, detect and parse Claude Code sessions. GSD itself
supports other runtimes (OpenAI Codex among them — gsd-tools resolves `~/.codex/gsd-core`),
so a user running GSD under Codex gets no executor, no session detection and no stream
parsing. That cuts against the "portable, any GSD user" constraint in CLAUDE.md.

## Solution

TBD. Likely: put a runtime trait behind `src/executor/` (spawn command, argv, output-stream
parser, session/liveness detection) with `claude` as the first impl and `codex` as the second;
make the runtime selectable per project or globally in config. Check what Codex's
non-interactive/JSON output mode looks like before designing the stream parser abstraction.
