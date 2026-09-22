# Codex `exec --json` transcripts

Captured from **codex-cli 0.155.1** on 2026-09-22 during the 260922-hdj
research probes (`codex exec --json`, stdin from `/dev/null`, scratch git
repository). Quoted in
`.planning/quick/260922-hdj-support-codex-as-well-as-claude-as-the-agent-runtime-todo-pl/260922-hdj-RESEARCH.md`,
section "Codex CLI facts".

| File | Probe | Exit |
|---|---|---|
| `01-exec-success.jsonl` | `-s read-only`, prompt: run `cat note.txt`, reply `DONE`. All 7 lines verbatim. | 0 |
| `02-exec-failure.jsonl` | `-m no-such-model-xyz`. Lines 1-4 verbatim. | 1 |

**Line 5 of `02-exec-failure.jsonl` (`turn.failed`) is RECONSTRUCTED.** The
research abbreviated it with `...`. It was rebuilt as valid JSON whose
`error.message` equals the preceding top-level `error` line's `message` string,
which is the relationship the research recorded ("the top-level `error` is
followed by `turn.failed`").

These files live outside `tests/fixtures/transcripts/` on purpose: that
directory's README inventory is enforced by `tests/driver_rate_limit.rs` and
describes Claude `stream-json` captures only.

They are replayed by `tests/fixtures/fake-codex.sh` (end-to-end driver tests)
and read through `include_str!` by the unit tests in
`src/executor/codex_json.rs`.
