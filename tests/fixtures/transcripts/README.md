# Golden `stream-json` transcripts

This directory holds **eight real captures and one synthesised file**. Files `01`–`08` are
real captures from the local `claude` 2.1.220 binary, taken during the Phase 15 transport
spikes — not hand-written guesses. File `09` is the one exception and is labelled below.
They exist so the entire `stream-json` parsing layer, the capability gate, the `--bare`
regression guard, the tolerant-parsing behaviour and the outcome-derivation matrix can be
tested with **zero subscription, network or quota dependency** (D-24).

Each file is NDJSON: one JSON object per line, one trailing newline, no blank lines.

## The one synthesised file (Phase 20, CTRL-07)

`09-rate-limit-rejected.ndjson` is **synthesised and is not a capture.** It exists because
a `rate_limit_event` carrying `status: "rejected"` cannot be captured without exhausting
the very subscription quota it describes — a five-hour or seven-day window shared with
every other Claude surface the operator has. Seven of the eight real captures carry a
`rate_limit_event`, but every one of them says `allowed` or `allowed_warning`, so the one
status the quota park actually keys on had no fixture at all.

**Provenance of its contents.** Every field name and every enum value in it is taken
verbatim from the string table of the installed 2.1.235 binary, recorded in
`.planning/phases/20-deterministic-decision-router-run-bounds/20-RESEARCH.md` under "The
Claude Rate-Limit Wire Protocol" with the byte offsets it was read from: the
`rateLimitType` set (`five_hour`, `seven_day`, `seven_day_opus`, `seven_day_sonnet`,
`seven_day_overage_included`), the `status` set (`allowed`, `allowed_warning`,
`rejected`), and the transcript renderer's own field order for a rejection
(`rate_limit: rejected (` → `rateLimitType` → `resetsAt`). Its `resetsAt` is the same
`seven_day` value fixture `08` carries. Every envelope around the event — the
`system/init`, the replay `user` message, the terminal `result` — follows the real
captures' shape. Nothing in it was invented to make a test pass.

**Its `resetsAt` is a fixed instant in the past, deliberately left there.** Replayed
against a real clock it is judged unknown, because `reset_time`'s sanity bound is
asymmetric: the full thirty-day window ahead, and clock skew only behind (WR-03). A
window that "resets" before now is nonsense by construction — there is nothing to wait
for — and rendering a past instant into a park detail tells a human to come back at a
time that has been and gone. Refreshing the value would only postpone the same staleness
by however long the new value happens to sit ahead of the day it was written, so the
end-to-end fixture test asserts on `resets_at=unknown`, and the unit tests that need an
accepted reset judge it against a `now` chosen to sit an hour before it.

**Do not treat it as evidence of what the wire emits.** It is evidence of what this
repository *believes* the wire emits, and research assumption **A2** records that belief's
limit: the emission path for a rejection on a `-p` stream is inferred from the binary's
strings rather than observed. That is why the driver carries a **second** detector on the
failure envelope's own `terminal_reason`, which this fixture also exercises.

## CLI version drift

The eight captures were taken against **2.1.220**, and the binary the enum inventory above
was read from is **2.1.235** — fifteen patch versions later. That drift is the reason the
parsing layer matches `subtype`, `terminal_reason`, `status` and `rateLimitType` as `&str`
with an explicit fallback arm rather than as typed enums: three of the five `rateLimitType`
values (`seven_day_opus`, `seven_day_sonnet`, `seven_day_overage_included`) do not appear in
any capture here, and a typed parse pinned at 2.1.220 would have lost all three. The
tolerant matching is load-bearing, not stylistic, and this directory is where the evidence
for that lives.

## Fixtures

