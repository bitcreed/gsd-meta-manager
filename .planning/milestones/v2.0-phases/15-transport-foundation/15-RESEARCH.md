# Phase 15: Transport Foundation — Duplex stream-json Executor - Research

**Researched:** 2026-07-29
**Domain:** Duplex NDJSON process transport (`claude -p --input-format stream-json`), process-group lifecycle, tokio event-loop restructure
**Confidence:** HIGH — every protocol claim below was produced by executing the local `claude` 2.1.220 binary in a throwaway scratch directory during this session. The serde model was compiled and run. The `process-wrap` API was read from the published 9.1.0 crate source, not from docs.rs prose.

---

## Summary

All three MUST-SPIKE questions are resolved, and two of the three answers change what the plan must build.

**OQ1 is confirmed at single-tool-call scale**: `--setting-sources project` is the mitigation, and the mechanism is now visible directly in the event stream rather than inferred — the hung arm emits `system/hook_started` + `system/hook_response` before `system/init`, the mitigated arm emits no hook events at all. What remains for the plan's gating Task 1 is strictly the *multi-step* generalisation (a real GSD skill that spawns subagent waves), which is an execution-time cost, not a research one.

**OQ3 is confirmed and is the opposite of the feared answer**: `--max-budget-usd` is **not** a no-op under subscription auth. It produced a distinct terminal state (`subtype: "error_max_budget_usd"`, `terminal_reason: "budget_exhausted"`, exit 1) with `apiKeySource: "none"`. The important nuance is that it is a *post-turn* circuit breaker — the turn it fires on runs to completion first — so it bounds the *next* turn, never the current one. D-16 stands unchanged; Phase 20's quota floor remains the real cost control, but it is now a supplement rather than the sole mechanism.

**OQ2 is resolved and refutes the community claim that the design was hedging against.** A message written to stdin mid-turn is **queued and executed as its own turn**, not dropped. But settling it surfaced a structural fact that no research document anticipated and that the planner must design around: **each queued turn emits its own `system/init` *and* its own `type:"result"` envelope.** `result` is a **turn** boundary, not a **run** terminator. Any executor that treats the first `result` as "the run ended" will truncate every multi-message run — which is every run that uses `send()`. This is flagged loudly in **Contradictions** below because D-10 and D-12 are written as though there is exactly one `result` per run.

**Primary recommendation:** Model the transport as *one process, N turns*. The run terminates on stdin EOF → process exit; each `result` closes a turn and contributes to a per-turn outcome list, with the run-level `RunOutcome` derived from the *last* turn's envelope plus the process exit code plus the disk/git delta. Build the `system/init` capability gate to fire on the **first** init only, and tolerate — never fail on — subsequent ones.

---

## Spike Outcomes (OQ1 / OQ2 / OQ3)

> Reported first because the orchestrator surfaces these upward verbatim. Every command below ran with cwd inside a disposable scratch directory under `/tmp/claude-1000/…/scratchpad/probes`, never against the `gsd-meta-manager` repo, always under an external `timeout`, and never with `--dangerously-skip-permissions` or `--permission-mode bypassPermissions`.

### OQ3 — Does `--max-budget-usd` apply under subscription auth? — **CONFIRMED**

**Verdict: CONFIRMED — the flag applies, and it is a post-turn circuit breaker, not a pre-flight gate.**

Command (arm A, absurdly low budget):

```bash
claude -p --input-format stream-json --output-format stream-json --verbose \
  --replay-user-messages --session-id <uuid> --setting-sources project \
  --max-budget-usd 0.00001 < msg.ndjson
```

where `msg.ndjson` is one line: `{"type":"user","message":{"role":"user","content":[{"type":"text","text":"Reply with exactly the word PONG and nothing else. Do not use any tools."}]}}`

Observed — exit code **1**, four stream lines, terminal envelope (usage/modelUsage elided):

```json
{"type":"result","subtype":"error_max_budget_usd","is_error":true,
 "terminal_reason":"budget_exhausted","stop_reason":"end_turn",
 "errors":["Reached maximum budget ($0.00001)"],
 "total_cost_usd":0.0616515,"duration_ms":3568,"duration_api_ms":0,
 "num_turns":1,"permission_denials":[]}
```

The matching `system/init` reported `"apiKeySource":"none"` — i.e. subscription/OAuth auth, no `ANTHROPIC_API_KEY` — so this is decisively the subscription path.

Control arm (identical prompt, flag omitted): exit **0**, wall 2s, `subtype:"success"`, `terminal_reason:"completed"`, `result:"PONG"`, `total_cost_usd: 0.0622905`.

**The nuance that matters:** in arm A the assistant turn *ran to completion* — the stream contains a full `assistant` message before the `result`. The budget check fired **after** the turn, not before it. `--max-budget-usd` therefore stops the *next* turn; it cannot prevent the current one from spending. It is a usable anomaly circuit-breaker and useless as a hard pre-spend cap.

**Also note:** `total_cost_usd` under subscription is a *notional* price for work that is not billed per call. D-16 ("plumbed as an option and nothing more") stands. Phase 20 must still not display this figure as "what this run cost."

**Remaining for Task 1:** nothing. OQ3 is closed.

---

### OQ1 — Does `--setting-sources project` suppress the hook hang? — **CONFIRMED (bounded scope)**

**Verdict: CONFIRMED at single-tool-call scale. The multi-step generalisation remains the plan's gating Task 1.**

Both arms used the same tool-using prompt — `{"type":"user","message":{"role":"user","content":[{"type":"text","text":"Read ./Cargo.toml and reply with the package version, nothing else."}]}}` — against a scratch dir containing a `Cargo.toml` with `version = "0.4.2"`, each under `timeout -s TERM 90`.

**Arm 1 — WITHOUT `--setting-sources project`:**

```bash
timeout -s TERM 90 claude -p --input-format stream-json --output-format stream-json \
  --verbose --replay-user-messages --session-id <uuid> --permission-mode dontAsk < msg.ndjson
```

| Signal | Value |
|--------|-------|
| exit code | **124** (external `timeout` fired — never a Claude code) |
| wall clock | **90s** |
| `duration_ms` | **89134** |
| `duration_api_ms` | 3447 |
| `subtype` | `error_during_execution` |
| `terminal_reason` | `aborted_tools` |
| `errors` | `["[ede_diagnostic] result_type=user last_content_type=n/a stop_reason=tool_use"]` |

Hang reproduced. `duration_ms` pinned to the wall-clock cap while `duration_api_ms` was ~3.4s — the model finished in seconds and the process then hung until SIGTERM, exactly as the research pass described.

**Arm 2 — WITH `--setting-sources project`:**

```bash
timeout -s TERM 90 claude -p --input-format stream-json --output-format stream-json \
  --verbose --replay-user-messages --session-id <uuid> --setting-sources project \
  --permission-mode dontAsk --strict-mcp-config < msg.ndjson
```

| Signal | Value |
|--------|-------|
| exit code | **0** |
| wall clock | **8s** |
| `duration_ms` | **6351** |
| `duration_api_ms` | 7621 |
| `subtype` | `success` |
| `terminal_reason` | `completed` |
| `result` | `"0.4.2"` — correct; the Read tool genuinely ran |

**Mechanism now directly observable, not inferred.** The hung arm's stream begins with four hook events *before* `system/init`:

```json
{"type":"system","subtype":"hook_started","hook_id":"…","hook_name":"SessionStart:startup","hook_event":"SessionStart","uuid":"…","session_id":"…"}
{"type":"system","subtype":"hook_response","hook_id":"…","hook_name":"SessionStart:startup","hook_event":"SessionStart","output":"","stdout":"","stderr":"","exit_code":0,"outcome":"success","uuid":"…","session_id":"…"}
```

The mitigated arm emits **no hook events at all**. That is the causal link, captured in the protocol itself.

**Gotcha for the deadline design (D-13):** in the clean arm `duration_api_ms` (7621) is **greater** than `duration_ms` (6351). `duration_api_ms` aggregates across API calls including parallel ones and is not a wall clock. Do not use it as one; the idle-cap timer must be driven by *time since last stream line observed by the reader task*, which is what D-13 already specifies.

**Remaining for Task 1:** the success-criterion-1 spike proper — a real multi-step GSD skill (e.g. `/gsd:execute-phase`) that itself spawns subagent waves, run headlessly to completion against a disposable scratch GSD project (D-28). Two specific things this bounded probe cannot speak to:

1. Subagent waves spawn nested `claude` processes; whether `--setting-sources project` propagates to them is untested.
2. A long skill run crosses the `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` background-subagent ceiling (silent 10-minute default, D-14). The single-tool probe never approached it.

---

### OQ2 — Mid-turn stdin injection and `control_request` interrupt semantics — **CONFIRMED (and it refutes the community claim)**

**Verdict: CONFIRMED. Mid-turn injection is QUEUED, not dropped. `control_request{subtype:"interrupt"}` works exactly as described. Bare `{"type":"interrupt"}` is REFUTED — it does nothing.**

#### (a) Mid-turn injection — queued and executed as its own turn

Driver: a FIFO on stdin held open across the run; `msg1` (a long essay task, ~68s) written at t=0; `msg2` (`"IGNORE the essay task. Reply with exactly the word INJECTED and nothing else."`) written at **t=12s**, mid-turn; stdin closed at t=14s.

Observed timeline (`thinking_tokens` noise elided):

| # | type | subtype | timestamp | note |
|---|------|---------|-----------|------|
| 1 | `system` | `init` | — | first init |
| 2 | `rate_limit_event` | — | — | |
| 3 | `user` | — | 08:04:54.731 | `isReplay: true` — msg1 echo |
| 4 | `assistant` | — | 08:05:26.993 | (thinking) |
| 5 | `assistant` | — | 08:06:01.797 | the essay |
| 6 | **`result`** | `success` | — | **turn 1 closes** |
| 7 | **`system`** | **`init`** | — | **second init** |
| 8 | `user` | — | 08:06:01.842 | `isReplay: true` — msg2 echo |
| 9 | `assistant` | — | 08:06:04.417 | `"INJECTED"` |
| 10 | **`result`** | `success` | — | **turn 2 closes** |

Process then exited **0** at wall 71s.

**Three load-bearing facts:**

1. **`result` is a turn boundary, not a run terminator.** Two `result` envelopes in one process. A second `system/init` is emitted at the start of the queued turn. See **Contradictions** — D-10/D-12 read as though there is one `result` per run.

2. **The replay echo is emitted at *dequeue*, not at *receipt*.** msg2 was written at t≈12s and echoed at 08:06:01.842 — 45ms *after* turn 1's `result`, roughly 55s after it was written. So `isReplay: true` is a "**started processing**" ack, not a "**received**" ack. That is precisely the distinction Phase 18's three-state UI (queued → delivered → acted-on) needs, and it means the design's only delivery ack is honest about what it acks.

3. **`num_turns` resets per envelope; `total_cost_usd` accumulates.**

   | field | turn 1 | turn 2 | behaviour |
   |-------|--------|--------|-----------|
   | `num_turns` | 1 | 1 | **per-turn, resets** |
   | `total_cost_usd` | 0.16959 | 0.23500 | **cumulative** |
   | `duration_ms` | 67115 | 2764 | **per-turn** |
   | `duration_api_ms` | 68553 | 71301 | **cumulative** |
   | `session_id` | same | same | stable |
   | `uuid` | differs | differs | per-envelope |

   The `num_turns` reset empirically confirms PITFALLS' claim that `--max-turns` counters reset when stream-json queues messages. D-16's "never a load-bearing bound" is now verified, not assumed.

The exact NDJSON wire shape of a user message on stdin — the planner needs this literally:

```json
{"type":"user","message":{"role":"user","content":[{"type":"text","text":"<prompt>"}]}}
```

And the echo that comes back:

```json
{"type":"user","message":{"role":"user","content":[{"type":"text","text":"<prompt>"}]},
 "session_id":"<uuid>","parent_tool_use_id":null,"uuid":"<msg-uuid>",
 "timestamp":"2026-07-29T08:01:29.247Z","isReplay":true}
```

`isReplay` is the discriminator. It is **camelCase**, and it is absent (not `false`) on non-replay `user` messages — so it must be `#[serde(default)] pub is_replay: bool`.

**stdin EOF semantics (confirms D-04):** closing the stdin handle did not kill the run. Claude drained the queued turn, finished it, and exited 0. EOF means "no more input", not "stop".

#### (b) `control_request` interrupt — CONFIRMED; bare form REFUTED

A first probe was ambiguous (the interrupt landed before the turn was truly streaming), so a clean disambiguation run was done: bare form at t=40s, `control_request` at t=55s, both well inside a streaming turn.

**Bare form — REFUTED.** Writing `{"type":"interrupt"}` at t=40s produced **no `control_response`, no effect**; the process was still alive 15s later. This corroborates community issue #41665.

**`control_request` — CONFIRMED.** Request written to stdin:

```json
{"type":"control_request","request_id":"req_1","request":{"subtype":"interrupt"}}
```

Response on stdout (verbatim — note the **double nesting** of `response`):

```json
{"type":"control_response","response":{"subtype":"success","request_id":"req_1","response":{"still_queued":[]}}}
```

Effect on a streaming turn: the partial assistant message is flushed, then a synthetic user message is injected —

```json
{"type":"user","message":{"role":"user","content":[{"type":"text","text":"[Request interrupted by user]"}]},…}
```

— then the run terminates:

```json
{"type":"result","subtype":"error_during_execution","is_error":true,
 "terminal_reason":"aborted_streaming","stop_reason":null,
 "errors":["[ede_diagnostic] result_type=user last_content_type=n/a stop_reason=null"],
 "num_turns":2,"total_cost_usd":0.000636,"duration_ms":53693}
```

Process exit code **1**.

**A real hazard the planner must handle:** in the first (ambiguous) probe the interrupt arrived before the target turn was running. It still returned `subtype:"success"` with `still_queued: []` — a success response that cancelled nothing meaningful. **`success` on a `control_response` means "the request was accepted", not "the thing you meant was cancelled."** Correlate on `request_id` *and* treat the `response.response.still_queued` array as the authoritative statement of what remains queued. That array is the `interrupt_cancel_queued_v1` capability surfacing.

**Remaining for Task 1:** nothing blocking. One item is deliberately **UNRESOLVED and moot**: whether an injected message is persisted to session history and survives `--resume` was not tested. It is moot because the message *executes* in-process, which is the property Phase 18 depends on. Record it as untested rather than as known-good.

---

## Contradictions with Committed Research and CONTEXT.md

> Per the task constraints: `[LOCKED-BY-RESEARCH]` decisions are not relitigated here. These are places where a probe produced evidence that the locked decision's *supporting facts* were incomplete or wrong. The decisions themselves stand; their factual annexes need amending.

### 1. LOUD — "one `result` envelope per run" is false, and D-10/D-12 read as though it is true

D-10 ("Outcome comes from four sources … the `type:"result"` envelope") and D-12's `RunOutcome` variant set are both phrased for a single terminal envelope. **A run that uses `send()` (D-22) emits one `result` per turn.** An executor that returns on the first `result` truncates every steered run and reports "succeeded" while work is still queued.

This does not overturn D-10 — the four corroboration sources are still right. It amends *how* the envelope source is read:

- `result` closes a **turn**. Collect them into a `Vec<TurnOutcome>`.
- The **run** terminates on stdin EOF → process exit. That is the only run terminator.
- Run-level `RunOutcome` derives from the **last** `result` + exit code + disk/git delta.
- `total_cost_usd` from the last envelope is the cumulative run cost; `num_turns` is **not** cumulative and must be summed if a run-level turn count is wanted.

### 2. LOUD — a second `system/init` is emitted per queued turn

D-06's gate ("read events until `system/init`, validate, withhold the first user message until validation passes") is correct and works — but only for the **first** init. The parser must tolerate subsequent inits without re-running the gate, re-arming the `--bare` guard as a run-abort, or treating it as a protocol error. Gate on first-init-only; treat later inits as informational.

### 3. The observed `subtype` / `terminal_reason` inventories in D-10 are incomplete

D-10 records `subtype` ∈ {`success`, `error_max_turns`, `error_during_execution`} and `terminal_reason` ∈ {`completed`, `aborted_tools`}. This session adds, all directly observed on 2.1.220:

| new value | field | produced by |
|-----------|-------|-------------|
| `error_max_budget_usd` | `subtype` | `--max-budget-usd` exceeded |
| `budget_exhausted` | `terminal_reason` | same |
| `aborted_streaming` | `terminal_reason` | `control_request` interrupt during streaming |

D-26's outcome-derivation test matrix must cover these. They are not exotic — two of the three are reachable from features this phase ships.

### 4. REFUTED — ARCHITECTURE's AP3 ("Injecting a message mid-turn") is wrong on 2.1.220

AP3 states: *"the in-flight turn ignores it **and** it isn't persisted to history, so it's also lost on `--resume`."* The first clause is confirmed; the second is refuted in the way that matters — the message is queued and **executes as its own turn**. AP3's prescribed workaround ("buffer, flush on `result`") is therefore **unnecessary**: the CLI already performs exactly that buffering internally. A driver that re-implements it would be duplicating CLI behaviour and would make the `still_queued` accounting harder to reason about.

Consequence for Phase 18 (which D-22 says must be told the OQ2 result): **Phase 18 can write and correlate directly.** It does not need a turn-boundary flush buffer. SUMMARY's "Resolution: the driver must buffer interjections and flush at the `result` turn boundary" is superseded.

### 5. Fields absent from the research pass's envelope sketch

The `result` envelope in STACK.md/CONTEXT.md shows `"result":"OK"` as though `result` were always present. **It is absent on error envelopes** (both the budget and interrupt runs omit it entirely). Likewise `api_error_status` is present on success envelopes and absent on error ones. `result` must be `Option<String>`.

New fields observed on `result` and not in any research doc: `stop_reason`, `errors[]`, `ttft_ms`, `ttft_stream_ms`, `time_to_request_ms`, `fast_mode_state`, `fast_mode_disabled_reason`, `uuid`.

New `system` subtype observed: **`thinking_tokens`** (`{"type":"system","subtype":"thinking_tokens","estimated_tokens":50,"estimated_tokens_delta":50,"uuid":"…","session_id":"…"}`). It arrived **22 times** in one 68-second turn. This is a high-frequency event type that no research document mentions — direct evidence for D-09's tolerant parsing and for D-17's bounded-batch drain.

---

## Project Constraints (from CLAUDE.md)

| Directive | Source | Implication for this phase |
|-----------|--------|----------------------------|
| `cargo build && cargo test && cargo clippy -- -D warnings` must pass | project gate | Every new module must be clippy-clean at the lib target. |
| `cargo clippy --all-targets` has exactly **5** pre-existing lints (browser.rs ×3, project_creator.rs ×1, state_reader/mod.rs ×1); the count must not grow | project gate | New `#[cfg(test)]` modules and any `tests/` additions must be clippy-clean. Do not fix the 5 (out of scope). |
| Release process: bump `Cargo.toml` version, `cargo update`, verify clean, update `STATE.md`, annotated tag with full changelog | CLAUDE.md | Not this phase's, but the MSRV key added by D-20 lands in the same file the release process edits. |
| Work must start through a GSD command | CLAUDE.md | Research only here; no source edits made. |
| Stack: ratatui 0.30 + crossterm 0.29 + tokio 1 (`full`) | CLAUDE.md | `tokio` `full` already provides `process`, `io-util`, `sync`, `time`. No Cargo change needed for those. |
| "Never block the render loop — all I/O goes through tokio channels (mpsc)" | CLAUDE.md stack patterns | Directly binds D-04 (parse in the reader task) and D-17. |
| "`std::sync::Mutex` in async code" listed under What NOT to Use | CLAUDE.md | Any shared executor state uses `tokio::sync` or message passing. |
| MSRV stated as 1.85+ in the stack table | CLAUDE.md:25 | One of the 8 sites D-20 must update. |

---

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| **TRANS-01** | Duplex `stream-json` over stdio; structured envelope, not text scraping | Wire shapes captured verbatim for `user` (stdin), `system/init`, `system/hook_*`, `system/thinking_tokens`, `assistant`, `rate_limit_event`, `control_request`/`control_response`, `result`. Compiled + validated tolerant serde model in **Code Examples**. Pipe/task topology in **Architecture Patterns → Pattern 1**. |
| **TRANS-02** | Outcome from `result` envelope + exit code + disk state + git; never prose | Four distinguishable terminal states captured as real transcripts (success / budget-exhausted / hook-hang-aborted / interrupt-aborted) with their exit codes. Amended multi-`result` reading rule in **Contradictions #1**. Existing disk/git primitives inventoried in **Don't Hand-Roll**. |
| **TRANS-03** | TUI event loop stays responsive while a run streams | `run_tui_loop` read directly; concrete `tokio::select!` shape in **Architecture Patterns → Pattern 3**; a *deterministic* proof strategy (not an assertion) in **Verifying Success Criterion 3**. Load evidence: 22 `thinking_tokens` events in one turn. |
| **TRANS-04** | `system/init` capability detection; refuse unsupported CLI up front, naming the missing capability | Full `system/init` captured verbatim below with exact field spellings and the exact `capabilities[]` contents on 2.1.220. Gate mechanics + the first-init-only amendment in **Contradictions #2**. |

---

## The `system/init` Event, Captured Verbatim (2.1.220)

The raw material for the D-06 capability gate, the D-07 version gate, and the D-08 `--bare` regression guard. `tools[]`, `slash_commands[]` and `cwd` elided for length; everything else is as observed.

```json
{
  "type": "system",
  "subtype": "init",
  "cwd": "<scratch dir>",
  "session_id": "51cb3f73-b679-4385-a852-d4e602eee5eb",
  "tools": ["Task","Bash","CronCreate","…","Write"],
  "mcp_servers": [],
  "model": "claude-opus-5[1m]",
  "permissionMode": "default",
  "slash_commands": ["deep-research","design-sync","dataviz","…"],
  "apiKeySource": "none",
  "claude_code_version": "2.1.220",
  "output_style": "default",
  "agents": ["claude","Explore","general-purpose","Plan","statusline-setup"],
  "skills": ["deep-research","design-sync","dataviz","update-config","verify","debug",
             "code-review","simplify","batch","fewer-permission-prompts","doctor","loop",
             "claude-api","run","run-skill-generator"],
  "plugins": [],
  "capabilities": ["interrupt_receipt_v1","interrupt_cancel_queued_v1","msg_lifecycle_v1"],
  "analytics_disabled": true,
  "product_feedback_disabled": false,
  "uuid": "297d5c3d-a78b-4648-b15c-40c776f2e040",
  "memory_paths": {"auto": "<home>/.claude/projects/<slug>/memory/"},
  "fast_mode_state": "off",
  "fast_mode_disabled_reason": "sdk_opt_in_required"
}
```

Load-bearing details for the planner:

- **`capabilities[]` on 2.1.220 is exactly three entries:** `interrupt_receipt_v1`, `interrupt_cancel_queued_v1`, `msg_lifecycle_v1`. D-06's recorded set matches. Confirmed stable across all six probe runs.
- **Field-name casing is mixed within the same object.** `apiKeySource` and `permissionMode` are camelCase; `claude_code_version`, `session_id`, `output_style`, `memory_paths` are snake_case. A blanket `#[serde(rename_all = "camelCase")]` on the init struct will silently break `claude_code_version`. Use per-field `#[serde(rename = "apiKeySource")]` and leave the rest snake_case.
- **`apiKeySource: "none"`** is the D-08 `--bare` regression guard's assertion target. Confirmed present and `"none"` under subscription auth on 2.1.220. Note the guard must assert `== "none"`, and the plan should decide explicitly what a *missing* `apiKeySource` means (recommend: treat absent as a guard failure, since a future `--bare`-by-default CLI dropping the field is exactly the silent regression D-08 is defending against).
- **`permissionMode` reflects the flag.** It read `"default"` with no flag and `"dontAsk"` with `--permission-mode dontAsk` (D-15). Cheap, free assertion that the flag actually took effect.
- **New fields since the research pass:** `output_style`, `agents[]`, `skills[]`, `analytics_disabled`, `product_feedback_disabled`, `uuid`, `memory_paths{}`, `fast_mode_state`, `fast_mode_disabled_reason`. Nine new fields in one CLI version — the strongest possible argument for D-09.
- **`memory_paths.auto` contains an absolute home path.** This is a redaction target for D-25.
- **`--setting-sources project` does not strip skills or agents.** Both arrays were fully populated in the mitigated arm. `--setting-sources` governs `settings.json` tiers (and therefore hooks); it is not a general hermeticity switch. Do not document it as one.

---

## Captured Transcripts (golden fixture candidates)

Seven real transcripts are staged in the session scratchpad. D-24 wants these checked in as golden fixtures; D-25 requires manual redaction first.

Base: `/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/4661fdcd-7717-459f-9c14-d7e5e713b2e9/scratchpad/transcripts/`

| File | Lines | Exit | What it pins down |
|------|-------|------|-------------------|
| `01-success-textonly.ndjson` | 5 | 0 | Clean baseline. `subtype:"success"`, `terminal_reason:"completed"`, `result:"PONG"`. Includes a `rate_limit_event`. |
| `02-budget-exhausted.ndjson` | 4 | 1 | `subtype:"error_max_budget_usd"`, `terminal_reason:"budget_exhausted"`, `errors[]` populated, `result` **absent**. |
| `03-tooluse-success-settingsources.ndjson` | 7 | 0 | Tool use completing cleanly under `--setting-sources project`. Two assistant turns + tool-result `user` message. |
| `04-hookhang-aborted-tools.ndjson` | 11 | 124 | The hang. `system/hook_started` ×2 + `hook_response` ×2 **before** `system/init`; `error_during_execution` / `aborted_tools`; `duration_ms` 89134 vs `duration_api_ms` 3447. |
| `05-queued-injection-two-turns.ndjson` | 31 | 0 | **The most valuable fixture.** Two `system/init`, two `result`, `isReplay` echoes for both messages, 22 `thinking_tokens` events. Pins the multi-turn reading rule and the bounded-drain requirement in one file. |
| `06-interrupt-aborted-streaming.ndjson` | 26 | 1 | Bare `{"type":"interrupt"}` producing nothing, then `control_request` → `control_response` → `[Request interrupted by user]` → `aborted_streaming`. |
| `07-interrupt-early.ndjson` | 7 | 1 | The race case: `control_response` `subtype:"success"` with `still_queued:[]` arriving before the turn was streaming. Pins the "success ≠ cancelled what you meant" hazard. |

**Redaction required before check-in (D-25):**

- Absolute paths: `/home/blk` and `/tmp/claude-1000` appear in `cwd` and `memory_paths.auto`. Replace with stable placeholders (`/home/testuser`, `/tmp/scratch`) so fixtures are host-independent as well as redacted.
- Session UUIDs and message UUIDs are real but non-secret. Recommend normalising them to fixed values anyway so tests can assert on them.
- **Scanned for credential-shaped strings** (`sk-*`, `ghp_*`, `Bearer …`, `ANTHROPIC_API_KEY`, `oauth_token`) across all seven files: **none found.** Under `--setting-sources project` with subscription auth, no token material reaches the stream.
- The essay content in `05`/`06` is model prose about semicolons — harmless, but it makes those files 37KB and 20KB. Consider trimming the `assistant` text bodies to a short marker while preserving every envelope, since the fixtures exist to test *envelope handling*, not content.

Also present, and worth keeping alongside: `probes/oq1/{NOSS,SS}.stderr`, `probes/oq2/*.stderr`, `probes/oq3/*.stderr` — **all zero bytes**. Useful negative evidence: on a well-behaved run `claude` writes nothing to stderr, so a non-empty stderr is itself a diagnostic signal worth surfacing (D-04 keeps the pipes separable precisely for this).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Process spawn + process-group containment | Executor (`src/executor/claude.rs`) | — | Single spawn seam (D-23, Pitfall 10). No other module may spawn. |
| NDJSON framing + parsing | Reader task (spawned, off render thread) | — | CLAUDE.md: never parse on the render loop. Claude caps its exit drain at 30s, so a slow consumer truncates the run tail. |
| Capability / version gate | Executor, pre-first-message | — | D-06: gate before any turn starts so refusal costs zero quota. |
| stdin injection | Dedicated writer task | — | D-04: two-pipe deadlock is live. Writer must not be the task awaiting exit. |
| Outcome derivation | Executor (`RunOutcome`) | `state_reader` (disk/git delta) | D-10/D-11. Envelope verdict and disk corroboration are computed together, once, at run end. |
| Deadline enforcement (wall + idle) | Supervisor task | — | D-13. Idle timer is driven by the reader task's last-line timestamp. |
| Teardown (SIGTERM → grace → SIGKILL → wait) | Executor | — | D-14. Phase 17 layers the user-facing kill switch on top. |
| Event delivery to UI | `mpsc` — **separate channel** from `Action` | — | D-17/D-19. High-volume stream traffic must not share the `Action` FIFO. |
| Rendering | *(none — this phase ships no UI)* | — | ROADMAP: Phase 18 owns every rendered driver surface. |

---

## Standard Stack

### Core additions

| Library | Version | Purpose | Why standard |
|---------|---------|---------|--------------|
| `process-wrap` | 9.1.0 | Process-group spawn + whole-tree signal | Explicit successor to `command-group` (same author, watchexec org). 9,741,588 total / 4,125,911 recent downloads; updated 2026-03-08. `tokio::process::Child::kill()` reaches only the direct child; `claude` spawns Bash grandchildren. [VERIFIED: crates.io API + crate source read locally] |
| `uuid` | 1.24.0 | Generate `--session-id` before spawn | 691M total / 156M recent; updated 2026-07-15. Owning the session id pre-spawn makes a crash before the first stdout byte resumable. Features `v4` + `serde`. [VERIFIED: crates.io API] |
| `tokio-util` | 0.7.19 | *Conditional* — `CancellationToken` and/or `LinesCodec` | 679M total / 143M recent; updated 2026-07-21. See D-05 recommendation below. [VERIFIED: crates.io API] |

**Installation:**

```toml
process-wrap = { version = "9.1.0", features = ["tokio1"] }
uuid = { version = "1.24", features = ["v4", "serde"] }
# only if the D-05 analysis below lands on "yes":
tokio-util = { version = "0.7", features = ["codec"] }
```

### Not needed — already present

`tokio` (`full` ⇒ `process`, `io-util`, `sync`, `time`, `macros`), `serde` + derive, `serde_json`, `chrono` + serde, `tempfile`, `futures`, `tracing`. **No Cargo change for any of these.**

### D-05 resolution: recommend `BufReader::lines()`, skip `tokio-util`

D-05 leaves this to the planner and asks for the reasoning to be recorded. The evidence favours **no new dependency**:

- The stdout reader is a plain `while let Some(line) = lines.next_line().await?` loop over `BufReader::new(stdout).lines()`. It reads better than a codec here and needs no `Stream` plumbing.
- Cancellation does **not** require `CancellationToken`. The reader task ends naturally when the child's stdout closes, which happens on process exit — which the teardown sequence (D-14) already forces. For the supervisor's own cancel signal, a `tokio::sync::watch::<bool>` or an `mpsc` command channel is sufficient and uses only what's already vendored.
- `tokio-util` would be added for exactly one type. STACK.md explicitly warns against adding it *solely* for framing, and the cancellation half of the justification dissolves once you notice teardown already closes the pipe.

**Caveat the plan should honour:** `BufReader::lines()` is unbounded per line. `claude` can emit very large single lines (a full essay `assistant` message was ~30KB in probe 05; a large tool result could be far bigger). `LinesCodec::new_with_max_length(n)` gives free protection that `lines()` does not. If the planner wants that bound, `tokio-util` earns its place — but a manual length check on the returned `String` is equally effective and free. **Recommendation: `BufReader::lines()` + an explicit max-line-length guard that emits a truncation event rather than failing the run.**

### Alternatives considered

| Instead of | Could use | Tradeoff |
|------------|-----------|----------|
| `process-wrap` | bare `tokio::process` + `kill_on_drop` | Orphans `claude`'s Bash grandchildren. Disqualified by the stoppability constraint — this is exactly Pitfall 6. |
| `process-wrap` | `command-group` 5.0.1 | Superseded by its own author; last release 2023-11-18. |
| `process-wrap` | `processkit` 3.0.2 | Right feature set, ~10.8k downloads, published 2026-07-25. Wrong risk profile for the module that owns the kill switch. |
| `BufReader::lines()` | `tokio_util::codec::FramedRead<_, LinesCodec>` | Adds a dep; buys `max_length` backpressure and a `Stream`. See D-05 resolution. |
| one task per pipe | `tokio-process-stream` | Merges stdout and stderr — the opposite of D-04's requirement that they stay separable. |

---

## Package Legitimacy Audit

| Package | Registry | Age / last update | Downloads | Source repo | Verdict | Disposition |
|---------|----------|-------------------|-----------|-------------|---------|-------------|
| `process-wrap` | crates.io | 9.1.0, 2026-03-08 | 9.74M total / 4.13M recent | github.com/watchexec/process-wrap | OK | Approved |
| `uuid` | crates.io | 1.24.0, 2026-07-15 | 691.7M total / 156.3M recent | github.com/uuid-rs/uuid | OK | Approved |
| `tokio-util` | crates.io | 0.7.19, 2026-07-21 | 679.2M total / 143.5M recent | github.com/tokio-rs/tokio | OK | Approved (conditional — see D-05) |

**Packages removed due to SLOP verdict:** none.
**Packages flagged suspicious:** none.

All three were verified against the live crates.io API this session, and `process-wrap` additionally by downloading and reading the published 9.1.0 `.crate` tarball. All three are pre-existing recommendations from the committed research pass, not newly discovered names. No `postinstall`-equivalent risk exists in the Cargo ecosystem for these (no `build.rs` network access; `process-wrap` declares `build = false`).

---

## `process-wrap` 9.1.0 — Verified API Surface

Read from the published crate source (`src/generic_wrap.rs`, `src/tokio/core.rs`, `src/tokio/process_group.rs`, `Cargo.toml`), not from docs.rs prose — the docs.rs module page does not carry method signatures.

### Manifest facts

```toml
edition      = "2024"
rust-version = "1.87.0"          # confirms D-03/D-20's MSRV rise
default      = ["creation-flags","job-object","kill-on-drop",
                "process-group","process-session","tracing"]
tokio1       = ["dep:nix","dep:futures","dep:tokio"]   # NOT in default — confirms D-03
```

`tokio1` pulls `tokio` with `io-util`, `macros`, `process`, `rt` — all already in the project's `full`. `tracing` is a default feature and integrates with the existing subscriber for free.

### Types and methods actually available

```rust
use process_wrap::tokio::*;   // CommandWrap, CommandWrapper, ChildWrapper,
                              // ProcessGroup, ProcessGroupChild, KillOnDrop
```

`CommandWrap` (generated by the `Wrap!` macro):

