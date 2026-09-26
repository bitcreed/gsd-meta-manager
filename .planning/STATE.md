---
gsd_state_version: "1.0"
milestone: v1.8.0
current_phase: 25
current_phase_name: Running Agents & Live Wave View
status: executing
stopped_at: Completed 25-05-PLAN.md
last_updated: "2026-09-26T03:39:29.395Z"
last_activity: 2026-09-25
last_activity_desc: Phase 25 execution started
state_head: bcf6938c74661d5611474f21c838a72d42e44db2
progress:
  total_phases: 10
  completed_phases: 6
  total_plans: 6
  completed_plans: 112
milestone_name: Roadmap Redesign
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-09-23)

**Core value:** See the state of every GSD project at a glance and act on any of them without leaving the TUI.
**Current focus:** Phase 25 — Running Agents & Live Wave View
Autonomous Orchestration Preview, cut after the release gate itself was repaired. The v2.0
**Autonomous Orchestration** milestone is NOT finished — v1.7.0/v1.7.1/v1.7.2 are interim releases
cut mid-milestone, at Phase 22. Work resumes at Phase 22 (container-execution-target).

## Current Position

Status: Ready to execute
  Phase 24, quick tasks 260922-hdh/hdi/hdj, 260923-lr8/lr9/lra/md1, 260924-drx and five debug
  fixes; README gained screenshots and a Codex section. `./scripts/pre-tag-check.sh --container
  v1.8.0` exit **0**, all five gates PASS, git banner MATCH, 49 suites / **2367 passed / 0 failed**
  / 15 ignored; `cargo publish --dry-run --locked` clean. Next: Phase 19 (halted — ask first).

Previous (v1.7.2):
  `--container` gate.** It supersedes and carries the contents of v1.7.0 and v1.7.1, NEITHER OF
  WHICH EVER PUBLISHED: for anyone installing from crates.io this is the jump from **1.6.0**.
  `Cargo.toml` moved 1.7.1 -> 1.7.2; `cargo update` relocked only this crate's own entry, with
  `generic-array` 0.14.7 (latest 0.14.9) and `unicode-width` 0.2.0 (latest 0.2.2) still held by
  upstream `=` pins. What made this cut different is quick task 260917-r4c: the publish job's
  `Test` step now runs `--no-fail-fast` (so a red run enumerates every failure instead of one per
  tag), provisions the GSD conformance oracle via `actions/setup-node@v4` +
  `scripts/install-conformance-oracle.sh`, and `scripts/pre-tag-check.sh` gained `--container`,
  which re-runs every gate inside an `ubuntu-latest` lookalike. Gate on this tree:
  `./scripts/pre-tag-check.sh --container v1.7.2` exit **0**, all five gates PASS, git banner
  MATCH at 2.55.0, 48 suites / **2121 passed / 0 failed** / 15 ignored. The bare local run is
  unchanged at exit 1 with gates 1/2/3/5 PASS and gate 4 failing on exactly one test, the
  environmental git-version witness (local git 2.53.0 vs constants derived against 2.55.0), which
  the container proves green on the runner's git.

Release history preserved (do not rewrite — this is the record):
  **v1.7.0 was tagged and pushed but NEVER PUBLISHED** — its publish job died at `cargo test` on
  the git-version witness. **v1.7.1 was tagged and pushed but NEVER PUBLISHED EITHER** — its
  publish job died at `cargo test` too, but on a DIFFERENT test,
  `tests/driver_router_conformance.rs::the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state`,
  which fails BY DESIGN when its oracle is absent and the `ubuntu-latest` runner had no GSD
  install. Fail-fast is what made each tag reveal exactly one failure; that is the cycle v1.7.2
  closes at the source.

Superseded detail from the v1.7.1 entry, kept verbatim:

