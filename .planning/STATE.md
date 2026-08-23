---
gsd_state_version: 1.0
milestone: v2.0
milestone_name: Autonomous Orchestration
current_phase: 21
current_phase_name: llm-goal-layer-prompt-injection-hardening
status: executing
stopped_at: Phase 19 human verification deferred by user; starting Phase 20 autonomous run
last_updated: "2026-08-23T03:47:50.358Z"
last_activity: 2026-08-22
last_activity_desc: Phase 21 execution resumed (wave continue)
state_head: 84143bbf0bce375e0273700377c508cb7ba1cc13
progress:
  total_phases: 10
  completed_phases: 6
  total_plans: 70
  completed_plans: 68
  percent: 60
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-31)

**Core value:** See the state of every GSD project at a glance and act on any of them without leaving the TUI.
**Current focus:** Phase 21 — LLM Goal Layer & Prompt-Injection Hardening

## Current Position

Phase: 21 (llm-goal-layer-prompt-injection-hardening) — READY TO EXECUTE
Plan: 1 of 18
Status: Ready to execute
  human-judgement UAT items were deferred by explicit user decision on 2026-08-19, not resolved.
Last activity: 2026-08-22 — Phase 21 execution started

## Deferred Verification

| Phase | State | Resume |
|-------|-------|--------|
| 19 | verification_deferred_human | /gsd-verify-work 19 |

Phase 19's `19-VERIFICATION.md` remains `status: human_needed` — 5/5 automated must-haves are
verified, and the four outstanding items are all reading judgements (does the pinned honesty
statement read as candour; are 19-08's composed proofs faithful decompositions; do the
residual-exposure disclosures read as admissions) plus the `driver_lock` one-off risk call.
Deferred by user request on 2026-08-19 so Phase 20 could start; **not** marked passed.

### Phase 17 gap-closure notes (autonomous run — review these)

- **The code review (`17-REVIEW.md`, `issues_found`) found SIX BLOCKERS after all 7 plans had
  executed and the gate was green.** All six were in the kill switch and its liveness probe —
  the phase's own safety contract — and each made the goal's word "stoppable" false on a
  reachable path. Plan **17-08** closed them: CR-01 (SIGTERM buffered during
  `Executor::start()`, orphaning the agent group), CR-02 (the stop signalled an agent-writable
  `pgid` out of `run.json`), CR-03 (the detached spawn dropped the TUI's `--config`, so a
  driver could run in a different project than the user selected), CR-04 (a run whose
  `--run-id` was not on argv read as dead and could not be stopped), CR-05 (`/proc`-only
  liveness answering `false` off Linux, consumed as "already gone"), CR-06 (unregistering a
  project abandoned its live agent with no stop path).

- **Seventeen WARNINGS (WR-02..WR-17) are deliberately DEFERRED**, with a one-line reason each
  in `17-08-PLAN.md`'s deferral table. WR-01 was folded in because CR-02's kernel-vs-record
  agreement check is meaningless while the record's `pgid` is assumed rather than read. The
  highest-value carry-forwards for a later phase: **WR-02** (`run_id` is joined into a path with
  no component validation — the run directory can escape the project, reproduced), **WR-10**
  (blocking `flock`/fs/git inside `async fn`, which already caused an observed deadlock in
  `tests/driver_lock.rs`), **WR-15** (`DriverStopped` drops the observed run even when nothing
  was stopped) and **WR-16** (the hidden `--claude-program` flag ships in release builds).

- **`ObservedRun.live: bool` became `liveness: Liveness` + `is_live()`** so the third state
  CR-05 requires can be carried. Phase 18 consumes `ObservedRun`; it must read the tri-state
  rather than reintroducing a boolean.

- **`--run-id` is now REQUIRED for a real `drive`** (a preview still works without one). The
  "generated when absent" mode `cli.rs` and `driver/mod.rs` used to document is gone, because a
  generated id never reaches argv and therefore made the run invisible to liveness.

### Phase 15 planning notes (autonomous run — review these)