| Fixture | Source probe | Exit | Lines | What it pins down |
|---------|--------------|------|-------|-------------------|
| `01-success-textonly.ndjson` | OQ3 control arm — text-only prompt, no budget flag | **0** | 5 | The clean baseline. `subtype:"success"`, `terminal_reason:"completed"`, `result:"PONG"`. Includes a `rate_limit_event`, so the parser must tolerate it in the happy path. |
| `02-budget-exhausted.ndjson` | OQ3 arm A — `--max-budget-usd 0.00001` | **1** | 4 | `subtype:"error_max_budget_usd"`, `terminal_reason:"budget_exhausted"`, `errors[]` populated, and the `result` field **absent entirely** — the reason `result` must be `Option<String>` (D-32). The assistant turn ran to completion first: the budget check is a *post-turn* circuit breaker. |
| `03-tooluse-success-settingsources.ndjson` | OQ1 arm 2 — tool-using prompt **with** `--setting-sources project` | **0** | 7 | Tool use completing cleanly under the mitigation. Two assistant turns plus a `user` message carrying a `tool_result`. No hook events anywhere. |
| `04-hookhang-aborted-tools.ndjson` | OQ1 arm 1 — same prompt **without** `--setting-sources project`, `timeout -s TERM 90` | **124** | 11 | **The hang.** `system/hook_started` ×2 and `system/hook_response` ×2 arriving *before* `system/init`; `subtype:"error_during_execution"` / `terminal_reason:"aborted_tools"`; `duration_ms` 89 134 pinned to the 90s cap while `duration_api_ms` is 3 447. The exit code comes from the external `timeout`, never from Claude — the reason the exit code is a liveness signal only (D-10). |
| `05-queued-injection-two-turns.ndjson` | OQ2 (a) — second user message written mid-turn on a held-open stdin | **0** | 31 | **The most load-bearing fixture.** Two `system/init` and two `type:"result"` envelopes in one process: `result` is a **turn** boundary, not a run terminator (D-29), and the capability gate must fire on the **first** init only (D-30). Also: `isReplay:true` echoes for both messages, `session_id` identical across both `result`s, `num_turns` resetting per envelope while `total_cost_usd` accumulates, and 21 `system/thinking_tokens` events — the bounded-drain requirement (D-17) in one file. |
| `06-interrupt-aborted-streaming.ndjson` | OQ2 (b) — bare `{"type":"interrupt"}` at t=40s, then a `control_request` at t=55s | **1** | 26 | The bare form produces **nothing** (REFUTED, community #41665). The `control_request` form produces a `control_response` with the **double-nested** `response.response.still_queued`, then a synthetic `[Request interrupted by user]` user message, then `terminal_reason:"aborted_streaming"` (D-31, D-32). |
| `07-interrupt-early.ndjson` | OQ2 (b) — interrupt landing *before* the turn was streaming | **1** | 7 | The race case. `control_response` returns `subtype:"success"` with `still_queued:[]` while cancelling nothing meaningful. Pins the hazard that **`success` means "the request was accepted", not "the thing you meant was cancelled"** — correlate on `request_id` and treat `still_queued` as authoritative (D-31). |
| `08-tooluse-queued-two-turns.ndjson` | Phase 15 Plan 01 Task 1, Step 4 — the A3 sub-probe: text-only essay at t=0, a **tool-using** message written mid-turn at t=12s, stdin closed at t=14s | **0** | 19 | Settles RESEARCH Assumption **A3**: the per-turn `system/init` + `result` behaviour holds when the queued turn **uses tools**, not just for text-only turns. Two `init`, two `result`, an `assistant` message carrying a real `tool_use` block (`Read`), a `user` tool-result message, and `result:"hello from the scratch project"` — the genuine file content. `num_turns` is 1 on turn 1 and 2 on turn 2, because the tool round-trip adds an internal turn. |
| `09-rate-limit-rejected.ndjson` | **SYNTHESISED — not a capture.** See "The one synthesised file" above | **1** | 4 | The only `status:"rejected"` quota event in the tree, on a `seven_day` window with `utilization:1` and a `resetsAt`. No assistant turn, because a rejected request never ran one: `init` → `rate_limit_event` → replay `user` → a terminal `result` with `is_error:true`, `subtype:"error_during_execution"`, `terminal_reason:"api_error_rate_limit"`, `errors[]` populated and no `result` field. Its `claude_code_version` is `2.1.235`, above `TESTED_MAXIMUM_CLAUDE_VERSION`, so it also exercises the gate's warn-and-proceed arm. |

Every one of the eight captures was taken under subscription auth: `apiKeySource` is
`"none"` in all eight `system/init` events, which is what the D-08 `--bare` regression guard
asserts against. `09` carries the same value for the same reason — a quota rejection is a
subscription condition and cannot arise on the API-key path at all.

## Redaction record (D-25)

This section is about the eight **captures** only. `09` was never captured, so it was never
unredacted: its paths, uuids and session id were written in the already-redacted shape the
transforms below produce, and there is no raw counterpart of it anywhere.

All eight captures were redacted before being committed. Git history is forever, and a log
written before a redaction retrofit stays unredacted forever (Pitfall G) — so the transform
ran once, at promotion time, and its output is what you see here. Four transforms, applied
uniformly, **preserving the line count and every envelope and every field**:

1. **Path normalisation.** The operator's home prefix `/home/…` was replaced with
   `/home/testuser` and the session scratchpad prefix with `/tmp/scratch`. The sweep runs
   over the whole line, not just `cwd` and `memory_paths.auto`, because the same prefixes
   appear inside tool-result payloads. This makes the fixtures host-independent as well as
   non-disclosing — tests can assert on the paths.

   **Retrofit (phase-15 code review, finding WR-15).** The original sweep matched only the
   *slash* form of those prefixes and therefore missed Claude Code's **dash-encoded** form
   of the same paths — the shape it uses for session directory names under
   `~/.claude/projects/`. `-home-<user>-<repo-path>` and `-tmp-claude-<uid>-` survived in
   seven of the eight fixtures, disclosing the operator's username and real repository
   location even though the slash-form scan reported clean. A second pass replaced
   `-home-…-<repo>` with `-home-testuser-project` and `-tmp-claude-<uid>-` with
   `-tmp-scratch-`, keeping the dash-encoded *shape* so the fixtures stay structurally
   realistic. **Any future redaction sweep must scan both encodings** — a `/home/<user>`
   grep alone is not sufficient evidence of a clean fixture.
2. **UUID normalisation.** Every `session_id` in file `NN` became
   `00000000-0000-4000-8000-0000000000NN`, and every `uuid` became a zero-padded sequential
   `11111111-1111-4111-8111-%012d` counting from 1 within that file. Tests can therefore
   assert on identity across turns — fixture `05` shows the same `session_id` on both
   `result` envelopes. `request_id`, `tool_use_id`, `hook_id` and message `id` values are
   not UUIDs and were left as captured.
3. **Prose trimming.** Any `assistant` prose body longer than 200 characters became the
   literal marker `[trimmed for fixture]`. This covers a text block's `text`, a thinking
   block's `thinking`, and a thinking block's `signature` — the last is the opaque 4-8 KB
   base64 blob that actually accounts for the bulk of `05`, `06` and `08`. The fixtures
   exist to exercise **envelope** handling, not model prose, and this takes them from tens
   of kilobytes to something diff-reviewable. A `result` envelope's own `result` field is
   **never** trimmed — those values are asserted on (`"PONG"`,
   `"hello from the scratch project"`), and fixture `05`'s long `result` is preserved
   verbatim for the same reason.
4. **Credential re-scan.** Re-run independently against the **output**, rather than trusting
   the researcher's note: `sk-…`, `ghp_`, `gho_`, `github_pat_`, `Bearer `,
   `ANTHROPIC_API_KEY`, `oauth_token`, `AKIA…`, `xox[baprs]-` and PEM private-key headers.
   **Zero hits** — this result held up under the WR-15 re-audit and is unchanged.

   The path claim originally recorded here ("zero hits for any absolute `/home/<user>`")
   was true only of the *slash* encoding and so overstated the sweep's coverage; see the
   retrofit note under transform 1. As of that retrofit, both the slash and dash-encoded
   forms scan clean. Under `--setting-sources project` with subscription auth, no token
   material reaches the stream.

## Staging directory

The unredacted captures remain in the gitignored
`.planning/phases/15-transport-foundation/transcripts-raw/`. Retiring that directory is
deliberately deferred to plan **15-02**, which deletes it only after the parser tests prove
every fixture here is readable — making the deletion a verified step rather than a blind
one. It also holds `08-oq1-multistep.ndjson`, the 902 KB `/gsd-execute-phase` capture from
the OQ1 spike, which is **not** promoted: its value is the verdict, and
`.planning/phases/15-transport-foundation/15-SPIKE-OQ1.md` carries the distilled evidence.