```rust
pub fn with_new(program: impl AsRef<OsStr>, init: impl FnOnce(&mut Command)) -> Self;
pub fn wrap<W: CommandWrapper + 'static>(&mut self, wrapper: W) -> &mut Self;
pub fn spawn(&mut self) -> std::io::Result<Box<dyn ChildWrapper>>;
pub fn command(&self) -> &Command;
pub fn command_mut(&mut self) -> &mut Command;
```

`ChildWrapper` (the trait object `spawn()` returns):

```rust
fn stdin(&mut self)  -> &mut Option<ChildStdin>;   // .take() for an owned handle
fn stdout(&mut self) -> &mut Option<ChildStdout>;
fn stderr(&mut self) -> &mut Option<ChildStderr>;
fn id(&self) -> Option<u32>;
fn signal(&self, sig: i32) -> Result<()>;                                   // unix only
fn start_kill(&mut self) -> Result<()>;
fn try_wait(&mut self) -> Result<Option<ExitStatus>>;
fn wait(&mut self) -> Pin<Box<dyn Future<Output = Result<ExitStatus>> + Send + '_>>;
fn kill(&mut self) -> Box<dyn Future<Output = Result<()>> + Send + '_>;      // = start_kill + wait
```

`ProcessGroup`:

```rust
pub fn leader() -> Self;
pub fn attach_to(leader: u32) -> Self;
```

### Four gotchas the planner must design around

**1. `start_kill()` on a process group sends SIGKILL, not SIGTERM.**

```rust
// src/tokio/process_group.rs
impl ChildWrapper for ProcessGroupChild {
    fn start_kill(&mut self) -> Result<()> { self.signal_imp(Signal::SIGKILL) }
    fn signal(&self, sig: i32) -> Result<()> { self.signal_imp(Signal::try_from(sig)?) }
}
fn signal_imp(&self, sig: Signal) -> Result<()> { killpg(self.pgid, sig).map_err(Error::from) }
```

D-14's sequence therefore maps to `child.signal(15)` → 10s grace → `child.start_kill()` → `child.wait().await`. **`kill()` is not the SIGTERM path** — it goes straight to SIGKILL and skips `claude`'s clean shutdown (turn abort, Bash-tree teardown, `SessionEnd` hooks, exit 143). Reaching for `kill()` silently discards the documented graceful path.

`signal()` takes a raw `i32`, so `15`/`9` work without depending on `nix` or `libc` directly.

**2. `signal(&self)` and `wait(&mut self)` cannot be used concurrently on the same handle.**

`wait()` borrows `&mut self` for the returned future's lifetime. A supervisor holding `wait()` across an `.await` cannot call `signal()` on the same object. Two clean resolutions:

- **Preferred — `tokio::select!` drops the loser.** Race `child.wait()` against the deadline/cancel arm; when the cancel arm wins, the `wait()` future is dropped, the `&mut` borrow ends, and `child.signal(15)` is callable in the same block.
- **Alternative — capture the pgid at spawn.** With `ProcessGroup::leader()` the child *is* the group leader, so `pgid == child.id()`. Confirmed in source: `wrap_child` computes `pgid = Pid::from_raw(inner.id().expect(...))`. Record `child.id()` at spawn and any task can `killpg` independently. This is also what Phase 17 needs for crash reconciliation and orphan reaping, so recording it now costs nothing.

**3. `spawn()` returns `Box<dyn ChildWrapper>`, so `ProcessGroupChild::pgid()` is not directly reachable.** `pub fn pgid(&self) -> u32` exists on the concrete type, but `downcast_ref` on `dyn ChildWrapper` is private. Use `child.id()` per gotcha 2 — same value, no downcast.

**4. `ProcessGroupChild::wait()` already reaps the whole group.** It awaits the leader, then loops `waitpid(-pgid, WNOHANG)` up to 10 times, then falls back to a blocking reap on `spawn_blocking`. This satisfies Pitfall 6's "wait() afterward or you leave zombies" — but only if `wait()` is actually awaited to completion. Dropping the future mid-reap leaves the grandchildren unreaped.

### Sketch (composition order matters — wrappers stack)

```rust
use process_wrap::tokio::*;
use std::process::Stdio;

let mut wrap = CommandWrap::with_new("claude", |cmd| {
    cmd.args(argv)
       .current_dir(project_root)
       .env("CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS", bg_ceiling_ms.to_string()) // D-14
       .stdin(Stdio::piped())
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());
});
wrap.wrap(ProcessGroup::leader());
wrap.wrap(KillOnDrop);          // backstop only — never the teardown story (Pitfall 6)

let mut child = wrap.spawn()?;
let pgid   = child.id().ok_or(/* reaped before we read the pid */)?;
let stdin  = child.stdin().take().expect("piped");
let stdout = child.stdout().take().expect("piped");
let stderr = child.stderr().take().expect("piped");
```

---

## The `Executor` Trait Revival

The v1.2 trait (`.planning/milestones/v1.2-phases/13-queue-execution-research/QUEUE-EXECUTION-DESIGN.md` §11) reconciled with ARCHITECTURE §7.1's amendments and this session's probe findings.

### v1.2 original

```rust
trait Executor {
    fn start(project_dir: PathBuf, command: String, options: ExecutionOptions) -> Result<ExecutionHandle>;
    fn cancel(handle: &ExecutionHandle) -> Result<()>;
    fn is_running(handle: &ExecutionHandle) -> bool;
}
struct ExecutionHandle { id: ExecutionId, session_id: Option<String>, events: Receiver<ExecutionEvent> }
enum ExecutionEvent { Output(String), Progress(ProgressUpdate), Checkpoint(CheckpointInfo), Completed(ExitStatus), Error(String) }
struct ExecutionOptions { timeout, budget_usd, auto_approve, model, resume_session, name }
```

### Reconciled shape for Phase 15

```rust
#[async_trait]                      // or hand-rolled; the project has no async-trait dep today
pub trait Executor {
    fn start(&self, project: &DrivableProject, command: String, options: ExecutionOptions)
        -> Result<ExecutionHandle, SpawnError>;          // D-23: capability token, not a path
    async fn send(&self, h: &mut ExecutionHandle, msg: UserMessage) -> Result<(), SendError>;
    async fn interrupt(&self, h: &mut ExecutionHandle) -> Result<InterruptAck, SendError>;
    async fn cancel(&self, h: &mut ExecutionHandle) -> Result<RunOutcome, ()>;
    fn is_running(&self, h: &ExecutionHandle) -> bool;
    fn capabilities(&self, h: &ExecutionHandle) -> &[String];
}
```

`ExecutionHandle` amendments (§7.1 asks for a stdin sink and `capabilities`; the probes add three more):

| Field | Source | Why |
|-------|--------|-----|
| `id: ExecutionId` | v1.2 | unchanged |
| `session_id: String` | **now non-`Option`** | We generate the UUID pre-spawn via `--session-id` (D-01), so it is always known. |
| `events: mpsc::Receiver<ExecutionEvent>` | v1.2 | unchanged |
| `stdin: Option<ChildStdin>` (or a writer-task command sender) | §7.1 | the injection channel |
| `capabilities: Vec<String>` | §7.1 | from first `system/init` |
| `pgid: u32` | **probe** | teardown from an independent task; Phase 17 reuses it for reaping |
| `claude_code_version: String` | **D-07** | recorded so Phase 16 journals it without a signature change |
| `pending_control: HashMap<String, oneshot::Sender<ControlResponse>>` | **probe** | `control_request`/`control_response` is request/response over a shared stream — `interrupt()` must correlate on `request_id` |

`ExecutionEvent` — v1.2's five variants map onto stream-json, plus what the probes require:

```rust
pub enum ExecutionEvent {
    SessionStarted { session_id: String, capabilities: Vec<String>,
                     claude_code_version: String, api_key_source: Option<String> },
    Message(StreamMessage),               // typed, parsed
    Unknown { raw: String },              // D-09 forward-compat: carried, never fatal
    Unparseable { raw: String, error: String },   // torn/invalid line — distinct from Unknown
    Stderr(String),                       // D-04 keeps stderr separable
    TurnCompleted(Box<ResultMessage>),    // one PER TURN — see Contradictions #1
    Cost { cumulative_usd: f64 },         // §7.1 asked for this; total_cost_usd is cumulative
    Exited(ExitStatus),
}
```

`ExecutionOptions` — every v1.2 field maps to a real flag; §7.1 adds `session_id` and `target`; the probes add the deadlines:

| Field | Flag / mechanism | Note |
|-------|------------------|------|
| `session_id: Uuid` | `--session-id` | generated pre-spawn |
| `target: ExecutionTarget` | argv prefix | `Host` only in this phase (D-21) |
| `budget_usd: Option<f64>` | `--max-budget-usd` | **works under subscription (OQ3)**; post-turn only |
| `model: Option<String>` | `--model` | |
| `resume_session: Option<String>` | `--resume <id>` | never bare `-r` — interactive picker in `-p` is a hang |
| `permission_mode` | `--permission-mode dontAsk` | D-15 |
| `setting_sources` | `--setting-sources project` | D-01; config toggle, default off for `user` |
| `wall_clock_cap: Duration` | supervisor | D-13 |
| `idle_cap: Duration` | supervisor, from last stream line | D-13 |
| `bg_wait_ceiling_ms` | `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` | D-14; set explicitly, never inherit |
| `name: Option<String>` | `-n/--name` | v1.2 field, still real |

**On `capabilities()` and backend honesty (§7.1):** keep the trait, but the doc comment should say plainly that `send`/`interrupt` are a *Claude capability tier*, not backend parity — a hypothetical Codex/aider backend satisfies `start`/`cancel` and not the rest. The probe reinforces this: `interrupt_cancel_queued_v1` is a Claude-specific protocol capability with a Claude-specific `still_queued` response shape.

**One dependency note:** the project has no `async-trait` today, and Rust 2021 does not support `async fn` in traits with `dyn` dispatch. Options: (a) add `async-trait`, (b) return `Pin<Box<dyn Future>>` by hand, (c) make the trait's async methods take `&self` and return concrete futures with static dispatch only. Since there is exactly one implementor in this phase and Phase 22 adds a *target*, not an impl (AP5), **(c) — a concrete `ClaudeExecutor` with the trait kept object-safe by boxing the futures — avoids a new dependency.** Planner's call; flagged because it is a real Cargo.toml consequence hiding inside a "just revive the trait" task.

---

## Architecture Patterns

### System architecture

```
                    ┌─────────────────────────────────────────────────────┐
   keypress ───────▶│  crossterm EventStream ──┐                          │
   250ms tick ─────▶│  spawn_tick ─────────────┤                          │
   fs change ──────▶│  FileWatcher ────────────┴──▶ Action mpsc (FIFO)    │
                    └──────────────────────────────────────┬──────────────┘
                                                           │
                    ┌──────────────────────────────────────▼──────────────┐
                    │  run_tui_loop  —  tokio::select! { biased; ... }    │
                    │   arm 1: Action rx        (input first — D-17)      │
                    │   arm 2: ExecEvent rx     (drained in bounded batch)│
                    │   arm 3: redraw tick                                │
                    └──────────────────────────────────────▲──────────────┘
                                                           │ ExecEvent mpsc
                                                           │ (SEPARATE channel)
   ┌───────────────────────────────────────────────────────┴──────────────┐
   │  ClaudeExecutor                                                       │
   │                                                                       │
   │   DrivableProject ──▶ argv build ──▶ CommandWrap                      │
   │                                       .wrap(ProcessGroup::leader())   │
   │                                                │                      │
   │                                        spawn() │  records pgid        │
   │                       ┌────────────────────────┼──────────────────┐   │
   │                       ▼                        ▼                  ▼   │
   │              stdout reader task        stderr reader task   writer task│
   │              BufReader::lines()        BufReader::lines()    (stdin)   │
   │                       │                        │                  ▲   │
   │              parse_line() ── D-09              │                  │   │
   │                       │                        │           send() │   │
   │                       ▼                        ▼         interrupt()  │
   │              ┌────────────────────────────────────┐                   │
   │              │ FIRST system/init ──▶ capability   │                   │
   │              │   gate (D-06) ──▶ pass? ──────────────▶ release first  │
   │              │                     fail? ─▶ teardown  │  user message │
   │              │   + --bare guard: apiKeySource=="none" │  (D-02)       │
   │              └────────────────────────────────────┘                   │
   │                       │                                               │
   │              each `result` ──▶ TurnCompleted (N per run!)             │
   │                       │                                               │
   │   supervisor task: select! { wait(), wall_cap, idle_cap, cancel }     │
   │              breach ──▶ signal(SIGTERM) ─▶ 10s ─▶ start_kill ─▶ wait  │
   └───────────────────────────────────────────────┬───────────────────────┘
                                                   │ at run end
                    ┌──────────────────────────────▼───────────────────────┐
                    │ Outcome derivation (TRANS-02)                        │
                    │   last ResultMessage  +  ExitStatus                  │
                    │   +  git HEAD/dirty delta (before vs after)          │
                    │   +  parse_project_state fingerprint (before/after)  │
                    │                    ──▶ RunOutcome                    │
                    └──────────────────────────────────────────────────────┘
```