- **All three MUST-SPIKE questions resolved empirically** against the local `claude` 2.1.220
  binary during the research step, in a throwaway scratch dir, never against this repo.
  **OQ1 CONFIRMED (bounded)** — `--setting-sources project` suppresses the `PreToolUse` hook
  hang: without it, exit 124 with `duration_ms` 89134 vs `duration_api_ms` 3447; with it, exit 0
  in 8s. The hung arm emits `hook_started`/`hook_response` *before* `system/init`; the mitigated
  arm emits no hook events. The multi-step generalisation (a GSD skill spawning subagent waves)
  remains plan 15-01 Task 1, the phase gate. **OQ2 CONFIRMED** — mid-turn stdin injection is
  QUEUED and runs as its own turn, refuting ARCHITECTURE's AP3; `control_request{subtype:"interrupt"}`
  works, bare `{"type":"interrupt"}` does nothing. **OQ3 CONFIRMED** — `--max-budget-usd` does
  apply under subscription auth (`apiKeySource: "none"`), yielding
  `error_max_budget_usd`/`budget_exhausted`, but only as a *post-turn* circuit breaker: it bounds
  the next turn, never the current one. D-16 stands; Phase 20's quota floor is still the real cost
  control.

- **Structural finding no research document anticipated:** `type:"result"` is a **turn** boundary,
  not a **run** terminator. A run receiving a second stdin message emits two `system/init` and two
  `result` envelopes in one process. An executor returning on the first `result` would truncate
  every steered run while reporting success. Recorded as amendments **D-29..D-32** in
  `15-CONTEXT.md` so the decision-coverage gate forces the plans to handle it.

- **Decision-coverage gate is live again and PASSES 32/32.** `15-CONTEXT.md` uses the
  `- **D-NN:** …` bullet form, which fixes the `could-not-parse` failure recorded for Phase 14
  below. Note for future phases: the gate matches `\bD-NN\b` **only inside designated sections**
  (plan frontmatter `must_haves`/`truths`/`objective`, designated headings, XML tag bodies), so
  plans must cite the ids inline, not merely implement the decisions.

- **ROADMAP gained an authoritative `**UI hint**: no` for Phase 15.** The blocking `ui.plan-gate`
  fired because `checkUiPresence` token-sniffed the bare word `ui` out of a risk bullet that refers
  to *Phase 18's* injection UI. Phase 15 ships no visual surface, so the author-declaration form was
  used rather than a transient `--skip-ui`, making the record durable for verify-work and progress.

- **`--research-phase` was interpreted as `--research`.** ROADMAP's Phase 15 entry says
  `Research: yes — /gsd-plan-phase --research-phase`, but that flag is research-**only** mode and
  exits before the planner runs, producing no plans. Ran the full research→plan→verify flow instead.
  The same wording appears on Phases 20 and 22 and will need the same reading.

- **Seven raw spike transcripts staged, gitignored.** `.planning/phases/15-transport-foundation/transcripts-raw/`
  holds the captured NDJSON (clean success, budget-exhausted, tool-use success, the hook hang, the
  two-turn queued injection, interrupt-during-streaming, and the interrupt race). They carry absolute
  host paths, so a `.gitignore` entry prevents accidental commit; plan 15-01 redacts and promotes
  them into `tests/fixtures/` and plan 15-02 deletes the staging directory. Scanned for
  credential-shaped strings — none found.

### Phase 14 planning notes (autonomous run — review these)

- **Decision-coverage gate overridden.** `check.decision-coverage-plan` returned
  `passed: false, reason: could-not-parse, total: 0, uncovered: []` — `14-CONTEXT.md` writes its
  decisions as `### UIFIX-0N — title` headings rather than the `- **D-NN:** …` bullets the parser
  requires, so zero decisions were extracted and none could be reported uncovered. Proceeded
  deliberately: retrofitting `D-NN` ids would make the gate report every id as uncovered (the plans
  cite `UIFIX-NN`), converting a cosmetic parse failure into a real block. The substantive check was
  performed instead by `gsd-plan-checker`, which returned VERIFICATION PASSED and confirmed each
  CONTEXT.md decision (verify-first, fix-at-source, key-routing, clamp-at-handler, testing
  convention, scope fences) is honored. **Follow-up:** consider normalising CONTEXT.md decision
  format to `D-NN` bullets for future phases so this gate is live.

