---
gsd_state_version: 1.0
milestone: v2.0
milestone_name: Autonomous Orchestration
current_phase: 19
current_phase_name: GITSAFE — Git & Blast-Radius Envelope
status: verifying
stopped_at: Completed 19-22-PLAN.md (corpus RED; 19-23 writes the rules)
last_updated: "2026-09-04T04:34:14.540Z"
last_activity: 2026-08-29
last_activity_desc: Phase 19 plan 12 executed — all 12 plans summarised
state_head: 483f6e020c8f1bf0269fa6ae769d90a0243decf9
progress:
  total_phases: 10
  completed_phases: 6
  total_plans: 102
  completed_plans: 101
  percent: 60
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-31)

**Core value:** See the state of every GSD project at a glance and act on any of them without leaving the TUI.
**Current focus:** Phase 19 — GITSAFE — Git & Blast-Radius Envelope

## Current Position

Phase: 19 (GITSAFE — Git & Blast-Radius Envelope) — ALL 12 PLANS SUMMARISED
Plan: 12 of 12 (12 summaries on disk)
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
Status: Phase complete — ready for verification
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
Last activity: 2026-08-29 — Phase 19 plan 12 executed; all 12 plans summarised

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

- T-19-86 (governed program's own operand names a governed command) and T-19-87 (${VAR} fragments a command past the splitter) are open, pinned and deferred to a round-4 plan; /gsd-secure-phase 19 cannot clear until they are closed
- T-19-97/T-19-98/T-19-99 open — corpus RED at 1605 (passed+failed); 19-19 writes the rule. /gsd-secure-phase 19 NOT cleared: T-19-86 and T-19-91 remain open at high
- 19-21: tests/envelope_callee_grammar.rs pins `git --super-prefix x push --force origin main` at force_push_blocked while pinning `git --super-prefix x status` at envelope_assertion_failed — identical leading tokens, so no rule obeying 19-21's prohibitions satisfies both. Row left RED. Both refuse at exit 2; only the identifier differs. **RESOLVED 2026-09-04 (989f21a)** — the 19-21 executor's analysis was verified by measurement against the built binary (fresh envelope root per row, root walked after) and HELD: both spellings refuse at exit 2 with an empty walk, `--super-prefix` is absent from GIT_GLOBAL_VALUE_OPTS after 19-21, and git 2.43.0 rejects it bare, separate and attached. `force_push_blocked` was a PRE-fix observation mislabelled as post-fix. Row 937 corrected to envelope_assertion_failed; exit code and empty walk still asserted; no `src/` change and no second reading site. All thirteen envelope_* binaries green; passed+failed = 1639, 0 failures. **This does NOT clear `/gsd-secure-phase 19`** — T-19-86 and T-19-91 remain OPEN at high, T-19-17r stays OUTSTANDING (no AR-19-13, not accepted), and only the WRAPPER-OPERAND sub-class of T-19-60 is closed.

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

## Session Continuity

Last session: 2026-09-04T04:34:00.987Z
Stopped at: Completed 19-22-PLAN.md (corpus RED; 19-23 writes the rules)
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
