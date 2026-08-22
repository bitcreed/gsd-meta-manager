---
phase: 21-llm-goal-layer-prompt-injection-hardening
verified: 2026-08-22T01:40:00Z
status: gaps_found
score: 4/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  gaps_closed:
    - "pass-4 gap 1 (the `Routed` arm carried no blankness check): CLOSED and structurally so. All three `CommandSource` variants now carry `payload::NonBlank` (src/driver/mod.rs:395/398/402); the field is private to a NESTED `pub(crate) mod payload` (:324, struct :331) with exactly one constructor and no `From`/`Deref`/`AsRef`/`into_inner`/serde. A blank payload is unrepresentable inside the type, not merely rejected by a remembered guard. Confirmed by direct read; I found no construction route past the constructor."
    - "pass-4 gap 2 (the matrix hand-exempted the `--target-phase` column with a false justification): CLOSED. The matrix at src/driver/mod.rs:2178-2196 is one uniform nested loop over 3 argv positions x 6 DEGENERATE payloads = 18 cells, every one asserted `Err(NoCommandSource)`. No `Ok` branch, no exemption comment. The column axis is tied to `command_source`'s arity at COMPILE time through `PositionBuilder`'s 3-tuple (:2163). Confirmed by direct read of the loop body."
    - "pass-4 gap 3 (the `--target-phase '   '` test pin was missing): CLOSED. `tests/driver_dry_run.rs:624 a_blank_target_phase_is_refused_in_preview_and_in_a_real_run` exists and passes; driver_dry_run is 15/15."
    - "pass-4 gap 4 (the trim tautology: DEGENERATE and the detector both keyed on production's own `trim`): CLOSED at the array level. `DEGENERATE` (:1888) now holds six literals including `\\u{200b}` and `\\u{feff}`, which `trim` cannot reach; `NonBlank::new`'s predicate is whitespace + control + zero-width/format, and the matrix genuinely refuses all six. (The DETECTOR half is a different story — see gaps: it is never fed a degenerate payload.)"
    - "pass-4 gap 5 (guard six's header omitted a live marker-to-EOF blind region): CLOSED, and closed better than asked. The header now names limits 4 and 5, and a new assertion `no_production_item_follows_a_test_module_marker` (tests/spawn_seam_guard.rs:2036) makes the blind region's emptiness a build failure. The live offender is gone: `src/state_reader/mod.rs` now has `pub fn count_backlog_items` at :312 ABOVE `mod tests` at :331 (was :530 vs :311). `cargo clippy --all-targets` went 5 -> 4 warnings; `items_after_test_module` is cleared. All three facts self-measured."
    - "pass-4 gap 6 (guard eight named no limitations at all): CLOSED. Guard eight now carries a five-item numbered limits block (tests/spawn_seam_guard.rs:2287-2300+), each limit with its failure direction, and closes two gaps for real: `Self::`-qualified needles and a variant-import assertion. Confirmed by direct read."
    - "pass-4 gap 7 (`.planning/REQUIREMENTS.md` had DRIVE-01/DRIVE-03 prematurely marked Complete): CLOSED. At HEAD all five phase-21 requirements read `[ ]` in the checklist (lines 62, 63, 83, 85, 86) and `Gaps Found` in the traceability table (lines 152, 153, 164, 166, 167) — now internally consistent, which they were not before. Confirmed by `git log -- .planning/REQUIREMENTS.md`: the last touch is `0c4f712` (the pass-4 verification commit), and neither 21-13 nor 21-14 flipped a requirement, honouring both plans' own process prohibition."
    - "round-4 named debt: the colliding `enum CommandSource` in src/driver/run.rs is renamed `IterationSource`; exactly one `enum CommandSource` is declared under src/. Guard eight asserts the single declaration rather than dodging the collision by needle shape."
  gaps_remaining:
    - "Criterion 1 fails a FOURTH consecutive verification cycle — but for the first time the root cause is NOT inside the mechanism the previous round built. The `NonBlank` type-level fix HELD under attack. All three of this pass's reproduced defects sit OUTSIDE its domain: `DriveArgs` carries six argv-derived string fields (`alias`, `command`, `target_phase`, `approved_plan`, `run_id`, `goal`) and round 4 converted exactly the three that `CommandSource` happens to have variants for. The three it did not convert are where the new Criticals are."
  regressions: []