- **Plan-checker's clippy warning was a false positive.** It reported the "5 pre-existing
  `--all-targets` lints" in `14-CONTEXT.md` as stale (claiming 0 today). Re-measured directly:
  `cargo clippy --all-targets -- -D warnings` still fails with exactly **5** pre-existing lints —
  3× `assert_eq!` with a literal bool (`browser.rs`), 1× owned-instance-for-comparison
  (`project_creator.rs`), 1× items-after-test-module (`state_reader/mod.rs`). The checker measured
  without `-D warnings`. `14-CONTEXT.md` and `14-PATTERNS.md` are **correct as written** and were
  deliberately NOT "corrected". The lib-target project gate `cargo clippy -- -D warnings` passes clean.

- **UI-SPEC gate honored, not skipped.** ROADMAP marks Phase 14 `UI hint: yes` and the blocking
  `ui.plan-gate` fired; `14-UI-SPEC.md` was generated and approved 6/6 by `gsd-ui-checker`.
  Research was skipped per ROADMAP (`Research: skip`) — no RESEARCH.md exists for this phase.

## Performance Metrics

**Velocity:**

- Total plans completed: 37
- Average duration: --
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 14 | 4 | - | - |
| 15 | 8 | - | - |
| 16 | 6 | - | - |
| 17 | 8 | - | - |
| 18 | 11 | - | - |

**Recent Trend (from v1.1):**

- Last 5 plans: 4min, 3min, 3min, 4min, 3min
- Trend: Stable (~3-4 min/plan)

*Updated after each plan completion*
| Phase 11 P01 | 3min | 2 tasks | 4 files |
| Phase 12 P01 | 2min | 2 tasks | 5 files |
| Phase 12 P02 | 18min | 2 tasks | 3 files |
| Phase 13 P01 | 15min | 2 tasks | 1 files |
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 18 P10 | 5h | 3 tasks | 6 files |
| Phase 18 P11 | 45min | 3 tasks | 8 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [v1.2]: Tech debt first to prevent archive browser from needing deleted ScreenAction variants
- [v1.2]: No new Cargo dependencies for v1.2 (except potentially pulldown-cmark for archive markdown styling)
- [v1.2]: Archive browser uses existing ListState pattern (not tui-tree-widget)
- [v1.2]: HANDOFF.json requires content check (not existence-only) to avoid stale badges
- [Phase 11]: HANDOFF file detection with non-empty content validation for pause badges
- [Phase 12]: Archive module in separate src/archive.rs file (not in detail.rs) for testability
- [Phase 12]: Abbreviated tab labels (5:Pipe, 7:Sess) to fit 8 tabs within 80 columns
- [Phase 13]: Strategy A (per-item isolated execution) recommended for v1.3 queue execution
- [Phase 13]: LLM-agnostic Executor trait interface for queue execution backends
- [Phase ?]: The four injection states are a pure set intersection over inbox.jsonl and the run's journal, recomputed every frame — no cache a restart could lose (18-10)
- [Phase ?]: interjected { delivered: false } stays queued: the journal is recording a FAILED stdin write, and promoting it would overstate what happened (18-10)
- [Phase ?]: terminal_state_cell matches a render-layer TerminalState with no wildcard, and TerminalState::from_outcome matches RunOutcome with no wildcard — a new upstream variant is a compile error while the table still covers the three verdict-only states (18-10)
- [Phase ?]: driver::run::outcome_label is pub(crate) so the render vocabulary is proved against the string the driver writes, not restated in a test (18-10)
- [Phase ?]: The help popup shares clamp_scroll/ViewportMetrics/PAGE_SCROLL_LINES with every other scrolling pane and gates its more-content indicator on a comparison, so no third copy of the max-scroll formula exists (18-11)
- [Phase ?]: Key documentation is asserted as WHOLE ROWS, never by contains(key) — a substring check on a one-letter key is vacuous, since "next" contains "x" (18-11)
- [Phase ?]: ProjectViewCache.driver_dry_run is a single Option where presence IS the mode; a value plus a separate active flag is two fields that can disagree (18-11)
- [Phase ?]: Action::DriverDryRunLoaded stores under two guards — a preview must be open AND its command must match — so a stale report cannot show one command's blast radius under another's name (18-11)
- [Phase ?]: prune_driver_maps now covers last_refresh and archive_cache too, and every_per_alias_driver_map_is_pruned destructures AppContext exhaustively, so a new alias-keyed field is a compile error until it has been classified (18-11)
- [quick 260729-vmp]: A new opt-in affordance PUSHES the existing DriverConfirmScreen and never writes the registry itself — do_toggle_opt_in stays the single write path that has to agree with the spawn seam (CTRL-03)
- [quick 260729-vmp]: `o` is tab-scoped on the detail screen DESPITE having no collision to resolve, so it cannot silently become a global detail-screen opt-in on tabs where it is undiscoverable; the scoping test is what makes the guard an enforced property rather than a comment (CTRL-03)