### Recommended module layout (per D-21)

```
src/executor/
├── mod.rs           # Executor trait, ExecutionOptions, ExecutionHandle,
│                    # ExecutionEvent, RunOutcome, ExecutionTarget{Host},
│                    # DrivableProject (the D-23 capability token)
├── claude.rs        # ClaudeExecutor: argv build, spawn, pipe tasks,
│                    # capability gate, supervisor, teardown
├── stream_json.rs   # serde types for the NDJSON protocol (D-09)
└── outcome.rs       # (optional) outcome-derivation matrix + disk/git snapshot
```

### Pattern 1: Pipe topology — three tasks, never two

**What:** stdout reader, stderr reader, and stdin writer are three independent `tokio::spawn`ed tasks. The task that awaits process exit is a *fourth* (the supervisor) and owns no pipe.

**Why:** D-04 names the two-pipe deadlock, and the probes make it concrete. Claude waits for queued output to drain before exiting (capped at 30s), and probe 05 shows it holding a 68-second turn open while stdin stays open. A task that both writes stdin and awaits exit will deadlock: the child blocks writing stdout while the parent blocks writing stdin.

**Also:** dropping the stdin handle is EOF, and EOF means "no more input", not "stop" — probe 05 closed stdin at t=14s and the run continued to t=71s, draining its queue, then exited 0. So `send()` must keep the handle alive for the whole run, and the writer task's shutdown is what ends the run cleanly.

### Pattern 2: The capability gate — spawn, gate, then release the prompt

**What:** spawn the process, read events until the **first** `system/init`, validate, and only then write the first user message.

**Why it makes "refused up front" literally true (TRANS-04 criterion 4):** because the prompt is delivered over stdin (D-02) rather than as a positional argument, no turn ever starts before validation. The refusal costs zero tokens and zero quota. This is not achievable with a positional prompt — the CLI would begin the turn immediately on startup.

Checks to run at the gate:

1. `capabilities[]` ⊇ required set. Missing → typed error naming the missing capability. Required on 2.1.220: `interrupt_receipt_v1` (for `interrupt()`), `interrupt_cancel_queued_v1` (for `still_queued` accounting), `msg_lifecycle_v1`.
2. `apiKeySource == "none"` — the D-08 `--bare` regression guard. Absent field → fail (see the `system/init` section).
3. `claude_code_version` ≥ 2.1.214 (D-07). Refuse below, naming the observed version; warn-and-proceed above 2.1.220.
4. Record `claude_code_version`, `session_id`, `capabilities`, `permissionMode` into the handle.

**Amendment from the probes:** gate on the **first** init only. Probe 05 shows a second `system/init` per queued turn. Re-running the gate there would be harmless for check 1 but would make check 2 a mid-run abort — precisely the "mysteriously context-free agent mid-run" failure D-08 is trying to convert into a *start-time* failure.

### Pattern 3: The `tokio::select!` restructure of `run_tui_loop`

**Current shape** (`src/main.rs:134-205`, read directly this session): a single `rx.recv().await` at `:151` over one unbounded mpsc, with the blocking `pending_editor` shell-out at `:156-197` inside the same loop body.

**Required shape** per D-17 — biased ordering, separate high-volume channel, bounded batch drain:

```rust
async fn run_tui_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    rx: &mut mpsc::UnboundedReceiver<Action>,
    exec_rx: &mut mpsc::Receiver<ExecutionEvent>,   // SEPARATE, bounded
) -> anyhow::Result<()> {
    const EXEC_BATCH: usize = 64;               // bounded drain per iteration
    let mut redraw = tokio::time::interval(Duration::from_millis(16));
    redraw.set_missed_tick_behavior(MissedTickBehavior::Delay);

    loop {
        if app.ctx.needs_redraw { app.needs_redraw = true; app.ctx.needs_redraw = false; }
        if app.needs_redraw {
            terminal.draw(|f| gsd_meta_manager::ui::render(f, app))?;
            app.needs_redraw = false;
        }

        tokio::select! {
            biased;                                   // arm order IS the priority order

            // 1. input + existing actions first — control keys never queue behind stream traffic
            Some(action) = rx.recv() => { app.update(action); }

            // 2. executor events, drained in a bounded batch so a burst can't starve a redraw
            Some(ev) = exec_rx.recv() => {
                app.apply_exec_event(ev);
                for _ in 1..EXEC_BATCH {
                    match exec_rx.try_recv() {
                        Ok(ev) => app.apply_exec_event(ev),
                        Err(_) => break,
                    }
                }
            }

            // 3. the loop's own timer — it can now make progress with no message at all
            _ = redraw.tick() => { /* redraw driven by needs_redraw above */ }
        }

        // pending_editor arm preserved verbatim from :156-197 (D-18)
        if let Some(path) = app.pending_editor.take() { /* ...unchanged... */ }
        if app.should_quit { break; }
    }
    Ok(())
}
```

**What actually breaks, and the regression risk:**

| Concern | Assessment |
|---------|------------|
| `EventBus` / `event.rs` | **No change.** ARCHITECTURE §1.1 is right that upstream multiplexing already works; `spawn_crossterm_reader` and `spawn_tick` keep cloning one `tx`. `event.rs` is explicitly listed as unchanged in §8.2. |
| `spawn_tick(250)` | **Keep it.** It drives the 20-tick session poll (`app.rs:246-257`) and the 3s status-message expiry (`:237-244`). The new `redraw` interval is a *separate* concern; removing `spawn_tick` would silently break session detection. |
| `biased;` + `rx.recv()` | With `biased`, arm 1 is polled first every iteration. If `Action` traffic is continuous, arms 2 and 3 can starve. Today's producers are keys, a 250ms tick, and debounced fs events — not continuous. **But** if a future phase routes anything high-volume through `Action`, this becomes a live starvation bug. D-17's "do not funnel per-token stream output through the `Action` FIFO" is load-bearing, not stylistic. Worth a comment at the arm. |
| `Some(x) = rx.recv()` in `select!` | When a channel closes, `recv()` returns `None`, the arm's pattern fails, and `select!` **disables that arm**. If *all* arms are disabled `select!` panics unless there's an `else`. Since `exec_rx` legitimately closes when no run is active, either keep a sender alive for the process lifetime (simplest — store a `tx` clone on `AppContext`) or add an `else => break`. **This is the most likely concrete bug in the restructure.** |
| `pending_editor` blocking shell-out | Preserved (D-18). It still blocks the loop, and during a run the executor's `mpsc` will buffer meanwhile — which is correct behaviour, not a leak, provided `exec_rx` is **bounded** so a long editor session applies backpressure rather than growing without limit. Argues for `mpsc::channel(N)` over `unbounded_channel` for the executor path. |
| `terminal.draw` position | Currently before the await; keeping it there means one draw per loop iteration. With a 16ms timer arm the loop now wakes even when idle — check that `needs_redraw` gating still prevents a busy redraw. It does, because `draw` is guarded by `app.needs_redraw`. |
| Existing tests | `src/ui/screens/normal.rs:870-878` already uses `ratatui::backend::TestBackend`. No existing test drives `run_tui_loop`, so there is no test to break — and that is itself the gap the TRANS-03 verification must fill. |

**Testability lever:** the single most useful refactor is to extract the `select!` body into a free function, e.g. `async fn pump(app: &mut App, rx: &mut …, exec_rx: &mut …, batch: usize) -> PumpOutcome`. `run_tui_loop` then becomes draw + `pump` + editor + quit-check. This makes TRANS-03's proof possible without a terminal (see below) and is a small, contained change well inside D-18's "beyond what the restructure strictly requires" line.

### Anti-patterns to avoid

- **Treating the first `type:"result"` as run completion.** Truncates every steered run. See Contradictions #1.
- **`child.kill()` as the teardown story.** Goes straight to SIGKILL, skipping `claude`'s clean SIGTERM path (turn abort, Bash-tree teardown, `SessionEnd` hooks, exit 143). Use `signal(15)` first.
- **Putting driver state on `ProjectState`.** It derives `PartialEq` and `app.rs` uses that equality to suppress "Updated: {alias}" status spam. Driver state changes every few seconds — probe 05 emitted 22 `thinking_tokens` in 68s. D-19.
- **Any process handle inside an `Action`.** `Action` derives `Clone`; `ChildStdin`/`JoinHandle` are not. D-19.
- **`deny_unknown_fields` anywhere in `stream_json.rs`.** Nine new `system/init` fields and one new `system` subtype appeared between the research pass and this session, on the same CLI version line. D-09.
- **Treating `control_response{subtype:"success"}` as "the turn was cancelled."** It means "request accepted". Probe 07 is the counterexample. Read `still_queued`.
- **Using `duration_api_ms` as a wall clock.** It is cumulative and can exceed `duration_ms` (probe 03: 7621 vs 6351).
- **Adding `--bare`.** D-08. The regression guard exists because Anthropic states it will become the `-p` default.

---

## Don't Hand-Roll

| Problem | Don't build | Use instead | Why |
|---------|-------------|-------------|-----|
| Killing the whole `claude` process tree | manual `fork`/`setpgid`/`killpg` | `process-wrap` `ProcessGroup::leader()` | Handles pgid capture, group signalling, and the multi-pass zombie reap (`waitpid(-pgid, WNOHANG)` ×10 then a blocking fallback) that a hand-rolled version invariably omits. |
| Git HEAD / dirty status for the TRANS-02 delta | a new git module | `src/state_reader/git_ops.rs` | Already shells out to git with the project's established idioms. `project_last_activity()` (`:92`) resolves last-commit time with an mtime fallback; `load_git_log()` (`:114`) and `load_diff_stat()` (`:161`) are already `async`. **Note:** there is no `head_sha()` / `is_dirty()` helper today — those are genuinely new, but they belong *in this file*, next to the existing ones, not in a parallel module (D-11 says "do not write a parallel one"). |
| `.planning/` artifact fingerprint for the disk delta | a new artifact scanner | `state_reader::parse_project_state()` (`mod.rs:103`) + `disk_status::DiskInference` | `parse_project_state` is synchronous, idempotent, never panics, and returns a `ProjectState` that already derives `PartialEq` — so the before/after comparison is literally `before != after`. `DiskInference` carries 22 artifact booleans per phase. This is the single highest-leverage reuse in the phase: D-11's "made_changes signal" is a one-line `PartialEq` on a type that already exists. Call it from `spawn_blocking` per the `app.rs:289-295` precedent. |
| NDJSON line framing | a hand-rolled byte scanner | `tokio::io::BufReader::lines()` | Handles partial reads, CRLF, and UTF-8 boundaries. |
| Session id generation | timestamp/counter strings | `uuid` v4 + `--session-id` | The CLI validates it as a UUID; generating pre-spawn makes a crash before first stdout resumable. |
| Tolerant enum dispatch over `type` | a hand-written `match` on `serde_json::Value` | `#[serde(tag = "type")]` + `#[serde(other)]` | Verified compiling below. A hand-rolled `Value` match loses type safety on every payload and re-implements what serde already does. |
| Atomic write of any state file | `File::create` + write | `src/config.rs` tempfile+persist idiom | Established project pattern. (Phase 16's concern, noted so Phase 15 doesn't invent a second one.) |