gaps:
  - truth: "A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs (ROADMAP criterion 1)"
    status: failed
    reason: "21-13-PLAN.md declares as its FIRST must_haves truth, traced to DRIVE-01 and DRIVE-03: `An argv payload that carries no visible instruction ... is refused with DriveError::NoCommandSource in EVERY argv position (--command, --target-phase, --goal), before any file, lock, journal, or run directory is created`. I falsified that by direct observation on the PUBLIC entry point `driver::drive`. It also declares a `verification: backstop` truth: `No run.json written by this build carries target_phase or gsd_command as an empty or whitespace-only string`. I falsified that by direct observation too. Three distinct reproductions below, each of the same class this phase has now failed four times — an unvalidated no-visible-instruction value reaching a persisted record where the empty string already means `field absent` (D-30). What has changed is the LOCATION: not a fourth forgotten arm inside `command_source`, but three argv strings that never enter `command_source` at all."
    artifacts:
      - path: "src/driver/run.rs"
        issue: "REPRODUCED (upgrading 21-REVIEW.md CR-01 from `inferred` to observed). `recorded_command` (:839-849) and `digested_command_fragment` (:867-872) both still spell `(None, None) => String::new()`, carrying the identical justification comment 21-14 declared refuted and removed from the third such arm 75 lines below. `recorded_command`'s value becomes `RunRecord.gsd_command` (:779). The ordering is fatal and I confirmed there is no earlier guard: I extracted lines 2261-2455 of `execute_run` (195 lines) and grepped them — ZERO `return Err`, and the only `?` sites are plan-approval staleness, `RunIdRequired` and `bounds::resolve`. So `make_run_record` at :2448 -> `JournalRun::start` writes run.json at :2514 -> the new `NoCommandSource` refusal fires at :2643. I called `pub async fn execute_run` directly (via `DrivableProject::for_testing_bypassing_opt_in`, a real fixture project, `escalate::resolve(None, 5)`) with `command: None, target_phase: None`. Result: `Err(NoCommandSource)` — AND on disk, run.json containing `\"gsd_command\": \"\"` and `\"goal\": \"\"`, plus a run directory and a 4-record journal.jsonl. The refusal 21-14 added is downstream of the corruption it was commissioned to prevent. This directly violates 21-14's own prohibition: `MUST NOT manufacture a sentinel or blank value as the 'safe' arm of an unreachable match` — violated twice, in the file the plan edited, in the commit that declared it."
      - path: "src/journal/mod.rs"
        issue: "REPRODUCED END TO END (21-REVIEW.md CR-02, and worse than the review showed). `is_plain_path_component` (:260-263) judges blankness with `value.trim().is_empty()`, while `payload::NonBlank::new` (:348-359 of driver/mod.rs) judges it as whitespace + control + zero-width/format. Two definitions of `blank`, shipped in the same commit, disagreeing. Measured against the real library: `is_plain_path_component(\"\\u{200b}\") = true`, `(\"\\u{feff}\") = true`, `(\"\\u{2060}\") = true`, `(\"   \") = false` — and `U+200B is_whitespace=false is_control=false`, so `char::is_whitespace`/`is_control` genuinely cannot see them. The review stopped at the predicate; I drove it through `driver::drive`. `drive(--command '/gsd:progress' --run-id '\\u{200b}')` returned `Ok(())` and COMPLETED A FULL RUN: the runs directory now contains an entry whose entire name is one zero-width character, with `run.json` recording `run_id: \"\\u{200b}\"` and `outcome: \"succeeded_no_changes\"`. A successful, persisted, terminal run record that no human can name, read back, or distinguish from a sibling run called `abc` vs `abc\\u{200b}`. Blast radius is 7 production call sites, not one: `--run-id` (driver/mod.rs:861, journal/mod.rs:314, journal/writer.rs:491), registry aliases (main.rs:244, envelope/mod.rs:204, envelope/cred.rs:758) and the goal-seam phase token (goal.rs:667). The narrowed pin at driver/mod.rs:1576 loops `[\"   \", \"\\t\", \"\\n  \\n\"]` — three of `DEGENERATE`'s six — in the same commit that added the other three, violating 21-13's own prohibition: `MUST NOT exempt any cell of an anti-recurrence enumeration by comment or hand-picked list`."
      - path: "src/driver/run.rs"
        issue: "REPRODUCED (21-REVIEW.md IN-01, which I grade materially higher than Info because it falsifies a named must_have truth on the public path). `make_run_record` writes `goal: args.goal.clone().unwrap_or_default()` (:763) — the raw argv string, never validated. `command_source`'s `(Some, None)` and `(None, Some(_))` arms structurally never touch `goal`, and the matrix's `PositionBuilder` only ever builds ONE-flag tuples, so the two-flag cell is a shape the anti-recurrence mechanism cannot express. Driven through `drive()` for three payloads, preview and real run, each returning `Ok(())`: `--command '/gsd:progress' --goal '   '` -> run.json `goal: \"   \"`; `--goal '\\u{200b}'` -> `goal: \"\\u{200b}\"`; `--goal '\\u{feff}'` -> `goal: \"\\u{feff}\"`. All three created the lock, the run directory, the journal and run.json — i.e. the `before any file, lock, journal, or run directory is created` clause is false for `--goal` whenever another source is present. Compounding it, `src/ui/screens/driver.rs:755`'s `goal_lines` gates on `goal.trim().is_empty()`, so a `\\u{200b}` goal takes the NON-empty branch and the TUI renders `goal: ` with a label and nothing after it — the surface asserting a goal was stated while showing none. The comment at driver/mod.rs:465-467 cites D-30 by name (`the empty string already means field absent ... so the record cannot be used as evidence of what ran`) roughly 300 lines from the `unwrap_or_default()` that produces exactly that value."
    missing:
      - "Convert the argv-derived string set by TYPE, not by hand. `DriveArgs` (src/driver/mod.rs:131) has exactly six string-bearing argv fields: `alias`, `command`, `target_phase`, `approved_plan`, `run_id`, `goal` (plus debug-only `claude_program`/`claude_args`). Round 4 converted 3 of 6 — precisely the three `CommandSource` has variants for. Validate at the parse boundary so `DriveArgs` itself cannot hold a blank; then `execute_run` has nothing blank to re-derive, both `String::new()` arms become unrepresentable rather than patched, and `--run-id`/`--goal` are covered by construction rather than by a fourth remembered check."
      - "ONE blankness predicate. Promote the visibility judgment out of `mod payload` into a shared `pub fn carries_no_visible_content` and have `NonBlank::new`, `journal::is_plain_path_component` (:263), `run::from_argv_goal` (:1945), `ui::screens::driver::goal_lines` (:755) and `app::goal_or_none` (:106) all call it. The tree currently holds 19 `trim().is_empty()` sites under `src/`; the five above are the ones judging a user-supplied payload. Keep `visibly_empty_numbered_entry` deliberately separate — that one is the oracle and MUST be able to disagree."
      - "Lift `DEGENERATE` to a shared test const that every blank-shape pin consumes (driver/mod.rs:1576 and journal/mod.rs:2715 currently hand-copy narrower subsets), so a seventh blank shape lands in every seam's pin at once."
      - "Hoist the `(None, None)` refusal in `execute_run` above `make_run_record` at :2448 as the minimal fix, and add the guard-suite needle the plan got two lines too narrow: `String::new()` may not appear as a match-arm value in run.rs's argv-derived record builders (the plan asserted `grep -c 'Fixed(String::new())' -> 0`, which both surviving arms pass)."
      - "Give the matrix a multi-flag row. Its `PositionBuilder` 3-tuple makes the COLUMN axis structurally exhaustive over argv arity, but every row it builds sets exactly one `Some`; the `(Some(command), _, Some(blank_goal))` cell that broke this round is unexpressible in its current shape."
  - truth: "The guard and doc machinery this round built or rewrote is itself accurate about what it does and does not check"
    status: partial
    reason: "Four confirmed accuracy defects in round 4's own new machinery, all independently reproduced or read rather than accepted on 21-REVIEW.md's word. None is independently exploitable today; each is what stops the NEXT reviewer having to rediscover it by hand. The most important is WR-01, because it is a guard header overclaiming its reach for the THIRD consecutive round."
    artifacts:
      - path: "tests/spawn_seam_guard.rs"
        issue: "REPRODUCED. The new boundary self-check's `ITEM_OPENERS` (:1991-2005) contains `\"pub \"` but not `\"pub(\"`, and `line.starts_with(\"pub \")` cannot match `pub(crate) fn ...`. I counted the tree: `grep -rn '^pub(crate) \\|^pub(super) ' src/` gives exactly 51 column-zero items structurally invisible to the assertion. The header (:2074-2078) claims `the only way a skipped region can be trusted is if it is empty, so this asserts exactly that` and bounds its residual gap with `the tree's production style puts items at column zero` — which is the wrong bound, because the 51 offenders ARE at column zero. `count_backlog_items` was caught only because it happened to be spelled bare `pub fn`. I separately confirmed the gap is latent not live: a scan for post-marker `pub(crate)`/`pub(super)` items across all of src/ returns 0 today. This matters more than a normal token-list gap because `clippy --lib -D warnings` (the designated gate) does NOT catch post-marker items — `items_after_test_module` needs `--all-targets`, explicitly not the gate — so this assertion is the only enforcement."
      - path: "src/driver/mod.rs"
        issue: "CONFIRMED by call-site enumeration. `visibly_empty_numbered_entry` (:1944) has exactly two call sites, :2086 and :2220, and both sit in the REALISTIC half of their loops, fed `\"/gsd:progress\"`, `\"20\"`, `\"get phase 22 verified\"`. The degenerate loop asserts `Err(NoCommandSource)` at :2188 BEFORE anything is rendered, so no degenerate payload can ever reach the detector. Its doc (:1936-1943) says `if payload::NonBlank were ever loosened ... this detector would keep calling a U+200B entry visibly empty and the matrix would go red. That disagreement is the whole value.` The conclusion is right and the named mechanism is wrong: a loosened `NonBlank` reddens the matrix at :2188's literal assertion, and the detector is never invoked. The trim tautology IS broken — by the literal `DEGENERATE` array, not by the detector. The detector's widened character classes are dead weight wearing the credit."
      - path: "tests/spawn_seam_guard.rs"
        issue: "CONFIRMED by reading. Guard eight's limit 1 claims an unanticipated variant spelling is `bounded by the per-function contributed >= COMMAND_SOURCE_VARIANT_COUNT non-vacuity below, which fails if a needle stops matching a site that still names all three` (:2286-2291). The assertion at :2433-2449 counts LINES attributed to the function, not DISTINCT NEEDLES matched. A function contributing four lines through two needles satisfies `>= 3` while a third needle has gone blind. The arithmetic is 3-and-3 today so the bound happens to bite; it stops biting the moment `command_source` grows a fourth construction line — which the fix for criterion 1 above is likely to do."
      - path: "src/driver/mod.rs"
        issue: "CONFIRMED by reading. `ALL_VARIANT_NAMES` (:1901) is a hand-maintained `[&str; 3]`. 21-13's must_haves claims `column-to-variant coverage is asserted against the single ALL_VARIANT_NAMES const that variant_name's wildcard-free match anchors`. The anchoring is one-directional: a fourth `CommandSource` variant is a compile error in `variant_name` (:1918), which forces classification — but nothing forces extending the const. If the new variant is produced by no argv position, `resolved` holds three names, `expected` holds three names, and the matrix stays green with a whole variant unexercised. The COLUMN axis is genuinely structural (`PositionBuilder`'s 3-tuple, :2163, is a compile error on a fourth argv parameter). The ROW axis is not. That is the `list a human must remember to extend` shape, one level up from the exemption this round removed."
      - path: "tests/driver_goal_seam.rs"
        issue: "CONFIRMED by reading (21-REVIEW.md WR-05, adjudicating 21-13's own deviation claim H). Inverting the falsified pin at :1368-1395 was the RIGHT call and recording it in place is the right house behaviour — but the executor undersold the cost. All four `HOSTILE_PHASE_TOKENS` (:1358-1363) carry C0/C1 control characters (`\\u{1b}`, `\\n`, `\\r`, `\\u{9b}`), so since 21-13 tightened `is_plain_path_component` every one is refused at src/driver/goal.rs:667 and nothing in the suite reaches the second refusal at :697. That branch is not dead — `untrusted::bounded` still truncates above 200 chars — but a defence-in-depth layer with zero coverage is a layer nobody will notice breaking."
    missing:
      - "Add `\"pub(\"` to `ITEM_OPENERS` and correct limit 4's bound sentence: the residual gap is an INDENTED item inside a post-marker `mod`, not `a first token outside the list`."
      - "Either correct `visibly_empty_numbered_entry`'s doc to name the mechanism that actually falsifies (the literal array), or give the detector a direct unit test — `assert!(visibly_empty_numbered_entry(\"1. \\u{200b}\").is_some())` and `(\"1. x\").is_none()` — which is cheap and makes the doc's claim true."
      - "Change guard eight's non-vacuity to assert PER-NEEDLE coverage rather than a line count."
      - "Tie `ALL_VARIANT_NAMES`'s length to the variant set through a wildcard-free `one_of_each()` so a fourth variant is a compile error there too, not only in `variant_name`."
      - "Add the one fixture that still reaches goal.rs:697 — a control-free phase token longer than `MAX_UNTRUSTED_FIELD_CHARS` (200)."