### Pending Todos

- ~~Phase 15 must spike three MUST-SPIKE questions before it closes~~ — **OQ2 and OQ3 are
  CLOSED** (resolved empirically during Phase 15 research, 2026-07-29; evidence and verbatim
  transcripts in `15-RESEARCH.md` §"Spike Outcomes"). **OQ1 is CONFIRMED at single-tool-call
  scale only**; the multi-step generalisation — a real GSD skill spawning subagent waves, run
  headlessly to completion — is plan **15-01 Task 1** and gates the whole phase. Two things
  the bounded probe could not answer and Task 1 must: whether `--setting-sources project`
  propagates to the nested `claude` processes subagent waves spawn, and what happens when a
  long run crosses the silent 10-minute `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` ceiling.

- Phase 17 must spike OQ4 (`--worktree` flag existence) before locking worktree isolation
- Phase 22 must spike OQ5 (podman rootless uid mapping / volume permissions) with podman
  actually installed

- MSRV rises 1.85 -> 1.87 in Phase 15 (process-wrap floor)

### Blockers/Concerns

- Tab bar overflow at 80 columns when adding 8th tab (Archive) -- resolve at Phase 12 design time
- `--bare` is slated to become the `-p` default and is incompatible with subscription
  auth; Phase 15 ships a version gate and a regression guard against it