Status: **v1.7.1 tagged and pushed (Release Gate Repair) — BUT IT DID NOT PUBLISH EITHER.**
  crates.io is still at **1.6.0**; neither 1.7.0 nor 1.7.1 exists there. v1.7.0's publish job died
  at `cargo test`; v1.7.1's publish job died at `cargo test` too, but on a DIFFERENT test —
  `tests/driver_router_conformance.rs::the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state`,
  which fails BY DESIGN when its oracle (`~/.claude/gsd-core/bin/gsd-tools.cjs` or a `gsd-tools` on
  PATH) is absent, and the `ubuntu-latest` runner has no GSD install. The escape hatch the test
  names itself is `GSD_META_MANAGER_ALLOW_MISSING_ORACLE=1`. **`scripts/pre-tag-check.sh` structurally
  cannot catch this**: the oracle IS installed on the developer machine, so the test passes locally
  and the gate is green on precisely the thing CI is red on. Next release must address that gap.
  The five fixes below all landed and the git-version witness DID go green on the runner
  (lib suite 1346 passed / 0 failed), so v1.7.1 is a strictly better tree than v1.7.0 —
  it is just not on crates.io. Nothing in non-test `src/` changed between the two — v1.7.1 is test, tooling and
  documentation only, which is what makes it a patch. Five quick tasks make it up: 260917-ii4
  (config-section constants re-derived against git 2.55.0, the runner's version — neither
  `INDIRECTION_SECTIONS` nor `REPARSED_COMMAND_SECTIONS` changed, and a git 2.54
  `alias.<name>.command` spelling falsified a doc justification, repaired in place), 260917-jdi
  (`scripts/pre-tag-check.sh`, a local dry run of the publish gate, with CLAUDE.md release step 3
  routed through it), 260917-k6y (`tests/driver_reattach.rs`: two spawn/write races closed by
  waiting on the artifact rather than on process liveness), 260917-lkg
  (`tests/envelope_tracer.rs`: the ETXTBSY exec race closed with a bounded ETXTBSY-only retry) and
  260917-nhc (`src/driver/run.rs`'s `proc_parse` test race closed with a bounded seqlock re-read).
  Pre-tag gate on the tagged tree: gates 1/2/3/5 PASS, gate 4 failing on exactly one test — the
  environmental git-version witness (local git 2.53.0 against constants derived against 2.55.0),
  green on the runner. 48 suites / 2120 passed / 1 failed / 15 ignored.

Carried forward from v1.7.0 (unchanged by this patch):
  The entire v2.0 driver stack (Phases 15-21: duplex stream-json transport, run journal, supervisor
  with detach/kill-switch/dry-run/opt-in gate, Driver tab with live watch and durable injection, the
  GITSAFE git/blast-radius envelope, the deterministic decision router with run bounds, and the LLM
  goal layer with prompt-injection hardening) is IN this release but is **hidden by default** behind
  the `GSDMM_EXPERIMENTAL_FEATURES` startup flag (quick 260917-fko) and the `drive` subcommand is
  hidden from `--help` (quick 260917-hc3). Nothing in that stack is reachable by a default install,
  which is why the release ships despite Phases 19/20/21 still reading "In Progress" on the ROADMAP
  (all plans executed; `/gsd-secure-phase 19` has not cleared, T-19-86/T-19-91 open at high).
  MSRV is 1.88 in this release — consumer-visible.

Active milestone (unchanged by the release, resume here):
Milestone: v2.0 Autonomous Orchestration (Phases 14-23) — 6 of 10 phases complete
Phase: 25 (Running Agents & Live Wave View) — EXECUTING
Plan: 6 of 6
  19-11 closed T-19-60 structurally (resolve the effective program by finding the first token
  whose BASENAME is in a closed GOVERNED_PROGRAMS set; NO wrapper-name list added, one DELETED).
  19-12 certifies that closure at the CLASS level rather than at the instance level:
  tests/envelope_wrapper_class.rs, one created file, 1181 lines, 11 tests, 0.2s.
  A hand-rolled fixed-seed LCG (seed 0x1912_C0DE_5EED_0060, no crate — T-19-SC holds) composes
  30 wrapper spellings chained 0-3 deep x 4 assignment prefixes x basename/absolute-path forms
  x 3 quoting styles x an optional sh -c / bash -lc payload layer, and asserts
  verdict(wrap(base)) == verdict(base) on exit code AND D-24 reason identifier, with the
  right-hand side MEASURED from the unwrapped command in the same run.
  MEASURED: 1680 refused cases / 773 distinct wrapper chains / 3 distinct D-24 reasons, and a
  paired allow corpus of 1080 permitted cases / 533 chains, each exit 0 with EMPTY stdout (D-32).
  12 wrapper names proved absent from the PRODUCTION LOGIC of policy.rs and hooks.rs (4 of them
  from the raw bytes too), plus an orchestrator-required POSITIVE control proving each
  include_str! file is the file the scanner thinks it is — both proved fail-first.
  T-19-74 pinned on both sides, including the command-line boundary where the residual begins.
  Gate: cargo test --no-fail-fast 1470 passed / 2 failed / 13 ignored; clippy -D warnings exit 0;
  --all-targets lint count unchanged. USE --no-fail-fast: a plain `cargo test` stops at
  driver_reattach and never reaches any envelope_* binary, reporting 1245/2/13 regardless of
  what was added. The 2 failures are the pre-existing driver_reattach pair (deferred-items.md).
Status: Ready to execute
  21-35 closes a producer/consumer wire-format mismatch: the producer emits the fused
  `--resume=<id>` (one argv element) while the consumer still parses only the split
  `["--resume", "<id>"]`, so TUI-resumed sessions became silently undetectable in /proc.
  The consumer learns BOTH forms (both are legitimate on the wire — sessions started
  outside the TUI still use split); the producer is NOT un-fused (round 11 measured that a
  `--` end-of-options separator deletes the resume feature outright).
  The round also adds the producer<->consumer round-trip control that spans the pair — no
  single-sided test could have caught this — and corrects, in place, the `read_session_id`
  doc comment that argued against a control there. That argument is right about the SECURITY
  property (it is a property of the sink) and is precisely what hid the FUNCTIONAL coupling.
  criterion 4 remains PRESENT_BEHAVIOUR_UNVERIFIED and permanently agent-unclosable — it needs
  a human with a live Claude subscription for the 10 #[ignore]d arms. 4/5 is the correct ceiling.
  Phase 19 human-judgement UAT items were deferred by explicit user decision on 2026-08-19, not resolved.
Last activity: 2026-09-25 — Phase 25 execution started
Note (260908-uqq): the D-06 "a capability refusal costs zero tokens and zero quota" guarantee no
longer holds unconditionally. It holds on the **eager arm** only (CLI announced inside the grace).
On the **late arm** — 2.1.266's measured shape — the grace releases the prompt first, so a refusal
aborts a turn that has already begun. Both arms still refuse. Do not re-broaden that claim.

## Deferred Verification

| Phase | State | Resume |
|-------|-------|--------|
| 19 | verification_deferred_human | /gsd-verify-work 19 |

Phase 19's `19-VERIFICATION.md` remains `status: human_needed` — 5/5 automated must-haves are
verified, and the four outstanding items are all reading judgements (does the pinned honesty
statement read as candour; are 19-08's composed proofs faithful decompositions; do the
residual-exposure disclosures read as admissions) plus the `driver_lock` one-off risk call.
Deferred by user request on 2026-08-19 so Phase 20 could start; **not** marked passed.

### Phase 22 planning — decision-coverage gate override (autonomous run — audit this)

`/gsd-plan-phase 22`'s step-13a decision-coverage gate returned
`{passed: false, reason: "could-not-parse", total: 16, covered: 0, uncovered: []}` and was
**overridden** by the orchestrator, with the human unavailable. It is a parser mismatch, not a
coverage gap: the gate's ID grammar reads `D-NN` / `D4-NN`, and `22-CONTEXT.md` uses the
three-segment `D-22-NN` form, so three bullets (D-22-08, D-22-13, D-22-15) were reported
unparseable and coverage counting was abandoned before it ran. Verified by direct grep instead:
all **19** decisions D-22-01…D-22-19 are cited in `22-0*-PLAN.md`, each at least twice
(D-22-19 sixteen times, D-22-18 twelve). No decision was dropped. Nothing in `22-CONTEXT.md` was
edited to satisfy the parser — it is a committed, locked discuss artifact. Re-surface at
verify-phase if the gate is fixed.

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

- Total plans completed: 44
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
| 24 | 7 | - | - |

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
| Phase 19 P09 | 15 min | 2 tasks | 2 files |
| Phase 19 P10 | 20 min | 3 tasks | 1 files |
| Phase 19 P11 | 24 min | 3 tasks | 4 files |
| Phase 19 P12 | 41 min | 3 tasks | 1 files |
| Phase 19 P13 | 50m | 3 tasks | 6 files |
| Phase 19 P14 | 1h | 3 tasks | 6 files |
| Phase 19 P15 | ~1h | 2 tasks | 7 files |
| Phase 19 P16 | 1 session | 3 tasks | 5 files |
| Phase 19 P17 | ~1 session | 3 tasks | 6 files |
| Phase 19 P18 | 75m | 3 tasks | 4 files |
| Phase 19 P20 | 1 session | 3 tasks | 4 files |
| Phase 19 P21 | 1 session | 3 tasks | 5 files |
| Phase 19 P22 | 1h | 3 tasks | 4 files |
| Phase 19 P24 | 2h | 3 tasks | 4 files |
| Phase 19 P25 | ~3h | 4 tasks | 6 files |
| Phase 19 P26 | one session | 3 tasks | 5 files |
| Phase 19 P27 | one session | 4 tasks | 7 files |
| Phase 19 P28 | 1 session | 3 tasks | 4 files |
| Phase 19 P29 | one session | 4 tasks | 11 files |
| Phase 19 P30 | 1 session | 3 tasks | 4 files |
| Phase 19 P31 | one session | 4 tasks | 6 files |
| Phase 19 P33 | one session | 5 tasks | 9 files |
| Phase 24 P01 | 12 min | 3 tasks | 9 files |
| Phase 24 P02 | 14 min | 3 tasks | 1 files |
| Phase 24 P03 | 9min | 2 tasks | 3 files |
| Phase 24 P04 | 18 min | 3 tasks | 2 files |
| Phase 24 P05 | 12 min | 3 tasks | 3 files |
| Phase 24 P06 | 16 min | 3 tasks | 5 files |
| Phase 24 P07 | 13min | 3 tasks | 7 files |
| Phase 25 P01 | 15 min | 3 tasks | 8 files |
| Phase 25 P02 | 11 min | 3 tasks | 3 files |
| Phase 25 P03 | 11 min | 3 tasks | 6 files |
| Phase 25 P04 | 8 min | 3 tasks | 7 files |
| Phase 25 P05 | 13 min | 3 tasks | 7 files |

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
- [Phase 19]: T-19-60 closed by resolving the effective program STRUCTURALLY: consume leading NAME=VALUE assignment words by the shell grammar, then find the first token whose BASENAME is in a closed GOVERNED_PROGRAMS set. No wrapper-name list is added anywhere — one is DELETED (NESTED_SHELLS) (19-11) — The set of things that can precede a program is open and unlistable (env, timeout, nohup, stdbuf, setsid, doas, busybox env, ...), so a list plus one row per name is green on the day it lands and silent on the seventh wrapper. The set of programs the envelope GOVERNS is closed and already defined by classify_git and pr_command_label (D-08). made-up-wrapper-9000 resolving identically to env is the proof the fix is over the class rather than over a list.
- [Phase 19]: GIT_CONFIG_COUNT=0 git push --force parks under hook_bypass_blocked, not force_push_blocked: the assignment is refused on its own account before resolution reaches the push (19-11) — One token does two things: it hides the command from layer 2 AND it stops core.hooksPath being injected, so no pre-push hook runs (D-09). Parking it under the reason --no-verify and core.hooksPath already use is where a later reader greps for a disarmed enforcement layer. ENVELOPE_ENV_KEYS is drift-pinned against cred::build_env_in, and the pin found GIT_SSH_COMMAND on its first run — a key the plan list omitted and whose reassignment would put the user ssh agent back within the run reach.
- [Phase 19]: The alphabet-absence control asserts wrapper names absent from PRODUCTION CODE (comments and the #[cfg(test)] module stripped), not from raw file bytes (19-12) — 8 of the 12 designated names appear in policy.rs raw text: 7 in the doc comments of GOVERNED_PROGRAMS and resolve_program, where they are listed precisely to record that the set of things which can precede a program is open and must NOT be enumerated, and made-up-wrapper-9000 4x in the unit-test module as 19-11's own class-level fixture. A literal raw-bytes reading would be RED today for the opposite of the reason the control exists. Stripping is the same fact 19-11's audit measured with grep -v '^\s*//'. Strengthened rather than relaxed: the 4 raw-absent names (unshare, firejail, torsocks, catchsegv) are additionally asserted against unprocessed bytes.
- [Phase 19]: Every absence control needs a POSITIVE control beside it: include_str! fails the build on a MISSING path but not on a WRONG-BUT-EXISTING one (19-12) — Point include_str! at a different source file, or at the same file twice, and every absence assertion passes having certified nothing. An absence assertion cannot distinguish "the name is not in this file" from "this is not the file I think it is". Each included file is now asserted to contain an anchor unique to it (fn resolve_program for policy.rs, fn classify_segments( for hooks.rs), absent from the other, present after stripping, ordered BEFORE the absence assertions. Proved fail-first by repointing HOOKS_SOURCE at policy.rs.
- [Phase 19]: X=git; env $X push --force is PERMITTED and is pinned as such: resolve_program step 7 closes the SAME-command-line binding, and a semicolon makes it a different command line (19-12) — The plan required the semicolon form to be REFUSED; measured exit 0. The plan's own rationale and resolve_program's doc both say step 7 refuses a binding "in the same command line", so the semicolon form is the T-19-74 residual by definition rather than a bound on it. Bound 2 is pinned as X=git env $X push --force (no semicolon, refused under envelope_assertion_failed) AND the semicolon form is pinned permitted in a dedicated boundary test, so the residual's edge cannot move outward OR inward unnoticed.
- [Phase 19]: Any phase-19 suite number must come from cargo test --no-fail-fast (19-12) — cargo test stops at the first failing test BINARY. driver_reattach fails (documented pre-existing pair) and envelope_* sorts after driver_*, so a plain run never executes any envelope test and reports 1245/2/13 no matter what was added. With --no-fail-fast the true total after 19-12 is 1470 passed / 2 failed / 13 ignored.
- [Phase 19]: T-19-60 is closed for the wrapper-operand sub-class only; T-19-86 registered, pinned and deferred
- [Phase 19]: /gsd-secure-phase 19 is NOT cleared by plan 19-13 alone — T-19-86 and T-19-87 remain open
- [Phase 19]: Rule A: the classifier decision region — every index reported by the scan the classifier itself runs, never a second scan
- [Phase 19]: T-19-87's two severed-prefix rows left RED on purpose, so Rule A and Rule B are each shown separately load-bearing
- [Phase 19]: T-19-91 measured, pinned and registered OPEN rather than closed — round discipline, and reflog/symbolic-ref have no second carrier
- [Phase 19]: Rule B is POSITIONAL — the substring/prefix/suffix/length formulation against ENVELOPE_ENV_KEYS stays withdrawn on two measurements (evadable by moving the split point; refuses every uppercase assignment)
- [Phase 19]: The word-splitting OPENER is excluded, which is what keeps command substitutions classified and ordinary shell running
- [Phase 19]: split_segments is DEFINED OVER split_segments_with_heads so a decision region comes from the same scan the classifier runs, never a second scan
- [Phase 19]: The false T-19-87 test was DELETED with a tombstone rather than re-worded; its two rows re-homed with corrected reason identifiers
- [Phase 19]: The bare-brace Rule B row is spelled without its binding, because the bound spelling is already refused under hook_bypass_blocked and would be a control that could not fail on its class
- [Phase 19]: 19-16: comma-list brace expansions produce N words, so only range spellings of the concatenated splice class assemble a real force push; comma spellings are corpus COVERAGE, labelled as such
- [Phase 19]: 19-16: git push {--force,origin} main is cwd-dependent — exit 0 in the envelope namespace, exit 2 outside it under an unrelated arm — so it is pinned with the repository passed explicitly
- [Phase 19]: The rule is INVERTED: a decision word must be provably LITERAL, so anything not provably literal is unresolvable and refuses
- [Phase 19]: Clause 2(b) reads what a splice PRODUCES over the WHOLE WORD, quote-removed, never what its alternatives are called
- [Phase 19]: A nested { inside an alternative is a fail-closed UNENUMERABLE trigger, not a recursion
- [Phase 19]: T-19-93 is closed in the tokenizer's literal-brace branch with neither forge scan changed
- [Phase 19]: The four FORGE rows assert a RESTORED COUNT, not a refusal — the SAFE-06 cap is bypassed UNCOUNTED rather than exceeded (19-18)
- [Phase 19]: Three rows whose post-fix verdict 19-18 cannot derive are RECORDED and never asserted: >$F, >*.log and {v}> (19-18)
- [Phase 19]: T-19-17r's bookkeeping gap recorded OUTSTANDING; no AR-19-13 row added and no acceptance made (19-18)
- [Phase 19]: 19-20: the callee's grammar is a THIRD corpus axis, not more entries in either shell axis — both existing axes are axes of bash and audit 6 verified that boundary closed
- [Phase 19]: 19-20: assert the POST-fix verdict wherever derivable so the row is red now and green after — no pin for the next plan to move, and no replacement exception needed
- [Phase 19]: 19-20: the anti-vacuity ratio floor is REPLACED by absolute per-file byte floors (40k/20k), not lowered — 428 bytes of comment headroom made it a documentation budget
- [Phase 19]: 19-21: an unestablished git verb slot is a refusal, not the next non-`-` word — the fail-closed treatment resolve_program's wrapper axis already had
- [Phase 19]: 19-21: git's option grammar is pinned against the installed git binary in a TEST, never consulted from the guard's path
- [Phase 19]: 19-22: the corpus for git's CONFIG RESOLUTION is written and RED before either rule exists — 9 failing names are 19-23's handoff contract
- [Phase 19]: 19-22: T-19-103 reproducers must sit on LAYER-2-PERMITTED bases; the five --force compositions are controls, fenced mechanically
- [Phase 19]: 19-22: a dotless -c key stays CONFINED — refusing it would turn round 7's whole generative property red behind CALLEE_KNOWN_LEADING_PREFIX
- [Phase 19]: 19-22: CONFIG_RESOLUTION_CLASSES is a FOURTH axis beside three byte-identical command-line axes — what the verb runs under, not the command line
- [Phase 19]: 19-22: GIT_CONFIG_NOSYSTEM recorded as a measured DEFEAT with an INERT harm — never a new threat ID, never a bypass
- [Phase 19]: A SIXTH class on the EXISTING config-resolution axis, not a fifth axis — the region moved, not the axis
- [Phase 19]: The shell-alias carve-out is git's own one-byte rule, fenced mechanically rather than by care
- [Phase 19]: T-19-108 will close only AS SCOPED; audit 7's !-bodied destructive pair is T-19-86 and stays open at high
- [Phase 19]: Option (b) adopted for T-19-108: option (a) rejected on three MEASURED grounds (second tokenizer, depth-3 recursion, carriers with no readable body)
- [Phase 19]: The re-parse value test reads the CARRIER as well as the first byte — measured, --config-env=alias.q='!EVIL' resolves /INCLUDE_WINS
- [Phase 19]: Region 2 gates on !is_read plus a VALUE WORD because is_write is blind to a dash-leading value
- [Phase 19]: T-19-110 registered OPEN and NOT fixed: widening is_write moves verdicts with no corpus able to fail on them
- [Phase 19]: 19-26: CONTROL_CARRIER_CLASSES is a genuine FIFTH axis — no existing axis property can be satisfied by a command reaching no governed program
- [Phase 19]: 19-26: C-15 (~/.config/gsd-meta-manager/config.json) is a DISTINCT route to SAFE-06 and gets control (e) — no rule; pr_cap_* deliberately NOT clamped
- [Phase 19]: 19-26: T-19-111 is registered SEPARATELY from T-19-86 and must be moved OUT of it — five mis-attribution sites enumerated
- [Phase 19]: 19-27: the carrier-operand rule — a segment naming an ABSOLUTE LITERAL path under envelope_dir_in(root, alias) is refused, raised ONCE before the resolution match; it fails OPEN in four named directions handed to no control
- [Phase 19]: 19-27: T-19-111 moved OUT of T-19-86 at all five sites, with no rule and no acceptance written for it
- [Phase 19]: 19-27: SECTION_ENVELOPE's Guaranteed cap clause repaired to a location fact making no completeness claim — C-15 is a fifth route to SAFE-06 and the deferred option (b) a sixth
- [Phase 19]: The redirection-target channel is a SEGMENT-borne fact, never a token in the stream — pinned falsifiable by segment-count assertions (19-28)
- [Phase 19]: The binary carrier C-10 is an EXACT-PATH boundary, not a directory prefix, because its directory is shared (19-28)
- [Phase 19]: The ledger size bound is DEADLINE-derived and must not be cap-derived, because the caps are unclamped (C-15) (19-28)
- [Phase 19]: T-19-115's tilde/glob/brace spellings get NO rule; the residue's arithmetic is corrected from four directions to seven (19-28)
- [Phase 19]: 19-29: the redirection target rides on the Segment, never as a token in the stream — proved by the SEGMENT-COUNT pins rather than argued
- [Phase 19]: 19-29: two protected paths, two boundary KINDS — the envelope directory as a PREFIX, the guard's own binary as an EXACT PATH
- [Phase 19]: 19-29: the credential control's criterion is 'git credential fill' failing closed, never 'git config --get-all', which still lists the helper
- [Phase 19]: 19-29: the ledger size bound is DEADLINE-derived and fails CLOSED; a cap-derived bound would be one the agent steers
- [Phase 19]: 19-30: the option-attachment boundary is /-anchored substrings of a LITERAL word, never a list of attachment characters (D-08 one level over)
- [Phase 19]: 19-30: containment is of the NORMALISED STRING, not the index — lexical_absolute_components strips leading ./ before testing for /
- [Phase 19]: The interior-path boundary is /-anchored substrings of a LITERAL word — not =, not a character list; containment is of the NORMALISED STRING, not of the index
- [Phase 19]: The residue is a CONDITION with NO count, handed to no pin, schedule or version witness
- [Phase 19]: credential.helper is denied by SECTION + FINAL COMPONENT, never a substring test
- [Phase 19]: The ledger KIND check goes inside the existing Ok arm; the link-following stat is kept
- [Phase 19]: cred.rs names the rule that acts rather than claiming a layer governs — prefer a claim that stays true to one that must be maintained
- [Phase 19]: T-19-122 NARROWED by a THIRD word class: a SECOND Segment field carrying LITERAL NON-PATHNAME redirection targets, chained at the ONE existing reading site. Design decided by which fenced pins each option turns RED.
- [Phase 19]: T-19-123 NARROWED by an ANCESTOR clause beside the PREFIX one, bounded at the envelope ROOT on ownership grounds. Grows the protected set by exactly one path; the residue above the root is registered, disclosed and UNACCEPTED.
- [Phase 19]: T-19-124 NARROWED: the candidate scan's work is now LINEAR in the word's length via suffix folds with structure sharing, with a fail-closed CANDIDATE_SCAN_WORK_CEILING a test DRIVES. Task 3 was NOT severed.
- [Phase 19]: Both authorised cross-fence pin moves performed with each reasoning REWRITTEN under WR-02 rather than deleted, and a STOP row added beside each.
- [Phase 19]: Audit 12's finish-line sentence is deliberately NOT written by plan 19-33; that is audit 13's judgement. /gsd-secure-phase 19 is NOT cleared.
- [Phase 24]: ROADMAP `Build phase` headings live in ProjectState.planned_phases, never phases — router/frontier stay GSD-conformant
- [Phase 24]: Roadmap list is a pure model (git-log lanes, transitive reduction with (implied via N), milestone bands with folds); cursor targets are keys, never row indices; Parallel = same wave
- [Phase 24]: RoadmapView splits side-by-side at >=100 cols, stacked below; stacked detail pane shrinks 9->6 rows and shortens the goal before edge lines
- [Phase 24]: Final tabs 1:Roadmap 2:Phases 3:Backlog 4:Git 5:Queue 6:Sess 7:Cfg 8:Docs + D:Drive; Archive is Docs › Milestones (`m`), switch_to_sub_view is the one arrival rule [INFERRED — audit]
- [Phase 24]: Roadmap footer left at 91/87 cols — trimming hints is a scope change, deferred
- [Phase 25]: 25-01: agent git counts live in worktrees::worktree_counts so every src/agents git call stays in worktrees.rs [inferred]
- [Phase 25]: 25-01: no-linked-worktree prefilter reports main_worktree=project_root, base_sha=None [inferred]
- [Phase 25]: 25-01: non-z porcelain fallback also decodes octal byte escapes (non-ASCII paths) [inferred]
- [Phase 25]: 25-02: an unparseable Claude meta is no data for its row; a mistyped field loses only itself [inferred]
- [Phase 25]: 25-02: read_meta_capped refuses symlinks and non-regular files (transcript redirect, FIFO hang) [inferred]
- [Phase 25]: 25-02: the MAX_AGENT_AGE_SECS session bound covers the whole second pass; the per-id worktree lookup is unbounded [inferred]
- [Phase 25]: 25-02: a live meta whose worktreePath names an already-enriched worktree attaches as a child [inferred]
- [Phase 25]: 25-03: commit-scope attribution routes through worktrees::commit_subjects so every src/agents git call stays in worktrees.rs [inferred]
- [Phase 25]: 25-03: worktree SUMMARY check excludes FIX/GAPCLOSURE summaries, matching main's Pass 2 pairing [inferred]
- [Phase 25]: 25-03: ladder drops P13 first (RESEARCH A3, deviates from D-C14 drop-from-right); Unknown-only plans read queued [inferred]
- [Phase 25]: 25-04: agent_views is pruned by per-scan wholesale replacement plus the registered-alias filter, not by prune_driver_maps [inferred]
- [Phase 25]: 25-04: DASHBOARD_HIGHLIGHT_SYMBOL is shared by dashboard_table and status_column_cells so the fit check cannot drift [inferred]
- [Phase 25]: 25-05: Sessions › Agents shares Sessions' tab index; m toggles through switch_to_sub_view; Enter/n inert on Agents [inferred details in SUMMARY]
- [Phase 25]: 25-05: agent_list_len is the one line count both render and scroll keys clamp against; ages floor-rounded from scanned_at
- [Phase 25]: 25-05: Docs and Sessions strips share two_sub_tab_strip; Agents footer adds only [m] sessions (prefix already has [j/k]) [inferred]

### Roadmap Evolution

- Phase 25 added (2026-09-25): Running Agents & Live Wave View — TUI-only, read-only view of running GSD agents and executor wave progress (worktrees + `~/.claude/projects/.../subagents/*.meta.json` + transcript mtime + PLAN `wave:`); seed evidence from ttbook in ROADMAP Phase 25 details; landed in v2.0 after the v1.8.0 interim release [inferred — no new milestone, same pattern as Phase 24]; Phase 19/21 left untouched (halted by user decision)
- Phase 24 added (2026-09-23): Roadmap tab redesign and detail-tab consolidation — TUI-only, no dependency on Phases 15-23; Phase 21 left untouched (stopped by user decision)

### Pending Todos

- [2026-07-29] [ui] Badge glyphs may misalign by one cell across terminals — [todo file](.planning/todos/pending/2026-07-29-badge-glyph-display-width-alignment.md)
- [2026-07-29] [ui] Driver tab renders blank pipeline row at terminal heights 8-13 — [todo file](.planning/todos/pending/2026-07-29-driver-tab-layout-at-medium-terminal-heights.md)
- [2026-07-29] [ui] Invalidate browser file cache after $EDITOR exits — [todo file](.planning/todos/pending/2026-07-29-invalidate-browser-cache-after-editor-exit.md)
- [2026-08-18] [ui] Surface destructive confirmations in a modal popup with Yes/No buttons — [todo file](.planning/todos/pending/2026-08-18-surface-destructive-confirmations-in-a-modal-popup-with-yes.md)
- [2026-08-19] [driver] The verify-work gate policy must be configurable — skip | defer | auto-validate — [todo file](.planning/todos/pending/2026-08-19-verify-work-gate-policy-configurable.md)
- [2026-09-15] [ui] Make queue input truly multi-line — [todo file](.planning/todos/pending/2026-09-15-make-queue-input-truly-multi-line.md) — Needs TBD — this todo's deliverable is a **UX proposal**, not a fix. Whoever picks.
- [2026-09-22] [driver] "Codex runtime: remainder after the MVP slice (260922-hdj) [AUDIT]" — [todo file](.planning/todos/pending/2026-09-22-codex-runtime-remainder-after-mvp.md)
- [2026-09-23] [ui] Add a global settings editor with unambiguous scope — [todo file](.planning/todos/pending/2026-09-23-add-a-global-settings-editor-with-unambiguous-scope.md)
- [2026-09-24] [ui] "Backlog tab: Enter-to-focus scrollable content pane + edit opens ROADMAP.md section" — [todo file](.planning/todos/pending/2026-09-24-backlog-tab-enter-to-focus-scrollable-content-pane-edit-open.md)

### Blockers/Concerns

- `--bare` is slated to become the `-p` default and is incompatible with subscription
  auth; Phase 15 ships a version gate and a regression guard against it

- Phase 13's queue-execution design self-dated "valid until 2026-04-30"; re-verify GSD's
  autonomous-mode / checkpoint contract during Phase 20 research

- T-19-86 (governed program's own operand names a governed command) and T-19-87 (${VAR} fragments a command past the splitter) are open, pinned and deferred to a round-4 plan; /gsd-secure-phase 19 cannot clear until they are closed
- T-19-97/T-19-98/T-19-99 open — corpus RED at 1605 (passed+failed); 19-19 writes the rule. /gsd-secure-phase 19 NOT cleared: T-19-86 and T-19-91 remain open at high
- 19-21: tests/envelope_callee_grammar.rs pins `git --super-prefix x push --force origin main` at force_push_blocked while pinning `git --super-prefix x status` at envelope_assertion_failed — identical leading tokens, so no rule obeying 19-21's prohibitions satisfies both. Row left RED. Both refuse at exit 2; only the identifier differs. **RESOLVED 2026-09-04 (989f21a)** — the 19-21 executor's analysis was verified by measurement against the built binary (fresh envelope root per row, root walked after) and HELD: both spellings refuse at exit 2 with an empty walk, `--super-prefix` is absent from GIT_GLOBAL_VALUE_OPTS after 19-21, and git 2.43.0 rejects it bare, separate and attached. `force_push_blocked` was a PRE-fix observation mislabelled as post-fix. Row 937 corrected to envelope_assertion_failed; exit code and empty walk still asserted; no `src/` change and no second reading site. All thirteen envelope_* binaries green; passed+failed = 1639, 0 failures. **This does NOT clear `/gsd-secure-phase 19`** — T-19-86 and T-19-91 remain OPEN at high, T-19-17r stays OUTSTANDING (no AR-19-13, not accepted), and only the WRAPPER-OPERAND sub-class of T-19-60 is closed.
- T-19-110 (NEW, high): scan_config under-reads a dash-leading config value, so git config core.hooksPath -c and -- exit 0 while /dev/null and - are refused. Found by 19-25, deliberately not fixed — needs a RED corpus first.
- 19-29 must add tests/envelope_control_carrier.rs to its files_modified: its direction_i_a_redirection_target_is_not_an_operand_and_stays_permitted pins three rows PERMITTED that 19-29's redirection rule refuses (19-28 finding)

- [Phase 24] Advisory, not blocking: 24-REVIEW.md has 5 warnings / 6 info open — WR-01 colliding BandKeys deadlock j/k, WR-02 `declared_phase_count` u32 overflow on third-party input (roadmap_md.rs:898, debug-build panic), WR-03 build-phase range includes its own id (spurious cycles), WR-04 width measured in chars (wide glyphs overflow columns), WR-05 Roadmap cursor does not write back to Phases selection. Fix via `/gsd-code-review 24 --fix` or a quick task.
- [Phase 24] Roadmap footer (91/87 cols) clips `[?]help` at 80 cols.

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
| 260828-15f | Round-scope round-13 finding ids in `21-REVIEW.md` (`R13-` prefix) so no bare id collides with a historical phase-21 finding | 2026-08-28 | 2e44f57 | [260828-15f](./quick/260828-15f-disambiguate-round-13-finding-ids-in-pha/) |
| 260908-uqq | Fix the CLI 2.1.266 startup deadlock — release the prompt on a `prompt_release_grace` instead of waiting forever for a `system/init` that arrives only after stdin; stop labelling a pre-gate stall `spawn_failed`; put the terminal reason in `journal.jsonl`. The D-06 zero-token refusal claim is now scoped to the eager arm only, corrected at 13 sites | 2026-09-08 | 114de68, 2fb6592 | [260908-uqq](./quick/260908-uqq-fix-driver-startup-deadlock-on-cli-2-1-2/) |
| 260908-w0d | Open sessions where the user actually is: inside tmux both launch sites now open a new window on the running server, and the GUI fallback discovers the desktop's real default terminal (`xdg-terminal-exec` → `x-terminal-emulator` → candidate list) instead of taking the first hardcoded name that happens to be installed. `$TERMINAL` demoted below tmux (D-01). Separator table extended with the measured `ptyxis`/`xdg-terminal-exec` → `--` and `x-terminal-emulator` → `-e` rows; all three source-derived pins updated to keep guarding, and pin 3's previously vacuous assertion replaced | 2026-09-08 | 46318ea, 05b3592, aa457c2 | [260908-w0d](./quick/260908-w0d-prefer-tmux-for-opening-sessions-discove/) |
| 260909-s0n | for each config option provide some help text that explains the option and the choices. | 2026-09-09 | ae76ce9 | [260909-s0n-for-each-config-option-provide-some-help](./quick/260909-s0n-for-each-config-option-provide-some-help/) |
| 260910-uej | A frontmatter block that libyaml rejects for a colon-space inside an unquoted plain scalar now gets ONE conservative in-memory quote-in-place repair and a single reparse, so sentriq's 1270-byte `stopped_at` reads instead of blanking the project. Recovery is a distinct fourth outcome (`FrontmatterOutcome::Recovered`), never a silent success: the dashboard cell is prefixed `~ ` and the detail pane says the file on disk is unchanged. A still-unreadable file now carries the fault position as FILE-relative numbers — `! STATE.md unreadable (line 7)`, detail `(line 7, column 218)` — numbers only, no parser message text, per `src/driver/untrusted.rs`. Verified: `passed` 6/6, +18 tests | 2026-09-10 | 109192f, 4930e8a, 6e1911e | [260910-uej](./quick/260910-uej-state-md-frontmatter-recovery-pass-and-a/) |
| 260915-f4n | HALTED at Task 1 by its own decision rule: `Cargo.toml` already had `rust-version = "1.87"` (landed in 41a7b3f — the titled Cargo.toml bump was a no-op), so the plan measured the floor for real instead and found it FALSE — `cargo +1.87 check --all-targets --locked` and the lib/bins-only escape hatch both exit 101; only `+1.88` passes. Root cause: `ratatui` 0.30.2 and `icu_properties` 2.3.0 (both direct deps, caret reqs, locked via ordinary `cargo update`) now declare `rust-version` 1.88. No repo file changed — raising the floor to 1.88 invalidates 9 documentation/provenance sites and is a human scope decision, not a quick-task edit; the CI `msrv` gate (Task 2, fully specified) was not written pending that decision | 2026-09-15 | — | [260915-f4n](./quick/260915-f4n-bump-msrv-from-1-85-to-1-87-in-cargo-tom/) |
| 260915-hsh | Update all Cargo dependencies to latest, including major (semver-breaking) bumps. 64 packages relocked to latest semver-compatible versions; two genuine majors landed with zero source changes — `dirs` 6.0.0 → 7.0.0 (publisher continuity proved across the repo's move to Codeberg: `soc` is still sole owner of 5.0.0/5.0.1/6.0.0/7.0.0) and `process-wrap` 9.1.0 → 10.0.0 (10.0's removed panicking accessors `inner_child`/`inner_child_mut`/`into_inner_child` are never called here; `stdin`/`stdout`/`stderr` byte-identical, `ProcessGroupChild::signal`/`start_kill`/`kill`/`wait` unchanged, so the SIGTERM→grace→SIGKILL kill switch is preserved by the dep itself). `notify` held at 8.2.0 and `notify-debouncer-full` at 0.7.0 — 9.0.0-rc.5 / 0.8.0-rc.2 are still release candidates, so the CLAUDE.md "8.x not 9.0 rc" note stands and 0 rc/pre-release strings are in the lockfile. `generic-array` 0.14.7 and `unicode-width` 0.2.0 stay behind latest under upstream `=` pins (the latter via ratatui 0.29.0, reached through tui-textarea 0.7.0; clears when tui-textarea ships for ratatui 0.30). MSRV floor re-measured at **1.88.0** — unchanged, so no MSRV site moved, though its claimants shifted (`darling` 0.24.1, `ignore`, `instability` joined ratatui/time/icu). Gates: build 0, clippy -D warnings 0, tests 2005 passed / 1 failed vs a measured 2002 / 4 baseline — final failing set is a strict subset, the remainder being the environmental git-version-constants test (installed git 2.53.0) | 2026-09-15 | 6d1fdcd, 8f8b36a, bd4d1d8 | [260915-hsh](./quick/260915-hsh-update-all-cargo-dependencies-to-latest-/) |
| 260916-vqw | Sync gsd-core config and expose new options | 2026-09-17 | 26cbef715bdb3c0982a09a7bbe4e3a884355e6c1 | /home/blk/projects/rust/gsd-meta-manager/.planning/quick/260916-vqw-sync-gsd-core-config-and-expose-new-options-area-config-seve |
| 260916-vqx | Show plan token estimate and actual counts | 2026-09-17 | f330b9a | /home/blk/projects/rust/gsd-meta-manager/.planning/quick/260916-vqx-show-plan-token-estimate-and-actual-counts-area-ui-severity |
| 260916-vr0 | Backlog tab shows empty despite non-zero count on overview | 2026-09-17 | f1cd6e9 | /home/blk/projects/rust/gsd-meta-manager/.planning/quick/260916-vr0-backlog-tab-shows-empty-despite-non-zero-count-on-overview-a |
| 260916-vr1 | Git view - show co-author model and full commit message on Enter | 2026-09-17 | 4a8094e | /home/blk/projects/rust/gsd-meta-manager/.planning/quick/260916-vr1-git-view-show-co-author-model-and-full-commit-message-on-ent |
| 260916-vqy | Visualize execution waves per phase in roadmap | 2026-09-17 | cf676b2 | /home/blk/projects/rust/gsd-meta-manager/.planning/quick/260916-vqy-visualize-execution-waves-per-phase-in-roadmap-area-ui-sever |
| 260916-vqz | Add b keybinding to open backlog from main screen | 2026-09-17 | d256191 | /home/blk/projects/rust/gsd-meta-manager/.planning/quick/260916-vqz-add-b-keybinding-to-open-backlog-from-main-screen-area-ui-se |
| 260917-fko | Gate driver/autonomous features behind a `GSDMM_EXPERIMENTAL_FEATURES` startup flag and mark them EXPERIMENTAL in the TUI. One env read at startup (`src/app.rs:522`), one pure parse helper (`src/experimental.rs` — truthy set `1`/`true`/`yes`/`on`, ASCII-case-insensitive and whitespace-trimmed; unset, empty, `0`/`false`/`no`/`off` and anything unrecognised are OFF), one `AppContext.experimental` field, and 28 gate points threaded from it. Flag off = the driver does not exist in the TUI: ten-tab bar (widths re-derived 107→96 full, 77→69 compact), no `Shift+D`, no `D` in the shared footer hint, `sub_view_from_index(10)` and a stored `Driver` sub-view both coerce to `PhaseList`, no dashboard `r`/`x`/`o`, no driven badge, no attention-first driver float, and zero driver rows or headings in help. Flag on = the Driver pane title and the help Driver heading carry the literal `EXPERIMENTAL`. `driver_opt_in` and `driver_max_concurrent` untouched in both states (pinned by test); the `drive` CLI subcommand deliberately NOT gated because the TUI respawns itself as `current_exe() drive` and env propagation through the envelope scrub is not guaranteed. +31 tests, none deleted; gates build 0, clippy `-D warnings` 0, tests 2118 passed / 1 failed vs a 2087/1 baseline — the one failure being the environmental git-version-constants test (installed git 2.53) | 2026-09-17 | b867a6d, 509063a, 1a63521, be2473c, 83aa7a1 | [260917-fko](./quick/260917-fko-gate-driver-autonomous-features-behind-a/) |
| 260917-hc3 | Hide the `drive` subcommand from `--help` while keeping it executable for the TUI self-respawn path. One clap attribute — `#[command(hide = true)]` on `Commands::Drive` (`src/cli.rs:68`) — removes the entry from the rendered help and from nothing else: the parser, `src/main.rs`'s dispatch match, `src/driver/spawn.rs::drive_argv`'s literal `"drive"` argv token and `spawn_detached`'s `current_exe()` exec are all untouched, as is the envelope env scrub. `drive` stays deliberately NOT gated on `GSDMM_EXPERIMENTAL_FEATURES` (260917-fko's exemption stands): the respawned child's stdio is `/dev/null`, so a refusal would break the spawn path invisibly for exactly the users who did set the flag. Two tests in a new `mod tests` in `src/cli.rs`: `the_drive_subcommand_is_absent_from_rendered_help` (both `render_help()` and `render_long_help()`, with a positive control asserting the still-visible `list` entry through the same entry-shape predicate, so absence cannot pass vacuously) and `the_drive_subcommand_still_parses_and_resolves` (`Cli::try_parse_from` yields `Commands::Drive` with alias and `--command` intact). Runtime-verified on the built binary both ways. +2 tests; gates build 0, clippy `-D warnings` 0, tests 2120 passed / 1 failed vs a 2118/1 baseline, failing set identical — the environmental git-version-constants test (installed git 2.53) | 2026-09-17 | 7ca2e94, 3ff42d4 | [260917-hc3](./quick/260917-hc3-hide-the-drive-subcommand-from-help-whil/) |
| 260917-ii4 | Re-derive the git config-section constants against git 2.55.0, the version the release CI runner carries — the re-derivation the version-witness pin's own failure message prescribes, performed rather than skipped. All twelve `Documentation/RelNotes/2.44.0.adoc` … `2.55.0.adoc` were read and cross-checked against master's `Documentation/config.adoc`, `Documentation/config/alias.adoc`, `Documentation/config/hook.adoc`, `config.c` and `alias.c`. **NEITHER ARRAY CHANGED, and that is the finding rather than an omission**: class (a) — no THIRD configuration-splicing section appeared, `config.c:git_config_include()` still splices on exactly `include.path` and `includeif.<cond>.path` (near-misses examined and excluded: 2.46/2.47's `includeIf.onbranch` fixes and the `hasconfig:remote.*.url` condition are the EXISTING section; 2.52's `:(optional)` path marker tolerates a missing file rather than reading config from it; 2.47's configurable object hash / ref backend configures git's own storage; 2.56's unreleased `worktree:` condition is a new CONDITION in the existing section and is covered by construction since the subsection is never read); class (b) — no SECOND section whose value git re-parses as a git command line, `alias.c` still has a single `parse_config_key(var, "alias", …)` call site (excluded: 2.48's `remote.<n>.serverOption` and 2.44's `fetch.all` inject one fixed flag into one subsystem; 2.54/2.55's `hook.<name>.command`/`.parallel`, `hook.<event>.jobs`, `hook.jobs` family is a shell oneliner or executable path — the file's own K2 class, recorded as examined with inheritance explicitly NOT claimed since it was never driven through the child-environment-dump harness; 2.51's `-h` alias-report tightening and 2.46's alias trace logging are behaviour inside the existing section). **The load-bearing find: git 2.54 gave `alias` a SECOND SPELLING** — `[alias "co"] command = checkout`, i.e. the three-level `alias.<name>.command`, with names that may be arbitrary UTF-8 in a raw-byte case-sensitive subsection. It adds no member, because its SECTION is still `alias` and `config_key_section` reads only to the first `.`, so the new spelling is refused with no code change; a guard that had enumerated the two-level `alias.<name>` shape would have failed OPEN on it in 2.54. That also FALSIFIED an existing doc claim — `config_key_names_a_reparsed_command_section` said the subsection is not read because "`alias` has none" — repaired in place to the reason that is now true. One executable line changed in the whole task (the version literal); every other changed line begins with `///`; the four `Measured against \`git version 2.43.0\`` probe tables are byte-identical, since the 2.44→2.55 re-derivation was DOCUMENTARY and rewriting them would fabricate a measurement never taken. Gates: build 0, clippy `-D warnings` 0, tests 48 suites / 2119 passed / 2 failed / 15 ignored, failing set identical before and after. The version pin now fails LOCALLY BY DESIGN (derived against 2.55.0 vs installed 2.53.0) and is expected to go green on the CI runner's 2.55.0; the second failing slot is a per-run-flaky process test, measured across three re-runs each way, not a regression | 2026-09-17 | ef4a2a8, 86cc7dd, 9b7d42a | [260917-ii4](./quick/260917-ii4-re-derive-git-config-section-constants-a/) |
| 260917-jdi | Add `scripts/pre-tag-check.sh`, a local pre-tag dry-run of the release gates, and route CLAUDE.md's Release Process step 3 through it. The prevention chosen for the v1.7.0 publish-gate failure, and deliberately the MINIMAL one: **no CI workflow was changed**. The script reproduces `.github/workflows/release.yml` locally, before a tag exists — tag/version match (`cargo metadata … \| jq -r '.packages[0].version'` against `${ARG#v}`, the same strip CI uses; with no argument it reports the crate version instead of failing), the msrv job's `cargo +<floor> check --all-targets --locked`, `cargo build --release`, `cargo test --no-fail-fast`, `cargo clippy -- -D warnings`. Three decisions carry the design. (1) Gate 4 uses `--no-fail-fast` where CI uses plain `cargo test`, ON PURPOSE and commented as such: fail-fast stopped CI at the lib binary and the later suites never ran, which is exactly how the v1.7.0 breakage stayed invisible. (2) The MSRV floor is read from `cargo metadata`'s `rust_version` and the toolchain detected READ-ONLY via `rustup toolchain list` — never probed with `cargo +<floor> --version`, which silently auto-installs the toolchain and so both mutates the machine and lies about the skip condition; an absent floor prints a marked `SKIPPED`, never a silent pass. (3) The git-version reconciliation block greps `CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION` out of `src/envelope/policy.rs` at runtime — no version literal is baked into the script, so it cannot go stale across the next re-derivation, and an unparseable constant is a hard error because the script has then lost its subject. A mismatch is a LOUD ADVISORY, banner before the gates and repeated in the EXIT-trap summary, but never itself a failure: local git is 2.53.0 against a constant of 2.55.0, so the mismatch is the CURRENT and CORRECT state of a properly-prepared release, and hard-failing would train the operator to ignore the one warning that matters. The banner states the three facts plainly — this run cannot validate that test the way CI will, the GitHub `ubuntu-latest` runner's ambient git is the AUTHORITY for it, and a green local gate does not guarantee the publish job passes. The version-witness test's consequent local failure is EXPLAINED in the summary, not filtered: a filter there is the same class of blindness the whole task exists to remove. On this tree the script exits 1 with gates 1/2/3/5 PASS and gate 4 FAILED on exactly the three known pre-existing failures (the version witness plus the two documented `tests/driver_reattach.rs` flakes), zero gates `NOT REACHED` | 2026-09-17 | cf2219c, b0cb5be | [260917-jdi](./quick/260917-jdi-add-scripts-pre-tag-check-sh-pre-tag-rel/) |
| 260917-k6y | Close the `tests/driver_reattach.rs` spawn/write race — the two flaking tests now synchronise on the ARTIFACT, not on process liveness. Confirmed mechanism (candidate M2 in the file's own header): `live_within` polls `driver::liveness::is_run_alive`, which is nothing but a read of `/proc/<pid>/cmdline`, and the kernel populates that file at `execve` — microseconds after `spawn()`, long before the driver has established its envelope, taken its lock, written `run.json` or emitted a single journal record. The assertions then read artifacts the writer had not produced yet, which is exactly why a FAILING run took ~0.5s against ~6.1s for a passing one. Three bounded artifact waits added beside `live_within`/`gone_within` in the file's own `*_within` idiom: `observed_within` (polls `reconcile::reconcile_all` until the run **id** appears, matching the id rather than `len() == 1`), `journal_record_within` (requires a line that actually parses as `ParsedLine::Record`, because `reader::tail_lines` returns `Ok` with zero lines for a file that does not exist and an `is_ok()` test would have returned true instantly and fixed nothing) and `run_json_within` (existence only, so a completed run cannot satisfy the wait that the still-null `ended_at` assertion depends on). All three reuse `live_within`'s deadline + 25ms shape at `Duration::from_secs(30)`, each layered AFTER the existing `live_within` assertion rather than replacing it, and each fails naming exactly what never appeared. No assertion deleted or loosened, no `#[ignore]`, no `--test-threads=1`, no `sleep`, no `src/` change. Fail-first reproduced at HEAD (1 red of 6 isolated runs, at 0.53s, with BOTH arms firing together); 10/10 isolated runs green afterwards, all at ~6.24s and none at the ~0.5s failure signature. Gates: 5× `rtk proxy cargo test --no-fail-fast` → 2120 passed / 1 failed / 15 ignored in four runs and 2119/2/15 in one, the constant failure being the environmental git-version-constants pin (installed git 2.53.0 vs constants derived against 2.55.0) and run 4's extra being the separate, still-OPEN `tests/envelope_tracer.rs` `Text file busy` flake — reported, not absorbed; `./scripts/pre-tag-check.sh` exits 1 with gate 1 SKIPPED, gates 2/3/5 PASS and gate 4 failing on **exactly one** test, down from three; `rtk proxy cargo clippy -- -D warnings` clean. Module header rewritten from "THIS FILE IS FLAKY" to a closed case (M2 confirmed and closed; M1 and M3 left explicitly open, unmeasured and unclaimed; the `--test-threads=1` correction and the historical data points preserved as the record of how the wrong answer was reached twice), the `driver_reattach` half of phase 21's `deferred-items.md` closed append-only (221 insertions, 0 deletions) with `envelope_tracer` left OPEN, and `.planning/todos/pending/2026-08-18-driver-reattach-spawn-artifact-race.md` retired to `completed/` by pure `git mv`. `Cargo.toml`, `.github/`, `src/` and tags all untouched — v1.7.1 remains a separate step. Ran unisolated on the primary checkout after the #1941/#48 worktree base-check auto-degrade | 2026-09-17 | b9f1795, b2bcec0, 7b5e3e8 | [260917-k6y](./quick/260917-k6y-fix-the-two-spawn-write-races-in-tests-d/) |
| 260917-lkg | Close the `tests/envelope_tracer.rs` `Text file busy` (ETXTBSY) flake in `a_relocated_copy_of_the_stub_refuses_instead_of_acting` — the last open flake on the v1.7.1 publish gate. **The recorded diagnosis was wrong and the correction is the finding.** `deferred-items.md` called it "the classic write-then-exec race", which accuses the `std::fs::copy` just before the second exec; `strace -f -e trace=execve` against a live reproducer caught the failing syscall three times and **every one was the SANCTIONED stub**, with the relocated copy's path never appearing in a failing `execve` at all. The CONTROL leg is what flaked; the copy is a bystander. The writer is a `NamedTempFile` descriptor from `src/envelope/hooks.rs::write_stub` inherited by a SIBLING libtest thread's forked child — `O_CLOEXEC` closes it at that child's own `execve`, but not one instruction before, and in that window the child is a writer on the stub's inode. ETXTBSY is a per-INODE condition (`i_writecount`), which is precisely why the tidy-looking repair fails: closing or syncing the `File` before `persist`'s rename changes nothing, since `rename(2)` does not change the inode and the inode was already exposed to a `fork` during the pre-rename write and chmod. Two further eliminations were considered and rejected IN WRITING rather than silently — a `hard_link` for the copy (exonerated by the strace, and it would share the sanctioned inode), and a process-wide RwLock serialising writes against spawns (needs every `Command::spawn` in the binary wrapped; any missed site reinstates the flake while *looking* deterministic). The offending descriptor is therefore not ours to close, so `run_stub` waits it out: a bounded, **ETXTBSY-only** retry on `spawn()` at `Instant::now() + Duration::from_secs(30)` with 25ms polls, the `*_within` idiom `260917-k6y` established in `tests/driver_reattach.rs`. Three things carry it and none may be relaxed — only `ErrorKind::ExecutableFileBusy` retries (a bad mode gives `PermissionDenied`, a bad shebang `ENOEXEC`, and both still fail immediately under the verbatim `the generated stub is executable` wording, so the loop cannot become a swallow-all); expiry **PANICS** rather than returning a synthesised `Output`, because an exec that never happened proves nothing about refusal and `Text file busy` may never stand in for the refusal asserted; and the sleep is INSIDE the poll loop, not a fixed pre-assertion `sleep`. No assertion deleted or loosened (33 asserts after vs 29 before, zero removed), no `#[ignore]`, no `--test-threads`, no `src/` change. Fail-first MEASURED on the untouched tree: 0/70 isolated runs and 0/6 full-suite runs, then **10 failures in 200** under an 8-way contended reproducer. After: **0 ETXTBSY in 1000 contended runs** (4×200 by the executor plus an independent 200-run coordinator re-check) with the retry branch provably EXERCISED — 8 absorbed retries logged — so the fix is proven in effect, not merely absent. Gates: 9× `rtk proxy cargo test --no-fail-fast` → 2120 passed / 1 failed / 15 ignored in eight and 2119/2/15 in one, target test green in ALL nine and `Text file busy` in none; the constant failure is the environmental git-version-constants pin (git 2.53.0 vs constants derived against 2.55.0) and run 7's extra is the separate per-run-flaky `driver::run::tests::the_current_group_agrees_with_the_proc_parse`, reported not absorbed. `./scripts/pre-tag-check.sh` exits 1 with gate 1 SKIPPED, gates 2/3/5 PASS and gate 4 failing on **exactly one** test — the version witness alone. `rtk proxy cargo clippy -- -D warnings` clean. Module header gained an ETXTBSY section recording why the copy was exonerated and what the wrong fix would have been; phase 21's `deferred-items.md` closed its `envelope_tracer` half append-only (253 insertions, **0 deletions**), quoting verbatim the sentences it supersedes including `260917-k6y`'s own "stays OPEN", so the two-binary item is fully closed. **One NEW, distinct race found and deliberately left OPEN**: `the stub reads its stdin: BrokenPipe` at `run_stub`'s `write_all`, 1 in 800 contended runs and 0 in nine full-suite runs, not introduced here (that block is byte-identical before and after) and not fixed because the plausible repair touches an assertion — a hard prohibition; registered with measurements in `deferred-items.md` per the T-19-91 precedent. `Cargo.toml`, `.github/`, `src/` and tags untouched — v1.7.1 remains a separate step. Ran unisolated on the primary checkout after the #1941/#48 worktree base-check auto-degrade | 2026-09-17 | 59108b1, e505eac, 41c0b5d | [260917-lkg](./quick/260917-lkg-make-the-envelope-tracer-relocated-stub-/) |
| 260917-nhc | Close the last flake on the v1.7.1 publish gate — `driver::run::tests::the_current_group_agrees_with_the_proc_parse` — and the finding is that the PRIOR RECORDED DIAGNOSIS WAS WRONG in both of its load-bearing clauses. `260908-uqq/deferred-items.md` wrote that "No test in the tree calls `establish_own_group()`, so the cross-test `setpgid` hazard the test's own doc names is not the mechanism" and that "`getpgrp()` cannot return a stale value, so the suspect is the `/proc/<pid>/stat` field parse under the load of many test binaries running concurrently". The `/proc` parse is EXONERATED and the cross-test `setpgid` hazard IS the mechanism: no test calls `establish_own_group()` DIRECTLY, but `src/driver/mod.rs:1834` — the "visible twin, driven end to end" arm of `a_target_phase_that_renders_as_another_is_refused_at_the_seam` — reaches it TRANSITIVELY through `drive` -> `execute_run` (`src/driver/run.rs:2553`), once per `LOOK_ALIKE_PAIRS` entry. On Linux `setpgid(0, 0)` resolves against the THREAD-GROUP LEADER, not the calling thread, so a libtest worker thread moves the WHOLE shared binary's process group, and the flaky test's two reads — `current_group()` then `kernel_process_group(std::process::id())` — straddle it. **This is a TEST-SIDE race; production `/proc` parsing and group tracking are correct and `src/` outside the test module is untouched.** Evidence, not theory, following the `260917-lkg` precedent: `strace -f -e trace=setpgid,execve` caught exactly 7 `setpgid(0, 0) = 0` calls from one TID that never `execve`s (a thread, not a forked child) while the main pid's own stream carried none, and correlating against libtest's fd-1 writes under `--test-threads=1` put all 7 inside the `LOOK_ALIKE_PAIRS` loop; a non-job-leader Python parent polling `/proc/<child>/stat` watched the group transition `[2751890, 2751905]` — inherited parent group to own pid — mid-run. That also explains why it never reproduced interactively: shell job control already makes a directly-launched binary its own group leader, so the `setpgid` is a no-op and no window exists. Fail-first was MEASURED before a byte was edited: **0 failures in 39 direct runs** against **3 in 80 contended runs** under the non-leader-parent reproducer (executor re-measured 2/60 on a snapshot of the untouched HEAD binary; 5 in 140 contended overall, ~3.6%). Every reproduced failure carried the predicted signature — the `assert_eq!` LEFT value constant across runs (the parent's pgid) and the RIGHT value per-run and equal to that run's own binary pid. The fix is a seqlock read in a documented helper, `group_observations_without_a_move_within(Duration::from_secs(30))`: `current_group()`, the `/proc` parse, `current_group()` again, returning only when the bracketing reads agree, 25ms polls, and **expiry PANICS** naming the moving-group condition rather than fabricating a pair. `leading == trailing` is sufficient rather than merely suggestive because `setpgid(0, 0)` always targets the caller's own pid, so it is one-way and idempotent and the group cannot move away and back inside one window — recorded with the note that a second, differently-targeted `setpgid` would require re-deriving it. **Nothing was weakened**: the `assert_eq!` and its failure message survive BYTE-IDENTICAL, the `.expect("this process's own /proc/<pid>/stat is readable")` is retained and is explicitly NOT a retry condition (an unreadable stat file is the parse failing, not the group moving, and retrying it would hide the D-04 defect the equality exists to catch), no `#[ignore]`, no `--test-threads=1`, no bare pre-assertion `sleep` (the 25ms sleep is inside the poll loop), no assertion deleted or loosened, no equality widened. After the fix: **0 failures in 200 contended runs** by the executor plus an independent **0 in 80** coordinator re-check against its own 3/80 baseline, with the retry branch **provably EXERCISED** — 7 firings across 7 distinct runs (3.5%), absorbed on attempt 1 every time, each logging the constant-left/per-run-right signature — so this is proven in effect rather than merely absent; anti-vacuity checked by confirming the retry string present in the rebuilt binary and absent from the HEAD snapshot. Gates: ten `rtk proxy cargo test --no-fail-fast` runs, all ten **2120 passed / 1 failed / 15 ignored**, the target test green in all ten and in no `failures:` block; the sole failure every run is the environmental git-version-constants pin (local git 2.53.0 against constants re-derived against 2.55.0, expected green on the runner). **No BrokenPipe in any of the ten** — the known, deliberately-open `tests/envelope_tracer.rs` `run_stub` race did not appear and was not touched. `./scripts/pre-tag-check.sh` exits 1 with gate 1 SKIPPED, gates 2/3/5 PASS and gate 4 failing on **exactly one** test — the version witness alone, which is the state a correctly-prepared v1.7.1 is supposed to be in. `rtk proxy cargo clippy -- -D warnings` clean. The paper record moved with the code: the module header gained a cross-test `setpgid` hazard section recording the mechanism, the mover, the strace and `/proc`-watch evidence, the measured rates and both exonerations, and the test's own comment now says the hazard it called hypothetical is ACTUAL; phase 21's `deferred-items.md` closed the row append-only (**277 insertions, 0 deletions**), quoting verbatim both superseded `260908-uqq` sentences. **Two documented non-goals, both flagged for audit**: `execute_run`'s `establish_own_group()` ordering was deliberately NOT changed (T-nhc-05, dispositioned `accept`) because production `drive` is a dedicated process where the group move is correct and intended, and re-ordering production code to accommodate a shared test binary would be the wrong trade; and the mover test was not stopped, since that would mean a test-only seam in production `execute_run` or deleting a real end-to-end control arm. `Cargo.toml`, `.github/`, `src/` beyond the `run.rs` test module, and tags all untouched — v1.7.1 remains a separate step. Ran unisolated on the primary checkout after the #1941/#48 worktree base-check auto-degrade | 2026-09-17 | 0e603eb, 9ea3204, dbab7f9 | [260917-nhc](./quick/260917-nhc-close-the-last-v1-7-1-publish-gate-flake/) |

| 260917-r4c | Close the release-gate environment gap that has now cost **three** failed publishes — and do it by AUDITING THE WHOLE ENVELOPE AT ONCE rather than one tag at a time. `.github/workflows/release.yml`'s `publish` job ran plain `cargo test` (fail-fast), so each tag surfaced exactly ONE environment-dependent failure, it got fixed, and the next tag surfaced the next: v1.7.0 the git-version witness, then three local races, then v1.7.1 `tests/driver_router_conformance.rs:377::the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state` — which fails BY DESIGN when its GSD oracle is absent, and an `ubuntu-latest` runner has Node but no GSD. `scripts/pre-tag-check.sh` structurally could not catch this class: it validates THIS machine, where the oracle and every other ambient dependency is always present, so the local gate was green on precisely what CI was red on. **The audit**: a faithful `ubuntu-latest` lookalike container — git 2.55.0 from `ppa:git-core/ppa` (the runner's version AND the version `CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION` records), Node 24 from NodeSource, `gh` 2.101.0 from the same `cli.github.com` apt repo GitHub's image uses, rustc stable + the 1.88.0 MSRV floor, non-root uid 1000, and a deliberately empty `HOME` with no `~/.claude` — then `cargo test --no-fail-fast`. All 45 integration binaries plus lib, bin and doc-tests RAN (verified by diffing the `Running tests/` list against `ls tests/*.rs`): **2118 passed, 3 failed**, the complete set. (1) the conformance oracle at `driver_router_conformance.rs:377` — the genuine runner failure. (2) and (3) `src/envelope/policy.rs:9478`'s two `gh api` grammar pins, `every_gh_api_value_opt_really_takes_a_separate_value_on_the_installed_gh` and `the_forge_value_opts_gh_half_is_pinned_two_sided_and_the_glab_half_cannot_be`, which failed only because the FIRST image build omitted `gh`; `ubuntu-latest` ships it, and with 2.101.0 present both PASS — a container-fidelity gap, not a runner failure, and the reason the image now installs `gh` from the runner's own repo. The git-version witness PASSES in the container, closing v1.7.0's item on the runner. **The fix PROVIDES the dependency rather than switching the check off**: `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` is set NOWHERE — it would make the conformance run vacuous, reporting green while checking nothing, which is the fail-open shape this codebase repeatedly rejects (WR-07). New `scripts/install-conformance-oracle.sh` reads `GSD_CORE_SYNCED_VERSION` out of `src/state_reader/config_json.rs` AT RUN TIME — no version literal is baked into a script or a workflow, the same staleness argument `pre-tag-check.sh`'s constant extraction makes — installs that exact pinned `@opengsd/gsd-core`, links it onto `Oracle::resolve()`'s FIRST branch at `$HOME/.claude/gsd-core` (so no `$GITHUB_PATH` surgery is needed across steps), and **verifies the oracle actually answers** a real `query init.manager` against a throwaway fixture before exiting 0, so the step cannot succeed while leaving the test vacuous. Two traps recorded: the npm package's `gsd-core/` carries no `VERSION` file, and `gsd-tools --version` exits non-zero with `Unknown flag: --version`, so neither is a liveness probe. The `publish` job gains `actions/setup-node@v4` (24) and that script BEFORE `Test`, and `Test` moves to `cargo test --no-fail-fast` — the root cause of the three-tag cycle, and the change that finally makes CI and `pre-tag-check.sh` gate 4 agree. The `msrv` job is byte-identical and **no new workflow file was created** (explicitly declined). `scripts/pre-tag-check.sh` gains `--container`: it builds `scripts/pre-tag-check.Dockerfile` and re-runs ITSELF inside it after running the same provisioning script CI runs, so a green container run certifies the publish job. The dispatch sits AFTER the `RULE`/`THIN` definitions and BEFORE the `trap ... EXIT`, which is what makes "a bare invocation behaves exactly as today" structural rather than hoped-for; `PRE_TAG_CHECK_IN_CONTAINER` guards recursion, and **there is deliberately no fallback** — no usable runtime is a loud hard error and exit 1, because a silent fallback to the local run would recreate the exact blind spot the flag exists to close. Verified independently by the coordinator: `./scripts/pre-tag-check.sh --container` exit 0 with gates 2/3/5 PASS, gate 4 PASS, the git banner reading **MATCH** (2.55.0 == 2.55.0), 48 `test result: ok` and zero FAILED, all three previously-failing tests green; the bare local run unchanged at exit 1 with gate 4 failing on **exactly one** test, the version witness alone; `rtk proxy cargo clippy -- -D warnings` clean; `--bogus` still exits 2, `--help` 0, the recursion guard 2, and a forced bogus runtime fails loudly at exit 1 having certified nothing. `src/`, `Cargo.toml` and `Cargo.lock` are untouched, no test was `#[ignore]`d, no tag cut — v1.7.2 remains a separate step. **One newly-observed, deliberately-open item**: a BrokenPipe at `tests/envelope_interior_path.rs:1068` in that file's own `drive` closure (the guard child refuses and exits before the parent finishes writing stdin) — a DIFFERENT site from the known `envelope_tracer.rs` `run_stub` race, seen once on a fully loaded dev machine and measured 0/30 uncontended, 0/12 whole-binary and absent from every container run; reported, not fixed and not absorbed. Ran unisolated on the primary checkout after the #1941/#48 worktree base-check auto-degrade | 2026-09-17 | 4799cb2, 3594ddf, 087a6ef | [260917-r4c](./quick/260917-r4c-provision-the-conformance-oracle-on-the-/) |
| 260922-hdh | Show registered projects whose folder is missing in red | 2026-09-22 | 0e112b2 | .planning/quick/260922-hdh-show-registered-projects-whose-folder-is-missing-in-red-todo |
| 260922-hdi | Filter the config screen by typing slash | 2026-09-22 | b90ef32 | .planning/quick/260922-hdi-filter-the-config-screen-by-typing-slash-todo-planning-todos |
| 260922-hdj | Support Codex as well as Claude as the agent runtime (MVP slice) | 2026-09-22 | 862c038 | .planning/quick/260922-hdj-support-codex-as-well-as-claude-as-the-agent-runtime-todo-pl |
| 260923-lr8 | Fix tmux pane TTY match bug (exact match) | 2026-09-23 | 7cf6910 | .planning/quick/260923-lr8-fix-tmux-pane-tty-match-bug-in-src-terminal-switch-rs-exact |
| 260923-lr9 | Detect interactive Codex CLI sessions (detection + badge + tmux switch, no resume) | 2026-09-23 | 45c2138 | .planning/quick/260923-lr9-detect-interactive-codex-cli-sessions-detection-badge-tmux-s |
| 260923-lra | README macOS note: /proc-based session detection, best-effort Codex | 2026-09-23 | 25dce1e | .planning/quick/260923-lra-readme-macos-note-live-session-detection-claude-and-codex-re |
| 260923-md1 | Roadmap tab renders phases as a left-to-right dependency graph (pure layout in `src/ui/roadmap_graph.rs`: longest-path layering, chain rows, ┬/└/┘ fan-out/fan-in junctions, reference rows for undrawable edges, cycle/unknown-dep notes) with milestone boundaries (ROADMAP `Phases A-B` ranges → header band + row-end `(M3 → M4: name)` labels, active milestone highlighted); graph default, `v` toggles the old box list | 2026-09-23 | 10a0bf3 | [260923-md1-roadmap-dependency-graph-view-with-miles](./quick/260923-md1-roadmap-dependency-graph-view-with-miles/) |
| 260924-drx | Backlog tab: Enter focuses a scrollable content pane (side-by-side at >=100 cols), j/k PgUp/PgDn scroll it, Enter/Esc close; `e` opens ROADMAP.md at the item's heading line (`$EDITOR +N`) | 2026-09-24 | a723b3a | [260924-drx-backlog-tab-enter-to-focus-scrollable-co](./quick/260924-drx-backlog-tab-enter-to-focus-scrollable-co/) |

## Session Continuity

Last session: 2026-09-26T03:39:28.916Z
Stopped at: Completed 25-05-PLAN.md
user-facing features); `cargo update` moved cc, find-msvc-tools, finl_unicode, instability, libredox,
lru, pest*, process-wrap, thiserror*; still held by upstream `=` pins: generic-array 0.14.7 (0.14.9),
unicode-width 0.2.0 (0.2.2). `assets/` excluded from the crate package. Router names Phase 19 next,
but its gap-closure loop is halted by user decision — do not resume without asking.
Resume file: None

Previous session (v1.7.2 release):
Release: `Cargo.toml` at 1.7.2. `cargo update` relocked **only this crate's own entry**
(1.7.1 -> 1.7.2); no dependency moved. Still behind latest, unchanged from v1.7.0/v1.7.1 and not
bumpable from this manifest: `generic-array` 0.14.7 (latest 0.14.9) and `unicode-width` 0.2.0
(latest 0.2.2), both held by upstream `=` pins. `Cargo.lock` is committed — CI publishes with
`cargo publish --locked`.
Verification ran through **both** arms of CLAUDE.md release step 3:
  `./scripts/pre-tag-check.sh --container v1.7.2` — **exit 0**, gates 1 (tag/version),
  2 (MSRV 1.88), 3 (`cargo build --release`), 4 (`cargo test --no-fail-fast`) and 5
  (`cargo clippy -- -D warnings`) ALL PASS; git banner **MATCH** (container git 2.55.0 against the
  constant's 2.55.0); 48 suites / 2121 passed / **0 failed** / 15 ignored. This is the arm that
  corresponds to CI, and it is the first time it has been green.
  `./scripts/pre-tag-check.sh v1.7.2` (bare, dev machine) — exit 1, gates 1/2/3/5 PASS, gate 4
  failing on **exactly one** test: the environmental git-version witness
  `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
  (local git 2.53.0 vs constants derived against 2.55.0). 48 suites / 2120 passed / 1 failed /
  15 ignored. That single failure is the state a correctly-prepared release is supposed to be in,
  and the container run is the proof it goes green on the runner.
Deliberately-open items carried, NOT fixed in this release: the `run_stub` BrokenPipe race in
`tests/envelope_tracer.rs` (~1 in 800 contended runs) and the second BrokenPipe at
`tests/envelope_interior_path.rs:1068`. Neither appeared in either gate run.
`master` pushed, `dev` fast-forwarded to `master` and pushed (never rebased — planning docs cite
shas), then the `v1.7.2` tag pushed last, triggering `.github/workflows/release.yml`.

Previous session (2026-09-17T23:30:00.000Z) — Tagged and pushed v1.7.1, which DID NOT PUBLISH:
Release: `Cargo.toml` at 1.7.1, `Cargo.lock` refreshed by `cargo update` — **only this crate's own
version entry moved** (1.7.0 -> 1.7.1); no dependency relocked, because v1.7.0's refresh was a day
earlier. `generic-array` 0.14.7 (latest 0.14.9) and `unicode-width` 0.2.0 (latest 0.2.2) remain
behind latest under upstream `=` pins — the same two carried over from v1.7.0, deferred, not
bumpable from this manifest. Verification ran through `./scripts/pre-tag-check.sh v1.7.1`
(CLAUDE.md release step 3): gates 1 (tag/version), 2 (MSRV 1.88 `cargo check --locked`),
3 (`cargo build --release`) and 5 (`cargo clippy -- -D warnings`) PASS, no gate NOT REACHED, and
gate 4 (`cargo test --no-fail-fast`, 48 suites / 2120 passed / 1 failed / 15 ignored) failing on
exactly one test — `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`,
which is environmental (local git 2.53.0 against constants deliberately re-derived against the
runner's 2.55.0) and is the state a correctly-prepared v1.7.1 is supposed to be in.
`master` pushed, `dev` fast-forwarded to `master` and pushed (never rebased — planning docs cite
shas), then the `v1.7.1` tag pushed last, which triggered `.github/workflows/release.yml`
(run 35290670227).
**RESULT: msrv job success, publish job FAILURE at the `Test` step — nothing published.**
crates.io remains at **1.6.0**; neither the v1.7.0 nor the v1.7.1 tag names an installable version.
The failing test is
`tests/driver_router_conformance.rs::the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state`
at `tests/driver_router_conformance.rs:377`, panicking because the GSD conformance oracle is not
present on the runner. It was NOT re-run and NOT fixed — that is the next task, and the honest
options are to install GSD/Node in the publish job or to set
`GSD_META_MANAGER_ALLOW_MISSING_ORACLE=1` there and accept that CI verifies none of the
transcription. Whichever is chosen, `scripts/pre-tag-check.sh` needs a way to reproduce the
runner's oracle-absent condition, because a developer machine always has the oracle and the
script was green on this exact tree.
(That repair was done by quick task 260917-r4c and v1.7.2 was cut on top of it — see above.)
Next action: v2.0 resumes (no release has closed the milestone): execute Phase 22, and
re-run `/gsd-secure-phase 19` so 19/20/21 can move off "In Progress".
Prior-session note, still live:
`21-20-PLAN.md` and both PASSED after revision (3b0ef4d, addc3cc); the ROADMAP now carries its
"Gap closure, round 7" block. Next action: execute round 7 (wave 1 = 21-19, wave 2 = 21-20).
Resume file: None
(both retained deliberately — the plan-checker step runs as a separate agent and needs them;
delete only after round 7 executes).
Note: `.planning/phases/19-gitsafe-git-blast-radius-envelope/.continue-here.md` is a stale
mid-execution checkpoint (claims task 3 of 8); all 12 plans have summaries. Ignore it.
Note (19-12): the next action for Phase 19 is **re-run `/gsd-secure-phase 19`** — `19-SECURITY.md`
still carries `threats_open: 1` / `status: blocked` and a Sign-Off block predating 19-11. Neither
19-11 nor 19-12 edited it, deliberately (D-25): re-running the audit is what should flip it, not
the plans that closed the finding writing their own verdict into it. `T-19-60` is closed and now
certified at the class level; the `T-19-74` pin the register names as 19-12's job is done.
Note (19-12): use `cargo test --no-fail-fast` for any phase-19 suite number. A plain `cargo test`
stops at the failing `driver_reattach` binary and never reaches any `envelope_*` binary.
Note: `.planning/ROADMAP.md`'s "Gap closure, round 7" block is now written (unexecuted `- [ ]`
boxes for 21-19/21-20); the outstanding-edit note that stood here is resolved.