deferred:
  - truth: "`registry::current_prompt_inputs` absent from `tests/async_blocking_guard.rs`'s BLOCKING_HELPERS"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md, round-4 adjudications)"
    evidence: "Async-hygiene class, not criterion 1's class; the fix forces production spawn_blocking rewiring in approve_plan and execute_run. Recorded with reason, not dropped."
  - truth: "The spawn-gate plan-half argument lives in a comment rather than a checked property"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md)"
    evidence: "The comment now states plainly that the plan half is a no-op and why that is sound; converting a sound, honestly-documented argument into a checked property is hardening, not gap closure."
  - truth: "Dead `PlanStep::rationale` field in src/driver/goal.rs"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md)"
    evidence: "No production reader; cosmetic dead field with no security or honesty bearing."
  - truth: "`tests/driver_reattach.rs` and `tests/envelope_tracer.rs` flake under parallel execution"
    addressed_in: "Next phase backlog (deferred-items.md, priority raised in round 4)"
    evidence: "Confirmed pre-existing against the unmodified base 6eb1d49 in a throwaway worktree; the untouched baseline flakes worse than the round-4 tree. Not a round-4 regression and not counted as a gap."
behavior_unverified_items: []
coincidental_reliance_items:
  - truth: "A blank `--alias` and a blank `--approved-plan` do not reach a persisted record"
    reason: undeclared-precondition
    harden: "Neither field is protected; both are incidentally refused by a DIFFERENT mechanism — `alias` by the registry lookup returning UnknownAlias, `approved_plan` by `parse_approval_token` failing to parse. Nothing in the phase's artifacts guarantees either. They are the two remaining unconverted `DriveArgs` string fields and belong in the same sweep, not left resting on a lookup's incidental behaviour. Note `is_plain_path_component` — which DOES govern registry aliases at main.rs:244 — accepts `\\u{200b}`, so two visually identical aliases resolve to two different envelope and credential paths."