**Key insight:** TRANS-02's disk/git half looks like new work and is mostly not. `parse_project_state` + `PartialEq` gives the artifact delta for free, and `git_ops.rs` is the right home for the two small git helpers that genuinely are new. The temptation to build a fresh "run diff" module is exactly what D-11 forbids.

---

## Tolerant serde Modelling of the stream-json Envelope

D-09 asks for `#[serde(tag = "type")]` with a catch-all that **preserves the raw line**. There is a subtlety that will bite a planner who assumes it works the obvious way, and a working shape that resolves it.

### The subtlety

`#[serde(other)]` is supported on internally-tagged enums, **but only on a unit variant.** You cannot write `#[serde(other)] Unknown(serde_json::Value)`. So the enum itself *cannot* carry the raw body.

The resolution is to preserve the raw line **outside** the enum, in the reader task that already owns the `String`. That also cleanly separates two cases a single catch-all would conflate: an *unknown message type* (forward-compat, carry it) versus a *malformed line* (torn write / truncation, a real diagnostic).

### Working shape — compiled and executed this session

The following was written to a scratch crate, built, run against real captured envelopes, and passed `cargo clippy --all-targets -- -D warnings` clean. It is not a sketch.

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamMessage {
    System(SystemMessage),
    Assistant(TurnMessage),
    User(TurnMessage),
    Result(Box<ResultMessage>),        // boxed: clippy::large_enum_variant, per action.rs:21 precedent
    ControlResponse(ControlResponse),
    RateLimitEvent(serde_json::Value),
    #[serde(other)]
    Unknown,                            // unit-only — raw body preserved by the caller
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "subtype", rename_all = "snake_case")]
pub enum SystemMessage {
    Init(Box<InitMessage>),
    #[serde(other)]
    Other,                              // hook_started / hook_response / thinking_tokens / api_retry / future
}

