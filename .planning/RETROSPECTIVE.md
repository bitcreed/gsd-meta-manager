# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — MVP

**Shipped:** 2026-03-26
**Phases:** 4 | **Plans:** 10 | **Commits:** 83

### What Was Built
- Rust TUI dashboard with color-coded project list, aggregate status bar, and vim-style navigation
- Live filesystem watching with 200ms debounce for auto-refresh on .planning/ changes
- Project detail view with phase breakdown, plan counts, status icons, and change tracking
- ASCII roadmap visualization with box-drawing pipeline widget
- Project creation flow with git init, modal input, hooks, and auto-registration
- Work enqueue system with QUEUE.md, tab-cyclable command suggestions

### What Worked
- Rust + ratatui stack delivered fast, portable single binary with no runtime dependencies
- TEA (The Elm Architecture) pattern kept TUI state predictable and testable
- Phase-by-phase GSD workflow matched well to incremental TUI feature building
- File watching via notify-debouncer-full was straightforward to integrate with tokio

### What Was Inefficient
- ROADMAP.md parser relies on markdown structure rather than disk truth — led to plan counting bugs
- STATE.md `completed_phases` counter creates stale state vs actual files on disk
- QUEUE.md is passive (GSD doesn't consume it) — users expect queued commands to be executable
- Some backlog directory names got malformed JSON slugs (999.1, 999.2)

### Patterns Established
- Atomic file writes via tempfile + rename for all state mutations (config, queue)
- `spawn_blocking` for IO-bound operations (git init, file discovery) to keep render loop responsive
- Channel-based architecture: crossterm events, notify events, and tick timer multiplexed via `tokio::select!`

### Key Lessons
1. Parse state from disk files, not markdown documents — markdown structure is fragile and accumulates parsing bugs
2. Phase completion should be inferred from PLAN/SUMMARY pairs, not a counter in STATE.md
3. Enqueue features need a clear execution path — passive reminders frustrate users who expect action

### Cost Observations
- Model profile: quality (opus throughout)
- 2-day build from greenfield to 22-requirement MVP
- Notable: research phase for tech stack paid off — zero mid-build stack pivots

---

## Milestone: v1.1 — Polish & Power Features

**Shipped:** 2026-03-27
**Phases:** 5 | **Plans:** 16

### What Was Built
- State reader accuracy with disk-based phase inference
- GSD integration badges (verified/inferred)
- Queue management CRUD, git history viewer, backlog browser
- Execution flow pipeline (D-R-P-E-V visualization)
- Claude session management

### What Worked
- Screen trait refactor (Phase 05) enabled adding 4 new tabs without explosion
- Disk-status inference proved more reliable than parsing STATE.md counters

### What Was Inefficient
- Some phases could have been parallelized (Queue + Git History were independent)

### Key Lessons
1. Refactor before feature expansion — Screen trait made 7-tab view possible
2. Disk truth beats document parsing for status detection

---

## Milestone: v1.2 — Housekeeping & Archive Browser

**Shipped:** 2026-04-01
**Phases:** 4 | **Plans:** 6

### What Was Built
- Zero-warning baseline (all dead_code resolved, clippy clean)
- Paused project detection (HANDOFF file → cyan badge on dashboard)
- Milestone Archive Browser (8th tab: 4-level drill-down, styled markdown, async loading)
- Queue execution design document (2 strategies, safety requirements, LLM-agnostic)

### What Worked
- Autonomous mode (`/gsd:autonomous`) ran 4 phases with minimal user intervention (only grey area approvals and one tech debt decision)
- Smart discuss with batch table proposals was fast — user accepted all recommended answers
- Phase 10 cleanup first prevented Phase 12 from needing to work around dead code annotations
- Research phase (13) produced actionable design document that can drive v1.3 planning directly

### What Was Inefficient
- Worktree merges occasionally caused binary/lib duplication conflicts (Phase 12 Wave 1)
- UI-SPEC checker's pixel-based spacing rules don't map well to terminal cell units — required revision loop for a false positive
- Phase 12 Plan 03 (visual UAT) was deferred — still needs manual testing

### Patterns Established
- `archive_file_name` cached for breadcrumb accuracy — can't rely on index lookups for mixed-level navigation
- Tab bar abbreviation pattern (Pipeline → Pipe, Sessions → Sess) for 80-column fit
- LLM-agnostic Executor trait pattern for future queue execution

### Key Lessons
1. Clean baseline before feature work pays off immediately (Phase 10 → Phase 12 dependency)
2. Documentation-only phases (research) work well in autonomous mode — no code conflicts
3. Worktree agents need explicit instructions about binary/lib crate structure to avoid duplicate module declarations
4. UI-SPEC checker rules need terminal-aware exceptions — pixel grid rules produce false positives for TUI apps

### Cost Observations
- Model profile: quality (opus for execution, sonnet for verification)
- 4 phases completed in a single autonomous session
- Notable: Phase 13 research used the GSD source directly — much more accurate than web-only research

---

## Milestone: v2.0 — Autonomous Orchestration

**Shipped:** 2026-10-29 (v2.0.0; interim releases v1.7.0-v1.15.0)
**Phases:** 20 (14-31 incl. 26.1, 28.1) | **Plans:** 195 | **Quick tasks:** 109 (+1 fast)

### What Was Built
- A sandboxed autonomous driver: bwrap + `systemd-run --user --scope` launcher with hard caps, egress proxy and no push credential inside; `driver push` outside the sandbox (Phases 28, 28.1)
- `driverctl` as single writer of run state and DIRECTION.md frontmatter (Phase 27); versioned master prompt and run templates (Phase 29); Driver tab steering through DIRECTION.md (Phase 30)
- TUI work that shipped independently: roadmap tab redesign (24), running agents and live wave view (25), UI fixes (14)

### What Worked
- Spikes before Rust: spikes 001/002 and the spike 003 re-run settled the safety model and driver behaviour cheaply, and the 2026-10-14 re-plan replaced two non-converging phases
- Interim point releases (v1.7.0-v1.15.0) kept shipping TUI value while the orchestration work was in flight
- Container pre-tag check closed the gap between a green local run and the CI publish job

### What Was Inefficient
- Phases 19 and 21 ran 13 gap rounds each without converging (every round found a new carrier class); capability removal made both moot
- Narrative deferred-items logs grew to ~300 audit hits that the acknowledge writer cannot anchor
- Review findings accumulated (123 open/deferred at close) faster than they were triaged

### Patterns Established
- Safety by capability removal and outside-enforced caps, not by parsing what the model asks to run
- Tool-written state only (single writer, exchange-verify splice); the model never writes its own status or counters
- Supersession as an explicit closure record rather than silently abandoning a phase

### Key Lessons
- When a guard needs a new rule every round, remove the capability instead
- Pre-register a spike's rubric before spending; post-hoc instrument fixes are redesign inputs, not verdict inputs
- Self-referential tests that read the live ROADMAP.md break at milestone close; read the archive (fast task 261029-fst)

### Cost Observations
- Spike 003: $7.81 of a $60 ceiling for the first attempt; Opus re-run in 26.1
- Notable: the two superseded gap loops were the largest single cost of the milestone

## Milestone: v2.3.0 — Multiplexer Backends

**Shipped:** 2026-11-10 (v2.3.0; v2.1.0 and v2.2.0 were interim releases cut without a milestone close)
**Phases:** 1 (33) | **Plans:** 2 | **Tasks:** 5 | **Quick tasks:** 1 (261110-qap)

### What Was Built
- `MultiplexerBackend` trait and static `REGISTRY` in `src/multiplexer/` replace the tmux-hardcoded `src/terminal_switch.rs`. tmux moved onto it unchanged. A herdr backend launches through one `layout.apply` socket request and focuses by session id, then pid, then a unique cwd. Precedence is innermost-wins. The fail-closed `ExecArgv` sink is shared (MUX-01..07, Phase 33)
- `gsd-meta-manager status --json` (schema v1) and a `--skill` agent-skill output (quick 261110-qap)

### What Worked
- Research facts about herdr (it exposes no tty; `layout.apply` is atomic) let three planned tasks be cut on evidence, with nothing added back
- Code review caught two real herdr focus mis-targets (WR-01 cwd tier guessing, WR-02 symlinked paths). UI review caught a raw backend error in the status line (UI-01), on a phase with no UI. Both gates earned their cost
- The milestone pass ran unattended and correctly cut nothing from a one-phase, user-requested milestone

### What Was Inefficient
- Review fixes landed after the verifier ran, so VERIFICATION.md went stale and had to be re-run before the close
- Two agents reported different full-suite counts (3755 and 3743) for the same tree. A re-run at HEAD gave 3755, and the 3743 run was never traced. A count is evidence only with its command and revision
- The v2.0 archived deferred-items logs still produce 241 audit hits that the acknowledge writer cannot anchor (table rows, or `no deferred item matched`)

### Patterns Established
- Backend seams start from an ecosystem scan. The trait is designed against every candidate tool, and only the ones needed now are implemented
- Unused dependencies are removed in the release dep pass, not in the fix that made them unused (musk cut M-0040, reinstated in 1f5f404)

### Key Lessons
- Re-run verification after review fixes that touch covered files, not only at phase end
- Record the exact command and revision next to any test count you hand off

### Cost Observations
- Phase 33 measured 1.91M tokens (+52.4M cache read) for 2 plans and 5 tasks, at the low end of this project's per-phase range (p25 1.75M)
- Musk net savings this milestone: p50 55k (basis harness, n=4). See `.planning/musk/RETRO-v2.3.0.md`

## Cross-Milestone Trends

### Process Evolution

| Milestone | Commits | Phases | Key Change |
|-----------|---------|--------|------------|
| v1.0 | 83 | 4 | Initial build — established TEA pattern, file watching, state reader |
| v1.1 | 79 | 5 | Screen trait refactor, 7-tab detail view, disk-based inference |
| v1.2 | 41 | 4 | Autonomous mode, archive browser, paused detection, queue execution design |
| v2.0 | ~2,300 | 20 | Sandboxed autonomous driver; spike-first re-plan; interim point releases |
| v2.3.0 | ~55 | 1 | Multiplexer backend seam + herdr; unattended single-phase milestone |

### Cumulative Quality

| Milestone | LOC | Tests | Key Metric |
|-----------|-----|-------|------------|
| v1.0 | 3,887 | 12 integration | Full requirement coverage (22/22) |
| v1.1 | 6,319 | 20 integration | Screen trait arch, 7 tabs |
| v1.2 | 7,630 | 25 integration | Zero warnings, 8 tabs, 9/9 requirements |
| v2.0 | ~198K (src+tests) | ~3,200 | Requirements 70/71 (1 accepted partial), 0 critical review findings |
| v2.3.0 | ~234K (src+tests) | 3,755 passed / 19 ignored | Requirements 7/7, review 0 critical / 3 warning / 5 info, all fixed |

### Top Lessons (Verified Across Milestones)

1. Infer state from disk truth, not parsed markdown counters
2. Refactor before feature expansion — pays off immediately in the next phase
3. Autonomous mode works best with well-specified success criteria and clean baselines