human_verification: []
---

# Phase 21: LLM Goal Layer & Prompt-Injection Hardening Verification Report

**Phase Goal:** A user states a goal once and the run pursues it, with the model confined to two narrow, bounded, hardened seams
**Verified:** 2026-08-22T01:40:00Z
**Status:** gaps_found
**Re-verification:** Yes — after gap closure (fourth cycle; fifth verification pass)

## Goal Achievement

This is the **fifth** verification pass and the **fourth** gap-closure cycle.
For the first time in this phase, **the mechanism the previous round built held
under attack** — and the phase still fails criterion 1. Those two facts are not
in tension, and separating them is the whole point of this report.

| Cycle | What closed | What the SAME cycle's diff broke |
|---|---|---|
| 1 → 2 (21-07..21-10) | round-2 CR-01/CR-02, WR-01, WR-05 | blank `--command`; approval token parsed after a live model consultation |
| 2 → 3 (21-11, 21-12) | both of the above | blank `--target-phase` accepted end to end (the `Routed` arm) |
| 3 → 4 (21-13, 21-14) | the `Routed` arm, the matrix exemption, the trim tautology, both guard headers, the REQUIREMENTS.md marking | **blank `--run-id`, blank `--goal` beside a source, blank `gsd_command` in a committed `run.json`** |

The first three rows are one story: an arm forgot the check, the fix added the
check to that arm, and the next arm forgot it. The fourth row is a **different**
story, and the difference is the finding.

### Did the `NonBlank` mechanism work? Yes — completely, within its domain.

I attacked it and could not break it, which matches what the round-4 reviewer
reports. `pub(crate) mod payload` is genuinely nested (`src/driver/mod.rs:324`),
`struct NonBlank(String)`'s field is private to that module (`:331`), the single
`impl` carries exactly `new` and `as_str`, and there is no `From`, `Deref`,
`AsRef`, `Into`, `into_inner` or serde derive to rebuild one from the `&str`
`as_str` hands back. All three `CommandSource` variants carry it. The matrix at
`:2178-2196` is one uniform nested loop — 3 argv positions × 6 `DEGENERATE`
payloads = **18 cells, every one asserted `Err(NoCommandSource)`**, with no `Ok`
branch, no by-name per-column sweep and no exemption comment anywhere in it. The
blankness boundary is pinned on the other side too (`"x"`, `" x "`,
`"\u{200b}x"`, `"x\u{feff}"` all resolve), so the refusal is a visibility rule
and not a length rule wearing its name. The column axis is a **compile-time**
constraint via `PositionBuilder`'s 3-tuple.

**Every value that passes through `command_source` is now blank-proof by
construction.** That is a real, durable, type-level guarantee and it is the
first thing in four cycles that did not regress.

### So why does criterion 1 still fail? Because the domain is smaller than the class.

`DriveArgs` (`src/driver/mod.rs:131`) carries **six** argv-derived string
fields:

| Field | Converted to `NonBlank` in round 4? | Status at HEAD |
|---|---|---|
| `command` | ✓ yes | protected |
| `target_phase` | ✓ yes | protected |
| `goal` | ✓ **only when it is the sole source** | ✗ **blank value persisted when supplied beside `--command`/`--target-phase`** |
| `run_id` | ✗ no | ✗ **`\u{200b}` accepted; a full run completes with an invisible name** |
| `alias` | ✗ no | incidentally refused by the registry lookup — not protected |
| `approved_plan` | ✗ no | incidentally refused by token parsing — not protected |

Round 4 converted **three of six** — precisely the three that `CommandSource`
happens to have variants for. All three of this pass's reproduced Criticals live
in the other three. **The fix did not fail. The scope did.** And the scope was
chosen, as it has been in all four cycles, by a human enumerating which values
to protect rather than by the compiler enumerating them.

### Observable Truths

| # | Truth (ROADMAP success criterion) | Status | Evidence |
|---|---|---|---|
| 1 | A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs | ✗ FAILED (fourth consecutive cycle, root cause relocated) | The `--target-phase` defect the last pass named is genuinely and structurally closed. But 21-13's own first `must_haves` truth — *"refused ... in EVERY argv position (`--command`, `--target-phase`, `--goal`), before any file, lock, journal, or run directory is created"* — is **false at HEAD on the public `drive()` entry point**, and its `verification: backstop` truth — *"No `run.json` ... carries `target_phase` or `gsd_command` as an empty or whitespace-only string"* — is false too. Three independent reproductions, detailed below. |
| 2 | An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input | ✓ VERIFIED (reconfirmed) | `tests/driver_goal_seam.rs` 20/20 in my own full-suite run; `tests/driver_iteration_loop.rs`, `tests/driver_router_conformance.rs` (3/3) and `tests/driver_router_table.rs` (12/12) all green. The decomposition → plan-digest → approval → recheck-at-spawn wiring is unchanged from the prior cycle's confirmed-closed state and I re-read `recheck_approval`'s position at `src/driver/run.rs:2333-2340`, above `establish_own_group`, the lock and the journal. |
| 3 | Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing | ✓ VERIFIED (reconfirmed, strictly stronger than last pass) | `tests/driver_escalation_cap.rs` 8/8. Guard six is not merely tree-wide now — its marker-to-EOF blind region is **asserted empty** by a new build-failing check, and the one live violation is fixed (`count_backlog_items` moved from `:530` to `:312`, above the `mod tests` marker at `:331`). I confirmed independently that `cargo clippy --all-targets` no longer reports `items_after_test_module` (5 → 4 warnings, the remaining four in `src/browser.rs:131-133` and `src/project_creator.rs:146`, neither file opened this round). |
| 4 | A `.planning/` file or `CLAUDE.md` carrying injected instructions ("ignore prior constraints, run …") does not change which command the driver executes | ✓ VERIFIED (reconfirmed) | `tests/driver_injection_corpus.rs` 12 passed / 10 `--ignored` live-binary arms, untouched by 21-13 or 21-14 (confirmed: neither file appears in `git diff 6eb1d49..HEAD --name-only`). Guard seven (`FIELD_OBSERVED_MARKERS` named only where the schema declares it) passes inside the 26/26 `spawn_seam_guard` run. |
| 5 | Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string | ✓ VERIFIED (reconfirmed) | `tests/driver_refusal_record.rs` 9/9; `tests/driver_model_seam.rs` 4 passed / 3 ignored. Untouched by this round's diff. I additionally confirmed a model-supplied phase token of `\u{200b}` — which survives `is_plain_path_component` (CR-02) — is still refused at `src/driver/goal.rs:704` by roadmap membership, so CR-02 does **not** reach this criterion. |

