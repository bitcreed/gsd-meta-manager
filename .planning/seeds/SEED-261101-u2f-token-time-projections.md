---
id: SEED-261101-u2f
status: dormant
planted: 2026-11-01
planted_during: v2.0 Autonomous Orchestration (shipped; between milestones)
trigger_when: "when scoping the next milestone after v2.0, or when cost/time visibility, token usage, or forecasting becomes a priority"  # inferred (human unavailable) — narrow via --enrich
scope: large  # inferred from the 5-phase outline below — a full milestone
audit_acknowledged:
  milestone: v2.3.0
  at: 2026-11-10
  status: dormant
---

# SEED-261101-u2f: Token & time projections

Status: brainstorm (2026-11-01), not yet a milestone and not ready for planning. Suggestions recorded; user decisions marked [DECIDED]. Items marked [INFERRED] were set at capture time without the user present and need audit.

## Why This Matters

Track per-project and global token spend and execution time (by phase, step kind, plan, role, model, provider) and project the remaining cost of milestones/phases. Must answer questions like:

- "Based on previous milestones, this one will take x Mtok and 5h30 on balanced, 7h on quality."
- "Phase 4 will take ... based on phases 1-3."
- "A discuss step takes ~5 ktok and ~20 min; verify takes 3 ktok and 15 min on Sonnet, 25 min on Opus."
- "Average/median project: 4 milestones, 25 phases, 24 Mtok, 14h30 on balanced."

Must work for Claude Code; Codex support desirable (lower fidelity acceptable).

## When to Surface

**Trigger:** when scoping the next milestone after v2.0, or when cost/time visibility, token usage, or forecasting becomes a priority. [INFERRED]

This seed will surface during `/gsd-new-milestone` when the milestone scope matches.

## Scope Estimate

**Large** — a full milestone; see the 5-phase outline below. [INFERRED]

## Decisions so far

- [DECIDED] Headline unit = tokens (not $, not subscription %) — subscriptions vary too much across users/time.
- [DECIDED] Tokens counted at charged value: cache reads at their discounted weight (e.g. x0.1), cache writes at their premium (5m x1.25, 1h x2.0). Raw buckets always kept in the ledger. Multipliers per provider/model in an editable table. Open: output weighted x1 (recommended, keeps "tokens" intuitive) vs x5 (becomes $-in-disguise).
- [DECIDED] Collection via a Claude Code `SessionEnd` hook running `gsd-meta-manager ingest --session <id>` (one-shot, no daemon). Opt-in, installed by the app itself (not via a GSD capability). Codex: cron/systemd timer or Codex `notify` (unverified).
- [DECIDED] Backfill: import existing transcripts on first run. Recommend raising `cleanupPeriodDays` (default 30 → transcripts deleted) — the app's own ledger is the only durable history.
- [DECIDED] SQLite acceptable, preferably optional (cargo feature, default on); JSONL aggregates as baseline.
- [DECIDED] No gsd-core changes required for v1 of the feature. gsd-core proposals filed only AFTER the feature ships, linking this app as the use case.
- [DECIDED] Multi-machine / multi-dev: "car range" model. Route (roadmap, plan counts, commit timestamps) is shared via git; the car (per-role/model rates) is the local ledger. Different machines → different projections is fine; must be close enough to be useful, not perfect.

## Data sources (verified on disk)

- `~/.claude/projects/<slug>/<session>.jsonl`: per assistant turn `message.usage` (input, cache_creation 5m/1h, cache_read, output, thinking), `message.model`, `timestamp`, `cwd`, `gitBranch`, `isSidechain`, `agentId`.
- `<session>/subagents/agent-*.jsonl` + `agent-*.meta.json`: `agentType` (e.g. gsd-planner), `description`, `model`, `parentAgentId`, `spawnDepth`, `toolUseId` → spawn prompt in parent transcript names phase/plan.
- `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`: `token_count` events with cumulative `total_token_usage` (input, cached_input, output, reasoning), `rate_limits`, session cwd.
- `.planning/config.json`: `model_profile`, `model_overrides`, `parallelization`, `granularity` (intended, not actual).
- PLAN.md frontmatter `estimate.tokens` and SUMMARY.md `actuals` / `metrics.duration` — already read by the app (`src/state_reader/disk_status.rs:212`, shown as "est 310k act 62.0k" in the Waves title, `src/ui/screens/detail.rs:10444`). Both are agent-authored; the ledger adds a *measured* third number.
- git commits per plan: reliable timestamps for time-window attribution.

## Attribution (in fidelity order)

1. Subagent meta.json role + spawn-prompt regex for phase/plan; `<command-name>` tags in parent session.
2. Worktree cwd → project via git common dir.
3. Time windows between a plan's first/last commit (primary method for Codex).
4. Leftover → "project, unattributed". Each ledger row carries attribution confidence: exact | inferred | unattributed.

Pitfalls: dedupe by `message.id` + `requestId` (streamed chunks, resumed/forked sessions copy history) — check ccusage as prior art; worktree cwd mapping; sessions from other machines have no token data (show "time only").

## Data model

Work unit row: project, milestone, phase, step kind (discuss | research | plan | plan-check | execute-plan | verify | gap-closure | quick), role, model, provider, machine, token buckets, active time, wall time, attribution.

Wall time for interactive steps excludes idle gaps > N min (human waits can be days/weeks — not tracked).

