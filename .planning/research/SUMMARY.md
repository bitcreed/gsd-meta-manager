# Project Research Summary

**Project:** GSD Meta Manager — v2.0 "Autonomous Orchestration"
**Domain:** Autonomous LLM agent orchestrator retrofitted into an existing observation-only Rust/ratatui multi-project TUI
**Researched:** 2026-07-28 / 2026-07-29
**Confidence:** MEDIUM-HIGH — CLI control-surface facts are HIGH (all four researchers independently executed the local `claude` 2.1.220 binary and got consistent results); existing-codebase facts are HIGH (direct `file:line` citations); ecosystem/incident/UX evidence is MEDIUM (multi-source web, cross-checked, individually tagged in each doc)

## Executive Summary

This milestone turns a read-only dashboard into a system that spawns `claude -p`, drives it through a GSD pipeline, commits, pushes, and opens PRs unattended. All four research passes converge on the same shape: a detached driver process (survives TUI close) that talks to `claude -p` over duplex `stream-json` (not `tmux send-keys`), decides its next move with a deterministic rule-based router over the existing D-R-P-E-V state machine (LLM invoked only for goal decomposition and genuine ambiguity), and journals everything to an append-only NDJSON file under `.planning/meta-manager/runs/` that the existing file watcher already picks up for free. The codebase is unusually well-suited to this: `derive_all_stage_statuses` already computes exactly the "where are we" signal a driver needs, and the `Executor` trait sketched in the v1.2 queue-execution design is the right seam, needing only `send()`/`interrupt()` added for duplex streaming.

The single most important cross-cutting correction is that `claude -p --input-format stream-json --replay-user-messages` is a real, confirmed-delivery mid-run injection channel — this was independently verified by three of the four researchers by executing the binary. It obsoletes the reasoning behind using tmux as the control channel (screen-scraping, no delivery confirmation) while leaving the recorded decision "`-p` drives" fully intact; tmux is demoted to an optional human-attach/watch convenience, never the write path. Equally important: this milestone converts every v1.x invariant that was "free" under read-only operation (watcher correctness, concurrency safety, error cosmetics, opt-in vacuousness) into a must-be-actively-enforced property, and every safety property (kill switch, budget cap, push boundary, opt-in gate) must live in the execution path — a hook deny, a process-group kill, a branch-protection rule — never in prompt text. The Replit production-database deletion during an explicit code freeze is the canonical proof that instructions are not enforcement.

The dominant risk is not "will the LLM misbehave" but "will the plumbing around it fail silently": process-group orphaning on kill, quota (not dollar) exhaustion on a subscription, an unbounded loop that looks like legitimate work, and a driver that races the human in the same working tree. Every one of these has a cheap, well-specified mitigation (process-group SIGTERM→SIGKILL, quota-floor park condition, no-progress/repeat-command detectors, git worktree isolation) — the roadmap's job is to make sure these ship in the same phase as the capability they protect, not as a follow-up.

## Key Findings

### Recommended Stack