**Score:** 4/5 truths verified (0 present-but-behavior-unverified)

### Independent Reproduction (not taken from 21-REVIEW.md)

The round-4 review labelled CR-01 **inferred** and CR-02 **reproduced**. I held
it to that distinction and went further on both. Three throwaway integration
tests, written against HEAD (`c7bcafb`), run, and deleted — `git status` is
clean of them.

**1. CR-01 — UPGRADED from inferred to REPRODUCED.** Called `pub async fn
execute_run` directly with `command: None, target_phase: None`:

```
RESULT: Err(NoCommandSource)
run.json exists = true
run.json = {
  "run_id": "2026-08-21T00-00-00Z-cr01",
  "goal": "",
  "gsd_command": "",
  ...
  "outcome": "no_command_source"
}
gsd_command field = Some(String(""))
journal exists = true   (4 records: run_started, 2 diagnostics, run_ended)
```

The typed refusal 21-14 added does fire — **after** `run.json`, the run
directory and the journal are already on disk carrying the exact empty string
D-30 reserves for "field absent". I confirmed no earlier guard exists by
extracting `execute_run`'s first 195 lines (`:2261-2455`) to a file and grepping
it: **zero** `return Err`, and the only `?` sites are plan-approval staleness,
`RunIdRequired` and `bounds::resolve`.

**2. CR-02 — REPRODUCED, and end-to-end rather than at the predicate.** First
the predicate split, measured against the real library:

```
is_plain_path_component("\u{200b}")             = true
is_plain_path_component("\u{feff}")             = true
is_plain_path_component("\u{2060}")             = true
is_plain_path_component("   ")                  = false
is_plain_path_component("a\nb")                 = false
run_paths(root, U+200B).is_some()               = true
U+200B is_whitespace=false is_control=false
U+FEFF is_whitespace=false is_control=false
```

Then through the public `drive()`, which the review did not do:

```
drive(--command '/gsd:progress' --run-id '\u{200b}')  ->  Ok(())
runs dir entries:
  "run.lock"
  "\u{200b}"   (chars: ['\u{200b}'])
      run.json run_id  = Some(String("\u{200b}"))
      run.json outcome = Some(String("succeeded_no_changes"))
  ".gitignore"
```

A complete, successful, terminal run record whose directory name is one
invisible character. Nothing on disk or on screen names that run.

**3. IN-01 — REPRODUCED on the public path** (the review rated this Info; I
grade it higher because it falsifies a named `must_haves` truth):

```
[whitespace] PREVIEW Ok(())  REAL Ok(())  run.json goal = Some(String("   "))
[zero-width] PREVIEW Ok(())  REAL Ok(())  run.json goal = Some(String("\u{200b}"))
[bom]        PREVIEW Ok(())  REAL Ok(())  run.json goal = Some(String("\u{feff}"))
```

Each created the lock, the run directory, `journal.jsonl` and `run.json` — so
the "before any file, lock, journal, or run directory is created" clause is
false for `--goal` whenever another source is present. `make_run_record` writes
`goal: args.goal.clone().unwrap_or_default()` (`src/driver/run.rs:763`), raw.
`command_source`'s `(Some, None)` and `(None, Some(_))` arms never touch `goal`,
and the matrix's `PositionBuilder` only ever builds one-flag tuples — the cell
is **unexpressible** in the anti-recurrence mechanism's current shape.

I also confirmed, by direct reading rather than by re-running the review's
commands:

- `recorded_command` at `:839-849` and `digested_command_fragment` at `:867-872`
  both still carry `(None, None) => String::new()` with the identical refuted
  comment; `git log` confirms `0dcac4e` removed only the third such arm.
- `ITEM_OPENERS` (`tests/spawn_seam_guard.rs:1991`) has `"pub "` and not
  `"pub("`; `grep -rn '^pub(crate) \|^pub(super) ' src/` returns exactly **51**.
  A scan for post-marker instances of that spelling returns **0** today, so the
  gap is latent rather than live.
- `visibly_empty_numbered_entry`'s only two call sites (`:2086`, `:2220`) both
  sit in the realistic half of their loops.
- `ALL_VARIANT_NAMES` (`:1901`) is a hand-maintained `[&str; 3]`.
- All four `HOSTILE_PHASE_TOKENS` carry control characters, so none reaches
  `src/driver/goal.rs:697` any more.