## Projection model

```
milestone ≈ Σ phases [ phase overhead (discuss+research+plan+check+verify by role×model)
                     + Σ plans executor cost (by plan size)
                     + P(gap-closure) × rework-round cost ]
```

- Percentiles (p50 + p10–p90 via bootstrap), not ±x% — long-tailed (gap-closure loops).
- Unplanned phases: plan count from history per `granularity`; wider band.
- Profile what-if: re-weight roles via learned cross-model ratios, fall back to price ratio.
- Fallback hierarchy with shrinkage: this project/this machine → all projects/this machine → bundled priors.
- Wall time = critical path: waves sequential, plans within a wave parallel (capped by parallelization).
- Backtest predicted vs actual for completed phases (builds trust in the band).
- Quick win: calibrate the planner's `estimate.tokens` by the historical measured/estimate ratio.

## gsd-core findings (checked ~/projects/node/gsd-core @ ebc5a1528, installed 1.19.0)

- Third-party capabilities supported since 1.6.0 (ADR-1244): global `~/.gsd/capabilities/<id>/`, project `.gsd/capabilities/<id>/` (consent bound to bundle contentHash). Full parity: hooks, MCP servers, command modules.
- Capabilities can register their own `gsd_run` command family: `commands: [{family, module, router}]`, module is a bare `*.cjs` basename required from the capability's own root (capability-validator.cjs ~L425). → a boundary-marker command is possible without gsd-core changes (tiny Node shim calling `gsd-meta-manager mark`).
- Loop points: discuss|plan|execute|verify|ship :pre/:post, execute:wave:pre/post. `contribution` hooks into the orchestrator are dispatched in execute-phase.md (L622, L991); NOT yet verified for discuss/plan/verify workflows.
- Capabilities can also write settings.json hooks with ledger-tracked removal (alternative SessionEnd installer).
- Optional capability (later phase): contribution hook tagging Agent descriptions `[gsd:21-03:executor]` → exact attribution via meta.json; step/command markers at loop points → exact wave/phase boundaries.

## UI concepts (after overload feedback)

Rule: list views carry ONE number per row; detail lives in the panel/tab for the selected row.

- **Dashboard** — single "Remaining" column (p50 tok · wall), uncertainty shown by rounding (`~6M · 6h ?` for thin data), one-line footer "in flight 12.3M · 11h20 · [$] stats".
- **Roadmap tab** — lanes unchanged except `~4p` (estimated plan count). Right panel becomes "Forecast": milestone remaining per profile (balanced/quality/budget, [p] cycles), band + gap-closure risk as an explicit add-on, selected-phase block (spent / left), compact Shipped summary.
- **Phases tab** — Steps table: step · model · tok · vs typical · active · wall (no waiting-on-user column). Waves title: "measured 1.18M · plan est 310k · self-rep 62.0k"; per-plan model/tok/time; calibrated-estimate line.
- **Global Stats screen** (`$` from dashboard) — user liked as-is: typical step medians with p10–p90 by model, typical phase/milestone/project (balanced vs quality wall), model mix bars, ledger health (turns, sessions, backfill start, % unattributed, hook status). Period and unit toggles, by-model/by-role views, CSV export.
- **Cfg tab** — Metrics section: hook install/uninstall, Codex timer, cleanupPeriodDays warning + fix, backfill re-scan, weighting table, fallback/band settings.
- **Sess tab** — tok · active column and attribution (→ P21 execute 21-02) per session.

## Suggested phase outline

1. Ingest + ledger (SessionEnd hook installer, Claude + Codex readers, dedupe, offset index, backfill, `ingest` subcommand)
2. Attribution (subagent meta, spawn prompts, commit windows, worktree mapping, confidence)
3. Projection engine (step-kind model, fallback hierarchy, bands, backtest, planner-estimate calibration)
4. TUI (dashboard column, Forecast panel, Phases steps, Stats screen, Cfg metrics, Sess column)
5. Optional GSD capability (tagging + markers)

## Idea pot / future

- MCP mode: gsd-meta-manager as an MCP server to answer detailed questions straight from the ledger.
- Opt-in team stats export/merge (never written into the repo by default).
- Range warning: "remaining milestone won't fit in the current 5h window".
- Budget alerts per project/milestone. CSV export.
- gsd-core upstream proposals (after shipping, linking this app): run id in spawn prompts; tool-written SUMMARY frontmatter (started_at/ended_at ISO, resolved model per agent).

## Open questions

- Output token weight x1 vs x5.
- Whether discuss/plan/verify workflows dispatch orchestrator contributions (needed for the optional capability).
- Codex collection trigger (`notify` vs timer) and per-model cache discount.

## Breadcrumbs

- `src/state_reader/disk_status.rs:212` — reads PLAN.md `estimate.tokens` / SUMMARY.md `actuals.tokens` (per-plan token cost struct ~L485).
- `src/ui/screens/detail.rs:10444` — Waves title "est … act …"; target for the "measured · plan est · self-rep" line.
- `.planning/config.json` — `model_profile`, `model_overrides`, `parallelization`, `granularity` inputs.
- No related ROADMAP phases or pending todos found at capture time.

## Notes

Captured via one-shot seed capture from the 2026-11-01 brainstorm; full content preserved. Trigger and scope were inferred at capture (human unavailable) — confirm via `/gsd-capture --seed --enrich SEED-261101-u2f`.
