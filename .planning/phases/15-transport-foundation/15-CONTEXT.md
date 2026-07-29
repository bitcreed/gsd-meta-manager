# Phase 15: Transport Foundation — Duplex stream-json Executor - Context

**Gathered:** 2026-07-29
**Status:** Ready for planning
**Mode:** Autonomous — grey areas proposed and auto-accepted by the orchestrator per user
direction ("best well-reasoned guess is good enough; don't ask questions"). Every decision
below is Claude's Discretion and is listed explicitly so it can be corrected at the end.
Decisions marked **[LOCKED-BY-RESEARCH]** were settled by the committed research pass
(`.planning/research/{SUMMARY,STACK,ARCHITECTURE,PITFALLS}.md`) and are carried in as
given — do not relitigate them during planning.

<domain>
## Phase Boundary

The tool can run a GSD command through `claude -p` over a structured two-way protocol and
know exactly how it ended.

In scope: TRANS-01 (duplex `stream-json` over stdio, structured envelope not text scraping),
TRANS-02 (outcome derived from the `result` envelope + exit code + disk state + git, never
prose), TRANS-03 (TUI event loop stays responsive while a run streams), TRANS-04 (runtime
capability detection via `system/init`, refuse unsupported CLIs up front).

Also in scope because research binds them here and nowhere earlier:

- The `run_tui_loop` restructure to `tokio::select!` — a **prerequisite**, not a cleanup
  (PITFALLS Pitfall 6; SUMMARY "Critical Pitfalls" #5).
- Process-group spawn/teardown at the executor level (`process-wrap`), because a bare
  `tokio::process` kill orphans `claude`'s Bash grandchildren.
- The MSRV rise 1.85 → 1.87 and its documentation surface.
- The CLI version gate + `--bare` regression guard (PITFALLS Pitfall 12).
- A typed run-outcome error surface (PITFALLS Pitfall 10 — "widen the error type *before*
  the driver").
- The single-spawn-seam capability token, type only (PITFALLS ordering constraint #3).
- Three MUST-SPIKE questions (OQ1/OQ2/OQ3) that gate the rest of the milestone.

Out of scope — each has a named later owner:

- The run journal, `run.json`, path classification in `watcher`/`app`, redact-at-capture
  → Phase 16.
- Detached spawn, PID liveness, crash reconciliation, the user-facing kill switch, dry-run,
  `flock`, the `driver_opt_in` config record and its UI → Phase 17.
- The Driver tab, live render, injection UI and its three-state delivery display → Phase 18.
- Git blast-radius envelope, `--disallowedTools` push denial, secret scanning → Phase 19.
- `decide()`, run bounds, quota-floor park, cost policy → Phase 20.
- LLM goal decomposition, prompt-injection hardening → Phase 21.
- `ExecutionTarget::Container`, `PathMap`, bollard, podman → Phase 22.

</domain>

<decisions>
## Implementation Decisions

### Transport shape

- **D-01:** **[LOCKED-BY-RESEARCH]** The spawn argv baseline is exactly
  `claude -p --input-format stream-json --output-format stream-json --verbose
  --replay-user-messages --session-id <uuid> --setting-sources project`. `--verbose` is
  required or `stream-json` emits nothing. `--setting-sources project` (omitting `user`) is
  the mitigation for the reproduced 180-240s hang — the user's global `PreToolUse` hooks are
  registered with no `timeout`, run synchronously on the agent's critical path, and have no
  TTY under `-p`. Keep a config toggle to re-enable `user` sources, defaulting off.

- **D-02:** The GSD command is delivered as a **user message written to stdin**, not as a
  positional prompt argument. Under `--input-format stream-json` this is the only channel,
  and it means the first message and every later injection travel one code path. The
  process is spawned first, then the prompt is written — which is what makes D-06's
  "refuse up front" literally true.

- **D-03:** **[LOCKED-BY-RESEARCH]** Spawn through `process-wrap` 9.1.0 (`tokio1` feature,
  which is **not** default) wrapped with `ProcessGroup::leader()`. Not bare
  `tokio::process` — `claude` spawns Bash grandchildren that `Child::kill()` would orphan.
  This raises the project MSRV 1.85 → 1.87; that is an accepted, real project-level change,
  not an incidental dependency detail (see D-20).

- **D-04:** One `tokio::spawn` reader task **per pipe** — stdout NDJSON and stderr raw, kept
  separable for diagnostics. Never merge them (this is why `tokio-process-stream` is
  rejected). Parsing happens in the reader task; parsed events are forwarded over `mpsc`.
  Never parse on the render thread. stdin writes are driven from a **separate** task from
  the one awaiting exit — the classic two-pipe deadlock is live here — and dropping the
  stdin handle is the EOF signal.

- **D-05:** Line framing: either `tokio_util::codec::FramedRead<_, LinesCodec>` or
  `tokio::io::BufReader::lines()`. STACK explicitly says either is acceptable and warns
  against adding `tokio-util` *solely* for framing. Since `CancellationToken` is also wanted
  for run cancellation, adding `tokio-util = { version = "0.7", features = ["codec"] }`
  is justified — but if the planner finds cancellation is cleanly expressible without it,
  `BufReader::lines()` alone is the preferred, dependency-free choice. Planner's call;
  record which and why.

### Capability gating and version safety

- **D-06:** **[LOCKED-BY-RESEARCH]** Feature-detect via the `capabilities[]` array on the
  `system/init` event — **never** by comparing version strings. Mechanism for "refused up
  front" (TRANS-04 success criterion #4): spawn, read events until `system/init`, validate
  the required capability set, and **withhold the first user message until validation
  passes**. On failure, tear the process group down and return a typed error naming the
  missing capability. No turn is ever started, so the refusal costs zero tokens and zero
  quota. Observed on 2.1.220: `interrupt_receipt_v1`, `interrupt_cancel_queued_v1`,
  `msg_lifecycle_v1`.

- **D-07:** Minimum supported Claude CLI is **2.1.214**. Rationale: below 2.1.208-2.1.214 a
  large piped response could truncate the final line and omit the `result` message — which
  breaks TRANS-02 outright, since the `result` envelope is the authoritative outcome. Refuse
  below the minimum naming the observed version; **warn but proceed** above the tested
  maximum (2.1.220, verified locally). Record `claude_code_version` from `system/init` in
  the run's in-memory state so Phase 16 can journal it without a signature change.

- **D-08:** **[LOCKED-BY-RESEARCH]** Never pass `--bare` — it skips OAuth/keychain reads and
  requires `ANTHROPIC_API_KEY` or an `apiKeyHelper`, breaking the subscription-auth
  constraint. Because Anthropic states `--bare` "will become the default for `-p` in a
  future release", ship a **mechanical regression guard**, not a comment: assert at
  `system/init` that `apiKeySource == "none"` (the subscription/OAuth path is alive). If a
  future CLI silently defaults to bare, that assertion fails loudly at run start instead of
  producing a mysteriously context-free agent mid-run.

- **D-09:** **[LOCKED-BY-RESEARCH]** Parse `stream-json` **tolerantly**. Never
  `deny_unknown_fields`. Model the envelope as `#[serde(tag = "type")]` with a catch-all
  variant that preserves the raw line, and the same for unknown `subtype`s. An unknown
  message type is a forward-compat event to be carried, not an error that fails a run.

### Outcome derivation

- **D-10:** **[LOCKED-BY-RESEARCH]** Outcome comes from four sources and never from the
  agent's prose: the `type:"result"` envelope (`subtype`, `is_error`, `terminal_reason`,
  `permission_denials[]`), the process exit code, disk state, and git. Observed `subtype`
  values: `success`, `error_max_turns`, `error_during_execution`. Observed
  `terminal_reason`: `completed`, `aborted_tools`. The exit code is a **liveness/crash
  signal only** — empirically the process hung 180-240s after the model finished and only
  emitted `result` on SIGTERM, with exit status coming from the external `timeout`, never
  from Claude.

- **D-11:** The disk/git half of TRANS-02 must actually be implemented in this phase, not
  stubbed. Concretely: snapshot git `HEAD` sha + dirty status and a `.planning/` artifact
  fingerprint (reuse the existing `parse_project_state` / `DiskInference` readers — do not
  write a parallel one) **before and after** the run, and report a `made_changes` signal
  alongside the envelope verdict. An envelope that says `success` while nothing changed on
  disk is reported as a distinct no-op outcome. This corroboration is precisely what makes
  success criterion #2 ("correct even when the agent's prose says otherwise") a testable
  claim rather than an assertion.

- **D-12:** **[LOCKED-BY-RESEARCH]** Introduce a typed `RunOutcome` now, not
  `Result<(), anyhow::Error>`. PITFALLS Pitfall 10 assigns "widen the error type before the
  driver" to this phase explicitly, because retrofitting later means the TUI can only show a
  message where it needs a state. Minimum variant set, derived from what the four sources
  can actually distinguish: succeeded (with/without changes), failed with a classified
  reason, permission-denied (carrying `permission_denials[]`), killed, timed-out
  (wall-clock), stalled (idle), capability-refused, spawn-failed. `src/error.rs` is
  currently a 178-byte placeholder comment — this is its first real content, or the types
  live in `src/executor/` and `error.rs` stays a stub; planner's call.

### Process lifecycle and deadlines

- **D-13:** **[LOCKED-BY-RESEARCH]** Never `child.wait().await` unbounded. Enforce **two
  independent deadlines**: a total wall-clock cap, and an **idle cap measured from the last
  stream event**. The idle cap is the correct stuck-detector because a long legitimate
  `/gsd:execute-phase` emits events continuously while a hook-hung run emits nothing —
  time-since-spawn cannot tell those apart.

- **D-14:** **[LOCKED-BY-RESEARCH]** Teardown sequence: SIGTERM to the **process group** →
  grace period (10s) → SIGKILL to the group → unconditional `wait()`. SIGTERM is the
  documented clean path (aborts the turn, tears down the Bash tree, runs `SessionEnd` hooks,
  exits 143); SIGKILL is the backstop; the `wait()` is what prevents zombies. Also set
  `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` explicitly rather than inheriting the silent
  10-minute default for background subagents. Phase 15 owns this at the *executor* level;
  Phase 17 owns the user-facing kill switch and detached-process reaping built on top.

- **D-15:** Driver-launched sessions use `--permission-mode dontAsk`, **not**
  `bypassPermissions` and never `--dangerously-skip-permissions` on the host. `dontAsk`
  denies anything outside `permissions.allow` and the read-only set, and denies
  `AskUserQuestion` even when an allow rule matches — converting "unattended run hangs on an
  interactive gate" from a hang into a fast, detectable failure the driver can park on. Also
  pass `--strict-mcp-config` so a project-local `.mcp.json` in a registered third-party repo
  cannot introduce tools (PITFALLS Pitfall 8 assigns this to TRANSPORT). The
  `--disallowedTools` git denylist is Phase 19's, not this phase's.

- **D-16:** `--max-budget-usd` is **plumbed as an option and nothing more**. REQUIREMENTS
  places it out of scope as a cost control; it is retained only as an anomaly
  circuit-breaker. Whatever OQ3 returns, Phase 15 builds no cost enforcement on it. Same for
  `--max-turns`: it works but is absent from `--help` on 2.1.220 and its counter resets when
  `--input-format stream-json` queues messages — treat it as a secondary belt at most, never
  a load-bearing bound.

### Event loop restructure (TRANS-03)

- **D-17:** Restructure `run_tui_loop` to `tokio::select!` as a **first-class, explicitly
  planned task**, not a side effect of another task. The two research documents appear to
  disagree here and the disagreement is worth resolving in writing, because a planner reading
  only ARCHITECTURE §1.1 would skip this:

  ARCHITECTURE is right that `EventBus` already multiplexes *upstream* (crossterm, tick, and
  watcher tasks all clone one `tx`), so adding a fourth source is mechanically a pure
  addition. PITFALLS is right that the restructure is still mandatory — for three reasons
  that are about **priority and progress**, not about multiplexing:

  1. A single unbounded FIFO gives control keys **no priority** over bulk stream traffic. A
     keypress queued behind several thousand `stream_event` lines waits for all of them.
     That is exactly the failure TRANS-03 forbids.
  2. The loop can only make progress when a message arrives; there is no timer arm of its
     own. It is currently rescued only by the incidental 250ms `spawn_tick`.
  3. The `pending_editor` path calls `ratatui::restore()` and then a **blocking**
     `std::process::Command::status()` on the render thread. Anything bolted onto that shape
     is unreadable while it runs.

  Required shape: `tokio::select!` with `biased;` ordering — input first, then the executor
  event channel, then tick/redraw — and executor events drained in **bounded batches per
  iteration** so a burst cannot starve a redraw. The high-volume executor channel is
  separate from the existing `Action` channel; do not funnel per-token stream output through
  the `Action` FIFO.

- **D-18:** Do **not** refactor the `pending_editor` blocking shell-out beyond what the
  `select!` restructure strictly requires. It is a known wart, it is not on this phase's
  critical path, and widening the blast radius of an event-loop change is exactly how a
  foundation phase turns into a regression hunt. Preserve its observable behavior; note the
  wart for a later quick task.

- **D-19:** Do not add live driver state to `ProjectState`. It derives `PartialEq`, and
  `app.rs` uses that equality to suppress "Updated: {alias}" status spam — a deliberate v1.4
  feature. Driver state changes every few seconds and would flood the status bar for an
  entire multi-hour run. Any state this phase needs to surface lives in a sibling map on
  `AppContext`. (Phase 16 owns the full version of this; Phase 15 must simply not violate it
  first.) Likewise, no process handle may ride inside an `Action` — `Action` derives `Clone`
  and `Child`/`ChildStdin`/`JoinHandle` are not `Clone`.

### MSRV and project surface

- **D-20:** The MSRV rise is a real deliverable with a documentation surface, not a
  `Cargo.toml` one-liner. `Cargo.toml` currently has **no** `rust-version` key at all —
  add `rust-version = "1.87"` (which also makes the floor machine-checkable for the first
  time) and update every place that states 1.85: `README.md` (2 sites), `CLAUDE.md`
  (stack table), `CONTRIBUTING.md`, `docs/TESTING.md`, `docs/GETTING-STARTED.md` (3 sites,
  including the "requires rustc 1.85 or newer" troubleshooting entry), `docs/DEVELOPMENT.md`.
  Local toolchain is 1.97.1, so the bump is satisfiable immediately.

### Module layout and the spawn seam

- **D-21:** New modules, following ARCHITECTURE's component table: `src/executor/mod.rs`
  (the `Executor` trait, `ExecutionOptions`, `ExecutionHandle`, `ExecutionEvent`,
  `RunOutcome`), `src/executor/claude.rs` (`ClaudeExecutor` — argv construction, spawn,
  pipe tasks), `src/executor/stream_json.rs` (serde types for the NDJSON protocol).
  Introduce `ExecutionTarget` with **only** the `Host` variant now, so Phase 22 adds a
  variant rather than changing a signature — a one-line cost today against a
  touch-every-call-site cost later.

- **D-22:** Revive the v1.2 `Executor` trait with the amendments research specifies:
  `start` / `cancel` / `is_running` **plus** `send()`, `interrupt()`, and `capabilities()`.
  Phase 15 implements all of them at the **transport** level — `send()` writes one NDJSON
  user message to stdin, `interrupt()` writes
  `{"type":"control_request","request_id":"…","request":{"subtype":"interrupt"}}` and awaits
  the matching `control_response`. The *policy* question (buffer interjections to the
  `result` turn boundary vs. steer mid-turn) is Phase 18's and depends on the OQ2 spike
  result — Phase 15's job is to make the mechanism exist and to **record the OQ2 finding in
  the phase SUMMARY** so Phase 18 does not re-derive it. Be honest in the trait docs that
  interjection is a Claude capability tier, not backend parity; that is what `capabilities()`
  is for.

- **D-23:** Enforce the single-spawn-seam now, as a **type**. `ClaudeExecutor`'s spawn entry
  point takes a capability token (`DrivableProject` or equivalent) rather than a bare path or
  a `bool`. Phase 15 defines the type and may construct it from an explicit/test-only
  constructor; Phase 17 supplies the validated `driver_opt_in` record as the only production
  constructor, plus the config schema, the UI, and the re-check-before-every-`Run`. Rationale
  for splitting it this way: adding the parameter now costs one signature; retrofitting it in
  Phase 17 means auditing every call site added between now and then — which is the precise
  failure PITFALLS Pitfall 10 names. The boundary is deliberate and narrow: Phase 15 owns the
  *seam*, Phase 17 owns the *record*.

### Testing

- **D-24:** The executor must be unit-testable **without spawning `claude`**. Build a fake
  executable fixture — a small script or test binary the executor can be pointed at — that
  replays a canned NDJSON transcript, and capture **real transcripts during the OQ1/OQ2
  spikes** to check in as golden fixtures. This puts the entire stream-json parsing layer,
  the capability gate, the `--bare` guard, the tolerant-parsing behavior, and the whole
  outcome-derivation matrix under CI with zero subscription, network, or quota dependency.
  It also gives the `--bare`-default regression guard something to assert against.

- **D-25:** Any captured transcript checked in as a fixture must be **manually reviewed and
  redacted at capture time**. SAFE-04 (redact-at-capture) formally lands in Phase 16, and
  PITFALLS is explicit that every log written before a redaction retrofit stays unredacted
  forever. A committed fixture is exactly such a log.

- **D-26:** Outcome derivation gets a test per distinguishable combination of
  (`subtype` × `is_error` × `terminal_reason` × exit code × disk-changed), not one happy-path
  test. This is the phase's highest-value test surface — TRANS-02 is a correctness claim
  about disagreement between sources, and disagreement is only observable in a matrix.

### Spike ordering

- **D-27:** **The OQ1 spike is task 1 and gates everything else in the phase.** Not a
  parallel investigation, not a verification step at the end. The entire milestone rests on
  whether `claude -p` reliably runs a multi-step GSD skill headlessly; research reproduced a
  180-240s hang 3/3 on any tool-using prompt. If OQ1 fails, the plan must **say so loudly and
  stop**, rather than building the executor on sand. A failed OQ1 is a legitimate, reportable
  outcome — the orchestrator surfaces it to the user, and the milestone's premise gets
  revisited rather than quietly worked around.

- **D-28:** The spike runs against a **disposable scratch GSD project**, created for the
  purpose outside the user's registered project set. It must **not** target
  `gsd-meta-manager` itself (an autonomous milestone run is in flight against this very
  repo — a spike driving it would race the orchestrator), and must not target any real
  registered project. Never `--dangerously-skip-permissions` during the spike.

### Amendments from Phase 15 research (added 2026-07-29, after `15-RESEARCH.md`)

The research spike resolved OQ1/OQ2/OQ3 empirically and, in doing so, proved that several
*supporting facts* behind the decisions above were incomplete. The decisions themselves
stand; these four amendments correct their factual annexes and are trackable in their own
right because the plans must handle each one explicitly. Full evidence, with verbatim
transcripts, is in `15-RESEARCH.md` §"Contradictions with Committed Research and CONTEXT.md".

- **D-29:** **`type:"result"` is a TURN boundary, not a RUN terminator.** Directly observed:
  a run that receives a second stdin message emits two `system/init` events and two `result`
  envelopes in one process. D-10 and D-12 are written as though there is exactly one
  `result` per run — that reading is wrong and would truncate every steered run while
  reporting "succeeded". Correct model: **one process, N turns.** Each `result` closes a turn
  and appends to a per-turn outcome list; the **run** terminates only on stdin EOF → process
  exit. Run-level `RunOutcome` derives from the **last** `result` + exit code + disk/git
  delta. `total_cost_usd` is cumulative across turns; `num_turns` and `duration_ms` are
  per-turn and reset, so a run-level turn count must be summed. Closing stdin does not kill
  the run — Claude drains queued turns, finishes, and exits 0.

- **D-30:** **The capability gate fires on the FIRST `system/init` only.** D-06's mechanism
  (spawn → read to `system/init` → validate → only then release the first user message) is
  confirmed working, but every subsequent queued turn emits its own `init`. Later inits are
  informational: they must not re-run the gate, must not re-arm the D-08 `--bare` guard as a
  run-abort, and must not be treated as a protocol error.

- **D-31:** **No turn-boundary flush buffer for injection.** ARCHITECTURE's AP3 ("a mid-turn
  message is dropped and lost from history") is REFUTED on 2.1.220 — the message is
  **queued and executed as its own turn**. The CLI already does the buffering AP3 prescribes,
  so a driver-side buffer would duplicate it and make the `still_queued` accounting harder to
  reason about. `send()` writes directly. Two hazards to carry: (a) the
  `--replay-user-messages` echo (`isReplay: true`, camelCase, **absent** rather than `false`
  on non-replay messages, so `#[serde(default)]`) is emitted at **dequeue**, not at receipt —
  it is a "started processing" ack, not a "received" ack, which is exactly the distinction
  Phase 18's queued→delivered→acted-on UI needs; (b) `control_response{subtype:"success"}`
  means "the request was accepted", NOT "the thing you meant was cancelled" — correlate on
  `request_id` and treat `response.response.still_queued` (note the double nesting) as
  authoritative. Bare `{"type":"interrupt"}` does nothing and must not be used.

- **D-32:** **The outcome matrix in D-26 must cover the terminal states this phase can
  actually reach**, which are more than D-10 listed. Newly observed on 2.1.220:
  `subtype: "error_max_budget_usd"` with `terminal_reason: "budget_exhausted"` (exit 1), and
  `terminal_reason: "aborted_streaming"` from a `control_request` interrupt (exit 1) — both
  reachable from features this phase ships. Also: the `result` field is **absent** on error
  envelopes, so it is `Option<String>`, and `api_error_status` is likewise success-only.
  Undocumented high-frequency event `system/thinking_tokens` was observed 22 times in a
  single 68-second turn — direct evidence for D-09's tolerant parsing and for D-17's
  bounded-batch drain, and it must not be treated as an unknown-type error.

### Claude's Discretion

- Plan granularity and wave structure — but OQ1 must be the first task of the first plan
  and every other plan must be gated on its result (D-27).
- Whether `RunOutcome` and friends live in `src/error.rs` or `src/executor/mod.rs`.
- Whether `tokio-util` is added or `BufReader::lines()` suffices (D-05).
- Exact test names and file placement, following the existing `#[cfg(test)] mod tests`
  convention.
- The concrete default values for the wall-clock and idle caps (D-13) — pick defensible
  starting numbers and make them configurable; real tuning data does not exist yet and is
  explicitly a v2.1 concern.
- Whether the fake-`claude` fixture is a shell script, a `#[cfg(test)]` binary, or a
  `tests/bin/` helper.

</decisions>

<code_context>
## Existing Code Insights

Every anchor below was read directly; line numbers are from the research pass and were
spot-checked against the current tree.

**The event loop (TRANS-03's target)**
- `src/main.rs:134-205` — `run_tui_loop`. A single `rx.recv().await` at `:151`, no
  `tokio::select!`. The `pending_editor` arm inside it calls `ratatui::restore()` then a
  blocking `std::process::Command::status()` on the render thread, then `ratatui::init()`.
- `src/event.rs` — `EventBus` holds one `mpsc::unbounded_channel`; `spawn_crossterm_reader`
  and `spawn_tick(250)` each clone `tx`. `FileWatcher` (`src/watcher.rs:40-71`) is the third
  producer. Adding a source is mechanically additive; see D-17 for why that is not the whole
  story.

**The Action contract**
- `src/action.rs` — `#[derive(Debug, Clone)] pub enum Action` with 11 variants.
  `ProjectStateLoaded` boxes its payload with the comment *"Boxed: ProjectState dwarfs every
  other variant (clippy::large_enum_variant)"*. Two constraints follow: no non-`Clone`
  process handle can ride in an `Action`, and any comparably large new variant must be boxed
  or clippy fails the build.
- `FileChanged` currently carries only `project_path` — it has no way to tell driver writes
  from GSD writes. Adding `changed_path` is **Phase 16's** change, not this phase's.

**State reading (D-11's corroboration source)**
- `src/state_reader/mod.rs:103-240` — `parse_project_state`, a synchronous full re-parse;
  idempotent, never panics. Reuse it for the disk-state fingerprint rather than writing a
  second reader.
- `src/state_reader/disk_status.rs` — `DiskInference` with 22 artifact booleans.
- `derive_all_stage_statuses` is still private inside `src/ui/screens/detail.rs` (~:3281-3328).
  Lifting it into `state_reader/` is **Phase 20's** job (the router needs it); Phase 15 does
  not need it and should not pre-emptively move it.

**Process and I/O idioms already in the codebase**
- `spawn_blocking` for sync fs/process work, result returned as an `Action` on a cloned `tx`
  (`src/app.rs` session detection ~:252-256, project re-parse ~:289-295).
- `src/session_detector.rs` — `pgrep -x claude` + `/proc/<pid>/{cwd,cmdline,stat,fd/0}`,
  Linux-only by construction, and documents the discipline: *"Uses std::process::Command
  (not tokio) — called from spawn_blocking."* Phase 17 reuses this for PID liveness; Phase 15
  does not need it.
- `src/config.rs` — atomic write via tempfile+persist. The idiom Phase 16's `run.json` will
  mirror.
- `src/cli.rs` — `Commands` enum with `Add`/`Remove`/`List`. The `Drive` subcommand is
  **Phase 17's** addition.
- `src/error.rs` — 178 bytes, a placeholder comment only ("reserved for future custom error
  types"). D-12 is its first real customer, if the planner puts the types there.

**Dependencies and toolchain**
- `Cargo.toml` — no `rust-version` key today. Existing deps include `tokio = { version = "1",
  features = ["full"] }` (so `process` is already available), `serde_json`, `chrono`,
  `tempfile`, `futures`. New in this phase: `process-wrap = { version = "9.1.0",
  features = ["tokio1"] }` (the `tokio1` frontend is **not** a default feature),
  `uuid = { version = "1.24", features = ["v4", "serde"] }`, and possibly
  `tokio-util = { version = "0.7", features = ["codec"] }` per D-05. `bollard` and `fs4` are
  **not** this phase's — they belong to Phases 22 and 17.
- Local toolchain: rustc 1.97.1. Local Claude CLI: 2.1.220.

**Project gates**
- `cargo build && cargo test && cargo clippy -- -D warnings` (lib target) must pass clean.
- `cargo clippy --all-targets -- -D warnings` currently fails with **exactly 5 pre-existing
  lints** — 3× `assert_eq!` with a literal bool (`browser.rs`), 1× owned-instance-for-
  comparison (`project_creator.rs`), 1× items-after-test-module (`state_reader/mod.rs`).
  Those are **not** this phase's to fix, and the count must not grow.

**MSRV documentation surface (D-20)** — `README.md:61`, `README.md:186`, `CLAUDE.md:25`,
`CONTRIBUTING.md:23`, `docs/TESTING.md:14`, `docs/GETTING-STARTED.md:13`,
`docs/GETTING-STARTED.md:150`, `docs/DEVELOPMENT.md:10`.

</code_context>

<specifics>
## Specific Ideas

### The three MUST-SPIKE questions

These are carried verbatim from ROADMAP's Phase 15 risk block and SUMMARY's blocking open
questions. All three must resolve before the phase closes; **OQ1 gates the phase** (D-27).

**OQ1 — does `claude -p` reliably execute a multi-step GSD skill headlessly?**
The whole milestone rests on it, and it has been open since the v1.2 design. Research
reproduced a hang **3/3** on *any* tool-using prompt:

| Run | `duration_api_ms` | `duration_ms` | `subtype` | `terminal_reason` |
|-----|------------------|--------------|-----------|-------------------|
| tool + `--max-turns 1`, 240s cap | 2117 | **239008** | `error_max_turns` | `aborted_tools` |
| tool + `--max-turns 1`, 180s cap | 2414 | **179043** | `error_max_turns` | `aborted_tools` |
| tool, no turn limit, 180s cap | 2716 | **179017** | `error_during_execution` | `aborted_tools` |

`duration_ms` equals the wall-clock cap every time while `duration_api_ms` is ~2s — the model
finished in seconds and the process then hung until an external SIGTERM arrived, at which
point it emitted `result`. A text-only prompt exited cleanly in 2.3s with code 0. Root cause:
`~/.claude/settings.json` registers `PreToolUse` hooks (`claudear.hooks.permission`,
`rtk-rewrite.sh`) with **no `timeout` field**; synchronous hooks block the agent's critical
path and have no controlling terminal under `-p`. `--setting-sources project` fixed the
synthetic case. **The spike must confirm this generalises to a real multi-step GSD skill
invocation** (something that itself spawns subagent waves, e.g. `/gsd:execute-phase`), not
just a single synthetic tool call. Run it against a disposable scratch project (D-28).

**OQ2 — mid-turn stdin injection + `control_request` interrupt semantics on 2.1.220.**
Currently MEDIUM confidence from community sources only; `--input-format stream-json` is
officially undocumented (anthropics/claude-code#24594). Two claims to test: (a) a user
message written while a turn is in flight is dropped by that turn *and* is not persisted to
session history, so it also vanishes on `--resume` (#41230); (b)
`{"type":"control_request","request_id":"req_1","request":{"subtype":"interrupt"}}` →
`{"type":"control_response","response":{"subtype":"success","request_id":"req_1"}}` works as
described, and a plain `{"type":"interrupt"}` does not (#41665). Note the countervailing
claim from the CLI reference: on v2.1.205+, a message sent while Claude is working stays
**queued** and runs as its own turn. The spike settles which is true on the installed
version. Also confirm `--replay-user-messages` echoes the injected message, since that echo
is the only delivery ack the design has. The result determines whether Phase 18 can steer
mid-turn or must buffer to the `result` boundary — **record the finding in the phase SUMMARY**.

**OQ3 — does `--max-budget-usd` apply at all under subscription auth?**
The flag's own help text says "API calls," and `system/init` reports
`"apiKeySource":"none"` under subscription. If it is a silently no-op, the "cheap dollar-cap
guardrail" story collapses to zero and the Phase 20 quota-floor mechanism becomes the *only*
cost control rather than a supplement. Either answer is actionable; an unanswered one is not.
Cheapest probe: set an absurdly low budget on a trivial run and observe whether the run is
refused, truncated, or unaffected, cross-checked against `total_cost_usd` in the `result`
envelope.

### The `result` envelope, captured verbatim during research

```json
{"type":"result","subtype":"success","is_error":false,"session_id":"...","num_turns":1,
 "terminal_reason":"completed","permission_denials":[],"result":"OK",
 "total_cost_usd":0.131564,"duration_ms":2341,"duration_api_ms":2166,
 "usage":{...},"modelUsage":{...},"api_error_status":null}
```

Other stream message types the parser must model (all tolerantly, per D-09): `system/init`
(carrying `session_id`, `model`, `tools`, `permissionMode`, `claude_code_version`,
`apiKeySource`, `capabilities[]`, `plugins[]`, optional `plugin_errors[]`),
`system/hook_started`, `system/hook_response`, `system/api_retry` (with an `error` category
including `rate_limit`, `overloaded`, `authentication_failed`, `billing_error`),
`assistant`, `user` (`parent_tool_use_id` non-null for subagent messages),
`rate_limit_event` (carrying `rate_limit_info{status, resetsAt, rateLimitType:"five_hour",
overageStatus}`), `stream_event` (only with `--include-partial-messages`),
`control_request` / `control_response`.

### Other headless behaviours worth designing around

- `--resume` scope is **directory-bound** — session lookup covers only the current project
  directory and its git worktrees. Always invoke from the project root, and always
  `--resume <id>` with an explicit value; a bare `-r` opens an interactive picker, which in
  `-p` with no TTY is at best an error and at worst a hang.
- Claude waits for queued output to drain before exiting, scaled to backlog and capped at
  30s. A slow stream consumer can truncate the tail of a run — another reason the reader is
  a dedicated task (D-04).
- Piped stdin is capped at **10MB**.
- Background Bash tasks are terminated ~5s after the final result; background
  subagents/workflows are waited for, capped at 10 minutes by default (D-14).
- Skills work in `-p`: `/gsd:…` invocations can be embedded directly in the prompt string and
  are expanded before running. This is what makes the driver viable at all — and is exactly
  what OQ1 tests at multi-step scale.

</specifics>

<deferred>
## Deferred Ideas

Each of these was considered and deliberately pushed out, with its owner named:

- **Run journal, `run.json`, byte-offset tailing, `FileChanged { changed_path }` path
  classification, redact-at-capture** → Phase 16. Phase 15 must not write journal files; it
  hands parsed events to its caller.
- **Detached spawn (`gsd-meta-manager drive <alias>`), `Commands::Drive`, PID+cmdline
  liveness, crash reconciliation, the user-facing kill switch, dry-run, `flock` single-run
  lock, the `driver_opt_in` config record and its UI** → Phase 17. Phase 15 owns only the
  executor-level teardown (D-14) and the capability *type* (D-23).
- **Driver tab, live stream render, ring-buffered output, three-state injection UI, dashboard
  badges** → Phase 18. Phase 15 ships no UI.
- **`--disallowedTools` git denylist, pre-push hook, secret scanning, scoped credentials** →
  Phase 19.
- **`decide()`, D-R-P-E-V router, lifting `derive_all_stage_statuses`, step/wall-clock/
  no-progress caps at the *run* level, quota-floor park** → Phase 20. Phase 15's deadlines
  (D-13) bound a single `claude` invocation, not a multi-step run.
- **LLM goal decomposition, `--json-schema`, untrusted-content delimiting, the GSD command
  enum** → Phase 21.
- **`ExecutionTarget::Container`, `PathMap`, bollard, podman, `setup-token` auth, `ps
  --format json` normalization** → Phase 22. Phase 15 introduces the enum with a single
  `Host` variant only (D-21).
- **Unix-socket control fast-path** — deferred to a later milestone by REQUIREMENTS; the
  durable file inbox covers the requirement and the socket is a latency optimization for a
  workflow whose unit of work is measured in minutes.
- **`claude agents --json` as a session-detection source** (`src/agents.rs`, ARCHITECTURE's
  N12) — strictly better than the current pgrep+`/proc` scraping and yields `sessionId` plus
  busy/idle, but REQUIREMENTS classifies it as a v1.x subsystem refactor, not a v2.0
  capability.
- **OQ4 (`--worktree` flag existence)** → Phase 17's spike. **OQ5 (podman rootless uid
  mapping / volume permissions)** → Phase 22's spike, which requires actually installing
  podman.
- **Fixing the `pending_editor` blocking shell-out on the render thread** — noted as a wart
  (D-18), not fixed here.
- **Fixing the 5 pre-existing `--all-targets` clippy lints** — unrelated; the count must not
  grow but is not this phase's to shrink.
- **`session_detector.rs`'s `pts/1` vs `pts/11` TTY substring match** — a real bug PITFALLS
  flags in passing, but it belongs to the tmux attach path, not the transport. Capture as a
  quick task.

</deferred>