- `.planning/REQUIREMENTS.md`'s last touch is `0c4f712` — neither 21-13 nor
  21-14 flipped a requirement. The pass-4 process regression is closed.

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/driver/mod.rs` (`mod payload` / `NonBlank`) | Blank payload unrepresentable by type | ✓ VERIFIED | Nested module `:324`, private field `:331`, one `impl` at `:333` with exactly `new`/`as_str`, derives `Debug, Clone, PartialEq, Eq` only. No escape route found. |
| `src/driver/mod.rs` (`command_source`, all 3 arms) | Every variant carries `NonBlank`; no arm forgets | ✓ VERIFIED | `:479`, `:489`, `:496` — each arm matches on `NonBlank::new` and maps `None` to `NoCommandSource`. The class fix, not a fourth per-arm trim. |
| `src/driver/mod.rs` (degenerate matrix) | Uniform, exemption-free, structurally exhaustive columns | ✓ VERIFIED (columns) / ⚠️ INCOMPLETE (rows, multi-flag) | 18 cells, uniform, no exemption. Column axis is a compile error on a fourth argv parameter. Row axis rests on a hand-maintained const, and multi-flag combinations are unexpressible. |
| `src/journal/mod.rs` (`is_plain_path_component`) | One definition of "blank", agreeing with the argv seam | ✗ CONTRADICTORY | `trim`-based; accepts `\u{200b}`, `\u{feff}`, `\u{2060}` that `NonBlank` refuses. Two definitions, same commit, 7 production call sites. |
| `src/driver/run.rs` (`recorded_command`, `digested_command_fragment`) | No manufactured blank in an unreachable arm | ✗ PRESENT (violates 21-14's own prohibition) | Both still `(None, None) => String::new()`; `recorded_command`'s value reaches a committed `run.json`. Reproduced. |
| `src/driver/run.rs` (`make_run_record`, `goal` field) | Goal validated before it is persisted | ✗ UNVALIDATED | `args.goal.clone().unwrap_or_default()` at `:763`. Reproduced for three payloads. |
| `src/driver/run.rs` (`IterationSource`) | Colliding enum renamed; exactly one `enum CommandSource` under `src/` | ✓ VERIFIED | Rename complete; guard eight asserts the single declaration rather than dodging by needle shape. |
| `src/state_reader/mod.rs` (`count_backlog_items`) | Above the `mod tests` marker; file clean under `items_after_test_module` | ✓ VERIFIED | `pub fn` at `:312`, marker at `:331`. Clippy `--all-targets` 5 → 4, self-measured. |
| `tests/spawn_seam_guard.rs` (boundary self-check) | Blind region provably empty | ⚠️ VERIFIED but overclaiming | The check is real, is a build failure, and caught the one live offender. Its `ITEM_OPENERS` cannot see the tree's 51 `pub(crate)`/`pub(super)` items, while the header calls the region "provably empty". |
| `tests/spawn_seam_guard.rs` (guard six header, limits 4/5) | Each limit named with its failure direction | ✓ VERIFIED | Both limits present, in the one-at-a-time register, with directions. The pass-4 finding is closed. |
| `tests/spawn_seam_guard.rs` (guard eight limits block) | Names what the scan cannot see | ⚠️ VERIFIED but one bound is weaker than stated | Five numbered limits with directions — a real improvement on "no limits at all". Limit 1's non-vacuity bound counts lines, not distinct needles. |
| `src/driver/mod.rs` (`visibly_empty_numbered_entry`) | Independent oracle that can falsify the guard | ⚠️ ORPHANED in effect | Written independently, but structurally unreachable by any degenerate payload. The tautology is broken by the literal array, not by this detector. |
| `.planning/REQUIREMENTS.md` | Accurate, internally consistent requirement status | ✓ VERIFIED (pass-4 regression closed) | All five phase-21 requirements read `[ ]` / `Gaps Found`; the checklist and traceability table now agree, which they did not before. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `DriveArgs.command` / `.target_phase` / `.goal` (sole source) | `CommandSource` | `payload::NonBlank::new` — the one place blankness is judged | ✓ WIRED | All three arms; 18/18 matrix cells; boundary pinned both sides. |
| `DriveArgs.goal` (beside another source) | `RunRecord.goal` | Validated before it is persisted | ✗ NOT WIRED | `unwrap_or_default()` at `run.rs:763`; `command_source` never sees it. Reproduced for `"   "`, `\u{200b}`, `\u{feff}`. |
| `DriveArgs.run_id` | `journal::run_paths` / run directory name | One blankness predicate shared with the argv seam | ✗ NOT WIRED | `is_plain_path_component`'s `trim` accepts what `NonBlank` refuses. Full run completes with an invisible directory name. |
| `execute_run` | `RunRecord.gsd_command` | `(None, None)` refused before anything is written | ✗ NOT WIRED | Refusal at `:2643` is downstream of `JournalRun::start` at `:2514`. Reproduced: `gsd_command: ""` on disk. |
| `payload::NonBlank::new` | `journal::is_plain_path_component` | One shared visibility predicate | ✗ NOT WIRED | Two independent predicates that disagree, shipped in the same commit. |
| `tests/spawn_seam_guard.rs` (boundary self-check) | every post-marker item under `src/` | `ITEM_OPENERS` token list | ⚠️ PARTIAL | Blind to 51 column-zero `pub(crate)`/`pub(super)` items; 0 live offenders today. |
| `src/driver/mod.rs` (matrix) | `command_source` (all variants) | Every degenerate payload × every argv position | ✓ WIRED (single-flag) / ⚠️ PARTIAL (multi-flag) | The exemption is gone. The shape cannot express a two-flag row. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `run.json` | `gsd_command` | `recorded_command(args)` → `(None, None) => String::new()` | ✗ | ✗ DISCONNECTED — reproduced as `""` on disk |
| `run.json` | `goal` | `args.goal.clone().unwrap_or_default()` | ✗ for blank/zero-width payloads | ✗ HOLLOW — reproduced as `"   "`, `"\u{200b}"`, `"\u{feff}"` |
| `run.json` / run directory | `run_id` | `is_plain_path_component` gate only | ✗ for zero-width payloads | ✗ HOLLOW — reproduced as an invisible directory name |
| `run.json` | `target_phase` | `args.target_phase` via `CommandSource::Routed(NonBlank)` | ✓ | ✓ FLOWING — this is the value the last pass failed on; it is genuinely fixed |
| TUI driver screen | `goal` line | `goal_lines(goal)` gated on `goal.trim().is_empty()` | ✗ for zero-width | ⚠️ STATIC — a `\u{200b}` goal takes the non-empty branch and renders a label with nothing after it |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Whole-suite regression | `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2` | **1320 passed, 0 failed, 13 ignored** across 35 binaries (self-run, once, not taken from any SUMMARY) | ✓ PASS |
| Build | `rtk proxy cargo build --all-targets` | exit 0, 0 warnings | ✓ PASS |
| Gate lint | `rtk proxy cargo clippy --lib -- -D warnings` | exit 0 | ✓ PASS |
| Unfiltered severity | `rtk proxy cargo clippy --all-targets` | 4 warnings, all pre-existing (`src/browser.rs:131-133`, `src/project_creator.rs:146`); `items_after_test_module` cleared | ✓ PASS (info) |
| Guard suite | within the full run | `spawn_seam_guard` 26/26 | ✓ PASS |
| Criterion-2 seam | within the full run | `driver_goal_seam` 20/20 | ✓ PASS |
| Criterion-4 corpus | within the full run | `driver_injection_corpus` 12 passed / 10 ignored | ✓ PASS |
| **CR-01 reproduction** | direct `execute_run(command: None, target_phase: None)` against a real fixture | `Err(NoCommandSource)` **and** `run.json` with `gsd_command: ""`, `goal: ""`, plus run dir + journal | ✗ CONFIRMS CR-01 (upgrades it from *inferred*) |
| **CR-02 predicate split** | `is_plain_path_component` / `run_paths` against the real library | `\u{200b}`, `\u{feff}`, `\u{2060}` → `true`; `"   "` → `false` | ✗ CONFIRMS CR-02 |
| **CR-02 end-to-end** | `drive(--command '/gsd:progress' --run-id '\u{200b}')` | `Ok(())`; run directory named `"\u{200b}"`, `run.json` `outcome: succeeded_no_changes` | ✗ CONFIRMS CR-02 (beyond what the review showed) |
| **IN-01 end-to-end** | `drive(--command '/gsd:progress' --goal '<blank>')` ×3 payloads, preview + real | all `Ok(())`; `run.json` `goal` = `"   "` / `"\u{200b}"` / `"\u{feff}"` | ✗ CONFIRMS IN-01 |
| `pub(crate)` blindness | `grep -rn '^pub(crate) \|^pub(super) ' src/` + post-marker scan | 51 invisible items tree-wide; 0 currently past a marker | ⚠️ CONFIRMS WR-01 (latent) |
| Debt markers in the round-4 diff | `git diff 6eb1d49..HEAD -- src/ tests/` (1659 lines) piped to grep | 0 `TBD`/`FIXME`/`XXX`; control needle `NonBlank` → 26 hits, so the grep is non-vacuous | ✓ PASS |
| `driver_reattach` flake | per orchestrator's throwaway-worktree run of base `6eb1d49` (5×: F,F,F,ok,ok) | pre-existing, not a regression; green in my own `--test-threads=2` run | ✓ NOT A GAP |

> Every count- or presence-bearing check above was run under `rtk proxy`, and
> each negative grep carries a positive control needle. A vacuous grep is the
> exact failure class this phase keeps re-shipping; two of them appear in this
> table with their controls stated.

### Probe Execution

Not applicable — this phase is not a migration/tooling phase and declares no
`scripts/*/tests/probe-*.sh`.

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|---|---|---|---|---|
| DRIVE-01 | 21-01, 21-04, 21-07, 21-09, 21-11, 21-13 | User states a goal once, driver pursues it without further input | ⚠️ PARTIAL | Multi-command pursuit (criterion 2) is verified and reconfirmed. "Reviewable before anything runs" fails again — this time because `--goal` beside another source is persisted unvalidated after the lock, journal and run directory exist. |
| DRIVE-03 | 21-01, 21-03, 21-04, 21-07, 21-08, 21-09, 21-11, 21-13 | Goal decomposed into a structured, machine-checkable, reviewable plan | ✗ BLOCKED | Same root cause. The decomposition itself is sound; the record that is supposed to be evidence of it is not. |
| DRIVE-04 | 21-02, 21-04, 21-06, 21-10, 21-12, 21-14 | Model escalation capped per run, exceeding it parks | ✓ SATISFIED | Criterion 3 verified; guard six is now self-checking over its own blind region, a strictly stronger control than the previous pass had. |
| SAFE-07 | 21-01, 21-03, 21-05, 21-06, 21-08, 21-10, 21-12, 21-14 | `.planning/` content passed inside an explicit untrusted boundary | ✓ SATISFIED | Criterion 4 verified; corpus untouched by this round's diff and passing with its arrival-before-influence assertions. |
| SAFE-08 | 21-01, 21-05, 21-06, 21-12, 21-14 | Model's action constrained to a fixed enum; free-form shell strings never executed | ✓ SATISFIED | Criterion 5 verified. I additionally confirmed CR-02's zero-width hole does not reach this seam: a `\u{200b}` phase token from the model survives `is_plain_path_component` but is refused by roadmap membership at `src/driver/goal.rs:704`. |

**No orphaned requirements.** The union of `requirements:` declared across all
fourteen plans (21-01 … 21-14) is exactly {DRIVE-01, DRIVE-03, DRIVE-04,
SAFE-07, SAFE-08}, matching REQUIREMENTS.md's phase-21 mapping and the ROADMAP
phase-21 `Requirements:` line. `.planning/REQUIREMENTS.md` is **correct at
HEAD** — the pass-4 premature-marking regression is closed and both plans
honoured their own `MUST NOT flip a requirement` prohibition.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/driver/run.rs` | `:848`, `:871` | `(None, None) => String::new()` — a manufactured blank as the "safe" arm of an unreachable match, twice, carrying the comment the same plan declared refuted | 🛑 Blocker | Violates 21-14's own prohibition verbatim. One copy reaches a committed `run.json`. Reproduced. |
| `src/driver/run.rs` | `:763` | `args.goal.clone().unwrap_or_default()` — raw argv into a persisted record | 🛑 Blocker | Falsifies 21-13's first `must_haves` truth on the public path. Reproduced for three payloads. |
| `src/journal/mod.rs` | `:263` | `value.trim().is_empty()` as "blank" while the sibling seam uses a wider visibility rule | 🛑 Blocker | Two disagreeing definitions in one commit; 7 production call sites; a full run completes with an invisible name. Reproduced end to end. |
| `src/driver/mod.rs` | `:1576` | The `--run-id` blankness pin loops `["   ", "\t", "\n  \n"]` — three of `DEGENERATE`'s six — in the commit that added the other three | 🛑 Blocker | Violates 21-13's own prohibition against hand-picked anti-recurrence lists. It is the fourth instance of the phase's signature defect, one level up. |
| `tests/spawn_seam_guard.rs` | `:1991-2005`, header `:2074-2078` | `ITEM_OPENERS` cannot match `pub(crate) `/`pub(super) ` (51 such items) while the header claims the region is "provably empty" and bounds the gap with "items are at column zero" — the offenders *are* at column zero | ⚠️ Warning | A guard header overclaiming its reach for the **third consecutive round**. 0 live offenders today. |
| `src/driver/mod.rs` | `:1936-1943`, `:2086`, `:2220` | `visibly_empty_numbered_entry`'s doc names a falsification mechanism it does not have; the detector is structurally unreachable by any degenerate payload | ⚠️ Warning | The tautology is genuinely broken — by the literal array. The detector wears credit it did not earn. |
| `tests/spawn_seam_guard.rs` | `:2433-2449`, claim at `:2286-2291` | Guard eight's non-vacuity counts attributed lines, not distinct needles | ⚠️ Warning | The bound bites today only because the arithmetic is 3-and-3; it stops the moment `command_source` grows a fourth construction line, which the criterion-1 fix likely adds. |
| `src/driver/mod.rs` | `:1901` | `ALL_VARIANT_NAMES` is a hand-maintained `[&str; 3]`; the anchoring to `variant_name` is one-directional | ⚠️ Warning | The column axis is structural; the row axis is a list a human must remember to extend — the shape this round removed one level down. |
| `tests/driver_goal_seam.rs` | `:1358-1395` | The inverted pin left `src/driver/goal.rs:697` reachable but with zero coverage | ⚠️ Warning | The deviation was recorded correctly and in place; the cost was undersold. |
| `src/ui/screens/driver.rs` | `:755` | `goal.trim().is_empty()` — a fifth blankness expression, on the surface that renders the user's own words | ℹ️ Info | A `\u{200b}` goal renders as a label with nothing after it. Folds into CR-02's one-predicate fix. |
| `src/driver/run.rs` | `:1945` | `from_argv_goal`'s `goal.trim()` — a fourth blankness expression | ℹ️ Info | Defence in depth, harmless today. Folds into the same fix. |

No unreferenced `TBD`/`FIXME`/`XXX` debt markers were introduced in this round's
diff (`6eb1d49..c7bcafb`, `src/` and `tests/`, 1659 lines) — confirmed with a
positive control needle (`NonBlank`, 26 hits) so the negative result is not
vacuous.

### Human Verification Required

None. Every finding in this report is a code-level fact, independently
reproduced against the built library at HEAD or read directly from the source —
not a judgment call and not taken from any SUMMARY or from `21-REVIEW.md`.

### Gaps Summary

**What genuinely closed this cycle** — all seven pass-4 gaps, reconfirmed by
direct reading and a self-run suite rather than by trusting `21-13-SUMMARY.md`
or `21-14-SUMMARY.md`: the `Routed` arm (closed structurally, not by a fourth
trim), the matrix's `--target-phase` exemption, the missing test pin, the
`trim` tautology (at the array level), guard six's fourth limit (closed *and*
converted into a build-failing self-check with the live offender relocated),
guard eight's absent limits block, and `.planning/REQUIREMENTS.md`'s premature
marking. The `enum CommandSource` collision named as debt in round 3 is also
retired. **That is the largest genuine closure in this phase's history, and
none of it regressed.**

**Criterion 1 fails a fourth time — and the failure has moved.** The first
three cycles each failed inside `command_source`: `Goal`, then `Command`, then
`Routed`. Round 4 attacked the class rather than the arm and **it worked**. I
tried to construct a blank `CommandSource` and could not; the matrix covers
18/18 cells with no exemption; the column axis is enforced by the compiler.
Nothing this pass found is inside that domain.

What this pass found is that **`NonBlank`'s domain is `CommandSource`, and the
class is `DriveArgs`**. Six argv-derived strings; three converted; three not.
The three unconverted ones are where all three reproduced Criticals live:

1. `--run-id '\u{200b}'` drives a **complete, successful, persisted run** whose
   directory name and `run_id` are one invisible character, because the seam
   that governs it defines "blank" as `trim` while the seam next door defines it
   as visibility. Two definitions, one commit, 7 call sites.
2. `--goal '   '` supplied beside `--command` is written verbatim into
   `run.json` **after** the lock, the run directory and the journal exist —
   falsifying 21-13's own first `must_haves` truth, which names `--goal`
   explicitly and says "before any file, lock, journal, or run directory is
   created".
3. `execute_run`'s two surviving `String::new()` arms put the D-30 field-absent
   sentinel into a committed `run.json`'s `gsd_command`, with the typed refusal
   21-14 added firing 129 lines too late — violating 21-14's own prohibition in
   the commit that declared it.

Each is the phase's signature defect: a no-visible-instruction value reaching a
persisted record that is supposed to be evidence. And in two of the three cases
the defect is a **hand-narrowed enumeration** — `["   ", "\t", "\n  \n"]` where
`DEGENERATE` has six; three fields converted where `DriveArgs` has six — which
is the precise thing both plans wrote prohibitions against.

Four Warning-level findings land in the round's own guard machinery. The one
that matters is `ITEM_OPENERS`: a guard header claiming a region is "provably
empty" while its token list is blind to the tree's dominant visibility spelling
(51 items), and bounding its own gap with an argument ("items are at column
zero") that is exactly backwards. **That is a guard overclaiming its reach for
the third consecutive round**, in the round commissioned to stop guards
overclaiming their reach.

### Recommendation

**Do not scope round 5 to CR-01 and CR-02.** A plan scoped to the two newest
defects would be the fifth instance of a bet that has now lost four times, and
the reason it keeps losing is visible in the data: **every cycle has enumerated
by hand which values to protect, and every hand-enumeration has been exactly one
item short.** The values were arms (three times), and now they are fields. The
next hand-enumeration will be short too.

The good news is that this round proved the *mechanism* is right — the type-level
fix held under attack, which is the first non-regression in four cycles. The
remaining work is a **bounded, enumerable sweep**, and it is bounded by the
compiler rather than by a human:

1. **Make `DriveArgs` the domain.** It is a single struct with a finite,
   compiler-visible field list: six argv-derived strings, three already
   converted. Validate at the parse boundary so `DriveArgs` cannot *hold* a
   blank. This is what makes the sweep provably complete: `execute_run` then has
   nothing blank to re-derive (both `String::new()` arms become unrepresentable
   rather than patched or reordered), `--run-id` and `--goal` are covered by
   construction, and `alias`/`approved_plan` stop resting on the incidental
   behaviour of a registry lookup and a token parser.
2. **Prove the sweep complete with a guard that reads the type, not a list.**
   Assert `DriveArgs` declares no bare `String`/`Option<String>` argv field. A
   seventh field then cannot be added without a deliberate decision, the same
   way `PositionBuilder`'s 3-tuple already makes a fourth argv parameter a
   compile error. This is the structural replacement for the hand-list, and it
   is the one thing four cycles have never had.
3. **One blankness predicate.** A shared `carries_no_visible_content` called by
   `NonBlank::new`, `is_plain_path_component`, `from_argv_goal`, `goal_lines`
   and `goal_or_none`. Five sites, all identified above; keep
   `visibly_empty_numbered_entry` deliberately separate — that one is the oracle
   and must be able to disagree.
4. **Lift `DEGENERATE` to a shared test const** every seam's pin consumes, so a
   seventh blank shape lands everywhere at once and `["   ", "\t", "\n  \n"]`
   cannot happen again.
5. **Give the matrix a multi-flag row**, since the cell that broke this round is
   currently unexpressible in its shape.

The five guard-accuracy Warnings are cheap, sit in files the plan will already
have open, and bear directly on why this recurs — they should ride the same
plan rather than be deferred a third time. In particular, add `"pub("` to
`ITEM_OPENERS` and fix limit 4's bound sentence: a guard that overclaims is how
a verifier gets told a region is clean when it is merely unchecked, and this
phase cannot afford a fourth instance of that.

**One process note.** Round 4 is the first cycle whose plans wrote explicit
`prohibitions` against the exact mistakes that then shipped inside them — hand-picked
enumerations (21-13) and manufactured blanks in unreachable arms (21-14). The
prohibitions were correctly written and correctly aimed; nothing in the pipeline
*checked* them against the diff before completion. Both are mechanically
checkable (`grep` for `String::new()` as a match-arm value in the record
builders; assert every blank-shape pin consumes the shared const). Round 5
should turn those two prohibitions into assertions in the same commit that
closes them, so the next round's declared prohibitions are enforced rather than
merely declared.

---

_Verified: 2026-08-22T01:40:00Z_
_Verifier: Claude (gsd-verifier)_