#[derive(Debug, Clone, Deserialize)]
pub struct InitMessage {
    pub session_id: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(rename = "apiKeySource")]              // camelCase — see the casing warning above
    pub api_key_source: Option<String>,
    pub claude_code_version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TurnMessage {
    pub session_id: Option<String>,
    #[serde(default, rename = "isReplay")]
    pub is_replay: bool,                            // absent on non-replay messages, so `default`
    pub uuid: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ResultMessage {
    pub subtype: String,                            // String, NOT an enum — new subtypes ship at patch level
    pub is_error: bool,
    pub terminal_reason: Option<String>,            // ditto
    #[serde(default)]
    pub permission_denials: Vec<serde_json::Value>,
    #[serde(default)]
    pub errors: Vec<String>,                        // present only on error envelopes
    pub result: Option<String>,                     // ABSENT on error envelopes
    pub total_cost_usd: Option<f64>,                // cumulative across turns
    pub num_turns: Option<u64>,                     // per-turn, resets
}

#[derive(Debug, Clone, Deserialize)]
pub struct ControlResponse { pub response: ControlResponseBody }

#[derive(Debug, Clone, Deserialize)]
pub struct ControlResponseBody {
    pub subtype: String,
    pub request_id: String,
    #[serde(default)]
    pub response: Option<serde_json::Value>,        // carries { "still_queued": [...] } — note double nesting
}

/// What the reader task yields. Parsing NEVER fails a run.
#[derive(Debug, Clone)]
pub enum Envelope {
    Parsed { raw: String, msg: StreamMessage },
    Unparseable { raw: String, error: String },
}

pub fn parse_line(raw: &str) -> Envelope {
    match serde_json::from_str::<StreamMessage>(raw) {
        Ok(msg) => Envelope::Parsed { raw: raw.to_string(), msg },
        Err(e)  => Envelope::Unparseable { raw: raw.to_string(), error: e.to_string() },
    }
}
```

### Verified behaviour

Run against eight inputs — six real captured envelopes plus two adversarial ones:

| Input | Result |
|-------|--------|
| `system/init` **with an injected unknown field** `"brand_new_field":1` | `System(Init(InitMessage{…}))` — unknown field silently ignored |
| `system/thinking_tokens` | `System(Other)` — new subtype absorbed, no error |
| `user` with `"isReplay":true` | `User(TurnMessage{ is_replay: true, … })` |
| `result` `error_max_budget_usd` (no `result` field) | `Result(ResultMessage{ result: None, errors: ["Reached maximum budget ($0.00001)"], … })` |
| `result` `success` | `Result(ResultMessage{ result: Some("PONG"), errors: [], … })` |
| `control_response` with nested `still_queued` | `ControlResponse{ response: { request_id: "req_1", response: Some({"still_queued": []}) } }` |
| **synthetic** `{"type":"totally_new_message_type_from_2_2_0", …}` | `Unknown` — a hypothetical 2.2.0 message type is carried, not fatal |
| **synthetic torn line** `{"type":"result","subtype":"suc` | `Unparseable{ error: "EOF while parsing a string at line 1 column 31" }` — correctly distinguished from `Unknown` |

Two design notes worth carrying into the plan:

1. **`subtype` and `terminal_reason` are `String`, not enums.** This session added three new values on one CLI version; a typed enum would need `#[serde(other)]` on each and would still lose the actual string. Match on `&str` in the outcome-derivation matrix and keep an explicit `_ =>` fallback arm.
2. **Internally-tagged enums buffer content** through serde's private `Content` intermediate, so they are slower than a plain struct. At the observed volume (tens of events per turn, a few thousand per long run) this is irrelevant — but it is another reason parsing lives in the reader task and not on the render thread.

---

## Testing Without Spawning `claude` (D-24)

### The repo's actual conventions

- `#[cfg(test)] mod tests` in-file is the dominant pattern — 18 source files use it.
- `tests/` exists with two integration tests: `registry_test.rs` and `state_reader_test.rs`.
- **`assert_fs` is used in exactly one place** — `tests/registry_test.rs:1-2` (`assert_fs::prelude::*`, `assert_fs::TempDir`). `tests/state_reader_test.rs` uses `tempfile::TempDir` instead. So the dev-dependency is real but not dominant; `tempfile::TempDir` is the more common choice and is already a *runtime* dependency.
- `ratatui::backend::TestBackend` already has a precedent at `src/ui/screens/normal.rs:870-878`.

### The three fake-`claude` options, weighed

| Option | Mechanics | Verdict |
|--------|-----------|---------|
| **A. Shell script fixture** — `tests/fixtures/fake-claude.sh` that `cat`s a transcript with optional delays | Executor is pointed at the script path. Zero Cargo surface. | **Simplest.** Unix-only, but driving is Unix-only by construction (`process_group(0)`; `session_detector.rs` is already `/proc`-based). Downside: needs the exec bit preserved in git, and expressing stdin-reactive behaviour (echo an injected message) in shell is fiddly. |
| **B. `[[bin]]` with `required-features`** — `tests/bin/fake_claude.rs` + `[[bin]] name="fake-claude" path="tests/bin/fake_claude.rs" required-features=["test-fixtures"]` | Real Rust, full control over timing and stdin reactions. | Adds a Cargo feature and a binary target to a published crate (this crate *is* published — `keywords`/`categories`/`repository` are set). Users would see a feature they must never enable. |
| **C. Self-reexec** — the test binary re-invokes `std::env::current_exe()` with a magic env var; `main`-equivalent short-circuits into fixture mode | No extra artifacts, no feature, cross-platform. | Standard pattern, but this crate's `main.rs` is the TUI entry point — adding a fixture branch there pollutes production code, which is exactly what D-18/Pitfall 10 warn against. |

### Recommendation: A, with a Rust helper for the reactive cases

Split by what's being tested, because most of the surface doesn't need a process at all:

1. **Pure parsing / outcome derivation / capability gate / `--bare` guard / tolerant parsing** — no process, no fixture binary. Feed the captured transcripts through `parse_line()` and the outcome matrix directly in `#[cfg(test)] mod tests`. **This covers the large majority of D-24's stated targets and all of D-26's matrix**, with zero spawn cost. Load the fixtures with `include_str!("../../tests/fixtures/transcripts/05-queued-injection-two-turns.ndjson")` — compile-time, no I/O, no `TempDir`.

2. **Spawn/pipe/teardown/deadline mechanics** — needs a real child. Use option **A**, a script at `tests/fixtures/fake-claude.sh`, in a handful of variants:
   - `replay.sh <transcript>` — cat a transcript line by line, exit with a given code.
   - `slow.sh` — emit `system/init`, then nothing (exercises the D-13 idle cap).
   - `spawner.sh` — emit `system/init`, then `sh -c 'sleep 300' &` (exercises the D-14 process-group kill; assert the grandchild is gone via `kill -0` on its pid).
   - `deaf.sh` — ignore SIGTERM (`trap '' TERM`) to force the SIGKILL escalation path.

   The `spawner.sh` and `deaf.sh` variants are the ones that genuinely justify a real process — they are the only way to prove the process-group teardown actually works, and a Rust fixture binary would not make them easier.

3. **`send()` / `interrupt()` correlation** — the only case where a reactive fixture helps. A small `awk`/`sh` loop reading stdin and echoing an `isReplay` line, or a `#[cfg(test)]`-only helper, is enough. Given it's one narrow case, keeping it in shell alongside the others avoids introducing a Cargo feature for a single test.

**Guard the process-spawning tests.** They are Unix-only and slower; gate with `#[cfg(unix)]` and consider `#[ignore]` for the ones with real 10-second grace periods so `cargo test` stays fast, with a documented `cargo test -- --ignored` in `docs/TESTING.md`.

---

## Verifying Success Criterion 3 (TUI responsive under stream load)

This is the least obvious of the four criteria to verify, and the honest starting point is: **you cannot prove UI responsiveness with a test on a real terminal, and a wall-clock latency assertion will be flaky in CI.** But the criterion decomposes into two properties that *are* deterministically provable.

### Property 1 (the real one): a keypress is never queued behind stream traffic

This is the actual TRANS-03 requirement, and it is a *priority* property, not a timing one. Extract the `select!` body into `pump()` (see Pattern 3's testability lever), then:

```rust
#[tokio::test]
async fn keypress_is_handled_before_a_flood_of_executor_events() {
    let (act_tx, mut act_rx) = mpsc::unbounded_channel();
    let (exec_tx, mut exec_rx) = mpsc::channel(16_384);

    // flood the executor channel FIRST — a naive FIFO would drain all of these first
    for _ in 0..10_000 { exec_tx.try_send(dummy_stream_event()).unwrap(); }
    // then enqueue exactly one keypress
    act_tx.send(Action::RawKey(key(KeyCode::Char('q')))).unwrap();

    let mut app = App::new_for_test();
    // one pump iteration
    pump(&mut app, &mut act_rx, &mut exec_rx, EXEC_BATCH).await;

    // biased; puts input first, so the key wins on iteration ONE
    assert!(app.should_quit, "keypress starved behind {} queued executor events",
            10_000 - exec_rx.len());
}
```

Deterministic, no terminal, no timing, no flake. It fails loudly if someone removes `biased;` or merges the executor channel into the `Action` FIFO — which is exactly the regression D-17 is guarding against.

### Property 2: a burst cannot starve a redraw

The bounded-batch requirement. Assert that draining N events takes ⌈N / EXEC_BATCH⌉ iterations rather than one, i.e. that control returns to the loop head:

```rust
#[tokio::test]
async fn executor_burst_is_drained_in_bounded_batches() {
    // ... seed 10_000 events, no actions ...
    pump(&mut app, &mut act_rx, &mut exec_rx, 64).await;
    assert_eq!(exec_rx.len(), 10_000 - 64,
               "one pump iteration must drain at most EXEC_BATCH events");
}
```

Together these two tests prove the mechanism the criterion depends on.

### Property 3 (supporting): frames actually render during a flood

Uses the existing `TestBackend` precedent to prove "keeps redrawing" in the criterion's literal words:

```rust
#[tokio::test]
async fn tui_renders_repeatedly_while_the_executor_channel_is_saturated() {
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    // spawn a task that floods exec_tx continuously for ~2s,
    // run the real loop, count terminal.draw() invocations
    assert!(draw_count > 20, "only {draw_count} frames rendered under load");
}
```

Slightly timing-flavoured but robust with a generous bound — it fails only if redraws stop entirely, which is the actual failure mode.

### What NOT to do

Do not write a p99-keypress-latency assertion against a wall clock. It will pass on a developer laptop and fail on a loaded CI runner, and it will get `#[ignore]`d within a month — at which point the criterion has no verification at all. If a latency measurement is wanted, make it an explicitly `#[ignore]`d benchmark documented in `docs/TESTING.md`, never a gate.

### Realistic load parameters (from the probes)

A 68-second turn emitted 22 `thinking_tokens` + 2 `assistant` + 1 `user` + 1 `init` + 1 `result` ≈ 27 events, i.e. **well under 1 event/second** *without* `--include-partial-messages`. With that flag (token-level `stream_event` deltas) the rate is orders of magnitude higher. So:

- The flood tests above should use 10k+ events precisely because normal operation will *not* stress the loop — the test must manufacture the pressure the design is guarding against.
- Phase 15 should **not** pass `--include-partial-messages` (D-01's argv baseline correctly omits it). If Phase 18 later wants token-level rendering, the bounded-batch drain is what makes that safe, and these tests are what prove it still holds.

---

## Common Pitfalls

### Pitfall A: Returning on the first `type:"result"`

**What goes wrong:** a steered run reports "succeeded" while later turns are still executing; the executor tears down mid-work.
**Why:** every research artifact describes `result` as *the* completion record, which is true only for single-message runs.
**How to avoid:** `result` → `TurnCompleted`. The run ends on stdin EOF → process exit. Derive `RunOutcome` from the last `result` + exit status + disk delta.
**Warning sign:** the reader loop contains `if msg.is_result() { break }`.

### Pitfall B: `child.kill()` instead of `signal(SIGTERM)`

**What goes wrong:** `claude` is SIGKILLed, so it never aborts its turn cleanly, never tears down its Bash tree via its own handler, never runs `SessionEnd` hooks, and never exits 143. Grandchildren may survive the first pass.
**Why:** `ProcessGroupChild::start_kill()` is SIGKILL, and `kill()` = `start_kill()` + `wait()`. Reading "kill" as "terminate politely" is natural and wrong.
**How to avoid:** `signal(15)` → 10s → `start_kill()` → `wait().await`. D-14.
**Warning sign:** `.kill().await` anywhere in the teardown path.

### Pitfall C: The `select!` arm that disables itself

**What goes wrong:** `exec_rx` closes when a run ends; its arm's `Some(ev) = …` pattern fails; the arm is permanently disabled. If every arm disables, `select!` panics.
**Why:** `select!` disables an arm whose pattern doesn't match, for the remainder of that `select!`'s life in the loop — a well-known sharp edge.
**How to avoid:** hold a long-lived `exec_tx` clone on `AppContext` so the channel never closes, or add `else => break`.
**Warning sign:** the executor channel is created per-run rather than once for the process lifetime.

### Pitfall D: Trusting `control_response{subtype:"success"}`

**What goes wrong:** the UI reports "interrupted" and the turn keeps streaming.
**Why:** `success` acknowledges the *request*. Probe 07 got `success` with `still_queued: []` while cancelling nothing meaningful.
**How to avoid:** correlate `request_id`, inspect `response.response.still_queued`, and treat the subsequent `[Request interrupted by user]` user message plus `terminal_reason:"aborted_streaming"` as the actual confirmation.

### Pitfall E: Idle-cap timer driven by the wrong clock

**What goes wrong:** a legitimate long run is killed, or a hook-hung run is not.
**Why:** `duration_api_ms` is cumulative and can exceed `duration_ms` (probe 03: 7621 > 6351). `duration_ms` only arrives *with* the `result`, i.e. too late to be a detector.
**How to avoid:** the idle timer is `Instant::now()` recorded by the **reader task** on every line it observes. D-13.

### Pitfall F: `#[serde(rename_all = "camelCase")]` on `InitMessage`

**What goes wrong:** `claude_code_version` silently deserialises as `None`; the D-07 version gate passes everything.
**Why:** `system/init` mixes casing — `apiKeySource`/`permissionMode` are camelCase, `claude_code_version`/`session_id`/`output_style` are snake_case.
**How to avoid:** per-field `#[serde(rename)]`, and make the version gate fail closed on `None`.

### Pitfall G: A committed fixture with unredacted paths

**What goes wrong:** the golden transcripts embed `/home/blk` and `/tmp/claude-1000` in `cwd` and `memory_paths.auto`, permanently. PITFALLS is explicit that logs written before a redaction retrofit stay unredacted forever.
**How to avoid:** normalise paths *and* UUIDs at check-in time (D-25). Host-independent fixtures are also a correctness win — an assertion on `cwd` would otherwise only pass on one machine.

---

## Runtime State Inventory

Phase 15 is additive (new modules, one restructured loop, one MSRV key) rather than a rename/refactor, so most categories are empty — but the phase does *spawn processes*, which creates genuinely new runtime state.

| Category | Items found | Action required |
|----------|-------------|-----------------|
| Stored data | **None.** Phase 15 writes no journal, no `run.json`, no cache (that is Phase 16). | None — verified against D-21's module list and the Phase 16 deferral. |
| Live service config | **None.** No external service is configured by this phase. | None. |
| OS-registered state | **New, and this phase creates it:** each run creates a **process group** whose pgid is held only in memory. A TUI crash mid-run orphans that group with no on-disk record to reap it from. | Phase 15 must record the pgid on the `ExecutionHandle` and tear the group down on normal exit. **Durable pgid recording and crash reconciliation are explicitly Phase 17's** — but Phase 15 should install a panic hook / shutdown path that kills live groups, because `color-eyre`'s handler will not reap children. |
| Secrets / env vars | Phase 15 **sets** `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` on the child (D-14). It reads no secrets; subscription auth is inherited ambiently via OAuth/keychain and confirmed working (`apiKeySource:"none"`). | Set the env var explicitly rather than inheriting the silent 10-minute default. No secret handling. |
| Build artifacts | `Cargo.lock` gains `process-wrap`, `uuid`, `nix`, `indexmap` (+ `tokio-util` if D-05 lands that way). Adding `rust-version = "1.87"` makes the floor machine-checkable for the first time. | Normal `cargo build`. Verify the 5-lint `--all-targets` clippy budget is unchanged after the new modules land. |

**Also new and easy to miss:** the child inherits the parent's environment. If the TUI is itself launched from inside a Claude Code session (plausible — this repo is developed that way), variables like `CLAUDE_CODE_*` may already be set in the parent and would leak into the driven child. Worth an explicit env scrub of `CLAUDE_*` on the spawned command, or at minimum a documented decision not to.

---

## Environment Availability

| Dependency | Required by | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `claude` CLI | the entire phase | ✓ | **2.1.220** (≥ D-07's 2.1.214 floor) | none — hard requirement |
| Claude subscription auth | every probe and every run | ✓ | `apiKeySource: "none"` confirmed | none |
| `rustc` | MSRV 1.87 (D-03/D-20) | ✓ | **1.97.1** — satisfies immediately | none needed |
| `cargo` | build/test | ✓ | bundled with 1.97.1 | — |
| `git` | TRANS-02 corroboration (D-11) | ✓ | already used by `git_ops.rs` | — |
| `uuidgen` | probes only | ✓ | — | `uuid` crate at runtime |
| `jq` | manual transcript inspection | ✓ | — | not a build dependency |
| Unix process groups (`killpg`, `setpgid`) | D-14 teardown | ✓ | Linux 7.0.0 | none — Unix-only is an accepted v2.0 limitation (OQ11) |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** none.

Notably `podman` is still absent from this machine, but that is Phase 22's problem and out of scope here.

---

## Validation Architecture

### Test framework

| Property | Value |
|----------|-------|
| Framework | built-in `cargo test` (libtest). No external harness. |
| Config file | none — `[dev-dependencies] assert_fs = "1"` in `Cargo.toml` is the only test-specific config |
| Quick run command | `cargo test --lib` |
| Full suite command | `cargo build && cargo test && cargo clippy -- -D warnings` (the project gate, from CLAUDE.md) |

### Phase requirements → test map

| Req | Behaviour | Type | Automated command | File exists? |
|-----|-----------|------|-------------------|--------------|
| TRANS-01 | Every captured envelope type parses; unknown type → `Unknown`; torn line → `Unparseable` | unit | `cargo test --lib executor::stream_json` | ❌ Wave 0 |
| TRANS-01 | Injected user message serialises to the exact observed NDJSON shape | unit | `cargo test --lib executor::stream_json::user_message_wire_shape` | ❌ Wave 0 |
| TRANS-01 | `send()` writes to stdin; `interrupt()` correlates `request_id` → `control_response` | integration (fake-claude) | `cargo test --test executor_transport` | ❌ Wave 0 |
| TRANS-02 | Outcome matrix over (`subtype` × `is_error` × `terminal_reason` × exit code × disk-changed) | unit | `cargo test --lib executor::outcome` | ❌ Wave 0 |
| TRANS-02 | `success` envelope + zero disk delta → distinct no-op outcome (the "prose lies" case) | unit | `cargo test --lib executor::outcome::success_without_changes_is_noop` | ❌ Wave 0 |
| TRANS-02 | Multi-`result` transcript (fixture 05) yields one run outcome, not two | unit | `cargo test --lib executor::outcome::multi_turn` | ❌ Wave 0 |
| TRANS-03 | Keypress handled before a 10k-event executor flood | unit (tokio) | `cargo test --lib main_loop::keypress_priority` | ❌ Wave 0 |
| TRANS-03 | Burst drained in bounded batches | unit (tokio) | `cargo test --lib main_loop::bounded_batch` | ❌ Wave 0 |
| TRANS-03 | Frames render during sustained load (TestBackend) | unit (tokio) | `cargo test --lib main_loop::renders_under_load` | ❌ Wave 0 |
| TRANS-04 | Missing capability → typed error naming it; no user message written | unit | `cargo test --lib executor::gate::refuses_missing_capability` | ❌ Wave 0 |
| TRANS-04 | `apiKeySource != "none"` → `--bare` guard fires at start | unit | `cargo test --lib executor::gate::bare_regression_guard` | ❌ Wave 0 |
| TRANS-04 | Version below 2.1.214 → refused naming the observed version | unit | `cargo test --lib executor::gate::version_floor` | ❌ Wave 0 |
| D-14 | Process-group teardown kills a grandchild | integration, `#[cfg(unix)]` | `cargo test --test executor_lifecycle -- --ignored` | ❌ Wave 0 |
| D-13 | Idle cap fires on a silent child; wall cap fires on a chatty one | integration, `#[cfg(unix)]` | `cargo test --test executor_lifecycle -- --ignored` | ❌ Wave 0 |

### Sampling rate

- **Per task commit:** `cargo test --lib`
- **Per wave merge:** `cargo build && cargo test && cargo clippy -- -D warnings`, plus a `cargo clippy --all-targets` run to confirm the pre-existing lint count is still exactly 5
- **Phase gate:** full suite green, plus the OQ1 multi-step spike result recorded, before `/gsd-verify-work`

### Wave 0 gaps

- [ ] `tests/fixtures/transcripts/*.ndjson` — the seven captured transcripts, redacted per D-25
- [ ] `tests/fixtures/fake-claude.sh` (+ `slow`/`spawner`/`deaf` variants) — covers the lifecycle tests
- [ ] `tests/executor_transport.rs` — spawn/pipe/send/interrupt integration
- [ ] `tests/executor_lifecycle.rs` — teardown and deadline integration, `#[cfg(unix)]`
- [ ] `pump()` extraction from `run_tui_loop` — a prerequisite for every TRANS-03 test
- [ ] `App::new_for_test()` or equivalent constructor — `App::new` currently takes a config path and loads from disk

No framework install needed.

---

## Security Domain

`security_enforcement` is not disabled, so this section applies. Phase 15's security surface is narrow but real: it is the phase that first gives the tool the ability to execute code.

### Applicable ASVS categories

| ASVS category | Applies | Standard control |
|---------------|---------|------------------|
| V2 Authentication | indirect | Subscription OAuth is inherited ambiently. Phase 15 must not weaken it — hence D-08's `apiKeySource == "none"` guard. No credential is read, stored, or transmitted by this phase. |
| V3 Session management | yes | `--session-id` is a caller-generated UUIDv4 (`uuid` `v4`, CSPRNG-backed). Do not substitute a counter or timestamp. |
| V4 Access control | yes | D-23's `DrivableProject` capability token is the control: spawning requires a type that only a validated opt-in record can produce. Compiler-enforced, single seam. |
| V5 Input validation | yes | Every stdout line is untrusted input from a process that itself consumed untrusted repo content. Tolerant parsing (D-09) must never `unwrap()` on a parsed field; `Unparseable` must be a carried event, not a panic. Max-line-length guard per the D-05 caveat. |
| V6 Cryptography | no | Phase 15 performs no cryptographic operation. |
| V7 Error handling / logging | yes | The typed `RunOutcome` (D-12) is the control. Raw stream content must not be `tracing::info!`'d wholesale — the log file at `~/.local/share/gsd-meta-manager/` would become an unredacted transcript, which is exactly the Phase 16 SAFE-04 problem arriving a phase early. |

### Known threat patterns for this stack

| Pattern | STRIDE | Standard mitigation |
|---------|--------|---------------------|
| Argument injection into the `claude` argv (a project alias or path containing `--flag`) | Tampering | `Command::args()` with a `Vec<OsString>` — never a shell string, never `sh -c`. `process-wrap` wraps `tokio::process::Command`, which does not invoke a shell. Validate that the project path is an existing directory before use. |
| A project-local `.mcp.json` in a registered third-party repo introducing tools | Elevation of privilege | `--strict-mcp-config` (D-15). Verified accepted by 2.1.220 in probe arm 2. |
| Unattended run hitting an interactive gate and hanging | Denial of service | `--permission-mode dontAsk` (D-15) converts the hang into a fast, classifiable failure; the D-13 idle cap is the backstop. |
| Orphaned process tree holding ports/files after a kill | Denial of service | Process-group SIGTERM → SIGKILL → `wait()` (D-14). Verified API in `process-wrap`. |
| Credential leakage into a committed test fixture | Information disclosure | D-25 manual redaction; this session's scan found no credential-shaped strings, but absolute home paths are present and must be normalised. |
| Log file accumulating unredacted agent output | Information disclosure | Log event *types* and counts at `info`, raw lines only at `trace`, and never enable `trace` by default. |
| A future CLI defaulting to `--bare`, silently dropping OAuth | Spoofing / DoS | The D-08 mechanical guard, asserted at first `system/init`. |
| Prompt injection via `.planning/` content | Tampering | **Out of scope for Phase 15** — Phase 21 (SAFE-07/08). Phase 15 must not, however, build anything that makes it harder: keep the command a fixed string from the caller, never a model-authored shell string. |

---

## Assumptions Log

| # | Claim | Section | Risk if wrong |
|---|-------|---------|---------------|
| A1 | `--setting-sources project` propagates to nested `claude` subagent processes spawned by a multi-step skill | OQ1 remaining work | If it does not, the hang returns at subagent depth and OQ1 fails at the scale that matters. **This is precisely what the plan's gating Task 1 must test.** |
| A2 | An injected message is persisted to session history and survives `--resume` | OQ2 (b) | Untested and deliberately so — moot for Phase 15/18 because the message executes in-process. Only matters if a later phase resumes a session after injection. |
| A3 | The `result`-per-turn behaviour holds for turns that use tools, not just text-only turns | Contradictions #1 | Probe 05 injected into a text-only essay turn. If tool-using turns behave differently, the outcome derivation needs revisiting. Fixture 03 (tool use) is single-turn so it does not settle this. Cheap to confirm during Task 1. |
| A4 | `capabilities[]` remains exactly the observed three on the CLI versions users will have | D-06 gate | The gate is written as a subset check, so *extra* capabilities are safe. A *renamed* capability would refuse a working CLI — which is the intended fail-closed direction, but will generate support noise. |
| A5 | `async-trait` is avoidable via boxed futures on a single implementor | Executor trait revival | If the planner finds the object-safety gymnastics worse than the dependency, adding `async-trait` is a small, defensible Cargo change — but it is a Cargo change that a "revive the trait" task would not obviously imply. |
| A6 | `EXEC_BATCH = 64` and a 16ms redraw interval are reasonable starting values | Pattern 3 | Explicitly per D-13's "pick defensible starting numbers and make them configurable". No tuning data exists; real numbers are a v2.1 concern. |
| A7 | The five pre-existing `--all-targets` clippy lints do not interact with the new modules | Project constraints | Verified only by reading the constraint, not by running `--all-targets` this session (the repo has an autonomous run in flight and was deliberately not built against). Plan should verify the count early. |

---

## Open Questions

1. **Does `result`-per-turn hold when the interrupted/queued turn uses tools?** (A3)
   - Known: confirmed for a text-only turn queued behind a text-only turn.
   - Unclear: whether a tool-using turn emits its own `system/init` + `result` identically.
   - Recommendation: fold a two-message tool-using probe into Task 1's spike — near-zero marginal cost since the spike is already spawning a real run.

2. **Should the driven child's environment be scrubbed of inherited `CLAUDE_*` variables?**
   - Known: the child inherits the parent env; this repo is plausibly developed from inside a Claude Code session.
   - Unclear: which inherited variables actually change `-p` behaviour.
   - Recommendation: scrub `CLAUDE_*` except the ones deliberately set (`CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS`), and record the decision. Cheap insurance against a class of "works on my machine" bug.

3. **Where does `RunOutcome` live — `src/error.rs` or `src/executor/`?** (Explicit Claude's-Discretion item in D-12.)
   - Recommendation: `src/executor/mod.rs`. `RunOutcome` is a *domain* type with success variants, not an error type; putting it in `error.rs` misfiles it. Keep `error.rs` for the genuinely error-shaped `SpawnError` / `SendError` / `CapabilityError`, which gives that placeholder its first real content without conflating the two.

4. **Bounded vs unbounded executor channel.**
   - Known: the existing `Action` channel is unbounded; the `pending_editor` shell-out can block the loop for minutes.
   - Recommendation: **bounded** (`mpsc::channel(N)`) for the executor path specifically, so a blocked render loop applies backpressure to the reader task rather than growing memory without limit. Note the interaction: backpressure on the reader risks Claude's 30s exit-drain cap, so N should be generous (thousands) and the reader should log rather than block silently when full.

---

## Sources

### Primary (HIGH confidence)

- **Live execution of `claude` 2.1.220** in a throwaway scratch directory, this session — seven runs producing the transcripts listed above. Source of every protocol claim: `system/init` shape and `capabilities[]`, the four `result` envelope variants, `isReplay` echo semantics and timing, per-turn `result`/`init` emission, `control_request`/`control_response` wire shape and `still_queued`, bare-`interrupt` refutation, `--max-budget-usd` under subscription, the hook-hang reproduction and its `--setting-sources project` mitigation, `hook_started`/`hook_response`/`thinking_tokens`/`rate_limit_event` shapes, exit codes 0/1/124.
- **`process-wrap` 9.1.0 published crate source**, downloaded and read: `Cargo.toml` (edition 2024, `rust-version = "1.87.0"`, `tokio1` not default), `src/generic_wrap.rs` (`with_new`/`wrap`/`spawn`), `src/tokio/core.rs` (`ChildWrapper` full method set), `src/tokio/process_group.rs` (`start_kill` = SIGKILL, `signal` = `killpg`, `wait` multi-pass reap, pgid = child pid).
- **Compiled and executed serde model** in a scratch crate — validated against six real envelopes plus a synthetic future message type and a torn line; passed `cargo clippy --all-targets -- -D warnings`.
- **Direct repository inspection:** `src/main.rs` (`run_tui_loop` :134-205, `rx.recv().await` :151, `pending_editor` :156-197), `src/event.rs` (whole file), `src/action.rs` (whole file), `src/error.rs` (3 lines), `src/app.rs` (`App` :110-116, `update` :235+, session poll :246-257), `src/ui/screens/mod.rs` (`AppContext` :111-132), `src/state_reader/mod.rs` (`ProjectState` :14-50 with `PartialEq`, `parse_project_state` :103), `src/state_reader/git_ops.rs` (full symbol list), `src/lib.rs`, `Cargo.toml`, `tests/` layout, `TestBackend` precedent at `src/ui/screens/normal.rs:870-878`.
- **crates.io API** for `process-wrap`, `uuid`, `tokio-util` — versions, update dates, download counts, repository URLs.

### Secondary (MEDIUM confidence)

- `.planning/research/{SUMMARY,STACK,ARCHITECTURE,PITFALLS}.md` — the committed research pass. Treated as given except where this session's probes contradict it (see Contradictions).
- `.planning/milestones/v1.2-phases/13-queue-execution-research/QUEUE-EXECUTION-DESIGN.md` §11 — the original `Executor` trait, read verbatim.
- `.planning/phases/15-transport-foundation/15-CONTEXT.md`, `.planning/ROADMAP.md` Phase 15 — scope and locked decisions.

### Tertiary (LOW confidence — flagged)

- docs.rs `process-wrap` 9.1.0 module page — consulted first, found to lack method signatures; **superseded** by reading the crate source. Not relied on.
- Community issues #41230 (mid-turn drop) and #41665 (bare interrupt) — cited in the research pass. #41230's claim is **refuted** for 2.1.220 by probe 05; #41665's is **corroborated** by probe 06. Neither was re-read this session; both are now settled empirically.

---

## Metadata

**Confidence breakdown:**

- **Spike outcomes (OQ1/OQ2/OQ3):** HIGH — direct execution with captured transcripts, and every claim reproducible from the staged files.
- **Standard stack:** HIGH — crate source read locally, registry data verified live.
- **`process-wrap` API:** HIGH — read from published source, not prose.
- **serde model:** HIGH — compiled, executed, clippy-clean against real inputs.
- **Event-loop restructure:** MEDIUM-HIGH — source read directly and the shape is standard tokio; the specific `select!` arm-disabling hazard is a known sharp edge, not something verified in this codebase.
- **Testing mechanics:** MEDIUM — conventions read directly from the repo; the fake-`claude` recommendation is a design judgement, not a verified result.
- **TRANS-03 verification strategy:** MEDIUM — the deterministic properties are sound, but no test was written this session.

**Research date:** 2026-07-29
**Valid until:** ~2026-08-12 (14 days). The Claude CLI ships behaviour changes at patch granularity — this session alone found three `result` field values and one `system` subtype that a research pass one day earlier had missed. Re-verify the `system/init` shape and `capabilities[]` if the local CLI advances past 2.1.220 before this phase executes.
