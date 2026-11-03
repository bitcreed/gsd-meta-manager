# One-off stream fixtures

Trimmed recordings of two real one-off runs (agent CLI 2.1.317, `--mode fast`),
used by the session-lifecycle and meter tests (quick 261103-mnr).

| File | Source run | Lines | What it pins |
|------|-----------|-------|--------------|
| `real-bg-subagents.stream.jsonl` | `/gsd-map-codebase` with four background subagents | 304 | background-task events, five results (the first with four tasks outstanding), usage with the 1-hour cache split, the real rate-limit event |
| `real-resumed.stream.jsonl` | a resumed `/gsd-docs-update` run with no background agents | 44 | a single result with nothing outstanding |

## How they were made

A whitelist script (kept outside the repository) read the recorded `stream.jsonl`
files and wrote one output line per input line, so line indices stay comparable
with the recording. Everything not listed was dropped:

- every line: `type`, `subtype`, `parent_tool_use_id`, `isReplay`; no session id, uuid or
  timestamp;
- ids (message, tool-use, task) became deterministic fakes that preserve equality;
- `assistant`: message id, model, role, the whole `usage` object, and content blocks
  reduced to `(text)`, an empty-input `tool_use`, or a bare `thinking` marker;
- `user`: replay text became `(prompt)` / `(message)`; tool results keep only
  `tool_use_id`, `is_error` and `(result)`;
- `system`: task ids, `is_backgrounded`, `status`, `patch.status`, usage counts and the
  `background_tasks_changed` task list; `init` keeps only the model; hook and
  thinking-counter lines keep only `type` and `subtype`;
- `result`: `is_error`, `num_turns`, `total_cost_usd`, `result_index`,
  `queued_turn_count`, `subagent_stats`, `origin.kind`, `modelUsage`, `usage`; the
  result text is dropped;
- `rate_limit_event`: `rate_limit_info`, whole.

The files are **regenerated from the recordings, never hand-edited**.

## Leak check

Run after every regeneration; both commands must print nothing:

```sh
grep -niE 'blk|/home/|/tmp/|@|(^|[^a-z])sk-|bearer|oauth|apikey|anthropic|<session id prefix>' \
  tests/fixtures/oneoff/*.jsonl
grep -ohiE '"[a-z0-9_]*token[a-z0-9_]*"' tests/fixtures/oneoff/*.jsonl | sort -u \
  | grep -viE '^"[a-z0-9_]*tokens(_delta|_details)?"$'
```

The second command lists any token-like string that is not a usage-count key.