New dependencies are narrow and well-justified: process-wrap 9.1.0 (process-group spawn/kill — mandatory, since `claude` spawns Bash grandchildren that a bare `tokio::process` kill would orphan), bollard 0.21.0 (one typed async client covers both `connect_with_local_defaults()` and `connect_with_podman_defaults()`, collapsing the docker/podman auto-detect decision into a constructor choice), tokio-util 0.7 (`CancellationToken` for run cancellation, `LinesCodec`/`FramedRead` for NDJSON framing), uuid 1.24 (caller-chosen `--session-id`, generated before spawn so a crash before the first stdout byte is still resumable), and fs4 1.1 (advisory file lock so two meta-manager instances can't drive the same project). No new dependency is needed for the journal format — plain NDJSON matches the project's existing "state lives in files, notify watches it" idiom exactly.

**Core additions:**
- process-wrap 9.1.0 (`tokio1` feature) — process-group spawn/kill; explicit successor to the unmaintained `command-group`
- bollard 0.21.0 — one client for docker AND podman; `negotiate_version()` needed for podman's lower API version
- tokio-util 0.7 — `CancellationToken` + `LinesCodec`/`FramedRead`, nearly free as an existing transitive dep
- uuid 1.24, fs4 1.1 — session-id ownership and run-lock

**Hard constraint on the project:** MSRV must rise from 1.85+ to 1.87 (process-wrap's floor). Flag this for requirements/roadmap — it's a small but real project-level change.

### Expected Features

FEATURES.md frames this correctly as a CI/CD-run-vocabulary product (goal → steps → status → log → terminal state → cost), not a chat product — and notes the existing dashboard (`state_reader/disk_status.rs`, D-R-P-E-V) is already ~80% of the observability surface; the driver's job is to emit a decision per step onto a display that already exists.

**Must have (table stakes, v2.0 launch):**
- Per-project opt-in enforced at the process-spawn seam (compiler-checkable capability type, not a bool)
- Driver loop: disk state → next GSD command → `claude -p`
- Run record on disk (goal verbatim, decisions, cost, session IDs, terminal state)
- Kill switch: process-group kill + sentinel file, responsive during an active run
- Dry-run mode showing the command sequence AND the diffstat/refspec list it would push
- Enforced budget (`--max-budget-usd`) + turn caps, plus a driver-level cumulative cap and a quota-floor park condition
- Execution-path guardrails (`--disallowedTools` deny push/main, worktree/branch confinement) — never prompt-only
- Live run view: goal header, step timeline (one row per GSD command), elapsed, cumulative cost, bounded live output
- Park with classified terminal state + full context (never an unclassified "loop ended")
- LLM-driven badge + "needs me" triage sort on the project list
- Mid-run message injection via `--input-format stream-json` with `--replay-user-messages` delivery confirmation

**Should have (v2.1, add after real-run data exists):**
- Stall detection (needs tuning data from real runs), approve-next/step-through, retry-a-stage, rollback to pre-stage SHA, pause-vs-stop as distinct verbs, fleet aggregate burn

**Defer (v2.2+):**
- Transcript browser (the `.jsonl` link-out suffices initially), cost-per-phase analytics (needs a corpus), multi-project goal coordination, non-Claude driver backends

**Anti-features flagged explicitly by FEATURES.md** (do not build): prompt-only guardrails, tmux-send-keys-as-control, unbounded autonomy, `--dangerously-skip-permissions` as default, auto-answering `AskUserQuestion`, trusting the agent's self-report over `is_error`/`terminal_reason`/git state, auto-merge without a human diff gate, per-step auto-commit, a second progress display duplicating the existing roadmap widget.

### Architecture Approach

The driver runs as a detached child of the same binary (`gsd-meta-manager drive <alias>`), not an in-process tokio task (dies with the TUI) and not a system daemon (portability). The TUI and driver share exactly one primitive — the filesystem journal at `.planning/meta-manager/runs/<run-id>/` — so "TUI closes, run continues" and "TUI restarts, run reappears" fall out for free rather than needing bespoke recovery machinery. `decide()` is a pure, synchronous, unit-testable function; the LLM is invoked exactly twice per run in the common case (goal decomposition once, ambiguity adjudication only when the router can't resolve a case) — never on the ordinary D→P→E→V advance path.

**Major components:**
1. Executor (`src/executor/`) — revived v1.2 trait + `ClaudeExecutor`, extended with `send()`/`interrupt()` for duplex streaming; `ExecutionTarget` enum (Host/Container) inside the one executor, not a second trait impl
2. Driver (`src/driver/`) — `mod.rs` (observe→decide→act→journal loop), `policy.rs` (pure D-R-P-E-V router — the highest test-density module in the milestone), `goal.rs` (LLM at the two narrow seams), `journal.rs`, `run.rs`, `supervisor.rs` (detached spawn, PID liveness, kill switch)
3. Container (`src/container/`) — runtime auto-detect (bollard), lifecycle, docker/podman `--format json` shape normalization
4. TUI additions — new Driver tab (11th), LLM-driven/parked badges, `Action::FileChanged` extended with `changed_path` so driver journal writes route to a cheap tail instead of a full `parse_project_state`

### Critical Pitfalls

1. Git blast radius enforced only in the prompt — must be a `pre-push` hook (branch namespace + force-push/`--no-verify` denial) PLUS server-side branch protection, never a `CLAUDE.md` sentence. Directly modeled on the Replit incident (agent deleted a production DB during an explicit freeze because nothing in the execution path enforced it).
2. Kill switch that stops the driver but not the process tree — `Command::kill()`/`kill_on_drop` reach only the direct child; `claude` spawns Bash grandchildren. Must be process-group SIGTERM→grace period→SIGKILL, with `wait()` afterward, recorded pgid in run state for orphan reaping on restart.
3. Cost caps written for API billing on a subscription that isn't billed that way — `--max-budget-usd` measures a notional price; the real scarce resource is the 5-hour/7-day quota shared across all the user's Claude surfaces. A `rate_limit` event in `api_retry` must park, never retry.
4. The loop that never terminates because nothing is broken — the classic $47K/11-day and $4,200/63-hour incidents. Must be caught by step cap, no-progress detector (state hash unchanged across N iterations), repeat-command detector, and wall-clock cap — all enforced outside the model, never as a self-check the agent performs on itself.
5. `run_tui_loop` in `main.rs` is a single-consumer `rx.recv().await` with no `tokio::select!` — bolting the driver onto this as-is makes the kill key unreadable during an active run. This restructure is a prerequisite for TRANSPORT, not a nicety.

## Reconciling the Four Documents

The four documents agree on nearly everything material; where they diverge, one supersedes the other rather than averaging:

- On `--bare`: STACK.md and ARCHITECTURE.md both independently flag `--bare` as disqualified for subscription auth, and PITFALLS.md adds the sharper detail that `--bare` is "slated to become the `-p` default in a future release" — making this a dated, tracked time-bomb rather than a one-time gotcha. Resolution: `--setting-sources project` (not `--bare`) is the mechanism, and a version-gate + regression test guarding against future `--bare`-as-default is a requirement, not a nice-to-have.
- On the injection channel: all four supersede the milestone's own PROJECT.md rationale for tmux (screen-scraping had no delivery confirmation) while explicitly preserving its conclusion (`-p` drives). ARCHITECTURE.md's AP6 states this most precisely: "this refines PROJECT.md — the conclusion stands, the rationale is obsolete." Treat tmux as attach-only ergonomics throughout the roadmap, never as a phase deliverable in its own right.
- On mid-turn injection semantics: STACK.md/FEATURES.md treat `--input-format stream-json` as solved; ARCHITECTURE.md and PITFALLS.md are more cautious — mid-turn stdin is dropped by the current turn and not persisted to history (so it also vanishes on `--resume`), and `--replay-user-messages` confirms delivery of the message but not that it landed inside the active turn. Resolution: this is a MUST-SPIKE item (see below), not a settled fact. The driver must buffer interjections and flush at the `result` turn boundary, or use `control_request{subtype:"interrupt"}` if it needs mid-turn steering — the interrupt protocol itself is undocumented and needs empirical verification.
- On `--max-turns`: STACK.md documents it as functional-but-hidden (absent from `--help`, works when tested). PITFALLS.md adds a sharper finding that turn counters reset when using `--input-format stream-json` with queued messages, meaning an injection-heavy run can silently evade the turn cap. Resolution: `--max-turns` is a secondary belt; the driver's own step counter, wall-clock cap, and no-progress detector are the actual bound — do not architecturally depend on `--max-turns`.
- On the `Executor` trait's "any LLM backend" rationale: ARCHITECTURE.md is explicit that this v1.2 rationale is now weaker — the v2.0 design leans on Claude-specific protocol details (duplex stream-json, `--replay-user-messages`, control_request/interrupt) that a hypothetical Codex/Aider backend wouldn't have. Keep the trait (it's still the right seam and costs little) but model interjection capability via a `capabilities()` method rather than pretending backend parity.
- On container-as-target vs container-as-second-Executor: ARCHITECTURE.md's AP5 is decisive and none of the others contradict it — container is an `ExecutionTarget` enum inside `ClaudeExecutor` (argv prefix + path map swap), not a second trait implementation, because stream-json parsing / interjection / interrupt / cost accounting / session handling would otherwise be duplicated and drift.
- On the journal being "free" via the existing watcher: ARCHITECTURE.md and PITFALLS.md both call out that "free" has a bill — a driver appending an event every few seconds will trigger `app.rs`'s existing full `parse_project_state` re-parse repeatedly for hours unless the `FileChanged` handler classifies the changed path first and routes driver-journal paths to a cheap byte-offset tail instead. This is presented as a required `app.rs` change in both documents, not an optimization.

## Consolidated Open Questions

Merged and deduplicated from all four documents, prioritized by whether they block phase planning.

### MUST resolve via spike before planning (blocking)

1. Does `claude -p` reliably execute a multi-step GSD skill (`/gsd:execute-phase 14`, which spawns subagent waves) headlessly, without hanging? ARCHITECTURE.md flags this as open since the v1.2 design and still unverified — "the entire milestone rests on it." STACK.md's own empirical run reproduced a 180-240s hang on ANY tool-using prompt due to a `PreToolUse` hook with no timeout, resolved by `--setting-sources project`. Spike must confirm the fix generalizes to a real multi-step GSD skill invocation, not just a synthetic tool call.
2. Exact mid-turn stdin injection behavior on v2.1.220 specifically — is a message truly dropped mid-turn, and does `control_request{subtype:"interrupt"}` work as documented in community sources? This determines whether "confirm-and-steer" is buildable in v2.0 or must wait. Currently MEDIUM confidence from secondary sources only; STACK.md/FEATURES.md flagged this explicitly as "not end-to-end tested — validate in a spike before committing the injection design."
3. Does `--max-budget-usd` apply at all under subscription (non-API-key) auth, or is it silently a no-op? The flag's own help text says "API calls." If it's a no-op under subscription auth, the entire "cheap dollar-cap guardrail" story in FEATURES.md/STACK.md collapses to zero and the quota-floor mechanism becomes the only cost control, not a supplementary one.
4. Does the `--worktree`/`-w` flag exist and behave as inferred? PITFALLS.md is explicit this was inferred only from the statusline JSON schema exposing `worktree.*` fields, not from a flag reference — "verify it exists and behaves as assumed before designing worktree isolation around it." Worktree isolation is load-bearing for both driver-vs-human concurrency safety and the licensing-of-`bypassPermissions` design.
5. Podman rootless behavior is entirely unexercised — no podman is installed on the research machine. Volume permission (`--userns=keep-id`), `--format json` shape (array vs NDJSON), and the general Docker-CLI-compat assumption are all LOW confidence. Given "auto-detect docker or podman" is a user-locked hard requirement, this must be spiked (install podman, verify `bollard::connect_with_podman_defaults()` end to end) before CONTAINER phase planning finalizes.

### Can be carried into a phase (non-blocking, resolve during discuss/plan)

6. What is "done" for a goal? Needs a machine-checkable completion predicate (e.g., milestone marked shipped in STATE.md) or the driver never terminates cleanly. Resolve during DRIVER phase's discuss step.
7. Does the driver auto-answer GSD's own interactive gates (verify/UAT checkpoints), or always park? Recommendation from FEATURES.md is "always park," which may mean a fully autonomous run parks at every phase boundary by design — a value-proposition question for requirements, not architecture.
8. One driver process for the whole fleet, or one per project? Per-project is simpler to kill/reason about; fleet-level is needed for a global budget/quota cap. Resolve during DRIVER phase planning — leans per-project per ARCHITECTURE.md's chosen design (detached child per `drive <alias>` invocation), with a config-level global concurrency cap layered on top per PITFALLS.md.
9. Does the run record get committed to the repo? `run.json` (small, goal+status) committed; `journal.jsonl` (large, LLM output volume) gitignored — this is ARCHITECTURE.md's recommendation and should be treated as the default answer unless requirements pushes back.
10. Fleet-wide vs per-project quota accounting — since the 5h/7d cap is shared across ALL the user's Claude surfaces (not just this tool), and N concurrent driven projects multiply burn N×, the global concurrency cap default (suggest 1) is a PITFALLS.md recommendation to validate against user preference during DRIVER phase discuss.
11. Windows detachment — `process_group(0)` is Unix-only, `session_detector.rs` is already Linux-only (`/proc`). Treat as an explicit accepted limitation (document it) rather than silently dropping Windows support without saying so.

## Implications for Roadmap

The hard build-order constraints from the four documents, reconciled into one sequence:

- PITFALLS.md's three ordering constraints: (1) GITSAFE must be co-resident with DRIVER, never a follow-up phase; (2) TRANSPORT must ship the kill switch and dry-run in its own phase, before DRIVER exists; (3) the opt-in gate belongs in TRANSPORT, enforced at the spawn seam, not in DRIVER.
- ARCHITECTURE.md's B0-B7 band sequence, which independently arrives at the same shape and adds the container-target and UI-fix bands.
- User-locked: 999.2 (injection/monitoring plumbing) before 999.3 (driver decision layer) — the transport 999.2 builds must be stream-json over stdio, not tmux, or 999.3 has to replace it.

### Phase 1: UI Fixes (independent, parallel-safe)
**Rationale:** Zero dependencies on anything else in this milestone; ships for momentum or runs in parallel throughout.
**Delivers:** HANDOFF pause badge, DRPEV leading-blank fix, markdown edit-mode activation, PageDown clamp.
**Avoids:** Nothing safety-relevant — this is a warm-up band, sequence-independent per PITFALLS.md's own note.

### Phase 2: Transport Foundation — Executor + Duplex Stream (999.2 start)
**Rationale:** Everything else stands on this. If duplex stream-json misbehaves on a real multi-step GSD skill invocation, that must surface in week one (Open Question #1), not discovered mid-milestone.
**Delivers:** `Executor` trait revived with `send()`/`interrupt()`; `ClaudeExecutor` spawning `claude -p --input-format stream-json --output-format stream-json --verbose --replay-user-messages --session-id <uuid> --setting-sources project`, host target only. Restructure `run_tui_loop` to `tokio::select!` (Pitfall 6 — a prerequisite, not incidental). Version-gate + feature-detect via `system/init.capabilities`, never version-string comparison.
**Uses:** process-wrap 9.1.0, tokio-util (LinesCodec/FramedRead), uuid 1.24.
**Implements:** N1-N3 from ARCHITECTURE.md's component table.
**Addresses:** ROADMAP 999.2's stream-json requirement.
**Avoids:** Pitfall 6 (process-tree orphaning), Pitfall 11 (undelivered injection), Pitfall 12 (CLI version drift) — version gate ships with first spawn.
**MUST-SPIKE gate before this phase closes:** Open Questions #1 and #2 above.

### Phase 3: Run Journal + State Substrate (999.2)
**Rationale:** The state substrate every later phase needs to reattach to. Without it, the kill switch and driver tab have nothing durable to reason about.
**Delivers:** Append-only NDJSON journal writer/tailer at `.planning/meta-manager/runs/<run-id>/`, atomic `run.json` (tempfile+persist, mirroring `config.rs`), path classification in `watcher`/`app` so driver writes route to a cheap tail (not full `parse_project_state` — the "free watching has a bill" fix), noise-suppression exclusion (driver state kept off `ProjectState`'s `PartialEq`, in a sibling `AppContext` map).
**Implements:** N6/N9/N10 (journal, run model), M1/M2/M3 modifications to `action.rs`/`app.rs`/`screens/mod.rs`.
**Avoids:** the two required `app.rs` changes flagged by both ARCHITECTURE.md §4.4 and PITFALLS.md Pitfall 3/10 (feedback-loop DoS, opt-in-type retrofit debt).

### Phase 4: Supervisor — Detach, Kill Switch, Dry-Run, Opt-In Gate (999.2)
**Rationale:** This IS the stoppability hard requirement (PROJECT.md), placed deliberately before the driver exists per PITFALLS.md's constraint #2 and ARCHITECTURE.md's explicit "B3 before B4" defense: building against an in-process assumption first would bake in a wrong model that every later screen/cache/Action would need reworked. Also the opt-in gate belongs here per PITFALLS.md's constraint #3 — enforced at the process-spawn seam as a compiler-checked capability type (`DrivableProject`), not a bool checked at scattered call sites.
**Delivers:** Detached child spawn (`gsd-meta-manager drive <alias>`, new process group), PID+cmdline liveness probe (reusing `session_detector.rs`'s `/proc` technique), crash reconciliation on TUI startup, kill switch (SIGTERM→process-group→grace period→SIGKILL, `wait()` unconditional), dry-run mode that emits a diffstat + refspec list (not just a command log — this is what makes dry-run "load-bearing"), `flock`-based single-execution lock (never a PID file), `DrivableProject` capability type + `registry.rs` schema addition for `driver_opt_in`.
**Implements:** N11 (supervisor), M7 (config schema).
**Addresses:** FEATURES.md's P1 table-stakes: kill switch, dry-run, opt-in enforcement.
**Avoids:** Pitfall 6 (process-group kill mandatory), Pitfall 9 (flock not PID-file, human-detection gate), Pitfall 10 (opt-in as a type).

### Phase 5: Driver Tab UI + Durable Injection (999.2 completion)
**Rationale:** Completes 999.2's "monitor output and inject commands from the TUI." A human can now drive a project step-by-step from the TUI — a natural, independently useful ship point before any autonomy exists.
**Delivers:** Driver tab (goal header, live stream render bounded to a ring buffer, queued/delivered/acted-on three-state injection UI), durable inbox path (`inbox.jsonl`) first, socket fast-path deferred. LLM-driven + parked badges on the project list, "needs me" triage sort.
**Implements:** N13, M4-M6.
**Addresses:** FEATURES.md's live run view, mid-run injection, badge requirements.
**Avoids:** Pitfall 11 (three-state delivery confirmation, not "we wrote to the pipe = sent").

### Phase 6: GITSAFE — Git/VCS Blast-Radius Envelope
**Rationale:** Co-resident with DRIVER per PITFALLS.md's hardest constraint — must exist before DRIVER's first unattended run, not after. Sequenced here, immediately before the decision layer, so the decision layer is built and tested against a real safety envelope from day one rather than a stub.
**Delivers:** Branch-namespace push allowlist enforced by a `pre-push` hook the meta-manager installs and re-asserts (deny `--force`/`--no-verify`/`core.hooksPath` rewrites via `--disallowedTools`), server-side branch-protection recommendation surfaced to the user, pre-push secret scan (gitleaks or equivalent, full-worktree not diff-only), redact-at-capture on the stream-json log (not render-time), scoped per-run git credential (never inherit the user's ambient credentials/SSH agent), PR-per-24h rate cap.
**Addresses:** ROADMAP's "fully autonomous including git push and PR creation" requirement, made safe.
**Avoids:** Pitfall 1 (git blast radius) and Pitfall 2 (secret leakage) — the two CRITICAL pitfalls most explicitly named as unconditional requirements, not options.

### Phase 7: Deterministic Decision Router + Dry-Run Integration (999.3 start)
**Rationale:** Rules before LLM. If the LLM layer lands first there is enormous pressure to let it paper over router gaps, and the router never gets written properly.
**Delivers:** Pure, synchronous `decide()` function — lift `derive_all_stage_statuses` out of `ui/screens/detail.rs` into `state_reader/` as a shared primitive; hard-gate ordering (paused → external-job-waiting → blocked → circuit-breaker → budget/step cap → milestone-terminal → D-R-P-E-V table lookup); no-progress detector (state hash unchanged across N iterations), repeat-command detector, wall-clock cap, quota-floor park condition (reads `api_retry` events, never retries on `rate_limit`). Dry-run mode wires through this layer cheaply since `decide()` is pure.
**Implements:** N7 (policy.rs) — highest test density in the milestone.
**Addresses:** FEATURES.md's P1 driver-loop and enforced-cap items.
**Avoids:** Pitfall 3 (runaway/oscillation — all five controls named must land here) and Pitfall 4 (cost/quota — the subscription-vs-dollar mismatch is the sharpest single finding across all four documents; treat `--max-budget-usd` as an anomaly circuit-breaker only, never as "the" cost control).

### Phase 8: LLM Goal Layer + Prompt-Injection Hardening
**Rationale:** Last, deliberately. By this point the run is fully observable, stoppable, and safety-enveloped, so the first genuinely autonomous decision has a net beneath it.
**Delivers:** `--json-schema`-enforced goal decomposition into a structured `RunGoal` (reviewed by the human before the run starts — a cheap, high-value blast-radius control), bounded `Ambiguous` adjudication with a per-run cap, structural prompt delimiting (`.planning/` content in an explicit untrusted block, never concatenated into instructions), command-output restricted to a fixed GSD-command enum (never `eval`'d shell strings), `--strict-mcp-config` to block project-local `.mcp.json` injection.
**Implements:** N8 (goal.rs).
**Addresses:** ROADMAP 999.3's driver decision layer.
**Avoids:** Pitfall 8 (prompt injection through `.planning/` — a real threat model here since this tool drives other people's cloned projects, not a theoretical one) and the anti-pattern of letting the LLM pick every GSD command.

### Phase 9 (parallel, can run alongside 4-8): Container Target
**Rationale:** Depends only on the executor (Phase 2); explicitly can run in parallel with Phases 4/5, but must land before Phase 7 closes so the driver never needs a stubbed target, and must resolve Open Questions #4/#5 (worktree flag, podman rootless) via spike first.
**Delivers:** `ExecutionTarget::Container`, docker/podman runtime auto-detect via bollard, named-volume-per-project + `CLAUDE_CONFIG_DIR` credential delivery (never bind-mount host `~/.claude`), `claude setup-token`/`CLAUDE_CODE_OAUTH_TOKEN` for subscription auth in-container, rootless podman `--userns=keep-id` + `:Z` SELinux flags, egress restriction (`init-firewall.sh`-style allowlist), non-root container user, `ps --format json` shape normalization (docker NDJSON vs podman array), `PathMap` host↔container path translation as a real type.
**Uses:** bollard 0.21.0.
**Addresses:** ROADMAP's docker/podman auto-detect hard requirement.
**Avoids:** Pitfall 7 (container credentials — the "two-file trap" `~/.claude` vs `~/.claude.json`) and Pitfall 9's container-specific concurrency corruption (mount-path parity with host, so `--resume` works identically in both).

### Phase Ordering Rationale

- GITSAFE (Phase 6) is placed immediately before the decision layer (Phase 7), not after it, so the decision layer's own tests run against a real safety envelope rather than a stub — this directly implements PITFALLS.md's non-negotiable "co-resident, not follow-up" constraint.
- The supervisor (Phase 4, kill switch/dry-run/opt-in) precedes the Driver tab (Phase 5) and the decision layer (Phase 7) because retrofitting detachment onto UI or decision code that assumed in-process ownership would force rework of every screen, cache field, and Action — a cost paid once now instead of repeatedly later.
- Rules (Phase 7) precede the LLM goal layer (Phase 8) so that ambiguity is forced to be named and handled by the router before an LLM is ever allowed to paper over a gap in it.
- Container (Phase 9) is deliberately parallel-eligible rather than sequential, since it depends only on the Phase 2 executor abstraction and touches no driver-decision code — but its spike-dependent open questions (#4, #5) mean it should start early enough that a stall there doesn't block Phase 7's close.

### Research Flags

Phases likely needing deeper `/gsd-plan-phase --research-phase` research during planning:
- **Phase 2 (Transport):** the stream-json protocol details (mid-turn drop, control_request/interrupt shape) are officially undocumented per all four researchers — needs a dedicated empirical spike, not just planning-time research.
- **Phase 7 (Decision Router):** GSD's own autonomous-mode semantics and `WAITING.json`/checkpoint contract — PITFALLS.md flags the existing Phase 13 queue-execution design as "four months stale" and needing re-verification before building on it.
- **Phase 9 (Container):** podman rootless uid mapping and volume permissions — asserted from Docker/devcontainer-centric docs, not verified; genuinely needs a hands-on spike since podman isn't installed on any research machine.

Phases with standard, well-documented patterns (research-phase can likely be skipped):
- **Phase 1 (UI fixes):** established codebase idioms, no new external unknowns.
- **Phase 4 (Supervisor):** process-group signal handling and `flock` are standard Unix patterns, corroborated across multiple independent sources by all four researchers.
- **Phase 6 (GITSAFE):** branch-protection/pre-push-hook patterns are well-established industry practice (GitHub Copilot's own `copilot/`-prefix model is a direct precedent).

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Crate versions verified live against crates.io; CLI flags verified by executing the local binary, not inferred from docs |
| Features | MEDIUM-HIGH | Control-surface facts HIGH (direct execution); competitive/UX evidence MEDIUM (multi-source web, individually cited) |
| Architecture | HIGH for existing-code claims (every one cites file:line); MEDIUM for ecosystem orchestration-pattern claims (web synthesis, no single authority) |
| Pitfalls | MEDIUM overall — codebase claims HIGH; first-party Anthropic doc claims tagged `LOW/first-party` by the confidence-classifier's provider-based tiering but treated as reliable; incident/community claims tagged `LOW/community`, directionally strong but numerically approximate |

**Overall confidence:** MEDIUM-HIGH. The control-surface facts that the whole design depends on (stream-json exists, `--replay-user-messages` acks, SIGTERM→143, subscription auth works under `-p`) were independently verified by execution across multiple researchers — this is unusually strong grounding for a pre-implementation research pass. The open questions that remain (mid-turn injection semantics, `--max-budget-usd` under subscription auth, podman rootless behavior, GSD multi-step skill execution under `-p`) are exactly the ones flagged as MUST-SPIKE above, and none of them invalidate the overall architecture — they refine specific mechanism choices within it.

### Gaps to Address

- Mid-turn injection semantics (Open Q #2): spike in Phase 2 before the Driver tab's injection UI is built on an assumption.
- `--max-budget-usd` under subscription auth (Open Q #3): spike in Phase 2 or 7; if it's a no-op, the quota-floor mechanism must be promoted from "supplementary" to "the only cost control" in the roadmap language and documentation.
- `--worktree` flag existence (Open Q #4): spike before Phase 4's isolation design locks in; fallback is manually creating the worktree with `git worktree add` and pointing the driver's cwd at it — a safe fallback exists either way.
- Podman rootless (Open Q #5): must install podman and exercise the full path in Phase 9 — currently zero direct verification.
- GSD multi-step skill under `-p` (Open Q #1): the highest-stakes unknown — if this doesn't work reliably, the entire premise needs rethinking, so it should be the very first thing tested in Phase 2, ahead of any other Phase 2 work.

## Sources

### Primary (HIGH confidence)
- Live execution of `claude` v2.1.220 (`--help`, `agents --json`, `-p` with various flag combinations) — flag inventory, stream-json envelope shapes, hang reproduction (3/3), auth behavior under subscription
- Direct repository source inspection: `src/main.rs`, `src/app.rs`, `src/action.rs`, `src/event.rs`, `src/watcher.rs`, `src/config.rs`, `src/cli.rs`, `src/session_detector.rs`, `src/terminal_switch.rs`, `src/state_reader/`, `src/ui/screens/`
- `.planning/PROJECT.md`, `.planning/ROADMAP.md`, `.planning/milestones/v1.2-phases/13-queue-execution-research/QUEUE-EXECUTION-DESIGN.md`
- cargo/crates.io live registry data for all new dependency versions

### Secondary (MEDIUM confidence)
- Official docs: Run Claude Code programmatically (headless), CLI reference, Development containers, Statusline — tagged LOW/first-party by the confidence classifier's provider-based tiering but treated as reliable for mechanism claims
- AI Incident Database #1152 (Replit), Cursor forum cost complaints, Devin/OpenHands/GitHub Actions competitive analysis, IssueTrojanBench guardrail-bypass study

### Tertiary (LOW confidence — flagged for validation)
- `--input-format stream-json` mid-turn semantics and control_request/interrupt shape — community GitHub issues only, officially undocumented, MUST-SPIKE
- Podman rootless volume/uid behavior — web synthesis, unexercised locally
- Exact cost figures in incident write-ups — self-reported forum anecdotes, directionally reliable not numerically precise

---
*Research completed: 2026-07-29*
*Ready for roadmap: yes*