- Phase 13's queue-execution design self-dated "valid until 2026-04-30"; re-verify GSD's
  autonomous-mode / checkpoint contract during Phase 20 research

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260325-reh | Fix 6 tech debt items from v1.0 audit | 2026-03-26 | 53b1ce5 | [260325-reh](./quick/260325-reh-fix-6-tech-debt-items-from-v1-0-audit-in/) |
| 260327-rhx | Rename project to gsd-meta-manager | 2026-03-28 | e018917 | [260327-rhx](./quick/260327-rhx-rename-the-project-to-gsd-meta-manager/) |
| 260401-t7y | Add tui-textarea + $EDITOR shell-out for archive/backlog | 2026-04-02 | 248a800 | [260401-t7y](./quick/260401-t7y-add-tui-textarea-for-markdown-viewing-an/) |
| 260403-p84 | Add PageUp/PageDown scrolling to detail screen | 2026-04-04 | 3d8d720 | [260403-p84](./quick/260403-p84-add-pageup-pagedown-scrolling-to-markdow/) |
| 260405-27p | Fix folder appears empty after returning from markdown view | 2026-04-05 | d8a578e | [260405-27p](./quick/260405-27p-fix-folder-appears-empty-after-returning/) |
| 260405-oum | Add Defaults tab to display and edit .planning/config.json settings | 2026-04-06 | 963c8a4 | [260405-oum](./quick/260405-oum-add-defaults-tab-to-display-and-edit-pla/) |
| 260405-urb | Write a README.md for GitHub | 2026-04-06 | d0130d8 | [260405-urb](./quick/260405-urb-write-a-readme-md-for-github/) |
| 260509-k9m | Extend Defaults tab: intel/graphify keys, text input, x-to-clear | 2026-05-09 | 3d77fa5 | [260509-k9m](./quick/260509-k9m-extend-defaults-tab-text-input-and-clear/) |
| 260509-zh2 | Defaults layering (~/.gsd/defaults.json), six-section layout, [d] toggle, pipeline sub-stage drill-down | 2026-05-09 | 75e80c2 | [260509-zh2](./quick/260509-zh2-defaults-layering-six-sections-pipeline/) |
| 260509-t8m | Tab-to-switch into a Claude session via tmux (overview + Sessions tab) | 2026-05-09 | ed1f3b2 | [260509-t8m](./quick/260509-t8m-tab-to-switch-to-tmux-session/) |
| 260512-ecm | Detect SECURITY.md sub-phase artifact and render Security row in Plan sub-stages | 2026-05-12 | e17afe7 | [260512-ecm](./quick/260512-ecm-add-support-for-gsd-s-optional-secure-su/) |
| 260512-epe | Auto-detect and register GSD projects from active claude sessions | 2026-05-12 | b350b46 | [260512-epe](./quick/260512-epe-auto-detect-and-register-gsd-projects-fr/) |
| 260512-eyv | Suppress "Updated" status when project state is unchanged | 2026-05-12 | 4dcd271 | [260512-eyv](./quick/260512-eyv-suppress-updated-status-when-project-sta/) |
| 260512-fe6 | Add markdown document browser tab for .planning/ rooted at active phase | 2026-05-12 | 106cbcc | [260512-fe6](./quick/260512-fe6-add-markdown-document-browser-tab-for-pl/) |
| 260515-vyt | Detect UAT.md sub-phase artifact and surface in pipeline drill-down | 2026-05-15 | de91154 | [260515-vyt](./quick/260515-vyt-add-uat-md-sub-phase-detection-to-meta-ma/) |
| 260515-w3a | Detect SPEC.md and EVAL-REVIEW.md sub-phase artifacts | 2026-05-15 | a88d82c | [260515-w3a](./quick/260515-w3a-add-spec-and-eval-review-sub-phase-detect/) |
| 260722-emn | Catch up to GSD 1.8.0: README value prop + tier 1-3 state-reader/feature updates (10 plans, 5 waves) | 2026-07-22 | ca0ad3b | [260722-emn](./quick/260722-emn-catch-up-to-gsd-1-8-0-readme-value-propo/) |
| 18 | Bump direct deps: notify-debouncer-full 0.7, serde_yml 0.0.13 | 2026-07-22 | 60f028d | — |
| 19 | Add GitHub Actions release workflow: crates.io publish on version tags | 2026-07-22 | b174ecb | — |
| 260728-kfx | Dedupe phases in parse_roadmap_phases so summary-checklist + Phase Details roadmaps do not list every phase twice | 2026-07-28 | df64162 | [260728-kfx](./quick/260728-kfx-dedupe-phases-in-parse-roadmap-phases-so/) |
| 260729-vmp | Let the user opt a project in to driving from the Driver tab (`o` pushes the same confirmation the dashboard's `o` does; spawn seam unchanged) | 2026-07-30 | 70157bd | [260729-vmp](./quick/260729-vmp-let-the-user-opt-a-project-in-to-driving/) |

## Session Continuity

Last session: 2026-08-19
Stopped at: Phase 19 fully executed (8/8 plans, verification human_needed → deferred by user); autonomous run started at Phase 20
Resume file: none — resume with /gsd-autonomous --from 20 --to 22
Note: `.planning/phases/19-gitsafe-git-blast-radius-envelope/.continue-here.md` is a stale
mid-execution checkpoint (claims task 3 of 8); all 8 plans have summaries. Ignore it.
